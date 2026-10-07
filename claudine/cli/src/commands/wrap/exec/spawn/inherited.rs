//! Inherited-output spawn mode: forwards child stdout/stderr to the terminal,
//! with optional line-prefix noise filtering.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use claudine::stream::logs::EarlyTermination;
use color_eyre::eyre::Result;
use tracing::Span;

use super::super::termination::wait_with_signal_and_early_termination;
use super::super::super::output_worker::Drained;
use super::super::super::run_scope::RunScope;
use super::super::super::stream_io::StreamOutput;
use super::super::reader_join::{
    JoinOutcome, ReaderBudget, ReaderProgress, ReaderStream, join_reader, queue_reader_warnings,
    reader_failure,
};
use super::super::timeouts::TimeoutConfig;
use super::super::{
    ChildIoOptions, ProcessResult, ProcessTelemetry, kill_process_group,
    resolve_first_response, stop_timing_ticker,
};
use super::setup;

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
///
/// ## Forwarding
///
/// Filtered lines, and the warnings about their readers, are queued on the
/// process-wide [`StreamOutput`] rather than written here, so a terminal that
/// stopped accepting output delays the return by at most the reader drain
/// limit. A forwarder still running at cutoff has its later lines refused.
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
    run_child_on(
        binary,
        args,
        env,
        cwd,
        timeout,
        interactive,
        io,
        child_spawned,
        &StreamOutput::shared(),
        ReaderBudget::default(),
    )
}

/// [`run_child`] forwarding through `output` under `budget`; tests inject both.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_child_on(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    interactive: bool,
    io: ChildIoOptions<'_>,
    child_spawned: &mut bool,
    output: &Arc<StreamOutput>,
    budget: ReaderBudget,
) -> Result<ProcessResult<i32>> {
    setup::debug_assert_child_env(env);

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

    let mut command = setup::base_command(binary, args, env, cwd);
    command
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

    if isolate_process_group {
        setup::isolate_into_process_group(&mut command);
    }

    let spawned_at = Instant::now();
    let mut child = command.spawn()?;
    let captured_pid = child.id();
    *child_spawned = true;
    crate::budget::record_child(captured_pid);
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
    let run_scope = RunScope::default();
    let loss_mark = output.loss();
    let stdout_handle = first_stdout_at.as_ref().map(|first_at| {
        let pipe = child.stdout.take().expect(
            "child stdout must be piped: Stdio::piped() was set on the child Command above",
        );
        spawn_forwarder(
            ReaderStream::Stdout,
            pipe,
            output,
            &run_scope,
            io.stdout_noise_prefixes,
            Arc::clone(first_at),
        )
    });
    let stderr_handle = first_stderr_at.as_ref().map(|first_at| {
        let pipe = child.stderr.take().expect(
            "child stderr must be piped: Stdio::piped() was set on the child Command above",
        );
        spawn_forwarder(
            ReaderStream::Stderr,
            pipe,
            output,
            &run_scope,
            io.stderr_noise_prefixes,
            Arc::clone(first_at),
        )
    });

    // Write stdin seed AFTER reader threads are spawned (see deadlock note above).
    //
    // A `BrokenPipe` here is benign: it means the child closed stdin (or
    // exited) before we finished writing the seed. That is a legitimate child
    // behavior — for example, agent stubs in tests, or providers that ignore
    // their seed and exit immediately. Treat it as success and let the
    // subsequent `wait_with_*` decide the real exit status. Any other I/O
    // error still propagates.
    if let Some(seed) = io.stdin_seed {
        setup::write_stdin_seed(&mut child, seed)?;
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
        let (timeout_ticker, watchdog_rx) = setup::wall_clock_timeout_ticker(timeout, spawned_at);
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

    let readers_since = Instant::now();
    let mut reader_warnings =
        join_forwarders([stdout_handle, stderr_handle], budget, readers_since);
    // A forwarder still running is detached; anything it reads from here on
    // is refused rather than presented after the run ended.
    run_scope.close();
    if filter_stdout || filter_stderr {
        queue_reader_warnings(output, &reader_warnings);
        let drained = output.drain(readers_since + budget.drain_limit);
        let loss = output.loss().since(&loss_mark);
        let stalled_now = drained == Drained::Disabled && !loss_mark.stalled;
        if stalled_now || loss.dropped_frames + loss.rejected_frames + loss.failed_writes > 0 {
            let warning = "Claudine could not forward all of the agent's output to the \
                           terminal: it was not accepting output"
                .to_string();
            tracing::warn!("{warning}");
            reader_warnings.push(warning);
        }
    }

    let total_elapsed = spawned_at.elapsed();
    let first_response = resolve_first_response(
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
        stream_tails: None,
        reader_warnings,
    })
}

