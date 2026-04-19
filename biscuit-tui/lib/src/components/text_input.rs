//! Single-line text input component.
//!
//! [`TextInput`] is a zero-sized [`StatefulWidget`] marker. The mutable
//! edit buffer, label, theme, and validation slot all live on
//! [`TextInputState`]. The state wraps `tui_input::Input` as a private
//! edit engine — callers never see `tui_input` types.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//!
//! let state = TextInputState::new()
//!     .with_label(Label::new("Name", LabelPosition::Above))
//!     .with_max_length(32)
//!     .with_value("Alice");
//! assert_eq!(state.value(), "Alice");
//! ```

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::StatefulWidget,
};
use tui_input::{Input, InputRequest};
use unicode_width::UnicodeWidthStr;

use crate::core::{
    ComponentTheme, EventOutcome, HandleEvent, Label, StandaloneState, ValidationState,
    render_with_label,
};

/// Mutable state for a [`TextInput`] widget.
///
/// Holds the edit buffer, optional label, optional max-length cap,
/// theme, and any active validation error.
#[derive(Debug, Clone, Default)]
pub struct TextInputState {
    inner: Input,
    label: Option<Label>,
    max_length: Option<usize>,
    theme: ComponentTheme,
    validation_error: Option<String>,
}

impl TextInputState {
    /// Creates an empty state with default theme and no label.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a label rendered at the configured position.
    pub fn with_label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Caps the buffer length (in characters) that the user can type.
    ///
    /// Keystrokes that would exceed the cap are silently dropped
    /// (keystroke-time rejection — no validation error surfaces).
    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    /// Initialises the buffer with `value` and places the cursor at
    /// the end.
    pub fn with_value(mut self, value: &str) -> Self {
        self.inner = Input::new(value.to_string());
        self
    }

    /// Replaces the active theme.
    pub fn with_theme(mut self, theme: ComponentTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Returns the current buffer contents.
    pub fn value(&self) -> &str {
        self.inner.value()
    }

    /// Returns the cursor position as a character offset.
    pub fn cursor(&self) -> usize {
        self.inner.cursor()
    }

    /// Returns the configured max length, if any.
    pub fn max_length(&self) -> Option<usize> {
        self.max_length
    }

    /// Returns a reference to the component's theme.
    pub fn theme(&self) -> &ComponentTheme {
        &self.theme
    }

    /// Returns a reference to the attached label, if any.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Sets an inline validation error. The message renders below the
    /// input on the next draw.
    pub fn set_validation_error(&mut self, message: impl Into<String>) {
        self.validation_error = Some(message.into());
    }
}

impl ValidationState for TextInputState {
    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn clear_validation_error(&mut self) {
        self.validation_error = None;
    }
}

impl StandaloneState for TextInputState {
    type Value = String;

    fn value(&self) -> Self::Value {
        self.inner.value().to_string()
    }
}

/// Single-line text input widget.
///
/// Rendered via [`StatefulWidget`] against a [`TextInputState`]. The
/// widget itself is zero-sized — all state lives in the paired
/// [`TextInputState`].
#[derive(Debug, Clone, Copy, Default)]
pub struct TextInput;

impl TextInput {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidget for TextInput {
    type State = TextInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let label = state.label.clone();
        let label_style = state.theme.label_style;
        let cursor_style = state.theme.cursor_style;
        let error_style = state.theme.error_style;
        let value = state.inner.value().to_string();
        let cursor = state.inner.cursor();
        let error = state.validation_error.clone();

        let inner_area = render_with_label(area, buf, label.as_ref(), label_style, |rect, b| {
            draw_body(rect, b, &value, cursor, cursor_style);
        });

        if let Some(message) = error
            && inner_area.y + 1 < area.y + area.height
        {
            let error_row = inner_area.y + 1;
            let error_line = Line::from(Span::styled(message, error_style));
            buf.set_line(inner_area.x, error_row, &error_line, inner_area.width);
        }
    }
}

fn draw_body(area: Rect, buf: &mut Buffer, value: &str, cursor: usize, cursor_style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let line = Line::from(value.to_string());
    buf.set_line(area.x, area.y, &line, area.width);

    let cursor_offset = visual_offset_for(value, cursor);
    let cursor_glyph = grapheme_at(value, cursor).unwrap_or(" ");

    if cursor_offset < area.width as usize {
        let x = area.x + cursor_offset as u16;
        buf.set_stringn(x, area.y, cursor_glyph, cursor_glyph.len(), cursor_style);
    }
}

fn visual_offset_for(value: &str, cursor_chars: usize) -> usize {
    let prefix: String = value.chars().take(cursor_chars).collect();
    prefix.width()
}

fn grapheme_at(value: &str, cursor_chars: usize) -> Option<&str> {
    let mut indices = value.char_indices();
    let start = indices.nth(cursor_chars).map(|(i, _)| i)?;
    let end = value[start..]
        .char_indices()
        .nth(1)
        .map(|(i, _)| start + i)
        .unwrap_or(value.len());
    Some(&value[start..end])
}

impl HandleEvent for TextInput {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        match event.code {
            KeyCode::Enter => EventOutcome::Submitted,
            KeyCode::Esc => EventOutcome::Cancelled,
            KeyCode::Left => edit(state, InputRequest::GoToPrevChar),
            KeyCode::Right => edit(state, InputRequest::GoToNextChar),
            KeyCode::Home => edit(state, InputRequest::GoToStart),
            KeyCode::End => edit(state, InputRequest::GoToEnd),
            KeyCode::Backspace => edit(state, InputRequest::DeletePrevChar),
            KeyCode::Delete => edit(state, InputRequest::DeleteNextChar),
            KeyCode::Char(c) => {
                if event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                    return EventOutcome::Ignored;
                }
                if let Some(max) = state.max_length
                    && state.inner.value().chars().count() >= max
                {
                    return EventOutcome::Consumed;
                }
                edit(state, InputRequest::InsertChar(c))
            }
            _ => EventOutcome::Ignored,
        }
    }
}

