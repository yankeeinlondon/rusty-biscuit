//! Windows wait/escalation implementation of the shared termination interface.
//!
//! Mirrors the Unix process-group ladder with a Job Object (kill-on-close)
//! and console-control events: press 1 sends `CTRL_BREAK_EVENT` to the child's
//! process group, press 2 force-terminates the Job. Early-termination,
//! completion, and watchdog channels drive the same forceful Job termination.
//!
//! Escalation state is **not** owned by the wait loop. Windows delivers a
//! console control event to the process, and a parallel sequence group runs
//! several wrapped children in sibling threads of that one process, so press
//! counting and fan-out live in the process-scoped
//! [`InterruptRegistry`](super::coordinator::InterruptRegistry). The console
//! handler is installed once for the process (refcounted by
//! [`ConsoleHandlerGuard`]) and drives every registered child plus every
//! registered sequence interrupt flag.

use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use claudine::stream::logs::EarlyTermination;
use color_eyre::eyre::Result;

use super::super::exit::exit_code_from_status;
use super::coordinator::{
    ChildToken, FlagToken, HandlerGuard, InstallRefcount, InterruptRegistry, PressAction,
    PressTarget, ProcessHandler,
};
use super::handle::{HandleCloser, OwnedRawHandle};
use super::reasons::{
    CompletionTermination, WatchdogTermination, early_termination_process_outcome,
    watchdog_request_to_early_termination,
};
use super::{INTERRUPT_FEEDBACK_FIRST, INTERRUPT_FEEDBACK_REPEAT, POST_SIGKILL_REAP_TIMEOUT};

/// Everything the console handler needs to terminate one child from a thread
/// that owns neither the [`Child`] nor any borrow of it.
///
/// Handles are carried as raw `isize` rather than `HANDLE` because the
/// registry is shared across threads and the Win32 pointer newtypes carry no
/// thread-safety guarantee. Validity is bounded by [`ChildRegistration`],
/// which deregisters before the owning wait loop returns and therefore before
/// the [`Child`] or the Job handle can be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowsChild {
    /// The only identifier `GenerateConsoleCtrlEvent` accepts.
    process_id: u32,
    process: isize,
    job: isize,
    /// `false` for interactive TUI passthrough, where the child shares this
    /// console and the terminal already delivers Ctrl+C to it.
    own_group: bool,
}

fn registry() -> &'static InterruptRegistry<WindowsChild> {
    static REGISTRY: OnceLock<InterruptRegistry<WindowsChild>> = OnceLock::new();
    REGISTRY.get_or_init(InterruptRegistry::new)
}

fn as_handle(raw: isize) -> windows::Win32::Foundation::HANDLE {
    windows::Win32::Foundation::HANDLE(raw as *mut core::ffi::c_void)
}

struct JobObjectCloser;

impl HandleCloser for JobObjectCloser {
    fn close(raw: isize) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(as_handle(raw));
        }
    }
}

/// Owns one child's Job Object for the lifetime of its wait scope.
///
/// Closing the last handle is what makes `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
/// destroy descendants the provider left behind, so this must close at the end
/// of the step rather than at process exit.
type OwnedJob = OwnedRawHandle<JobObjectCloser>;

/// Emit the Q14 visible-feedback line once per press.
///
/// The Unix ladder emits the identical bytes from its signal handler; both
/// hosts therefore show the same wording. The write is best-effort — a closed
/// stderr must not fail the handler.
/// Write one rung's feedback line straight to stderr.
///
/// This deliberately bypasses [`StreamOutput`], the synchronized render sink the
/// spec's *Reporting Concurrency* section otherwise requires everything to go
/// through, for a reason that is structural rather than a leftover from the Unix
/// port: [`claudine_console_ctrl_handler`] is a context-free
/// `extern "system" fn(u32)` that Windows invokes on its own thread, while the
/// sink is per-run state living behind an `Arc<Mutex<…>>` in the call chain.
/// Reaching it from here would mean parking a global handle to the live run's
/// sink purely so a static byte string can take a lock.
///
/// What that costs is bounded and not a torn line: the payload is a single
/// newline-terminated static slice written in one `write_all`, and `Stderr`
/// locks internally for the call, so the bytes cannot interleave with another
/// writer's. The sink's *cursor bookkeeping* simply does not learn about this
/// line, which can cost one row of alignment in the frame that follows — the
/// same trade the Unix handler makes, where `libc::write` is additionally forced
/// by async-signal-safety.
///
/// Both hosts emit byte-identical bytes; see [`INTERRUPT_FEEDBACK_FIRST`].
fn emit_interrupt_feedback(count: u8) {
    let msg = if count == 1 {
        INTERRUPT_FEEDBACK_FIRST
    } else {
        INTERRUPT_FEEDBACK_REPEAT
    };
    let _ = std::io::Write::write_all(&mut std::io::stderr(), msg);
}

