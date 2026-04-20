//! Wrapper-grade composition executor.
//!
//! [`execute_composition_request`] is the single execution pipeline for
//! both `claudine compose` and `claudine inline-compose`. It provides
//! full wrapper-grade behavior: environment setup, harness detection from
//! effective (composed) frontmatter, structured streaming, and inline
//! closure.

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::io::{IsTerminal, Write};
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
    InlineClosurePlan, SelectedProvider, SelectionReason, build_candidate_set, select_provider,
};
use claudine::events::Provider;
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

/// Result of executing a single composition step through the wrapper pipeline.
pub(crate) struct SingleCompositionOutcome {
    /// The process exit code.
    pub exit_code: i32,
    /// The provider that ran the step.
    pub provider: Provider,
}

/// Result of running a structured composition stream.
///
/// Produced by [`run_structured_composition`] and consumed by both the
/// compose and inline-compose callers. The shared function does not emit
/// the summary; callers decide the timing and routing.
struct CompositionStreamResult {
    exit_code: i32,
    assistant_text: String,
    summary: claudine::stream::summary::StreamExecutionSummary,
    details: StructuredSummaryDetails,
    had_streamed_assistant: bool,
    /// Shares a `SectionTracker` with the live sink so post-stream trailer
    /// emitters see consistent section state. Only the compose caller uses
    /// this; inline-compose ignores it.
    section_stream: super::section::SectionStream,
}

/// Mode-specific inputs for [`execute_without_harness`].
///
/// Carries the inline-only parameters (closure plan, target path,
/// interactivity, stderr verbosity) so the merged function can branch its
/// post-execution logic without dragging optional parameters through every
/// call.
enum CompositionExecutionMode<'a> {
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

