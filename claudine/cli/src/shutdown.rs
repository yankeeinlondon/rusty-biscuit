//! The CLI's one ordinary exit path.
//!
//! Outbound messages and desktop notifications run as tracked background
//! tasks (`claudine::messaging::drain_deliveries`). `std::process::exit` does
//! not wait for them, so every ordinary exit goes through [`finish`], which
//! drains them while the Tokio runtime is still running, reports any that did
//! not finish, flushes stdout and stderr, and exits with the command's code.
//!
//! ## Contract
//!
//! - `async_main` ends only in [`finish`]; commands return their exit code and
//!   never exit themselves. Errors raised before the runtime exists go through
//!   [`exit_before_runtime`], since no delivery can have started yet.
//! - The process exits while the runtime is alive. Dropping the runtime would
//!   wait on blocking tasks, such as `handle`'s stdin read, and reintroduce the
//!   hang `handle`'s deadline exists to prevent.
//! - A second Ctrl+C during the drain takes the compose force-exit rung. A
//!   compose run hands its interrupt guard over with [`hold_interrupt_guard`];
//!   for every other command [`finish`] installs the ladder itself, and only
//!   when a delivery is still running, so a run that sent nothing exits exactly
//!   as before.
//! - `claudine/cli/tests/l1/exit_site_guard.rs` fails on a direct exit call
//!   outside this file and the allowlisted forced-exit sites.

use std::sync::{Mutex, OnceLock};

use claudine::messaging::{DELIVERY_DRAIN_BUDGET, drain_deliveries, has_pending_deliveries};
use tokio::time::Instant;

use crate::commands::compose::interrupt::{UserInterruptGuard, install_drain_interrupt_guard};

/// The latest moment the drain may run to, when a command set one.
static DRAIN_DEADLINE: OnceLock<Instant> = OnceLock::new();

/// A compose run's interrupt guard, kept alive until the process exits.
static INTERRUPT_HOLD: Mutex<Option<UserInterruptGuard>> = Mutex::new(None);

/// Cap the exit-time drain at `deadline`.
///
/// `claudine handle` sets its own overall deadline here so the drain cannot
/// push a hook handler past it. The first call wins.
pub(crate) fn set_drain_deadline(deadline: Instant) {
    let _ = DRAIN_DEADLINE.set(deadline);
}

/// Keep a compose run's interrupt guard installed through the exit drain.
///
/// Without this the guard drops when the command returns, and a Ctrl+C
/// during the drain would be ignored (Unix) or kill the process with the
/// wrong code (Windows).
pub(crate) fn hold_interrupt_guard(guard: UserInterruptGuard) {
    *INTERRUPT_HOLD.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(guard);
}

/// Drain pending deliveries, report the unfinished ones, flush, and exit with
/// `code`.
///
/// The drain waits at most [`DELIVERY_DRAIN_BUDGET`] in total, or less when a
/// command set an earlier deadline. It never changes `code`.
pub(crate) async fn finish(code: i32) -> std::convert::Infallible {
    let deadline = effective_deadline(Instant::now(), DRAIN_DEADLINE.get().copied());
    let _drain_interrupt = (has_pending_deliveries() && !holds_interrupt_guard())
        .then(install_drain_interrupt_guard);
    drain_deliveries(deadline).await.report();
    flush_streams();
    std::process::exit(code)
}

/// Flush and exit with `code` before the Tokio runtime exists.
pub(crate) fn exit_before_runtime(code: i32) -> ! {
    flush_streams();
    std::process::exit(code)
}

fn effective_deadline(now: Instant, command_deadline: Option<Instant>) -> Instant {
    let budget = now + DELIVERY_DRAIN_BUDGET;
    command_deadline.map_or(budget, |deadline| deadline.min(budget))
}

fn holds_interrupt_guard() -> bool {
    INTERRUPT_HOLD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_some()
}

fn flush_streams() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_drain_budget_applies_when_no_command_set_a_deadline() {
        let now = Instant::now();
        assert_eq!(effective_deadline(now, None), now + DELIVERY_DRAIN_BUDGET);
    }

    #[test]
    fn an_earlier_command_deadline_caps_the_drain() {
        let now = Instant::now();
        let handle_deadline = now + Duration::from_secs(2);
        assert_eq!(effective_deadline(now, Some(handle_deadline)), handle_deadline);
    }

    #[test]
    fn a_later_command_deadline_does_not_extend_the_budget() {
        let now = Instant::now();
        let handle_deadline = now + Duration::from_secs(60);
        assert_eq!(
            effective_deadline(now, Some(handle_deadline)),
            now + DELIVERY_DRAIN_BUDGET
        );
    }

    #[test]
    fn a_passed_command_deadline_is_kept_in_the_past() {
        let now = Instant::now();
        let passed = now - Duration::from_secs(1);
        assert_eq!(effective_deadline(now, Some(passed)), passed);
    }
}
