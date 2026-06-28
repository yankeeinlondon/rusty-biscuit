use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::LifecycleSignal;
use claudine::composition::lifecycle_context::{LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming};
use claudine::composition::lifecycle_control::{ControlDispatch, control_budget_for, decide_control};
use claudine::composition::lifecycle_executor::{
    LifecycleEventOutcome, StackControl, StackExecutionContext, SystemShellRunner,
};
use claudine::composition::CompositionError;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use darkmatter::effects::EffectEngine;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use tracing::info_span;

use super::super::composition::IterationSummarySignals;
use super::{
    CachedHarnessLoopContext, HarnessPromptState, MaterializedHarnessPrompt, build_harness_launch,
    execute_harness_attempt, harness_prompt_mode_label, materialize_harness_prompt, HarnessPromptMode,
};

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
struct TerminalEventOutcome {
    /// The control + action-error the (possibly downgraded) event reported.
    /// For a `success`/`blocked` stack that downgraded via `error()`, this is
    /// the *failure* event's outcome (so its recovery control is dispatchable).
    outcome: LifecycleEventOutcome,
    /// Present when a `success`/`blocked` stack downgraded the run to failure
    /// via an explicit `error()`. This is the `err` the subsequent `finalize`
    /// must carry so a `finalize.stack` can branch on `err` and recover.
    downgrade_err: Option<LifecycleErrorInfo>,
    /// The event name to report when an evaluation error surfaces from
    /// `outcome`. Matches the signal name (`"success"`/`"blocked"`/
    /// `"failure"`) — except when a `success`/`blocked` stack downgraded via
    /// explicit `error()`, in which case `outcome` holds the downgraded
    /// `failure` event's result and this is `"failure"` so the surfaced
    /// diagnostic points at the right stack.
    effective_event: &'static str,
}

