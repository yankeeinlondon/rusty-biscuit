use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use claudine::signals::{SignalHub, SignalSource};
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
    WatchdogTermination, apply_early_termination_to_summary, early_termination_guard_context,
    early_termination_message, wait_with_signal_and_early_termination,
};
use super::timeouts::TimeoutConfig;
use super::watchdog::{
    spawn_flush_if_idle_ticker, spawn_prompt_timing_monitor, spawn_timeout_watchdog_ticker,
    spawn_wall_clock_timeout_ticker,
};
use super::{
    ChildIoOptions, ErrorParser, OutputTextCallback, ProcessResult, ProcessTelemetry,
    ReasoningCallback, SemanticParserBuilder, join_with_timeout, join_with_timeout_or,
    kill_process_group, new_assistant_stream, stop_timing_ticker,
};
use claudine::render::{AssistantStream, StreamRenderable};

/// Windows process-creation flag that puts the child in a new process group
/// (the Win32 `CREATE_NEW_PROCESS_GROUP`). A new group is the prerequisite for
/// targeting the child tree with `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT,
/// group)` from the unified wait loop. Defined locally to avoid pulling the
/// `windows` crate into the `Command` build site.
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

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
/// The wait is delegated to the shared `wait_with_signal_and_early_termination`
/// loop, so this path owns the same SIGINT ladder and group-targeted
/// SIGTERM→SIGKILL escalation as every other spawn path. `interactive` selects
/// the ladder: `true` keeps the full `SIGINT → SIGTERM → SIGKILL` (three
/// presses to force-kill); `false` compresses it to `SIGTERM → SIGKILL` (F5).
///
/// ## Timeout
///
/// When `timeout` is `Some(seconds)`, a wall-clock ticker feeds the unified
/// wait loop, which terminates the child's process group on breach (SIGTERM,
/// escalating to SIGKILL after the configured `kill_grace`). Routing the
/// timeout through the signal-aware loop is what keeps Ctrl+C effective even
/// when a `timeout` is set.
///
/// ## Stdout Filtering
///
/// When `stdout_noise_prefixes` is non-empty, stdout is piped through a
/// filter that suppresses lines starting with any of the given prefixes.
/// This is used in non-interactive mode to strip provider debug noise
/// (e.g. Gemini CLI's hook execution logs) from the response.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_child(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    interactive: bool,
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

    // Windows parity (Q15): isolate the child into a new process group so the
    // unified wait loop's Job Object + `GenerateConsoleCtrlEvent` can target
    // the whole tree. Mirrors the `process_group(0)` condition above.
    #[cfg(windows)]
    if isolate_process_group {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    let spawned_at = Instant::now();
    let mut child = command.spawn()?;
    let captured_pid = child.id();
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(captured_pid));

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

    // Every spawn path now routes through the one signal-aware wait loop
    // (Phase 5): it owns the SIGINT ladder and group-targeted SIGTERM→SIGKILL
    // escalation, so a configured `timeout` can no longer disable Ctrl+C. The
    // direct path has no content guards, so the early-termination channel
    // stays open but idle — keeping `_early_tx` alive avoids a per-poll
    // disconnect warning. A configured wall-clock `timeout` is enforced by a
    // minimal ticker feeding the same watchdog channel the streaming path uses.
    let (exit_code, termination) = {
        let kill_grace = TimeoutConfig::resolve(None, None).kill_grace;
        let (_early_tx, early_rx) = std::sync::mpsc::channel::<EarlyTermination>();
        let (timeout_ticker, watchdog_rx) = match timeout {
            Some(seconds) => {
                let (tx, rx) = std::sync::mpsc::channel();
                let ticker =
                    spawn_wall_clock_timeout_ticker(Duration::from_secs(seconds), spawned_at, tx);
                (Some(ticker), Some(rx))
            }
            None => (None, None),
        };
        let (code, termination, _early) = wait_with_signal_and_early_termination(
            &mut child,
            isolate_process_group,
            early_rx,
            watchdog_rx,
            kill_grace,
            interactive,
        )?;
        stop_timing_ticker(timeout_ticker);
        (code, termination)
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
        agent_pid: Some(captured_pid),
        // The direct path has no content guards (F3); a content trip can
        // never originate here.
        guard_context: None,
        signals: Vec::new(),
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

    // This loop installs a child-targeting SIGINT handler, so the
    // compose-scoped Ctrl+C guard must defer to it while it runs.
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();

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
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();
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
///
/// `signal_hub` is the run's signal fan-in when the caller has provider
/// attribution (exit-source records and bespoke exit mappings need the
/// provider table); `None` falls back to a bespoke-only hub that records
/// just the termination mirror.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_child_capture(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    interactive: bool,
    io: ChildIoOptions<'_>,
    child_spawned: &mut bool,
    volume_cap: Option<claudine::runaway::CaptureVolumeCap>,
    signal_hub: Option<Arc<SignalHub>>,
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

    // Windows parity (Q15): new process group so the unified wait loop can
    // target the whole tree. Mirrors the `process_group(0)` block above.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    let spawned_at = Instant::now();
    let mut child = command.spawn()?;
    let captured_pid = child.id();
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(captured_pid));

    // Shared first-response trackers (always piped in capture mode).
    let first_stdout_at = Arc::new(std::sync::Mutex::new(None));
    let first_stderr_at = Arc::new(std::sync::Mutex::new(None));

    // Unified early-termination channel (Phase 6). The capture path runs no
    // exit-expression or repetition detection (F3 — Ctrl+C + volume cap
    // only); the per-run volume cap below is the sole content-driven sender,
    // bounding the otherwise-unbounded capture `String`. Created before the
    // reader threads so each can hold a sender clone. Kept alive on the main
    // thread so the wait loop never sees a premature disconnect.
    let (early_tx, early_rx) = std::sync::mpsc::channel::<EarlyTermination>();

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
    let stdout_cap = volume_cap.clone();
    let stdout_early_tx = early_tx.clone();
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_pipe);
        capture_stream_with_volume_cap(
            reader,
            &stdout_noise,
            &first_stdout_at_clone,
            stdout_cap.as_ref(),
            &stdout_early_tx,
        )
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
    let stderr_cap = volume_cap.clone();
    let stderr_early_tx = early_tx.clone();
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        capture_stream_with_volume_cap(
            reader,
            &stderr_noise,
            &first_stderr_at_clone,
            stderr_cap.as_ref(),
            &stderr_early_tx,
        )
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

    // Route through the unified signal-aware wait loop (Phase 5) so Ctrl+C
    // terminates the child even with a `timeout` configured. The capture path
    // always isolates the child into its own process group (above), so signals
    // reach descendants. The reader threads above feed `early_rx` when the
    // per-run volume cap trips (F3); the wall-clock `timeout` is enforced by a
    // minimal ticker on the same loop.
    let signal_hub = signal_hub.unwrap_or_else(|| Arc::new(SignalHub::without_table()));
    let (exit_code, termination, guard_context, early_termination) = {
        let kill_grace = TimeoutConfig::resolve(None, None).kill_grace;
        // Drop the main-thread sender so the channel disconnects once both
        // reader threads finish — the reader-thread clones are the only
        // senders that should keep it alive.
        drop(early_tx);
        let (timeout_ticker, watchdog_rx) = match timeout {
            Some(seconds) => {
                let (tx, rx) = std::sync::mpsc::channel();
                let ticker =
                    spawn_wall_clock_timeout_ticker(Duration::from_secs(seconds), spawned_at, tx);
                (Some(ticker), Some(rx))
            }
            None => (None, None),
        };
        let (code, termination, early) = wait_with_signal_and_early_termination(
            &mut child,
            true,
            early_rx,
            watchdog_rx,
            kill_grace,
            interactive,
        )?;
        stop_timing_ticker(timeout_ticker);
        let guard_context = early.as_ref().and_then(early_termination_guard_context);
        (code, termination, guard_context, early)
    };

    kill_process_group(&mut child);

    let thread_join_timeout = Duration::from_secs(5);
    let stdout = join_with_timeout_or(stdout_handle, thread_join_timeout, String::new());
    let stderr = join_with_timeout_or(stderr_handle, thread_join_timeout, String::new());

    // Bespoke signal mirror (E5) for the capture path's terminations
    // (per-run volume cap, wall-clock timeout).
    if let Some(termination) = early_termination.as_ref() {
        signal_hub.emit_bespoke(termination.to_signal_event(), SignalSource::Stream);
    }
    // Exit source (E5): the same once-per-run
    // `{exit_code, stdout_tail, stderr_tail}` synthesis as the
    // structured-stream path (inert on the hub-less fallback, which compiles
    // no detection table).
    signal_hub.observe_json(
        SignalSource::Exit,
        &claudine::signals::exit_source_payload(exit_code, &stdout, &stderr),
    );
    // Resolved-model drift check against the expected-offerings baseline,
    // before flush/drain so any drift event rides this run's signals.
    // No-op on the hub-less fallback (no provider attribution).
    super::super::catalog_drift::emit_resolved_model_drift(&signal_hub);
    // End-of-run harvest flush (E6): persist unmatched error/warning-class
    // candidates when opted in; a no-op otherwise (and always on the
    // hub-less fallback, which cannot enable harvesting).
    claudine::signals::harvest::flush_hub(&signal_hub);
    let signals = signal_hub.drain();

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
        agent_pid: Some(captured_pid),
        guard_context,
        signals,
    })
}

