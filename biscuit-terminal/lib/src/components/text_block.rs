use std::any::Any;

use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::layout::TargetValue;
use renderable::markdown::MarkdownRenderable;
use renderable::style::{PerMode, Style as RStyle, TextEmphasis, UnderlineStyle};
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

use crate::{
    components::renderable::{BrowserRenderable, TerminalRenderable},
    render_tree::{TerminalRenderOptions, render_terminal_node},
    terminal::Terminal,
    utils::{
        color::Color,
        layout::Layout,
        styling::{FontWeight, UnderliningRequest},
    },
};

/// A uniformly styled block of text for terminal output.
///
/// TextBlock provides a convenient way to apply consistent styling (colors,
/// font weights, italic, underline, etc.) to a block of text. It handles
/// the ANSI escape sequences required for terminal rendering and supports
/// both foreground and background colors.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::text_block::TextBlock;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
/// use biscuit_terminal::utils::color::{BasicColor, Color};
///
/// // Basic usage - create styled text
/// let block = TextBlock::new("Hello, World!");
///
/// // Builder pattern for styling
/// let mut styled = TextBlock::new("Important message");
/// styled
///     .using_bold_text()
///     .with_foreground_color(Color::BasicColor(BasicColor::Red))
///     .with_background_color(Color::BasicColor(BasicColor::BrightBlack));
///
/// // Render to string with ANSI escape codes
/// let output = styled.render_optimistic(Some(80));
/// assert!(output.contains("\x1b[1m")); // bold
/// ```
///
/// ## Supported Styles
///
/// - **Font weight**: Normal, Bold, Dim
/// - **Colors**: Foreground and background colors (Basic, RGB, or Web colors)
/// - **Italic**: Italic text styling
/// - **Strikethrough**: Strikethrough text
/// - **Blink**: Blinking text (rarely supported)
/// - **Underline**: Single, double, or curl underlining
///
/// ## Notes
///
/// Previously the bespoke terminal renderer only applied italic and
/// `FontWeight`, leaving the stored foreground/background color, underline,
/// strikethrough, and blink fields inert. The render-tree path activates
/// every stored field — when rendering through the canonical tree, all
/// stored style fields now reach the terminal, Browser, and (for color slots
/// only — Markdown ignores `Style` by contract) MarkdownPlus targets.
///
/// ## Layout & Style Contract
///
/// `TextBlock` is a block component that routes through the shared render-tree
/// fold (spec C1). All applicable `Layout` properties (`margin`, `padding`,
/// `width`, `max_width`, `alignment`, `word_wrap`) and `Style` properties
/// (`color`, `background`, `emphasis`, `border`) are honored on Terminal and
/// Browser; Markdown degrades layout/appearance attrs by Decision D1 and
/// preserves the plain text content.
#[derive(Debug)]
pub struct TextBlock {
    content: String,
    font_weight: FontWeight,
    /// optionally specify the foreground/text color
    fg_color: Option<Color>,
    /// optionally specify the background color
    bg_color: Option<Color>,
    italic: bool,
    strikethrough: bool,
    blink: bool,
    underline: UnderliningRequest,
    layout: Layout,
    /// Caller-supplied block appearance overlaid onto [`build_style`] so both
    /// render paths carry it. [`build_style`] supplies the field-derived
    /// defaults; this slot holds the override set via
    /// [`TerminalRenderable::with_style`].
    style: RStyle,
}

impl Default for TextBlock {
    fn default() -> TextBlock {
        TextBlock {
            content: "".to_string(),
            font_weight: FontWeight::Normal,
            fg_color: None,
            bg_color: None,
            italic: false,
            strikethrough: false,
            blink: false,
            underline: UnderliningRequest::None,
            layout: Layout::default(),
            style: RStyle::default(),
        }
    }
}

impl TextBlock {
    pub fn new<T: Into<String>>(content: T) -> Self {
        TextBlock {
            content: content.into(),
            ..Default::default()
        }
    }

    /// Enable italics styling.
    pub fn using_italics(&mut self) -> &mut Self {
        self.italic = true;
        self
    }

    /// Enable bold text styling.
    pub fn using_bold_text(&mut self) -> &mut Self {
        self.font_weight = FontWeight::Bold;
        self
    }

    /// Enable strikethrough styling.
    pub fn use_strikethrough_on_content(&mut self) -> &mut Self {
        self.strikethrough = true;
        self
    }

    /// Enable dim text styling.
    pub fn using_dim_text(&mut self) -> &mut Self {
        self.font_weight = FontWeight::Dim;
        self
    }

    /// Set the foreground (text) color.
    pub fn with_foreground_color(&mut self, color: Color) -> &mut Self {
        self.fg_color = Some(color);
        self
    }

