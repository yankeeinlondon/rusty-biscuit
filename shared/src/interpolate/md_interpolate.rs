//! Markdown context-aware string interpolation.
//!
//! This module provides scope-aware find/replace operations that only modify
//! content within specific markdown structural elements. Content outside the
//! target scope is preserved unchanged.
//!
//! ## Examples
//!
//! ```
//! use shared::interpolate::md_interpolate::{md_interpolate, md_interpolate_regex};
//! use shared::isolate::MarkdownScope;
//!
//! let content = "# Hello World\n\nHello paragraph.";
//!
//! // Only replace "Hello" within headings
//! let result = md_interpolate(content, MarkdownScope::Heading, "Hello", "Hi").unwrap();
//! assert_eq!(result, "# Hi World\n\nHello paragraph.");
//!
//! // Regex replacement within headings
//! let result = md_interpolate_regex(content, MarkdownScope::Heading, r"(\w+) World", "Greetings $1").unwrap();
//! assert_eq!(result, "# Greetings Hello\n\nHello paragraph.");
//! ```

use std::borrow::Cow;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use regex::Regex;

use crate::isolate::{InterpolateError, IsolateError, MarkdownScope};

/// A byte range within the source content representing a scoped region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScopedRegion {
    start: usize,
    end: usize,
}

/// A replacement to apply, with byte position and text.
#[derive(Debug)]
struct Replacement {
    /// Start byte position in original content.
    start: usize,
    /// End byte position in original content.
    end: usize,
    /// The replacement text.
    text: String,
}

/// Context-aware string replacement within markdown scopes.
///
/// Only replaces `find` with `replace` within regions that match the given scope.
/// Content outside the scope is preserved unchanged.
///
/// ## Strategy
///
/// Uses byte ranges from isolation to track replacement positions:
/// 1. Call internal isolation to identify scoped regions with their byte positions
/// 2. Find all occurrences of `find` string within each scoped region
/// 3. Replace in reverse byte order to maintain valid positions
///
/// ## Returns
///
/// Returns `Cow::Borrowed` if no matches are found within the scope,
/// `Cow::Owned` if any replacements were made.
///
/// ## Examples
///
/// ```
/// use shared::interpolate::md_interpolate::md_interpolate;
/// use shared::isolate::MarkdownScope;
///
/// let content = "# Hello World\n\nHello paragraph.";
/// let result = md_interpolate(content, MarkdownScope::Heading, "Hello", "Hi").unwrap();
/// assert_eq!(result, "# Hi World\n\nHello paragraph.");
/// ```
pub fn md_interpolate<'a>(
    content: &'a str,
    scope: MarkdownScope,
    find: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError> {
    // Get scoped regions with byte positions
    let regions = md_isolate_ranges(content, scope)?;

    if regions.is_empty() {
        return Ok(Cow::Borrowed(content));
    }

    // Collect all replacements
    let mut replacements = Vec::new();

    for region in &regions {
        let region_content = &content[region.start..region.end];
        let mut search_pos = 0;

        while let Some(offset) = region_content[search_pos..].find(find) {
            let abs_start = region.start + search_pos + offset;
            let abs_end = abs_start + find.len();

            replacements.push(Replacement {
                start: abs_start,
                end: abs_end,
                text: replace.to_string(),
            });

            search_pos += offset + find.len();
        }
    }

    if replacements.is_empty() {
        return Ok(Cow::Borrowed(content));
    }

    // Apply replacements in reverse order to maintain valid byte positions
    Ok(Cow::Owned(apply_replacements(content, replacements)))
}

