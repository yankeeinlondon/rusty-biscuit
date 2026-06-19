use std::process::Child;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use claudine::stream::logs::EarlyTermination;
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use std::sync::mpsc::{Receiver, TryRecvError};

use super::exit::exit_code_from_status;

/// Maximum time to wait for a child to reap after SIGKILL before giving up.
///
/// A wedged (D-state) child may never reap; the wrapper must not hang
/// indefinitely. This cap is intentionally conservative: the kernel has
/// already been asked to destroy the process, and the caller's timeout
/// budget has long been exhausted.
const POST_SIGKILL_REAP_TIMEOUT: Duration = Duration::from_secs(10);

/// Reason for a watchdog-initiated termination.
///
/// The unified two-rule design has exactly two reasons. Stuck-subagent
/// detail is surfaced through [`WatchdogTermination::stuck_subagents`] for
/// the rendered error block, not through a distinct reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTerminationReason {
    /// Wall-clock budget (`timeout`) elapsed since the child was spawned.
    Timeout,
    /// Stream-silence budget (`step_timeout`) elapsed since the last parent
    /// stream event. Stuck subagents (if any) are carried in the
    /// [`WatchdogTermination`] for diagnostic enrichment.
    StepTimeout,
}

/// Request sent by the watchdog ticker to the exec wait loop asking for
/// child-process termination.
///
/// Carries the reason, a human-readable message, and optional snapshots
/// of stuck subagents so the summary can be enriched with details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WatchdogTermination {
    pub(crate) reason: WatchdogTerminationReason,
    pub(crate) message: String,
    pub(crate) stuck_subagents: Vec<super::subagent_watchdog::ActiveSubagentSnapshot>,
}

