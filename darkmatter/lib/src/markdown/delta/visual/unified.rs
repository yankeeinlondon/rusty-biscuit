//! Unified diff renderer for narrow terminals (<=110 columns).
//!
//! Layout:
//! ```text
//!   10 11 │ context line
//! - 12    │ removed line
//! +    13 │ added line
//! ```
//!
//! Long lines are word-wrapped to fit within the available content width,
//! with continuation lines showing empty line numbers and maintaining
//! the appropriate styling.

use super::VisualDiffOptions;
use super::diff::{DiffLine, InlineSpan};
use textwrap::{Options as WrapOptions, wrap};
use unicode_width::UnicodeWidthStr;

// ANSI escape codes
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const UNDERLINE: &str = "\x1b[4m";

// Foreground colors
const FG_RED: &str = "\x1b[31m";
const FG_GREEN: &str = "\x1b[32m";

// Background colors (256-color mode)
const BG_REMOVED: &str = "\x1b[48;5;52m";
const BG_ADDED: &str = "\x1b[48;5;22m";
const BG_CHANGED_DEL: &str = "\x1b[48;5;88m";
const BG_CHANGED_ADD: &str = "\x1b[48;5;28m";

/// Render a unified diff.
pub fn render(
    diff: &[DiffLine],
    label_original: &str,
    label_updated: &str,
    options: &VisualDiffOptions,
) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "{DIM}─── {RESET}{}{DIM} → {RESET}{}{DIM} ───{RESET}\n",
        label_original, label_updated
    ));

    // Calculate content width
    // Layout: [prefix:1] [space] [old_num:4] [space] [new_num:4] [space] [│] [space] [content]
    // Fixed elements: 1 + 1 + 4 + 1 + 4 + 1 + 1 + 1 = 14 chars
    let content_width = if options.terminal_width > 14 {
        options.terminal_width as usize - 14
    } else {
        60 // Minimum fallback
    };

    // Apply context filtering
    let visible_lines = filter_with_context(diff, options.context_lines);

    let mut prev_was_separator = true; // Start true to avoid leading separator
    for (idx, line) in diff.iter().enumerate() {
        if !visible_lines.contains(&idx) {
            // Check if we need a separator
            if !prev_was_separator && idx > 0 {
                // Add hunk separator
                output.push_str(&format!("{DIM}  ···{RESET}\n"));
                prev_was_separator = true;
            }
            continue;
        }
        prev_was_separator = false;

        match line {
            DiffLine::Context {
                line_no_old,
                line_no_new,
                content,
            } => {
                let lines = format_context_line(*line_no_old, *line_no_new, content, content_width);
                for formatted_line in lines {
                    output.push_str(&formatted_line);
                    output.push('\n');
                }
            }
            DiffLine::Removed {
                line_no,
                content,
                inline_changes,
            } => {
                let lines = format_removed_line(*line_no, content, inline_changes, content_width);
                for formatted_line in lines {
                    output.push_str(&formatted_line);
                    output.push('\n');
                }
            }
            DiffLine::Added {
                line_no,
                content,
                inline_changes,
            } => {
                let lines = format_added_line(*line_no, content, inline_changes, content_width);
                for formatted_line in lines {
                    output.push_str(&formatted_line);
                    output.push('\n');
                }
            }
        }
    }

    output
}

/// Filter lines to show only changes and surrounding context.
fn filter_with_context(
    diff: &[DiffLine],
    context_lines: usize,
) -> std::collections::HashSet<usize> {
    use std::collections::HashSet;

    let mut visible = HashSet::new();

    // First pass: mark all change lines
    let change_indices: Vec<usize> = diff
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.is_context())
        .map(|(idx, _)| idx)
        .collect();

    // Second pass: add context around each change
    for &change_idx in &change_indices {
        // Add lines before
        let start = change_idx.saturating_sub(context_lines);
        for i in start..=change_idx {
            visible.insert(i);
        }

        // Add lines after
        let end = (change_idx + context_lines + 1).min(diff.len());
        for i in change_idx..end {
            visible.insert(i);
        }
    }

    visible
}

/// Format a context line, returning multiple lines if wrapping is needed.
fn format_context_line(
    line_no_old: usize,
    line_no_new: usize,
    content: &str,
    max_width: usize,
) -> Vec<String> {
    let wrapped = wrap_to_width(content, max_width);
    let mut lines = Vec::with_capacity(wrapped.len());

    for (idx, line_content) in wrapped.iter().enumerate() {
        if idx == 0 {
            // First line shows line numbers
            lines.push(format!(
                "  {DIM}{:>4} {:>4}{RESET} {DIM}│{RESET} {}",
                line_no_old, line_no_new, line_content
            ));
        } else {
            // Continuation lines have empty line number area
            lines.push(format!(
                "  {DIM}          {RESET} {DIM}│{RESET} {}",
                line_content
            ));
        }
    }

    lines
}

