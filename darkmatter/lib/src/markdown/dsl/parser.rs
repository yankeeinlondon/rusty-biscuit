//! Parser for code block DSL syntax.
//!
//! Parses info strings like:
//! - `rust`
//! - `ts title="Main"`
//! - `js line-numbering=true highlight=1,4-6`

use super::{CodeBlockMeta, HighlightSpec, parse_highlight_spec};
use crate::markdown::MarkdownError;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex for matching key=value pairs.
    /// Supports: key="value", key='value', key=value
    /// Keys can contain alphanumeric characters, underscores, and hyphens
    static ref KV_PATTERN: Regex = Regex::new(
        r#"([\w-]+)=(?:"([^"]*)"|'([^']*)'|(\S+))"#
    ).unwrap();
}

/// Parses a code block info string into metadata.
///
/// ## Format
///
/// The info string consists of:
/// 1. First whitespace-delimited token: language identifier
/// 2. Remaining tokens: key=value pairs
///
/// Supported keys:
/// - `title`: Code block title (string)
/// - `line-numbering`: Whether to show line numbers (boolean)
/// - `highlight`: Lines to highlight (comma-separated numbers and ranges)
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::dsl::parse_code_info;
///
/// // Language only
/// let meta = parse_code_info("rust").unwrap();
/// assert_eq!(meta.language, "rust");
/// assert!(meta.title.is_none());
///
/// // With title
/// let meta = parse_code_info(r#"ts title="Main function""#).unwrap();
/// assert_eq!(meta.language, "ts");
/// assert_eq!(meta.title, Some("Main function".to_string()));
///
/// // With line numbering and highlighting
/// let meta = parse_code_info("js line-numbering=true highlight=1,4-6").unwrap();
/// assert_eq!(meta.language, "js");
/// assert!(meta.line_numbering);
/// assert!(meta.highlight.contains(1));
/// assert!(meta.highlight.contains(5));
/// ```
///
/// ## Errors
///
/// Returns an error if:
/// - Highlight ranges are invalid (start > end)
/// - Highlight values are not valid numbers
pub fn parse_code_info(info_string: &str) -> Result<CodeBlockMeta, MarkdownError> {
    let info_string = info_string.trim();
    if info_string.is_empty() {
        return Ok(CodeBlockMeta::default());
    }

    // Extract language (first whitespace-delimited token)
    let parts: Vec<&str> = info_string.splitn(2, char::is_whitespace).collect();
    let language = parts[0].to_string();
    let remainder = parts.get(1).copied().unwrap_or("");

    let mut meta = CodeBlockMeta {
        language,
        ..Default::default()
    };

    // Parse key=value pairs
    for captures in KV_PATTERN.captures_iter(remainder) {
        let key = captures.get(1).unwrap().as_str();
        // Try each capture group (double-quoted, single-quoted, unquoted)
        let value = captures
            .get(2)
            .or_else(|| captures.get(3))
            .or_else(|| captures.get(4))
            .unwrap()
            .as_str();

        match key {
            "title" => {
                meta.title = Some(value.to_string());
            }
            "line-numbering" => {
                meta.line_numbering = parse_bool(value);
            }
            "highlight" => {
                meta.highlight = parse_highlight(value)?;
            }
            _ => {
                // Store unknown keys in custom map for extensibility
                meta.custom.insert(key.to_string(), value.to_string());
            }
        }
    }

    Ok(meta)
}

/// Parses a boolean value from string.
fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "1" | "yes" | "on")
}

