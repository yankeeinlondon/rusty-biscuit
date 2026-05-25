//! Prose → render-tree inline node projection.
//!
//! The [`ProseDocument`] IR is private to the `prose` module; this file
//! lowers it into the canonical [`RenderNode`] inline shape used by
//! [`renderable::tree`]. Containers that embed a [`Prose`] component (today:
//! [`BlockQuote`](crate::components::block_quote::BlockQuote)) can call
//! [`Prose::to_render_nodes`] to project a styled paragraph that the
//! terminal tree renderer lowers back to SGR — preserving the inline
//! `<b>` / `<i>` / `<red>` styling that the user authored.
//!
//! Mapping:
//! - `ProseNode::Text` → `NodeKind::Text`
//! - `ProseNode::Span` with bold/italic/strikethrough only → semantic
//!   `NodeKind::Strong` / `NodeKind::Emphasis` / `NodeKind::Delete`
//!   wrappers (nested in that order so a single span carrying multiple
//!   emphasis flags still expresses each one).
//! - `ProseNode::Span` carrying color/background/dim/blink/underline →
//!   `NodeKind::Span` with a [`Style`] attached on `attrs` so the terminal
//!   renderer's `render_inline_node` path lowers it to SGR via
//!   `text_appearance_sgr`.
//! - `ProseNode::Link` → `NodeKind::Link` with un-resolved `href` (each
//!   target re-resolves per its own rules, matching the existing Prose
//!   emitters).
//! - `ProseNode::CodeBlock` → `NodeKind::Code` (block-level). Code blocks
//!   only appear at top level in practice, but we preserve them so a
//!   future container that allows them gets the right tree shape.
//!
//! `inverse` and `hidden` from `ProseStyle` have no semantic peer in
//! `Style` and are intentionally dropped — they were Prose-only knobs.

use renderable::color::Color;
use renderable::layout::TargetValue;
use renderable::style::{PerMode, Style, TextEmphasis};
use renderable::tree::RenderNode;

use super::ir::{ProseDocument, ProseNode, ProseStyle};
use super::prose::Prose;

impl Prose {
    /// Projects this prose into a sequence of inline [`RenderNode`]s.
    ///
    /// Use this when embedding `Prose` content inside a container
    /// component's render-tree projection so the canonical render tree
    /// carries the styled inline structure (and the terminal tree
    /// renderer lowers it back to SGR) instead of a flattened plain-text
    /// blob.
    #[must_use]
    pub fn to_render_nodes(&self) -> Vec<RenderNode> {
        let doc: ProseDocument = self.document();
        doc.children.iter().map(node_to_render_node).collect()
    }
}

