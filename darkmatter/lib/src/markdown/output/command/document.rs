//! The document result of `md compose`, `md render`, `md clean`, and `md set`.

use std::borrow::Cow;

use biscuit_terminal::output::{OutputTarget, terminal_target_options};
use renderable::output::{CommandOutput, OutputOptions, Width};
use renderable::tree::{Document, RenderNode};
use serde::Serialize;

use crate::layout::{DarkmatterPage, PageRenderError};
use crate::markdown::Markdown;
use crate::markdown::cleanup::{reflow_to_width, strip_incidental_newlines};
use crate::markdown::compose::expression::Diagnostic;
use crate::markdown::reference::validate::ReferenceValidationReport;

/// Whether the command succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// The document was produced.
    Success,
    /// The command failed; `diagnostics` says why.
    Failure,
}

/// One timed stage of a run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerfStage {
    /// The stage's name.
    pub name: String,
    /// How long it took.
    pub milliseconds: f64,
}

/// The timings a run collected.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerfData {
    /// The stages, in execution order.
    pub stages: Vec<PerfStage>,
}

/// Text a lifecycle step wrote while a data output owned stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleOutputEntry {
    /// The lifecycle stage.
    pub stage: String,
    /// The step within the stage.
    pub step: String,
    /// What the step wrote.
    pub text: String,
}

/// The reference check's totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReferenceSummary {
    /// References checked.
    pub references: usize,
    /// Distinct documents visited.
    pub documents: usize,
    /// References without an error.
    pub ok: usize,
    /// References with an error.
    pub broken: usize,
}

impl ReferenceSummary {
    /// The same counts the validation report's summary line shows.
    #[must_use]
    pub fn from_report(report: &ReferenceValidationReport) -> Self {
        let references = report.references_scanned;
        let broken = report.error_count();
        Self {
            references,
            documents: report.document_count(),
            ok: references.saturating_sub(broken),
            broken,
        }
    }
}

/// The compose/render data envelope.
///
/// Field order is the serialized order. `perf` is omitted when `None`; every
/// other `None` serializes as null, so a reader can rely on the keys.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocumentEnvelope {
    /// The input file, when there is one.
    pub file: Option<String>,
    /// Whether the command succeeded.
    pub outcome: Outcome,
    /// The Markdown the matching formatted output prints.
    pub document: Option<String>,
    /// Every finding of the run.
    pub diagnostics: Vec<Diagnostic>,
    /// The reference check's totals, when it ran.
    pub summary: Option<ReferenceSummary>,
    /// Timings, when collected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perf: Option<PerfData>,
    /// Output lifecycle steps wrote, in order.
    pub lifecycle_output: Vec<LifecycleOutputEntry>,
}

impl CommandOutput for DocumentEnvelope {
    type Data = Self;

    fn as_prose(&self) -> Document {
        self.document.as_deref().map_or_else(empty_document, prose_of)
    }

    fn as_data(&self) -> Self {
        self.clone()
    }
}

fn empty_document() -> Document {
    Document::new(RenderNode::root(Vec::new()))
}

fn prose_of(markdown: &str) -> Document {
    Markdown::from(markdown).as_document().unwrap_or_else(|_| empty_document())
}

/// The Markdown a document command prints: frontmatter when
/// `include_frontmatter`, then the body under `width`.
///
/// | `width` | Body |
/// |---|---|
/// | `Source` | unchanged |
/// | `Columns(n)` | reflowed to `n` columns |
/// | `Unlimited` | paragraphs unwrapped |
///
/// The frontmatter is never reflowed.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::output::command::document_markdown;
/// use renderable::output::Width;
///
/// let document = Markdown::from("---\ntitle: A\n---\none\ntwo\n");
/// assert_eq!(document_markdown(&document, false, Width::Unlimited), "one two");
/// assert_eq!(document_markdown(&document, true, Width::Source), "---\ntitle: A\n---\none\ntwo");
/// ```
#[must_use]
pub fn document_markdown(document: &Markdown, include_frontmatter: bool, width: Width) -> String {
    let content = document.content();
    let body = match width {
        Width::Source => Cow::Borrowed(content),
        Width::Columns(columns) => Cow::Owned(reflow_to_width(content, columns.max(1) as usize)),
        Width::Unlimited => Cow::Owned(strip_incidental_newlines(content)),
    };
    if !include_frontmatter {
        return body.into_owned();
    }
    let full = document.as_string();
    let frontmatter = full.strip_suffix(content).unwrap_or_default();
    format!("{frontmatter}{body}")
}