/// A thread forwarding one of the child's streams to this process's own.
pub(super) struct ForwardReader {
    pub(super) stream: ReaderStream,
    pub(super) progress: ReaderProgress,
    pub(super) handle: thread::JoinHandle<Option<std::io::Error>>,
}

/// Join the forwarders under one shared clock and return a warning for each
/// that panicked, outlived its bound, or failed to write.
///
/// A forwarding problem never changes the run's outcome: the child's exit
/// code and termination are already known, and only its output may be
/// incomplete.
pub(super) fn join_forwarders(
    readers: [Option<ForwardReader>; 2],
    budget: ReaderBudget,
    since: Instant,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for reader in readers.into_iter().flatten() {
        let outcome = join_reader(reader.handle, &reader.progress, budget, since);
        match &outcome {
            JoinOutcome::Joined(Some(error)) => warnings.push(format!(
                "Claudine could not forward the agent's {} to the terminal: {error}",
                reader.stream.label()
            )),
            _ => warnings
                .extend(reader_failure(reader.stream, &outcome, budget).map(|f| f.message)),
        }
    }
    warnings
}

/// Start a thread forwarding `pipe`'s lines to `stream` through `output`, as
/// part of `scope`'s run.
pub(super) fn spawn_forwarder<R: Read + Send + 'static>(
    stream: ReaderStream,
    pipe: R,
    output: &Arc<StreamOutput>,
    scope: &RunScope,
    noise_prefixes: &[&str],
    first_at: Arc<std::sync::Mutex<Option<Instant>>>,
) -> ForwardReader {
    let prefixes: Vec<String> = noise_prefixes.iter().map(|s| s.to_string()).collect();
    let plain = crate::log::is_plain();
    let mut out: Box<dyn Write + Send> = match stream {
        ReaderStream::Stderr => Box::new(output.stderr_writer()),
        ReaderStream::Stdout | ReaderStream::Output => Box::new(output.stdout_writer()),
    };
    let progress = ReaderProgress::default();
    let reader_progress = progress.clone();
    let scope = scope.clone();
    let handle = thread::spawn(move || {
        let _scope_guard = scope.enter();
        forward_lines(
            BufReader::new(pipe),
            &mut out,
            &prefixes,
            &first_at,
            plain,
            &reader_progress,
        )
    });
    ForwardReader {
        stream,
        progress,
        handle,
    }
}

/// Copy `reader`'s lines to `out`, skipping noise lines.
///
/// Keeps draining the pipe after a write fails, so the child is never wedged
/// on a full pipe, and returns the first write error.
pub(super) fn forward_lines<R: BufRead, W: Write>(
    reader: R,
    out: &mut W,
    prefixes: &[String],
    first_at: &Arc<std::sync::Mutex<Option<Instant>>>,
    plain: bool,
    progress: &ReaderProgress,
) -> Option<std::io::Error> {
    let mut first_error = None;
    for line in progress.track(reader.lines()) {
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
        // One write per line: each write is one frame for the output worker.
        if let Err(error) = out.write_all(format!("{stripped}\n").as_bytes()) {
            first_error.get_or_insert(error);
        }
    }
    first_error
}
