//! Core data model for the harness subsystem.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

/// Normalized failure event for lifecycle recovery routing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FailureEvent {
    AgentFailure,
    /// Timed-out termination.
    ///
    /// Both wall-clock `timeout` and silence `step_timeout` produce this
    /// variant so lifecycle recovery can treat all timeouts uniformly.
    Timeout,
    ShellAuditDenied,
}

impl std::fmt::Display for FailureEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentFailure => write!(f, "agent_failure"),
            Self::Timeout => write!(f, "timeout"),
            Self::ShellAuditDenied => write!(f, "shell_audit_denied"),
        }
    }
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
    /// Honest error label carried from the stream summary so lifecycle
    /// recovery can read it. Populated for guard-driven terminations
    /// (`"exit_expression"`, `"runaway_repetition"`, `"runaway_volume"`),
    /// for the timeout labels (`"timeout"`, `"step_timeout"`), and for
    /// provider-semantic kinds the stream parsers set (`"rate_limit"`,
    /// `"billing_error"`, `"unauthorized"`, …); `None` for non-error
    /// outcomes and for terminations that did not synthesize a summary
    /// error kind.
    pub error_kind: Option<String>,
    /// Provider-surfaced or wrapper-synthesized error message from the
    /// stream summary (e.g. `"Too many requests"`, or the guard/timeout
    /// prose `apply_early_termination_to_summary` stamps at trip time).
    /// `None` on the capture and interactive paths, which have no stream
    /// parser.
    pub error_message: Option<String>,
    /// Configured duration of the timeout rule that fired, populated only
    /// when `termination` is [`ProcessTermination::TimedOut`] (which rule is
    /// disambiguated via `error_kind`: `"step_timeout"` = stream silence,
    /// otherwise wall-clock).
    pub timeout_secs: Option<u64>,
    /// Structured detail for a content-guard trip (exit-expression pattern,
    /// repetition cycle, or volume counters). At most the cluster relevant
    /// to the trip is populated; the remaining fields stay `None`. `None`
    /// for non-guard terminations.
    pub guard_context: Option<GuardContext>,
}

/// Structured detail for a content-guard trip, threaded from the stream
/// summary into [`AttemptOutcome`] so lifecycle recovery can branch on the
/// guard kind without re-parsing the message string.
///
/// Every field is optional; only the cluster relevant to the trip is
/// populated:
/// - exit-expression → `pattern` (+ optional `scope`);
/// - runaway-repetition → `cycle_len` + `repeats`;
/// - runaway-volume → `lines` + `bytes`;
/// - stalled-generation → `generation_count` + `stall_duration_ms`, plus the
///   optional OpenCode identity fields (`session_id`, `step`, `agent`,
///   `provider_id`, `model_id`, `mode`) when OpenCode tagged them.
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
    /// Generation attempts since the last progress-class event at the moment
    /// the stalled-generation backstop tripped.
    pub generation_count: Option<u32>,
    /// Elapsed progress silence in milliseconds at the stalled-generation trip.
    pub stall_duration_ms: Option<u64>,
    /// Safe OpenCode identity metadata for the stalled generation, surfaced so
    /// lifecycle consumers can branch on the run without parsing the prose
    /// message. Each is present only when OpenCode tagged it on the stalled
    /// `service=llm` record; none ever carries prompt text, tool payloads,
    /// HTTP URLs, authorization headers, or raw stderr lines.
    ///
    /// OpenCode session id of the stalled generation.
    pub session_id: Option<String>,
    /// Reasoning step the stall was observed on.
    pub step: Option<u32>,
    /// Agent name driving the generation (e.g. `rust-developer`).
    pub agent: Option<String>,
    /// Provider id of the model attempting the generation.
    pub provider_id: Option<String>,
    /// Model id attempting the generation.
    pub model_id: Option<String>,
    /// OpenCode generation mode (e.g. `primary`, `all`).
    pub mode: Option<String>,
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
    /// [`Self::TimedOut`] (which would re-run the provider and reproduce the
    /// runaway) and from [`Self::Interrupted`] (a user cancel that suppresses
    /// recovery). [`crate::harness::classify_failure`] maps this to
    /// [`FailureEvent::AgentFailure`] so a runaway triggers normal fail-fast
    /// lifecycle recovery, never a retry.
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

/// Describes a provider's resume capability for the harness.
#[derive(Debug, Clone)]
pub struct ResumeLaunchSpec {
    /// Whether this provider supports session resume.
    pub supported: bool,
    /// Human-readable description of resume behavior.
    pub description: Vec<&'static str>,
}

// ---------------------------------------------------------------------------
// Shell audit types (used by audit.rs)
// ---------------------------------------------------------------------------

/// Where an audited command originates.
#[derive(Debug, Clone)]
pub enum AuditedCommandSource {
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
