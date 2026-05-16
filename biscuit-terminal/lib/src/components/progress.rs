use crate::{components::renderable::TerminalRenderable, terminal::Terminal, utils::layout::{Layout, LayoutTerminalExt}};
use std::any::Any;

/// A horizontal progress bar for terminal display.
///
/// Renders a progress bar showing completion percentage with configurable
/// width, fill/empty characters, and color support.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::progress::Progress;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// let bar = Progress::new(0.75); // 75% complete
/// let output = bar.render_optimistic(Some(40));
/// assert!(output.contains("75%"));
/// ```
#[derive(Debug, Clone)]
pub struct Progress {
    /// Value between 0.0 and 1.0
    value: f32,
    /// Optional label shown before the bar
    label: Option<String>,
    /// Width of the bar portion in characters (default: 20)
    bar_width: u32,
    /// Character for filled portion
    fill_char: char,
    /// Character for empty portion
    empty_char: char,
    /// Left bracket character
    left_bracket: char,
    /// Right bracket character
    right_bracket: char,
    /// Layout configuration
    layout: Layout,
}

impl Progress {
    /// Creates a new progress bar with the given value (clamped to 0.0..=1.0).
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            label: None,
            bar_width: 20,
            fill_char: '█',  // U+2588
            empty_char: '·', // U+00B7 (middle dot)
            left_bracket: '[',
            right_bracket: ']',
            layout: Layout::default(),
        }
    }

    /// Sets a label to display before the progress bar.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the width of the bar portion in characters.
    pub fn with_bar_width(mut self, width: u32) -> Self {
        self.bar_width = width;
        self
    }

    /// Sets the character used for the filled portion of the bar.
    pub fn with_fill_char(mut self, ch: char) -> Self {
        self.fill_char = ch;
        self
    }

    /// Sets the character used for the empty portion of the bar.
    pub fn with_empty_char(mut self, ch: char) -> Self {
        self.empty_char = ch;
        self
    }

    /// Sets the bracket characters (left and right).
    pub fn with_brackets(mut self, left: char, right: char) -> Self {
        self.left_bracket = left;
        self.right_bracket = right;
        self
    }

    /// Renders the progress bar content (without layout application).
    fn render_bar(&self) -> String {
        let percentage = (self.value * 100.0).round() as u32;
        let filled_count =
            ((self.value * self.bar_width as f32).round() as u32).min(self.bar_width);
        let empty_count = self.bar_width.saturating_sub(filled_count);

        let bar = format!(
            "{}{}{}",
            self.fill_char.to_string().repeat(filled_count as usize),
            self.empty_char.to_string().repeat(empty_count as usize),
            ""
        );

        let percentage_str = format!("{:3}%", percentage);

        if let Some(ref label) = self.label {
            format!(
                "{} {}{}{} {}",
                label, self.left_bracket, bar, self.right_bracket, percentage_str
            )
        } else {
            format!(
                "{}{}{} {}",
                self.left_bracket, bar, self.right_bracket, percentage_str
            )
        }
    }
}

impl TerminalRenderable for Progress {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let bar_content = self.render_bar();
        // Label and bar segments form a single visual unit — align as a block.
        self.layout.apply_block_layout(&bar_content, width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let bar_content = self.render_bar();
        self.layout.apply_block_layout(&bar_content, width)
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_zero() {
        let bar = Progress::new(0.0);
        let output = bar.render_optimistic(Some(80));
        assert!(output.contains("  0%"), "Should show 0%, got: {}", output);
        assert!(output.contains('['), "Should contain left bracket");
        assert!(output.contains(']'), "Should contain right bracket");
        // All characters should be empty (20 dots for default bar_width)
        let expected_empty = "·".repeat(20);
        assert!(
            output.contains(&expected_empty),
            "Should have 20 empty chars, got: {}",
            output
        );
        assert!(!output.contains('█'), "Should not contain fill character");
    }

    #[test]
    fn test_progress_half() {
        let bar = Progress::new(0.5);
        let output = bar.render_optimistic(Some(80));
        assert!(output.contains("50%"), "Should show 50%, got: {}", output);
        assert!(output.contains('█'), "Should contain fill character");
        assert!(output.contains('·'), "Should contain empty character");
    }

    #[test]
    fn test_progress_full() {
        let bar = Progress::new(1.0);
        let output = bar.render_optimistic(Some(80));
        assert!(output.contains("100%"), "Should show 100%, got: {}", output);
        // All characters should be filled (20 blocks for default bar_width)
        let expected_filled = "█".repeat(20);
        assert!(
            output.contains(&expected_filled),
            "Should have 20 fill chars, got: {}",
            output
        );
        assert!(!output.contains('·'), "Should not contain empty characters");
    }

    #[test]
    fn test_progress_with_label() {
        let bar = Progress::new(0.75).with_label("Loading");
        let output = bar.render_optimistic(Some(80));
        assert!(
            output.contains("Loading"),
            "Should show label, got: {}",
            output
        );
        assert!(
            output.contains("75%"),
            "Should show percentage, got: {}",
            output
        );
    }

    #[test]
    fn test_progress_clamps_value() {
        // Test value > 1.0 clamps to 1.0
        let bar_high = Progress::new(1.5);
        let output_high = bar_high.render_optimistic(Some(80));
        assert!(
            output_high.contains("100%"),
            "Should clamp to 100%, got: {}",
            output_high
        );

        // Test value < 0.0 clamps to 0.0
        let bar_low = Progress::new(-0.5);
        let output_low = bar_low.render_optimistic(Some(80));
        assert!(
            output_low.contains("0%"),
            "Should clamp to 0%, got: {}",
            output_low
        );
    }

    #[test]
    fn test_progress_custom_bar_width() {
        let bar = Progress::new(0.5).with_bar_width(10);
        let output = bar.render_optimistic(Some(80));
        // Should have 5 filled and 5 empty (total 10)
        assert!(output.contains("50%"), "Should show 50%, got: {}", output);
    }

    #[test]
    fn test_progress_custom_characters() {
        let bar = Progress::new(0.5)
            .with_fill_char('#')
            .with_empty_char('-')
            .with_brackets('(', ')');
        let output = bar.render_optimistic(Some(80));
        assert!(output.contains('#'), "Should contain custom fill char");
        assert!(output.contains('-'), "Should contain custom empty char");
        assert!(output.contains('('), "Should contain custom left bracket");
        assert!(output.contains(')'), "Should contain custom right bracket");
    }

    #[test]
    fn test_progress_percentage_alignment() {
        // Check that percentages are right-aligned in the format
        let bar_0 = Progress::new(0.0);
        let output_0 = bar_0.render_optimistic(Some(80));
        assert!(
            output_0.contains("  0%"),
            "0% should be right-aligned with 2 spaces"
        );

        let bar_75 = Progress::new(0.75);
        let output_75 = bar_75.render_optimistic(Some(80));
        assert!(
            output_75.contains(" 75%"),
            "75% should be right-aligned with 1 space"
        );

        let bar_100 = Progress::new(1.0);
        let output_100 = bar_100.render_optimistic(Some(80));
        assert!(
            output_100.contains("100%"),
            "100% should have no leading space"
        );
    }

    #[test]
    fn test_progress_uses_layout() {
        use crate::utils::layout::Margin;

        let bar = Progress::new(0.5).left_margin(Margin::Chars(4));
        let output = bar.render_optimistic(Some(80));
        assert!(
            output.starts_with("    "),
            "Should have left margin of 4 spaces"
        );
    }
}
