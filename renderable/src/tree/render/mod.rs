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
mod shared;

pub use browser::{
    BrowserDocumentBody, BrowserRenderOptions, RawHtmlPolicy, render_browser_document,
    render_browser_document_body, render_browser_document_html, render_browser_node,
};
pub use markdown::{
    MarkdownDialect, MarkdownRenderOptions, MarkdownStyleOptions, render_markdown_document,
    render_markdown_node,
};

use crate::browser::fragment::{BrowserFragment, Ready};
use crate::color::TerminalCodeContext;
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
/// The terminal hook takes a [`TerminalCodeContext`] containing width, color
/// depth, and color mode. This struct lives in `renderable::color` so that
/// `CodeRenderer` can accept it without depending on `biscuit-terminal` — the
/// dependency runs the other way. `biscuit-terminal` populates the context
/// from its `TerminalRenderContext` and passes it through.
///
/// ## No-color contract
///
/// - Implementors SHOULD treat [`ColorDepth::None`] as "emit no ANSI styling."
/// - If unable to honor the supplied context, implementors SHOULD return `None`
///   to let the tree renderer fall back to plain rendering.
/// - Implementors MUST NOT perform ambient capability detection (reading
///   `$COLORTERM`, `$NO_COLOR`, etc.). All capability information is passed
///   explicitly via [`TerminalCodeContext`].
///
/// [`NodeKind::Code`]: crate::tree::NodeKind::Code
/// [`render_terminal_code`]: CodeRenderer::render_terminal_code
/// [`ColorDepth::None`]: crate::color::ColorDepth::None
pub trait CodeRenderer {
    /// Renders a code block to a terminal string.
    ///
    /// Returns `Some(output)` with styled terminal output, or `None` to fall
    /// back to plain rendering.
    ///
    /// ## Arguments
    ///
    /// - `lang`: The language tag (e.g., `"rust"`, `"yaml"`), if any.
    /// - `value`: The raw code content.
    /// - `meta`: The fenced code-block info string beyond the language token
    ///   (e.g. `title="…" line-numbering=true highlight=2`), if any. A renderer
    ///   that understands the directive may use it for titles, line numbering,
    ///   and highlighted-line ranges; renderers that do not understand it should
    ///   ignore it.
    /// - `attrs`: Node attributes including optional code render hints.
    /// - `context`: Terminal capability context (width, color depth, mode).
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        meta: Option<&str>,
        attrs: &NodeAttrs,
        context: TerminalCodeContext,
    ) -> Option<String>;

    /// Renders a code block to an HTML fragment.
    ///
    /// Returns `Some(fragment)` with styled HTML output, or `None` to fall
    /// back to plain rendering.
    ///
    /// ## Arguments
    ///
    /// - `lang`: The language tag (e.g., `"rust"`, `"yaml"`), if any.
    /// - `value`: The raw code content.
    /// - `meta`: The fenced code-block info string beyond the language token
    ///   (title / line-numbering / highlight directive), if any.
    /// - `attrs`: Node attributes including optional code render hints.
    fn render_browser_code(
        &self,
        lang: Option<&str>,
        value: &str,
        meta: Option<&str>,
        attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>>;

    /// Promotes a Mermaid diagram to a static SVG fragment.
    ///
    /// This is deliberately separate from [`render_browser_code`] so the browser
    /// renderer can tell a *successful* SVG promotion from a *failed* one: a
    /// `None` here means "I could not produce an SVG", which lets the renderer
    /// apply its strictness policy (reject under `Strict`, diagnose and fall
    /// back under `Warn`). Folding Mermaid into `render_browser_code` would
    /// overload `Some(fragment)` — an implementor that catches its own SVG
    /// failure and returns a code-block fragment would hide the failure from the
    /// renderer and silently bypass strictness.
    ///
    /// The default implementation returns `None` (no Mermaid support), so the
    /// renderer degrades per its strictness policy.
    ///
    /// ## Arguments
    ///
    /// - `value`: The raw Mermaid diagram source.
    /// - `meta`: The fenced code-block info string beyond the language token.
    /// - `attrs`: Node attributes including optional code render hints.
    ///
    /// [`render_browser_code`]: CodeRenderer::render_browser_code
    fn render_browser_mermaid(
        &self,
        value: &str,
        meta: Option<&str>,
        attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>> {
        let _ = (value, meta, attrs);
        None
    }
}
