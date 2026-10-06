//! One bounded round trip to the controlling terminal, shared by every live
//! query (OSC 10/11/12 colors, CSI 14 t window pixels, DSR cursor position).
//!
//! Each request is followed by a DA1 (`CSI c`) sentinel. Terminals process
//! input in order and virtually all of them answer DA1, so the DA1 reply marks
//! the end of whatever the terminal was going to say about the request: a
//! terminal that answers DA1 but not the request is known not to support it
//! without waiting out a timeout.
//!
//! Two process-wide memos keep a non-answering terminal cheap:
//!
//! - **Silent terminal.** When no escape-introduced byte arrives within the
//!   silence budget, the terminal is recorded as silent and every later query
//!   in the process fails immediately without writing anything. A terminal that
//!   answers nothing therefore costs one bounded wait per process, not one per
//!   query.
//! - **Unanswered request.** When DA1 arrives with no reply before it, that
//!   exact request is never written again in this process.
//!
//! Reads block in `select(2)` against a deadline; there is no sleep loop.
//! `select` is used rather than `poll` because macOS `poll` does not support
//! tty devices.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::discovery::raw_mode::{RawModeGuard, TERMINAL_QUERY_MUTEX};

/// The DA1 (primary device attributes) request appended to every query.
pub(crate) const DA1_REQUEST: &[u8] = b"\x1b[c";

/// How long a local terminal may stay completely silent before it is treated
/// as one that answers nothing; this bounds the whole-process cost of a pty
/// nobody reads. WezTerm (20260716 build) was measured answering every query
/// kind, DA1 included, in 170-200 ms from a background pane (2026-10-05), so a
/// budget near that misreads a real terminal as silent and leaks its late
/// reply onto the shell prompt.
pub(crate) const LOCAL_SILENCE_BUDGET: Duration = Duration::from_millis(500);

/// Upper bound on a single read buffer; terminal replies are a few dozen bytes.
const MAX_RESPONSE_BYTES: usize = 4096;

static TERMINAL_SILENT: AtomicBool = AtomicBool::new(false);
static UNANSWERED: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Why a round trip produced no reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TtyQueryError {
    /// The terminal answered nothing within the silence budget, now or
    /// earlier in this process.
    Silent,
    /// The terminal answered DA1 but not the request, now or earlier.
    Unanswered,
    /// The controlling terminal could not be opened, configured, or read.
    Io(String),
}

impl std::fmt::Display for TtyQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Silent => f.write_str("terminal does not answer queries"),
            Self::Unanswered => f.write_str("terminal does not answer this query"),
            Self::Io(e) => f.write_str(e),
        }
    }
}

/// Whether an earlier round trip in this process found the terminal silent.
pub(crate) fn terminal_known_silent() -> bool {
    TERMINAL_SILENT.load(Ordering::Relaxed)
}

/// Write `request` plus the DA1 sentinel to `/dev/tty` and return the bytes
/// the terminal sent before its DA1 reply.
///
/// `timeout` bounds the whole exchange. The wait for the *first* escape byte
/// is further capped by [`silence_budget`], so a silent terminal costs at
/// most that budget. Callers own their preconditions (TTY, CI, multiplexer).
///
/// When the deadline passes after some reply bytes but before DA1 (a terminal
/// that answers the request but not DA1), the collected bytes are returned.
pub(crate) fn round_trip(request: &[u8], timeout: Duration) -> Result<Vec<u8>, TtyQueryError> {
    use std::io::{Read, Write};
    use std::os::unix::io::AsRawFd;

    if terminal_known_silent() {
        return Err(TtyQueryError::Silent);
    }
    if is_memoized_unanswered(request) {
        return Err(TtyQueryError::Unanswered);
    }

    // `/dev/tty`, not stdout: the request must reach the controlling terminal
    // even when stdout is captured, or a wrapper re-emitting the capture later
    // triggers a reply that lands as garbage on the next prompt.
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| TtyQueryError::Io(format!("open /dev/tty: {e}")))?;
    let fd = tty.as_raw_fd();

    let _lock = TERMINAL_QUERY_MUTEX
        .lock()
        .map_err(|_| TtyQueryError::Io("terminal query mutex poisoned".into()))?;
    // Re-check under the lock: a concurrent query may have just found the
    // terminal silent while this one waited.
    if terminal_known_silent() {
        return Err(TtyQueryError::Silent);
    }
    let _guard = RawModeGuard::new(fd).map_err(TtyQueryError::Io)?;

    let mut wire = Vec::with_capacity(request.len() + DA1_REQUEST.len());
    wire.extend_from_slice(request);
    wire.extend_from_slice(DA1_REQUEST);
    tty.write_all(&wire)
        .and_then(|()| tty.flush())
        .map_err(|e| TtyQueryError::Io(e.to_string()))?;

    let start = Instant::now();
    let deadline = start + timeout;
    let silence_deadline = start + silence_budget(timeout);
    let mut response: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 256];

    loop {
        if let Some(da1_at) = find_da1_reply(&response) {
            response.truncate(da1_at);
            if !response.contains(&0x1b) {
                memoize_unanswered(request);
                return Err(TtyQueryError::Unanswered);
            }
            return Ok(response);
        }
        // Stray non-escape bytes (typeahead, a `^D` some pty hosts inject on
        // EOF) are not evidence that the terminal is listening.
        let heard = response.contains(&0x1b);
        let limit = if heard {
            deadline
        } else {
            silence_deadline.min(deadline)
        };
        let now = Instant::now();
        if now >= limit || response.len() >= MAX_RESPONSE_BYTES {
            break;
        }
        match wait_readable(fd, limit - now) {
            Ok(true) => match tty.read(&mut buffer) {
                Ok(n) if n > 0 => response.extend_from_slice(&buffer[..n]),
                // Readable but zero bytes is hangup; waiting cannot help.
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(TtyQueryError::Io(e.to_string())),
            },
            Ok(false) => {}
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(TtyQueryError::Io(e.to_string())),
        }
    }

    if response.contains(&0x1b) {
        // Answered something, but not DA1: hand back what arrived.
        return Ok(response);
    }
    TERMINAL_SILENT.store(true, Ordering::Relaxed);
    tracing::debug!(
        waited_ms = start.elapsed().as_millis() as u64,
        "terminal answered nothing; skipping further terminal queries this process"
    );
    Err(TtyQueryError::Silent)
}

