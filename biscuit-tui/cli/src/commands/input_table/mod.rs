//! `question input-table` subcommand.
//!
//! Maps CLI args onto a [`biscuit_tui::InputTableState`], runs the
//! component via [`biscuit_tui::run_standalone`], and writes the
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
#[cfg(test)]
use biscuit_tui::components::input_table::BooleanSwitchConfig;
use biscuit_tui::components::input_table::CellValue;
use biscuit_tui::{
    ABORTED_KIND, CANCELLED_KIND, FrameChromeConfig, HeightSpec, InputTable, InputTableColumn,
    InputTableState, Row, RowCell, run_standalone_with_chrome,
};
#[cfg(test)]
use biscuit_tui::{ChoiceInput, ChoiceOption};

use crate::output::OutputMode;

mod columns;

use columns::{ColumnSpec, parse_columns};

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
pub fn run(
    args: InputTableArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
    show_on_exit: bool,
) -> io::Result<i32> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let chrome = FrameChromeConfig {
        show_on_exit,
        ..Default::default()
    };
    run_with_writer(args, output, height, &mut lock, |state, height| {
        run_standalone_with_chrome(InputTable::new(), state, height, chrome)
    })
}

fn run_with_writer<F, W>(
    args: InputTableArgs,
    output: OutputMode,
    height: Option<HeightSpec>,
    writer: &mut W,
    run_prompt: F,
) -> io::Result<i32>
where
    F: FnOnce(InputTableState, Option<HeightSpec>) -> io::Result<Vec<Row>>,
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
        Err(e) if e.kind() == ABORTED_KIND => Ok(1),
        Err(e) => Err(e),
    }
}

/// Parses `--rows` JSON into typed [`Row`] values keyed by column id.
///
/// Each inner array is interpreted column-aware: boolean columns accept
/// truthy/falsy values, choose-many accepts arrays or comma-strings, etc.
fn parse_rows_typed(columns: &[ColumnSpec], json: Option<&str>) -> io::Result<Vec<Row>> {
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
            Ok(Row::new(
                inner
                    .iter()
                    .zip(columns.iter())
                    .map(|(val, col)| {
                        RowCell::new(col.id().to_string(), parse_cell_value(val, col))
                    })
                    .collect(),
            ))
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
mod tests;
