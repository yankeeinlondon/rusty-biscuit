//! What a push may publish: HEAD's scope receipt after a successful review,
//! and a validation receipt after a complete run of the gates on exactly the
//! outgoing tree.
//!
//! The two prove different things. Scope describes the committed tree and
//! never depends on the checkout or the gates. Validation describes what the
//! gates ran, so it is withheld (with its reason printed) whenever the run
//! may not have been the outgoing tree's or may not have finished. Reports
//! move to their durable home before a receipt names them; publication
//! failures are advisory, because CI then calculates and executes itself.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gix::ObjectId;
use sniff::os::OsType;

use super::review::PushReview;
use super::{Diagnostic, Flow, GateRun, HelperError, WithholdReason};

/// The notes ref HEAD's scope receipt is published on.
pub const SCOPE_NOTES_REF: &str = "refs/notes/ci-local/scope";
/// The staging subdirectory the gates write reports into.
pub(super) const REPORTS_DIRECTORY: &str = "reports";
/// `BISCUIT_CI_SCOPE_OUT` and `BISCUIT_CI_PLAN_OUT` of the gates.
pub(super) const GATE_SCOPE_FILE: &str = "gate-scope.json";
pub(super) const GATE_PLAN_FILE: &str = "gate-plan.json";
/// Written by the gates when they were interrupted after staging reports.
pub(super) const INTERRUPTED_MARKER: &str = "interrupted";
const MANIFEST: &str = "manifest.jsonl";
/// Overrides the durable evidence root (default `<home>/.rusty-biscuit/ci-evidence`).
pub const EVIDENCE_DIRECTORY_VARIABLE: &str = "BISCUIT_CI_EVIDENCE_DIR";

/// The fate of HEAD's scope receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopePublication {
    /// No receipt was recorded (no HEAD context, an all-zero base, a refused
    /// recording, or a blocked review).
    NotRecorded,
    Published { base: ObjectId },
    /// Recorded locally but not attached or pushed; CI plans itself.
    Failed { base: ObjectId, message: String },
}

/// The fate of the validation receipt once the gates ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validation {
    Withheld(WithholdReason),
    /// `record-cells` refused the staged reports.
    RecordFailed { report_dir: PathBuf },
    /// Recorded and published to the remote.
    Published {
        environment: String,
        cells: Option<usize>,
        report_dir: PathBuf,
    },
    /// Recorded locally; the remote did not take it (publish status 4).
    RecordedLocally { environment: String, report_dir: PathBuf },
    /// The observation could not be recorded.
    PublishFailed { report_dir: PathBuf },
}

/// The first nine hex digits, as the hook names commits.
pub fn short(id: ObjectId) -> String {
    let mut text = id.to_string();
    text.truncate(9);
    text
}

/// The CI environment of a host: macOS, native Windows, WSL2 (a Linux whose
/// kernel names Microsoft), or other Linux. `None` for any other OS.
pub fn ci_environment(os: &OsType, kernel: &str) -> Option<&'static str> {
    match os {
        OsType::MacOS => Some("macos-latest"),
        OsType::Windows => Some("windows-latest"),
        OsType::Linux if kernel.to_ascii_lowercase().contains("microsoft") => Some("wsl2-ubuntu"),
        OsType::Linux => Some("ubuntu-latest"),
        _ => None,
    }
}

/// Attaches and pushes HEAD's scope receipt, if the review recorded one.
pub(super) fn publish_scope(flow: &mut Flow<'_, '_>, remote_name: &str, review: &PushReview) {
    let Some(receipt) = &review.scope_receipt else {
        return;
    };
    let head = short(review.head);
    let base = short(receipt.base);
    let published = std::fs::read(&receipt.path)
        .map_err(|error| format!("cannot read {}: {error}", receipt.path.display()))
        .and_then(|bytes| flow.session.publish_scope_note(&bytes, review.head));
    flow.report.outcome.scope = match published {
        Ok(()) => {
            flow.info(format!(
                "Published scope evidence for {head} (base {base}) on {SCOPE_NOTES_REF}."
            ));
            ScopePublication::Published { base: receipt.base }
        }
        Err(message) => {
            flow.say(Diagnostic::Advisory(format!(
                "Scope evidence PUBLICATION failed for {head} (calculated against {base}, not published to {remote_name}); CI will calculate scope itself. {message}"
            )));
            ScopePublication::Failed {
                base: receipt.base,
                message,
            }
        }
    };
}

