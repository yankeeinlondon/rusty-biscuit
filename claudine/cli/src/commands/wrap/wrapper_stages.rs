use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_terminal::terminal::Terminal;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};

use super::flags::WrapperArgs;
use super::profile::{self, WrapperProfile};
use super::{
    McpRuntimeInfo, StructuredCodexOutput, env, exec, flags, harness_orch, policy,
    structured_verbosity, system_prompt, wrap_terminal, wrapper_exec,
};
use crate::log;

pub(crate) fn validate_timeout_constraints(
    args: &WrapperArgs,
    interactive_requested: bool,
    non_interactive_requested: bool,
    edit_requested: bool,
) -> Result<()> {
    if args.timeout.is_some() && interactive_requested {
        return Err(eyre!("--timeout cannot be used with --interactive mode"));
    }
    if args.step_timeout.is_some() && interactive_requested {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    if edit_requested && interactive_requested {
        return Err(eyre!("--edit cannot be used with --interactive"));
    }
    if args.timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }
    if args.step_timeout.is_some() && !non_interactive_requested {
        return Err(eyre!(
            "--step-timeout can only be used in non-interactive mode \
             (provide a prompt or use a composition switch)"
        ));
    }
    Ok(())
}

pub(crate) fn parse_cli_timeouts(args: &WrapperArgs) -> Result<Option<std::time::Duration>> {
    let cli_timeout_duration: Option<std::time::Duration> = match args.timeout.as_deref() {
        Some(raw) => Some(
            claudine::harness::parse_timeout(raw, Path::new("<--timeout>"))
                .map_err(|e| eyre!("invalid --timeout value: {e}"))?,
        ),
        None => None,
    };
    if cli_timeout_duration == Some(std::time::Duration::from_secs(0)) {
        return Err(eyre!(
            "--timeout: zero duration is not accepted; omit the flag to disable the wall-clock timeout"
        ));
    }

    if let Some(raw) = args.step_timeout.as_deref() {
        let parsed = claudine::harness::parse_timeout(raw, Path::new("<--step-timeout>"))
            .map_err(|e| eyre!("invalid --step-timeout value: {e}"))?;
        if parsed == std::time::Duration::from_secs(0) {
            return Err(eyre!(
                "--step-timeout: zero duration is not accepted; omit the flag or set the env var to 0s to disable the step timeout"
            ));
        }
    }

    Ok(cli_timeout_duration)
}

