//! What the host can lend an editor: its terminal, or only a display.
//!
//! A terminal editor needs the host's one interactive terminal for as long as
//! it runs. Three things decide whether it may have it: what the process can
//! see ([`EnvironmentProbe`]), whether the calling thread was told it has no
//! terminal of its own ([`deny_terminal_in_this_thread`], entered by each
//! parallel group member), and whether another editor already holds the
//! terminal (a process-wide guard).

use std::cell::Cell;
use std::io::IsTerminal as _;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

use sniff::os::DisplaySession;

thread_local! {
    static TERMINAL_DENIED: Cell<bool> = const { Cell::new(false) };
}

/// Restores the calling thread's previous terminal permission when dropped.
///
/// Not `Send`: the permission belongs to the thread that entered it.
#[must_use = "the denial ends when the scope is dropped"]
pub struct TerminalDenyScope {
    previous: bool,
    _thread_bound: PhantomData<*const ()>,
}

impl Drop for TerminalDenyScope {
    fn drop(&mut self) {
        TERMINAL_DENIED.with(|denied| denied.set(self.previous));
    }
}

/// Refuses terminal editors on this thread until the scope is dropped.
///
/// A parallel group member runs beside others that share the parent's
/// terminal, so it has none to lend, even when the process has one. A
/// nested serial stack on the same thread inherits the denial; a fresh
/// thread starts permitted, so a member must enter this explicitly.
pub fn deny_terminal_in_this_thread() -> TerminalDenyScope {
    let previous = TERMINAL_DENIED.with(|denied| denied.replace(true));
    TerminalDenyScope { previous, _thread_bound: PhantomData }
}

/// Whether [`deny_terminal_in_this_thread`] is in effect on this thread.
pub fn terminal_denied() -> bool {
    TERMINAL_DENIED.with(Cell::get)
}

/// Held while one terminal editor owns the host's terminal.
static INTERACTIVE: AtomicBool = AtomicBool::new(false);

/// The process's exclusive claim on its interactive terminal, released on
/// drop.
pub(crate) struct InteractiveSurface(());

impl InteractiveSurface {
    /// `None` while another editor holds the terminal.
    pub(crate) fn claim() -> Option<Self> {
        INTERACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self(()))
    }
}

impl Drop for InteractiveSurface {
    fn drop(&mut self) {
        INTERACTIVE.store(false, Ordering::Release);
    }
}

/// What this process can offer an editor, observed once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentProbe {
    pub stdin_tty: bool,
    pub stdout_tty: bool,
    pub stderr_tty: bool,
    /// The process group owns the terminal's foreground (always `true` on
    /// Windows, which has no terminal process groups).
    pub foreground: bool,
    pub display: DisplaySession,
}

impl EnvironmentProbe {
    /// Observes the current process.
    pub fn process() -> Self {
        Self {
            stdin_tty: std::io::stdin().is_terminal(),
            stdout_tty: std::io::stdout().is_terminal(),
            stderr_tty: std::io::stderr().is_terminal(),
            foreground: in_terminal_foreground(),
            display: sniff::os::detect_display_session(),
        }
    }
}

#[cfg(unix)]
fn in_terminal_foreground() -> bool {
    // SAFETY: plain queries on fd 0 and the calling process. `tcgetpgrp` is
    // `-1` when stdin is not a terminal, which never equals a group ID.
    unsafe { libc::tcgetpgrp(libc::STDIN_FILENO) == libc::getpgrp() }
}

#[cfg(not(unix))]
fn in_terminal_foreground() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denial_is_scoped_nested_and_per_thread() {
        assert!(!terminal_denied());
        {
            let _outer = deny_terminal_in_this_thread();
            assert!(terminal_denied());
            {
                let _inner = deny_terminal_in_this_thread();
                assert!(terminal_denied());
            }
            assert!(terminal_denied(), "the inner scope restores the outer denial");
            std::thread::scope(|scope| {
                scope.spawn(|| assert!(!terminal_denied(), "a fresh thread starts permitted"));
            });
        }
        assert!(!terminal_denied());
    }

    #[test]
    fn the_interactive_surface_is_exclusive_until_released() {
        let held = InteractiveSurface::claim().expect("free at first");
        assert!(InteractiveSurface::claim().is_none());
        drop(held);
        assert!(InteractiveSurface::claim().is_some());
    }
}
