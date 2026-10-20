use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

use super::super::app::{App, AppMode, GenderTab, ModalState};
use super::super::reducers::{apply_tts_provider_selection, apply_voice_selection};
use super::super::widgets::toggle::Toggle;
use claudine::config::claudine_config::{Gender, TtsConfigSettings, TtsValue, VoiceSelection};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let is_detail = app.mode == AppMode::Detail;
    let is_enabled = !matches!(app.config.tts, TtsValue::Boolean(false));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Toggle
            Constraint::Length(1), // blank
            Constraint::Length(1), // Provider line
            Constraint::Length(1), // blank
            Constraint::Length(1), // Female voice
            Constraint::Length(1), // Male voice
            Constraint::Length(1), // blank
            Constraint::Length(1), // default gender legend
            Constraint::Length(1), // blank
            Constraint::Length(2), // hook-only scope note
            Constraint::Min(0),
        ])
        .split(area);

    // "Text to Speech (TTS):" toggle
    let toggle = Toggle::new("Text to Speech (TTS)", is_enabled, is_detail);
    frame.render_widget(toggle, chunks[0]);

    frame.render_widget(
        Paragraph::new(Span::styled(
            HOOK_SCOPE_NOTE,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ))
        .wrap(Wrap { trim: true }),
        chunks[9],
    );

    if !is_enabled {
        let disabled_msg = Paragraph::new(Span::styled(
            "TTS is disabled. Press T to enable.",
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(disabled_msg, chunks[2]);
        // Still render modals if any
        render_modals(frame, area, app);
        return;
    }

    let (_provider_slug, provider_display, female, male, default_gender) = resolve_tts_display(app);

    // Provider line
    let provider_line = Line::from(vec![
        Span::styled("Provider: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            provider_display,
            if is_detail {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(provider_line), chunks[2]);

    // Female voice
    let mut female_spans = vec![
        Span::styled(
            "Female Voice: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(&female),
    ];
    if default_gender == Gender::Female {
        female_spans.push(Span::styled(
            "  *",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(female_spans)), chunks[4]);

    // Male voice
    let mut male_spans = vec![
        Span::styled(
            "Male Voice: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(&male),
    ];
    if default_gender == Gender::Male {
        male_spans.push(Span::styled(
            "  *",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(male_spans)), chunks[5]);

    // Default gender legend
    let legend = Line::from(vec![
        Span::styled(
            "*",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " indicates the default gender to be used",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]);
    frame.render_widget(Paragraph::new(legend), chunks[7]);

    render_modals(frame, area, app);
}

fn render_modals(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(ModalState::TtsProvider { highlighted }) = &app.modal {
        let providers = super::super::get_tts_provider_list();
        let items: Vec<String> = providers
            .iter()
            .map(|p| {
                let display = super::super::tts_provider_display_name(p);
                if matches!(p, biscuit_speaks::types::TtsProvider::Cloud(_)) {
                    format!("{display} [cloud]")
                } else {
                    display.to_string()
                }
            })
            .collect();
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
        let title = match &app.modal {
            Some(ModalState::VoiceSelector {
                gender: GenderTab::Female,
                ..
            }) => "Select Female Voice",
            _ => "Select Male Voice",
        };
        render_voice_selector_modal(frame, area, title, &app.cached_voices, *highlighted);
    }
}

fn voice_quality_icon(quality: biscuit_speaks::VoiceQuality) -> &'static str {
    match quality {
        biscuit_speaks::VoiceQuality::Excellent => " ⭐️",
        biscuit_speaks::VoiceQuality::Low | biscuit_speaks::VoiceQuality::Moderate => " 👎",
        _ => "",
    }
}

fn render_voice_selector_modal(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    voices: &[(String, biscuit_speaks::VoiceQuality)],
    highlighted: usize,
) {
    super::super::widgets::modal::render_modal(frame, area, title, 50, 70, |frame, area| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // blank separator
                Constraint::Length(1), // legend
                Constraint::Length(1), // blank separator
                Constraint::Length(1), // hotkey bar
            ])
            .split(area);

        let items: Vec<ListItem> = voices
            .iter()
            .enumerate()
            .map(|(i, (name, quality))| {
                let icon = voice_quality_icon(*quality);
                let style = if i == highlighted {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(name.as_str(), style),
                    Span::raw(icon),
                ]))
            })
            .collect();

        let list = List::new(items).highlight_symbol(">> ").highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default().with_selected(Some(highlighted));
        frame.render_stateful_widget(list, chunks[0], &mut state);

        // Quality legend
        let legend = Paragraph::new(Line::from(vec![Span::styled(
            "Voices with excellent quality are indicated with ⭐️, poor voice quality with 👎.",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )]));
        frame.render_widget(legend, chunks[2]);

        let hotkey_line = super::super::widgets::modal::build_modal_hotkey_line(&[
            ("ENTER", "Select"),
            ("ESC", "Cancel"),
        ]);
        let hotkey_widget = Paragraph::new(hotkey_line)
            .alignment(Alignment::Center)
            .style(Style::default().bg(Color::Indexed(236)));
        frame.render_widget(hotkey_widget, chunks[4]);
    });
}

/// The toggle maps to `"tts": false`, which only silences hook `speak`
/// actions; lifecycle `say` and its ahead-of-time synthesis ignore it.
const HOOK_SCOPE_NOTE: &str =
    "This toggle controls hook speech only; lifecycle `say` speech and its pre-warming are not affected.";

/// The provider slug the tab shows and lists voices for: the object's own
/// provider, else the best available one (what `"tts": true` would use).
fn effective_provider_slug(provider: Option<&str>) -> Option<String> {
    provider.map(str::to_string).or_else(|| {
        super::super::get_tts_provider_list()
            .first()
            .map(|best| super::super::tts_provider_slug(best).to_string())
    })
}

fn resolve_default_voice_name(provider_slug: &str, gender: Gender) -> String {
    let bs_gender = match gender {
        Gender::Female => biscuit_speaks::Gender::Female,
        Gender::Male => biscuit_speaks::Gender::Male,
    };
    if let Some(provider) = super::super::tts_provider_from_slug(provider_slug) {
        biscuit_speaks::default_voice_name(&provider, bs_gender).to_string()
    } else {
        "default".to_string()
    }
}

/// Resolve display strings for provider, female voice, and male voice.
fn resolve_tts_display(app: &App) -> (String, String, String, String, Gender) {
    match &app.config.tts {
        TtsValue::Config(cfg) => {
            let slug = effective_provider_slug(cfg.provider.as_deref())
                .unwrap_or_else(|| "none".to_string());
            let display = super::super::tts_provider_from_slug(&slug)
                .map(|p| super::super::tts_provider_display_name(&p).to_string())
                .unwrap_or_else(|| slug.clone());

            let female = match &cfg.voice {
                Some(VoiceSelection::Single(v)) => v.clone(),
                Some(VoiceSelection::Gendered { female, .. }) => female.clone(),
                None => resolve_default_voice_name(&slug, Gender::Female),
            };
            let male = match &cfg.voice {
                Some(VoiceSelection::Gendered { male, .. }) => male.clone(),
                _ => resolve_default_voice_name(&slug, Gender::Male),
            };
            (slug, display, female, male, cfg.gender)
        }
        _ => {
            // TTS is enabled (true) but no config yet - resolve best available provider
            let providers = super::super::get_tts_provider_list();
            if let Some(best) = providers.first() {
                let slug = super::super::tts_provider_slug(best).to_string();
                let display = super::super::tts_provider_display_name(best).to_string();
                (
                    slug.clone(),
                    display,
                    resolve_default_voice_name(&slug, Gender::Female),
                    resolve_default_voice_name(&slug, Gender::Male),
                    Gender::Female,
                )
            } else {
                (
                    "none".to_string(),
                    "No providers available".to_string(),
                    "N/A".to_string(),
                    "N/A".to_string(),
                    Gender::Female,
                )
            }
        }
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let is_enabled = !matches!(app.config.tts, TtsValue::Boolean(false));

    match key.code {
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if is_enabled {
                app.config.tts = TtsValue::Boolean(false);
            } else {
                // Enable with the best available provider
                let providers = super::super::get_tts_provider_list();
                if let Some(best) = providers.first() {
                    let slug = super::super::tts_provider_slug(best).to_string();
                    app.config.tts = TtsValue::Config(TtsConfigSettings {
                        provider: Some(slug),
                        voice: None,
                        gender: Gender::Female,
                        prewarm: None,
                    });
                } else {
                    app.config.tts = TtsValue::Boolean(true);
                }
            }
            app.dirty = true;
        }
        _ if !is_enabled => {}
        // Shift+F → set preferred gender to female
        KeyCode::Char('F') => {
            ensure_tts_config(app);
            if let TtsValue::Config(ref mut cfg) = app.config.tts {
                cfg.gender = Gender::Female;
                app.dirty = true;
            }
        }
        // Shift+M → set preferred gender to male
        KeyCode::Char('M') => {
            ensure_tts_config(app);
            if let TtsValue::Config(ref mut cfg) = app.config.tts {
                cfg.gender = Gender::Male;
                app.dirty = true;
            }
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            let current_slug = match &app.config.tts {
                TtsValue::Config(cfg) => cfg.provider.clone(),
                _ => None,
            };
            let providers = super::super::get_tts_provider_list();
            let highlighted = current_slug
                .and_then(|slug| {
                    providers
                        .iter()
                        .position(|p| super::super::tts_provider_slug(p) == slug)
                })
                .unwrap_or(0);
            app.modal = Some(ModalState::TtsProvider { highlighted });
        }
        KeyCode::Char('f') => {
            let provider = match &app.config.tts {
                TtsValue::Config(cfg) => effective_provider_slug(cfg.provider.as_deref()),
                _ => return,
            };
            let Some(provider) = provider else { return };
            app.cached_voices = super::super::query_voices_for_provider(&provider);
            if app.cached_voices.is_empty() {
                return;
            }
            let current_voice = match &app.config.tts {
                TtsValue::Config(cfg) => match &cfg.voice {
                    Some(VoiceSelection::Single(v)) => Some(v.clone()),
                    Some(VoiceSelection::Gendered { female, .. }) => Some(female.clone()),
                    None => None,
                },
                _ => None,
            };
            let highlighted = current_voice
                .and_then(|v| app.cached_voices.iter().position(|(name, _)| *name == v))
                .unwrap_or(0);
            app.modal = Some(ModalState::VoiceSelector {
                gender: GenderTab::Female,
                highlighted,
            });
        }
        KeyCode::Char('m') => {
            let provider = match &app.config.tts {
                TtsValue::Config(cfg) => effective_provider_slug(cfg.provider.as_deref()),
                _ => return,
            };
            let Some(provider) = provider else { return };
            app.cached_voices = super::super::query_voices_for_provider(&provider);
            if app.cached_voices.is_empty() {
                return;
            }
            let current_voice = match &app.config.tts {
                TtsValue::Config(cfg) => match &cfg.voice {
                    Some(VoiceSelection::Gendered { male, .. }) => Some(male.clone()),
                    _ => None,
                },
                _ => None,
            };
            let highlighted = current_voice
                .and_then(|v| app.cached_voices.iter().position(|(name, _)| *name == v))
                .unwrap_or(0);
            app.modal = Some(ModalState::VoiceSelector {
                gender: GenderTab::Male,
                highlighted,
            });
        }
        _ => {}
    }
}

pub fn handle_tts_provider_modal(app: &mut App, key: KeyEvent) {
    let providers = super::super::get_tts_provider_list();
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
                let slug = super::super::tts_provider_slug(provider).to_string();
                app.config.tts =
                    TtsValue::Config(apply_tts_provider_selection(&app.config.tts, slug));
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
    let count = voices.len();
    if count == 0 {
        if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
            app.modal = None;
        }
        return;
    }

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
            let selected_voice = voices.get(idx).map(|(name, _)| name.clone());

            ensure_tts_config(app);
            if let TtsValue::Config(ref mut cfg) = app.config.tts
                && let Some(voice_name) = selected_voice
            {
                // The listed voices belong to the effective provider; pin it
                // so the chosen voice is not sent to a different default.
                if cfg.provider.is_none() {
                    cfg.provider = effective_provider_slug(None);
                }
                cfg.voice = Some(apply_voice_selection(
                    cfg.voice.as_ref(),
                    gender_tab,
                    voice_name,
                ));
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
        let providers = super::super::get_tts_provider_list();
        let slug = providers
            .first()
            .map(|p| super::super::tts_provider_slug(p).to_string())
            .unwrap_or_else(|| "say".to_string());
        app.config.tts = TtsValue::Config(TtsConfigSettings {
            provider: Some(slug),
            voice: None,
            gender: Gender::Female,
            prewarm: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use claudine::config::claudine_config::ClaudineConfig;

    fn app_with_tts(tts: TtsValue) -> App {
        let config = ClaudineConfig {
            tts,
            ..ClaudineConfig::default()
        };
        App::new(config, None, None, false, None, None)
    }

    fn prewarm_off(provider: Option<&str>) -> TtsValue {
        TtsValue::Config(TtsConfigSettings {
            provider: provider.map(str::to_string),
            voice: None,
            gender: Gender::Female,
            prewarm: Some(false),
        })
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn prewarm_of(app: &App) -> Option<bool> {
        match &app.config.tts {
            TtsValue::Config(cfg) => cfg.prewarm,
            other => panic!("expected the object form, got {other:?}"),
        }
    }

    #[test]
    fn gender_edits_keep_prewarm() {
        for provider in [Some("say"), None] {
            let mut app = app_with_tts(prewarm_off(provider));
            handle_key(&mut app, key(KeyCode::Char('M')));
            assert_eq!(prewarm_of(&app), Some(false), "{provider:?}");
            handle_key(&mut app, key(KeyCode::Char('F')));
            assert_eq!(prewarm_of(&app), Some(false), "{provider:?}");
            assert!(app.dirty);
        }
    }

    #[test]
    fn voice_selection_keeps_prewarm() {
        let mut app = app_with_tts(prewarm_off(Some("say")));
        app.cached_voices = vec![("Samantha".to_string(), biscuit_speaks::VoiceQuality::Unknown)];
        app.modal = Some(ModalState::VoiceSelector {
            gender: GenderTab::Female,
            highlighted: 0,
        });

        handle_voice_selector_modal(&mut app, key(KeyCode::Enter));

        match &app.config.tts {
            TtsValue::Config(cfg) => {
                assert_eq!(cfg.prewarm, Some(false));
                assert_eq!(cfg.provider.as_deref(), Some("say"));
                assert_eq!(cfg.voice, Some(VoiceSelection::Single("Samantha".to_string())));
            }
            other => panic!("expected the object form, got {other:?}"),
        }
    }

    #[test]
    fn switching_to_a_boolean_restores_the_prewarm_default() {
        let mut app = app_with_tts(prewarm_off(Some("say")));
        handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(matches!(app.config.tts, TtsValue::Boolean(false)));
        assert!(app.config.tts.prewarm_enabled());
    }

    #[test]
    fn edited_prewarm_survives_save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut app = app_with_tts(prewarm_off(None));
        handle_key(&mut app, key(KeyCode::Char('M')));

        claudine::dispatch::loader::save_claudine_config(&app.config, &path).unwrap();
        let reloaded = claudine::dispatch::loader::load_claudine_config(Some(&path), None).unwrap();

        assert!(!reloaded.tts.prewarm_enabled());
        match reloaded.tts {
            TtsValue::Config(cfg) => {
                assert_eq!(cfg.provider, None);
                assert_eq!(cfg.gender, Gender::Male);
            }
            other => panic!("expected the object form, got {other:?}"),
        }
    }

    #[test]
    fn tab_explains_the_toggle_covers_hook_speech_only() {
        for tts in [TtsValue::Boolean(false), prewarm_off(Some("say"))] {
            let backend = TestBackend::new(120, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            let app = app_with_tts(tts.clone());
            terminal.draw(|frame| render(frame, frame.area(), &app)).unwrap();
            let content: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            assert!(content.contains("hook speech only"), "{tts:?}");
            assert!(content.contains("pre-warming are not affected"), "{tts:?}");
        }
    }
}