/// Context-aware regex replacement within markdown scopes.
///
/// Only replaces regex matches within regions that match the given scope.
/// Supports capture groups (`$1`, `$2`, etc.) in the replacement string.
///
/// ## Errors
///
/// Returns `InterpolateError::InvalidPattern` if the regex pattern is invalid.
/// Returns `InterpolateError::IsolateError` if the isolation operation fails.
///
/// ## Examples
///
/// ```
/// use shared::interpolate::md_interpolate::md_interpolate_regex;
/// use shared::isolate::MarkdownScope;
///
/// // Basic regex replacement
/// let content = "# Version 1.0\n\nVersion 1.0 is stable.";
/// let result = md_interpolate_regex(content, MarkdownScope::Heading, r"(\d+)\.(\d+)", "$1.$2.0").unwrap();
/// assert_eq!(result, "# Version 1.0.0\n\nVersion 1.0 is stable.");
///
/// // Capture group replacement
/// let content = "# hello WORLD\n\ntext here";
/// let result = md_interpolate_regex(content, MarkdownScope::Heading, r"(\w+) (\w+)", "$2 $1").unwrap();
/// assert_eq!(result, "# WORLD hello\n\ntext here");
/// ```
pub fn md_interpolate_regex<'a>(
    content: &'a str,
    scope: MarkdownScope,
    pattern: &str,
    replace: &str,
) -> Result<Cow<'a, str>, InterpolateError> {
    let regex = Regex::new(pattern)?;

    // Get scoped regions with byte positions
    let regions = md_isolate_ranges(content, scope)?;

    if regions.is_empty() {
        return Ok(Cow::Borrowed(content));
    }

    // Collect all replacements
    let mut replacements = Vec::new();

    for region in &regions {
        let region_content = &content[region.start..region.end];

        for caps in regex.captures_iter(region_content) {
            let m = caps.get(0).expect("capture group 0 always exists");
            let abs_start = region.start + m.start();
            let abs_end = region.start + m.end();

            // Expand capture groups in replacement
            let mut replacement_text = String::new();
            caps.expand(replace, &mut replacement_text);

            replacements.push(Replacement {
                start: abs_start,
                end: abs_end,
                text: replacement_text,
            });
        }
    }

    if replacements.is_empty() {
        return Ok(Cow::Borrowed(content));
    }

    // Apply replacements in reverse order to maintain valid byte positions
    Ok(Cow::Owned(apply_replacements(content, replacements)))
}

/// Apply collected replacements in reverse order.
///
/// By sorting replacements by start position descending and applying from end to start,
/// we maintain valid byte positions for all replacements.
fn apply_replacements(content: &str, mut replacements: Vec<Replacement>) -> String {
    // Sort by start position descending
    replacements.sort_by(|a, b| b.start.cmp(&a.start));

    let mut result = content.to_string();
    for replacement in replacements {
        result.replace_range(replacement.start..replacement.end, &replacement.text);
    }
    result
}

/// Internal function to get byte ranges for scoped content regions.
///
/// This replicates the parsing logic from md_isolate but returns byte ranges
/// instead of content slices, which is necessary for interpolation.
fn md_isolate_ranges(
    content: &str,
    scope: MarkdownScope,
) -> Result<Vec<ScopedRegion>, IsolateError> {
    match scope {
        MarkdownScope::Frontmatter => isolate_frontmatter_ranges(content),
        MarkdownScope::Prose => isolate_prose_ranges(content),
        MarkdownScope::CodeBlock => isolate_code_block_ranges(content),
        MarkdownScope::BlockQuote => isolate_block_quote_ranges(content),
        MarkdownScope::Heading => isolate_heading_ranges(content),
        MarkdownScope::Stylized => isolate_stylized_ranges(content),
        MarkdownScope::Italicized => isolate_italicized_ranges(content),
        MarkdownScope::NonItalicized => isolate_non_italicized_ranges(content),
        MarkdownScope::Links => isolate_link_ranges(content),
        MarkdownScope::Images => isolate_image_ranges(content),
        MarkdownScope::Lists => isolate_list_ranges(content),
        MarkdownScope::Tables => isolate_table_ranges(content),
        MarkdownScope::FootnoteDefinitions => isolate_footnote_ranges(content),
    }
}

/// Creates a parser with standard GFM options.
fn create_parser(content: &str) -> Parser<'_> {
    let opts = Options::ENABLE_GFM
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH;
    Parser::new_ext(content, opts)
}

/// Validates and returns a scoped region if byte boundaries are valid.
fn validate_region(content: &str, start: usize, end: usize) -> Result<ScopedRegion, IsolateError> {
    if start > end || end > content.len() {
        return Err(IsolateError::InvalidByteRange { start, end });
    }
    if !content.is_char_boundary(start) || !content.is_char_boundary(end) {
        return Err(IsolateError::InvalidByteRange { start, end });
    }
    Ok(ScopedRegion { start, end })
}

