//! Level-1 PTY tests for OSC52 clipboard.
//!
//! Verifies that `set_clipboard` emits the correct OSC52 escape sequence
//! when running inside a PTY with a supported terminal.
//!
//! Run `cargo build -p biscuit-terminal --example discovery_probe` first.

mod common;

use std::time::Duration;

use common::pty::{drive_probe, spawn_with_env};

fn probe_output(session: &mut expectrl::session::OsSession, completion_marker: &str) -> String {
    String::from_utf8_lossy(&drive_probe(
        session,
        &mut [],
        completion_marker,
        Duration::from_secs(3),
    ))
    .into_owned()
}

#[test]
fn osc52_sequence_emitted_to_tty() {
    let mut session = spawn_with_env(&[("PROBE", "clipboard"), ("PROBE_TERM_PROGRAM", "WezTerm")]);

    let output = probe_output(&mut session, "clipboard_result=");

    // OSC52 write sequence:
    // ESC ] 52 ; c ; <base64> BEL
    // base64("hello-pty") = "aGVsbG8tcHR5"
    assert!(
        output.contains("\x1b]52;c;"),
        "expected OSC52 header in output, got: {output:?}"
    );
    assert!(
        output.contains("aGVsbG8tcHR5"),
        "expected base64 payload in output, got: {output:?}"
    );
    assert!(
        output.contains("\x07"),
        "expected BEL terminator in output, got: {output:?}"
    );
}

#[test]
fn osc52_support_returns_true_in_supported_terminal() {
    let mut session = spawn_with_env(&[
        ("PROBE", "clipboard_support"),
        ("PROBE_TERM_PROGRAM", "kitty"),
    ]);

    let output = probe_output(&mut session, "osc52_support=");

    assert!(
        output.contains("osc52_support=true"),
        "expected osc52_support=true for kitty, got: {output}"
    );
}

#[test]
fn osc52_support_returns_false_in_unknown_terminal() {
    let mut session = spawn_with_env(&[
        ("PROBE", "clipboard_support"),
        ("PROBE_TERM_PROGRAM", "UnknownTerm"),
    ]);

    let output = probe_output(&mut session, "osc52_support=");

    assert!(
        output.contains("osc52_support=false"),
        "expected osc52_support=false for unknown terminal, got: {output}"
    );
}

#[test]
fn set_clipboard_with_target_emits_targeted_sequence() {
    let mut session = spawn_with_env(&[
        ("PROBE", "clipboard_target"),
        ("PROBE_TERM_PROGRAM", "WezTerm"),
    ]);

    let output = probe_output(&mut session, "clipboard_target_result=");

    // Primary target specifier: ;p;
    assert!(
        output.contains("\x1b]52;p;"),
        "expected OSC52 primary target header in output, got: {output:?}"
    );
    // base64("primary-pty") = "cHJpbWFyeS1wdHk="
    assert!(
        output.contains("cHJpbWFyeS1wdHk="),
        "expected base64 payload in output, got: {output:?}"
    );
    assert!(
        output.contains("\x07"),
        "expected BEL terminator in output, got: {output:?}"
    );
}

#[test]
fn clear_clipboard_emits_clear_sequence() {
    let mut session = spawn_with_env(&[
        ("PROBE", "clipboard_clear"),
        ("PROBE_TERM_PROGRAM", "WezTerm"),
    ]);

    let output = probe_output(&mut session, "clipboard_clear_result=");

    // OSC52 clear sequence: ESC ] 52 ; c ; ! BEL
    assert!(
        output.contains("\x1b]52;c;!\x07"),
        "expected OSC52 clear sequence in output, got: {output:?}"
    );
}

#[test]
fn get_clipboard_returns_none_in_pty() {
    let mut session = spawn_with_env(&[
        ("PROBE", "clipboard_get"),
        ("PROBE_TERM_PROGRAM", "WezTerm"),
    ]);

    let output = probe_output(&mut session, "clipboard_get=");

    assert!(
        output.contains("clipboard_get=None"),
        "expected clipboard_get=None in output, got: {output}"
    );
}
