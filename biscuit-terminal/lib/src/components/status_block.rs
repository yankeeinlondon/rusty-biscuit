use std::any::Any;

use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::style::{Border, BorderLineStyle, BorderSides, BorderWeight, PerMode, Style};
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

use crate::{
    components::{
        block_quote::BlockQuote,
        prose::Prose,
        renderable::{BrowserRenderable, RenderableTerminalContent, TerminalRenderable},
        status::{Status, StatusState},
    },
    discovery::detection::ColorDepth,
    discovery::eval::strip_ansi_codes,
    render_tree::{TerminalRenderOptions, render_terminal_node},
    terminal::Terminal,
    utils::{
        color::Color,
        layout::{Layout, Length, Edges, TargetValue},
        wrap_policy::WordWrap,
    },
};

/// The default body border applied when [`StatusBlock`] uses the canonical
/// thick left border. Any other value routes terminal rendering through the
/// bespoke compatibility path so an arbitrary terminal prefix string is
/// honored verbatim.
const DEFAULT_BORDER: &str = "┃ ";

/// A severity-colored block with optional header, body, and hint content.
///
/// `StatusBlock` combines a [`Status`] header line, a [`BlockQuote`] body, and
/// an optional prose hint into one renderable surface for error- and
/// warning-style output.
///
/// ## Rendering
///
/// The component normally routes through the canonical render tree
/// ([`TreeRenderable::render_tree`]) so Terminal, Browser, and Markdown
/// targets share one projection. The thick left border (`┃ `) is represented
/// as a typed [`Border`] in the projected tree. An arbitrary border prefix
/// supplied through [`StatusBlock::border`] is a terminal compatibility
/// surface; the [`TerminalRenderable`] impl routes those custom-prefix paths
/// through a bespoke renderer ([`StatusBlock::render_bespoke`]) because the
/// canonical tree cannot express arbitrary target-specific prefix strings.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::status_block::StatusBlock;
/// use biscuit_terminal::components::status::StatusState;
/// use biscuit_terminal::components::prose::Prose;
///
/// let block = StatusBlock::new(StatusState::Error)
///     .header("<b>Shell Expansion Failed</b>")
///     .body(Prose::new("Missing closing brace in `${...}` directive."))
///     .hint("Check the template syntax and retry.");
/// let _ = block; // see TerminalRenderable / BrowserRenderable / MarkdownRenderable
/// ```
///
/// ## Layout & Style Contract
///
/// `StatusBlock` is an internal-layout component (spec C2). It projects to a
/// [`Root`](renderable::tree::NodeKind::Root) carrying the configured
/// [`Layout`] on the root node; the shared render-tree fold resolves the
/// outer box, and the body / hint text wraps inside the resolved content
/// width:
///
/// - [`Width::Auto`] (the constructor default's `WrapProse(8, None)` word
///   wrap) and [`Width::Fixed`] **fill** the available width by wrapping the
///   body / hint; [`Width::FitContent`] hugs short body content.
/// - **Slack sink** (spec D2): the message / body region. The header icon,
///   severity color, status prefix, and the thick left border chrome (`┃ `)
///   stay fixed across width modes — only the body text column absorbs slack
///   by wrapping.
/// - The typed thick left [`Border`] is lowered by the shared fold, not
///   re-implemented as bespoke chrome on the default path; only an arbitrary
///   string prefix (set via [`StatusBlock::border`]) routes through
///   [`StatusBlock::render_bespoke`] because the render tree cannot express
///   target-specific prefix strings.
/// - A fractional `Fixed(50%)` is resolved exactly once by the fold; the
///   body wraps to the resolved content width and never re-resolves the raw
///   percentage (the `Fixed(50%) → 25%` double-application bug).
///
/// [`Width::Auto`]: renderable::layout::Width::Auto
/// [`Width::Fixed`]: renderable::layout::Width::Fixed
/// [`Width::FitContent`]: renderable::layout::Width::FitContent
#[derive(Debug, Clone)]
pub struct StatusBlock {
    severity: StatusState,
    header: Option<String>,
    body: Vec<Prose>,
    hint: Option<String>,
    border_color: Option<Color>,
    border: String,
    layout: Layout,
    style: Style,
}

impl StatusBlock {
    /// Creates a new status block for the given severity.
    pub fn new(severity: StatusState) -> Self {
        Self {
            severity,
            header: None,
            body: Vec::new(),
            hint: None,
            border_color: None,
            border: DEFAULT_BORDER.to_string(),
            layout: Layout {
                margin: Edges {
                    right: TargetValue::universal(Length::ch(5)),
                    ..Edges::default()
                },
                word_wrap: WordWrap::WrapProse(Some(8), None),
                ..Layout::default()
            },
            style: Style::default(),
        }
    }

