use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use tracing::{debug, warn};

use super::deps::*;
use crate::config::json_conversion::{self, ConfigNotice, ScopeState};
use crate::config::load::read_document;
use crate::config::schema::{CONFIG_DIR, CONFIG_FILE_NAME, ConfigScope};

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
    /// The process snapshot hook `when` conditions resolve file references
    /// and `ctx.*` against.
    pub(crate) request: darkmatter::markdown::compose::RequestSnapshot,
    pub(crate) messaging: RuntimeMessagingSettings,
    pub(crate) protect_service: Option<ProtectService>,
    pub(crate) events: HashMap<AgenticEvent, RuntimeEventBinding>,
}

impl CanonicalRuntimeConfig {
    pub fn config(&self) -> &ClaudineConfig {
        &self.config
    }

    /// The process snapshot this runtime was compiled for.
    pub fn request_snapshot(&self) -> &darkmatter::markdown::compose::RequestSnapshot {
        &self.request
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
    request: &darkmatter::markdown::compose::RequestSnapshot,
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
        request: request.clone(),
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
            let compiled =
                Regex::new(pattern).map_err(|error| ClaudineError::TemplateErrorWithCause {
                    message: format!("invalid mapper regex for event={event}: {error} ({pattern})"),
                    source: error,
                })?;
            Ok(CompiledMapper::Regex { pattern: compiled })
        }
    }
}

/// Bridge [`ClaudineConfig`] TTS settings to legacy [`GlobalSettings`].
///
/// Constructs a minimal [`GlobalSettings`] containing only the TTS
/// configuration and the pre-warm switch, suitable for
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
                provider: cfg.provider.clone(),
                voice,
                rate: None,
            })
        }
    };
    GlobalSettings {
        tts,
        tts_prewarm: config.tts.prewarm_enabled(),
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

/// The user configuration file, `~/.claudine/config.yaml`, whether or not
/// it exists.
pub fn user_config_path() -> PathBuf {
    let home = biscuit_file::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    ConfigScope::file_under(&home)
}

/// The repository configuration file, `{repo_root}/.claudine/config.yaml`.
pub fn repo_config_path(repo_root: &Path) -> PathBuf {
    ConfigScope::file_under(repo_root)
}

/// Whether a user configuration exists: `config.yaml`, or a legacy
/// `config.json` the next load converts.
pub fn user_config_exists() -> bool {
    let path = user_config_path();
    path.is_file() || legacy_path(&path).is_some_and(|legacy| legacy.is_file())
}

/// Whether `path` is a canonical configuration location
/// (`<base>/.claudine/config.yaml`), the only kind whose legacy
/// `config.json` sibling is converted. Any other path is read as YAML,
/// exactly where it is.
fn is_canonical(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == CONFIG_FILE_NAME)
        && path.parent().and_then(Path::file_name).is_some_and(|dir| dir == CONFIG_DIR)
}

fn legacy_path(path: &Path) -> Option<PathBuf> {
    is_canonical(path).then(|| path.with_file_name(json_conversion::LEGACY_FILE_NAME))
}

// ==========================================================================
// ClaudineConfig loading and merging
// ==========================================================================

/// A loaded configuration and the conversion notices the load produced,
/// which the caller reports once.
#[derive(Debug, Clone)]
pub struct LoadedClaudineConfig {
    /// The merged, validated configuration.
    pub config: ClaudineConfig,
    /// What converting a legacy `config.json` did or found (code
    /// `dm.config.migration`).
    pub notices: Vec<ConfigNotice>,
}

/// Load and validate a [`ClaudineConfig`], logging conversion notices.
///
/// See [`load_claudine_config_with_notices`].
///
/// ## Errors
///
/// As [`load_claudine_config_with_notices`].
pub fn load_claudine_config(user_path: Option<&Path>, repo_root: Option<&Path>) -> Result<ClaudineConfig> {
    let loaded = load_claudine_config_with_notices(user_path, repo_root)?;
    for notice in &loaded.notices {
        warn!(code = notice.code, "{}", notice.message);
    }
    Ok(loaded.config)
}