#[allow(clippy::too_many_arguments)]
fn execute_terminal_event(
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
fn run_failure_event_for_downgrade(
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
        guard.context().context,
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
fn emit_lifecycle_top_level_already_recorded(
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
        guard.context().context,
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
fn run_lifecycle_event(
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
        guard.context().context,
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
fn emit_blocked_finalize_with_err(
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
                guard.context().context,
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
    // An evaluation error raised *inside* finalize halts the run; do not
    // re-enter finalize.
    finalize_outcome.evaluation_error.as_ref().map(|eval_info| {
        crate::output::error_walker::emit_lifecycle_evaluation_error_block(
            CompositionError::lifecycle_evaluation("finalize", source_path, eval_info),
            term,
        )
    })
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
fn emit_failure_finalize_with_err(
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
    // An evaluation error raised *inside* finalize halts the run; do not
    // re-enter finalize.
    finalize_outcome.evaluation_error.as_ref().map(|eval_info| {
        crate::output::error_walker::emit_lifecycle_evaluation_error_block(
            CompositionError::lifecycle_evaluation("finalize", source_path, eval_info),
            term,
        )
    })
}

/// Run only the stack for `signal` (no top-level communication).
///
/// Used to preview success/blocked stacks for explicit `Error` control actions
/// before committing to the terminal signal.
#[allow(clippy::too_many_arguments)]
fn run_lifecycle_stack_only(
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
        guard.context().context,
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
fn build_lifecycle_stack_context_for_materialized<'a>(
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
        err,
        timing,
        current,
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

/// Capture the lifecycle stack-only `timing`/`current` globals for an event.
///
/// `current.env` is the live process environment and `current.ctx` is the full
/// Darkmatter `ctx.*` namespace, both captured **now** so a side effect or
/// external change since `prepare` is observable through `current.*` at event
/// time. `timing` measures wall-clock elapsed against `loop_start`
/// (`document_ms` and `total_ms`; the harness loop has no sequence-step clock,
/// so `step_ms` stays `None`).
fn capture_lifecycle_globals(
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

/// Run a proxy target document's `initialize` event after re-parsing its
/// lifecycle, respecting target-side `Skip`, `Proxy`, `Error`, and action-error
/// routing.
///
/// Called when `proxy_tracking.pending` is consumed at the top of the harness
/// loop. Resets the guard so the target gets a fresh `initialize` emission
/// before pre-flight checks run.
#[allow(clippy::too_many_arguments)]
fn run_target_initialize(
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    loop_start: std::time::Instant,
) -> TargetInitializeAction {
    lifecycle_guard.reset_for_proxy();
    let outcome = run_lifecycle_event(
        lifecycle_guard,
        LifecycleSignal::Initialize,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        None,
        loop_start,
    );
    // A late-binding evaluation error on the target's `initialize` routes
    // through `failure` → `finalize` and aborts the hand-off (Decision #5).
    if let Some(err) = handle_setup_evaluation_error(
        &outcome,
        "initialize",
        lifecycle_guard,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        loop_start,
    ) {
        return TargetInitializeAction::Abort(err);
    }
    if let Some(control) = outcome.control.as_ref() {
        match control {
            StackControl::Skip => TargetInitializeAction::ExitCleanly,
            StackControl::Error { reason } => {
                let msg = reason
                    .clone()
                    .unwrap_or_else(|| "lifecycle initialize error".to_string());
                let action_error = LifecycleErrorInfo::from_action_failure("error", msg.clone());
                if let Some(ce) = emit_failure_finalize_with_err(
                    lifecycle_guard,
                    materialized,
                    source_path,
                    repo_root,
                    term,
                    effect_engine,
                    &action_error,
                    loop_start,
                ) {
                    return TargetInitializeAction::Abort(ce.into());
                }
                TargetInitializeAction::Abort(eyre!(msg))
            }
            StackControl::Proxy { target } => {
                let resolved = match claudine::composition::resolve_proxy_target(
                    target,
                    source_path,
                    repo_root,
                ) {
                    Ok(path) => path,
                    Err(e) => {
                        return TargetInitializeAction::Abort(eyre!(
                            "lifecycle initialize proxy: {e}"
                        ))
                    }
                };
                TargetInitializeAction::Repoint { resolved }
            }
            StackControl::Stop => TargetInitializeAction::Proceed,
            StackControl::Retry { .. }
            | StackControl::Resume { .. }
            | StackControl::Defer { .. } => TargetInitializeAction::Abort(eyre!(
                "lifecycle control action {control:?} is not valid at initialize"
            )),
        }
    } else if outcome.routes_to_failure(LifecycleSignal::Initialize) {
        let err = outcome.action_error.as_ref();
        // `emit_failure_finalize_with_err` requires a non-optional error; when
        // the outcome had no action_error, synthesize one so failure/finalize
        // still run with an `err` global.
        let synthetic =
            LifecycleErrorInfo::from_action_failure("error", "lifecycle initialize failed");
        let err_ref = err.cloned().unwrap_or(synthetic);
        if let Some(ce) = emit_failure_finalize_with_err(
            lifecycle_guard,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            &err_ref,
            loop_start,
        ) {
            return TargetInitializeAction::Abort(ce.into());
        }
        TargetInitializeAction::Abort(eyre!("lifecycle initialize failed"))
    } else {
        TargetInitializeAction::Proceed
    }
}

/// Per-control retry/resume budget tracking for one `run_harness_loop` call.
///
/// A lifecycle `retry`/`resume` control declares `max_attempts` relative to
/// the attempt at which it first fires. The budget (the absolute attempt
/// ceiling) is computed once on first firing and reused so the ceiling does
/// not drift as the attempt counter advances.
#[derive(Default)]
struct ControlBudgets {
    retry: Option<u32>,
    resume: Option<u32>,
}

impl ControlBudgets {
    /// Return (and lazily establish) the budget for a control firing at
    /// `attempt`. `max_attempts` is the additional-attempts parameter.
    fn budget_for(slot: &mut Option<u32>, attempt: u32, max_attempts: u32) -> u32 {
        *slot.get_or_insert_with(|| control_budget_for(attempt, max_attempts))
    }
}

/// Proxy hand-off bookkeeping for one `run_harness_loop` call.
///
/// `chain` is the ordered list of resolved documents visited by proxy,
/// including the originating document once the first hand-off is accepted; it
/// drives the cycle/hop-limit guard.
/// `pending` is set by the `Proxy` dispatch arm and consumed at the loop top,
/// signalling that the guard's lifecycle config must be re-parsed from the
/// newly materialized target before its events fire.
#[derive(Default)]
struct ProxyTracking {
    chain: Vec<std::path::PathBuf>,
    pending: bool,
}

/// What the loop should do after dispatching a terminal-event control.
#[derive(Debug)]
enum TerminalControlAction {
    /// No actionable control (Stop/Skip/Error/None) — fall through to the
    /// loop's normal terminal handling (finalize + return).
    Fallthrough,
    /// Re-enter the loop for another attempt at `next_attempt`.
    Continue { next_attempt: u32 },
    /// A control could not be honored; abort the run with this error.
    Abort(color_eyre::eyre::Report),
}

#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
const REQUEUE_SESSION_ID: &str = "claudine-deferred-execution";
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
const REQUEUE_SOURCE: &str = "claudine.lifecycle.requeue";
/// Environment variable that overrides the directory used by the
/// rendezvous deferred-queue fallback file. When unset the fallback
/// lives under `<config_dir>/claudine/rendezvous/deferred-queue.jsonl`.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
const REQUEUE_FALLBACK_DIR_ENV: &str = "CLAUDINE_RENDEZVOUS_FALLBACK_DIR";
/// Fallback file name appended to the resolved fallback directory when no
/// rendezvous daemon is reachable. Each line is the JSON serialization of
/// the same `AppendEntryRequest` shape the daemon would have received, so a
/// future daemon can drain it verbatim.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
const REQUEUE_FALLBACK_FILE_NAME: &str = "deferred-queue.jsonl";

/// Errors that can occur while persisting a `requeue(...)` deferred-prompt
/// entry.
///
/// The contract is daemon-first with a durable fallback (see
/// [`enqueue_requeue_entry`]). Only failures that lose the prompt surface
/// here; a daemon connect/append failure that successfully falls back to the
/// JSONL file is `Ok(())`.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
enum RequeueEnqueueError {
    #[error("failed to connect to rendezvous daemon at {endpoint}: {source}")]
    Connect {
        endpoint: std::path::PathBuf,
        #[source]
        source: rendezvous_client::ConnectError,
    },
    #[error("rendezvous append-entry RPC failed: {0}")]
    Rpc(#[from] tonic::Status),
    #[error("failed to serialize requeue metadata: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("no Tokio runtime is available for rendezvous enqueue")]
    NoRuntime,
    /// The daemon was unreachable AND the durable fallback write failed.
    /// The prompt is lost; surface this to the user as a hard failure.
    #[error(
        "rendezvous daemon unreachable ({daemon_error}) and fallback write to {path} failed: {source}"
    )]
    FallbackWrite {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
        daemon_error: String,
    },
}

/// Resolve the durable fallback directory for the deferred-prompt queue.
///
/// Order:
/// 1. `CLAUDINE_RENDEZVOUS_FALLBACK_DIR` env var (test isolation / power
///    users).
/// 2. `<config_dir>/claudine/rendezvous/` via the `dirs` crate (per-user,
///    cross-platform: `~/Library/Application Support` on macOS,
///    `~/.config` on Linux, `%APPDATA%` on Windows).
/// 3. `~/.claudine/rendezvous/` as a last-resort home-dir fallback.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
fn requeue_fallback_dir() -> Option<std::path::PathBuf> {
    if let Some(explicit) = std::env::var_os(REQUEUE_FALLBACK_DIR_ENV)
        && !explicit.is_empty()
    {
        return Some(std::path::PathBuf::from(explicit));
    }
    let base = dirs::config_dir().or_else(dirs::home_dir)?;
    Some(base.join("claudine").join("rendezvous"))
}

/// Resolve the absolute fallback file path (without touching the disk).
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
fn requeue_fallback_path() -> Option<std::path::PathBuf> {
    requeue_fallback_dir().map(|d| d.join(REQUEUE_FALLBACK_FILE_NAME))
}

/// Append one deferred-prompt entry to the durable fallback JSONL file as a
/// single line. Creates the parent directory if needed. Each line carries
/// the same shape as the `AppendEntryRequest` the daemon would have
/// received so a future daemon can drain the file verbatim.
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
fn write_requeue_fallback(
    path: &Path,
    request: &rendezvous_core::AppendEntryRequest,
) -> std::result::Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut entry = serde_json::Map::new();
    entry.insert(
        "owner_node_id".to_string(),
        serde_json::Value::String(request.owner_node_id.clone()),
    );
    entry.insert(
        "session_id".to_string(),
        serde_json::Value::String(request.session_id.clone()),
    );
    entry.insert(
        "source".to_string(),
        serde_json::Value::String(request.source.clone()),
    );
    entry.insert(
        "level".to_string(),
        serde_json::Value::String(request.level.clone()),
    );
    entry.insert(
        "message".to_string(),
        serde_json::Value::String(request.message.clone()),
    );
    // `metadata_json` arrives as a JSON-encoded string; embed it as a parsed
    // object so the line is human-readable and round-trips cleanly. Fall
    // back to the raw string if the daemon-side producer emitted non-object
    // JSON.
    let metadata_value = serde_json::from_str::<serde_json::Value>(&request.metadata_json)
        .unwrap_or_else(|_| serde_json::Value::String(request.metadata_json.clone()));
    entry.insert("metadata_json".to_string(), metadata_value);
    let line = serde_json::Value::Object(entry);
    let mut serialized = serde_json::to_string(&line)?;
    serialized.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    use std::io::Write;
    file.write_all(serialized.as_bytes())?;
    file.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
async fn enqueue_requeue_entry_async(
    provider: Provider,
    prompt_state: &HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    delay: &str,
    reason: Option<&str>,
) -> std::result::Result<(), RequeueEnqueueError> {
    let endpoint = rendezvous_core::socket::default_socket_path();
    let metadata = serde_json::json!({
        "kind": "claudine.lifecycle.requeue",
        "provider": provider.as_slug(),
        "prompt_mode": harness_prompt_mode_label(prompt_state.mode),
        "source_path": prompt_state.source_path,
        "original_ref": prompt_state.original_ref,
        "repo_root": repo_root,
        "delay": delay,
        "reason": reason,
        "prompt": materialized.prompt,
        "frontmatter": materialized.frontmatter,
    });
    let request = rendezvous_core::AppendEntryRequest {
        owner_node_id: String::new(),
        session_id: REQUEUE_SESSION_ID.to_string(),
        source: REQUEUE_SOURCE.to_string(),
        level: "info".to_string(),
        message: format!(
            "deferred {} for {}",
            prompt_state.source_path.display(),
            delay
        ),
        metadata_json: serde_json::to_string(&metadata)?,
    };
    // Daemon-first: try the live rendezvous daemon over the platform's IPC
    // transport (UDS on unix, named pipe on windows). On any connect or
    // append failure, durably persist the entry to the local fallback file
    // so the prompt is never lost. Only a fallback write failure surfaces.
    match try_enqueue_via_daemon(endpoint.clone(), &request).await {
        Ok(()) => Ok(()),
        Err(daemon_err) => {
            let Some(fallback_path) = requeue_fallback_path() else {
                // No writable fallback location: surface the daemon error.
                return Err(daemon_err);
            };
            let daemon_error = daemon_err.to_string();
            write_requeue_fallback(&fallback_path, &request).map_err(|source| {
                RequeueEnqueueError::FallbackWrite {
                    path: fallback_path.clone(),
                    source,
                    daemon_error: daemon_error.clone(),
                }
            })?;
            tracing::warn!(
                target: "claudine::lifecycle::requeue",
                daemon_error = %daemon_error,
                fallback_path = %fallback_path.display(),
                "rendezvous daemon unreachable; deferred prompt persisted to fallback file",
            );
            Ok(())
        }
    }
}

/// Attempt the live-daemon append-entry RPC. The connector dispatches by
/// platform (`connect_uds` on unix, `connect_named_pipe` on windows).
#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
async fn try_enqueue_via_daemon(
    endpoint: std::path::PathBuf,
    request: &rendezvous_core::AppendEntryRequest,
) -> std::result::Result<(), RequeueEnqueueError> {
    let mut client = rendezvous_client::connect(endpoint.clone())
        .await
        .map_err(|source| RequeueEnqueueError::Connect {
            endpoint: endpoint.clone(),
            source,
        })?;
    client.append_entry(request.clone()).await?;
    Ok(())
}

#[allow(dead_code)] // retained for the future rendezvous deferred-execution backend
fn enqueue_requeue_entry(
    provider: Provider,
    prompt_state: &HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    delay: &str,
    reason: Option<&str>,
) -> std::result::Result<(), RequeueEnqueueError> {
    let handle =
        tokio::runtime::Handle::try_current().map_err(|_| RequeueEnqueueError::NoRuntime)?;
    tokio::task::block_in_place(|| {
        handle.block_on(enqueue_requeue_entry_async(
            provider,
            prompt_state,
            materialized,
            repo_root,
            delay,
            reason,
        ))
    })
}

/// What the loop should do after running a proxy target document's
/// `initialize` event.
#[derive(Debug)]
enum TargetInitializeAction {
    /// Target's `initialize` completed cleanly; proceed to pre-flight/start.
    Proceed,
    /// Target's `initialize` opted out via `skip`; exit the run cleanly.
    ExitCleanly,
    /// Target's `initialize` could not be honored; abort with this error.
    Abort(color_eyre::eyre::Report),
    /// Target's `initialize` proxied again; repoint the loop and continue.
    Repoint { resolved: std::path::PathBuf },
}

/// Translate **any** lifecycle event's stack [`StackControl`] into a loop
/// action, applying the retry/resume/proxy/requeue runtime effect.
///
/// This is the single, **event-agnostic** handler-dispatch path shared by every
/// event whose stack can carry a recovery handler (`blocked`/`failure`/
/// `finalize`). Which control is *valid* in which event is the parse-time
/// pre-scan's job, so this function does not branch on the event; it derives
/// `Retry` re-entry from the guard's `provider_launched()` state, not the
/// signal.
///
/// Reuses the existing redirect/resume substrate: a retry bumps the attempt
/// and `continue`s; a resume seeds `next_resume_session_id` +
/// `next_prompt_override`; a proxy swaps `source_path`/`original_ref` and
/// resets the guard for a fresh `initialize`; a requeue records the
/// materialized prompt in rendezvous and exits the current run.
#[allow(clippy::too_many_arguments)]
fn dispatch_terminal_control(
    outcome: &LifecycleEventOutcome,
    attempt: u32,
    budgets: &mut ControlBudgets,
    session_id: Option<&str>,
    profile: &dyn super::super::profile::WrapperProfile,
    // `_provider` / `_materialized` are retained on the signature for when
    // `defer` is wired to the rendezvous deferred-execution backend (it will
    // need them to enqueue); unused while `defer` returns not-implemented.
    _provider: Provider,
    prompt_state: &mut HarnessPromptState,
    _materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    proxy: &mut ProxyTracking,
    term: &Terminal,
    show_checks: bool,
) -> TerminalControlAction {
    let Some(control) = outcome.control.as_ref() else {
        return TerminalControlAction::Fallthrough;
    };

    // Compute the control budget (only retry/resume consume one).
    let budget = match control {
        StackControl::Retry { max_attempts, .. } => {
            ControlBudgets::budget_for(&mut budgets.retry, attempt, *max_attempts)
        }
        StackControl::Resume { max_attempts, .. } => {
            ControlBudgets::budget_for(&mut budgets.resume, attempt, *max_attempts)
        }
        _ => 0,
    };

    let dispatch = decide_control(
        control,
        attempt,
        budget,
        session_id.is_some(),
        lifecycle_guard.provider_launched(),
    );

    match dispatch {
        ControlDispatch::Stop | ControlDispatch::Exhausted => TerminalControlAction::Fallthrough,
        ControlDispatch::Retry {
            delay,
            reenter_preflight,
        } => {
            if show_checks {
                let what = if reenter_preflight { "pre-flight" } else { "the agent" };
                claudine::harness::report::report_lifecycle_recovery(
                    &format!("lifecycle retry: re-running {what} (attempt {})", attempt + 1),
                    term,
                );
            }
            if !delay.is_zero() {
                std::thread::sleep(delay);
            }
            // The terminal event already fired for this iteration; reset the
            // guard's per-iteration state so the retried attempt can emit its
            // own start/terminal/finalize without the terminal slot being
            // suppressed as already-taken.
            lifecycle_guard.reset_for_next_iteration();
            TerminalControlAction::Continue {
                next_attempt: attempt + 1,
            }
        }
        ControlDispatch::Resume { message } => {
            // Honor the provider's resume capability. The CLI-side resume gate
            // surfaces a clear error when the provider cannot resume or the
            // session id is missing.
            if let Err(e) = super::super::resume::check_resume_support(
                &profile.provider().to_string(),
                profile.supports_resume(),
                session_id,
            ) {
                return TerminalControlAction::Abort(eyre!("{e}"));
            }
            prompt_state.next_resume_session_id = session_id.map(|id| id.to_string());
            prompt_state.next_prompt_override = Some(message);
            prompt_state.prompt_tail.clear();
            if show_checks {
                claudine::harness::report::report_lifecycle_recovery(
                    &format!("lifecycle resume: resuming session (attempt {})", attempt + 1),
                    term,
                );
            }
            // Reset per-iteration guard state (the failure terminal already
            // fired) so the resumed attempt emits its own lifecycle events.
            lifecycle_guard.reset_for_next_iteration();
            TerminalControlAction::Continue {
                next_attempt: attempt + 1,
            }
        }
        ControlDispatch::ResumeWithoutSession => {
            TerminalControlAction::Abort(
                CompositionError::LifecycleResumeWithoutSession {
                    source_path: prompt_state.source_path.clone(),
                }
                .into(),
            )
        }
        ControlDispatch::Proxy { target } => {
            let resolve_ctx = claudine::harness::HarnessResolutionContext {
                source_path: &prompt_state.source_path,
                repo_root,
            };
            let resolved = match claudine::harness::resolve_harness_path(&target, &resolve_ctx) {
                Ok(path) => path,
                Err(e) => return TerminalControlAction::Abort(eyre!("lifecycle proxy: {e}")),
            };
            // Cycle / hop-limit guard: a `failure` stack that proxies back to a
            // document whose own `failure` stack proxies again would loop
            // forever. Reject a self-proxy, an A->B->A cycle, or an
            // over-long chain with a typed error rather than hanging.
            if !proxy.chain.iter().any(|p| p == &prompt_state.source_path) {
                proxy.chain.push(prompt_state.source_path.clone());
            }
            if !claudine::composition::proxy_handoff_allowed(&proxy.chain, &resolved) {
                return TerminalControlAction::Abort(
                    CompositionError::LifecycleProxyCycle {
                        source_path: prompt_state.source_path.clone(),
                        target: target.clone(),
                        chain: proxy
                            .chain
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect(),
                        limit: claudine::composition::MAX_PROXY_HOPS,
                    }
                    .into(),
                );
            }
            // Swap the running document for the target and reset per-iteration
            // guard state so the target runs a fresh `initialize`/pre-flight.
            prompt_state.source_path = resolved.clone();
            prompt_state.original_ref = target.clone();
            prompt_state.prompt_tail.clear();
            prompt_state.next_prompt_override = None;
            prompt_state.next_resume_session_id = None;
            lifecycle_guard.reset_for_proxy();
            // Record the hop and flag that the loop top must re-parse the
            // guard's lifecycle config from the target's frontmatter — without
            // this the target's events would run against the proxying
            // document's lifecycle (and the original `failure`/`proxy` stack
            // would re-fire, looping forever).
            proxy.chain.push(resolved.clone());
            proxy.pending = true;
            if show_checks {
                claudine::harness::report::report_lifecycle_recovery(
                    &format!("lifecycle proxy: handing off to {}", resolved.display()),
                    term,
                );
            }
            // Re-enter at attempt 1 so the target document gets a clean
            // pre-flight / freeze cycle rather than inheriting the proxying
            // document's attempt count.
            TerminalControlAction::Continue { next_attempt: 1 }
        }
        ControlDispatch::Defer { .. } => {
            // `defer` (deferred re-execution) is accepted in every event, but its
            // runtime home — the rendezvous deferred-execution scheduler — is not
            // ready to receive prompts yet, so surface a clear "not implemented"
            // error rather than enqueuing. The rendezvous enqueue machinery
            // (`enqueue_requeue_entry`) is retained for when it lands.
            TerminalControlAction::Abort(
                CompositionError::LifecycleDeferNotImplemented {
                    source_path: prompt_state.source_path.clone(),
                }
                .into(),
            )
        }
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
fn surface_catch_evaluation_error(
    source_path: &Path,
    failure_outcome: Option<&LifecycleEventOutcome>,
    finalize_outcome: Option<&LifecycleEventOutcome>,
    early: CompositionError,
    term: &Terminal,
) -> color_eyre::eyre::Report {
    let surfaced = if let Some(fin_info) =
        finalize_outcome.and_then(|o| o.evaluation_error.as_ref())
    {
        crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            source_path,
            "finalize",
            fin_info,
            term,
        )
    } else if let Some(fail_info) = failure_outcome.and_then(|o| o.evaluation_error.as_ref()) {
        crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            source_path,
            "failure",
            fail_info,
            term,
        )
    } else {
        // No catch-event raise: the original (already-emitted) error surfaces.
        early
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
fn handle_terminal_evaluation_error(
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
fn handle_setup_evaluation_error(
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

/// Run the `finalize` event, then dispatch any recovery control action its
/// stack ended in (`retry`/`resume`/`requeue`/`proxy`).
///
/// `finalize` is the optional-error terminal event, so it doubles as a
/// last-chance recovery surface: a `finalize.stack` that decides the work was
/// not actually done (typically guarded by `when: "err"`) can recover exactly
/// as the `failure` event can. The returned [`TerminalControlAction`] tells the
/// caller whether to re-enter the loop (`Continue`), propagate a hard error
/// (`Abort`), or proceed to its normal terminal return (`Fallthrough`).
///
/// On a recovery re-entry the dispatch resets the guard's per-iteration state,
/// so the next attempt emits its own `start`/terminal/`finalize` signals.
#[allow(clippy::too_many_arguments)]
fn run_finalize_with_recovery(
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
    loop_start: std::time::Instant,
    attempt: u32,
    budgets: &mut ControlBudgets,
    session_id: Option<&str>,
    profile: &dyn super::super::profile::WrapperProfile,
    provider: Provider,
    prompt_state: &mut HarnessPromptState,
    proxy: &mut ProxyTracking,
    show_checks: bool,
) -> TerminalControlAction {
    let finalize_outcome = run_lifecycle_event(
        lifecycle_guard,
        LifecycleSignal::Finalize,
        materialized,
        &prompt_state.source_path,
        repo_root,
        term,
        effect_engine,
        err,
        loop_start,
    );
    // An evaluation error raised *inside* `finalize` halts the run, but must
    // not re-enter `finalize` (Decision #3 / re-entry guard): surface it as a
    // hard abort directly rather than recursing through the recovery path.
    if let Some(info) = finalize_outcome.evaluation_error.as_ref() {
        // A raise inside `finalize` halts; no further events fire. Emit the
        // styled block now (Decision #2) and mark it already-emitted.
        return TerminalControlAction::Abort(
            crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                CompositionError::lifecycle_evaluation(
                    "finalize",
                    &prompt_state.source_path,
                    info,
                ),
                term,
            )
            .into(),
        );
    }
    dispatch_terminal_control(
        &finalize_outcome,
        attempt,
        budgets,
        session_id,
        profile,
        provider,
        prompt_state,
        materialized,
        repo_root,
        lifecycle_guard,
        proxy,
        term,
        show_checks,
    )
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(crate) fn run_harness_loop(
    provider: Provider,
    profile: &dyn super::super::profile::WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    effective_non_interactive: bool,
    cli_timeout: Option<String>,
    cli_step_timeout: Option<String>,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    prompt_state: &mut HarnessPromptState,
    repo_root: Option<&Path>,
    shell_options: claudine::harness::ShellApprovalOptions,
    use_structured: bool,
    structured_codex_output: Option<&super::super::policy::StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    term: &Terminal,
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    // Set when an `initialize`-stack `proxy(...)` already redirected to this
    // target document upstream. Seeds the proxy chain (so a proxy back to the
    // original is caught as a cycle) and triggers the loop-top lifecycle
    // re-parse so the guard adopts the target's lifecycle.
    initial_proxy_target: Option<&Path>,
    // When `true`, every structured-stream attempt in the harness loop
    // emits the prompt-scoped timing header and — if the parsed plan
    // carries `timeout_warn` / `step_timeout_warn` — their fire-once
    // warning lines. Wrapper passthrough callers with no prompt file
    // pass `false` to suppress the header entirely; composition callers
    // pass `true`.
    emit_prompt_timing: bool,
) -> Result<(i32, Option<crate::perf::AgentExecutionPerf>, Option<IterationSummarySignals>)> {
    let mutation_root = repo_root.unwrap_or(child_cwd).to_path_buf();
    let effect_engine = EffectEngine::builder()
        .mutation_root(&mutation_root)
        .auto_rehash(false)
        .build();
    let mut harness_context = CachedHarnessLoopContext::with_shell_options(
        &prompt_state.source_path,
        repo_root,
        shell_options,
    );
    let mut attempt = 1u32;
    let mut initial_materialized = initial_materialized;
    let mut harness_perf: Option<crate::perf::AgentExecutionPerf> = None;
    let mut terminal_signals: Option<IterationSummarySignals> = None;
    let mut _harness_attempts: usize = 0;
    // Run-level wall-clock anchor for lifecycle `timing.{document_ms,total_ms}`
    // emitted at each event. Captured before the first attempt so it spans the
    // whole harness loop (all retry/resume iterations).
    let loop_start = std::time::Instant::now();
    // Per-control retry/resume ceilings established on first firing of a
    // lifecycle `retry`/`resume` control in a terminal stack.
    let mut control_budgets = ControlBudgets::default();
    // Proxy hand-off chain + pending flag. A `Some(target)` initial proxy
    // (an `initialize`-stack `proxy(...)`) already swapped `source_path`
    // upstream, so seed the chain with it and flag a re-parse: the guard was
    // built against the *original* document's lifecycle and must adopt the
    // target's before its events fire.
    let mut proxy_tracking = ProxyTracking::default();
    if let Some(initial_target) = initial_proxy_target {
        if !proxy_tracking
            .chain
            .iter()
            .any(|p| p == lifecycle_guard.context().source_path)
        {
            proxy_tracking
                .chain
                .push(lifecycle_guard.context().source_path.to_path_buf());
        }
        proxy_tracking.chain.push(initial_target.to_path_buf());
        proxy_tracking.pending = true;
    }

    loop {
        let _attempt_cycle_span = info_span!(
            "harness_attempt_cycle",
            provider = %provider,
            attempt,
            prompt_mode = harness_prompt_mode_label(prompt_state.mode),
            source_path = %prompt_state.source_path.display(),
        )
        .entered();
        harness_context.refresh(&prompt_state.source_path, repo_root);
        let materialized = if let Some(seed) = initial_materialized.take() {
            seed
        } else {
            info_span!(
                "harness_materialize_prompt",
                attempt,
                source_path = %prompt_state.source_path.display(),
            )
            .in_scope(|| materialize_harness_prompt(prompt_state, repo_root, child_cwd))
            .map_err(|e| {
                let err_info =
                    LifecycleErrorInfo::from_action_failure("materialize", e.to_string());
                // Materialization failed, so there is no prompt to carry into the
                // stack context. Synthesize an empty one: the guard still holds the
                // (proxying/original) document's parsed lifecycle, so its
                // blocked/finalize stacks fire. `frontmatter: Null` makes the
                // stack-context builder fall back to an empty frontmatter map, so
                // any `when:` referencing frontmatter resolves against {} — correct,
                // because the real frontmatter never materialized.
                let empty = MaterializedHarnessPrompt {
                    frontmatter: serde_json::Value::Null,
                    prompt: String::new(),
                    env_overrides: Vec::new(),
                    inline_closure_plan: None,
                    live_frontmatter: MaterializedHarnessPrompt::live_cell_from(
                        &serde_json::Value::Null,
                    ),
                };
                // A lifecycle evaluation error raised by the synthesized empty
                // prompt's blocked/finalize stack takes precedence over the
                // original materialize error — the lifecycle raise is the more
                // actionable diagnosis and must halt the run.
                match emit_blocked_finalize_with_err(
                    lifecycle_guard,
                    &empty,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    &err_info,
                    loop_start,
                ) {
                    Some(ce) => ce.into(),
                    None => eyre!("{e}"),
                }
            })?
        };

        // A proxy hand-off (from `initialize`, `blocked`, or `failure`)
        // swapped `source_path` to the target. The guard still holds the
        // proxying document's lifecycle, so repoint it at the target's —
        // parsed from the freshly materialized target frontmatter — before any
        // of the target's events fire. Without this the target's own
        // `start`/`success`/`finalize` never run and the proxying document's
        // `failure`/`proxy` stack re-fires, looping forever.
        if proxy_tracking.pending {
            proxy_tracking.pending = false;
            match claudine::composition::parse_lifecycle_config(
                &materialized.frontmatter,
                &prompt_state.source_path,
            ) {
                Ok(target_lifecycle) => lifecycle_guard.set_config(target_lifecycle),
                Err(e) => {
                    let err_info = LifecycleErrorInfo::from_composition_error(&e);
                    // A lifecycle evaluation error raised by the target's
                    // blocked/finalize stack takes precedence over the original
                    // target-lifecycle parse error — the lifecycle raise is the
                    // more actionable diagnosis and must halt the run.
                    return Err(emit_blocked_finalize_with_err(
                        lifecycle_guard,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        &err_info,
                        loop_start,
                    )
                    .map(color_eyre::eyre::Report::from)
                    .unwrap_or_else(|| eyre!("{e}")));
                }
            }
            // The proxied document enters at its own `initialize` — a fresh
            // prompt run. Reset the guard and emit the target's `initialize`
            // before pre-flight checks, honoring target-side `Skip`/`Proxy`/
            // `Error` logic.
            match run_target_initialize(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                loop_start,
            ) {
                TargetInitializeAction::Proceed => {}
                TargetInitializeAction::ExitCleanly => {
                    return Ok((0, None, None));
                }
                TargetInitializeAction::Abort(e) => return Err(e),
                TargetInitializeAction::Repoint { resolved } => {
                    if !proxy_tracking
                        .chain
                        .iter()
                        .any(|p| p == &prompt_state.source_path)
                    {
                        proxy_tracking.chain.push(prompt_state.source_path.clone());
                    }
                    if !claudine::composition::proxy_handoff_allowed(
                        &proxy_tracking.chain,
                        &resolved,
                    ) {
                        return Err(CompositionError::LifecycleProxyCycle {
                            source_path: prompt_state.source_path.clone(),
                            target: resolved.display().to_string(),
                            chain: proxy_tracking
                                .chain
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect(),
                            limit: claudine::composition::MAX_PROXY_HOPS,
                        }
                        .into());
                    }
                    prompt_state.source_path = resolved.clone();
                    prompt_state.original_ref = resolved.display().to_string();
                    prompt_state.prompt_tail.clear();
                    prompt_state.next_prompt_override = None;
                    prompt_state.next_resume_session_id = None;
                    proxy_tracking.chain.push(resolved.clone());
                    proxy_tracking.pending = true;
                    if show_checks {
                        claudine::harness::report::report_lifecycle_recovery(
                            &format!("lifecycle proxy: handing off to {}", resolved.display()),
                            term,
                        );
                    }
                    // Re-enter at attempt 1 so the target document gets a clean
                    // pre-flight / freeze cycle rather than inheriting the
                    // proxying document's attempt count.
                    attempt = 1;
                    continue;
                }
            }
        }

        let plan = info_span!(
            "harness_plan_parse",
            attempt,
            source_path = %prompt_state.source_path.display(),
        )
        .in_scope(|| {
            claudine::harness::parse_harness_plan(
                &materialized.frontmatter,
                &prompt_state.source_path,
            )
        })
        .map_err(|e| {
            let err_info = LifecycleErrorInfo::from_harness_error(&e);
            // A lifecycle evaluation error raised by the blocked/finalize stack
            // takes precedence over the original harness-plan parse error —
            // the lifecycle raise is the more actionable diagnosis and must
            // halt the run.
            match emit_blocked_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                Some(ce) => ce.into(),
                None => eyre!("{e}"),
            }
        })?;

        // Source-file existence reporting
        if show_checks {
            claudine::harness::report::report_source_file(
                &prompt_state.original_ref,
                &prompt_state.source_path,
                term,
            );
        }
        if !prompt_state.source_path.exists() {
            if show_checks {
                claudine::harness::report::report_unhandled_failure(
                    "source file does not exist — cannot proceed",
                    term,
                );
            }
            let err_info = LifecycleErrorInfo::from_action_failure(
                "missing_source",
                format!(
                    "source file does not exist: {}",
                    prompt_state.source_path.display()
                ),
            );
            // A lifecycle evaluation error raised by the blocked/finalize stack
            // takes precedence over the original missing-source error — the
            // lifecycle raise is the more actionable diagnosis and must halt
            // the run non-zero instead of being swallowed.
            if let Some(ce) = emit_blocked_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                return Err(ce.into());
            }
            return Err(eyre!(
                "source file does not exist: {}",
                prompt_state.source_path.display()
            ));
        }

        // The parsed harness plan is used for shell audit and timeout
        // configuration. Pre/post validation checks have been removed.

        // Shell audit preflight.
        //
        // Composition flows (Compose/Inline) preflight all shell commands
        // before the provider starts — template directives during composition
        // and harness commands in execute_composition_request.  The per-
        // attempt audit below is redundant for those modes because:
        //
        //   1. source_text is None, so source-page ::shell directives are
        //      excluded (they were discovered via Darkmatter's graph walker
        //      during composition, which respects ::block when="false").
        //   2. Harness commands were approved and cached during the
        //      composition preflight pass.
        //   3. The approval handler is frozen after attempt 1, so no new
        //      interactive prompts are possible.
        //
        // Only Passthrough mode needs the per-attempt audit because it reads
        // raw source text and the source file may change between
        // redirect/retry iterations.
        if matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            let source_text = std::fs::read_to_string(&prompt_state.source_path).ok();

            let auditable =
                claudine::harness::collect_auditable_commands(source_text.as_deref())?;

            let audit_report = info_span!(
                "harness_shell_audit",
                attempt,
                command_count = auditable.len(),
            )
            .in_scope(|| {
                claudine::harness::audit_shell_commands(&auditable, harness_context.shell_options())
            });

            if show_checks {
                claudine::harness::report::report_shell_audit_header(
                    audit_report.outcomes.len(),
                    term,
                );
                claudine::harness::report::report_shell_audit_outcomes(&audit_report, term);
            }

            if !audit_report.all_passed() {
                let failed = audit_report.failures();
                let msg = format!(
                    "shell audit failed: {} denied directive(s) in source page",
                    failed.len()
                );
                if show_checks {
                    claudine::harness::report::report_unhandled_failure(
                        "shell audit failed for source-page directives — cannot proceed",
                        term,
                    );
                }
                let err_info = LifecycleErrorInfo::from_action_failure("shell_audit", &msg);
                // A lifecycle evaluation error raised by the blocked/finalize
                // stack takes precedence over the original shell-audit error —
                // the lifecycle raise is the more actionable diagnosis and must
                // halt the run non-zero instead of being swallowed.
                if let Some(ce) = emit_blocked_finalize_with_err(
                    lifecycle_guard,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    &err_info,
                    loop_start,
                ) {
                    return Err(ce.into());
                }
                return Err(eyre!(msg));
            }
        }

        // Composition flows resolved all shell approvals during preflight.
        // Freeze the approval set so redirect/retry iterations cannot
        // trigger new interactive prompts — only cached/whitelisted
        // commands pass; new uncached commands are denied.  Passthrough
        // mode has no prior preflight so its handler stays active.
        if attempt == 1 && !matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            harness_context.freeze_shell_approvals();
        }

        // Pre-check validation has been removed. Shell audit still runs above
        // for Passthrough mode; composition flows audit during preflight.

        // Emit start lifecycle signal before the first provider launch.
        let start_outcome = run_lifecycle_event(
            lifecycle_guard,
            LifecycleSignal::Start,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            None,
            loop_start,
        );
        // A late-binding evaluation error on `start` (a crashed `when:` guard or
        // interpolation) routes through `failure` → `finalize` like any other
        // setup failure and halts non-zero (Decision #5). Checked before the
        // control match because an evaluation raise leaves `control` `None`.
        if let Some(err) = handle_setup_evaluation_error(
            &start_outcome,
            "start",
            lifecycle_guard,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            loop_start,
        ) {
            return Err(err);
        }
        if let Some(ref control) = start_outcome.control {
            match control {
                StackControl::Error { reason } => {
                    let msg = reason
                        .clone()
                        .unwrap_or_else(|| "lifecycle start error".to_string());
                    let err_info =
                        LifecycleErrorInfo::from_action_failure("error", msg.as_str());
                    if let Some(ce) = emit_failure_finalize_with_err(
                        lifecycle_guard,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        &err_info,
                        loop_start,
                    ) {
                        return Err(ce.into());
                    }
                    return Err(eyre!(msg));
                }
                StackControl::Stop => {}
                // retry/resume/proxy/requeue at `start` dispatch through the
                // uniform path. The provider has not launched yet, so `resume`
                // surfaces `ResumeWithoutSession` and `retry` re-enters before
                // the agent runs.
                _ => match dispatch_terminal_control(
                    &start_outcome,
                    attempt,
                    &mut control_budgets,
                    None,
                    profile,
                    provider,
                    prompt_state,
                    &materialized,
                    repo_root,
                    lifecycle_guard,
                    &mut proxy_tracking,
                    term,
                    show_checks,
                ) {
                    TerminalControlAction::Continue { next_attempt } => {
                        attempt = next_attempt;
                        continue;
                    }
                    TerminalControlAction::Abort(err) => {
                        // No error info is available at the start-control-abort
                        // point, so `finalize` runs with `None` `err`; but its
                        // own stack may still raise an evaluation error, which
                        // must surface and halt rather than being swallowed by
                        // the abort `return`.
                        let finalize_outcome = run_lifecycle_event(
                            lifecycle_guard,
                            LifecycleSignal::Finalize,
                            &materialized,
                            &prompt_state.source_path,
                            repo_root,
                            term,
                            &effect_engine,
                            None,
                            loop_start,
                        );
                        if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                            return Err(
                                crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                                    CompositionError::lifecycle_evaluation(
                                        "finalize",
                                        &prompt_state.source_path,
                                        eval_info,
                                    ),
                                    term,
                                )
                                .into(),
                            );
                        }
                        return Err(err);
                    }
                    TerminalControlAction::Fallthrough => {}
                },
            }
        }
        if start_outcome.routes_to_failure(LifecycleSignal::Start) {
            // Record the `Failure` terminal signal FIRST, while we still hold
            // `&mut guard`, so the subsequent `Finalize` actually fires. The
            // error-carrying context built below immutably borrows
            // `guard.emitter()`/`guard.context()`, so recording must happen
            // before the borrow split. Skipping this (calling `run_event_stack`
            // directly) would leave `terminal_emitted` false and silently
            // suppress `finalize`.
            if lifecycle_guard.record_event_emission(LifecycleSignal::Failure) {
                let (timing, current) = capture_lifecycle_globals(
                    &prompt_state.source_path,
                    repo_root,
                    lifecycle_guard.context().launch_area,
                    loop_start,
                );
                let ctx = build_lifecycle_stack_context_for_materialized(
                    LifecycleSignal::Failure,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    lifecycle_guard.context().launch_area,
                    lifecycle_guard.context().context,
                    term,
                    lifecycle_guard.emitter(),
                    lifecycle_guard.context().settings,
                    lifecycle_guard.context().messaging,
                    &effect_engine,
                    start_outcome.action_error.as_ref(),
                    Some(&timing),
                    Some(&current),
                );
                let failure_outcome =
                    lifecycle_guard.run_event_stack(LifecycleSignal::Failure, &ctx);
                // If `failure` raised, thread its error (not the original) into
                // finalize so a `finalize.stack` can branch on the failure raise.
                let synthetic =
                    LifecycleErrorInfo::from_action_failure("error", "lifecycle start failed");
                let active_err = failure_outcome
                    .evaluation_error
                    .as_ref()
                    .or(start_outcome.action_error.as_ref())
                    .unwrap_or(&synthetic);
                let finalize_outcome = run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Finalize,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    Some(active_err),
                    loop_start,
                );
                if failure_outcome.evaluation_error.is_some()
                    || finalize_outcome.evaluation_error.is_some()
                {
                    let info = start_outcome
                        .action_error
                        .as_ref()
                        .unwrap_or(&synthetic);
                    // The original `start` failure was not an evaluation error;
                    // a catch event raised. Emit the surfaced evaluation error
                    // now — no further lifecycle events fire (Decision #2).
                    return Err(
                        crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                            CompositionError::catch_evaluation_error(
                                &prompt_state.source_path,
                                "start",
                                info,
                                Some(&failure_outcome),
                                Some(&finalize_outcome),
                            ),
                            term,
                        )
                        .into(),
                    );
                }
            } else {
                run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Finalize,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    start_outcome.action_error.as_ref(),
                    loop_start,
                );
            }
            return Err(eyre!("lifecycle start failed"));
        }

        // Pre-run snapshot capture for post-check comparisons has been
        // removed along with post-check validation.

        let launch = build_harness_launch(
            provider,
            profile,
            base_args,
            base_env,
            prompt_state,
            &materialized,
            effective_non_interactive,
            cli_timeout.clone(),
            plan.timeout,
            cli_step_timeout.clone(),
            plan.step_timeout,
        )
        .map_err(|e| {
            let err_info = LifecycleErrorInfo::from_action_failure("harness_launch", e.to_string());
            // A lifecycle evaluation error raised by the failure/finalize
            // stack takes precedence over the original harness-launch error —
            // the lifecycle raise is the more actionable diagnosis and must
            // halt the run.
            match emit_failure_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                Some(ce) => ce.into(),
                None => eyre!("{e}"),
            }
        })?;
        let _launch_span = info_span!(
            "harness_launch_plan",
            attempt,
            timeout_secs = launch
                .timeout_config
                .timeout
                .map(|d| d.as_secs())
                .unwrap_or(0),
            step_timeout_secs = launch
                .timeout_config
                .step_timeout
                .map(|d| d.as_secs())
                .unwrap_or(0),
        )
        .entered();

        // Build the prompt-scoped timing context for this attempt. The
        // warn thresholds are re-read from each parsed plan so a
        // handler that redirects to a different source document picks
        // up the replacement document's warn values, not the original's.
        let prompt_timing = if emit_prompt_timing {
            Some(super::super::composition::build_prompt_timing_context(
                &prompt_state.source_path,
                repo_root,
                plan.timeout_warn,
                plan.step_timeout_warn,
            ))
        } else {
            None
        };

        let mut child_spawned = false;
        let attempt_result = execute_harness_attempt(
            attempt,
            provider,
            profile,
            binary_path,
            child_cwd,
            &launch,
            prompt_state.mode,
            prompt_state,
            &materialized,
            effective_non_interactive,
            use_structured,
            structured_codex_output,
            stdout_noise,
            stderr_noise,
            suppress_stderr_on_success,
            show_checks,
            stream_verbosity,
            detail_requested,
            env_context,
            dispatch_context,
            term,
            &mut child_spawned,
            prompt_timing,
        );

        // Mark launched as soon as spawn succeeded — before propagating
        // any post-spawn error — so the guard correctly classifies
        // subsequent failures as `Failure` rather than `Blocked`.
        if child_spawned {
            lifecycle_guard.mark_provider_launched();
        }
        // `execute_harness_attempt` can fail before spawning a child (e.g. a
        // malformed runaway `exit_expressions` regex in
        // `resolve_guard_inputs`/`compile_for_model`) or while delivering the
        // prompt. This is still post-`start`, so route through the typed
        // failure + finalize stacks (with `err`) before propagating.
        let (outcome, perf, iteration_signals) = attempt_result
        .map_err(|e| {
            let err_info =
                LifecycleErrorInfo::from_action_failure("harness_attempt", e.to_string());
            // A lifecycle evaluation error raised by the failure/finalize
            // stack takes precedence over the original harness-attempt error —
            // the lifecycle raise is the more actionable diagnosis and must
            // halt the run.
            match emit_failure_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                Some(ce) => ce.into(),
                None => eyre!("{e}"),
            }
        })?;
        if let Some(p) = perf {
            _harness_attempts += 1;
            match harness_perf.as_mut() {
                Some(acc) => {
                    acc.launches += p.launches;
                    acc.total_elapsed += p.total_elapsed;
                    if acc.first_response_latency.is_none() && p.first_response_latency.is_some() {
                        acc.first_response_latency = p.first_response_latency;
                    }
                    if let Some(api) = p.provider_api_duration {
                        acc.provider_api_duration = Some(
                            acc.provider_api_duration
                                .unwrap_or(std::time::Duration::ZERO)
                                + api,
                        );
                    }
                }
                None => {
                    harness_perf = Some(p);
                }
            }
        }

        if outcome.termination == claudine::harness::ProcessTermination::Interrupted {
            // Surface the interrupt to the user before we let the guard
            // close: without this the wrapper would silently return 130
            // and the operator has no feedback that Claudine noticed.
            eprintln!("{}", crate::output::format_user_interrupt_status());
            let err_info = LifecycleErrorInfo::from_action_failure(
                "interrupted",
                "user interrupted the run",
            );
            let failure_outcome = execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            )
            .outcome;
            // A late-binding evaluation error raised *by the failure stack*
            // halts the run: the helper runs `finalize` once with the
            // evaluation error as `err` and returns the typed catch error, so
            // do not also run a separate finalize when it returns `Some`.
            if let Some(err) = handle_terminal_evaluation_error(
                &failure_outcome,
                "failure",
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                loop_start,
            ) {
                return Err(err);
            }
            // `failure` did not raise: run `finalize` once, but a raise inside
            // `finalize` itself still halts the run non-zero rather than being
            // swallowed by the interrupt's `Ok` return.
            let finalize_outcome = run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            );
            if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                return Err(
                    crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                        CompositionError::lifecycle_evaluation(
                            "finalize",
                            &prompt_state.source_path,
                            eval_info,
                        ),
                        term,
                    )
                    .into(),
                );
            }
            terminal_signals = iteration_signals;
            return Ok((outcome.exit_code, harness_perf, terminal_signals));
        }

        if let Some(failure_event) = claudine::harness::classify_failure(&outcome) {
            let message = match failure_event {
                claudine::harness::FailureEvent::Timeout => {
                    format!("provider timed out (attempt {attempt})")
                }
                claudine::harness::FailureEvent::AgentFailure => {
                    format!(
                        "agent exited with error code {} (attempt {attempt})",
                        outcome.exit_code
                    )
                }
                _ => format!("failure on attempt {attempt}"),
            };
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&message, term);
            }
            // Attribute the failure honestly: prefer the per-guard label the
            // outcome already carries (e.g. `step_timeout`, `runaway_repetition`)
            // so a `failure.stack` referencing `err.variant` branches correctly;
            // fall back to `agent_failure` when no structured label exists.
            let err_info = LifecycleErrorInfo::from_action_failure(
                outcome.error_kind.as_deref().unwrap_or("agent_failure"),
                message.as_str(),
            );
            let failure_outcome = execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            )
            .outcome;
            // A late-binding evaluation error raised *by the failure stack*
            // halts the run: surface it, run `finalize` once with the
            // evaluation error as `err`, and return non-zero. It is not a
            // recoverable control, so this precedes control dispatch.
            if let Some(err) = handle_terminal_evaluation_error(
                &failure_outcome,
                "failure",
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                loop_start,
            ) {
                return Err(err);
            }
            // A `failure.stack` may end in a lifecycle control action
            // (retry/resume/requeue/proxy). Dispatch it before finalizing so a
            // re-entry skips finalize for this iteration.
            match dispatch_terminal_control(
                &failure_outcome,
                attempt,
                &mut control_budgets,
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                &materialized,
                repo_root,
                lifecycle_guard,
                &mut proxy_tracking,
                term,
                show_checks,
            ) {
                TerminalControlAction::Continue { next_attempt } => {
                    attempt = next_attempt;
                    continue;
                }
                TerminalControlAction::Abort(err) => {
                    let finalize_outcome = run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Finalize,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        Some(&err_info),
                        loop_start,
                    );
                    if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                        return Err(
                            crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                                CompositionError::lifecycle_evaluation(
                                    "finalize",
                                    &prompt_state.source_path,
                                    eval_info,
                                ),
                                term,
                            )
                            .into(),
                        );
                    }
                    return Err(err);
                }
                TerminalControlAction::Fallthrough => {}
            }
            // `finalize` is a last-chance recovery surface: its stack may
            // recover (retry/resume/requeue/proxy) when `failure` did not.
            match run_finalize_with_recovery(
                lifecycle_guard,
                &materialized,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
                attempt,
                &mut control_budgets,
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                &mut proxy_tracking,
                show_checks,
            ) {
                TerminalControlAction::Continue { next_attempt } => {
                    attempt = next_attempt;
                    continue;
                }
                TerminalControlAction::Abort(err) => return Err(err),
                TerminalControlAction::Fallthrough => {}
            }
            // For provider-level failures, preserve the exit code at the
            // boundary rather than converting it into an `eyre` error. This
            // lets callers (e.g. `compose --loop`) inspect the terminal
            // attempt's iteration signals to build an honest
            // `LoopIterationFailed` cause.
            terminal_signals = iteration_signals;
            return Ok((outcome.exit_code, harness_perf, terminal_signals));
        }

        // For inline mode, apply closure after a successful provider run.
        if let Some(closure_plan) = materialized.inline_closure_plan.as_ref()
            && outcome.exit_code == 0
            && let Err(failures) = super::super::inline::try_inline_closure(
                closure_plan,
                &outcome.final_response,
                &prompt_state.source_path,
                child_cwd,
                show_checks,
                term,
            )
        {
            let fail_msg = format!(
                "inline closure failed ({} {}): {}",
                failures.len(),
                if failures.len() == 1 { "failure" } else { "failures" },
                failures.join("; "),
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            let err_info =
                LifecycleErrorInfo::from_action_failure("inline_closure", fail_msg.as_str());
            let failure_outcome = execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            )
            .outcome;
            if let Some(err) = handle_terminal_evaluation_error(
                &failure_outcome,
                "failure",
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                loop_start,
            ) {
                return Err(err);
            }
            match dispatch_terminal_control(
                &failure_outcome,
                attempt,
                &mut control_budgets,
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                &materialized,
                repo_root,
                lifecycle_guard,
                &mut proxy_tracking,
                term,
                show_checks,
            ) {
                TerminalControlAction::Continue { next_attempt } => {
                    attempt = next_attempt;
                    continue;
                }
                TerminalControlAction::Abort(err) => {
                    let finalize_outcome = run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Finalize,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        Some(&err_info),
                        loop_start,
                    );
                    if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                        return Err(
                            crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                                CompositionError::lifecycle_evaluation(
                                    "finalize",
                                    &prompt_state.source_path,
                                    eval_info,
                                ),
                                term,
                            )
                            .into(),
                        );
                    }
                    return Err(err);
                }
                TerminalControlAction::Fallthrough => {}
            }
            match run_finalize_with_recovery(
                lifecycle_guard,
                &materialized,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
                attempt,
                &mut control_budgets,
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                &mut proxy_tracking,
                show_checks,
            ) {
                TerminalControlAction::Continue { next_attempt } => {
                    attempt = next_attempt;
                    continue;
                }
                TerminalControlAction::Abort(err) => return Err(err),
                TerminalControlAction::Fallthrough => {}
            }
            return Err(eyre!("{fail_msg}"));
        }

        // A successful provider run proceeds to the success lifecycle event.
        // The `success.stack` may end in a flow-control action — either a direct
        // `resume`/`retry`/`proxy`/`requeue` (e.g. the agent finished but an
        // expected artifact is missing, so `resume` it), or an `error()` that
        // downgrades the run to failure (handled inside `execute_terminal_event`,
        // which then carries an `err` into `finalize`). Both surface as
        // `success.outcome.control`, so dispatch it uniformly.
        let success = execute_terminal_event(
            lifecycle_guard,
            LifecycleSignal::Success,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            None,
            loop_start,
        );
        // A late-binding evaluation error on the `success` event (a crashed
        // `when:` guard or interpolation — the canonical swallowed-error case)
        // halts the run: the provider already succeeded, so per Decision #3 we
        // do not fire `failure`. Surface it, run `finalize` once with the error
        // as `err`, and return non-zero rather than reporting a false success.
        if let Some(err) = handle_terminal_evaluation_error(
            &success.outcome,
            success.effective_event,
            lifecycle_guard,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            loop_start,
        ) {
            return Err(err);
        }
        match dispatch_terminal_control(
            &success.outcome,
            attempt,
            &mut control_budgets,
            outcome.session_id.as_deref(),
            profile,
            provider,
            prompt_state,
            &materialized,
            repo_root,
            lifecycle_guard,
            &mut proxy_tracking,
            term,
            show_checks,
        ) {
            TerminalControlAction::Continue { next_attempt } => {
                attempt = next_attempt;
                continue;
            }
            TerminalControlAction::Abort(err) => {
                let finalize_outcome = run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Finalize,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    success.downgrade_err.as_ref(),
                    loop_start,
                );
                if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                    return Err(
                        crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                            CompositionError::lifecycle_evaluation(
                                "finalize",
                                &prompt_state.source_path,
                                eval_info,
                            ),
                            term,
                        )
                        .into(),
                    );
                }
                return Err(err);
            }
            TerminalControlAction::Fallthrough => {}
        }
        // `finalize` carries the downgrade `err` (if any) and may recover.
        match run_finalize_with_recovery(
            lifecycle_guard,
            &materialized,
            repo_root,
            term,
            &effect_engine,
            success.downgrade_err.as_ref(),
            loop_start,
            attempt,
            &mut control_budgets,
            outcome.session_id.as_deref(),
            profile,
            provider,
            prompt_state,
            &mut proxy_tracking,
            show_checks,
        ) {
            TerminalControlAction::Continue { next_attempt } => {
                attempt = next_attempt;
                continue;
            }
            TerminalControlAction::Abort(err) => return Err(err),
            TerminalControlAction::Fallthrough => {}
        }
        terminal_signals = iteration_signals;
        return Ok((outcome.exit_code, harness_perf, terminal_signals));
    }
}

