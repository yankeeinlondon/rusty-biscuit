use std::process::Child;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use claudine::runaway::Trip;
use claudine::stream::logs::{EarlyTermination, StalledGenerationContext};
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

/// Request sent when the wrapper has already received the successful final
/// response but the provider keeps its transport process alive.
///
/// The wait loop still terminates the child tree through the same signal /
/// Job Object path used for watchdog and content-guard termination, but the
/// resulting process outcome remains `Completed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletionTermination;

/// Visible feedback line emitted on each counted interrupt (Q14).
///
/// One static byte string per rung so the write stays async-signal-safe
/// (no allocation, no formatting) — `libc::write(2, …)` is the only side
/// effect beyond the atomic counter and `libc::kill`. The glyphs are
/// UTF-8 encoded inline so the message survives any locale setting.
#[cfg(unix)]
const INTERRUPT_FEEDBACK_FIRST: &[u8] =
    "\u{26a0} interrupt received \u{2014} press again to escalate\n".as_bytes();
#[cfg(unix)]
const INTERRUPT_FEEDBACK_REPEAT: &[u8] =
    "\u{26a0} interrupt received \u{2014} escalating\n".as_bytes();

/// Emit the Q14 visible-feedback line to stderr (fd 2) without allocating.
///
/// `count` is the just-incremented press counter (1-indexed). The first
/// press tells the user a second press will escalate; subsequent presses
/// acknowledge that escalation is underway. The write is best-effort — a
/// short write or a closed stderr must never panic the signal handler.
#[cfg(unix)]
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
#[cfg(unix)]
fn escalation_signal(interactive: bool, count: u8) -> i32 {
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
#[cfg(unix)]
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

#[cfg(unix)]
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
#[cfg(unix)]
pub(crate) fn send_signal_to_child(child_pid: u32, child_in_own_pgroup: bool, signal: i32) {
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
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    mut watchdog_rx: Option<Receiver<WatchdogTermination>>,
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
        watchdog_rx.take(),
        None,
        kill_grace,
        interactive,
    )
}

#[cfg(not(unix))]
pub(crate) fn wait_with_signal_early_termination_and_completion(
    child: &mut Child,
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    mut watchdog_rx: Option<Receiver<WatchdogTermination>>,
    completion_rx: Option<Receiver<CompletionTermination>>,
    kill_grace: Duration,
    interactive: bool,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    windows_wait_loop(
        child,
        child_in_own_pgroup,
        early_rx,
        watchdog_rx.take(),
        completion_rx,
        kill_grace,
        interactive,
    )
}

/// Visible feedback line emitted on each counted interrupt on Windows (Q14).
///
/// Mirrors the Unix ladder's messages byte-for-byte so users see identical
/// feedback regardless of host. Emitted from the wait loop (not the console
/// handler) so the handler stays minimal — just an atomic increment and a
/// `TRUE` return — and feedback uses the normal `eprintln!` path.
#[cfg(not(unix))]
const INTERRUPT_FEEDBACK_FIRST: &[u8] =
    "\u{26a0} interrupt received \u{2014} press again to escalate\n".as_bytes();

#[cfg(not(unix))]
const INTERRUPT_FEEDBACK_REPEAT: &[u8] =
    "\u{26a0} interrupt received \u{2014} escalating\n".as_bytes();

/// Process-global console-interrupt counter for Windows.
///
/// `SetConsoleCtrlHandler` accepts only a plain `extern "system" fn` (no
/// captured state), so the press counter lives in a process-global static.
/// Claudine waits on at most one wrapped child per process at a time
/// (composition `sequence` runs are strictly serial), so there is no
/// concurrency hazard — each [`windows_wait_loop`] call resets the counter
/// at entry.
#[cfg(not(unix))]
static CONSOLE_INTERRUPT_COUNT: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(0);

/// Process-global flag set when a graceful console break has already been
/// sent to the child, so the next press escalates to a forceful
/// `TerminateJobObject` / `TerminateProcess`.
#[cfg(not(unix))]
static CONSOLE_FORCE_KILL_SENT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Windows console-control handler installed by [`windows_wait_loop`].
///
/// Returns `TRUE` for `CTRL_C_EVENT` and `CTRL_BREAK_EVENT` to suppress the
/// default disposition (terminate the whole console process group); we
/// drive termination explicitly from the wait loop. The only side effect
/// is incrementing the global counter — visible feedback, escalation, and
/// PID-recycle guards all live in the wait loop where they can use the
/// full Rust API.
#[cfg(not(unix))]
unsafe extern "system" fn claudine_console_ctrl_handler(ctrl_type: u32) -> windows::core::BOOL {
    use windows::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};

    if ctrl_type == CTRL_C_EVENT || ctrl_type == CTRL_BREAK_EVENT {
        CONSOLE_INTERRUPT_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        windows::core::BOOL(1) // TRUE — suppress default
    } else {
        // Close, Logoff, Shutdown — let the default handler decide.
        windows::core::BOOL(0)
    }
}

