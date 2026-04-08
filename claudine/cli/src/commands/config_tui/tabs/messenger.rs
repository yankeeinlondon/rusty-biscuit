use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState};
use claudine::config::claudine_config::{ClaudineMessengerConfig, MessengerProviderConfig};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let configs: Vec<(String, &MessengerProviderConfig)> = app
        .config
        .messenger
        .as_ref()
        .map(|m| {
            m.configurations
                .iter()
                .map(|(k, v)| (k.clone(), v))
                .collect()
        })
        .unwrap_or_default();

    let active_name = app
        .config
        .messenger
        .as_ref()
        .and_then(|m| m.active_config.as_deref())
        .unwrap_or("None");

    // Calculate layout based on content
    let config_lines = configs.len().max(1); // at least 1 line for "no configs" message
    let repo_line_count: u16 = if app.is_in_repo { 1 } else { 0 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),               // Active config line
            Constraint::Length(repo_line_count), // Repo override line (0 when not in repo)
            Constraint::Length(1),               // blank
            Constraint::Length(1),               // "Configurations:" heading
            Constraint::Length(config_lines as u16),
            Constraint::Length(1), // blank
            Constraint::Length(1), // [+ Add New] button
            Constraint::Min(0),
        ])
        .split(area);

    // Active messenger line
    let select_style = if is_detail && app.messenger_focus == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if is_detail {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let select_line = Line::from(vec![
        Span::styled("Active", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": "),
        Span::styled(format!("[{active_name}]"), select_style),
    ]);
    frame.render_widget(Paragraph::new(select_line), chunks[0]);

    // Repo override line (only when in a repo)
    if app.is_in_repo {
        let repo_override_text = match app
            .repo_config
            .as_ref()
            .and_then(|rc| rc.active_messenger.as_ref())
        {
            Some(Some(name)) => format!("Repo override: {name}"),
            Some(None) => "Repo override: (disabled)".to_string(),
            None => "Repo override: (inherits user)".to_string(),
        };
        let repo_line = Paragraph::new(Span::styled(
            repo_override_text,
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(repo_line, chunks[1]);
    }

    // Configurations heading
    let heading = Paragraph::new(Span::styled(
        "Configurations:",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(heading, chunks[3]);

    // List configurations
    if configs.is_empty() {
        let no_configs = Paragraph::new(Span::styled(
            "  No messenger configurations. Press A to add one.",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(no_configs, chunks[4]);
    } else {
        let items: Vec<ListItem> = configs
            .iter()
            .map(|(name, config)| {
                let provider_type = match config {
                    MessengerProviderConfig::Discord { .. } => "Discord",
                    MessengerProviderConfig::Slack { .. } => "Slack",
                    MessengerProviderConfig::Signal { .. } => "Signal",
                    MessengerProviderConfig::Whatsapp { .. } => "Whatsapp",
                };
                let is_active = app
                    .config
                    .messenger
                    .as_ref()
                    .and_then(|m| m.active_config.as_deref())
                    == Some(name.as_str());
                let marker = if is_active { " ✓" } else { "" };
                let style = if is_active {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(name.as_str(), style.add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!(" ({provider_type})"),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(marker, Style::default().fg(Color::Green)),
                ]))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, chunks[4]);
    }

    // Add button
    let add_style = if is_detail && app.messenger_focus == 1 {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let add_line = Paragraph::new(Span::styled("[+ Add New]", add_style));
    frame.render_widget(add_line, chunks[6]);

    // Render modals
    if let Some(ModalState::MessengerSelect {
        highlighted,
        for_repo,
    }) = &app.modal
    {
        let config_names: Vec<String> = configs.iter().map(|(k, _)| k.clone()).collect();
        let (title, items) = if *for_repo {
            let mut items = vec!["(inherit user)".to_string(), "(disabled)".to_string()];
            items.extend(config_names);
            ("Repo Messenger Override", items)
        } else {
            let mut items = vec!["(none)".to_string()];
            items.extend(config_names);
            ("Select Active Messenger", items)
        };
        super::super::widgets::modal::render_list_modal(frame, area, title, &items, *highlighted);
    }

    if let Some(ModalState::MessengerAdd { highlighted }) = &app.modal {
        let providers = ["discord", "slack", "signal", "whatsapp"];
        let items: Vec<String> = providers.iter().map(|s| s.to_string()).collect();
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Add Messenger Provider",
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::MessengerInput {
        provider,
        field_index,
        label,
        buffer,
        ..
    }) = &app.modal
    {
        let total = messenger_fields_with_name(provider).len();
        let title = format!("{} ({}/{})", provider, field_index + 1, total);
        super::super::widgets::modal::render_modal(frame, area, &title, 55, 20, |frame, area| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // label
                    Constraint::Length(1), // input line
                    Constraint::Length(1), // blank
                    Constraint::Length(1), // hotkeys
                    Constraint::Min(0),
                ])
                .split(area);

            let label_widget = Paragraph::new(Span::styled(
                label.as_str(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            frame.render_widget(label_widget, chunks[0]);

            let input_line = Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Yellow)),
                Span::raw(buffer.as_str()),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]);
            frame.render_widget(Paragraph::new(input_line), chunks[1]);

            let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
                ("ENTER", "Next"),
                ("ESC", "Cancel"),
            ]);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[3]);
        });
    }
}

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
        KeyCode::Char('r') | KeyCode::Char('R') if app.is_in_repo => {
            let configs: Vec<String> = app
                .config
                .messenger
                .as_ref()
                .map(|m| m.configurations.keys().cloned().collect())
                .unwrap_or_default();
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
                // Select active - only show select modal if there are configs
                let has_configs = app
                    .config
                    .messenger
                    .as_ref()
                    .map(|m| !m.configurations.is_empty())
                    .unwrap_or(false);
                if has_configs {
                    let configs: Vec<String> = app
                        .config
                        .messenger
                        .as_ref()
                        .map(|m| m.configurations.keys().cloned().collect())
                        .unwrap_or_default();
                    let active = app
                        .config
                        .messenger
                        .as_ref()
                        .and_then(|m| m.active_config.as_deref());
                    let highlighted = active
                        .and_then(|name| configs.iter().position(|k| k == name))
                        .map(|i| i + 1) // +1 for "(none)" at index 0
                        .unwrap_or(0);
                    app.modal = Some(ModalState::MessengerSelect {
                        highlighted,
                        for_repo: false,
                    });
                } else {
                    // No configs to select from - offer to add instead
                    app.modal = Some(ModalState::MessengerAdd { highlighted: 0 });
                }
            }
            1 => {
                app.modal = Some(ModalState::MessengerAdd { highlighted: 0 });
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn handle_messenger_select_modal(app: &mut App, key: KeyEvent) {
    let for_repo = matches!(
        app.modal,
        Some(ModalState::MessengerSelect { for_repo: true, .. })
    );

    let configs: Vec<String> = app
        .config
        .messenger
        .as_ref()
        .map(|m| m.configurations.keys().cloned().collect())
        .unwrap_or_default();

    if for_repo {
        // Repo selection: 0 = inherit, 1 = disabled, 2+ = config names
        let count = configs.len() + 2;
        match key.code {
            KeyCode::Up => {
                let idx = app.modal_highlighted();
                if idx > 0 {
                    app.set_modal_highlighted(idx - 1);
                }
            }
            KeyCode::Down => {
                let idx = app.modal_highlighted();
                if idx + 1 < count {
                    app.set_modal_highlighted(idx + 1);
                }
            }
            KeyCode::Enter => {
                let idx = app.modal_highlighted();
                if app.repo_config.is_none() {
                    app.repo_config =
                        Some(claudine::config::claudine_config::RepoOverrideConfig::default());
                }
                if let Some(ref mut repo_cfg) = app.repo_config {
                    match idx {
                        0 => repo_cfg.active_messenger = None,       // inherit
                        1 => repo_cfg.active_messenger = Some(None), // disabled
                        i => {
                            if let Some(name) = configs.get(i - 2) {
                                repo_cfg.active_messenger = Some(Some(name.clone()));
                            }
                        }
                    }
                }
                app.repo_dirty = true;
                app.modal = None;
            }
            KeyCode::Esc => {
                app.modal = None;
            }
            _ => {}
        }
    } else {
        // User-scope selection: 0 = (none), 1+ = config names
        let count = configs.len() + 1;
        match key.code {
            KeyCode::Up => {
                let idx = app.modal_highlighted();
                if idx > 0 {
                    app.set_modal_highlighted(idx - 1);
                }
            }
            KeyCode::Down => {
                let idx = app.modal_highlighted();
                if idx + 1 < count {
                    app.set_modal_highlighted(idx + 1);
                }
            }
            KeyCode::Enter => {
                let idx = app.modal_highlighted();
                ensure_messenger_config(app);
                if let Some(ref mut messenger) = app.config.messenger {
                    if idx == 0 {
                        messenger.active_config = None;
                    } else if let Some(name) = configs.get(idx - 1) {
                        messenger.active_config = Some(name.clone());
                    }
                }
                app.dirty = true;
                app.modal = None;
            }
            KeyCode::Esc => {
                app.modal = None;
            }
            _ => {}
        }
    }
}

pub fn handle_messenger_add_modal(app: &mut App, key: KeyEvent) {
    let providers = ["discord", "slack", "signal", "whatsapp"];
    let count = providers.len();
    match key.code {
        KeyCode::Up => {
            let idx = app.modal_highlighted();
            if idx > 0 {
                app.set_modal_highlighted(idx - 1);
            }
        }
        KeyCode::Down => {
            let idx = app.modal_highlighted();
            if idx + 1 < count {
                app.set_modal_highlighted(idx + 1);
            }
        }
        KeyCode::Enter => {
            let idx = app.modal_highlighted();
            if let Some(provider) = providers.get(idx) {
                // Start with a "Configuration Name" field so the user can give
                // a unique name (allowing multiple configs per provider).
                app.push_modal(ModalState::MessengerInput {
                    provider: provider.to_string(),
                    field_index: 0,
                    fields: Vec::new(),
                    buffer: String::new(),
                    label: "Configuration Name".to_string(),
                });
            }
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}

/// Returns the ordered list of (label, default_value) for a messenger provider,
/// **including** the leading "Configuration Name" field at index 0.
fn messenger_fields_with_name(provider: &str) -> Vec<(String, String)> {
    let mut fields = vec![("Configuration Name".to_string(), provider.to_string())];
    fields.extend(messenger_fields(provider));
    fields
}

/// Returns the ordered list of (label, default_value) for a messenger provider.
fn messenger_fields(provider: &str) -> Vec<(String, String)> {
    match provider {
        "discord" => vec![
            ("Channel ID".to_string(), String::new()),
            (
                "Bot Token Env Var".to_string(),
                "DISCORD_BOT_TOKEN".to_string(),
            ),
        ],
        "slack" => vec![
            ("Channel ID".to_string(), String::new()),
            (
                "Bot Token Env Var".to_string(),
                "SLACK_BOT_TOKEN".to_string(),
            ),
        ],
        "signal" => vec![
            ("Recipient".to_string(), String::new()),
            ("RPC URL Env Var".to_string(), "SIGNAL_RPC_URL".to_string()),
            ("Account Env Var".to_string(), "SIGNAL_ACCOUNT".to_string()),
        ],
        "whatsapp" => vec![
            ("Recipient".to_string(), String::new()),
            (
                "Access Token Env Var".to_string(),
                "WHATSAPP_ACCESS_TOKEN".to_string(),
            ),
            (
                "Phone Number ID Env Var".to_string(),
                "WHATSAPP_PHONE_NUMBER_ID".to_string(),
            ),
        ],
        _ => vec![],
    }
}

/// Build the actual `MessengerProviderConfig` from collected field values.
fn build_messenger_from_fields(
    provider: &str,
    fields: &[(String, String)],
) -> Option<MessengerProviderConfig> {
    match provider {
        "discord" => Some(MessengerProviderConfig::Discord {
            channel_id: fields.get(0).map(|(_, v)| v.clone()).unwrap_or_default(),
            bot_token_env: fields
                .get(1)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "DISCORD_BOT_TOKEN".to_string()),
        }),
        "slack" => Some(MessengerProviderConfig::Slack {
            channel_id: fields.get(0).map(|(_, v)| v.clone()).unwrap_or_default(),
            bot_token_env: fields
                .get(1)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "SLACK_BOT_TOKEN".to_string()),
        }),
        "signal" => Some(MessengerProviderConfig::Signal {
            recipient: fields.get(0).map(|(_, v)| v.clone()).unwrap_or_default(),
            rpc_url_env: fields
                .get(1)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "SIGNAL_RPC_URL".to_string()),
            account_env: fields
                .get(2)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "SIGNAL_ACCOUNT".to_string()),
        }),
        "whatsapp" => Some(MessengerProviderConfig::Whatsapp {
            recipient: fields.get(0).map(|(_, v)| v.clone()).unwrap_or_default(),
            access_token_env: fields
                .get(1)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "WHATSAPP_ACCESS_TOKEN".to_string()),
            phone_number_id_env: fields
                .get(2)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "WHATSAPP_PHONE_NUMBER_ID".to_string()),
        }),
        _ => None,
    }
}

