//! The refresh and review lifecycle: selection, prepared runs, stage checks,
//! approval, promotion, and retention of local records.
//!
//! A platform refresh is one run in the gitignored state area
//! (`messenger/.research-state/runs/<platform>/<run_id>/`). [`prepare`] selects
//! due platforms, validates the operator's limits, and writes the three pass
//! inputs plus a Claudine sequence document; Claudine runs the passes under a
//! shared budget ledger. [`check`] judges the stage outputs (never an agent's
//! exit code), validates the candidate, and computes the delta. [`promote`]
//! is the only operation that changes accepted research: it applies the
//! approval policy ([`approval`]), then publishes the candidate with its
//! durable review record through the snapshot protocol. [`cleanup`] previews
//! and, only when asked, removes old local records.
//!
//! Messenger never spawns Claudine or an agent here; it prepares inputs and
//! prints the commands. Nothing here changes `CapabilitySet`, the roster,
//! the reviewed mappings, or delivery behavior.
//!
//! Design record: `messenger/features/2026-09-17-research-metadata-pipeline/architecture.md`.

pub mod approval;
pub mod check;
pub mod cleanup;
pub mod config;
pub mod input;
pub mod prepare;
pub mod promote;
pub mod records;
pub mod review;
pub mod select;
pub mod state;

pub use config::{RunConfigError, RunLimits};
pub use input::{DecisionReason, InputError, Maintainer};
pub use state::{RunId, RunRecord, RunStatus, Stage, StateArea};

use super::error::ResearchError;
use super::generate::GenerateError;
use super::publish::PublishError;
use state::StateError;

/// Lifecycle failures. A refusal leaves accepted research and the selected
/// snapshot unchanged.
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    Research(#[from] ResearchError),
    #[error(transparent)]
    Publish(#[from] PublishError),
    #[error(transparent)]
    Generate(Box<GenerateError>),
    #[error(transparent)]
    Config(#[from] RunConfigError),
    #[error(transparent)]
    Input(#[from] InputError),
    #[error("run {run_id} is {status}; {action} needs {needs}")]
    WrongStatus { run_id: String, status: RunStatus, action: &'static str, needs: &'static str },
    #[error("run {run_id} cannot be promoted: {}", reasons.join("; "))]
    NotEligible { run_id: String, reasons: Vec<String> },
    #[error("run {run_id} already used its {max} recovery attempts; start a new run", max = state::MAX_RECOVERY_ATTEMPTS)]
    RecoveryLimit { run_id: String },
    #[error("run {run_id}'s budget ledger is {state}: {guidance}")]
    Ledger { run_id: String, state: String, guidance: &'static str },
    #[error("run {run_id} cannot resume: {blocker} ({path}); reject it or resolve that run first")]
    OtherRunBlocks { run_id: String, path: String, blocker: String },
    #[error("another `messenger research prepare` holds {path}; run it again once that one finishes")]
    PrepareBusy { path: String },
    #[error("{platform} is not an active roster platform")]
    NotInRoster { platform: String },
    #[error("the roster does not load cleanly; run `messenger research validate`")]
    InvalidRoster,
    /// Claudine starts agents, and later `shell:` checks, in the Git top level
    /// of the launch directory, so a run prepared under a subdirectory of a
    /// work tree would resolve its repository-relative paths elsewhere.
    #[error(
        "the research root {root} is inside the Git work tree {top_level} but is not its top level; \
         prepare from the top level, or use a root outside any Git repository"
    )]
    NotRepositoryTopLevel { root: String, top_level: String },
}

impl From<GenerateError> for RefreshError {
    fn from(error: GenerateError) -> Self {
        Self::Generate(Box::new(error))
    }
}
