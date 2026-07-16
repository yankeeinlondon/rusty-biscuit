//! Composition pre-flight `blocked` / `finalize` lifecycle event helpers.
//!
//! [`emit_preflight_blocked_and_finalize`] routes a pre-flight failure through
//! the stack-aware event runner so a document's `blocked.stack` and
//! `finalize.stack` side effects fire (the legacy top-level-only emitter was
//! retired). [`PreflightBlockedOutcome`] carries either the blocked stack's
//! flow-control action or a typed evaluation error raised by the stack itself.

use std::path::Path;

use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::{
    LifecycleEmitter, LifecycleRunGuard, LifecycleSignal,
};
use claudine::composition::lifecycle_executor::{LifecycleEventOutcome, StackControl};
use claudine::composition::{
    CompositionError, LifecycleCatchExecution, LifecycleCatchProtocol, LifecycleCatchState,
    LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming,
    LifecycleTransitionAbort, LifecycleTransitionDecision, LifecycleTransitionInput,
    decide_lifecycle_transition,
};
use claudine::composition::lifecycle_executor::StackExecutionContext;
use claudine::composition::lifecycle_executor::SystemShellRunner;
use claudine::events::GlobalSettings;
use claudine::messaging::RuntimeMessagingSettings;
use darkmatter::effects::EffectEngine;

/// Outcome of running the pre-flight `blocked` + `finalize` lifecycle events.
///
/// `Control` carries the blocked stack's flow-control action (if any) for the
/// caller to dispatch. `EvaluationError` carries a typed
/// [`CompositionError::LifecycleEvaluationError`] raised by the `blocked` or
/// `finalize` stack itself — it takes precedence over the original pre-flight
/// failure because a lifecycle expression crash is the actionable cause.
#[derive(Debug)]
pub(super) enum PreflightBlockedOutcome {
    /// No evaluation error; the blocked stack's flow-control action (if any).
    Control(Option<StackControl>),
    /// A late-binding evaluation error raised in `blocked` (routed through
    /// failure + finalize with the evaluation error as `err`) or in `finalize`
    /// (surfaced without re-entering finalize).
    EvaluationError(CompositionError),
}

/// Run the `blocked` and `finalize` lifecycle events (top-level
/// communication **and** typed stack) for a composition preflight failure.
///
/// The spec requires a blocked iteration to reach `blocked` then `finalize`,
/// each firing both its top-level communication surface and its typed stack
/// (`spec.md:436`, `spec.md:650`, `spec.md:652`). Pre-flight failures
/// (harness-plan parse, shell-approval denial, dry-run pre-check) used to
/// call [`LifecycleRunGuard::emit_blocked_or_failure`], which only fires the
/// legacy top-level subset (`stderr`/`message`/`notify`/audio) and skips both
/// the typed stacks and `finalize`. That left documents relying on
/// `blocked.stack` / `finalize.stack` side effects (e.g.
/// `{append_line: ["events.log", "blocked"]}`) without either marker.
///
/// This helper builds [`StackExecutionContext`] from the same local lifecycle
/// bindings used by `initialize` (see `init_ctx` in the composition executor):
/// the context borrows the *local* `emitter`/`settings`/etc. — not the guard — so
/// [`LifecycleRunGuard::execute_event`] can take `&mut guard` without a
/// borrow conflict. `execute_event` records the emission and runs the
/// top-level + stack in one call, and sets `terminal_emitted = true` so the
/// guard's `Drop` safety-net cannot double-emit.
///
/// `err_info` should faithfully describe which preflight failed
/// (e.g. `from_action_failure("harness_plan", msg)`) so a user-authored
/// `blocked.stack` can reference `{{ err.msg }}` meaningfully.
///
/// A late-binding evaluation error raised by the `blocked` or `finalize`
/// stack (a crashed `when:` guard, an unknown root under DM2 strict mode)
/// takes precedence over the original pre-flight failure: it is routed
/// through `failure` + `finalize` carrying the evaluation error as `err`
/// (when raised by `blocked`) and returned as a typed
/// [`CompositionError::LifecycleEvaluationError`] so the caller halts
/// non-zero on the actionable cause. A raise inside `finalize` itself is
/// surfaced without re-entering `finalize` (the re-entry guard from
/// Decision #3 / `handle_terminal_evaluation_error`).
/// Decide which evaluation error surfaces after a pre-flight `blocked` catch
/// ran its `failure`/`finalize` events, keeping the "already emitted to stderr"
/// bookkeeping correct (Decision #2).
///
/// The shared runtime router selects the winning event; this adapter renders
/// that event as a [`CompositionError`] for the pre-flight caller.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_preflight_blocked_and_finalize(
    guard: &mut LifecycleRunGuard<'_>,
    effect_engine: &EffectEngine,
    emitter: &dyn LifecycleEmitter,
    settings: &GlobalSettings,
    messaging: &RuntimeMessagingSettings,
    term: &Terminal,
    source_path: &Path,
    repo_root: Option<&Path>,
    base_dir: Option<&Path>,
    ctx_base_dir: Option<&Path>,
    prepared_context: Option<&darkmatter::markdown::compose::ComposeContext>,
    frontmatter: &serde_json::Map<String, serde_json::Value>,
    document_start: std::time::Instant,
    err_info: LifecycleErrorInfo,
) -> PreflightBlockedOutcome {
    let timing = LifecycleTiming::from_instants(document_start, None, std::time::Instant::now());
    // `current.ctx.*` follows the launch area like event-time `ctx.*` capture.
    let current_anchor = ctx_base_dir.or(base_dir).unwrap_or(source_path);
    let current = LifecycleCurrent::capture_at_event(current_anchor);

    let blocked_ctx = StackExecutionContext {
        signal: LifecycleSignal::Blocked,
        frontmatter,
        // Single pre-flight `blocked` event — no later event shares this state.
        live_frontmatter: None,
        err: Some(&err_info),
        timing: Some(&timing),
        current: Some(&current),
        base_dir,
        ctx_base_dir,
        prepared_context,
        effect_engine,
        shell_runner: &SystemShellRunner,
        emitter,
        term,
        source_path,
        repo_root,
        messaging,
        settings,
    };
    let blocked_outcome = guard.execute_event(LifecycleSignal::Blocked, &blocked_ctx);
    let original_early = blocked_outcome.evaluation_error.as_ref().map(|info| {
        crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            source_path, "blocked", info, term,
        )
    });
    let mut protocol = LifecycleCatchProtocol::new(
        LifecycleSignal::Blocked,
        LifecycleCatchState {
            terminal_slot: guard.terminal_signal(),
            finalize_emitted: guard.finalize_emitted(),
        },
        Some(err_info.clone()),
        blocked_outcome,
    );
    while let Some(step) = protocol.next_step().cloned() {
        let event_ctx = blocked_ctx.with_signal(step.signal);
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
    let result = protocol.finish().expect("preflight catch protocol completed");
    if let (Some(signal), Some(info)) = (
        result.evaluation_error_signal,
        result.evaluation_error.as_ref(),
    ) {
        let error = if signal == LifecycleSignal::Blocked {
            original_early.expect("origin evaluation error was emitted")
        } else {
            crate::output::error_walker::emit_lifecycle_evaluation_error_early(
                source_path,
                match signal {
                    LifecycleSignal::Failure => "failure",
                    LifecycleSignal::Finalize => "finalize",
                    _ => unreachable!("catch protocol returned an unexpected signal"),
                },
                info,
                term,
            )
        };
        return PreflightBlockedOutcome::EvaluationError(error);
    }
    PreflightBlockedOutcome::Control(result.control)
}

