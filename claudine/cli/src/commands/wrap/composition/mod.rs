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
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::{
    DefaultLifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal,
};
use claudine::composition::{
    CompositionClosurePlan, CompositionError, CompositionExecutionRequest, CompositionMode,
    InlineClosurePlan, ModelResolutionReason, ResolvedExecutionTarget, SelectionReason,
    build_installed_snapshot, build_picker_plan, resolve_target_non_tty_with_catalog,
};
use claudine::config::claudine_config::ProviderModelOverride;
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use inquire::Select;
use sniff::programs::InstalledAiClients;

use super::env;
use super::exec;
use super::live_semantic_sink::LiveSemanticSink;
use super::profile::{self, WrapperProfile};
use super::{
    HarnessPromptMode, HarnessPromptState, StreamSummaryContext, StructuredCodexOutput,
    StructuredSummaryDetails, WrapperHarnessPermissionProbe,
    build_harness_shell_options_with_cache, emit_stream_summary_with_context, format_summary_prose,
    format_verbose_summary_details_prose, materialized_harness_prompt_from_prepared,
    resolve_binary_path_direct, run_harness_loop, structured_verbosity, switch_process_cwd,
    wrap_terminal,
};
use crate::log;

pub(crate) mod inline_guards;
pub(crate) mod legacy_goose;
pub(crate) mod prep_context;
pub(crate) mod structured;
pub(crate) mod summary;

// Re-export the public API so existing callers don't break.
pub(crate) use inline_guards::{cleanup_inline_output, split_frontmatter_and_body};
pub(crate) use prep_context::CompositionPrepContext;
pub(crate) use structured::run_structured_composition;
pub(crate) use summary::{emit_composition_summary, emit_minimal_composition_summary};

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
    /// Populated for the non-harness structured-stream path (the only
    /// path that can carry a rate-limit trailer or a watchdog
    /// `error_kind`). `None` for the dry-run, harness, and legacy paths
    /// where these signals aren't available at this layer.
    pub iteration_signals: Option<IterationSummarySignals>,
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

/// Result of running a structured composition stream.
///
/// Produced by [`run_structured_composition`] and consumed by both the
/// compose and inline-compose callers. The shared function does not emit
/// the summary; callers decide the timing and routing.
pub(crate) struct CompositionStreamResult {
    exit_code: i32,
    assistant_text: String,
    summary: claudine::stream::summary::StreamExecutionSummary,
    details: StructuredSummaryDetails,
    had_streamed_assistant: bool,
    /// Shares a `SectionTracker` with the live sink so post-stream trailer
    /// emitters see consistent section state. Only the compose caller uses
    /// this; inline-compose ignores it.
    section_stream: super::section::SectionStream,
    /// Child-process telemetry for perf reporting.
    telemetry: exec::ProcessTelemetry,
}

/// Mode-specific inputs for [`execute_without_harness`].
///
/// Carries the inline-only parameters (closure plan, target path,
/// interactivity, stderr verbosity) so the merged function can branch its
/// post-execution logic without dragging optional parameters through every
/// call.
#[derive(Clone, Copy)]
pub(crate) enum CompositionExecutionMode<'a> {
    Direct,
    Inline {
        closure_plan: &'a InlineClosurePlan,
        resolved_path: &'a std::path::Path,
        session_interactive: bool,
        show_checks: bool,
    },
}

/// Build a [`PromptTimingContext`] from a resolved prompt path, the
/// effective repo root (when any), and the optional warn thresholds
/// parsed from harness frontmatter.
///
/// `display_path` is resolved in the order repo root → CWD → `$HOME`
/// (falling back to the absolute path when none apply) per the feature
/// spec's "relative path" rules for the OSC8 link text.
pub(crate) fn build_prompt_timing_context(
    absolute_path: &std::path::Path,
    repo_root: Option<&std::path::Path>,
    timeout_warn: Option<std::time::Duration>,
    step_timeout_warn: Option<std::time::Duration>,
) -> claudine::stream::prompt_timing::PromptTimingContext {
    let display_path = resolve_prompt_display_path(absolute_path, repo_root);
    claudine::stream::prompt_timing::PromptTimingContext {
        absolute_path: absolute_path.to_path_buf(),
        display_path,
        timeout_warn,
        step_timeout_warn,
    }
}

/// Source-precedence input for [`resolve_timeouts`].
///
/// `cli` is the value passed via `--timeout` / `--step-timeout`.
/// `frontmatter` is the value parsed from `HarnessPlan.timeout` /
/// `HarnessPlan.step_timeout`.
/// `env_var` is the env-var name to consult as the third-priority source.
/// `built_in` is the final fallback (e.g. `Some(30m)` for `step_timeout`,
/// `None` for `timeout`).
pub(crate) struct TimeoutResolutionInput<'a> {
    pub cli: Option<String>,
    pub frontmatter: Option<std::time::Duration>,
    pub env_var: &'a str,
    pub built_in: Option<std::time::Duration>,
}

/// Resolve a single timeout following the documented precedence chain:
///
///   CLI flag > frontmatter > env-var default > built-in default.
///
/// Env values use the same `parse_timeout` grammar as frontmatter
/// (`30s`, `5m`, `2h`). An env value of `0s` (or any zero duration via the
/// grammar) **disables** the rule for this run, returning `None` even if a
/// non-zero built-in default exists. Invalid env values are silently
/// ignored and the chain falls through to the next layer.
pub(crate) fn resolve_single_timeout(
    input: TimeoutResolutionInput<'_>,
) -> Option<std::time::Duration> {
    if let Some(raw) = input.cli {
        match claudine::harness::parse_timeout(&raw, std::path::Path::new("<cli>")) {
            Ok(d) => return Some(d),
            Err(_) => {
                // Invalid CLI value should have been caught earlier, but
                // fall through rather than panicking.
            }
        }
    }
    if let Some(d) = input.frontmatter {
        return Some(d);
    }
    match std::env::var(input.env_var) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                input.built_in
            } else if is_zero_duration_literal(trimmed) {
                // Spec: env value of `0s` disables the rule (parse_timeout
                // itself rejects zero, so we recognise the literal here).
                None
            } else {
                match claudine::harness::parse_timeout(trimmed, std::path::Path::new("<env>")) {
                    Ok(d) => Some(d),
                    Err(_) => input.built_in,
                }
            }
        }
        Err(_) => input.built_in,
    }
}

