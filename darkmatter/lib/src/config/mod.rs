//! Darkmatter's configuration files: their shipped schema, the loader that
//! captures them for a request, the locked, comment-preserving writer every
//! program that edits them shares, the recovery receipts a migration
//! leaves between publishing a file and cleaning up its sources, the
//! effective `network` permission remote reads take their consent from, and
//! the evaluated permission policy with its approval persistence, setup
//! (`md init` and a host's setup handshake), and host registration (the
//! manifests `register` entries name, loaded into passive host profiles).
//!
//! User-facing guides: `darkmatter/docs/topics/configuration.md`,
//! `darkmatter/docs/topics/config-writes.md`, and
//! `darkmatter/docs/topics/host-registration.md`.

pub mod init;
pub mod load;
pub mod migration;
pub mod migration_receipt;
pub mod network;
pub mod permissions;
pub mod registration;
pub mod schema;
pub mod write;
