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

use biscuit_terminal::terminal::Terminal;
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
use super::profile::{self, WrapperProfile};
use super::{
    HarnessPromptMode, HarnessPromptState, LiveStreamSink, StructuredCodexOutput,
    StructuredSummaryDetails, WrapperHarnessPermissionProbe, build_harness_shell_options,
    emit_stream_summary_no_separator_with_context, emit_stream_summary_with_context,
    materialized_harness_prompt_from_prepared, resolve_binary_path, run_harness_loop,
    strip_prompt_from_args, structured_verbosity, wrap_terminal,
};
use crate::log;

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
    let term = wrap_terminal();
    let cwd = std::env::current_dir()?;
    let verbose_requested = verbose > 0;
    let quiet = request.quiet;
    let silent = request.silent;
    let show_checks = !silent;

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

    let source_repo_root = request.prepared.source_repo_root.as_deref();
    let favorite = load_config_favorite(source_repo_root.unwrap_or(&cwd));

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
        crate::output::log_wrapper_header(
            profile,
            request.yolo,
            effective_non_interactive,
            request.session_interactive, // interactive_override placeholder
            verbose_requested,
            request.repo,
            compose_display.as_ref(),
            request.operation.as_deref(),
            None, // no inline prompt text for compose
            Some(&compose_source_hint),
            &Default::default(), // env_plan not built yet; header doesn't need it for compose
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
        &cwd,
        &[],
        needs_repo_shadow_home,
        needs_mcp_shadow_home || needs_repo_shadow_home,
        source_repo_root,
    )?;

    // -- Operation env override -----------------------------------------------

    if let Some(ref op) = request.operation {
        env_plan.env.insert("OPERATION".into(), op.clone().into());
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
                    let effective_root = source_repo_root.unwrap_or(&cwd);
                    let (shadow_env, shadow_path) =
                        super::repo_home::build_repo_home_env(provider, effective_root, false)?;
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
    let mut stdin_seed: Option<String> = None;

    profile.apply_prompt_body(
        &mut child_args,
        &mut stdin_seed,
        &effective_prompt,
        effective_non_interactive,
    )?;

    // -- Yolo ----------------------------------------------------------------

    if request.yolo {
        let mut env_overrides = Vec::new();
        if let Some(warn) = profile.apply_yolo(&mut child_args, &mut env_overrides)?
            && !silent
            && !quiet
        {
            log::warn(&warn);
        }
        for (key, value) in env_overrides {
            env_plan.env.insert(key.into(), value.into());
        }
        // Note: yolo support already consumed at env_plan build time
    }

    if effective_non_interactive {
        profile.apply_non_interactive(&mut child_args)?;
        // Only apply default model if --model was not explicitly provided.
        if request.model.is_none() {
            profile.apply_non_interactive_defaults(&mut child_args);
        }
    }

    // Universal --model flag
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
        // OpenCode needs MODEL env var when --model is passed via passthrough
        if provider == Provider::OpenCode && effective_non_interactive {
            env_plan.env.insert("MODEL".into(), model.clone().into());
        }
    }

    // Universal --output flag
    if let Some(ref output_str) = request.output {
        use super::profile::OutputFormat;
        let format: OutputFormat = output_str.parse().map_err(|e: String| eyre!(e))?;
        if let Some(warn) = profile.apply_output_format(&mut child_args, format)
            && !silent
            && !quiet
        {
            log::warn(&warn);
        }
    }

    // Universal --system-prompt flag
    if let Some(ref prompt) = request.system_prompt {
        let resolved = super::resolve_system_prompt(prompt)?;
        if let Some(warn) = profile.apply_system_prompt(&mut child_args, &resolved)
            && !silent
            && !quiet
        {
            log::warn(&warn);
        }
    }

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

    let effective_repo_root = source_repo_root.or(env_plan.repo_root.as_deref());
    let child_cwd = effective_repo_root.unwrap_or(&cwd);

    profile.validate_final_args(&child_args, effective_non_interactive, stdin_seed.is_some())?;

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
        );
        return Ok(0);
    }

    // -- Harness detection from effective frontmatter ---------------------
    // THE key architectural fix: harness properties are read from the
    // composed frontmatter, not from raw source state.

    let harness_enabled =
        claudine::harness::has_harness_properties(&request.prepared.effective_frontmatter);

    if harness_enabled {
        let resolve_ctx = claudine::harness::HarnessResolutionContext {
            source_path: &request.prepared.resolved_path,
            repo_root: effective_repo_root,
        };
        let shell_options =
            build_harness_shell_options(&request.prepared.resolved_path, effective_repo_root);
        // Validate that the harness plan can be parsed before proceeding.
        let mut plan = claudine::harness::parse_harness_plan_with_shell(
            &request.prepared.effective_frontmatter,
            &request.prepared.resolved_path,
            &resolve_ctx,
            Some(&shell_options),
        )
        .map_err(|e| eyre!("{e}"))?;

        // For inline composition, prepend a system-owned writability check
        // so that handler recovery paths can respond to permission failures
        // instead of hard-failing before the handler system exists.
        if is_inline {
            plan.pre_checks.insert(
                0,
                claudine::harness::inline_writability_pre_check(&request.prepared.resolved_path),
            );
        }

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
        .map_err(|reason| eyre!("{reason}"))?;
    }

    // -- Structured streaming decision ------------------------------------

    let stdout_noise = if effective_non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    let stderr_noise = profile.stderr_noise_prefixes();

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

    // -- Preflight output (env details + prompt block) ---------------------
    // The header was already emitted early (right after profile lookup).
    // Now emit the env details and prompt block with full env_plan.

    // Detect the environment from the source repo root when available so
    // that git/repo metadata reflects the composition source, not the
    // caller's CWD (which may be in a different repo entirely).
    let env_detect_root = effective_repo_root.unwrap_or(&cwd);
    let env_context = claudine::events::detect_environment_fast(env_detect_root);

    if !silent {
        // Everything below is suppressed by --quiet (consistent with direct-wrap)
        if !quiet {
            if request.session_interactive || verbose_requested {
                crate::output::log_wrapper_env_details(&env_plan, None, &term, verbose);
            }
        }

        // Composed prompt block: always shown unless --silent
        crate::output::log_compose_prompt(&request.prepared.prompt, verbose_requested, &term);

        // Blank line to separate preamble from execution output
        if !quiet {
            crate::log::message("");
        }
    }

    // -- Execution --------------------------------------------------------

    let dispatch_context = composition_dispatch_context(&request, &selected.reason);

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

        let mut harness_base_args = child_args.clone();
        strip_prompt_from_args(provider, &mut harness_base_args);
        if !use_structured {
            profile.prepare_captured_output(&mut harness_base_args);
        }

        run_harness_loop(
            provider,
            profile,
            binary_path.as_path(),
            child_cwd,
            effective_non_interactive,
            request.timeout,
            &harness_base_args,
            &env_plan.env,
            &mut prompt_state,
            effective_repo_root,
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            show_checks,
            stream_verbosity,
            verbose_requested,
            &env_context,
            &dispatch_context,
            Some(materialized_harness_prompt_from_prepared(&request.prepared)),
            &term,
        )
    } else if is_inline {
        let closure_plan = match &request.prepared.closure {
            CompositionClosurePlan::Inline(plan) => plan,
            _ => unreachable!("is_inline is true but closure is not Inline"),
        };
        execute_inline_without_harness(
            provider,
            profile,
            &binary_path,
            &child_args,
            &env_plan.env,
            child_cwd,
            stdin_seed.as_deref(),
            request.session_interactive,
            closure_plan,
            &request.prepared.resolved_path,
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            stream_verbosity,
            verbose_requested,
            show_checks,
            &env_context,
            &dispatch_context,
            &term,
        )
    } else {
        execute_direct_without_harness(
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
            verbose_requested,
            &env_context,
            &dispatch_context,
        )
    }
}

