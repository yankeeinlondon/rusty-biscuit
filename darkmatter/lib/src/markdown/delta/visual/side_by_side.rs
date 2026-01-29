//! Side-by-side diff renderer for wide terminals (>110 columns).
//!
//! Layout:
//! ```text
//! [left_num:4] [left_content] │ [right_num:4] [right_content]
//! ```
//!
//! Long lines are word-wrapped to fit within the available panel width,
//! with continuation lines showing empty line numbers and maintaining
//! the appropriate background color.

use super::VisualDiffOptions;
use super::diff::{DiffLine, InlineSpan};
use textwrap::{Options as WrapOptions, wrap};
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
                let lines = format_context_line(*line_no_old, *line_no_new, content, content_width);
                for line in lines {
                    output.push_str(&line);
                    output.push('\n');
                }
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
                    let lines = format_paired_line(left, right, content_width);
                    for line in lines {
                        output.push_str(&line);
                        output.push('\n');
                    }
                }
            }
            DiffLine::Added { .. } => {
                // Standalone added line (no preceding removed)
                let lines = format_paired_line(None, Some(&diff[i]), content_width);
                for line in lines {
                    output.push_str(&line);
                    output.push('\n');
                }
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
    let left_label = truncate_label(label_left, content_width);
    let right_label = truncate_label(label_right, content_width);

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

/// Truncate a label to fit within a maximum width (for headers only).
///
/// Unlike content which wraps, labels are truncated with an ellipsis.
fn truncate_label(s: &str, max_width: usize) -> String {
    if s.width() <= max_width {
        return s.to_string();
    }

    // Need to truncate - reserve space for ellipsis
    let target_width = max_width.saturating_sub(1);
    let mut result = String::new();
    let mut current_width = 0;

    for ch in s.chars() {
        let char_width = ch.width().unwrap_or(0);
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

/// Format a context line (unchanged, shown on both sides).
///
/// Returns one or more output lines if the content needs to wrap.
fn format_context_line(
    line_no_old: usize,
    line_no_new: usize,
    content: &str,
    content_width: usize,
) -> Vec<String> {
    let wrapped = wrap_to_width(content, content_width);
    let mut lines = Vec::with_capacity(wrapped.len());

    for (idx, line_content) in wrapped.iter().enumerate() {
        let padding = content_width.saturating_sub(line_content.width());

        if idx == 0 {
            // First line shows line numbers
            lines.push(format!(
                "{DIM}{:>4}{RESET} {}{} {DIM}│{RESET} {DIM}{:>4}{RESET} {}",
                line_no_old,
                line_content,
                " ".repeat(padding),
                line_no_new,
                line_content,
            ));
        } else {
            // Continuation lines have empty line number area
            lines.push(format!(
                "{DIM}    {RESET} {}{} {DIM}│{RESET} {DIM}    {RESET} {}",
                line_content,
                " ".repeat(padding),
                line_content,
            ));
        }
    }

    lines
}

/// Format a paired line (removed on left, added on right).
///
/// Returns one or more output lines if either side needs to wrap.
/// When wrapping occurs, both sides are padded to have the same number
/// of visual lines.
fn format_paired_line(
    left: Option<&DiffLine>,
    right: Option<&DiffLine>,
    content_width: usize,
) -> Vec<String> {
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

    // Wrap both sides
    let left_wrapped = if left.is_some() {
        wrap_to_width(left_content, content_width)
    } else {
        vec![String::new()]
    };

    let right_wrapped = if right.is_some() {
        wrap_to_width(right_content, content_width)
    } else {
        vec![String::new()]
    };

    // Determine the maximum number of lines between both sides
    let max_lines = left_wrapped.len().max(right_wrapped.len());
    let mut output_lines = Vec::with_capacity(max_lines);

    for idx in 0..max_lines {
        let left_line = left_wrapped.get(idx).map(|s| s.as_str()).unwrap_or("");
        let right_line = right_wrapped.get(idx).map(|s| s.as_str()).unwrap_or("");

        // For the first line, show the line number; for continuation lines, show empty
        let (show_left_num, show_right_num) = if idx == 0 {
            (left_num, right_num)
        } else {
            (None, None)
        };

        // Determine if we have content on each side (for coloring)
        let has_left_content = left.is_some() && idx < left_wrapped.len();
        let has_right_content = right.is_some() && idx < right_wrapped.len();

        // Format left side with appropriate styling
        let left_formatted = if has_left_content {
            // For first line with spans, use full span formatting
            // For continuation lines, just use base background
            if idx == 0 && left_spans.is_some() {
                format_content_with_spans(left_line, left_spans, content_width, true)
            } else {
                let padding = content_width.saturating_sub(left_line.width());
                format!("{BG_REMOVED}{}{}{RESET}", left_line, " ".repeat(padding))
            }
        } else if left.is_some() {
            // Empty continuation line with background
            format!("{BG_REMOVED}{}{RESET}", " ".repeat(content_width))
        } else {
            " ".repeat(content_width)
        };

        // Format right side with appropriate styling
        let right_formatted = if has_right_content {
            if idx == 0 && right_spans.is_some() {
                format_content_with_spans(right_line, right_spans, content_width, false)
            } else {
                let padding = content_width.saturating_sub(right_line.width());
                format!("{BG_ADDED}{}{}{RESET}", right_line, " ".repeat(padding))
            }
        } else if right.is_some() {
            // Empty continuation line with background
            format!("{BG_ADDED}{}{RESET}", " ".repeat(content_width))
        } else {
            " ".repeat(content_width)
        };

        // Line numbers
        let left_num_str = match show_left_num {
            Some(n) => format!("{BG_REMOVED}{:>4}{RESET}", n),
            None if left.is_some() => format!("{BG_REMOVED}    {RESET}"),
            None => "    ".to_string(),
        };

        let right_num_str = match show_right_num {
            Some(n) => format!("{BG_ADDED}{:>4}{RESET}", n),
            None if right.is_some() => format!("{BG_ADDED}    {RESET}"),
            None => "    ".to_string(),
        };

        output_lines.push(format!(
            "{} {} {DIM}│{RESET} {} {}",
            left_num_str, left_formatted, right_num_str, right_formatted
        ));
    }

    output_lines
}

/// Format content with inline change highlighting.
///
/// This function formats a single line of content (possibly wrapped) with
/// inline change spans. The `content` parameter is expected to be the first
/// line of wrapped content, and spans are adjusted to fit within this line.
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
            // No spans, just apply base background with wrapping
            let wrapped = wrap_to_width(content, max_width);
            let first_line = wrapped.first().map(|s| s.as_str()).unwrap_or("");
            let padding = max_width.saturating_sub(first_line.width());
            return format!("{}{}{}{RESET}", bg_base, first_line, " ".repeat(padding));
        }
    };

    // Wrap content to get the first line
    let wrapped = wrap_to_width(content, max_width);
    let first_line = wrapped.first().map(|s| s.as_str()).unwrap_or("");
    let first_line_len = first_line.len();

    let mut result = String::new();
    let mut visual_width = 0;
    let mut byte_pos = 0;

    // Process spans that fall within the first wrapped line
    for span in spans {
        if visual_width >= max_width || byte_pos >= first_line_len {
            break;
        }

        // Calculate how much of this span falls within the first line
        let span_start_in_line = span.start.max(byte_pos);
        let span_end_in_line = span.end.min(first_line_len);

        if span_start_in_line >= span_end_in_line {
            // This span is entirely outside the first line or already processed
            continue;
        }

        // Add any content before this span (if there's a gap)
        if span_start_in_line > byte_pos {
            let gap_content = &content[byte_pos..span_start_in_line];
            let gap_width = gap_content.width();
            let remaining = max_width.saturating_sub(visual_width);
            if gap_width <= remaining {
                result.push_str(&format!("{}{}{RESET}", bg_base, gap_content));
                visual_width += gap_width;
            }
        }

        // Add the span content
        let span_content = &content[span_start_in_line..span_end_in_line];
        let span_width = span_content.width();
        let remaining = max_width.saturating_sub(visual_width);

        if span_width <= remaining {
            if span.emphasized {
                result.push_str(&format!(
                    "{}{BOLD}{UNDERLINE}{}{RESET}",
                    bg_emphasis, span_content
                ));
            } else {
                result.push_str(&format!("{}{}{RESET}", bg_base, span_content));
            }
            visual_width += span_width;
        }

        byte_pos = span_end_in_line;
    }

    // Add any remaining content after all spans
    if byte_pos < first_line_len && visual_width < max_width {
        let remaining_content = &content[byte_pos..first_line_len];
        let remaining_width = remaining_content.width();
        let space_left = max_width.saturating_sub(visual_width);
        if remaining_width <= space_left {
            result.push_str(&format!("{}{}{RESET}", bg_base, remaining_content));
            visual_width += remaining_width;
        }
    }

    // Pad to full width
    if visual_width < max_width {
        result.push_str(&format!(
            "{}{}{RESET}",
            bg_base,
            " ".repeat(max_width - visual_width)
        ));
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
    fn test_wrap_to_width_short_string() {
        // String fits within width - single line returned
        let result = wrap_to_width("Hello", 10);
        assert_eq!(result, vec!["Hello"]);
    }

    #[test]
    fn test_wrap_to_width_long_string() {
        // String exceeds width - wraps to multiple lines
        let result = wrap_to_width("Hello World", 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "Hello");
        assert_eq!(result[1], "World");
    }

    #[test]
    fn test_wrap_to_width_empty() {
        let result = wrap_to_width("", 5);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_wrap_to_width_zero_width() {
        let result = wrap_to_width("Hello", 0);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_wrap_to_width_unicode() {
        // CJK characters are 2 columns wide
        // "世界你好" (2+2+2+2=8 width) should wrap at width 4
        let result = wrap_to_width("世界你好", 4);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "世界");
        assert_eq!(result[1], "你好");
    }

    #[test]
    fn test_wrap_to_width_long_word_breaks() {
        // A single long word should be broken if necessary
        let result = wrap_to_width("abcdefghij", 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "abcde");
        assert_eq!(result[1], "fghij");
    }

    #[test]
    fn test_format_context_line_short() {
        let lines = format_context_line(1, 1, "Hello", 20);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Hello"));
        assert!(lines[0].contains("│"));
    }

    #[test]
    fn test_format_context_line_wraps() {
        // Regression test: long lines should wrap instead of truncate
        let long_content = "This is a very long line that should wrap to multiple visual lines";
        let lines = format_context_line(42, 42, long_content, 20);

        // Should produce multiple visual lines
        assert!(
            lines.len() > 1,
            "Long content should wrap to multiple lines"
        );

        // First line should show line numbers
        assert!(lines[0].contains("42"));
        assert!(lines[0].contains("│"));

        // All lines should contain the separator
        for line in &lines {
            assert!(line.contains("│"));
        }
    }

    #[test]
    fn test_format_paired_line_wraps() {
        // Regression test: long lines in paired diff should wrap
        let long_removed = "This is a very long line that was removed and should wrap properly";
        let long_added = "This is a very long line that was added and should also wrap";

        let removed = DiffLine::Removed {
            line_no: 10,
            content: long_removed.to_string(),
            inline_changes: vec![],
        };
        let added = DiffLine::Added {
            line_no: 10,
            content: long_added.to_string(),
            inline_changes: vec![],
        };

        let lines = format_paired_line(Some(&removed), Some(&added), 20);

        // Should produce multiple visual lines
        assert!(
            lines.len() > 1,
            "Long paired content should wrap to multiple lines"
        );

        // First line should show line numbers
        assert!(lines[0].contains("10"));
    }

    #[test]
    fn test_render_empty_diff() {
        let diff: Vec<DiffLine> = vec![];
        let options = VisualDiffOptions::with_width(120);
        let output = render(&diff, "a.md", "b.md", &options);
        // Should just have header
        assert!(!output.is_empty());
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
                content: "This is a very long line that exceeds the available width and must wrap to multiple lines".to_string(),
                inline_changes: vec![],
            },
            DiffLine::Added {
                line_no: 2,
                content: "This is another very long line that also exceeds the width and wraps".to_string(),
                inline_changes: vec![],
            },
        ];

        // Use a narrow width to force wrapping
        let options = VisualDiffOptions::with_width(80);
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
