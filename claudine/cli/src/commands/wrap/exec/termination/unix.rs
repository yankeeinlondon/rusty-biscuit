//! Unix wait/escalation implementation of the shared termination interface.
//!
//! The child leads its own process group (or shares the parent's for
//! interactive TUI passthrough); this module owns the process-group signal
//! ladder (`SIGINT → SIGTERM → SIGKILL`, compressed to `SIGTERM → SIGKILL`
//! for non-interactive runs) and the poll loop that drives early-termination,
//! completion, and watchdog channels to child-process termination.

use std::process::Child;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use claudine::stream::logs::EarlyTermination;
use color_eyre::eyre::Result;

use super::super::exit::exit_code_from_status;
use super::reasons::{
    CompletionTermination, WatchdogTermination, early_termination_process_outcome,
    watchdog_request_to_early_termination,
};
use super::{INTERRUPT_FEEDBACK_FIRST, INTERRUPT_FEEDBACK_REPEAT, POST_SIGKILL_REAP_TIMEOUT};

/// Emit the Q14 visible-feedback line to stderr (fd 2) without allocating.
///
/// `count` is the just-incremented press counter (1-indexed). The first
/// press tells the user a second press will escalate; subsequent presses
/// acknowledge that escalation is underway. The write is best-effort — a
/// short write or a closed stderr must never panic the signal handler.
unsafe fn emit_interrupt_feedback(count: u8) {
    let msg = if count == 1 {
        INTERRUPT_FEEDBACK_FIRST
    } else {
        INTERRUPT_FEEDBACK_REPEAT
    };
    // SAFETY: `write(2, …)` is async-signal-safe (POSIX.1-2008) and the
    // buffer is a static `'static` slice — no dangling pointer. Return
    // value is intentionally ignored: the handler must not fail.
    unsafe {
        let _ = libc::write(2, msg.as_ptr() as *const libc::c_void, msg.len());
    }
}

/// Resolve the next escalation signal from the press count and interactivity.
///
/// Interactive runs keep the full `SIGINT → SIGTERM → SIGKILL` ladder
/// (a human mid-session is protected from an accidental single press);
/// non-interactive runs compress it to `SIGTERM → SIGKILL` (F5) since
/// no human is present to react to a graceful SIGINT — pressing once
/// should make progress toward killing the runaway.
pub(super) fn escalation_signal(interactive: bool, count: u8) -> i32 {
    if interactive {
        match count {
            1 => libc::SIGINT,
            2 => libc::SIGTERM,
            _ => libc::SIGKILL,
        }
    } else {
        match count {
            1 => libc::SIGTERM,
            _ => libc::SIGKILL,
        }
    }
}

