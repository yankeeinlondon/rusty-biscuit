#![allow(unused_imports)]

pub(crate) mod env;
pub(crate) mod exec;
pub(crate) mod live_semantic_sink;
pub(crate) mod profile;
pub(crate) mod repo_home;
pub(crate) mod runaway_guard;
pub(crate) mod section;
pub(crate) mod session_report;
pub(crate) mod stream_io;
pub(crate) mod subagent_watchdog;
pub(crate) mod system_prompt;
pub(crate) mod wire_io;

pub(crate) mod composition;
pub(crate) mod selection_ui;
pub(crate) mod sequence;

// New split modules
pub(crate) mod catalog_drift;
pub(crate) mod flags;
pub(crate) mod harness_orch;
pub(crate) mod inline;
pub(crate) mod overlay;
pub(crate) mod policy;
pub(crate) mod prompt_source;
pub(crate) mod resume;
pub(crate) mod wrapper_stages;
pub(crate) mod wrapper_exec;
pub(crate) mod wrapper_mcp;

// Re-exports from split modules
pub use flags::WrapperArgs;
pub(crate) use flags::{
    extract_wrapper_flags_from_passthrough, has_explicit_native_output_request,
    reject_retired_composition_flags,
};
pub(crate) use harness_orch::{
    AttemptLaunch, CachedHarnessLoopContext, HarnessPromptMode, HarnessPromptState,
    MaterializedHarnessPrompt, apply_composition_shell_overrides, build_harness_launch,
    build_harness_shell_options, build_harness_shell_options_with_cache, execute_harness_attempt,
    find_wrapper_harness_source, harness_policy_root, harness_prompt_mode_label,
    materialize_harness_prompt, materialize_passthrough_harness_seed,
    materialized_harness_prompt_from_prepared, run_harness_loop,
};
pub(crate) use inline::{
    extract_tags_from_prompt, report_inline_agent_status, strip_prompt_tags_for_provider,
    try_inline_closure,
};
pub(crate) use overlay::{frontmatter_map_to_value, merge_frontmatter_overlay};
pub(crate) use policy::{
    StreamSummaryContext, StructuredCodexOutput, StructuredSummaryDetails,
    build_structured_plumbing, emit_stream_summary, format_summary_prose,
    format_verbose_summary_details_prose,
};
pub(crate) use prompt_source::{maybe_edit_prompt_source, maybe_edit_prompt_source_with};
pub(crate) use resume::{
    append_resume_passthrough_args, check_resume_support, normalize_resume_args,
};
use wrapper_stages::{
    apply_opencode_yolo_config_overlay, detect_wrapper_harness, emit_preflight_preamble,
    parse_cli_timeouts, prepare_stream_and_prompt, resolve_and_apply_system_prompt,
    run_execution_stage, validate_timeout_constraints,
};

use biscuit_terminal::terminal::Terminal;
use claudine::composition::InstalledProviderSnapshot;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, WrapErr, eyre};
use profile::{OutputFormat, WrapperProfile};
use sniff::programs::InstalledAiClients;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info_span;

use crate::log;

#[derive(Debug, Clone, Default)]
pub(crate) struct McpRuntimeInfo {
    pub(crate) servers: Vec<String>,
    pub(crate) default_servers: Vec<String>,
    pub(crate) explicit_servers: Vec<String>,
    pub(crate) tag_servers: Vec<String>,
    pub(crate) resolved_tags: Vec<String>,
    pub(crate) missing_tags: Vec<String>,
    pub(crate) ambiguous_tags: Vec<String>,
    pub(crate) cleaned_prompt: Option<String>,
    pub(crate) env_vars_set: Vec<String>,
    pub(crate) temp_files: Vec<PathBuf>,
    pub(crate) extra_args: Vec<String>,
}

pub(crate) fn structured_verbosity(silent: bool, quiet: bool) -> Verbosity {
    if silent {
        Verbosity::Silent
    } else if quiet {
        Verbosity::Quiet
    } else {
        Verbosity::Normal
    }
}

pub(crate) fn wrap_terminal() -> Terminal {
    crate::log::terminal()
}

fn wrap_terminal_for_mode(non_interactive: bool) -> Terminal {
    if non_interactive {
        crate::log::optimistic_terminal(None)
    } else {
        wrap_terminal()
    }
}

#[cfg(test)]
pub(crate) fn resolve_binary_path(
    profile: &dyn WrapperProfile,
    clients: &InstalledAiClients,
) -> Result<PathBuf> {
    let ai_cli = profile.provider().sniff_ai_cli();
    clients
        .path(ai_cli)
        .ok_or_else(|| binary_missing_error(profile))
}

