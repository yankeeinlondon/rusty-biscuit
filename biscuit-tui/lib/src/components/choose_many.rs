//! Multi-selection list component.
//!
//! [`ChooseMany`] is a zero-sized [`StatefulWidget`] marker that
//! renders a vertical list of [`ChoiceOption`]s with checkbox glyphs.
//! Users toggle selections with Space and submit with Enter.
//! `max_selections` is enforced at keystroke time (toggle-on is
//! silently dropped when the cap is reached); `required` and
//! `min_selections` are validated at submit time.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::prelude::*;
//!
//! let input: ChoiceInput = ChoiceInput::new("toppings", "Pick toppings")
//!     .with_options(vec![
//!         ChoiceOption::new("p", "Pepperoni", "pepperoni"),
//!         ChoiceOption::new("m", "Mushrooms", "mushrooms"),
//!     ])
//!     .with_max_selections(1);
//! let state = ChooseManyState::new(input);
//! assert!(state.selected_values().is_empty());
//! ```

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::StatefulWidget,
};

use crate::core::{
    ComponentTheme, EventOutcome, HandleEvent, KeyBindings, Label, StandaloneState,
    ValidationState, render_with_label,
};

use super::choose::{ChoiceInput, ChoiceOption, SelectionMode};
use super::choose_one::{build_hotkeys, first_enabled_index, last_enabled_index};

const CHECKED_GLYPH: &str = "☑";
const UNCHECKED_GLYPH: &str = "☐";

/// Mutable state for a [`ChooseMany`] widget.
///
/// Wraps a [`ChoiceInput<V>`] (forced to `SelectionMode::Multiple`)
/// and adds transient UI state: hovered row, per-option selected flags,
/// scroll offset, precomputed hotkey map, key bindings, and any active
/// validation error.
///
/// The type parameter `V` defaults to `String` for backward
/// compatibility.
#[derive(Debug, Clone)]
pub struct ChooseManyState<V = String> {
    input: ChoiceInput<V>,
    selected: Vec<bool>,
    hover: usize,
    scroll_offset: usize,
    hotkeys: HashMap<char, usize>,
    label: Option<Label>,
    theme: ComponentTheme,
    bindings: KeyBindings,
    validation_error: Option<String>,
}

impl<V: Clone + PartialEq> ChooseManyState<V> {
    /// Builds a new state from a [`ChoiceInput<V>`].
    ///
    /// The selection mode is forced to [`SelectionMode::Multiple`].
    pub fn new(mut input: ChoiceInput<V>) -> Self {
        input.selection_mode = SelectionMode::Multiple;
        let selected = vec![false; input.options.len()];
        let hotkeys = build_hotkeys(&input.options);
        let hover = first_enabled_index(&input.options).unwrap_or(0);
        Self {
            input,
            selected,
            hover,
            scroll_offset: 0,
            hotkeys,
            label: None,
            theme: ComponentTheme::default(),
            bindings: KeyBindings::default(),
            validation_error: None,
        }
    }