// -- Inline closure execution (non-harness) -------------------------------

#[allow(clippy::too_many_arguments)]
fn execute_inline_without_harness(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &std::path::Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &std::path::Path,
    stdin_seed: Option<&str>,
    session_interactive: bool,
    closure_plan: &InlineClosurePlan,
    resolved_path: &std::path::Path,
    use_structured: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    verbose_requested: bool,
    show_checks: bool,
    env_context: &claudine::events::EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
) -> Result<i32> {
    // Run the provider and capture output.
    let (agent_exit, _agent_termination, final_response, deferred_summary) = if use_structured {
        run_structured_inline(
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
            term,
        )?
    } else {
        run_legacy_inline(
            provider,
            profile,
            binary_path,
            child_args,
            child_env,
            child_cwd,
            stdin_seed,
            session_interactive,
            structured_codex_output,
            stdout_noise,
            stderr_noise,
            term,
        )?
    };

    // -- Post-execution: validate disk state and apply closure -------------

    let mut final_exit = agent_exit;
    let provider_name = crate::output::capitalize_provider(provider);
    let should_separate_checks = deferred_summary
        .as_ref()
        .is_some_and(|(summary, _, _)| !summary.assistant_text.trim().is_empty());

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

    // Interrupted: report partial state and bail
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
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            match claudine::composition::closure::apply_inline_closure(
                closure_plan,
                &replacement_body,
                resolved_path,
                &today,
            ) {
                Ok(()) => {
                    if show_checks {
                        log::message(&crate::output::fm_check_ok(
                            "Applied the captured replacement body to the target document",
                            term,
                        ));
                        log::message(&crate::output::fm_check_ok(
                            "Preserved original frontmatter and updated <bold>last_updated</bold>",
                            term,
                        ));
                    }

                    // Post-processing: run Darkmatter cleanup on the
                    // generated markdown for higher-quality output.
                    match cleanup_inline_output(resolved_path) {
                        Ok(true) => {
                            if show_checks {
                                log::message(&crate::output::fm_check_ok(
                                    "Cleaned up generated markdown formatting",
                                    term,
                                ));
                            }
                        }
                        Ok(false) => {} // no changes needed
                        Err(error) => {
                            if show_checks {
                                log::message(&crate::output::fm_check_fail(
                                    &format!("markdown cleanup failed: {error}"),
                                    term,
                                ));
                            }
                            // Non-fatal: the document was already written
                            // successfully, cleanup is a quality pass.
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

    // Emit deferred metadata summary
    if let Some((summary, details, _)) = deferred_summary {
        if stream_verbosity != Verbosity::Silent {
            eprintln!();
        }
        emit_stream_summary_no_separator_with_context(
            &summary,
            profile,
            env_context,
            stream_verbosity,
            verbose_requested,
            &details,
            Some(dispatch_context),
        );
    } else {
        // Legacy (non-structured) inline path: emit synthetic session-end
        // event so composition metadata is consistently logged.
        emit_legacy_composition_session_event(provider, final_exit, env_context, dispatch_context);
    }

    Ok(final_exit)
}

type InlineRunResult = (
    i32,
    claudine::harness::ProcessTermination,
    String,
    Option<(
        claudine::stream::summary::StreamExecutionSummary,
        StructuredSummaryDetails,
        bool,
    )>,
);

#[allow(clippy::too_many_arguments)]
fn run_structured_inline(
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
    term: &Terminal,
) -> Result<InlineRunResult> {
    let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
    let parser_config = claudine::stream::ParserConfig::default();
    let parser = claudine::stream::create_parser(
        provider,
        LiveStreamSink::new(
            provider,
            env_context.clone(),
            stream_verbosity,
            summary_details.clone(),
        )
        .with_context_extra(dispatch_context.clone()),
        parser_config,
    );
    let stream_result = exec::run_child_stream(
        binary_path,
        child_args,
        child_env,
        child_cwd,
        None,
        stderr_noise,
        profile.suppress_structured_stderr_on_success(),
        stdin_seed,
        parser,
    )?;
    let termination = stream_result.termination;
    let mut summary = stream_result.data;

    let had_streamed_assistant =
        provider != Provider::Codex && !summary.assistant_text.trim().is_empty();
    if let Some(codex_output) = structured_codex_output {
        codex_output.apply_to_summary(&mut summary);
    }
    if !had_streamed_assistant && !summary.assistant_text.trim().is_empty() {
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

    if summary.exit_code == 0 && summary.assistant_text.trim().is_empty() {
        log::warn("the agent did not provide a summarized message on their completed work!");
    }

    let exit = summary.exit_code;
    let details = summary_details.lock().unwrap().clone();
    Ok((
        exit,
        termination,
        summary.assistant_text.clone(),
        Some((summary, details, had_streamed_assistant)),
    ))
}

#[allow(clippy::too_many_arguments)]
fn run_legacy_inline(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &std::path::Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &std::path::Path,
    stdin_seed: Option<&str>,
    session_interactive: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    term: &Terminal,
) -> Result<InlineRunResult> {
    if session_interactive {
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
        )?;
        let final_response = if provider == Provider::Codex {
            if let Some(output) = structured_codex_output {
                let text = std::fs::read_to_string(&output.last_message_path).unwrap_or_default();
                let _ = std::fs::remove_file(&output.last_message_path);
                text
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        Ok((result.data, result.termination, final_response, None))
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
        )?;
        let response = profile.parse_captured_output(&capture.data.stdout);
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
        if !capture.data.stderr.trim().is_empty() {
            eprintln!("{}", capture.data.stderr);
        }
        Ok((capture.data.exit_code, capture.termination, response, None))
    }
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

    let cleaned = darkmatter::markdown::cleanup::cleanup_content(&text);

    if cleaned == text {
        return Ok(false);
    }

    std::fs::write(path, cleaned.as_bytes())
        .map_err(|e| eyre!("failed to write cleaned output to {}: {e}", path.display()))?;

    Ok(true)
}

// -- Direct execution (non-harness) ---------------------------------------

#[allow(clippy::too_many_arguments)]
fn execute_direct_without_harness(
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
    verbose_requested: bool,
    env_context: &claudine::events::EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
) -> Result<i32> {
    if use_structured {
        let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
        let parser_config = claudine::stream::ParserConfig::default();
        let parser = claudine::stream::create_parser(
            provider,
            LiveStreamSink::new(
                provider,
                env_context.clone(),
                stream_verbosity,
                summary_details.clone(),
            )
            .with_context_extra(dispatch_context.clone()),
            parser_config,
        );
        let stream_result = exec::run_child_stream(
            binary_path,
            child_args,
            child_env,
            child_cwd,
            None,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            stdin_seed,
            parser,
        )?;
        let mut summary = stream_result.data;
        if let Some(codex_output) = structured_codex_output {
            codex_output.apply_to_summary(&mut summary);
        }

        // Codex doesn't stream text; render its captured response
        if provider == Provider::Codex && !summary.assistant_text.is_empty() {
            let text = &summary.assistant_text;
            let term = wrap_terminal();
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

        emit_stream_summary_with_context(
            &summary,
            profile,
            env_context,
            stream_verbosity,
            verbose_requested,
            &summary_details.lock().unwrap().clone(),
            dispatch_context,
        );

        Ok(summary.exit_code)
    } else {
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
        )?;

        // Emit a synthetic session-end event for non-structured composition
        // runs so that composition metadata is consistently logged.
        emit_legacy_composition_session_event(provider, result.data, env_context, dispatch_context);

        Ok(result.data)
    }
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
    let config = claudine::dispatch::loader::load_config(None, repo_root.as_deref()).ok()?;
    config.settings.linking?.preference.first().copied()
}

// -- Legacy composition session event --------------------------------------

/// Emit a synthetic `SessionEnd` event for non-structured composition runs.
///
/// Structured runs emit this via `emit_stream_summary_with_context`, but
/// legacy (non-structured) paths return raw process results without any
/// event emission. This ensures composition metadata is consistently logged
/// across all execution paths for future resume/reporting UX.
fn emit_legacy_composition_session_event(
    provider: Provider,
    exit_code: i32,
    env_context: &claudine::events::EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
) {
    use claudine::events::{AgenticEvent, EventMeta};

    let mut extra = HashMap::new();
    extra.insert("synthetic".into(), serde_json::Value::Bool(true));
    extra.insert(
        "synthetic_kind".into(),
        serde_json::Value::String("composition_legacy_summary".into()),
    );
    extra.insert(
        "exit_code".into(),
        serde_json::Value::Number(exit_code.into()),
    );
    for (key, value) in dispatch_context {
        extra.insert(key.clone(), value.clone());
    }

    let meta = EventMeta {
        provider,
        event: AgenticEvent::SessionEnd,
        timestamp: chrono::Utc::now(),
        session_id: None,
        cwd: None,
        tool_name: None,
        tool_input: None,
        tool_response: None,
        error: None,
        prompt: None,
        agent_type: None,
        notification_type: None,
        notification_message: None,
        extra,
        env: env_context.clone(),
    };

    if let Err(e) = claudine::stream::reporting::write_summary_event(&meta) {
        tracing::warn!("Failed to write legacy composition session event: {e}");
    }
}
