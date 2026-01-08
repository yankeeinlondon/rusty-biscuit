//! Markdown content isolation using pulldown-cmark.
//!
//! This module provides the [`md_isolate`] function for extracting specific
//! structural elements from markdown documents based on [`MarkdownScope`].
//!
//! ## Examples
//!
//! ```
//! use shared::isolate::{md_isolate, MarkdownScope, IsolateAction};
//!
//! let content = "# Heading\n\nSome **bold** text.\n\n```rust\nfn main() {}\n```";
//!
//! // Extract code blocks
//! let result = md_isolate(content, MarkdownScope::CodeBlock, IsolateAction::LeaveAsVector);
//! assert!(result.is_ok());
//!
//! // Extract headings and concatenate
//! let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::Concatenate(Some("\n".to_string())));
//! assert!(result.is_ok());
//! ```

use std::borrow::Cow;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::isolate::{IsolateAction, IsolateError, IsolateResult, MarkdownScope};

/// Validates byte range and returns a borrowed slice of the content.
///
/// ## Errors
///
/// Returns [`IsolateError::InvalidByteRange`] if the range does not align
/// with UTF-8 character boundaries.
fn validate_and_slice<'a>(
    content: &'a str,
    start: usize,
    end: usize,
) -> Result<Cow<'a, str>, IsolateError> {
    if start > end || end > content.len() {
        return Err(IsolateError::InvalidByteRange { start, end });
    }
    if !content.is_char_boundary(start) || !content.is_char_boundary(end) {
        return Err(IsolateError::InvalidByteRange { start, end });
    }
    Ok(Cow::Borrowed(&content[start..end]))
}

/// Applies the isolation action to collected content pieces.
fn apply_action<'a>(pieces: Vec<Cow<'a, str>>, action: IsolateAction) -> IsolateResult<'a> {
    match action {
        IsolateAction::LeaveAsVector => IsolateResult::Vector(pieces),
        IsolateAction::Concatenate(None) => {
            let concatenated: String = pieces.iter().map(|c| c.as_ref()).collect();
            IsolateResult::Concatenated(concatenated)
        }
        IsolateAction::Concatenate(Some(delim)) => {
            let concatenated = pieces
                .iter()
                .map(|c| c.as_ref())
                .collect::<Vec<_>>()
                .join(&delim);
            IsolateResult::Concatenated(concatenated)
        }
    }
}

/// Parses YAML frontmatter from markdown content.
///
/// Returns the frontmatter content (excluding delimiters) and the byte offset
/// where the document body begins.
fn parse_frontmatter(content: &str) -> Option<(usize, usize)> {
    let trimmed = content.trim_start();
    let leading_whitespace = content.len() - trimmed.len();

    if !trimmed.starts_with("---") {
        return None;
    }

    // Find the end of the first line (the opening ---)
    let after_opener = trimmed.get(3..)?;
    let opener_end = after_opener.find('\n')? + 3 + leading_whitespace;

    // Find the closing ---
    let body_start = opener_end + 1;
    let remaining = content.get(body_start..)?;

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
            return Some((body_start, pos));
        }
    }

    None
}

