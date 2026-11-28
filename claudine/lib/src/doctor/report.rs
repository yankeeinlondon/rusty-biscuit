//! Findings and how a check's findings become its row.
//!
//! A row's severity is its worst finding's (`error > warn > info > ok`),
//! and its detail lists every finding in a fixed order: general findings,
//! then the user scope, then the repository scope, each by path, keeping
//! the check's own order within one path. Every dynamic piece of text is
//! passed through [`display_text`] here, so a check never has to.

use std::path::{Path, PathBuf};

use super::{DoctorRow, Severity};

/// Which configuration scope a finding concerns; also the order findings
/// are listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Scope {
    /// Not tied to one scope (the home, the launch context, a tool).
    General,
    User,
    Repository,
}

/// One thing a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) severity: Severity,
    pub(crate) scope: Scope,
    /// The file it concerns, when there is one.
    pub(crate) path: Option<PathBuf>,
    /// One line, naming no authored value.
    pub(crate) summary: String,
    /// Further lines of explanation.
    pub(crate) notes: Vec<String>,
    pub(crate) remedy: Option<String>,
    /// Whether the check's repair would address it.
    pub(crate) fixable: bool,
}

impl Finding {
    pub(crate) fn new(severity: Severity, scope: Scope, summary: impl Into<String>) -> Self {
        Self { severity, scope, path: None, summary: summary.into(), notes: Vec::new(), remedy: None, fixable: false }
    }

    #[must_use]
    pub(crate) fn at(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub(crate) fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    #[must_use]
    pub(crate) fn remedy(mut self, remedy: impl Into<String>) -> Self {
        self.remedy = Some(remedy.into());
        self
    }

    #[must_use]
    pub(crate) const fn fixable(mut self) -> Self {
        self.fixable = true;
        self
    }
}

/// Everything one check found. With no finding above `ok`, the row's
/// summary is `ok_summary`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckOutcome {
    pub(crate) ok_summary: String,
    pub(crate) findings: Vec<Finding>,
}

impl CheckOutcome {
    pub(crate) fn new(ok_summary: impl Into<String>) -> Self {
        Self { ok_summary: ok_summary.into(), findings: Vec::new() }
    }

    pub(crate) fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Whether the check's repair applies to at least one finding.
    pub(crate) fn is_fixable(&self) -> bool {
        self.findings.iter().any(|finding| finding.fixable)
    }

    fn severity(&self) -> Severity {
        self.findings.iter().map(|finding| finding.severity).max().unwrap_or(Severity::Ok)
    }
}

impl From<Vec<Finding>> for CheckOutcome {
    fn from(findings: Vec<Finding>) -> Self {
        Self { ok_summary: "no problems found".into(), findings }
    }
}

/// What one repair attempt did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepairOutcome {
    /// Each change made, in order (a path replaced, removed, or edited).
    pub(crate) changes: Vec<String>,
    /// Why the attempt stopped, when it failed.
    pub(crate) failure: Option<String>,
}

/// The row for check `id`: `outcome` is the last reading of the disk, and
/// `attempt` the repair tried before it, if any. A failed attempt keeps the
/// row at `error` even when the disk now reads as healthy.
pub(super) fn row(id: &'static str, outcome: CheckOutcome, attempt: Option<&RepairOutcome>) -> DoctorRow {
    let mut findings = outcome.findings.clone();
    findings.sort_by(|left, right| (left.scope, &left.path).cmp(&(right.scope, &right.path)));

    let mut severity = outcome.severity();
    let mut summary = summary_of(&outcome);
    let mut lines: Vec<String> = Vec::new();
    for finding in &findings {
        let lead = finding.path.as_deref().map(|path| format!("{}: ", display_path(path))).unwrap_or_default();
        lines.push(format!("{lead}{}", display_text(&finding.summary)));
        lines.extend(finding.notes.iter().map(|note| display_text(note)));
        if let Some(remedy) = &finding.remedy {
            lines.push(format!("Remedy: {}", display_text(remedy)));
        }
    }
    match attempt {
        Some(RepairOutcome { changes, failure: None }) => {
            if changes.is_empty() {
                lines.push("Repair: nothing needed changing".into());
            }
            lines.extend(changes.iter().map(|change| format!("Repaired: {}", display_text(change))));
        }
        Some(RepairOutcome { changes, failure: Some(reason) }) => {
            severity = Severity::Error;
            summary = format!("the repair failed: {}", display_text(reason));
            lines.push(format!("Repair failed: {}", display_text(reason)));
            if changes.is_empty() {
                lines.push("No change was made before the failure".into());
            }
            lines.extend(changes.iter().map(|change| format!("Already changed: {}", display_text(change))));
        }
        None => {}
    }
    DoctorRow { id, severity, summary, detail: lines.join("\n"), fixable: outcome.is_fixable() }
}

/// The worst finding's summary, counting the others at that severity; the
/// check's own summary when nothing is above `ok`.
fn summary_of(outcome: &CheckOutcome) -> String {
    let severity = outcome.severity();
    if severity == Severity::Ok {
        return display_text(&outcome.ok_summary);
    }
    let mut worst = outcome.findings.iter().filter(|finding| finding.severity == severity);
    let first = worst.next().map(|finding| display_text(&finding.summary)).unwrap_or_default();
    match worst.count() {
        0 => first,
        more => format!("{first} (+{more} more)"),
    }
}

/// A path as the report shows it.
pub(crate) fn display_path(path: &Path) -> String {
    display_text(&path.display().to_string())
}

/// `text` on one line with every control character spelled as an escape
/// (`\u{1b}`), so nothing a file name or key holds can move the cursor,
/// color the terminal, or break a row across lines.
pub(crate) fn display_text(text: &str) -> String {
    let mut shown = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control() || is_bidi_control(character) {
            shown.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
        } else {
            shown.push(character);
        }
    }
    shown
}

/// The bidirectional formatting characters, which reorder the text a
/// terminal displays without being control characters.
const fn is_bidi_control(character: char) -> bool {
    matches!(character, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}
