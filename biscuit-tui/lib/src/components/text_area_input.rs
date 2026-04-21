//! Multi-line text input component.
//!
//! [`TextAreaInput`] is a zero-sized [`StatefulWidget`] marker. The
//! mutable edit state, label, theme, and scrollbar/submit-key
//! configuration all live on [`TextAreaInputState`]. The state wraps
//! `tui_textarea::TextArea` as a private edit engine — callers never
//! see `tui_textarea` types.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//!
//! let state = TextAreaInputState::new(60, 10)
//!     .with_label(Label::new("Notes", LabelPosition::Above))
//!     .with_scrollbar(true)
//!     .with_value(&["line 1", "line 2"]);
//! assert_eq!(state.value(), "line 1\nline 2");
//! ```
//!
//! ## Notes
//!
//! - The default submit key is `Ctrl-S`. Override with
//!   [`TextAreaInputState::with_submit_key`].
//! - [`tui_textarea::TextArea`] treats `Enter` as "insert newline" so
//!   this component never maps `Enter` to submission.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};
use tui_textarea::TextArea;

use crate::core::{
    ComponentTheme, EventOutcome, HandleEvent, KeyBindings, Label, StandaloneState,
    ValidationState, render_with_label,
};

/// Mutable state for a [`TextAreaInput`] widget.
///
/// Holds the multi-line edit buffer, optional label, preferred width
/// and height, scrollbar preference, key bindings, theme, and any
/// active validation error.
#[derive(Debug, Clone)]
pub struct TextAreaInputState {
    inner: TextArea<'static>,
    label: Option<Label>,
    theme: ComponentTheme,
    show_scrollbar: bool,
    preferred_width: u16,
    preferred_height: u16,
    bindings: KeyBindings,
    validation_error: Option<String>,
}

impl Default for TextAreaInputState {
    fn default() -> Self {
        Self {
            inner: TextArea::default(),
            label: None,
            theme: ComponentTheme {
                help_hint: "Ctrl+S=Submit  Esc=Cancel".into(),
                ..ComponentTheme::default()
            },
            show_scrollbar: false,
            preferred_width: 60,
            preferred_height: 10,
            bindings: KeyBindings {
                submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
                ..KeyBindings::default()
            },
            validation_error: None,
        }
    }
}

