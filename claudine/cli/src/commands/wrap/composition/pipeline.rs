//! Ordered setup pipeline for one composition attempt.

use color_eyre::eyre::WrapErr;
use std::path::PathBuf;
use std::time::Instant;

use super::*;
use claudine::composition::{
    LifecycleCatchProtocol, LifecycleCatchResult, LifecycleCatchState, LifecycleTransitionAbort,
    LifecycleTransitionDecision, LifecycleTransitionError, LifecycleTransitionInput, decide_lifecycle_transition,
};

enum CompositionPhaseResult<T> {
    Proceed(T),
    Completed(Box<SingleCompositionOutcome>),
    Blocked(color_eyre::Report),
    Failed(color_eyre::Report),
}

impl<T> CompositionPhaseResult<T> {
    fn from_result(result: Result<T>) -> Self {
        match result {
            Ok(value) => Self::Proceed(value),
            Err(error) => Self::Failed(error),
        }
    }

    fn from_blocked_result(result: Result<T>) -> Self {
        match result {
            Ok(value) => Self::Proceed(value),
            Err(error) => Self::Blocked(error),
        }
    }
}

struct CompositionAttempt<'guard, 'runtime> {
    request: CompositionExecutionRequest,
    verbose: u8,
    perf_enabled: bool,
    perf_collector: Option<crate::perf::CommandPerfCollector>,
    last_checkpoint: Instant,
    document_start: Instant,
    term: Terminal,
    external_guard: Option<&'guard mut LifecycleRunGuard<'runtime>>,
    skip_preflight: bool,
}

impl<'guard, 'runtime> CompositionAttempt<'guard, 'runtime> {
    fn new(
        request: CompositionExecutionRequest,
        verbose: u8,
        startup_timings: Option<crate::perf::StartupTimings>,
        perf_enabled: bool,
        external_guard: Option<&'guard mut LifecycleRunGuard<'runtime>>,
        skip_preflight: bool,
    ) -> Self {
        let perf_collector = if perf_enabled {
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
        let document_start = Instant::now();
        Self {
            request,
            verbose,
            perf_enabled,
            perf_collector,
            last_checkpoint: document_start,
            document_start,
            term: wrap_terminal(),
            external_guard,
            skip_preflight,
        }
    }
}

struct SelectionPhase {
    launch_cwd: PathBuf,
    launch_workspace: env::LaunchWorkspaceContext,
    target: ResolvedExecutionTarget,
    provider: Provider,
    is_inline: bool,
    profile: &'static dyn WrapperProfile,
    binary_path: PathBuf,
    effective_non_interactive: bool,
}

struct EnvironmentPhase {
    env_plan: env::EnvPlan,
    effective_prompt: String,
    mcp_extra_args: Vec<String>,
}

struct CommandPhase {
    effective_sp: claudine::system_prompt::ResolvedSystemPrompt,
    args_before_prompt: Vec<String>,
    use_structured: bool,
    structured_codex_output: Option<crate::commands::wrap::policy::StructuredCodexOutput>,
    stdout_noise: &'static [&'static str],
    stderr_noise: &'static [&'static str],
    stream_verbosity: Verbosity,
}

struct LifecyclePhase {
    shell_options: claudine::harness::ShellApprovalOptions,
    emitter: DefaultLifecycleEmitter,
    settings: claudine::events::GlobalSettings,
    messaging: claudine::messaging::RuntimeMessagingSettings,
    effect_engine: EffectEngine,
    context: darkmatter::markdown::compose::ComposeContext,
}

fn record_substage(
    collector: &mut Option<crate::perf::CommandPerfCollector>,
    checkpoint: &mut Instant,
    name: &'static str,
) {
    if let Some(collector) = collector {
        collector.mark_substage(name, checkpoint.elapsed());
        *checkpoint = Instant::now();
    }
}

macro_rules! proceed_phase {
    ($phase:expr) => {
        match $phase {
            CompositionPhaseResult::Proceed(value) => value,
            CompositionPhaseResult::Completed(outcome) => return Ok(*outcome),
            CompositionPhaseResult::Blocked(error)
            | CompositionPhaseResult::Failed(error) => return Err(error),
        }
    };
}

