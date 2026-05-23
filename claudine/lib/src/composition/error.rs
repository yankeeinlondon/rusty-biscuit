//! Composition-specific error types.

use std::path::PathBuf;

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use chrono::{DateTime, Utc};
use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::compose::shell_expansion::ShellExpansionError;

use super::types::ResolutionMode;
use crate::provider::Provider;
use thiserror::Error;

/// Exit code emitted when a loop run halts because a provider rate-limit
/// signal was observed and the configured policy was `abort` (or pause was
/// unsafe due to a missing `reset_at`).
///
/// Mirrors `EX_TEMPFAIL` (`75`) from BSD `sysexits.h` so shell wrappers can
/// distinguish a transient rate-limit halt from a generic non-zero exit
/// (typically `1`).
pub const LOOP_RATE_LIMITED_EXIT_CODE: i32 = 75;

/// Errors that can occur during composition workflows.
#[derive(Error, Debug)]
pub enum CompositionError {
    /// The file reference string could not be parsed.
    #[error("invalid file reference: {0}")]
    InvalidReference(String),

    /// The resolved file does not exist.
    #[error("file not found: {0}")]
    FileNotFound(String),

    /// The file is not a Markdown document.
    #[error("not a Markdown file (expected .md or .markdown): {0}")]
    NotMarkdown(String),

    /// The Markdown file could not be loaded or parsed.
    #[error("failed to load Markdown: {0}")]
    MarkdownLoad(String),

    /// Inline composition requires a `prompt` frontmatter property.
    #[error("frontmatter is missing a `prompt` property")]
    PromptPropertyMissing,

    /// The `prompt` frontmatter property is not a string.
    #[error("frontmatter `prompt` must be a string, got {0}")]
    PromptPropertyWrongType(String),

