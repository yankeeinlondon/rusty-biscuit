use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::Json5;
use regex::Regex;
use tracing::{debug, info, warn};

use crate::actions::{CompiledMapper, HookAction, Mapper};
use crate::config::atomic::atomic_write;
use crate::config::claudine_config::{ClaudineConfig, ClaudineMessengerConfig, MessengerProviderConfig, RepoOverrideConfig};
use crate::config::migration;
use crate::error::{ClaudineError, Result};
use crate::events::{
    AgenticEvent, CanonicalProviderSettings, EventBinding, GlobalSettings, HookerConfig,
    LinkingSettings, Provider, ProviderConfig,
};
use crate::messaging::{MessagingRouteConfig, RuntimeMessagingSettings, ScopedMessagingSettings};
use crate::services::protect::catalog::ProtectPlatform;
use crate::services::protect::config::{ProtectConfig, ProtectRuleToggles};
use crate::services::protect::service::ProtectService;

/// Candidate file names for user-level configuration.
const USER_CONFIG_NAMES: &[&str] = &[".claudine/config.json"];

/// Repo-level config file name.
const REPO_CONFIG_NAME: &str = ".claudine/config.json";

/// Runtime configuration with precompiled matcher and mapper regexes.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    settings: GlobalSettings,
    messaging: RuntimeMessagingSettings,
    providers: HashMap<Provider, RuntimeProviderConfig>,
    protect_service: Option<ProtectService>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RuntimeProviderConfig {
    events: HashMap<AgenticEvent, RuntimeEventBinding>,
}

/// Event binding ready for dispatch without per-event regex compilation.
#[derive(Debug, Clone)]
pub struct RuntimeEventBinding {
    enabled: bool,
    actions: Vec<HookAction>,
    matcher: Option<Regex>,
    compiled_mappers: Vec<Option<CompiledMapper>>,
}

impl RuntimeConfig {
    /// Get global settings.
    pub fn settings(&self) -> &GlobalSettings {
        &self.settings
    }

    /// Get runtime messaging settings.
    pub fn messaging(&self) -> &RuntimeMessagingSettings {
        &self.messaging
    }

    /// Get the cached protect service, if available.
    pub fn protect_service(&self) -> Option<&ProtectService> {
        self.protect_service.as_ref()
    }

    /// Get an event binding for a specific provider and event.
    pub fn get_binding(
        &self,
        provider: Provider,
        event: &AgenticEvent,
    ) -> Option<&RuntimeEventBinding> {
        self.providers
            .get(&provider)
            .and_then(|provider_config| provider_config.events.get(event))
    }
}

impl RuntimeConfig {
    /// Build a RuntimeConfig directly for testing.
    #[cfg(test)]
    pub fn new_for_test(
        settings: GlobalSettings,
        providers: HashMap<Provider, RuntimeProviderConfig>,
        protect_service: Option<ProtectService>,
    ) -> Self {
        Self {
            settings,
            messaging: RuntimeMessagingSettings::default(),
            providers,
            protect_service,
        }
    }
}

impl RuntimeEventBinding {
    /// Whether the binding is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Actions to execute for this binding.
    pub fn actions(&self) -> &[HookAction] {
        &self.actions
    }

    /// Precompiled matcher regex for this binding.
    pub fn matcher(&self) -> Option<&Regex> {
        self.matcher.as_ref()
    }

    /// Per-action compiled mapper metadata aligned with [`Self::actions`].
    pub fn compiled_mappers(&self) -> &[Option<CompiledMapper>] {
        &self.compiled_mappers
    }

    /// Build a RuntimeEventBinding directly for testing.
    #[cfg(test)]
    pub fn new_for_test(enabled: bool, actions: Vec<HookAction>, matcher: Option<Regex>) -> Self {
        let compiled_mappers = vec![None; actions.len()];
        Self {
            enabled,
            actions,
            matcher,
            compiled_mappers,
        }
    }
}

#[cfg(test)]
impl RuntimeProviderConfig {
    pub fn new_for_test(events: HashMap<AgenticEvent, RuntimeEventBinding>) -> Self {
        Self { events }
    }
}

/// Load and merge Claudine configuration.
///
/// Resolves `~/.claudine/config.json` (user) then merges with
/// `.claudine/config.json` (repo).
/// Repo-level provider/event bindings replace user-level.
/// Settings merge field-by-field.
///
/// ## Errors
///
/// Returns `ConfigNotFound` when no configuration file is found at any
/// expected location.
pub fn load_config(user: Option<&Path>, repo_root: Option<&Path>) -> Result<HookerConfig> {
    let user_config = load_user_config(user)?;
    let repo_config = load_repo_config(repo_root)?;

    let config = match (user_config, repo_config) {
        (Some(user_cfg), Some(repo_cfg)) => {
            debug!("Merging user and repo configurations");
            merge_configs(user_cfg, repo_cfg)
        }
        (Some(cfg), None) => cfg,
        (None, Some(cfg)) => cfg,
        (None, None) => {
            let path = user.map(PathBuf::from).unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join(USER_CONFIG_NAMES[0])
            });
            return Err(ClaudineError::ConfigNotFound(path));
        }
    };

    config.validate()?;
    Ok(config)
}

/// Load configuration and compile regex-based runtime structures.
///
/// This compiles:
/// - Event matchers from `EventBinding.matcher`
/// - Call action regex mappers from `Mapper::Regex`
///
/// Invalid regex patterns fail at load time with contextual error messages.
pub fn load_runtime_config(user: Option<&Path>, repo_root: Option<&Path>) -> Result<RuntimeConfig> {
    let user_config = load_user_config(user)?;
    let repo_config = load_repo_config(repo_root)?;

    // Capture messaging scopes before merging
    let user_messaging = user_config
        .as_ref()
        .and_then(|c| c.settings.messaging.clone());
    let repo_messaging = repo_config
        .as_ref()
        .and_then(|c| c.settings.messaging.clone());

    // Validate messaging per-scope before merging
    if let Some(ref messaging) = user_messaging {
        messaging.validate("user")?;
    }
    if let Some(ref messaging) = repo_messaging {
        messaging.validate("repo")?;
    }

    let config = match (user_config, repo_config) {
        (Some(user_cfg), Some(repo_cfg)) => {
            debug!("Merging user and repo configurations");
            merge_configs(user_cfg, repo_cfg)
        }
        (Some(cfg), None) => cfg,
        (None, Some(cfg)) => cfg,
        (None, None) => {
            let path = user.map(PathBuf::from).unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("~"))
                    .join(USER_CONFIG_NAMES[0])
            });
            return Err(ClaudineError::ConfigNotFound(path));
        }
    };

    config.validate()?;
    compile_runtime_config_with_messaging(config, user_messaging, repo_messaging)
}

#[allow(dead_code)]
fn compile_runtime_config(config: HookerConfig) -> Result<RuntimeConfig> {
    compile_runtime_config_with_messaging(config, None, None)
}

fn compile_runtime_config_with_messaging(
    config: HookerConfig,
    user_messaging: Option<crate::messaging::ScopedMessagingSettings>,
    repo_messaging: Option<crate::messaging::ScopedMessagingSettings>,
) -> Result<RuntimeConfig> {
    let HookerConfig {
        version: _,
        settings,
        providers,
    } = config;

    let mut runtime_providers = HashMap::new();

    for (provider, provider_config) in providers {
        let mut runtime_events = HashMap::new();

        for (event, binding) in provider_config.events {
            let matcher = binding
                .matcher
                .as_deref()
                .map(|pattern| {
                    Regex::new(pattern).map_err(|error| {
                        ClaudineError::TemplateError(format!(
                            "invalid matcher regex for provider={provider} event={event}: {error} ({pattern})"
                        ))
                    })
                })
                .transpose()?;

            let compiled_mappers = binding
                .actions
                .iter()
                .map(|action| compile_action_mapper(action, provider, event))
                .collect::<Result<Vec<_>>>()?;

            runtime_events.insert(
                event,
                RuntimeEventBinding {
                    enabled: binding.enabled,
                    actions: binding.actions,
                    matcher,
                    compiled_mappers,
                },
            );
        }

        runtime_providers.insert(
            provider,
            RuntimeProviderConfig {
                events: runtime_events,
            },
        );
    }

    let protect_service = settings
        .protect
        .as_ref()
        .map(|protect| ProtectService::new(protect.clone(), ProtectPlatform::current()))
        .transpose()?;

    Ok(RuntimeConfig {
        settings,
        messaging: RuntimeMessagingSettings {
            user: user_messaging,
            repo: repo_messaging,
        },
        providers: runtime_providers,
        protect_service,
    })
}

fn compile_action_mapper(
    action: &HookAction,
    provider: Provider,
    event: AgenticEvent,
) -> Result<Option<CompiledMapper>> {
    let HookAction::Call { mapper, .. } = action else {
        return Ok(None);
    };

    mapper
        .as_ref()
        .map(|mapper| compile_mapper(mapper, provider, event))
        .transpose()
}

fn compile_mapper(
    mapper: &Mapper,
    provider: Provider,
    event: AgenticEvent,
) -> Result<CompiledMapper> {
    match mapper {
        Mapper::JsonField { field } => Ok(CompiledMapper::JsonField {
            field: field.clone(),
        }),
        Mapper::JsonObject => Ok(CompiledMapper::JsonObject),
        Mapper::ExitCode => Ok(CompiledMapper::ExitCode),
        Mapper::Regex { pattern } => {
            let compiled = Regex::new(pattern).map_err(|error| {
                ClaudineError::TemplateError(format!(
                    "invalid mapper regex for provider={provider} event={event}: {error} ({pattern})"
                ))
            })?;
            Ok(CompiledMapper::Regex { pattern: compiled })
        }
    }
}

// ==========================================================================
// CanonicalRuntimeConfig (compiled from the new ClaudineConfig)
// ==========================================================================