/// The one formatted output a [`DocumentOutput`] was prepared for.
#[derive(Debug, Clone, PartialEq)]
enum Prepared {
    Terminal(String),
    Markdown,
    Html(String),
    MarkdownPlus(Box<Document>),
    Data,
}

/// A prepared document result for `md render`, `md clean`, and `md set`.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::output::{OutputTarget, render_output};
/// use biscuit_terminal::terminal::Terminal;
/// use darkmatter::layout::DarkmatterPage;
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::output::command::DocumentOutput;
/// use renderable::output::{OutputOptions, TerminalProfile, Width};
///
/// let options = OutputOptions::new(Width::Columns(40), TerminalProfile::plain());
/// let page = DarkmatterPage::new(&Terminal::new_optimistic(80));
/// let output = DocumentOutput::prepare(&Markdown::from("**hi**"), &page, OutputTarget::Plain, &options)?;
/// assert_eq!(render_output(&output, OutputTarget::Plain, &options)?.text.trim(), "hi");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentOutput {
    prepared: Prepared,
    markdown: String,
    envelope: DocumentEnvelope,
}

impl DocumentOutput {
    /// Renders `target` once, now, so a failure surfaces here and never inside
    /// an override. A data target renders nothing.
    ///
    /// ## Errors
    ///
    /// Whatever the page's matching render returns.
    pub fn prepare(
        document: &Markdown,
        page: &DarkmatterPage,
        target: OutputTarget,
        options: &OutputOptions,
    ) -> Result<Self, PageRenderError> {
        let prepared = match target {
            OutputTarget::Terminal | OutputTarget::Plain => Prepared::Terminal(
                page.clone()
                    .with_output_options(&terminal_target_options(target, options))
                    .render(document)?,
            ),
            OutputTarget::Html => Prepared::Html(
                page.clone()
                    .with_browser_width(options.width)
                    .render_to_browser_document(document)?,
            ),
            OutputTarget::Markdown => Prepared::Markdown,
            OutputTarget::MarkdownPlus => Prepared::MarkdownPlus(Box::new(page.render_document(document)?)),
            OutputTarget::Json | OutputTarget::Json5 | OutputTarget::Yaml | OutputTarget::Toml => Prepared::Data,
        };
        let markdown = document_markdown(document, true, options.width);
        let envelope = DocumentEnvelope {
            file: None,
            outcome: Outcome::Success,
            document: Some(markdown.clone()),
            diagnostics: Vec::new(),
            summary: None,
            perf: None,
            lifecycle_output: Vec::new(),
        };
        Ok(Self {
            prepared,
            markdown,
            envelope,
        })
    }

    /// Sets the data envelope; the default is a success carrying
    /// [`markdown`](Self::markdown) and no diagnostics.
    #[must_use]
    pub fn with_envelope(mut self, envelope: DocumentEnvelope) -> Self {
        self.envelope = envelope;
        self
    }

    /// Replaces the Markdown with `text`, for a command whose text is not a
    /// reserialization of its document: `md clean` and `md set` keep every
    /// frontmatter byte they did not change.
    #[must_use]
    pub fn with_markdown_source(mut self, text: String) -> Self {
        self.envelope.document = Some(text.clone());
        self.markdown = text;
        self
    }

    /// The text [`document_markdown`] produced at prepare.
    #[must_use]
    pub fn markdown(&self) -> &str {
        &self.markdown
    }
}

impl CommandOutput for DocumentOutput {
    type Data = DocumentEnvelope;

    fn as_prose(&self) -> Document {
        match &self.prepared {
            Prepared::MarkdownPlus(document) => (**document).clone(),
            _ => prose_of(&self.markdown),
        }
    }

    fn as_data(&self) -> DocumentEnvelope {
        self.envelope.clone()
    }

    fn render_terminal(&self, _options: &OutputOptions) -> Option<String> {
        match &self.prepared {
            Prepared::Terminal(text) => Some(text.clone()),
            _ => None,
        }
    }

    fn render_markdown(&self, _options: &OutputOptions) -> Option<String> {
        matches!(self.prepared, Prepared::Markdown).then(|| self.markdown.clone())
    }

    fn render_html(&self, _options: &OutputOptions) -> Option<String> {
        match &self.prepared {
            Prepared::Html(text) => Some(text.clone()),
            _ => None,
        }
    }
}
