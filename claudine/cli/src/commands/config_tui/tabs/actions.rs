use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{ActionView, App, AppMode, ModalState};
use claudine::actions::HookAction;
use claudine::config::claudine_config::Gender;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionSource {
    User,
    Repo,
}

impl ActionSource {
    fn badge(self) -> &'static str {
        match self {
            ActionSource::User => "user",
            ActionSource::Repo => "repo",
        }
    }

    fn view(self) -> ActionView {
        match self {
            ActionSource::User => ActionView::User,
            ActionSource::Repo => ActionView::Repo,
        }
    }
}

#[derive(Debug, Clone)]
struct ActionEntry {
    event: AgenticEvent,
    actions: Vec<HookAction>,
    source: ActionSource,
}

pub fn configured_event_count(app: &App) -> usize {
    action_entries_for_view(app).len()
}

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
<<<<<<< Updated upstream
        let unconfigured = get_unconfigured_events(app);
||||||| Stash base
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
=======
        let unconfigured = unconfigured_events(app);
>>>>>>> Stashed changes
        let items: Vec<String> = if unconfigured.is_empty() {
            vec!["(all events configured)".to_string()]
        } else {
            unconfigured
                .iter()
                .map(|e| e.human_name().to_string())
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

<<<<<<< Updated upstream
    if let Some(ModalState::ActionTypeChooser { event, highlighted }) = &app.modal {
        let items: Vec<String> = ACTION_TYPE_LABELS.iter().map(|s| s.to_string()).collect();
        let title = format!("Add Action to: {}", event.human_name());
        super::super::widgets::modal::render_list_modal(frame, area, &title, &items, *highlighted);
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

||||||| Stash base
=======
    if let Some(ModalState::ActionTypeSelector { highlighted, .. }) = &app.modal {
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Select Action Type",
            &ACTION_TYPE_LABELS.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            *highlighted,
        );
    }

>>>>>>> Stashed changes
    if let Some(ModalState::ConfirmDelete { event_index }) = &app.modal {
        let sorted = action_entries_for_view(app);
        let event_name = sorted
            .get(*event_index)
            .map(|entry| entry.event.human_name())
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

                let msg = Paragraph::new(format!(
                    "Delete all {} actions for {}?",
                    app.actions_view.label().to_lowercase(),
                    event_name
                ))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);
                frame.render_widget(msg, chunks[1]);

                let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
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
<<<<<<< Updated upstream

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
        super::super::widgets::modal::render_modal(frame, area, &title, 55, 50, |frame, area| {
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

            let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
                ("ENTER", "Edit"),
                ("ESC", "Done"),
            ]);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[2]);
        });
    }

    if let Some(ModalState::ActionFieldInput { label, buffer, .. }) = &app.modal {
        super::super::widgets::modal::render_modal(frame, area, label, 60, 20, |frame, area| {
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

            let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
                ("ENTER", "Confirm"),
                ("ESC", "Cancel"),
            ]);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[2]);
        });
    }

    if let Some(ModalState::TextInput { label, buffer, .. }) = &app.modal {
        super::super::widgets::modal::render_modal(frame, area, label, 60, 20, |frame, area| {
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

            let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
                ("ENTER", "Confirm"),
                ("ESC", "Cancel"),
            ]);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, chunks[2]);
        });
    }
