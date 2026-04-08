use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState};
use claudine::actions::HookAction;
use claudine::events::AgenticEvent;

/// Action types a user can add to an event.
const ACTION_TYPE_LABELS: &[&str] = &[
    "Sound Effect",
    "Speak (using TTS provider)",
    "Message (to chat app)",
    "Shell Command",
    "Report (to STDOUT)",
    "Call (synchronous with response)",
];

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
        let text = Paragraph::new("No actions configured. Press 'A' to add an event.")
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(text, area);
    } else {
        let max_summary_width = area.width.saturating_sub(25) as usize; // leave room for event name
        let items: Vec<ListItem> = configured_events
            .iter()
            .enumerate()
            .map(|(i, (event, actions))| {
                let action_summary = summarize_actions(actions, max_summary_width);
                let name_style = if i == app.list_index && is_detail {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().add_modifier(Modifier::BOLD)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(event.human_name(), name_style),
                    Span::raw(": "),
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
    }

    // Render parent modals from the stack underneath the current modal
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
            unconfigured.iter().map(|e| e.human_name().to_string()).collect()
        };
        super::super::widgets::modal::render_list_modal(
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
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            &title,
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::ActionSoundSelector { highlighted, .. }) = &app.modal {
        let sounds = super::super::get_sound_effect_names();
        let items: Vec<String> = sounds.iter().map(|s| s.to_string()).collect();
        super::super::widgets::modal::render_list_modal_with_hotkeys(
            frame,
            area,
            "Select Sound Effect",
            &items,
            *highlighted,
            &[("P", "Play"), ("ENTER", "Select"), ("ESC", "Cancel")],
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
            .map(|(e, _)| e.human_name())
            .unwrap_or("?");
        super::super::widgets::modal::render_modal(
            frame,
            area,
            "Confirm Delete",
            40,
            25,
            |frame, area| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // top padding
                        Constraint::Length(1), // message
                        Constraint::Length(1), // blank
                        Constraint::Length(1), // hotkeys (centered)
                        Constraint::Length(1), // bottom padding
                        Constraint::Min(0),
                    ])
                    .split(area);

                let msg = Paragraph::new(format!("Delete all actions for {}?", event_name))
                    .style(Style::default().fg(Color::White))
                    .alignment(Alignment::Center);
                frame.render_widget(msg, chunks[1]);

                let hotkey_line =
                    super::super::widgets::modal::build_modal_hotkey_line(&[
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
    {
        if let Some(actions) = app.config.actions.get(event)
            && let Some(action) = actions.get(*action_index)
        {
            let fields = get_action_fields(action);
            let title = format!("Edit {} Fields", action.type_pascal_case());
            super::super::widgets::modal::render_modal(
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
                                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
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

                    let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
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
    }

    if let Some(ModalState::ActionFieldInput {
        label, buffer, ..
    }) = &app.modal
    {
        super::super::widgets::modal::render_modal(
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
                    super::super::widgets::modal::build_modal_hotkey_line(&[
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

    if let Some(ModalState::TextInput {
        label, buffer, ..
    }) = &app.modal
    {
        super::super::widgets::modal::render_modal(
            frame,
            area,
            label,
            60,
            20,
            |frame, area| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // input line
                        Constraint::Length(1), // blank
                        Constraint::Length(1), // hotkeys
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
                    super::super::widgets::modal::build_modal_hotkey_line(&[
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

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

fn summarize_actions(actions: &[HookAction], max_width: usize) -> String {
    let summaries: Vec<String> = actions
        .iter()
        .map(|action| match action {
            HookAction::SoundEffect { effect, .. } => format!("Sound({effect})"),
            HookAction::Speak { message, .. } => {
                let preview = truncate_str(message, 20);
                format!("Speak(\"{preview}\")")
            }
            HookAction::Message { message, .. } => {
                let preview = truncate_str(message, 20);
                format!("Message(\"{preview}\")")
            }
            HookAction::Bash { command, .. } => {
                let preview = truncate_str(command, 20);
                format!("Shell(\"{preview}\")")
            }
            HookAction::Report { .. } => "Report".to_string(),
            HookAction::Call { command, .. } => format!("Call({command})"),
            _ => action.type_pascal_case().to_string(),
        })
        .collect();
    let joined = summaries.join(", ");
    truncate_str(&joined, max_width)
}

fn get_unconfigured_events(app: &App) -> Vec<AgenticEvent> {
    AgenticEvent::ALL
        .into_iter()
        .filter(|e| app.config.actions.get(e).is_none_or(|a| a.is_empty()))
        .collect()
}

fn render_edit_actions_modal(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    event: AgenticEvent,
    highlighted: usize,
) {
    let actions = app
        .config
        .actions
        .get(&event)
        .cloned()
        .unwrap_or_default();
    let title = format!("Edit: {}", event.human_name());

    super::super::widgets::modal::render_modal(
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
                    Constraint::Length(1), // blank separator
                    Constraint::Length(1), // hotkey bar
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

                let list = List::new(items)
                    .highlight_symbol(">> ")
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                let mut state = ListState::default().with_selected(Some(highlighted));
                frame.render_stateful_widget(list, chunks[0], &mut state);
            }

            let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
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

/// Returns (label, current_value, action_type) for editable actions.
fn editable_text(action: &HookAction) -> Option<(String, String, usize)> {
    match action {
        HookAction::SoundEffect { effect, .. } => {
            Some(("Sound Effect".to_string(), effect.clone(), 0))
        }
        HookAction::Speak { message, .. } => {
            Some(("Speak Message".to_string(), message.clone(), 1))
        }
        HookAction::Message { message, .. } => {
            Some(("Message Text".to_string(), message.clone(), 2))
        }
        HookAction::Bash { command, .. } => {
            Some(("Shell Command".to_string(), command.clone(), 3))
        }
        HookAction::Report { .. } => None,
        HookAction::Call { command, .. } => {
            Some(("Call Command".to_string(), command.clone(), 5))
        }
        _ => None,
    }
}

fn format_action_detail(action: &HookAction) -> String {
    match action {
        HookAction::SoundEffect { effect, .. } => format!("Sound Effect: {effect}"),
        HookAction::Speak { message, .. } => format!("Speak: \"{}\"", truncate_str(message, 40)),
        HookAction::Message { message, .. } => {
            format!("Message: \"{}\"", truncate_str(message, 40))
        }
        HookAction::Bash { command, .. } => {
            format!("Shell Command: \"{}\"", truncate_str(command, 40))
        }
        HookAction::Report { .. } => "Report (to STDOUT)".to_string(),
        HookAction::Call { command, .. } => format!("Call: {command}"),
        _ => action.type_pascal_case().to_string(),
    }
}

pub fn handle_edit_actions_modal(app: &mut App, key: KeyEvent) {
    let event = match &app.modal {
        Some(ModalState::EditActions { event, .. }) => *event,
        _ => return,
    };
    let action_count = app
        .config
        .actions
        .get(&event)
        .map_or(0, |a| a.len());

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
        KeyCode::Enter => {
            if action_count > 0 {
                let idx = app.modal_highlighted();
                // Open the multi-field editor so all action properties are
                // accessible, not just the primary text field.
                app.push_modal(ModalState::ActionFieldList {
                    event,
                    action_index: idx,
                    highlighted: 0,
                });
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.push_modal(ModalState::ActionTypeChooser {
                event,
                highlighted: 0,
            });
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if action_count > 0 {
                let idx = app.modal_highlighted();
                if let Some(actions) = app.config.actions.get_mut(&event) {
                    if idx < actions.len() {
                        actions.remove(idx);
                        app.dirty = true;
                    }
                    if actions.is_empty() {
                        app.config.actions.remove(&event);
                        app.pop_modal();
                        return;
                    }
                }
                let new_count = app.config.actions.get(&event).map_or(0, |a| a.len());
                if app.modal_highlighted() >= new_count && new_count > 0 {
                    app.set_modal_highlighted(new_count - 1);
                }
            }
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
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
            let unconfigured = get_unconfigured_events(app);
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
        KeyCode::Enter => {
            if configured_count > 0 {
                let mut event_keys: Vec<AgenticEvent> = app
                    .config
                    .actions
                    .iter()
                    .filter(|(_, actions)| !actions.is_empty())
                    .map(|(event, _)| *event)
                    .collect();
                event_keys.sort_by_key(|event| event.as_slug());
                if let Some(event) = event_keys.get(app.list_index).copied() {
                    app.modal = Some(ModalState::EditActions {
                        event,
                        highlighted: 0,
                    });
                }
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
                    // Sound Effect - open sound selector
                    app.push_modal(ModalState::ActionSoundSelector {
                        event,
                        highlighted: 0,
                        edit_index: None,
                    });
                }
                1 => {
                    // Speak - need text input
                    app.push_modal(ModalState::TextInput {
                        event,
                        action_type: 1,
                        buffer: String::new(),
                        label: "Speak Message".to_string(),
                        edit_index: None,
                    });
                }
                2 => {
                    // Message - need text input
                    app.push_modal(ModalState::TextInput {
                        event,
                        action_type: 2,
                        buffer: String::new(),
                        label: "Message Text".to_string(),
                        edit_index: None,
                    });
                }
                3 => {
                    // Shell Command - need text input
                    app.push_modal(ModalState::TextInput {
                        event,
                        action_type: 3,
                        buffer: String::new(),
                        label: "Shell Command".to_string(),
                        edit_index: None,
                    });
                }
                4 => {
                    // Report
                    let action = HookAction::Report { handler: None };
                    app.config
                        .actions
                        .entry(event)
                        .or_default()
                        .push(action);
                    app.dirty = true;
                    app.pop_to_edit_actions();
                }
                5 => {
                    // Call - need command text input
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
                // Editing an existing action in place
                if let Some(actions) = app.config.actions.get_mut(&event)
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
                // Adding a new action
                let action = match action_type {
                    1 => HookAction::Speak {
                        message: text,
                        voice: None,
                        gender: None,
                    },
                    2 => HookAction::Message {
                        message: text,
                        image: None,
                    },
                    3 => HookAction::Bash {
                        command: text,
                        params: String::new(),
                    },
                    5 => HookAction::Call {
                        command: text,
                        args: None,
                        timeout_ms: None,
                        mapper: None,
                    },
                    _ => {
                        app.pop_modal();
                        return;
                    }
                };
                app.config
                    .actions
                    .entry(event)
                    .or_default()
                    .push(action);
            }
            app.dirty = true;
            app.pop_to_edit_actions();
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}

/// Returns the list of (field_name, display_label, current_value) for all editable
/// fields on the given action, so every property is reachable from the TUI.
fn get_action_fields(action: &HookAction) -> Vec<(&'static str, &'static str, String)> {
    match action {
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
        } => vec![
            ("effect", "Effect", effect.clone()),
            ("volume", "Volume", format!("{volume}")),
            ("speed", "Speed", format!("{speed}")),
        ],
        HookAction::Speak {
            message,
            voice,
            gender,
        } => vec![
            ("message", "Message", message.clone()),
            (
                "voice",
                "Voice",
                voice.clone().unwrap_or_default(),
            ),
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
        HookAction::Message { message, image } => vec![
            ("message", "Message", message.clone()),
            ("image", "Image Path", image.clone().unwrap_or_default()),
        ],
        HookAction::Bash { command, params } => vec![
            ("command", "Command", command.clone()),
            ("params", "Parameters", params.clone()),
        ],
        HookAction::Report { handler } => {
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
                ("include_metadata", "Include Metadata (true/false)", metadata),
            ]
        }
        HookAction::Call {
            command,
            args,
            timeout_ms,
            mapper,
        } => vec![
            ("command", "Command", command.clone()),
            (
                "args",
                "Args (comma-separated)",
                args.as_ref()
                    .map(|a| a.join(", "))
                    .unwrap_or_default(),
            ),
            (
                "timeout_ms",
                "Timeout (ms)",
                timeout_ms
                    .map(|t| t.to_string())
                    .unwrap_or_default(),
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
fn apply_action_field(action: &mut HookAction, field_name: &str, value: String) {
    match action {
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
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
        HookAction::Message { message, image } => match field_name {
            "message" => *message = value,
            "image" => {
                *image = if value.is_empty() { None } else { Some(value) };
            }
            _ => {}
        },
        HookAction::Bash { command, params } => match field_name {
            "command" => *command = value,
            "params" => *params = value,
            _ => {}
        },
        HookAction::Report { handler } => {
            // Lazily initialize handler if needed.
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
        } => match field_name {
            "command" => *command = value,
            "args" => {
                let parsed: Vec<String> = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                *args = if parsed.is_empty() { None } else { Some(parsed) };
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
                } else if let Some(pattern) = value.strip_prefix("regex:") {
                    Some(claudine::actions::Mapper::Regex {
                        pattern: pattern.to_string(),
                    })
                } else {
                    None
                };
            }
            _ => {}
        },
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

    let field_count = app
        .config
        .actions
        .get(&event)
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
            if let Some(actions) = app.config.actions.get(&event)
                && let Some(action) = actions.get(action_index)
            {
                let fields = get_action_fields(action);
                if let Some((name, label, current_value)) = fields.get(idx) {
                    // For the "effect" field of SoundEffect, use the sound selector.
                    if *name == "effect" && matches!(action, HookAction::SoundEffect { .. }) {
                        let sounds = super::super::get_sound_effect_names();
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
            if let Some(actions) = app.config.actions.get_mut(&event)
                && let Some(action) = actions.get_mut(action_index)
            {
                apply_action_field(action, &field_name, value);
                app.dirty = true;
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
    let sounds = super::super::get_sound_effect_names();
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
                    // Editing existing action
                    if let Some(actions) = app.config.actions.get_mut(&event)
                        && let Some(action) = actions.get_mut(edit_idx)
                    {
                        if let HookAction::SoundEffect { effect, .. } = action {
                            *effect = effect_name.to_string();
                        }
                    }
                } else {
                    // Adding new action
                    let action = HookAction::SoundEffect {
                        effect: effect_name.to_string(),
                        volume: 1.0,
                        speed: 1.0,
                    };
                    app.config
                        .actions
                        .entry(event)
                        .or_default()
                        .push(action);
                }
                app.dirty = true;
                app.pop_to_edit_actions();
            }
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
}
