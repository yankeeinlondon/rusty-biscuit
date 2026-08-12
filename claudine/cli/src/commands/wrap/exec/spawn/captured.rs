//! Captured-output spawn mode: pipes child stdout/stderr into strings with
//! noise filtering and the per-run volume cap, instead of forwarding live.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use claudine::signals::{SignalHub, SignalSource};
use claudine::stream::logs::EarlyTermination;
use color_eyre::eyre::Result;
use tracing::Span;

use super::super::termination::{
    early_termination_guard_context, trip_to_early_termination,
    wait_with_signal_and_early_termination,
};
use super::super::timeouts::TimeoutConfig;
use super::super::{
    ChildIoOptions, ProcessResult, ProcessTelemetry, join_with_timeout_or, kill_process_group,
    resolve_first_response, stop_timing_ticker,
};
use super::setup;

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
    setup::debug_assert_child_env(env);

    let needs_stdin_pipe = io.stdin_seed.is_some();

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
    if let Some(seed) = io.stdin_seed {
        setup::write_stdin_seed(&mut child, seed)?;
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
        let (timeout_ticker, watchdog_rx) = setup::wall_clock_timeout_ticker(timeout, spawned_at);
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
    crate::commands::wrap::catalog_drift::emit_resolved_model_drift(&signal_hub);
    // End-of-run harvest flush (E6): persist unmatched error/warning-class
    // candidates when opted in; a no-op otherwise (and always on the
    // hub-less fallback, which cannot enable harvesting).
    claudine::signals::harvest::flush_hub(&signal_hub);
    let signals = signal_hub.drain();

    let total_elapsed = spawned_at.elapsed();
    let first_response = resolve_first_response(
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
pub(super) fn capture_stream_with_volume_cap<R: BufRead>(
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
            let _ = early_tx.send(trip_to_early_termination(trip));
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
