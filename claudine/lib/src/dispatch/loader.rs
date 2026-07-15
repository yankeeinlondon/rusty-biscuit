use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::Json5;
use regex::Regex;
use tracing::{debug, info, warn};

use super::deps::*;

/// Candidate file names for user-level configuration.
const USER_CONFIG_NAMES: &[&str] = &[".claudine/config.json"];

/// Repo-level config file name.
const REPO_CONFIG_NAME: &str = ".claudine/config.json";

/// Event binding ready for dispatch without per-event regex compilation.
#[derive(Debug, Clone)]
pub struct RuntimeEventBinding {
    enabled: bool,
    actions: Vec<HookAction>,
    matcher: Option<RuntimeMatcher>,
    compiled_mappers: Vec<Option<CompiledMapper>>,
}

impl RuntimeEventBinding {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn actions(&self) -> &[HookAction] {
        &self.actions
    }

    pub fn matcher(&self) -> Option<&RuntimeMatcher> {
        self.matcher.as_ref()
    }

    /// Per-action compiled mapper metadata aligned with [`Self::actions`].
    pub fn compiled_mappers(&self) -> &[Option<CompiledMapper>] {
        &self.compiled_mappers
    }

    #[cfg(test)]
    pub fn new_for_test(
        enabled: bool,
        actions: Vec<HookAction>,
        matcher: Option<RuntimeMatcher>,
    ) -> Self {
        let compiled_mappers = vec![None; actions.len()];
        Self {
            enabled,
            actions,
            matcher,
            compiled_mappers,
        }
    }
}

// ==========================================================================
// CanonicalRuntimeConfig (compiled from ClaudineConfig)
// ==========================================================================

/// Runtime configuration compiled from [`ClaudineConfig`].
///
/// Indexes event bindings by canonical event rather than by provider+event.
#[derive(Debug, Clone)]
pub struct CanonicalRuntimeConfig {
    pub(crate) config: ClaudineConfig,
    pub(crate) messaging: RuntimeMessagingSettings,
    pub(crate) protect_service: Option<ProtectService>,
    pub(crate) events: HashMap<AgenticEvent, RuntimeEventBinding>,
}

impl CanonicalRuntimeConfig {
    pub fn config(&self) -> &ClaudineConfig {
        &self.config
    }

    pub fn messaging(&self) -> &RuntimeMessagingSettings {
        &self.messaging
    }

    pub fn protect_service(&self) -> Option<&ProtectService> {
        self.protect_service.as_ref()
    }

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
    let compiled_matchers: HashMap<AgenticEvent, Option<RuntimeMatcher>> = compile_many(
        &config
            .matchers
            .iter()
            .map(|(event, raw)| (*event, raw.as_str()))
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .collect();

    let mut events = HashMap::new();
    for (event, actions) in &config.actions {
        let compiled_mappers = actions
            .iter()
            .map(|action| compile_canonical_action_mapper(action, *event))
            .collect::<Result<Vec<_>>>()?;

        let matcher = compiled_matchers.get(event).and_then(|m| m.clone());

        events.insert(
            *event,
            RuntimeEventBinding {
                enabled: true,
                actions: actions.clone(),
                matcher,
                compiled_mappers,
            },
        );
    }

    // Bindings that have a matcher configured but no actions still need a
    // runtime binding so that future protect-only or logging-only flows
    // can honor the matcher. Today this is harmless: dispatch will skip
    // the actions block but still run protect/log paths.
    for event in config.matchers.keys() {
        if events.contains_key(event) {
            continue;
        }
        let matcher = compiled_matchers.get(event).and_then(|m| m.clone());
        events.insert(
            *event,
            RuntimeEventBinding {
                enabled: true,
                actions: Vec::new(),
                matcher,
                compiled_mappers: Vec::new(),
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
/// configuration, suitable for
/// [`LifecycleRuntimeContext`](crate::composition::lifecycle::LifecycleRuntimeContext).
pub fn bridge_tts_settings(config: &ClaudineConfig) -> GlobalSettings {
    use crate::config::tts::{Gender, TtsValue, VoiceSelection};
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
/// Reuses the private `bridge_messenger_to_runtime` helper from
/// [`compile_canonical_runtime`].
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
        MessengerProviderConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => MessagingRouteConfig::DiscordWebhook {
            webhook_url: webhook_url.clone(),
            webhook_url_env: webhook_url_env.clone(),
        },
        MessengerProviderConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => MessagingRouteConfig::SlackWebhook {
            webhook_url: webhook_url.clone(),
            webhook_url_env: webhook_url_env.clone(),
        },
    }
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
                warn!(
                    ?repo_path,
                    "Repo config was old format; backed up and ignored"
                );
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
        warn!(
            ?path,
            "Repo override config was old format; backed up and ignored"
        );
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

/// Parse a raw string as JSON5 and return a [`serde_json::Value`].
fn parse_json5_to_value(raw: &str) -> Result<serde_json::Value> {
    let json5 = Json5::from_str(raw)
        .map_err(|e| ClaudineError::ConfigValidation(format!("JSON5 parse error: {e}")))?;
    Ok(json5.as_json_value().clone())
}

#[cfg(test)]
mod tests;
