use crate::{
    components::renderable::TerminalRenderable,
    discovery::detection::ColorDepth,
    render_tree::style::color_sgr,
    terminal::Terminal,
    utils::layout::{Layout, LayoutTerminalExt},
};
use renderable::color::Color;
use renderable::tree::{ProgressHints, RenderNode};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Typed style slot for a [`Progress`] bar's track and bracket appearance.
///
/// Spec B D5 model: a rich component exposes a typed component style struct
/// rather than scattered bespoke fields. `ProgressStyle` is the migrated home
/// of `Progress`'s former `fill_char` / `empty_char` / `left_bracket` /
/// `right_bracket` glyph fields, and also carries the optional slot colors
/// for the filled track, empty track, and brackets. A slot color is a
/// [`Color`] so it degrades across terminal color depths through the same
/// shared lowering the [`Style`](renderable::style::Style) primitive uses.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::progress::ProgressStyle;
///
/// let style = ProgressStyle::default();
/// assert_eq!(style.fill_char, '█');
/// assert_eq!(style.empty_char, '·');
/// assert!(style.filled_color.is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProgressStyle {
    /// Glyph painted for the filled portion of the track.
    pub fill_char: char,
    /// Glyph painted for the empty portion of the track.
    pub empty_char: char,
    /// Glyph painted at the left bracket.
    pub left_bracket: char,
    /// Glyph painted at the right bracket.
    pub right_bracket: char,
    /// Color of the filled portion of the track, if any.
    pub filled_color: Option<Color>,
    /// Color of the empty portion of the track, if any.
    pub empty_color: Option<Color>,
    /// Color of the left and right bracket glyphs, if any.
    pub bracket_color: Option<Color>,
}

impl Default for ProgressStyle {
    fn default() -> Self {
        Self {
            fill_char: '█',  // U+2588
            empty_char: '·', // U+00B7 (middle dot)
            left_bracket: '[',
            right_bracket: ']',
            filled_color: None,
            empty_color: None,
            bracket_color: None,
        }
    }
}

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
    /// Typed style slot for the track and bracket glyphs.
    style: ProgressStyle,
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
            style: ProgressStyle::default(),
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
    ///
    /// Compatibility shim: the glyph is stored on the typed [`ProgressStyle`]
    /// slot.
    pub fn with_fill_char(mut self, ch: char) -> Self {
        self.style.fill_char = ch;
        self
    }

    /// Sets the character used for the empty portion of the bar.
    ///
    /// Compatibility shim: the glyph is stored on the typed [`ProgressStyle`]
    /// slot.
    pub fn with_empty_char(mut self, ch: char) -> Self {
        self.style.empty_char = ch;
        self
    }

    /// Sets the bracket characters (left and right).
    ///
    /// Compatibility shim: the glyphs are stored on the typed
    /// [`ProgressStyle`] slot.
    pub fn with_brackets(mut self, left: char, right: char) -> Self {
        self.style.left_bracket = left;
        self.style.right_bracket = right;
        self
    }

    /// Sets the color of the filled portion of the track.
    ///
    /// Stored on the typed [`ProgressStyle`] slot and lowered through the
    /// shared, capability-aware color path when the bar renders.
    pub fn with_filled_color(mut self, color: Color) -> Self {
        self.style.filled_color = Some(color);
        self
    }

    /// Sets the color of the empty portion of the track.
    pub fn with_empty_color(mut self, color: Color) -> Self {
        self.style.empty_color = Some(color);
        self
    }

    /// Sets the color of the left and right bracket glyphs.
    pub fn with_bracket_color(mut self, color: Color) -> Self {
        self.style.bracket_color = Some(color);
        self
    }

    /// The typed [`ProgressStyle`] slot for this bar's glyphs and colors.
    pub fn style(&self) -> ProgressStyle {
        self.style
    }

    /// Renders the progress bar content (without layout application).
    ///
    /// `depth` is the terminal's [`ColorDepth`]; any declared slot colors are
    /// degraded against it through the shared lowering path so the colored
    /// track and brackets render on truecolor, 256-color, and 16-color
    /// terminals and are dropped only when the terminal has no color support.
    fn render_bar(&self, depth: ColorDepth) -> String {
        let percentage = (self.value * 100.0).round() as u32;
        let filled_count =
            ((self.value * self.bar_width as f32).round() as u32).min(self.bar_width);
        let empty_count = self.bar_width.saturating_sub(filled_count);

        let filled = paint_fg(
            &self.style.fill_char.to_string().repeat(filled_count as usize),
            self.style.filled_color,
            depth,
        );
        let empty = paint_fg(
            &self.style.empty_char.to_string().repeat(empty_count as usize),
            self.style.empty_color,
            depth,
        );
        let left = paint_fg(
            &self.style.left_bracket.to_string(),
            self.style.bracket_color,
            depth,
        );
        let right = paint_fg(
            &self.style.right_bracket.to_string(),
            self.style.bracket_color,
            depth,
        );

        let percentage_str = format!("{percentage:3}%");

        if let Some(ref label) = self.label {
            format!("{label} {left}{filled}{empty}{right} {percentage_str}")
        } else {
            format!("{left}{filled}{empty}{right} {percentage_str}")
        }
    }
}

