//! OSC (Operating System Command) queries for terminal colors.
//!
//! This module provides terminal color detection using a **hybrid approach**:
//! actual OSC queries when supported, with heuristic fallbacks for reliability.
//!
//! ## Strategy
//!
//! Color detection uses a cascading fallback chain:
//!
//! 1. **Actual OSC query** (Unix only, supported terminals) - Most accurate,
//!    sends escape sequence and parses response with timeout handling
//! 2. **`COLORFGBG` environment variable** - Set by some terminals with
//!    foreground/background color indices
//! 3. **Terminal application defaults** - Known default colors for detected
//!    terminal emulators (Kitty, iTerm2, etc.)
//!
//! ## Supported Terminals (Actual Queries)
//!
//! The following terminals support actual OSC 10/11/12 queries:
//! - Kitty
//! - WezTerm
//! - iTerm2
//! - Alacritty
//! - Ghostty
//! - Foot
//! - Contour
//!
//! ## Limitations
//!
//! - **Multiplexers**: tmux, Zellij, and GNU Screen may not pass through
//!   OSC queries correctly. When detected, we fall back to heuristics.
//! - **CI environments**: Actual queries are skipped in CI to avoid hangs.
//! - **Non-TTY**: When stdout is not a terminal, returns `None`.
//!
//! ## Support Detection
//!
//! Use [`osc10_support`], [`osc11_support`], and [`osc12_support`] to check
//! if the current terminal likely supports color queries. These functions
//! use heuristics only (no actual I/O) and cache results for the session.
//!
//! ## Examples
//!
//! ### Basic Color Detection
//!
//! ```
//! use biscuit_terminal::discovery::osc_queries::{bg_color, text_color, RgbValue};
//!
//! if let Some(bg) = bg_color() {
//!     let luminance = bg.luminance();
//!     if luminance > 0.5 {
//!         println!("Light background detected");
//!     } else {
//!         println!("Dark background detected");
//!     }
//! }
//! ```
//!
//! ### Check Support Before Querying
//!
//! ```no_run
//! use biscuit_terminal::discovery::osc_queries::{osc11_support, bg_color};
//!
//! if osc11_support() {
//!     // Terminal likely supports background color queries
//!     if let Some(bg) = bg_color() {
//!         println!("Background: {}", bg);
//!     }
//! } else {
//!     println!("Terminal doesn't support OSC color queries");
//! }
//! ```
//!
//! ### Custom Timeout
//!
//! ```no_run
//! use std::time::Duration;
//! use biscuit_terminal::discovery::osc_queries::bg_color_with_timeout;
//!
//! // Use shorter timeout for faster fallback
//! let bg = bg_color_with_timeout(Duration::from_millis(50));
//! ```

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use thiserror::Error;

use crate::discovery::detection::{get_terminal_app, is_tty, TerminalApp};
use crate::discovery::os_detection::is_ci;

/// Errors that can occur when querying terminal colors via OSC sequences.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OscQueryError {
    /// Standard output is not connected to a TTY.
    #[error("not connected to a TTY")]
    NotTty,

    /// Running in a CI environment where terminal queries are not supported.
    #[error("running in CI environment")]
    CiEnvironment,

    /// The terminal does not support this OSC query.
    #[error("terminal does not support OSC {0} queries")]
    Unsupported(u8),

    /// The query timed out waiting for a response.
    #[error("OSC query timed out after {0:?}")]
    Timeout(Duration),

    /// Failed to parse the terminal's response.
    #[error("failed to parse OSC response: {0}")]
    ParseError(String),

    /// An I/O error occurred during the query.
    #[error("I/O error: {0}")]
    IoError(String),

    /// Running inside a terminal multiplexer that may not pass through OSC queries.
    #[error("running inside multiplexer ({0}), OSC queries may not work")]
    Multiplexer(String),
}

/// Default timeout for OSC queries (not used in current implementation but
/// provided for future use or downstream compatibility).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(100);

/// RGB color with 8-bit components.
///
/// Represents a color in the sRGB color space with values from 0-255
/// for each channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbValue {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

