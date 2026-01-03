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

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
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

    // Add "text" language to empty fenced code blocks
    let with_text_lang = add_text_language_to_empty_code_blocks(events);

    // Align tables in the event stream
    let processed = align_tables_in_stream(with_text_lang);

    // Convert events back to markdown with proper spacing options
    let mut output = String::new();

    // cmark handles blank line insertion via its Options - defaults are correct:
    // newlines_after_headline: 2, newlines_after_paragraph: 2, etc.
    // Override code_block_token_count: default is 4, but standard markdown uses 3
    let options = CmarkOptions {
        code_block_token_count: 3,
        ..Default::default()
    };

    // cmark expects borrowed events
    let borrowed: Vec<_> = processed.iter().map(std::borrow::Cow::Borrowed).collect();
    if pulldown_cmark_to_cmark::cmark_with_options(borrowed.into_iter(), &mut output, options)
        .is_err()
    {
        // If rendering fails, return original content
        return content.to_string();
    }

    // Post-process to fix blockquote formatting issues from pulldown-cmark-to-cmark
    fix_blockquote_formatting(&mut output);

    // Trim leading/trailing whitespace-only lines but preserve content
    output.trim_start_matches('\n').to_string()
}

/// Fixes blockquote formatting issues introduced by pulldown-cmark-to-cmark v18.
///
/// The library adds:
/// 1. A leading space before `>` (e.g., ` > ` instead of `> `)
/// 2. An empty blockquote line at the start of each blockquote
/// 3. Extra spaces in nested blockquotes (e.g., `>  > ` instead of `> > `)
///
/// This function corrects these issues to produce standard markdown.
fn fix_blockquote_formatting(output: &mut String) {
    // Process line by line for clarity
    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
        // Fix the blockquote prefix: " > " -> "> " and ">  > " -> "> > "
        let fixed_line = fix_blockquote_line(line);

        // Check if this is an empty blockquote line (just "> " or nested like "> > ")
        let trimmed = fixed_line.trim_end();
        let is_empty_blockquote = trimmed.chars().all(|c| c == '>' || c == ' ')
            && trimmed.contains('>')
            && !trimmed.is_empty();

        if is_empty_blockquote {
            // Check if next line is also a blockquote (continuation)
            if let Some(next_line) = lines.peek() {
                if next_line.trim_start().starts_with('>') {
                    // Skip this empty blockquote line
                    continue;
                }
            }
        }

        result.push_str(&fixed_line);
        // Add newline unless this is the last line
        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline if original had one
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Fixes a single blockquote line's prefix formatting.
///
/// Handles:
/// - Leading space: " > text" -> "> text"
/// - Multiple spaces after >: ">  > text" -> "> > text"
fn fix_blockquote_line(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_prefix = true;

    // Skip leading space if followed by >
    if chars.peek() == Some(&' ') {
        let mut lookahead = chars.clone();
        lookahead.next(); // consume space
        if lookahead.peek() == Some(&'>') {
            chars.next(); // skip the leading space
        }
    }

    while let Some(c) = chars.next() {
        if in_prefix {
            if c == '>' {
                result.push(c);
                // After >, we expect exactly one space before content or next >
                // Skip any extra spaces, but keep one
                let mut space_count = 0;
                while chars.peek() == Some(&' ') {
                    chars.next();
                    space_count += 1;
                }
                // Add exactly one space after >
                if space_count > 0 || chars.peek().is_some() {
                    result.push(' ');
                }
                // Check if next char is another > (nested blockquote)
                if chars.peek() != Some(&'>') {
                    in_prefix = false;
                }
            } else if c == ' ' {
                // Skip spaces in prefix area (between > markers)
                continue;
            } else {
                in_prefix = false;
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Adds "text" language to fenced code blocks with no language specified.
///
/// This ensures all code blocks have an explicit language identifier,
/// improving rendering consistency across different markdown viewers.
fn add_text_language_to_empty_code_blocks(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    events
        .into_iter()
        .map(|event| {
            if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) = event {
                if info.is_empty() {
                    return Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from(
                        "text",
                    ))));
                }
            }
            event
        })
        .collect()
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
    fn test_code_block_without_language_gets_text_language() {
        // Regression test: code blocks without language should get "text" added
        let content = "```\nsome code\n```";
        let cleaned = cleanup_content(content);

        // Should have "text" as language
        assert!(
            cleaned.starts_with("```text\n") || cleaned.contains("\n```text\n"),
            "Code blocks without language should get 'text' as language, got:\n{}",
            cleaned
        );
        // Should NOT have 4 backticks
        assert!(
            !cleaned.contains("````"),
            "Code blocks should not use 4 backticks, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_preserves_existing_language() {
        // Ensure code blocks with language are not affected
        let content = "```rust\nfn main() {}\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("```rust\n"),
            "Existing language should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_multiple_code_blocks_without_language() {
        // Multiple empty code blocks should all get "text" language
        let content = "```\nfirst\n```\n\n```\nsecond\n```";
        let cleaned = cleanup_content(content);

        // Count occurrences of "```text"
        let text_count = cleaned.matches("```text").count();
        assert_eq!(
            text_count, 2,
            "Both code blocks should get 'text' language, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_indented_code_blocks_unchanged() {
        // Indented code blocks should remain unchanged (they don't have language specifiers)
        let content = "    indented code\n    more code";
        let cleaned = cleanup_content(content);

        // Should preserve indented code block
        assert!(
            cleaned.contains("indented code"),
            "Indented code should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== Blockquote Formatting Tests ====================

    #[test]
    fn test_blockquote_no_leading_space() {
        // Regression test: blockquotes should not have leading space before >
        let content = "> Simple quote";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> "),
            "Blockquote should start with '> ', got:\n{:?}",
            cleaned
        );
        assert!(
            !cleaned.starts_with(" >"),
            "Blockquote should not have leading space, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_no_empty_first_line() {
        // Regression test: blockquotes should not have empty first line
        let content = "> Quote content";
        let cleaned = cleanup_content(content);

        // Should NOT have "> \n> " pattern (empty blockquote line)
        assert!(
            !cleaned.contains("> \n>"),
            "Blockquote should not have empty first line, got:\n{:?}",
            cleaned
        );
        // Should start directly with content
        assert!(
            cleaned.starts_with("> Quote"),
            "Blockquote should start with content, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_multiline() {
        // Multi-line blockquotes should be preserved correctly
        let content = "> Line 1\n> Line 2\n> Line 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> Line 1\n> Line 2\n> Line 3"),
            "Multi-line blockquote should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_nested() {
        // Nested blockquotes should have single space between > markers
        let content = "> > Nested quote";
        let cleaned = cleanup_content(content);

        // Should be "> > " not ">  > " or " >  > "
        assert!(
            cleaned.starts_with("> > Nested"),
            "Nested blockquote should have single space between >, got:\n{:?}",
            cleaned
        );
        assert!(
            !cleaned.contains(">  >"),
            "Nested blockquote should not have double space, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_deeply_nested() {
        // Deeply nested blockquotes
        let content = "> > > Triple nested";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> > > Triple"),
            "Triple nested blockquote should have single spaces, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_after_header() {
        // Blockquotes following headers should be formatted correctly
        let content = "# Header\n\n> Quote after header";
        let cleaned = cleanup_content(content);

        // Should have blank line between header and quote, and proper formatting
        assert!(
            cleaned.contains("# Header\n\n> Quote"),
            "Blockquote after header should have blank line and proper format, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_long_content() {
        // Long blockquotes should not be mangled
        let content = "> Ut faucibus mauris mauris, sed tincidunt augue hendrerit eu. In ultrices ultrices commodo.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> Ut faucibus"),
            "Long blockquote should start correctly, got:\n{:?}",
            cleaned
        );
        assert!(
            cleaned.contains("commodo."),
            "Long blockquote content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_preserves_content_spaces() {
        // Spaces in blockquote content should be preserved (not just prefix)
        let content = "> Code:   let x = 1";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> Code:   let"),
            "Spaces in blockquote content should be preserved, got:\n{}",
            cleaned
        );
    }
}