/// Decides, retains, records, and publishes the validation receipt of a
/// gate run. Returns `false` when the staged reports could not be retained,
/// so the caller keeps the staging directory for the developer to read.
pub(super) fn publish_validation(
    flow: &mut Flow<'_, '_>,
    remote_name: &str,
    review: &PushReview,
    gates: &GateRun,
    staging: &Path,
) -> bool {
    let reports = staging.join(REPORTS_DIRECTORY);
    if let Some(reason) = withhold_reason(flow, review, gates, staging) {
        return withhold(flow, reason);
    }
    let Some(first) = &review.first else {
        return withhold(flow, WithholdReason::NoReviewedComparison);
    };
    let Some(environment) = flow.session.environment() else {
        return withhold(flow, WithholdReason::UnsupportedOs);
    };
    let head = short(review.head);

    // The staging directory dies with this process, so the reports move to
    // their durable home first and the receipt names that location; a receipt
    // is never written for reports that are not there.
    let report_dir = match evidence_root(flow) {
        Ok(root) => root.join(review.head.to_string()).join(&environment),
        Err(problem) => {
            flow.say(Diagnostic::Advisory(format!(
                "Could not retain the staged reports: {problem}; no receipt is published."
            )));
            withhold(flow, WithholdReason::ReportRetentionFailed);
            return false;
        }
    };
    if let Err(error) = std::fs::create_dir_all(&report_dir).and_then(|()| copy_tree(&reports, &report_dir)) {
        flow.say(Diagnostic::Advisory(format!(
            "Could not retain the staged reports under {} ({error}); no receipt is published.",
            report_dir.display()
        )));
        withhold(flow, WithholdReason::ReportRetentionFailed);
        return false;
    }

    // No prior receipt is fed to record-cells: the note is a collection of
    // observations, and `observation_notes.py` unions this run into it after
    // fetching the remote ref itself.
    let prior = staging.join("prior-receipt.json");
    let receipt = staging.join("receipt.json");
    let scripts = flow.session.root().join("scripts").join("ci");
    let checkout = flow.session.root().to_path_buf();
    let base = first.base.to_string();
    let record: Vec<OsString> = vec![
        scripts.join("local_evidence.py").into(),
        "record-cells".into(),
        "--plan".into(),
        first.raw_plan.clone().into(),
        "--stage".into(),
        reports.into(),
        "--base".into(),
        base.into(),
        "--head".into(),
        review.head.to_string().into(),
        "--environment".into(),
        environment.clone().into(),
        "--report-dir".into(),
        report_dir.clone().into(),
        "--prior-receipt".into(),
        prior.clone().into(),
    ];
    let recorded = std::fs::write(&prior, b"")
        .map_err(|error| HelperError::failed(format!("cannot write {}: {error}", prior.display())))
        .and_then(|()| flow.helper(&checkout, &record))
        .and_then(|recorded| {
            std::fs::write(&receipt, &recorded.stdout)
                .map(|()| recorded)
                .map_err(|error| HelperError::failed(format!("cannot write {}: {error}", receipt.display())))
        });
    let recorded = match recorded {
        Ok(recorded) => recorded,
        Err(error) => {
            let detail = match error {
                HelperError::Interrupted(code) => format!("it was interrupted (exit {code})"),
                HelperError::Failed { detail, .. } => detail,
            };
            flow.say(Diagnostic::Advisory(format!(
                "No validation receipt for {head}: record-cells refused the staged reports. {detail}"
            )));
            flow.report.outcome.validation = Some(Validation::RecordFailed { report_dir });
            return true;
        }
    };
    let cells = serde_json::from_slice::<serde_json::Value>(&recorded.stdout)
        .ok()
        .and_then(|receipt| receipt.get("cells").and_then(|cells| cells.as_array()).map(Vec::len));

    let publish: Vec<OsString> = vec![
        scripts.join("observation_notes.py").into(),
        "publish-receipt".into(),
        "--receipt".into(),
        receipt.into(),
        "--plan".into(),
        first.raw_plan.clone().into(),
        "--head".into(),
        review.head.to_string().into(),
        "--environment".into(),
        environment.clone().into(),
        "--remote".into(),
        remote_name.into(),
    ];
    let published = flow.helper(&checkout, &publish);
    flow.report.outcome.validation = Some(match published {
        Ok(_) => {
            let count = cells.map_or_else(|| "the".to_owned(), |cells| cells.to_string());
            flow.info(format!(
                "Published {count} {environment} cell(s) of evidence for {head} (base {}). Reports retained under {}.",
                short(first.base),
                report_dir.display()
            ));
            Validation::Published {
                environment,
                cells,
                report_dir,
            }
        }
        // `observation_notes.py`'s status for "recorded locally, not pushed".
        Err(HelperError::Failed { code: Some(4), .. }) => {
            flow.say(Diagnostic::Advisory(format!(
                "Local evidence recorded but could not be published; CI will retain {environment}."
            )));
            Validation::RecordedLocally {
                environment,
                report_dir,
            }
        }
        Err(error) => {
            let detail = match error {
                HelperError::Interrupted(code) => format!("it was interrupted (exit {code})"),
                HelperError::Failed { detail, .. } => detail,
            };
            flow.say(Diagnostic::Advisory(format!(
                "No validation receipt for {head}: the observation could not be recorded. {detail}"
            )));
            Validation::PublishFailed { report_dir }
        }
    });
    true
}

