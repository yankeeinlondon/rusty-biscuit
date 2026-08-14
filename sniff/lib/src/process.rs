//! Bounded subprocess execution.
//!
//! Every child process sniff spawns goes through [`run_with_timeout`]. It is the
//! single place that owns the deadline, the pipe draining, and process-tree
//! termination/reaping, so a wedged or verbose child can never wedge a detection.
//!
//! See `sniff/features/2026-07-16-performance/phases/_completed/06-remote-network-and-subprocess/spec.md`
//! for the contract this module implements.
//!
//! ## What tree termination guarantees
//!
//! Windows containment is total: the child is assigned to a kill-on-close Job
//! Object while still suspended, membership is inherited by every descendant,
//! and the kernel enforces it. Nothing can escape.
//!
//! Unix has no equivalent primitive, so the guarantee is layered and the top
//! layer is best-effort:
//!
//! 1. **Guaranteed** — every process in the child's process group dies, whether
//!    or not it is still parented to the child. `kill(-pgid)` is atomic with
//!    respect to forks: a process forked after the signal is sent inherits a
//!    dead parent's group and is signaled too.
//! 2. **Best-effort** — a descendant that calls `setsid()` leaves that group.
//!    Sniff keeps such a process addressable by sampling the child's descendant
//!    tree on a coarse interval while the child is alive, so its PID survives
//!    the reparenting that follows the child's exit. PIDs are re-validated by
//!    process start time before signaling, so a recycled PID is never hit.
//!
//! The unclosable gap is a descendant that both forks *and* calls `setsid()`
//! entirely between two samples, whose parent then exits. Closing it portably
//! would need Linux cgroups or `PR_SET_CHILD_SUBREAPER` (Linux-only, and
//! process-global — not a library's to set), or a supervising process.
//!
//! Who reaches that gap depends on what is being run. Sniff's own *detection*
//! probes are a fixed, in-tree set of well-known commands, none of which
//! daemonize. The *installation* boundary
//! ([`crate::programs::install`]) is different in kind: it executes third-party
//! package managers (Brew, npm, pip, Cargo, Go) and downloaded remote shell
//! installers, whose lifecycle and build hooks are outside sniff's control and
//! may fork and detach. So on Unix a timed-out installation may leave a
//! detached descendant running — and still modifying the host — after sniff has
//! reported the timeout. Installs surface this through
//! [`crate::programs::install::InstallCapturedResult::timed_out`].
//!
//! `tests::a_descendant_that_detaches_between_samples_escapes_containment` is
//! the executable record of this residual: it is the assertion that flips if a
//! future change ever closes the gap. It manufactures the escape through
//! [`sample_hook`], a test-only callback on the sampler, so the window it
//! exploits is an interval boundary by construction and not by timing estimate.

use std::ffi::OsStr;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tracing::warn;

use crate::performance::{self, counters};

/// Named per-probe deadlines.
///
/// These are policy. Changing one changes what sniff promises a caller about how
/// long a detection can block — treat an edit here as a policy change, never as
/// an incidental refactor.
pub(crate) mod timeouts {
    use std::time::Duration;

    /// Service-manager listing and enrichment commands.
    pub(crate) const SERVICE_COMMAND: Duration = Duration::from_secs(3);

    /// Windows `powershell` locale fallback.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) const WINDOWS_LOCALE: Duration = Duration::from_secs(3);

    /// Windows PowerShell audio-device probe.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) const WINDOWS_AUDIO: Duration = Duration::from_secs(5);

    /// Windows default-route probe.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) const WINDOWS_DEFAULT_ROUTE: Duration = Duration::from_secs(3);

    /// Windows timezone probe.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) const WINDOWS_TIMEZONE: Duration = Duration::from_secs(3);

    /// Windows BurntToast PowerShell module probe.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) const WINDOWS_BURNTTOAST: Duration = Duration::from_secs(3);

    /// macOS `diskutil info`.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) const DISKUTIL: Duration = Duration::from_secs(5);

    /// Install-plan host verification probes.
    pub(crate) const HOST_CAPABILITY: Duration = Duration::from_secs(2);

    /// Program `--version` probes.
    pub(crate) const PROGRAM_SCHEMA: Duration = Duration::from_secs(3);

    /// Explicit remote-tracking refresh (`git fetch`).
    pub(crate) const REMOTE_REFRESH: Duration = Duration::from_secs(30);

    /// NTP status queries.
    pub(crate) const NTP: Duration = Duration::from_secs(3);
}

/// How often the deadline is checked while the child runs.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// How long a child must live before sniff starts recording its descendants.
///
/// Sampling exists to keep a `setsid` descendant addressable after the direct
/// child exits: once the kernel reparents it, nothing links it back to the child
/// PID and nothing keeps it in the original process group. Deferring the first
/// sample by a whole interval is what keeps the common case free — a probe that
/// completes inside one interval never scans the process table at all.
#[cfg_attr(target_os = "windows", allow(dead_code))]
const DESCENDANT_SAMPLE_INTERVAL: Duration = Duration::from_millis(250);

/// Test-only observation point, invoked after every completed descendant sample.
///
/// Sampling is the only thing standing between a `setsid` descendant and the
/// containment gap this module documents, so a test that pins that gap has to
/// know precisely when a sample finished. A hook installed here runs
/// synchronously on the supervising thread, which means a test that *blocks*
/// inside it holds the sampler still for exactly as long as it blocks — the
/// escape it manufactures then lands strictly between two samples on any host
/// at any load, with no timing estimate involved.
///
/// The slot is thread-local rather than global because [`ProcessTree::sample`]
/// always runs on the thread that called [`run_command_with_timeout`]; an
/// installed hook therefore cannot leak into a concurrently running test.
#[cfg(all(unix, test))]
mod sample_hook {
    use std::cell::RefCell;

    thread_local! {
        static HOOK: RefCell<Option<Box<dyn FnMut()>>> = const { RefCell::new(None) };
    }

