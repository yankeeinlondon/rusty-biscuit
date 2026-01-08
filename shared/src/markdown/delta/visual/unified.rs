//! Unified diff renderer for narrow terminals (<=110 columns).
//!
//! Layout:
//! ```text
//!   10 11 │ context line
//! - 12    │ removed line
//! +    13 │ added line
//! ```

use super::diff::{DiffLine, InlineSpan};
use super::VisualDiffOptions;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
                output.push_str(&format_context_line(
                    *line_no_old,
                    *line_no_new,
                    content,
                    content_width,
                ));
            }
            DiffLine::Removed {
                line_no,
                content,
                inline_changes,
            } => {
                output.push_str(&format_removed_line(
                    *line_no,
                    content,
                    inline_changes,
                    content_width,
                ));
            }
            DiffLine::Added {
                line_no,
                content,
                inline_changes,
            } => {
                output.push_str(&format_added_line(
                    *line_no,
                    content,
                    inline_changes,
                    content_width,
                ));
            }
        }
        output.push('\n');
    }

    output
}

/// Filter lines to show only changes and surrounding context.
fn filter_with_context(diff: &[DiffLine], context_lines: usize) -> std::collections::HashSet<usize> {
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

/// Format a context line.
fn format_context_line(
    line_no_old: usize,
    line_no_new: usize,
    content: &str,
    max_width: usize,
) -> String {
    let truncated = truncate_to_width(content, max_width);
    format!(
        "  {DIM}{:>4} {:>4}{RESET} {DIM}│{RESET} {}",
        line_no_old, line_no_new, truncated
    )
}

/// Format a removed line.
fn format_removed_line(
    line_no: usize,
    content: &str,
    inline_changes: &[InlineSpan],
    max_width: usize,
) -> String {
    let formatted_content = format_with_inline_changes(content, inline_changes, max_width, true);
    format!(
        "{FG_RED}-{RESET} {BG_REMOVED}{:>4}{RESET}      {DIM}│{RESET} {}",
        line_no, formatted_content
    )
}

/// Format an added line.
fn format_added_line(
    line_no: usize,
    content: &str,
    inline_changes: &[InlineSpan],
    max_width: usize,
) -> String {
    let formatted_content = format_with_inline_changes(content, inline_changes, max_width, false);
    format!(
        "{FG_GREEN}+{RESET}      {BG_ADDED}{:>4}{RESET} {DIM}│{RESET} {}",
        line_no, formatted_content
    )
}

/// Format content with inline change highlighting.
fn format_with_inline_changes(
    content: &str,
    spans: &[InlineSpan],
    max_width: usize,
    is_removed: bool,
) -> String {
    if spans.is_empty() {
        return truncate_to_width(content, max_width);
    }

    let bg_emphasis = if is_removed {
        BG_CHANGED_DEL
    } else {
        BG_CHANGED_ADD
    };

    let mut result = String::new();
    let mut visual_width = 0;

    for span in spans {
        if visual_width >= max_width {
            break;
        }

        let span_content = if span.end <= content.len() {
            &content[span.start..span.end]
        } else if span.start < content.len() {
            &content[span.start..]
        } else {
            continue;
        };

        let remaining_width = max_width.saturating_sub(visual_width);
        let truncated_span = truncate_to_width(span_content, remaining_width);
        let span_visual_width = truncated_span.width();

        if span.emphasized {
            result.push_str(&format!(
                "{}{BOLD}{UNDERLINE}{}{RESET}",
                bg_emphasis, truncated_span
            ));
        } else {
            result.push_str(&truncated_span);
        }

        visual_width += span_visual_width;
    }

    result
}

/// Truncate a string to fit within a visual width.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    // First check if the string fits entirely
    if s.width() <= max_width {
        return s.to_string();
    }

    // Need to truncate - reserve space for ellipsis
    let ellipsis_width = 1;
    let target_width = max_width.saturating_sub(ellipsis_width);

    let mut result = String::new();
    let mut current_width = 0;

    for ch in s.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + char_width > target_width {
            break;
        }
        result.push(ch);
        current_width += char_width;
    }

    if max_width > 0 {
        result.push('…');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_context_line() {
        let line = format_context_line(10, 11, "Hello World", 50);
        assert!(line.contains("10"));
        assert!(line.contains("11"));
        assert!(line.contains("Hello World"));
        assert!(line.contains("│"));
    }

    #[test]
    fn test_format_removed_line() {
        let spans = vec![InlineSpan {
            start: 0,
            end: 5,
            emphasized: true,
        }];
        let line = format_removed_line(10, "Hello", &spans, 50);
        assert!(line.contains("-"));
        assert!(line.contains("10"));
    }

    #[test]
    fn test_format_added_line() {
        let spans = vec![InlineSpan {
            start: 0,
            end: 5,
            emphasized: true,
        }];
        let line = format_added_line(10, "Hello", &spans, 50);
        assert!(line.contains("+"));
        assert!(line.contains("10"));
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
}
