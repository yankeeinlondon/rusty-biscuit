//! Loop execution orchestration.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use tracing::info;

use super::error::CompositionError;
use super::lifecycle::{LifecycleConfig, LifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal};
use super::lifecycle_context::LifecycleErrorInfo;
use super::lifecycle_control::{MAX_PROXY_HOPS, proxy_handoff_allowed, resolve_proxy_target};
use super::lifecycle_executor::{ShellRunner, StackControl, StackExecutionContext};
use super::loop_actions::ActionStaging;
use super::loop_config::{extract_control_variables, resolve_loop_config};
use super::loop_expression::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
use super::prepare::{PrepareOptions, prepare_direct, prepare_inline};
use super::types::{CompositionMode, LoopConfig, OnRateLimit, ResolvedCompositionSource};
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
            init_proxy_target: None,
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

/// Build the initial frontmatter for a loop from resolved control variables.
///
/// Runs one compose pass to resolve the document, then lifts only:
/// - CLI `set_overrides` keys, carried verbatim so the body sees them every
///   iteration;
/// - control variables (action targets, condition identifiers, and identifiers
///   referenced by action-value templates), resolved from
///   `effective_frontmatter`.
///
/// Derived/presentation frontmatter keys are intentionally omitted so they
/// re-resolve each iteration against current state and ambients.
///
/// `mode` selects the seed compose pass so seeding matches the iteration
/// executor:
/// - [`CompositionMode::ChainedDocument`] composes the document body (as
///   `compose` does); a doc with an empty body fails seed resolution with
///   [`CompositionError::ComposedBodyEmpty`].
/// - [`CompositionMode::InlineFrontmatterPrompt`] composes the frontmatter
///   `prompt` value as the body (as `inline-compose` does); a doc whose
///   prompt lives in frontmatter resolves even when the body is empty.
///   Without this mode split, an inline-compose doc with an empty body
///   would fail seed resolution before iteration 1 even though the
///   iteration executor composes the `prompt:` frontmatter value.
///
/// ## Errors
///
/// Returns `CompositionError` when the seed compose pass fails.
pub fn build_loop_seed(
    source: &ResolvedCompositionSource,
    config: &LoopConfig,
    prepare_options: PrepareOptions,
    mode: CompositionMode,
) -> Result<Map<String, Value>, CompositionError> {
    Ok(build_loop_seed_with_lifecycle(source, config, prepare_options, mode)?.seed)
}

/// The seed frontmatter for a loop plus the **full** parsed lifecycle config.
///
/// [`build_loop_seed`] lifts only iteration-control variables into the seed,
/// dropping every lifecycle event block (`initialize`/`start`/`success`/
/// `blocked`/`failure`/`finalize` and the `loop:` gate's concerns). The loop
/// runner needs those blocks to fire lifecycle events, so this struct carries
/// the lifecycle config parsed from the document's full composed frontmatter
/// alongside the control-variable seed.
#[derive(Debug)]
pub struct LoopSeed {
    /// Iteration-control seed frontmatter (control variables + CLI setters).
    pub seed: Map<String, Value>,
    /// Lifecycle config parsed from the **full** composed effective
    /// frontmatter — carries every event block, unlike [`Self::seed`].
    pub lifecycle: super::lifecycle::LifecycleConfig,
}

/// Build the loop seed and parse the lifecycle config from the full composed
/// frontmatter.
///
/// This runs the same single compose pass as [`build_loop_seed`] but returns
/// the document's complete lifecycle config (parsed from
/// `prepared.effective_frontmatter`, which contains the lifecycle event
/// blocks) in addition to the control-variable-only seed. The loop runner
/// uses the lifecycle config so loop iterations fire `initialize`/`start`/
/// terminal/`finalize` and the `loop:` gate concerns — the seed alone would
/// parse to an empty lifecycle config because the control-variable lift drops
/// every event block.
///
/// ## Errors
///
/// Returns `CompositionError` when the compose pass fails.
pub fn build_loop_seed_with_lifecycle(
    source: &ResolvedCompositionSource,
    config: &LoopConfig,
    prepare_options: PrepareOptions,
    mode: CompositionMode,
) -> Result<LoopSeed, CompositionError> {
    let prepared = match mode {
        CompositionMode::ChainedDocument => prepare_direct(source, prepare_options.clone())?,
        CompositionMode::InlineFrontmatterPrompt => {
            prepare_inline(source, prepare_options.clone())?
        }
    };
    let effective = &prepared.effective_frontmatter;
    let control_vars = extract_control_variables(config);

    let mut seed = Map::new();

    if let Some(Value::Object(set_overrides)) = &prepare_options.set_overrides {
        for (key, value) in set_overrides {
            seed.insert(key.clone(), value.clone());
        }
    }

    for name in control_vars {
        if let Some(value) = effective.get(&name) {
            seed.insert(name, value.clone());
        }
    }

    Ok(LoopSeed {
        seed,
        lifecycle: prepared.lifecycle,
    })
}

