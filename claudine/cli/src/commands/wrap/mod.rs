#![allow(unused_imports)]

pub(crate) mod env;
pub(crate) mod exec;
pub(crate) mod live_semantic_sink;
pub(crate) mod profile;
pub(crate) mod repo_home;
pub(crate) mod section;
pub(crate) mod stream_io;
pub(crate) mod subagent_watchdog;
pub(crate) mod system_prompt;
pub(crate) mod wire_io;

pub(crate) mod composition;
pub(crate) mod selection_ui;
pub(crate) mod sequence;

// New split modules
pub(crate) mod flags;
pub(crate) mod harness_orch;
pub(crate) mod inline;
pub(crate) mod overlay;
pub(crate) mod policy;
pub(crate) mod prompt_source;
pub(crate) mod resume;

// Re-exports from split modules
pub use flags::WrapperArgs;
pub(crate) use flags::{
    extract_wrapper_flags_from_passthrough, has_explicit_native_output_request,
    reject_retired_composition_flags,
};
pub(crate) use harness_orch::{
    AttemptLaunch, CachedHarnessLoopContext, HarnessPromptMode, HarnessPromptState,
    MaterializedHarnessPrompt, build_harness_launch, build_harness_shell_options,
    build_harness_shell_options_with_cache, execute_harness_attempt, find_wrapper_harness_source,
    harness_policy_root, harness_prompt_mode_label, materialize_harness_prompt,
    materialize_passthrough_harness_seed, materialized_harness_prompt_from_prepared,
    run_harness_loop,
};
pub(crate) use inline::{
    extract_tags_from_prompt, report_inline_agent_status, strip_prompt_tags_for_provider,
    try_inline_closure,
};
pub(crate) use overlay::{frontmatter_map_to_value, merge_frontmatter_overlay};
pub(crate) use policy::{
    StreamSummaryContext, StructuredCodexOutput, StructuredSummaryDetails,
    WrapperHarnessPermissionProbe, build_structured_plumbing, emit_stream_summary,
    emit_stream_summary_with_context, format_summary_prose, format_verbose_summary_details_prose,
};
pub(crate) use prompt_source::{maybe_edit_prompt_source, maybe_edit_prompt_source_with};
pub(crate) use resume::{
    NextAttemptPlan, append_resume_passthrough_args, apply_next_attempt_plan,
    build_next_attempt_plan, normalize_resume_args, try_resolve_handler,
};

use biscuit_terminal::terminal::Terminal;
use claudine::composition::InstalledProviderSnapshot;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use profile::{OutputFormat, WrapperProfile};
use sniff::programs::InstalledAiClients;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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

/// Shared startup detection results for the direct wrap path.
///
/// Populated by a single `sniff::detect_with_plan` call and consumed by
/// three independent downstream structures that would otherwise each
/// trigger their own full repo walk.
pub(crate) struct WrapStartupDetection {
    pub(crate) env_context: EnvironmentContext,
    pub(crate) launch_context: claudine::system_prompt::LaunchContext,
    pub(crate) launch_workspace: claudine::composition::LaunchWorkspaceContext,
}

/// Run one sniff-based filesystem scan and build every startup context
/// the direct wrap path needs from the shared result.
///
/// On a cold filesystem cache in a large monorepo the previous pipeline
/// walked the tree 3-5 times (once in `detect_environment_fast`, once in
/// `LaunchContext::from_cwd`, and twice inside `build_child_env`). This
/// helper collapses that into a single scan and then builds the three
/// consumer contexts from borrowed data.
pub(crate) fn detect_wrap_startup(cwd: &Path) -> Result<WrapStartupDetection> {
    use sniff::request::*;

    let plan = DetectionPlan::new()
        .base_dir(cwd.to_path_buf())
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::summary())
                .repo(RepoRequest::structure())
                .without_file_inventory()
                .without_docs()
                .without_formatting(),
        );

    let result = sniff::detect_with_plan(plan)
        .map_err(|e| eyre!("startup detection failed for '{}': {e}", cwd.display()))?;

    let launch_context = claudine::system_prompt::LaunchContext::from_sniff_result(&result, cwd);

    let (git_root, repo) = result
        .filesystem
        .as_ref()
        .map(|f| {
            (
                f.git.as_ref().map(|g| g.repo_root.clone()),
                f.repo.as_ref().cloned(),
            )
        })
        .unwrap_or((None, None));

    let launch_workspace =
        env::launch_workspace_context_from_repo_info(cwd, git_root.as_deref(), repo.as_ref());

    let env_context = claudine::events::environment_context_from_sniff_result(result);

    Ok(WrapStartupDetection {
        env_context,
        launch_context,
        launch_workspace,
    })
}

