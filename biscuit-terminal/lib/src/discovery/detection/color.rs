use std::env;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use termini::{NumberCapability, TermInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorMode {
    /// the background color is light, and text characters must be dark
    /// to provide adequate contrast
    Light,
    /// the background color is dark, and text characters must be light
    /// to provide the adequate contrast
    Dark,
    /// we were unable to detect the color mode but in this case a
    /// lot of functionality will treat this as "dark mode"
    Unknown,
}

impl ColorMode {
    /// Return the opposite known mode.
    ///
    /// inverts the color mode from light to dark and visa-versa.
    ///
    /// > **Note:** if color mode was `Unknown` then inversion results in light mode.
    pub const fn inverted(self) -> Self {
        match self {
            ColorMode::Light => ColorMode::Dark,
            ColorMode::Dark | ColorMode::Unknown => ColorMode::Light,
        }
    }

    /// Because we want `ColorMode` to represent a truthful state
    /// we have allowed an **Unknown** state but sometimes that state
    /// gets in the way and we need to resolve to `ColorMode::Unknown`
    /// to `ColorMode::Dark`
    pub const fn resolve_unknown(self) -> Self {
        match self {
            ColorMode::Unknown => ColorMode::Dark,
            _ => self,
        }
    }
}

/// Detect the terminal's color depth capability.
///
/// Detection strategy:
/// 1. Honor `NO_COLOR` (any non-empty value) by returning
///    [`ColorDepth::None`], unless `FORCE_COLOR` or `CLICOLOR_FORCE` is
///    also set — those explicit overrides take precedence per the
///    de-facto convention used by clap, supports-color, chalk, etc.
/// 2. Check `COLORTERM` environment variable for "truecolor" or "24bit"
/// 3. Query terminfo `MaxColors` capability
/// 4. Default to `ColorDepth::None` if detection fails — unless a force
///    override is set, which floors the result at [`ColorDepth::Basic`]
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
    // Honor NO_COLOR (https://no-color.org): any non-empty value disables
    // color output, unless FORCE_COLOR / CLICOLOR_FORCE explicitly opts
    // back in.
    let force_color = env::var_os("FORCE_COLOR")
        .filter(|v| !v.is_empty())
        .is_some()
        || env::var_os("CLICOLOR_FORCE")
            .filter(|v| !v.is_empty())
            .is_some();
    let no_color = env::var_os("NO_COLOR").filter(|v| !v.is_empty()).is_some();
    if no_color && !force_color {
        tracing::debug!(
            color_depth = ?ColorDepth::None,
            source = "NO_COLOR",
            "NO_COLOR environment variable set; disabling color output"
        );
        return ColorDepth::None;
    }

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
    let detected = match TermInfo::from_env() {
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
    };

    // A force override that only neutralized NO_COLOR is not a force: per the
    // supports-color/chalk convention the docblock cites, FORCE_COLOR must
    // yield color even when nothing is detectable — an environment with no
    // COLORTERM and an unresolvable TERM (hosted macOS lacks brew tmux's
    // `tmux-256color` terminfo entry) otherwise silently drops to None.
    if detected == ColorDepth::None && force_color {
        tracing::debug!(
            color_depth = ?ColorDepth::Basic,
            source = "FORCE_COLOR",
            "detection found no color support; FORCE_COLOR mandates basic ANSI"
        );
        return ColorDepth::Basic;
    }
    detected
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
    // Cache per process: the underlying `bg_color()` OSC probe is already
    // cached, and the macOS `AppleInterfaceStyle` fallback forks a subprocess.
    // The attached terminal's light/dark mode does not change within a process,
    // so repeated `Terminal` constructions must not re-pay either cost.
    static COLOR_MODE_CACHE: OnceLock<ColorMode> = OnceLock::new();
    *COLOR_MODE_CACHE.get_or_init(detect_color_mode)
}

/// Uncached color-mode detection backing [`color_mode`].
fn detect_color_mode() -> ColorMode {
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

    // macOS: Check AppleInterfaceStyle. Only worth the subprocess spawn when a
    // terminal is actually attached — in a fully-redirected context (`bg_color`
    // is `None` exactly there) the mode is unobservable, so fall through to the
    // default instead of forking `defaults`.
    #[cfg(target_os = "macos")]
    if super::dimensions::is_tty()
        && let Ok(output) = std::process::Command::new("defaults")
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

    // Default to Dark (most common for terminal users)
    ColorMode::Dark
}