/// Resolve the child binary path directly via `which`, without scanning the
/// entire set of known AI CLIs. Used on the hot path of the direct wrapper
/// so we don't pay for a full PATH walk over ~9 binaries when only one is
/// needed.
///
/// When `snapshot` is provided and contains a path for the provider, the
/// cached path is returned immediately, avoiding the `which` syscall.
pub(crate) fn resolve_binary_path_direct(
    profile: &dyn WrapperProfile,
    snapshot: Option<&InstalledProviderSnapshot>,
) -> Result<PathBuf> {
    if let Some(snapshot) = snapshot
        && let Some(path) = snapshot.binary_path(profile.provider())
    {
        return Ok(path.to_path_buf());
    }
    which::which(profile.binary()).map_err(|_| binary_missing_error(profile))
}

fn binary_missing_error(profile: &dyn WrapperProfile) -> color_eyre::eyre::Error {
    eyre!(
        "cannot run wrapped {} session because '{}' is not installed or not on PATH (docs: {})",
        profile.provider(),
        profile.binary(),
        profile.provider().docs_url()
    )
}

fn bootstrap_mcp_state(repo_root: Option<&std::path::Path>) -> Result<bool> {
    use claudine::mcp::defaults::{save_repo_defaults, save_user_defaults};
    use claudine::mcp::import::McpImporter;
    use claudine::mcp::state::McpProviderStateStore;
    use claudine::mcp::types::{McpDefaults, defaults_path, repo_defaults_path};

    let needs_bootstrap = !claudine::mcp::types::catalog_path().exists()
        || !defaults_path().exists()
        || !claudine::mcp::types::provider_state_path().exists()
        || repo_root.is_some_and(|root| !repo_defaults_path(root).exists());
    if !needs_bootstrap {
        return Ok(false);
    }

    let mut catalog = claudine::mcp::catalog::McpCatalogStore::load()
        .wrap_err("failed to load MCP catalog for bootstrap")?;
    let mut state = McpProviderStateStore::load()
        .wrap_err("failed to load MCP provider-state for bootstrap")?;
    let mut importer = McpImporter::new(&mut catalog, &mut state);
    let _ = importer.import_all(repo_root);
    catalog
        .save()
        .wrap_err("failed to save bootstrapped MCP catalog")?;
    state
        .save()
        .wrap_err("failed to save bootstrapped MCP provider-state")?;

    if !defaults_path().exists() {
        save_user_defaults(&McpDefaults::default())
            .wrap_err("failed to create bootstrapped MCP defaults")?;
    }
    if let Some(repo_root) = repo_root
        && !repo_defaults_path(repo_root).exists()
    {
        save_repo_defaults(repo_root, &McpDefaults::default())
            .wrap_err("failed to create bootstrapped repo MCP defaults")?;
    }

    Ok(true)
}

/// Run a wrapped provider command.
pub fn run_provider_wrapper(
    provider: Provider,
    args: WrapperArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    if args.help {
        flags::print_wrapper_help(provider);
        return Ok(());
    }

    let mut perf_collector =
        startup_timings.map(|timings| crate::perf::CommandPerfCollector::new("Wrapper", timings));

    let (code, stderr_capture, model_source) =
        run_provider_wrapper_inner(provider, args, verbose, perf_collector.as_mut())?;

    if code != 0 {
        let term = wrap_terminal();
        let report = crate::output::error_report::AgentErrorReport::from_exit_code_with_source(
            provider,
            code,
            stderr_capture.as_deref(),
            model_source.as_ref(),
        );
        report.render(&term);
    }

    // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
    // The perf report is always emitted to stderr when requested.
    if let Some(collector) = perf_collector {
        crate::perf::emit_report(&collector.into_report());
    }

    std::process::exit(code);
}

