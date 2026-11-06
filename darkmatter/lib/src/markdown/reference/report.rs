//! The reference report: a [`ReferenceValidationReport`] as a command output.
//!
//! The report is grouped by the document that contains each problem, because a
//! fix happens file by file; the per-reason overview lives in its summary.
//! `as_prose` gives every Markdown and browser output absolute-path links. The
//! terminal builds the same structure with `file://` links and no blank line
//! between a heading and its rows, which a Markdown table could not survive.

use std::path::{Path, PathBuf};

use biscuit_file::{PathIdentity, canonicalize_simplified, to_portable_string};
use biscuit_terminal::components::prose::InlineProse;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableColumn};
use biscuit_terminal::output::{OutputTarget, render_output};
use biscuit_terminal::render_tree::render_terminal_output;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Layout, WordWrap};
use renderable::layout::{Edges, Length, TargetValue, Width as LayoutWidth};
use renderable::output::link_metadata::{LinkMetadata, QueryParams};
use renderable::output::{CommandOutput, OutputOptions, Width};
use renderable::tree::{Document, RenderNode, RenderStrictness, SequenceJoin, TreeRenderable};
use serde::Serialize;

use super::types::{ReferenceOrigin, ReferenceRecord, ReferenceSyntax, ReferenceTrailStep};
use super::validate::{ReferenceIssue, ReferenceSeverity, ReferenceValidationReport};
use crate::markdown::compose::ComposeSource;

/// How far a table or a summary tally sits under its heading in a terminal.
const INDENT: u32 = 2;

/// Which issues a report's groups list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferenceReportVisibility {
    /// Errors only; warnings and info are not mentioned.
    #[default]
    Errors,
    /// Every severity, with warning and info counts in the summary.
    All,
}

/// The data form of a [`ValidationReportView`]: the report's fields, plus
/// `references` when the view lists them.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReportData {
    #[serde(flatten)]
    report: ReferenceValidationReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    references: Option<Vec<ReferenceRecord>>,
}

/// A [`ReferenceValidationReport`] as a [`CommandOutput`].
///
/// A report with no errors prints its summary alone, or with its warning and
/// info groups under [`ReferenceReportVisibility::All`]; the output is never
/// empty.
#[derive(Debug, Clone)]
pub struct ValidationReportView {
    report: ReferenceValidationReport,
    root_source: Option<ComposeSource>,
    launch_dir: Option<PathBuf>,
    visibility: ReferenceReportVisibility,
    listing: bool,
    layout: Layout,
}

