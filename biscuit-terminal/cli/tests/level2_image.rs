//! Level-2 tests for image rendering.
//!
//! These tests run the `bt image` command inside a real terminal emulator
//! (WezTerm, Kitty) to validate that image protocol bytes are emitted
//! correctly and that scroll-compensation and rounding branches behave
//! as documented.
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
// Basic image rendering
// ------------------------------------------------------------------

#[test]
fn level2_image_renders_in_wezterm() {
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

    let frame = harness.capture().expect("capture failed");
    // Debug output is always captured; verify the image rendered for WezTerm.
    assert!(
        frame.plain.contains("--- image debug ---") && frame.plain.contains("Wezterm"),
        "expected debug output confirming image rendered. plain:\n{}",
        frame.plain
    );
}

#[test]
fn level2_image_renders_in_kitty() {
    use biscuit_test_harness::kitty::KittyHarness;

    if !KittyHarness::available() {
        skip_with_reason("Kitty remote control (set KITTY_LISTEN_ON)");
        return;
    }

    let mut harness = KittyHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    let path = fixture_path("tiny.png");
    send_bt_command(&mut harness, &format!("image {}", path));

    let frame = harness.capture().expect("capture failed");
    assert!(
        frame.raw.contains("\x1b_G"),
        "expected raw output to contain Kitty graphics protocol APC sequence (\\x1b_G). raw:\n{}",
        frame.raw
    );
}

// ------------------------------------------------------------------
// Scroll and rounding
// ------------------------------------------------------------------

#[test]
fn level2_image_scroll_compensation_at_bottom_margin() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    let path = fixture_path("tiny.png");
    // Clear the screen and position cursor near bottom to trigger scroll.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(&mut harness, 22);
    send_bt_command(&mut harness, &format!("image --debug {}", path));

    let frame = harness.capture().expect("capture failed");
    // The debug output should show that scroll compensation was predicted.
    assert!(
        frame.plain.contains("SCROLL needed"),
        "expected debug output to indicate scroll compensation at bottom margin. plain:\n{}",
        frame.plain
    );
}

#[test]
fn level2_warp_uses_floor_rounding() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    // Spoof Warp detection so bt uses floor rounding.
    harness
        .send_text(b"export TERM_PROGRAM=WarpTerminal\n")
        .expect("send_text failed");
    harness.settle();

    // Clear screen and position cursor high so image doesn't scroll.
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    position_cursor(&mut harness, 5);

    let path = fixture_path("tiny.png");
    send_bt_command(&mut harness, &format!("image --debug {}", path));

    let frame = harness.capture().expect("capture failed");
    // Debug output for Warp should show the app name and floor value.
    assert!(
        frame.plain.contains("Warp") && frame.plain.contains("floor="),
        "expected debug output to show Warp app detection and floor rounding. plain:\n{}",
        frame.plain
    );
}

#[test]
fn level2_image_meta_to_stderr() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    let path = fixture_path("tiny.png");
    send_bt_command(&mut harness, &format!("image --meta {}", path));

    let frame = harness.capture().expect("capture failed");
    // stderr metadata is merged into the pane capture. The JSON may be
    // wrapped across lines, so join before searching.
    let joined: String = frame.plain.lines().collect();
    assert!(
        joined.contains("\"cache_hit\"") && joined.contains("\"render_time_ms\""),
        "expected stderr metadata to contain JSON keys. plain:\n{}",
        frame.plain
    );
}
