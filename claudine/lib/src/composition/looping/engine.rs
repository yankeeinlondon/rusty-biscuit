//! Loop execution orchestration.

use std::path::Path;

use serde_json::{Map, Value};
use tracing::info;

use super::super::coordinator::{EvaluatedProxyRequest, ProxyProvenance, SurfacedHandoff};
use super::super::error::CompositionError;
use super::super::lifecycle::{LifecycleConfig, LifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal};
use super::super::lifecycle_context::LifecycleErrorInfo;
use super::super::lifecycle_executor::{ShellRunner, StackControl, StackExecutionContext};
use super::super::lifecycle::runtime::{
    LifecycleCatchExecution, LifecycleCatchProtocol, LifecycleCatchResult, LifecycleCatchState,
};
use super::super::types::{LoopConfig, OnRateLimit};
use super::expression::{LoopAmbient, LoopExpressionLookup};
use super::frame::{LoopDecision, LoopFrame, LoopOutcome, LoopResultKind};
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

/// Execute a document loop with integrated lifecycle events.
///
/// This is the only document loop engine, and it is post-checked: every pass
/// runs before its condition is read, so a loop always runs at least once. The
/// pass state lives in a [`LoopFrame`] allocated before `initialize`, so
/// `initialize` already reads pass 1's `loop` values. The engine emits
/// `initialize` exactly once, delegates `start`/terminal/`finalize` emission to
/// the executor, and then, after each pass:
///
/// 1. runs the loop gate,
/// 2. reads `while`/`until` against current values and fresh file reads,
/// 3. classifies the cap only when the condition still asks for a pass, and
/// 4. stages the action only when another pass will run.
///
/// A failed pass is an unhandled failure: the loop ends with it before its
/// gate. Cancellation and handoffs also end the loop before the gate, the
/// condition, or the action.
///
/// `initialize_frontmatter` is the full bootstrap namespace, separate from
/// iteration-control state. Its live mutations remain visible to catch
/// handlers; `runtime_state` carries `set` writes into subsequent preparations.
#[allow(clippy::too_many_arguments)]
pub fn execute_loop_with_lifecycle<E>(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    initialize_frontmatter: &Map<String, Value>,
    runtime_state: Option<&crate::composition::RuntimeState>,
    options: LoopExecutionOptions,
    lifecycle_config: &LifecycleConfig,
    lifecycle_ctx: &LifecycleRuntimeContext<'_>,
    effect_engine: &darkmatter::effects::EffectEngine,
    shell_runner: &dyn ShellRunner,
    emitter: &dyn LifecycleEmitter,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    document_epoch: Option<&crate::invocation_context::DocumentEpoch>,
    mut executor: E,
) -> Result<LoopExecutionResult, CompositionError>
where
    E: FnMut(
        LoopIterationContext,
        &mut LifecycleRunGuard<'_>,
    ) -> Result<LoopIterationOutput, CompositionError>,
{
    if let Some(epoch) = document_epoch {
        epoch.record_prepared_context_consumer(
            crate::invocation_context::PreparedContextConsumer::LoopCondition,
        );
        if lifecycle_ctx.context.is_some() {
            epoch.record_prepared_context_consumer(
                crate::invocation_context::PreparedContextConsumer::Lifecycle,
            );
        }
    }
    let mut frame = LoopFrame::new(
        config,
        options.max_iterations,
        LoopResultKind::Document,
        prompt_path,
        "loop",
    )?;
    let on_rate_limit = options
        .on_rate_limit
        .or(config.on_rate_limit)
        .unwrap_or_default();

    let base_dir = prompt_path.parent();

    // One invocation-owned refresh capability for the whole loop: `current.*`
    // in any iteration observes the fact through the retained launch evidence.
    // Without an epoch there is no evidence, and every read fails closed.
    let current_authority = document_epoch.map(crate::invocation_context::DocumentEpoch::current_authority);

    // Run-level wall-clock anchor for lifecycle `timing.*` across all
    // iterations and the loop gate.
    let loop_start = std::time::Instant::now();

    let mut guard = LifecycleRunGuard::new(lifecycle_config, lifecycle_ctx, emitter);

    // Emit initialize once before any iteration runs. It is inside the loop:
    // it reads pass 1's values.
    let init_timing = capture_loop_lifecycle_timing(loop_start);
    let initialize_live = std::sync::Mutex::new(initialize_frontmatter.clone());
    let entry_ambient = frame.ambient();
    let mut init_ctx = build_loop_stack_context(
        LifecycleSignal::Initialize,
        initialize_frontmatter,
        lifecycle_ctx,
        effect_engine,
        shell_runner,
        emitter,
        base_dir,
        file_resolution_context,
        Some(&init_timing),
        current_authority.clone(),
        Some(&entry_ambient),
    );
    init_ctx.live_frontmatter = Some(&initialize_live);
    init_ctx.runtime_state = runtime_state;
    let init_outcome = guard.execute_event(LifecycleSignal::Initialize, &init_ctx);
    if guard.cancel_if_requested(&init_ctx) {
        return Ok(LoopExecutionResult::success(initial_frontmatter, 0, String::new(), 130));
    }
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
            StackControl::Proxy {
                target,
                overlay,
                location,
            } => {
                // Neither resolution nor the cycle/hop decision happens here.
                // The engine has no run ledger, so it can only see this one
                // document — which is why this call site used to pass a
                // single-element chain and miss multi-hop history entirely.
                // The coordinator commits the request against the
                // invocation-wide chain instead.
                //
                // No Failure/Finalize/loop-gate events fire on a clean
                // hand-off: the document is being replaced, not failed. The
                // target's own `initialize` decides what happens next.
                let request = EvaluatedProxyRequest::new(
                    target,
                    overlay,
                    ProxyProvenance::new(
                        prompt_path.to_path_buf(),
                        location,
                        vec![prompt_path.to_path_buf()],
                    ),
                );
                return Ok(LoopExecutionResult::success(
                    initial_frontmatter,
                    0,
                    String::new(),
                    0,
                )
                .with_handoff(SurfacedHandoff::Request(request)));
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
            // The loop's frame exists before `initialize`, so this `break`
            // ends this loop: no pass runs, nothing is composed or launched,
            // and `finalize` still runs once. Unlike `skip`, it is a
            // successful loop outcome.
            StackControl::Break { .. } => {
                let broken = frame.pending_break().unwrap_or_default();
                guard.defuse();
                let finalize = guard.execute_event(
                    LifecycleSignal::Finalize,
                    &init_ctx.with_signal(LifecycleSignal::Finalize),
                );
                // A failed cleanup beats the pending break.
                if let Some(info) = finalize.evaluation_error.as_ref().or(finalize.action_error.as_ref()) {
                    return Ok(LoopExecutionResult::failure(
                        initial_frontmatter,
                        0,
                        String::new(),
                        1,
                        CompositionError::lifecycle_evaluation("finalize", prompt_path, info),
                    ));
                }
                if let Some(StackControl::Error { reason }) = finalize.control {
                    return Ok(LoopExecutionResult::failure(
                        initial_frontmatter,
                        0,
                        String::new(),
                        1,
                        CompositionError::LifecycleInitializeFailed {
                            source_path: prompt_path.to_path_buf(),
                            reason: reason.unwrap_or_else(|| "lifecycle finalize error".to_string()),
                        },
                    ));
                }
                return Ok(LoopExecutionResult::success(initial_frontmatter, 0, String::new(), 0)
                    .with_break(broken));
            }
        }
    }

    let mut frontmatter = initial_frontmatter;
    let mut last_output = String::new();
    // The prepared snapshot of the most recent iteration's run; the loop's
    // starting snapshot until an iteration reports its own.
    let mut run_context: Option<darkmatter::markdown::compose::ComposeContext> = None;

    loop {
        let iteration = frame.count();
        let context = LoopIterationContext {
            iteration,
            frontmatter: frontmatter.clone(),
            ambient: frame.ambient(),
        };

        // The executor is responsible for emitting start, the terminal event,
        // and finalize through the shared guard.
        let mut output = match executor(context, &mut guard) {
            Ok(output) => output,
            Err(error) => {
                if guard.cancel_if_requested(&init_ctx) {
                    return Ok(LoopExecutionResult::success(frontmatter, iteration, last_output, 130));
                }
                // A pass that could not complete is an unhandled failure.
                return Ok(LoopExecutionResult::failure(
                    frontmatter,
                    iteration,
                    String::new(),
                    1,
                    error,
                ));
            }
        };

        if output.terminal_signal == Some(LifecycleSignal::Cancelled)
            || guard.cancel_if_requested(&init_ctx)
        {
            return Ok(LoopExecutionResult::success(frontmatter, iteration, output.output, 130));
        }

        // A proxy surfaced by the iteration's own lifecycle ends the source
        // document *now*: the target is not an extra iteration of this loop
        // (R7), so no loop gate runs and no further iteration begins. Checked
        // before failure handling on purpose — a proxy selected by a
        // `failure` stack is a clean handoff, not a failed iteration.
        if let Some(handoff) = output.handoff {
            let exit_code = output.exit_code;
            return Ok(LoopExecutionResult::success(
                frontmatter,
                iteration,
                output.output,
                exit_code,
            )
            .with_handoff(handoff));
        }

        if let Some(context) = output.context.take() {
            run_context = Some(context);
        }
        // The gate after this iteration belongs to this iteration's run.
        let iteration_lifecycle = LifecycleRuntimeContext {
            context: run_context.as_ref().or(lifecycle_ctx.context),
            ..*lifecycle_ctx
        };
        let lifecycle_ctx = &iteration_lifecycle;

        last_output = output.output;
        let last_exit_code = output.exit_code;
        frame.record_pass(Value::String(last_output.clone()), last_exit_code);
        let exit_reason = output.exit_reason.clone();

        let terminal_signal = output
            .terminal_signal
            .or_else(|| if output.error.is_some() || output.exit_code != 0 {
                Some(LifecycleSignal::Failure)
            } else {
                Some(LifecycleSignal::Success)
            });
        // A failed pass that its own `failure` event did not recover is an
        // unhandled failure: it ends the loop before the gate.
        if matches!(
            terminal_signal,
            Some(LifecycleSignal::Blocked) | Some(LifecycleSignal::Failure)
        ) {
            return Ok(LoopExecutionResult::failure(
                frontmatter,
                iteration,
                last_output,
                last_exit_code,
                output.error.unwrap_or_else(|| {
                    CompositionError::LoopIterationFailed {
                        iteration,
                        prompt_path: prompt_path.to_path_buf(),
                        exit_code: last_exit_code,
                        reason: "iteration failed".to_string(),
                        exit_reason,
                        // No iteration error was attached, so there is no
                        // chain to project from.
                        snapshot: None,
                    }
                }),
            ));
        }

        // A `break` in this pass ended it after its cleanup: the loop ends
        // without its gate, condition, or action.
        if let Some(broken) = frame.pending_break() {
            return Ok(LoopExecutionResult::success(frontmatter, iteration, last_output, last_exit_code)
                .with_break(broken));
        }

        // The gate and the condition read the pass's effective frontmatter
        // beneath the loop's control state; actions stage onto the control
        // state alone.
        let mut state = output.frontmatter.take().unwrap_or_default();
        state.extend(frontmatter.clone());
        let ambient = frame.ambient();
        let timing = capture_loop_lifecycle_timing(loop_start);
        let mut gate_ctx = build_loop_stack_context(
            LifecycleSignal::Loop,
            &state,
            lifecycle_ctx,
            effect_engine,
            shell_runner,
            emitter,
            base_dir,
            file_resolution_context,
            Some(&timing),
            current_authority.clone(),
            Some(&ambient),
        );
        gate_ctx.runtime_state = runtime_state;
        if let LoopGateOutcome::Fail(error) = run_loop_gate(&mut guard, &gate_ctx, prompt_path, iteration) {
            return Ok(LoopExecutionResult::failure(
                frontmatter,
                iteration,
                last_output,
                last_exit_code,
                error,
            ));
        }

        // A `break` in the gate ends the loop before its condition.
        if let Some(broken) = frame.pending_break() {
            return Ok(LoopExecutionResult::success(frontmatter, iteration, last_output, last_exit_code)
                .with_break(broken));
        }

        let lookup = LoopExpressionLookup::new(&state, &ambient, file_resolution_context, prompt_path)
            .with_file_ref_fallback_dir(lifecycle_ctx.launch_area)
            .with_prepared_context(lifecycle_ctx.context)
            .with_runtime_state(runtime_state);
        let decided = frame.decide(&lookup);
        let decision = match decided {
            Ok(decision) => decision,
            Err(error) => {
                return Ok(LoopExecutionResult::failure(frontmatter, iteration, last_output, last_exit_code, error));
            }
        };
        let next_frontmatter = match decision {
            LoopDecision::Complete => {
                return Ok(LoopExecutionResult::success(
                    frontmatter,
                    iteration,
                    last_output,
                    last_exit_code,
                )
                .with_outcome(LoopOutcome::ConditionMet));
            }
            LoopDecision::Exhausted(error) => {
                return Ok(LoopExecutionResult::failure(
                    frontmatter,
                    iteration,
                    last_output,
                    last_exit_code,
                    error,
                )
                .with_outcome(LoopOutcome::Exhausted));
            }
            LoopDecision::Continue => {
                // The rate-limit policy applies only once another pass is
                // certain to run, so it never stalls a loop that just ended.
                match decide_rate_limit_action(
                    output.rate_limit.as_ref(),
                    on_rate_limit,
                    prompt_path,
                    iteration,
                    output.provider_id.clone(),
                    output.model_id.clone(),
                    options.interrupt_check,
                    options.pause_reset_margin.unwrap_or(PAUSE_RESET_MARGIN),
                ) {
                    RateLimitOutcome::Proceed | RateLimitOutcome::Interrupted => {}
                    RateLimitOutcome::Abort(error) => {
                        return Ok(LoopExecutionResult::failure(
                            frontmatter,
                            iteration,
                            last_output,
                            last_exit_code,
                            error,
                        ));
                    }
                }
                match frame.stage_actions(&frontmatter, &lookup) {
                    Ok(next) => next,
                    Err(error) => {
                        return Ok(LoopExecutionResult::failure(frontmatter, iteration, last_output, last_exit_code, error));
                    }
                }
            }
        };
        frontmatter = next_frontmatter;
        guard.reset_for_next_iteration();
        frame.advance();
    }
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