impl TextAreaInputState {
    /// Creates an empty state with the given preferred `width` and
    /// `height` (in terminal cells).
    ///
    /// The preferred size is advisory — the caller chooses the actual
    /// draw area at render time. It is reported through
    /// [`TextAreaInputState::preferred_width`] /
    /// [`TextAreaInputState::preferred_height`] so that containers
    /// (e.g. an `InputTable` cell or the CLI's inline viewport) can
    /// size themselves appropriately.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            preferred_width: width,
            preferred_height: height,
            ..Self::default()
        }
    }

    /// Attaches a label rendered at the configured position.
    pub fn with_label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Enables or disables the scrollbar overlay.
    pub fn with_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    /// Initialises the buffer with `lines` and moves the cursor to the
    /// end of the last line.
    pub fn with_value<S: AsRef<str>>(mut self, lines: &[S]) -> Self {
        let owned: Vec<String> = lines.iter().map(|s| s.as_ref().to_string()).collect();
        let mut textarea = if owned.is_empty() {
            TextArea::default()
        } else {
            TextArea::new(owned)
        };
        textarea.move_cursor(tui_textarea::CursorMove::Bottom);
        textarea.move_cursor(tui_textarea::CursorMove::End);
        self.inner = textarea;
        self
    }

    /// Replaces the active theme.
    pub fn with_theme(mut self, theme: ComponentTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Replaces the key bindings.
    pub fn with_key_bindings(mut self, bindings: KeyBindings) -> Self {
        self.bindings = bindings;
        self
    }

    /// Overrides the submit key (the keystroke that completes editing
    /// and returns [`EventOutcome::Submitted`]). Default is `Ctrl-S`.
    ///
    /// This is a convenience wrapper over `with_key_bindings` for
    /// backwards compatibility.
    pub fn with_submit_key(mut self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        self.bindings.submit = vec![KeyEvent::new(code, modifiers)];
        self
    }

    /// Returns the buffer contents as a single string with `\n`
    /// between lines.
    pub fn value(&self) -> String {
        self.inner.lines().join("\n")
    }

    /// Returns the buffer contents as borrowed lines.
    pub fn lines(&self) -> &[String] {
        self.inner.lines()
    }

    /// Returns the cursor position as `(row, column)` in character
    /// offsets.
    pub fn cursor(&self) -> (usize, usize) {
        self.inner.cursor()
    }

    /// Returns the preferred render width.
    pub fn preferred_width(&self) -> u16 {
        self.preferred_width
    }

    /// Returns the preferred render height.
    pub fn preferred_height(&self) -> u16 {
        self.preferred_height
    }

    /// Returns whether the scrollbar overlay is enabled.
    pub fn scrollbar_enabled(&self) -> bool {
        self.show_scrollbar
    }

    /// Returns a reference to the attached label, if any.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Returns a reference to the component's theme.
    pub fn theme(&self) -> &ComponentTheme {
        &self.theme
    }

    /// Returns a reference to the key bindings.
    pub fn key_bindings(&self) -> &KeyBindings {
        &self.bindings
    }

    /// Sets an inline validation error. The message renders below the
    /// body on the next draw.
    pub fn set_validation_error(&mut self, message: impl Into<String>) {
        self.validation_error = Some(message.into());
    }
}

impl ValidationState for TextAreaInputState {
    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn clear_validation_error(&mut self) {
        self.validation_error = None;
    }
}

impl StandaloneState for TextAreaInputState {
    type Value = String;

    fn value(&self) -> Self::Value {
        TextAreaInputState::value(self)
    }

    fn help_hint(&self) -> &str {
        &self.theme.help_hint
    }
}

/// Multi-line text input widget.
///
/// Rendered via [`StatefulWidget`] against a [`TextAreaInputState`].
/// The widget itself is zero-sized — all state lives on the paired
/// state struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextAreaInput;

impl TextAreaInput {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidget for TextAreaInput {
    type State = TextAreaInputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let label = state.label.clone();
        let label_style = state.theme.label_style;
        let error_style = state.theme.error_style;
        let show_scrollbar = state.show_scrollbar;
        let error = state.validation_error.clone();
        let (reserve_for_error, error_rows) = match error.as_deref() {
            Some(_) if area.height >= 2 => (true, 1u16),
            _ => (false, 0u16),
        };

        let body_area = if reserve_for_error {
            Rect::new(area.x, area.y, area.width, area.height - error_rows)
        } else {
            area
        };

        let inner_area =
            render_with_label(body_area, buf, label.as_ref(), label_style, |rect, b| {
                draw_body(rect, b, &state.inner, show_scrollbar);
            });

        if let Some(message) = error {
            let error_row = inner_area.y + inner_area.height;
            if error_row < area.y + area.height {
                let error_line = Line::from(Span::styled(message, error_style));
                buf.set_line(area.x, error_row, &error_line, area.width);
            }
        }
    }
}

fn draw_body(area: Rect, buf: &mut Buffer, textarea: &TextArea<'static>, show_scrollbar: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let total_lines = textarea.lines().len();
    let overflow = show_scrollbar && total_lines > area.height as usize && area.width > 1;

    let text_area = if overflow {
        Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height)
    } else {
        area
    };

    Widget::render(textarea, text_area, buf);

    if overflow {
        let (cursor_row, _) = textarea.cursor();
        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .viewport_content_length(area.height as usize)
            .position(cursor_row);
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .render(area, buf, &mut scrollbar_state);
    }
}

