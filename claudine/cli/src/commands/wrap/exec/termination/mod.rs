//! Wrapper process-termination policy and platform wait/escalation.
//!
//! Split by stable responsibility so the provider-neutral policy has one owner
//! and the two platform loops share it rather than copying it:
//!
//! - [`reasons`] — provider-neutral termination reasons ([`WatchdogTermination`],
//!   [`CompletionTermination`]) and the projection from watchdog/detector inputs
//!   into the [`EarlyTermination`] the wait loop carries and its resulting
//!   [`ProcessTermination`](claudine::harness::ProcessTermination).
//! - [`summary`] — early-termination summary and guard-context projection.
//! - [`message`] — human-facing early-termination message rendering.
//! - platform wait/escalation: [`unix`] (process-group signal ladder) and
//!   [`windows`] (Job Object + console-control events) each implement one wait
//!   interface — `wait_with_signal_and_early_termination`,
//!   `wait_with_signal_early_termination_and_completion`, and
//!   `wait_with_signal_handling` — behind the same contract. Only the platform
//!   mechanism differs; the one semantic signal ladder and one
//!   early-termination projection above are shared.
//!
//! [`EarlyTermination`]: claudine::stream::logs::EarlyTermination

use std::time::Duration;

mod message;
mod reasons;
mod summary;

#[cfg(unix)]
mod unix;
#[cfg(not(unix))]
mod windows;

pub(crate) use message::early_termination_message;
pub(crate) use reasons::{
    CompletionTermination, WatchdogTermination, WatchdogTerminationReason,
    early_termination_process_outcome, trip_to_early_termination,
    watchdog_request_to_early_termination,
};
pub(crate) use summary::{apply_early_termination_to_summary, early_termination_guard_context};

#[cfg(unix)]
pub(crate) use unix::{
    wait_with_signal_and_early_termination, wait_with_signal_early_termination_and_completion,
    wait_with_signal_handling,
};
#[cfg(not(unix))]
pub(crate) use windows::{
    wait_with_signal_and_early_termination, wait_with_signal_early_termination_and_completion,
    wait_with_signal_handling,
};

/// Maximum time to wait for a child to reap after SIGKILL before giving up.
///
/// A wedged (D-state) child may never reap; the wrapper must not hang
/// indefinitely. This cap is intentionally conservative: the kernel has
/// already been asked to destroy the process, and the caller's timeout
/// budget has long been exhausted. Shared by both platform loops.
pub(super) const POST_SIGKILL_REAP_TIMEOUT: Duration = Duration::from_secs(10);

/// Visible feedback line emitted on the first counted interrupt (Q14).
///
/// One static byte string per rung so the Unix async-signal-safe write stays
/// allocation- and format-free; the Windows loop emits the identical bytes
/// from its poll body so users see the same feedback regardless of host. The
/// glyphs are UTF-8 encoded inline so the message survives any locale setting.
pub(super) const INTERRUPT_FEEDBACK_FIRST: &[u8] =
    "\u{26a0} interrupt received \u{2014} press again to escalate\n".as_bytes();
/// Visible feedback line emitted on every subsequent counted interrupt (Q14).
pub(super) const INTERRUPT_FEEDBACK_REPEAT: &[u8] =
    "\u{26a0} interrupt received \u{2014} escalating\n".as_bytes();

#[cfg(test)]
mod tests;
