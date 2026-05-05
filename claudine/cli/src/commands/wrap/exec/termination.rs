use std::process::Child;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use claudine::stream::logs::EarlyTermination;
use claudine::stream::progress::LiveMetrics;
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use std::sync::mpsc::{Receiver, TryRecvError};

use super::exit::exit_code_from_status;

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
#[allow(clippy::too_many_arguments)]
pub(crate) fn wait_with_signal_and_early_termination(
    child: &mut Child,
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    watchdog_rx: Option<Receiver<WatchdogTermination>>,
    live_metrics: Option<LiveMetrics>,
    stop_threshold: Duration,
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

        if early_termination.is_none() {
            match early_rx.try_recv() {
                Ok(signal) => {
                    tracing::info!(
                        child_pid,
                        "early-termination signal received; sending SIGTERM to child process group",
                    );
                    let kill_pid = if child_in_own_pgroup {
                        -(child_pid as i32)
                    } else {
                        child_pid as i32
                    };
                    unsafe {
                        libc::kill(kill_pid, libc::SIGTERM);
                    }
                    early_termination = Some(signal);
                    grace_deadline = Some(Instant::now() + grace_period);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    // Channel closed with no signal; continue normal polling.
                }
            }
        }

        // Watchdog-initiated termination (unified `timeout` / `step_timeout`).
        if early_termination.is_none() && let Some(ref wd_rx) = watchdog_rx {
            match wd_rx.try_recv() {
                Ok(req) => {
                    tracing::warn!(
                        child_pid,
                        reason = ?req.reason,
                        "watchdog termination received; sending SIGTERM to child process group",
                    );
                    let kill_pid = if child_in_own_pgroup {
                        -(child_pid as i32)
                    } else {
                        child_pid as i32
                    };
                    unsafe {
                        libc::kill(kill_pid, libc::SIGTERM);
                    }
                    early_termination = Some(watchdog_request_to_early_termination(req));
                    grace_deadline = Some(Instant::now() + grace_period);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
            }
        }

        if early_termination.is_none()
            && let Some(metrics) = live_metrics.as_ref()
            && let Some(signal) =
                super::timeouts::detect_opencode_hang_termination(metrics, Instant::now(), stop_threshold)
        {
            let kill_pid = if child_in_own_pgroup {
                -(child_pid as i32)
            } else {
                child_pid as i32
            };
            unsafe {
                libc::kill(kill_pid, libc::SIGTERM);
            }
            early_termination = Some(signal);
            grace_deadline = Some(Instant::now() + grace_period);
        }

        if let Some(deadline) = grace_deadline
            && Instant::now() >= deadline
        {
            tracing::warn!(
                child_pid,
                "child did not exit after early-termination SIGTERM; escalating to SIGKILL",
            );
            let kill_pid = if child_in_own_pgroup {
                -(child_pid as i32)
            } else {
                child_pid as i32
            };
            unsafe {
                libc::kill(kill_pid, libc::SIGKILL);
            }
            grace_deadline = None;
        }

        std::thread::sleep(poll_interval);
    }
}

#[cfg(not(unix))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn wait_with_signal_and_early_termination(
    child: &mut Child,
    _child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    watchdog_rx: Option<Receiver<WatchdogTermination>>,
    live_metrics: Option<LiveMetrics>,
    stop_threshold: Duration,
    kill_grace: Duration,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    let mut early_termination: Option<EarlyTermination> = None;
    let mut grace_deadline: Option<Instant> = None;
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

        if early_termination.is_none() {
            match early_rx.try_recv() {
                Ok(signal) => {
                    let _ = child.kill();
                    early_termination = Some(signal);
                    grace_deadline = Some(Instant::now() + grace_period);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
            }
        }

        if early_termination.is_none() && let Some(ref wd_rx) = watchdog_rx {
            match wd_rx.try_recv() {
                Ok(req) => {
                    let _ = child.kill();
                    early_termination = Some(watchdog_request_to_early_termination(req));
                    grace_deadline = Some(Instant::now() + grace_period);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
            }
        }

        if early_termination.is_none()
            && let Some(metrics) = live_metrics.as_ref()
            && let Some(signal) =
                super::timeouts::detect_opencode_hang_termination(metrics, Instant::now(), stop_threshold)
        {
            let _ = child.kill();
            early_termination = Some(signal);
            grace_deadline = Some(Instant::now() + grace_period);
        }

        if let Some(deadline) = grace_deadline
            && Instant::now() >= deadline
        {
            let _ = child.kill();
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
        EarlyTermination::CompletedButHung { .. } => {
            summary.exit_code = 0;
            summary.is_error = false;
            summary.error_kind = None;
            summary.error_message = None;
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
        Some(EarlyTermination::Timeout { .. }) => {
            claudine::harness::ProcessTermination::TimedOut
        }
        Some(EarlyTermination::StepTimeout { .. }) => {
            claudine::harness::ProcessTermination::TimedOut
        }
        Some(EarlyTermination::CompletedButHung { .. }) => {
            claudine::harness::ProcessTermination::Completed
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
    fn apply_early_termination_completed_but_hung_restores_success() {
        let mut summary = StreamExecutionSummary {
            exit_code: 143,
            is_error: true,
            error_kind: Some("agent_native".into()),
            error_message: Some("killed".into()),
            ..Default::default()
        };

        apply_early_termination_to_summary(
            &mut summary,
            &EarlyTermination::CompletedButHung {
                message: "OpenCode reported stop but stayed alive".into(),
            },
        );

        assert_eq!(summary.exit_code, 0);
        assert!(!summary.is_error);
        assert!(summary.error_kind.is_none());
        assert!(summary.error_message.is_none());
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
        use crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot;
        use super::{WatchdogTermination, WatchdogTerminationReason};

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
        use crate::commands::wrap::exec::subagent_watchdog::ActiveSubagentSnapshot;
        use super::{WatchdogTermination, WatchdogTerminationReason};

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
        assert_eq!(outstanding[0].elapsed_since_progress, Duration::from_secs(900));
    }
}
