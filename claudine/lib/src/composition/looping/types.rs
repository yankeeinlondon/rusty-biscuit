//! Loop execution value types — runtime options, per-iteration context and
//! output, and the final loop result.
//!
//! These are the data types consumed and produced by the engine
//! ([`super::engine`]); the engine module holds the execution/routing/gate
//! logic proper.

use std::path::PathBuf;

use serde_json::{Map, Value};

use super::super::error::CompositionError;
use super::super::lifecycle::LifecycleSignal;
use super::super::types::OnRateLimit;
use super::expression::LoopAmbient;
use crate::stream::summary::RateLimitInfo;

/// Runtime options that can override per-document loop configuration.
///
/// `PartialEq` is intentionally not derived: the `interrupt_check` field is
/// a function pointer, and function-pointer equality is not meaningful in
/// Rust.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoopExecutionOptions {
    /// Runtime iteration cap override.
    pub max_iterations: Option<usize>,
    /// Runtime fail-fast override.
    pub fail_fast: Option<bool>,
    /// Runtime rate-limit policy override. When set, takes precedence over
    /// any per-document [`LoopConfig::on_rate_limit`](super::super::types::LoopConfig::on_rate_limit).
    pub on_rate_limit: Option<OnRateLimit>,
    /// Optional interrupt poll, used by the engine during rate-limit
    /// pause sleeps to short-circuit if the user hits Ctrl+C. The function
    /// should return `true` when an interrupt has been observed.
    ///
    /// The engine itself never installs signal handlers — that remains the
    /// CLI's responsibility. When `None`, pause sleeps run to completion.
    pub interrupt_check: Option<fn() -> bool>,
    /// Override for the safety margin added on top of a provider's `reset_at`
    /// when pausing for a rate limit. `None` uses the built-in
    /// `PAUSE_RESET_MARGIN`. The CLI populates this from
    /// `CLAUDINE_PAUSE_RESET_MARGIN`; tests inject a near-zero value to keep
    /// pause-policy coverage fast without weakening it.
    pub pause_reset_margin: Option<std::time::Duration>,
}

/// Context passed to a single loop iteration executor.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopIterationContext {
    /// 1-based iteration index.
    pub iteration: usize,
    /// Frontmatter state for this iteration.
    pub frontmatter: Map<String, Value>,
    /// Ambient variables for this iteration.
    pub ambient: LoopAmbient,
}

impl LoopIterationContext {
    /// Build `set_overrides` for prompt preparation.
    ///
    /// The returned object contains the current frontmatter plus read-only
    /// ambient loop variables. Ambient variables intentionally shadow
    /// frontmatter keys for the duration of an iteration.
    pub fn as_set_overrides(&self) -> Value {
        let mut overrides = self.frontmatter.clone();
        insert_ambient_overrides(&mut overrides, &self.ambient);
        Value::Object(overrides)
    }
}

/// Result from executing one prompt iteration.
#[derive(Debug, Default)]
pub struct LoopIterationOutput {
    /// Captured stdout or composed output for this iteration.
    pub output: String,
    /// Process-style exit code for this iteration.
    pub exit_code: i32,
    /// Optional execution error associated with the exit code.
    pub error: Option<CompositionError>,
    /// Terminal lifecycle signal emitted by this iteration, if any.
    ///
    /// Used by the loop engine to apply `fail_fast` semantics and to
    /// sequence the post-`finalize` loop gate.
    pub terminal_signal: Option<LifecycleSignal>,
    /// Rate-limit signal observed during this iteration, when present.
    ///
    /// Read by the engine between iterations to apply the configured
    /// [`OnRateLimit`] policy. May be set even on successful iterations —
    /// providers commonly attach a trailing rate-limit notice after a
    /// completion summary.
    pub rate_limit: Option<RateLimitInfo>,
    /// Structured `error_kind` from the iteration's session_end JSONL row
    /// (e.g. `step_timeout`, `wall_clock_timeout`, `usage_limit_reached`).
    ///
    /// Used by the loop runner to construct
    /// [`CompositionError::LoopIterationFailed`] with an honest cause
    /// instead of overloading [`CompositionError::LoopInvalid`].
    pub exit_reason: Option<String>,
    /// Provider identifier reported by the iteration's summary, when known.
    /// Used by the engine to enrich [`CompositionError::LoopRateLimited`]
    /// with attribution.
    pub provider_id: Option<String>,
    /// Model identifier reported by the iteration's summary, when known.
    /// Used by the engine to enrich [`CompositionError::LoopRateLimited`]
    /// with attribution.
    pub model_id: Option<String>,
}