    /// Shortcut constructor from a bare vec of options.
    pub fn from_options(options: Vec<ChoiceOption<V>>) -> Self {
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

    /// Replaces the key bindings.
    pub fn with_key_bindings(mut self, bindings: KeyBindings) -> Self {
        self.bindings = bindings;
        self
    }

    /// Pre-selects options by their stable identifiers.
    pub fn with_initial_selection(mut self, ids: &[&str]) -> Self {
        for id in ids {
            if let Some((idx, _)) = self
                .input
                .options
                .iter()
                .enumerate()
                .find(|(_, option)| option.id == *id)
            {
                self.selected[idx] = true;
            }
        }
        self
    }

    /// Returns the full list of options.
    pub fn options(&self) -> &[ChoiceOption<V>] {
        &self.input.options
    }

    /// Returns the index of the currently hovered option, if any.
    pub fn hover(&self) -> Option<usize> {
        if self.input.options.is_empty() {
            None
        } else {
            Some(self.hover)
        }
    }

    /// Returns whether the option at `idx` is currently selected.
    pub fn is_selected(&self, idx: usize) -> bool {
        self.selected.get(idx).copied().unwrap_or(false)
    }

    /// Returns the stable ids of every selected option.
    pub fn selected_ids(&self) -> Vec<&str> {
        self.selected
            .iter()
            .zip(self.input.options.iter())
            .filter_map(|(flag, option)| {
                if *flag {
                    Some(option.id.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the typed values of every selected option.
    pub fn selected_values(&self) -> Vec<&V> {
        self.selected
            .iter()
            .zip(self.input.options.iter())
            .filter_map(
                |(flag, option)| {
                    if *flag { Some(&option.value) } else { None }
                },
            )
            .collect()
    }

    /// Returns how many options are currently selected.
    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|flag| **flag).count()
    }

    /// Returns `true` when the input was marked required.
    pub fn required(&self) -> bool {
        self.input.required
    }

    /// Returns the `min_selections` cap, if any.
    pub fn min_selections(&self) -> Option<usize> {
        self.input.min_selections
    }

    /// Returns the `max_selections` cap, if any.
    pub fn max_selections(&self) -> Option<usize> {
        self.input.max_selections
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

    /// Returns a reference to the key bindings.
    pub fn key_bindings(&self) -> &KeyBindings {
        &self.bindings
    }

    /// Sets an inline validation error.
    pub fn set_validation_error(&mut self, message: impl Into<String>) {
        self.validation_error = Some(message.into());
    }
}

impl<V: Clone + PartialEq> ValidationState for ChooseManyState<V> {
    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn clear_validation_error(&mut self) {
        self.validation_error = None;
    }
}

impl<V: Clone + PartialEq> StandaloneState for ChooseManyState<V> {
    type Value = Vec<V>;

    fn value(&self) -> Self::Value {
        self.selected_values().into_iter().cloned().collect()
    }

    fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    fn help_hint(&self) -> &str {
        &self.theme.help_hint
    }
}

/// Multi-selection list widget.
///
/// Rendered via [`StatefulWidget`] against a [`ChooseManyState<V>`]. The
/// widget itself is zero-sized — all state lives on the paired state
/// struct.
///
/// The type parameter `V` must match the state's value type. Defaults
/// to `String` for backward compatibility.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChooseMany<V = String> {
    _phantom: std::marker::PhantomData<V>,
}

impl<V> ChooseMany<V> {
    /// Convenience constructor.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<V: Clone + PartialEq> StatefulWidget for ChooseMany<V> {
    type State = ChooseManyState<V>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let label = state.label.clone();
        let label_style = state.theme.label_style;

        let inner_area = render_with_label(area, buf, label.as_ref(), label_style, |rect, b| {
            draw_list(rect, b, state);
        });

        if let Some(message) = state.validation_error.as_deref()
            && let Some(error_row) = error_row_y(inner_area, state.input.options.len())
        {
            let error_line = Line::from(Span::styled(message.to_string(), state.theme.error_style));
            buf.set_line(inner_area.x, error_row, &error_line, inner_area.width);
        }
    }
}

impl<V: Clone + PartialEq> HandleEvent for ChooseMany<V> {
    fn handle_event(&self, state: &mut Self::State, event: KeyEvent) -> EventOutcome {
        // Check bindings first.
        if KeyBindings::matches(&state.bindings.cancel, &event) {
            return EventOutcome::Cancelled;
        }
        if KeyBindings::matches(&state.bindings.submit, &event) {
            return submit(state);
        }
        if KeyBindings::matches(&state.bindings.toggle, &event) {
            toggle_hover(state);
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.up, &event) {
            move_hover(state, -1);
            return EventOutcome::Consumed;
        }
        if KeyBindings::matches(&state.bindings.down, &event) {
            move_hover(state, 1);
            return EventOutcome::Consumed;
        }

        // Home/End/vim-style jumps.
        if event.modifiers.is_empty() {
            match event.code {
                KeyCode::Home | KeyCode::Char('g') => {
                    jump_to(
                        state,
                        first_enabled_index(&state.input.options).unwrap_or(0),
                    );
                    return EventOutcome::Consumed;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    jump_to(
                        state,
                        last_enabled_index(&state.input.options)
                            .unwrap_or(state.input.options.len().saturating_sub(1)),
                    );
                    return EventOutcome::Consumed;
                }
                KeyCode::Char(c) => {
                    // Hotkey matching.
                    if let Some(&idx) = state.hotkeys.get(&c.to_ascii_lowercase()) {
                        jump_to(state, idx);
                        toggle_at(state, idx);
                        return EventOutcome::Consumed;
                    }
                }
                _ => {}
            }
        }

        EventOutcome::Ignored
    }
}

fn submit<V: Clone + PartialEq>(state: &mut ChooseManyState<V>) -> EventOutcome {
    let count = state.selected_count();
    if state.input.required && count == 0 {
        state.validation_error = Some("Please make a selection".into());
        return EventOutcome::Consumed;
    }
    if let Some(min) = state.input.min_selections
        && count < min
    {
        state.validation_error = Some(format!("Please select at least {min}"));
        return EventOutcome::Consumed;
    }
    EventOutcome::Submitted
}

fn toggle_hover<V: Clone + PartialEq>(state: &mut ChooseManyState<V>) {
    let idx = state.hover;
    toggle_at(state, idx);
}

fn toggle_at<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, idx: usize) {
    let Some(option) = state.input.options.get(idx) else {
        return;
    };
    if option.disabled {
        return;
    }
    let currently_selected = state.selected[idx];
    if !currently_selected
        && let Some(max) = state.input.max_selections
        && state.selected_count() >= max
    {
        // Keystroke-time rejection: silently drop toggle-on.
        return;
    }
    state.selected[idx] = !currently_selected;
    state.validation_error = None;
}

fn move_hover<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, delta: i32) {
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

fn jump_to<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, idx: usize) {
    if state.input.options.get(idx).is_some_and(|o| !o.disabled) {
        state.hover = idx;
    }
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

fn draw_list<V: Clone + PartialEq>(area: Rect, buf: &mut Buffer, state: &mut ChooseManyState<V>) {
    use unicode_width::UnicodeWidthStr;

    if area.width == 0 || area.height == 0 || state.input.options.is_empty() {
        return;
    }

    let body_rows = if state.validation_error.is_some() && area.height > 1 {
        area.height - 1
    } else {
        area.height
    };
    let visible = body_rows as usize;

    adjust_scroll(state, visible);

    let hover_style = state.theme.selected_style;
    let disabled_style = state.theme.disabled_style;

    // Compute focus prefix width: indicator + 1 space, or collapse to single space if empty/whitespace
    let focus_prefix_width = if state.theme.focus_indicator.trim().is_empty() {
        1
    } else {
        state.theme.focus_indicator.width() + 1
    };

    for (row, idx) in (state.scroll_offset..state.input.options.len())
        .take(visible)
        .enumerate()
    {
        let option = &state.input.options[idx];
        let indicator = if state.selected[idx] {
            CHECKED_GLYPH
        } else {
            UNCHECKED_GLYPH
        };

        // Focus indicator prefix on hovered row, blank padding otherwise
        let focus_prefix = if idx == state.hover {
            if state.theme.focus_indicator.trim().is_empty() {
                " ".to_string()
            } else {
                format!("{} ", state.theme.focus_indicator)
            }
        } else {
            " ".repeat(focus_prefix_width)
        };

        let prefix = format!("{focus_prefix}{indicator} ");
        let label_style = if option.disabled {
            disabled_style
        } else if idx == state.hover {
            hover_style
        } else if state.selected[idx] {
            state.theme.selected_label_style
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

    // Paint overflow indicators at top-right / bottom-right when scrollable
    let overflow_style = state
        .theme
        .selected_style
        .add_modifier(ratatui::style::Modifier::DIM);

    if state.scroll_offset > 0 && area.width > 0 {
        // Top overflow indicator
        let x = area.x + area.width - 1;
        let y = area.y;
        buf[(x, y)]
            .set_symbol(&state.theme.overflow_up_indicator)
            .set_style(overflow_style);
    }

    if state.scroll_offset + visible < state.input.options.len() && area.width > 0 && visible > 0 {
        // Bottom overflow indicator
        let x = area.x + area.width - 1;
        let y = area.y + (visible - 1) as u16;
        buf[(x, y)]
            .set_symbol(&state.theme.overflow_down_indicator)
            .set_style(overflow_style);
    }
}

fn adjust_scroll<V: Clone + PartialEq>(state: &mut ChooseManyState<V>, visible: usize) {
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
    use crossterm::event::{KeyCode, KeyModifiers};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn fixture_input() -> ChoiceInput<String> {
        ChoiceInput::new("toppings", "Pick toppings").with_options(vec![
            ChoiceOption::new("p", "Pepperoni", "pepperoni"),
            ChoiceOption::new("m", "Mushrooms", "mushrooms"),
            ChoiceOption::new("o", "Olives", "olives"),
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
    fn new_starts_with_no_selections() {
        let state = ChooseManyState::new(fixture_input());
        assert_eq!(state.selected_count(), 0);
        assert_eq!(state.hover(), Some(0));
    }

    #[test]
    fn space_toggles_selection_on_and_off() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(state.is_selected(0));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(!state.is_selected(0));
    }

    #[test]
    fn hotkey_toggles_matching_option() {
        let mut state = ChooseManyState::new(fixture_input());
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('m')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.is_selected(1));
        assert_eq!(state.hover(), Some(1));
    }

    #[test]
    fn max_selections_blocks_additional_toggle_on() {
        let input = fixture_input().with_max_selections(1);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert_eq!(state.selected_count(), 1);
        assert!(state.is_selected(0));
        assert!(!state.is_selected(1));
    }

    #[test]
    fn max_selections_allows_toggle_off_when_at_cap() {
        let input = fixture_input().with_max_selections(1);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn enter_with_no_selection_on_required_input_sets_error() {
        let input = fixture_input().required();
        let mut state = ChooseManyState::new(input);
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(<ChooseManyState as ValidationState>::validation_error(&state).is_some());
    }

    #[test]
    fn enter_with_fewer_than_min_selections_sets_error() {
        let input = fixture_input().with_min_selections(2);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        let message = <ChooseManyState as ValidationState>::validation_error(&state).unwrap();
        assert!(message.contains('2'));
    }

    #[test]
    fn enter_with_min_selections_met_submits() {
        let input = fixture_input().with_min_selections(1);
        let mut state = ChooseManyState::new(input);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Submitted);
    }

    #[test]
    fn esc_cancels() {
        let mut state = ChooseManyState::new(fixture_input());
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Cancelled);
    }

    #[test]
    fn disabled_options_cannot_be_toggled() {
        let input = ChoiceInput::<String>::new("x", "P").with_options(vec![
            ChoiceOption::new("a", "Alpha", "alpha"),
            ChoiceOption::new("b", "Beta", "beta").disabled(),
        ]);
        let mut state = ChooseManyState::new(input);
        state.hover = 1;
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert!(!state.is_selected(1));
    }

    #[test]
    fn selected_values_returns_value_fields_in_order() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let values = state.selected_values();
        assert_eq!(
            values,
            vec![&"pepperoni".to_string(), &"olives".to_string()]
        );
    }

    #[test]
    fn render_draws_checkbox_glyphs_per_row() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        // Focus prefix on hovered row 0, blank padding on others
        assert!(buffer_row(&buf, 0).starts_with("▶ ☑ Pepperoni"));
        assert!(buffer_row(&buf, 1).starts_with("  ☐ Mushrooms"));
        assert!(buffer_row(&buf, 2).starts_with("  ☐ Olives"));
    }

    #[test]
    fn render_with_label_above_draws_label_then_list() {
        let mut state = ChooseManyState::new(fixture_input())
            .with_label(Label::new("Toppings", LabelPosition::Above));
        let area = Rect::new(0, 0, 20, 4);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 0), "Toppings");
        // First option has focus prefix (hover defaults to 0)
        assert!(buffer_row(&buf, 1).starts_with("▶ ☐ Pepperoni"));
    }

