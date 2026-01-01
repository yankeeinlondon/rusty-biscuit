//! Syntax highlighting infrastructure for markdown code blocks and prose.
//!
//! This module provides theme enumeration, theme pairing (light/dark),
//! and grammar loading utilities for syntax highlighting using syntect.

mod grammars;
mod scope_cache;
mod themes;

pub use themes::{ColorMode, ThemePair};

use syntect::highlighting::Theme as SyntectTheme;
use syntect::parsing::SyntaxSet;

/// Primary API for syntax highlighting with theme support.
///
/// ## Examples
///
/// ```
/// use shared::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
///
/// let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
/// ```
#[derive(Debug)]
pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme: SyntectTheme,
    theme_pair: ThemePair,
    color_mode: ColorMode,
}

impl CodeHighlighter {
    /// Creates a new code highlighter with the specified theme pair and color mode.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
    ///
    /// let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
    /// ```
    pub fn new(theme_pair: ThemePair, color_mode: ColorMode) -> Self {
        let syntax_set = grammars::load_syntax_set();
        let theme = themes::load_theme(theme_pair, color_mode);

        Self {
            syntax_set,
            theme,
            theme_pair,
            color_mode,
        }
    }

    /// Returns a reference to the syntax set.
    pub fn syntax_set(&self) -> &SyntaxSet {
        &self.syntax_set
    }

    /// Returns a reference to the current theme.
    pub fn theme(&self) -> &SyntectTheme {
        &self.theme
    }

    /// Returns the current theme pair.
    pub fn theme_pair(&self) -> ThemePair {
        self.theme_pair
    }

    /// Returns the current color mode.
    pub fn color_mode(&self) -> ColorMode {
        self.color_mode
    }

    /// Updates the color mode and reloads the theme.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
    ///
    /// let mut highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
    /// highlighter.set_color_mode(ColorMode::Light);
    /// ```
    pub fn set_color_mode(&mut self, color_mode: ColorMode) {
        if self.color_mode != color_mode {
            self.color_mode = color_mode;
            self.theme = themes::load_theme(self.theme_pair, color_mode);
        }
    }

    /// Updates the theme pair and reloads the theme.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::highlighting::{CodeHighlighter, ThemePair, ColorMode};
    ///
    /// let mut highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
    /// highlighter.set_theme_pair(ThemePair::Solarized);
    /// ```
    pub fn set_theme_pair(&mut self, theme_pair: ThemePair) {
        if self.theme_pair != theme_pair {
            self.theme_pair = theme_pair;
            self.theme = themes::load_theme(theme_pair, self.color_mode);
        }
    }
}

impl Default for CodeHighlighter {
    fn default() -> Self {
        Self::new(ThemePair::Base16Ocean, ColorMode::Dark)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_highlighter_new() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        assert_eq!(highlighter.theme_pair(), ThemePair::Github);
        assert_eq!(highlighter.color_mode(), ColorMode::Dark);
    }

    #[test]
    fn test_code_highlighter_default() {
        let highlighter = CodeHighlighter::default();
        assert_eq!(highlighter.theme_pair(), ThemePair::Base16Ocean);
        assert_eq!(highlighter.color_mode(), ColorMode::Dark);
    }

    #[test]
    fn test_set_color_mode() {
        let mut highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        highlighter.set_color_mode(ColorMode::Light);
        assert_eq!(highlighter.color_mode(), ColorMode::Light);
    }

    #[test]
    fn test_set_theme_pair() {
        let mut highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        highlighter.set_theme_pair(ThemePair::Solarized);
        assert_eq!(highlighter.theme_pair(), ThemePair::Solarized);
    }

    #[test]
    fn test_syntax_set_available() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let syntax_set = highlighter.syntax_set();
        assert!(syntax_set.find_syntax_by_extension("rs").is_some());
    }

    #[test]
    fn test_theme_available() {
        let highlighter = CodeHighlighter::new(ThemePair::Github, ColorMode::Dark);
        let theme = highlighter.theme();
        assert!(theme.settings.background.is_some());
    }
}
