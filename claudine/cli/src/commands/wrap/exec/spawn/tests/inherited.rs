//! Inherited-output spawn mode: PID capture and wall-clock timeout.

use super::*;

/// Spawn must populate `ProcessResult.agent_pid` immediately after
/// `command.spawn()?` returns. The captured PID must match a real positive
/// integer. A platform shell exits successfully so the fixture does not
/// assume a Unix executable layout. This test exercises the
/// legacy/interactive spawn path used by direct wrappers and legacy
/// composition runs.
#[test]
fn run_child_captures_agent_pid_after_successful_spawn() {
    let env = minimal_env();
    let cwd = test_cwd();
    let (binary, args) = test_shell_command("exit 0", "exit 0");
    let mut child_spawned = false;

    let result = run_child(
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
    )
    .expect("spawning the platform shell fixture must succeed");

    assert!(child_spawned, "child_spawned flag must flip on success");
    let pid = result
        .agent_pid
        .expect("agent_pid must be Some after a successful spawn");
    assert!(pid > 0, "spawned child PID must be a positive integer");
}

/// VC-5.2 / tasks 5.1+5.2: the same wall-clock routing for the direct
/// (`run_child`) path with inherited stdio. Proves both non-streaming
/// spawn paths share the one signal-aware wait loop.
#[cfg(unix)]
#[test]
fn run_child_wall_clock_timeout_reaps_child() {
    use std::time::{Duration, Instant};

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