/// Deliver one ladder rung to one child.
fn apply_press(target: &PressTarget<WindowsChild>) {
    use windows::Win32::System::Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent};
    use windows::Win32::System::JobObjects::TerminateJobObject;
    use windows::Win32::System::Threading::TerminateProcess;

    let child = target.child;
    match (target.action, child.own_group) {
        (PressAction::Graceful, true) => {
            // `CTRL_C_EVENT` cannot target a specific group, so Ctrl+Break is
            // the closest "ask nicely" rung Windows offers.
            unsafe {
                let _ = GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child.process_id);
            }
            tracing::info!(
                child_pid = child.process_id,
                "interrupt received; sent CTRL_BREAK_EVENT to child process group",
            );
        }
        (PressAction::Graceful, false) => {
            // Passthrough child shares this console; the terminal already
            // delivered the chord. Only the press count matters here.
        }
        (PressAction::Force, true) => {
            unsafe {
                let _ = TerminateJobObject(as_handle(child.job), 1);
            }
            tracing::warn!(
                child_pid = child.process_id,
                "repeat interrupt; force-terminating Job Object",
            );
        }
        (PressAction::Force, false) => unsafe {
            let _ = TerminateProcess(as_handle(child.process), 1);
        },
    }
}

/// Windows console-control handler, installed once per process.
///
/// Returns `TRUE` for `CTRL_C_EVENT` and `CTRL_BREAK_EVENT` to suppress the
/// default disposition (terminate the whole console process group); termination
/// is driven explicitly so every wrapped child is reached and the sequence's
/// own interrupt flag is set first. Unlike a Unix signal handler this runs on a
/// dedicated thread with the full Rust API available, so the fan-out happens
/// here rather than being polled for by each wait loop.
unsafe extern "system" fn claudine_console_ctrl_handler(ctrl_type: u32) -> windows::core::BOOL {
    use windows::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};

    if ctrl_type == CTRL_C_EVENT || ctrl_type == CTRL_BREAK_EVENT {
        let outcome = registry().record_press();
        emit_interrupt_feedback(outcome.count);
        for target in &outcome.targets {
            apply_press(target);
        }
        // Runs after child fan-out because its forceful rung ends the process:
        // a compose run's second press must not pre-empt the Job terminations
        // above.
        crate::commands::compose::interrupt::on_console_interrupt(outcome.count);
        windows::core::BOOL(1) // TRUE — suppress default
    } else {
        // Close, Logoff, Shutdown — let the default handler decide.
        windows::core::BOOL(0)
    }
}

static CONSOLE_HANDLER_REFCOUNT: InstallRefcount = InstallRefcount::new();

/// Refcounted installation of the process-wide console handler.
///
/// `SetConsoleCtrlHandler` stacks registrations, so installing per wait loop
/// would multiply-count a single chord once a parallel group is running. The
/// handler is installed on the first guard and removed on the last, which also
/// restores the default Ctrl+C disposition once Claudine owns no children.
///
/// Holders are not only wait loops: a sequence run
/// ([`register_sequence_interrupt_flag`]) and a compose run
/// ([`register_compose_interrupt_handler`]) hold one for their whole duration,
/// which is what gives both a press producer before any child exists.
struct ConsoleHandler;

impl ProcessHandler for ConsoleHandler {
    fn refcount() -> &'static InstallRefcount {
        &CONSOLE_HANDLER_REFCOUNT
    }

    fn install() {
        use windows::Win32::System::Console::SetConsoleCtrlHandler;
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(claudine_console_ctrl_handler), true);
        }
    }

    fn remove() {
        use windows::Win32::System::Console::SetConsoleCtrlHandler;
        unsafe {
            let _ = SetConsoleCtrlHandler(Some(claudine_console_ctrl_handler), false);
        }
    }
}

