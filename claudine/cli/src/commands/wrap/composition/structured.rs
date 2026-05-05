use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use biscuit_terminal::terminal::Terminal;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::Result;

use crate::commands::wrap::exec;
use crate::commands::wrap::live_semantic_sink::LiveSemanticSink;
use crate::commands::wrap::profile::WrapperProfile;
use super::{CompositionStreamResult, StructuredCodexOutput, StructuredSummaryDetails};
use crate::commands::wrap::subagent_watchdog::TimeoutConfig;

/// Run a provider through the structured stream pipeline shared by both
/// `compose` and `inline-compose`.
///
/// The function builds the live semantic sink, runs the child process, and
/// applies any Codex-captured post-hoc text to the resulting summary. It
/// does not emit the summary and does not render assistant text to stdout;
/// callers decide both timing and section-stream routing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_structured_composition(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &Path,
    stdin_seed: Option<&str>,
    wire_prompt: Option<&str>,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    child_spawned: &mut bool,
    prompt_timing: Option<claudine::stream::prompt_timing::PromptTimingContext>,
    timeout_config: TimeoutConfig,
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
    let watchdog_state = Some(sink.watchdog_state());
    let section_stream = sink.section_stream();
    let (build_parser, stderr_bridge) =
        crate::commands::wrap::build_structured_plumbing(provider, sink, parser_config);
    let stream_result = if let Some(wire_prompt) = wire_prompt {
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
        let _ = stdin_seed;
        crate::commands::wrap::wire_io::run_kimi_wire_session(
            crate::commands::wrap::wire_io::WireSessionConfig {
                binary: binary_path,
                args: child_args,
                env: child_env,
                cwd: child_cwd,
                prompt: wire_prompt.to_string(),
                timeout: timeout_config.timeout,
                client_name: env!("CARGO_PKG_NAME"),
                client_version: env!("CARGO_PKG_VERSION"),
                capabilities: crate::commands::wrap::wire_io::WireClientCapabilities::default_for_claudine(),
                env_context: env_context.clone(),
            },
            crate::commands::wrap::wire_io::WireSessionWiring {
                build_parser,
                stream_output,
                live_metrics,
                runtime_context,
            },
            child_spawned,
        )?
    } else {
        exec::run_child_stream_semantic(
            binary_path,
            child_args,
            child_env,
            child_cwd,
            timeout_config,
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
            watchdog_state,
            Some(section_stream.tracker()),
        )?
    };
    let telemetry = stream_result.telemetry;
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
        telemetry,
    })
}

/// Run the structured branch of [`super::execute_without_harness`].
///
/// Delegates to [`run_structured_composition`] and handles post-hoc assistant
/// text rendering. Returns the exit code, assistant text, and the full stream
/// result so the caller can emit the summary at the right time.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_structured_branch(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    child_env: &std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>,
    child_cwd: &Path,
    stdin_seed: Option<&str>,
    wire_prompt: Option<&str>,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    child_spawned: &mut bool,
    prompt_timing: Option<claudine::stream::prompt_timing::PromptTimingContext>,
    timeout_config: TimeoutConfig,
    is_inline: bool,
    agent_perf_out: &mut Option<crate::perf::AgentExecutionPerf>,
    term: &Terminal,
) -> Result<(i32, String, Option<CompositionStreamResult>)> {
    let result = run_structured_composition(
        provider,
        profile,
        binary_path,
        child_args,
        child_env,
        child_cwd,
        stdin_seed,
        wire_prompt,
        structured_codex_output,
        stderr_noise,
        stream_verbosity,
        env_context,
        dispatch_context,
        child_spawned,
        prompt_timing,
        timeout_config,
    )?;
    *agent_perf_out = Some(result.telemetry.into_agent_perf(result.summary.duration_ms));

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
    Ok((exit, response, Some(result)))
}
