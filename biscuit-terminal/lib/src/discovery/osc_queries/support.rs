//! OSC support detection (heuristics + per-session cache).

use std::sync::OnceLock;

use crate::discovery::detection::{TerminalApp, get_terminal_app, is_tty};
use crate::discovery::os_detection::is_ci;

use super::query::detect_multiplexer;

// Session cache for OSC support detection
static OSC10_SUPPORT: OnceLock<bool> = OnceLock::new();
static OSC11_SUPPORT: OnceLock<bool> = OnceLock::new();
static OSC12_SUPPORT: OnceLock<bool> = OnceLock::new();

/// Internal function to check if OSC queries are supported based on heuristics.
///
/// This function NEVER attempts actual OSC queries. It uses:
/// - TTY detection
/// - CI environment detection
/// - Multiplexer detection
/// - Terminal app detection
fn is_osc_query_supported_heuristic(code: u8) -> bool {
    if !is_tty() {
        tracing::debug!(code, "OSC{} not supported: not a TTY", code);
        return false;
    }

    if is_ci() {
        tracing::debug!(code, "OSC{} not supported: CI environment", code);
        return false;
    }

    if detect_multiplexer().is_some() {
        tracing::debug!(code, "OSC{} not supported: inside multiplexer", code);
        return false;
    }

    let app = get_terminal_app();
    let supported = matches!(
        app,
        TerminalApp::Kitty
            | TerminalApp::Wezterm
            | TerminalApp::ITerm2
            | TerminalApp::Alacritty
            | TerminalApp::Ghostty
            | TerminalApp::Foot
            | TerminalApp::Contour
    );

    tracing::debug!(
        code,
        terminal = ?app,
        supported,
        "OSC{} support detection via terminal app",
        code
    );

    supported
}

/// Check if the terminal supports OSC 10 (foreground color) queries.
///
/// This function uses heuristics to determine support without
/// actually querying the terminal. Results are cached for the session.
///
/// ## Returns
///
/// `true` if the terminal likely supports OSC 10 queries.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::osc_queries::osc10_support;
///
/// if osc10_support() {
///     println!("Terminal supports foreground color queries");
/// }
/// ```
pub fn osc10_support() -> bool {
    *OSC10_SUPPORT.get_or_init(|| is_osc_query_supported_heuristic(10))
}

/// Check if the terminal supports OSC 11 (background color) queries.
pub fn osc11_support() -> bool {
    *OSC11_SUPPORT.get_or_init(|| is_osc_query_supported_heuristic(11))
}

/// Check if the terminal supports OSC 12 (cursor color) queries.
pub fn osc12_support() -> bool {
    *OSC12_SUPPORT.get_or_init(|| is_osc_query_supported_heuristic(12))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_support_functions_dont_panic() {
        let _ = osc10_support();
        let _ = osc11_support();
        let _ = osc12_support();
    }

    #[test]
    fn test_osc_support_functions_consistent() {
        let first = osc11_support();
        let second = osc11_support();
        assert_eq!(first, second, "Cached OSC support should be consistent");
    }
}
