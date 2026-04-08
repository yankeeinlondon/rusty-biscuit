use crossterm::event::{KeyCode, KeyEvent};

use claudine::config::claudine_config::ClaudineConfig;

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

pub struct App {
    pub mode: AppMode,
    pub focused_tab: Tab,
    pub selected_tab: Option<Tab>,
    pub config: ClaudineConfig,
    pub is_in_repo: bool,
    pub should_quit: bool,
    pub dirty: bool,
    pub repo_config: Option<ClaudineConfig>,
    pub repo_config_path: Option<std::path::PathBuf>,
    pub repo_dirty: bool,
    pub repo_name: Option<String>,
    pub branch_name: Option<String>,
    pub list_index: usize,
    pub modal: Option<ModalState>,
    pub modal_stack: Vec<ModalState>,
    pub cached_voices: Vec<(String, biscuit_speaks::VoiceQuality)>,
    pub messenger_focus: usize,
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
        repo_config: Option<ClaudineConfig>,
        repo_config_path: Option<std::path::PathBuf>,
        is_in_repo: bool,
        repo_name: Option<String>,
        branch_name: Option<String>,
    ) -> Self {
        Self {
            mode: AppMode::Overview,
            focused_tab: Tab::Preferences,
            selected_tab: None,
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
        }
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
        }
    }

    pub fn modal_highlighted(&self) -> usize {
        match &self.modal {
            Some(ModalState::AgentSelector { highlighted }) => *highlighted,
            Some(ModalState::UserProviderSelector { highlighted }) => *highlighted,
            Some(ModalState::RepoProviderSelector { highlighted }) => *highlighted,
            Some(ModalState::SoundSelector { highlighted, .. }) => *highlighted,
            Some(ModalState::ProtectRules { highlighted }) => *highlighted,
            Some(ModalState::EditActions { highlighted, .. }) => *highlighted,
            Some(ModalState::TtsProvider { highlighted }) => *highlighted,
            Some(ModalState::VoiceSelector { highlighted, .. }) => *highlighted,
            Some(ModalState::MessengerSelect { highlighted }) => *highlighted,
            Some(ModalState::MessengerAdd { highlighted }) => *highlighted,
            Some(ModalState::EventSelector { highlighted }) => *highlighted,
            Some(ModalState::ActionTypeChooser { highlighted, .. }) => *highlighted,
            Some(ModalState::ConfirmDelete { .. }) => 0,
            Some(ModalState::TextInput { .. }) => 0,
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
                ModalState::ProtectRules { highlighted } => *highlighted = new_idx,
                ModalState::EditActions { highlighted, .. } => *highlighted = new_idx,
                ModalState::TtsProvider { highlighted } => *highlighted = new_idx,
                ModalState::VoiceSelector { highlighted, .. } => *highlighted = new_idx,
                ModalState::MessengerSelect { highlighted } => *highlighted = new_idx,
                ModalState::MessengerAdd { highlighted } => *highlighted = new_idx,
                ModalState::EventSelector { highlighted } => *highlighted = new_idx,
                ModalState::ActionTypeChooser { highlighted, .. } => *highlighted = new_idx,
                ModalState::ConfirmDelete { .. } => {}
                ModalState::TextInput { .. } => {}
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
