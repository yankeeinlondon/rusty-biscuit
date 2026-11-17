//! Shared raw terminal mode infrastructure.
//!
//! Provides an RAII guard for entering/exiting raw terminal mode and a global
//! mutex to serialize all terminal queries (OSC, DSR, CSI 14 t, etc.).

use std::sync::Mutex;

/// Global mutex to serialize terminal queries.
///
/// Multiple concurrent queries (OSC color probes, DSR cursor position,
/// CSI 14 t window size) would race on stdin/stdout and corrupt responses.
/// All query functions must hold this lock.
pub static TERMINAL_QUERY_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard that enters raw terminal mode on creation and restores the
/// original terminal state on drop.
///
/// ## Errors
///
/// Returns `Err` if the caller is not in the terminal's foreground process
/// group, or if `tcgetattr` or `tcsetattr` fails (e.g., not a real TTY).
#[cfg(unix)]
pub struct RawModeGuard {
    original: libc::termios,
    fd: libc::c_int,
}

#[cfg(unix)]
impl RawModeGuard {
    /// Enter raw mode on the given file descriptor.
    ///
    /// Disables canonical mode and echo. Sets VMIN=0 and VTIME=1 (100ms read timeout).
    ///
    /// ## Errors
    ///
    /// Returns `Err`, changing nothing, when the terminal's foreground process
    /// group is not the caller's or cannot be read (`tcgetpgrp` fails, as it
    /// does for a terminal that is not the controlling one): a background job
    /// that changes the terminal's settings is stopped by `SIGTTOU`, and one
    /// that reads it by `SIGTTIN`, so an optional probe would suspend its
    /// whole command instead of falling back.
    ///
    /// Returns `Err` if `tcgetattr` or `tcsetattr` fails. Also returns `Err` on WSL 1,
    /// where `tcsetattr` reports success but the Windows console layer keeps stdin in
    /// cooked mode — any subsequent `read()` would block until the user presses Enter.
    /// Callers relying on this guard for a bounded probe should treat the error as a
    /// signal to fall back to heuristic detection.
    pub fn new(fd: libc::c_int) -> Result<Self, String> {
        if crate::discovery::os_detection::is_wsl1() {
            return Err("WSL 1 does not honor raw-mode termios flags".into());
        }
        ensure_foreground(fd)?;
        let mut original: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
            return Err("failed to get terminal attributes".into());
        }
        let mut raw = original;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO);
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 1;
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
            return Err("failed to set raw mode".into());
        }
        Ok(Self { original, fd })
    }

    /// Enter raw mode on stdin (`STDIN_FILENO`).
    pub fn stdin() -> Result<Self, String> {
        Self::new(libc::STDIN_FILENO)
    }
}

/// `tcgetpgrp` raises no job-control signal, even from a background group.
#[cfg(unix)]
fn ensure_foreground(fd: libc::c_int) -> Result<(), String> {
    // SAFETY: both calls only read process and terminal state.
    let (foreground, own) = unsafe { (libc::tcgetpgrp(fd), libc::getpgrp()) };
    if foreground == -1 {
        return Err(format!(
            "terminal foreground group unknown: {}",
            std::io::Error::last_os_error()
        ));
    }
    if foreground != own {
        return Err("process is not in the terminal's foreground group".into());
    }
    Ok(())
}

