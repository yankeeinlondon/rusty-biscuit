//! Boundary resolver for theme + color mode selection.
//!
//! Phase 2 collapses the duplicated `ThemePair -> Theme` resolution paths
//! (one per [`CodeHighlighter::for_*`](super::CodeHighlighter) wrapper) into
//! a single resolver keyed off the render surface. The same source of truth
//! now feeds the page surface, the prose theme, and the nested code-block
//! panel — fixing the dual color-mode defect the public
//! [`CodeBlock`](crate::markdown::code_block::CodeBlock) and
//! [`DarkmatterPage`](crate::layout::DarkmatterPage) components used to inherit
//! (a `Terminal`-derived page mode and an env-derived code-panel mode could
//! disagree when one detection leg resolved to `Light` and the other to
//! `Dark`).
//!
//! Production code (the tree entry points, [`CodeBlock`], and
//! [`DarkmatterPage`](crate::layout::DarkmatterPage)) routes through
//! [`ThemePair::resolve_for_surface`]. Tests and one-off helpers keep using
//! [`CodeHighlighter::new`](super::CodeHighlighter::new).

use biscuit_terminal::terminal::Terminal;

use super::themes::{CodeBlockMode, ColorMode, Theme, ThemePair};

/// Render surface for theme resolution.
///
/// The terminal is the source of truth when available; a `ColorMode` fallback
/// keeps the resolver usable for callers that have not constructed a
/// [`Terminal`] (e.g. env-driven detection in tests, or the YAML-line helper
/// that runs without a terminal at all).
pub(crate) enum Surface<'a> {
    Terminal(&'a Terminal),
    Mode(ColorMode),
}

impl Surface<'_> {
    /// The page color mode for this surface, with `Unknown` resolved to `Dark`
    /// so the resolver never returns an inverted `Unknown` panel.
    pub(crate) fn page_mode(&self) -> ColorMode {
        match self {
            Surface::Terminal(term) => term.color_mode().resolve_unknown(),
            Surface::Mode(mode) => mode.resolve_unknown(),
        }
    }
}

/// Resolved theme + color mode for a render surface.
///
/// Returned by [`ThemePair::resolve_for_surface`]. The `color_mode` field is
/// the code-panel mode — for `CodeBlockMode::Inverse` (default) it is the
/// page mode's inversion, for `CodeBlockMode::Same` it equals the page mode,
/// etc. Callers building a [`CodeHighlighter`](super::CodeHighlighter) pass
/// both fields to [`CodeHighlighter::from_theme`](super::CodeHighlighter::from_theme).
pub(crate) struct ResolvedTheme {
    /// The selected theme pair (after override / `THEME` env fallback).
    /// Held for callers that want to log or compare the resolved pair
    /// against the requested one.
    #[allow(dead_code)]
    pub theme_pair: ThemePair,
    /// The specific `Theme` variant to load — the resolved pair, narrowed
    /// to the panel color mode.
    pub theme: Theme,
    /// The code-panel color mode (the inversion of the page mode under
    /// the default `CodeBlockMode::Inverse`).
    pub color_mode: ColorMode,
}

impl ThemePair {
    /// Resolves this [`ThemePair`] for a render surface and a
    /// [`CodeBlockMode`].
    ///
    /// The same boundary resolver feeds the page surface, the prose theme,
    /// and the nested code-block panel:
    ///
    /// * `surface` supplies the page color mode (the source of truth —
    ///   either a `Terminal` or a fallback `ColorMode`).
    /// * `mode_override` is an explicit `ThemePair` override; when `None`,
    ///   the `THEME` environment variable is honored (see
    ///   [`Self::selected`]).
    /// * `code_block_mode` chooses the variant the code panel adopts relative
    ///   to the page mode — `Inverse` (default) for the contrasting panel,
    ///   `Same` for the matching one, `Dark` / `Light` to pin.
    ///
    /// The single boundary keeps the page surface and the code panel
    /// resolving against the same source of truth — no more "panel does not
    /// invert in dark mode" defect from an env-only detector disagreeing with
    /// a real terminal.
    pub(crate) fn resolve_for_surface(
        self,
        surface: Surface<'_>,
        mode_override: Option<ThemePair>,
        code_block_mode: CodeBlockMode,
    ) -> ResolvedTheme {
        let page_mode = surface.page_mode();
        let panel_mode = code_block_mode.resolve(page_mode);
        let theme_pair = self.selected(mode_override);
        let theme = theme_pair.resolve(panel_mode);
        ResolvedTheme {
            theme_pair,
            theme,
            color_mode: panel_mode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::ColorMode as TerminalColorMode;
    use serial_test::serial;

    fn terminal_with_color_mode(mode: TerminalColorMode) -> Terminal {
        let mut term = Terminal::new_optimistic(80);
        term.color_mode = mode;
        term
    }

    #[test]
    #[serial]
    fn surface_terminal_reads_color_mode() {
        let dark = terminal_with_color_mode(TerminalColorMode::Dark);
        assert_eq!(Surface::Terminal(&dark).page_mode(), ColorMode::Dark);

        let light = terminal_with_color_mode(TerminalColorMode::Light);
        assert_eq!(Surface::Terminal(&light).page_mode(), ColorMode::Light);
    }

    #[test]
    fn surface_terminal_resolves_unknown_to_dark() {
        let unknown = terminal_with_color_mode(TerminalColorMode::Unknown);
        assert_eq!(Surface::Terminal(&unknown).page_mode(), ColorMode::Dark);
    }

    #[test]
    fn surface_mode_resolves_unknown_to_dark() {
        assert_eq!(Surface::Mode(ColorMode::Unknown).page_mode(), ColorMode::Dark);
        assert_eq!(Surface::Mode(ColorMode::Light).page_mode(), ColorMode::Light);
        assert_eq!(Surface::Mode(ColorMode::Dark).page_mode(), ColorMode::Dark);
    }

    /// Dark terminal, default `Inverse` code-block mode: panel must be `Light`
    /// (the dark-vs-light contrast default). This is the cross-surface
    /// invariant: the *same* `Terminal::color_mode()` feeds both surfaces, so
    /// a dark page always inverts to a light panel and vice versa.
    #[test]
    fn resolve_for_surface_inverts_default_dark_terminal_to_light_panel() {
        let dark = terminal_with_color_mode(TerminalColorMode::Dark);
        let resolved = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&dark),
            None,
            CodeBlockMode::default(),
        );
        assert_eq!(resolved.color_mode, ColorMode::Light);
        // OneHalf light variant is `OneHalfLight`.
        assert!(matches!(
            resolved.theme,
            super::super::themes::Theme::OneHalfLight
        ));
    }

