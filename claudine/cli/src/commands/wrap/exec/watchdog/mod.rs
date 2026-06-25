//! Unified two-rule timeout watchdog.
//!
//! Split across three focused submodules:
//! - [`evaluate`] — the [`evaluate_timeout_tick`] predicate and its
//!   [`WatchdogTickResult`].
//! - [`breach`] — breach-message formatting and standalone error rendering.
//! - [`spawn`] — the three monitor-spawner threads.

mod breach;
mod evaluate;
mod spawn;

// External callers (`exec::spawn`) reach the spawners through `watchdog::`; the
// `tests/*.rs` suites and the breach/evaluate cross-references reach the rest
// the same way.
pub(crate) use breach::{
    OpenCodeBreachContext, format_duration, format_step_timeout_breach_message,
    render_watchdog_error_to_stream,
};
pub(crate) use evaluate::{WatchdogTickResult, evaluate_timeout_tick};
pub(crate) use spawn::{
    spawn_flush_if_idle_ticker, spawn_prompt_timing_monitor, spawn_timeout_watchdog_ticker,
    spawn_wall_clock_timeout_ticker,
};

// Re-exported so the `tests/*.rs` suites reach these names through
// `use super::super::*` — the standard handles and sibling-module types the
// old single-file module imported at its head.
#[cfg(test)]
pub(crate) use std::sync::Arc;
#[cfg(test)]
pub(crate) use std::time::{Duration, Instant};
#[cfg(test)]
pub(crate) use super::subagent_watchdog::WatchdogState;
#[cfg(test)]
pub(crate) use super::termination::WatchdogTerminationReason;
#[cfg(test)]
pub(crate) use super::timeouts::TimeoutConfig;

#[cfg(test)]
mod tests {
    mod breach_messages;
    mod opencode;
    mod timeout_evaluation;
}