||||||| Stash base
=======

    if let Some(ModalState::ActionListEditor { event, highlighted }) = &app.modal {
        let items: Vec<String> = app
            .config
            .actions
            .get(event)
            .map(|actions| {
                actions
                    .iter()
                    .map(|a| action_summary_line(a))
                    .collect()
            })
            .unwrap_or_default();
        let title = format!("Actions: {}", event.as_slug());
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            &title,
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::ActionFieldEditor {
        event,
        action_index,
        field_index,
    }) = &app.modal
    {
        if let Some(action) = app
            .config
            .actions
            .get(event)
            .and_then(|a| a.get(*action_index))
        {
            let fields = get_action_fields(action);
            let items: Vec<String> = fields
                .iter()
                .map(|(name, value)| format!("{name}: {value}"))
                .collect();
            let title = format!("Edit {}", action.type_pascal_case());
            super::super::widgets::modal::render_list_modal(
                frame,
                area,
                &title,
                &items,
                *field_index,
            );
        }
    }

    if let Some(ModalState::ActionFieldInput {
        field_name, input, ..
    }) = &app.modal
    {
        super::super::widgets::modal::render_modal(
            frame,
            area,
            field_name,
            50,
            5,
            |frame, inner| {
                let text = Paragraph::new(Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::Cyan)),
                    Span::raw(input.as_str()),
                    Span::styled("_", Style::default().fg(Color::Yellow)),
                ]));
                frame.render_widget(text, inner);
            },
        );
    }
}

const ACTION_TYPE_LABELS: &[&str] = &[
    "SoundEffect", "Speak", "Bash", "Call", "Report", "Message",
];

fn default_action_for_type(label: &str) -> HookAction {
    match label {
        "SoundEffect" => HookAction::SoundEffect {
            effect: "doorbell".to_string(),
            volume: 1.0,
            speed: 1.0,
        },
        "Speak" => HookAction::Speak {
            message: "{{event}}".to_string(),
            voice: None,
            gender: None,
        },
        "Bash" => HookAction::Bash {
            command: "echo".to_string(),
            params: "{{event}}".to_string(),
        },
        "Call" => HookAction::Call {
            command: "echo".to_string(),
            args: None,
            timeout_ms: None,
            mapper: None,
        },
        "Report" => HookAction::Report { handler: None },
        "Message" => HookAction::Message {
            message: "{{event}} fired on {{project}}".to_string(),
            image: None,
        },
        _ => HookAction::Report { handler: None },
    }
}

fn unconfigured_events(app: &App) -> Vec<AgenticEvent> {
    AgenticEvent::ALL
        .into_iter()
        .filter(|e| app.config.actions.get(e).map_or(true, |a| a.is_empty()))
        .collect()
>>>>>>> Stashed changes
}

<<<<<<< Updated upstream
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

fn action_entries_for_view(app: &App) -> Vec<ActionEntry> {
    match app.actions_view {
        ActionView::Effective => effective_action_entries(app),
        ActionView::User => action_entries_from_map(&app.config.actions, ActionSource::User),
        ActionView::Repo => app
            .repo_config
            .as_ref()
            .map(|repo| action_entries_from_map(&repo.actions, ActionSource::Repo))
            .unwrap_or_default(),
    }
}

fn action_entries_from_map(
    actions: &HashMap<AgenticEvent, Vec<HookAction>>,
    source: ActionSource,
) -> Vec<ActionEntry> {
    let mut entries: Vec<_> = actions
        .iter()
        .filter(|(_, actions)| !actions.is_empty())
        .map(|(event, actions)| ActionEntry {
            event: *event,
            actions: actions.clone(),
            source,
        })
        .collect();
    entries.sort_by_key(|entry| entry.event.as_slug());
    entries
}

fn effective_action_entries(app: &App) -> Vec<ActionEntry> {
    let mut merged = action_entries_from_map(&app.config.actions, ActionSource::User);
    if let Some(repo) = &app.repo_config {
        for (event, actions) in &repo.actions {
            if actions.is_empty() {
                continue;
            }

            if let Some(existing) = merged.iter_mut().find(|entry| entry.event == *event) {
                existing.actions = actions.clone();
                existing.source = ActionSource::Repo;
            } else {
                merged.push(ActionEntry {
                    event: *event,
                    actions: actions.clone(),
                    source: ActionSource::Repo,
                });
            }
        }
    }
    merged.sort_by_key(|entry| entry.event.as_slug());
    merged
}

fn current_actions_map(app: &App) -> Option<&HashMap<AgenticEvent, Vec<HookAction>>> {
    match app.actions_view {
        ActionView::Effective => None,
        ActionView::User => Some(&app.config.actions),
        ActionView::Repo => app.repo_config.as_ref().map(|repo| &repo.actions),
    }
}

