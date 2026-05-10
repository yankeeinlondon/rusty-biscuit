use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState};
use claudine::config::claudine_config::{ClaudineMessengerConfig, MessengerProviderConfig};

pub mod masked_input;
pub mod redaction;
pub mod routes;
pub mod test_connection;

use redaction::MASKED_WEBHOOK_DETAIL;
use routes::{PROVIDERS, build_messenger_from_fields, is_webhook, messenger_fields_with_name};
use test_connection::{build_test_route_from_modal, can_test};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let configs = sorted_messenger_configs(app);

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
                let (provider_type, masked_detail) = match config {
                    MessengerProviderConfig::Discord { .. } => ("Discord", None),
                    MessengerProviderConfig::Slack { .. } => ("Slack", None),
                    MessengerProviderConfig::Signal { .. } => ("Signal", None),
                    MessengerProviderConfig::Whatsapp { .. } => ("Whatsapp", None),
                    MessengerProviderConfig::DiscordWebhook { webhook_url, .. } => (
                        "Discord Webhook",
                        if webhook_url.is_some() {
                            Some(MASKED_WEBHOOK_DETAIL)
                        } else {
                            None
                        },
                    ),
                    MessengerProviderConfig::SlackWebhook { webhook_url, .. } => (
                        "Slack Webhook",
                        if webhook_url.is_some() {
                            Some(MASKED_WEBHOOK_DETAIL)
                        } else {
                            None
                        },
                    ),
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
                let detail = masked_detail
                    .map(|d| format!(" ({provider_type}) · {d}"))
                    .unwrap_or_else(|| format!(" ({provider_type})"));
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(name.as_str(), style.add_modifier(Modifier::BOLD)),
                    Span::styled(detail, Style::default().fg(Color::Gray)),
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
        let items: Vec<String> = PROVIDERS.iter().map(|s| s.to_string()).collect();
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
        is_secret,
        error,
        test_status,
        ..
    }) = &app.modal
    {
        let total = messenger_fields_with_name(provider).len();
        let title = format!("{} ({}/{})", provider, field_index + 1, total);
        let can_test_now = can_test(provider, *field_index);
        let has_error = error.is_some();
        let has_status = test_status.is_some();
        super::super::widgets::modal::render_modal(frame, area, &title, 55, 20, |frame, area| {
            let mut constraints: Vec<Constraint> = vec![
                Constraint::Length(1), // label
                Constraint::Length(1), // input line
            ];
            if has_error {
                constraints.push(Constraint::Length(1)); // error line
            }
            if has_status {
                constraints.push(Constraint::Length(1)); // test status line
            }
            constraints.push(Constraint::Length(1)); // blank
            constraints.push(Constraint::Length(1)); // hotkeys
            constraints.push(Constraint::Min(0));

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(area);

            let label_widget = Paragraph::new(Span::styled(
                label.as_str(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            frame.render_widget(label_widget, chunks[0]);

            let display_buffer = masked_input::render_buffer(buffer, *is_secret);
            let input_line = Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Yellow)),
                Span::raw(display_buffer),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]);
            frame.render_widget(Paragraph::new(input_line), chunks[1]);

            let mut chunk_idx = 2;
            if has_error {
                let error_text = error.as_ref().unwrap();
                let error_widget = Paragraph::new(Span::styled(
                    error_text.as_str(),
                    Style::default().fg(Color::Red),
                ));
                frame.render_widget(error_widget, chunks[chunk_idx]);
                chunk_idx += 1;
            }

            if has_status {
                let status_text = test_status.as_ref().unwrap();
                let status_color = if status_text.starts_with("✓") {
                    Color::Green
                } else {
                    Color::Yellow
                };
                let status_widget = Paragraph::new(Span::styled(
                    status_text.as_str(),
                    Style::default().fg(status_color),
                ));
                frame.render_widget(status_widget, chunks[chunk_idx]);
                chunk_idx += 1;
            }

            let mut hotkeys = vec![("ENTER", "Next"), ("ESC", "Cancel")];
            if can_test_now {
                hotkeys.push(("T", "Test"));
            }
            let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&hotkeys);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[chunk_idx]);
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

