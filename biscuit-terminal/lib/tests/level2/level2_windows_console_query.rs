//! Level 2 tests of the native Windows console query port: `discovery_probe`
//! runs in a windowless pseudoconsole (ConPTY, through `xpty`) and this test
//! plays the terminal, answering the queries the console forwards.
//!
//! `xpty` is used rather than the shared `expectrl` helper because the console
//! host behind `expectrl`'s pseudoconsole was measured answering DA1 itself
//! and not forwarding the child's queries (the `os` skill's
//! `windows-console.md`).
//!
//! The probe's stdout goes to a file through `cmd /c … > out.txt`: the
//! pseudoconsole's output stream is a repaint of the screen, not the text the
//! child wrote, while the library reaches the console through `CONIN$` and
//! `CONOUT$` regardless of the redirect.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use test_toolkit::{Level, require_level};
use xpty::{Child, CommandBuilder, MasterPty, PtySize, PtySystem as _, native_pty_system};

/// Each wait stays well under nextest's termination, so a stall fails with
/// the console stream and the probe's output instead of a bare TIMEOUT.
const STEP: Duration = Duration::from_secs(15);

const OSC10_QUERY: &[u8] = b"\x1b]10;?\x07";
const OSC11_QUERY: &[u8] = b"\x1b]11;?\x07";
const DA1_QUERY: &[u8] = b"\x1b[c";
/// Windows Terminal's DA1 reply; attribute `4` is Sixel.
const SIXEL_DA1_REPLY: &[u8] = b"\x1b[?61;4;6;7;14;21;22;23;24;28;32;42c";
/// The same reply without Sixel.
const PLAIN_DA1_REPLY: &[u8] = b"\x1b[?61;6;7;14;21;22;23;24;28;32;42c";

/// A reply written whenever `query` appears in the console's output; `keys`
/// are typed just before the first reply.
struct Answer {
    query: &'static [u8],
    reply: &'static [u8],
    keys: &'static [u8],
}

const fn answer(query: &'static [u8], reply: &'static [u8]) -> Answer {
    Answer {
        query,
        reply,
        keys: b"",
    }
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|w| *w == needle)
        .count()
}

fn probe_path() -> PathBuf {
    // <target>/<profile>/deps/<test exe> → <target>/<profile>/examples/
    let exe = std::env::current_exe().expect("current_exe");
    let profile = exe
        .parent()
        .and_then(Path::parent)
        .expect("test binary under <profile>/deps");
    let probe = profile.join("examples").join("discovery_probe.exe");
    assert!(
        probe.exists(),
        "{} is missing; build it with `cargo build -p biscuit-terminal --example discovery_probe`",
        probe.display()
    );
    probe
}

/// `discovery_probe` in a pseudoconsole with `WT_SESSION` set, so the color
/// gate passes and the graphics route asks for a probe.
struct ProbeConsole {
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    stream: Arc<Mutex<Vec<u8>>>,
    output: PathBuf,
    _dir: tempfile::TempDir,
    _master: Box<dyn MasterPty + Send>,
}

