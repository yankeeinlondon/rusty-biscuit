//! Terminal layout application.
//!
//! The layout *data* types ([`Layout`], [`Alignment`], [`Margin`],
//! [`RowFill`], [`MaxWidth`]) live in [`renderable::layout`] and [`WordWrap`]
//! in [`renderable::wrap_policy`]; they are re-exported here for backwards
//! compatibility. Terminal-specific ANSI-width application lives in this
//! crate as the [`LayoutTerminalExt`] extension trait.

use crate::{
    terminal::Terminal,
    utils::block_constraint::{split_lines, visible_width, wrap_lines},
};

pub use renderable::layout::{Alignment, Layout, Margin, MaxWidth, RowFill};
pub use renderable::wrap_policy::WordWrap;

/// A `RenderableWrapper` is a utility which operates at a
/// lower level than a `TerminalRenderable` **component** and takes in
/// string content and outputs that string _wrapped_ in some
/// sort of formatting.
pub trait RenderableWrapper {
    fn render<T: Into<String>>(&self, content: T) -> String;

    fn fallback_render<T: Into<String>>(&self, content: T, term: &Terminal) -> String;
}

/// Terminal-specific layout application for [`Layout`].
///
/// These methods perform ANSI-width math and so live in `biscuit-terminal`
/// rather than `renderable`. Bring this trait into scope (it is re-exported
/// from [`crate::prelude`]) to call `apply_layout` / `apply_block_layout` on a
/// [`Layout`] value.
pub trait LayoutTerminalExt {
    /// Apply the layout to content, aligning each line independently.
    ///
    /// Each line is aligned within the available width on its own — useful for
    /// prose where wrapped lines are logically standalone and a "centered
    /// paragraph" effect is desired.
    ///
    /// For content with horizontal structure that must be preserved across
    /// lines (tree connectors, borders, column edges, bullet columns), use
    /// [`apply_block_layout`](Self::apply_block_layout) instead.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::utils::layout::{Alignment, Layout, LayoutTerminalExt};
    ///
    /// let layout = Layout {
    ///     alignment: Alignment::Left,
    ///     ..Layout::default()
    /// };
    /// let result = layout.apply_layout("Hello", 80);
    /// assert!(result.starts_with("Hello"));
    /// ```
    fn apply_layout(&self, content: &str, terminal_width: u32) -> String;

    /// Apply the layout to content as a cohesive block.
    ///
    /// Unlike [`apply_layout`](Self::apply_layout), which aligns each line
    /// independently, this method computes a single alignment offset based on
    /// the widest line in the block and applies that same offset to every
    /// line. This preserves the horizontal spatial structure of the content
    /// (e.g., tree connectors, ASCII art) while still shifting the block as a
    /// whole within the available width.
    ///
    /// Word wrap is intentionally **not** applied: block-aligned content
    /// typically relies on horizontal spatial relationships that wrapping
    /// would destroy.
    fn apply_block_layout(&self, content: &str, terminal_width: u32) -> String;
}

impl LayoutTerminalExt for Layout {
    fn apply_block_layout(&self, content: &str, terminal_width: u32) -> String {
        let left = Layout::resolve_margin(&self.left_margin, terminal_width);
        let right = Layout::resolve_margin(&self.right_margin, terminal_width);

        let available_width = terminal_width.saturating_sub(left).saturating_sub(right);
        if available_width == 0 {
            return String::new();
        }

        let had_trailing_newline = content.ends_with('\n');
        let mut lines = split_lines(content);
        if had_trailing_newline
            && let Some(last) = lines.last()
            && last.is_empty()
        {
            lines.pop();
        }

        let max_line_width = lines.iter().map(|l| visible_width(l)).max().unwrap_or(0);
        let block_padding = available_width.saturating_sub(max_line_width);
        let pre_align = match self.alignment {
            Alignment::Left => 0,
            Alignment::Right => block_padding,
            Alignment::Center => block_padding / 2,
        };

        let left_padding = " ".repeat(left as usize);
        let block_padding_str = " ".repeat(pre_align as usize);

        let should_fill = match &self.row_fill_strategy {
            RowFill::Fill => true,
            RowFill::Auto => self.page_bg_color.is_some(),
            RowFill::Exact => false,
        };

        let mut result = String::new();
        for line in &lines {
            result.push_str(&left_padding);
            result.push_str(&block_padding_str);
            result.push_str(line);

            if should_fill {
                let line_width = visible_width(line);
                let trailing = max_line_width.saturating_sub(line_width);
                result.push_str(&" ".repeat(trailing as usize));
            }

            result.push('\n');
        }

        if !had_trailing_newline && result.ends_with('\n') {
            result.pop();
        }

        result
    }

