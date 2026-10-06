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
    cursor_position_with_timeout(Duration::from_secs(1))
}

/// Query cursor position with a custom timeout.
pub fn cursor_position_with_timeout(timeout: Duration) -> Option<CursorPosition> {
    query_cursor_position(timeout).ok()
}

/// Internal DSR query implementation.
///
/// The exchange goes through the shared `/dev/tty` round trip, so a terminal
/// found silent earlier in the process is not asked again.
#[cfg(unix)]
fn query_cursor_position(timeout: Duration) -> Result<CursorPosition, String> {
    use crate::discovery::detection::is_tty;
    use crate::discovery::os_detection::is_ci;

    if !is_tty() {
        tracing::trace!("DSR cursor position query skipped: not a TTY");
        return Err("not a tty".into());
    }
    if is_ci() {
        tracing::trace!("DSR cursor position query skipped: CI environment");
        return Err("CI environment".into());
    }
    if let Some(multiplexer) = super::osc_queries::query::detect_multiplexer() {
        tracing::trace!(
            multiplexer,
            "DSR cursor position query skipped: terminal multiplexer"
        );
        return Err(format!("terminal multiplexer: {multiplexer}"));
    }

    let response = super::tty_query::round_trip(b"\x1b[6n", timeout).map_err(|e| {
        tracing::trace!("DSR cursor position query failed: {e}");
        e.to_string()
    })?;

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
#[cfg(any(unix, test))]
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