pub fn handle_messenger_select_modal(app: &mut App, key: KeyEvent) {
    let for_repo = matches!(
        app.modal,
        Some(ModalState::MessengerSelect { for_repo: true, .. })
    );

    let configs: Vec<String> = app
        .config
        .messenger
        .as_ref()
        .map(|m| {
            let mut names: Vec<_> = m.configurations.keys().cloned().collect();
            names.sort();
            names
        })
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

fn sorted_messenger_configs(app: &App) -> Vec<(String, &MessengerProviderConfig)> {
    let mut configs: Vec<_> = app
        .config
        .messenger
        .as_ref()
        .map(|m| {
            m.configurations
                .iter()
                .map(|(name, config)| (name.clone(), config))
                .collect()
        })
        .unwrap_or_default();
    configs.sort_by(|left, right| left.0.cmp(&right.0));
    configs
}

fn sorted_messenger_names(app: &App) -> Vec<String> {
    sorted_messenger_configs(app)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
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

pub fn handle_messenger_add_modal(app: &mut App, key: KeyEvent) {
    let count = PROVIDERS.len();
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
            if let Some(provider) = PROVIDERS.get(idx) {
                // Start with a "Configuration Name" field so the user can give
                // a unique name (allowing multiple configs per provider).
                app.push_modal(ModalState::MessengerInput {
                    provider: provider.to_string(),
                    field_index: 0,
                    fields: Vec::new(),
                    buffer: String::new(),
                    label: "Configuration Name".to_string(),
                    is_secret: false,
                    error: None,
                    test_status: None,
                });
            }
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
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
                .unwrap_or_else(|_| Err("Connection test timed out".to_string()));
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

fn ensure_messenger_config(app: &mut App) {
    if app.config.messenger.is_none() {
        app.config.messenger = Some(ClaudineMessengerConfig {
            active_config: None,
            configurations: Default::default(),
        });
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::routes::messenger_fields;
    use super::*;
    use claudine::config::claudine_config::ClaudineConfig;
    use claudine::messaging::MessagingRouteConfig;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn test_app() -> App {
        App::new(ClaudineConfig::default(), None, None, false, None, None)
    }

    #[test]
    fn messenger_names_are_sorted_alphabetically() {
        let mut app = test_app();
        app.config.messenger = Some(ClaudineMessengerConfig {
            active_config: None,
            configurations: std::collections::HashMap::from([
                (
                    "zeta".to_string(),
                    MessengerProviderConfig::Slack {
                        channel_id: "1".to_string(),
                        bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                    },
                ),
                (
                    "alpha".to_string(),
                    MessengerProviderConfig::Discord {
                        channel_id: "2".to_string(),
                        bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                    },
                ),
            ]),
        });

        assert_eq!(
            sorted_messenger_names(&app),
            vec!["alpha".to_string(), "zeta".to_string()]
        );
    }

    #[test]
    fn s_hotkey_opens_select_modal() {
        let mut app = test_app();
        app.config.messenger = Some(ClaudineMessengerConfig {
            active_config: Some("alpha".to_string()),
            configurations: std::collections::HashMap::from([(
                "alpha".to_string(),
                MessengerProviderConfig::Discord {
                    channel_id: "2".to_string(),
                    bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                },
            )]),
        });

        handle_key(&mut app, key(KeyCode::Char('s')));

        assert!(matches!(
            app.modal,
            Some(ModalState::MessengerSelect {
                highlighted: 1,
                for_repo: false
            })
        ));
    }

    #[test]
    fn webhook_provider_field_definitions() {
        let discord_fields = messenger_fields("discord_webhook");
        assert_eq!(discord_fields.len(), 2);
        assert_eq!(discord_fields[0].0, "Webhook URL");
        assert!(discord_fields[0].2); // is_secret
        assert_eq!(discord_fields[0].1, ""); // default empty
        assert_eq!(discord_fields[1].0, "Webhook URL Env Var");
        assert!(!discord_fields[1].2); // not secret
        assert_eq!(discord_fields[1].1, "DISCORD_WEBHOOK_URL");

        let slack_fields = messenger_fields("slack_webhook");
        assert_eq!(slack_fields.len(), 2);
        assert_eq!(slack_fields[0].0, "Webhook URL");
        assert!(slack_fields[0].2); // is_secret
        assert_eq!(slack_fields[1].0, "Webhook URL Env Var");
        assert!(!slack_fields[1].2); // not secret
        assert_eq!(slack_fields[1].1, "SLACK_WEBHOOK_URL");
    }

    #[test]
    fn build_messenger_from_fields_discord_webhook() {
        let fields = vec![
            (
                "Webhook URL".to_string(),
                "https://discord.com/api/webhooks/123/abc".to_string(),
            ),
            (
                "Webhook URL Env Var".to_string(),
                "DISCORD_WEBHOOK_URL".to_string(),
            ),
        ];
        let config = build_messenger_from_fields("discord_webhook", &fields).unwrap();
        match config {
            MessengerProviderConfig::DiscordWebhook {
                webhook_url,
                webhook_url_env,
            } => {
                assert_eq!(
                    webhook_url,
                    Some("https://discord.com/api/webhooks/123/abc".to_string())
                );
                assert_eq!(webhook_url_env, "DISCORD_WEBHOOK_URL");
            }
            _ => panic!("expected DiscordWebhook"),
        }
    }

    #[test]
    fn build_messenger_from_fields_slack_webhook_env_only() {
        let fields = vec![
            ("Webhook URL".to_string(), "".to_string()),
            (
                "Webhook URL Env Var".to_string(),
                "SLACK_WEBHOOK_URL".to_string(),
            ),
        ];
        let config = build_messenger_from_fields("slack_webhook", &fields).unwrap();
        match config {
            MessengerProviderConfig::SlackWebhook {
                webhook_url,
                webhook_url_env,
            } => {
                assert_eq!(webhook_url, None); // blank becomes None
                assert_eq!(webhook_url_env, "SLACK_WEBHOOK_URL");
            }
            _ => panic!("expected SlackWebhook"),
        }
    }

    #[test]
    fn add_modal_includes_webhook_providers() {
        let mut app = test_app();
        handle_key(&mut app, key(KeyCode::Char('a')));

        assert!(matches!(
            app.modal,
            Some(ModalState::MessengerAdd { highlighted: 0 })
        ));

        // The providers array should have 6 entries including webhooks
        // We verify by checking that discord_webhook and slack_webhook
        // can be selected
        if let Some(ModalState::MessengerAdd { .. }) = &app.modal {
            // Move down to discord_webhook (index 1)
            handle_messenger_add_modal(&mut app, key(KeyCode::Down));
            assert_eq!(app.modal_highlighted(), 1);

            // Select it
            handle_messenger_add_modal(&mut app, key(KeyCode::Enter));
            assert!(matches!(
                app.modal,
                Some(ModalState::MessengerInput {
                    provider,
                    field_index: 0,
                    label,
                    is_secret: false,
                    error: None,
                    ..
                }) if provider == "discord_webhook" && label == "Configuration Name"
            ));
        }
    }

    #[test]
    fn webhook_url_validation_rejects_invalid_url() {
        let mut app = test_app();
        // Start a discord_webhook input flow
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 1, // Webhook URL field (index 0 is name, index 1 is URL)
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: "not-a-valid-url".to_string(),
            label: "Webhook URL".to_string(),
            is_secret: true,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

        // Should stay on the same field with an error
        assert!(matches!(
            app.modal,
            Some(ModalState::MessengerInput {
                provider,
                field_index: 1,
                error: Some(_),
                is_secret: true,
                ..
            }) if provider == "discord_webhook"
        ));
    }

    #[test]
    fn webhook_url_validation_accepts_valid_url() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 1,
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: "https://discord.com/api/webhooks/123/abc".to_string(),
            label: "Webhook URL".to_string(),
            is_secret: true,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

        // Should advance to the next field (env var)
        assert!(matches!(
            app.modal,
            Some(ModalState::MessengerInput {
                provider,
                field_index: 2,
                label,
                is_secret: false,
                error: None,
                ..
            }) if provider == "discord_webhook" && label == "Webhook URL Env Var"
        ));
    }

    #[test]
    fn env_only_webhook_skips_url_validation() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "slack_webhook".to_string(),
            field_index: 1,
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: "".to_string(), // empty URL
            label: "Webhook URL".to_string(),
            is_secret: true,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

        // Should advance to env var field since URL is blank
        assert!(matches!(
            app.modal,
            Some(ModalState::MessengerInput {
                provider,
                field_index: 2,
                label,
                is_secret: false,
                error: None,
                ..
            }) if provider == "slack_webhook" && label == "Webhook URL Env Var"
        ));
    }

    #[test]
    fn no_desktop_provider_in_add_modal() {
        let mut app = test_app();
        handle_key(&mut app, key(KeyCode::Char('a')));

        if let Some(ModalState::MessengerAdd { .. }) = &app.modal {
            // Try to find "desktop" in the provider list by scrolling through all
            for i in 0..PROVIDERS.len() {
                app.set_modal_highlighted(i);
                handle_messenger_add_modal(&mut app, key(KeyCode::Enter));
                if let Some(ModalState::MessengerInput { provider, .. }) = &app.modal {
                    assert_ne!(provider, "desktop");
                    // Cancel and reopen
                    app.modal = Some(ModalState::MessengerAdd { highlighted: i });
                }
            }
        }
    }

    // =====================================================================
    // Webhook test connection workflow (Phase 5)
    // =====================================================================

    #[test]
    fn build_test_route_from_modal_uses_current_buffer() {
        let fields = vec![("Configuration Name".to_string(), "test".to_string())];
        let route = build_test_route_from_modal(
            "discord_webhook",
            &fields,
            "https://discord.com/api/webhooks/123/abc",
            1,
        );
        assert!(route.is_some());
        match route.unwrap() {
            MessagingRouteConfig::DiscordWebhook {
                webhook_url,
                webhook_url_env,
            } => {
                assert_eq!(
                    webhook_url,
                    Some("https://discord.com/api/webhooks/123/abc".to_string())
                );
                assert_eq!(webhook_url_env, "DISCORD_WEBHOOK_URL");
            }
            _ => panic!("expected DiscordWebhook"),
        }
    }

    #[test]
    fn build_test_route_from_modal_uses_collected_fields() {
        let fields = vec![
            ("Configuration Name".to_string(), "test".to_string()),
            (
                "Webhook URL".to_string(),
                "https://hooks.slack.com/services/T1/B1/X".to_string(),
            ),
        ];
        let route = build_test_route_from_modal("slack_webhook", &fields, "MY_SLACK_URL", 2);
        assert!(route.is_some());
        match route.unwrap() {
            MessagingRouteConfig::SlackWebhook {
                webhook_url,
                webhook_url_env,
            } => {
                assert_eq!(
                    webhook_url,
                    Some("https://hooks.slack.com/services/T1/B1/X".to_string())
                );
                assert_eq!(webhook_url_env, "MY_SLACK_URL");
            }
            _ => panic!("expected SlackWebhook"),
        }
    }

    #[test]
    fn build_test_route_from_modal_returns_none_for_non_webhook() {
        let fields = vec![("Configuration Name".to_string(), "test".to_string())];
        let route = build_test_route_from_modal("discord", &fields, "123", 1);
        assert!(route.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_connection_key_sets_status_for_webhook() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 1,
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: "not-a-valid-url".to_string(),
            label: "Webhook URL".to_string(),
            is_secret: true,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    test_status: Some(_),
                    ..
                })
            ),
            "expected test_status to be set, got {:?}",
            app.modal
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_connection_key_does_not_mark_dirty() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "slack_webhook".to_string(),
            field_index: 1,
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: "bad-url".to_string(),
            label: "Webhook URL".to_string(),
            is_secret: true,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
        );

        assert!(!app.dirty, "test connection should not mark config dirty");
    }

    #[test]
    fn test_connection_key_ignored_for_non_webhook() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord".to_string(),
            field_index: 1,
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: "123".to_string(),
            label: "Channel ID".to_string(),
            is_secret: false,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    test_status: None,
                    ..
                })
            ),
            "test_status should remain None for non-webhook provider"
        );
    }

    #[test]
    fn test_connection_key_ignored_before_url_field() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 0, // still on config name
            fields: vec![],
            buffer: "test".to_string(),
            label: "Configuration Name".to_string(),
            is_secret: false,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    test_status: None,
                    ..
                })
            ),
            "test_status should remain None before URL field"
        );
    }

    #[test]
    fn typing_clears_test_status() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 1,
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: String::new(),
            label: "Webhook URL".to_string(),
            is_secret: true,
            error: None,
            test_status: Some("previous result".to_string()),
        });

        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    test_status: None,
                    ..
                })
            ),
            "typing should clear test_status"
        );
    }

    #[test]
    fn messenger_render_does_not_expose_raw_webhook_url() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = test_app();
        app.mode = AppMode::Detail;
        app.config.messenger = Some(ClaudineMessengerConfig {
            active_config: Some("alerts".to_string()),
            configurations: std::collections::HashMap::from([
                (
                    "alerts".to_string(),
                    MessengerProviderConfig::DiscordWebhook {
                        webhook_url: Some(
                            "https://discord.com/api/webhooks/123/abc_SECRET_TOKEN_xyz".to_string(),
                        ),
                        webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
                    },
                ),
                (
                    "deploys".to_string(),
                    MessengerProviderConfig::SlackWebhook {
                        webhook_url: Some(
                            "https://hooks.slack.com/services/T00/B00/SECRET_TOKEN_xyz".to_string(),
                        ),
                        webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
                    },
                ),
            ]),
        });

        terminal
            .draw(|f| {
                let area = f.area();
                render(f, area, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();

        assert!(
            !text.contains("https://discord.com/api/webhooks/"),
            "raw Discord webhook URL must not appear in rendered TUI buffer"
        );
        assert!(
            !text.contains("https://hooks.slack.com/services/"),
            "raw Slack webhook URL must not appear in rendered TUI buffer"
        );
        assert!(
            !text.contains("abc_SECRET_TOKEN_xyz"),
            "raw webhook token must not appear in rendered TUI buffer"
        );
        assert!(
            !text.contains("SECRET_TOKEN_xyz"),
            "raw secret text must not appear in rendered TUI buffer"
        );
        // Masking should show asterisks instead
        assert!(
            text.contains("********"),
            "masked placeholder should appear in rendered TUI buffer"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_connection_key_sets_testing_status_immediately() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 1,
            fields: vec![("Configuration Name".to_string(), "test".to_string())],
            buffer: "not-a-valid-url".to_string(),
            label: "Webhook URL".to_string(),
            is_secret: true,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::Char('T'), KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    test_status: Some(status),
                    ..
                }) if status == "Testing…"
            ),
            "test_status should be 'Testing…' immediately after T keypress, got {:?}",
            app.modal
        );

        assert!(
            app.pending_test.is_some(),
            "pending_test receiver should be set after T keypress"
        );

        // Give the background task time to complete and the event loop
        // would poll the receiver on the next iteration.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Simulate what the event loop does: poll the receiver
        if let Some(ref rx) = app.pending_test {
            match rx.try_recv() {
                Ok(result) => {
                    app.pending_test = None;
                    if let Some(ModalState::MessengerInput {
                        test_status, error, ..
                    }) = &mut app.modal
                    {
                        *test_status = Some(match result {
                            Ok(()) => "✓ Test connection successful".to_string(),
                            Err(e) => format!("✗ {}", e),
                        });
                        *error = None;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Task may still be running; that's fine for this test
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    app.pending_test = None;
                }
            }
        }
    }

    #[test]
    fn modal_back_navigation_returns_to_previous_field() {
        let mut app = test_app();
        // Start on the Configuration Name field
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 0,
            fields: vec![],
            buffer: "deploys".to_string(),
            label: "Configuration Name".to_string(),
            is_secret: false,
            error: None,
            test_status: None,
        });

        // Press Enter to advance to Webhook URL field
        handle_messenger_input_modal(&mut app, key(KeyCode::Enter));

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    field_index: 1,
                    label,
                    is_secret: true,
                    ..
                }) if label == "Webhook URL"
            ),
            "should advance to Webhook URL field"
        );

        // Type some characters (avoid 't'/'T' since those trigger test)
        handle_messenger_input_modal(&mut app, key(KeyCode::Char('a')));
        handle_messenger_input_modal(&mut app, key(KeyCode::Char('b')));
        handle_messenger_input_modal(&mut app, key(KeyCode::Char('c')));

        // Press BackTab to go back to Configuration Name
        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    field_index: 0,
                    label,
                    buffer,
                    is_secret: false,
                    ..
                }) if label == "Configuration Name" && buffer == "deploys"
            ),
            "should return to Configuration Name field with previous value, got {:?}",
            app.modal
        );
    }

    #[test]
    fn modal_back_navigation_from_second_provider_field() {
        let mut app = test_app();
        // Simulate being on the env-var field (field_index 2) with both
        // previous fields committed.
        app.modal = Some(ModalState::MessengerInput {
            provider: "discord_webhook".to_string(),
            field_index: 2,
            fields: vec![
                ("Configuration Name".to_string(), "alerts".to_string()),
                (
                    "Webhook URL".to_string(),
                    "https://discord.com/api/webhooks/123/abc".to_string(),
                ),
            ],
            buffer: "DISCORD_WEBHOOK_URL".to_string(),
            label: "Webhook URL Env Var".to_string(),
            is_secret: false,
            error: None,
            test_status: None,
        });

        // Press BackTab to go back to Webhook URL field
        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    field_index: 1,
                    label,
                    buffer,
                    is_secret: true,
                    error: None,
                    test_status: None,
                    ..
                }) if label == "Webhook URL"
                    && buffer == "https://discord.com/api/webhooks/123/abc"
            ),
            "should return to Webhook URL field with secret=true and previous value, got {:?}",
            app.modal
        );

        // The fields vec should now only have Configuration Name
        if let Some(ModalState::MessengerInput { fields, .. }) = &app.modal {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0, "Configuration Name");
            assert_eq!(fields[0].1, "alerts");
        }
    }

    #[test]
    fn modal_back_navigation_ignored_on_first_field() {
        let mut app = test_app();
        app.modal = Some(ModalState::MessengerInput {
            provider: "slack_webhook".to_string(),
            field_index: 0,
            fields: vec![],
            buffer: "test".to_string(),
            label: "Configuration Name".to_string(),
            is_secret: false,
            error: None,
            test_status: None,
        });

        handle_messenger_input_modal(
            &mut app,
            KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE),
        );

        assert!(
            matches!(
                &app.modal,
                Some(ModalState::MessengerInput {
                    field_index: 0,
                    label,
                    buffer,
                    ..
                }) if label == "Configuration Name" && buffer == "test"
            ),
            "should stay on first field when BackTab pressed"
        );
    }

    #[test]
    fn masked_input_render_buffer_masks_secrets() {
        assert_eq!(masked_input::render_buffer("hello", true), "●●●●●");
        assert_eq!(masked_input::render_buffer("hello", false), "hello");
        assert_eq!(masked_input::render_buffer("", true), "");
    }
}
