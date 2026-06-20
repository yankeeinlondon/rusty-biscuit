//! Core data model for the harness subsystem.

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::harness::error::HarnessError;
pub use crate::harness::failure::{
    FailurePhase, ValidationEvent, ValidationFailure, ValidationRuleId,
};

/// Top-level harness plan parsed from composed frontmatter.
#[derive(Debug, Clone)]
pub struct HarnessPlan {
    /// Absolute path to the source document.
    pub source_path: PathBuf,
    /// Per-page wall-clock timeout, if specified.
    pub timeout: Option<std::time::Duration>,
    /// Per-page step-silence timeout, if specified.
    ///
    /// Resets on every stream event; when silence exceeds this budget the
    /// child is killed. Streaming-only: ignored in capture and passthrough
    /// modes. Parse-time validation requires `step_timeout <= timeout` when
    /// both are present.
    pub step_timeout: Option<std::time::Duration>,
    /// Wall-clock warning threshold. When the prompt has been running for
    /// this long, claudine emits a single `Status::Warning` line instead
    /// of killing the child. Parse-time validation requires
    /// `timeout_warn < timeout` when both are present.
    pub timeout_warn: Option<std::time::Duration>,
    /// Step-silence warning threshold. When the provider has been silent
    /// for this long, claudine emits a single `Status::Warning` line per
    /// stall episode instead of killing the child. Parse-time validation
    /// requires `step_timeout_warn < step_timeout` when both are present.
    pub step_timeout_warn: Option<std::time::Duration>,
    /// Validations that must pass before launching the provider.
    pub pre_checks: Vec<ValidationRule>,
    /// Validations that must pass after the provider completes.
    pub post_checks: Vec<ValidationRule>,
    /// YAML-declared handler table.
    pub handlers: HandlerTable,
    /// Programmatic handler command (the `handle` frontmatter property).
    pub programmatic_handler: Option<ApprovedRuntimeCommand>,
}

/// Whether a validation can run pre, post, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationPhase {
    PreOnly,
    PostOnly,
    Both,
}

/// Origin metadata for a validation rule, used by the failure reporter to
/// surface the file, line range (best-effort), and YAML snippet that produced
/// the rule.
///
/// `None` indicates a system-owned rule with no markdown origin (e.g. the
/// inline writability pre-check) or a programmatically constructed rule from
/// tests.
#[derive(Debug, Clone)]
pub struct RuleSource {
    /// Absolute path to the source markdown file the rule was authored in.
    pub file: PathBuf,
    /// Best-effort 1-indexed inclusive line range within the source file's
    /// frontmatter where the rule appears. `None` when range recovery was
    /// not attempted or failed.
    pub line_range: Option<std::ops::RangeInclusive<usize>>,
    /// YAML snippet representing the single rule, suitable for syntax-
    /// highlighted display.
    pub yaml_snippet: String,
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
    /// Origin metadata for failure reporting; `None` for system-owned rules.
    pub source: Option<RuleSource>,
}

/// All supported validation operations with their typed parameters.
#[derive(Debug, Clone)]
pub enum ValidationKind {
    FileExists {
        file: PathBuf,
    },
    DirExists {
        dir: PathBuf,
    },
    JsonFileExists {
        file: PathBuf,
        shape: Option<StructuredShape>,
    },
    YamlFileExists {
        file: PathBuf,
        shape: Option<StructuredShape>,
    },
    TomlFileExists {
        file: PathBuf,
    },
    HasWritePermission {
        file: PathBuf,
    },
    ShellCommand {
        command: ApprovedRuntimeCommand,
        show_stdout: bool,
        show_stderr: bool,
    },
    NoDirtySourceCode {
        root: PathBuf,
    },
    HasDirtySourceCode {
        root: PathBuf,
    },

    // Post-only: file comparison
    FileChanged {
        file: PathBuf,
    },
    FileUnchanged {
        file: PathBuf,
    },

    // Post-only: frontmatter comparison
    FrontmatterPropChanged {
        prop: String,
    },
    FrontmatterPropUnchanged {
        prop: String,
    },
    FrontmatterPropEquals {
        expected: IndexMap<String, serde_json::Value>,
    },

