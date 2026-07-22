//! Wrapper process-termination policy and platform wait/escalation.
//!
//! Split by stable responsibility so the provider-neutral policy has one owner
//! and the two platform loops share it rather than copying it:
//!
//! - [`reasons`] — provider-neutral termination reasons ([`WatchdogTermination`],
//!   [`CompletionTermination`]) and the projection from watchdog/detector inputs
//!   into the [`EarlyTermination`] the wait loop carries and its resulting
//!   [`ProcessTermination`](claudine::harness::ProcessTermination).
//! - [`coordinator`] — process-scoped interrupt bookkeeping: which children an
//!   interrupt fans out to, how far each one's ladder has advanced, and which
//!   sequence interrupt flags a press must set.
//! - [`handle`] — RAII ownership for the raw OS handles a platform wait loop
//!   creates, so a Job Object's kill-on-close fires at its own scope's end.
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

/// Compiled on every host so its bookkeeping can be unit-tested anywhere, but
/// only the Windows wait loop instantiates it — Unix gets its process-wide
/// interrupt semantics from the kernel's signal delivery instead.
#[cfg_attr(unix, allow(dead_code))]
mod coordinator;
/// Same rationale as [`coordinator`]: compiled everywhere so the ownership and
/// drop-order bookkeeping is unit-testable on hosts where the Win32 calls are
/// not, but only the Windows wait loop instantiates it.
#[cfg_attr(unix, allow(dead_code))]
mod handle;
mod message;
mod reasons;
mod summary;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

// `windows` (the crate) is a `[target.'cfg(windows)'.dependencies]` entry, so a
// target that is neither Unix nor Windows must fail by name here rather than by
// unresolved-import inside the module.
#[cfg(not(any(unix, windows)))]
compile_error!(
    "claudine's process-termination policy supports Unix and Windows only: \
     this target has neither a POSIX signal ladder nor Win32 Job Objects"
);

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
#[cfg(windows)]
pub(crate) use windows::{
    ComposeInterruptHandlerGuard, register_compose_interrupt_handler,
    register_sequence_interrupt_flag, wait_with_signal_and_early_termination,
    wait_with_signal_early_termination_and_completion, wait_with_signal_handling,
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
