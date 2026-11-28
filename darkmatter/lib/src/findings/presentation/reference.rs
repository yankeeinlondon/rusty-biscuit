//! The reference presentation: a [`ReferenceValidationReport`] grouped by the
//! document that contains each problem, because a fix happens file by file;
//! the per-reason overview lives in its summary.
//!
//! `as_prose` gives every Markdown and browser output absolute-path links. The
//! terminal builds the same structure with `file://` links and no blank line
//! between a heading and its rows, which a Markdown table could not survive.

use std::path::PathBuf;

use biscuit_terminal::render_tree::render_terminal_output;
use renderable::output::OutputOptions;
use renderable::output::link_metadata::{LinkMetadata, QueryParams};
use renderable::tree::{Document, RenderNode, RenderStrictness};

use super::{INDENT, LinkStyle, PathDisplay, dim, file_name, indented, markup, noun, rows_table};
use crate::markdown::compose::ComposeSource;
use crate::markdown::reference::types::{ReferenceOrigin, ReferenceSyntax, ReferenceTrailStep};
use crate::markdown::reference::validate::{ReferenceIssue, ReferenceSeverity, ReferenceValidationReport};

/// Which issues a report's groups list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferenceReportVisibility {
    /// Errors only; warnings and info are not mentioned.
    #[default]
    Errors,
    /// Every severity, with warning and info counts in the summary.
    All,
}

/// The reference presentation of a [`ReferenceValidationReport`].
///
/// A report with no errors presents its summary alone, or with its warning
/// and info groups under [`ReferenceReportVisibility::All`]; the output is
/// never empty.
#[derive(Debug, Clone)]
pub struct ReferencePresentation {
    report: ReferenceValidationReport,
    root_source: Option<ComposeSource>,
    paths: PathDisplay,
    visibility: ReferenceReportVisibility,
    listing: bool,
}

impl ReferencePresentation {
    /// A presentation of `report`'s errors, with paths absolute until
    /// [`with_launch_dir`](Self::with_launch_dir) gives them a base.
    pub fn new(report: ReferenceValidationReport) -> Self {
        Self {
            report,
            root_source: None,
            paths: PathDisplay::default(),
            visibility: ReferenceReportVisibility::default(),
            listing: false,
        }
    }

    /// Names the document the title reports on; defaults to the traversal's
    /// root.
    #[must_use]
    pub fn with_root_source(mut self, source: ComposeSource) -> Self {
        self.root_source = Some(source);
        self
    }

    /// The directory the command was launched from, captured by the caller.
    /// Group headings are relative to it, so a report built elsewhere reads
    /// the same.
    #[must_use]
    pub fn with_launch_dir(mut self, launch_dir: impl Into<PathBuf>) -> Self {
        self.paths.launch_dir = Some(launch_dir.into());
        self
    }

    /// Chooses which severities the groups list.
    #[must_use]
    pub fn with_visibility(mut self, visibility: ReferenceReportVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Adds every reference and non-blocking warning before the summary.
    #[must_use]
    pub fn with_reference_listing(mut self) -> Self {
        self.listing = true;
        self
    }

    #[allow(missing_docs)]
    pub fn report(&self) -> &ReferenceValidationReport {
        &self.report
    }

    /// Whether the references are listed.
    pub fn lists_references(&self) -> bool {
        self.listing
    }

    /// The Markdown and browser document: absolute-path links, sections as
    /// fragments.
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
        let mut sections = Vec::new();
        if self.report.error_count() > 0 {
            sections.push(vec![self.title(links)]);
        }
        for group in self.groups() {
            sections.push(vec![
                self.group_heading(&group, links),
                rows_table(
                    group.issues.iter().flat_map(|issue| self.issue_rows(issue, links)).collect(),
                    links,
                ),
            ]);
        }
        if self.listing {
            sections.extend(self.listing_sections(links));
        }
        sections.push(self.summary(links));
        Document::new(links.root(sections))
    }

    fn title(&self, links: LinkStyle) -> RenderNode {
        let root = self
            .root_source
            .as_ref()
            .or_else(|| self.report.documents.first().map(|document| &document.source))
            .unwrap_or(&ComposeSource::Unknown);
        let name = match root {
            ComposeSource::File(path) => self.paths.path_link(path, links, vec![RenderNode::text(file_name(path))]),
            other => RenderNode::text(other.display()),
        };
        RenderNode::paragraph(vec![RenderNode::strong(vec![
            RenderNode::text("Broken references in "),
            name,
        ])])
    }

