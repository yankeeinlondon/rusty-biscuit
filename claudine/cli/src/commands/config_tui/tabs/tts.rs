use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, GenderTab, ModalState};
use super::super::widgets::toggle::Toggle;
use claudine::config::claudine_config::{Gender, TtsConfigSettings, TtsValue, VoiceSelection};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;
    let is_enabled = !matches!(app.config.tts, TtsValue::Boolean(false));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

    let toggle = Toggle::new("Text-to-Speech", is_enabled, is_detail);
    frame.render_widget(toggle, chunks[0]);

    let style = if is_enabled {
        Style::default()
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let (provider, female, male, default_gender) = match &app.config.tts {
        TtsValue::Config(cfg) => {
            let female = match &cfg.voice {
                Some(VoiceSelection::Single(v)) => v.as_str(),
                Some(VoiceSelection::Gendered { female, .. }) => female.as_str(),
                None => "(auto)",
            };
            let male = match &cfg.voice {
                Some(VoiceSelection::Gendered { male, .. }) => male.as_str(),
                _ => "(auto)",
            };
            (cfg.provider.as_str(), female, male, cfg.gender)
        }
        _ => ("(auto)", "(auto)", "(auto)", Gender::Female),
    };

    let info_line = Line::from(vec![
        Span::styled(provider, style.add_modifier(Modifier::BOLD)),
        Span::styled(" -> ", style),
        if default_gender == Gender::Female {
            Span::styled(female, style)
        } else {
            Span::styled(female, style.fg(Color::DarkGray))
        },
        Span::styled(" / ", style),
        if default_gender == Gender::Male {
            Span::styled(male, style)
        } else {
            Span::styled(male, style.fg(Color::DarkGray))
        },
    ]);
    frame.render_widget(Paragraph::new(info_line), chunks[2]);

    if is_detail {
        let help = Paragraph::new(" t: toggle | p: provider | f: female voice | m: male voice")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[3]);
    }

    if let Some(ModalState::TtsProvider { highlighted }) = &app.modal {
        let providers = super::super::get_tts_providers();
        let items: Vec<String> = providers.iter().map(|s| s.to_string()).collect();
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "TTS Provider",
            &items,
            *highlighted,
        );
    }

    if let Some(ModalState::VoiceSelector {
        gender: _,
        highlighted,
    }) = &app.modal
    {
        let mut items = vec!["(auto)".to_string()];
        items.extend(app.cached_voices.iter().cloned());
        super::super::widgets::modal::render_list_modal(
            frame,
            area,
            "Select Voice",
            &items,
            *highlighted,
        );
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let is_enabled = !matches!(app.config.tts, TtsValue::Boolean(false));

    match key.code {
        KeyCode::Char('t') | KeyCode::Char('T') => {
            app.config.tts = if is_enabled {
                TtsValue::Boolean(false)
            } else {
                TtsValue::Boolean(true)
            };
            app.dirty = true;
        }
        _ if !is_enabled => {}
        KeyCode::Char('p') => {
            app.modal = Some(ModalState::TtsProvider { highlighted: 0 });
        }
        KeyCode::Char('f') => {
            let provider = match &app.config.tts {
                TtsValue::Config(cfg) => cfg.provider.clone(),
                _ => "auto".to_string(),
            };
            app.cached_voices = super::super::query_voices_for_provider(&provider);
            app.modal = Some(ModalState::VoiceSelector {
                gender: GenderTab::Female,
                highlighted: 0,
            });
        }
        KeyCode::Char('m') => {
            let provider = match &app.config.tts {
                TtsValue::Config(cfg) => cfg.provider.clone(),
                _ => "auto".to_string(),
            };
            app.cached_voices = super::super::query_voices_for_provider(&provider);
            app.modal = Some(ModalState::VoiceSelector {
                gender: GenderTab::Male,
                highlighted: 0,
            });
        }
        _ => {}
    }
}

pub fn handle_tts_provider_modal(app: &mut App, key: KeyEvent) {
    let providers = super::super::get_tts_providers();
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
                let current_voice = match &app.config.tts {
                    TtsValue::Config(cfg) => cfg.voice.clone(),
                    _ => None,
                };
                let current_gender = match &app.config.tts {
                    TtsValue::Config(cfg) => cfg.gender,
                    _ => Gender::Female,
                };
                app.config.tts = TtsValue::Config(TtsConfigSettings {
                    provider: provider.to_string(),
                    voice: current_voice,
                    gender: current_gender,
                });
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

pub fn handle_voice_selector_modal(app: &mut App, key: KeyEvent) {
    let gender_tab = match &app.modal {
        Some(ModalState::VoiceSelector { gender, .. }) => *gender,
        _ => return,
    };
    let voices = &app.cached_voices;
    let count = voices.len() + 1;
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
            let selected_voice = if idx == 0 {
                None
            } else {
                Some(voices[idx - 1].clone())
            };

            ensure_tts_config(app);
            if let TtsValue::Config(ref mut cfg) = app.config.tts {
                match (&cfg.voice, selected_voice) {
                    (Some(VoiceSelection::Gendered { male, female }), Some(v)) => {
                        match gender_tab {
                            GenderTab::Female => {
                                cfg.voice = Some(VoiceSelection::Gendered {
                                    male: male.clone(),
                                    female: v,
                                });
                            }
                            GenderTab::Male => {
                                cfg.voice = Some(VoiceSelection::Gendered {
                                    male: v,
                                    female: female.clone(),
                                });
                            }
                        }
                    }
                    (_, Some(v)) => {
                        cfg.voice = Some(VoiceSelection::Single(v));
                    }
                    (_, None) => {
                        cfg.voice = None;
                    }
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

fn ensure_tts_config(app: &mut App) {
    if matches!(app.config.tts, TtsValue::Boolean(true)) {
        app.config.tts = TtsValue::Config(TtsConfigSettings {
            provider: "auto".to_string(),
            voice: None,
            gender: Gender::Female,
        });
    }
}
