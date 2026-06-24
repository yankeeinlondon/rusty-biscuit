//! Error types for the harness module.

use std::path::PathBuf;

/// All errors that can occur within the harness subsystem.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    // --- Parse / configuration ---
    /// Frontmatter property is structurally invalid or has an unexpected type.
    #[error("{source_path}: invalid `{property}` frontmatter property: {detail}")]
    InvalidFrontmatter {
        source_path: PathBuf,
        property: String,
        detail: String,
    },

    /// A timeout string could not be parsed.
    #[error("{source_path}: invalid timeout \"{raw}\": {detail}")]
    InvalidTimeout {
        source_path: PathBuf,
        raw: String,
        detail: String,
    },

    /// A shell command failed during execution.
    #[error("shell command execution failed: {detail}")]
    ShellCommandExecutionFailed { detail: String },

    // --- Shell approval failures ---
    /// A shell command was denied by the approval system.
    #[error("shell command denied: {command}")]
    ShellCommandDenied { command: String },

    /// A shell command matched the blacklist.
    #[error("shell command blacklisted: \"{command}\" — {reason}")]
    ShellCommandBlacklisted { command: String, reason: String },

    // --- Path resolution ---
    /// A `@`-prefixed path requires a repo root, but none was available.
    #[error("repo root required to resolve path \"{path}\"")]
    RepoRootRequired { path: String },

    /// Path resolution failed for another reason.
    #[error("path resolution failed for \"{raw}\": {detail}")]
    PathResolutionFailed { raw: String, detail: String },

    // --- Shell audit ---
    /// Failed to parse shell directives from source page during audit.
    #[error("shell audit parse error: {detail}")]
    ShellAuditParseError { detail: String },
}
