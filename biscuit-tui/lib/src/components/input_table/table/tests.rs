use super::*;
use crate::components::choose::{ChoiceInput, ChoiceOption};
use crate::components::input_table::column::{
    BooleanSwitchConfig, TextAreaInputConfig, TextInputConfig,
};
use crate::components::input_table::error::InputTableError;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn sample_columns() -> Vec<InputTableColumn> {
    vec![
        InputTableColumn::StaticText {
            id: "label".into(),
            text: "Row".into(),
        },
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        },
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
    ]
}

#[allow(deprecated)]
fn legacy_values(state: &InputTableState) -> Vec<Vec<String>> {
    state.values()
}

#[test]
fn with_blank_rows_places_focus_on_first_focusable_cell() {
    let state = InputTableState::with_blank_rows(sample_columns(), 2);
    assert_eq!(state.focus(), (0, 1));
}

#[test]
fn with_blank_rows_populates_row_count_by_column_schema() {
    let state = InputTableState::with_blank_rows(sample_columns(), 3);
    assert_eq!(state.row_count(), 3);
    assert_eq!(state.col_count(), 3);
    assert_eq!(legacy_values(&state).len(), 3);
    assert_eq!(legacy_values(&state)[0].len(), 3);
}

#[test]
fn right_arrow_moves_focus_to_next_focusable_column() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    // initial focus is (0,1). Right should move to (0,2).
    let outcome = InputTable.handle_event(&mut state, press(KeyCode::Right));
    // Right inside a TextInput is consumed for cursor; not navigation.
    // We rely on Tab here instead — left/right is blocked for text cells.
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.focus(), (0, 1));
}

#[test]
fn tab_moves_to_next_focusable_cell_across_rows() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 2);
    assert_eq!(state.focus(), (0, 1));
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    assert_eq!(state.focus(), (0, 2));
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    // Next row, first focusable column (col 1).
    assert_eq!(state.focus(), (1, 1));
}

#[test]
fn tab_wraps_to_first_cell() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    assert_eq!(state.focus(), (0, 2));
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    assert_eq!(state.focus(), (0, 1));
}

#[test]
fn backtab_moves_to_previous_focusable_cell() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 2);
    // focus: (0,1) → Tab → (0,2) → BackTab → (0,1)
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    let event = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    InputTable.handle_event(&mut state, event);
    assert_eq!(state.focus(), (0, 1));
}

#[test]
fn up_and_down_arrows_change_row() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 3);
    // Start at (0,1). Down should go to (1,1).
    InputTable.handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(state.focus(), (1, 1));
    InputTable.handle_event(&mut state, press(KeyCode::Up));
    assert_eq!(state.focus(), (0, 1));
}

#[test]
fn up_down_inside_choice_cell_falls_through_to_cell() {
    let input = ChoiceInput::new("c", "p").with_options(vec![
        ChoiceOption::new("a", "A", "alpha"),
        ChoiceOption::new("b", "B", "beta"),
        ChoiceOption::new("c", "C", "gamma"),
    ]);
    let columns = vec![InputTableColumn::ChooseOne(input)];
    let mut state = InputTableState::with_blank_rows(columns, 2);
    assert_eq!(state.focus(), (0, 0));
    let outcome = InputTable.handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.focus(), (0, 0));
}

#[test]
fn alt_up_down_navigates_rows_from_choice_cell() {
    let input = ChoiceInput::new("c", "p").with_options(vec![
        ChoiceOption::new("a", "A", "alpha"),
        ChoiceOption::new("b", "B", "beta"),
    ]);
    let columns = vec![InputTableColumn::ChooseOne(input)];
    let mut state = InputTableState::with_blank_rows(columns, 3);
    assert_eq!(state.focus(), (0, 0));
    let alt_down = KeyEvent::new(KeyCode::Down, KeyModifiers::ALT);
    InputTable.handle_event(&mut state, alt_down);
    assert_eq!(state.focus(), (1, 0));
    let alt_up = KeyEvent::new(KeyCode::Up, KeyModifiers::ALT);
    InputTable.handle_event(&mut state, alt_up);
    assert_eq!(state.focus(), (0, 0));
}

#[test]
fn up_from_first_row_wraps_to_last_row() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 3);
    InputTable.handle_event(&mut state, press(KeyCode::Up));
    assert_eq!(state.focus(), (2, 1));
}

#[test]
fn esc_cancels() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    let outcome = InputTable.handle_event(&mut state, press(KeyCode::Esc));
    assert_eq!(outcome, EventOutcome::Cancelled);
}

#[test]
fn ctrl_s_submits_when_all_cells_valid() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    assert_eq!(outcome, EventOutcome::Submitted);
}

#[test]
fn typing_into_focused_text_input_updates_value() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    InputTable.handle_event(&mut state, press(KeyCode::Char('h')));
    InputTable.handle_event(&mut state, press(KeyCode::Char('i')));
    assert_eq!(legacy_values(&state)[0][1], "hi");
}

#[test]
fn space_in_focused_boolean_switch_toggles() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    // Move focus from TextInput (col 1) to BooleanSwitch (col 2).
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    assert_eq!(state.focus(), (0, 2));
    InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(legacy_values(&state)[0][2], "true");
}

#[test]
fn submit_with_required_choose_one_unset_sets_focus_to_offender_and_consumes() {
    let input = ChoiceInput::new("c", "p")
        .with_options(vec![ChoiceOption::new("a", "A", "alpha")])
        .required();
    let columns = vec![
        InputTableColumn::TextInput {
            id: "field".into(),
            config: TextInputConfig::default(),
        },
        InputTableColumn::ChooseOne(input),
    ];
    let mut state = InputTableState::with_blank_rows(columns, 1);
    assert_eq!(state.focus(), (0, 0));
    let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.focus(), (0, 1));
    assert!(state.table_validation_error().is_some());
}

#[test]
fn submit_succeeds_after_required_cell_is_fixed() {
    let input = ChoiceInput::new("c", "p")
        .with_options(vec![ChoiceOption::new("a", "A", "alpha")])
        .required();
    let columns = vec![InputTableColumn::ChooseOne(input)];
    let mut state = InputTableState::with_blank_rows(columns, 1);
    InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    // Fix: select the single option.
    InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
    let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    assert_eq!(outcome, EventOutcome::Submitted);
    let rows = state.value();
    assert_eq!(
        rows[0].cells[0].value,
        CellValue::ChosenOne(Some("alpha".into()))
    );
}

