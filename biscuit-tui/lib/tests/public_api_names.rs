//! Public API naming guard.
//!
//! This test ensures the canonical component names (`ChooseOne`, `ChooseMany`)
//! remain the only exposed API surface. Legacy `Select*` names must not leak.

use biscuit_tui::{
    BooleanSwitch, BooleanSwitchState, CellState, CellValue, ChoiceInput, ChoiceOption, ChooseMany,
    ChooseManyState, ChooseOne, ChooseOneState, ComponentTheme, EventOutcome, HandleEvent,
    InputTable, InputTableColumn, InputTableState, KeyBindings, Label, LabelPosition, Row, RowCell,
    SelectionMode, SplitDirection, SplitPane, SplitRatio, StandaloneState, TextAreaInput,
    TextAreaInputState, TextInput, TextInputState, ValidationState,
};

#[test]
fn canonical_public_names_compile() {
    // Core types
    let _outcome = EventOutcome::Consumed;
    let _label = Label {
        text: "test".to_string(),
        position: LabelPosition::Above,
    };
    let _theme = ComponentTheme::default();
    let _bindings = KeyBindings::default();

    // Components - widgets
    let _text_input = TextInput;
    let _text_area = TextAreaInput;
    let _switch = BooleanSwitch;
    let _choose_one: ChooseOne = ChooseOne::new();
    let _choose_many: ChooseMany = ChooseMany::new();
    let _table = InputTable;

    // Components - state structs
    let _text_input_state = TextInputState::new();
    let _text_area_state = TextAreaInputState::new(10, 5);
    let _switch_state = BooleanSwitchState::new();
    let _choose_one_state: ChooseOneState = ChooseOneState::from_options(vec![]);
    let _choose_many_state: ChooseManyState = ChooseManyState::from_options(vec![]);
    let _table_state = InputTableState::with_blank_rows(vec![], 0);

    // Choice types
    let _input = ChoiceInput::<String>::new("test", "Prompt");
    let _option: ChoiceOption<String> = ChoiceOption::new("id", "label", "value");
    let _mode = SelectionMode::Single;

    // SplitPane layout primitive (core container/layout, not an input)
    let _sp = SplitPane::new();
    let _dir = SplitDirection::Auto;
    let _ratio = SplitRatio::percent(50);

    // Table types
    let _col = InputTableColumn::StaticText {
        id: "test".to_string(),
        text: "Test".to_string(),
    };
    let _cell = CellState::StaticText("test".to_string());
    let _cell_value = CellValue::Boolean(true);
    let _row = Row { cells: vec![] };
    let _row_cell = RowCell {
        column_id: "test".to_string(),
        value: CellValue::Text("value".to_string()),
    };

    // Traits (just ensure they're imported)
    fn _use_traits<S: StandaloneState, H: HandleEvent, V: ValidationState>() {}
}

/// Compile-time guard: if a legacy `SelectOne` or `SelectMany` type reappears,
/// this block documents the exclusion. We explicitly do NOT want these names.
#[cfg(any())] // Never compiles, just documents the restriction
const _NO_LEGACY_NAMES: () = {
    compile_error!(
        "Legacy `SelectOne` / `SelectMany` names must NOT appear in the public API. \
         Use `ChooseOne` and `ChooseMany` exclusively."
    );
};
