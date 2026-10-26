//! The branch steps of a confirmed removal: run after the checkout and its
//! registration are gone, each bound to what it was approved against.
//!
//! Consent to delete the checkout says nothing about commits, so every
//! protection stays: a local branch is deleted only at its approved tip,
//! a conditionally approved one only while it is still safe when a later
//! run carries it out, and a remote branch only through its validated push
//! endpoint and under its lease. A step that cannot run is kept and reported
//! (the run exits 1); it never asks about the checkout again, and a branch
//! already gone counts as done.

use std::path::Path;

use crate::error::WorktreeError;
use crate::remove::recovery::compact::{BranchAction, BranchStep, StepApproval, StepState, UnboundTarget};
use crate::remove::remote::{delete_remote_branch, push_endpoints, remote_branch_absent};
use crate::remove::remove_local_branch;

/// Rechecks a conditionally approved branch on a run after the one that
/// assessed it (resume, handoff), with the same proof the second run of a
/// handoff requires.
pub trait Revalidate {
    /// `Err` with the reason when local branch `name` at `tip` is no longer
    /// safe to delete. `remote_deleted` says the operation also deletes the
    /// branch on origin, so that copy (and its open PR) is no proof.
    fn still_safe(&self, name: &str, tip: &str, remote_deleted: bool) -> Result<(), String>;
}

/// [`Revalidate`] for a caller with nothing to check against: a conditional
/// step is kept.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unverifiable;

impl Revalidate for Unverifiable {
    fn still_safe(&self, _name: &str, _tip: &str, _remote_deleted: bool) -> Result<(), String> {
        Err("its safety could not be checked again".into())
    }
}

/// One branch step's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchResult {
    pub target: UnboundTarget,
    pub outcome: BranchOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchOutcome {
    Deleted,
    /// It was already gone, which completes the step.
    AlreadyAbsent,
    /// Kept, with a plain reason (the tip moved, it is no longer safe, the
    /// endpoint changed, the lease failed, or Git's own error).
    Kept(String),
    /// Approved by an earlier `wt` whose record cannot say where or at what
    /// it was approved, so it is never run.
    NotCarriedOver,
}

impl BranchOutcome {
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Deleted | Self::AlreadyAbsent)
    }
}

impl BranchResult {
    /// A step an earlier run of the operation finished.
    pub(crate) fn recorded(step: &BranchStep) -> Self {
        let outcome = match (&step.action, step.state) {
            (BranchAction::Unbound { .. }, _) => BranchOutcome::NotCarriedOver,
            (_, StepState::Done) => BranchOutcome::Deleted,
            (_, StepState::Kept | StepState::Pending) => BranchOutcome::Kept("an earlier run kept it".into()),
        };
        Self { target: target(&step.action), outcome }
    }
}

/// What `action` deletes.
pub(crate) fn target(action: &BranchAction) -> UnboundTarget {
    match action {
        BranchAction::Local { name, .. } => UnboundTarget::Local { name: name.clone() },
        BranchAction::Remote { remote, destination, .. } => UnboundTarget::Remote { remote: remote.clone(), name: destination.clone() },
        BranchAction::Unbound { target } => target.clone(),
    }
}

/// Runs one pending step. `fresh` says the approval was given in this run,
/// so a conditional step was just assessed and its endpoint just read;
/// `remote_deleted` says the operation has a remote step too.
pub(crate) fn run(base: &Path, step: &BranchStep, revalidate: &dyn Revalidate, fresh: bool, remote_deleted: bool) -> BranchOutcome {
    match &step.action {
        BranchAction::Unbound { .. } => BranchOutcome::NotCarriedOver,
        BranchAction::Local { name, expected_tip } => {
            if crate::git_metadata::commit_of(base, &format!("refs/heads/{name}")) == Some(None) {
                return BranchOutcome::AlreadyAbsent;
            }
            if !fresh
                && step.approval == StepApproval::Conditional
                && let Err(why) = revalidate.still_safe(name, expected_tip, remote_deleted)
            {
                return BranchOutcome::Kept(why);
            }
            match remove_local_branch(base, name, expected_tip) {
                Ok(()) => BranchOutcome::Deleted,
                Err(WorktreeError::BranchMoved { found, .. }) => {
                    BranchOutcome::Kept(format!("it moved to {found} after it was checked"))
                }
                Err(error) => BranchOutcome::Kept(error.to_string()),
            }
        }
        BranchAction::Remote { destination, endpoint, expected_head, .. } => {
            if !fresh {
                match push_endpoints(base) {
                    Ok(endpoints) if endpoints.iter().any(|current| current == endpoint) => {}
                    Ok(_) => return BranchOutcome::Kept(format!("origin no longer pushes to {endpoint}")),
                    Err(error) => return BranchOutcome::Kept(format!("origin's push URL could not be read: {error}")),
                }
            }
            match delete_remote_branch(base, endpoint, destination, expected_head) {
                Ok(()) => BranchOutcome::Deleted,
                Err(_) if remote_branch_absent(base, endpoint, destination) => BranchOutcome::AlreadyAbsent,
                Err(error) => BranchOutcome::Kept(error),
            }
        }
    }
}
