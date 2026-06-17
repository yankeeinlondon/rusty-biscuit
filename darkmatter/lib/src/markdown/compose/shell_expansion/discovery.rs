//! Shell command discovery (compatibility re-export).
//!
//! The collection walk moved to [`crate::markdown::compose::preflight::collect`]
//! when it became condition-blind (compose-pipeline v2). This module re-exports
//! the public entry point so existing `shell_expansion::discovery` import paths
//! keep resolving; new code should depend on `compose::preflight` directly.

pub use crate::markdown::compose::preflight::collect::collect_shell_commands;
