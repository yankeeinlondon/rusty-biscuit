use claudine::actions::HookAction;

/// Returns the list of (field_name, display_label, current_value) for all editable
/// fields on the given action, so every property is reachable from the TUI.
pub(super) fn get_action_fields(action: &HookAction) -> Vec<(&'static str, &'static str, String)> {
    match action {
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
            ..
        } => vec![
            ("effect", "Effect", effect.clone()),
            ("volume", "Volume", format!("{volume}")),
            ("speed", "Speed", format!("{speed}")),
        ],
        HookAction::Speak {
            message,
            voice,
            gender,
            ..
        } => vec![
            ("message", "Message", message.clone()),
            ("voice", "Voice", voice.clone().unwrap_or_default()),
            (
                "gender",
                "Gender",
                gender
                    .map(|g| match g {
                        claudine::config::claudine_config::Gender::Male => "male",
                        claudine::config::claudine_config::Gender::Female => "female",
                    })
                    .unwrap_or("")
                    .to_string(),
            ),
        ],
        HookAction::Message { message, image, .. } => vec![
            ("message", "Message", message.clone()),
            ("image", "Image Path", image.clone().unwrap_or_default()),
        ],
        HookAction::Bash {
            command, params, ..
        } => vec![
            ("command", "Command", command.clone()),
            ("params", "Parameters", params.clone()),
        ],
        HookAction::Report { handler, .. } => {
            let (fmt, template, metadata) = match handler {
                Some(h) => (
                    match h.format {
                        claudine::actions::ReportFormat::Text => "text",
                        claudine::actions::ReportFormat::Json => "json",
                        claudine::actions::ReportFormat::Compact => "compact",
                        _ => "text",
                    }
                    .to_string(),
                    h.template.clone().unwrap_or_default(),
                    if h.include_metadata {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    },
                ),
                None => (String::new(), String::new(), String::new()),
            };
            vec![
                ("format", "Format (text/json/compact)", fmt),
                ("template", "Template", template),
                (
                    "include_metadata",
                    "Include Metadata (true/false)",
                    metadata,
                ),
            ]
        }
        HookAction::Call {
            command,
            args,
            timeout_ms,
            mapper,
            ..
        } => vec![
            ("command", "Command", command.clone()),
            (
                "args",
                "Args (comma-separated)",
                args.as_ref().map(|a| a.join(", ")).unwrap_or_default(),
            ),
            (
                "timeout_ms",
                "Timeout (ms)",
                timeout_ms.map(|t| t.to_string()).unwrap_or_default(),
            ),
            (
                "mapper",
                "Mapper",
                mapper
                    .as_ref()
                    .map(|m| match m {
                        claudine::actions::Mapper::JsonField { field } => {
                            format!("json_field:{field}")
                        }
                        claudine::actions::Mapper::JsonObject => "json_object".to_string(),
                        claudine::actions::Mapper::ExitCode => "exit_code".to_string(),
                        claudine::actions::Mapper::Regex { pattern } => {
                            format!("regex:{pattern}")
                        }
                        _ => String::new(),
                    })
                    .unwrap_or_default(),
            ),
        ],
        _ => vec![],
    }
}

/// Apply a text value to a specific named field on an action.
pub(super) fn apply_action_field(action: &mut HookAction, field_name: &str, value: String) {
    match action {
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
            ..
        } => match field_name {
            "effect" => *effect = value,
            "volume" => {
                if let Ok(v) = value.parse::<f32>() {
                    *volume = v.clamp(0.0, 1.0);
                }
            }
            "speed" => {
                if let Ok(v) = value.parse::<f32>() {
                    *speed = v.max(0.1);
                }
            }
            _ => {}
        },
        HookAction::Speak {
            message,
            voice,
            gender,
            ..
        } => match field_name {
            "message" => *message = value,
            "voice" => {
                *voice = if value.is_empty() { None } else { Some(value) };
            }
            "gender" => {
                *gender = match value.to_lowercase().as_str() {
                    "male" => Some(claudine::config::claudine_config::Gender::Male),
                    "female" => Some(claudine::config::claudine_config::Gender::Female),
                    _ => None,
                };
            }
            _ => {}
        },
        HookAction::Message { message, image, .. } => match field_name {
            "message" => *message = value,
            "image" => {
                *image = if value.is_empty() { None } else { Some(value) };
            }
            _ => {}
        },
        HookAction::Bash {
            command, params, ..
        } => match field_name {
            "command" => *command = value,
            "params" => *params = value,
            _ => {}
        },
        HookAction::Report { handler, .. } => {
            let h = handler.get_or_insert_with(|| claudine::actions::ReportHandler {
                format: claudine::actions::ReportFormat::Text,
                template: None,
                include_metadata: false,
            });
            match field_name {
                "format" => {
                    h.format = match value.to_lowercase().as_str() {
                        "json" => claudine::actions::ReportFormat::Json,
                        "compact" => claudine::actions::ReportFormat::Compact,
                        _ => claudine::actions::ReportFormat::Text,
                    };
                }
                "template" => {
                    h.template = if value.is_empty() { None } else { Some(value) };
                }
                "include_metadata" => {
                    h.include_metadata = value == "true";
                }
                _ => {}
            }
        }
        HookAction::Call {
            command,
            args,
            timeout_ms,
            mapper,
            ..
        } => match field_name {
            "command" => *command = value,
            "args" => {
                let parsed: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                *args = if parsed.is_empty() {
                    None
                } else {
                    Some(parsed)
                };
            }
            "timeout_ms" => {
                *timeout_ms = value.parse::<u64>().ok();
            }
            "mapper" => {
                *mapper = if value.is_empty() {
                    None
                } else if value == "json_object" {
                    Some(claudine::actions::Mapper::JsonObject)
                } else if value == "exit_code" {
                    Some(claudine::actions::Mapper::ExitCode)
                } else if let Some(field) = value.strip_prefix("json_field:") {
                    Some(claudine::actions::Mapper::JsonField {
                        field: field.to_string(),
                    })
                } else {
                    value
                        .strip_prefix("regex:")
                        .map(|pattern| claudine::actions::Mapper::Regex {
                            pattern: pattern.to_string(),
                        })
                };
            }
            _ => {}
        },
        _ => {}
    }
}
