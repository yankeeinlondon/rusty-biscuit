//! The compact recovery record (`record_version` 2): the approval a confirmed
//! removal saves before its first unlock, move, or deletion, and the progress
//! it has made since.
//!
//! Unlike the legacy record ([`super::record`]), a compact record **is** the
//! approval. A later `wt remove` of the same checkout finishes the operation
//! from it without asking again, for as long as the recorded objects (the
//! checkout's identity, the repository, the worktree record) still match;
//! the record has no expiry. It never authorizes deleting a replacement:
//! every identity is checked again before it is acted on, and an alias only
//! finds the record, never a directory.
//!
//! `progress` is a hint about where to look. A resumed operation inspects the
//! paths and the registration as they are now; a marker alone never proves a
//! step finished.
//!
//! [`parse`] is strict: every field is required, `null` is never read as
//! absent, unknown or repeated keys and trailing content are refused, and an
//! unknown `record_version` is refused before the rest is read.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sniff::filesystem::query::{FileIdentity, NativeString};

use super::OperationId;
use super::record::{QUARANTINE_PREFIX, RecordInvalid, RepositoryBinding};
use crate::strict_json;

/// The `record_version` of a compact record.
pub const COMPACT_RECORD_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactRecord {
    pub record_version: u32,
    pub operation_id: OperationId,
    /// Milliseconds since the Unix epoch.
    pub created_at_ms: u64,
    /// The name the user passed to `wt remove`.
    pub requested_name: String,
    /// Other names that resolve to this checkout (branch, directory). Lookup
    /// keys only, never filesystem authority; empty when there are none.
    pub aliases: Vec<String>,
    pub original: NativeString,
    /// What is deleted: a hidden sibling of `original` named with
    /// [`QUARANTINE_PREFIX`] when [`DeletionMode::Moved`], `original` itself
    /// when [`DeletionMode::InPlace`].
    pub deletion_path: NativeString,
    pub deletion_mode: DeletionMode,
    /// The checkout directory's identity, which the move preserves.
    pub checkout_identity: FileIdentity,
    pub repository: RepositoryBinding,
    /// The worktree was locked and the user approved unlocking it.
    pub unlock_needed: bool,
    /// Run in order, after the registration is removed.
    pub branch_steps: Vec<BranchStep>,
    pub progress: Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionMode {
    /// The checkout is moved to a hidden sibling, then deleted there.
    Moved,
    /// The checkout could not be moved safely and is deleted where it is.
    InPlace,
}

/// The last step the operation recorded as finished, in flowchart order.
/// Branch steps keep their own [`StepState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Progress {
    /// The record is durable; nothing has been mutated.
    Approved,
    /// The worktree's lock is gone, or there was none.
    Unlocked,
    /// The checkout is at `deletion_path`: moved, or marked for deletion in
    /// place.
    Placed,
    ContentRemoved,
    /// Git's registration and the copy record are gone.
    Unregistered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchStep {
    pub action: BranchAction,
    pub approval: StepApproval,
    pub state: StepState,
}

/// A branch deletion, bound to what it was approved against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BranchAction {
    /// Deletes local branch `name` while its tip is `expected_tip`.
    Local { name: String, expected_tip: String },
    /// Deletes `destination` through the validated push `endpoint`, leased to
    /// `expected_head`.
    Remote { remote: String, destination: String, endpoint: String, expected_head: String },
    /// A legacy branch step whose approval cannot be rebuilt (a legacy
    /// remote step binds no endpoint or head). It is never run; its state is
    /// always [`StepState::Kept`], and the removal reports it and exits 1.
    Unbound { target: UnboundTarget },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum UnboundTarget {
    Local { name: String },
    Remote { remote: String, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepApproval {
    /// The user forced it; only the tip, endpoint, and lease are rechecked.
    Explicit,
    /// Approved because the branch was safe to delete; safety is
    /// revalidated before the step runs.
    Conditional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Pending,
    /// Deleted, or already absent.
    Done,
    /// Left in place: the branch changed, safety failed, or the step is
    /// [`BranchAction::Unbound`].
    Kept,
}

/// The `record_version` of a stored record, read before anything else so an
/// unknown version is refused without interpreting the rest.
pub fn record_version(bytes: &[u8]) -> Result<u64, RecordInvalid> {
    let members = strict_json::members(bytes)
        .ok_or_else(|| RecordInvalid::Malformed("not one JSON object with distinct keys".into()))?;
    members
        .get("record_version")
        .ok_or_else(|| RecordInvalid::Malformed("record_version is missing or not a number".into()))
}

/// Parses and validates a compact record.
pub fn parse(bytes: &[u8]) -> Result<CompactRecord, RecordInvalid> {
    let version = record_version(bytes)?;
    if version != u64::from(COMPACT_RECORD_VERSION) {
        return Err(RecordInvalid::UnsupportedVersion(version));
    }
    let record: CompactRecord =
        serde_json::from_slice(bytes).map_err(|error| RecordInvalid::Malformed(error.to_string()))?;
    record.validate()?;
    Ok(record)
}

impl CompactRecord {
    /// Checks what serde cannot: path shapes, the deletion path's relation
    /// to the original, the formats of bound values, and that branch steps
    /// only finish after the registration is gone.
    pub fn validate(&self) -> Result<(), RecordInvalid> {
        let malformed = |what: &str| Err(RecordInvalid::Malformed(what.to_string()));
        if self.record_version != COMPACT_RECORD_VERSION {
            return Err(RecordInvalid::UnsupportedVersion(self.record_version.into()));
        }
        if self.requested_name.is_empty() || self.aliases.iter().any(String::is_empty) {
            return malformed("a requested name or alias is empty");
        }
        let original = absolute(&self.original, "original")?;
        let deletion = absolute(&self.deletion_path, "deletion_path")?;
        if original.parent().is_none() {
            return malformed("original has no parent directory");
        }
        match self.deletion_mode {
            DeletionMode::Moved => {
                let hidden = deletion.file_name().is_some_and(|name| name.as_encoded_bytes().starts_with(QUARANTINE_PREFIX.as_bytes()));
                if deletion.parent() != original.parent() || !hidden {
                    return malformed("a moved checkout's deletion_path is not a hidden sibling of the original");
                }
            }
            DeletionMode::InPlace if deletion != original => {
                return malformed("an in-place deletion_path is not the original");
            }
            DeletionMode::InPlace => {}
        }
        let common = absolute(&self.repository.common_dir, "common_dir")?;
        let admin = absolute(&self.repository.admin_dir, "admin_dir")?;
        if admin.parent() != Some(common.join("worktrees").as_path()) {
            return malformed("admin_dir is not a worktree record of common_dir");
        }
        let (mut locals, mut remotes) = (0, 0);
        for step in &self.branch_steps {
            step.validate()?;
            match &step.action {
                BranchAction::Local { .. } | BranchAction::Unbound { target: UnboundTarget::Local { .. } } => locals += 1,
                BranchAction::Remote { .. } | BranchAction::Unbound { target: UnboundTarget::Remote { .. } } => remotes += 1,
            }
            if step.state != StepState::Pending && self.progress < Progress::Unregistered && !step.is_unbound() {
                return malformed("a branch step finished before the registration was removed");
            }
        }
        if locals > 1 || remotes > 1 {
            return malformed("branch_steps holds more than one local or one remote step");
        }
        Ok(())
    }

    /// Requires the record to describe the repository and worktree record
    /// `expected` names, compared by identity.
    pub fn check_binding(&self, expected: &RepositoryBinding) -> Result<(), RecordInvalid> {
        let ours = &self.repository;
        if ours.common_dir_identity != expected.common_dir_identity
            || ours.admin_dir_identity != expected.admin_dir_identity
            || ours.admin_name() != expected.admin_name()
        {
            return Err(RecordInvalid::IdentityMismatch);
        }
        Ok(())
    }

    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("a compact record always serializes")
    }
}

impl BranchStep {
    fn is_unbound(&self) -> bool {
        matches!(self.action, BranchAction::Unbound { .. })
    }

    fn validate(&self) -> Result<(), RecordInvalid> {
        let malformed = |what: &str| Err(RecordInvalid::Malformed(what.to_string()));
        match &self.action {
            BranchAction::Local { name, expected_tip } => {
                if name.is_empty() || !is_commit(expected_tip) {
                    return malformed("a local branch step needs a name and a commit id");
                }
            }
            BranchAction::Remote { remote, destination, endpoint, expected_head } => {
                if remote.is_empty() || destination.is_empty() || endpoint.is_empty() || !is_commit(expected_head) {
                    return malformed("a remote branch step needs a remote, destination, endpoint, and commit id");
                }
            }
            BranchAction::Unbound { target } => {
                let named = match target {
                    UnboundTarget::Local { name } => !name.is_empty(),
                    UnboundTarget::Remote { remote, name } => !remote.is_empty() && !name.is_empty(),
                };
                if !named {
                    return malformed("an unbound branch step has an empty name");
                }
                if self.state != StepState::Kept {
                    return malformed("an unbound branch step is never run");
                }
            }
        }
        Ok(())
    }
}

fn absolute(path: &NativeString, field: &str) -> Result<PathBuf, RecordInvalid> {
    let path = path.as_path();
    if path.as_os_str().as_encoded_bytes().contains(&0) || !path.is_absolute() {
        return Err(RecordInvalid::Malformed(format!("{field} is not an absolute native path")));
    }
    Ok(Path::to_path_buf(path))
}

fn is_commit(text: &str) -> bool {
    [40, 64].contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests;
