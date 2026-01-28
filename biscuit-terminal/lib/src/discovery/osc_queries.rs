//! OSC (Operating System Command) queries for terminal colors.
//!
//! This module provides safe, environment-based detection of terminal colors
//! without performing actual terminal I/O queries (which can block or corrupt
//! terminal state).
//!
//! ## Strategy
//!
//! Instead of sending OSC escape sequences and parsing responses (which requires
//! raw mode and has timeout/blocking risks), we use:
//!
//! 1. Environment variables like `COLORFGBG`
//! 2. Known terminal application defaults
//! 3. macOS system appearance detection
//!
//! ## Examples
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

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::discovery::detection::{get_terminal_app, is_tty, TerminalApp};
use crate::discovery::os_detection::is_ci;

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

/// Query terminal color via environment heuristics (safe approach).
///
/// This function does NOT perform actual terminal I/O. Instead, it uses:
/// 1. `COLORFGBG` environment variable
/// 2. Terminal application defaults
/// 3. System appearance (macOS)
fn query_osc_color(code: u8) -> Option<RgbValue> {
    // Skip if not a TTY or in CI
    if !is_tty() {
        return None;
    }
    if is_ci() {
        return None;
    }

    // Try COLORFGBG environment variable (format: "fg;bg" as color indices)
    if let Ok(colorfgbg) = std::env::var("COLORFGBG")
        && let Some(color) = parse_colorfgbg(&colorfgbg, code)
    {
        return Some(color);
    }

    // Fall back to terminal app defaults
    let term_app = get_terminal_app();
    get_terminal_default_color(&term_app, code)
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
}