/// Real Windows implementation of the unified wait loop with parity to the
/// Unix group-signal/escalation behavior (Q15).
///
/// Spawns the child in `CREATE_NEW_PROCESS_GROUP` (set at the `Command`
/// build site in `spawn.rs`), assigns it to a Job Object with
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so the whole tree terminates as a
/// unit, and registers a console Ctrl+C handler that increments a counter.
/// The escalation ladder mirrors the spec's Windows analog:
///
/// - Press 1 → `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group)`
///   (graceful — Windows has no SIGINT/SIGTERM split, so Ctrl+Break is the
///   closest "ask nicely" rung; `interactive` does not differentiate because
///   Windows lacks a separate first-rung interrupt).
/// - Press 2 → `TerminateJobObject` (forceful, kills the whole tree).
///
/// For the interactive-TUI passthrough case (`child_in_own_pgroup == false`)
/// the child shares the console and receives Ctrl+C naturally from the
/// terminal; we still register the handler so the press counter is visible
/// and the ladder's forceful rung (`child.kill()`) can still fire when the
/// user presses a second time.
///
/// ## Verification gap
///
/// Runtime Windows behavior cannot be exercised from the macOS dev host;
/// the implementation must be validated on a Windows host or in CI. The
/// structure mirrors the spec's Q15 design (Job Object + console events)
/// and is written to compile cleanly on `x86_64-pc-windows-gnu`.
#[cfg(not(unix))]
fn windows_wait_loop(
    child: &mut Child,
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    watchdog_rx: Option<Receiver<WatchdogTermination>>,
    completion_rx: Option<Receiver<CompletionTermination>>,
    kill_grace: Duration,
    _interactive: bool,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    use std::os::windows::io::AsRawHandle;
    use std::sync::atomic::Ordering;

    // Mirror the Unix loop: while this loop owns the child's interrupt
    // escalation, the compose-scoped Ctrl+C guard must not force-exit the
    // wrapper. Cleared on every exit path.
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{
        CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent, SetConsoleCtrlHandler,
    };
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows::Win32::System::Threading::GetProcessId;

    let child_handle: HANDLE = {
        // Safety: `Child::as_raw_handle()` borrows the owned process HANDLE.
        // Wrapping the raw `*mut c_void` in a Win32 `HANDLE` is safe as long
        // as the `Child` outlives this borrow — which it does (the `&mut
        // Child` parameter is held for the duration of the loop).
        HANDLE(child.as_raw_handle())
    };
    let child_process_id = unsafe { GetProcessId(child_handle) };
    let child_pid = child.id();

    // Reset the global console-interrupt state for this run.
    CONSOLE_INTERRUPT_COUNT.store(0, Ordering::SeqCst);
    CONSOLE_FORCE_KILL_SENT.store(false, Ordering::SeqCst);

    // Create a Job Object with kill-on-close so any descendants die when
    // we close the job handle (whether by normal Drop or by forceful
    // `TerminateJobObject`).
    let job = unsafe { CreateJobObjectW(None, None) }?;
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )?;
    }

    // Assign the child to the Job. Per Windows docs, assignment works after
    // spawn as long as the child hasn't spawned descendants yet — true here
    // because we assign immediately after `Command::spawn`.
    if child_in_own_pgroup {
        unsafe { AssignProcessToJobObject(job, child_handle)?; }
    }

    // Install the console Ctrl-C handler. Removed on Drop via the guard so
    // subsequent calls do not stack handlers.
    unsafe { SetConsoleCtrlHandler(Some(claudine_console_ctrl_handler), true)?; }
    let _handler_guard = ConsoleHandlerGuard(claudine_console_ctrl_handler);

    let mut early_termination: Option<EarlyTermination> = None;
    let mut grace_deadline: Option<Instant> = None;
    let mut reap_deadline: Option<Instant> = None;
    let mut watchdog_rx = watchdog_rx;
    let mut completion_rx = completion_rx;
    let mut completion_requested = false;
    let mut last_emitted_count: u8 = 0;
    let poll_interval = Duration::from_millis(75);
    let grace_period = kill_grace;

    loop {
        if let Some(status) = child.try_wait()? {
            let code = if completion_requested {
                0
            } else {
                exit_code_from_status(status)
            };
            let pressed = CONSOLE_INTERRUPT_COUNT.load(Ordering::SeqCst) > 0;
            let termination = if pressed {
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

        // Drain the process-global console counter and react to new presses.
        let global_count = CONSOLE_INTERRUPT_COUNT.load(Ordering::SeqCst);
        if global_count > last_emitted_count {
            // Q14 — emit one feedback line per newly observed press.
            let msg = if global_count == 1 {
                INTERRUPT_FEEDBACK_FIRST
            } else {
                INTERRUPT_FEEDBACK_REPEAT
            };
            let _ = std::io::Write::write_all(&mut std::io::stderr(), msg);
            last_emitted_count = global_count;

            if !CONSOLE_FORCE_KILL_SENT.load(Ordering::SeqCst)
                && child.try_wait()?.is_none()
            {
                if child_in_own_pgroup {
                    if global_count == 1 {
                        // Graceful rung: send CTRL_BREAK_EVENT to the
                        // child's process group. CTRL_C_EVENT cannot
                        // target a specific group.
                        unsafe {
                            let _ = GenerateConsoleCtrlEvent(
                                CTRL_BREAK_EVENT,
                                child_process_id,
                            );
                        }
                        tracing::info!(
                            child_pid,
                            "interrupt received; sent CTRL_BREAK_EVENT to child process group",
                        );
                    } else {
                        // Forceful rung: terminate the entire Job.
                        unsafe {
                            let _ = TerminateJobObject(job, 1);
                        }
                        CONSOLE_FORCE_KILL_SENT.store(true, Ordering::SeqCst);
                        reap_deadline = Some(Instant::now() + POST_SIGKILL_REAP_TIMEOUT);
                        tracing::warn!(
                            child_pid,
                            "second interrupt received; force-terminating Job Object",
                        );
                    }
                } else if global_count >= 2 {
                    // Interactive TUI passthrough: child shares our
                    // console and already received Ctrl+C from the
                    // terminal. Force-kill on the second press.
                    let _ = child.kill();
                    CONSOLE_FORCE_KILL_SENT.store(true, Ordering::SeqCst);
                    reap_deadline = Some(Instant::now() + POST_SIGKILL_REAP_TIMEOUT);
                }
            }
        }

        if let Some(deadline) = reap_deadline
            && Instant::now() >= deadline
        {
            tracing::error!(
                child_pid,
                "child did not reap after TerminateJobObject; giving up to avoid hanging the wrapper"
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
                        let _ = unsafe { TerminateJobObject(job, 1) };
                        CONSOLE_FORCE_KILL_SENT.store(true, Ordering::SeqCst);
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
                        let _ = unsafe { TerminateJobObject(job, 0) };
                        CONSOLE_FORCE_KILL_SENT.store(true, Ordering::SeqCst);
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

        if early_termination.is_none()
            && let Some(ref wd_rx) = watchdog_rx
        {
            match wd_rx.try_recv() {
                Ok(req) => {
                    if child.try_wait()?.is_none() {
                        let _ = unsafe { TerminateJobObject(job, 1) };
                        CONSOLE_FORCE_KILL_SENT.store(true, Ordering::SeqCst);
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
                let _ = unsafe { TerminateJobObject(job, 1) };
                CONSOLE_FORCE_KILL_SENT.store(true, Ordering::SeqCst);
                reap_deadline = Some(Instant::now() + POST_SIGKILL_REAP_TIMEOUT);
            }
            grace_deadline = None;
        }

        std::thread::sleep(poll_interval);
    }
}

/// RAII guard that uninstalls a Windows console handler on Drop.
///
/// `SetConsoleCtrlHandler(Some(fn), false)` removes the handler; if it is
/// the last registered handler the default Ctrl+C disposition is restored.
/// Removing the handler on Drop prevents the previous run's handler from
/// leaking into the next wait-loop call.
#[cfg(not(unix))]
struct ConsoleHandlerGuard(unsafe extern "system" fn(u32) -> windows::core::BOOL);

#[cfg(not(unix))]
impl Drop for ConsoleHandlerGuard {
    fn drop(&mut self) {
        use windows::Win32::System::Console::SetConsoleCtrlHandler;
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(self.0), false);
        }
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
        EarlyTermination::ExitExpression { pattern, scope } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("exit_expression".into());
            summary.error_message = Some(render_exit_expression_message(pattern, scope.as_deref()));
        }
        EarlyTermination::RunawayRepetition {
            cycle_len, repeats, ..
        } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("runaway_repetition".into());
            summary.error_message = Some(render_runaway_repetition_message(*cycle_len, *repeats));
        }
        EarlyTermination::RunawayVolume { lines, bytes } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("runaway_volume".into());
            summary.error_message = Some(render_runaway_volume_message(*lines, *bytes));
        }
        EarlyTermination::RepeatedStreamError { count } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("repeated_stream_error".into());
            summary.error_message = Some(render_repeated_stream_error_message(*count));
        }
        EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("stalled_generation".into());
            summary.error_message = Some(render_stalled_generation_message(
                *generation_count,
                *stall_duration,
                context,
            ));
        }
    }
}