/// Translate a compose pre-flight `blocked` stack's surfaced flow-control action
/// into the error that should replace the generic blocked error.
///
/// At this layer the provider has not launched and there is no run-loop to
/// re-enter, so `resume` reports `ResumeWithoutSession` and `retry`/`requeue`/
/// `proxy` report the typed setup-phase-deferred error (a `blocked`-proxy is
/// decided mid-pre-flight, so it needs the same re-entry a `retry` would).
/// Returns `None` for `stop`/`error`/no-control, leaving the original blocked
/// error in place.
pub(super) fn preflight_blocked_control_error(
    control: Option<StackControl>,
    source_path: &Path,
) -> Option<CompositionError> {
    let outcome = LifecycleEventOutcome {
        control,
        ..LifecycleEventOutcome::default()
    };
    let decision = decide_lifecycle_transition(&LifecycleTransitionInput {
        event: LifecycleSignal::Blocked,
        terminal_slot: Some(LifecycleSignal::Blocked),
        provider_launched: false,
        has_prior_error: true,
        outcome: &outcome,
        has_session: false,
        attempt: 1,
        control_budget: 0,
        proxy_hops_used: 0,
        proxy_target_seen: false,
        finalize_emitted: true,
    });
    match decision {
        LifecycleTransitionDecision::Abort(LifecycleTransitionAbort::ResumeWithoutSession) => {
            Some(CompositionError::LifecycleResumeWithoutSession {
                source_path: source_path.to_path_buf(),
            })
        }
        LifecycleTransitionDecision::Reenter(_) => {
            Some(setup_phase_deferred("blocked", "retry", source_path))
        }
        LifecycleTransitionDecision::Abort(
            LifecycleTransitionAbort::DeferredExecutionUnsupported,
        ) => Some(CompositionError::LifecycleDeferNotImplemented {
            source_path: source_path.to_path_buf(),
        }),
        LifecycleTransitionDecision::ProxyHandoff { .. } => {
            Some(setup_phase_deferred("blocked", "proxy", source_path))
        }
        LifecycleTransitionDecision::Continue
        | LifecycleTransitionDecision::CatchFailure { .. }
        | LifecycleTransitionDecision::Finalize { .. }
        | LifecycleTransitionDecision::TerminalSuccess
        | LifecycleTransitionDecision::TerminalFailure { .. }
        | LifecycleTransitionDecision::Abort(_) => None,
    }
}

/// Build the typed setup-phase-deferred-recovery error.
pub(super) fn setup_phase_deferred(event: &str, action: &str, source_path: &Path) -> CompositionError {
    CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
        source_path: source_path.to_path_buf(),
        event: event.to_string(),
        action: action.to_string(),
    }
}
