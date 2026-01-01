//! Theme enumeration, pairing, and loading infrastructure.
//!
//! Provides a curated set of theme pairs that adapt to light/dark modes,
//! with descriptions and utilities for loading syntect themes.

use lazy_static::lazy_static;
use std::collections::HashMap;
use syntect::highlighting::Theme as SyntectTheme;
use two_face::theme::{
    EmbeddedLazyThemeSet, EmbeddedThemeName, extra as extra_themes,
};

/// Color mode for theme resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorMode {
    /// Light color mode.
    Light,
    /// Dark color mode.
    Dark,
}

/// Primary API surface - theme pairs that adapt to light/dark modes.
///
/// Each variant represents a paired light/dark theme combination.
/// Use `resolve()` to get the appropriate theme for a color mode.
///
/// ## Examples
///
/// ```
/// use shared::markdown::highlighting::{ThemePair, ColorMode};
///
/// let theme = ThemePair::Github;
/// let desc = theme.description(ColorMode::Dark);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ThemePair {
    /// Base16 Ocean theme pair.
    Base16Ocean,
    /// GitHub theme pair.
    Github,
    /// Gruvbox theme pair.
    Gruvbox,
    /// OneHalf theme pair.
    OneHalf,
    /// Solarized theme pair.
    Solarized,
    /// Nord theme pair (dark only).
    Nord,
    /// Dracula theme pair (dark only).
    Dracula,
    /// Monokai theme pair.
    Monokai,
    /// Visual Studio Dark theme pair (dark only).
    VisualStudioDark,
}

impl ThemePair {
    /// Resolves the theme pair to a specific theme variant based on color mode.
    ///
    /// This is an internal method for resolving to the Theme enum.
    pub(crate) fn resolve(self, mode: ColorMode) -> Theme {
        match (self, mode) {
            (ThemePair::Base16Ocean, ColorMode::Dark) => Theme::Base16OceanDark,
            (ThemePair::Base16Ocean, ColorMode::Light) => Theme::Base16OceanLight,
            (ThemePair::Github, ColorMode::Dark) => Theme::GithubDark,
            (ThemePair::Github, ColorMode::Light) => Theme::GithubLight,
            (ThemePair::Gruvbox, ColorMode::Dark) => Theme::GruvboxDark,
            (ThemePair::Gruvbox, ColorMode::Light) => Theme::GruvboxLight,
            (ThemePair::OneHalf, ColorMode::Dark) => Theme::OneHalfDark,
            (ThemePair::OneHalf, ColorMode::Light) => Theme::OneHalfLight,
            (ThemePair::Solarized, ColorMode::Dark) => Theme::SolarizedDark,
            (ThemePair::Solarized, ColorMode::Light) => Theme::SolarizedLight,
            (ThemePair::Nord, _) => Theme::Nord,
            (ThemePair::Dracula, _) => Theme::Dracula,
            (ThemePair::Monokai, _) => Theme::MonokaiExtended,
            (ThemePair::VisualStudioDark, _) => Theme::VisualStudioDark,
        }
    }

    /// Returns a human-readable description of the theme for the given mode.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::markdown::highlighting::{ThemePair, ColorMode};
    ///
    /// let desc = ThemePair::Github.description(ColorMode::Dark);
    /// assert_eq!(desc, "GitHub's dark mode theme with blue accents");
    /// ```
    pub fn description(self, mode: ColorMode) -> &'static str {
        THEME_DESCRIPTIONS
            .get(&self.resolve(mode))
            .copied()
            .unwrap_or("Unknown theme")
    }

    /// Returns all available theme pairs.
    pub fn all() -> &'static [ThemePair] {
        &[
            ThemePair::Base16Ocean,
            ThemePair::Github,
            ThemePair::Gruvbox,
            ThemePair::OneHalf,
            ThemePair::Solarized,
            ThemePair::Nord,
            ThemePair::Dracula,
            ThemePair::Monokai,
            ThemePair::VisualStudioDark,
        ]
    }
}

/// Internal theme enum - individual theme variants.
///
/// This is the internal representation of themes. External code should
/// use `ThemePair` which provides light/dark pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Theme {
    Base16OceanDark,
    Base16OceanLight,
    GithubDark,
    GithubLight,
    GruvboxDark,
    GruvboxLight,
    OneHalfDark,
    OneHalfLight,
    SolarizedDark,
    SolarizedLight,
    Nord,
    Dracula,
    MonokaiExtended,
    VisualStudioDark,
}

impl Theme {
    /// Returns the embedded theme name for two-face loading.
    fn to_embedded_name(self) -> EmbeddedThemeName {
        match self {
            Theme::Base16OceanDark => EmbeddedThemeName::Base16OceanDark,
            Theme::Base16OceanLight => EmbeddedThemeName::Base16OceanLight,
            Theme::GithubDark => EmbeddedThemeName::Github,
            Theme::GithubLight => EmbeddedThemeName::InspiredGithub,
            Theme::GruvboxDark => EmbeddedThemeName::GruvboxDark,
            Theme::GruvboxLight => EmbeddedThemeName::GruvboxLight,
            Theme::OneHalfDark => EmbeddedThemeName::OneHalfDark,
            Theme::OneHalfLight => EmbeddedThemeName::OneHalfLight,
            Theme::SolarizedDark => EmbeddedThemeName::SolarizedDark,
            Theme::SolarizedLight => EmbeddedThemeName::SolarizedLight,
            Theme::Nord => EmbeddedThemeName::Nord,
            Theme::Dracula => EmbeddedThemeName::Dracula,
            Theme::MonokaiExtended => EmbeddedThemeName::MonokaiExtended,
            #[allow(deprecated)]
            Theme::VisualStudioDark => EmbeddedThemeName::VisualStudioDarkPlus,
        }
    }
}

