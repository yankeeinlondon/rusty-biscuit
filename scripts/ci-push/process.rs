//! Supervised child processes: the one way `ci-push` starts a program.
//!
//! # Ownership contract
//!
//! Every child [`ChildCommand::run`] starts is owned as a whole process tree,
//! and `run` returns only after that tree is gone: the direct child has been
//! reaped, every descendant still alive has been killed, and the pipe readers
//! have been joined. This holds after a normal exit too, so a gate that
//! backgrounds a straggler cannot keep `ci-push` waiting on a pipe or keep a
//! review worktree's directory busy. It holds on an early error or a panic
//! through `Drop`. It cannot hold after `ci-push` itself is force-killed.
//!
//! - **Unix:** the child starts in a new process group (`process_group(0)`),
//!   so the group id is its pid and the whole tree is one `killpg` away.
//!   Terminal Ctrl-C reaches the foreground group only, which is `ci-push`'s;
//!   the child learns of it from the supervisor, which forwards the received
//!   signal to the group, then `SIGKILL`s the group after a short grace.
//! - **Windows:** the child is created suspended, assigned to a
//!   kill-on-close Job Object, and only then resumed, so it cannot create a
//!   descendant outside the Job. Termination is `TerminateJobObject`; closing
//!   the Job (including on `ci-push`'s own exit) kills what remains. Where
//!   the Job cannot be assigned (a session whose own Job forbids nesting), the
//!   child still runs and termination covers the direct child alone.
//!
//! # Cancellation
//!
//! Signal handlers only record which signal arrived ([`install_cancellation_handlers`]);
//! supervisors poll that record through a [`Cancellation`] and do the
//! cleanup on an ordinary thread. Tests use a detached [`Cancellation::new`]
//! token instead of process-wide signal state.
//!
//! # Environment
//!
//! Every child gets `GIT_TERMINAL_PROMPT=0`, `PYTHONDONTWRITEBYTECODE=1`, and
//! `PYTHONUTF8=1`, set last so no caller can undo them. Removing the
//! repository-pinning `GIT_*` variables and `BISCUIT_CI_PLAN_IN` is opt-in per
//! command, because a child that reviews a specific repository may need them.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Variables that pin Git to one repository. A child that must discover its
/// repository from its working directory runs without them.
pub const REPOSITORY_PINNING_VARIABLES: [&str; 6] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_COMMON_DIR",
    "GIT_PREFIX",
];

/// Set on every child; never prompt for credentials, never write `.pyc` into
/// a reviewed tree, and never encode Python's piped output as a code page.
pub const FIXED_VARIABLES: [(&str, &str); 3] = [
    ("GIT_TERMINAL_PROMPT", "0"),
    ("PYTHONDONTWRITEBYTECODE", "1"),
    ("PYTHONUTF8", "1"),
];

/// The gates' plan input; an inherited value must never select the plan.
pub const PLAN_INPUT_VARIABLE: &str = "BISCUIT_CI_PLAN_IN";

/// How often a supervisor checks its child, deadline, and cancellation.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// How long output readers get to reach end-of-file once the tree is gone.
const READER_GRACE: Duration = Duration::from_millis(500);

/// How long a forwarded signal gets to end the direct child before the whole
/// tree is killed outright.
#[cfg(unix)]
const TERMINATE_GRACE: Duration = Duration::from_secs(1);

/// The most a group kill spends re-killing members forked while it ran. A
/// bound, not a pace: the loop ends as soon as the group has no live member.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
const GROUP_REAP_LIMIT: Duration = Duration::from_millis(250);

/// The catchable signal that cancels a run. Windows Ctrl-C (and Ctrl-Break)
/// is [`CancelSignal::Interrupt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelSignal {
    Interrupt,
    Terminate,
    Hangup,
}

impl CancelSignal {
    /// The conventional `128 + signal number` status: 130, 143, or 129.
    pub fn exit_code(self) -> u8 {
        128 + self.number() as u8
    }

    /// POSIX numbers, identical on every Unix this repository supports; the
    /// Windows handler records Ctrl-C as `SIGINT`'s number.
    fn number(self) -> i32 {
        match self {
            CancelSignal::Hangup => 1,
            CancelSignal::Interrupt => 2,
            CancelSignal::Terminate => 15,
        }
    }

    fn from_number(number: i32) -> Option<Self> {
        match number {
            1 => Some(CancelSignal::Hangup),
            2 => Some(CancelSignal::Interrupt),
            15 => Some(CancelSignal::Terminate),
            _ => None,
        }
    }
}