fn fallback_wrap_startup(cwd: &Path) -> WrapStartupDetection {
    WrapStartupDetection {
        env_context: EnvironmentContext::default(),
        launch_context: claudine::system_prompt::LaunchContext {
            cwd: cwd.to_path_buf(),
            repo_root: None,
            package_area_root: None,
            package_root: None,
        },
        launch_workspace: claudine::composition::LaunchWorkspaceContext {
            launch_cwd: cwd.to_path_buf(),
            repo_root: None,
            child_cwd: cwd.to_path_buf(),
            package_context: None,
            warnings: Vec::new(),
        },
    }
}

pub(crate) fn switch_process_cwd(child_cwd: &Path) -> Result<()> {
    let current = std::env::current_dir()?;
    if current != child_cwd {
        std::env::set_current_dir(child_cwd)?;
    }
    Ok(())
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
        .map_err(|e| eyre!("failed to load MCP catalog for bootstrap: {e}"))?;
    let mut state = McpProviderStateStore::load()
        .map_err(|e| eyre!("failed to load MCP provider-state for bootstrap: {e}"))?;
    let mut importer = McpImporter::new(&mut catalog, &mut state);
    let _ = importer.import_all(repo_root);
    catalog
        .save()
        .map_err(|e| eyre!("failed to save bootstrapped MCP catalog: {e}"))?;
    state
        .save()
        .map_err(|e| eyre!("failed to save bootstrapped MCP provider-state: {e}"))?;

    if !defaults_path().exists() {
        save_user_defaults(&McpDefaults::default())
            .map_err(|e| eyre!("failed to create bootstrapped MCP defaults: {e}"))?;
    }
    if let Some(repo_root) = repo_root
        && !repo_defaults_path(repo_root).exists()
    {
        save_repo_defaults(repo_root, &McpDefaults::default())
            .map_err(|e| eyre!("failed to create bootstrapped repo MCP defaults: {e}"))?;
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

    let wrapper_start = std::time::Instant::now();
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
        let total = wrapper_start.elapsed();
        let report = collector.into_report(total);
        eprint!("{}", crate::perf::render_perf_report(&report));
    }

    std::process::exit(code);
}