/// Render the user-facing message for an [`EarlyTermination`] trip.
///
/// Returns `None` only when the variant carries no inline message (none
/// today — every variant has one). Used both by
/// [`apply_early_termination_to_summary`] (to populate `summary.error_message`)
/// and by the spawn-side post-wait match (to emit a styled `Warning` line on
/// stderr) so the two surfaces never drift apart.
pub(crate) fn early_termination_message(termination: &EarlyTermination) -> Option<String> {
    match termination {
        EarlyTermination::RateLimit { message, .. } => Some(message.clone()),
        EarlyTermination::Timeout { message } => Some(message.clone()),
        EarlyTermination::StepTimeout { message, .. } => Some(message.clone()),
        EarlyTermination::ExitExpression { pattern, scope } => {
            Some(render_exit_expression_message(pattern, scope.as_deref()))
        }
        EarlyTermination::RunawayRepetition { cycle_len, repeats } => {
            Some(render_runaway_repetition_message(*cycle_len, *repeats))
        }
        EarlyTermination::RunawayVolume { lines, bytes } => {
            Some(render_runaway_volume_message(*lines, *bytes))
        }
        EarlyTermination::RepeatedStreamError { count } => {
            Some(render_repeated_stream_error_message(*count))
        }
        EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        } => Some(render_stalled_generation_message(
            *generation_count,
            *stall_duration,
            context,
        )),
    }
}

