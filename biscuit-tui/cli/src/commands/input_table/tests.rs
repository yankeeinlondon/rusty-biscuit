use super::*;

#[test]
fn parse_columns_accepts_minimal_static_text() {
    let json = r#"[{"type":"static-text","text":"Name"}]"#;
    let specs = parse_columns(json).unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].id(), "col_0");
}

#[test]
fn parse_columns_rejects_non_array() {
    let err = parse_columns("{}").unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn parse_columns_rejects_unknown_type() {
    let err = parse_columns(r#"[{"type":"frobnicate"}]"#).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn parse_columns_assigns_default_id_from_index() {
    let json = r#"[{"type":"text-input"},{"type":"boolean-switch"}]"#;
    let specs = parse_columns(json).unwrap();
    assert_eq!(specs[0].id(), "col_0");
    assert_eq!(specs[1].id(), "col_1");
}

#[test]
fn parse_columns_preserves_explicit_id() {
    let json = r#"[{"type":"text-input","id":"name"}]"#;
    let specs = parse_columns(json).unwrap();
    assert_eq!(specs[0].id(), "name");
}

#[test]
fn parse_columns_text_input_reads_max_length() {
    let json = r#"[{"type":"text-input","max_length":10}]"#;
    let specs = parse_columns(json).unwrap();
    match &specs[0] {
        ColumnSpec::TextInput { config, .. } => assert_eq!(config.max_length, Some(10)),
        _ => panic!("expected TextInput"),
    }
}

#[test]
fn parse_columns_choose_one_accepts_string_options() {
    let json = r#"[{"type":"choose-one","options":["A","B"],"required":true}]"#;
    let specs = parse_columns(json).unwrap();
    match &specs[0] {
        ColumnSpec::ChooseOne { input, .. } => {
            assert_eq!(input.options.len(), 2);
            assert!(input.required);
        }
        _ => panic!("expected ChooseOne"),
    }
}

#[test]
fn parse_columns_choose_many_accepts_object_options_with_id_value() {
    let json = r#"[{"type":"choose-many","options":[{"label":"Alpha","id":"a","value":"alpha"},{"label":"Beta"}]}]"#;
    let specs = parse_columns(json).unwrap();
    match &specs[0] {
        ColumnSpec::ChooseMany { input, .. } => {
            assert_eq!(input.options.len(), 2);
            assert_eq!(input.options[0].id, "a");
            assert_eq!(input.options[0].value, "alpha");
            assert_eq!(input.options[1].id, "Beta");
        }
        _ => panic!("expected ChooseMany"),
    }
}

#[test]
fn parse_rows_typed_parses_boolean_truthy() {
    let columns = vec![
        ColumnSpec::BooleanSwitch {
            id: "a".into(),
            config: BooleanSwitchConfig::default(),
        },
        ColumnSpec::BooleanSwitch {
            id: "b".into(),
            config: BooleanSwitchConfig::default(),
        },
    ];
    let rows = parse_rows_typed(&columns, Some(r#"[[true, "yes"]]"#)).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("a"), Some(&CellValue::Boolean(true)));
    assert_eq!(rows[0].get("b"), Some(&CellValue::Boolean(true)));
}

#[test]
fn parse_rows_typed_parses_chosen_many_from_array() {
    let columns = vec![ColumnSpec::ChooseMany {
        id: "choices".into(),
        input: ChoiceInput::new("c", "p").with_options(vec![
            ChoiceOption::new("a", "A", "alpha"),
            ChoiceOption::new("b", "B", "beta"),
        ]),
    }];
    let rows = parse_rows_typed(&columns, Some(r#"[[["a","b"]]]"#)).unwrap();
    assert_eq!(
        rows[0].get("choices"),
        Some(&CellValue::ChosenMany(vec!["a".into(), "b".into()]))
    );
}

#[test]
fn parse_rows_typed_parses_chosen_many_from_comma_string() {
    let columns = vec![ColumnSpec::ChooseMany {
        id: "choices".into(),
        input: ChoiceInput::new("c", "p").with_options(vec![]),
    }];
    let rows = parse_rows_typed(&columns, Some(r#"[["a,b"]]"#)).unwrap();
    assert_eq!(
        rows[0].get("choices"),
        Some(&CellValue::ChosenMany(vec!["a".into(), "b".into()]))
    );
}

#[test]
fn write_matrix_raw_emits_array_of_row_objects() {
    use biscuit_tui::RowCell;

    let rows = vec![Row {
        cells: vec![
            RowCell {
                column_id: "name".into(),
                value: CellValue::Text("alice".into()),
            },
            RowCell {
                column_id: "active".into(),
                value: CellValue::Boolean(true),
            },
        ],
    }];
    let mut buf = Vec::new();
    write_matrix(&mut buf, &rows, OutputMode::Raw).unwrap();
    let text = String::from_utf8(buf).unwrap();
    assert!(text.contains("\"name\":\"alice\""), "got {text}");
    assert!(text.contains("\"active\":true"), "got {text}");
    assert!(text.ends_with('\n'));
}

#[test]
fn write_matrix_null_emits_key_equals_value_nul_separated() {
    use biscuit_tui::RowCell;

    let rows = vec![Row {
        cells: vec![RowCell {
            column_id: "name".into(),
            value: CellValue::Text("alice".into()),
        }],
    }];
    let mut buf = Vec::new();
    write_matrix(&mut buf, &rows, OutputMode::Null).unwrap();
    assert_eq!(buf, b"name=alice\0");
}

#[test]
fn write_matrix_boolean_emits_json_bool_not_string() {
    use biscuit_tui::RowCell;

    let rows = vec![Row {
        cells: vec![RowCell {
            column_id: "active".into(),
            value: CellValue::Boolean(true),
        }],
    }];
    let mut buf = Vec::new();
    write_matrix(&mut buf, &rows, OutputMode::Json).unwrap();
    let text = String::from_utf8(buf).unwrap();
    // Should be JSON `true`, not `"true"`
    assert!(text.contains("\"active\":true"), "got {text}");
}

#[test]
fn write_matrix_chosen_many_emits_json_array() {
    use biscuit_tui::RowCell;

    let rows = vec![Row {
        cells: vec![RowCell {
            column_id: "choices".into(),
            value: CellValue::ChosenMany(vec!["a".into(), "b".into()]),
        }],
    }];
    let mut buf = Vec::new();
    write_matrix(&mut buf, &rows, OutputMode::Json).unwrap();
    let text = String::from_utf8(buf).unwrap();
    // Should be JSON array, not `"a,b"`
    assert!(text.contains("[\"a\",\"b\"]"), "got {text}");
}

#[test]
fn run_writes_typed_row_json_from_initial_rows() {
    let args = InputTableArgs {
        columns: r#"[{"type":"text-input","id":"name"},{"type":"boolean-switch","id":"active"},{"type":"choose-many","id":"tags","options":["alpha","beta"]}]"#.into(),
        rows: Some(r#"[["alice", true, ["alpha","beta"]]]"#.into()),
    };
    let mut output = Vec::new();

    let status = run_with_writer(
        args,
        OutputMode::Raw,
        Some(HeightSpec::Cells(8)),
        &mut output,
        |state, height| {
            assert_eq!(height, Some(HeightSpec::Cells(8)));
            let rows = state.value().to_vec();
            assert_eq!(rows[0].get_text("name"), Some("alice"));
            assert_eq!(rows[0].get_boolean("active"), Some(true));
            assert_eq!(
                rows[0].get_chosen_many("tags"),
                Some(&["alpha".to_string(), "beta".to_string()][..])
            );
            Ok(rows)
        },
    )
    .unwrap();

    assert_eq!(status, 0);
    let rendered: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        rendered,
        json!([{ "name": "alice", "active": true, "tags": ["alpha", "beta"] }])
    );
}

#[test]
fn run_returns_130_without_output_on_ctrl_c() {
    let args = InputTableArgs {
        columns: r#"[{"type":"text-input","id":"name"}]"#.into(),
        rows: None,
    };
    let mut output = Vec::new();

    let status = run_with_writer(
        args,
        OutputMode::Json,
        None,
        &mut output,
        |_state, _height| Err(io::Error::new(CANCELLED_KIND, "interrupted")),
    )
    .unwrap();

    assert_eq!(status, 130);
    assert!(output.is_empty());
}

#[test]
fn run_returns_1_without_output_on_esc() {
    let args = InputTableArgs {
        columns: r#"[{"type":"text-input","id":"name"}]"#.into(),
        rows: None,
    };
    let mut output = Vec::new();

    let status = run_with_writer(
        args,
        OutputMode::Json,
        None,
        &mut output,
        |_state, _height| Err(io::Error::new(ABORTED_KIND, "cancelled")),
    )
    .unwrap();

    assert_eq!(status, 1);
    assert!(output.is_empty());
}

#[test]
fn parse_columns_rejects_oversized_preferred_width() {
    let json = r#"[{"type":"text-area-input","preferred_width":99999}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("preferred_width"), "got {msg}");
    assert!(msg.contains("overflows u16"), "got {msg}");
}

#[test]
fn parse_columns_rejects_oversized_preferred_height() {
    let json = r#"[{"type":"text-area-input","preferred_height":99999}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("preferred_height"), "got {msg}");
    assert!(msg.contains("overflows u16"), "got {msg}");
}

#[test]
fn parse_columns_rejects_oversized_max_length() {
    // usize matches u64 on 64-bit hosts, so the practical overflow case is a
    // non-integer numeric (fractional or negative) — exercise that surface.
    let json = r#"[{"type":"text-input","max_length":-1}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("max_length"), "got {msg}");
    assert!(msg.contains("non-negative integer"), "got {msg}");
}

#[test]
fn parse_columns_rejects_string_preferred_width() {
    let json = r#"[{"type":"text-area-input","preferred_width":"wide"}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("preferred_width"), "got {msg}");
    assert!(msg.contains("expected number"), "got {msg}");
}

#[test]
fn parse_columns_rejects_wrong_type_initial_on_text_input() {
    let json = r#"[{"type":"text-input","initial":42}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("initial"), "got {msg}");
    assert!(msg.contains("expected string"), "got {msg}");
}

#[test]
fn parse_columns_rejects_wrong_type_required_on_choice() {
    let json = r#"[{"type":"choose-one","options":["A"],"required":"yes"}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("required"), "got {msg}");
    assert!(msg.contains("expected boolean"), "got {msg}");
}

#[test]
fn parse_columns_rejects_wrong_type_scrollbar_on_text_area() {
    let json = r#"[{"type":"text-area-input","scrollbar":"false"}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("scrollbar"), "got {msg}");
    assert!(msg.contains("expected boolean"), "got {msg}");
}

#[test]
fn parse_columns_rejects_non_string_element_in_text_area_initial() {
    let json = r#"[{"type":"text-area-input","initial":["a",42]}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("initial"), "got {msg}");
    assert!(msg.contains("must be string"), "got {msg}");
}

#[test]
fn parse_columns_rejects_wrong_type_initial_on_text_area() {
    let json = r#"[{"type":"text-area-input","initial":"single-string"}]"#;
    let err = parse_columns(json).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("initial"), "got {msg}");
    assert!(msg.contains("expected array of strings"), "got {msg}");
}

#[test]
fn parse_rows_typed_rejects_non_string_for_text_input_cell() {
    let columns = vec![ColumnSpec::TextInput {
        id: "name".into(),
        config: TextInputConfig::default(),
    }];
    let err = parse_rows_typed(&columns, Some(r#"[[42]]"#)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("row 0 column 'name'"), "got {msg}");
    assert!(msg.contains("expected string"), "got {msg}");
}

#[test]
fn parse_rows_typed_rejects_invalid_boolean_string() {
    let columns = vec![ColumnSpec::BooleanSwitch {
        id: "active".into(),
        config: BooleanSwitchConfig::default(),
    }];
    let err = parse_rows_typed(&columns, Some(r#"[["maybe"]]"#)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("row 0 column 'active'"), "got {msg}");
    assert!(msg.contains("expected"), "got {msg}");
}

#[test]
fn parse_rows_typed_rejects_object_for_boolean_cell() {
    let columns = vec![ColumnSpec::BooleanSwitch {
        id: "active".into(),
        config: BooleanSwitchConfig::default(),
    }];
    let err = parse_rows_typed(&columns, Some(r#"[[{}]]"#)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("row 0 column 'active'"), "got {msg}");
}

#[test]
fn parse_rows_typed_rejects_number_for_choose_one_cell() {
    let columns = vec![ColumnSpec::ChooseOne {
        id: "c".into(),
        input: ChoiceInput::new("c", "p").with_options(vec![ChoiceOption::new("a", "A", "alpha")]),
    }];
    let err = parse_rows_typed(&columns, Some(r#"[[5]]"#)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("row 0 column 'c'"), "got {msg}");
    assert!(msg.contains("expected string"), "got {msg}");
}

#[test]
fn parse_rows_typed_rejects_number_for_text_area_cell() {
    let columns = vec![ColumnSpec::TextAreaInput {
        id: "notes".into(),
        config: TextAreaInputConfig::default(),
    }];
    let err = parse_rows_typed(&columns, Some(r#"[[5]]"#)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("row 0 column 'notes'"), "got {msg}");
}

#[test]
fn parse_rows_typed_rejects_object_for_choose_many_cell() {
    let columns = vec![ColumnSpec::ChooseMany {
        id: "tags".into(),
        input: ChoiceInput::new("tags", "p").with_options(vec![]),
    }];
    let err = parse_rows_typed(&columns, Some(r#"[[{}]]"#)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("row 0 column 'tags'"), "got {msg}");
}

#[test]
fn parse_rows_typed_includes_row_index_in_length_mismatch() {
    let columns = vec![
        ColumnSpec::TextInput {
            id: "a".into(),
            config: TextInputConfig::default(),
        },
        ColumnSpec::TextInput {
            id: "b".into(),
            config: TextInputConfig::default(),
        },
    ];
    // First row valid (2 cells); second row under-length so the row index
    // in the message is 1, not 0.
    let err = parse_rows_typed(&columns, Some(r#"[["a","b"],["c"]]"#)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("row 1"), "got {msg}");
    assert!(msg.contains("1 cells, expected 2"), "got {msg}");
}

#[test]
fn run_with_writer_surfaces_try_new_failure_as_invalid_input() {
    // Build a schema where two columns share the same id. parse_columns does
    // not reject duplicate ids (the lib is the source of truth for that), so
    // the resulting row carries the same id twice and try_new rejects it via
    // InputTableError::DuplicateColumnId — surfaced as InvalidInput.
    let args = InputTableArgs {
        columns: r#"[{"type":"text-input","id":"dup"},{"type":"text-input","id":"dup"}]"#.into(),
        rows: Some(r#"[["a","b"]]"#.into()),
    };
    let mut output = Vec::new();
    let err = run_with_writer(
        args,
        OutputMode::Raw,
        None,
        &mut output,
        |_state, _height| unreachable!("validation should reject before prompt runs"),
    )
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    let msg = err.to_string();
    assert!(msg.contains("duplicate column id"), "got {msg}");
    assert!(msg.contains("'dup'"), "got {msg}");
}

#[test]
fn run_with_writer_accepts_documented_permissive_paths() {
    let args = InputTableArgs {
        columns: r#"[{"type":"boolean-switch","id":"a"},{"type":"text-area-input","id":"b"},{"type":"choose-many","id":"c","options":["x","y"]}]"#.into(),
        rows: Some(r#"[["on", "line1\nline2", "x, y"]]"#.into()),
    };
    let mut output = Vec::new();
    let status = run_with_writer(
        args,
        OutputMode::Raw,
        None,
        &mut output,
        |state, _height| {
            let rows = state.value().to_vec();
            assert_eq!(rows[0].get_boolean("a"), Some(true));
            assert_eq!(
                rows[0].get("b"),
                Some(&CellValue::TextArea(vec!["line1".into(), "line2".into()]))
            );
            assert_eq!(
                rows[0].get("c"),
                Some(&CellValue::ChosenMany(vec!["x".into(), "y".into()]))
            );
            Ok(rows)
        },
    )
    .unwrap();
    assert_eq!(status, 0);
}
