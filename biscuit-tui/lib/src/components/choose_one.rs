//! Single-selection list component.
//!
//! [`ChooseOne`] is a zero-sized [`StatefulWidget`] marker that renders
//! a vertical list of [`ChoiceOption`]s. State tracks the currently
//! hovered option, the selected option (if any), a scroll viewport,
//! hotkey mappings, and any submit-time validation error.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//!
//! let input = ChoiceInput::new("colour", "Pick a colour")
//!     .with_options(vec![
//!         ChoiceOption::new("r", "Red", "red"),
//!         ChoiceOption::new("g", "Green", "green"),
//!     ])
//!     .required();
//! let state = ChooseOneState::new(input);
//! assert!(state.selected_index().is_none());
//! ```

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::StatefulWidget,
};

use crate::core::{
    ComponentTheme, EventOutcome, HandleEvent, Label, StandaloneState, ValidationState,
    render_with_label,
};

use super::choose::{ChoiceInput, ChoiceOption, SelectionMode};

/// Mutable state for a [`ChooseOne`] widget.
///
/// Wraps a [`ChoiceInput<String>`] and adds transient UI state: the
/// hovered row, the selected row (if any), scroll offset, precomputed
/// hotkey map and any active submit-time validation error.
#[derive(Debug, Clone)]
pub struct ChooseOneState {
    input: ChoiceInput<String>,
    hover: usize,
    selected: Option<usize>,
    scroll_offset: usize,
    hotkeys: HashMap<char, usize>,
    label: Option<Label>,
    theme: ComponentTheme,
    validation_error: Option<String>,
}

impl ChooseOneState {
    /// Builds a new state from a [`ChoiceInput<String>`].
    ///
    /// The selection mode is forced to [`SelectionMode::Single`]
    /// regardless of what `input` specifies — `ChooseOne` is a
    /// single-select component.
    pub fn new(mut input: ChoiceInput<String>) -> Self {
        input.selection_mode = SelectionMode::Single;
        let hotkeys = build_hotkeys(&input.options);
        let hover = first_enabled_index(&input.options).unwrap_or(0);
        Self {
            input,
            hover,
            selected: None,
            scroll_offset: 0,
            hotkeys,
            label: None,
            theme: ComponentTheme::default(),
            validation_error: None,
        }
    }

    /// Shortcut constructor from a bare vec of options.
    pub fn from_options(options: Vec<ChoiceOption<String>>) -> Self {
        Self::new(ChoiceInput::new("", "").with_options(options))
    }

    /// Attaches a label rendered at the configured position.
    pub fn with_label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Replaces the active theme.
    pub fn with_theme(mut self, theme: ComponentTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Pre-selects an option by its stable identifier.
    pub fn with_initial_selection(mut self, id: &str) -> Self {
        if let Some((idx, _)) = self
            .input
            .options
            .iter()
            .enumerate()
            .find(|(_, option)| option.id == id)
        {
            self.selected = Some(idx);
            self.hover = idx;
        }
        self
    }

    /// Returns the full list of options.
    pub fn options(&self) -> &[ChoiceOption<String>] {
        &self.input.options
    }

    /// Returns the index of the currently hovered option, if the list
    /// is non-empty.
    pub fn hover(&self) -> Option<usize> {
        if self.input.options.is_empty() {
            None
        } else {
            Some(self.hover)
        }
    }

    /// Returns the index of the currently selected option, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    /// Returns the stable `id` of the selected option, if any.
    pub fn selected_id(&self) -> Option<&str> {
        self.selected
            .and_then(|idx| self.input.options.get(idx))
            .map(|option| option.id.as_str())
    }

    /// Returns the typed `value` of the selected option, if any.
    pub fn selected_value(&self) -> Option<&str> {
        self.selected
            .and_then(|idx| self.input.options.get(idx))
            .map(|option| option.value.as_str())
    }

    /// Returns `true` when the input was marked required.
    pub fn required(&self) -> bool {
        self.input.required
    }

    /// Returns a reference to the attached label, if any.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Returns a reference to the component theme.
    pub fn theme(&self) -> &ComponentTheme {
        &self.theme
    }

    /// Returns the precomputed hotkey map keyed by lowercase char.
    pub fn hotkeys(&self) -> &HashMap<char, usize> {
        &self.hotkeys
    }

    /// Sets an inline validation error.
    pub fn set_validation_error(&mut self, message: impl Into<String>) {
        self.validation_error = Some(message.into());
    }
}

impl ValidationState for ChooseOneState {
    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn clear_validation_error(&mut self) {
        self.validation_error = None;
    }
}

impl StandaloneState for ChooseOneState {
    type Value = Option<String>;

