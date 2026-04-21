//! Shared helpers for the error snapshot integration tests.

use biscuit_terminal::errors::BlockError;

/// Strips CSI ANSI escape sequences so assertions run against plain text.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Renders `err` at 80 columns with an optimistic terminal, then strips ANSI.
pub fn render(err: &dyn BlockError) -> String {
    strip_ansi(&err.report_block_error_optimistic(Some(80)))
}

/// Asserts the rendered output contains every fragment in `must_contain`.
#[track_caller]
pub fn assert_contains_all(out: &str, must_contain: &[&str]) {
    for fragment in must_contain {
        assert!(
            out.contains(fragment),
            "expected output to contain `{fragment}`; got:\n{out}"
        );
    }
}