/// Isolates specific content from a markdown document.
///
/// Parses the markdown content and extracts elements matching the specified
/// [`MarkdownScope`]. The extraction uses zero-copy where possible by returning
/// borrowed slices of the original content.
///
/// ## Arguments
///
/// * `content` - The markdown document to parse
/// * `scope` - The type of content to extract
/// * `action` - How to return the results (vector or concatenated)
///
/// ## Returns
///
/// Returns an [`IsolateResult`] containing the extracted content, or an
/// [`IsolateError`] if parsing fails or byte ranges are invalid.
///
/// ## Examples
///
/// ```
/// use shared::isolate::{md_isolate, MarkdownScope, IsolateAction, IsolateResult};
///
/// let content = "# Title\n\nParagraph with *italic* text.";
///
/// // Extract all italic text
/// let result = md_isolate(content, MarkdownScope::Italicized, IsolateAction::LeaveAsVector).unwrap();
/// if let IsolateResult::Vector(items) = result {
///     assert_eq!(items.len(), 1);
///     assert_eq!(items[0], "italic");
/// }
/// ```
pub fn md_isolate<'a>(
    content: &'a str,
    scope: MarkdownScope,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    match scope {
        MarkdownScope::Frontmatter => isolate_frontmatter(content, action),
        MarkdownScope::Prose => isolate_prose(content, action),
        MarkdownScope::CodeBlock => isolate_code_blocks(content, action),
        MarkdownScope::BlockQuote => isolate_block_quotes(content, action),
        MarkdownScope::Heading => isolate_headings(content, action),
        MarkdownScope::Stylized => isolate_stylized(content, action),
        MarkdownScope::Italicized => isolate_italicized(content, action),
        MarkdownScope::NonItalicized => isolate_non_italicized(content, action),
        MarkdownScope::Links => isolate_links(content, action),
        MarkdownScope::Images => isolate_images(content, action),
        MarkdownScope::Lists => isolate_lists(content, action),
        MarkdownScope::Tables => isolate_tables(content, action),
        MarkdownScope::FootnoteDefinitions => isolate_footnotes(content, action),
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

/// Isolates YAML frontmatter content.
fn isolate_frontmatter<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let pieces = if let Some((start, end)) = parse_frontmatter(content) {
        vec![validate_and_slice(content, start, end)?]
    } else {
        Vec::new()
    };
    Ok(apply_action(pieces, action))
}

/// Isolates prose text (text outside code blocks, block quotes, headings, lists).
fn isolate_prose<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();

    // Track nesting depth for elements we want to exclude
    let mut in_code_block = false;
    let mut block_quote_depth: u32 = 0;
    let mut heading_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Start(Tag::BlockQuote(_)) => block_quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => block_quote_depth = block_quote_depth.saturating_sub(1),
            Event::Start(Tag::Heading { .. }) => heading_depth += 1,
            Event::End(TagEnd::Heading(_)) => heading_depth = heading_depth.saturating_sub(1),
            Event::Text(_) | Event::SoftBreak | Event::HardBreak => {
                if !in_code_block && block_quote_depth == 0 && heading_depth == 0 {
                    let slice = validate_and_slice(content, range.start, range.end)?;
                    if !slice.trim().is_empty() || matches!(event, Event::SoftBreak | Event::HardBreak) {
                        pieces.push(slice);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates code block content.
fn isolate_code_blocks<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut in_code_block = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Text(_) if in_code_block => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates block quote content.
fn isolate_block_quotes<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut block_quote_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::BlockQuote(_)) => block_quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => block_quote_depth = block_quote_depth.saturating_sub(1),
            Event::Text(_) if block_quote_depth > 0 => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates heading text content.
fn isolate_headings<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut in_heading = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. }) => in_heading = true,
            Event::End(TagEnd::Heading(_)) => in_heading = false,
            Event::Text(_) | Event::Code(_) if in_heading => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates stylized content (bold, italic, strikethrough).
fn isolate_stylized<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut style_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Strong | Tag::Emphasis | Tag::Strikethrough) => style_depth += 1,
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough) => {
                style_depth = style_depth.saturating_sub(1);
            }
            Event::Text(_) if style_depth > 0 => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates italic (emphasis) content only.
fn isolate_italicized<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut emphasis_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Emphasis) => emphasis_depth += 1,
            Event::End(TagEnd::Emphasis) => emphasis_depth = emphasis_depth.saturating_sub(1),
            Event::Text(_) if emphasis_depth > 0 => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates all content except italic text.
fn isolate_non_italicized<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut emphasis_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Emphasis) => emphasis_depth += 1,
            Event::End(TagEnd::Emphasis) => emphasis_depth = emphasis_depth.saturating_sub(1),
            Event::Text(_) if emphasis_depth == 0 => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates link text and URLs.
