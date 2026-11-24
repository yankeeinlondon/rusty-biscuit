//! Bounded subprocess execution and process-tree ownership.
//!
//! Every child process sniff spawns goes through `run_with_timeout`. It is the
//! single place that owns the deadline, the pipe draining, and process-tree
//! termination/reaping, so a wedged or verbose child can never wedge a detection.
//! It also removes the Git installation hint this process set for `gix`
//! ([`crate::programs::git_program::forget_installation_hint`]), so a child
//! sees the environment it would have inherited.
//!
//! The module's public items serve other crates' subprocesses:
//!
//! - [`ProcessTree`] and its configuration ([`TreeConfig`], [`TreeIsolation`],
//!   [`CleanupScan`], [`TreeTermination`]) own a spawned child and every
//!   process it starts. `run_with_timeout` uses the same owner. The rules,
//!   guarantees, and per-OS limits are in `sniff/docs/topics/process-trees.md`.
//! - [`ForegroundHandoff`] gives an owned interactive child the controlling
//!   terminal's foreground and restores the caller's foreground and terminal
//!   settings afterwards. It holds terminal state only; the tree it is used
//!   beside keeps containment, termination, and release.
//! - [`configure_detached_child`] serves the opposite case: a caller's
//!   intentionally detached child that nothing waits on or bounds.
//!
//! ## What a timed-out command can leave behind
//!
//! On Windows the owner contains every descendant once it is attached. On
//! Unix it reaches the process group and the descendants a sample recorded;
//! a descendant that forks *and* calls `setsid()` entirely between two
//! samples, whose parent then exits, escapes. Closing that gap portably would
//! need Linux cgroups or `PR_SET_CHILD_SUBREAPER` (Linux-only, and
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
//! `tree::sample_hook`, a test-only callback on the sampler, so the window it
//! exploits is an interval boundary by construction and not by timing estimate.

use std::ffi::OsStr;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tracing::warn;

use crate::performance::{self, counters};

mod foreground;
mod tree;

pub use foreground::ForegroundHandoff;
pub use tree::{
    CleanupScan, ProcessTree, TreeConfig, TreeIsolation, TreeTermination,
    WINDOWS_CREATE_NEW_PROCESS_GROUP, WINDOWS_CREATE_NO_WINDOW,
};

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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
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
                        if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
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
    /// The executable could not be spawned (missing, not executable, ...), or
    /// its process tree could not be owned, in which case the child was killed
    /// and reaped before it ran (Windows) or before its output was read.
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
/// [`ProcessError::Spawn`] if the child could not start or its process tree
/// could not be owned (the child has then been killed and reaped),
/// [`ProcessError::Timeout`] if it exceeded `timeout` (in which case it has been
/// killed and reaped).
///
/// ## Notes
///
/// Both pipes are drained on dedicated threads while the parent polls the
/// deadline. This is the whole point of the helper: a `try_wait()` loop over an
/// undrained `Stdio::piped()` deadlocks any child that writes more than one pipe
/// buffer, because the child blocks in `write()` and so never exits for the loop
/// to observe.
///
/// The child's process tree is owned by a [`ProcessTree`] and terminated before
/// the readers are joined, on success as well as on timeout. A command that
/// deliberately detaches — or a third-party installer that does so on its own —
/// can outlive this boundary on Unix (see the module documentation), so a
/// `Timeout` is not proof that every process the command started has stopped.
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
/// executable, arguments, working directory, and environment are preserved,
/// except that an `EXEPATH` this process set for `gix` is removed (one the
/// caller set is inherited); this boundary owns stdin and captured
/// stdout/stderr so it can enforce the same supervision contract for every
/// subprocess.
///
/// ## Notes
///
/// This is the form the installation boundary uses to run third-party package
/// managers and downloaded installer scripts. On Unix a descendant they detach
/// between two descendant samples can outlive the timeout; see the module
/// documentation for the exact residual.
pub(crate) fn run_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<CapturedOutput, ProcessError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    TreeConfig::process_group().configure(command);
    crate::programs::git_program::forget_installation_hint(command);

    let program = command.get_program().to_owned();
    let mut child = command.spawn().map_err(ProcessError::Spawn)?;
    performance::increment_counter(counters::PROC_SPAWNS, 1);
    let mut process_tree = match ProcessTree::attach(&child) {
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
        log_cleanup_failure(process_tree.terminate(CleanupScan::Always), &program);
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
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    performance::increment_counter(counters::PROC_TIMEOUTS, 1);
                    warn!(
                        program = %program.to_string_lossy(),
                        timeout_ms = timeout.as_millis(),
                        "subprocess exceeded its deadline; terminating supervised processes"
                    );
                    log_cleanup_failure(process_tree.terminate(CleanupScan::Always), &program);
                    // Reap, so the killed child cannot linger as a zombie.
                    let _ = child.wait();
                    break Err(ProcessError::Timeout);
                }
                // The owner decides when a sample is due; most calls are free.
                process_tree.sample();
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                log_cleanup_failure(process_tree.terminate(CleanupScan::Always), &program);
                let _ = child.wait();
                break Err(ProcessError::Spawn(e));
            }
        }
    };

    // The direct child may exit while a descendant still owns an inherited pipe
    // handle, or while one it detached with `setsid` is still running. Terminate
    // the recorded and current descendants and the process group, or the Job
    // Object, before joining readers. Failure paths already terminated and
    // disarmed the owner, so this call is then a no-op; otherwise it scans only
    // when the child was alive long enough to be sampled.
    log_cleanup_failure(process_tree.terminate(CleanupScan::WhenObserved), &program);
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

