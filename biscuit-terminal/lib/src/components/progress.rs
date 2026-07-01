use crate::{
    components::renderable::{BrowserRenderable, TerminalRenderable},
    discovery::detection::ColorDepth,
    render_tree::style::color_sgr,
    render_tree::{TerminalRenderOptions, render_terminal_node},
    terminal::Terminal,
    utils::layout::Layout,
};
use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::color::Color;
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{ProgressHints, RenderNode, RenderStrictness, TreeRenderable};
use renderable::style::Style as RStyle;
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
///
/// ## Layout & Style Contract
///
/// `Progress` is an internal-layout component (spec C2). It projects to a
/// [`Paragraph`](renderable::tree::NodeKind::Paragraph) carrying
/// [`ProgressHints`] and the configured [`Layout`]; the shared render-tree
/// fold resolves the outer box, and the bar renders inside it:
///
/// - [`Width::Auto`] (default) and [`Width::Fixed`] resolve the outer box
///   (margin / alignment / width-cap); the bar itself renders at its explicit
///   [`bar_width`](Self::with_bar_width) (default 20). [`Width::FitContent`]
///   hugs the bar's natural width (`label + brackets + bar_width +
///   percentage`) rather than padding to the available width.
/// - **Slack sink** (spec D2): the bar track. When the outer box is too
///   narrow for the natural `label + brackets + bar_width + percentage`
///   width, the rendered output clamps inside the box; the label, brackets,
///   and percentage are fixed-width, so the bar is the conceptual slack
///   absorber. When the box is wider than the natural width, alignment
///   positions the bar within the available area.
/// - A fractional `Fixed(50%)` is resolved exactly once by the fold; the bar
///   still renders at its full `bar_width` and never re-resolves the raw
///   percentage (the `Fixed(50%) → 25%` double-application bug).
/// - The projected [`ProgressHints`] round-trip carries the [`Layout`] onto
///   the paragraph node (C4), so the box contract survives a second render
///   pass.
///
/// [`Width::Auto`]: renderable::layout::Width::Auto
/// [`Width::Fixed`]: renderable::layout::Width::Fixed
/// [`Width::FitContent`]: renderable::layout::Width::FitContent
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
    /// Caller-supplied block appearance (color/background/emphasis/border)
    /// overlaid onto the projected node so both render paths carry it.
    block_style: RStyle,
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
            block_style: RStyle::default(),
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

    /// Builds the canonical [`NodeKind::Paragraph`] tree node for this progress
    /// bar, carrying [`ProgressHints`] on its `attrs`.
    ///
    /// This is the **single private projection helper**. Both
    /// [`TreeRenderable::render_tree`] and the legacy
    /// [`TerminalRenderable::render_tree_node`] hook delegate to it, so the
    /// terminal compatibility surface cannot drift away from the canonical
    /// tree-renderable producer.
    ///
    /// The paragraph's visible text is `"{label} {percentage}%"` (or just
    /// `"{percentage}%"` when there is no label) so renderers without progress
    /// hint support degrade to readable plain text. Renderers that recognize
    /// the hints reconstruct the full bar.
    ///
    /// A non-default [`Layout`] is recorded on the node's `attrs` so renderers
    /// honor the bar's margins, alignment, and width.
    ///
    /// [`NodeKind::Paragraph`]: renderable::tree::NodeKind::Paragraph
    fn to_render_node(&self) -> RenderNode {
        // `self.value` is clamped to `0.0..=1.0` by `Progress::new`, so the
        // product is in `0.0..=100.0` and the `as u32` cast is lossless.
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
        crate::components::renderable::overlay_style_onto_node(&mut node, &self.block_style);
        node
    }

    /// Renders the progress bar through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl to route Terminal output
    /// through the same tree the Browser and Markdown paths consume.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// string: the [`TerminalRenderable::render`] trait is infallible by
    /// contract, and surfacing a `[render-tree error: …]` sentinel as in-band
    /// terminal text would pollute user output.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_node();
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Progress",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
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
    /// Renders to a terminal string at an explicit width.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`]
    /// so terminal output matches the Browser and Markdown paths for the same
    /// component.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        self.render_via_tree(&term)
    }

    /// Renders to the supplied terminal.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`].
    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn style(&self) -> RStyle {
        self.block_style.clone()
    }

    fn style_mut(&mut self) -> Option<&mut RStyle> {
        Some(&mut self.block_style)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Projects this progress bar into a [`NodeKind::Paragraph`] render-tree
    /// node carrying [`ProgressHints`].
    ///
    /// Delegates to the single private projection helper
    /// [`Self::to_render_node`], shared with [`TreeRenderable::render_tree`] so
    /// the terminal compatibility hook and the canonical tree producer cannot
    /// drift.
    ///
    /// [`NodeKind::Paragraph`]: renderable::tree::NodeKind::Paragraph
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_node())
    }
}

