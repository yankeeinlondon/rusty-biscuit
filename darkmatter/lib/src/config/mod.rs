//! Darkmatter's configuration files: the locked, comment-preserving writer
//! every program that edits them shares, and the recovery receipts a
//! migration leaves between publishing a file and cleaning up its sources.
//!
//! User-facing guide: `darkmatter/docs/topics/config-writes.md`.

pub mod migration_receipt;
pub mod write;
