//! Shared raw terminal mode infrastructure.
//!
//! Provides an RAII guard for entering/exiting raw terminal mode and a global
//! mutex to serialize all terminal queries (OSC, DSR, CSI 14 t, etc.).

#[cfg(unix)]
use std::sync::Mutex;

/// Global mutex to serialize terminal queries.
///
/// Multiple concurrent queries (OSC color probes, DSR cursor position,
/// CSI 14 t window size) would race on stdin/stdout and corrupt responses.
/// All query functions must hold this lock.
#[cfg(unix)]
pub static TERMINAL_QUERY_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard that enters raw terminal mode on creation and restores the
/// original terminal state on drop.
///
/// ## Errors
///
/// Returns `Err` if `tcgetattr` or `tcsetattr` fails (e.g., not a real TTY).
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
    pub fn new(fd: libc::c_int) -> Result<Self, String> {
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

#[cfg(unix)]
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}
