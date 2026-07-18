//! Lifecycle event execution for the harness loop: running one event's
//! top-level communication + stack, terminal-event downgrade handling, and
//! the shared stack-context / `timing`/`current` capture helpers.

use super::*;

/// Execute a terminal lifecycle event, converting an explicit `Error` control
/// action into `Failure` for events that would otherwise record a successful
/// or blocked outcome.
///
/// `Success` and `Blocked` fire their top-level communication **first** (the
/// spec's top-level-before-stack contract: top-level properties are
/// unconditional and execute before the stack), recording the terminal signal,
/// then run their stack **exactly once**. If that stack terminates with
/// `StackControl::Error`, the run is downgraded to `Failure`: the guard's
/// terminal signal is re-designated to `Failure` and the `Failure` event's
/// top-level communication + stack fire. The already-fired success/blocked
/// top-level communication is **kept** — the spec requires top-level to fire
/// before stack processing, so an `error()` later in the stack cannot un-fire
/// it. Otherwise the success/blocked signal stays terminal. This preserves the
/// spec rule that an explicit `error()` in a success/blocked stack downgrades
/// the run, without running the success/blocked stack twice.
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
        // Take the terminal slot and fire the top-level communication FIRST
        // (before the stack), per the spec's top-level-before-stack contract.
        // If the slot was already taken by another terminal signal, do nothing.
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
        // A raised top-level interpolation fails the event closed before its
        // stack runs (mirroring `StackExecutionContext::execute_event`): report
        // it as the event's evaluation error so the caller halts the run.
        if let Some(info) = emit_lifecycle_top_level_already_recorded(
            guard,
            signal,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            err,
            loop_start,
        ) {
            return TerminalEventOutcome {
                outcome: LifecycleEventOutcome {
                    evaluation_error: Some(info),
                    ..Default::default()
                },
                downgrade_err: None,
                effective_event: event_name,
            };
        }
        // Now run the success/blocked stack exactly once.
        let outcome = run_lifecycle_stack_only(
            guard,
            signal,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            err,
            loop_start,
        );
        if let Some(StackControl::Error { reason }) = outcome.control.as_ref() {
            // The stack downgraded the run. Re-designate the already-recorded
            // terminal signal to `Failure` (keeping `terminal_emitted` true so
            // the later `finalize` still fires) and run the `failure` event's
            // top-level + stack directly. We must NOT call
            // `record_event_emission(Failure)` — the terminal slot is already
            // taken — so the failure event is run via a hand-built context.
            // The already-fired success/blocked top-level communication is
            // intentionally preserved.
            guard.redesignate_terminal_to_failure();
            let action_error = LifecycleErrorInfo::from_action_failure(
                "error",
                reason.clone().unwrap_or_default(),
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

/// Run the `Failure` event (top-level communication + stack) when a
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
    let (timing, current) =
        capture_lifecycle_globals(source_path, repo_root, guard.context().launch_area, loop_start);
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
        Some(&current),
    );
    guard.run_event_stack(LifecycleSignal::Failure, &ctx)
}

/// Emit only the top-level communication properties for `signal` (no stack),
/// for a terminal slot the caller has **already** recorded.
///
/// Used by [`execute_terminal_event`] for `success`/`blocked`: the caller takes
/// the terminal slot via [`LifecycleRunGuard::record_event_emission`] and then
/// calls this to fire the communication surface *before* the stack runs, per
/// the spec's top-level-before-stack contract. This helper does **not** record
/// emission state — the caller owns that.
///
/// Returns the late-binding evaluation error if the top-level interpolation
/// raised, so the terminal-phase caller can halt the run before the stack runs.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_lifecycle_top_level_already_recorded(
    guard: &claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
    loop_start: std::time::Instant,
) -> Option<LifecycleErrorInfo> {
    let (timing, current) =
        capture_lifecycle_globals(source_path, repo_root, guard.context().launch_area, loop_start);
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
        Some(&current),
    );
    ctx.emit_top_level_for_signal(guard.config())
}

/// Run one lifecycle event (top-level + stack), recording emission state in
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
    let (timing, current) =
        capture_lifecycle_globals(source_path, repo_root, guard.context().launch_area, loop_start);
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
        Some(&current),
    );
    guard.run_event_stack(signal, &ctx)
}

/// Run only the stack for `signal` (no top-level communication).
///
/// Used to preview success/blocked stacks for explicit `Error` control actions
/// before committing to the terminal signal.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_lifecycle_stack_only(
    guard: &claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
    loop_start: std::time::Instant,
) -> LifecycleEventOutcome {
    let (timing, current) =
        capture_lifecycle_globals(source_path, repo_root, guard.context().launch_area, loop_start);
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
        Some(&current),
    );
    ctx.execute_stack_for_signal(guard.config())
}

/// Build a stack context from a materialized prompt and guard-derived routes.
///
/// `timing` and `current` are the lifecycle stack-only globals. Callers own
/// them — they are captured fresh per event and outlive this context — see the
/// `run_lifecycle_event` / `emit_lifecycle_top_level_already_recorded` /
/// `run_lifecycle_stack_only` helpers.
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
    current: Option<&'a LifecycleCurrent>,
) -> StackExecutionContext<'a> {
    static EMPTY_FRONTMATTER: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>> =
        std::sync::OnceLock::new();
    let fm_map = materialized
        .frontmatter
        .as_object()
        .unwrap_or_else(|| EMPTY_FRONTMATTER.get_or_init(serde_json::Map::new));
    let base_dir = source_path.parent().or(repo_root);
    StackExecutionContext {
        signal,
        frontmatter: fm_map,
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
        current,
        group: None,
        base_dir,
        ctx_base_dir: launch_area,
        prepared_context,
        effect_engine,
        shell_runner: &SystemShellRunner,
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
/// The entry is published to the per-attempt live cell as well, so an
/// event-time `{{ last(outputs) }}` resolves against the same array a later
/// composition will be handed.
pub(super) fn commit_run_output(materialized: &MaterializedHarnessPrompt, stdout: &str) {
    materialized.runtime_state.append_output(stdout);
    materialized.live_frontmatter.borrow_mut().insert(
        claudine::composition::OUTPUTS_KEY.to_string(),
        materialized.runtime_state.outputs_value(),
    );
}

/// Capture the lifecycle stack-only `timing`/`current` globals for an event.
///
/// `current.env` is the live process environment and `current.ctx` is the full
/// Darkmatter `ctx.*` namespace, both captured **now** so a side effect or
/// external change since `prepare` is observable through `current.*` at event
/// time. `timing` measures wall-clock elapsed against `loop_start`
/// (`document_ms` and `total_ms`; the harness loop has no sequence-step clock,
/// so `step_ms` stays `None`).
pub(super) fn capture_lifecycle_globals(
    source_path: &Path,
    repo_root: Option<&Path>,
    launch_area: Option<&Path>,
    loop_start: std::time::Instant,
) -> (LifecycleTiming, LifecycleCurrent) {
    let base_dir = source_path.parent().or(repo_root);
    // `current.ctx.*` follows the launch area like event-time `ctx.*` capture.
    let current = match launch_area.or(base_dir) {
        Some(dir) => LifecycleCurrent::capture_at_event(dir),
        None => LifecycleCurrent::capture_env_only(),
    };
    let timing =
        LifecycleTiming::from_instants(loop_start, Some(loop_start), std::time::Instant::now());
    (timing, current)
}
