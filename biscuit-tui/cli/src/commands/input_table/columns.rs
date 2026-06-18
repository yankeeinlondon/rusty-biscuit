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

pub(super) fn build_choice_input(
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
