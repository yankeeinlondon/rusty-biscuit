//! Bounded launcher for login-shell queries.
//!
//! Alias resolution and the shell classification probes both start the user's
//! shell with its profile loaded, ask one question, and read the answer from
//! stdout. A profile is arbitrary code, so this launcher owns every bound that
//! makes that safe to do during a compose: closed stdin, discarded stderr, a
//! deadline, a cap on retained stdout, and cleanup of whatever the profile left
//! running.
//!
//! ## Job-control hazard
//!
//! Reading aliases and functions requires an *interactive* shell (`-i`),
//! because that is what makes a shell source the rc file they are defined in.
//! Interactive mode also enables job control, and a job-control shell whose
//! process group is not the foreground process group of its controlling
//! terminal signals itself with `SIGTTIN` and is stopped by the kernel — it
//! never exits, so a caller waiting on its stdout waits forever. The shell
//! signals its whole process group, which it inherits from us, so an
//! unprotected caller can be stopped alongside it. Being in the background is
//! the normal case for anything spawned by a test harness or a subprocess
//! chain (nextest, for one, puts every test binary in its own process group),
//! which is why the Unix child gets its own session via `setsid`.
//!
//! The hazard is invisible without a terminal: with no controlling terminal at
//! all (CI, most agent harnesses) job control cannot engage and the shell exits
//! in microseconds. Re-verify by hand under a PTY:
//!
//! ```text
//! script -qc "cargo nextest run -p claudine-cli -E 'test(compose_preflight_discovers_shell_inside_false_block)'" /dev/null
//! ```
//!
//! ## Cleanup
//!
//! Unix: the shell leads a new session and process group, and the whole group
//! is killed once the shell exits or times out, so a profile's background job
//! cannot outlive the query or hold stdout open. A descendant that calls
//! `setsid` itself leaves the group and is not reached; the reader stops
//! waiting for it after a short grace period rather than blocking the compose.
//!
//! Windows: the shell is started without a console window and assigned to a
//! kill-on-close Job Object, which is terminated when the query ends. The
//! assignment happens just after spawn, so a descendant created in that
//! window (before the shell has loaded its runtime) would escape the Job.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Retained stdout. Answers are printed last, so the tail is what is kept.
const STDOUT_TAIL_LIMIT: usize = 64 * 1024;

/// How often the deadline is checked while the shell runs.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// How long to wait for stdout EOF after the process tree has been killed.
/// Only a `setsid` escapee can keep the pipe open this long.
const EOF_GRACE: Duration = Duration::from_millis(100);

/// Runs `command` as a detached, bounded shell query.
///
/// ## Returns
///
/// The exit status and the last [`STDOUT_TAIL_LIMIT`] bytes of stdout, or
/// `None` when the shell could not be spawned or did not exit within
/// `timeout`.
pub(super) fn run_shell_query(command: &mut Command, timeout: Duration) -> Option<(ExitStatus, Vec<u8>)> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    configure_detached(command);

    let mut child = command.spawn().ok()?;
    let tree = match ProcessTree::attach(&child) {
        Some(tree) => tree,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };

    let tail = Arc::new(Mutex::new(Vec::new()));
    let (eof_tx, eof_rx) = mpsc::channel();
    if let Some(mut stdout) = child.stdout.take() {
        let tail = Arc::clone(&tail);
        std::thread::spawn(move || {
            let mut chunk = [0_u8; 8192];
            loop {
                match stdout.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        let mut tail = tail.lock().unwrap_or_else(|poison| poison.into_inner());
                        tail.extend_from_slice(&chunk[..read]);
                        if tail.len() > STDOUT_TAIL_LIMIT {
                            let excess = tail.len() - STDOUT_TAIL_LIMIT;
                            tail.drain(..excess);
                        }
                    }
                }
            }
            let _ = eof_tx.send(());
        });
    }

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL_INTERVAL),
            Ok(None) | Err(_) => {
                tree.terminate();
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    // The shell has exited or been killed; nothing it started may outlive the
    // query or keep stdout open.
    tree.terminate();
    let status = status?;

    let _ = eof_rx.recv_timeout(EOF_GRACE);
    let stdout = std::mem::take(&mut *tail.lock().unwrap_or_else(|poison| poison.into_inner()));
    Some((status, stdout))
}

