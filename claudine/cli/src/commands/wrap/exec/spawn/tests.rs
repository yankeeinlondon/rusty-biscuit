use super::*;

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

/// Minimal env that satisfies the `PATH` / `HOME` debug-asserts inside
/// every spawn function. Test-owned so we never depend on the host
/// shell's environment.
fn minimal_env() -> HashMap<OsString, OsString> {
    let mut env = HashMap::new();
    env.insert(OsString::from("PATH"), OsString::from("/usr/bin:/bin"));
    env.insert(OsString::from("HOME"), OsString::from("/tmp"));
    env
}

/// Find a working `/bin/true`-equivalent on the test host. macOS ships
/// `/usr/bin/true`; Linux distros typically have both `/bin/true` and
/// `/usr/bin/true`. We prefer `/usr/bin/true` (always present on macOS)
/// and fall back to `/bin/true`.
fn true_binary() -> &'static Path {
    if Path::new("/usr/bin/true").exists() {
        Path::new("/usr/bin/true")
    } else {
        Path::new("/bin/true")
    }
}

/// Spawn must populate `ProcessResult.agent_pid` immediately after
/// `command.spawn()?` returns. The captured PID must match a real
/// positive integer. This test exercises the legacy/interactive spawn
/// path used by direct wrappers and legacy composition runs.
#[test]
fn run_child_captures_agent_pid_after_successful_spawn() {
    let env = minimal_env();
    let cwd = Path::new("/tmp");
    let mut child_spawned = false;

    let result = run_child(
        true_binary(),
        &[],
        &env,
        cwd,
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned,
    )
    .expect("spawning /usr/bin/true must succeed on the test host");

    assert!(child_spawned, "child_spawned flag must flip on success");
    let pid = result
        .agent_pid
        .expect("agent_pid must be Some after a successful spawn");
    assert!(pid > 0, "spawned child PID must be a positive integer");
}

/// `run_child_capture` shares the same spawn path as `run_child` and
/// must also stamp `agent_pid`. This is the path used by legacy
/// composition runs and harness-orchestration capture fallbacks.
#[test]
fn run_child_capture_captures_agent_pid_after_successful_spawn() {
    let env = minimal_env();
    let cwd = Path::new("/tmp");
    let mut child_spawned = false;

    let result = run_child_capture(
        Path::new("/bin/echo"),
        &["ok".to_string()],
        &env,
        cwd,
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
    .expect("spawning /bin/echo must succeed on the test host");

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
    let cwd = Path::new("/tmp");
    let mut child_spawned = false;

    let result = run_child_capture(
        Path::new("/nonexistent/binary/that/does/not/exist"),
        &[],
        &env,
        cwd,
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
/// environment. Spawns `/usr/bin/env` and inspects the captured
/// stdout for the `CLAUDINE_PID=<claudine_pid>` line.
#[test]
fn run_child_capture_propagates_claudine_pid_to_child_environment() {
    let mut env = minimal_env();
    let claudine_pid = std::process::id();
    env.insert(
        OsString::from("CLAUDINE_PID"),
        OsString::from(claudine_pid.to_string()),
    );
    let cwd = Path::new("/tmp");
    let mut child_spawned = false;

    let result = run_child_capture(
        Path::new("/usr/bin/env"),
        &[],
        &env,
        cwd,
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
    .expect("spawning /usr/bin/env must succeed on the test host");

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
    let cwd = Path::new("/tmp");

    let mut child_spawned_a = false;
    let result_a = run_child_capture(
        Path::new("/bin/echo"),
        &["first".to_string()],
        &env,
        cwd,
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
    let result_b = run_child_capture(
        Path::new("/bin/echo"),
        &["second".to_string()],
        &env,
        cwd,
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

/// Locate a `sleep`-equivalent for the wall-clock-timeout tests. macOS and
/// Linux both ship `/bin/sleep`; some Linux distros only have
/// `/usr/bin/sleep`.
#[cfg(unix)]
fn sleep_binary() -> &'static Path {
    if Path::new("/bin/sleep").exists() {
        Path::new("/bin/sleep")
    } else {
        Path::new("/usr/bin/sleep")
    }
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

/// VC-5.2 / tasks 5.1+5.2: the same wall-clock routing for the direct
/// (`run_child`) path with inherited stdio. Proves both non-streaming
/// spawn paths share the one signal-aware wait loop.
#[cfg(unix)]
#[test]
fn run_child_wall_clock_timeout_reaps_child() {
    let env = minimal_env();
    let cwd = Path::new("/tmp");
    let mut child_spawned = false;

    let start = Instant::now();
    let result = run_child(
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
        "the wall-clock timeout must kill the child promptly; took {elapsed:?}"
    );
}
