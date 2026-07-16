//! Hybrid OSC color query (actual + heuristic) plus terminal default fallbacks.

use std::time::Duration;

#[cfg(unix)]
use crate::discovery::raw_mode::{RawModeGuard, TERMINAL_QUERY_MUTEX};

use crate::discovery::detection::{TerminalApp, get_terminal_app, is_tty};
use crate::discovery::os_detection::is_ci;

#[cfg(unix)]
use super::parse::parse_osc_color_response;
use super::parse::{ansi_index_to_rgb, parse_colorfgbg};
use super::types::{DEFAULT_TIMEOUT, OscQueryError, RgbValue};

// Counts *actual* tty round-trips attempted per OSC code (10/11/12), i.e. calls
// that reach `query_osc_actual`. This is the quantity the per-process colour
// caches exist to suppress, and it cannot be observed from outside the process
// when the terminal is a real emulator (WezTerm/Kitty answer on the wire but
// consume the query, so no PTY master is available to count it).
//
// Active under `cfg(test)` and under the `osc-query-counter` feature, which the
// `test-l2` recipe enables so the real-terminal cache proof
// (`tests/level2_terminal_osc_wezterm.rs`) can read the count out of the
// `discovery_probe` example. Not part of the released API.
#[cfg(any(test, feature = "osc-query-counter"))]
static ACTUAL_QUERY_COUNTS: [std::sync::atomic::AtomicUsize; 3] = [
    std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0),
];

/// Slot index for OSC codes 10/11/12.
// `dead_code`: the only recording call site is Unix-gated (the actual query path
// does not exist on Windows), so these are unreferenced on non-Unix targets.
#[allow(dead_code)]
#[cfg(any(test, feature = "osc-query-counter"))]
fn counter_slot(code: u8) -> Option<usize> {
    match code {
        10 => Some(0),
        11 => Some(1),
        12 => Some(2),
        _ => None,
    }
}

