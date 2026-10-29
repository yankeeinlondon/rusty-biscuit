//! Delivery of the schema triggers discovery skipped as invalid
//! (`dm.schema.invalid_trigger`) for the commands that are not `md compose`.
//!
//! The library only records findings; each command decides where they go.
//! Ordinary output writes them to stderr as warning rows, the JSON reports
//! carry them as `{source, code, path, message}` rows, and an invocation
//! reports each trigger once however many documents discover it.

use std::collections::HashSet;

use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::prelude::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::schemas::{InvalidTriggerFinding, TriggerSource};
use serde::Serialize;
use std::path::PathBuf;

/// The findings one invocation has already delivered, keyed by the trigger's
/// full identity (never its file name), so a trigger shared by several
/// documents is reported on the first document that discovers it.
#[derive(Default)]
pub(crate) struct DeliveredFindings {
    seen: HashSet<TriggerSource>,
}

impl DeliveredFindings {
    /// The findings in `findings` not delivered before, in discovery order;
    /// each is marked delivered.
    pub(crate) fn first_seen(&mut self, findings: &[InvalidTriggerFinding]) -> Vec<InvalidTriggerFinding> {
        findings
            .iter()
            .filter(|finding| self.seen.insert(finding.identity().clone()))
            .cloned()
            .collect()
    }
}

/// One finding in a JSON report; the same shape as a schema advisory row.
/// `path` is the trigger file, `null` for an embedded trigger.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct FindingRow {
    pub source: &'static str,
    pub code: &'static str,
    pub path: Option<PathBuf>,
    pub message: String,
}

impl From<&InvalidTriggerFinding> for FindingRow {
    fn from(finding: &InvalidTriggerFinding) -> Self {
        Self {
            source: finding.source_family(),
            code: finding.code(),
            path: finding.source().path().map(PathBuf::from),
            message: finding.message(),
        }
    }
}

/// Writes each finding to stderr as one warning row, the presentation
/// `md compose` gives the same warning.
pub(crate) fn emit_warnings(findings: &[InvalidTriggerFinding], terminal: &Terminal) {
    for finding in findings {
        eprintln!("{}", warning_row(finding, terminal));
    }
}

fn warning_row(finding: &InvalidTriggerFinding, terminal: &Terminal) -> String {
    Status::from_prose(Prose::escape_text_outside_code_spans(&finding.message()))
        .with_line_breaks(LineBreaks::Hard)
        .state(StatusState::Warning)
        .render(terminal)
}