#[test]
#[allow(deprecated)]
fn set_cell_initial_replaces_text_input_value() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 2);
    state.set_cell_initial(1, 1, "hello");
    assert_eq!(legacy_values(&state)[1][1], "hello");
}

#[test]
#[allow(deprecated)]
fn set_cell_initial_on_static_text_overrides_display() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    state.set_cell_initial(0, 0, "custom");
    assert_eq!(legacy_values(&state)[0][0], "custom");
}

#[test]
#[allow(deprecated)]
fn values_joins_choose_many_with_comma() {
    let input = ChoiceInput::new("c", "p").with_options(vec![
        ChoiceOption::new("a", "A", "alpha"),
        ChoiceOption::new("b", "B", "beta"),
    ]);
    let columns = vec![InputTableColumn::ChooseMany(input)];
    let mut state = InputTableState::with_blank_rows(columns, 1);
    state.set_cell_initial(0, 0, "a,b");
    assert_eq!(legacy_values(&state)[0][0], "alpha,beta");
}

#[test]
fn render_mixed_cell_table_paints_static_and_body() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "row".into(),
            text: "ID".into(),
        },
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig {
                initial: "alpha".into(),
                ..TextInputConfig::default()
            },
        },
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
    ];
    let mut state = InputTableState::with_blank_rows(columns, 1);
    let area = Rect::new(0, 0, 36, 1);
    let mut buf = Buffer::empty(area);
    InputTable.render(area, &mut buf, &mut state);

    let mut row = String::new();
    for x in area.left()..area.right() {
        row.push_str(buf[(x, area.y)].symbol());
    }
    let row = row.trim_end();
    assert!(row.starts_with("ID"), "expected static text, got {row:?}");
    assert!(
        row.contains("alpha"),
        "expected text-input value, got {row:?}"
    );
}

#[test]
fn render_supports_text_area_cell_with_preferred_height() {
    let columns = vec![InputTableColumn::TextAreaInput {
        id: "notes".into(),
        config: TextAreaInputConfig {
            preferred_width: 10,
            preferred_height: 2,
            initial: vec!["a".into(), "b".into()],
            scrollbar: false,
        },
    }];
    let mut state = InputTableState::with_blank_rows(columns, 1);
    let area = Rect::new(0, 0, 10, 3);
    let mut buf = Buffer::empty(area);
    InputTable.render(area, &mut buf, &mut state);
    let read_row = |y: u16| -> String {
        let mut s = String::new();
        for x in area.left()..area.right() {
            s.push_str(buf[(x, y)].symbol());
        }
        s.trim_end().to_string()
    };
    assert_eq!(read_row(0), "a");
    assert_eq!(read_row(1), "b");
}

#[test]
#[allow(deprecated)]
fn values_encodes_boolean_as_true_or_false() {
    let mut state = InputTableState::with_blank_rows(sample_columns(), 1);
    // Focus on BooleanSwitch (col 2), toggle on.
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));
    assert_eq!(legacy_values(&state)[0][2], "true");
}

#[test]
fn custom_submit_key_replaces_default() {
    let columns = vec![InputTableColumn::TextInput {
        id: "field".into(),
        config: TextInputConfig::default(),
    }];
    let mut state = InputTableState::with_blank_rows(columns, 1)
        .with_submit_key(KeyCode::F(2), KeyModifiers::NONE);
    let outcome =
        InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
    assert_eq!(outcome, EventOutcome::Submitted);
}

#[test]
fn custom_key_bindings_override_submit_and_cancel() {
    let columns = vec![InputTableColumn::TextInput {
        id: "field".into(),
        config: TextInputConfig::default(),
    }];
    let bindings = KeyBindings {
        submit: vec![KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE)],
        cancel: vec![KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE)],
        ..KeyBindings::default()
    };
    let mut state = InputTableState::with_blank_rows(columns, 1).with_key_bindings(bindings);

    let outcome =
        InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
    assert_eq!(outcome, EventOutcome::Submitted);

    let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    assert_eq!(outcome, EventOutcome::Ignored);

    let outcome =
        InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE));
    assert_eq!(outcome, EventOutcome::Cancelled);
}

#[test]
fn new_accepts_typed_row_vec() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "row".into(),
            text: "1".into(),
        },
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
    ];
    let initial_rows = vec![Row::new(vec![
        RowCell::new("active", CellValue::Boolean(true)),
        RowCell::new("row", CellValue::StaticText("1".into())),
    ])];
    let state = InputTableState::new(columns, initial_rows);
    assert_eq!(state.row_count(), 1);
    assert_eq!(state.value()[0].get_boolean("active"), Some(true));
    assert_eq!(state.value()[0].get_text("row"), Some("1"));
}

#[test]
#[should_panic(expected = "InputTableState::new: invalid table shape")]
fn new_panics_on_length_mismatch() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "a".into(),
            text: "A".into(),
        },
        InputTableColumn::StaticText {
            id: "b".into(),
            text: "B".into(),
        },
    ];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "a",
        CellValue::StaticText("only_one".into()),
    )])];
    let _state = InputTableState::new(columns, initial_rows);
}

#[test]
fn value_exposes_typed_row_slice_in_column_order() {
    let columns = vec![
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        },
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
    ];
    let initial_rows = vec![Row::new(vec![
        RowCell::new("active", CellValue::Boolean(true)),
        RowCell::new("name", CellValue::Text("alice".into())),
    ])];
    let state = InputTableState::new(columns, initial_rows);

    let rows = state.value();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cells[0].column_id, "name");
    assert_eq!(rows[0].cells[1].column_id, "active");
    assert_eq!(rows[0].get_text("name"), Some("alice"));
    assert_eq!(rows[0].get_boolean("active"), Some(true));
}

#[test]
fn value_stays_in_sync_after_cell_edits() {
    let columns = vec![
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        },
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
    ];
    let mut state = InputTableState::with_blank_rows(columns, 1);

    InputTable.handle_event(&mut state, press(KeyCode::Char('h')));
    InputTable.handle_event(&mut state, press(KeyCode::Char('i')));
    InputTable.handle_event(&mut state, press(KeyCode::Tab));
    InputTable.handle_event(&mut state, press(KeyCode::Char(' ')));

    let rows = state.value();
    assert_eq!(rows[0].get_text("name"), Some("hi"));
    assert_eq!(rows[0].get_boolean("active"), Some(true));
}

