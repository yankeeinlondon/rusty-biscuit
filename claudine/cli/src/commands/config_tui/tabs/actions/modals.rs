use claudine::actions::HookAction;
use claudine::events::AgenticEvent;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::super::app::{ActionView, App, AppMode, ModalState};
use super::entries::{
    action_entries_for_view, configured_event_count, current_actions_map, current_actions_map_mut,
    get_unconfigured_events, mark_current_actions_dirty, switch_effective_selection_to_source_view,
};
use super::fields::{apply_action_field, get_action_fields};
use super::summary::{format_action_detail, summarize_actions, truncate_str};
use super::{ACTION_TYPE_LABELS, ActionSource};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;
    let entries = action_entries_for_view(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let header = Line::from(vec![
        Span::styled("View", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": "),
        Span::styled(
            app.actions_view.label(),
            if app.is_in_repo && app.actions_view == ActionView::Effective {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            },
        ),
        Span::styled(
            if app.is_in_repo {
                "  U user  R repo  V effective"
            } else {
                ""
            },
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), chunks[0]);

    if entries.is_empty() {
        let text = Paragraph::new(
            if app.is_in_repo && app.actions_view == ActionView::Effective {
                "No actions configured. Switch to User or Repo view to add one."
            } else {
                "No actions configured. Press 'A' to add an event."
            },
        )
        .style(Style::default().fg(Color::Gray));
        frame.render_widget(text, chunks[1]);
    } else {
        let max_summary_width = chunks[1].width.saturating_sub(32) as usize;
        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let action_summary = summarize_actions(&entry.actions, max_summary_width);
                let name_style = if i == app.list_index && is_detail {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().add_modifier(Modifier::BOLD)
                };
                let mut spans = vec![Span::styled(entry.event.human_name(), name_style)];
                if app.is_in_repo {
                    spans.push(Span::styled(
                        format!(" [{}]", entry.source.badge()),
                        Style::default().fg(match entry.source {
                            ActionSource::User => Color::Blue,
                            ActionSource::Repo => Color::Magenta,
                        }),
                    ));
                }
                spans.extend([
                    Span::raw(": "),
                    Span::styled(
                        action_summary,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]);
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().fg(Color::Yellow))
            .highlight_symbol(">> ");
        frame.render_widget(list, chunks[1]);
    }

    for stacked in &app.modal_stack {
        if let ModalState::EditActions { event, highlighted } = stacked {
            render_edit_actions_modal(frame, area, app, *event, *highlighted);
        }
    }

    if let Some(ModalState::EditActions { event, highlighted }) = &app.modal {
        render_edit_actions_modal(frame, area, app, *event, *highlighted);
    }

    if let Some(ModalState::EventSelector { highlighted }) = &app.modal {
        let unconfigured = get_unconfigured_events(app);
        let items: Vec<String> = if unconfigured.is_empty() {
            vec!["(all events configured)".to_string()]
        } else {
            unconfigured
                .iter()
                .map(|e| e.human_name().to_string())
                .collect()
        };
        super::super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Add Event",
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::ActionTypeChooser { event, highlighted }) = &app.modal {
        let items: Vec<String> = ACTION_TYPE_LABELS.iter().map(|s| s.to_string()).collect();
        let title = format!("Add Action to: {}", event.human_name());
        super::super::super::widgets::modal::render_list_modal(
            frame, area, &title, &items, *highlighted,
        );
    }

    if let Some(ModalState::ActionSoundSelector { highlighted, .. }) = &app.modal {
        let sounds = super::super::super::get_sound_effect_names();
        let items: Vec<String> = sounds.iter().map(|s| s.to_string()).collect();
        super::super::super::widgets::modal::render_list_modal_with_hotkeys(
            frame,
            area,
            "Select Sound Effect",
            &items,
            *highlighted,
            &[("P", "Play"), ("ENTER", "Select"), ("ESC", "Cancel")],
        );
    }

    if let Some(ModalState::ConfirmDelete { event_index }) = &app.modal {
        let sorted = action_entries_for_view(app);
        let event_name = sorted
            .get(*event_index)
            .map(|entry| entry.event.human_name())
            .unwrap_or("?");
        super::super::super::widgets::modal::render_modal(
            frame,
            area,
            "Confirm Delete",
            40,
            25,
            |frame, area| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(area);

                let msg = Paragraph::new(format!(
                    "Delete all {} actions for {}?",
                    app.actions_view.label().to_lowercase(),
                    event_name
                ))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);
                frame.render_widget(msg, chunks[1]);

                let hotkey_line = super::super::super::widgets::modal::build_modal_hotkey_line(&[
                    ("Y", "Confirm"),
                    ("N", "Cancel"),
                ]);
                let hotkey_widget = Paragraph::new(hotkey_line)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(Color::Indexed(236)));
                frame.render_widget(hotkey_widget, chunks[3]);
            },
        );
    }

    if let Some(ModalState::ActionFieldList {
        event,
        action_index,
        highlighted,
    }) = &app.modal
        && let Some(actions) = current_actions_map(app).and_then(|actions| actions.get(event))
        && let Some(action) = actions.get(*action_index)
    {
        let fields = get_action_fields(action);
        let title = format!("Edit {} Fields", action.type_pascal_case());
        super::super::super::widgets::modal::render_modal(
            frame,
            area,
            &title,
            55,
            50,
            |frame, area| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(0),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .split(area);

                let items: Vec<ListItem> = fields
                    .iter()
                    .enumerate()
                    .map(|(i, (_, label, value))| {
                        let display_value = if value.is_empty() {
                            "(empty)".to_string()
                        } else {
                            truncate_str(value, 30)
                        };
                        let style = if i == *highlighted {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        let value_style = if value.is_empty() {
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::ITALIC)
                        } else {
                            Style::default().fg(Color::Cyan)
                        };
                        ListItem::new(Line::from(vec![
                            Span::styled(format!("{label}: "), style),
                            Span::styled(display_value, value_style),
                        ]))
                    })
                    .collect();

                let list = List::new(items);
                frame.render_widget(list, chunks[0]);

                let hotkey_line =
                    super::super::super::widgets::modal::build_modal_hotkey_line(&[
                        ("ENTER", "Edit"),
                        ("ESC", "Done"),
                    ]);
                let hotkey_widget = Paragraph::new(hotkey_line)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(Color::Indexed(236)));
                frame.render_widget(hotkey_widget, chunks[2]);
            },
        );
    }

    if let Some(ModalState::ActionFieldInput { label, buffer, .. }) = &app.modal {
        super::super::super::widgets::modal::render_modal(
            frame,
            area,
            label,
            60,
            20,
            |frame, area| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(area);

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
                frame.render_widget(Paragraph::new(input_line), chunks[0]);

                let hotkey_line =
                    super::super::super::widgets::modal::build_modal_hotkey_line(&[
                        ("ENTER", "Confirm"),
                        ("ESC", "Cancel"),
                    ]);
                let hotkey_widget = Paragraph::new(hotkey_line)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(Color::Indexed(236)));
                frame.render_widget(hotkey_widget, chunks[2]);
            },
        );
    }

    if let Some(ModalState::TextInput { label, buffer, .. }) = &app.modal {
        super::super::super::widgets::modal::render_modal(
            frame,
            area,
            label,
            60,
            20,
            |frame, area| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(0),
                    ])
                    .split(area);

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
                frame.render_widget(Paragraph::new(input_line), chunks[0]);

                let hotkey_line =
                    super::super::super::widgets::modal::build_modal_hotkey_line(&[
                        ("ENTER", "Confirm"),
                        ("ESC", "Cancel"),
                    ]);
                let hotkey_widget = Paragraph::new(hotkey_line)
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(Color::Indexed(236)));
                frame.render_widget(hotkey_widget, chunks[2]);
            },
        );
    }
}

