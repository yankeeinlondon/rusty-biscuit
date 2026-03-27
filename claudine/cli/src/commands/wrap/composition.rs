//! Wrapper-grade composition executor.
//!
//! [`execute_composition_request`] is the single execution pipeline for
//! both `claudine compose` and `claudine inline-compose`. It provides
//! full wrapper-grade behavior: environment setup, harness detection from
//! effective (composed) frontmatter, structured streaming, and inline
//! closure.


use std::collections::BTreeSet;
use std::io::{IsTerminal, Write};
use std::sync::{Arc, Mutex};

use biscuit_terminal::terminal::Terminal;
use claudine::composition::{
    CompositionClosurePlan, CompositionError, CompositionExecutionRequest, InlineClosurePlan,
    SelectedProvider, SelectionReason, build_candidate_set, select_provider,
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
    StructuredSummaryDetails, build_harness_shell_options, emit_stream_summary,
    emit_stream_summary_no_separator, rewrite_markdown_preserving_frontmatter,
    resolve_binary_path, run_harness_loop, strip_prompt_from_args, structured_verbosity,
    wrap_terminal,
};
use crate::log;

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

    let favorite = load_config_favorite();

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

    // -- Inline + interactive check ---------------------------------------

    if request.session_interactive && is_inline {
        return Err(
            CompositionError::InlineInteractiveUnsupported(provider.to_string()).into(),
        );
    }

    // -- Profile, binary, arguments, environment --------------------------

    let profile = profile::profile_for_provider(provider)
        .ok_or_else(|| eyre!("'{}' cannot be wrapped", provider))?;
    let binary_path = resolve_binary_path(profile, &clients)?;

    let effective_non_interactive = !request.session_interactive;
    let mut child_args = Vec::new();
    let mut stdin_seed: Option<String> = None;

    profile.apply_prompt_body(
        &mut child_args,
        &mut stdin_seed,
        &request.prepared.prompt,
        effective_non_interactive,
    )?;

    if effective_non_interactive {
        profile.apply_non_interactive(&mut child_args)?;
        profile.apply_non_interactive_defaults(&mut child_args);
    }

    let env_plan = env::build_child_env(
        profile,
        provider,
        &[],                         // no include overrides
        false,                       // no yolo
        request.session_interactive, // interactive
        &[],                         // no raw agent params
        &cwd,
        &[],                         // no env overrides
        false,                       // no repo mode
        false,                       // no mcp shadow home
    )?;

    let child_cwd = env_plan.repo_root.as_deref().unwrap_or(&cwd);

    profile.validate_final_args(&child_args, effective_non_interactive, stdin_seed.is_some())?;

    // -- Harness detection from effective frontmatter ---------------------
    // THE key architectural fix: harness properties are read from the
    // composed frontmatter, not from raw source state.

    let harness_enabled =
        claudine::harness::has_harness_properties(&request.prepared.effective_frontmatter);

    if harness_enabled {
        let resolve_ctx = claudine::harness::HarnessResolutionContext {
            source_path: &request.prepared.resolved_path,
            repo_root: env_plan.repo_root.as_deref(),
        };
        let shell_options = build_harness_shell_options(
            &request.prepared.resolved_path,
            env_plan.repo_root.as_deref(),
        );
        // Validate that the harness plan can be parsed before proceeding.
        claudine::harness::parse_harness_plan_with_shell(
            &request.prepared.effective_frontmatter,
            &request.prepared.resolved_path,
            &resolve_ctx,
            Some(&shell_options),
        )
        .map_err(|e| eyre!("{e}"))?;
    }

    // -- Structured streaming decision ------------------------------------

    let stdout_noise = if effective_non_interactive {
        profile.stdout_noise_prefixes()
    } else {
        &[]
    };
    let stderr_noise = profile.stderr_noise_prefixes();

    let use_structured = profile.supports_structured_stream() && effective_non_interactive;
    let stream_verbosity = structured_verbosity(silent, false);

    if use_structured {
        profile.apply_structured_stream(&mut child_args);
    }

    let structured_codex_output = if use_structured && provider == Provider::Codex {
        Some(StructuredCodexOutput::prepare(&mut child_args))
    } else {
        None
    };

    // -- Preflight output -------------------------------------------------

    let env_context = claudine::events::detect_environment_fast(&cwd);

    if !silent {
        eprintln!(
            "  Using {} ({})",
            provider,
            reason_label(selected.reason)
        );
        eprintln!();
    }

    // -- Execution --------------------------------------------------------

    if harness_enabled {
        let harness_mode = if is_inline {
            HarnessPromptMode::Inline
        } else {
            HarnessPromptMode::Compose
        };

        let mut prompt_state = HarnessPromptState {
            mode: harness_mode,
            source_path: request.prepared.resolved_path.clone(),
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
            None,
            &harness_base_args,
            &env_plan.env,
            &mut prompt_state,
            env_plan.repo_root.as_deref(),
            use_structured,
            structured_codex_output.as_ref(),
            stdout_noise,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            show_checks,
            stream_verbosity,
            verbose_requested,
            &env_context,
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
    term: &Terminal,
) -> Result<i32> {
    // Run the provider and capture output.
    let (agent_exit, _agent_termination, deferred_summary) = if use_structured {
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
            term,
        )?
    } else {
        run_legacy_inline(
            binary_path,
            child_args,
            child_env,
            child_cwd,
            stdin_seed,
            stdout_noise,
            stderr_noise,
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
        report_interruption(resolved_path, &display_path, term);
        return Ok(1);
    }

    // Read the file from disk to see what the agent did
    match std::fs::read_to_string(resolved_path) {
        Ok(disk_text) => {
            let on_disk: darkmatter::markdown::Markdown = disk_text.clone().into();
            let disk_body_hash = on_disk.hash_body(false);
            let body_updated = disk_body_hash != closure_plan.original_body_hash;

            if body_updated {
                if show_checks {
                    log::message(&crate::output::fm_check_ok(
                        "Agent updated the target document's body",
                        term,
                    ));
                }

                if agent_exit != 0 && show_checks {
                    log::warn(
                        "agent reported an error but the target file was updated; \
                         treating as success",
                    );
                }
                final_exit = 0;

                // Check for frontmatter tamper; rewrite preserving original if tampered
                let disk_fm_hash = on_disk.hash_frontmatter(false);
                let fm_tampered = disk_fm_hash != closure_plan.original_frontmatter_hash;

                let frontmatter_source = if fm_tampered {
                    if show_checks {
                        log::message(&crate::output::fm_check_fail(
                            "Agent ignored instruction to leave frontmatter untouched \
                             (<i>we have reverted their changes</i>)",
                            term,
                        ));
                    }
                    &closure_plan.original_document_text
                } else {
                    if show_checks {
                        log::message(&crate::output::fm_check_ok(
                            "Agent left frontmatter untouched (<i>as instructed</i>)",
                            term,
                        ));
                    }
                    &disk_text
                };

                let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                let doc_string = rewrite_markdown_preserving_frontmatter(
                    frontmatter_source,
                    on_disk.content(),
                    &today,
                )
                .map_err(|e| eyre!("failed to reconstruct document: {e}"))?;

                claudine::config::atomic::atomic_write(resolved_path, doc_string.as_bytes())
                    .map_err(|e| eyre!("failed to write: {e}"))?;

                if show_checks {
                    log::message(&crate::output::fm_check_ok(
                        "Updated <bold>last_updated</bold> property to today's date",
                        term,
                    ));
                }
            } else if agent_exit == 0 {
                if show_checks {
                    log::message(&crate::output::fm_check_fail(
                        &format!(
                            "the referenced file -- {display_path} -- did not get \
                             updated even though the Agent reported a successful outcome!"
                        ),
                        term,
                    ));
                }
                final_exit = 1;
            }
        }
        Err(e) => {
            log::error(&format!(
                "failed to read {display_path} after agent completion: {e}"
            ));
            final_exit = 1;
        }
    }

    // Emit deferred metadata summary
    if let Some((summary, details, _)) = deferred_summary {
        if stream_verbosity != Verbosity::Silent {
            eprintln!();
        }
        emit_stream_summary_no_separator(
            &summary,
            profile,
            env_context,
            stream_verbosity,
            verbose_requested,
            &details,
        );
    }

    Ok(final_exit)
}

type InlineRunResult = (
    i32,
    claudine::harness::ProcessTermination,
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
        ),
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
        Some((summary, details, had_streamed_assistant)),
    ))
}

