//! Real-kache proof of the config-selector facts `scripts/kache-host.sh
//! config-source` is built on (`2026-09-23-ensuring-kache-support`, review 6):
//!
//! - a non-empty `KACHE_CONFIG` makes kache load that file, and
//!   `ignore_env = true` in the managed file does not stop the selection —
//!   `kache doctor --json` then resolves the alternate file's store, so the
//!   managed pin no longer matches;
//! - `kache daemon --json` names the file the running daemon loaded as
//!   `daemon_config_path`;
//! - while the pinned store does not exist yet, doctor's `Cache dir` detail
//!   carries a ` (will be created on first build)` note that is not part of
//!   the path.
//!
//! The L1 fixtures (`kache_init_contracts.rs`, `kache_status_contracts.rs`)
//! fake kache on these points; this test is what keeps the fake honest.
//!
//! Isolation: the real host script and the installed `kache` run with a
//! cleared environment whose `HOME`, `XDG_CONFIG_HOME`, `CARGO_HOME`, and
//! `TMPDIR` point inside a scratch root. The only daemon is a foreground
//! `kache daemon run` the test owns, bound to a socket in the scratch
//! alternate store and stopped on drop. The host's config, store, and daemon
//! are never read or written.
//!
//! Real tier (`real_`): it needs the installed kache binary. Without one it
//! skips with a `SKIP:` line; `BISCUIT_KACHE_REAL_REQUIRED=1` turns the skip
//! into a failure.

#![cfg(unix)]

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use tempfile::TempDir;

/// Set to `1` to fail instead of skip when kache is missing.
const REQUIRED_VAR: &str = "BISCUIT_KACHE_REAL_REQUIRED";

/// Deadline for the scratch daemon to answer; below nextest's termination.
const DEADLINE: Duration = Duration::from_secs(10);

/// The daemon binds a Unix socket inside the store, and `sun_path` holds 104
/// bytes on macOS, so a long temp dir falls back to `/tmp`.
const MAX_SCRATCH_BASE_LEN: usize = 48;

fn repo_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

#[test]
fn real_kache_config_selector_bypasses_the_managed_pin() {
    if find_on_path("kache").is_none() {
        skip_or_require("`kache` is not on PATH");
        return;
    }
    let scratch = Scratch::new();

    // Control: no selector, so kache loads the managed file and resolves its
    // pin — a store that does not exist yet, whose doctor detail carries the
    // "will be created" note.
    assert!(
        !scratch.managed_store().exists(),
        "the managed store must not exist before doctor runs"
    );
    let control = scratch.config_source(None);
    assert_line(
        &control,
        &format!(
            "kache-host: config-source scope=cli state=managed selector=- path={}",
            scratch.managed_config().display()
        ),
    );
    assert_line(
        &control,
        &format!(
            "kache-host: store={} source=doctor",
            scratch.managed_store().display()
        ),
    );
    assert_line(
        &control,
        &format!(
            "kache-host: config pin=match value={}",
            scratch.managed_store().display()
        ),
    );
    assert_line(
        &control,
        "kache-host: config-source scope=daemon state=unknown selector=- path=-",
    );

    // KACHE_CONFIG selects the alternate file although the managed one sets
    // `ignore_env = true`.
    let alternate = scratch.alternate_config();
    let selected = scratch.config_source(Some(&alternate));
    assert_line(
        &selected,
        &format!(
            "kache-host: config-source scope=cli state=override selector=KACHE_CONFIG path={}",
            alternate.display()
        ),
    );
    assert_line(
        &selected,
        &format!(
            "kache-host: store={} source=doctor",
            scratch.alternate_store().display()
        ),
    );
    assert_line(
        &selected,
        &format!(
            "kache-host: config pin=mismatch value={}",
            scratch.managed_store().display()
        ),
    );

    // A daemon launched under the selector reports the file it loaded.
    let _daemon = ScratchDaemon::start(&scratch, &alternate);
    let with_daemon = scratch.config_source(Some(&alternate));
    assert_line(
        &with_daemon,
        &format!(
            "kache-host: config-source scope=daemon state=override selector=daemon_config_path path={}",
            alternate.display()
        ),
    );
}

fn assert_line(output: &str, expected: &str) {
    assert!(
        output.lines().any(|line| line == expected),
        "expected the line\n  {expected}\nin:\n{output}"
    );
}

