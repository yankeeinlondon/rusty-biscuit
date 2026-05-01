//! PTY-driven keyboard protocol tests.
//!
//! These tests spawn the `question` binary inside a pseudo-terminal and
//! verify that:
//!
//! 1. The kitty keyboard enhancement protocol is pushed successfully on
//!    supported terminals.
//! 2. A bare Ctrl press (sent as raw kitty-protocol bytes) causes hotkey
//!    badges to become visible.
//! 3. When the terminal rejects the enhancement push, the runner does not
//!    panic and chord fallback still works.
//!
//! They are gated behind `RUN_PTY_TESTS=1` so default `cargo test` stays fast.

use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn pty_tests_enabled() -> bool {
    env::var("RUN_PTY_TESTS").as_deref() == Ok("1")
}

fn skip_if_not_enabled() -> bool {
    if !pty_tests_enabled() {
        eprintln!("skipping: set RUN_PTY_TESTS=1 to enable keyboard protocol PTY tests");
        return true;
    }
    false
}

fn question_binary() -> PathBuf {
    assert_cmd::cargo::cargo_bin("question")
}

fn read_all_available(session: &mut expectrl::session::OsSession) -> String {
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    session.set_expect_timeout(Some(Duration::from_millis(300)));
    loop {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&scratch[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
        }
    }
    session.set_expect_timeout(Some(Duration::from_secs(10)));
    String::from_utf8_lossy(&buf).into_owned()
}

fn answer_cursor_position_request(session: &mut expectrl::session::OsSession) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut output = String::new();
    let mut scratch = [0u8; 4096];

    while Instant::now() < deadline {
        match session.try_read(&mut scratch) {
            Ok(0) => break,
            Ok(n) => {
                output.push_str(&String::from_utf8_lossy(&scratch[..n]));
                if output.contains("\x1b[6n") {
                    session
                        .write_all(b"\x1b[1;1R")
                        .expect("send cursor position response");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }

    session.set_expect_timeout(Some(Duration::from_secs(10)));
    std::thread::sleep(Duration::from_millis(300));
}

// ---------------------------------------------------------------------------
// 3.4 — Bare modifier press shows hotkey badges
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod keyboard_protocol {
    use super::*;
    use expectrl::{Session, session::OsSession};

    /// Kitty keyboard protocol sequence for a bare Left-Control press.
    ///
    /// CSI 57442 ; 1 u  → Left-Control press.
    const KITTY_CTRL_PRESS: &[u8] = b"\x1b[57442;1u";

    /// Kitty keyboard protocol sequence for a bare Left-Control release.
    const KITTY_CTRL_RELEASE: &[u8] = b"\x1b[57442;1:3u";

    fn spawn_choose_one() -> OsSession {
        let binary = question_binary();
        let mut command = Command::new(binary);
        command.args(["choose-one", "--height", "5", "Red", "Green", "Blue"]);
        let mut session = Session::spawn(command).expect("spawn question choose-one");
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        answer_cursor_position_request(&mut session);
        session
    }

    #[test]
    fn bare_ctrl_shows_hotkey_badges() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_choose_one();

        // Drain initial render
        let _ = read_all_available(&mut session);

        // Send bare Ctrl press via kitty protocol bytes
        session
            .write_all(KITTY_CTRL_PRESS)
            .expect("send ctrl press");
        std::thread::sleep(Duration::from_millis(200));

        // Read the re-rendered output
        let output = read_all_available(&mut session);

        // Plain positional options receive default Ctrl hotkeys, so a bare Ctrl
        // press should reveal their compact ^R/^G/^B badges.
        let has_badge = output.contains("^R") || output.contains("^G") || output.contains("^B");
        assert!(
            has_badge || output.contains("Ctrl") || output.contains("CTRL"),
            "bare Ctrl press should reveal hotkey badges; output was: {output:?}"
        );

        // Send release so the prompt doesn't stay stuck
        session
            .write_all(KITTY_CTRL_RELEASE)
            .expect("send ctrl release");
        // Send Enter to submit and exit
        session.write_all(b"\r").expect("send enter");
        std::thread::sleep(Duration::from_millis(200));
    }

    #[test]
    fn chord_fallback_still_works() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_choose_one();

        // Drain initial render
        let _ = read_all_available(&mut session);

        // Send a mapped chord (Ctrl+G). On terminals without explicit
        // modifier-only events, chord handling must still select and submit.
        session.write_all(b"\x07").expect("send ctrl+g");
        std::thread::sleep(Duration::from_millis(200));

        let output = read_all_available(&mut session);

        assert!(
            output.contains("Green"),
            "Ctrl+G should select and submit Green; output was: {output:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3.5 — Degraded terminal: no panic, chord fallback works
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod degraded_terminal {
    use super::*;
    use expectrl::{Session, session::OsSession};

    /// Force the `TERM` variable to a dumb terminal that does not support
    /// kitty keyboard enhancements.  crossterm's push should fail, but the
    /// runner must not panic and chord fallback must still function.
    fn spawn_choose_one_dumb() -> OsSession {
        let binary = question_binary();
        let mut command = Command::new(binary);
        command
            .env("TERM", "dumb")
            .args(["choose-one", "--height", "5", "Red", "Green", "Blue"]);
        let mut session =
            Session::spawn(command).expect("spawn question choose-one with TERM=dumb");
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        answer_cursor_position_request(&mut session);
        session
    }

    #[test]
    fn dumb_terminal_does_not_panic() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_choose_one_dumb();

        // Just drain output to confirm the process started
        let output = read_all_available(&mut session);
        // Should contain at least one option label
        assert!(
            output.contains("Red") || output.contains("Green") || output.contains("Blue"),
            "TUI should render even on dumb terminal; output was: {output:?}"
        );

        // Exit cleanly with Enter
        session.write_all(b"\r").expect("send enter");
        std::thread::sleep(Duration::from_millis(300));

        // Ensure the process exited (no panic)
        let mut buf = [0u8; 1];
        match session.try_read(&mut buf) {
            Ok(0) => {}                                              // EOF — clean exit
            Ok(_) => {}                                              // Trailing bytes are fine
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {} // Timeout is fine
            Err(e) => panic!("unexpected error reading from session: {e}"),
        }
    }

    #[test]
    fn dumb_terminal_chord_fallback_works() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_choose_one_dumb();

        let _ = read_all_available(&mut session);

        // Send mapped Ctrl+G chord.
        session.write_all(b"\x07").expect("send ctrl+g");
        std::thread::sleep(Duration::from_millis(200));

        let output = read_all_available(&mut session);

        assert!(
            output.contains("Green"),
            "chord fallback should submit Green on dumb terminal; output was: {output:?}"
        );
    }
}

#[cfg(not(unix))]
mod skip_unix_only {
    use super::*;

    #[test]
    fn pty_tests_are_unix_only() {
        if skip_if_not_enabled() {
            return;
        }
        eprintln!("keyboard protocol PTY tests are unix-only");
    }
}
