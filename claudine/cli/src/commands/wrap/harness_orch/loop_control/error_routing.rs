//! Blocked/failure/finalize error routing: threading a setup failure or a
//! late-binding evaluation error through the catch events (Decisions #2/#3/#5)
//! and deciding which raise surfaces.

use super::*;

/// Route a pre-launch setup failure through the stack-aware terminal +
/// `Finalize` events carrying an `err` payload.
///
/// Mirrors [`LifecycleRunGuard::emit_blocked_or_failure`]'s signal selection
/// (`Failure` once the provider launched, `Blocked` before) but, unlike the
/// legacy `emit_blocked_or_err`, runs the typed stack and the `finalize` event
/// so user-authored `blocked.stack`/`failure.stack`/`finalize.stack` fire with
/// `err.kind`/`err.variant`/`err.msg` available. Used by the harness-loop
/// setup-failure sites (materialize / target-lifecycle parse / harness-plan
/// parse) that occur after the lifecycle has already started.
///
/// Returns `Some(CompositionError::LifecycleEvaluationError)` when the
/// terminal stack or the `finalize` stack itself raised a late-binding
/// evaluation error — the caller must propagate that error in place of the
/// original setup error so the run halts non-zero on the lifecycle raise
/// instead of the (less informative) setup failure. Returns `None` when no
/// evaluation error occurred, so the caller propagates the original setup
/// error unchanged.
///
/// Terminal-slot conflict: when `Blocked` is selected and its stack raises,
/// the terminal slot is already taken. To still fire the `failure` stack with
/// the evaluation error as `err`, the helper redesignates the slot to
/// `Failure` and runs the failure stack directly via
/// [`LifecycleRunGuard::run_event_stack`] (bypassing
/// [`LifecycleRunGuard::record_event_emission`], which would refuse the taken
/// slot). Mirrors [`run_failure_event_for_downgrade`].
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_blocked_finalize_with_err(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: &LifecycleErrorInfo,
    loop_start: std::time::Instant,
) -> Option<CompositionError> {
    let (terminal, terminal_event_name) = if guard.provider_launched() {
        (LifecycleSignal::Failure, "failure")
    } else {
        (LifecycleSignal::Blocked, "blocked")
    };
    // NOTE: these are rare internal setup-failure paths (materialize /
    // target-lifecycle parse) reached inside `inspect_err`/`Err` arms that
    // propagate their own error. A `blocked.stack` flow-control action here is
    // not yet dispatched — the common compose pre-flight `blocked` path (shell
    // audit / schema) is handled by `preflight_blocked_control_error`.
    let terminal_outcome = run_lifecycle_event(
        guard,
        terminal,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(err),
        loop_start,
    );
    if let Some(eval_info) = terminal_outcome.evaluation_error.as_ref() {
        // The terminal stack raised. When `Blocked` took the slot, the
        // failure stack can only fire after redesignating the slot to
        // `Failure` and bypassing `record_event_emission` (it would refuse
        // the taken slot). When `Failure` already took the slot, do not
        // re-enter failure.
        let failure_outcome = if matches!(terminal, LifecycleSignal::Blocked) {
            guard.redesignate_terminal_to_failure();
            let (timing, current) = capture_lifecycle_globals(
                source_path,
                repo_root,
                guard.context().launch_area,
                loop_start,
            );
            let failure_ctx = build_lifecycle_stack_context_for_materialized(
                LifecycleSignal::Failure,
                materialized,
                source_path,
                repo_root,
                guard.context().launch_area,
                guard.effective_prepared_context(),
                term,
                guard.emitter(),
                guard.context().settings,
                guard.context().messaging,
                effect_engine,
                Some(eval_info),
                Some(&timing),
                Some(&current),
            );
            Some(guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx))
        } else {
            None
        };
        // If `failure` raised, thread its error (not the original) into
        // finalize so a `finalize.stack` can branch on the failure raise.
        let active_err = failure_outcome
            .as_ref()
            .and_then(|o| o.evaluation_error.as_ref())
            .unwrap_or(eval_info);
        let finalize_outcome = run_lifecycle_event(
            guard,
            LifecycleSignal::Finalize,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            Some(active_err),
            loop_start,
        );
        // The original `err` was a setup/dispatch failure (not an evaluation
        // error); only the catch event raised. Emit the surfaced evaluation
        // error now — no further lifecycle events fire (Decision #2).
        return Some(crate::output::error_walker::emit_lifecycle_evaluation_error_block(
            CompositionError::catch_evaluation_error(
                source_path,
                terminal_event_name,
                eval_info,
                failure_outcome.as_ref(),
                Some(&finalize_outcome),
            ),
            term,
        ));
    }
    let finalize_outcome = run_lifecycle_event(
        guard,
        LifecycleSignal::Finalize,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(err),
        loop_start,
    );
    let decision = if matches!(terminal, LifecycleSignal::Blocked) {
        route_blocked_finalize(&terminal_outcome, None, Some(&finalize_outcome))
    } else {
        route_failure_finalize(&terminal_outcome, Some(&finalize_outcome))
    };
    // An evaluation error raised *inside* finalize halts the run; do not
    // re-enter finalize.
    if decision.evaluation_error_signal == Some(LifecycleSignal::Finalize) {
        finalize_outcome.evaluation_error.as_ref().map(|eval_info| {
            crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                CompositionError::lifecycle_evaluation("finalize", source_path, eval_info),
                term,
            )
        })
    } else {
        None
    }
}

