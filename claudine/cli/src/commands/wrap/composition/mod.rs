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
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::{
    DefaultLifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext, LifecycleSignal,
};
use claudine::composition::{
    CompositionClosurePlan, CompositionError, CompositionExecutionRequest, CompositionMode,
    InlineClosurePlan, ResolvedExecutionTarget, SelectionReason, build_installed_snapshot,
    build_picker_plan, resolve_target_non_tty_with_catalog,
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
    resolve_binary_path, run_harness_loop, structured_verbosity, switch_process_cwd, wrap_terminal,
};
use crate::log;

pub(crate) mod structured;
pub(crate) mod summary;
pub(crate) mod inline_guards;
pub(crate) mod legacy_goose;

// Re-export the public API so existing callers don't break.
pub(crate) use structured::run_structured_composition;
pub(crate) use summary::{emit_composition_summary, emit_minimal_composition_summary};
pub(crate) use inline_guards::{cleanup_inline_output, split_frontmatter_and_body};

/// Result of executing a single composition step through the wrapper pipeline.
pub(crate) struct SingleCompositionOutcome {
    /// The process exit code.
    pub exit_code: i32,
    /// The provider that ran the step.
    pub provider: Provider,
    /// Execution perf metadata, when `--perf` was enabled.
    pub agent_perf: Option<crate::perf::AgentExecutionPerf>,
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

fn resolve_prompt_display_path(
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
    hints: &claudine::composition::EffectiveSelectionHints,
    explicit_provider: Option<Provider>,
    excluded: &std::collections::BTreeSet<Provider>,
    cli_model: Option<&str>,
    source_repo_root: Option<&Path>,
) -> Result<ResolvedExecutionTarget> {
    let cwd = std::env::current_dir()?;
    let clients = InstalledAiClients::new();
    let installed: Vec<Provider> = PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
        .collect();
    let snapshot = build_installed_snapshot(&installed, excluded);

    let selection_config = load_selection_config(source_repo_root.unwrap_or(&cwd));
    let catalog = match &selection_config {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    catalog.refresh_blocking();
    let favorite = selection_config.as_ref().and_then(|c| c.favorite);

    let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

    if is_tty {
        if let Some(provider) = explicit_provider {
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
        let plan = claudine::composition::build_picker_plan_with_hints(hints, &snapshot, favorite)
            .map_err(|e| eyre!("{e}"))?;
        let provider = super::selection_ui::prompt_one_shot_provider(plan)
            .map_err(|e| eyre!("provider selection cancelled: {e}"))?;
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
        claudine::composition::resolve_target_non_tty_with_hints(
            explicit_provider,
            hints,
            &snapshot,
            favorite,
            cli_model,
            Some(&catalog),
        )
        .map_err(|e| eyre!("{e}"))
    }
}

/// Inject `AGENT` into both the parent process env and the supplied
/// `env_overrides` map so composition templates and downstream
/// system-prompt rendering see the chosen provider's slug.
pub(crate) fn install_agent_env_for_composition(
    target: &ResolvedExecutionTarget,
    env_overrides: &mut std::collections::BTreeMap<String, String>,
) {
    let slug = target.provider.as_slug().to_string();
    // SAFETY: composition entry runs on the main task before any worker
    // threads or hooks have spawned. Setting AGENT here is the only
    // mutation of the parent process env at this point, satisfying
    // Rust 2024's `set_var` safety contract.
    unsafe {
        std::env::set_var("AGENT", &slug);
    }
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

    let _span = tracing::info_span!("composition_prepare").entered();

    let term = wrap_terminal();
    let launch_cwd = std::env::current_dir()?;
    let detail_requested = verbose > 0;
    let quiet = request.quiet;
    let silent = request.silent;
    let show_checks = !silent;

    let source_repo_root = request.prepared.source_repo_root.as_deref();
    let launch_workspace = env::resolve_launch_workspace_context(&launch_cwd, source_repo_root);

    // -- Provider detection and selection ---------------------------------

    let clients = InstalledAiClients::new();
    let installed: Vec<Provider> = PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
        .collect();

    let snapshot = build_installed_snapshot(&installed, &request.excluded);

    let selection_config = load_selection_config(source_repo_root.unwrap_or(&launch_cwd));
    let catalog = match &selection_config {
        Some(cfg) => claudine::model_catalog::ModelCatalogService::with_overrides(
            cfg.model_overrides.clone(),
        ),
        None => claudine::model_catalog::ModelCatalogService::new(),
    };
    catalog.refresh_blocking();
    let favorite = selection_config.as_ref().and_then(|c| c.favorite);

    // If a target was already resolved upstream (sequence review, non-TTY
    // preflight, etc.), use it directly.
    let target = if let Some(ref t) = request.resolved_target {
        t.clone()
    } else {
        let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();

        if is_tty {
            // TTY mode: explicit flag wins unconditionally; otherwise show picker.
            if let Some(provider) = request.explicit_provider {
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
            // Non-TTY mode: strict chain resolution, never prompt.
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

    // -- Profile, binary, arguments, environment --------------------------

    let profile = profile::profile_for_provider(provider)
        .ok_or_else(|| eyre!("'{}' cannot be wrapped", provider))?;
    let binary_path = resolve_binary_path(profile, &clients)?;

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

    let needs_mcp_shadow_home = (request.mcp || !request.mcp_use.is_empty())
        && matches!(provider, Provider::Codex | Provider::Gemini);
    let needs_repo_shadow_home = request.repo;
    let raw_agent_params: Vec<String> = std::env::args().skip(1).collect();
    let yolo_enabled = request.yolo;
    let mut env_plan = env::build_child_env(
        profile,
        provider,
        &request.include,
        yolo_enabled,
        request.session_interactive,
        &raw_agent_params,
        &launch_cwd,
        &[],
        needs_repo_shadow_home,
        needs_mcp_shadow_home || needs_repo_shadow_home,
        source_repo_root,
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
    }

    let mut child_args = Vec::new();

    // -- Yolo ----------------------------------------------------------------

    if request.yolo {
        let mut env_overrides = Vec::new();
        if let Some(warn) = profile.apply_yolo_for_mode(
            &mut child_args,
            &mut env_overrides,
            !effective_non_interactive,
        )? && !silent
            && !quiet
        {
            log::warn(&warn);
        }
        for (key, value) in env_overrides {
            env_plan.env.insert(key.into(), value.into());
        }
        // Note: yolo support already consumed at env_plan build time
    }

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

    let launch_context = match claudine::system_prompt::LaunchContext::from_cwd(&launch_cwd) {
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
            }
        }
    };
    let effective_sp = claudine::system_prompt::resolve_and_prepare_for_session(
        &request.system_prompt_args,
        &launch_context,
        effective_non_interactive,
    )?;

    let mut sp_artifacts: Vec<super::system_prompt::SystemPromptArtifact> = Vec::new();

    match &effective_sp {
        claudine::system_prompt::EffectiveSystemPrompt::None
        | claudine::system_prompt::EffectiveSystemPrompt::Disabled { .. } => {}
        claudine::system_prompt::EffectiveSystemPrompt::Ready(prepared) => {
            let application =
                profile.apply_system_prompt(prepared, !effective_non_interactive, &launch_cwd)?;
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
        use biscuit_terminal::prelude::Renderable as _;

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
    let env_detect_root = effective_repo_root.unwrap_or(&launch_cwd);
    let env_context = claudine::events::detect_environment_fast(env_detect_root);

    if !silent {
        if !quiet && (request.session_interactive || detail_requested) {
            crate::output::log_wrapper_env_details(&env_plan, None, &term, verbose);
        }

        crate::output::log_system_prompt(&effective_sp, detail_requested, silent, quiet, &term);

        if matches!(
            effective_sp,
            claudine::system_prompt::EffectiveSystemPrompt::Ready(_)
        ) && effective_non_interactive
        {
            crate::log::message("");
        }

        if effective_non_interactive {
            crate::output::log_compose_prompt(&request.prepared.prompt, detail_requested, &term);
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
            Some(materialized_harness_prompt_from_prepared(&request.prepared,
            )),
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
        );

        let mut child_spawned = false;
        let mut agent_perf: Option<crate::perf::AgentExecutionPerf> = None;
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
    let config =
        claudine::dispatch::loader::load_claudine_config(None, repo_root.as_deref()).ok()?;
    Some(SelectionConfig {
        favorite: config.preferred_agent,
        model_overrides: config.models,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    #[serial_test::serial]
    fn load_selection_config_handles_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path();

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

        assert!(result.is_none());
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
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(30 * 60)));
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
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(30 * 60)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_accepts_duration_strings_cli() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        let cfg = resolve_timeouts(
            Some("2h".into()),
            None,
            Some("5m".into()),
            None,
        );
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
        let cfg = resolve_timeouts(
            Some("0s".into()),
            None,
            Some("0s".into()),
            None,
        );
        assert_eq!(cfg.timeout, None);
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(30 * 60)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_accepts_hour_and_minute_cli() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        let cfg = resolve_timeouts(
            Some("2h".into()),
            None,
            Some("30m".into()),
            None,
        );
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(7200)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(1800)));
    }

    #[test]
    #[serial_test::serial]
    fn resolve_timeouts_cli_duration_string_parsed() {
        let _g1 = EnvGuard::clear("CLAUDINE_TIMEOUT");
        let _g2 = EnvGuard::clear("CLAUDINE_STEP_TIMEOUT");

        let cfg = resolve_timeouts(
            Some("2h".into()),
            None,
            Some("5m".into()),
            None,
        );
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
        let cfg = resolve_timeouts(
            Some("60".into()),
            None,
            Some("45".into()),
            None,
        );
        assert_eq!(cfg.timeout, Some(std::time::Duration::from_secs(3600)));
        assert_eq!(cfg.step_timeout, Some(std::time::Duration::from_secs(300)));
    }
}