fn render_edit_actions_modal(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    event: AgenticEvent,
    highlighted: usize,
) {
    let actions = current_actions_map(app)
        .and_then(|actions| actions.get(&event))
        .cloned()
        .unwrap_or_default();
    let title = format!("Edit {}: {}", app.actions_view.label(), event.human_name());

    super::super::super::widgets::modal::render_modal(
        frame,
        area,
        &title,
        55,
        70,
        |frame, area| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(area);

            if actions.is_empty() {
                let msg = Paragraph::new("No actions configured.")
                    .style(Style::default().fg(Color::Gray));
                frame.render_widget(msg, chunks[0]);
            } else {
                let items: Vec<ListItem> = actions
                    .iter()
                    .enumerate()
                    .map(|(i, action)| {
                        let summary = format_action_detail(action);
                        let style = if i == highlighted {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        ListItem::new(Line::from(Span::styled(summary, style)))
                    })
                    .collect();

                let list = List::new(items).highlight_symbol(">> ").highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );
                let mut state = ListState::default().with_selected(Some(highlighted));
                frame.render_stateful_widget(list, chunks[0], &mut state);
            }

            let hotkey_line = super::super::super::widgets::modal::build_modal_hotkey_line(&[
                ("ENTER", "Edit"),
                ("A", "Add"),
                ("D", "Delete"),
                ("ESC", "Done"),
            ]);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[2]);
        },
    );
}

