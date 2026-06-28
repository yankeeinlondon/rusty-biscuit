//! Wrapper-grade composition executor.
//!
//! [`execute_composition_request`] is the single execution pipeline for
//! both `claudine compose` and `claudine inline-compose`. It provides
//! full wrapper-grade behavior: environment setup, harness detection from
//! effective (composed) frontmatter, structured streaming, and inline
//! closure.

use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::{
    DefaultLifecycleEmitter, LifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext,
    LifecycleSignal,
};
use claudine::composition::lifecycle_executor::{
    LifecycleEventOutcome, StackControl, StackExecutionContext, SystemShellRunner,
};
use claudine::composition::{
    AgentResolutionState, CompositionClosurePlan, CompositionError, CompositionExecutionRequest,
    CompositionMode, InlineClosurePlan, ModelResolutionReason, ResolvedExecutionTarget,
    SelectionReason, SessionInteractivitySource, agent_state_breakdown, build_installed_snapshot,
    build_picker_plan, classify_agent_resolution, invalid_agent_message,
    resolve_target_non_tty_with_catalog,
};
use claudine::config::claudine_config::ProviderModelOverride;
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use darkmatter::effects::EffectEngine;
use inquire::Select;
use sniff::programs::InstalledAiClients;

use super::env;
use super::profile::{self, WrapperProfile};
use super::{
    HarnessPromptMode, HarnessPromptState, StructuredCodexOutput, apply_composition_shell_overrides,
    build_harness_shell_options_with_cache, materialized_harness_prompt_from_prepared,
    resolve_binary_path_direct, run_harness_loop, structured_verbosity, wrap_terminal,
};
use super::exec::switch_process_cwd;
use crate::log;

pub(crate) mod dry_run;
pub(crate) mod prep_context;
pub(crate) mod target;
pub(crate) mod timeouts;

pub(crate) use prep_context::CompositionPrepContext;
pub(crate) use target::{
    agent_prompt_message, composition_dispatch_context, eagerly_resolve_target,
    install_agent_env_for_composition, refresh_for_model_validation, resolve_execution_target,
    scoped_picker_plan_for_state,
};
pub(crate) use timeouts::{
    TimeoutResolutionInput, build_prompt_timing_context, format_interactive_timeout_conflict,
    frontmatter_timeout_duration, resolve_single_timeout, resolve_timeouts,
};
#[cfg(test)]
pub(crate) use target::{picker_scope_for_state, resolve_live_target_with_tty};

/// W0 instrumentation counter: increments every time
/// [`select_launch_workspace`] falls back to the legacy
/// `env::resolve_launch_workspace_context` call.
///
/// The fallback path performs a fresh `detect_git` + `detect_repo`
/// filesystem scan, which is exactly the redundancy W0 was designed to
/// remove. The counter is process-global so a regression test can
/// observe it across whatever spawn / fixture machinery the test uses
/// without having to thread an injectable counter through the entire
/// composition request type. Tests reset it via
/// [`reset_launch_workspace_fallbacks_for_tests`].
static LAUNCH_WORKSPACE_FALLBACK_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Choose the launch workspace context for the executor.
///
/// Returns the precomputed `prep` value when present (the W0 hot path).
/// Falls back to the legacy `env::resolve_launch_workspace_context` walk
/// only for library callers that don't thread a `CompositionPrepContext`
/// (none in the production CLI).
///
/// The fallback branch increments [`LAUNCH_WORKSPACE_FALLBACK_COUNT`] so
/// regression tests can prove the production hot path stays on the
/// no-walk branch even after future refactors.
pub(crate) fn select_launch_workspace(
    prep: Option<&env::LaunchWorkspaceContext>,
    launch_cwd: &Path,
    source_repo_root: Option<&Path>,
) -> env::LaunchWorkspaceContext {
    if let Some(p) = prep {
        return p.clone();
    }
    LAUNCH_WORKSPACE_FALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
    env::resolve_launch_workspace_context(launch_cwd, source_repo_root)
}

/// Test-only: snapshot of the fallback counter.
#[cfg(test)]
pub(crate) fn launch_workspace_fallback_count_for_tests() -> usize {
    LAUNCH_WORKSPACE_FALLBACK_COUNT.load(Ordering::SeqCst)
}

/// Test-only: reset the fallback counter so an isolated test can
/// observe a clean baseline.
#[cfg(test)]
pub(crate) fn reset_launch_workspace_fallbacks_for_tests() {
    LAUNCH_WORKSPACE_FALLBACK_COUNT.store(0, Ordering::SeqCst);
}

/// Enforce the `--repo` legacy hard-fail contract when prep-time
/// launch-context detection failed.
///
/// `CompositionPrepContext` runs a single shared `sniff::detect_with_plan`
/// scan and falls back to a default `LaunchContext` on failure so best-
/// effort consumers can keep going. `--repo` is not a best-effort
/// consumer: it requires real repo detection. When the prep scan failed
/// **and** `--repo` is set, surface the captured sniff error as a hard
/// run abort, matching the behavior of the legacy non-prep path that
/// called `LaunchContext::from_cwd` directly.
fn enforce_repo_launch_detection(
    repo: bool,
    prep_launch_detection_error: Option<&str>,
) -> Result<()> {
    if repo && let Some(error) = prep_launch_detection_error {
        return Err(eyre!(
            "--repo requires startup repo detection, but launch-context detection failed: {error}"
        ));
    }
    Ok(())
}

