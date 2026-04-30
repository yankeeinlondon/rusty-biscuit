//! Conservative terminal capability detection for choice rendering.
//!
//! Provides [`TerminalStyle`] — a small struct that reports the
//! terminal's likely background colour and whether a Nerd Font is
//! available. Both detections are best-effort heuristics; when
//! uncertain the library falls back to dark-background-safe standard
//! Unicode glyphs.

use std::env;

use ratatui::style::{Color, Modifier, Style};

use crate::components::ActiveChoiceColor;

#[cfg(test)]
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Detected background luminance of the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TerminalBackground {
    /// Dark background (black or near-black).
    Dark,
    /// Light background (white or near-white).
    Light,
    /// Could not be determined — callers should pick conservative
    /// dark-background-safe colours.
    #[default]
    Unknown,
}

/// Conservative Nerd Font detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NerdFontStatus {
    /// A Nerd Font is likely installed (e.g. `NERD_FONT` env var set).
    Likely,
    /// No evidence of a Nerd Font — use standard Unicode fallbacks.
    #[default]
    Unknown,
}

/// Snapshot of terminal capabilities relevant to choice rendering.
///
/// Used by the shared choice renderer to pick indicator glyphs and
/// active-row background colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TerminalStyle {
    /// Detected terminal background colour.
    pub background: TerminalBackground,
    /// Whether a Nerd Font is likely available.
    pub nerd_font: NerdFontStatus,
}

impl TerminalStyle {
    /// Returns a [`TerminalStyle`] inferred from the process
    /// environment.
    ///
    /// ## Detection heuristics
    ///
    /// - **Background** — reads `COLORFGBG` (common on xterm-compatible
    ///   terminals). A trailing `0` or `8` is treated as dark;
    ///   `7` or `15` as light. Everything else is [`Unknown`].
    /// - **Nerd Font** — checks for the presence of a `NERD_FONT`
    ///   environment variable (any non-empty value counts as likely).
    ///   Also recognises `TERM_PROGRAM` values that bundle Nerd Font
    ///   glyphs (e.g. `WezTerm`, `iTerm.app`).
    ///
    /// Callers that already know the terminal profile (e.g. a TUI
    /// framework that queried the OS) can construct [`TerminalStyle`]
    /// directly and skip this function.
    pub fn from_env() -> Self {
        #[cfg(test)]
        let _env_lock = ENV_LOCK.lock().expect("terminal style env lock poisoned");
        let background = detect_background();
        let nerd_font = detect_nerd_font();
        Self {
            background,
            nerd_font,
        }
    }
}

fn detect_background() -> TerminalBackground {
    if let Ok(fgbg) = env::var("COLORFGBG") {
        // xterm-style "foreground;background" string.
        if let Some(bg) = fgbg.split(';').next_back() {
            match bg.trim() {
                "0" | "8" => return TerminalBackground::Dark,
                "7" | "15" => return TerminalBackground::Light,
                _ => {}
            }
        }
    }
    TerminalBackground::Unknown
}

/// Resolves the active-row [`Style`] for a [`ChooseOne`] /
/// [`ChooseMany`] hovered item.
///
/// The choice renderer applies the returned style to the prefix +
/// label + one trailing blank cell of the active row only — never to
/// the rest of the row width — so the highlight visibly tracks the
/// item content rather than the entire viewport.
///
/// ## Returns
///
/// A [`Style`] with:
///
/// - a faint background colour drawn from a small per-`ActiveChoiceColor`
///   palette tuned for the detected [`TerminalBackground`];
/// - a foreground colour that meets the spec's contrast rule (white on
///   `Dark` and `Unknown`, black on `Light`);
/// - the bold modifier set; underline left off.
///
/// ## Notes
///
/// The palette uses 256-colour indices to maximise portability across
/// terminals that do not advertise truecolour:
///
/// | `ActiveChoiceColor` | `Dark` / `Unknown` bg | `Light` bg |
/// | --- | --- | --- |
/// | `Grey` | 238 (faint dark grey) | 252 (faint light grey) |
/// | `Green` | 22 (deep green) | 194 (pale mint) |
/// | `Yellow` | 58 (deep ochre) | 230 (pale cream) |
/// | `Red` | 52 (deep maroon) | 224 (pale pink) |
///
/// Foregrounds are [`Color::White`] on `Dark`/`Unknown` and
/// [`Color::Black`] on `Light`. The `Unknown` branch defers to the
/// dark-safe palette so that mistakenly-light terminals still produce
/// legible (if slightly low-contrast) output.
///
/// [`ChooseOne`]: crate::components::ChooseOne
/// [`ChooseMany`]: crate::components::ChooseMany
pub fn resolve_active_style(color: ActiveChoiceColor, background: TerminalBackground) -> Style {
    let (bg_index, fg) = match background {
        TerminalBackground::Light => {
            let idx = match color {
                ActiveChoiceColor::Grey => 252,
                ActiveChoiceColor::Green => 194,
                ActiveChoiceColor::Yellow => 230,
                ActiveChoiceColor::Red => 224,
            };
            (idx, Color::Black)
        }
        TerminalBackground::Dark | TerminalBackground::Unknown => {
            let idx = match color {
                ActiveChoiceColor::Grey => 238,
                ActiveChoiceColor::Green => 22,
                ActiveChoiceColor::Yellow => 58,
                ActiveChoiceColor::Red => 52,
            };
            (idx, Color::White)
        }
    };
    Style::default()
        .bg(Color::Indexed(bg_index))
        .fg(fg)
        .add_modifier(Modifier::BOLD)
}

