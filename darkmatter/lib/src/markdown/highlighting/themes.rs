//! Theme enumeration, pairing, and loading infrastructure.
//!
//! Provides a curated set of theme pairs that adapt to light/dark modes,
//! with descriptions and utilities for loading syntect themes.

pub use biscuit_terminal::discovery::detection::ColorMode;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::convert::TryFrom;
use syntect::highlighting::Theme as SyntectTheme;
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName, extra as extra_themes};

/// Error type for invalid theme name parsing.
#[derive(Debug, Clone)]
pub struct InvalidThemeName(pub String);

impl std::error::Error for InvalidThemeName {}

impl std::fmt::Display for InvalidThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid theme name: '{}'. Valid names: github, one-half, base16-ocean, gruvbox, solarized, nord, dracula, monokai, vs-dark",
            self.0
        )
    }
}

/// How a code block's theme variant is chosen relative to the page color mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodeBlockMode {
    /// Opposite variant from the page (default): dark page -> light panel.
    #[default]
    Inverse,
    /// Always the dark variant.
    Dark,
    /// Always the light variant.
    Light,
    /// Same variant as the page.
    Same,
}

impl CodeBlockMode {
    /// Resolves the code block's color mode given the page's color mode.
    #[must_use]
    pub fn resolve(self, page_mode: ColorMode) -> ColorMode {
        match self {
            CodeBlockMode::Inverse => page_mode.inverted(),
            CodeBlockMode::Same => page_mode,
            CodeBlockMode::Dark => ColorMode::Dark,
            CodeBlockMode::Light => ColorMode::Light,
        }
    }
}

impl std::str::FromStr for CodeBlockMode {
    type Err = InvalidCodeBlockMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "inverse" => Ok(CodeBlockMode::Inverse),
            "dark" => Ok(CodeBlockMode::Dark),
            "light" => Ok(CodeBlockMode::Light),
            "same" => Ok(CodeBlockMode::Same),
            _ => Err(InvalidCodeBlockMode(s.to_string())),
        }
    }
}

impl std::fmt::Display for CodeBlockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            CodeBlockMode::Inverse => "inverse",
            CodeBlockMode::Dark => "dark",
            CodeBlockMode::Light => "light",
            CodeBlockMode::Same => "same",
        };
        f.write_str(name)
    }
}

/// Error type for invalid [`CodeBlockMode`] parsing.
#[derive(Debug, Clone)]
pub struct InvalidCodeBlockMode(pub String);

impl std::error::Error for InvalidCodeBlockMode {}

impl std::fmt::Display for InvalidCodeBlockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid code-block mode: '{}'. Valid values: inverse, dark, light, same",
            self.0
        )
    }
}

/// Primary API surface - theme pairs that adapt to light/dark modes.
///
/// Each variant represents a paired light/dark theme combination.
/// Use `resolve()` to get the appropriate theme for a color mode.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::{ThemePair, ColorMode};
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

impl TryFrom<&str> for ThemePair {
    type Error = InvalidThemeName;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "github" => Ok(ThemePair::Github),
            "base-16-ocean" | "base16-ocean" => Ok(ThemePair::Base16Ocean),
            "gruvbox" => Ok(ThemePair::Gruvbox),
            "one-half" | "onehalf" => Ok(ThemePair::OneHalf),
            "solarized" => Ok(ThemePair::Solarized),
            "nord" => Ok(ThemePair::Nord),
            "dracula" => Ok(ThemePair::Dracula),
            "monokai" => Ok(ThemePair::Monokai),
            "visual-studio-dark" | "vs-dark" => Ok(ThemePair::VisualStudioDark),
            _ => Err(InvalidThemeName(s.to_string())),
        }
    }
}

impl ThemePair {
    pub(crate) fn selected(self, theme: Option<ThemePair>) -> ThemePair {
        theme
            .or_else(|| {
                std::env::var("THEME")
                    .ok()
                    .and_then(|name| ThemePair::try_from(name.as_str()).ok())
            })
            .unwrap_or(self)
    }

