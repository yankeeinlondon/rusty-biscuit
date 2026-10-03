//! [`Prose`] and [`InlineProse`] → render-tree projection and the
//! [`TreeRenderable`] impls.
//!
//! Both components parse the shared grammar ([`super::blocks`]) straight into
//! canonical [`RenderNode`]s; there is no intervening component-local IR.
//! `Prose` projects to a `Root` of `Paragraph` and `Code` blocks carrying its
//! layout; `InlineProse` projects to one neutral `Span` of phrasing nodes.
//!
//! The tag → node mapping (applied by the parser via [`project_span`] and the
//! `RenderNode` constructors):
//! - literal text → `NodeKind::Text`
//! - newlines → `NodeKind::SoftBreak` / `NodeKind::HardBreak`
//! - bold/italic/strikethrough only → semantic `NodeKind::Strong` /
//!   `NodeKind::Emphasis` / `NodeKind::Delete` wrappers (nested in that order
//!   so a single span carrying multiple emphasis flags still expresses each
//!   one).
//! - color/background/dim/blink/underline/inverse → `NodeKind::Span` with a
//!   [`Style`] attached on `attrs` so the terminal renderer's
//!   `render_inline_node` path lowers it to SGR via `text_appearance_sgr`.
//! - links → `NodeKind::Link` with un-resolved `href` (each target re-resolves
//!   per its own rules).
//! - code spans → `NodeKind::InlineCode`
//! - fenced code blocks → `NodeKind::Code` in `Prose`, `NodeKind::InlineCode`
//!   in `InlineProse`.
//!
//! `<inverse>` / `<reverse>` carry `TextEmphasis::inverse`. `<hidden>` has no
//! semantic peer and is dropped by the parser to inert literal text, so it
//! never reaches this projection.

use renderable::color::Color;
use renderable::layout::{Layout, TargetValue};
use renderable::style::{PaintColor, PerMode, Style, TextEmphasis};
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::tree::render::{BrowserRenderOptions, render_browser_node};
use renderable::tree::{NodeKind, RenderNode, TreeRenderable};

use super::inline_prose::InlineProse;
use super::prose::Prose;
use super::styles::ProseStyle;

impl Prose {
    /// The block nodes of this prose: a `Paragraph` per paragraph (carrying
    /// the [`ProseTag`](super::ProseTag) element) and a `Code` per fenced block.
    pub(crate) fn block_nodes(&self) -> Vec<RenderNode> {
        let element = self.tag.into();
        let mut blocks = super::blocks::parse_blocks(self.content(), self.line_breaks);
        for block in &mut blocks {
            if matches!(block.kind, NodeKind::Paragraph { .. }) {
                block.attrs.browser_mut_or_default().block_element = element;
                block.attrs.retain_non_default_browser();
            }
        }
        blocks
    }

    /// Interim bridge for containers that still expect a flat inline
    /// sequence: each paragraph's children, consecutive paragraphs joined by
    /// a literal blank line, and fenced blocks as block-level `Code` nodes
    /// for the container's own folding.
    ///
    /// Containers are re-typed onto `InlineProse` or block children (with
    /// layout transfer) next; this goes away with that change.
    pub(crate) fn interim_container_nodes(&self) -> Vec<RenderNode> {
        let mut out = Vec::new();
        let mut previous_was_paragraph = false;
        for block in super::blocks::parse_blocks(self.content(), self.line_breaks) {
            match block.kind {
                NodeKind::Paragraph { children } => {
                    if previous_was_paragraph {
                        out.push(RenderNode::text("\n\n"));
                    }
                    out.extend(children);
                    previous_was_paragraph = true;
                }
                _ => {
                    out.push(block);
                    previous_was_paragraph = false;
                }
            }
        }
        out
    }
}

impl TreeRenderable for Prose {
    /// Projects the prose into a `Root` of `Paragraph` and `Code` blocks,
    /// with the prose layout on the root. Empty and whitespace-only content
    /// gives a `Root` with no children.
    fn render_tree(&self) -> RenderNode {
        let mut root = RenderNode::root(self.block_nodes());
        if self.layout != Layout::default() {
            root.attrs.set_layout(&self.layout);
        }
        root
    }

    /// Surfaces Prose's layout (margins, alignment, word wrap) to the tree
    /// renderers.
    fn tree_layout(&self) -> Option<Layout> {
        if self.layout != Layout::default() {
            Some(self.layout.clone())
        } else {
            None
        }
    }
}