    fn apply_layout(&self, content: &str, terminal_width: u32) -> String {
        let left = Layout::resolve_margin(&self.left_margin, terminal_width);
        let right = Layout::resolve_margin(&self.right_margin, terminal_width);

        // Calculate available width for content
        let available_width = terminal_width.saturating_sub(left).saturating_sub(right);
        if available_width == 0 {
            return String::new();
        }

        // Remember if content ended with a newline so we can preserve it.
        // Without this, split_lines produces an empty trailing element that
        // gets padded with left_margin spaces — leaving invisible trailing
        // whitespace that triggers zsh's missing-newline indicator (%).
        let had_trailing_newline = content.ends_with('\n');

        // Split content into lines
        let mut lines = split_lines(content);

        // Remove the empty trailing element produced by a trailing newline;
        // we'll restore the newline at the end instead.
        if had_trailing_newline
            && let Some(last) = lines.last()
            && last.is_empty()
        {
            lines.pop();
        }

        // Apply word wrapping
        let wrapped_lines = match &self.word_wrap {
            WordWrap::None => lines,
            _ => wrap_lines(lines, &self.word_wrap, available_width),
        };

        // Build result with margins and alignment
        let left_padding = " ".repeat(left as usize);
        let mut result = String::new();

        // Determine if we should fill rows
        let should_fill = match &self.row_fill_strategy {
            RowFill::Fill => true,
            RowFill::Auto => self.page_bg_color.is_some(),
            RowFill::Exact => false,
        };

        for line in wrapped_lines {
            let line_width = visible_width(&line);
            let padding_needed = available_width.saturating_sub(line_width);

            // Apply alignment
            let (pre_align, post_align) = match self.alignment {
                Alignment::Left => (0, padding_needed),
                Alignment::Right => (padding_needed, 0),
                Alignment::Center => {
                    let left_pad = padding_needed / 2;
                    let right_pad = padding_needed - left_pad;
                    (left_pad, right_pad)
                }
            };

            result.push_str(&left_padding);
            result.push_str(&" ".repeat(pre_align as usize));
            result.push_str(&line);

            if should_fill {
                result.push_str(&" ".repeat(post_align as usize));
            }

            result.push('\n');
        }

        // Remove trailing newline (unless content originally ended with one,
        // which was stripped above and should be preserved)
        if !had_trailing_newline && result.ends_with('\n') {
            result.pop();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_left_margin() {
        let layout = Layout {
            left_margin: Margin::Chars(4),
            ..Layout::default()
        };
        let result = layout.apply_layout("Hello", 80);
        assert!(result.starts_with("    Hello"));
    }

    #[test]
    fn test_layout_center_alignment() {
        let layout = Layout {
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let result = layout.apply_layout("Hi", 80);
        // With 80 width and 2 char content, should have padding
        assert!(result.contains("Hi"));
    }

    #[test]
    fn test_layout_trailing_newline_not_padded() {
        // Content ending with \n (e.g., image escape sequences with scroll
        // compensation) should preserve the trailing newline without adding
        // left-margin padding to the empty trailing line.
        let layout = Layout {
            left_margin: Margin::Chars(4),
            ..Layout::default()
        };
        let result = layout.apply_layout("hello\n", 80);
        // Should end with \n, not with trailing spaces
        assert!(
            result.ends_with('\n'),
            "Trailing newline should be preserved"
        );
        assert!(
            !result.ends_with("    \n"),
            "Empty trailing line should not be padded"
        );
        assert_eq!(result, "    hello\n");
    }

    #[test]
    fn test_layout_no_trailing_newline_unchanged() {
        // Content without trailing newline should not gain one
        let layout = Layout {
            left_margin: Margin::Chars(4),
            ..Layout::default()
        };
        let result = layout.apply_layout("hello", 80);
        assert!(!result.ends_with('\n'));
        assert_eq!(result, "    hello");
    }

    #[test]
    fn test_word_wrap_applied() {
        let layout = Layout {
            word_wrap: WordWrap::WrapProse(None, None),
            ..Layout::default()
        };
        // With default 80 width, a long line should wrap
        let long_text = "a".repeat(100);
        let result = layout.apply_layout(&long_text, 80);
        // Should have been split into multiple lines
        assert!(result.contains('\n') || result.len() <= 80);
    }

    #[test]
    fn test_apply_block_layout_left_alignment_unchanged() {
        let layout = Layout {
            alignment: Alignment::Left,
            ..Layout::default()
        };
        let content = "abc\nde\nf";
        let result = layout.apply_block_layout(content, 20);
        assert_eq!(result, "abc\nde\nf");
    }

    #[test]
    fn test_apply_block_layout_center_shifts_block_uniformly() {
        let layout = Layout {
            alignment: Alignment::Center,
            ..Layout::default()
        };
        // Max line width = 5 ("aaaaa"), available = 11, padding = 6, pre = 3
        let content = "aaaaa\nbb\nccc";
        let result = layout.apply_block_layout(content, 11);
        let lines: Vec<&str> = result.split('\n').collect();
        // All lines get the same left padding (3 spaces), preserving relative offsets
        assert_eq!(lines, vec!["   aaaaa", "   bb", "   ccc"]);
    }

    #[test]
    fn test_apply_block_layout_right_aligns_to_widest_line() {
        let layout = Layout {
            alignment: Alignment::Right,
            ..Layout::default()
        };
        // Max line width = 5, available = 10, padding = 5
        let content = "aaaaa\nbb\nccc";
        let result = layout.apply_block_layout(content, 10);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines, vec!["     aaaaa", "     bb", "     ccc"]);
    }

    #[test]
    fn test_apply_block_layout_preserves_tree_structure() {
        // Simulates the case reported in the bug: tree connectors must stay
        // aligned when the block itself is centered.
        let layout = Layout {
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let content = "root\n├── a\n│   └── long_name\n└── b";
        let result = layout.apply_block_layout(content, 40);
        // All lines should be prefixed with the same number of spaces, so the
        // first non-space char in each line maintains its relative position
        // to the others.
        let lines: Vec<&str> = result.split('\n').collect();
        let leading: Vec<usize> = lines
            .iter()
            .map(|l| l.chars().take_while(|c| *c == ' ').count())
            .collect();
        assert!(
            leading.iter().all(|n| *n == leading[0]),
            "all lines should share the same left padding: {leading:?}"
        );
    }

    #[test]
    fn test_apply_block_layout_respects_margins() {
        let layout = Layout {
            left_margin: Margin::Chars(2),
            right_margin: Margin::Chars(2),
            alignment: Alignment::Center,
            ..Layout::default()
        };
        // Available = 20 - 2 - 2 = 16. Max line = 4 ("abcd"), block padding = 12, pre = 6
        // Total leading = left_margin (2) + pre_align (6) = 8
        let result = layout.apply_block_layout("abcd\nab", 20);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines, vec!["        abcd", "        ab"]);
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_layout_never_panics(
            text in ".*",
            terminal_width in 1..=500u32,
            margin_left in 0..=100u32,
            margin_right in 0..=100u32,
            indentation in 0..=100u32
        ) {
            let layout = Layout {
                left_margin: Margin::Chars(margin_left),
                right_margin: Margin::Chars(margin_right),
                word_wrap: WordWrap::WrapProse(Some(indentation), None),
                ..Default::default()
            };
            let _ = layout.apply_layout(&text, terminal_width);
            let _ = layout.available_width(terminal_width);
        }
    }
}
