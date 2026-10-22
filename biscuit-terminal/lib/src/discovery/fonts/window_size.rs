//! Window pixel size and cell size queries: the `TIOCGWINSZ` ioctl and
//! CSI 14 t on Unix, the CSI 16 t answer from the startup exchange on native
//! Windows.

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
    use std::time::Duration;

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

    let response =
        match crate::discovery::tty_query::round_trip(b"\x1b[14t", Duration::from_millis(100)) {
            Ok(response) => response,
            Err(e) => {
                tracing::debug!("window_size_pixels(): {e}");
                return None;
            }
        };

    // Parse response: \x1b[4;height;widtht
    let result = parse_csi_14t_response(&response);
    if result.is_some() {
        tracing::debug!("window_size_pixels(): detected {:?}", result);
    } else {
        tracing::debug!(
            "window_size_pixels(): failed to parse response: {:?}",
            String::from_utf8_lossy(&response)
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

/// The XTWINOPS cell-size request (`CSI 16 t`). The terminal answers
/// `CSI 6 ; <height> ; <width> t`, in pixels.
pub(crate) const CELL_SIZE_REQUEST: &[u8] = b"\x1b[16t";

/// The largest cell dimension accepted from a `CSI 16 t` reply. A real cell
/// is tens of pixels; anything beyond this is a corrupt or hostile reply.
const MAX_CELL_PIXELS: u32 = 1024;

/// The cell size in the first complete `CSI 6 ; <height> ; <width> t` reply
/// anywhere in `bytes`, which may hold other replies (OSC colors, DA1, Kitty)
/// around it.
///
/// A reply with a zero dimension (a terminal that does not know its pixels)
/// or one above [`MAX_CELL_PIXELS`] is `None`, as is an incomplete one.
pub(crate) fn find_cell_size_reply(bytes: &[u8]) -> Option<CellSize> {
    let mut from = 0;
    while let Some(rel) = bytes[from..].windows(4).position(|w| w == b"\x1b[6;") {
        let start = from + rel;
        from = start + 1;
        let params = &bytes[start + 4..];
        let len = params
            .iter()
            .take_while(|b| b.is_ascii_digit() || **b == b';')
            .count();
        if params.get(len) != Some(&b't') {
            continue;
        }
        let text = std::str::from_utf8(&params[..len]).ok()?;
        let mut fields = text.split(';');
        let (Some(height), Some(width), None) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let (Ok(height), Ok(width)) = (height.parse::<u32>(), width.parse::<u32>()) else {
            continue;
        };
        let valid = |px: u32| (1..=MAX_CELL_PIXELS).contains(&px);
        return (valid(width) && valid(height)).then_some(CellSize { width, height });
    }
    None
}

/// Divide window pixel dimensions by the character grid to yield cell size.
///
/// Returns `None` if any dimension is zero, which guards against both an
/// unpopulated pixel size (terminal reports `0` for `ws_xpixel`/`ws_ypixel`)
/// and a divide-by-zero on the grid.
#[cfg(any(not(windows), test))]
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

#[cfg(not(any(unix, windows)))]
fn cell_size_ioctl() -> Option<CellSize> {
    None
}

/// Calculate the cell size (font dimensions) in pixels.
///
/// On Unix this is measured on every call, so it follows font changes:
/// the synchronous `TIOCGWINSZ` ioctl first, which is deterministic and free
/// of the read-timeout race inherent to the CSI 14 t escape query, and only
/// when the ioctl leaves the pixel fields unpopulated, the terminal's
/// [`window_size_pixels`] divided by the grid.
///
/// Native Windows has no pixel ioctl. There the size is the terminal's answer
/// to `CSI 16 t`, asked once per process inside the startup exchange (see
/// `docs/discovery/terminal-queries.md`); a font change later in the process
/// is not seen.
///
/// ## Returns
///
/// - `Some(CellSize)` if the terminal reported a usable measurement
/// - `None` otherwise; callers then use [`CellSize::FALLBACK`]
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
    platform_cell_size()
}

#[cfg(windows)]
fn platform_cell_size() -> Option<CellSize> {
    crate::discovery::startup_exchange::live_cell_size()
}

#[cfg(not(windows))]
fn platform_cell_size() -> Option<CellSize> {
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
    fn cell_size_reply_reads_height_then_width() {
        // Windows Terminal's live answer (2026-10-21): 20 px tall, 10 px wide.
        assert_eq!(
            find_cell_size_reply(b"\x1b[6;20;10t"),
            Some(CellSize {
                width: 10,
                height: 20
            })
        );
    }

    #[test]
    fn cell_size_reply_is_found_among_the_startup_exchange_replies() {
        let bytes = b"\x1b]10;rgb:cccc/dddd/eeee\x1b\\\x1b]11;rgb:1111/2222/3333\x07\
            \x1b[6;20;10t\x1b_Gi=7;OK\x1b\\\x1b[?61;4;6c";
        assert_eq!(
            find_cell_size_reply(bytes),
            Some(CellSize {
                width: 10,
                height: 20
            })
        );
    }

    #[test]
    fn cell_size_reply_split_across_reads_is_read_once_complete() {
        // The console delivers a reply in pieces; the exchange accumulates
        // them and parses the whole buffer.
        let wire = b"\x1b]11;rgb:1111/2222/3333\x07\x1b[6;20;10t\x1b[?61c";
        let complete_at = wire.iter().position(|&b| b == b't').expect("final byte") + 1;
        let mut collected = Vec::new();
        for chunk in wire.chunks(3) {
            collected.extend_from_slice(chunk);
            let expected = (collected.len() >= complete_at).then_some(CellSize {
                width: 10,
                height: 20,
            });
            assert_eq!(find_cell_size_reply(&collected), expected, "after {} bytes", collected.len());
        }
    }

    #[test]
    fn cell_size_reply_rejects_zero_and_absurd_dimensions() {
        assert_eq!(find_cell_size_reply(b"\x1b[6;0;0t"), None);
        assert_eq!(find_cell_size_reply(b"\x1b[6;20;0t"), None);
        assert_eq!(find_cell_size_reply(b"\x1b[6;0;10t"), None);
        assert_eq!(find_cell_size_reply(b"\x1b[6;20;1025t"), None);
        assert_eq!(find_cell_size_reply(b"\x1b[6;99999999999;10t"), None);
        assert_eq!(
            find_cell_size_reply(b"\x1b[6;1024;1t"),
            Some(CellSize {
                width: 1,
                height: 1024
            })
        );
    }

    #[test]
    fn other_csi_replies_are_not_a_cell_size() {
        assert_eq!(find_cell_size_reply(b"\x1b[6;20t"), None);
        assert_eq!(find_cell_size_reply(b"\x1b[6;20;10;5t"), None);
        // CSI 14 t's window-pixel reply and a cursor position report.
        assert_eq!(find_cell_size_reply(b"\x1b[4;1080;1920t"), None);
        assert_eq!(find_cell_size_reply(b"\x1b[6;20R"), None);
        assert_eq!(find_cell_size_reply(b"\x1b[?61;4c"), None);
    }

    #[test]
    fn a_malformed_cell_size_reply_does_not_hide_a_later_one() {
        assert_eq!(
            find_cell_size_reply(b"\x1b[6;20t\x1b[6;18;9t"),
            Some(CellSize {
                width: 9,
                height: 18
            })
        );
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
