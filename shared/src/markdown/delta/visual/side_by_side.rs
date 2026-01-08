//! Side-by-side diff renderer for wide terminals (>110 columns).
//!
//! Layout:
//! ```text
//! [left_num:4] [left_content] │ [right_num:4] [right_content]
//! ```

use super::diff::{DiffLine, InlineSpan};
use super::VisualDiffOptions;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// ANSI escape codes
const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const UNDERLINE: &str = "\x1b[4m";

// Background colors (256-color mode)
const BG_REMOVED: &str = "\x1b[48;5;52m"; // Dark red
const BG_ADDED: &str = "\x1b[48;5;22m"; // Dark green
const BG_CHANGED_DEL: &str = "\x1b[48;5;88m"; // Brighter red for inline changes
const BG_CHANGED_ADD: &str = "\x1b[48;5;28m"; // Brighter green for inline changes

/// Render a side-by-side diff.
pub fn render(
    diff: &[DiffLine],
    label_original: &str,
    label_updated: &str,
    options: &VisualDiffOptions,
) -> String {
    let mut output = String::new();

    // Calculate column widths
    // Layout: [num:4] [space] [content] [space] [│] [space] [num:4] [space] [content]
    // Fixed elements: 4 + 1 + 1 + 3 + 4 + 1 = 14 chars
    let content_width = if options.terminal_width > 14 {
        (options.terminal_width as usize - 14) / 2
    } else {
        30 // Minimum fallback
    };

    // Header
    output.push_str(&format_header(
        label_original,
        label_updated,
        content_width,
        options,
    ));
    output.push('\n');

    // Track pairing of removed/added lines for side-by-side display
    let mut i = 0;
    while i < diff.len() {
        match &diff[i] {
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
                output.push('\n');
                i += 1;
            }
            DiffLine::Removed { .. } => {
                // Collect consecutive removed lines
                let mut removed_lines = vec![];
                while i < diff.len() && diff[i].is_removed() {
                    removed_lines.push(&diff[i]);
                    i += 1;
                }

                // Collect consecutive added lines
                let mut added_lines = vec![];
                while i < diff.len() && diff[i].is_added() {
                    added_lines.push(&diff[i]);
                    i += 1;
                }

                // Pair them up
                let max_lines = removed_lines.len().max(added_lines.len());
                for j in 0..max_lines {
                    let left = removed_lines.get(j).copied();
                    let right = added_lines.get(j).copied();
                    output.push_str(&format_paired_line(left, right, content_width));
                    output.push('\n');
                }
            }
            DiffLine::Added { .. } => {
                // Standalone added line (no preceding removed)
                output.push_str(&format_paired_line(None, Some(&diff[i]), content_width));
                output.push('\n');
                i += 1;
            }
        }
    }

    output
}

/// Format the header line with labels.
fn format_header(
    label_left: &str,
    label_right: &str,
    content_width: usize,
    _options: &VisualDiffOptions,
) -> String {
    let left_label = truncate_to_width(label_left, content_width);
    let right_label = truncate_to_width(label_right, content_width);

    let left_padding = content_width.saturating_sub(left_label.width());
    let right_padding = content_width.saturating_sub(right_label.width());

    format!(
        "{DIM}     {BOLD}{}{RESET}{DIM}{} │      {BOLD}{}{RESET}{DIM}{}",
        left_label,
        " ".repeat(left_padding),
        right_label,
        " ".repeat(right_padding),
    )
}

/// Format a context line (unchanged, shown on both sides).
fn format_context_line(
    line_no_old: usize,
    line_no_new: usize,
    content: &str,
    content_width: usize,
) -> String {
    let truncated = truncate_to_width(content, content_width);
    let padding = content_width.saturating_sub(truncated.width());

    format!(
        "{DIM}{:>4}{RESET} {}{} {DIM}│{RESET} {DIM}{:>4}{RESET} {}",
        line_no_old,
        truncated,
        " ".repeat(padding),
        line_no_new,
        truncated,
    )
}

