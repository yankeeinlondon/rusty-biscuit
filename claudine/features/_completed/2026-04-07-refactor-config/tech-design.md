# Technical Design: Refactoring Claudine Config and Actions

This document outlines the technical implementation for refactoring Claudine's configuration model and updating the action system, as specified in the [Refactor Config Spec](./spec.md).

## Architecture Overview

The refactor moves Claudine from a per-provider event configuration to a canonical, cross-provider model. Configuration will be consolidated into a single `ClaudineConfig` struct, supported by the `biscuit-file` library for JSON5 deserialization.

Key architectural changes:
- **Centralized Configuration**: All global settings, service toggles (Logging, Protect), and canonical actions are stored in `~/.claudine/config.json`.
- **Service Abstraction**: Logging and Protect are treated as internal services rather than explicit per-event actions in the user config.
- **Canonical Dispatch**: The dispatch pipeline will use the `actions` map from the new config, indexed by `AgenticEvent`.
- **TUI-driven Configuration**: A new `claudine config` command replaces `claudine init`, providing a rich `ratatui` interface.

## 1. Data Structures

The new configuration schema will be implemented in `claudine/lib/src/config/claudine_config.rs`.

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::events::{AgenticEvent, Provider};
use crate::actions::HookAction;
use crate::services::protect::config::ProtectConfig;
use biscuit_speaks::{TtsProvider, Voice};
use messenger::provider::{DiscordConfig, SlackConfig, SignalConfig, WhatsAppConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudineConfig {
    /// How to handle TTS functionality on the host
    pub tts: TtsValue,
    
    /// What messaging platform configurations are available and active
    pub messenger: Option<ClaudineMessengerConfig>,
    
    /// whether or not to use the logging service
    pub logging: bool,
    
    /// whether or not to use the protect service
    pub protect: ProtectValue,
    
    /// Actions bound to canonical Claudine events (cross-provider)
    pub actions: HashMap<AgenticEvent, Vec<HookAction>>,
    
    /// the preferred agent to use for lazy composition operations
    pub preferred_agent: Provider,
    
    /// The canonical provider for this scope
    pub canonical_provider: Option<Provider>,
    
    /// Default sound effects for different outcomes
    #[serde(default)]
    pub default_sounds: DefaultSounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudineMessengerConfig {
    /// The key of the currently active messenger configuration
    pub active_config: Option<String>,
    /// A map of user-defined names (e.g., "My Personal Discord") to their provider settings
    #[serde(default)]
    pub configurations: HashMap<String, MessengerProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum MessengerProviderConfig {
    Discord(DiscordConfig),
    Slack(SlackConfig),
    Signal(SignalConfig),
    Whatsapp(WhatsAppConfig),
    // Extensible for other providers supported by the messenger crate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TtsValue {
    Boolean(bool),
    Config(TtsConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub provider: TtsProvider,
    pub voice: Option<VoiceSelection>,
    #[serde(default = "default_gender")]
    pub gender: Gender,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VoiceSelection {
    Single(String), // Voice ID
    Genders { male: String, female: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProtectValue {
    Boolean(bool),
    Config(ProtectConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultSounds {
    pub success: Option<String>,
    pub attention: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
}

fn default_gender() -> Gender {
    Gender::Female
}
```

### 1.1 Updated `HookAction`

The `HookAction` enum in `claudine/lib/src/actions/hook_action.rs` will be updated:

```rust
pub enum HookAction {
    /// Updated: 'name' -> 'effect'
    SoundEffect {
        effect: String,
        #[serde(default = "default_volume")]
        volume: f32,
        #[serde(default = "default_speed")]
        speed: f32,
    },
    
    /// Updated: added 'voice', 'gender'
    Speak {
        message: String,
        voice: Option<String>,
        gender: Option<Gender>,
    },
    
    /// New: replaces 'fire_and_forget'
    Bash {
        command: String,
        #[serde(default)]
        params: String,
    },
    
    // Existing actions retained as-is
    Message { ... },
    Report { ... },
    Call { ... },
    
    // Removed: Log (handled by logging service)
}
```

## 2. Configuration Management

### 2.1 User and Repo Scoping

`claudine-lib` will resolve configuration by:
1.  Loading `~/.claudine/config.json` (User Scope).
2.  If in a git repo, loading `{repo}/.claudine/config.json` (Repo Scope).
3.  Merging:
    - `canonical_provider` in Repo Scope overrides User Scope.
    - `actions` merge **per-event**. If the user has actions for `SessionStart` and the repo has actions for `BeforeTool`, both apply. If both scopes define actions for `BeforeTool`, the Repo Scope's `BeforeTool` array **fully replaces** the User Scope's `BeforeTool` array (no additive concatenation).

### 2.2 Migration Strategy

On startup, Claudine will check the format of `config.json`.
- If it detects the old per-provider format (e.g., presence of `claude`, `gemini` keys at root):
    - Rename `config.json` to `config.json.bak`.
    - Treat as "no config found" and trigger **Initialization Process**.

### 2.3 Default Initialization

Default `ProtectConfig` will have all 12 built-in rule groups enabled.
Default `logging` will be `true`.
Default `preferred_agent` will be the first available/installed agent found by `discover_agents_full()`.

## 3. Initialization Process

The `claudine` CLI will check for the existence of `~/.claudine/config.json` before executing any command (except `--help`). If missing, it runs the `InitializationProcess`.

**Non-Interactive / CI Safeguard:** Before prompting, Claudine must check if the environment is a TTY (`std::io::IsTerminal`). If it is running in a headless/CI environment, it must **bypass the interactive prompts** and silently write the default configuration (TTS: off, Logging: on, Protect: default) to prevent hanging.

### Step-by-Step Flow (Interactive):
1.  **Welcome & TTS**: 
    - Explain Claudine's use of TTS for attention and errors.
    - Auto-detect best TTS provider on host.
    - Inform user of choice or offer to install one if none found.
2.  **Messenger**:
    - Explain remote notifications via `messenger`.
    - Offer to configure or skip for later.
3.  **Preferred Agent**:
    - Present list of installed agents.
    - Ask user to select their "favorite" for lazy operations (`compose`, `inline-compose`).
4.  **Services (Logging & Protect)**:
    - Brief explanation of both services.
    - State they are enabled by default.
    - Confirmation prompt to acknowledge.
5.  **Canonical Actions**:
    - Explain the cross-provider event model.
    - Inform user about default `human-in-the-loop` sound effect.
6.  **Finalize**:
    - Write generated `ClaudineConfig` to `~/.claudine/config.json`.
    - Print file location and recommendation to use `claudine config`.

## 4. `claudine config` TUI

Implemented using `ratatui` and `crossterm`.

### State Management
```rust
struct AppState {
    mode: AppMode,
    current_tab: Tab,
    focused_tab: Tab,
    config: ClaudineConfig,
    is_in_repo: bool,
    active_modal: Option<Modal>,
    // Selection state within tabs
    tab_state: TabState,
}

enum AppMode { Overview, Detail }
enum Tab { Preferences, Services, Actions, TTS, Messenger }
enum Modal { SoundSelector, AgentSelector, FeatureSelector, ... }
```

### Tab Details
- **Preferences**: Dropdowns for Preferred Agent, User/Repo Canonical Provider. Sound effect modals for defaults.
- **Services**: Toggles for Logging/Protect. "C" opens Protect Feature Modal (vertical checkbox list).
- **TTS**: Toggle T, Provider P, Voice F/M, Gender SHIFT+F/M.
  - *Note:* If the user changes the `TtsProvider`, the application must automatically clear out any currently selected voice strings and reset them to the new provider's default voices, as voice IDs do not map across providers.
- **Messenger**: Select Box for active config, Add button for new ones.
- **Actions**: Vertical list of events with configured actions. ENTER/E opens Event Modal to manage individual actions.

## 5. Action System Refactor

### 5.1 `Bash` Action Execution
- Validation: Check if executable exists on PATH or is a valid path.
- Special handling for JS/TS:
    - Check for shebang.
    - Fallback: `bun` (if present) -> `node` (for JS).
- Parameters: Handlebars-style interpolation of event metadata variables.
  - **Security Critical:** Interpolated string variables must be strictly shell-escaped (e.g., wrapped in single quotes with internal quotes escaped) prior to command execution to prevent arbitrary shell injection.

### 5.2 `Speak` Action
- Integration with `biscuit-speaks`:
    - Use configured `TtsProvider` and `Voice`.
    - Support gender override in the action itself.

## 6. Testing Strategy

- **Unit Tests**:
    - Serialization/Deserialization of `ClaudineConfig` with JSON5.
    - Migration logic (detecting old config, backing up).
    - `Bash` action command discovery, validation, and shell-escaping.
- **Integration Tests**:
    - Mocking `biscuit-speaks` and `playa` to verify actions trigger correctly.
    - Verification of canonical event dispatching with the new flattened `actions` map.
- **TUI Testing**:
    - Component rendering tests.
    - State transition validation (Overview -> Detail -> Modal).
