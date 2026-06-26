use std::io;

use serde_json::Value;
use biscuit_tui::components::input_table::{
    BooleanSwitchConfig, TextAreaInputConfig, TextInputConfig,
};
use biscuit_tui::{ChoiceInput, ChoiceOption, InputTableColumn};

#[derive(Debug, Clone)]
pub(super) enum ColumnSpec {
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
    pub(super) fn id(&self) -> &str {
        match self {
            Self::StaticText { id, .. }
            | Self::TextInput { id, .. }
            | Self::BooleanSwitch { id, .. }
            | Self::TextAreaInput { id, .. }
            | Self::ChooseOne { id, .. }
            | Self::ChooseMany { id, .. } => id,
        }
    }

    pub(super) fn to_column(&self) -> InputTableColumn {
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

pub(super) fn parse_columns(json: &str) -> io::Result<Vec<ColumnSpec>> {
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
            let text = opt_string_field(object, index, "text", "")?;
            Ok(ColumnSpec::StaticText { id, text })
        }
        "text-input" => {
            let initial = opt_string_field(object, index, "initial", "")?;
            let max_length = opt_usize_field(object, index, "max_length")?;
            Ok(ColumnSpec::TextInput {
                id,
                config: TextInputConfig {
                    initial,
                    max_length,
                },
            })
        }
        "boolean-switch" => {
            let initial = opt_bool_field(object, index, "initial", false)?;
            let on_label = opt_string_field(object, index, "on_label", "ON")?;
            let off_label = opt_string_field(object, index, "off_label", "OFF")?;
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
            let preferred_width = opt_u16_field(object, index, "preferred_width")?.unwrap_or(40);
            let preferred_height =
                opt_u16_field(object, index, "preferred_height")?.unwrap_or(3);
            let initial = opt_string_array_field(object, index, "initial")?;
            let scrollbar = opt_bool_field(object, index, "scrollbar", false)?;
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
            input: build_choice_input(&id, object, index)?,
        }),
        "choose-many" => Ok(ColumnSpec::ChooseMany {
            id: id.clone(),
            input: build_choice_input(&id, object, index)?,
        }),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("column {index}: unknown type '{other}'"),
        )),
    }
}

pub(super) fn build_choice_input(
    id: &str,
    object: &serde_json::Map<String, Value>,
    index: usize,
) -> io::Result<ChoiceInput<String>> {
    let options_value = object.get("options").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("column {index}: missing 'options' array"),
        )
    })?;
    let array = options_value.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("column {index}: 'options' must be a JSON array"),
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
    let required = opt_bool_field(object, index, "required", false)?;
    if required {
        input = input.required();
    }
    if let Some(min) = opt_usize_field(object, index, "min_selections")? {
        input = input.with_min_selections(min);
    }
    if let Some(max) = opt_usize_field(object, index, "max_selections")? {
        input = input.with_max_selections(max);
    }
    Ok(input)
}

/// Human-readable JSON type name for diagnostics.
pub(super) fn json_kind(val: &Value) -> &'static str {
    match val {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn column_error(index: usize, field: &str, msg: String) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("column {index} field '{field}': {msg}"),
    )
}

/// Returns the string value of `field`, or `default` when absent or null.
///
/// Returns `InvalidInput` when the field is present with a non-string JSON
/// value, so schema mistakes surface at parse time instead of being silently
/// coerced.
fn opt_string_field(
    object: &serde_json::Map<String, Value>,
    index: usize,
    field: &str,
    default: &str,
) -> io::Result<String> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(default.to_string()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(column_error(
            index,
            field,
            format!("expected string, got {}", json_kind(other)),
        )),
    }
}

/// Returns the bool value of `field`, or `default` when absent or null.
fn opt_bool_field(
    object: &serde_json::Map<String, Value>,
    index: usize,
    field: &str,
    default: bool,
) -> io::Result<bool> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => Err(column_error(
            index,
            field,
            format!("expected boolean, got {}", json_kind(other)),
        )),
    }
}

/// Returns the `u16` value of `field`, or `None` when absent or null.
///
/// Rejects oversized values and non-integer numerics so dimension fields
/// cannot truncate silently.
fn opt_u16_field(
    object: &serde_json::Map<String, Value>,
    index: usize,
    field: &str,
) -> io::Result<Option<u16>> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => match n.as_u64() {
            Some(raw) => u16::try_from(raw).map(Some).map_err(|_| {
                column_error(
                    index,
                    field,
                    format!("value {raw} overflows u16 (max {})", u16::MAX),
                )
            }),
            None => Err(column_error(
                index,
                field,
                format!("expected non-negative integer, got {n}"),
            )),
        },
        Some(other) => Err(column_error(
            index,
            field,
            format!("expected number, got {}", json_kind(other)),
        )),
    }
}

/// Returns the `usize` value of `field`, or `None` when absent or null.
///
/// Uses `usize::try_from` so 32-bit targets surface overflow as `InvalidInput`
/// instead of truncating.
fn opt_usize_field(
    object: &serde_json::Map<String, Value>,
    index: usize,
    field: &str,
) -> io::Result<Option<usize>> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => match n.as_u64() {
            Some(raw) => usize::try_from(raw).map(Some).map_err(|_| {
                column_error(
                    index,
                    field,
                    format!(
                        "value {raw} overflows usize on this target (max {})",
                        usize::MAX
                    ),
                )
            }),
            None => Err(column_error(
                index,
                field,
                format!("expected non-negative integer, got {n}"),
            )),
        },
        Some(other) => Err(column_error(
            index,
            field,
            format!("expected number, got {}", json_kind(other)),
        )),
    }
}

/// Returns the string-array value of `field`, or an empty vec when absent.
///
/// Rejects arrays containing non-string elements so silent coercion cannot
/// drop or stringify mismatched entries.
fn opt_string_array_field(
    object: &serde_json::Map<String, Value>,
    index: usize,
    field: &str,
) -> io::Result<Vec<String>> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                other => Err(column_error(
                    index,
                    field,
                    format!("array element must be string, got {}", json_kind(other)),
                )),
            })
            .collect(),
        Some(other) => Err(column_error(
            index,
            field,
            format!("expected array of strings, got {}", json_kind(other)),
        )),
    }
}
