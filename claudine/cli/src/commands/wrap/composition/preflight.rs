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
    CompositionError, LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming,
    LifecycleTransitionAbort, LifecycleTransitionDecision, LifecycleTransitionInput,
    decide_lifecycle_transition, route_blocked_finalize,
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

    // A late-binding evaluation error on `blocked` (a crashed `when:` guard or
    // interpolation) routes through `failure` + `finalize` carrying the
    // evaluation error as `err`, then surfaces the typed run failure
    // (Decision #5). `blocked` already took the terminal slot, so we
    // redesignate it to `Failure` (mirroring the `error()`-downgrade path in
    // `execute_terminal_event`) and run the failure stack directly via
    // `run_event_stack` (which bypasses the already-recorded slot). The
    // subsequent `execute_event(Finalize)` still works because
    // `terminal_emitted` remains true and `finalize_emitted` is unset.
    if let Some(eval_info) = blocked_outcome.evaluation_error.as_ref() {
        // Surface the original `blocked` crash to stderr before the
        // `failure`/`finalize` catch events fire (Decision #2).
        let early = crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            source_path,
            "blocked",
            eval_info,
            term,
        );
        guard.redesignate_terminal_to_failure();
        let failure_ctx = blocked_ctx.with_signal(LifecycleSignal::Failure);
        let failure_ctx = failure_ctx.with_error(eval_info);
        let failure_outcome = guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);
        // If `failure` raised, thread its error (not the original) into
        // finalize so a `finalize.stack` can branch on the failure raise.
        let active_err = failure_outcome
            .evaluation_error
            .as_ref()
            .unwrap_or(eval_info);
        let finalize_ctx = blocked_ctx.with_signal(LifecycleSignal::Finalize);
        let finalize_ctx = finalize_ctx.with_error(active_err);
        let finalize_outcome = guard.execute_event(LifecycleSignal::Finalize, &finalize_ctx);
        return PreflightBlockedOutcome::EvaluationError(surface_preflight_catch_error(
            source_path,
            Some(&failure_outcome),
            Some(&finalize_outcome),
            early,
            term,
        ));
    }

    // No evaluation error on `blocked`: run `finalize` with the original
    // pre-flight `err`. `with_signal` borrows `blocked_ctx` by shared
    // reference, which does not conflict with the `&mut guard` `execute_event`
    // requires because the guard and the context borrow from disjoint locals
    // (emitter/settings/... passed in as arguments, not pulled out of the
    // guard).
    let finalize_ctx = blocked_ctx.with_signal(LifecycleSignal::Finalize);
    let finalize_outcome = guard.execute_event(LifecycleSignal::Finalize, &finalize_ctx);

    // A raise inside `finalize` itself halts without re-entering `finalize`
    // (the re-entry guard). Surface it to stderr at the point of error.
    let decision = route_blocked_finalize(&blocked_outcome, None, Some(&finalize_outcome));
    if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
        let early = crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            source_path,
            "finalize",
            eval_info,
            term,
        );
        return PreflightBlockedOutcome::EvaluationError(early);
    }

    // Surface the blocked stack's flow-control action (if any) so the caller
    // can dispatch it. At the compose pre-flight layer the provider has not
    // launched and there is no run-loop to re-enter, so the caller maps
    // `resume` → `ResumeWithoutSession` and `retry`/`requeue`/`proxy` → a
    // typed setup-phase-deferred error rather than silently dropping the
    // control.
    PreflightBlockedOutcome::Control(decision.control)
}

/// Decide which evaluation error surfaces after a pre-flight `blocked` catch
/// ran its `failure`/`finalize` events, keeping the "already emitted to stderr"
/// bookkeeping correct (Decision #2).
///
/// The shared runtime router selects the winning event; this adapter renders
/// that event as a [`CompositionError`] for the pre-flight caller.
pub(super) fn surface_preflight_catch_error(
    source_path: &Path,
    failure_outcome: Option<&LifecycleEventOutcome>,
    finalize_outcome: Option<&LifecycleEventOutcome>,
    early: CompositionError,
    term: &Terminal,
) -> CompositionError {
    let empty = LifecycleEventOutcome::default();
    let decision = claudine::composition::route_failure_finalize(
        failure_outcome.unwrap_or(&empty),
        finalize_outcome,
    );
    let (event, info) = match decision.evaluation_error_signal {
        Some(LifecycleSignal::Finalize) => (
            "finalize",
            finalize_outcome.and_then(|outcome| outcome.evaluation_error.as_ref()),
        ),
        Some(LifecycleSignal::Failure) => (
            "failure",
            failure_outcome.and_then(|outcome| outcome.evaluation_error.as_ref()),
        ),
        _ => return early,
    };
    crate::output::error_walker::emit_lifecycle_evaluation_error_early(
        source_path,
        event,
        info.expect("routing decision identifies an evaluation error"),
        term,
    )
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