    /// Darkmatter composition failed for a reason other than a known
    /// structured shell-expansion failure.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker can
    /// render a rich `BlockError` report (transclusion cycles, reference
    /// errors, etc.) instead of a flat string.
    #[error("compose failed: {0}")]
    ComposeFailed(#[source] MarkdownError),

    /// A shell expansion directive inside the composed document failed in a
    /// structurally-known way. Carries the underlying `ShellExpansionError`
    /// so the CLI layer can render a rich, source-aware error report.
    ///
    /// The `Display` impl of this variant intentionally matches the legacy
    /// `ComposeFailed` wrapper so callers that only use `to_string()` stay
    /// compatible, while callers that want structured data can `match` on
    /// the variant itself.
    #[error("compose failed: Shell expansion failed: {error}")]
    ShellExpansionFailed {
        /// The file being composed when the shell expansion error fired.
        source_path: PathBuf,
        /// The underlying structured shell expansion error.
        #[source]
        error: Box<ShellExpansionError>,
    },

    /// No installed providers can run composition.
    #[error("no runnable providers available (all excluded or uninstalled)")]
    NoRunnableProviders,

    /// The `agent` frontmatter hint does not match any known provider.
    #[error("agent hint `{0}` does not match any known provider")]
    AgentHintInvalid(String),

    /// The `agent` frontmatter property is not a valid type.
    #[error("frontmatter `agent` must be a string or array of strings, got {0}")]
    AgentHintWrongType(String),

    /// The `model` frontmatter property is not a valid type.
    #[error("frontmatter `model` must be a string or array of strings, got {0}")]
    ModelHintWrongType(String),

    /// The `agent` frontmatter hint matches multiple providers.
    #[error("agent hint `{hint}` is ambiguous; matches: {matches}")]
    AgentHintAmbiguous {
        hint: String,
        matches: String,
        /// The matched providers for interactive disambiguation.
        providers: Vec<Provider>,
    },

    /// Provider selection could not resolve in the current mode.
    #[error(
        "could not resolve provider in {mode} mode; installed: {installed:?}, \
         favorite_agent: {favorite_agent:?}, frontmatter_agent_present: {frontmatter_agent_present}"
    )]
    SelectionUnavailable {
        mode: ResolutionMode,
        installed: Vec<Provider>,
        favorite_agent: Option<Provider>,
        frontmatter_agent_present: bool,
    },

    /// A model is required but none was resolved.
    #[error("model selection failed for {provider}: {reason}")]
    ModelSelectionFailed { provider: Provider, reason: String },

    /// Interactive selection is required but no TTY is available.
    #[error(
        "interactive provider selection required but no TTY available; use an explicit provider flag (--claude, --codex, etc.) or add an `agent` frontmatter property"
    )]
    InteractiveSelectionRequired,

    /// Inline composition with `-i` is not supported for this provider
    /// because it cannot capture the final assistant message.
    #[error(
        "inline-compose with --interactive is not supported for {0}; the provider cannot capture the final assistant message"
    )]
    InlineInteractiveUnsupported(String),

    /// The provider returned an invalid response for inline composition.
    #[error("invalid inline composition response: {0}")]
    InvalidInlineResponse(String),

    /// Atomic file write failed during inline composition.
    #[error("atomic write failed: {0}")]
    AtomicWriteFailed(String),

    /// The composition target file lacks required read/write permissions.
    #[error("insufficient file permissions (need read+write): {0}")]
    InsufficientFilePermissions(String),

    /// Pre-flight shell command discovery failed.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker can
    /// render a rich `BlockError` report (e.g. transclusion cycles or
    /// reference errors encountered while walking the document graph) instead
    /// of a flat string.
    #[error("pre-flight discovery failed: {0}")]
    PreFlightDiscoveryFailed(#[source] MarkdownError),

    /// A general pre-flight failure (blacklisted command, missing handler, etc.).
    #[error("pre-flight shell approval failed: {0}")]
    PreFlightFailed(String),

    /// The user denied a shell command during pre-flight approval.
    #[error(
        "Aborted: shell command '{command}' was denied during pre-flight approval \
         (source: {source_file}, line {line}). No provider session was started."
    )]
    ShellCommandDenied {
        command: String,
        source_file: PathBuf,
        line: usize,
    },

    /// A lifecycle notification property failed to deserialize.
    #[error("invalid lifecycle property `{property}`: {message}")]
    LifecycleInvalid {
        /// The frontmatter property that failed to parse (e.g. `"success"`).
        property: String,
        /// The underlying deserialization error message.
        message: String,
        /// The source file whose frontmatter contained the invalid property.
        source_file: PathBuf,
        /// The unknown field name extracted from the serde error (e.g. `"speak"`).
        unknown_field: Option<String>,
        /// The list of valid field names for the lifecycle notification.
        expected_fields: Vec<String>,
    },

    /// A lifecycle notification property has both `say` and `say_first`.
    #[error("lifecycle property `{0}` has both `say` and `say_first`; only one is allowed")]
    LifecycleSayConflict(String),

    /// A lifecycle notification property references an unknown sound effect.
    #[error("lifecycle property `{0}` references unknown sound effect `{1}`")]
    LifecycleUnknownEffect(String, String),

    // -- Sequence errors -------------------------------------------------------
    /// The `sequence` frontmatter value is not a valid type (must be a list or a string).
    #[error("invalid sequence definition: {0}")]
    SequenceInvalid(String),

    /// The sequence list is empty.
    #[error("sequence list is empty; at least one step is required")]
    SequenceEmpty,

    /// The external YAML file could not be loaded.
    #[error("failed to load external sequence file: {0}")]
    SequenceExternalLoad(String),

    /// The external YAML file has an unexpected root shape.
    #[error("external sequence file has wrong structure: {0}")]
    SequenceExternalWrongType(String),

    /// An object step is missing the required `name` property.
    #[error("sequence step at index {index} is missing required `name` property")]
    SequenceStepNameMissing { index: usize },

    /// The `name` property of an object step is not a string.
    #[error("sequence step at index {index} has `name` of type {found}, expected string")]
    SequenceStepNameWrongType { index: usize, found: String },

    /// A template value is not a string.
    #[error("sequence template key `{key}` has type {found}, expected string")]
    SequenceTemplateWrongType { key: String, found: String },

    /// Templates require all list items to be objects.
    #[error("sequence templates require all list items to be objects (dictionaries)")]
    SequenceTemplateRequiresObjectItems,

    /// A template key collides with a reserved sequence overlay key.
    #[error("sequence template key `{0}` collides with reserved sequence key")]
    SequenceReservedTemplateKey(String),

    /// One or more sequence steps failed provider/model resolution in non-TTY mode.
    #[error("sequence selection failed for {failure_count} step(s): {failures:?}")]
    SequenceSelectionFailed {
        failures: Vec<SequenceSelectionFailure>,
        failure_count: usize,
    },

    // -- Loop errors -----------------------------------------------------------
    /// The `loop` frontmatter value is invalid.
    #[error("invalid loop definition: {0}")]
    LoopInvalid(String),

    /// Loop execution exceeded its configured safety cap.
    #[error(
        "loop limit exceeded for {prompt_path} at iteration {iteration}; cap is {cap}",
        prompt_path = prompt_path.display()
    )]
    LoopLimitExceeded {
        /// Maximum allowed iteration count.
        cap: usize,
        /// Prompt file being executed.
        prompt_path: PathBuf,
        /// 1-based iteration that exceeded the limit.
        iteration: usize,
    },

    /// A loop action failed validation or execution.
    #[error(
        "invalid loop action at iteration {iteration}, action {action_index} of {total_actions}: {message}"
    )]
    InvalidAction {
        /// 1-based iteration index.
        iteration: usize,
        /// 1-based action index.
        action_index: usize,
        /// Total actions in this iteration.
        total_actions: usize,
        /// Human-readable failure reason.
        message: String,
    },

    /// Increment targeted a property with an unsupported type.
    #[error(
        "invalid increment at iteration {iteration}, action {action_index} of {total_actions}: property `{property}` has type {found}"
    )]
    InvalidIncrementType {
        /// 1-based iteration index.
        iteration: usize,
        /// 1-based action index.
        action_index: usize,
        /// Total actions in this iteration.
        total_actions: usize,
        /// Property that was targeted.
        property: String,
        /// Actual property type.
        found: String,
    },

    /// Decrement targeted a property with an unsupported type.
    #[error(
        "invalid decrement at iteration {iteration}, action {action_index} of {total_actions}: property `{property}` has type {found}"
    )]
    InvalidDecrementType {
        /// 1-based iteration index.
        iteration: usize,
        /// 1-based action index.
        action_index: usize,
        /// Total actions in this iteration.
        total_actions: usize,
        /// Property that was targeted.
        property: String,
        /// Actual property type.
        found: String,
    },

    /// Loop execution was interrupted by the user (SIGINT / Ctrl+C).
    #[error(
        "user interrupted looping operation in {prompt_path}",
        prompt_path = prompt_path.display()
    )]
    LoopInterrupted {
        /// Prompt file being executed when the interrupt was observed.
        prompt_path: PathBuf,
    },

    /// A loop iteration's provider exited non-zero.
    ///
    /// This is a *runtime* failure distinct from [`Self::LoopInvalid`] (a
    /// frontmatter parse problem) and from [`Self::LoopInterrupted`]
    /// (Ctrl+C). The `reason` field carries a human-readable cause derived
    /// from the iteration's `session_end` JSONL row (e.g. `step_timeout`,
    /// `wall-clock timeout`, `signal SIGTERM`, `provider exited non-zero`).
    /// The `exit_reason` field carries the structured `error_kind` string
    /// for downstream tooling.
    #[error(
        "loop iteration {iteration} of {prompt_path}: {reason} (exit code {exit_code})",
        prompt_path = prompt_path.display()
    )]
    LoopIterationFailed {
        /// 1-based iteration that failed.
        iteration: usize,
        /// Prompt file being executed.
        prompt_path: PathBuf,
        /// Process-style exit code for the failed iteration.
        exit_code: i32,
        /// Human-readable cause. Never empty.
        reason: String,
        /// Structured `error_kind` from the iteration's session_end row when
        /// one is present (e.g. `step_timeout`, `wall_clock_timeout`,
        /// `signal`, `usage_limit_reached`). `None` when no row was written.
        exit_reason: Option<String>,
    },

    /// A loop iteration completed but reported a provider rate limit, and
    /// either the configured `on_rate_limit` policy was `abort` or no
    /// `reset_at` was available to safely pause.
    ///
    /// Maps to exit code [`LOOP_RATE_LIMITED_EXIT_CODE`] (`75`, `EX_TEMPFAIL`).
    #[error(
        "loop halted at iteration {iteration} of {prompt_path}: provider rate limited{}{}{}",
        provider.as_ref().map(|p| format!(" ({p})")).unwrap_or_default(),
        reset_at.as_ref().map(|r| format!("; resets at {}", r.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S"))).unwrap_or_default(),
        message.as_ref().map(|m| format!("\n  ↳ {m}")).unwrap_or_default(),
        prompt_path = prompt_path.display()
    )]
    LoopRateLimited {
        /// 1-based iteration that produced the rate-limit trailer.
        iteration: usize,
        /// Prompt file being executed.
        prompt_path: PathBuf,
        /// Provider id reported by the throttling source, when known.
        provider: Option<String>,
        /// Model id reported by the throttling source, when known.
        model: Option<String>,
        /// When the cap is reported to reset, when known.
        reset_at: Option<DateTime<Utc>>,
        /// Provider-supplied human-readable message, when known.
        message: Option<String>,
    },
}

