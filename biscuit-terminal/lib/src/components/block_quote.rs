use std::any::Any;
use std::rc::Rc;

use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::layout::TargetValue;
use renderable::markdown::MarkdownRenderable;
use renderable::style::{Border, BorderSides, PaintColor, PerMode, Style};
use renderable::target::RenderTarget;
use renderable::tree::render::{MarkdownDialect, MarkdownRenderOptions, render_markdown_node};
use renderable::tree::{NodeKind, RenderNode, RenderStrictness, TreeRenderable};

use crate::{
    components::{
        prose::Prose,
        renderable::{BrowserRenderable, RenderableTerminalContent, TerminalRenderable},
    },
    render_tree::{BrowserTreeComponent, TerminalRenderOptions, render_terminal_node},
    terminal::Terminal,
    utils::{
        block_constraint::{split_lines, visible_width, wrap_lines},
        color::{Color, Tailwind},
        layout::{Layout, LayoutTerminalExt, Length},
    },
};

/// The default left-border prefix applied to each quoted line.
///
/// A custom prefix supplied through [`BlockQuote::with_border`] is the only
/// thing that diverts the [`TerminalRenderable`] impl off the render-tree
/// path onto the legacy compatibility renderer.
const DEFAULT_BORDER: &str = "│ ";

/// Wraps a plain [`Color`] as an opaque universal [`Style`] color value.
fn universal_color(color: Color) -> TargetValue<PerMode<PaintColor>> {
    TargetValue::universal(PerMode::universal(color))
}

/// Extracts the terminal [`Color`] from an optional `Style` color slot.
///
/// A universal value resolves directly; an adaptive value resolves to its
/// dark-mode branch — the migrated builder shims only ever store universal
/// values, so this is a lossless inverse for the compatibility path.
fn color_of(slot: &Option<TargetValue<PerMode<PaintColor>>>) -> Option<Color> {
    match slot.as_ref()?.resolve(RenderTarget::Terminal)? {
        PerMode::Universal(paint) => Some(paint.color),
        PerMode::Adaptive { dark, .. } => Some(dark.color),
    }
}

/// Builds a left-only [`Border`] carrying `color`.
fn left_border(color: Color) -> Border {
    Border {
        color: Some(universal_color(color)),
        sides: BorderSides::Sides {
            top: false,
            right: false,
            bottom: false,
            left: true,
        },
        ..Border::default()
    }
}

/// Renders quoted text with a distinctive left border.
///
/// A block quote displays text with a visual border on the left side,
/// typically used to highlight quoted content, testimonials, or
/// notable passages in a visually distinct manner.
///
/// ## Structure
///
/// - Each line is prefixed with `│ ` (U+2502 + space)
/// - Optional attribution can be added at the bottom preceded by `—`
/// - Content is automatically wrapped to fit the terminal width
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::block_quote::BlockQuote;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// // Simple block quote from a string
/// let quote = BlockQuote::from("The only way to do great work is to love what you do.");
/// let result = quote.render_optimistic(Some(60));
/// assert!(result.contains("The only way"));
/// assert!(result.contains("│")); // colored border present
///
/// // Block quote with attribution
/// let quote = BlockQuote::new(
///     "To be, or not to be, that is the question.".into(),
///     Some("William Shakespeare")
/// );
/// let result = quote.render_optimistic(None);
/// assert!(result.contains("To be, or not to be"));
/// assert!(result.contains("— William Shakespeare"));
///
/// // Styled block quote with custom colors
/// use biscuit_terminal::utils::color::{Color, Tailwind};
/// let quote = BlockQuote::from("Important quote")
///     .with_text_color(Color::Tailwind(Tailwind::White))
///     .with_bg_color(Color::Tailwind(Tailwind::Gray800))
///     .with_left_block_color(Color::Tailwind(Tailwind::Blue400));
/// let result = quote.render_optimistic(None);
/// assert!(result.contains("Important quote"));
///
/// // Block quote from Prose component (rich text)
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::RenderableTerminalContent;
/// let prose = Prose::new("This is <b>bold</b> and <i>italic</i> text.");
/// let quote = BlockQuote::new(
///     RenderableTerminalContent::from(prose),
///     Some("Anonymous")
/// );
/// let result = quote.render_optimistic(None);
/// assert!(result.contains("│"));
/// assert!(result.contains("— Anonymous"));
/// ```
///
/// ## Layout & Style Contract
///
/// `BlockQuote` is a block component that routes through the shared render-tree
/// fold (spec C1). All applicable `Layout` properties (`margin`, `padding`,
/// `width`, `max_width`, `alignment`, `word_wrap`) and `Style` properties
/// (`color`, `background`, `emphasis`, `border`) are honored on Terminal and
/// Browser; Markdown degrades layout/appearance attrs by Decision D1 and
/// preserves only structural output.
///
/// The default left border (`│ `) is represented as a typed
/// [`Border`](renderable::style::Border) on the projected node. A custom
/// border prefix supplied through [`BlockQuote::with_border`] routes the
/// terminal [`TerminalRenderable`] impl through a bespoke compatibility
/// renderer because the render tree cannot express an arbitrary target-specific
/// prefix string.
#[derive(Debug, Clone)]
pub struct BlockQuote {
    /// the content being wrapped in the block quote
    content: RenderableTerminalContent,

    /// if the quote is being attributed to someone
    /// you can add that and it will be placed
    attribution: Option<String>,

    /// Declared appearance — the migrated home of the former `text_color`,
    /// `bg_color`, and `left_block_color` fields. Foreground color rides on
    /// [`Style::color`], background on [`Style::background`], and the left
    /// border color on [`Style::border`].
    style: Style,
    /// The border string prefixed to each line (default: `"│ "`).
    border: String,
    layout: Layout,
}

impl Default for BlockQuote {
    fn default() -> Self {
        use crate::utils::wrap_policy::WordWrap;
        BlockQuote {
            content: RenderableTerminalContent::String("".to_string()),
            attribution: None,
            style: Style {
                border: Some(left_border(Color::Tailwind(Tailwind::Gray500))),
                ..Style::default()
            },
            border: DEFAULT_BORDER.to_string(),
            layout: Layout {
                word_wrap: WordWrap::WrapProse(Some(8), None),
                ..Layout::default()
            },
        }
    }
}

