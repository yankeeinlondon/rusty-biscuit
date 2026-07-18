//! Loop execution orchestration.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use tracing::info;

use super::super::error::CompositionError;
use super::super::lifecycle::{LifecycleConfig, LifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal};
use super::super::lifecycle_context::LifecycleErrorInfo;
use super::super::lifecycle_control::{MAX_PROXY_HOPS, proxy_handoff_allowed, resolve_proxy_target};
use super::super::lifecycle_executor::{ShellRunner, StackControl, StackExecutionContext};
use super::super::lifecycle::runtime::{
    LifecycleCatchExecution, LifecycleCatchProtocol, LifecycleCatchResult, LifecycleCatchState,
};
use super::super::prepare::PrepareOptions;
use super::super::types::{CompositionMode, LoopConfig, OnRateLimit, ResolvedCompositionSource};
use super::actions::ActionStaging;
use super::config::resolve_loop_config;
use super::expression::{LoopAmbient, LoopExpressionLookup, evaluate_condition};
use super::seed::build_loop_seed;
use super::types::{
    LoopExecutionOptions, LoopExecutionResult, LoopIterationContext, LoopIterationOutput,
};
use crate::stream::summary::RateLimitInfo;

/// Default safety cap for prompt loops.
pub const DEFAULT_MAX_ITERATIONS: usize = 100;

