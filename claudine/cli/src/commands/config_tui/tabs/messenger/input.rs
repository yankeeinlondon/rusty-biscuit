//! Messenger tab key/input handling: the top-level tab key dispatch and the
//! field-by-field input modal (text entry, webhook validation, the `T`
//! test-connection trigger, and field navigation).

use crossterm::event::{KeyCode, KeyEvent};

use super::super::super::app::{App, ModalState};
use super::routes::{build_messenger_from_fields, is_webhook, messenger_fields_with_name};
use super::test_connection::{build_test_route_from_modal, can_test};
use super::{ensure_messenger_config, sorted_messenger_names};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            app.messenger_focus = (app.messenger_focus + 1) % 2;
        }
        KeyCode::BackTab => {
            app.messenger_focus = (app.messenger_focus + 2 - 1) % 2;
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.modal = Some(ModalState::MessengerAdd { highlighted: 0 });
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            open_messenger_select_modal(app);
        }
        KeyCode::Char('r') | KeyCode::Char('R') if app.is_in_repo => {
            let configs = sorted_messenger_names(app);
            // Index 0 = "(inherit user)", 1 = "(disabled)", 2+ = config names
            let repo_active = app
                .repo_config
                .as_ref()
                .and_then(|rc| rc.active_messenger.as_ref());
            let highlighted = match repo_active {
                None => 0,       // inherits
                Some(None) => 1, // disabled
                Some(Some(name)) => configs
                    .iter()
                    .position(|k| k == name)
                    .map(|i| i + 2)
                    .unwrap_or(0),
            };
            app.modal = Some(ModalState::MessengerSelect {
                highlighted,
                for_repo: true,
            });
        }
        KeyCode::Enter => match app.messenger_focus {
            0 => {
                open_messenger_select_modal(app);
            }
            1 => {
                app.modal = Some(ModalState::MessengerAdd { highlighted: 0 });
            }
            _ => {}
        },
        _ => {}
    }
}

fn open_messenger_select_modal(app: &mut App) {
    let configs = sorted_messenger_names(app);
    if configs.is_empty() {
        app.modal = Some(ModalState::MessengerAdd { highlighted: 0 });
        return;
    }

    let active = app
        .config
        .messenger
        .as_ref()
        .and_then(|m| m.active_config.as_deref());
    let highlighted = active
        .and_then(|name| configs.iter().position(|k| k == name))
        .map(|i| i + 1)
        .unwrap_or(0);
    app.modal = Some(ModalState::MessengerSelect {
        highlighted,
        for_repo: false,
    });
}