impl HandleEvent for TextAreaInput {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        // Check bindings first.
        if KeyBindings::matches(&state.bindings.cancel, &event) {
            return EventOutcome::Cancelled;
        }
        if KeyBindings::matches(&state.bindings.submit, &event) {
            return EventOutcome::Submitted;
        }

        // Forward everything else to tui_textarea.
        state.clear_validation_error();
        state.inner.input(event);
        EventOutcome::Consumed
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

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    fn type_chars(widget: &TextAreaInput, state: &mut TextAreaInputState, chars: &str) {
        for c in chars.chars() {
            widget.handle_event(state, press(KeyCode::Char(c)));
        }
    }

    fn buffer_row(buf: &Buffer, y: u16) -> String {
        let mut row = String::new();
        for x in buf.area.left()..buf.area.right() {
            row.push_str(buf[(x, y)].symbol());
        }
        row.trim_end().to_string()
    }

    #[test]
    fn new_has_empty_value() {
        let state = TextAreaInputState::new(60, 10);
        assert_eq!(state.value(), "");
        assert_eq!(state.lines(), &[String::new()]);
        assert_eq!(state.preferred_width(), 60);
        assert_eq!(state.preferred_height(), 10);
    }

    #[test]
    fn with_value_initialises_multiple_lines() {
        let state = TextAreaInputState::new(40, 5).with_value(&["one", "two", "three"]);
        assert_eq!(state.lines(), &["one", "two", "three"]);
        assert_eq!(state.value(), "one\ntwo\nthree");
    }

    #[test]
    fn with_value_places_cursor_at_end() {
        let state = TextAreaInputState::new(40, 5).with_value(&["abc", "de"]);
        let (row, col) = state.cursor();
        assert_eq!(row, 1);
        assert_eq!(col, 2);
    }

    #[test]
    fn value_joins_lines_with_newlines() {
        let state = TextAreaInputState::new(40, 5).with_value(&["alpha", "beta"]);
        assert_eq!(state.value(), "alpha\nbeta");
    }

    #[test]
    fn typing_appends_characters() {
        let mut state = TextAreaInputState::new(40, 5);
        type_chars(&TextAreaInput, &mut state, "abc");
        assert_eq!(state.value(), "abc");
    }

