//! The finished, ordered report of a [`FindingSink`](super::FindingSink).

use std::path::{Path, PathBuf};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::word_wrap::word_wrap;
use biscuit_terminal::utils::wrap_policy::WordWrap;
use serde::Serialize;

use super::presentation::glyph_markup;
use super::sink::{Subject, SubjectId};
use super::{Diagnostic, DiagnosticSource, Severity};

/// One finding in a report, with the subject it is grouped under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportEntry {
    pub(super) subject: SubjectId,
    pub(super) diagnostic: Diagnostic,
    /// The skipped check and its reason, for a `dm.check.not_checked` row;
    /// a human report groups those under one line per reason.
    pub(super) check: Option<(String, String)>,
}

impl ReportEntry {
    #[allow(missing_docs)]
    pub fn subject(&self) -> SubjectId {
        self.subject
    }

    #[allow(missing_docs)]
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
}

/// How many findings of each severity a report holds; the data form of a
/// report's `summary`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct FindingCounts {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

/// Findings grouped by subject, in the order they were registered and pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingReport {
    subjects: Vec<Subject>,
    /// Ordered by subject, then push order within the subject.
    entries: Vec<ReportEntry>,
}

/// How a human report lays out its rows.
#[derive(Debug, Clone, Default)]
pub struct HumanLayout {
    /// The column width rows wrap at; `None` never wraps.
    pub width: Option<u32>,
    /// Paths under this directory are shown relative to it, with `/`.
    pub launch_dir: Option<PathBuf>,
}

impl FindingReport {
    pub(super) fn new(subjects: Vec<Subject>, mut entries: Vec<ReportEntry>) -> Self {
        // A stable sort keeps push order within each subject.
        entries.sort_by_key(|entry| entry.subject);
        Self { subjects, entries }
    }

    #[allow(missing_docs)]
    pub fn subjects(&self) -> &[Subject] {
        &self.subjects
    }

    /// Every finding, grouped by subject in registration order.
    pub fn entries(&self) -> &[ReportEntry] {
        &self.entries
    }

    /// Every finding's record, in report order: the data form's
    /// `diagnostics`.
    pub fn diagnostics(&self) -> Vec<&Diagnostic> {
        self.entries.iter().map(|entry| &entry.diagnostic).collect()
    }

    /// The subject `id` refers to.
    pub fn subject(&self, id: SubjectId) -> Option<&Subject> {
        self.subjects.get(id.index())
    }

    #[allow(missing_docs)]
    pub fn counts(&self) -> FindingCounts {
        let mut counts = FindingCounts::default();
        for entry in &self.entries {
            match entry.diagnostic.severity {
                Severity::Error => counts.errors += 1,
                Severity::Warning => counts.warnings += 1,
                Severity::Info => counts.info += 1,
            }
        }
        counts
    }

    /// Whether any finding is an error or a warning.
    pub fn has_problems(&self) -> bool {
        let counts = self.counts();
        counts.errors + counts.warnings > 0
    }

    /// Each subject that holds findings, in registration order, with its
    /// findings in push order and its skipped checks grouped by reason.
    pub(super) fn subject_findings(&self) -> Vec<SubjectFindings<'_>> {
        let mut shown = Vec::new();
        for (index, subject) in self.subjects.iter().enumerate() {
            let id = SubjectId::from_index(index);
            let mut group = SubjectFindings { subject, findings: Vec::new(), not_checked: Vec::new() };
            for entry in self.entries.iter().filter(|entry| entry.subject == id) {
                match &entry.check {
                    Some((check, reason)) => match group.not_checked.iter_mut().find(|(held, _)| held == reason) {
                        Some((_, checks)) => checks.push(check.clone()),
                        None => group.not_checked.push((reason.clone(), vec![check.clone()])),
                    },
                    None => group.findings.push(&entry.diagnostic),
                }
            }
            if !group.findings.is_empty() || !group.not_checked.is_empty() {
                shown.push(group);
            }
        }
        shown
    }

    /// The report's rows for a person: each subject with findings under its
    /// heading, one blank line between subjects. Authored tokens are written
    /// as plain text; only the severity glyphs and labels are styled, for
    /// `term`.
    pub fn render_human(&self, term: &Terminal, layout: &HumanLayout) -> String {
        let launch_dir = layout.launch_dir.as_deref();
        let mut blocks: Vec<String> = Vec::new();
        for group in self.subject_findings() {
            let mut lines = vec![styled(term, &format!("<b>{}</b>", Prose::escape_text(&group.subject.heading())))];
            for diagnostic in &group.findings {
                lines.extend(row(term, layout, diagnostic.severity, &diagnostic.message));
                for detail in finding_details(launch_dir, diagnostic) {
                    lines.extend(wrapped(layout, 6, 6, &detail));
                }
                if let Some(hint) = &diagnostic.hint {
                    let mut hint_lines = wrapped(layout, 6, 12, &format!("hint: {hint}"));
                    if let Some(first) = hint_lines.first_mut() {
                        *first = first.replacen("hint:", &styled(term, "<dim>hint:</dim>"), 1);
                    }
                    lines.extend(hint_lines);
                }
            }
            for (reason, checks) in &group.not_checked {
                lines.extend(row(term, layout, Severity::Info, &not_checked_text(reason, checks)));
            }
            blocks.push(lines.join("\n"));
        }
        blocks.join("\n\n")
    }
}

