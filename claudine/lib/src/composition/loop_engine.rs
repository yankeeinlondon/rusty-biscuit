//! Loop execution orchestration.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::error::CompositionError;
use super::loop_actions::ActionStaging;
use super::loop_config::resolve_loop_config;
use super::loop_expression::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
use super::types::{LoopConfig, OnRateLimit, ResolvedCompositionSource};
use crate::stream::summary::RateLimitInfo;

/// Default safety cap for prompt loops.
pub const DEFAULT_MAX_ITERATIONS: usize = 100;

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
    /// any per-document [`LoopConfig::on_rate_limit`].
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

/// How long the engine sleeps between interrupt-flag checks while pausing
/// for a rate-limit reset.
///
/// Short enough to be responsive to Ctrl+C without burning CPU on the poll.
const PAUSE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// Safety margin added on top of a provider's `reset_at` to absorb skew —
/// providers commonly return `429` for a moment after the nominal reset.
const PAUSE_RESET_MARGIN: std::time::Duration = std::time::Duration::from_secs(5);

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
}

impl LoopExecutionResult {
    fn success(
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
        }
    }

    fn failure(
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
        }
    }
}

/// Execute a loop defined on a resolved composition source.
///
/// Returns `Ok(None)` when the source has no `loop` frontmatter.
///
/// ## Errors
///
/// Returns parse/evaluation errors that prevent the engine from determining
/// loop control flow. Per-iteration prompt/action failures are represented in
/// [`LoopExecutionResult::error`] according to fail-fast semantics.
pub fn execute_loop(
    source: &ResolvedCompositionSource,
    options: LoopExecutionOptions,
    executor: impl FnMut(LoopIterationContext) -> Result<LoopIterationOutput, CompositionError>,
) -> Result<Option<LoopExecutionResult>, CompositionError> {
    let Some(config) = resolve_loop_config(source)? else {
        return Ok(None);
    };
    let initial_frontmatter = source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    execute_loop_with_config(
        &source.resolved_path,
        &config,
        initial_frontmatter,
        options,
        executor,
    )
    .map(Some)
}

