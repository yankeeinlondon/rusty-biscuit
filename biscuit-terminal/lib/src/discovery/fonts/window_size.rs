//! Window pixel size and cell size queries via CSI 14 t.

use super::types::{CellSize, WindowSizePixels};
#[cfg(unix)]
use crate::discovery::detection::is_tty;

/// Get the terminal window size in pixels.
///
/// This function queries the window pixel dimensions using the
/// XTWINOPS escape sequence (CSI 14 t).
///
/// ## Detection Method
///
/// The CSI 14 t escape sequence queries the terminal for its window size
/// in pixels. The terminal responds with a sequence like:
/// `\033[4;height;widtht`
///
/// ## Returns
///
/// - `Some(WindowSizePixels)` if window pixel size can be detected
/// - `None` if detection fails (not a TTY, terminal doesn't support CSI 14 t, timeout)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::window_size_pixels;
///
/// if let Some(size) = window_size_pixels() {
///     println!("Window size: {}x{} pixels", size.width, size.height);
/// } else {
///     println!("Could not detect window pixel size");
/// }
/// ```
#[cfg(unix)]
pub fn window_size_pixels() -> Option<WindowSizePixels> {
    use crate::discovery::raw_mode::{RawModeGuard, TERMINAL_QUERY_MUTEX};
    use std::io::{Read, Write};
    use std::os::unix::io::AsRawFd;
    use std::time::{Duration, Instant};

    // Must be a TTY to query
    if !is_tty() {
        tracing::trace!("window_size_pixels(): not a TTY");
        return None;
    }

    // Skip terminal queries in CI environments (no real terminal available)
    if crate::discovery::os_detection::is_ci() {
        tracing::trace!("window_size_pixels(): skipping in CI environment");
        return None;
    }

    // Open /dev/tty for direct terminal access
    let mut tty = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        Ok(f) => f,
        Err(e) => {
            tracing::trace!("window_size_pixels(): failed to open /dev/tty: {}", e);
            return None;
        }
    };

    let fd = tty.as_raw_fd();

    // Serialize terminal access to prevent race conditions
    let _lock = TERMINAL_QUERY_MUTEX.lock().ok()?;

    // Enter raw mode on /dev/tty fd (RAII guard restores on drop)
    let _guard = RawModeGuard::new(fd).ok()?;

    // Write CSI 14 t query
    let query = b"\x1b[14t";
    if tty.write_all(query).is_err() {
        tracing::trace!("window_size_pixels(): failed to write query");
        return None;
    }
    let _ = tty.flush();

    // Read response with timeout and hard limits to prevent hangs
    let timeout = Duration::from_millis(100);
    let start = Instant::now();
    let mut buffer = Vec::with_capacity(32);
    let mut byte = [0u8; 1];
    const MAX_BYTES: usize = 64;
    const MAX_ITERATIONS: usize = 100;
    let mut iterations = 0;

    while start.elapsed() < timeout && buffer.len() < MAX_BYTES && iterations < MAX_ITERATIONS {
        iterations += 1;
        match tty.read(&mut byte) {
            Ok(1) => {
                buffer.push(byte[0]);
                // Check if we've received the terminating 't'
                if byte[0] == b't' && buffer.len() > 4 {
                    break;
                }
            }
            Ok(0) => {
                // No data available, continue waiting
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    // Parse response: \x1b[4;height;widtht
    let result = parse_csi_14t_response(&buffer);
    if result.is_some() {
        tracing::debug!("window_size_pixels(): detected {:?}", result);
    } else {
        tracing::debug!(
            "window_size_pixels(): failed to parse response: {:?}",
            String::from_utf8_lossy(&buffer)
        );
    }

    result
}

/// Get the terminal window size in pixels (non-Unix stub).
#[cfg(not(unix))]
pub fn window_size_pixels() -> Option<WindowSizePixels> {
    tracing::trace!("window_size_pixels(): not supported on this platform");
    None
}

/// Parse the CSI 14 t response format.
///
/// Expected format: `\x1b[4;height;widtht`
#[cfg(any(unix, test))]
fn parse_csi_14t_response(response: &[u8]) -> Option<WindowSizePixels> {
    // Find the CSI sequence start
    let esc_pos = response.iter().position(|&b| b == 0x1b)?;
    let after_esc = &response[esc_pos..];

    // Validate CSI format: ESC [ 4 ; ... t
    if after_esc.len() < 5 {
        return None;
    }
    if after_esc[1] != b'[' {
        return None;
    }
    if after_esc[2] != b'4' {
        return None;
    }
    if after_esc[3] != b';' {
        return None;
    }

    // Find the terminating 't'
    let t_pos = after_esc.iter().position(|&b| b == b't')?;
    let params = &after_esc[4..t_pos];

    // Parse "height;width" from params
    let params_str = std::str::from_utf8(params).ok()?;
    let parts: Vec<&str> = params_str.split(';').collect();

    if parts.len() != 2 {
        return None;
    }

    let height: u32 = parts[0].parse().ok()?;
    let width: u32 = parts[1].parse().ok()?;

    Some(WindowSizePixels { width, height })
}

/// Divide window pixel dimensions by the character grid to yield cell size.
///
/// Returns `None` if any dimension is zero, which guards against both an
/// unpopulated pixel size (terminal reports `0` for `ws_xpixel`/`ws_ypixel`)
/// and a divide-by-zero on the grid.
fn cell_from_pixels(xpixel: u32, ypixel: u32, cols: u32, rows: u32) -> Option<CellSize> {
    if xpixel == 0 || ypixel == 0 || cols == 0 || rows == 0 {
        return None;
    }
    Some(CellSize {
        width: xpixel / cols,
        height: ypixel / rows,
    })
}

/// Read cell size from the `TIOCGWINSZ` ioctl.
///
/// The `winsize` struct carries pixel **and** grid dimensions in one atomic
/// kernel read, so the cell size is computed from a self-consistent snapshot —
/// no escape sequence, no read timeout, and no chance of a resize landing
/// between two separate queries. Terminals that populate `ws_xpixel`/`ws_ypixel`
/// (iTerm2, kitty, ghostty, WezTerm, Warp, Apple Terminal) resolve here; those
/// that leave them at `0` fall through to the CSI 14 t query.
#[cfg(unix)]
fn cell_size_ioctl() -> Option<CellSize> {
    use std::os::unix::io::AsRawFd;

    if !is_tty() {
        return None;
    }

    // Target the controlling terminal directly so the ioctl is correct even
    // when stdout/stderr are piped.
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .ok()?;

    // SAFETY: an all-zero `winsize` is a valid initial value; `ioctl` either
    // overwrites it on success or leaves it untouched on the error path we
    // discard below.
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::ioctl(tty.as_raw_fd(), libc::TIOCGWINSZ, &mut ws) };
    if rc != 0 {
        return None;
    }

    cell_from_pixels(
        ws.ws_xpixel as u32,
        ws.ws_ypixel as u32,
        ws.ws_col as u32,
        ws.ws_row as u32,
    )
}

#[cfg(not(unix))]
fn cell_size_ioctl() -> Option<CellSize> {
    None
}

/// Calculate the cell size (font dimensions) in pixels.
///
/// Prefers the synchronous `TIOCGWINSZ` ioctl, which is deterministic and free
/// of the read-timeout race inherent to the CSI 14 t escape query. Only when the
/// ioctl leaves the pixel fields unpopulated does this fall back to querying the
/// terminal with [`window_size_pixels`].
///
/// ## Returns
///
/// - `Some(CellSize)` if pixel and grid dimensions can be determined
/// - `None` if neither source yields a usable measurement
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::cell_size;
///
/// if let Some(size) = cell_size() {
///     println!("Cell size: {}x{} pixels", size.width, size.height);
/// }
/// ```
pub fn cell_size() -> Option<CellSize> {
    if let Some(cs) = cell_size_ioctl() {
        return Some(cs);
    }

    let window = window_size_pixels()?;
    let cols = crate::discovery::detection::terminal_width();
    let rows = crate::discovery::detection::terminal_height();
    cell_from_pixels(window.width, window.height, cols, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csi_14t_response_valid() {
        // Standard response: ESC[4;1080;1920t
        let response = b"\x1b[4;1080;1920t";
        let result = parse_csi_14t_response(response);
        assert_eq!(
            result,
            Some(WindowSizePixels {
                width: 1920,
                height: 1080
            })
        );
    }

    #[test]
    fn test_parse_csi_14t_response_with_prefix() {
        // Response may have garbage before it
        let response = b"garbage\x1b[4;600;800t";
        let result = parse_csi_14t_response(response);
        assert_eq!(
            result,
            Some(WindowSizePixels {
                width: 800,
                height: 600
            })
        );
    }

    #[test]
    fn test_parse_csi_14t_response_empty() {
        let response = b"";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_no_esc() {
        let response = b"[4;100;200t";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_wrong_command() {
        // CSI 5 instead of CSI 4
        let response = b"\x1b[5;100;200t";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_missing_semicolon() {
        let response = b"\x1b[4;100t";
        // Only one number, should fail
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_parse_csi_14t_response_non_numeric() {
        let response = b"\x1b[4;abc;deft";
        assert_eq!(parse_csi_14t_response(response), None);
    }

    #[test]
    fn test_cell_size_does_not_panic() {
        // cell_size() should not panic regardless of environment
        let _ = cell_size();
    }

    #[test]
    fn test_cell_from_pixels_divides_window_by_grid() {
        // 1920x1080 over a 120x40 grid -> 16x27 cells. This is the ioctl
        // happy path the squished-graph fix relies on.
        assert_eq!(
            cell_from_pixels(1920, 1080, 120, 40),
            Some(CellSize {
                width: 16,
                height: 27
            })
        );
    }

    #[test]
    fn test_cell_from_pixels_rejects_unpopulated_pixels() {
        // A terminal that reports a grid but zero pixel dimensions (the
        // ioctl-unsupported case) must yield None so cell_size() falls through
        // to the CSI 14 t query rather than fabricating a tiny cell.
        assert_eq!(cell_from_pixels(0, 0, 120, 40), None);
        assert_eq!(cell_from_pixels(1920, 0, 120, 40), None);
    }

    #[test]
    fn test_cell_from_pixels_rejects_zero_grid() {
        // Guards the division against a zero grid dimension.
        assert_eq!(cell_from_pixels(1920, 1080, 0, 40), None);
        assert_eq!(cell_from_pixels(1920, 1080, 120, 0), None);
    }

    #[test]
    #[ignore = "opens /dev/tty and sends escape sequences - run manually in real terminal"]
    fn test_window_size_pixels_does_not_panic() {
        let _ = window_size_pixels();
    }
}
