//! Multi-target rendering for [`Icon`].
//!
//! Browser/markdown targets emit the assembled SVG as a raw inline-HTML node
//! via [`TreeRenderable`]. Terminal rendering uses the degradation ladder
//! implemented through [`TerminalRenderable`] (glyph → image (when the `image`
//! feature is enabled) → text identifier).

use renderable::tree::{RenderNode, TreeRenderable};

use crate::icon::Icon;

/// Structured payload for the `"icon"` extension token, carrying all rungs of
/// the terminal degradation ladder so the shared renderer can pick the best
/// available representation at render time.
#[derive(serde::Serialize)]
struct IconPayload {
    nerd_font: Option<String>,
    unicode: Option<String>,
    text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg(feature = "image")]
    svg: String,
    #[serde(default)]
    nerd_font_preferred: bool,
}

impl TreeRenderable for Icon {
    /// Projects the icon into a canonical render tree.
    ///
    /// Browser and markdown targets fold the inner `Html` node into inline SVG.
    /// Terminal targets recognize the `"icon"` extension token and walk the
    /// degradation ladder encoded in the JSON payload (Nerd Font → Unicode →
    /// image → text identifier).
    fn render_tree(&self) -> RenderNode {
        #[cfg(feature = "image")]
        let payload = IconPayload {
            nerd_font: self.nerd_font_char().map(|c| c.to_string()),
            unicode: self.unicode_char().map(|c| c.to_string()),
            text: self.id.clone(),
            svg: self.svg(),
            nerd_font_preferred: self.nerd_font,
        };
        #[cfg(not(feature = "image"))]
        let payload = IconPayload {
            nerd_font: self.nerd_font_char().map(|c| c.to_string()),
            unicode: self.unicode_char().map(|c| c.to_string()),
            text: self.id.clone(),
            nerd_font_preferred: self.nerd_font,
        };
        let json = serde_json::to_string(&payload).unwrap_or_default();
        RenderNode::root(vec![RenderNode::extended(
            "icon",
            vec![RenderNode::html(self.svg(), false)],
            Some(json),
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DomainIcon;
    use renderable::tree::RenderStrictness;
    use renderable::tree::render::{
        render_browser_node, render_markdown_node, BrowserRenderOptions, MarkdownDialect,
        MarkdownRenderOptions, RawHtmlPolicy,
    };

    #[test]
    fn render_browser_target_emits_inline_svg_verbatim() {
        let icon = crate::domain::Os::Apple.icon();
        let node = icon.render_tree();
        let opts =
            BrowserRenderOptions { raw_html: RawHtmlPolicy::Allow, ..Default::default() };
        let rendered = render_browser_node(&node, &opts).unwrap();
        let html = rendered.output.render();
        assert!(html.contains("<svg"));
    }

    #[test]
    fn markdown_plus_target_emits_inline_svg() {
        let icon = crate::domain::Os::Apple.icon();
        let node = icon.render_tree();
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..Default::default()
        };
        let rendered = render_markdown_node(&node, &opts).unwrap();
        assert!(rendered.output.contains("<svg"));
    }

    #[test]
    fn markdown_commonmark_strict_rejects_raw_html() {
        let icon = crate::domain::Os::Apple.icon();
        let node = icon.render_tree();
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::Markdown,
            strictness: RenderStrictness::Strict,
            ..Default::default()
        };
        let result = render_markdown_node(&node, &opts);
        assert!(
            result.is_err(),
            "expected error when rendering raw HTML under Markdown with Strict"
        );
    }

    #[test]
    fn terminal_renderable_prefers_unicode_glyph() {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;

        let term = Terminal::new_optimistic(80);
        let out = crate::domain::Emoji::Happy.icon().render(&term);
        assert!(out.contains('\u{1F600}'));
    }

    #[test]
    fn terminal_renderable_uses_id_as_text_fallback() {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::terminal::Terminal;

        let term = Terminal::default();
        let out = crate::domain::Os::Finder.icon().render(&term);
        assert!(out.contains("hugeicons:apple-finder"));
    }

    #[test]
    fn tree_renders_unicode_glyph_to_terminal_through_shared_adapter() {
        use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
        use renderable::tree::TreeRenderable;

        let icon = crate::domain::Emoji::Happy.icon();
        let tree = icon.render_tree();
        let opts = TerminalRenderOptions::default();
        let rendered = render_terminal_node(&tree, &opts).unwrap();

        assert!(
            rendered.output.contains('\u{1F600}'),
            "expected Unicode grinning face in tree-rendered terminal output; got: {}",
            rendered.output
        );
    }

    #[test]
    fn tree_renders_identifier_to_terminal_through_shared_adapter() {
        use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
        use renderable::tree::TreeRenderable;

        let icon = crate::domain::Os::Finder.icon();
        let tree = icon.render_tree();
        let opts = TerminalRenderOptions::default();
        let rendered = render_terminal_node(&tree, &opts).unwrap();

        assert!(
            rendered.output.contains("hugeicons:apple-finder"),
            "expected icon identifier in tree-rendered terminal output; got: {}",
            rendered.output
        );
    }

    #[test]
    fn tree_renders_nerd_font_glyph_to_terminal_through_shared_adapter() {
        use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
        use renderable::tree::TreeRenderable;

        let icon = crate::domain::DevOps::Github.icon().nerd_font(true);
        let tree = icon.render_tree();
        let opts = TerminalRenderOptions::default();
        let rendered = render_terminal_node(&tree, &opts).unwrap();

        assert!(
            rendered.output.contains('\u{f09b}'),
            "expected Nerd Font github glyph in tree-rendered terminal output; got: {}",
            rendered.output
        );
    }

    #[test]
    #[cfg(feature = "image")]
    fn tree_payload_includes_svg_for_image_tier() {
        use renderable::tree::TreeRenderable;

        let icon = crate::domain::Os::Apple.icon();
        let tree = icon.render_tree();

        let root = match &tree.kind {
            renderable::tree::NodeKind::Root { children } => children,
            _ => panic!("expected root node"),
        };
        let ext = match &root[0].kind {
            renderable::tree::NodeKind::Extended { token, payload, .. } => {
                assert_eq!(token, "icon");
                payload
            }
            _ => panic!("expected extended node"),
        };
        let payload = ext.as_ref().expect("expected payload");
        assert!(payload.contains("svg"), "expected SVG field in payload");
        assert!(
            payload.contains("ic:baseline-apple"),
            "expected text identifier in payload"
        );
    }

    #[test]
    #[cfg(not(feature = "image"))]
    fn tree_payload_omits_svg_without_image_feature() {
        use renderable::tree::TreeRenderable;

        let icon = crate::domain::Os::Apple.icon();
        let tree = icon.render_tree();

        let root = match &tree.kind {
            renderable::tree::NodeKind::Root { children } => children,
            _ => panic!("expected root node"),
        };
        let ext = match &root[0].kind {
            renderable::tree::NodeKind::Extended { token, payload, .. } => {
                assert_eq!(token, "icon");
                payload
            }
            _ => panic!("expected extended node"),
        };
        let payload = ext.as_ref().expect("expected payload");
        assert!(!payload.contains("svg"), "expected no SVG field in payload without image feature");
        assert!(
            payload.contains("ic:baseline-apple"),
            "expected text identifier in payload"
        );
    }
}