fn run_provider_wrapper_inner(
    provider: Provider,
    args: WrapperArgs,
    verbose: u8,
    mut perf_collector: Option<&mut crate::perf::CommandPerfCollector>,
) -> Result<(i32, Option<String>, Option<profile::OpenCodeModelSource>)> {
    let profile = profile::profile_for_provider(provider).ok_or_else(|| {
        eyre!(
            "'{}' cannot be wrapped (it is a VS Code extension)",
            provider
        )
    })?;

    // SAFETY: this runs at subcommand entry on the main task before any
    // child threads, sub-renders, or hooks have been spawned. No concurrent
    // env reads exist at this point, so mutating the process env upholds
    // Rust 2024's `std::env::set_var` safety contract. Setting AGENT
    // here lets `{{env.AGENT}}` resolve in any prompt rendered by the
    // parent (system prompt, dispatch templates) before the child launch.
    unsafe {
        std::env::set_var("AGENT", profile.agent_env());
    }

    let cwd = std::env::current_dir()?;

    let binary_path = info_span!(
        "wrapper_binary_resolution",
        provider = %provider,
    )
    .in_scope(|| resolve_binary_path_direct(profile, None))?;

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

    // One sniff-based scan produces the raw git + repo-structure data that
    // the rest of startup needs. The result is consumed by three separate
    // consumers (EnvironmentContext, LaunchContext, LaunchWorkspaceContext)
    // without ever walking the filesystem again — this avoids the 3-5
    // redundant tree walks the earlier pipeline performed.
    let startup = match detect_wrap_startup(&cwd) {
        Ok(startup) => startup,
        Err(error) => {
            if repo_requested {
                return Err(eyre!(
                    "--repo requires startup repo detection, but startup detection failed: {error}"
                ));
            }
            deferred_warnings.push(format!(
                "startup detection failed; continuing without repo/package context: {error}"
            ));
            fallback_wrap_startup(&cwd)
        }
    };
    let env_context = startup.env_context;
    let launch_context = startup.launch_context;
    let launch_workspace = startup.launch_workspace;

    // In the direct-wrap path the child inherits stdin automatically —
    // we don't need to detect or seed it. Pass false so that a non-tty
    // test environment (or any shell running without an attached terminal)
    // doesn't accidentally classify the session as non-interactive.
    // Composition (Task 14) is where InheritStdin / stdin_seed matters.
    let has_piped_stdin = false;

    // Extract the prompt up-front into a typed PromptSource, leaving
    // child_args free of any prompt characters. Downstream apply_*
    // methods see clean args; prompt_delivery is the only code path
    // that places the prompt back in.
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

    // Early check: --timeout + --interactive is always an error
    if args.timeout.is_some() && interactive_requested {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }

    // Early check: --step-timeout + --interactive is always an error
    if args.step_timeout.is_some() && interactive_requested {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }

    // clap catches direct `--edit` + `--interactive` conflicts, but once a
    // prompt token starts passthrough capture either flag can arrive from the
    // fallback extractor instead. Keep the merged behavior consistent.
    if edit_requested && interactive_requested {
        return Err(eyre!("--edit cannot be used with --interactive"));
    }

    // Early check: --timeout requires non-interactive mode
    if args.timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }

    // Early check: --step-timeout requires non-interactive mode
    if args.step_timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--step-timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }

    // Parse `--timeout DURATION` and `--step-timeout DURATION` once via the
    // same parser frontmatter uses so CLI and frontmatter errors share one
    // grammar. The `Path` argument is only used to decorate `HarnessError`;
    // the CLI flag has no source file, so we use a synthetic label.
    let cli_timeout_duration: Option<std::time::Duration> = match args.timeout.as_deref() {
        Some(raw) => Some(
            claudine::harness::parse_timeout(raw, std::path::Path::new("<--timeout>"))
                .map_err(|e| eyre!("invalid --timeout value: {e}"))?,
        ),
        None => None,
    };
    if cli_timeout_duration == Some(std::time::Duration::from_secs(0)) {
        return Err(eyre!(
            "--timeout: zero duration is not accepted; omit the flag to disable the wall-clock timeout"
        ));
    }

    let cli_step_timeout_duration: Option<std::time::Duration> = match args.step_timeout.as_deref()
    {
        Some(raw) => Some(
            claudine::harness::parse_timeout(raw, std::path::Path::new("<--step-timeout>"))
                .map_err(|e| eyre!("invalid --step-timeout value: {e}"))?,
        ),
        None => None,
    };
    if cli_step_timeout_duration == Some(std::time::Duration::from_secs(0)) {
        return Err(eyre!(
            "--step-timeout: zero duration is not accepted; omit the flag or set the env var to 0s to disable the step timeout"
        ));
    }

    // The effective interactivity state is determined solely by the explicit flag.
    let effective_non_interactive = non_interactive_requested;
    let term = wrap_terminal_for_mode(effective_non_interactive);

    profile.reject_direct_yolo(&child_args)?;
    flags::reject_retired_composition_flags(&child_args)?;

    if yolo_requested
        && let Some(warn) = profile.apply_yolo_for_mode(
            &mut child_args,
            &mut env_overrides,
            !non_interactive_requested,
        )?
    {
        deferred_warnings.push(warn);
        // A returned warning means yolo was NOT actually applied for this
        // invocation (e.g. OpenCode interactive mode), so the summary and
        // header badge should reflect the disabled state.
        yolo_enabled = false;
    }
    if yolo_requested && !profile.has_supported_yolo() {
        yolo_enabled = false;
    }

    profile.apply_entrypoint(&mut child_args, non_interactive_requested);

    if non_interactive_requested {
        profile.apply_non_interactive_flags(&mut child_args)?;
    }

    // OpenCode model resolution (replaces apply_non_interactive_defaults +
    // validate_non_interactive_requirements).
    let opencode_model_source: Option<profile::OpenCodeModelSource> =
        if provider == Provider::OpenCode {
            let has_model = env_overrides.iter().any(|(k, _)| k == "MODEL");
            match profile::apply_opencode_model_resolution(
                &mut child_args,
                &mut |k, v| env_overrides.push((k, v)),
                has_model,
                args.model.as_deref(),
                non_interactive_requested,
                &profile::OpenCodeEnvSnapshot::from_system(),
            ) {
                Ok(source) => source,
                Err(_) => {
                    let term = wrap_terminal();
                    let report =
                        crate::output::error_report::AgentErrorReport::no_model_provided(provider);
                    report.render(&term);
                    std::process::exit(1);
                }
            }
        } else {
            None
        };

    // Universal --model flag (non-OpenCode providers, and OpenCode when
    // the user passed --model explicitly but we already handled it above).
    if provider != Provider::OpenCode {
        if let Some(ref model) = args.model
            && let Some(warn) = profile.apply_model(&mut child_args, &mut env_overrides, model)
        {
            deferred_warnings.push(warn);
        }
    } else if let Some(ref model) = args.model
        && !non_interactive_requested
    {
        // Interactive OpenCode with --model: use the standard apply_model path.
        if let Some(warn) = profile.apply_model(&mut child_args, &mut env_overrides, model) {
            deferred_warnings.push(warn);
        }
    }

    // Non-OpenCode providers still use the trait-based validation.
    if provider != Provider::OpenCode && non_interactive_requested {
        profile.validate_non_interactive_requirements(&child_args)?;
    }

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

    let sp_args = claudine::system_prompt::SystemPromptArgs {
        append_file: args.append_system_prompt.clone(),
        replace_file: args.replace_system_prompt.clone(),
    };
    let effective_sp = claudine::system_prompt::resolve_and_prepare_for_session(
        &sp_args,
        &launch_context,
        non_interactive_requested,
    )?;

    let mut sp_artifacts: Vec<system_prompt::SystemPromptArtifact> = Vec::new();

    match &effective_sp {
        claudine::system_prompt::EffectiveSystemPrompt::None
        | claudine::system_prompt::EffectiveSystemPrompt::Disabled { .. } => {}
        claudine::system_prompt::EffectiveSystemPrompt::Ready(prepared) => {
            let application =
                profile.apply_system_prompt(prepared, !non_interactive_requested, &cwd)?;
            child_args.extend(application.args);
            env_overrides.extend(
                application
                    .env
                    .into_iter()
                    .map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into())),
            );
            sp_artifacts = application.artifacts;
            for warn in application.warnings {
                deferred_warnings.push(warn);
            }
        }
    }
    let _ = &sp_artifacts;

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
    )?;

    if !silent_requested && !quiet_requested {
        use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
        use biscuit_terminal::prelude::Renderable as _;

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

    let mut mcp_runtime = None;
    let mut mcp_cleanup: Option<(
        Box<dyn claudine::mcp::inject::McpInjector>,
        claudine::mcp::inject::InjectionResult,
    )> = None;

    // MCP session composition
    if args.mcp || !args.mcp_use.is_empty() {
        use claudine::mcp::catalog::McpCatalogStore;
        use claudine::mcp::inject::injector_for_provider;
        use claudine::mcp::session::{compute_session_set, lex_tags};

        let repo_root_ref = env_plan.repo_root.as_deref();
        if bootstrap_mcp_state(repo_root_ref)? {
            deferred_messages.push(
                "MCP bootstrap: created Claudine MCP state from discoverable provider configs."
                    .to_string(),
            );
        }
        let catalog =
            McpCatalogStore::load().map_err(|e| eyre!("failed to load MCP catalog: {e}"))?;
        let (cleaned_prompt, prompt_tags) =
            inline::extract_tags_from_prompt(prompt_source.as_inline(), lex_tags);
        if let Some(ref cleaned) = cleaned_prompt {
            prompt_source = profile::PromptSource::Inline(cleaned.clone());
        }
        let prompt_is_interactive =
            std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
        let mut session = compute_session_set(
            &catalog,
            repo_root_ref,
            &args.mcp_use,
            &prompt_tags,
            |tag, _tier, candidates| {
                if args.strict || non_interactive_requested || !prompt_is_interactive {
                    return None;
                }
                inquire::Select::new(
                    &format!("`#{tag}` matched multiple MCP servers. Choose one:"),
                    candidates.to_vec(),
                )
                .prompt()
                .ok()
            },
        )
        .map_err(|e| eyre!("MCP session error: {e}"))?;
        session.cleaned_prompt = cleaned_prompt.clone();

        for warning in &session.warnings {
            deferred_warnings.push(warning.clone());
        }
        if !session.missing_tags.is_empty() {
            if args.strict {
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
            for tag in &session.missing_tags {
                deferred_warnings.push(format!("tag `#{tag}` was not found in the MCP catalog"));
            }
        }
        if !session.ambiguous_tags.is_empty() {
            if args.strict || non_interactive_requested {
                let message = session
                    .ambiguous_tags
                    .iter()
                    .map(|tag| format!("#{} -> {}", tag.tag, tag.candidates.join(", ")))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(eyre!("ambiguous MCP tag(s): {message}"));
            }
            // Interactive non-strict: warn and drop ambiguous tags
            for tag in &session.ambiguous_tags {
                deferred_warnings.push(format!(
                    "tag `#{0}` is ambiguous ({1}); dropped from session",
                    tag.tag,
                    tag.candidates.join(", ")
                ));
            }
            session.ambiguous_tags.clear();
        }

        let mut runtime = McpRuntimeInfo {
            servers: session
                .servers
                .iter()
                .map(|server| server.id.clone())
                .collect(),
            default_servers: session.default_servers.clone(),
            explicit_servers: session.explicit_servers.clone(),
            tag_servers: session.tag_servers.clone(),
            resolved_tags: session
                .resolved_tags
                .iter()
                .map(|tag| format!("#{} -> {} ({:?})", tag.tag, tag.resolved_to, tag.match_tier))
                .collect(),
            missing_tags: session
                .missing_tags
                .iter()
                .map(|tag| format!("#{tag}"))
                .collect(),
            ambiguous_tags: session
                .ambiguous_tags
                .iter()
                .map(|tag| format!("#{} -> {}", tag.tag, tag.candidates.join(", ")))
                .collect(),
            cleaned_prompt: session.cleaned_prompt.clone(),
            ..McpRuntimeInfo::default()
        };

        if let Some(injector) = injector_for_provider(provider) {
            if !session.servers.is_empty() {
                let shadow = env_plan.shadow_home_path.as_deref();
                // Injector works with String env; bridge to OsString env plan
                let mut string_env = std::collections::HashMap::new();
                let result = injector
                    .inject(&session.servers, &mut string_env, shadow)
                    .map_err(|e| eyre!("MCP injection failed: {e}"))?;

                // Merge injected env vars into the OsString env plan
                for (k, v) in string_env {
                    env_plan.env.insert(k.into(), v.into());
                }

                for arg in &result.extra_args {
                    child_args.push(arg.clone());
                }

                runtime.env_vars_set = result.env_vars_set.clone();
                runtime.temp_files = result.temp_files.clone();
                runtime.extra_args = result.extra_args.clone();

                mcp_cleanup = Some((injector, result));
            }
        } else {
            return Err(eyre!(
                "provider {} does not support runtime MCP injection.\n\
                 Use `claudine mcp export {} --apply` to write servers to its native config instead.",
                provider,
                provider.as_slug()
            ));
        }

        deferred_messages.push(if runtime.servers.is_empty() {
            "MCP: no active servers".to_string()
        } else {
            format!("MCP: {}", runtime.servers.join(", "))
        });
        if !runtime.resolved_tags.is_empty() {
            deferred_messages.push(format!("MCP tags: {}", runtime.resolved_tags.join(", ")));
        }
        mcp_runtime = Some(runtime);
    }

    let child_cwd = env_plan.child_cwd.as_path();

    if effective_non_interactive && !silent_requested {
        log::info(&crate::output::format_launch_directory(child_cwd));
    }

    if let Some(collector) = perf_collector.as_mut() {
        collector.mark_env_setup_complete();
    }

    // --dry-run: print what would be executed and exit
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

    switch_process_cwd(child_cwd)?;

    let dispatch_context = HashMap::new();

    // Output verbosity: --silent suppresses everything, --quiet hides env/info preamble
    // but still shows the system prompt when one is active.
    if !silent_requested {
        if !quiet_requested {
            crate::output::log_wrapper_env_details(&env_plan, mcp_runtime.as_ref(), &term, verbose);

            if let Some(info_message) =
                crate::output::removed_env_info_message(&env_plan.removed, &term)
            {
                log::message(&info_message);
            }
            if repo_requested {
                log::message(&crate::output::repo_flag_info_message(
                    &term,
                    env_plan.shadow_home_path.as_deref(),
                ));
            }
            for warning in &env_plan.warnings {
                log::message(&crate::output::post_env_warning_message(warning, &term));
            }
            for warning in &deferred_warnings {
                log::message(&crate::output::post_env_warning_message(warning, &term));
            }
            for message in &deferred_messages {
                log::message(&crate::output::post_env_message(message, &term));
            }

            if let Some(ref source) = opencode_model_source {
                use biscuit_terminal::components::status::Status;
                use biscuit_terminal::prelude::Renderable as _;
                let status = Status::from_prose(source.status_markup());
                log::message(&status.render(&term));
            }
        }

        crate::output::log_system_prompt(
            &effective_sp,
            detail_requested,
            silent_requested,
            quiet_requested,
            &term,
        );

        // Blank line to separate preamble from execution output. Keep it aligned with
        // whichever preamble blocks were actually emitted.
        if !quiet_requested
            || matches!(
                effective_sp,
                claudine::system_prompt::EffectiveSystemPrompt::Ready(_)
            )
        {
            log::message("");
        }
    }

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

    // Decide whether to use internal structured stream parsing.
    // Conditions: provider supports it, non-interactive, no explicit output format.
    let use_structured = profile.supports_structured_stream()
        && effective_non_interactive
        && args.output.is_none()
        && !flags::has_explicit_native_output_request(provider, &child_args);
    wrapper_span.record("structured_mode", use_structured);
    let stream_verbosity = structured_verbosity(silent_requested, quiet_requested);

    // `step_timeout` is only enforceable in structured-stream mode because
    // it gates on `LiveMetrics::last_event_at`. Warn and drop it otherwise so
    // the downstream exec path never has to re-check.
    let cli_step_timeout = if !use_structured && args.step_timeout.is_some() {
        if !silent_requested {
            log::warn(
                "--step-timeout is only enforced in structured-stream mode; \
                 this run does not qualify, so the flag will be ignored",
            );
        }
        None
    } else {
        args.step_timeout.clone()
    };

    if use_structured {
        profile.apply_structured_stream(&mut child_args);
    }

    let structured_codex_output = if use_structured && provider == Provider::Codex {
        Some(StructuredCodexOutput::prepare(&mut child_args))
    } else {
        None
    };

    let mut wire_prompt: Option<String> = None;
    let stdin_seed: Option<String> = if let Some(prompt) = prompt_source.as_inline() {
        let delivery = profile.prompt_delivery(&child_args, prompt, effective_non_interactive)?;
        if let Some(body) = delivery.as_wire_rpc() {
            wire_prompt = Some(body.to_string());
        }
        delivery.apply_to(&mut child_args)
    } else {
        None
    };

    profile::require_prompt_present(profile.binary(), effective_non_interactive, &prompt_source)?;

    if tracing::enabled!(tracing::Level::WARN) {
        profile::validate_argv_flags_before_separator(profile.binary(), &child_args);
    }

    let wrapper_harness = {
        let base_prompt = prompt_source
            .as_inline()
            .map(|s| s.to_string())
            .or_else(|| stdin_seed.clone());
        let harness_source = base_prompt.as_ref().and_then(|_| {
            harness_orch::find_wrapper_harness_source(provider, env_plan.repo_root.as_deref(), &cwd)
        });

        if let (Some(base_prompt), Some(source_path)) = (base_prompt, harness_source) {
            let seed = harness_orch::materialize_passthrough_harness_seed(
                &source_path,
                base_prompt.clone(),
            )?;
            let harness_enabled = claudine::harness::has_harness_properties(&seed.frontmatter);
            if harness_enabled {
                let resolve_ctx = claudine::harness::HarnessResolutionContext {
                    source_path: &source_path,
                    repo_root: env_plan.repo_root.as_deref(),
                };
                let shell_options = harness_orch::build_harness_shell_options(
                    &source_path,
                    env_plan.repo_root.as_deref(),
                );
                let plan = claudine::harness::parse_harness_plan(
                    &seed.frontmatter,
                    &source_path,
                    &resolve_ctx,
                )
                .map_err(|e| eyre!("{e}"))?;

                // Pre-flight harness shell commands
                let _harness_preflight = claudine::composition::resolve_shell_approvals(
                    None,
                    None,
                    Some(&plan),
                    &shell_options,
                )
                .map_err(|e| eyre!("{e}"))?;

                drop(plan);

                Some((source_path, base_prompt, seed, shell_options))
            } else {
                None
            }
        } else {
            None
        }
    };

    if !silent_requested && !quiet_requested {
        use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
        use biscuit_terminal::prelude::Renderable as _;

        let status = Status::from_prose("Pre-flight checks have passed".to_string())
            .state(StatusState::Success)
            .theme(StatusTheme::Circular);
        crate::log::message(&status.render(&term));
    }

    // Execute the provider. Composition and harness execution are handled by
    // `claudine compose` / `claudine inline-compose` through the wrapper-grade
    // composition executor; the wrapper path handles plain prompt passthrough.
    let (exit_code, stderr_capture) = if let Some((
        source_path,
        base_prompt,
        initial_materialized,
        shell_options,
    )) = wrapper_harness
    {
        let mut prompt_state = HarnessPromptState {
            mode: HarnessPromptMode::Passthrough,
            original_ref: source_path.display().to_string(),
            source_path,
            base_prompt: Some(base_prompt),
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
        };

        let mut harness_base_args = child_args.clone();
        if !use_structured {
            profile.prepare_captured_output(&mut harness_base_args);
        }

        let source_path_for_lifecycle = prompt_state.source_path.clone();
        let default_lifecycle = claudine::composition::LifecycleConfig::default();
        let default_lifecycle_settings = claudine::events::GlobalSettings::default();
        let default_lifecycle_messaging = claudine::messaging::RuntimeMessagingSettings {
            user: None,
            repo: None,
        };
        let default_lifecycle_ctx = claudine::composition::LifecycleRuntimeContext {
            settings: &default_lifecycle_settings,
            messaging: &default_lifecycle_messaging,
            term: &term,
            source_path: &source_path_for_lifecycle,
            repo_root: env_plan.repo_root.as_deref(),
        };
        let default_lifecycle_emitter = claudine::composition::DefaultLifecycleEmitter;

        let (harness_code, harness_perf) = harness_orch::run_harness_loop(
            provider,
            profile,
            binary_path.as_path(),
            child_cwd,
            effective_non_interactive,
            args.timeout.clone(),
            cli_step_timeout.clone(),
            &harness_base_args,
            &env_plan.env,
            &mut prompt_state,
            env_plan.repo_root.as_deref(),
            shell_options,
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            !silent_requested,
            stream_verbosity,
            detail_requested,
            &env_context,
            &dispatch_context,
            Some(initial_materialized),
            &term,
            &default_lifecycle,
            &default_lifecycle_ctx,
            &default_lifecycle_emitter,
            true,
        )?;
        if let (Some(collector), Some(perf)) = (perf_collector.as_mut(), harness_perf) {
            collector.set_agent_perf(perf);
        }
        (harness_code, None)
    } else if use_structured {
        let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
        let parser_config = claudine::stream::ParserConfig {
            model: args.model.clone(),
        };
        let sink = live_semantic_sink::LiveSemanticSink::with_default_wiring(
            provider,
            env_context.clone(),
            child_cwd,
            stream_verbosity,
            summary_details.clone(),
        )
        .with_context_extra(dispatch_context.clone());
        let live_metrics = sink.live_metrics();
        let stream_output = sink.stream_output();
        let watchdog_state = Some(sink.watchdog_state());
        // Snapshot the sink's section-stream handle before the sink is
        // moved into the parser closure. Post-stream trailer and
        // Codex-final-stdout emission uses this handle so every section
        // transition shares the same tracker state.
        let section_stream = sink.section_stream();
        let (build_parser, stderr_bridge) =
            policy::build_structured_plumbing(provider, sink, parser_config);
        let mut _spawned = false;
        let stream_result = if let Some(wire_prompt) = wire_prompt.clone() {
            let runtime_context =
                match claudine::dispatch::DispatchRuntimeContext::load_for_env(&env_context) {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        tracing::warn!(%provider, "failed to preload wire runtime config: {error}");
                        claudine::dispatch::DispatchRuntimeContext::default()
                    }
                };
            let _ = stderr_bridge;
            wire_io::run_kimi_wire_session(
                wire_io::WireSessionConfig {
                    binary: binary_path.as_path(),
                    args: &child_args,
                    env: &env_plan.env,
                    cwd: child_cwd,
                    prompt: wire_prompt,
                    timeout: args.timeout.as_ref().and_then(|raw| {
                        claudine::harness::parse_timeout(raw, std::path::Path::new("<cli>")).ok()
                    }),
                    client_name: env!("CARGO_PKG_NAME"),
                    client_version: env!("CARGO_PKG_VERSION"),
                    capabilities: wire_io::WireClientCapabilities::default_for_claudine(),
                    env_context: env_context.clone(),
                },
                wire_io::WireSessionWiring {
                    build_parser,
                    stream_output,
                    live_metrics,
                    runtime_context,
                },
                &mut _spawned,
            )?
        } else {
            let timeout_config = composition::resolve_timeouts(
                args.timeout.clone(),
                None,
                cli_step_timeout.clone(),
                None,
            );
            exec::run_child_stream_semantic(
                binary_path.as_path(),
                &child_args,
                &env_plan.env,
                child_cwd,
                timeout_config,
                stderr_noise,
                profile.suppress_structured_stderr_on_success(),
                stream_verbosity != Verbosity::Silent,
                stdin_seed.as_deref(),
                build_parser,
                &mut _spawned,
                live_metrics,
                stream_output,
                stderr_bridge,
                None,
                watchdog_state,
                Some(section_stream.tracker()),
            )?
        };
        let mut summary = stream_result.data;
        let api_duration_ms = summary.duration_ms;
        if let Some(collector) = perf_collector.as_mut() {
            collector.set_agent_perf(stream_result.telemetry.into_agent_perf(api_duration_ms));
        }
        if let Some(ref sid) = summary.session_id {
            wrapper_span.record("session_id", tracing::field::display(sid));
        }
        if let Some(codex_output) = structured_codex_output.as_ref() {
            codex_output.apply_to_summary(&mut summary);
        }
        if provider == Provider::Codex && !summary.assistant_text.is_empty() {
            section_stream.enter_final_stdout();
            let text = &summary.assistant_text;
            if std::io::stdout().is_terminal() {
                let rendered = crate::output::render_assistant_markdown(text, &term);
                std::io::stdout().write_all(rendered.as_bytes())?;
                if !rendered.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            } else {
                std::io::stdout().write_all(text.as_bytes())?;
                if !text.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            }
            std::io::stdout().flush()?;
        }

        policy::emit_stream_summary(
            &summary,
            profile,
            &env_context,
            stream_verbosity,
            detail_requested,
            &summary_details.lock().unwrap().clone(),
            Some(&section_stream),
        );

        let stderr_text = summary.stderr_text.clone();
        (summary.exit_code, stderr_text)
    } else {
        // Legacy path: forward I/O to terminal
        let mut _spawned = false;
        let result = exec::run_child(
            binary_path.as_path(),
            &child_args,
            &env_plan.env,
            child_cwd,
            cli_timeout_duration.map(|d| d.as_secs()),
            exec::ChildIoOptions {
                stdout_noise_prefixes: stdout_noise,
                stderr_noise_prefixes: stderr_noise,
                stdin_seed: stdin_seed.as_deref(),
            },
            &mut _spawned,
        )?;
        if let Some(collector) = perf_collector.as_mut() {
            collector.set_agent_perf(result.telemetry.into_agent_perf(None));
        }
        (result.data, None)
    };

    // MCP injector cleanup: remove temp files written during injection
    if let Some((injector, injection_result)) = mcp_cleanup
        && let Err(e) = injector.cleanup(&injection_result)
    {
        tracing::warn!("MCP injector cleanup failed: {e}");
    }

    Ok((exit_code, stderr_capture, opencode_model_source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn missing_binary_preflight_has_actionable_message() {
        let clients = InstalledAiClients::default();
        let profile = profile::profile_for_provider(Provider::Codex).unwrap();

        let error = resolve_binary_path(profile, &clients).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("cannot run wrapped Codex session"));
        assert!(message.contains("docs:"));
    }

    #[test]
    fn package_name_display_shows_resolved_package_and_area() {
        let env_plan = env::EnvPlan {
            env: HashMap::new(),
            removed: Vec::new(),
            included: Vec::new(),
            added: Vec::new(),
            repo_root: None,
            child_cwd: PathBuf::from("/tmp"),
            package_context: Some(claudine::composition::PackageContext {
                package_area: "claudine".to_string(),
                package: Some("claudine-cli".to_string()),
                candidates: vec!["claudine-cli".to_string()],
            }),
            warnings: Vec::new(),
            shadow_home_path: None,
        };

        let rendered = crate::output::package_name_display(&env_plan).unwrap();
        assert!(rendered.contains("claudine-cli"));
        assert!(rendered.contains("area: claudine"));
    }

    #[test]
    fn package_name_display_is_hidden_when_package_is_ambiguous() {
        let env_plan = env::EnvPlan {
            env: HashMap::new(),
            removed: Vec::new(),
            included: Vec::new(),
            added: Vec::new(),
            repo_root: None,
            child_cwd: PathBuf::from("/tmp"),
            package_context: Some(claudine::composition::PackageContext {
                package_area: "claudine".to_string(),
                package: None,
                candidates: vec!["claudine".to_string(), "claudine-cli".to_string()],
            }),
            warnings: Vec::new(),
            shadow_home_path: None,
        };

        assert!(crate::output::package_name_display(&env_plan).is_none());
    }
}