    /// Sets a prose-formatted header rendered as a [`Status`] line.
    pub fn header(mut self, header: impl Into<String>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Sets the body content as a vector of [`Prose`] items.
    ///
    /// Each item is rendered individually and stacked vertically with a
    /// single blank line between them inside a continuous block quote.
    pub fn body(mut self, body: impl crate::components::prose::IntoProseVec) -> Self {
        self.body = body.into_prose_vec();
        self
    }

    /// Sets the body to a single [`Prose`] item.
    ///
    /// Convenience shortcut for the common case where the body is a single
    /// styled line or paragraph.
    pub fn body_line(self, line: impl Into<Prose>) -> Self {
        self.body(vec![line.into()])
    }

    /// Sets a prose-formatted hint.
    ///
    /// When body content is present, the hint is rendered inside the same
    /// block quote as the body, separated from it by a blank paragraph and
    /// styled with italic emphasis. When the body is empty, the hint renders
    /// as a standalone paragraph outside any block quote. Blank hints are
    /// omitted.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Overrides the severity-derived border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Overrides the border glyph used by the body block quote.
    ///
    /// A non-default value here is a terminal-only compatibility knob: the
    /// canonical render tree models the default `┃ ` border as a typed thick
    /// left [`Border`], but cannot express an arbitrary prefix string. The
    /// [`TerminalRenderable`] impl detects a custom border and routes
    /// rendering through the bespoke fallback in this case; Browser and
    /// Markdown output never see the custom prefix.
    pub fn border(mut self, border: impl Into<String>) -> Self {
        self.border = border.into();
        self
    }

    /// Whether this block uses the default thick left border.
    ///
    /// The tree path can faithfully represent the default border via a typed
    /// [`Border`]; an arbitrary prefix string falls back to the bespoke
    /// terminal renderer.
    fn has_default_border(&self) -> bool {
        self.border == DEFAULT_BORDER
    }

    fn resolved_border_color(&self) -> Color {
        self.border_color
            .unwrap_or_else(|| self.severity.default_color())
    }

    fn non_blank_hint(&self) -> Option<&str> {
        self.hint.as_deref().filter(|h| !h.trim().is_empty())
    }

    /// Returns the portable Unicode fallback icon for a [`StatusState`].
    ///
    /// The tree projection uses these stable glyphs so Markdown and Browser
    /// targets emit the same icon regardless of whether a Nerd Font is
    /// available at render time.
    fn severity_icon(state: &StatusState) -> &'static str {
        match state {
            StatusState::NotStarted => "\u{25fb}", // ◻
            StatusState::Active => "\u{25fd}",     // ◽
            StatusState::Success => "\u{2713}",    // ✓
            StatusState::Error => "\u{292b}",      // ⤫
            #[allow(deprecated)]
            StatusState::Failure => "\u{292b}", // ⤫ (deprecated alias for Error)
            StatusState::Warning => "\u{26a0}",    // ⚠
            StatusState::Info => "\u{2139}",       // ℹ
            StatusState::ToolUse => "\u{1f527}",   // 🔧
            StatusState::Subagent => "\u{1f916}",  // 🤖
        }
    }

