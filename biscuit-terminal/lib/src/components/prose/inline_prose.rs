//! Public [`InlineProse`] struct and its target trait implementations.

use std::any::Any;

use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::browser::{BrowserRenderable, PageOptions};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{MarkdownDialect, MarkdownRenderOptions, render_markdown_node};
use renderable::tree::{RenderNode, RenderStrictness, TreeRenderable};

use super::LineBreaks;
use super::prose::Prose;
use crate::components::renderable::TerminalRenderable;
use crate::render_tree::{TerminalRenderOptions, render_terminal_node};
use crate::terminal::Terminal;
use crate::utils::layout::Layout;

/// Inline prose: phrasing content parsed from the same grammar as
/// [`Prose`], for text that sits inside a line — a table cell, a list
/// label, a badge, a value in a sentence.
///
/// It holds tags, `**`/`_` emphasis, links, code spans, escapes, and soft
/// and hard breaks, but no blocks: blank lines are ordinary breaks and a
/// fenced code block becomes inline code (language hint dropped, line
/// endings turned into spaces). It has no layout of its own; its container
/// positions and wraps it.
///
/// ```rust
/// use biscuit_terminal::components::prose::InlineProse;
/// use renderable::browser::BrowserRenderable;
/// use renderable::markdown::MarkdownRenderable;
///
/// let label = InlineProse::new("Run `md hash` on [the plan](https://x.io/plan)");
/// assert_eq!(
///     label.render_html_fragment().render(),
///     r#"Run <code>md hash</code> on <a href="https://x.io/plan">the plan</a>"#
/// );
/// assert_eq!(label.render_markdown(), "Run `md hash` on [the plan](https://x.io/plan)");
/// ```
#[derive(Debug, Clone, Default)]
pub struct InlineProse {
    content: String,
    line_breaks: LineBreaks,
    /// Present only because [`TerminalRenderable`] requires a layout; it is
    /// never applied (see the trait impl).
    layout: Layout,
}

impl InlineProse {
    /// Create inline prose from `content`.
    pub fn new<T: Into<String>>(content: T) -> Self {
        Self {
            content: content.into(),
            ..Self::default()
        }
    }

    /// Returns the raw content as received.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set what a newline means. In [`LineBreaks::Soft`] any run of newlines
    /// is one soft break; in [`LineBreaks::Hard`] each newline is a hard
    /// break.
    pub fn with_line_breaks(mut self, line_breaks: LineBreaks) -> Self {
        self.line_breaks = line_breaks;
        self
    }

    /// Escape text so it renders literally; see [`Prose::escape_text`].
    pub fn escape_text(s: &str) -> String {
        Prose::escape_text(s)
    }

    /// Build a safely-quoted tag attribute value; see [`Prose::quoted_attr`].
    pub fn quoted_attr(value: &str) -> String {
        Prose::quoted_attr(value)
    }

    /// The phrasing nodes of this content, for embedding in a container's
    /// render tree.
    #[must_use]
    pub fn to_render_nodes(&self) -> Vec<RenderNode> {
        super::blocks::parse_inline(&self.content, self.line_breaks)
    }

    fn render_markdown_with(&self, dialect: MarkdownDialect) -> String {
        let opts = MarkdownRenderOptions {
            dialect,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&self.render_tree(), &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "InlineProse",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl From<&str> for InlineProse {
    fn from(content: &str) -> Self {
        Self::new(content)
    }
}

impl From<String> for InlineProse {
    fn from(content: String) -> Self {
        Self::new(content)
    }
}

impl TerminalRenderable for InlineProse {
    /// Renders one styled run with no block layout.
    ///
    /// A layout set through the trait's builders is not applied: inline
    /// content is positioned by its container. Setting one logs a warning
    /// rather than being dropped silently.
    fn render(&self, term: &Terminal) -> String {
        if self.layout != Layout::default() {
            tracing::warn!(
                component = "InlineProse",
                "InlineProse has no layout; position it through its container"
            );
        }
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&self.render_tree(), &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "InlineProse",
                    error = %error,
                    "render_terminal_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.render_tree())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

impl MarkdownRenderable for InlineProse {
    /// Inline Markdown with no trailing paragraph break.
    fn render_markdown(&self) -> String {
        self.render_markdown_with(MarkdownDialect::Markdown)
    }

    fn render_markdown_plus(&self) -> String {
        self.render_markdown_with(MarkdownDialect::MarkdownPlus)
    }
}

impl BrowserRenderable for InlineProse {
    /// Phrasing HTML with no outer element: each child node is rendered by
    /// the shared browser renderer and the results are concatenated.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        super::tree::concat_html("InlineProse", self.to_render_nodes())
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
