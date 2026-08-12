//! Structured-stream execution arm for the direct provider wrapper.
//!
//! Lifted verbatim from `run_provider_wrapper_inner`'s execution dispatch:
//! spawns the provider through the semantic stream pipeline (or the Kimi
//! wire transport), records perf, emits the structured summary, and returns
//! the `(exit_code, captured_stderr)` pair. Behavior is identical to the
//! inline arm it replaced.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use biscuit_terminal::terminal::Terminal;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::Result;

use super::flags::WrapperArgs;
use super::policy::StructuredSummaryDetails;
use super::profile::WrapperProfile;
use super::{StructuredCodexOutput, composition, env, exec, live_semantic_sink, policy, wire_io};

/// Run the provider through the structured-stream pipeline and return the
/// `(exit_code, captured_stderr)` pair.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_structured_stream_session(
    args: &WrapperArgs,
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &std::path::Path,
    child_args: &[String],
    env_plan: &env::EnvPlan,
    child_cwd: &std::path::Path,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    wire_prompt: Option<String>,
    stdin_seed: Option<&str>,
    cli_step_timeout: Option<String>,
    stderr_noise: &[&str],
    term: &Terminal,
    wrapper_span: &tracing::Span,
    mut perf_collector: Option<&mut crate::perf::CommandPerfCollector>,
    status_reporter: super::session_report::StatusReporter,
) -> Result<(i32, Option<String>)> {
    let summary_details = Arc::new(Mutex::new(StructuredSummaryDetails::default()));
    let parser_config = claudine::stream::ParserConfig {
        model: args.model.clone(),
    };
    let mut sink = live_semantic_sink::LiveSemanticSink::with_default_wiring(
        provider,
        env_context.clone(),
        child_cwd,
        stream_verbosity,
        summary_details.clone(),
        // A direct wrapper run is never a sequence task; nothing to attribute.
        None,
    )
    .with_context_extra(dispatch_context.clone())
    .with_status_reporter(status_reporter);
    // Arm the runaway-output content detector (Phase 6). Direct wrappers
    // have no frontmatter layer; the launch-time model comes from `--model`
    // (often absent). Resolve + validate the guard inputs once, compile the
    // in-scope set for the launch model, then hand the validated inputs to
    // the sink so it can re-scope when the provider reports its actual model
    // via `SessionStart`.
    let guard_inputs = std::sync::Arc::new(super::runaway_guard::resolve_guard_inputs(
        provider,
        env_context,
        None,
    )?);
    let runaway_guards = guard_inputs.compile_for_model(args.model.as_deref())?;
    sink.set_content_detector(runaway_guards.detector);
    sink.set_guard_rescope_source(guard_inputs, args.model.as_deref());
    let live_metrics = sink.live_metrics();
    let stream_output = sink.stream_output();
    let watchdog_state = Some(sink.watchdog_state());
    // Snapshot the sink's section-stream handle before the sink is
    // moved into the parser closure. Post-stream trailer and
    // Codex-final-stdout emission uses this handle so every section
    // transition shares the same tracker state.
    let section_stream = sink.section_stream();
    // Resolve the OpenCode stalled-generation backstop for the direct wrapper
    // path (CLI > env > built-in `10m`; no frontmatter layer here). Only the
    // OpenCode branch of `build_structured_plumbing` consumes it.
    let stall_timeout = composition::resolve_stall_timeout(args.stall_timeout.clone());
    // One signal hub per run: shared by the stdout reader thread, the
    // OpenCode stderr bridge, and the post-wait termination synthesis.
    // Harvest opt-in (E6) is resolved inside the builder.
    let signal_hub = policy::provider_signal_hub(provider);
    let (build_parser, stderr_bridge, content_early_rx) = policy::build_structured_plumbing(
        provider,
        sink,
        parser_config,
        stall_timeout,
        &signal_hub,
    );
    let mut _spawned = false;
    let stream_result = if let Some(wire_prompt) = wire_prompt.clone() {
        let runtime_context =
            match claudine::dispatch::DispatchRuntimeContext::load_for_env(env_context) {
                Ok(runtime) => runtime,
                Err(error) => {
                    tracing::warn!(%provider, "failed to preload wire runtime config: {error}");
                    claudine::dispatch::DispatchRuntimeContext::default()
                }
            };
        let _ = stderr_bridge;
        wire_io::run_kimi_wire_session(
            wire_io::WireSessionConfig {
                binary: binary_path,
                args: child_args,
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
                content_early_rx,
            },
            &mut _spawned,
        )?
    } else {
        let timeout_config = composition::resolve_timeouts(
            args.timeout.clone(),
            None,
            cli_step_timeout.clone(),
            None,
        )
        .with_provider(provider);
        exec::run_child_stream_semantic(
            binary_path,
            child_args,
            &env_plan.env,
            child_cwd,
            timeout_config,
            stderr_noise,
            profile.suppress_structured_stderr_on_success(),
            stream_verbosity != Verbosity::Silent,
            stdin_seed,
            build_parser,
            &mut _spawned,
            live_metrics,
            stream_output,
            stderr_bridge,
            None,
            watchdog_state,
            Some(section_stream.tracker()),
            content_early_rx,
            signal_hub,
            // Direct wrapper path: no sequence task owns this stream.
            None,
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
    if let Some(codex_output) = structured_codex_output {
        codex_output.apply_to_summary(&mut summary);
    }
    // Codex delivers its final assistant message via the
    // `--output-last-message` file; this block renders that file's
    // contents to stdout. Suppress on user interrupt because the
    // overlapping `agent_message` prose already rendered live as
    // `▌ ` thinking blocks above.
    if provider == Provider::Codex
        && !summary.assistant_text.is_empty()
        && !crate::output::user_interrupt_observed()
    {
        crate::output::emit_final_message(&summary.assistant_text, term, Some(&section_stream))?;
    }

    policy::emit_stream_summary(
        &summary,
        profile,
        env_context,
        stream_verbosity,
        detail_requested,
        &summary_details.lock().unwrap().clone(),
        Some(&section_stream),
        stream_result.agent_pid,
        &stream_result.signals,
        args.model.as_deref(),
    );

    let stderr_text = summary.stderr_text.clone();
    Ok((summary.exit_code, stderr_text))
}
