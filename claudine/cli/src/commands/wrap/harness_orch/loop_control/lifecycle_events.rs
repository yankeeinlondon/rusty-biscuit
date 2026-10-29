//! Lifecycle event execution for the harness loop: running one event's
//! stack, terminal-event downgrade handling, and
//! the shared stack-context and `timing` capture helpers.

use claudine::composition::LifecycleRunGuardExt;
use super::*;

/// Execute a terminal lifecycle event, converting an explicit `Error` control
/// action into `Failure` for events that would otherwise record a successful
/// or blocked outcome.
///
/// `Success` and `Blocked` record the terminal signal, then run their stack
/// **exactly once**. If that stack terminates with `StackControl::Error`, the
/// run is downgraded to `Failure`: the guard's terminal signal is re-designated
/// to `Failure` and the `Failure` event's stack runs. Actions that ran before
/// the `error` directive keep their effects — items run in order, so an
/// `error` later in the stack cannot un-fire them. Otherwise the
/// success/blocked signal stays terminal. This preserves the rule that an
/// explicit `error` in a success/blocked stack downgrades the run, without
/// running the success/blocked stack twice.
/// Result of running a terminal lifecycle event via [`execute_terminal_event`].
#[derive(Default)]
pub(super) struct TerminalEventOutcome {
    /// The control + action-error the (possibly downgraded) event reported.
    /// For a `success`/`blocked` stack that downgraded via `error()`, this is
    /// the *failure* event's outcome (so its recovery control is dispatchable).
    pub(super) outcome: LifecycleEventOutcome,
    /// Present when a `success`/`blocked` stack downgraded the run to failure
    /// via an explicit `error()`. This is the `err` the subsequent `finalize`
    /// must carry so a `finalize.stack` can branch on `err` and recover.
    pub(super) downgrade_err: Option<LifecycleErrorInfo>,
    /// The event name to report when an evaluation error surfaces from
    /// `outcome`. Matches the signal name (`"success"`/`"blocked"`/
    /// `"failure"`) — except when a `success`/`blocked` stack downgraded via
    /// explicit `error()`, in which case `outcome` holds the downgraded
    /// `failure` event's result and this is `"failure"` so the surfaced
    /// diagnostic points at the right stack.
    pub(super) effective_event: &'static str,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_terminal_event(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
    loop_start: std::time::Instant,
) -> TerminalEventOutcome {
    if matches!(signal, LifecycleSignal::Success | LifecycleSignal::Blocked) {
        // Take the terminal slot. If another terminal signal already took it,
        // do nothing.
        if !guard.record_event_emission(signal) {
            return TerminalEventOutcome::default();
        }
        // `signal` is Success or Blocked here; the downgrading return below
        // overrides this local to `"failure"` because `outcome` then holds the
        // failure event's result.
        let event_name: &'static str = if matches!(signal, LifecycleSignal::Success) {
            "success"
        } else {
            "blocked"
        };
        let outcome = {
            let timing = capture_lifecycle_timing(loop_start);
            let ctx = build_lifecycle_stack_context_for_materialized(
                signal,
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
                err,
                Some(&timing),
            );
            guard.run_event_stack(signal, &ctx)
        };
        if let Some(StackControl::Error { reason }) = outcome.control.as_ref() {
            // The stack downgraded the run. Re-designate the already-recorded
            // terminal signal to `Failure` (keeping `terminal_emitted` true so
            // the later `finalize` still fires) and run the `failure` event's
            // stack directly. We must NOT call `record_event_emission(Failure)`
            // — the terminal slot is already taken — so the failure event is
            // run via a hand-built context. Actions that ran before the
            // `error` directive keep their effects.
            guard.redesignate_terminal_to_failure();
            // A reasonless `error` gets the same default message as one raised
            // from `start`, so `err.msg` and the rendered error never go blank.
            let action_error = LifecycleErrorInfo::from_action_failure(
                "error",
                reason
                    .clone()
                    .unwrap_or_else(|| format!("lifecycle {event_name} error")),
            );
            let failure_outcome = run_failure_event_for_downgrade(
                guard,
                materialized,
                source_path,
                repo_root,
                term,
                effect_engine,
                &action_error,
                loop_start,
            );
            return TerminalEventOutcome {
                outcome: failure_outcome,
                downgrade_err: Some(action_error),
                effective_event: "failure",
            };
        }
        return TerminalEventOutcome {
            outcome,
            downgrade_err: None,
            effective_event: event_name,
        };
    }
    TerminalEventOutcome {
        outcome: run_lifecycle_event(
            guard,
            signal,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            err,
            loop_start,
        ),
        downgrade_err: None,
        effective_event: "failure",
    }
}

