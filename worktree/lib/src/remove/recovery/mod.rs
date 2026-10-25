//! Durable state of a confirmed removal, kept so that an interrupted
//! operation is finished by the next `wt remove` without asking again.
//!
//! Each operation owns `<common-git-dir>/wt-removal/<operation-id>/`, which
//! holds one compact `record.json` ([`compact::CompactRecord`],
//! `record_version` 3). It is saved before the first unlock, move,
//! deletion, or unregistration of any kind of target, records each step as
//! it finishes, and is the approval a later
//! run finishes from; a finished operation's directory is deleted
//! ([`store::PendingRemoval`]).
//!
//! An operation an earlier `wt` saved (`record_version` 1, [`record`], with
//! an optional `report.json` and deletion journals beside it) is migrated to
//! a compact record when it is resumed ([`legacy`]); its journals and report
//! are then deleted unread.
//!
//! The store lives in the common Git directory, outside both checkout paths
//! and outside the target's own worktree record, which removal deletes.
//! [`lock`] keys an OS-held lock on that record; [`store`] ties them together.
//!
//! ## Crash guarantees
//!
//! Against process termination (not power loss, which depends on the
//! filesystem honoring `fsync`):
//!
//! - An operation directory appears all at once: it is built under a hidden
//!   temporary name, flushed, and renamed into place. A crash before the
//!   rename leaves only a hidden directory that names no operation, and
//!   nothing has been mutated yet.
//! - `record.json` is replaced by write-temp, `fsync`, rename, then `fsync`
//!   of the directory, so a reader sees the old or the new record, never a
//!   torn one. A legacy record is migrated the same way, before its leftovers
//!   are deleted.
//! - A finished operation's directory is renamed to a hidden name before it
//!   is deleted, so a crash part-way never leaves a partial record.
//! - On Windows a directory cannot be opened to `fsync` it, so the rename of
//!   a replaced file is durable only once the filesystem flushes its
//!   metadata. Std's rename replaces an existing file there as on Unix, which
//!   every replacement above relies on; each replaced file is closed first.

pub mod compact;
pub mod legacy;
pub mod lock;
pub mod record;
pub mod store;

pub(crate) mod storage;

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::WorktreeError;

pub use storage::StorageProblem;

/// A 128-bit random operation identifier, as 32 lowercase hex characters.
/// It names the operation's directory, so a malformed one never parses.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct OperationId(String);

impl OperationId {
    /// A new identifier from the OS random source.
    pub fn generate() -> Result<Self, WorktreeError> {
        super::handoff::new_token().map(Self)
    }

    pub fn parse(text: &str) -> Option<Self> {
        super::handoff::is_token(text).then(|| Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for OperationId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| serde::de::Error::custom("an operation id is 32 lowercase hex characters"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_parse_and_differ() {
        let first = OperationId::generate().unwrap();
        let second = OperationId::generate().unwrap();
        assert_eq!(OperationId::parse(first.as_str()), Some(first.clone()));
        assert_ne!(first, second);
    }

    #[test]
    fn an_id_that_could_name_another_path_never_parses() {
        for text in ["", "../../etc", "0123456789ABCDEF0123456789ABCDEF", "0123456789abcdef0123456789abcde", "0123456789abcdef0123456789abcdef0"] {
            assert_eq!(OperationId::parse(text), None, "{text:?}");
            assert!(serde_json::from_value::<OperationId>(serde_json::json!(text)).is_err(), "{text:?}");
        }
    }
}
