//! Per-terminal font parsing trait and dispatch helpers.

use std::fs;
use std::path::PathBuf;

use crate::discovery::config_paths::get_terminal_config_path;
use crate::discovery::detection::TerminalApp;

use super::alacritty::{parse_alacritty_font_name, parse_alacritty_font_size};
use super::ghostty::{parse_ghostty_font_name, parse_ghostty_font_size};
use super::kitty::{parse_kitty_font_name, parse_kitty_font_size};
use super::wezterm::{parse_wezterm_font_name, parse_wezterm_font_size};

/// Parser interface for terminal config formats.
///
/// Each supported terminal emulator implements this trait by providing
/// content parsers for its config format. The dispatch helpers below
/// route a [`TerminalApp`] through the appropriate parser.
pub trait TerminalFontParser {
    /// Extract the configured font family name from the given config content.
    fn parse_font_name(content: &str) -> Option<String>;

    /// Extract the configured font size (points) from the given config content.
    fn parse_font_size(content: &str) -> Option<u32>;
}

/// Type alias for font name config parser entries.
type FontNameConfigEntry = (PathBuf, fn(&str) -> Option<String>);

/// Type alias for font size config parser entries.
type FontSizeConfigEntry = (PathBuf, fn(&str) -> Option<u32>);

/// Dispatch font-name parsing for the given [`TerminalApp`].
///
/// Returns `None` for terminals without a registered parser; callers should
/// fall back to [`fallback_font_name_scan`] when this returns `None`.
pub(super) fn parse_font_name_for(app: &TerminalApp, content: &str) -> Option<String> {
    match app {
        TerminalApp::Wezterm => parse_wezterm_font_name(content),
        TerminalApp::Ghostty => parse_ghostty_font_name(content),
        TerminalApp::Kitty => parse_kitty_font_name(content),
        TerminalApp::Alacritty => parse_alacritty_font_name(content),
        _ => None,
    }
}

/// Dispatch font-size parsing for the given [`TerminalApp`].
pub(super) fn parse_font_size_for(app: &TerminalApp, content: &str) -> Option<u32> {
    match app {
        TerminalApp::Wezterm => parse_wezterm_font_size(content),
        TerminalApp::Ghostty => parse_ghostty_font_size(content),
        TerminalApp::Kitty => parse_kitty_font_size(content),
        TerminalApp::Alacritty => parse_alacritty_font_size(content),
        _ => None,
    }
}

/// Read the terminal's config file (if available) and run the given parser
/// against its content. Returns `None` if the path is missing or unreadable.
pub(super) fn read_and_parse<T>(app: &TerminalApp, parser: fn(&TerminalApp, &str) -> Option<T>) -> Option<T> {
    let config_path = get_terminal_config_path(app)?;

    if !config_path.exists() {
        tracing::trace!("read_and_parse(): config file does not exist: {:?}", config_path);
        return None;
    }

    let content = fs::read_to_string(&config_path).ok()?;
    parser(app, &content)
}

/// Fallback font name detection by scanning known config file locations.
///
/// This is used when terminal detection fails or the detected terminal
/// doesn't have a config parser. It tries common config file locations
/// for popular terminals.
pub(super) fn fallback_font_name_scan() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let home = std::path::Path::new(&home);

    let configs: Vec<FontNameConfigEntry> = vec![
        (
            home.join(".config/alacritty/alacritty.toml"),
            parse_alacritty_font_name,
        ),
        (
            home.join(".config/alacritty/alacritty.yml"),
            parse_alacritty_font_name,
        ),
        (home.join(".config/kitty/kitty.conf"), parse_kitty_font_name),
        (
            home.join(".config/wezterm/wezterm.lua"),
            parse_wezterm_font_name,
        ),
        (home.join(".wezterm.lua"), parse_wezterm_font_name),
        (home.join(".config/ghostty/config"), parse_ghostty_font_name),
    ];

    for (path, parser) in configs {
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
            && let Some(font) = parser(&content)
        {
            tracing::trace!(
                "fallback_font_name_scan(): found font '{}' in {:?}",
                font,
                path
            );
            return Some(font);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(font) = super::iterm2::query_iterm2_font_name() {
            tracing::trace!(
                "fallback_font_name_scan(): found font '{}' from iTerm2 preferences",
                font
            );
            return Some(font);
        }
    }

    tracing::trace!("fallback_font_name_scan(): no font found in any config files");
    None
}

/// Fallback font size detection by scanning known config file locations.
pub(super) fn fallback_font_size_scan() -> Option<u32> {
    let home = std::env::var("HOME").ok()?;
    let home = std::path::Path::new(&home);

    let configs: Vec<FontSizeConfigEntry> = vec![
        (
            home.join(".config/alacritty/alacritty.toml"),
            parse_alacritty_font_size,
        ),
        (
            home.join(".config/alacritty/alacritty.yml"),
            parse_alacritty_font_size,
        ),
        (home.join(".config/kitty/kitty.conf"), parse_kitty_font_size),
        (
            home.join(".config/wezterm/wezterm.lua"),
            parse_wezterm_font_size,
        ),
        (home.join(".wezterm.lua"), parse_wezterm_font_size),
        (home.join(".config/ghostty/config"), parse_ghostty_font_size),
    ];

    for (path, parser) in configs {
        if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
            && let Some(size) = parser(&content)
        {
            tracing::trace!(
                "fallback_font_size_scan(): found size {} in {:?}",
                size,
                path
            );
            return Some(size);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(size) = super::iterm2::query_iterm2_font_size() {
            tracing::trace!(
                "fallback_font_size_scan(): found size {} from iTerm2 preferences",
                size
            );
            return Some(size);
        }
    }

    tracing::trace!("fallback_font_size_scan(): no font size found in any config files");
    None
}
