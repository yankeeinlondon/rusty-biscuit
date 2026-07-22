use crossterm::event::{KeyCode, KeyEvent};

use claudine::config::claudine_config::{ClaudineConfig, RepoOverrideConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Overview,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Preferences,
    Services,
    Actions,
    Tts,
    Messenger,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Preferences,
        Tab::Services,
        Tab::Actions,
        Tab::Tts,
        Tab::Messenger,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Tab::Preferences => "Preferences",
            Tab::Services => "Services",
            Tab::Actions => "Actions",
            Tab::Tts => "TTS",
            Tab::Messenger => "Messenger",
        }
    }

    pub fn next(&self) -> Tab {
        let idx = Tab::ALL.iter().position(|t| t == self).unwrap();
        Tab::ALL[(idx + 1) % Tab::ALL.len()]
    }

    pub fn prev(&self) -> Tab {
        let idx = Tab::ALL.iter().position(|t| t == self).unwrap();
        Tab::ALL[(idx + Tab::ALL.len() - 1) % Tab::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionView {
    Effective,
    User,
    Repo,
}

impl ActionView {
    pub fn label(&self) -> &'static str {
        match self {
            ActionView::Effective => "Effective",
            ActionView::User => "User",
            ActionView::Repo => "Repo",
        }
    }
}

pub struct App {
    pub mode: AppMode,
    pub focused_tab: Tab,
    pub selected_tab: Option<Tab>,
    pub actions_view: ActionView,
    pub config: ClaudineConfig,
    pub is_in_repo: bool,
    pub should_quit: bool,
    pub dirty: bool,
    pub repo_config: Option<RepoOverrideConfig>,
    pub repo_config_path: Option<std::path::PathBuf>,
    pub repo_dirty: bool,
    pub repo_name: Option<String>,
    pub branch_name: Option<String>,
    pub list_index: usize,
    pub modal: Option<ModalState>,
    pub modal_stack: Vec<ModalState>,
    pub cached_voices: Vec<(String, biscuit_speaks::VoiceQuality)>,
    pub messenger_focus: usize,
    /// Cached provider discovery results (computed once in `App::new()`).
    pub cached_agents: Vec<claudine::config::AgentInfo>,
    /// Pending webhook test connection result receiver.
    /// When Some, the TUI event loop polls this channel for async test results.
    ///
    /// Carries the typed failure rather than pre-rendered prose: the event loop
    /// is the render boundary, and `MessagingError`'s `Display` is already
    /// webhook-redacted.
    pub pending_test:
        Option<std::sync::mpsc::Receiver<Result<(), claudine::messaging::MessagingError>>>,
}

#[derive(Debug, Clone)]
pub enum ModalState {
    AgentSelector {
        highlighted: usize,
    },
    UserProviderSelector {
        highlighted: usize,
    },
    RepoProviderSelector {
        highlighted: usize,
    },
    SoundSelector {
        category: SoundCategory,
        highlighted: usize,
    },
    ProtectRules {
        highlighted: usize,
        staged_rules: Box<claudine::protect::config::ProtectRuleToggles>,
    },
    EditActions {
        event: claudine::events::AgenticEvent,
        highlighted: usize,
    },
    TtsProvider {
        highlighted: usize,
    },
    VoiceSelector {
        gender: GenderTab,
        highlighted: usize,
    },
    MessengerSelect {
        highlighted: usize,
        for_repo: bool,
    },
    MessengerAdd {
        highlighted: usize,
    },
    EventSelector {
        highlighted: usize,
    },
    ActionTypeChooser {
        event: claudine::events::AgenticEvent,
        highlighted: usize,
    },
    ConfirmDelete {
        event_index: usize,
    },
    TextInput {
        event: claudine::events::AgenticEvent,
        action_type: usize,
        buffer: String,
        label: String,
        /// When Some, update the action at this index instead of appending.
        edit_index: Option<usize>,
    },
    /// Sound effect picker for adding/editing a sound_effect action.
    ActionSoundSelector {
        event: claudine::events::AgenticEvent,
        highlighted: usize,
        /// When Some, update the action at this index instead of appending.
        edit_index: Option<usize>,
    },
    /// Input modal for messenger provider config fields.
    MessengerInput {
        provider: String,
        /// Current field being edited (index into the provider's field list).
        field_index: usize,
        /// Accumulated field values collected so far.
        fields: Vec<(String, String)>,
        buffer: String,
        label: String,
        /// Whether the current field should be rendered as a secret (masked).
        is_secret: bool,
        /// Optional validation error to display for the current field.
        error: Option<String>,
        /// Optional test-connection status for webhook providers.
        test_status: Option<String>,
    },
    /// Multi-field editor for an individual action's properties.
    ActionFieldList {
        event: claudine::events::AgenticEvent,
        action_index: usize,
        highlighted: usize,
    },
    /// Text input for a single field within the action field editor.
    ActionFieldInput {
        event: claudine::events::AgenticEvent,
        action_index: usize,
        field_name: String,
        buffer: String,
        label: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCategory {
    Success,
    Attention,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenderTab {
    Female,
    Male,
}

impl App {
    pub fn new(
        config: ClaudineConfig,
        repo_config: Option<RepoOverrideConfig>,
        repo_config_path: Option<std::path::PathBuf>,
        is_in_repo: bool,
        repo_name: Option<String>,
        branch_name: Option<String>,
    ) -> Self {
        let cached_agents = claudine::config::discover_agents_full();
        Self {
            mode: AppMode::Overview,
            focused_tab: Tab::Preferences,
            selected_tab: None,
            actions_view: if is_in_repo {
                ActionView::Effective
            } else {
                ActionView::User
            },
            config,
            is_in_repo,
            should_quit: false,
            dirty: false,
            repo_config,
            repo_config_path,
            repo_dirty: false,
            repo_name,
            branch_name,
            list_index: 0,
            modal: None,
            modal_stack: Vec::new(),
            cached_voices: Vec::new(),
            messenger_focus: 0,
            cached_agents,
            pending_test: None,
        }
    }

    /// Returns the list of available (installed) providers from the cached discovery.
    pub fn available_providers(&self) -> Vec<claudine::provider::Provider> {
        self.cached_agents
            .iter()
            .filter(|a| a.on_path)
            .map(|a| a.provider)
            .collect()
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if let Some(ref modal) = self.modal {
            self.handle_modal_key(key, modal.clone());
            return;
        }

        match self.mode {
            AppMode::Overview => self.handle_overview_key(key),
            AppMode::Detail => self.handle_detail_key(key),
        }
    }

    fn handle_overview_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::Right => {
                self.focused_tab = self.focused_tab.next();
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.focused_tab = self.focused_tab.prev();
            }
            KeyCode::Enter => {
                self.mode = AppMode::Detail;
                self.selected_tab = Some(self.focused_tab);
                self.list_index = 0;
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    fn handle_detail_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.mode = AppMode::Overview;
            self.selected_tab = None;
            return;
        }

        match self.focused_tab {
            Tab::Preferences => super::tabs::preferences::handle_key(self, key),
            Tab::Services => super::tabs::services::handle_key(self, key),
            Tab::Tts => super::tabs::tts::handle_key(self, key),
            Tab::Messenger => super::tabs::messenger::handle_key(self, key),
            Tab::Actions => super::tabs::actions::handle_key(self, key),
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent, modal: ModalState) {
        match modal {
            ModalState::AgentSelector { .. } => {
                super::tabs::preferences::handle_agent_selector_modal(self, key);
            }
            ModalState::UserProviderSelector { .. } => {
                super::tabs::preferences::handle_user_provider_modal(self, key);
            }
            ModalState::RepoProviderSelector { .. } => {
                super::tabs::preferences::handle_repo_provider_modal(self, key);
            }
            ModalState::SoundSelector { category, .. } => {
                super::tabs::preferences::handle_sound_selector_modal(self, key, category);
            }
            ModalState::ProtectRules { .. } => {
                super::tabs::services::handle_protect_rules_modal(self, key);
            }
            ModalState::EditActions { .. } => {
                super::tabs::actions::handle_edit_actions_modal(self, key);
            }
            ModalState::TtsProvider { .. } => {
                super::tabs::tts::handle_tts_provider_modal(self, key);
            }
            ModalState::VoiceSelector { .. } => {
                super::tabs::tts::handle_voice_selector_modal(self, key);
            }
            ModalState::MessengerSelect { .. } => {
                super::tabs::messenger::handle_messenger_select_modal(self, key);
            }
            ModalState::MessengerAdd { .. } => {
                super::tabs::messenger::handle_messenger_add_modal(self, key);
            }
            ModalState::EventSelector { .. } => {
                super::tabs::actions::handle_event_selector_modal(self, key);
            }
            ModalState::ActionTypeChooser { .. } => {
                super::tabs::actions::handle_action_type_chooser_modal(self, key);
            }
            ModalState::ConfirmDelete { .. } => {
                super::tabs::actions::handle_confirm_delete_modal(self, key);
            }
            ModalState::TextInput { .. } => {
                super::tabs::actions::handle_text_input_modal(self, key);
            }
            ModalState::ActionSoundSelector { .. } => {
                super::tabs::actions::handle_action_sound_selector_modal(self, key);
            }
            ModalState::MessengerInput { .. } => {
                super::tabs::messenger::handle_messenger_input_modal(self, key);
            }
            ModalState::ActionFieldList { .. } => {
                super::tabs::actions::handle_action_field_list_modal(self, key);
            }
            ModalState::ActionFieldInput { .. } => {
                super::tabs::actions::handle_action_field_input_modal(self, key);
            }
        }
    }

    pub fn modal_highlighted(&self) -> usize {
        match &self.modal {
            Some(ModalState::AgentSelector { highlighted }) => *highlighted,
            Some(ModalState::UserProviderSelector { highlighted }) => *highlighted,
            Some(ModalState::RepoProviderSelector { highlighted }) => *highlighted,
            Some(ModalState::SoundSelector { highlighted, .. }) => *highlighted,
            Some(ModalState::ProtectRules { highlighted, .. }) => *highlighted,
            Some(ModalState::EditActions { highlighted, .. }) => *highlighted,
            Some(ModalState::TtsProvider { highlighted }) => *highlighted,
            Some(ModalState::VoiceSelector { highlighted, .. }) => *highlighted,
            Some(ModalState::MessengerSelect { highlighted, .. }) => *highlighted,
            Some(ModalState::MessengerAdd { highlighted }) => *highlighted,
            Some(ModalState::EventSelector { highlighted }) => *highlighted,
            Some(ModalState::ActionTypeChooser { highlighted, .. }) => *highlighted,
            Some(ModalState::ConfirmDelete { .. }) => 0,
            Some(ModalState::TextInput { .. }) => 0,
            Some(ModalState::ActionSoundSelector { highlighted, .. }) => *highlighted,
            Some(ModalState::MessengerInput { .. }) => 0,
            Some(ModalState::ActionFieldList { highlighted, .. }) => *highlighted,
            Some(ModalState::ActionFieldInput { .. }) => 0,
            None => 0,
        }
    }

    pub fn set_modal_highlighted(&mut self, new_idx: usize) {
        if let Some(ref mut modal) = self.modal {
            match modal {
                ModalState::AgentSelector { highlighted } => *highlighted = new_idx,
                ModalState::UserProviderSelector { highlighted } => *highlighted = new_idx,
                ModalState::RepoProviderSelector { highlighted } => *highlighted = new_idx,
                ModalState::SoundSelector { highlighted, .. } => *highlighted = new_idx,
                ModalState::ProtectRules { highlighted, .. } => *highlighted = new_idx,
                ModalState::EditActions { highlighted, .. } => *highlighted = new_idx,
                ModalState::TtsProvider { highlighted } => *highlighted = new_idx,
                ModalState::VoiceSelector { highlighted, .. } => *highlighted = new_idx,
                ModalState::MessengerSelect { highlighted, .. } => *highlighted = new_idx,
                ModalState::MessengerAdd { highlighted } => *highlighted = new_idx,
                ModalState::EventSelector { highlighted } => *highlighted = new_idx,
                ModalState::ActionTypeChooser { highlighted, .. } => *highlighted = new_idx,
                ModalState::ConfirmDelete { .. } => {}
                ModalState::TextInput { .. } => {}
                ModalState::ActionSoundSelector { highlighted, .. } => *highlighted = new_idx,
                ModalState::MessengerInput { .. } => {}
                ModalState::ActionFieldList { highlighted, .. } => *highlighted = new_idx,
                ModalState::ActionFieldInput { .. } => {}
            }
        }
    }

    /// Push the current modal onto the stack and set a new one on top.
    pub fn push_modal(&mut self, new_modal: ModalState) {
        if let Some(current) = self.modal.take() {
            self.modal_stack.push(current);
        }
        self.modal = Some(new_modal);
    }

    /// Pop back one level: restore the parent modal from the stack.
    pub fn pop_modal(&mut self) {
        self.modal = self.modal_stack.pop();
    }

    /// Pop until EditActions is the current modal, or close all if none found.
    pub fn pop_to_edit_actions(&mut self) {
        loop {
            match &self.modal {
                Some(ModalState::EditActions { .. }) | None => return,
                _ => self.modal = self.modal_stack.pop(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::config::claudine_config::ClaudineConfig;
    use claudine::events::AgenticEvent;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn test_app() -> App {
        App::new(ClaudineConfig::default(), None, None, false, None, None)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn overview_enter_switches_to_detail_mode() {
        let mut app = test_app();
        app.focused_tab = Tab::Services;

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.mode, AppMode::Detail);
        assert_eq!(app.selected_tab, Some(Tab::Services));
        assert_eq!(app.list_index, 0);
    }

    #[test]
    fn detail_escape_returns_to_overview() {
        let mut app = test_app();
        app.mode = AppMode::Detail;
        app.selected_tab = Some(Tab::Actions);

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.mode, AppMode::Overview);
        assert_eq!(app.selected_tab, None);
    }

    #[test]
    fn push_and_pop_modal_restore_parent_state() {
        let mut app = test_app();
        app.modal = Some(ModalState::EditActions {
            event: AgenticEvent::SessionStart,
            highlighted: 1,
        });

        app.push_modal(ModalState::ActionTypeChooser {
            event: AgenticEvent::SessionStart,
            highlighted: 2,
        });
        assert!(matches!(
            app.modal,
            Some(ModalState::ActionTypeChooser { highlighted: 2, .. })
        ));
        assert_eq!(app.modal_stack.len(), 1);

        app.pop_modal();
        assert!(matches!(
            app.modal,
            Some(ModalState::EditActions { highlighted: 1, .. })
        ));
    }

    #[test]
    fn pop_to_edit_actions_unwinds_modal_stack() {
        let mut app = test_app();
        app.modal = Some(ModalState::EditActions {
            event: AgenticEvent::SessionStart,
            highlighted: 0,
        });
        app.push_modal(ModalState::ActionTypeChooser {
            event: AgenticEvent::SessionStart,
            highlighted: 0,
        });
        app.push_modal(ModalState::TextInput {
            event: AgenticEvent::SessionStart,
            action_type: 0,
            buffer: String::new(),
            label: "Prompt".to_string(),
            edit_index: None,
        });

        app.pop_to_edit_actions();

        assert!(matches!(app.modal, Some(ModalState::EditActions { .. })));
        assert!(app.modal_stack.is_empty());
    }
}