/// The first catchable signal this process received; `0` until then. Only
/// signal handlers and [`Cancellation::cancel`] on the process token write it.
static PROCESS_SIGNAL: AtomicI32 = AtomicI32::new(0);

/// Whether a run has been cancelled. [`Cancellation::process`] reads the
/// state the signal handlers write; [`Cancellation::new`] is an independent
/// token that only [`Cancellation::cancel`] sets.
#[derive(Debug, Clone, Default)]
pub struct Cancellation {
    /// `None` is the process-wide state.
    token: Option<Arc<AtomicI32>>,
}

impl Cancellation {
    pub fn process() -> Self {
        Self { token: None }
    }

    pub fn new() -> Self {
        Self {
            token: Some(Arc::new(AtomicI32::new(0))),
        }
    }

    /// Records `signal` unless a signal was already recorded; the first one
    /// decides the exit code.
    pub fn cancel(&self, signal: CancelSignal) {
        let _ =
            self.state()
                .compare_exchange(0, signal.number(), Ordering::SeqCst, Ordering::SeqCst);
    }

    pub fn requested(&self) -> Option<CancelSignal> {
        CancelSignal::from_number(self.state().load(Ordering::SeqCst))
    }

    fn state(&self) -> &AtomicI32 {
        self.token.as_deref().unwrap_or(&PROCESS_SIGNAL)
    }
}

/// The exit code the process should end with because it was cancelled, or
/// `None` when no handled signal has arrived.
pub fn exit_code_for_cancellation() -> Option<u8> {
    Cancellation::process()
        .requested()
        .map(CancelSignal::exit_code)
}

/// Routes `SIGINT`, `SIGTERM`, and `SIGHUP` (Unix) or Ctrl-C (Windows) into
/// the process-wide [`Cancellation`] instead of ending the process. Call once,
/// before the first child starts.
///
/// A Unix signal the process inherited as ignored stays ignored, as a shell
/// would leave it (`nohup` ignores `SIGHUP`).
///
/// # Errors
///
/// The OS refused to install a handler (or, on Windows, one was already set).
#[cfg(unix)]
pub fn install_cancellation_handlers() -> io::Result<()> {
    extern "C" fn record(signal: libc::c_int) {
        // Only an atomic store: anything else is not async-signal-safe.
        let _ = PROCESS_SIGNAL.compare_exchange(0, signal, Ordering::SeqCst, Ordering::SeqCst);
    }

    for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
        // SAFETY: the structs are zero-initialized C data filled before use;
        // `record` touches nothing but an atomic.
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = record as extern "C" fn(libc::c_int) as libc::sighandler_t;
            action.sa_flags = libc::SA_RESTART;
            libc::sigemptyset(&mut action.sa_mask);
            let mut previous: libc::sigaction = std::mem::zeroed();
            if libc::sigaction(signal, &action, &mut previous) != 0 {
                return Err(io::Error::last_os_error());
            }
            if previous.sa_sigaction == libc::SIG_IGN
                && libc::sigaction(signal, &previous, std::ptr::null_mut()) != 0
            {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

/// See the Unix twin.
#[cfg(windows)]
pub fn install_cancellation_handlers() -> io::Result<()> {
    ctrlc::set_handler(|| Cancellation::process().cancel(CancelSignal::Interrupt))
        .map_err(io::Error::other)
}

/// Whether a child that exited with `status` was interrupted rather than
/// failing on its own: killed by a signal, or a status of 128 or more (the
/// shell's `128 + signal`, and every Windows NTSTATUS-style exit code).
pub fn is_interruption(status: ExitStatus) -> bool {
    match status.code() {
        Some(code) => code as u32 >= 128,
        None => true,
    }
}

/// How a supervised child ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    Exited(ExitStatus),
    /// The deadline passed; the tree was killed.
    TimedOut,
    /// Cancellation was requested; the tree was killed, or the child was
    /// never started because cancellation came first.
    Cancelled(CancelSignal),
}

/// A finished child and the output it was captured with. Inherited output is
/// not captured, and leaves both buffers empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    pub termination: Termination,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// A capture is partial: a stream exceeded the capture limit (the excess
    /// was read and discarded), or a process outside the owned tree still held
    /// a pipe open after the tree ended.
    pub incomplete: bool,
}

impl Completed {
    pub fn success(&self) -> bool {
        matches!(self.termination, Termination::Exited(status) if status.success())
    }

    /// Timed out, cancelled, or exited as [`is_interruption`] describes.
    pub fn interrupted(&self) -> bool {
        match self.termination {
            Termination::Exited(status) => is_interruption(status),
            Termination::TimedOut | Termination::Cancelled(_) => true,
        }
    }
}