    /// Set the background color.
    pub fn with_background_color(&mut self, color: Color) -> &mut Self {
        self.bg_color = Some(color);
        self
    }

    /// Enable blinking text.
    pub fn make_content_blink(&mut self) -> &mut Self {
        self.blink = true;
        self
    }

    /// Set the underline style.
    pub fn with_underline(&mut self, under: UnderliningRequest) -> &mut Self {
        self.underline = under;
        self
    }

    /// Builds the canonical [`Style`](renderable::style::Style) from the
    /// component's stored fields.
    ///
    /// All stored style fields are mapped:
    ///
    /// - `font_weight` → `emphasis.bold` / `emphasis.dim`
    /// - `fg_color` → `color`
    /// - `bg_color` → `background`
    /// - `italic` → `emphasis.italic`
    /// - `strikethrough` → `emphasis.strikethrough`
    /// - `blink` → `emphasis.blink`
    /// - `underline` → `emphasis.underline` (shape only; underline color
    ///   has no `Style` slot today and is intentionally dropped — see the
    ///   spec's "Follow-up Clarifications" section).
    fn build_style(&self) -> RStyle {
        let emphasis = TextEmphasis {
            bold: matches!(self.font_weight, FontWeight::Bold),
            dim: matches!(self.font_weight, FontWeight::Dim),
            italic: self.italic,
            strikethrough: self.strikethrough,
            blink: self.blink,
            // `TextBlock` has no inverse-video control.
            inverse: false,
            underline: underline_style_from_request(&self.underline),
        };
        let color = self
            .fg_color
            .map(|c| TargetValue::universal(PerMode::universal(c)));
        let background = self
            .bg_color
            .map(|c| TargetValue::universal(PerMode::universal(c)));
        RStyle {
            color,
            background,
            emphasis,
            border: None,
        }
    }

    /// Single private projection helper.
    ///
    /// Returns a `NodeKind::Paragraph` with one `NodeKind::Text` child, with
    /// `Style` and non-default `Layout` recorded on the node's `NodeAttrs`.
    /// Both [`TreeRenderable::render_tree`] and the legacy
    /// [`TerminalRenderable::render_tree_node`] hook delegate to this so the
    /// terminal compatibility surface cannot drift from the canonical
    /// producer.
    fn to_render_node(&self) -> RenderNode {
        let mut node = RenderNode::paragraph(vec![RenderNode::text(&self.content)]);
        let style = self.style.overlay_onto(&self.build_style());
        if !style.is_empty() {
            node.attrs.set_style(&style);
        }
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        node
    }

    /// Renders this `TextBlock` through the canonical render tree.
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
                    component = "TextBlock",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

/// Maps the bespoke [`UnderliningRequest`] to a render-tree
/// [`UnderlineStyle`].
///
/// The optional underline color carried by [`UnderliningRequest`] is dropped
/// — [`UnderlineStyle`] records shape only. See the TextBlock spec's
/// "Follow-up Clarifications" for the rationale: the existing bespoke path
/// does not render underline at all, so preserving underline color is not a
/// parity requirement, and a future colored-underline feature should be
/// renderer-wide rather than `TextBlock`-only.
fn underline_style_from_request(req: &UnderliningRequest) -> Option<UnderlineStyle> {
    match req {
        UnderliningRequest::None => None,
        UnderliningRequest::Straight(_) => Some(UnderlineStyle::Straight),
        UnderliningRequest::Double(_) => Some(UnderlineStyle::Double),
        UnderliningRequest::Dotted(_) => Some(UnderlineStyle::Dotted),
        UnderliningRequest::Curly(_) => Some(UnderlineStyle::Curly),
        UnderliningRequest::Dashed(_) => Some(UnderlineStyle::Dashed),
    }
}

impl TerminalRenderable for TextBlock {
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn style(&self) -> RStyle {
        self.style.overlay_onto(&self.build_style())
    }

    fn style_mut(&mut self) -> Option<&mut RStyle> {
        Some(&mut self.style)
    }

    /// Projects this `TextBlock` into a `NodeKind::Paragraph` render-tree
    /// node.
    ///
    /// Delegates to the single private projection helper
    /// [`Self::to_render_node`], shared with [`TreeRenderable::render_tree`]
    /// so the terminal compatibility hook and the canonical tree producer
    /// cannot drift.
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_node())
    }
}

impl TreeRenderable for TextBlock {
    /// Projects the text block into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`TextBlock::to_render_node`] so this canonical entry point and the
    /// terminal-compatibility [`TerminalRenderable::render_tree_node`] hook
    /// share one source of truth.
    fn render_tree(&self) -> RenderNode {
        self.to_render_node()
    }
}

