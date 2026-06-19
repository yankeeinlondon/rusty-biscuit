//! Messenger tab rendering: the tab body, configuration list, and the three
//! messenger modals (select / add / input).

use ratatui::prelude::*;
use ratatui::widgets::*;

use claudine::config::claudine_config::MessengerProviderConfig;

use super::super::super::app::{App, AppMode, ModalState};
use super::super::super::widgets::modal::{
    build_modal_hotkey_line, render_list_modal, render_modal,
};
use super::masked_input;
use super::redaction::MASKED_WEBHOOK_DETAIL;
use super::routes::{PROVIDERS, messenger_fields_with_name};
use super::sorted_messenger_configs;
use super::test_connection::can_test;

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
        render_list_modal(frame, area, title, &items, *highlighted);
    }

    if let Some(ModalState::MessengerAdd { highlighted }) = &app.modal {
        let items: Vec<String> = PROVIDERS.iter().map(|s| s.to_string()).collect();
        render_list_modal(frame, area, "Add Messenger Provider", &items, *highlighted);
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
        render_modal(frame, area, &title, 55, 20, |frame, area| {
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
            let hotkey_line = build_modal_hotkey_line(&hotkeys);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[chunk_idx]);
        });
    }
}