/// Execute a loop with an already parsed configuration and initial state.
///
/// This is the core engine used by tests and by higher-level CLI integration.
///
/// ## Errors
///
/// Returns condition evaluation errors. Runtime prompt/action failures are
/// carried by the returned [`LoopExecutionResult`] so callers can report the
/// final state together with the error.
pub fn execute_loop_with_config(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    options: LoopExecutionOptions,
    mut executor: impl FnMut(LoopIterationContext) -> Result<LoopIterationOutput, CompositionError>,
) -> Result<LoopExecutionResult, CompositionError> {
    let max_iterations = options
        .max_iterations
        .or(config.max_iterations)
        .unwrap_or(DEFAULT_MAX_ITERATIONS);
    let fail_fast = options.fail_fast.or(config.fail_fast).unwrap_or(true);
    let on_rate_limit = options
        .on_rate_limit
        .or(config.on_rate_limit)
        .unwrap_or_default();

    // Read-side expression functions in loop conditions resolve against the
    // prompt document's directory; the probe re-runs each iteration while this
    // base stays fixed.
    let base_dir = prompt_path.parent();

    let mut frontmatter = initial_frontmatter;
    let mut iteration_count = 0usize;
    let mut last_output = String::new();
    let mut last_exit_code = 0i32;

    for iteration in 1..=max_iterations {
        let is_last = compute_is_last(
            prompt_path,
            config,
            &frontmatter,
            iteration,
            max_iterations,
            &last_output,
            last_exit_code,
        )?;
        let ambient = LoopAmbient::new(
            iteration,
            iteration == 1,
            is_last,
            last_output.clone(),
            last_exit_code,
        );
        let lookup = LoopExpressionLookup::new(&frontmatter, &ambient).with_base_dir(base_dir);
        if !evaluate_condition(&config.condition, &lookup)? {
            return Ok(LoopExecutionResult::success(
                frontmatter,
                iteration_count,
                last_output,
                last_exit_code,
            ));
        }

        let context = LoopIterationContext {
            iteration,
            frontmatter: frontmatter.clone(),
            ambient,
        };
        let output = match executor(context) {
            Ok(output) => output,
            Err(error) => {
                last_output.clear();
                last_exit_code = 1;
                iteration_count += 1;
                if fail_fast {
                    return Ok(LoopExecutionResult::failure(
                        frontmatter,
                        iteration_count,
                        last_output,
                        last_exit_code,
                        error,
                    ));
                }
                continue;
            }
        };

        last_output = output.output;
        last_exit_code = output.exit_code;
        iteration_count += 1;
        let iteration_rate_limit = output.rate_limit.clone();
        let iteration_provider = output.provider_id.clone();
        let iteration_model = output.model_id.clone();

        if let Some(error) = output.error {
            if fail_fast {
                return Ok(LoopExecutionResult::failure(
                    frontmatter,
                    iteration_count,
                    last_output,
                    last_exit_code,
                    error,
                ));
            }
            continue;
        }

        // Apply the rate-limit policy when the iteration completed and a
        // throttling signal was attached. Skipped on the very last
        // iteration because the loop is about to exit anyway — pausing or
        // aborting would just delay (or falsely fail) a clean finish.
        if !is_last {
            match decide_rate_limit_action(
                iteration_rate_limit.as_ref(),
                on_rate_limit,
                prompt_path,
                iteration,
                iteration_provider,
                iteration_model,
                options.interrupt_check,
                options.pause_reset_margin.unwrap_or(PAUSE_RESET_MARGIN),
            ) {
                RateLimitOutcome::Proceed => {}
                RateLimitOutcome::Interrupted => {
                    // Caller's wrapped executor will short-circuit the
                    // next iteration and produce the LoopInterrupted
                    // error, so we just continue the loop here.
                }
                RateLimitOutcome::Abort(error) => {
                    return Ok(LoopExecutionResult::failure(
                        frontmatter,
                        iteration_count,
                        last_output,
                        last_exit_code,
                        error,
                    ));
                }
            }
        }

        // Build a post-executor lookup so action-time templates resolve
        // against the iteration that just ran: ambient `_loop_count`,
        // `_loop_is_first`, `_loop_is_last` reflect this iteration, while
        // `_loop_last_output` and `_loop_last_exit_code` reflect what the
        // executor produced moments ago.
        let post_ambient = LoopAmbient::new(
            iteration,
            iteration == 1,
            is_last,
            last_output.clone(),
            last_exit_code,
        );
        let post_lookup =
            LoopExpressionLookup::new(&frontmatter, &post_ambient).with_base_dir(base_dir);
        match apply_actions(config, &frontmatter, iteration, Some(&post_lookup)) {
            Ok(next_frontmatter) => frontmatter = next_frontmatter,
            Err(error) => {
                if fail_fast {
                    return Ok(LoopExecutionResult::failure(
                        frontmatter,
                        iteration_count,
                        last_output,
                        last_exit_code,
                        error,
                    ));
                }
            }
        }

        if iteration == max_iterations
            && should_continue_after_cap(
                config,
                &frontmatter,
                iteration + 1,
                &last_output,
                last_exit_code,
                base_dir,
            )?
        {
            return Ok(LoopExecutionResult::failure(
                frontmatter,
                iteration_count,
                last_output,
                last_exit_code,
                CompositionError::LoopLimitExceeded {
                    cap: max_iterations,
                    prompt_path: PathBuf::from(prompt_path),
                    iteration,
                },
            ));
        }
    }

    Ok(LoopExecutionResult::success(
        frontmatter,
        iteration_count,
        last_output,
        last_exit_code,
    ))
}

/// Outcome of consulting the rate-limit policy after an iteration.
enum RateLimitOutcome {
    /// No throttle, or `Continue` policy — keep iterating normally.
    Proceed,
    /// We paused and the interrupt callback fired during the sleep.
    /// Caller continues; the wrapped executor will surface the interrupt
    /// on the next iteration.
    Interrupted,
    /// Policy was `Abort` (or `Pause` without a usable reset clock).
    /// Caller propagates the contained error.
    Abort(CompositionError),
}