/// How long the engine sleeps between interrupt-flag checks while pausing
/// for a rate-limit reset.
///
/// Short enough to be responsive to Ctrl+C without burning CPU on the poll.
const PAUSE_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// Safety margin added on top of a provider's `reset_at` to absorb skew —
/// providers commonly return `429` for a moment after the nominal reset.
const PAUSE_RESET_MARGIN: std::time::Duration = std::time::Duration::from_secs(5);

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
            None,
        )?;
        let ambient = LoopAmbient::new(
            iteration,
            iteration == 1,
            is_last,
            last_output.clone(),
            last_exit_code,
        );
        let lookup = LoopExpressionLookup::new(&frontmatter, &ambient)
            .with_base_dir(base_dir)
            .with_file_ref_fallback_dir(None);
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
        let post_lookup = LoopExpressionLookup::new(&frontmatter, &post_ambient)
            .with_base_dir(base_dir)
            .with_file_ref_fallback_dir(None);
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
                None,
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
    let (init_timing, init_current) =
        capture_loop_lifecycle_globals(base_dir, lifecycle_ctx.launch_area, loop_start);
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
    let init_result = execute_loop_catch_protocol(
        &mut guard,
        &init_ctx,
        LifecycleSignal::Initialize,
        None,
        init_outcome.clone(),
    );
    if let Some(info) = init_result.evaluation_error.as_ref() {
        let surfaced_signal = init_result
            .evaluation_error_signal
            .expect("initialize evaluation error remains terminal");
        return Ok(LoopExecutionResult::failure(
            initial_frontmatter,
            0,
            String::new(),
            0,
            CompositionError::lifecycle_evaluation(
                surfaced_signal.property_name(),
                prompt_path,
                info,
            ),
        ));
    }
    if let Some(setup_error) = init_result.setup_error.as_ref() {
        return Ok(LoopExecutionResult::failure(
            initial_frontmatter,
            0,
            String::new(),
            0,
            CompositionError::LifecycleInitializeFailed {
                source_path: prompt_path.to_path_buf(),
                reason: setup_error.msg.clone(),
            },
        ));
    }

    // `initialize` is the only event that can re-route the whole run via a
    // lifecycle control action. Mirror the non-loop path
    // (`wrap/composition/mod.rs::execute_composition_request_inner_with_guard`):
    // `Skip` ends the run cleanly with no further events, `Error` routes to
    // `failure`/`finalize` and returns a typed error, `Proxy` resolves the
    // target and asks the caller to hand off, and `Stop` falls through to the
    // iteration loop. `Retry`/`Resume`/`Defer` are rejected at parse time, so
    // they are defensive fall-throughs here.
    if let Some(control) = init_result.control.clone() {
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
                unreachable!("the catch protocol consumes initialize error control: {reason:?}");
            }
            StackControl::Proxy { target } => {
                let resolved = match resolve_proxy_target(
                    &target,
                    prompt_path,
                    lifecycle_ctx.repo_root,
                ) {
                    Ok(path) => path,
                    Err(err) => {
                        // Resolution failure (missing file, unresolvable
                        // `@repo/…` reference) is reported as an initialize
                        // failure so the user sees the underlying cause. The
                        // typed `HarnessError`'s Diagnostic facets are threaded
                        // through so `err.code` / `err.detail.*` reach a
                        // `failure`/`finalize` stack.
                        let reason =
                            format!("proxy target `{target}` could not be resolved: {err}");
                        return Ok(route_init_failure_typed(
                            &mut guard,
                            &init_ctx,
                            prompt_path,
                            &init_outcome,
                            LifecycleErrorInfo::from_harness_error(&err),
                            reason,
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
            StackControl::Defer { .. } => {
                return Ok(LoopExecutionResult::failure(
                    initial_frontmatter,
                    0,
                    String::new(),
                    0,
                    CompositionError::LifecycleDeferNotImplemented {
                        source_path: prompt_path.to_path_buf(),
                    },
                ));
            }
            StackControl::Stop => {
                // `Stop` only ends the initialize stack; the run continues
                // into the iteration loop with the outcome unchanged.
            }
        }
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
                    .with_base_dir(base_dir)
                    .with_file_ref_fallback_dir(lifecycle_ctx.launch_area);
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
                lifecycle_ctx.launch_area,
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

/// Route an `initialize` failure built from an already-typed error snapshot,
/// preserving the source error's `Diagnostic` facets on the `err` global.
///
/// Used where the initialize-phase failure carries a typed cause (e.g. a
/// `Proxy` target that fails to resolve via [`crate::harness::HarnessError`]):
/// threading [`LifecycleErrorInfo::from_harness_error`] through here keeps
/// `err.code` / `err.detail.*` projecting for a `failure`/`finalize` stack
/// instead of flattening to a bare message. `fallback_reason` populates the
/// terminal [`CompositionError::LifecycleInitializeFailed`] when neither catch
/// event raised.
fn route_init_failure_typed(
    guard: &mut LifecycleRunGuard<'_>,
    init_ctx: &StackExecutionContext<'_>,
    prompt_path: &Path,
    init_outcome: &super::super::lifecycle_executor::LifecycleEventOutcome,
    action_error: LifecycleErrorInfo,
    fallback_reason: String,
) -> LoopExecutionResult {
    let mut routed_outcome = init_outcome.clone();
    routed_outcome.action_error = Some(action_error.clone());
    let result = execute_loop_catch_protocol(
        guard,
        init_ctx,
        LifecycleSignal::Initialize,
        Some(&action_error),
        routed_outcome,
    );
    let error = if let (Some(signal), Some(info)) = (
        result.evaluation_error_signal,
        result.evaluation_error.as_ref(),
    ) {
        CompositionError::lifecycle_evaluation(
            signal.property_name(),
            prompt_path,
            info,
        )
    } else {
        CompositionError::LifecycleInitializeFailed {
            source_path: prompt_path.to_path_buf(),
            reason: fallback_reason,
        }
    };
    // The returned result reports the initialize-time frontmatter. We clone it
    // from `init_ctx` (which borrows the caller's `initial_frontmatter`) rather
    // than take ownership: the caller still holds `init_ctx` across this call,
    // so moving the frontmatter in would conflict with that live borrow.
    LoopExecutionResult::failure(init_ctx.frontmatter.clone(), 0, String::new(), 0, error)
}

fn execute_loop_catch_protocol(
    guard: &mut LifecycleRunGuard<'_>,
    origin_ctx: &StackExecutionContext<'_>,
    origin: LifecycleSignal,
    prior_error: Option<&LifecycleErrorInfo>,
    origin_outcome: super::super::lifecycle_executor::LifecycleEventOutcome,
) -> LifecycleCatchResult {
    let mut protocol = LifecycleCatchProtocol::new(
        origin,
        LifecycleCatchState {
            terminal_slot: guard.terminal_signal(),
            finalize_emitted: guard.finalize_emitted(),
        },
        prior_error.cloned(),
        origin_outcome,
    );
    while let Some(step) = protocol.next_step().cloned() {
        let event_ctx = origin_ctx.with_signal(step.signal);
        let event_ctx = match step.error.as_ref() {
            Some(error) => event_ctx.with_error(error),
            None => event_ctx,
        };
        let outcome = match step.execution {
            LifecycleCatchExecution::Record => guard.execute_event(step.signal, &event_ctx),
            LifecycleCatchExecution::RedesignateBlockedAsFailure => {
                guard.redesignate_terminal_to_failure();
                guard.run_event_stack(step.signal, &event_ctx)
            }
        };
        assert!(protocol.record(step.signal, outcome));
    }
    protocol.finish().expect("loop catch protocol completed")
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
    let (timing, current) =
        capture_loop_lifecycle_globals(base_dir, lifecycle_ctx.launch_area, loop_start);
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

    // A late-binding evaluation error in the gate stack (a crashed `when:`
    // guard or interpolation) halts the loop *before* the `while`/`until`
    // condition is evaluated and before any mutation is applied — the run
    // cannot trust a condition computed against a document whose gate just
    // raised. Unlike an unintentional dispatch failure, an evaluation error is
    // not tolerated on a terminal-phase event (Decision #3). `loop` is a
    // terminal-phase event, so — like `success`/`failure` — it does NOT
    // retroactively fire `failure` (the provider already ran); it fires
    // `finalize` exactly once carrying the error as the `err` global so a
    // `finalize.stack` can react, then surfaces the typed evaluation error
    // (precedence: a raise inside `finalize` beats the loop raise).
    if let Some(info) = loop_outcome.evaluation_error.as_ref() {
        let result = execute_loop_catch_protocol(
            guard,
            &loop_ctx,
            LifecycleSignal::Loop,
            Some(info),
            loop_outcome.clone(),
        );
        let surfaced_signal = result
            .evaluation_error_signal
            .expect("loop evaluation error remains terminal");
        let surfaced_info = result
            .evaluation_error
            .as_ref()
            .expect("evaluation signal carries error info");
        return Ok(LoopGateOutcome::Fail(
            CompositionError::lifecycle_evaluation(
                surfaced_signal.property_name(),
                prompt_path,
                surfaced_info,
            ),
        ));
    }

    let decision = LifecycleCatchProtocol::new(
        LifecycleSignal::Loop,
        LifecycleCatchState {
            terminal_slot: guard.terminal_signal(),
            finalize_emitted: guard.finalize_emitted(),
        },
        None,
        loop_outcome,
    )
    .finish()
    .expect("clean loop gate requires no catch event");

    // An explicit `error(...)` in the gate stack converts the loop's final
    // outcome to failure and exits — before the condition is evaluated and
    // before any mutation is applied. Only the explicit `Error` lifecycle
    // action does this; an unintentional action error leaves the outcome
    // unchanged because `loop` is a terminal-phase event
    // (`routes_to_failure(Loop)` is always false).
    if let Some(StackControl::Error { reason }) = &decision.control {
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
    if let Some(StackControl::Defer { .. }) = &decision.control {
        return Ok(LoopGateOutcome::Fail(
            CompositionError::LifecycleDeferNotImplemented {
                source_path: prompt_path.to_path_buf(),
            },
        ));
    }
    let deferred_action = match &decision.control {
        Some(StackControl::Retry { .. }) => Some("retry"),
        Some(StackControl::Resume { .. }) => Some("resume"),
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

    let lookup = LoopExpressionLookup::new(frontmatter, ambient)
        .with_base_dir(base_dir)
        .with_file_ref_fallback_dir(lifecycle_ctx.launch_area);
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
    timing: Option<&'a super::super::lifecycle_context::LifecycleTiming>,
    current: Option<&'a super::super::lifecycle_context::LifecycleCurrent>,
) -> StackExecutionContext<'a> {
    StackExecutionContext {
        signal,
        frontmatter,
        // The loop engine fires a single `loop` gate concern per iteration and
        // threads frontmatter across iterations via `apply_actions` /
        // `next_frontmatter`. There is no second lifecycle event within one gate
        // that would need to observe this gate's mutations, so the cross-event
        // live cell is unnecessary here; intra-stack visibility is handled by
        // `execute_stack`'s local working map.
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing,
        current,
        base_dir,
        ctx_base_dir: lifecycle_ctx.launch_area,
        prepared_context: lifecycle_ctx.context,
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
    ctx_base_dir: Option<&Path>,
    loop_start: std::time::Instant,
) -> (
    super::super::lifecycle_context::LifecycleTiming,
    super::super::lifecycle_context::LifecycleCurrent,
) {
    // `current.ctx.*` follows the launch area like the event-time `ctx.*`
    // capture; `current.env.*` is launch-area independent.
    let current = match ctx_base_dir.or(base_dir) {
        Some(dir) => super::super::lifecycle_context::LifecycleCurrent::capture_at_event(dir),
        None => super::super::lifecycle_context::LifecycleCurrent::capture_env_only(),
    };
    let timing = super::super::lifecycle_context::LifecycleTiming::from_instants(
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

#[allow(clippy::too_many_arguments)]
fn compute_is_last(
    prompt_path: &Path,
    config: &LoopConfig,
    frontmatter: &Map<String, Value>,
    iteration: usize,
    max_iterations: usize,
    last_output: &str,
    last_exit_code: i32,
    file_ref_fallback_dir: Option<&Path>,
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
    let speculative_lookup = LoopExpressionLookup::new(frontmatter, &speculative_ambient)
        .with_base_dir(base_dir)
        .with_file_ref_fallback_dir(file_ref_fallback_dir);
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
    let lookup = LoopExpressionLookup::new(&next_frontmatter, &next_ambient)
        .with_base_dir(base_dir)
        .with_file_ref_fallback_dir(file_ref_fallback_dir);
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
    file_ref_fallback_dir: Option<&Path>,
) -> Result<bool, CompositionError> {
    let ambient = LoopAmbient::new(next_iteration, false, true, last_output, last_exit_code);
    let lookup = LoopExpressionLookup::new(frontmatter, &ambient)
        .with_base_dir(base_dir)
        .with_file_ref_fallback_dir(file_ref_fallback_dir);
    evaluate_condition(&config.condition, &lookup)
}

#[cfg(test)]
mod tests;
