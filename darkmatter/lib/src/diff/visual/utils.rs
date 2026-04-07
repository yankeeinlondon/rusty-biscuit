//! Shared utilities for visual diff rendering.

use super::diff::DiffLine;
use biscuit_terminal::utils::UnicodeWidthStr;
use std::collections::HashSet;

/// Filter lines to show only changes and surrounding context.
///
/// Returns a set of indices for lines that should be visible in the rendered diff.
/// This includes all changed lines (additions/removals) plus the specified number
/// of context lines before and after each change.
pub(super) fn filter_with_context(diff: &[DiffLine], context_lines: usize) -> HashSet<usize> {
    let mut visible = HashSet::new();

    // First pass: mark all change lines.
    let change_indices: Vec<usize> = diff
        .iter()
        .enumerate()
        .filter(|(_, line)| !line.is_context())
        .map(|(idx, _)| idx)
        .collect();

    // Second pass: add context around each change.
    for &change_idx in &change_indices {
        let start = change_idx.saturating_sub(context_lines);
        for i in start..=change_idx {
            visible.insert(i);
        }

        let end = (change_idx + context_lines + 1).min(diff.len());
        for i in change_idx..end {
            visible.insert(i);
        }
    }

    visible
}

/// Wrap a string to fit within a visual width, returning multiple lines if needed.
///
/// Wraps text to fit within `max_width` display columns. Tries to break at word
/// boundaries (whitespace); falls back to hard character-level breaks for words
/// longer than `max_width`.
pub(super) fn wrap_to_width(s: &str, max_width: usize) -> Vec<String> {
    if s.is_empty() || max_width == 0 {
        return vec![String::new()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width: usize = 0;

    for word in s.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);

        if word_width > max_width {
            // Flush current line before handling the long word
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            // Hard-break the long word using biscuit-terminal
            let chunks = biscuit_terminal::utils::block_constraint::wrap_lines(
                vec![word.to_string()],
                &biscuit_terminal::utils::layout::WordWrap::None,
                max_width as u32,
            );
            let num_chunks = chunks.len();
            for (i, chunk) in chunks.into_iter().enumerate() {
                if i < num_chunks - 1 {
                    lines.push(chunk);
                } else {
                    // Last chunk may be partial — carry it as the current line
                    current_width = UnicodeWidthStr::width(chunk.as_str());
                    current = chunk;
                }
            }
        } else if current_width == 0 {
            current = word.to_string();
            current_width = word_width;
        } else if current_width + 1 + word_width <= max_width {
            current.push(' ');
            current.push_str(word);
            current_width += 1 + word_width;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
            current_width = word_width;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
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
    fn test_filter_with_context() {
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
}
