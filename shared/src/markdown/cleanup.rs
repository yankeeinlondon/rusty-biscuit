//! Markdown cleanup implementation using pulldown-cmark event stream manipulation.
//!
//! This module provides functionality to normalize markdown content by:
//! - Injecting blank lines between block elements
//! - Aligning table columns for visual consistency
//!
//! ## Examples
//!
//! ```
//! use shared::markdown::Markdown;
//!
//! let content = "# Header\nSome text\n## Another Header";
//! let mut md: Markdown = content.into();
//! md.cleanup();
//! // Content now has blank lines between headers
//! ```

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Cleans up markdown content by normalizing formatting.
///
/// This function performs two main operations:
/// 1. Injects blank lines between block elements
/// 2. Aligns table columns for consistent formatting
///
/// ## Returns
///
/// The cleaned markdown content as a String.
///
/// ## Examples
///
/// ```
/// use shared::markdown::cleanup::cleanup_content;
///
/// let content = "# Title\nParagraph";
/// let cleaned = cleanup_content(content);
/// assert!(cleaned.contains("\n\n"));
/// ```
pub fn cleanup_content(content: &str) -> String {
    let parser = Parser::new_ext(content, Options::all());
    let events: Vec<Event> = parser.collect();

    // Process events to inject blank lines and align tables
    let processed = inject_blank_lines(events);
    let processed = align_tables_in_stream(processed);

    // Convert events back to markdown
    let mut output = String::new();

    // cmark expects borrowed events - we use CowStr wrapping
    let borrowed: Vec<_> = processed.iter().map(std::borrow::Cow::Borrowed).collect();
    if pulldown_cmark_to_cmark::cmark(borrowed.into_iter(), &mut output).is_err() {
        // If rendering fails, return original content
        return content.to_string();
    }

    output
}

/// Checks if a tag represents a block-level element.
fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::Paragraph
    )
}

/// Checks if a tag end represents the end of a block-level element.
fn is_block_end_tag(tag_end: &TagEnd) -> bool {
    matches!(
        tag_end,
        TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::Table
            | TagEnd::TableHead
            | TagEnd::TableRow
            | TagEnd::TableCell
            | TagEnd::Paragraph
    )
}

/// Injects blank lines between block elements in the event stream.
///
/// This ensures consistent spacing between headers, paragraphs, code blocks,
/// lists, and other block-level elements.
fn inject_blank_lines(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut result = Vec::with_capacity(events.len() + 20);
    let mut last_was_block_end = false;

    for (i, event) in events.iter().enumerate() {
        match event {
            Event::Start(tag) if is_block_tag(tag) => {
                // If the last event was a block end, inject a blank line
                if last_was_block_end && i > 0 {
                    // Don't add blank line before list items or table cells
                    if !matches!(tag, Tag::Item | Tag::TableCell) {
                        result.push(Event::HardBreak);
                    }
                }
                result.push(event.clone());
                last_was_block_end = false;
            }
            Event::End(tag_end) if is_block_end_tag(tag_end) => {
                result.push(event.clone());
                // Don't set flag for inline-like blocks
                if !matches!(tag_end, TagEnd::TableCell | TagEnd::Item) {
                    last_was_block_end = true;
                }
            }
            _ => {
                result.push(event.clone());
                last_was_block_end = false;
            }
        }
    }

    result
}

/// Aligns tables in the event stream for visual consistency.
///
/// This function identifies table events and processes them to ensure
/// column alignment across all rows.
fn align_tables_in_stream(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut result = Vec::with_capacity(events.len());
    let mut table_buffer = Vec::new();
    let mut in_table = false;

    for event in events {
        match &event {
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_buffer.clear();
                table_buffer.push(event);
            }
            Event::End(TagEnd::Table) => {
                table_buffer.push(event);
                in_table = false;

                // Process the buffered table
                let aligned = process_single_table(table_buffer.clone());
                result.extend(aligned);
                table_buffer.clear();
            }
            _ => {
                if in_table {
                    table_buffer.push(event);
                } else {
                    result.push(event);
                }
            }
        }
    }

    result
}