/// Environment variable overriding [`LOCAL_SILENCE_BUDGET`], in milliseconds.
pub(crate) const SILENCE_BUDGET_ENV: &str = "BISCUIT_TERMINAL_QUERY_SILENCE_MS";

/// The longest a query waits for the terminal's first escape byte, never more
/// than `timeout`.
///
/// [`SILENCE_BUDGET_ENV`] wins when it parses. Otherwise remote sessions keep
/// the caller's full timeout, since a network round trip can legitimately
/// exceed the local budget.
fn silence_budget(timeout: Duration) -> Duration {
    use crate::discovery::detection::{Connection, detect_connection};
    if let Some(ms) = std::env::var(SILENCE_BUDGET_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
    {
        return timeout.min(Duration::from_millis(ms));
    }
    match detect_connection() {
        Connection::Local => timeout.min(LOCAL_SILENCE_BUDGET),
        _ => timeout,
    }
}

fn is_memoized_unanswered(request: &[u8]) -> bool {
    UNANSWERED
        .lock()
        .map(|seen| seen.iter().any(|r| r == request))
        .unwrap_or(false)
}

fn memoize_unanswered(request: &[u8]) {
    if let Ok(mut seen) = UNANSWERED.lock()
        && !seen.iter().any(|r| r == request)
    {
        seen.push(request.to_vec());
    }
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

/// Index of the first complete DA1 reply (`ESC [ ? <digits;> c`), if any.
pub(crate) fn find_da1_reply(bytes: &[u8]) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = bytes[from..].windows(3).position(|w| w == b"\x1b[?") {
        let start = from + rel;
        let params = &bytes[start + 3..];
        let len = params
            .iter()
            .take_while(|b| b.is_ascii_digit() || **b == b';')
            .count();
        if params.get(len) == Some(&b'c') {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn da1_reply_is_found_after_a_query_reply() {
        let bytes = b"\x1b]11;rgb:0000/0000/0000\x07\x1b[?62;22c";
        assert_eq!(find_da1_reply(bytes), Some(24));
    }

    #[test]
    fn da1_reply_alone_is_found_at_start() {
        assert_eq!(find_da1_reply(b"\x1b[?1;2c"), Some(0));
    }

    #[test]
    fn incomplete_or_foreign_csi_is_not_da1() {
        assert_eq!(find_da1_reply(b"\x1b[?62;22"), None);
        assert_eq!(find_da1_reply(b"\x1b[12;34R"), None);
        assert_eq!(find_da1_reply(b"\x1b[?2027;1$y"), None);
    }

    #[test]
    fn unanswered_memo_matches_exact_request_only() {
        let request = b"\x1b[test-memo-unique t";
        assert!(!is_memoized_unanswered(request));
        memoize_unanswered(request);
        assert!(is_memoized_unanswered(request));
        assert!(!is_memoized_unanswered(b"\x1b[other t"));
    }
}
