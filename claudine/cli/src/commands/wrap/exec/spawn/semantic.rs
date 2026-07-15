//! Semantic-stream spawn mode: drives a structured stream parser and live
//! renderer, with watchdog timeouts, the OpenCode stderr bridge, content
//! guards, and per-run signal collection.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::render::{AssistantStream, StreamRenderable};
use claudine::signals::{SignalHub, SignalSource};
use claudine::stream::logs::{EarlyTermination, StderrBridgeHandle, StderrIngestOutcome};
use claudine::stream::parser::{SemanticStreamParser, StreamParseError};
use claudine::stream::progress::LiveMetrics;
use claudine::stream::prompt_timing::PromptTimingContext;
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use tracing::{Span, info_span};

use super::super::stream_capture::StreamCapture;
use super::super::subagent_watchdog::WatchdogState;
use super::super::termination::{
    WatchdogTermination, apply_early_termination_to_summary, early_termination_guard_context,
    early_termination_message, wait_with_signal_and_early_termination, wait_with_signal_handling,
};
use super::super::timeouts::TimeoutConfig;
use super::super::watchdog::{
    spawn_flush_if_idle_ticker, spawn_prompt_timing_monitor, spawn_timeout_watchdog_ticker,
};
use super::super::{
    ErrorParser, OutputTextCallback, ProcessResult, ProcessTelemetry, ReasoningCallback,
    SemanticParserBuilder, join_with_timeout_or, kill_process_group, new_assistant_stream,
    resolve_first_response, stop_timing_ticker,
};
use super::setup;
use crate::commands::wrap::section::SectionTracker;
use crate::commands::wrap::stream_io::StreamOutput;