/// Route a **post-`start`** setup failure through the stack-aware `Failure` +
/// `Finalize` events carrying an `err` payload.
///
/// Mirrors [`emit_blocked_finalize_with_err`] but hardcodes `Failure` as the
/// terminal signal: these sites run after `start` has fired and pre-flight has
/// already passed, so the failure is never semantically `Blocked` (which means
/// pre-flight failed). The harness setup steps between `start` and the first
/// terminal event — snapshot capture, launch construction, and the
/// pre-spawn portion of attempt execution — propagate their errors with a bare
/// `?`; without this routing only `LifecycleRunGuard::drop`'s legacy
/// `emit_signal` path would run, which never executes the typed
/// `failure.stack`/`finalize.stack` nor exposes `err.kind`/`err.variant`/
/// `err.msg`. Used for the snapshot / launch / attempt `?` sites.
///
/// Returns `Some(CompositionError::LifecycleEvaluationError)` when the
/// `failure` stack or the `finalize` stack raised a late-binding evaluation
/// error; the caller must propagate that error in place of the original setup
/// error so the run halts on the lifecycle raise. `Failure` is the terminal
/// signal, so there is no slot-conflict redesignation here (unlike
/// [`emit_blocked_finalize_with_err`]).
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_failure_finalize_with_err(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: &LifecycleErrorInfo,
    loop_start: std::time::Instant,
) -> Option<CompositionError> {
    let failure_outcome = run_lifecycle_event(
        guard,
        LifecycleSignal::Failure,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(err),
        loop_start,
    );
    if let Some(eval_info) = failure_outcome.evaluation_error.as_ref() {
        // Carry the evaluation error (not the original `err`) into finalize so
        // a `finalize.stack` can branch on the lifecycle raise. Do not re-enter
        // failure — its slot is already taken.
        let finalize_outcome = run_lifecycle_event(
            guard,
            LifecycleSignal::Finalize,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            Some(eval_info),
            loop_start,
        );
        // The original `err` was a setup/dispatch failure (not an evaluation
        // error); only the catch `failure` raised. Emit the surfaced evaluation
        // error now — no further lifecycle events fire (Decision #2).
        return Some(crate::output::error_walker::emit_lifecycle_evaluation_error_block(
            CompositionError::catch_evaluation_error(
                source_path,
                "failure",
                eval_info,
                Some(&failure_outcome),
                Some(&finalize_outcome),
            ),
            term,
        ));
    }
    let finalize_outcome = run_lifecycle_event(
        guard,
        LifecycleSignal::Finalize,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(err),
        loop_start,
    );
    let decision = route_failure_finalize(&failure_outcome, Some(&finalize_outcome));
    // An evaluation error raised *inside* finalize halts the run; do not
    // re-enter finalize.
    if decision.evaluation_error_signal == Some(LifecycleSignal::Finalize) {
        finalize_outcome.evaluation_error.as_ref().map(|eval_info| {
            crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                CompositionError::lifecycle_evaluation("finalize", source_path, eval_info),
                term,
            )
        })
    } else {
        None
    }
}

