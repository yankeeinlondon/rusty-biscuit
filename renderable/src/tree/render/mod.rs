//! Target renderers that fold a canonical [`RenderNode`] tree into concrete
//! output formats.
//!
//! Each renderer takes a [`RenderNode`] (or a whole [`Document`]) plus
//! target-specific options and produces a [`Rendered`] value carrying the
//! output and any non-fatal [`Diagnostic`]s, or a fatal [`RenderError`].
//!
//! [`RenderNode`]: crate::tree::RenderNode
//! [`Document`]: crate::tree::Document
//! [`Rendered`]: crate::tree::Rendered
//! [`Diagnostic`]: crate::tree::Diagnostic
//! [`RenderError`]: crate::tree::RenderError

pub mod browser;
pub mod markdown;

pub use browser::{
    BrowserRenderOptions, RawHtmlPolicy, render_browser_document, render_browser_node,
};
pub use markdown::{
    MarkdownDialect, MarkdownRenderOptions, MarkdownStyleOptions, render_markdown_document,
    render_markdown_node,
};

use crate::browser::fragment::{BrowserFragment, Ready};
use crate::tree::NodeAttrs;

/// A hook for custom code-block rendering (e.g. syntax highlighting).
///
/// The terminal and browser tree renderers consult an optional `CodeRenderer`
/// when they meet a [`NodeKind::Code`] node. A renderer that returns `Some`
/// supplies bespoke output; returning `None` lets the tree renderer fall back
/// to its built-in plain code-block rendering.
///
/// ## Architecture note
///
/// The terminal hook takes a primitive `width: u32` rather than a
/// `TerminalRenderContext`. `TerminalRenderContext` lives in `biscuit-terminal`,
/// and `renderable` must not depend on `biscuit-terminal` — the dependency
/// runs the other way. The biscuit-terminal tree renderer extracts `width`
/// from its `TerminalRenderContext` when invoking [`render_terminal_code`].
///
/// [`NodeKind::Code`]: crate::tree::NodeKind::Code
/// [`render_terminal_code`]: CodeRenderer::render_terminal_code
pub trait CodeRenderer {
    /// Renders a code block to a terminal string. Returns `None` to fall
    /// back to plain rendering.
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
        width: u32,
    ) -> Option<String>;

    /// Renders a code block to an HTML fragment. Returns `None` to fall
    /// back to plain rendering.
    fn render_browser_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>>;
}
