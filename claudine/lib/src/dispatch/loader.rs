use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::Json5;
use regex::Regex;
use tracing::{debug, info, warn};

use crate::actions::{CompiledMapper, HookAction, Mapper};
use crate::config::atomic::atomic_write;
use crate::config::claudine_config::{
    ClaudineConfig, ClaudineMessengerConfig, MessengerProviderConfig, RepoOverrideConfig,
};
use crate::config::migration;
use crate::error::{ClaudineError, Result};
use crate::events::{AgenticEvent, GlobalSettings};
use crate::messaging::{MessagingRouteConfig, RuntimeMessagingSettings, ScopedMessagingSettings};
use crate::services::protect::catalog::ProtectPlatform;
use crate::services::protect::service::ProtectService;

/// Candidate file names for user-level configuration.
const USER_CONFIG_NAMES: &[&str] = &[".claudine/config.json"];

/// Repo-level config file name.
const REPO_CONFIG_NAME: &str = ".claudine/config.json";

/// Event binding ready for dispatch without per-event regex compilation.
#[derive(Debug, Clone)]
pub struct RuntimeEventBinding {
    enabled: bool,
    actions: Vec<HookAction>,
    matcher: Option<Regex>,
    compiled_mappers: Vec<Option<CompiledMapper>>,
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
    use crate::config::claudine_config::{Gender, TtsValue, VoiceSelection};
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
    if let Some(override_value) = &repo.active_messenger
        && let Some(messenger) = &mut user.messenger
    {
        messenger.active_config = override_value.clone();
    }
}

/// Parse a raw string as JSON5 and return a [`serde_json::Value`].
fn parse_json5_to_value(raw: &str) -> Result<serde_json::Value> {
    let json5 = Json5::from_str(raw)
        .map_err(|e| ClaudineError::ConfigValidation(format!("JSON5 parse error: {e}")))?;
    Ok(json5.as_json_value().clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::*;
    use crate::config::claudine_config::DefaultSounds;
    use crate::events::*;

    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn user_config_path_returns_default_when_none_exists() {
        let path = user_config_path();
        assert!(path.to_string_lossy().contains(".claudine"));
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

        let config = load_claudine_config(Some(&config_dir.join("config.json")), None).unwrap();
        assert!(config.logging);
        assert!(
            config
                .actions
                .contains_key(&crate::events::AgenticEvent::HumanInTheLoop)
        );
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

        let session_start = &user.actions[&AgenticEvent::SessionStart];
        assert_eq!(session_start.len(), 1);
        if let HookAction::SoundEffect { effect, .. } = &session_start[0] {
            assert_eq!(effect, "repo-sound");
        } else {
            panic!("Expected SoundEffect");
        }

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
        assert!(
            messaging
                .user
                .as_ref()
                .unwrap()
                .configs
                .contains_key("alerts")
        );
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

    #[test]
    fn repo_old_format_config_backed_up_and_ignored() {
        let dir = tempfile::tempdir().unwrap();

        let user_path = dir.path().join("user-config.json");
        let user_config = ClaudineConfig {
            preferred_agent: Provider::Claude,
            ..ClaudineConfig::default()
        };
        save_claudine_config(&user_config, &user_path).unwrap();

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

        let loaded = load_claudine_config(Some(&user_path), Some(&repo_dir)).unwrap();
        assert_eq!(
            loaded.preferred_agent,
            Provider::Claude,
            "user config should be returned when repo config is old format"
        );

        assert!(
            !repo_config_path.exists(),
            "old-format repo config should have been renamed"
        );
        assert!(
            repo_config_dir.join("config.json.bak").exists(),
            "backup of old-format repo config should exist"
        );
    }

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
        assert!(result.is_none(), "old-format config should return Ok(None)");

        assert!(!path.exists(), "old-format config should have been renamed");
        assert!(
            dir.path().join("config.json.bak").exists(),
            "backup should exist"
        );
    }

    // =====================================================================
    // preferred_agent tests
    // =====================================================================

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

    #[test]
    fn load_claudine_config_preferred_agent_not_overridden_by_repo() {
        let dir = tempfile::tempdir().unwrap();

        let user_path = dir.path().join("user.json");
        let user_config = ClaudineConfig {
            preferred_agent: Provider::Codex,
            ..ClaudineConfig::default()
        };
        save_claudine_config(&user_config, &user_path).unwrap();

        let repo_dir = dir.path().join("repo");
        std::fs::create_dir_all(repo_dir.join(".claudine")).unwrap();
        let repo_override = RepoOverrideConfig {
            canonical_provider: Some(Provider::Gemini),
            ..RepoOverrideConfig::default()
        };
        save_repo_override_config(&repo_override, &repo_dir.join(".claudine/config.json")).unwrap();

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
    // Repo config migration (old format detection)
    // =====================================================================

    #[test]
    fn load_claudine_config_old_format_creates_backup_and_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".claudine");
        std::fs::create_dir_all(&config_dir).unwrap();

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

        let result = load_claudine_config(Some(&config_path), None);
        assert!(result.is_err(), "old format should produce an error");
        assert!(
            matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
            "error should be ConfigNotFound"
        );

        assert!(
            !config_path.exists(),
            "original config should have been renamed"
        );
        assert!(
            config_dir.join("config.json.bak").exists(),
            "backup file should exist"
        );
    }

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
        std::fs::write(&config_path, serde_json::to_string(&old_config).unwrap()).unwrap();

        let _ = load_claudine_config(Some(&config_path), None);

        let result = load_claudine_config(Some(&config_path), None);
        assert!(
            matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)),
            "second load should also return ConfigNotFound"
        );
    }

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
