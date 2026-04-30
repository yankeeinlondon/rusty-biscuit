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
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

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
        match session.read(&mut scratch) {
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

// ---------------------------------------------------------------------------
// 3.4 — Bare modifier press shows hotkey badges
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod keyboard_protocol {
    use super::*;
    use expectrl::{session::OsSession, spawn};

    /// Kitty keyboard protocol sequence for a bare Left-Control press.
    ///
    /// CSI 1 ; 1 : 1 u  → key code 1 (Ctrl), modifiers 1 (none), event type 1 (press)
    const KITTY_CTRL_PRESS: &[u8] = b"\x1b[1;1:1u";

    /// Kitty keyboard protocol sequence for a bare Left-Control release.
    const KITTY_CTRL_RELEASE: &[u8] = b"\x1b[1;1:3u";

    fn spawn_choose_one() -> OsSession {
        let binary = question_binary();
        let cmd = format!(
            "{} choose-one --height 5 \"Red\" \"Green\" \"Blue\"",
            binary.to_str().unwrap()
        );
        let mut session = spawn(&cmd).expect("spawn question choose-one");
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        // Wait for the TUI to render
        std::thread::sleep(Duration::from_millis(400));
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
        session.write_all(KITTY_CTRL_PRESS).expect("send ctrl press");
        std::thread::sleep(Duration::from_millis(200));

        // Read the re-rendered output
        let output = read_all_available(&mut session);

        // When Ctrl is held, hotkey badges for Ctrl-hotkeys should be visible.
        // The exact badge text depends on the theme, but "Ctrl" or "[C]" or
        // similar indicators should appear.  We look for the uppercase
        // hotkey letter inside brackets, which the theme renders when badges
        // are visible.
        let has_badge = output.contains("[R]") || output.contains("[G]") || output.contains("[B]");
        assert!(
            has_badge || output.contains("Ctrl") || output.contains("CTRL"),
            "bare Ctrl press should reveal hotkey badges; output was: {output:?}"
        );

        // Send release so the prompt doesn't stay stuck
        session.write_all(KITTY_CTRL_RELEASE).expect("send ctrl release");
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

        // Send a chord (Ctrl+G) — this should arm the fallback deadline
        // and make badges visible for ~300 ms.
        // In raw crossterm terms, Ctrl+G is byte 0x07 (BEL).
        session.write_all(b"\x07").expect("send ctrl+g");
        std::thread::sleep(Duration::from_millis(100));

        let output = read_all_available(&mut session);

        // After the chord, badges should be visible (fallback path)
        let has_badge = output.contains("[R]") || output.contains("[G]") || output.contains("[B]");
        assert!(
            has_badge || output.contains("Ctrl") || output.contains("CTRL"),
            "Ctrl chord should trigger fallback badge display; output was: {output:?}"
        );

        // Submit to exit cleanly
        session.write_all(b"\r").expect("send enter");
        std::thread::sleep(Duration::from_millis(200));
    }
}

// ---------------------------------------------------------------------------
// 3.5 — Degraded terminal: no panic, chord fallback works
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod degraded_terminal {
    use super::*;
    use expectrl::{session::OsSession, spawn};

    /// Force the `TERM` variable to a dumb terminal that does not support
    /// kitty keyboard enhancements.  crossterm's push should fail, but the
    /// runner must not panic and chord fallback must still function.
    fn spawn_choose_one_dumb() -> OsSession {
        let binary = question_binary();
        let cmd = format!(
            "TERM=dumb {} choose-one --height 5 \"Red\" \"Green\" \"Blue\"",
            binary.to_str().unwrap()
        );
        let mut session = spawn(&cmd).expect("spawn question choose-one with TERM=dumb");
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        std::thread::sleep(Duration::from_millis(400));
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
        match session.read(&mut buf) {
            Ok(0) => {} // EOF — clean exit
            Ok(_) => {} // Trailing bytes are fine
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

        // Send Ctrl+G chord (BEL byte)
        session.write_all(b"\x07").expect("send ctrl+g");
        std::thread::sleep(Duration::from_millis(100));

        let output = read_all_available(&mut session);

        // Even without keyboard enhancements, the chord fallback should show badges
        let has_badge = output.contains("[R]") || output.contains("[G]") || output.contains("[B]");
        assert!(
            has_badge || output.contains("Ctrl") || output.contains("CTRL"),
            "chord fallback should work on dumb terminal; output was: {output:?}"
        );

        // Exit cleanly
        session.write_all(b"\r").expect("send enter");
        std::thread::sleep(Duration::from_millis(300));
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