    /// Resolves the theme pair to a specific theme variant based on color mode (light/dark).
    ///
    /// This is an internal method for resolving to the Theme enum. The public
    /// boundary resolver is
    /// [`resolve_for_surface`](super::resolve::ThemePair::resolve_for_surface)
    /// in [`super::resolve`], which feeds both the page surface and the code
    /// panel through the same source of truth.
    pub(crate) fn resolve(self, mode: ColorMode) -> Theme {
        match (self, mode) {
            (ThemePair::Base16Ocean, ColorMode::Dark | ColorMode::Unknown) => Theme::Base16OceanDark,
            (ThemePair::Base16Ocean, ColorMode::Light) => Theme::Base16OceanLight,
            (ThemePair::Github, ColorMode::Dark | ColorMode::Unknown) => Theme::GithubDark,
            (ThemePair::Github, ColorMode::Light) => Theme::GithubLight,
            (ThemePair::Gruvbox, ColorMode::Dark | ColorMode::Unknown) => Theme::GruvboxDark,
            (ThemePair::Gruvbox, ColorMode::Light) => Theme::GruvboxLight,
            (ThemePair::OneHalf, ColorMode::Dark | ColorMode::Unknown) => Theme::OneHalfDark,
            (ThemePair::OneHalf, ColorMode::Light) => Theme::OneHalfLight,
            (ThemePair::Solarized, ColorMode::Dark | ColorMode::Unknown) => Theme::SolarizedDark,
            (ThemePair::Solarized, ColorMode::Light) => Theme::SolarizedLight,
            (ThemePair::Nord, ColorMode::Dark | ColorMode::Unknown) => Theme::Nord,
            (ThemePair::Nord, ColorMode::Light) => Theme::OneHalfLight,
            (ThemePair::Dracula, ColorMode::Dark | ColorMode::Unknown) => Theme::Dracula,
            (ThemePair::Dracula, ColorMode::Light) => Theme::OneHalfLight,
            (ThemePair::Monokai, ColorMode::Dark | ColorMode::Unknown) => Theme::MonokaiExtended,
            (ThemePair::Monokai, ColorMode::Light) => Theme::OneHalfLight,
            (ThemePair::VisualStudioDark, ColorMode::Dark | ColorMode::Unknown) => Theme::VisualStudioDark,
            (ThemePair::VisualStudioDark, ColorMode::Light) => Theme::GithubLight,
        }
    }

    /// Parses a theme name from a string, returning the default theme (OneHalf) on failure.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::highlighting::ThemePair;
    ///
    /// assert_eq!(ThemePair::from_str_or_default("github"), ThemePair::Github);
    /// assert_eq!(ThemePair::from_str_or_default("unknown"), ThemePair::OneHalf);
    /// ```
    pub fn from_str_or_default(s: &str) -> Self {
        Self::try_from(s).unwrap_or(ThemePair::OneHalf)
    }