/// Per-step failure information for sequence selection errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceSelectionFailure {
    /// 1-based step number.
    pub step: usize,
    /// Display name of the step.
    pub step_name: String,
    /// Why the step failed resolution.
    pub reason: String,
    /// Providers that were installed at the time of resolution.
    pub installed: Vec<Provider>,
}

impl BlockError for CompositionError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        match self {
            CompositionError::LifecycleInvalid {
                property,
                source_file,
                unknown_field,
                expected_fields,
                ..
            } => {
                let dotted_property = match unknown_field {
                    Some(field) => format!("{property}.{field}"),
                    None => property.clone(),
                };

                let file_display = source_file.display().to_string();
                let escaped = escape_prose_path(&file_display);
                let file_link = format!(
                    "<a href=\"{escaped}\">{}</a>",
                    escape_prose_path(&source_file.file_name().map_or_else(
                        || file_display.to_string(),
                        |n| n.to_string_lossy().to_string()
                    ))
                );

                let mut body =
                    format!("Unknown property <cyan>`{dotted_property}`</cyan> in {file_link}");
                if !expected_fields.is_empty() {
                    body.push_str("\n\n<b>Expected one of:</b>");
                    for field in expected_fields {
                        body.push_str(&format!("\n- <cyan>`{field}`</cyan>"));
                    }
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "CompositionError",
                        "invalid lifecycle property",
                    ))
                    .body(body)
                    .hint("Check the lifecycle frontmatter section in your prompt file.")
            }
            CompositionError::LoopIterationFailed {
                iteration,
                exit_code,
                reason,
                exit_reason,
                ..
            } => {
                // Surface the actionable cause (`step_timeout`,
                // `wall-clock timeout`, signal, …) in the header instead of
                // the generic `composition failed` line. The cause comes
                // from the iteration's session_end JSONL row's
                // `extra.exit_reason` — not from `LoopInvalid` (which is
                // reserved for frontmatter parse errors).
                let title = exit_reason
                    .clone()
                    .unwrap_or_else(|| "iteration failed".to_string());
                let body =
                    format!("Iteration {iteration} exited with code {exit_code}.\n\n{reason}");
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", &title))
                    .body(body)
            }
            CompositionError::LoopRateLimited { .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("CompositionError", "rate limited"))
                .body(self.to_string())
                .hint(
                    "Re-run after the listed reset time, or use \
                         `--on-rate-limit pause` to wait automatically.",
                ),
            _ => {
                let msg = self.to_string();
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("CompositionError", "composition failed"))
                    .body(msg)
            }
        }
    }
}