/// Outcome of a loop gate.
pub(crate) enum LoopGateOutcome {
    /// The gate passed; read the condition.
    Proceed,
    /// The gate failed the loop owner. The condition is **not** read and no
    /// action is staged.
    Fail(CompositionError),
}

/// Run the loop gate that follows a finished pass, through `guard`.
///
/// `gate_ctx` is the owner's `loop` event context, carrying the `loop` values
/// of the pass that just finished. Every looping primitive shares this: a
/// document builds the context from its frontmatter, a container from its
/// lexical roots and runtime state.
///
/// An explicit `error(...)` in the gate fails the loop owner
/// ([`CompositionError::LifecycleLoopGateFailed`]), before the condition. An
/// evaluation error in the gate (a crashed `when:` or interpolation) fails it
/// too, as the typed evaluation error. An *unintentional* action error does
/// not: `loop` is a terminal-phase event, so the gate proceeds to the
/// condition. Recovery directives are refused here: a gate has no attempt to
/// re-enter and no document to replace.
pub(crate) fn run_loop_gate(
    guard: &mut LifecycleRunGuard<'_>,
    gate_ctx: &StackExecutionContext<'_>,
    source_path: &Path,
    iteration: usize,
) -> LoopGateOutcome {
    let loop_outcome = guard.execute_event(LifecycleSignal::Loop, gate_ctx);

    // A late-binding evaluation error in the gate stack halts the loop before
    // its condition: the run cannot trust a condition computed against state
    // whose gate just raised. The pass's own `finalize` already ran, so the
    // catch protocol fires nothing again; it selects the error to surface.
    if let Some(info) = loop_outcome.evaluation_error.as_ref() {
        let result = execute_loop_catch_protocol(
            guard,
            gate_ctx,
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
        return LoopGateOutcome::Fail(CompositionError::lifecycle_evaluation(
            surfaced_signal.property_name(),
            source_path,
            surfaced_info,
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

    match &decision.control {
        Some(StackControl::Error { reason }) => LoopGateOutcome::Fail(
            CompositionError::LifecycleLoopGateFailed {
                source_path: source_path.to_path_buf(),
                iteration,
                reason: reason
                    .clone()
                    .unwrap_or_else(|| "lifecycle loop gate error".to_string()),
            },
        ),
        Some(StackControl::Defer { .. }) => LoopGateOutcome::Fail(
            CompositionError::LifecycleDeferNotImplemented {
                source_path: source_path.to_path_buf(),
            },
        ),
        Some(control @ (StackControl::Retry { .. } | StackControl::Resume { .. } | StackControl::Proxy { .. })) => {
            let action = match control {
                StackControl::Retry { .. } => "retry",
                StackControl::Resume { .. } => "resume",
                _ => "proxy",
            };
            LoopGateOutcome::Fail(CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                source_path: source_path.to_path_buf(),
                event: "loop".to_string(),
                action: action.to_string(),
            })
        }
        _ => LoopGateOutcome::Proceed,
    }
}

/// Build a stack execution context for loop lifecycle events.
///
/// `timing` is the lifecycle stack-only global; the caller owns it (captured
/// fresh per event so it outlives this borrowed context). `current` is the
/// invocation's refresh authority for the lazy `current` root. `loop_values`
/// is the `loop` global: the gate's just-finished iteration, or `None` for
/// `initialize`, which runs before any iteration.
#[allow(clippy::too_many_arguments)]
fn build_loop_stack_context<'a>(
    signal: LifecycleSignal,
    frontmatter: &'a Map<String, Value>,
    lifecycle_ctx: &'a LifecycleRuntimeContext<'a>,
    effect_engine: &'a darkmatter::effects::EffectEngine,
    shell_runner: &'a dyn ShellRunner,
    emitter: &'a dyn LifecycleEmitter,
    base_dir: Option<&'a Path>,
    file_resolution_context: &'a biscuit_file::FileResolutionContext,
    timing: Option<&'a super::super::lifecycle_context::LifecycleTiming>,
    current: Option<darkmatter::markdown::compose::CurrentAuthority>,
    loop_values: Option<&'a LoopAmbient>,
) -> StackExecutionContext<'a> {
    StackExecutionContext {
        seq: None,
        signal,
        scope: None,
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
        // Each call builds one event's context: one `current` observation.
        current: current.as_ref().map(darkmatter::markdown::compose::CurrentAuthority::memoized),
        // A loop gate is not inside a sequence group; `group.*` has no scope
        // here.
        group: None,
        loop_values,
        base_dir,
        ctx_base_dir: lifecycle_ctx.launch_area,
        prepared_context: lifecycle_ctx.context,
        file_resolution_context,
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

/// Capture the lifecycle stack-only `timing` global for a loop event.
///
/// Measures wall-clock elapsed against `loop_start` (`document_ms` and
/// `total_ms`; `step_ms` stays `None` outside a sequence).
fn capture_loop_lifecycle_timing(
    loop_start: std::time::Instant,
) -> super::super::lifecycle_context::LifecycleTiming {
    super::super::lifecycle_context::LifecycleTiming::from_instants(
        loop_start,
        Some(loop_start),
        std::time::Instant::now(),
    )
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

#[cfg(test)]
mod tests;
