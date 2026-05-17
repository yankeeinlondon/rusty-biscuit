use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use claudine::stream::logs::{EarlyTermination, StderrBridgeHandle, StderrIngestOutcome};
use claudine::stream::parser::{SemanticStreamParser, StreamParseError};
use claudine::stream::progress::LiveMetrics;
use claudine::stream::prompt_timing::PromptTimingContext;
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use tracing::{Span, info_span};

use std::sync::Mutex;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};

use super::super::section::SectionTracker;
use super::super::stream_io::StreamOutput;
use super::exit::exit_code_from_status;
use super::stream_capture::StreamCapture;
use super::subagent_watchdog::WatchdogState;
use super::termination::{
    WatchdogTermination, apply_early_termination_to_summary, wait_with_signal_and_early_termination,
};
use super::timeouts::{TimeoutConfig, wait_with_timeout};
use super::watchdog::{
    spawn_flush_if_idle_ticker, spawn_prompt_timing_monitor, spawn_timeout_watchdog_ticker,
};
use super::{
    ChildIoOptions, ErrorParser, OutputTextCallback, ProcessResult, ProcessTelemetry,
    ReasoningCallback, SemanticParserBuilder, StreamTextRenderer, join_with_timeout,
    join_with_timeout_or, kill_process_group, stop_timing_ticker,
};

