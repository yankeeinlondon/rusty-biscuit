//! Level-2 tests for prose styling, OSC8 hyperlinks, NO_COLOR, and layout.
//!
//! These tests run the `bt` CLI inside a real terminal emulator so that
//! escape-sequence output is validated against the actual terminal's
//! display path.
//!
//! ## Skip-clean contract
//!
//! Every test checks `harness.available()` before spawning. When the
//! required terminal is absent the test prints `skipping: requires <X>`
//! to stderr and returns immediately. No `#[ignore]` markers are used.

mod common;

use biscuit_test_harness::{skip_with_reason, CapturedFrame, TerminalHarness};
use common::send_bt_command;

// ------------------------------------------------------------------
// WezTerm — SGR, OSC8, NO_COLOR
// ------------------------------------------------------------------

#[test]
fn level2_prose_emits_sgr_in_real_terminal() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(&mut harness, "prose \"<red>x</red>\"");
    // Give WezTerm extra time to render the color sequence before capture.
    std::thread::sleep(std::time::Duration::from_millis(500));

    let frame = harness.capture().expect("capture failed");
    assert_sgr_red_present(&frame);
}

#[test]
fn level2_prose_osc8_link_renders() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(
        &mut harness,
        "prose \"<a href=https://example.com>link</a>\"",
    );

    let frame = harness.capture().expect("capture failed");
    assert_osc8_link_present(&frame, "https://example.com", "link");
}

#[test]
fn level2_no_color_strips_sgr_in_real_terminal() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Prefix the command with NO_COLOR=1 so the shell exports it for bt.
    harness
        .send_text(b"NO_COLOR=1 bt prose \"<red>x</red>\"\n")
        .expect("send_text failed");
    harness.settle();

    let frame = harness.capture().expect("capture failed");
    assert_no_sgr_red(&frame);
}

// ------------------------------------------------------------------
// Kitty — SGR, OSC8
// ------------------------------------------------------------------

#[test]
fn level2_prose_emits_sgr_in_kitty() {
    use biscuit_test_harness::kitty::KittyHarness;

    if !KittyHarness::available() {
        skip_with_reason("Kitty remote control (set KITTY_LISTEN_ON)");
        return;
    }

    let mut harness = KittyHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(&mut harness, "prose \"<red>x</red>\"");

    let frame = harness.capture().expect("capture failed");
    assert_sgr_red_present(&frame);
}

#[test]
fn level2_prose_osc8_link_renders_in_kitty() {
    use biscuit_test_harness::kitty::KittyHarness;

    if !KittyHarness::available() {
        skip_with_reason("Kitty remote control (set KITTY_LISTEN_ON)");
        return;
    }

    let mut harness = KittyHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(
        &mut harness,
        "prose \"<a href=https://example.com>link</a>\"",
    );

    let frame = harness.capture().expect("capture failed");
    assert_osc8_link_present(&frame, "https://example.com", "link");
}

// ------------------------------------------------------------------
// Layout — padleft, columns
// ------------------------------------------------------------------

#[test]
fn level2_pad_columns_respect_actual_pane_width() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(&mut harness, "padleft 30 \"x\"");

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // Find a line where 'x' is the last non-space character and there are leading spaces.
    let line = plain
        .lines()
        .find(|l| {
            let trimmed_end = l.trim_end();
            trimmed_end.ends_with('x')
        })
        .expect("expected a line containing padded 'x'");

    // The line should have 29 spaces before the x, making the x the 30th column.
    let x_pos = line.chars().position(|c| c == 'x').expect("x should be present");
    assert_eq!(x_pos, 29, "expected 'x' at column 30 (index 29), got index {}", x_pos);
}

#[test]
fn level2_columns_word_wrap_in_pane() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Use a long word in the right column so it must wrap in a narrow pane.
    let long_word = "supercalifragilisticexpialidocious";
    send_bt_command(
        &mut harness,
        &format!("columns \"Title\" \"{}\"", long_word),
    );

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // The long word should appear somewhere in the captured text (possibly wrapped).
    let combined: String = plain.lines().collect();
    assert!(
        combined.contains(long_word),
        "expected captured text to contain the long word"
    );

    // Word wrap implies the text occupies more than one line.
    assert!(
        plain.lines().count() > 1,
        "expected multiple lines due to word wrap"
    );
}

// ------------------------------------------------------------------
// Assertion helpers
// ------------------------------------------------------------------

/// Asserts that `frame.raw` contains an SGR red sequence (either 31 or 91).
fn assert_sgr_red_present(frame: &CapturedFrame) {
    let has_31 = frame.raw.contains("\x1b[31m");
    let has_91 = frame.raw.contains("\x1b[91m");
    assert!(
        has_31 || has_91,
        "expected raw output to contain SGR red sequence (\\x1b[31m or \\x1b[91m). raw:\n{}",
        frame.raw
    );
}

/// Asserts that `frame.raw` contains an OSC8 hyperlink sequence for `url`
/// and that `frame.plain` contains the visible `label` text.
fn assert_osc8_link_present(frame: &CapturedFrame, url: &str, label: &str) {
    assert!(
        frame.raw.contains(&format!("\x1b]8;;{}", url)),
        "expected raw output to contain OSC8 hyperlink sequence for {}. raw:\n{}",
        url,
        frame.raw
    );
    assert!(
        frame.plain.contains(label),
        "expected plain text to contain label '{}'. plain:\n{}",
        label,
        frame.plain
    );
}

/// Asserts that `frame.raw` does NOT contain SGR red sequences
/// in the bt command output (as opposed to the shell prompt).
fn assert_no_sgr_red(frame: &CapturedFrame) {
    // Find the command line and check only the output that follows it.
    let lines: Vec<&str> = frame.raw.lines().collect();
    let mut found_cmd = false;
    for line in lines {
        if line.contains("NO_COLOR=1") && line.contains("bt prose") {
            found_cmd = true;
            continue;
        }
        if found_cmd && !line.trim().is_empty() {
            // This should be the bt output line.
            assert!(
                !line.contains("\x1b[31m"),
                "expected NO \\x1b[31m with NO_COLOR=1. line:\n{}",
                line
            );
            assert!(
                !line.contains("\x1b[91m"),
                "expected NO \\x1b[91m with NO_COLOR=1. line:\n{}",
                line
            );
            return;
        }
    }
    // If we couldn't isolate the output, fall back to checking the whole capture.
    assert!(
        !frame.raw.contains("\x1b[31m"),
        "expected NO \\x1b[31m with NO_COLOR=1. raw:\n{}",
        frame.raw
    );
    assert!(
        !frame.raw.contains("\x1b[91m"),
        "expected NO \\x1b[91m with NO_COLOR=1. raw:\n{}",
        frame.raw
    );
}