pub(super) fn execute_composition_request_inner_with_guard<'guard, 'runtime>(
    request: CompositionExecutionRequest,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
    perf_enabled: bool,
    external_guard: Option<&'guard mut LifecycleRunGuard<'runtime>>,
    skip_preflight: bool,
) -> Result<SingleCompositionOutcome> {
    let mut attempt = CompositionAttempt::new(
        request,
        verbose,
        startup_timings,
        perf_enabled,
        external_guard,
        skip_preflight,
    );

    let prepare_span = tracing::info_span!("composition_prepare").entered();
    let selection = proceed_phase!(resolve_selection_and_launch(&mut attempt));
    let mut environment =
        proceed_phase!(prepare_environment_and_mcp(&mut attempt, &selection));
    let command = proceed_phase!(construct_argv_and_system_prompt(
        &mut attempt,
        &selection,
        &mut environment,
    ));
    drop(prepare_span);

    let _preflight_span = tracing::info_span!("composition_preflight").entered();
    let lifecycle = proceed_phase!(construct_lifecycle_runtime(
        &mut attempt,
        &selection,
        &environment,
    ));
    let outcome = proceed_phase!(CompositionPhaseResult::from_blocked_result(
        provider_run_handoff(
            &mut attempt,
            &selection,
            &environment,
            &command,
            &lifecycle,
        ),
    ));
    Ok(outcome)
}

fn resolve_selection_and_launch(
    attempt: &mut CompositionAttempt<'_, '_>,
) -> CompositionPhaseResult<SelectionPhase> {
    let request = &attempt.request;
    let term = &attempt.term;
    if request.dry_run && request.resolved_target.is_none() {
        let render = dry_run::DryRunRender::from_request(request);
        crate::log::data(&render.body);
        crate::log::message(&dry_run::render_hr(term));
        crate::log::message(&dry_run::render_frontmatter_heading(term));
        crate::log::message("");
        crate::log::message(&dry_run::render_frontmatter(&render.frontmatter, term));
        crate::log::message(&dry_run::render_metadata_table(&render, term));
        if let Some(collector) = attempt.perf_collector.as_mut() {
            collector.set_dry_run();
        }
        let outcome = SingleCompositionOutcome {
            exit_code: 0,
            provider: Provider::Claude,
            agent_perf: None,
            iteration_signals: None,
            terminal_signal: None,
            final_output: None,
        };
        if let Some(collector) = attempt.perf_collector.take() {
            crate::perf::emit_report(&collector.into_report());
        }
        return CompositionPhaseResult::Completed(Box::new(outcome));
    }

    CompositionPhaseResult::from_result((|| -> Result<SelectionPhase> {
        let verbose = attempt.verbose;
        let perf_collector = &mut attempt.perf_collector;
        let last_checkpoint = &mut attempt.last_checkpoint;
        let launch_cwd = std::env::current_dir()?;
        let detail_requested = verbose > 0;
        let silent = request.silent;

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
        let target = resolve_execution_target(request, &launch_cwd, source_repo_root)?;

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
            perf_collector,
            last_checkpoint,
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
                term,
            );
        }

        record_substage(perf_collector, last_checkpoint, "header env plan");
        Ok(SelectionPhase {
            launch_cwd,
            launch_workspace,
            target,
            provider,
            is_inline,
            profile,
            binary_path,
            effective_non_interactive,
        })
    })())
}