/// Recognise env-var literals that the user means as "disable this rule".
///
/// Accepts plain `0`, `0s`, `0 seconds`, `0m`, `0h`, etc. — anything whose
/// numeric component is `0` regardless of unit. Case-insensitive on the unit.
fn is_zero_duration_literal(value: &str) -> bool {
    let trimmed = value.trim();
    let digits_end = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    if digits_end == 0 {
        return false;
    }
    let (digits, _rest) = trimmed.split_at(digits_end);
    digits.parse::<u64>().is_ok_and(|n| n == 0)
}

/// Resolve `timeout` and `step_timeout` simultaneously and assemble a
/// [`TimeoutConfig`] for the watchdog ticker.
///
/// CLI > frontmatter > env > built-in. Built-ins are `None` for `timeout`
/// (no wall-clock kill unless opted in) and `30m` for `step_timeout`.
/// Supporting knobs (`kill_grace`, `interval`) are read from env via
/// [`super::subagent_watchdog::TimeoutConfig::resolve`].
pub(crate) fn resolve_timeouts(
    cli_timeout: Option<String>,
    plan_timeout: Option<std::time::Duration>,
    cli_step_timeout: Option<String>,
    plan_step_timeout: Option<std::time::Duration>,
) -> super::subagent_watchdog::TimeoutConfig {
    let timeout = resolve_single_timeout(TimeoutResolutionInput {
        cli: cli_timeout,
        frontmatter: plan_timeout,
        env_var: "CLAUDINE_TIMEOUT",
        built_in: None,
    });
    let step_timeout = resolve_single_timeout(TimeoutResolutionInput {
        cli: cli_step_timeout,
        frontmatter: plan_step_timeout,
        env_var: "CLAUDINE_STEP_TIMEOUT",
        built_in: Some(std::time::Duration::from_secs(30 * 60)),
    });
    super::subagent_watchdog::TimeoutConfig::resolve(timeout, step_timeout)
}

pub(crate) fn resolve_prompt_display_path(
    path: &std::path::Path,
    repo_root: Option<&std::path::Path>,
) -> String {
    if let Some(root) = repo_root
        && let Ok(rel) = path.strip_prefix(root)
    {
        return rel.display().to_string();
    }
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(rel) = path.strip_prefix(&cwd)
    {
        return rel.display().to_string();
    }
    if let Some(home) = dirs::home_dir()
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", rel.display());
    }
    path.display().to_string()
}

fn composition_dispatch_context(
    request: &CompositionExecutionRequest,
    target: &ResolvedExecutionTarget,
) -> HashMap<String, serde_json::Value> {
    let mut context = HashMap::new();
    context.insert(
        "composition_file_ref".into(),
        serde_json::Value::String(request.file_ref.clone()),
    );
    context.insert(
        "composition_mode".into(),
        serde_json::Value::String(match request.mode {
            CompositionMode::InlineFrontmatterPrompt => "inline".to_string(),
            CompositionMode::ChainedDocument => "compose".to_string(),
        }),
    );
    context.insert(
        "composition_source_path".into(),
        serde_json::Value::String(request.prepared.resolved_path.display().to_string()),
    );
    context.insert(
        "provider_selection_reason".into(),
        serde_json::Value::String(format!("{:?}", target.provider_reason)),
    );
    context.insert(
        "resolved_model".into(),
        serde_json::Value::String(target.model.clone().unwrap_or_default()),
    );
    context.insert(
        "model_selection_reason".into(),
        serde_json::Value::String(format!("{:?}", target.model_reason)),
    );
    context.insert(
        "selection_mode".into(),
        serde_json::Value::String(
            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                "tty"
            } else {
                "non-tty"
            }
            .to_string(),
        ),
    );
    context
}