#[allow(clippy::too_many_arguments)]
fn decide_rate_limit_action(
    rate_limit: Option<&RateLimitInfo>,
    policy: OnRateLimit,
    prompt_path: &Path,
    iteration: usize,
    provider: Option<String>,
    model: Option<String>,
    interrupt_check: Option<fn() -> bool>,
    reset_margin: std::time::Duration,
) -> RateLimitOutcome {
    let Some(rl) = rate_limit else {
        return RateLimitOutcome::Proceed;
    };
    if rl.is_throttled != Some(true) {
        return RateLimitOutcome::Proceed;
    }

    let now = chrono::Utc::now();
    let reset = rl.reset_at;
    let safe_pause_window = match reset {
        Some(reset_at) if reset_at > now => Some(
            (reset_at - now)
                .to_std()
                .unwrap_or(std::time::Duration::ZERO)
                + reset_margin,
        ),
        _ => None,
    };

    match (policy, safe_pause_window) {
        (OnRateLimit::Continue, _) => RateLimitOutcome::Proceed,
        (OnRateLimit::Pause, Some(duration)) => {
            if interruptible_sleep(duration, interrupt_check) {
                RateLimitOutcome::Interrupted
            } else {
                RateLimitOutcome::Proceed
            }
        }
        // No usable reset clock under `Pause` falls back to `Abort` to
        // avoid an unbounded sleep. Explicit `Abort` lands here too.
        (OnRateLimit::Pause, None) | (OnRateLimit::Abort, _) => {
            RateLimitOutcome::Abort(CompositionError::LoopRateLimited {
                iteration,
                prompt_path: prompt_path.to_path_buf(),
                provider,
                model,
                reset_at: reset,
                message: rl.message.clone(),
            })
        }
    }
}

/// Sleep up to `duration`, polling `interrupt_check` every
/// [`PAUSE_POLL_INTERVAL`]. Returns `true` when the sleep was cut short by
/// an interrupt.
fn interruptible_sleep(
    duration: std::time::Duration,
    interrupt_check: Option<fn() -> bool>,
) -> bool {
    let deadline = std::time::Instant::now() + duration;
    loop {
        if let Some(check) = interrupt_check
            && check()
        {
            return true;
        }
        let now = std::time::Instant::now();
        if now >= deadline {
            return false;
        }
        let remaining = deadline - now;
        let slice = remaining.min(PAUSE_POLL_INTERVAL);
        std::thread::sleep(slice);
    }
}

fn apply_actions(
    config: &LoopConfig,
    frontmatter: &Map<String, Value>,
    iteration: usize,
    lookup: Option<&dyn darkmatter::markdown::compose::expression::EvaluationLookup>,
) -> Result<Map<String, Value>, CompositionError> {
    let mut stage = ActionStaging::new(frontmatter, iteration, config.actions.len());
    for (index, action) in config.actions.iter().enumerate() {
        stage.apply_action(action, index + 1, lookup)?;
    }
    Ok(stage.commit_map())
}

fn compute_is_last(
    prompt_path: &Path,
    config: &LoopConfig,
    frontmatter: &Map<String, Value>,
    iteration: usize,
    max_iterations: usize,
    last_output: &str,
    last_exit_code: i32,
) -> Result<bool, CompositionError> {
    if iteration == max_iterations {
        return Ok(true);
    }

    let base_dir = prompt_path.parent();

    // Speculative is_last computation: render templates against the
    // pre-iteration state. `_loop_last_output` / `_loop_last_exit_code`
    // here reflect the prior iteration (or the seed values on iteration 1)
    // because the current iteration has not run yet.
    let speculative_ambient = LoopAmbient::new(
        iteration,
        iteration == 1,
        iteration == max_iterations,
        last_output.to_string(),
        last_exit_code,
    );
    let speculative_lookup =
        LoopExpressionLookup::new(frontmatter, &speculative_ambient).with_base_dir(base_dir);
    let Ok(next_frontmatter) =
        apply_actions(config, frontmatter, iteration, Some(&speculative_lookup))
    else {
        return Ok(false);
    };
    let next_ambient = LoopAmbient::new(
        iteration + 1,
        false,
        iteration + 1 == max_iterations,
        last_output,
        last_exit_code,
    );
    let lookup = LoopExpressionLookup::new(&next_frontmatter, &next_ambient).with_base_dir(base_dir);
    evaluate_condition(&config.condition, &lookup)
        .map(|will_continue| !will_continue)
        .map_err(|error| match error {
            CompositionError::LoopInvalid(message) => CompositionError::LoopInvalid(format!(
                "failed to compute loop is_last for {} at iteration {iteration}: {message}",
                prompt_path.display()
            )),
            other => other,
        })
}

