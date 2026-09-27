//! Signal-delivery fixtures for L1 tests whose subject is a running wrapper.
//!
//! Two pieces, used together:
//!
//! - [`SignalledRun`] owns a spawned `claudine` and everything it leaves in its
//!   process group. The wrapper is its own group leader, so dropping the run
//!   — after a passing assertion or during a panic's unwind — `SIGKILL`s the
//!   whole group, including a lifecycle shell that outlived a deliberate
//!   wrapper force-exit. The leader is reaped only after that kill, so the
//!   group id cannot have been reused by then. Every wait is bounded, so a hung
//!   wrapper panics into that cleanup instead of being killed by nextest's
//!   timeout, which would skip it.
//! - [`ShellBarrier`] is a readiness/release handshake with a shell snippet:
//!   the snippet announces it has been reached, then blocks until the test
//!   releases it. A test synchronizes on the state it asserts about rather
//!   than on a sleep. Dropping it ends the blocked shell, which may sit in no
//!   process group the test owns.
//!
//! Output goes to files rather than pipes: a shell that survives the wrapper
//! keeps the wrapper's descriptors open, so a pipe read would wait on it, and a
//! file can be polled for a notice while the wrapper is still running.

use std::cell::Cell;
use std::fs;
use std::os::unix::process::ExitStatusExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use super::strip_ansi;

/// Poll interval for every readiness and exit wait in this module.
const POLL: Duration = Duration::from_millis(5);

/// Substring of the notice the compose SIGINT handler writes on the first
/// press.
pub const COMPOSE_INTERRUPT_NOTICE: &str = "User interrupted compose operation";

/// Substring of the notice a repeat press writes when it arms the terminal
/// lifecycle grace.
pub const GRACE_ARMED_NOTICE: &str = "letting the lifecycle event finish";

/// Substring of the notice written when the grace expires with the run still
/// going.
pub const GRACE_EXPIRED_NOTICE: &str = "lifecycle event still running — force-exiting";

/// Mirror of the CLI's `TERMINAL_LIFECYCLE_EXIT_GRACE`, which a test binary
/// cannot import.
pub const TERMINAL_LIFECYCLE_EXIT_GRACE: Duration = Duration::from_millis(500);

/// How long dropping a [`ShellBarrier`] waits for a released shell to exit
/// before killing it. A released shell polls every 10 ms.
const BARRIER_EXIT_TIMEOUT: Duration = Duration::from_secs(5);

/// A spawned `claudine` whose process group the fixture owns.
///
/// The wrapper stays unreaped — a zombie once it exits — until `Drop`: its pid
/// is the group id, and an unreaped pid cannot be reused, so the group kill in
/// `Drop` can only reach processes this run started.
pub struct SignalledRun {
    child: Child,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    /// Observed without reaping; see [`peek_exit`].
    exited: Option<(ExitStatus, Instant)>,
}

/// The exit of a [`SignalledRun`], with the stderr it wrote.
pub struct RunExit {
    pub status: ExitStatus,
    /// When the fixture observed the exit; at most one poll interval late.
    pub exited_at: Instant,
    /// ANSI-stripped stderr.
    pub stderr: String,
}

impl SignalledRun {
    /// Spawn `command` as the leader of a new process group, with stdout and
    /// stderr captured to files in `output_dir`.
    ///
    /// `command` should come from `CliProcessFixture::command_std()` (or the
    /// builder's `build_std()`), so the L1 spawn contract still applies.
    pub fn spawn(mut command: Command, output_dir: &Path) -> Self {
        use std::os::unix::process::CommandExt as _;

        let stdout_path = output_dir.join("signalled-run.stdout");
        let stderr_path = output_dir.join("signalled-run.stderr");
        let child = command
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(fs::File::create(&stdout_path).expect("create stdout capture"))
            .stderr(fs::File::create(&stderr_path).expect("create stderr capture"))
            .spawn()
            .expect("spawn claudine");
        Self {
            child,
            stdout_path,
            stderr_path,
            exited: None,
        }
    }

    pub fn pid(&self) -> i32 {
        self.child.id() as i32
    }