/// Format a removed line, returning multiple lines if wrapping is needed.
fn format_removed_line(
    line_no: usize,
    content: &str,
    inline_changes: &[InlineSpan],
    max_width: usize,
) -> Vec<String> {
    let wrapped = wrap_to_width(content, max_width);
    let mut lines = Vec::with_capacity(wrapped.len());

    for (idx, line_content) in wrapped.iter().enumerate() {
        if idx == 0 {
            // First line shows line number and inline changes
            let formatted_content =
                format_with_inline_changes(line_content, inline_changes, max_width, true);
            lines.push(format!(
                "{FG_RED}-{RESET} {BG_REMOVED}{:>4}{RESET}      {DIM}│{RESET} {}",
                line_no, formatted_content
            ));
        } else {
            // Continuation lines
            lines.push(format!(
                "{FG_RED} {RESET} {BG_REMOVED}    {RESET}      {DIM}│{RESET} {}",
                line_content
            ));
        }
    }

    lines
}

/// Format an added line, returning multiple lines if wrapping is needed.
fn format_added_line(
    line_no: usize,
    content: &str,
    inline_changes: &[InlineSpan],
    max_width: usize,
) -> Vec<String> {
    let wrapped = wrap_to_width(content, max_width);
    let mut lines = Vec::with_capacity(wrapped.len());

    for (idx, line_content) in wrapped.iter().enumerate() {
        if idx == 0 {
            // First line shows line number and inline changes
            let formatted_content =
                format_with_inline_changes(line_content, inline_changes, max_width, false);
            lines.push(format!(
                "{FG_GREEN}+{RESET}      {BG_ADDED}{:>4}{RESET} {DIM}│{RESET} {}",
                line_no, formatted_content
            ));
        } else {
            // Continuation lines
            lines.push(format!(
                "{FG_GREEN} {RESET}      {BG_ADDED}    {RESET} {DIM}│{RESET} {}",
                line_content
            ));
        }
    }

    lines
}

/// Format content with inline change highlighting.
///
/// This function formats a single wrapped line with inline change spans.
/// Spans are byte offsets in the original content, so we map them to the
/// current line segment.
fn format_with_inline_changes(
    content: &str,
    spans: &[InlineSpan],
    max_width: usize,
    is_removed: bool,
) -> String {
    if spans.is_empty() {
        // No spans - just return the content (already wrapped by caller)
        return content.to_string();
    }

    let bg_emphasis = if is_removed {
        BG_CHANGED_DEL
    } else {
        BG_CHANGED_ADD
    };

    let content_len = content.len();
    let mut result = String::new();
    let mut visual_width = 0;
    let mut byte_pos = 0;

    // Process spans that fall within this line segment
    for span in spans {
        if visual_width >= max_width || byte_pos >= content_len {
            break;
        }

        // Calculate how much of this span falls within this line
        let span_start_in_line = span.start.max(byte_pos);
        let span_end_in_line = span.end.min(content_len);

        if span_start_in_line >= span_end_in_line {
            continue;
        }

        // Add any content before this span (gap)
        if span_start_in_line > byte_pos && span_start_in_line <= content_len {
            let gap_content = &content[byte_pos..span_start_in_line];
            result.push_str(gap_content);
            visual_width += gap_content.width();
        }

        // Add the span content
        if span_start_in_line < content_len {
            let span_content = &content[span_start_in_line..span_end_in_line];

            if span.emphasized {
                result.push_str(&format!(
                    "{}{BOLD}{UNDERLINE}{}{RESET}",
                    bg_emphasis, span_content
                ));
            } else {
                result.push_str(span_content);
            }
            visual_width += span_content.width();
        }

        byte_pos = span_end_in_line;
    }

    // Add any remaining content after all spans
    if byte_pos < content_len {
        let remaining = &content[byte_pos..];
        result.push_str(remaining);
    }

    result
}

