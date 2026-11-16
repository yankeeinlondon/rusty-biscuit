//! Handing the controlling terminal's foreground to an owned child.
//!
//! [`ForegroundHandoff`] sits beside [`super::ProcessTree`]: the tree owns the
//! child's processes, while the handoff owns the terminal's foreground group
//! and settings for as long as the child may read from the terminal. The rules
//! are described in `sniff/docs/topics/process-trees.md`.

use std::io;
use std::process::Command;

/// Gives a child the controlling terminal's foreground and restores the
/// caller's foreground and terminal settings afterwards.
///
/// A child in its own process group is a *background* group: the kernel stops
/// it with `SIGTTIN` when it reads the terminal, and delivers typed Ctrl-C to
/// the caller instead of to it. A handoff makes the child's group the
/// terminal's foreground group before the child executes its program, so an
/// interactive command (an editor, a prompt, a lifecycle shell) reads the
/// keyboard and receives Ctrl-C itself.
///
/// ## Lifecycle
///
/// The caller drives every step, in this order:
///
/// 1. configure the command for a fresh process group
///    ([`super::TreeConfig::configure`] with
///    [`super::TreeIsolation::ProcessGroup`]);
/// 2. [`ForegroundHandoff::prepare`] the command;
/// 3. spawn it **once**, while the guard is alive, and immediately attach the
///    [`super::ProcessTree`];
/// 4. wait for the child, then finish or release the tree and reap the direct
///    child;
/// 5. call [`ForegroundHandoff::restore`] before printing diagnostics or
///    starting further work that uses the terminal.
///
/// Declare the guard *before* the tree, so that when a panic unwinds the
/// caller's scope the tree's cleanup runs first and the terminal is restored
/// afterwards. Reaping stays with the caller: the guard never waits on a
/// process.
///
/// A prepared command spawned more than once, or after its guard is dropped,
/// transfers the foreground to a group the guard no longer restores from.
///
/// ## Inactive guards
///
/// Preparation changes nothing, and returns a guard whose
/// [`is_active`](Self::is_active) is `false`, when the caller has no
/// controlling terminal or is not in the terminal's foreground group (a
/// background job must not take the terminal from the group that has it).
/// Native Windows has no foreground group, so its guard is always inactive and
/// the launch is unchanged.
///
/// ## Errors
///
/// [`prepare`](Self::prepare) returns [`io::ErrorKind::WouldBlock`] while
/// another active handoff exists in this process: a terminal has one
/// foreground group, so a second interactive child would be left in the
/// background. Other terminal query failures are returned unchanged before
/// anything is spawned. The command's spawn fails with
/// [`io::ErrorKind::InvalidInput`] when the command was not configured to lead
/// a fresh process group, and with the transfer's own error when the child
/// cannot take the foreground.
///
/// ## Drop
///
/// Dropping a guard that has not been restored attempts the restoration once,
/// reports a failure through `tracing`, and never panics. It does not run
/// after `std::process::exit`, an abort, or a force-exit, which leave the
/// terminal as the child had it.
pub struct ForegroundHandoff {
    #[cfg(unix)]
    inner: unix::Handoff<unix::LibcTerminal>,
    #[cfg(not(unix))]
    _private: (),
}

impl ForegroundHandoff {
    /// Arranges for `command`'s child to take the terminal's foreground before
    /// it executes, and records the state to restore.
    ///
    /// `command` must already be configured to lead a fresh process group.
    ///
    /// ## Errors
    ///
    /// See [the type's errors](Self#errors).
    #[cfg(unix)]
    pub fn prepare(command: &mut Command) -> io::Result<Self> {
        Ok(Self {
            inner: unix::Handoff::prepare(unix::LibcTerminal, &unix::CLAIM, command)?,
        })
    }

