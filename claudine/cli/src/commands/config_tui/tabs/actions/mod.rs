use claudine::actions::HookAction;
use claudine::events::AgenticEvent;

use super::super::app::ActionView;

mod entries;
mod fields;
mod modals;
mod summary;

pub use entries::configured_event_count;
pub use modals::{
    handle_action_field_input_modal, handle_action_field_list_modal,
    handle_action_sound_selector_modal, handle_action_type_chooser_modal,
    handle_confirm_delete_modal, handle_edit_actions_modal, handle_event_selector_modal,
    handle_key, handle_text_input_modal, render,
};

/// Action types a user can add to an event.
pub(super) const ACTION_TYPE_LABELS: &[&str] = &[
    "Sound Effect",
    "Speak (using TTS provider)",
    "Message (to chat app)",
    "Shell Command",
    "Report (to STDOUT)",
    "Call (synchronous with response)",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActionSource {
    User,
    Repo,
}

impl ActionSource {
    pub(super) fn badge(self) -> &'static str {
        match self {
            ActionSource::User => "user",
            ActionSource::Repo => "repo",
        }
    }

    pub(super) fn view(self) -> ActionView {
        match self {
            ActionSource::User => ActionView::User,
            ActionSource::Repo => ActionView::Repo,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ActionEntry {
    pub event: AgenticEvent,
    pub actions: Vec<HookAction>,
    pub source: ActionSource,
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::super::super::app::{App, AppMode, ModalState};
    use super::*;
    use claudine::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn test_app(is_in_repo: bool) -> App {
        let _ = AppMode::Detail;
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
            vec![HookAction::Report {
                handler: None,
                when: None,
            }],
        );
        app.config.actions.insert(
            AgenticEvent::BeforeTool,
            vec![HookAction::Bash {
                command: "user".to_string(),
                params: String::new(),
                when: None,
            }],
        );
        app.repo_config.as_mut().unwrap().actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::Speak {
                message: "repo".to_string(),
                voice: None,
                gender: None,
                when: None,
            }],
        );

        let entries = entries::action_entries_for_view(&app);

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
            vec![HookAction::Report {
                handler: None,
                when: None,
            }],
        );
        app.repo_config.as_mut().unwrap().actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::Speak {
                message: "repo".to_string(),
                voice: None,
                gender: None,
                when: None,
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