/// Run the `Failure` event's stack when a
/// success/blocked stack downgraded the run via an explicit `error()`.
///
/// The terminal slot was already taken by the success/blocked signal (and
/// re-designated to `Failure` by the caller), so this runs the failure event
/// directly rather than through [`run_lifecycle_event`] /
/// [`LifecycleRunGuard::record_event_emission`], which would refuse the taken
/// slot. `terminal_emitted` stays true so a subsequent `finalize` fires.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_failure_event_for_downgrade(
    guard: &claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: &LifecycleErrorInfo,
    loop_start: std::time::Instant,
) -> LifecycleEventOutcome {
    let timing = capture_lifecycle_timing(loop_start);
    let ctx = build_lifecycle_stack_context_for_materialized(
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
        Some(err),
        Some(&timing),
    );
    guard.run_event_stack(LifecycleSignal::Failure, &ctx)
}

/// Run one lifecycle event's stack, recording emission state in
/// `guard` and returning the event outcome.
///
/// The helper is careful to release the mutable borrow used for state recording
/// before building the [`StackExecutionContext`] that immutably borrows the
/// guard's emitter.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_lifecycle_event(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
    loop_start: std::time::Instant,
) -> LifecycleEventOutcome {
    if !guard.record_event_emission(signal) {
        return LifecycleEventOutcome::default();
    }
    let timing = capture_lifecycle_timing(loop_start);
    let ctx = build_lifecycle_stack_context_for_materialized(
        signal,
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
        err,
        Some(&timing),
    );
    guard.run_event_stack(signal, &ctx)
}

/// Build a stack context from a materialized prompt and guard-derived routes.
///
/// `timing` is the lifecycle stack-only global; callers capture it fresh per
/// event — see [`run_lifecycle_event`] and [`execute_terminal_event`]. Build
/// one context per event: the lazy `current` root is served by the
/// invocation's refresh authority, taken from the materialization's document
/// epoch, and memoized for this context so every read in the event agrees.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_lifecycle_stack_context_for_materialized<'a>(
    signal: LifecycleSignal,
    materialized: &'a MaterializedHarnessPrompt,
    source_path: &'a Path,
    repo_root: Option<&'a Path>,
    launch_area: Option<&'a Path>,
    prepared_context: Option<&'a darkmatter::markdown::compose::ComposeContext>,
    term: &'a Terminal,
    emitter: &'a dyn claudine::composition::LifecycleEmitter,
    settings: &'a claudine::events::GlobalSettings,
    messaging: &'a claudine::messaging::RuntimeMessagingSettings,
    effect_engine: &'a EffectEngine,
    err: Option<&'a LifecycleErrorInfo>,
    timing: Option<&'a LifecycleTiming>,
) -> StackExecutionContext<'a> {
    let base_dir = source_path.parent().or(repo_root);
    StackExecutionContext {
        seq: None,
        signal,
        scope: None,
        frontmatter: &materialized.frontmatter_view,
        // The per-attempt live cell carried by `materialized` is shared across
        // every lifecycle event in this iteration, so a `start.stack`
        // frontmatter mutation is visible to a later `success`/`finalize`
        // event-time interpolation (review-2 cross-event contract).
        live_frontmatter: Some(&materialized.live_frontmatter),
        // Invocation-scoped, unlike the per-attempt live cell above: a `set`
        // written here survives re-materialization and (for a sequence) the
        // step boundary.
        runtime_state: Some(&materialized.runtime_state),
        err,
        timing,
        // One event, one `current` observation per key.
        current: materialized
            .document_epoch
            .as_ref()
            .map(|epoch| epoch.current_authority().memoized()),
        group: None,
        loop_values: materialized.loop_values.as_ref(),
        base_dir,
        ctx_base_dir: launch_area,
        prepared_context,
        file_resolution_context: &materialized.file_resolution_context,
        effect_engine,
        // The materialized document's `shell` actions start from its own
        // environment.
        shell_runner: materialized
            .document_epoch
            .as_ref()
            .map_or(&SystemShellRunner as &dyn claudine::composition::ShellRunner, |epoch| epoch.shell_runner()),
        emitter,
        term,
        source_path,
        repo_root,
        messaging,
        settings,
    }
}

/// Commit a completed run's captured stdout as the next `outputs` entry.
///
/// Called on the success path only, *before* the `success` event fires, which
/// is what gives the lifecycle hooks their specified temporal view: `success`
/// and `finalize` see the entry this run produced, while `initialize`/`start`
/// (and `failure`, which never reaches here) see only prior entries.
///
/// Event-time `seq.outputs` and later composition both read this runtime
/// cell; no generated output property is published into frontmatter.
pub(super) fn commit_run_output(materialized: &MaterializedHarnessPrompt, stdout: &str) {
    materialized.runtime_state.append_output(stdout);

}

/// Capture the lifecycle stack-only `timing` global for an event.
///
/// Measures wall-clock elapsed against `loop_start` (`document_ms` and
/// `total_ms`; the harness loop has no sequence-step clock, so `step_ms` stays
/// `None`).
pub(super) fn capture_lifecycle_timing(loop_start: std::time::Instant) -> LifecycleTiming {
    LifecycleTiming::from_instants(loop_start, Some(loop_start), std::time::Instant::now())
}
