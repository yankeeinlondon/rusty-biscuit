use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::LifecycleSignal;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::info_span;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessPromptMode {
    Passthrough,
    Inline,
    Compose,
}

pub(crate) fn harness_prompt_mode_label(mode: HarnessPromptMode) -> &'static str {
    match mode {
        HarnessPromptMode::Passthrough => "passthrough",
        HarnessPromptMode::Inline => "inline",
        HarnessPromptMode::Compose => "compose",
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HarnessPromptState {
    pub(crate) mode: HarnessPromptMode,
    pub(crate) source_path: PathBuf,
    /// The original file reference string (for reporting).
    pub(crate) original_ref: String,
    pub(crate) base_prompt: Option<String>,
    pub(crate) overlay: indexmap::IndexMap<String, serde_json::Value>,
    pub(crate) prompt_tail: Vec<String>,
    pub(crate) next_prompt_override: Option<String>,
    pub(crate) next_resume_session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct MaterializedHarnessPrompt {
    pub(crate) frontmatter: serde_json::Value,
    pub(crate) prompt: String,
    pub(crate) env_overrides: Vec<(String, String)>,
    pub(crate) inline_closure_plan: Option<claudine::composition::InlineClosurePlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct AttemptLaunch {
    pub(crate) args: Vec<String>,
    pub(crate) env: HashMap<OsString, OsString>,
    pub(crate) stdin_seed: Option<String>,
    /// Wire-mode JSON-RPC prompt body, when the provider's prompt
    /// delivery requested transport via [`super::wire_io::run_kimi_wire_session`]
    /// instead of stdin / argv. Mutually exclusive with `stdin_seed` for
    /// the same launch.
    pub(crate) wire_prompt: Option<String>,
    /// Unified timeout configuration resolved through the full precedence
    /// chain (CLI > frontmatter > env > built-in default). Drives the
    /// timeout watchdog ticker for `timeout` (wall-clock) and
    /// `step_timeout` (stream silence). Carries the supporting knobs
    /// `kill_grace` and `interval` from `CLAUDINE_KILL_GRACE` /
    /// `CLAUDINE_WATCHDOG_INTERVAL`.
    pub(crate) timeout_config: super::subagent_watchdog::TimeoutConfig,
}

pub(crate) fn harness_policy_root(source_path: &Path, repo_root: Option<&Path>) -> Option<PathBuf> {
    let source_dir = source_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())?;

    if let Some(source_repo_root) = find_git_root(source_dir) {
        return Some(source_repo_root);
    }

    if let Some(repo_root) = repo_root
        && source_path.starts_with(repo_root)
    {
        return Some(repo_root.to_path_buf());
    }

    Some(source_dir.to_path_buf())
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;

    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    None
}

pub(crate) fn build_harness_shell_options(
    source_path: &Path,
    repo_root: Option<&Path>,
) -> claudine::harness::ShellApprovalOptions {
    build_harness_shell_options_with_cache(source_path, repo_root, None)
}

/// Build shell approval options, optionally reusing a shared approval
/// cache. Callers like the sequence orchestrator pass a shared cache so
/// that "allow once" approvals from earlier steps carry over to later
/// ones for the duration of the sequence run.
///
/// The interactive approval handler is installed whenever the process
/// can actually prompt — i.e. stdin and stderr are both TTYs. This is
/// independent of whether the spawned agent runs in interactive mode:
/// shell approval happens during preflight, before any agent is launched,
/// so there is no TTY contention. Non-TTY environments (CI, piped input)
/// get no handler and unapproved commands hard-fail as before.
pub(crate) fn build_harness_shell_options_with_cache(
    source_path: &Path,
    repo_root: Option<&Path>,
    shared_cache: Option<claudine::composition::SharedApprovalCache>,
) -> claudine::harness::ShellApprovalOptions {
    let approval_handler: Option<
        std::sync::Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>,
    > = if darkmatter_cli::approval::can_prompt_interactively() {
        Some(std::sync::Arc::new(
            darkmatter_cli::approval::CliShellApprovalHandler,
        ))
    } else {
        None
    };
    let mut opts = claudine::harness::ShellApprovalOptions {
        policy_root: harness_policy_root(source_path, repo_root),
        approval_handler,
        ..Default::default()
    };
    if let Some(cache) = shared_cache {
        opts.approval_cache = cache;
    }
    opts
}

#[derive(Clone)]
pub(crate) struct CachedHarnessLoopContext {
    pub(crate) source_path: PathBuf,
    pub(crate) repo_root: Option<PathBuf>,
    pub(crate) shell_options: claudine::harness::ShellApprovalOptions,
}

impl CachedHarnessLoopContext {
    pub(crate) fn with_shell_options(
        source_path: &Path,
        repo_root: Option<&Path>,
        shell_options: claudine::harness::ShellApprovalOptions,
    ) -> Self {
        Self {
            source_path: source_path.to_path_buf(),
            repo_root: repo_root.map(Path::to_path_buf),
            shell_options,
        }
    }

    pub(crate) fn refresh(&mut self, source_path: &Path, repo_root: Option<&Path>) {
        let repo_root = repo_root.map(Path::to_path_buf);
        if self.source_path != source_path || self.repo_root != repo_root {
            self.source_path = source_path.to_path_buf();
            self.repo_root = repo_root;
            self.shell_options.policy_root =
                harness_policy_root(&self.source_path, self.repo_root.as_deref());
        }
    }

    pub(crate) fn resolve_context(&self,
    ) -> claudine::harness::HarnessResolutionContext<'_> {
        claudine::harness::HarnessResolutionContext {
            source_path: &self.source_path,
            repo_root: self.repo_root.as_deref(),
        }
    }

    pub(crate) fn shell_options(&self) -> &claudine::harness::ShellApprovalOptions {
        &self.shell_options
    }

    /// Strip the interactive approval handler so subsequent harness-loop
    /// iterations operate in deny-only mode.  Cached and whitelisted
    /// commands still pass; new uncached commands are denied without
    /// prompting.  This enforces the spec contract: "all shell approvals
    /// are resolved before the provider workflow begins."
    pub(crate) fn freeze_shell_approvals(&mut self) {
        self.shell_options.approval_handler = None;
    }
}

pub(crate) fn materialized_harness_prompt_from_prepared(
    prepared: &claudine::composition::PreparedComposition,
) -> MaterializedHarnessPrompt {
    let inline_closure_plan = match &prepared.closure {
        claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan.clone()),
        claudine::composition::CompositionClosurePlan::Direct => None,
    };

    MaterializedHarnessPrompt {
        frontmatter: prepared.effective_frontmatter.clone(),
        prompt: prepared.prompt.clone(),
        env_overrides: Vec::new(),
        inline_closure_plan,
    }
}