fn resolve_prompt_display_path(path: &std::path::Path, repo_root: Option<&std::path::Path>) -> String {
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
    selection_reason: &SelectionReason,
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
        serde_json::Value::String(format!("{selection_reason:?}")),
    );
    context
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
) -> Result<i32> {
    let outcome = execute_composition_request_inner(request, verbose)?;
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
) -> Result<SingleCompositionOutcome> {
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
    let installed: Vec<Provider> = [
        Provider::Claude,
        Provider::Codex,
        Provider::Gemini,
        Provider::Goose,
        Provider::KimiCode,
        Provider::OpenCode,
        Provider::QwenCode,
    ]
    .into_iter()
    .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
    .collect();

    let favorite = load_config_favorite(source_repo_root.unwrap_or(&launch_cwd));

    let selected = match select_provider(
        request.explicit_provider,
        &request.prepared,
        &installed,
        &request.excluded,
        favorite,
    ) {
        Ok(s) => s,
        Err(CompositionError::InteractiveSelectionRequired) => {
            interactive_select(&installed, &request.excluded)?
        }
        Err(CompositionError::AgentHintAmbiguous { providers, .. }) => {
            if is_tty() {
                interactive_select_from(&providers)?
            } else {
                return Err(eyre!(
                    "agent hint is ambiguous and no TTY available for interactive selection"
                ));
            }
        }
        Err(e) => return Err(eyre!("{e}")),
    };

    let provider = selected.provider;
    let selection_reason = selected.reason;
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
                request.model.as_deref(),
                effective_non_interactive,
                &super::profile::OpenCodeEnvSnapshot::from_system(),
            )?
        } else {
            None
        };

    // Universal --model flag (non-OpenCode providers, and OpenCode interactive).
    if provider != Provider::OpenCode {
        if let Some(ref model) = request.model {
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
    } else if let Some(ref model) = request.model
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
    let stdin_seed = profile
        .prompt_delivery(&child_args, &effective_prompt, effective_non_interactive)?
        .apply_to(&mut child_args);

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
        return Ok(SingleCompositionOutcome {
            exit_code: 0,
            provider,
        });
    }

    switch_process_cwd(child_cwd)?;

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

    // -- Execution --------------------------------------------------------

    let dispatch_context = composition_dispatch_context(&request, &selection_reason);

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
        let exit_code = run_harness_loop(
            provider,
            profile,
            binary_path.as_path(),
            child_cwd,
            effective_non_interactive,
            request.timeout,
            request.step_timeout,
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
        Ok(SingleCompositionOutcome {
            exit_code,
            provider,
        })
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

        let mut child_spawned = false;
        let exit_result = execute_without_harness(
            mode,
            provider,
            profile,
            &binary_path,
            &child_args,
            &env_plan.env,
            child_cwd,
            stdin_seed.as_deref(),
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

        Ok(SingleCompositionOutcome {
            exit_code,
            provider,
        })
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
) -> Result<i32> {
    let is_inline = matches!(mode, CompositionExecutionMode::Inline { .. });

    // -- Run the provider --------------------------------------------------
    let (agent_exit, final_response, deferred_summary) = if use_structured {
        let result = run_structured_composition(
            provider,
            profile,
            binary_path,
            child_args,
            child_env,
            child_cwd,
            stdin_seed,
            structured_codex_output,
            stderr_noise,
            stream_verbosity,
            env_context,
            dispatch_context,
            child_spawned,
            prompt_timing,
        )?;

        // Render assistant text to stdout when the provider did not stream
        // it live. Compose routes through the section stream so the trailer
        // summary sees a consistent section state; inline writes directly
        // because the body will be captured into the target file.
        if !result.had_streamed_assistant && !result.summary.assistant_text.trim().is_empty() {
            if !is_inline {
                result.section_stream.enter_final_stdout();
            }
            let text = &result.summary.assistant_text;
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

        let exit = result.exit_code;
        let response = result.assistant_text.clone();
        (exit, response, Some(result))
    } else {
        // Legacy (non-structured) path. Inline must capture the response so
        // the closure plan can rewrite the target file; direct just runs the
        // child and lets it write to stdout on its own.
        match &mode {
            CompositionExecutionMode::Inline {
                session_interactive,
                ..
            } => {
                if *session_interactive {
                    let result = exec::run_child(
                        binary_path,
                        child_args,
                        child_env,
                        child_cwd,
                        None,
                        exec::ChildIoOptions {
                            stdout_noise_prefixes: stdout_noise,
                            stderr_noise_prefixes: stderr_noise,
                            stdin_seed,
                        },
                        child_spawned,
                    )?;
                    let response = if provider == Provider::Codex {
                        if let Some(output) = structured_codex_output {
                            let text = std::fs::read_to_string(&output.last_message_path)
                                .unwrap_or_default();
                            let _ = std::fs::remove_file(&output.last_message_path);
                            text
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    (result.data, response, None)
                } else {
                    let mut capture_args = child_args.to_vec();
                    profile.prepare_captured_output(&mut capture_args);
                    let capture = exec::run_child_capture(
                        binary_path,
                        &capture_args,
                        child_env,
                        child_cwd,
                        None,
                        exec::ChildIoOptions {
                            stdout_noise_prefixes: stdout_noise,
                            stderr_noise_prefixes: stderr_noise,
                            stdin_seed,
                        },
                        child_spawned,
                    )?;
                    let response = profile.parse_captured_output(&capture.data.stdout);
                    if !response.trim().is_empty() {
                        if std::io::stdout().is_terminal() {
                            let rendered =
                                crate::output::render_assistant_markdown(&response, term);
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
                    if !capture.data.stderr.trim().is_empty() {
                        eprintln!("{}", capture.data.stderr);
                    }
                    (capture.data.exit_code, response, None)
                }
            }
            CompositionExecutionMode::Direct => {
                let result = exec::run_child(
                    binary_path,
                    child_args,
                    child_env,
                    child_cwd,
                    None,
                    exec::ChildIoOptions {
                        stdout_noise_prefixes: stdout_noise,
                        stderr_noise_prefixes: stderr_noise,
                        stdin_seed,
                    },
                    child_spawned,
                )?;
                (result.data, String::new(), None)
            }
        }
    };

    // -- Direct mode: emit summary immediately and return -------------------
    let (closure_plan, resolved_path, show_checks) = match mode {
        CompositionExecutionMode::Direct => {
            if let Some(result) = deferred_summary {
                emit_composition_summary(
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
                emit_minimal_composition_summary(
                    provider,
                    agent_exit,
                    profile,
                    env_context,
                    dispatch_context,
                );
            }
            return Ok(agent_exit);
        }
        CompositionExecutionMode::Inline {
            closure_plan,
            resolved_path,
            show_checks,
            ..
        } => (closure_plan, resolved_path, show_checks),
    };

    // -- Inline mode: validate disk state and apply closure -----------------
    let mut final_exit = agent_exit;
    let provider_name = crate::output::capitalize_provider(provider);
    let should_separate_checks = deferred_summary
        .as_ref()
        .is_some_and(|result| !result.summary.assistant_text.trim().is_empty());

    if show_checks && should_separate_checks {
        eprintln!();
        eprintln!();
    }

    let was_interrupted = agent_exit == 130 || agent_exit == 143;

    if was_interrupted && show_checks {
        log::message(&crate::output::fm_check_fail(
            &format!("{provider_name} agent was interrupted by the user (code {agent_exit})"),
            term,
        ));
    } else if agent_exit == 0 && show_checks {
        log::message(&crate::output::fm_check_ok(
            &format!("{provider_name} agent completed successfully"),
            term,
        ));
    } else if agent_exit != 0 && show_checks {
        log::message(&crate::output::fm_check_fail(
            &format!("{provider_name} agent exited with error (code {agent_exit})"),
            term,
        ));
    }

    let display_path = resolved_path
        .strip_prefix(child_cwd)
        .unwrap_or(resolved_path)
        .display();

    if was_interrupted {
        report_interruption(&display_path, final_response.trim(), term);
        return Ok(1);
    }

    if agent_exit == 0 {
        let replacement_body = match claudine::composition::closure::extract_replacement_body(
            &final_response,
        ) {
            Ok(body) => body,
            Err(error) => {
                if show_checks {
                    log::message(&crate::output::fm_check_fail(
                        &format!(
                            "the referenced file -- {display_path} -- did not receive a valid replacement body: {error}"
                        ),
                        term,
                    ));
                }
                final_exit = 1;
                String::new()
            }
        };

        if final_exit == 0 {
            let post_run_fm = std::fs::read_to_string(resolved_path).ok().map(|text| {
                let md: darkmatter::markdown::Markdown = text.into();
                md.frontmatter().as_map().clone()
            });

            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            match claudine::composition::closure::apply_inline_closure(
                closure_plan,
                &replacement_body,
                resolved_path,
                &today,
                post_run_fm.as_ref(),
            ) {
                Ok(result) => {
                    if show_checks {
                        log::message(&crate::output::fm_check_ok(
                            "Applied the captured replacement body to the target document",
                            term,
                        ));
                        log::message(&crate::output::fm_check_ok(
                            "Preserved original frontmatter and updated <bold>last_updated</bold>",
                            term,
                        ));

                        for key in &result.new_properties {
                            log::message(&crate::output::fm_check_ok(
                                &format!("Merged new frontmatter property <bold>\"{key}\"</bold>"),
                                term,
                            ));
                        }

                        for key in &result.reverted_properties {
                            let status = Status::from_prose(format!(
                                "Agent modified frontmatter property <b>\"{key}\"</b> — reverted to original value"
                            ))
                            .state(StatusState::Warning);
                            log::message(&status.render(term));
                        }
                    }

                    match cleanup_inline_output(resolved_path) {
                        Ok(true) => {
                            if show_checks {
                                log::message(&crate::output::fm_check_ok(
                                    "Cleaned up generated markdown formatting",
                                    term,
                                ));
                            }
                        }
                        Ok(false) => {}
                        Err(error) => {
                            if show_checks {
                                log::message(&crate::output::fm_check_fail(
                                    &format!("markdown cleanup failed: {error}"),
                                    term,
                                ));
                            }
                        }
                    }
                }
                Err(error) => {
                    if show_checks {
                        log::message(&crate::output::fm_check_fail(
                            &format!("failed to rewrite {display_path}: {error}"),
                            term,
                        ));
                    }
                    final_exit = 1;
                }
            }
        }
    }

    if let Some(result) = deferred_summary {
        if stream_verbosity != Verbosity::Silent {
            eprintln!();
        }
        emit_composition_summary(
            &result.summary,
            &result.details,
            profile,
            env_context,
            stream_verbosity,
            detail_requested,
            dispatch_context,
            None,
            true,
        );
    } else {
        emit_minimal_composition_summary(
            provider,
            final_exit,
            profile,
            env_context,
            dispatch_context,
        );
    }

    Ok(final_exit)
}

/// Run a provider through the structured stream pipeline shared by both
/// `compose` and `inline-compose`.
///
/// The function builds the live semantic sink, runs the child process, and
/// applies any Codex-captured post-hoc text to the resulting summary. It
/// does not emit the summary and does not render assistant text to stdout;
/// callers decide both timing and section-stream routing.
#[allow(clippy::too_many_arguments)]
fn run_structured_composition(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &std::path::Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &std::path::Path,
    stdin_seed: Option<&str>,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    env_context: &claudine::events::EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    child_spawned: &mut bool,
    prompt_timing: Option<claudine::stream::prompt_timing::PromptTimingContext>,
) -> Result<CompositionStreamResult> {
    let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
    let parser_config = claudine::stream::ParserConfig::default();
    let sink = LiveSemanticSink::with_default_wiring(
        provider,
        env_context.clone(),
        child_cwd,
        stream_verbosity,
        summary_details.clone(),
    )
    .with_context_extra(dispatch_context.clone());
    let live_metrics = sink.live_metrics();
    let stream_output = sink.stream_output();
    let section_stream = sink.section_stream();
    let (build_parser, stderr_bridge) =
        super::build_structured_plumbing(provider, sink, parser_config);
    let stream_result = exec::run_child_stream_semantic(
        binary_path,
        child_args,
        child_env,
        child_cwd,
        None,
        None,
        stderr_noise,
        profile.suppress_structured_stderr_on_success(),
        stream_verbosity != Verbosity::Silent,
        stdin_seed,
        build_parser,
        child_spawned,
        live_metrics,
        stream_output,
        stderr_bridge,
        prompt_timing,
    )?;
    let mut summary = stream_result.data;

    let had_streamed_assistant =
        provider != Provider::Codex && !summary.assistant_text.trim().is_empty();
    if let Some(codex_output) = structured_codex_output {
        codex_output.apply_to_summary(&mut summary);
    }

    let details = summary_details.lock().unwrap().clone();
    Ok(CompositionStreamResult {
        exit_code: summary.exit_code,
        assistant_text: summary.assistant_text.clone(),
        summary,
        details,
        had_streamed_assistant,
        section_stream,
    })
}

/// Emit the structured-stream summary trailer for a composition run.
///
/// Unifies the two previous paths:
/// - `defer_section_separator = false` (compose): routes through
///   [`emit_stream_summary_with_context`] so the section tracker inserts the
///   separator blank exactly once.
/// - `defer_section_separator = true` (inline-compose): prints the trailer
///   directly so the caller controls the blank-line spacing (e.g. around
///   closure validation output).
///
/// Both paths write the JSONL summary event.
#[allow(clippy::too_many_arguments)]
fn emit_composition_summary(
    summary: &claudine::stream::summary::StreamExecutionSummary,
    details: &StructuredSummaryDetails,
    profile: &dyn WrapperProfile,
    env_context: &claudine::events::EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    dispatch_context: &HashMap<String, serde_json::Value>,
    section_stream: Option<&super::section::SectionStream>,
    defer_section_separator: bool,
) {
    if defer_section_separator {
        use biscuit_terminal::components::prose::Prose;

        if verbosity != Verbosity::Silent
            && let Some(markup) = format_summary_prose(summary)
        {
            let term = crate::log::terminal();
            let rendered = Prose::new(markup).render(&term);
            eprintln!("{rendered}");
        }
        if verbosity != Verbosity::Silent
            && verbose
            && let Some(markup) = format_verbose_summary_details_prose(summary, details)
        {
            let term = crate::log::terminal();
            let rendered = Prose::new(markup).render(&term);
            eprintln!("  {rendered}");
        }

        if let Some(protocol) = profile.stream_protocol() {
            let meta = claudine::stream::reporting::summary_to_event_meta_with_context(
                summary,
                protocol,
                env_context,
                Some(dispatch_context),
            );
            if let Err(e) = claudine::stream::reporting::write_summary_event(&meta) {
                tracing::warn!("Failed to write stream summary event: {e}");
            }
        }
    } else {
        emit_stream_summary_with_context(
            StreamSummaryContext {
                summary,
                profile,
                env_context,
                verbosity,
                verbose,
                details,
                section_stream,
            },
            dispatch_context,
        );
    }
}

/// Emit a minimal composition summary for legacy (non-structured) runs.
///
/// Builds a minimal [`StreamExecutionSummary`] (exit_code, is_error, provider)
/// and routes through [`emit_composition_summary`] with no section stream and
/// deferred section separators. This delivers the same stderr trailer and
/// JSONL event that structured runs emit, so legacy paths reach full parity
/// with structured runs.
fn emit_minimal_composition_summary(
    provider: Provider,
    exit_code: i32,
    profile: &dyn WrapperProfile,
    env_context: &claudine::events::EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
) {
    let summary = claudine::stream::summary::StreamExecutionSummary {
        provider,
        exit_code,
        is_error: exit_code != 0,
        ..Default::default()
    };
    let details = StructuredSummaryDetails::default();
    emit_composition_summary(
        &summary,
        &details,
        profile,
        env_context,
        Verbosity::Normal,
        false,
        dispatch_context,
        None,
        true,
    );
}

fn report_interruption(
    display_path: &std::path::Display<'_>,
    captured_body: &str,
    term: &Terminal,
) {
    if captured_body.is_empty() {
        log::message(&crate::output::fm_check_fail(
            &format!(
                "<b>User interrupted the agent with CTRL+C; the body of \
                 <blue-500>{display_path}</blue-500> is empty so it appears \
                 no work was accomplished.</b>"
            ),
            term,
        ));
    } else {
        log::message(&crate::output::fm_check_fail(
            &format!(
                "<b>User interrupted the agent with CTRL+C; the body of \
                 <blue-500>{display_path}</blue-500> has been at least \
                 partially filled:</b>"
            ),
            term,
        ));
        eprintln!();
        for line in captured_body.lines() {
            eprintln!("  {line}");
        }
    }
}

// -- Post-processing: Darkmatter cleanup ----------------------------------

/// Run Darkmatter's cleanup pass over a written inline composition file.
///
/// Reads the file, applies `cleanup_content` to the body (preserving
/// frontmatter), and writes back only if the content changed.
///
/// Returns `Ok(true)` when the file was updated, `Ok(false)` when no
/// changes were needed.
fn cleanup_inline_output(path: &std::path::Path) -> Result<bool> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| eyre!("failed to read {}: {e}", path.display()))?;

    // Split frontmatter from body so cleanup operates only on the body,
    // preserving frontmatter (including YAML block scalars) byte-for-byte.
    let (frontmatter_prefix, body) = split_frontmatter_and_body(&text);

    let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(body);

    if cleaned_body == body {
        return Ok(false);
    }

    let mut output = String::with_capacity(frontmatter_prefix.len() + cleaned_body.len());
    output.push_str(frontmatter_prefix);
    output.push_str(&cleaned_body);

    std::fs::write(path, output.as_bytes())
        .map_err(|e| eyre!("failed to write cleaned output to {}: {e}", path.display()))?;

    Ok(true)
}

/// Split text into a frontmatter prefix (including closing delimiter) and the body.
///
/// If the text starts with `---\n`, scans for the closing `---\n` and returns
/// everything up to and including that line as the prefix. Otherwise returns
/// an empty prefix and the full text as body.
fn split_frontmatter_and_body(text: &str) -> (&str, &str) {
    let mut lines = text.split_inclusive('\n');
    let first = match lines.next() {
        Some(l) => l,
        None => return ("", text),
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return ("", text);
    }

    let mut offset = first.len();
    for line in lines {
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return (&text[..offset], &text[offset..]);
        }
    }

    // No closing delimiter — treat entire text as body
    ("", text)
}

// -- Interactive provider selection ---------------------------------------

fn interactive_select(
    installed: &[Provider],
    excluded: &BTreeSet<Provider>,
) -> Result<SelectedProvider> {
    if !is_tty() {
        return Err(eyre!(
            "interactive provider selection required but no TTY available; \
             use an explicit provider flag (--claude, --codex, etc.) or \
             add an `agent` frontmatter property"
        ));
    }

    let candidates = build_candidate_set(installed, excluded);
    if candidates.is_empty() {
        return Err(eyre!("no runnable providers available"));
    }

    interactive_select_from(&candidates)
}

fn interactive_select_from(candidates: &[Provider]) -> Result<SelectedProvider> {
    let options: Vec<String> = candidates.iter().map(|p| p.to_string()).collect();
    let selection = Select::new("Choose a provider:", options)
        .prompt()
        .map_err(|e| eyre!("selection cancelled: {e}"))?;

    let provider = candidates
        .iter()
        .find(|p| p.to_string() == selection)
        .copied()
        .ok_or_else(|| eyre!("invalid selection"))?;

    Ok(SelectedProvider {
        provider,
        reason: SelectionReason::InteractiveChoice,
    })
}

fn is_tty() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

// -- Config loading -------------------------------------------------------

fn load_config_favorite(cwd: &Path) -> Option<Provider> {
    let repo_root = sniff::filesystem::git::detect_git(cwd, false, 1)
        .ok()
        .flatten()
        .map(|info| info.repo_root);
    let config =
        claudine::dispatch::loader::load_claudine_config(None, repo_root.as_deref()).ok()?;
    Some(config.preferred_agent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_basic() {
        let text = "---\ntitle: Test\n---\n# Body\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "---\ntitle: Test\n---\n");
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn split_frontmatter_block_scalar() {
        let text = concat!(
            "---\n",
            "prompt: |-\n",
            "    First line\n",
            "\n",
            "    - bullet\n",
            "last_updated: 2026-03-18\n",
            "---\n",
            "# Body\n",
        );
        let (prefix, body) = split_frontmatter_and_body(text);
        assert!(prefix.ends_with("---\n"));
        assert!(prefix.contains("prompt: |-"));
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn split_frontmatter_no_frontmatter() {
        let text = "# Just a heading\n\nContent\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "");
        assert_eq!(body, text);
    }

    #[test]
    fn split_frontmatter_unclosed() {
        let text = "---\ntitle: Test\nNo closing\n";
        let (prefix, body) = split_frontmatter_and_body(text);
        assert_eq!(prefix, "");
        assert_eq!(body, text);
    }

    #[test]
    fn cleanup_preserves_frontmatter_block_scalar() {
        // Reproduces the bug: cleanup_content on full text corrupts YAML
        // block scalar indentation. The fix splits frontmatter from body
        // so cleanup only operates on the body.
        let frontmatter = concat!(
            "---\n",
            "prompt: |-\n",
            "    First line of prompt\n",
            "\n",
            "    - bullet one\n",
            "    - bullet two\n",
            "\n",
            "    Final paragraph\n",
            "last_updated: 2026-03-18\n",
            "---\n",
        );
        let body = "# Body\n\nSome content\n";
        let text = format!("{frontmatter}{body}");

        let (prefix, body_part) = split_frontmatter_and_body(&text);

        // Frontmatter must be preserved byte-for-byte
        assert_eq!(prefix, frontmatter);

        // Cleaning only the body should not corrupt frontmatter
        let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(body_part);
        let result = format!("{prefix}{cleaned_body}");

        // The frontmatter portion must remain unchanged
        assert!(result.starts_with(frontmatter));
    }
}