    /// The visible issues under their `(source, trail)` documents, in
    /// traversal order. An issue from a document the traversal did not record
    /// follows those, in first-occurrence order.
    fn groups(&self) -> Vec<IssueGroup<'_>> {
        let mut groups: Vec<IssueGroup<'_>> = Vec::new();
        for document in &self.report.documents {
            group_for(&mut groups, &document.source, &document.included_via);
        }
        let shown = self.report.issues.iter().filter(|issue| match self.visibility {
            ReferenceReportVisibility::Errors => issue.severity == ReferenceSeverity::Error,
            ReferenceReportVisibility::All => true,
        });
        for issue in shown {
            group_for(&mut groups, &issue.origin.source, &issue.included_via).issues.push(issue);
        }
        groups.retain(|group| !group.issues.is_empty());
        groups
    }

    fn group_heading(&self, group: &IssueGroup<'_>, links: LinkStyle) -> RenderNode {
        let name = match group.source {
            ComposeSource::File(path) => {
                self.paths.path_link(path, links, vec![RenderNode::text(self.paths.relative_display(path))])
            }
            other => RenderNode::text(other.display()),
        };
        let mut children = vec![RenderNode::strong(vec![name])];
        if !group.trail.is_empty() {
            let steps: Vec<String> = group.trail.iter().map(trail_step_text).collect();
            children.push(RenderNode::text(" "));
            children.extend(dim(&format!("(transcluded at {})", steps.join(" → "))));
        }
        RenderNode::paragraph(children)
    }

    fn issue_rows(&self, issue: &ReferenceIssue, links: LinkStyle) -> Vec<RenderNode> {
        let reason = issue.code.reason_phrase();
        let mut problem = match issue.severity {
            ReferenceSeverity::Error => markup(&format!("<red-500>{reason}</red-500>")),
            ReferenceSeverity::Warning => markup(&format!("<yellow-500>{reason}</yellow-500>")),
            ReferenceSeverity::Info => dim(reason),
        };
        problem.push(RenderNode::text(" "));
        problem.push(construct_node(issue.origin.syntax));

        let mut rows = vec![RenderNode::table_row(vec![
            RenderNode::table_cell(problem),
            RenderNode::table_cell(vec![RenderNode::inline_code(issue.reference_display.clone())]),
            RenderNode::table_cell(vec![self.location_node(&issue.origin, links)]),
        ])];
        let hint = issue
            .resolution_failure()
            .and_then(|failure| failure.glob_hint(&issue.reference_display));
        for detail in hint.into_iter().chain(issue.detail.as_deref()) {
            rows.push(RenderNode::table_row(vec![
                RenderNode::table_cell(dim(detail)),
                RenderNode::table_cell(Vec::new()),
                RenderNode::table_cell(Vec::new()),
            ]));
        }
        rows
    }

    /// The source document's position as one link. A document with no file
    /// names itself instead, and a position the provenance could not prove is
    /// left out rather than guessed.
    fn location_node(&self, origin: &ReferenceOrigin, links: LinkStyle) -> RenderNode {
        let ComposeSource::File(path) = &origin.source else {
            return RenderNode::text(origin.source.display());
        };
        let mut query = QueryParams::new();
        let mut label = vec![RenderNode::text(file_name(path))];
        if let Some(authored) = &origin.authored {
            label.extend(dim(&format!(":{}:{}", authored.line, authored.column)));
            query.insert("line", authored.line as u64);
            query.insert("column", authored.column as u64);
        }
        let mut link = match links {
            LinkStyle::Portable => {
                let mut destination = self.paths.portable_destination(path);
                if let Some(section) = &origin.section {
                    destination.push('#');
                    destination.push_str(section);
                }
                RenderNode::link(destination, None, label)
            }
            // `heading` travels as a parameter, not a fragment, so a link
            // printed beside its text (`-v`) stays the bare file URI.
            LinkStyle::Terminal => {
                if let Some(section) = &origin.section {
                    query.insert("heading", section.as_str());
                }
                RenderNode::link(self.paths.file_uri(path), None, label)
            }
        };
        link.attrs.set_link_metadata(LinkMetadata { query });
        link
    }

    fn listing_sections(&self, links: LinkStyle) -> Vec<Vec<RenderNode>> {
        let mut sections = Vec::new();
        let rows: Vec<RenderNode> = self
            .report
            .references
            .iter()
            .map(|record| {
                RenderNode::table_row(vec![
                    RenderNode::table_cell(vec![construct_node(record.origin.syntax)]),
                    RenderNode::table_cell(vec![RenderNode::inline_code(record.target.raw().unwrap_or_default())]),
                    RenderNode::table_cell(vec![self.location_node(&record.origin, links)]),
                ])
            })
            .collect();
        if !rows.is_empty() {
            sections.push(vec![
                RenderNode::paragraph(vec![RenderNode::strong(vec![RenderNode::text("All references")])]),
                rows_table(rows, links),
            ]);
        }
        if !self.report.warnings.is_empty() {
            sections.push(self.report.warnings.iter().map(|warning| RenderNode::paragraph(dim(warning))).collect());
        }
        sections
    }

    /// The scope and result line, and when it has anything to say the tally
    /// line.
    fn summary(&self, links: LinkStyle) -> Vec<RenderNode> {
        let scanned = self.report.references_scanned;
        let documents = self.report.document_count();
        let broken = self.report.error_count();
        let scope = format!(
            "Checked {scanned} {} across {documents} {}: ",
            noun(scanned, "reference", "references"),
            noun(documents, "document", "documents"),
        );
        let mut line = vec![RenderNode::text(scope)];
        if broken == 0 {
            line.push(RenderNode::text("all ok"));
        } else {
            line.push(RenderNode::text(format!("{} ok, ", scanned.saturating_sub(broken))));
            line.extend(markup(&format!("<red-500><b>{broken} broken</b></red-500>")));
        }

        let mut summary = vec![RenderNode::paragraph(line)];
        let tally = self.tally();
        if !tally.is_empty() {
            let mut tally_line = RenderNode::paragraph(dim(&tally.join(" · ")));
            if links == LinkStyle::Terminal {
                tally_line.attrs.set_layout(&indented(INDENT));
            }
            summary.push(tally_line);
        }
        summary
    }

    /// The tally by reason, most common first and ties by first occurrence,
    /// then under [`ReferenceReportVisibility::All`] the other severities. One
    /// distinct reason adds nothing to the scope line, so it is left out.
    fn tally(&self) -> Vec<String> {
        let mut reasons: Vec<(&str, usize)> = Vec::new();
        for issue in self.report.issues.iter().filter(|issue| issue.severity == ReferenceSeverity::Error) {
            let reason = issue.code.reason_phrase();
            match reasons.iter_mut().find(|(known, _)| *known == reason) {
                Some((_, count)) => *count += 1,
                None => reasons.push((reason, 1)),
            }
        }
        reasons.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        let mut tally: Vec<String> = Vec::new();
        if reasons.len() > 1 {
            tally.extend(reasons.iter().map(|(reason, count)| format!("{count} × {reason}")));
        }
        if self.visibility == ReferenceReportVisibility::All {
            let count_of = |severity: ReferenceSeverity| {
                self.report.issues.iter().filter(|issue| issue.severity == severity).count()
            };
            let warnings = count_of(ReferenceSeverity::Warning);
            if warnings > 0 {
                tally.push(format!("{warnings} {}", noun(warnings, "warning", "warnings")));
            }
            let info = count_of(ReferenceSeverity::Info);
            if info > 0 {
                tally.push(format!("{info} info"));
            }
        }
        tally
    }
}

