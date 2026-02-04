use std::path::{Path, PathBuf};

use tracing::{debug, warn};

use crate::error::{ClaudineError, Result};
use crate::events::HookerConfig;

/// Candidate file names for user-level configuration.
const USER_CONFIG_NAMES: &[&str] = &[".hooker", ".hook-config"];

/// Repo-level config file name.
const REPO_CONFIG_NAME: &str = ".hooker";

/// Load and merge Claudine configuration.
///
/// Resolves `~/.hooker` or `~/.hook-config` (user) then merges with
/// `.hooker` (repo). Repo-level event bindings replace user-level per-event.
/// Settings merge field-by-field.
///
/// ## Errors
///
/// Returns `ConfigNotFound` when no configuration file is found at any
/// expected location.
pub fn load_config(
    user: Option<&Path>,
    repo_root: Option<&Path>,
) -> Result<HookerConfig> {
    let user_config = load_user_config(user)?;
    let repo_config = load_repo_config(repo_root);

    match (user_config, repo_config) {
        (Some(user_cfg), Some(repo_cfg)) => {
            debug!("Merging user and repo configurations");
            Ok(merge_configs(user_cfg, repo_cfg))
        }
        (Some(cfg), None) => Ok(cfg),
        (None, Some(cfg)) => Ok(cfg),
        (None, None) => {
            let path = user
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("~"))
                        .join(USER_CONFIG_NAMES[0])
                });
            Err(ClaudineError::ConfigNotFound(path))
        }
    }
}

/// Attempt to load the user-level config.
///
/// If an explicit path is given, read that file.
/// Otherwise, search the home directory for known config filenames.
fn load_user_config(explicit: Option<&Path>) -> Result<Option<HookerConfig>> {
    if let Some(path) = explicit {
        if path.exists() {
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
        if path.exists() {
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
/// Looks for `.hooker` in the given repo root. Returns `None` if
/// no repo root is provided or the file doesn't exist.
fn load_repo_config(repo_root: Option<&Path>) -> Option<HookerConfig> {
    let root = repo_root?;
    let path = root.join(REPO_CONFIG_NAME);

    if !path.exists() {
        return None;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!(?path, %e, "Failed to read repo config");
            return None;
        }
    };

    match serde_json::from_str(&content) {
        Ok(config) => {
            debug!(?path, "Loaded repo config");
            Some(config)
        }
        Err(e) => {
            warn!(?path, %e, "Failed to parse repo config");
            None
        }
    }
}

/// Merge two configs: repo events replace user events per-event,
/// settings merge field-by-field (repo overrides user).
fn merge_configs(user: HookerConfig, repo: HookerConfig) -> HookerConfig {
    let mut events = user.events;

    // Repo events completely replace matching user events
    for (event_key, repo_binding) in repo.events {
        events.insert(event_key, repo_binding);
    }

    // Settings: repo fields override user fields individually
    let settings = crate::events::GlobalSettings {
        default_log_target: repo
            .settings
            .default_log_target
            .or(user.settings.default_log_target),
        tts: repo.settings.tts.or(user.settings.tts),
    };

    HookerConfig {
        version: repo.version,
        settings,
        events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn write_cfg(dir: &Path, name: &str, config: &HookerConfig) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, serde_json::to_string(config).unwrap()).unwrap();
        path
    }

    fn empty_cfg() -> HookerConfig {
        HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            events: HashMap::new(),
        }
    }

    fn speak_binding(msg: &str) -> EventBinding {
        EventBinding {
            enabled: true,
            actions: vec![EventAction::Speak { message: msg.to_string() }],
            matcher: None,
            overrides: HashMap::new(),
        }
    }

    #[test]
    fn load_config_from_explicit_user_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = empty_cfg();
        config.events.insert(AgenticEvent::SessionStart, speak_binding("hello"));
        let path = write_cfg(tmp.path(), ".hooker", &config);

        let loaded = load_config(Some(&path), None).unwrap();
        assert_eq!(loaded.version, "1.0");
        assert!(loaded.events.contains_key(&AgenticEvent::SessionStart));
    }

    #[test]
    fn missing_config_returns_config_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let result = load_config(Some(&tmp.path().join("nonexistent")), None);
        assert!(matches!(result.unwrap_err(), ClaudineError::ConfigNotFound(_)));
    }

    #[test]
    fn merge_repo_events_replace_user_events() {
        let tmp = tempfile::tempdir().unwrap();
        let mut user_config = empty_cfg();
        user_config.events.insert(AgenticEvent::SessionStart, speak_binding("user hello"));
        user_config.events.insert(AgenticEvent::TurnComplete, speak_binding("user turn"));
        let user_path = write_cfg(tmp.path(), "user-config", &user_config);

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let mut repo_config = empty_cfg();
        repo_config.events.insert(AgenticEvent::SessionStart, speak_binding("repo hello"));
        write_cfg(&repo_dir, ".hooker", &repo_config);

        let loaded = load_config(Some(&user_path), Some(&repo_dir)).unwrap();
        // session_start replaced by repo
        match &loaded.events[&AgenticEvent::SessionStart].actions[0] {
            EventAction::Speak { message } => assert_eq!(message, "repo hello"),
            _ => panic!("Expected Speak"),
        }
        // turn_complete preserved from user
        match &loaded.events[&AgenticEvent::TurnComplete].actions[0] {
            EventAction::Speak { message } => assert_eq!(message, "user turn"),
            _ => panic!("Expected Speak"),
        }
    }

    #[test]
    fn merge_settings_field_by_field() {
        let user = HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings {
                default_log_target: Some(LogTarget::LocalFile {
                    path: PathBuf::from("/tmp/user.jsonl"),
                }),
                tts: Some(TtsSettings {
                    provider: Some("say".to_string()),
                    voice: Some("Samantha".to_string()),
                    rate: None,
                }),
            },
            events: HashMap::new(),
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
            },
            events: HashMap::new(),
        };
        let merged = merge_configs(user, repo);
        assert!(matches!(&merged.settings.default_log_target, Some(LogTarget::LocalFile { .. })));
        let tts = merged.settings.tts.unwrap();
        assert_eq!(tts.provider.as_deref(), Some("espeak"));
        assert_eq!(tts.rate, Some(1.5));
    }

    #[test]
    fn load_config_repo_only() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let mut config = empty_cfg();
        config.events.insert(AgenticEvent::BeforeTool, EventBinding {
            enabled: true,
            actions: vec![],
            matcher: Some("Bash".to_string()),
            overrides: HashMap::new(),
        });
        write_cfg(&repo_dir, ".hooker", &config);
        let loaded = load_config(Some(&tmp.path().join("nope")), Some(&repo_dir)).unwrap();
        assert!(loaded.events.contains_key(&AgenticEvent::BeforeTool));
    }
}
