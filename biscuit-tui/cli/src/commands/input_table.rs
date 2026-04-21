//! `question input-table` subcommand.
//!
//! Maps CLI args onto a [`tui_chrome::InputTableState`], runs the
//! component via [`tui_chrome::run_standalone`], and writes the
//! captured row-major values as a JSON array of row objects.
//!
//! ## JSON Schemas
//!
//! `--columns` accepts a JSON array of column definitions. Each item
//! is an object with a required `type` field plus type-specific
//! configuration:
//!
//! ```json
//! [
//!   {"type": "static-text", "text": "Row"},
//!   {"type": "text-input", "id": "name", "max_length": 50},
//!   {"type": "boolean-switch", "id": "active", "initial": false},
//!   {"type": "choose-one", "id": "colour", "options": ["Red", "Green", "Blue"]},
//!   {"type": "choose-many", "id": "toppings", "options": ["A", "B"]},
//!   {"type": "text-area-input", "id": "notes", "preferred_height": 3}
//! ]
//! ```
//!
//! `--rows` is a JSON array of per-row cell arrays; each inner array
//! supplies typed per-cell initial values matching the column order.

use std::io::{self, Write};

use clap::Args;
use serde_json::{Value, json};
use tui_chrome::components::input_table::{
    BooleanSwitchConfig, CellValue, TextAreaInputConfig, TextInputConfig,
};
use tui_chrome::{
    CANCELLED_KIND, ChoiceInput, ChoiceOption, InputTable, InputTableColumn, InputTableState, Row,
    run_standalone,
};

use crate::output::OutputMode;

/// Arguments accepted by the `input-table` subcommand.
#[derive(Debug, Args)]
pub struct InputTableArgs {
    /// JSON array of column definitions.
    #[arg(long)]
    pub columns: String,

    /// JSON array of row value arrays. When omitted a
    /// single empty row is created.
    #[arg(long)]
    pub rows: Option<String>,
}

/// Runs the `input-table` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error or an invalid JSON payload.
pub fn run(args: InputTableArgs, output: OutputMode, height: Option<u16>) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone(InputTable::new(), state, height)
    })
}

fn run_with_writer<F, W>(
    args: InputTableArgs,
    output: OutputMode,
    height: Option<u16>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(InputTableState, Option<u16>) -> io::Result<Vec<Row>>,
    W: Write,
{
    let column_specs = parse_columns(&args.columns)?;
    let columns: Vec<InputTableColumn> = column_specs.iter().map(|spec| spec.to_column()).collect();
    let row_values = parse_rows_typed(&column_specs, args.rows.as_deref())?;

    let row_count = row_values.len().max(1);
    let state = if row_values.is_empty() {
        InputTableState::with_blank_rows(columns, row_count)
    } else {
        InputTableState::new(columns, row_values)
    };

    match run_prompt(state, height) {
        Ok(rows) => {
            write_matrix(writer, &rows, output)?;
            writer.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ColumnSpec {
    StaticText {
        id: String,
        text: String,
    },
    TextInput {
        id: String,
        config: TextInputConfig,
    },
    BooleanSwitch {
        id: String,
        config: BooleanSwitchConfig,
    },
    TextAreaInput {
        id: String,
        config: TextAreaInputConfig,
    },
    ChooseOne {
        id: String,
        input: ChoiceInput<String>,
    },
    ChooseMany {
        id: String,
        input: ChoiceInput<String>,
    },
}

impl ColumnSpec {
    #[allow(dead_code)]
    fn id(&self) -> &str {
        match self {
            Self::StaticText { id, .. }
            | Self::TextInput { id, .. }
            | Self::BooleanSwitch { id, .. }
            | Self::TextAreaInput { id, .. }
            | Self::ChooseOne { id, .. }
            | Self::ChooseMany { id, .. } => id,
        }
    }

    fn to_column(&self) -> InputTableColumn {
        match self.clone() {
            Self::StaticText { id, text } => InputTableColumn::StaticText { id, text },
            Self::TextInput { id, config } => InputTableColumn::TextInput { id, config },
            Self::BooleanSwitch { id, config } => InputTableColumn::BooleanSwitch { id, config },
            Self::TextAreaInput { id, config } => InputTableColumn::TextAreaInput { id, config },
            Self::ChooseOne { input, .. } => InputTableColumn::ChooseOne(input),
            Self::ChooseMany { input, .. } => InputTableColumn::ChooseMany(input),
        }
    }
}

fn parse_columns(json: &str) -> io::Result<Vec<ColumnSpec>> {
    let value: Value = serde_json::from_str(json)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("--columns: {e}")))?;
    let array = value.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "--columns must be a JSON array",
        )
    })?;
    array
        .iter()
        .enumerate()
        .map(|(i, item)| parse_column(i, item))
        .collect()
}