    #[test]
    fn enter_inserts_a_newline_rather_than_submitting() {
        let mut state = TextAreaInputState::new(40, 5);
        type_chars(&TextAreaInput, &mut state, "ab");
        let outcome = TextAreaInput.handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.lines(), &["ab", ""]);
    }

    #[test]
    fn backspace_deletes_previous_char() {
        let mut state = TextAreaInputState::new(40, 5).with_value(&["abc"]);
        let outcome = TextAreaInput.handle_event(&mut state, press(KeyCode::Backspace));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.value(), "ab");
    }

    #[test]
    fn arrow_keys_move_cursor() {
        let mut state = TextAreaInputState::new(40, 5).with_value(&["abc", "def"]);
        TextAreaInput.handle_event(&mut state, press(KeyCode::Up));
        let (row, _) = state.cursor();
        assert_eq!(row, 0);
    }

    #[test]
    fn ctrl_s_submits_by_default() {
        let mut state = TextAreaInputState::new(40, 5).with_value(&["text"]);
        let outcome = TextAreaInput.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn esc_cancels() {
        let mut state = TextAreaInputState::new(40, 5);
        let outcome = TextAreaInput.handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn custom_submit_key_replaces_default() {
        let mut state = TextAreaInputState::new(40, 5)
            .with_submit_key(KeyCode::Char('d'), KeyModifiers::CONTROL);
        let outcome = TextAreaInput.handle_event(&mut state, ctrl(KeyCode::Char('d')));
        assert_eq!(outcome, EventOutcome::Submitted);
        // The previous default (`Ctrl-S`) no longer submits.
        let outcome = TextAreaInput.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Consumed);
    }

    #[test]
    fn custom_key_bindings_override_submit_and_cancel() {
        let bindings = KeyBindings {
            submit: vec![ctrl(KeyCode::Char('d'))],
            cancel: vec![press(KeyCode::F(4))],
            ..KeyBindings::default()
        };
        let mut state = TextAreaInputState::new(40, 5).with_key_bindings(bindings);

        let outcome = TextAreaInput.handle_event(&mut state, ctrl(KeyCode::Char('d')));
        assert_eq!(outcome, EventOutcome::Submitted);

        let outcome = TextAreaInput.handle_event(&mut state, ctrl(KeyCode::Char('s')));
        assert_eq!(outcome, EventOutcome::Consumed);

        let outcome = TextAreaInput.handle_event(&mut state, press(KeyCode::F(4)));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn typing_clears_active_validation_error() {
        let mut state = TextAreaInputState::new(40, 5);
        state.set_validation_error("required");
        assert_eq!(
            <TextAreaInputState as ValidationState>::validation_error(&state),
            Some("required"),
        );
        TextAreaInput.handle_event(&mut state, press(KeyCode::Char('x')));
        assert!(<TextAreaInputState as ValidationState>::validation_error(&state).is_none());
    }

    #[test]
    fn standalone_value_matches_state_value() {
        let state = TextAreaInputState::new(40, 5).with_value(&["a", "b"]);
        assert_eq!(StandaloneState::value(&state), "a\nb");
    }

    #[test]
    fn render_without_label_draws_lines_from_first_row() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let mut state = TextAreaInputState::new(10, 3).with_value(&["hi", "yo"]);
        TextAreaInput.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "hi");
        assert_eq!(buffer_row(&buf, 1), "yo");
    }

    #[test]
    fn render_with_label_above_draws_label_then_body() {
        let area = Rect::new(0, 0, 10, 4);
        let mut buf = Buffer::empty(area);
        let mut state = TextAreaInputState::new(10, 3)
            .with_value(&["hi"])
            .with_label(Label::new("Notes", LabelPosition::Above));
        TextAreaInput.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "Notes");
        assert_eq!(buffer_row(&buf, 1), "hi");
    }

    #[test]
    fn render_draws_validation_error_below_body() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        let mut state = TextAreaInputState::new(20, 2).with_value(&["hi"]);
        state.set_validation_error("required");
        TextAreaInput.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "hi");
        assert_eq!(buffer_row(&buf, 2), "required");
    }

    #[test]
    fn scrollbar_overlay_renders_when_content_exceeds_viewport() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let lines: Vec<String> = (0..10).map(|i| format!("l{i}")).collect();
        let mut state = TextAreaInputState::new(10, 3)
            .with_scrollbar(true)
            .with_value(&lines);
        TextAreaInput.render(area, &mut buf, &mut state);
        // Rightmost column should contain scrollbar glyphs (non-space).
        let mut any_glyph = false;
        for y in area.top()..area.bottom() {
            let sym = buf[(area.right() - 1, y)].symbol();
            if !sym.is_empty() && sym != " " {
                any_glyph = true;
                break;
            }
        }
        assert!(any_glyph, "expected scrollbar overlay on rightmost column");
    }

    #[test]
    fn scrollbar_is_hidden_when_content_fits() {
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        let mut state = TextAreaInputState::new(10, 5)
            .with_scrollbar(true)
            .with_value(&["one", "two"]);
        TextAreaInput.render(area, &mut buf, &mut state);
        // No scrollbar glyph expected on any row.
        for y in area.top()..area.bottom() {
            let sym = buf[(area.right() - 1, y)].symbol();
            assert!(
                sym.is_empty()
                    || sym == " "
                    || sym == "w"
                    || sym == "o"
                    || sym == "e"
                    || sym == "n",
                "unexpected glyph {sym:?} on rightmost column at y={y}",
            );
        }
    }
}