/// A child that could not be started or supervised.
#[derive(Debug)]
pub struct ProcessError {
    pub program: OsString,
    pub source: io::Error,
}

impl ProcessError {
    pub fn kind(&self) -> io::ErrorKind {
        self.source.kind()
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot run {}: {}",
            Path::new(&self.program).display(),
            self.source
        )
    }
}

impl std::error::Error for ProcessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug, Clone)]
enum EnvOp {
    Set(OsString, OsString),
    Remove(OsString),
}

#[derive(Debug, Clone, Copy)]
enum OutputMode {
    Capture { limit: usize },
    Inherit,
}

/// A native program and argv to run under supervision. Arguments are separate
/// values; nothing is ever parsed by a shell.
#[derive(Debug, Clone)]
pub struct ChildCommand {
    program: OsString,
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    env: Vec<EnvOp>,
    output: OutputMode,
    deadline: Option<Duration>,
    cancellation: Cancellation,
}

impl ChildCommand {
    /// Captures both streams without a limit, has no deadline, and observes
    /// the process-wide cancellation.
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            output: OutputMode::Capture { limit: usize::MAX },
            deadline: None,
            cancellation: Cancellation::process(),
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    /// Environment changes apply in call order over the inherited
    /// environment, before [`FIXED_VARIABLES`].
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push(EnvOp::Set(key.into(), value.into()));
        self
    }

    pub fn env_remove(mut self, key: impl Into<OsString>) -> Self {
        self.env.push(EnvOp::Remove(key.into()));
        self
    }

    /// Removes [`REPOSITORY_PINNING_VARIABLES`] at this point in the call
    /// order, so the child finds its repository from its working directory.
    pub fn repository_independent(mut self) -> Self {
        self.env.extend(
            REPOSITORY_PINNING_VARIABLES
                .iter()
                .map(|name| EnvOp::Remove(name.into())),
        );
        self
    }

    /// Removes an inherited [`PLAN_INPUT_VARIABLE`] at this point in the call
    /// order.
    pub fn clear_plan_input(mut self) -> Self {
        self.env.push(EnvOp::Remove(PLAN_INPUT_VARIABLE.into()));
        self
    }

    /// The child writes to this process's stdout and stderr (a gate's output
    /// reaches the developer as it happens).
    pub fn inherit_output(mut self) -> Self {
        self.output = OutputMode::Inherit;
        self
    }

    /// Keeps at most `bytes` of each captured stream; the rest is still read,
    /// so the child never blocks on a full pipe.
    pub fn capture_limit(mut self, bytes: usize) -> Self {
        self.output = OutputMode::Capture { limit: bytes };
        self
    }

    /// Kills the tree once `deadline` has passed since the spawn.
    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn cancellation(mut self, cancellation: Cancellation) -> Self {
        self.cancellation = cancellation;
        self
    }

    pub fn program(&self) -> &OsStr {
        &self.program
    }

    pub fn get_args(&self) -> &[OsString] {
        &self.args
    }

    /// Runs the child to completion under the module's ownership contract.
    ///
    /// # Errors
    ///
    /// The program could not be started (an [`io::ErrorKind::NotFound`] kind
    /// means it does not exist), its tree could not be owned, or waiting on
    /// it or reading its output failed. The tree is gone in every case.
    pub fn run(self) -> Result<Completed, ProcessError> {
        let error = |source: io::Error| ProcessError {
            program: self.program.clone(),
            source,
        };
        if let Some(signal) = self.cancellation.requested() {
            return Ok(Completed {
                termination: Termination::Cancelled(signal),
                stdout: Vec::new(),
                stderr: Vec::new(),
                incomplete: false,
            });
        }

        let mut command = self.build();
        let child = command.spawn().map_err(error)?;
        let mut owned = Owned::new(child).map_err(error)?;
        let readers = match self.output {
            OutputMode::Capture { limit } => Some([
                drain(owned.child.stdout.take(), limit),
                drain(owned.child.stderr.take(), limit),
            ]),
            OutputMode::Inherit => None,
        };

        let started = Instant::now();
        let termination = loop {
            match owned.child.try_wait() {
                Ok(Some(status)) => break Termination::Exited(status),
                Ok(None) => {}
                Err(source) => return Err(error(source)),
            }
            if let Some(signal) = self.cancellation.requested() {
                owned.terminate(Some(signal));
                break Termination::Cancelled(signal);
            }
            if self
                .deadline
                .is_some_and(|deadline| started.elapsed() >= deadline)
            {
                owned.terminate(None);
                break Termination::TimedOut;
            }
            std::thread::sleep(POLL_INTERVAL);
        };
        owned.finish();

        let [stdout, stderr] = match readers {
            Some(readers) => collect(readers).map_err(error)?,
            None => [Captured::default(), Captured::default()],
        };
        Ok(Completed {
            termination,
            incomplete: stdout.incomplete || stderr.incomplete,
            stdout: stdout.bytes,
            stderr: stderr.bytes,
        })
    }

    fn build(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args).stdin(Stdio::null());
        match self.output {
            OutputMode::Capture { .. } => command.stdout(Stdio::piped()).stderr(Stdio::piped()),
            OutputMode::Inherit => command.stdout(Stdio::inherit()).stderr(Stdio::inherit()),
        };
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        for op in &self.env {
            match op {
                EnvOp::Set(key, value) => command.env(key, value),
                EnvOp::Remove(key) => command.env_remove(key),
            };
        }
        for (key, value) in FIXED_VARIABLES {
            command.env(key, value);
        }
        isolate(&mut command);
        command
    }
}