/// Render the error message for an [`EarlyTermination::ExitExpression`] trip,
/// naming the matched pattern and (when present) its scope.
fn render_exit_expression_message(pattern: &str, scope: Option<&str>) -> String {
    match scope {
        Some(scope) if !scope.is_empty() => {
            format!("exit expression matched ({scope}): {pattern}")
        }
        _ => format!("exit expression matched: {pattern}"),
    }
}

/// Render the error message for an [`EarlyTermination::RunawayRepetition`]
/// trip, naming the detected cycle length and observed repeat count.
fn render_runaway_repetition_message(cycle_len: usize, repeats: usize) -> String {
    format!(
        "runaway repetition detected (cycle length {cycle_len}, {repeats} repeats); \
         terminated to stop the loop"
    )
}

/// Render the error message for an [`EarlyTermination::RunawayVolume`] trip,
/// naming the line and byte counters at the moment of breach.
fn render_runaway_volume_message(lines: u64, bytes: u64) -> String {
    format!(
        "output volume cap exceeded ({lines} lines, {bytes} bytes); \
         terminated to bound the runaway"
    )
}

/// Render the error message for an [`EarlyTermination::RepeatedStreamError`]
/// trip, naming the consecutive-failure count at the moment of breach.
fn render_repeated_stream_error_message(count: u32) -> String {
    format!(
        "provider stream failed {count} times with no progress; \
         terminated to stop the retry loop"
    )
}