impl From<String> for BlockQuote {
    fn from(value: String) -> Self {
        BlockQuote::new(RenderableTerminalContent::String(value), None::<String>)
    }
}

impl From<&String> for BlockQuote {
    fn from(value: &String) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::String(value.into()),
            None::<String>,
        )
    }
}

impl From<&str> for BlockQuote {
    fn from(value: &str) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::String(value.into()),
            None::<String>,
        )
    }
}

impl From<Prose> for BlockQuote {
    fn from(value: Prose) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new(value)),
            None::<String>,
        )
    }
}

impl From<&Prose> for BlockQuote {
    fn from(value: &Prose) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new((*value).clone())),
            None::<String>,
        )
    }
}

impl BlockQuote {
    /// Create a block quote by passing in the content and _optionally_ an attribution.
    pub fn new<U: Into<String>>(
        content: RenderableTerminalContent,
        attribution: Option<U>,
    ) -> Self {
        Self {
            content,
            attribution: attribution.map(|attribution| attribution.into()),
            ..BlockQuote::default()
        }
    }

    /// Set the text color.
    ///
    /// Compatibility shim: the color is stored on the declared [`Style`]'s
    /// [`color`](Style::color) slot.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.style.color = Some(universal_color(color));
        self
    }

    /// Set the background color.
    ///
    /// Compatibility shim: the color is stored on the declared [`Style`]'s
    /// [`background`](Style::background) slot.
    pub fn with_bg_color(mut self, color: Color) -> Self {
        self.style.background = Some(universal_color(color));
        self
    }

    /// Set the left block/border color.
    ///
    /// Compatibility shim: the color is stored on the declared [`Style`]'s
    /// left [`border`](Style::border) slot.
    pub fn with_left_block_color(mut self, color: Color) -> Self {
        match &mut self.style.border {
            Some(border) => border.color = Some(universal_color(color)),
            None => self.style.border = Some(left_border(color)),
        }
        self
    }

    /// The declared text color, if any.
    pub fn text_color(&self) -> Option<Color> {
        color_of(&self.style.color)
    }

    /// The declared background color, if any.
    pub fn bg_color(&self) -> Option<Color> {
        color_of(&self.style.background)
    }

    /// The declared left block/border color, if any.
    pub fn left_block_color(&self) -> Option<Color> {
        self.style
            .border
            .as_ref()
            .and_then(|border| color_of(&border.color))
    }

    /// Set a custom border string (default: `"│ "`).
    ///
    /// The visible width of the border is subtracted from the available
    /// content width, so wider borders reduce the text area accordingly.
    pub fn with_border(mut self, border: impl Into<String>) -> Self {
        self.border = border.into();
        self
    }

    /// Render the block quote content with a left border.
    ///
    /// The border width is determined by the `border` field and the
    /// child content width is reduced accordingly.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let border_width = visible_width(&self.border);
        let child_width = term_width.saturating_sub(border_width);

        // Render child content with constrained width
        let content: String = match &self.content {
            RenderableTerminalContent::String(s) => {
                // Wrap plain strings to child_width using the layout's word wrap strategy
                let lines = split_lines(s);
                let wrapped = wrap_lines(lines, &self.layout.word_wrap, child_width);
                wrapped.join("\n")
            }
            RenderableTerminalContent::Component(component) => {
                // Use render() with the constrained child width so nested
                // components respect the block quote's border
                term.map_or_else(
                    || component.render_optimistic(Some(child_width)),
                    |t| component.render_in_width(t, child_width),
                )
            }
        };

        // Build the colored border string
        let colored_border = self.colorize_border(&self.border);
        // Attribution border uses the first visible char only (no trailing space)
        let border_char = self.border.trim_end();
        let colored_border_char = self.colorize_border(border_char);

        let mut result = String::new();

        // Split content into lines and prefix each with the quote border
        for line in content.lines() {
            result.push_str(&colored_border);
            result.push_str(line);
            result.push('\n');
        }

        // Add attribution if present (with blank line separator)
        if let Some(ref attribution) = self.attribution {
            result.push_str(&colored_border_char);
            result.push('\n'); // blank line
            result.push_str(&colored_border_char);
            result.push_str(" — ");
            result.push_str(attribution);
            result.push('\n');
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Apply the declared left block color to a border string segment.
    fn colorize_border(&self, border: &str) -> String {
        match self.left_block_color() {
            Some(color) => match color.to_rgb() {
                Some((r, g, b)) => {
                    format!("\x1b[38;2;{r};{g};{b}m{border}\x1b[39m")
                }
                None => border.to_string(),
            },
            None => border.to_string(),
        }
    }
}

impl BlockQuote {
    /// Whether this quote carries the canonical left-border prefix.
    ///
    /// The render-tree path can only express the default `│ ` border via a
    /// typed [`Style::Border`]; an arbitrary terminal prefix string is a
    /// compatibility-only knob. When the prefix is the default, the
    /// [`TerminalRenderable`] impl routes through the tree renderer; otherwise
    /// it falls back to the bespoke [`render_content`](Self::render_content)
    /// path that honors the custom prefix verbatim.
    //
    // Arbitrary prefixes set via `BlockQuote::with_border` stay on the
    // internal bespoke escape hatch; there is no public `render_bespoke`
    // hook on `BlockQuote` — the only way to reach that path is through
    // `TerminalRenderable::render` when this predicate returns `false`.
    fn has_default_border(&self) -> bool {
        self.border == DEFAULT_BORDER
    }