struct IssueGroup<'a> {
    source: &'a ComposeSource,
    trail: &'a [ReferenceTrailStep],
    issues: Vec<&'a ReferenceIssue>,
}

fn group_for<'g, 'a>(
    groups: &'g mut Vec<IssueGroup<'a>>,
    source: &'a ComposeSource,
    trail: &'a [ReferenceTrailStep],
) -> &'g mut IssueGroup<'a> {
    let index = groups
        .iter()
        .position(|group| group.source == source && group.trail == trail)
        .unwrap_or_else(|| {
            groups.push(IssueGroup {
                source,
                trail,
                issues: Vec::new(),
            });
            groups.len() - 1
        });
    &mut groups[index]
}

/// The construct phrase, italic, with its syntax token as inline code:
/// `in ::file`, but the words `in hyperlink` plain.
fn construct_node(syntax: ReferenceSyntax) -> RenderNode {
    let phrase = syntax.construct_phrase();
    let children = match syntax {
        ReferenceSyntax::MarkdownLink
        | ReferenceSyntax::HtmlAnchor
        | ReferenceSyntax::MarkdownImage
        | ReferenceSyntax::HtmlImage => vec![RenderNode::text(phrase)],
        _ => match phrase.rsplit_once(' ') {
            Some((words, token)) => {
                vec![RenderNode::text(format!("{words} ")), RenderNode::inline_code(token)]
            }
            None => vec![RenderNode::text(phrase)],
        },
    };
    RenderNode::emphasis(children)
}

fn trail_step_text(step: &ReferenceTrailStep) -> String {
    let name = match &step.file {
        ComposeSource::File(path) => file_name(path),
        other => other.display().into_owned(),
    };
    match step.line {
        Some(line) => format!("{name}:{line}"),
        None => name,
    }
}