/// Render the error message for an [`EarlyTermination::StalledGeneration`]
/// trip, naming the generation-attempt count, the elapsed progress silence,
/// and any available safe OpenCode context (session id, step, agent, provider
/// id, model id, mode). Never includes prompt text or tool payloads.
fn render_stalled_generation_message(
    generation_count: u32,
    stall_duration: Duration,
    context: &StalledGenerationContext,
) -> String {
    let seconds = stall_duration.as_secs();
    let mut message = format!(
        "provider attempted {generation_count} generations over {seconds}s with no progress; \
         terminated to stop the stalled-generation loop"
    );

    let mut details: Vec<String> = Vec::new();
    if let Some(session_id) = &context.session_id {
        details.push(format!("session={session_id}"));
    }
    if let Some(step) = context.step {
        details.push(format!("step={step}"));
    }
    if let Some(agent) = &context.agent {
        details.push(format!("agent={agent}"));
    }
    if let Some(provider_id) = &context.provider_id {
        details.push(format!("provider={provider_id}"));
    }
    if let Some(model_id) = &context.model_id {
        details.push(format!("model={model_id}"));
    }
    if let Some(mode) = &context.mode {
        details.push(format!("mode={mode}"));
    }
    if !details.is_empty() {
        message.push_str(&format!(" ({})", details.join(", ")));
    }

    message
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
        // Content-guard trips (exit-expression / runaway-repetition /
        // runaway-volume) map to `Aborted` so `classify_failure` yields
        // `AgentFailure` — never `TimedOut` (which would route through the
        // lifecycle `failure` stack as a retryable timeout and reproduce the
        // runaway).
        //
        // The repeated-stream-error backstop is also a fail-fast abort: the
        // provider failed every retry, so a retryable timeout classification
        // would only reproduce the loop.
        //
        // The stalled-generation backstop fires for the same reason: retrying
        // a silently-dropped generation loop reproduces the stall, so it must
        // never route through `TimedOut` / `handle_timeout:`.
        Some(
            EarlyTermination::ExitExpression { .. }
            | EarlyTermination::RunawayRepetition { .. }
            | EarlyTermination::RunawayVolume { .. }
            | EarlyTermination::RepeatedStreamError { .. }
            | EarlyTermination::StalledGeneration { .. },
        ) => claudine::harness::ProcessTermination::Aborted,
        None => claudine::harness::ProcessTermination::Completed,
    }
}

/// Convert a lib-side detector [`Trip`] into the lib-side
/// [`EarlyTermination`] the wait loop carries on its termination channel.
///
/// This is the single bridge between the pure content detector
/// (`claudine::runaway`, Phase 2) and the termination-channel types — keeping
/// it here means the detector never imports [`EarlyTermination`]. Fields are
/// copied verbatim; the detector owns the structured detail, the termination
/// channel owns the routing.
///
/// The consumer is the live semantic sink's content detector
/// (`live_semantic_sink`), which sends the converted signal on the unified
/// early-termination channel the wait loop polls.
pub(crate) fn trip_to_early_termination(trip: Trip) -> EarlyTermination {
    match trip {
        Trip::ExitExpression { pattern, scope } => {
            EarlyTermination::ExitExpression { pattern, scope }
        }
        Trip::RunawayRepetition { cycle_len, repeats } => {
            EarlyTermination::RunawayRepetition { cycle_len, repeats }
        }
        Trip::RunawayVolume { lines, bytes } => EarlyTermination::RunawayVolume { lines, bytes },
    }
}

/// Extract the structured [`GuardContext`] for a content-guard
/// [`EarlyTermination`], or `None` for the non-content variants
/// (rate-limit / timeout / step-timeout, which carry no guard context).
///
/// Only the cluster relevant to the trip is populated; every other field
/// stays `None`. Carried onto [`ProcessResult`](super::ProcessResult) so the
/// attempt outcome (and, in Phase 7, the failure-handler payload) can read
/// the guard detail without re-parsing the prose `error_message`.
///
/// [`GuardContext`]: claudine::harness::GuardContext
pub(crate) fn early_termination_guard_context(
    termination: &EarlyTermination,
) -> Option<claudine::harness::GuardContext> {
    use claudine::harness::GuardContext;
    match termination {
        EarlyTermination::ExitExpression { pattern, scope } => Some(GuardContext {
            pattern: Some(pattern.clone()),
            scope: scope.clone(),
            ..GuardContext::default()
        }),
        EarlyTermination::RunawayRepetition { cycle_len, repeats } => Some(GuardContext {
            cycle_len: Some(*cycle_len),
            repeats: Some(*repeats),
            ..GuardContext::default()
        }),
        EarlyTermination::RunawayVolume { lines, bytes } => Some(GuardContext {
            lines: Some(*lines),
            bytes: Some(*bytes),
            ..GuardContext::default()
        }),
        EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        } => Some(GuardContext {
            generation_count: Some(*generation_count),
            stall_duration_ms: Some(stall_duration.as_millis() as u64),
            // Carry only the safe identity metadata the detector captured;
            // each stays `None` when OpenCode never tagged it.
            session_id: context.session_id.clone(),
            step: context.step,
            agent: context.agent.clone(),
            provider_id: context.provider_id.clone(),
            model_id: context.model_id.clone(),
            mode: context.mode.clone(),
            ..GuardContext::default()
        }),
        EarlyTermination::RateLimit { .. }
        | EarlyTermination::Timeout { .. }
        | EarlyTermination::StepTimeout { .. }
        | EarlyTermination::RepeatedStreamError { .. } => None,
    }
}

#[cfg(test)]
mod tests;
