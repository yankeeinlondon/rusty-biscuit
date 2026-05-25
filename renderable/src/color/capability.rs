//! Terminal color capability descriptors.
//!
//! This module defines **capability descriptors** that describe what a terminal
//! can display, distinct from the color *value* types (`Color`, `HdrColor`, etc.)
//! that represent specific colors. ANSI escape code emission lives in
//! `biscuit-terminal`; these types are purely descriptive.

use serde::{Deserialize, Serialize};

/// The color depth a terminal supports.
///
/// Describes the palette size / precision available for rendering. Code
/// renderers use this to choose between plain output, ANSI-16, ANSI-256, or
/// true-color escape sequences.
///
/// ## Examples
///
/// ```
/// use renderable::color::ColorDepth;
///
/// let depth = ColorDepth::TrueColor;
/// assert_ne!(depth, ColorDepth::None);
/// ```
///
/// ## Notes
///
/// - `None`: No color support (e.g., `NO_COLOR` set, dumb terminal).
/// - `Minimal`: 8-color ANSI palette (colors 0–7).
/// - `Basic`: 16-color ANSI palette (colors 0–15, including bright variants).
/// - `Enhanced`: 256-color extended palette.
/// - `TrueColor`: 24-bit RGB (16.7 million colors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorDepth {
    /// No color support.
    None,
    /// 8-color ANSI palette.
    Minimal,
    /// 16-color ANSI palette.
    #[default]
    Basic,
    /// 256-color extended palette.
    Enhanced,
    /// 24-bit true color.
    TrueColor,
}

/// The background color mode of a terminal.
///
/// Used to select appropriate foreground colors for contrast. For example, a
/// dark background typically needs light foreground colors.
///
/// ## Examples
///
/// ```
/// use renderable::color::ColorMode;
///
/// let mode = ColorMode::Dark;
/// assert_ne!(mode, ColorMode::Light);
/// ```
///
/// ## Notes
///
/// - `Light`: Light background (dark text preferred).
/// - `Dark`: Dark background (light text preferred).
/// - `Unknown`: The terminal renderer could not determine the background. This
///   is a faithful signal, not an error. A code renderer MUST NOT run ambient
///   capability detection to resolve `Unknown` — doing so reintroduces the
///   inconsistency the supplied context exists to prevent. When a renderer
///   must pick a concrete light/dark treatment, it resolves `Unknown` against
///   its own configured option, and defaults to [`ColorMode::Dark`] when no
///   such option is configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    /// Light background.
    Light,
    /// Dark background.
    #[default]
    Dark,
    /// Unknown background (detection failed or unavailable).
    Unknown,
}

/// Terminal context for code rendering.
///
/// Bundles the information a [`CodeRenderer`] needs to produce
/// terminal-appropriate output without performing ambient capability detection.
///
/// ## Examples
///
/// ```
/// use renderable::color::{ColorDepth, ColorMode, TerminalCodeContext};
///
/// let ctx = TerminalCodeContext::new(120, ColorDepth::TrueColor, ColorMode::Dark);
/// assert_eq!(ctx.width(), 120);
/// assert_eq!(ctx.color_depth(), ColorDepth::TrueColor);
/// assert_eq!(ctx.color_mode(), ColorMode::Dark);
/// ```
///
/// ## Notes
///
/// This struct lives in `renderable` so that [`CodeRenderer`] (also in
/// `renderable`) can accept it without creating a dependency cycle.
/// `biscuit-terminal` populates the context from its `TerminalRenderContext`
/// and passes it through.
///
/// [`CodeRenderer`]: crate::tree::CodeRenderer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCodeContext {
    width: u32,
    color_depth: ColorDepth,
    color_mode: ColorMode,
}

impl TerminalCodeContext {
    /// Creates a new terminal code context.
    ///
    /// ## Arguments
    ///
    /// - `width`: Available width in columns for the code block.
    /// - `color_depth`: The terminal's color capability.
    /// - `color_mode`: The terminal's background color mode.
    #[inline]
    #[must_use]
    pub const fn new(width: u32, color_depth: ColorDepth, color_mode: ColorMode) -> Self {
        Self {
            width,
            color_depth,
            color_mode,
        }
    }

    /// Returns the available width in columns.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the terminal's color depth.
    #[inline]
    #[must_use]
    pub const fn color_depth(&self) -> ColorDepth {
        self.color_depth
    }

    /// Returns the terminal's background color mode.
    #[inline]
    #[must_use]
    pub const fn color_mode(&self) -> ColorMode {
        self.color_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_depth_default_is_basic() {
        assert_eq!(ColorDepth::default(), ColorDepth::Basic);
    }

    #[test]
    fn color_mode_default_is_dark() {
        assert_eq!(ColorMode::default(), ColorMode::Dark);
    }

    #[test]
    fn terminal_code_context_accessors() {
        let ctx = TerminalCodeContext::new(80, ColorDepth::Enhanced, ColorMode::Light);
        assert_eq!(ctx.width(), 80);
        assert_eq!(ctx.color_depth(), ColorDepth::Enhanced);
        assert_eq!(ctx.color_mode(), ColorMode::Light);
    }

    #[test]
    fn color_depth_serde_round_trip() {
        for depth in [
            ColorDepth::None,
            ColorDepth::Minimal,
            ColorDepth::Basic,
            ColorDepth::Enhanced,
            ColorDepth::TrueColor,
        ] {
            let json = serde_json::to_string(&depth).unwrap();
            let decoded: ColorDepth = serde_json::from_str(&json).unwrap();
            assert_eq!(depth, decoded);
        }
    }

    #[test]
    fn color_mode_serde_round_trip() {
        for mode in [ColorMode::Light, ColorMode::Dark, ColorMode::Unknown] {
            let json = serde_json::to_string(&mode).unwrap();
            let decoded: ColorMode = serde_json::from_str(&json).unwrap();
            assert_eq!(mode, decoded);
        }
    }
}