/// Runtime configuration compiled from the new [`ClaudineConfig`].
///
/// Unlike the old [`RuntimeConfig`] which indexes by provider+event,
/// this indexes by canonical event only.
#[derive(Debug, Clone)]
pub struct CanonicalRuntimeConfig {
    pub(crate) config: ClaudineConfig,
    pub(crate) messaging: RuntimeMessagingSettings,
    pub(crate) protect_service: Option<ProtectService>,
    pub(crate) events: HashMap<AgenticEvent, RuntimeEventBinding>,
}

impl CanonicalRuntimeConfig {
    /// Get the underlying [`ClaudineConfig`].
    pub fn config(&self) -> &ClaudineConfig {
        &self.config
    }

    /// Get runtime messaging settings.
    pub fn messaging(&self) -> &RuntimeMessagingSettings {
        &self.messaging
    }

    /// Get the cached protect service, if available.
    pub fn protect_service(&self) -> Option<&ProtectService> {
        self.protect_service.as_ref()
    }

    /// Get an event binding for a canonical event.
    pub fn get_binding(&self, event: &AgenticEvent) -> Option<&RuntimeEventBinding> {
        self.events.get(event)
    }
}

/// Compile a [`ClaudineConfig`] into a [`CanonicalRuntimeConfig`].
///
/// This iterates the flat event→actions map, compiles regex mappers for
/// `Call` actions, builds the protect service if enabled, and bridges
/// messenger settings to the existing [`RuntimeMessagingSettings`] type.
pub fn compile_canonical_runtime(
    config: ClaudineConfig,
    _repo_root: Option<&Path>,
) -> Result<CanonicalRuntimeConfig> {
    // 1. Compile event bindings
    let mut events = HashMap::new();
    for (event, actions) in &config.actions {
        let compiled_mappers = actions
            .iter()
            .map(|action| compile_canonical_action_mapper(action, *event))
            .collect::<Result<Vec<_>>>()?;

        events.insert(
            *event,
            RuntimeEventBinding {
                enabled: true,
                actions: actions.clone(),
                matcher: None,
                compiled_mappers,
            },
        );
    }

    // 2. Build ProtectService if enabled
    let protect_service = if config.protect.enabled {
        Some(ProtectService::new(
            config.protect.clone(),
            ProtectPlatform::current(),
        )?)
    } else {
        None
    };

    // 3. Bridge messenger config to RuntimeMessagingSettings
    let messaging = config
        .messenger
        .as_ref()
        .map(bridge_messenger_to_runtime)
        .unwrap_or_default();

    Ok(CanonicalRuntimeConfig {
        config,
        messaging,
        protect_service,
        events,
    })
}

/// Compile mapper metadata for a single action in the canonical config.
///
/// Same logic as [`compile_action_mapper`] but without a provider context,
/// since canonical events are provider-agnostic.
fn compile_canonical_action_mapper(
    action: &HookAction,
    event: AgenticEvent,
) -> Result<Option<CompiledMapper>> {
    let HookAction::Call { mapper, .. } = action else {
        return Ok(None);
    };

    mapper
        .as_ref()
        .map(|mapper| compile_canonical_mapper(mapper, event))
        .transpose()
}

/// Compile a single mapper without provider context.
fn compile_canonical_mapper(mapper: &Mapper, event: AgenticEvent) -> Result<CompiledMapper> {
    match mapper {
        Mapper::JsonField { field } => Ok(CompiledMapper::JsonField {
            field: field.clone(),
        }),
        Mapper::JsonObject => Ok(CompiledMapper::JsonObject),
        Mapper::ExitCode => Ok(CompiledMapper::ExitCode),
        Mapper::Regex { pattern } => {
            let compiled = Regex::new(pattern).map_err(|error| {
                ClaudineError::TemplateError(format!(
                    "invalid mapper regex for event={event}: {error} ({pattern})"
                ))
            })?;
            Ok(CompiledMapper::Regex { pattern: compiled })
        }
    }
}

/// Bridge [`ClaudineConfig`] TTS settings to legacy [`GlobalSettings`].
///
/// Constructs a minimal [`GlobalSettings`] containing only the TTS
/// configuration, suitable for [`LifecycleRuntimeContext`].
pub fn bridge_tts_settings(config: &ClaudineConfig) -> GlobalSettings {
    use crate::config::claudine_config::{TtsValue, VoiceSelection, Gender};
    use crate::events::TtsSettings;

    let tts = match &config.tts {
        TtsValue::Boolean(false) => None,
        TtsValue::Boolean(true) => Some(TtsSettings {
            provider: None,
            voice: None,
            rate: None,
        }),
        TtsValue::Config(cfg) => {
            let voice = match &cfg.voice {
                Some(VoiceSelection::Single(v)) => Some(v.clone()),
                Some(VoiceSelection::Gendered { male, female }) => match cfg.gender {
                    Gender::Male => Some(male.clone()),
                    Gender::Female => Some(female.clone()),
                },
                None => None,
            };
            Some(TtsSettings {
                provider: Some(cfg.provider.clone()),
                voice,
                rate: None,
            })
        }
    };
    GlobalSettings {
        tts,
        ..GlobalSettings::default()
    }
}

/// Bridge [`ClaudineConfig`] messenger settings to [`RuntimeMessagingSettings`].
///
/// Reuses [`bridge_messenger_to_runtime`] from [`compile_canonical_runtime`].
pub fn bridge_messaging_settings(config: &ClaudineConfig) -> RuntimeMessagingSettings {
    config
        .messenger
        .as_ref()
        .map(bridge_messenger_to_runtime)
        .unwrap_or_default()
}

/// Bridge [`ClaudineMessengerConfig`] to [`RuntimeMessagingSettings`].
///
/// The new config uses [`MessengerProviderConfig`] variants while the
/// existing runtime uses [`MessagingRouteConfig`]. This function converts
/// between the two and wraps the result as a user-scope
/// [`ScopedMessagingSettings`].
fn bridge_messenger_to_runtime(messenger: &ClaudineMessengerConfig) -> RuntimeMessagingSettings {
    let configs: HashMap<String, MessagingRouteConfig> = messenger
        .configurations
        .iter()
        .map(|(name, provider_cfg)| (name.clone(), bridge_provider_config(provider_cfg)))
        .collect();

    let scoped = ScopedMessagingSettings {
        active: messenger.active_config.clone(),
        configs,
    };

    RuntimeMessagingSettings {
        user: Some(scoped),
        repo: None,
    }
}

/// Convert a single [`MessengerProviderConfig`] to [`MessagingRouteConfig`].
fn bridge_provider_config(cfg: &MessengerProviderConfig) -> MessagingRouteConfig {
    match cfg {
        MessengerProviderConfig::Discord {
            channel_id,
            bot_token_env,
        } => MessagingRouteConfig::Discord {
            channel_id: channel_id.clone(),
            bot_token: None,
            bot_token_env: bot_token_env.clone(),
        },
        MessengerProviderConfig::Slack {
            channel_id,
            bot_token_env,
        } => MessagingRouteConfig::Slack {
            channel_id: channel_id.clone(),
            bot_token: None,
            bot_token_env: bot_token_env.clone(),
        },
        MessengerProviderConfig::Signal {
            recipient,
            rpc_url_env,
            account_env,
        } => MessagingRouteConfig::Signal {
            recipient: recipient.clone(),
            rpc_url: None,
            rpc_url_env: rpc_url_env.clone(),
            account: None,
            account_env: account_env.clone(),
        },
        MessengerProviderConfig::Whatsapp {
            recipient,
            access_token_env,
            phone_number_id_env,
        } => MessagingRouteConfig::WhatsApp {
            recipient: recipient.clone(),
            access_token: None,
            access_token_env: access_token_env.clone(),
            phone_number_id: None,
            phone_number_id_env: phone_number_id_env.clone(),
        },
    }
}

/// Attempt to load the user-level config.
///
/// If an explicit path is given, read that file.
/// Otherwise, search the home directory for known config filenames.
fn load_user_config(explicit: Option<&Path>) -> Result<Option<HookerConfig>> {
    if let Some(path) = explicit {
        if path.is_file() {
            let content = std::fs::read_to_string(path)?;
            let config: HookerConfig = serde_json::from_str(&content)?;
            debug!(?path, "Loaded user config");
            return Ok(Some(config));
        }
        return Ok(None);
    }

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            warn!("Could not determine home directory");
            return Ok(None);
        }
    };

    for name in USER_CONFIG_NAMES {
        let path = home.join(name);
        if path.is_file() {
            let content = std::fs::read_to_string(&path)?;
            let config: HookerConfig = serde_json::from_str(&content)?;
            debug!(?path, "Loaded user config");
            return Ok(Some(config));
        }
    }

    Ok(None)
}

/// Attempt to load the repo-level config.
///
/// Looks for `.claudine/config.json` in the given repo root. Returns `None` if
/// no repo root is provided or the file doesn't exist.
///
/// Repo config must explicitly define `settings.linking.canonical_provider`.
fn load_repo_config(repo_root: Option<&Path>) -> Result<Option<HookerConfig>> {
    let Some(root) = repo_root else {
        return Ok(None);
    };
    let path = root.join(REPO_CONFIG_NAME);

    if !path.is_file() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)?;
    let raw: serde_json::Value = serde_json::from_str(&content)?;
    validate_repo_linking_config(&raw, &path)?;
    let config: HookerConfig = serde_json::from_value(raw)?;
    debug!(?path, "Loaded repo config");
    Ok(Some(config))
}

fn validate_repo_linking_config(raw: &serde_json::Value, path: &Path) -> Result<()> {
    if raw
        .pointer("/settings/linking/canonical_provider")
        .is_some()
    {
        return Ok(());
    }

    Err(ClaudineError::ConfigValidation(format!(
        "repo config {} must define settings.linking.canonical_provider",
        path.display()
    )))
}

/// Get the path to the user config file.
///
/// Returns the first existing config file path, or the default path if none exists.
pub fn user_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));

    for name in USER_CONFIG_NAMES {
        let path = home.join(name);
        if path.is_file() {
            return path;
        }
    }

    // Return default path
    home.join(USER_CONFIG_NAMES[0])
}