    // Post-only: response checks
    ResponseLengthAtLeast {
        length: usize,
    },
    ResponseLengthAtMost {
        length: usize,
    },
    ResponseIncludes {
        needle: String,
    },
    ResponseMissing {
        needle: String,
    },
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
            _ => Err(format!(
                "invalid shape \"{s}\"; expected \"scalar\", \"array\", or \"object\""
            )),
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
    /// Timed-out termination.
    ///
    /// Both wall-clock `timeout` and silence `step_timeout` produce this
    /// variant so handler authors write a single `handle_timeout` block.
    Timeout,
    Validation(ValidationEvent),
    ShellAuditDenied,
}

impl std::fmt::Display for FailureEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentFailure => write!(f, "agent_failure"),
            Self::Timeout => write!(f, "timeout"),
            Self::Validation(v) => write!(f, "{v}"),
            Self::ShellAuditDenied => write!(f, "shell_audit_denied"),
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
    /// Honest per-guard label carried from the stream summary so the failure
    /// handler payload can read it. Populated for guard-driven terminations
    /// (`"exit_expression"`, `"runaway_repetition"`, `"runaway_volume"`) and
    /// for the legacy timeout labels (`"timeout"`, `"step_timeout"`); `None`
    /// for non-error outcomes and for terminations that did not synthesize a
    /// summary error kind.
    pub error_kind: Option<String>,
    /// Structured detail for a content-guard trip (exit-expression pattern,
    /// repetition cycle, or volume counters). At most the cluster relevant
    /// to the trip is populated; the remaining fields stay `None`. `None`
    /// for non-guard terminations.
    pub guard_context: Option<GuardContext>,
}

/// Structured detail for a content-guard trip, threaded from the stream
/// summary into [`AttemptOutcome`] so the programmatic failure handler can
/// branch on the guard kind without re-parsing the message string.
///
/// Every field is optional; only the cluster relevant to the trip is
/// populated:
/// - exit-expression → `pattern` (+ optional `scope`);
/// - runaway-repetition → `cycle_len` + `repeats`;
/// - runaway-volume → `lines` + `bytes`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardContext {
    /// Matched exit-expression pattern (literal substring or regex source).
    pub pattern: Option<String>,
    /// Exit-expression scope the run was checked against
    /// (e.g. `"opencode/kimi-for-coding/k2p7"`).
    pub scope: Option<String>,
    /// Detected repetition cycle length `L`.
    pub cycle_len: Option<usize>,
    /// Consecutive matching cycles observed at trip time.
    pub repeats: Option<usize>,
    /// Per-turn (streaming) or per-run (capture) line counter at breach.
    pub lines: Option<u64>,
    /// Per-turn (streaming) or per-run (capture) byte counter at breach.
    pub bytes: Option<u64>,
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
    /// Killed by a claudine content guard — an exit-expression match, a
    /// runaway-repetition trip, or a volume-cap breach. Distinct from
    /// [`Self::TimedOut`] (which routes through the `handle_timeout:`
    /// retry path) and from [`Self::Interrupted`] (a user cancel that
    /// suppresses failure handling). [`crate::harness::classify_failure`]
    /// maps this to [`FailureEvent::AgentFailure`] so a runaway triggers
    /// the normal fail-fast handler, never a retry.
    Aborted,
}

impl std::fmt::Display for ProcessTermination {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::TimedOut => write!(f, "timeout"),
            Self::Interrupted => write!(f, "interrupted"),
            Self::LaunchFailed => write!(f, "launch_failed"),
            Self::Aborted => write!(f, "aborted"),
        }
    }
}

/// Validation-specific details for failure handling.
#[derive(Debug, Clone)]
pub struct FailureCheck {
    /// The normalized validation name.
    pub name: ValidationEvent,
    /// Optional subject key for subject-specific handling.
    pub subject_key: Option<String>,
}

/// Provider permission assessment for `has_write_permission`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionAssessment {
    Allowed,
    Denied { reason: String },
    Unknown { reason: String },
}

/// Provider-aware permission probe used by `has_write_permission`.
pub trait HarnessPermissionProbe: Send + Sync {
    /// Returns whether the current provider launch is expected to allow writes
    /// to `path` for the given source document.
    fn can_write(&self, path: &Path, source_path: &Path) -> PermissionAssessment;
}

