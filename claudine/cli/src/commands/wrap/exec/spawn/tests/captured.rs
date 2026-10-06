//! Captured-output spawn mode: PID/env capture, wall-clock timeout, and the
//! per-run volume cap.

use super::super::super::reader_join::{ReaderBudget, ReaderProgress, ReaderStream};
use super::super::captured::{
    CaptureReader, capture_stream_with_volume_cap, join_captures,
};
use super::*;
use claudine::stream::logs::EarlyTermination;
use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Run the capture loop to completion and return the buffer it filled.
fn capture_all<R: BufRead>(
    reader: R,
    cap: Option<&claudine::runaway::CaptureVolumeCap>,
    tx: &Sender<EarlyTermination>,
) -> String {
    let first_at = Arc::new(Mutex::new(None));
    let buffer = Mutex::new(String::new());
    capture_stream_with_volume_cap(
        reader,
        &[],
        &first_at,
        cap,
        tx,
        &buffer,
        &ReaderProgress::default(),
    );
    buffer.into_inner().unwrap()
}

/// A pipe stand-in: reads block until bytes are sent, and hit EOF only when
/// the sender is dropped, so a held-open pipe is a sender kept alive.
struct HeldPipe(Receiver<Vec<u8>>, Vec<u8>);

impl Read for HeldPipe {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.1.is_empty() {
            match self.0.recv() {
                Ok(bytes) => self.1 = bytes,
                Err(_) => return Ok(0),
            }
        }
        let n = out.len().min(self.1.len());
        out[..n].copy_from_slice(&self.1[..n]);
        self.1.drain(..n);
        Ok(n)
    }
}

/// Short bounds so no test waits on the production 5 s / 120 s values.
const BUDGET: ReaderBudget = ReaderBudget {
    pipe_cap: Duration::from_millis(400),
    pipe_settle: Duration::from_millis(50),
    drain_limit: Duration::from_secs(30),
};

fn wait_until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "the reader never got there");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// A capture reader over `pipe`, plus the sender that holds the pipe open.
fn capture_reader(stream: ReaderStream) -> (CaptureReader, Sender<Vec<u8>>) {
    let (held_open, pipe) = mpsc::channel::<Vec<u8>>();
    let buffer = Arc::new(Mutex::new(String::new()));
    let progress = ReaderProgress::default();
    let (thread_buffer, thread_progress) = (Arc::clone(&buffer), progress.clone());
    let handle = std::thread::spawn(move || {
        let (tx, _rx) = mpsc::channel::<EarlyTermination>();
        capture_stream_with_volume_cap(
            BufReader::new(HeldPipe(pipe, Vec::new())),
            &[],
            &Arc::new(Mutex::new(None)),
            None,
            &tx,
            &thread_buffer,
            &thread_progress,
        );
    });
    (
        CaptureReader {
            stream,
            buffer,
            progress,
            handle,
        },
        held_open,
    )
}

/// A capture reader whose pipe has already reached EOF.
fn finished_reader(stream: ReaderStream, text: &str) -> CaptureReader {
    let (reader, held_open) = capture_reader(stream);
    held_open.send(text.as_bytes().to_vec()).unwrap();
    drop(held_open);
    wait_until(|| reader.handle.is_finished());
    reader
}

/// A pipe a descendant holds open: the reader keeps what it had collected,
/// the join gives up after the pipe cap, and the result says it is partial.
#[test]
fn a_held_open_capture_pipe_keeps_the_partial_buffer_and_warns() {
    let (stdout, held_open) = capture_reader(ReaderStream::Stdout);
    held_open.send(b"partial output
".to_vec()).unwrap();
    let buffer = Arc::clone(&stdout.buffer);
    wait_until(|| !buffer.lock().unwrap().is_empty());
    let stderr = finished_reader(ReaderStream::Stderr, "all of stderr\n");

    let ([out, err], warnings) = join_captures([stdout, stderr], BUDGET, Instant::now());

    assert_eq!(out, "partial output");
    assert_eq!(err, "all of stderr");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("agent's stdout"), "{}", warnings[0]);
    assert!(
        warnings[0].contains("held by a process the agent started"),
        "{}",
        warnings[0]
    );
    assert!(!warnings[0].contains("panicked"), "{}", warnings[0]);
    drop(held_open);
}

