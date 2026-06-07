//! Multi-target rendering for [`Icon`].
//!
//! Browser/markdown targets emit the assembled SVG as a raw inline-HTML node
//! via [`TreeRenderable`]. Terminal rendering uses the degradation ladder
//! implemented through [`TerminalRenderable`] (glyph → image (when the `image`
//! feature is enabled) → text identifier).

use renderable::tree::{RenderNode, TreeRenderable};

use crate::icon::Icon;

impl TreeRenderable for Icon {
    /// Projects the icon into a single inline raw-HTML node carrying the SVG.
    fn render_tree(&self) -> RenderNode {
        RenderNode::root(vec![RenderNode::html(self.svg(), false)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DomainIcon;
    use renderable::tree::render::{
        render_browser_node, render_markdown_node, BrowserRenderOptions, MarkdownDialect,
        MarkdownRenderOptions, RawHtmlPolicy,
    };

    #[test]
    fn browser_target_emits_inline_svg_verbatim() {
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
}
