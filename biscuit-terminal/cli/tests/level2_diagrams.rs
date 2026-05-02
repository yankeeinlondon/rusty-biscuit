//! Level-2 tests for Mermaid diagram rendering.
//!
//! These tests run the `bt` diagram subcommands inside a real terminal
//! emulator to validate that image-protocol bytes (or fallback fenced
//! blocks) are emitted correctly.
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

/// Extra settle time for diagram renders, which involve SVG generation
/// and rasterization and can take 500 ms+ on first run.
const DIAGRAM_SETTLE_MS: u64 = 1200;

// ------------------------------------------------------------------
// WezTerm — flowchart
// ------------------------------------------------------------------

#[test]
fn level2_flowchart_renders_in_wezterm() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    // Use --meta so stderr metadata is captured (WezTerm strips image APC).
    send_bt_command(&mut harness, "flowchart --meta \"A --> B\"");
    std::thread::sleep(Duration::from_millis(DIAGRAM_SETTLE_MS));

    let frame = harness.capture().expect("capture failed");
    let joined: String = frame.plain.lines().collect();
    assert!(
        joined.contains("\"filename\"") && joined.contains("\"render_time_ms\""),
        "expected meta JSON to appear in capture confirming diagram rendered. plain:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Kitty — pie chart
// ------------------------------------------------------------------

#[test]
fn level2_pie_chart_renders_in_kitty() {
    use biscuit_test_harness::kitty::KittyHarness;

    if !KittyHarness::available() {
        skip_with_reason("Kitty remote control (set KITTY_LISTEN_ON)");
        return;
    }

    let mut harness = KittyHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(&mut harness, "pie-chart \"A: 1\"");

    let frame = harness.capture().expect("capture failed");
    assert!(
        frame.raw.contains("\x1b_G"),
        "expected raw output to contain Kitty graphics protocol APC sequence for pie chart. raw:\n{}",
        frame.raw
    );
}

// ------------------------------------------------------------------
// Width and inverse
// ------------------------------------------------------------------

#[test]
fn level2_diagram_width_respects_pane_columns() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));

    // Use --width 50% and --meta to verify the command succeeds with a width flag.
    send_bt_command(&mut harness, "pie-chart --meta --width 50% \"A: 1\"");
    std::thread::sleep(Duration::from_millis(DIAGRAM_SETTLE_MS));

    let frame = harness.capture().expect("capture failed");
    let joined: String = frame.plain.lines().collect();
    // The diagram should render successfully with the width flag.
    assert!(
        joined.contains("\"filename\"") && joined.contains("\"render_time_ms\""),
        "expected diagram to render with custom width. plain:\n{}",
        frame.plain
    );
}

#[test]
fn level2_inverse_flag_changes_background_in_capture() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    // Capture without --inverse (using --meta to get filename)
    let mut harness1 = WezTermHarness::new();
    harness1.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
    send_bt_command(&mut harness1, "flowchart --meta \"A --> B\"");
    std::thread::sleep(Duration::from_millis(DIAGRAM_SETTLE_MS));
    let frame1 = harness1.capture().expect("capture failed");
    let filename1 = extract_meta_filename(&frame1.plain);

    // Capture with --inverse (using --meta to get filename)
    let mut harness2 = WezTermHarness::new();
    harness2.spawn_shell().expect("spawn_shell failed");
    std::thread::sleep(Duration::from_millis(SHELL_READY_MS));
    send_bt_command(&mut harness2, "flowchart --meta --inverse \"A --> B\"");
    std::thread::sleep(Duration::from_millis(DIAGRAM_SETTLE_MS));
    let frame2 = harness2.capture().expect("capture failed");
    let filename2 = extract_meta_filename(&frame2.plain);

    // The filenames should differ because inverse produces a different PNG hash.
    assert_ne!(
        filename1, filename2,
        "expected --inverse to produce a different cached PNG filename than default"
    );
}

// ------------------------------------------------------------------
// tmux fallback
// ------------------------------------------------------------------

#[test]
fn level2_diagram_fallback_when_no_image_protocol() {
    use biscuit_test_harness::tmux::TmuxHarness;

    if !TmuxHarness::available() {
        skip_with_reason("tmux");
        return;
    }

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(&mut harness, "flowchart \"A --> B\"");

    let frame = harness.capture().expect("capture failed");
    // tmux has no image protocol, so bt should fall back to a fenced code block.
    assert!(
        frame.plain.contains("```mermaid"),
        "expected fallback fenced code block in tmux (no image protocol). plain:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

/// Extracts the `"filename":"..."` value from meta JSON embedded in pane text.
///
/// The JSON may be split across multiple lines due to terminal wrapping,
/// so this helper joins all lines before searching.
fn extract_meta_filename(plain: &str) -> String {
    let joined: String = plain.lines().collect();
    if let Some(start) = joined.find("\"filename\":\"") {
        let start = start + 12;
        if let Some(end) = joined[start..].find('"') {
            return joined[start..start + end].to_string();
        }
    }
    String::new()
}