fn run_provider_wrapper_inner(
    provider: Provider,
    args: WrapperArgs,
    verbose: u8,
    mut perf_collector: Option<&mut crate::perf::CommandPerfCollector>,
) -> Result<(i32, Option<String>, Option<profile::OpenCodeModelSource>)> {
    // ------------------------------------------------------------------
    // Stage 1: Resolve profile and binary
    // ------------------------------------------------------------------
    let profile = profile::profile_for_provider(provider).ok_or_else(|| {
        eyre!(
            "'{}' cannot be wrapped (it is a VS Code extension)",
            provider
        )
    })?;

    let cwd = std::env::current_dir()?;

    let binary_path = info_span!(
        "wrapper_binary_resolution",
        provider = %provider,
    )
    .in_scope(|| resolve_binary_path_direct(profile, None))?;

    // ------------------------------------------------------------------
    // Stage 2: Extract launch intent from CLI and passthrough argv
    // ------------------------------------------------------------------
    let raw_agent_params: Vec<String> = std::env::args().skip(2).collect();
    let mut child_args = args.passthrough.clone();
    let extracted = flags::extract_wrapper_flags_from_passthrough(&mut child_args)?;
    let yolo_requested = args.yolo || extracted.yolo;
    let mut yolo_enabled = yolo_requested;
    let interactive_requested = args.interactive || extracted.interactive;
    let edit_requested = args.edit || extracted.edit;
    let repo_requested = args.repo || extracted.repo;
    let quiet_requested = args.quiet || extracted.quiet;
    let silent_requested = args.silent || extracted.silent;
    let detail_requested = verbose > 0 || extracted.verbose;
    let mut env_overrides: Vec<(String, String)> = Vec::new();
    let mut deferred_warnings: Vec<String> = Vec::new();
    let mut deferred_messages: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // Stage 3: Extract and optionally edit the prompt source
    // ------------------------------------------------------------------
    // In the direct-wrap path the child inherits stdin automatically —
    // we don't need to detect or seed it. Pass false so that a non-tty
    // test environment (or any shell running without an attached terminal)
    // doesn't accidentally classify the session as non-interactive.
    // Composition (Task 14) is where InheritStdin / stdin_seed matters.
    let has_piped_stdin = false;

    let (extracted_args, mut prompt_source) =
        profile::extract_prompt_source_from_passthrough(profile, &child_args, has_piped_stdin)?;
    child_args = extracted_args;

    if edit_requested {
        let Some(edited_prompt) =
            prompt_source::maybe_edit_prompt_source(prompt_source, silent_requested)?
        else {
            return Ok((0, None, None));
        };
        prompt_source = edited_prompt;
    }

    // Default: non-interactive when a prompt reaches the child, interactive
    // otherwise. --interactive/-i overrides the default back to interactive.
    let has_prompt = prompt_source.has_prompt_or_stdin();
    let non_interactive_requested = if interactive_requested {
        false
    } else {
        has_prompt
    };

    // ------------------------------------------------------------------
    // Stage 4: Detect startup context (env, launch, workspace)
    // ------------------------------------------------------------------
    let startup = env::detect_wrap_startup_or_fallback(
        &cwd,
        has_prompt,
        repo_requested,
        &mut deferred_warnings,
    )?;
    let env_context = startup.env_context;
    let launch_context = startup.launch_context;
    let launch_workspace = startup.launch_workspace;

    let wrapper_span = info_span!(
        "wrapper_session",
        binary_path = %binary_path.display(),
        has_prompt,
        interactive_requested,
        edit_requested,
        yolo_requested,
        model_override = %args.model.as_deref().unwrap_or(""),
        provider = %provider,
        session_id = tracing::field::Empty,
        child_pid = tracing::field::Empty,
        structured_mode = tracing::field::Empty,
    );
    let _wrapper_guard = wrapper_span.enter();

    // ------------------------------------------------------------------
    // Stage 5: Validate and parse timeouts
    // ------------------------------------------------------------------
    validate_timeout_constraints(
        &args,
        interactive_requested,
        non_interactive_requested,
        edit_requested,
    )?;
    let cli_timeout_duration = parse_cli_timeouts(&args)?;

    // The effective interactivity state is determined solely by the explicit flag.
    let effective_non_interactive = non_interactive_requested;
    let term = wrap_terminal_for_mode(effective_non_interactive);

    // ------------------------------------------------------------------
    // Stage 6: Apply provider profile to argv and env
    // ------------------------------------------------------------------
    profile.reject_direct_yolo(&child_args)?;
    flags::reject_retired_composition_flags(&child_args)?;

    if yolo_requested {
        let outcome = profile.apply_yolo_for_mode(
            &mut child_args,
            &mut env_overrides,
            !non_interactive_requested,
        )?;
        if let Some(warn) = outcome.warning {
            deferred_warnings.push(warn);
        }
        // `outcome.applied` is the single source of truth. The summary
        // and header badge should reflect this — not `yolo_requested`
        // intent on its own — because some launch modes silently
        // suppress the flag (e.g. OpenCode interactive TUI).
        yolo_enabled = outcome.applied;
    }
    if yolo_requested && !profile.has_supported_yolo() {
        yolo_enabled = false;
    }
    tracing::debug!(
        target: "claudine::wrap::yolo",
        provider = %profile.provider(),
        request_yolo = yolo_requested,
        effective_yolo = yolo_enabled,
        non_interactive = non_interactive_requested,
        child_args = ?child_args,
        "yolo applied to provider argv",
    );

    profile.apply_entrypoint(&mut child_args, non_interactive_requested);

    if non_interactive_requested {
        profile.apply_non_interactive_flags(&mut child_args)?;
    }

    // Model resolution, universal --model, and non-interactive validation —
    // shared prep stage (see `commands::exec_prep`). Only the no-model
    // presentation stays wrapper-specific: render the agent error report and
    // exit instead of propagating.
    let has_model_env = env_overrides.iter().any(|(k, _)| k == "MODEL");
    let opencode_model_source: Option<profile::OpenCodeModelSource> =
        match crate::commands::exec_prep::resolve_model_and_validate(
            provider,
            profile,
            &mut child_args,
            args.model.as_deref(),
            non_interactive_requested,
            has_model_env,
            &mut |key, value| env_overrides.push((key, value)),
            &mut |warn| deferred_warnings.push(warn),
        ) {
            Ok(source) => source,
            Err(crate::commands::exec_prep::ModelStageError::NoOpenCodeModel(_)) => {
                let term = wrap_terminal();
                let report =
                    crate::output::error_report::AgentErrorReport::no_model_provided(provider);
                report.render(&term);
                std::process::exit(1);
            }
            Err(err) => return Err(err.into_report()),
        };

    // Universal --output flag
    if let Some(ref output_str) = args.output {
        let format: OutputFormat = output_str.parse().map_err(|e: String| eyre!(e))?;
        if let Some(warn) = profile.apply_output_format(&mut child_args, format) {
            deferred_warnings.push(warn);
        }
    }

    let prompt_display = prompt_source.as_inline().map(|s| s.to_string());
    let interactive_override = interactive_requested && has_prompt;
    let effective_operation = args.operation.clone().or(extracted.operation);

    if let Some(ref op) = effective_operation {
        env_overrides.push(("OPERATION".to_string(), op.clone()));
    }

    // ------------------------------------------------------------------
    // Stage 7: Emit wrapper header
    // ------------------------------------------------------------------
    if !silent_requested {
        let header_env_plan = env::EnvPlan {
            package_context: launch_workspace.package_context.clone(),
            ..Default::default()
        };

        crate::output::log_wrapper_header(
            profile,
            yolo_enabled,
            effective_non_interactive,
            interactive_override,
            detail_requested,
            repo_requested,
            None,
            false, // not a sequence
            effective_operation.as_deref(),
            prompt_display.as_deref(),
            None, // no compose source hint for direct wrapper
            &header_env_plan,
            &term,
        );
    }

    // ------------------------------------------------------------------
    // Stage 8: Resolve and apply system prompt
    // ------------------------------------------------------------------
    let (effective_sp, _sp_artifacts) = resolve_and_apply_system_prompt(
        profile,
        &args,
        &launch_context,
        &launch_workspace,
        &cwd,
        non_interactive_requested,
        &mut child_args,
        &mut env_overrides,
        &mut deferred_warnings,
    )?;

    // ------------------------------------------------------------------
    // Stage 9: Apply sandbox and build child environment
    // ------------------------------------------------------------------
    if args.sandbox
        && let Some(warn) = profile.apply_sandbox(&mut child_args)
    {
        deferred_warnings.push(warn);
    }

    let needs_mcp_shadow_home = (args.mcp || !args.mcp_use.is_empty())
        && matches!(provider, Provider::Codex | Provider::Gemini);

    let mut env_plan = env::build_child_env_with_launch(
        profile,
        provider,
        &args.include,
        yolo_enabled,
        !non_interactive_requested,
        &raw_agent_params,
        &env_overrides,
        repo_requested,
        needs_mcp_shadow_home,
        launch_workspace,
        false,
    )?;

    if !silent_requested && !quiet_requested {
        use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
        use biscuit_terminal::prelude::TerminalRenderable as _;

        let status = Status::from_prose("Starting pre-flight checks".to_string())
            .state(StatusState::Info)
            .theme(StatusTheme::Circular);
        crate::log::message(&status.render(&term));
    }

    if args.timeout.is_some() && !effective_non_interactive {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt)"
        ));
    }

    if args.step_timeout.is_some() && !effective_non_interactive {
        return Err(eyre!(
            "--step-timeout can only be used in non-interactive mode \
             (provide a prompt)"
        ));
    }

    // ------------------------------------------------------------------
    // Stage 10: Compose MCP session
    // ------------------------------------------------------------------
    let (mcp_runtime, mcp_cleanup) = wrapper_mcp::compose_mcp_session(
        &args,
        provider,
        non_interactive_requested,
        &mut env_plan,
        &mut child_args,
        &mut prompt_source,
        &mut deferred_warnings,
        &mut deferred_messages,
    )?;

    // ------------------------------------------------------------------
    // Stage 10.5: OpenCode YOLO config overlay
    // ------------------------------------------------------------------
    // Applied last (after MCP) so the permission block is the authoritative
    // Claudine overlay and survives the MCP / system-prompt writers. Gated on
    // OpenCode + YOLO-took-effect + non-interactive; a no-op otherwise.
    apply_opencode_yolo_config_overlay(
        provider,
        yolo_enabled,
        non_interactive_requested,
        &mut env_plan,
    )?;

    let child_cwd = env_plan.child_cwd.as_path();

    if effective_non_interactive && !silent_requested {
        log::info(&crate::output::format_launch_directory(child_cwd));
    }

    if let Some(collector) = perf_collector.as_mut() {
        collector.mark_env_setup_complete();
    }

    // ------------------------------------------------------------------
    // Stage 11: Dry-run or continue to execution
    // ------------------------------------------------------------------
    let sp_display_lines = system_prompt::describe_effective(&effective_sp);
    if args.dry_run {
        crate::output::log_dry_run(
            profile,
            &binary_path,
            &child_args,
            repo_requested,
            &env_plan,
            mcp_runtime.as_ref(),
            child_cwd,
            &term,
            sp_display_lines.as_deref(),
        );
        if let Some(collector) = perf_collector.as_mut() {
            collector.set_dry_run();
        }
        return Ok((0, None, None));
    }

    // ------------------------------------------------------------------
    // Stage 12: Switch to child cwd and emit preflight preamble
    // ------------------------------------------------------------------
    exec::switch_process_cwd(child_cwd)?;

    let dispatch_context = HashMap::new();

    emit_preflight_preamble(
        &env_plan,
        mcp_runtime.as_ref(),
        &effective_sp,
        &deferred_warnings,
        &deferred_messages,
        repo_requested,
        silent_requested,
        quiet_requested,
        detail_requested,
        &launch_context,
        opencode_model_source.as_ref(),
        &term,
        verbose,
    );

    // ------------------------------------------------------------------
    // Stage 13: Decide stream mode and deliver prompt
    // ------------------------------------------------------------------
    let (
        use_structured,
        stream_verbosity,
        cli_step_timeout,
        stdout_noise,
        stderr_noise,
        structured_codex_output,
        wire_prompt,
        stdin_seed,
    ) = prepare_stream_and_prompt(
        profile,
        provider,
        &args,
        &mut child_args,
        &prompt_source,
        effective_non_interactive,
        silent_requested,
        quiet_requested,
        &wrapper_span,
    )?;

    profile::require_prompt_present(profile.binary(), effective_non_interactive, &prompt_source)?;

    if tracing::enabled!(tracing::Level::WARN) {
        profile::validate_argv_flags_before_separator(profile.binary(), &child_args);
    }

    // ------------------------------------------------------------------
    // Stage 14: Detect wrapper harness
    // ------------------------------------------------------------------
    let wrapper_harness = detect_wrapper_harness(
        provider,
        profile,
        &prompt_source,
        stdin_seed.as_deref(),
        &env_plan,
        child_cwd,
        &cwd,
    )?;

    if !silent_requested && !quiet_requested {
        use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
        use biscuit_terminal::prelude::TerminalRenderable as _;

        let status = Status::from_prose("Pre-flight checks have passed".to_string())
            .state(StatusState::Success)
            .theme(StatusTheme::Circular);
        crate::log::message(&status.render(&term));
    }

    // ------------------------------------------------------------------
    // Stage 15: Execute the provider
    // ------------------------------------------------------------------
    let (exit_code, stderr_capture) = run_execution_stage(
        wrapper_harness,
        use_structured,
        provider,
        profile,
        &binary_path,
        &child_args,
        &env_plan,
        child_cwd,
        effective_non_interactive,
        &args,
        cli_timeout_duration,
        cli_step_timeout,
        stdout_noise,
        stderr_noise,
        stream_verbosity,
        detail_requested,
        structured_codex_output.as_ref(),
        wire_prompt,
        stdin_seed.as_deref(),
        &env_context,
        &dispatch_context,
        &term,
        &wrapper_span,
        perf_collector,
    )?;

    // ------------------------------------------------------------------
    // Stage 16: Cleanup and return
    // ------------------------------------------------------------------
    exec::cleanup_mcp_injection(mcp_cleanup);

    Ok((exit_code, stderr_capture, opencode_model_source))
}

#[cfg(test)]
mod tests;