fn prepare_environment_and_mcp(
    attempt: &mut CompositionAttempt<'_, '_>,
    selection: &SelectionPhase,
) -> CompositionPhaseResult<EnvironmentPhase> {
    CompositionPhaseResult::from_result((|| -> Result<EnvironmentPhase> {
        let request = &attempt.request;
        let perf_collector = &mut attempt.perf_collector;
        let last_checkpoint = &mut attempt.last_checkpoint;
        let perf_enabled = attempt.perf_enabled;
        let SelectionPhase {
            launch_cwd: _,
            launch_workspace,
            target: _,
            provider,
            is_inline: _,
            profile,
            binary_path: _,
            effective_non_interactive,
        } = selection;
        let provider = *provider;
        let profile = *profile;
        let effective_non_interactive = *effective_non_interactive;
        let silent = request.silent;
        let source_repo_root = request.prepared.source_repo_root.as_deref();
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
            *last_checkpoint = std::time::Instant::now();
        }

        let mut effective_prompt = request.prepared.prompt.clone();
        let mut mcp_extra_args = Vec::new();
        if request.mcp || !request.mcp_use.is_empty() {
            use claudine::mcp::catalog::McpCatalogStore;
            use claudine::mcp::inject::injector_for_provider;
            use claudine::mcp::session::{compute_session_set, lex_tags};

            let repo_root_ref = source_repo_root.or(env_plan.repo_root.as_deref());
            let _ = super::super::bootstrap_mcp_state(repo_root_ref)?;
            let catalog =
                McpCatalogStore::load().wrap_err("failed to load MCP catalog")?;
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
            .wrap_err("MCP session error")?;

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
                        .wrap_err("MCP injection failed")?;

                    // The OpenCode inline config is shared with the system-prompt
                    // and YOLO producers, so it must merge into any value already on
                    // the plan rather than overwrite it; every other key is a plain
                    // set. Mirrors the direct wrapper at wrapper_mcp.rs.
                    super::super::wrapper_mcp::merge_injected_env_into_plan(string_env, &mut env_plan)?;
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
            record_substage(perf_collector, last_checkpoint, "mcp composition");
        } else {
            if let Some(collector) = perf_collector {
                collector.mark_substage("mcp composition", std::time::Duration::ZERO);
            }
        }

        Ok(EnvironmentPhase {
            env_plan,
            effective_prompt,
            mcp_extra_args,
        })
    })())
}