/// A panic is reported as a panic with its payload, a timeout as a timeout;
/// the two never share wording, and a non-string payload says so.
#[test]
fn capture_readers_report_panics_apart_from_timeouts() {
    let panicking = |stream: ReaderStream, payload_is_string: bool| {
        let buffer = Arc::new(Mutex::new(String::from("kept")));
        let progress = ReaderProgress::default();
        let reader_progress = progress.clone();
        let handle = std::thread::spawn(move || {
            if reader_progress.track(["line"].into_iter()).next().is_some() {
                if payload_is_string {
                    panic!("reader exploded");
                }
                std::panic::panic_any(42_u8);
            }
        });
        CaptureReader {
            stream,
            buffer,
            progress,
            handle,
        }
    };
    let (timed_out, held_open) = capture_reader(ReaderStream::Stderr);

    let ([out, _], warnings) = join_captures(
        [panicking(ReaderStream::Stdout, true), timed_out],
        BUDGET,
        Instant::now(),
    );
    assert_eq!(out, "kept", "a panicked reader still yields its buffer");
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert_eq!(
        warnings[0],
        "Stream stdout reader thread panicked: reader exploded"
    );
    assert!(warnings[1].contains("agent's stderr"), "{}", warnings[1]);
    assert!(warnings[1].contains("pipe is still open"), "{}", warnings[1]);

    let (_, warnings) = join_captures(
        [
            panicking(ReaderStream::Stdout, false),
            finished_reader(ReaderStream::Stderr, ""),
        ],
        BUDGET,
        Instant::now(),
    );
    assert_eq!(
        warnings,
        ["Stream stdout reader thread panicked: the panic payload was not a string"]
    );
    drop(held_open);
}