/// Execute a loop defined on a resolved composition source.
///
/// Returns `Ok(None)` when the source has no `loop` frontmatter.
///
/// `prepare_options` is used to build the loop seed: one compose pass is run
/// before iteration 1 so control variables hold resolved, typed values. CLI
/// `key=value` setters in `prepare_options.set_overrides` are preserved in
/// the seed.
///
/// `mode` selects the seed compose pass and is forwarded to
/// [`build_loop_seed`]. It must match the composition mode of the caller
/// (`ChainedDocument` for `compose`, `InlineFrontmatterPrompt` for
/// `inline-compose`) so seeding and iteration 1 resolve from the same body.
///
/// ## Errors
///
/// Returns parse/evaluation errors that prevent the engine from determining
/// loop control flow. Per-iteration prompt/action failures are represented in
/// [`LoopExecutionResult::error`] according to fail-fast semantics.
pub fn execute_loop(
    source: &ResolvedCompositionSource,
    options: LoopExecutionOptions,
    prepare_options: PrepareOptions,
    mode: CompositionMode,
    executor: impl FnMut(LoopIterationContext) -> Result<LoopIterationOutput, CompositionError>,
) -> Result<Option<LoopExecutionResult>, CompositionError> {
    let Some(config) = resolve_loop_config(source)? else {
        return Ok(None);
    };
    let initial_frontmatter = build_loop_seed(source, &config, prepare_options, mode)?;
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

/// Execute a loop with integrated lifecycle events.
///
/// This is the Phase 6 loop-gate driver. It emits `initialize` exactly once
/// before the first iteration, delegates `start`/terminal/`finalize` emission
/// to the provided executor, and runs the post-`finalize` loop gate in the
/// required order:
///
/// 1. Loop lifecycle concerns (against pre-mutation frontmatter).
/// 2. Evaluate `while`/`until` condition (against pre-mutation frontmatter).
/// 3. Apply per-iteration mutations only when continuing.
///
/// Loop concerns run on every gate pass, including the terminal pass that
/// exits. Under `fail_fast: true`, iterations ending in `blocked` or `failure`
/// emit `finalize` through the executor and then exit before the loop gate.
/// Under `fail_fast: false`, failed iterations reach the loop gate.
#[allow(clippy::too_many_arguments)]
pub fn execute_loop_with_lifecycle<E>(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    options: LoopExecutionOptions,
    lifecycle_config: &LifecycleConfig,
    lifecycle_ctx: &LifecycleRuntimeContext<'_>,
    effect_engine: &darkmatter::effects::EffectEngine,
    shell_runner: &dyn ShellRunner,
    emitter: &dyn LifecycleEmitter,
    mut executor: E,
) -> Result<LoopExecutionResult, CompositionError>
where
    E: FnMut(
        LoopIterationContext,
        &mut LifecycleRunGuard<'_>,
    ) -> Result<LoopIterationOutput, CompositionError>,
{
    let max_iterations = options
        .max_iterations
        .or(config.max_iterations)
        .unwrap_or(DEFAULT_MAX_ITERATIONS);
    let fail_fast = options.fail_fast.or(config.fail_fast).unwrap_or(true);
    let on_rate_limit = options
        .on_rate_limit
        .or(config.on_rate_limit)
        .unwrap_or_default();

    let base_dir = prompt_path.parent();

    // Run-level wall-clock anchor for lifecycle `timing.*` across all
    // iterations and the loop gate.
    let loop_start = std::time::Instant::now();

    let mut guard = LifecycleRunGuard::new(lifecycle_config, lifecycle_ctx, emitter);

    // Emit initialize once before any iteration runs.
    let (init_timing, init_current) = capture_loop_lifecycle_globals(base_dir, loop_start);
    let init_ctx = build_loop_stack_context(
        LifecycleSignal::Initialize,
        &initial_frontmatter,
        lifecycle_ctx,
        effect_engine,
        shell_runner,
        emitter,
        base_dir,
        Some(&init_timing),
        Some(&init_current),
    );
    let init_outcome = guard.execute_event(LifecycleSignal::Initialize, &init_ctx);

    // `initialize` is the only event that can re-route the whole run via a
    // lifecycle control action. Mirror the non-loop path
    // (`wrap/composition/mod.rs::execute_composition_request_inner_with_guard`):
    // `Skip` ends the run cleanly with no further events, `Error` routes to
    // `failure`/`finalize` and returns a typed error, `Proxy` resolves the
    // target and asks the caller to hand off, and `Stop` falls through to the
    // iteration loop. `Retry`/`Resume`/`Requeue` are rejected at parse time, so
    // they are defensive fall-throughs here.
    if let Some(control) = init_outcome.control.clone() {
        match control {
            StackControl::Skip => {
                info!(
                    source_path = %prompt_path.display(),
                    "lifecycle `initialize` skip: ending run before any iteration"
                );
                return Ok(LoopExecutionResult::success(
                    initial_frontmatter,
                    0,
                    String::new(),
                    0,
                ));
            }
            StackControl::Error { reason } => {
                let msg = reason
                    .clone()
                    .unwrap_or_else(|| "lifecycle initialize error".to_string());
                return Ok(route_init_failure(
                    &mut guard,
                    &init_ctx,
                    prompt_path,
                    msg,
                ));
            }
            StackControl::Proxy { target } => {
                let resolved = match resolve_proxy_target(
                    &target,
                    prompt_path,
                    lifecycle_ctx.repo_root,
                ) {
                    Ok(path) => path,
                    Err(message) => {
                        // Resolution failure (missing file, unresolvable
                        // `@repo/…` reference) is reported as an initialize
                        // failure so the user sees the underlying cause.
                        return Ok(route_init_failure(
                            &mut guard,
                            &init_ctx,
                            prompt_path,
                            format!("proxy target `{target}` could not be resolved: {message}"),
                        ));
                    }
                };
                if !proxy_handoff_allowed(&[prompt_path.to_path_buf()], &resolved) {
                    return Ok(LoopExecutionResult::failure(
                        initial_frontmatter,
                        0,
                        String::new(),
                        0,
                        CompositionError::LifecycleProxyCycle {
                            source_path: prompt_path.to_path_buf(),
                            target: target.clone(),
                            chain: vec![prompt_path.display().to_string()],
                            limit: MAX_PROXY_HOPS,
                        },
                    ));
                }
                // No Failure/Finalize/loop-gate events fire on a clean
                // hand-off: the document is being replaced, not failed. The
                // caller re-enters with the target, whose own `initialize`
                // decides what happens next.
                return Ok(LoopExecutionResult::success(
                    initial_frontmatter,
                    0,
                    String::new(),
                    0,
                )
                .with_init_proxy_target(resolved));
            }
            StackControl::Resume { .. } => {
                // Pre-launch: no provider session to resume.
                return Ok(LoopExecutionResult::failure(
                    initial_frontmatter,
                    0,
                    String::new(),
                    0,
                    CompositionError::LifecycleResumeWithoutSession {
                        source_path: prompt_path.to_path_buf(),
                    },
                ));
            }
            StackControl::Retry { .. } => {
                return Ok(LoopExecutionResult::failure(
                    initial_frontmatter,
                    0,
                    String::new(),
                    0,
                    CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                        source_path: prompt_path.to_path_buf(),
                        event: "initialize".to_string(),
                        action: "retry".to_string(),
                    },
                ));
            }
            StackControl::Requeue { .. } => {
                return Ok(LoopExecutionResult::failure(
                    initial_frontmatter,
                    0,
                    String::new(),
                    0,
                    CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                        source_path: prompt_path.to_path_buf(),
                        event: "initialize".to_string(),
                        action: "requeue".to_string(),
                    },
                ));
            }
            StackControl::Stop => {
                // `Stop` only ends the initialize stack; the run continues
                // into the iteration loop with the outcome unchanged.
            }
        }
    }

    if init_outcome.routes_to_failure(LifecycleSignal::Initialize) {
        let reason = init_outcome
            .action_error
            .as_ref()
            .map(|e| e.msg.clone())
            .unwrap_or_else(|| "lifecycle initialize failed".to_string());
        return Ok(route_init_failure(
            &mut guard,
            &init_ctx,
            prompt_path,
            reason,
        ));
    }

    let mut frontmatter = initial_frontmatter;
    let mut iteration_count = 0usize;
    let mut last_output = String::new();
    let mut last_exit_code = 0i32;

    for iteration in 1..=max_iterations {
        // Under post-finalize checking the current frontmatter is the
        // pre-mutation state for this iteration. `_loop_is_last` should be
        // true exactly when the loop will exit after this iteration, which
        // is when the condition is falsy against the current frontmatter.
        let is_last = if iteration == max_iterations {
            true
        } else {
            let pre_mutation_ambient = LoopAmbient::new(
                iteration,
                iteration == 1,
                false,
                last_output.clone(),
                last_exit_code,
            );
            let pre_mutation_lookup =
                LoopExpressionLookup::new(&frontmatter, &pre_mutation_ambient)
                    .with_base_dir(base_dir);
            !evaluate_condition(&config.condition, &pre_mutation_lookup)?
        };
        let ambient = LoopAmbient::new(
            iteration,
            iteration == 1,
            is_last,
            last_output.clone(),
            last_exit_code,
        );

        let context = LoopIterationContext {
            iteration,
            frontmatter: frontmatter.clone(),
            ambient,
        };

        // The executor is responsible for emitting start, the terminal event,
        // and finalize through the shared guard.
        let output = match executor(context, &mut guard) {
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
                guard.reset_for_next_iteration();
                continue;
            }
        };

        last_output = output.output;
        last_exit_code = output.exit_code;
        iteration_count += 1;
        let iteration_rate_limit = output.rate_limit.clone();
        let iteration_provider = output.provider_id.clone();
        let iteration_model = output.model_id.clone();
        let exit_reason = output.exit_reason.clone();

        let terminal_signal = output
            .terminal_signal
            .or_else(|| if output.error.is_some() || output.exit_code != 0 {
                Some(LifecycleSignal::Failure)
            } else {
                Some(LifecycleSignal::Success)
            });
        let terminal_failed = matches!(
            terminal_signal,
            Some(LifecycleSignal::Blocked) | Some(LifecycleSignal::Failure)
        );

        if terminal_failed && fail_fast {
            return Ok(LoopExecutionResult::failure(
                frontmatter,
                iteration_count,
                last_output,
                last_exit_code,
                output.error.unwrap_or_else(|| {
                    CompositionError::LoopIterationFailed {
                        iteration,
                        prompt_path: prompt_path.to_path_buf(),
                        exit_code: last_exit_code,
                        reason: "iteration failed".to_string(),
                        exit_reason,
                    }
                }),
            ));
        }

        // Rate-limit policy is consulted between iterations, before the loop gate.
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
                RateLimitOutcome::Interrupted => {}
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

        // Post-finalize loop gate: concerns → condition → mutations.
        let gate_ambient = LoopAmbient::new(
            iteration,
            iteration == 1,
            is_last,
            last_output.clone(),
            last_exit_code,
        );
    match run_loop_gate(
        config,
        prompt_path,
        &frontmatter,
        &gate_ambient,
        base_dir,
        &mut guard,
        lifecycle_ctx,
        effect_engine,
        shell_runner,
        emitter,
        loop_start,
    )? {
            LoopGateOutcome::Exit => {
                return Ok(LoopExecutionResult::success(
                    frontmatter,
                    iteration_count,
                    last_output,
                    last_exit_code,
                ));
            }
            LoopGateOutcome::Continue(next_frontmatter) => {
                frontmatter = next_frontmatter;
            }
            LoopGateOutcome::Fail(error) => {
                return Ok(LoopExecutionResult::failure(
                    frontmatter,
                    iteration_count,
                    last_output,
                    last_exit_code,
                    error,
                ));
            }
        }

        guard.reset_for_next_iteration();

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