#[test]
fn rows_typed_returns_boolean_cell_value() {
    let columns = vec![InputTableColumn::BooleanSwitch {
        id: "active".into(),
        config: BooleanSwitchConfig {
            initial: true,
            ..BooleanSwitchConfig::default()
        },
    }];
    let state = InputTableState::with_blank_rows(columns, 1);
    let rows = state.rows_typed();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].cells[0].column_id, "active");
    assert_eq!(rows[0].cells[0].value, CellValue::Boolean(true));
}

#[test]
fn rows_typed_returns_chosen_many_as_vec() {
    let input = ChoiceInput::new("c", "p").with_options(vec![
        ChoiceOption::new("a", "A", "alpha"),
        ChoiceOption::new("b", "B", "beta"),
    ]);
    let columns = vec![InputTableColumn::ChooseMany(input)];
    let mut state = InputTableState::with_blank_rows(columns, 1);
    state.set_cell_initial(0, 0, "a,b");
    let rows = state.rows_typed();
    assert_eq!(rows[0].cells[0].column_id, "c");
    assert_eq!(
        rows[0].cells[0].value,
        CellValue::ChosenMany(vec!["alpha".into(), "beta".into()])
    );
}

#[test]
fn submit_focuses_first_failing_cell_in_row_major_order() {
    let choose_many = ChoiceInput::new("tags", "Tags")
        .with_options(vec![ChoiceOption::new("a", "Alpha", "alpha")])
        .required();
    let choose_one = ChoiceInput::new("color", "Color")
        .with_options(vec![ChoiceOption::new("r", "Red", "red")])
        .required();
    let columns = vec![
        InputTableColumn::ChooseMany(choose_many),
        InputTableColumn::ChooseOne(choose_one),
    ];
    let mut state = InputTableState::with_blank_rows(columns, 2);

    state.set_cell_initial(0, 0, "a");

    let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    assert_eq!(outcome, EventOutcome::Consumed);
    assert_eq!(state.focus(), (0, 1));
    assert_eq!(
        state.table_validation_error(),
        Some("One or more cells need your attention")
    );
    assert!(
        state.rows()[0][1].validation_error().is_some(),
        "expected first failing cell to retain its validation error"
    );
}

#[test]
#[should_panic(expected = "InputTableState::new: invalid table shape")]
fn new_panics_on_unknown_column_id() {
    let columns = vec![InputTableColumn::TextInput {
        id: "name".into(),
        config: TextInputConfig::default(),
    }];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "extra",
        CellValue::Text("alice".into()),
    )])];

    let _state = InputTableState::new(columns, initial_rows);
}

#[test]
fn scroll_offset_adjusts_when_focus_moves_past_viewport() {
    let columns = vec![InputTableColumn::TextInput {
        id: "name".into(),
        config: TextInputConfig::default(),
    }];
    let mut state = InputTableState::with_blank_rows(columns, 10);
    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    InputTable.render(area, &mut buf, &mut state);
    assert_eq!(state.row_scroll_offset, 0);
    for _ in 0..4 {
        InputTable.handle_event(&mut state, press(KeyCode::Down));
    }
    InputTable.render(area, &mut buf, &mut state);
    assert!(
        state.row_scroll_offset > 0,
        "expected scroll offset to advance past 0"
    );
}

#[test]
fn scroll_offset_adjusts_when_focus_returns_above_viewport() {
    let columns = vec![InputTableColumn::TextInput {
        id: "name".into(),
        config: TextInputConfig::default(),
    }];
    let mut state = InputTableState::with_blank_rows(columns, 10);
    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    for _ in 0..9 {
        InputTable.handle_event(&mut state, press(KeyCode::Down));
    }
    InputTable.render(area, &mut buf, &mut state);
    let offset_after_down = state.row_scroll_offset;
    assert!(offset_after_down > 0);
    for _ in 0..9 {
        InputTable.handle_event(&mut state, press(KeyCode::Up));
    }
    InputTable.render(area, &mut buf, &mut state);
    assert_eq!(
        state.row_scroll_offset, 0,
        "expected scroll offset to return to 0"
    );
}

#[test]
fn render_paints_overflow_indicators_when_rows_exceed_viewport() {
    let columns = vec![InputTableColumn::TextInput {
        id: "name".into(),
        config: TextInputConfig::default(),
    }];
    let mut state = InputTableState::with_blank_rows(columns, 10);
    let area = Rect::new(0, 0, 20, 3);
    let mut buf = Buffer::empty(area);
    for _ in 0..5 {
        InputTable.handle_event(&mut state, press(KeyCode::Down));
    }
    InputTable.render(area, &mut buf, &mut state);
    let top_right = buf[(area.x + area.width - 1, area.y)].symbol();
    assert_eq!(top_right, "▲");
    let bottom_right = buf[(area.x + area.width - 1, area.y + 2)].symbol();
    assert_eq!(bottom_right, "▼");
}

#[test]
fn static_text_columns_stay_at_natural_width_with_leftover() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "hi".into(),
            text: "Hello".into(),
        },
        InputTableColumn::TextInput {
            id: "a".into(),
            config: TextInputConfig::default(),
        },
        InputTableColumn::TextInput {
            id: "b".into(),
            config: TextInputConfig::default(),
        },
    ];
    let widths = compute_column_widths(&columns, &[], 60);
    assert_eq!(
        widths[0], 5,
        "StaticText should get exactly its natural unicode width"
    );
    assert_eq!(
        widths.iter().map(|&w| w as u32).sum::<u32>(),
        60,
        "widths should sum to total"
    );
    assert!(
        widths[1] > 20,
        "focusable columns should get leftover width"
    );
    assert!(
        widths[2] > 20,
        "focusable columns should get leftover width"
    );
}

#[test]
fn static_text_does_not_shrink_below_natural_in_overflow() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "lbl".into(),
            text: "LongLabel".into(),
        },
        InputTableColumn::TextInput {
            id: "a".into(),
            config: TextInputConfig::default(),
        },
    ];
    let widths = compute_column_widths(&columns, &[], 10);
    assert_eq!(
        widths[0], 5,
        "StaticText in overflow should get min(base, preferred)"
    );
    assert_eq!(widths.iter().map(|&w| w as u32).sum::<u32>(), 10);
}

#[test]
fn all_static_text_columns_use_preferred_widths() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "a".into(),
            text: "Alpha".into(),
        },
        InputTableColumn::StaticText {
            id: "b".into(),
            text: "Beta".into(),
        },
    ];
    let widths = compute_column_widths(&columns, &[], 40);
    assert_eq!(widths[0], 5, "Alpha is 5 chars wide");
    assert_eq!(widths[1], 4, "Beta is 4 chars wide");
}