/// Wrap a string to fit within a visual width, returning multiple lines if needed.
///
/// Uses textwrap for intelligent word breaking. Each returned line is guaranteed
/// to fit within `max_width` display columns.
fn wrap_to_width(s: &str, max_width: usize) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }

    // Handle edge case of very narrow width
    if max_width == 0 {
        return vec![String::new()];
    }

    // Configure textwrap options for proper word wrapping
    let options = WrapOptions::new(max_width).break_words(true); // Break long words if needed

    let wrapped: Vec<String> = wrap(s, options)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect();

    // Ensure we always return at least one line
    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_to_width_short() {
        let result = wrap_to_width("Hello", 10);
        assert_eq!(result, vec!["Hello"]);
    }

    #[test]
    fn test_wrap_to_width_long() {
        let result = wrap_to_width("Hello World", 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "Hello");
        assert_eq!(result[1], "World");
    }

    #[test]
    fn test_format_context_line_short() {
        let lines = format_context_line(10, 11, "Hello World", 50);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("10"));
        assert!(lines[0].contains("11"));
        assert!(lines[0].contains("Hello World"));
        assert!(lines[0].contains("│"));
    }

    #[test]
    fn test_format_context_line_wraps() {
        // Regression test: long lines should wrap instead of truncate
        let long_content = "This is a very long context line that should wrap";
        let lines = format_context_line(10, 11, long_content, 15);

        // Should produce multiple visual lines
        assert!(
            lines.len() > 1,
            "Long content should wrap to multiple lines"
        );

        // First line should show line numbers
        assert!(lines[0].contains("10"));
        assert!(lines[0].contains("11"));

        // All lines should contain the separator
        for line in &lines {
            assert!(line.contains("│"));
        }
    }

    #[test]
    fn test_format_removed_line_short() {
        let spans = vec![InlineSpan {
            start: 0,
            end: 5,
            emphasized: true,
        }];
        let lines = format_removed_line(10, "Hello", &spans, 50);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("-"));
        assert!(lines[0].contains("10"));
    }

    #[test]
    fn test_format_removed_line_wraps() {
        // Regression test: long removed lines should wrap
        let long_content = "This is a very long removed line that should wrap properly";
        let lines = format_removed_line(42, long_content, &[], 20);

        assert!(lines.len() > 1, "Long removed line should wrap");
        assert!(lines[0].contains("42"));
        assert!(lines[0].contains("-"));
    }

    #[test]
    fn test_format_added_line_short() {
        let spans = vec![InlineSpan {
            start: 0,
            end: 5,
            emphasized: true,
        }];
        let lines = format_added_line(10, "Hello", &spans, 50);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("+"));
        assert!(lines[0].contains("10"));
    }

    #[test]
    fn test_format_added_line_wraps() {
        // Regression test: long added lines should wrap
        let long_content = "This is a very long added line that should wrap properly";
        let lines = format_added_line(42, long_content, &[], 20);

        assert!(lines.len() > 1, "Long added line should wrap");
        assert!(lines[0].contains("42"));
        assert!(lines[0].contains("+"));
    }

    #[test]
    fn test_context_filtering() {
        let diff = vec![
            DiffLine::Context {
                line_no_old: 1,
                line_no_new: 1,
                content: "Line 1".to_string(),
            },
            DiffLine::Context {
                line_no_old: 2,
                line_no_new: 2,
                content: "Line 2".to_string(),
            },
            DiffLine::Context {
                line_no_old: 3,
                line_no_new: 3,
                content: "Line 3".to_string(),
            },
            DiffLine::Removed {
                line_no: 4,
                content: "Old".to_string(),
                inline_changes: vec![],
            },
            DiffLine::Added {
                line_no: 4,
                content: "New".to_string(),
                inline_changes: vec![],
            },
            DiffLine::Context {
                line_no_old: 5,
                line_no_new: 5,
                content: "Line 5".to_string(),
            },
        ];

        // With context_lines = 1, should show lines 2,3,4(rem),4(add),5
        let visible = filter_with_context(&diff, 1);
        assert!(visible.contains(&2)); // 1 line before change
        assert!(visible.contains(&3)); // Change
        assert!(visible.contains(&4)); // Change
        assert!(visible.contains(&5)); // 1 line after change
    }

    #[test]
    fn test_render_empty_diff() {
        let diff: Vec<DiffLine> = vec![];
        let options = VisualDiffOptions::with_width(80);
        let output = render(&diff, "a.md", "b.md", &options);
        // Should have header
        assert!(output.contains("a.md"));
        assert!(output.contains("b.md"));
    }

    #[test]
    fn test_render_long_lines_wrap() {
        // Regression test: ensure long lines in full render wrap properly
        let diff = vec![
            DiffLine::Context {
                line_no_old: 1,
                line_no_new: 1,
                content: "Short line".to_string(),
            },
            DiffLine::Removed {
                line_no: 2,
                content: "This is a very long removed line that exceeds the available width and must wrap to multiple lines for proper display".to_string(),
                inline_changes: vec![],
            },
            DiffLine::Added {
                line_no: 2,
                content: "This is another very long added line that also exceeds the width and needs wrapping".to_string(),
                inline_changes: vec![],
            },
        ];

        // Use a narrow width to force wrapping (unified mode is for narrow terminals)
        let options = VisualDiffOptions::with_width(60);
        let output = render(&diff, "old.md", "new.md", &options);

        // Count newlines - should be more than just 4 (header + 3 diff lines)
        // because long lines wrap
        let newline_count = output.chars().filter(|c| *c == '\n').count();
        assert!(
            newline_count > 4,
            "Long lines should wrap, producing more output lines. Got {} lines",
            newline_count
        );
    }
}