/// Polling wait loop used when an [`EarlyTermination`] receiver is attached
/// to the structured stream executor.
///
/// Behaves like [`wait_with_signal_handling`] while also polling the
/// stderr-bridge channel and the watchdog ticker channel. When a signal
/// arrives, the child's process group is sent `SIGTERM` and escalated to
/// `SIGKILL` after a grace period. User Ctrl-C still reports `Interrupted`;
/// wrapper-driven early termination (rate-limit recovery, timeout, or
/// step_timeout) preserves a normal `Completed` termination so downstream
/// failure handling can inspect synthesized summary fields instead of
/// treating the run like a user cancel.
///
/// Timeout enforcement is delegated to the watchdog ticker
/// (`spawn_timeout_watchdog_ticker`), which sends `WatchdogTermination`
/// requests through the `watchdog_rx` channel. This function only consumes
/// those signals and escalates them to child-process termination.
///
/// Isolated to the bridge path so non-OpenCode runs keep the existing
/// `child.wait()`-based helper.
#[cfg(unix)]
pub(crate) fn wait_with_signal_and_early_termination(
    child: &mut Child,
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    watchdog_rx: Option<Receiver<WatchdogTermination>>,
    kill_grace: Duration,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    use std::sync::atomic::{AtomicBool, AtomicU8};

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_exited = Arc::new(AtomicBool::new(false));
    let child_pid = child.id();

    let counter = Arc::clone(&interrupt_count);
    let exited = Arc::clone(&child_exited);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            // Don't signal a PID we no longer own (it may have been
            // recycled onto an unrelated process).
            if exited.load(Ordering::SeqCst) {
                return;
            }
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if !child_in_own_pgroup {
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

    let mut early_termination: Option<EarlyTermination> = None;
    let mut grace_deadline: Option<Instant> = None;
    let mut reap_deadline: Option<Instant> = None;
    let mut watchdog_rx = watchdog_rx;
    let poll_interval = Duration::from_millis(75);
    let grace_period = kill_grace;

    loop {
        if let Some(status) = child.try_wait()? {
            // Mark the PID as reaped before the grace window's signal guard
            // can drop so we never signal a recycled PID.
            child_exited.store(true, Ordering::SeqCst);
            let code = exit_code_from_status(status);
            let was_interrupted = interrupt_count.load(Ordering::SeqCst) > 0;
            let termination = if was_interrupted {
                claudine::harness::ProcessTermination::Interrupted
            } else if early_termination.is_some() {
                early_termination_process_outcome(early_termination.as_ref())
            } else {
                claudine::harness::ProcessTermination::Completed
            };
            return Ok((code, termination, early_termination));
        }

        if let Some(deadline) = reap_deadline
            && Instant::now() >= deadline
        {
            child_exited.store(true, Ordering::SeqCst);
            tracing::error!(
                child_pid,
                "child did not reap after SIGKILL; giving up to avoid hanging the wrapper"
            );
            return Ok((
                137,
                claudine::harness::ProcessTermination::TimedOut,
                early_termination,
            ));
        }

        if early_termination.is_none() {
            match early_rx.try_recv() {
                Ok(signal) => {
                    // Re-check ownership immediately before signaling to
                    // close the PID-recycle window between the loop-top
                    // try_wait and the kill.
                    if child.try_wait()?.is_none() {
                        tracing::info!(
                            child_pid,
                            "early-termination signal received; sending SIGTERM to child process group",
                        );
                        send_signal_to_child(child_pid, child_in_own_pgroup, libc::SIGTERM);
                        early_termination = Some(signal);
                        grace_deadline = Some(Instant::now() + grace_period);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!(
                        "early-termination channel disconnected; early-exit signals will no longer be processed"
                    );
                }
            }
        }

        // Watchdog-initiated termination (unified `timeout` / `step_timeout`).
        if early_termination.is_none()
            && let Some(ref wd_rx) = watchdog_rx
        {
            match wd_rx.try_recv() {
                Ok(req) => {
                    // Re-check ownership immediately before signaling to
                    // close the PID-recycle window.
                    if child.try_wait()?.is_none() {
                        tracing::warn!(
                            child_pid,
                            reason = ?req.reason,
                            "watchdog termination received; sending SIGTERM to child process group",
                        );
                        send_signal_to_child(child_pid, child_in_own_pgroup, libc::SIGTERM);
                        early_termination = Some(watchdog_request_to_early_termination(req));
                        grace_deadline = Some(Instant::now() + grace_period);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!(
                        "watchdog ticker channel disconnected; timeout enforcement disabled for remainder of run"
                    );
                    // Stop polling the disconnected channel.
                    watchdog_rx = None;
                }
            }
        }

        if let Some(deadline) = grace_deadline
            && Instant::now() >= deadline
        {
            // Re-check ownership before the unconditional grace SIGKILL,
            // which is the most exposed PID-recycle site.
            if child.try_wait()?.is_none() {
                tracing::warn!(
                    child_pid,
                    "child did not exit after early-termination SIGTERM; escalating to SIGKILL",
                );
                send_signal_to_child(child_pid, child_in_own_pgroup, libc::SIGKILL);
                reap_deadline = Some(Instant::now() + POST_SIGKILL_REAP_TIMEOUT);
            }
            grace_deadline = None;
        }

        std::thread::sleep(poll_interval);
    }
}

/// Send a signal to the child, choosing the process-group form when safe.
///
/// When `child_in_own_pgroup` is `true` the child leads its own process
/// group, so `-pid` reaches descendants. When it is `false` the child
/// shares the parent's process group (interactive TUI mode); a negative
/// PID would hit Claudine and the terminal itself, so only the immediate
/// child is targeted. The caller must re-check `child.try_wait()`
/// immediately before this call to avoid signaling a recycled PID.
#[cfg(unix)]
fn send_signal_to_child(child_pid: u32, child_in_own_pgroup: bool, signal: i32) {
    let kill_pid = if child_in_own_pgroup {
        -(child_pid as i32)
    } else {
        child_pid as i32
    };
    unsafe {
        libc::kill(kill_pid, signal);
    }
}

#[cfg(not(unix))]
pub(crate) fn wait_with_signal_and_early_termination(
    child: &mut Child,
    _child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    mut watchdog_rx: Option<Receiver<WatchdogTermination>>,
    kill_grace: Duration,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    let mut early_termination: Option<EarlyTermination> = None;
    let mut grace_deadline: Option<Instant> = None;
    let mut reap_deadline: Option<Instant> = None;
    let poll_interval = Duration::from_millis(75);
    let grace_period = kill_grace;

    loop {
        if let Some(status) = child.try_wait()? {
            let code = exit_code_from_status(status);
            let termination = if early_termination.is_some() {
                early_termination_process_outcome(early_termination.as_ref())
            } else {
                claudine::harness::ProcessTermination::Completed
            };
            return Ok((code, termination, early_termination));
        }

        if let Some(deadline) = reap_deadline
            && Instant::now() >= deadline
        {
            tracing::error!(
                child_pid = child.id(),
                "child did not reap after kill; giving up to avoid hanging the wrapper"
            );
            return Ok((
                1,
                claudine::harness::ProcessTermination::TimedOut,
                early_termination,
            ));
        }

        if early_termination.is_none() {
            match early_rx.try_recv() {
                Ok(signal) => {
                    if child.try_wait()?.is_none() {
                        let _ = child.kill();
                        early_termination = Some(signal);
                        grace_deadline = Some(Instant::now() + grace_period);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!(
                        "early-termination channel disconnected; early-exit signals will no longer be processed"
                    );
                }
            }
        }

        if early_termination.is_none()
            && let Some(ref wd_rx) = watchdog_rx
        {
            match wd_rx.try_recv() {
                Ok(req) => {
                    if child.try_wait()?.is_none() {
                        let _ = child.kill();
                        early_termination = Some(watchdog_request_to_early_termination(req));
                        grace_deadline = Some(Instant::now() + grace_period);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!(
                        "watchdog ticker channel disconnected; timeout enforcement disabled for remainder of run"
                    );
                    watchdog_rx = None;
                }
            }
        }

        if let Some(deadline) = grace_deadline
            && Instant::now() >= deadline
        {
            if child.try_wait()?.is_none() {
                let _ = child.kill();
                reap_deadline = Some(Instant::now() + POST_SIGKILL_REAP_TIMEOUT);
            }
            grace_deadline = None;
        }

        std::thread::sleep(poll_interval);
    }
}

/// Overwrite the synthesized summary fields when the stderr bridge or the
/// OpenCode wait loop signals an early-exit condition.
///
/// Preserves any parser-provided `rate_limit` fields field-by-field:
/// `is_throttled` is forced to `Some(true)`, `message` is only set when
/// absent, and `reset_at` keeps the first non-`None` value.
pub(crate) fn apply_early_termination_to_summary(
    summary: &mut StreamExecutionSummary,
    termination: &EarlyTermination,
) {
    match termination {
        EarlyTermination::RateLimit { message, reset_at } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("usage_limit_reached".into());
            summary.error_message = Some(message.clone());

            let mut rate_limit = summary.rate_limit.clone().unwrap_or_default();
            rate_limit.is_throttled = Some(true);
            if rate_limit.message.is_none() {
                rate_limit.message = Some(message.clone());
            }
            if rate_limit.reset_at.is_none()
                && let Some(reset) = reset_at
            {
                rate_limit.reset_at = Some(*reset);
            }
            summary.rate_limit = Some(rate_limit);
        }
        EarlyTermination::Timeout { message } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("timeout".into());
            summary.error_message = Some(message.clone());
        }
        EarlyTermination::StepTimeout { message, .. } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("step_timeout".into());
            summary.error_message = Some(message.clone());
        }
    }
}

/// Convert a [`WatchdogTermination`] request from the watchdog ticker into
/// the lib-side [`EarlyTermination`] variant the wait loop carries until
/// the synthesized summary is built.
///
/// `Timeout` and `StepTimeout` are the only two reasons; `StepTimeout`
/// preserves the stuck-subagent snapshots as `outstanding` so the
/// rendered error block can name any subagents still in flight.
pub(crate) fn watchdog_request_to_early_termination(req: WatchdogTermination) -> EarlyTermination {
    match req.reason {
        WatchdogTerminationReason::Timeout => EarlyTermination::Timeout {
            message: req.message,
        },
        WatchdogTerminationReason::StepTimeout => EarlyTermination::StepTimeout {
            message: req.message,
            outstanding: req
                .stuck_subagents
                .iter()
                .map(|snap| snap.to_stuck_info())
                .collect(),
        },
    }
}

pub(crate) fn early_termination_process_outcome(
    early_termination: Option<&EarlyTermination>,
) -> claudine::harness::ProcessTermination {
    match early_termination {
        Some(EarlyTermination::Timeout { .. }) => claudine::harness::ProcessTermination::TimedOut,
        Some(EarlyTermination::StepTimeout { .. }) => {
            claudine::harness::ProcessTermination::TimedOut
        }
        Some(EarlyTermination::RateLimit { .. }) => {
            claudine::harness::ProcessTermination::Completed
        }
        None => claudine::harness::ProcessTermination::Completed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::stream::summary::StreamExecutionSummary;

    #[test]
    fn apply_early_termination_rate_limit_sets_usage_limit_summary_fields() {
        use chrono::TimeZone;
        let reset_at = chrono::Utc
            .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
            .unwrap();
        let mut summary = StreamExecutionSummary {
            exit_code: 143,
            is_error: false,
            ..Default::default()
        };
        let termination = EarlyTermination::RateLimit {
            message: "Usage limit reached; resets at 2026-04-16 04:18:56 UTC".into(),
            reset_at: Some(reset_at),
        };

        apply_early_termination_to_summary(&mut summary, &termination);

        assert_eq!(summary.exit_code, 1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("usage_limit_reached"));
        assert!(
            summary
                .error_message
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains("usage limit"),
        );
        let rl = summary.rate_limit.as_ref().expect("rate_limit populated");
        assert_eq!(rl.is_throttled, Some(true));
        assert_eq!(rl.reset_at, Some(reset_at));
        assert!(
            rl.message
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains("usage limit")
        );
    }

    #[test]
    fn apply_early_termination_preserves_existing_rate_limit_fields() {
        use chrono::TimeZone;
        let existing_reset = chrono::Utc.with_ymd_and_hms(2026, 4, 16, 2, 0, 0).unwrap();
        let mut summary = StreamExecutionSummary {
            rate_limit: Some(claudine::stream::summary::RateLimitInfo {
                is_throttled: Some(false),
                retry_after_ms: Some(5000),
                message: Some("pre-existing".into()),
                reset_at: Some(existing_reset),
            }),
            ..Default::default()
        };
        let incoming_reset = chrono::Utc
            .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
            .unwrap();
        let termination = EarlyTermination::RateLimit {
            message: "Usage limit reached".into(),
            reset_at: Some(incoming_reset),
        };

        apply_early_termination_to_summary(&mut summary, &termination);

        let rl = summary.rate_limit.as_ref().unwrap();
        // is_throttled is forced to true even when existing said false.
        assert_eq!(rl.is_throttled, Some(true));
        // Existing message is preserved.
        assert_eq!(rl.message.as_deref(), Some("pre-existing"));
        // Existing reset_at is preserved (do not clobber parser-provided state).
        assert_eq!(rl.reset_at, Some(existing_reset));
        // retry_after_ms is untouched.
        assert_eq!(rl.retry_after_ms, Some(5000));
    }

    #[test]
    fn early_termination_process_outcome_maps_step_timeout_to_timed_out() {
        let termination = EarlyTermination::StepTimeout {
            message: "no stream activity for 6s; terminating due to step_timeout".into(),
            outstanding: Vec::new(),
        };

        let outcome = early_termination_process_outcome(Some(&termination));

        assert_eq!(outcome, claudine::harness::ProcessTermination::TimedOut);
    }

    #[test]
    fn apply_early_termination_step_timeout_sets_step_timeout_error() {
        let mut summary = StreamExecutionSummary::default();

        apply_early_termination_to_summary(
            &mut summary,
            &EarlyTermination::StepTimeout {
                message: "no stream activity for 6s; terminating due to step_timeout".into(),
                outstanding: Vec::new(),
            },
        );

        assert_eq!(summary.exit_code, 1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("step_timeout"));
        assert!(
            summary
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("no stream activity"),
        );
    }

    #[test]
    fn early_termination_process_outcome_maps_timeout_to_timed_out() {
        let termination = EarlyTermination::Timeout {
            message: "wall-clock budget exceeded after 2h".into(),
        };

        let outcome = early_termination_process_outcome(Some(&termination));

        assert_eq!(outcome, claudine::harness::ProcessTermination::TimedOut);
    }

    #[test]
    fn apply_early_termination_timeout_sets_timeout_error() {
        let mut summary = StreamExecutionSummary::default();

        apply_early_termination_to_summary(
            &mut summary,
            &EarlyTermination::Timeout {
                message: "wall-clock budget exceeded after 2h".into(),
            },
        );

        assert_eq!(summary.exit_code, 1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("timeout"));
        assert_eq!(
            summary.error_message.as_deref(),
            Some("wall-clock budget exceeded after 2h"),
        );
    }

    #[test]
    fn watchdog_request_to_early_termination_maps_timeout_reason() {
        use super::{WatchdogTermination, WatchdogTerminationReason};
        use crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot;

        let req = WatchdogTermination {
            reason: WatchdogTerminationReason::Timeout,
            message: "wall-clock budget exceeded".into(),
            stuck_subagents: Vec::new(),
        };

        let early = watchdog_request_to_early_termination(req);
        assert!(matches!(
            early,
            EarlyTermination::Timeout { ref message } if message == "wall-clock budget exceeded"
        ));
    }

    #[test]
    fn watchdog_request_to_early_termination_carries_stuck_subagents() {
        use super::{WatchdogTermination, WatchdogTerminationReason};
        use crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot;

        let now = Instant::now();
        let snapshot = ActiveSubagentSnapshot {
            id: "ses_a".into(),
            name: Some("Commit feature work".into()),
            started_at: now,
            last_progress_at: now,
            elapsed_since_start: Duration::from_secs(900),
            elapsed_since_progress: Duration::from_secs(900),
        };
        let req = WatchdogTermination {
            reason: WatchdogTerminationReason::StepTimeout,
            message: "no stream activity for 30m".into(),
            stuck_subagents: vec![snapshot],
        };

        let early = watchdog_request_to_early_termination(req);
        let outstanding = match early {
            EarlyTermination::StepTimeout { outstanding, .. } => outstanding,
            other => panic!("expected StepTimeout, got {other:?}"),
        };
        assert_eq!(outstanding.len(), 1);
        assert_eq!(outstanding[0].id, "ses_a");
        assert_eq!(outstanding[0].name.as_deref(), Some("Commit feature work"));
        assert_eq!(
            outstanding[0].elapsed_since_progress,
            Duration::from_secs(900)
        );
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

    /// Smoke-test for the non-Unix parity path. Only runs on Windows, but the
    /// branch is compile-checked on every target.
    #[cfg(not(unix))]
    #[test]
    fn non_unix_wait_loop_returns_on_child_exit() {
        use std::process::Command;
        use std::sync::mpsc::channel;

        let mut child = Command::new("cmd")
            .args(["/C", "timeout /T 1 /nobreak >nul"])
            .spawn()
            .expect("cmd must be available");
        let (_early_tx, early_rx) = channel::<EarlyTermination>();
        let (_watchdog_tx, watchdog_rx) = channel::<WatchdogTermination>();

        let result = wait_with_signal_and_early_termination(
            &mut child,
            false,
            early_rx,
            Some(watchdog_rx),
            Duration::from_secs(1),
        );

        let (code, termination, _) = result.expect("wait loop must return when child exits");
        assert_eq!(code, 0);
        assert_eq!(termination, claudine::harness::ProcessTermination::Completed);
    }
}
