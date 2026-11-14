//! A spawned child and everything it starts, stopped as one unit.
//!
//! Unix: the child must lead its own process group (`process_group(0)` or
//! `setsid`), so the group ID equals its PID and a signal to the negative PID
//! reaches every member that has not moved to another group. A descendant
//! that calls `setsid` or `setpgid` itself leaves the group and is not reached.
//!
//! Windows: the child is assigned to a kill-on-close Job Object just after
//! spawn. A descendant created in the instant between spawn and assignment
//! (before the child has loaded its runtime) escapes the Job; that window is
//! accepted by every caller.

use std::process::Child;
#[cfg(unix)]
use std::time::{Duration, Instant};
#[cfg(windows)]
use std::time::Duration;

/// How often [`ProcessTree::terminate_gracefully`] checks whether the root has
/// exited during its grace period.
#[cfg(unix)]
const GRACE_POLL: Duration = Duration::from_millis(20);

#[cfg(unix)]
pub(crate) struct ProcessTree {
    /// Equal to the root's PID, because the root leads its own group.
    group: libc::pid_t,
}

#[cfg(unix)]
impl ProcessTree {
    pub(crate) fn attach(child: &Child) -> Option<Self> {
        libc::pid_t::try_from(child.id()).ok().map(|group| Self { group })
    }

    /// Kills every member of the group at once.
    pub(crate) fn terminate(&self) {
        // SAFETY: `kill` with a negative PID signals a process group and has
        // no memory-safety preconditions. An already-empty group is `ESRCH`.
        unsafe {
            libc::kill(-self.group, libc::SIGKILL);
        }
    }

    /// Asks the group to exit, then kills whatever is left and reaps the root.
    ///
    /// `SIGTERM` goes to the group; the root gets up to `grace` to exit; then
    /// `SIGKILL` goes to the group unconditionally, which also sweeps a member
    /// that ignored `SIGTERM` while the root obeyed it.
    ///
    /// ## Notes
    ///
    /// When the root exits within the grace period it is reaped before the
    /// sweep, so the group ID is no longer pinned by the root. The kernel
    /// keeps a PID in use while any group member carries it as its group ID,
    /// and an empty group's ID can only be reused after every member is gone,
    /// in which case the sweep has nothing it should reach. That remaining
    /// window (the group empties and its ID is recycled between the reap and
    /// the sweep) is accepted.
    pub(crate) fn terminate_gracefully(&self, grace: Duration, root: &mut Child) {
        // SAFETY: as in `terminate`; signals only this group.
        unsafe {
            libc::kill(-self.group, libc::SIGTERM);
        }
        let deadline = Instant::now() + grace;
        let mut reaped = false;
        while Instant::now() < deadline {
            match root.try_wait() {
                Ok(Some(_)) => {
                    reaped = true;
                    break;
                }
                Ok(None) => std::thread::sleep(GRACE_POLL),
                Err(_) => break,
            }
        }
        self.terminate();
        if !reaped {
            let _ = root.wait();
        }
    }

    /// Lets the tree outlive this handle; nothing is signalled.
    pub(crate) fn release(self) {}
}

#[cfg(windows)]
pub(crate) struct ProcessTree {
    job: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl ProcessTree {
    pub(crate) fn attach(child: &Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};

        // SAFETY: every handle passed below is either the freshly created Job
        // or the live child's process handle, both valid for these calls.
        unsafe {
            let job = CreateJobObjectW(None, None).ok()?;
            let assigned = set_kill_on_close(job, true)
                .and_then(|()| AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())));
            if assigned.is_err() {
                let _ = CloseHandle(job);
                return None;
            }
            Some(Self { job })
        }
    }

    pub(crate) fn terminate(&self) {
        // SAFETY: `job` stays open until `drop`.
        unsafe {
            let _ = windows::Win32::System::JobObjects::TerminateJobObject(self.job, 1);
        }
    }

    /// Windows has no graceful group signal for a console or GUI program, so
    /// this terminates the Job and reaps the root; `grace` is unused.
    pub(crate) fn terminate_gracefully(&self, _grace: Duration, root: &mut Child) {
        self.terminate();
        let _ = root.wait();
    }

    /// Lets the tree outlive this handle: kill-on-close is cleared before the
    /// Job is closed, so a process the root left running (an editor's main
    /// window started by its waiting client) keeps running.
    pub(crate) fn release(self) {
        // SAFETY: `job` is open until `drop`, which runs right after.
        unsafe {
            let _ = set_kill_on_close(self.job, false);
        }
    }
}

/// Sets or clears `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` on `job`.
///
/// ## Safety
///
/// `job` must be an open Job Object handle.
#[cfg(windows)]
unsafe fn set_kill_on_close(job: windows::Win32::Foundation::HANDLE, enabled: bool) -> windows::core::Result<()> {
    use windows::Win32::System::JobObjects::{
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    if enabled {
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    }
    // SAFETY: the caller guarantees `job` is open; `info` outlives the call.
    unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&info).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        // SAFETY: `job` was created by `attach` and is closed exactly once.
        // Kill-on-close (unless `release` cleared it) makes the close a
        // backstop for `terminate`.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.job);
        }
    }
}

#[cfg(not(any(unix, windows)))]
pub(crate) struct ProcessTree;

#[cfg(not(any(unix, windows)))]
impl ProcessTree {
    pub(crate) fn attach(_child: &Child) -> Option<Self> {
        Some(Self)
    }

    pub(crate) fn terminate(&self) {}

    pub(crate) fn terminate_gracefully(&self, _grace: std::time::Duration, root: &mut Child) {
        let _ = root.kill();
        let _ = root.wait();
    }

    pub(crate) fn release(self) {}
}