    /// Arranges for `command`'s child to take the terminal's foreground; on
    /// native Windows there is none, so the guard is inactive.
    ///
    /// ## Errors
    ///
    /// Never fails on native Windows.
    #[cfg(not(unix))]
    pub fn prepare(command: &mut Command) -> io::Result<Self> {
        let _ = command;
        Ok(Self { _private: () })
    }

    /// Whether preparation armed a transfer, so that restoration has terminal
    /// state to put back.
    pub fn is_active(&self) -> bool {
        #[cfg(unix)]
        {
            self.inner.is_active()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    /// Puts back the caller's foreground group and terminal settings.
    ///
    /// Idempotent: once restoration has succeeded, or for an inactive guard,
    /// this returns `Ok(())` and changes nothing. Both the foreground group
    /// and the settings are attempted even when the first fails.
    ///
    /// ## Errors
    ///
    /// Returns the first failure. The guard stays armed after a failure, so
    /// dropping it tries once more.
    pub fn restore(&mut self) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.inner.restore()
        }
        #[cfg(not(unix))]
        {
            Ok(())
        }
    }
}

impl std::fmt::Debug for ForegroundHandoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForegroundHandoff").field("active", &self.is_active()).finish()
    }
}

#[cfg(unix)]
mod unix {
    use std::fs::File;
    use std::io;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    use std::os::unix::process::CommandExt;
    use std::process::Command;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Set while an active handoff exists in this process.
    ///
    /// A flag rather than a held `MutexGuard`: a guard holding a `MutexGuard`
    /// could not be sent between threads, and the claim is released by the
    /// handoff's `Drop`, which also runs when a panic unwinds, so there is no
    /// poisoned state to recover.
    pub(super) static CLAIM: AtomicBool = AtomicBool::new(false);

    /// The parent-side terminal and signal-mask operations a handoff performs.
    ///
    /// Production uses [`LibcTerminal`]; the tests script failures through
    /// the same guard code.
    pub(super) trait TerminalOps {
        /// The open terminal, retained until restoration.
        type Terminal;
        /// The terminal settings to restore.
        type Settings;
        /// A thread's saved signal mask.
        type Mask;

        /// Opens the controlling terminal; `Ok(None)` when there is none.
        fn open(&self) -> io::Result<Option<Self::Terminal>>;
        fn host_group(&self) -> libc::pid_t;
        fn foreground_group(&self, terminal: &Self::Terminal) -> io::Result<libc::pid_t>;
        fn settings(&self, terminal: &Self::Terminal) -> io::Result<Self::Settings>;
        fn set_foreground(&self, terminal: &Self::Terminal, group: libc::pid_t) -> io::Result<()>;
        fn set_settings(&self, terminal: &Self::Terminal, settings: &Self::Settings) -> io::Result<()>;
        /// Blocks `SIGTTOU` on the calling thread, returning the mask it had.
        fn block_ttou(&self) -> io::Result<Self::Mask>;
        fn restore_mask(&self, mask: &Self::Mask) -> io::Result<()>;
        /// Registers the child-side transfer on `command`.
        fn register_transfer(&self, command: &mut Command, terminal: &Self::Terminal, host_group: libc::pid_t);
    }

    /// Whether an error means "there is no controlling terminal".
    pub(super) fn is_no_terminal(error: &io::Error) -> bool {
        matches!(error.raw_os_error(), Some(libc::ENOENT | libc::ENXIO | libc::ENOTTY))
    }

    struct Armed<T: TerminalOps> {
        terminal: T::Terminal,
        host_group: libc::pid_t,
        settings: T::Settings,
        claim: &'static AtomicBool,
    }

    pub(super) struct Handoff<T: TerminalOps> {
        ops: T,
        /// `Some` from a successful active preparation until complete
        /// restoration (or drop).
        armed: Option<Armed<T>>,
    }

