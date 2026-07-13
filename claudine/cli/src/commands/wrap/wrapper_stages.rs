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
    McpRuntimeInfo, StructuredCodexOutput, env, exec, flags, harness_orch, session_report,
    structured_verbosity, system_prompt, wrapper_exec,
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

    // Validate --stall-timeout on the direct-wrapper path. Mirrors the
    // --step-timeout validation but accepts `0s` as the disable sentinel
    // (`parse_timeout_allow_zero`), so a typo (`nope`, `5x`) FAILS here
    // instead of silently falling through to the `10m` built-in during
    // `resolve_stall_timeout`.
    if let Some(raw) = args.stall_timeout.as_deref() {
        claudine::harness::parse_timeout_allow_zero(raw, Path::new("<--stall-timeout>"))
            .map_err(|e| eyre!("invalid --stall-timeout value: {e}"))?;
    }

    Ok(cli_timeout_duration)
}

/// Stage 10.5 — merge the YOLO permission overlay into `OPENCODE_CONFIG_CONTENT`.
///
/// `--dangerously-skip-permissions` only auto-approves the **parent** OpenCode
/// session; subagent (Task) sessions fall back to the `"ask"` default and, with
/// no TTY under `opencode run`, block forever. The durable fix is a config-level
/// `permission` block, which OpenCode applies session-wide — parent and children
/// alike. This runs **after** MCP (Stage 10) so it is the last Claudine overlay
/// and cannot be weakened by the MCP or system-prompt writers; merging (never
/// overwriting) keeps their `mcp` / `instructions` keys intact.
///
/// Gated on OpenCode + YOLO-took-effect + non-interactive. Off the gate it is a
/// no-op, so no `permission` key appears for other providers, non-YOLO runs, or
/// interactive sessions (where OpenCode YOLO is already `not_applied`).
///
/// ## Errors
///
/// Propagates [`claudine::opencode_config::merge_overlay`] failure when the
/// existing `OPENCODE_CONFIG_CONTENT` is present but not valid UTF-8 or not a
/// JSON object.
pub(crate) fn apply_opencode_yolo_config_overlay(
    provider: Provider,
    yolo_enabled: bool,
    non_interactive: bool,
    env_plan: &mut env::EnvPlan,
) -> Result<()> {
    if provider != Provider::OpenCode || !yolo_enabled || !non_interactive {
        return Ok(());
    }

    const KEY: &str = "OPENCODE_CONFIG_CONTENT";
    let key = std::ffi::OsStr::new(KEY);
    let current = env_plan.env.get(key).map(|v| v.as_os_str());
    let merged = claudine::opencode_config::merge_overlay(
        current,
        claudine::opencode_config::yolo_permission_block(),
    )
    .map_err(|e| eyre!("failed to merge OPENCODE_CONFIG_CONTENT: {e}"))?;
    env_plan
        .env
        .insert(key.to_os_string(), std::ffi::OsString::from(merged.clone()));

    // Keep the dry-run / preflight env display authoritative. That display
    // renders from `env_plan.added`, not `env_plan.env` (the child's actual
    // env), so the merged value must replace any earlier OPENCODE_CONFIG_CONTENT
    // entry the system-prompt fold contributed — otherwise `--dry-run` would
    // show a stale config missing the permission block.
    match env_plan.added.iter_mut().find(|(k, _)| k == KEY) {
        Some(entry) => entry.1 = merged,
        None => env_plan.added.push((KEY.to_string(), merged)),
    }
    Ok(())
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

    let structured_codex_output = crate::commands::exec_prep::prepare_codex_structured_output(
        provider,
        use_structured,
        false,
        child_args,
    );

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
            // Direct-wrapper passthrough runs carry no compose params.
            rematerialize: Default::default(),
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
            args.stall_timeout.clone(),
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
            stream_verbosity == Verbosity::Silent,
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
        // Presence bracket for the direct structured-stream path (the
        // harness path above reports per attempt inside
        // `execute_harness_attempt`).
        let session_presence = session_report::SessionPresence::started(
            provider,
            args.model.as_deref(),
            !effective_non_interactive,
            env_context,
            &env_plan.env,
        );
        let status_reporter = session_presence.status_reporter();
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
            status_reporter,
        )
    } else {
        let _session_presence = session_report::SessionPresence::started(
            provider,
            args.model.as_deref(),
            !effective_non_interactive,
            env_context,
            &env_plan.env,
        );
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

// Acceptance-criteria → test mapping (spec.md §"Acceptance criteria").
// Each criterion is traceable to a passing test so reviewers can audit coverage:
//   #1 subagent external path auto-allowed — `opencode_yolo_spawn_spec_*`
//      (permission block present on the assembled child) + the permission-block
//      presence tests below.
//   #2 three `allow` keys under YOLO non-interactive —
//      `opencode_yolo_non_interactive_adds_three_allow_keys`.
//   #3 no `permission` key when not YOLO —
//      `opencode_non_yolo_non_interactive_leaves_env_untouched`.
//   #4 `instructions` + `mcp` + `permission` coexist —
//      `yolo_merge_preserves_instructions_and_mcp` (here) +
//      `opencode_merge_coexists_with_instructions` (mcp/inject.rs).
//   #5 `--dangerously-skip-permissions` still on argv —
//      `opencode_yolo_spawn_spec_has_argv_flag_and_config_permission`.
//   #6 `doom_loop` auto-allowed — asserted in every permission-block test.
//   #7 permission JSON byte-identical cross-platform —
//      `yolo_permission_block_serializes_independent_of_insertion_order`
//      (opencode_config.rs); the block carries no path content.
//   #8 existing user config merged or redacted-rejected —
//      `merge_overlay_*` (opencode_config.rs) +
//      `opencode_merge_preserves_user_supplied_config` (mcp/inject.rs).
//   #9 policy path emits no native `--yolo` —
//      `opencode_one_shot_*` (permissions/providers/opencode.rs).
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Parse a `WrapperArgs` from a bare arg list via a clap probe so the
    /// `--stall-timeout` validation can be exercised without constructing the
    /// struct by hand.
    fn wrapper_args_from(extra: &[&str]) -> WrapperArgs {
        use clap::Parser;

        // `WrapperArgs` declares its own `help` field, so the auto-generated
        // `--help` must be disabled to avoid a duplicate-argument panic.
        #[derive(Debug, clap::Parser)]
        #[command(disable_help_flag = true)]
        struct Probe {
            #[command(flatten)]
            args: WrapperArgs,
        }

        let mut argv = vec!["probe"];
        argv.extend_from_slice(extra);
        Probe::try_parse_from(argv)
            .expect("probe must parse")
            .args
    }

    #[test]
    fn parse_cli_timeouts_rejects_invalid_stall_timeout() {
        let args = wrapper_args_from(&["--stall-timeout", "nope"]);
        let err = parse_cli_timeouts(&args).unwrap_err();
        assert!(
            err.to_string().contains("invalid --stall-timeout value"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_cli_timeouts_accepts_zero_and_valid_stall_timeout() {
        for value in ["0s", "0.5s", "10m"] {
            let args = wrapper_args_from(&["--stall-timeout", value]);
            assert!(
                parse_cli_timeouts(&args).is_ok(),
                "--stall-timeout {value} should be accepted"
            );
        }
    }

    const KEY: &str = "OPENCODE_CONFIG_CONTENT";

    fn env_plan_with(current: Option<&str>) -> env::EnvPlan {
        let mut plan = env::EnvPlan::default();
        if let Some(raw) = current {
            plan.env
                .insert(std::ffi::OsString::from(KEY), std::ffi::OsString::from(raw));
        }
        plan
    }

    fn config_value(plan: &env::EnvPlan) -> Option<Value> {
        plan.env
            .get(std::ffi::OsStr::new(KEY))
            .and_then(|os| os.to_str())
            .map(|raw| serde_json::from_str(raw).expect("config is valid JSON"))
    }

    #[test]
    fn opencode_yolo_non_interactive_adds_three_allow_keys() {
        let mut plan = env_plan_with(None);
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let config = config_value(&plan).expect("permission overlay was written");
        let permission = config.get("permission").and_then(Value::as_object).unwrap();
        assert_eq!(permission["*"], "allow");
        assert_eq!(permission["external_directory"], "allow");
        assert_eq!(permission["doom_loop"], "allow");
    }

    #[test]
    fn opencode_yolo_interactive_adds_no_permission_key() {
        let mut plan = env_plan_with(None);
        // Interactive mirrors `YoloOutcome::not_applied`: the gate is closed.
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, false, &mut plan).unwrap();
        assert!(config_value(&plan).is_none());
    }

    #[test]
    fn opencode_non_yolo_non_interactive_leaves_env_untouched() {
        let existing = r#"{"instructions":["/tmp/sp.md"]}"#;
        let mut plan = env_plan_with(Some(existing));
        apply_opencode_yolo_config_overlay(Provider::OpenCode, false, true, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert!(config.get("permission").is_none());
        assert_eq!(config["instructions"], serde_json::json!(["/tmp/sp.md"]));
    }

    #[test]
    fn non_opencode_provider_is_a_no_op() {
        let mut plan = env_plan_with(None);
        apply_opencode_yolo_config_overlay(Provider::Claude, true, true, &mut plan).unwrap();
        assert!(config_value(&plan).is_none());
    }

    #[test]
    fn yolo_merge_preserves_instructions_and_mcp() {
        // Simulate Phase 2 output: both system-prompt and MCP writers ran.
        let existing = r#"{"instructions":["/tmp/sp.md"],"mcp":{"srv":{}}}"#;
        let mut plan = env_plan_with(Some(existing));
        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let config = config_value(&plan).unwrap();
        assert_eq!(config["instructions"], serde_json::json!(["/tmp/sp.md"]));
        assert_eq!(config["mcp"], serde_json::json!({ "srv": {} }));
        assert_eq!(config["permission"]["*"], "allow");
        assert_eq!(config["permission"]["external_directory"], "allow");
        assert_eq!(config["permission"]["doom_loop"], "allow");
    }

    /// Spawn-spec: a non-interactive YOLO OpenCode child carries
    /// `--dangerously-skip-permissions` on argv (from the profile) **and** the
    /// permission block in `OPENCODE_CONFIG_CONTENT` (from Stage 10.5).
    #[test]
    fn opencode_yolo_spawn_spec_has_argv_flag_and_config_permission() {
        let opencode = profile::profile_for_provider(Provider::OpenCode).unwrap();
        let mut argv = vec!["run".to_string()];
        let mut env_overrides: Vec<(String, String)> = Vec::new();
        let outcome = opencode
            .apply_yolo_for_mode(&mut argv, &mut env_overrides, false)
            .unwrap();
        assert!(outcome.applied);
        assert!(argv.iter().any(|a| a == "--dangerously-skip-permissions"));

        let mut plan = env_plan_with(None);
        apply_opencode_yolo_config_overlay(Provider::OpenCode, outcome.applied, true, &mut plan)
            .unwrap();
        let config = config_value(&plan).unwrap();
        assert_eq!(config["permission"]["external_directory"], "allow");
        assert_eq!(config["permission"]["doom_loop"], "allow");
    }

    /// Dry-run observable: `--dry-run` renders the env from `env_plan.added`
    /// (not the child `env`), so the merged permission block must appear there
    /// too — otherwise a dry-run would hide the YOLO config it is meant to show.
    #[test]
    fn opencode_yolo_overlay_is_visible_in_added_env() {
        // Simulate the system-prompt fold having already recorded an entry in
        // `added`; the overlay must replace it, not duplicate it.
        let existing = r#"{"instructions":["/tmp/sp.md"]}"#;
        let mut plan = env_plan_with(Some(existing));
        plan.added.push((KEY.to_string(), existing.to_string()));

        apply_opencode_yolo_config_overlay(Provider::OpenCode, true, true, &mut plan).unwrap();

        let entries: Vec<&(String, String)> =
            plan.added.iter().filter(|(k, _)| k == KEY).collect();
        assert_eq!(entries.len(), 1, "no duplicate OPENCODE_CONFIG_CONTENT entry");
        let shown: Value = serde_json::from_str(&entries[0].1).unwrap();
        assert_eq!(shown["instructions"], serde_json::json!(["/tmp/sp.md"]));
        assert_eq!(shown["permission"]["*"], "allow");
        assert_eq!(shown["permission"]["external_directory"], "allow");
        assert_eq!(shown["permission"]["doom_loop"], "allow");
    }
}