pub(crate) fn materialize_passthrough_harness_seed(
    source_path: &Path,
    prompt: String,
) -> Result<MaterializedHarnessPrompt> {
    super::overlay::materialize_passthrough_harness_seed(source_path, prompt)
}

pub(crate) fn find_wrapper_harness_source(
    provider: Provider,
    repo_root: Option<&Path>,
    cwd: &Path,
) -> Option<PathBuf> {
    let capabilities = claudine::provider::provider_info(provider).agent_capabilities();
    let search_root = repo_root.unwrap_or(cwd);

    capabilities
        .runtime
        .system_prompt
        .memory_files
        .iter()
        .filter(|path| !path.starts_with('~'))
        .map(PathBuf::from)
        .find_map(|relative| {
            let candidate = search_root.join(relative);
            candidate.is_file().then_some(candidate)
        })
}

pub(crate) fn materialize_harness_prompt(
    state: &HarnessPromptState,
    _repo_root: Option<&Path>,
) -> Result<MaterializedHarnessPrompt> {
    let source_text = fs::read_to_string(&state.source_path)
        .map_err(|e| eyre!("failed to read '{}': {e}", state.source_path.display()))?;
    let mut effective_markdown: darkmatter::markdown::Markdown = source_text.clone().into();
    super::overlay::merge_frontmatter_overlay(
        effective_markdown.frontmatter_mut().as_map_mut(),
        &state.overlay,
    );

    let (mut prompt, frontmatter, env_overrides, inline_closure_plan) = match state.mode {
        HarnessPromptMode::Passthrough => {
            let options = darkmatter::markdown::compose::ComposeOptions::new()
                .with_source_file(&state.source_path);
            let (composed, _report) = effective_markdown.compose_with(options)?;
            let prompt = state.base_prompt.clone().ok_or_else(|| {
                eyre!(
                    "missing passthrough prompt seed for '{}'",
                    state.source_path.display()
                )
            })?;
            (
                prompt,
                super::overlay::frontmatter_map_to_value(composed.frontmatter()),
                Vec::new(),
                None,
            )
        }
        HarnessPromptMode::Compose => {
            let options = darkmatter::markdown::compose::ComposeOptions::new()
                .with_source_file(&state.source_path);
            let (composed, _report) = effective_markdown.compose_with(options)?;
            let body = composed.content().to_string();

            let env_overrides = Vec::new();

            (
                body,
                super::overlay::frontmatter_map_to_value(composed.frontmatter()),
                env_overrides,
                None,
            )
        }
        HarnessPromptMode::Inline => {
            claudine::composition::validate_file_permissions(&state.source_path)
                .map_err(|e| eyre!("frontmatter-prompt: {e}"))?;
            let source = claudine::composition::ResolvedCompositionSource {
                original_ref: state.source_path.display().to_string(),
                resolved_path: state.source_path.clone(),
                original_text: source_text.clone(),
                markdown: effective_markdown.clone(),
            };
            let prepared = claudine::composition::prepare_inline(
                &source,
                claudine::composition::PrepareOptions::default(),
            )?;
            (
                prepared.prompt,
                prepared.effective_frontmatter,
                Vec::new(),
                match prepared.closure {
                    claudine::composition::CompositionClosurePlan::Inline(plan) => Some(plan),
                    claudine::composition::CompositionClosurePlan::Direct => None,
                },
            )
        }
    };

    if let Some(ref override_prompt) = state.next_prompt_override {
        prompt = override_prompt.clone();
    } else {
        for tail in &state.prompt_tail {
            prompt.push_str("\n\n");
            prompt.push_str(tail);
        }
    }

    Ok(MaterializedHarnessPrompt {
        frontmatter,
        prompt,
        env_overrides,
        inline_closure_plan,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_harness_launch(
    provider: Provider,
    profile: &dyn super::profile::WrapperProfile,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    state: &mut HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    effective_non_interactive: bool,
    cli_timeout: Option<String>,
    plan_timeout: Option<std::time::Duration>,
    cli_step_timeout: Option<String>,
    plan_step_timeout: Option<std::time::Duration>,
) -> Result<AttemptLaunch> {
    let mut args = if let Some(session_id) = state.next_resume_session_id.take() {
        let mut args = super::resume::normalize_resume_args(profile, profile.build_resume_args(&session_id)?);
        super::resume::append_resume_passthrough_args(&mut args, base_args);
        args
    } else {
        base_args.to_vec()
    };
    state.next_prompt_override = None;

    let prompt = super::inline::strip_prompt_tags_for_provider(provider, &materialized.prompt);
    let prompt_source = super::profile::PromptSource::Inline(prompt.clone());
    let delivery = profile.prompt_delivery(&args, &prompt, effective_non_interactive)?;
    let wire_prompt = delivery.as_wire_rpc().map(str::to_string);
    let stdin_seed = delivery.apply_to(&mut args);
    super::profile::require_prompt_present(profile.binary(), effective_non_interactive, &prompt_source)?;

    let mut env = base_env.clone();
    for (key, value) in &materialized.env_overrides {
        env.insert(key.clone().into(), value.clone().into());
    }

    let timeout_config = super::composition::resolve_timeouts(
        cli_timeout,
        plan_timeout,
        cli_step_timeout,
        plan_step_timeout,
    );

    Ok(AttemptLaunch {
        args,
        env,
        stdin_seed,
        wire_prompt,
        timeout_config,
    })
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(crate) fn execute_harness_attempt(
    attempt: u32,
    provider: Provider,
    profile: &dyn super::profile::WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    launch: &AttemptLaunch,
    prompt_mode: HarnessPromptMode,
    prompt_state: &HarnessPromptState,
    _materialized: &MaterializedHarnessPrompt,
    use_structured: bool,
    structured_codex_output: Option<&super::policy::StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
    child_spawned: &mut bool,
    prompt_timing: Option<claudine::stream::prompt_timing::PromptTimingContext>,
) -> Result<(
    claudine::harness::AttemptOutcome,
    Option<crate::perf::AgentExecutionPerf>,
)> {
    let _attempt_span = info_span!(
        "harness_attempt",
        provider = %provider,
        attempt,
        prompt_mode = harness_prompt_mode_label(prompt_mode),
        source_path = %prompt_state.source_path.display(),
        use_structured,
    )
    .entered();
    // Step-silence enforcement requires live `SemanticEvent` ticks, which only
    // exist in the structured-stream path. Capture and passthrough attempts
    // drop the value with a warning rather than silently ignoring it.
    let launch = if !use_structured && launch.timeout_config.step_timeout.is_some() {
        use biscuit_terminal::components::renderable::Renderable;
        use biscuit_terminal::components::status::{Status, StatusState};
        let rendered = Status::new(
            "step_timeout is only enforced in structured-stream mode; \
             ignoring for this capture/passthrough attempt"
                .to_string(),
        )
        .state(StatusState::Warning)
        .render(term);
        eprintln!("{rendered}");
        let mut adjusted = launch.clone();
        adjusted.timeout_config.step_timeout = None;
        adjusted
    } else {
        launch.clone()
    };
    let launch = &launch;
    let (exit_code, termination, session_id, final_response, stderr_text, perf) = if use_structured
    {
        let summary_details = Arc::new(Mutex::new(super::policy::StructuredSummaryDetails::default()));
        let parser_config = claudine::stream::ParserConfig::default();
        let sink = super::live_semantic_sink::LiveSemanticSink::with_default_wiring(
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
        let section_stream = sink.section_stream();
        let (build_parser, stderr_bridge) =
            super::policy::build_structured_plumbing(provider, sink, parser_config);
        let stream_result = if let Some(wire_prompt) = launch.wire_prompt.clone() {
            let runtime_context =
                match claudine::dispatch::DispatchRuntimeContext::load_for_env(env_context) {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        tracing::warn!(%provider, "failed to preload wire runtime config: {error}");
                        claudine::dispatch::DispatchRuntimeContext::default()
                    }
                };
            let _ = stderr_bridge;
            let _ = prompt_timing;
            super::wire_io::run_kimi_wire_session(
                super::wire_io::WireSessionConfig {
                    binary: binary_path,
                    args: &launch.args,
                    env: &launch.env,
                    cwd: child_cwd,
                    prompt: wire_prompt,
                    timeout: launch.timeout_config.timeout,
                    client_name: env!("CARGO_PKG_NAME"),
                    client_version: env!("CARGO_PKG_VERSION"),
                    capabilities: super::wire_io::WireClientCapabilities::default_for_claudine(),
                    env_context: env_context.clone(),
                },
                super::wire_io::WireSessionWiring {
                    build_parser,
                    stream_output,
                    live_metrics,
                    runtime_context,
                },
                child_spawned,
            )?
        } else {
            super::exec::run_child_stream_semantic(
                binary_path,
                &launch.args,
                &launch.env,
                child_cwd,
                launch.timeout_config,
                stderr_noise,
                suppress_stderr_on_success,
                stream_verbosity != Verbosity::Silent,
                launch.stdin_seed.as_deref(),
                build_parser,
                child_spawned,
                live_metrics,
                stream_output,
                stderr_bridge,
                prompt_timing,
                watchdog_state,
                Some(section_stream.tracker()),
            )?
        };
        let api_duration_ms = stream_result.data.duration_ms;
        let perf = Some(stream_result.telemetry.into_agent_perf(api_duration_ms));
        let termination = stream_result.termination;
        let mut summary = stream_result.data;
        if let Some(codex_output) = structured_codex_output {
            codex_output.apply_to_summary(&mut summary);
        }
        if provider == Provider::Codex && !summary.assistant_text.is_empty() {
            section_stream.enter_final_stdout();
            let text = &summary.assistant_text;
            if std::io::stdout().is_terminal() {
                let rendered = crate::output::render_assistant_markdown(text, term);
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

        super::policy::emit_stream_summary(
            &summary,
            profile,
            env_context,
            stream_verbosity,
            detail_requested,
            &summary_details.lock().unwrap().clone(),
            Some(&section_stream),
        );

        (
            summary.exit_code,
            termination,
            summary.session_id.clone(),
            summary.assistant_text.clone(),
            summary.stderr_text.clone(),
            perf,
        )
    } else {
        let capture = super::exec::run_child_capture(
            binary_path,
            &launch.args,
            &launch.env,
            child_cwd,
            launch.timeout_config.timeout.map(|d| d.as_secs()),
            super::exec::ChildIoOptions {
                stdout_noise_prefixes: stdout_noise,
                stderr_noise_prefixes: stderr_noise,
                stdin_seed: launch.stdin_seed.as_deref(),
            },
            child_spawned,
        )?;
        let perf = Some(capture.telemetry.into_agent_perf(None));
        let termination = capture.termination;
        let stdout = capture.data.stdout;
        let stderr = capture.data.stderr;
        let response = profile.parse_captured_output(&stdout);

        if !response.trim().is_empty() {
            if std::io::stdout().is_terminal() {
                let rendered = crate::output::render_assistant_markdown(&response, term);
                std::io::stdout().write_all(rendered.as_bytes())?;
                if !rendered.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            } else {
                std::io::stdout().write_all(response.as_bytes())?;
                if !response.ends_with('\n') {
                    std::io::stdout().write_all(b"\n")?;
                }
            }
            std::io::stdout().flush()?;
        }

        if !stderr.trim().is_empty() {
            eprintln!("{stderr}");
        }

        (
            capture.data.exit_code,
            termination,
            None,
            response,
            (!stderr.trim().is_empty()).then_some(stderr),
            perf,
        )
    };

    if prompt_mode == HarnessPromptMode::Inline {
        super::inline::report_inline_agent_status(
            provider,
            &prompt_state.source_path,
            &final_response,
            exit_code,
            termination,
            child_cwd,
            show_checks,
            term,
        );
    }

    Ok((
        claudine::harness::AttemptOutcome {
            attempt,
            session_id,
            final_response,
            exit_code,
            termination,
            stderr_text,
        },
        perf,
    ))
}

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(crate) fn run_harness_loop(
    provider: Provider,
    profile: &dyn super::profile::WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    effective_non_interactive: bool,
    cli_timeout: Option<String>,
    cli_step_timeout: Option<String>,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    prompt_state: &mut HarnessPromptState,
    repo_root: Option<&Path>,
    shell_options: claudine::harness::ShellApprovalOptions,
    use_structured: bool,
    structured_codex_output: Option<&super::policy::StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    term: &Terminal,
    lifecycle: &claudine::composition::LifecycleConfig,
    lifecycle_ctx: &claudine::composition::LifecycleRuntimeContext<'_>,
    lifecycle_emitter: &dyn claudine::composition::LifecycleEmitter,
    // When `true`, every structured-stream attempt in the harness loop
    // emits the prompt-scoped timing header and — if the parsed plan
    // carries `timeout_warn` / `step_timeout_warn` — their fire-once
    // warning lines. Wrapper passthrough callers with no prompt file
    // pass `false` to suppress the header entirely; composition callers
    // pass `true`.
    emit_prompt_timing: bool,
) -> Result<(i32, Option<crate::perf::AgentExecutionPerf>)> {
    const DEFAULT_MAX_RETRIES: u32 = 3;
    let mut guard =
        claudine::composition::LifecycleRunGuard::new(lifecycle, lifecycle_ctx, lifecycle_emitter);
    let permission_probe =
        super::policy::WrapperHarnessPermissionProbe::new(provider, base_args.to_vec(), repo_root);
    let mut harness_context = CachedHarnessLoopContext::with_shell_options(
        &prompt_state.source_path,
        repo_root,
        shell_options,
    );
    let mut attempt = 1u32;
    let mut initial_materialized = initial_materialized;
    let mut harness_perf: Option<crate::perf::AgentExecutionPerf> = None;
    let mut _harness_attempts: usize = 0;

    loop {
        let _attempt_cycle_span = info_span!(
            "harness_attempt_cycle",
            provider = %provider,
            attempt,
            prompt_mode = harness_prompt_mode_label(prompt_state.mode),
            source_path = %prompt_state.source_path.display(),
        )
        .entered();
        harness_context.refresh(&prompt_state.source_path, repo_root);
        let materialized = if let Some(seed) = initial_materialized.take() {
            seed
        } else {
            info_span!(
                "harness_materialize_prompt",
                attempt,
                source_path = %prompt_state.source_path.display(),
            )
            .in_scope(|| materialize_harness_prompt(prompt_state, repo_root))
            .map_err(|e| guard.emit_blocked_or_err(e))?
        };
        let resolve_ctx = harness_context.resolve_context();
        let mut plan = info_span!(
            "harness_plan_parse",
            attempt,
            source_path = %prompt_state.source_path.display(),
        )
        .in_scope(|| {
            claudine::harness::parse_harness_plan(
                &materialized.frontmatter,
                &prompt_state.source_path,
                &resolve_ctx,
            )
        })
        .map_err(|e| guard.emit_blocked_or_err(e))?;

        // Source-file existence reporting
        if show_checks {
            claudine::harness::report::report_source_file(
                &prompt_state.original_ref,
                &prompt_state.source_path,
                term,
            );
        }
        if !prompt_state.source_path.exists() {
            if show_checks {
                claudine::harness::report::report_unhandled_failure(
                    "source file does not exist — cannot proceed",
                    term,
                );
            }
            guard.emit_blocked_or_failure();
            return Err(eyre!(
                "source file does not exist: {}",
                prompt_state.source_path.display()
            ));
        }

        // For inline composition, prepend a system-owned writability
        // pre-check so handler recovery paths can respond to permission
        // failures.
        if matches!(prompt_state.mode, HarnessPromptMode::Inline) {
            plan.pre_checks.insert(
                0,
                claudine::harness::inline_writability_pre_check(&prompt_state.source_path),
            );
        }

        // Shell audit preflight.
        //
        // Composition flows (Compose/Inline) preflight all shell commands
        // before the provider starts — template directives during composition
        // and harness commands in execute_composition_request.  The per-
        // attempt audit below is redundant for those modes because:
        //
        //   1. source_text is None, so source-page ::shell directives are
        //      excluded (they were discovered via Darkmatter's graph walker
        //      during composition, which respects ::block when="false").
        //   2. Harness commands were approved and cached during the
        //      composition preflight pass.
        //   3. The approval handler is frozen after attempt 1, so no new
        //      interactive prompts are possible.
        //
        // Only Passthrough mode needs the per-attempt audit because it reads
        // raw source text and the source file may change between
        // redirect/retry iterations.
        if matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            let source_text = std::fs::read_to_string(&prompt_state.source_path).ok();

            let auditable =
                claudine::harness::collect_auditable_commands(&plan, source_text.as_deref())?;

            let audit_report = info_span!(
                "harness_shell_audit",
                attempt,
                command_count = auditable.len(),
            )
            .in_scope(|| {
                claudine::harness::audit_shell_commands(&auditable,
                    harness_context.shell_options(),
                )
            });

            if show_checks {
                claudine::harness::report::report_shell_audit_header(
                    audit_report.outcomes.len(),
                    term,
                );
                claudine::harness::report::report_shell_audit_outcomes(&audit_report, term);
            }

            if !audit_report.all_passed() {
                let failed = audit_report.failures();
                let (source_failures, harness_failures): (Vec<_>, Vec<_>) =
                    failed.into_iter().partition(|o| {
                        matches!(
                            o.command.source,
                            claudine::harness::AuditedCommandSource::ComposeSourceLine { .. }
                        )
                    });

                // Source-page ::shell failures are terminal in v1 — no recovery.
                if !source_failures.is_empty() {
                    if show_checks {
                        claudine::harness::report::report_unhandled_failure(
                            "shell audit failed for source-page directives — cannot proceed",
                            term,
                        );
                    }
                    guard.emit_blocked_or_failure();
                    return Err(eyre!(
                        "shell audit failed: {} denied directive(s) in source page",
                        source_failures.len()
                    ));
                }

                // Non-source failures flow through handler resolution.
                if !harness_failures.is_empty() {
                    let contexts = claudine::harness::build_audit_failure_context(
                        &harness_failures,
                        provider.as_slug(),
                        plan.source_path.as_path(),
                        attempt,
                    );
                    if let Some(next_plan) = super::resume::try_resolve_handler(
                        &contexts,
                        &plan,
                        attempt,
                        DEFAULT_MAX_RETRIES,
                        profile,
                        None,
                        &prompt_state.source_path,
                        repo_root,
                        show_checks,
                        term,
                    )? {
                        attempt = next_plan.next_attempt;
                        continue;
                    }

                    let msg = format!(
                        "shell audit failed: {} command(s) denied. \
                         No handler available to resolve.",
                        harness_failures.len()
                    );
                    if show_checks {
                        claudine::harness::report::report_unhandled_failure(&msg, term);
                    }
                    guard.emit_blocked_or_failure();
                    return Err(eyre!("shell audit failed"));
                }
            }
        }

        // Composition flows resolved all shell approvals during preflight.
        // Freeze the approval set so redirect/retry iterations cannot
        // trigger new interactive prompts — only cached/whitelisted
        // commands pass; new uncached commands are denied.  Passthrough
        // mode has no prior preflight so its handler stays active.
        if attempt == 1 && !matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            harness_context.freeze_shell_approvals();
        }

        let pre_report = info_span!(
            "harness_pre_validation",
            attempt,
            rule_count = plan.pre_checks.len(),
        )
        .in_scope(|| claudine::harness::evaluate_pre_checks(&plan, Some(&permission_probe)));

        if show_checks {
            claudine::harness::report::report_phase_discovery(
                claudine::harness::FailurePhase::PreCheck,
                pre_report.count(),
                term,
            );
            claudine::harness::report::report_check_outcomes(&pre_report, term);
        }

        if !pre_report.all_passed() {
            let failures = pre_report.failures();
            let contexts = claudine::harness::build_validation_failure_context(
                &failures,
                provider.as_slug(),
                plan.source_path.as_path(),
                attempt,
                None,
                None,
            );
            if let Some(next_plan) = super::resume::try_resolve_handler(
                &contexts,
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                None,
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            let fail_msg = format!(
                "pre-check validation failed ({} {})",
                failures.len(),
                if failures.len() == 1 {
                    "failure"
                } else {
                    "failures"
                }
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            guard.emit_blocked_or_failure();
            return Err(eyre!("{fail_msg}"));
        }

        // Emit start lifecycle signal before the first provider launch.
        guard.emit_start_once();

        let snapshot = info_span!(
            "harness_pre_snapshot",
            attempt,
            rule_count = plan.post_checks.len(),
        )
        .in_scope(|| claudine::harness::capture_pre_run_snapshot(&plan))
        .map_err(|e| eyre!("harness snapshot: {e}"))?;
        let launch = build_harness_launch(
            provider,
            profile,
            base_args,
            base_env,
            prompt_state,
            &materialized,
            effective_non_interactive,
            cli_timeout.clone(),
            plan.timeout,
            cli_step_timeout.clone(),
            plan.step_timeout,
        )?;
        let _launch_span = info_span!(
            "harness_launch_plan",
            attempt,
            timeout_secs = launch.timeout_config.timeout.map(|d| d.as_secs()).unwrap_or(0),
            step_timeout_secs = launch.timeout_config.step_timeout.map(|d| d.as_secs()).unwrap_or(0),
        )
        .entered();

        // Build the prompt-scoped timing context for this attempt. The
        // warn thresholds are re-read from each parsed plan so a
        // handler that redirects to a different source document picks
        // up the replacement document's warn values, not the original's.
        let prompt_timing = if emit_prompt_timing {
            Some(
                super::composition::build_prompt_timing_context(
                    &prompt_state.source_path,
                    repo_root,
                    plan.timeout_warn,
                    plan.step_timeout_warn,
                ),
            )
        } else {
            None
        };

        let mut child_spawned = false;
        let attempt_result = execute_harness_attempt(
            attempt,
            provider,
            profile,
            binary_path,
            child_cwd,
            &launch,
            prompt_state.mode,
            prompt_state,
            &materialized,
            use_structured,
            structured_codex_output,
            stdout_noise,
            stderr_noise,
            suppress_stderr_on_success,
            show_checks,
            stream_verbosity,
            detail_requested,
            env_context,
            dispatch_context,
            term,
            &mut child_spawned,
            prompt_timing,
        );

        // Mark launched as soon as spawn succeeded — before propagating
        // any post-spawn error — so the guard correctly classifies
        // subsequent failures as `Failure` rather than `Blocked`.
        if child_spawned {
            guard.mark_provider_launched();
        }
        let (outcome, perf) = attempt_result?;
        if let Some(p) = perf {
            _harness_attempts += 1;
            match harness_perf.as_mut() {
                Some(acc) => {
                    acc.launches += p.launches;
                    acc.total_elapsed += p.total_elapsed;
                    if acc.first_response_latency.is_none() && p.first_response_latency.is_some() {
                        acc.first_response_latency = p.first_response_latency;
                    }
                    if let Some(api) = p.provider_api_duration {
                        acc.provider_api_duration = Some(
                            acc.provider_api_duration
                                .unwrap_or(std::time::Duration::ZERO)
                                + api,
                        );
                    }
                }
                None => {
                    harness_perf = Some(p);
                }
            }
        }

        if outcome.termination == claudine::harness::ProcessTermination::Interrupted {
            // Surface the interrupt to the user before we let the guard
            // close: without this the wrapper would silently return 130
            // and the operator has no feedback that Claudine noticed.
            eprintln!("{}", crate::output::format_user_interrupt_status());
            guard.emit_terminal(LifecycleSignal::Failure);
            return Ok((outcome.exit_code, harness_perf));
        }

        if let Some(failure_event) = claudine::harness::classify_failure(&outcome) {
            let message = match failure_event {
                claudine::harness::FailureEvent::Timeout => {
                    format!("provider timed out (attempt {attempt})")
                }
                claudine::harness::FailureEvent::AgentFailure => {
                    format!(
                        "agent exited with error code {} (attempt {attempt})",
                        outcome.exit_code
                    )
                }
                _ => format!("failure on attempt {attempt}"),
            };
            let ctx = claudine::harness::build_agent_failure_context(
                provider.as_slug(),
                plan.source_path.as_path(),
                failure_event,
                message.clone(),
                attempt,
                outcome.session_id.clone(),
                Some(outcome.clone()),
            );
            if let Some(next_plan) = super::resume::try_resolve_handler(
                &[ctx],
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                outcome.session_id.as_deref(),
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&message, term);
            }
            guard.emit_terminal(LifecycleSignal::Failure);
            return Err(eyre!("{message}"));
        }

        // For inline mode, apply closure BEFORE post-checks so that
        // file-state checks (file_changed, frontmatter comparisons, etc.)
        // observe the final rewritten document rather than the pre-closure
        // source file.
        if let Some(closure_plan) = materialized.inline_closure_plan.as_ref()
            && outcome.exit_code == 0
            && let Err(failures) = super::inline::try_inline_closure(
                closure_plan,
                &outcome.final_response,
                &prompt_state.source_path,
                child_cwd,
                show_checks,
                term,
            )
        {
            let contexts = claudine::harness::build_validation_failure_context(
                &failures,
                provider.as_slug(),
                plan.source_path.as_path(),
                attempt,
                outcome.session_id.clone(),
                Some(outcome.clone()),
            );
            if let Some(next_plan) = super::resume::try_resolve_handler(
                &contexts,
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                outcome.session_id.as_deref(),
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            let fail_msg = format!(
                "inline closure validation failed ({} {})",
                failures.len(),
                if failures.len() == 1 {
                    "failure"
                } else {
                    "failures"
                }
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            guard.emit_terminal(LifecycleSignal::Failure);
            return Err(eyre!("{fail_msg}"));
        }

        // Evaluate post-checks. In inline mode this now runs against the
        // post-closure document so file-state checks see the final artifact.
        let post_report = info_span!(
            "harness_post_validation",
            attempt,
            rule_count = plan.post_checks.len(),
        )
        .in_scope(|| {
            claudine::harness::evaluate_post_checks(
                &plan,
                &snapshot,
                &outcome,
                Some(&permission_probe),
            )
        });

        if show_checks {
            claudine::harness::report::report_phase_discovery(
                claudine::harness::FailurePhase::PostCheck,
                post_report.count(),
                term,
            );
            claudine::harness::report::report_check_outcomes(&post_report, term);
        }

        if post_report.all_passed() {
            guard.emit_terminal(LifecycleSignal::Success);
            return Ok((outcome.exit_code, harness_perf));
        }

        let failures = post_report.failures();
        let contexts = claudine::harness::build_validation_failure_context(
            &failures,
            provider.as_slug(),
            plan.source_path.as_path(),
            attempt,
            outcome.session_id.clone(),
            Some(outcome.clone()),
        );
        if let Some(next_plan) = super::resume::try_resolve_handler(
            &contexts,
            &plan,
            attempt,
            DEFAULT_MAX_RETRIES,
            profile,
            outcome.session_id.as_deref(),
            &prompt_state.source_path,
            repo_root,
            show_checks,
            term,
        )? {
            attempt = next_plan.next_attempt;
            super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
            continue;
        }
        let fail_msg = format!(
            "post-check validation failed ({} {})",
            failures.len(),
            if failures.len() == 1 {
                "failure"
            } else {
                "failures"
            }
        );
        if show_checks {
            claudine::harness::report::report_unhandled_failure(&fail_msg, term);
        }
        guard.emit_terminal(LifecycleSignal::Failure);
        return Err(eyre!("{fail_msg}"));
    }
}