impl MarkdownRenderable for TextBlock {
    /// Renders the text block as portable Markdown via the canonical render
    /// tree.
    ///
    /// The Markdown tree renderer ignores `Style` by contract (per
    /// `layout-and-style.md`), so styled text blocks emit plain text. Style
    /// preservation for MarkdownPlus is a renderer-wide feature deferred
    /// outside this migration (see spec RT-TEXTBLOCK-002, DENIED).
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        match render_markdown_node(&node, &MarkdownRenderOptions::default()) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "TextBlock",
                    dialect = "Markdown",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders the text block as MarkdownPlus via the canonical render tree.
    ///
    /// Currently identical to portable Markdown for `TextBlock` style: the
    /// canonical Markdown renderer does not lower `Style` on either dialect.
    /// See spec RT-TEXTBLOCK-002 (DENIED) — styled MarkdownPlus is deferred
    /// to a renderer-wide design.
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
                    component = "TextBlock",
                    dialect = "MarkdownPlus",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl BrowserRenderable for TextBlock {
    /// Renders the text block as an HTML fragment via the canonical render
    /// tree.
    ///
    /// The browser tree renderer lowers `Style` per RT-TEXTBLOCK-001:
    ///
    /// - `Style.color` / `Style.background` → CSS `color` / `background-color`
    /// - `TextEmphasis.bold` / `italic` / `strikethrough` → `<strong>` /
    ///   `<em>` / `<s>` semantic wrappers
    /// - Underline variants → `text-decoration: underline` (+ style)
    /// - Dim → `opacity: 0.6`
    /// - Blink → `text-decoration: blink`
    ///
    /// `Layout` is applied to the same `<p>` element via the existing browser
    /// layout lowering, sharing the `style` attribute without overwriting it.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// [`BrowserFragment`]: the [`BrowserRenderable`] contract is infallible.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "TextBlock",
                    error = %error,
                    "render_browser_node failed; emitting empty fragment"
                );
                BrowserFragment::new()
                    .define_as_text_fragment(String::new())
                    .finalize()
            }
        }
    }

    /// Wraps the HTML fragment in a complete [`HtmlPage`], optionally applying
    /// the supplied [`PageOptions`].
    ///
    /// ## Notes
    ///
    /// If the underlying tree render fails, [`Self::render_html_fragment`]
    /// returns an empty fragment (the [`BrowserRenderable`] contract is
    /// infallible). The resulting page may therefore have an empty body; the
    /// failure is surfaced via the `tracing::error!` log in
    /// [`Self::render_html_fragment`] rather than via a page-level error
    /// return.
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
    use crate::utils::color::BasicColor;

    #[test]
    fn render_optimistic_via_tree_for_plain_text() {
        let block = TextBlock::new("Hello");
        let out = block.render_optimistic(Some(80));
        // Plain text routes through the tree and emits the text verbatim.
        assert!(out.contains("Hello"));
    }

    #[test]
    fn bold_via_tree_emits_bold_sgr() {
        let mut block = TextBlock::new("Bold");
        block.using_bold_text();
        let out = block.render_optimistic(Some(80));
        assert!(out.contains("\x1b[1m"), "expected bold SGR in {out:?}");
    }

    #[test]
    fn fg_color_via_tree_is_now_active() {
        // Foreground color was previously stored but inert in the bespoke
        // path. After the IR flip it reaches the terminal.
        let mut block = TextBlock::new("Red");
        block.with_foreground_color(Color::BasicColor(BasicColor::Red));
        let out = block.render_optimistic(Some(80));
        assert!(
            out.contains("\x1b["),
            "expected an SGR sequence for fg color in {out:?}"
        );
    }

    #[test]
    fn underline_style_from_request_drops_color() {
        let s = underline_style_from_request(&UnderliningRequest::Straight(Some(
            Color::BasicColor(BasicColor::Red),
        )));
        assert_eq!(s, Some(UnderlineStyle::Straight));
        let n = underline_style_from_request(&UnderliningRequest::None);
        assert!(n.is_none());
    }

    #[test]
    fn tree_renderable_and_compatibility_hook_share_projection() {
        // The canonical `TreeRenderable::render_tree` entry point and the
        // terminal compatibility `TerminalRenderable::render_tree_node` hook
        // MUST share one private projection helper so they cannot drift.
        let mut block = TextBlock::new("X");
        block.using_bold_text();
        block.with_foreground_color(Color::BasicColor(BasicColor::Red));

        let canonical = <TextBlock as TreeRenderable>::render_tree(&block);
        let compat = block.render_tree_node().expect("tree node");
        assert_eq!(
            serde_json::to_value(&canonical).unwrap(),
            serde_json::to_value(&compat).unwrap(),
            "canonical and compatibility projections must serialize identically"
        );
    }
}
