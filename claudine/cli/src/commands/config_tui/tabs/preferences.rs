use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState, SoundCategory};
use super::super::widgets::toggle::Toggle;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let agent_name = app.config.preferred_agent.to_string();
    let agent_line = Line::from(vec![
        Span::styled(
            "Preferred Agent",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(
            format!("[{agent_name}]"),
            if is_detail {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(agent_line), chunks[0]);

    let user_provider = app
        .config
        .canonical_provider
        .map(|p| p.to_string())
        .unwrap_or_else(|| "(not set)".to_string());
    let user_line = Line::from(vec![
        Span::styled(
            "User Provider",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(
            format!("[{user_provider}]"),
            if is_detail {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(user_line), chunks[1]);

    if app.is_in_repo {
        let repo_line = Paragraph::new(Line::from(vec![
            Span::styled(
                "Repo Provider",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(": "),
            Span::styled("[not set]", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(repo_line, chunks[2]);
    } else {
        let repo_line = Paragraph::new(Span::styled(
            "Repo Provider: (not in a git repo)",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(repo_line, chunks[2]);
    }

    let sounds_header = Paragraph::new(Span::styled(
        "Default Sounds",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(sounds_header, chunks[4]);

    let success = app
        .config
        .default_sounds
        .success
        .as_deref()
        .unwrap_or("none");
    let attention = app
        .config
        .default_sounds
        .attention
        .as_deref()
        .unwrap_or("none");
    let error = app.config.default_sounds.error.as_deref().unwrap_or("none");
    let sounds_line = Line::from(vec![
        Span::styled("S:", Style::default().fg(Color::Green)),
        Span::raw(format!(" {success}  ")),
        Span::styled("A:", Style::default().fg(Color::Yellow)),
        Span::raw(format!(" {attention}  ")),
        Span::styled("E:", Style::default().fg(Color::Red)),
        Span::raw(format!(" {error}")),
    ]);
    frame.render_widget(Paragraph::new(sounds_line), chunks[5]);

    if is_detail {
        let help = Paragraph::new(
            " a: Agent | u: User provider | r: Repo provider | 1: Success | 2: Attention | 3: Error",
        )
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[6]);
    }

    if let Some(ModalState::AgentSelector { highlighted }) = &app.modal {
        let agents = super::super::get_provider_list();
        let items: Vec<String> = agents.iter().map(|p| p.to_string()).collect();
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Select Preferred Agent",
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::UserProviderSelector { highlighted }) = &app.modal {
        let providers = super::super::get_provider_list();
        let mut items: Vec<String> = providers.iter().map(|p| p.to_string()).collect();
        items.insert(0, "(clear)".to_string());
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Select User Provider",
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::RepoProviderSelector { highlighted }) = &app.modal {
        let providers = super::super::get_provider_list();
        let mut items: Vec<String> = providers.iter().map(|p| p.to_string()).collect();
        items.insert(0, "(clear)".to_string());
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Select Repo Provider",
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::SoundSelector {
        category,
        highlighted,
    }) = &app.modal
    {
        let title = match category {
            SoundCategory::Success => "Success Sound",
            SoundCategory::Attention => "Attention Sound",
            SoundCategory::Error => "Error Sound",
        };
        let sounds = super::super::get_sound_effect_names();
        let mut items = vec!["(none)".to_string()];
        items.extend(sounds.iter().map(|s| s.to_string()));
        super::super::widgets::modal::render_list_modal(frame, area, title, &items, *highlighted);
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.modal = Some(ModalState::AgentSelector { highlighted: 0 });
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            app.modal = Some(ModalState::UserProviderSelector { highlighted: 0 });
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if app.is_in_repo {
                app.modal = Some(ModalState::RepoProviderSelector { highlighted: 0 });
            }
        }
        KeyCode::Char('1') => {
            app.modal = Some(ModalState::SoundSelector {
                category: SoundCategory::Success,
                highlighted: 0,
            });
        }
        KeyCode::Char('2') => {
            app.modal = Some(ModalState::SoundSelector {
                category: SoundCategory::Attention,
                highlighted: 0,
            });
        }
        KeyCode::Char('3') => {
            app.modal = Some(ModalState::SoundSelector {
                category: SoundCategory::Error,
                highlighted: 0,
            });
        }
        _ => {}
    }
}

pub fn handle_agent_selector_modal(app: &mut App, key: KeyEvent) {
    let providers = super::super::get_provider_list();
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
                app.config.preferred_agent = *provider;
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

pub fn handle_user_provider_modal(app: &mut App, key: KeyEvent) {
    let providers = super::super::get_provider_list();
    let count = providers.len() + 1;
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
            if idx == 0 {
                app.config.canonical_provider = None;
            } else if let Some(provider) = providers.get(idx - 1) {
                app.config.canonical_provider = Some(*provider);
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

pub fn handle_repo_provider_modal(app: &mut App, key: KeyEvent) {
    handle_user_provider_modal(app, key);
}

pub fn handle_sound_selector_modal(app: &mut App, key: KeyEvent, _category: SoundCategory) {
    let category = match app.modal {
        Some(ModalState::SoundSelector { category, .. }) => category,
        _ => return,
    };
    let sounds = super::super::get_sound_effect_names();
    let count = sounds.len() + 1;
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
            let value = if idx == 0 {
                None
            } else {
                Some(sounds[idx - 1].to_string())
            };
            match category {
                SoundCategory::Success => app.config.default_sounds.success = value,
                SoundCategory::Attention => app.config.default_sounds.attention = value,
                SoundCategory::Error => app.config.default_sounds.error = value,
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