    /// Send `SIGINT` to the wrapper (not its group, as a terminal would).
    ///
    /// ## Returns
    ///
    /// The instant taken immediately *before* the send. The handler cannot run
    /// earlier than that, so it is a strict lower bound for anything the press
    /// starts — a deadline measured from it can only read long, never short.
    ///
    /// ## Panics
    ///
    /// When the signal cannot be sent. An exited wrapper is not reaped until
    /// `Drop`, so a late press still reaches it (and is ignored).
    pub fn interrupt(&self) -> Instant {
        let sent_at = Instant::now();
        // SAFETY: `kill(2)` with a pid this fixture spawned and has not reaped.
        let result = unsafe { libc::kill(self.pid(), libc::SIGINT) };
        assert_eq!(
            result,
            0,
            "SIGINT to claudine (pid {}) failed: {}; stderr:\n{}",
            self.pid(),
            std::io::Error::last_os_error(),
            self.stderr()
        );
        sent_at
    }

    /// Wait until stderr contains `needle` (matched after stripping ANSI).
    ///
    /// ## Returns
    ///
    /// When the needle was observed; at most one poll interval after the write.
    ///
    /// ## Panics
    ///
    /// When `timeout` elapses, or the wrapper exits without writing it.
    pub fn wait_for_stderr(&mut self, needle: &str, timeout: Duration) -> Instant {
        self.wait_for_stderr_count(needle, 1, timeout)
    }

    /// Wait until stderr contains `needle` at least `count` times — how a test
    /// acknowledges a repeat press whose notice is identical to the last one.
    ///
    /// ## Returns
    ///
    /// When the `count`th occurrence was observed; at most one poll interval
    /// after the write.
    ///
    /// ## Panics
    ///
    /// As [`SignalledRun::wait_for_stderr`].
    pub fn wait_for_stderr_count(
        &mut self,
        needle: &str,
        count: usize,
        timeout: Duration,
    ) -> Instant {
        let deadline = Instant::now() + timeout;
        let written = |run: &Self| run.stderr().matches(needle).count() >= count;
        loop {
            if written(self) {
                return Instant::now();
            }
            if self.poll_exit().is_some() {
                // The final write may have landed between the read and the exit.
                if written(self) {
                    return Instant::now();
                }
                panic!(
                    "claudine exited ({:?}) without writing {needle:?} {count} time(s); stderr:\n{}",
                    self.exited.map(|(status, _)| status),
                    self.stderr()
                );
            }
            assert!(
                Instant::now() < deadline,
                "claudine did not write {needle:?} {count} time(s) within {timeout:?}; stderr:\n{}",
                self.stderr()
            );
            std::thread::sleep(POLL);
        }
    }

    /// Return at `until`, having confirmed the wrapper was still running
    /// throughout. For a test whose subject is that the wrapper does *not*
    /// exit during a window.
    ///
    /// ## Panics
    ///
    /// When the wrapper exits before `until`.
    pub fn assert_running_until(&mut self, until: Instant) {
        loop {
            if let Some((status, _)) = self.poll_exit() {
                panic!(
                    "claudine exited ({status:?}) {:?} before it was allowed to; stderr:\n{}",
                    until.saturating_duration_since(Instant::now()),
                    self.stderr()
                );
            }
            let now = Instant::now();
            if now >= until {
                return;
            }
            std::thread::sleep(POLL.min(until - now));
        }
    }

    /// Wait until `path` exists — a marker a fake provider, one of its
    /// descendants, or a lifecycle action writes.
    ///
    /// ## Panics
    ///
    /// When `timeout` elapses, or the wrapper exits first.
    pub fn wait_for_path(&mut self, path: &Path, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        while !path.exists() {
            if let Some((status, _)) = self.poll_exit() {
                panic!(
                    "claudine exited ({status:?}) before {} appeared; stderr:\n{}",
                    path.display(),
                    self.stderr()
                );
            }
            assert!(
                Instant::now() < deadline,
                "{} did not appear within {timeout:?}; stderr:\n{}",
                path.display(),
                self.stderr()
            );
            std::thread::sleep(POLL);
        }
    }