fn isolate_links<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut in_link = false;
    let mut current_link_url: Option<Cow<'a, str>> = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = true;
                // Store URL as owned since dest_url is a CowStr from pulldown-cmark
                current_link_url = Some(Cow::Owned(dest_url.to_string()));
            }
            Event::End(TagEnd::Link) => {
                // Add the URL after the link text
                if let Some(url) = current_link_url.take() {
                    pieces.push(url);
                }
                in_link = false;
            }
            Event::Text(_) if in_link => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates image alt text and URLs.
fn isolate_images<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut in_image = false;
    let mut current_image_url: Option<Cow<'a, str>> = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                in_image = true;
                current_image_url = Some(Cow::Owned(dest_url.to_string()));
            }
            Event::End(TagEnd::Image) => {
                if let Some(url) = current_image_url.take() {
                    pieces.push(url);
                }
                in_image = false;
            }
            Event::Text(_) if in_image => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates list item content.
fn isolate_lists<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut list_depth: u32 = 0;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Text(_) if list_depth > 0 => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates table cell content.
fn isolate_tables<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut in_table = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Table(_)) => in_table = true,
            Event::End(TagEnd::Table) => in_table = false,
            Event::Text(_) if in_table => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

/// Isolates footnote definition content.
fn isolate_footnotes<'a>(
    content: &'a str,
    action: IsolateAction,
) -> Result<IsolateResult<'a>, IsolateError> {
    let parser = create_parser(content);
    let mut pieces = Vec::new();
    let mut in_footnote = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::FootnoteDefinition(_)) => in_footnote = true,
            Event::End(TagEnd::FootnoteDefinition) => in_footnote = false,
            Event::Text(_) if in_footnote => {
                let slice = validate_and_slice(content, range.start, range.end)?;
                pieces.push(slice);
            }
            _ => {}
        }
    }

    Ok(apply_action(pieces, action))
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Frontmatter tests
    // =========================================================================

    #[test]
    fn test_frontmatter_basic() {
        let content = "---\ntitle: Hello\nauthor: World\n---\n\n# Content";
        let result = md_isolate(content, MarkdownScope::Frontmatter, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("title: Hello"));
            assert!(items[0].contains("author: World"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_frontmatter_missing() {
        let content = "# No Frontmatter\n\nJust content.";
        let result = md_isolate(content, MarkdownScope::Frontmatter, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert!(items.is_empty());
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_frontmatter_with_dots_closer() {
        let content = "---\nkey: value\n...\n\nBody text";
        let result = md_isolate(content, MarkdownScope::Frontmatter, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("key: value"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Prose tests
    // =========================================================================

    #[test]
    fn test_prose_basic() {
        let content = "# Heading\n\nThis is prose text.\n\nMore prose here.";
        let result = md_isolate(content, MarkdownScope::Prose, IsolateAction::Concatenate(Some(" ".to_string())))
            .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            assert!(text.contains("This is prose text."));
            assert!(text.contains("More prose here."));
            // Heading should not be included
            assert!(!text.contains("Heading"));
        } else {
            panic!("Expected Concatenated result");
        }
    }

    #[test]
    fn test_prose_excludes_code_blocks() {
        let content = "Text before.\n\n```rust\nfn code() {}\n```\n\nText after.";
        let result = md_isolate(content, MarkdownScope::Prose, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(!combined.contains("fn code()"));
            assert!(combined.contains("Text before."));
            assert!(combined.contains("Text after."));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Code block tests
    // =========================================================================

    #[test]
    fn test_code_block_fenced() {
        let content = "# Example\n\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```";
        let result = md_isolate(content, MarkdownScope::CodeBlock, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("fn main()"));
            assert!(items[0].contains("println!"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_code_block_multiple() {
        let content = "```python\nprint('a')\n```\n\nText\n\n```js\nconsole.log('b');\n```";
        let result = md_isolate(content, MarkdownScope::CodeBlock, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            assert!(items[0].contains("print"));
            assert!(items[1].contains("console.log"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Block quote tests
    // =========================================================================

    #[test]
    fn test_block_quote_basic() {
        let content = "Normal text.\n\n> This is quoted.\n> More quote.\n\nAfter.";
        let result = md_isolate(content, MarkdownScope::BlockQuote, IsolateAction::Concatenate(Some(" ".to_string())))
            .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            assert!(text.contains("This is quoted."));
            assert!(text.contains("More quote."));
            assert!(!text.contains("Normal text."));
        } else {
            panic!("Expected Concatenated result");
        }
    }

    #[test]
    fn test_block_quote_nested() {
        let content = "> Outer quote\n>> Inner quote\n> Back to outer";
        let result = md_isolate(content, MarkdownScope::BlockQuote, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert!(!items.is_empty());
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("Outer quote"));
            assert!(combined.contains("Inner quote"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Heading tests
    // =========================================================================

    #[test]
    fn test_heading_all_levels() {
        let content = "# H1\n\n## H2\n\n### H3\n\n#### H4\n\n##### H5\n\n###### H6";
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 6);
            assert_eq!(items[0], "H1");
            assert_eq!(items[1], "H2");
            assert_eq!(items[2], "H3");
            assert_eq!(items[3], "H4");
            assert_eq!(items[4], "H5");
            assert_eq!(items[5], "H6");
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_heading_with_inline_code() {
        let content = "# Heading with `code`\n\nBody text.";
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::Concatenate(None))
            .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            assert!(text.contains("Heading with"));
            assert!(text.contains("code"));
        } else {
            panic!("Expected Concatenated result");
        }
    }

    // =========================================================================
    // Stylized tests
    // =========================================================================

    #[test]
    fn test_stylized_bold() {
        let content = "Normal **bold** text.";
        let result = md_isolate(content, MarkdownScope::Stylized, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "bold");
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_stylized_mixed() {
        let content = "**bold** and *italic* and ~~strike~~";
        let result = md_isolate(content, MarkdownScope::Stylized, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 3);
            assert!(items.iter().any(|i| i == "bold"));
            assert!(items.iter().any(|i| i == "italic"));
            assert!(items.iter().any(|i| i == "strike"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Italicized tests
    // =========================================================================

    #[test]
    fn test_italicized_basic() {
        let content = "Normal *italic* text.";
        let result = md_isolate(content, MarkdownScope::Italicized, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "italic");
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_italicized_excludes_bold() {
        let content = "**bold** and *italic* and ~~strike~~";
        let result = md_isolate(content, MarkdownScope::Italicized, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "italic");
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // NonItalicized tests
    // =========================================================================

    #[test]
    fn test_non_italicized() {
        let content = "Normal *italic* text.";
        let result = md_isolate(content, MarkdownScope::NonItalicized, IsolateAction::Concatenate(None))
            .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            assert!(text.contains("Normal"));
            assert!(text.contains("text."));
            assert!(!text.contains("italic"));
        } else {
            panic!("Expected Concatenated result");
        }
    }

    // =========================================================================
    // Links tests
    // =========================================================================

    #[test]
    fn test_links_inline() {
        let content = "Check out [Example](https://example.com) for more.";
        let result = md_isolate(content, MarkdownScope::Links, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            assert!(items.iter().any(|i| i == "Example"));
            assert!(items.iter().any(|i| i == "https://example.com"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_links_multiple() {
        let content = "[First](https://first.com) and [Second](https://second.com)";
        let result = md_isolate(content, MarkdownScope::Links, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 4);
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Images tests
    // =========================================================================

    #[test]
    fn test_images_basic() {
        let content = "![Alt text](https://example.com/image.png)";
        let result = md_isolate(content, MarkdownScope::Images, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            assert!(items.iter().any(|i| i == "Alt text"));
            assert!(items.iter().any(|i| i.contains("image.png")));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_images_empty_alt() {
        let content = "![](https://example.com/img.jpg)";
        let result = md_isolate(content, MarkdownScope::Images, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            // Only URL should be present
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("img.jpg"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Lists tests
    // =========================================================================

    #[test]
    fn test_lists_unordered() {
        let content = "- Item one\n- Item two\n- Item three";
        let result = md_isolate(content, MarkdownScope::Lists, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 3);
            assert!(items.iter().any(|i| i == "Item one"));
            assert!(items.iter().any(|i| i == "Item two"));
            assert!(items.iter().any(|i| i == "Item three"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_lists_ordered() {
        let content = "1. First\n2. Second\n3. Third";
        let result = md_isolate(content, MarkdownScope::Lists, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 3);
            assert!(items.iter().any(|i| i == "First"));
            assert!(items.iter().any(|i| i == "Second"));
            assert!(items.iter().any(|i| i == "Third"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_lists_nested() {
        let content = "- Parent\n  - Child\n    - Grandchild";
        let result = md_isolate(content, MarkdownScope::Lists, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("Parent"));
            assert!(combined.contains("Child"));
            assert!(combined.contains("Grandchild"));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Tables tests (GFM)
    // =========================================================================

    #[test]
    fn test_tables_basic() {
        let content = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let result = md_isolate(content, MarkdownScope::Tables, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert!(!items.is_empty());
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("Header 1"));
            assert!(combined.contains("Header 2"));
            assert!(combined.contains("Cell 1"));
            assert!(combined.contains("Cell 2"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_tables_empty_when_no_tables() {
        let content = "# Heading\n\nJust regular text.";
        let result = md_isolate(content, MarkdownScope::Tables, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert!(items.is_empty());
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Footnote tests (GFM)
    // =========================================================================

    #[test]
    fn test_footnotes_basic() {
        let content = "Text with footnote[^1].\n\n[^1]: This is the footnote content.";
        let result = md_isolate(content, MarkdownScope::FootnoteDefinitions, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("This is the footnote content."));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_footnotes_multiple() {
        let content = "First[^a] and second[^b].\n\n[^a]: Note A.\n\n[^b]: Note B.";
        let result = md_isolate(content, MarkdownScope::FootnoteDefinitions, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("Note A."));
            assert!(combined.contains("Note B."));
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Action tests
    // =========================================================================

    #[test]
    fn test_action_concatenate_no_delimiter() {
        let content = "# One\n\n## Two\n\n### Three";
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::Concatenate(None))
            .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            assert_eq!(text, "OneTwoThree");
        } else {
            panic!("Expected Concatenated result");
        }
    }

    #[test]
    fn test_action_concatenate_with_delimiter() {
        let content = "# One\n\n## Two\n\n### Three";
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::Concatenate(Some(", ".to_string())))
            .unwrap();

        if let IsolateResult::Concatenated(text) = result {
            assert_eq!(text, "One, Two, Three");
        } else {
            panic!("Expected Concatenated result");
        }
    }

    // =========================================================================
    // Edge case tests
    // =========================================================================

    #[test]
    fn test_empty_content() {
        let content = "";
        let result = md_isolate(content, MarkdownScope::Prose, IsolateAction::LeaveAsVector)
            .unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn test_zero_copy_borrowing() {
        let content = "# Heading";
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            // Verify it's a borrowed slice
            match &items[0] {
                Cow::Borrowed(s) => assert_eq!(*s, "Heading"),
                Cow::Owned(_) => panic!("Expected Borrowed, got Owned"),
            }
        } else {
            panic!("Expected Vector result");
        }
    }

    // =========================================================================
    // Additional edge case tests
    // =========================================================================

    #[test]
    fn test_unicode_content() {
        let content = "# \u{1F600} Emoji Title \u{1F389}\n\n\u{1F680} Body emoji";
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("\u{1F600}"));
            assert!(items[0].contains("\u{1F389}"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_multibyte_utf8_boundaries() {
        // Test with various multi-byte UTF-8 characters
        let content = "# \u{00E9}\u{00E8}\u{00EA}"; // e-acute, e-grave, e-circumflex (2 bytes each)
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "\u{00E9}\u{00E8}\u{00EA}");
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_cjk_characters() {
        // Test with Chinese, Japanese, Korean characters
        let content = "# \u{4E2D}\u{6587}\u{65E5}\u{672C}\u{8A9E}"; // Chinese/Japanese/Korean chars
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("\u{4E2D}"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_whitespace_only_content() {
        let content = "   \n\n   \t   \n";
        let result = md_isolate(content, MarkdownScope::Prose, IsolateAction::LeaveAsVector)
            .unwrap();

        // Whitespace-only content should return empty result
        if let IsolateResult::Vector(items) = result {
            // Filter out pure whitespace
            let non_empty: Vec<_> = items.iter().filter(|s| !s.trim().is_empty()).collect();
            assert!(non_empty.is_empty());
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_deeply_nested_lists() {
        let content = "- Level 1\n  - Level 2\n    - Level 3\n      - Level 4";
        let result = md_isolate(content, MarkdownScope::Lists, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("Level 1"));
            assert!(combined.contains("Level 2"));
            assert!(combined.contains("Level 3"));
            assert!(combined.contains("Level 4"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_code_block_with_language_info() {
        let content = "```typescript\nconst x: number = 42;\n```";
        let result = md_isolate(content, MarkdownScope::CodeBlock, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("const x"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_strikethrough_in_stylized() {
        let content = "Normal ~~strikethrough text~~ more normal.";
        let result = md_isolate(content, MarkdownScope::Stylized, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], "strikethrough text");
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_reference_style_links() {
        let content = "[Link text][ref]\n\n[ref]: https://example.com";
        let result = md_isolate(content, MarkdownScope::Links, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            // Should capture link text and URL
            assert!(items.iter().any(|i| i.as_ref() == "Link text"));
            assert!(items.iter().any(|i| i.contains("example.com")));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_autolinks() {
        let content = "Visit <https://example.com> for more info.";
        let result = md_isolate(content, MarkdownScope::Links, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            // Autolinks should be captured
            assert!(!items.is_empty());
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_setext_style_headings() {
        let content = "Heading Level 1\n================\n\nHeading Level 2\n----------------";
        let result = md_isolate(content, MarkdownScope::Heading, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            assert_eq!(items.len(), 2);
            assert!(items.iter().any(|i| i.as_ref() == "Heading Level 1"));
            assert!(items.iter().any(|i| i.as_ref() == "Heading Level 2"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_horizontal_rules_dont_affect_prose() {
        let content = "Before rule.\n\n---\n\nAfter rule.";
        let result = md_isolate(content, MarkdownScope::Prose, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("Before rule."));
            assert!(combined.contains("After rule."));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_indented_code_block() {
        let content = "Text before.\n\n    fn indented_code() {}\n\nText after.";
        let result = md_isolate(content, MarkdownScope::CodeBlock, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            // Indented code blocks should be captured
            let combined: String = items.iter().map(|c| c.as_ref()).collect();
            assert!(combined.contains("indented_code"));
        } else {
            panic!("Expected Vector result");
        }
    }

    #[test]
    fn test_inline_code_not_in_code_block() {
        let content = "Text with `inline code` here.\n\n```\nblock code\n```";
        let result = md_isolate(content, MarkdownScope::CodeBlock, IsolateAction::LeaveAsVector)
            .unwrap();

        if let IsolateResult::Vector(items) = result {
            // Only block code should be captured, not inline
            assert_eq!(items.len(), 1);
            assert!(items[0].contains("block code"));
            assert!(!items[0].contains("inline code"));
        } else {
            panic!("Expected Vector result");
        }
    }
}