fn construct_argv_and_system_prompt(
    attempt: &mut CompositionAttempt<'_, '_>,
    selection: &SelectionPhase,
    environment: &mut EnvironmentPhase,
) -> CompositionPhaseResult<CommandPhase> {
    CompositionPhaseResult::from_result((|| -> Result<CommandPhase> {
        let request = &attempt.request;
        let term = &attempt.term;
        let perf_collector = &mut attempt.perf_collector;
        let last_checkpoint = &mut attempt.last_checkpoint;
        let SelectionPhase {
            launch_cwd,
            launch_workspace,
            target,
            provider,
            is_inline,
            profile,
            binary_path: _,
            effective_non_interactive,
        } = selection;
        let provider = *provider;
        let is_inline = *is_inline;
        let profile = *profile;
        let effective_non_interactive = *effective_non_interactive;
        let EnvironmentPhase {
            env_plan,
            effective_prompt,
            mcp_extra_args,
        } = environment;
        let silent = request.silent;
        let quiet = request.quiet;
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
            term,
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
            env_plan,
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
        let _opencode_model_source: Option<super::super::profile::OpenCodeModelSource> =
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
            let format: super::super::profile::OutputFormat = (*output_str).into();
            if let Some(warn) = profile.apply_output_format(&mut child_args, format)
                && !silent
                && !quiet
            {
                log::warn(&warn);
            }
        }

        record_substage(perf_collector, last_checkpoint, "argv assembly");

        enforce_repo_launch_detection(
            request.repo,
            request.prep_launch_detection_error.as_ref(),
        )?;
        let mut launch_context = if let Some(prep) = request.prep_launch_context.as_ref() {
            // Phase fix (2026-05-09-slow-prep): reuse the launch_context computed
            // by the shared sniff scan in `CompositionPrepContext` instead of
            // re-running `sniff::detect_with_plan` here.
            prep.clone()
        } else {
            match claudine::system_prompt::LaunchContext::from_cwd(launch_cwd) {
                Ok(context) => context,
                Err(error) => {
                    if request.repo {
                        return Err(error).wrap_err(
                            "--repo requires startup repo detection, but launch-context detection failed",
                        );
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

        let scoped_tmp = super::super::system_prompt::scoped_tmp_dir(launch_workspace);

        let mut sp_artifacts: Vec<super::super::system_prompt::SystemPromptArtifact> = Vec::new();

        match &effective_sp {
            claudine::system_prompt::ResolvedSystemPrompt::None
            | claudine::system_prompt::ResolvedSystemPrompt::Disabled { .. } => {}
            claudine::system_prompt::ResolvedSystemPrompt::Ready(prepared) => {
                let application = profile.apply_system_prompt(
                    prepared,
                    !effective_non_interactive,
                    launch_cwd,
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
                    // applied earlier in the command phase, which silently re-opens the
                    // subagent `external_directory` hang the YOLO overlay exists to
                    // prevent. Route it through the same merge the MCP fold uses so
                    // `instructions`, `mcp`, and `permission` coexist.
                    if k == std::ffi::OsStr::new("OPENCODE_CONFIG_CONTENT") {
                        let injected = std::collections::HashMap::from([(
                            "OPENCODE_CONFIG_CONTENT".to_string(),
                            v.to_string_lossy().to_string(),
                        )]);
                        super::super::wrapper_mcp::merge_injected_env_into_plan(injected, env_plan)?;
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

        record_substage(perf_collector, last_checkpoint, "system prompt");

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

        child_args.append(mcp_extra_args);

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
        let prompt_source = super::super::profile::PromptSource::Inline(effective_prompt.clone());
        let delivery =
            profile.prompt_delivery(&child_args, effective_prompt, effective_non_interactive)?;
        // The harness loop rebuilds prompt delivery from the materialized prompt,
        // so the returned wire/stdin seed values are not needed here. We still
        // apply the delivery to `child_args` so the argv validation below sees the
        // final provider argv.
        delivery.apply_to(&mut child_args);

        let child_cwd = env_plan.child_cwd.as_path();

        super::super::profile::require_prompt_present(
            profile.binary(),
            effective_non_interactive,
            &prompt_source,
        )?;

        if tracing::enabled!(tracing::Level::WARN) {
            super::super::profile::validate_argv_flags_before_separator(profile.binary(), &child_args);
        }

        record_substage(
            perf_collector,
            last_checkpoint,
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

        if !silent && !quiet {
            use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
            use biscuit_terminal::prelude::TerminalRenderable as _;

            let status = Status::from_prose("Starting pre-flight checks".to_string())
                .state(StatusState::Info)
                .theme(StatusTheme::Circular);
            crate::log::message(&status.render(term));
        }

        Ok(CommandPhase {
            effective_sp,
            args_before_prompt,
            use_structured,
            structured_codex_output,
            stdout_noise,
            stderr_noise,
            stream_verbosity,
        })
    })())
}

fn construct_lifecycle_runtime(
    attempt: &mut CompositionAttempt<'_, '_>,
    selection: &SelectionPhase,
    environment: &EnvironmentPhase,
) -> CompositionPhaseResult<LifecyclePhase> {
    CompositionPhaseResult::from_result({
        let request = &attempt.request;
        let SelectionPhase {
            launch_cwd,
            launch_workspace,
            target: _,
            provider: _,
            is_inline: _,
            profile: _,
            binary_path: _,
            effective_non_interactive: _,
        } = selection;
        let env_plan = &environment.env_plan;
        let source_repo_root = request.prepared.source_repo_root.as_deref();
        let effective_repo_root = source_repo_root.or(env_plan.repo_root.as_deref());
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

        Ok(LifecyclePhase {
            shell_options,
            emitter,
            settings: lifecycle_settings,
            messaging: lifecycle_messaging,
            effect_engine: lifecycle_effect_engine,
            context: lifecycle_context,
        })
    })
}


fn execute_initialize_catch(
    guard: &mut LifecycleRunGuard<'_>,
    init_ctx: &StackExecutionContext<'_>,
    source_path: &Path,
    term: &Terminal,
    init_outcome: LifecycleEventOutcome,
) -> Result<LifecycleCatchResult> {
    let original_early = init_outcome.evaluation_error.as_ref().map(|info| {
        crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            source_path,
            "initialize",
            info,
            term,
        )
    });
    let mut protocol = LifecycleCatchProtocol::new(
        LifecycleSignal::Initialize,
        LifecycleCatchState {
            terminal_slot: guard.terminal_signal(),
            finalize_emitted: guard.finalize_emitted(),
        },
        None,
        init_outcome,
    );
    while let Some(step) = protocol.next_step().cloned() {
        let event_ctx = init_ctx.with_signal(step.signal);
        let event_ctx = match step.error.as_ref() {
            Some(error) => event_ctx.with_error(error),
            None => event_ctx,
        };
        let outcome = guard.execute_event(step.signal, &event_ctx);
        assert!(protocol.record(step.signal, outcome));
    }
    let result = protocol.finish().expect("initialize catch protocol completed");
    let Some(signal) = result.evaluation_error_signal else {
        return Ok(result);
    };
    if signal == LifecycleSignal::Initialize {
        return Err(original_early
            .expect("initialize evaluation error was emitted")
            .into());
    }
    let info = result
        .evaluation_error
        .as_ref()
        .expect("evaluation signal carries error info");
    Err(crate::output::error_walker::emit_lifecycle_evaluation_error_early(
        source_path,
        match signal {
            LifecycleSignal::Failure => "failure",
            LifecycleSignal::Finalize => "finalize",
            _ => unreachable!("catch protocol returned an unexpected signal"),
        },
        info,
        term,
    )
    .into())
}

fn route_initialize(
    guard: &mut LifecycleRunGuard<'_>,
    init_ctx: &StackExecutionContext<'_>,
    source_path: &Path,
    effective_repo_root: Option<&Path>,
    provider: Provider,
    term: &Terminal,
    file_resolution_context: Option<&biscuit_file::FileResolutionContext>,
) -> CompositionPhaseResult<Option<PathBuf>> {
    let routed = (|| -> Result<CompositionPhaseResult<Option<PathBuf>>> {
        let init_outcome = guard.execute_event(LifecycleSignal::Initialize, init_ctx);
        let init_result = execute_initialize_catch(
            guard,
            init_ctx,
            source_path,
            term,
            init_outcome.clone(),
        )?;
        if let Some(setup_error) = init_result.setup_error.as_ref() {
            let message = if matches!(
                init_result.control,
                Some(StackControl::Error { .. })
            ) {
                setup_error.msg.clone()
            } else {
                "lifecycle initialize failed".to_string()
            };
            return Err(eyre!(message));
        }
        let init_decision = decide_lifecycle_transition(&LifecycleTransitionInput {
            event: LifecycleSignal::Initialize,
            terminal_slot: None,
            provider_launched: false,
            has_prior_error: false,
            outcome: &init_outcome,
            has_session: false,
            attempt: 1,
            control_budget: 0,
            proxy_hops_used: 0,
            proxy_target_seen: false,
            finalize_emitted: false,
        });
        // Set by an `initialize` Proxy control: the resolved target document the
        // run is handed off to. Threaded into `run_composition_body` so the harness loop
        // re-composes and runs the target instead of the original document.
        let mut init_proxy_target: Option<std::path::PathBuf> = None;
        if let Some(ref control) = init_result.control {
            match (&init_decision, control) {
                (_, StackControl::Skip) => {
                    // Clean whole-document opt-out: no pre-flight, no provider,
                    // no finalize, no loop gate. Sequence orchestrator treats this
                    // as a successful step and advances.
                    return Ok(CompositionPhaseResult::Completed(Box::new(
                        SingleCompositionOutcome {
                            exit_code: 0,
                            provider,
                            agent_perf: None,
                            iteration_signals: None,
                            terminal_signal: None,
                            final_output: None,
                        },
                    )));
                }
                (
                    LifecycleTransitionDecision::ProxyHandoff { target },
                    StackControl::Proxy { .. },
                ) => {
                    // Hand off to the target document. Resolve the reference
                    // (`@repo/…`, relative, or absolute) against the source so
                    // `run_composition_body` runs the target via the harness loop's
                    // re-materialize path. The harness loop resets the lifecycle
                    // guard and re-emits the target's own `initialize` before its
                    // pre-flight / start / terminal / finalize lifecycle runs.
                    let resolved = match file_resolution_context {
                        Some(context) => claudine::composition::resolve_proxy_target_in_context(
                            target,
                            source_path,
                            context,
                        ),
                        None => claudine::composition::resolve_proxy_target(
                            target,
                            source_path,
                            effective_repo_root,
                        ),
                    }
                    .map_err(|e| CompositionError::InvalidFileReference {
                        context: Box::new(claudine::composition::FileReferenceContext {
                            source_path: source_path.to_path_buf(),
                            event: Some("initialize".to_string()),
                            // Wildcard index: the stack position that authored
                            // the `proxy` is not threaded back through the
                            // outcome, so name the stable path, not a concrete
                            // `[0]` we cannot verify (spec §D3/§D6).
                            property: "initialize.stack[*].proxy".to_string(),
                            reference: target.clone(),
                            hint: PROXY_TARGET_HINT.to_string(),
                        }),
                        source: e,
                    })?;
                    if !claudine::composition::proxy_handoff_allowed(
                        std::slice::from_ref(&source_path.to_path_buf()),
                        &resolved,
                    ) {
                        return Err(CompositionError::LifecycleProxyCycle {
                            source_path: source_path.to_path_buf(),
                            target: target.clone(),
                            chain: vec![source_path.display().to_string()],
                            limit: claudine::composition::MAX_PROXY_HOPS,
                        }
                        .into());
                    }
                    init_proxy_target = Some(resolved);
                }
                (LifecycleTransitionDecision::Continue, StackControl::Stop) => {}
                (
                    LifecycleTransitionDecision::Abort(
                        LifecycleTransitionAbort::ResumeWithoutSession,
                    ),
                    StackControl::Resume { .. },
                ) => {
                    // Pre-launch: there is no provider session to resume.
                    return Err(CompositionError::LifecycleResumeWithoutSession {
                        source_path: source_path.to_path_buf(),
                    }
                    .into());
                }
                (LifecycleTransitionDecision::Reenter(_), StackControl::Retry { .. }) => {
                    // `retry` (re-run pre-flight) needs run-loop re-entry that does
                    // not exist in the setup phase.
                    return Err(CompositionError::LifecycleSetupPhaseRecoveryUnsupported {
                        source_path: source_path.to_path_buf(),
                        event: "initialize".to_string(),
                        action: "retry".to_string(),
                    }
                    .into());
                }
                (
                    LifecycleTransitionDecision::Abort(
                        LifecycleTransitionAbort::DeferredExecutionUnsupported,
                    ),
                    StackControl::Defer { .. },
                ) => {
                    // `defer` has no runtime home yet (rendezvous scheduler pending).
                    return Err(CompositionError::LifecycleDeferNotImplemented {
                        source_path: source_path.to_path_buf(),
                    }
                    .into());
                }
                _ => unreachable!("initialize control and transition decision diverged"),
            }
        }
        Ok(CompositionPhaseResult::Proceed(init_proxy_target))
    })();
    match routed {
        Ok(result) => result,
        Err(error) => CompositionPhaseResult::Blocked(error),
    }
}

fn provider_run_handoff(
    attempt: &mut CompositionAttempt<'_, '_>,
    selection: &SelectionPhase,
    environment: &EnvironmentPhase,
    command: &CommandPhase,
    lifecycle: &LifecyclePhase,
) -> Result<SingleCompositionOutcome> {
    let request = &attempt.request;
    let SelectionPhase {
        launch_cwd,
        launch_workspace,
        target,
        provider,
        is_inline,
        profile,
        binary_path,
        effective_non_interactive,
    } = selection;
    let provider = *provider;
    let is_inline = *is_inline;
    let profile = *profile;
    let effective_non_interactive = *effective_non_interactive;
    let env_plan = &environment.env_plan;
    let source_repo_root = request.prepared.source_repo_root.as_deref();
    let effective_repo_root = source_repo_root.or(env_plan.repo_root.as_deref());
    let child_cwd = env_plan.child_cwd.as_path();
    let CommandPhase {
        effective_sp,
        args_before_prompt,
        use_structured,
        structured_codex_output,
        stdout_noise,
        stderr_noise,
        stream_verbosity,
    } = command;
    let use_structured = *use_structured;
    let stdout_noise = *stdout_noise;
    let stderr_noise = *stderr_noise;
    let stream_verbosity = *stream_verbosity;
    let LifecyclePhase {
        shell_options,
        emitter,
        settings: lifecycle_settings,
        messaging: lifecycle_messaging,
        effect_engine: lifecycle_effect_engine,
        context: lifecycle_context,
    } = lifecycle;
    let term = &attempt.term;
    let document_start = attempt.document_start;
    let silent = request.silent;
    let quiet = request.quiet;
    let show_checks = !silent;
    let detail_requested = attempt.verbose > 0;
    let verbose = attempt.verbose;
    let skip_preflight = attempt.skip_preflight;
    let perf_collector = &mut attempt.perf_collector;
    let lifecycle = &request.prepared.lifecycle;
    // Common execution body used both when the caller provides an external
    // lifecycle guard (loop re-entry) and when this function owns the guard
    // (single-run / first loop iteration). The shared, read-only state is
    // bundled in `CompositionRunCtx`; the mutable `guard` and
    // `perf_collector` vary per call and are passed explicitly.
    let ctx = runner::CompositionRunCtx {
        request,
        target,
        provider,
        effective_repo_root,
        launch_workspace,
        launch_cwd,
        binary_path,
        lifecycle_effect_engine,
        emitter,
        lifecycle_settings,
        lifecycle_messaging,
        lifecycle_context,
        term,
        document_start,
        shell_options,
        silent,
        quiet,
        is_inline,
        profile,
        effective_non_interactive,
        args_before_prompt,
        child_cwd,
        use_structured,
        structured_codex_output,
        stdout_noise,
        stderr_noise,
        show_checks,
        stream_verbosity,
        detail_requested,
        env_plan,
        effective_sp,
        verbose,
    };

    // When the caller passes an external guard (loop re-entry), it has already
    // emitted `initialize` and owns the lifecycle runtime context. Run only
    // the per-iteration body and return.
    if let Some(guard) = attempt.external_guard.take() {
        return runner::run_composition_body(
            &ctx,
            guard,
            perf_collector,
            skip_preflight,
            None,
        );
    }

    // Bundle the shared lifecycle bindings (constructed above the closure)
    // into the runtime context the guard drives.
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: lifecycle_settings,
        messaging: lifecycle_messaging,
        term,
        source_path: &request.prepared.resolved_path,
        repo_root: effective_repo_root,
        launch_area: Some(launch_workspace.launch_cwd.as_path()),
        context: Some(lifecycle_context),
    };

    // --- Initialize lifecycle event --------------------------------------
    // Fires after prompt/frontmatter resolution and CLI/frontmatter override
    // merge, but before $schema validation and shell pre-flight.
    let mut guard = LifecycleRunGuard::new(lifecycle, &lifecycle_ctx, emitter);
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
        runtime_state: None,
        err: None,
        timing: Some(&lifecycle_timing),
        current: Some(&lifecycle_current),
        group: None,
        base_dir,
        ctx_base_dir: Some(launch_workspace.launch_cwd.as_path()),
        prepared_context: Some(lifecycle_context),
        file_resolution_context: request.prepared.rematerialize.file_resolution_context.as_ref(),
        effect_engine: lifecycle_effect_engine,
        shell_runner: &SystemShellRunner,
        emitter,
        term,
        source_path: &request.prepared.resolved_path,
        repo_root: effective_repo_root,
        messaging: lifecycle_messaging,
        settings: lifecycle_settings,
    };
    let init_proxy_target = proceed_phase!(route_initialize(
        &mut guard,
        &init_ctx,
        &request.prepared.resolved_path,
        effective_repo_root,
        provider,
        term,
        request.prepared.rematerialize.file_resolution_context.as_ref(),
    ));

    runner::run_composition_body(
        &ctx,
        &mut guard,
        perf_collector,
        false,
        init_proxy_target.as_deref(),
    )
}

#[cfg(test)]
mod tests;