    /// Uninstalls the hook when dropped, so a panicking test cannot leave one
    /// armed for whatever runs next on this thread.
    pub(super) struct HookGuard(());

    impl Drop for HookGuard {
        fn drop(&mut self) {
            HOOK.with(|slot| *slot.borrow_mut() = None);
        }
    }

    pub(super) fn install(hook: impl FnMut() + 'static) -> HookGuard {
        HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
        HookGuard(())
    }

    pub(super) fn after_sample() {
        // Moved out of the slot for the duration of the call so that a hook
        // which touches the slot itself cannot trip a re-entrant borrow panic.
        let Some(mut hook) = HOOK.with(|slot| slot.borrow_mut().take()) else {
            return;
        };
        hook();
        HOOK.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_none() {
                *slot = Some(hook);
            }
        });
    }
}

#[cfg(unix)]
mod pipe_reader {
    use std::io::Read;
    use std::os::fd::AsRawFd;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    pub(super) struct DrainControl {
        active: Arc<AtomicBool>,
    }

    impl DrainControl {
        pub(super) fn new() -> Self {
            Self {
                active: Arc::new(AtomicBool::new(true)),
            }
        }

        pub(super) fn configure<R>(&self, _pipe: &R) -> std::io::Result<()> {
            Ok(())
        }

        pub(super) fn spawn<R>(&self, mut pipe: R) -> std::thread::JoinHandle<Vec<u8>>
        where
            R: AsRawFd + Read + Send + 'static,
        {
            let active = Arc::clone(&self.active);
            std::thread::spawn(move || {
                let mut output = Vec::new();
                let mut chunk = [0_u8; 8192];
                let fd = pipe.as_raw_fd();
                let mut cleanup_deadline = None;
                loop {
                    if !active.load(Ordering::Relaxed) {
                        let deadline = cleanup_deadline
                            .get_or_insert_with(|| Instant::now() + Duration::from_millis(50));
                        if Instant::now() >= *deadline {
                            break;
                        }
                    }
                    let mut poll_fd = libc::pollfd {
                        fd,
                        events: libc::POLLIN,
                        revents: 0,
                    };
                    // SAFETY: `poll_fd` points to one valid entry and `fd`
                    // remains owned by `pipe` for the lifetime of this thread.
                    let ready = unsafe { libc::poll(&mut poll_fd, 1, 10) };
                    if ready == 0 {
                        if cleanup_deadline.is_some() {
                            break;
                        }
                        continue;
                    }
                    if ready == -1 {
                        if std::io::Error::last_os_error().kind()
                            == std::io::ErrorKind::Interrupted
                        {
                            continue;
                        }
                        break;
                    }
                    match pipe.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(read) => output.extend_from_slice(&chunk[..read]),
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(_) => break,
                    }
                }
                output
            })
        }

        pub(super) fn finish(&self) {
            self.active.store(false, Ordering::Relaxed);
        }
    }
}

#[cfg(target_os = "windows")]
mod pipe_reader {
    use std::io::Read;

    pub(super) struct DrainControl;

    impl DrainControl {
        pub(super) fn new() -> Self {
            Self
        }

        pub(super) fn configure<R>(&self, _pipe: &R) -> std::io::Result<()> {
            Ok(())
        }

        pub(super) fn spawn<R>(&self, mut pipe: R) -> std::thread::JoinHandle<Vec<u8>>
        where
            R: Read + Send + 'static,
        {
            std::thread::spawn(move || {
                let mut output = Vec::new();
                let _ = pipe.read_to_end(&mut output);
                output
            })
        }

        pub(super) fn finish(&self) {}
    }
}

#[cfg(unix)]
mod process_tree {
    use std::collections::HashMap;
    use std::process::{Child, Command};

    use std::os::unix::process::CommandExt;
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

    pub(super) struct ProcessTree {
        process_group: libc::pid_t,
        /// Descendants seen while the direct child was alive, each paired with
        /// the start time identifying that incarnation. Retaining them is what
        /// survives a `setsid` descendant being reparented away from the child:
        /// after reparenting, neither the parent chain nor the process group
        /// names it any more, so a snapshot taken at cleanup cannot find it.
        observed: HashMap<Pid, u64>,
        system: Option<System>,
    }

    pub(super) fn configure(command: &mut Command) {
        command.process_group(0);
    }

    impl ProcessTree {
        pub(super) fn attach(child: &Child) -> std::io::Result<Self> {
            let process_group = libc::pid_t::try_from(child.id()).map_err(|_| {
                std::io::Error::other("child process ID does not fit the platform pid_t")
            })?;
            Ok(Self {
                process_group,
                observed: HashMap::new(),
                system: None,
            })
        }

        /// Records the child's current descendant tree.
        ///
        /// Called on a coarse interval while the child runs, never on the path a
        /// short-lived child takes.
        pub(super) fn sample(&mut self) {
            let roots = self.roots();
            let system = self.system.get_or_insert_with(System::new);
            refresh(system);
            for (pid, start_time) in descendants(system, &roots) {
                self.observed.insert(pid, start_time);
            }
            // Compiled out of every non-test build, so the production sampler
            // is unchanged. See `super::sample_hook`.
            #[cfg(test)]
            super::sample_hook::after_sample();
        }

        /// Kills the supervised tree.
        ///
        /// `force_scan` requests a process-table scan even when nothing was ever
        /// observed; callers set it on failure paths, where one scan is cheap
        /// relative to a deadline that has already elapsed. On a clean exit with
        /// no recorded descendants the scan is skipped entirely and only the
        /// process group is signaled.
        pub(super) fn terminate(&mut self, force_scan: bool) -> std::io::Result<()> {
            if force_scan || !self.observed.is_empty() {
                self.kill_recorded_and_current();
            }

            // SAFETY: `configure` creates a fresh process group whose ID is the
            // direct child's PID. A negative PID addresses that group only.
            let result = unsafe { libc::kill(-self.process_group, libc::SIGKILL) };
            if result == 0 {
                return Ok(());
            }

            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                Ok(())
            } else {
                Err(error)
            }
        }