/// Render each node with the shared browser renderer and concatenate the
/// results with no outer element.
pub(super) fn concat_html(component: &'static str, nodes: Vec<RenderNode>) -> BrowserFragment<Ready> {
    let opts = BrowserRenderOptions::default();
    let mut html = String::new();
    for node in nodes {
        match render_browser_node(&node, &opts) {
            Ok(rendered) => html.push_str(&rendered.output.render()),
            Err(error) => tracing::error!(
                component,
                error = %error,
                "render_browser_node failed; skipping node"
            ),
        }
    }
    BrowserFragment::new().define_as_raw_html(html).finalize()
}

impl TreeRenderable for InlineProse {
    /// Projects the inline content into one neutral `Span` holding its
    /// phrasing nodes, also when the content is empty.
    fn render_tree(&self) -> RenderNode {
        RenderNode::span(Vec::new(), self.to_render_nodes())
    }
}

/// Wraps `color` as the `Style` color slot shape — terminal target,
/// universal across light/dark.
fn universal_color(color: Color) -> TargetValue<PerMode<PaintColor>> {
    TargetValue::universal(PerMode::universal(color))
}

/// Builds an inline [`Style`] from a [`ProseStyle`].
///
/// Maps the emphasis leaf 1:1 and lifts foreground/background colors into
/// the matching `Style` slots. Returns `None` when the prose style is
/// purely the default (no styling at all) — the caller should then emit
/// the children as bare text.
fn style_from_prose(prose_style: &ProseStyle) -> Option<Style> {
    let mut style = Style::default();
    let mut any = false;

    // Lift emphasis leaf
    if !prose_style.emphasis.is_empty() {
        style.emphasis = prose_style.emphasis;
        any = true;
    }
    if let Some(fg) = prose_style.fg {
        style.color = Some(universal_color(fg));
        any = true;
    }
    if let Some(bg) = prose_style.bg {
        style.background = Some(universal_color(bg));
        any = true;
    }

    if any { Some(style) } else { None }
}

/// Project a styled Prose span into a render-tree inline node.
///
/// Pure emphasis styles map onto semantic wrappers (Strong / Emphasis /
/// Delete) for the cleanest cross-target output. Anything richer — colors,
/// dim, blink, underline variants, inverse — rides on a `NodeKind::Span` with
/// a `Style` attached, which the terminal renderer lowers through
/// `text_appearance_sgr`.
pub(super) fn project_span(style: &ProseStyle, children: Vec<RenderNode>) -> RenderNode {
    // If the style is purely semantic emphasis (bold / italic /
    // strikethrough — no color, no underline/dim/blink), emit the
    // canonical Strong / Emphasis / Delete wrappers so Markdown and
    // Browser produce semantic output.
    if style.fg.is_none() && style.bg.is_none() && pure_semantic_emphasis(&style.emphasis) {
        return wrap_semantic_emphasis(&style.emphasis, children);
    }

    // Otherwise lower the whole `ProseStyle` onto an inline `NodeKind::Span`
    // carrying a `Style` attribute; the terminal renderer's `render_inline_node`
    // arm for `NodeKind::Span` applies `text_appearance_sgr` for us.
    match style_from_prose(style) {
        Some(style) => {
            let mut node = RenderNode::span(Vec::new(), children);
            node.attrs.set_style(&style);
            node
        }
        None => RenderNode::span(Vec::new(), children),
    }
}

/// Project a styled Prose span whose children may include a block-level node,
/// pushing the result(s) onto `out`.
///
/// The render tree forbids a block-level `Code` node inside a phrasing-only
/// `Span`, so a fenced code block nested in a styled span — e.g.
/// `<red>before ```code``` after</red>` — cannot ride inside one span wrapper
/// (the validator would reject it and the terminal renderer would emit empty
/// output). The span is split around each block child: every contiguous inline
/// run is wrapped by [`project_span`], restoring the enclosing style on both
/// sides of the block, and the block child is emitted as a sibling.
///
/// `Code` is the only block-level node the Prose parser produces; all other
/// children (text, links, nested spans) accumulate into the surrounding run.
pub(super) fn project_styled_span(
    style: &ProseStyle,
    children: Vec<RenderNode>,
    out: &mut Vec<RenderNode>,
) {
    let mut run: Vec<RenderNode> = Vec::new();
    for child in children {
        if matches!(child.kind, NodeKind::Code { .. }) {
            if !run.is_empty() {
                out.push(project_span(style, std::mem::take(&mut run)));
            }
            out.push(child);
        } else {
            run.push(child);
        }
    }
    if !run.is_empty() {
        out.push(project_span(style, run));
    }
}

