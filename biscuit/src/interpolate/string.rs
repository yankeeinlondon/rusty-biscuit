//! Simple string find/replace interpolation.
//!
//! This module provides context-unaware string replacement that operates on
//! the entire content without regard to document structure.

use std::borrow::Cow;

/// Simple string find/replace.
///
/// Replaces all occurrences of `find` with `replace` in the given content.
/// Returns `Cow::Borrowed` if no match is found, `Cow::Owned` if replacement occurred.
///
/// ## Examples
///
/// ```
/// use shared::interpolate::interpolate;
///
/// // Basic replacement
/// let result = interpolate("Hello world", "world", "Rust");
/// assert_eq!(result, "Hello Rust");
///
/// // No match returns borrowed reference
/// let result = interpolate("Hello world", "foo", "bar");
/// assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
/// assert_eq!(result, "Hello world");
///
/// // Multiple occurrences are all replaced
/// let result = interpolate("foo bar foo", "foo", "baz");
/// assert_eq!(result, "baz bar baz");
/// ```
pub fn interpolate<'a>(content: &'a str, find: &str, replace: &str) -> Cow<'a, str> {
    if content.contains(find) {
        Cow::Owned(content.replace(find, replace))
    } else {
        Cow::Borrowed(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_replacement() {
        let result = interpolate("Hello world", "world", "Rust");
        assert_eq!(result, "Hello Rust");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn test_no_match_returns_borrowed() {
        let result = interpolate("Hello world", "foo", "bar");
        assert_eq!(result, "Hello world");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_multiple_occurrences() {
        let result = interpolate("foo bar foo baz foo", "foo", "qux");
        assert_eq!(result, "qux bar qux baz qux");
    }

    #[test]
    fn test_empty_find_string() {
        // Empty string is found at every position
        let result = interpolate("abc", "", "X");
        assert_eq!(result, "XaXbXcX");
    }

    #[test]
    fn test_empty_content() {
        let result = interpolate("", "foo", "bar");
        assert_eq!(result, "");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_replace_with_empty() {
        let result = interpolate("Hello world", "world", "");
        assert_eq!(result, "Hello ");
    }

    #[test]
    fn test_replace_with_longer_string() {
        let result = interpolate("Hi", "Hi", "Hello there");
        assert_eq!(result, "Hello there");
    }

    #[test]
    fn test_case_sensitive() {
        let result = interpolate("Hello World", "world", "Rust");
        assert_eq!(result, "Hello World");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_special_characters() {
        let result = interpolate("foo $bar baz", "$bar", "qux");
        assert_eq!(result, "foo qux baz");
    }

    #[test]
    fn test_newlines_in_content() {
        let result = interpolate("line1\nline2\nline3", "line2", "replaced");
        assert_eq!(result, "line1\nreplaced\nline3");
    }

    #[test]
    fn test_unicode_content() {
        let result = interpolate("Hello \u{1F600} world", "\u{1F600}", "\u{1F389}");
        assert_eq!(result, "Hello \u{1F389} world");
    }

    #[test]
    fn test_overlapping_patterns() {
        // Non-overlapping replacement behavior
        let result = interpolate("aaa", "aa", "b");
        assert_eq!(result, "ba");
    }
}