/// Decide which evaluation error surfaces after the catch events ran, and
/// keep the "already emitted to stderr" bookkeeping correct (Decision #2).
///
/// The original raise was already emitted at the catch point (`early`, an
/// already-marked error). [`CompositionError::catch_evaluation_error`] applies
/// the precedence rule "a raise inside `finalize` beats a raise inside
/// `failure` beats the original". When a *catch event* raised a newer error,
/// that newer crash has **not** yet been shown, so this emits it once now (no
/// further lifecycle events fire after `finalize`) and marks it emitted. When
/// no catch event raised, the surfaced error is the original `early` marker
/// returned unchanged — exactly one styled emission either way.
pub(super) fn surface_catch_evaluation_error(
    source_path: &Path,
    failure_outcome: Option<&LifecycleEventOutcome>,
    finalize_outcome: Option<&LifecycleEventOutcome>,
    early: CompositionError,
    term: &Terminal,
) -> color_eyre::eyre::Report {
    let empty = LifecycleEventOutcome::default();
    let decision = route_failure_finalize(failure_outcome.unwrap_or(&empty), finalize_outcome);
    let surfaced = match decision.evaluation_error_signal {
        Some(LifecycleSignal::Finalize) => {
            let info = finalize_outcome
                .and_then(|outcome| outcome.evaluation_error.as_ref())
                .expect("routing decision identifies a finalize evaluation error");
            crate::output::error_walker::emit_lifecycle_evaluation_error_early(
                source_path, "finalize", info, term,
            )
        }
        Some(LifecycleSignal::Failure) => {
            let info = failure_outcome
                .and_then(|outcome| outcome.evaluation_error.as_ref())
                .expect("routing decision identifies a failure evaluation error");
            crate::output::error_walker::emit_lifecycle_evaluation_error_early(
                source_path, "failure", info, term,
            )
        }
        _ => early,
    };
    surfaced.into()
}

/// Handle a terminal-phase lifecycle **evaluation** error (Decision #3).
///
/// A `when:` guard, top-level string, or action-value interpolation that
/// *raised* on `success`/`failure`/`loop` halts the run: the provider already
/// ran (and may have genuinely succeeded), so this does **not** retroactively
/// fire `failure`. Instead it runs `finalize` exactly once carrying the error
/// as the `err` global (so a `finalize.stack` can react), then returns the
/// typed run failure for the caller to propagate non-zero.
///
/// Returns `None` when `outcome` carries no evaluation error, so the caller
/// continues with its normal terminal handling (control dispatch + finalize).
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_terminal_evaluation_error(
    outcome: &LifecycleEventOutcome,
    event: &str,
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    loop_start: std::time::Instant,
) -> Option<color_eyre::eyre::Report> {
    let info = outcome.evaluation_error.as_ref()?;
    // Surface the original crash to stderr at the point of error, before the
    // `finalize` catch event fires (Decision #2). The returned error is marked
    // already-emitted so the outer renderer does not print the styled block a
    // second time — unless a later raise inside `finalize` supersedes it.
    let early = crate::output::error_walker::emit_lifecycle_evaluation_error_early(
        source_path,
        event,
        info,
        term,
    );
    let finalize_outcome = run_lifecycle_event(
        guard,
        LifecycleSignal::Finalize,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(info),
        loop_start,
    );
    Some(surface_catch_evaluation_error(
        source_path,
        None,
        Some(&finalize_outcome),
        early,
        term,
    ))
}

/// Handle a setup-phase lifecycle **evaluation** error (Decision #5).
///
/// `initialize`/`start`/`blocked` route an evaluation error through `failure`
/// then `finalize` (carrying it as the `err` global) exactly like any other
/// setup failure, then return the typed run failure for the caller to
/// propagate non-zero. Returns `None` when `outcome` carries no evaluation
/// error.
#[allow(clippy::too_many_arguments)]
pub(super) fn handle_setup_evaluation_error(
    outcome: &LifecycleEventOutcome,
    event: &str,
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    loop_start: std::time::Instant,
) -> Option<color_eyre::eyre::Report> {
    let info = outcome.evaluation_error.as_ref()?;
    // Surface the original crash to stderr at the point of error, before the
    // `failure`/`finalize` catch events fire (Decision #2). Marked already-
    // emitted so the outer renderer does not double-emit; a later catch-event
    // raise supersedes it.
    let early = crate::output::error_walker::emit_lifecycle_evaluation_error_early(
        source_path,
        event,
        info,
        term,
    );
    let failure_outcome = run_lifecycle_event(
        guard,
        LifecycleSignal::Failure,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(info),
        loop_start,
    );
    // If `failure` raised, thread its error (not the original) into finalize
    // so a `finalize.stack` can branch on the failure raise.
    let active_err = failure_outcome
        .evaluation_error
        .as_ref()
        .unwrap_or(info);
    let finalize_outcome = run_lifecycle_event(
        guard,
        LifecycleSignal::Finalize,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(active_err),
        loop_start,
    );
    Some(surface_catch_evaluation_error(
        source_path,
        Some(&failure_outcome),
        Some(&finalize_outcome),
        early,
        term,
    ))
}