impl TreeRenderable for Progress {
    /// Projects the progress bar into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`Progress::to_render_node`] so this canonical entry point and the
    /// terminal-compatibility [`TerminalRenderable::render_tree_node`] hook
    /// share one source of truth.
    ///
    /// The projected tree is a [`NodeKind::Paragraph`] carrying
    /// [`ProgressHints`] on its `attrs`. Renderers that recognize the hints
    /// reconstruct the bar; renderers that do not still produce readable
    /// `"{label} {percentage}%"` text.
    ///
    /// [`NodeKind::Paragraph`]: renderable::tree::NodeKind::Paragraph
    fn render_tree(&self) -> RenderNode {
        self.to_render_node()
    }
}

impl MarkdownRenderable for Progress {
    /// Renders the progress bar as portable Markdown via the canonical render
    /// tree.
    ///
    /// Portable Markdown has no native progress widget or color model, so the
    /// output is the paragraph fallback text — `"{label} {percentage}%"` or
    /// just `"{percentage}%"`. Glyphs, colors, bracket presentation, and
    /// layout are intentionally dropped because no portable CommonMark form
    /// preserves them.
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        match render_markdown_node(&node, &MarkdownRenderOptions::default()) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Progress",
                    dialect = "Markdown",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders the progress bar as MarkdownPlus via the canonical render tree.
    ///
    /// MarkdownPlus allows inline HTML, so the MarkdownPlus dialect of the
    /// tree renderer emits the same semantic CSS progress bar shape used by
    /// the browser renderer (RT-PROGRESS-002). Color slots are preserved as
    /// inline CSS and non-default glyphs survive in `data-*` attributes.
    fn render_markdown_plus(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Progress",
                    dialect = "MarkdownPlus",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl BrowserRenderable for Progress {
    /// Renders the progress bar as an HTML fragment via the canonical render
    /// tree.
    ///
    /// The browser tree renderer (RT-PROGRESS-001) emits semantic
    /// `role="progressbar"` HTML with the documented `progress` /
    /// `progress-label` / `progress-track` / `progress-filled` /
    /// `progress-percentage` classes, ARIA value attributes, color slots
    /// lowered to inline CSS `background-color`, and non-default glyphs
    /// preserved in `data-*` attributes.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// [`BrowserFragment`]: the [`BrowserRenderable`] contract is infallible,
    /// and surfacing a `[render-tree error: …]` sentinel as in-band HTML would
    /// pollute the rendered page. This mirrors the Terminal path's behavior.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Progress",
                    error = %error,
                    "render_browser_node failed; emitting empty fragment"
                );
                BrowserFragment::new()
                    .define_as_text_fragment(String::new())
                    .finalize()
            }
        }
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
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
        assert!(
            output.contains("\x1b[32m"),
            "filled color SGR missing: {output:?}"
        );
        assert!(
            output.contains("\x1b[36m"),
            "bracket color SGR missing: {output:?}"
        );
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
        let bar = Progress::new(0.5).with_filled_color(Color::BasicColor(BasicColor::Green));
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
        use crate::utils::layout::{Length, Edges};
        let mut bar = Progress::new(0.5);
        bar.layout_mut().margin = Edges::x(Length::ch(2));
        let node = bar.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }
}