#[cfg(test)]
mod terminal_event_tests {
    use super::*;
    use claudine::composition::{
        LifecycleConfig, LifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext,
        parse_lifecycle_config,
    };
    use claudine::events::GlobalSettings;
    use claudine::messaging::RuntimeMessagingSettings;
    use std::sync::Mutex;

    /// The harness-loop wiring captures non-empty `timing`/`current` globals so
    /// terminal events expose `timing.document_ms`/`timing.total_ms` and a
    /// populated `current.env` — the regression this feature closes (previously
    /// every site hardcoded `timing: None, current: None`).
    #[test]
    fn capture_lifecycle_globals_populates_timing_and_current() {
        let loop_start = std::time::Instant::now();
        let (timing, current) =
            capture_lifecycle_globals(Path::new("prompt.md"), Some(Path::new(".")), None, loop_start);

        assert!(timing.document_ms.is_some(), "document_ms is populated");
        assert!(timing.total_ms.is_some(), "total_ms is populated");
        assert!(
            current.env.is_object() && !current.env.as_object().unwrap().is_empty(),
            "current.env is a non-empty environment snapshot"
        );
    }

    /// The injected globals the harness-loop builder attaches resolve
    /// `current.env.*` and `timing.document_ms` through Darkmatter's layered
    /// lookup (DM2) — proving the wiring reaches expression evaluation, not just
    /// the struct fields.
    #[test]
    #[serial_test::serial(env_loop_control_current)]
    fn attached_globals_resolve_through_lookup() {
        use claudine::composition::lifecycle_injected_globals;
        use darkmatter::markdown::compose::expression::{
            EvaluationLookup, evaluate, is_truthy, parse,
        };
        use darkmatter::markdown::compose::subtree::LayeredLookup;
        use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};