/// Resolve the execution target *before* composition templates are
/// rendered, so `{{env.AGENT}}` in the body or inline `prompt` resolves
/// to the chosen provider.
///
/// Mirrors the resolution logic in [`execute_composition_request_inner`]
/// — explicit flag wins, then frontmatter agent hint, then favorite, with
/// a TTY picker when no signal yields a unique answer. The hints come
/// from raw frontmatter (no compose), so an `agent: "{{...}}"` template
/// is treated as absent and falls back to the picker / favorite.
pub(crate) fn eagerly_resolve_target(
    ctx: &CompositionPrepContext,
    hints: &claudine::composition::EffectiveSelectionHints,
    explicit_provider: Option<Provider>,
    cli_model: Option<&str>,
) -> Result<ResolvedExecutionTarget> {
    // Phase 2 (2026-05-09-slow-prep): the installed-provider snapshot and
    // selection config are pre-built on the shared `CompositionPrepContext`
    // so this function no longer rediscovers the source repo root, reloads
    // the claudine config, or re-runs host detection.
    let snapshot = &ctx.installed_snapshot;
    let selection_config = ctx.selection_config.as_ref();
    let catalog = match selection_config {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    // Phase 1 (2026-05-09-slow-prep): no global `refresh_blocking()` here.
    // Catalog refresh is deferred until *after* a provider is selected and
    // is scoped to that provider only. Provider selection itself never
    // touches the catalog.
    let favorite = selection_config.and_then(|c| c.favorite);

    let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

    if is_tty {
        if let Some(provider) = explicit_provider {
            // Probe model resolution without catalog to determine whether
            // an env var override makes refresh unnecessary.
            let (_, probe_reason) =
                claudine::composition::resolve_model_with_hints(provider, hints, cli_model, None);
            refresh_for_model_validation(&catalog, provider, hints, Some(&probe_reason));
            let (model, model_reason) = claudine::composition::resolve_model_with_hints(
                provider,
                hints,
                cli_model,
                Some(&catalog),
            );
            return Ok(ResolvedExecutionTarget {
                provider,
                provider_reason: claudine::composition::ProviderResolutionReason::ExplicitFlag,
                model,
                model_reason,
            });
        }
        let plan = claudine::composition::build_picker_plan_with_hints(hints, snapshot, favorite)
            .map_err(|e| eyre!("{e}"))?;
        let provider = super::selection_ui::prompt_one_shot_provider(plan)
            .map_err(|e| eyre!("provider selection cancelled: {e}"))?;
        // Probe model resolution without catalog to determine whether
        // an env var override makes refresh unnecessary.
        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(provider, hints, cli_model, None);
        refresh_for_model_validation(&catalog, provider, hints, Some(&probe_reason));
        let (model, model_reason) = claudine::composition::resolve_model_with_hints(
            provider,
            hints,
            cli_model,
            Some(&catalog),
        );
        Ok(ResolvedExecutionTarget {
            provider,
            provider_reason: claudine::composition::ProviderResolutionReason::InteractivePicker,
            model,
            model_reason,
        })
    } else {
        // Non-TTY: provider resolution doesn't touch the catalog, so we
        // perform a first pass with `None` to learn the provider, refresh
        // only that provider's catalog (when needed), and re-resolve so
        // model validation observes the freshly fetched data.
        let provider_only = claudine::composition::resolve_target_non_tty_with_hints(
            explicit_provider,
            hints,
            snapshot,
            favorite,
            cli_model,
            None,
        )
        .map_err(|e| eyre!("{e}"))?;
        refresh_for_model_validation(
            &catalog,
            provider_only.provider,
            hints,
            Some(&provider_only.model_reason),
        );
        claudine::composition::resolve_target_non_tty_with_hints(
            explicit_provider,
            hints,
            snapshot,
            favorite,
            cli_model,
            Some(&catalog),
        )
        .map_err(|e| eyre!("{e}"))
    }
}

/// Refresh a single provider's catalog only when frontmatter `model`
/// hints will actually be validated against it.
///
/// CLI `--model`, provider-specific environment variables, and the generic
/// `MODEL` env var all win over the frontmatter `model` hint, so when one
/// of those is supplied the catalog is never consulted and refresh would
/// be wasted work. Static-source providers (Claude, Codex) refresh in O(1)
/// with no subprocess, but we still skip when no validation will occur.
pub(crate) fn refresh_for_model_validation(
    catalog: &claudine::model_catalog::ModelCatalogService,
    provider: Provider,
    hints: &claudine::composition::EffectiveSelectionHints,
    resolved_model_reason: Option<&ModelResolutionReason>,
) {
    if hints.model.is_none() {
        return;
    }
    if matches!(
        resolved_model_reason,
        Some(
            ModelResolutionReason::ExplicitCli
                | ModelResolutionReason::ProviderEnv(_)
                | ModelResolutionReason::GenericEnv
        )
    ) {
        return;
    }
    let _span =
        tracing::info_span!("compose_prep.model_catalog", provider = %provider.as_slug()).entered();
    // W3: prefer non-blocking refresh so the current run never waits on a
    // dynamic-source subprocess (`opencode models`) when a cache already
    // exists. The async path falls back to blocking on true cold-cache.
    catalog.refresh_provider_async(provider);
}

/// Same gating as [`refresh_for_model_validation`] but reads the
/// `model` hint from a fully prepared composition.
fn refresh_for_prepared_model_validation(
    catalog: &claudine::model_catalog::ModelCatalogService,
    provider: Provider,
    prepared: &claudine::composition::PreparedComposition,
    resolved_model_reason: Option<&ModelResolutionReason>,
) {
    refresh_for_model_validation(
        catalog,
        provider,
        &prepared.selection_hints,
        resolved_model_reason,
    );
}

/// Inject `AGENT` into the supplied `env_overrides` map so composition
/// templates and downstream system-prompt rendering see the chosen provider's
/// slug. The wrapper no longer mutates the parent process env for AGENT;
/// child processes receive it through `build_child_env_with_launch` and
/// composition contexts receive it through `env_overrides`.
pub(crate) fn install_agent_env_for_composition(
    target: &ResolvedExecutionTarget,
    env_overrides: &mut std::collections::BTreeMap<String, String>,
) {
    let slug = target.provider.as_slug().to_string();
    env_overrides.insert("AGENT".to_string(), slug);
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
    let total_start = std::time::Instant::now();
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
    let mut last_checkpoint = total_start;
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
    let target = if let Some(ref t) = request.resolved_target {
        t.clone()
    } else {
        let snapshot = match request.installed_snapshot {
            Some(ref s) => s.clone(),
            None => {
                let clients = InstalledAiClients::new();
                let installed: Vec<Provider> = PROVIDERS_DISPLAY_ORDER
                    .into_iter()
                    .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
                    .collect();
                build_installed_snapshot(&installed, &request.excluded, &clients)
            }
        };

        let selection_config = load_selection_config(source_repo_root.unwrap_or(&launch_cwd));
        let catalog = match &selection_config {
            Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
                cfg.model_overrides.clone(),
            ),
            None => claudine::model_catalog::ModelCatalogService::new(),
        };
        // Phase 1 (2026-05-09-slow-prep): refresh is provider-scoped and
        // only runs after we know which provider was selected. The
        // unconditional global `refresh_blocking()` previously emitted
        // from this point was the dominant prep-time cost in the trace
        // and has been removed.
        let favorite = selection_config.as_ref().and_then(|c| c.favorite);

        let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

        if is_tty {
            // TTY mode: explicit flag wins unconditionally; otherwise show picker.
            if let Some(provider) = request.explicit_provider {
                // Probe model resolution without catalog to determine whether
                // an env var override makes refresh unnecessary.
                let (_, probe_reason) = claudine::composition::resolve_model_with_catalog(
                    provider,
                    &request.prepared,
                    request.model.as_deref(),
                    None,
                );
                refresh_for_prepared_model_validation(
                    &catalog,
                    provider,
                    &request.prepared,
                    Some(&probe_reason),
                );
                let (model, model_reason) = claudine::composition::resolve_model_with_catalog(
                    provider,
                    &request.prepared,
                    request.model.as_deref(),
                    Some(&catalog),
                );
                ResolvedExecutionTarget {
                    provider,
                    provider_reason: claudine::composition::ProviderResolutionReason::ExplicitFlag,
                    model,
                    model_reason,
                }
            } else {
                let plan = build_picker_plan(&request.prepared, &snapshot, favorite)
                    .map_err(|e| eyre!("{e}"))?;
                let provider = super::selection_ui::prompt_one_shot_provider(plan)
                    .map_err(|e| eyre!("provider selection cancelled: {e}"))?;
                // Probe model resolution without catalog to determine whether
                // an env var override makes refresh unnecessary.
                let (_, probe_reason) = claudine::composition::resolve_model_with_catalog(
                    provider,
                    &request.prepared,
                    request.model.as_deref(),
                    None,
                );
                refresh_for_prepared_model_validation(
                    &catalog,
                    provider,
                    &request.prepared,
                    Some(&probe_reason),
                );
                let (model, model_reason) = claudine::composition::resolve_model_with_catalog(
                    provider,
                    &request.prepared,
                    request.model.as_deref(),
                    Some(&catalog),
                );
                ResolvedExecutionTarget {
                    provider,
                    provider_reason:
                        claudine::composition::ProviderResolutionReason::InteractivePicker,
                    model,
                    model_reason,
                }
            }
        } else {
            // Non-TTY: provider resolution doesn't touch the catalog. First
            // pass with no catalog to determine the provider, then refresh
            // only that provider, then re-resolve with the catalog so
            // model validation observes the freshly fetched data.
            let provider_only = resolve_target_non_tty_with_catalog(
                request.explicit_provider,
                &request.prepared,
                &snapshot,
                favorite,
                request.model.as_deref(),
                None,
            )
            .map_err(|e| eyre!("{e}"))?;
            refresh_for_prepared_model_validation(
                &catalog,
                provider_only.provider,
                &request.prepared,
                Some(&provider_only.model_reason),
            );
            resolve_target_non_tty_with_catalog(
                request.explicit_provider,
                &request.prepared,
                &snapshot,
                favorite,
                request.model.as_deref(),
                Some(&catalog),
            )
            .map_err(|e| eyre!("{e}"))?
        }
    };

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
        return Err(CompositionError::InlineInteractiveUnsupported(provider.to_string()).into());
    }

    let effective_non_interactive = !request.session_interactive;

    // -- Early header --------------------------------------------------------
    // Emit the execution line as early as possible so the user sees feedback
    // before expensive env/MCP/harness work begins.

    let compose_display = if is_inline {
        Some(crate::output::ComposeDisplay::InlineCompose)
    } else {
        Some(crate::output::ComposeDisplay::Compose)
    };

    // Show the original file reference (e.g., "@prompts/commit.md")
    let compose_source_hint = request.file_ref.clone();

    if !silent {
        let header_env_plan = env::EnvPlan {
            package_context: launch_workspace.package_context.clone(),
            ..Default::default()
        };

        crate::output::log_wrapper_header(
            profile,
            request.yolo,
            effective_non_interactive,
            request.session_interactive,
            detail_requested,
            request.repo,
            compose_display.as_ref(),
            request.sequence,
            request.operation.as_deref(),
            None, // no inline prompt text for compose
            Some(&compose_source_hint),
            &header_env_plan,
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

    record_substage(&mut perf_collector, &mut last_checkpoint, "child env build");

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
                    let (shadow_env, shadow_path) = super::repo_home::build_repo_home_env(
                        provider,
                        env_plan.child_cwd.as_path(),
                        false,
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

                for (key, value) in string_env {
                    env_plan.env.insert(key.into(), value.into());
                }
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

    // Timeout validation
    if request.timeout.is_some() && request.session_interactive {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
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
    let wire_prompt = delivery.as_wire_rpc().map(str::to_string);
    let stdin_seed = delivery.apply_to(&mut child_args);

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

    let sp_display_lines = super::system_prompt::describe_effective(&effective_sp);

    record_substage(
        &mut perf_collector,
        &mut last_checkpoint,
        "stream + prompt delivery",
    );

    if let Some(collector) = perf_collector.as_mut() {
        collector.mark_env_setup_complete();
    }

    // --dry-run: print what would be executed and exit
    if request.dry_run {
        crate::output::log_dry_run(
            profile,
            &binary_path,
            &child_args,
            request.repo,
            &env_plan,
            None,
            child_cwd,
            &term,
            sp_display_lines.as_deref(),
        );
        if let Some(collector) = perf_collector.as_mut() {
            collector.set_dry_run();
        }
        let outcome = SingleCompositionOutcome {
            exit_code: 0,
            provider,
            agent_perf: None,
            // Dry-run never produces a per-iteration summary.
            iteration_signals: None,
        };
        // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
        // The perf report is always emitted to stderr when requested.
        if let Some(collector) = perf_collector {
            let total = total_start.elapsed();
            let report = collector.into_report(total);
            eprint!("{}", crate::perf::render_perf_report(&report));
        }
        return Ok(outcome);
    }

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

    // -- Harness detection from effective frontmatter ---------------------
    // THE key architectural fix: harness properties are read from the
    // composed frontmatter, not from raw source state.

    let harness_enabled =
        claudine::harness::has_harness_properties(&request.prepared.effective_frontmatter);

    let shell_options = build_harness_shell_options_with_cache(
        &request.prepared.resolved_path,
        effective_repo_root,
        request.shared_approval_cache.clone(),
    );

    // --- Lifecycle notification setup ---
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

    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &lifecycle_settings,
        messaging: &lifecycle_messaging,
        term: &term,
        source_path: &request.prepared.resolved_path,
        repo_root: effective_repo_root,
    };

    let mut guard = LifecycleRunGuard::new(lifecycle, &lifecycle_ctx, &emitter);

    if harness_enabled {
        let resolve_ctx = claudine::harness::HarnessResolutionContext {
            source_path: &request.prepared.resolved_path,
            repo_root: effective_repo_root,
        };
        // Validate that the harness plan can be parsed before proceeding.
        let mut plan = claudine::harness::parse_harness_plan(
            &request.prepared.effective_frontmatter,
            &request.prepared.resolved_path,
            &resolve_ctx,
        )
        .map_err(|e| {
            guard.emit_blocked_or_failure();
            eyre!("{e}")
        })?;

        // For inline composition, prepend a system-owned writability check
        // so that handler recovery paths can respond to permission failures
        // instead of hard-failing before the handler system exists.
        if is_inline {
            plan.pre_checks.insert(
                0,
                claudine::harness::inline_writability_pre_check(&request.prepared.resolved_path),
            );
        }

        // ── Pre-flight shell approval for harness commands ───────────
        let _harness_preflight = claudine::composition::resolve_shell_approvals(
            None, // template commands already approved during compose
            None,
            Some(&plan),
            &shell_options,
        )
        .map_err(|e| {
            guard.emit_blocked_or_failure();
            eyre!("{e}")
        })?;

        // Plan is validated; the harness loop will re-parse if needed.
        drop(plan);
    } else if is_inline {
        // Non-harness inline: validate writability using the same OS +
        // provider-policy check that the harness path uses. Without harness
        // frontmatter there is no handler system to recover, so a failure
        // here is fatal.
        let permission_probe =
            WrapperHarnessPermissionProbe::new(provider, child_args.clone(), effective_repo_root);
        claudine::harness::check_write_permission(
            &request.prepared.resolved_path,
            &request.prepared.resolved_path,
            Some(&permission_probe),
        )
        .map_err(|reason| {
            guard.emit_blocked_or_failure();
            eyre!("{reason}")
        })?;
    }

    // Emit a single preflight-complete indicator for direct compose and
    // inline-compose runs. Sequence runs handle their own preflight
    // messaging in the orchestrator (`wrap::sequence::execute_sequence`)
    // and must not re-emit per step.
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

    // -- Preflight output (env details + prompt block) ---------------------
    // The header was already emitted early (right after profile lookup).
    // Now emit the env details and prompt block with full env_plan.

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

    if harness_enabled {
        let harness_mode = if is_inline {
            HarnessPromptMode::Inline
        } else {
            HarnessPromptMode::Compose
        };

        let mut prompt_state = HarnessPromptState {
            mode: harness_mode,
            source_path: request.prepared.resolved_path.clone(),
            original_ref: request.file_ref.clone(),
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

        // Harness loop manages the guard internally; defuse ours.
        guard.defuse();
        let (exit_code, harness_perf) = run_harness_loop(
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
            Some(materialized_harness_prompt_from_prepared(&request.prepared)),
            &term,
            lifecycle,
            &lifecycle_ctx,
            &emitter,
            true,
        )?;
        if let (Some(collector), Some(perf)) = (perf_collector.as_mut(), harness_perf) {
            collector.set_agent_perf(perf);
        }
        let outcome = SingleCompositionOutcome {
            exit_code,
            provider,
            agent_perf: perf_collector
                .as_ref()
                .and_then(|c| c.agent_perf())
                .or(harness_perf),
            // The harness loop manages its own per-step summaries
            // internally; surfacing them through this outer struct is a
            // future enhancement. For now `compose --loop` against a
            // harness-enabled provider falls back to the legacy
            // behavior (no rate-limit-aware pause and no `exit_reason`
            // pickup at the loop boundary).
            iteration_signals: None,
        };
        // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
        // The perf report is always emitted to stderr when requested.
        if let Some(collector) = perf_collector {
            let total = total_start.elapsed();
            let report = collector.into_report(total);
            eprint!("{}", crate::perf::render_perf_report(&report));
        }
        Ok(outcome)
    } else {
        guard.emit_start_once();

        let mode = if is_inline {
            let closure_plan = match &request.prepared.closure {
                CompositionClosurePlan::Inline(plan) => plan,
                _ => unreachable!("is_inline is true but closure is not Inline"),
            };
            CompositionExecutionMode::Inline {
                closure_plan,
                resolved_path: &request.prepared.resolved_path,
                session_interactive: request.session_interactive,
                show_checks,
            }
        } else {
            CompositionExecutionMode::Direct
        };

        // Non-harness compose: no `*_warn` thresholds are parseable from
        // frontmatter (no harness block), but we still anchor the
        // periodic `t=0` / `t=10m` timing header on this prompt so users
        // see their composition running.
        let prompt_timing = Some(build_prompt_timing_context(
            &request.prepared.resolved_path,
            effective_repo_root,
            None,
            None,
        ));

        let timeout_config = resolve_timeouts(
            request.timeout.clone(),
            None,
            request.step_timeout.clone(),
            None,
        )
        .with_provider(provider);

        let mut child_spawned = false;
        let mut agent_perf: Option<crate::perf::AgentExecutionPerf> = None;
        let mut iteration_signals: Option<IterationSummarySignals> = None;
        let exit_result = execute_without_harness(
            mode,
            provider,
            profile,
            &binary_path,
            &child_args,
            &env_plan.env,
            child_cwd,
            stdin_seed.as_deref(),
            wire_prompt.as_deref(),
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            stream_verbosity,
            detail_requested,
            &env_context,
            &dispatch_context,
            &term,
            &mut child_spawned,
            prompt_timing,
            &mut agent_perf,
            &mut iteration_signals,
            timeout_config,
        );

        // Mark launched as soon as spawn succeeded — before propagating
        // any post-spawn error — so the guard correctly classifies
        // subsequent failures as `Failure` rather than `Blocked`.
        if child_spawned {
            guard.mark_provider_launched();
        }
        let exit_code = exit_result?;

        if exit_code == 0 {
            guard.emit_terminal(LifecycleSignal::Success);
        } else {
            guard.emit_terminal(LifecycleSignal::Failure);
        }

        if let (Some(collector), Some(perf)) = (perf_collector.as_mut(), agent_perf) {
            collector.set_agent_perf(perf);
        }
        let outcome = SingleCompositionOutcome {
            exit_code,
            provider,
            agent_perf: perf_collector
                .as_ref()
                .and_then(|c| c.agent_perf())
                .or(agent_perf),
            iteration_signals,
        };
        // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
        // The perf report is always emitted to stderr when requested.
        if let Some(collector) = perf_collector {
            let total = total_start.elapsed();
            let report = collector.into_report(total);
            eprint!("{}", crate::perf::render_perf_report(&report));
        }
        Ok(outcome)
    }
}

// -- Composition execution (non-harness) ----------------------------------

/// Execute a composition request without the harness loop.
///
/// Shared implementation for both `compose` (Direct) and `inline-compose`
/// (Inline). Mode-specific behavior is gated by [`CompositionExecutionMode`]:
///
/// - **Direct (compose)**: post-hoc assistant text is routed through the live
///   sink's section stream so the trailer summary sees consistent state, and
///   the summary is emitted immediately after the run.
/// - **Inline (inline-compose)**: assistant text is written straight to
///   stdout (the body is also captured for closure write-back), the agent
///   response is validated against the configured closure plan, the target
///   file is rewritten and cleaned, and the summary is deferred until after
///   closure validation messages so the section separator does not split
///   that block.
#[allow(clippy::too_many_arguments)]
fn execute_without_harness(
    mode: CompositionExecutionMode<'_>,
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &std::path::Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &std::path::Path,
    stdin_seed: Option<&str>,
    wire_prompt: Option<&str>,
    use_structured: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    detail_requested: bool,
    env_context: &claudine::events::EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
    child_spawned: &mut bool,
    prompt_timing: Option<claudine::stream::prompt_timing::PromptTimingContext>,
    agent_perf_out: &mut Option<crate::perf::AgentExecutionPerf>,
    iteration_signals_out: &mut Option<IterationSummarySignals>,
    timeout_config: super::subagent_watchdog::TimeoutConfig,
) -> Result<i32> {
    let is_inline = matches!(mode, CompositionExecutionMode::Inline { .. });

    let (agent_exit, final_response, deferred_summary) = if use_structured {
        structured::run_structured_branch(
            provider,
            profile,
            binary_path,
            child_args,
            child_env,
            child_cwd,
            stdin_seed,
            wire_prompt,
            structured_codex_output,
            stderr_noise,
            stream_verbosity,
            env_context,
            dispatch_context,
            child_spawned,
            prompt_timing,
            timeout_config,
            is_inline,
            agent_perf_out,
            term,
        )?
    } else {
        legacy_goose::run_legacy_branch(
            mode,
            provider,
            profile,
            binary_path,
            child_args,
            child_env,
            child_cwd,
            stdin_seed,
            stdout_noise,
            stderr_noise,
            structured_codex_output,
            child_spawned,
            agent_perf_out,
            timeout_config,
            term,
        )?
    };

    let _span = tracing::info_span!("composition_postprocess").entered();

    // Lift loop-relevant signals from the per-iteration summary before
    // the summary is consumed by the renderer below. The `compose --loop`
    // orchestrator reads these to apply the rate-limit policy and to
    // build an honest `LoopIterationFailed` error.
    if let Some(result) = deferred_summary.as_ref() {
        *iteration_signals_out = Some(IterationSummarySignals::from_summary(&result.summary));
    }

    match mode {
        CompositionExecutionMode::Direct => {
            if let Some(result) = deferred_summary {
                summary::emit_composition_summary(
                    &result.summary,
                    &result.details,
                    profile,
                    env_context,
                    stream_verbosity,
                    detail_requested,
                    dispatch_context,
                    Some(&result.section_stream),
                    false,
                );
            } else {
                summary::emit_minimal_composition_summary(
                    provider,
                    agent_exit,
                    profile,
                    env_context,
                    dispatch_context,
                );
            }
            Ok(agent_exit)
        }
        CompositionExecutionMode::Inline {
            closure_plan,
            resolved_path,
            session_interactive,
            show_checks,
        } => inline_guards::apply_inline_closure(
            agent_exit,
            final_response,
            deferred_summary,
            closure_plan,
            resolved_path,
            session_interactive,
            show_checks,
            provider,
            profile,
            env_context,
            stream_verbosity,
            detail_requested,
            dispatch_context,
            term,
            child_cwd,
        ),
    }
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
mod tests {
    use super::*;

    /// W0 regression: when the executor is given a precomputed
    /// `prep_launch_workspace`, [`select_launch_workspace`] must
    /// short-circuit before reaching the legacy
    /// `env::resolve_launch_workspace_context` walk. The fallback
    /// counter is process-global so we reset it under `serial_test` to
    /// observe a clean baseline.
    #[test]
    #[serial_test::serial]
    fn select_launch_workspace_uses_prep_without_calling_fallback() {
        use std::path::PathBuf;
        reset_launch_workspace_fallbacks_for_tests();

        let prep = env::LaunchWorkspaceContext {
            launch_cwd: PathBuf::from("/tmp/launch"),
            repo_root: Some(PathBuf::from("/tmp/launch")),
            child_cwd: PathBuf::from("/tmp/launch"),
            package_context: None,
            warnings: Vec::new(),
        };

        let result = select_launch_workspace(Some(&prep), Path::new("/tmp/launch"), None);
        assert_eq!(result.launch_cwd, prep.launch_cwd);
        assert_eq!(
            launch_workspace_fallback_count_for_tests(),
            0,
            "providing prep_launch_workspace must not call the fallback walker"
        );
    }

    /// W0 contract: the fallback path is still reachable for legacy
    /// callers that don't thread a `CompositionPrepContext` (e.g. a
    /// hand-built library invocation). Calling without `prep` must
    /// increment the counter exactly once so a future refactor that
    /// adds a hidden second walk would fail this assertion.
    #[test]
    #[serial_test::serial]
    fn select_launch_workspace_falls_back_once_when_prep_missing() {
        reset_launch_workspace_fallbacks_for_tests();
        // Point the fallback walker at an empty tempdir rather than the real
        // current dir: the counter increments before the walk, so the
        // contract holds, while avoiding an expensive repo scan of the whole
        // monorepo worktree.
        let cwd = tempfile::tempdir().unwrap();

        let _ = select_launch_workspace(None, cwd.path(), None);

        assert_eq!(
            launch_workspace_fallback_count_for_tests(),
            1,
            "missing prep_launch_workspace must call the fallback walker exactly once"
        );
    }

    #[test]
    fn enforce_repo_launch_detection_passes_when_no_error() {
        // Successful prep-time sniff scan: the executor should proceed
        // regardless of whether `--repo` is set.
        assert!(enforce_repo_launch_detection(false, None).is_ok());
        assert!(enforce_repo_launch_detection(true, None).is_ok());
    }

    #[test]
    fn enforce_repo_launch_detection_passes_when_repo_off() {
        // Sniff failed during prep but `--repo` is not set, so the
        // best-effort default is acceptable and the executor proceeds.
        assert!(
            enforce_repo_launch_detection(false, Some("filesystem probe failed: io error")).is_ok()
        );
    }

    #[test]
    fn enforce_repo_launch_detection_fails_when_repo_and_sniff_failed() {
        // The legacy contract: `--repo` requires startup repo detection,
        // so a captured prep-time sniff failure must abort the run with
        // a hard error that surfaces the original sniff message.
        let result = enforce_repo_launch_detection(true, Some("filesystem probe failed: io error"));
        let err = result.expect_err("--repo + prep sniff failure must error");
        let message = err.to_string();
        assert!(
            message.contains("--repo requires startup repo detection"),
            "expected --repo guard message, got: {message}"
        );
        assert!(
            message.contains("filesystem probe failed: io error"),
            "expected captured sniff error in message, got: {message}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn load_selection_config_returns_both_favorite_and_overrides() {
        use claudine::config::claudine_config::{
            DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
        };

        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();
        let claudine_dir = home.join(".claudine");
        std::fs::create_dir_all(&claudine_dir).unwrap();

        use claudine::config::claudine_config::ClaudineConfig;

        let config = ClaudineConfig {
            preferred_agent: Some(Provider::Codex),
            models: {
                let mut m = HashMap::new();
                m.insert(
                    Provider::Codex,
                    ProviderModelOverride::Detailed(DetailedModelOverride {
                        mode: ModelOverrideMode::Add,
                        values: vec!["gpt-5".into()],
                    }),
                );
                m
            },
            ..ClaudineConfig::default()
        };
        let config_path = claudine_dir.join("config.json");
        claudine::dispatch::loader::save_claudine_config(&config, &config_path).unwrap();

        let old_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", home);
        }

        let result = load_selection_config(home);

        unsafe {
            if let Some(old) = old_home {
                std::env::set_var("HOME", old);
            } else {
                std::env::remove_var("HOME");
            }
        }

        let cfg = result.expect("should load config");
        assert_eq!(cfg.favorite, Some(Provider::Codex));
        assert!(cfg.model_overrides.contains_key(&Provider::Codex));
        assert_eq!(cfg.model_overrides[&Provider::Codex].values(), &["gpt-5"]);
    }

    #[test]
    fn load_selection_config_handles_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let nonexistent = dir.path().join("no-such-config.json");
        let result =
            claudine::dispatch::loader::load_claudine_config(Some(&nonexistent), None);
        assert!(
            result.is_err(),
            "expected error for missing config file"
        );
    }

    #[test]
    fn catalog_initialized_with_config_overrides() {
        use claudine::config::claudine_config::{
            DetailedModelOverride, ModelOverrideMode, ProviderModelOverride,
        };

        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Codex,
            ProviderModelOverride::Detailed(DetailedModelOverride {
                mode: ModelOverrideMode::Add,
                values: vec!["gpt-5".into()],
            }),
        );

        let config = SelectionConfig {
            favorite: Some(Provider::Codex),
            model_overrides: overrides,
        };

        let catalog =
            claudine::model_catalog::ModelCatalogService::with_overrides(config.model_overrides);

        // Static catalog model should still be valid (additive mode)
        assert!(catalog.is_valid(Provider::Codex, "o3-mini"));
        // Override model should also be valid
        assert!(catalog.is_valid(Provider::Codex, "gpt-5"));
        // Non-overridden provider should use static catalog
        assert!(catalog.is_valid(Provider::Claude, "claude-3-7-sonnet-20250219"));
    }

    /// RAII guard that restores the prior value of an env var on drop. Used
    /// to keep the resolve_timeouts tests hermetic.
    struct EnvGuard {
        key: &'static str,
        prior: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prior = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, prior }
        }

        fn clear(key: &'static str) -> Self {
            let prior = std::env::var(key).ok();
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, prior }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.prior {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_cli_wins_over_frontmatter_env_and_default() {
        let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
        let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");

        let cfg = resolve_timeouts(
            Some("60s".into()),
            Some(std::time::Duration::from_secs(120)),
            Some("45s".into()),
            Some(std::time::Duration::from_secs(90)),
        );
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(60)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(45)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_frontmatter_wins_over_env_and_default() {
        let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
        let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");

        let cfg = resolve_timeouts(
            None,
            Some(std::time::Duration::from_secs(7200)),
            None,
            Some(std::time::Duration::from_secs(900)),
        );
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(900)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_env_wins_over_built_in_default() {
        let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
        let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "10m");

        let cfg = resolve_timeouts(None, None, None, None);
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(3600)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(600)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_built_in_default_used_when_nothing_set() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        let cfg = resolve_timeouts(None, None, None, None);
        // No CLI/frontmatter/env: timeout has no built-in default (None);
        // step_timeout falls back to 30m.
        assert_eq!(cfg.timeout, None);
        assert_eq!(
            cfg.step_timeout,
            Some(std::time::Duration::from_secs(30 * 60))
        );
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_zero_env_disables_rule() {
        let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "0s");
        let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "0s");

        let cfg = resolve_timeouts(None, None, None, None);
        assert_eq!(cfg.timeout, None);
        assert_eq!(cfg.step_timeout, None);
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_invalid_env_falls_back_to_built_in() {
        let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "garbage");
        let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "also garbage");

        let cfg = resolve_timeouts(None, None, None, None);
        assert_eq!(cfg.timeout, None);
        assert_eq!(
            cfg.step_timeout,
            Some(std::time::Duration::from_secs(30 * 60))
        );
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_accepts_duration_strings_cli() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        let cfg = resolve_timeouts(Some("2h".into()), None, Some("5m".into()), None);
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(300)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_cli_zero_rejected() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        // parse_timeout rejects 0s, so CLI layer falls through to next
        // precedence (which is None here), resulting in built-in defaults.
        let cfg = resolve_timeouts(Some("0s".into()), None, Some("0s".into()), None);
        assert_eq!(cfg.timeout, None);
        assert_eq!(
            cfg.step_timeout,
            Some(std::time::Duration::from_secs(30 * 60))
        );
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_accepts_hour_and_minute_cli() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        let cfg = resolve_timeouts(Some("2h".into()), None, Some("30m".into()), None);
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(1800)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_cli_duration_string_parsed() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        let cfg = resolve_timeouts(Some("2h".into()), None, Some("5m".into()), None);
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(300)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_rejects_bare_seconds_cli() {
        let _g1 = EnvGuard::set("CLAUDINE_TIMEOUT", "1h");
        let _g2 = EnvGuard::set("CLAUDINE_STEP_TIMEOUT", "5m");

        // Bare seconds like "60" are rejected by parse_timeout, so CLI
        // falls through to env / frontmatter / built-in.
        let cfg = resolve_timeouts(Some("60".into()), None, Some("45".into()), None);
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(3600)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(300)));
    }

    // -- Dynamic refresh gating tests (Phase 2) ---------------------------

    fn make_hints_with_model(model: &str) -> claudine::composition::EffectiveSelectionHints {
        use claudine::composition::ModelHint;
        claudine::composition::EffectiveSelectionHints {
            agent: None,
            model: Some(ModelHint::Single(model.into())),
        }
    }

    #[test]
    #[serial_test::serial]
    fn opencode_model_env_skips_refresh_for_frontmatter_model() {
        let _g = EnvGuard::set("OPENCODE_MODEL", "fast");
        let _g2 = EnvGuard::clear("MODEL");

        let tmp = tempfile::tempdir().unwrap();
        let catalog =
            claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let hints = make_hints_with_model("slow");

        // Probe resolution without catalog tells us the model comes from env
        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(Provider::OpenCode, &hints, None, None);
        assert!(matches!(
            probe_reason,
            ModelResolutionReason::ProviderEnv("OPENCODE_MODEL")
        ));

        refresh_for_model_validation(&catalog, Provider::OpenCode, &hints, Some(&probe_reason));

        // No opencode models subprocess should have been attempted
        assert_eq!(catalog.opencode_fetch_attempts(), 0);
    }

    #[test]
    #[serial_test::serial]
    fn generic_model_env_skips_refresh_for_frontmatter_model() {
        let _g = EnvGuard::set("MODEL", "fast");
        let _g2 = EnvGuard::clear("OPENCODE_MODEL");

        let tmp = tempfile::tempdir().unwrap();
        let catalog =
            claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let hints = make_hints_with_model("slow");

        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(Provider::OpenCode, &hints, None, None);
        assert!(matches!(probe_reason, ModelResolutionReason::GenericEnv));

        refresh_for_model_validation(&catalog, Provider::OpenCode, &hints, Some(&probe_reason));

        assert_eq!(catalog.opencode_fetch_attempts(), 0);
    }

    #[test]
    #[serial_test::serial]
    fn provider_specific_model_env_skips_refresh_for_frontmatter_model() {
        let _g = EnvGuard::set("CLAUDE_MODEL", "claude-3-7-sonnet-20250219");
        let _g2 = EnvGuard::clear("MODEL");

        let tmp = tempfile::tempdir().unwrap();
        let catalog =
            claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let hints = make_hints_with_model("slow");

        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(Provider::Claude, &hints, None, None);
        assert!(matches!(
            probe_reason,
            ModelResolutionReason::ProviderEnv("CLAUDE_MODEL")
        ));

        refresh_for_model_validation(&catalog, Provider::Claude, &hints, Some(&probe_reason));

        // Claude is a static provider; refresh writes to cache. With env
        // override the refresh should be skipped, so no cache file.
        let cache_file = tmp.path().join("claude.json");
        assert!(!cache_file.exists(), "refresh should have been skipped");
    }

    #[test]
    #[serial_test::serial]
    fn frontmatter_model_without_env_override_refreshes_dynamic_provider() {
        let _g1 = EnvGuard::clear("OPENCODE_MODEL");
        let _g2 = EnvGuard::clear("MODEL");

        let tmp = tempfile::tempdir().unwrap();
        let catalog =
            claudine::model_catalog::ModelCatalogService::with_cache_dir(tmp.path().to_path_buf());
        let hints = make_hints_with_model("slow");

        let (_, probe_reason) =
            claudine::composition::resolve_model_with_hints(Provider::OpenCode, &hints, None, None);
        assert!(matches!(
            probe_reason,
            ModelResolutionReason::FrontmatterSingle | ModelResolutionReason::ProviderDefault
        ));

        refresh_for_model_validation(&catalog, Provider::OpenCode, &hints, Some(&probe_reason));

        // Refresh should have been attempted (will fail gracefully since
        // opencode is not on PATH, but the attempt counter increments).
        assert_eq!(catalog.opencode_fetch_attempts(), 1);
    }
}