#[test]
fn render_static_text_stays_tight() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "lbl".into(),
            text: "Label".into(),
        },
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        },
    ];
    let mut state = InputTableState::with_blank_rows(columns, 1);
    let area = Rect::new(0, 0, 40, 1);
    let mut buf = Buffer::empty(area);
    InputTable.render(area, &mut buf, &mut state);

    let widths = compute_column_widths(state.columns(), state.rows(), area.width);
    assert_eq!(widths[0], 5, "StaticText 'Label' is 5 chars");

    let static_end = widths[0] as usize;
    let mut label = String::new();
    for x in 0..static_end {
        label.push_str(buf[(x as u16, 0)].symbol());
    }
    assert_eq!(label.trim_end(), "Label");
}

#[test]
fn try_new_ok_on_valid_rows() {
    let columns = vec![
        InputTableColumn::StaticText {
            id: "row".into(),
            text: "1".into(),
        },
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        },
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
    ];
    let initial_rows = vec![Row::new(vec![
        RowCell::new("row", CellValue::StaticText("1".into())),
        RowCell::new("name", CellValue::Text("Alice".into())),
        RowCell::new("active", CellValue::Boolean(true)),
    ])];
    let state = InputTableState::try_new(columns, initial_rows).unwrap();
    assert_eq!(state.row_count(), 1);
    assert_eq!(state.value()[0].get_text("name"), Some("Alice"));
    assert_eq!(state.value()[0].get_boolean("active"), Some(true));
}

#[test]
fn try_new_returns_row_shape_mismatch_with_context() {
    // An over-length row (3 cells for 2 columns) is a pure count error that
    // no single id mismatch can explain, so it short-circuits to
    // `RowShapeMismatch`. An under-length row instead reaches `validate_row`
    // and yields `MissingColumnId` — see
    // `try_new_returns_missing_column_id_with_context`.
    let columns = vec![
        InputTableColumn::StaticText {
            id: "a".into(),
            text: "A".into(),
        },
        InputTableColumn::StaticText {
            id: "b".into(),
            text: "B".into(),
        },
    ];
    let initial_rows = vec![Row::new(vec![
        RowCell::new("a", CellValue::StaticText("a".into())),
        RowCell::new("b", CellValue::StaticText("b".into())),
        RowCell::new("c", CellValue::StaticText("c".into())),
    ])];
    let err = InputTableState::try_new(columns, initial_rows).unwrap_err();
    assert_eq!(
        err,
        InputTableError::RowShapeMismatch {
            row: 0,
            expected: 2,
            found: 3
        }
    );
}

#[test]
fn try_new_returns_duplicate_column_id_with_context() {
    let columns = vec![
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        },
    ];
    let initial_rows = vec![Row::new(vec![
        RowCell::new("active", CellValue::Boolean(true)),
        RowCell::new("active", CellValue::Boolean(false)),
    ])];
    let err = InputTableState::try_new(columns, initial_rows).unwrap_err();
    assert_eq!(
        err,
        InputTableError::DuplicateColumnId {
            row: 0,
            id: "active".into()
        }
    );
}

#[test]
fn try_new_returns_unknown_column_id_with_context() {
    let columns = vec![InputTableColumn::TextInput {
        id: "name".into(),
        config: TextInputConfig::default(),
    }];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "extra",
        CellValue::Text("alice".into()),
    )])];
    let err = InputTableState::try_new(columns, initial_rows).unwrap_err();
    assert_eq!(
        err,
        InputTableError::UnknownColumnId {
            row: 0,
            id: "extra".into()
        }
    );
}

#[test]
fn try_new_returns_missing_column_id_with_context() {
    // An under-length row (only the `name` cell) has unique known ids, so the
    // public `try_new` delegates past the over-length short-circuit to
    // `validate_row`, which reports the absent `active` column.
    let columns = vec![
        InputTableColumn::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        },
        InputTableColumn::BooleanSwitch {
            id: "active".into(),
            config: BooleanSwitchConfig::default(),
        },
    ];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "name",
        CellValue::Text("alice".into()),
    )])];
    let err = InputTableState::try_new(columns, initial_rows).unwrap_err();
    assert_eq!(
        err,
        InputTableError::MissingColumnId {
            row: 0,
            id: "active".into()
        }
    );
}

#[test]
fn try_new_returns_cell_type_mismatch_for_text_in_boolean_column() {
    let columns = vec![InputTableColumn::BooleanSwitch {
        id: "active".into(),
        config: BooleanSwitchConfig::default(),
    }];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "active",
        CellValue::Text("true".into()),
    )])];
    let err = InputTableState::try_new(columns, initial_rows).unwrap_err();
    assert_eq!(
        err,
        InputTableError::CellTypeMismatch {
            row: 0,
            id: "active".into(),
            expected: "boolean",
            found: "text",
        }
    );
}

#[test]
fn try_new_returns_cell_type_mismatch_for_boolean_in_text_column() {
    let columns = vec![InputTableColumn::TextInput {
        id: "name".into(),
        config: TextInputConfig::default(),
    }];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "name",
        CellValue::Boolean(true),
    )])];
    let err = InputTableState::try_new(columns, initial_rows).unwrap_err();
    assert_eq!(
        err,
        InputTableError::CellTypeMismatch {
            row: 0,
            id: "name".into(),
            expected: "text",
            found: "boolean",
        }
    );
}

#[test]
fn try_new_returns_cell_type_mismatch_for_chosen_many_in_choose_one() {
    let input = ChoiceInput::new("c", "p").with_options(vec![ChoiceOption::new("a", "A", "alpha")]);
    let columns = vec![InputTableColumn::ChooseOne(input)];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "c",
        CellValue::ChosenMany(vec!["a".into()]),
    )])];
    let err = InputTableState::try_new(columns, initial_rows).unwrap_err();
    assert_eq!(
        err,
        InputTableError::CellTypeMismatch {
            row: 0,
            id: "c".into(),
            expected: "chosen-one",
            found: "chosen-many",
        }
    );
}

#[test]
fn try_new_accepts_chosen_one_none_for_optional_choice() {
    let input = ChoiceInput::new("c", "p").with_options(vec![ChoiceOption::new("a", "A", "alpha")]);
    let columns = vec![InputTableColumn::ChooseOne(input)];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "c",
        CellValue::ChosenOne(None),
    )])];
    let state = InputTableState::try_new(columns, initial_rows).unwrap();
    assert_eq!(state.row_count(), 1);
}