fn current_actions_map_mut(app: &mut App) -> Option<&mut HashMap<AgenticEvent, Vec<HookAction>>> {
    match app.actions_view {
        ActionView::Effective => None,
        ActionView::User => Some(&mut app.config.actions),
        ActionView::Repo => {
            if app.repo_config.is_none() {
                app.repo_config =
                    Some(claudine::config::claudine_config::RepoOverrideConfig::default());
            }
            app.repo_config.as_mut().map(|repo| &mut repo.actions)
        }
    }
}

fn mark_current_actions_dirty(app: &mut App) {
    match app.actions_view {
        ActionView::Effective => {}
        ActionView::User => app.dirty = true,
        ActionView::Repo => app.repo_dirty = true,
    }
}

fn switch_effective_selection_to_source_view(app: &mut App) {
    if app.actions_view != ActionView::Effective {
        return;
    }

    let Some(selected) = action_entries_for_view(app).get(app.list_index).cloned() else {
        return;
    };

    app.actions_view = selected.source.view();
    if let Some(index) = action_entries_for_view(app)
        .iter()
        .position(|entry| entry.event == selected.event)
    {
        app.list_index = index;
    }
}

fn get_unconfigured_events(app: &App) -> Vec<AgenticEvent> {
    let Some(actions) = current_actions_map(app) else {
        return Vec::new();
    };

    AgenticEvent::ALL
        .into_iter()
        .filter(|event| {
            actions
                .get(event)
                .is_none_or(|configured| configured.is_empty())
        })
        .collect()
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

    super::super::widgets::modal::render_modal(frame, area, &title, 55, 70, |frame, area| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // blank separator
                Constraint::Length(1), // hotkey bar
            ])
            .split(area);

        if actions.is_empty() {
            let msg =
                Paragraph::new("No actions configured.").style(Style::default().fg(Color::Gray));
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
    });
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
        }
        KeyCode::Esc => {
            app.pop_modal();
        }
        _ => {}
    }
