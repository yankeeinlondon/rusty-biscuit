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
) -> LifecycleEventOutcome {
    if matches!(signal, LifecycleSignal::Success | LifecycleSignal::Blocked) {
        // Take the terminal slot and fire the top-level communication FIRST
        // (before the stack), per the spec's top-level-before-stack contract.
        // If the slot was already taken by another terminal signal, do nothing.
        if !guard.record_event_emission(signal) {
            return LifecycleEventOutcome::default();
        }
        emit_lifecycle_top_level_already_recorded(
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
            return run_failure_event_for_downgrade(
                guard,
                materialized,
                source_path,
                repo_root,
                term,
                effect_engine,
                &action_error,
                loop_start,
            );
        }
        return outcome;
    }
    run_lifecycle_event(
        guard,
        signal,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        err,
        loop_start,
    )
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
    let (timing, current) = capture_lifecycle_globals(source_path, repo_root, loop_start);
    let ctx = build_lifecycle_stack_context_for_materialized(
        LifecycleSignal::Failure,
        materialized,
        source_path,
        repo_root,
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
) {
    let (timing, current) = capture_lifecycle_globals(source_path, repo_root, loop_start);
    let ctx = build_lifecycle_stack_context_for_materialized(
        signal,
        materialized,
        source_path,
        repo_root,
        term,
        guard.emitter(),
        guard.context().settings,
        guard.context().messaging,
        effect_engine,
        err,
        Some(&timing),
        Some(&current),
    );
    ctx.emit_top_level_for_signal(guard.config());
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
    let (timing, current) = capture_lifecycle_globals(source_path, repo_root, loop_start);
    let ctx = build_lifecycle_stack_context_for_materialized(
        signal,
        materialized,
        source_path,
        repo_root,
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
) {
    let terminal = if guard.provider_launched() {
        LifecycleSignal::Failure
    } else {
        LifecycleSignal::Blocked
    };
    run_lifecycle_event(
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
    run_lifecycle_event(
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
) {
    run_lifecycle_event(
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
    run_lifecycle_event(
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
    let (timing, current) = capture_lifecycle_globals(source_path, repo_root, loop_start);
    let ctx = build_lifecycle_stack_context_for_materialized(
        signal,
        materialized,
        source_path,
        repo_root,
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
        err,
        timing,
        current,
        base_dir,
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
    loop_start: std::time::Instant,
) -> (LifecycleTiming, LifecycleCurrent) {
    let base_dir = source_path.parent().or(repo_root);
    let current = match base_dir {
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
    if let Some(control) = outcome.control.as_ref() {
        match control {
            StackControl::Skip => TargetInitializeAction::ExitCleanly,
            StackControl::Error { reason } => {
                let msg = reason
                    .clone()
                    .unwrap_or_else(|| "lifecycle initialize error".to_string());
                let action_error = LifecycleErrorInfo::from_action_failure("error", msg.clone());
                run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Failure,
                    materialized,
                    source_path,
                    repo_root,
                    term,
                    effect_engine,
                    Some(&action_error),
                    loop_start,
                );
                run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Finalize,
                    materialized,
                    source_path,
                    repo_root,
                    term,
                    effect_engine,
                    Some(&action_error),
                    loop_start,
                );
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
            | StackControl::Requeue { .. } => TargetInitializeAction::Abort(eyre!(
                "lifecycle control action {control:?} is not valid at initialize"
            )),
        }
    } else if outcome.routes_to_failure(LifecycleSignal::Initialize) {
        let err = outcome.action_error.as_ref();
        run_lifecycle_event(
            lifecycle_guard,
            LifecycleSignal::Failure,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            err,
            loop_start,
        );
        run_lifecycle_event(
            lifecycle_guard,
            LifecycleSignal::Finalize,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            err,
            loop_start,
        );
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

const REQUEUE_SESSION_ID: &str = "claudine-deferred-execution";
const REQUEUE_SOURCE: &str = "claudine.lifecycle.requeue";
/// Environment variable that overrides the directory used by the
/// rendezvous deferred-queue fallback file. When unset the fallback
/// lives under `<config_dir>/claudine/rendezvous/deferred-queue.jsonl`.
const REQUEUE_FALLBACK_DIR_ENV: &str = "CLAUDINE_RENDEZVOUS_FALLBACK_DIR";
/// Fallback file name appended to the resolved fallback directory when no
/// rendezvous daemon is reachable. Each line is the JSON serialization of
/// the same `AppendEntryRequest` shape the daemon would have received, so a
/// future daemon can drain it verbatim.
const REQUEUE_FALLBACK_FILE_NAME: &str = "deferred-queue.jsonl";

/// Errors that can occur while persisting a `requeue(...)` deferred-prompt
/// entry.
///
/// The contract is daemon-first with a durable fallback (see
/// [`enqueue_requeue_entry`]). Only failures that lose the prompt surface
/// here; a daemon connect/append failure that successfully falls back to the
/// JSONL file is `Ok(())`.
#[derive(Debug, thiserror::Error)]
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
fn requeue_fallback_path() -> Option<std::path::PathBuf> {
    requeue_fallback_dir().map(|d| d.join(REQUEUE_FALLBACK_FILE_NAME))
}

/// Append one deferred-prompt entry to the durable fallback JSONL file as a
/// single line. Creates the parent directory if needed. Each line carries
/// the same shape as the `AppendEntryRequest` the daemon would have
/// received so a future daemon can drain the file verbatim.
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
    /// Target's `initialize` opted out via `skip()`; exit the run cleanly.
    ExitCleanly,
    /// Target's `initialize` could not be honored; abort with this error.
    Abort(color_eyre::eyre::Report),
    /// Target's `initialize` proxied again; repoint the loop and continue.
    Repoint { resolved: std::path::PathBuf },
}

/// Translate a terminal `failure`/`blocked` event's [`StackControl`] into a
/// loop action, applying the retry/resume/proxy/requeue runtime effect.
///
/// Reuses the existing redirect/resume substrate: a retry bumps the attempt
/// and `continue`s; a resume seeds `next_resume_session_id` +
/// `next_prompt_override`; a proxy swaps `source_path`/`original_ref` and
/// resets the guard for a fresh `initialize`; a requeue records the
/// materialized prompt in rendezvous and exits the current run.
#[allow(clippy::too_many_arguments)]
fn dispatch_terminal_control(
    signal: LifecycleSignal,
    outcome: &LifecycleEventOutcome,
    attempt: u32,
    budgets: &mut ControlBudgets,
    session_id: Option<&str>,
    profile: &dyn super::super::profile::WrapperProfile,
    provider: Provider,
    prompt_state: &mut HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
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

    let dispatch = decide_control(signal, control, attempt, budget, session_id.is_some());

    match dispatch {
        ControlDispatch::Stop | ControlDispatch::Exhausted => TerminalControlAction::Fallthrough,
        ControlDispatch::Retry { delay, from_blocked } => {
            if show_checks {
                let what = if from_blocked { "pre-flight" } else { "the agent" };
                claudine::harness::report::report_handler_engagement(
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
                claudine::harness::report::report_handler_engagement(
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
                claudine::harness::report::report_handler_engagement(
                    &format!("lifecycle proxy: handing off to {}", resolved.display()),
                    term,
                );
            }
            // Re-enter at attempt 1 so the target document gets a clean
            // pre-flight / freeze cycle rather than inheriting the proxying
            // document's attempt count.
            TerminalControlAction::Continue { next_attempt: 1 }
        }
        ControlDispatch::Requeue { delay, reason } => {
            match enqueue_requeue_entry(
                provider,
                prompt_state,
                materialized,
                repo_root,
                &delay,
                reason.as_deref(),
            ) {
                Ok(()) => {
                    if show_checks {
                        claudine::harness::report::report_handler_engagement(
                            &format!(
                                "lifecycle requeue: deferred {} for {delay}",
                                prompt_state.source_path.display()
                            ),
                            term,
                        );
                    }
                    TerminalControlAction::Fallthrough
                }
                Err(err) => TerminalControlAction::Abort(
                    CompositionError::LifecycleRequeueEnqueueFailed {
                    source_path: prompt_state.source_path.clone(),
                    delay,
                    reason,
                        message: err.to_string(),
                }
                .into(),
                ),
            }
        }
    }
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
            .inspect_err(|e| {
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
                };
                emit_blocked_finalize_with_err(
                    lifecycle_guard,
                    &empty,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    &err_info,
                    loop_start,
                );
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
                    emit_blocked_finalize_with_err(
                        lifecycle_guard,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        &err_info,
                        loop_start,
                    );
                    return Err(e.into());
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
                        claudine::harness::report::report_handler_engagement(
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
        .inspect_err(|e| {
            let err_info = LifecycleErrorInfo::from_harness_error(e);
            emit_blocked_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            );
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
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Blocked,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            );
            run_lifecycle_event(
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
                run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Blocked,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    Some(&err_info),
                    loop_start,
                );
                run_lifecycle_event(
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
        if let Some(ref control) = start_outcome.control {
            match control {
                StackControl::Error { reason } => {
                    let msg = reason
                        .clone()
                        .unwrap_or_else(|| "lifecycle start error".to_string());
                    let err_info =
                        LifecycleErrorInfo::from_action_failure("error", msg.as_str());
                    run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Failure,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        Some(&err_info),
                        loop_start,
                    );
                    run_lifecycle_event(
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
                    return Err(eyre!(msg));
                }
                StackControl::Stop => {}
                _ => {
                    return Err(eyre!(
                        "lifecycle control action {control:?} is not valid at start"
                    ));
                }
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
                    loop_start,
                );
                let ctx = build_lifecycle_stack_context_for_materialized(
                    LifecycleSignal::Failure,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    lifecycle_guard.emitter(),
                    lifecycle_guard.context().settings,
                    lifecycle_guard.context().messaging,
                    &effect_engine,
                    start_outcome.action_error.as_ref(),
                    Some(&timing),
                    Some(&current),
                );
                lifecycle_guard.run_event_stack(LifecycleSignal::Failure, &ctx);
            }
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
        .inspect_err(|e| {
            let err_info = LifecycleErrorInfo::from_action_failure("harness_launch", e.to_string());
            emit_failure_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            );
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
            .inspect_err(|e| {
                let err_info =
                    LifecycleErrorInfo::from_action_failure("harness_attempt", e.to_string());
                emit_failure_finalize_with_err(
                    lifecycle_guard,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    &err_info,
                    loop_start,
                );
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
            execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            );
            run_lifecycle_event(
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
            );
            // A `failure.stack` may end in a lifecycle control action
            // (retry/resume/requeue/proxy). Dispatch it before finalizing so a
            // re-entry skips finalize for this iteration.
            match dispatch_terminal_control(
                LifecycleSignal::Failure,
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
                    run_lifecycle_event(
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
                    return Err(err);
                }
                TerminalControlAction::Fallthrough => {}
            }
            run_lifecycle_event(
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
            );
            match dispatch_terminal_control(
                LifecycleSignal::Failure,
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
                    run_lifecycle_event(
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
                    return Err(err);
                }
                TerminalControlAction::Fallthrough => {}
            }
            run_lifecycle_event(
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
            return Err(eyre!("{fail_msg}"));
        }

        // Post-check validation has been removed; a successful provider run
        // proceeds directly to the success lifecycle event.
        execute_terminal_event(
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
        run_lifecycle_event(
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
            capture_lifecycle_globals(Path::new("prompt.md"), Some(Path::new(".")), loop_start);

        assert!(timing.document_ms.is_some(), "document_ms is populated");
        assert!(timing.total_ms.is_some(), "total_ms is populated");
        assert!(
            current.env.is_object() && !current.env.as_object().unwrap().is_empty(),
            "current.env is a non-empty environment snapshot"
        );
    }

    /// The `LifecycleLookup` resolves `current.env.*` and `timing.document_ms`
    /// from the globals the harness-loop builder attaches — proving the wiring
    /// reaches expression evaluation, not just the struct fields.
    #[test]
    #[serial_test::serial(env_loop_control_current)]
    fn attached_globals_resolve_through_lookup() {
        use claudine::composition::lifecycle_context::LifecycleLookup;
        use darkmatter::markdown::compose::expression::{
            EvaluationLookup, evaluate, is_truthy, parse,
        };

        let key = "CLAUDINE_TEST_LOOP_CONTROL_LATE_BIND";
        // SAFETY: serialized via #[serial]; no other thread reads this var.
        unsafe { std::env::set_var(key, "ready") };
        let (timing, current) =
            capture_lifecycle_globals(Path::new("prompt.md"), Some(Path::new(".")), loop_start_now());
        unsafe { std::env::remove_var(key) };

        let fm = serde_json::Map::new();
        let lookup = LifecycleLookup::new(&fm)
            .with_timing(&timing)
            .with_current(&current);

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
        MaterializedHarnessPrompt {
            frontmatter,
            prompt: String::new(),
            env_overrides: Vec::new(),
            inline_closure_plan: None,
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
    /// stack ends in `error('downgraded')` so it routes to `failure`.
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
                "stack": [{"action": "append_line('events.log', 'ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
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

    #[test]
    fn success_stack_error_routes_to_failure_keeps_success_comm_before_failure() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "succeeded",
                "stack": [{"action": ["append_line('events.log', 'ran')", "error('downgraded')"]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
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
                "stack": [{"action": "append_line('events.log', 'ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
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
                "stack": [{"action": "stderr('success-stack')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
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
                "stack": [{"action": "stderr('blocked-stack')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
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
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            },
            "finalize": {
                "stderr": "finalized",
                "stack": [{"action": "append_line('events.log', 'finalize-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
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
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            },
            "finalize": {
                "stack": [{"action": "append_line('events.log', 'finalize-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
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
                "stack": [{"action": ["append_line('events.log', 'ran')", "error('downgraded')"]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
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
            action_error: None,
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
            LifecycleSignal::Failure,
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
    fn dispatch_retry_exhausts_after_budget() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
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
            LifecycleSignal::Failure,
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
            LifecycleSignal::Failure,
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
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Resume {
            message: "x".to_string(),
            max_attempts: 1,
        });
        let action = dispatch_terminal_control(
            LifecycleSignal::Failure,
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
                    err.to_string().contains("requires a provider session"),
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
            LifecycleSignal::Failure,
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
    fn dispatch_requeue_aborts_when_enqueue_fails() {
        let fx = fixture(serde_json::json!({}));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let outcome = outcome_with(StackControl::Requeue {
            delay: "5m".to_string(),
            reason: Some("later".to_string()),
        });
        let action = dispatch_terminal_control(
            LifecycleSignal::Failure,
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
                    err.to_string().contains("requeue")
                        && err.to_string().contains("5m")
                        && err.to_string().contains("rendezvous"),
                    "unexpected: {err}"
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
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let action = dispatch_terminal_control(
            LifecycleSignal::Failure,
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
        };
        let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
        let mut state = prompt_state(&fx.source_path);
        let mut budgets = ControlBudgets::default();
        let action = dispatch_terminal_control(
            LifecycleSignal::Failure,
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
                    {"action": "append_line('events.log', 'blocked-kind=' + err.kind)"},
                    {"action": "append_line('events.log', 'blocked-variant=' + err.variant)"},
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": "append_line('events.log', 'finalize-msg=' + err.msg)"},
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
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            },
            "finalize": {
                "stack": [{"action": "append_line('events.log', 'finalize-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
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
                "stack": [{"action": "append_line('events.log', 'blocked-kind=' + err.kind)"}]
            },
            "finalize": {
                "stack": [{"action": "append_line('events.log', 'finalize-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
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
                    {"action": "append_line('events.log', 'failure-kind=' + err.kind)"},
                    {"action": "append_line('events.log', 'failure-variant=' + err.variant)"},
                ]
            },
            "finalize": {
                "stack": [
                    {"when": "err", "action": "append_line('events.log', 'finalize-msg=' + err.msg)"},
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
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            },
            "finalize": {
                "stack": [{"action": "append_line('events.log', 'finalize-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
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
        MaterializedHarnessPrompt {
            frontmatter: serde_json::json!({"title": "deferred"}),
            prompt: prompt.to_string(),
            env_overrides: Vec::new(),
            inline_closure_plan: None,
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
