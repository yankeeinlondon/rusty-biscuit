use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use tracing::{debug, info, warn};

use crate::actions::{CompiledMapper, HookAction, Mapper};
use crate::config::atomic::atomic_write;
use crate::error::{ClaudineError, Result};
use crate::events::{
    AgenticEvent, CanonicalProviderSettings, GlobalSettings, HookerConfig, LinkingSettings, Provider,
};
use crate::services::{ProtectConfig, ProtectPosture};

/// Candidate file names for user-level configuration.
const USER_CONFIG_NAMES: &[&str] = &[".claudine/config.json"];

/// Repo-level config file name.
const REPO_CONFIG_NAME: &str = ".claudine/config.json";

/// Runtime configuration with precompiled matcher and mapper regexes.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    settings: GlobalSettings,
    providers: HashMap<Provider, RuntimeProviderConfig>,
}

#[derive(Debug, Clone, Default)]
struct RuntimeProviderConfig {
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
    let config = load_config(user, repo_root)?;
    compile_runtime_config(config)
}

fn compile_runtime_config(config: HookerConfig) -> Result<RuntimeConfig> {
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

    Ok(RuntimeConfig {
        settings,
        providers: runtime_providers,
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
        (Some(user_cfg), None) => Some(user_cfg.clone()),
        (None, Some(repo_cfg)) => Some(repo_cfg.clone()),
        (Some(user_cfg), Some(repo_cfg)) => {
            let mut merged = user_cfg.merge_with(repo_cfg);
            let allow_downgrade =
                user_cfg.allow_repo_posture_downgrade || repo_cfg.allow_repo_posture_downgrade;

            if !allow_downgrade {
                if user_cfg.posture == ProtectPosture::Strict
                    && merged.posture != ProtectPosture::Strict
                {
                    merged.posture = ProtectPosture::Strict;
                }

                if user_cfg.enabled && !merged.enabled {
                    merged.enabled = true;
                }
            }

            Some(merged)
        }
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
    use crate::events::*;
    use crate::services::{ProtectConfig, ProtectPosture};
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
            HookAction::Speak { message } => assert_eq!(message, "repo session"),
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
    fn merge_protect_does_not_silently_weaken_strict_user_posture() {
        let user = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                protect: Some(ProtectConfig {
                    enabled: true,
                    posture: ProtectPosture::Strict,
                    ..ProtectConfig::default()
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let repo = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                protect: Some(ProtectConfig {
                    enabled: false,
                    posture: ProtectPosture::Advisory,
                    ..ProtectConfig::default()
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let merged = merge_configs(user, repo);
        let protect = merged.settings.protect.expect("missing merged protect");
        assert!(protect.enabled);
        assert_eq!(protect.posture, ProtectPosture::Strict);
    }

    #[test]
    fn merge_protect_allows_explicit_repo_downgrade() {
        let user = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                protect: Some(ProtectConfig {
                    enabled: true,
                    posture: ProtectPosture::Strict,
                    ..ProtectConfig::default()
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let repo = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                protect: Some(ProtectConfig {
                    enabled: false,
                    posture: ProtectPosture::Advisory,
                    allow_repo_posture_downgrade: true,
                    ..ProtectConfig::default()
                }),
                ..GlobalSettings::default()
            },
            providers: HashMap::new(),
        };

        let merged = merge_configs(user, repo);
        let protect = merged.settings.protect.expect("missing merged protect");
        assert!(!protect.enabled);
        assert_eq!(protect.posture, ProtectPosture::Advisory);
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
        assert_eq!(linking.canonical_provider.user_skill, Some(Provider::Claude));
        assert_eq!(
            linking.canonical_provider.user_command,
            Some(Provider::Claude)
        );
        assert_eq!(
            linking.canonical_provider.user_agent,
            Some(Provider::Claude)
        );

        // Repo-scoped canonical providers come from repo config
        assert_eq!(linking.canonical_provider.repo_skill, Some(Provider::Claude));
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
        assert_eq!(linking.canonical_provider.user_skill, Some(Provider::Claude));
        // repo_skill overridden by repo config
        assert_eq!(linking.canonical_provider.repo_skill, Some(Provider::Claude));
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
}