pub fn handle_messenger_input_modal(app: &mut App, key: KeyEvent) {
    let (_provider, field_index, total_fields) = match &app.modal {
        Some(ModalState::MessengerInput {
            provider,
            field_index,
            ..
        }) => {
            // Total fields includes the leading "Configuration Name" field.
            let total = messenger_fields_with_name(provider).len();
            (provider.clone(), *field_index, total)
        }
        _ => return,
    };

    match key.code {
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if !can_test(&_provider, field_index) {
                return;
            }

            // Prevent spawning a second test while one is already running
            if app.pending_test.is_some() {
                return;
            }

            // Build temporary route config from modal state
            let (buffer, fields, provider_name) = match &app.modal {
                Some(ModalState::MessengerInput {
                    buffer,
                    fields,
                    provider,
                    ..
                }) => (buffer.clone(), fields.clone(), provider.clone()),
                _ => return,
            };

            let Some(route_config) =
                build_test_route_from_modal(&provider_name, &fields, &buffer, field_index)
            else {
                return;
            };

            // Show "Testing…" immediately so the user knows something happened
            if let Some(ModalState::MessengerInput {
                test_status, error, ..
            }) = &mut app.modal
            {
                *test_status = Some("Testing…".to_string());
                *error = None;
            }

            // Spawn the test in a background Tokio task and wire the result
            // back through a synchronous channel polled by the event loop.
            let (tx, rx) = std::sync::mpsc::channel();
            app.pending_test = Some(rx);

            tokio::spawn(async move {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    claudine::messaging::test_webhook_connection(&route_config),
                )
                .await
                .unwrap_or(Err(
                    claudine::messaging::MessagingError::TestConnectionTimeout,
                ));
                let _ = tx.send(result);
            });
        }
        KeyCode::Char(c) => {
            if let Some(ModalState::MessengerInput {
                buffer,
                error,
                test_status,
                ..
            }) = &mut app.modal
            {
                buffer.push(c);
                *error = None; // Clear error on input
                *test_status = None; // Clear test status on input
            }
        }
        KeyCode::Backspace => {
            if let Some(ModalState::MessengerInput {
                buffer,
                error,
                test_status,
                ..
            }) = &mut app.modal
            {
                buffer.pop();
                *error = None; // Clear error on input
                *test_status = None; // Clear test status on input
            }
        }
        KeyCode::Enter => {
            // Save current field and advance or finish
            let (current_buffer, mut collected_fields, provider_name) = match &app.modal {
                Some(ModalState::MessengerInput {
                    buffer,
                    fields,
                    provider,
                    ..
                }) => (buffer.clone(), fields.clone(), provider.clone()),
                _ => return,
            };

            // Use the name-aware field definitions (index 0 = config name).
            let all_field_defs = messenger_fields_with_name(&provider_name);
            let value = if current_buffer.is_empty() {
                all_field_defs
                    .get(field_index)
                    .map(|(_, def, _)| def.clone())
                    .unwrap_or_default()
            } else {
                current_buffer
            };

            // Validate webhook URL fields before advancing
            if let Some((label, _, is_secret)) = all_field_defs.get(field_index)
                && *is_secret
                && !value.trim().is_empty()
            {
                let valid = if is_webhook(&provider_name) {
                    match provider_name.as_str() {
                        "discord_webhook" => {
                            claudine::messaging::validate_discord_webhook_url(&value)
                        }
                        "slack_webhook" => claudine::messaging::validate_slack_webhook_url(&value),
                        _ => true,
                    }
                } else {
                    true
                };
                if !valid {
                    let error_msg = format!("Invalid {} format", label);
                    app.modal = Some(ModalState::MessengerInput {
                        provider: provider_name,
                        field_index,
                        fields: collected_fields,
                        buffer: value,
                        label: label.clone(),
                        is_secret: *is_secret,
                        error: Some(error_msg),
                        test_status: None,
                    });
                    return;
                }
            }

            collected_fields.push((
                all_field_defs
                    .get(field_index)
                    .map(|(l, _, _)| l.clone())
                    .unwrap_or_default(),
                value,
            ));

            if field_index + 1 < total_fields {
                // Advance to next field
                let next_idx = field_index + 1;
                let (next_label, next_default, next_secret) = all_field_defs[next_idx].clone();
                app.modal = Some(ModalState::MessengerInput {
                    provider: provider_name,
                    field_index: next_idx,
                    fields: collected_fields,
                    buffer: next_default,
                    label: next_label,
                    is_secret: next_secret,
                    error: None,
                    test_status: None,
                });
            } else {
                // All fields collected. Index 0 is the user-defined config name;
                // the remaining fields are provider-specific settings.
                let config_name = collected_fields
                    .first()
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| provider_name.clone());
                let provider_fields: Vec<(String, String)> =
                    collected_fields.into_iter().skip(1).collect();
                if let Some(config) = build_messenger_from_fields(&provider_name, &provider_fields)
                {
                    ensure_messenger_config(app);
                    if let Some(ref mut messenger) = app.config.messenger {
                        messenger.configurations.insert(config_name.clone(), config);
                        messenger.active_config = Some(config_name);
                    }
                    app.dirty = true;
                }
                // Pop back to main messenger view (clear modal stack too)
                app.modal = None;
                app.modal_stack.clear();
            }
        }
        KeyCode::BackTab => {
            if field_index == 0 {
                return;
            }

            let (provider_name, mut collected_fields, _current_buffer, current_idx) =
                match &app.modal {
                    Some(ModalState::MessengerInput {
                        provider,
                        fields,
                        buffer,
                        field_index,
                        ..
                    }) => (
                        provider.clone(),
                        fields.clone(),
                        buffer.clone(),
                        *field_index,
                    ),
                    _ => return,
                };

            // Pop the last committed field to go back to it
            let Some((prev_label, prev_value)) = collected_fields.pop() else {
                return;
            };

            let all_field_defs = messenger_fields_with_name(&provider_name);
            let prev_idx = current_idx - 1;
            let (_, _, prev_secret) = all_field_defs
                .get(prev_idx)
                .cloned()
                .unwrap_or_else(|| (prev_label.clone(), String::new(), false));

            app.modal = Some(ModalState::MessengerInput {
                provider: provider_name,
                field_index: prev_idx,
                fields: collected_fields,
                buffer: prev_value,
                label: prev_label,
                is_secret: prev_secret,
                error: None,
                test_status: None,
            });
        }
        KeyCode::Esc => {
            // Cancel the entire add flow
            app.modal = None;
            app.modal_stack.clear();
        }
        _ => {}
    }
}