fn should_continue_after_cap(
    config: &LoopConfig,
    frontmatter: &Map<String, Value>,
    next_iteration: usize,
    last_output: &str,
    last_exit_code: i32,
    base_dir: Option<&Path>,
) -> Result<bool, CompositionError> {
    let ambient = LoopAmbient::new(next_iteration, false, true, last_output, last_exit_code);
    let lookup = LoopExpressionLookup::new(frontmatter, &ambient).with_base_dir(base_dir);
    evaluate_condition(&config.condition, &lookup)
}

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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;

    use serde_json::json;

    use super::*;
    use crate::composition::types::{LoopAction, LoopCondition};

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn counter_loop(max: usize) -> LoopConfig {
        LoopConfig {
            condition: LoopCondition::While(format!("counter < {max}")),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        }
    }

    #[test]
    fn runs_until_condition_stops_and_commits_actions() {
        let config = counter_loop(3);
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| Ok(LoopIterationOutput::success("ok")),
        )
        .unwrap();

        assert!(result.error.is_none());
        assert_eq!(result.iteration_count, 3);
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(3)));
        assert_eq!(result.final_exit_code, 0);
        assert_eq!(result.last_output, "ok");
    }

    #[test]
    fn injects_ambient_values_and_current_frontmatter() {
        let config = counter_loop(2);
        let seen = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0, "iteration": 99})),
            LoopExecutionOptions::default(),
            |ctx| {
                seen.borrow_mut().push(ctx.as_set_overrides());
                Ok(LoopIterationOutput::success(format!(
                    "run {}",
                    ctx.iteration
                )))
            },
        )
        .unwrap();

        assert!(result.error.is_none());
        let seen = seen.borrow();
        assert_eq!(seen[0]["counter"], json!(0));
        // User frontmatter property `iteration` is preserved verbatim
        // because loop ambients live under `_loop_*`.
        assert_eq!(seen[0]["iteration"], json!(99));
        assert_eq!(seen[0]["_loop_count"], json!(1));
        assert_eq!(seen[0]["_loop_is_first"], json!(true));
        assert_eq!(seen[0]["_loop_last_output"], json!(""));
        assert_eq!(seen[1]["counter"], json!(1));
        assert_eq!(seen[1]["iteration"], json!(99));
        assert_eq!(seen[1]["_loop_count"], json!(2));
        assert_eq!(seen[1]["_loop_is_first"], json!(false));
        assert_eq!(seen[1]["_loop_last_output"], json!("run 1"));
    }

    #[test]
    fn computes_is_last_from_post_action_condition() {
        let config = counter_loop(3);
        let seen = RefCell::new(Vec::new());
        execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |ctx| {
                seen.borrow_mut().push(ctx.ambient.is_last);
                Ok(LoopIterationOutput::success("ok"))
            },
        )
        .unwrap();

        assert_eq!(&*seen.borrow(), &[false, false, true]);
    }

    #[test]
    fn computes_is_last_when_max_iterations_is_stopping_condition() {
        let config = LoopConfig {
            condition: LoopCondition::While("true".into()),
            actions: vec![],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let seen = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            Map::new(),
            LoopExecutionOptions {
                max_iterations: Some(2),
                fail_fast: None,
                on_rate_limit: None,
                interrupt_check: None,
                pause_reset_margin: None,
            },
            |ctx| {
                seen.borrow_mut().push(ctx.ambient.is_last);
                Ok(LoopIterationOutput::success("ok"))
            },
        )
        .unwrap();

        assert_eq!(&*seen.borrow(), &[false, true]);
        assert!(matches!(
            result.error,
            Some(CompositionError::LoopLimitExceeded {
                cap: 2,
                iteration: 2,
                ..
            })
        ));
    }

    #[test]
    fn fail_fast_false_continues_after_iteration_failure() {
        let config = LoopConfig {
            condition: LoopCondition::While("_loop_count < 4".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: Some(false),
            on_rate_limit: None,
        };
        let seen_exit_codes = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |ctx| {
                seen_exit_codes
                    .borrow_mut()
                    .push(ctx.ambient.last_exit_code);
                if ctx.iteration == 1 {
                    Ok(LoopIterationOutput::failure(
                        "failed",
                        42,
                        CompositionError::LoopInvalid("iteration failed".into()),
                    ))
                } else {
                    Ok(LoopIterationOutput::success("ok"))
                }
            },
        )
        .unwrap();

        assert!(result.error.is_none());
        assert_eq!(&*seen_exit_codes.borrow(), &[0, 42, 0]);
        assert_eq!(result.iteration_count, 3);
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(2)));
    }

    #[test]
    fn fail_fast_true_stops_after_iteration_failure() {
        let config = counter_loop(3);
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| {
                Ok(LoopIterationOutput::failure(
                    "failed",
                    9,
                    CompositionError::LoopInvalid("iteration failed".into()),
                ))
            },
        )
        .unwrap();

        assert_eq!(result.iteration_count, 1);
        assert_eq!(result.final_exit_code, 9);
        assert!(matches!(
            result.error,
            Some(CompositionError::LoopInvalid(_))
        ));
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(0)));
    }

    #[test]
    fn fail_fast_false_discards_failed_action_stage() {
        let config = LoopConfig {
            condition: LoopCondition::While("_loop_count < 3".into()),
            actions: vec![
                LoopAction::Increment("counter".into()),
                LoopAction::Increment("bad".into()),
            ],
            max_iterations: None,
            fail_fast: Some(false),
            on_rate_limit: None,
        };
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0, "bad": "abc"})),
            LoopExecutionOptions::default(),
            |_ctx| Ok(LoopIterationOutput::success("ok")),
        )
        .unwrap();

        assert!(result.error.is_none());
        assert_eq!(result.iteration_count, 2);
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(0)));
        assert_eq!(result.final_frontmatter.get("bad"), Some(&json!("abc")));
    }

    #[test]
    fn set_template_renders_against_post_executor_iteration_state() {
        // After iteration N runs, `set(stamp, {{_loop_count}})` should land
        // a typed JSON number reflecting the iteration that just ran (N),
        // and `set(echo, {{_loop_last_output}})` should reflect the output
        // the executor produced moments ago.
        let config = LoopConfig {
            condition: LoopCondition::While("_loop_count < 3".into()),
            actions: vec![
                LoopAction::Set {
                    prop: "stamp".into(),
                    value: Value::String("{{_loop_count}}".into()),
                },
                LoopAction::Set {
                    prop: "echo".into(),
                    value: Value::String("{{_loop_last_output}}".into()),
                },
            ],
            max_iterations: Some(2),
            fail_fast: None,
            on_rate_limit: None,
        };
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            Map::new(),
            LoopExecutionOptions::default(),
            |ctx| {
                Ok(LoopIterationOutput::success(format!(
                    "ran-{}",
                    ctx.iteration
                )))
            },
        )
        .unwrap();

        assert!(result.error.is_none());
        // After iteration 2 runs and its actions apply, `stamp` should be
        // the JSON number 2 (typed), and `echo` should be the string output
        // captured from iteration 2's executor.
        assert_eq!(result.final_frontmatter.get("stamp"), Some(&json!(2)));
        assert_eq!(result.final_frontmatter.get("echo"), Some(&json!("ran-2")));
    }

    #[test]
    fn five_iteration_counter_loop() {
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 5".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| Ok(LoopIterationOutput::success("tick")),
        )
        .unwrap();

        assert!(result.error.is_none());
        assert_eq!(result.iteration_count, 5);
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(5)));
        assert_eq!(result.last_output, "tick");
    }

    #[test]
    fn until_loop_runs_until_condition_met() {
        // until: "counter >= 2" means "continue while counter < 2"
        // actions increment counter each iteration, so 2 iterations run
        let config = LoopConfig {
            condition: LoopCondition::Until("counter >= 2".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| Ok(LoopIterationOutput::success("ok")),
        )
        .unwrap();

        assert!(result.error.is_none());
        // Iteration 1: counter=0 < 2 -> continue -> counter=1
        // Iteration 2: counter=1 < 2 -> continue -> counter=2
        // Iteration 3: counter=2 >= 2 -> stop
        assert_eq!(result.iteration_count, 2);
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(2)));
    }

    #[test]
    fn until_loop_with_counter_reaches_target() {
        // Continue until counter >= 3; actions increment each iteration
        let config = LoopConfig {
            condition: LoopCondition::Until("counter >= 3".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| Ok(LoopIterationOutput::success("ok")),
        )
        .unwrap();

        assert!(result.error.is_none());
        // Iteration 1: counter=0 < 3 -> continue -> counter=1
        // Iteration 2: counter=1 < 3 -> continue -> counter=2
        // Iteration 3: counter=2 < 3 -> continue -> counter=3
        // Iteration 4: counter=3 >= 3 -> stop
        assert_eq!(result.iteration_count, 3);
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(3)));
    }

    #[test]
    fn append_accumulates_log_across_iterations() {
        let config = LoopConfig {
            condition: LoopCondition::While("_loop_count < 4".into()),
            actions: vec![LoopAction::Append {
                prop: "log".into(),
                value: json!({"event": "tick"}),
            }],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"log": ""})),
            LoopExecutionOptions::default(),
            |_ctx| Ok(LoopIterationOutput::success("ok")),
        )
        .unwrap();

        assert!(result.error.is_none());
        assert_eq!(result.iteration_count, 3);
        let log = result
            .final_frontmatter
            .get("log")
            .unwrap()
            .as_str()
            .unwrap();
        assert_eq!(log.matches("tick").count(), 3);
    }

    #[test]
    fn last_output_and_last_exit_code_propagate() {
        let config = LoopConfig {
            condition: LoopCondition::While("_loop_count < 4".into()),
            actions: vec![],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let outputs = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({})),
            LoopExecutionOptions::default(),
            |ctx| {
                let out = format!("run-{}", ctx.iteration);
                outputs.borrow_mut().push((
                    ctx.iteration,
                    ctx.ambient.last_output.clone(),
                    ctx.ambient.last_exit_code,
                ));
                Ok(LoopIterationOutput::success(out))
            },
        )
        .unwrap();

        assert!(result.error.is_none());
        assert_eq!(result.iteration_count, 3);
        assert_eq!(result.last_output, "run-3");

        let seen = outputs.borrow();
        assert_eq!(seen[0], (1, String::new(), 0));
        assert_eq!(seen[1], (2, "run-1".into(), 0));
        assert_eq!(seen[2], (3, "run-2".into(), 0));
    }

    #[test]
    fn last_exit_code_reflects_failure_in_next_iteration() {
        let config = LoopConfig {
            condition: LoopCondition::While("_loop_count < 4".into()),
            actions: vec![],
            max_iterations: None,
            fail_fast: Some(false),
            on_rate_limit: None,
        };
        let exit_codes = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({})),
            LoopExecutionOptions::default(),
            |ctx| {
                exit_codes.borrow_mut().push(ctx.ambient.last_exit_code);
                if ctx.iteration == 2 {
                    Ok(LoopIterationOutput::failure(
                        "bad",
                        7,
                        CompositionError::LoopInvalid("boom".into()),
                    ))
                } else {
                    Ok(LoopIterationOutput::success("ok"))
                }
            },
        )
        .unwrap();

        assert!(result.error.is_none());
        assert_eq!(result.iteration_count, 3);
        assert_eq!(result.final_exit_code, 0);

        let seen = exit_codes.borrow();
        assert_eq!(&*seen, &[0, 0, 7]);
    }

    #[test]
    fn until_file_exists_resolves_against_prompt_parent() {
        // `until="file_exists('artifact')"` continues while the artifact is
        // absent and stops once the executor creates it under the prompt's
        // parent directory — proving the loop condition's read-side function
        // resolves against the prompt document root, re-probed each iteration.
        let dir = tempfile::TempDir::new().unwrap();
        let prompt_path = dir.path().join("loop.md");
        let artifact = dir.path().join("artifact");

        let config = LoopConfig {
            condition: LoopCondition::Until("file_exists('artifact')".into()),
            actions: vec![],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };

        let result = execute_loop_with_config(
            &prompt_path,
            &config,
            Map::new(),
            LoopExecutionOptions::default(),
            |ctx| {
                // Create the artifact on the third iteration; earlier passes
                // see it absent and keep looping.
                if ctx.iteration == 3 {
                    std::fs::write(&artifact, "done").unwrap();
                }
                Ok(LoopIterationOutput::success("ok"))
            },
        )
        .unwrap();

        assert!(result.error.is_none(), "got: {result:?}");
        assert_eq!(result.iteration_count, 3);
    }

    // ── Rate-limit policy tests ──────────────────────────────────────────

    fn throttled(message: Option<&str>, reset_in_secs: Option<i64>) -> RateLimitInfo {
        RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: None,
            message: message.map(str::to_string),
            reset_at: reset_in_secs.map(|s| chrono::Utc::now() + chrono::Duration::seconds(s)),
        }
    }

    #[test]
    fn rate_limit_continue_policy_proceeds_without_pausing() {
        // While-condition exits after 2 successful iterations. Even though
        // iteration 1 carries a rate-limit trailer, the `Continue` policy
        // means we don't pause and we don't abort.
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 2".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: Some(OnRateLimit::Continue),
        };

        let observed = RefCell::new(Vec::new());
        let start = std::time::Instant::now();
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |ctx| {
                observed.borrow_mut().push(ctx.iteration);
                Ok(LoopIterationOutput::success("ok")
                    .with_rate_limit(Some(throttled(Some("hit cap"), Some(60)))))
            },
        )
        .unwrap();
        let elapsed = start.elapsed();

        assert!(result.error.is_none(), "got: {result:?}");
        assert_eq!(result.iteration_count, 2);
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "Continue policy should not sleep; elapsed = {elapsed:?}"
        );
    }

    #[test]
    fn rate_limit_abort_policy_halts_with_structured_error() {
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 5".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: Some(OnRateLimit::Abort),
        };

        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| {
                Ok(LoopIterationOutput::success("ok")
                    .with_rate_limit(Some(throttled(Some("usage cap"), Some(60))))
                    .with_attribution(Some("k2p6".into()), Some("kimi-for-coding".into())))
            },
        )
        .unwrap();

        assert_eq!(result.iteration_count, 1);
        match result.error {
            Some(CompositionError::LoopRateLimited {
                iteration,
                provider,
                model,
                reset_at,
                message,
                ..
            }) => {
                assert_eq!(iteration, 1);
                assert_eq!(provider.as_deref(), Some("k2p6"));
                assert_eq!(model.as_deref(), Some("kimi-for-coding"));
                assert!(reset_at.is_some());
                assert_eq!(message.as_deref(), Some("usage cap"));
            }
            other => panic!("expected LoopRateLimited, got {other:?}"),
        }
    }

    #[test]
    fn rate_limit_pause_with_no_reset_falls_back_to_abort() {
        // No `reset_at` → Pause cannot wait an unbounded amount, so we
        // abort cleanly with the same structured error.
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 5".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: Some(OnRateLimit::Pause),
        };

        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| {
                Ok(LoopIterationOutput::success("ok")
                    .with_rate_limit(Some(throttled(Some("no reset clock"), None))))
            },
        )
        .unwrap();

        assert_eq!(result.iteration_count, 1);
        assert!(
            matches!(
                result.error,
                Some(CompositionError::LoopRateLimited { reset_at: None, .. })
            ),
            "got: {:?}",
            result.error
        );
    }

    #[test]
    fn rate_limit_pause_skipped_on_final_iteration() {
        // When the loop is already going to exit (is_last == true), the
        // engine must not pause — it would block for nothing.
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 1".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: Some(OnRateLimit::Pause),
        };

        let start = std::time::Instant::now();
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| {
                // Iteration 1 IS the last (counter goes 0 → 1, condition fails next round).
                Ok(LoopIterationOutput::success("ok")
                    .with_rate_limit(Some(throttled(Some("trailer on last"), Some(300)))))
            },
        )
        .unwrap();
        let elapsed = start.elapsed();

        assert!(result.error.is_none(), "got: {result:?}");
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "should skip pause on last iteration; elapsed = {elapsed:?}"
        );
    }

    #[test]
    fn rate_limit_default_policy_is_pause() {
        // Neither options nor config set on_rate_limit. With no reset_at,
        // the default Pause falls back to Abort.
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 5".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };

        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions::default(),
            |_ctx| {
                Ok(LoopIterationOutput::success("ok").with_rate_limit(Some(throttled(None, None))))
            },
        )
        .unwrap();

        assert!(
            matches!(result.error, Some(CompositionError::LoopRateLimited { .. })),
            "default should be Pause→Abort fallback; got: {:?}",
            result.error
        );
    }

    #[test]
    fn rate_limit_pause_sleeps_until_reset_then_continues() {
        // With Pause policy the engine must sleep until `reset_at` (plus the
        // safety margin) before running the next iteration. We inject a zero
        // margin and a 1s reset so the test verifies the wait-then-continue
        // behaviour without burning the production 5s margin.
        let config = LoopConfig {
            condition: LoopCondition::While("counter < 2".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: Some(OnRateLimit::Pause),
        };

        let start = std::time::Instant::now();
        let result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions {
                pause_reset_margin: Some(std::time::Duration::ZERO),
                ..LoopExecutionOptions::default()
            },
            |ctx| {
                let rl = if ctx.iteration == 1 {
                    // 1s reset + 0 margin → the engine pauses ~1s before
                    // proceeding to iteration 2.
                    Some(throttled(Some("brief cap"), Some(1)))
                } else {
                    None
                };
                Ok(LoopIterationOutput::success("ok").with_rate_limit(rl))
            },
        )
        .unwrap();
        let elapsed = start.elapsed();

        assert!(result.error.is_none(), "got: {result:?}");
        assert_eq!(result.iteration_count, 2);
        // 1s reset + 0 margin → it must have waited, but not unbounded.
        assert!(
            elapsed >= std::time::Duration::from_millis(500),
            "expected ~1s pause; elapsed = {elapsed:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(10),
            "pause should not be unbounded; elapsed = {elapsed:?}"
        );
    }

    #[test]
    fn rate_limit_pause_is_interrupt_aware() {
        // When the interrupt_check callback returns true, the pause exits
        // immediately and the engine returns Proceed (caller will see the
        // interrupt on the next iteration via its wrapped executor).
        use std::sync::atomic::{AtomicBool, Ordering};

        // Static flag because LoopExecutionOptions.interrupt_check is a
        // bare `fn() -> bool` (Copy).
        static FIRED: AtomicBool = AtomicBool::new(false);
        FIRED.store(true, Ordering::SeqCst);
        fn always_interrupted() -> bool {
            FIRED.load(Ordering::SeqCst)
        }

        let config = LoopConfig {
            condition: LoopCondition::While("counter < 2".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: Some(OnRateLimit::Pause),
        };

        let start = std::time::Instant::now();
        let _result = execute_loop_with_config(
            Path::new("loop.md"),
            &config,
            object(json!({"counter": 0})),
            LoopExecutionOptions {
                interrupt_check: Some(always_interrupted),
                ..LoopExecutionOptions::default()
            },
            |ctx| {
                let rl = if ctx.iteration == 1 {
                    // Long reset to prove the interrupt cut it short.
                    Some(throttled(Some("long cap"), Some(60)))
                } else {
                    None
                };
                Ok(LoopIterationOutput::success("ok").with_rate_limit(rl))
            },
        )
        .unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "interrupt should cut pause short; elapsed = {elapsed:?}"
        );
        FIRED.store(false, Ordering::SeqCst);
    }
}
