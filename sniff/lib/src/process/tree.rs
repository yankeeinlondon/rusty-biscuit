//! Ownership of a spawned child and every process it starts.
//!
//! A [`ProcessTree`] lets a caller stop a command's background work along with
//! the command itself, so a leftover descendant cannot keep running or hold an
//! output pipe open after the caller has moved on. The caller keeps the
//! [`Child`], its pipes, and its wait loop; the owner only sets the child up,
//! watches for descendants, and terminates or releases them.
//!
//! The rules, guarantees, and limits on each OS are described in
//! `sniff/docs/topics/process-trees.md`.

use std::io;
use std::process::{Child, Command, ExitStatus};
use std::time::Duration;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as platform;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as platform;

/// Windows `CREATE_NO_WINDOW`: the child gets no console window.
pub const WINDOWS_CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Windows `CREATE_NEW_PROCESS_GROUP`: the child leads a new console process
/// group, so console control events sent to the caller's group skip it.
pub const WINDOWS_CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Windows `CREATE_SUSPENDED`, added to every owned command so the child cannot
/// run, and so cannot start a descendant, before it is assigned to its Job.
const WINDOWS_CREATE_SUSPENDED: u32 = 0x0000_0004;

/// How long a child must live before its descendants are first recorded, and
/// the shortest gap between two later recordings.
///
/// Deferring the first sample by a whole interval keeps the common case free:
/// a command that completes inside one interval never scans the process table.
#[cfg_attr(windows, allow(dead_code))]
pub(crate) const DESCENDANT_SAMPLE_INTERVAL: Duration = Duration::from_millis(250);

/// The creation-flag word an owned Windows command is spawned with.
///
/// `Command::creation_flags` replaces the previous value rather than adding to
/// it, so suspension and the caller's own flags are combined here, once.
#[cfg_attr(not(windows), allow(dead_code))]
const fn combined_creation_flags(flags: u32) -> u32 {
    flags | WINDOWS_CREATE_SUSPENDED
}

/// How an owned command is separated from its caller on Unix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeIsolation {
    /// The child leads a new process group and keeps the caller's session
    /// and controlling terminal.
    ProcessGroup,
    /// The child leads a new session, and with it a new process group, with no
    /// controlling terminal. Windows has no session concept, so there this is
    /// the same as [`TreeIsolation::ProcessGroup`].
    Session,
}

/// How to configure a command whose process tree will be owned.
///
/// ## Examples
///
/// ```
/// use sniff::process::{TreeConfig, WINDOWS_CREATE_NO_WINDOW};
///
/// let config = TreeConfig::session().windows_creation_flags(WINDOWS_CREATE_NO_WINDOW);
/// assert_ne!(config, TreeConfig::session());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeConfig {
    #[cfg_attr(windows, allow(dead_code))]
    isolation: TreeIsolation,
    #[cfg_attr(not(windows), allow(dead_code))]
    windows_creation_flags: u32,
}

impl TreeConfig {
    /// A new process group on Unix; no extra creation flags on Windows.
    pub const fn process_group() -> Self {
        Self {
            isolation: TreeIsolation::ProcessGroup,
            windows_creation_flags: 0,
        }
    }

    /// A new session on Unix; no extra creation flags on Windows.
    pub const fn session() -> Self {
        Self {
            isolation: TreeIsolation::Session,
            windows_creation_flags: 0,
        }
    }

    /// Sets the Windows creation flags the child is spawned with, such as
    /// [`WINDOWS_CREATE_NO_WINDOW`] or [`WINDOWS_CREATE_NEW_PROCESS_GROUP`].
    /// Ignored on Unix. `CREATE_SUSPENDED` is always added by
    /// [`TreeConfig::configure`] and need not be passed.
    #[must_use]
    pub const fn windows_creation_flags(self, flags: u32) -> Self {
        Self {
            windows_creation_flags: flags,
            ..self
        }
    }

    /// Prepares `command` so its child can be attached with
    /// [`ProcessTree::attach`]. Call it before `Command::spawn`.
    ///
    /// ## Notes
    ///
    /// - On Unix, call this before registering any `pre_exec` hook of your own
    ///   (a terminal foreground handoff, for example): hooks then run after
    ///   the child has its new group or session.
    /// - On Windows, never call `Command::creation_flags` after this: it would
    ///   replace the combined word and drop `CREATE_SUSPENDED`. Pass your flags
    ///   through [`TreeConfig::windows_creation_flags`] instead.
    pub fn configure(self, command: &mut Command) {
        platform::configure(command, self);
    }
}