/// Format a paired line (removed on left, added on right).
fn format_paired_line(
    left: Option<&DiffLine>,
    right: Option<&DiffLine>,
    content_width: usize,
) -> String {
    let (left_num, left_content, left_spans) = match left {
        Some(DiffLine::Removed {
            line_no,
            content,
            inline_changes,
        }) => (Some(*line_no), content.as_str(), Some(inline_changes)),
        _ => (None, "", None),
    };

    let (right_num, right_content, right_spans) = match right {
        Some(DiffLine::Added {
            line_no,
            content,
            inline_changes,
        }) => (Some(*line_no), content.as_str(), Some(inline_changes)),
        _ => (None, "", None),
    };

    // Format left side
    let left_formatted = if left.is_some() {
        format_content_with_spans(left_content, left_spans, content_width, true)
    } else {
        " ".repeat(content_width)
    };

    // Format right side
    let right_formatted = if right.is_some() {
        format_content_with_spans(right_content, right_spans, content_width, false)
    } else {
        " ".repeat(content_width)
    };

    // Line numbers
    let left_num_str = match left_num {
        Some(n) => format!("{BG_REMOVED}{:>4}{RESET}", n),
        None => "    ".to_string(),
    };

    let right_num_str = match right_num {
        Some(n) => format!("{BG_ADDED}{:>4}{RESET}", n),
        None => "    ".to_string(),
    };

    format!(
        "{} {} {DIM}│{RESET} {} {}",
        left_num_str, left_formatted, right_num_str, right_formatted
    )
}

/// Format content with inline change highlighting.
fn format_content_with_spans(
    content: &str,
    spans: Option<&Vec<InlineSpan>>,
    max_width: usize,
    is_removed: bool,
) -> String {
    let bg_base = if is_removed { BG_REMOVED } else { BG_ADDED };
    let bg_emphasis = if is_removed {
        BG_CHANGED_DEL
    } else {
        BG_CHANGED_ADD
    };

    let spans = match spans {
        Some(s) if !s.is_empty() => s,
        _ => {
            // No spans, just apply base background
            let truncated = truncate_to_width(content, max_width);
            let padding = max_width.saturating_sub(truncated.width());
            return format!("{}{}{}{RESET}", bg_base, truncated, " ".repeat(padding));
        }
    };

    let mut result = String::new();
    let mut visual_width = 0;

    for span in spans {
        if visual_width >= max_width {
            break;
        }

        let span_content = &content[span.start..span.end.min(content.len())];
        let span_width = span_content.width();

        // Check if we need to truncate this span
        let remaining_width = max_width.saturating_sub(visual_width);
        let truncated_span = if span_width > remaining_width {
            truncate_to_width(span_content, remaining_width)
        } else {
            span_content.to_string()
        };

        if span.emphasized {
            result.push_str(&format!("{}{BOLD}{UNDERLINE}{}{RESET}", bg_emphasis, truncated_span));
        } else {
            result.push_str(&format!("{}{}{RESET}", bg_base, truncated_span));
        }

        visual_width += truncated_span.width();
    }

    // Pad to full width
    if visual_width < max_width {
        result.push_str(&format!("{}{}{RESET}", bg_base, " ".repeat(max_width - visual_width)));
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
    fn test_truncate_to_width() {
        assert_eq!(truncate_to_width("Hello", 10), "Hello");
        assert_eq!(truncate_to_width("Hello World", 5), "Hell…");
        assert_eq!(truncate_to_width("", 5), "");
    }

    #[test]
    fn test_truncate_unicode() {
        // CJK characters are 2 columns wide
        // "世界" (2+2=4 width) fits exactly in max_width 4, no truncation
        assert_eq!(truncate_to_width("世界", 4), "世界");

        // "世界你好" (2+2+2+2=8 width) exceeds max_width 4
        // With 1-width ellipsis reserved, target is 3, so "世" (2) fits
        // Result: "世…" (2+1=3 width)
        assert_eq!(truncate_to_width("世界你好", 4), "世…");
    }

    #[test]
    fn test_format_context_line() {
        let line = format_context_line(1, 1, "Hello", 20);
        assert!(line.contains("Hello"));
        assert!(line.contains("│"));
    }

    #[test]
    fn test_render_empty_diff() {
        let diff: Vec<DiffLine> = vec![];
        let options = VisualDiffOptions::with_width(120);
        let output = render(&diff, "a.md", "b.md", &options);
        // Should just have header
        assert!(!output.is_empty());
    }
}