/// Load and validate a [`ClaudineConfig`].
///
/// If `user_path` is `Some`, it is read exactly there as YAML; otherwise the
/// user file is [`user_config_path()`]. If `repo_root` is provided,
/// `{repo_root}/.claudine/config.yaml` is merged on top of it. Each file is
/// judged by its scope's schema before it is read, and the merged result by
/// [`ClaudineConfig::validate`].
///
/// At a canonical location, a legacy `config.json` beside the YAML file is
/// converted first (see [`crate::config::json_conversion`]); a pre-2026
/// per-provider `config.json` is backed up to `<path>.bak` and ignored, so
/// the caller can run the setup wizard.
///
/// ## Errors
///
/// - [`ClaudineError::ConfigNotFound`] when the user file does not exist or
///   was a pre-2026 file that was backed up;
/// - [`ClaudineError::ConfigInvalid`] when a file is not valid, located by
///   file, line, column, and JSON pointer;
/// - [`ClaudineError::ConfigConversion`] when a legacy file cannot be
///   converted (it is kept);
/// - the semantic errors of [`ClaudineConfig::validate`], and I/O errors.
pub fn load_claudine_config_with_notices(
    user_path: Option<&Path>,
    repo_root: Option<&Path>,
) -> Result<LoadedClaudineConfig> {
    let path = user_path.map(PathBuf::from).unwrap_or_else(user_config_path);
    let mut notices = Vec::new();

    if is_canonical(&path)
        && json_conversion::prepare_scope(&path, ConfigScope::User, &mut notices)? == ScopeState::LegacyFormatIgnored
    {
        return Err(ClaudineError::ConfigNotFound(path));
    }
    if !path.is_file() {
        return Err(ClaudineError::ConfigNotFound(path));
    }
    let document = read_document(&path, ConfigScope::User)?;
    let mut config = ClaudineConfig::from_effective(document.effective, Some(&path))?;
    debug!(?path, "Loaded ClaudineConfig (user)");

    // Skip the repo file when it resolves to the user file (HOME at the repo
    // root), so one file is never read as both scopes.
    if let Some(root) = repo_root {
        let repo_path = repo_config_path(root);
        let same_file = repo_path
            .canonicalize()
            .ok()
            .zip(path.canonicalize().ok())
            .is_some_and(|(r, u)| r == u);
        if !same_file && let Some(repo_override) = read_repo_override(&repo_path, &mut notices)? {
            debug!(?repo_path, "Loaded RepoOverrideConfig");
            merge_repo_override(&mut config, &repo_override);
        }
    }

    config.validate()?;
    Ok(LoadedClaudineConfig { config, notices })
}

/// Load a repo-scoped [`RepoOverrideConfig`] from the given path, read as
/// YAML. At a canonical location a legacy `config.json` beside it is
/// converted first.
///
/// Returns `Ok(None)` if the file does not exist.
///
/// ## Errors
///
/// [`ClaudineError::ConfigInvalid`] for an invalid file,
/// [`ClaudineError::ConfigConversion`] for a legacy file that cannot be
/// converted, and I/O errors.
pub fn load_repo_override_config(path: &Path) -> Result<Option<RepoOverrideConfig>> {
    let mut notices = Vec::new();
    let config = read_repo_override(path, &mut notices)?;
    for notice in &notices {
        warn!(code = notice.code, "{}", notice.message);
    }
    Ok(config)
}

fn read_repo_override(path: &Path, notices: &mut Vec<ConfigNotice>) -> Result<Option<RepoOverrideConfig>> {
    if is_canonical(path)
        && json_conversion::prepare_scope(path, ConfigScope::Repository, notices)? == ScopeState::LegacyFormatIgnored
    {
        warn!(?path, "Repo config was old format; backed up and ignored");
        return Ok(None);
    }
    if !path.is_file() {
        return Ok(None);
    }
    let document = read_document(path, ConfigScope::Repository)?;
    RepoOverrideConfig::from_effective(document.effective, Some(path)).map(Some)
}

#[cfg(test)]
mod tests;