    /// Wait for the wrapper to exit.
    ///
    /// Only the wrapper is awaited; descendants still in its group are left for
    /// `Drop`, which is what makes a force-exit test possible.
    ///
    /// ## Panics
    ///
    /// When `timeout` elapses first.
    pub fn wait_for_exit(&mut self, timeout: Duration) -> RunExit {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some((status, exited_at)) = self.poll_exit() {
                return RunExit {
                    status,
                    exited_at,
                    stderr: self.stderr(),
                };
            }
            assert!(
                Instant::now() < deadline,
                "claudine did not exit within {timeout:?}; stderr:\n{}",
                self.stderr()
            );
            std::thread::sleep(POLL);
        }
    }

    /// ANSI-stripped stderr written so far.
    pub fn stderr(&self) -> String {
        strip_ansi(&fs::read_to_string(&self.stderr_path).unwrap_or_default())
    }

    /// Raw stdout written so far.
    pub fn stdout(&self) -> String {
        fs::read_to_string(&self.stdout_path).unwrap_or_default()
    }

    fn poll_exit(&mut self) -> Option<(ExitStatus, Instant)> {
        if self.exited.is_none()
            && let Some(status) = peek_exit(self.pid())
        {
            self.exited = Some((status, Instant::now()));
        }
        self.exited
    }
}

impl Drop for SignalledRun {
    fn drop(&mut self) {
        // Nothing has reaped the leader yet, so its pid — the group id — is
        // still reserved and the group holds only processes this run started;
        // ESRCH just means it is already empty.
        // SAFETY: `kill(2)` on a group this fixture created and still pins.
        unsafe {
            libc::kill(-self.pid(), libc::SIGKILL);
        }
        let _ = self.child.wait();
    }
}