/// The first reason the gate run is not exact-tree, complete-run evidence
/// of the outgoing HEAD, checked in the order a developer can act on.
fn withhold_reason(
    flow: &mut Flow<'_, '_>,
    review: &PushReview,
    gates: &GateRun,
    staging: &Path,
) -> Option<WithholdReason> {
    let reports = staging.join(REPORTS_DIRECTORY);
    if gates.interrupted || reports.join(INTERRUPTED_MARKER).exists() {
        return Some(WithholdReason::Interrupted);
    }
    if !review.pushes_head {
        return Some(WithholdReason::OutgoingHeadMissing);
    }
    if !gates.selection.is_empty() {
        return Some(WithholdReason::SelectionOverride);
    }
    if review.initial_dirty {
        return Some(WithholdReason::InitialDirty);
    }
    if flow.session.head().map(|head| head.commit).ok() != Some(review.head) {
        return Some(WithholdReason::HeadChanged);
    }
    if flow.session.is_dirty().unwrap_or(true) {
        return Some(WithholdReason::FinalDirty);
    }
    if gates.incomplete {
        return Some(WithholdReason::Incomplete);
    }
    if gates.plan_in.is_none() {
        return Some(WithholdReason::NoReviewedComparison);
    }
    if !nonempty(&staging.join(GATE_PLAN_FILE)) {
        return Some(WithholdReason::NoLocalPlan);
    }
    if !nonempty(&reports.join(MANIFEST)) {
        return Some(WithholdReason::EmptyManifest);
    }
    None
}

fn withhold(flow: &mut Flow<'_, '_>, reason: WithholdReason) -> bool {
    flow.say(Diagnostic::Withheld(reason));
    flow.report.outcome.validation = Some(Validation::Withheld(reason));
    true
}

fn nonempty(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

/// `BISCUIT_CI_EVIDENCE_DIR`, else `<home>/.rusty-biscuit/ci-evidence`; a
/// relative root is taken against the invoking checkout.
fn evidence_root(flow: &mut Flow<'_, '_>) -> Result<PathBuf, String> {
    let root = match flow
        .session
        .variable(EVIDENCE_DIRECTORY_VARIABLE)
        .filter(|value| !value.is_empty())
    {
        Some(value) => PathBuf::from(value),
        None => flow
            .session
            .home()
            .ok_or_else(|| {
                format!("no home directory is known and {EVIDENCE_DIRECTORY_VARIABLE} is not set")
            })?
            .join(".rusty-biscuit")
            .join("ci-evidence"),
    };
    Ok(if root.is_absolute() {
        root
    } else {
        flow.session.root().join(root)
    })
}

/// Copies `source`'s contents over `destination`, keeping what is already
/// there: a retry's reports overlay the reports of reused cells.
fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