/// Spawn a provider child process with structured semantic stream parsing.
///
/// This is the Phase 3.4 replacement for [`run_child_stream`]. The
/// difference is the stdout loop: instead of switching on a returned
/// [`SemanticEvent`]s, the parser drives a [`SemanticEventSink`] that the
/// caller has already wired up for status rendering, dispatch, metrics,
/// and JSONL logging. This function's only rendering responsibility is
/// wiring the terminal-local `AssistantStream` instance to the sink
/// through the builder callback so it can run inside the parser thread.
/// Reasoning rendering is owned entirely by `LiveSemanticSink`.
///
/// `signal_hub` is the run's shared signal fan-in: the caller creates it
/// (and typically also hands a clone to the OpenCode stderr bridge via
/// `build_structured_plumbing`); this function feeds it stdout JSON lines
/// plus the post-wait termination mirror, then drains it into
/// `ProcessResult.signals`.
///
/// [`SemanticEventSink`]: claudine::stream::semantic::SemanticEventSink
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_child_stream_semantic(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout_config: TimeoutConfig,
    stderr_noise_prefixes: &[&str],
    suppress_stderr_on_success: bool,
    show_timing_output: bool,
    stdin_seed: Option<&str>,
    build_parser: SemanticParserBuilder,
    child_spawned: &mut bool,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
    stderr_bridge: Option<StderrBridgeHandle>,
    prompt_timing: Option<PromptTimingContext>,
    watchdog_state: Option<Arc<std::sync::Mutex<WatchdogState>>>,
    section_tracker: Option<Arc<Mutex<SectionTracker>>>,
    content_early_rx: Option<std::sync::mpsc::Receiver<EarlyTermination>>,
    signal_hub: Arc<SignalHub>,
) -> Result<ProcessResult<StreamExecutionSummary>> {
    setup::debug_assert_child_env(env);

    let needs_stdin_pipe = stdin_seed.is_some();
    let started_at = Instant::now();
    let started_at_wall = chrono::Local::now();

    let mut command = setup::base_command(binary, args, env, cwd);
    command
        .stdin(if needs_stdin_pipe {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    setup::isolate_into_process_group(&mut command);

    let mut child = command.spawn()?;
    let captured_pid = child.id();
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(captured_pid));

    // Terminal-local renderer for OutputText (stdout markdown). Wrapped
    // in Arc<Mutex<_>> so the builder closures can retain independent
    // handles without untangling lifetimes across the FnMut boundary of
    // the sink's callback storage. Shared with the flush-if-idle ticker
    // so buffered markdown can be surfaced even when the provider stalls
    // without closing stdout.
    //
    // Note: reasoning rendering is owned entirely by LiveSemanticSink,
    // which emits BlockQuote-formatted thinking text through the
    // section-aware stderr emitter. The reasoning_cb passed to the
    // parser builder is a no-op.
    let text_renderer: Arc<std::sync::Mutex<AssistantStream>> =
        Arc::new(std::sync::Mutex::new(new_assistant_stream()));

    let stdout_output = stream_output.clone();

    // Dedicated 30-second ticker that flushes any buffered markdown the
    // provider has not terminated with a paragraph boundary. Independent
    // from the prompt-timing monitor per feature spec.
    let flush_ticker = Some(spawn_flush_if_idle_ticker(
        stream_output.clone(),
        text_renderer.clone(),
        watchdog_state.clone(),
        section_tracker.clone(),
        timeout_config,
    ));

    // Prompt-scoped periodic header + warnings. Only started when the
    // caller provided a prompt context (every composition run) and when
    // the CLI is rendering timing output at all. Wrapper passthrough
    // runs pass `prompt_timing = None` and skip the monitor entirely.
    let timing_monitor = if show_timing_output {
        prompt_timing.map(|ctx| {
            spawn_prompt_timing_monitor(
                started_at,
                started_at_wall,
                ctx,
                timeout_config.timeout,
                timeout_config.step_timeout,
                live_metrics.clone(),
                stream_output.clone(),
            )
        })
    } else {
        None
    };

    // First-response trackers for structured stream mode:
    // semantic stdout (preferred), raw stdout (fallback), stderr (final fallback).
    let first_semantic_at = Arc::new(std::sync::Mutex::new(None));
    let first_raw_stdout_at = Arc::new(std::sync::Mutex::new(None));
    let first_stderr_at = Arc::new(std::sync::Mutex::new(None));

    // Spawn reader threads BEFORE writing stdin (see run_child deadlock note).
    let stdout_pipe = child
        .stdout
        .take()
        .expect("child stdout must be piped: Stdio::piped() was set on the child Command above");
    let stream_span = Span::current();
    let stdout_renderer = text_renderer.clone();
    let first_semantic_at_clone = Arc::clone(&first_semantic_at);
    let first_raw_stdout_at_clone = Arc::clone(&first_raw_stdout_at);
    let stdout_byte_metrics = live_metrics.clone();
    // Opt-in raw NDJSON capture for post-mortem analysis. Activated by
    // `CLAUDINE_RAW_STREAM_DIR`; `None` (and zero overhead) otherwise.
    let stream_capture_owned = StreamCapture::open(timeout_config.provider, child.id(), started_at);
    // Signal detection (Phase E4/E5): the run's shared hub observes every
    // stdout JSON line, independent of the semantic parser. Other producers
    // (the OpenCode stderr bridge, the post-wait termination synthesis
    // below) feed the same hub, so cross-source dedup is automatic.
    let stdout_signal_hub = Arc::clone(&signal_hub);
    // Bounded ring of the raw stdout lines the child wrote, retained so the
    // exit-source payload can carry a `stdout_tail`. Some providers (notably
    // Antigravity's `agy`) write terminal auth errors to stdout, which is
    // consumed here rather than surfaced to the join like `captured` stderr.
    let stdout_tail_ring: Arc<std::sync::Mutex<std::collections::VecDeque<String>>> =
        Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
    let stdout_tail_ring_clone = Arc::clone(&stdout_tail_ring);
    let stdout_handle = thread::spawn(move || {
        let _stream_guard = stream_span.enter();
        let _parse_span = info_span!("stream_parse").entered();
        let reader = BufReader::new(stdout_pipe);
        let mut out = stdout_output.stdout_writer();

        let text_renderer = stdout_renderer;

        let output_cb: OutputTextCallback = {
            let text = text_renderer.clone();
            let mut writer = stdout_output.stdout_writer();
            let first_at = first_semantic_at_clone;
            Box::new(move |chunk: &str| {
                if !chunk.is_empty() {
                    let mut g = first_at.lock().unwrap();
                    if g.is_none() {
                        *g = Some(Instant::now());
                    }
                }
                if let Ok(mut r) = text.lock() {
                    for frame in r.append(chunk) {
                        let _ = writer.write_all(frame.as_bytes());
                    }
                    let _ = writer.flush();
                }
            })
        };
        let reasoning_cb: ReasoningCallback = Box::new(|_chunk: &str| {});

        let mut parser: Box<dyn SemanticStreamParser> =
            build_parser(output_cb, reasoning_cb, Some(captured_pid));
        let mut fallback_mode = false;
        let mut stream_capture = stream_capture_owned;

        for line in reader.lines() {
            let Ok(line) = line else { break };

            let line_at = Instant::now();
            {
                let mut g = first_raw_stdout_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(line_at);
                }
            }

            // Retain the raw (pre-render) line in the capped tail ring so the
            // exit-source payload reflects what the child actually wrote.
            {
                let mut ring = stdout_tail_ring_clone.lock().unwrap();
                if ring.len() == claudine::signals::EXIT_STDOUT_TAIL_LINES {
                    ring.pop_front();
                }
                ring.push_back(line.clone());
            }

            // Provider-agnostic activity heartbeat: refresh the byte clock
            // BEFORE feeding the line to the semantic parser, so even
            // partially-buffered or post-completion-only providers (notably
            // OpenCode, which emits no `tool_start` / `task_started`) keep
            // the silence rule honest. Whitespace-only lines are ignored.
            if let Ok(mut g) = stdout_byte_metrics.lock() {
                g.record_byte_activity(&line, line_at);
            }

            // Mirror the raw line to the post-mortem capture file when
            // `CLAUDINE_RAW_STREAM_DIR` is set. No-op otherwise.
            if let Some(capture) = stream_capture.as_mut() {
                capture.record_line(&line, line_at);
            }

            // Offer the line to the signal hub independently of the
            // semantic parser (and of fallback mode). A malformed JSON line
            // is silently skipped here — the parser path already reports
            // malformed lines. Version auto-narrowing lives inside the hub.
            {
                let trimmed = line.trim_start();
                if trimmed.starts_with('{')
                    && let Ok(payload) = serde_json::from_str::<serde_json::Value>(trimmed)
                {
                    stdout_signal_hub.observe_json(SignalSource::Stream, &payload);
                }
            }

            if fallback_mode {
                let _ = writeln!(out, "{}", crate::log::maybe_strip(&line));
                continue;
            }

            match parser.feed_line(&line) {
                Ok(()) => {}
                Err(StreamParseError::MalformedLine { .. }) => {
                    // Semantic parsers emit Warning events instead of
                    // returning MalformedLine, but guard the variant here
                    // in case a legacy adapter still surfaces it.
                    tracing::debug!("skipping malformed stream line: {line}");
                }
                Err(StreamParseError::Fatal(_)) => {
                    if let Ok(mut r) = text_renderer.lock() {
                        for frame in r.close() {
                            let _ = out.write_all(frame.as_bytes());
                        }
                        let _ = out.flush();
                    }
                    fallback_mode = true;
                    let _ = writeln!(out, "{}", crate::log::maybe_strip(&line));
                }
            }
        }

        if let Ok(mut r) = text_renderer.lock() {
            for frame in r.close() {
                let _ = out.write_all(frame.as_bytes());
            }
            let _ = out.flush();
        }
        parser
    });

    let pipe = child
        .stderr
        .take()
        .expect("child stderr must be piped: Stdio::piped() was set on the child Command above");
    let prefixes: Vec<String> = stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let plain = crate::log::is_plain();
    let stderr_term = crate::log::terminal();
    let termination_term = stderr_term.clone();
    let stderr_span = Span::current();
    let (mut bridge_for_thread, finalize_for_main, bridge_early_rx) = match stderr_bridge {
        Some(StderrBridgeHandle {
            bridge,
            finalize,
            early_terminate,
        }) => (Some(bridge), Some(finalize), early_terminate),
        None => (None, None, None),
    };
    // The content detector (Phase 6) feeds the same wait loop. For OpenCode
    // it shares the bridge's channel (so `content_early_rx` is `None`); for
    // every other provider the detector's dedicated receiver arrives here.
    // The two are mutually exclusive by construction, so picking whichever
    // is `Some` gives the wait loop the one receiver to poll.
    let early_terminate_rx = bridge_early_rx.or(content_early_rx);
    let has_bridge = bridge_for_thread.is_some();
    let capture_always = has_bridge;
    let first_stderr_at_clone = Arc::clone(&first_stderr_at);
    let stderr_byte_metrics = live_metrics.clone();
    let stderr_handle = thread::spawn(move || {
        let _stderr_guard = stderr_span.enter();
        let reader = BufReader::new(pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };

            // Refresh the byte heartbeat for every non-empty stderr line,
            // including noise-prefixed and bridge-consumed lines — those are
            // still bytes flowing from the wrapped child and prove it is
            // making progress. Done before noise filtering for that reason.
            let line_at = Instant::now();
            if let Ok(mut g) = stderr_byte_metrics.lock() {
                g.record_byte_activity(&line, line_at);
            }

            if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }

            {
                let mut g = first_stderr_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(line_at);
                }
            }

            // When a bridge is installed, offer the raw line first so it can
            // classify structured log records (and their multi-line tails) in
            // real time. Consumed lines are suppressed from raw passthrough
            // and never land in the captured buffer — the semantic event the
            // bridge emitted already carries everything operators need.
            if let Some(bridge) = bridge_for_thread.as_mut()
                && matches!(bridge.ingest(&line), StderrIngestOutcome::Consumed)
            {
                continue;
            }

            let formatted = crate::output::try_format_api_error(&line, &stderr_term);
            let output_line = formatted.as_deref().unwrap_or(&line);
            let output_line = if plain {
                biscuit_terminal::prelude::strip_escape_codes(output_line)
            } else {
                output_line.to_string()
            };

            if suppress_stderr_on_success || capture_always {
                if !captured.is_empty() {
                    captured.push('\n');
                }
                captured.push_str(&output_line);
            }
            if !suppress_stderr_on_success {
                let mut err = std::io::stderr().lock();
                let _ = writeln!(err, "{output_line}");
            }
        }
        captured
    });

    if let Some(seed) = stdin_seed {
        // BrokenPipe is benign: child closed stdin or exited before we
        // finished writing the seed. See `run_child` for the same rationale.
        setup::write_stdin_seed(&mut child, seed)?;
    }

    // Watchdog channel: the ticker sends termination requests; the wait
    // loop receives them and escalates SIGTERM → SIGKILL via the same
    // pathway used for stderr-bridge early termination.
    let (watchdog_tx, watchdog_rx): (
        std::sync::mpsc::Sender<WatchdogTermination>,
        std::sync::mpsc::Receiver<WatchdogTermination>,
    ) = std::sync::mpsc::channel();
    let watchdog_enabled = timeout_config.any_enabled() && watchdog_state.is_some();
    let mut watchdog_ticker = None;
    if watchdog_enabled && let Some(state) = watchdog_state {
        watchdog_ticker = Some(spawn_timeout_watchdog_ticker(
            timeout_config,
            started_at,
            state,
            watchdog_tx,
            live_metrics.clone(),
            stream_output.clone(),
        ));
    }
    // Promote to the advanced wait path whenever the watchdog ticker or
    // stderr early-terminate bridge is active. The watchdog is the sole
    // source of timeout-driven termination; the wait loop only consumes
    // signals from channels.
    let needs_advanced_wait = early_terminate_rx.is_some() || watchdog_enabled;
    let (exit_code, termination, early_termination) = if needs_advanced_wait {
        // Synthesize a disconnected receiver when no stderr bridge is
        // installed so the wait loop can still receive watchdog signals.
        let rx = early_terminate_rx.unwrap_or_else(|| {
            let (_tx, rx) = std::sync::mpsc::channel();
            rx
        });
        let wd_rx = if watchdog_enabled {
            Some(watchdog_rx)
        } else {
            None
        };
        // Structured streaming is always a non-interactive run (it requires
        // `effective_non_interactive`), so the compressed SIGTERM-first
        // ladder (F5) applies: no human is mid-session to react to a SIGINT.
        wait_with_signal_and_early_termination(
            &mut child,
            true,
            rx,
            wd_rx,
            timeout_config.kill_grace,
            false,
        )?
    } else {
        let (code, term) = wait_with_signal_handling(&mut child, true)?;
        (code, term, None)
    };

    // Surface the early-termination message as a styled `Warning` line on
    // stderr so the user sees an immediate reason for the kill (the summary
    // re-derives the same message via `apply_early_termination_to_summary`
    // below). Every variant carries a message — the catch-all `Some(_)`
    // arm keeps the match exhaustive as new variants are added without
    // silently dropping the inline notification.
    if let Some(message) = early_termination
        .as_ref()
        .and_then(early_termination_message)
    {
        let rendered = Status::new(&message)
            .state(StatusState::Warning)
            .render(&termination_term);
        stream_output.emit_stderr_line(&rendered);
    }

    kill_process_group(&mut child);
    stop_timing_ticker(flush_ticker);
    stop_timing_ticker(timing_monitor);
    stop_timing_ticker(watchdog_ticker);

    let thread_join_timeout = Duration::from_secs(5);
    let parser: Box<dyn SemanticStreamParser> = join_with_timeout_or(
        stdout_handle,
        thread_join_timeout,
        Box::new(ErrorParser { exit_code }),
    );

    let captured = join_with_timeout_or(stderr_handle, thread_join_timeout, String::new());
    if suppress_stderr_on_success && exit_code != 0 && !captured.is_empty() {
        eprintln!("{captured}");
    }

    // Exit source (E5): synthesize the ratified
    // `{exit_code, stdout_tail, stderr_tail}` payload once per run. This is
    // what makes `source: exit` detection records (and the qwen 53/55/130
    // bespoke exit mapping) live — those terminations bypass any terminal
    // `result` event, so only the wrapper can observe them. `captured` is the
    // same stderr the error-report path consumes via `summary.stderr_text`;
    // `stdout_tail` carries the child's last stdout lines (the surface
    // Antigravity's `agy` writes its auth errors to).
    let stdout_tail = {
        let ring = stdout_tail_ring.lock().unwrap();
        ring.iter().cloned().collect::<Vec<_>>().join("\n")
    };
    signal_hub.observe_json(
        SignalSource::Exit,
        &claudine::signals::exit_source_payload(exit_code, &stdout_tail, &captured),
    );

    let mut summary = parser.finish(exit_code);
    if summary.duration_ms.is_none() {
        summary.duration_ms = Some(started_at.elapsed().as_millis() as u64);
    }

    // Apply early-termination overrides before the stderr finalizer so the
    // finalizer's badge recomputation observes the synthesized error fields
    // (for example, a `usage_limit_reached` error_kind that maps to a Quota
    // badge). The bridge signaled early termination from the stderr thread;
    // the main wait loop then killed the child's process group, so the raw
    // exit code reflects SIGTERM rather than a meaningful provider status.
    if let Some(termination) = early_termination.as_ref() {
        apply_early_termination_to_summary(&mut summary, termination);
        // Bespoke signal mirror (E5): every termination synthesized into the
        // summary is also a taxonomy signal. `Stream` because the temporal
        // guards judge stream content/liveness; for OpenCode bridge-origin
        // trips the bridge already emitted the same kind from
        // `fire_early_termination` and the sink's correlation window folds
        // this second emission into it.
        signal_hub.emit_bespoke(termination.to_signal_event(), SignalSource::Stream);
    }

    // Merge stderr-derived diagnostics after both reader threads have
    // joined. The bridge accumulated counters into its own shared state
    // during streaming; the finalizer reads that state and enriches the
    // summary. `stderr_text` is attached here so every structured
    // bridge-enabled session carries the captured stderr regardless of
    // `suppress_stderr_on_success`. Badge recomputation happens inside
    // the finalizer so stderr-derived diagnostics show up in the final
    // `summary.badges` vector.
    if let Some(finalize) = finalize_for_main {
        if !captured.is_empty() && summary.stderr_text.is_none() {
            summary.stderr_text = Some(captured.clone());
        }
        finalize(&mut summary);
    }

    let first_response = resolve_first_response(
        *first_semantic_at.lock().unwrap(),
        *first_raw_stdout_at.lock().unwrap(),
        *first_stderr_at.lock().unwrap(),
        started_at,
    );

    // Structured guard detail for a content-guard trip (None for ordinary
    // completions, timeouts, and rate-limit aborts).
    let guard_context = early_termination
        .as_ref()
        .and_then(early_termination_guard_context);

    // Resolved-model drift check against the expected-offerings baseline,
    // before flush/drain so any drift event rides this run's signals.
    crate::commands::wrap::catalog_drift::emit_resolved_model_drift(&signal_hub);
    // End-of-run harvest flush (E6): persist unmatched error/warning-class
    // candidates when opted in; a no-op otherwise.
    claudine::signals::harvest::flush_hub(&signal_hub);

    let result = ProcessResult {
        data: summary,
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed: started_at.elapsed(),
            first_response_latency: first_response,
        },
        agent_pid: Some(captured_pid),
        guard_context,
        signals: signal_hub.drain(),
    };
    if !result.signals.is_empty() {
        let per_kind: Vec<String> = result
            .signals
            .iter()
            .map(|signal| {
                format!(
                    "{}x{}",
                    <&'static str>::from(signal.event.kind()),
                    signal.occurrences
                )
            })
            .collect();
        tracing::debug!(signals = ?per_kind, "signal collection summary");
    }
    Ok(result)
}