||||||| Stash base
fn summarize_actions(actions: &[HookAction]) -> String {
    let mut types: Vec<&str> = Vec::new();
    for action in actions {
        let slug = action.type_pascal_case();
        if !types.contains(&slug) {
            types.push(slug);
        }
    }
    types.join(", ")
=======
fn summarize_actions(actions: &[HookAction]) -> String {
    use std::collections::HashSet;
    let types: HashSet<&str> = actions.iter().map(|a| a.type_pascal_case()).collect();
    let mut sorted: Vec<&str> = types.into_iter().collect();
    sorted.sort_unstable();
    sorted.join(", ")
>>>>>>> Stashed changes
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
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if app.actions_view != ActionView::Effective {
                let unconfigured = get_unconfigured_events(app);
                if !unconfigured.is_empty() {
                    app.modal = Some(ModalState::EventSelector { highlighted: 0 });
                }
            }
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if configured_count > 0 {
                let mut events: Vec<AgenticEvent> = app
                    .config
                    .actions
                    .iter()
                    .filter(|(_, a)| !a.is_empty())
                    .map(|(e, _)| *e)
                    .collect();
                events.sort_by_key(|e| e.as_slug());
                if let Some(event) = events.get(app.list_index).copied() {
                    app.modal = Some(ModalState::ActionListEditor {
                        event,
                        highlighted: 0,
                    });
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if configured_count > 0 {
                switch_effective_selection_to_source_view(app);
                app.modal = Some(ModalState::ConfirmDelete {
                    event_index: app.list_index,
                });
            }
        }
        KeyCode::Enter | KeyCode::Char('e') | KeyCode::Char('E') => {
            if configured_count > 0 {
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
<<<<<<< Updated upstream
    let unconfigured = get_unconfigured_events(app);
||||||| Stash base
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
=======
    let unconfigured = unconfigured_events(app);
>>>>>>> Stashed changes
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
<<<<<<< Updated upstream
            if let Some(event) = unconfigured.get(idx).copied() {
                app.push_modal(ModalState::ActionTypeChooser {
                    event,
                    highlighted: 0,
                });
||||||| Stash base
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
=======
            if let Some(event) = unconfigured.get(idx).copied() {
                app.modal = Some(ModalState::ActionTypeSelector {
                    event,
                    highlighted: 0,
                });
            }
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}

pub fn handle_action_type_selector_modal(app: &mut App, key: KeyEvent) {
    let event = match &app.modal {
        Some(ModalState::ActionTypeSelector { event, .. }) => *event,
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
            if let Some(label) = ACTION_TYPE_LABELS.get(idx) {
                let action = default_action_for_type(label);
                app.config
                    .actions
                    .entry(event)
                    .or_default()
                    .push(action);
                app.dirty = true;
>>>>>>> Stashed changes
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
                    if let Some(actions) = current_actions_map_mut(app) {
                        actions.entry(event).or_default().push(action);
                        mark_current_actions_dirty(app);
                    }
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
<<<<<<< Updated upstream

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
                    if let Some(actions) =
                        current_actions_map_mut(app).and_then(|actions| actions.get_mut(&event))
                        && let Some(action) = actions.get_mut(edit_idx)
                        && let HookAction::SoundEffect { effect, .. } = action
                    {
                        *effect = effect_name.to_string();
                    }
                } else {
                    // Adding new action
                    let action = HookAction::SoundEffect {
                        effect: effect_name.to_string(),
                        volume: 1.0,
                        speed: 1.0,
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

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use claudine::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn test_app(is_in_repo: bool) -> App {
        App::new(
            ClaudineConfig::default(),
            if is_in_repo {
                Some(RepoOverrideConfig::default())
            } else {
                None
            },
            None,
            is_in_repo,
            None,
            None,
        )
    }

    #[test]
    fn effective_view_uses_repo_replacements_per_event() {
        let mut app = test_app(true);
        app.config.actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::Report { handler: None }],
        );
        app.config.actions.insert(
            AgenticEvent::BeforeTool,
            vec![HookAction::Bash {
                command: "user".to_string(),
                params: String::new(),
            }],
        );
        app.repo_config.as_mut().unwrap().actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::Speak {
                message: "repo".to_string(),
                voice: None,
                gender: None,
            }],
        );

        let entries = action_entries_for_view(&app);

        assert_eq!(entries.len(), 2);
        assert_eq!(configured_event_count(&app), 2);
        assert_eq!(entries[0].event, AgenticEvent::BeforeTool);
        assert_eq!(entries[0].source, ActionSource::User);
        assert_eq!(entries[1].event, AgenticEvent::SessionStart);
        assert_eq!(entries[1].source, ActionSource::Repo);
        assert!(matches!(entries[1].actions[0], HookAction::Speak { .. }));
    }

    #[test]
    fn edit_hotkey_from_effective_view_switches_to_repo_scope() {
        let mut app = test_app(true);
        app.config.actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::Report { handler: None }],
        );
        app.repo_config.as_mut().unwrap().actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::Speak {
                message: "repo".to_string(),
                voice: None,
                gender: None,
            }],
        );

        handle_key(&mut app, key(KeyCode::Char('e')));

        assert_eq!(app.actions_view, ActionView::Repo);
        assert!(matches!(
            app.modal,
            Some(ModalState::EditActions {
                event: AgenticEvent::SessionStart,
                highlighted: 0
            })
        ));
    }
}
||||||| Stash base
=======