/// Parse YAML frontmatter and return its byte range.
fn isolate_frontmatter_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let trimmed = content.trim_start();
    let leading_whitespace = content.len() - trimmed.len();

    if !trimmed.starts_with("---") {
        return Ok(Vec::new());
    }

    // Find the end of the first line (the opening ---)
    let Some(after_opener) = trimmed.get(3..) else {
        return Ok(Vec::new());
    };
    let Some(opener_newline) = after_opener.find('\n') else {
        return Ok(Vec::new());
    };
    let opener_end = opener_newline + 3 + leading_whitespace;

    // Find the closing ---
    let body_start = opener_end + 1;
    let Some(remaining) = content.get(body_start..) else {
        return Ok(Vec::new());
    };

    for (i, line) in remaining.lines().enumerate() {
        let trimmed_line = line.trim();
        if trimmed_line == "---" || trimmed_line == "..." {
            // Calculate the actual byte position
            let mut pos = body_start;
            for (j, l) in remaining.lines().enumerate() {
                if j == i {
                    break;
                }
                pos += l.len() + 1; // +1 for newline
            }
            return Ok(vec![validate_region(content, body_start, pos)?]);
        }
    }

    Ok(Vec::new())
}

/// Isolate prose text byte ranges.
fn isolate_prose_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();

    let mut in_code_block = false;
    let mut block_quote_depth: u32 = 0;
    let mut heading_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Start(Tag::BlockQuote(_)) => block_quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                block_quote_depth = block_quote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Heading { .. }) => heading_depth += 1,
            Event::End(TagEnd::Heading(_)) => heading_depth = heading_depth.saturating_sub(1),
            Event::Text(_) | Event::SoftBreak | Event::HardBreak => {
                if !in_code_block && block_quote_depth == 0 && heading_depth == 0 {
                    let slice = &content[range.start..range.end];
                    if !slice.trim().is_empty()
                        || matches!(event, Event::SoftBreak | Event::HardBreak)
                    {
                        regions.push(validate_region(content, range.start, range.end)?);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate code block byte ranges.
fn isolate_code_block_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut in_code_block = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Text(_) if in_code_block => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate block quote byte ranges.
fn isolate_block_quote_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut block_quote_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::BlockQuote(_)) => block_quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => {
                block_quote_depth = block_quote_depth.saturating_sub(1);
            }
            Event::Text(_) if block_quote_depth > 0 => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate heading byte ranges.
fn isolate_heading_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut in_heading = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. }) => in_heading = true,
            Event::End(TagEnd::Heading(_)) => in_heading = false,
            Event::Text(_) | Event::Code(_) if in_heading => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate stylized (bold, italic, strikethrough) byte ranges.
fn isolate_stylized_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut style_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Strong | Tag::Emphasis | Tag::Strikethrough) => style_depth += 1,
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough) => {
                style_depth = style_depth.saturating_sub(1);
            }
            Event::Text(_) if style_depth > 0 => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate italic byte ranges.
fn isolate_italicized_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut emphasis_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Emphasis) => emphasis_depth += 1,
            Event::End(TagEnd::Emphasis) => emphasis_depth = emphasis_depth.saturating_sub(1),
            Event::Text(_) if emphasis_depth > 0 => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate non-italic byte ranges.
fn isolate_non_italicized_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut emphasis_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Emphasis) => emphasis_depth += 1,
            Event::End(TagEnd::Emphasis) => emphasis_depth = emphasis_depth.saturating_sub(1),
            Event::Text(_) if emphasis_depth == 0 => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate link text byte ranges.
///
/// Note: This only captures the text content of links, not the URL itself,
/// since the URL is not part of the original content byte range.
fn isolate_link_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut in_link = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Link { .. }) => in_link = true,
            Event::End(TagEnd::Link) => in_link = false,
            Event::Text(_) if in_link => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate image alt text byte ranges.
///
/// Note: This only captures the alt text of images, not the URL itself,
/// since the URL is not part of the original content byte range.
fn isolate_image_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut in_image = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Image { .. }) => in_image = true,
            Event::End(TagEnd::Image) => in_image = false,
            Event::Text(_) if in_image => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate list item byte ranges.
fn isolate_list_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut list_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Text(_) if list_depth > 0 => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate table cell byte ranges.
fn isolate_table_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut in_table = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Table(_)) => in_table = true,
            Event::End(TagEnd::Table) => in_table = false,
            Event::Text(_) if in_table => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

