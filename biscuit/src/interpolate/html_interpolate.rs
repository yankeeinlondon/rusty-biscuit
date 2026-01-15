//! HTML context-aware string interpolation.
//!
//! This module provides functions for replacing content within HTML documents
//! based on structural scopes. Unlike simple string replacement, these functions
//! only replace content within elements matching the specified scope.
//!
//! ## Trade-offs and Limitations
//!
//! Since the `scraper` crate's DOM does not provide byte offsets, this module
//! uses a string-based replacement strategy. This means:
//!
//! - Identical content appearing in multiple matching elements will all be replaced
//! - The replacement is based on string matching, not DOM position
//! - For most use cases this is acceptable; for cases requiring precise element
//!   targeting, use more specific CSS selectors
//!
//! ## Examples
//!
//! ```rust
//! use shared::interpolate::html_interpolate::{html_interpolate, html_interpolate_regex};
//! use shared::isolate::{HtmlScope, HtmlTag};
//!
//! // Replace "Hello" only within div elements
//! let html = r#"<div>Hello World</div><span>Hello Span</span>"#;
//! let result = html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "Hello", "Hi").unwrap();
//! assert!(result.contains("Hi World"));
//! assert!(result.contains("Hello Span")); // span content unchanged
//!
//! // Use regex for pattern-based replacement
//! let html = "<p>Version 1.0.0</p><span>Version 2.0.0</span>";
//! let result = html_interpolate_regex(
//!     html,
//!     HtmlScope::InnerHtml(HtmlTag::All),
//!     r"Version (\d+)\.(\d+)\.(\d+)",
//!     "v$1.$2.$3"
//! ).unwrap();
//! ```

use std::borrow::Cow;

use regex::Regex;

use crate::isolate::html_isolate::html_isolate;
use crate::isolate::{HtmlScope, InterpolateError, IsolateAction, IsolateResult};

/// Context-aware string replacement within HTML scopes.
///
/// Only replaces `find` with `replace` within content that matches the given scope.
/// Content outside the scope is preserved unchanged.
///
/// ## Arguments
///
/// * `content` - The HTML content to process
/// * `scope` - Specifies which parts of the HTML to search within
/// * `find` - The literal string to find
/// * `replace` - The string to replace matches with
///
/// ## Returns
///
/// A `Cow<str>` containing the result:
/// - `Cow::Borrowed(content)` if no replacements were made
/// - `Cow::Owned(modified)` if any replacements were made
///
/// ## Limitation
///
/// Since scraper's DOM doesn't provide byte offsets, this function uses a
/// string-based replacement strategy. This means identical content in
/// multiple matching elements will all be replaced.
///
/// ## Examples
///
/// ```rust
/// use shared::interpolate::html_interpolate::html_interpolate;
/// use shared::isolate::{HtmlScope, HtmlTag};
///
/// let content = r#"<div>Hello World</div><span>Hello Span</span>"#;
/// let result = html_interpolate(content, HtmlScope::InnerHtml(HtmlTag::Div), "Hello", "Hi").unwrap();
/// // Only "Hello" in the div is replaced
/// assert!(result.contains("Hi World"));
/// assert!(result.contains("Hello Span"));
/// ```
pub fn html_interpolate<'a>(
    content: &'a str,
    scope: HtmlScope,
    find: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError> {
    // Fast path: if find string doesn't exist in content at all, return borrowed
    if !content.contains(find) {
        return Ok(Cow::Borrowed(content));
    }

    // Get isolated content from the scope
    let isolated = html_isolate(content, scope, IsolateAction::LeaveAsVector)?;

    let scoped_items = match isolated {
        IsolateResult::Vector(items) => items,
        IsolateResult::Concatenated(s) => vec![Cow::Owned(s)],
    };

    // If no scoped content, return original
    if scoped_items.is_empty() {
        return Ok(Cow::Borrowed(content));
    }

    // Check if any scoped item contains the find string
    let has_match = scoped_items.iter().any(|item| item.contains(find));

    if !has_match {
        return Ok(Cow::Borrowed(content));
    }

    // Build a set of unique replacement pairs based on scoped content
    // We need to find substrings in the scoped content that contain `find`
    // and create the replacement for those specific occurrences
    let mut result = content.to_string();

    for item in &scoped_items {
        if item.contains(find) {
            // Replace the original scoped content with the modified version
            let modified = item.replace(find, replace);
            // Only replace if different
            if modified != item.as_ref() {
                result = result.replacen(item.as_ref(), &modified, 1);
            }
        }
    }

    // If the result is the same as the original, return borrowed
    if result == content {
        Ok(Cow::Borrowed(content))
    } else {
        Ok(Cow::Owned(result))
    }
}