impl RgbValue {
    /// Create a new RGB color.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Calculate relative luminance using the sRGB luminance formula.
    ///
    /// Returns a value between 0.0 (black) and 1.0 (white).
    ///
    /// This uses the ITU-R BT.709 coefficients for sRGB:
    /// - Red: 0.2126
    /// - Green: 0.7152
    /// - Blue: 0.0722
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::discovery::osc_queries::RgbValue;
    ///
    /// let black = RgbValue::new(0, 0, 0);
    /// assert!((black.luminance() - 0.0).abs() < 0.01);
    ///
    /// let white = RgbValue::new(255, 255, 255);
    /// assert!((white.luminance() - 1.0).abs() < 0.01);
    /// ```
    pub fn luminance(&self) -> f64 {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Check if this color is considered "light" (luminance > 0.5).
    pub fn is_light(&self) -> bool {
        self.luminance() > 0.5
    }

    /// Check if this color is considered "dark" (luminance <= 0.5).
    pub fn is_dark(&self) -> bool {
        self.luminance() <= 0.5
    }
}

impl std::fmt::Display for RgbValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

/// Query background color via OSC 11 heuristics.
///
/// Returns `None` if:
/// - Not running in a TTY
/// - Running in a CI environment
/// - No color information is available
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::osc_queries::bg_color;
///
/// if let Some(bg) = bg_color() {
///     println!("Background color: {}", bg);
/// }
/// ```
pub fn bg_color() -> Option<RgbValue> {
    query_osc_color(11)
}

/// Query foreground/text color via OSC 10 heuristics.
///
/// Returns `None` if:
/// - Not running in a TTY
/// - Running in a CI environment
/// - No color information is available
pub fn text_color() -> Option<RgbValue> {
    query_osc_color(10)
}

/// Query cursor color via OSC 12 heuristics.
///
/// Returns `None` if:
/// - Not running in a TTY
/// - Running in a CI environment
/// - No color information is available
///
/// Note: Cursor color often defaults to the same as foreground text color.
pub fn cursor_color() -> Option<RgbValue> {
    query_osc_color(12)
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
fn query_osc_color(code: u8) -> Option<RgbValue> {
    query_osc_color_with_timeout(code, DEFAULT_TIMEOUT)
}

/// Query terminal color with a custom timeout.
///
/// This is the internal implementation with configurable timeout.
/// Use `bg_color()`, `text_color()`, or `cursor_color()` for the public API.
fn query_osc_color_with_timeout(code: u8, timeout: Duration) -> Option<RgbValue> {
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

        // Only attempt actual query if:
        // 1. Terminal is known to support OSC queries
        // 2. Not inside a multiplexer
        if supports_osc && detect_multiplexer().is_none() {
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
                        error = %e,
                        "OSC{} actual query failed, falling back to heuristics",
                        code
                    );
                    // Fall through to heuristics
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

    tracing::debug!(code, "OSC{} color detection failed, no source available", code);
    None
}

/// Query background color with a custom timeout.
///
/// This variant allows specifying a custom timeout for the actual OSC query.
/// Use `bg_color()` for the default timeout.
///
/// ## Arguments
///
/// * `timeout` - Maximum time to wait for terminal response
///
/// ## Returns
///
/// `Some(RgbValue)` if detection succeeded, `None` otherwise.
pub fn bg_color_with_timeout(timeout: Duration) -> Option<RgbValue> {
    query_osc_color_with_timeout(11, timeout)
}

/// Query foreground/text color with a custom timeout.
///
/// This variant allows specifying a custom timeout for the actual OSC query.
/// Use `text_color()` for the default timeout.
pub fn text_color_with_timeout(timeout: Duration) -> Option<RgbValue> {
    query_osc_color_with_timeout(10, timeout)
}

/// Query cursor color with a custom timeout.
///
/// This variant allows specifying a custom timeout for the actual OSC query.
/// Use `cursor_color()` for the default timeout.
pub fn cursor_color_with_timeout(timeout: Duration) -> Option<RgbValue> {
    query_osc_color_with_timeout(12, timeout)
}

/// Parse the `COLORFGBG` environment variable.
///
/// Format: `"fg_index;bg_index"` where indices are ANSI color numbers (0-15).
/// Some terminals use `"fg;bg;brightness"` format with an optional third value.
///
/// - code 10: foreground
/// - code 11: background
/// - code 12: cursor (typically same as foreground)
fn parse_colorfgbg(value: &str, code: u8) -> Option<RgbValue> {
    let parts: Vec<&str> = value.split(';').collect();
    if parts.len() < 2 {
        return None;
    }

    let index = match code {
        10 | 12 => parts[0].parse::<u8>().ok()?,
        11 => parts.get(1).and_then(|s| s.parse::<u8>().ok())?,
        _ => return None,
    };

    ansi_index_to_rgb(index)
}

/// Convert ANSI color index (0-15) to RGB.
///
/// Uses the standard ANSI/VGA color palette:
/// - 0-7: Normal colors (black, red, green, yellow, blue, magenta, cyan, white)
/// - 8-15: Bright variants
fn ansi_index_to_rgb(index: u8) -> Option<RgbValue> {
    match index {
        0 => Some(RgbValue::new(0, 0, 0)),           // Black
        1 => Some(RgbValue::new(205, 49, 49)),       // Red
        2 => Some(RgbValue::new(13, 188, 121)),      // Green
        3 => Some(RgbValue::new(229, 229, 16)),      // Yellow
        4 => Some(RgbValue::new(36, 114, 200)),      // Blue
        5 => Some(RgbValue::new(188, 63, 188)),      // Magenta
        6 => Some(RgbValue::new(17, 168, 205)),      // Cyan
        7 => Some(RgbValue::new(229, 229, 229)),     // White
        8 => Some(RgbValue::new(102, 102, 102)),     // Bright Black (Gray)
        9 => Some(RgbValue::new(241, 76, 76)),       // Bright Red
        10 => Some(RgbValue::new(35, 209, 139)),     // Bright Green
        11 => Some(RgbValue::new(245, 245, 67)),     // Bright Yellow
        12 => Some(RgbValue::new(59, 142, 234)),     // Bright Blue
        13 => Some(RgbValue::new(214, 112, 214)),    // Bright Magenta
        14 => Some(RgbValue::new(41, 184, 219)),     // Bright Cyan
        15 => Some(RgbValue::new(255, 255, 255)),    // Bright White
        _ => None,
    }
}

/// Get default colors for known terminal applications.
///
/// Most modern terminals default to dark mode, but we can be more specific
/// for terminals where we know the default theme.
fn get_terminal_default_color(app: &TerminalApp, code: u8) -> Option<RgbValue> {
    match app {
        // Apple Terminal defaults to white background (light mode)
        TerminalApp::AppleTerminal => match code {
            10 | 12 => Some(RgbValue::new(0, 0, 0)),         // Black text
            11 => Some(RgbValue::new(255, 255, 255)),        // White background
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
        | TerminalApp::Wast => match code {
            10 | 12 => Some(RgbValue::new(229, 229, 229)),   // Light text
            11 => Some(RgbValue::new(30, 30, 30)),           // Dark background
            _ => None,
        },

        // Unknown terminal - default to dark mode (most common for terminal users)
        TerminalApp::Other(_) => match code {
            10 | 12 => Some(RgbValue::new(229, 229, 229)),   // Light text
            11 => Some(RgbValue::new(30, 30, 30)),           // Dark background
            _ => None,
        },
    }
}


/// Check if running inside a terminal multiplexer.
///
/// Returns `Some(name)` if a multiplexer is detected, `None` otherwise.
/// Multiplexers like tmux and Zellij may not pass through OSC queries correctly.
fn detect_multiplexer() -> Option<&'static str> {
    if std::env::var("TMUX").is_ok() {
        Some("tmux")
    } else if std::env::var("ZELLIJ").is_ok() {
        Some("zellij")
    } else if std::env::var("STY").is_ok() {
        // GNU Screen
        Some("screen")
    } else {
        None
    }
}

/// Parse an OSC color response from the terminal.
///
/// Terminals respond to OSC 10/11/12 queries with:
/// - BEL-terminated: `\x1b]<code>;rgb:<rrrr>/<gggg>/<bbbb>\x07`
/// - ST-terminated: `\x1b]<code>;rgb:<rrrr>/<gggg>/<bbbb>\x1b\\`
///
/// The RGB values are 16-bit hex values (0000-ffff) that need to be
/// converted to 8-bit (0-255).
///
/// ## Arguments
///
/// * `response` - Raw bytes from terminal response
/// * `expected_code` - The OSC code we expect (10, 11, or 12)
///
/// ## Returns
///
/// `Some(RgbValue)` if parsing succeeded, `None` otherwise.
pub fn parse_osc_color_response(response: &[u8], expected_code: u8) -> Option<RgbValue> {
    // Convert to string, stopping at BEL (\x07) or ST (\x1b\)
    let response_str = std::str::from_utf8(response).ok()?;

    // Find the end of the response (BEL or ST)
    let end_pos = response_str
        .find('\x07')
        .or_else(|| response_str.find("\x1b\\"))
        .unwrap_or(response_str.len());

    let content = &response_str[..end_pos];

    // Expected format: \x1b]<code>;rgb:<rrrr>/<gggg>/<bbbb>
    // Or without escape: <code>;rgb:<rrrr>/<gggg>/<bbbb>

    // Skip leading escape sequence if present
    let content = content.strip_prefix("\x1b]").unwrap_or(content);

    // Parse code and rgb values
    let parts: Vec<&str> = content.splitn(2, ';').collect();
    if parts.len() != 2 {
        return None;
    }

    // Verify the code matches
    let code: u8 = parts[0].parse().ok()?;
    if code != expected_code {
        return None;
    }

    // Parse rgb:<rrrr>/<gggg>/<bbbb>
    let rgb_part = parts[1].strip_prefix("rgb:")?;
    let rgb_parts: Vec<&str> = rgb_part.split('/').collect();
    if rgb_parts.len() != 3 {
        return None;
    }

    // Parse 16-bit hex values and convert to 8-bit
    let r16 = u16::from_str_radix(rgb_parts[0], 16).ok()?;
    let g16 = u16::from_str_radix(rgb_parts[1], 16).ok()?;
    let b16 = u16::from_str_radix(rgb_parts[2], 16).ok()?;

    Some(RgbValue::new(
        convert_16bit_to_8bit(r16),
        convert_16bit_to_8bit(g16),
        convert_16bit_to_8bit(b16),
    ))
}

/// Convert a 16-bit color component to 8-bit with proper rounding.
///
/// Uses the formula: `(val * 255 + 32767) / 65535`
/// This ensures 0xffff maps to 255, not 254.
#[inline]
fn convert_16bit_to_8bit(val: u16) -> u8 {
    ((val as u32 * 255 + 32767) / 65535) as u8
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

    // RAII guard for terminal state
    struct RawModeGuard {
        original: libc::termios,
        fd: libc::c_int,
    }

    impl RawModeGuard {
        fn new() -> Result<Self, OscQueryError> {
            let fd = libc::STDIN_FILENO;
            let mut original: libc::termios = unsafe { std::mem::zeroed() };

            if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
                return Err(OscQueryError::IoError("failed to get terminal attributes".into()));
            }

            let mut raw = original;
            // Disable canonical mode and echo
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            // Set minimum characters and timeout for read
            raw.c_cc[libc::VMIN] = 0;
            raw.c_cc[libc::VTIME] = 1; // 100ms timeout per read

            if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
                return Err(OscQueryError::IoError("failed to set raw mode".into()));
            }

            Ok(Self { original, fd })
        }
    }

    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            // Always restore original terminal state
            unsafe {
                libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
            }
        }
    }

    // Enter raw mode
    let _guard = RawModeGuard::new()?;

    // Send the OSC query: \x1b]<code>;?\x07
    let query = format!("\x1b]{};?\x07", code);
    let mut stdout = std::io::stdout();
    stdout
        .write_all(query.as_bytes())
        .map_err(|e| OscQueryError::IoError(e.to_string()))?;
    stdout
        .flush()
        .map_err(|e| OscQueryError::IoError(e.to_string()))?;

    // Read response with timeout
    let mut buffer = [0u8; 64];
    let mut response = Vec::new();
    let start = std::time::Instant::now();
    let mut stdin = std::io::stdin();

    while start.elapsed() < timeout {
        match stdin.read(&mut buffer) {
            Ok(0) => {
                // No data, continue waiting
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(n) => {
                response.extend_from_slice(&buffer[..n]);
                // Check if we have a complete response (ends with BEL or ST)
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
    // Not a TTY - no support
    if !is_tty() {
        tracing::debug!(code, "OSC{} not supported: not a TTY", code);
        return false;
    }

    // CI environment - no support
    if is_ci() {
        tracing::debug!(code, "OSC{} not supported: CI environment", code);
        return false;
    }

    // Multiplexer detected - no support (may not pass through)
    if detect_multiplexer().is_some() {
        tracing::debug!(code, "OSC{} not supported: inside multiplexer", code);
        return false;
    }

    // Check terminal app support
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
///
/// This function uses heuristics to determine support without
/// actually querying the terminal. Results are cached for the session.
///
/// ## Returns
///
/// `true` if the terminal likely supports OSC 11 queries.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::osc_queries::osc11_support;
///
/// if osc11_support() {
///     println!("Terminal supports background color queries");
/// }
/// ```
pub fn osc11_support() -> bool {
    *OSC11_SUPPORT.get_or_init(|| is_osc_query_supported_heuristic(11))
}

/// Check if the terminal supports OSC 12 (cursor color) queries.
///
/// This function uses heuristics to determine support without
/// actually querying the terminal. Results are cached for the session.
///
/// ## Returns
///
/// `true` if the terminal likely supports OSC 12 queries.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::osc_queries::osc12_support;
///
/// if osc12_support() {
///     println!("Terminal supports cursor color queries");
/// }
/// ```
pub fn osc12_support() -> bool {
    *OSC12_SUPPORT.get_or_init(|| is_osc_query_supported_heuristic(12))
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_luminance_black() {
        let black = RgbValue::new(0, 0, 0);
        assert!((black.luminance() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_white() {
        let white = RgbValue::new(255, 255, 255);
        assert!((white.luminance() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_gray() {
        let gray = RgbValue::new(128, 128, 128);
        // Gray should be around 0.5 luminance
        assert!(gray.luminance() > 0.2 && gray.luminance() < 0.8);
    }

    #[test]
    fn test_rgb_luminance_red() {
        // Red contributes 0.2126, so pure red should be around 0.21
        let red = RgbValue::new(255, 0, 0);
        assert!((red.luminance() - 0.2126).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_green() {
        // Green contributes 0.7152, so pure green should be around 0.72
        let green = RgbValue::new(0, 255, 0);
        assert!((green.luminance() - 0.7152).abs() < 0.01);
    }

    #[test]
    fn test_rgb_luminance_blue() {
        // Blue contributes 0.0722, so pure blue should be around 0.07
        let blue = RgbValue::new(0, 0, 255);
        assert!((blue.luminance() - 0.0722).abs() < 0.01);
    }

    #[test]
    fn test_rgb_is_light_dark() {
        let black = RgbValue::new(0, 0, 0);
        assert!(black.is_dark());
        assert!(!black.is_light());

        let white = RgbValue::new(255, 255, 255);
        assert!(white.is_light());
        assert!(!white.is_dark());
    }

    #[test]
    fn test_rgb_display() {
        let color = RgbValue::new(100, 150, 200);
        assert_eq!(color.to_string(), "rgb(100, 150, 200)");
    }

    #[test]
    fn test_ansi_index_to_rgb_valid() {
        // Black
        let black = ansi_index_to_rgb(0);
        assert!(black.is_some());
        assert_eq!(black.unwrap(), RgbValue::new(0, 0, 0));

        // White
        let white = ansi_index_to_rgb(15);
        assert!(white.is_some());
        assert_eq!(white.unwrap(), RgbValue::new(255, 255, 255));

        // All 16 colors should be valid
        for i in 0..16 {
            assert!(ansi_index_to_rgb(i).is_some(), "Index {} should be valid", i);
        }
    }

    #[test]
    fn test_ansi_index_to_rgb_invalid() {
        // Out of range
        assert!(ansi_index_to_rgb(16).is_none());
        assert!(ansi_index_to_rgb(255).is_none());
    }

    #[test]
    fn test_parse_colorfgbg_valid_dark() {
        // "15;0" = white foreground (15) on black background (0)
        let bg = parse_colorfgbg("15;0", 11);
        assert!(bg.is_some());
        assert_eq!(bg.unwrap(), RgbValue::new(0, 0, 0)); // Black

        let fg = parse_colorfgbg("15;0", 10);
        assert!(fg.is_some());
        assert_eq!(fg.unwrap(), RgbValue::new(255, 255, 255)); // White
    }

    #[test]
    fn test_parse_colorfgbg_valid_light() {
        // "0;15" = black foreground on white background
        let bg = parse_colorfgbg("0;15", 11);
        assert!(bg.is_some());
        assert_eq!(bg.unwrap(), RgbValue::new(255, 255, 255)); // White

        let fg = parse_colorfgbg("0;15", 10);
        assert!(fg.is_some());
        assert_eq!(fg.unwrap(), RgbValue::new(0, 0, 0)); // Black
    }

    #[test]
    fn test_parse_colorfgbg_cursor_uses_fg() {
        // Cursor (code 12) should use the foreground index
        let cursor = parse_colorfgbg("15;0", 12);
        assert!(cursor.is_some());
        assert_eq!(cursor.unwrap(), RgbValue::new(255, 255, 255)); // Same as fg
    }

    #[test]
    fn test_parse_colorfgbg_with_brightness() {
        // Some terminals use "fg;bg;brightness" format
        let bg = parse_colorfgbg("7;0;1", 11);
        assert!(bg.is_some());
        assert_eq!(bg.unwrap(), RgbValue::new(0, 0, 0)); // Black
    }

    #[test]
    fn test_parse_colorfgbg_invalid() {
        // Empty string
        assert!(parse_colorfgbg("", 11).is_none());

        // Not a number
        assert!(parse_colorfgbg("abc", 11).is_none());

        // Missing background
        assert!(parse_colorfgbg("15", 11).is_none());

        // Invalid code
        assert!(parse_colorfgbg("15;0", 99).is_none());
    }

    #[test]
    fn test_parse_colorfgbg_out_of_range_index() {
        // Index 20 is out of ANSI 16-color range
        assert!(parse_colorfgbg("20;0", 10).is_none());
        assert!(parse_colorfgbg("15;20", 11).is_none());
    }

    #[test]
    fn test_get_terminal_default_color_apple_terminal() {
        // Apple Terminal defaults to light mode
        let app = TerminalApp::AppleTerminal;

        let bg = get_terminal_default_color(&app, 11);
        assert!(bg.is_some());
        assert!(bg.unwrap().is_light()); // White background

        let fg = get_terminal_default_color(&app, 10);
        assert!(fg.is_some());
        assert!(fg.unwrap().is_dark()); // Black text
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
            assert!(fg.unwrap().is_light(), "{:?} should default to light fg", app);
        }
    }

    #[test]
    fn test_get_terminal_default_color_unknown() {
        let app = TerminalApp::Other("unknown".to_string());

        // Unknown terminals default to dark mode
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
    fn test_default_timeout_value() {
        // Verify the constant is set correctly
        assert_eq!(DEFAULT_TIMEOUT, Duration::from_millis(100));
    }

    // Note: We don't test bg_color(), text_color(), cursor_color() directly
    // because they depend on TTY state which varies between test environments.
    // The internal functions are thoroughly tested above.

    // --- Phase 1 Tests: Error types and parsing ---

    #[test]
    fn test_osc_query_error_display() {
        // Test that all error variants have proper Display implementations
        let errors = [
            OscQueryError::NotTty,
            OscQueryError::CiEnvironment,
            OscQueryError::Unsupported(11),
            OscQueryError::Timeout(Duration::from_millis(100)),
            OscQueryError::ParseError("test".into()),
            OscQueryError::IoError("test".into()),
            OscQueryError::Multiplexer("tmux".into()),
        ];

        for err in &errors {
            let msg = err.to_string();
            assert!(!msg.is_empty(), "Error {:?} should have a message", err);
        }
    }

    #[test]
    fn test_osc_query_error_variants() {
        // Ensure Debug derive works
        let err = OscQueryError::NotTty;
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotTty"));
    }

    #[test]
    fn test_parse_osc_color_response_bel_terminated() {
        // Standard BEL-terminated response: \x1b]11;rgb:ffff/ffff/ffff\x07
        let response = b"\x1b]11;rgb:ffff/ffff/ffff\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some(), "Should parse BEL-terminated response");
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 255, "Red should be 255 for 0xffff");
        assert_eq!(rgb.g, 255, "Green should be 255 for 0xffff");
        assert_eq!(rgb.b, 255, "Blue should be 255 for 0xffff");
    }

    #[test]
    fn test_parse_osc_color_response_st_terminated() {
        // ST-terminated response: \x1b]11;rgb:ffff/ffff/ffff\x1b\
        let response = b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some(), "Should parse ST-terminated response");
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 255);
        assert_eq!(rgb.b, 255);
    }

    #[test]
    fn test_parse_osc_color_response_black() {
        // Black: rgb:0000/0000/0000
        let response = b"\x1b]11;rgb:0000/0000/0000\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 0);
        assert_eq!(rgb.g, 0);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn test_parse_osc_color_response_various_codes() {
        // Test OSC 10 (foreground)
        let response = b"\x1b]10;rgb:e5e5/e5e5/e5e5\x07";
        let result = parse_osc_color_response(response, 10);
        assert!(result.is_some());

        // Test OSC 12 (cursor)
        let response = b"\x1b]12;rgb:00ff/ff00/0000\x07";
        let result = parse_osc_color_response(response, 12);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_wrong_code() {
        // Response has code 11 but we expect 10
        let response = b"\x1b]11;rgb:ffff/ffff/ffff\x07";
        let result = parse_osc_color_response(response, 10);
        assert!(result.is_none(), "Should reject mismatched code");
    }

    #[test]
    fn test_parse_osc_color_response_invalid_format() {
        // Missing rgb: prefix
        let response = b"\x1b]11;ffff/ffff/ffff\x07";
        assert!(parse_osc_color_response(response, 11).is_none());

        // Wrong number of components
        let response = b"\x1b]11;rgb:ffff/ffff\x07";
        assert!(parse_osc_color_response(response, 11).is_none());

        // Invalid hex
        let response = b"\x1b]11;rgb:gggg/ffff/ffff\x07";
        assert!(parse_osc_color_response(response, 11).is_none());

        // Empty response
        assert!(parse_osc_color_response(b"", 11).is_none());
    }

    #[test]
    fn test_convert_16bit_to_8bit() {
        // Test boundary conditions
        assert_eq!(convert_16bit_to_8bit(0), 0);
        assert_eq!(convert_16bit_to_8bit(0xffff), 255, "0xffff should map to 255, not 254");
        assert_eq!(convert_16bit_to_8bit(0x8000), 128, "0x8000 should map to ~128");

        // Test some intermediate values
        assert_eq!(convert_16bit_to_8bit(0x0101), 1); // Roughly 1/255
        assert_eq!(convert_16bit_to_8bit(0xfefe), 254); // Roughly 254/255
    }

    #[test]
    fn test_detect_multiplexer_none() {
        // In test environment (no TMUX/ZELLIJ/STY), should return None
        // Note: This test assumes tests don't run in a multiplexer
        // In CI, these env vars are typically not set
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
        // Query behavior depends on the test environment:
        // - CI/piped: NotTty or CiEnvironment error
        // - Real terminal: May succeed or timeout
        // - Multiplexer: Multiplexer error
        let result = query_osc_actual(11, Duration::from_millis(50));

        match result {
            Err(OscQueryError::NotTty) | Err(OscQueryError::CiEnvironment) => {
                // Expected in non-TTY test environments
            }
            Err(OscQueryError::Multiplexer(_)) => {
                // Acceptable if running in tmux/zellij
            }
            Ok(_) => {
                // Running in a real TTY that supports OSC queries - acceptable
            }
            Err(OscQueryError::Timeout(_)) => {
                // Terminal didn't respond in time - acceptable
            }
            Err(OscQueryError::ParseError(_)) => {
                // Terminal responded but couldn't parse - acceptable
            }
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

    // --- Phase 5: Additional comprehensive tests ---

    #[test]
    fn test_parse_osc_color_response_mid_gray() {
        // Test mid-gray (0x8080) -> should convert to ~128
        let response = b"\x1b]11;rgb:8080/8080/8080\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 128);
        assert_eq!(rgb.g, 128);
        assert_eq!(rgb.b, 128);
    }

    #[test]
    fn test_parse_osc_color_response_with_trailing_data() {
        // Response with extra data after terminator
        let response = b"\x1b]11;rgb:ffff/0000/0000\x07extra data";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
        let rgb = result.unwrap();
        assert_eq!(rgb.r, 255);
        assert_eq!(rgb.g, 0);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn test_parse_osc_color_response_lowercase_hex() {
        // Lowercase hex should work
        let response = b"\x1b]11;rgb:abcd/ef01/2345\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_uppercase_hex() {
        // Uppercase hex should work
        let response = b"\x1b]11;rgb:ABCD/EF01/2345\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_mixed_case_hex() {
        // Mixed case hex should work
        let response = b"\x1b]11;rgb:AbCd/eF01/23aB\x07";
        let result = parse_osc_color_response(response, 11);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_osc_color_response_short_hex() {
        // Short hex (less than 4 digits) should still parse
        // Some terminals might return shorter values
        let response = b"\x1b]11;rgb:ff/ff/ff\x07";
        let result = parse_osc_color_response(response, 11);
        // This should parse the short hex as-is
        assert!(result.is_some());
        let rgb = result.unwrap();
        // 0xff as 16-bit is 255, which converts to ~1 in 8-bit
        assert!(rgb.r <= 1);
    }

    #[test]
    fn test_convert_16bit_to_8bit_full_range() {
        // Test conversion accuracy across the full range
        // Every 257 steps in 16-bit should map to 1 step in 8-bit (approximately)
        for i in 0u8..=255 {
            let val16 = (i as u32 * 65535 / 255) as u16;
            let result = convert_16bit_to_8bit(val16);
            // Allow for rounding differences
            assert!(
                result.abs_diff(i) <= 1,
                "16-bit {} should map to ~{}, got {}",
                val16,
                i,
                result
            );
        }
    }

    #[test]
    fn test_osc_query_error_clone() {
        // Test that OscQueryError is Clone
        let err = OscQueryError::NotTty;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_osc_query_error_eq() {
        // Test PartialEq for OscQueryError
        assert_eq!(OscQueryError::NotTty, OscQueryError::NotTty);
        assert_ne!(OscQueryError::NotTty, OscQueryError::CiEnvironment);
        assert_eq!(
            OscQueryError::Timeout(Duration::from_millis(100)),
            OscQueryError::Timeout(Duration::from_millis(100))
        );
        assert_ne!(
            OscQueryError::Timeout(Duration::from_millis(100)),
            OscQueryError::Timeout(Duration::from_millis(200))
        );
    }

    #[test]
    fn test_rgb_value_serialization() {
        // Test that RgbValue can be serialized/deserialized
        let color = RgbValue::new(100, 150, 200);
        let json = serde_json::to_string(&color).unwrap();
        let deserialized: RgbValue = serde_json::from_str(&json).unwrap();
        assert_eq!(color, deserialized);
    }

    #[test]
    fn test_rgb_value_const_new() {
        // Test that RgbValue::new is const
        const COLOR: RgbValue = RgbValue::new(255, 128, 0);
        assert_eq!(COLOR.r, 255);
        assert_eq!(COLOR.g, 128);
        assert_eq!(COLOR.b, 0);
    }

    // Integration test for OSC support functions
    // These tests verify the functions don't panic and return consistent values
    #[test]
    fn test_osc_support_functions_dont_panic() {
        // These should never panic, regardless of environment
        let _ = osc10_support();
        let _ = osc11_support();
        let _ = osc12_support();
    }

    #[test]
    fn test_osc_support_functions_consistent() {
        // OnceLock caching means repeated calls should return the same value
        let first = osc11_support();
        let second = osc11_support();
        assert_eq!(first, second, "Cached OSC support should be consistent");
    }

    // Mark integration tests that require a real terminal as ignored
    #[test]
    #[ignore = "requires real terminal - run manually"]
    fn test_actual_terminal_query_integration() {
        // This test would actually query the terminal
        // Only run manually in a real terminal environment
        if is_tty() && !is_ci() {
            let bg = bg_color();
            println!("Background color: {:?}", bg);
            let fg = text_color();
            println!("Foreground color: {:?}", fg);
            let cursor = cursor_color();
            println!("Cursor color: {:?}", cursor);
        }
    }
}