    fn value(&self) -> Self::Value {
        self.selected_value().map(|v| v.to_string())
    }

    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }
}

/// Single-selection list widget.
///
/// Rendered via [`StatefulWidget`] against a [`ChooseOneState`]. The
/// widget itself is zero-sized — all state lives on the paired state
/// struct.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChooseOne;

impl ChooseOne {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self
    }
}

impl StatefulWidget for ChooseOne {
    type State = ChooseOneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let label = state.label.clone();
        let label_style = state.theme.label_style;

        let inner_area = render_with_label(area, buf, label.as_ref(), label_style, |rect, b| {
            draw_list(rect, b, state);
        });

        if let Some(message) = state.validation_error.as_deref()
            && let Some(error_row) = error_row_y(inner_area, state.input.options.len())
        {
            let error_line =
                Line::from(Span::styled(message.to_string(), state.theme.error_style));
            buf.set_line(inner_area.x, error_row, &error_line, inner_area.width);
        }
    }
}

impl HandleEvent for ChooseOne {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        if event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return EventOutcome::Ignored;
        }
        match event.code {
            KeyCode::Esc => EventOutcome::Cancelled,
            KeyCode::Enter => submit(state),
            KeyCode::Char(' ') => {
                select_hover(state);
                EventOutcome::Consumed
            }
            KeyCode::Up | KeyCode::Char('k') => {
                move_hover(state, -1);
                EventOutcome::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                move_hover(state, 1);
                EventOutcome::Consumed
            }
            KeyCode::Home | KeyCode::Char('g') => {
                jump_to(state, first_enabled_index(&state.input.options).unwrap_or(0));
                EventOutcome::Consumed
            }
            KeyCode::End | KeyCode::Char('G') => {
                jump_to(
                    state,
                    last_enabled_index(&state.input.options)
                        .unwrap_or(state.input.options.len().saturating_sub(1)),
                );
                EventOutcome::Consumed
            }
            KeyCode::Char(c) => {
                if let Some(&idx) = state.hotkeys.get(&c.to_ascii_lowercase()) {
                    jump_to(state, idx);
                    select_at(state, idx);
                    return EventOutcome::Consumed;
                }
                EventOutcome::Ignored
            }
            _ => EventOutcome::Ignored,
        }
    }
}

fn submit(state: &mut ChooseOneState) -> EventOutcome {
    if state.selected.is_none() && state.input.required {
        state.validation_error = Some("Please make a selection".into());
        return EventOutcome::Consumed;
    }
    EventOutcome::Submitted
}

fn select_hover(state: &mut ChooseOneState) {
    let idx = state.hover;
    select_at(state, idx);
}

fn select_at(state: &mut ChooseOneState, idx: usize) {
    if let Some(option) = state.input.options.get(idx)
        && !option.disabled
    {
        state.selected = Some(idx);
        state.validation_error = None;
    }
}

fn move_hover(state: &mut ChooseOneState, delta: i32) {
    if state.input.options.is_empty() {
        return;
    }
    let len = state.input.options.len();
    let mut idx = state.hover as i32;
    for _ in 0..len {
        idx += delta;
        if idx < 0 {
            idx += len as i32;
        } else if idx >= len as i32 {
            idx -= len as i32;
        }
        let candidate = idx as usize;
        if !state.input.options[candidate].disabled {
            state.hover = candidate;
            return;
        }
    }
}

fn jump_to(state: &mut ChooseOneState, idx: usize) {
    if state.input.options.get(idx).is_some_and(|o| !o.disabled) {
        state.hover = idx;
    }
}

pub(super) fn first_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> {
    options.iter().position(|option| !option.disabled)
}

pub(super) fn last_enabled_index<V>(options: &[ChoiceOption<V>]) -> Option<usize> {
    options.iter().rposition(|option| !option.disabled)
}

