//! Shared runtime state for choice components.
//!
//! `ChooseOne` and `ChooseMany` keep different selection payloads, but
//! their input configuration, navigation, filtering, hotkey display,
//! layout cache, theme, and validation bookkeeping are identical. This
//! module owns that shared state so the two components do not depend on
//! each other's internals.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::core::{ComponentTheme, FuzzyFilter, KeyBindings, Label, TerminalStyle};

use super::choice_layout::ChoiceLayout;
use super::choose::{ChoiceInput, ChoiceOption, HotkeyDisplayMode, HotkeySpec};

/// Duration that hotkey badges remain visible after a Ctrl/Alt chord on
/// terminals that do not emit modifier-only key events.
///
/// Crossterm's modifier-only key events (`KeyEventKind::Press` with a
/// `KeyCode::Modifier`) are not portable: most terminals only deliver
/// the modifier alongside a chord (`Ctrl+R`), never on its own. To make
/// badges discoverable in those environments, every Ctrl/Alt chord
/// arms a deadline. While `Instant::now() < deadline` the badges
/// remain visible; once it expires they hide again.
pub(crate) const HOTKEY_DISPLAY_FALLBACK: Duration = Duration::from_millis(300);

/// Shared transient UI state for choice widgets.
#[derive(Debug, Clone)]
pub struct ChoiceCommonState<V = String> {
    pub(super) input: ChoiceInput<V>,
    pub(super) hover: usize,
    pub(super) scroll_offset: usize,
    pub(super) ctrl_hotkeys: HashMap<char, usize>,
    pub(super) alt_hotkeys: HashMap<char, usize>,
    pub(super) label: Option<Label>,
    pub(super) theme: ComponentTheme,
    pub(super) bindings: KeyBindings,
    pub(super) validation_error: Option<String>,
    pub(super) filter: FuzzyFilter,
    pub(super) filter_visible: bool,
    pub(super) cached_labels: Vec<String>,
    pub(super) layout_cache: ChoiceLayout,
    pub(super) hotkey_display: HotkeyDisplayMode,
    pub(super) hotkey_display_deadline: Option<Instant>,
    pub(super) hotkey_display_override: Option<HotkeyDisplayMode>,
    /// Sticky badge-visibility mode toggled by `Ctrl+Space` /
    /// `Alt+Space`.
    ///
    /// When `Some`, takes precedence over the dynamic deadline path so
    /// users on terminals that do not emit bare modifier press/release
    /// events still have a portable way to surface the badges. `None`
    /// means the dynamic logic governs visibility.
    pub(super) hotkey_display_sticky: Option<HotkeyDisplayMode>,
    pub(super) terminal_style: TerminalStyle,
}

impl<V> ChoiceCommonState<V> {
    /// Builds shared state after the caller has forced selection mode
    /// and applied option ordering.
    pub(super) fn new(input: ChoiceInput<V>) -> Self {
        let (ctrl_hotkeys, alt_hotkeys) = build_effective_hotkeys(&input.options);
        let hover = first_enabled_index(&input.options).unwrap_or(0);
        let cached_labels: Vec<String> = input.options.iter().map(|o| o.label.clone()).collect();
        let mut filter = FuzzyFilter::new();
        filter.clear(&cached_labels);
        Self {
            input,
            hover,
            scroll_offset: 0,
            ctrl_hotkeys,
            alt_hotkeys,
            label: None,
            theme: ComponentTheme::default(),
            bindings: KeyBindings::default(),
            validation_error: None,
            filter,
            filter_visible: false,
            cached_labels,
            layout_cache: ChoiceLayout::default(),
            hotkey_display: HotkeyDisplayMode::Hidden,
            hotkey_display_deadline: None,
            hotkey_display_override: None,
            hotkey_display_sticky: None,
            terminal_style: TerminalStyle::from_env(),
        }
    }

    pub(super) fn with_label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    pub(super) fn with_theme(mut self, theme: ComponentTheme) -> Self {
        self.theme = theme;
        self
    }

    pub(super) fn with_key_bindings(mut self, bindings: KeyBindings) -> Self {
        self.bindings = bindings;
        self
    }

    pub(super) fn with_active_color(mut self, color: super::choose::ActiveChoiceColor) -> Self {
        self.input.active_color = color;
        self
    }

    pub(super) fn with_hotkey_display(mut self, mode: HotkeyDisplayMode) -> Self {
        self.hotkey_display_override = Some(mode);
        self.hotkey_display = mode;
        self.hotkey_display_deadline = None;
        self
    }

