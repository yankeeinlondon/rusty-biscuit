use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState};
use claudine::config::claudine_config::{ClaudineMessengerConfig, MessengerProviderConfig};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let active_name = app
        .config
        .messenger
        .as_ref()
        .and_then(|m| m.active_config.as_deref())
        .unwrap_or("None");

    let select_style = if is_detail && app.messenger_focus == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else if is_detail {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let add_style = if is_detail && app.messenger_focus == 1 {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else if is_detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let select_line = Line::from(vec![
        Span::styled("Active", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": "),
        Span::styled(format!("[{active_name}]"), select_style),
        Span::raw("  "),
        Span::styled("[+ Add]", add_style),
    ]);
    frame.render_widget(Paragraph::new(select_line), chunks[0]);

    let count = app
        .config
        .messenger
        .as_ref()
        .map(|m| m.configurations.len())
        .unwrap_or(0);
    let detail = Paragraph::new(format!("{count} messenger configuration(s)"))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(detail, chunks[2]);

    if is_detail && app.modal.is_none() {
        let edit_indicator = if app.messenger_focus == 2 { ">" } else { " " };
        let help = Paragraph::new(format!(
            " Tab/Shift-Tab: focus | Enter: activate | {edit_indicator}Edit active config"
        ))
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[2]);
    }

    if let Some(ModalState::MessengerSelect { highlighted }) = &app.modal {
        let configs: Vec<String> = app
            .config
            .messenger
            .as_ref()
            .map(|m| m.configurations.keys().cloned().collect())
            .unwrap_or_default();
        let mut items = vec!["(none)".to_string()];
        items.extend(configs);
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Select Active Messenger",
            &items,
            *highlighted,
        );
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
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            app.messenger_focus = (app.messenger_focus + 1) % 3;
        }
        KeyCode::BackTab => {
            app.messenger_focus = (app.messenger_focus + 2) % 3;
        }
        KeyCode::Enter => match app.messenger_focus {
            0 => {
                app.modal = Some(ModalState::MessengerSelect { highlighted: 0 });
            }
            1 => {
                app.modal = Some(ModalState::MessengerAdd { highlighted: 0 });
            }
            2 => {
                if let Some(name) = app
                    .config
                    .messenger
                    .as_ref()
                    .and_then(|m| m.active_config.clone())
                {
                    app.modal = Some(ModalState::MessengerEdit {
                        config_name: name,
                        field_index: 0,
                    });
                }
            }
            _ => {}
        },
        _ => {}
    }
}

pub fn handle_messenger_select_modal(app: &mut App, key: KeyEvent) {
    let configs: Vec<String> = app
        .config
        .messenger
        .as_ref()
        .map(|m| m.configurations.keys().cloned().collect())
        .unwrap_or_default();
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
                ensure_messenger_config(app);
                let name = provider.to_string();
                let config = match *provider {
                    "discord" => MessengerProviderConfig::Discord {
                        channel_id: String::new(),
                        bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                    },
                    "slack" => MessengerProviderConfig::Slack {
                        channel_id: String::new(),
                        bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                    },
                    "signal" => MessengerProviderConfig::Signal {
                        recipient: String::new(),
                        rpc_url_env: "SIGNAL_RPC_URL".to_string(),
                        account_env: "SIGNAL_ACCOUNT".to_string(),
                    },
                    "whatsapp" => MessengerProviderConfig::Whatsapp {
                        recipient: String::new(),
                        access_token_env: "WHATSAPP_ACCESS_TOKEN".to_string(),
                        phone_number_id_env: "WHATSAPP_PHONE_NUMBER_ID".to_string(),
                    },
                    _ => {
                        app.modal = None;
                        return;
                    }
                };
                if let Some(ref mut messenger) = app.config.messenger {
                    messenger.configurations.insert(name.clone(), config);
                    messenger.active_config = Some(name);
                }
                app.dirty = true;
            }
            app.modal = None;
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}

pub fn handle_messenger_edit_modal(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Esc {
        app.modal = None;
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
