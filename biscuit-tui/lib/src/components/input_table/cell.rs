//! Per-cell state enum for [`InputTable`](super::InputTable).
//!
//! A [`CellState`] wraps the state struct of one of the library's
//! primitive input components and forwards event handling and value
//! extraction to that inner state. [`CellState::StaticText`] carries a
//! plain `String` for display-only columns.

use crossterm::event::KeyEvent;

use crate::components::{
    BooleanSwitch, BooleanSwitchState, ChooseMany, ChooseManyState, ChooseOne, ChooseOneState,
    TextAreaInput, TextAreaInputState, TextInput, TextInputState,
};
use crate::core::{EventOutcome, HandleEvent, ValidationState};

use super::column::{
    BooleanSwitchConfig, InputTableColumn, TextAreaInputConfig, TextInputConfig,
};

/// Runtime state for a single cell of an [`InputTable`].
///
/// Each variant holds the corresponding component's state. Event
/// handling and rendering are delegated to the wrapped state via the
/// primitive widget (e.g. [`TextInput`](crate::components::TextInput)).
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum CellState {
    /// Display-only cell; the string renders verbatim.
    StaticText(String),
    /// Boolean toggle-switch cell.
    BooleanSwitch(BooleanSwitchState),
    /// Single-line text input cell.
    TextInput(TextInputState),
    /// Multi-line text editor cell.
    TextAreaInput(TextAreaInputState),
    /// Single-select choice cell.
    ChooseOne(ChooseOneState),
    /// Multi-select choice cell.
    ChooseMany(ChooseManyState),
}

impl CellState {
    /// Builds the initial [`CellState`] for a cell in `column`.
    ///
    /// Each primitive state is seeded from the column's configuration
    /// payload.
    pub fn from_column(column: &InputTableColumn) -> Self {
        match column {
            InputTableColumn::StaticText(text) => CellState::StaticText(text.clone()),
            InputTableColumn::BooleanSwitch(config) => CellState::BooleanSwitch(
                build_boolean_switch_state(config),
            ),
            InputTableColumn::TextInput(config) => {
                CellState::TextInput(build_text_input_state(config))
            }
            InputTableColumn::TextAreaInput(config) => {
                CellState::TextAreaInput(build_text_area_input_state(config))
            }
            InputTableColumn::ChooseOne(input) => {
                CellState::ChooseOne(ChooseOneState::new(input.clone()))
            }
            InputTableColumn::ChooseMany(input) => {
                CellState::ChooseMany(ChooseManyState::new(input.clone()))
            }
        }
    }

    /// Returns `true` when the cell can receive keyboard focus.
    pub fn is_focusable(&self) -> bool {
        !matches!(self, CellState::StaticText(_))
    }

    /// Returns the cell's preferred minimum render height in rows.
    ///
    /// Single-line cells ([`TextInput`](crate::components::TextInput),
    /// [`BooleanSwitch`](crate::components::BooleanSwitch), and
    /// [`StaticText`](Self::StaticText)) return `1`. Multi-line cells
    /// return their own natural extent.
    pub fn min_height(&self) -> u16 {
        match self {
            CellState::StaticText(_) | CellState::BooleanSwitch(_) | CellState::TextInput(_) => 1,
            CellState::TextAreaInput(state) => state.preferred_height().max(1),
            CellState::ChooseOne(state) => state.options().len().max(1) as u16,
            CellState::ChooseMany(state) => state.options().len().max(1) as u16,
        }
    }

    /// Returns the active validation error, if any.
    ///
    /// [`StaticText`](Self::StaticText) and
    /// [`BooleanSwitch`](Self::BooleanSwitch) cells never have
    /// validation state and always return `None`.
    pub fn validation_error(&self) -> Option<&str> {
        match self {
            CellState::StaticText(_) | CellState::BooleanSwitch(_) => None,
            CellState::TextInput(state) => state.validation_error(),
            CellState::TextAreaInput(state) => state.validation_error(),
            CellState::ChooseOne(state) => <ChooseOneState as ValidationState>::validation_error(state),
            CellState::ChooseMany(state) => <ChooseManyState as ValidationState>::validation_error(state),
        }
    }