pub(super) fn build_hotkeys<V>(options: &[ChoiceOption<V>]) -> HashMap<char, usize> {
    let mut map = HashMap::new();
    for (idx, option) in options.iter().enumerate() {
        if option.disabled {
            continue;
        }
        if let Some(first) = option.label.chars().next() {
            let lower = first.to_ascii_lowercase();
            map.entry(lower).or_insert(idx);
        }
    }
    map
}

fn error_row_y(inner_area: Rect, option_count: usize) -> Option<u16> {
    if inner_area.height == 0 {
        return None;
    }
    let list_rows = option_count.min(inner_area.height as usize) as u16;
    let candidate = inner_area.y + list_rows;
    if candidate < inner_area.y + inner_area.height {
        Some(candidate)
    } else {
        None
    }
}

fn draw_list(area: Rect, buf: &mut Buffer, state: &mut ChooseOneState) {
    if area.width == 0 || area.height == 0 || state.input.options.is_empty() {
        return;
    }

    // Reserve one row for a validation error when present.
    let body_rows = if state.validation_error.is_some() && area.height > 1 {
        area.height - 1
    } else {
        area.height
    };
    let visible = body_rows as usize;

    adjust_scroll(state, visible);

    let selected_indicator = state.theme.selected_indicator.clone();
    let unselected_indicator = state.theme.unselected_indicator.clone();
    let hover_style = state.theme.selected_style;
    let disabled_style = state.theme.disabled_style;

    for (row, idx) in (state.scroll_offset..state.input.options.len())
        .take(visible)
        .enumerate()
    {
        let option = &state.input.options[idx];
        let indicator = if Some(idx) == state.selected {
            &selected_indicator
        } else {
            &unselected_indicator
        };
        let prefix = format!("{indicator} ");
        let label_style = if option.disabled {
            disabled_style
        } else if idx == state.hover {
            hover_style
        } else {
            Style::default()
        };
        let line = Line::from(vec![
            Span::raw(prefix),
            Span::styled(option.label.clone(), label_style),
        ]);
        let y = area.y + row as u16;
        buf.set_line(area.x, y, &line, area.width);
    }
}

