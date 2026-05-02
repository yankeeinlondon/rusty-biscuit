//! PTY test helpers for Level-1 biscuit-terminal tests.
//!
//! These helpers spawn the `discovery_probe` example binary inside a
//! pseudoterminal (via [`expectrl`]) so that `is_tty()` returns `true`
//! and OSC / DSR sequences can be manufactured by the test.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use expectrl::session::OsSession;

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