#[cfg(unix)]
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    type Settings = (
        libc::tcflag_t,
        libc::tcflag_t,
        libc::tcflag_t,
        libc::tcflag_t,
        [libc::cc_t; libc::NCCS],
    );

    /// The settings without `PENDIN`, which the kernel sets itself when
    /// canonical mode is turned back on, so no one restores it.
    fn settings(fd: libc::c_int) -> Option<Settings> {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        // SAFETY: `t` is a valid termios for tcgetattr to fill.
        (unsafe { libc::tcgetattr(fd, &mut t) } == 0).then_some((
            t.c_iflag,
            t.c_oflag,
            t.c_cflag,
            t.c_lflag & !libc::PENDIN,
            t.c_cc,
        ))
    }

    /// A fresh pty pair; nothing has made it a controlling terminal.
    fn open_pty() -> (libc::c_int, libc::c_int) {
        let (mut master, mut slave) = (-1, -1);
        // SAFETY: out-pointers are valid; name, termios, and winsize may be null.
        let rc = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(rc, 0, "openpty: {}", std::io::Error::last_os_error());
        (master, slave)
    }

    fn close(fds: &[libc::c_int]) {
        for fd in fds {
            // SAFETY: each descriptor came from `open_pty` and is closed once.
            unsafe { libc::close(*fd) };
        }
    }

    #[test]
    fn a_terminal_whose_foreground_cannot_be_read_is_refused_unchanged() {
        let (master, slave) = open_pty();
        let before = settings(slave).expect("tcgetattr on the slave");

        let result = RawModeGuard::new(slave);

        assert!(
            matches!(&result, Err(e) if e.starts_with("terminal foreground group unknown")),
            "{:?}",
            result.as_ref().err()
        );
        assert_eq!(settings(slave), Some(before), "settings unchanged");
        drop(result);
        close(&[master, slave]);
    }

    /// Runs in a forked child: makes the slave its controlling terminal, so
    /// its own group is the foreground, enters raw mode there, then checks a
    /// grandchild in a group of its own is refused. Returns an exit code.
    fn foreground_child(slave: libc::c_int) -> i32 {
        // SAFETY: plain process and terminal syscalls in a single-threaded
        // forked child.
        unsafe {
            if libc::setsid() == -1 {
                return 10;
            }
            if libc::ioctl(slave, libc::TIOCSCTTY as _, 0) == -1 {
                return 11;
            }
            if libc::tcgetpgrp(slave) != libc::getpgrp() {
                return 12;
            }
        }
        let Some(before) = settings(slave) else { return 13 };
        match RawModeGuard::new(slave) {
            Err(_) => return 14,
            Ok(guard) => {
                if settings(slave).is_none_or(|s| s.3 & libc::ICANON != 0) {
                    return 15;
                }
                drop(guard);
            }
        }
        if settings(slave) != Some(before) {
            return 16;
        }

        // SAFETY: as above; the grandchild only calls the guard and exits.
        unsafe {
            let grandchild = libc::fork();
            if grandchild == 0 {
                libc::setpgid(0, 0);
                let code = match RawModeGuard::new(slave) {
                    Ok(_) => 20,
                    Err(_) if settings(slave) != Some(before) => 21,
                    Err(_) => 0,
                };
                libc::_exit(code);
            }
            if grandchild == -1 {
                return 17;
            }
            let mut status = 0;
            libc::waitpid(grandchild, &mut status, libc::WUNTRACED);
            if libc::WIFSTOPPED(status) {
                // Pre-fix behavior: the background tcsetattr raised SIGTTOU.
                libc::kill(grandchild, libc::SIGKILL);
                libc::waitpid(grandchild, &mut status, 0);
                return 22;
            }
            if libc::WIFEXITED(status) {
                libc::WEXITSTATUS(status)
            } else {
                23
            }
        }
    }

    #[test]
    fn raw_mode_is_entered_from_the_foreground_and_refused_from_the_background() {
        let (master, slave) = open_pty();
        // Initialize the cached WSL probe before fork, so the children only
        // make syscalls.
        crate::discovery::os_detection::is_wsl1();

        // SAFETY: the child runs `foreground_child` and `_exit`s.
        let child = unsafe { libc::fork() };
        assert_ne!(child, -1, "fork: {}", std::io::Error::last_os_error());
        if child == 0 {
            let code = foreground_child(slave);
            // SAFETY: leave the forked child without running test-harness code.
            unsafe { libc::_exit(code) };
        }
        let mut status = 0;
        // SAFETY: waits for the child forked above.
        unsafe { libc::waitpid(child, &mut status, 0) };
        close(&[master, slave]);

        assert!(libc::WIFEXITED(status), "child ended by a signal: {status:#x}");
        let code = libc::WEXITSTATUS(status);
        let meaning = match code {
            0 => "ok",
            10..=13 => "could not set up the pty as the child's controlling terminal",
            14 => "the foreground group was refused raw mode",
            15 => "raw mode did not clear ICANON",
            16 => "dropping the guard did not restore the settings",
            20 => "a background group entered raw mode",
            21 => "refusing a background group changed the settings",
            22 => "a background group was stopped by SIGTTOU",
            _ => "unexpected child failure",
        };
        assert_eq!(code, 0, "{meaning}");
    }
}