/// Route an `initialize` failure (explicit `error(...)` or unintentional
/// action error) through `failure` + `finalize` and return the typed loop
/// failure.
///
/// Mirrors the non-loop path at
/// `wrap/composition/mod.rs::execute_composition_request_inner_with_guard`:
/// `failure` runs against the action-error context, `finalize` against the
/// same context re-pointed at `Finalize`. The returned result reports zero
/// iterations because no provider invocation ran.
fn route_init_failure(
    guard: &mut LifecycleRunGuard<'_>,
    init_ctx: &StackExecutionContext<'_>,
    prompt_path: &Path,
    reason: String,
) -> LoopExecutionResult {
    let action_error = LifecycleErrorInfo::from_action_failure("error", reason.clone());
    guard.execute_event(
        LifecycleSignal::Failure,
        &init_ctx.with_error(&action_error),
    );
    guard.execute_event(
        LifecycleSignal::Finalize,
        &init_ctx
            .with_error(&action_error)
            .with_signal(LifecycleSignal::Finalize),
    );
    // The returned result reports the initialize-time frontmatter. We clone it
    // from `init_ctx` (which borrows the caller's `initial_frontmatter`) rather
    // than take ownership: the caller still holds `init_ctx` across this call,
    // so moving the frontmatter in would conflict with that live borrow.
    LoopExecutionResult::failure(
        init_ctx.frontmatter.clone(),
        0,
        String::new(),
        0,
        CompositionError::LifecycleInitializeFailed {
            source_path: prompt_path.to_path_buf(),
            reason,
        },
    )
}

/// Outcome of the post-finalize loop gate.
enum LoopGateOutcome {
    /// The condition is satisfied; continue with the mutated frontmatter.
    Continue(Map<String, Value>),
    /// The condition stopped the loop; do not apply mutations.
    Exit,
    /// The gate stack raised an explicit `error(...)`: convert the loop's
    /// final outcome to failure and exit. The condition is **not** evaluated
    /// and mutations are **not** applied — the error takes precedence over the
    /// condition, even on a non-final pass.
    Fail(CompositionError),
}

