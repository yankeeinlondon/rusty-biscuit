//! The data-output collector: one value on stdout for a run, success or
//! failure, and nothing on stderr.
//!
//! [`ReportSink`] gathers each diagnostic when the run reports it (in discovery
//! order), the document, the reference totals, the timings, and what lifecycle
//! steps printed. [`ReportSink::finish`] adds the failure's diagnostics and
//! writes the envelope. Codes and messages come from the library's shared
//! diagnostics (`darkmatter::markdown::compose::report_diagnostics`); this
//! module only collects and serializes them. Nothing is evaluated or rendered
//! again.

use std::sync::Mutex;
use std::time::Duration;

use biscuit_terminal::output::{OutputError, OutputTarget, render_output};
use color_eyre::eyre::{Report, Result};
use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
use darkmatter::markdown::compose::report_diagnostics::{
    error_diagnostics, reference_issue_diagnostic, warning_diagnostic,
};
use darkmatter::markdown::compose::{ComposePerfReport, ComposeWarning};
use darkmatter::markdown::output::command::{
    DocumentEnvelope, LifecycleOutputEntry, Outcome, PerfData, PerfStage, ReferenceSummary,
};
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceSeverity, ReferenceValidationReport};
use renderable::OutputOptions;

use super::{OPEN_FAILED_CODE, ResolvedOutput, SERIALIZATION_CODE, output_diagnostic, stdout_text, width_warning_diagnostic};
use crate::artifact::{ArtifactOpener, OutputArtifact, ProcessOpener, open_artifact_with};

/// The run's envelope, filled as it proceeds.
///
/// Locks because lifecycle delivery requires `Sync`; each field is only ever
/// touched for one push.
pub struct ReportSink {
    target: OutputTarget,
    options: OutputOptions,
    show: bool,
    file: Option<String>,
    diagnostics: Mutex<Vec<Diagnostic>>,
    document: Mutex<Option<String>>,
    summary: Mutex<Option<ReferenceSummary>>,
    perf: Option<Mutex<Vec<PerfStage>>>,
    lifecycle: Mutex<Vec<LifecycleOutputEntry>>,
}

impl ReportSink {
    /// A collector for `output`, with `file` as the command line gave it and
    /// the width warning already recorded. `perf` makes the envelope carry
    /// timings; `show` also opens the serialized value.
    #[must_use]
    pub fn new(output: &ResolvedOutput, file: Option<String>, show: bool, perf: bool) -> Self {
        let diagnostics = output.width_warning.as_deref().map(width_warning_diagnostic).into_iter().collect();
        Self {
            target: output.target,
            options: output.options.clone(),
            show,
            file,
            diagnostics: Mutex::new(diagnostics),
            document: Mutex::new(None),
            summary: Mutex::new(None),
            perf: perf.then(|| Mutex::new(Vec::new())),
            lifecycle: Mutex::new(Vec::new()),
        }
    }

    /// Records `warnings`, as a formatted run would print them.
    pub fn warnings(&self, warnings: &[ComposeWarning]) {
        self.diagnostics.lock().expect("sink lock").extend(warnings.iter().map(warning_diagnostic));
    }

    /// Records `diagnostics` the library produced for a failure.
    pub fn diagnostics(&self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics.lock().expect("sink lock").extend(diagnostics);
    }

    /// Records the reference check's totals and the issues a formatted run
    /// prints: errors, and missing targets inside inserted text.
    pub fn references(&self, report: &ReferenceValidationReport) {
        self.summary(ReferenceSummary::from_report(report));
        let reported = report.issues.iter().filter(|issue| {
            issue.severity == ReferenceSeverity::Error
                || (issue.severity == ReferenceSeverity::Warning && issue.code == ReferenceIssueCode::MissingLocalTarget)
        });
        self.diagnostics.lock().expect("sink lock").extend(reported.map(reference_issue_diagnostic));
    }

    /// Records the totals of a reference check that found nothing to report.
    pub fn summary(&self, summary: ReferenceSummary) {
        *self.summary.lock().expect("sink lock") = Some(summary);
    }

    /// Records a failure's diagnostics, except those already recorded: a
    /// policy stop the CLI raised repeats the warnings it was given.
    pub fn failure(&self, error: &Report) {
        let mut diagnostics = self.diagnostics.lock().expect("sink lock");
        let mut recorded = diagnostics.clone();
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
        diagnostics.extend(fresh);
    }

    /// Holds the document for a successful result.
    pub fn document(&self, text: String) {
        *self.document.lock().expect("sink lock") = Some(text);
    }