/// Outcome of running the pre-flight `blocked` + `finalize` lifecycle events.
///
/// `Control` carries the blocked stack's flow-control action (if any) for the
/// caller to dispatch. `EvaluationError` carries a typed
/// [`CompositionError::LifecycleEvaluationError`] raised by the `blocked` or
/// `finalize` stack itself — it takes precedence over the original pre-flight
/// failure because a lifecycle expression crash is the actionable cause.
#[derive(Debug)]
enum PreflightBlockedOutcome {
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
/// This helper mirrors the [`StackExecutionContext`] pattern the
/// `initialize` event uses (see `init_ctx` in
/// [`execute_composition_request_inner_with_guard`]): the context borrows
/// the *local* `emitter`/`settings`/etc. — not the guard — so
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
/// Mirrors `harness_orch::loop_control::surface_catch_evaluation_error` but
/// returns a [`CompositionError`] (the pre-flight outcome carries one, not a
/// `Report`): the original `blocked` raise was already emitted as `early`; if a
/// catch event raised a newer crash, that one is emitted now (no further
/// lifecycle events fire) and marked emitted, else `early` surfaces unchanged.
fn surface_preflight_catch_error(
    source_path: &Path,
    failure_outcome: Option<&LifecycleEventOutcome>,
    finalize_outcome: Option<&LifecycleEventOutcome>,
    early: CompositionError,
    term: &Terminal,
) -> CompositionError {
    if let Some(fin_info) = finalize_outcome.and_then(|o| o.evaluation_error.as_ref()) {
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
        early
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_preflight_blocked_and_finalize(
    guard: &mut LifecycleRunGuard<'_>,
    effect_engine: &EffectEngine,
    emitter: &dyn LifecycleEmitter,
    settings: &claudine::events::GlobalSettings,
    messaging: &claudine::messaging::RuntimeMessagingSettings,
    term: &Terminal,
    source_path: &Path,
    repo_root: Option<&Path>,
    base_dir: Option<&Path>,
    ctx_base_dir: Option<&Path>,
    prepared_context: Option<&darkmatter::markdown::compose::ComposeContext>,
    frontmatter: &serde_json::Map<String, serde_json::Value>,
    document_start: std::time::Instant,
    err_info: claudine::composition::LifecycleErrorInfo,
) -> PreflightBlockedOutcome {
    let timing = claudine::composition::LifecycleTiming::from_instants(
        document_start,
        None,
        std::time::Instant::now(),
    );
    // `current.ctx.*` follows the launch area like event-time `ctx.*` capture.
    let current_anchor = ctx_base_dir.or(base_dir).unwrap_or(source_path);
    let current =
        claudine::composition::LifecycleCurrent::capture_at_event(current_anchor);

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
    PreflightBlockedOutcome::Control(blocked_outcome.control)
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
fn preflight_blocked_control_error(
    control: Option<StackControl>,
    source_path: &Path,
) -> Option<CompositionError> {
    match control? {
        StackControl::Resume { .. } => Some(CompositionError::LifecycleResumeWithoutSession {
            source_path: source_path.to_path_buf(),
        }),
        StackControl::Retry { .. } => Some(setup_phase_deferred("blocked", "retry", source_path)),
        StackControl::Defer { .. } => Some(CompositionError::LifecycleDeferNotImplemented {
            source_path: source_path.to_path_buf(),
        }),
        StackControl::Proxy { .. } => Some(setup_phase_deferred("blocked", "proxy", source_path)),
        StackControl::Stop | StackControl::Skip | StackControl::Error { .. } => None,
    }
}

/// Build the typed setup-phase-deferred-recovery error.
fn setup_phase_deferred(event: &str, action: &str, source_path: &Path) -> CompositionError {
    CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
        source_path: source_path.to_path_buf(),
        event: event.to_string(),
        action: action.to_string(),
    }
}

/// Result of executing a single composition step through the wrapper pipeline.
pub(crate) struct SingleCompositionOutcome {
    /// The process exit code.
    pub exit_code: i32,
    /// The provider that ran the step.
    pub provider: Provider,
    /// Execution perf metadata, when `--perf` was enabled.
    pub agent_perf: Option<crate::perf::AgentExecutionPerf>,
    /// Iteration-level summary signals lifted from the structured stream
    /// for consumption by the `compose --loop` orchestrator.
    ///
    /// Non-dry-run composition carries the harness loop's terminal-attempt
    /// structured summary signals when available (the rate-limit trailer or
    /// watchdog `error_kind`). `None` for the dry-run path, which never
    /// launches a provider.
    pub iteration_signals: Option<IterationSummarySignals>,
    /// Terminal lifecycle signal emitted by the harness loop, if known.
    ///
    /// Used by the loop engine to apply `fail_fast` semantics and to sequence
    /// the post-`finalize` loop gate.
    pub terminal_signal: Option<LifecycleSignal>,
}

/// Iteration-level signals lifted from the per-iteration
/// [`claudine::stream::summary::StreamExecutionSummary`] so the
/// `compose --loop` orchestrator can drive rate-limit-aware iteration and
/// build [`claudine::composition::CompositionError::LoopIterationFailed`]
/// with an honest cause.
#[derive(Debug, Default, Clone)]
pub(crate) struct IterationSummarySignals {
    /// Rate-limit trailer observed during the iteration. May be present on
    /// both successful and failed iterations.
    pub rate_limit: Option<claudine::stream::summary::RateLimitInfo>,
    /// Structured `error_kind` (e.g. `step_timeout`, `wall_clock_timeout`,
    /// `usage_limit_reached`). Mirrors the JSONL session_end row's
    /// `extra.exit_reason`.
    pub exit_reason: Option<String>,
    /// Human-readable failure detail from the iteration's summary, when
    /// present (e.g. "no stream activity for 30m; terminating due to
    /// step_timeout").
    pub error_message: Option<String>,
    /// Resolved provider identifier (e.g. `"k2p6"`) from the iteration's
    /// summary, when known. Carried into [`CompositionError::LoopRateLimited`]
    /// for honest attribution.
    pub provider_id: Option<String>,
    /// Resolved model identifier (e.g. `"kimi-for-coding"`) from the
    /// iteration's summary, when known.
    pub model_id: Option<String>,
}

impl IterationSummarySignals {
    /// Extract the loop-relevant fields from a fully-built
    /// [`claudine::stream::summary::StreamExecutionSummary`].
    pub fn from_summary(summary: &claudine::stream::summary::StreamExecutionSummary) -> Self {
        Self {
            rate_limit: summary.rate_limit.clone(),
            exit_reason: summary.error_kind.clone(),
            error_message: summary.error_message.clone(),
            // Use the Provider enum's display form (e.g. "opencode"). The
            // finer-grained AI-SDK provider (e.g. "k2p6") typically lives
            // inside `rate_limit.message`.
            provider_id: Some(summary.provider.to_string()),
            model_id: summary.model.clone(),
        }
    }
}

/// Render the one-line execution header for a composition run.
///
/// Shared by the up-front emit in `compose` / `inline-compose` (which
/// resolves the agent eagerly so the line appears immediately) and the
/// in-pipeline emit for callers that did not pre-render it.
///
/// Returns `false` without emitting when `provider` has no wrapper
/// profile, so the caller leaves the header to the executor rather than
/// silently dropping it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_execution_header(
    provider: Provider,
    yolo: bool,
    session_interactive: bool,
    detail_requested: bool,
    repo: bool,
    is_inline: bool,
    sequence: bool,
    operation: Option<&str>,
    file_ref: &str,
    package_context: Option<claudine::composition::PackageContext>,
    term: &Terminal,
) -> bool {
    let Some(profile) = profile::profile_for_provider(provider) else {
        return false;
    };
    let compose_display = if is_inline {
        crate::output::ComposeDisplay::InlineCompose
    } else {
        crate::output::ComposeDisplay::Compose
    };
    let header_env_plan = env::EnvPlan {
        package_context,
        ..Default::default()
    };
    crate::output::log_wrapper_header(
        profile,
        yolo,
        !session_interactive,
        session_interactive,
        detail_requested,
        repo,
        Some(&compose_display),
        sequence,
        operation,
        None, // no inline prompt text for compose
        Some(file_ref),
        &header_env_plan,
        term,
    );
    true
}

/// Execute a composition request through the wrapper-grade pipeline.
///
/// Handles provider selection, environment setup, harness detection from
/// the effective (composed) frontmatter, structured streaming, and inline
/// closure. All downstream decisions read from
/// `request.prepared.effective_frontmatter`, never from raw source state.
pub(crate) fn execute_composition_request(
    request: CompositionExecutionRequest,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
    perf_enabled: bool,
) -> Result<i32> {
    let outcome =
        execute_composition_request_inner(request, verbose, startup_timings, perf_enabled)?;
    Ok(outcome.exit_code)
}

/// Inner implementation that returns the full [`SingleCompositionOutcome`].
///
/// The public [`execute_composition_request`] wraps this to return just
/// the exit code; callers that need provider/reason metadata (e.g. the
/// sequence orchestrator) can call this directly.
pub(crate) fn execute_composition_request_inner(
    request: CompositionExecutionRequest,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
    perf_enabled: bool,
) -> Result<SingleCompositionOutcome> {
    execute_composition_request_inner_with_guard(
        request,
        verbose,
        startup_timings,
        perf_enabled,
        None,
        false,
    )
}

/// Execute a single composition attempt using an externally-managed lifecycle
/// guard.
///
/// Used by the loop engine for iteration 2+. The caller has already emitted
/// `initialize` and run schema/shell preflight once; this function skips those
/// and emits only `start`, the terminal event, and `finalize` through the
/// shared guard.
pub(crate) fn execute_composition_attempt(
    request: CompositionExecutionRequest,
    verbose: u8,
    perf_enabled: bool,
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    skip_preflight: bool,
) -> Result<SingleCompositionOutcome> {
    execute_composition_request_inner_with_guard(
        request,
        verbose,
        None,
        perf_enabled,
        Some(guard),
        skip_preflight,
    )
}

fn execute_composition_request_inner_with_guard(
    request: CompositionExecutionRequest,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
    perf_enabled: bool,
    external_guard: Option<&mut claudine::composition::LifecycleRunGuard<'_>>,
    skip_preflight: bool,
) -> Result<SingleCompositionOutcome> {
    let mut perf_collector = if perf_enabled {
        startup_timings.map(|timings| {
            crate::perf::CommandPerfCollector::new_with_composition(
                "Composition",
                timings,
                request.prepared.compose_perf.clone(),
            )
        })
    } else {
        None
    };
    // Local checkpoint origin for the env-setup sub-stage chain only. The
    // headline is the threaded wall-clock baseline sampled at report build,
    // not this mid-flight timer (TM-1).
    let mut last_checkpoint = std::time::Instant::now();
    // Stable anchor for lifecycle `timing.document_ms`: the moment this
    // document's execution began. Unlike `last_checkpoint` (reset by every
    // sub-stage record), this is never advanced, so `document_ms` measures
    // elapsed time since the document started, not since the last sub-stage.
    let document_start = last_checkpoint;
    /// Helper to record a named sub-stage timing and reset the checkpoint.
    fn record_substage(
        collector: &mut Option<crate::perf::CommandPerfCollector>,
        checkpoint: &mut std::time::Instant,
        name: &'static str,
    ) {
        if let Some(c) = collector {
            let elapsed = checkpoint.elapsed();
            c.mark_substage(name, elapsed);
            *checkpoint = std::time::Instant::now();
        }
    }

    let _span = tracing::info_span!("composition_prepare").entered();

    let term = wrap_terminal();

    // Early dry-run seam for unresolved agent states. When the upstream
    // caller left `resolved_target` as `None` (e.g. compose --dry-run with
    // no explicit provider and no auto-selectable frontmatter hint), skip
    // provider selection, header emission, and harness preflight entirely.
    // The renderer reports the classified state in the metadata table.
    if request.dry_run && request.resolved_target.is_none() {
        let render = dry_run::DryRunRender::from_request(&request);

        crate::log::data(&render.body);
        crate::log::message(&dry_run::render_hr(&term));
        crate::log::message(&dry_run::render_frontmatter_heading(&term));
        crate::log::message("");
        crate::log::message(&dry_run::render_frontmatter(&render.frontmatter, &term));
        crate::log::message(&dry_run::render_metadata_table(&render, &term));

        if let Some(collector) = perf_collector.as_mut() {
            collector.set_dry_run();
        }
        let outcome = SingleCompositionOutcome {
            exit_code: 0,
            // Provider is intentionally unknown for unresolved dry-run;
            // the placeholder is never displayed because the caller returns
            // immediately on the dry-run path.
            provider: claudine::provider::Provider::Claude,
            agent_perf: None,
            iteration_signals: None,
            terminal_signal: None,
        };
        if let Some(collector) = perf_collector {
            crate::perf::emit_report(&collector.into_report());
        }
        return Ok(outcome);
    }

    let launch_cwd = std::env::current_dir()?;
    let detail_requested = verbose > 0;
    let quiet = request.quiet;
    let silent = request.silent;
    let show_checks = !silent;

    let source_repo_root = request.prepared.source_repo_root.as_deref();
    let launch_workspace = select_launch_workspace(
        request.prep_launch_workspace.as_ref(),
        &launch_cwd,
        source_repo_root,
    );

    // -- Provider detection and selection ---------------------------------

    // If a target was already resolved upstream (sequence review, non-TTY
    // preflight, eager resolution via `CompositionPrepContext`), reuse it
    // and skip the entire provider/config/catalog discovery phase. This
    // removes a duplicate `InstalledAiClients::new()` PATH scan and a
    // redundant `load_selection_config()` (which itself runs `detect_git`
    // off the launch CWD) on the hot path.
    let target = resolve_execution_target(&request, &launch_cwd, source_repo_root)?;

    let provider = target.provider;
    let _selection_reason = match target.provider_reason {
        claudine::composition::ProviderResolutionReason::ExplicitFlag => {
            SelectionReason::ExplicitProvider
        }
        claudine::composition::ProviderResolutionReason::FrontmatterSingle
        | claudine::composition::ProviderResolutionReason::FrontmatterList => {
            SelectionReason::FrontmatterHint
        }
        claudine::composition::ProviderResolutionReason::FavoriteAgent => {
            SelectionReason::ConfigFavorite
        }
        _ => SelectionReason::InteractiveChoice,
    };
    let is_inline = matches!(request.prepared.closure, CompositionClosurePlan::Inline(_));
    record_substage(
        &mut perf_collector,
        &mut last_checkpoint,
        "target resolution",
    );

    // -- Profile, binary, arguments, environment --------------------------

    let profile = profile::profile_for_provider(provider)
        .ok_or_else(|| eyre!("'{}' cannot be wrapped", provider))?;
    let binary_path = resolve_binary_path_direct(profile, request.installed_snapshot.as_ref())?;

    // -- Inline + interactive check ---------------------------------------

    if request.session_interactive && is_inline && !profile.supports_interactive_inline_closure() {
        return Err(CompositionError::InlineInteractiveUnsupported {
            provider: provider.to_string(),
            source_kind: request.session_interactive_source,
        }
        .into());
    }

    let effective_non_interactive = !request.session_interactive;

    // -- Header ----------------------------------------------------------
    // `compose` / `inline-compose` resolve the agent eagerly and render
    // the execution line up front (before the expensive prepare/compose
    // work) so the user sees it immediately; `request.header_emitted` is
    // set in that case and we must not re-emit. The in-pipeline emit below
    // covers callers that did not pre-render — the dry-run-unresolved
    // corner (agent only known after resolution here) and sequence steps.

    if !silent && !request.header_emitted {
        emit_execution_header(
            provider,
            request.yolo,
            request.session_interactive,
            detail_requested,
            request.repo,
            is_inline,
            request.sequence,
            request.operation.as_deref(),
            &request.file_ref,
            launch_workspace.package_context.clone(),
            &term,
        );
    }

    record_substage(&mut perf_collector, &mut last_checkpoint, "header env plan");

    let needs_mcp_shadow_home = (request.mcp || !request.mcp_use.is_empty())
        && matches!(provider, Provider::Codex | Provider::Gemini);
    let needs_repo_shadow_home = request.repo;
    let raw_agent_params: Vec<String> = std::env::args().skip(1).collect();
    let yolo_enabled = request.yolo;
    let mut env_plan = env::build_child_env_with_launch(
        profile,
        provider,
        &request.include,
        yolo_enabled,
        request.session_interactive,
        &raw_agent_params,
        &[],
        needs_repo_shadow_home,
        needs_mcp_shadow_home || needs_repo_shadow_home,
        launch_workspace.clone(),
        perf_enabled,
    )?;

    // -- Operation env override -----------------------------------------------

    if let Some(ref op) = request.operation {
        env_plan.env.insert("OPERATION".into(), op.clone().into());
    }

    // -- Request-level env overrides ------------------------------------------
    // These cover execution-time env vars that must also be visible during
    // composition (preflight, prompt interpolation). Sequence execution uses
    // this to propagate `FAIL_FAST` into the child process env.
    for (key, value) in &request.env_overrides {
        env_plan
            .env
            .insert(key.clone().into(), value.clone().into());
    }

    // `child env build` carries a measured breakdown (env sanitize / shadow
    // home sync → repo root detect) so the substage's cost is itemized rather
    // than opaque. The launch-child root is threaded through, so `repo root
    // detect` is microsecond-scale local work and the shadow sync's filesystem
    // linking is what remains; only the fallback (no supplied root) still pays
    // the sniff git walk. The children are `Breakdown`, so they do not enter
    // the substage's reconciliation (TR-1).
    if let Some(c) = perf_collector.as_mut() {
        let elapsed = last_checkpoint.elapsed();
        c.mark_substage_with_children(
            "child env build",
            elapsed,
            std::mem::take(&mut env_plan.perf_substages),
        );
        last_checkpoint = std::time::Instant::now();
    }

    let mut effective_prompt = request.prepared.prompt.clone();
    let mut mcp_extra_args = Vec::new();
    if request.mcp || !request.mcp_use.is_empty() {
        use claudine::mcp::catalog::McpCatalogStore;
        use claudine::mcp::inject::injector_for_provider;
        use claudine::mcp::session::{compute_session_set, lex_tags};

        let repo_root_ref = source_repo_root.or(env_plan.repo_root.as_deref());
        let _ = super::bootstrap_mcp_state(repo_root_ref)?;
        let catalog =
            McpCatalogStore::load().map_err(|e| eyre!("failed to load MCP catalog: {e}"))?;
        let (cleaned_prompt, prompt_tags) = lex_tags(&effective_prompt);
        let prompt_is_interactive = request.session_interactive
            && std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal();
        let session = compute_session_set(
            &catalog,
            repo_root_ref,
            &request.mcp_use,
            &prompt_tags,
            |tag, _tier, candidates| {
                if request.strict || effective_non_interactive || !prompt_is_interactive {
                    return None;
                }
                Select::new(
                    &format!("`#{tag}` matched multiple MCP servers. Choose one:"),
                    candidates.to_vec(),
                )
                .prompt()
                .ok()
            },
        )
        .map_err(|e| eyre!("MCP session error: {e}"))?;

        if !session.missing_tags.is_empty() {
            if request.strict {
                return Err(eyre!(
                    "unresolved MCP tag(s): {}",
                    session
                        .missing_tags
                        .iter()
                        .map(|tag| format!("#{tag}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !silent {
                for tag in &session.missing_tags {
                    log::warn(&format!("tag `#{tag}` was not found in the MCP catalog"));
                }
            }
        }
        if !session.ambiguous_tags.is_empty() {
            if request.strict || effective_non_interactive {
                let message = session
                    .ambiguous_tags
                    .iter()
                    .map(|tag| format!("#{} -> {}", tag.tag, tag.candidates.join(", ")))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(eyre!("ambiguous MCP tag(s): {message}"));
            }
            if !silent {
                for tag in &session.ambiguous_tags {
                    log::warn(&format!(
                        "tag `#{}` is ambiguous ({}); dropped from session",
                        tag.tag,
                        tag.candidates.join(", ")
                    ));
                }
            }
        }

        effective_prompt = session.cleaned_prompt.unwrap_or(cleaned_prompt);

        if let Some(injector) = injector_for_provider(provider) {
            if !session.servers.is_empty() {
                if needs_mcp_shadow_home && env_plan.shadow_home_path.is_none() {
                    let (shadow_env, shadow_path, _) = super::repo_home::build_repo_home_env(
                        provider,
                        env_plan.child_cwd.as_path(),
                        false,
                        false,
                        Some(env_plan.child_cwd.as_path()),
                    )?;
                    for (key, value) in shadow_env {
                        env_plan.env.insert(key, value);
                    }
                    env_plan.shadow_home_path = shadow_path;
                }
                let shadow = env_plan.shadow_home_path.as_deref();
                let mut string_env = std::collections::HashMap::new();
                let result = injector
                    .inject(&session.servers, &mut string_env, shadow)
                    .map_err(|e| eyre!("MCP injection failed: {e}"))?;

                // The OpenCode inline config is shared with the system-prompt
                // and YOLO producers, so it must merge into any value already on
                // the plan rather than overwrite it; every other key is a plain
                // set. Mirrors the direct wrapper at wrapper_mcp.rs.
                super::wrapper_mcp::merge_injected_env_into_plan(string_env, &mut env_plan)?;
                mcp_extra_args.extend(result.extra_args);
            }
        } else {
            return Err(eyre!(
                "provider {} does not support runtime MCP injection.\n\
                 Use `claudine mcp export {} --apply` to write servers to its native config instead.",
                provider,
                provider.as_slug()
            ));
        }
        record_substage(&mut perf_collector, &mut last_checkpoint, "mcp composition");
    } else {
        if let Some(ref mut collector) = perf_collector {
            collector.mark_substage("mcp composition", std::time::Duration::ZERO);
        }
    }

    let mut child_args = Vec::new();

    // -- Yolo ----------------------------------------------------------------

    // `effective_yolo` is the single source of truth for whether the
    // provider's native bypass actually took effect on this launch.
    // Reporter / badge surfaces should read this — never `request.yolo`
    // (intent) on its own, since interactive OpenCode silently suppresses
    // the flag.
    let mut effective_yolo = false;
    if request.yolo {
        let mut env_overrides = Vec::new();
        let outcome = profile.apply_yolo_for_mode(
            &mut child_args,
            &mut env_overrides,
            !effective_non_interactive,
        )?;
        effective_yolo = outcome.applied;
        if let Some(warn) = outcome.warning
            && !silent
            && !quiet
        {
            log::warn(&warn);
        }
        for (key, value) in env_overrides {
            env_plan.env.insert(key.into(), value.into());
        }
    }
    // Override the YOLO env var in the child process with the
    // post-apply truth so the dispatch reporter (which stamps event
    // metadata from this var) reflects what actually landed, not what
    // was intended. `build_child_env_with_launch` set this from
    // `request.yolo` before we knew the outcome — replace it now.
    env_plan.env.insert(
        "YOLO".into(),
        if effective_yolo { "true" } else { "false" }.into(),
    );

    // Merge the OpenCode YOLO permission block into `OPENCODE_CONFIG_CONTENT`
    // as the last Claudine overlay, mirroring the direct wrapper's Stage 10.5.
    // Runs after the MCP fold above so it cannot be weakened by the MCP or
    // system-prompt writers; gated on YOLO having actually taken effect
    // (`effective_yolo`, not `request.yolo`) plus non-interactive.
    crate::commands::wrap::wrapper_stages::apply_opencode_yolo_config_overlay(
        provider,
        effective_yolo,
        effective_non_interactive,
        &mut env_plan,
    )?;
    // One-shot debug trace of the final argv that goes to the provider
    // so operators can verify the catalog flag actually landed without
    // running an external `ps`. Gated by `tracing` (RUST_LOG); never
    // unconditional output.
    tracing::debug!(
        target: "claudine::wrap::yolo",
        provider = %profile.provider(),
        request_yolo = request.yolo,
        effective_yolo,
        non_interactive = effective_non_interactive,
        child_args = ?child_args,
        "yolo applied to provider argv",
    );

    profile.apply_entrypoint(&mut child_args, effective_non_interactive);

    if effective_non_interactive {
        profile.apply_non_interactive_flags(&mut child_args)?;
    }

    // OpenCode model resolution (replaces apply_non_interactive_defaults +
    // validate_non_interactive_requirements).
    let _opencode_model_source: Option<super::profile::OpenCodeModelSource> =
        if provider == Provider::OpenCode {
            let has_model = env_plan
                .env
                .contains_key(&std::ffi::OsString::from("MODEL"));
            super::profile::apply_opencode_model_resolution(
                &mut child_args,
                &mut |k, v| {
                    env_plan.env.insert(k.into(), v.into());
                },
                has_model,
                target.model.as_deref(),
                effective_non_interactive,
                &super::profile::OpenCodeEnvSnapshot::from_system(),
            )?
        } else {
            None
        };

    // Universal --model flag (non-OpenCode providers, and OpenCode interactive).
    if provider != Provider::OpenCode {
        if let Some(ref model) = target.model {
            let mut env_overrides = Vec::new();
            if let Some(warn) = profile.apply_model(&mut child_args, &mut env_overrides, model)
                && !silent
                && !quiet
            {
                log::warn(&warn);
            }
            for (key, value) in env_overrides {
                env_plan.env.insert(key.into(), value.into());
            }
        }
    } else if let Some(ref model) = target.model
        && !effective_non_interactive
    {
        let mut env_overrides = Vec::new();
        if let Some(warn) = profile.apply_model(&mut child_args, &mut env_overrides, model)
            && !silent
            && !quiet
        {
            log::warn(&warn);
        }
        for (key, value) in env_overrides {
            env_plan.env.insert(key.into(), value.into());
        }
    }

    // Non-OpenCode providers still use the trait-based validation.
    if provider != Provider::OpenCode && effective_non_interactive {
        profile.validate_non_interactive_requirements(&child_args)?;
    }

    // Universal --output flag
    if let Some(ref output_str) = request.output {
        let format: super::profile::OutputFormat = (*output_str).into();
        if let Some(warn) = profile.apply_output_format(&mut child_args, format)
            && !silent
            && !quiet
        {
            log::warn(&warn);
        }
    }

    record_substage(&mut perf_collector, &mut last_checkpoint, "argv assembly");

    enforce_repo_launch_detection(request.repo, request.prep_launch_detection_error.as_deref())?;
    let mut launch_context = if let Some(prep) = request.prep_launch_context.as_ref() {
        // Phase fix (2026-05-09-slow-prep): reuse the launch_context computed
        // by the shared sniff scan in `CompositionPrepContext` instead of
        // re-running `sniff::detect_with_plan` here.
        prep.clone()
    } else {
        match claudine::system_prompt::LaunchContext::from_cwd(&launch_cwd) {
            Ok(context) => context,
            Err(error) => {
                if request.repo {
                    return Err(eyre!(
                        "--repo requires startup repo detection, but launch-context detection failed: {error}"
                    ));
                }
                if !silent && !quiet {
                    log::warn(&format!(
                        "launch-context detection failed; continuing without repo/package context: {error}"
                    ));
                }
                claudine::system_prompt::LaunchContext {
                    cwd: launch_cwd.clone(),
                    repo_root: None,
                    package_area_root: None,
                    package_root: None,
                    agent: None,
                }
            }
        }
    };
    // Plumb the provider slug into the launch context so system-prompt
    // templates that reference {{env.AGENT}} resolve correctly without
    // mutating the parent process env.
    launch_context.agent = Some(target.provider.as_slug().to_string());
    let effective_sp = claudine::system_prompt::resolve_and_prepare_for_session(
        &request.system_prompt_args,
        &launch_context,
        effective_non_interactive,
    )?;

    let scoped_tmp = super::system_prompt::scoped_tmp_dir(&launch_workspace);
    super::system_prompt::maybe_gitignore_claudine_tmp(
        launch_workspace
            .repo_root
            .as_deref()
            .unwrap_or(&launch_workspace.launch_cwd),
    );

    let mut sp_artifacts: Vec<super::system_prompt::SystemPromptArtifact> = Vec::new();

    match &effective_sp {
        claudine::system_prompt::ResolvedSystemPrompt::None
        | claudine::system_prompt::ResolvedSystemPrompt::Disabled { .. } => {}
        claudine::system_prompt::ResolvedSystemPrompt::Ready(prepared) => {
            let application = profile.apply_system_prompt(
                prepared,
                !effective_non_interactive,
                &launch_cwd,
                &scoped_tmp,
            )?;
            child_args.extend(application.args);
            for (k, v) in application.env {
                if k == "HOME" && env_plan.env.contains_key(std::ffi::OsStr::new("HOME")) {
                    continue;
                }
                // `OPENCODE_CONFIG_CONTENT` is the one inline config the
                // system-prompt writer shares with the MCP and YOLO producers.
                // A raw insert here clobbers the YOLO `permission` overlay
                // applied earlier (mod.rs:826), which silently re-opens the
                // subagent `external_directory` hang the YOLO overlay exists to
                // prevent. Route it through the same merge the MCP fold uses so
                // `instructions`, `mcp`, and `permission` coexist.
                if k == std::ffi::OsStr::new("OPENCODE_CONFIG_CONTENT") {
                    let injected = std::collections::HashMap::from([(
                        "OPENCODE_CONFIG_CONTENT".to_string(),
                        v.to_string_lossy().to_string(),
                    )]);
                    super::wrapper_mcp::merge_injected_env_into_plan(injected, &mut env_plan)?;
                    continue;
                }
                env_plan.env.insert(
                    k.to_string_lossy().to_string().into(),
                    v.to_string_lossy().to_string().into(),
                );
            }
            sp_artifacts = application.artifacts;
            for warn in application.warnings {
                if !silent && !quiet {
                    log::warn(&warn);
                }
            }
        }
    }
    let _ = &sp_artifacts;

    // Universal --sandbox flag
    if request.sandbox
        && let Some(warn) = profile.apply_sandbox(&mut child_args)
        && !silent
        && !quiet
    {
        log::warn(&warn);
    }

    record_substage(&mut perf_collector, &mut last_checkpoint, "system prompt");

    // Timeout/interactive conflict, evaluated against the RESOLVED session
    // mode and the RESOLVED timeout plan. Interactive sessions never honor a
    // wall-clock or step-silence deadline, so an explicitly requested timeout
    // from any source — CLI flag, composed frontmatter, or env var — conflicts
    // with a resolved-interactive session. The built-in 30m step_timeout
    // default is excluded (`built_in: None` below): it is always present and
    // is simply ignored in interactive mode, so it must not trip the conflict.
    // The early CLI-only guards in the command entry points stay as fast
    // syntax feedback; this is the authoritative check now that frontmatter
    // (`interactive: true`) can also select interactive mode.
    if request.session_interactive {
        let fm = &request.prepared.effective_frontmatter;
        let sp = request.prepared.resolved_path.as_path();
        let explicit_timeout = resolve_single_timeout(TimeoutResolutionInput {
            cli: request.timeout.clone(),
            frontmatter: frontmatter_timeout_duration(fm, "timeout", sp),
            env_var: "CLAUDINE_TIMEOUT",
            built_in: None,
        });
        let explicit_step_timeout = resolve_single_timeout(TimeoutResolutionInput {
            cli: request.step_timeout.clone(),
            frontmatter: frontmatter_timeout_duration(fm, "step_timeout", sp),
            env_var: "CLAUDINE_STEP_TIMEOUT",
            built_in: None,
        });
        if explicit_timeout.is_some() {
            return Err(eyre!(format_interactive_timeout_conflict(
                request.session_interactive_source,
                "--timeout",
            )));
        }
        if explicit_step_timeout.is_some() {
            return Err(eyre!(format_interactive_timeout_conflict(
                request.session_interactive_source,
                "--step-timeout",
            )));
        }
    }

    child_args.extend(mcp_extra_args);

    // -- Structured streaming decision ------------------------------------

    let stdout_noise = if effective_non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    // Interactive TUIs (Codex, OpenCode, etc.) must inherit stderr directly.
    // A non-empty stderr filter causes `exec::run_child` to pipe stderr,
    // which flips `isolate_process_group` on and leaves the child in a
    // background pgroup — it then hangs on SIGTTIN when reading the TTY.
    let stderr_noise = if effective_non_interactive {
        profile.stderr_noise_prefixes()
    } else {
        &[]
    };

    let use_structured = profile.supports_structured_stream() && effective_non_interactive;
    let stream_verbosity = structured_verbosity(silent, quiet);

    if use_structured {
        profile.apply_structured_stream(&mut child_args);
    }

    let structured_codex_output = if provider == Provider::Codex
        && (use_structured || (request.session_interactive && is_inline))
    {
        Some(StructuredCodexOutput::prepare(&mut child_args))
    } else {
        None
    };

    // Deliver the prompt after provider-specific flags have been assembled.
    // Some CLIs, notably OpenCode, treat the first positional argument as the
    // task body and may stop parsing subsequent flags. Appending the prompt too
    // early can silently disable structured-output flags, leaving Claudine
    // waiting forever for a stream the provider never enters.
    //
    // Snapshot args before prompt delivery so the harness loop gets a
    // prompt-free base (the harness manages prompt delivery itself).
    let args_before_prompt = child_args.clone();
    let prompt_source = super::profile::PromptSource::Inline(effective_prompt.clone());
    let delivery =
        profile.prompt_delivery(&child_args, &effective_prompt, effective_non_interactive)?;
    // The harness loop rebuilds prompt delivery from the materialized prompt,
    // so the returned wire/stdin seed values are not needed here. We still
    // apply the delivery to `child_args` so the argv validation below sees the
    // final provider argv.
    delivery.apply_to(&mut child_args);

    let effective_repo_root = source_repo_root.or(env_plan.repo_root.as_deref());
    let child_cwd = env_plan.child_cwd.as_path();

    super::profile::require_prompt_present(
        profile.binary(),
        effective_non_interactive,
        &prompt_source,
    )?;

    if tracing::enabled!(tracing::Level::WARN) {
        super::profile::validate_argv_flags_before_separator(profile.binary(), &child_args);
    }

    record_substage(
        &mut perf_collector,
        &mut last_checkpoint,
        "stream + prompt delivery",
    );

    if let Some(collector) = perf_collector.as_mut() {
        collector.mark_env_setup_complete();
    }

    // --dry-run no longer exits here. The seam now sits *after* the harness
    // shell-approval preflight block below, so shell-approval decisions
    // participate in the dry-run gate before the composed output is rendered.
    // See the `request.dry_run` early-return after preflight.

    switch_process_cwd(child_cwd)?;

    drop(_span);

    let _span = tracing::info_span!("composition_preflight").entered();

    if !silent && !quiet {
        use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
        use biscuit_terminal::prelude::TerminalRenderable as _;

        let status = Status::from_prose("Starting pre-flight checks".to_string())
            .state(StatusState::Info)
            .theme(StatusTheme::Circular);
        crate::log::message(&status.render(&term));
    }

    // -- Harness plan preflight -------------------------------------------
    // Every non-dry-run document is parsed into a harness plan. Documents
    // lacking harness frontmatter yield the bare (all-empty) plan; the loop
    // re-parses from the materialized frontmatter on retry attempts. The plan
    // now carries only timeout configuration; the removed pre/post validation
    // checks and handler recovery DSL are no longer evaluated.

    let shell_options = apply_composition_shell_overrides(
        build_harness_shell_options_with_cache(
            &request.prepared.resolved_path,
            effective_repo_root,
            request.shared_approval_cache.clone(),
        ),
        request.dry_run,
        request.yolo,
    );

    // --- Lifecycle notification setup ------------------------------------
    // Constructed up here (rather than just before the guard) so the
    // `run_body` closure below can capture these by reference and route
    // composition-preflight failures through the stack-aware event runner
    // (`emit_preflight_blocked_and_finalize`). Pre-flight failures must fire
    // `blocked.stack` + `finalize.stack`, which requires the same emitter /
    // settings / messaging / effect-engine the post-closure `initialize`
    // event uses — so the bindings are hoisted to a single shared
    // construction site.
    let lifecycle = &request.prepared.lifecycle;
    let emitter = DefaultLifecycleEmitter;

    // Skip runtime config loading when no lifecycle notifications are configured.
    let (lifecycle_settings, lifecycle_messaging) = if lifecycle.is_empty() {
        (
            claudine::events::GlobalSettings::default(),
            claudine::messaging::RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
        )
    } else {
        match claudine::dispatch::loader::load_claudine_config(None, effective_repo_root) {
            Ok(config) => (
                claudine::dispatch::loader::bridge_tts_settings(&config),
                claudine::dispatch::loader::bridge_messaging_settings(&config),
            ),
            Err(_) => (
                claudine::events::GlobalSettings::default(),
                claudine::messaging::RuntimeMessagingSettings {
                    user: None,
                    repo: None,
                },
            ),
        }
    };

    // Build an effect engine for lifecycle side effects / expression functions.
    // Writes are confined to the repo root when known, otherwise the launch cwd.
    let lifecycle_mutation_root = effective_repo_root.unwrap_or(launch_cwd.as_path());
    let lifecycle_effect_engine = EffectEngine::builder()
        .mutation_root(lifecycle_mutation_root)
        .auto_rehash(false)
        .build();

    // Capture the early-binding context ONCE for this run, against the launch
    // area (the package area the caller launched from), and reuse the same
    // snapshot for every lifecycle event (the pre-flight `blocked`/`finalize`
    // stacks and the post-closure `initialize`) so plain `ctx.*`/`env.*` cannot
    // diverge per event and no per-event sniff scan runs. Demand-driven over
    // the effective frontmatter (which still carries the deferred lifecycle
    // `{{ctx.*}}` strings) plus the composed body. `env_overrides` mirror what
    // `prepare` applied. `current.*` stays event-time and is captured below.
    let lifecycle_context = {
        let fm_json =
            serde_json::to_string(&request.prepared.effective_frontmatter).unwrap_or_default();
        let scan = format!("{fm_json}\n{}", request.prepared.prompt);
        let mut ctx = darkmatter::markdown::compose::ComposeContext::capture_for_content(
            launch_workspace.launch_cwd.as_path(),
            &scan,
        );
        for (key, value) in &request.env_overrides {
            ctx.env_mut().insert(key.clone(), value.clone());
        }
        ctx
    };

    // Common execution body used both when the caller provides an external
    // lifecycle guard (loop re-entry) and when this function owns the guard
    // (single-run / first loop iteration). The closure captures the prep
    // state by reference; it is only invoked while this stack frame lives.
    let run_body = |
        guard: &mut claudine::composition::LifecycleRunGuard<'_>,
        skip_preflight: bool,
        proxy_source: Option<&Path>,
    | -> Result<SingleCompositionOutcome> {
        // Composed frontmatter / source-derived base dir, reused by every
        // composition-preflight failure path so the blocked+finalize stacks
        // see the same `frontmatter` and `base_dir` namespaces the
        // post-closure `initialize` event does.
        let fm_map = request.prepared.effective_frontmatter.as_object();
        let empty_frontmatter = serde_json::Map::new();
        let frontmatter = fm_map.unwrap_or(&empty_frontmatter);
        let base_dir = request
            .prepared
            .resolved_path
            .parent()
            .or(effective_repo_root);
        // Validate that the harness plan can be parsed before proceeding.
        let plan = claudine::harness::parse_harness_plan(
            &request.prepared.effective_frontmatter,
            &request.prepared.resolved_path,
        )
        .map_err(|e| {
            // Route through the stack-aware runner so `blocked.stack` and
            // `finalize.stack` fire (spec.md:436/650/652), not just the
            // legacy top-level surface.
            let preflight_outcome = emit_preflight_blocked_and_finalize(
                guard,
                &lifecycle_effect_engine,
                &emitter,
                &lifecycle_settings,
                &lifecycle_messaging,
                &term,
                &request.prepared.resolved_path,
                effective_repo_root,
                base_dir,
                Some(launch_workspace.launch_cwd.as_path()),
                Some(&lifecycle_context),
                frontmatter,
                document_start,
                claudine::composition::LifecycleErrorInfo::from_action_failure(
                    "harness_plan",
                    e.to_string(),
                ),
            );
            match preflight_outcome {
                PreflightBlockedOutcome::EvaluationError(ce) => ce.into(),
                PreflightBlockedOutcome::Control(control) => {
                    match preflight_blocked_control_error(
                        control,
                        &request.prepared.resolved_path,
                    ) {
                        Some(ce) => ce.into(),
                        None => eyre!("{e}"),
                    }
                }
            }
        })?;

        // The parsed harness plan is used only for shell-command audit and
        // timeout configuration; there are no longer pre/post validation
        // checks that need an effective-plan transform.

        // ── Pre-flight shell approval for harness commands ───────────
        if !skip_preflight {
            let _harness_preflight = claudine::composition::resolve_shell_approvals(
                None, // template commands already approved during compose
                None,
                &shell_options,
                Some(&request.prepared.lifecycle),
                Some(&request.prepared.resolved_path),
            )
            .map_err(|e| {
                // Shell-audit denial (or any other shell-approval failure)
                // is a composition-preflight blocked path: route through
                // the stack-aware runner so `blocked.stack` and
                // `finalize.stack` fire.
                let preflight_outcome = emit_preflight_blocked_and_finalize(
                    guard,
                    &lifecycle_effect_engine,
                    &emitter,
                    &lifecycle_settings,
                    &lifecycle_messaging,
                    &term,
                    &request.prepared.resolved_path,
                    effective_repo_root,
                    base_dir,
                    Some(launch_workspace.launch_cwd.as_path()),
                    Some(&lifecycle_context),
                    frontmatter,
                    document_start,
                    claudine::composition::LifecycleErrorInfo::from_action_failure(
                        "shell_approval",
                        e.to_string(),
                    ),
                );
                match preflight_outcome {
                    PreflightBlockedOutcome::EvaluationError(ce) => ce.into(),
                    PreflightBlockedOutcome::Control(control) => {
                        match preflight_blocked_control_error(
                            control,
                            &request.prepared.resolved_path,
                        ) {
                            Some(ce) => ce.into(),
                            None => eyre!("{e}"),
                        }
                    }
                }
            })?;

            // Emit the preflight-complete indicator for direct compose and
            // inline-compose runs. This must sit *before* the dry-run seam below:
            // dry-run returns early, so a completion message placed after it would
            // never render for dry-run — leaving the "Starting pre-flight checks"
            // spinner without its matching "complete" line. Sequence runs handle
            // their own preflight messaging in the orchestrator
            // (`wrap::sequence::execute_sequence`) and must not re-emit per step.
            if !request.sequence && !silent && !quiet {
                let compose_label = if is_inline {
                    "inline composition"
                } else {
                    "composition"
                };
                let status = Status::from_prose(format!(
                    "<b>Preflight:</b> shell commands approved for this {compose_label}"
                ))
                .state(StatusState::Info);
                log::message(&status.render(&term));
            }
        }

        // --dry-run seam: the full composition pipeline (compose, real shell
        // expansion, shell approval, harness pre-checks) has now run. Stop here —
        // before any provider launches — and emit the composed artifacts:
        //   - the composed body → stdout (the data product; pipeable/redirectable)
        //   - the finalized frontmatter (highlighted YAML) → stderr
        //   - a metadata table → stderr (after the frontmatter)
        // `--quiet` / `--silent` do not suppress this render: the dry-run output
        // *is* the command's purpose.
        if request.dry_run {
            // Dry-run never launches the provider or mutates the source.
            // Pre-check validation has been removed; only timeout parsing
            // and shell-command audit run during composition preflight.
            let render = dry_run::DryRunRender::from_request(&request);

            crate::log::data(&render.body);
            crate::log::message(&dry_run::render_hr(&term));
            crate::log::message(&dry_run::render_frontmatter_heading(&term));
            crate::log::message("");
            crate::log::message(&dry_run::render_frontmatter(&render.frontmatter, &term));
            crate::log::message(&dry_run::render_metadata_table(&render, &term));

            if let Some(collector) = perf_collector.as_mut() {
                collector.set_dry_run();
            }
            let outcome = SingleCompositionOutcome {
                exit_code: 0,
                provider,
                agent_perf: None,
                // Dry-run never produces a per-iteration summary.
                iteration_signals: None,
                terminal_signal: None,
            };
            // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
            // The perf report is always emitted to stderr when requested.
            if let Some(collector) = perf_collector {
                crate::perf::emit_report(&collector.into_report());
            }
            return Ok(outcome);
        }

        // Plan is validated; the harness loop re-parses from the materialized
        // frontmatter, so the live path no longer needs this copy.
        drop(plan);

        // -- Preflight output (env details + prompt block) ---------------------
        // The execution header was already emitted (up front by compose /
        // inline-compose, or above for callers that did not pre-render). Now
        // emit the env details and prompt block with the full env_plan.

        // Detect the environment from the source repo root when available so
        // that git/repo metadata reflects the composition source, not the
        // caller's CWD (which may be in a different repo entirely).
        //
        // Phase 4 (2026-05-09-slow-prep): `detect_environment_fast` is still on
        // the critical path after Phases 1–2, but its direct cost is minimal
        // (~8 ms for git summary + repo structure). The `compose_prep.environment`
        // span added in Phase 3 makes this cost visible in traces. Making the
        // context truly lazy would require invasive changes to LiveSemanticSink,
        // DispatchRuntimeContext, and the wire-session path because the context is
        // consumed synchronously before the child spawns. Per the spec, when lazy
        // creation is too invasive we instrument and defer deeper work.
        let env_detect_root = effective_repo_root.unwrap_or(&launch_cwd);
        let env_context = {
            let _span = tracing::info_span!("compose_prep.environment").entered();
            // Phase fix (2026-05-09-slow-prep): reuse the cached
            // `EnvironmentContext` when the prep-time sniff already covers the
            // requested env_detect_root. The cached scan was rooted at the
            // launch CWD, but sniff walks up to find the enclosing git/repo
            // root, so the resulting env_context is equivalent to one rooted
            // at `env_detect_root` whenever:
            //   1. env_detect_root == launch_cwd (trivial), OR
            //   2. launch_cwd is a subdirectory of env_detect_root AND the
            //      cached env_context's git repo_root or repo root matches
            //      env_detect_root (the common monorepo-subdir case).
            // When neither holds (e.g. `--repo` pins a different root or the
            // source lives in an unrelated repo), fall back to a fresh scan.
            let cached_matches = request.prep_env_context.as_ref().is_some_and(|prep| {
                if env_detect_root == launch_cwd.as_path() {
                    return true;
                }
                if !launch_cwd.starts_with(env_detect_root) {
                    return false;
                }
                let git_root_match = prep
                    .git
                    .as_ref()
                    .map(|g| g.repo_root.as_path() == env_detect_root)
                    .unwrap_or(false);
                let repo_root_match = prep
                    .repo
                    .as_ref()
                    .map(|r| r.root.as_path() == env_detect_root)
                    .unwrap_or(false);
                git_root_match || repo_root_match
            });
            if cached_matches {
                request
                    .prep_env_context
                    .as_ref()
                    .expect("cached_matches implies Some")
                    .clone()
            } else {
                claudine::events::detect_environment_fast(env_detect_root)
            }
        };

        if !silent {
            if !quiet && (request.session_interactive || detail_requested) {
                crate::output::log_wrapper_env_details(&env_plan, None, &term, verbose);
            }

            let scope_for_report = effective_repo_root.unwrap_or(&launch_cwd);
            crate::output::log_system_prompt_with_scope(
                &effective_sp,
                detail_requested,
                silent,
                quiet,
                Some(scope_for_report),
                &term,
            );

            if matches!(
                effective_sp,
                claudine::system_prompt::ResolvedSystemPrompt::Ready(_)
            ) && effective_non_interactive
            {
                crate::log::message("");
            }

            if effective_non_interactive {
                crate::output::log_compose_prompt(
                    &request.prepared.prompt,
                    detail_requested,
                    silent,
                    quiet,
                    &term,
                );
            }

            if !quiet {
                crate::log::message("");
            }
        }

        drop(_span);

        let _span = tracing::info_span!("composition_execute").entered();

        // -- Execution --------------------------------------------------------

        let dispatch_context = composition_dispatch_context(&request, &target);

        let harness_mode = if is_inline {
            HarnessPromptMode::Inline
        } else {
            HarnessPromptMode::Compose
        };

        // When an `initialize` Proxy redirected to a different document, the
        // harness loop re-materializes (re-composes frontmatter + body) from
        // `source_path` each attempt, so swapping the path here runs the
        // target document — its body, frontmatter, harness pre-checks, and
        // its `start`/`success`/`failure`/`finalize` lifecycle. Seed
        // `initial_materialized = None` so the loop composes the target rather
        // than reusing the proxying document's prepared prompt.
        let (effective_source, effective_ref, seed_materialized) = match proxy_source {
            Some(target) => (
                target.to_path_buf(),
                target.display().to_string(),
                None,
            ),
            None => (
                request.prepared.resolved_path.clone(),
                request.file_ref.clone(),
                Some(materialized_harness_prompt_from_prepared(&request.prepared)),
            ),
        };

        let mut prompt_state = HarnessPromptState {
            mode: harness_mode,
            source_path: effective_source,
            original_ref: effective_ref,
            base_prompt: None,
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
        };

        let mut harness_base_args = args_before_prompt.clone();
        if !use_structured {
            profile.prepare_captured_output(&mut harness_base_args);
        }

        let (exit_code, harness_perf, harness_signals) = run_harness_loop(
            provider,
            profile,
            binary_path.as_path(),
            child_cwd,
            effective_non_interactive,
            request.timeout.clone(),
            request.step_timeout.clone(),
            &harness_base_args,
            &env_plan.env,
            &mut prompt_state,
            effective_repo_root,
            shell_options.clone(),
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            show_checks,
            stream_verbosity,
            detail_requested,
            &env_context,
            &dispatch_context,
            seed_materialized,
            &term,
            guard,
            proxy_source,
            true,
        )?;
        if let (Some(collector), Some(perf)) = (perf_collector.as_mut(), harness_perf) {
            collector.set_agent_perf(perf);
        }
        let terminal_signal = guard.terminal_signal();
        let outcome = SingleCompositionOutcome {
            exit_code,
            provider,
            agent_perf: perf_collector
                .as_ref()
                .and_then(|c| c.agent_perf())
                .or(harness_perf),
            // The harness loop now surfaces the terminal attempt's iteration
            // signals, so `compose --loop` receives the same rate-limit /
            // exit_reason pickup for every composition document.
            iteration_signals: harness_signals,
            terminal_signal,
        };
        // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
        // The perf report is always emitted to stderr when requested.
        if let Some(collector) = perf_collector {
            crate::perf::emit_report(&collector.into_report());
        }
        Ok(outcome)
    };

    // When the caller passes an external guard (loop re-entry), it has already
    // emitted `initialize` and owns the lifecycle runtime context. Run only
    // the per-iteration body and return.
    if let Some(guard) = external_guard {
        return run_body(guard, skip_preflight, None);
    }

    // Bundle the shared lifecycle bindings (constructed above the closure)
    // into the runtime context the guard drives.
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &lifecycle_settings,
        messaging: &lifecycle_messaging,
        term: &term,
        source_path: &request.prepared.resolved_path,
        repo_root: effective_repo_root,
        launch_area: Some(launch_workspace.launch_cwd.as_path()),
        context: Some(&lifecycle_context),
    };

    // --- Initialize lifecycle event --------------------------------------
    // Fires after prompt/frontmatter resolution and CLI/frontmatter override
    // merge, but before $schema validation and shell pre-flight.
    let mut guard = LifecycleRunGuard::new(lifecycle, &lifecycle_ctx, &emitter);
    let fm_map = request.prepared.effective_frontmatter.as_object();
    let empty_frontmatter = serde_json::Map::new();
    let base_dir = request
        .prepared
        .resolved_path
        .parent()
        .or(effective_repo_root);
    // Lifecycle stack-only globals for the `initialize` event (and its
    // `with_signal`/`with_error` derivations, which copy these references).
    // `current.env`/`current.ctx` are captured now, so a side effect or
    // external change since `prepare` is observable through `current.*`. The
    // document-start instant anchors `timing.document_ms` at this event.
    // `current.ctx.*` follows the launch area like event-time `ctx.*` capture.
    let lifecycle_current =
        claudine::composition::lifecycle_context::LifecycleCurrent::capture_at_event(
            launch_workspace.launch_cwd.as_path(),
        );
    let lifecycle_timing = claudine::composition::lifecycle_context::LifecycleTiming::from_instants(
        document_start,
        None,
        std::time::Instant::now(),
    );
    let init_ctx = StackExecutionContext {
        signal: LifecycleSignal::Initialize,
        frontmatter: fm_map.unwrap_or(&empty_frontmatter),
        // Single pre-launch `initialize` event; the cross-event live cell is
        // owned by the harness loop, which re-materializes frontmatter before
        // its own events fire.
        live_frontmatter: None,
        err: None,
        timing: Some(&lifecycle_timing),
        current: Some(&lifecycle_current),
        base_dir,
        ctx_base_dir: Some(launch_workspace.launch_cwd.as_path()),
        prepared_context: Some(&lifecycle_context),
        effect_engine: &lifecycle_effect_engine,
        shell_runner: &SystemShellRunner,
        emitter: &emitter,
        term: &term,
        source_path: &request.prepared.resolved_path,
        repo_root: effective_repo_root,
        messaging: &lifecycle_messaging,
        settings: &lifecycle_settings,
    };
    let init_outcome = guard.execute_event(LifecycleSignal::Initialize, &init_ctx);
    // A late-binding evaluation error on `initialize` (a crashed `when:` guard
    // or interpolation) routes through `failure` → `finalize` like any other
    // setup failure and halts non-zero (Decision #5). Checked before the
    // control match because an evaluation raise leaves `control` `None`.
    if let Some(info) = init_outcome.evaluation_error.as_ref() {
        // Surface the original `initialize` crash to stderr before the
        // `failure`/`finalize` catch events fire (Decision #2).
        let early = crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            &request.prepared.resolved_path,
            "initialize",
            info,
            &term,
        );
        let failure_outcome =
            guard.execute_event(LifecycleSignal::Failure, &init_ctx.with_error(info));
        // If `failure` raised, thread its error (not the original) into
        // finalize so a `finalize.stack` can branch on the failure raise.
        let active_err = failure_outcome
            .evaluation_error
            .as_ref()
            .unwrap_or(info);
        let finalize_outcome = guard.execute_event(
            LifecycleSignal::Finalize,
            &init_ctx.with_error(active_err).with_signal(LifecycleSignal::Finalize),
        );
        return Err(surface_preflight_catch_error(
            &request.prepared.resolved_path,
            Some(&failure_outcome),
            Some(&finalize_outcome),
            early,
            &term,
        )
        .into());
    }
    // Set by an `initialize` Proxy control: the resolved target document the
    // run is handed off to. Threaded into `run_body` so the harness loop
    // re-composes and runs the target instead of the original document.
    let mut init_proxy_target: Option<std::path::PathBuf> = None;
    if let Some(ref control) = init_outcome.control {
        match control {
            StackControl::Skip => {
                // Clean whole-document opt-out: no pre-flight, no provider,
                // no finalize, no loop gate. Sequence orchestrator treats this
                // as a successful step and advances.
                return Ok(SingleCompositionOutcome {
                    exit_code: 0,
                    provider,
                    agent_perf: None,
                    iteration_signals: None,
                    terminal_signal: None,
                });
            }
            StackControl::Error { reason } => {
                let msg = reason
                    .clone()
                    .unwrap_or_else(|| "lifecycle initialize error".to_string());
                let action_error =
                    claudine::composition::lifecycle_context::LifecycleErrorInfo::from_action_failure(
                        "error",
                        msg.clone(),
                    );
                let failure_outcome = guard.execute_event(
                    LifecycleSignal::Failure,
                    &init_ctx.with_error(&action_error),
                );
                // If `failure` raised, thread its error (not the original) into
                // finalize so a `finalize.stack` can branch on the failure raise.
                let active_err = failure_outcome
                    .evaluation_error
                    .as_ref()
                    .unwrap_or(&action_error);
                let finalize_outcome = guard.execute_event(
                    LifecycleSignal::Finalize,
                    &init_ctx.with_error(active_err).with_signal(LifecycleSignal::Finalize),
                );
                if failure_outcome.evaluation_error.is_some()
                    || finalize_outcome.evaluation_error.is_some()
                {
                    // A catch event raised an evaluation error (the original was
                    // an explicit `error(...)`, not an evaluation error, so it
                    // was not early-emitted). Surface the catch raise to stderr;
                    // no further lifecycle events fire (Decision #2). The `early`
                    // fallback is unreachable inside this guard.
                    let early = CompositionError::catch_evaluation_error(
                        &request.prepared.resolved_path,
                        "initialize",
                        &action_error,
                        Some(&failure_outcome),
                        Some(&finalize_outcome),
                    )
                    .already_emitted();
                    return Err(surface_preflight_catch_error(
                        &request.prepared.resolved_path,
                        Some(&failure_outcome),
                        Some(&finalize_outcome),
                        early,
                        &term,
                    )
                    .into());
                }
                return Err(eyre!(msg));
            }
            StackControl::Proxy { target } => {
                // Hand off to the target document. Resolve the reference
                // (`@repo/…`, relative, or absolute) against the source so
                // `run_body` runs the target via the harness loop's
                // re-materialize path. The harness loop resets the lifecycle
                // guard and re-emits the target's own `initialize` before its
                // pre-flight / start / terminal / finalize lifecycle runs.
                let resolved = claudine::composition::resolve_proxy_target(
                    target,
                    &request.prepared.resolved_path,
                    effective_repo_root,
                )
                .map_err(|e| eyre!("lifecycle initialize proxy: {e}"))?;
                if !claudine::composition::proxy_handoff_allowed(
                    std::slice::from_ref(&request.prepared.resolved_path),
                    &resolved,
                ) {
                    return Err(CompositionError::LifecycleProxyCycle {
                        source_path: request.prepared.resolved_path.clone(),
                        target: target.clone(),
                        chain: vec![request.prepared.resolved_path.display().to_string()],
                        limit: claudine::composition::MAX_PROXY_HOPS,
                    }
                    .into());
                }
                init_proxy_target = Some(resolved);
            }
            StackControl::Stop => {}
            StackControl::Resume { .. } => {
                // Pre-launch: there is no provider session to resume.
                return Err(CompositionError::LifecycleResumeWithoutSession {
                    source_path: request.prepared.resolved_path.clone(),
                }
                .into());
            }
            StackControl::Retry { .. } => {
                // `retry` (re-run pre-flight) needs run-loop re-entry that does
                // not exist in the setup phase.
                return Err(CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                    source_path: request.prepared.resolved_path.clone(),
                    event: "initialize".to_string(),
                    action: "retry".to_string(),
                }
                .into());
            }
            StackControl::Defer { .. } => {
                // `defer` has no runtime home yet (rendezvous scheduler pending).
                return Err(CompositionError::LifecycleDeferNotImplemented {
                    source_path: request.prepared.resolved_path.clone(),
                }
                .into());
            }
        }
    }
    if init_outcome.routes_to_failure(LifecycleSignal::Initialize) {
        let original_info = init_outcome.action_error.as_ref();
        let failure_ctx = match original_info {
            Some(e) => init_ctx.with_error(e),
            None => init_ctx.with_signal(LifecycleSignal::Failure),
        };
        let failure_outcome = guard.execute_event(LifecycleSignal::Failure, &failure_ctx);
        // If `failure` raised, thread its error (not the original) into finalize
        // so a `finalize.stack` can branch on the failure raise. If there was no
        // original action error, synthesize one for finalize to carry.
        let synthetic_info =
            claudine::composition::lifecycle_context::LifecycleErrorInfo::from_action_failure(
                "error",
                "lifecycle initialize failed",
            );
        let active_err = failure_outcome
            .evaluation_error
            .as_ref()
            .or(original_info)
            .unwrap_or(&synthetic_info);
        let finalize_ctx = init_ctx.with_signal(LifecycleSignal::Finalize);
        let finalize_ctx = finalize_ctx.with_error(active_err);
        let finalize_outcome = guard.execute_event(LifecycleSignal::Finalize, &finalize_ctx);
        if failure_outcome.evaluation_error.is_some()
            || finalize_outcome.evaluation_error.is_some()
        {
            // A catch event raised an evaluation error while routing this
            // setup failure. Surface it to stderr; no further lifecycle events
            // fire (Decision #2). The `early` fallback is unreachable here.
            let info = original_info.unwrap_or(&synthetic_info);
            let early = CompositionError::catch_evaluation_error(
                &request.prepared.resolved_path,
                "initialize",
                info,
                Some(&failure_outcome),
                Some(&finalize_outcome),
            )
            .already_emitted();
            return Err(surface_preflight_catch_error(
                &request.prepared.resolved_path,
                Some(&failure_outcome),
                Some(&finalize_outcome),
                early,
                &term,
            )
            .into());
        }
        return Err(eyre!("lifecycle initialize failed"));
    }

    run_body(&mut guard, false, init_proxy_target.as_deref())
}

// -- Config loading -------------------------------------------------------

pub(crate) struct SelectionConfig {
    pub favorite: Option<Provider>,
    #[allow(dead_code)]
    pub model_overrides: HashMap<Provider, ProviderModelOverride>,
}

pub(crate) fn load_selection_config(cwd: &Path) -> Option<SelectionConfig> {
    let repo_root = sniff::filesystem::git::detect_git(cwd, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root);
    load_selection_config_for_repo(repo_root.as_deref())
}

/// Load selection config against a pre-detected repo root.
///
/// Skips the internal `sniff::filesystem::git::detect_git` call that
/// [`load_selection_config`] performs, so callers that already discovered
/// the source repo root (typically via [`CompositionPrepContext`]) avoid a
/// redundant filesystem walk on the compose hot path.
pub(crate) fn load_selection_config_for_repo(repo_root: Option<&Path>) -> Option<SelectionConfig> {
    let config = claudine::dispatch::loader::load_claudine_config(None, repo_root).ok()?;
    Some(SelectionConfig {
        favorite: config.preferred_agent,
        model_overrides: config.models,
    })
}

#[cfg(test)]
mod tests;