impl ValidationReportView {
    /// A view showing `report`'s errors, with paths absolute until
    /// [`with_launch_dir`](Self::with_launch_dir) gives them a base.
    pub fn new(report: ReferenceValidationReport) -> Self {
        Self {
            report,
            root_source: None,
            launch_dir: None,
            visibility: ReferenceReportVisibility::default(),
            listing: false,
            layout: Layout::default(),
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
    /// Group headings are relative to it; the process's current directory is
    /// never consulted, so a report built elsewhere reads the same.
    #[must_use]
    pub fn with_launch_dir(mut self, launch_dir: impl Into<PathBuf>) -> Self {
        self.launch_dir = Some(launch_dir.into());
        self
    }

    /// Chooses which severities the groups list.
    #[must_use]
    pub fn with_visibility(mut self, visibility: ReferenceReportVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Adds every reference and non-blocking warning before the summary, and
    /// `references` to the data form.
    #[must_use]
    pub fn with_reference_listing(mut self) -> Self {
        self.listing = true;
        self
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
            ComposeSource::File(path) => self.path_link(path, links, vec![RenderNode::text(file_name(path))]),
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
                self.path_link(path, links, vec![RenderNode::text(self.relative_display(path))])
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
                let mut destination = self.portable_destination(path);
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
                RenderNode::link(self.file_uri(path), None, label)
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
                tally_line.attrs.set_layout(&indented());
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

    fn absolute_path(&self, path: &Path) -> PathBuf {
        match &self.launch_dir {
            Some(launch_dir) if !path.is_absolute() => launch_dir.join(path),
            _ => path.to_path_buf(),
        }
    }

    /// `path` relative to the launch directory, `/`-separated on every OS; its
    /// absolute form without a launch directory or a route to it.
    ///
    /// A path outside the launch directory by spelling alone (`/var` for
    /// `/private/var`) is routed again through both canonical forms, so an
    /// alias does not turn a file beside the reader into a climb to the root.
    fn relative_display(&self, path: &Path) -> String {
        let absolute = self.absolute_path(path);
        let Some(launch_dir) = self.launch_dir.as_deref() else {
            return to_portable_string(&absolute);
        };
        let route_between = |file: &Path, dir: &Path| PathIdentity::new(file).relative_from(&PathIdentity::new(dir));
        let canonical = |path: &Path| canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());
        let route = route_between(&absolute, launch_dir)
            .filter(|route| route.parent_hops() == 0)
            .or_else(|| route_between(&canonical(&absolute), &canonical(launch_dir)));
        match route {
            Some(route) => to_portable_string(&route.to_path_buf()),
            None => to_portable_string(&absolute),
        }
    }

    /// The absolute path as a link destination whose `#` and `?` still belong
    /// to the path. Nothing else is rewritten.
    fn portable_destination(&self, path: &Path) -> String {
        to_portable_string(&self.absolute_path(path))
            .replace('#', "%23")
            .replace('?', "%3F")
    }

    /// The path's `file://` URI, or its portable text when no URI can name it.
    fn file_uri(&self, path: &Path) -> String {
        let absolute = self.absolute_path(path);
        url::Url::from_file_path(&absolute).map_or_else(|()| to_portable_string(&absolute), String::from)
    }

    fn path_link(&self, path: &Path, links: LinkStyle, children: Vec<RenderNode>) -> RenderNode {
        let destination = match links {
            LinkStyle::Portable => self.portable_destination(path),
            LinkStyle::Terminal => self.file_uri(path),
        };
        RenderNode::link(destination, None, children)
    }
}

impl CommandOutput for ValidationReportView {
    type Data = ValidationReportData;

    fn as_prose(&self) -> Document {
        self.document(LinkStyle::Portable)
    }

    fn as_data(&self) -> ValidationReportData {
        ValidationReportData {
            report: self.report.clone(),
            references: self.listing.then(|| self.report.references.clone()),
        }
    }

    fn render_terminal(&self, options: &OutputOptions) -> Option<String> {
        render_terminal_output(&self.document(LinkStyle::Terminal), options, RenderStrictness::Warn).ok().map(|rendered| rendered.output)
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
        let width = term.detected_columns().and_then(Width::columns).unwrap_or(Width::Unlimited);
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

/// Which outputs a built document serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkStyle {
    /// Markdown, MarkdownPlus, and HTML: absolute paths, section fragments.
    Portable,
    /// `terminal` and `plain`: bare `file://` URIs, section as a parameter.
    Terminal,
}

impl LinkStyle {
    /// The document root. Each section's blocks sit one line apart in a
    /// terminal and sections one blank line apart; elsewhere everything is an
    /// ordinary separate block, where a heading must stay clear of a table.
    fn root(self, sections: Vec<Vec<RenderNode>>) -> RenderNode {
        match self {
            Self::Portable => RenderNode::root(sections.into_iter().flatten().collect()),
            Self::Terminal => {
                let mut children = Vec::new();
                for section in sections {
                    if !children.is_empty() {
                        children.push(RenderNode::text("\n\n"));
                    }
                    for (index, block) in section.into_iter().enumerate() {
                        if index > 0 {
                            children.push(RenderNode::text("\n"));
                        }
                        children.push(block);
                    }
                }
                let mut root = RenderNode::root(children);
                root.attrs.set_sequence_join(SequenceJoin::None);
                root
            }
        }
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

/// A borderless three-column table of `rows`, whose empty header a terminal
/// omits and a Markdown table requires.
fn rows_table(rows: Vec<RenderNode>, links: LinkStyle) -> RenderNode {
    let mut table = Table::new()
        // A reference and a location are copied as written, so a width that
        // cannot hold them stacks the row rather than split them.
        .with_columns(vec![
            TableColumn::new(""),
            TableColumn::new("").with_word_wrap(WordWrap::None),
            TableColumn::new("").with_word_wrap(WordWrap::None),
        ])
        .borderless()
        .render_tree();
    if links == LinkStyle::Terminal {
        table.attrs.set_layout(&indented());
    }
    if let Some(children) = table.children_mut() {
        children.extend(rows);
    }
    table
}

fn indented() -> Layout {
    Layout {
        margin: Edges {
            left: TargetValue::universal(Length::ch(INDENT)),
            ..Edges::default()
        },
        width: LayoutWidth::FitContent,
        ..Layout::default()
    }
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

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| to_portable_string(path), |name| name.to_string_lossy().into_owned())
}

/// Phrasing nodes for Prose `text`, whose styling tags take effect.
fn markup(text: &str) -> Vec<RenderNode> {
    InlineProse::new(text).to_render_nodes()
}

/// Phrasing nodes for the literal `text`, dimmed.
fn dim(text: &str) -> Vec<RenderNode> {
    markup(&format!("<dim>{}</dim>", InlineProse::escape_text(text)))
}

fn noun(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}