    /// Light terminal, default `Inverse` code-block mode: panel must be
    /// `Dark`.
    #[test]
    fn resolve_for_surface_inverts_default_light_terminal_to_dark_panel() {
        let light = terminal_with_color_mode(TerminalColorMode::Light);
        let resolved = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&light),
            None,
            CodeBlockMode::default(),
        );
        assert_eq!(resolved.color_mode, ColorMode::Dark);
    }

    /// `CodeBlockMode::Same` returns the page mode unchanged — the code
    /// panel matches the page surface. This is the cross-surface guardrail:
    /// even with `Same`, the same terminal's `color_mode` reaches both.
    #[test]
    fn resolve_for_surface_same_mode_returns_page_mode() {
        let dark = terminal_with_color_mode(TerminalColorMode::Dark);
        let resolved = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&dark),
            None,
            CodeBlockMode::Same,
        );
        assert_eq!(resolved.color_mode, ColorMode::Dark);

        let light = terminal_with_color_mode(TerminalColorMode::Light);
        let resolved = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&light),
            None,
            CodeBlockMode::Same,
        );
        assert_eq!(resolved.color_mode, ColorMode::Light);
    }

    /// `Unknown` terminal mode falls back to `Dark` (page-mode default) and
    /// the panel inverts to `Light`. The terminal is still the source of
    /// truth — the fallback is *not* an env-derived mode, just the standard
    /// `Unknown → Dark` mapping.
    #[test]
    fn resolve_for_surface_unknown_terminal_resolves_to_dark_page_light_panel() {
        let unknown = terminal_with_color_mode(TerminalColorMode::Unknown);
        let resolved = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&unknown),
            None,
            CodeBlockMode::default(),
        );
        assert_eq!(resolved.color_mode, ColorMode::Light);
    }

    /// The `mode_override` parameter wins over the default theme pair
    /// (matches the deleted `for_code_block(term, Some(theme))` contract).
    #[test]
    fn resolve_for_surface_override_wins_over_default() {
        let dark = terminal_with_color_mode(TerminalColorMode::Dark);
        let resolved = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&dark),
            Some(ThemePair::Gruvbox),
            CodeBlockMode::default(),
        );
        assert_eq!(resolved.theme_pair, ThemePair::Gruvbox);
        assert!(matches!(
            resolved.theme,
            super::super::themes::Theme::GruvboxLight
        ));
    }

    /// Fallback `Surface::Mode` produces the same resolution as a terminal
    /// whose `color_mode` matches.
    #[test]
    fn resolve_for_surface_mode_fallback_matches_terminal() {
        let dark = terminal_with_color_mode(TerminalColorMode::Dark);
        let from_term = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&dark),
            None,
            CodeBlockMode::default(),
        );
        let from_mode = ThemePair::OneHalf.resolve_for_surface(
            Surface::Mode(ColorMode::Dark),
            None,
            CodeBlockMode::default(),
        );
        assert_eq!(from_term.color_mode, from_mode.color_mode);
        assert!(matches!(from_term.theme, super::super::themes::Theme::OneHalfLight));
        assert!(matches!(from_mode.theme, super::super::themes::Theme::OneHalfLight));
    }

    /// `CodeBlockMode::Dark` pins the panel to `Dark` regardless of the
    /// page surface — keeps the cross-surface invariant explicit (a light
    /// page with `Dark` panels still agrees on the *terminal* source of
    /// truth; the override is the only way for the panel to *diverge* from
    /// the page).
    #[test]
    fn resolve_for_surface_pinned_dark_ignores_page_mode() {
        let light = terminal_with_color_mode(TerminalColorMode::Light);
        let resolved = ThemePair::OneHalf.resolve_for_surface(
            Surface::Terminal(&light),
            None,
            CodeBlockMode::Dark,
        );
        assert_eq!(resolved.color_mode, ColorMode::Dark);
    }
}
