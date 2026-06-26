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
//!
//! ## Permissive Row-Value Contracts
//!
//! The CLI accepts a small set of compatibility coercions for row values,
//! documented here so schema mistakes are not confused with intentional
//! parsing. Everything outside these contracts is an `InvalidInput` error
//! with row/column context.
//!
//! - **boolean-switch** rows accept a JSON boolean, a JSON number
//!   (non-zero is true), or one of the strings `true`, `on`, `yes`, `1`,
//!   `false`, `off`, `no`, `0` (case-insensitive). Other strings and
//!   any other JSON type are errors.
//! - **text-area-input** rows accept either a JSON array of strings or a
//!   single JSON string split on `\n`. Any other JSON type is an error.
//! - **choose-many** rows accept either a JSON array of strings or a
//!   single JSON string split on `,` (with whitespace trimmed and
//!   empties dropped). Any other JSON type is an error.
//! - **static-text**, **text-input**, and **choose-one** rows accept a
//!   JSON string (or JSON null, treated as the empty string). Any other
//!   JSON type is an error.

use std::io::{self, Write};

use clap::Args;
use serde_json::{Value, json};
#[cfg(test)]
use biscuit_tui::components::input_table::{
    BooleanSwitchConfig, TextAreaInputConfig, TextInputConfig,
};
use biscuit_tui::components::input_table::CellValue;
use biscuit_tui::{
    ABORTED_KIND, CANCELLED_KIND, FrameChromeConfig, HeightSpec, InputTable, InputTableColumn,
    InputTableState, Row, RowCell, run_standalone_with_chrome,
};
#[cfg(test)]
use biscuit_tui::{ChoiceInput, ChoiceOption};

use crate::output::OutputMode;

mod columns;

use columns::{ColumnSpec, json_kind, parse_columns};

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
        InputTableState::try_new(columns, row_values)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
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
/// Per-cell JSON values that do not match the documented permissive
/// contract for the column type produce `InvalidInput` errors tagged with
/// `row {n} column '{id}'` context.
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
        .enumerate()
        .map(|(row_idx, row)| {
            let inner = row.as_array().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("--rows: row {row_idx} must be a JSON array"),
                )
            })?;
            if inner.len() != columns.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "--rows: row {row_idx} has {} cells, expected {}",
                        inner.len(),
                        columns.len()
                    ),
                ));
            }
            let cells: Vec<RowCell> = inner
                .iter()
                .zip(columns.iter())
                .map(|(val, col)| {
                    let cv = parse_cell_value(val, col).map_err(|e| {
                        io::Error::new(
                            e.kind(),
                            format!("--rows: row {row_idx} column '{}': {}", col.id(), e),
                        )
                    })?;
                    Ok(RowCell::new(col.id().to_string(), cv))
                })
                .collect::<io::Result<_>>()?;
            Ok(Row::new(cells))
        })
        .collect()
}

/// Parses a single JSON cell value into the appropriate `CellValue` variant
/// based on the column type.
///
/// Permissive coercions (boolean from number/string allowlist, text-area
/// from newline-split string, choose-many from comma-split string) are
/// documented at the module level. Any other mismatch produces
/// `InvalidInput` with the column's expected shape.
fn parse_cell_value(val: &Value, col: &ColumnSpec) -> io::Result<CellValue> {
    match col {
        ColumnSpec::StaticText { .. } => match val {
            Value::String(s) => Ok(CellValue::StaticText(s.clone())),
            Value::Null => Ok(CellValue::StaticText(String::new())),
            other => Err(type_mismatch_err("string", other)),
        },
        ColumnSpec::BooleanSwitch { .. } => parse_boolean_value(val).map(CellValue::Boolean),
        ColumnSpec::TextInput { .. } => match val {
            Value::String(s) => Ok(CellValue::Text(s.clone())),
            Value::Null => Ok(CellValue::Text(String::new())),
            other => Err(type_mismatch_err("string", other)),
        },
        ColumnSpec::TextAreaInput { .. } => parse_text_area_value(val).map(CellValue::TextArea),
        ColumnSpec::ChooseOne { .. } => match val {
            Value::String(s) => Ok(if s.is_empty() {
                CellValue::ChosenOne(None)
            } else {
                CellValue::ChosenOne(Some(s.clone()))
            }),
            Value::Null => Ok(CellValue::ChosenOne(None)),
            other => Err(type_mismatch_err("string", other)),
        },
        ColumnSpec::ChooseMany { .. } => parse_choose_many_value(val).map(CellValue::ChosenMany),
    }
}

fn parse_boolean_value(val: &Value) -> io::Result<bool> {
    match val {
        Value::Bool(b) => Ok(*b),
        Value::Number(n) => Ok(n.as_i64().unwrap_or(0) != 0),
        Value::String(s) => {
            let lower = s.trim().to_ascii_lowercase();
            match lower.as_str() {
                "true" | "on" | "yes" | "1" => Ok(true),
                "false" | "off" | "no" | "0" => Ok(false),
                _ => Err(type_mismatch_err(
                    "boolean (true|on|yes|1|false|off|no|0)",
                    val,
                )),
            }
        }
        other => Err(type_mismatch_err(
            "boolean (bool, number, or true|on|yes|1|false|off|no|0)",
            other,
        )),
    }
}

fn parse_text_area_value(val: &Value) -> io::Result<Vec<String>> {
    match val {
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                Value::Null => Ok(String::new()),
                other => Err(type_mismatch_err("string", other)),
            })
            .collect(),
        Value::String(s) => Ok(s.split('\n').map(str::to_string).collect()),
        Value::Null => Ok(Vec::new()),
        other => Err(type_mismatch_err("array of strings or newline-split string", other)),
    }
}

fn parse_choose_many_value(val: &Value) -> io::Result<Vec<String>> {
    match val {
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                Value::Null => Ok(String::new()),
                other => Err(type_mismatch_err("string", other)),
            })
            .collect(),
        Value::String(s) => Ok(s
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()),
        Value::Null => Ok(Vec::new()),
        other => Err(type_mismatch_err("array of strings or comma-split string", other)),
    }
}

fn type_mismatch_err(expected: &str, found: &Value) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("expected {expected}, got {}", json_kind(found)),
    )
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