    /// CSS-style severity class suffix (e.g. `"error"`, `"warning"`).
    fn severity_class(&self) -> &'static str {
        match self.severity {
            StatusState::NotStarted => "not-started",
            StatusState::Active => "active",
            StatusState::Success => "success",
            StatusState::Error => "error",
            #[allow(deprecated)]
            StatusState::Failure => "error",
            StatusState::Warning => "warning",
            StatusState::Info => "info",
            StatusState::ToolUse => "tool-use",
            StatusState::Subagent => "subagent",
        }
    }

    /// Plain-text projection of a prose source string.
    ///
    /// Renders the prose through a deliberately uncolored, non-Nerd terminal
    /// and strips any residual escape codes so bracketed tags such as
    /// `<b>Bold</b>` flatten to `Bold` rather than leaking the markup into
    /// Markdown or Browser output.
    fn prose_plain_text(prose_src: &str) -> String {
        let mut term = Terminal::builder()
            .width(80)
            .color_depth(ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);
        strip_ansi_codes(&Prose::new(prose_src).render(&term))
    }

    /// Flatten the body into a single plain-text blob with `\n\n` between
    /// items.
    fn body_plain_text(&self) -> String {
        self.body
            .iter()
            .map(|p| Self::prose_plain_text(p.content()))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Builds the canonical projection of this status block.
    ///
    /// This is the **single private projection helper**. The
    /// [`TreeRenderable::render_tree`] entry point delegates to it; the
    /// Browser and Markdown adapters render the returned node directly. The
    /// projection emits a `Root` with one optional header `Paragraph` plus a
    /// body surface that is one of:
    ///
    /// - a `BlockQuote` carrying `leading blank, body, [blank, hint]` when
    ///   body content is present — the trailing `blank, hint` pair is only
    ///   emitted for a non-blank hint;
    /// - a standalone `Paragraph` hint when the body is empty and a
    ///   non-blank hint is configured;
    /// - nothing beyond the optional header when both body and hint are
    ///   empty.
    ///
    /// The default border maps to a typed thick left [`Border`] on the body
    /// node so the terminal tree renderer reproduces the `┃ ` glyph. Arbitrary
    /// custom borders are intentionally *not* encoded here; the
    /// [`TerminalRenderable`] impl detects them and falls back to the bespoke
    /// path.
    fn to_render_node(&self) -> RenderNode {
        let severity_color = self.resolved_border_color();
        let severity_class = format!("status-block--{}", self.severity_class());
        let mut children: Vec<RenderNode> = Vec::new();

        if let Some(header_text) = &self.header {
            let icon = Self::severity_icon(&self.severity);
            let header_plain = Self::prose_plain_text(header_text);
            let header_text = format!("{icon} {header_plain}");
            let mut node = RenderNode::paragraph(vec![RenderNode::text(header_text)]);
            node.attrs.classes = vec!["status-block__header".into()];
            node.attrs.set_style(&Style {
                color: Some(TargetValue::universal(PerMode::universal(severity_color))),
                ..Style::default()
            });
            children.push(node);
        }

        if !self.body.is_empty() {
            let body_text = self.body_plain_text();
            let mut block_children: Vec<RenderNode> = Vec::new();

            block_children.push(RenderNode::paragraph(vec![RenderNode::text(
                String::new(),
            )]));
            block_children.push(RenderNode::paragraph(vec![RenderNode::text(body_text)]));

            if let Some(hint_text) = self.non_blank_hint() {
                block_children.push(RenderNode::paragraph(vec![RenderNode::text(
                    String::new(),
                )]));

                let hint = Self::prose_plain_text(hint_text);

                let mut hint_node = RenderNode::paragraph(vec![RenderNode::emphasis(vec![
                    RenderNode::text(hint),
                ])]);
                hint_node.attrs.classes = vec!["status-block__hint".into()];
                block_children.push(hint_node);
            }

            let mut node = RenderNode::block_quote(block_children);
            node.attrs.classes = vec!["status-block__body".into()];
            node.attrs.set_style(&Style {
                border: Some(Border {
                    color: Some(TargetValue::universal(PerMode::universal(severity_color))),
                    weight: BorderWeight::Thick,
                    line_style: BorderLineStyle::Solid,
                    sides: BorderSides::Sides {
                        top: false,
                        right: false,
                        bottom: false,
                        left: true,
                    },
                    ..Border::default()
                }),
                ..Style::default()
            });
            // The thick left border's inner gap is `padding` now — the terminal
            // tree renderer no longer inserts an implicit space inside the
            // border — so reserve one cell to reproduce the `┃ ` gap.
            node.attrs.set_layout(&Layout {
                padding: Edges {
                    left: TargetValue::universal(Length::ch(1)),
                    ..Edges::default()
                },
                ..Layout::default()
            });
            children.push(node);
        } else if let Some(hint_text) = self.non_blank_hint() {
            let hint = Self::prose_plain_text(hint_text);
            let mut node = RenderNode::paragraph(vec![RenderNode::text(hint)]);
            node.attrs.classes = vec!["status-block__hint".into()];
            children.push(node);
        }

        let mut root = RenderNode::root(children);
        root.attrs.classes = vec!["status-block".into(), severity_class];
        // `StatusBlock::new()` has a non-default layout (right margin 5,
        // `WrapProse(8, None)`), so the layout must always be seeded here —
        // current `TreeComponent` and `BrowserTreeComponent` adapters render
        // the node returned by `render_tree()` without applying the optional
        // `tree_layout()` hook.
        root.attrs.set_layout(&self.layout);
        crate::components::renderable::overlay_style_onto_node(&mut root, &self.style);
        root
    }

    /// Renders the status block through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl when the component carries the
    /// default `┃ ` border. Failures are logged via `tracing::error!` and fall
    /// back to an empty string: the [`TerminalRenderable::render`] trait is
    /// infallible by contract, and surfacing a `[render-tree error: …]`
    /// sentinel as in-band terminal text would pollute user output.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_node();
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "StatusBlock",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders the status block via the sanctioned bespoke escape hatch.
    ///
    /// Retained as a `#[doc(hidden)]` surface because [`StatusBlock`] supports
    /// an arbitrary string [`StatusBlock::border`] prefix that the render
    /// tree's standard [`Border`] enum cannot express. The active
    /// [`TerminalRenderable::render`] path routes through
    /// [`Self::render_via_tree`] for the default border and falls back to
    /// this method for arbitrary prefixes.
    ///
    /// ## Notes
    ///
    /// `#[doc(hidden)]` because this is an internal escape hatch, not part
    /// of the public surface; `pub` so integration parity tests can reach
    /// it. Removing this without first adding an equivalent
    /// arbitrary-border capability to the render tree is a regression.
    /// Returns `true` when the configured header contains Prose markup that
    /// would lose meaning if the header were flattened to plain text.
    ///
    /// The canonical projection in [`Self::to_render_node`] is shared with
    /// the Browser and Markdown adapters, so it intentionally produces a
    /// single plain-text [`renderable::tree::NodeKind::Text`] for the
    /// header. That projection is correct for those targets but would erase
    /// markdown-link OSC 8 hyperlinks and bracketed-tag SGR styling on the
    /// terminal. This guard detects the cases where bespoke rendering is
    /// the more faithful path.
    fn header_needs_prose_rendering(&self) -> bool {
        let Some(header) = &self.header else {
            return false;
        };
        // Markdown link syntax `[text](url)` — lowered to OSC 8 by Prose.
        if header.contains("](") && header.contains('[') {
            return true;
        }
        // Bracketed Prose tags — `<bold>`, `<a href=...>`, `<red>`, etc.
        if header.contains('<') && header.contains('>') {
            return true;
        }
        false
    }

    #[doc(hidden)]
    pub fn render_bespoke(&self, term: &Terminal) -> String {
        let mut parts = Vec::new();

        if let Some(ref header_text) = self.header {
            let status = Status::from_prose(header_text).state(self.severity.clone());
            parts.push(status.render(term));
        }

        if !self.body.is_empty() {
            let mut composed_parts: Vec<String> = Vec::new();

            let body_composed = self
                .body
                .iter()
                .map(|p| p.render(term))
                .collect::<Vec<_>>()
                .join("\n\n");
            composed_parts.push(body_composed);

            if let Some(hint_text) = self.non_blank_hint() {
                composed_parts.push(Prose::new(format!("<i>{}</i>", hint_text)).render(term));
            }

            let composed = composed_parts.join("\n\n");
            let composed = format!("\n{composed}");
            let mut block =
                BlockQuote::new(RenderableTerminalContent::String(composed), None::<&str>)
                    .with_left_block_color(self.resolved_border_color())
                    .with_border(&self.border);
            block.layout_mut().margin.left = self.layout.margin.left.clone();
            block.layout_mut().margin.right = self.layout.margin.right.clone();
            block.layout_mut().word_wrap = self.layout.word_wrap.clone();
            parts.push(block.render(term));
        } else if let Some(hint_text) = self.non_blank_hint() {
            // No body but there is a hint — keep as a separate paragraph.
            parts.push(Prose::new(hint_text).render(term));
        }

        parts.join("\n")
    }
}