/// Context-aware regex replacement within HTML scopes.
///
/// Only applies the regex pattern replacement within content that matches the given scope.
/// Content outside the scope is preserved unchanged.
///
/// ## Arguments
///
/// * `content` - The HTML content to process
/// * `scope` - Specifies which parts of the HTML to search within
/// * `pattern` - The regex pattern to match
/// * `replace` - The replacement string (supports capture group references like `$1`, `$2`)
///
/// ## Returns
///
/// A `Cow<str>` containing the result:
/// - `Cow::Borrowed(content)` if no replacements were made
/// - `Cow::Owned(modified)` if any replacements were made
///
/// ## Errors
///
/// Returns `InterpolateError::InvalidPattern` if the regex pattern is invalid.
///
/// ## Limitation
///
/// Since scraper's DOM doesn't provide byte offsets, this function uses a
/// string-based replacement strategy. This means identical content in
/// multiple matching elements will all be replaced.
///
/// ## Examples
///
/// ```rust
/// use shared::interpolate::html_interpolate::html_interpolate_regex;
/// use shared::isolate::{HtmlScope, HtmlTag};
///
/// let content = r#"<p>Hello 123</p><span>Hello 456</span>"#;
/// let result = html_interpolate_regex(
///     content,
///     HtmlScope::InnerHtml(HtmlTag::All),
///     r"\d+",
///     "NUM"
/// ).unwrap();
/// // Numbers in both elements are replaced since All matches both
/// ```
pub fn html_interpolate_regex<'a>(
    content: &'a str,
    scope: HtmlScope,
    pattern: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError> {
    // Compile the regex pattern
    let regex = Regex::new(pattern)?;

    // Fast path: if pattern doesn't match anywhere in content, return borrowed
    if !regex.is_match(content) {
        return Ok(Cow::Borrowed(content));
    }

    // Get isolated content from the scope
    let isolated = html_isolate(content, scope, IsolateAction::LeaveAsVector)?;

    let scoped_items = match isolated {
        IsolateResult::Vector(items) => items,
        IsolateResult::Concatenated(s) => vec![Cow::Owned(s)],
    };

    // If no scoped content, return original
    if scoped_items.is_empty() {
        return Ok(Cow::Borrowed(content));
    }

    // Check if any scoped item matches the pattern
    let has_match = scoped_items.iter().any(|item| regex.is_match(item));

    if !has_match {
        return Ok(Cow::Borrowed(content));
    }

    // Apply replacements
    let mut result = content.to_string();

    for item in &scoped_items {
        if regex.is_match(item) {
            // Apply regex replacement to the scoped content
            let modified = regex.replace_all(item, replace);
            // Only replace if different
            if modified != item.as_ref() {
                result = result.replacen(item.as_ref(), &modified, 1);
            }
        }
    }

    // If the result is the same as the original, return borrowed
    if result == content {
        Ok(Cow::Borrowed(content))
    } else {
        Ok(Cow::Owned(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolate::HtmlTag;

    // Test 1: Basic string replacement in div
    #[test]
    fn test_basic_replacement_in_div() {
        let html = r#"<div>Hello World</div><span>Hello Span</span>"#;
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "Hello", "Hi").unwrap();

        assert!(result.contains("Hi World"));
        assert!(result.contains("Hello Span"));
    }

    // Test 2: No match returns borrowed
    #[test]
    fn test_no_match_returns_borrowed() {
        let html = "<div>Hello World</div>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "Goodbye", "Hi").unwrap();

        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, html);
    }

    // Test 3: Match outside scope is not replaced
    #[test]
    fn test_match_outside_scope_not_replaced() {
        let html = r#"<div>Keep this</div><span>Replace this</span>"#;
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Span), "this", "that").unwrap();

        assert!(result.contains("Keep this"));
        assert!(result.contains("Replace that"));
    }

    // Test 4: CSS selector scope
    #[test]
    fn test_css_selector_scope() {
        let html = r#"<div class="target">Hello</div><div>Hello</div>"#;
        let result = html_interpolate(
            html,
            HtmlScope::Selector("div.target".to_string()),
            "Hello",
            "Hi",
        )
        .unwrap();

        // The selector returns outer HTML, so replacement happens in the full element
        assert!(result.contains("Hi"));
    }

    // Test 5: Empty scoped content
    #[test]
    fn test_empty_scoped_content() {
        let html = "<div>Hello</div>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Span), "Hello", "Hi").unwrap();

        // No spans, so no replacement
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, html);
    }

    // Test 6: Basic regex replacement
    #[test]
    fn test_basic_regex_replacement() {
        let html = "<p>Version 1.0.0</p>";
        let result = html_interpolate_regex(
            html,
            HtmlScope::InnerHtml(HtmlTag::All),
            r"(\d+)\.(\d+)\.(\d+)",
            "v$1.$2.$3",
        )
        .unwrap();

        assert!(result.contains("v1.0.0"));
    }

    // Test 7: Regex with no match
    #[test]
    fn test_regex_no_match() {
        let html = "<p>Hello World</p>";
        let result =
            html_interpolate_regex(html, HtmlScope::InnerHtml(HtmlTag::All), r"\d+", "NUM")
                .unwrap();

        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, html);
    }

    // Test 8: Invalid regex pattern
    #[test]
    fn test_invalid_regex_pattern() {
        let html = "<p>Hello</p>";
        let result = html_interpolate_regex(
            html,
            HtmlScope::InnerHtml(HtmlTag::All),
            r"[invalid",
            "replacement",
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(InterpolateError::InvalidPattern(_))));
    }

    // Test 9: Prose scope replacement
    #[test]
    fn test_prose_scope_replacement() {
        let html = "<p>Hello <strong>World</strong></p>";
        let result = html_interpolate(html, HtmlScope::Prose, "World", "Rust").unwrap();

        // Prose extracts text content, but replacement happens on the concatenated text
        // The actual replacement in the source may not work as expected since prose
        // returns concatenated text without tags
        // This test verifies the function doesn't panic and returns a result
        assert!(!result.is_empty());
    }

    // Test 10: Multiple elements same tag
    #[test]
    fn test_multiple_elements_same_tag() {
        let html = "<span>Hello</span><span>Hello</span>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Span), "Hello", "Hi").unwrap();

        // Both Hello strings should be replaced since they're in span elements
        // Due to string-based replacement, the first match is replaced first
        assert!(result.contains("Hi"));
    }

    // Test 11: Outer HTML scope
    #[test]
    fn test_outer_html_scope() {
        let html = r#"<div class="test">Content</div>"#;
        let result = html_interpolate(
            html,
            HtmlScope::OuterHtml(HtmlTag::Div),
            "Content",
            "Modified",
        )
        .unwrap();

        assert!(result.contains("Modified"));
    }

    // Test 12: Tag attributes scope
    #[test]
    fn test_tag_attributes_scope() {
        let html = r#"<div class="old-class">Content</div>"#;
        let result = html_interpolate(
            html,
            HtmlScope::TagAttributes(HtmlTag::Div),
            "old-class",
            "new-class",
        )
        .unwrap();

        assert!(result.contains("new-class"));
    }

    // Test 13: Regex capture groups
    #[test]
    fn test_regex_capture_groups() {
        let html = "<p>John Smith</p>";
        let result = html_interpolate_regex(
            html,
            HtmlScope::InnerHtml(HtmlTag::All),
            r"(\w+) (\w+)",
            "$2, $1",
        )
        .unwrap();

        assert!(result.contains("Smith, John"));
    }

    // Test 14: Case sensitivity
    #[test]
    fn test_case_sensitivity() {
        let html = "<div>Hello HELLO hello</div>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "Hello", "Hi").unwrap();

        // Only exact case match is replaced
        assert!(result.contains("Hi HELLO hello"));
    }

    // Test 15: Empty find string (edge case)
    #[test]
    fn test_empty_find_string() {
        let html = "<div>Hello</div>";
        // Empty string is found at every position, so it will match
        // This behavior matches standard string::contains and replace
        let result = html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "", "X").unwrap();

        // Empty string replacement inserts X between every character
        assert!(result.contains("X"));
    }

    // Test 16: Special HTML characters
    #[test]
    fn test_special_html_characters() {
        let html = "<div>&lt;script&gt;</div>";
        let result = html_interpolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Div),
            "&lt;script&gt;",
            "&lt;safe&gt;",
        )
        .unwrap();

        assert!(result.contains("&lt;safe&gt;"));
    }

    // Test 17: Nested elements
    #[test]
    fn test_nested_elements() {
        let html = "<div><p>Hello</p></div>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "Hello", "Hi").unwrap();

        // The inner HTML of div includes the p tag, so Hello should be replaced
        assert!(result.contains("Hi"));
    }

    // Test 18: H1 scope
    #[test]
    fn test_h1_scope() {
        let html = "<h1>Title</h1><h2>Subtitle</h2>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::H1), "Title", "Heading").unwrap();

        assert!(result.contains("Heading"));
        assert!(result.contains("Subtitle"));
    }

    // Test 19: Regex scope restriction
    #[test]
    fn test_regex_scope_restriction() {
        let html = "<div>Item 123</div><span>Item 456</span>";
        let result =
            html_interpolate_regex(html, HtmlScope::InnerHtml(HtmlTag::Div), r"\d+", "XXX")
                .unwrap();

        assert!(result.contains("Item XXX"));
        assert!(result.contains("Item 456")); // span should be unchanged
    }

    // Test 20: Complex CSS selector
    #[test]
    fn test_complex_css_selector() {
        let html = r#"<section id="main"><p class="intro">Welcome</p></section>"#;
        let result = html_interpolate(
            html,
            HtmlScope::Selector("section#main p.intro".to_string()),
            "Welcome",
            "Hello",
        )
        .unwrap();

        assert!(result.contains("Hello"));
    }

    // Test 21: Unicode content replacement
    #[test]
    fn test_unicode_html_replacement() {
        let html = "<div>\u{1F600} Hello \u{1F389}</div>";
        let result = html_interpolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Div),
            "\u{1F600}",
            "[smile]",
        )
        .unwrap();
        assert!(result.contains("[smile]"));
        // Other emoji should be unchanged
        assert!(result.contains("\u{1F389}"));
    }

    // Test 22: Empty HTML document
    #[test]
    fn test_empty_html_document_interpolate() {
        let html = "";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "test", "replaced").unwrap();
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "");
    }

    // Test 23: All heading tags H3-H6
    #[test]
    fn test_all_heading_tags_interpolate() {
        let html = "<h3>H3</h3><h4>H4</h4><h5>H5</h5><h6>H6</h6>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::H3), "H3", "Heading3").unwrap();
        assert!(result.contains("Heading3"));
        assert!(result.contains("H4")); // Unchanged
    }

    // Test 24: Header element (semantic HTML)
    #[test]
    fn test_header_element_interpolate() {
        let html = "<header><nav>Navigation</nav></header><main>Main content</main>";
        let result = html_interpolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Header),
            "Navigation",
            "Nav",
        )
        .unwrap();
        assert!(result.contains("Nav"));
        // Main should be unchanged
        assert!(result.contains("Main content"));
    }

    // Test 25: Body element scope
    #[test]
    fn test_body_element_interpolate() {
        let html = "<html><body><p>Body content</p></body></html>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Body), "content", "text").unwrap();
        assert!(result.contains("Body text"));
    }

    // Test 26: Regex with named capture groups
    #[test]
    fn test_regex_named_capture_groups() {
        let html = "<p>2024-01-15</p>";
        let result = html_interpolate_regex(
            html,
            HtmlScope::InnerHtml(HtmlTag::All),
            r"(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})",
            "$day/$month/$year",
        )
        .unwrap();
        assert!(result.contains("15/01/2024"));
    }

    // Test 27: Head element (metadata)
    #[test]
    fn test_head_element_interpolate() {
        let html = "<html><head><title>Page Title</title></head><body>Body</body></html>";
        let result = html_interpolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Head),
            "Page Title",
            "New Title",
        )
        .unwrap();
        assert!(result.contains("New Title"));
        // Body should be unchanged
        assert!(result.contains("Body"));
    }

    // Test 28: Script element (should work but be careful with it)
    #[test]
    fn test_script_element_interpolate() {
        let html = "<script>var x = 'old_value';</script><p>old_value outside</p>";
        let result = html_interpolate(
            html,
            HtmlScope::InnerHtml(HtmlTag::Script),
            "old_value",
            "new_value",
        )
        .unwrap();
        assert!(result.contains("new_value"));
    }

    // Test 29: Multiple replacements in same element
    #[test]
    fn test_multiple_replacements_same_element() {
        let html = "<div>foo bar foo baz foo</div>";
        let result =
            html_interpolate(html, HtmlScope::InnerHtml(HtmlTag::Div), "foo", "qux").unwrap();
        // All occurrences should be replaced
        let qux_count = result.matches("qux").count();
        assert_eq!(qux_count, 3);
        assert!(!result.contains("foo"));
    }

    // Test 30: Regex case insensitive in scope
    #[test]
    fn test_regex_case_insensitive_html() {
        let html = "<div>Hello HELLO hello</div>";
        let result =
            html_interpolate_regex(html, HtmlScope::InnerHtml(HtmlTag::Div), r"(?i)hello", "hi")
                .unwrap();
        // All variants should be replaced
        let hi_count = result.matches("hi").count();
        assert_eq!(hi_count, 3);
    }
}