    impl<T: TerminalOps> Handoff<T> {
        pub(super) fn prepare(ops: T, claim: &'static AtomicBool, command: &mut Command) -> io::Result<Self> {
            if claim.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "another terminal foreground handoff is active in this process",
                ));
            }
            match Self::capture(&ops, command) {
                Ok(Some((terminal, host_group, settings))) => Ok(Self {
                    ops,
                    armed: Some(Armed { terminal, host_group, settings, claim }),
                }),
                Ok(None) => {
                    claim.store(false, Ordering::Release);
                    Ok(Self { ops, armed: None })
                }
                Err(error) => {
                    claim.store(false, Ordering::Release);
                    Err(error)
                }
            }
        }

        #[allow(clippy::type_complexity)]
        fn capture(ops: &T, command: &mut Command) -> io::Result<Option<(T::Terminal, libc::pid_t, T::Settings)>> {
            let terminal = match ops.open() {
                Ok(Some(terminal)) => terminal,
                Ok(None) => return Ok(None),
                Err(error) if is_no_terminal(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            let host_group = ops.host_group();
            let foreground = match ops.foreground_group(&terminal) {
                Ok(group) => group,
                Err(error) if is_no_terminal(&error) => return Ok(None),
                Err(error) => return Err(error),
            };
            if foreground != host_group {
                return Ok(None);
            }
            let settings = ops.settings(&terminal)?;
            ops.register_transfer(command, &terminal, host_group);
            Ok(Some((terminal, host_group, settings)))
        }

        pub(super) fn is_active(&self) -> bool {
            self.armed.is_some()
        }

        pub(super) fn restore(&mut self) -> io::Result<()> {
            let Some(armed) = &self.armed else {
                return Ok(());
            };
            let mask = self.ops.block_ttou()?;
            let foreground = self.ops.set_foreground(&armed.terminal, armed.host_group);
            let settings = self.ops.set_settings(&armed.terminal, &armed.settings);
            let unmask = self.ops.restore_mask(&mask);
            let complete = foreground.is_ok() && settings.is_ok();
            if complete && let Some(armed) = self.armed.take() {
                armed.claim.store(false, Ordering::Release);
            }
            foreground.and(settings).and(unmask)
        }
    }

    impl<T: TerminalOps> std::fmt::Debug for Handoff<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Handoff").field("active", &self.is_active()).finish()
        }
    }

    impl<T: TerminalOps> Drop for Handoff<T> {
        fn drop(&mut self) {
            if self.armed.is_none() {
                return;
            }
            if let Err(error) = self.restore() {
                tracing::warn!(%error, "could not restore the terminal after a foreground handoff");
            }
            if let Some(armed) = self.armed.take() {
                armed.claim.store(false, Ordering::Release);
            }
        }
    }

    /// The real controlling terminal, reached through `/dev/tty`.
    pub(super) struct LibcTerminal;

    fn check(result: libc::c_int) -> io::Result<()> {
        if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }

    fn ttou_set() -> libc::sigset_t {
        // SAFETY: `sigemptyset`/`sigaddset` only fill this local set.
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGTTOU);
            set
        }
    }

    impl TerminalOps for LibcTerminal {
        /// Shared with the child-side hook so the descriptor it uses stays
        /// open for as long as the command can be spawned.
        type Terminal = Arc<File>;
        type Settings = libc::termios;
        type Mask = libc::sigset_t;

        fn open(&self) -> io::Result<Option<Self::Terminal>> {
            // std opens with close-on-exec; `O_NOCTTY` keeps a caller without
            // a controlling terminal from acquiring this one.
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOCTTY)
                .open("/dev/tty")
            {
                Ok(file) => Ok(Some(Arc::new(file))),
                Err(error) if is_no_terminal(&error) => Ok(None),
                Err(error) => Err(error),
            }
        }

        fn host_group(&self) -> libc::pid_t {
            // SAFETY: `getpgrp` has no preconditions.
            unsafe { libc::getpgrp() }
        }

        fn foreground_group(&self, terminal: &Self::Terminal) -> io::Result<libc::pid_t> {
            // SAFETY: a query on a descriptor `terminal` keeps open.
            let group = unsafe { libc::tcgetpgrp(terminal.as_raw_fd()) };
            if group < 0 { Err(io::Error::last_os_error()) } else { Ok(group) }
        }

        fn settings(&self, terminal: &Self::Terminal) -> io::Result<Self::Settings> {
            // SAFETY: `termios` is plain data that `tcgetattr` fills on
            // success; the descriptor is kept open by `terminal`.
            unsafe {
                let mut settings: libc::termios = std::mem::zeroed();
                check(libc::tcgetattr(terminal.as_raw_fd(), &mut settings))?;
                Ok(settings)
            }
        }

        fn set_foreground(&self, terminal: &Self::Terminal, group: libc::pid_t) -> io::Result<()> {
            // SAFETY: plain call on a descriptor `terminal` keeps open.
            check(unsafe { libc::tcsetpgrp(terminal.as_raw_fd(), group) })
        }

        fn set_settings(&self, terminal: &Self::Terminal, settings: &Self::Settings) -> io::Result<()> {
            // `TCSANOW`, not `TCSADRAIN`: restoration must not wait on output
            // a stopped or wedged reader never drains.
            // SAFETY: plain call with settings captured by `tcgetattr`.
            check(unsafe { libc::tcsetattr(terminal.as_raw_fd(), libc::TCSANOW, settings) })
        }

        fn block_ttou(&self) -> io::Result<Self::Mask> {
            let set = ttou_set();
            // SAFETY: valid pointers to local sets. `pthread_sigmask` changes
            // only this thread's mask; the process-wide disposition of
            // `SIGTTOU` is never touched.
            unsafe {
                let mut previous: libc::sigset_t = std::mem::zeroed();
                let result = libc::pthread_sigmask(libc::SIG_BLOCK, &set, &mut previous);
                if result != 0 {
                    return Err(io::Error::from_raw_os_error(result));
                }
                Ok(previous)
            }
        }

        fn restore_mask(&self, mask: &Self::Mask) -> io::Result<()> {
            // SAFETY: restores a mask returned by `block_ttou`.
            let result = unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, mask, std::ptr::null_mut()) };
            if result == 0 { Ok(()) } else { Err(io::Error::from_raw_os_error(result)) }
        }

        fn register_transfer(&self, command: &mut Command, terminal: &Self::Terminal, host_group: libc::pid_t) {
            let terminal = Arc::clone(terminal);
            let ttou = ttou_set();
            // SAFETY: the closure runs in the forked child after std has
            // applied the command's `process_group(0)` (or an earlier
            // `setsid` hook) and cleared the signal mask, and before `exec`.
            // It calls only async-signal-safe functions (`getpid`, `getpgrp`,
            // `sigprocmask`, `tcsetpgrp`) on copied data and a descriptor the
            // parent keeps open; it allocates and locks nothing. Blocking
            // `SIGTTOU` lets the new background group take the foreground
            // without being stopped. Close-on-exec removes the child's copy of
            // the descriptor once the program executes.
            unsafe {
                command.pre_exec(move || {
                    let fd = terminal.as_raw_fd();
                    let pid = libc::getpid();
                    let group = libc::getpgrp();
                    if group <= 0 || group != pid || group == host_group {
                        return Err(io::Error::from_raw_os_error(libc::EINVAL));
                    }
                    let mut previous: libc::sigset_t = std::mem::zeroed();
                    if libc::sigprocmask(libc::SIG_BLOCK, &ttou, &mut previous) != 0 {
                        return Err(io::Error::last_os_error());
                    }
                    let transfer = if libc::tcsetpgrp(fd, group) == 0 {
                        Ok(())
                    } else {
                        Err(io::Error::last_os_error())
                    };
                    let unmask = if libc::sigprocmask(libc::SIG_SETMASK, &previous, std::ptr::null_mut()) == 0 {
                        Ok(())
                    } else {
                        Err(io::Error::last_os_error())
                    };
                    transfer.and(unmask)
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
