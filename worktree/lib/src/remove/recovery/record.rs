//! The legacy recovery record (`record_version` 1): what one removal
//! operation set out to do, bound to the exact checkout, repository, and
//! approvals it was decided on.
//!
//! Its checkout-removal intent is continued approval, but only through
//! migration to a [`super::compact::CompactRecord`] ([`super::legacy`]), and
//! only while its repository and worktree-record bindings still match. Its
//! branch actions are rebuilt only where their bindings are recorded, never a
//! remote one. `completed_phase` is a hint about where to look, never proof
//! of what the paths or Git hold now.
//!
//! [`parse`] is strict: every field is required, `null` is never read as
//! absent, unknown or repeated keys and trailing content are refused, and an
//! unknown `record_version` is refused before the rest is read.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sniff::filesystem::query::{FileIdentity, NativeString, PathUsageReport};

use super::OperationId;
use crate::strict_json;

/// The only `record_version` this build reads or writes.
pub const RECORD_VERSION: u32 = 1;

/// The quarantine's file-name prefix; the record refuses any other.
pub const QUARANTINE_PREFIX: &str = ".wt-remove-";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryRecord {
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
    /// A hidden sibling of `original`, named with [`QUARANTINE_PREFIX`].
    pub quarantine: NativeString,
    /// The checkout directory's identity, which the move preserves.
    pub checkout_identity: FileIdentity,
    pub repository: RepositoryBinding,
    pub branch: BranchBinding,
    /// The commit the checkout's `HEAD` was approved at.
    pub approved_tip: String,
    /// [`crate::remove::Inventory::fingerprint`] the consent was given for.
    pub consent_fingerprint: String,
    /// The decisive Sniff query that found no usage.
    pub discovery: PathUsageReport,
    /// Always includes [`IntendedAction::RemoveCheckout`].
    pub intended_actions: Vec<IntendedAction>,
    pub completed_phase: Phase,
}

/// The repository and the target's worktree record (its admin directory),
/// by path and identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryBinding {
    pub common_dir: NativeString,
    pub common_dir_identity: FileIdentity,
    /// `<common_dir>/worktrees/<name>`.
    pub admin_dir: NativeString,
    pub admin_dir_identity: FileIdentity,
}

impl RepositoryBinding {
    /// The admin directory's name, which keys the operation lock.
    pub fn admin_name(&self) -> Option<&std::ffi::OsStr> {
        self.admin_dir.as_path().file_name()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchBinding {
    Named(String),
    Detached,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum IntendedAction {
    RemoveCheckout,
    DeleteLocalBranch { name: String },
    DeleteRemoteBranch { remote: String, name: String },
}

/// The last phase the operation recorded as finished, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// The record is durable; nothing has been mutated.
    Prepared,
    Moved,
    /// Git's record points at the quarantine.
    Associated,
    Deleting,
    ContentRemoved,
    Unregistered,
    BranchActions,
    /// Every requested step finished. A completed record is never pending.
    Completed,
}

/// Why a record cannot be used. The operation refuses to resume and keeps
/// whatever the record describes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordInvalid {
    #[error("the recovery record is malformed: {0}")]
    Malformed(String),
    #[error("the recovery record has version {0}, which this wt does not read")]
    UnsupportedVersion(u64),
    #[error("the recovery record belongs to a different repository or worktree record")]
    IdentityMismatch,
    #[error("the recovery record's operation id does not match its directory")]
    OperationMismatch,
}

/// Parses and validates a record.
pub fn parse(bytes: &[u8]) -> Result<RecoveryRecord, RecordInvalid> {
    let members = strict_json::members(bytes)
        .ok_or_else(|| RecordInvalid::Malformed("not one JSON object with distinct keys".into()))?;
    let version: u64 = members
        .get("record_version")
        .ok_or_else(|| RecordInvalid::Malformed("record_version is missing or not a number".into()))?;
    if version != u64::from(RECORD_VERSION) {
        return Err(RecordInvalid::UnsupportedVersion(version));
    }
    let record: RecoveryRecord =
        serde_json::from_slice(bytes).map_err(|error| RecordInvalid::Malformed(error.to_string()))?;
    record.validate()?;
    Ok(record)
}

impl RecoveryRecord {
    /// Checks what serde cannot: path shapes, the quarantine's relation to
    /// the original, and the formats of the bound values.
    pub fn validate(&self) -> Result<(), RecordInvalid> {
        let malformed = |what: &str| Err(RecordInvalid::Malformed(what.to_string()));
        if self.record_version != RECORD_VERSION {
            return Err(RecordInvalid::UnsupportedVersion(self.record_version.into()));
        }
        if self.requested_name.is_empty() || self.aliases.iter().any(String::is_empty) {
            return malformed("a requested name or alias is empty");
        }
        let original = absolute(&self.original, "original")?;
        let quarantine = absolute(&self.quarantine, "quarantine")?;
        let quarantine_name = quarantine.file_name().map(|name| name.as_encoded_bytes());
        if original.parent().is_none()
            || quarantine.parent() != original.parent()
            || !quarantine_name.is_some_and(|name| name.starts_with(QUARANTINE_PREFIX.as_bytes()))
        {
            return malformed("quarantine is not a hidden sibling of the original");
        }
        let common = absolute(&self.repository.common_dir, "common_dir")?;
        let admin = absolute(&self.repository.admin_dir, "admin_dir")?;
        if admin.parent() != Some(common.join("worktrees").as_path()) {
            return malformed("admin_dir is not a worktree record of common_dir");
        }
        if let BranchBinding::Named(name) = &self.branch
            && name.is_empty()
        {
            return malformed("branch name is empty");
        }
        if !is_hex(&self.approved_tip, &[40, 64]) {
            return malformed("approved_tip is not a commit id");
        }
        if !is_hex(&self.consent_fingerprint, &[64]) {
            return malformed("consent_fingerprint is not a 64-character digest");
        }
        let removals = self.intended_actions.iter().filter(|action| **action == IntendedAction::RemoveCheckout).count();
        if removals != 1 {
            return malformed("intended_actions must include remove_checkout exactly once");
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
        serde_json::to_vec_pretty(self).expect("a recovery record always serializes")
    }
}

fn absolute(path: &NativeString, field: &str) -> Result<PathBuf, RecordInvalid> {
    let path = path.as_path();
    if path.as_os_str().as_encoded_bytes().contains(&0) || !path.is_absolute() {
        return Err(RecordInvalid::Malformed(format!("{field} is not an absolute native path")));
    }
    Ok(Path::to_path_buf(path))
}

fn is_hex(text: &str, lengths: &[usize]) -> bool {
    lengths.contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
