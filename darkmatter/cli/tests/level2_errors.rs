//! Level-2 tests for darkmatter error rendering (OSC 8, gutter style, etc.).

use biscuit_test_harness::{CapturedFrame, TerminalHarness, skip_with_reason};
use serial_test::serial;
use tempfile::tempdir;
use std::fs;

#[test]
#[serial(level2_terminal)]
fn level2_error_header_contains_osc8_hyperlink() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("unterminated.md");
    // Unterminated page block to trigger PageBlockError::UnterminatedBlock
    // (which includes the file path in the error header as an OSC8 hyperlink)
    fs::write(&file_path, "::block when=\"true\"\nbody\n").unwrap();

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Run `md compose` to trigger page-block parsing
    let cmd = format!("md compose {}", file_path.display());
    harness.send_command_with_env(&cmd, &[]).expect("send_command_with_env failed");
    
    // Wait for output
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let frame = harness.capture().expect("capture failed");
    
    // Verify OSC 8 hyperlink in the header
    // The header should contain something like \x1b]8;;file:///...unterminated.md\x07unterminated.md\x1b]8;;\x07
    // Use canonicalize because macOS /var is a symlink to /private/var.
    let canonical = file_path.canonicalize().expect("canonicalize failed");
    let expected_url = format!("file://{}", canonical.to_string_lossy());
    assert_osc8_link_present(&frame, &expected_url, "unterminated.md");
}

#[test]
#[serial(level2_terminal)]
fn level2_error_excerpt_contains_gutter_and_dimming() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("unterminated.md");
    // Unterminated block to trigger PageBlockError::UnterminatedBlock
    fs::write(&file_path, "::block when=\"true\"\nbody\n").unwrap();

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    let cmd = format!("md compose {}", file_path.display());
    harness.send_command_with_env(&cmd, &[]).expect("send_command_with_env failed");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let frame = harness.capture().expect("capture failed");
    
    // Verify gutter marker and line numbers
    assert!(frame.plain.contains("> 1 │ ::block"));
    
    // Verify dimming (if possible via hex check or similar)
    // <dim> maps to \x1b[2m or \x1b[0;2m depending on context
    let has_dim = frame.raw.contains("\x1b[2m") || frame.raw.contains("\x1b[0;2m");
    assert!(
        has_dim,
        "expected dimmed output in raw capture. raw:\n{}",
        frame.raw
    );
}

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