type ConsoleHandlerGuard = HandlerGuard<ConsoleHandler>;

/// Deregisters one child from the process-scoped registry on every exit path.
struct ChildRegistration {
    token: ChildToken,
    _handler: ConsoleHandlerGuard,
}

impl ChildRegistration {
    fn new(child: WindowsChild) -> Self {
        let handler = ConsoleHandlerGuard::acquire();
        Self {
            token: registry().register_child(child),
            _handler: handler,
        }
    }

    fn presses(&self) -> u8 {
        registry().child_presses(self.token)
    }
}

impl Drop for ChildRegistration {
    fn drop(&mut self) {
        registry().deregister_child(self.token);
    }
}

/// Keeps a sequence's shared `interrupted` flag wired to the console handler.
///
/// This is the Windows producer for the flag the serial loop checks at every
/// step boundary and every shell task polls while running — the counterpart of
/// the Unix `signal_hook` SIGINT registration. Without it a Windows Ctrl+C can
/// stop the current child but cannot stop the *sequence*.
pub(crate) struct SequenceInterruptGuard {
    token: FlagToken,
    _handler: ConsoleHandlerGuard,
}

impl Drop for SequenceInterruptGuard {
    fn drop(&mut self) {
        registry().deregister_flag(self.token);
    }
}

/// Register a sequence's shared interrupt flag with the process coordinator.
pub(crate) fn register_sequence_interrupt_flag(flag: &Arc<AtomicBool>) -> SequenceInterruptGuard {
    let handler = ConsoleHandlerGuard::acquire();
    SequenceInterruptGuard {
        token: registry().register_flag(flag),
        _handler: handler,
    }
}

/// Keeps the console handler installed for a compose run's whole duration.
///
/// A compose run owns no child during prep, so without this the handler would
/// not be installed until the first provider child spawned and a Ctrl+C during
/// the (slow) prep window would take the console's default disposition. The
/// compose ladder itself is driven from
/// [`crate::commands::compose::interrupt::on_console_interrupt`]; this type
/// carries only the installation.
pub(crate) struct ComposeInterruptHandlerGuard {
    _handler: ConsoleHandlerGuard,
}

/// Install (or join) the process-wide console handler on behalf of a compose run.
pub(crate) fn register_compose_interrupt_handler() -> ComposeInterruptHandlerGuard {
    ComposeInterruptHandlerGuard {
        _handler: ConsoleHandlerGuard::acquire(),
    }
}

