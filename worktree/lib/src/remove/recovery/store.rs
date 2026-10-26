//! The recovery store at `<common-git-dir>/wt-removal/`: opening it and
//! taking an operation's lock. Saving, finding, and resuming an operation is
//! [`compact`]'s; the module docs of [`super`] state the crash guarantees.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::lock::{self, LockError, OperationLock};
use super::record::{RecordInvalid, RepositoryBinding};
use super::storage::{self, StorageProblem};
use super::OperationId;

/// The store's directory name inside the common Git directory.
pub const STORE_DIR: &str = "wt-removal";
pub const RECORD_FILE: &str = "record.json";
/// A legacy operation's report, read only to migrate that operation
/// ([`super::legacy`]); nothing writes one any more.
pub const REPORT_FILE: &str = "report.json";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{} cannot be used: {problem}", path.display())]
    Storage { path: PathBuf, problem: StorageProblem },
    #[error(transparent)]
    Lock(#[from] LockError),
    /// The record does not describe the worktree the lock covers.
    #[error("the removal lock held does not cover this worktree")]
    WrongLock,
    /// An earlier removal of this worktree has not finished.
    #[error("removal {0} of this worktree has not finished")]
    Pending(OperationId),
    #[error("removal {0} already completed")]
    Completed(OperationId),
    #[error("removal record {id} cannot be used: {reason}")]
    Record { id: String, reason: RecordInvalid },
    #[error("a removal's progress can only move forward")]
    PhaseOrder,
    /// The approval could not be saved; nothing was unlocked, moved, or
    /// deleted.
    #[error("the removal could not be saved, so nothing was deleted: {0}")]
    NotSaved(#[source] io::Error),
    /// A wait for another removal's lock was cancelled.
    #[error("the wait for another wt remove of this worktree was cancelled")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] io::Error),
}

fn storage_error(path: &Path) -> impl FnOnce(StorageProblem) -> StoreError + '_ {
    move |problem| StoreError::Storage { path: path.to_path_buf(), problem }
}

#[derive(Debug, Clone)]
pub struct RecoveryStore {
    root: PathBuf,
}

impl RecoveryStore {
    /// Opens the store of the repository whose common Git directory is
    /// `common_git_dir`, creating it owner-only when absent.
    pub fn open(common_git_dir: &Path) -> Result<Self, StoreError> {
        let metadata = fs::metadata(common_git_dir).map_err(|error| storage_error(common_git_dir)(error.into()))?;
        if !metadata.is_dir() {
            return Err(storage_error(common_git_dir)(StorageProblem::NotDirectory));
        }
        let root = common_git_dir.join(STORE_DIR);
        storage::ensure_private_dir(&root).map_err(storage_error(&root))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory of operation `id`.
    pub fn operation_dir(&self, id: &OperationId) -> PathBuf {
        self.root.join(id.as_str())
    }

    /// Takes the lock of the worktree record `binding` names, without
    /// waiting; [`LockError::Busy`] when another removal holds it.
    pub fn lock(&self, binding: &RepositoryBinding) -> Result<OperationLock, StoreError> {
        let name = binding.admin_name().ok_or(StoreError::WrongLock)?;
        Ok(lock::acquire(&self.root, name)?)
    }
}

mod compact;

pub use compact::{PendingRemoval, Stored, StoredOperation};
#[cfg(test)]
pub(crate) use compact::fault as compact_fault;

#[cfg(test)]
mod tests;
