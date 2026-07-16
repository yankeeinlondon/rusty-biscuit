//! Blocked/failure/finalize error routing: threading a setup failure or a
//! late-binding evaluation error through the catch events (Decisions #2/#3/#5)
//! and deciding which raise surfaces.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn run_catch_protocol(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    origin: LifecycleSignal,
    origin_outcome: LifecycleEventOutcome,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
    loop_start: std::time::Instant,
) -> claudine::composition::LifecycleCatchResult {
    let mut protocol = LifecycleCatchProtocol::new(
        origin,
        LifecycleCatchState {
            terminal_slot: guard.terminal_signal(),
            finalize_emitted: guard.finalize_emitted(),
        },
        err.cloned(),
        origin_outcome,
    );
    while let Some(step) = protocol.next_step().cloned() {
        let outcome = match step.execution {
            LifecycleCatchExecution::Record => run_lifecycle_event(
                guard,
                step.signal,
                materialized,
                source_path,
                repo_root,
                term,
                effect_engine,
                step.error.as_ref(),
                loop_start,
            ),
            LifecycleCatchExecution::RedesignateBlockedAsFailure => {
                guard.redesignate_terminal_to_failure();
                let (timing, current) = capture_lifecycle_globals(
                    source_path,
                    repo_root,
                    guard.context().launch_area,
                    loop_start,
                );
                let failure_ctx = build_lifecycle_stack_context_for_materialized(
                    step.signal,
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
                    step.error.as_ref(),
                    Some(&timing),
                    Some(&current),
                );
                guard.run_event_stack(step.signal, &failure_ctx)
            }
        };
        assert!(protocol.record(step.signal, outcome));
    }
    protocol.finish().expect("lifecycle catch protocol completed")
}

pub(super) fn render_catch_evaluation(
    result: &claudine::composition::LifecycleCatchResult,
    source_path: &Path,
    term: &Terminal,
) -> Option<CompositionError> {
    let signal = result.evaluation_error_signal?;
    let info = result
        .evaluation_error
        .as_ref()
        .expect("evaluation signal carries error info");
    Some(crate::output::error_walker::emit_lifecycle_evaluation_error_block(
        CompositionError::lifecycle_evaluation(
            match signal {
                LifecycleSignal::Blocked => "blocked",
                LifecycleSignal::Failure => "failure",
                LifecycleSignal::Finalize => "finalize",
                _ => unreachable!("catch protocol returned an unexpected signal"),
            },
            source_path,
            info,
        ),
        term,
    ))
}

/// Route a pre-launch setup failure through the stack-aware terminal +
/// `Finalize` events carrying an `err` payload.
///
/// Uses [`LifecycleRunGuard::emit_blocked_or_failure`]'s signal contract
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
/// slot). This is the same terminal-slot rule used by
/// [`run_failure_event_for_downgrade`].
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
    let terminal = if guard.provider_launched() {
        LifecycleSignal::Failure
    } else {
        LifecycleSignal::Blocked
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
    let result = run_catch_protocol(
        guard,
        terminal,
        terminal_outcome,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(err),
        loop_start,
    );
    render_catch_evaluation(&result, source_path, term)
}

/// Route a **post-`start`** setup failure through the stack-aware `Failure` +
/// `Finalize` events carrying an `err` payload.
///
/// Uses `Failure` as the terminal signal because these sites run after `start`
/// has fired and pre-flight has
/// already passed, so the failure is never semantically `Blocked` (which means
/// pre-flight failed). The harness setup steps between `start` and the first
/// terminal event — launch construction and the pre-spawn portion of attempt
/// execution — propagate their errors with a bare `?`; without this routing
/// only `LifecycleRunGuard::drop`'s legacy
/// `emit_signal` path would run, which never executes the typed
/// `failure.stack`/`finalize.stack` nor exposes `err.kind`/`err.variant`/
/// `err.msg`. Used for the launch and attempt `?` sites.
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
    let result = run_catch_protocol(
        guard,
        LifecycleSignal::Failure,
        failure_outcome,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(err),
        loop_start,
    );
    render_catch_evaluation(&result, source_path, term)
}

pub(super) fn surface_protocol_evaluation(
    result: &claudine::composition::LifecycleCatchResult,
    origin: LifecycleSignal,
    source_path: &Path,
    early: Option<CompositionError>,
    term: &Terminal,
) -> color_eyre::eyre::Report {
    let surfaced = result
        .evaluation_error_signal
        .filter(|signal| *signal != origin)
        .map(|signal| {
            let info = result
                .evaluation_error
                .as_ref()
                .expect("evaluation signal carries error info");
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
        })
        .unwrap_or_else(|| early.expect("origin evaluation error was emitted"));
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
    let origin = match event {
        "success" => LifecycleSignal::Success,
        "failure" => LifecycleSignal::Failure,
        "loop" => LifecycleSignal::Loop,
        _ => unreachable!("terminal evaluation routing received a non-terminal event"),
    };
    let result = run_catch_protocol(
        guard,
        origin,
        outcome.clone(),
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        Some(info),
        loop_start,
    );
    Some(surface_protocol_evaluation(
        &result,
        origin,
        source_path,
        Some(early),
        term,
    ))
}