#[allow(clippy::too_many_arguments)]
fn run_legacy_inline(
    binary_path: &std::path::Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &std::path::Path,
    stdin_seed: Option<&str>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
) -> Result<InlineRunResult> {
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
    Ok((result.data, result.termination, None))
}

fn report_interruption(
    resolved_path: &std::path::Path,
    display_path: &std::path::Display<'_>,
    term: &Terminal,
) {
    let body_on_disk = std::fs::read_to_string(resolved_path)
        .ok()
        .map(|text| {
            let md: darkmatter::markdown::Markdown = text.into();
            md.content().trim().to_string()
        })
        .unwrap_or_default();

    if body_on_disk.is_empty() {
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
        for line in body_on_disk.lines() {
            eprintln!("  {line}");
        }
    }
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
            ),
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

        emit_stream_summary(
            &summary,
            profile,
            env_context,
            stream_verbosity,
            verbose_requested,
            &summary_details.lock().unwrap().clone(),
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

fn load_config_favorite() -> Option<Provider> {
    let config = claudine::dispatch::loader::load_config(None, None).ok()?;
    config.settings.linking?.preference.first().copied()
}

// -- Display helpers ------------------------------------------------------

pub(crate) fn reason_label(reason: SelectionReason) -> &'static str {
    match reason {
        SelectionReason::ExplicitProvider => "explicit",
        SelectionReason::SingleInstalled => "only installed",
        SelectionReason::FrontmatterHint => "frontmatter hint",
        SelectionReason::ConfigFavorite => "config favorite",
        SelectionReason::InteractiveChoice => "interactive choice",
    }
}