/// Wait for the child, tracking console interrupts for correct termination
/// labeling (Windows counterpart of the Unix `wait_with_signal_handling`).
///
/// This path is selected whenever the semantic spawn attaches neither an
/// early-termination receiver nor a watchdog. It runs the same loop as the
/// channel-driven waits with those channels absent, because a plain
/// `child.wait()` could neither observe a press nor report
/// [`Interrupted`](claudine::harness::ProcessTermination::Interrupted) — and
/// the child sits in its own Windows process group, so the terminal chord does
/// not reach it incidentally.
pub(crate) fn wait_with_signal_handling(
    child: &mut Child,
    child_in_own_pgroup: bool,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    // No channel can fire on this path, so no grace deadline is ever armed and
    // the value is inert.
    let (_tx, early_rx) = std::sync::mpsc::channel();
    let (code, termination, _) = windows_wait_loop(
        child,
        child_in_own_pgroup,
        early_rx,
        None,
        None,
        Duration::ZERO,
        false,
    )?;
    Ok((code, termination))
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

/// Real Windows implementation of the unified wait loop with parity to the
/// Unix group-signal/escalation behavior (Q15).
///
/// Spawns the child in `CREATE_NEW_PROCESS_GROUP` (set at the `Command`
/// build site in `spawn/setup.rs`), assigns it to a Job Object with
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` so the whole tree terminates as a
/// unit, and registers itself with the process-scoped coordinator. The
/// escalation ladder mirrors the spec's Windows analog:
///
/// - Press 1 → `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group)`
///   (graceful — Windows has no SIGINT/SIGTERM split, so Ctrl+Break is the
///   closest "ask nicely" rung; `interactive` does not differentiate because
///   Windows lacks a separate first-rung interrupt).
/// - Press 2 → `TerminateJobObject` (forceful, kills the whole tree).
///
/// The ladder is tracked per registration, so one child reaching the forceful
/// rung never suppresses a sibling's graceful rung — the case a parallel
/// sequence group produces.
///
/// For the interactive-TUI passthrough case (`child_in_own_pgroup == false`)
/// the child shares the console and receives Ctrl+C naturally from the
/// terminal; it still registers so the press counter is visible and the
/// ladder's forceful rung can still fire on a second press.
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

    // Mirror the Unix loop: while this loop owns the child's interrupt
    // escalation, the compose-scoped Ctrl+C guard must not force-exit the
    // wrapper. Cleared on every exit path.
    let _wait_loop_active = crate::output::WaitLoopActiveGuard::new();

    use windows::Win32::Foundation::HANDLE;
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

    // Create a Job Object with kill-on-close so descendants the child leaves
    // behind die when this scope ends. `OwnedJob` is taken before the two
    // fallible setup calls below so their `?` cannot leak the Job.
    let job = OwnedJob::new(unsafe { CreateJobObjectW(None, None) }?.0 as isize);
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            as_handle(job.raw()),
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )?;
    }

    // Assign the child to the Job. Per Windows docs, assignment works after
    // spawn as long as the child hasn't spawned descendants yet — true here
    // because we assign immediately after `Command::spawn`.
    if child_in_own_pgroup {
        unsafe { AssignProcessToJobObject(as_handle(job.raw()), child_handle)?; }
    }

    // Declared after `job` so it drops *first*: the console handler may
    // dereference the Job handle from another thread, and deregistering is
    // what bounds that window. Closing the Job before deregistering would
    // hand the handler a closed handle.
    let registration = ChildRegistration::new(WindowsChild {
        process_id: child_process_id,
        process: child_handle.0 as isize,
        job: job.raw(),
        own_group: child_in_own_pgroup,
    });

    let mut early_termination: Option<EarlyTermination> = None;
    let mut grace_deadline: Option<Instant> = None;
    let mut reap_deadline: Option<Instant> = None;
    let mut watchdog_rx = watchdog_rx;
    let mut completion_rx = completion_rx;
    let mut completion_requested = false;
    let poll_interval = Duration::from_millis(75);
    let grace_period = kill_grace;

    loop {
        let presses = registration.presses();

        if let Some(status) = child.try_wait()? {
            let code = if completion_requested {
                0
            } else {
                exit_code_from_status(status)
            };
            let termination = if presses > 0 {
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

        // The handler already delivered the rung; arm the reap watchdog once
        // this child's forceful rung has been sent.
        if presses >= 2 && reap_deadline.is_none() {
            reap_deadline = Some(Instant::now() + POST_SIGKILL_REAP_TIMEOUT);
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
                        let _ = unsafe { TerminateJobObject(as_handle(job.raw()), 1) };
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
                        let _ = unsafe { TerminateJobObject(as_handle(job.raw()), 0) };
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
                        let _ = unsafe { TerminateJobObject(as_handle(job.raw()), 1) };
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
                let _ = unsafe { TerminateJobObject(as_handle(job.raw()), 1) };
                reap_deadline = Some(Instant::now() + POST_SIGKILL_REAP_TIMEOUT);
            }
            grace_deadline = None;
        }

        std::thread::sleep(poll_interval);
    }
}

/// Runtime regressions for the Job Object's ownership contract.
///
/// These need a real Windows kernel — a Job Object, an inherited file handle,
/// and the host's per-process handle table — so they are gated to a Windows
/// host rather than merely to a Windows *target*. The cross-platform half of
/// the same contract (released exactly once, on error paths, before the
/// registration guard) lives in [`super::handle`] and runs everywhere.
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    /// `CREATE_NEW_PROCESS_GROUP` — the flag `spawn/setup.rs` sets in
    /// production, and the precondition for `child_in_own_pgroup == true`.
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    /// Long enough that the descendant's own lifetime can never be mistaken for
    /// a Job kill within the assertion budget below.
    const DESCENDANT_PINGS: u32 = 300;

    fn unique_marker(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "claudine-job-{label}-{}-{}.txt",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ))
    }

    /// Spawn a parent that launches a detached descendant and then exits.
    ///
    /// `start /B` makes the descendant outlive the parent while inheriting the
    /// stdout handle, so the marker file stays open — and therefore
    /// undeletable — for exactly as long as the descendant lives. That turns
    /// "is the descendant gone?" into a filesystem question, which needs no pid
    /// bookkeeping and cannot be answered by a stale process-table entry.
    fn spawn_parent_with_detached_descendant(marker: &Path) -> std::process::Child {
        let stdout = fs::File::create(marker).expect("create marker file");
        let command_line = format!("start /B ping -n {DESCENDANT_PINGS} 127.0.0.1");
        Command::new("cmd.exe")
            .arg("/C")
            .arg(&command_line)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .creation_flags(CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .expect("spawn parent cmd.exe")
    }

    /// Poll until the marker can be deleted, tracking the largest size seen.
    ///
    /// ## Returns
    ///
    /// `(deleted_within_budget, max_bytes_written)`. The byte count is what
    /// proves the descendant actually ran: a descendant that never started
    /// would leave an empty file that deletes immediately.
    fn wait_for_marker_release(marker: &Path, budget: Duration) -> (bool, u64) {
        let deadline = Instant::now() + budget;
        let mut max_len = 0;
        loop {
            if let Ok(meta) = fs::metadata(marker) {
                max_len = max_len.max(meta.len());
            }
            if fs::remove_file(marker).is_ok() {
                return (true, max_len);
            }
            if Instant::now() >= deadline {
                return (false, max_len);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn process_handle_count() -> u32 {
        use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};
        let mut count = 0u32;
        unsafe {
            GetProcessHandleCount(GetCurrentProcess(), &mut count).expect("query handle count");
        }
        count
    }

    /// Finding 4: the Job handle used to be a bare `HANDLE`, so
    /// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` could not fire at the step
    /// boundary and the descendant survived until Claudine itself exited.
    ///
    /// Nothing in this path terminates the Job explicitly — no channel is
    /// attached — so a descendant that dies promptly can only have been killed
    /// by the owned handle closing.
    #[test]
    fn job_close_terminates_a_descendant_when_the_wait_scope_ends() {
        let marker = unique_marker("descendant");
        let mut child = spawn_parent_with_detached_descendant(&marker);

        let (_code, _termination) =
            wait_with_signal_handling(&mut child, true).expect("wait for parent");

        let (released, bytes) = wait_for_marker_release(&marker, Duration::from_secs(15));
        assert!(
            bytes > 0,
            "descendant never wrote to the marker, so the test proved nothing"
        );
        assert!(
            released,
            "descendant outlived the wait scope; kill-on-job-close did not fire"
        );
    }

    /// A long sequence used to leak one Job handle per provider child.
    #[test]
    fn repeated_wait_scopes_do_not_grow_the_process_handle_count() {
        // Warm up so first-use allocations are not attributed to the loop.
        for _ in 0..3 {
            let mut child = Command::new("cmd.exe")
                .args(["/C", "exit 0"])
                .creation_flags(CREATE_NEW_PROCESS_GROUP)
                .spawn()
                .expect("spawn cmd.exe");
            wait_with_signal_handling(&mut child, true).expect("wait");
        }

        let before = process_handle_count();
        const RUNS: u32 = 20;
        for _ in 0..RUNS {
            let mut child = Command::new("cmd.exe")
                .args(["/C", "exit 0"])
                .creation_flags(CREATE_NEW_PROCESS_GROUP)
                .spawn()
                .expect("spawn cmd.exe");
            wait_with_signal_handling(&mut child, true).expect("wait");
        }
        let after = process_handle_count();

        // Slack absorbs unrelated runtime handle churn; the leak this guards
        // against was one handle per run, which RUNS makes unmistakable.
        assert!(
            after <= before + 5,
            "handle count grew from {before} to {after} over {RUNS} wait scopes"
        );
    }
}