pub fn handle_messenger_input_modal(app: &mut App, key: KeyEvent) {
    let (provider, field_index, total_fields) = match &app.modal {
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
        KeyCode::Char(c) => {
            if let Some(ModalState::MessengerInput { buffer, .. }) = &mut app.modal {
                buffer.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(ModalState::MessengerInput { buffer, .. }) = &mut app.modal {
                buffer.pop();
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
                    .map(|(_, def)| def.clone())
                    .unwrap_or_default()
            } else {
                current_buffer
            };
            collected_fields.push((
                all_field_defs
                    .get(field_index)
                    .map(|(l, _)| l.clone())
                    .unwrap_or_default(),
                value,
            ));

            if field_index + 1 < total_fields {
                // Advance to next field
                let next_idx = field_index + 1;
                let (next_label, next_default) = all_field_defs[next_idx].clone();
                app.modal = Some(ModalState::MessengerInput {
                    provider: provider_name,
                    field_index: next_idx,
                    fields: collected_fields,
                    buffer: next_default,
                    label: next_label,
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
        KeyCode::Esc => {
            // Cancel the entire add flow
            app.modal = None;
            app.modal_stack.clear();
        }
        _ => {}
    }
}

fn ensure_messenger_config(app: &mut App) {
    if app.config.messenger.is_none() {
        app.config.messenger = Some(ClaudineMessengerConfig {
            active_config: None,
            configurations: Default::default(),
        });
    }
}