    /// Renders the block quote through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl when the component carries the
    /// default left border. The legacy bespoke path is retained only as a
    /// compatibility fallback for [`BlockQuote::with_border`].
    ///
    /// See the BlockQuote spec (`RT-BQ-001`) and the
    /// `approved-render-tree-functionality` notes for the render-tree route's
    /// scope versus the bespoke `with_border` fallback.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let root = RenderNode::root(vec![self.render_tree()]);
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&root, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                // Surface the failure through `tracing` so observability tools
                // see it, then fall back to an empty string. Returning an
                // error sentinel as in-band terminal text is a footgun — it
                // pollutes user output with diagnostic strings — and the
                // `TerminalRenderable` trait contract is infallible, so we
                // must not panic either.
                tracing::error!(
                    component = "BlockQuote",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl TerminalRenderable for BlockQuote {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        if self.has_default_border() {
            // Tree path — uses the typed `Style::Border` lowered by the
            // terminal tree renderer.
            let term = match term_width {
                Some(width) => Terminal::new_optimistic(width),
                None => Terminal::default(),
            };
            return self.render_via_tree(&term);
        }
        // Compatibility fallback for `with_border()` custom prefixes.
        let width = term_width.unwrap_or(80);
        let available = self.layout.available_width(width);
        let content = self.render_content(None, available);
        // BlockQuote has a `┃ ` border on every line — the border must form a
        // straight vertical bar, so the block must be aligned as a unit.
        self.layout.apply_block_layout(&content, width)
    }

    fn render(&self, term: &Terminal) -> String {
        if self.has_default_border() {
            return self.render_via_tree(term);
        }
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_block_layout(&content, width)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn style(&self) -> Style {
        self.style.clone()
    }

    fn style_mut(&mut self) -> Option<&mut Style> {
        Some(&mut self.style)
    }

    fn is_block_level(&self) -> bool {
        true
    }

    /// Exposes the tree projection through the canonical
    /// [`TerminalRenderable::render_tree_node`] hook so cross-target adapters
    /// (and nested [`RenderableTerminalContent::Component`] projection) consume
    /// `BlockQuote` structurally instead of degrading to ANSI-stripped text.
    ///
    /// Delegates to [`TreeRenderable::render_tree`] so both public entry
    /// points share one source of truth.
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(<Self as TreeRenderable>::render_tree(self))
    }
}

impl MarkdownRenderable for BlockQuote {
    /// Renders the block quote as portable Markdown via the canonical render
    /// tree.
    ///
    /// The tree projection ignores any custom terminal prefix supplied through
    /// [`BlockQuote::with_border`] and emits standard CommonMark block-quote
    /// syntax (`> text` per line). Styling (text color, background, border
    /// color) is intentionally dropped — Markdown has no semantic
    /// representation for visual appearance.
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_default()
    }

    /// MarkdownPlus output is identical to portable Markdown for a block
    /// quote.
    ///
    /// The block quote's tree projection is plain text with attribution as a
    /// trailing paragraph; the only divergence MarkdownPlus could offer would
    /// be inline HTML for style preservation, but `render_tree()`
    /// intentionally flattens `Prose` styling, so there is nothing extra to
    /// emit in the richer dialect.
    fn render_markdown_plus(&self) -> String {
        let node = self.render_tree();
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        render_markdown_node(&node, &opts)
            .map(|r| r.output)
            .unwrap_or_default()
    }
}

impl BrowserRenderable for BlockQuote {
    /// Renders the block quote as an HTML fragment via the canonical render
    /// tree.
    ///
    /// Delegates through [`BrowserTreeComponent`], which folds the projected
    /// `NodeKind::BlockQuote` into a `<blockquote>` element containing the
    /// rendered child paragraphs. Style lowering for the browser is not yet
    /// wired (see `layout-and-style.md` §6), so the element renders with
    /// browser-default styling for now.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        BrowserTreeComponent::new(self.clone()).render_html_fragment()
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        BrowserTreeComponent::new(self.clone()).render_html_page(page)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BlockQuote {
    /// Builds the inline node sequence for inline quote content in the
    /// canonical render tree.
    ///
    /// Used only when the content is inherently inline — a plain `String` or a
    /// [`Prose`] component. The caller
    /// ([`Self::project_block_children`]) folds the returned sequence into
    /// block-level children: a plain `String` is one `Text` node that becomes
    /// a single `Paragraph`, while a `Prose` body that carries a fenced code
    /// block yields a `Code` node that must stay a block-level sibling rather
    /// than nesting inside a `Paragraph`. Non-`Prose` block-level components
    /// project structurally through [`Self::project_block_children`] instead.
    ///
    /// Delegates to the shared
    /// [`project_renderable_content`](crate::render_tree::projection::project_renderable_content)
    /// helper in
    /// [`ProjectionMode::InlineOnly`](crate::render_tree::projection::ProjectionMode::InlineOnly)
    /// so [`Prose`] inline runs survive as `Strong` / `Emphasis` / styled
    /// `Span` nodes that the terminal tree renderer lowers back to SGR.
    fn paragraph_children(&self) -> Vec<RenderNode> {
        use crate::render_tree::projection::{ProjectionMode, project_renderable_content};
        project_renderable_content(&self.content, ProjectionMode::InlineOnly)
    }

    /// Builds the top-level children for the quote's projected
    /// [`NodeKind::BlockQuote`].
    ///
    /// `String` content and [`Prose`] components are inline — their projected
    /// node sequence is folded into block-level children via
    /// [`fold_prose_nodes_into_blocks`](crate::render_tree::projection::fold_prose_nodes_into_blocks),
    /// so contiguous inline runs become a single `Paragraph` and any fenced
    /// code block a `Prose` body carries stays a block-level sibling. Any
    /// other component projects structurally via
    /// [`ProjectionMode::Structural`](crate::render_tree::projection::ProjectionMode::Structural):
    /// its `render_tree_node` output (or the structural fallback) becomes a
    /// direct block-level child of the `BlockQuote`. This closes the Stage 3
    /// projection gap where nested `Section`, `List`, and `Table` components
    /// inside a quote previously flattened to ANSI-stripped text.
    ///
    /// When structural projection yields inline-only nodes (the
    /// `RenderStrictness::Warn` fallback emits a single `Text`), they are
    /// wrapped in a `Paragraph` so the `BlockQuote` contract still has
    /// block-level children.
    fn project_block_children(&self) -> Vec<RenderNode> {
        use crate::render_tree::projection::{
            ProjectionMode, fold_prose_nodes_into_blocks, project_renderable_content,
        };

        // Inline content (String, Prose) folds into block-level children:
        // contiguous inline runs become a single `Paragraph` and any fenced
        // code block a `Prose` body carries stays a block-level sibling.
        // Wrapping the whole sequence in one `Paragraph` would nest that
        // block-level `Code` inside phrasing content, which the render tree
        // rejects.
        let is_inline_content = match &self.content {
            RenderableTerminalContent::String(_) => true,
            RenderableTerminalContent::Component(component) => {
                component.as_any().downcast_ref::<Prose>().is_some()
            }
        };
        if is_inline_content {
            return fold_prose_nodes_into_blocks(self.paragraph_children());
        }

        let nodes = project_renderable_content(
            &self.content,
            ProjectionMode::Structural {
                terminal_hint: None,
            },
        );

        if nodes.iter().all(|n| is_inline_node_kind(&n.kind)) {
            vec![RenderNode::paragraph(nodes)]
        } else {
            nodes
        }
    }
}

/// Returns `true` if a [`NodeKind`] is phrasing-level (inline).
///
/// Mirrors the validator's `is_inline_kind` so [`BlockQuote::project_block_children`]
/// can decide whether to wrap a structural projection in a `Paragraph`. Kept
/// local because the validator's helper is not part of the public render-tree
/// API.
fn is_inline_node_kind(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Text { .. }
            | NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Span { .. }
            | NodeKind::InlineCode { .. }
            | NodeKind::Link { .. }
            | NodeKind::Image { .. }
            | NodeKind::FootnoteReference { .. }
            | NodeKind::SoftBreak
            | NodeKind::HardBreak
    )
}

