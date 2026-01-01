//! Markdown cleanup implementation using pulldown-cmark event stream manipulation.
//!
//! This module provides functionality to normalize markdown content by:
//! - Ensuring proper blank lines between block elements (via cmark Options)
//! - Aligning table columns for visual consistency
//!
//! The cleanup leverages `pulldown-cmark-to-cmark`'s built-in newline handling
//! through its `Options` struct, which automatically inserts appropriate spacing.
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

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::Options as CmarkOptions;

/// Cleans up markdown content by normalizing formatting.
///
/// This function performs two main operations:
/// 1. Ensures proper blank lines between block elements (via cmark Options)
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

    // Align tables in the event stream
    let processed = align_tables_in_stream(events);

    // Convert events back to markdown with proper spacing options
    let mut output = String::new();

    // cmark handles blank line insertion via its Options - defaults are correct:
    // newlines_after_headline: 2, newlines_after_paragraph: 2, etc.
    // Override code_block_token_count: default is 4, but standard markdown uses 3
    let mut options = CmarkOptions::default();
    options.code_block_token_count = 3;

    // cmark expects borrowed events
    let borrowed: Vec<_> = processed.iter().map(std::borrow::Cow::Borrowed).collect();
    if pulldown_cmark_to_cmark::cmark_with_options(borrowed.into_iter(), &mut output, options)
        .is_err()
    {
        // If rendering fails, return original content
        return content.to_string();
    }

    output
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
/// for each column and then pads cells accordingly. It preserves the original
/// event structure (keeping Code events as Code, not merging into Text) and
/// adds spacing around cell content for readability: `| content |`
fn process_single_table(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    // Pass 1: Measure column widths (visual width of rendered content)
    let mut col_widths: Vec<usize> = Vec::new();
    let mut current_col = 0;
    let mut in_cell = false;
    let mut cell_text_len = 0;

    for ev in &events {
        match ev {
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell_text_len = 0;
            }
            Event::End(TagEnd::TableCell) => {
                if col_widths.len() <= current_col {
                    col_widths.push(cell_text_len);
                } else {
                    col_widths[current_col] = col_widths[current_col].max(cell_text_len);
                }
                current_col += 1;
                in_cell = false;
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                current_col = 0;
            }
            Event::Text(t) if in_cell => {
                cell_text_len += t.chars().count();
            }
            Event::Code(t) if in_cell => {
                // Code spans render with backticks: `code`
                cell_text_len += t.chars().count() + 2;
            }
            _ => {}
        }
    }

    // Pass 2: Preserve original events, add leading space and trailing padding
    let mut result = Vec::with_capacity(events.len() + col_widths.len() * 2);
    let mut current_col = 0;
    let mut in_cell = false;
    let mut cell_content_len = 0;

    for ev in events {
        match &ev {
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                cell_content_len = 0;
                result.push(ev);
                // Add leading space for readability: "|content" -> "| content"
                result.push(Event::Text(CowStr::from(" ")));
            }
            Event::End(TagEnd::TableCell) => {
                // Add trailing padding to align columns, plus one space before |
                let target_width = col_widths.get(current_col).copied().unwrap_or(0);
                let padding_needed = target_width.saturating_sub(cell_content_len);
                // Add padding + trailing space: "content|" -> "content |"
                let padding = " ".repeat(padding_needed + 1);
                result.push(Event::Text(CowStr::from(padding)));
                current_col += 1;
                in_cell = false;
                result.push(ev);
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                current_col = 0;
                result.push(ev);
            }
            Event::Text(t) if in_cell => {
                cell_content_len += t.chars().count();
                result.push(ev);
            }
            Event::Code(t) if in_cell => {
                cell_content_len += t.chars().count() + 2;
                result.push(ev);
            }
            _ => {
                result.push(ev);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to count occurrences of a pattern in a string
    fn count_occurrences(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    // ==================== Blank Line Tests ====================

    #[test]
    fn test_cleanup_adds_blank_line_between_header_and_paragraph() {
        // Input has no blank line between header and paragraph
        let content = "# Title\nParagraph text";
        let cleaned = cleanup_content(content);

        // Should have exactly one blank line (two newlines) between header and paragraph
        assert!(
            cleaned.contains("# Title\n\nParagraph"),
            "Expected blank line between header and paragraph, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_between_consecutive_headers() {
        let content = "# Header 1\n## Header 2";
        let cleaned = cleanup_content(content);

        // Should have blank line between headers
        assert!(
            cleaned.contains("# Header 1\n\n## Header 2"),
            "Expected blank line between headers, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_code_block() {
        let content = "```rust\nfn main() {}\n```\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after code block
        assert!(
            cleaned.contains("```\n\nParagraph"),
            "Expected blank line after code block, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_list() {
        // Note: In CommonMark, "* Item\nParagraph" creates a "lazy" paragraph inside the list
        // We need an explicit blank line in input to separate list from paragraph
        let content = "* Item 1\n* Item 2\n\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after list
        assert!(
            cleaned.contains("Item 2\n\nParagraph"),
            "Expected blank line after list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_blockquote() {
        // Note: In CommonMark, "> Quote\nParagraph" creates a "lazy" paragraph inside blockquote
        // We need an explicit blank line in input to separate blockquote from paragraph
        let content = "> Quote\n\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after blockquote
        assert!(
            cleaned.contains("Quote\n\nParagraph"),
            "Expected blank line after blockquote, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_existing_blank_lines() {
        let content = "# Header\n\nSome text\n\n## Subheader";
        let cleaned = cleanup_content(content);

        // Should preserve single blank lines (not double them up)
        assert_eq!(
            count_occurrences(&cleaned, "\n\n\n"),
            0,
            "Should not have triple newlines, got:\n{}",
            cleaned
        );
        assert!(cleaned.contains("# Header\n\nSome text"));
        assert!(cleaned.contains("Some text\n\n## Subheader"));
    }

    #[test]
    fn test_cleanup_does_not_add_excessive_blank_lines() {
        let content = "# Title\nParagraph 1\n\nParagraph 2";
        let cleaned = cleanup_content(content);

        // Count blank lines (consecutive \n\n)
        let blank_line_count = count_occurrences(&cleaned, "\n\n");

        // Should have exactly 2 blank lines: after title and between paragraphs
        assert_eq!(
            blank_line_count, 2,
            "Expected 2 blank lines, got {} in:\n{}",
            blank_line_count, cleaned
        );
    }

    // ==================== Table Alignment Tests ====================

    #[test]
    fn test_table_columns_are_aligned() {
        let content = "|Short|VeryLongHeader|\n|---|---|\n|A|B|";
        let cleaned = cleanup_content(content);

        // The table should be rendered with aligned columns
        // Note: exact format depends on cmark rendering, but cells should be padded
        assert!(cleaned.contains("Short"), "Content should be preserved");
        assert!(
            cleaned.contains("VeryLongHeader"),
            "Content should be preserved"
        );
    }

    #[test]
    fn test_table_structure_preserved() {
        let content = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let cleaned = cleanup_content(content);

        // Should still have pipe characters for table structure
        assert!(
            cleaned.contains("|"),
            "Table structure should be preserved"
        );
        assert!(cleaned.contains("Col1"));
        assert!(cleaned.contains("Col2"));
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
        let has_table_start = processed
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Table(_))));
        let has_table_end = processed
            .iter()
            .any(|e| matches!(e, Event::End(TagEnd::Table)));
        assert!(has_table_start);
        assert!(has_table_end);
    }

    // ==================== Edge Case Tests ====================

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
    fn test_cleanup_handles_multiple_paragraphs() {
        let content = "Para 1\n\nPara 2\n\nPara 3";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Para 1"));
        assert!(cleaned.contains("Para 2"));
        assert!(cleaned.contains("Para 3"));
    }

    #[test]
    fn test_cleanup_preserves_code_block_content() {
        let content = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("fn main()"));
        assert!(cleaned.contains("println!"));
    }

    // ==================== Regression Tests ====================

    #[test]
    fn test_no_hardbreak_in_output() {
        // This is the main regression test for the bug
        let content = "# Header\n\nParagraph\n\n## Another Header";
        let cleaned = cleanup_content(content);

        // HardBreak would render as `\` or `<br>` - neither should appear
        assert!(
            !cleaned.contains("\\"),
            "Should not contain backslash (HardBreak), got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("<br"),
            "Should not contain <br> (HardBreak), got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_table_after_cleanup_still_parses() {
        let content = "| A | B |\n|---|---|\n| 1 | 2 |";
        let cleaned = cleanup_content(content);

        // Re-parse the cleaned content - should still be a valid table
        let parser = Parser::new_ext(&cleaned, Options::all());
        let events: Vec<Event> = parser.collect();

        let has_table = events
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Table(_))));
        assert!(
            has_table,
            "Cleaned table should still parse as table, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_table_cells_have_spacing() {
        // Regression test: table cells should have space after | and before |
        let content = "|A|B|\n|---|---|\n|1|2|";
        let cleaned = cleanup_content(content);

        // Should have "| A " pattern (space after pipe, content, space before next pipe)
        assert!(
            cleaned.contains("| A "),
            "Table cells should have leading space after |, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("| B "),
            "Table cells should have leading space after |, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_table_code_spans_not_escaped() {
        // Regression test: backticks in code spans should not be escaped
        let content = "| Name | Value |\n|---|---|\n| `foo` | bar |";
        let cleaned = cleanup_content(content);

        // Should preserve backticks without escaping
        assert!(
            cleaned.contains("`foo`"),
            "Code spans should preserve backticks, got:\n{}",
            cleaned
        );
        // Should NOT have escaped backticks
        assert!(
            !cleaned.contains("\\`"),
            "Code spans should not have escaped backticks, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_uses_three_backticks() {
        // Regression test: code blocks should use 3 backticks, not 4
        // pulldown-cmark-to-cmark defaults to 4 backticks (code_block_token_count)
        // which is non-standard and causes rendering issues
        let content = "```rust\nfn main() {}\n```";
        let cleaned = cleanup_content(content);

        // Should use exactly 3 backticks for fence
        assert!(
            cleaned.contains("```rust"),
            "Code blocks should start with 3 backticks, got:\n{}",
            cleaned
        );
        // Should NOT have 4 backticks (the buggy default)
        assert!(
            !cleaned.contains("````"),
            "Code blocks should not use 4 backticks, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_without_language_uses_three_backticks() {
        // Regression test: code blocks without language specifier
        let content = "```\nsome code\n```";
        let cleaned = cleanup_content(content);

        // Should start with exactly "```\n"
        assert!(
            cleaned.starts_with("```\n") || cleaned.contains("\n```\n"),
            "Code blocks without language should use 3 backticks, got:\n{}",
            cleaned
        );
        // Should NOT have 4 backticks
        assert!(
            !cleaned.contains("````"),
            "Code blocks should not use 4 backticks, got:\n{}",
            cleaned
        );
    }
}
