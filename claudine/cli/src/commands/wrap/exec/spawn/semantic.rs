//! Semantic-stream spawn mode: drives a structured stream parser and live
//! renderer, with watchdog timeouts, the OpenCode stderr bridge, content
//! guards, and per-run signal collection.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::render::{AssistantStream, StreamRenderable};
use claudine::signals::{SignalHub, SignalSource};
use claudine::stream::logs::{EarlyTermination, StderrBridgeHandle, StderrIngestOutcome};
use claudine::stream::parser::SemanticStreamParser;
use claudine::stream::progress::LiveMetrics;
use claudine::stream::prompt_timing::PromptTimingContext;
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use tracing::{Span, info_span};

use super::super::super::run_scope::{RunScope, feed_line as feed_and_publish};
use super::super::control::StdioControl;
use super::super::stream_capture::StreamCapture;
use super::super::subagent_watchdog::WatchdogState;
use super::super::termination::{
    WatchdogTermination, apply_early_termination_to_summary, early_termination_guard_context,
    early_termination_message, wait_with_signal_early_termination_and_completion,
    wait_with_signal_handling,
};
use super::super::timeouts::TimeoutConfig;
use super::super::watchdog::{
    spawn_flush_if_idle_ticker, spawn_prompt_timing_monitor, spawn_timeout_watchdog_ticker,
};
use super::super::reader_join::{
    JoinOutcome, ParserSlot, ReaderBudget, ReaderProgress, ReaderStream, join_reader,
    reader_failure, reader_warning_line, settle_parser,
};
use super::super::{
    OutputTextCallback, ProcessResult, ProcessTelemetry, ReasoningCallback, SemanticParserBuilder,
    kill_process_group, new_assistant_stream_inset, resolve_first_response, stop_timing_ticker,
};
use super::super::super::run_scope::observation::Operation;
use super::super::super::run_scope::retention::{RetainingReader, last_message};
use super::retained::{LineSource, Preflight, spawn_retained};
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
/// Drain the assistant renderer's final frames to whichever stdout path is live.
///
/// The framed path also flushes its held partial line: the stream is over, so a
/// fragment waiting for a newline it will never receive must still be shown.
fn drain_close(
    renderer: &Arc<std::sync::Mutex<AssistantStream>>,
    framed: &Option<Arc<std::sync::Mutex<claudine::render::TaskFrameWriter>>>,
    out: &mut impl Write,
) {
    let Ok(mut renderer) = renderer.lock() else {
        return;
    };
    let frames = renderer.close();
    match framed {
        Some(framed) => {
            if let Ok(mut framed) = framed.lock() {
                for frame in frames {
                    framed.write(&frame);
                }
                framed.flush();
            }
        }
        None => {
            for frame in frames {
                let _ = out.write_all(frame.as_bytes());
            }
            let _ = out.flush();
        }
    }
}