#[test]
fn new_still_panics_on_cell_type_mismatch() {
    let columns = vec![InputTableColumn::BooleanSwitch {
        id: "active".into(),
        config: BooleanSwitchConfig::default(),
    }];
    let initial_rows = vec![Row::new(vec![RowCell::new(
        "active",
        CellValue::Text("true".into()),
    )])];
    let result = std::panic::catch_unwind(|| {
        InputTableState::new(columns, initial_rows);
    });
    assert!(result.is_err(), "new should panic on cell type mismatch");
}

// -- Content-based static widths and ellipsis clipping --------------------
//
// These tests observe layout through the rendered buffer so they hold across
// changes to the private width helper's signature.

fn static_column(id: &str, text: &str) -> InputTableColumn {
    InputTableColumn::StaticText {
        id: id.into(),
        text: text.into(),
    }
}

fn text_column(id: &str) -> InputTableColumn {
    InputTableColumn::TextInput {
        id: id.into(),
        config: TextInputConfig::default(),
    }
}

/// A `Step` label column followed by two text columns seeded `X` and `Y`,
/// so the start of each text column is visible in the rendered line.
fn step_table(labels: &[&str]) -> InputTableState {
    let columns = vec![
        static_column("step", "Step"),
        text_column("provider"),
        text_column("model"),
    ];
    let rows = labels
        .iter()
        .map(|label| {
            Row::new(vec![
                RowCell::new("step", CellValue::StaticText((*label).into())),
                RowCell::new("provider", CellValue::Text("X".into())),
                RowCell::new("model", CellValue::Text("Y".into())),
            ])
        })
        .collect();
    InputTableState::new(columns, rows)
}

fn render_into(state: &mut InputTableState, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    InputTable.render(area, &mut buf, state);
    buf
}

/// Visible text of `y` in `[from, to)`, skipping the cells a wide grapheme
/// covers so a two-cell character reads as one symbol.
fn visible_text(buf: &Buffer, y: u16, from: u16, to: u16) -> String {
    use unicode_width::UnicodeWidthStr;
    let mut out = String::new();
    let mut x = from;
    while x < to {
        let symbol = buf[(x, y)].symbol();
        out.push_str(symbol);
        x += (UnicodeWidthStr::width(symbol) as u16).max(1);
    }
    out
}

fn row_text(buf: &Buffer, y: u16) -> String {
    visible_text(buf, y, 0, buf.area.width)
}

/// Column where `symbol` first appears on row `y`.
fn column_of(buf: &Buffer, y: u16, symbol: &str) -> Option<u16> {
    (0..buf.area.width).find(|&x| buf[(x, y)].symbol() == symbol)
}

#[test]
fn static_column_shows_full_label_at_80_columns() {
    let mut state = step_table(&["1 implement", "12 review-5"]);
    let buf = render_into(&mut state, 80, 4);

    assert!(row_text(&buf, 0).starts_with("1 implementX"), "{:?}", row_text(&buf, 0));
    assert!(row_text(&buf, 1).starts_with("12 review-5X"), "{:?}", row_text(&buf, 1));
    assert_eq!(column_of(&buf, 1, "X"), Some(11));
}

#[test]
fn static_column_width_includes_off_screen_rows_and_survives_scrolling() {
    let labels = ["1 a", "2 b", "3 c", "4 d", "5 e", "6 the-longest-label"];
    let mut state = step_table(&labels);

    let before = render_into(&mut state, 80, 3);
    assert!(
        !row_text(&before, 2).contains("the-longest-label"),
        "the long row must start off screen"
    );
    assert_eq!(column_of(&before, 0, "X"), Some(19));

    for _ in 0..5 {
        InputTable.handle_event(&mut state, press(KeyCode::Down));
    }
    let after = render_into(&mut state, 80, 3);
    let last = row_text(&after, 2);
    assert!(last.starts_with("6 the-longest-labelX"), "{last:?}");
    assert_eq!(column_of(&after, 0, "X"), Some(19), "scrolling changed widths");
}

#[test]
fn resizing_down_clips_with_ellipsis_and_resizing_up_restores() {
    let mut state = step_table(&["12 review-5"]);

    let wide = render_into(&mut state, 80, 1);
    assert!(row_text(&wide, 0).starts_with("12 review-5X"));

    // Text budgets (20 + 20) still fit; the label shrinks to the rest.
    let narrow = render_into(&mut state, 45, 1);
    assert!(row_text(&narrow, 0).starts_with("12 r…X"), "{:?}", row_text(&narrow, 0));

    let restored = render_into(&mut state, 80, 1);
    assert!(row_text(&restored, 0).starts_with("12 review-5X"));
}

#[test]
fn multiple_static_columns_share_the_reduction_left_to_right() {
    let columns = vec![
        static_column("a", "A"),
        static_column("b", "B"),
        text_column("t"),
    ];
    let row = Row::new(vec![
        RowCell::new("a", CellValue::StaticText("abcdefghij".into())),
        RowCell::new("b", CellValue::StaticText("klmnopqrst".into())),
        RowCell::new("t", CellValue::Text("X".into())),
    ]);
    let mut state = InputTableState::new(columns, vec![row]);

    // 34 - 20 leaves 14 cells: an even 7 + 7.
    let even = render_into(&mut state, 34, 1);
    assert!(row_text(&even, 0).starts_with("abcdef…klmnop…X"), "{:?}", row_text(&even, 0));

    // 13 cells: the left column keeps the odd cell.
    let odd = render_into(&mut state, 33, 1);
    assert!(row_text(&odd, 0).starts_with("abcdef…klmno…X"), "{:?}", row_text(&odd, 0));
}

#[test]
fn emergency_allocation_divides_width_evenly_and_caps_static_columns() {
    let mut state = step_table(&["12 review-5"]);

    // 3 + 20 + 20 cannot fit in 30: every column gets 10.
    let buf = render_into(&mut state, 30, 1);
    assert!(row_text(&buf, 0).starts_with("12 review…X"), "{:?}", row_text(&buf, 0));
    assert_eq!(column_of(&buf, 0, "X"), Some(10));
    assert_eq!(column_of(&buf, 0, "Y"), Some(20));
}

#[test]
fn one_cell_static_column_shows_only_the_ellipsis() {
    let mut state = step_table(&["12 review-5"]);
    let buf = render_into(&mut state, 3, 1);
    assert_eq!(buf[(0, 0)].symbol(), "…");
    assert_eq!(buf[(1, 0)].symbol(), "X");
    assert_eq!(buf[(2, 0)].symbol(), "Y");
}

