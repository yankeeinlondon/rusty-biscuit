use biscuit_terminal::terminal::Terminal;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info_span;

use claudine::composition::IterationSummarySignals;
use super::{
    AttemptLaunch, HarnessPromptMode, HarnessPromptState, MaterializedHarnessPrompt,
    harness_prompt_mode_label,
};

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(crate) fn execute_harness_attempt(
    attempt: u32,
    provider: Provider,
    profile: &dyn super::super::profile::WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    launch: &AttemptLaunch,
    prompt_mode: HarnessPromptMode,
    prompt_state: &HarnessPromptState,
    materialized: &MaterializedHarnessPrompt,
    effective_non_interactive: bool,
    use_structured: bool,
    structured_codex_output: Option<&super::super::policy::StructuredCodexOutput>,
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
    task_frame_writer: Option<claudine::render::TaskFrameWriter>,
) -> Result<(
    claudine::harness::AttemptOutcome,
    Option<crate::perf::AgentExecutionPerf>,
    Option<IterationSummarySignals>,
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
    // Step-silence and stalled-generation enforcement both require live
    // `SemanticEvent` ticks, which only exist in the structured-stream path.
    // Capture and passthrough attempts drop the values with a warning rather
    // than silently ignoring a user-configured budget.
    let launch = if !use_structured
        && (launch.timeout_config.step_timeout.is_some() || launch.stall_timeout.is_some())
    {
        use biscuit_terminal::components::renderable::TerminalRenderable;
        use biscuit_terminal::components::status::{Status, StatusState};
        if launch.timeout_config.step_timeout.is_some() && launch.step_timeout_user_configured {
            let rendered = Status::new(
                "step_timeout is only enforced in structured-stream mode; \
                 ignoring for this capture/passthrough attempt"
                    .to_string(),
            )
            .state(StatusState::Warning)
            .render(term);
            eprintln!("{rendered}");
        }
        if launch.stall_timeout.is_some() && launch.stall_timeout_user_configured {
            let rendered = Status::new(
                "stall_timeout is only enforced in structured-stream mode; \
                 ignoring for this capture/passthrough attempt"
                    .to_string(),
            )
            .state(StatusState::Warning)
            .render(term);
            eprintln!("{rendered}");
        }
        let mut adjusted = launch.clone();
        adjusted.timeout_config.step_timeout = None;
        adjusted.stall_timeout = None;
        adjusted
    } else {
        launch.clone()
    };
    let launch = &launch;

    // Resolve the runaway-output guards once for this attempt (Phase 6).
    // The model is taken from the `MODEL` env override or the frontmatter
    // `model` key; an unknown model still matches global/agent scopes. The
    // streaming branch consumes `detector`; the capture branch consumes
    // `capture_volume_cap`.
    let run_model = materialized
        .env_overrides
        .iter()
        .find(|(key, _)| key == "MODEL")
        .map(|(_, value)| value.clone())
        .or_else(|| {
            materialized
                .frontmatter
                .get("model")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });
    // Resolve + validate the guard inputs once (model-independent), then
    // compile the in-scope set for the launch-time `run_model`. The
    // validated inputs are kept (as an `Arc`) so the structured-stream sink
    // can re-scope the exit-expression set when the provider reports its
    // actual model via `SessionStart`.
    let guard_inputs = Arc::new(
        crate::commands::wrap::runaway_guard::resolve_guard_inputs(
            provider,
            env_context,
            Some(&materialized.frontmatter),
        )?,
    );
    let runaway_guards = guard_inputs.compile_for_model(run_model.as_deref())?;

    // Presence bracket: STARTED now, ENDED when this guard drops on any
    // exit path of the attempt (best-effort; no-op without a daemon).
    let session_presence = crate::commands::wrap::session_report::SessionPresence::started(
        provider,
        run_model.as_deref(),
        !effective_non_interactive,
        env_context,
        &launch.env,
    );

    let (
        exit_code,
        termination,
        session_id,
        final_response,
        stderr_text,
        perf,
        iteration_signals,
        error_kind,
        guard_context,
        error_message,
        timeout_secs,
    ) = if use_structured
    {
        let summary_details = Arc::new(Mutex::new(
            super::super::policy::StructuredSummaryDetails::default(),
        ));
        let parser_config = claudine::stream::ParserConfig::default();
        let mut sink = super::super::live_semantic_sink::LiveSemanticSink::with_default_wiring(
            provider,
            env_context.clone(),
            child_cwd,
            stream_verbosity,
            summary_details.clone(),
        )
        .with_context_extra(dispatch_context.clone())
        .with_status_reporter(session_presence.status_reporter());
        // Arm the content detector (Phase 6) before plumbing so
        // `build_structured_plumbing` can wire the trip channel, and wire
        // the re-scope source so a provider-reported `SessionStart` model
        // can bring agent/model-scoped exit expressions into scope.
        sink.set_content_detector(runaway_guards.detector);
        sink.set_guard_rescope_source(guard_inputs.clone(), run_model.as_deref());
        let live_metrics = sink.live_metrics();
        let stream_output = sink.stream_output();
        let watchdog_state = Some(sink.watchdog_state());
        let section_stream = sink.section_stream();
        // One signal hub per attempt: shared by the stdout reader thread,
        // the OpenCode stderr bridge, and the post-wait termination
        // synthesis. Harvest opt-in (E6) is resolved inside the builder.
        let signal_hub = super::super::policy::provider_signal_hub(provider);
        let (build_parser, stderr_bridge, content_early_rx) =
            super::super::policy::build_structured_plumbing(
                provider,
                sink,
                parser_config,
                launch.stall_timeout,
                &signal_hub,
            );
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
            super::super::wire_io::run_kimi_wire_session(
                super::super::wire_io::WireSessionConfig {
                    binary: binary_path,
                    args: &launch.args,
                    env: &launch.env,
                    cwd: child_cwd,
                    prompt: wire_prompt,
                    timeout: launch.timeout_config.timeout,
                    client_name: env!("CARGO_PKG_NAME"),
                    client_version: env!("CARGO_PKG_VERSION"),
                    capabilities: super::super::wire_io::WireClientCapabilities::default_for_claudine(),
                    env_context: env_context.clone(),
                },
                super::super::wire_io::WireSessionWiring {
                    build_parser,
                    stream_output,
                    live_metrics,
                    runtime_context,
                    content_early_rx,
                },
                child_spawned,
            )?
        } else {
            super::super::exec::run_child_stream_semantic(
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
                content_early_rx,
                signal_hub,
                task_frame_writer,
            )?
        };
        let api_duration_ms = stream_result.data.duration_ms;
        let perf = Some(stream_result.telemetry.into_agent_perf(api_duration_ms));
        let termination = stream_result.termination;
        let captured_agent_pid = stream_result.agent_pid;
        // Capture the structured guard context (Phase 6) before `data` is
        // moved out below; `None` for non-content terminations.
        let stream_guard_context = stream_result.guard_context.clone();
        let mut summary = stream_result.data;
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
            crate::output::emit_final_message(
                &summary.assistant_text,
                term,
                Some(&section_stream),
            )?;
        }

        super::super::policy::emit_stream_summary(
            &summary,
            profile,
            env_context,
            stream_verbosity,
            detail_requested,
            &summary_details.lock().unwrap().clone(),
            Some(&section_stream),
            captured_agent_pid,
            &stream_result.signals,
            run_model.as_deref(),
        );

        let effective_response = {
            let details = summary_details.lock().unwrap();
            if prompt_mode == HarnessPromptMode::Inline {
                let fr = details.final_response.trim();
                if fr.is_empty() {
                    summary.assistant_text.clone()
                } else {
                    details.final_response.clone()
                }
            } else {
                summary.assistant_text.clone()
            }
        };

        let mut iteration_summary_signals = IterationSummarySignals::from_summary(&summary);
        iteration_summary_signals.apply_projected_rate_limit(
            claudine::signals::project::rate_limit_info(&stream_result.signals),
        );
        let iteration_signals = Some(iteration_summary_signals);

        // Thread the honest per-guard `error_kind` (C3a) and structured
        // guard context from the synthesized summary / process result into
        // the attempt outcome.
        let error_kind = summary.error_kind.clone();
        // Which timeout rule fired is disambiguated by the synthesized
        // `error_kind`; thread the matching configured duration so the
        // failure message can name it.
        let timeout_secs = matches!(
            termination,
            claudine::harness::ProcessTermination::TimedOut
        )
        .then(|| {
            match error_kind.as_deref() {
                Some("step_timeout") => launch.timeout_config.step_timeout,
                _ => launch.timeout_config.timeout,
            }
            .map(|duration| duration.as_secs())
        })
        .flatten();

        (
            summary.exit_code,
            termination,
            summary.session_id.clone(),
            effective_response,
            summary.stderr_text.clone(),
            perf,
            iteration_signals,
            error_kind,
            stream_guard_context,
            summary.error_message.clone(),
            timeout_secs,
        )
    } else if effective_non_interactive {
        let capture = super::super::exec::run_child_capture(
            binary_path,
            &launch.args,
            &launch.env,
            child_cwd,
            launch.timeout_config.timeout.map(|d| d.as_secs()),
            false,
            super::super::exec::ChildIoOptions {
                stdout_noise_prefixes: stdout_noise,
                stderr_noise_prefixes: stderr_noise,
                stdin_seed: launch.stdin_seed.as_deref(),
            },
            child_spawned,
            // Capture path gets the per-run volume cap only (F3).
            Some(runaway_guards.capture_volume_cap.clone()),
            // Provider-attributed hub so exit-source records and bespoke
            // exit mappings (qwen 53/55/130) fire on the capture path too;
            // harvest opt-in (E6) is resolved inside the builder.
            Some(super::super::policy::provider_signal_hub(provider)),
        )?;
        let perf = Some(capture.telemetry.into_agent_perf(None));
        let termination = capture.termination;
        // A capture-path content trip is always the volume cap (F3).
        let capture_guard_context = capture.guard_context.clone();
        let capture_error_kind = capture_guard_context
            .as_ref()
            .map(|_| "runaway_volume".to_string());
        let stdout = capture.data.stdout;
        let stderr = capture.data.stderr;
        let response = profile.parse_captured_output(&stdout);

        if !response.trim().is_empty() {
            crate::output::emit_final_message(&response, term, None)?;
        }

        if !stderr.trim().is_empty() {
            eprintln!("{stderr}");
        }

        // No stream parser on this path, so no synthesized error message;
        // only the wall-clock `timeout` rule applies here.
        let capture_timeout_secs = matches!(
            termination,
            claudine::harness::ProcessTermination::TimedOut
        )
        .then(|| launch.timeout_config.timeout.map(|duration| duration.as_secs()))
        .flatten();

        (
            capture.data.exit_code,
            termination,
            None,
            response,
            (!stderr.trim().is_empty()).then_some(stderr),
            perf,
            None,
            capture_error_kind,
            capture_guard_context,
            None,
            capture_timeout_secs,
        )
    } else {
        // Interactive TUI path: inherit stdout/stderr directly so the
        // provider can drive the terminal. For Codex inline composition,
        // the final response is recovered from `--output-last-message`.
        let result = super::super::exec::run_child(
            binary_path,
            &launch.args,
            &launch.env,
            child_cwd,
            launch.timeout_config.timeout.map(|d| d.as_secs()),
            true,
            super::super::exec::ChildIoOptions {
                stdout_noise_prefixes: &[],
                stderr_noise_prefixes: &[],
                stdin_seed: launch.stdin_seed.as_deref(),
            },
            child_spawned,
        )?;
        let perf = Some(result.telemetry.into_agent_perf(None));
        let termination = result.termination;
        // `structured_codex_output` is only prepared for Codex
        // (exec_prep::prepare_codex_structured_output), so no provider
        // check is needed here.
        let response = structured_codex_output
            .map(|output| output.take_last_message())
            .unwrap_or_default();
        // No stream parser on this path, so no synthesized error message;
        // only the wall-clock `timeout` rule applies here.
        let interactive_timeout_secs = matches!(
            termination,
            claudine::harness::ProcessTermination::TimedOut
        )
        .then(|| launch.timeout_config.timeout.map(|duration| duration.as_secs()))
        .flatten();

        (
            result.data,
            termination,
            None,
            response,
            None,
            perf,
            None,
            None,
            None,
            None,
            interactive_timeout_secs,
        )
    };

    if prompt_mode == HarnessPromptMode::Inline {
        super::super::inline::report_inline_agent_status(
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
            error_kind,
            guard_context,
            error_message,
            timeout_secs,
        },
        perf,
        iteration_signals,
    ))
}