impl LoopIterationOutput {
    /// Construct a successful iteration output.
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            exit_code: 0,
            error: None,
            terminal_signal: Some(LifecycleSignal::Success),
            rate_limit: None,
            exit_reason: None,
            provider_id: None,
            model_id: None,
        }
    }

    /// Construct a failed iteration output with a process-style exit code.
    pub fn failure(output: impl Into<String>, exit_code: i32, error: CompositionError) -> Self {
        Self {
            output: output.into(),
            exit_code,
            error: Some(error),
            terminal_signal: Some(LifecycleSignal::Failure),
            rate_limit: None,
            exit_reason: None,
            provider_id: None,
            model_id: None,
        }
    }

    /// Attach provider/model attribution to this output (builder style).
    #[must_use]
    pub fn with_attribution(
        mut self,
        provider_id: Option<String>,
        model_id: Option<String>,
    ) -> Self {
        self.provider_id = provider_id;
        self.model_id = model_id;
        self
    }

    /// Attach a rate-limit signal to this output (builder style).
    #[must_use]
    pub fn with_rate_limit(mut self, rate_limit: Option<RateLimitInfo>) -> Self {
        self.rate_limit = rate_limit;
        self
    }

    /// Attach a structured exit reason to this output (builder style).
    #[must_use]
    pub fn with_exit_reason(mut self, exit_reason: Option<String>) -> Self {
        self.exit_reason = exit_reason;
        self
    }

    /// Attach the terminal lifecycle signal emitted by the iteration.
    #[must_use]
    pub fn with_terminal_signal(mut self, signal: Option<LifecycleSignal>) -> Self {
        self.terminal_signal = signal;
        self
    }
}

/// Final result from a loop run.
#[derive(Debug)]
pub struct LoopExecutionResult {
    /// Exit code from the last executed iteration, or `0` if no iteration ran.
    pub final_exit_code: i32,
    /// Final committed frontmatter state.
    pub final_frontmatter: Map<String, Value>,
    /// Number of prompt iterations that actually ran.
    pub iteration_count: usize,
    /// Last captured iteration output.
    pub last_output: String,
    /// Optional loop, action, or iteration execution error.
    pub error: Option<CompositionError>,
    /// Resolved target document when `initialize` returned `Proxy`. The caller
    /// re-enters with this document so the target's own `initialize` (and its
    /// `Skip`/`Proxy`/`Error` controls) get a chance to run. `None` in every
    /// other case.
    pub init_proxy_target: Option<PathBuf>,
}

impl LoopExecutionResult {
    pub(super) fn success(
        final_frontmatter: Map<String, Value>,
        iteration_count: usize,
        last_output: String,
        final_exit_code: i32,
    ) -> Self {
        Self {
            final_exit_code,
            final_frontmatter,
            iteration_count,
            last_output,
            error: None,
            init_proxy_target: None,
        }
    }

    pub(super) fn failure(
        final_frontmatter: Map<String, Value>,
        iteration_count: usize,
        last_output: String,
        final_exit_code: i32,
        error: CompositionError,
    ) -> Self {
        Self {
            final_exit_code,
            final_frontmatter,
            iteration_count,
            last_output,
            error: Some(error),
            init_proxy_target: None,
        }
    }

    /// Attach a resolved proxy target for the caller to hand off to.
    #[must_use]
    pub fn with_init_proxy_target(mut self, target: PathBuf) -> Self {
        self.init_proxy_target = Some(target);
        self
    }
}

/// Insert the read-only ambient loop variables (`_loop_*`) into a frontmatter
/// override map for prompt preparation.
fn insert_ambient_overrides(frontmatter: &mut Map<String, Value>, ambient: &LoopAmbient) {
    frontmatter.insert(
        "_loop_count".to_string(),
        Value::Number(ambient.iteration.into()),
    );
    frontmatter.insert("_loop_is_first".to_string(), Value::Bool(ambient.is_first));
    frontmatter.insert("_loop_is_last".to_string(), Value::Bool(ambient.is_last));
    frontmatter.insert(
        "_loop_last_output".to_string(),
        Value::String(ambient.last_output.clone()),
    );
    frontmatter.insert(
        "_loop_last_exit_code".to_string(),
        Value::Number(ambient.last_exit_code.into()),
    );
}
