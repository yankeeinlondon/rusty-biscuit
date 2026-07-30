// Each integration test in `cli/tests/` is compiled as its own crate
// and only consumes a subset of these shared helpers. That is the
// canonical Rust integration-test layout, but the dead-code lint does
// not understand it: from any single test crate's perspective, the
// helpers it does not call appear unused. Suppress the lint here so
// the shared module compiles cleanly under `-D warnings` regardless
// of which subset of helpers the consuming test reaches for.
#![allow(dead_code)]

use std::process::Output;

use assert_cmd::Command;

#[cfg(unix)]
pub mod pty;

#[cfg(unix)]
pub mod real_terminal;

/// Whether the `expect` driver [`run_question_in_pty`] shells out to is
/// reachable.
///
/// `expect` is a provisioned external tool, not something the test can supply:
/// macOS ships it, GitHub's Linux runners do not, and Windows has no port. Its
/// absence must therefore skip cleanly rather than fail.
///
/// ## Notes
///
/// These tests were green in CI for a long time only because their names contain
/// `real_tty`, which the then-unanchored `test(/real_/)` filterset matched by
/// accident. Anchoring that predicate to a path-segment boundary removed the
/// accident and exposed the dependency — they had never actually run.
pub fn expect_driver_available() -> bool {
    Command::new("expect")
        .arg("-v")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Drive `question` through a real PTY using the `expect` script driver.
///
/// ## Panics
///
/// Panics if `expect` is not on `PATH`. Callers must gate on
/// [`expect_driver_available`] via `require_level!(Level::L2, …)` first — the
/// resource is a real terminal, which is L2 by the tier taxonomy.
pub fn run_question_in_pty(args: &[&str], send_sequence: &str, expected_code: i32) -> Output {
    let binary = assert_cmd::cargo::cargo_bin("question");
    let script = build_expect_script(
        binary
            .to_str()
            .expect("question binary path should be valid UTF-8"),
        args,
        send_sequence,
    );

    let mut cmd = Command::new("expect");
    cmd.arg("-c").arg(script);
    let assert = cmd.assert().code(expected_code);
    assert.get_output().clone()
}

pub fn clean_terminal_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&strip_csi_sequences(bytes)).into_owned()
}

fn build_expect_script(binary: &str, args: &[&str], send_sequence: &str) -> String {
    let mut script = String::from("log_user 0\nset timeout 5\nset cmd [list ");
    script.push('"');
    script.push_str(&escape_tcl_double_quoted(binary, true));
    script.push('"');
    for arg in args {
        script.push(' ');
        script.push('"');
        script.push_str(&escape_tcl_double_quoted(arg, true));
        script.push('"');
    }
    script.push_str("]\neval spawn -noecho $cmd\nafter 300\nsend -- \"");
    script.push_str(&escape_tcl_double_quoted(send_sequence, false));
    script.push_str("\"\nexpect eof\nputs -nonewline $expect_out(buffer)\nset status [wait]\nexit [lindex $status 3]\n");
    script
}

fn escape_tcl_double_quoted(input: &str, escape_backslash: bool) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' if escape_backslash => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '$' => escaped.push_str("\\$"),
            '[' => escaped.push_str("\\["),
            ']' => escaped.push_str("\\]"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn strip_csi_sequences(bytes: &[u8]) -> Vec<u8> {
    let mut cleaned = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == 0x1b && index + 1 < bytes.len() && bytes[index + 1] == b'[' {
            index += 2;
            while index < bytes.len() {
                let byte = bytes[index];
                index += 1;
                if (0x40..=0x7e).contains(&byte) {
                    break;
                }
            }
            continue;
        }

        cleaned.push(bytes[index]);
        index += 1;
    }

    cleaned
}