/// Wraps `text` in a foreground SGR escape for `color`, degraded to `depth`.
///
/// Returns `text` unchanged when there is no color, when the text is empty,
/// or when `depth` is [`ColorDepth::None`] — the shared `color_sgr` lowering
/// yields `None` in the last case.
pub(crate) fn paint_fg(text: &str, color: Option<Color>, depth: ColorDepth) -> String {
    if text.is_empty() {
        return String::new();
    }
    match color.and_then(|c| color_sgr(c, &depth, false)) {
        Some(sgr) => format!("{sgr}{text}\x1b[39m"),
        None => text.to_string(),
    }
}

impl TerminalRenderable for Progress {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        // The optimistic path assumes a truecolor terminal, matching the
        // other components' `render_optimistic` (`Terminal::new_optimistic`).
        let bar_content = self.render_bar(ColorDepth::TrueColor);
        // Label and bar segments form a single visual unit — align as a block.
        self.layout.apply_block_layout(&bar_content, width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let bar_content = self.render_bar(term.color_depth);
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

    /// Projects the progress bar to a [`NodeKind::Paragraph`] carrying
    /// [`ProgressHints`].
    ///
    /// The paragraph's visible text is `"{label} {percentage}%"` (or just
    /// `"{percentage}%"` when there is no label) so renderers without progress
    /// hint support degrade to readable plain text. Renderers that recognize
    /// the hints reconstruct the full bar.
    ///
    /// [`NodeKind::Paragraph`]: renderable::tree::NodeKind::Paragraph
    fn render_tree_node(&self) -> Option<RenderNode> {
        let percentage = (self.value * 100.0).round() as u32;
        let visible = match &self.label {
            Some(label) => format!("{label} {percentage}%"),
            None => format!("{percentage}%"),
        };

        let mut node = RenderNode::paragraph(vec![RenderNode::text(visible)]);
        node.attrs.set_progress_hints(&ProgressHints {
            value: self.value,
            bar_width: self.bar_width,
            fill_char: self.style.fill_char,
            empty_char: self.style.empty_char,
            left_bracket: self.style.left_bracket,
            right_bracket: self.style.right_bracket,
            filled_color: self.style.filled_color,
            empty_color: self.style.empty_color,
            bracket_color: self.style.bracket_color,
        });
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        Some(node)
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
        use crate::utils::layout::{Length, TargetValue};

        let bar = Progress::new(0.5).left_margin(TargetValue::universal(Length::ch(4)));
        let output = bar.render_optimistic(Some(80));
        assert!(
            output.starts_with("    "),
            "Should have left margin of 4 spaces"
        );
    }

    #[test]
    fn progress_style_default_glyphs() {
        let style = ProgressStyle::default();
        assert_eq!(style.fill_char, '█');
        assert_eq!(style.empty_char, '·');
        assert_eq!(style.left_bracket, '[');
        assert_eq!(style.right_bracket, ']');
    }

    #[test]
    fn progress_style_serde_roundtrip() {
        let style = ProgressStyle {
            fill_char: '#',
            empty_char: '-',
            left_bracket: '(',
            right_bracket: ')',
            filled_color: Some(Color::BasicColor(renderable::color::BasicColor::Green)),
            empty_color: None,
            bracket_color: Some(Color::BasicColor(renderable::color::BasicColor::Cyan)),
        };
        let json = serde_json::to_string(&style).unwrap();
        let back: ProgressStyle = serde_json::from_str(&json).unwrap();
        assert_eq!(style, back);
    }

    #[test]
    fn progress_builder_shims_write_to_style_slot() {
        let bar = Progress::new(0.5)
            .with_fill_char('#')
            .with_empty_char('-')
            .with_brackets('(', ')');
        assert_eq!(bar.style().fill_char, '#');
        assert_eq!(bar.style().empty_char, '-');
        assert_eq!(bar.style().left_bracket, '(');
        assert_eq!(bar.style().right_bracket, ')');
    }

    #[test]
    fn progress_slot_color_builders_write_to_style() {
        use renderable::color::BasicColor;
        let bar = Progress::new(0.5)
            .with_filled_color(Color::BasicColor(BasicColor::Green))
            .with_empty_color(Color::BasicColor(BasicColor::Black))
            .with_bracket_color(Color::BasicColor(BasicColor::Cyan));
        assert_eq!(
            bar.style().filled_color,
            Some(Color::BasicColor(BasicColor::Green))
        );
        assert_eq!(
            bar.style().empty_color,
            Some(Color::BasicColor(BasicColor::Black))
        );
        assert_eq!(
            bar.style().bracket_color,
            Some(Color::BasicColor(BasicColor::Cyan))
        );
    }

    #[test]
    fn progress_renders_slot_colors_as_sgr() {
        use renderable::color::BasicColor;
        let bar = Progress::new(0.5)
            .with_filled_color(Color::BasicColor(BasicColor::Green))
            .with_bracket_color(Color::BasicColor(BasicColor::Cyan));
        let output = bar.render_optimistic(Some(80));
        // Green fg (code 32) for the filled track, cyan fg (code 36) for the
        // brackets — both lowered through the shared color path.
        assert!(output.contains("\x1b[32m"), "filled color SGR missing: {output:?}");
        assert!(output.contains("\x1b[36m"), "bracket color SGR missing: {output:?}");
    }

    #[test]
    fn progress_slot_colors_degrade_with_color_depth() {
        use crate::discovery::detection::ColorDepth;
        use renderable::color::{BasicColor, RgbColor};

        // An RGB filled color degrades across color depths.
        let bar = Progress::new(0.5).with_filled_color(Color::Rgb(RgbColor::new(
            10,
            200,
            40,
            BasicColor::Green,
        )));

        let mut term = Terminal::new_optimistic(80);
        term.color_depth = ColorDepth::TrueColor;
        assert!(
            bar.render(&term).contains("\x1b[38;2;10;200;40m"),
            "truecolor terminal should get a 24-bit fill color"
        );

        term.color_depth = ColorDepth::Basic;
        let basic = bar.render(&term);
        assert!(
            !basic.contains("\x1b[38;2;") && basic.contains("\x1b[32m"),
            "16-color terminal should degrade to the basic fallback: {basic:?}"
        );

        term.color_depth = ColorDepth::None;
        let none = bar.render(&term);
        assert!(
            !none.contains('\x1b'),
            "a terminal with no color support must emit no color SGR: {none:?}"
        );
    }

    #[test]
    fn progress_render_tree_node_carries_slot_colors() {
        use renderable::color::BasicColor;
        let bar = Progress::new(0.5)
            .with_filled_color(Color::BasicColor(BasicColor::Green));
        let node = bar.render_tree_node().unwrap();
        let hints = node.attrs.progress_hints().expect("progress hints");
        assert_eq!(
            hints.filled_color,
            Some(Color::BasicColor(BasicColor::Green))
        );
        assert_eq!(hints.empty_color, None);
    }

    #[test]
    fn progress_render_tree_node_carries_layout_when_margins_set() {
        use crate::utils::layout::{Length, Margin};
        let mut bar = Progress::new(0.5);
        bar.layout_mut().margin = Margin::x(Length::ch(2));
        let node = bar.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }
}