    /// Returns the cell's captured value as a string.
    ///
    /// ## Notes
    ///
    /// - [`BooleanSwitch`](Self::BooleanSwitch) emits `"true"` or
    ///   `"false"`.
    /// - [`TextAreaInput`](Self::TextAreaInput) joins lines with
    ///   `\n`.
    /// - [`ChooseOne`](Self::ChooseOne) returns the selected value or
    ///   an empty string when nothing is selected.
    /// - [`ChooseMany`](Self::ChooseMany) returns selected values
    ///   joined with `,`.
    pub fn value_string(&self) -> String {
        match self {
            CellState::StaticText(text) => text.clone(),
            CellState::BooleanSwitch(state) => {
                if state.checked() { "true".into() } else { "false".into() }
            }
            CellState::TextInput(state) => state.value().to_string(),
            CellState::TextAreaInput(state) => state.value(),
            CellState::ChooseOne(state) => state
                .selected_value()
                .map(str::to_string)
                .unwrap_or_default(),
            CellState::ChooseMany(state) => state
                .selected_values()
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
                .join(","),
        }
    }

    /// Routes a key event into the cell's inner component.
    ///
    /// [`StaticText`](Self::StaticText) cells always return
    /// [`EventOutcome::Ignored`].
    pub fn handle_event(&mut self, event: KeyEvent) -> EventOutcome {
        match self {
            CellState::StaticText(_) => EventOutcome::Ignored,
            CellState::BooleanSwitch(state) => BooleanSwitch.handle_event(state, event),
            CellState::TextInput(state) => TextInput.handle_event(state, event),
            CellState::TextAreaInput(state) => TextAreaInput.handle_event(state, event),
            CellState::ChooseOne(state) => ChooseOne.handle_event(state, event),
            CellState::ChooseMany(state) => ChooseMany.handle_event(state, event),
        }
    }

    /// Overrides the cell's initial value from a string payload.
    ///
    /// Parsing is best-effort and cell-type specific:
    ///
    /// - `TextInput`, `TextAreaInput` (splits on `\n`): the raw
    ///   string is adopted as the initial buffer.
    /// - `BooleanSwitch`: `"true"`, `"yes"`, `"on"`, `"1"` →
    ///   checked; everything else → unchecked.
    /// - `ChooseOne`: the string is interpreted as an option `id` to
    ///   pre-select.
    /// - `ChooseMany`: comma-separated option `id`s to pre-select.
    /// - `StaticText`: the string replaces the display text.
    pub fn set_initial_value(&mut self, value: &str) {
        match self {
            CellState::StaticText(text) => *text = value.to_string(),
            CellState::BooleanSwitch(state) => {
                let lower = value.trim().to_ascii_lowercase();
                let checked = matches!(lower.as_str(), "true" | "yes" | "on" | "1");
                state.set_checked(checked);
            }
            CellState::TextInput(state) => {
                *state = state.clone().with_value(value);
            }
            CellState::TextAreaInput(state) => {
                let lines: Vec<&str> = value.split('\n').collect();
                *state = state.clone().with_value(&lines);
            }
            CellState::ChooseOne(state) => {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    *state = state.clone().with_initial_selection(trimmed);
                }
            }
            CellState::ChooseMany(state) => {
                let ids: Vec<&str> = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                *state = state.clone().with_initial_selection(&ids);
            }
        }
    }
}

fn build_boolean_switch_state(config: &BooleanSwitchConfig) -> BooleanSwitchState {
    BooleanSwitchState::new()
        .with_labels(config.on_label.clone(), config.off_label.clone())
        .with_value(config.initial)
}

fn build_text_input_state(config: &TextInputConfig) -> TextInputState {
    let mut state = TextInputState::new();
    if !config.initial.is_empty() {
        state = state.with_value(&config.initial);
    }
    if let Some(max) = config.max_length {
        state = state.with_max_length(max);
    }
    state
}