    /// Returns the kebab-case name of this theme pair.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::highlighting::ThemePair;
    ///
    /// assert_eq!(ThemePair::Github.kebab_name(), "github");
    /// assert_eq!(ThemePair::OneHalf.kebab_name(), "one-half");
    /// assert_eq!(ThemePair::Base16Ocean.kebab_name(), "base16-ocean");
    /// ```
    pub const fn kebab_name(self) -> &'static str {
        match self {
            ThemePair::Github => "github",
            ThemePair::Base16Ocean => "base16-ocean",
            ThemePair::Gruvbox => "gruvbox",
            ThemePair::OneHalf => "one-half",
            ThemePair::Solarized => "solarized",
            ThemePair::Nord => "nord",
            ThemePair::Dracula => "dracula",
            ThemePair::Monokai => "monokai",
            ThemePair::VisualStudioDark => "vs-dark",
        }
    }

    /// Returns a human-readable description of the theme for the given mode.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::highlighting::{ThemePair, ColorMode};
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
    ///
    /// Note: two-face's `EmbeddedThemeName::Github` and `InspiredGithub` are both
    /// light themes. For GithubDark we use ColdarkDark as a suitable dark alternative.
    fn to_embedded_name(self) -> EmbeddedThemeName {
        match self {
            Theme::Base16OceanDark => EmbeddedThemeName::Base16OceanDark,
            Theme::Base16OceanLight => EmbeddedThemeName::Base16OceanLight,
            // Note: two-face's Github themes are both light. ColdarkDark is a good dark substitute.
            Theme::GithubDark => EmbeddedThemeName::ColdarkDark,
            Theme::GithubLight => EmbeddedThemeName::Github,
            Theme::GruvboxDark => EmbeddedThemeName::GruvboxDark,
            Theme::GruvboxLight => EmbeddedThemeName::GruvboxLight,
            Theme::OneHalfDark => EmbeddedThemeName::OneHalfDark,
            Theme::OneHalfLight => EmbeddedThemeName::OneHalfLight,
            Theme::SolarizedDark => EmbeddedThemeName::SolarizedDark,
            Theme::SolarizedLight => EmbeddedThemeName::SolarizedLight,
            Theme::Nord => EmbeddedThemeName::Nord,
            Theme::Dracula => EmbeddedThemeName::Dracula,
            Theme::MonokaiExtended => EmbeddedThemeName::MonokaiExtended,
            // Note: VisualStudioDarkPlus was removed in two-face 0.5.0, using OneHalfDark as alternative
            Theme::VisualStudioDark => EmbeddedThemeName::OneHalfDark,
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

/// Returns the default code theme for a given prose theme.
///
/// Code blocks use the page theme pair, resolved against a color mode chosen by
/// the code renderer's [`CodeBlockMode`] (default `Inverse`). This function
/// therefore returns the prose theme unchanged; explicit code-theme overrides
/// are handled by [`detect_code_theme`].
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::{ThemePair, get_code_theme_for_prose};
///
/// assert_eq!(get_code_theme_for_prose(ThemePair::OneHalf), ThemePair::OneHalf);
/// assert_eq!(get_code_theme_for_prose(ThemePair::Base16Ocean), ThemePair::Base16Ocean);
/// ```
pub fn get_code_theme_for_prose(prose_theme: ThemePair) -> ThemePair {
    prose_theme
}

/// Detects the prose theme from environment variables.
///
/// Checks the `THEME` environment variable and parses it as a ThemePair.
/// Returns `ThemePair::OneHalf` if the variable is not set or contains an invalid value.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::detect_prose_theme;
/// // When THEME is not set, returns default
/// let theme = detect_prose_theme();
/// ```
pub fn detect_prose_theme() -> ThemePair {
    let theme = std::env::var("THEME")
        .ok()
        .map(|s| ThemePair::from_str_or_default(&s))
        .unwrap_or(ThemePair::OneHalf);

    tracing::info!(
        prose_theme = %theme.kebab_name(),
        env_var = std::env::var("THEME").ok().as_deref(),
        "Detected prose theme"
    );

    theme
}

/// Detects the code theme from environment variables or derives it from the prose theme.
///
/// First checks the `CODE_THEME` environment variable. If not set or invalid,
/// uses the given prose theme. Code blocks resolve that theme pair against a
/// color mode chosen by the code renderer's [`CodeBlockMode`] (default
/// `Inverse`), so by default the code block theme is the page theme on the
/// opposite surface.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::{detect_code_theme, ThemePair};
/// // Uses the page/prose theme pair by default
/// let code_theme = detect_code_theme(ThemePair::OneHalf);
/// ```
pub fn detect_code_theme(prose_theme: ThemePair) -> ThemePair {
    let env_theme = std::env::var("CODE_THEME").ok();
    let theme = env_theme
        .as_ref()
        .map(|s| ThemePair::from_str_or_default(s))
        .unwrap_or_else(|| get_code_theme_for_prose(prose_theme));

    tracing::info!(
        code_theme = %theme.kebab_name(),
        prose_theme = %prose_theme.kebab_name(),
        env_var = env_theme.as_deref(),
        derived = env_theme.is_none(),
        "Detected code theme"
    );

    theme
}

/// Detects the color mode from environment variables.
///
/// Detection priority:
/// 1. `NO_COLOR` - If set, returns Dark mode (respects no-color.org)
/// 2. `COLORFGBG` - Parses "fg;bg" format (bg < 7 is dark)
/// 3. Default: Dark mode (most common in terminal environments)
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::highlighting::detect_color_mode;
/// let mode = detect_color_mode();
/// ```
pub fn detect_color_mode() -> ColorMode {
    // Check NO_COLOR first (respect user preference per no-color.org)
    if std::env::var("NO_COLOR").is_ok() {
        tracing::info!(
            color_mode = "dark",
            source = "NO_COLOR",
            "Detected color mode from NO_COLOR env var"
        );
        return ColorMode::Dark;
    }

    // Check COLORFGBG (format: "fg;bg" where bg < 7 is dark)
    if let Ok(colorfgbg) = std::env::var("COLORFGBG")
        && let Some(bg) = colorfgbg.split(';').next_back()
        && let Ok(bg_num) = bg.parse::<u8>()
    {
        let mode = if bg_num < 7 {
            ColorMode::Dark
        } else {
            ColorMode::Light
        };
        tracing::info!(
            color_mode = %if mode == ColorMode::Dark { "dark" } else { "light" },
            source = "COLORFGBG",
            colorfgbg = %colorfgbg,
            bg_value = bg_num,
            "Detected color mode from COLORFGBG env var"
        );
        return mode;
    }

    // Default to dark mode (more common in terminal environments)
    tracing::info!(
        color_mode = "dark",
        source = "default",
        "Using default dark color mode (no env vars set)"
    );
    ColorMode::Dark
}

/// Loads a syntect theme for the given theme pair and color mode.
///
/// ## Panics
///
/// Panics if the theme cannot be loaded (should never happen with valid Theme variants).
pub(crate) fn load_theme(theme_pair: ThemePair, color_mode: ColorMode) -> &'static SyntectTheme {
    let resolved = theme_pair.resolve_for_surface(
        crate::markdown::highlighting::Surface::Mode(color_mode),
        Some(theme_pair),
        CodeBlockMode::Same,
    );
    load_resolved_theme(resolved.theme)
}

/// Borrows the embedded syntect theme for `theme` from the process-wide
/// [`struct@THEME_SET`].
///
/// The theme is returned by reference rather than cloned: the scope-selector
/// tables are large and a code block only ever reads them, so a per-block clone
/// was pure waste on the render hot path (finding 23).
pub(crate) fn load_resolved_theme(theme: Theme) -> &'static SyntectTheme {
    let embedded_name = theme.to_embedded_name();

    THEME_SET.get(embedded_name)
}