pub(crate) fn resolve_opencode_model(
    provider: Provider,
    _profile: &dyn WrapperProfile,
    child_args: &mut Vec<String>,
    env_overrides: &mut Vec<(String, String)>,
    model: Option<&str>,
    non_interactive_requested: bool,
) -> Result<Option<profile::OpenCodeModelSource>> {
    let has_model = env_overrides.iter().any(|(k, _)| k == "MODEL");
    match profile::apply_opencode_model_resolution(
        child_args,
        &mut |k, v| env_overrides.push((k, v)),
        has_model,
        model,
        non_interactive_requested,
        &profile::OpenCodeEnvSnapshot::from_system(),
    ) {
        Ok(source) => Ok(source),
        Err(_) => {
            let term = wrap_terminal();
            let report = crate::output::error_report::AgentErrorReport::no_model_provided(provider);
            report.render(&term);
            std::process::exit(1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_and_apply_system_prompt(
    profile: &dyn WrapperProfile,
    args: &WrapperArgs,
    launch_context: &claudine::system_prompt::LaunchContext,
    launch_workspace: &claudine::composition::LaunchWorkspaceContext,
    cwd: &Path,
    non_interactive_requested: bool,
    child_args: &mut Vec<String>,
    env_overrides: &mut Vec<(String, String)>,
    deferred_warnings: &mut Vec<String>,
) -> Result<(
    claudine::system_prompt::ResolvedSystemPrompt,
    Vec<system_prompt::SystemPromptArtifact>,
)> {
    let sp_args = claudine::system_prompt::SystemPromptArgs {
        append_file: args.append_system_prompt.clone(),
        replace_file: args.replace_system_prompt.clone(),
    };
    let mut launch_context = launch_context.clone();
    launch_context.agent = Some(profile.agent_env().to_string());
    let effective_sp = claudine::system_prompt::resolve_and_prepare_for_session(
        &sp_args,
        &launch_context,
        non_interactive_requested,
    )?;

    let mut sp_artifacts: Vec<system_prompt::SystemPromptArtifact> = Vec::new();

    let scoped_tmp = system_prompt::scoped_tmp_dir(launch_workspace);
    system_prompt::maybe_gitignore_claudine_tmp(
        launch_workspace
            .repo_root
            .as_deref()
            .unwrap_or(&launch_workspace.launch_cwd),
    );

    match &effective_sp {
        claudine::system_prompt::ResolvedSystemPrompt::None
        | claudine::system_prompt::ResolvedSystemPrompt::Disabled { .. } => {}
        claudine::system_prompt::ResolvedSystemPrompt::Ready(prepared) => {
            let application = profile.apply_system_prompt(
                prepared,
                !non_interactive_requested,
                cwd,
                &scoped_tmp,
            )?;
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

    Ok((effective_sp, sp_artifacts))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_preflight_preamble(
    env_plan: &env::EnvPlan,
    mcp_runtime: Option<&McpRuntimeInfo>,
    effective_sp: &claudine::system_prompt::ResolvedSystemPrompt,
    deferred_warnings: &[String],
    deferred_messages: &[String],
    repo_requested: bool,
    silent_requested: bool,
    quiet_requested: bool,
    detail_requested: bool,
    launch_context: &claudine::system_prompt::LaunchContext,
    opencode_model_source: Option<&profile::OpenCodeModelSource>,
    term: &Terminal,
    verbose: u8,
) {
    if silent_requested {
        return;
    }

    if !quiet_requested {
        crate::output::log_wrapper_env_details(env_plan, mcp_runtime, term, verbose);

        if let Some(info_message) = crate::output::removed_env_info_message(&env_plan.removed, term)
        {
            log::message(&info_message);
        }
        if repo_requested {
            log::message(&crate::output::repo_flag_info_message(
                term,
                env_plan.shadow_home_path.as_deref(),
            ));
        }
        for warning in &env_plan.warnings {
            log::message(&crate::output::post_env_warning_message(warning, term));
        }
        for warning in deferred_warnings {
            log::message(&crate::output::post_env_warning_message(warning, term));
        }
        for message in deferred_messages {
            log::message(&crate::output::post_env_message(message, term));
        }

        if let Some(source) = opencode_model_source {
            use biscuit_terminal::components::status::Status;
            use biscuit_terminal::prelude::TerminalRenderable as _;
            let status = Status::from_prose(source.status_markup());
            log::message(&status.render(term));
        }
    }

    let scope_for_report = launch_context
        .repo_root
        .as_deref()
        .unwrap_or(&launch_context.cwd);
    crate::output::log_system_prompt_with_scope(
        effective_sp,
        detail_requested,
        silent_requested,
        quiet_requested,
        Some(scope_for_report),
        term,
    );

    if !quiet_requested
        || matches!(
            effective_sp,
            claudine::system_prompt::ResolvedSystemPrompt::Ready(_)
        )
    {
        log::message("");
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(crate) fn prepare_stream_and_prompt(
    profile: &dyn WrapperProfile,
    provider: Provider,
    args: &WrapperArgs,
    child_args: &mut Vec<String>,
    prompt_source: &profile::PromptSource,
    effective_non_interactive: bool,
    silent_requested: bool,
    quiet_requested: bool,
    wrapper_span: &tracing::Span,
) -> Result<(
    bool,
    Verbosity,
    Option<String>,
    &'static [&'static str],
    &'static [&'static str],
    Option<StructuredCodexOutput>,
    Option<String>,
    Option<String>,
)> {
    let stdout_noise = if effective_non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    let stderr_noise = if effective_non_interactive {
        profile.stderr_noise_prefixes()
    } else {
        &[]
    };

    let use_structured = profile.supports_structured_stream()
        && effective_non_interactive
        && args.output.is_none()
        && !flags::has_explicit_native_output_request(provider, child_args);
    wrapper_span.record("structured_mode", use_structured);
    let stream_verbosity = structured_verbosity(silent_requested, quiet_requested);

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
        profile.apply_structured_stream(child_args);
    }

    let structured_codex_output = if use_structured && provider == Provider::Codex {
        Some(StructuredCodexOutput::prepare(child_args))
    } else {
        None
    };

    let mut wire_prompt: Option<String> = None;
    let stdin_seed: Option<String> = if let Some(prompt) = prompt_source.as_inline() {
        let delivery = profile.prompt_delivery(child_args, prompt, effective_non_interactive)?;
        if let Some(body) = delivery.as_wire_rpc() {
            wire_prompt = Some(body.to_string());
        }
        delivery.apply_to(child_args)
    } else {
        None
    };

    Ok((
        use_structured,
        stream_verbosity,
        cli_step_timeout,
        stdout_noise,
        stderr_noise,
        structured_codex_output,
        wire_prompt,
        stdin_seed,
    ))
}

#[allow(clippy::type_complexity)]
pub(crate) fn detect_wrapper_harness(
    provider: Provider,
    _profile: &dyn WrapperProfile,
    prompt_source: &profile::PromptSource,
    stdin_seed: Option<&str>,
    env_plan: &env::EnvPlan,
    child_cwd: &Path,
    cwd: &Path,
) -> Result<
    Option<(
        PathBuf,
        String,
        harness_orch::MaterializedHarnessPrompt,
        claudine::harness::ShellApprovalOptions,
    )>,
> {
    let base_prompt = prompt_source
        .as_inline()
        .map(|s| s.to_string())
        .or_else(|| stdin_seed.map(|s| s.to_string()));
    let harness_source = base_prompt.as_ref().and_then(|_| {
        harness_orch::find_wrapper_harness_source(provider, env_plan.repo_root.as_deref(), cwd)
    });

    if let (Some(base_prompt), Some(source_path)) = (base_prompt, harness_source) {
        let seed = harness_orch::materialize_passthrough_harness_seed(
            &source_path,
            base_prompt.clone(),
            Some(child_cwd),
        )?;
        let harness_enabled = claudine::harness::has_harness_properties(&seed.frontmatter);
        if harness_enabled {
            let shell_options = harness_orch::build_harness_shell_options(
                &source_path,
                env_plan.repo_root.as_deref(),
            );

            Ok(Some((source_path, base_prompt, seed, shell_options)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_execution_stage(
    wrapper_harness: Option<(
        PathBuf,
        String,
        harness_orch::MaterializedHarnessPrompt,
        claudine::harness::ShellApprovalOptions,
    )>,
    use_structured: bool,
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    env_plan: &env::EnvPlan,
    child_cwd: &Path,
    effective_non_interactive: bool,
    args: &WrapperArgs,
    cli_timeout_duration: Option<std::time::Duration>,
    cli_step_timeout: Option<String>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    detail_requested: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    wire_prompt: Option<String>,
    stdin_seed: Option<&str>,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
    wrapper_span: &tracing::Span,
    mut perf_collector: Option<&mut crate::perf::CommandPerfCollector>,
) -> Result<(i32, Option<String>)> {
    if let Some((source_path, base_prompt, initial_materialized, shell_options)) = wrapper_harness {
        let mut prompt_state = harness_orch::HarnessPromptState {
            mode: harness_orch::HarnessPromptMode::Passthrough,
            original_ref: source_path.display().to_string(),
            source_path,
            base_prompt: Some(base_prompt),
            overlay: indexmap::IndexMap::new(),
            prompt_tail: Vec::new(),
            next_prompt_override: None,
            next_resume_session_id: None,
        };

        let mut harness_base_args = child_args.to_vec();
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
            term,
            source_path: &source_path_for_lifecycle,
            repo_root: env_plan.repo_root.as_deref(),
            // This stage receives no `LaunchWorkspaceContext`; `ctx.*` capture
            // falls back to the prompt/source directory.
            launch_area: None,
            // No prepared snapshot here; lifecycle falls back to demand-driven
            // capture rooted at the source directory.
            context: None,
        };
        let default_lifecycle_emitter = claudine::composition::DefaultLifecycleEmitter;
        let mut lifecycle_guard = claudine::composition::LifecycleRunGuard::new(
            &default_lifecycle,
            &default_lifecycle_ctx,
            &default_lifecycle_emitter,
        );

        let (harness_code, harness_perf, _harness_signals) = harness_orch::run_harness_loop(
            provider,
            profile,
            binary_path,
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
            structured_codex_output,
            stdout_noise,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            stream_verbosity != Verbosity::Silent,
            stream_verbosity,
            detail_requested,
            env_context,
            dispatch_context,
            Some(initial_materialized),
            term,
            &mut lifecycle_guard,
            None,
            true,
        )?;
        if let (Some(collector), Some(perf)) = (perf_collector.as_mut(), harness_perf) {
            collector.set_agent_perf(perf);
        }
        Ok((harness_code, None))
    } else if use_structured {
        wrapper_exec::run_structured_stream_session(
            args,
            provider,
            profile,
            binary_path,
            child_args,
            env_plan,
            child_cwd,
            env_context,
            dispatch_context,
            stream_verbosity,
            detail_requested,
            structured_codex_output,
            wire_prompt,
            stdin_seed,
            cli_step_timeout,
            stderr_noise,
            term,
            wrapper_span,
            perf_collector,
        )
    } else {
        let mut _spawned = false;
        let result = exec::run_child(
            binary_path,
            child_args,
            &env_plan.env,
            child_cwd,
            cli_timeout_duration.map(|d| d.as_secs()),
            !effective_non_interactive,
            exec::ChildIoOptions {
                stdout_noise_prefixes: stdout_noise,
                stderr_noise_prefixes: stderr_noise,
                stdin_seed,
            },
            &mut _spawned,
        )?;
        if let Some(collector) = perf_collector.as_mut() {
            collector.set_agent_perf(result.telemetry.into_agent_perf(None));
        }
        Ok((result.data, None))
    }
}
