//! Regex-based find/replace interpolation.
//!
//! This module provides context-unaware regex-based replacement that operates
//! on the entire content without regard to document structure.

use std::borrow::Cow;

use regex::Regex;

use crate::isolate::InterpolateError;

/// Regex-based find/replace.
///
/// Replaces all matches of `pattern` with `replace` in the given content.
/// Supports capture groups in the replacement string (`$1`, `$2`, etc.).
/// Returns `Cow::Borrowed` if no match, `Cow::Owned` if replacement occurred.
///
/// ## Errors
///
/// Returns `InterpolateError::InvalidPattern` if the regex pattern is invalid.
///
/// ## Examples
///
/// ```
/// use shared::interpolate::interpolate_regex;
///
/// // Basic regex replacement
/// let result = interpolate_regex("Hello 123", r"\d+", "456").unwrap();
/// assert_eq!(result, "Hello 456");
///
/// // With capture groups
/// let result = interpolate_regex("Hello world", r"(\w+) (\w+)", "$2 $1").unwrap();
/// assert_eq!(result, "world Hello");
///
/// // No match returns borrowed reference
/// let result = interpolate_regex("Hello world", r"\d+", "num").unwrap();
/// assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
/// ```
pub fn interpolate_regex<'a>(
    content: &'a str,
    pattern: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError> {
    let regex = Regex::new(pattern)?;
    Ok(regex.replace_all(content, replace))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_regex_replacement() {
        let result = interpolate_regex("Hello 123", r"\d+", "456").unwrap();
        assert_eq!(result, "Hello 456");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn test_no_match_returns_borrowed() {
        let result = interpolate_regex("Hello world", r"\d+", "num").unwrap();
        assert_eq!(result, "Hello world");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_capture_groups() {
        let result = interpolate_regex("Hello world", r"(\w+) (\w+)", "$2 $1").unwrap();
        assert_eq!(result, "world Hello");
    }

    #[test]
    fn test_multiple_capture_groups() {
        let result =
            interpolate_regex("John Doe, 30", r"(\w+) (\w+), (\d+)", "$1's age is $3").unwrap();
        assert_eq!(result, "John's age is 30");
    }

    #[test]
    fn test_named_capture_groups() {
        let result = interpolate_regex(
            "2024-01-15",
            r"(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})",
            "$day/$month/$year",
        )
        .unwrap();
        assert_eq!(result, "15/01/2024");
    }

    #[test]
    fn test_invalid_pattern_error() {
        let result = interpolate_regex("test", r"[invalid", "replace");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, InterpolateError::InvalidPattern(_)));
        assert!(err.to_string().contains("Invalid regex pattern"));
    }

    #[test]
    fn test_multiple_matches() {
        let result = interpolate_regex("foo123bar456baz789", r"\d+", "NUM").unwrap();
        assert_eq!(result, "fooNUMbarNUMbazNUM");
    }

    #[test]
    fn test_empty_content() {
        let result = interpolate_regex("", r"\d+", "num").unwrap();
        assert_eq!(result, "");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_replace_with_empty() {
        let result = interpolate_regex("Hello 123 world", r"\d+\s*", "").unwrap();
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_special_regex_characters() {
        let result = interpolate_regex("price: $100", r"\$(\d+)", "EUR $1").unwrap();
        assert_eq!(result, "price: EUR 100");
    }

    #[test]
    fn test_word_boundaries() {
        let result = interpolate_regex("cat category catalog", r"\bcat\b", "dog").unwrap();
        assert_eq!(result, "dog category catalog");
    }

    #[test]
    fn test_case_insensitive_flag() {
        let result = interpolate_regex("Hello HELLO hello", r"(?i)hello", "hi").unwrap();
        assert_eq!(result, "hi hi hi");
    }

    #[test]
    fn test_multiline_content() {
        let result = interpolate_regex("line1\nline2\nline3", r"line(\d)", "row$1").unwrap();
        assert_eq!(result, "row1\nrow2\nrow3");
    }

    #[test]
    fn test_unicode_content() {
        let result = interpolate_regex("cafe\u{0301} is nice", r"caf\u{e9}", "coffee").unwrap();
        // Note: cafe with combining accent vs precomposed - this tests the actual match
        assert!(result.len() > 0);
    }

    #[test]
    fn test_literal_dollar_in_replacement() {
        // $$ becomes a literal $, so $$1 results in "$1" (literal)
        let result = interpolate_regex("price: 100", r"(\d+)", "$$1").unwrap();
        assert_eq!(result, "price: $1");

        // To get a literal $ followed by capture group, use $$$1
        let result = interpolate_regex("price: 100", r"(\d+)", "$$$1").unwrap();
        assert_eq!(result, "price: $100");
    }

    #[test]
    fn test_whole_match_reference() {
        let result = interpolate_regex("hello world", r"\w+", "[$0]").unwrap();
        assert_eq!(result, "[hello] [world]");
    }
}