/// Logs a failed tree cleanup. The command's own outcome is reported unchanged.
fn log_cleanup_failure(result: std::io::Result<()>, program: &OsStr) {
    if let Err(error) = result {
        warn!(
            error = %error,
            program = %program.to_string_lossy(),
            "could not terminate the subprocess's process tree"
        );
    }
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

/// Configure a command so its child outlives the caller and shares neither its
/// process group nor the pipes a parent may be reading from.
///
/// This is for a caller's intentionally detached, unbounded child, which
/// `run_with_timeout` must not supervise. Callers must still give the child
/// `Stdio::null()` (or their own handles); this only prevents the caller's
/// inherited stdio from leaking into it.
pub fn configure_detached_child(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        stop_inheriting_std_handles();
        command.creation_flags(detached_creation_flags());
    }
}

/// Windows children inherit every inheritable handle in the parent, and the
/// stdio handles a parent handed this process stay inheritable even when the
/// child is given `Stdio::null()`. A caller capturing this process's output
/// would then wait for the detached child to exit before its read returned.
/// Clearing the flag on this process's own copies is safe because
/// `Stdio::inherit()` duplicates the handle with inheritance re-enabled.
#[cfg(windows)]
fn stop_inheriting_std_handles() {
    use windows::Win32::Foundation::{HANDLE_FLAG_INHERIT, HANDLE_FLAGS, SetHandleInformation};
    use windows::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    for id in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: both calls only inspect or flag a handle this process owns;
        // a null or invalid handle is skipped rather than passed through.
        unsafe {
            let Ok(handle) = GetStdHandle(id) else {
                continue;
            };
            if handle.0.is_null() {
                continue;
            }
            let _ = SetHandleInformation(handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0));
        }
    }
}

