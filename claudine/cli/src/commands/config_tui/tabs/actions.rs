use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState};
use claudine::actions::HookAction;
use claudine::events::AgenticEvent;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let mut configured_events: Vec<(&AgenticEvent, &Vec<HookAction>)> = app
        .config
        .actions
        .iter()
        .filter(|(_, actions)| !actions.is_empty())
        .collect();
    configured_events.sort_by_key(|(event, _)| event.as_slug());

    if configured_events.is_empty() {
        let text = Paragraph::new("No actions configured. Press 'a' to add an event.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        if is_detail {
            let help = Paragraph::new(" a: add event").style(Style::default().fg(Color::DarkGray));
            let help_area = Rect {
                y: area.y + 1,
                ..area
            };
            frame.render_widget(help, help_area);
        }
        return;
    }

    let items: Vec<ListItem> = configured_events
        .iter()
        .enumerate()
        .map(|(i, (event, actions))| {
            let action_summary = summarize_actions(actions);
            let style = if i == app.list_index && is_detail {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(event.as_slug(), style.add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(
                    action_summary,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol(">> ");
    frame.render_widget(list, area);

    if is_detail && app.modal.is_none() {
        let count = configured_events.len();
        let help_text = if count > 0 {
            " a: add event | d: delete | e: edit | up/down: navigate"
        } else {
            " a: add event"
        };
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
        let help_y = area.y + area.height.saturating_sub(1);
        let help_area = Rect {
            y: help_y,
            height: 1,
            ..area
        };
        frame.render_widget(help, help_area);
    }

    if let Some(ModalState::EventSelector { highlighted }) = &app.modal {
        let existing_events: Vec<AgenticEvent> = app
            .config
            .actions
            .keys()
            .filter(|e| app.config.actions.get(e).map_or(true, |a| a.is_empty()))
            .copied()
            .collect();
        let unconfigured: Vec<AgenticEvent> = AgenticEvent::ALL
            .into_iter()
            .filter(|e| !existing_events.contains(e))
            .collect();
        let items: Vec<String> = if unconfigured.is_empty() {
            vec!["(all events configured)".to_string()]
        } else {
            unconfigured
                .iter()
                .map(|e| e.as_slug().to_string())
                .collect()
        };
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Add Event",
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::ConfirmDelete { event_index }) = &app.modal {
        let events: Vec<(&AgenticEvent, &Vec<HookAction>)> = app
            .config
            .actions
            .iter()
            .filter(|(_, actions)| !actions.is_empty())
            .collect();
        let mut sorted: Vec<_> = events;
        sorted.sort_by_key(|(event, _)| event.as_slug());
        let event_name = sorted
            .get(*event_index)
            .map(|(e, _)| e.as_slug())
            .unwrap_or("?");
        super::super::widgets::modal::render_modal(
            frame,
            area,
            "Confirm Delete",
            40,
            20,
            |frame, area| {
                let text = Paragraph::new(format!(
                    "Delete all actions for {}?\n\n y: confirm | Esc: cancel",
                    event_name
                ))
                .style(Style::default().fg(Color::White));
                frame.render_widget(text, area);
            },
        );
    }
}

fn summarize_actions(actions: &[HookAction]) -> String {
    let mut types: Vec<&str> = Vec::new();
    for action in actions {
        let slug = action.type_pascal_case();
        if !types.contains(&slug) {
            types.push(slug);
        }
    }
    types.join(", ")
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let configured_count = app
        .config
        .actions
        .iter()
        .filter(|(_, a)| !a.is_empty())
        .count();

    match key.code {
        KeyCode::Char('a') | KeyCode::Char('A') => {
            let unconfigured: Vec<AgenticEvent> = AgenticEvent::ALL
                .into_iter()
                .filter(|e| app.config.actions.get(e).map_or(true, |a| a.is_empty()))
                .collect();
            if !unconfigured.is_empty() {
                app.modal = Some(ModalState::EventSelector { highlighted: 0 });
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if configured_count > 0 {
                app.modal = Some(ModalState::ConfirmDelete {
                    event_index: app.list_index,
                });
            }
        }
        KeyCode::Up => {
            if app.list_index > 0 {
                app.list_index -= 1;
            }
        }
        KeyCode::Down => {
            if app.list_index + 1 < configured_count {
                app.list_index += 1;
            }
        }
        _ => {}
    }
}

pub fn handle_event_selector_modal(app: &mut App, key: KeyEvent) {
    let existing_events: Vec<AgenticEvent> = app
        .config
        .actions
        .keys()
        .filter(|e| app.config.actions.get(e).map_or(true, |a| a.is_empty()))
        .copied()
        .collect();
    let unconfigured: Vec<AgenticEvent> = AgenticEvent::ALL
        .into_iter()
        .filter(|e| !existing_events.contains(e))
        .collect();
    let count = unconfigured.len();

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
            if let Some(event) = unconfigured.get(idx) {
                app.config.actions.insert(
                    *event,
                    vec![HookAction::SoundEffect {
                        effect: "doorbell".to_string(),
                        volume: 1.0,
                        speed: 1.0,
                    }],
                );
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

pub fn handle_confirm_delete_modal(app: &mut App, key: KeyEvent) {
    let event_index = match &app.modal {
        Some(ModalState::ConfirmDelete { event_index }) => *event_index,
        _ => return,
    };

    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            let mut event_keys: Vec<AgenticEvent> = app
                .config
                .actions
                .iter()
                .filter(|(_, actions)| !actions.is_empty())
                .map(|(event, _)| *event)
                .collect();
            event_keys.sort_by_key(|event| event.as_slug());
            if let Some(event) = event_keys.get(event_index).copied() {
                app.config.actions.remove(&event);
                app.dirty = true;
            }
            if app.list_index > 0 {
                app.list_index -= 1;
            }
            app.modal = None;
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}