/// Spawn the provider child process and return its exit code.
///
/// ## Environment
///
/// The `env` parameter must be the **complete** environment for the child
/// process. The child is launched with `env_clear()` followed by `envs(env)`,
/// so any variable not present in `env` will be absent from the child. This
/// is the only gate for environment sanitization — if a variable is missing
/// from `env`, it will not reach the child.
///
/// ## Signal Handling
///
/// If Claudine receives a second SIGINT (Ctrl-C) while waiting for the child,
/// it sends SIGTERM to the child. A third SIGINT sends SIGKILL.
///
/// ## Timeout
///
/// When `timeout` is `Some(seconds)`, the child is sent SIGTERM after the
/// specified duration, followed by SIGKILL after a 5-second grace period.
///
/// ## Stdout Filtering
///
/// When `stdout_noise_prefixes` is non-empty, stdout is piped through a
/// filter that suppresses lines starting with any of the given prefixes.
/// This is used in non-interactive mode to strip provider debug noise
/// (e.g. Gemini CLI's hook execution logs) from the response.
pub(crate) fn run_child(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    io: ChildIoOptions<'_>,
    child_spawned: &mut bool,
) -> Result<ProcessResult<i32>> {
    // Debug assertion: critical variables must be present.
    debug_assert!(
        env.contains_key(&OsString::from("PATH")),
        "child env is missing PATH — env::build_child_env likely has a bug"
    );
    debug_assert!(
        env.contains_key(&OsString::from("HOME")),
        "child env is missing HOME — env::build_child_env likely has a bug"
    );

    let filter_stdout = !io.stdout_noise_prefixes.is_empty();
    let filter_stderr = !io.stderr_noise_prefixes.is_empty();

    let needs_stdin_pipe = io.stdin_seed.is_some();

    // Whether we isolate the child into its own process group. Needed only
    // when we pipe streams (so we can clean up orphaned descendants that
    // keep the pipe fds open — see `kill_process_group`). For pure TTY
    // inheritance (interactive TUIs like Claude/Codex), isolating into a
    // background pgroup causes the child to receive SIGTTIN on stdin read
    // and hang indefinitely.
    let isolate_process_group = filter_stdout || filter_stderr || needs_stdin_pipe;

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(if needs_stdin_pipe {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stdout(if filter_stdout {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stderr(if filter_stderr {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });

    #[cfg(unix)]
    if isolate_process_group {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let spawned_at = Instant::now();
    let mut child = command.spawn()?;
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(child.id()));

    // Shared first-response trackers. Each channel stamps the first
    // non-filtered line it sees so we can compute best-effort latency
    // even on the legacy passthrough path.
    let first_stdout_at: Option<Arc<std::sync::Mutex<Option<Instant>>>> = if filter_stdout {
        Some(Arc::new(std::sync::Mutex::new(None)))
    } else {
        None
    };
    let first_stderr_at: Option<Arc<std::sync::Mutex<Option<Instant>>>> = if filter_stderr {
        Some(Arc::new(std::sync::Mutex::new(None)))
    } else {
        None
    };

    // Spawn stdout/stderr reader threads BEFORE writing stdin to avoid a
    // pipe deadlock: if the prompt exceeds the OS pipe buffer (~64 KB on
    // macOS) and the child writes to stdout/stderr during startup, both
    // processes block on pipe I/O with no reader on the other end.
    let stdout_handle = if filter_stdout {
        let pipe = child.stdout.take().expect(
            "child stdout must be piped: Stdio::piped() was set on the child Command above",
        );
        let prefixes: Vec<String> = io
            .stdout_noise_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plain = crate::log::is_plain();
        let first_at = first_stdout_at.clone().expect("set when filter_stdout");
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut out = std::io::stdout().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
                }
                {
                    let mut g = first_at.lock().unwrap();
                    if g.is_none() {
                        *g = Some(Instant::now());
                    }
                }
                let stripped = if plain {
                    biscuit_terminal::prelude::strip_escape_codes(&line)
                } else {
                    line
                };
                let _ = writeln!(out, "{stripped}");
            }
        }))
    } else {
        None
    };

    let stderr_handle = if filter_stderr {
        let pipe = child.stderr.take().expect(
            "child stderr must be piped: Stdio::piped() was set on the child Command above",
        );
        let prefixes: Vec<String> = io
            .stderr_noise_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plain = crate::log::is_plain();
        let first_at = first_stderr_at.clone().expect("set when filter_stderr");
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut err = std::io::stderr().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
                }
                {
                    let mut g = first_at.lock().unwrap();
                    if g.is_none() {
                        *g = Some(Instant::now());
                    }
                }
                let stripped = if plain {
                    biscuit_terminal::prelude::strip_escape_codes(&line)
                } else {
                    line
                };
                let _ = writeln!(err, "{stripped}");
            }
        }))
    } else {
        None
    };

    // Write stdin seed AFTER reader threads are spawned (see deadlock note above).
    //
    // A `BrokenPipe` here is benign: it means the child closed stdin (or
    // exited) before we finished writing the seed. That is a legitimate child
    // behavior — for example, agent stubs in tests, or providers that ignore
    // their seed and exit immediately. Treat it as success and let the
    // subsequent `wait_with_*` decide the real exit status. Any other I/O
    // error still propagates.
    if let Some(seed) = io.stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        match stdin_pipe.write_all(seed.as_bytes()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                // Child closed stdin or exited early; nothing more to do.
            }
            Err(e) => return Err(e.into()),
        }
        // Drop closes the pipe so the child sees EOF.
    }

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child, isolate_process_group)?
    };

    if isolate_process_group {
        kill_process_group(&mut child);
    }

    let thread_join_timeout = Duration::from_secs(5);
    if let Some(handle) = stdout_handle {
        join_with_timeout(handle, thread_join_timeout);
    }
    if let Some(handle) = stderr_handle {
        join_with_timeout(handle, thread_join_timeout);
    }

    let total_elapsed = spawned_at.elapsed();
    let first_response = super::resolve_first_response(
        None,
        first_stdout_at.as_ref().and_then(|a| *a.lock().unwrap()),
        first_stderr_at.as_ref().and_then(|a| *a.lock().unwrap()),
        spawned_at,
    );

    Ok(ProcessResult {
        data: exit_code,
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed,
            first_response_latency: first_response,
        },
    })
}