/// Describes a provider's resume capability for the harness.
#[derive(Debug, Clone)]
pub struct ResumeLaunchSpec {
    /// Whether this provider supports session resume.
    pub supported: bool,
    /// Human-readable description of resume behavior.
    pub description: Vec<&'static str>,
}

// ---------------------------------------------------------------------------
// Structured validation outcomes (used by report.rs)
// ---------------------------------------------------------------------------

/// Structured outcome of evaluating a single validation rule.
#[derive(Debug, Clone)]
pub struct ValidationCheckOutcome {
    pub rule_id: ValidationRuleId,
    pub event: ValidationEvent,
    pub subject_key: Option<String>,
    pub passed: bool,
    /// Prose-ready markup body (not ANSI-rendered). Rendering belongs in `report.rs`.
    pub markup: String,
    /// Human-readable failure reason when `passed` is false.
    pub failure_message: Option<String>,
    /// Origin metadata cloned from the originating `ValidationRule`; `None`
    /// for outcomes derived from system-owned or programmatically constructed
    /// rules.
    pub source: Option<RuleSource>,
}

/// All outcomes for one validation phase (pre or post).
#[derive(Debug, Clone)]
pub struct ValidationPhaseReport {
    pub phase: FailurePhase,
    pub outcomes: Vec<ValidationCheckOutcome>,
}

impl ValidationPhaseReport {
    /// True when every outcome passed.
    pub fn all_passed(&self) -> bool {
        self.outcomes.iter().all(|o| o.passed)
    }

    /// Collect failed outcomes into `Vec<ValidationFailure>` for existing error propagation.
    pub fn failures(&self) -> Vec<ValidationFailure> {
        self.outcomes
            .iter()
            .filter(|o| !o.passed)
            .map(|o| ValidationFailure {
                rule_id: o.rule_id,
                event: o.event.clone(),
                phase: self.phase,
                subject_key: o.subject_key.clone(),
                message: o
                    .failure_message
                    .clone()
                    .unwrap_or_else(|| "check failed".to_string()),
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.outcomes.len()
    }

    /// Convert to the legacy error if any checks failed.
    pub fn into_result(self) -> Result<Self, HarnessError> {
        if self.all_passed() {
            Ok(self)
        } else {
            let failures = self.failures();
            match self.phase {
                FailurePhase::PreCheck | FailurePhase::ShellAudit => {
                    Err(HarnessError::PreCheckFailed { failures })
                }
                FailurePhase::PostCheck => Err(HarnessError::PostCheckFailed { failures }),
                FailurePhase::Agent => Err(HarnessError::PreCheckFailed { failures }),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shell audit types (used by audit.rs)
// ---------------------------------------------------------------------------

/// Where an audited command originates.
#[derive(Debug, Clone)]
pub enum AuditedCommandSource {
    PreCheck(ValidationRuleId),
    PostCheck(ValidationRuleId),
    ProgrammaticHandle,
    DeclarativeHandler {
        event: FailureEvent,
        subject_key: Option<String>,
    },
    ComposeSourceLine {
        line: usize,
    },
}

/// A command discovered during shell audit.
#[derive(Debug, Clone)]
pub struct AuditedCommand {
    pub source: AuditedCommandSource,
    pub raw: String,
    pub executable: String,
    pub args: Vec<String>,
}

/// Result of auditing a single command.
#[derive(Debug, Clone)]
pub struct ShellAuditOutcome {
    pub command: AuditedCommand,
    pub passed: bool,
    /// Prose-ready human-readable message.
    pub message: String,
}

/// Complete audit report.
#[derive(Debug, Clone)]
pub struct ShellAuditReport {
    pub outcomes: Vec<ShellAuditOutcome>,
}

impl ShellAuditReport {
    /// True when every audited command passed.
    pub fn all_passed(&self) -> bool {
        self.outcomes.iter().all(|o| o.passed)
    }

    /// Collect failed outcomes.
    pub fn failures(&self) -> Vec<&ShellAuditOutcome> {
        self.outcomes.iter().filter(|o| !o.passed).collect()
    }
}

// Note: HarnessResolutionContext lives in resolve.rs