/// Whether an immediate termination scans the process table for descendants
/// when none were recorded by [`ProcessTree::sample`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupScan {
    /// Scan only when descendants were recorded. A short command that ends
    /// normally then costs no process-table scan.
    WhenObserved,
    /// Always scan. Use it after a timeout, a cancellation, or a wait error.
    Always,
}

/// What [`ProcessTree::terminate_gracefully`] observed.
///
/// The two results are separate so a failed cleanup never hides or replaces
/// the direct child's exit status.
#[derive(Debug)]
pub struct TreeTermination {
    /// The direct child's exit status, from waiting for it.
    pub status: io::Result<ExitStatus>,
    /// Whether the tree's remaining members were terminated.
    pub cleanup: io::Result<()>,
}

/// Owns a spawned child's process tree: the child and every process it starts.
///
/// Terminating a process stops it; reaping one collects the exit status of an
/// exited direct child, which only its parent can do. The caller always reaps
/// the direct child with [`Child::wait`]; this owner never reaps it on its own
/// (except inside [`ProcessTree::terminate_gracefully`], which is handed the
/// `Child`), and its `Drop`, holding no `Child`, cannot reap anything.
/// Descendants other than the direct child are reaped by the OS once their own
/// parent exits.
///
/// ## Examples
///
/// Configure, spawn, attach (killing and waiting for the child if attachment
/// fails), drive sampling from the wait loop, then clean up:
///
/// ```no_run
/// use std::process::Command;
/// use std::time::Duration;
///
/// use sniff::process::{CleanupScan, ProcessTree, TreeConfig};
///
/// let mut command = Command::new("make");
/// TreeConfig::process_group().configure(&mut command);
/// let mut child = command.spawn()?;
/// let mut tree = match ProcessTree::attach(&child) {
///     Ok(tree) => tree,
///     Err(error) => {
///         let _ = child.kill();
///         let _ = child.wait();
///         return Err(error);
///     }
/// };
///
/// let status = loop {
///     if let Some(status) = child.try_wait()? {
///         break status;
///     }
///     tree.sample();
///     std::thread::sleep(Duration::from_millis(10));
/// };
///
/// if let Err(error) = tree.terminate(CleanupScan::WhenObserved) {
///     eprintln!("could not stop the command's background processes: {error}");
/// }
/// println!("make exited with {status}");
/// # Ok::<(), std::io::Error>(())
/// ```
#[must_use = "dropping an owned tree terminates it"]
pub struct ProcessTree {
    core: platform::Core,
}

impl ProcessTree {
    /// Takes ownership of `child`'s process tree.
    ///
    /// The command must have been prepared with [`TreeConfig::configure`], and
    /// `child` must not have been reaped yet. Call this immediately after a
    /// successful spawn, before starting readers or anything else that can
    /// fail.
    ///
    /// On Windows the suspended child is assigned to a kill-on-close Job and
    /// only then resumed, so every descendant it starts is in the Job. On Unix
    /// the child's process group is verified to be its own.
    ///
    /// ## Errors
    ///
    /// Returns the underlying OS error when the Job cannot be created,
    /// configured, or assigned, or the child's thread cannot be found or
    /// resumed (Windows); or when the child is a live process in a foreign
    /// process group or has an unusable process ID (Unix). On error the caller
    /// must kill and wait for `child`: on Windows it may still be suspended.
    pub fn attach(child: &Child) -> io::Result<Self> {
        platform::Core::attach(child).map(|core| Self { core })
    }

    /// Records the child's current descendants on Unix, when a recording is
    /// due. Call it on every iteration of the wait loop: the first recording
    /// is due one interval (250 ms) after attachment and later ones no more
    /// often than that, so a call that is not due costs a clock read. Windows
    /// needs no recording and does nothing.
    pub fn sample(&mut self) {
        self.core.sample();
    }

    /// Hard-stops the tree: the process group and the recorded descendants
    /// still running (Unix), or the Job (Windows).
    ///
    /// After a successful call the owner is disarmed: a later call and the
    /// drop do nothing. The caller still waits for the direct child.
    ///
    /// ## Errors
    ///
    /// Returns the error from signaling the group or terminating the Job; an
    /// already empty Unix group is success. On error the owner stays armed,
    /// so its drop tries again; the caller should fall back to
    /// [`Child::kill`].
    pub fn terminate(&mut self, scan: CleanupScan) -> io::Result<()> {
        self.core.terminate(scan)
    }