impl TreeRenderable for BlockQuote {
    /// Projects the block quote into the canonical render tree as a
    /// [`NodeKind::BlockQuote`](renderable::tree::NodeKind::BlockQuote).
    ///
    /// `String` content and [`Prose`] components become a single `Paragraph`
    /// wrapping inline children: `String` becomes one `Text` node and `Prose`
    /// becomes structured inline nodes (`Strong` / `Emphasis` / styled
    /// `Span`) so its declared styling survives on the Terminal target.
    /// Non-`Prose` components project structurally via
    /// [`Self::project_block_children`] — nested `Section`, `List`, `Table`,
    /// etc. appear as direct block-level children of the projected
    /// `BlockQuote` instead of flattening to ANSI-stripped text.
    ///
    /// When an attribution is present it is appended as a final `Paragraph`
    /// containing a single `Text` of the form `— {attribution}`.
    ///
    /// A non-default [`Layout`] (margin, alignment, max-width, word wrap) is
    /// recorded on the root node's attributes so the tree renderers apply it,
    /// matching the bespoke `render` path.
    ///
    /// The declared [`Style`] — the left border plus any text and background
    /// color set through the builder shims — is recorded on the node so the
    /// terminal tree renderer lowers the same appearance the bespoke renderer
    /// produces. The Markdown and Browser renderers ignore the styling color
    /// slots (they have no semantic Markdown peer), so their output continues
    /// to match a plain quote.
    fn render_tree(&self) -> RenderNode {
        let mut children = self.project_block_children();

        if let Some(attribution) = &self.attribution {
            children.push(RenderNode::paragraph(vec![RenderNode::text(format!(
                "— {attribution}"
            ))]));
        }

        let mut node = RenderNode::block_quote(children);
        // The left border's inner gap is `Layout::padding` now — the terminal
        // tree renderer no longer inserts an implicit space inside the border —
        // so reserve one cell to reproduce the `│ ` gap. This is the
        // component's *default*: a caller that configured its own left padding
        // (through `layout_mut`) keeps it, so a larger / percentage / per-target
        // gap is not silently clobbered. The intrinsic word-wrap baseline is
        // applied by the renderer's block-quote branch, not read from the node,
        // so carrying the layout on the node is harmless.
        let mut node_layout = self.layout.clone();
        if node_layout.padding.left == TargetValue::universal(Length::Zero) {
            node_layout.padding.left = TargetValue::universal(Length::ch(1));
        }
        node.attrs.set_layout(&node_layout);
        if !self.style.is_empty() {
            node.attrs.set_style(&self.style);
        }
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strip ANSI escape sequences for test assertions.
    fn strip_ansi(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut in_escape = false;
        for ch in s.chars() {
            if in_escape {
                if ch.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else if ch == '\x1b' {
                in_escape = true;
            } else {
                result.push(ch);
            }
        }
        result
    }

    // =========================================================================
    // Basic Construction Tests
    // =========================================================================

    #[test]
    fn test_simple_block_quote_from_str() {
        let quote = BlockQuote::from("Hello world");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello world");
    }

    #[test]
    fn test_simple_block_quote_from_string() {
        let quote = BlockQuote::from(String::from("Hello world"));
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello world");
    }

    #[test]
    fn test_simple_block_quote_from_string_ref() {
        let content = String::from("Hello world");
        let quote = BlockQuote::from(&content);
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello world");
    }

    #[test]
    fn test_block_quote_new_with_renderable_content() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("Direct content"),
            None::<&str>,
        );
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Direct content");
    }

    #[test]
    fn render_tree_seeds_default_left_padding_gap() {
        // With no caller-configured padding the projection reserves the 1ch
        // gap that replaces the border's old implicit interior space.
        let quote = BlockQuote::from("Hello world");
        let node = quote.render_tree();
        let layout = node.attrs.layout_ref().expect("layout recorded");
        assert_eq!(
            layout.padding.left,
            TargetValue::universal(Length::ch(1)),
            "default left padding gap must be seeded"
        );
    }

    #[test]
    fn render_tree_preserves_caller_left_padding() {
        // A caller that deliberately set a wider left padding keeps it — the
        // default 1ch gap must not clobber an explicit configuration.
        let mut quote = BlockQuote::from("Hello world");
        quote.layout_mut().padding.left = TargetValue::universal(Length::ch(4));
        let node = quote.render_tree();
        let layout = node.attrs.layout_ref().expect("layout recorded");
        assert_eq!(
            layout.padding.left,
            TargetValue::universal(Length::ch(4)),
            "explicit caller left padding must survive projection"
        );
    }

    // =========================================================================
    // Multiline Content Tests
    // =========================================================================

