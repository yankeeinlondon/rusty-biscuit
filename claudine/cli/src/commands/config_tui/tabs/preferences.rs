use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, ModalState, SoundCategory};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Preferred Agent
            Constraint::Length(1), // "Canonical Sources:" heading
            Constraint::Length(1), // User Scoped Provider
            Constraint::Length(1), // Repo Scoped Provider
            Constraint::Length(1), // blank
            Constraint::Length(1), // "Default Sounds" heading
            Constraint::Length(1), // Success
            Constraint::Length(1), // Attention
            Constraint::Length(1), // Error
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

    // Canonical Sources heading
    let heading = Paragraph::new(Span::styled(
        "Canonical Sources:",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(heading, chunks[1]);

    // User Scoped Provider (indented)
    let user_provider = app
        .config
        .canonical_provider
        .map(|p| p.to_string())
        .unwrap_or_else(|| "(not set)".to_string());
    let user_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("User Scoped Provider", Style::default()),
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
    frame.render_widget(Paragraph::new(user_line), chunks[2]);

    // Repo Scoped Provider (indented)
    if app.is_in_repo {
        let repo_provider = app
            .repo_config
            .as_ref()
            .and_then(|rc| rc.canonical_provider)
            .map(|p| format!("[{}]", p))
            .unwrap_or_else(|| "[not set]".to_string());
        let repo_line = Line::from(vec![
            Span::raw("  "),
            Span::styled("Repo Scoped Provider", Style::default()),
            Span::raw(": "),
            Span::styled(
                repo_provider,
                if is_detail {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Gray)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(repo_line), chunks[3]);
    } else {
        let repo_line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Repo Scoped Provider: (not in a git repo)",
                Style::default().fg(Color::Gray),
            ),
        ]);
        frame.render_widget(Paragraph::new(repo_line), chunks[3]);
    }

    let sounds_header = Paragraph::new(Span::styled(
        "Default Sounds:",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(sounds_header, chunks[5]);

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

    let value_style = Style::default().fg(Color::Indexed(250));
    let success_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Success: ", Style::default().fg(Color::Green)),
        Span::styled(success, value_style),
    ]);
    let attention_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Attention: ", Style::default().fg(Color::Yellow)),
        Span::styled(attention, value_style),
    ]);
    let error_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Error: ", Style::default().fg(Color::Red)),
        Span::styled(error, value_style),
    ]);
    frame.render_widget(Paragraph::new(success_line), chunks[6]);
    frame.render_widget(Paragraph::new(attention_line), chunks[7]);
    frame.render_widget(Paragraph::new(error_line), chunks[8]);

    if let Some(ModalState::AgentSelector { highlighted }) = &app.modal {
        let agents = super::super::get_available_providers(app);
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
        let providers = super::super::get_available_providers(app);
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
        super::super::widgets::modal::render_list_modal_with_hotkeys(
            frame,
            area,
            title,
            &items,
            *highlighted,
            &[("P", "Play"), ("ENTER", "Select"), ("ESC", "Cancel")],
        );
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('a') | KeyCode::Char('A') => {
            let agents = super::super::get_available_providers(app);
            let highlighted = agents
                .iter()
                .position(|p| *p == app.config.preferred_agent)
                .unwrap_or(0);
            app.modal = Some(ModalState::AgentSelector { highlighted });
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            let providers = super::super::get_available_providers(app);
            let highlighted = app
                .config
                .canonical_provider
                .and_then(|cp| providers.iter().position(|p| *p == cp))
                .map(|i| i + 1) // +1 for "(clear)" at index 0
                .unwrap_or(0);
            app.modal = Some(ModalState::UserProviderSelector { highlighted });
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if app.is_in_repo {
                let providers = super::super::get_provider_list();
                let current = app
                    .repo_config
                    .as_ref()
                    .and_then(|rc| rc.canonical_provider);
                let highlighted = current
                    .and_then(|cp| providers.iter().position(|p| *p == cp))
                    .map(|i| i + 1) // +1 for "(clear)" at index 0
                    .unwrap_or(0);
                app.modal = Some(ModalState::RepoProviderSelector { highlighted });
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let current = app.config.default_sounds.success.as_deref();
            let sounds = super::super::get_sound_effect_names();
            let highlighted = current
                .and_then(|name| sounds.iter().position(|s| *s == name))
                .map(|i| i + 1) // +1 because index 0 is "(none)"
                .unwrap_or(0);
            app.modal = Some(ModalState::SoundSelector {
                category: SoundCategory::Success,
                highlighted,
            });
        }
        // Note: 'A' is taken by Agent selector, so Attention uses 'N' (atteNtion)
        KeyCode::Char('n') | KeyCode::Char('N') => {
            let current = app.config.default_sounds.attention.as_deref();
            let sounds = super::super::get_sound_effect_names();
            let highlighted = current
                .and_then(|name| sounds.iter().position(|s| *s == name))
                .map(|i| i + 1)
                .unwrap_or(0);
            app.modal = Some(ModalState::SoundSelector {
                category: SoundCategory::Attention,
                highlighted,
            });
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            let current = app.config.default_sounds.error.as_deref();
            let sounds = super::super::get_sound_effect_names();
            let highlighted = current
                .and_then(|name| sounds.iter().position(|s| *s == name))
                .map(|i| i + 1)
                .unwrap_or(0);
            app.modal = Some(ModalState::SoundSelector {
                category: SoundCategory::Error,
                highlighted,
            });
        }
        _ => {}
    }
}

pub fn handle_agent_selector_modal(app: &mut App, key: KeyEvent) {
    let providers = super::super::get_available_providers(app);
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
    let providers = super::super::get_available_providers(app);
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
            if app.repo_config.is_none() {
                app.repo_config =
                    Some(claudine::config::claudine_config::RepoOverrideConfig::default());
            }
            if let Some(ref mut repo_cfg) = app.repo_config {
                if idx == 0 {
                    repo_cfg.canonical_provider = None;
                } else if let Some(provider) = providers.get(idx - 1) {
                    repo_cfg.canonical_provider = Some(*provider);
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
        KeyCode::Char('p') | KeyCode::Char('P') => {
            let idx = app.modal_highlighted();
            if idx > 0
                && let Some(effect) = playa::SoundEffect::from_name(sounds[idx - 1])
            {
                // Play in a background thread so we don't block the TUI
                std::thread::spawn(move || {
                    let _ = effect.play();
                });
            }
        }
        KeyCode::Enter | KeyCode::Char('d') | KeyCode::Char('D') => {
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