/// Wait for the child, forwarding SIGINT/SIGTERM on repeated Ctrl-C.
///
/// When `child_in_own_pgroup` is true, the child was spawned with
/// `process_group(0)` and the installed SIGINT handler manually forwards
/// signals to `-child_pid` so descendants also receive them. When it is
/// false, the child shares the parent's process group (required for
/// interactive TUIs that read the controlling TTY); in that case the
/// terminal already delivers SIGINT to the child naturally, so we only
/// track the interrupt count locally.
///
/// Returns `(exit_code, termination_kind)`.
#[cfg(unix)]
fn wait_with_signal_handling(
    child: &mut Child,
    child_in_own_pgroup: bool,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_exited = Arc::new(AtomicBool::new(false));
    let child_pid = child.id();

    let counter = Arc::clone(&interrupt_count);
    let exited = Arc::clone(&child_exited);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            // Never signal a PID we no longer own — the kernel may have
            // recycled `child_pid` onto an unrelated process by now.
            if exited.load(Ordering::SeqCst) {
                return;
            }
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if !child_in_own_pgroup {
                // Child shares our process group; the terminal already
                // delivered SIGINT to it. Just track the count so the
                // termination kind is reported correctly.
                return;
            }
            match count {
                1 => {
                    libc::kill(-(child_pid as i32), libc::SIGINT);
                }
                2 => {
                    libc::kill(-(child_pid as i32), libc::SIGTERM);
                }
                _ => {
                    libc::kill(-(child_pid as i32), libc::SIGKILL);
                }
            }
        })
    }?;

    let status = child.wait()?;
    // Set the exit flag BEFORE the `_guard` drops so a SIGINT that arrives
    // in the narrow window between `wait` returning and the guard being
    // dropped still sees an exited child and refuses to signal the PID.
    child_exited.store(true, Ordering::SeqCst);
    let code = exit_code_from_status(status);
    let was_interrupted = interrupt_count.load(Ordering::SeqCst) > 0;
    let termination = if was_interrupted {
        claudine::harness::ProcessTermination::Interrupted
    } else {
        claudine::harness::ProcessTermination::Completed
    };
    Ok((code, termination))
}

#[cfg(not(unix))]
fn wait_with_signal_handling(
    child: &mut Child,
    _child_in_own_pgroup: bool,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    let status = child.wait()?;
    Ok((
        exit_code_from_status(status),
        claudine::harness::ProcessTermination::Completed,
    ))
}

/// Captured output from a child process.
pub(crate) struct CapturedChildOutput {
    pub(crate) exit_code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

/// Spawn a provider child process and capture its output.
///
/// Behaves like `run_child()` but pipes stdout and stderr into strings
/// instead of forwarding to the terminal. Noise filtering still applies
/// to the captured output. No output is printed live.
pub(crate) fn run_child_capture(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    io: ChildIoOptions<'_>,
    child_spawned: &mut bool,
) -> Result<ProcessResult<CapturedChildOutput>> {
    debug_assert!(
        env.contains_key(&OsString::from("PATH")),
        "child env is missing PATH"
    );
    debug_assert!(
        env.contains_key(&OsString::from("HOME")),
        "child env is missing HOME"
    );

    let needs_stdin_pipe = io.stdin_seed.is_some();

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(if needs_stdin_pipe {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let spawned_at = Instant::now();
    let mut child = command.spawn()?;
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(child.id()));

    // Shared first-response trackers (always piped in capture mode).
    let first_stdout_at = Arc::new(std::sync::Mutex::new(None));
    let first_stderr_at = Arc::new(std::sync::Mutex::new(None));

    // Spawn reader threads BEFORE writing stdin (see run_child deadlock note).

    // Capture stdout into a string, applying noise filtering
    let stdout_pipe = child
        .stdout
        .take()
        .expect("child stdout must be piped: Stdio::piped() was set on the child Command above");
    let stdout_noise: Vec<String> = io
        .stdout_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let first_stdout_at_clone = Arc::clone(&first_stdout_at);
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if stdout_noise.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            {
                let mut g = first_stdout_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(Instant::now());
                }
            }
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    // Capture stderr into a string, applying noise filtering
    let stderr_pipe = child
        .stderr
        .take()
        .expect("child stderr must be piped: Stdio::piped() was set on the child Command above");
    let stderr_noise: Vec<String> = io
        .stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let first_stderr_at_clone = Arc::clone(&first_stderr_at);
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if stderr_noise.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            {
                let mut g = first_stderr_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(Instant::now());
                }
            }
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    // Write stdin seed AFTER reader threads are spawned (see run_child deadlock note).
    // A `BrokenPipe` here is benign — the child closed stdin or exited before
    // we finished writing the seed. See `run_child` for the same rationale.
    if let Some(seed) = io.stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        match stdin_pipe.write_all(seed.as_bytes()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => return Err(e.into()),
        }
    }

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child, true)?
    };

    kill_process_group(&mut child);

    let thread_join_timeout = Duration::from_secs(5);
    let stdout = join_with_timeout_or(stdout_handle, thread_join_timeout, String::new());
    let stderr = join_with_timeout_or(stderr_handle, thread_join_timeout, String::new());

    let total_elapsed = spawned_at.elapsed();
    let first_response = super::resolve_first_response(
        None,
        *first_stdout_at.lock().unwrap(),
        *first_stderr_at.lock().unwrap(),
        spawned_at,
    );

    Ok(ProcessResult {
        data: CapturedChildOutput {
            exit_code,
            stdout,
            stderr,
        },
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed,
            first_response_latency: first_response,
        },
    })
}