pub fn handle_edit_actions_modal(app: &mut App, key: KeyEvent) {
    let event = match &app.modal {
        Some(ModalState::EditActions { event, .. }) => *event,
        _ => return,
    };
    let action_count = current_actions_map(app)
        .and_then(|actions| actions.get(&event))
        .map_or(0, |actions| actions.len());

    match key.code {
        KeyCode::Up => {
            let idx = app.modal_highlighted();
            if idx > 0 {
                app.set_modal_highlighted(idx - 1);
            }
        }
        KeyCode::Down => {
            let idx = app.modal_highlighted();
            if action_count > 0 && idx + 1 < action_count {
                app.set_modal_highlighted(idx + 1);
            }
        }
        KeyCode::Enter if action_count > 0 => {
            let idx = app.modal_highlighted();
            app.push_modal(ModalState::ActionFieldList {
                event,
                action_index: idx,
                highlighted: 0,
            });
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.push_modal(ModalState::ActionTypeChooser {
                event,
                highlighted: 0,
            });
        }
        KeyCode::Char('d') | KeyCode::Char('D') if action_count > 0 => {
            let idx = app.modal_highlighted();
            let mut remove_event = false;
            if let Some(actions) =
                current_actions_map_mut(app).and_then(|actions| actions.get_mut(&event))
            {
                if idx < actions.len() {
                    actions.remove(idx);
                }
                if actions.is_empty() {
                    remove_event = true;
                }
            }
            if remove_event && let Some(actions) = current_actions_map_mut(app) {
                actions.remove(&event);
                mark_current_actions_dirty(app);
                app.pop_modal();
                return;
            }
            mark_current_actions_dirty(app);
            let new_count = current_actions_map(app)
                .and_then(|actions| actions.get(&event))
                .map_or(0, |actions| actions.len());
            if app.modal_highlighted() >= new_count && new_count > 0 {
                app.set_modal_highlighted(new_count - 1);
            }
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let configured_count = configured_event_count(app);

    match key.code {
        KeyCode::Char('u') | KeyCode::Char('U') if app.is_in_repo => {
            app.actions_view = ActionView::User;
            app.list_index = 0;
        }
        KeyCode::Char('r') | KeyCode::Char('R') if app.is_in_repo => {
            app.actions_view = ActionView::Repo;
            app.list_index = 0;
        }
        KeyCode::Char('v') | KeyCode::Char('V') if app.is_in_repo => {
            app.actions_view = ActionView::Effective;
            app.list_index = 0;
        }
        KeyCode::Char('a') | KeyCode::Char('A') if app.actions_view != ActionView::Effective => {
            let unconfigured = get_unconfigured_events(app);
            if !unconfigured.is_empty() {
                app.modal = Some(ModalState::EventSelector { highlighted: 0 });
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') if configured_count > 0 => {
            switch_effective_selection_to_source_view(app);
            app.modal = Some(ModalState::ConfirmDelete {
                event_index: app.list_index,
            });
        }
        KeyCode::Enter | KeyCode::Char('e') | KeyCode::Char('E') if configured_count > 0 => {
            switch_effective_selection_to_source_view(app);
            let event_keys: Vec<AgenticEvent> = action_entries_for_view(app)
                .into_iter()
                .map(|entry| entry.event)
                .collect();
            if let Some(event) = event_keys.get(app.list_index).copied() {
                app.modal = Some(ModalState::EditActions {
                    event,
                    highlighted: 0,
                });
            }
        }
        KeyCode::Up if app.list_index > 0 => {
            app.list_index -= 1;
        }
        KeyCode::Down if app.list_index + 1 < configured_count => {
            app.list_index += 1;
        }
        _ => {}
    }
}

pub fn handle_event_selector_modal(app: &mut App, key: KeyEvent) {
    let unconfigured = get_unconfigured_events(app);
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
            if let Some(event) = unconfigured.get(idx).copied() {
                app.push_modal(ModalState::ActionTypeChooser {
                    event,
                    highlighted: 0,
                });
            }
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}

pub fn handle_action_type_chooser_modal(app: &mut App, key: KeyEvent) {
    let (event, _) = match &app.modal {
        Some(ModalState::ActionTypeChooser { event, highlighted }) => (*event, *highlighted),
        _ => return,
    };
    let count = ACTION_TYPE_LABELS.len();

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
            match idx {
                0 => {
                    app.push_modal(ModalState::ActionSoundSelector {
                        event,
                        highlighted: 0,
                        edit_index: None,
                    });
                }
                1 => {
                    app.push_modal(ModalState::TextInput {
                        event,
                        action_type: 1,
                        buffer: String::new(),
                        label: "Speak Message".to_string(),
                        edit_index: None,
                    });
                }
                2 => {
                    app.push_modal(ModalState::TextInput {
                        event,
                        action_type: 2,
                        buffer: String::new(),
                        label: "Message Text".to_string(),
                        edit_index: None,
                    });
                }
                3 => {
                    app.push_modal(ModalState::TextInput {
                        event,
                        action_type: 3,
                        buffer: String::new(),
                        label: "Shell Command".to_string(),
                        edit_index: None,
                    });
                }
                4 => {
                    let action = HookAction::Report {
                        handler: None,
                        when: None,
                    };
                    if let Some(actions) = current_actions_map_mut(app) {
                        actions.entry(event).or_default().push(action);
                        mark_current_actions_dirty(app);
                    }
                    app.pop_to_edit_actions();
                }
                5 => {
                    app.push_modal(ModalState::TextInput {
                        event,
                        action_type: 5,
                        buffer: String::new(),
                        label: "Call Command".to_string(),
                        edit_index: None,
                    });
                }
                _ => {
                    app.pop_modal();
                }
            }
        }
        KeyCode::Esc => {
            app.pop_modal();
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
            let event_keys: Vec<AgenticEvent> = action_entries_for_view(app)
                .into_iter()
                .map(|entry| entry.event)
                .collect();
            if let Some(event) = event_keys.get(event_index).copied()
                && let Some(actions) = current_actions_map_mut(app)
            {
                actions.remove(&event);
                mark_current_actions_dirty(app);
            }
            if app.list_index > 0 {
                app.list_index -= 1;
            }
            app.pop_modal();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}

pub fn handle_text_input_modal(app: &mut App, key: KeyEvent) {
    let (event, action_type, edit_index, buffer) = match &mut app.modal {
        Some(ModalState::TextInput {
            event,
            action_type,
            buffer,
            edit_index,
            ..
        }) => (*event, *action_type, *edit_index, buffer),
        _ => return,
    };

    match key.code {
        KeyCode::Char(c) => {
            buffer.push(c);
        }
        KeyCode::Backspace => {
            buffer.pop();
        }
        KeyCode::Enter => {
            let text = buffer.clone();
            if text.is_empty() {
                return;
            }

            if let Some(idx) = edit_index {
                if let Some(actions) =
                    current_actions_map_mut(app).and_then(|actions| actions.get_mut(&event))
                    && let Some(action) = actions.get_mut(idx)
                {
                    match action {
                        HookAction::SoundEffect { effect, .. } => *effect = text,
                        HookAction::Speak { message, .. } => *message = text,
                        HookAction::Message { message, .. } => *message = text,
                        HookAction::Bash { command, .. } => *command = text,
                        HookAction::Call { command, .. } => *command = text,
                        _ => {}
                    }
                }
            } else {
                let action = match action_type {
                    1 => HookAction::Speak {
                        message: text,
                        voice: None,
                        gender: None,
                        when: None,
                    },
                    2 => HookAction::Message {
                        message: text,
                        image: None,
                        when: None,
                    },
                    3 => HookAction::Bash {
                        command: text,
                        params: String::new(),
                        when: None,
                    },
                    5 => HookAction::Call {
                        command: text,
                        args: None,
                        timeout_ms: None,
                        mapper: None,
                        when: None,
                    },
                    _ => {
                        app.pop_modal();
                        return;
                    }
                };
                if let Some(actions) = current_actions_map_mut(app) {
                    actions.entry(event).or_default().push(action);
                }
            }
            mark_current_actions_dirty(app);
            app.pop_to_edit_actions();
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}

pub fn handle_action_field_list_modal(app: &mut App, key: KeyEvent) {
    let (event, action_index) = match &app.modal {
        Some(ModalState::ActionFieldList {
            event,
            action_index,
            ..
        }) => (*event, *action_index),
        _ => return,
    };

    let field_count = current_actions_map(app)
        .and_then(|actions| actions.get(&event))
        .and_then(|a| a.get(action_index))
        .map(|a| get_action_fields(a).len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Up => {
            let idx = app.modal_highlighted();
            if idx > 0 {
                app.set_modal_highlighted(idx - 1);
            }
        }
        KeyCode::Down => {
            let idx = app.modal_highlighted();
            if idx + 1 < field_count {
                app.set_modal_highlighted(idx + 1);
            }
        }
        KeyCode::Enter => {
            let idx = app.modal_highlighted();
            if let Some(actions) = current_actions_map(app).and_then(|actions| actions.get(&event))
                && let Some(action) = actions.get(action_index)
            {
                let fields = get_action_fields(action);
                if let Some((name, label, current_value)) = fields.get(idx) {
                    if *name == "effect" && matches!(action, HookAction::SoundEffect { .. }) {
                        let sounds = super::super::super::get_sound_effect_names();
                        let highlighted = sounds
                            .iter()
                            .position(|s| *s == current_value.as_str())
                            .unwrap_or(0);
                        app.push_modal(ModalState::ActionSoundSelector {
                            event,
                            highlighted,
                            edit_index: Some(action_index),
                        });
                    } else {
                        app.push_modal(ModalState::ActionFieldInput {
                            event,
                            action_index,
                            field_name: name.to_string(),
                            buffer: current_value.clone(),
                            label: label.to_string(),
                        });
                    }
                }
            }
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}

pub fn handle_action_field_input_modal(app: &mut App, key: KeyEvent) {
    let (event, action_index, field_name, buffer) = match &mut app.modal {
        Some(ModalState::ActionFieldInput {
            event,
            action_index,
            field_name,
            buffer,
            ..
        }) => (*event, *action_index, field_name.clone(), buffer),
        _ => return,
    };

    match key.code {
        KeyCode::Char(c) => {
            buffer.push(c);
        }
        KeyCode::Backspace => {
            buffer.pop();
        }
        KeyCode::Enter => {
            let value = buffer.clone();
            if let Some(actions) =
                current_actions_map_mut(app).and_then(|actions| actions.get_mut(&event))
                && let Some(action) = actions.get_mut(action_index)
            {
                apply_action_field(action, &field_name, value);
                mark_current_actions_dirty(app);
            }
            app.pop_modal();
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}

pub fn handle_action_sound_selector_modal(app: &mut App, key: KeyEvent) {
    let (event, edit_index) = match &app.modal {
        Some(ModalState::ActionSoundSelector {
            event, edit_index, ..
        }) => (*event, *edit_index),
        _ => return,
    };
    let sounds = super::super::super::get_sound_effect_names();
    let count = sounds.len();

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
        KeyCode::Char('p') | KeyCode::Char('P') => {
            let idx = app.modal_highlighted();
            if let Some(effect) = playa::SoundEffect::from_name(sounds[idx]) {
                std::thread::spawn(move || {
                    let _ = effect.play();
                });
            }
        }
        KeyCode::Enter => {
            let idx = app.modal_highlighted();
            if let Some(&effect_name) = sounds.get(idx) {
                if let Some(edit_idx) = edit_index {
                    if let Some(actions) =
                        current_actions_map_mut(app).and_then(|actions| actions.get_mut(&event))
                        && let Some(action) = actions.get_mut(edit_idx)
                        && let HookAction::SoundEffect { effect, .. } = action
                    {
                        *effect = effect_name.to_string();
                    }
                } else {
                    let action = HookAction::SoundEffect {
                        effect: effect_name.to_string(),
                        volume: 1.0,
                        speed: 1.0,
                        when: None,
                    };
                    if let Some(actions) = current_actions_map_mut(app) {
                        actions.entry(event).or_default().push(action);
                    }
                }
                mark_current_actions_dirty(app);
                app.pop_to_edit_actions();
            }
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}