/// Wait for the child, acknowledging and forwarding SIGINT/SIGTERM on
/// repeated Ctrl-C.
///
/// When `child_in_own_pgroup` is true, the child was spawned with
/// `process_group(0)` and the installed SIGINT handler manually forwards
/// signals to `-child_pid` so descendants also receive them. When it is
/// false, the child shares the parent's process group (required for
/// interactive TUIs that read the controlling TTY); in that case the
/// terminal already delivers SIGINT to the child naturally, so we only
/// track the interrupt count locally.
///
/// Returns `(exit_code, termination_kind)`.
pub(crate) fn wait_with_signal_handling(
    child: &mut Child,
    child_in_own_pgroup: bool,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::sync::atomic::{AtomicBool, AtomicU8};

    // This loop installs a child-targeting SIGINT handler, so the
    // compose-scoped Ctrl+C guard must defer to it while it runs.
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_exited = Arc::new(AtomicBool::new(false));
    let child_pid = child.id();

    let counter = Arc::clone(&interrupt_count);
    let exited = Arc::clone(&child_exited);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            // Never signal a PID we no longer own — the kernel may have
            // recycled `child_pid` onto an unrelated process by now.
            if exited.load(Ordering::SeqCst) {
                return;
            }
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            emit_interrupt_feedback(count);
            if !child_in_own_pgroup {
                // Child shares our process group; the terminal already
                // delivered SIGINT to it. Just track the count so the
                // termination kind is reported correctly.
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

    let status = child.wait()?;
    // Set the exit flag BEFORE the `_guard` drops so a SIGINT that arrives
    // in the narrow window between `wait` returning and the guard being
    // dropped still sees an exited child and refuses to signal the PID.
    child_exited.store(true, Ordering::SeqCst);
    let code = exit_code_from_status(status);
    let was_interrupted = interrupt_count.load(Ordering::SeqCst) > 0;
    let termination = if was_interrupted {
        claudine::harness::ProcessTermination::Interrupted
    } else {
        claudine::harness::ProcessTermination::Completed
    };
    Ok((code, termination))
}

/// Polling wait loop used when an [`EarlyTermination`] receiver is attached
/// to the structured stream executor.
///
/// Behaves like the legacy `wait_with_signal_handling` helper while also
/// polling the stderr-bridge channel and the watchdog ticker channel. When
/// a signal arrives, the child's process group is sent the next rung of the
/// escalation ladder and force-killed after a grace period. User Ctrl-C
/// still reports `Interrupted`; wrapper-driven early termination
/// (rate-limit recovery, timeout, or step_timeout) preserves a normal
/// `Completed` termination so downstream failure handling can inspect
/// synthesized summary fields instead of treating the run like a user
/// cancel.
///
/// Timeout enforcement is delegated to the watchdog ticker
/// (`spawn_timeout_watchdog_ticker`), which sends `WatchdogTermination`
/// requests through the `watchdog_rx` channel. This function only consumes
/// those signals and escalates them to child-process termination.
///
/// ## Interrupt ladder
///
/// When `interactive` is `true` the full `SIGINT → SIGTERM → SIGKILL`
/// ladder applies (three presses to force-kill). When `interactive` is
/// `false` the ladder compresses to `SIGTERM → SIGKILL` (F5: a
/// non-interactive run should make progress toward killing the child on
/// the first press). Every counted press also emits a visible stderr line
/// (Q14) so the user knows the press registered during an output flood.
pub(crate) fn wait_with_signal_and_early_termination(
    child: &mut Child,
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    watchdog_rx: Option<Receiver<WatchdogTermination>>,
    kill_grace: Duration,
    interactive: bool,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    wait_with_signal_early_termination_and_completion(
        child,
        child_in_own_pgroup,
        early_rx,
        watchdog_rx,
        None,
        kill_grace,
        interactive,
    )
}

pub(crate) fn wait_with_signal_early_termination_and_completion(
    child: &mut Child,
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    watchdog_rx: Option<Receiver<WatchdogTermination>>,
    completion_rx: Option<Receiver<CompletionTermination>>,
    kill_grace: Duration,
    interactive: bool,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    use std::sync::atomic::{AtomicBool, AtomicU8};

    // While this loop runs, the child-targeted SIGINT→SIGTERM→SIGKILL ladder
    // below owns escalation, so the compose-scoped Ctrl+C guard must defer to
    // it rather than force-exiting the wrapper. Cleared on every exit path.
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();

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
            // Q14 — visible feedback per press. Always emit, even when the
            // child shares our process group (interactive TUI passthrough):
            // the user pressed Ctrl-C and deserves acknowledgement that the
            // press registered.
            emit_interrupt_feedback(count);
            if !child_in_own_pgroup {
                // Child shares our process group; the terminal already
                // delivered SIGINT to it. We've recorded the count for
                // correct termination labeling — nothing more to do.
                return;
            }
            let signal = escalation_signal(interactive, count);
            libc::kill(-(child_pid as i32), signal);
        })
    }?;

    let mut early_termination: Option<EarlyTermination> = None;
    let mut grace_deadline: Option<Instant> = None;
    let mut reap_deadline: Option<Instant> = None;
    let mut watchdog_rx = watchdog_rx;
    let mut completion_rx = completion_rx;
    let mut completion_requested = false;
    let poll_interval = Duration::from_millis(75);
    let grace_period = kill_grace;

    loop {
        if let Some(status) = child.try_wait()? {
            // Mark the PID as reaped before the grace window's signal guard
            // can drop so we never signal a recycled PID.
            child_exited.store(true, Ordering::SeqCst);
            let code = if completion_requested {
                0
            } else {
                exit_code_from_status(status)
            };
            let was_interrupted = interrupt_count.load(Ordering::SeqCst) > 0;
            let termination = if was_interrupted {
                claudine::harness::ProcessTermination::Interrupted
            } else if completion_requested {
                claudine::harness::ProcessTermination::Completed
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

        if early_termination.is_none()
            && !completion_requested
            && let Some(ref done_rx) = completion_rx
        {
            match done_rx.try_recv() {
                Ok(CompletionTermination) => {
                    if child.try_wait()?.is_none() {
                        tracing::info!(
                            child_pid,
                            "completion termination received; sending SIGTERM to child process group",
                        );
                        send_signal_to_child(child_pid, child_in_own_pgroup, libc::SIGTERM);
                        completion_requested = true;
                        grace_deadline = Some(Instant::now() + grace_period);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    completion_rx = None;
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
pub(super) fn send_signal_to_child(child_pid: u32, child_in_own_pgroup: bool, signal: i32) {
    let kill_pid = if child_in_own_pgroup {
        -(child_pid as i32)
    } else {
        child_pid as i32
    };
    unsafe {
        libc::kill(kill_pid, signal);
    }
}