/// Read a captured stream line-by-line, applying noise filtering and the
/// per-run volume cap (Phase 6, F3).
///
/// Returns the accumulated (noise-filtered) capture buffer. While the cap is
/// under its thresholds, each kept line is appended and counted. The moment
/// the running line/byte totals breach the cap, an
/// [`EarlyTermination::RunawayVolume`] is sent once on `early_tx` (so the
/// wait loop terminates the child) and the buffer **stops growing** — further
/// lines are drained from the pipe but discarded, bounding memory. A `None`
/// cap (or a disabled one) never trips and never sends.
fn capture_stream_with_volume_cap<R: BufRead>(
    reader: R,
    noise_prefixes: &[String],
    first_at: &Arc<std::sync::Mutex<Option<Instant>>>,
    cap: Option<&claudine::runaway::CaptureVolumeCap>,
    early_tx: &std::sync::mpsc::Sender<EarlyTermination>,
) -> String {
    let mut captured = String::new();
    let mut lines: u64 = 0;
    let mut bytes: u64 = 0;
    let mut tripped = false;
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if noise_prefixes.iter().any(|p| line.starts_with(p.as_str())) {
            continue;
        }
        {
            let mut g = first_at.lock().unwrap();
            if g.is_none() {
                *g = Some(Instant::now());
            }
        }

        // Once tripped, keep draining the pipe (so the child is not wedged
        // on a full pipe before the SIGTERM lands) but never grow the buffer.
        if tripped {
            continue;
        }

        // Account volume before appending: count the line plus its implicit
        // newline so the totals match the bytes the child emitted.
        lines = lines.saturating_add(1);
        bytes = bytes.saturating_add(line.len() as u64).saturating_add(1);

        if let Some(cap) = cap
            && let Some(trip) = cap.check(lines, bytes)
        {
            tripped = true;
            let _ = early_tx.send(super::termination::trip_to_early_termination(trip));
            // Drop this breaching line too — the buffer is now frozen.
            continue;
        }

        if !captured.is_empty() {
            captured.push('\n');
        }
        captured.push_str(&line);
    }
    captured
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

    // Windows parity (Q15): new process group so the unified wait loop can
    // target the whole tree. Mirrors the `process_group(0)` block above.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

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

    let first_response = super::resolve_first_response(
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
    super::super::catalog_drift::emit_resolved_model_drift(&signal_hub);
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

#[cfg(test)]
mod tests;