    /// Records one timed stage, when timings were asked for.
    pub fn perf_stage(&self, name: &str, elapsed: Duration) {
        if let Some(perf) = &self.perf {
            perf.lock().expect("sink lock").push(PerfStage {
                name: name.to_string(),
                milliseconds: elapsed.as_secs_f64() * 1000.0,
            });
        }
    }

    /// Records the pipeline's per-stage timings.
    pub fn perf_pipeline(&self, report: &ComposePerfReport) {
        for metric in &report.metrics {
            self.perf_stage(&metric.stage.to_string(), metric.elapsed);
        }
    }

    /// Records what one lifecycle step printed.
    pub fn lifecycle_output(&self, stage: &str, step: &str, text: &str) {
        self.lifecycle.lock().expect("sink lock").push(LifecycleOutputEntry {
            stage: stage.to_string(),
            step: step.to_string(),
            text: text.to_string(),
        });
    }

    /// Writes the one value for the run's `result` and returns how the command
    /// ends: `Ok` on success, [`ReportedExit`] on failure.
    ///
    /// An interrupted run (status 130) writes nothing: the status reports a
    /// signal, not an outcome. A write to a closed stdout fails without a
    /// second payload.
    ///
    /// ## Errors
    ///
    /// [`ReportedExit`] for a failed run; a usage error unchanged, since no
    /// command ran.
    pub fn finish(self, result: Result<()>) -> Result<()> {
        self.finish_with(result, &ProcessOpener::detect())
    }

    fn finish_with(self, result: Result<()>, opener: &dyn ArtifactOpener) -> Result<()> {
        let status = match result {
            Ok(()) => None,
            Err(error) if error.downcast_ref::<clap::Error>().is_some() => return Err(error),
            Err(error) => match error.downcast_ref::<ReportedExit>() {
                Some(exit) if exit.0 == INTERRUPTED => return Err(error),
                Some(exit) => Some(exit.0),
                None => {
                    self.failure(&error);
                    Some(1)
                }
            },
        };
        let (target, options, show) = (self.target, self.options.clone(), self.show);
        let mut status = status;
        let mut envelope = self.into_envelope(status.is_none());

        let mut rendered = match render_output(&envelope, target, &options) {
            Ok(rendered) => rendered.text,
            Err(error) => {
                status = Some(1);
                envelope = serialization_failure(envelope, &error);
                match render_output(&envelope, target, &options) {
                    Ok(rendered) => rendered.text,
                    // The stripped value holds no document and no data of the
                    // run's; nothing else can be said in this format.
                    Err(_) => return Err(ReportedExit(1).into()),
                }
            }
        };

        if show {
            let artifact = OutputArtifact { content: rendered.clone(), extension: target.extension(), label: target.as_str() };
            if let Err(error) = open_artifact_with(&artifact, opener) {
                status = Some(1);
                envelope.outcome = Outcome::Failure;
                envelope.document = None;
                envelope.diagnostics.push(output_diagnostic(OPEN_FAILED_CODE, Severity::Error, &error.to_string()));
                rendered = match render_output(&envelope, target, &options) {
                    Ok(rendered) => rendered.text,
                    Err(_) => return Err(ReportedExit(1).into()),
                };
            }
        }

        // A closed stdout has nowhere to say so; the status reports it.
        if super::write_stdout_text(&stdout_text(rendered, target)).is_err() {
            return Err(ReportedExit(1).into());
        }
        match status {
            None => Ok(()),
            Some(code) => Err(ReportedExit(code).into()),
        }
    }

    fn into_envelope(self, success: bool) -> DocumentEnvelope {
        DocumentEnvelope {
            file: self.file,
            outcome: if success { Outcome::Success } else { Outcome::Failure },
            document: self.document.into_inner().expect("sink lock").filter(|_| success),
            diagnostics: self.diagnostics.into_inner().expect("sink lock"),
            summary: self.summary.into_inner().expect("sink lock"),
            perf: self.perf.map(|perf| PerfData { stages: perf.into_inner().expect("sink lock") }),
            lifecycle_output: self.lifecycle.into_inner().expect("sink lock"),
        }
    }
}

/// `envelope` reduced to what any format can hold, naming what `error` could
/// not write.
fn serialization_failure(mut envelope: DocumentEnvelope, error: &OutputError) -> DocumentEnvelope {
    envelope.outcome = Outcome::Failure;
    envelope.document = None;
    envelope.diagnostics = vec![output_diagnostic(SERIALIZATION_CODE, Severity::Error, &error.to_string())];
    envelope.lifecycle_output = Vec::new();
    envelope
}

/// The status of a run the user interrupted.
pub const INTERRUPTED: i32 = 130;

/// A failed run whose failure is already reported: `md` exits with this status
/// and prints nothing more.
#[derive(Debug)]
pub struct ReportedExit(pub i32);

