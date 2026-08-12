//! Timeout-evaluation core for the unified two-rule watchdog.
//!
//! [`evaluate_timeout_tick`] is the pure predicate the ticker thread calls each
//! cadence: it applies the wall-clock (`timeout`) and stream-silence
//! (`step_timeout`) rules, encodes the OpenCode grace windows, and produces a
//! [`WatchdogTickResult`] with a fully formatted breach message
//! ([`super::breach`]) when a rule fires.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use claudine::stream::progress::LiveMetrics;

use super::super::subagent_watchdog::WatchdogState;
use super::super::termination::{WatchdogTermination, WatchdogTerminationReason};
use super::super::timeouts::TimeoutConfig;
use super::breach::{OpenCodeBreachContext, format_duration, format_step_timeout_breach_message};

/// Result of evaluating the unified two-rule timeout watchdog on a single tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WatchdogTickResult {
    /// No rules triggered; continue monitoring.
    Ok,
    /// A timeout rule breached; terminate with this request.
    Breach(WatchdogTermination),
}

/// Evaluate the unified `timeout` and `step_timeout` rules for one tick.
///
/// Rules:
///
/// 1. **Wall-clock (`timeout`).** If `config.timeout` is set and
///    `now - started_at >= timeout`, fire `Timeout`.
/// 2. **Stream-silence (`step_timeout`).** If `config.step_timeout` is set
///    AND at least one activity event has been observed
///    (`LiveMetrics.last_event_at.is_some()`) AND no tools or subagents
///    are currently in flight AND
///    `now - last_event_at >= step_timeout`, fire `StepTimeout` with
///    `outstanding = watchdog_state.active_subagents(now)` for diagnostic
///    enrichment.
///
/// The wall-clock rule is evaluated first so a deadline that elapses on
/// the same tick as a silence breach is reported as `timeout` rather than
/// `step_timeout`.
///
/// `fired` is an atomic flag that prevents double-fire across both rules;
/// once set to `true`, all subsequent evaluations return `Ok`.
pub(crate) fn evaluate_timeout_tick(
    config: &TimeoutConfig,
    now: Instant,
    started_at: Instant,
    watchdog_state: &Arc<std::sync::Mutex<WatchdogState>>,
    live_metrics: &LiveMetrics,
    fired: &AtomicBool,
) -> WatchdogTickResult {
    if fired.load(Ordering::SeqCst) {
        return WatchdogTickResult::Ok;
    }

    // Rule 1: wall-clock budget.
    if let Some(budget) = config.timeout {
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed >= budget {
            fired.store(true, Ordering::SeqCst);
            let message = format!(
                "wall-clock budget exceeded after {}",
                format_duration(elapsed),
            );
            return WatchdogTickResult::Breach(WatchdogTermination {
                reason: WatchdogTerminationReason::Timeout,
                message,
                stuck_subagents: Vec::new(),
            });
        }
    }

    // Rule 2: stream silence. Requires that at least one activity signal
    // has been observed past initial session start, matching the existing
    // first-event grace semantics. Activity is the more recent of the
    // structured-event clock (`last_event_at`) and the raw-byte clock
    // (`last_byte_at`), so providers whose stream is sparse enough that
    // structured events lag behind real progress (notably OpenCode) still
    // refresh the silence reference whenever bytes flow.
    //
    // Stuck-aware evaluation: a tool or subagent is "stuck" when its
    // `last_progress_at` is older than the step_timeout budget. The rule
    // is suppressed only when ALL in-flight items are active (none stuck).
    // If any item is stuck, the silence rule is allowed to fire so hung
    // work does not block termination indefinitely.
    if let Some(budget) = config.step_timeout {
        let (
            last_activity_at,
            last_event_at,
            last_byte_at,
            stuck_tools,
            stuck_subagents,
            any_active,
            any_stuck,
            provider_status_seen,
            step_in_flight,
            subagent_done_count,
        ) = match live_metrics.lock() {
            Ok(g) => {
                let stuck_tools: Vec<claudine::stream::progress::InFlightTool> =
                    g.stuck_tools(now, budget).into_iter().cloned().collect();
                let stuck_subagents: Vec<claudine::stream::progress::InFlightSubagent> = g
                    .stuck_subagents(now, budget)
                    .into_iter()
                    .cloned()
                    .collect();
                let any_stuck = !stuck_tools.is_empty() || !stuck_subagents.is_empty();
                let any_active = !g.in_flight.is_empty() || !g.in_flight_subagents.is_empty();
                let provider_status_seen = g.provider_status.is_some();
                let step_in_flight = g.step_in_flight;
                let subagent_done_count = g.subagent_done_count;
                (
                    g.last_activity_at(),
                    g.last_event_at,
                    g.last_byte_at,
                    stuck_tools,
                    stuck_subagents,
                    any_active,
                    any_stuck,
                    provider_status_seen,
                    step_in_flight,
                    subagent_done_count,
                )
            }
            Err(_) => return WatchdogTickResult::Ok,
        };
        // OpenCode-specific grace: this provider does not emit
        // `tool_start` or `task_started` events, so `in_flight` /
        // `in_flight_subagents` stay empty during legitimate work and
        // the stuck-aware suppression above has nothing to suppress
        // against. Two distinct conditions suppress the silence rule:
        //
        // 1. **Cold start** — `step_in_flight` is false AND
        //    `provider_status` is None: no `step_start` and no
        //    `step_finish` have been observed yet. Suppress
        //    unconditionally so slow startup / slow first turns are not
        //    misclassified as a hang.
        // 2. **Mid-step with recent activity** — `step_in_flight` is
        //    true (a step is open between `step_start` and the next
        //    `step_finish`) AND at least one of the structured-event
        //    clock or the raw-byte clock is still within the budget.
        //    Mid-step silence is expected while subagents work, but if
        //    BOTH clocks are stale beyond the budget the breach still
        //    fires — the per-step grace must not override the
        //    byte-heartbeat backstop, otherwise an OpenCode session
        //    that emits `step_start` and then dies silently (the
        //    `2026-05-10` ndjson hang) is suppressed forever.
        //
        // The wall-clock `timeout` rule above remains the unconditional
        // backstop in both cases.
        let both_clocks_stale = match (last_event_at, last_byte_at) {
            (Some(e), Some(b)) => {
                now.saturating_duration_since(e) >= budget
                    && now.saturating_duration_since(b) >= budget
            }
            _ => false,
        };
        let is_cold_start = !step_in_flight && !provider_status_seen;
        let mid_step_with_recent_activity = step_in_flight && !both_clocks_stale;
        if config.provider == Some(claudine::provider::Provider::OpenCode)
            && (is_cold_start || mid_step_with_recent_activity)
        {
            return WatchdogTickResult::Ok;
        }
        // Suppress step_timeout when all in-flight items are active (none stuck).
        if any_active && !any_stuck {
            return WatchdogTickResult::Ok;
        }
        if let Some(last) = last_activity_at {
            let silence = now.saturating_duration_since(last);
            if silence >= budget {
                let (outstanding, recent_subagents) = match watchdog_state.lock() {
                    Ok(g) => {
                        let outstanding = g.outstanding_at_breach(now);
                        let recent = g.recent_subagents.clone();
                        (outstanding, recent)
                    }
                    Err(_) => (Vec::new(), std::collections::VecDeque::new()),
                };
                let is_opencode = config.provider == Some(claudine::provider::Provider::OpenCode);
                fired.store(true, Ordering::SeqCst);
                let message = format_step_timeout_breach_message(
                    silence,
                    &outstanding,
                    &stuck_tools,
                    &stuck_subagents,
                    is_opencode.then_some(OpenCodeBreachContext {
                        subagent_done_count,
                        step_in_flight,
                        recent_subagents,
                        now,
                    }),
                );
                return WatchdogTickResult::Breach(WatchdogTermination {
                    reason: WatchdogTerminationReason::StepTimeout,
                    message,
                    stuck_subagents: outstanding,
                });
            }
        }
    }

    WatchdogTickResult::Ok
}