    #[test]
    fn render_draws_validation_error_on_row_below_list() {
        let input = fixture_input().required();
        let mut state = ChooseManyState::new(input);
        state.set_validation_error("Please make a selection");
        let area = Rect::new(0, 0, 30, 5);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);
        assert_eq!(buffer_row(&buf, 3), "Please make a selection");
    }

    #[test]
    fn end_key_jumps_hover_to_last_enabled_option() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::End));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn vim_capital_g_jumps_hover_to_last_enabled_option() {
        let mut state = ChooseManyState::new(fixture_input());
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('G')));
        assert_eq!(state.hover(), Some(2));
    }

    #[test]
    fn initial_selection_pre_selects_listed_ids() {
        let state = ChooseManyState::new(fixture_input()).with_initial_selection(&["p", "o"]);
        assert!(state.is_selected(0));
        assert!(!state.is_selected(1));
        assert!(state.is_selected(2));
    }

    #[test]
    fn custom_submit_binding_overrides_default() {
        let bindings = KeyBindings {
            submit: vec![KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        // Ctrl-S should submit.
        let outcome = ChooseMany::new().handle_event(
            &mut state,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        );
        assert_eq!(outcome, EventOutcome::Submitted);

        // Enter should be ignored (no longer bound to submit).
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn custom_cancel_binding_works() {
        let bindings = KeyBindings {
            cancel: vec![press(KeyCode::Char('q'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

        // q should cancel.
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('q')));
        assert_eq!(outcome, EventOutcome::Cancelled);

        // Esc should be ignored.
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Esc));
        assert_eq!(outcome, EventOutcome::Ignored);
    }

    #[test]
    fn custom_toggle_binding_works() {
        let bindings = KeyBindings {
            toggle: vec![press(KeyCode::Char('t'))],
            ..KeyBindings::default()
        };
        let mut state = ChooseManyState::new(fixture_input()).with_key_bindings(bindings);

        // t should toggle.
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char('t')));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(state.is_selected(0));

        // Space should be ignored (no longer bound to toggle).
        let outcome = ChooseMany::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        assert_eq!(outcome, EventOutcome::Ignored);
        assert!(state.is_selected(0));
    }

    #[test]
    fn typed_value_u32_flow() {
        // Build from strings, project to u32.
        let options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("one", "One", "1"),
            ChoiceOption::new("two", "Two", "2"),
            ChoiceOption::new("three", "Three", "3"),
        ];
        let typed_options: Vec<ChoiceOption<u32>> = options
            .into_iter()
            .map(|opt| opt.map_value(|v| v.parse::<u32>().unwrap()))
            .collect();
        let input: ChoiceInput<u32> =
            ChoiceInput::new("nums", "Pick numbers").with_options(typed_options);
        let mut state: ChooseManyState<u32> = ChooseManyState::new(input);

        // Select first and third options.
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<u32>::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        assert_eq!(state.selected_values(), vec![&1_u32, &3_u32]);
        assert_eq!(state.value(), vec![1_u32, 3_u32]);
    }

    #[test]
    fn typed_value_enum_flow() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum Topping {
            Pepperoni,
            Mushrooms,
            Olives,
        }
        let options: Vec<ChoiceOption<Topping>> = vec![
            ChoiceOption::new("p", "Pepperoni", Topping::Pepperoni),
            ChoiceOption::new("m", "Mushrooms", Topping::Mushrooms),
            ChoiceOption::new("o", "Olives", Topping::Olives),
        ];
        let input: ChoiceInput<Topping> =
            ChoiceInput::new("toppings", "Pick toppings").with_options(options);
        let mut state: ChooseManyState<Topping> = ChooseManyState::new(input);

        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Char(' ')));
        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::<Topping>::new().handle_event(&mut state, press(KeyCode::Char(' ')));

        assert_eq!(
            state.selected_values(),
            vec![&Topping::Pepperoni, &Topping::Olives]
        );
        assert_eq!(state.value(), vec![Topping::Pepperoni, Topping::Olives]);
    }

    #[test]
    fn render_overflow_indicators_when_scrolled() {
        // 6 options, 3 rows tall, hover on option 3 (scrolls to show 1-3)
        let options: Vec<ChoiceOption<String>> = (0..6)
            .map(|i| ChoiceOption::new(format!("id{i}"), format!("Option {i}"), format!("val{i}")))
            .collect();
        let input = ChoiceInput::new("test", "Test").with_options(options);
        let mut state = ChooseManyState::new(input);

        // Move hover to option 3
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        ChooseMany::new().handle_event(&mut state, press(KeyCode::Down));
        assert_eq!(state.hover, 3);

        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);

        // scroll_offset will be 1 (hover=3, visible=3, offset = 3+1-3 = 1), showing options 1,2,3
        // Top row should contain overflow up indicator (option 0 is hidden)
        let top_row = buffer_row(&buf, 0);
        assert!(
            top_row.contains("▲"),
            "Expected ▲ in top row, got: {top_row}"
        );

        // Bottom row should contain overflow down indicator (options 4,5 are hidden)
        let bottom_row = buffer_row(&buf, 2);
        assert!(
            bottom_row.contains("▼"),
            "Expected ▼ in bottom row, got: {bottom_row}"
        );

        // Bottom row (option 3, which is hovered) should have focus prefix
        let bottom_visible_row = buffer_row(&buf, 2);
        assert!(
            bottom_visible_row.starts_with("▶ "),
            "Expected focus prefix on hovered row: {bottom_visible_row}"
        );
        assert!(
            bottom_visible_row.contains("Option 3"),
            "Expected hovered option 3 in bottom visible row: {bottom_visible_row}"
        );
    }

    #[test]
    fn render_disabled_row_with_dim_style() {
        let mut options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("a", "Active", "active".to_string()),
            ChoiceOption::new("d", "Disabled", "disabled".to_string()),
        ];
        options[1].disabled = true;
        let input = ChoiceInput::new("test", "Test").with_options(options);
        let mut state = ChooseManyState::new(input);

        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);
        ChooseMany::new().render(area, &mut buf, &mut state);

        // Row 1 is disabled; check that the label span has disabled style
        let y = 1;
        let mut found_disabled = false;
        for x in 0..area.width {
            let cell = &buf[(area.x + x, area.y + y)];
            if cell.symbol() == "D" {
                // First char of "Disabled"
                assert!(
                    cell.style().fg == Some(ratatui::style::Color::DarkGray)
                        || cell
                            .style()
                            .add_modifier
                            .contains(ratatui::style::Modifier::DIM),
                    "Disabled row label should have DarkGray or DIM style"
                );
                found_disabled = true;
                break;
            }
        }
        assert!(
            found_disabled,
            "Did not find disabled label 'Disabled' in buffer"
        );
    }
}