fn build_text_area_input_state(config: &TextAreaInputConfig) -> TextAreaInputState {
    let mut state = TextAreaInputState::new(config.preferred_width, config.preferred_height)
        .with_scrollbar(config.scrollbar);
    if !config.initial.is_empty() {
        state = state.with_value(&config.initial);
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::choose::{ChoiceInput, ChoiceOption};
    use crossterm::event::{KeyCode, KeyModifiers};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn static_text_cell_is_not_focusable_and_ignores_events() {
        let column = InputTableColumn::StaticText("Name".into());
        let mut cell = CellState::from_column(&column);
        assert!(!cell.is_focusable());
        assert_eq!(cell.value_string(), "Name");
        assert_eq!(cell.handle_event(press(KeyCode::Char('x'))), EventOutcome::Ignored);
    }

    #[test]
    fn boolean_switch_cell_reflects_config_initial() {
        let column = InputTableColumn::BooleanSwitch(BooleanSwitchConfig {
            initial: true,
            ..BooleanSwitchConfig::default()
        });
        let cell = CellState::from_column(&column);
        assert!(cell.is_focusable());
        assert_eq!(cell.value_string(), "true");
    }

    #[test]
    fn boolean_switch_cell_accepts_space_toggle() {
        let column = InputTableColumn::BooleanSwitch(BooleanSwitchConfig::default());
        let mut cell = CellState::from_column(&column);
        cell.handle_event(press(KeyCode::Char(' ')));
        assert_eq!(cell.value_string(), "true");
    }

    #[test]
    fn text_input_cell_accepts_typing() {
        let column = InputTableColumn::TextInput(TextInputConfig::default());
        let mut cell = CellState::from_column(&column);
        cell.handle_event(press(KeyCode::Char('h')));
        cell.handle_event(press(KeyCode::Char('i')));
        assert_eq!(cell.value_string(), "hi");
    }

    #[test]
    fn text_input_cell_honours_max_length() {
        let column = InputTableColumn::TextInput(TextInputConfig {
            initial: String::new(),
            max_length: Some(2),
        });
        let mut cell = CellState::from_column(&column);
        for c in "abcdef".chars() {
            cell.handle_event(press(KeyCode::Char(c)));
        }
        assert_eq!(cell.value_string(), "ab");
    }

    #[test]
    fn text_area_input_cell_returns_joined_lines() {
        let column = InputTableColumn::TextAreaInput(TextAreaInputConfig {
            initial: vec!["alpha".into(), "beta".into()],
            ..TextAreaInputConfig::default()
        });
        let cell = CellState::from_column(&column);
        assert_eq!(cell.value_string(), "alpha\nbeta");
    }

    #[test]
    fn choose_one_cell_sets_initial_selection_by_id() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
        ]);
        let column = InputTableColumn::ChooseOne(input);
        let mut cell = CellState::from_column(&column);
        cell.set_initial_value("b");
        assert_eq!(cell.value_string(), "beta");
    }

    #[test]
    fn choose_many_cell_comma_joins_selected_values() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
            ChoiceOption::new("c", "C", "gamma"),
        ]);
        let column = InputTableColumn::ChooseMany(input);
        let mut cell = CellState::from_column(&column);
        cell.set_initial_value("a, c");
        assert_eq!(cell.value_string(), "alpha,gamma");
    }

    #[test]
    fn required_choose_one_reports_validation_error_on_submit() {
        let input = ChoiceInput::new("c", "p")
            .with_options(vec![ChoiceOption::new("a", "A", "alpha")])
            .required();
        let column = InputTableColumn::ChooseOne(input);
        let mut cell = CellState::from_column(&column);
        let outcome = cell.handle_event(press(KeyCode::Enter));
        assert_eq!(outcome, EventOutcome::Consumed);
        assert!(cell.validation_error().is_some());
    }

    #[test]
    fn static_text_min_height_is_one() {
        let cell = CellState::StaticText("a".into());
        assert_eq!(cell.min_height(), 1);
    }

    #[test]
    fn choose_one_min_height_equals_option_count() {
        let input = ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "a"),
            ChoiceOption::new("b", "B", "b"),
            ChoiceOption::new("c", "C", "c"),
        ]);
        let column = InputTableColumn::ChooseOne(input);
        let cell = CellState::from_column(&column);
        assert_eq!(cell.min_height(), 3);
    }
}
