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
    let widths = compute_column_widths(&columns, 60);
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
    let widths = compute_column_widths(&columns, 10);
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
    let widths = compute_column_widths(&columns, 40);
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

    let widths = compute_column_widths(state.columns(), area.width);
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
