//! The lock that keeps two removals of one worktree from running at once.
//!
//! It is an OS-held exclusive lock (`fs4`) on
//! `<common-git-dir>/wt-removal/locks/<key>.lock`, where the key hashes the
//! name of the target's worktree record. The lock file is never deleted and
//! its existence means nothing: only a held lock is an active operation, and
//! the OS releases a crashed holder's lock.

use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use super::storage::{StorageProblem, ensure_private_dir};
use crate::cache::try_lock_sidecar;

/// Locks of a store are kept in this subdirectory.
pub const LOCKS_DIR: &str = "locks";

/// A held operation lock; dropping it releases the lock.
#[derive(Debug)]
pub struct OperationLock {
    _file: File,
    key: String,
}

impl OperationLock {
    /// The hashed worktree record name this lock covers.
    pub fn key(&self) -> &str {
        &self.key
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// Another process is removing this worktree.
    #[error("another wt remove is already removing this worktree")]
    Busy,
    #[error("the removal lock directory cannot be used: {0}")]
    Storage(StorageProblem),
    #[error("could not take the removal lock: {0}")]
    Io(#[from] std::io::Error),
}

/// The lock key for the worktree record named `admin_name`.
pub fn key_for(admin_name: &OsStr) -> String {
    biscuit_hash::blake3_hash_bytes(admin_name.as_encoded_bytes())[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Takes the lock for `admin_name` in the store at `store_root` without
/// waiting.
pub(super) fn acquire(store_root: &Path, admin_name: &OsStr) -> Result<OperationLock, LockError> {
    let dir = store_root.join(LOCKS_DIR);
    ensure_private_dir(&dir).map_err(LockError::Storage)?;
    let key = key_for(admin_name);
    let path: PathBuf = dir.join(format!("{key}.lock"));
    match try_lock_sidecar(&path)? {
        Some(file) => Ok(OperationLock { _file: file, key }),
        None => Err(LockError::Busy),
    }
}
