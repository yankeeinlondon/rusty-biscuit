//! Boolean toggle switch component.
//!
//! [`BooleanSwitch`] is a zero-sized [`StatefulWidget`] marker. The
//! mutable state, label, on/off labels and theme live on
//! [`BooleanSwitchState`]. A boolean value is always valid so the
//! component does not implement
//! [`ValidationState`](crate::core::ValidationState).
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//!
//! let state = BooleanSwitchState::new()
//!     .with_label(Label::new("Enabled", LabelPosition::Above))
//!     .with_value(true);
//! assert!(state.checked());
//! ```

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::StatefulWidget,
};

use crate::core::{
    ComponentTheme, EventOutcome, HandleEvent, Label, StandaloneState, render_with_label,
};

/// Mutable state for a [`BooleanSwitch`] widget.
///
/// Holds the current checked flag, an optional label, customisable
/// on/off captions, and a theme reference.
#[derive(Debug, Clone)]
pub struct BooleanSwitchState {
    checked: bool,
    label: Option<Label>,
    on_label: String,
    off_label: String,
    theme: ComponentTheme,
}

impl Default for BooleanSwitchState {
    fn default() -> Self {
        Self {
            checked: false,
            label: None,
            on_label: "ON".into(),
            off_label: "OFF".into(),
            theme: ComponentTheme::default(),
        }
    }
}

impl BooleanSwitchState {
    /// Creates a new unchecked state with default theme and labels.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a label rendered at the configured position.
    pub fn with_label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the initial checked value.
    pub fn with_value(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Replaces the on/off captions painted on the switch track.
    ///
    /// Defaults are `"ON"` and `"OFF"`.
    pub fn with_labels(mut self, on: impl Into<String>, off: impl Into<String>) -> Self {
        self.on_label = on.into();
        self.off_label = off.into();
        self
    }

    /// Replaces the active theme.
    pub fn with_theme(mut self, theme: ComponentTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Returns the current checked flag.
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Sets the checked flag directly.
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    /// Flips the checked flag.
    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    /// Returns the on-side caption.
    pub fn on_label(&self) -> &str {
        &self.on_label
    }

    /// Returns the off-side caption.
    pub fn off_label(&self) -> &str {
        &self.off_label
    }

    /// Returns a reference to the attached label, if any.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Returns a reference to the component's theme.
    pub fn theme(&self) -> &ComponentTheme {
        &self.theme
    }
}

impl StandaloneState for BooleanSwitchState {
    type Value = bool;

    fn value(&self) -> Self::Value {
        self.checked
    }
}

/// Boolean toggle switch widget.
///
/// Rendered via [`StatefulWidget`] against a [`BooleanSwitchState`].
/// The widget itself is zero-sized — all state lives on the paired
/// state struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct BooleanSwitch;

impl BooleanSwitch {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidget for BooleanSwitch {
    type State = BooleanSwitchState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let label = state.label.clone();
        let label_style = state.theme.label_style;
        let selected_style = state.theme.selected_style;
        let thumb = state.theme.switch_thumb;
        let checked = state.checked;
        let on_label = state.on_label.clone();
        let off_label = state.off_label.clone();

        render_with_label(area, buf, label.as_ref(), label_style, |rect, b| {
            draw_body(
                rect,
                b,
                checked,
                &off_label,
                &on_label,
                thumb,
                selected_style,
            );
        });
    }
}

fn draw_body(
    area: Rect,
    buf: &mut Buffer,
    checked: bool,
    off_label: &str,
    on_label: &str,
    thumb: char,
    selected_style: Style,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let off_side = if checked {
        format!(" {off_label} ")
    } else {
        format!("{thumb}{off_label} ")
    };
    let on_side = if checked {
        format!(" {thumb}{on_label}")
    } else {
        format!(" {on_label} ")
    };

    let default_style = Style::default();
    let (off_style, on_style) = if checked {
        (default_style, selected_style)
    } else {
        (selected_style, default_style)
    };

    let line = Line::from(vec![
        Span::raw("["),
        Span::styled(off_side, off_style),
        Span::raw("|"),
        Span::styled(on_side, on_style),
        Span::raw("]"),
    ]);

    buf.set_line(area.x, area.y, &line, area.width);
}

impl HandleEvent for BooleanSwitch {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        if event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return EventOutcome::Ignored;
        }
        match event.code {
            KeyCode::Enter => EventOutcome::Submitted,
            KeyCode::Esc => EventOutcome::Cancelled,
            KeyCode::Char(' ') => {
                state.toggle();
                EventOutcome::Consumed
            }
            KeyCode::Left | KeyCode::Char('h') => {
                state.set_checked(false);
                EventOutcome::Consumed
            }
            KeyCode::Right | KeyCode::Char('l') => {
                state.set_checked(true);
                EventOutcome::Consumed
            }
            _ => EventOutcome::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Label, LabelPosition};
    use ratatui::buffer::Buffer;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn buffer_row(buf: &Buffer, y: u16) -> String {
        let mut row = String::new();
        for x in buf.area.left()..buf.area.right() {
            row.push_str(buf[(x, y)].symbol());
        }
        row.trim_end().to_string()
    }

    #[test]
    fn new_starts_unchecked() {
        let state = BooleanSwitchState::new();
        assert!(!state.checked());
    }

    #[test]
    fn with_value_sets_initial_checked() {
        let state = BooleanSwitchState::new().with_value(true);
        assert!(state.checked());
    }

    #[test]
    fn with_labels_overrides_captions() {
        let state = BooleanSwitchState::new().with_labels("YES", "NO");
        assert_eq!(state.on_label(), "YES");
        assert_eq!(state.off_label(), "NO");
    }

    #[test]
    fn toggle_flips_checked() {
        let mut state = BooleanSwitchState::new();
        state.toggle();
        assert!(state.checked());
        state.toggle();
        assert!(!state.checked());
    }

    #[test]
    fn set_checked_sets_value_directly() {
        let mut state = BooleanSwitchState::new();
        state.set_checked(true);
        assert!(state.checked());
        state.set_checked(false);
        assert!(!state.checked());
    }

    #[test]
    fn space_toggles() {
        let mut state = BooleanSwitchState::new();
        let outcome = BooleanSwitch.handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.checked());
        BooleanSwitch.handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(!state.checked());
    }

