//! Captured-output spawn mode: PID/env capture, wall-clock timeout, and the
//! per-run volume cap.

use super::super::captured::capture_stream_with_volume_cap;
use super::*;
use claudine::stream::logs::EarlyTermination;
use std::sync::Arc;

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
    let first_at = Arc::new(std::sync::Mutex::new(None));
    let (tx, rx) = std::sync::mpsc::channel::<EarlyTermination>();

    // 10_000 distinct lines — far past the 50-line cap.
    let mut input = String::new();
    for i in 0..10_000u32 {
        input.push_str(&format!("line {i}\n"));
    }
    let captured = capture_stream_with_volume_cap(
        Cursor::new(input.into_bytes()),
        &[],
        &first_at,
        Some(&cap),
        &tx,
    );

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

    let first_at = Arc::new(std::sync::Mutex::new(None));
    let (tx, rx) = std::sync::mpsc::channel::<EarlyTermination>();
    let input = "a\nb\nc\n".to_string();
    let captured =
        capture_stream_with_volume_cap(Cursor::new(input.into_bytes()), &[], &first_at, None, &tx);
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