fn skip_or_require(reason: &str) {
    if env::var(REQUIRED_VAR).as_deref() == Ok("1") {
        panic!("{REQUIRED_VAR}=1, but {reason}");
    }
    eprintln!("SKIP: {reason}; set {REQUIRED_VAR}=1 to fail instead");
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// The scratch root: an isolated home holding the managed kache config, and an
/// alternate config beside it. Each pins its own store with
/// `ignore_env = true`; neither store exists up front.
struct Scratch {
    // Held for its drop, which deletes the tree.
    _root: TempDir,
    root: PathBuf,
    inherited_path: OsString,
}

impl Scratch {
    fn new() -> Self {
        let base = match env::temp_dir().canonicalize() {
            Ok(base) if base.as_os_str().len() <= MAX_SCRATCH_BASE_LEN => base,
            _ => PathBuf::from("/tmp"),
        };
        let temp = tempfile::Builder::new()
            .prefix("real-kache-cfg-")
            .tempdir_in(base)
            .expect("create scratch root");
        // kache reports canonical paths; macOS's temp dir sits behind the
        // `/var` -> `/private/var` symlink.
        let root = temp.path().canonicalize().expect("canonical scratch root");
        let scratch = Self {
            _root: temp,
            root,
            inherited_path: env::var_os("PATH").unwrap_or_default(),
        };
        for dir in ["home/.config/kache", "alternate", "cargo-home", "tmp"] {
            fs::create_dir_all(scratch.root.join(dir)).expect("create scratch dir");
        }
        write_pin(&scratch.managed_config(), &scratch.managed_store());
        write_pin(&scratch.alternate_config(), &scratch.alternate_store());
        scratch
    }

    fn managed_config(&self) -> PathBuf {
        self.root.join("home/.config/kache/config.toml")
    }

    fn managed_store(&self) -> PathBuf {
        self.root.join("managed-store")
    }

    fn alternate_config(&self) -> PathBuf {
        self.root.join("alternate/config.toml")
    }

    fn alternate_store(&self) -> PathBuf {
        self.root.join("alternate-store")
    }

    fn command(&self, program: &Path, kache_config: Option<&Path>) -> Command {
        let home = self.root.join("home");
        let mut command = Command::new(program);
        command
            .env_clear()
            .current_dir(&self.root)
            .env("PATH", &self.inherited_path)
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env("CARGO_HOME", self.root.join("cargo-home"))
            .env("TMPDIR", self.root.join("tmp"));
        if let Some(config) = kache_config {
            command.env("KACHE_CONFIG", config);
        }
        command
    }

    /// `scripts/kache-host.sh config-source`, run by the real script against
    /// the installed kache.
    fn config_source(&self, kache_config: Option<&Path>) -> String {
        let script = repo_root().join("scripts/kache-host.sh");
        let output = self
            .command(Path::new("bash"), kache_config)
            .arg(script)
            .arg("config-source")
            .output()
            .expect("run kache-host.sh config-source");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "config-source failed ({}):\n{stdout}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        stdout
    }
}

fn write_pin(config: &Path, store: &Path) {
    let store = store.to_str().expect("scratch paths are UTF-8");
    assert!(!store.contains('"'), "store path must fit a TOML string");
    fs::write(
        config,
        format!("[cache]\nlocal_store = \"{store}\"\nignore_env = true\n"),
    )
    .expect("write scratch kache config");
}

/// A foreground `kache daemon run` under a selected config, stopped on drop.
struct ScratchDaemon<'a> {
    scratch: &'a Scratch,
    config: PathBuf,
    child: Child,
}

impl<'a> ScratchDaemon<'a> {
    fn start(scratch: &'a Scratch, config: &Path) -> Self {
        let log_path = scratch.root.join("daemon.log");
        let log = fs::File::create(&log_path).expect("create daemon log");
        let child = scratch
            .command(Path::new("kache"), Some(config))
            .args(["daemon", "run"])
            .stdin(Stdio::null())
            .stdout(log.try_clone().expect("clone daemon log handle"))
            .stderr(log)
            .spawn()
            .expect("spawn kache daemon run");
        let mut daemon = Self {
            scratch,
            config: config.to_path_buf(),
            child,
        };
        daemon.wait_until_ready(&log_path);
        daemon
    }

    fn wait_until_ready(&mut self, log_path: &Path) {
        let started = Instant::now();
        loop {
            if let Ok(Some(exit)) = self.child.try_wait() {
                panic!(
                    "the scratch kache daemon exited with {exit}; log: {}",
                    fs::read_to_string(log_path).unwrap_or_default()
                );
            }
            let output = self
                .scratch
                .command(Path::new("kache"), Some(&self.config))
                .args(["daemon", "--json"])
                .output()
                .expect("run kache daemon --json");
            if let Ok(status) = serde_json::from_slice::<Value>(&output.stdout)
                && status.pointer("/daemon_running") == Some(&Value::Bool(true))
            {
                // Guards against answering for any daemon but this one.
                let socket = status
                    .pointer("/socket")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                assert!(
                    Path::new(socket).starts_with(self.scratch.alternate_store()),
                    "the daemon socket {socket} is outside the scratch store"
                );
                return;
            }
            assert!(
                started.elapsed() < DEADLINE,
                "the scratch kache daemon did not come up within {DEADLINE:?}; log: {}",
                fs::read_to_string(log_path).unwrap_or_default()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for ScratchDaemon<'_> {
    fn drop(&mut self) {
        let _ = self
            .scratch
            .command(Path::new("kache"), Some(&self.config))
            .args(["daemon", "stop"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
