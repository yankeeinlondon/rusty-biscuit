//! Windows process-tree ownership: a kill-on-close Job Object assigned while
//! the child is still suspended.
//!
//! Job membership is inherited by every process the child creates and is
//! enforced by the kernel. Because the child is assigned before its first
//! thread runs, there is no window in which it can start a descendant outside
//! the Job, and no descendant sampling is needed.

use std::io;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use std::time::Duration;

use ::windows::Win32::Foundation::HANDLE;
use ::windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use ::windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use ::windows::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};
use ::windows::core::{Owned, PCWSTR};

use super::{CleanupScan, TreeConfig, TreeTermination, combined_creation_flags};

/// The fallible operations, in the order `attach` performs them, plus the two
/// later Job operations. Tests observe and fail them through [`hooks`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    CreateJob,
    SetKillOnClose,
    Assign,
    FindThread,
    Resume,
    ClearKillOnClose,
    Terminate,
}

/// Records `step` and returns an injected failure, if any (tests only).
fn step(step: Step) -> io::Result<()> {
    #[cfg(test)]
    {
        hooks::record(step)
    }
    #[cfg(not(test))]
    {
        let _ = step;
        Ok(())
    }
}

/// Thread-local log of performed steps and injected failures.
#[cfg(test)]
pub(super) mod hooks {
    use std::cell::RefCell;
    use std::io;

    use super::Step;

    thread_local! {
        static LOG: RefCell<Vec<Step>> = const { RefCell::new(Vec::new()) };
        static FAULTS: RefCell<Vec<(Step, io::Error)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(step: Step) -> io::Result<()> {
        LOG.with(|log| log.borrow_mut().push(step));
        FAULTS.with(|faults| {
            let mut faults = faults.borrow_mut();
            match faults.iter().position(|(faulty, _)| *faulty == step) {
                Some(index) => Err(faults.remove(index).1),
                None => Ok(()),
            }
        })
    }

    /// Makes the next `step` fail with `error`.
    pub(in crate::process::tree) fn inject(step: Step, error: io::Error) {
        FAULTS.with(|faults| faults.borrow_mut().push((step, error)));
    }

    /// Returns and clears the log.
    pub(in crate::process::tree) fn take_log() -> Vec<Step> {
        LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
    }

    /// Clears the log and any unused injected failures.
    pub(in crate::process::tree) fn reset() {
        let _ = take_log();
        FAULTS.with(|faults| faults.borrow_mut().clear());
    }
}

pub(super) fn configure(command: &mut Command, config: TreeConfig) {
    command.creation_flags(combined_creation_flags(config.windows_creation_flags));
}

pub(super) struct Core {
    job: Owned<HANDLE>,
    root: u32,
    armed: bool,
}

impl Core {
    pub(super) fn attach(child: &Child) -> io::Result<Self> {
        step(Step::CreateJob)?;
        // SAFETY: a null name and no security attributes create an unnamed,
        // non-inheritable Job; the handle is owned at once.
        let job = unsafe {
            Owned::new(CreateJobObjectW(None, PCWSTR::null()).map_err(io::Error::other)?)
        };
        step(Step::SetKillOnClose)?;
        set_limits(&job, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE)?;
        step(Step::Assign)?;
        // SAFETY: `job` is a live Job handle and the child's process handle
        // stays valid while `child` is borrowed.
        unsafe { AssignProcessToJobObject(*job, HANDLE(child.as_raw_handle())) }
            .map_err(io::Error::other)?;

        // From here a failure drops an armed, assigned owner, which terminates
        // the Job and with it the still-suspended child.
        let core = Self {
            job,
            root: child.id(),
            armed: true,
        };
        resume_initial_thread(child.id())?;
        Ok(core)
    }

    pub(super) fn root(&self) -> u32 {
        self.root
    }

    /// Job membership is inherited and kernel-enforced, so nothing is sampled.
    pub(super) fn sample(&mut self) {}

    pub(super) fn terminate(&mut self, _scan: CleanupScan) -> io::Result<()> {
        if !self.armed {
            return Ok(());
        }
        step(Step::Terminate)?;
        // SAFETY: `job` is an owned, live Job handle.
        unsafe { TerminateJobObject(*self.job, 1) }.map_err(io::Error::other)?;
        self.armed = false;
        Ok(())
    }

    /// Windows has no graceful signal for a tree, so `_grace` is unused and the
    /// Job is terminated at once.
    pub(super) fn terminate_gracefully(
        &mut self,
        _grace: Duration,
        child: &mut Child,
    ) -> TreeTermination {
        let cleanup = self.terminate(CleanupScan::Always);
        if cleanup.is_err() {
            let _ = child.kill();
        }
        TreeTermination {
            status: child.wait(),
            cleanup,
        }
    }

    pub(super) fn release(mut self) -> io::Result<()> {
        if !self.armed {
            return Ok(());
        }
        // On failure `self` drops still armed, which terminates the Job:
        // closing a Job that still has kill-on-close would kill it anyway.
        step(Step::ClearKillOnClose)?;
        set_limits(&self.job, JOB_OBJECT_LIMIT(0))?;
        self.armed = false;
        Ok(())
    }
}

impl Drop for Core {
    /// Best-effort termination when neither terminated nor released; the
    /// `Owned` handle then closes the Job.
    fn drop(&mut self) {
        if self.armed {
            // SAFETY: `job` is an owned, live Job handle.
            let _ = unsafe { TerminateJobObject(*self.job, 1) };
        }
    }
}

fn set_limits(job: &Owned<HANDLE>, flags: JOB_OBJECT_LIMIT) -> io::Result<()> {
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = flags;
    // SAFETY: `limits` is a fully initialized structure of the size passed,
    // and `job` is an owned, live Job handle.
    unsafe {
        SetInformationJobObject(
            **job,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&limits).cast(),
            std::mem::size_of_val(&limits) as u32,
        )
    }
    .map_err(io::Error::other)
}

/// Resumes the suspended child's initial thread, its only thread until it
/// runs.
fn resume_initial_thread(process_id: u32) -> io::Result<()> {
    step(Step::FindThread)?;
    // SAFETY: the snapshot handle is owned at once and `entry` is sized for
    // the iteration calls.
    let thread_id = unsafe {
        let snapshot =
            Owned::new(CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(io::Error::other)?);
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        Thread32First(*snapshot, &mut entry).map_err(io::Error::other)?;
        loop {
            if entry.th32OwnerProcessID == process_id {
                break entry.th32ThreadID;
            }
            if Thread32Next(*snapshot, &mut entry).is_err() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "suspended child process has no thread",
                ));
            }
        }
    };

    step(Step::Resume)?;
    // SAFETY: the thread handle is owned at once and valid for `ResumeThread`.
    unsafe {
        let thread =
            Owned::new(OpenThread(THREAD_SUSPEND_RESUME, false, thread_id).map_err(io::Error::other)?);
        // `u32::MAX` is the documented failure sentinel.
        if ResumeThread(*thread) == u32::MAX {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
