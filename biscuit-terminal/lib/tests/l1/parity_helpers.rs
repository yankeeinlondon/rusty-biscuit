//! Shared test utilities for component parity testing.
//!
//! All Group 1 component parity tests import from this module for
//! consistent test infrastructure.

use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;

/// Creates an optimistic terminal with specified width.
///
/// This is a convenience wrapper for tests that need consistent
/// terminal configuration. The optimistic terminal has full capabilities
/// (Kitty image support, TrueColor, italic, OSC8 links, etc.) and a
/// fixed width, making tests deterministic across environments.
pub fn test_terminal(width: u32) -> Terminal {
    Terminal::new_optimistic(width)
}

/// Standard width matrix for parity testing.
///
/// Tests should exercise components at multiple widths to catch
/// wrapping and layout edge cases. These values represent:
/// - 40: Narrow terminal (mobile, split panes)
/// - 80: Standard terminal width
/// - 120: Wide terminal (modern displays)
pub const PARITY_WIDTHS: &[u32] = &[40, 80, 120];

/// Strips ANSI escape codes from output for content comparison.
///
/// Delegates to `strip_escape_codes` from biscuit-terminal, which
/// handles CSI, OSC (with both BEL and ST terminators), and Fe escape
/// sequences.
pub fn strip_ansi(s: &str) -> String {
    strip_escape_codes(s)
}