/// The exit status of child `pid` once it has exited, leaving it unreaped
/// (`WNOWAIT`) so its pid stays reserved.
///
/// ## Panics
///
/// When `waitid(2)` fails, which means `pid` is not an unreaped child.
fn peek_exit(pid: i32) -> Option<ExitStatus> {
    // `si_code` values for an exited child; libc exports them only on Linux,
    // and macOS uses the same numbers.
    const CLD_EXITED: i32 = 1;
    const CLD_KILLED: i32 = 2;
    const CLD_DUMPED: i32 = 3;

    // SAFETY: `siginfo_t` is plain data, for which all-zero is valid.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // SAFETY: `info` is a valid out-pointer for the duration of the call.
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    assert_eq!(
        result,
        0,
        "waitid on claudine (pid {pid}) failed: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: after a successful `waitid`, the child fields are initialized;
    // with `WNOHANG` and nothing to report, they stay zero.
    let (child, status) = unsafe { (info.si_pid(), info.si_status()) };
    if child == 0 {
        return None;
    }
    // Rebuild the `wait(2)` status word `ExitStatus` decodes.
    let raw = match info.si_code {
        CLD_EXITED => (status & 0xff) << 8,
        CLD_KILLED => status,
        CLD_DUMPED => status | 0x80,
        code => panic!("waitid reported pid {pid} with non-exit si_code {code}"),
    };
    Some(ExitStatus::from_raw(raw))
}

/// A readiness/release handshake between a test and a shell snippet it
/// planted in a fake provider or a lifecycle `shell` action.
///
/// The blocked shell may sit outside every process group the test owns (an
/// agent's orphaned descendant, say), so the barrier ends it itself: dropping
/// it runs [`ShellBarrier::release_or_kill`]. The marker files live in the
/// test's workspace, which must therefore outlive the barrier. Both snippets
/// also stop waiting once that directory is gone, which covers a shell that
/// had not yet recorded its pid when the barrier was dropped.
pub struct ShellBarrier {
    /// The directory holding both marker files.
    dir: PathBuf,
    reached: PathBuf,
    release: PathBuf,
    /// Set once [`ShellBarrier::release_or_kill`] has ended the shell, so a
    /// later call cannot probe or kill its pid after it has been recycled.
    ended: Cell<bool>,
}

impl ShellBarrier {
    /// A barrier whose marker files live in `dir` under `name`.
    pub fn new(dir: &Path, name: &str) -> Self {
        Self {
            dir: dir.to_path_buf(),
            reached: dir.join(format!("{name}.reached")),
            release: dir.join(format!("{name}.release")),
            ended: Cell::new(false),
        }
    }

    /// POSIX shell that records its shell's pid as having reached the barrier,
    /// then blocks until [`ShellBarrier::release`] or until the barrier's
    /// directory is removed.
    ///
    /// Everything that must hold while the test acts (a `trap`, say) has to be
    /// in place before this snippet runs.
    pub fn block_snippet(&self) -> String {
        let reached = shell_quote(&self.reached);
        let staging = shell_quote(&self.reached.with_extension("staging"));
        let release = shell_quote(&self.release);
        let dir = shell_quote(&self.dir);
        // Staged then renamed, so an observer never reads an empty pid.
        format!(
            "echo $$ > {staging} && mv {staging} {reached}; \
             while [ ! -e {release} ] && [ -d {dir} ]; do /bin/sleep 0.01; done"
        )
    }

    /// POSIX shell that blocks until another process has reached the barrier,
    /// or until the barrier's directory is removed.
    pub fn await_reached_snippet(&self) -> String {
        format!(
            "while [ ! -e {} ] && [ -d {} ]; do /bin/sleep 0.01; done",
            shell_quote(&self.reached),
            shell_quote(&self.dir)
        )
    }

    /// Wait for the snippet to reach the barrier.
    ///
    /// ## Returns
    ///
    /// The pid of the shell that reached it.
    ///
    /// ## Panics
    ///
    /// As [`SignalledRun::wait_for_path`].
    pub fn wait_reached(&self, run: &mut SignalledRun, timeout: Duration) -> i32 {
        run.wait_for_path(&self.reached, timeout);
        let pid = fs::read_to_string(&self.reached).expect("read barrier pid");
        pid.trim()
            .parse()
            .unwrap_or_else(|_| panic!("barrier recorded a non-pid {pid:?}"))
    }

    /// The file the snippet writes on reaching the barrier, for a test that
    /// cannot wait through a [`SignalledRun`] (one driving a terminal pane).
    pub fn reached_path(&self) -> &Path {
        &self.reached
    }

    pub fn release(&self) {
        fs::write(&self.release, b"").expect("release barrier");
    }

    /// Release the barrier and wait, up to `timeout`, for the shell that
    /// reached it to exit; `SIGKILL` it if it has not. Never panics, so it is
    /// safe from a `Drop`, and idempotent.
    ///
    /// On return the shell has exited or been killed, so the directory holding
    /// the marker files may be removed. A shell that never recorded its pid is
    /// not waited for; it stops once that directory is gone.
    ///
    /// ## Notes
    ///
    /// The shell is usually not this process's child, so its pid cannot be
    /// pinned: the kill trusts that the pid, alive at the probe just before
    /// it, was not recycled within `timeout`.
    pub fn release_or_kill(&self, timeout: Duration) {
        if self.ended.get() {
            return;
        }
        let _ = fs::write(&self.release, b"");
        let Some(pid) = fs::read_to_string(&self.reached)
            .ok()
            .and_then(|pid| pid.trim().parse::<i32>().ok())
        else {
            return;
        };
        // SAFETY: signal 0 only probes whether `pid` exists.
        let alive = || unsafe { libc::kill(pid, 0) } == 0;
        let deadline = Instant::now() + timeout;
        while alive() {
            if Instant::now() >= deadline {
                eprintln!(
                    "barrier shell (pid {pid}) still running {timeout:?} after its release; killing it"
                );
                // SAFETY: `kill(2)` on the pid the snippet recorded; see Notes.
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                }
                break;
            }
            std::thread::sleep(POLL);
        }
        self.ended.set(true);
    }
}

impl Drop for ShellBarrier {
    fn drop(&mut self) {
        self.release_or_kill(BARRIER_EXIT_TIMEOUT);
    }
}

/// Single-quote `path` for POSIX `sh`.
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}
