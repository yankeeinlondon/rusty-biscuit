//! Recording fake TTS executables and the isolated environment they run in.
//!
//! [`TtsFixture`] owns every location a synthesis can touch: a `PATH` holding
//! only the fakes it installs, the OS temporary directory (and so the shared
//! audio cache), the voice-capability cache file, and Playa's spool. Each fake
//! records its arguments, its stdin when the real tool reads stdin, and the
//! output path it was asked to write, so a test can count syntheses and see
//! exactly what was requested.
//!
//! The fakes are `/bin/sh` scripts. `std::process::Command` resolves a bare
//! program name to an `.exe` on Windows, so a script fake cannot stand in
//! there; Windows coverage of the shared behavior comes from the library's
//! portable unit tests.

#![allow(dead_code, reason = "each test binary uses a different subset of the helpers")]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use test_toolkit::{EnvGuard, LockedAudioSpool};

/// A host executable a provider or Playa launches by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeTool {
    Say,
    KokoroTts,
    GttsCli,
    Echogarden,
    /// The host player Playa falls back to when native decoding fails, which
    /// it always does for [`fake_audio`]. Installing it makes playback a
    /// recorded boundary that never reaches an audio device.
    Mpv,
}

impl FakeTool {
    /// The program name the provider resolves on `PATH`.
    pub fn program(self) -> &'static str {
        match self {
            Self::Say => "say",
            Self::KokoroTts => "kokoro-tts",
            Self::GttsCli => "gtts-cli",
            Self::Echogarden => "echogarden",
            Self::Mpv => "mpv",
        }
    }

    /// Shell that sets `output` to the file this invocation must write, or
    /// leaves it empty for a non-synthesis invocation such as a voice listing.
    fn locate_output(self) -> &'static str {
        match self {
            Self::Say => {
                r#"output=; while [ "$#" -gt 0 ]; do if [ "$1" = -o ]; then output="$2"; shift; fi; shift; done"#
            }
            Self::GttsCli => {
                r#"output=; while [ "$#" -gt 0 ]; do if [ "$1" = --output ]; then output="$2"; shift; fi; shift; done"#
            }
            // `kokoro-tts INPUT OUTPUT --voice …`; `--help-voices` is the readiness probe.
            Self::KokoroTts => r#"output=; case "$1" in -*) ;; *) output="$2" ;; esac"#,
            // `echogarden speak TEXT OUTPUT …`; `list-voices` writes no file.
            Self::Echogarden => r#"output=; [ "$1" = speak ] && output="$3""#,
            Self::Mpv => "output=",
        }
    }

    /// Only `say` receives its text on stdin; the others would block reading
    /// an inherited stdin.
    fn reads_stdin(self) -> bool {
        matches!(self, Self::Say)
    }
}

/// What a fake does when it is asked to synthesize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behavior {
    /// Write [`fake_audio`] to the output path and exit 0.
    Synthesize,
    /// Write a partial file to the output path, then exit 1.
    FailAfterPartialWrite,
    /// Write a partial file to the output path, then sleep for a minute so a
    /// test can cancel the caller and check the child was killed.
    HangAfterPartialWrite,
}

/// Bytes a successful fake writes for `tool`.
pub fn fake_audio(tool: FakeTool) -> Vec<u8> {
    format!("fake {} audio", tool.program()).into_bytes()
}

/// One recorded launch of a fake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub pid: u32,
    /// Arguments, one per line as recorded; an argument containing a newline
    /// is split.
    pub args: Vec<String>,
    /// Text read from stdin, for tools that read it.
    pub stdin: Option<String>,
    /// File the invocation was asked to write; `None` for a non-synthesis call.
    pub output: Option<PathBuf>,
}

/// An isolated environment for driving providers against recording fakes.
///
/// Process-wide environment changes are held by [`EnvGuard`]s and restored on
/// drop, so tests using a fixture must be `#[serial_test::serial]`.
pub struct TtsFixture {
    root: tempfile::TempDir,
    _env: Vec<EnvGuard>,
}

