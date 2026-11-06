//! A command result assembled from its two sources and any overrides.
//!
//! Every `md` command except the document pair has the same needs: a serialized
//! data value, a document for the generic folds, and sometimes a component's
//! own terminal rendering or today's exact Markdown or HTML bytes. One generic
//! value serves them all, so no command writes its own target `match`.

use biscuit_terminal::render_tree::render_terminal_output;
use renderable::output::{CommandOutput, OutputOptions};
use renderable::tree::{Document, RenderNode, RenderStrictness};
use serde::Serialize;

use crate::markdown::Markdown;

type TerminalFold = Box<dyn Fn(&OutputOptions) -> String>;

/// A [`CommandOutput`] built from its parts.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::output::{OutputTarget, render_output};
/// use darkmatter::markdown::output::command::{CommandValue, prose_document};
/// use renderable::{OutputOptions, TerminalProfile, Width};
///
/// let value = CommandValue::new(vec![1, 2], prose_document("**two** items"));
/// let options = OutputOptions::new(Width::Unlimited, TerminalProfile::plain());
/// assert_eq!(render_output(&value, OutputTarget::Plain, &options)?.text.trim(), "two items");
/// assert!(render_output(&value, OutputTarget::Json, &options)?.text.contains('2'));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct CommandValue<D> {
    data: D,
    prose: Document,
    terminal: Option<TerminalFold>,
    markdown: Option<String>,
    html: Option<String>,
}

impl<D> CommandValue<D> {
    /// A value with `data` behind the data family and `prose` behind every
    /// formatted target.
    #[must_use]
    pub fn new(data: D, prose: Document) -> Self {
        Self { data, prose, terminal: None, markdown: None, html: None }
    }

    /// Overrides `terminal` and `plain`. `fold` receives the plain profile for
    /// `plain`, so it must not read the host.
    #[must_use]
    pub fn with_terminal(mut self, fold: impl Fn(&OutputOptions) -> String + 'static) -> Self {
        self.terminal = Some(Box::new(fold));
        self
    }

    /// Overrides `markdown` and `markdown-plus` with `text`.
    #[must_use]
    pub fn with_markdown(mut self, text: String) -> Self {
        self.markdown = Some(text);
        self
    }

    /// Overrides `html` with `text`.
    #[must_use]
    pub fn with_html(mut self, text: String) -> Self {
        self.html = Some(text);
        self
    }
}

impl<D: Serialize + Clone> CommandOutput for CommandValue<D> {
    type Data = D;

    fn as_prose(&self) -> Document {
        self.prose.clone()
    }

    fn as_data(&self) -> D {
        self.data.clone()
    }

    fn render_terminal(&self, options: &OutputOptions) -> Option<String> {
        self.terminal.as_ref().map(|fold| fold(options))
    }

    fn render_markdown(&self, _options: &OutputOptions) -> Option<String> {
        self.markdown.clone()
    }

    fn render_markdown_plus(&self, _options: &OutputOptions) -> Option<String> {
        self.markdown.clone()
    }

    fn render_html(&self, _options: &OutputOptions) -> Option<String> {
        self.html.clone()
    }
}

/// `markdown` parsed into a document; empty when it does not parse.
#[must_use]
pub fn prose_document(markdown: &str) -> Document {
    Markdown::from(markdown).as_document().unwrap_or_else(|_| empty_document())
}

/// A document with no content.
#[must_use]
pub fn empty_document() -> Document {
    Document::new(RenderNode::root(Vec::new()))
}

/// `source` as one code block tagged `language`.
#[must_use]
pub fn code_document(language: Option<&str>, source: &str) -> Document {
    Document::new(RenderNode::root(vec![RenderNode::code(language.map(str::to_string), None, source)]))
}

/// The generic terminal fold of `document`, which is what any command prints
/// for `plain`.
#[must_use]
pub fn terminal_fold(document: &Document, options: &OutputOptions) -> String {
    render_terminal_output(document, options, RenderStrictness::Warn).map_or_else(|error| error.to_string(), |rendered| rendered.output)
}
