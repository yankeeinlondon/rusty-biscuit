//! Error types for the harness module.

use std::path::PathBuf;

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use serde_json::{Value, json};

use crate::diagnostics::{Category, Diagnostic, Disposition, Origin, code_spec};

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

impl BlockError for HarnessError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("HarnessError", self.code()))
            .body(self.to_string())
    }
}

impl Diagnostic for HarnessError {
    fn code(&self) -> &'static str {
        match self {
            // Author-authored harness frontmatter (timeouts, typed properties).
            HarnessError::InvalidFrontmatter { .. } | HarnessError::InvalidTimeout { .. } => {
                "composition.lifecycle_invalid"
            }
            // Shell directives the author wrote in the prompt document.
            HarnessError::ShellCommandExecutionFailed { .. }
            | HarnessError::ShellCommandDenied { .. }
            | HarnessError::ShellCommandBlacklisted { .. }
            | HarnessError::ShellAuditParseError { .. } => "composition.shell_expansion",
            // Path resolution beneath the run.
            HarnessError::RepoRootRequired { .. } | HarnessError::PathResolutionFailed { .. } => {
                "io.read_failed"
            }
        }
    }

    fn category(&self) -> Category {
        code_spec(self.code())
            .map(|spec| spec.category)
            .unwrap_or(Category::Composition)
    }

    fn disposition(&self) -> Disposition {
        code_spec(self.code())
            .map(|spec| spec.disposition)
            .unwrap_or(Disposition::Correctable)
    }

    fn origin(&self) -> Origin {
        code_spec(self.code())
            .map(|spec| spec.origin)
            .unwrap_or(Origin::Author)
    }

    fn detail(&self) -> Value {
        match self {
            HarnessError::InvalidFrontmatter {
                property, detail, ..
            } => json!({ "property": property, "message": detail }),
            HarnessError::InvalidTimeout { detail, .. } => {
                json!({ "property": "timeout", "message": detail })
            }
            HarnessError::ShellCommandExecutionFailed { detail } => {
                json!({ "command": detail })
            }
            HarnessError::ShellCommandDenied { command }
            | HarnessError::ShellCommandBlacklisted { command, .. } => {
                json!({ "command": command })
            }
            HarnessError::ShellAuditParseError { detail } => json!({ "command": detail }),
            HarnessError::RepoRootRequired { path } => json!({ "path": path }),
            HarnessError::PathResolutionFailed { raw, .. } => json!({ "path": raw }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_denied_classifies_as_shell_expansion() {
        let err = HarnessError::ShellCommandDenied {
            command: "rm -rf /".to_string(),
        };
        assert_eq!(err.code(), "composition.shell_expansion");
        assert_eq!(err.category(), Category::Composition);
        assert_eq!(err.origin(), Origin::Author);
        assert_eq!(err.detail()["command"], json!("rm -rf /"));
    }

    #[test]
    fn invalid_frontmatter_classifies_as_lifecycle_invalid() {
        let err = HarnessError::InvalidFrontmatter {
            source_path: PathBuf::from("p.md"),
            property: "timeout".to_string(),
            detail: "must be a string".to_string(),
        };
        assert_eq!(err.code(), "composition.lifecycle_invalid");
        assert_eq!(err.detail()["property"], json!("timeout"));
        assert_eq!(err.detail()["message"], json!("must be a string"));
    }

    #[test]
    fn path_resolution_classifies_as_io_read() {
        let err = HarnessError::RepoRootRequired {
            path: "@/x".to_string(),
        };
        assert_eq!(err.code(), "io.read_failed");
        assert_eq!(err.category(), Category::Io);
        assert_eq!(err.detail()["path"], json!("@/x"));
    }
}