impl TerminalRenderable for StatusBlock {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        self.render(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        if self.has_default_border() && !self.header_needs_prose_rendering() {
            self.render_via_tree(term)
        } else {
            // Compatibility fallback for `StatusBlock::border()` arbitrary
            // terminal prefixes and for Prose-bearing headers (markdown
            // links, bold/italic/color tags). The canonical
            // [`to_render_node`] projection flattens header prose to plain
            // text so the Browser and Markdown adapters get stable output;
            // that flattening discards OSC 8 hyperlinks and SGR styling. The
            // bespoke path renders the header through [`Status::from_prose`]
            // which preserves those, so we route here when the header
            // declares Prose content that would otherwise drop on the floor.
            self.render_bespoke(term)
        }
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

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Exposes the tree projection through the canonical
    /// [`TerminalRenderable::render_tree_node`] hook so cross-target adapters
    /// (and nested [`RenderableTerminalContent::Component`] projection) consume
    /// `StatusBlock` structurally instead of degrading to ANSI-stripped text.
    ///
    /// Delegates to [`TreeRenderable::render_tree`] so both public entry
    /// points share one source of truth.
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(<Self as TreeRenderable>::render_tree(self))
    }
}

impl TreeRenderable for StatusBlock {
    /// Projects the status block into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`StatusBlock::to_render_node`].
    fn render_tree(&self) -> RenderNode {
        self.to_render_node()
    }
}

