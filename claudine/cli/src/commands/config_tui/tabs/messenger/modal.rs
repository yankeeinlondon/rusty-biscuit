//! Messenger modal-selection key handling: the active-messenger select modal
//! (user-scope and repo-override variants) and the add-provider modal.

use crossterm::event::{KeyCode, KeyEvent};

use super::super::super::app::{App, ModalState};
use super::ensure_messenger_config;
use super::routes::PROVIDERS;

pub fn handle_messenger_select_modal(app: &mut App, key: KeyEvent) {
    let for_repo = matches!(
        app.modal,
        Some(ModalState::MessengerSelect { for_repo: true, .. })
    );

    let configs: Vec<String> = app
        .config
        .messenger
        .as_ref()
        .map(|m| {
            let mut names: Vec<_> = m.configurations.keys().cloned().collect();
            names.sort();
            names
        })
        .unwrap_or_default();

    if for_repo {
        // Repo selection: 0 = inherit, 1 = disabled, 2+ = config names
        let count = configs.len() + 2;
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
                    match idx {
                        0 => repo_cfg.active_messenger = None,       // inherit
                        1 => repo_cfg.active_messenger = Some(None), // disabled
                        i => {
                            if let Some(name) = configs.get(i - 2) {
                                repo_cfg.active_messenger = Some(Some(name.clone()));
                            }
                        }
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
    } else {
        // User-scope selection: 0 = (none), 1+ = config names
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
}

pub fn handle_messenger_add_modal(app: &mut App, key: KeyEvent) {
    let count = PROVIDERS.len();
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
            if let Some(provider) = PROVIDERS.get(idx) {
                // Start with a "Configuration Name" field so the user can give
                // a unique name (allowing multiple configs per provider).
                app.push_modal(ModalState::MessengerInput {
                    provider: provider.to_string(),
                    field_index: 0,
                    fields: Vec::new(),
                    buffer: String::new(),
                    label: "Configuration Name".to_string(),
                    is_secret: false,
                    error: None,
                    test_status: None,
                });
            }
        }
        KeyCode::Esc => {
            app.modal = None;
        }
        _ => {}
    }
}
