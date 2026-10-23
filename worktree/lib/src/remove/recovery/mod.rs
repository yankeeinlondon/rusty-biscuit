//! Durable state of a contention-aware removal, kept so that a stopped or
//! crashed operation can be found, explained, and resumed.
//!
//! Each operation owns `<common-git-dir>/wt-removal/<operation-id>/`.
//!
//! A confirmed removal saves only a compact `record.json`
//! ([`compact::CompactRecord`], `record_version` 2) before its first unlock,
//! move, or deletion; the record is the approval a later run finishes from,
//! and a finished operation's directory is deleted
//! ([`store::PendingRemoval`]). An operation an earlier `wt` saved (the
//! layout below, `record_version` 1) is migrated to a compact record when it
//! is resumed ([`legacy`]), and its journals and report are then deleted
//! unread.
//!
//! The legacy layout, still written by the journal-based lifecycle:
//!
//! | File | Written | Kept after completion |
//! | --- | --- | --- |
//! | `record.json` ([`record`]) | before any mutation; rewritten as phases finish | yes, marked completed |
//! | `journal.jsonl` ([`journal`]) | the deletion inventory before any mutation; intent before each deletion batch, result after | only when the final report could not list its entries |
//! | `journal-<n>.jsonl` | when a retry resumes the operation with a fresh inventory, a copy of the journal so far is kept under the next free `<n>` | as `journal.jsonl` |
//! | `report.json` ([`super::outcome::OperationReport`]) | when the operation stops or completes; a retry carries the earlier attempt's report into its own | yes |
//!
//! Every attempt's journal counts toward the report: the archives are read
//! with the current journal (see [`store::Operation::journal_entries`]), and
//! the completed report lists every entry before the journals are deleted.
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
//! - `record.json` and `report.json` are replaced by write-temp, `fsync`,
//!   rename, then `fsync` of the directory, so a reader sees the old or the
//!   new record, never a torn one.
//! - A retry replaces `journal.jsonl` without ever leaving the operation
//!   without a readable journal: it first copies the old journal durably to
//!   `journal-<n>.jsonl` (reusing the newest archive when a stopped retry
//!   already wrote the same bytes), builds and flushes the new journal under a
//!   hidden name, then renames it over `journal.jsonl`. A crash or failed write
//!   before that rename leaves the previous attempt's journal in place, so
//!   recovery reads it as before and nothing new is authorized; at worst a
//!   hidden temporary file is left beside it, which nothing reads.
//! - A discarded operation's directory is renamed to a hidden name before it
//!   is deleted, so a crash part-way never leaves a record without its journal.
//! - The journal is append-only. A batch's intent is flushed before any of
//!   its entries is unlinked, and its result is flushed after. A crash between
//!   the two leaves an intent without a result; recovery reports an absent
//!   entry of that batch as `uncertain`, never as deleted by this operation.
//!   A torn final line is a write the crash interrupted and is ignored.
//! - On Windows a directory cannot be opened to `fsync` it, so the rename of
//!   a replaced file is durable only once the filesystem flushes its
//!   metadata. Std's rename replaces an existing file there as on Unix, which
//!   every replacement above relies on; each replaced file is closed first.

pub mod compact;
pub mod journal;
pub mod legacy;
pub mod lock;
pub mod record;
pub mod space;
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