/// Resolves the theme's inline-code foreground and background for one color mode.
///
/// Reduces the [`ThemePair`] to a single syntect theme via `color_mode`, then
/// reads the `markup.raw.inline.markdown` scope style. When the theme declares
/// no distinct inline background, syntect resolves the theme's global
/// background; a transparent or pure-black result falls back to a subtle
/// mode-appropriate gray, matching the pre-render-tree inline-code contract.
///
/// ## Returns
///
/// `(foreground, background)` as opaque RGB triples.
pub(crate) fn inline_code_colors(
    theme_pair: ThemePair,
    color_mode: ColorMode,
) -> ((u8, u8, u8), (u8, u8, u8)) {
    use super::prose::ProseHighlighter;

    let syntect = load_theme(theme_pair, color_mode);
    let highlighter = ProseHighlighter::new(syntect);
    let style = highlighter.style_for_inline_code(&[highlighter.base_scope()]);
    let fg = style.foreground;
    let bg = style.background;

    let foreground = (fg.r, fg.g, fg.b);
    let background = if bg.a > 0 && (bg.r > 0 || bg.g > 0 || bg.b > 0) {
        (bg.r, bg.g, bg.b)
    } else {
        match color_mode {
            ColorMode::Light => (226, 229, 236),
            ColorMode::Dark | ColorMode::Unknown => (50, 50, 55),
        }
    };
    (foreground, background)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
    use biscuit_terminal::terminal::Terminal;
    use syntect::highlighting::Color;

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
        let dark = ThemePair::Nord.resolve(ColorMode::Dark);
        let light = ThemePair::Nord.resolve(ColorMode::Light);
        assert_eq!(dark, Theme::Nord);
        assert_eq!(light, Theme::OneHalfLight);
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

        // Dark themes should have dark backgrounds (r,g,b all < 100 typically)
        let bg = theme.settings.background.unwrap();
        assert!(
            bg.r < 100 && bg.g < 100 && bg.b < 100,
            "Expected dark background, got RGB({},{},{})",
            bg.r,
            bg.g,
            bg.b
        );
    }

    #[test]
    fn test_load_theme_solarized_light() {
        let theme = load_theme(ThemePair::Solarized, ColorMode::Light);
        assert!(theme.settings.background.is_some());
    }

    #[test]
    fn test_github_light_has_light_background() {
        let theme = load_theme(ThemePair::Github, ColorMode::Light);
        assert!(theme.settings.background.is_some());

        // Light themes should have light backgrounds (r,g,b all > 200 typically)
        let bg = theme.settings.background.unwrap();
        assert!(
            bg.r > 200 && bg.g > 200 && bg.b > 200,
            "Expected light background, got RGB({},{},{})",
            bg.r,
            bg.g,
            bg.b
        );
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

    #[test]
    fn all_theme_pair_code_block_backgrounds_are_lighter_in_dark_terminals() {
        for theme_pair in ThemePair::all() {
            let dark_terminal_code_theme = theme_pair
                .resolve_for_surface(
                    crate::markdown::highlighting::Surface::Mode(ColorMode::Dark),
                    Some(*theme_pair),
                    CodeBlockMode::default(),
                )
                .theme;
            let light_terminal_code_theme = theme_pair
                .resolve_for_surface(
                    crate::markdown::highlighting::Surface::Mode(ColorMode::Light),
                    Some(*theme_pair),
                    CodeBlockMode::default(),
                )
                .theme;
            let dark_terminal_bg = load_resolved_theme(dark_terminal_code_theme)
                .settings
                .background
                .expect("code block theme should define a background");
            let light_terminal_bg = load_resolved_theme(light_terminal_code_theme)
                .settings
                .background
                .expect("code block theme should define a background");

            assert_lighter(
                *theme_pair,
                "code block background in a dark terminal",
                dark_terminal_bg,
                light_terminal_bg,
            );
        }
    }

    #[test]
    fn test_try_from_valid_themes() {
        assert_eq!(ThemePair::try_from("github").unwrap(), ThemePair::Github);
        assert_eq!(ThemePair::try_from("one-half").unwrap(), ThemePair::OneHalf);
        assert_eq!(ThemePair::try_from("onehalf").unwrap(), ThemePair::OneHalf);
        assert_eq!(
            ThemePair::try_from("base16-ocean").unwrap(),
            ThemePair::Base16Ocean
        );
        assert_eq!(
            ThemePair::try_from("base-16-ocean").unwrap(),
            ThemePair::Base16Ocean
        );
        assert_eq!(
            ThemePair::try_from("vs-dark").unwrap(),
            ThemePair::VisualStudioDark
        );
        assert_eq!(
            ThemePair::try_from("visual-studio-dark").unwrap(),
            ThemePair::VisualStudioDark
        );
    }

    #[test]
    fn test_try_from_case_insensitive() {
        assert_eq!(ThemePair::try_from("GitHub").unwrap(), ThemePair::Github);
        assert_eq!(ThemePair::try_from("GITHUB").unwrap(), ThemePair::Github);
        assert_eq!(ThemePair::try_from("One-Half").unwrap(), ThemePair::OneHalf);
    }

    #[test]
    fn test_try_from_invalid_theme() {
        let result = ThemePair::try_from("unknown");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unknown"));
        assert!(err.to_string().contains("Valid names"));
    }

    #[test]
    fn test_from_str_or_default() {
        assert_eq!(ThemePair::from_str_or_default("github"), ThemePair::Github);
        assert_eq!(
            ThemePair::from_str_or_default("unknown"),
            ThemePair::OneHalf
        );
        assert_eq!(ThemePair::from_str_or_default(""), ThemePair::OneHalf);
    }

    #[test]
    fn test_kebab_name() {
        assert_eq!(ThemePair::Github.kebab_name(), "github");
        assert_eq!(ThemePair::OneHalf.kebab_name(), "one-half");
        assert_eq!(ThemePair::Base16Ocean.kebab_name(), "base16-ocean");
        assert_eq!(ThemePair::VisualStudioDark.kebab_name(), "vs-dark");
    }

    #[test]
    fn test_kebab_name_roundtrip() {
        for theme_pair in ThemePair::all() {
            let name = theme_pair.kebab_name();
            let parsed = ThemePair::try_from(name).unwrap();
            assert_eq!(*theme_pair, parsed, "Roundtrip failed for {:?}", theme_pair);
        }
    }

    #[test]
    fn test_code_theme_defaults_to_prose_theme_pair() {
        assert_eq!(
            get_code_theme_for_prose(ThemePair::OneHalf),
            ThemePair::OneHalf
        );
        assert_eq!(
            get_code_theme_for_prose(ThemePair::Github),
            ThemePair::Github
        );
        assert_eq!(
            get_code_theme_for_prose(ThemePair::Base16Ocean),
            ThemePair::Base16Ocean
        );
        assert_eq!(
            get_code_theme_for_prose(ThemePair::Monokai),
            ThemePair::Monokai
        );
    }

    #[test]
    fn test_invalid_theme_name_display() {
        let err = InvalidThemeName("badtheme".to_string());
        let msg = err.to_string();
        assert!(msg.contains("badtheme"));
        assert!(msg.contains("github"));
        assert!(msg.contains("one-half"));
    }

    // Environment variable detection tests - require serial_test to prevent race conditions
    use serial_test::serial;

    /// Helper for setting environment variables with automatic cleanup
    struct ScopedEnv {
        key: &'static str,
        original: Option<String>,
    }

    impl ScopedEnv {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }

        fn unset(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, original }
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            unsafe {
                if let Some(ref val) = self.original {
                    std::env::set_var(self.key, val);
                } else {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    #[test]
    #[serial]
    fn test_detect_prose_theme_with_env_var() {
        let _env = ScopedEnv::set("THEME", "github");
        assert_eq!(detect_prose_theme(), ThemePair::Github);
    }

    #[test]
    #[serial]
    fn test_detect_prose_theme_with_invalid_env_var() {
        let _env = ScopedEnv::set("THEME", "invalid-theme");
        assert_eq!(detect_prose_theme(), ThemePair::OneHalf);
    }

    #[test]
    #[serial]
    fn test_detect_prose_theme_without_env_var() {
        let _env = ScopedEnv::unset("THEME");
        assert_eq!(detect_prose_theme(), ThemePair::OneHalf);
    }

    #[test]
    #[serial]
    fn test_theme_pair_selected_uses_option_before_theme_env() {
        let _env = ScopedEnv::set("THEME", "github");
        assert_eq!(
            ThemePair::OneHalf.selected(Some(ThemePair::Gruvbox)),
            ThemePair::Gruvbox
        );
    }

    #[test]
    #[serial]
    fn test_theme_pair_selected_uses_case_insensitive_theme_env() {
        let _env = ScopedEnv::set("THEME", "gRuVbOx");
        assert_eq!(ThemePair::OneHalf.selected(None), ThemePair::Gruvbox);
    }

    #[test]
    #[serial]
    fn test_theme_pair_selected_ignores_invalid_theme_env() {
        let _env = ScopedEnv::set("THEME", "no-such-theme");
        assert_eq!(ThemePair::OneHalf.selected(None), ThemePair::OneHalf);
    }

    #[test]
    #[serial]
    fn test_resolve_for_surface_page_uses_terminal_mode_for_all_pairs() {
        let _env = ScopedEnv::unset("THEME");
        for theme_pair in ThemePair::all() {
            let light = terminal_with_color_mode(TerminalColorMode::Light);
            let dark = terminal_with_color_mode(TerminalColorMode::Dark);
            let unknown = terminal_with_color_mode(TerminalColorMode::Unknown);

            let page_light = theme_pair.resolve_for_surface(
                crate::markdown::highlighting::Surface::Terminal(&light),
                None,
                CodeBlockMode::Same,
            );
            assert_eq!(
                page_light.theme,
                theme_pair.resolve(ColorMode::Light),
                "page light resolution failed for {:?}",
                theme_pair
            );
            let page_dark = theme_pair.resolve_for_surface(
                crate::markdown::highlighting::Surface::Terminal(&dark),
                None,
                CodeBlockMode::Same,
            );
            assert_eq!(
                page_dark.theme,
                theme_pair.resolve(ColorMode::Dark),
                "page dark resolution failed for {:?}",
                theme_pair
            );
            let page_unknown = theme_pair.resolve_for_surface(
                crate::markdown::highlighting::Surface::Terminal(&unknown),
                None,
                CodeBlockMode::Same,
            );
            assert_eq!(
                page_unknown.theme,
                theme_pair.resolve(ColorMode::Dark),
                "page unknown-mode fallback failed for {:?}",
                theme_pair
            );
        }
    }

    #[test]
    #[serial]
    fn test_resolve_for_surface_code_block_inverts_terminal_mode_for_all_pairs() {
        let _env = ScopedEnv::unset("THEME");
        for theme_pair in ThemePair::all() {
            let light = terminal_with_color_mode(TerminalColorMode::Light);
            let dark = terminal_with_color_mode(TerminalColorMode::Dark);
            let unknown = terminal_with_color_mode(TerminalColorMode::Unknown);

            let code_light = theme_pair.resolve_for_surface(
                crate::markdown::highlighting::Surface::Terminal(&light),
                None,
                CodeBlockMode::default(),
            );
            assert_eq!(
                code_light.theme,
                theme_pair.resolve(ColorMode::Dark),
                "code light inversion failed for {:?}",
                theme_pair
            );
            let code_dark = theme_pair.resolve_for_surface(
                crate::markdown::highlighting::Surface::Terminal(&dark),
                None,
                CodeBlockMode::default(),
            );
            assert_eq!(
                code_dark.theme,
                theme_pair.resolve(ColorMode::Light),
                "code dark inversion failed for {:?}",
                theme_pair
            );
            let code_unknown = theme_pair.resolve_for_surface(
                crate::markdown::highlighting::Surface::Terminal(&unknown),
                None,
                CodeBlockMode::default(),
            );
            assert_eq!(
                code_unknown.theme,
                theme_pair.resolve(ColorMode::Light),
                "code unknown-mode inversion fallback failed for {:?}",
                theme_pair
            );
        }
    }

    #[test]
    #[serial]
    fn test_resolve_for_surface_uses_theme_env_when_option_missing() {
        let _env = ScopedEnv::set("THEME", "GITHUB");
        let dark = terminal_with_color_mode(TerminalColorMode::Dark);

        let page = ThemePair::OneHalf.resolve_for_surface(
            crate::markdown::highlighting::Surface::Terminal(&dark),
            None,
            CodeBlockMode::Same,
        );
        assert_eq!(page.theme, ThemePair::Github.resolve(ColorMode::Dark));
        let code = ThemePair::OneHalf.resolve_for_surface(
            crate::markdown::highlighting::Surface::Terminal(&dark),
            None,
            CodeBlockMode::default(),
        );
        assert_eq!(code.theme, ThemePair::Github.resolve(ColorMode::Light));
    }

    #[test]
    #[serial]
    fn test_resolve_for_surface_option_overrides_theme_env() {
        let _env = ScopedEnv::set("THEME", "github");
        let dark = terminal_with_color_mode(TerminalColorMode::Dark);

        let page = ThemePair::OneHalf.resolve_for_surface(
            crate::markdown::highlighting::Surface::Terminal(&dark),
            Some(ThemePair::Gruvbox),
            CodeBlockMode::Same,
        );
        assert_eq!(page.theme, ThemePair::Gruvbox.resolve(ColorMode::Dark));
        let code = ThemePair::OneHalf.resolve_for_surface(
            crate::markdown::highlighting::Surface::Terminal(&dark),
            Some(ThemePair::Gruvbox),
            CodeBlockMode::default(),
        );
        assert_eq!(code.theme, ThemePair::Gruvbox.resolve(ColorMode::Light));
    }

    #[test]
    #[serial]
    fn test_detect_code_theme_with_env_var() {
        let _env = ScopedEnv::set("CODE_THEME", "dracula");
        assert_eq!(detect_code_theme(ThemePair::Github), ThemePair::Dracula);
    }

    #[test]
    #[serial]
    fn test_detect_code_theme_without_env_var() {
        let _env = ScopedEnv::unset("CODE_THEME");
        assert_eq!(detect_code_theme(ThemePair::OneHalf), ThemePair::OneHalf);
    }

    #[test]
    #[serial]
    fn test_detect_color_mode_no_color() {
        let _env = ScopedEnv::set("NO_COLOR", "1");
        assert_eq!(detect_color_mode(), ColorMode::Dark);
    }

    #[test]
    #[serial]
    fn test_detect_color_mode_colorfgbg_dark() {
        let _no_color = ScopedEnv::unset("NO_COLOR");
        let _env = ScopedEnv::set("COLORFGBG", "15;0");
        assert_eq!(detect_color_mode(), ColorMode::Dark);
    }

    #[test]
    #[serial]
    fn test_detect_color_mode_colorfgbg_light() {
        let _no_color = ScopedEnv::unset("NO_COLOR");
        let _env = ScopedEnv::set("COLORFGBG", "0;15");
        assert_eq!(detect_color_mode(), ColorMode::Light);
    }

    #[test]
    #[serial]
    fn test_detect_color_mode_default() {
        let _no_color = ScopedEnv::unset("NO_COLOR");
        let _colorfgbg = ScopedEnv::unset("COLORFGBG");
        assert_eq!(detect_color_mode(), ColorMode::Dark);
    }

    fn terminal_with_color_mode(mode: TerminalColorMode) -> Terminal {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = mode;
        term
    }

    fn assert_lighter(theme_pair: ThemePair, label: &str, dark_mode: Color, light_mode: Color) {
        let dark_luminance = luminance(dark_mode);
        let light_luminance = luminance(light_mode);
        assert!(
            dark_luminance > light_luminance,
            "{theme_pair:?} {label} should be lighter in dark mode than light mode: dark={dark_mode:?} ({dark_luminance:.3}), light={light_mode:?} ({light_luminance:.3})"
        );
    }

    fn luminance(color: Color) -> f32 {
        fn channel(value: u8) -> f32 {
            let value = f32::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }

    #[test]
    fn code_block_mode_resolve_against_dark_page() {
        assert_eq!(CodeBlockMode::Inverse.resolve(ColorMode::Dark), ColorMode::Light);
        assert_eq!(CodeBlockMode::Same.resolve(ColorMode::Dark), ColorMode::Dark);
        assert_eq!(CodeBlockMode::Dark.resolve(ColorMode::Dark), ColorMode::Dark);
        assert_eq!(CodeBlockMode::Light.resolve(ColorMode::Dark), ColorMode::Light);
    }

    #[test]
    fn code_block_mode_resolve_against_light_page() {
        assert_eq!(CodeBlockMode::Inverse.resolve(ColorMode::Light), ColorMode::Dark);
        assert_eq!(CodeBlockMode::Same.resolve(ColorMode::Light), ColorMode::Light);
        assert_eq!(CodeBlockMode::Dark.resolve(ColorMode::Light), ColorMode::Dark);
        assert_eq!(CodeBlockMode::Light.resolve(ColorMode::Light), ColorMode::Light);
    }

    #[test]
    fn code_block_mode_default_is_inverse() {
        assert_eq!(CodeBlockMode::default(), CodeBlockMode::Inverse);
    }

    #[test]
    fn code_block_mode_from_str_valid() {
        use std::str::FromStr;
        assert_eq!(CodeBlockMode::from_str("inverse").unwrap(), CodeBlockMode::Inverse);
        assert_eq!(CodeBlockMode::from_str("DARK").unwrap(), CodeBlockMode::Dark);
        assert_eq!(CodeBlockMode::from_str("Light").unwrap(), CodeBlockMode::Light);
        assert_eq!(CodeBlockMode::from_str(" same ").unwrap(), CodeBlockMode::Same);
    }

    #[test]
    fn code_block_mode_from_str_invalid() {
        use std::str::FromStr;
        assert!(CodeBlockMode::from_str("sideways").is_err());
    }

    #[test]
    fn code_block_mode_display_roundtrips() {
        use std::str::FromStr;
        for mode in [
            CodeBlockMode::Inverse,
            CodeBlockMode::Dark,
            CodeBlockMode::Light,
            CodeBlockMode::Same,
        ] {
            assert_eq!(CodeBlockMode::from_str(&mode.to_string()).unwrap(), mode);
        }
    }
}