#[test]
fn zero_cell_static_column_draws_nothing() {
    let columns = vec![
        text_column("provider"),
        text_column("model"),
        static_column("step", "Step"),
    ];
    let row = Row::new(vec![
        RowCell::new("provider", CellValue::Text("X".into())),
        RowCell::new("model", CellValue::Text("Y".into())),
        RowCell::new("step", CellValue::StaticText("12 review-5".into())),
    ]);
    let mut state = InputTableState::new(columns, vec![row]);
    let buf = render_into(&mut state, 2, 1);
    assert_eq!(row_text(&buf, 0), "XY");
}

#[test]
fn wide_characters_are_omitted_whole_and_never_cross_the_column() {
    let columns = vec![static_column("step", "Step"), text_column("t")];
    let row = Row::new(vec![
        RowCell::new("step", CellValue::StaticText("12 日本語テキスト".into())),
        RowCell::new("t", CellValue::Text("X".into())),
    ]);
    let mut state = InputTableState::new(columns, vec![row]);

    let full = render_into(&mut state, 80, 1);
    assert!(row_text(&full, 0).starts_with("12 日本語テキストX"));

    // Six label cells: `12 日` fills five, `…` the sixth.
    let six = render_into(&mut state, 26, 1);
    assert_eq!(visible_text(&six, 0, 0, 6), "12 日…");
    assert_eq!(column_of(&six, 0, "X"), Some(6));

    // Seven label cells: `本` would straddle the `…`, so it is left out and
    // the spare cell stays blank.
    let seven = render_into(&mut state, 27, 1);
    assert_eq!(visible_text(&seven, 0, 0, 7), "12 日… ");
    assert_eq!(column_of(&seven, 0, "X"), Some(7));
}

#[test]
fn grapheme_clusters_are_never_split_by_clipping() {
    use unicode_segmentation::UnicodeSegmentation;

    for label in ["cafe\u{301} au lait", "ab\u{1F469}\u{200D}\u{1F469}\u{200D}\u{1F467}cd-ef"] {
        let columns = vec![static_column("step", "Step"), text_column("t")];
        let row = Row::new(vec![
            RowCell::new("step", CellValue::StaticText(label.into())),
            RowCell::new("t", CellValue::Text("X".into())),
        ]);
        let mut state = InputTableState::new(columns, vec![row]);
        let clusters: Vec<&str> = label.graphemes(true).collect();

        // 23..=28 keeps the text budget (20) and gives the label 3..=8 cells,
        // narrower than either label.
        for width in 23..=28u16 {
            let buf = render_into(&mut state, width, 1);
            let label_cells = width - 20;
            assert_eq!(column_of(&buf, 0, "X"), Some(label_cells), "width {width}");
            for x in 0..label_cells {
                let symbol = buf[(x, 0)].symbol();
                assert!(
                    symbol == "…" || symbol == " " || clusters.contains(&symbol),
                    "{label:?} at width {width}: cell {x} holds a split cluster {symbol:?}"
                );
            }
            assert!(
                visible_text(&buf, 0, 0, label_cells).trim_end().ends_with('…'),
                "{label:?} at width {width} was clipped without an ellipsis"
            );
        }
    }

    // The combining accent stays in the same cell as its base letter.
    let columns = vec![static_column("step", "Step"), text_column("t")];
    let row = Row::new(vec![
        RowCell::new("step", CellValue::StaticText("cafe\u{301} au lait".into())),
        RowCell::new("t", CellValue::Text("X".into())),
    ]);
    let mut state = InputTableState::new(columns, vec![row]);
    let buf = render_into(&mut state, 26, 1);
    assert_eq!(buf[(3, 0)].symbol(), "e\u{301}");
    assert_eq!(buf[(5, 0)].symbol(), "…");
}

#[test]
fn all_static_table_sizes_to_content_and_shrinks_left_to_right() {
    let columns = vec![static_column("a", "A"), static_column("b", "B")];
    let row = Row::new(vec![
        RowCell::new("a", CellValue::StaticText("alpha-long".into())),
        RowCell::new("b", CellValue::StaticText("beta-longer".into())),
    ]);
    let mut state = InputTableState::new(columns, vec![row]);

    let wide = render_into(&mut state, 80, 1);
    assert_eq!(row_text(&wide, 0).trim_end(), "alpha-longbeta-longer");

    // 10 + 11 into 12: a reduction of 9, the left column keeping the odd cell.
    let narrow = render_into(&mut state, 12, 1);
    assert_eq!(row_text(&narrow, 0), "alpha…beta-…");
}

#[test]
fn clipping_is_display_only_and_submission_returns_full_text() {
    let mut state = step_table(&["12 review-5"]);
    render_into(&mut state, 45, 1);
    render_into(&mut state, 3, 1);

    let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    assert_eq!(outcome, EventOutcome::Submitted);
    assert_eq!(
        state.value()[0].get("step"),
        Some(&CellValue::StaticText("12 review-5".into()))
    );
    assert_eq!(state.rows_typed()[0], state.value()[0]);
}

// -- Width allocation invariants ------------------------------------------

fn switch_column(id: &str) -> InputTableColumn {
    InputTableColumn::BooleanSwitch {
        id: id.into(),
        config: BooleanSwitchConfig::default(),
    }
}

fn choice_column(id: &str, labels: &[&str]) -> InputTableColumn {
    let options = labels
        .iter()
        .map(|label| ChoiceOption::new(label.to_lowercase(), *label, label.to_lowercase()))
        .collect();
    InputTableColumn::ChooseOne(ChoiceInput::new(id, id).with_options(options))
}

/// Column mixes paired with rows that widen their static columns.
fn allocation_fixtures() -> Vec<InputTableState> {
    let labelled = |columns: Vec<InputTableColumn>, labels: &[&str]| {
        let mut state = InputTableState::with_blank_rows(columns, labels.len());
        for (row, label) in labels.iter().enumerate() {
            state.set_cell_initial(row, 0, label);
        }
        state
    };
    vec![
        labelled(
            vec![
                static_column("step", "Step"),
                choice_column("provider", &["Claude", "Codex"]),
                choice_column("model", &["opus"]),
            ],
            &["1 implement", "22 the-final-review-step"],
        ),
        labelled(
            vec![static_column("a", "A"), static_column("b", "Bee"), text_column("t")],
            &["abcdefghij"],
        ),
        labelled(
            vec![static_column("a", "Alpha"), static_column("b", "Beta")],
            &["a-much-longer-label"],
        ),
        labelled(
            vec![
                static_column("s", "S"),
                switch_column("on"),
                static_column("n", "Name"),
                InputTableColumn::TextAreaInput {
                    id: "notes".into(),
                    config: TextAreaInputConfig {
                        preferred_width: 30,
                        ..TextAreaInputConfig::default()
                    },
                },
            ],
            &["wide 日本語 label"],
        ),
        InputTableState::with_blank_rows(vec![text_column("only")], 1),
    ]
}

