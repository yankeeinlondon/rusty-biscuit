//! Cursor position query using DSR (Device Status Report).
//!
//! Sends `ESC[6n` and parses the terminal's `ESC[{row};{col}R` response
//! to determine the current cursor position. This is useful for diagnostic
//! and calibration purposes in image rendering.
//!
//! ## Limitations
//!
//! - Unix only (requires raw mode terminal access)
//! - Skipped in CI environments and multiplexers
//! - Requires a real TTY (not a pipe)

use std::time::Duration;

/// Cursor position as reported by the terminal via DSR.
///
/// Row and column are 1-based (matching the terminal's CSI coordinate system).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    /// Row (1-based, top of screen = 1)
    pub row: u32,
    /// Column (1-based, left edge = 1)
    pub col: u32,
}

/// Query the current cursor position from the terminal.
///
/// Sends DSR (`ESC[6n`) and parses the CPR (`ESC[row;colR`) response.
/// Uses the same raw mode / mutex infrastructure as OSC color queries.
///
/// ## Returns
///
/// `Some(CursorPosition)` on success, `None` if the query fails or times out.
pub fn cursor_position() -> Option<CursorPosition> {
    cursor_position_with_timeout(Duration::from_millis(100))
}

/// Query cursor position with a custom timeout.
pub fn cursor_position_with_timeout(timeout: Duration) -> Option<CursorPosition> {
    query_cursor_position(timeout).ok()
}

/// Internal DSR query implementation.
#[cfg(unix)]
fn query_cursor_position(timeout: Duration) -> Result<CursorPosition, String> {
    use crate::discovery::detection::is_tty;
    use crate::discovery::os_detection::is_ci;
    use std::io::{Read, Write};

    if !is_tty() {
        tracing::trace!("DSR cursor position query skipped: not a TTY");
        return Err("not a tty".into());
    }
    if is_ci() {
        tracing::trace!("DSR cursor position query skipped: CI environment");
        return Err("CI environment".into());
    }

    // RAII guard for raw mode
    struct RawModeGuard {
        original: libc::termios,
        fd: libc::c_int,
    }

    impl RawModeGuard {
        fn new() -> Result<Self, String> {
            let fd = libc::STDIN_FILENO;
            let mut original: libc::termios = unsafe { std::mem::zeroed() };
            if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
                tracing::trace!("DSR cursor position query failed: tcgetattr error");
                return Err("failed to get terminal attributes".into());
            }
            let mut raw = original;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 1;
            if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
                tracing::trace!("DSR cursor position query failed: tcsetattr error");
                return Err("failed to set raw mode".into());
            }
            Ok(Self { original, fd })
        }
    }

    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            unsafe {
                libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
            }
        }
    }

    let _guard = RawModeGuard::new()?;

    // Send DSR: ESC[6n
    let mut stdout = std::io::stdout();
    stdout.write_all(b"\x1b[6n").map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())?;

    // Read response: ESC[{row};{col}R
    let mut buffer = [0u8; 32];
    let mut response = Vec::new();
    let start = std::time::Instant::now();
    let mut stdin = std::io::stdin();

    while start.elapsed() < timeout {
        match stdin.read(&mut buffer) {
            Ok(0) => std::thread::sleep(Duration::from_millis(5)),
            Ok(n) => {
                response.extend_from_slice(&buffer[..n]);
                if response.contains(&b'R') {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    parse_cpr_response(&response).ok_or_else(|| {
        tracing::trace!(
            response = ?String::from_utf8_lossy(&response),
            "DSR cursor position query failed: could not parse CPR response"
        );
        "failed to parse CPR response".into()
    })
}

#[cfg(not(unix))]
fn query_cursor_position(_timeout: Duration) -> Result<CursorPosition, String> {
    tracing::trace!("DSR cursor position query: not supported on this platform");
    Err("cursor position query not supported on this platform".into())
}

/// Parse a CPR (Cursor Position Report) response.
///
/// Expected format: `ESC[{row};{col}R`
/// Searches backwards from the `R` terminator to handle echoed query noise.
fn parse_cpr_response(data: &[u8]) -> Option<CursorPosition> {
    let text = std::str::from_utf8(data).ok()?;

    // Find the R terminator, then scan backwards for the nearest ESC[
    let r_pos = text.rfind('R')?;
    let before_r = &text[..r_pos];
    let esc_pos = before_r.rfind("\x1b[")?;
    let params = &text[esc_pos + 2..r_pos];

    let mut parts = params.split(';');
    let row: u32 = parts.next()?.parse().ok()?;
    let col: u32 = parts.next()?.parse().ok()?;

    Some(CursorPosition { row, col })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpr_response_valid() {
        let response = b"\x1b[15;1R";
        let pos = parse_cpr_response(response).unwrap();
        assert_eq!(pos.row, 15);
        assert_eq!(pos.col, 1);
    }

    #[test]
    fn test_parse_cpr_response_large_values() {
        let response = b"\x1b[100;200R";
        let pos = parse_cpr_response(response).unwrap();
        assert_eq!(pos.row, 100);
        assert_eq!(pos.col, 200);
    }

    #[test]
    fn test_parse_cpr_response_with_prefix_noise() {
        // Terminal may echo back the query before the response
        let response = b"\x1b[6n\x1b[25;80R";
        let pos = parse_cpr_response(response).unwrap();
        assert_eq!(pos.row, 25);
        assert_eq!(pos.col, 80);
    }

    #[test]
    fn test_parse_cpr_response_invalid() {
        assert!(parse_cpr_response(b"garbage").is_none());
        assert!(parse_cpr_response(b"\x1b[R").is_none());
        assert!(parse_cpr_response(b"").is_none());
    }

    #[test]
    fn test_parse_cpr_response_single_digit() {
        let response = b"\x1b[1;1R";
        let pos = parse_cpr_response(response).unwrap();
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 1);
    }
}
