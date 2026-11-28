//! The generic presentation of a [`FindingReport`]: each subject's heading,
//! then its findings as glyph rows with their details and hint, then its
//! skipped checks grouped by reason.

use std::path::PathBuf;

use biscuit_terminal::render_tree::render_terminal_output;
use renderable::output::{CommandOutput, OutputOptions};
use renderable::tree::{Document, RenderNode, RenderStrictness};

use super::super::report::{FindingReport, finding_details, not_checked_text};
use super::super::{Diagnostic, Severity};
use super::{LinkStyle, dim, glyph_markup, indented, markup};

/// How far a finding's row sits under its subject heading.
const ROW_INDENT: u32 = 2;
/// How far a finding's details and hint sit under its subject heading.
const DETAIL_INDENT: u32 = 6;

/// A [`FindingReport`] as a render tree, from
/// [`FindingReport::presentation`].
#[derive(Debug, Clone)]
pub struct FindingPresentation<'a> {
    report: &'a FindingReport,
    launch_dir: Option<PathBuf>,
    summary: Option<String>,
}

impl FindingReport {
    /// The report's generic presentation, with paths as recorded until
    /// [`FindingPresentation::with_launch_dir`] gives them a base.
    pub fn presentation(&self) -> FindingPresentation<'_> {
        FindingPresentation { report: self, launch_dir: None, summary: None }
    }
}

impl FindingPresentation<'_> {
    /// Paths under `launch_dir`, captured by the caller, are shown relative to
    /// it, as [`FindingReport::render_human`] shows them.
    #[must_use]
    pub fn with_launch_dir(mut self, launch_dir: impl Into<PathBuf>) -> Self {
        self.launch_dir = Some(launch_dir.into());
        self
    }

    /// Ends the presentation with `summary`, a closing tally the caller words
    /// (`Checked notes.md: 1 error, 1 warning`), one blank line after the
    /// last subject.
    #[must_use]
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// The Markdown and browser document.
    pub fn as_prose(&self) -> Document {
        self.document(LinkStyle::Portable)
    }

    /// The `terminal` and `plain` output for `options`, or `None` when the
    /// tree cannot be folded.
    pub fn render_terminal(&self, options: &OutputOptions) -> Option<String> {
        render_terminal_output(&self.document(LinkStyle::Terminal), options, RenderStrictness::Warn)
            .ok()
            .map(|rendered| rendered.output)
    }

    fn document(&self, links: LinkStyle) -> Document {
        let mut sections: Vec<Vec<RenderNode>> = self
            .report
            .subject_findings()
            .into_iter()
            .map(|group| {
                let mut blocks = vec![RenderNode::paragraph(vec![RenderNode::strong(vec![RenderNode::text(
                    group.subject.heading(),
                )])])];
                for diagnostic in group.findings {
                    blocks.extend(self.finding_blocks(diagnostic, links));
                }
                for (reason, checks) in &group.not_checked {
                    blocks.push(row(Severity::Info, &not_checked_text(reason, checks), links));
                }
                blocks
            })
            .collect();
        if let Some(summary) = &self.summary {
            sections.push(vec![RenderNode::paragraph(vec![RenderNode::text(summary.clone())])]);
        }
        Document::new(links.root(sections))
    }

    fn finding_blocks(&self, diagnostic: &Diagnostic, links: LinkStyle) -> Vec<RenderNode> {
        let mut blocks = vec![row(diagnostic.severity, &diagnostic.message, links)];
        for detail in finding_details(self.launch_dir.as_deref(), diagnostic) {
            blocks.push(detail_block(vec![RenderNode::text(detail)], links));
        }
        if let Some(hint) = &diagnostic.hint {
            let mut children = dim("hint:");
            children.push(RenderNode::text(format!(" {hint}")));
            blocks.push(detail_block(children, links));
        }
        blocks
    }
}

impl CommandOutput for FindingPresentation<'_> {
    /// The data form is the report's records, in report order.
    type Data = Vec<Diagnostic>;

    fn as_prose(&self) -> Document {
        FindingPresentation::as_prose(self)
    }

    fn as_data(&self) -> Vec<Diagnostic> {
        self.report.diagnostics().into_iter().cloned().collect()
    }

    fn render_terminal(&self, options: &OutputOptions) -> Option<String> {
        FindingPresentation::render_terminal(self, options)
    }
}

/// A finding's glyph and its literal `text`.
fn row(severity: Severity, text: &str, links: LinkStyle) -> RenderNode {
    let mut children = markup(glyph_markup(severity));
    children.push(RenderNode::text(format!(" {text}")));
    let mut row = RenderNode::paragraph(children);
    if links == LinkStyle::Terminal {
        row.attrs.set_layout(&indented(ROW_INDENT));
    }
    row
}

fn detail_block(children: Vec<RenderNode>, links: LinkStyle) -> RenderNode {
    let mut block = RenderNode::paragraph(children);
    if links == LinkStyle::Terminal {
        block.attrs.set_layout(&indented(DETAIL_INDENT));
    }
    block
}