/// Parses highlight specification from string.
///
/// Format: comma-separated list of numbers and ranges.
/// Examples: "1", "1,3,5", "1-3", "1,4-6,10"
fn parse_highlight(s: &str) -> Result<HighlightSpec, MarkdownError> {
    parse_highlight_spec(s).map_err(|e| MarkdownError::InvalidLineRange(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::dsl::{HighlightSpecParseError, ValidLineRange};

    #[test]
    fn test_parse_empty_string() {
        let meta = parse_code_info("").unwrap();
        assert_eq!(meta.language, "");
        assert!(meta.title.is_none());
        assert!(!meta.line_numbering);
        assert!(meta.highlight.is_empty());
    }

    #[test]
    fn test_parse_language_only() {
        let meta = parse_code_info("rust").unwrap();
        assert_eq!(meta.language, "rust");
        assert!(meta.title.is_none());
        assert!(!meta.line_numbering);
        assert!(meta.highlight.is_empty());
    }

    #[test]
    fn test_parse_with_title_double_quotes() {
        let meta = parse_code_info(r#"ts title="Main function""#).unwrap();
        assert_eq!(meta.language, "ts");
        assert_eq!(meta.title, Some("Main function".to_string()));
    }

    #[test]
    fn test_parse_with_title_single_quotes() {
        let meta = parse_code_info(r#"js title='Helper'"#).unwrap();
        assert_eq!(meta.language, "js");
        assert_eq!(meta.title, Some("Helper".to_string()));
    }

    #[test]
    fn test_parse_with_title_unquoted() {
        let meta = parse_code_info("python title=Main").unwrap();
        assert_eq!(meta.language, "python");
        assert_eq!(meta.title, Some("Main".to_string()));
    }

    #[test]
    fn test_parse_line_numbering_true() {
        let meta = parse_code_info("rust line-numbering=true").unwrap();
        assert!(meta.line_numbering);
    }

    #[test]
    fn test_parse_line_numbering_false() {
        let meta = parse_code_info("rust line-numbering=false").unwrap();
        assert!(!meta.line_numbering);
    }

    #[test]
    fn test_parse_line_numbering_variants() {
        assert!(
            parse_code_info("rust line-numbering=1")
                .unwrap()
                .line_numbering
        );
        assert!(
            parse_code_info("rust line-numbering=yes")
                .unwrap()
                .line_numbering
        );
        assert!(
            parse_code_info("rust line-numbering=on")
                .unwrap()
                .line_numbering
        );
        assert!(
            !parse_code_info("rust line-numbering=no")
                .unwrap()
                .line_numbering
        );
        assert!(
            !parse_code_info("rust line-numbering=0")
                .unwrap()
                .line_numbering
        );
    }

    #[test]
    fn test_parse_highlight_single_line() {
        let meta = parse_code_info("rust highlight=5").unwrap();
        assert!(meta.highlight.contains(5));
        assert!(!meta.highlight.contains(4));
        assert!(!meta.highlight.contains(6));
    }

    #[test]
    fn test_parse_highlight_multiple_lines() {
        let meta = parse_code_info("rust highlight=1,3,5").unwrap();
        assert!(meta.highlight.contains(1));
        assert!(meta.highlight.contains(3));
        assert!(meta.highlight.contains(5));
        assert!(!meta.highlight.contains(2));
        assert!(!meta.highlight.contains(4));
    }

    #[test]
    fn test_parse_highlight_range() {
        let meta = parse_code_info("rust highlight=4-6").unwrap();
        assert!(meta.highlight.contains(4));
        assert!(meta.highlight.contains(5));
        assert!(meta.highlight.contains(6));
        assert!(!meta.highlight.contains(3));
        assert!(!meta.highlight.contains(7));
    }

    #[test]
    fn test_parse_highlight_mixed() {
        let meta = parse_code_info("rust highlight=1,4-6,10").unwrap();
        assert!(meta.highlight.contains(1));
        assert!(meta.highlight.contains(4));
        assert!(meta.highlight.contains(5));
        assert!(meta.highlight.contains(6));
        assert!(meta.highlight.contains(10));
        assert!(!meta.highlight.contains(2));
        assert!(!meta.highlight.contains(7));
        assert!(!meta.highlight.contains(9));
    }

    #[test]
    fn test_parse_highlight_invalid_range() {
        let result = parse_code_info("rust highlight=6-4");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_highlight_invalid_number() {
        let result = parse_code_info("rust highlight=abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_multiple_attributes() {
        let meta =
            parse_code_info(r#"ts title="Greet" line-numbering=true highlight=1,4-6"#).unwrap();
        assert_eq!(meta.language, "ts");
        assert_eq!(meta.title, Some("Greet".to_string()));
        assert!(meta.line_numbering);
        assert!(meta.highlight.contains(1));
        assert!(meta.highlight.contains(5));
        assert_eq!(meta.highlight.len(), 2); // Two ranges: single(1) and range(4-6)
    }

    #[test]
    fn test_parse_custom_attributes() {
        let meta = parse_code_info("rust custom-key=custom-value another=123").unwrap();
        assert_eq!(meta.language, "rust");
        assert_eq!(
            meta.custom.get("custom-key"),
            Some(&"custom-value".to_string())
        );
        assert_eq!(meta.custom.get("another"), Some(&"123".to_string()));
    }

    #[test]
    fn test_highlight_spec_empty() {
        let spec = HighlightSpec::new();
        assert!(spec.is_empty());
        assert_eq!(spec.len(), 0);
        assert!(!spec.contains(1));
    }

    #[test]
    fn test_highlight_spec_add_line() {
        let mut spec = HighlightSpec::new();
        spec.add_line(5);
        assert!(!spec.is_empty());
        assert_eq!(spec.len(), 1);
        assert!(spec.contains(5));
        assert!(!spec.contains(4));
    }

    #[test]
    fn test_highlight_spec_add_range() {
        let mut spec = HighlightSpec::new();
        spec.add_range(3, 7).unwrap();
        assert_eq!(spec.len(), 1);
        assert!(spec.contains(3));
        assert!(spec.contains(5));
        assert!(spec.contains(7));
        assert!(!spec.contains(2));
        assert!(!spec.contains(8));
    }

    #[test]
    fn test_valid_line_range_single() {
        let range = ValidLineRange::single(5);
        assert_eq!(range.start(), 5);
        assert_eq!(range.end(), 5);
        assert!(range.contains(5));
        assert!(!range.contains(4));
        assert!(!range.contains(6));
    }

    #[test]
    fn test_valid_line_range_range() {
        let range = ValidLineRange::range(3, 7).unwrap();
        assert_eq!(range.start(), 3);
        assert_eq!(range.end(), 7);
        assert!(range.contains(3));
        assert!(range.contains(5));
        assert!(range.contains(7));
        assert!(!range.contains(2));
        assert!(!range.contains(8));
    }

    #[test]
    fn test_valid_line_range_invalid() {
        let result = ValidLineRange::range(7, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_highlight_spec_single_line() {
        let spec = parse_highlight_spec("5").unwrap();
        assert!(spec.contains(5));
        assert!(!spec.contains(4));
        assert!(!spec.contains(6));
    }

    #[test]
    fn test_parse_highlight_spec_multiple_lines() {
        let spec = parse_highlight_spec("1,3,5").unwrap();
        assert!(spec.contains(1));
        assert!(spec.contains(3));
        assert!(spec.contains(5));
        assert!(!spec.contains(2));
        assert!(!spec.contains(4));
    }

    #[test]
    fn test_parse_highlight_spec_range() {
        let spec = parse_highlight_spec("4-6").unwrap();
        assert!(spec.contains(4));
        assert!(spec.contains(5));
        assert!(spec.contains(6));
        assert!(!spec.contains(3));
        assert!(!spec.contains(7));
    }

    #[test]
    fn test_parse_highlight_spec_mixed() {
        let spec = parse_highlight_spec("1,4-6,10").unwrap();
        assert!(spec.contains(1));
        assert!(spec.contains(4));
        assert!(spec.contains(5));
        assert!(spec.contains(6));
        assert!(spec.contains(10));
        assert!(!spec.contains(2));
        assert!(!spec.contains(7));
        assert!(!spec.contains(9));
    }

    #[test]
    fn test_parse_highlight_spec_empty_segments() {
        let spec = parse_highlight_spec("1,,3,",).unwrap();
        assert!(spec.contains(1));
        assert!(spec.contains(3));
        assert_eq!(spec.len(), 2);
    }

    #[test]
    fn test_parse_highlight_spec_invalid_range_format() {
        let err = parse_highlight_spec("1-2-3").unwrap_err();
        assert!(matches!(err, HighlightSpecParseError::InvalidRangeFormat { .. }));
    }

    #[test]
    fn test_parse_highlight_spec_invalid_start_number() {
        let err = parse_highlight_spec("abc-5").unwrap_err();
        assert!(matches!(err, HighlightSpecParseError::InvalidStartNumber { .. }));
    }

    #[test]
    fn test_parse_highlight_spec_invalid_end_number() {
        let err = parse_highlight_spec("5-xyz").unwrap_err();
        assert!(matches!(err, HighlightSpecParseError::InvalidEndNumber { .. }));
    }

    #[test]
    fn test_parse_highlight_spec_invalid_line_number() {
        let err = parse_highlight_spec("abc").unwrap_err();
        assert!(matches!(err, HighlightSpecParseError::InvalidLineNumber { .. }));
    }

    #[test]
    fn test_parse_highlight_spec_invalid_range() {
        let err = parse_highlight_spec("6-4").unwrap_err();
        assert!(matches!(err, HighlightSpecParseError::InvalidRange { start: 6, end: 4 }));
    }

    #[test]
    fn test_parse_highlight_spec_error_display_matches_invalid_line_range() {
        let cases = [
            ("1-2-3", "Invalid range format: 1-2-3"),
            ("abc-5", "Invalid start number: abc"),
            ("5-xyz", "Invalid end number: xyz"),
            ("abc", "Invalid line number: abc"),
            ("6-4", "6-4 (start must be <= end)"),
        ];
        for (input, expected) in cases {
            let err = parse_highlight_spec(input).unwrap_err();
            assert_eq!(err.to_string(), expected, "input: {input}");
        }
    }
}
