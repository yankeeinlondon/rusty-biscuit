//! Font detection utilities for terminal applications.
//!
//! This module provides functions for detecting font-related terminal capabilities,
//! including font name, font size, Nerd Font detection, and ligature support.
//!
//! ## Detection Strategy
//!
//! Font detection uses config file parsing or system queries for supported terminals:
//! - **Wezterm**: Parses `wezterm.lua` for `config.font` and `config.font_size`
//! - **iTerm2** (macOS): Queries macOS preferences via `defaults read`
//! - **Kitty**: Parses `kitty.conf` for `font_family` and `font_size`
//! - **Alacritty**: Parses TOML for `[font.normal] family` and `[font] size`
//! - **Ghostty**: Queries `ghostty +show-config` for font settings
//!
//! ## Terminal-Specific Support
//!
//! | Terminal | Font Name | Font Size | Notes |
//! |----------|-----------|-----------|-------|
//! | Wezterm | ✅ | ✅ | Full support via config parsing |
//! | iTerm2 | ✅ | ✅ | Full support via macOS preferences |
//! | Kitty | ✅ | ✅ | Parses `font_family` and `font_size` from kitty.conf |
//! | Alacritty | ✅ | ✅ | Parses TOML config; detection improved via config file fallback |
//! | Ghostty | ⚠️ | ⚠️ | Tries `ghostty +show-config`; may not report font settings |
//!
//! ## Submodules
//!
//! - [`types`] — `FontLigature`, `WindowSizePixels`, `CellSize`
//! - [`nerd`] — Nerd Font name + env detection
//! - [`window_size`] — CSI 14 t window pixel queries and cell-size derivation
//! - [`parser`] — `TerminalFontParser` trait and dispatch helpers
//! - Per-terminal parsers: [`wezterm`], [`ghostty`], [`kitty`], [`alacritty`], [`iterm2`]
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::discovery::fonts::{font_name, font_size, detect_nerd_font, ligature_support_likely};
//!
//! if let Some(name) = font_name() {
//!     println!("Font: {}", name);
//! }
//! if let Some(size) = font_size() {
//!     println!("Size: {}pt", size);
//! }
//! if detect_nerd_font() == Some(true) {
//!     println!("Nerd Font icons available!");
//! }
//! if ligature_support_likely() {
//!     println!("Ligatures likely supported");
//! }
//! ```

pub mod alacritty;
pub mod ghostty;
pub mod iterm2;
pub mod kitty;
pub mod nerd;
pub mod parser;
pub mod types;
pub mod wezterm;
pub mod window_size;

pub use nerd::{detect_nerd_font, is_nerd_font_name};
pub use parser::TerminalFontParser;
pub use types::{CellSize, FontLigature, WindowSizePixels};
pub use window_size::{cell_size, window_size_pixels};

use crate::discovery::detection::{TerminalApp, get_terminal_app, is_tty};

/// Get the font name used by the terminal.
///
/// Detects the font by parsing the terminal's configuration file or
/// querying system preferences (for macOS terminals like iTerm2).
///
/// ## Supported Terminals
///
/// | Terminal | Config Format | Font Setting |
/// |----------|--------------|--------------|
/// | Wezterm | Lua | `config.font = wezterm.font("Name")` |
/// | Ghostty | Key=Value | `font-family = Name` |
/// | Kitty | Conf | `font_family Name` |
/// | Alacritty | TOML | `[font.normal] family = "Name"` |
/// | iTerm2 | macOS prefs | `defaults read com.googlecode.iterm2` |
///
/// ## Returns
///
/// - `Some(String)` - The font family name from config
/// - `None` - If config not found, not readable, or font not specified
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::font_name;
///
/// if let Some(font) = font_name() {
///     println!("Terminal font: {}", font);
/// } else {
///     println!("Font not detected - assuming monospace");
/// }
/// ```
pub fn font_name() -> Option<String> {
    let app = get_terminal_app();

    // Handle terminals that use macOS preferences instead of config files
    #[cfg(target_os = "macos")]
    if matches!(app, TerminalApp::ITerm2) {
        let result = iterm2::query_iterm2_font_name();
        if result.is_some() {
            tracing::debug!("font_name(): detected {:?} from iTerm2 preferences", result);
        }
        return result;
    }

    let result = parser::read_and_parse(&app, parser::parse_font_name_for);

    if result.is_some() {
        tracing::debug!("font_name(): detected {:?}", result);
        return result;
    }

    // If primary detection failed, try fallback scan
    tracing::debug!("font_name(): primary detection failed, trying fallback scan");
    parser::fallback_font_name_scan()
}