fn detect_nerd_font() -> NerdFontStatus {
    if env::var("NERD_FONT").is_ok_and(|v| !v.is_empty()) {
        return NerdFontStatus::Likely;
    }
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "WezTerm" | "iTerm.app" | "Ghostty" => return NerdFontStatus::Likely,
            _ => {}
        }
    }
    NerdFontStatus::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::MutexGuard;

    fn env_lock() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().expect("terminal style env lock poisoned")
    }

    #[test]
    fn default_terminal_style_is_unknown() {
        let style = TerminalStyle::default();
        assert_eq!(style.background, TerminalBackground::Unknown);
        assert_eq!(style.nerd_font, NerdFontStatus::Unknown);
    }

    #[test]
    fn detect_background_dark_from_colorfgbg() {
        let _lock = env_lock();
        let _guard = env_guard("COLORFGBG", "15;0");
        assert_eq!(detect_background(), TerminalBackground::Dark);
    }

    #[test]
    fn detect_background_light_from_colorfgbg() {
        let _lock = env_lock();
        let _guard = env_guard("COLORFGBG", "0;15");
        assert_eq!(detect_background(), TerminalBackground::Light);
    }

    #[test]
    fn detect_background_unknown_when_empty() {
        let _lock = env_lock();
        let _guard = env_guard("COLORFGBG", "");
        assert_eq!(detect_background(), TerminalBackground::Unknown);
    }

    #[test]
    fn detect_nerd_font_from_env_var() {
        let _lock = env_lock();
        let _guard = env_guard("NERD_FONT", "1");
        assert_eq!(detect_nerd_font(), NerdFontStatus::Likely);
    }

    #[test]
    fn resolve_active_style_dark_grey_default() {
        let style = resolve_active_style(ActiveChoiceColor::Grey, TerminalBackground::Dark);
        assert_eq!(style.bg, Some(Color::Indexed(238)));
        assert_eq!(style.fg, Some(Color::White));
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(!style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn resolve_active_style_light_grey() {
        let style = resolve_active_style(ActiveChoiceColor::Grey, TerminalBackground::Light);
        assert_eq!(style.bg, Some(Color::Indexed(252)));
        assert_eq!(style.fg, Some(Color::Black));
    }

    #[test]
    fn resolve_active_style_unknown_uses_dark_palette() {
        let dark = resolve_active_style(ActiveChoiceColor::Grey, TerminalBackground::Dark);
        let unknown = resolve_active_style(ActiveChoiceColor::Grey, TerminalBackground::Unknown);
        assert_eq!(dark, unknown);
    }

    #[test]
    fn resolve_active_style_green_dark_uses_deep_green_index() {
        let style = resolve_active_style(ActiveChoiceColor::Green, TerminalBackground::Dark);
        assert_eq!(style.bg, Some(Color::Indexed(22)));
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn resolve_active_style_yellow_dark_uses_deep_ochre_index() {
        let style = resolve_active_style(ActiveChoiceColor::Yellow, TerminalBackground::Dark);
        assert_eq!(style.bg, Some(Color::Indexed(58)));
    }

    #[test]
    fn resolve_active_style_red_light_uses_pale_pink_index() {
        let style = resolve_active_style(ActiveChoiceColor::Red, TerminalBackground::Light);
        assert_eq!(style.bg, Some(Color::Indexed(224)));
        assert_eq!(style.fg, Some(Color::Black));
    }

    #[test]
    fn detect_nerd_font_from_term_program_wezterm() {
        let _lock = env_lock();
        let _guard1 = env_guard("NERD_FONT", "");
        let _guard2 = env_guard("TERM_PROGRAM", "WezTerm");
        assert_eq!(detect_nerd_font(), NerdFontStatus::Likely);
    }

    // Simple RAII guard that sets an env var for the duration of a test
    // and restores the previous value (if any) on drop.
    struct EnvGuard {
        key: String,
        old: Option<String>,
    }

    fn env_guard(key: &str, value: &str) -> EnvGuard {
        let old = env::var(key).ok();
        unsafe {
            if value.is_empty() {
                env::remove_var(key);
            } else {
                env::set_var(key, value);
            }
        }
        EnvGuard {
            key: key.to_string(),
            old,
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.old {
                    Some(v) => env::set_var(&self.key, v),
                    None => env::remove_var(&self.key),
                }
            }
        }
    }
}