fn escape_prose_path(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '<' | '>' | '{' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_iteration_failed_display_surfaces_reason_and_iteration() {
        let err = CompositionError::LoopIterationFailed {
            iteration: 2,
            prompt_path: PathBuf::from("fixes/plan.md"),
            exit_code: 1,
            reason: "step_timeout after 30m of stream silence".to_string(),
            exit_reason: Some("step_timeout".to_string()),
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("loop iteration 2 of fixes/plan.md"),
            "got: {rendered}"
        );
        assert!(
            rendered.contains("step_timeout after 30m of stream silence"),
            "got: {rendered}"
        );
        assert!(
            !rendered.contains("invalid loop definition"),
            "got: {rendered}"
        );
    }

    #[test]
    fn loop_rate_limited_display_includes_reset_time_when_present() {
        let reset = DateTime::parse_from_rfc3339("2026-05-12T18:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let err = CompositionError::LoopRateLimited {
            iteration: 1,
            prompt_path: PathBuf::from("plan.md"),
            provider: Some("k2p6".to_string()),
            model: Some("kimi-for-coding".to_string()),
            reset_at: Some(reset),
            message: Some("Usage limit reached for k2p6".to_string()),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("k2p6"), "got: {rendered}");
        assert!(rendered.contains("resets at"), "got: {rendered}");
        assert!(rendered.contains("Usage limit reached"), "got: {rendered}");
    }

    #[test]
    fn loop_rate_limited_display_omits_optional_fields_when_absent() {
        let err = CompositionError::LoopRateLimited {
            iteration: 3,
            prompt_path: PathBuf::from("plan.md"),
            provider: None,
            model: None,
            reset_at: None,
            message: None,
        };
        let rendered = err.to_string();
        assert!(
            rendered.starts_with("loop halted at iteration 3 of plan.md"),
            "got: {rendered}"
        );
        assert!(
            rendered.contains("provider rate limited"),
            "got: {rendered}"
        );
        // No reset clause when reset_at is absent
        assert!(!rendered.contains("resets at"), "got: {rendered}");
    }

    #[test]
    fn loop_iteration_failed_falls_back_when_no_exit_reason() {
        let err = CompositionError::LoopIterationFailed {
            iteration: 4,
            prompt_path: PathBuf::from("plan.md"),
            exit_code: 1,
            reason: "provider exited non-zero".to_string(),
            exit_reason: None,
        };
        let rendered = err.to_string();
        assert!(
            rendered.contains("provider exited non-zero"),
            "got: {rendered}"
        );
    }

    #[test]
    fn loop_invalid_still_reserved_for_frontmatter_problems() {
        // Sanity: LoopInvalid is still the right variant for malformed
        // frontmatter and renders distinctly from the runtime-fault
        // variants above.
        let err = CompositionError::LoopInvalid("`loop.max` must be greater than zero".to_string());
        let rendered = err.to_string();
        assert!(
            rendered.starts_with("invalid loop definition:"),
            "got: {rendered}"
        );
    }
}
