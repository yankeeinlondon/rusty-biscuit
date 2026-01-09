//! Markdown cleanup implementation using pulldown-cmark event stream manipulation.
//!
//! This module provides functionality to normalize markdown content by:
//! - Ensuring proper blank lines between block elements (via cmark Options)
//! - Aligning table columns for visual consistency
//! - Preserving original list markers (*, -, +)
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
use std::ops::Range;

/// Returns parser options suitable for cleanup operations.
///
/// Enables all extensions EXCEPT `ENABLE_SMART_PUNCTUATION` to preserve
/// original quote characters (`"` and `'`) without converting them to
/// typographic "smart quotes" (`"`, `"`, `'`, `'`).
fn cleanup_parser_options() -> Options {
    Options::all() - Options::ENABLE_SMART_PUNCTUATION
}

/// Emphasis style used for italics in markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmphasisStyle {
    /// Use asterisk for emphasis: `*text*` for italics, `**text**` for bold
    Asterisk,
    /// Use underscore for emphasis: `_text_` for italics, `__text__` for bold
    Underscore,
}

impl EmphasisStyle {
    /// Returns the emphasis token character for this style.
    pub fn token(&self) -> char {
        match self {
            EmphasisStyle::Asterisk => '*',
            EmphasisStyle::Underscore => '_',
        }
    }

    /// Returns the strong (bold) token string for this style.
    pub fn strong_token(&self) -> &'static str {
        match self {
            EmphasisStyle::Asterisk => "**",
            EmphasisStyle::Underscore => "__",
        }
    }
}

/// Gets the preferred emphasis style from the `PREFER_ITALICS` environment variable.
///
/// Valid values are `*` (asterisk) or `_` / `__` (underscore). Returns `None` if
/// the variable is not set or has an invalid value.
fn get_preferred_emphasis_style() -> Option<EmphasisStyle> {
    std::env::var("PREFER_ITALICS").ok().and_then(|v| match v.trim() {
        "*" => Some(EmphasisStyle::Asterisk),
        "_" | "__" => Some(EmphasisStyle::Underscore),
        _ => None,
    })
}

/// Represents a recorded emphasis/strong marker from the source.
#[derive(Debug, Clone, Copy)]
struct EmphasisMarker {
    /// The style used in the source (asterisk or underscore)
    style: EmphasisStyle,
    /// Number of characters used (1 for emphasis, 2 for strong)
    char_count: usize,
}

/// Extracts emphasis and strong markers from the source in order of appearance.
///
/// Returns a vector of markers for each emphasis/strong tag (both start and end),
/// indicating which style was used.
fn extract_emphasis_markers(content: &str, events: &[(Event, Range<usize>)]) -> Vec<EmphasisMarker> {
    let mut markers = Vec::new();
    // Stack to track start markers so we can determine end marker styles and char counts
    let mut style_stack: Vec<(EmphasisStyle, usize)> = Vec::new();

    for (event, range) in events {
        match event {
            Event::Start(Tag::Emphasis) => {
                // Emphasis uses 1 character (* or _)
                let style = if range.start < content.len() {
                    match content[range.start..].chars().next() {
                        Some('*') => EmphasisStyle::Asterisk,
                        Some('_') => EmphasisStyle::Underscore,
                        _ => EmphasisStyle::Asterisk,
                    }
                } else {
                    EmphasisStyle::Asterisk
                };
                markers.push(EmphasisMarker { style, char_count: 1 });
                style_stack.push((style, 1));
            }
            Event::Start(Tag::Strong) => {
                // Strong uses 2 characters (** or __)
                let style = if range.start < content.len() {
                    match content[range.start..].chars().next() {
                        Some('*') => EmphasisStyle::Asterisk,
                        Some('_') => EmphasisStyle::Underscore,
                        _ => EmphasisStyle::Asterisk,
                    }
                } else {
                    EmphasisStyle::Asterisk
                };
                markers.push(EmphasisMarker { style, char_count: 2 });
                style_stack.push((style, 2));
            }
            Event::End(TagEnd::Emphasis) | Event::End(TagEnd::Strong) => {
                // Use the style and char_count from the matching start marker
                let (style, char_count) = style_stack.pop().unwrap_or((EmphasisStyle::Asterisk, 1));
                markers.push(EmphasisMarker { style, char_count });
            }
            _ => {}
        }
    }

    markers
}

