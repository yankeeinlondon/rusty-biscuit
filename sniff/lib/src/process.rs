//! Bounded subprocess execution.
//!
//! Every child process sniff spawns goes through [`run_with_timeout`]. It is the
//! single place that owns the deadline, the pipe draining, and process-tree
//! termination/reaping, so a wedged or verbose child can never wedge a detection.
//!
//! See `sniff/features/2026-07-16-performance/phases/06-remote-network-and-subprocess/spec.md`
//! for the contract this module implements.

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
    use std::process::{Child, Command};

    use std::os::unix::process::CommandExt;

    pub(super) struct ProcessTree {
        process_group: libc::pid_t,
    }

    pub(super) fn configure(command: &mut Command) {
        command.process_group(0);
    }

    impl ProcessTree {
        pub(super) fn attach(child: &Child) -> std::io::Result<Self> {
            let process_group = libc::pid_t::try_from(child.id()).map_err(|_| {
                std::io::Error::other("child process ID does not fit the platform pid_t")
            })?;
            Ok(Self { process_group })
        }

        pub(super) fn terminate(&self) -> std::io::Result<()> {
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

        pub(super) fn terminate(&self) -> std::io::Result<()> {
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
    let process_tree = match process_tree::ProcessTree::attach(&child) {
        Ok(process_tree) => process_tree,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProcessError::Spawn(error));
        }
    };

    // Unix pipe readers use bounded readiness polling because a descendant can
    // leave the process group while retaining an inherited write end. Windows
    // Job Objects provide real tree containment, so EOF remains authoritative.
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
        let _ = process_tree.terminate();
        let _ = child.wait();
        return Err(ProcessError::Spawn(error));
    }
    let stdout_handle = stdout.map(|pipe| drain_control.spawn(pipe));
    let stderr_handle = stderr.map(|pipe| drain_control.spawn(pipe));

    let start = Instant::now();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    performance::increment_counter(counters::PROC_TIMEOUTS, 1);
                    warn!(
                        program = %program.to_string_lossy(),
                        timeout_ms = timeout.as_millis(),
                        "subprocess exceeded its deadline; terminating supervised processes"
                    );
                    let _ = process_tree.terminate();
                    // Reap, so the killed child cannot linger as a zombie.
                    let _ = child.wait();
                    break Err(ProcessError::Timeout);
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                let _ = process_tree.terminate();
                let _ = child.wait();
                break Err(ProcessError::Spawn(e));
            }
        }
    };

    // The direct child may exit while a descendant still owns an inherited pipe
    // handle. Terminate the process group or Job Object before joining readers;
    // cancellable Unix readers cover descendants that escaped the group.
    let _ = process_tree.terminate();
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
        let descendant = std::process::Command::new(executable)
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
                return;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
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
        let expected = format!(
            "{}|preserved",
            working_dir.path().canonicalize().unwrap().display()
        );
        assert!(
            output.stdout_lossy().lines().any(|line| line == expected),
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