fn adjust_scroll(state: &mut ChooseOneState, visible: usize) {
    if visible == 0 {
        return;
    }
    if state.hover < state.scroll_offset {
        state.scroll_offset = state.hover;
    } else if state.hover >= state.scroll_offset + visible {
        state.scroll_offset = state.hover + 1 - visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Label, LabelPosition};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn fixture_input() -> ChoiceInput<String> {
        ChoiceInput::new("colour", "Pick a colour").with_options(vec![
            ChoiceOption::new("r", "Red", "red"),
            ChoiceOption::new("g", "Green", "green"),
            ChoiceOption::new("b", "Blue", "blue"),
        ])
    }

    fn buffer_row(buf: &Buffer, y: u16) -> String {
        let mut row = String::new();
        for x in buf.area.left()..buf.area.right() {
            row.push_str(buf[(x, y)].symbol());
        }
        row.trim_end().to_string()
    }

    #[test]
    fn new_starts_with_hover_on_first_enabled_option() {
        let state = ChooseOneState::new(fixture_input());
        assert_eq!(state.hover(), Some(0));
        assert!(state.selected_index().is_none());
    }

    #[test]
    fn hover_skips_disabled_options() {
        let input = ChoiceInput::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "A", "a").disabled(),
            ChoiceOption::new("b", "B", "b"),
        ]);
        let state = ChooseOneState::new(input);
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn down_arrow_moves_hover_forward() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne.handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn up_arrow_wraps_around() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne.handle_event(&mut state, press(KeyCode::Up));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn vim_j_and_k_navigate() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne.handle_event(&mut state, press(KeyCode::Char('j')));
        assert_eq!(state.hover(), Some(1));
        ChooseOne.handle_event(&mut state, press(KeyCode::Char('k')));
        assert_eq!(state.hover(), Some(0));
    }

    #[test]
    fn space_selects_hovered_option() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne.handle_event(&mut state, press(KeyCode::Down));
        let outcome = ChooseOne.handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.selected_value(), Some("green"));
    }

    #[test]
    fn hotkey_selects_target_option_directly() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne.handle_event(&mut state, press(KeyCode::Char('b')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_index(), Some(2));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn hotkey_is_case_insensitive() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne.handle_event(&mut state, press(KeyCode::Char('R')));
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn space_on_disabled_option_is_ignored() {
        let input = ChoiceInput::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "A", "a"),
            ChoiceOption::new("b", "B", "b").disabled(),
        ]);
        let mut state = ChooseOneState::new(input);
        state.hover = 1;
        ChooseOne.handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(state.selected_index().is_none());
    }

    #[test]
    fn enter_without_selection_on_required_input_sets_validation_error() {
        let input = fixture_input().required();
        let mut state = ChooseOneState::new(input);
        let outcome = ChooseOne.handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(
            <ChooseOneState as ValidationState>::validation_error(&state)
                .unwrap()
                .contains("selection"),
        );
    }

    #[test]
    fn enter_after_selection_submits_even_when_required() {
        let input = fixture_input().required();
        let mut state = ChooseOneState::new(input);
        ChooseOne.handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = ChooseOne.handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn enter_without_selection_not_required_submits() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne.handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn esc_cancels() {
        let mut state = ChooseOneState::new(fixture_input());
        let outcome = ChooseOne.handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn modifier_chords_are_ignored() {
        let mut state = ChooseOneState::new(fixture_input());
        let event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        let outcome = ChooseOne.handle_event(&mut state, event);
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn initial_selection_pins_hover_and_selected() {
        let state = ChooseOneState::new(fixture_input()).with_initial_selection("g");
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn hotkeys_map_lowercased_first_chars() {
        let state = ChooseOneState::new(fixture_input());
        assert_eq!(state.hotkeys().get(&'r'), Some(&0));
        assert_eq!(state.hotkeys().get(&'g'), Some(&1));
        assert_eq!(state.hotkeys().get(&'b'), Some(&2));
    }

    #[test]
    fn hotkey_duplicates_go_to_first_occurrence() {
        let input = ChoiceInput::new("x", "P").with_options(vec![
            ChoiceOption::new("a1", "Apple", "apple"),
            ChoiceOption::new("a2", "Avocado", "avocado"),
        ]);
        let state = ChooseOneState::new(input);
        assert_eq!(state.hotkeys().get(&'a'), Some(&0));
    }

    #[test]
    fn render_draws_indicator_and_label_per_row() {
        let mut state = ChooseOneState::new(fixture_input());
        ChooseOne.handle_event(&mut state, press(KeyCode::Char(' ')));
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseOne.render(area, &mut buf, &mut state);
        assert!(buffer_row(&buf, 0).starts_with("● Red"));
        assert!(buffer_row(&buf, 1).starts_with("○ Green"));
        assert!(buffer_row(&buf, 2).starts_with("○ Blue"));
    }

    #[test]
    fn render_with_label_above_draws_label_then_list() {
        let mut state = ChooseOneState::new(fixture_input())
            .with_label(Label::new("Colour", LabelPosition::Above));
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        ChooseOne.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "Colour");
        assert!(buffer_row(&buf, 1).starts_with("○ Red"));
    }

    #[test]
    fn render_draws_validation_error_on_row_below_list() {
        let input = fixture_input().required();
        let mut state = ChooseOneState::new(input);
        state.set_validation_error("Please make a selection");
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseOne.render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 3), "Please make a selection");
    }

    #[test]
    fn scroll_offset_tracks_hover_past_visible_window() {
        let options: Vec<ChoiceOption<String>> = (0..6)
            .map(|i| {
                ChoiceOption::new(
                    format!("id{i}"),
                    format!("Option {i}"),
                    format!("value{i}"),
                )
            })
            .collect();
        let mut state =
            ChooseOneState::new(ChoiceInput::new("x", "P").with_options(options));
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        for _ in 0..4 {
            ChooseOne.handle_event(&mut state, press(KeyCode::Down));
        }
        ChooseOne.render(area, &mut buf, &mut state);
        assert!(state.scroll_offset > 0);
        assert!(buffer_row(&buf, 2).contains("Option 4"));
    }
}