lazy_static! {
    /// Static lookup table for theme descriptions.
    static ref THEME_DESCRIPTIONS: HashMap<Theme, &'static str> = {
        let mut map = HashMap::new();
        map.insert(
            Theme::Base16OceanDark,
            "Base16 Ocean dark - blue-green palette with excellent contrast",
        );
        map.insert(
            Theme::Base16OceanLight,
            "Base16 Ocean light - soft blue-green palette for light backgrounds",
        );
        map.insert(
            Theme::GithubDark,
            "GitHub's dark mode theme with blue accents",
        );
        map.insert(
            Theme::GithubLight,
            "GitHub's light mode theme - clean and minimal",
        );
        map.insert(
            Theme::GruvboxDark,
            "Gruvbox dark - retro groove warm color palette",
        );
        map.insert(
            Theme::GruvboxLight,
            "Gruvbox light - retro groove with cream backgrounds",
        );
        map.insert(
            Theme::OneHalfDark,
            "OneHalf dark - balanced palette inspired by Atom's One Dark",
        );
        map.insert(
            Theme::OneHalfLight,
            "OneHalf light - soft colors on light backgrounds",
        );
        map.insert(
            Theme::SolarizedDark,
            "Solarized dark - precision colors for machines and people",
        );
        map.insert(
            Theme::SolarizedLight,
            "Solarized light - precision colors on light backgrounds",
        );
        map.insert(
            Theme::Nord,
            "Nord - arctic, north-bluish color palette",
        );
        map.insert(
            Theme::Dracula,
            "Dracula - dark theme with vibrant purple and pink accents",
        );
        map.insert(
            Theme::MonokaiExtended,
            "Monokai Extended - classic editor theme with vibrant colors",
        );
        map.insert(
            Theme::VisualStudioDark,
            "Visual Studio Dark - Microsoft's professional dark theme",
        );
        map
    };

    /// Lazily loaded theme set from two-face.
    static ref THEME_SET: EmbeddedLazyThemeSet = extra_themes();
}

/// Loads a syntect theme for the given theme pair and color mode.
///
/// ## Panics
///
/// Panics if the theme cannot be loaded (should never happen with valid Theme variants).
pub(super) fn load_theme(theme_pair: ThemePair, color_mode: ColorMode) -> SyntectTheme {
    let theme = theme_pair.resolve(color_mode);
    let embedded_name = theme.to_embedded_name();

    THEME_SET
        .get(embedded_name)
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_mode_equality() {
        assert_eq!(ColorMode::Light, ColorMode::Light);
        assert_eq!(ColorMode::Dark, ColorMode::Dark);
        assert_ne!(ColorMode::Light, ColorMode::Dark);
    }

    #[test]
    fn test_theme_pair_resolve_github() {
        let dark = ThemePair::Github.resolve(ColorMode::Dark);
        let light = ThemePair::Github.resolve(ColorMode::Light);
        assert_eq!(dark, Theme::GithubDark);
        assert_eq!(light, Theme::GithubLight);
    }

    #[test]
    fn test_theme_pair_resolve_solarized() {
        let dark = ThemePair::Solarized.resolve(ColorMode::Dark);
        let light = ThemePair::Solarized.resolve(ColorMode::Light);
        assert_eq!(dark, Theme::SolarizedDark);
        assert_eq!(light, Theme::SolarizedLight);
    }

    #[test]
    fn test_theme_pair_resolve_nord() {
        // Nord is dark-only, so both modes resolve to the same theme
        let dark = ThemePair::Nord.resolve(ColorMode::Dark);
        let light = ThemePair::Nord.resolve(ColorMode::Light);
        assert_eq!(dark, Theme::Nord);
        assert_eq!(light, Theme::Nord);
    }

    #[test]
    fn test_theme_description() {
        let desc = ThemePair::Github.description(ColorMode::Dark);
        assert!(desc.contains("GitHub"));
        assert!(desc.contains("dark"));
    }

    #[test]
    fn test_theme_pair_all() {
        let all = ThemePair::all();
        assert!(all.len() >= 9);
        assert!(all.contains(&ThemePair::Github));
        assert!(all.contains(&ThemePair::Solarized));
    }

    #[test]
    fn test_load_theme_github_dark() {
        let theme = load_theme(ThemePair::Github, ColorMode::Dark);
        assert!(theme.settings.background.is_some());
    }

    #[test]
    fn test_load_theme_solarized_light() {
        let theme = load_theme(ThemePair::Solarized, ColorMode::Light);
        assert!(theme.settings.background.is_some());
    }

    #[test]
    fn test_all_themes_load() {
        // Verify all theme pairs can load in both modes
        for theme_pair in ThemePair::all() {
            let dark_theme = load_theme(*theme_pair, ColorMode::Dark);
            assert!(dark_theme.settings.background.is_some());

            let light_theme = load_theme(*theme_pair, ColorMode::Light);
            assert!(light_theme.settings.background.is_some());
        }
    }
}
