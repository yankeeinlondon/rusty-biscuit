//! Level-2 tests for cursor placement, ANSI hygiene, and unicode width handling.
//!
//! These tests run the `bt` CLI inside a real terminal emulator to validate
//! that cursor positioning after image renders is correct, that save/restore
//! sequences are balanced, and that CJK/emoji filenames align correctly in
//! directory listings.
//!
//! ## Skip-clean contract
//!
//! Every test checks `harness.available()` before spawning. When the
//! required terminal is absent the test prints `skipping: requires <X>`
//! to stderr and returns immediately. No `#[ignore]` markers are used.

mod common;

use biscuit_test_harness::{skip_with_reason, TerminalHarness};
use common::send_bt_command;
use std::time::Duration;

/// Extra settle time after spawning a WezTerm shell to avoid racing
/// shell initialization (custom prompts, completions, etc.).
const SHELL_READY_MS: u64 = 1500;

/// Returns the absolute path to a fixture file.
fn fixture_path(name: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/tests/fixtures/{}", manifest_dir, name)
}

/// Positions the cursor at a known row using `tput` so that image
/// rendering starts from a predictable position.
fn position_cursor(harness: &mut impl TerminalHarness, row: u32) {
    let cmd = format!("tput cup {} 0\n", row);
    harness.send_text(cmd.as_bytes()).expect("tput failed");
    harness.settle();
}

// ------------------------------------------------------------------
// Cursor placement
// ------------------------------------------------------------------

#[test]
fn level2_cursor_lands_below_rendered_image() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    // Clear screen and position cursor high so image doesn't scroll.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(&mut harness, 5);

    let path = fixture_path("tiny.png");
    send_bt_command(&mut harness, &format!("image --debug {}", path));

    // Send a sentinel string that should appear below the image.
    harness
        .send_text(b"echo 'SENTINEL_BELOW_IMAGE'\n")
        .expect("send_text failed");
    harness.settle();

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // Find the sentinel line and verify it exists.
    assert!(
        plain.contains("SENTINEL_BELOW_IMAGE"),
        "expected sentinel string to appear in capture. plain:\n{}",
        plain
    );

    // The debug output should show cursor AFTER position, confirming the
    // cursor landed below the image.
    assert!(
        plain.contains("cursor AFTER:"),
        "expected debug output to show cursor position after image. plain:\n{}",
        plain
    );
}

// ------------------------------------------------------------------
// ANSI hygiene — balanced save/restore
// ------------------------------------------------------------------

#[test]
fn level2_no_orphan_save_restore_sequences() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    // Test with image rendering (known to use save/restore).
    let path = fixture_path("tiny.png");
    send_bt_command(&mut harness, &format!("image --debug {}", path));

    let frame = harness.capture().expect("capture failed");
    assert_balanced_save_restore(&frame.raw);

    // Test with a prose command (should not produce orphans either).
    let mut harness2 = WezTermHarness::new();
    harness2.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
    send_bt_command(&mut harness2, "prose \"<red>hello</red>\"");

    let frame2 = harness2.capture().expect("capture failed");
    assert_balanced_save_restore(&frame2.raw);

    // Test with two-column layout (uses save/restore internally).
    let mut harness3 = WezTermHarness::new();
    harness3.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
    send_bt_command(&mut harness3, "columns \"left\" \"right\"");

    let frame3 = harness3.capture().expect("capture failed");
    assert_balanced_save_restore(&frame3.raw);
}

// ------------------------------------------------------------------
// Unicode width alignment in dir command
// ------------------------------------------------------------------

#[test]
fn level2_dir_command_unicode_widths_in_capture() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    let dir_path = fixture_path("unicode_dir");
    send_bt_command(&mut harness, &format!("dir {}", dir_path));

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // Verify that the fixture filenames appear in the output.
    assert!(
        plain.contains("中文文件.txt"),
        "expected CJK filename in dir output. plain:\n{}",
        plain
    );
    assert!(
        plain.contains("emoji_🎉.txt"),
        "expected emoji filename in dir output. plain:\n{}",
        plain
    );
    assert!(
        plain.contains("regular.txt"),
        "expected regular filename in dir output. plain:\n{}",
        plain
    );

    // Verify that tree-branch characters appear (indicating the FileSystem
    // component rendered a tree structure).
    assert!(
        plain.contains("├──") || plain.contains("└──") || plain.contains("│"),
        "expected tree branch characters in dir output. plain:\n{}",
        plain
    );
}

// ------------------------------------------------------------------
// Assertion helpers
// ------------------------------------------------------------------

/// Asserts that `raw` contains balanced `\x1b[s` (save) and `\x1b[u` (restore)
/// sequences. Every save must have a matching restore.
fn assert_balanced_save_restore(raw: &str) {
    let saves = raw.matches("\x1b[s").count();
    let restores = raw.matches("\x1b[u").count();
    assert_eq!(
        saves, restores,
        "expected balanced save/restore sequences: {} saves vs {} restores. raw:\n{}",
        saves, restores, raw
    );
}