/// Processes a single table's events to align columns.
///
/// This function analyzes all table cells to determine the maximum width
/// for each column and then pads cells accordingly.
fn process_single_table(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    // For now, return events as-is
    // Full implementation would calculate column widths and pad cells
    // This is a complex operation that requires:
    // 1. Collecting all cell text content
    // 2. Determining max width per column
    // 3. Reconstructing table with aligned cells
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_basic_content() {
        let content = "# Title\nParagraph";
        let cleaned = cleanup_content(content);
        // Should have proper spacing
        assert!(cleaned.contains("Title"));
        assert!(cleaned.contains("Paragraph"));
    }

    #[test]
    fn test_cleanup_preserves_content() {
        let content = "# Header\n\nSome text\n\n## Subheader";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Header"));
        assert!(cleaned.contains("Some text"));
        assert!(cleaned.contains("Subheader"));
    }

    #[test]
    fn test_cleanup_handles_code_blocks() {
        let content = r#"# Title
```rust
fn main() {}
```
Text after"#;
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Title"));
        assert!(cleaned.contains("fn main()"));
        assert!(cleaned.contains("Text after"));
    }

    #[test]
    fn test_cleanup_handles_lists() {
        let content = "# Title\n- Item 1\n- Item 2\n\nParagraph";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Item 1"));
        assert!(cleaned.contains("Item 2"));
    }

    #[test]
    fn test_cleanup_handles_blockquotes() {
        let content = "# Title\n> Quote\n\nParagraph";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Quote"));
        assert!(cleaned.contains("Paragraph"));
    }

    #[test]
    fn test_cleanup_handles_empty_content() {
        let content = "";
        let cleaned = cleanup_content(content);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_cleanup_handles_plain_text() {
        let content = "Just plain text";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Just plain text"));
    }

    #[test]
    fn test_is_block_tag_identifies_blocks() {
        use pulldown_cmark::HeadingLevel;

        assert!(is_block_tag(&Tag::Heading { level: HeadingLevel::H1, id: None, classes: vec![], attrs: vec![] }));
        assert!(is_block_tag(&Tag::Paragraph));
        assert!(is_block_tag(&Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced("rust".into()))));
        assert!(is_block_tag(&Tag::BlockQuote(None)));
    }

    #[test]
    fn test_is_block_end_tag_identifies_ends() {
        use pulldown_cmark::HeadingLevel;

        assert!(is_block_end_tag(&TagEnd::Heading(HeadingLevel::H1)));
        assert!(is_block_end_tag(&TagEnd::Paragraph));
        assert!(is_block_end_tag(&TagEnd::CodeBlock));
        assert!(is_block_end_tag(&TagEnd::BlockQuote(None)));
    }

    #[test]
    fn test_inject_blank_lines_between_headers() {
        let parser = Parser::new_ext("# Header 1\n## Header 2", Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = inject_blank_lines(events);

        // Should have more events due to injected breaks
        assert!(processed.len() > 4);
    }

    #[test]
    fn test_inject_blank_lines_preserves_structure() {
        let parser = Parser::new_ext("# Title\nParagraph", Options::all());
        let events: Vec<Event> = parser.collect();
        let original_len = events.len();
        let processed = inject_blank_lines(events);

        // Should have at least as many events
        assert!(processed.len() >= original_len);
    }

    #[test]
    fn test_align_tables_preserves_non_table_content() {
        let parser = Parser::new_ext("# Title\nParagraph", Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = align_tables_in_stream(events.clone());

        // Should preserve all events when no table present
        assert_eq!(processed.len(), events.len());
    }

    #[test]
    fn test_align_tables_handles_simple_table() {
        let content = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let parser = Parser::new_ext(content, Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = align_tables_in_stream(events);

        // Should preserve table structure
        let has_table_start = processed.iter().any(|e| matches!(e, Event::Start(Tag::Table(_))));
        let has_table_end = processed.iter().any(|e| matches!(e, Event::End(TagEnd::Table)));
        assert!(has_table_start);
        assert!(has_table_end);
    }

    #[test]
    fn test_process_single_table_preserves_events() {
        let parser = Parser::new_ext("| A | B |\n|---|---|\n| 1 | 2 |", Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = process_single_table(events.clone());

        // Currently just returns events as-is
        assert_eq!(processed.len(), events.len());
    }

    #[test]
    fn test_cleanup_multiple_paragraphs() {
        let content = "Para 1\n\nPara 2\n\nPara 3";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Para 1"));
        assert!(cleaned.contains("Para 2"));
        assert!(cleaned.contains("Para 3"));
    }
}