/// Save the configuration to the user config file.
///
/// Writes the config to `~/.claudine/config.json` (or existing config location)
/// using atomic writes.
///
/// ## Errors
///
/// Returns errors if serialization fails or the file cannot be written.
pub fn save_config(config: &HookerConfig) -> Result<PathBuf> {
    let path = user_config_path();
    let content = serde_json::to_string_pretty(config)?;
    atomic_write(&path, content.as_bytes())?;
    info!(?path, "Saved config");
    Ok(path)
}

// ==========================================================================
// ClaudineConfig (new flat format) loading, merging, and saving
// ==========================================================================

/// Load and validate a [`ClaudineConfig`].
///
/// If `user_path` is `Some`, it is used directly; otherwise the default
/// user config path is resolved via [`user_config_path()`].
///
/// When the file contains the old per-provider format, it is backed up to
/// `<path>.bak` and [`ClaudineError::ConfigNotFound`] is returned so that
/// the caller can run the interactive migration wizard.
///
/// If `repo_root` is provided, the repo-level config at
/// `{repo_root}/.claudine/config.json` is loaded and merged on top of the
/// user config.
///
/// ## Errors
///
/// Returns [`ClaudineError::ConfigNotFound`] when the config file does not
/// exist or was detected as the old format and backed up.
/// Returns other errors for I/O failures, parse errors, or validation
/// failures.
pub fn load_claudine_config(
    user_path: Option<&Path>,
    repo_root: Option<&Path>,
) -> Result<ClaudineConfig> {
    let path = user_path
        .map(PathBuf::from)
        .unwrap_or_else(user_config_path);

    if !path.is_file() {
        return Err(ClaudineError::ConfigNotFound(path));
    }

    let raw = std::fs::read_to_string(&path)?;
    let value = parse_json5_to_value(&raw)?;

    if migration::is_old_format(&value) {
        migration::backup_old_config(&path)?;
        return Err(ClaudineError::ConfigNotFound(path));
    }

    let mut config: ClaudineConfig =
        serde_json::from_value(value).map_err(ClaudineError::JsonParse)?;
    debug!(?path, "Loaded ClaudineConfig (user)");

    // Merge repo-level config if present.
    // Skip when the repo config resolves to the same file as the user config
    // (e.g., HOME points at the repo root) to avoid re-parsing the same file
    // as a different type.
    if let Some(root) = repo_root {
        let repo_path = root.join(REPO_CONFIG_NAME);
        let same_file = repo_path
            .canonicalize()
            .ok()
            .zip(path.canonicalize().ok())
            .is_some_and(|(r, u)| r == u);
        if repo_path.is_file() && !same_file {
            let repo_raw = std::fs::read_to_string(&repo_path)?;
            let repo_value = parse_json5_to_value(&repo_raw)?;
            if migration::is_old_format(&repo_value) {
                migration::backup_old_config(&repo_path)?;
                warn!(?repo_path, "Repo config was old format; backed up and ignored");
            } else {
                let repo_override: RepoOverrideConfig =
                    serde_json::from_value(repo_value).map_err(ClaudineError::JsonParse)?;
                debug!(?repo_path, "Loaded RepoOverrideConfig");
                merge_repo_override(&mut config, &repo_override);
            }
        }
    }

    config.validate()?;
    Ok(config)
}

/// Save a [`ClaudineConfig`] to disk as pretty-printed JSON.
///
/// Creates parent directories if they do not exist and writes atomically.
///
/// ## Errors
///
/// Returns errors if serialization fails or the file cannot be written.
pub fn save_claudine_config(config: &ClaudineConfig, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(config)?;
    atomic_write(path, json.as_bytes())?;
    info!(?path, "Saved ClaudineConfig");
    Ok(())
}

/// Load a repo-scoped [`RepoOverrideConfig`] from the given path.
///
/// Returns `Ok(None)` if the file does not exist.
///
/// ## Errors
///
/// Returns errors for I/O failures or parse errors.
pub fn load_repo_override_config(path: &Path) -> Result<Option<RepoOverrideConfig>> {
    if !path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    let value = parse_json5_to_value(&raw)?;
    if migration::is_old_format(&value) {
        migration::backup_old_config(path)?;
        warn!(?path, "Repo override config was old format; backed up and ignored");
        return Ok(None);
    }
    let config: RepoOverrideConfig =
        serde_json::from_value(value).map_err(ClaudineError::JsonParse)?;
    debug!(?path, "Loaded RepoOverrideConfig");
    Ok(Some(config))
}

/// Save a [`RepoOverrideConfig`] to disk as pretty-printed JSON.
///
/// Creates parent directories if they do not exist and writes atomically.
///
/// ## Errors
///
/// Returns errors if serialization fails or the file cannot be written.
pub fn save_repo_override_config(config: &RepoOverrideConfig, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(config)?;
    atomic_write(path, json.as_bytes())?;
    info!(?path, "Saved RepoOverrideConfig");
    Ok(())
}

/// Convert a [`ClaudineConfig`] into a [`HookerConfig`] for backward
/// compatibility with [`AgentConfigurator::register()`].
///
/// Since `ClaudineConfig` stores events provider-agnostically, the
/// resulting `HookerConfig` creates a [`ProviderConfig`] for each
/// specified provider, all sharing the same event bindings from
/// [`ClaudineConfig::actions`].
///
/// Settings are populated with defaults — configurator implementations
/// only access the per-provider event bindings, not global settings.
pub fn claudine_config_to_hooker(
    config: &ClaudineConfig,
    providers: &[Provider],
) -> HookerConfig {
    let mut provider_configs = HashMap::new();
    for &provider in providers {
        let events: HashMap<AgenticEvent, EventBinding> = config
            .actions
            .iter()
            .map(|(event, actions)| {
                (
                    *event,
                    EventBinding {
                        enabled: true,
                        actions: actions.clone(),
                        matcher: None,
                    },
                )
            })
            .collect();
        provider_configs.insert(provider, ProviderConfig { events });
    }

    HookerConfig {
        version: "2.0".to_string(),
        settings: GlobalSettings {
            protect: Some(config.protect.clone()),
            ..GlobalSettings::default()
        },
        providers: provider_configs,
    }
}

/// Merge a repo-level [`RepoOverrideConfig`] into a user-level config.
///
/// Merge rules:
/// - `canonical_provider`: repo overrides user if repo has `Some`.
/// - `actions`: per-event replacement — if repo defines actions for an event,
///   that vector fully replaces the user's entry for the same event.
fn merge_repo_override(user: &mut ClaudineConfig, repo: &RepoOverrideConfig) {
    // canonical_provider: repo overrides user if set
    if repo.canonical_provider.is_some() {
        user.canonical_provider = repo.canonical_provider;
    }

    // actions: per-event replacement
    for (event, repo_actions) in &repo.actions {
        user.actions.insert(*event, repo_actions.clone());
    }

    // active_messenger: repo overrides the active config key only
    if let Some(ref override_value) = repo.active_messenger {
        if let Some(ref mut messenger) = user.messenger {
            messenger.active_config = override_value.clone();
        }
    }
}

/// Parse a raw string as JSON5 and return a [`serde_json::Value`].
fn parse_json5_to_value(raw: &str) -> Result<serde_json::Value> {
    let json5 = Json5::from_str(raw).map_err(|e| {
        ClaudineError::ConfigValidation(format!("JSON5 parse error: {e}"))
    })?;
    Ok(json5.as_json_value().clone())
}

/// Remove unsupported events from the config.
///
/// For each provider, removes events that are not supported via hooks.
/// Returns a list of `(Provider, Vec<removed_event_names>)` for reporting.
///
/// ## Example
///
/// If Codex has `tool_error` and `subagent_stop` configured but doesn't support them
/// via hooks, those events will be removed from the Codex config.
pub fn remove_unsupported_events(config: &mut HookerConfig) -> Vec<(Provider, Vec<String>)> {
    let mut removed: Vec<(Provider, Vec<String>)> = Vec::new();

    for (provider, provider_config) in config.providers.iter_mut() {
        let unsupported_events: Vec<_> = provider_config
            .events
            .keys()
            .filter(|event| !provider.supports_event_via_hook(event))
            .cloned()
            .collect();

        if !unsupported_events.is_empty() {
            let event_names: Vec<String> =
                unsupported_events.iter().map(|e| e.to_string()).collect();

            // Remove the unsupported events
            for event in &unsupported_events {
                provider_config.events.remove(event);
            }

            removed.push((*provider, event_names));
        }
    }

    removed
}

/// Merge two configs: repo provider configs replace user provider configs,
/// settings merge field-by-field (repo overrides user).
fn merge_configs(user: HookerConfig, repo: HookerConfig) -> HookerConfig {
    let mut providers = user.providers;

    // Repo provider configs completely replace matching user provider configs
    for (provider, repo_provider_config) in repo.providers {
        providers.insert(provider, repo_provider_config);
    }

    // Settings: repo fields override user fields individually
    let protect = merge_protect_configs(
        user.settings.protect.as_ref(),
        repo.settings.protect.as_ref(),
    );

    let linking = merge_linking_settings(
        user.settings.linking.as_ref(),
        repo.settings.linking.as_ref(),
    );

    let settings = crate::events::GlobalSettings {
        default_log_target: repo
            .settings
            .default_log_target
            .or(user.settings.default_log_target),
        tts: repo.settings.tts.or(user.settings.tts),
        linking,
        protect,
        messaging: repo.settings.messaging.or(user.settings.messaging),
    };

    HookerConfig {
        version: repo.version,
        settings,
        providers,
    }
}

fn merge_protect_configs(
    user: Option<&ProtectConfig>,
    repo: Option<&ProtectConfig>,
) -> Option<ProtectConfig> {
    match (user, repo) {
        (None, None) => None,
        (Some(u), None) => Some(u.clone()),
        (None, Some(r)) => Some(r.clone()),
        (Some(user_cfg), Some(repo_cfg)) => {
            let enabled = user_cfg.enabled || repo_cfg.enabled;
            let rules = merge_rule_toggles(&user_cfg.rules, &repo_cfg.rules);

            // Combine custom patterns: repo first, then user
            let mut custom_patterns = repo_cfg.custom_patterns.clone();
            custom_patterns.extend(user_cfg.custom_patterns.iter().cloned());

            Some(ProtectConfig {
                enabled,
                rules,
                custom_patterns,
            })
        }
    }
}