    pub(super) fn with_terminal_style(mut self, terminal_style: TerminalStyle) -> Self {
        self.terminal_style = terminal_style;
        self
    }

    /// Resolves the effective [`HotkeyDisplayMode`] at `now` against
    /// the fallback deadline.
    pub(super) fn current_hotkey_display(&self, now: Instant) -> HotkeyDisplayMode {
        if let Some(forced) = self.hotkey_display_override {
            return forced;
        }
        if let Some(sticky) = self.hotkey_display_sticky {
            return sticky;
        }
        match self.hotkey_display_deadline {
            Some(deadline) if now < deadline => self.hotkey_display,
            Some(_) => HotkeyDisplayMode::Hidden,
            None => self.hotkey_display,
        }
    }
}

/// Implements the common builder methods shared by choice states.
macro_rules! impl_choice_common_builders {
    ($state:ident) => {
        /// Attaches a label rendered at the configured position.
        pub fn with_label(mut self, label: Label) -> Self {
            self.common = self.common.with_label(label);
            self
        }

        /// Replaces the active theme.
        pub fn with_theme(mut self, theme: ComponentTheme) -> Self {
            self.common = self.common.with_theme(theme);
            self
        }

        /// Replaces the key bindings.
        pub fn with_key_bindings(mut self, bindings: KeyBindings) -> Self {
            self.common = self.common.with_key_bindings(bindings);
            self
        }

        /// Sets the [`ActiveChoiceColor`] used for the actively hovered
        /// option. Sugar over mutating the underlying [`ChoiceInput`].
        pub fn with_active_color(mut self, color: super::choose::ActiveChoiceColor) -> Self {
            self.common = self.common.with_active_color(color);
            self
        }

        /// Forces the hotkey badge display mode for the lifetime of this
        /// state.
        pub fn with_hotkey_display(mut self, mode: HotkeyDisplayMode) -> Self {
            self.common = self.common.with_hotkey_display(mode);
            self
        }

        /// Replaces the detected terminal style used for rendering choice
        /// indicators and active-row contrast.
        pub fn with_terminal_style(mut self, terminal_style: TerminalStyle) -> Self {
            self.common = self.common.with_terminal_style(terminal_style);
            self
        }
    };
}

pub(super) use impl_choice_common_builders;

pub(super) fn first_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> {
    options.iter().position(|option| !option.disabled)
}

pub(super) fn last_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> {
    options.iter().rposition(|option| !option.disabled)
}

/// Detects whether `event` is a "modifier-only" key press for `Ctrl`
/// or `Alt`.
pub(super) fn modifier_only_mode(event: &KeyEvent) -> Option<HotkeyDisplayMode> {
    match event.code {
        KeyCode::Modifier(mod_key) => {
            use crossterm::event::ModifierKeyCode;
            match mod_key {
                ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => {
                    Some(HotkeyDisplayMode::CtrlHeld)
                }
                ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => {
                    Some(HotkeyDisplayMode::AltHeld)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Detects the portable badge-visibility chord (Ctrl+Space or
/// Alt+Space) regardless of press/release direction.
pub(super) fn sticky_toggle_mode(event: &KeyEvent) -> Option<HotkeyDisplayMode> {
    let has_ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = event.modifiers.contains(KeyModifiers::ALT);
    let is_space_chord = match event.code {
        KeyCode::Char(' ') => true,
        KeyCode::Char('\0') if has_ctrl => true,
        _ => false,
    };
    if !is_space_chord {
        return None;
    }
    match (has_ctrl, has_alt) {
        (true, false) => Some(HotkeyDisplayMode::CtrlHeld),
        (false, true) => Some(HotkeyDisplayMode::AltHeld),
        _ => None,
    }
}

/// Builds separate maps for effective Ctrl and Alt hotkeys.
pub(super) fn build_effective_hotkeys<V>(
    options: &[ChoiceOption<V>],
) -> (HashMap<char, usize>, HashMap<char, usize>) {
    let mut ctrl = HashMap::new();
    let mut alt = HashMap::new();
    for (idx, option) in options.iter().enumerate() {
        if let Some(hotkey) = option.effective_hotkey() {
            match hotkey {
                HotkeySpec::Ctrl(c) => {
                    ctrl.entry(c.to_ascii_lowercase()).or_insert(idx);
                }
                HotkeySpec::Alt(c) => {
                    alt.entry(c.to_ascii_lowercase()).or_insert(idx);
                }
            }
        }
    }
    (ctrl, alt)
}
