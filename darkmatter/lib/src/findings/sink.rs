//! The ordered finding collector.

use std::path::PathBuf;

use super::report::{FindingReport, ReportEntry};
use super::{Diagnostic, DiagnosticRelated, NOT_CHECKED_CODE, Severity, Trail};

/// A subject registered with a [`FindingSink`]: an opaque index, assigned in
/// registration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubjectId(usize);

/// A finding pushed into a [`FindingSink`]: an opaque index, assigned in push
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FindingId(usize);

/// What findings are grouped under: a document for `md`, or one occurrence of
/// a task for Claudine.
///
/// Two subjects may share a file or a label; each registration is its own
/// subject, so two task occurrences loading the same document stay apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    /// The host noun (`task`, `group`), or `None` for a document subject.
    pub noun: Option<&'static str>,
    /// The display label: a task's name, or a document's path as shown.
    pub label: String,
    /// The authored property path of the occurrence (`sequence[1]`), when
    /// the subject is a construct inside a document.
    pub property: Option<String>,
    /// The file the subject is authored in, when known.
    pub file: Option<PathBuf>,
}

impl Subject {
    /// A document subject shown as `label`.
    pub fn document(label: impl Into<String>, file: Option<PathBuf>) -> Self {
        Self { noun: None, label: label.into(), property: None, file }
    }

    /// A host construct subject, such as task `find` at `sequence[0]`.
    pub fn construct(
        noun: &'static str,
        label: impl Into<String>,
        property: impl Into<String>,
        file: Option<PathBuf>,
    ) -> Self {
        Self { noun: Some(noun), label: label.into(), property: Some(property.into()), file }
    }

    /// The heading a report writes for this subject:
    /// `task find (sequence[0])`, or the document's label.
    pub fn heading(&self) -> String {
        match (self.noun, &self.property) {
            (Some(noun), Some(property)) => format!("{noun} {} ({property})", self.label),
            (Some(noun), None) => format!("{noun} {}", self.label),
            (None, _) => self.label.clone(),
        }
    }
}

/// Collects findings across a document tree without stopping.
///
/// Subjects keep their registration order and findings their push order
/// within each subject. Pushing a finding equivalent to one already held for
/// the same subject (same code, severity, location, message, and trail)
/// returns the original's id and stores nothing. Two producers that report
/// one problem may differ only in how much of its location they proved: a
/// finding that names only the file of a held finding located at a line in
/// that file, all else equal, is that finding, and the held one keeps (or
/// takes) the line.
#[derive(Debug, Default)]
pub struct FindingSink {
    subjects: Vec<Subject>,
    entries: Vec<ReportEntry>,
}

impl FindingSink {
    #[allow(missing_docs)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `subject`, which the report shows after every subject
    /// registered before it.
    pub fn register_subject(&mut self, subject: Subject) -> SubjectId {
        self.subjects.push(subject);
        SubjectId(self.subjects.len() - 1)
    }

    /// Records `finding` for `subject`.
    ///
    /// ## Panics
    ///
    /// When `subject` was not registered with this sink.
    pub fn push(&mut self, subject: SubjectId, finding: Diagnostic) -> FindingId {
        assert!(subject.0 < self.subjects.len(), "finding pushed for an unregistered subject");
        if let Some(index) = self.entries.iter().position(|entry| {
            entry.subject == subject
                && (equivalent(&entry.diagnostic, &finding) || located_in(&entry.diagnostic, &finding))
        }) {
            return FindingId(index);
        }
        if let Some(index) =
            self.entries.iter().position(|entry| entry.subject == subject && located_in(&finding, &entry.diagnostic))
        {
            let held = &mut self.entries[index].diagnostic;
            held.source = finding.source;
            held.span = finding.span;
            return FindingId(index);
        }
        self.entries.push(ReportEntry { subject, diagnostic: finding, check: None });
        FindingId(self.entries.len() - 1)
    }

    /// Records that `check` could not run for `subject` because of `reason`,
    /// as one `info` [`NOT_CHECKED_CODE`] finding. `blocked_by` names the
    /// finding that stopped it; its location becomes the record's related
    /// location when it has one.
    pub fn not_checked(
        &mut self,
        subject: SubjectId,
        check: &str,
        reason: &str,
        blocked_by: Option<FindingId>,
    ) -> FindingId {
        self.not_checked_within(subject, check, reason, blocked_by, Trail::default())
    }

    /// [`Self::not_checked`] for a subject that is one occurrence among
    /// several (a task a repeated reference reaches): the record carries
    /// `trail`, so occurrences that share a file stay apart in data output.
    pub fn not_checked_within(
        &mut self,
        subject: SubjectId,
        check: &str,
        reason: &str,
        blocked_by: Option<FindingId>,
        trail: Trail,
    ) -> FindingId {
        let mut finding = Diagnostic::projected(
            NOT_CHECKED_CODE,
            Severity::Info,
            format!("not checked: {check} ({reason})"),
            None,
        )
        .with_trail(trail);
        if let Some(source) = blocked_by
            .and_then(|id| self.entries.get(id.0))
            .and_then(|entry| entry.diagnostic.source.clone().map(|source| (source, entry.diagnostic.message.clone())))
            .map(|(source, message)| DiagnosticRelated { message, source })
        {
            finding.related.push(source);
        }
        let id = self.push(subject, finding);
        if let Some(entry) = self.entries.get_mut(id.0) {
            entry.check = Some((check.to_string(), reason.to_string()));
        }
        id
    }

    /// The finding `id` refers to.
    pub fn get(&self, id: FindingId) -> Option<&Diagnostic> {
        self.entries.get(id.0).map(|entry| &entry.diagnostic)
    }

    /// Whether any finding is an error.
    pub fn has_errors(&self) -> bool {
        self.entries.iter().any(|entry| entry.diagnostic.severity == Severity::Error)
    }

    /// The finished report: subjects in registration order, each subject's
    /// findings in push order.
    pub fn finish(self) -> FindingReport {
        FindingReport::new(self.subjects, self.entries)
    }
}

fn equivalent(held: &Diagnostic, new: &Diagnostic) -> bool {
    held.code == new.code
        && held.severity == new.severity
        && held.source == new.source
        && held.span == new.span
        && held.message == new.message
        && held.trail == new.trail
}

/// Whether `file_only` is `located` but for naming only `located`'s file,
/// not its line.
fn located_in(located: &Diagnostic, file_only: &Diagnostic) -> bool {
    let (Some(at), Some(file)) = (&located.source, &file_only.source) else { return false };
    at.line.is_some()
        && file.line.is_none()
        && file.column.is_none()
        && file.file.is_some()
        && file.file == at.file
        && file_only.span.is_none()
        && located.code == file_only.code
        && located.severity == file_only.severity
        && located.message == file_only.message
        && located.trail == file_only.trail
}

impl SubjectId {
    pub(super) fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub(super) fn index(self) -> usize {
        self.0
    }
}