/// Get the font size in points.
///
/// Detects the font size by parsing the terminal's configuration file or
/// querying system preferences (for macOS terminals like iTerm2).
///
/// ## Supported Terminals
///
/// | Terminal | Config Format | Size Setting |
/// |----------|--------------|--------------|
/// | Wezterm | Lua | `config.font_size = 13` |
/// | Ghostty | Key=Value | `font-size = 14` |
/// | Kitty | Conf | `font_size 14.0` |
/// | Alacritty | TOML | `[font] size = 12` |
/// | iTerm2 | macOS prefs | `defaults read com.googlecode.iterm2` |
///
/// ## Returns
///
/// - `Some(u32)` - The font size in points
/// - `None` - If config not found, not readable, or size not specified
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::font_size;
///
/// if let Some(size) = font_size() {
///     println!("Font size: {}pt", size);
/// } else {
///     println!("Font size not detected");
/// }
/// ```
pub fn font_size() -> Option<u32> {
    let app = get_terminal_app();

    // Handle terminals that use macOS preferences instead of config files
    #[cfg(target_os = "macos")]
    if matches!(app, TerminalApp::ITerm2) {
        let result = iterm2::query_iterm2_font_size();
        if result.is_some() {
            tracing::debug!("font_size(): detected {:?} from iTerm2 preferences", result);
        }
        return result;
    }

    let result = parser::read_and_parse(&app, parser::parse_font_size_for);

    if result.is_some() {
        tracing::debug!("font_size(): detected {:?}", result);
        return result;
    }

    // If primary detection failed, try fallback scan
    tracing::debug!("font_size(): primary detection failed, trying fallback scan");
    parser::fallback_font_size_scan()
}

/// Get the font ligatures enabled in the terminal.
///
/// Font ligatures are special glyphs that combine multiple characters
/// into a single glyph (e.g., `fi`, `fl`, `!=`, `=>`).
///
/// ## Why Detection Is Difficult
///
/// - The terminal's font rendering engine handles ligatures internally
/// - No standard escape sequence to query ligature support
/// - Support depends on both the terminal AND the font being used
/// - Users can enable/disable ligatures in terminal settings
///
/// ## Returns
///
/// - `None` - Ligature support cannot be reliably detected
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::font_ligatures;
///
/// assert!(font_ligatures().is_none());
/// println!("Ligature detection not available - assume support for modern terminals");
/// ```
pub fn font_ligatures() -> Option<Vec<FontLigature>> {
    // There is no reliable way to detect which ligatures are enabled.
    tracing::debug!("font_ligatures() returns None - no reliable way to detect enabled ligatures");
    None
}

/// Check if the terminal is likely to support font ligatures.
///
/// This is a heuristic based on the detected terminal emulator.
/// It does **not** check if ligatures are actually enabled or if
/// the current font supports them.
///
/// ## Returns
///
/// - `true` - Terminal is likely to support ligatures
/// - `false` - Terminal typically does not support ligatures
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::fonts::ligature_support_likely;
///
/// if ligature_support_likely() {
///     println!("Terminal likely supports ligatures (e.g., ->, =>, !=)");
/// } else {
///     println!("Terminal typically does not support ligatures");
/// }
/// ```
pub fn ligature_support_likely() -> bool {
    // If not a TTY, no styling support
    if !is_tty() {
        return false;
    }

    let term_app = get_terminal_app();

    matches!(
        term_app,
        TerminalApp::ITerm2
            | TerminalApp::Kitty
            | TerminalApp::Alacritty
            | TerminalApp::Wezterm
            | TerminalApp::Ghostty
            | TerminalApp::Warp
            | TerminalApp::VsCode
            | TerminalApp::Wast
            | TerminalApp::Contour
            | TerminalApp::Foot
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_name_does_not_panic() {
        let _ = font_name();
    }

    #[test]
    fn test_font_size_does_not_panic() {
        let _ = font_size();
    }

    #[test]
    fn test_font_ligatures_returns_none() {
        assert!(font_ligatures().is_none());
    }

    #[test]
    fn test_ligature_support_likely_does_not_panic() {
        let _ = ligature_support_likely();
    }
}