        fn roots(&self) -> Vec<Pid> {
            let mut roots: Vec<Pid> = u32::try_from(self.process_group)
                .map(|root| vec![Pid::from_u32(root)])
                .unwrap_or_default();
            roots.extend(self.observed.keys().copied());
            roots
        }

        fn kill_recorded_and_current(&mut self) {
            let roots = self.roots();
            let system = self.system.get_or_insert_with(System::new);
            refresh(system);

            let mut targets: Vec<Pid> = descendants(system, &roots)
                .into_iter()
                .map(|(pid, _)| pid)
                .collect();
            for (pid, start_time) in &self.observed {
                // A PID the kernel has since recycled belongs to an unrelated
                // process. Matching the recorded start time is what keeps this
                // from signaling it.
                let same_incarnation = system
                    .process(*pid)
                    .is_some_and(|process| process.start_time() == *start_time);
                if same_incarnation && !targets.contains(pid) {
                    targets.push(*pid);
                }
            }

            for target in targets.into_iter().rev() {
                let Ok(target) = libc::pid_t::try_from(target.as_u32()) else {
                    continue;
                };
                // SAFETY: each PID is a live process that this scan either found
                // in the supervised tree or confirmed as the same incarnation
                // recorded while the child was running.
                let _ = unsafe { libc::kill(target, libc::SIGKILL) };
            }

            self.observed.clear();
        }
    }

    fn refresh(system: &mut System) {
        system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
    }

    /// Transitive closure of `roots` over the parent relation, deepest last.
    fn descendants(system: &System, roots: &[Pid]) -> Vec<(Pid, u64)> {
        let mut found: Vec<(Pid, u64)> = Vec::new();
        loop {
            let previous_len = found.len();
            for (&pid, process) in system.processes() {
                if roots.contains(&pid) || found.iter().any(|(seen, _)| *seen == pid) {
                    continue;
                }
                let Some(parent) = process.parent() else {
                    continue;
                };
                if roots.contains(&parent) || found.iter().any(|(seen, _)| *seen == parent) {
                    found.push((pid, process.start_time()));
                }
            }
            if found.len() == previous_len {
                break;
            }
        }
        found
    }
}

#[cfg(target_os = "windows")]
mod process_tree {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command};

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows::Win32::System::Threading::{
        CREATE_SUSPENDED, OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
    };
    use windows::core::Owned;

    pub(super) struct ProcessTree {
        job: Owned<HANDLE>,
    }

    pub(super) fn configure(command: &mut Command) {
        command.creation_flags(CREATE_SUSPENDED.0);
    }

    impl ProcessTree {
        pub(super) fn attach(child: &Child) -> std::io::Result<Self> {
            // The process is still suspended, so it cannot create a descendant
            // before assignment to the job. Every owned raw handle is wrapped
            // immediately and remains valid for each Win32 call below.
            unsafe {
                let job = Owned::new(
                    CreateJobObjectW(None, windows::core::PCWSTR::null())
                        .map_err(std::io::Error::other)?,
                );
                let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    *job,
                    JobObjectExtendedLimitInformation,
                    std::ptr::from_ref(&limits).cast(),
                    std::mem::size_of_val(&limits) as u32,
                )
                .map_err(std::io::Error::other)?;

                let process = HANDLE(child.as_raw_handle());
                AssignProcessToJobObject(*job, process).map_err(std::io::Error::other)?;
                resume_primary_thread(child.id())?;

                Ok(Self { job })
            }
        }

        /// Job Object membership is inherited and kernel-enforced, so Windows
        /// needs no descendant sampling to keep an escaping process addressable.
        pub(super) fn sample(&mut self) {}

        pub(super) fn terminate(&mut self, _force_scan: bool) -> std::io::Result<()> {
            // SAFETY: `job` is an owned, live Job Object handle. Termination is
            // idempotent for the helper's cleanup paths.
            unsafe { TerminateJobObject(*self.job, 1).map_err(std::io::Error::other) }
        }
    }

    unsafe fn resume_primary_thread(process_id: u32) -> std::io::Result<()> {
        // SAFETY: the snapshot and thread handles are owned for this scope. The
        // child was created suspended and has exactly one initial thread before
        // this function resumes it.
        unsafe {
            let snapshot = Owned::new(
                CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
                    .map_err(std::io::Error::other)?,
            );
            let mut entry = THREADENTRY32 {
                dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                ..Default::default()
            };
            Thread32First(*snapshot, &mut entry).map_err(std::io::Error::other)?;

            loop {
                if entry.th32OwnerProcessID == process_id {
                    let thread = Owned::new(
                        OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID)
                            .map_err(std::io::Error::other)?,
                    );
                    if ResumeThread(*thread) == u32::MAX {
                        return Err(std::io::Error::last_os_error());
                    }
                    return Ok(());
                }
                if Thread32Next(*snapshot, &mut entry).is_err() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "suspended child process has no thread",
                    ));
                }
            }
        }
    }
}

/// Captured result of a completed child process.
#[derive(Debug)]
pub(crate) struct CapturedOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

impl CapturedOutput {
    /// Stdout as UTF-8, replacing invalid sequences.
    pub(crate) fn stdout_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }
}

/// Why a bounded child failed to produce output.
#[derive(Debug)]
pub(crate) enum ProcessError {
    /// The executable could not be spawned (missing, not executable, ...).
    Spawn(std::io::Error),
    /// The child exceeded its deadline and was killed.
    Timeout,
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "could not spawn: {e}"),
            Self::Timeout => write!(f, "timed out"),
        }
    }
}

impl std::error::Error for ProcessError {}

impl From<ProcessError> for std::io::Error {
    fn from(err: ProcessError) -> Self {
        match err {
            ProcessError::Spawn(e) => e,
            ProcessError::Timeout => {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "command timed out")
            }
        }
    }
}

