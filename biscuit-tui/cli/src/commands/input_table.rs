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
//! `--rows` is a JSON array of arrays of strings; each inner array
//! supplies per-cell initial values matching the column order.

use std::io::{self, Write};

use clap::Args;
use serde_json::{Value, json};
use tui_chrome::{
    CANCELLED_KIND, ChoiceInput, ChoiceOption, InputTable, InputTableColumn, InputTableState,
    run_standalone,
};
use tui_chrome::components::input_table::{
    BooleanSwitchConfig, TextAreaInputConfig, TextInputConfig,
};

use crate::output::OutputMode;

/// Arguments accepted by the `input-table` subcommand.
#[derive(Debug, Args)]
pub struct InputTableArgs {
    /// JSON array of column definitions.
    #[arg(long)]
    pub columns: String,

    /// JSON array of row value arrays (strings). When omitted a
    /// single empty row is created.
    #[arg(long)]
    pub rows: Option<String>,

    /// Render inline in `N` rows below the cursor instead of
    /// fullscreen.
    #[arg(long)]
    pub height: Option<u16>,
}

/// Runs the `input-table` subcommand.
///
/// ## Returns
///
/// `Ok(0)` on submission, `Ok(130)` on cancellation, `Err` on a
/// terminal I/O error or an invalid JSON payload.
pub fn run(args: InputTableArgs, output: OutputMode) -> io::Result<i32> {
    let column_specs = parse_columns(&args.columns)?;
    let columns: Vec<InputTableColumn> =
        column_specs.iter().map(|spec| spec.to_column()).collect();
    let row_values = parse_rows(args.rows.as_deref())?;

    let row_count = row_values.len().max(1);
    let mut state = InputTableState::new(columns, row_count);
    for (row_idx, row) in row_values.iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            state.set_cell_initial(row_idx, col_idx, value);
        }
    }

    match run_standalone(InputTable::new(), state, args.height) {
        Ok(matrix) => {
            let stdout = io::stdout();
            let mut lock = stdout.lock();
            write_matrix(&mut lock, &column_specs, &matrix, output)?;
            lock.flush()?;
            Ok(0)
        }
        Err(e) if e.kind() == CANCELLED_KIND => Ok(130),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Clone)]
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
            Self::StaticText { text, .. } => InputTableColumn::StaticText(text),
            Self::TextInput { config, .. } => InputTableColumn::TextInput(config),
            Self::BooleanSwitch { config, .. } => InputTableColumn::BooleanSwitch(config),
            Self::TextAreaInput { config, .. } => InputTableColumn::TextAreaInput(config),
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
    let kind = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| {
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
            let initial = object.get("initial").and_then(Value::as_bool).unwrap_or(false);
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
                Value::String(label) => {
                    Ok(ChoiceOption::new(label.clone(), label.clone(), label.clone()))
                }
                Value::Object(map) => {
                    let label = map
                        .get("label")
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
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

    let mut input: ChoiceInput<String> =
        ChoiceInput::new(id.to_string(), "").with_options(options);
    if object.get("required").and_then(Value::as_bool).unwrap_or(false) {
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

fn parse_rows(json: Option<&str>) -> io::Result<Vec<Vec<String>>> {
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
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "each row must be a JSON array of strings",
                )
            })?;
            Ok(inner
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                })
                .collect())
        })
        .collect()
}

fn write_matrix<W: Write>(
    writer: &mut W,
    columns: &[ColumnSpec],
    matrix: &[Vec<String>],
    mode: OutputMode,
) -> io::Result<()> {
    // InputTable always emits JSON-shaped data. `Raw`/`Json` both
    // produce pretty JSON; `Null` emits NUL-separated `k=v` tokens for
    // shell consumption.
    match mode {
        OutputMode::Raw | OutputMode::Json => {
            let rows: Vec<Value> = matrix
                .iter()
                .map(|row| {
                    let mut object = serde_json::Map::new();
                    for (col, value) in columns.iter().zip(row.iter()) {
                        object.insert(col.id().to_string(), json!(value));
                    }
                    Value::Object(object)
                })
                .collect();
            let encoded = serde_json::to_string(&Value::Array(rows))
                .unwrap_or_else(|_| String::from("[]"));
            writer.write_all(encoded.as_bytes())?;
            writer.write_all(b"\n")
        }
        OutputMode::Null => {
            for row in matrix {
                for (col, value) in columns.iter().zip(row.iter()) {
                    writer.write_all(format!("{}={}", col.id(), value).as_bytes())?;
                    writer.write_all(&[0])?;
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
    fn parse_rows_reads_2d_string_array() {
        let rows = parse_rows(Some(r#"[["a","b"],["c","d"]]"#)).unwrap();
        assert_eq!(rows, vec![vec!["a", "b"], vec!["c", "d"]]);
    }

    #[test]
    fn parse_rows_coerces_null_to_empty_string() {
        let rows = parse_rows(Some(r#"[[null,"x"]]"#)).unwrap();
        assert_eq!(rows, vec![vec!["", "x"]]);
    }

    #[test]
    fn parse_rows_without_input_returns_empty_vec() {
        let rows = parse_rows(None).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn write_matrix_raw_emits_array_of_row_objects() {
        let columns = vec![
            ColumnSpec::TextInput {
                id: "name".into(),
                config: TextInputConfig::default(),
            },
            ColumnSpec::BooleanSwitch {
                id: "active".into(),
                config: BooleanSwitchConfig::default(),
            },
        ];
        let matrix = vec![vec!["alice".to_string(), "true".to_string()]];
        let mut buf = Vec::new();
        write_matrix(&mut buf, &columns, &matrix, OutputMode::Raw).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.contains("\"name\":\"alice\""), "got {text}");
        assert!(text.contains("\"active\":\"true\""), "got {text}");
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn write_matrix_null_emits_key_equals_value_nul_separated() {
        let columns = vec![ColumnSpec::TextInput {
            id: "name".into(),
            config: TextInputConfig::default(),
        }];
        let matrix = vec![vec!["alice".to_string()]];
        let mut buf = Vec::new();
        write_matrix(&mut buf, &columns, &matrix, OutputMode::Null).unwrap();
        assert_eq!(buf, b"name=alice\0");
    }
}