/// Both streams are held open: the two joins together wait one pipe cap, not
/// one each, and a line arriving halfway does not restart the clock.
#[test]
fn both_capture_joins_share_one_deadline_and_a_new_line_does_not_reset_it() {
    let (stdout, out_held) = capture_reader(ReaderStream::Stdout);
    let (stderr, err_held) = capture_reader(ReaderStream::Stderr);
    let since = Instant::now();
    let late_line = out_held.clone();
    std::thread::spawn(move || {
        std::thread::sleep(BUDGET.pipe_cap / 2);
        let _ = late_line.send(b"late line
".to_vec());
    });

    let ([out, _], warnings) = join_captures([stdout, stderr], BUDGET, since);

    let waited = since.elapsed();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(waited >= BUDGET.pipe_cap, "gave up early: {waited:?}");
    assert!(
        waited < BUDGET.pipe_cap * 2,
        "two joins waited more than one limit: {waited:?}"
    );
    assert_eq!(out, "late line");
    drop((out_held, err_held));
}

/// A reader that finished is joined even when the clock ran out long ago.
#[test]
fn a_finished_capture_reader_is_joined_even_past_the_deadline() {
    let reader = finished_reader(ReaderStream::Stdout, "done\n");
    let long_ago = Instant::now()
        .checked_sub(Duration::from_secs(3600))
        .unwrap_or_else(Instant::now);

    let ([out, _], warnings) = join_captures(
        [reader, finished_reader(ReaderStream::Stderr, "")],
        BUDGET,
        long_ago,
    );

    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(out.starts_with("done"), "{out:?}");
}

/// VC-6.4: the per-run volume cap bounds the capture buffer and sends a
/// `RunawayVolume` trip once the running totals breach the threshold.
/// Feeds far more lines than the cap allows and asserts (a) a single trip
/// is sent, (b) the returned buffer stays bounded near the cap rather than
/// growing without limit.
#[test]
fn capture_volume_cap_trips_and_bounds_buffer() {
    use std::io::Cursor;

    // Cap at 50 lines; bytes effectively unbounded so the line cap fires.
    let cap = claudine::runaway::CaptureVolumeCap::new(true, 50, u64::MAX);
    let (tx, rx) = std::sync::mpsc::channel::<EarlyTermination>();

    // 10_000 distinct lines — far past the 50-line cap.
    let mut input = String::new();
    for i in 0..10_000u32 {
        input.push_str(&format!("line {i}\n"));
    }
    let captured = capture_all(Cursor::new(input.into_bytes()), Some(&cap), &tx);

    // Exactly one trip, and it is a volume trip.
    match rx.try_recv() {
        Ok(EarlyTermination::RunawayVolume { lines, .. }) => {
            assert!(lines > 50, "trip must carry the breaching line count: {lines}");
        }
        other => panic!("expected one RunawayVolume trip, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "the cap must send exactly once");

    // The buffer is frozen at the cap — only the first ~50 lines, never
    // the full 10_000-line flood.
    let buffered_lines = captured.lines().count();
    assert!(
        buffered_lines <= 51,
        "buffer must stay bounded near the cap; got {buffered_lines} lines"
    );
}

/// A disabled (or absent) cap never trips and captures everything.
#[test]
fn capture_volume_cap_disabled_captures_all() {
    use std::io::Cursor;

    let (tx, rx) = std::sync::mpsc::channel::<EarlyTermination>();
    let input = "a\nb\nc\n".to_string();
    let captured = capture_all(Cursor::new(input.into_bytes()), None, &tx);
    assert!(rx.try_recv().is_err(), "no cap means no trip");
    assert_eq!(captured, "a\nb\nc");
}

/// `run_child_capture` shares the same spawn path as `run_child` and
/// must also stamp `agent_pid`. This is the path used by legacy
/// composition runs and harness-orchestration capture fallbacks.
#[test]
fn run_child_capture_captures_agent_pid_after_successful_spawn() {
    let env = minimal_env();
    let cwd = test_cwd();
    let (binary, args) = test_shell_command("echo ok", "echo ok");
    let mut child_spawned = false;

    let result = run_child_capture(
        &binary,
        &args,
        &env,
        &cwd,
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned,
        None,
        None,
    )
    .expect("spawning the platform shell fixture must succeed");

    assert!(child_spawned);
    let pid = result
        .agent_pid
        .expect("agent_pid must be Some after a successful spawn");
    assert!(pid > 0);
}

/// A failed spawn must return `Err` and leave the `child_spawned`
/// flag untouched. Crucially, no `ProcessResult` is constructed on
/// the failure path, so the caller cannot observe a fabricated
/// `agent_pid` value.
#[test]
fn run_child_failed_spawn_returns_err_without_agent_pid() {
    let env = minimal_env();
    let cwd = test_cwd();
    let mut child_spawned = false;

    let result = run_child_capture(
        Path::new("/nonexistent/binary/that/does/not/exist"),
        &[],
        &env,
        &cwd,
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned,
        None,
        None,
    );

    assert!(
        result.is_err(),
        "spawning a nonexistent binary must return Err"
    );
    assert!(
        !child_spawned,
        "child_spawned flag must stay false when spawn fails"
    );
}

/// End-to-end proof that `CLAUDINE_PID`, injected by the env-plan
/// builder in `wrap/env.rs`, actually reaches the spawned child's
/// environment. Spawns the platform's environment-listing command and
/// inspects captured stdout for the `CLAUDINE_PID=<claudine_pid>` line.
#[test]
fn run_child_capture_propagates_claudine_pid_to_child_environment() {
    let mut env = minimal_env();
    let claudine_pid = std::process::id();
    env.insert(
        OsString::from("CLAUDINE_PID"),
        OsString::from(claudine_pid.to_string()),
    );
    let cwd = test_cwd();
    let (binary, args) = test_shell_command("env", "set");
    let mut child_spawned = false;

    let result = run_child_capture(
        &binary,
        &args,
        &env,
        &cwd,
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned,
        None,
        None,
    )
    .expect("spawning the platform environment fixture must succeed");

    assert!(child_spawned);
    let expected_line = format!("CLAUDINE_PID={claudine_pid}");
    assert!(
        result.data.stdout.contains(&expected_line),
        "child stdout must include {expected_line:?}; got: {:?}",
        result.data.stdout,
    );
    let pid = result
        .agent_pid
        .expect("agent_pid must be Some after a successful spawn");
    assert!(pid > 0);
}

/// Each spawn returns a fresh `ProcessResult` with its own
/// `agent_pid`. This is the per-attempt reset guarantee that
/// harness retries and composition iterations depend on —
/// no stale PID can leak from a previous attempt into the next.
#[test]
fn consecutive_spawns_produce_distinct_agent_pids() {
    let env = minimal_env();
    let cwd = test_cwd();
    let (binary_a, args_a) = test_shell_command("echo first", "echo first");

    let mut child_spawned_a = false;
    let result_a = run_child_capture(
        &binary_a,
        &args_a,
        &env,
        &cwd,
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned_a,
        None,
        None,
    )
    .expect("first spawn must succeed");

    let mut child_spawned_b = false;
    let (binary_b, args_b) = test_shell_command("echo second", "echo second");
    let result_b = run_child_capture(
        &binary_b,
        &args_b,
        &env,
        &cwd,
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned_b,
        None,
        None,
    )
    .expect("second spawn must succeed");

    let pid_a = result_a
        .agent_pid
        .expect("first spawn must capture agent_pid");
    let pid_b = result_b
        .agent_pid
        .expect("second spawn must capture agent_pid");
    assert!(
        pid_a != pid_b,
        "consecutive spawns must produce distinct PIDs \
         (got pid_a={pid_a}, pid_b={pid_b}); \
         if they collide the per-attempt reset contract is broken"
    );
}

/// VC-5.2 / tasks 5.1+5.2: a configured wall-clock `timeout` now routes
/// through the unified signal-aware wait loop (via the dedicated
/// wall-clock ticker) on the capture path. A child that would otherwise
/// sleep far past the budget must be terminated promptly and reported as
/// `TimedOut` — proving the path no longer depends on the retired
/// `wait_with_timeout`.
#[cfg(unix)]
#[test]
fn run_child_capture_wall_clock_timeout_reaps_child() {
    use std::time::{Duration, Instant};

    let env = minimal_env();
    let cwd = Path::new("/tmp");
    let mut child_spawned = false;

    let start = Instant::now();
    let result = run_child_capture(
        sleep_binary(),
        &["30".to_string()],
        &env,
        cwd,
        Some(1),
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned,
        None,
        None,
    )
    .expect("spawning sleep must succeed on the test host");
    let elapsed = start.elapsed();

    assert!(child_spawned);
    assert_eq!(
        result.termination,
        claudine::harness::ProcessTermination::TimedOut,
        "a breached wall-clock timeout must report TimedOut"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "the wall-clock timeout must kill the child well before its own \
         30s sleep elapses; took {elapsed:?}"
    );
}