pub fn handle_action_list_editor(app: &mut App, key: KeyEvent) {
    let event = match &app.modal {
        Some(ModalState::ActionListEditor { event, .. }) => *event,
        _ => return,
    };
    let count = app
        .config
        .actions
        .get(&event)
        .map(|a| a.len())
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
            if idx + 1 < count {
                app.set_modal_highlighted(idx + 1);
            }
        }
        KeyCode::Enter => {
            let idx = app.modal_highlighted();
            if idx < count {
                app.modal = Some(ModalState::ActionFieldEditor {
                    event,
                    action_index: idx,
                    field_index: 0,
                });
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.modal = Some(ModalState::ActionTypeSelector {
                event,
                highlighted: 0,
            });
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            let idx = app.modal_highlighted();
            if let Some(actions) = app.config.actions.get_mut(&event) {
                if idx < actions.len() {
                    actions.remove(idx);
                    app.dirty = true;
                    if actions.is_empty() {
                        app.config.actions.remove(&event);
                        app.modal = None;
                        return;
                    }
                    let new_idx = idx.min(actions.len().saturating_sub(1));
                    app.set_modal_highlighted(new_idx);
                }
            }
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}

pub fn handle_action_field_editor(app: &mut App, key: KeyEvent) {
    let (event, action_index, field_index) = match &app.modal {
        Some(ModalState::ActionFieldEditor {
            event,
            action_index,
            field_index,
        }) => (*event, *action_index, *field_index),
        _ => return,
    };
    let fields = match app
        .config
        .actions
        .get(&event)
        .and_then(|a| a.get(action_index))
    {
        Some(action) => get_action_fields(action),
        None => {
            app.modal = None;
            return;
        }
    };
    let count = fields.len();

    match key.code {
        KeyCode::Up => {
            if field_index > 0 {
                app.set_modal_highlighted(field_index - 1);
            }
        }
        KeyCode::Down => {
            if field_index + 1 < count {
                app.set_modal_highlighted(field_index + 1);
            }
        }
        KeyCode::Enter => {
            if let Some((name, value)) = fields.get(field_index) {
                app.modal = Some(ModalState::ActionFieldInput {
                    event,
                    action_index,
                    field_name: name.clone(),
                    input: value.clone(),
                    cursor: value.len(),
                });
            }
        }
        KeyCode::Esc => {
            app.modal = Some(ModalState::ActionListEditor {
                event,
                highlighted: action_index,
            });
        }
        _ => {}
    }
}

pub fn handle_action_field_input(app: &mut App, key: KeyEvent) {
    let (event, action_index, field_name, input, cursor) = match &mut app.modal {
        Some(ModalState::ActionFieldInput {
            event,
            action_index,
            field_name,
            input,
            cursor,
        }) => (*event, *action_index, field_name.clone(), input, cursor),
        _ => return,
    };

    match key.code {
        KeyCode::Char(c) => {
            input.insert(*cursor, c);
            *cursor += 1;
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                input.remove(*cursor);
            }
        }
        KeyCode::Left => {
            if *cursor > 0 {
                *cursor -= 1;
            }
        }
        KeyCode::Right => {
            if *cursor < input.len() {
                *cursor += 1;
            }
        }
        KeyCode::Enter => {
            let value = input.clone();
            if let Some(actions) = app.config.actions.get_mut(&event) {
                if let Some(action) = actions.get_mut(action_index) {
                    set_action_field(action, &field_name, value);
                    app.dirty = true;
                }
            }
            app.modal = Some(ModalState::ActionFieldEditor {
                event,
                action_index,
                field_index: 0,
            });
        }
        KeyCode::Esc => {
            app.modal = Some(ModalState::ActionFieldEditor {
                event,
                action_index,
                field_index: 0,
            });
        }
        _ => {}
    }
}

fn action_summary_line(action: &HookAction) -> String {
    match action {
        HookAction::SoundEffect { effect, .. } => format!("SoundEffect: {effect}"),
        HookAction::Speak { message, .. } => format!("Speak: {message}"),
        HookAction::Bash { command, .. } => format!("Bash: {command}"),
        HookAction::Call { command, .. } => format!("Call: {command}"),
        HookAction::Report { .. } => "Report".to_string(),
        HookAction::Message { message, .. } => format!("Message: {message}"),
        _ => action.type_pascal_case().to_string(),
    }
}

