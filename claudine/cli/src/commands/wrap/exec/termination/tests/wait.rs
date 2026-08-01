//! Platform wait-loop escalation, completion, and watchdog behavior.

use super::*;

#[cfg(windows)]
fn windows_ping() -> std::path::PathBuf {
    std::env::var_os("SYSTEMROOT")
        .map(std::path::PathBuf::from)
        .expect("Windows test host must provide SYSTEMROOT")
        .join("System32")
        .join("PING.EXE")
}

/// Regression: a disconnected watchdog channel must log a warning rather
/// than silently disabling timeout enforcement. The wait loop should still
/// return normally once the child exits.
#[cfg(unix)]
#[test]
#[tracing_test::traced_test]
fn disconnected_watchdog_channel_warns_and_returns_on_child_exit() {
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::mpsc::channel;

    let mut child = Command::new("sleep")
        .arg("0.5")
        .process_group(0)
        .spawn()
        .expect("sleep must be available on PATH");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (watchdog_tx, watchdog_rx) = channel::<WatchdogTermination>();
    // Drop the sender to disconnect the channel before the loop polls it.
    drop(watchdog_tx);

    let result = wait_with_signal_and_early_termination(
        &mut child,
        true,
        early_rx,
        Some(watchdog_rx),
        Duration::from_secs(1),
        true,
    );

    let (code, termination, _) = result.expect("wait loop must return when child exits");
    assert_eq!(code, 0, "sleep should exit 0; got {code}");
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
    assert!(
        logs_contain("watchdog ticker channel disconnected"),
        "expected warning log for disconnected watchdog channel"
    );
}

/// The SIGTERM/SIGKILL escalation path should still reap a normally
/// exiting child after an early-termination signal. This exercises the
/// loop-driven kill path and its PID-recycle guard (the guard re-checks
/// try_wait immediately before each signal).
#[cfg(unix)]
#[test]
fn early_termination_signal_reaps_child_and_reports_timed_out() {
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::mpsc::channel;

    let mut child = Command::new("sleep")
        .arg("10")
        .process_group(0)
        .spawn()
        .expect("sleep must be available on PATH");
    let (early_tx, early_rx) = channel::<EarlyTermination>();
    let (_watchdog_tx, watchdog_rx) = channel::<WatchdogTermination>();

    // Send the early-termination signal from another thread so the wait
    // loop has time to start polling.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = early_tx.send(EarlyTermination::Timeout {
            message: "test timeout".into(),
        });
    });

    let result = wait_with_signal_and_early_termination(
        &mut child,
        true,
        early_rx,
        Some(watchdog_rx),
        Duration::from_millis(100),
        true,
    );

    let (code, termination, early) = result.expect("wait loop must return");
    assert!(
        matches!(early, Some(EarlyTermination::Timeout { ref message }) if message == "test timeout"),
        "early termination should be carried through; got {early:?}"
    );
    assert_eq!(termination, claudine::harness::ProcessTermination::TimedOut);
    // Killed by SIGTERM or SIGKILL; either way the exit code is non-zero.
    assert!(code != 0, "child should have been terminated; got {code}");
}

/// Kimi wire mode uses completion termination after receiving the final
/// prompt response: the child tree must still be terminated through the
/// shared signal-aware path, but the wrapper outcome remains Completed.
#[cfg(unix)]
#[test]
fn completion_termination_reaps_child_and_reports_completed() {
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::mpsc::channel;

    let mut child = Command::new("sleep")
        .arg("10")
        .process_group(0)
        .spawn()
        .expect("sleep must be available on PATH");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (completion_tx, completion_rx) = channel::<CompletionTermination>();

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = completion_tx.send(CompletionTermination);
    });

    let result = wait_with_signal_early_termination_and_completion(
        &mut child,
        true,
        early_rx,
        None,
        Some(completion_rx),
        Duration::from_millis(100),
        false,
    );

    let (code, termination, early) = result.expect("wait loop must return");
    assert_eq!(code, 0);
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
    assert!(early.is_none(), "completion is not an error path: {early:?}");
}

/// VC-5.4: the non-interactive ladder is SIGTERM-first (F5). A single
/// counted press on a non-interactive run must escalate straight to
/// SIGTERM (no human is mid-session to react to a graceful SIGINT), while
/// an interactive run keeps the full `SIGINT → SIGTERM → SIGKILL` ladder.
#[cfg(unix)]
#[test]
fn escalation_signal_compresses_ladder_when_non_interactive() {
    use super::super::unix::escalation_signal;

    // Interactive: three presses to force-kill.
    assert_eq!(escalation_signal(true, 1), libc::SIGINT);
    assert_eq!(escalation_signal(true, 2), libc::SIGTERM);
    assert_eq!(escalation_signal(true, 3), libc::SIGKILL);

    // Non-interactive: SIGTERM on the first press, SIGKILL on the next.
    assert_eq!(escalation_signal(false, 1), libc::SIGTERM);
    assert_eq!(escalation_signal(false, 2), libc::SIGKILL);
    assert_eq!(escalation_signal(false, 3), libc::SIGKILL);
}

/// Smoke-test for the Windows parity path using an OS executable rather than
/// the `timeout` name, which may resolve to a Unix compatibility tool earlier
/// on `PATH`.
#[cfg(windows)]
#[test]
fn non_unix_wait_loop_returns_on_child_exit() {
    use std::process::{Command, Stdio};
    use std::sync::mpsc::channel;

    let mut child = Command::new(windows_ping())
        .args(["-n", "2", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Windows ping fixture must be available");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (_watchdog_tx, watchdog_rx) = channel::<WatchdogTermination>();

    let result = wait_with_signal_and_early_termination(
        &mut child,
        false,
        early_rx,
        Some(watchdog_rx),
        Duration::from_secs(1),
        true,
    );

    let (code, termination, _) = result.expect("wait loop must return when child exits");
    assert_eq!(code, 0);
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
}

/// Windows-specific coverage for Kimi's prompt-finished fallback: the
/// child is spawned in a new process group and completion termination
/// travels through the Job Object wait loop rather than `Child::kill`.
#[cfg(windows)]
#[test]
fn windows_completion_termination_uses_job_object_path() {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::mpsc::channel;

    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    let mut child = Command::new(windows_ping())
        .args(["-n", "31", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .expect("Windows ping fixture must be available");
    let (_early_tx, early_rx) = channel::<EarlyTermination>();
    let (completion_tx, completion_rx) = channel::<CompletionTermination>();

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = completion_tx.send(CompletionTermination);
    });

    let result = wait_with_signal_early_termination_and_completion(
        &mut child,
        true,
        early_rx,
        None,
        Some(completion_rx),
        Duration::from_millis(100),
        false,
    );

    let (code, termination, early) = result.expect("wait loop must return");
    assert_eq!(code, 0);
    assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
    assert!(early.is_none(), "completion is not an error path: {early:?}");
}
