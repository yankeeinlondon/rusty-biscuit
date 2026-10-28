//! `md compose --output report-json`: the composition report.
//!
//! A report-mode run writes exactly one JSON object to stdout, on success
//! and on every failure after argument parsing:
//!
//! ```json
//! { "outcome": "failure", "diagnostics": [ … ], "document": null }
//! ```
//!
//! [`ReportSink`] collects each diagnostic when the run reports it (in
//! discovery order) and the composed document when the run would print it;
//! [`ReportSink::finish`] adds the failure's diagnostics and writes the
//! object. Codes and messages come from the library's shared diagnostics
//! (`darkmatter::markdown::compose::report_diagnostics`); this module only
//! collects and serializes them. Nothing is evaluated or rendered again.
//!
//! See `darkmatter/docs/cli/report-json.md`.

use std::cell::RefCell;
use std::io::Write as _;

use color_eyre::eyre::{Report, Result, eyre};
use darkmatter::markdown::compose::ComposeWarning;
use darkmatter::markdown::compose::expression::Diagnostic;
use darkmatter::markdown::compose::report_diagnostics::{
    INPUT_READ_CODE, error_diagnostics, reference_issue_diagnostic, warning_diagnostic,
};
use darkmatter::markdown::reference::validate::{
    ReferenceIssueCode, ReferenceSeverity, ReferenceValidationReport,
};
use serde::Serialize;

/// The report's diagnostics and document, filled as the run proceeds.
#[derive(Default)]
pub(crate) struct ReportSink {
    diagnostics: RefCell<Vec<Diagnostic>>,
    document: RefCell<Option<String>>,
}

impl ReportSink {
    /// Records `warnings`, as the human run would print them.
    pub(crate) fn warnings(&self, warnings: &[ComposeWarning]) {
        self.diagnostics.borrow_mut().extend(warnings.iter().map(warning_diagnostic));
    }

    /// Records `diagnostics` the library produced for a failure.
    pub(crate) fn diagnostics(&self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.borrow_mut().extend(diagnostics);
    }

    /// Records the reference issues a human run prints: errors, and missing
    /// targets inside inserted text.
    pub(crate) fn references(&self, report: &ReferenceValidationReport) {
        let reported = report.issues.iter().filter(|issue| {
            issue.severity == ReferenceSeverity::Error
                || (issue.severity == ReferenceSeverity::Warning
                    && issue.code == ReferenceIssueCode::MissingLocalTarget)
        });
        self.diagnostics.borrow_mut().extend(reported.map(reference_issue_diagnostic));
    }

    /// Records a failure's diagnostics, except those already recorded: a
    /// policy stop the CLI raised repeats the warnings it was given.
    pub(crate) fn failure(&self, error: &Report) {
        let mut recorded = self.diagnostics.borrow().clone();
        let fresh: Vec<Diagnostic> = failure_diagnostics(error)
            .into_iter()
            .filter(|diagnostic| match recorded.iter().position(|seen| seen == diagnostic) {
                Some(index) => {
                    recorded.remove(index);
                    false
                }
                None => true,
            })
            .collect();
        self.diagnostics.borrow_mut().extend(fresh);
    }

    /// Holds the composed document for a successful report.
    pub(crate) fn document(&self, text: String) {
        *self.document.borrow_mut() = Some(text);
    }

    /// Writes the report for the run's `result` and returns how the command
    /// ends: `Ok` on success, [`ReportedExit`] on failure.
    ///
    /// ## Errors
    ///
    /// [`ReportedExit`] for a failed run (status 1, or the status the run
    /// chose); a usage error unchanged, since it precedes report mode; a write
    /// error when stdout is closed.
    pub(crate) fn finish(self, result: Result<()>) -> Result<()> {
        let status = match result {
            Ok(()) => None,
            Err(error) if error.downcast_ref::<clap::Error>().is_some() => return Err(error),
            Err(error) => match error.downcast_ref::<ReportedExit>() {
                Some(exit) => Some(exit.0),
                None => {
                    self.failure(&error);
                    Some(1)
                }
            },
        };
        let document = self.document.into_inner().filter(|_| status.is_none());
        let report = ComposeReportJson {
            outcome: if status.is_none() { "success" } else { "failure" },
            diagnostics: self.diagnostics.into_inner(),
            document,
        };
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &report)
            .map_err(|error| eyre!("failed to write the composition report: {error}"))?;
        writeln!(stdout)
            .and_then(|()| stdout.flush())
            .map_err(|error| eyre!("failed to write the composition report: {error}"))?;
        match status {
            None => Ok(()),
            Some(code) => Err(ReportedExit(code).into()),
        }
    }
}

/// A failed report-mode run whose report is written: `md` exits with this
/// status and prints nothing more.
#[derive(Debug)]
pub struct ReportedExit(pub(crate) i32);

impl ReportedExit {
    /// The status `md` exits with.
    pub fn code(&self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for ReportedExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the composition failed (exit {}); see the report", self.0)
    }
}

impl std::error::Error for ReportedExit {}

/// A failure whose human message the CLI words itself, carrying the shared
/// diagnostics of the typed error it replaced.
#[derive(Debug)]
pub(crate) struct ClassifiedFailure {
    message: String,
    diagnostics: Vec<Diagnostic>,
}

impl ClassifiedFailure {
    /// `message`, classified by the replaced error's `diagnostics`.
    pub(crate) fn report(message: Report, diagnostics: Vec<Diagnostic>) -> Report {
        Self { message: message.to_string(), diagnostics }.into()
    }
}

impl std::fmt::Display for ClassifiedFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ClassifiedFailure {}

fn failure_diagnostics(error: &Report) -> Vec<Diagnostic> {
    for cause in error.chain() {
        if let Some(classified) = cause.downcast_ref::<ClassifiedFailure>() {
            return classified.diagnostics.clone();
        }
        if let Some(collection) = cause.downcast_ref::<crate::collection_failure::CollectionFailure>() {
            return collection.diagnostics().to_vec();
        }
        if cause.downcast_ref::<crate::io::DocumentArgumentError>().is_some() {
            return vec![Diagnostic {
                code: INPUT_READ_CODE.to_string(),
                severity: darkmatter::markdown::compose::expression::Severity::Error,
                message: error.to_string(),
                source: None,
                expected_type: None,
                actual_type: None,
                span: None,
                iteration_path: Default::default(),
            }];
        }
    }
    let root: &(dyn std::error::Error + 'static) = error.as_ref();
    error_diagnostics(root)
}

/// The report object; every field is always written.
#[derive(Serialize)]
struct ComposeReportJson {
    outcome: &'static str,
    /// The library's records, which serialize with `null` for absent values.
    diagnostics: Vec<Diagnostic>,
    document: Option<String>,
}