fn parse_column(index: usize, value: &Value) -> io::Result<ColumnSpec> {
    let object = value.as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("column {index}: expected object"),
        )
    })?;
    let kind = object.get("type").and_then(Value::as_str).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("column {index}: missing 'type' string"),
        )
    })?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let id = if id.is_empty() {
        format!("col_{index}")
    } else {
        id
    };

    match kind {
        "static-text" => {
            let text = object
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            Ok(ColumnSpec::StaticText { id, text })
        }
        "text-input" => {
            let initial = object
                .get("initial")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let max_length = object
                .get("max_length")
                .and_then(Value::as_u64)
                .map(|n| n as usize);
            Ok(ColumnSpec::TextInput {
                id,
                config: TextInputConfig {
                    initial,
                    max_length,
                },
            })
        }
        "boolean-switch" => {
            let initial = object
                .get("initial")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let on_label = object
                .get("on_label")
                .and_then(Value::as_str)
                .unwrap_or("ON")
                .to_string();
            let off_label = object
                .get("off_label")
                .and_then(Value::as_str)
                .unwrap_or("OFF")
                .to_string();
            Ok(ColumnSpec::BooleanSwitch {
                id,
                config: BooleanSwitchConfig {
                    initial,
                    on_label,
                    off_label,
                },
            })
        }
        "text-area-input" => {
            let preferred_width = object
                .get("preferred_width")
                .and_then(Value::as_u64)
                .map(|n| n as u16)
                .unwrap_or(40);
            let preferred_height = object
                .get("preferred_height")
                .and_then(Value::as_u64)
                .map(|n| n as u16)
                .unwrap_or(3);
            let initial: Vec<String> = object
                .get("initial")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let scrollbar = object
                .get("scrollbar")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Ok(ColumnSpec::TextAreaInput {
                id,
                config: TextAreaInputConfig {
                    preferred_width,
                    preferred_height,
                    initial,
                    scrollbar,
                },
            })
        }
        "choose-one" => Ok(ColumnSpec::ChooseOne {
            id: id.clone(),
            input: build_choice_input(&id, object)?,
        }),
        "choose-many" => Ok(ColumnSpec::ChooseMany {
            id: id.clone(),
            input: build_choice_input(&id, object)?,
        }),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("column {index}: unknown type '{other}'"),
        )),
    }
}

fn build_choice_input(
    id: &str,
    object: &serde_json::Map<String, Value>,
) -> io::Result<ChoiceInput<String>> {
    let options_value = object
        .get("options")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing 'options' array"))?;
    let array = options_value.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "'options' must be a JSON array",
        )
    })?;
    let options: Vec<ChoiceOption<String>> = array
        .iter()
        .map(|item| -> io::Result<ChoiceOption<String>> {
            match item {
                Value::String(label) => Ok(ChoiceOption::new(
                    label.clone(),
                    label.clone(),
                    label.clone(),
                )),
                Value::Object(map) => {
                    let label = map.get("label").and_then(Value::as_str).ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "option object missing 'label' string",
                        )
                    })?;
                    let option_id = map
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or(label)
                        .to_string();
                    let value = map
                        .get("value")
                        .and_then(Value::as_str)
                        .unwrap_or(label)
                        .to_string();
                    Ok(ChoiceOption::new(option_id, label.to_string(), value))
                }
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "option must be a string or object",
                )),
            }
        })
        .collect::<io::Result<_>>()?;

    let mut input: ChoiceInput<String> = ChoiceInput::new(id.to_string(), "").with_options(options);
    if object
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        input = input.required();
    }
    if let Some(min) = object.get("min_selections").and_then(Value::as_u64) {
        input = input.with_min_selections(min as usize);
    }
    if let Some(max) = object.get("max_selections").and_then(Value::as_u64) {
        input = input.with_max_selections(max as usize);
    }
    Ok(input)
}

/// Parses `--rows` JSON into typed `Vec<Vec<CellValue>>`.
///
/// Each inner array is interpreted column-aware: boolean columns accept
/// truthy/falsy values, choose-many accepts arrays or comma-strings, etc.
fn parse_rows_typed(columns: &[ColumnSpec], json: Option<&str>) -> io::Result<Vec<Vec<CellValue>>> {
    let Some(source) = json else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(source)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("--rows: {e}")))?;
    let array = value.as_array().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "--rows must be a JSON array")
    })?;

    array
        .iter()
        .map(|row| {
            let inner = row.as_array().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "each row must be a JSON array")
            })?;
            if inner.len() != columns.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("row has {} cells, expected {}", inner.len(), columns.len()),
                ));
            }
            Ok(inner
                .iter()
                .zip(columns.iter())
                .map(|(val, col)| parse_cell_value(val, col))
                .collect())
        })
        .collect()
}