/// Run the post-finalize loop gate.
///
/// Executes loop lifecycle concerns against pre-mutation frontmatter, then
/// evaluates the `while`/`until` condition, then applies mutations only if
/// the loop should continue.
///
/// An explicit `error(...)` lifecycle action in the gate stack surfaces as
/// [`StackControl::Error`] and converts the final outcome to failure
/// ([`LoopGateOutcome::Fail`]), short-circuiting before the condition and
/// mutations. An *unintentional* action error does **not** invert the outcome:
/// `loop` is a terminal-phase event, so `routes_action_error_to_failure`
/// (consulted via [`LifecycleEventOutcome::routes_to_failure`]) is false for it
/// and the gate proceeds to the condition with the outcome unchanged.
#[allow(clippy::too_many_arguments)]
fn run_loop_gate(
    config: &LoopConfig,
    prompt_path: &Path,
    frontmatter: &Map<String, Value>,
    ambient: &LoopAmbient,
    base_dir: Option<&Path>,
    guard: &mut LifecycleRunGuard<'_>,
    lifecycle_ctx: &LifecycleRuntimeContext<'_>,
    effect_engine: &darkmatter::effects::EffectEngine,
    shell_runner: &dyn ShellRunner,
    emitter: &dyn LifecycleEmitter,
    loop_start: std::time::Instant,
) -> Result<LoopGateOutcome, CompositionError> {
    let (timing, current) = capture_loop_lifecycle_globals(base_dir, loop_start);
    let loop_ctx = build_loop_stack_context(
        LifecycleSignal::Loop,
        frontmatter,
        lifecycle_ctx,
        effect_engine,
        shell_runner,
        emitter,
        base_dir,
        Some(&timing),
        Some(&current),
    );
    let loop_outcome = guard.execute_event(LifecycleSignal::Loop, &loop_ctx);

    // An explicit `error(...)` in the gate stack converts the loop's final
    // outcome to failure and exits — before the condition is evaluated and
    // before any mutation is applied. Only the explicit `Error` lifecycle
    // action does this; an unintentional action error leaves the outcome
    // unchanged because `loop` is a terminal-phase event
    // (`routes_to_failure(Loop)` is always false).
    if let Some(StackControl::Error { reason }) = &loop_outcome.control {
        let reason = reason
            .clone()
            .unwrap_or_else(|| "lifecycle loop gate error".to_string());
        return Ok(LoopGateOutcome::Fail(
            CompositionError::LifecycleLoopGateFailed {
                source_path: prompt_path.to_path_buf(),
                reason,
            },
        ));
    }
    // The loop gate runs in the loop engine, which has no provider re-entry,
    // hand-off, or deferred-queue machinery, so a recovery control here is a
    // clear failure rather than a silent drop. (Recovery from a completed
    // iteration belongs in `failure`/`finalize`/`success`.) `Stop` and absence
    // fall through to the normal condition evaluation.
    let deferred_action = match &loop_outcome.control {
        Some(StackControl::Retry { .. }) => Some("retry"),
        Some(StackControl::Resume { .. }) => Some("resume"),
        Some(StackControl::Requeue { .. }) => Some("requeue"),
        Some(StackControl::Proxy { .. }) => Some("proxy"),
        _ => None,
    };
    if let Some(action) = deferred_action {
        return Ok(LoopGateOutcome::Fail(
            CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                source_path: prompt_path.to_path_buf(),
                event: "loop".to_string(),
                action: action.to_string(),
            },
        ));
    }

    let lookup = LoopExpressionLookup::new(frontmatter, ambient).with_base_dir(base_dir);
    if !evaluate_condition(&config.condition, &lookup)? {
        return Ok(LoopGateOutcome::Exit);
    }

    let next_frontmatter = apply_actions(config, frontmatter, ambient.iteration, Some(&lookup))?;
    Ok(LoopGateOutcome::Continue(next_frontmatter))
}

/// Build a stack execution context for loop lifecycle events.
///
/// `timing` and `current` are the lifecycle stack-only globals. The caller owns
/// them (captured fresh per event so they outlive this borrowed context).
#[allow(clippy::too_many_arguments)]
fn build_loop_stack_context<'a>(
    signal: LifecycleSignal,
    frontmatter: &'a Map<String, Value>,
    lifecycle_ctx: &'a LifecycleRuntimeContext<'a>,
    effect_engine: &'a darkmatter::effects::EffectEngine,
    shell_runner: &'a dyn ShellRunner,
    emitter: &'a dyn LifecycleEmitter,
    base_dir: Option<&'a Path>,
    timing: Option<&'a super::lifecycle_context::LifecycleTiming>,
    current: Option<&'a super::lifecycle_context::LifecycleCurrent>,
) -> StackExecutionContext<'a> {
    StackExecutionContext {
        signal,
        frontmatter,
        err: None,
        timing,
        current,
        base_dir,
        effect_engine,
        shell_runner,
        emitter,
        term: lifecycle_ctx.term,
        source_path: lifecycle_ctx.source_path,
        repo_root: lifecycle_ctx.repo_root,
        messaging: lifecycle_ctx.messaging,
        settings: lifecycle_ctx.settings,
    }
}

