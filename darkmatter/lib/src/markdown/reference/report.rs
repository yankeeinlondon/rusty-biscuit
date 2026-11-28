//! The reference report as a command output: a compatibility adapter over
//! [`ReferencePresentation`], which owns the rendering.

use std::path::PathBuf;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::output::{OutputTarget, render_output};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Layout;
use renderable::output::{CommandOutput, OutputOptions, Width};
use renderable::tree::Document;
use serde::Serialize;

use super::types::ReferenceRecord;
use super::validate::ReferenceValidationReport;
use crate::findings::ReferencePresentation;
pub use crate::findings::ReferenceReportVisibility;
use crate::markdown::compose::ComposeSource;

/// The data form of a [`ValidationReportView`]: the report's fields, plus
/// `references` when the view lists them.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReportData {
    #[serde(flatten)]
    report: ReferenceValidationReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    references: Option<Vec<ReferenceRecord>>,
}

/// A [`ReferenceValidationReport`] as a [`CommandOutput`], presented by
/// [`ReferencePresentation`].
///
/// A report with no errors prints its summary alone, or with its warning and
/// info groups under [`ReferenceReportVisibility::All`]; the output is never
/// empty.
#[derive(Debug, Clone)]
pub struct ValidationReportView {
    presentation: ReferencePresentation,
    layout: Layout,
}

impl ValidationReportView {
    /// A view showing `report`'s errors, with paths absolute until
    /// [`with_launch_dir`](Self::with_launch_dir) gives them a base.
    pub fn new(report: ReferenceValidationReport) -> Self {
        Self {
            presentation: ReferencePresentation::new(report),
            layout: Layout::default(),
        }
    }

    /// Names the document the title reports on; defaults to the traversal's
    /// root.
    #[must_use]
    pub fn with_root_source(mut self, source: ComposeSource) -> Self {
        self.presentation = self.presentation.with_root_source(source);
        self
    }

    /// The directory the command was launched from, captured by the caller.
    /// Group headings are relative to it; the process's current directory is
    /// never consulted, so a report built elsewhere reads the same.
    #[must_use]
    pub fn with_launch_dir(mut self, launch_dir: impl Into<PathBuf>) -> Self {
        self.presentation = self.presentation.with_launch_dir(launch_dir);
        self
    }

    /// Chooses which severities the groups list.
    #[must_use]
    pub fn with_visibility(mut self, visibility: ReferenceReportVisibility) -> Self {
        self.presentation = self.presentation.with_visibility(visibility);
        self
    }

    /// Adds every reference and non-blocking warning before the summary, and
    /// `references` to the data form.
    #[must_use]
    pub fn with_reference_listing(mut self) -> Self {
        self.presentation = self.presentation.with_reference_listing();
        self
    }
}

impl CommandOutput for ValidationReportView {
    type Data = ValidationReportData;

    fn as_prose(&self) -> Document {
        self.presentation.as_prose()
    }

    fn as_data(&self) -> ValidationReportData {
        let report = self.presentation.report();
        ValidationReportData {
            report: report.clone(),
            references: self.presentation.lists_references().then(|| report.references.clone()),
        }
    }

    fn render_terminal(&self, options: &OutputOptions) -> Option<String> {
        self.presentation.render_terminal(options)
    }
}

impl TerminalRenderable for ValidationReportView {
    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    /// The `terminal` output for `term` at verbosity zero.
    fn render(&self, term: &Terminal) -> String {
        let width = term
            .unbounded_width
            .map(Width::from)
            .or_else(|| term.detected_columns().and_then(Width::columns))
            .unwrap_or(Width::Unlimited);
        let options = OutputOptions::new(width, term.output_profile());
        match render_output(self, OutputTarget::Terminal, &options) {
            Ok(rendered) => rendered.text,
            Err(error) => {
                tracing::error!(%error, "reference report failed to render");
                String::new()
            }
        }
    }
}
