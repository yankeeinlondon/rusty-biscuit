//! Validation failure types — the shared boundary between error reporting and
//! the harness data model.
//!
//! This module is intentionally a leaf: it depends on neither [`crate::harness::error`]
//! nor [`crate::harness::model`] so both of those modules can import from here
//! without creating an import cycle.

use serde::{Deserialize, Serialize};

/// Stable identifier for a validation rule, preserving author declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValidationRuleId(pub u32);

/// Which lifecycle event this validation maps to for handler lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationEvent {
    FileExists,
    DirExists,
    JsonFileExists,
    YamlFileExists,
    TomlFileExists,
    HasWritePermission,
    ShellCommand,
    NoDirtySourceCode,
    HasDirtySourceCode,
    FileChanged,
    FileUnchanged,
    FrontmatterPropChanged,
    FrontmatterPropUnchanged,
    FrontmatterPropEquals,
    ResponseLengthAtLeast,
    ResponseLengthAtMost,
    ResponseIncludes,
    ResponseMissing,
    InlineResponseEmpty,
    InlineBodyUnchanged,
}

impl std::fmt::Display for ValidationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::FileExists => "file_exists",
            Self::DirExists => "dir_exists",
            Self::JsonFileExists => "json_file_exists",
            Self::YamlFileExists => "yaml_file_exists",
            Self::TomlFileExists => "toml_file_exists",
            Self::HasWritePermission => "has_write_permission",
            Self::ShellCommand => "shell_command",
            Self::NoDirtySourceCode => "no_dirty_source_code",
            Self::HasDirtySourceCode => "has_dirty_source_code",
            Self::FileChanged => "file_changed",
            Self::FileUnchanged => "file_unchanged",
            Self::FrontmatterPropChanged => "frontmatter_prop_changed",
            Self::FrontmatterPropUnchanged => "frontmatter_prop_unchanged",
            Self::FrontmatterPropEquals => "frontmatter_prop_equals",
            Self::ResponseLengthAtLeast => "response_length_at_least",
            Self::ResponseLengthAtMost => "response_length_at_most",
            Self::ResponseIncludes => "response_includes",
            Self::ResponseMissing => "response_missing",
            Self::InlineResponseEmpty => "inline_response_empty",
            Self::InlineBodyUnchanged => "inline_body_unchanged",
        };
        write!(f, "{s}")
    }
}

/// Which execution phase produced a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailurePhase {
    PreCheck,
    PostCheck,
    Agent,
    ShellAudit,
}

impl std::fmt::Display for FailurePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreCheck => write!(f, "pre_check"),
            Self::PostCheck => write!(f, "post_check"),
            Self::Agent => write!(f, "agent"),
            Self::ShellAudit => write!(f, "shell_audit"),
        }
    }
}

/// A single validation failure with context.
#[derive(Debug, Clone)]
pub struct ValidationFailure {
    /// The rule that failed.
    pub rule_id: ValidationRuleId,
    /// The event name.
    pub event: ValidationEvent,
    /// Which phase reported the failure.
    pub phase: FailurePhase,
    /// Optional subject key.
    pub subject_key: Option<String>,
    /// Human-readable failure message (already rendered).
    pub message: String,
}