/// Normalizes whitespace for semantic comparison.
///
/// - Trims leading/trailing whitespace from each line
/// - Filters out empty lines
/// - Joins remaining lines with newlines
///
/// Useful for comparing rendered output where exact spacing may differ
/// but semantic content should match.
pub fn normalize_whitespace(s: &str) -> String {
    s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Asserts that two strings contain the same tokens after normalization.
///
/// Both strings are stripped of ANSI escape codes and whitespace-normalized
/// before comparison. Useful for semantic parity where exact formatting
/// may differ between renderers.
///
/// ## Panics
///
/// Panics if the normalized content differs, with a message showing both
/// the actual and expected normalized strings.
pub fn assert_semantic_match(actual: &str, expected: &str) {
    let actual_normalized = normalize_whitespace(&strip_ansi(actual));
    let expected_normalized = normalize_whitespace(&strip_ansi(expected));
    assert_eq!(
        actual_normalized, expected_normalized,
        "Semantic content mismatch:\nActual: {}\nExpected: {}",
        actual_normalized, expected_normalized
    );
}

/// Asserts content contains expected tokens (order-independent).
///
/// Strips ANSI escape codes before checking for each token. Each token
/// must appear somewhere in the stripped content, but ordering is not
/// enforced.
///
/// ## Panics
///
/// Panics if any token is not found, with a message identifying the
/// missing token and showing the stripped content.
pub fn assert_contains_tokens(content: &str, tokens: &[&str]) {
    let stripped = strip_ansi(content);
    for token in tokens {
        assert!(
            stripped.contains(token),
            "Expected token '{}' not found in:\n{}",
            token,
            stripped
        );
    }
}

// ---------------------------------------------------------------------------
// Unit tests for helper functions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // strip_ansi tests
    // -----------------------------------------------------------------------

    #[test]
    fn strip_ansi_removes_sgr_sequences() {
        let styled = "\x1b[31mred\x1b[0m text \x1b[1;4mbold underline\x1b[0m";
        let plain = strip_ansi(styled);
        assert_eq!(plain, "red text bold underline");
    }

    #[test]
    fn strip_ansi_removes_osc8_links() {
        let linked = "\x1b]8;;https://example.com\x07click here\x1b]8;;\x07";
        let plain = strip_ansi(linked);
        assert_eq!(plain, "click here");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        let plain = "Hello, world!";
        assert_eq!(strip_ansi(plain), plain);
    }

    #[test]
    fn strip_ansi_handles_empty_string() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn strip_ansi_removes_256_color_codes() {
        let colored = "\x1b[38;5;196mred 256\x1b[0m";
        assert_eq!(strip_ansi(colored), "red 256");
    }

    #[test]
    fn strip_ansi_removes_truecolor_codes() {
        let colored = "\x1b[38;2;255;0;0mtrue red\x1b[0m";
        assert_eq!(strip_ansi(colored), "true red");
    }

    // -----------------------------------------------------------------------
    // normalize_whitespace tests
    // -----------------------------------------------------------------------

    #[test]
    fn normalize_whitespace_trims_lines() {
        let input = "  hello  \n  world  ";
        assert_eq!(normalize_whitespace(input), "hello\nworld");
    }

    #[test]
    fn normalize_whitespace_removes_empty_lines() {
        let input = "line1\n\n\nline2\n\n";
        assert_eq!(normalize_whitespace(input), "line1\nline2");
    }

    #[test]
    fn normalize_whitespace_handles_single_line() {
        let input = "  single line  ";
        assert_eq!(normalize_whitespace(input), "single line");
    }

    #[test]
    fn normalize_whitespace_handles_empty_input() {
        assert_eq!(normalize_whitespace(""), "");
    }

    #[test]
    fn normalize_whitespace_handles_only_whitespace() {
        assert_eq!(normalize_whitespace("   \n\n   \n  "), "");
    }

    #[test]
    fn normalize_whitespace_preserves_internal_spaces() {
        let input = "hello   world";
        // Internal spaces are preserved by trim, only leading/trailing removed
        assert_eq!(normalize_whitespace(input), "hello   world");
    }

    // -----------------------------------------------------------------------
    // assert_semantic_match tests
    // -----------------------------------------------------------------------

    #[test]
    fn semantic_match_passes_for_identical_content() {
        assert_semantic_match("hello world", "hello world");
    }

    #[test]
    fn semantic_match_passes_ignoring_ansi() {
        assert_semantic_match("\x1b[31mhello\x1b[0m world", "hello world");
    }

    #[test]
    fn semantic_match_passes_ignoring_whitespace_differences() {
        assert_semantic_match("  hello  \n  world  ", "hello\nworld");
    }

    #[test]
    #[should_panic(expected = "Semantic content mismatch")]
    fn semantic_match_fails_for_different_content() {
        assert_semantic_match("hello", "world");
    }

    // -----------------------------------------------------------------------
    // assert_contains_tokens tests
    // -----------------------------------------------------------------------

    #[test]
    fn contains_tokens_passes_when_all_present() {
        let content = "The quick brown fox jumps over the lazy dog";
        assert_contains_tokens(content, &["quick", "fox", "dog"]);
    }

    #[test]
    fn contains_tokens_ignores_ansi_codes() {
        let content = "\x1b[31mred\x1b[0m and \x1b[34mblue\x1b[0m";
        assert_contains_tokens(content, &["red", "blue"]);
    }

    #[test]
    #[should_panic(expected = "Expected token 'missing' not found")]
    fn contains_tokens_fails_when_token_missing() {
        assert_contains_tokens("hello world", &["hello", "missing"]);
    }

    #[test]
    fn contains_tokens_handles_empty_token_list() {
        // Empty token list should pass for any content
        assert_contains_tokens("any content", &[]);
    }

    // -----------------------------------------------------------------------
    // test_terminal tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_terminal_has_correct_width() {
        let term = test_terminal(100);
        assert_eq!(term.width(), 100);
    }

    #[test]
    fn test_terminal_has_full_capabilities() {
        use biscuit_terminal::discovery::detection::{ColorDepth, ImageSupport};

        let term = test_terminal(80);
        assert_eq!(term.image_support, ImageSupport::Kitty);
        assert_eq!(term.color_depth, ColorDepth::TrueColor);
        assert!(term.supports_italic);
        assert!(term.osc_link_support);
    }

    // -----------------------------------------------------------------------
    // PARITY_WIDTHS tests
    // -----------------------------------------------------------------------

    #[test]
    fn parity_widths_contains_standard_values() {
        assert!(PARITY_WIDTHS.contains(&40));
        assert!(PARITY_WIDTHS.contains(&80));
        assert!(PARITY_WIDTHS.contains(&120));
    }

    #[test]
    fn parity_widths_has_three_entries() {
        assert_eq!(PARITY_WIDTHS.len(), 3);
    }
}