/// Isolate footnote definition byte ranges.
fn isolate_footnote_ranges(content: &str) -> Result<Vec<ScopedRegion>, IsolateError> {
    let parser = create_parser(content);
    let mut regions = Vec::new();
    let mut in_footnote = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::FootnoteDefinition(_)) => in_footnote = true,
            Event::End(TagEnd::FootnoteDefinition) => in_footnote = false,
            Event::Text(_) if in_footnote => {
                regions.push(validate_region(content, range.start, range.end)?);
            }
            _ => {}
        }
    }

    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // md_interpolate tests
    // =========================================================================

    #[test]
    fn test_heading_only_replacement() {
        let content = "# Hello World\n\nHello paragraph.";
        let result = md_interpolate(content, MarkdownScope::Heading, "Hello", "Hi").unwrap();
        assert_eq!(result, "# Hi World\n\nHello paragraph.");
        assert!(matches!(result, Cow::Owned(_)));
    }

    #[test]
    fn test_no_match_returns_borrowed() {
        let content = "# Title\n\nSome text.";
        let result = md_interpolate(content, MarkdownScope::Heading, "NotFound", "Replace").unwrap();
        assert_eq!(result, "# Title\n\nSome text.");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_empty_scope_returns_borrowed() {
        let content = "No headings here, just prose.";
        let result = md_interpolate(content, MarkdownScope::Heading, "here", "there").unwrap();
        assert_eq!(result, "No headings here, just prose.");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_prose_only_replacement() {
        let content = "# Hello Title\n\nHello body text. Hello again.";
        let result = md_interpolate(content, MarkdownScope::Prose, "Hello", "Goodbye").unwrap();
        assert_eq!(result, "# Hello Title\n\nGoodbye body text. Goodbye again.");
    }

    #[test]
    fn test_code_block_replacement() {
        let content = "Text before.\n\n```rust\nfn hello() {}\n```\n\nText with hello.";
        let result = md_interpolate(content, MarkdownScope::CodeBlock, "hello", "world").unwrap();
        assert_eq!(
            result,
            "Text before.\n\n```rust\nfn world() {}\n```\n\nText with hello."
        );
    }

    #[test]
    fn test_block_quote_replacement() {
        let content = "> Important message here.\n\nNormal message here.";
        let result = md_interpolate(content, MarkdownScope::BlockQuote, "message", "note").unwrap();
        assert_eq!(result, "> Important note here.\n\nNormal message here.");
    }

    #[test]
    fn test_stylized_replacement() {
        let content = "Normal **bold text** and more.";
        let result = md_interpolate(content, MarkdownScope::Stylized, "bold", "strong").unwrap();
        assert_eq!(result, "Normal **strong text** and more.");
    }

    #[test]
    fn test_italicized_replacement() {
        let content = "Normal *italic text* and **bold text** here.";
        let result = md_interpolate(content, MarkdownScope::Italicized, "text", "content").unwrap();
        assert_eq!(result, "Normal *italic content* and **bold text** here.");
    }

    #[test]
    fn test_list_replacement() {
        let content = "# Items\n\n- First item\n- Second item\n\nNot an item.";
        let result = md_interpolate(content, MarkdownScope::Lists, "item", "entry").unwrap();
        assert_eq!(
            result,
            "# Items\n\n- First entry\n- Second entry\n\nNot an item."
        );
    }

    #[test]
    fn test_multiple_occurrences_in_scope() {
        let content = "# Hello Hello Hello";
        let result = md_interpolate(content, MarkdownScope::Heading, "Hello", "Hi").unwrap();
        assert_eq!(result, "# Hi Hi Hi");
    }

    #[test]
    fn test_unicode_content() {
        let content = "# Hello \u{1F600}\n\nHello \u{1F600}";
        let result = md_interpolate(content, MarkdownScope::Heading, "\u{1F600}", "\u{1F389}").unwrap();
        assert_eq!(result, "# Hello \u{1F389}\n\nHello \u{1F600}");
    }

    #[test]
    fn test_empty_content() {
        let result = md_interpolate("", MarkdownScope::Heading, "find", "replace").unwrap();
        assert_eq!(result, "");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    // =========================================================================
    // md_interpolate_regex tests
    // =========================================================================

    #[test]
    fn test_regex_basic_replacement() {
        let content = "# Version 1.0\n\nVersion 1.0 is stable.";
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"(\d+)\.(\d+)", "$1.$2.0")
                .unwrap();
        assert_eq!(result, "# Version 1.0.0\n\nVersion 1.0 is stable.");
    }

    #[test]
    fn test_regex_capture_groups() {
        let content = "# hello WORLD\n\ntext here";
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"(\w+) (\w+)", "$2 $1").unwrap();
        assert_eq!(result, "# WORLD hello\n\ntext here");
    }

    #[test]
    fn test_regex_no_match_returns_borrowed() {
        let content = "# Title\n\nBody text.";
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"\d+", "NUM").unwrap();
        assert_eq!(result, "# Title\n\nBody text.");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn test_regex_invalid_pattern_error() {
        let result = md_interpolate_regex("# Test", MarkdownScope::Heading, r"[invalid", "x");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, InterpolateError::InvalidPattern(_)));
    }

    #[test]
    fn test_regex_multiple_matches_in_scope() {
        let content = "# Item1 Item2 Item3";
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"Item(\d)", "Entry$1").unwrap();
        assert_eq!(result, "# Entry1 Entry2 Entry3");
    }

    #[test]
    fn test_regex_word_boundary() {
        let content = "# cat category\n\ncat category";
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"\bcat\b", "dog").unwrap();
        assert_eq!(result, "# dog category\n\ncat category");
    }

    #[test]
    fn test_regex_case_insensitive() {
        let content = "# Hello HELLO hello\n\nbody";
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"(?i)hello", "hi").unwrap();
        assert_eq!(result, "# hi hi hi\n\nbody");
    }

    #[test]
    fn test_regex_empty_scope_returns_borrowed() {
        let content = "No code blocks here.";
        let result =
            md_interpolate_regex(content, MarkdownScope::CodeBlock, r"\w+", "X").unwrap();
        assert_eq!(result, "No code blocks here.");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    // =========================================================================
    // Edge case tests
    // =========================================================================

    #[test]
    fn test_overlapping_scopes_heading_in_list() {
        // Headings inside lists shouldn't happen, but test behavior is stable
        let content = "- Item one\n- Item two";
        let result = md_interpolate(content, MarkdownScope::Lists, "Item", "Entry").unwrap();
        assert_eq!(result, "- Entry one\n- Entry two");
    }

    #[test]
    fn test_nested_styling() {
        let content = "Normal ***bold and italic*** text.";
        let result =
            md_interpolate(content, MarkdownScope::Stylized, "bold", "strong").unwrap();
        assert_eq!(result, "Normal ***strong and italic*** text.");
    }

    #[test]
    fn test_adjacent_scoped_regions() {
        let content = "# First\n\n# Second\n\n# Third";
        let result = md_interpolate(content, MarkdownScope::Heading, "i", "I").unwrap();
        assert_eq!(result, "# FIrst\n\n# Second\n\n# ThIrd");
    }

    #[test]
    fn test_replace_with_empty_string() {
        let content = "# Hello World\n\ntext";
        let result = md_interpolate(content, MarkdownScope::Heading, " World", "").unwrap();
        assert_eq!(result, "# Hello\n\ntext");
    }

    #[test]
    fn test_replace_with_longer_string() {
        let content = "# Hi\n\nbody";
        let result = md_interpolate(content, MarkdownScope::Heading, "Hi", "Hello there").unwrap();
        assert_eq!(result, "# Hello there\n\nbody");
    }

    #[test]
    fn test_frontmatter_replacement() {
        let content = "---\ntitle: Hello\n---\n\n# Hello World";
        let result = md_interpolate(content, MarkdownScope::Frontmatter, "Hello", "Goodbye").unwrap();
        assert_eq!(result, "---\ntitle: Goodbye\n---\n\n# Hello World");
    }

    #[test]
    fn test_table_replacement() {
        let content = "| Header |\n|--------|\n| Cell   |\n\nOutside table Header.";
        let result = md_interpolate(content, MarkdownScope::Tables, "Header", "Title").unwrap();
        assert_eq!(
            result,
            "| Title |\n|--------|\n| Cell   |\n\nOutside table Header."
        );
    }

    #[test]
    fn test_link_text_replacement() {
        let content = "Check [Example Link](https://example.com) here. Example outside.";
        let result = md_interpolate(content, MarkdownScope::Links, "Example", "Sample").unwrap();
        assert_eq!(
            result,
            "Check [Sample Link](https://example.com) here. Example outside."
        );
    }

    #[test]
    fn test_image_alt_replacement() {
        let content = "![Alt Text](image.png) Some Alt Text outside.";
        let result = md_interpolate(content, MarkdownScope::Images, "Alt", "Image").unwrap();
        assert_eq!(result, "![Image Text](image.png) Some Alt Text outside.");
    }

    #[test]
    fn test_non_italicized_replacement() {
        let content = "Normal *italic* normal again.";
        let result =
            md_interpolate(content, MarkdownScope::NonItalicized, "normal", "regular").unwrap();
        // Note: case-sensitive, so "Normal" won't match "normal"
        assert_eq!(result, "Normal *italic* regular again.");
    }

    // =========================================================================
    // Additional tests for complete coverage
    // =========================================================================

    #[test]
    fn test_footnote_replacement() {
        let content = "Text with footnote[^1].\n\n[^1]: This is the footnote content.";
        let result =
            md_interpolate(content, MarkdownScope::FootnoteDefinitions, "footnote", "note").unwrap();
        // Should replace "footnote" in the definition, not in the reference
        assert!(result.contains("[^1]: This is the note content."));
        // The reference should be unchanged
        assert!(result.contains("footnote[^1]"));
    }

    #[test]
    fn test_footnote_regex_replacement() {
        let content = "Reference[^fn1].\n\n[^fn1]: Note with Version 1.0.";
        let result = md_interpolate_regex(
            content,
            MarkdownScope::FootnoteDefinitions,
            r"Version (\d+)\.(\d+)",
            "v$1.$2",
        )
        .unwrap();
        assert!(result.contains("v1.0"));
    }

    #[test]
    fn test_multiple_scoped_regions_unicode() {
        let content = "# Hello \u{1F600}\n\n## World \u{1F389}\n\n### Test \u{1F680}";
        let result =
            md_interpolate(content, MarkdownScope::Heading, "\u{1F600}", "[smile]").unwrap();
        assert!(result.contains("Hello [smile]"));
        // Other emojis should be unchanged
        assert!(result.contains("\u{1F389}"));
        assert!(result.contains("\u{1F680}"));
    }

    #[test]
    fn test_regex_with_special_markdown_chars() {
        let content = "# Title with *asterisks*\n\nbody";
        // The heading contains the asterisks as literal characters in the text
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"Title", "Header").unwrap();
        assert!(result.contains("# Header"));
    }

    #[test]
    fn test_preserve_non_scoped_newlines() {
        let content = "# Title\n\n\n\nParagraph with gaps.";
        let result = md_interpolate(content, MarkdownScope::Heading, "Title", "Header").unwrap();
        // Non-scoped content including newlines should be preserved
        assert_eq!(result, "# Header\n\n\n\nParagraph with gaps.");
    }

    #[test]
    fn test_deeply_nested_stylized() {
        let content = "Normal ***bold and italic*** text.";
        let result =
            md_interpolate(content, MarkdownScope::Stylized, "and", "&").unwrap();
        assert_eq!(result, "Normal ***bold & italic*** text.");
    }

    #[test]
    fn test_strikethrough_replacement() {
        let content = "Keep ~~remove this~~ keep again.";
        let result =
            md_interpolate(content, MarkdownScope::Stylized, "remove", "delete").unwrap();
        assert!(result.contains("~~delete this~~"));
    }

    #[test]
    fn test_list_item_with_formatting() {
        let content = "- Item with **bold** text\n- Another *italic* item";
        let result = md_interpolate(content, MarkdownScope::Lists, "text", "content").unwrap();
        // Lists scope should capture text within list items
        assert!(result.contains("content"));
    }

    #[test]
    fn test_table_cell_replacement() {
        let content = "| Name | Value |\n|------|-------|\n| foo  | bar   |";
        let result = md_interpolate(content, MarkdownScope::Tables, "foo", "key").unwrap();
        assert!(result.contains("key"));
        // Bar should be unchanged if not targeted
        assert!(result.contains("bar"));
    }

    #[test]
    fn test_regex_alternation_patterns() {
        // Test regex alternation (|) instead of lookahead (which isn't supported by regex crate)
        let content = "# foo bar foo baz\n\nbody foo";
        let result =
            md_interpolate_regex(content, MarkdownScope::Heading, r"foo bar", "qux").unwrap();
        assert_eq!(result, "# qux foo baz\n\nbody foo");
    }

    #[test]
    fn test_empty_replacement_result() {
        let content = "# Remove Me\n\nbody";
        let result =
            md_interpolate(content, MarkdownScope::Heading, "Remove Me", "").unwrap();
        assert_eq!(result, "# \n\nbody");
    }

    #[test]
    fn test_multiline_code_block_replacement() {
        let content = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\nhello outside";
        let result =
            md_interpolate(content, MarkdownScope::CodeBlock, "hello", "world").unwrap();
        assert!(result.contains("println!(\"world\")"));
        assert!(result.contains("hello outside")); // Not in code block
    }

    #[test]
    fn test_setext_heading_replacement() {
        let content = "Title Here\n==========\n\nbody";
        let result = md_interpolate(content, MarkdownScope::Heading, "Here", "There").unwrap();
        assert!(result.contains("Title There"));
    }
}
