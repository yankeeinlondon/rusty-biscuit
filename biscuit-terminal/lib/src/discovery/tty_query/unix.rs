//! The Unix [`QueryPort`]: `/dev/tty` in raw mode, read through `select(2)`.
//!
//! `select` is used rather than `poll` because macOS `poll` does not support
//! tty devices. Reads block against a deadline; there is no sleep loop.

use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use super::{QueryPort, ReadOutcome, TtyQueryError};
use crate::discovery::raw_mode::RawModeGuard;

pub(super) struct UnixPort {
    tty: File,
    // Dropped after nothing reads `tty` any more, restoring its modes.
    _guard: RawModeGuard,
}

/// Opens `/dev/tty`, not stdout: the request must reach the controlling
/// terminal even when stdout is captured, or a wrapper re-emitting the
/// capture later triggers a reply that lands as garbage on the next prompt.
pub(super) fn open_port() -> Result<UnixPort, TtyQueryError> {
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| TtyQueryError::Io(format!("open /dev/tty: {e}")))?;
    let guard = RawModeGuard::new(tty.as_raw_fd()).map_err(TtyQueryError::Io)?;
    Ok(UnixPort { tty, _guard: guard })
}

impl QueryPort for UnixPort {
    fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.tty.write_all(bytes)?;
        self.tty.flush()
    }

    fn read_within(&mut self, buffer: &mut [u8], timeout: Duration) -> std::io::Result<ReadOutcome> {
        if !wait_readable(self.tty.as_raw_fd(), timeout)? {
            return Ok(ReadOutcome::TimedOut);
        }
        match self.tty.read(buffer)? {
            // Readable but zero bytes is hangup.
            0 => Ok(ReadOutcome::Hangup),
            n => Ok(ReadOutcome::Data(n)),
        }
    }

    fn input_pending(&mut self) -> std::io::Result<bool> {
        wait_readable(self.tty.as_raw_fd(), Duration::ZERO)
    }

    /// A tty cannot take input back; typed keys read during an exchange are
    /// lost, as they always have been for the color query.
    fn return_typeahead(&mut self, _response: &[u8], _typeahead: &[std::ops::Range<usize>]) {}
}

/// Block until `fd` is readable or `timeout` elapses. `Ok(false)` is a timeout.
fn wait_readable(fd: libc::c_int, timeout: Duration) -> std::io::Result<bool> {
    if fd < 0 || fd as usize >= libc::FD_SETSIZE {
        return Err(std::io::Error::other("tty descriptor out of select range"));
    }
    // SAFETY: `set` is a plain bitset zero-initialised by FD_ZERO, `fd` is in
    // range (checked above), and every pointer passed to select is valid for
    // the duration of the call.
    unsafe {
        let mut set: libc::fd_set = std::mem::zeroed();
        libc::FD_ZERO(&mut set);
        libc::FD_SET(fd, &mut set);
        let mut tv = libc::timeval {
            tv_sec: timeout.as_secs() as libc::time_t,
            tv_usec: timeout.subsec_micros() as libc::suseconds_t,
        };
        let rc = libc::select(
            fd + 1,
            &mut set,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut tv,
        );
        match rc {
            -1 => Err(std::io::Error::last_os_error()),
            0 => Ok(false),
            _ => Ok(true),
        }
    }
}