/// Capture the lifecycle stack-only `timing`/`current` globals for a loop
/// event.
///
/// `current.env`/`current.ctx` are captured **now** so a side effect or
/// external change since `prepare` (or since a prior iteration) is observable
/// through `current.*`. `timing` measures wall-clock elapsed against
/// `loop_start` (`document_ms` and `total_ms`; `step_ms` stays `None` outside a
/// sequence).
fn capture_loop_lifecycle_globals(
    base_dir: Option<&Path>,
    loop_start: std::time::Instant,
) -> (
    super::lifecycle_context::LifecycleTiming,
    super::lifecycle_context::LifecycleCurrent,
) {
    let current = match base_dir {
        Some(dir) => super::lifecycle_context::LifecycleCurrent::capture_at_event(dir),
        None => super::lifecycle_context::LifecycleCurrent::capture_env_only(),
    };
    let timing = super::lifecycle_context::LifecycleTiming::from_instants(
        loop_start,
        Some(loop_start),
        std::time::Instant::now(),
    );
    (timing, current)
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

    use darkmatter::markdown::{Frontmatter, Markdown};
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::composition::types::{LoopAction, LoopCondition};

    /// The loop-engine wiring captures non-empty `timing`/`current` globals so
    /// loop lifecycle events (`initialize`, `loop`) expose `timing.document_ms`
    /// and a populated `current.env`, rather than the pre-fix `None`/`None`.
    #[test]
    fn capture_loop_lifecycle_globals_populates_timing_and_env() {
        let loop_start = std::time::Instant::now();
        let (timing, current) = capture_loop_lifecycle_globals(Some(Path::new(".")), loop_start);

        assert!(
            timing.document_ms.is_some(),
            "document_ms is populated from the run-level instant"
        );
        assert!(
            timing.total_ms.is_some(),
            "total_ms is populated because a run_start instant is supplied"
        );
        assert!(
            current.env.is_object() && !current.env.as_object().unwrap().is_empty(),
            "current.env is a non-empty process-environment snapshot"
        );
        // base_dir = "." → ctx is captured (at minimum ctx.today).
        assert!(
            current.ctx.get("today").is_some(),
            "current.ctx snapshot carries today"
        );
    }

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

    fn make_source(frontmatter: &[(&str, serde_json::Value)]) -> ResolvedCompositionSource {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("loop.md");
        let mut fm = darkmatter::markdown::Frontmatter::new();
        for (key, value) in frontmatter {
            fm.insert(key, value.clone()).unwrap();
        }
        let md = darkmatter::markdown::Markdown::with_frontmatter(fm, "Body");
        std::fs::write(&file, md.as_string()).unwrap();
        let original_text = std::fs::read_to_string(&file).unwrap();
        let markdown: darkmatter::markdown::Markdown = original_text.clone().into();
        ResolvedCompositionSource {
            original_ref: file.to_string_lossy().to_string(),
            resolved_path: file,
            original_text,
            markdown,
        }
    }

    fn make_source_with_body(
        frontmatter: &[(&str, serde_json::Value)],
        body: &str,
    ) -> ResolvedCompositionSource {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("loop.md");
        let mut fm = Frontmatter::new();
        for (key, value) in frontmatter {
            fm.insert(key, value.clone()).unwrap();
        }
        let md = Markdown::with_frontmatter(fm, body);
        std::fs::write(&file, md.as_string()).unwrap();
        let original_text = std::fs::read_to_string(&file).unwrap();
        let markdown: Markdown = original_text.clone().into();
        ResolvedCompositionSource {
            original_ref: file.to_string_lossy().to_string(),
            resolved_path: file,
            original_text,
            markdown,
        }
    }

    #[test]
    fn build_loop_seed_resolves_control_variables_and_omits_derived() {
        let source = make_source(&[
            ("phase", json!("{{ initial_phase || 1 }}")),
            ("total_phases", json!("{{ 6 }}")),
            (
                "pass_icon",
                json!("{{ _loop_is_last ? '✅' : '🧑‍💻' }}"),
            ),
            (
                "loop",
                json!({"until": "phase > total_phases", "action": "increment(phase)"}),
            ),
        ]);
        let config = resolve_loop_config(&source).unwrap().unwrap();
        let options = PrepareOptions {
            set_overrides: Some(json!({"initial_phase": 1})),
            ..PrepareOptions::default()
        };

        let seed =
            build_loop_seed(&source, &config, options, CompositionMode::ChainedDocument).unwrap();

        assert_eq!(seed.get("phase"), Some(&json!(1)));
        assert_eq!(seed.get("total_phases"), Some(&json!(6)));
        assert!(!seed.contains_key("pass_icon"), "derived keys must not be lifted into the seed");
        assert_eq!(seed.get("initial_phase"), Some(&json!(1)));
    }

    /// The control-variable-only seed drops every lifecycle event block, so
    /// parsing lifecycle from it yields an empty config — the root cause of the
    /// loop-path lifecycle bug. `build_loop_seed_with_lifecycle` instead parses
    /// lifecycle from the full composed frontmatter, so the event blocks and
    /// the `loop:` gate concerns survive even though they are absent from the
    /// seed.
    #[test]
    fn build_loop_seed_with_lifecycle_carries_event_blocks_dropped_from_seed() {
        let source = make_source(&[
            ("phase", json!(1)),
            (
                "loop",
                json!({
                    "until": "phase > 2",
                    "action": "increment(phase)",
                    "stack": [{"action": "append_line('events.log', 'gate')"}],
                }),
            ),
            ("initialize", json!({"stack": [{"action": "append_line('events.log', 'initialize')"}]})),
            ("start", json!({"stack": [{"action": "append_line('events.log', 'start')"}]})),
            ("finalize", json!({"stack": [{"action": "append_line('events.log', 'finalize')"}]})),
        ]);
        let config = resolve_loop_config(&source).unwrap().unwrap();

        let result = build_loop_seed_with_lifecycle(
            &source,
            &config,
            PrepareOptions::default(),
            CompositionMode::ChainedDocument,
        )
        .unwrap();

        // The seed itself carries only control variables, never event blocks.
        assert!(!result.seed.contains_key("initialize"));
        assert!(!result.seed.contains_key("finalize"));

        // The parsed lifecycle, however, carries every event block plus the
        // `loop:` gate concerns — exactly what the loop runner needs.
        assert!(!result.lifecycle.is_empty(), "lifecycle must not be empty");
        assert!(result.lifecycle.stacks.initialize.is_some());
        assert!(result.lifecycle.stacks.start.is_some());
        assert!(result.lifecycle.stacks.finalize.is_some());
        assert!(
            result.lifecycle.stacks.loop_gate.is_some(),
            "the loop gate's lifecycle concerns must survive into the parsed config"
        );
    }

    #[test]
    fn build_loop_seed_inline_mode_resolves_prompt_frontmatter_with_empty_body() {
        let source = make_source_with_body(
            &[
                ("prompt", json!("Build phase {{phase}}")),
                ("phase", json!("{{ start || 1 }}")),
                (
                    "loop",
                    json!({"while": "phase < 2", "action": "increment(phase)"}),
                ),
            ],
            "",
        );
        let config = resolve_loop_config(&source).unwrap().unwrap();

        // Inline mode composes the `prompt:` frontmatter value as the body,
        // so an empty document body still resolves and the control variable
        // `phase` lifts into the seed.
        let seed = build_loop_seed(
            &source,
            &config,
            PrepareOptions::default(),
            CompositionMode::InlineFrontmatterPrompt,
        )
        .expect("inline seed should resolve from prompt frontmatter with empty body");
        assert_eq!(seed.get("phase"), Some(&json!(1)));

        // Direct mode composes the document body itself, which is empty, so
        // seeding fails before iteration 1 with `ComposedBodyEmpty`. This
        // locks in the mode distinction that motivates parameterizing
        // `build_loop_seed` by `CompositionMode`.
        let direct = build_loop_seed(
            &source,
            &config,
            PrepareOptions::default(),
            CompositionMode::ChainedDocument,
        );
        assert!(
            matches!(direct, Err(CompositionError::ComposedBodyEmpty { .. })),
            "direct mode with empty body should fail seed resolution; got {direct:?}"
        );
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

    // ── Seeded-loop integration tests ────────────────────────────────────

    #[test]
    fn seeded_loop_repro_runs_to_completion_with_live_derived_variable() {
        let source = make_source_with_body(
            &[
                ("phase", json!("{{ start || 1 }}")),
                ("total_phases", json!(6)),
                (
                    "pass_icon",
                    json!("{{ _loop_is_last ? '✅' : '🧑‍💻' }}"),
                ),
                (
                    "loop",
                    json!({"until": "phase > total_phases", "action": "increment(phase)"}),
                ),
            ],
            "Implement Phase {{ phase }} of {{ total_phases }}",
        );
        let config = resolve_loop_config(&source).unwrap().unwrap();
        let seed = build_loop_seed(
            &source,
            &config,
            PrepareOptions::default(),
            CompositionMode::ChainedDocument,
        )
        .unwrap();

        let captured = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            &source.resolved_path,
            &config,
            seed,
            LoopExecutionOptions::default(),
            |ctx| {
                let prepared = prepare_direct(
                    &source,
                    PrepareOptions {
                        set_overrides: Some(ctx.as_set_overrides()),
                        ..PrepareOptions::default()
                    },
                )?;
                let pass_icon = prepared
                    .effective_frontmatter
                    .as_object()
                    .and_then(|fm| fm.get("pass_icon"))
                    .cloned();
                let body = prepared.prompt.clone();
                captured.borrow_mut().push((
                    ctx.iteration,
                    ctx.frontmatter.get("phase").cloned(),
                    body,
                    pass_icon,
                ));
                Ok(LoopIterationOutput::success(prepared.prompt))
            },
        )
        .unwrap();

        assert!(result.error.is_none(), "expected clean run, got {result:?}");
        assert_eq!(result.iteration_count, 6);
        assert_eq!(result.final_frontmatter.get("phase"), Some(&json!(7)));

        let seen = captured.into_inner();
        assert_eq!(seen.len(), 6);
        for (index, (iteration, phase, body, pass_icon)) in seen.iter().enumerate() {
            let n = index + 1;
            assert_eq!(*iteration, n);
            assert_eq!(*phase, Some(json!(n)));
            assert_eq!(body.trim(), format!("Implement Phase {n} of 6"));
            let expected_icon = if n == 6 { "✅" } else { "🧑‍💻" };
            assert_eq!(
                pass_icon.as_ref().and_then(|v| v.as_str()),
                Some(expected_icon),
                "pass_icon on iteration {n} should be {expected_icon}"
            );
        }
    }

    #[test]
    fn seeded_loop_reports_honest_error_for_non_numeric_control_variable() {
        let source = make_source_with_body(
            &[
                ("area", json!("claudine")),
                (
                    "loop",
                    json!({"while": "true", "action": "increment(area)"}),
                ),
            ],
            "work",
        );
        let config = resolve_loop_config(&source).unwrap().unwrap();
        let seed = build_loop_seed(
            &source,
            &config,
            PrepareOptions::default(),
            CompositionMode::ChainedDocument,
        )
        .unwrap();

        let result = execute_loop_with_config(
            &source.resolved_path,
            &config,
            seed,
            LoopExecutionOptions::default(),
            |_ctx| Ok(LoopIterationOutput::success("ok")),
        )
        .unwrap();

        assert_eq!(result.iteration_count, 1);
        match result.error {
            Some(CompositionError::InvalidIncrementType {
                property,
                found,
                value_excerpt,
                ..
            }) => {
                assert_eq!(property, "area");
                assert_eq!(found, "string");
                assert!(
                    value_excerpt.contains("claudine"),
                    "excerpt should quote the value: {value_excerpt}"
                );
            }
            other => panic!("expected InvalidIncrementType, got {other:?}"),
        }
    }

    #[test]
    fn seeded_loop_doc_namespace_condition_retains_readonly_control_value() {
        let source = make_source_with_body(
            &[
                ("counter", json!(0)),
                ("total", json!(2)),
                (
                    "loop",
                    json!({"while": "doc.counter < doc.total", "action": "increment(counter)"}),
                ),
            ],
            "Step {{ counter }} of {{ total }}",
        );
        let config = resolve_loop_config(&source).unwrap().unwrap();
        let seed = build_loop_seed(
            &source,
            &config,
            PrepareOptions::default(),
            CompositionMode::ChainedDocument,
        )
        .unwrap();

        assert_eq!(seed.get("counter"), Some(&json!(0)));
        assert_eq!(seed.get("total"), Some(&json!(2)));

        let captured = RefCell::new(Vec::new());
        let result = execute_loop_with_config(
            &source.resolved_path,
            &config,
            seed,
            LoopExecutionOptions::default(),
            |ctx| {
                let prepared = prepare_direct(
                    &source,
                    PrepareOptions {
                        set_overrides: Some(ctx.as_set_overrides()),
                        ..PrepareOptions::default()
                    },
                )?;
                let body = prepared.prompt.clone();
                captured.borrow_mut().push((ctx.iteration, body));
                Ok(LoopIterationOutput::success(prepared.prompt))
            },
        )
        .unwrap();

        assert!(result.error.is_none(), "expected clean run, got {result:?}");
        assert_eq!(result.iteration_count, 2);
        assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(2)));

        let seen = captured.into_inner();
        assert_eq!(seen.len(), 2);
        for (index, (iteration, body)) in seen.iter().enumerate() {
            let n = index + 1;
            assert_eq!(*iteration, n);
            // Iteration N uses the counter value BEFORE the increment fires
            // at the end of the iteration (counter 0→1→2), so the rendered
            // body shows the starting counter for that pass.
            assert_eq!(body.trim(), format!("Step {} of 2", n - 1));
        }
    }

    // ── Loop-path `initialize` lifecycle-control tests ───────────────────
    //
    // These exercise `execute_loop_with_lifecycle` directly (the
    // `compose --loop` driver) to prove that the `initialize` event's
    // returned `LifecycleEventOutcome` is honored for all four controls
    // — the gap Finding 2 reported. The non-loop path
    // (`wrap/composition/mod.rs`) handles the same controls; these lock the
    // loop path to identical behavior so there is no mode divergence.

    use std::sync::Mutex;

    use crate::composition::lifecycle::{
        LifecycleConfig, LifecycleEmitter, LifecycleRuntimeContext, parse_lifecycle_config,
    };

    /// Test emitter that records every emitted lifecycle signal so a test can
    /// assert which events fired (and, crucially, which did *not*).
    #[derive(Default)]
    struct SignalRecorder {
        signals: Mutex<Vec<LifecycleSignal>>,
    }

    impl SignalRecorder {
        fn signals(&self) -> Vec<LifecycleSignal> {
            self.signals.lock().unwrap().clone()
        }
    }

    impl LifecycleEmitter for SignalRecorder {
        fn emit_stderr(
            &self,
            signal: LifecycleSignal,
            _text: &str,
            _term: &biscuit_terminal::terminal::Terminal,
        ) {
            self.signals.lock().unwrap().push(signal);
        }
        fn emit_message(
            &self,
            _text: &str,
            _source_path: &Path,
            _repo_root: Option<&Path>,
            _messaging: &crate::messaging::RuntimeMessagingSettings,
        ) {
        }
        fn emit_speech(&self, _text: &str, _tts_config: biscuit_speaks::TtsConfig) {}
        fn emit_effect(&self, _name: &str) {}
        fn emit_notification(&self, _title: &str) {}
    }

    fn lifecycle_from(json: serde_json::Value) -> LifecycleConfig {
        parse_lifecycle_config(&json, Path::new("loop.md")).unwrap()
    }

    /// Drive `execute_loop_with_lifecycle` against a parsed lifecycle config,
    /// counting executor (iteration) invocations. The executor always succeeds
    /// so any iteration that runs is unambiguously the engine's decision, not a
    /// terminal-signal artifact.
    fn run_loop_lifecycle(
        prompt_path: &Path,
        config: &LoopConfig,
        initial_frontmatter: Map<String, Value>,
        lifecycle: &LifecycleConfig,
        emitter: &dyn LifecycleEmitter,
        invocations: &RefCell<usize>,
    ) -> LoopExecutionResult {
        let settings = crate::events::GlobalSettings::default();
        let messaging = crate::messaging::RuntimeMessagingSettings {
            user: None,
            repo: None,
        };
        let term = biscuit_terminal::terminal::Terminal::default();
        let lifecycle_ctx = LifecycleRuntimeContext {
            settings: &settings,
            messaging: &messaging,
            term: &term,
            source_path: prompt_path,
            repo_root: prompt_path.parent(),
        };
        let effect_engine = darkmatter::effects::EffectEngine::builder()
            .mutation_root(prompt_path.parent().unwrap_or(Path::new(".")))
            .auto_rehash(false)
            .build();
        execute_loop_with_lifecycle(
            prompt_path,
            config,
            initial_frontmatter,
            LoopExecutionOptions::default(),
            lifecycle,
            &lifecycle_ctx,
            &effect_engine,
            &crate::composition::lifecycle_executor::SystemShellRunner,
            emitter,
            |_ctx, _guard| {
                *invocations.borrow_mut() += 1;
                Ok(LoopIterationOutput::success("ran"))
            },
        )
        .unwrap()
    }

    /// `skip` at `initialize` in a looping document ends the run immediately:
    /// zero iterations, no executor invocation, no terminal/`finalize`/`loop`
    /// events — the whole-document opt-out the spec requires (spec.md:338).
    #[test]
    fn loop_initialize_skip_ends_run_with_zero_iterations() {
        let config = counter_loop(3);
        let lifecycle = lifecycle_from(json!({
            "initialize": { "stack": [{ "action": "skip" }] },
            "finalize": { "stack": [{ "action": "append_line('never.log', 'finalize')" }] },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);

        let result = run_loop_lifecycle(
            Path::new("loop.md"),
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );

        assert!(result.error.is_none(), "skip is a clean opt-out: {result:?}");
        assert_eq!(result.iteration_count, 0, "no iteration runs after skip");
        assert_eq!(*invocations.borrow(), 0, "the executor must never be invoked");
        assert!(
            result.init_proxy_target.is_none(),
            "skip is not a proxy hand-off"
        );
        // Only `initialize` may have emitted (the stack control fires before any
        // terminal handling); no terminal/finalize/loop signal escaped.
        let signals = emitter.signals();
        assert!(
            !signals.contains(&LifecycleSignal::Finalize),
            "skip must not run finalize; got {signals:?}"
        );
        assert!(
            !signals.contains(&LifecycleSignal::Success)
                && !signals.contains(&LifecycleSignal::Failure),
            "skip emits no terminal signal; got {signals:?}"
        );
    }

    /// `error(...)` at `initialize` routes the run to `failure` then `finalize`
    /// and terminates the loop with a typed `LifecycleInitializeFailed` — no
    /// iteration runs (initialize fires once, before iterations). Mirrors the
    /// non-loop path (spec.md:607).
    #[test]
    fn loop_initialize_error_routes_to_failure_and_finalize() {
        let config = counter_loop(3);
        let lifecycle = lifecycle_from(json!({
            "initialize": { "stack": [{ "action": "error('preflight refused')" }] },
            "failure": { "stderr": "fail" },
            "finalize": { "stderr": "final" },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);

        let result = run_loop_lifecycle(
            Path::new("loop.md"),
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );

        assert_eq!(*invocations.borrow(), 0, "no iteration runs after an init error");
        assert_eq!(result.iteration_count, 0);
        match &result.error {
            Some(CompositionError::LifecycleInitializeFailed { reason, .. }) => {
                assert!(
                    reason.contains("preflight refused"),
                    "the authored reason must survive: {reason}"
                );
            }
            other => panic!("expected LifecycleInitializeFailed, got {other:?}"),
        }
        let signals = emitter.signals();
        assert!(
            signals.contains(&LifecycleSignal::Failure),
            "init error routes to failure; got {signals:?}"
        );
        assert!(
            signals.contains(&LifecycleSignal::Finalize),
            "init error then runs finalize; got {signals:?}"
        );
    }

    /// `stop` at `initialize` ends only the initialize stack; the run proceeds
    /// into the iteration loop unchanged (spec.md:337). Proven by parity: the
    /// iteration count with a `stop` init control equals the count from an
    /// otherwise-identical document whose initialize stack is benign (an
    /// `info(...)` that never re-routes), so `stop` is confirmed to leave the
    /// loop untouched without hard-coding the engine's iteration arithmetic.
    #[test]
    fn loop_initialize_stop_proceeds_into_iterations() {
        let run = |action: &str| {
            let config = counter_loop(3);
            let lifecycle = lifecycle_from(json!({
                "initialize": { "stack": [{ "action": action }] },
            }));
            let emitter = SignalRecorder::default();
            let invocations = RefCell::new(0usize);
            let result = run_loop_lifecycle(
                Path::new("loop.md"),
                &config,
                object(json!({ "counter": 0 })),
                &lifecycle,
                &emitter,
                &invocations,
            );
            (result, invocations.into_inner())
        };

        let (stop_result, stop_invocations) = run("stop");
        let (baseline_result, baseline_invocations) = run("info('init ran')");

        assert!(stop_result.error.is_none(), "stop is benign: {stop_result:?}");
        assert!(stop_invocations > 0, "the loop must run after a benign stop");
        assert_eq!(
            stop_result.iteration_count, baseline_result.iteration_count,
            "stop must not change how many iterations run"
        );
        assert_eq!(
            stop_invocations, baseline_invocations,
            "stop must not change executor invocations vs. a benign init stack"
        );
        assert_eq!(stop_result.iteration_count, stop_invocations);
    }

    /// `proxy(...)` at `initialize` resolves the target and hands off without
    /// running any iteration, terminal, `finalize`, or `loop` event — the
    /// caller re-enters with the target's own `initialize` (spec.md:340,607).
    #[test]
    fn loop_initialize_proxy_hands_off_without_iterating() {
        let dir = TempDir::new().unwrap();
        let prompt = dir.path().join("loop.md");
        std::fs::write(&prompt, "---\n---\nbody").unwrap();
        let target = dir.path().join("target.md");
        std::fs::write(&target, "---\n---\nbody").unwrap();

        let config = counter_loop(3);
        let lifecycle = lifecycle_from(json!({
            "initialize": { "stack": [{ "action": "proxy('target.md')" }] },
            "finalize": { "stderr": "final" },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);

        let result = run_loop_lifecycle(
            &prompt,
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );

        assert!(result.error.is_none(), "clean proxy hand-off: {result:?}");
        assert_eq!(result.iteration_count, 0, "no iteration runs on a proxy hand-off");
        assert_eq!(*invocations.borrow(), 0);
        assert_eq!(
            result.init_proxy_target.as_deref(),
            Some(target.as_path()),
            "the resolved target is surfaced for the caller to re-enter"
        );
        let signals = emitter.signals();
        assert!(
            !signals.contains(&LifecycleSignal::Finalize)
                && !signals.contains(&LifecycleSignal::Failure)
                && !signals.contains(&LifecycleSignal::Success),
            "a clean hand-off fires no terminal/finalize/loop events; got {signals:?}"
        );
    }

    /// A `proxy(...)` target that cannot be resolved (missing file) is reported
    /// as an initialize failure (routed through failure + finalize), matching
    /// the non-loop path's behavior rather than silently iterating.
    #[test]
    fn loop_initialize_proxy_unresolvable_routes_to_failure() {
        let dir = TempDir::new().unwrap();
        let prompt = dir.path().join("loop.md");
        std::fs::write(&prompt, "---\n---\nbody").unwrap();

        let config = counter_loop(3);
        let lifecycle = lifecycle_from(json!({
            "initialize": { "stack": [{ "action": "proxy('does-not-exist.md')" }] },
            "finalize": { "stderr": "final" },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);

        let result = run_loop_lifecycle(
            &prompt,
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );

        assert_eq!(*invocations.borrow(), 0, "no iteration runs on a failed proxy");
        assert!(result.init_proxy_target.is_none());
        assert!(
            matches!(
                result.error,
                Some(CompositionError::LifecycleInitializeFailed { .. })
            ),
            "an unresolvable proxy target is an initialize failure; got {:?}",
            result.error
        );
    }

    // ── Loop-gate `error(...)` tests (Finding 3) ─────────────────────────
    //
    // The `loop:` gate is a terminal-phase event, so only an *explicit*
    // `error(...)` lifecycle action converts the loop's final outcome to
    // failure and exits the loop. An *unintentional* action error there must
    // leave the outcome unchanged (`routes_to_failure(Loop)` is always false).

    /// An explicit `error(...)` in the `loop:` gate stack converts the loop's
    /// final outcome to failure and exits — even though the `until` condition
    /// would otherwise continue iterating. The error takes precedence over the
    /// condition, so the gate's mutation (`increment(counter)`) is NOT applied
    /// and no further iteration runs (spec.md:334-341, "convert final outcome
    /// to failure and exit the loop").
    #[test]
    fn loop_gate_explicit_error_fails_and_exits_without_mutation() {
        // `until: counter > 5` with `counter` starting at 0 would continue
        // looping, so an exit here can only come from the gate's `error(...)`.
        let config = LoopConfig {
            condition: LoopCondition::Until("counter > 5".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let lifecycle = lifecycle_from(json!({
            "loop": { "stack": [{ "action": "error('gate rejected final state')" }] },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);

        let result = run_loop_lifecycle(
            Path::new("loop.md"),
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );

        match &result.error {
            Some(CompositionError::LifecycleLoopGateFailed { reason, .. }) => {
                assert!(
                    reason.contains("gate rejected final state"),
                    "the authored reason must survive: {reason}"
                );
            }
            other => panic!("expected LifecycleLoopGateFailed, got {other:?}"),
        }
        assert_eq!(
            *invocations.borrow(),
            1,
            "exactly one iteration ran before the gate failed the loop"
        );
        assert_eq!(result.iteration_count, 1);
        assert_eq!(
            result.final_frontmatter.get("counter"),
            Some(&json!(0)),
            "the gate mutation must NOT be applied when the gate raises an error"
        );
    }

    /// An *unintentional* action error in the `loop:` gate stack (a `shell`
    /// command that exits non-zero) must NOT invert the outcome: `loop` is a
    /// terminal-phase event, so the gate proceeds to the condition and the loop
    /// finishes successfully once the condition stops it. Contrast with the
    /// explicit-`error` test above.
    #[test]
    fn loop_gate_unintentional_error_does_not_invert_outcome() {
        // `until: counter > 1` with `counter` starting at 0 and an
        // `increment(counter)` gate mutation: the gate evaluates its condition
        // against the pre-mutation state, so the loop runs two iterations
        // (counter 0→1→2) before the condition stops it. The `shell('false')`
        // gate action errors on every pass but, being unintentional at a
        // terminal-phase event, never aborts the loop.
        let config = LoopConfig {
            condition: LoopCondition::Until("counter > 1".into()),
            actions: vec![LoopAction::Increment("counter".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let lifecycle = lifecycle_from(json!({
            "loop": { "stack": [{ "action": "shell('false')" }] },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);

        let result = run_loop_lifecycle(
            Path::new("loop.md"),
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );

        assert!(
            result.error.is_none(),
            "an unintentional gate action error must not fail the loop: {result:?}"
        );
        assert_eq!(
            result.final_frontmatter.get("counter"),
            Some(&json!(2)),
            "the loop ran to completion: the gate mutation applied on each \
             continuing pass despite the unintentional action error"
        );
    }
}