impl ReportedExit {
    /// The status `md` exits with.
    #[must_use]
    pub fn code(&self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for ReportedExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the command failed (exit {}); see its report", self.0)
    }
}

impl std::error::Error for ReportedExit {}

/// A failure whose human message the CLI words itself, carrying the shared
/// diagnostics of the typed error it replaced.
#[derive(Debug)]
pub struct ClassifiedFailure {
    message: String,
    diagnostics: Vec<Diagnostic>,
}

impl ClassifiedFailure {
    /// `message`, classified by the replaced error's `diagnostics`.
    pub fn report(message: Report, diagnostics: Vec<Diagnostic>) -> Report {
        Self { message: message.to_string(), diagnostics }.into()
    }
}

impl std::fmt::Display for ClassifiedFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ClassifiedFailure {}

/// The shared records a failed run reports for `error`.
pub(crate) fn failure_diagnostics(error: &Report) -> Vec<Diagnostic> {
    for cause in error.chain() {
        if let Some(classified) = cause.downcast_ref::<ClassifiedFailure>() {
            return classified.diagnostics.clone();
        }
        if let Some(collection) = cause.downcast_ref::<crate::collection_failure::CollectionFailure>() {
            return collection.diagnostics().to_vec();
        }
        if let Some(argument) = cause.downcast_ref::<crate::io::DocumentArgumentError>() {
            return vec![argument.diagnostic()];
        }
    }
    let root: &(dyn std::error::Error + 'static) = error.as_ref();
    error_diagnostics(root)
}

#[cfg(test)]
mod tests {
    use color_eyre::eyre::eyre;
    use renderable::{TerminalProfile, Width};

    use super::*;

    fn sink(target: OutputTarget, show: bool) -> ReportSink {
        let output = ResolvedOutput {
            target,
            options: OutputOptions::new(Width::Source, TerminalProfile::plain()),
            explicit_width: false,
            width_warning: None,
        };
        ReportSink::new(&output, Some("doc.md".to_string()), show, false)
    }

    struct Refusing;

    impl ArtifactOpener for Refusing {
        fn open(&self, _: &std::path::Path) -> std::io::Result<()> {
            Err(std::io::Error::other("no default application"))
        }
    }

    #[test]
    fn a_failure_keeps_earlier_diagnostics_and_drops_the_document() {
        let sink = sink(OutputTarget::Json, false);
        sink.document("body".to_string());
        sink.diagnostics(vec![output_diagnostic("dm.test.first", Severity::Warning, "first")]);
        sink.failure(&eyre!("broken"));
        let envelope = sink.into_envelope(false);
        assert_eq!(envelope.outcome, Outcome::Failure);
        assert_eq!(envelope.document, None);
        assert_eq!(envelope.diagnostics[0].code, "dm.test.first");
        assert_eq!(envelope.diagnostics.len(), 2);
    }

    #[test]
    fn an_interrupted_run_writes_no_value_and_keeps_its_status() {
        let result = sink(OutputTarget::Json, false).finish_with(Err(ReportedExit(INTERRUPTED).into()), &Refusing);
        let error = result.unwrap_err();
        assert_eq!(error.downcast_ref::<ReportedExit>().unwrap().code(), INTERRUPTED);
    }

    #[test]
    fn a_usage_error_passes_through_unreported() {
        use clap::CommandFactory as _;
        let usage = crate::args::Cli::command().error(clap::error::ErrorKind::ArgumentConflict, "bad");
        let error = sink(OutputTarget::Json, false).finish_with(Err(usage.into()), &Refusing).unwrap_err();
        assert!(error.downcast_ref::<clap::Error>().is_some());
    }

    #[test]
    fn an_open_failure_fails_the_run() {
        let sink = sink(OutputTarget::Yaml, true);
        sink.document("body".to_string());
        let error = sink.finish_with(Ok(()), &Refusing).unwrap_err();
        assert_eq!(error.downcast_ref::<ReportedExit>().unwrap().code(), 1);
    }

    #[test]
    fn a_serialization_failure_says_what_could_not_be_written() {
        let failure = OutputError::Serialize {
            target: OutputTarget::Toml,
            path: Some("/items/1".to_string()),
            message: "TOML cannot represent a null inside an array".to_string(),
        };
        let envelope = serialization_failure(sink(OutputTarget::Toml, false).into_envelope(true), &failure);
        assert_eq!(envelope.outcome, Outcome::Failure);
        assert_eq!(envelope.diagnostics[0].code, SERIALIZATION_CODE);
        assert!(envelope.diagnostics[0].message.contains("/items/1"));
    }
}