fn edit(state: &mut TextInputState, request: InputRequest) -> EventOutcome {
    state.clear_validation_error();
    state.inner.handle(request);
    EventOutcome::Consumed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Label, LabelPosition};
    use ratatui::buffer::Buffer;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn type_chars(widget: &TextInput, state: &mut TextInputState, chars: &str) {
        for c in chars.chars() {
            widget.handle_event(state, press(KeyCode::Char(c)));
        }
    }

    #[test]
    fn new_has_empty_value() {
        let state = TextInputState::new();
        assert_eq!(state.value(), "");
        assert_eq!(state.cursor(), 0);
        assert!(state.max_length().is_none());
    }

    #[test]
    fn with_value_places_cursor_at_end() {
        let state = TextInputState::new().with_value("hello");
        assert_eq!(state.value(), "hello");
        assert_eq!(state.cursor(), 5);
    }

    #[test]
    fn typing_appends_characters() {
        let mut state = TextInputState::new();
        type_chars(&TextInput, &mut state, "abc");
        assert_eq!(state.value(), "abc");
        assert_eq!(state.cursor(), 3);
    }

    #[test]
    fn backspace_deletes_previous_char() {
        let mut state = TextInputState::new().with_value("abc");
        let outcome = TextInput.handle_event(&mut state, press(KeyCode::Backspace));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.value(), "ab");
    }

    #[test]
    fn cursor_left_moves_back_one() {
        let mut state = TextInputState::new().with_value("abc");
        TextInput.handle_event(&mut state, press(KeyCode::Left));
        assert_eq!(state.cursor(), 2);
    }

    #[test]
    fn home_and_end_jump_across_the_buffer() {
        let mut state = TextInputState::new().with_value("abcdef");
        TextInput.handle_event(&mut state, press(KeyCode::Home));
        assert_eq!(state.cursor(), 0);
        TextInput.handle_event(&mut state, press(KeyCode::End));
        assert_eq!(state.cursor(), 6);
    }

    #[test]
    fn delete_removes_next_char() {
        let mut state = TextInputState::new().with_value("abc");
        TextInput.handle_event(&mut state, press(KeyCode::Home));
        TextInput.handle_event(&mut state, press(KeyCode::Delete));
        assert_eq!(state.value(), "bc");
    }

    #[test]
    fn max_length_blocks_additional_keystrokes_silently() {
        let mut state = TextInputState::new().with_max_length(3);
        type_chars(&TextInput, &mut state, "abcd");
        assert_eq!(state.value(), "abc");
        let outcome = TextInput.handle_event(&mut state, press(KeyCode::Char('e')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.value(), "abc");
    }

    #[test]
    fn enter_submits() {
        let mut state = TextInputState::new().with_value("ok");
        let outcome = TextInput.handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn esc_cancels() {
        let mut state = TextInputState::new();
        let outcome = TextInput.handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn unhandled_keys_return_ignored() {
        let mut state = TextInputState::new();
        let outcome = TextInput.handle_event(&mut state, press(KeyCode::F(5)));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn modifier_chars_are_ignored() {
        let mut state = TextInputState::new();
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let outcome = TextInput.handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Ignored);
        assert_eq!(state.value(), "");
    }

    #[test]
    fn typing_clears_active_validation_error() {
        let mut state = TextInputState::new();
        state.set_validation_error("required");
        assert_eq!(
            <TextInputState as ValidationState>::validation_error(&state),
            Some("required"),
        );
        TextInput.handle_event(&mut state, press(KeyCode::Char('a')));
        assert!(<TextInputState as ValidationState>::validation_error(&state).is_none());
    }

    fn buffer_row(buf: &Buffer, y: u16) -> String {
        let mut row = String::new();
        for x in buf.area.left()..buf.area.right() {
            row.push_str(buf[(x, y)].symbol());
        }
        row.trim_end().to_string()
    }

    #[test]
    fn render_without_label_draws_value_on_first_row() {
        let area = Rect::new(0, 0, 10, 1);
        let mut buf = Buffer::empty(area);
        let mut state = TextInputState::new().with_value("hi");
        TextInput.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "hi");
    }

    #[test]
    fn render_with_label_above_draws_label_then_body() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let mut state = TextInputState::new()
            .with_value("hi")
            .with_label(Label::new("Name", LabelPosition::Above));
        TextInput.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "Name");
        assert_eq!(buffer_row(&buf, 1), "hi");
    }

    #[test]
    fn render_with_label_left_reserves_left_columns() {
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let mut state = TextInputState::new()
            .with_value("hi")
            .with_label(Label::new("Name", LabelPosition::Left));
        TextInput.render(area, &mut buf, &mut state);
        let row = buffer_row(&buf, 0);
        assert!(row.starts_with("Name"));
        assert!(row.contains("hi"));
    }

    #[test]
    fn render_draws_validation_error_below_body() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        let mut state = TextInputState::new().with_value("hi");
        state.set_validation_error("required");
        TextInput.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "hi");
        assert_eq!(buffer_row(&buf, 1), "required");
    }
}
