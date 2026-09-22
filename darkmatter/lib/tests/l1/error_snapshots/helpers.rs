//! Shared helpers for the error snapshot integration tests.

use biscuit_terminal::errors::{BlockError, SourceContext};
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use std::path::PathBuf;

/// Builds a `SourceContext` for tests with sane defaults.
///
/// `display` is used both as the relative display path and joined under
/// `/tmp/test/` for the absolute path.
pub fn test_ctx(content: &str, display: &str) -> SourceContext {
    SourceContext::new(
        PathBuf::from("/tmp/test").join(display),
        PathBuf::from(display),
        content.to_string(),
    )
}

/// Builds a `SourceContext` with `n_lines` of placeholder content so excerpt
/// rendering at any line in `1..=n_lines` produces a non-empty gutter.
pub fn test_ctx_lines(n_lines: usize, display: &str) -> SourceContext {
    let content: String = (1..=n_lines)
        .map(|i| format!("line {i}\n"))
        .collect::<String>();
    test_ctx(&content, display)
}

/// Strips CSI ANSI escape sequences so assertions run against plain text.
pub fn strip_ansi(s: &str) -> String {
    strip_escape_codes(s)
}

/// Renders `err` at 80 columns with an optimistic terminal, then strips ANSI.
pub fn render(err: &dyn BlockError) -> String {
    strip_ansi(&err.report_block_error_optimistic(Some(80)))
}

/// Renders `err` at 80 columns with an optimistic terminal, preserving ANSI
/// escape sequences (OSC 8 hyperlinks, SGR styling). Used for assertions on
/// styling that the plain-text [`render`] helper would strip.
pub fn render_with_ansi(err: &dyn BlockError) -> String {
    err.report_block_error_optimistic(Some(80))
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
