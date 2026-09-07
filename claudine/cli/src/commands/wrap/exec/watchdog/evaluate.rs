//! Timeout-evaluation core for the unified two-rule watchdog.
//!
//! [`evaluate_timeout_tick`] is the pure predicate the ticker thread calls each
//! cadence: it applies the wall-clock (`timeout`) and stream-silence
//! (`step_timeout`) rules, encodes the OpenCode per-step grace, and produces a
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
///    AND no tools or subagents are currently in flight AND
///    `now - silence_reference >= step_timeout`, fire `StepTimeout` with
///    `outstanding = watchdog_state.active_subagents(now)` for diagnostic
///    enrichment. The silence reference is
///    [`LiveMetricsState::silence_reference`](claudine::stream::progress::LiveMetricsState::silence_reference):
///    the newest of `started_at`, `last_event_at`, and `last_byte_at`. A
///    child that spawns and then emits nothing is bounded from `started_at`.
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

    // Rule 2: stream silence, measured from the single reference the
    // library owns (`LiveMetricsState::silence_reference`): the newest of
    // the child's spawn instant, the structured-event clock
    // (`last_event_at`), and the raw-byte clock (`last_byte_at`). Spawn is
    // the reference only until real output exists — that is what bounds a
    // child that starts successfully and then goes silent forever — and
    // the byte clock keeps providers whose structured events lag behind
    // real progress (notably OpenCode) from being killed while producing.
    //
    // Stuck-aware evaluation: a tool or subagent is "stuck" when its
    // `last_progress_at` is older than the step_timeout budget. The rule
    // is suppressed only when ALL in-flight items are active (none stuck).
    // If any item is stuck, the silence rule is allowed to fire so hung
    // work does not block termination indefinitely.
    if let Some(budget) = config.step_timeout {
        let (
            silence_reference,
            silence_origin,
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
                    g.silence_reference(started_at),
                    g.silence_origin(),
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
        let silence = now.saturating_duration_since(silence_reference);
        // OpenCode-specific grace: this provider does not emit
        // `tool_start` or `task_started` events, so `in_flight` /
        // `in_flight_subagents` stay empty during legitimate work and
        // the stuck-aware suppression above has nothing to suppress
        // against. What survives is the mid-step window: `step_in_flight`
        // is true (a step is open between `step_start` and the next
        // `step_finish`) AND at least one of the structured-event clock or
        // the raw-byte clock is still within the budget. Mid-step silence
        // is expected while subagents work, but if BOTH clocks are stale
        // beyond the budget the breach still fires — the per-step grace
        // must not override the byte-heartbeat backstop, otherwise an
        // OpenCode session that emits `step_start` and then dies silently
        // (the `2026-05-10` ndjson hang) is suppressed forever.
        //
        // Cold start (no `step_start` and no `step_finish` yet) has no arm
        // of its own. It used to suppress unconditionally, which made a
        // provider that never got past its own bootstrap unkillable unless
        // the opt-in wall-clock `timeout` was set. It is now bounded by the
        // shared silence budget below, like every other silent state.
        let both_clocks_stale = match (last_event_at, last_byte_at) {
            (Some(e), Some(b)) => {
                now.saturating_duration_since(e) >= budget
                    && now.saturating_duration_since(b) >= budget
            }
            _ => false,
        };
        let mid_step_with_recent_activity = step_in_flight && !both_clocks_stale;
        if config.provider == Some(claudine::provider::Provider::OpenCode)
            && mid_step_with_recent_activity
        {
            return WatchdogTickResult::Ok;
        }
        // Suppress step_timeout when all in-flight items are active (none stuck).
        if any_active && !any_stuck {
            return WatchdogTickResult::Ok;
        }
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
                silence_origin,
                &outstanding,
                &stuck_tools,
                &stuck_subagents,
                is_opencode.then_some(OpenCodeBreachContext {
                    subagent_done_count,
                    step_in_flight,
                    first_step_completed: provider_status_seen,
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

    WatchdogTickResult::Ok
}
