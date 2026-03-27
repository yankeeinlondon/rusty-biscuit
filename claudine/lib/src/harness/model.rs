//! Core data model for the harness subsystem.

use std::path::PathBuf;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Top-level harness plan parsed from composed frontmatter.
#[derive(Debug, Clone)]
pub struct HarnessPlan {
    /// Absolute path to the source document.
    pub source_path: PathBuf,
    /// Per-page timeout, if specified.
    pub timeout: Option<std::time::Duration>,
    /// Validations that must pass before launching the provider.
    pub pre_checks: Vec<ValidationRule>,
    /// Validations that must pass after the provider completes.
    pub post_checks: Vec<ValidationRule>,
    /// YAML-declared handler table.
    pub handlers: HandlerTable,
    /// Programmatic handler command (the `handle` frontmatter property).
    pub programmatic_handler: Option<ApprovedRuntimeCommand>,
}

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
        };
        write!(f, "{s}")
    }
}

/// Whether a validation can run pre, post, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationPhase {
    PreOnly,
    PostOnly,
    Both,
}

/// A single parsed validation rule.
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// Stable ID preserving author declaration order.
    pub id: ValidationRuleId,
    /// The event name used for handler lookup.
    pub event: ValidationEvent,
    /// Which phase(s) this validation is allowed in.
    pub phase: ValidationPhase,
    /// The specific kind and its parameters.
    pub kind: ValidationKind,
    /// Optional user-provided message template.
    pub message_template: Option<String>,
    /// Normalized subject key for subject-specific handler matching.
    pub subject_key: Option<String>,
}

/// All supported validation operations with their typed parameters.
#[derive(Debug, Clone)]
pub enum ValidationKind {
    FileExists { file: PathBuf },
    DirExists { dir: PathBuf },
    JsonFileExists { file: PathBuf, shape: Option<StructuredShape> },
    YamlFileExists { file: PathBuf, shape: Option<StructuredShape> },
    TomlFileExists { file: PathBuf },
    HasWritePermission { file: PathBuf },
    ShellCommand { command: ApprovedRuntimeCommand, show_stdout: bool, show_stderr: bool },
    NoDirtySourceCode { root: PathBuf },
    HasDirtySourceCode { root: PathBuf },

    // Post-only: file comparison
    FileChanged { file: PathBuf },
    FileUnchanged { file: PathBuf },

    // Post-only: frontmatter comparison
    FrontmatterPropChanged { prop: String },
    FrontmatterPropUnchanged { prop: String },
    FrontmatterPropEquals { expected: IndexMap<String, serde_json::Value> },

    // Post-only: response checks
    ResponseLengthAtLeast { length: usize },
    ResponseLengthAtMost { length: usize },
    ResponseIncludes { needle: String },
    ResponseMissing { needle: String },
}

/// Broad shape constraint for structured file validations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StructuredShape {
    Scalar,
    Array,
    Object,
}

impl std::str::FromStr for StructuredShape {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scalar" => Ok(Self::Scalar),
            "array" => Ok(Self::Array),
            "object" => Ok(Self::Object),
            _ => Err(format!("invalid shape \"{s}\"; expected \"scalar\", \"array\", or \"object\"")),
        }
    }
}

/// A runtime command that has been tokenized and approved.
#[derive(Debug, Clone)]
pub struct ApprovedRuntimeCommand {
    /// The original raw command string.
    pub raw: String,
    /// The resolved executable name/path.
    pub executable: String,
    /// The parsed arguments.
    pub args: Vec<String>,
}

/// YAML-declared handler table split by specificity.
#[derive(Debug, Clone, Default)]
pub struct HandlerTable {
    /// Subject-specific handlers (matched by event + subject_key).
    pub exact: Vec<HandlerRule>,
    /// Generic handlers (matched by event only).
    pub generic: Vec<HandlerRule>,
}

/// A single handler rule binding an event (optionally with a subject) to an action.
#[derive(Debug, Clone)]
pub struct HandlerRule {
    /// The failure event this handler matches.
    pub event: FailureEvent,
    /// Optional subject key for subject-specific matching.
    pub subject_key: Option<String>,
    /// The action to take when matched.
    pub action: HandlerAction,
}

/// Recovery actions available in handler declarations.
#[derive(Debug, Clone)]
pub enum HandlerAction {
    Retry {
        prompt_suffix: Option<String>,
        set: Option<IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
        retries: Option<u32>,
    },
    Resume {
        prompt: String,
        set: Option<IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
        retries: Option<u32>,
    },
    Redirect {
        file: String,
        set: Option<IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
        resume: bool,
    },
    Deviate {
        command: ApprovedRuntimeCommand,
        set: Option<IndexMap<String, serde_json::Value>>,
        msg: Option<String>,
        say: Option<String>,
    },
}

/// Normalized failure event for handler lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FailureEvent {
    AgentFailure,
    Timeout,
    Validation(ValidationEvent),
}

impl std::fmt::Display for FailureEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentFailure => write!(f, "agent_failure"),
            Self::Timeout => write!(f, "timeout"),
            Self::Validation(v) => write!(f, "{v}"),
        }
    }
}

/// Pre-run snapshot for post-check comparison.
#[derive(Debug, Clone, Default)]
pub struct PreRunSnapshot {
    /// Source document Markdown (for frontmatter comparison).
    pub source_markdown: Option<darkmatter::markdown::Markdown>,
    /// File fingerprints keyed by absolute path.
    pub tracked_files: IndexMap<PathBuf, FileFingerprint>,
    /// Frontmatter property values captured before the run.
    pub tracked_frontmatter: IndexMap<String, serde_json::Value>,
}

/// Content fingerprint for a single file at a point in time.
#[derive(Debug, Clone)]
pub struct FileFingerprint {
    pub exists: bool,
    pub is_dir: bool,
    /// BLAKE3 hex hash of file contents, if readable.
    pub blake3: Option<String>,
}

/// Result of a single provider attempt.
#[derive(Debug, Clone)]
pub struct AttemptOutcome {
    /// Which attempt number (1-indexed).
    pub attempt: u32,
    /// Session ID captured from provider stream, if available.
    pub session_id: Option<String>,
    /// The provider's final assistant response text.
    pub final_response: String,
    /// Process exit code.
    pub exit_code: i32,
    /// How the process terminated.
    pub termination: ProcessTermination,
    /// Captured stderr text, if available.
    pub stderr_text: Option<String>,
}

/// How a child process terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessTermination {
    /// Normal completion (exit code may still be non-zero).
    Completed,
    /// Killed due to timeout.
    TimedOut,
    /// Killed by user interrupt (SIGINT/SIGTERM).
    Interrupted,
    /// Process could not be spawned.
    LaunchFailed,
}

impl std::fmt::Display for ProcessTermination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::TimedOut => write!(f, "timeout"),
            Self::Interrupted => write!(f, "interrupted"),
            Self::LaunchFailed => write!(f, "launch_failed"),
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
    /// Optional subject key.
    pub subject_key: Option<String>,
    /// Human-readable failure message (already rendered).
    pub message: String,
}

/// Describes a provider's resume capability for the harness.
#[derive(Debug, Clone)]
pub struct ResumeLaunchSpec {
    /// Whether this provider supports session resume.
    pub supported: bool,
    /// Human-readable description of resume behavior.
    pub description: Vec<&'static str>,
}

// Note: HarnessResolutionContext lives in resolve.rs