    #[test]
    fn test_multiline_block_quote() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("Line 1\nLine 2"),
            None::<&str>,
        );
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Line 1\n│ Line 2");
    }

    #[test]
    fn test_multiline_block_quote_three_lines() {
        let quote = BlockQuote::from("First\nSecond\nThird");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ First\n│ Second\n│ Third");
    }

    #[test]
    fn test_block_quote_with_empty_lines() {
        let quote = BlockQuote::from("Before\n\nAfter");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Before\n│ \n│ After");
    }

    // =========================================================================
    // Attribution Tests
    // =========================================================================

    #[test]
    fn test_block_quote_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let result = quote.render_optimistic(None);
        // The render-tree path renders the attribution as a separate child
        // paragraph; the blank separator line therefore carries the bordered
        // space prefix `│ ` rather than the bare `│` that the bespoke renderer
        // emitted.
        assert_eq!(
            strip_ansi(&result),
            "│ To be or not to be\n│ \n│ — Shakespeare"
        );
    }

    #[test]
    fn test_block_quote_with_string_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("The unexamined life is not worth living"),
            Some(String::from("Socrates")),
        );
        let result = quote.render_optimistic(None);
        assert_eq!(
            strip_ansi(&result),
            "│ The unexamined life is not worth living\n│ \n│ — Socrates"
        );
    }

    #[test]
    fn test_multiline_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("Line one\nLine two"),
            Some("Author"),
        );
        let result = quote.render_optimistic(None);
        assert_eq!(
            strip_ansi(&result),
            "│ Line one\n│ Line two\n│ \n│ — Author"
        );
    }

    // =========================================================================
    // Builder Pattern (with_*) Tests
    // =========================================================================

    #[test]
    fn test_with_text_color() {
        let quote =
            BlockQuote::from("Colored text").with_text_color(Color::Tailwind(Tailwind::Blue500));
        assert!(quote.text_color().is_some());
    }

    #[test]
    fn test_with_bg_color() {
        let quote =
            BlockQuote::from("Background").with_bg_color(Color::Tailwind(Tailwind::Gray100));
        assert!(quote.bg_color().is_some());
    }

    #[test]
    fn test_with_left_block_color() {
        let quote = BlockQuote::from("Custom border")
            .with_left_block_color(Color::Tailwind(Tailwind::Red500));
        assert_eq!(
            quote.left_block_color(),
            Some(Color::Tailwind(Tailwind::Red500))
        );
    }

    #[test]
    fn test_builder_chain() {
        let quote = BlockQuote::from("Full styling")
            .with_text_color(Color::Tailwind(Tailwind::White))
            .with_bg_color(Color::Tailwind(Tailwind::Gray800))
            .with_left_block_color(Color::Tailwind(Tailwind::Green500));

        assert!(quote.text_color().is_some());
        assert!(quote.bg_color().is_some());
        assert!(quote.left_block_color().is_some());
    }

    // =========================================================================
    // Default Tests
    // =========================================================================

    #[test]
    fn test_default_has_gray_left_block() {
        let quote = BlockQuote::default();
        assert_eq!(
            quote.left_block_color(),
            Some(Color::Tailwind(Tailwind::Gray500))
        );
    }

    #[test]
    fn test_default_has_no_text_color() {
        let quote = BlockQuote::default();
        assert!(quote.text_color().is_none());
    }

    #[test]
    fn test_default_has_no_bg_color() {
        let quote = BlockQuote::default();
        assert!(quote.bg_color().is_none());
    }

    #[test]
    fn test_default_has_no_attribution() {
        let quote = BlockQuote::default();
        assert!(quote.attribution.is_none());
    }

    // =========================================================================
    // From<Prose> Tests
    // =========================================================================

    #[test]
    fn test_from_prose() {
        let prose = Prose::new("Prose content");
        let quote = BlockQuote::from(prose);
        let result = quote.render_optimistic(None);
        assert!(result.contains("Prose content"));
    }

    #[test]
    fn test_from_prose_ref() {
        let prose = Prose::new("Prose reference");
        let quote = BlockQuote::from(&prose);
        let result = quote.render_optimistic(None);
        assert!(result.contains("Prose reference"));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_content() {
        let quote = BlockQuote::from("");
        let result = quote.render_optimistic(None);
        // The render-tree path still emits a single bordered line for the
        // empty paragraph child. After ANSI stripping the visible bytes are
        // the border prefix `│ ` only.
        assert_eq!(strip_ansi(&result), "│ ");
    }

    #[test]
    fn test_single_character() {
        let quote = BlockQuote::from("X");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ X");
    }

    #[test]
    fn test_unicode_content() {
        let quote = BlockQuote::from("Hello 世界 🌍");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello 世界 🌍");
    }

    #[test]
    fn test_whitespace_only() {
        let quote = BlockQuote::from("   ");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│    ");
    }

    // =========================================================================
    // TerminalRenderable Trait Tests
    // =========================================================================

    #[test]
    fn test_fallback_render_same_as_render() {
        let quote = BlockQuote::from("Fallback test");
        let term = Terminal::default();
        let render_result = quote.render_optimistic(None);
        let fallback_result = quote.render(&term);
        assert_eq!(render_result, fallback_result);
    }

    #[test]
    fn test_clone() {
        let quote = BlockQuote::from("Clone me")
            .with_text_color(Color::Tailwind(Tailwind::Blue500))
            .with_left_block_color(Color::Tailwind(Tailwind::Red500));
        let cloned = quote.clone();

        assert_eq!(
            quote.render_optimistic(None),
            cloned.render_optimistic(None)
        );
        assert_eq!(quote.text_color(), cloned.text_color());
        assert_eq!(quote.left_block_color(), cloned.left_block_color());
    }

    #[test]
    fn test_debug() {
        let quote = BlockQuote::from("Debug test");
        let debug_str = format!("{:?}", quote);
        assert!(debug_str.contains("BlockQuote"));
    }

    // =========================================================================
    // RenderableTerminalContent Integration Tests
    // =========================================================================

    #[test]
    fn test_renderable_content_string_variant() {
        let content = RenderableTerminalContent::String("direct string".to_string());
        let quote = BlockQuote::new(content, None::<&str>);
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ direct string");
    }

    #[test]
    fn test_renderable_content_component_variant() {
        let prose = Prose::new("<b>bold content</b>");
        let content = RenderableTerminalContent::Component(Rc::new(prose));
        let quote = BlockQuote::new(content, None::<&str>);
        let result = quote.render_optimistic(None);
        // The prose renders its bold content, which should appear in the quote
        assert!(strip_ansi(&result).starts_with("│ "));
        assert!(result.contains("bold content"));
    }

    #[test]
    fn test_styled_prose_in_block_quote() {
        let prose = Prose::new("<red>error</red>: something went wrong");
        let quote = BlockQuote::from(prose);
        let result = quote.render_optimistic(None);
        // Should contain the border and the content
        assert!(strip_ansi(&result).contains("│ "));
        assert!(result.contains("error"));
        assert!(result.contains("something went wrong"));
    }

    #[test]
    fn test_multiline_prose_in_block_quote() {
        let prose = Prose::new("Line 1\nLine 2\nLine 3");
        let quote = BlockQuote::from(prose);
        let result = quote.render_optimistic(None);
        // Each line should have a border
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("│ "));
        assert!(lines[1].starts_with("│ "));
        assert!(lines[2].starts_with("│ "));
    }

    #[test]
    fn test_prose_with_attribution() {
        let prose = Prose::new("<i>In the beginning...</i>");
        let quote = BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new(prose)),
            Some("Genesis"),
        );
        let result = quote.render_optimistic(None);
        assert!(result.contains("In the beginning"));
        assert!(result.contains("— Genesis"));
    }

    // =========================================================================
    // Complex Scenario Tests
    // =========================================================================

    #[test]
    fn test_block_quote_preserves_internal_newlines() {
        let content = "First paragraph.\n\nSecond paragraph.";
        let quote = BlockQuote::from(content);
        let result = quote.render_optimistic(None);
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "│ First paragraph.");
        assert_eq!(lines[1], "│ ");
        assert_eq!(lines[2], "│ Second paragraph.");
    }

    #[test]
    fn test_block_quote_with_tabs() {
        let quote = BlockQuote::from("Code:\n\tindented");
        let result = quote.render_optimistic(None);
        let stripped = strip_ansi(&result);
        assert!(stripped.contains("│ Code:"));
        assert!(stripped.contains("│ \tindented"));
    }

    #[test]
    fn test_attribution_only() {
        // Attribution with empty-ish content
        let quote = BlockQuote::new(RenderableTerminalContent::from("Quote"), Some("Author"));
        let result = quote.render_optimistic(None);
        let stripped = strip_ansi(&result);
        assert!(stripped.contains("│ Quote"));
        assert!(stripped.contains("│ — Author"));
    }

    // =========================================================================
    // Word Wrap Tests
    // =========================================================================

    #[test]
    fn test_long_string_wraps_at_width() {
        // At width 40, child_width = 38 (minus "│ " border).
        // A long string should be split across lines, each prefixed with "│ "
        let long = "word ".repeat(20); // 100 chars
        let quote = BlockQuote::from(long.trim());
        let result = quote.render_optimistic(Some(40));
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert!(
            lines.len() > 1,
            "Expected wrapping but got single line: {result}"
        );
        assert!(lines.iter().all(|l| l.starts_with("│ ")));
    }

    #[test]
    fn test_embedded_newlines_preserved_with_wrapping() {
        // Simulates a YAML >- prompt with paragraph breaks
        let text = "Do a deep dive on JSON Schema:\n- what types? - how is validation done?\nFor any code examples always use Rust.";
        let quote = BlockQuote::from(text);
        let result = quote.render_optimistic(Some(80));
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        // Should have at least 3 border-prefixed lines (one per \n-separated segment)
        assert!(
            lines.len() >= 3,
            "Expected >=3 lines but got {}: {result}",
            lines.len()
        );
        assert!(
            lines
                .iter()
                .all(|l| l.starts_with("│ ") || l.starts_with("      │")),
            "All lines should have border prefix: {result}"
        );
    }

    #[test]
    fn test_short_string_does_not_wrap() {
        let quote = BlockQuote::from("short text");
        let result = quote.render_optimistic(Some(80));
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "│ short text");
    }

    // =========================================================================
    // TreeRenderable Tests
    // =========================================================================

    // `render_markdown_node`, `MarkdownRenderOptions`, and `TreeRenderable` are
    // already imported at the file level via `use super::*`.
    use renderable::tree::NodeKind;
    use renderable::tree::render::{BrowserRenderOptions, render_browser_node};

    #[test]
    fn test_render_tree_root_is_block_quote() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        assert!(matches!(node.kind, NodeKind::BlockQuote { .. }));
    }

    #[test]
    fn test_render_tree_contains_text() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        let json = serde_json::to_string(&node).expect("serialize");
        assert!(json.contains("Quoted text"));
    }

    #[test]
    fn test_render_tree_without_attribution_has_single_child() {
        let quote = BlockQuote::from("Just a quote");
        let node = quote.render_tree();
        assert_eq!(node.children().len(), 1);
    }

    #[test]
    fn test_render_tree_with_attribution_appends_child() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let node = quote.render_tree();
        assert_eq!(node.children().len(), 2);
        let json = serde_json::to_string(&node).expect("serialize");
        assert!(json.contains("— Shakespeare"));
    }

    #[test]
    fn test_render_tree_extracts_text_from_component() {
        let prose = Prose::new("<b>bold content</b>");
        let quote = BlockQuote::from(prose);
        let node = quote.render_tree();
        let json = serde_json::to_string(&node).expect("serialize");
        // Component text is extracted with ANSI escapes stripped.
        assert!(json.contains("bold content"));
    }

    #[test]
    fn test_render_tree_to_markdown() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default())
            .expect("markdown render");
        assert_eq!(rendered.output, "> Quoted text");
    }

    #[test]
    fn test_render_tree_to_markdown_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let node = quote.render_tree();
        let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default())
            .expect("markdown render");
        assert!(rendered.output.starts_with("> "));
        assert!(rendered.output.contains("To be or not to be"));
        assert!(rendered.output.contains("— Shakespeare"));
    }

    #[test]
    fn test_render_tree_to_browser() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        let rendered =
            render_browser_node(&node, &BrowserRenderOptions::default()).expect("browser render");
        let html = rendered.output.render();
        assert!(html.contains("<blockquote"));
        assert!(html.contains("Quoted text"));
    }

    // =========================================================================
    // MarkdownRenderable Tests
    // =========================================================================

    #[test]
    fn test_render_markdown_simple() {
        let quote = BlockQuote::from("Quoted text");
        assert_eq!(quote.render_markdown(), "> Quoted text");
    }

    #[test]
    fn test_render_markdown_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let output = quote.render_markdown();
        assert!(output.starts_with("> "));
        assert!(output.contains("To be or not to be"));
        assert!(output.contains("— Shakespeare"));
    }

    #[test]
    fn test_render_markdown_multiline() {
        let quote = BlockQuote::from("First line\nSecond line");
        let output = quote.render_markdown();
        for line in output.lines() {
            assert!(
                line.starts_with('>'),
                "every line should be prefixed with `>`: {line:?}"
            );
        }
        assert!(output.contains("First line"));
        assert!(output.contains("Second line"));
    }

    #[test]
    fn test_render_markdown_from_prose_strips_styling() {
        let prose = Prose::new("<b>bold content</b>");
        let quote = BlockQuote::from(prose);
        let output = quote.render_markdown();
        // Prose styling is intentionally flattened by the tree projection.
        assert!(output.contains("bold content"));
        assert!(output.starts_with("> "));
    }

    #[test]
    fn test_render_markdown_empty_quote() {
        let quote = BlockQuote::from("");
        // Empty content should still be representable; even a single `>` line
        // is acceptable. The contract is only "no panic and valid Markdown".
        let _ = quote.render_markdown();
    }

    #[test]
    fn test_render_markdown_plus_matches_markdown() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("portable across dialects"),
            Some("Author"),
        );
        assert_eq!(quote.render_markdown(), quote.render_markdown_plus());
    }

    #[test]
    fn test_render_markdown_ignores_style() {
        // Style attributes (color, background, border) have no Markdown
        // representation; the output must match an unstyled quote.
        let styled = BlockQuote::from("text")
            .with_text_color(Color::Tailwind(Tailwind::Red500))
            .with_bg_color(Color::Tailwind(Tailwind::Gray100))
            .with_left_block_color(Color::Tailwind(Tailwind::Blue500));
        let plain = BlockQuote::from("text");
        assert_eq!(styled.render_markdown(), plain.render_markdown());
    }

    #[test]
    fn test_render_markdown_custom_border_uses_canonical_markdown() {
        // A custom terminal-only border prefix must not leak into Markdown.
        let quote = BlockQuote::from("text").with_border("> ");
        let output = quote.render_markdown();
        assert!(output.starts_with("> "));
        assert!(!output.contains("│"));
    }

    // =========================================================================
    // BrowserRenderable Tests
    // =========================================================================

    #[test]
    fn test_browser_renderable_fragment_contains_blockquote() {
        let quote = BlockQuote::from("Quoted text");
        let html = BrowserRenderable::render_html_fragment(&quote).render();
        assert!(html.contains("<blockquote"));
        assert!(html.contains("Quoted text"));
    }

    #[test]
    fn test_browser_renderable_fragment_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let html = BrowserRenderable::render_html_fragment(&quote).render();
        assert!(html.contains("<blockquote"));
        assert!(html.contains("To be or not to be"));
        assert!(html.contains("— Shakespeare"));
        // Attribution becomes a second paragraph inside the blockquote.
        assert_eq!(html.matches("<p>").count(), 2);
    }

    #[test]
    fn test_browser_renderable_fragment_from_prose() {
        let prose = Prose::new("<b>bold content</b>");
        let quote = BlockQuote::from(prose);
        let html = BrowserRenderable::render_html_fragment(&quote).render();
        assert!(html.contains("<blockquote"));
        // Prose styling is flattened to plain text by `render_tree`.
        assert!(html.contains("bold content"));
    }

    #[test]
    fn test_browser_renderable_empty_fragment() {
        let quote = BlockQuote::from("");
        let html = BrowserRenderable::render_html_fragment(&quote).render();
        assert!(html.contains("<blockquote"));
    }

    #[test]
    fn test_browser_renderable_page_includes_fragment() {
        let quote = BlockQuote::from("Quoted text");
        let page = BrowserRenderable::render_html_page(&quote, None);
        let html = page.render().expect("render");
        assert!(html.contains("<html"));
        assert!(html.contains("<body>"));
        assert!(html.contains("<blockquote"));
        assert!(html.contains("Quoted text"));
    }

    #[test]
    fn test_browser_renderable_page_with_options() {
        use renderable::browser::PageOptions;

        let quote = BlockQuote::from("Quoted text");
        let page = BrowserRenderable::render_html_page(
            &quote,
            Some(PageOptions {
                css_variables: Some(vec![("accent".to_string(), "#336699".to_string())]),
                ..PageOptions::default()
            }),
        );
        let html = page.render().expect("render");
        // PageOptions wire through to the rolled-up CSS, and the fragment is
        // preserved inside the body.
        assert!(html.contains("<blockquote"));
        assert!(html.contains("Quoted text"));
    }

    #[test]
    fn test_browser_renderable_as_any_downcasts() {
        let quote = BlockQuote::from("any");
        let any = BrowserRenderable::as_any(&quote);
        assert!(any.downcast_ref::<BlockQuote>().is_some());
    }

    #[test]
    fn test_browser_renderable_styled_still_produces_blockquote() {
        // Browser style lowering carries `color` through as an inline CSS
        // declaration (e.g. `<blockquote style="color:#…">`), so an exact
        // `<blockquote>` substring match is too tight when the element
        // carries attributes. We only check the semantic element name and
        // the content survive.
        let quote = BlockQuote::from("Important")
            .with_text_color(Color::Tailwind(Tailwind::Red500))
            .with_left_block_color(Color::Tailwind(Tailwind::Blue500));
        let html = BrowserRenderable::render_html_fragment(&quote).render();
        assert!(html.contains("<blockquote"));
        assert!(html.contains("</blockquote>"));
        assert!(html.contains("Important"));
    }

    // =========================================================================
    // Compatibility Fallback (with_border) Regression Tests
    // =========================================================================

    #[test]
    fn test_custom_border_uses_legacy_renderer() {
        // The default `│ ` border routes through the tree renderer. A custom
        // border such as `"> "` must keep working through the bespoke
        // compatibility path — the prefix is preserved verbatim per line, and
        // attribution uses the trimmed border character.
        let quote = BlockQuote::from("hello world").with_border("> ");
        let result = quote.render_optimistic(Some(80));
        assert_eq!(strip_ansi(&result), "> hello world");
    }

    #[test]
    fn test_custom_border_with_attribution_preserves_prefix() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("Quoted text"),
            Some("Source"),
        );
        let custom = quote.with_border("!! ");
        let result = custom.render_optimistic(Some(80));
        let stripped = strip_ansi(&result);
        // Every non-blank content/attribution line begins with the custom
        // border; the bespoke renderer uses `!!` (trimmed) on the blank
        // separator before attribution.
        assert!(stripped.contains("!! Quoted text"));
        assert!(stripped.contains("!! — Source"));
        // The default `│` glyph must not appear once a custom border is set.
        assert!(!stripped.contains('│'));
    }

    #[test]
    fn test_custom_border_drops_default_styled_glyph() {
        // The bespoke compatibility path emits the literal custom prefix
        // verbatim; no box-drawing `│` glyph should appear once a non-default
        // border has been set, regardless of the underlying `Style.border`
        // color slot inherited from `BlockQuote::default()`.
        let quote = BlockQuote::from("plain").with_border("> ");
        let result = quote.render_optimistic(Some(80));
        assert!(strip_ansi(&result).starts_with("> "));
        assert!(!strip_ansi(&result).contains('│'));
    }

    // =========================================================================
    // Inline Prose Styling Preservation (regression for review feedback)
    // =========================================================================

    /// A `Prose` body carrying `<b>foo</b>` must emit SGR bold around `foo`
    /// when the quote renders on the terminal target via the render-tree path.
    ///
    /// Prior to the prose IR projection this assertion failed: the BlockQuote
    /// tree projection flattened Prose to plain text via `plain_text()`, so
    /// `bt quote "<b>foo</b>"` lost all inline styling. The fix downcasts the
    /// content component to `Prose` and projects its IR into structured inline
    /// nodes (Strong / Emphasis / styled Span) that the terminal tree
    /// renderer lowers back to SGR.
    #[test]
    fn test_prose_bold_inline_styling_survives_terminal_tree_render() {
        let prose = Prose::new("<b>foo</b> bar");
        let quote = BlockQuote::from(prose);
        let term = Terminal::new_optimistic(80);
        let rendered = quote.render(&term);

        // SGR bold open code is present.
        assert!(
            rendered.contains("\x1b[1m"),
            "expected SGR bold open (`\\x1b[1m`) around bold span; got: {rendered:?}"
        );
        // The visible text `foo` and `bar` survive.
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("foo"), "missing `foo` in: {stripped:?}");
        assert!(stripped.contains("bar"), "missing `bar` in: {stripped:?}");
    }

    /// Italic inline styling lowers to SGR `\x1b[3m`.
    #[test]
    fn test_prose_italic_inline_styling_survives_terminal_tree_render() {
        let prose = Prose::new("plain <i>emphatic</i> tail");
        let quote = BlockQuote::from(prose);
        let term = Terminal::new_optimistic(80);
        let rendered = quote.render(&term);

        assert!(
            rendered.contains("\x1b[3m"),
            "expected SGR italic open (`\\x1b[3m`); got: {rendered:?}"
        );
        assert!(strip_ansi(&rendered).contains("emphatic"));
    }

    /// Colored inline runs lower to truecolor SGR escapes.
    #[test]
    fn test_prose_color_inline_styling_survives_terminal_tree_render() {
        let prose = Prose::new("<red>error</red>: something went wrong");
        let quote = BlockQuote::from(prose);
        let term = Terminal::new_optimistic(80);
        let rendered = quote.render(&term);

        // Either truecolor (`38;2;…`) or basic-color (`31`) is acceptable —
        // the optimistic terminal picks whichever palette matches its
        // declared color depth. Both signal "color SGR was emitted".
        assert!(
            rendered.contains("\x1b[38;2;") || rendered.contains("\x1b[31m"),
            "expected a foreground color SGR for <red>; got: {rendered:?}"
        );
        assert!(strip_ansi(&rendered).contains("error"));
    }

    // =========================================================================
    // Embedded Prose Fenced Code (regression for review-2 container gap)
    // =========================================================================

    /// `true` if any `Paragraph` node anywhere in the tree directly contains a
    /// block-level `Code` child — the invalid shape that tripped render-tree
    /// validation before the fold fix.
    fn paragraph_contains_code(node: &RenderNode) -> bool {
        let bad_here = matches!(node.kind, NodeKind::Paragraph { .. })
            && node
                .children()
                .iter()
                .any(|c| matches!(c.kind, NodeKind::Code { .. }));
        bad_here || node.children().iter().any(paragraph_contains_code)
    }

    /// `true` if a block-level `Code` node appears anywhere in the tree.
    fn has_code(node: &RenderNode) -> bool {
        matches!(node.kind, NodeKind::Code { .. })
            || node.children().iter().any(has_code)
    }

    /// Regression: a `Prose` body carrying a fenced code block embedded in a
    /// `BlockQuote` must project to a render-tree-valid shape. Before the fold
    /// fix the quote wrapped the whole Prose node sequence — including the
    /// block-level `Code` node — in a single `Paragraph`, which tree
    /// validation rejected, so the terminal renderer emitted empty output
    /// (`bt quote '<red>before \`\`\`code\`\`\` after</red>'`).
    #[test]
    fn test_prose_fenced_code_in_block_quote_renders_via_tree() {
        let prose = Prose::new("<red>before\n```\ncode\n```\nafter</red>");
        let quote = BlockQuote::from(prose);
        let term = Terminal::new_optimistic(80);
        let rendered = quote.render(&term);

        // A non-empty render proves validation passed: `render_via_tree`
        // swallows a validation failure into an empty string.
        assert!(!rendered.is_empty(), "expected non-empty render");
        let stripped = strip_ansi(&rendered);
        assert!(stripped.contains("before"), "missing `before`: {stripped:?}");
        assert!(stripped.contains("code"), "missing `code`: {stripped:?}");
        assert!(stripped.contains("after"), "missing `after`: {stripped:?}");
    }

    /// The embedded fenced code block stays a block-level sibling of the
    /// surrounding paragraphs instead of nesting inside one.
    #[test]
    fn test_prose_fenced_code_in_block_quote_is_block_sibling() {
        let prose = Prose::new("<red>before\n```\ncode\n```\nafter</red>");
        let quote = BlockQuote::from(prose);
        let node = quote.render_tree();
        assert!(
            !paragraph_contains_code(&node),
            "a block-level Code node must not nest inside a Paragraph"
        );
        assert!(has_code(&node), "expected a block-level Code node in the tree");
    }
}