/// Merge rule toggles: repo overrides user per-group.
fn merge_rule_toggles(user: &ProtectRuleToggles, repo: &ProtectRuleToggles) -> ProtectRuleToggles {
    ProtectRuleToggles {
        filesystem_destruction: repo
            .filesystem_destruction
            .clone()
            .or_else(|| user.filesystem_destruction.clone()),
        disk_manipulation: repo
            .disk_manipulation
            .clone()
            .or_else(|| user.disk_manipulation.clone()),
        remote_execution: repo
            .remote_execution
            .clone()
            .or_else(|| user.remote_execution.clone()),
        git_destructive: repo
            .git_destructive
            .clone()
            .or_else(|| user.git_destructive.clone()),
        system_sabotage: repo
            .system_sabotage
            .clone()
            .or_else(|| user.system_sabotage.clone()),
        network_sabotage: repo
            .network_sabotage
            .clone()
            .or_else(|| user.network_sabotage.clone()),
        container_cloud: repo
            .container_cloud
            .clone()
            .or_else(|| user.container_cloud.clone()),
        database_nukes: repo
            .database_nukes
            .clone()
            .or_else(|| user.database_nukes.clone()),
        obfuscated_execution: repo
            .obfuscated_execution
            .clone()
            .or_else(|| user.obfuscated_execution.clone()),
        prompt_injection: repo
            .prompt_injection
            .clone()
            .or_else(|| user.prompt_injection.clone()),
        credential_exfiltration: repo
            .credential_exfiltration
            .clone()
            .or_else(|| user.credential_exfiltration.clone()),
        sensitive_paths: repo
            .sensitive_paths
            .clone()
            .or_else(|| user.sensitive_paths.clone()),
    }
}

/// Merge linking settings field-by-field.
///
/// `preference` uses repo if non-empty, otherwise user.
/// `canonical_provider` merges slot-by-slot: repo non-`None` values override user.
fn merge_linking_settings(
    user: Option<&LinkingSettings>,
    repo: Option<&LinkingSettings>,
) -> Option<LinkingSettings> {
    match (user, repo) {
        (None, None) => None,
        (Some(u), None) => Some(u.clone()),
        (None, Some(r)) => Some(r.clone()),
        (Some(u), Some(r)) => Some(LinkingSettings {
            preference: if r.preference.is_empty() {
                u.preference.clone()
            } else {
                r.preference.clone()
            },
            canonical_provider: merge_canonical_providers(
                &u.canonical_provider,
                &r.canonical_provider,
            ),
        }),
    }
}

