//! Darkmatter's [`CodeRenderer`] implementation for the `renderable` render tree.
//!
//! The render-tree terminal and browser renderers fall back to a plain fenced
//! code block when they meet a [`NodeKind::Code`](renderable::tree::NodeKind::Code)
//! node. [`TerminalCodeRenderer`] wires darkmatter's syntax-highlighted
//! code-block path into that hook so render-tree output matches the bespoke
//! [`YamlBlock`](crate::markdown::YamlBlock) (and Markdown code fence) renderer.

use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::color::{ColorMode as RenderableColorMode, TerminalCodeContext};
use renderable::tree::{CodeRenderer, NodeAttrs};

use crate::markdown::{
    dsl::CodeBlockMeta,
    highlighting::{CodeHighlighter, ColorMode},
    output::code_block::{render_html_code_block, render_terminal_code_block},
    output::html::HtmlOptions,
    output::terminal::{TerminalOptions, format_header_row},
};

/// Darkmatter's [`CodeRenderer`] hook for the render tree.
///
/// Reproduces the bespoke darkmatter code-block output (header row plus
/// syntax-highlighted body) so the render-tree path stays byte-compatible with
/// the bespoke [`YamlBlock`](crate::markdown::YamlBlock) renderer.
///
/// ## Notes
///
/// The terminal hook produces only the `{header}\n{body}` string; it does not
/// apply layout. The render-tree renderer applies node layout (margins,
/// alignment, word-wrap) separately, so applying it here would double it.
#[derive(Debug, Default, Clone, Copy)]
pub struct TerminalCodeRenderer;

impl TerminalCodeRenderer {
    /// Creates a new [`TerminalCodeRenderer`].
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Maps a `renderable` [`ColorMode`](RenderableColorMode) onto darkmatter's
/// highlighting [`ColorMode`].
///
/// Per the [`CodeRenderer`] no-color contract, `Unknown` resolves to `Dark`
/// (the renderer's configured default) without ambient detection.
fn map_color_mode(mode: RenderableColorMode) -> ColorMode {
    match mode {
        RenderableColorMode::Light => ColorMode::Light,
        RenderableColorMode::Dark | RenderableColorMode::Unknown => ColorMode::Dark,
    }
}

impl CodeRenderer for TerminalCodeRenderer {
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
        context: TerminalCodeContext,
    ) -> Option<String> {
        let hints = attrs.code_hints();
        let color_mode = map_color_mode(context.color_mode());
        let options = TerminalOptions::default();
        let highlighter = CodeHighlighter::new(options.code_theme, color_mode);
        let meta = CodeBlockMeta::default();
        let language = lang.unwrap_or("");

        // Body: the syntax-highlighted code block. `target_width = None`
        // matches the bespoke renderer, which lets the render-tree layout
        // pass own width handling.
        let body = render_terminal_code_block(
            value,
            language,
            &highlighter,
            &options,
            &meta,
            color_mode,
            None,
        )
        .ok()?;

        // Header: emitted only when the projection requested one.
        if hints.header_row {
            let label = hints.language_label.as_deref().unwrap_or(language);
            let bg_color = highlighter
                .theme()
                .settings
                .background
                .unwrap_or(syntect::highlighting::Color::BLACK);
            let header = format_header_row(
                meta.title.as_deref(),
                label,
                bg_color,
                color_mode,
                context.width() as u16,
            );
            Some(format!("{header}\n{body}"))
        } else {
            Some(body)
        }
    }

    fn render_browser_code(
        &self,
        lang: Option<&str>,
        value: &str,
        _attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>> {
        let options = HtmlOptions::default();
        let highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);
        let meta = CodeBlockMeta::default();
        let language = lang.unwrap_or("");

        let html =
            render_html_code_block(value, language, &meta, &highlighter, &options).ok()?;
        Some(
            BrowserFragment::new()
                .define_as_raw_html(html)
                .finalize(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::color::ColorDepth;
    use renderable::tree::CodeRenderHints;

    fn yaml_attrs() -> NodeAttrs {
        let mut attrs = NodeAttrs::default();
        attrs.set_code_hints(&CodeRenderHints {
            header_row: true,
            language_label: Some("yaml".into()),
            highlight: true,
        });
        attrs
    }

    #[test]
    fn terminal_code_emits_header_and_highlighted_body() {
        let renderer = TerminalCodeRenderer::new();
        let context = TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark);
        let out = renderer
            .render_terminal_code(Some("yaml"), "foo: 1\nbar: 2", &yaml_attrs(), context)
            .expect("renders");

        assert!(out.contains(" yaml "), "header label missing: {out:?}");
        assert!(out.contains("\x1b["), "expected ANSI styling");
        let plain = crate::testing::strip_ansi_codes(&out);
        assert!(plain.contains("foo: 1") && plain.contains("bar: 2"));
    }

    #[test]
    fn terminal_code_omits_header_when_not_requested() {
        let renderer = TerminalCodeRenderer::new();
        let context = TerminalCodeContext::new(80, ColorDepth::TrueColor, RenderableColorMode::Dark);
        let out = renderer
            .render_terminal_code(Some("yaml"), "foo: 1", &NodeAttrs::default(), context)
            .expect("renders");
        assert!(!out.contains(" yaml "), "header should be absent: {out:?}");
    }

    #[test]
    fn browser_code_emits_language_class() {
        let renderer = TerminalCodeRenderer::new();
        let fragment = renderer
            .render_browser_code(Some("yaml"), "foo: 1", &NodeAttrs::default())
            .expect("renders");
        assert!(fragment.render().contains("language-yaml"));
    }
}
