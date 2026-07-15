//! Windows wait/escalation implementation of the shared termination interface.
//!
//! Mirrors the Unix process-group ladder with a Job Object (kill-on-close)
//! and console-control events: press 1 sends `CTRL_BREAK_EVENT` to the child's
//! process group, press 2 force-terminates the Job. Early-termination,
//! completion, and watchdog channels drive the same forceful Job termination.

use std::process::Child;
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

/// Wait for the child, tracking console interrupts for correct termination
/// labeling (Windows counterpart of the Unix `wait_with_signal_handling`).
pub(crate) fn wait_with_signal_handling(
    child: &mut Child,
    _child_in_own_pgroup: bool,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();
    let status = child.wait()?;
    Ok((
        exit_code_from_status(status),
        claudine::harness::ProcessTermination::Completed,
    ))
}

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

/// Process-global console-interrupt counter for Windows.
///
/// `SetConsoleCtrlHandler` accepts only a plain `extern "system" fn` (no
/// captured state), so the press counter lives in a process-global static.
/// Claudine waits on at most one wrapped child per process at a time
/// (composition `sequence` runs are strictly serial), so there is no
/// concurrency hazard — each [`windows_wait_loop`] call resets the counter
/// at entry.
static CONSOLE_INTERRUPT_COUNT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Process-global flag set when a graceful console break has already been
/// sent to the child, so the next press escalates to a forceful
/// `TerminateJobObject` / `TerminateProcess`.
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
/// build site in `spawn/setup.rs`), assigns it to a Job Object with
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
struct ConsoleHandlerGuard(unsafe extern "system" fn(u32) -> windows::core::BOOL);

impl Drop for ConsoleHandlerGuard {
    fn drop(&mut self) {
        use windows::Win32::System::Console::SetConsoleCtrlHandler;
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(self.0), false);
        }
    }
}