#[allow(dead_code)]
#[cfg(any(test, feature = "osc-query-counter"))]
fn record_actual_query(code: u8) {
    if let Some(slot) = counter_slot(code) {
        ACTUAL_QUERY_COUNTS[slot].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Returns how many actual tty round-trips have been attempted for `code`.
///
/// Test-only instrumentation gated behind the `osc-query-counter` feature (and
/// `cfg(test)`); not part of the released API. Returns 0 for codes other than
/// 10, 11, and 12.
#[cfg(any(test, feature = "osc-query-counter"))]
pub fn actual_query_count(code: u8) -> usize {
    match counter_slot(code) {
        Some(slot) => ACTUAL_QUERY_COUNTS[slot].load(std::sync::atomic::Ordering::Relaxed),
        None => 0,
    }
}

/// Human-readable name for an OSC color query code.
fn osc_color_name(code: u8) -> &'static str {
    match code {
        10 => "foreground color",
        11 => "background color",
        12 => "cursor color",
        _ => "unknown color",
    }
}

/// Query terminal color using a hybrid approach.
///
/// This function tries multiple detection methods in order:
/// 1. Actual OSC query (if supported and not in CI/multiplexer)
/// 2. `COLORFGBG` environment variable
/// 3. Terminal application defaults
///
/// The fallback chain ensures we always return a reasonable result
/// when possible, while preferring actual terminal queries for accuracy.
pub(super) fn query_osc_color(code: u8) -> Option<RgbValue> {
    query_osc_color_with_timeout(code, DEFAULT_TIMEOUT)
}

/// Query terminal color with a custom timeout.
pub(super) fn query_osc_color_with_timeout(code: u8, timeout: Duration) -> Option<RgbValue> {
    // Silence unused warning on non-Unix platforms; used inside #[cfg(unix)] below.
    let _ = timeout;

    // Skip if not a TTY or in CI
    if !is_tty() {
        tracing::debug!(code, "OSC{} query skipped: not a TTY", code);
        return None;
    }
    if is_ci() {
        tracing::debug!(code, "OSC{} query skipped: CI environment", code);
        return None;
    }

    // Try actual OSC query first (if terminal supports it)
    #[cfg(unix)]
    {
        let term_app = get_terminal_app();
        let supports_osc = matches!(
            term_app,
            TerminalApp::Kitty
                | TerminalApp::Wezterm
                | TerminalApp::ITerm2
                | TerminalApp::Alacritty
                | TerminalApp::Ghostty
                | TerminalApp::Foot
                | TerminalApp::Contour
        );

        if supports_osc && detect_multiplexer().is_none() {
            #[cfg(any(test, feature = "osc-query-counter"))]
            record_actual_query(code);

            match query_osc_actual(code, timeout) {
                Ok(color) => {
                    tracing::debug!(
                        code,
                        r = color.r,
                        g = color.g,
                        b = color.b,
                        source = "actual_query",
                        "OSC{} color detected via actual query",
                        code
                    );
                    return Some(color);
                }
                Err(e) => {
                    tracing::debug!(
                        code,
                        query = osc_color_name(code),
                        error = %e,
                        "OSC{} ({}) query failed, falling back to heuristics",
                        code,
                        osc_color_name(code)
                    );
                }
            }
        }
    }

    // Fallback 1: Try COLORFGBG environment variable
    if let Ok(colorfgbg) = std::env::var("COLORFGBG")
        && let Some(color) = parse_colorfgbg(&colorfgbg, code)
    {
        tracing::debug!(
            code,
            colorfgbg = %colorfgbg,
            r = color.r,
            g = color.g,
            b = color.b,
            source = "COLORFGBG",
            "OSC{} color detected via COLORFGBG env var",
            code
        );
        return Some(color);
    }

    // Fallback 2: Terminal app defaults
    let term_app = get_terminal_app();
    if let Some(color) = get_terminal_default_color(&term_app, code) {
        tracing::debug!(
            code,
            terminal = ?term_app,
            r = color.r,
            g = color.g,
            b = color.b,
            source = "terminal_defaults",
            "OSC{} color detected via terminal defaults",
            code
        );
        return Some(color);
    }

    // Suppress unused warnings on non-unix
    let _ = ansi_index_to_rgb;

    tracing::debug!(
        code,
        query = osc_color_name(code),
        "OSC{} ({}) detection failed, no source available",
        code,
        osc_color_name(code)
    );
    None
}

/// Get default colors for known terminal applications.
fn get_terminal_default_color(app: &TerminalApp, code: u8) -> Option<RgbValue> {
    match app {
        // Apple Terminal defaults to white background (light mode)
        TerminalApp::AppleTerminal => match code {
            10 | 12 => Some(RgbValue::new(0, 0, 0)),
            11 => Some(RgbValue::new(255, 255, 255)),
            _ => None,
        },

        // Most modern terminals default to dark themes
        TerminalApp::Kitty
        | TerminalApp::Alacritty
        | TerminalApp::Wezterm
        | TerminalApp::ITerm2
        | TerminalApp::Ghostty
        | TerminalApp::Warp
        | TerminalApp::Foot
        | TerminalApp::Contour
        | TerminalApp::GnomeTerminal
        | TerminalApp::Konsole
        | TerminalApp::VsCode
        | TerminalApp::WindowsTerminal
        | TerminalApp::Wast => match code {
            10 | 12 => Some(RgbValue::new(229, 229, 229)),
            11 => Some(RgbValue::new(30, 30, 30)),
            _ => None,
        },

        TerminalApp::Other(_) => match code {
            10 | 12 => Some(RgbValue::new(229, 229, 229)),
            11 => Some(RgbValue::new(30, 30, 30)),
            _ => None,
        },
    }
}

/// Check if running inside a terminal multiplexer.
pub(super) fn detect_multiplexer() -> Option<&'static str> {
    if std::env::var("TMUX").is_ok() {
        Some("tmux")
    } else if std::env::var("ZELLIJ").is_ok() {
        Some("zellij")
    } else if std::env::var("STY").is_ok() {
        Some("screen")
    } else {
        None
    }
}