/// One subject's part of a report: its findings, then its skipped checks as
/// `(reason, checks)`, in first-occurrence order.
pub(super) struct SubjectFindings<'a> {
    pub(super) subject: &'a Subject,
    pub(super) findings: Vec<&'a Diagnostic>,
    pub(super) not_checked: Vec<(String, Vec<String>)>,
}

/// The row text of the checks skipped for one reason.
pub(super) fn not_checked_text(reason: &str, checks: &[String]) -> String {
    format!("not checked: {} ({reason})", checks.join(", "))
}

/// The lines under a finding's message, before its hint: its notes, what
/// its reference resolved to, and where it was written, with paths under
/// `launch_dir` shown relative to it.
pub(super) fn finding_details(launch_dir: Option<&Path>, diagnostic: &Diagnostic) -> Vec<String> {
    let mut details: Vec<String> = diagnostic.notes.clone();
    if let Some(reference) = &diagnostic.reference_details {
        if let (Some(construct), Some(written)) = (&reference.construct, &reference.reference) {
            details.push(format!("{construct} {written}"));
        }
        if let Some(resolved) = &reference.resolved {
            // A path shown relative to the launch directory needs no base
            // when the base is that directory.
            let base = reference.base.as_ref().filter(|base| {
                launch_dir.is_none_or(|launch| {
                    biscuit_file::PathIdentity::new(Path::new(base.as_str())) != biscuit_file::PathIdentity::new(launch)
                })
            });
            match base {
                Some(base) => details.push(format!(
                    "resolved to {} (relative to {})",
                    display_path(launch_dir, resolved),
                    display_path(launch_dir, base)
                )),
                None => details.push(format!("resolved to {}", display_path(launch_dir, resolved))),
            }
        }
        if let Some(suggestions) = reference.suggestions.as_ref().filter(|found| !found.is_empty()) {
            details.push(format!("did you mean: {}", suggestions.join(", ")));
        }
    }
    if let Some(source) = &diagnostic.source
        && let Some(location) = location_text(launch_dir, source)
    {
        details.push(location);
    }
    details
}

/// The glyph line of one finding: `  ✗ message`, wrapped under its text.
fn row(term: &Terminal, layout: &HumanLayout, severity: Severity, text: &str) -> Vec<String> {
    let glyph = styled(term, glyph_markup(severity));
    let mut lines = wrapped(layout, 4, 4, text);
    if let Some(first) = lines.first_mut() {
        *first = format!("  {glyph} {}", first.trim_start());
    }
    lines
}

/// `text` wrapped at the layout width: the first line indented by `first`
/// columns, the rest by `rest`.
fn wrapped(layout: &HumanLayout, first: usize, rest: usize, text: &str) -> Vec<String> {
    let Some(width) = layout.width else {
        return vec![format!("{}{text}", " ".repeat(first))];
    };
    let available = width.saturating_sub(first.max(rest) as u32).max(20);
    word_wrap(text, WordWrap::WrapProse(None, None), available)
        .into_iter()
        .enumerate()
        .map(|(index, line)| format!("{}{}", " ".repeat(if index == 0 { first } else { rest }), line.trim_end()))
        .collect()
}

/// `markup` rendered for `term`, with no trailing newline.
fn styled(term: &Terminal, markup: &str) -> String {
    Prose::new(markup).render(term).trim_end_matches('\n').to_string()
}

fn location_text(launch_dir: Option<&Path>, source: &DiagnosticSource) -> Option<String> {
    let file = source.file.as_ref()?;
    let shown = display_path(launch_dir, &biscuit_file::to_forward_slash_string(file));
    Some(match (source.line, source.column) {
        (Some(line), Some(column)) => format!("at {shown}:{line}:{column}"),
        (Some(line), None) => format!("at {shown}:{line}"),
        _ => format!("in {shown}"),
    })
}

/// `path` relative to the launch directory, with `/`, when it lies under it;
/// otherwise as given.
fn display_path(launch_dir: Option<&Path>, path: &str) -> String {
    let Some(base) = launch_dir else {
        return path.to_string();
    };
    match Path::new(path).strip_prefix(base) {
        Ok(relative) if !relative.as_os_str().is_empty() => biscuit_file::to_forward_slash_string(relative),
        _ => path.to_string(),
    }
}