/// Wraps `color` as the `Style` color slot shape — terminal target,
/// universal across light/dark.
fn universal_color(color: Color) -> TargetValue<PerMode<Color>> {
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

/// Returns `true` when `style` has no semantic inline peer in the tree.
///
/// `inverse` and `hidden` were Prose-only knobs and have no representation
/// in the canonical `Style`; we surface them as plain text rather than
/// dropping them inside a styled wrapper.
fn is_prose_only(style: &ProseStyle) -> bool {
    style.emphasis.is_empty()
        && style.fg.is_none()
        && style.bg.is_none()
        && (style.inverse || style.hidden)
}

/// Project a single `ProseNode` into a `RenderNode`.
fn node_to_render_node(node: &ProseNode) -> RenderNode {
    match node {
        ProseNode::Text(value) => RenderNode::text(value),
        ProseNode::Link { href, children } => {
            let kids: Vec<RenderNode> = children.iter().map(node_to_render_node).collect();
            RenderNode::link(href, None, kids)
        }
        ProseNode::CodeBlock { lang, value } => {
            // `Code` is block-level; in the inline context we currently
            // emit it the renderable tree call expects. The Prose IR only
            // produces CodeBlock at top level, so a downstream rendering
            // of an inline-stream-with-a-code-block reads as a fenced
            // block in Markdown / `<pre><code>` in Browser / dim block in
            // Terminal — the established mapping.
            RenderNode::code(lang.clone(), None, value.clone())
        }
        ProseNode::Span { style, children } => {
            let kids: Vec<RenderNode> = children.iter().map(node_to_render_node).collect();
            project_span(style, kids)
        }
    }
}

/// Project a `ProseNode::Span` into a render-tree inline node.
///
/// Pure emphasis styles map onto semantic wrappers (Strong / Emphasis /
/// Delete) for the cleanest cross-target output. Anything richer — colors,
/// dim, blink, underline variants — rides on a `NodeKind::Span` with a
/// `Style` attached, which the terminal renderer lowers through
/// `text_appearance_sgr`.
fn project_span(style: &ProseStyle, children: Vec<RenderNode>) -> RenderNode {
    // Prose-only knobs with no inline peer in `Style` (`inverse`, `hidden`):
    // drop the wrapper and emit children as a bare `Span` carrying nothing.
    if is_prose_only(style) {
        return RenderNode::span(Vec::new(), children);
    }

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

/// `true` when only bold/italic/strikethrough are set — no dim, blink, or
/// underline variant.
fn pure_semantic_emphasis(em: &TextEmphasis) -> bool {
    !em.dim && !em.blink && em.underline.is_none()
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
        let nodes = Prose::new("hello world").to_render_nodes();
        assert_eq!(nodes.len(), 1);
        match &nodes[0].kind {
            NodeKind::Text { value } => assert_eq!(value, "hello world"),
            other => panic!("expected text node, got {other:?}"),
        }
    }

    #[test]
    fn bold_tag_projects_to_strong_node() {
        let nodes = Prose::new("<b>x</b>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Strong { .. }));
    }

    #[test]
    fn italic_tag_projects_to_emphasis_node() {
        let nodes = Prose::new("<i>x</i>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Emphasis { .. }));
    }

    #[test]
    fn strikethrough_tag_projects_to_delete_node() {
        let nodes = Prose::new("<~>x</~>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Delete { .. }));
    }

    #[test]
    fn red_color_projects_to_styled_span() {
        let nodes = Prose::new("<red>x</red>").to_render_nodes();
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
        let nodes = Prose::new("<red-500>x</red-500>").to_render_nodes();
        assert!(matches!(first(&nodes).kind, NodeKind::Span { .. }));
        let style = first(&nodes).attrs.style().expect("style attached");
        assert!(matches!(
            style.color,
            Some(TargetValue::Universal(PerMode::Universal(Color::Tailwind(
                Tailwind::Red500
            ))))
        ));
    }

    #[test]
    fn link_projects_to_link_node() {
        let nodes = Prose::new("<a href=\"https://example.com\">go</a>").to_render_nodes();
        match &first(&nodes).kind {
            NodeKind::Link { url, .. } => assert_eq!(url, "https://example.com"),
            other => panic!("expected link node, got {other:?}"),
        }
    }

    #[test]
    fn nested_bold_italic_nests_strong_then_emphasis() {
        let nodes = Prose::new("<b><i>x</i></b>").to_render_nodes();
        match &first(&nodes).kind {
            NodeKind::Strong { children } => {
                assert!(matches!(children[0].kind, NodeKind::Emphasis { .. }))
            }
            other => panic!("expected strong wrapper, got {other:?}"),
        }
    }

    #[test]
    fn mixed_text_and_styled_run_preserves_order() {
        let nodes = Prose::new("plain <b>bold</b> tail").to_render_nodes();
        assert_eq!(nodes.len(), 3);
        assert!(matches!(nodes[0].kind, NodeKind::Text { .. }));
        assert!(matches!(nodes[1].kind, NodeKind::Strong { .. }));
        assert!(matches!(nodes[2].kind, NodeKind::Text { .. }));
    }
}
