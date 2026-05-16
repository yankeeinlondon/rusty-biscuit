use crate::stylesheet::Stylesheet;

/// A component capable of rendering itself as Markdown output.
///
/// Markdown is a superset of HTML: components that lower cleanly to
/// ergonomic Markdown implement [`render_markdown`](MarkdownRenderable::render_markdown);
/// components that need richer styling can consume a [`Stylesheet`] via
/// [`render_markdown_with_style`](MarkdownRenderable::render_markdown_with_style)
/// and project the Markdown-addressable rules into the output.
///
/// ## Notes
///
/// - `render_markdown_with_style` defaults to ignoring the stylesheet
///   and delegating to `render_markdown`, so a component opts into
///   style-aware Markdown only when it has something to do with it.
pub trait MarkdownRenderable {
    /// Renders the component as a Markdown string.
    fn render_markdown(&self) -> String;

    /// Renders the component as Markdown, optionally consuming a
    /// [`Stylesheet`] for style-aware output.
    ///
    /// The default ignores `style` and delegates to
    /// [`render_markdown`](MarkdownRenderable::render_markdown).
    fn render_markdown_with_style(&self, _style: Option<Stylesheet>) -> String {
        self.render_markdown()
    }
}