impl MarkdownRenderable for StatusBlock {
    /// Renders the status block as portable Markdown via the canonical render
    /// tree.
    ///
    /// Output is structural: an optional header paragraph (icon + plain
    /// text), an optional `> body` block quote, and an optional hint
    /// paragraph, each separated by a blank line. Severity color and any
    /// custom border color are intentionally dropped because Markdown has no
    /// portable peer for visual appearance, and a custom border prefix never
    /// reaches this path — the bespoke compatibility fallback is terminal-
    /// only.
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        match render_markdown_node(&node, &MarkdownRenderOptions::default()) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "StatusBlock",
                    dialect = "Markdown",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders the status block as MarkdownPlus via the canonical render
    /// tree.
    ///
    /// StatusBlock's tree projection is structurally pure CommonMark
    /// (paragraph + block quote + paragraph), so MarkdownPlus output matches
    /// portable Markdown.
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
                    component = "StatusBlock",
                    dialect = "MarkdownPlus",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl BrowserRenderable for StatusBlock {
    /// Renders the status block as an HTML fragment via the canonical render
    /// tree.
    ///
    /// The fragment is a `<div class="status-block status-block--{severity}">`
    /// containing optional `<p class="status-block__header">`,
    /// `<blockquote class="status-block__body">`, and
    /// `<p class="status-block__hint">` children. Severity color is recorded
    /// in the tree `Style` and lowers to CSS where the Browser renderer
    /// supports it; the structural classes always survive so downstream
    /// stylesheets can theme the block.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// fragment: the [`BrowserRenderable`] contract is infallible.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "StatusBlock",
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
    use crate::{discovery::detection::ColorDepth, utils::color::Tailwind};
    use renderable::tree::NodeKind;

    fn strip_ansi(s: &str) -> String {
        strip_ansi_codes(s)
    }

    fn no_color_terminal(width: u32) -> Terminal {
        let mut term = Terminal::builder()
            .width(width)
            .color_depth(ColorDepth::None)
            .build();
        term.is_nerd_font = Some(false);
        term
    }

    fn color_terminal(width: u32) -> Terminal {
        let mut term = Terminal::builder()
            .width(width)
            .color_depth(ColorDepth::TrueColor)
            .build();
        term.is_nerd_font = Some(false);
        term
    }

    // ── Severity color resolution ────────────────────────────────────

    #[test]
    fn error_severity_uses_red500() {
        let block = StatusBlock::new(StatusState::Error);
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Red500)
        );
    }

    #[test]
    fn warning_severity_uses_orange500() {
        let block = StatusBlock::new(StatusState::Warning);
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Orange500)
        );
    }

    #[test]
    fn info_severity_uses_blue500() {
        let block = StatusBlock::new(StatusState::Info);
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Blue500)
        );
    }

    #[test]
    fn default_color_matches_status_icon() {
        for state in [
            StatusState::Error,
            StatusState::Warning,
            StatusState::Info,
            StatusState::Success,
            StatusState::Active,
            StatusState::ToolUse,
            StatusState::Subagent,
        ] {
            assert_eq!(
                StatusBlock::new(state.clone()).resolved_border_color(),
                state.default_color()
            );
        }
    }

    #[test]
    fn custom_border_color_overrides_severity() {
        let block = StatusBlock::new(StatusState::Warning)
            .border_color(Color::Tailwind(Tailwind::Orange700));
        assert_eq!(
            block.resolved_border_color(),
            Color::Tailwind(Tailwind::Orange700)
        );
    }

    // ── Tree projection structure ─────────────────────────────────────

    #[test]
    fn render_tree_root_is_root_with_status_block_classes() {
        let block = StatusBlock::new(StatusState::Error).body("body");
        let node = block.render_tree();
        assert!(matches!(node.kind, NodeKind::Root { .. }));
        assert!(node.attrs.classes.iter().any(|c| c == "status-block"));
        assert!(
            node.attrs
                .classes
                .iter()
                .any(|c| c == "status-block--error")
        );
    }

    #[test]
    fn render_tree_seeds_layout_on_root() {
        // StatusBlock::new() has a non-default layout (right margin = 5);
        // the projected root must carry it directly so the adapter renderers
        // honor it without relying on the optional `tree_layout()` hook.
        let block = StatusBlock::new(StatusState::Info).body("body");
        let node = block.render_tree();
        assert!(node.attrs.layout().is_some());
    }

    #[test]
    fn header_only_emits_one_paragraph() {
        let block = StatusBlock::new(StatusState::Info).header("Title");
        let node = block.render_tree();
        let children = match &node.kind {
            NodeKind::Root { children } => children,
            _ => panic!("expected Root"),
        };
        assert_eq!(children.len(), 1);
        assert!(matches!(children[0].kind, NodeKind::Paragraph { .. }));
        assert!(
            children[0]
                .attrs
                .classes
                .iter()
                .any(|c| c == "status-block__header")
        );
    }

    #[test]
    fn body_only_emits_one_block_quote() {
        let block = StatusBlock::new(StatusState::Info).body("body text");
        let node = block.render_tree();
        let children = match &node.kind {
            NodeKind::Root { children } => children,
            _ => panic!("expected Root"),
        };
        assert_eq!(children.len(), 1);
        assert!(matches!(children[0].kind, NodeKind::BlockQuote { .. }));
        assert!(
            children[0]
                .attrs
                .classes
                .iter()
                .any(|c| c == "status-block__body")
        );
    }

    #[test]
    fn all_parts_emit_header_and_block_quote_with_hint_inside() {
        let block = StatusBlock::new(StatusState::Error)
            .header("Header")
            .body("Body")
            .hint("Hint");
        let node = block.render_tree();
        let children = match &node.kind {
            NodeKind::Root { children } => children,
            _ => panic!("expected Root"),
        };
        assert_eq!(children.len(), 2);
        assert!(matches!(children[0].kind, NodeKind::Paragraph { .. }));
        assert!(matches!(children[1].kind, NodeKind::BlockQuote { .. }));
        let bq_children = match &children[1].kind {
            NodeKind::BlockQuote { children } => children,
            _ => panic!("expected BlockQuote"),
        };
        assert_eq!(
            bq_children.len(),
            4,
            "leading blank + body + hint separator blank + hint"
        );
        let hint_node = bq_children
            .iter()
            .find(|c| c.attrs.classes.iter().any(|cl| cl == "status-block__hint"))
            .expect("hint paragraph inside block quote");
        let hint_children = match &hint_node.kind {
            NodeKind::Paragraph { children } => children,
            _ => panic!("hint must be a Paragraph"),
        };
        assert!(
            hint_children
                .iter()
                .any(|c| matches!(c.kind, NodeKind::Emphasis { .. })),
            "hint paragraph must contain an Emphasis child for portable italic"
        );
    }

    #[test]
    fn body_style_carries_thick_left_border() {
        let block = StatusBlock::new(StatusState::Error).body("text");
        let node = block.render_tree();
        let body = match &node.kind {
            NodeKind::Root { children } => &children[0],
            _ => panic!("expected Root"),
        };
        let style = body.attrs.style().expect("body has style");
        let border = style.border.as_ref().expect("body has border");
        assert_eq!(border.weight, BorderWeight::Thick);
        assert_eq!(border.line_style, BorderLineStyle::Solid);
        match border.sides {
            BorderSides::Sides {
                top,
                right,
                bottom,
                left,
            } => {
                assert!(!top && !right && !bottom && left);
            }
            other => panic!("expected per-side border, got {other:?}"),
        }
    }

    #[test]
    fn header_style_carries_severity_color() {
        let block = StatusBlock::new(StatusState::Warning).header("Header");
        let node = block.render_tree();
        let header = match &node.kind {
            NodeKind::Root { children } => &children[0],
            _ => panic!("expected Root"),
        };
        assert!(header.attrs.style().and_then(|s| s.color.clone()).is_some());
    }

    // ── Fallback icon mapping ─────────────────────────────────────────

    #[test]
    fn severity_icon_returns_portable_unicode() {
        assert_eq!(StatusBlock::severity_icon(&StatusState::Error), "⤫");
        assert_eq!(StatusBlock::severity_icon(&StatusState::Warning), "⚠");
        assert_eq!(StatusBlock::severity_icon(&StatusState::Info), "ℹ");
        assert_eq!(StatusBlock::severity_icon(&StatusState::Success), "✓");
        assert_eq!(StatusBlock::severity_icon(&StatusState::Active), "◽");
        assert_eq!(StatusBlock::severity_icon(&StatusState::NotStarted), "◻");
        assert_eq!(StatusBlock::severity_icon(&StatusState::ToolUse), "🔧");
        assert_eq!(StatusBlock::severity_icon(&StatusState::Subagent), "🤖");
    }

    #[test]
    fn header_text_in_tree_includes_fallback_icon() {
        let block = StatusBlock::new(StatusState::Error).header("Failed");
        let node = block.render_tree();
        let json = serde_json::to_string(&node).unwrap();
        assert!(
            json.contains("⤫ Failed"),
            "expected fallback icon prefix: {json}"
        );
    }

    // ── Prose flattening ──────────────────────────────────────────────

    #[test]
    fn header_prose_tags_are_flattened() {
        let block = StatusBlock::new(StatusState::Error).header("<b>Bold</b> tail");
        let node = block.render_tree();
        let json = serde_json::to_string(&node).unwrap();
        // The raw `<b>` markup must not leak into the tree.
        assert!(!json.contains("<b>"));
        assert!(json.contains("Bold"));
        assert!(json.contains("tail"));
    }

    #[test]
    fn body_prose_tags_are_flattened() {
        let block = StatusBlock::new(StatusState::Info).body(Prose::new("<b>bold</b> text"));
        let md = block.render_markdown();
        assert!(!md.contains("<b>"));
        assert!(md.contains("bold"));
        assert!(md.contains("text"));
    }

    #[test]
    fn multiple_body_items_join_with_blank_line() {
        let block = StatusBlock::new(StatusState::Info)
            .body(vec![Prose::new("first"), Prose::new("second")]);
        let body_text = block.body_plain_text();
        assert!(body_text.contains("first"));
        assert!(body_text.contains("second"));
        assert!(body_text.contains("\n\n"));
    }

    // ── Terminal rendering: default border (tree path) ────────────────

    #[test]
    fn default_border_terminal_render_uses_tree_path() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error).body("Body text");
        let rendered = strip_ansi(&block.render(&term));
        // The terminal tree renderer reproduces the canonical thick left
        // border glyph from the typed `Style::Border` mapping.
        assert!(
            rendered.contains("┃ Body text"),
            "expected default `┃ ` border, got: {rendered:?}"
        );
    }

    #[test]
    fn default_border_with_header_and_hint() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error)
            .header("<b>Shell expansion failed</b>")
            .body("Missing closing brace")
            .hint("Check the template syntax and retry.");
        let rendered = strip_ansi(&block.render(&term));
        assert!(rendered.contains("⤫ Shell expansion failed"));
        assert!(rendered.contains("┃ Missing closing brace"));
        assert!(rendered.contains("Check the template syntax and retry."));
    }

    #[test]
    fn body_color_appears_on_color_terminal() {
        let term = color_terminal(80);
        let block = StatusBlock::new(StatusState::Error).body("Body");
        let rendered = block.render(&term);
        // Either truecolor or basic color may be emitted depending on the
        // terminal's color depth; either signal proves SGR was applied.
        assert!(rendered.contains('\x1b'));
    }

    // ── Compatibility fallback: arbitrary border ──────────────────────

    #[test]
    fn custom_border_routes_through_bespoke() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error)
            .body("Custom prefix line")
            .border("!! ");
        let rendered = strip_ansi(&block.render(&term));
        // The bespoke fallback honors the arbitrary prefix verbatim.
        assert!(
            rendered.contains("!! Custom prefix line"),
            "expected custom prefix in bespoke output: {rendered:?}"
        );
        // And the default `┃` glyph must not appear once a custom border is
        // set.
        assert!(!rendered.contains('┃'));
    }

    #[test]
    fn custom_border_does_not_affect_markdown() {
        // Markdown always uses the canonical tree projection — a custom
        // terminal-only prefix never leaks into Markdown output.
        let block = StatusBlock::new(StatusState::Error)
            .body("Body")
            .border("!! ");
        let md = block.render_markdown();
        assert!(!md.contains("!! "));
        assert!(md.contains("> Body"));
    }

    #[test]
    fn custom_border_does_not_affect_html() {
        // The Browser path always uses the canonical tree projection.
        let block = StatusBlock::new(StatusState::Error)
            .body("Body")
            .border("!! ");
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(!html.contains("!! "));
        assert!(html.contains("<blockquote"));
    }

    // ── Markdown ──────────────────────────────────────────────────────

    #[test]
    fn markdown_body_only_is_block_quote() {
        let block = StatusBlock::new(StatusState::Error).body("Body");
        let md = block.render_markdown();
        assert!(md.contains("> Body"));
    }

    #[test]
    fn markdown_all_parts_have_three_sections() {
        let block = StatusBlock::new(StatusState::Error)
            .header("Header")
            .body("Body")
            .hint("Hint");
        let md = block.render_markdown();
        // Header line first, then `> Body`, then hint line.
        let header_pos = md.find("Header").expect("header present");
        let body_pos = md.find("> Body").expect("body block quote present");
        let hint_pos = md.find("Hint").expect("hint present");
        assert!(header_pos < body_pos);
        assert!(body_pos < hint_pos);
    }

    #[test]
    fn markdown_header_plus_hint_no_body_omits_block_quote() {
        let block = StatusBlock::new(StatusState::Info)
            .header("Header")
            .hint("Hint only");
        let md = block.render_markdown();
        assert!(md.contains("Header"));
        assert!(md.contains("Hint only"));
        assert!(!md.contains("> "));
    }

    #[test]
    fn markdown_emits_severity_fallback_icon_for_every_state() {
        let cases = [
            (StatusState::Error, "⤫"),
            (StatusState::Warning, "⚠"),
            (StatusState::Info, "ℹ"),
            (StatusState::Success, "✓"),
            (StatusState::Active, "◽"),
            (StatusState::NotStarted, "◻"),
            (StatusState::ToolUse, "🔧"),
            (StatusState::Subagent, "🤖"),
        ];
        for (state, icon) in cases {
            let block = StatusBlock::new(state.clone()).header("Header");
            let md = block.render_markdown();
            assert!(
                md.contains(icon),
                "severity {state:?} should emit icon {icon}: {md:?}"
            );
        }
    }

    #[test]
    fn markdown_plus_matches_markdown() {
        let block = StatusBlock::new(StatusState::Error)
            .header("Header")
            .body("Body")
            .hint("Hint");
        assert_eq!(block.render_markdown(), block.render_markdown_plus());
    }

    #[test]
    fn markdown_ignores_border_color() {
        let plain = StatusBlock::new(StatusState::Error).body("text");
        let styled = StatusBlock::new(StatusState::Error)
            .body("text")
            .border_color(Color::Tailwind(Tailwind::Purple700));
        assert_eq!(plain.render_markdown(), styled.render_markdown());
    }

    // ── Browser ───────────────────────────────────────────────────────

    #[test]
    fn html_fragment_emits_status_block_classes() {
        let block = StatusBlock::new(StatusState::Error)
            .header("Header")
            .body("Body")
            .hint("Hint");
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(html.contains("status-block"));
        assert!(html.contains("status-block--error"));
        assert!(html.contains("status-block__header"));
        assert!(html.contains("status-block__body"));
        assert!(html.contains("status-block__hint"));
    }

    #[test]
    fn html_body_only_omits_header_and_hint_classes() {
        let block = StatusBlock::new(StatusState::Info).body("Body");
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(html.contains("status-block__body"));
        assert!(!html.contains("status-block__header"));
        assert!(!html.contains("status-block__hint"));
        assert!(html.contains("<blockquote"));
    }

    #[test]
    fn html_header_only_has_no_blockquote() {
        let block = StatusBlock::new(StatusState::Info).header("Header");
        let html = BrowserRenderable::render_html_fragment(&block).render();
        assert!(html.contains("status-block__header"));
        assert!(!html.contains("<blockquote"));
    }

    #[test]
    fn html_page_includes_fragment() {
        let block = StatusBlock::new(StatusState::Info).body("Body");
        let page = BrowserRenderable::render_html_page(&block, None);
        let html = page.render().expect("render");
        assert!(html.contains("<html"));
        assert!(html.contains("<body>"));
        assert!(html.contains("<blockquote"));
    }

    #[test]
    fn html_emits_severity_class_for_every_state() {
        let cases = [
            (StatusState::Error, "status-block--error"),
            (StatusState::Warning, "status-block--warning"),
            (StatusState::Info, "status-block--info"),
            (StatusState::Success, "status-block--success"),
            (StatusState::Active, "status-block--active"),
            (StatusState::NotStarted, "status-block--not-started"),
            (StatusState::ToolUse, "status-block--tool-use"),
            (StatusState::Subagent, "status-block--subagent"),
        ];
        for (state, class) in cases {
            let block = StatusBlock::new(state.clone()).body("body");
            let html = BrowserRenderable::render_html_fragment(&block).render();
            assert!(
                html.contains(class),
                "severity {state:?} should emit class {class}: {html:?}"
            );
        }
    }

    // ── Layout, clone, debug ──────────────────────────────────────────

    #[test]
    fn margins_respected_through_tree_path() {
        let term = no_color_terminal(32);
        let block = StatusBlock::new(StatusState::Error)
            .body("alpha beta gamma delta epsilon")
            .left_margin(TargetValue::universal(Length::ch(4)))
            .right_margin(TargetValue::universal(Length::ch(10)));
        let rendered = strip_ansi(&block.render(&term));
        let lines: Vec<_> = rendered.lines().collect();
        assert!(lines.len() > 1, "expected wrapped output: {rendered:?}");
        assert!(
            lines.iter().all(|line| line.starts_with("    ┃ ")),
            "every line should start with the 4-space left margin and `┃ ` border: {rendered:?}"
        );
    }

    #[test]
    fn render_optimistic_matches_render() {
        let width = 80;
        let term = Terminal::new_optimistic(width);
        let block = StatusBlock::new(StatusState::Error)
            .header("<b>Header</b>")
            .body(Prose::new("<blue>Body</blue>"))
            .hint("<dim>Hint</dim>");
        assert_eq!(block.render_optimistic(Some(width)), block.render(&term));
    }

    #[test]
    fn is_block_level() {
        let block = StatusBlock::new(StatusState::Error);
        assert!(block.is_block_level());
    }

    #[test]
    fn clone_preserves_all_fields() {
        let block = StatusBlock::new(StatusState::Warning)
            .header("Header")
            .body("Body")
            .hint("Hint")
            .border_color(Color::Tailwind(Tailwind::Orange700))
            .border("┃ ")
            .left_margin(TargetValue::universal(Length::ch(2)))
            .right_margin(TargetValue::universal(Length::ch(3)));
        let cloned = block.clone();
        assert_eq!(
            cloned.render_optimistic(Some(80)),
            block.render_optimistic(Some(80))
        );
    }

    #[test]
    fn debug_output_contains_type_name() {
        let block = StatusBlock::new(StatusState::Error).body("debug me");
        assert!(format!("{block:?}").contains("StatusBlock"));
    }

    #[test]
    fn empty_body_no_block_quote_in_tree() {
        let block = StatusBlock::new(StatusState::Info)
            .header("Header")
            .hint("Hint only");
        let node = block.render_tree();
        let children = match &node.kind {
            NodeKind::Root { children } => children,
            _ => panic!("expected Root"),
        };
        assert!(
            !children
                .iter()
                .any(|c| matches!(c.kind, NodeKind::BlockQuote { .. }))
        );
    }

    #[test]
    fn render_bespoke_still_available_for_parity() {
        let term = no_color_terminal(80);
        let block = StatusBlock::new(StatusState::Error).body("Body");
        let bespoke = strip_ansi(&block.render_bespoke(&term));
        assert!(bespoke.contains("┃ Body"));
    }

    #[test]
    fn deprecated_failure_serializes_as_error() {
        #[allow(deprecated)]
        let icon = StatusBlock::severity_icon(&StatusState::Failure);
        assert_eq!(icon, "⤫");
    }
}