#[derive(Debug, Default)]
struct Captured {
    bytes: Vec<u8>,
    incomplete: bool,
}

/// One pipe's reader thread and what it has captured so far.
struct Reader {
    thread: JoinHandle<io::Result<()>>,
    captured: Arc<Mutex<Captured>>,
}

fn drain<R: Read + Send + 'static>(source: Option<R>, limit: usize) -> Reader {
    let captured = Arc::new(Mutex::new(Captured::default()));
    let shared = Arc::clone(&captured);
    let thread = std::thread::spawn(move || {
        let Some(mut source) = source else {
            return Ok(());
        };
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = match source.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let mut captured = shared.lock().unwrap_or_else(PoisonError::into_inner);
            let room = limit.saturating_sub(captured.bytes.len());
            captured.bytes.extend_from_slice(&buffer[..read.min(room)]);
            captured.incomplete |= read > room;
        }
    });
    Reader { thread, captured }
}

/// Joins both readers once the tree is gone. A reader still blocked after
/// [`READER_GRACE`] is held open by a process outside the owned tree (one
/// that left its process group or Job); it is abandoned rather than waited
/// for, and its capture is marked incomplete.
fn collect(readers: [Reader; 2]) -> io::Result<[Captured; 2]> {
    let stop = Instant::now() + READER_GRACE;
    while Instant::now() < stop && !readers.iter().all(|reader| reader.thread.is_finished()) {
        std::thread::sleep(POLL_INTERVAL);
    }
    let mut collected = [Captured::default(), Captured::default()];
    for (reader, slot) in readers.into_iter().zip(&mut collected) {
        let finished = reader.thread.is_finished();
        if finished {
            reader
                .thread
                .join()
                .unwrap_or_else(|_| Err(io::Error::other("an output reader panicked")))?;
        }
        *slot = std::mem::take(
            &mut *reader
                .captured
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        slot.incomplete |= !finished;
    }
    Ok(collected)
}

/// A running child and its tree. Dropping it kills the tree and reaps the
/// child, which is what ends the tree on an early error or a panic.
struct Owned {
    child: Child,
    tree: Tree,
    /// Once reaped, the group id may be reused, so the tree is never
    /// signalled again.
    finished: bool,
}

impl Owned {
    fn new(mut child: Child) -> io::Result<Self> {
        match Tree::own(&child) {
            Ok(tree) => Ok(Self {
                child,
                tree,
                finished: false,
            }),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(error)
            }
        }
    }

    /// Ends the still-running tree: on Unix `signal` (or `SIGTERM` for a
    /// deadline) first, with a grace for the direct child to exit.
    fn terminate(&mut self, signal: Option<CancelSignal>) {
        self.tree.terminate(&mut self.child, signal);
    }

    /// Kills whatever is left of the tree and reaps the direct child.
    fn finish(&mut self) {
        if !self.finished {
            self.tree.kill_all(&mut self.child);
            let _ = self.child.wait();
            self.finished = true;
        }
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(unix)]
fn isolate(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(unix)]
struct Tree {
    /// Equal to the direct child's pid, per `process_group(0)`.
    pgid: libc::pid_t,
}

#[cfg(unix)]
impl Tree {
    fn own(child: &Child) -> io::Result<Self> {
        let pgid = libc::pid_t::try_from(child.id()).map_err(io::Error::other)?;
        Ok(Self { pgid })
    }

    fn terminate(&self, child: &mut Child, signal: Option<CancelSignal>) {
        let number = match signal {
            Some(signal) => signal.number(),
            None => libc::SIGTERM,
        };
        // SAFETY: the group contains only this child's descendants.
        unsafe {
            libc::killpg(self.pgid, number);
        }
        let stop = Instant::now() + TERMINATE_GRACE;
        while Instant::now() < stop && matches!(child.try_wait(), Ok(None)) {
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    fn kill_all(&self, child: &mut Child) {
        // SAFETY: as in `terminate`; an empty group is an `ESRCH` no-op.
        unsafe {
            libc::killpg(self.pgid, libc::SIGKILL);
        }
        // The direct child too, so the reap that follows can never block.
        let _ = child.kill();
        // WHY repeat: macOS does not make `fork` atomic with a group signal,
        // so a child forked while the kill went out joins the group without
        // it. Repeat until the group answers with an error (`ESRCH` when
        // empty, `EPERM` when only zombies remain). Linux delivers to such a
        // child and answers `0` for a zombie-only group, so it must not loop.
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            let stop = Instant::now() + GROUP_REAP_LIMIT;
            // SAFETY: as above.
            while unsafe { libc::killpg(self.pgid, libc::SIGKILL) } == 0 && Instant::now() < stop {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

#[cfg(windows)]
fn isolate(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED};

    // Suspended so `Tree::own` assigns the Job before the child runs a single
    // instruction; a new process group so the console's Ctrl-C reaches only
    // `ci-push`, which then terminates the Job, as on Unix.
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_SUSPENDED);
}

#[cfg(windows)]
struct Tree {
    /// The kill-on-close Job, or null when the child could not be assigned.
    job: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl Tree {
    fn own(child: &Child) -> io::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        // SAFETY: plain Win32 calls on handles this function owns; `info`
        // outlives the call that reads it.
        let job = unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let limited = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) != 0;
            if limited && AssignProcessToJobObject(job, child.as_raw_handle()) != 0 {
                job
            } else {
                // A Job that forbids nesting (an SSH session's) refuses the
                // assignment; the child is then owned alone.
                CloseHandle(job);
                std::ptr::null_mut()
            }
        };
        let tree = Self { job };
        resume_main_thread(child.id())?;
        Ok(tree)
    }

    fn terminate(&self, child: &mut Child, _signal: Option<CancelSignal>) {
        self.kill_all(child);
    }

    fn kill_all(&self, child: &mut Child) {
        if self.job.is_null() {
            let _ = child.kill();
        } else {
            // SAFETY: `job` is a Job handle this tree owns until `drop`.
            unsafe {
                windows_sys::Win32::System::JobObjects::TerminateJobObject(self.job, 1);
            }
        }
    }
}

#[cfg(windows)]
impl Drop for Tree {
    fn drop(&mut self) {
        if !self.job.is_null() {
            // SAFETY: closed exactly once; kill-on-close ends any member left.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.job);
            }
        }
    }
}

/// Resumes the only thread of a process created suspended.
///
/// The Tool Help functions are declared here because `windows-sys`'s
/// `Win32_System_Diagnostics_ToolHelp` feature is not enabled for this
/// package; they are stable, documented `kernel32` exports.
#[cfg(windows)]
fn resume_main_thread(pid: u32) -> io::Result<()> {
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    #[repr(C)]
    struct ThreadEntry32 {
        size: u32,
        usage: u32,
        thread_id: u32,
        owner_process_id: u32,
        base_priority: i32,
        delta_priority: i32,
        flags: u32,
    }
    const TH32CS_SNAPTHREAD: u32 = 0x0000_0004;
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> HANDLE;
        fn Thread32First(snapshot: HANDLE, entry: *mut ThreadEntry32) -> BOOL;
        fn Thread32Next(snapshot: HANDLE, entry: *mut ThreadEntry32) -> BOOL;
    }

    // SAFETY: the snapshot and thread handles are closed on every path;
    // `entry.size` is set as the API requires.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut entry: ThreadEntry32 = std::mem::zeroed();
        entry.size = std::mem::size_of::<ThreadEntry32>() as u32;
        let mut more = Thread32First(snapshot, &mut entry) != 0;
        let mut outcome = Err(io::Error::other(format!(
            "no thread of suspended child {pid} to resume"
        )));
        while more {
            if entry.owner_process_id == pid {
                let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.thread_id);
                outcome = if thread.is_null() {
                    Err(io::Error::last_os_error())
                } else {
                    // `u32::MAX` is `ResumeThread`'s failure sentinel.
                    let resumed = ResumeThread(thread);
                    let result = if resumed == u32::MAX {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(())
                    };
                    CloseHandle(thread);
                    result
                };
                break;
            }
            more = Thread32Next(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        outcome
    }
}