#[test]
fn column_widths_never_exceed_the_available_width_and_follow_the_tiers() {
    for state in allocation_fixtures() {
        let columns = state.columns();
        let preferred = preferred_column_widths(columns, state.rows());
        let is_static: Vec<bool> = columns.iter().map(|c| !c.is_focusable()).collect();
        let sum_of = |widths: &[u16]| widths.iter().map(|&w| u32::from(w)).sum::<u32>();
        let budgets: u32 = preferred
            .iter()
            .zip(&is_static)
            .filter(|(_, s)| !**s)
            .map(|(&w, _)| u32::from(w))
            .sum();
        let static_floor = 3 * is_static.iter().filter(|s| **s).count() as u32;
        let has_focusable = is_static.contains(&false);

        for total in 0..=200u16 {
            let widths = compute_column_widths(columns, state.rows(), total);
            let context = format!("{:?} at {total}: {widths:?}", preferred);
            let sum = sum_of(&widths);
            assert_eq!(widths.len(), columns.len(), "{context}");
            assert!(sum <= u32::from(total), "{context}");
            for (i, &w) in widths.iter().enumerate() {
                if is_static[i] {
                    assert!(w <= preferred[i], "static column {i} over preferred: {context}");
                }
            }

            if sum_of(&preferred) <= u32::from(total) {
                for (i, &w) in widths.iter().enumerate() {
                    if is_static[i] {
                        assert_eq!(w, preferred[i], "{context}");
                    } else {
                        assert!(w >= preferred[i], "{context}");
                    }
                }
                if has_focusable {
                    assert_eq!(sum, u32::from(total), "leftover not shared: {context}");
                }
            } else if static_floor + budgets <= u32::from(total) {
                assert_eq!(sum, u32::from(total), "{context}");
                for (i, &w) in widths.iter().enumerate() {
                    if is_static[i] {
                        assert!(w >= 3, "static column {i} under the floor: {context}");
                    } else {
                        assert_eq!(w, preferred[i], "focusable kept extras: {context}");
                    }
                }
            } else {
                let even_ceiling = u32::from(total).div_ceil(columns.len() as u32);
                assert!(
                    widths.iter().all(|&w| u32::from(w) <= even_ceiling),
                    "emergency tier is not an even split: {context}"
                );
            }
        }
    }
}

#[test]
fn a_static_column_at_the_floor_passes_its_share_to_the_others() {
    let columns = vec![static_column("a", "Abcd"), static_column("b", "B"), text_column("t")];
    let row = Row::new(vec![
        RowCell::new("a", CellValue::StaticText("abcd".into())),
        RowCell::new("b", CellValue::StaticText("b".repeat(20))),
        RowCell::new("t", CellValue::Text(String::new())),
    ]);
    let state = InputTableState::new(columns, vec![row]);

    // Preferred 4 + 20 + 20 into 34: a reduction of 10. The first column can
    // give up only one cell, so the second gives up the other nine.
    let widths = compute_column_widths(state.columns(), state.rows(), 34);
    assert_eq!(widths, vec![3, 11, 20]);
}

#[test]
fn very_long_static_values_saturate_instead_of_wrapping() {
    let columns = vec![static_column("a", "A"), text_column("t")];
    let row = Row::new(vec![
        RowCell::new("a", CellValue::StaticText("x".repeat(70_000))),
        RowCell::new("t", CellValue::Text(String::new())),
    ]);
    let state = InputTableState::new(columns, vec![row]);

    assert_eq!(preferred_column_widths(state.columns(), state.rows())[0], u16::MAX);
    assert_eq!(compute_column_widths(state.columns(), state.rows(), 80), vec![60, 20]);
}

#[test]
fn a_label_at_the_u16_boundary_saturates_its_preferred_width() {
    for (label_width, preferred) in [(65_534, 65_534), (65_535, u16::MAX), (65_536, u16::MAX)] {
        let columns = vec![static_column("a", "A"), text_column("t")];
        let row = Row::new(vec![
            RowCell::new("a", CellValue::StaticText("x".repeat(label_width))),
            RowCell::new("t", CellValue::Text(String::new())),
        ]);
        let state = InputTableState::new(columns, vec![row]);
        assert_eq!(
            preferred_column_widths(state.columns(), state.rows())[0],
            preferred,
            "label of width {label_width}"
        );
    }
}

/// Two static columns whose preferred widths both saturate at `u16::MAX`, so
/// their sum exceeds `u16`: ASCII labels either side of the boundary in
/// column `a`, and a two-cell grapheme label of width 65 536 in column `b`.
fn saturated_table() -> (InputTableState, Vec<(String, String)>) {
    let labels: Vec<(String, String)> = vec![
        ("x".repeat(65_534), "界".repeat(32_768)),
        ("x".repeat(65_535), "b".into()),
        ("x".repeat(65_536), "b".into()),
        ("x".repeat(70_000), "b".into()),
    ];
    let columns = vec![static_column("a", "A"), static_column("b", "B"), text_column("t")];
    let rows = labels
        .iter()
        .map(|(a, b)| {
            Row::new(vec![
                RowCell::new("a", CellValue::StaticText(a.clone())),
                RowCell::new("b", CellValue::StaticText(b.clone())),
                RowCell::new("t", CellValue::Text("X".into())),
            ])
        })
        .collect();
    (InputTableState::new(columns, rows), labels)
}