#[cfg(unix)]
fn configure_detached(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // `setsid`, not `process_group(0)`: only a new session removes the
    // controlling terminal. Measured under a PTY, both the unmodified spawn and
    // the `process_group(0)` variant reach state `T` within 20ms; the `setsid`
    // variant exits normally. A session with no controlling terminal cannot
    // support job control, so the shell disables it (warning on the discarded
    // stderr) and still sources its rc files.
    //
    // SAFETY: `pre_exec` runs in the forked child between `fork` and `exec`,
    // where only async-signal-safe calls are legal. `setsid` is on POSIX's
    // async-signal-safe list, and nothing else here allocates, locks, or
    // touches inherited state. `setsid` can only fail with `EPERM` when the
    // caller is already a process group leader, which a freshly forked child
    // never is.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(windows)]
fn configure_detached(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    // Never open or focus a console window for a probe.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(any(unix, windows)))]
fn configure_detached(_command: &mut Command) {}

#[cfg(unix)]
struct ProcessTree {
    /// Equal to the shell's PID, because the shell leads its own session.
    group: libc::pid_t,
}

#[cfg(unix)]
impl ProcessTree {
    fn attach(child: &std::process::Child) -> Option<Self> {
        libc::pid_t::try_from(child.id()).ok().map(|group| Self { group })
    }

    fn terminate(&self) {
        // SAFETY: `kill` with a negative PID signals a process group and has
        // no memory-safety preconditions. An already-empty group is `ESRCH`.
        unsafe {
            libc::kill(-self.group, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
struct ProcessTree {
    job: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl ProcessTree {
    fn attach(child: &std::process::Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        // SAFETY: every handle passed below is either the freshly created Job
        // or the live child's process handle, both valid for these calls.
        unsafe {
            let job = CreateJobObjectW(None, None).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let assigned = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .and_then(|()| AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())));
            if assigned.is_err() {
                let _ = CloseHandle(job);
                return None;
            }
            Some(Self { job })
        }
    }

    fn terminate(&self) {
        // SAFETY: `job` stays open until `drop`.
        unsafe {
            let _ = windows::Win32::System::JobObjects::TerminateJobObject(self.job, 1);
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        // SAFETY: `job` was created by `attach` and is closed exactly once.
        // Kill-on-close makes the close a backstop for `terminate`.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.job);
        }
    }
}

#[cfg(not(any(unix, windows)))]
struct ProcessTree;

#[cfg(not(any(unix, windows)))]
impl ProcessTree {
    fn attach(_child: &std::process::Child) -> Option<Self> {
        Some(Self)
    }

    fn terminate(&self) {}
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn stub(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("stub-shell");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// A background job that inherits stdout must neither hold the query open
    /// nor survive it.
    #[test]
    fn a_background_descendant_is_killed_and_does_not_hold_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("child.pid");
        let shell = stub(
            dir.path(),
            &format!("sleep 60 & echo $! > '{}'\necho answer", pid_file.display()),
        );

        let started = Instant::now();
        let (status, stdout) =
            run_shell_query(&mut Command::new(&shell), Duration::from_secs(10)).expect("shell exits");

        assert!(status.success());
        assert_eq!(String::from_utf8_lossy(&stdout).trim(), "answer");
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
        let pid: libc::pid_t = std::fs::read_to_string(&pid_file).unwrap().trim().parse().unwrap();
        wait_for_exit(pid);
    }

    #[test]
    fn a_hanging_shell_and_its_descendants_are_killed_at_the_deadline() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("child.pid");
        let shell = stub(
            dir.path(),
            &format!("sleep 60 & echo $! > '{}'\nwait", pid_file.display()),
        );

        let started = Instant::now();
        let result = run_shell_query(&mut Command::new(&shell), Duration::from_millis(300));

        assert!(result.is_none());
        assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
        let pid: libc::pid_t = std::fs::read_to_string(&pid_file).unwrap().trim().parse().unwrap();
        wait_for_exit(pid);
    }

    #[test]
    fn noisy_output_is_bounded_to_the_tail() {
        let dir = tempfile::tempdir().unwrap();
        let shell = stub(
            dir.path(),
            "i=0; while [ $i -lt 2000 ]; do echo 'noise noise noise noise noise noise noise noise'; i=$((i+1)); done\necho final-line",
        );

        let (_, stdout) =
            run_shell_query(&mut Command::new(&shell), Duration::from_secs(10)).expect("shell exits");

        assert!(stdout.len() <= STDOUT_TAIL_LIMIT);
        assert!(String::from_utf8_lossy(&stdout).trim_end().ends_with("final-line"));
    }

    #[test]
    fn a_missing_shell_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(run_shell_query(&mut Command::new(dir.path().join("absent")), Duration::from_secs(1)).is_none());
    }

    /// Polls until `pid` no longer exists; the killed group member may take a
    /// moment to be reaped by init.
    fn wait_for_exit(pid: libc::pid_t) {
        let deadline = Instant::now() + Duration::from_secs(10);
        // SAFETY: signal 0 only checks for existence.
        while unsafe { libc::kill(pid, 0) } == 0 {
            assert!(Instant::now() < deadline, "descendant {pid} survived the query");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
