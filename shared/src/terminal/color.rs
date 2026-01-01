//! Terminal color detection utilities
//!
//! This module provides functions for detecting the color depth and capabilities
//! of the current terminal.

use std::env;
use termini::{NumberCapability, StringCapability, TermInfo};

/// Returns the maximum number of colors the terminal supports.
///
/// This function checks the COLORTERM environment variable first for truecolor
/// support, then falls back to querying terminfo for the number of colors.
///
/// ## Returns
///
/// The number of colors supported:
/// - 16,777,216 if COLORTERM indicates truecolor/24bit support
/// - The value from terminfo's MaxColors capability if available
/// - 0 if no color support can be detected
///
/// ## Examples
///
/// ```
/// use shared::terminal::color_depth;
///
/// let depth = color_depth();
/// if depth >= 16_777_216 {
///     println!("Terminal supports truecolor!");
/// } else if depth >= 256 {
///     println!("Terminal supports 256 colors");
/// } else if depth >= 8 {
///     println!("Terminal supports basic colors");
/// } else {
///     println!("No color support detected");
/// }
/// ```
pub fn color_depth() -> u32 {
    // Check COLORTERM environment variable first
    if let Ok(colorterm) = env::var("COLORTERM") {
        let colorterm_lower = colorterm.to_lowercase();
        if colorterm_lower == "truecolor" || colorterm_lower == "24bit" {
            tracing::info!(
                color_depth = 16_777_216,
                source = "COLORTERM",
                colorterm = %colorterm,
                "Detected truecolor support from COLORTERM env var"
            );
            return 16_777_216; // 2^24 colors
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
            tracing::info!(
                color_depth = depth,
                source = "terminfo",
                "Detected color depth from terminfo"
            );
            depth
        }
        Err(e) => {
            tracing::info!(
                color_depth = 0,
                source = "fallback",
                error = %e,
                "Failed to query terminfo, defaulting to no color"
            );
            0
        }
    }
}

/// Returns whether the terminal supports setting the foreground color.
///
/// This function checks terminfo for the presence of the SetForeground capability.
///
/// ## Returns
///
/// - `true` if the terminal supports setting foreground colors
/// - `false` if the capability is not available or terminfo cannot be queried
///
/// ## Examples
///
/// ```
/// use shared::terminal::supports_setting_foreground;
///
/// if supports_setting_foreground() {
///     println!("\x1b[31mThis text is red!\x1b[0m");
/// } else {
///     println!("This text has no color");
/// }
/// ```
pub fn supports_setting_foreground() -> bool {
    match TermInfo::from_env() {
        Ok(term_info) => {
            // Check for SetForeground capability
            term_info
                .utf8_string_cap(StringCapability::SetForeground)
                .is_some()
        }
        Err(_) => false,
    }
}