fn get_action_fields(action: &HookAction) -> Vec<(String, String)> {
    match action {
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
        } => vec![
            ("effect".to_string(), effect.clone()),
            ("volume".to_string(), volume.to_string()),
            ("speed".to_string(), speed.to_string()),
        ],
        HookAction::Speak {
            message,
            voice,
            gender,
        } => vec![
            ("message".to_string(), message.clone()),
            (
                "voice".to_string(),
                voice.as_deref().unwrap_or("").to_string(),
            ),
            (
                "gender".to_string(),
                gender
                    .map(|g| match g {
                        Gender::Female => "female",
                        Gender::Male => "male",
                    })
                    .unwrap_or("")
                    .to_string(),
            ),
        ],
        HookAction::Bash { command, params } => vec![
            ("command".to_string(), command.clone()),
            ("params".to_string(), params.clone()),
        ],
        HookAction::Call {
            command,
            args,
            timeout_ms,
            ..
        } => vec![
            ("command".to_string(), command.clone()),
            (
                "args".to_string(),
                args.as_ref()
                    .map(|a| a.join(", "))
                    .unwrap_or_default(),
            ),
            (
                "timeout_ms".to_string(),
                timeout_ms
                    .map(|t| t.to_string())
                    .unwrap_or_default(),
            ),
        ],
        HookAction::Report { .. } => vec![],
        HookAction::Message { message, image } => vec![
            ("message".to_string(), message.clone()),
            (
                "image".to_string(),
                image.as_deref().unwrap_or("").to_string(),
            ),
        ],
        _ => vec![],
    }
}