/// Spawn a provider child process with structured semantic stream parsing.
///
/// This is the Phase 3.4 replacement for [`run_child_stream`]. The
/// difference is the stdout loop: instead of switching on a returned
/// [`SemanticEvent`]s, the parser drives a [`SemanticEventSink`] that the
/// caller has already wired up for status rendering, dispatch, metrics,
/// and JSONL logging. This function's only rendering responsibility is
/// wiring the terminal-local `StreamTextRenderer` instance to the sink
/// through the builder callback so it can run inside the parser thread.
/// Reasoning rendering is owned entirely by `LiveSemanticSink`.
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
) -> Result<ProcessResult<StreamExecutionSummary>> {
    debug_assert!(env.contains_key(&OsString::from("PATH")));
    debug_assert!(env.contains_key(&OsString::from("HOME")));

    let needs_stdin_pipe = stdin_seed.is_some();
    let started_at = Instant::now();
    let started_at_wall = chrono::Local::now();

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(if needs_stdin_pipe {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command.spawn()?;
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(child.id()));

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
    let text_renderer: Arc<std::sync::Mutex<StreamTextRenderer>> =
        Arc::new(std::sync::Mutex::new(StreamTextRenderer::new()));

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
                    r.push(&mut writer, chunk);
                }
            })
        };
        let reasoning_cb: ReasoningCallback = Box::new(|_chunk: &str| {});

        let mut parser: Box<dyn SemanticStreamParser> = build_parser(output_cb, reasoning_cb);
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
                        r.flush_remaining(&mut out);
                    }
                    fallback_mode = true;
                    let _ = writeln!(out, "{}", crate::log::maybe_strip(&line));
                }
            }
        }

        if let Ok(mut r) = text_renderer.lock() {
            r.flush_remaining(&mut out);
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
    let (mut bridge_for_thread, finalize_for_main, early_terminate_rx) = match stderr_bridge {
        Some(StderrBridgeHandle {
            bridge,
            finalize,
            early_terminate,
        }) => (Some(bridge), Some(finalize), early_terminate),
        None => (None, None, None),
    };
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

    if let Some(seed) = stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        // BrokenPipe is benign: child closed stdin or exited before we
        // finished writing the seed. See `run_child` for the same rationale.
        match stdin_pipe.write_all(seed.as_bytes()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => return Err(e.into()),
        }
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
        wait_with_signal_and_early_termination(
            &mut child,
            true,
            rx,
            wd_rx,
            timeout_config.kill_grace,
        )?
    } else {
        let (code, term) = wait_with_signal_handling(&mut child, true)?;
        (code, term, None)
    };

    if let Some(
        EarlyTermination::Timeout { message } | EarlyTermination::StepTimeout { message, .. },
    ) = early_termination.as_ref()
    {
        let rendered = Status::new(message)
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

    let first_response = super::resolve_first_response(
        *first_semantic_at.lock().unwrap(),
        *first_raw_stdout_at.lock().unwrap(),
        *first_stderr_at.lock().unwrap(),
        started_at,
    );

    Ok(ProcessResult {
        data: summary,
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed: started_at.elapsed(),
            first_response_latency: first_response,
        },
    })
}