/// Runs `program` with `args` under an explicit deadline, capturing both pipes.
///
/// The executable is invoked directly — never through a shell — so no argument is
/// subject to shell interpretation. Stdin is null, so a child that reads input
/// sees EOF rather than blocking on a terminal sniff may not own.
///
/// ## Returns
///
/// The child's [`CapturedOutput`] regardless of exit status; a non-zero exit is a
/// result, not an error, because several callers parse output from a failing
/// command. Use [`CapturedOutput::status`] to discriminate.
///
/// ## Errors
///
/// [`ProcessError::Spawn`] if the child could not start, [`ProcessError::Timeout`]
/// if it exceeded `timeout` (in which case it has been killed and reaped).
///
/// ## Notes
///
/// Both pipes are drained on dedicated threads while the parent polls the
/// deadline. This is the whole point of the helper: a `try_wait()` loop over an
/// undrained `Stdio::piped()` deadlocks any child that writes more than one pipe
/// buffer, because the child blocks in `write()` and so never exits for the loop
/// to observe.
///
/// Tree termination is total on Windows and best-effort on Unix; see the module
/// documentation for exactly what each platform guarantees. A command that
/// deliberately detaches — or a third-party installer that does so on its own —
/// can outlive this boundary on Unix, so a `Timeout` is not proof that every
/// process the command started has stopped.
pub(crate) fn run_with_timeout<S, A>(
    program: S,
    args: &[A],
    timeout: Duration,
) -> Result<CapturedOutput, ProcessError>
where
    S: AsRef<OsStr>,
    A: AsRef<OsStr>,
{
    let mut command = Command::new(program.as_ref());
    command.args(args.iter().map(AsRef::as_ref));
    run_command_with_timeout(&mut command, timeout)
}

/// Runs a configured command under an explicit deadline, capturing both pipes.
///
/// This is the builder-capable form of [`run_with_timeout`]. The caller's
/// executable, arguments, working directory, and environment are preserved;
/// this boundary owns stdin and captured stdout/stderr so it can enforce the
/// same supervision contract for every subprocess.
///
/// ## Notes
///
/// This is the form the installation boundary uses to run third-party package
/// managers and downloaded installer scripts. Tree termination for those is
/// total on Windows and best-effort on Unix; see the module documentation for
/// the exact residual.
pub(crate) fn run_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<CapturedOutput, ProcessError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    process_tree::configure(command);

    let program = command.get_program().to_owned();
    let mut child = command.spawn().map_err(ProcessError::Spawn)?;
    performance::increment_counter(counters::PROC_SPAWNS, 1);
    let mut process_tree = match process_tree::ProcessTree::attach(&child) {
        Ok(process_tree) => process_tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProcessError::Spawn(error));
        }
    };

    // Unix pipe readers use bounded readiness polling because descendant
    // discovery and signaling cannot make pipe EOF atomic. Windows Job Objects
    // provide kernel-enforced containment, so EOF remains authoritative.
    let drain_control = pipe_reader::DrainControl::new();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    if let Err(error) = stdout
        .as_ref()
        .map(|pipe| drain_control.configure(pipe))
        .transpose()
        .and_then(|_| {
            stderr
                .as_ref()
                .map(|pipe| drain_control.configure(pipe))
                .transpose()
        })
    {
        let _ = process_tree.terminate(true);
        let _ = child.wait();
        return Err(ProcessError::Spawn(error));
    }
    let stdout_handle = stdout.map(|pipe| drain_control.spawn(pipe));
    let stderr_handle = stderr.map(|pipe| drain_control.spawn(pipe));

    let start = Instant::now();
    let mut next_sample = DESCENDANT_SAMPLE_INTERVAL;
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    performance::increment_counter(counters::PROC_TIMEOUTS, 1);
                    warn!(
                        program = %program.to_string_lossy(),
                        timeout_ms = timeout.as_millis(),
                        "subprocess exceeded its deadline; terminating supervised processes"
                    );
                    let _ = process_tree.terminate(true);
                    // Reap, so the killed child cannot linger as a zombie.
                    let _ = child.wait();
                    break Err(ProcessError::Timeout);
                }
                if elapsed >= next_sample {
                    process_tree.sample();
                    next_sample = elapsed + DESCENDANT_SAMPLE_INTERVAL;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                let _ = process_tree.terminate(true);
                let _ = child.wait();
                break Err(ProcessError::Spawn(e));
            }
        }
    };

    // The direct child may exit while a descendant still owns an inherited pipe
    // handle, or while one it detached with `setsid` is still running. Terminate
    // the recorded and current descendants and the process group, or the Job
    // Object, before joining readers. Failure paths already terminated, so this
    // call scans only when the child was alive long enough to be sampled.
    let _ = process_tree.terminate(false);
    drain_control.finish();

    // Unix readers drain every byte already available, then stop at the bounded
    // poll interval; they never require EOF from a session-detached descendant.
    // Windows readers observe EOF after Job Object termination. A panic degrades
    // to empty output.
    let stdout = stdout_handle
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();
    let stderr = stderr_handle
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();

    let status = result?;

    Ok(CapturedOutput {
        status,
        stdout,
        stderr,
    })
}