        let key = "CLAUDINE_TEST_LOOP_CONTROL_LATE_BIND";
        // SAFETY: serialized via #[serial]; no other thread reads this var.
        unsafe { std::env::set_var(key, "ready") };
        let (timing, current) =
            capture_lifecycle_globals(
                Path::new("prompt.md"),
                Some(Path::new(".")),
                None,
                loop_start_now(),
            );
        unsafe { std::env::remove_var(key) };

        let state = EffectiveStateBuilder::new()
            .with_context(ComposeContext::capture_for_content(Path::new("."), ""))
            .build()
            .unwrap();
        let globals = lifecycle_injected_globals(None, Some(&timing), Some(&current));
        let lookup = LayeredLookup::new(&state, &globals, None);

        let when = parse(&format!("current.env.{key} == 'ready'")).expect("parses");
        assert!(
            is_truthy(&evaluate(&when, &lookup).expect("evaluates")),
            "the late-bound env value resolves through the attached current global"
        );
        assert!(
            lookup.get("timing.document_ms").is_some(),
            "timing.document_ms resolves through the attached timing global"
        );
    }

    fn loop_start_now() -> std::time::Instant {
        std::time::Instant::now()
    }

    /// One emitted top-level communication, recorded by [`RecordingEmitter`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Emitted {
        Stderr(LifecycleSignal, String),
        Message(String),
        Speech(String),
    }

    /// Lifecycle emitter test double that records every emission.
    #[derive(Default)]
    struct RecordingEmitter {
        events: Mutex<Vec<Emitted>>,
    }

    impl RecordingEmitter {
        fn events(&self) -> Vec<Emitted> {
            self.events.lock().unwrap().clone()
        }
    }

    impl LifecycleEmitter for RecordingEmitter {
        fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _term: &Terminal) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Stderr(signal, text.to_string()));
        }
        fn emit_message(
            &self,
            text: &str,
            _source_path: &Path,
            _repo_root: Option<&Path>,
            _messaging: &RuntimeMessagingSettings,
        ) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Message(text.to_string()));
        }
        fn emit_speech(&self, text: &str, _config: biscuit_speaks::TtsConfig) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Speech(text.to_string()));
        }
        fn emit_effect(&self, _name: &str) {}
        fn emit_notification(&self, _title: &str) {}
    }

    fn materialized(frontmatter: serde_json::Value) -> MaterializedHarnessPrompt {
        let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&frontmatter);
        MaterializedHarnessPrompt {
            frontmatter,
            prompt: String::new(),
            env_overrides: Vec::new(),
            inline_closure_plan: None,
            live_frontmatter,
        }
    }

    /// Number of lines a stack's `append_line` side effect wrote — i.e. the
    /// number of times the stack actually executed its side effects.
    fn line_count(path: &Path) -> usize {
        std::fs::read_to_string(path)
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        log_path: PathBuf,
        config: LifecycleConfig,
        settings: GlobalSettings,
        messaging: RuntimeMessagingSettings,
        term: Terminal,
        source_path: PathBuf,
        materialized: MaterializedHarnessPrompt,
    }

    use std::path::PathBuf;

    /// Build a fixture whose `success` and `blocked` stacks each append one
    /// line to `events.log` (a side-effect counter) and carry a top-level
    /// `stderr` communication. When `with_error` is set, the named event's
    /// stack ends in `{error: "downgraded"}` so it routes to `failure`.
    fn fixture(frontmatter: serde_json::Value) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("prompt.md");
        let log_path = dir.path().join("events.log");
        let config = parse_lifecycle_config(&frontmatter, &source_path).unwrap();
        Fixture {
            _dir: dir,
            log_path,
            config,
            settings: GlobalSettings::default(),
            messaging: RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
            term: Terminal::default(),
            source_path,
            materialized: materialized(frontmatter),
        }
    }

    fn engine(root: &Path) -> EffectEngine {
        EffectEngine::builder()
            .mutation_root(root)
            .auto_rehash(false)
            .build()
    }

    #[test]
    fn success_stack_side_effects_run_exactly_once() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "succeeded",
                "stack": [{"action": {"append_line": ["events.log", "ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        // The stack's side effect fired exactly once (was twice before the fix).
        assert_eq!(line_count(&fx.log_path), 1, "stack ran exactly once");
        // Top-level success communication fired (the stack stayed success).
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Success, "succeeded".to_string())]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    }

    /// Phase 2: a terminal-phase `success.when` evaluation error is now filed in
    /// the dedicated evaluation-error channel, not the tolerated `action_error`.
    ///
    /// The first stack item's `when:` references an undefined frontmatter root,
    /// so it *raises* at event time (it does not evaluate cleanly to `false`).
    /// The spec (Decision #1) requires distinguishing such an **evaluation**
    /// error from a side-effect **dispatch** failure: the former must halt the
    /// run, the latter keeps today's log-and-continue policy.
    ///
    /// This asserts only the executor-level classification (Phase 2): the raise
    /// surfaces through `evaluation_error` and never lands in `action_error`. The
    /// orchestration that turns this into a `finalize`-with-`err` + non-zero
    /// outcome is wired in Phase 3.
    #[test]
    fn success_when_evaluation_error_is_not_swallowed_as_action_error() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "ready"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // The `when:` raised, so the stack never reached an action: nothing was
        // emitted and no control action fired.
        assert!(outcome.control.is_none(), "no control action ran");
        assert!(emitter.events().is_empty(), "the guarded action never ran");
        // The raise is filed in the dedicated evaluation-error channel, not the
        // tolerated dispatch-failure `action_error` channel the success path
        // would otherwise drop.
        assert!(
            outcome.action_error.is_none(),
            "a terminal-phase `when:` evaluation error must not be filed as an `action_error`"
        );
        assert!(
            outcome.evaluation_error.is_some(),
            "the `when:` raise surfaces through the halting evaluation-error channel"
        );
    }

    /// Phase 3: a terminal-phase `success` evaluation error halts the run.
    ///
    /// `handle_terminal_evaluation_error` runs `finalize` exactly once with the
    /// evaluation error exposed as `err` (so a `when: "err"` finalize branch
    /// fires) and returns the typed `LifecycleEvaluationError`. It does **not**
    /// fire `failure` (Decision #3): the provider already succeeded.
    #[test]
    fn success_evaluation_error_runs_finalize_with_err_and_returns_failure() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "ready"}}]
            },
            "finalize": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "finalized-with-err"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );
        assert!(
            success.outcome.evaluation_error.is_some(),
            "the success `when:` raised"
        );

        let err = handle_terminal_evaluation_error(
            &success.outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("a terminal evaluation error produces a run failure");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`success`") && rendered.contains("evaluation error"),
            "the run failure names the event and is a typed evaluation error: {rendered}"
        );
        // `finalize` fired exactly once and saw `err` populated (its `when: err`
        // branch ran), proving the error was threaded into the finalize context.
        assert!(guard.finalize_emitted(), "finalize fired");
        assert_eq!(
            line_count(&fx.log_path),
            1,
            "finalize ran once with `err` available"
        );
        // The provider succeeded, so `failure` was NOT fired (Decision #3).
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    }

    /// A `success` stack that downgrades to `failure` via explicit `error()`,
    /// where the resulting `failure` stack then raises an evaluation error,
    /// must surface the error attributed to `failure` — not `success`. After a
    /// downgrade, `outcome` holds the failure event's result, so
    /// `effective_event` must be `"failure"`; without it, the success caller
    /// would hardcode `"success"` and the diagnostic would point at the wrong
    /// event.
    #[test]
    fn downgraded_success_failure_raise_reports_failure_event() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"action": {"error": "downgraded"}}]
            },
            "failure": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // The success stack downgraded via `error()`, and the failure stack
        // raised an evaluation error in its `when:`.
        assert!(
            success.outcome.evaluation_error.is_some(),
            "the downgraded failure stack raised"
        );
        assert_eq!(
            success.effective_event, "failure",
            "effective_event must be `failure` after a downgrade"
        );

        let err = handle_terminal_evaluation_error(
            &success.outcome,
            success.effective_event,
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("the failure evaluation error halts the run");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the error must name the failure event, not success; got: {rendered}"
        );
        assert!(
            !rendered.contains("`success`"),
            "the error must NOT name the success event; got: {rendered}"
        );
    }

    /// Regression guard: a `success` stack whose own `when:` raises (no
    /// downgrade) keeps `effective_event == "success"`. Confirms the fix did
    /// not break the non-downgrading path.
    #[test]
    fn success_evaluation_error_non_downgrading_reports_success_event() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(
            success.outcome.evaluation_error.is_some(),
            "the success `when:` raised"
        );
        assert_eq!(
            success.effective_event, "success",
            "effective_event stays `success` when no downgrade occurred"
        );
    }

    /// Phase 3: a terminal-phase side-effect **dispatch** failure is NOT
    /// escalated — `handle_terminal_evaluation_error` returns `None` and runs no
    /// `finalize`, so the caller keeps today's log-and-continue policy.
    #[test]
    fn terminal_dispatch_failure_keeps_previous_outcome() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalized"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        assert!(guard.record_event_emission(LifecycleSignal::Success));
        let eng = engine(fx._dir.path());

        // A dispatch failure populates `action_error`, never `evaluation_error`.
        let outcome = LifecycleEventOutcome {
            action_error: Some(LifecycleErrorInfo::from_action_failure("shell", "boom")),
            ..Default::default()
        };
        let halted = handle_terminal_evaluation_error(
            &outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );
        assert!(halted.is_none(), "a dispatch failure does not halt the run");
        assert!(!guard.finalize_emitted(), "no finalize was forced");
        assert_eq!(line_count(&fx.log_path), 0, "the finalize stack did not run");
    }

    /// Behavior-matrix counterpart to the `success.when` raise: a terminal-phase
    /// `when:` that evaluates **cleanly to `false`** just skips its item — no
    /// evaluation error, no halt. This is the crashed-vs-clean-false distinction
    /// at the orchestration layer: a clean `false` guard must never be confused
    /// with a swallowed raise.
    #[test]
    fn terminal_clean_false_guard_skips_without_halting() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "1 == 2", "action": {"append_line": ["events.log", "ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // A clean `false` guard raises nothing and dispatches nothing.
        assert!(
            success.outcome.evaluation_error.is_none(),
            "a clean `false` guard must not file an evaluation error"
        );
        assert!(success.outcome.action_error.is_none());
        assert_eq!(line_count(&fx.log_path), 0, "the guarded action was skipped");

        // The clean false therefore does not halt the run.
        let halted = handle_terminal_evaluation_error(
            &success.outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );
        assert!(halted.is_none(), "a clean false guard does not halt the run");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    }

    /// Phase 3: a setup-phase evaluation error routes through `failure` then
    /// `finalize` (Decision #5), threading the error as `err`, and returns the
    /// typed run failure.
    #[test]
    fn setup_evaluation_error_routes_through_failure_and_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        // Model a `start` stack that raised at event time (no terminal recorded).
        let outcome = LifecycleEventOutcome {
            evaluation_error: Some(LifecycleErrorInfo::from_action_failure(
                "when",
                "`when:` references undefined variable `missing_root`",
            )),
            ..Default::default()
        };
        let err = handle_setup_evaluation_error(
            &outcome,
            "start",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("a setup evaluation error produces a run failure");

        assert!(err.to_string().contains("`start`"), "names the start event");
        // Both `failure` and `finalize` ran, each seeing `err` populated.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
            "setup evaluation error routes through failure then finalize, both with `err`"
        );
        assert!(guard.finalize_emitted(), "finalize fired");
    }

    /// Phase 3: an evaluation error raised *inside* `finalize` aborts the run
    /// without re-entering `finalize` (the re-entry guard). `finalize` fires
    /// exactly once and the recovery path returns `Abort` with the typed error.
    #[test]
    fn finalize_evaluation_error_aborts_without_reentry() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "x"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        // Model the live call site: a terminal already fired this iteration.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let eng = engine(fx._dir.path());
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let mut proxy = ProxyTracking::default();

        let action = run_finalize_with_recovery(
            &mut guard,
            &fx.materialized,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Claude,
            &mut state,
            &mut proxy,
            false,
        );

        match action {
            TerminalControlAction::Abort(err) => {
                assert!(
                    err.to_string().contains("`finalize`"),
                    "the abort names the finalize event: {err}"
                );
            }
            other => panic!("expected Abort from a finalize evaluation error, got {other:?}"),
        }
        // `finalize` ran exactly once — the abort did not loop back into it.
        assert!(guard.finalize_emitted(), "finalize fired exactly once");
    }

    #[test]
    fn success_stack_error_routes_to_failure_keeps_success_comm_before_failure() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "succeeded",
                "stack": [{"action": [{"append_line": ["events.log", "ran"]}, {"error": "downgraded"}]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        // Outcome reflects the failure event's run (no Error control surviving).
        assert!(outcome.control.is_none());
        // Success stack ran once (append + error), failure stack ran once.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["ran", "failure-ran"],
            "success stack and failure stack each ran exactly once"
        );
        // The success top-level comm fired FIRST (top-level-before-stack), then
        // the downgrade fired the failure top-level comm. The success comm is
        // NOT suppressed — the spec requires top-level to fire before stack
        // processing, so a later `error()` cannot un-fire it.
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Success, "succeeded".to_string()),
                Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
            ]
        );
        // Guard recorded the downgraded terminal signal as Failure.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    }

    #[test]
    fn blocked_stack_side_effects_run_exactly_once() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked",
                "stack": [{"action": {"append_line": ["events.log", "ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        assert_eq!(line_count(&fx.log_path), 1, "blocked stack ran exactly once");
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Blocked, "blocked".to_string())]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    }

    /// Top-level communication for `success` fires before any `stack:`
    /// communication in the same event.
    #[test]
    fn success_top_level_communication_fires_before_stack_communication() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "success-top",
                "stack": [{"action": {"stderr": "success-stack"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Success, "success-top".to_string()),
                Emitted::Stderr(LifecycleSignal::Success, "success-stack".to_string()),
            ],
            "top-level communication must fire before stack communication"
        );
    }

    /// Top-level communication for `blocked` fires before any `stack:`
    /// communication in the same event.
    #[test]
    fn blocked_top_level_communication_fires_before_stack_communication() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked-top",
                "stack": [{"action": {"stderr": "blocked-stack"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(outcome.control.is_none());
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Blocked, "blocked-top".to_string()),
                Emitted::Stderr(LifecycleSignal::Blocked, "blocked-stack".to_string()),
            ],
            "top-level communication must fire before stack communication"
        );
    }

    /// Reproduces the exact guard call sequence of the `run_harness_loop`
    /// `routes_to_failure(Start)` branch: a `start` stack action errored, so
    /// the failure path records `Failure`, runs the error-carrying failure
    /// stack, and then must reach `finalize`. Asserts the failure AND finalize
    /// stacks each ran exactly once and `finalize_emitted()` is true — proving
    /// `finalize` is not skipped (the Finding 2 defect).
    #[test]
    fn start_stack_action_error_records_failure_then_runs_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "failed",
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stderr": "finalized",
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // The provider never launched (a setup-phase `start` error routes here),
        // but `start` was emitted — mirror the loop's pre-launch state.
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        // --- The fixed `routes_to_failure(Start)` branch sequence ---
        let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");
        // 1. Record `Failure` FIRST (the fix). This sets `terminal_emitted`.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        // 2. Run the error-carrying failure stack via `run_event_stack`.
        let failure_ctx = build_lifecycle_stack_context_for_materialized(
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            None,
            None,
            &fx.term,
            guard.emitter(),
            guard.context().settings,
            guard.context().messaging,
            &eng,
            Some(&action_error),
            None,
            None,
        );
        guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);
        // 3. Finalize must now fire (records + runs because terminal_emitted).
        run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&action_error),
            std::time::Instant::now(),
        );

        // Finalize was NOT skipped.
        assert!(
            guard.finalize_emitted(),
            "finalize must fire after a setup-phase failure"
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        // Both the failure stack and the finalize stack ran exactly once.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
            "failure stack and finalize stack each ran exactly once"
        );
        // Both top-level comms fired, failure before finalize.
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
                Emitted::Stderr(LifecycleSignal::Finalize, "finalized".to_string()),
            ]
        );
    }

    /// Locks in WHY the fix is needed: calling `run_event_stack(Failure, ...)`
    /// WITHOUT first `record_event_emission(Failure)` leaves `terminal_emitted`
    /// false, so a subsequent `Finalize` is a no-op (the finalize stack never
    /// runs). This documents the Finding 2 defect the fix removes.
    #[test]
    fn failure_stack_without_record_skips_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        // Defective sequence: run the failure stack directly, no record.
        let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");
        let failure_ctx = build_lifecycle_stack_context_for_materialized(
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            None,
            None,
            &fx.term,
            guard.emitter(),
            guard.context().settings,
            guard.context().messaging,
            &eng,
            Some(&action_error),
            None,
            None,
        );
        guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);
        // `Finalize` is a no-op because no terminal signal was recorded.
        run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        assert!(
            !guard.finalize_emitted(),
            "without record_event_emission(Failure) the finalize is skipped"
        );
        // The failure stack ran but the finalize stack did not.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran"],
            "finalize stack must not run when the terminal signal was never recorded"
        );
    }

    #[test]
    fn blocked_stack_error_routes_to_failure_keeps_blocked_comm_before_failure() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked",
                "stack": [{"action": [{"append_line": ["events.log", "ran"]}, {"error": "downgraded"}]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["ran", "failure-ran"],
            "blocked stack and failure stack each ran exactly once"
        );
        // The blocked top-level comm fired FIRST (top-level-before-stack), then
        // the downgrade fired the failure top-level comm.
        assert_eq!(
            emitter.events(),
            vec![
                Emitted::Stderr(LifecycleSignal::Blocked, "blocked".to_string()),
                Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
            ]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    }

    // -- dispatch_terminal_control runtime-wiring tests --------------------

    use claudine::composition::lifecycle_executor::{LifecycleEventOutcome, StackControl};
    use claudine::composition::RetryBackoff;

    fn prompt_state(source: &Path) -> HarnessPromptState {
        HarnessPromptState {
            mode: HarnessPromptMode::Compose,
            source_path: source.to_path_buf(),
            original_ref: source.display().to_string(),
            base_prompt: None,
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
        }
    }

    /// A real provider profile that supports session resume (Claude).
    fn resume_capable_profile() -> &'static dyn super::super::super::profile::WrapperProfile {
        super::super::super::profile::profile_for_provider(Provider::Claude)
            .expect("claude profile exists")
    }

    fn outcome_with(control: StackControl) -> LifecycleEventOutcome {
        LifecycleEventOutcome {
            control: Some(control),
            ..Default::default()
        }
    }

    fn dispatch_guard<'a>(
        config: &'a LifecycleConfig,
        ctx: &'a LifecycleRuntimeContext<'a>,
        emitter: &'a RecordingEmitter,
    ) -> LifecycleRunGuard<'a> {
        LifecycleRunGuard::new(config, ctx, emitter)
    }

    #[test]
    fn dispatch_retry_from_failure_continues_and_resets_guard() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        // Mark a Failure terminal as already emitted to model the live call site.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();

        let outcome = outcome_with(StackControl::Retry {
            max_attempts: 2,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Continue { next_attempt } => assert_eq!(next_attempt, 2),
            other => panic!("expected Continue, got {other:?}"),
        }
        // Guard was reset so the retried attempt can emit a fresh terminal.
        assert_eq!(guard.terminal_signal(), None);
    }

    #[test]
    fn dispatch_retry_from_finalize_continues_and_resets_guard() {
        // `finalize` is a last-chance recovery surface: a `finalize.stack`
        // ending in `retry` must re-enter the loop exactly as `failure` does.
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        // Model the live call site: a terminal signal and `finalize` already
        // fired this iteration before the finalize stack's control dispatches.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        assert!(guard.record_event_emission(LifecycleSignal::Finalize));
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();

        let outcome = outcome_with(StackControl::Retry {
            max_attempts: 1,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Continue { next_attempt } => assert_eq!(next_attempt, 2),
            other => panic!("expected Continue, got {other:?}"),
        }
        // Guard was reset so the retried attempt can emit a fresh terminal.
        assert_eq!(guard.terminal_signal(), None);
    }

    #[test]
    fn dispatch_resume_from_finalize_seeds_prompt_state() {
        // `resume` is valid at `finalize` too (parity with `failure`).
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();

        let outcome = outcome_with(StackControl::Resume {
            message: "finish the task".to_string(),
            max_attempts: 1,
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Continue { .. }));
        assert_eq!(state.next_prompt_override.as_deref(), Some("finish the task"));
        assert_eq!(state.next_resume_session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn dispatch_retry_exhausts_after_budget() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        // Pre-seed the retry budget to ceiling 2 (max_attempts 1 firing at 1).
        let mut budgets = ControlBudgets {
            retry: Some(2),
            resume: None,
        };
        let outcome = outcome_with(StackControl::Retry {
            max_attempts: 1,
            backoff: RetryBackoff::Fixed,
            delay: "0s".to_string(),
        });
        // attempt 2 has reached the ceiling → fall through (no continue).
        let action = dispatch_terminal_control(
            &outcome,
            2,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Fallthrough));
    }

    #[test]
    fn dispatch_resume_with_session_seeds_prompt_state() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Resume {
            message: "please finish the task".to_string(),
            max_attempts: 1,
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            Some("sess-42"),
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(
            action,
            TerminalControlAction::Continue { next_attempt: 2 }
        ));
        assert_eq!(state.next_resume_session_id.as_deref(), Some("sess-42"));
        assert_eq!(
            state.next_prompt_override.as_deref(),
            Some("please finish the task")
        );
    }

    #[test]
    fn dispatch_resume_without_session_aborts_typed() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Resume {
            message: "x".to_string(),
            max_attempts: 1,
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Abort(err) => {
                assert!(
                    err.to_string().contains("requires a live provider session"),
                    "unexpected: {err}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_proxy_swaps_source_and_resets_guard_for_fresh_run() {
        let fx = fixture(serde_json::json!({}));
        let target = fx._dir.path().join("target.md");
        std::fs::write(&target, "---\n---\nbody\n").unwrap();
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        // Use an absolute target so resolution is unambiguous.
        let outcome = outcome_with(StackControl::Proxy {
            target: target.display().to_string(),
        });
        let action = dispatch_terminal_control(
            &outcome,
            3,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        // Proxy re-enters at attempt 1 for a fresh run.
        assert!(matches!(
            action,
            TerminalControlAction::Continue { next_attempt: 1 }
        ));
        assert_eq!(state.source_path, target);
        // The guard was fully reset (initialize will fire again).
        assert!(!guard.initialize_emitted());
        assert_eq!(guard.terminal_signal(), None);
    }

    #[test]
    fn dispatch_defer_aborts_not_implemented() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Defer {
            delay: "5m".to_string(),
            reason: Some("later".to_string()),
        });
        let action = dispatch_terminal_control(
            &outcome,
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        match action {
            TerminalControlAction::Abort(err) => {
                let msg = err.to_string();
                assert!(
                    msg.contains("defer")
                        && msg.contains("not implemented")
                        && msg.contains("rendezvous"),
                    "expected the defer-not-implemented error, got: {err}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_stop_falls_through() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let action = dispatch_terminal_control(
            &outcome_with(StackControl::Stop),
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Fallthrough));
    }

    #[test]
    fn dispatch_no_control_falls_through() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let action = dispatch_terminal_control(
            &LifecycleEventOutcome::default(),
            1,
            &mut budgets,
            None,
            resume_capable_profile(),
            Provider::Goose,
            &mut state,
            &fx.materialized,
            Some(fx._dir.path()),
            &mut guard,
            &mut ProxyTracking::default(),
            &fx.term,
            false,
        );
        assert!(matches!(action, TerminalControlAction::Fallthrough));
    }

    // -- emit_blocked_finalize_with_err (Finding 5) ------------------------

    /// Before the provider launches, the helper selects `Blocked` as the
    /// terminal signal (matching `emit_blocked_or_failure`'s pre/post-launch
    /// rule) and runs both the blocked and finalize stacks, with `err`
    /// available to the stack expression engine.
    #[test]
    fn emit_blocked_finalize_pre_launch_runs_blocked_then_finalize_with_err() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [
                    {"action": {"append_line": ["events.log", "{{ 'blocked-kind=' + err.kind }}"]}},
                    {"action": {"append_line": ["events.log", "{{ 'blocked-variant=' + err.variant }}"]}},
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "{{ 'finalize-msg=' + err.msg }}"]}},
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        // Pre-launch → the terminal signal is Blocked, and finalize fired.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec![
                "blocked-kind=LifecycleAction",
                "blocked-variant=materialize",
                "finalize-msg=boom",
            ],
            "blocked stack observes err.kind/err.variant; finalize `when: err` is \
             truthy and observes err.msg"
        );
    }

    /// Once the provider has launched, the helper selects `Failure` as the
    /// terminal signal (the post-launch branch of `emit_blocked_or_failure`).
    #[test]
    fn emit_blocked_finalize_post_launch_selects_failure() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
        );
    }

    /// The materialize-failure call site has no real prompt, so it passes a
    /// synthetic prompt whose `frontmatter` is `Value::Null`. The stack-context
    /// builder must fall back to an empty frontmatter map (rather than panic or
    /// skip the stack), so the guard's own blocked/finalize stacks still fire
    /// and `err` remains available.
    #[test]
    fn emit_blocked_finalize_tolerates_null_frontmatter_materialized() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "{{ 'blocked-kind=' + err.kind }}"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");
        // The site-1 synthetic prompt: materialization failed, so there is no
        // frontmatter to carry — `Value::Null` exercises the empty-map fallback.
        let synthetic = materialized(serde_json::Value::Null);

        emit_blocked_finalize_with_err(
            &mut guard,
            &synthetic,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["blocked-kind=LifecycleAction", "finalize-ran"],
            "the guard's blocked/finalize stacks fire and observe err even when the \
             materialized prompt's frontmatter is null"
        );
    }

    // -- emit_failure_finalize_with_err (post-start setup `?` sites) --------

    /// The post-start setup sites (snapshot / launch / pre-spawn attempt) run
    /// after `start` and pre-flight have passed, so their terminal signal is
    /// always `Failure` — never `Blocked` — and `finalize` must follow with
    /// `err` available to both stacks. Here the guard has emitted `start` but
    /// the provider has NOT launched, the case the existing
    /// `provider_launched()`-driven helper would mis-route to `Blocked`.
    #[test]
    fn emit_failure_finalize_forces_failure_when_not_launched() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "failed",
                "stack": [
                    {"action": {"append_line": ["events.log", "{{ 'failure-kind=' + err.kind }}"]}},
                    {"action": {"append_line": ["events.log", "{{ 'failure-variant=' + err.variant }}"]}},
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "{{ 'finalize-msg=' + err.msg }}"]}},
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Reach `start` without launching the provider — exactly the state at
        // the snapshot / launch / pre-spawn-attempt `?` sites.
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        assert!(!guard.provider_launched());
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_snapshot", "boom");

        emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        // Terminal is Failure (not Blocked) and finalize fired.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        // Top-level failure communication fired.
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string())]
        );
        // Both stacks ran with `err` available.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec![
                "failure-kind=LifecycleAction",
                "failure-variant=harness_snapshot",
                "finalize-msg=boom",
            ],
            "failure stack observes err.kind/err.variant; finalize `when: err` is \
             truthy and observes err.msg"
        );
    }

    /// The materialized prompt for an attempt-execution failure carries the
    /// real frontmatter, but the helper must equally tolerate a synthetic
    /// `Value::Null` frontmatter (empty-map fallback) without skipping the
    /// stacks — mirroring the blocked-helper's null tolerance.
    #[test]
    fn emit_failure_finalize_tolerates_null_frontmatter() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");
        let synthetic = materialized(serde_json::Value::Null);

        emit_failure_finalize_with_err(
            &mut guard,
            &synthetic,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["failure-ran", "finalize-ran"],
        );
    }

    // -- emit_*_with_err: late-binding evaluation error surfacing ------------

    /// Convenience: assert the helper returned a `LifecycleEvaluationError`
    /// naming `event`, then hand back the inner error for any extra checks.
    ///
    /// These surfacing helpers now mark the error already-emitted (Decision #2:
    /// the styled block was rendered to stderr at the catch point), so the
    /// returned error is wrapped in `LifecycleEvaluationAlreadyEmitted`. Unwrap
    /// it before asserting the inner shape — the presence of the marker confirms
    /// the early emit fired.
    fn assert_lifecycle_eval_error(
        result: Option<CompositionError>,
        event: &str,
    ) -> CompositionError {
        let err = result.expect("helper must return Some on a lifecycle evaluation raise");
        let inner = match &err {
            CompositionError::LifecycleEvaluationAlreadyEmitted { inner } => inner.as_ref(),
            other => other,
        };
        match inner {
            CompositionError::LifecycleEvaluationError { event: got, .. } => {
                assert_eq!(
                    got, event,
                    "expected LifecycleEvaluationError for `{event}`, got `{got}`"
                );
                err
            }
            other => panic!("expected LifecycleEvaluationError, got {other:?}"),
        }
    }

    /// Pre-launch: a `blocked.stack` `when:` raise must surface as a typed
    /// evaluation error naming `blocked`, and the helper must still fire the
    /// `failure` and `finalize` stacks (with the evaluation error as `err`) by
    /// redesignating the already-taken terminal slot. Without the redesignate
    /// fix, the failure stack would be silently refused and "failure-ran" would
    /// never appear.
    #[test]
    fn emit_blocked_finalize_pre_launch_blocked_raise_surfaces_failure_and_finalize() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            },
            "failure": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — do NOT call `mark_provider_launched()` — so the helper
        // selects `Blocked` as the terminal signal.
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        let typed = assert_lifecycle_eval_error(result, "blocked");
        assert!(
            typed.to_string().contains("evaluation error"),
            "error message surfaces evaluation error: {}",
            typed
        );
        // Redesignation took effect: terminal signal flipped Blocked → Failure.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        // The key assertion: both failure and finalize stacks ran with the
        // evaluation error as `err` (the redesignate fix lets failure fire).
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert!(
            logged.contains(&"failure-ran"),
            "failure stack fired with eval error as err: {logged:?}"
        );
        assert!(
            logged.contains(&"finalize-ran"),
            "finalize stack fired with eval error as err: {logged:?}"
        );
    }

    /// Post-launch: the helper selects `Failure` as the terminal signal. A
    /// `failure.stack` `when:` raise surfaces as a typed evaluation error
    /// naming `failure`, and the `finalize` stack still fires with the
    /// evaluation error as `err`. Failure is already terminal, so no
    /// redesignation is needed.
    #[test]
    fn emit_blocked_finalize_post_launch_failure_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "failure");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert!(
            logged.contains(&"finalize-ran"),
            "finalize stack fired with eval error as err: {logged:?}"
        );
        // Finalize fired exactly once — the helper did not re-enter failure.
        assert_eq!(
            logged.iter().filter(|l| **l == "finalize-ran").count(),
            1,
            "finalize fired exactly once (no re-entry into failure)"
        );
    }

    /// A `finalize.stack` raise surfaces as a typed evaluation error naming
    /// `finalize`. The helper must not re-enter finalize, and the (already
    /// fired) blocked stack must not fire a second time.
    #[test]
    fn emit_blocked_finalize_finalize_raise_surfaces_without_reentry() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
            },
            "finalize": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert_eq!(
            logged.iter().filter(|l| **l == "blocked-ran").count(),
            1,
            "blocked stack fired exactly once (no re-entry)"
        );
    }

    /// review-4 regression: the pre-start **missing-source** setup-failure
    /// branch routes through `emit_blocked_finalize_with_err` (pre-launch →
    /// `Blocked`). A `blocked.when` raise must surface a typed evaluation error
    /// — proving the branch no longer swallows it in favor of the generic
    /// "source file does not exist" fallback. The surfaced event names the
    /// terminal event (`blocked`); the redesignate-to-failure path runs the
    /// `failure`/`finalize` stacks but the typed error still reports the slot
    /// where the raise occurred.
    #[test]
    fn missing_source_branch_blocked_raise_surfaces_not_swallowed() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — the missing-source branch is reached before the provider
        // launches, so do NOT mark it launched; the helper selects `Blocked`.
        let eng = engine(fx._dir.path());
        // The exact err_info the missing-source branch builds.
        let err_info = LifecycleErrorInfo::from_action_failure(
            "missing_source",
            "source file does not exist: prompt.md",
        );

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        // The terminal slot was `blocked`, so the typed error names `blocked`;
        // the evaluation error is surfaced rather than swallowed.
        let typed = assert_lifecycle_eval_error(result, "blocked");
        let rendered = typed.to_string();
        assert!(
            rendered.contains("evaluation error"),
            "error surfaces the evaluation error: {rendered}"
        );
        // The generic missing-source fallback is NOT the surfaced error.
        assert!(
            !rendered.contains("source file does not exist"),
            "the lifecycle raise supersedes the generic fallback: {rendered}"
        );
    }

    /// review-4 regression: the pre-start **shell-audit** setup-failure branch
    /// routes through `emit_blocked_finalize_with_err`. A `finalize.when` raise
    /// (with a clean `blocked`) must surface a typed evaluation error naming
    /// `finalize` without re-entering finalize — proving the branch no longer
    /// swallows it in favor of the generic "shell audit failed" fallback.
    #[test]
    fn shell_audit_branch_finalize_raise_surfaces_not_swallowed() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
            },
            "finalize": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — the shell-audit branch fires before launch.
        let eng = engine(fx._dir.path());
        // The exact err_info the shell-audit branch builds.
        let err_info = LifecycleErrorInfo::from_action_failure(
            "shell_audit",
            "shell audit failed: 1 denied directive(s) in source page",
        );

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        let typed = assert_lifecycle_eval_error(result, "finalize");
        let rendered = typed.to_string();
        assert!(
            rendered.contains("evaluation error"),
            "error surfaces the evaluation error: {rendered}"
        );
        // The generic shell-audit fallback is NOT the surfaced error.
        assert!(
            !rendered.contains("shell audit failed"),
            "the lifecycle raise supersedes the generic fallback: {rendered}"
        );
        // The clean blocked stack fired exactly once and finalize did not
        // re-enter.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().filter(|l| *l == "blocked-ran").count(),
            1,
            "blocked stack fired exactly once (no re-entry)"
        );
    }

    /// `emit_failure_finalize_with_err` — a `failure.stack` raise surfaces as
    /// a typed evaluation error naming `failure`, and the `finalize` stack
    /// still fires with the evaluation error as `err`.
    #[test]
    fn emit_failure_finalize_failure_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Reach `start` without launching the provider — exactly the state at
        // the snapshot / launch / pre-spawn-attempt `?` sites.
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_snapshot", "boom");

        let result = emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "failure");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert!(
            logged.contains(&"finalize-ran"),
            "finalize stack fired with eval error as err: {logged:?}"
        );
    }

    /// `emit_failure_finalize_with_err` — a `finalize.stack` raise surfaces as
    /// a typed evaluation error naming `finalize`. The failure stack (already
    /// fired) must not fire a second time.
    #[test]
    fn emit_failure_finalize_finalize_raise_surfaces_without_reentry() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [
                    {"when": "missing_root == true", "action": {"stderr": "never"}}
                ]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");

        let result = emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        let logged: Vec<&str> = lines.lines().collect();
        assert_eq!(
            logged.iter().filter(|l| **l == "failure-ran").count(),
            1,
            "failure stack fired exactly once (no re-entry)"
        );
    }

    /// Precedence: when both `failure` and `finalize` raise after a setup
    /// error, the surfaced error must name `finalize` — the latest lifecycle
    /// crash — not `failure`. Previously the failure raise hid the finalize
    /// raise behind it.
    #[test]
    fn emit_failure_finalize_both_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        assert!(guard.record_event_emission(LifecycleSignal::Start));
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");

        let result = emit_failure_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert!(guard.finalize_emitted(), "finalize must have fired");
    }

    /// Precedence: a `success.when` raise followed by a `finalize.when` raise
    /// must surface the finalize raise — not the original `success` raise.
    /// Drives the same path the runtime takes for a terminal evaluation
    /// error: `execute_terminal_event` records the raise, then
    /// `handle_terminal_evaluation_error` runs `finalize` carrying it.
    #[test]
    fn success_raise_then_finalize_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let success = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );
        assert!(
            success.outcome.evaluation_error.is_some(),
            "the success `when:` raised"
        );

        let err = handle_terminal_evaluation_error(
            &success.outcome,
            "success",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("the terminal evaluation error halts the run");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the error must name the finalize event, not success; got: {rendered}"
        );
        assert!(
            !rendered.contains("`success`"),
            "the error must NOT name the success event; got: {rendered}"
        );
    }

    /// Precedence: a setup-phase `initialize`/`start` raise followed by a
    /// `failure.when` raise must surface `failure`, and `finalize` must
    /// receive the FAILURE evaluation error as `err` (not the original). The
    /// `finalize.stack` interpolates `{{ err.event }}` so we can prove it
    /// observed the failure raise.
    #[test]
    fn setup_raise_then_failure_raise_surfaces_failure_and_threads_into_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stack": [{"when": "failure_typo == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stack": [{
                    "when": "err",
                    "action": {"append_line": ["events.log", "finalize-saw-{{err.variant}}"]}
                }]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        // Model a `start` stack that raised at event time.
        let outcome = LifecycleEventOutcome {
            evaluation_error: Some(LifecycleErrorInfo::from_action_failure(
                "when",
                "`when:` references undefined variable `missing_root`",
            )),
            ..Default::default()
        };
        let err = handle_setup_evaluation_error(
            &outcome,
            "start",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .expect("the setup evaluation error halts the run");

        let rendered = err.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the error must name the failure event (failure raised); got: {rendered}"
        );
        assert!(
            !rendered.contains("`start`"),
            "the error must NOT name the start event; got: {rendered}"
        );

        // `finalize` ran with the FAILURE evaluation error as `err` — its
        // appended marker interpolates `err.variant`, which the failure raise
        // fills with `when` (the variant of the failure `when:` raise), not
        // the original `missing_root` text.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert!(
            lines.contains("finalize-saw-when"),
            "finalize must have observed the failure evaluation error (variant=when); got: {lines}"
        );
    }

    /// Precedence: a `blocked.when` raise (terminal) followed by a catch
    /// `finalize.when` raise must surface `finalize`. Pre-launch so the
    /// helper selects `Blocked`; the redesignation path runs `failure` (no
    /// raise authored), then `finalize` raises.
    #[test]
    fn emit_blocked_finalize_blocked_raise_then_finalize_raise_surfaces_finalize() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "failure": {
                "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}]
            },
            "finalize": {
                "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Pre-launch — do NOT call `mark_provider_launched()` — so the helper
        // selects `Blocked` as the terminal signal and redesignates to Failure.
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert_lifecycle_eval_error(result, "finalize");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
        assert!(guard.finalize_emitted(), "finalize must have fired");
        // The failure stack ran (no raise authored) and saw `err`.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert!(
            lines.contains("failure-ran"),
            "failure stack ran with the original blocked evaluation error as err: {lines}"
        );
    }

    /// Happy-path regression: with no evaluation raises the helper returns
    /// `None` and the caller propagates the original setup error unchanged.
    #[test]
    fn emit_blocked_finalize_returns_none_when_no_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
            },
            "finalize": {
                "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());
        let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

        let result = emit_blocked_finalize_with_err(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            &err_info,
            std::time::Instant::now(),
        );

        assert!(result.is_none(), "no evaluation error → returns None");
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
        assert!(guard.finalize_emitted(), "finalize still fires on the happy path");
    }

    // -- Broken-path regression tests: explicit error(...) / routes_to_failure
    //    catch paths that previously discarded failure/finalize outcomes ------
    //
    // These exercise the previously-broken catch paths where an explicit
    // lifecycle control (`error(...)`), action-error routing (`routes_to_failure`),
    // or terminal-control abort still runs failure/finalize but discarded the
    // returned outcomes — swallowing any evaluation error raised by those catch
    // events.

    /// `run_target_initialize` — a target's `initialize.error(...)` whose catch
    /// `failure.when:` raises surfaces the FAILURE evaluation error, not the
    /// original `error(...)` reason. Proves the previously-discarded failure
    /// outcome now threads through `catch_evaluation_error`.
    #[test]
    fn target_initialize_error_with_failure_raise_surfaces_failure_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "initialize": {
                "stack": [{"action": {"error": "target refused"}}]
            },
            "failure": {
                "stderr": "fail",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": { "stderr": "final" }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let action = run_target_initialize(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );

        match action {
            TargetInitializeAction::Abort(report) => {
                let rendered = report.to_string();
                assert!(
                    rendered.contains("`failure`"),
                    "the surfaced error must name the failure event; got: {rendered}"
                );
                assert!(
                    rendered.contains("evaluation error"),
                    "the surfaced error must mention evaluation error; got: {rendered}"
                );
                assert!(
                    !rendered.contains("target refused"),
                    "the original `error(...)` reason must NOT be the surfaced error; got: {rendered}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    /// `run_target_initialize` — a target's `initialize` action error that
    /// `routes_to_failure` whose catch `failure.when:` raises surfaces the
    /// FAILURE evaluation error, not the generic "lifecycle initialize failed"
    /// fallback. Proves the previously-discarded failure outcome now threads
    /// through `catch_evaluation_error` for the routes_to_failure path.
    #[test]
    fn target_initialize_routes_to_failure_with_raise_surfaces_failure_evaluation_error() {
        let fx = fixture(serde_json::json!({
            // A `shell: false` action errors and routes_to_failure(Initialize).
            "initialize": {
                "stack": [{"action": {"shell": "false"}}]
            },
            "failure": {
                "stderr": "fail",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": { "stderr": "final" }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let action = run_target_initialize(
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );

        match action {
            TargetInitializeAction::Abort(report) => {
                let rendered = report.to_string();
                assert!(
                    rendered.contains("`failure`"),
                    "the surfaced error must name the failure event; got: {rendered}"
                );
                assert!(
                    rendered.contains("evaluation error"),
                    "the surfaced error must mention evaluation error; got: {rendered}"
                );
                assert!(
                    !rendered.contains("lifecycle initialize failed"),
                    "the generic fallback message must NOT be the surfaced error; got: {rendered}"
                );
            }
            other => panic!("expected Abort, got {other:?}"),
        }
    }

    /// Start `routes_to_failure` catch path (Location G): when `failure.when`
    /// raises after a start action error, the surfaced error must name
    /// `failure`, and finalize must receive the FAILURE evaluation error as
    /// `err` (not the original action error) so a `finalize.stack` can branch
    /// on the failure raise. Simulates the inline `run_harness_loop` code
    /// path's primitives directly (record_event_emission + run_event_stack +
    /// run_lifecycle_event) since the surrounding function is impractical to
    /// call from a unit test.
    #[test]
    fn start_routes_to_failure_with_raise_surfaces_failure_and_threads_into_finalize() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "fail",
                "stack": [{
                    "when": "missing_root == true",
                    "action": {"stderr": "never"}
                }]
            },
            "finalize": {
                "stderr": "final",
                "stack": [{
                    "when": "err",
                    "action": {"append_line": ["events.log", "finalize-saw-{{err.variant}}"]}
                }]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // Mirror run_harness_loop's pre-start state.
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        // Model a `start` outcome that routed to failure with an action error.
        let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");

        // Replicate the Location G fix: record Failure FIRST, then run the
        // error-carrying failure stack via run_event_stack, threading any
        // failure raise into finalize.
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let failure_ctx = build_lifecycle_stack_context_for_materialized(
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            None,
            None,
            &fx.term,
            guard.emitter(),
            guard.context().settings,
            guard.context().messaging,
            &eng,
            Some(&action_error),
            None,
            None,
        );
        let failure_outcome = guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);

        // The fix: thread active_err into finalize (failure raise > original).
        // When failure raises, active_err is the failure evaluation error; the
        // synthetic-fallback case (no original action_error and no failure
        // raise) is exercised by the runtime paths but not duplicated here.
        let active_err = failure_outcome
            .evaluation_error
            .as_ref()
            .unwrap_or(&action_error);
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(active_err),
            std::time::Instant::now(),
        );

        // The failure raised, so the surfaced error must name `failure`.
        assert!(
            failure_outcome.evaluation_error.is_some(),
            "the failure `when:` raised"
        );
        let ce = CompositionError::catch_evaluation_error(
            &fx.source_path,
            "start",
            &action_error,
            Some(&failure_outcome),
            Some(&finalize_outcome),
        );
        let rendered = ce.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the surfaced error must name the failure event; got: {rendered}"
        );
        assert!(
            !rendered.contains("`start`"),
            "the surfaced error must NOT name the start event; got: {rendered}"
        );

        // finalize ran with the FAILURE evaluation error as `err` — its
        // appended marker interpolates `err.variant`, which the failure raise
        // fills with `when` (the variant of the failure `when:` raise), not
        // the original `shell` action_error variant.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert!(
            lines.contains("finalize-saw-when"),
            "finalize must have observed the failure evaluation error (variant=when); got: {lines}"
        );
    }

    /// Terminal-control abort catch path (Locations H/I/J): when `finalize.when`
    /// raises after a terminal-control Abort decision, the surfaced error must
    /// name `finalize` (the catch event's raise), not the original abort
    /// reason. Simulates the inline `run_harness_loop` Abort arm directly.
    #[test]
    fn terminal_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stderr": "final",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // The failure/success event already fired cleanly before the Abort.
        guard.mark_provider_launched();
        guard.record_event_emission(LifecycleSignal::Failure);
        let eng = engine(fx._dir.path());

        // Replicate the Location H/I/J fix: run finalize carrying the abort's
        // err_info; if finalize raises, surface the finalize evaluation error.
        let err_info = LifecycleErrorInfo::from_action_failure("agent_failure", "boom");
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        );

        let surfaced_err: color_eyre::eyre::Report = if let Some(eval_info) =
            finalize_outcome.evaluation_error.as_ref()
        {
            CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
        } else {
            // The original abort reason would surface here on the happy path.
            eyre!("original abort reason")
        };

        let rendered = surfaced_err.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the surfaced error must name the finalize event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
        assert!(
            !rendered.contains("original abort reason"),
            "the original abort reason must NOT be the surfaced error; got: {rendered}"
        );
    }

    /// Interrupt branch (review-4 Sites B+C): when the run is interrupted and a
    /// `failure.when` raises, `handle_terminal_evaluation_error` must surface a
    /// `failure`-named evaluation error and run `finalize` exactly once (the
    /// helper owns the finalize run; the interrupt branch must not also run a
    /// second finalize). Drives the fixed primitives directly since
    /// `run_harness_loop` is impractical from a unit test.
    #[test]
    fn interrupt_failure_when_raise_surfaces_failure_and_runs_finalize_once() {
        let fx = fixture(serde_json::json!({
            "failure": {
                "stderr": "fail",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            },
            "finalize": {
                "stderr": "final",
                "stack": [{"action": {"append_line": ["events.log", "finalized"]}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // The provider launched before the interrupt, so the Failure slot path
        // is taken (mirrors the interrupt branch's `execute_terminal_event`).
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let err_info =
            LifecycleErrorInfo::from_action_failure("interrupted", "user interrupted the run");
        let failure_outcome = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        )
        .outcome;

        let surfaced = handle_terminal_evaluation_error(
            &failure_outcome,
            "failure",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        );

        let report = surfaced.expect("failure `when:` raise must surface a halting error");
        let rendered = report.to_string();
        assert!(
            rendered.contains("`failure`"),
            "the surfaced error must name the failure event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
        // `handle_terminal_evaluation_error` runs `finalize` once internally; the
        // interrupt branch must NOT run it again (no recursive re-entry).
        assert_eq!(
            line_count(&fx.log_path),
            1,
            "finalize ran exactly once (handler-owned, no double finalize)"
        );
    }

    /// Interrupt branch (review-4 Sites B+C): a clean `failure` followed by a
    /// raising `finalize.when`. `handle_terminal_evaluation_error` returns
    /// `None` (failure did not raise), then the interrupt branch's own finalize
    /// run raises → a `finalize`-named evaluation error halts the run, and the
    /// `Ok((exit_code, ...))` happy path is NOT taken.
    #[test]
    fn interrupt_finalize_when_raise_surfaces_finalize_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "failure": { "stderr": "fail" },
            "finalize": {
                "stderr": "final",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let err_info =
            LifecycleErrorInfo::from_action_failure("interrupted", "user interrupted the run");
        let failure_outcome = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Failure,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        )
        .outcome;

        // The clean `failure` stack does not raise.
        assert!(
            handle_terminal_evaluation_error(
                &failure_outcome,
                "failure",
                &mut guard,
                &fx.materialized,
                &fx.source_path,
                Some(fx._dir.path()),
                &fx.term,
                &eng,
                std::time::Instant::now(),
            )
            .is_none(),
            "a clean failure must not surface an evaluation error"
        );

        // The interrupt branch then runs `finalize`, which raises here.
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            Some(&err_info),
            std::time::Instant::now(),
        );

        let surfaced: Option<color_eyre::eyre::Report> =
            finalize_outcome.evaluation_error.as_ref().map(|eval_info| {
                CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
            });
        let report = surfaced.expect("finalize `when:` raise must halt instead of returning Ok");
        let rendered = report.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the surfaced error must name the finalize event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
    }

    /// Start control-abort site (review-4 Site A): when the `start`
    /// control-dispatch aborts and `finalize.when` raises, the surfaced error
    /// must name `finalize` (the catch event's raise), not the original abort
    /// reason. The start-abort finalize runs with `None` `err` (no error info is
    /// available at that point), so this mirrors
    /// `terminal_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error`
    /// but with a `None` finalize `err`.
    #[test]
    fn start_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error() {
        let fx = fixture(serde_json::json!({
            "finalize": {
                "stderr": "final",
                "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
            launch_area: None,
            context: None,
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        // A terminal slot was taken before the control-abort decision, so the
        // subsequent `finalize` is eligible to fire (its run is gated on a
        // recorded terminal emission).
        guard.mark_provider_launched();
        guard.record_event_emission(LifecycleSignal::Failure);
        let eng = engine(fx._dir.path());

        // Replicate the Site A fix: finalize runs with `None` err; if it raises,
        // surface the finalize evaluation error in place of the abort reason.
        let finalize_outcome = run_lifecycle_event(
            &mut guard,
            LifecycleSignal::Finalize,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
            std::time::Instant::now(),
        );

        let surfaced_err: color_eyre::eyre::Report =
            if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
            } else {
                eyre!("original abort reason")
            };

        let rendered = surfaced_err.to_string();
        assert!(
            rendered.contains("`finalize`"),
            "the surfaced error must name the finalize event; got: {rendered}"
        );
        assert!(
            rendered.contains("evaluation error"),
            "the surfaced error must mention evaluation error; got: {rendered}"
        );
        assert!(
            !rendered.contains("original abort reason"),
            "the original abort reason must NOT be the surfaced error; got: {rendered}"
        );
    }
}

#[cfg(test)]
mod requeue_fallback_tests {
    use super::*;
    use indexmap::IndexMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Build a `requeue(...)`-shaped prompt state pointing at `source`.
    fn requeue_prompt_state(source: &Path) -> HarnessPromptState {
        HarnessPromptState {
            mode: HarnessPromptMode::Compose,
            source_path: source.to_path_buf(),
            original_ref: source.display().to_string(),
            base_prompt: None,
            overlay: IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
        }
    }

    /// Build a materialized prompt with the deferred-prompt body the requeue
    /// action is supposed to persist.
    fn requeue_materialized(prompt: &str) -> MaterializedHarnessPrompt {
        let frontmatter = serde_json::json!({"title": "deferred"});
        let live_frontmatter = MaterializedHarnessPrompt::live_cell_from(&frontmatter);
        MaterializedHarnessPrompt {
            frontmatter,
            prompt: prompt.to_string(),
            env_overrides: Vec::new(),
            inline_closure_plan: None,
            live_frontmatter,
        }
    }

    /// The cross-platform Windows-facing contract: when the rendezvous daemon
    /// is unreachable, `enqueue_requeue_entry` must NOT abort — it must
    /// return `Ok(())` and append exactly one durable fallback entry whose
    /// shape matches what the daemon would have received. This is the exact
    /// code path a Windows user takes (no daemon runs there), proven on the
    /// macOS host by pointing `RENDEZVOUS_SOCKET` at a non-existent socket.
    #[tokio::test]
    #[serial_test::serial(requeue_fallback)]
    async fn enqueue_requeue_entry_falls_back_to_durable_file_when_daemon_unreachable() {
        let fallback_dir = TempDir::new().expect("tempdir");
        let fallback_path: PathBuf = fallback_dir.path().join(REQUEUE_FALLBACK_FILE_NAME);
        let _socket_env =
            test_toolkit::EnvGuard::set_safe("RENDEZVOUS_SOCKET", "/tmp/does-not-exist-rs.sock");
        let _fallback_env =
            test_toolkit::EnvGuard::set_safe(REQUEUE_FALLBACK_DIR_ENV, fallback_dir.path());

        let workspace = TempDir::new().expect("workspace tempdir");
        let source_path = workspace.path().join("deferred.md");
        std::fs::write(&source_path, "defer body").expect("write source");
        let prompt_state = requeue_prompt_state(&source_path);
        let materialized = requeue_materialized("Body to defer through rendezvous\n");

        let result = enqueue_requeue_entry_async(
            Provider::Goose,
            &prompt_state,
            &materialized,
            Some(workspace.path()),
            "5m",
            Some("provider failed"),
        )
        .await;
        assert!(
            result.is_ok(),
            "daemon-unreachable requeue must succeed via fallback; got {:?}",
            result.err()
        );

        // Exactly one JSONL line was appended.
        let contents = std::fs::read_to_string(&fallback_path).expect("fallback file written");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 1, "exactly one fallback entry; got {lines:?}");
        let entry: serde_json::Value =
            serde_json::from_str(lines[0]).expect("fallback line is valid JSON");

        // The entry carries the same shape as AppendEntryRequest.
        assert_eq!(entry["source"], REQUEUE_SOURCE);
        assert_eq!(entry["level"], "info");
        assert_eq!(entry["session_id"], REQUEUE_SESSION_ID);
        assert_eq!(entry["owner_node_id"], "");
        let message = entry["message"].as_str().expect("message is a string");
        assert!(
            message.contains("deferred.md") && message.contains("5m"),
            "entry message should identify the prompt and delay; got {message:?}"
        );

        // `metadata_json` is embedded as a parsed object — its inner shape is
        // the contract a future daemon drain depends on.
        let metadata = &entry["metadata_json"];
        assert_eq!(metadata["kind"], "claudine.lifecycle.requeue");
        assert_eq!(metadata["provider"], "goose");
        assert_eq!(metadata["delay"], "5m");
        assert_eq!(metadata["reason"], "provider failed");
        assert_eq!(metadata["prompt"], "Body to defer through rendezvous\n");
        assert!(
            metadata["source_path"]
                .as_str()
                .is_some_and(|p| p.ends_with("deferred.md")),
            "metadata should record source_path; got {metadata}"
        );
    }

    /// A second requeue on the same fallback file appends rather than
    /// overwriting — the queue is durable and accumulates across runs.
    #[tokio::test]
    #[serial_test::serial(requeue_fallback)]
    async fn enqueue_requeue_entry_fallback_appends_across_calls() {
        let fallback_dir = TempDir::new().expect("tempdir");
        let fallback_path: PathBuf = fallback_dir.path().join(REQUEUE_FALLBACK_FILE_NAME);
        let _socket_env =
            test_toolkit::EnvGuard::set_safe("RENDEZVOUS_SOCKET", "/tmp/does-not-exist-rs.sock");
        let _fallback_env =
            test_toolkit::EnvGuard::set_safe(REQUEUE_FALLBACK_DIR_ENV, fallback_dir.path());

        let workspace = TempDir::new().expect("workspace tempdir");
        let source_path = workspace.path().join("deferred.md");
        std::fs::write(&source_path, "defer body").expect("write source");
        let prompt_state = requeue_prompt_state(&source_path);
        let materialized = requeue_materialized("body\n");

        enqueue_requeue_entry_async(
            Provider::Goose,
            &prompt_state,
            &materialized,
            Some(workspace.path()),
            "1m",
            None,
        )
        .await
        .expect("first enqueue");
        enqueue_requeue_entry_async(
            Provider::Goose,
            &prompt_state,
            &materialized,
            Some(workspace.path()),
            "2m",
            None,
        )
        .await
        .expect("second enqueue");

        let contents = std::fs::read_to_string(&fallback_path).expect("fallback file");
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2, "fallback file accumulates entries");
        let first: serde_json::Value = serde_json::from_str(lines[0]).expect("first entry parses");
        let second: serde_json::Value =
            serde_json::from_str(lines[1]).expect("second entry parses");
        assert_eq!(first["metadata_json"]["delay"], "1m");
        assert_eq!(second["metadata_json"]["delay"], "2m");
    }
}