impl ProbeConsole {
    fn launch(answers: Vec<Answer>, env: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 40,
                cols: 160,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("open a pseudoconsole");
        let mut cmd = CommandBuilder::new("cmd.exe");
        cmd.args(["/d", "/c"]);
        cmd.arg(probe_path());
        cmd.args([">", "out.txt", "2>&1"]);
        cmd.cwd(dir.path());
        // An SSH marker would give every query the remote silence budget and
        // close the graphics route; the others would replace the identity
        // this test sets.
        for var in [
            "SSH_CLIENT",
            "SSH_CONNECTION",
            "SSH_TTY",
            "TERM",
            "TERM_PROGRAM",
            "MSYSTEM",
            "WEZTERM_UNIX_SOCKET",
            "WEZTERM_PANE",
            "BISCUIT_GRAPHICS",
            "CI",
            "GITHUB_ACTIONS",
        ] {
            cmd.env_remove(var);
        }
        cmd.env("PROBE", "graphics");
        // The child's std handles are pipes behind cmd's redirect, so its own
        // TTY check would say no.
        cmd.env("PROBE_FORCE_TTY", "true");
        cmd.env("WT_SESSION", "biscuit-terminal-level2");
        cmd.env("BISCUIT_TERMINAL_QUERY_SILENCE_MS", "1000");
        for (key, value) in env {
            cmd.env(key, value);
        }
        let child = pair
            .slave
            .spawn_command(cmd)
            .expect("start discovery_probe");
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("pseudoconsole reader");
        let writer = Arc::new(Mutex::new(
            pair.master.take_writer().expect("pseudoconsole writer"),
        ));
        let stream = Arc::new(Mutex::new(Vec::new()));
        let (sink, keyboard) = (Arc::clone(&stream), Arc::clone(&writer));
        // Draining continuously also keeps ConPTY from blocking the child on
        // a full output pipe. The console asks DA1 itself at startup and
        // holds the child until it is answered.
        std::thread::spawn(move || {
            let mut answered = vec![0usize; answers.len()];
            let mut chunk = [0u8; 4096];
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let mut seen = sink.lock().unwrap();
                seen.extend_from_slice(&chunk[..n]);
                let mut keyboard = keyboard.lock().unwrap();
                for (answer, answered) in answers.iter().zip(answered.iter_mut()) {
                    while count(&seen, answer.query) > *answered {
                        if *answered == 0 {
                            let _ = keyboard.write_all(answer.keys);
                        }
                        let _ = keyboard.write_all(answer.reply);
                        *answered += 1;
                    }
                }
                let _ = keyboard.flush();
            }
        });
        Self {
            child,
            writer,
            stream,
            output: dir.path().join("out.txt"),
            _dir: dir,
            _master: pair.master,
        }
    }

    fn output(&self) -> String {
        std::fs::read_to_string(&self.output).unwrap_or_default()
    }

    fn diagnostics(&self) -> String {
        format!(
            "probe output:\n{}\nconsole stream: {:?}",
            self.output(),
            String::from_utf8_lossy(&self.stream.lock().unwrap())
        )
    }

    fn wait_for_output(&self, needle: &str) {
        let deadline = Instant::now() + STEP;
        while !self.output().contains(needle) {
            assert!(
                Instant::now() < deadline,
                "{needle:?} never appeared; {}",
                self.diagnostics()
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn type_keys(&self, keys: &[u8]) {
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(keys).expect("write to the console");
        writer.flush().expect("flush the console");
    }

    /// Waits for the probe to exit and returns its output.
    fn finish(mut self) -> String {
        let deadline = Instant::now() + STEP;
        while !matches!(self.child.try_wait(), Ok(Some(_))) {
            assert!(
                Instant::now() < deadline,
                "discovery_probe did not exit; {}",
                self.diagnostics()
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        let output = self.output();
        assert!(
            output.contains("graphics_done"),
            "probe did not finish; {}",
            self.diagnostics()
        );
        output
    }
}

impl Drop for ProbeConsole {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn line<'a>(output: &'a str, key: &str) -> &'a str {
    output
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no {key} line in:\n{output}"))
}

#[test]
fn measured_colors_reach_a_windows_program() {
    require_level!(Level::L2, true, "ConPTY");
    let console = ProbeConsole::launch(
        vec![
            answer(OSC10_QUERY, b"\x1b]10;rgb:cccc/dddd/eeee\x1b\\"),
            answer(OSC11_QUERY, b"\x1b]11;rgb:1111/2222/3333\x1b\\"),
            answer(DA1_QUERY, PLAIN_DA1_REPLY),
        ],
        &[],
    );
    let output = console.finish();

    assert_eq!(
        line(&output, "graphics_text_color[0]"),
        "Some(RgbValue { r: 204, g: 221, b: 238 })"
    );
    assert_eq!(
        line(&output, "graphics_bg_color[0]"),
        "Some(RgbValue { r: 17, g: 34, b: 51 })"
    );
}

#[test]
fn da1_sixel_attribute_verifies_the_sixel_route() {
    require_level!(Level::L2, true, "ConPTY");
    let console = ProbeConsole::launch(vec![answer(DA1_QUERY, SIXEL_DA1_REPLY)], &[]);
    let output = console.finish();

    assert_eq!(line(&output, "graphics_format[0]"), "Sixel", "{output}");
    // A WindowsTerminal.exe of 1.22 or later among the ancestors decides
    // Sixel from its version before any probe; under the test runner there
    // normally is none, so the DA1 reply is what decides.
    let confidence = line(&output, "graphics_confidence[0]");
    match confidence {
        "Verified" => assert!(
            line(&output, "graphics_reason[0]").contains("DA1"),
            "{output}"
        ),
        "Known" => {}
        other => panic!("unexpected confidence {other}:\n{output}"),
    }
}

#[test]
fn keys_typed_during_the_exchange_reach_the_program_afterwards() {
    require_level!(Level::L2, true, "ConPTY");
    // Typed only once the query is on the wire: keys already waiting when
    // the exchange starts would make the library skip the graphics query.
    let console = ProbeConsole::launch(
        vec![
            Answer {
                query: OSC11_QUERY,
                reply: b"\x1b]11;rgb:1111/2222/3333\x1b\\",
                keys: b"abc",
            },
            answer(OSC10_QUERY, b"\x1b]10;rgb:cccc/dddd/eeee\x1b\\"),
            answer(DA1_QUERY, PLAIN_DA1_REPLY),
        ],
        &[("PROBE_READ_TYPEAHEAD_MS", "3000")],
    );
    // The route lines print after the exchange has returned the keys, so
    // Enter lands behind them.
    console.wait_for_output("graphics_bg_color[0]");
    console.type_keys(b"\r");
    let output = console.finish();

    assert_eq!(
        line(&output, "typeahead_line"),
        r#"Some("abc")"#,
        "{output}"
    );
    assert_eq!(
        line(&output, "graphics_bg_color[0]"),
        "Some(RgbValue { r: 17, g: 34, b: 51 })"
    );
}