/// Merge canonical provider slots: repo non-`None` values override user.
fn merge_canonical_providers(
    user: &CanonicalProviderSettings,
    repo: &CanonicalProviderSettings,
) -> CanonicalProviderSettings {
    CanonicalProviderSettings {
        user_skill: repo.user_skill.or(user.user_skill),
        user_command: repo.user_command.or(user.user_command),
        user_agent: repo.user_agent.or(user.user_agent),
        user_script: repo.user_script.or(user.user_script),
        repo_skill: repo.repo_skill.or(user.repo_skill),
        repo_command: repo.repo_command.or(user.repo_command),
        repo_agent: repo.repo_agent.or(user.repo_agent),
        repo_script: repo.repo_script.or(user.repo_script),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::*;
    use crate::config::claudine_config::DefaultSounds;
    use crate::events::*;
    use crate::services::protect::config::RuleGroupDetailedConfig;
    use crate::services::protect::{
        CustomPattern, ProtectConfig, ProtectRuleToggles, RuleGroupConfig,
    };

    use std::collections::HashMap;
    use std::path::PathBuf;

    fn write_cfg(dir: &Path, name: &str, config: &HookerConfig) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(config).unwrap()).unwrap();
        path
    }

    fn empty_cfg() -> HookerConfig {
        HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            providers: HashMap::new(),
        }
    }

    fn speak_binding(msg: &str) -> EventBinding {
        EventBinding {
            enabled: true,
            actions: vec![HookAction::Speak {
                message: msg.to_string(),
                voice: None,
                gender: None,
            }],
            matcher: None,
        }
    }

    #[test]
    fn load_config_from_explicit_user_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = empty_cfg();

        let mut claude_config = ProviderConfig::default();
        claude_config
            .events
            .insert(AgenticEvent::SessionStart, speak_binding("hello"));
        config.providers.insert(Provider::Claude, claude_config);

        let path = write_cfg(tmp.path(), ".claudine/config.json", &config);

        let loaded = load_config(Some(&path), None).unwrap();
        assert_eq!(loaded.version, "1.0");
        assert!(loaded.providers.contains_key(&Provider::Claude));
        assert!(
            loaded.providers[&Provider::Claude]
                .events
                .contains_key(&AgenticEvent::SessionStart)
        );
    }

    #[test]
    fn missing_config_returns_config_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let result = load_config(Some(&tmp.path().join("nonexistent")), None);
        assert!(matches!(
            result.unwrap_err(),
            ClaudineError::ConfigNotFound(_)
        ));
    }

    #[test]
    fn merge_repo_providers_replace_user_providers() {
        let tmp = tempfile::tempdir().unwrap();

        // User config: Claude has session_start and turn_complete
        let mut user_config = empty_cfg();
        let mut claude_user = ProviderConfig::default();
        claude_user
            .events
            .insert(AgenticEvent::SessionStart, speak_binding("user session"));
        claude_user
            .events
            .insert(AgenticEvent::TurnComplete, speak_binding("user turn"));
        user_config.providers.insert(Provider::Claude, claude_user);
        let user_path = write_cfg(tmp.path(), "user-config", &user_config);

        // Repo config: Claude only has session_start (replaces entirely)
        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let mut repo_config = empty_cfg();
        repo_config.settings.linking = Some(LinkingSettings {
            preference: vec![],
            canonical_provider: CanonicalProviderSettings {
                repo_skill: Some(Provider::Claude),
                ..CanonicalProviderSettings::default()
            },
        });
        let mut claude_repo = ProviderConfig::default();
        claude_repo
            .events
            .insert(AgenticEvent::SessionStart, speak_binding("repo session"));
        repo_config.providers.insert(Provider::Claude, claude_repo);
        write_cfg(&repo_dir, ".claudine/config.json", &repo_config);

        let loaded = load_config(Some(&user_path), Some(&repo_dir)).unwrap();

        // Claude config from repo replaces user entirely
        let claude = &loaded.providers[&Provider::Claude];
        assert_eq!(claude.events.len(), 1); // Only session_start, no turn_complete
        match &claude.events[&AgenticEvent::SessionStart].actions[0] {
            HookAction::Speak { message, .. } => assert_eq!(message, "repo session"),
            _ => panic!("Expected Speak"),
        }
    }

    #[test]
    fn merge_preserves_non_overlapping_providers() {
        let tmp = tempfile::tempdir().unwrap();

        // User config: Claude and Codex
        let mut user_config = empty_cfg();
        let mut claude_user = ProviderConfig::default();
        claude_user
            .events
            .insert(AgenticEvent::SessionStart, speak_binding("claude hello"));
        user_config.providers.insert(Provider::Claude, claude_user);

        let mut codex_user = ProviderConfig::default();
        codex_user
            .events
            .insert(AgenticEvent::TurnComplete, speak_binding("codex done"));
        user_config.providers.insert(Provider::Codex, codex_user);
        let user_path = write_cfg(tmp.path(), "user-config", &user_config);

        // Repo config: Only Gemini
        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let mut repo_config = empty_cfg();
        repo_config.settings.linking = Some(LinkingSettings {
            preference: vec![],
            canonical_provider: CanonicalProviderSettings {
                repo_skill: Some(Provider::Gemini),
                ..CanonicalProviderSettings::default()
            },
        });
        let mut gemini_repo = ProviderConfig::default();
        gemini_repo
            .events
            .insert(AgenticEvent::BeforeTool, speak_binding("gemini tool"));
        repo_config.providers.insert(Provider::Gemini, gemini_repo);
        write_cfg(&repo_dir, ".claudine/config.json", &repo_config);

        let loaded = load_config(Some(&user_path), Some(&repo_dir)).unwrap();

        // All three providers should be present
        assert!(loaded.providers.contains_key(&Provider::Claude));
        assert!(loaded.providers.contains_key(&Provider::Codex));
        assert!(loaded.providers.contains_key(&Provider::Gemini));
    }

    #[test]
    fn merge_settings_field_by_field() {
        let user = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                default_log_target: Some(LogTarget::File {
                    path: Some(PathBuf::from("/tmp/user.jsonl")),
                    rotate_daily: false,
                }),
                tts: Some(TtsSettings {
                    provider: Some("say".to_string()),
                    voice: Some("Samantha".to_string()),
                    rate: None,
                }),
                linking: None,
                protect: None,
                messaging: None,
            },
            providers: HashMap::new(),
        };
        let repo = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                default_log_target: None,
                tts: Some(TtsSettings {
                    provider: Some("espeak".to_string()),
                    voice: None,
                    rate: Some(1.5),
                }),
                linking: None,
                protect: None,
                messaging: None,
            },
            providers: HashMap::new(),
        };
        let merged = merge_configs(user, repo);
        assert!(matches!(
            &merged.settings.default_log_target,
            Some(LogTarget::File { .. })
        ));
        let tts = merged.settings.tts.unwrap();
        assert_eq!(tts.provider.as_deref(), Some("espeak"));
        assert_eq!(tts.rate, Some(1.5));
    }

    #[test]
    fn merge_linking_preserves_user_canonical_when_repo_sets_repo_slots() {
        let user = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                linking: Some(LinkingSettings {
                    preference: vec![Provider::Claude, Provider::Codex],
                    canonical_provider: CanonicalProviderSettings {
                        user_skill: Some(Provider::Claude),
                        user_command: Some(Provider::Claude),
                        user_agent: Some(Provider::Claude),
                        ..CanonicalProviderSettings::default()
                    },
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let repo = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                linking: Some(LinkingSettings {
                    preference: vec![Provider::Claude, Provider::Gemini],
                    canonical_provider: CanonicalProviderSettings {
                        repo_skill: Some(Provider::Claude),
                        repo_command: Some(Provider::Claude),
                        repo_agent: Some(Provider::Claude),
                        ..CanonicalProviderSettings::default()
                    },
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let merged = merge_configs(user, repo);
        let linking = merged.settings.linking.expect("missing linking");

        // Repo preference wins
        assert_eq!(linking.preference, vec![Provider::Claude, Provider::Gemini]);

        // User-scoped canonical providers survive from user config
        assert_eq!(
            linking.canonical_provider.user_skill,
            Some(Provider::Claude)
        );
        assert_eq!(
            linking.canonical_provider.user_command,
            Some(Provider::Claude)
        );
        assert_eq!(
            linking.canonical_provider.user_agent,
            Some(Provider::Claude)
        );

        // Repo-scoped canonical providers come from repo config
        assert_eq!(
            linking.canonical_provider.repo_skill,
            Some(Provider::Claude)
        );
        assert_eq!(
            linking.canonical_provider.repo_command,
            Some(Provider::Claude)
        );
        assert_eq!(
            linking.canonical_provider.repo_agent,
            Some(Provider::Claude)
        );
    }

    #[test]
    fn merge_linking_repo_overrides_user_canonical_when_both_set() {
        let user = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                linking: Some(LinkingSettings {
                    preference: vec![],
                    canonical_provider: CanonicalProviderSettings {
                        user_skill: Some(Provider::Claude),
                        repo_skill: Some(Provider::Codex),
                        ..CanonicalProviderSettings::default()
                    },
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let repo = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                linking: Some(LinkingSettings {
                    preference: vec![],
                    canonical_provider: CanonicalProviderSettings {
                        repo_skill: Some(Provider::Claude),
                        ..CanonicalProviderSettings::default()
                    },
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let merged = merge_configs(user, repo);
        let linking = merged.settings.linking.expect("missing linking");

        // user_skill preserved from user config (repo didn't set it)
        assert_eq!(
            linking.canonical_provider.user_skill,
            Some(Provider::Claude)
        );
        // repo_skill overridden by repo config
        assert_eq!(
            linking.canonical_provider.repo_skill,
            Some(Provider::Claude)
        );
    }

    #[test]
    fn merge_linking_user_preference_used_when_repo_empty() {
        let user = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                linking: Some(LinkingSettings {
                    preference: vec![Provider::Claude, Provider::Codex],
                    canonical_provider: CanonicalProviderSettings::default(),
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let repo = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                linking: Some(LinkingSettings {
                    preference: vec![],
                    canonical_provider: CanonicalProviderSettings {
                        repo_skill: Some(Provider::Claude),
                        ..CanonicalProviderSettings::default()
                    },
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let merged = merge_configs(user, repo);
        let linking = merged.settings.linking.expect("missing linking");

        // User preference used since repo's is empty
        assert_eq!(linking.preference, vec![Provider::Claude, Provider::Codex]);
    }

    #[test]
    fn load_config_repo_only() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let mut config = empty_cfg();
        config.settings.linking = Some(LinkingSettings {
            preference: vec![],
            canonical_provider: CanonicalProviderSettings {
                repo_skill: Some(Provider::Claude),
                ..CanonicalProviderSettings::default()
            },
        });
        let mut claude_config = ProviderConfig::default();
        claude_config.events.insert(
            AgenticEvent::BeforeTool,
            EventBinding {
                enabled: true,
                actions: vec![],
                matcher: Some("Bash".to_string()),
            },
        );
        config.providers.insert(Provider::Claude, claude_config);
        write_cfg(&repo_dir, ".claudine/config.json", &config);

        let loaded = load_config(Some(&tmp.path().join("nope")), Some(&repo_dir)).unwrap();
        assert!(loaded.providers.contains_key(&Provider::Claude));
        assert!(
            loaded.providers[&Provider::Claude]
                .events
                .contains_key(&AgenticEvent::BeforeTool)
        );
    }

    #[test]
    fn load_config_fails_when_repo_missing_canonical_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let user_path = write_cfg(tmp.path(), "user-config", &empty_cfg());
        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let repo_json = serde_json::json!({
            "version": "1.0",
            "settings": {
                "linking": {
                    "preference": ["claude", "codex"]
                }
            },
            "providers": {}
        });
        let repo_config_path = repo_dir.join(".claudine/config.json");
        if let Some(parent) = repo_config_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(
            &repo_config_path,
            serde_json::to_string_pretty(&repo_json).unwrap(),
        )
        .unwrap();

        let error = load_config(Some(&user_path), Some(&repo_dir)).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("settings.linking.canonical_provider"));
    }

    #[test]
    fn load_config_accepts_repo_when_canonical_provider_key_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let user_path = write_cfg(tmp.path(), "user-config", &empty_cfg());
        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let repo_json = serde_json::json!({
            "version": "1.0",
            "settings": {
                "linking": {
                    "canonical_provider": {
                        "repo_skill": "claude"
                    }
                }
            },
            "providers": {}
        });
        let repo_config_path = repo_dir.join(".claudine/config.json");
        if let Some(parent) = repo_config_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(
            &repo_config_path,
            serde_json::to_string_pretty(&repo_json).unwrap(),
        )
        .unwrap();

        let loaded = load_config(Some(&user_path), Some(&repo_dir)).unwrap();
        let linking = loaded.settings.linking.expect("missing linking settings");
        assert_eq!(
            linking.canonical_provider.repo_skill,
            Some(Provider::Claude)
        );
    }

    #[test]
    fn remove_unsupported_events_removes_from_codex() {
        let mut config = empty_cfg();

        // Codex only supports turn_complete via hook
        // tool_error and subagent_stop are not supported via hook
        let mut codex_config = ProviderConfig::default();
        codex_config
            .events
            .insert(AgenticEvent::TurnComplete, speak_binding("turn done"));
        codex_config
            .events
            .insert(AgenticEvent::ToolError, speak_binding("tool error"));
        codex_config
            .events
            .insert(AgenticEvent::SubagentStop, speak_binding("subagent stop"));
        config.providers.insert(Provider::Codex, codex_config);

        let removed = remove_unsupported_events(&mut config);

        // Should have removed 2 events from Codex
        assert_eq!(removed.len(), 1);
        let (provider, events) = &removed[0];
        assert_eq!(*provider, Provider::Codex);
        assert_eq!(events.len(), 2);
        assert!(events.contains(&"tool_error".to_string()));
        assert!(events.contains(&"subagent_stop".to_string()));

        // Config should only have turn_complete remaining
        let codex = config.providers.get(&Provider::Codex).unwrap();
        assert_eq!(codex.events.len(), 1);
        assert!(codex.events.contains_key(&AgenticEvent::TurnComplete));
    }

    #[test]
    fn remove_unsupported_events_preserves_supported() {
        let mut config = empty_cfg();

        // Claude supports all these events via hook
        let mut claude_config = ProviderConfig::default();
        claude_config
            .events
            .insert(AgenticEvent::TurnComplete, speak_binding("turn done"));
        claude_config
            .events
            .insert(AgenticEvent::BeforeTool, speak_binding("before tool"));
        claude_config
            .events
            .insert(AgenticEvent::SessionStart, speak_binding("session start"));
        config.providers.insert(Provider::Claude, claude_config);

        let removed = remove_unsupported_events(&mut config);

        // Nothing should be removed - Claude supports all these via hooks
        assert!(removed.is_empty());

        // All events should still be present
        let claude = config.providers.get(&Provider::Claude).unwrap();
        assert_eq!(claude.events.len(), 3);
    }

    #[test]
    fn remove_unsupported_events_handles_multiple_providers() {
        let mut config = empty_cfg();

        // Codex: turn_complete supported, tool_error not
        let mut codex_config = ProviderConfig::default();
        codex_config
            .events
            .insert(AgenticEvent::TurnComplete, speak_binding("codex done"));
        codex_config
            .events
            .insert(AgenticEvent::ToolError, speak_binding("codex error"));
        config.providers.insert(Provider::Codex, codex_config);

        // OpenCode: turn_complete and before_tool supported, subagent_stop not
        let mut opencode_config = ProviderConfig::default();
        opencode_config
            .events
            .insert(AgenticEvent::TurnComplete, speak_binding("opencode done"));
        opencode_config
            .events
            .insert(AgenticEvent::BeforeTool, speak_binding("opencode tool"));
        opencode_config.events.insert(
            AgenticEvent::SubagentStop,
            speak_binding("opencode subagent"),
        );
        config.providers.insert(Provider::OpenCode, opencode_config);

        let removed = remove_unsupported_events(&mut config);

        // Should have removed events from both providers
        assert_eq!(removed.len(), 2);

        // Verify Codex had tool_error removed
        let codex = config.providers.get(&Provider::Codex).unwrap();
        assert_eq!(codex.events.len(), 1);
        assert!(codex.events.contains_key(&AgenticEvent::TurnComplete));

        // Verify OpenCode had subagent_stop removed
        let opencode = config.providers.get(&Provider::OpenCode).unwrap();
        assert_eq!(opencode.events.len(), 2);
        assert!(opencode.events.contains_key(&AgenticEvent::TurnComplete));
        assert!(opencode.events.contains_key(&AgenticEvent::BeforeTool));
    }

    #[test]
    fn user_config_path_returns_default_when_none_exists() {
        // This test just verifies the function doesn't panic
        let path = user_config_path();
        assert!(path.to_string_lossy().contains(".claudine"));
    }

    #[test]
    fn load_runtime_config_precompiles_matcher_and_mapper_regex() {
        let tmp = tempfile::tempdir().unwrap();
        let config = serde_json::json!({
            "version": "1.0",
            "settings": {},
            "providers": {
                "claude": {
                    "events": {
                        "before_tool": {
                            "enabled": true,
                            "matcher": "Bash|Edit",
                            "actions": [
                                {
                                    "type": "call",
                                    "command": "echo",
                                    "args": ["allow because safe"],
                                    "mapper": {
                                        "type": "regex",
                                        "pattern": "(?P<decision>allow|deny)\\s+because\\s+(?P<reason>.*)"
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        });

        let path = tmp.path().join(".claudine/config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();

        let runtime = load_runtime_config(Some(&path), None).unwrap();
        let binding = runtime
            .get_binding(Provider::Claude, &AgenticEvent::BeforeTool)
            .expect("missing runtime binding");

        assert!(binding.matcher().is_some());
        assert_eq!(binding.actions().len(), 1);
        assert_eq!(binding.compiled_mappers().len(), 1);
        assert!(binding.compiled_mappers()[0].is_some());
    }

    #[test]
    fn load_runtime_config_fails_on_invalid_matcher_regex() {
        let tmp = tempfile::tempdir().unwrap();
        let config = serde_json::json!({
            "version": "1.0",
            "settings": {},
            "providers": {
                "claude": {
                    "events": {
                        "before_tool": {
                            "enabled": true,
                            "matcher": "[invalid(",
                            "actions": [
                                { "type": "speak", "message": "hello" }
                            ]
                        }
                    }
                }
            }
        });

        let path = tmp.path().join(".claudine/config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();

        let error = load_runtime_config(Some(&path), None).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("invalid matcher regex"));
        assert!(message.contains("before_tool"));
    }

    #[test]
    fn load_runtime_config_fails_on_invalid_mapper_regex() {
        let tmp = tempfile::tempdir().unwrap();
        let config = serde_json::json!({
            "version": "1.0",
            "settings": {},
            "providers": {
                "claude": {
                    "events": {
                        "before_tool": {
                            "enabled": true,
                            "actions": [
                                {
                                    "type": "call",
                                    "command": "echo",
                                    "mapper": {
                                        "type": "regex",
                                        "pattern": "[invalid("
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        });

        let path = tmp.path().join(".claudine/config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();

        let error = load_runtime_config(Some(&path), None).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("invalid mapper regex"));
        assert!(message.contains("before_tool"));
    }

    #[test]
    fn load_config_fails_fast_on_unknown_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let json = serde_json::json!({
            "version": "1.0",
            "settings": {},
            "providers": {
                "claude": {
                    "events": {
                        "session_start": {
                            "enabled": true,
                            "actions": [],
                            "legacy_field": true
                        }
                    }
                }
            }
        });
        let path = tmp.path().join(".claudine/config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();

        let error = load_config(Some(&path), None).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("unknown field"));
        assert!(message.contains("legacy_field"));
    }

    #[test]
    fn runtime_config_preserves_messaging_scopes() {
        use crate::messaging::{MessagingRouteConfig, ScopedMessagingSettings};

        let user_config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                messaging: Some(ScopedMessagingSettings {
                    active: Some("my-slack".to_string()),
                    configs: std::collections::HashMap::from([(
                        "my-slack".to_string(),
                        MessagingRouteConfig::Slack {
                            channel_id: "C123".to_string(),
                            bot_token: None,
                            bot_token_env: "SLACK_BOT_TOKEN".to_string(),
                        },
                    )]),
                }),
                ..Default::default()
            },
            providers: std::collections::HashMap::new(),
        };

        let repo_config = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                messaging: Some(ScopedMessagingSettings {
                    active: Some("alerts".to_string()),
                    configs: std::collections::HashMap::from([(
                        "alerts".to_string(),
                        MessagingRouteConfig::Discord {
                            channel_id: "999".to_string(),
                            bot_token: None,
                            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                        },
                    )]),
                }),
                ..Default::default()
            },
            providers: std::collections::HashMap::new(),
        };

        let merged = merge_configs(user_config.clone(), repo_config.clone());
        let runtime = compile_runtime_config_with_messaging(
            merged,
            user_config.settings.messaging,
            repo_config.settings.messaging,
        )
        .unwrap();

        let messaging = runtime.messaging();
        assert!(messaging.user.is_some());
        assert!(messaging.repo.is_some());
        assert_eq!(
            messaging.repo.as_ref().unwrap().active.as_deref(),
            Some("alerts")
        );
        assert_eq!(
            messaging.user.as_ref().unwrap().active.as_deref(),
            Some("my-slack")
        );
    }

    #[test]
    fn merge_protect_preserves_user_custom_patterns_when_repo_has_config() {
        let user = ProtectConfig {
            enabled: true,
            rules: ProtectRuleToggles::default(),
            custom_patterns: vec![CustomPattern {
                name: "user_pattern".to_string(),
                pattern: "user_danger".to_string(),
            }],
        };
        let repo = ProtectConfig {
            enabled: true,
            rules: ProtectRuleToggles::default(),
            custom_patterns: vec![],
        };
        let merged = merge_protect_configs(Some(&user), Some(&repo)).unwrap();
        assert_eq!(
            merged.custom_patterns.len(),
            1,
            "user custom_patterns should be preserved"
        );
        assert_eq!(merged.custom_patterns[0].name, "user_pattern");
    }

    #[test]
    fn merge_protect_combines_custom_patterns_from_both_scopes() {
        let user = ProtectConfig {
            enabled: true,
            rules: ProtectRuleToggles::default(),
            custom_patterns: vec![CustomPattern {
                name: "user_pattern".to_string(),
                pattern: "user_danger".to_string(),
            }],
        };
        let repo = ProtectConfig {
            enabled: true,
            rules: ProtectRuleToggles::default(),
            custom_patterns: vec![CustomPattern {
                name: "repo_pattern".to_string(),
                pattern: "repo_danger".to_string(),
            }],
        };
        let merged = merge_protect_configs(Some(&user), Some(&repo)).unwrap();
        assert_eq!(
            merged.custom_patterns.len(),
            2,
            "both custom_patterns should be present"
        );
    }

    #[test]
    fn merge_protect_preserves_user_group_toggles() {
        let mut user = ProtectConfig::default();
        user.rules.git_destructive = Some(RuleGroupConfig::Toggle(false));
        let repo = ProtectConfig::default();
        let merged = merge_protect_configs(Some(&user), Some(&repo)).unwrap();
        assert_eq!(
            merged.rules.git_destructive,
            Some(RuleGroupConfig::Toggle(false)),
            "user group toggle should be preserved when repo doesn't set it"
        );
    }

    #[test]
    fn merge_protect_repo_group_toggle_overrides_user() {
        let mut user = ProtectConfig::default();
        user.rules.git_destructive = Some(RuleGroupConfig::Toggle(false));
        let mut repo = ProtectConfig::default();
        repo.rules.git_destructive = Some(RuleGroupConfig::Toggle(true));
        let merged = merge_protect_configs(Some(&user), Some(&repo)).unwrap();
        assert_eq!(
            merged.rules.git_destructive,
            Some(RuleGroupConfig::Toggle(true)),
            "repo group toggle should override user"
        );
    }

    #[test]
    fn merge_protect_preserves_user_allow_paths() {
        let mut user = ProtectConfig::default();
        user.rules.filesystem_destruction =
            Some(RuleGroupConfig::Detailed(RuleGroupDetailedConfig {
                enabled: true,
                allow_paths: vec!["node_modules".to_string()],
            }));
        let repo = ProtectConfig::default();
        let merged = merge_protect_configs(Some(&user), Some(&repo)).unwrap();
        match &merged.rules.filesystem_destruction {
            Some(RuleGroupConfig::Detailed(d)) => {
                assert!(d.allow_paths.contains(&"node_modules".to_string()));
            }
            other => panic!("expected Detailed, got {other:?}"),
        }
    }

    #[test]
    fn merge_protect_user_only_returns_user() {
        let mut user = ProtectConfig::default();
        user.custom_patterns = vec![CustomPattern {
            name: "test".to_string(),
            pattern: "test".to_string(),
        }];
        let merged = merge_protect_configs(Some(&user), None).unwrap();
        assert_eq!(merged.custom_patterns.len(), 1);
    }

    #[test]
    fn merge_protect_repo_only_returns_repo() {
        let mut repo = ProtectConfig::default();
        repo.rules.git_destructive = Some(RuleGroupConfig::Toggle(false));
        let merged = merge_protect_configs(None, Some(&repo)).unwrap();
        assert_eq!(
            merged.rules.git_destructive,
            Some(RuleGroupConfig::Toggle(false))
        );
    }

    #[test]
    fn merge_protect_none_none_returns_none() {
        assert!(merge_protect_configs(None, None).is_none());
    }

    #[test]
    fn runtime_config_propagates_protect_service_error() {
        let tmp = tempfile::tempdir().unwrap();
        let config = serde_json::json!({
            "version": "1.0",
            "settings": {
                "protect": {
                    "enabled": true,
                    "custom_patterns": [
                        { "name": "bad", "pattern": "[invalid(" }
                    ]
                }
            },
            "providers": {}
        });

        let path = tmp.path().join(".claudine/config.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();

        let result = load_runtime_config(Some(&path), None);
        assert!(
            result.is_err(),
            "should propagate ProtectService construction error, not swallow it"
        );
    }

    // =====================================================================
    // ClaudineConfig loading / saving / merging tests
    // =====================================================================

    #[test]
    fn load_claudine_config_from_json5() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".claudine");
        std::fs::create_dir_all(&config_dir).unwrap();

        let json5_content = r#"{
            // TTS auto-detect
            tts: true,
            logging: true,
            protect: true,
            preferred_agent: "claude",
            actions: {
                human_in_the_loop: [
                    { type: "sound_effect", effect: "doorbell", },
                ],
            },
        }"#;
        std::fs::write(config_dir.join("config.json"), json5_content).unwrap();

        let config =
            load_claudine_config(Some(&config_dir.join("config.json")), None).unwrap();
        assert!(config.logging);
        assert!(config
            .actions
            .contains_key(&crate::events::AgenticEvent::HumanInTheLoop));
    }

    #[test]
    fn load_claudine_config_detects_old_format() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".claudine");
        std::fs::create_dir_all(&config_dir).unwrap();

        let old = r#"{"version":"1.0","settings":{},"providers":{}}"#;
        let path = config_dir.join("config.json");
        std::fs::write(&path, old).unwrap();

        let result = load_claudine_config(Some(&path), None);
        assert!(result.is_err());
        assert!(config_dir.join("config.json.bak").exists());
    }

    #[test]
    fn merge_repo_canonical_provider_overrides_user() {
        let mut user = ClaudineConfig {
            canonical_provider: Some(Provider::Claude),
            ..ClaudineConfig::default()
        };
        let repo = RepoOverrideConfig {
            canonical_provider: Some(Provider::Gemini),
            ..RepoOverrideConfig::default()
        };
        merge_repo_override(&mut user, &repo);
        assert_eq!(user.canonical_provider, Some(Provider::Gemini));
    }

    #[test]
    fn merge_repo_actions_replace_user_per_event() {
        let mut user = ClaudineConfig::default();
        user.actions.insert(
            AgenticEvent::SessionStart,
            vec![HookAction::SoundEffect {
                effect: "user-sound".to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
        );
        user.actions.insert(
            AgenticEvent::TurnComplete,
            vec![HookAction::Report { handler: None }],
        );

        let repo = RepoOverrideConfig {
            actions: std::collections::HashMap::from([(
                AgenticEvent::SessionStart,
                vec![HookAction::SoundEffect {
                    effect: "repo-sound".to_string(),
                    volume: 0.5,
                    speed: 1.0,
                }],
            )]),
            ..RepoOverrideConfig::default()
        };

        merge_repo_override(&mut user, &repo);

        // SessionStart replaced by repo
        let session_start = &user.actions[&AgenticEvent::SessionStart];
        assert_eq!(session_start.len(), 1);
        if let HookAction::SoundEffect { effect, .. } = &session_start[0] {
            assert_eq!(effect, "repo-sound");
        } else {
            panic!("Expected SoundEffect");
        }

        // TurnComplete untouched
        assert!(user.actions.contains_key(&AgenticEvent::TurnComplete));
    }

    #[test]
    fn merge_repo_override_applies_active_messenger() {
        use crate::config::claudine_config::{ClaudineMessengerConfig, MessengerProviderConfig};

        let mut user = ClaudineConfig::default();
        user.messenger = Some(ClaudineMessengerConfig {
            active_config: Some("personal".to_string()),
            configurations: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "personal".to_string(),
                    MessengerProviderConfig::Discord {
                        channel_id: "123".to_string(),
                        bot_token_env: "TOKEN".to_string(),
                    },
                );
                m.insert(
                    "work".to_string(),
                    MessengerProviderConfig::Slack {
                        channel_id: "C456".to_string(),
                        bot_token_env: "SLACK_URL".to_string(),
                    },
                );
                m
            },
        });

        let repo = RepoOverrideConfig {
            active_messenger: Some(Some("work".to_string())),
            ..Default::default()
        };

        merge_repo_override(&mut user, &repo);

        assert_eq!(
            user.messenger.as_ref().unwrap().active_config.as_deref(),
            Some("work"),
        );
    }

    #[test]
    fn merge_repo_override_disables_messenger_with_null() {
        use crate::config::claudine_config::ClaudineMessengerConfig;

        let mut user = ClaudineConfig::default();
        user.messenger = Some(ClaudineMessengerConfig {
            active_config: Some("personal".to_string()),
            configurations: std::collections::HashMap::new(),
        });

        let repo = RepoOverrideConfig {
            active_messenger: Some(None),
            ..Default::default()
        };

        merge_repo_override(&mut user, &repo);

        assert_eq!(user.messenger.as_ref().unwrap().active_config, None);
    }

    #[test]
    fn save_and_reload_claudine_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".claudine/config.json");

        let config = ClaudineConfig::default();
        save_claudine_config(&config, &path).unwrap();

        let loaded = load_claudine_config(Some(&path), None).unwrap();
        assert_eq!(loaded.preferred_agent, config.preferred_agent);
        assert_eq!(loaded.logging, config.logging);
    }

    // =====================================================================
    // CanonicalRuntimeConfig tests
    // =====================================================================

    #[test]
    fn compile_canonical_runtime_indexes_by_event() {
        let mut config = ClaudineConfig::default();
        config.actions.insert(
            AgenticEvent::HumanInTheLoop,
            vec![HookAction::SoundEffect {
                effect: "doorbell".to_string(),
                volume: 1.0,
                speed: 1.0,
            }],
        );
        config.default_sounds = DefaultSounds::default();

        let runtime = compile_canonical_runtime(config, None).unwrap();
        assert!(runtime.get_binding(&AgenticEvent::HumanInTheLoop).is_some());
        assert!(runtime.get_binding(&AgenticEvent::SessionStart).is_none());
    }

    #[test]
    fn compile_canonical_runtime_builds_protect() {
        let mut config = ClaudineConfig::default();
        config.protect.enabled = true;
        config.default_sounds = DefaultSounds::default();

        let runtime = compile_canonical_runtime(config, None).unwrap();
        assert!(runtime.protect_service().is_some());
    }

    #[test]
    fn compile_canonical_runtime_no_protect_when_disabled() {
        let mut config = ClaudineConfig::default();
        config.protect.enabled = false;
        config.default_sounds = DefaultSounds::default();

        let runtime = compile_canonical_runtime(config, None).unwrap();
        assert!(runtime.protect_service().is_none());
    }

    #[test]
    fn compile_canonical_runtime_compiles_call_mappers() {
        let mut config = ClaudineConfig::default();
        config.protect.enabled = false;
        config.default_sounds = DefaultSounds::default();
        config.actions.insert(
            AgenticEvent::BeforeTool,
            vec![HookAction::Call {
                command: "echo".to_string(),
                args: Some(vec!["allow because safe".to_string()]),
                mapper: Some(Mapper::Regex {
                    pattern: r"(?P<decision>allow|deny)\s+because\s+(?P<reason>.*)".to_string(),
                }),
                timeout_ms: None,
            }],
        );

        let runtime = compile_canonical_runtime(config, None).unwrap();
        let binding = runtime
            .get_binding(&AgenticEvent::BeforeTool)
            .expect("missing binding");
        assert_eq!(binding.actions().len(), 1);
        assert_eq!(binding.compiled_mappers().len(), 1);
        assert!(binding.compiled_mappers()[0].is_some());
    }

    #[test]
    fn compile_canonical_runtime_fails_on_invalid_mapper_regex() {
        let mut config = ClaudineConfig::default();
        config.protect.enabled = false;
        config.default_sounds = DefaultSounds::default();
        config.actions.insert(
            AgenticEvent::BeforeTool,
            vec![HookAction::Call {
                command: "echo".to_string(),
                args: None,
                mapper: Some(Mapper::Regex {
                    pattern: "[invalid(".to_string(),
                }),
                timeout_ms: None,
            }],
        );

        let error = compile_canonical_runtime(config, None).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("invalid mapper regex"));
        assert!(message.contains("before_tool"));
    }

    #[test]
    fn compile_canonical_runtime_bridges_messenger_config() {
        use crate::config::claudine_config::{ClaudineMessengerConfig, MessengerProviderConfig};

        let mut config = ClaudineConfig::default();
        config.protect.enabled = false;
        config.default_sounds = DefaultSounds::default();
        config.messenger = Some(ClaudineMessengerConfig {
            active_config: Some("alerts".to_string()),
            configurations: HashMap::from([(
                "alerts".to_string(),
                MessengerProviderConfig::Discord {
                    channel_id: "999".to_string(),
                    bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
                },
            )]),
        });

        let runtime = compile_canonical_runtime(config, None).unwrap();
        let messaging = runtime.messaging();
        assert!(messaging.user.is_some());
        assert_eq!(
            messaging.user.as_ref().unwrap().active.as_deref(),
            Some("alerts")
        );
        assert!(messaging.user.as_ref().unwrap().configs.contains_key("alerts"));
    }

    #[test]
    fn compile_canonical_runtime_no_messenger_gives_empty_messaging() {
        let mut config = ClaudineConfig::default();
        config.protect.enabled = false;
        config.default_sounds = DefaultSounds::default();
        config.messenger = None;

        let runtime = compile_canonical_runtime(config, None).unwrap();
        let messaging = runtime.messaging();
        assert!(messaging.user.is_none());
        assert!(messaging.repo.is_none());
    }

    #[test]
    fn bridge_provider_config_discord() {
        let cfg = MessengerProviderConfig::Discord {
            channel_id: "123".to_string(),
            bot_token_env: "MY_TOKEN".to_string(),
        };
        let route = bridge_provider_config(&cfg);
        match route {
            MessagingRouteConfig::Discord {
                channel_id,
                bot_token,
                bot_token_env,
            } => {
                assert_eq!(channel_id, "123");
                assert_eq!(bot_token, None);
                assert_eq!(bot_token_env, "MY_TOKEN");
            }
            other => panic!("expected Discord, got {other:?}"),
        }
    }

    #[test]
    fn bridge_provider_config_slack() {
        let cfg = MessengerProviderConfig::Slack {
            channel_id: "C456".to_string(),
            bot_token_env: "SLACK_TOKEN".to_string(),
        };
        let route = bridge_provider_config(&cfg);
        match route {
            MessagingRouteConfig::Slack {
                channel_id,
                bot_token,
                bot_token_env,
            } => {
                assert_eq!(channel_id, "C456");
                assert_eq!(bot_token, None);
                assert_eq!(bot_token_env, "SLACK_TOKEN");
            }
            other => panic!("expected Slack, got {other:?}"),
        }
    }

    #[test]
    fn bridge_provider_config_signal() {
        let cfg = MessengerProviderConfig::Signal {
            recipient: "+15551234567".to_string(),
            rpc_url_env: "SIG_RPC".to_string(),
            account_env: "SIG_ACCT".to_string(),
        };
        let route = bridge_provider_config(&cfg);
        match route {
            MessagingRouteConfig::Signal {
                recipient,
                rpc_url,
                rpc_url_env,
                account,
                account_env,
            } => {
                assert_eq!(recipient, "+15551234567");
                assert_eq!(rpc_url, None);
                assert_eq!(rpc_url_env, "SIG_RPC");
                assert_eq!(account, None);
                assert_eq!(account_env, "SIG_ACCT");
            }
            other => panic!("expected Signal, got {other:?}"),
        }
    }

    #[test]
    fn bridge_provider_config_whatsapp() {
        let cfg = MessengerProviderConfig::Whatsapp {
            recipient: "+15559876543".to_string(),
            access_token_env: "WA_TOKEN".to_string(),
            phone_number_id_env: "WA_PHONE".to_string(),
        };
        let route = bridge_provider_config(&cfg);
        match route {
            MessagingRouteConfig::WhatsApp {
                recipient,
                access_token,
                access_token_env,
                phone_number_id,
                phone_number_id_env,
            } => {
                assert_eq!(recipient, "+15559876543");
                assert_eq!(access_token, None);
                assert_eq!(access_token_env, "WA_TOKEN");
                assert_eq!(phone_number_id, None);
                assert_eq!(phone_number_id_env, "WA_PHONE");
            }
            other => panic!("expected WhatsApp, got {other:?}"),
        }
    }

    #[test]
    fn canonical_runtime_exposes_config() {
        let config = ClaudineConfig::default();
        let runtime = compile_canonical_runtime(config.clone(), None).unwrap();
        assert_eq!(runtime.config().preferred_agent, config.preferred_agent);
    }

    // =====================================================================
    // Repo-scoped old-format config migration
    // =====================================================================

    /// When the repo-level `.claudine/config.json` contains old-format content,
    /// `load_claudine_config` should succeed (using user config only), and the
    /// repo config should be renamed to `.bak`.
    #[test]
    fn repo_old_format_config_backed_up_and_ignored() {
        let dir = tempfile::tempdir().unwrap();

        // Write a valid user config
        let user_path = dir.path().join("user-config.json");
        let user_config = ClaudineConfig {
            preferred_agent: Provider::Claude,
            ..ClaudineConfig::default()
        };
        save_claudine_config(&user_config, &user_path).unwrap();

        // Write an old-format repo config
        let repo_dir = dir.path().join("repo");
        let repo_config_dir = repo_dir.join(".claudine");
        std::fs::create_dir_all(&repo_config_dir).unwrap();
        let repo_config_path = repo_config_dir.join("config.json");
        let old_format = serde_json::json!({
            "claude": {},
            "gemini": {}
        });
        std::fs::write(
            &repo_config_path,
            serde_json::to_string(&old_format).unwrap(),
        )
        .unwrap();

        // load_claudine_config should succeed (not crash with parse error)
        let loaded = load_claudine_config(Some(&user_path), Some(&repo_dir)).unwrap();
        assert_eq!(
            loaded.preferred_agent,
            Provider::Claude,
            "user config should be returned when repo config is old format"
        );

        // The repo config should have been renamed to .bak
        assert!(
            !repo_config_path.exists(),
            "old-format repo config should have been renamed"
        );
        assert!(
            repo_config_dir.join("config.json.bak").exists(),
            "backup of old-format repo config should exist"
        );
    }

    /// `load_repo_override_config` returns `Ok(None)` when the file is old-format
    /// and backs up the file.
    #[test]
    fn load_repo_override_config_old_format_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let old_format = serde_json::json!({
            "version": "1.0",
            "settings": {},
            "providers": {
                "claude": {
                    "events": {}
                }
            }
        });
        std::fs::write(&path, serde_json::to_string(&old_format).unwrap()).unwrap();

        let result = load_repo_override_config(&path).unwrap();
        assert!(
            result.is_none(),
            "old-format config should return Ok(None)"
        );

        // The file should have been backed up
        assert!(
            !path.exists(),
            "old-format config should have been renamed"
        );
        assert!(
            dir.path().join("config.json.bak").exists(),
            "backup should exist"
        );
    }

    // =====================================================================
    // Phase 5.2: preferred_agent honored by load_claudine_config
    // =====================================================================

    /// Writes a ClaudineConfig with a non-default preferred_agent and
    /// verifies that `load_claudine_config` returns it faithfully.
    #[test]
    fn load_claudine_config_honors_preferred_agent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = ClaudineConfig {
            preferred_agent: Provider::Codex,
            ..ClaudineConfig::default()
        };
        save_claudine_config(&config, &path).unwrap();

        let loaded = load_claudine_config(Some(&path), None).unwrap();
        assert_eq!(
            loaded.preferred_agent,
            Provider::Codex,
            "preferred_agent should be Codex as written"
        );
    }

    /// Verifies that each provider variant round-trips through
    /// `save_claudine_config` + `load_claudine_config` as preferred_agent.
    #[test]
    fn load_claudine_config_preferred_agent_all_providers() {
        let providers = [
            Provider::Claude,
            Provider::Codex,
            Provider::Gemini,
            Provider::Goose,
            Provider::KimiCode,
            Provider::OpenCode,
            Provider::QwenCode,
            Provider::RooCode,
        ];

        let dir = tempfile::tempdir().unwrap();
        for provider in providers {
            let path = dir.path().join(format!("{provider:?}.json"));
            let config = ClaudineConfig {
                preferred_agent: provider,
                ..ClaudineConfig::default()
            };
            save_claudine_config(&config, &path).unwrap();
            let loaded = load_claudine_config(Some(&path), None).unwrap();
            assert_eq!(
                loaded.preferred_agent, provider,
                "preferred_agent round-trip failed for {provider:?}"
            );
        }
    }

    /// Verifies that repo config merge does NOT override preferred_agent
    /// Repo config only has canonical_provider and actions; preferred_agent
    /// is user-only and cannot appear in a `RepoOverrideConfig`.
    #[test]
    fn load_claudine_config_preferred_agent_not_overridden_by_repo() {
        let dir = tempfile::tempdir().unwrap();

        // User config with preferred_agent = Codex
        let user_path = dir.path().join("user.json");
        let user_config = ClaudineConfig {
            preferred_agent: Provider::Codex,
            ..ClaudineConfig::default()
        };
        save_claudine_config(&user_config, &user_path).unwrap();

        // Repo config only sets canonical_provider (no preferred_agent field)
        let repo_dir = dir.path().join("repo");
        std::fs::create_dir_all(repo_dir.join(".claudine")).unwrap();
        let repo_override = RepoOverrideConfig {
            canonical_provider: Some(Provider::Gemini),
            ..RepoOverrideConfig::default()
        };
        save_repo_override_config(&repo_override, &repo_dir.join(".claudine/config.json"))
            .unwrap();

        let loaded = load_claudine_config(Some(&user_path), Some(&repo_dir)).unwrap();
        assert_eq!(
            loaded.preferred_agent,
            Provider::Codex,
            "repo should not override user's preferred_agent"
        );
        assert_eq!(
            loaded.canonical_provider,
            Some(Provider::Gemini),
            "repo should override canonical_provider"
        );
    }

    // =====================================================================
    // Phase 5.3: Repo config migration (old format detection)
    // =====================================================================

    /// Writes an old-format config to the user config path, loads via
    /// `load_claudine_config`, and verifies the backup was created
    /// and a `ConfigNotFound` error was returned.
    #[test]
    fn load_claudine_config_old_format_creates_backup_and_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".claudine");
        std::fs::create_dir_all(&config_dir).unwrap();

        // Write old per-provider format
        let old_config = serde_json::json!({
            "version": "1.0",
            "settings": {
                "tts": { "provider": "say" }
            },
            "providers": {
                "claude": {
                    "events": {
                        "session_start": {
                            "enabled": true,
                            "actions": [
                                { "type": "speak", "message": "hello" }
                            ]
                        }
                    }
                }
            }
        });

        let config_path = config_dir.join("config.json");
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&old_config).unwrap(),
        )
        .unwrap();

        // Load should fail with ConfigNotFound
        let result = load_claudine_config(Some(&config_path), None);
        assert!(result.is_err(), "old format should produce an error");
        assert!(
            matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
            "error should be ConfigNotFound"
        );

        // Original file should be gone, .bak should exist
        assert!(
            !config_path.exists(),
            "original config should have been renamed"
        );
        assert!(
            config_dir.join("config.json.bak").exists(),
            "backup file should exist"
        );
    }

    /// After old format backup, a second load attempt also returns
    /// `ConfigNotFound` since the original file no longer exists.
    #[test]
    fn load_claudine_config_after_old_format_backup_returns_config_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".claudine");
        std::fs::create_dir_all(&config_dir).unwrap();

        let old_config = serde_json::json!({
            "version": "1.0",
            "settings": {},
            "providers": {}
        });
        let config_path = config_dir.join("config.json");
        std::fs::write(
            &config_path,
            serde_json::to_string(&old_config).unwrap(),
        )
        .unwrap();

        // First load triggers backup
        let _ = load_claudine_config(Some(&config_path), None);

        // Second load: file is gone
        let result = load_claudine_config(Some(&config_path), None);
        assert!(
            matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
            "second load should also return ConfigNotFound"
        );
    }

    /// Old format with root-level provider keys (no version/providers) is
    /// also detected and backed up.
    #[test]
    fn load_claudine_config_detects_old_provider_keys_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let old_config = serde_json::json!({
            "claude": { "events": {} },
            "gemini": { "events": {} }
        });
        std::fs::write(&path, serde_json::to_string(&old_config).unwrap()).unwrap();

        let result = load_claudine_config(Some(&path), None);
        assert!(
            matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
            "root-level provider keys should be detected as old format"
        );
        assert!(
            dir.path().join("config.json.bak").exists(),
            "backup should be created for root-level provider key format"
        );
    }

    /// A valid new-format config is NOT treated as old format and loads fine.
    #[test]
    fn load_claudine_config_does_not_backup_new_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let new_config = ClaudineConfig {
            preferred_agent: Provider::Claude,
            ..ClaudineConfig::default()
        };
        save_claudine_config(&new_config, &path).unwrap();

        let loaded = load_claudine_config(Some(&path), None).unwrap();
        assert_eq!(loaded.preferred_agent, Provider::Claude);
        assert!(
            !dir.path().join("config.json.bak").exists(),
            "new format should not produce a backup"
        );
    }
}