#[cfg(windows)]
const fn detached_creation_flags() -> u32 {
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn detached_process_uses_a_new_process_group() {
        let mut command = Command::new("sh");
        command
            .args(["-c", "printf '%s %s' \"$$\" \"$(ps -o pgid= -p $$ | tr -d ' ')\""])
            .stdout(Stdio::piped());
        configure_detached_child(&mut command);
        let output = command.output().expect("process-group fixture should run");
        assert!(output.status.success());
        let ids = String::from_utf8(output.stdout).expect("fixture output should be UTF-8");
        let mut ids = ids.split_whitespace();
        assert_eq!(ids.next(), ids.next(), "child PID should equal its new PGID");
    }

    #[cfg(windows)]
    #[test]
    fn detached_process_combines_all_required_windows_flags() {
        assert_eq!(detached_creation_flags(), 0x0800_0208);
    }

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
    #[allow(clippy::zombie_processes)]
    fn child_spawns_detached_pipe_holding_descendant() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(DETACHED_PIPE_HOLDING_DESCENDANT);
        let descendant = std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("detached pipe-holding descendant should spawn");
        let descendant_pid =
            libc::pid_t::try_from(descendant.id()).expect("descendant process ID should fit pid_t");
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

    #[cfg(unix)]
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    #[allow(clippy::zombie_processes)]
    fn child_spawns_quiet_detached_descendant() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(QUIET_DETACHED_DESCENDANT);
        std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("quiet detached descendant should spawn");
        std::thread::sleep(Duration::from_secs(30));
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
    #[allow(clippy::zombie_processes)]
    fn child_detaches_descendant_then_exits() {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let args = test_child_args(QUIET_DETACHED_DESCENDANT);
        let descendant = std::process::Command::new(executable)
            .args(args)
            .spawn()
            .expect("quiet detached descendant should spawn");
        let descendant_pid =
            libc::pid_t::try_from(descendant.id()).expect("descendant process ID should fit pid_t");

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            // SAFETY: `descendant_pid` names the child spawned above. A session
            // leader's process-group ID equals its PID.
            if unsafe { libc::getpgid(descendant_pid) } == descendant_pid {
                // Outlive at least two descendant samples, so the escape is
                // recorded while this process is still its parent.
                std::thread::sleep(tree::DESCENDANT_SAMPLE_INTERVAL * 3);
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
        let descendant_pid =
            libc::pid_t::try_from(descendant.id()).expect("descendant process ID should fit pid_t");
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
        let _hook = tree::sample_hook::install(move || {
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
        let out = run_with_timeout(executable, &args, Duration::from_secs(30))
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
                line.strip_suffix("|preserved")
                    .is_some_and(|dir| std::fs::canonicalize(dir).is_ok_and(|d| d == expected_dir))
            }),
            "configured child output should contain the preserved cwd and environment"
        );
    }

    const HINT_HOLDER: &str = "process::tests::installation_hint_holder";
    const EXEPATH_REPORTER: &str = "process::tests::exepath_reporter";
    /// Tells [`installation_hint_holder`] who set `EXEPATH`: `sniff` (the
    /// holder sets the hint itself) or `caller` (inherited from the test).
    const HINT_ROLE: &str = "SNIFF_INSTALLATION_HINT_ROLE";

    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn exepath_reporter() {
        let exepath = std::env::var_os("EXEPATH");
        println!(
            "exepath={}|",
            exepath.map_or("<unset>".into(), |value| value.to_string_lossy().into_owned())
        );
    }

    /// Plays a sniff process: names the installation as `main` does, then
    /// starts [`exepath_reporter`] through the chokepoint and checks what it
    /// inherited. Off Windows nothing is ever named, so the `sniff` role
    /// sets the hint directly.
    #[test]
    #[ignore = "subprocess fixture invoked by the process behavior tests"]
    fn installation_hint_holder() {
        let role = std::env::var(HINT_ROLE).expect("run by the hint tests");
        // SAFETY: this child process runs this one test (`--test-threads=1`)
        // and has started nothing else yet.
        unsafe { crate::programs::git_program::name_installation_for_gix() };
        #[cfg(not(windows))]
        if role == "sniff" {
            // SAFETY: as above.
            unsafe { crate::programs::git_program::set_installation_hint("sniff-hint".as_ref()) };
        }
        let own = std::env::var_os("EXEPATH").expect("EXEPATH is set in this process");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(test_child_args(EXEPATH_REPORTER));
        let output = run_command_with_timeout(&mut command, Duration::from_secs(5))
            .expect("reporter should complete");
        assert!(output.status.success());
        let expected = match role.as_str() {
            "sniff" => "exepath=<unset>|".to_string(),
            "caller" => format!("exepath={}|", own.to_string_lossy()),
            other => panic!("{HINT_ROLE}={other}"),
        };
        assert!(output.stdout_lossy().contains(&expected), "{}", output.stdout_lossy());
        // The hint stays in this process, where gix reads it.
        assert_eq!(std::env::var_os("EXEPATH"), Some(own));
    }

    fn run_hint_holder(role: &str, exepath: Option<&str>) {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(test_child_args(HINT_HOLDER))
            .arg("--test-threads=1")
            .env(HINT_ROLE, role)
            .env_remove("GIT_EXEC_PATH");
        match exepath {
            Some(value) => command.env("EXEPATH", value),
            None => command.env_remove("EXEPATH"),
        };
        let output = run_command_with_timeout(&mut command, Duration::from_secs(5))
            .expect("hint holder should complete");
        let stdout = output.stdout_lossy();
        assert!(
            output.status.success() && stdout.contains("1 passed"),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// On Windows the holder names this host's Git for Windows installation
    /// for real, so the test needs one on `PATH`, as Windows CI has.
    #[test]
    fn a_child_does_not_inherit_the_installation_hint_sniff_set() {
        run_hint_holder("sniff", None);
    }

    #[test]
    fn a_child_inherits_the_installation_hint_its_caller_set() {
        run_hint_holder("caller", Some("caller-hint"));
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
            "quiet detached descendant {pid} survived cleanup"
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
        let out = run_with_timeout(
            "sh",
            &["-c", "echo oops >&2; exit 3"],
            Duration::from_secs(5),
        )
        .expect("a non-zero exit is not a helper error");

        assert!(!out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "oops");
        assert_eq!(
            run_for_stdout("sh", &["-c", "exit 3"], Duration::from_secs(5)),
            None
        );
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
        let out = run_with_timeout(
            "echo",
            &["$HOME; rm -rf /", "&&", "|"],
            Duration::from_secs(5),
        )
        .expect("echo should run");

        assert_eq!(out.stdout_lossy().trim(), "$HOME; rm -rf / && |");
    }
}