// ---------------------------------------------------------------------------
// Boundary conversions to renderable types
// ---------------------------------------------------------------------------

use renderable::color::ColorDepth as RenderColorDepth;
use renderable::color::ColorMode as RenderColorMode;

impl From<ColorDepth> for RenderColorDepth {
    fn from(depth: ColorDepth) -> Self {
        match depth {
            ColorDepth::None => RenderColorDepth::None,
            ColorDepth::Minimal => RenderColorDepth::Minimal,
            ColorDepth::Basic => RenderColorDepth::Basic,
            ColorDepth::Enhanced => RenderColorDepth::Enhanced,
            ColorDepth::TrueColor => RenderColorDepth::TrueColor,
        }
    }
}

impl From<&ColorDepth> for RenderColorDepth {
    fn from(depth: &ColorDepth) -> Self {
        match depth {
            ColorDepth::None => RenderColorDepth::None,
            ColorDepth::Minimal => RenderColorDepth::Minimal,
            ColorDepth::Basic => RenderColorDepth::Basic,
            ColorDepth::Enhanced => RenderColorDepth::Enhanced,
            ColorDepth::TrueColor => RenderColorDepth::TrueColor,
        }
    }
}

impl From<ColorMode> for RenderColorMode {
    fn from(mode: ColorMode) -> Self {
        match mode {
            ColorMode::Light => RenderColorMode::Light,
            ColorMode::Dark => RenderColorMode::Dark,
            ColorMode::Unknown => RenderColorMode::Unknown,
        }
    }
}

impl From<&ColorMode> for RenderColorMode {
    fn from(mode: &ColorMode) -> Self {
        match mode {
            ColorMode::Light => RenderColorMode::Light,
            ColorMode::Dark => RenderColorMode::Dark,
            ColorMode::Unknown => RenderColorMode::Unknown,
        }
    }
}

impl From<RenderColorMode> for ColorMode {
    fn from(mode: RenderColorMode) -> Self {
        match mode {
            RenderColorMode::Light => ColorMode::Light,
            RenderColorMode::Dark => ColorMode::Dark,
            RenderColorMode::Unknown => ColorMode::Unknown,
        }
    }
}

impl From<&RenderColorMode> for ColorMode {
    fn from(mode: &RenderColorMode) -> Self {
        match mode {
            RenderColorMode::Light => ColorMode::Light,
            RenderColorMode::Dark => ColorMode::Dark,
            RenderColorMode::Unknown => ColorMode::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_depth_eq() {
        assert_eq!(ColorDepth::TrueColor, ColorDepth::TrueColor);
        assert_ne!(ColorDepth::None, ColorDepth::TrueColor);
    }

    /// FORCE_COLOR must yield color even when nothing is detectable — the
    /// hosted-macOS shape: no COLORTERM, a TERM terminfo cannot resolve.
    #[test]
    fn force_color_mandates_basic_when_nothing_is_detectable() {
        // SAFETY: nextest runs each test in its own process; no other thread
        // observes these process-wide vars. set_var is unsafe in 2024 edition.
        unsafe {
            std::env::remove_var("COLORTERM");
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("CLICOLOR_FORCE");
            std::env::set_var("TERM", "definitely-not-a-terminfo-entry");
            std::env::set_var("FORCE_COLOR", "1");
        }
        assert_ne!(color_depth(), ColorDepth::None);
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

    #[test]
    fn color_depth_conversion_round_trip() {
        for depth in [
            ColorDepth::None,
            ColorDepth::Minimal,
            ColorDepth::Basic,
            ColorDepth::Enhanced,
            ColorDepth::TrueColor,
        ] {
            let render: RenderColorDepth = depth.into();
            let render_ref: RenderColorDepth = (&depth).into();
            assert_eq!(render, render_ref);
        }
    }

    #[test]
    fn color_mode_conversion_round_trip() {
        for mode in [ColorMode::Light, ColorMode::Dark, ColorMode::Unknown] {
            let render: RenderColorMode = mode.into();
            let render_ref: RenderColorMode = (&mode).into();
            assert_eq!(render, render_ref);
        }
    }

    /// `color_mode()` is cached per process (finding 21) so repeated `Terminal`
    /// constructions do not re-probe the background color or re-fork the macOS
    /// `defaults read` subprocess. The value must be stable across calls.
    #[test]
    fn color_mode_is_stable_across_calls() {
        assert_eq!(color_mode(), color_mode(), "cached color_mode must be stable");
    }
}
