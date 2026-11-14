//! Darkmatter's configuration files: their shipped schema, the loader that
//! captures them for a request, the locked, comment-preserving writer every
//! program that edits them shares, and the recovery receipts a migration
//! leaves between publishing a file and cleaning up its sources.
//!
//! User-facing guides: `darkmatter/docs/topics/configuration.md` and
//! `darkmatter/docs/topics/config-writes.md`.

pub mod load;
pub mod migration_receipt;
pub mod schema;
pub mod write;