/// Runs a command under a deadline and returns stdout only on a successful exit.
///
/// The common shape for probes that treat any failure — spawn, non-zero exit, or
/// timeout — as "unavailable".
pub(crate) fn run_for_stdout<S, A>(program: S, args: &[A], timeout: Duration) -> Option<String>
where
    S: AsRef<OsStr>,
    A: AsRef<OsStr>,
{
    let out = run_with_timeout(program, args, timeout).ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LARGE_OUTPUT_CHILD: &str = "process::tests::child_writes_large_output";
    const PIPE_HOLDING_CHILD: &str = "process::tests::child_spawns_pipe_holding_descendant";
    const PIPE_HOLDING_DESCENDANT: &str = "process::tests::pipe_holding_descendant";
    const CONFIGURED_CHILD: &str = "process::tests::configured_child";
    #[cfg(unix)]
    const DETACHED_PIPE_HOLDING_CHILD: &str =
        "process::tests::child_spawns_detached_pipe_holding_descendant";
    #[cfg(unix)]
    const DETACHED_PIPE_HOLDING_DESCENDANT: &str =
        "process::tests::detached_pipe_holding_descendant";
    #[cfg(unix)]
    const QUIET_DETACHED_CHILD: &str = "process::tests::child_spawns_quiet_detached_descendant";
    #[cfg(unix)]
    const QUIET_DETACHED_DESCENDANT: &str = "process::tests::quiet_detached_descendant";
    #[cfg(unix)]
    const EXITING_PARENT_CHILD: &str = "process::tests::child_detaches_descendant_then_exits";
    #[cfg(unix)]
    const BETWEEN_SAMPLES_CHILD: &str = "process::tests::child_detaches_between_samples";
    #[cfg(unix)]
    const BETWEEN_SAMPLES_DESCENDANT: &str = "process::tests::between_samples_descendant";
    #[cfg(unix)]
    const DETACHED_PID_FILE: &str = "SNIFF_DETACHED_PID_FILE";
    /// Written by the supervising thread's post-sample hook; releases the
    /// between-samples fixture to fork.
    #[cfg(unix)]
    const BETWEEN_SAMPLES_GO_FILE: &str = "SNIFF_BETWEEN_SAMPLES_GO_FILE";
    /// Written by the between-samples fixture the instant it has a descendant
    /// PID, so the test's cleanup guard is armed before anything can panic.
    #[cfg(unix)]
    const BETWEEN_SAMPLES_PID_FILE: &str = "SNIFF_BETWEEN_SAMPLES_PID_FILE";
    /// Written by the escaped descendant once it observes that it has been
    /// reparented — proof that its direct parent has already exited.
    #[cfg(unix)]
    const BETWEEN_SAMPLES_REPARENT_FILE: &str = "SNIFF_BETWEEN_SAMPLES_REPARENT_FILE";
    const SLEEPING_CHILD: &str = "process::tests::child_sleeps";

    fn test_child_args(name: &str) -> Vec<std::ffi::OsString> {
        [name, "--exact", "--ignored", "--nocapture"]
            .into_iter()
            .map(Into::into)
            .collect()
    }

    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn child_writes_large_output() {
        use std::io::Write;

        let stdout = vec![b'o'; 1_048_576];
        let stderr = vec![b'e'; 1_048_576];
        std::io::stdout().write_all(&stdout).unwrap();
        std::io::stderr().write_all(&stderr).unwrap();
    }

    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn child_sleeps() {
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn configured_child() {
        println!(
            "{}|{}",
            std::env::current_dir().unwrap().display(),
            std::env::var("SNIFF_CONFIGURED_CHILD").unwrap_or_default()
        );
    }

    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    // The descendant outliving this process is the regression under test, so it
    // must not be waited on. No zombie can result: this fixture exits
    // immediately after spawning, and a zombie needs a live parent — on exit
    // the descendant is reparented and reaped by init.
    #[allow(clippy::zombie_processes)]
    fn child_spawns_pipe_holding_descendant() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(PIPE_HOLDING_DESCENDANT);
        std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("pipe-holding descendant should spawn");
    }

    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn pipe_holding_descendant() {
        std::thread::sleep(Duration::from_secs(30));
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn child_spawns_detached_pipe_holding_descendant() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(DETACHED_PIPE_HOLDING_DESCENDANT);
        let mut descendant = std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("detached pipe-holding descendant should spawn");
        let descendant_pid = libc::pid_t::try_from(descendant.id())
            .expect("descendant process ID should fit pid_t");
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            // SAFETY: `descendant_pid` names the child spawned above. A session
            // leader's process-group ID equals its PID.
            if unsafe { libc::getpgid(descendant_pid) } == descendant_pid {
                std::thread::sleep(Duration::from_secs(30));
                // The scenario under test ends with the sleep (in practice the
                // test SIGKILLs this fixture long before); reap the descendant
                // so no path leaves it running or zombied.
                let _ = descendant.kill();
                let _ = descendant.wait();
                return;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        let _ = descendant.kill();
        let _ = descendant.wait();
        panic!("descendant did not establish its own session");
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn detached_pipe_holding_descendant() {
        use std::io::Write;

        // SAFETY: this fixture is a fresh child and is not a process-group
        // leader, so it can establish an isolated session for the regression.
        assert_ne!(unsafe { libc::setsid() }, -1, "setsid should succeed");
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if stdout.write_all(b"o").is_err() || stderr.write_all(b"e").is_err() {
                return;
            }
        }
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn child_spawns_quiet_detached_descendant() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(QUIET_DETACHED_DESCENDANT);
        let mut descendant = std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("quiet detached descendant should spawn");
        std::thread::sleep(Duration::from_secs(30));
        // The scenario under test ends with the sleep (in practice the test
        // SIGKILLs this fixture long before); reap the descendant so no path
        // leaves it running or zombied.
        let _ = descendant.kill();
        let _ = descendant.wait();
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn quiet_detached_descendant() {
        // SAFETY: this fixture is a fresh child and is not a process-group
        // leader, so it can establish an isolated session for the regression.
        assert_ne!(unsafe { libc::setsid() }, -1, "setsid should succeed");
        let pid_file = std::env::var_os(DETACHED_PID_FILE)
            .expect("the parent fixture should provide a PID file");
        std::fs::write(pid_file, std::process::id().to_string())
            .expect("detached descendant PID should be reported");
        std::thread::sleep(Duration::from_secs(30));
    }

    /// Spawns a `setsid` descendant, waits for the detachment to be real, then
    /// exits successfully — leaving the descendant reparented and outside the
    /// supervised process group before sniff reaches its cleanup.
    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    // Waiting on the descendant would defeat the fixture: the reparented
    // survivor IS the regression. No zombie can result: every path out of this
    // function (return or panic) terminates the fixture process while the
    // 30-second descendant is still alive, and a zombie needs a live parent —
    // init reaps the descendant after reparenting.
    #[allow(clippy::zombie_processes)]
    fn child_detaches_descendant_then_exits() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(QUIET_DETACHED_DESCENDANT);
        let descendant = std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("quiet detached descendant should spawn");
        let descendant_pid = libc::pid_t::try_from(descendant.id())
            .expect("descendant process ID should fit pid_t");

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            // SAFETY: `descendant_pid` names the child spawned above. A session
            // leader's process-group ID equals its PID.
            if unsafe { libc::getpgid(descendant_pid) } == descendant_pid {
                // Outlive at least two descendant samples, so the escape is
                // recorded while this process is still its parent.
                std::thread::sleep(DESCENDANT_SAMPLE_INTERVAL * 3);
                return;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        panic!("descendant did not establish its own session");
    }

    /// Reproduces the residual the module documentation admits: a descendant
    /// created *and* detached entirely between two of sniff's samples, whose
    /// parent then exits at once.
    ///
    /// This fixture does not time itself. It waits for the supervising thread
    /// to release it from inside that thread's post-sample hook, which is what
    /// makes the window it forks in an interval boundary by construction rather
    /// than by estimate.
    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    // Waiting on the descendant would defeat the fixture: the parent must exit
    // the instant the descendant's `setsid` lands, leaving it escaped between
    // two sampler intervals. No zombie can result: every path out of this
    // function terminates the fixture process while the descendant is still
    // alive, and a zombie needs a live parent — init reaps it after
    // reparenting.
    #[allow(clippy::zombie_processes)]
    fn child_detaches_between_samples() {
        let go_file = std::path::PathBuf::from(
            std::env::var_os(BETWEEN_SAMPLES_GO_FILE)
                .expect("the test should provide a release marker path"),
        );
        let pid_file = std::env::var_os(BETWEEN_SAMPLES_PID_FILE)
            .expect("the test should provide a PID file path");

        let deadline = Instant::now() + Duration::from_secs(20);
        while !go_file.exists() {
            assert!(
                Instant::now() < deadline,
                "the sampler hook never released this fixture"
            );
            std::thread::sleep(POLL_INTERVAL);
        }

        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(BETWEEN_SAMPLES_DESCENDANT);
        let descendant = std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("between-samples descendant should spawn");
        let descendant_pid = libc::pid_t::try_from(descendant.id())
            .expect("descendant process ID should fit pid_t");
        std::fs::write(pid_file, descendant_pid.to_string())
            .expect("between-samples descendant PID should be reported");

        // Exiting before `setsid` returns would reproduce nothing: until then
        // the descendant is still in the supervised process group, where the
        // guaranteed layer of cleanup reaches it.
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            // SAFETY: `descendant_pid` names the child spawned above. A session
            // leader's process-group ID equals its PID.
            if unsafe { libc::getpgid(descendant_pid) } == descendant_pid {
                return;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        panic!("descendant did not establish its own session");
    }

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn between_samples_descendant() {
        let reparent_file = std::env::var_os(BETWEEN_SAMPLES_REPARENT_FILE)
            .expect("the test should provide a reparent marker path");

        // SAFETY: this fixture is a fresh child and is not a process-group
        // leader, so it can establish an isolated session for the regression.
        assert_ne!(unsafe { libc::setsid() }, -1, "setsid should succeed");

        // `setsid` leaves the parent relation intact, and that relation is what
        // the sampler walks. The escape is only complete once the direct child
        // has exited and the kernel has reparented this process, so publish the
        // marker then and not a moment earlier — the supervising thread blocks
        // on it, and releasing early would let a sample observe this process
        // while it is still reachable.
        // SAFETY: `getppid` takes no arguments and cannot fail.
        let original_parent = unsafe { libc::getppid() };
        let deadline = Instant::now() + Duration::from_secs(20);
        // SAFETY: as above.
        while unsafe { libc::getppid() } == original_parent {
            assert!(
                Instant::now() < deadline,
                "the direct child never exited, so no reparenting occurred"
            );
            std::thread::sleep(POLL_INTERVAL);
        }
        std::fs::write(reparent_file, std::process::id().to_string())
            .expect("reparented descendant should publish its marker");

        std::thread::sleep(Duration::from_secs(30));
    }

    #[cfg(unix)]
    fn read_pid(path: &std::path::Path) -> Option<libc::pid_t> {
        std::fs::read_to_string(path).ok()?.trim().parse().ok()
    }

    /// SIGKILLs an escaped descendant on every exit path, including panics.
    ///
    /// The PID slot is shared with the sampler hook, which fills it as soon as
    /// the fixture publishes one — so the guard is armed before any assertion
    /// runs, not after. The escapee is reparented to init, so it is not ours to
    /// `wait` on, but leaving a 30s sleeper behind on a developer's or CI host
    /// is not acceptable either.
    #[cfg(unix)]
    struct EscapedDescendant(std::sync::Arc<std::sync::atomic::AtomicI32>);

    #[cfg(unix)]
    impl EscapedDescendant {
        fn new() -> Self {
            Self(std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0)))
        }

        fn slot(&self) -> std::sync::Arc<std::sync::atomic::AtomicI32> {
            std::sync::Arc::clone(&self.0)
        }

        fn pid(&self) -> Option<libc::pid_t> {
            match self.0.load(std::sync::atomic::Ordering::SeqCst) {
                0 => None,
                pid => Some(pid),
            }
        }
    }

    #[cfg(unix)]
    impl Drop for EscapedDescendant {
        fn drop(&mut self) {
            let Some(pid) = self.pid() else {
                return;
            };
            // SAFETY: the PID was reported by a fixture this test spawned.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
    }

    /// Encodes the documented Unix containment gap — it is a record of what
    /// sniff currently does, not an endorsement of it.
    ///
    /// A descendant that forks and calls `setsid()` wholly between two samples,
    /// whose parent then exits, is never observed by any sample and is outside
    /// the process group by cleanup time, so nothing names it. The installation
    /// boundary runs third-party package managers and downloaded installer
    /// scripts through this same helper, which makes the outcome reachable
    /// rather than theoretical: sniff can report an install timeout while an
    /// escaped process keeps modifying the host.
    ///
    /// If Unix containment is ever made total, this assertion is what flips —
    /// invert it and update the module documentation together.
    ///
    /// The window is closed on the sampler's own terms rather than guessed at:
    /// the post-sample hook releases the fixture and then blocks until the
    /// escaped descendant reports that it has been reparented, which cannot
    /// happen before the direct child has exited. No sample can run while the
    /// hook is blocked, and the loop's next `try_wait` observes an
    /// already-exited child, so no sample can run afterwards either. The test
    /// therefore reaches a verdict on every run at any host load; there is no
    /// path that reports success without asserting the residual.
    #[cfg(unix)]
    #[test]
    fn a_descendant_that_detaches_between_samples_escapes_containment() {
        let scratch = tempfile::tempdir().expect("temporary marker directory should exist");
        let go_file = scratch.path().join("go");
        let pid_file = scratch.path().join("descendant-pid");
        let reparent_file = scratch.path().join("reparented");

        let escapee = EscapedDescendant::new();

        let pid_slot = escapee.slot();
        let hook_go_file = go_file.clone();
        let hook_pid_file = pid_file.clone();
        let hook_reparent_file = reparent_file.clone();
        let mut released = false;
        let _hook = sample_hook::install(move || {
            if released {
                return;
            }
            released = true;
            std::fs::write(&hook_go_file, b"go").expect("release marker should be writable");

            let deadline = Instant::now() + Duration::from_secs(20);
            while Instant::now() < deadline {
                if let Some(pid) = read_pid(&hook_pid_file) {
                    pid_slot.store(pid, std::sync::atomic::Ordering::SeqCst);
                }
                if hook_reparent_file.exists() {
                    return;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        });

        let executable = std::env::current_exe().expect("current test executable should resolve");
        let mut command = Command::new(executable);
        command
            .args(test_child_args(BETWEEN_SAMPLES_CHILD))
            .env(BETWEEN_SAMPLES_GO_FILE, &go_file)
            .env(BETWEEN_SAMPLES_PID_FILE, &pid_file)
            .env(BETWEEN_SAMPLES_REPARENT_FILE, &reparent_file);

        let start = Instant::now();
        let out = run_command_with_timeout(&mut command, Duration::from_secs(60))
            .expect("the direct child exits successfully, it does not time out");
        let returned_after = start.elapsed();

        assert!(out.status.success(), "the direct child should exit cleanly");
        assert!(
            returned_after < Duration::from_secs(15),
            "an escaped descendant must not delay the helper's return"
        );
        assert!(
            reparent_file.exists(),
            "the descendant should have been reparented before the helper returned"
        );

        let pid = escapee
            .pid()
            .expect("the fixture should report its escaped descendant");

        // SAFETY: signal zero performs existence and permission checks without
        // delivering a signal to the reported process.
        let exists = unsafe { libc::kill(pid, 0) } == 0;
        assert!(
            exists,
            "descendant {pid} was contained; Unix tree termination is now stronger than \
             the module documentation claims — invert this test and correct the docs"
        );
    }

    /// A child that outstrips its pipe buffer must still be captured whole.
    ///
    /// This is the regression the module exists for: the pre-Phase-6 `try_wait()`
    /// loops left stdout undrained, so this child would block in `write()` and be
    /// killed at the deadline with its output lost.
    #[test]
    fn output_larger_than_a_pipe_buffer_does_not_deadlock() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(LARGE_OUTPUT_CHILD);
        let out = run_with_timeout(
            executable,
            &args,
            Duration::from_secs(30),
        )
        .expect("child should complete, not time out");

        assert!(out.status.success());
        assert!(out.stdout.iter().filter(|byte| **byte == b'o').count() >= 1_048_576);
        assert!(out.stderr.iter().filter(|byte| **byte == b'e').count() >= 1_048_576);
    }

    /// Both pipes drain concurrently, so a child filling stderr as well as stdout
    /// cannot wedge on either.
    #[test]
    fn both_pipes_drain_concurrently() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(LARGE_OUTPUT_CHILD);
        let out = run_with_timeout(executable, &args, Duration::from_secs(30))
            .expect("child should complete");

        assert!(out.stdout.len() >= 1_048_576);
        assert!(out.stderr.len() >= 1_048_576);
    }

    #[test]
    fn configured_command_preserves_cwd_and_environment() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(CONFIGURED_CHILD);
        let working_dir = tempfile::tempdir().expect("temporary working directory should exist");
        let mut command = Command::new(executable);
        command
            .args(args)
            .current_dir(working_dir.path())
            .env("SNIFF_CONFIGURED_CHILD", "preserved");

        let output = run_command_with_timeout(&mut command, Duration::from_secs(5))
            .expect("configured child should complete");

        assert!(output.status.success());
        let expected_dir = working_dir.path().canonicalize().unwrap();
        assert!(
            output.stdout_lossy().lines().any(|line| {
                line.strip_suffix("|preserved").is_some_and(|dir| {
                    std::fs::canonicalize(dir).is_ok_and(|d| d == expected_dir)
                })
            }),
            "configured child output should contain the preserved cwd and environment"
        );
    }

    /// Tests inject a short deadline rather than sleeping for a production one.
    #[test]
    fn a_hung_child_is_killed_at_its_deadline() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(SLEEPING_CHILD);
        let start = Instant::now();
        let result = run_with_timeout(executable, &args, Duration::from_millis(200));

        assert!(matches!(result, Err(ProcessError::Timeout)));
        // Generous bound: asserts the deadline was honored, not the scheduler's precision.
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "killed at deadline, not after the child's own 30s"
        );
    }

    /// A descendant can retain inherited stdout/stderr after its direct parent
    /// exits. Tree cleanup must close those handles before the reader joins.
    #[test]
    fn a_pipe_holding_descendant_cannot_extend_the_deadline() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(PIPE_HOLDING_CHILD);
        let start = Instant::now();
        let out = run_with_timeout(executable, &args, Duration::from_millis(200))
            .expect("the direct child exits successfully");

        assert!(out.status.success());
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "descendant-held pipes must not delay reader joins"
        );
    }

    /// A Unix descendant can escape the helper's process group with `setsid`
    /// while retaining both inherited pipes. Reader cleanup must remain bounded
    /// even though Unix has no Job Object equivalent to contain that process.
    #[cfg(unix)]
    #[test]
    fn a_session_detached_descendant_cannot_block_pipe_cleanup() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(DETACHED_PIPE_HOLDING_CHILD);
        let start = Instant::now();
        let result = run_with_timeout(executable, &args, Duration::from_secs(3));

        assert!(matches!(result, Err(ProcessError::Timeout)));
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "session-detached pipe holders must not delay reader cleanup"
        );

        // The original child was reaped even though its detached descendant is
        // no longer addressable through the original process group.
        // SAFETY: WNOHANG only reports already-exited children.
        let rc = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        assert_eq!(rc, -1, "the direct child should not remain waitable");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }

    /// Timeout cleanup must terminate a quiet descendant even after `setsid`
    /// moves it outside the direct child's process group.
    #[cfg(unix)]
    #[test]
    fn a_quiet_session_detached_descendant_is_terminated() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(QUIET_DETACHED_CHILD);
        let pid_file = tempfile::NamedTempFile::new().expect("temporary PID file should exist");
        let mut command = Command::new(executable);
        command.args(args).env(DETACHED_PID_FILE, pid_file.path());

        let result = run_command_with_timeout(&mut command, Duration::from_secs(1));

        assert!(matches!(result, Err(ProcessError::Timeout)));
        let pid = std::fs::read_to_string(pid_file.path())
            .expect("detached descendant should report its PID")
            .parse::<libc::pid_t>()
            .expect("reported descendant PID should be numeric");
        // SAFETY: signal zero performs existence and permission checks without
        // delivering a signal to the reported process.
        let exists = unsafe { libc::kill(pid, 0) } == 0;
        assert!(!exists, "quiet detached descendant {pid} survived cleanup");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    /// The escaped-descendant case the process group cannot express: the child
    /// exits *successfully*, so the descendant is already reparented and outside
    /// the group by the time cleanup runs. Only the PIDs recorded while the child
    /// was alive still name it.
    #[cfg(unix)]
    #[test]
    fn a_detached_descendant_is_terminated_after_its_parent_exits_successfully() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(EXITING_PARENT_CHILD);
        let pid_file = tempfile::NamedTempFile::new().expect("temporary PID file should exist");
        let mut command = Command::new(executable);
        command.args(args).env(DETACHED_PID_FILE, pid_file.path());

        let out = run_command_with_timeout(&mut command, Duration::from_secs(30))
            .expect("the direct child exits successfully, it does not time out");
        assert!(out.status.success(), "the direct child should exit cleanly");

        let pid = std::fs::read_to_string(pid_file.path())
            .expect("detached descendant should report its PID")
            .parse::<libc::pid_t>()
            .expect("reported descendant PID should be numeric");

        // SIGKILL delivery and reaping by init are asynchronous, so poll rather
        // than assert once; a surviving fixture sleeps far longer than this.
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut last_errno = None;
        while Instant::now() < deadline {
            // SAFETY: signal zero performs existence and permission checks
            // without delivering a signal to the reported process.
            if unsafe { libc::kill(pid, 0) } != 0 {
                last_errno = std::io::Error::last_os_error().raw_os_error();
                break;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        assert_eq!(
            last_errno,
            Some(libc::ESRCH),
            "detached descendant {pid} survived its parent's successful exit"
        );
    }

    /// A timed-out child is waited on, so it cannot survive as a zombie.
    #[cfg(unix)]
    #[test]
    fn a_timed_out_child_is_reaped() {
        let result = run_with_timeout("sleep", &["30"], Duration::from_millis(100));
        assert!(matches!(result, Err(ProcessError::Timeout)));

        // If the child were unreaped it would remain our zombie; `wait`ing for any
        // child would then succeed. ECHILD ("no child processes") is the pass.
        // SAFETY: `waitpid` with WNOHANG only reports already-exited children.
        let rc = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        assert_eq!(rc, -1, "no unreaped child should remain");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }

    /// A non-zero exit is a result, not an error — callers parse failing output.
    #[cfg(unix)]
    #[test]
    fn a_failing_exit_still_returns_captured_output() {
        let out = run_with_timeout("sh", &["-c", "echo oops >&2; exit 3"], Duration::from_secs(5))
            .expect("a non-zero exit is not a helper error");

        assert!(!out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "oops");
        assert_eq!(run_for_stdout("sh", &["-c", "exit 3"], Duration::from_secs(5)), None);
    }

    #[test]
    fn a_missing_executable_is_a_spawn_error() {
        let result = run_with_timeout(
            "sniff-nonexistent-program-xyz",
            &["--version"],
            Duration::from_secs(5),
        );
        assert!(matches!(result, Err(ProcessError::Spawn(_))));
    }

    /// Arguments reach the child verbatim — the helper never invokes a shell, so
    /// shell metacharacters are inert data.
    #[cfg(unix)]
    #[test]
    fn arguments_are_not_shell_interpreted() {
        let out = run_with_timeout("echo", &["$HOME; rm -rf /", "&&", "|"], Duration::from_secs(5))
            .expect("echo should run");

        assert_eq!(out.stdout_lossy().trim(), "$HOME; rm -rf / && |");
    }
}
