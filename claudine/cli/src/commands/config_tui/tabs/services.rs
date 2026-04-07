use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState};
use super::super::widgets::toggle::Toggle;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

    let logging_toggle = Toggle::new("Logging", app.config.logging, is_detail);
    frame.render_widget(logging_toggle, chunks[0]);

    let protect_enabled = app.config.protect.enabled;
    let protect_status = if protect_enabled {
        "enabled".to_string()
    } else {
        "disabled".to_string()
    };

    let protect_line = Line::from(vec![
        Span::styled("Protect", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(
            if protect_enabled { "ON " } else { "OFF" },
            if protect_enabled {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            },
        ),
        Span::raw("  "),
        Span::styled(
            protect_status,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]);
    frame.render_widget(Paragraph::new(protect_line), chunks[2]);

    if is_detail {
        let help =
            Paragraph::new(" l: toggle Logging | p: toggle Protect | c: configure Protect rules")
                .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[3]);
    }

    if let Some(ModalState::ProtectRules { highlighted }) = &app.modal {
        render_protect_rules_modal(frame, area, app, *highlighted);
    }
}

fn render_protect_rules_modal(frame: &mut Frame, area: Rect, app: &App, highlighted: usize) {
    let rule_names = super::super::get_protect_rule_names();
    let enabled_rules = &app.config.protect.rules;

    super::super::widgets::modal::render_modal(
        frame,
        area,
        "Protect Rules",
        50,
        70,
        |frame, area| {
            let items: Vec<ListItem> = rule_names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let is_enabled = super::super::is_protect_rule_enabled(enabled_rules, name);
                    let check = if is_enabled { "[x]" } else { "[ ]" };
                    let style = if i == highlighted {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(Line::from(Span::styled(format!("{check} {name}"), style)))
                })
                .collect();

            let list = List::new(items);
            frame.render_widget(list, area);
        },
    );
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.config.logging = !app.config.logging;
            app.dirty = true;
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.config.protect.enabled = !app.config.protect.enabled;
            app.dirty = true;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.modal = Some(ModalState::ProtectRules { highlighted: 0 });
        }
        _ => {}
    }
}

pub fn handle_protect_rules_modal(app: &mut App, key: KeyEvent) {
    let rule_names = super::super::get_protect_rule_names();
    let count = rule_names.len();
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
        KeyCode::Char(' ') => {
            let idx = app.modal_highlighted();
            if let Some(name) = rule_names.get(idx) {
                super::super::toggle_protect_rule(&mut app.config.protect.rules, name);
                app.dirty = true;
            }
        }
        KeyCode::Enter | KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}