/// `true` when only bold/italic/strikethrough are set — no dim, blink, or
/// underline variant.
fn pure_semantic_emphasis(em: &TextEmphasis) -> bool {
    !em.dim && !em.blink && !em.inverse && em.underline.is_none()
}

/// Nests `Strong` (bold) > `Emphasis` (italic) > `Delete` (strikethrough)
/// wrappers around `children` per the flags set on `em`.
///
/// Order matches the terminal renderer's inline projection so the
/// resulting Markdown reads as `***bold italic***` rather than a permuted
/// nesting.
fn wrap_semantic_emphasis(em: &TextEmphasis, children: Vec<RenderNode>) -> RenderNode {
    let mut inner = children;
    if em.strikethrough {
        inner = vec![RenderNode::delete(inner)];
    }
    if em.italic {
        inner = vec![RenderNode::emphasis(inner)];
    }
    if em.bold {
        inner = vec![RenderNode::strong(inner)];
    }
    // Exactly one wrapper survives at the outermost layer when only one
    // flag is set; when none are set the caller has already excluded that
    // case via `style_from_prose`. Multi-flag spans return the nested
    // wrappers via the surrounding `synthetic_span` indirection only if
    // we wrap >1 — but the explicit re-wrapping above always returns a
    // single node when at least one flag is set.
    if inner.len() == 1 {
        inner.into_iter().next().unwrap()
    } else {
        // Defensive: no semantic flags set — surface the children inside a
        // bare span so the caller still receives a single node.
        RenderNode::span(Vec::new(), inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::color::{Color, Tailwind};
    use renderable::tree::NodeKind;

    fn first(nodes: &[RenderNode]) -> &RenderNode {
        nodes.first().expect("at least one node")
    }

    #[test]
    fn plain_text_projects_to_text_node() {
        let nodes = InlineProse::new("hello world").to_render_nodes();
        assert_eq!(nodes.len(), 1);
        match &nodes[0].kind {
            NodeKind::Text { value } => assert_eq!(value, "hello world"),
            other => panic!("expected text node, got {other:?}"),
        }
    }

    #[test]
    fn bold_tag_projects_to_strong_node() {
        let nodes = InlineProse::new("<b>x</b>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Strong { .. }));
    }

    #[test]
    fn italic_tag_projects_to_emphasis_node() {
        let nodes = InlineProse::new("<i>x</i>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Emphasis { .. }));
    }

    #[test]
    fn strikethrough_tag_projects_to_delete_node() {
        let nodes = InlineProse::new("<~>x</~>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Delete { .. }));
    }

    #[test]
    fn red_color_projects_to_styled_span() {
        let nodes = InlineProse::new("<red>x</red>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Span { .. }));
        let style = first(&nodes).attrs.style().expect("style attached");
        assert_eq!(
            style.color,
            Some(universal_color(Color::BasicColor(
                renderable::color::BasicColor::Red
            )))
        );
    }

    #[test]
    fn tailwind_color_projects_to_styled_span() {
        let nodes = InlineProse::new("<red-500>x</red-500>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Span { .. }));
        let style = first(&nodes).attrs.style().expect("style attached");
        assert!(matches!(
            style.color,
            Some(TargetValue::Universal(PerMode::Universal(PaintColor {
                color: Color::Tailwind(Tailwind::Red500),
                ..
            })))
        ));
    }

    #[test]
    fn link_projects_to_link_node() {
        let nodes = InlineProse::new("<a href=\"https://example.com\">go</a>").to_render_nodes();
        match &first(&nodes).kind {
            NodeKind::Link { url, .. } => assert_eq!(url, "https://example.com"),
            other => panic!("expected link node, got {other:?}"),
        }
    }

    #[test]
    fn nested_bold_italic_nests_strong_then_emphasis() {
        let nodes = InlineProse::new("<b><i>x</i></b>").to_render_nodes();
        match &first(&nodes).kind {
            NodeKind::Strong { children } => {
                assert!(matches!(children[0].kind, NodeKind::Emphasis { .. }))
            }
            other => panic!("expected strong wrapper, got {other:?}"),
        }
    }

    #[test]
    fn mixed_text_and_styled_run_preserves_order() {
        let nodes = InlineProse::new("plain <b>bold</b> tail").to_render_nodes();
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0].kind, NodeKind::Text { .. }));
        assert!(matches!(nodes[1].kind, NodeKind::Strong { .. }));
        assert!(matches!(nodes[2].kind, NodeKind::Text { .. }));
    }
}
