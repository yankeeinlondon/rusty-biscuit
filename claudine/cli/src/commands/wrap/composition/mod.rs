//! Wrapper-grade composition executor.
//!
//! [`execute_composition_request`] is the single execution pipeline for
//! both `claudine compose` and `claudine inline-compose`. It provides
//! full wrapper-grade behavior: environment setup, harness detection from
//! effective (composed) frontmatter, structured streaming, and inline
//! closure.

use std::io::IsTerminal;
use std::path::Path;

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
    CompositionMode, InlineClosurePlan, IterationSummarySignals, ModelResolutionReason,
    ResolvedExecutionTarget, SelectionReason, SessionInteractivitySource, agent_state_breakdown,
    build_installed_snapshot, build_picker_plan, classify_agent_resolution,
    invalid_agent_message, resolve_target_non_tty_with_catalog,
};
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use darkmatter::effects::EffectEngine;
use inquire::Select;
use sniff::programs::InstalledAiClients;

use super::env;
use super::profile::{self, WrapperProfile};
use super::{
    HarnessPromptMode, HarnessPromptState, apply_composition_shell_overrides,
    build_harness_shell_options_with_cache, materialized_harness_prompt_from_prepared,
    resolve_binary_path_direct, run_harness_loop, structured_verbosity, wrap_terminal,
};
use super::exec::switch_process_cwd;
use crate::log;

/// Remediation for an unresolvable lifecycle `proxy(...)` target.
///
/// Names the shared `FileReference` grammar `resolve_harness_path` delegates
/// to, so the hint stays accurate to the contract rather than to one failure's
/// text.
pub(crate) const PROXY_TARGET_HINT: &str =
    "A `proxy` target must name an existing Markdown document: an absolute path, \
     an explicit `./` or `../` path relative to the document that declares it, a \
     bare path (tried at the repository root first, then next to the document), \
     `@`-prefixed for a magic-root search, or `~`-prefixed for a home path.";

pub(crate) mod dry_run;
pub(crate) mod launch;
mod preflight;
mod pipeline;
pub(crate) mod prep_context;
mod provider_args;
pub(crate) mod runner;
pub(crate) mod selection;
pub(crate) mod target;
pub(crate) mod timeouts;

pub(crate) use prep_context::CompositionPrepContext;
pub(crate) use selection::{
    SelectionConfig, load_selection_config, load_selection_config_for_repo,
};
pub(crate) use target::{
    agent_prompt_message, composition_dispatch_context, eagerly_resolve_target,
    install_agent_env_for_composition, refresh_for_model_validation, resolve_execution_target,
    scoped_picker_plan_for_state,
};
pub(crate) use timeouts::{
    TimeoutResolutionInput, build_prompt_timing_context, format_interactive_timeout_conflict,
    frontmatter_timeout_duration, resolve_single_timeout, resolve_stall_timeout, resolve_timeouts,
};
#[cfg(test)]
pub(crate) use target::{picker_scope_for_state, resolve_live_target_with_tty};

#[cfg(test)]
pub(crate) use launch::{
    launch_workspace_fallback_count_for_tests, reset_launch_workspace_fallbacks_for_tests,
};
pub(crate) use launch::select_launch_workspace;
use launch::enforce_repo_launch_detection;
use preflight::{
    PreflightBlockedOutcome, emit_preflight_blocked_and_finalize, preflight_blocked_control_error,
    setup_phase_deferred,
};

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
    pipeline::execute_composition_request_inner_with_guard(
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
    pipeline::execute_composition_request_inner_with_guard(
        request,
        verbose,
        None,
        perf_enabled,
        Some(guard),
        skip_preflight,
    )
}

#[cfg(test)]
mod tests;
