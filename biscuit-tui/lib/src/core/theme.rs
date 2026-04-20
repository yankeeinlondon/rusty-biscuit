//! Visual constants shared by every tui-chrome component.
//!
//! [`ComponentTheme`] centralises the glyphs and styles used across
//! the library so that components render consistently. Each component
//! state owns its own theme instance — callers can mutate it at
//! runtime or replace it wholesale.

use ratatui::style::{Color, Modifier, Style};

/// Centralised visual constants for every input component.
///
/// ## Notes
///
/// A [`Default`] impl is provided with sensible values that are
/// legible on both light and dark backgrounds; callers can override
/// individual fields or replace the theme entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentTheme {
    /// Glyph prefix that marks the currently focused row (e.g. the
    /// hovered option in `ChooseOne`).
    pub focus_indicator: String,

    /// Glyph that marks a selected option (e.g. the filled radio).
    pub selected_indicator: String,

    /// Glyph that marks an unselected option (e.g. the empty radio).
    pub unselected_indicator: String,

    /// Text rendered on the "on" side of a `BooleanSwitch` track.
    pub switch_on: String,

    /// Text rendered on the "off" side of a `BooleanSwitch` track.
    pub switch_off: String,

    /// The thumb glyph painted inside the active side of a switch.
    pub switch_thumb: char,

    /// Style applied to the character beneath the cursor in a
    /// `TextInput`.
    pub cursor_style: Style,

    /// Style applied to the selected option in `ChooseOne` /
    /// `ChooseMany`.
    pub selected_style: Style,

    /// Style applied to inline validation error text.
    pub error_style: Style,

    /// Style applied to rendered label text.
    pub label_style: Style,

    /// Style applied to disabled options.
    pub disabled_style: Style,

    /// Glyph rendered at the top of a list viewport when content is
    /// scrolled above the visible window.
    pub overflow_up_indicator: String,

    /// Glyph rendered at the bottom of a list viewport when content is
    /// scrolled below the visible window.
    pub overflow_down_indicator: String,
}

impl Default for ComponentTheme {
    fn default() -> Self {
        Self {
            focus_indicator: "▶".into(),
            selected_indicator: "●".into(),
            unselected_indicator: "○".into(),
            switch_on: " ON ".into(),
            switch_off: " OFF".into(),
            switch_thumb: '●',
            cursor_style: Style::default().add_modifier(Modifier::REVERSED),
            selected_style: Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            error_style: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            label_style: Style::default().add_modifier(Modifier::BOLD),
            disabled_style: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
            overflow_up_indicator: "▲".into(),
            overflow_down_indicator: "▼".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_populates_all_string_fields() {
        let theme = ComponentTheme::default();
        assert!(!theme.focus_indicator.is_empty());
        assert!(!theme.selected_indicator.is_empty());
        assert!(!theme.unselected_indicator.is_empty());
        assert!(!theme.switch_on.is_empty());
        assert!(!theme.switch_off.is_empty());
    }

    #[test]
    fn default_switch_thumb_is_a_visible_glyph() {
        let theme = ComponentTheme::default();
        assert!(!theme.switch_thumb.is_whitespace());
    }

    #[test]
    fn default_styles_are_distinct() {
        let theme = ComponentTheme::default();
        assert_ne!(theme.cursor_style, theme.selected_style);
        assert_ne!(theme.error_style, theme.label_style);
        assert_ne!(theme.selected_style, theme.disabled_style);
    }

    #[test]
    fn default_error_style_is_red_and_bold() {
        let theme = ComponentTheme::default();
        assert_eq!(theme.error_style.fg, Some(Color::Red));
        assert!(theme.error_style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn theme_is_clonable() {
        let theme = ComponentTheme::default();
        let cloned = theme.clone();
        assert_eq!(theme, cloned);
    }

    #[test]
    fn overflow_indicators_default_to_arrows() {
        let theme = ComponentTheme::default();
        assert_eq!(theme.overflow_up_indicator, "▲");
        assert_eq!(theme.overflow_down_indicator, "▼");
    }

    #[test]
    fn overflow_indicators_are_non_empty() {
        let theme = ComponentTheme::default();
        assert!(!theme.overflow_up_indicator.is_empty());
        assert!(!theme.overflow_down_indicator.is_empty());
    }
}
