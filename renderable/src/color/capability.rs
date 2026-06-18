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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCodeContext {
    width: u32,
    color_depth: ColorDepth,
    color_mode: ColorMode,
    code_theme_name: Option<String>,
    line_numbers: bool,
    page_text_color: Option<(u8, u8, u8)>,
    page_background_color: Option<(u8, u8, u8)>,
}

impl TerminalCodeContext {
    /// Creates a new terminal code context.
    ///
    /// The optional code-theme name is left unset; use
    /// [`with_code_theme_name`](Self::with_code_theme_name) to carry one.
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
            code_theme_name: None,
            line_numbers: false,
            page_text_color: None,
            page_background_color: None,
        }
    }

    /// Sets whether the code block should render a line-number gutter.
    ///
    /// Off by default; a consumer (e.g. darkmatter's page line-number toggle)
    /// turns it on so the [`CodeRenderer`] emits the gutter.
    ///
    /// [`CodeRenderer`]: crate::tree::CodeRenderer
    #[inline]
    #[must_use]
    pub const fn with_line_numbers(mut self, line_numbers: bool) -> Self {
        self.line_numbers = line_numbers;
        self
    }

    /// Sets the terminal page surface colors carried to the code renderer.
    ///
    /// `page_text_color` is the detected foreground used by the surrounding
    /// page and `page_background_color` is the detected page background. Code
    /// renderers can use these when they need the concrete surrounding surface;
    /// theme-based renderers should still resolve a complete theme variant
    /// rather than mixing page colors with unrelated syntax token colors.
    #[inline]
    #[must_use]
    pub const fn with_page_surface(
        mut self,
        page_text_color: Option<(u8, u8, u8)>,
        page_background_color: Option<(u8, u8, u8)>,
    ) -> Self {
        self.page_text_color = page_text_color;
        self.page_background_color = page_background_color;
        self
    }

    /// Returns whether the code block should render a line-number gutter.
    #[inline]
    #[must_use]
    pub const fn line_numbers(&self) -> bool {
        self.line_numbers
    }

    /// Returns the detected surrounding page text color, when known.
    #[inline]
    #[must_use]
    pub const fn page_text_color(&self) -> Option<(u8, u8, u8)> {
        self.page_text_color
    }

    /// Returns the detected surrounding page background color, when known.
    #[inline]
    #[must_use]
    pub const fn page_background_color(&self) -> Option<(u8, u8, u8)> {
        self.page_background_color
    }

    /// Sets the optional code-theme name carried to the [`CodeRenderer`].
    ///
    /// This is a plain string (e.g. `"github"`, `"dracula"`) so `renderable`
    /// stays free of any consumer-specific theme types. The consuming
    /// [`CodeRenderer`] maps it back to its own theme representation.
    ///
    /// [`CodeRenderer`]: crate::tree::CodeRenderer
    #[inline]
    #[must_use]
    pub fn with_code_theme_name(mut self, name: Option<String>) -> Self {
        self.code_theme_name = name;
        self
    }

    /// Returns the optional code-theme name, if a consumer set one.
    #[inline]
    #[must_use]
    pub fn code_theme_name(&self) -> Option<&str> {
        self.code_theme_name.as_deref()
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
        let ctx = TerminalCodeContext::new(80, ColorDepth::Enhanced, ColorMode::Light)
            .with_page_surface(Some((10, 20, 30)), Some((40, 50, 60)));
        assert_eq!(ctx.width(), 80);
        assert_eq!(ctx.color_depth(), ColorDepth::Enhanced);
        assert_eq!(ctx.color_mode(), ColorMode::Light);
        assert_eq!(ctx.page_text_color(), Some((10, 20, 30)));
        assert_eq!(ctx.page_background_color(), Some((40, 50, 60)));
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
