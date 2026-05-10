use std::env;

use serde::{Deserialize, Serialize};
use termini::{NumberCapability, TermInfo};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorDepth {
    /// no color support
    None,
    /// 8 colors
    Minimal,
    /// 16 colors (8 normal plus "bright" variants)
    Basic,
    /// 256 color palette (8 bit)
    Enhanced,
    /// 16 million colors (24 bit)
    TrueColor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorMode {
    /// the background color is light, and text characters must be dark
    /// to provide adequate contrast
    Light,
    /// the background color is dark, and text characters must be light
    /// to provide the adequate contrast
    Dark,
    Unknown,
}

/// Detect the terminal's color depth capability.
///
/// Detection strategy:
/// 1. Check `COLORTERM` environment variable for "truecolor" or "24bit"
/// 2. Query terminfo `MaxColors` capability
/// 3. Default to `ColorDepth::None` if detection fails
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{color_depth, ColorDepth};
///
/// match color_depth() {
///     ColorDepth::TrueColor => println!("24-bit color (16M colors)"),
///     ColorDepth::Enhanced => println!("256 colors"),
///     ColorDepth::Basic => println!("16 colors"),
///     ColorDepth::Minimal => println!("8 colors"),
///     ColorDepth::None => println!("No color support"),
/// }
/// ```
pub fn color_depth() -> ColorDepth {
    // Check COLORTERM environment variable first
    if let Ok(colorterm) = env::var("COLORTERM") {
        let colorterm_lower = colorterm.to_lowercase();
        if colorterm_lower == "truecolor" || colorterm_lower == "24bit" {
            tracing::debug!(
                color_depth = ?ColorDepth::TrueColor,
                source = "COLORTERM",
                colorterm = %colorterm,
                "Detected truecolor support from COLORTERM env var"
            );
            return ColorDepth::TrueColor;
        }
    }

    // Fallback to terminfo
    match TermInfo::from_env() {
        Ok(term_info) => {
            // Query the MaxColors capability
            let depth = term_info
                .number_cap(NumberCapability::MaxColors)
                .map(|n| n as u32)
                .unwrap_or(0);

            let color_depth = match depth {
                d if d >= 16_777_216 => ColorDepth::TrueColor,
                d if d >= 256 => ColorDepth::Enhanced,
                d if d >= 16 => ColorDepth::Basic,
                d if d >= 8 => ColorDepth::Minimal,
                _ => ColorDepth::None,
            };

            tracing::debug!(
                ?color_depth,
                source = "terminfo",
                "Detected color depth from terminfo"
            );
            color_depth
        }
        Err(e) => {
            tracing::warn!(
                color_depth = ?ColorDepth::None,
                source = "fallback",
                error = %e,
                "Failed to query terminfo, defaulting to no color"
            );
            ColorDepth::None
        }
    }
}

/// Whether the terminal is in "light" or "dark" mode.
///
/// Detection strategy:
/// 1. Try to get background color from OSC queries and determine from luminance
/// 2. Check `DARK_MODE` environment variable
/// 3. On macOS, check `AppleInterfaceStyle` system preference
/// 4. Default to Dark (most common for terminal users)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{color_mode, ColorMode};
///
/// match color_mode() {
///     ColorMode::Light => println!("Light mode - use dark text"),
///     ColorMode::Dark => println!("Dark mode - use light text"),
///     ColorMode::Unknown => println!("Unknown - use default colors"),
/// }
/// ```
pub fn color_mode() -> ColorMode {
    // Try to get background color and determine from luminance
    if let Some(bg) = crate::discovery::osc_queries::bg_color() {
        let luminance = bg.luminance();
        if luminance > 0.5 {
            return ColorMode::Light;
        } else {
            return ColorMode::Dark;
        }
    }

    // Check common environment variables
    if let Ok(mode) = env::var("DARK_MODE") {
        if mode == "0" || mode.to_lowercase() == "false" {
            return ColorMode::Light;
        }
        if mode == "1" || mode.to_lowercase() == "true" {
            return ColorMode::Dark;
        }
    }

    // macOS: Check AppleInterfaceStyle
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().to_lowercase() == "dark" {
                    return ColorMode::Dark;
                }
            }
            // If command succeeds but no "Dark" value, it's Light mode
            // (AppleInterfaceStyle is only set when Dark mode is active)
            return ColorMode::Light;
        }
    }

    // Default to Dark (most common for terminal users)
    ColorMode::Dark
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_depth_eq() {
        assert_eq!(ColorDepth::TrueColor, ColorDepth::TrueColor);
        assert_ne!(ColorDepth::None, ColorDepth::TrueColor);
    }

    #[test]
    fn test_color_mode_matches() {
        assert!(matches!(ColorMode::Dark, ColorMode::Dark));
        assert!(!matches!(ColorMode::Light, ColorMode::Dark));
    }

    #[test]
    fn test_color_depth_serialize() {
        let depth = ColorDepth::TrueColor;
        let json = serde_json::to_string(&depth).unwrap();
        assert!(json.contains("TrueColor"));
    }

    #[test]
    fn test_color_mode_serialize() {
        let mode = ColorMode::Dark;
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("Dark"));
    }
}