impl TtsFixture {
    /// Create the fixture and point `PATH`, `TMPDIR`/`TMP`/`TEMP`,
    /// `BISCUIT_SPEAKS_CACHE`, and `PLAYA_SPOOL_DIR` into it.
    ///
    /// `PLAYA_DRY_RUN` is removed so synthesis is observable. The spool
    /// directory is named but not created; [`Self::assert_spool_absent`]
    /// proves nothing created it, and [`Self::lock_spool`] creates it for a
    /// test that publishes.
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("fixture root");
        for dir in ["bin", "tmp", "records", "capabilities"] {
            std::fs::create_dir(root.path().join(dir)).expect("fixture directory");
        }
        let tmp = root.path().join("tmp");
        let env = vec![
            EnvGuard::set_safe("PATH", root.path().join("bin")),
            EnvGuard::set_safe("TMPDIR", &tmp),
            EnvGuard::set_safe("TMP", &tmp),
            EnvGuard::set_safe("TEMP", &tmp),
            EnvGuard::set_safe(
                "BISCUIT_SPEAKS_CACHE",
                root.path().join("capabilities").join("biscuit-speaks-cache.json"),
            ),
            EnvGuard::set_safe("PLAYA_SPOOL_DIR", root.path().join("spool")),
            EnvGuard::remove_safe("PLAYA_DRY_RUN"),
        ];
        Self { root, _env: env }
    }

    /// Install `tool` on the fixture `PATH`, replacing any earlier install
    /// while keeping its recorded invocations.
    pub fn install(&self, tool: FakeTool, behavior: Behavior) -> PathBuf {
        let record = self.records_dir().join(tool.program());
        let capture_stdin = if tool.reads_stdin() {
            "/bin/cat > \"$rec.stdin\""
        } else {
            ""
        };
        let on_output = match behavior {
            Behavior::Synthesize => format!(
                "printf '%s' '{}' > \"$output\"",
                String::from_utf8(fake_audio(tool)).unwrap()
            ),
            Behavior::FailAfterPartialWrite => {
                "printf partial > \"$output\"; echo 'scripted failure' >&2; exit 1".into()
            }
            Behavior::HangAfterPartialWrite => {
                "printf partial > \"$output\"; exec /bin/sleep 60".into()
            }
        };
        let script = format!(
            "#!/bin/sh\nrec={record}-$$\nprintf '%s\\n' \"$@\" > \"$rec.args\"\n{capture_stdin}\n{locate}\n\
             if [ -n \"$output\" ]; then\n  printf '%s' \"$output\" > \"$rec.output\"\n  {on_output}\nfi\nexit 0\n",
            record = shell_quote(&record),
            locate = tool.locate_output(),
        );
        let program = self.bin_dir().join(tool.program());
        std::fs::write(&program, script).expect("fake executable");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
            .expect("fake executable mode");
        program
    }

    /// Every recorded launch of `tool`, in no particular order.
    pub fn invocations(&self, tool: FakeTool) -> Vec<Invocation> {
        let prefix = format!("{}-", tool.program());
        let mut invocations = Vec::new();
        for entry in std::fs::read_dir(self.records_dir()).expect("records directory") {
            let name = entry.unwrap().file_name().into_string().unwrap();
            let Some(pid) = name
                .strip_prefix(&prefix)
                .and_then(|rest| rest.strip_suffix(".args"))
            else {
                continue;
            };
            let base = self.records_dir().join(format!("{prefix}{pid}"));
            let read = |extension: &str| std::fs::read_to_string(base.with_extension(extension)).ok();
            invocations.push(Invocation {
                pid: pid.parse().expect("recorded pid"),
                args: read("args").unwrap().lines().map(str::to_owned).collect(),
                stdin: read("stdin"),
                output: read("output").map(PathBuf::from),
            });
        }
        invocations
    }

    /// Launches of `tool` that were asked to write an output file.
    pub fn syntheses(&self, tool: FakeTool) -> Vec<Invocation> {
        self.invocations(tool)
            .into_iter()
            .filter(|invocation| invocation.output.is_some())
            .collect()
    }

    /// Every file in the fixture's temporary directory, including partial and
    /// temporary files, sorted by name.
    pub fn temp_entries(&self) -> Vec<PathBuf> {
        let mut entries: Vec<_> = std::fs::read_dir(self.temp_dir())
            .expect("fixture temp directory")
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        entries
    }

    /// Published shared-audio-cache files.
    pub fn audio_cache_entries(&self) -> Vec<PathBuf> {
        self.temp_entries()
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("biscuit-speaks-"))
            })
            .collect()
    }

    /// Fail unless nothing created Playa's spool, which also holds its journal.
    pub fn assert_spool_absent(&self) {
        assert!(
            !self.spool_dir().exists(),
            "no spool or journal may be created at {}",
            self.spool_dir().display()
        );
    }

    /// Create the spool and take its worker lock, so jobs a test publishes are
    /// never played and are removed under `queue.lock` even on unwind.
    pub fn lock_spool(&self) -> LockedAudioSpool {
        let spool = LockedAudioSpool::new(&self.spool_dir());
        for dir in ["files", "requests"] {
            std::fs::create_dir_all(self.spool_dir().join(dir)).expect("spool directory");
        }
        spool
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.path().join("bin")
    }

    pub fn temp_dir(&self) -> PathBuf {
        self.root.path().join("tmp")
    }

    pub fn capability_cache(&self) -> PathBuf {
        self.root.path().join("capabilities").join("biscuit-speaks-cache.json")
    }

    pub fn spool_dir(&self) -> PathBuf {
        self.root.path().join("spool")
    }

    fn records_dir(&self) -> PathBuf {
        self.root.path().join("records")
    }
}

/// Whether `pid` still names a live process.
pub fn process_alive(pid: u32) -> bool {
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("kill -0 {pid} 2>/dev/null"))
        .status()
        .expect("probe process")
        .success()
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}