fn set_action_field(action: &mut HookAction, field_name: &str, value: String) {
    match action {
        HookAction::SoundEffect {
            effect,
            volume,
            speed,
        } => match field_name {
            "effect" => *effect = value,
            "volume" => {
                if let Ok(v) = value.parse() {
                    *volume = v;
                }
            }
            "speed" => {
                if let Ok(v) = value.parse() {
                    *speed = v;
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
            "voice" => *voice = if value.is_empty() { None } else { Some(value) },
            "gender" => {
                *gender = match value.to_lowercase().as_str() {
                    "female" => Some(Gender::Female),
                    "male" => Some(Gender::Male),
                    _ => None,
                }
            }
            _ => {}
        },
        HookAction::Bash { command, params } => match field_name {
            "command" => *command = value,
            "params" => *params = value,
            _ => {}
        },
        HookAction::Call {
            command,
            args,
            timeout_ms,
            ..
        } => match field_name {
            "command" => *command = value,
            "args" => {
                *args = if value.is_empty() {
                    None
                } else {
                    Some(value.split(", ").map(String::from).collect())
                }
            }
            "timeout_ms" => *timeout_ms = value.parse().ok(),
            _ => {}
        },
        HookAction::Report { .. } => {}
        HookAction::Message { message, image } => match field_name {
            "message" => *message = value,
            "image" => *image = if value.is_empty() { None } else { Some(value) },
            _ => {}
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::config::claudine_config::ClaudineConfig;
    use crossterm::event::KeyEvent;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    fn test_app() -> App {
        App::new(ClaudineConfig::default(), None, None, false)
    }

    fn test_app_with_action() -> App {
        let mut app = test_app();
        app.config.actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::SoundEffect {
                effect: "doorbell".to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
        );
        app.mode = AppMode::Detail;
        app
    }

    #[test]
    fn add_event_opens_event_selector() {
        let mut app = test_app();
        app.mode = AppMode::Detail;
        handle_key(&mut app, key(KeyCode::Char('a')));
        assert!(matches!(app.modal, Some(ModalState::EventSelector { .. })));
    }

    #[test]
    fn select_event_opens_action_type_selector() {
        let mut app = test_app();
        app.modal = Some(ModalState::EventSelector { highlighted: 0 });
        handle_event_selector_modal(&mut app, key(KeyCode::Enter));
        assert!(matches!(
            app.modal,
            Some(ModalState::ActionTypeSelector { .. })
        ));
    }

    #[test]
    fn select_action_type_adds_action_to_config() {
        let mut app = test_app();
        app.modal = Some(ModalState::ActionTypeSelector {
            event: AgenticEvent::SessionStart,
            highlighted: 2, // Bash
        });
        handle_action_type_selector_modal(&mut app, key(KeyCode::Enter));
        assert!(app.dirty);
        let actions = app.config.actions.get(&AgenticEvent::SessionStart).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], HookAction::Bash { .. }));
    }

    #[test]
    fn delete_event_removes_from_config() {
        let mut app = test_app_with_action();
        app.modal = Some(ModalState::ConfirmDelete { event_index: 0 });
        handle_confirm_delete_modal(&mut app, key(KeyCode::Char('y')));
        assert!(app.config.actions.get(&AgenticEvent::SessionStart).is_none());
        assert!(app.dirty);
    }

    #[test]
    fn edit_opens_action_list_editor() {
        let mut app = test_app_with_action();
        handle_key(&mut app, key(KeyCode::Char('e')));
        assert!(matches!(
            app.modal,
            Some(ModalState::ActionListEditor {
                event: AgenticEvent::SessionStart,
                ..
            })
        ));
    }

    #[test]
    fn action_list_editor_enter_opens_field_editor() {
        let mut app = test_app_with_action();
        app.modal = Some(ModalState::ActionListEditor {
            event: AgenticEvent::SessionStart,
            highlighted: 0,
        });
        handle_action_list_editor(&mut app, key(KeyCode::Enter));
        assert!(matches!(
            app.modal,
            Some(ModalState::ActionFieldEditor {
                event: AgenticEvent::SessionStart,
                action_index: 0,
                field_index: 0,
            })
        ));
    }

    #[test]
    fn action_list_editor_delete_removes_action() {
        let mut app = test_app_with_action();
        app.config
            .actions
            .get_mut(&AgenticEvent::SessionStart)
            .unwrap()
            .push(HookAction::Bash {
                command: "echo".to_string(),
                params: "test".to_string(),
            });
        app.modal = Some(ModalState::ActionListEditor {
            event: AgenticEvent::SessionStart,
            highlighted: 0,
        });
        handle_action_list_editor(&mut app, key(KeyCode::Char('d')));
        assert!(app.dirty);
        let actions = app.config.actions.get(&AgenticEvent::SessionStart).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], HookAction::Bash { .. }));
    }

    #[test]
    fn action_list_editor_delete_last_closes_modal() {
        let mut app = test_app_with_action();
        app.modal = Some(ModalState::ActionListEditor {
            event: AgenticEvent::SessionStart,
            highlighted: 0,
        });
        handle_action_list_editor(&mut app, key(KeyCode::Char('d')));
        assert!(app.modal.is_none());
        assert!(app.config.actions.get(&AgenticEvent::SessionStart).is_none());
    }

    #[test]
    fn action_list_editor_add_opens_type_selector() {
        let mut app = test_app_with_action();
        app.modal = Some(ModalState::ActionListEditor {
            event: AgenticEvent::SessionStart,
            highlighted: 0,
        });
        handle_action_list_editor(&mut app, key(KeyCode::Char('a')));
        assert!(matches!(
            app.modal,
            Some(ModalState::ActionTypeSelector {
                event: AgenticEvent::SessionStart,
                ..
            })
        ));
    }

    #[test]
    fn field_editor_enter_opens_field_input() {
        let mut app = test_app_with_action();
        app.modal = Some(ModalState::ActionFieldEditor {
            event: AgenticEvent::SessionStart,
            action_index: 0,
            field_index: 0,
        });
        handle_action_field_editor(&mut app, key(KeyCode::Enter));
        assert!(matches!(
            app.modal,
            Some(ModalState::ActionFieldInput { .. })
        ));
    }

    #[test]
    fn field_input_enter_saves_value() {
        let mut app = test_app_with_action();
        app.modal = Some(ModalState::ActionFieldInput {
            event: AgenticEvent::SessionStart,
            action_index: 0,
            field_name: "effect".to_string(),
            input: "chime".to_string(),
            cursor: 5,
        });
        handle_action_field_input(&mut app, key(KeyCode::Enter));
        assert!(app.dirty);
        let action = &app.config.actions[&AgenticEvent::SessionStart][0];
        match action {
            HookAction::SoundEffect { effect, .. } => assert_eq!(effect, "chime"),
            _ => panic!("expected SoundEffect"),
        }
    }

    #[test]
    fn set_action_field_updates_bash_command() {
        let mut action = HookAction::Bash {
            command: "echo".to_string(),
            params: "hello".to_string(),
        };
        set_action_field(&mut action, "command", "notify-send".to_string());
        match &action {
            HookAction::Bash { command, .. } => assert_eq!(command, "notify-send"),
            _ => panic!("expected Bash"),
        }
    }

    #[test]
    fn set_action_field_updates_speak_gender() {
        let mut action = HookAction::Speak {
            message: "test".to_string(),
            voice: None,
            gender: None,
        };
        set_action_field(&mut action, "gender", "male".to_string());
        match &action {
            HookAction::Speak { gender, .. } => assert_eq!(*gender, Some(Gender::Male)),
            _ => panic!("expected Speak"),
        }
    }

    #[test]
    fn set_action_field_clears_optional_on_empty() {
        let mut action = HookAction::Speak {
            message: "test".to_string(),
            voice: Some("Alex".to_string()),
            gender: None,
        };
        set_action_field(&mut action, "voice", String::new());
        match &action {
            HookAction::Speak { voice, .. } => assert!(voice.is_none()),
            _ => panic!("expected Speak"),
        }
    }

    #[test]
    fn set_action_field_parses_volume() {
        let mut action = HookAction::SoundEffect {
            effect: "beep".to_string(),
            volume: 1.0,
            speed: 1.0,
        };
        set_action_field(&mut action, "volume", "0.5".to_string());
        match &action {
            HookAction::SoundEffect { volume, .. } => {
                assert!((volume - 0.5).abs() < f32::EPSILON)
            }
            _ => panic!("expected SoundEffect"),
        }
    }

    #[test]
    fn set_action_field_ignores_invalid_number() {
        let mut action = HookAction::SoundEffect {
            effect: "beep".to_string(),
            volume: 1.0,
            speed: 1.0,
        };
        set_action_field(&mut action, "volume", "not_a_number".to_string());
        match &action {
            HookAction::SoundEffect { volume, .. } => {
                assert!((volume - 1.0).abs() < f32::EPSILON)
            }
            _ => panic!("expected SoundEffect"),
        }
    }

    #[test]
    fn get_action_fields_returns_correct_fields_for_bash() {
        let action = HookAction::Bash {
            command: "echo".to_string(),
            params: "hello world".to_string(),
        };
        let fields = get_action_fields(&action);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("command".to_string(), "echo".to_string()));
        assert_eq!(
            fields[1],
            ("params".to_string(), "hello world".to_string())
        );
    }

    #[test]
    fn get_action_fields_returns_correct_fields_for_message() {
        let action = HookAction::Message {
            message: "hello".to_string(),
            image: Some("screenshot.png".to_string()),
        };
        let fields = get_action_fields(&action);
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("message".to_string(), "hello".to_string()));
        assert_eq!(
            fields[1],
            ("image".to_string(), "screenshot.png".to_string())
        );
    }

    #[test]
    fn summarize_actions_deduplicates() {
        let actions = vec![
            HookAction::SoundEffect {
                effect: "a".to_string(),
                volume: 1.0,
                speed: 1.0,
            },
            HookAction::SoundEffect {
                effect: "b".to_string(),
                volume: 1.0,
                speed: 1.0,
            },
            HookAction::Bash {
                command: "echo".to_string(),
                params: String::new(),
            },
        ];
        let summary = summarize_actions(&actions);
        assert_eq!(summary, "Bash, SoundEffect");
    }

    #[test]
    fn navigate_list_clamps_bounds() {
        let mut app = test_app_with_action();
        app.config.actions.insert(
            AgenticEvent::SessionEnd,
            vec![HookAction::Report { handler: None }],
        );
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.list_index, 1);
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.list_index, 1);
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.list_index, 0);
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.list_index, 0);
    }
}
>>>>>>> Stashed changes