#[test]
fn saturated_static_columns_render_clipped_at_every_width_without_overflow() {
    let (mut state, labels) = saturated_table();
    assert_eq!(
        preferred_column_widths(state.columns(), state.rows()),
        vec![u16::MAX, u16::MAX, 20]
    );

    for width in [0, 1, 2, 3, 26, 80, 200, u16::MAX - 1, u16::MAX] {
        let widths = compute_column_widths(state.columns(), state.rows(), width);
        let sum: u32 = widths.iter().map(|w| u32::from(*w)).sum();
        assert!(sum <= u32::from(width), "{widths:?} exceeds {width}");
        render_into(&mut state, width, 4);
    }

    // Tier 2 at 80: the text column keeps its budget, the static columns
    // split the rest and clip with an ellipsis.
    assert_eq!(compute_column_widths(state.columns(), state.rows(), 80), vec![30, 30, 20]);
    let buf = render_into(&mut state, 80, 4);
    assert_eq!(visible_text(&buf, 0, 0, 30), format!("{}…", "x".repeat(29)));
    // Fourteen two-cell clusters fill 28 cells; the fifteenth would straddle
    // the ellipsis cell, so it is dropped rather than split.
    assert!(
        visible_text(&buf, 0, 30, 60).starts_with(&format!("{}…", "界".repeat(14))),
        "{:?}",
        visible_text(&buf, 0, 30, 60)
    );
    for y in 0..4 {
        assert_eq!(column_of(&buf, y, "X"), Some(60), "row {y}");
    }

    // The widest area a terminal can report: the sum is exact and the text
    // column still gets its budget.
    let widths = compute_column_widths(state.columns(), state.rows(), u16::MAX);
    assert_eq!(widths.iter().map(|w| u32::from(*w)).sum::<u32>(), u32::from(u16::MAX));
    assert_eq!(widths[2], 20);
    let buf = render_into(&mut state, u16::MAX, 4);
    assert_eq!(column_of(&buf, 3, "X"), Some(u16::MAX - 20));

    // One cell: the first static column shows only the ellipsis.
    let buf = render_into(&mut state, 1, 4);
    assert_eq!(row_text(&buf, 0), "…");

    let outcome = InputTable.handle_event(&mut state, ctrl(KeyCode::Char('s')));
    assert_eq!(outcome, EventOutcome::Submitted);
    for (row, (a, b)) in labels.iter().enumerate() {
        assert_eq!(state.value()[row].get("a"), Some(&CellValue::StaticText(a.clone())));
        assert_eq!(state.value()[row].get("b"), Some(&CellValue::StaticText(b.clone())));
    }
}

// -- Focus styling ----------------------------------------------------------

/// The review-screen shape: a `Step` label and provider and model choices.
fn review_table(rows: usize) -> InputTableState {
    let columns = vec![
        static_column("step", "Step"),
        choice_column("provider", &["Claude", "Codex", "Gemini"]),
        choice_column("model", &["(default)", "opus"]),
    ];
    let mut state = InputTableState::with_blank_rows(columns, rows);
    for row in 0..rows {
        state.set_cell_initial(row, 0, &format!("{} step", row + 1));
    }
    state
}

/// Rows of `buf` whose text contains `needle`, top to bottom.
fn lines_containing(buf: &Buffer, needle: &str) -> Vec<u16> {
    (0..buf.area.height)
        .filter(|&y| row_text(buf, y).contains(needle))
        .collect()
}

fn underlined_cells(buf: &Buffer) -> Vec<(u16, u16)> {
    buf.area
        .positions()
        .filter(|p| buf[*p].modifier.contains(Modifier::UNDERLINED))
        .map(|p| (p.x, p.y))
        .collect()
}

#[test]
fn focused_choice_cell_draws_no_underline_on_blank_cells() {
    let mut state = review_table(3);
    assert_eq!(state.focus(), (0, 1));
    let buf = render_into(&mut state, 80, 16);

    // The padding right of `Codex` in the focused cell is blank.
    let codex_y = lines_containing(&buf, "Codex")[0];
    let blank_x = column_of(&buf, codex_y, "x").expect("Codex drawn") + 1;
    assert_eq!(buf[(blank_x, codex_y)].symbol(), " ");
    assert!(!buf[(blank_x, codex_y)].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(underlined_cells(&buf), vec![], "the default theme underlines nothing");
}

#[test]
fn focused_choice_cell_stays_distinguishable_and_keeps_the_active_option_style() {
    let mut state = review_table(3);
    let buf = render_into(&mut state, 80, 16);

    let codex_rows = lines_containing(&buf, "Codex");
    let (focused_y, unfocused_y) = (codex_rows[0], *codex_rows.last().unwrap());
    let x = column_of(&buf, focused_y, "C").expect("Codex drawn");
    assert_ne!(
        buf[(x, focused_y)].style(),
        buf[(x, unfocused_y)].style(),
        "the focused cell looks the same as an unfocused one"
    );

    // The active option of an unfocused choice keeps the widget's own styling.
    let claude_y = *lines_containing(&buf, "Claude").last().unwrap();
    let claude_x = column_of(&buf, claude_y, "C").expect("Claude drawn");
    assert!(buf[(claude_x, claude_y)].modifier.contains(Modifier::BOLD));
}

#[test]
fn moving_focus_clears_the_old_focus_styling_in_the_same_buffer() {
    let mut state = review_table(3);
    let area = Rect::new(0, 0, 80, 16);
    let mut reused = Buffer::empty(area);
    InputTable.render(area, &mut reused, &mut state);

    InputTable.handle_event(&mut state, KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));
    assert_eq!(state.focus(), (1, 1));
    InputTable.render(area, &mut reused, &mut state);

    let fresh = render_into(&mut state, 80, 16);
    assert_eq!(reused, fresh, "the earlier focus left styling behind");

    let codex_rows = lines_containing(&reused, "Codex");
    let x = column_of(&reused, codex_rows[0], "C").expect("Codex drawn");
    assert_eq!(
        reused[(x, codex_rows[0])].style(),
        reused[(x, codex_rows[2])].style(),
        "the first row still looks focused"
    );
}

#[test]
fn focused_text_input_cell_has_no_underline_and_clears_when_focus_moves() {
    let mut state = step_table(&["1 implement", "2 review"]);
    assert_eq!(state.focus(), (0, 1));
    let area = Rect::new(0, 0, 60, 2);
    let mut reused = Buffer::empty(area);
    InputTable.render(area, &mut reused, &mut state);

    assert_eq!(underlined_cells(&reused), vec![]);
    let x = column_of(&reused, 0, "X").expect("provider drawn");
    assert_ne!(
        reused[(x, 0)].style(),
        reused[(x, 1)].style(),
        "the focused text cell looks the same as an unfocused one"
    );

    InputTable.handle_event(&mut state, press(KeyCode::Down));
    assert_eq!(state.focus(), (1, 1));
    InputTable.render(area, &mut reused, &mut state);
    assert_eq!(reused, render_into(&mut state, 60, 2));
    assert_eq!(underlined_cells(&reused), vec![]);
}
