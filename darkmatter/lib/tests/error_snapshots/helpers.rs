//! Shared helpers for the error snapshot integration tests.

use biscuit_terminal::errors::BlockError;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;

/// Strips CSI ANSI escape sequences so assertions run against plain text.
pub fn strip_ansi(s: &str) -> String {
    strip_escape_codes(s)
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