    /// Asks the tree to exit, then hard-stops whatever is left and waits for
    /// the direct child.
    ///
    /// On Unix, `SIGTERM` goes to the group and recorded descendants, the
    /// direct child gets up to `grace` to exit, and then the remaining members
    /// are killed, even when the child exited first. Windows has no graceful
    /// signal for a tree, so it terminates the Job at once and ignores
    /// `grace`. If the hard stop fails, the direct child is killed before it
    /// is waited for.
    pub fn terminate_gracefully(&mut self, grace: Duration, child: &mut Child) -> TreeTermination {
        self.core.terminate_gracefully(grace, child)
    }

    /// Lets the tree outlive this owner; nothing is signaled.
    ///
    /// ## Errors
    ///
    /// On Windows, returns the error from clearing the Job's kill-on-close
    /// limit. The owner is then dropped still armed, so the tree is
    /// terminated. Unix never fails.
    pub fn release(self) -> io::Result<()> {
        self.core.release()
    }
}

impl std::fmt::Debug for ProcessTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessTree")
            .field("root", &self.core.root())
            .finish_non_exhaustive()
    }
}

/// Test-only observation point, invoked after every completed descendant sample.
///
/// Sampling is the only thing standing between a `setsid` descendant and the
/// Unix containment gap, so a test that pins that gap has to know precisely
/// when a sample finished. A hook installed here runs synchronously on the
/// sampling thread, which means a test that *blocks* inside it holds the
/// sampler still for exactly as long as it blocks — the escape it manufactures
/// then lands strictly between two samples on any host at any load, with no
/// timing estimate involved.
///
/// The slot is thread-local because [`ProcessTree::sample`] always runs on the
/// thread driving the wait loop; an installed hook therefore cannot leak into
/// a concurrently running test.
#[cfg(all(unix, test))]
pub(in crate::process) mod sample_hook {
    use std::cell::RefCell;

    thread_local! {
        static HOOK: RefCell<Option<Box<dyn FnMut()>>> = const { RefCell::new(None) };
    }

    /// Uninstalls the hook when dropped, so a panicking test cannot leave one
    /// armed for whatever runs next on this thread.
    pub(in crate::process) struct HookGuard(());

    impl Drop for HookGuard {
        fn drop(&mut self) {
            HOOK.with(|slot| *slot.borrow_mut() = None);
        }
    }

    pub(in crate::process) fn install(hook: impl FnMut() + 'static) -> HookGuard {
        HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
        HookGuard(())
    }

    pub(in crate::process) fn after_sample() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspension_is_added_to_no_flags() {
        assert_eq!(combined_creation_flags(0), 0x0000_0004);
    }

    #[test]
    fn suspension_is_combined_with_no_window() {
        assert_eq!(
            combined_creation_flags(WINDOWS_CREATE_NO_WINDOW),
            0x0800_0004
        );
    }

    #[test]
    fn suspension_is_combined_with_a_new_process_group() {
        assert_eq!(
            combined_creation_flags(WINDOWS_CREATE_NEW_PROCESS_GROUP),
            0x0000_0204
        );
    }

    #[test]
    fn suspension_is_never_lost_and_caller_flags_are_kept() {
        for flags in [
            0,
            WINDOWS_CREATE_SUSPENDED,
            WINDOWS_CREATE_NO_WINDOW | WINDOWS_CREATE_NEW_PROCESS_GROUP,
            u32::MAX,
            0x0000_0008,
        ] {
            let combined = combined_creation_flags(flags);
            assert_eq!(combined & WINDOWS_CREATE_SUSPENDED, WINDOWS_CREATE_SUSPENDED);
            assert_eq!(combined & flags, flags, "caller flags {flags:#x} were dropped");
        }
    }

    #[test]
    fn creation_flags_are_recorded_and_isolation_kept() {
        let config = TreeConfig::process_group().windows_creation_flags(WINDOWS_CREATE_NO_WINDOW);
        assert_eq!(config.isolation, TreeIsolation::ProcessGroup);
        assert_eq!(config.windows_creation_flags, WINDOWS_CREATE_NO_WINDOW);
        assert_eq!(TreeConfig::session().isolation, TreeIsolation::Session);
        assert_eq!(TreeConfig::session().windows_creation_flags, 0);
    }
}
