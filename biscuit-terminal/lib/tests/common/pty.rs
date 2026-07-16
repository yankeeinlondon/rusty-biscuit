//! PTY test helpers for biscuit-terminal PTY tests.
//!
//! These helpers spawn the `discovery_probe` example binary inside a
//! pseudoterminal (via [`expectrl`]) so that `is_tty()` returns `true`
//! and OSC / DSR sequences can be manufactured by the test.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use expectrl::session::OsSession;

/// The OSC 10 (foreground color) query the library writes to `/dev/tty`,
/// `ESC ] 10 ; ? BEL`. The `;?` form is distinct from any manufactured
/// `;rgb:` reply, so counting it in the master stream is unambiguous.
pub const OSC10_QUERY: &[u8] = b"\x1b]10;?\x07";

/// The OSC 11 (background color) query, `ESC ] 11 ; ? BEL`.
pub const OSC11_QUERY: &[u8] = b"\x1b]11;?\x07";

/// Count non-overlapping occurrences of `needle` in `haystack`.
pub fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    let mut count = 0;
    let mut i = 0;
    while i + needle.len() <= haystack.len() {
        if &haystack[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count
}

/// Locate the `discovery_probe` example binary.
///
/// Derives the path from the current test executable location so it
/// works in both debug and release profiles without relying on
/// `CARGO_BIN_EXE_` env vars (which cargo only sets for binaries, not
/// examples).
fn discovery_probe_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe unavailable");
    // test exe is at  <target_dir>/<profile>/deps/<name>
    // we want        <target_dir>/<profile>/examples/discovery_probe
    let mut dir = exe.parent().unwrap().to_path_buf(); // deps/
    dir = dir.parent().unwrap().to_path_buf(); // <profile>/
    dir.join("examples").join("discovery_probe")
}

/// Base environment variables that prevent hangs and unwanted side-effects
/// in the spawned probe.
///
/// * `NO_COLOR=1` – disables color output so assertions don't have to
///   cope with SGR sequences.
pub fn anti_hang_env() -> Vec<(&'static str, &'static str)> {
    vec![("NO_COLOR", "1")]
}

/// Spawn the `discovery_probe` example binary in a PTY with the given
/// extra environment variables.
///
/// Caller-supplied variables override the base anti-hang set.
///
/// ## Panics
///
/// Panics if the binary cannot be found or the PTY spawn fails.
pub fn spawn_with_env(envs: &[(&str, &str)]) -> OsSession {
    let bin = discovery_probe_path();
    if !bin.exists() {
        panic!(
            "discovery_probe example not found at {}. \
             Run `cargo build -p biscuit-terminal --example discovery_probe` first.",
            bin.display()
        );
    }
    let mut cmd = Command::new(&bin);

    // Remove terminal-specific env vars that would override TERM_PROGRAM
    // detection, so tests can manufacture a terminal identity cleanly.
    for var in [
        "WEZTERM_UNIX_SOCKET",
        "WEZTERM_PANE",
        "KITTY_WINDOW_ID",
        "KITTY_PID",
        "ITERM_SESSION_ID",
        "ITERM_PROFILE",
        "GHOSTTY_RESOURCES_DIR",
        "ALACRITTY_WINDOW_ID",
        "ALACRITTY_SOCKET",
        "ALACRITTY_LOG",
        "WT_SESSION",
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "TRAVIS",
        "CIRCLECI",
    ] {
        cmd.env_remove(var);
    }

    for (k, v) in anti_hang_env() {
        cmd.env(k, v);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }

    expectrl::Session::spawn(cmd).expect("failed to spawn discovery_probe in PTY")
}

/// Convenience: spawn with only the anti-hang environment.
pub fn spawn_probe() -> OsSession {
    spawn_with_env(&[])
}

/// Read everything currently available from `session` without blocking.
///
/// Uses a short timeout so that slow output is still captured, but the
/// function returns promptly when the pipe is dry.
pub fn try_read_available(session: &mut OsSession) -> String {
    let mut buf = String::new();
    let mut scratch = [0u8; 4096];

    // Poll a few times with a tiny sleep — enough for the probe to finish
    // a single query/response cycle.
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(10));
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                buf.push_str(&String::from_utf8_lossy(&scratch[..n]));
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
        }
    }
    buf
}

/// A one-shot OSC answer: when `query` first appears in the master stream,
/// `reply` is written back into the PTY exactly once.
pub struct OscAnswer {
    pub query: &'static [u8],
    pub reply: &'static [u8],
    sent: bool,
}

impl OscAnswer {
    pub fn new(query: &'static [u8], reply: &'static [u8]) -> Self {
        Self {
            query,
            reply,
            sent: false,
        }
    }
}

/// Drive a probe session: read the master stream, answer each pending
/// [`OscAnswer`] once when its query first appears, and stop when the ASCII
/// `until` marker is seen in the collected bytes or `deadline` elapses.
///
/// Returns the raw master bytes collected. Callers can both byte-count OSC
/// query sequences (interleaved from `/dev/tty`) and parse the probe's
/// `key=value` stdout lines from the same buffer, since both share the PTY.
///
/// ## Panics
///
/// Panics if `until` is never seen before `deadline`.
pub fn drive_probe(
    session: &mut OsSession,
    answers: &mut [OscAnswer],
    until: &str,
    deadline: Duration,
) -> Vec<u8> {
    let start = Instant::now();
    let mut collected: Vec<u8> = Vec::new();
    let mut scratch = [0u8; 4096];

    loop {
        match session.try_read(&mut scratch) {
            Ok(0) => {}
            Ok(n) => collected.extend_from_slice(&scratch[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }

        for answer in answers.iter_mut() {
            if !answer.sent && count_occurrences(&collected, answer.query) > 0 {
                let _ = session.write_all(answer.reply);
                let _ = session.flush();
                answer.sent = true;
            }
        }

        if find_subslice(&collected, until.as_bytes()) {
            return collected;
        }
        if start.elapsed() > deadline {
            panic!(
                "drive_probe: marker {until:?} not seen within {deadline:?}; collected {} bytes: {:?}",
                collected.len(),
                String::from_utf8_lossy(&collected)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    collected
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    count_occurrences(haystack, needle) > 0
}