/// Merges two early-termination receivers into one for the wait loop.
fn merge_early(
    first: Option<std::sync::mpsc::Receiver<EarlyTermination>>,
    second: Option<std::sync::mpsc::Receiver<EarlyTermination>>,
) -> Option<std::sync::mpsc::Receiver<EarlyTermination>> {
    match (first, second) {
        (Some(first), Some(second)) => {
            let (tx, rx) = std::sync::mpsc::channel();
            for source in [first, second] {
                let tx = tx.clone();
                thread::spawn(move || {
                    for termination in source {
                        if tx.send(termination).is_err() {
                            break;
                        }
                    }
                });
            }
            Some(rx)
        }
        (first, second) => first.or(second),
    }
}

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
/// `control`, when given with a `stdin_seed`, keeps the child's stdin for a
/// retained-stdin control session: the seed becomes the task the session
/// submits once the child is ready, instead of raw stdin bytes. A child that
/// is not ready before submission is replaced by the session's fallback
/// launch, which runs through this same function with the seed on stdin.
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
    // Frames assistant text under one sequence task's bar before it reaches
    // stdout. `None` writes the stream undecorated, as every non-sequence run
    // does.
    task_frame_writer: Option<claudine::render::TaskFrameWriter>,
    control: Option<Arc<dyn StdioControl>>,
) -> Result<ProcessResult<StreamExecutionSummary>> {
    setup::debug_assert_child_env(env);

    let started_at = Instant::now();
    let started_at_wall = chrono::Local::now();
    let run_scope = RunScope::with_origin(started_at);

    // A control session needs a task to submit; without one the child runs
    // exactly as an ordinary structured stream would.
    let control = control.zip(stdin_seed);
    #[cfg(unix)]
    let mut descendant_watch = None;
    let (mut child, stdout_source, stderr_source, control_receivers): (Child, LineSource, LineSource, _) =
        match &control {
            Some((session, task)) => {
                match spawn_retained(binary, args, env, cwd, session, task, child_spawned, run_scope.clone())? {
                    Preflight::Ready(ready) => {
                        let ready = *ready;
                        #[cfg(unix)]
                        {
                            descendant_watch = Some(ready.descendants);
                        }
                        (ready.child, ready.stdout, ready.stderr, Some((ready.early, ready.completion)))
                    }
                    Preflight::NotReady { reason, stderr } => {
                        let term = crate::log::terminal();
                        for line in &stderr {
                            stream_output.emit_stderr_line(line);
                        }
                        let fallback = session
                            .fallback()
                            .filter(|_| !crate::output::user_interrupt_observed());
                        let Some(fallback) = fallback else {
                            return Err(color_eyre::eyre::eyre!(
                                "the provider did not become ready over its control protocol ({reason}); \
                                 the task was not submitted"
                            ));
                        };
                        let warning = format!("{reason}. {}", fallback.warning);
                        stream_output.emit_stderr_line(&Status::new(&warning).state(StatusState::Warning).render(&term));
                        return run_child_stream_semantic(
                            binary,
                            &fallback.args,
                            env,
                            cwd,
                            timeout_config,
                            stderr_noise_prefixes,
                            suppress_stderr_on_success,
                            show_timing_output,
                            stdin_seed,
                            build_parser,
                            child_spawned,
                            live_metrics,
                            stream_output,
                            stderr_bridge,
                            prompt_timing,
                            watchdog_state,
                            section_tracker,
                            content_early_rx,
                            signal_hub,
                            task_frame_writer,
                            None,
                        );
                    }
                }
            }
            None => {
                let mut command = setup::base_command(binary, args, env, cwd);
                command
                    .stdin(if stdin_seed.is_some() { Stdio::piped() } else { Stdio::null() })
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                setup::isolate_into_process_group(&mut command);
                let mut child = command.spawn()?;
                *child_spawned = true;
                crate::budget::record_child(child.id());
                Span::current().record("child_pid", tracing::field::display(child.id()));
                let stdout = child
                    .stdout
                    .take()
                    .expect("child stdout must be piped: Stdio::piped() was set on the child Command above");
                let stderr = child
                    .stderr
                    .take()
                    .expect("child stderr must be piped: Stdio::piped() was set on the child Command above");
                (child, Box::new(BufReader::new(RetainingReader::new(stdout, run_scope.clone())).lines()), Box::new(BufReader::new(stderr).lines()), None)
            }
        };
    let captured_pid = child.id();

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
    // One shared writer, not a clone per closure: the streaming callback and
    // the end-of-stream `close` must agree about the held partial line.
    let framed_writer = task_frame_writer
        .map(|writer| Arc::new(std::sync::Mutex::new(writer)));
    let text_inset = framed_writer
        .as_ref()
        .map_or(0, |writer| writer.lock().map_or(0, |w| w.gutter_width() as u32));
    let text_renderer: Arc<std::sync::Mutex<AssistantStream>> =
        Arc::new(std::sync::Mutex::new(new_assistant_stream_inset(text_inset)));

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
        framed_writer.clone(),
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
    let stream_span = Span::current();
    let stdout_renderer = text_renderer.clone();
    let first_semantic_at_clone = Arc::clone(&first_semantic_at);
    let first_raw_stdout_at_clone = Arc::clone(&first_raw_stdout_at);
    let stdout_byte_metrics = live_metrics.clone();
    // Replies to the control session's own commands (a steering send, a
    // state check) are not agent progress and must not hold off the
    // stream-silence rule.
    let control_replies = control.as_ref().map(|(session, _)| Arc::clone(session));
    // Opt-in raw NDJSON capture for post-mortem analysis. Activated by
    // `CLAUDINE_RAW_STREAM_DIR`; `None` (and zero overhead) otherwise.
    let mut stream_capture_owned = StreamCapture::open(timeout_config.provider, captured_pid, started_at);
    if let Some(capture) = stream_capture_owned.as_mut() { capture.observe_with(run_scope.clone()); }
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
    let stdout_progress = ReaderProgress::default();
    let stdout_reader_progress = stdout_progress.clone();
    let parser_slot: ParserSlot = Arc::new(Mutex::new(None));
    let reader_run_scope = run_scope.clone();
    let reader_parser_slot = Arc::clone(&parser_slot);
    let stdout_handle = thread::spawn(move || {
        let _stream_guard = stream_span.enter();
        let _parse_span = info_span!("stream_parse").entered();
        let _scope_guard = reader_run_scope.enter();
        let mut out = stdout_output.stdout_writer();

        let text_renderer = stdout_renderer;
        let framed_close = framed_writer.clone();

        let output_cb: OutputTextCallback = {
            let text = text_renderer.clone();
            let mut writer = stdout_output.stdout_writer();
            let framed = framed_writer.clone();
            let first_at = first_semantic_at_clone;
            Box::new(move |chunk: &str| {
                if !chunk.is_empty() {
                    let mut g = first_at.lock().unwrap();
                    if g.is_none() {
                        *g = Some(Instant::now());
                    }
                }
                if let Ok(mut r) = text.lock() {
                    super::super::super::run_scope::observe(Operation::RenderComputation);
                    let frames = r.append(chunk);
                    match framed.as_ref() {
                        // Framed: the writer emits only complete lines through
                        // the coordinator, so there is nothing to flush per
                        // chunk — a mid-line flush would publish half a line
                        // under the bar.
                        Some(framed) => {
                            if let Ok(mut framed) = framed.lock() {
                                for frame in frames {
                                    framed.write(&frame);
                                }
                            }
                        }
                        None => {
                            for frame in frames {
                                let _ = writer.write_all(frame.as_bytes());
                            }
                            let _ = writer.flush();
                        }
                    }
                }
            })
        };
        let reasoning_cb: ReasoningCallback = Box::new(|_chunk: &str| {});

        let mut parser: Box<dyn SemanticStreamParser> =
            build_parser(output_cb, reasoning_cb, Some(captured_pid));
        let mut stream_capture = stream_capture_owned;

        let mut read_failed = false;
        for line in stdout_reader_progress.track_observed(stdout_source, reader_run_scope.clone(), false) {
            let Ok(line) = line else { read_failed = true; break };

            reader_run_scope.observe_stdout(Operation::RecordProcessing);
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
            let control_reply = control_replies.as_ref().is_some_and(|session| session.is_control_reply(&line));
            if !control_reply && let Ok(mut g) = stdout_byte_metrics.lock() {
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
                reader_run_scope.observe_stdout(Operation::JsonDecode);
                let trimmed = line.trim_start();
                if trimmed.starts_with('{')
                    && let Ok(payload) = serde_json::from_str::<serde_json::Value>(trimmed)
                {
                    reader_run_scope.observe_stdout(Operation::SignalObservation);
                    stdout_signal_hub.observe_json(SignalSource::Stream, &payload);
                }
            }

            feed_and_publish(&mut *parser, &line);
        }

        if !read_failed && let Some(capture) = stream_capture.as_mut() { capture.reached_eof(); }
        drop(stream_capture);

        // Handed back before the final render so a render that outlives the
        // join still leaves the run its real summary.
        *reader_parser_slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(parser);
        drain_close(&text_renderer, &framed_close, &mut out);
    });

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
    let (control_early_rx, completion_rx) = control_receivers.unzip();
    let early_terminate_rx = merge_early(bridge_early_rx.or(content_early_rx), control_early_rx);
    let has_bridge = bridge_for_thread.is_some();
    let capture_always = has_bridge;
    let first_stderr_at_clone = Arc::clone(&first_stderr_at);
    let stderr_byte_metrics = live_metrics.clone();
    // Bounded ring of the stderr lines that reach the terminal, for the
    // native-exit failure report.
    let stderr_tail_ring: Arc<std::sync::Mutex<std::collections::VecDeque<String>>> =
        Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
    let stderr_tail_ring_clone = Arc::clone(&stderr_tail_ring);
    let stderr_progress = ReaderProgress::default();
    let stderr_reader_progress = stderr_progress.clone();
    let stderr_output = stream_output.clone();
    let stderr_run_scope = run_scope.clone();
    let stderr_handle = thread::spawn(move || {
        let _stderr_guard = stderr_span.enter();
        // The same run as the stdout reader: once it is closed, passthrough
        // lines are refused instead of landing in a later run.
        let _scope_guard = stderr_run_scope.enter_stderr();
        let mut captured = String::new();
        for line in stderr_reader_progress.track_observed(stderr_source, stderr_run_scope.clone(), true) {
            let Ok(line) = line else { break };
            stderr_run_scope.observe_stderr(Operation::RecordProcessing);

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

            {
                let mut ring = stderr_tail_ring_clone.lock().unwrap();
                if ring.len() == claudine::signals::EXIT_STDERR_TAIL_LINES {
                    ring.pop_front();
                }
                ring.push_back(line.clone());
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
                stderr_output.emit_stderr_undecorated(&output_line);
            }
        }
        captured
    });

    // A controlled child already has its task; its stdin stays with the
    // session.
    if control.is_none()
        && let Some(seed) = stdin_seed
    {
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
    let needs_advanced_wait = early_terminate_rx.is_some() || watchdog_enabled || completion_rx.is_some();
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
        wait_with_signal_early_termination_and_completion(
            &mut child,
            true,
            rx,
            wd_rx,
            completion_rx,
            timeout_config.kill_grace,
            false,
        )?
    } else {
        let (code, term) = wait_with_signal_handling(&mut child, true)?;
        (code, term, None)
    };
    // The child is gone: nothing more can be written to it, and anything
    // waiting on its answers must stop waiting.
    if let Some((session, _)) = &control {
        session.finish();
    }

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
    // A controlled provider's tools may live outside its process group; a
    // survivor is reported separately from the run's own outcome.
    #[cfg(unix)]
    if let Some(watch) = descendant_watch {
        let survivors = watch.reap(timeout_config.kill_grace);
        if survivors > 0 {
            let noun = if survivors == 1 { "process" } else { "processes" };
            let message = format!(
                "{survivors} tool {noun} started by the provider outlived it; Claudine terminated {}",
                if survivors == 1 { "it" } else { "them" }
            );
            stream_output.emit_stderr_line(&Status::new(&message).state(StatusState::Warning).render(&termination_term));
        }
    }
    stop_timing_ticker(flush_ticker);
    stop_timing_ticker(timing_monitor);
    stop_timing_ticker(watchdog_ticker);

    #[cfg(feature = "test-fixtures")]
    let reader_budget = super::super::completion_fixture::budget();
    #[cfg(not(feature = "test-fixtures"))]
    let reader_budget = ReaderBudget::default();
    let readers_since = Instant::now();
    run_scope.observe_settlement(Operation::JoinStart);
    let stdout_outcome = join_reader(stdout_handle, &stdout_progress, reader_budget, readers_since);
    run_scope.observe_settlement(if matches!(stdout_outcome, JoinOutcome::TimedOut(_)) { Operation::Cutoff } else { Operation::JoinEnd });
    if timeout_config.provider == Some(claudine::provider::Provider::Codex)
        && let Some(index) = args.iter().position(|arg| arg == "--output-last-message")
        && let Some(path) = args.get(index + 1)
        && let Ok((text, complete)) = last_message(Path::new(path))
    {
        // File text never establishes a verdict or replaces parsed text.
        run_scope.retain_fallback_answer(&text, complete);
    }
    let completion_observation = run_scope.freeze_with_output(Some(stream_output.observation_since(started_at)));
    let (parser, stdout_warning) = settle_parser(
        stdout_outcome,
        &parser_slot,
        exit_code,
        reader_budget,
        &run_scope.snapshot(),
    );
    // The summary is settled. A reader still running past this point is
    // abandoned: its output and lifecycle events are discarded, so it can
    // neither write into the next iteration nor emit for a finished run.
    run_scope.close();
    let stderr_outcome = join_reader(stderr_handle, &stderr_progress, reader_budget, readers_since);
    let stderr_warning = reader_failure(ReaderStream::Stderr, &stderr_outcome, reader_budget)
        .map(|failure| failure.message);
    let captured = match stderr_outcome {
        JoinOutcome::Joined(captured) => captured,
        JoinOutcome::Panicked(_) | JoinOutcome::TimedOut(_) => String::new(),
    };
    let reader_warnings: Vec<String> =
        [stdout_warning, stderr_warning].into_iter().flatten().collect();
    for warning in &reader_warnings {
        tracing::warn!("{warning}");
        stream_output.emit_stderr_line(&reader_warning_line(warning, &termination_term));
    }
    if suppress_stderr_on_success && exit_code != 0 && !captured.is_empty() {
        stream_output.emit_stderr_undecorated(&captured);
    }
    // Everything queued so far gets until the same clock the readers were
    // joined against; the trailer written after this run waits on what is left.
    let output_deadline = readers_since + reader_budget.drain_limit;
    stream_output.set_drain_deadline(output_deadline);
    stream_output.drain(output_deadline);

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

    run_scope.observe_settlement(Operation::SummaryConstruction);
    let mut summary = parser.finish(exit_code);
    run_scope.observe_settlement(Operation::Settlement);
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

    #[cfg(feature = "test-fixtures")]
    super::super::completion_fixture::publish(timeout_config.provider, exit_code, &completion_observation)?;

    let result = ProcessResult {
        data: summary,
        completion_observation: Some(completion_observation),
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed: started_at.elapsed(),
            first_response_latency: first_response,
        },
        agent_pid: Some(captured_pid),
        guard_context,
        signals: signal_hub.drain(),
        reader_warnings,
        stream_tails: Some(super::super::StreamTails {
            stdout: stdout_tail,
            stderr: stderr_tail_ring.lock().unwrap().iter().cloned().collect::<Vec<_>>().join("\n"),
        }),
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
