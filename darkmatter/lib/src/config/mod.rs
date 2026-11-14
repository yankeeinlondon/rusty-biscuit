//! Darkmatter's configuration files: their shipped schema, the loader that
//! captures them for a request, the locked, comment-preserving writer every
//! program that edits them shares, the recovery receipts a migration
//! leaves between publishing a file and cleaning up its sources, the
//! effective `network` permission remote reads take their consent from, and
//! the evaluated permission policy with its approval persistence.
//!
//! User-facing guides: `darkmatter/docs/topics/configuration.md` and
//! `darkmatter/docs/topics/config-writes.md`.

pub mod load;
pub mod migration;
pub mod migration_receipt;
pub mod network;
pub mod permissions;
pub mod schema;
pub mod write;