/// Parses a single JSON cell value into the appropriate `CellValue` variant
/// based on the column type.
fn parse_cell_value(val: &Value, col: &ColumnSpec) -> CellValue {
    match col {
        ColumnSpec::StaticText { .. } => CellValue::StaticText(value_to_string(val)),
        ColumnSpec::BooleanSwitch { .. } => {
            let truthy = match val {
                Value::Bool(b) => *b,
                Value::String(s) => {
                    let lower = s.trim().to_ascii_lowercase();
                    matches!(lower.as_str(), "true" | "on" | "yes" | "1")
                }
                Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
                _ => false,
            };
            CellValue::Boolean(truthy)
        }
        ColumnSpec::TextInput { .. } => CellValue::Text(value_to_string(val)),
        ColumnSpec::TextAreaInput { .. } => {
            if let Value::Array(arr) = val {
                CellValue::TextArea(arr.iter().map(value_to_string).collect())
            } else {
                let text = value_to_string(val);
                CellValue::TextArea(text.split('\n').map(|s| s.to_string()).collect())
            }
        }
        ColumnSpec::ChooseOne { .. } => {
            let id = value_to_string(val);
            if id.is_empty() {
                CellValue::ChosenOne(None)
            } else {
                CellValue::ChosenOne(Some(id))
            }
        }
        ColumnSpec::ChooseMany { .. } => {
            if let Value::Array(arr) = val {
                CellValue::ChosenMany(arr.iter().map(value_to_string).collect())
            } else {
                let text = value_to_string(val);
                CellValue::ChosenMany(
                    text.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                )
            }
        }
    }
}

fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Writes typed rows as JSON or NUL-separated key=value pairs.
fn write_matrix<W: Write>(writer: &mut W, rows: &[Row], mode: OutputMode) -> io::Result<()> {
    match mode {
        OutputMode::Raw | OutputMode::Json => {
            let json_rows: Vec<Value> = rows
                .iter()
                .map(|row| {
                    let mut object = serde_json::Map::new();
                    for cell in &row.cells {
                        let json_value = match &cell.value {
                            CellValue::StaticText(s) | CellValue::Text(s) => json!(s),
                            CellValue::Boolean(b) => json!(b),
                            CellValue::TextArea(lines) => json!(lines.join("\n")),
                            CellValue::ChosenOne(opt) => {
                                opt.as_ref().map_or(Value::Null, |s| json!(s))
                            }
                            CellValue::ChosenMany(vec) => json!(vec),
                        };
                        object.insert(cell.column_id.clone(), json_value);
                    }
                    Value::Object(object)
                })
                .collect();
            let encoded = serde_json::to_string(&Value::Array(json_rows))
                .unwrap_or_else(|_| String::from("[]"));
            writer.write_all(encoded.as_bytes())?;
            writer.write_all(b"\n")
        }
        OutputMode::Null => {
            for row in rows {
                for cell in &row.cells {
                    match &cell.value {
                        CellValue::ChosenMany(vec) => {
                            for item in vec {
                                writer
                                    .write_all(format!("{}={}", cell.column_id, item).as_bytes())?;
                                writer.write_all(&[0])?;
                            }
                        }
                        other => {
                            let value_str = match other {
                                CellValue::StaticText(s) | CellValue::Text(s) => s.clone(),
                                CellValue::Boolean(b) => b.to_string(),
                                CellValue::TextArea(lines) => lines.join("\n"),
                                CellValue::ChosenOne(opt) => opt.clone().unwrap_or_default(),
                                CellValue::ChosenMany(_) => unreachable!(),
                            };
                            writer.write_all(
                                format!("{}={}", cell.column_id, value_str).as_bytes(),
                            )?;
                            writer.write_all(&[0])?;
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(rows[0][0], CellValue::Boolean(true));
        assert_eq!(rows[0][1], CellValue::Boolean(true));
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
            rows[0][0],
            CellValue::ChosenMany(vec!["a".into(), "b".into()])
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
            rows[0][0],
            CellValue::ChosenMany(vec!["a".into(), "b".into()])
        );
    }

    #[test]
    fn write_matrix_raw_emits_array_of_row_objects() {
        use tui_chrome::RowCell;

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
        use tui_chrome::RowCell;

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
        use tui_chrome::RowCell;

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
        use tui_chrome::RowCell;

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
            Some(8),
            &mut output,
            |state, height| {
                assert_eq!(height, Some(8));
                let rows = state.rows_typed();
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
    fn run_returns_130_without_output_on_cancel() {
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
            |_state, _height| Err(io::Error::new(CANCELLED_KIND, "cancelled")),
        )
        .unwrap();

        assert_eq!(status, 130);
        assert!(output.is_empty());
    }
}