/// Perform an actual OSC query to the terminal.
///
/// This function sends an OSC query sequence and reads the response.
/// It requires raw mode and has timeout handling.
///
/// ## Arguments
///
/// * `code` - OSC code to query (10=foreground, 11=background, 12=cursor)
/// * `timeout` - Maximum time to wait for response
///
/// ## Returns
///
/// `Ok(RgbValue)` on success, or an `OscQueryError` on failure.
///
/// ## Platform Support
///
/// This function is only available on Unix platforms. On other platforms,
/// it returns `Err(OscQueryError::Unsupported)`.
#[cfg(unix)]
pub fn query_osc_actual(code: u8, timeout: Duration) -> Result<RgbValue, OscQueryError> {
    use std::io::{Read, Write};
    use std::os::unix::io::AsRawFd;

    // Pre-flight checks
    if !is_tty() {
        return Err(OscQueryError::NotTty);
    }
    if is_ci() {
        return Err(OscQueryError::CiEnvironment);
    }
    if let Some(mux) = detect_multiplexer() {
        return Err(OscQueryError::Multiplexer(mux.to_string()));
    }

    // Open /dev/tty for the query I/O so the request bytes reach the
    // controlling terminal even when stdout is redirected to a pipe (as
    // happens when the binary is invoked under `output="$(...)"`).
    // Writing to `std::io::stdout()` there would land the query in the
    // captured stream; the terminal would never receive it; and any
    // wrapper re-emitting `$output` later would trigger a delayed reply
    // that arrives as garbage on the next shell prompt.
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| OscQueryError::IoError(format!("open /dev/tty: {e}")))?;
    let fd = tty.as_raw_fd();

    let _lock = TERMINAL_QUERY_MUTEX
        .lock()
        .map_err(|_| OscQueryError::IoError("terminal query mutex poisoned".into()))?;

    let _guard = RawModeGuard::new(fd).map_err(OscQueryError::IoError)?;

    let query = format!("\x1b]{};?\x07", code);
    tty.write_all(query.as_bytes())
        .map_err(|e| OscQueryError::IoError(e.to_string()))?;
    tty.flush()
        .map_err(|e| OscQueryError::IoError(e.to_string()))?;

    let mut buffer = [0u8; 64];
    let mut response = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match tty.read(&mut buffer) {
            Ok(0) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(n) => {
                response.extend_from_slice(&buffer[..n]);
                if response.contains(&0x07) || response.windows(2).any(|w| w == b"\x1b\\") {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                return Err(OscQueryError::IoError(e.to_string()));
            }
        }
    }

    if response.is_empty() {
        return Err(OscQueryError::Timeout(timeout));
    }

    parse_osc_color_response(&response, code)
        .ok_or_else(|| OscQueryError::ParseError("invalid response format".into()))
}

/// Stub for non-Unix platforms.
#[cfg(not(unix))]
pub fn query_osc_actual(code: u8, _timeout: Duration) -> Result<RgbValue, OscQueryError> {
    Err(OscQueryError::Unsupported(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_terminal_default_color_apple_terminal() {
        let app = TerminalApp::AppleTerminal;

        let bg = get_terminal_default_color(&app, 11);
        assert!(bg.is_some());
        assert!(bg.unwrap().is_light());

        let fg = get_terminal_default_color(&app, 10);
        assert!(fg.is_some());
        assert!(fg.unwrap().is_dark());
    }

    #[test]
    fn test_get_terminal_default_color_modern_terminals() {
        let terminals = [
            TerminalApp::Kitty,
            TerminalApp::Alacritty,
            TerminalApp::Wezterm,
            TerminalApp::ITerm2,
            TerminalApp::Ghostty,
        ];

        for app in terminals {
            let bg = get_terminal_default_color(&app, 11);
            assert!(bg.is_some(), "{:?} should have default bg", app);
            assert!(bg.unwrap().is_dark(), "{:?} should default to dark bg", app);

            let fg = get_terminal_default_color(&app, 10);
            assert!(fg.is_some(), "{:?} should have default fg", app);
            assert!(
                fg.unwrap().is_light(),
                "{:?} should default to light fg",
                app
            );
        }
    }

    #[test]
    fn test_get_terminal_default_color_unknown() {
        let app = TerminalApp::Other("unknown".to_string());

        let bg = get_terminal_default_color(&app, 11);
        assert!(bg.is_some());
        assert!(bg.unwrap().is_dark());
    }

    #[test]
    fn test_get_terminal_default_color_invalid_code() {
        let app = TerminalApp::Kitty;
        assert!(get_terminal_default_color(&app, 99).is_none());
    }

    #[test]
    fn test_detect_multiplexer_none() {
        if std::env::var("TMUX").is_err()
            && std::env::var("ZELLIJ").is_err()
            && std::env::var("STY").is_err()
        {
            assert!(detect_multiplexer().is_none());
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_query_osc_actual_not_tty_in_tests() {
        let result = query_osc_actual(11, Duration::from_millis(50));

        match result {
            Err(OscQueryError::NotTty) | Err(OscQueryError::CiEnvironment) => {}
            Err(OscQueryError::Multiplexer(_)) => {}
            Ok(_) => {}
            Err(OscQueryError::Timeout(_)) => {}
            Err(OscQueryError::ParseError(_)) => {}
            Err(e) => {
                panic!("Unexpected error variant: {:?}", e);
            }
        }
    }

    #[test]
    #[cfg(not(unix))]
    fn test_query_osc_actual_unsupported_non_unix() {
        let result = query_osc_actual(11, Duration::from_millis(50));
        assert!(matches!(result, Err(OscQueryError::Unsupported(11))));
    }
}