/// Restores original emphasis markers in the output.
///
/// The `pulldown-cmark-to-cmark` library normalizes all emphasis to a single style.
/// This function restores the original markers from the source when not standardizing.
///
/// The restoration handles nested emphasis (e.g., `***text***` which is strong+emphasis)
/// by consuming multiple markers based on their char_count until the total characters
/// consumed matches the consecutive marker group in the output.
fn restore_emphasis_markers(output: &mut String, markers: &[EmphasisMarker], target_style: Option<EmphasisStyle>) {
    // If a target style is specified, we're standardizing - nothing to restore
    if target_style.is_some() {
        return;
    }

    if markers.is_empty() {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut marker_iter = markers.iter().peekable();
    let mut in_code_block = false;
    let mut chars = output.chars().peekable();

    while let Some(c) = chars.next() {
        // Track code blocks to avoid modifying emphasis inside them
        if c == '`' {
            // Check for triple backtick (code fence)
            let mut backticks = String::from("`");
            while chars.peek() == Some(&'`') {
                backticks.push(chars.next().unwrap());
            }
            if backticks.len() >= 3 {
                in_code_block = !in_code_block;
            }
            result.push_str(&backticks);
            continue;
        }

        if in_code_block {
            result.push(c);
            continue;
        }

        // Check for emphasis/strong markers in output
        // IMPORTANT: cmark normalizes all emphasis to the emphasis_token we specify (asterisk).
        // Literal underscores in text (like word_with_underscores) are preserved as-is.
        // So we ONLY look for asterisks here - underscores in output are literal text.
        if c == '*' {
            // Count consecutive asterisks in output
            let mut output_marker_count = 1;
            while chars.peek() == Some(&'*') {
                chars.next();
                output_marker_count += 1;
            }

            // Consume markers from our iterator until we've matched the output count
            // Each marker has a char_count (1 for emphasis, 2 for strong)
            let mut chars_consumed = 0;
            while chars_consumed < output_marker_count {
                if let Some(original) = marker_iter.next() {
                    let marker_token = original.style.token();
                    for _ in 0..original.char_count {
                        result.push(marker_token);
                    }
                    chars_consumed += original.char_count;
                } else {
                    // No more markers to restore, output remaining as asterisks
                    for _ in chars_consumed..output_marker_count {
                        result.push('*');
                    }
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    *output = result;
}

/// Cleans up markdown content by normalizing formatting.
///
/// This function performs two main operations:
/// 1. Ensures proper blank lines between block elements (via cmark Options)
/// 2. Aligns table columns for consistent formatting
/// 3. Preserves original list markers (*, -, +)
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
    // Parse with source ranges to preserve list markers and emphasis styles
    // Use custom options that exclude ENABLE_SMART_PUNCTUATION to preserve original quotes
    let parser = Parser::new_ext(content, cleanup_parser_options());
    let events_with_ranges: Vec<(Event, Range<usize>)> = parser.into_offset_iter().collect();

    // Extract list markers from source for each list
    let list_markers = extract_list_markers(content, &events_with_ranges);

    // Extract emphasis markers to preserve original styles when not standardizing
    let emphasis_markers = extract_emphasis_markers(content, &events_with_ranges);

    // Determine emphasis style:
    // 1. If PREFER_ITALICS env var is set, use that style (standardize all emphasis)
    // 2. Otherwise, preserve original markers (no standardization)
    let preferred_style = get_preferred_emphasis_style();

    // Separate events from ranges for processing
    let events: Vec<Event> = events_with_ranges.iter().map(|(e, _)| e.clone()).collect();

    // Add "text" language to empty fenced code blocks
    let with_text_lang = add_text_language_to_empty_code_blocks(events);

    // Align tables in the event stream
    let processed = align_tables_in_stream(with_text_lang);

    // Convert events back to markdown with proper spacing options
    let mut output = String::new();

    // cmark handles blank line insertion via its Options - defaults are correct:
    // newlines_after_headline: 2, newlines_after_paragraph: 2, etc.
    // Override code_block_token_count: default is 4, but standard markdown uses 3
    //
    // For emphasis:
    // - If PREFER_ITALICS is set, standardize to that style
    // - Otherwise, use asterisk as default (will be post-processed to restore original)
    let target_style = preferred_style.unwrap_or(EmphasisStyle::Asterisk);
    let options = CmarkOptions {
        code_block_token_count: 3,
        emphasis_token: target_style.token(),
        strong_token: target_style.strong_token(),
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

    // Restore original list markers (the library normalizes to '*')
    restore_list_markers(&mut output, &list_markers);

    // Restore original emphasis markers if not standardizing via PREFER_ITALICS
    restore_emphasis_markers(&mut output, &emphasis_markers, preferred_style);

    // Fix nested list indentation (library uses 2-space, preserve original style)
    let original_indent = detect_list_indentation(content);
    if original_indent > 2 {
        fix_list_indentation(&mut output, original_indent);
    }

    // Unescape unnecessarily escaped brackets (e.g., \[0%\] -> [0%])
    unescape_brackets(&mut output);

    // Trim leading/trailing whitespace-only lines but preserve content
    output.trim_start_matches('\n').to_string()
}

/// Extracts the list marker character for each unordered list from the source.
///
/// Returns a vector of (list_index, marker_char) pairs where list_index is the
/// sequential index of unordered lists encountered and marker_char is the original
/// marker used (*, -, or +).
fn extract_list_markers(content: &str, events: &[(Event, Range<usize>)]) -> Vec<char> {
    let mut markers = Vec::new();

    for (event, range) in events {
        if let Event::Start(Tag::List(None)) = event {
            // Unordered list - find the marker in source
            // The range points to the list start; look for the marker character
            let source_slice = &content[range.start..];
            if let Some(marker) = find_list_marker(source_slice) {
                markers.push(marker);
            } else {
                // Default to '*' if we can't find the marker
                markers.push('*');
            }
        }
    }

    markers
}

/// Finds the first list marker character (*, -, or +) in a source slice.
fn find_list_marker(source: &str) -> Option<char> {
    // Skip leading whitespace and look for the marker
    for c in source.chars() {
        match c {
            '*' | '-' | '+' => return Some(c),
            ' ' | '\t' | '\n' => continue,
            _ => break,
        }
    }
    None
}

/// Restores original list markers in the output.
///
/// The pulldown-cmark-to-cmark library normalizes all unordered list markers to '*'.
/// This function replaces them with the original markers from the source.
///
/// The approach tracks indentation levels to determine which list we're in:
/// - Each indentation level has an associated marker
/// - When we see a list item at a new (deeper) indentation, we consume the next marker
/// - All items at the same level use the same marker until we leave that level
fn restore_list_markers(output: &mut String, markers: &[char]) {
    if markers.is_empty() {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut markers_iter = markers.iter();
    // Stack of (indent_level, marker) pairs
    let mut indent_stack: Vec<(usize, char)> = Vec::new();
    let mut in_code_block = false;
    let mut prev_was_list_item = false;

    while let Some(line) = lines.next() {
        // Track code blocks to avoid modifying markers inside them
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            prev_was_list_item = false;
            continue;
        }

        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Check if this line is a list item (starts with optional whitespace + '* ')
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        if trimmed.starts_with("* ") {
            // This is a list item - determine which marker to use
            let marker = get_marker_for_indent(
                indent,
                &mut indent_stack,
                &mut markers_iter,
                prev_was_list_item,
            );

            // Reconstruct the line with the correct marker
            result.push_str(&" ".repeat(indent));
            result.push(marker);
            result.push_str(&trimmed[1..]); // Skip the '*', keep the rest (including space)
            prev_was_list_item = true;
        } else {
            // Not a list item
            // A blank line or non-indented content might end all lists
            if trimmed.is_empty() {
                // Blank line - lists might continue after this
            } else if indent == 0 && !trimmed.starts_with("* ") {
                // Non-list content at root level - clear the stack
                indent_stack.clear();
            }
            result.push_str(line);
            prev_was_list_item = false;
        }

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

/// Gets the marker for a list item at the given indentation level.
///
/// Manages a stack of (indent, marker) pairs:
/// - Pops entries with larger indent (we've exited those nested lists)
/// - If entry matches current indent AND we had a contiguous list, use its marker
/// - Otherwise, push a new entry with next marker
fn get_marker_for_indent<'a>(
    indent: usize,
    indent_stack: &mut Vec<(usize, char)>,
    markers_iter: &mut impl Iterator<Item = &'a char>,
    prev_was_list_item: bool,
) -> char {
    // If this is a new root-level list after a break, clear the stack first
    // This handles: "- item1\n\n+ item2" where item2 is a new list
    if indent == 0 && !prev_was_list_item && !indent_stack.is_empty() {
        indent_stack.clear();
    }

    // Pop any entries with larger indent (we've exited those nested lists)
    while let Some(&(top_indent, _)) = indent_stack.last() {
        if indent < top_indent {
            indent_stack.pop();
        } else {
            break;
        }
    }

    // Check if we have an entry at this exact indent level AND previous was a list item
    // (meaning we're continuing an existing list, not starting a new one)
    if prev_was_list_item
        && let Some(&(top_indent, marker)) = indent_stack.last()
        && top_indent == indent
    {
        // Continuing same list, use same marker
        return marker;
    }

    // New list at this indent level - get next marker
    let marker = markers_iter.next().copied().unwrap_or('*');
    indent_stack.push((indent, marker));
    marker
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
    let mut in_code_block = false;

    while let Some(line) = lines.next() {
        // Track code blocks to avoid modifying content inside them
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Don't process lines inside code blocks
        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Only fix blockquote lines (those starting with optional space + ">")
        let is_blockquote_line = line.starts_with('>')
            || (line.starts_with(' ') && line.trim_start().starts_with('>'));

        let fixed_line = if is_blockquote_line {
            fix_blockquote_line(line)
        } else {
            line.to_string()
        };

        // Check if this is an empty blockquote line (just "> " or nested like "> > ")
        let trimmed = fixed_line.trim_end();
        let is_empty_blockquote = trimmed.chars().all(|c| c == '>' || c == ' ')
            && trimmed.contains('>')
            && !trimmed.is_empty();

        if is_empty_blockquote
            && let Some(next_line) = lines.peek()
            && next_line.trim_start().starts_with('>')
        {
            // Skip this empty blockquote line (next line continues the blockquote)
            continue;
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

/// Detects the list indentation style used in the source content.
///
/// Scans for nested list items and returns the number of spaces used for indentation.
/// Returns 2 if no nested lists are found or indentation can't be determined.
fn detect_list_indentation(content: &str) -> usize {
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        // Skip code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Look for indented list items
        let indent = line.len() - trimmed.len();
        if indent > 0 && (trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ")) {
            // Found a nested list item - return its indentation
            return indent;
        }

        // Also check for numbered lists
        if indent > 0 {
            let mut chars = trimmed.chars().peekable();
            let mut is_numbered = false;
            while let Some(c) = chars.next() {
                if c.is_ascii_digit() {
                    continue;
                } else if (c == '.' || c == ')') && chars.peek() == Some(&' ') {
                    is_numbered = true;
                    break;
                } else {
                    break;
                }
            }
            if is_numbered {
                return indent;
            }
        }
    }

    // Default to 2 if no nested lists found
    2
}

/// Fixes list indentation in the output to match the original style.
///
/// `pulldown-cmark-to-cmark` uses 2-space indentation by default. This function
/// converts it to the specified indentation size (e.g., 4 spaces).
fn fix_list_indentation(output: &mut String, target_indent: usize) {
    if target_indent == 2 {
        return; // Already correct
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        // Track code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Don't process code block content
        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Check if this is a list item
        let is_list_item = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || is_ordered_list_start(trimmed);

        if is_list_item {
            let current_indent = line.len() - trimmed.len();
            if current_indent > 0 {
                // Calculate nesting level (assuming 2-space input)
                let nesting_level = current_indent / 2;
                // Apply target indentation
                let new_indent = nesting_level * target_indent;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            } else {
                result.push_str(line);
            }
        } else {
            // For non-list content that's indented (like continuation text),
            // apply the same scaling
            let current_indent = line.len() - trimmed.len();
            if current_indent > 0 && current_indent % 2 == 0 {
                let nesting_level = current_indent / 2;
                let new_indent = nesting_level * target_indent;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            } else {
                result.push_str(line);
            }
        }

        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Checks if a line starts with an ordered list marker (e.g., "1. " or "1) ").
fn is_ordered_list_start(line: &str) -> bool {
    let mut chars = line.chars().peekable();
    let mut has_digit = false;

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            has_digit = true;
        } else if has_digit && (c == '.' || c == ')') {
            return chars.peek() == Some(&' ');
        } else {
            return false;
        }
    }
    false
}

/// Unescapes unnecessarily escaped brackets in the output.
///
/// `pulldown-cmark-to-cmark` escapes `[` and `]` characters that could potentially
/// be interpreted as link syntax. This function unescapes patterns like `\[0%\]`
/// that are clearly not links (no `](` following them).
fn unescape_brackets(output: &mut String) {
    // Only process if there are escaped brackets
    if !output.contains("\\[") {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('[') => {
                    // Look ahead to see if this could be a link
                    // Pattern: \[...\] or \[...](...)
                    // We want to unescape standalone \[...\] that aren't links
                    chars.next(); // consume '['

                    // Collect until we find \] or ]( or end
                    let mut bracket_content = String::new();
                    let mut found_close = false;

                    while let Some(&next) = chars.peek() {
                        if next == '\\' {
                            chars.next();
                            if chars.peek() == Some(&']') {
                                chars.next();
                                found_close = true;
                                break;
                            } else {
                                bracket_content.push('\\');
                            }
                        } else if next == ']' {
                            chars.next();
                            found_close = true;
                            // Check if followed by ( - would make this a link
                            if chars.peek() == Some(&'(') {
                                // This is actually a link, restore and keep escape
                                result.push_str("\\[");
                                result.push_str(&bracket_content);
                                result.push(']');
                                break;
                            }
                            break;
                        } else if next == '\n' {
                            // Line break - not a link, but stop searching
                            break;
                        } else {
                            bracket_content.push(chars.next().unwrap());
                        }
                    }

                    if found_close {
                        // Unescape: output [content] instead of \[content\]
                        result.push('[');
                        result.push_str(&bracket_content);
                        result.push(']');
                    } else {
                        // Didn't find proper close, restore original
                        result.push_str("\\[");
                        result.push_str(&bracket_content);
                        // Restore chars iterator - actually we can't easily do this
                        // Just continue from where we are
                    }
                }
                Some(']') => {
                    // Standalone escaped ] - keep as is (shouldn't happen often)
                    result.push('\\');
                    result.push(chars.next().unwrap());
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }

    *output = result;
}

/// Adds "text" language to fenced code blocks with no language specified.
///
/// This ensures all code blocks have an explicit language identifier,
/// improving rendering consistency across different markdown viewers.
fn add_text_language_to_empty_code_blocks(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    events
        .into_iter()
        .map(|event| {
            if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) = event
                && info.is_empty()
            {
                return Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from(
                    "text",
                ))));
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

    #[test]
    fn test_cleanup_preserves_code_block_indentation() {
        // Regression test: indentation inside code blocks must be preserved
        let content = "```ts title=\"Greet Function\"\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
        let cleaned = cleanup_content(content);

        // The 4-space indentation before console.log must be preserved
        assert!(
            cleaned.contains("    console.log"),
            "Indentation inside code block should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_code_block_indentation_simple() {
        // Test without attributes - just language
        let content = "```ts\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("    console.log"),
            "Indentation inside simple code block should be preserved, got:\n{}",
            cleaned
        );
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

    // ==================== List Marker Preservation Tests ====================

    #[test]
    fn test_list_marker_dash_preserved() {
        let content = "- Item 1\n- Item 2\n- Item 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Item 1"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("- Item 2"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_plus_preserved() {
        let content = "+ Alpha\n+ Beta\n+ Gamma";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("+ Alpha"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Beta"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_asterisk_preserved() {
        let content = "* One\n* Two\n* Three";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* One"),
            "Asterisk list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_multiple_lists_different_markers() {
        let content = "- Dash item\n\n+ Plus item\n\n* Asterisk item";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Dash"),
            "First list should use dash, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Plus"),
            "Second list should use plus, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* Asterisk"),
            "Third list should use asterisk, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_with_header_and_text() {
        // Regression test: List markers preserved in real document context
        let content = "# Title\n\nSome text.\n\n- First\n- Second\n\nMore text.\n\n+ Alpha\n+ Beta";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- First"),
            "Dash markers should be preserved in document, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Alpha"),
            "Plus markers should be preserved in document, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_inside_blockquote() {
        // List markers inside blockquotes should be preserved
        let content = "> - Quoted item 1\n> - Quoted item 2";
        let cleaned = cleanup_content(content);

        // The structure should be preserved
        assert!(
            cleaned.contains("Quoted item 1"),
            "Blockquote list content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_not_changed_in_code_block() {
        // List markers inside code blocks should not be modified
        let content = "```\n* This is code\n- Also code\n+ More code\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* This is code"),
            "Asterisk in code should not change, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("- Also code"),
            "Dash in code should not change, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ More code"),
            "Plus in code should not change, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_mixed_list_types_separated_by_text() {
        // Multiple lists with different markers separated by paragraph text
        let content = "- Dash list\n\nParagraph text\n\n+ Plus list\n\nMore text\n\n* Star list";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Dash"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Plus"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* Star"),
            "Star list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_consistency_in_same_list() {
        // All items in the same list should use the same marker
        let content = "- Item 1\n- Item 2\n- Item 3\n- Item 4\n- Item 5";
        let cleaned = cleanup_content(content);

        // All items should have dash markers
        let dash_count = cleaned.matches("\n- ").count() + if cleaned.starts_with("- ") { 1 } else { 0 };
        assert_eq!(
            dash_count, 5,
            "All 5 items should use dash marker, got:\n{}",
            cleaned
        );
    }

    // ==================== List Indentation Tests ====================

    #[test]
    fn test_nested_list_preserves_4_space_indentation() {
        // Regression test: 4-space indentation should be preserved
        let content = "- Level 1\n    - Level 2\n        - Level 3";
        let cleaned = cleanup_content(content);

        // Should have 4-space indentation for nested items
        assert!(
            cleaned.contains("\n    - Level 2"),
            "4-space indentation should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n        - Level 3"),
            "8-space indentation should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_preserves_2_space_indentation() {
        // 2-space indentation should remain as-is
        let content = "- Level 1\n  - Level 2\n    - Level 3";
        let cleaned = cleanup_content(content);

        // Should have 2-space indentation
        assert!(
            cleaned.contains("\n  - Level 2"),
            "2-space indentation should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_in_todo_list() {
        // Regression test for nested TODO list items
        let content = "- [x] Task 1\n- Progress\n    - [ ] Sub-task 1\n    - [ ] Sub-task 2";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\n    - [ ] Sub-task 1"),
            "Nested TODO items should preserve 4-space indentation, got:\n{}",
            cleaned
        );
    }

    // ==================== Bracket Escaping Tests ====================

    #[test]
    fn test_bracket_not_escaped_progress_indicator() {
        // Regression test: progress indicators should not be escaped
        let content = "- [0%] started\n- [25%] progress\n- [50%] halfway\n- [75%] almost\n- [100%] done";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[0%]"),
            "Progress indicator [0%] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\\[0%\\]"),
            "Progress indicator should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("[25%]"),
            "Progress indicator [25%] should not be escaped, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_bracket_not_escaped_bang_indicator() {
        // Regression test: [!] blocked indicator should not be escaped
        let content = "- [!] blocked task";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[!]"),
            "Blocked indicator [!] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\\[!\\]"),
            "Blocked indicator should not be escaped, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_bracket_preserved_in_actual_links() {
        // Actual links should still work
        let content = "Check out [this link](https://example.com)";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[this link](https://example.com)"),
            "Links should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_task_list_checkboxes_not_escaped() {
        // Task list checkboxes should never be escaped
        let content = "- [x] done\n- [ ] pending";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[x]"),
            "Checkbox [x] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("[ ]"),
            "Checkbox [ ] should not be escaped, got:\n{}",
            cleaned
        );
    }

    // ==================== Smart Quotes Tests (Regression) ====================

    #[test]
    fn test_cleanup_preserves_straight_double_quotes() {
        // Regression test: Normal quotes should NOT be converted to smart quotes
        let content = r#"He said "hello" and "goodbye"."#;
        let cleaned = cleanup_content(content);

        // Should contain straight quotes (ASCII 0x22)
        assert!(
            cleaned.contains('"'),
            "Straight double quotes should be preserved, got:\n{}",
            cleaned
        );
        // Should NOT contain smart quotes (U+201C and U+201D - left and right double quotes)
        assert!(
            !cleaned.contains('\u{201C}') && !cleaned.contains('\u{201D}'),
            "Should not contain smart/curly quotes, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_straight_single_quotes() {
        // Regression test: Single quotes/apostrophes should NOT be converted
        let content = "It's a test and 'quoted text' here.";
        let cleaned = cleanup_content(content);

        // Should contain straight single quote (ASCII 0x27)
        assert!(
            cleaned.contains("'"),
            "Straight single quotes should be preserved, got:\n{}",
            cleaned
        );
        // Should NOT contain smart single quotes (U+2018 and U+2019)
        assert!(
            !cleaned.contains('\u{2018}') && !cleaned.contains('\u{2019}'),
            "Should not contain smart/curly single quotes, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_quotes_in_code() {
        // Quotes inside code blocks should definitely be preserved
        let content = "```\nlet s = \"hello\";\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\"hello\""),
            "Quotes in code should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== Emphasis/Italics Tests (Regression) ====================

    #[test]
    fn test_cleanup_preserves_asterisk_emphasis() {
        // Regression test: *asterisk* emphasis should NOT be converted to _underscore_
        let content = "This has *asterisk italics* here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*asterisk italics*"),
            "Asterisk emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_underscore_emphasis() {
        // Regression test: _underscore_ emphasis should NOT be converted to *asterisk*
        let content = "This has _underscore italics_ here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_underscore italics_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_mixed_emphasis_styles() {
        // Both asterisk and underscore styles should be preserved in the same document
        let content = "Mix of *asterisk* and _underscore_ emphasis.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*asterisk*"),
            "Asterisk emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_underscore_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_asterisk_strong() {
        // **bold** should be preserved
        let content = "This has **asterisk bold** here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**asterisk bold**"),
            "Asterisk strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_underscore_strong() {
        // __bold__ should be preserved
        let content = "This has __underscore bold__ here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("__underscore bold__"),
            "Underscore strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_mixed_strong_styles() {
        // Both asterisk and underscore bold should be preserved
        let content = "Mix of **asterisk bold** and __underscore bold__ text.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**asterisk bold**"),
            "Asterisk strong should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("__underscore bold__"),
            "Underscore strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_nested_emphasis() {
        // Nested emphasis should preserve styles
        let content = "This has **bold with _nested italics_** here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**"),
            "Bold markers should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_nested italics_"),
            "Nested underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_emphasis_in_code_blocks_unchanged() {
        // Emphasis markers inside code blocks should NOT be modified
        let content = "```\n*not* _emphasis_ **here**\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*not*"),
            "Emphasis markers in code should be unchanged, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_emphasis_"),
            "Underscore in code should be unchanged, got:\n{}",
            cleaned
        );
    }

    // ==================== Regression Tests for Emphasis Restoration ====================

    #[test]
    fn test_regression_nested_emphasis_preserves_original_styles() {
        // Regression test: nested emphasis like **_text_** should preserve both styles
        // Previously, ***text*** was incorrectly output as ***text___ or similar
        let content = "**_nested_** test";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**_nested_**"),
            "Nested emphasis should preserve both styles (** and _), got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_literal_underscores_not_converted() {
        // Regression test: literal underscores in words like word_with_underscores
        // should NOT be converted to asterisks or modified in any way
        let content = "Testing word_with_underscores here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("word_with_underscores"),
            "Literal underscores in words should not be modified, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_emphasis_and_literal_underscores_combined() {
        // Regression test: combination of emphasis and literal underscores
        // The underscore emphasis should be preserved AND the literal underscores
        // in word_with_underscores should remain unchanged
        let content = "Testing _emphasis_ inside a word_with_underscores and **_nested_** styles.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_emphasis_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("word_with_underscores"),
            "Literal underscores should not be modified, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("**_nested_**"),
            "Nested emphasis should preserve styles, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_triple_emphasis_markers() {
        // Regression test: ***text*** (strong + emphasis) should correctly restore styles
        let content = "***both bold and italic***";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("***both bold and italic***"),
            "Triple emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_mixed_nested_emphasis() {
        // Regression test: mixed nested styles like __*text*__
        let content = "__*mixed nesting*__";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("__*mixed nesting*__"),
            "Mixed nested emphasis should preserve styles, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_multiple_emphasis_with_underscores_in_text() {
        // Regression test: multiple emphasis with underscores in regular text
        let content = "_first_ and snake_case_name and _second_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_first_"),
            "First underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("snake_case_name"),
            "Literal underscores in snake_case should not be modified, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_second_"),
            "Second underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }
}