    #[test]
    fn left_sets_off() {
        let mut state = BooleanSwitchState::new().with_value(true);
        let outcome = BooleanSwitch.handle_event(&mut state, press(KeyCode::Left));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(!state.checked());
    }

    #[test]
    fn right_sets_on() {
        let mut state = BooleanSwitchState::new();
        let outcome = BooleanSwitch.handle_event(&mut state, press(KeyCode::Right));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.checked());
    }

    #[test]
    fn vim_h_sets_off_and_l_sets_on() {
        let mut state = BooleanSwitchState::new().with_value(true);
        BooleanSwitch.handle_event(&mut state, press(KeyCode::Char('h')));
        assert!(!state.checked());
        BooleanSwitch.handle_event(&mut state, press(KeyCode::Char('l')));
        assert!(state.checked());
    }

    #[test]
    fn enter_submits() {
        let mut state = BooleanSwitchState::new();
        let outcome = BooleanSwitch.handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn esc_cancels() {
        let mut state = BooleanSwitchState::new();
        let outcome = BooleanSwitch.handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn unhandled_keys_return_ignored() {
        let mut state = BooleanSwitchState::new();
        let outcome = BooleanSwitch.handle_event(&mut state, press(KeyCode::F(5)));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn modifier_chars_are_ignored() {
        let mut state = BooleanSwitchState::new();
        let event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        let outcome = BooleanSwitch.handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Ignored);
        assert!(!state.checked());
    }

    #[test]
    fn value_returns_bool() {
        let state = BooleanSwitchState::new().with_value(true);
        assert!(StandaloneState::value(&state));
    }

    #[test]
    fn render_unchecked_draws_thumb_on_off_side() {
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let mut state = BooleanSwitchState::new();
        BooleanSwitch.render(area, &mut buf, &mut state);
        let row = buffer_row(&buf, 0);
        assert_eq!(row, "[●OFF | ON ]");
    }

    #[test]
    fn render_checked_draws_thumb_on_on_side() {
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let mut state = BooleanSwitchState::new().with_value(true);
        BooleanSwitch.render(area, &mut buf, &mut state);
        let row = buffer_row(&buf, 0);
        assert_eq!(row, "[ OFF | ●ON]");
    }

    #[test]
    fn render_with_label_above_draws_label_then_body() {
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);
        let mut state = BooleanSwitchState::new()
            .with_label(Label::new("Enabled", LabelPosition::Above));
        BooleanSwitch.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "Enabled");
        assert_eq!(buffer_row(&buf, 1), "[●OFF | ON ]");
    }

    #[test]
    fn render_with_custom_labels() {
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let mut state = BooleanSwitchState::new()
            .with_labels("YES", "NO")
            .with_value(true);
        BooleanSwitch.render(area, &mut buf, &mut state);
        let row = buffer_row(&buf, 0);
        assert_eq!(row, "[ NO | ●YES]");
    }
}
