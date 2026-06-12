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
