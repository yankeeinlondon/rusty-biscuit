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
    invalid_agent_message, resolve_target_non_tty_with_catalog, route_blocked_finalize,
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

pub(crate) mod dry_run;
pub(crate) mod launch;
mod preflight;
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
    setup_phase_deferred, surface_preflight_catch_error,
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
        crate::output::emit_execution_header(
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
                crate::commands::exec_prep::ensure_shadow_home(
                    provider,
                    needs_mcp_shadow_home,
                    &mut env_plan,
                )?;
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

    // Seed the child argv with the forwarded provider tail, at the same base
    // position as direct-wrapper passthrough — ahead of Claudine's entrypoint,
    // model, transport, system-prompt, MCP, and prompt-delivery injections
    // (`apply_entrypoint` inserts the entrypoint subcommand at index 0, so a
    // tail-first base still lands after it, exactly like the wrapper). Announce
    // it once per distinct (provider, tail) before launch.
    provider_args::announce_forwarded_tail(
        provider,
        &request.provider_args,
        request.provider_args_explicit,
        silent,
        quiet,
        &term,
    );
    let mut child_args = request.provider_args.clone();

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

    // Model resolution, universal --model, and non-interactive validation —
    // shared prep stage (see `commands::exec_prep`). Unlike the direct
    // wrapper, composition propagates every failure (including OpenCode
    // no-model) as an error.
    let has_model_env = env_plan
        .env
        .contains_key(&std::ffi::OsString::from("MODEL"));
    let _opencode_model_source: Option<super::profile::OpenCodeModelSource> =
        crate::commands::exec_prep::resolve_model_and_validate(
            provider,
            profile,
            &mut child_args,
            target.model.as_deref(),
            effective_non_interactive,
            has_model_env,
            &mut |key, value| {
                env_plan.env.insert(key.into(), value.into());
            },
            &mut |warn| {
                if !silent && !quiet {
                    log::warn(&warn);
                }
            },
        )
        .map_err(crate::commands::exec_prep::ModelStageError::into_report)?;

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

    let structured_codex_output = crate::commands::exec_prep::prepare_codex_structured_output(
        provider,
        use_structured,
        request.session_interactive && is_inline,
        &mut child_args,
    );

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
    // `run_composition_body` call below can capture these by reference and route
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
    // (single-run / first loop iteration). The shared, read-only state is
    // bundled in `CompositionRunCtx`; the mutable `guard` and
    // `perf_collector` vary per call and are passed explicitly.
    let ctx = runner::CompositionRunCtx {
        request: &request,
        target: &target,
        provider,
        effective_repo_root,
        launch_workspace: &launch_workspace,
        launch_cwd: &launch_cwd,
        binary_path: &binary_path,
        lifecycle_effect_engine: &lifecycle_effect_engine,
        emitter: &emitter,
        lifecycle_settings: &lifecycle_settings,
        lifecycle_messaging: &lifecycle_messaging,
        lifecycle_context: &lifecycle_context,
        term: &term,
        document_start,
        shell_options: &shell_options,
        silent,
        quiet,
        is_inline,
        profile,
        effective_non_interactive,
        args_before_prompt: &args_before_prompt,
        child_cwd,
        use_structured,
        structured_codex_output: &structured_codex_output,
        stdout_noise,
        stderr_noise,
        show_checks,
        stream_verbosity,
        detail_requested,
        env_plan: &env_plan,
        effective_sp: &effective_sp,
        verbose,
    };

    // When the caller passes an external guard (loop re-entry), it has already
    // emitted `initialize` and owns the lifecycle runtime context. Run only
    // the per-iteration body and return.
    if let Some(guard) = external_guard {
        return runner::run_composition_body(
            &ctx,
            guard,
            &mut perf_collector,
            skip_preflight,
            None,
        );
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
    // run is handed off to. Threaded into `run_composition_body` so the harness loop
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
                // `run_composition_body` runs the target via the harness loop's
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

    runner::run_composition_body(
        &ctx,
        &mut guard,
        &mut perf_collector,
        false,
        init_proxy_target.as_deref(),
    )
}

#[cfg(test)]
mod tests;
