use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::Result;
use crate::provider::Provider;
use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};

pub(crate) struct QwenConfigurator;

impl AgentConfigurator for QwenConfigurator {
    fn provider(&self) -> Provider {
        Provider::QwenCode
    }

    fn register(
        &self,
        _plan: &ProviderHookPlan,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
        }

        // Qwen Code doesn't have a native hook system yet.
        // There's an open feature request for Claude-style hooks.
        // For now, we skip registration with a message indicating no hook support.
        Ok(RegistrationResult::Skipped(SkipReason::NoHookSupport))
    }

    fn deregister(&self, config_dir: Option<&Path>) -> Result<()> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(());
        }

        // Nothing to deregister since we never registered hooks
        Ok(())
    }

    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(false);
        }

        // Check if there's a claudine configuration in the settings
        let content = fs::read_to_string(&settings_path)?;
        let root: Value = serde_json::from_str(&content)?;

        // Look for any claudine-related configuration
        // Since Qwen Code doesn't have hooks, we check for a custom claudine key
        Ok(root
            .get("claudine")
            .and_then(|c| c.get("enabled"))
            .and_then(|e| e.as_bool())
            .unwrap_or(false))
    }

    fn registered_events(&self, config_dir: Option<&Path>) -> Result<Vec<String>> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(vec![]);
        }

        // No events registered since Qwen Code doesn't support hooks yet
        Ok(vec![])
    }

    fn supports_config_registration(&self) -> bool {
        false // QwenCode uses stream-json output, not config file hooks
    }
}

/// Resolve the settings.json path, using override dir for tests.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("settings.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".qwen").join("settings.json")
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::events::AgenticEvent;

    use super::*;

    fn test_plan(events: Vec<AgenticEvent>) -> ProviderHookPlan {
        ProviderHookPlan {
            events,
            canonical_for: None,
        }
    }

    #[test]
    fn register_skips_when_not_detected() {
        let tmp = TempDir::new().unwrap();
        let configurator = QwenConfigurator;
        let config = test_plan(vec![AgenticEvent::TurnComplete]);

        let result = configurator.register(&config, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NotDetected)
        ));
    }

    #[test]
    fn register_skips_due_to_no_hook_support() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"model": "qwen-coder"}"#).unwrap();

        let configurator = QwenConfigurator;
        let config = test_plan(vec![AgenticEvent::TurnComplete]);

        let result = configurator.register(&config, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NoHookSupport)
        ));
    }

    #[test]
    fn deregister_succeeds_when_no_config() {
        let tmp = TempDir::new().unwrap();
        let configurator = QwenConfigurator;

        // Should not error even when config doesn't exist
        configurator.deregister(Some(tmp.path())).unwrap();
    }

    #[test]
    fn deregister_succeeds_with_existing_config() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"model": "qwen-coder"}"#).unwrap();

        let configurator = QwenConfigurator;
        configurator.deregister(Some(tmp.path())).unwrap();

        // Config file should still exist (we don't modify it)
        assert!(settings.exists());
    }

    #[test]
    fn is_registered_returns_false_when_no_config() {
        let tmp = TempDir::new().unwrap();
        let configurator = QwenConfigurator;

        assert!(!configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn is_registered_returns_false_without_claudine_key() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"model": "qwen-coder"}"#).unwrap();

        let configurator = QwenConfigurator;
        assert!(!configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn is_registered_returns_true_with_claudine_enabled() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(
            &settings,
            r#"{"claudine": {"enabled": true}, "model": "qwen-coder"}"#,
        )
        .unwrap();

        let configurator = QwenConfigurator;
        assert!(configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn registered_events_returns_empty_vec() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"model": "qwen-coder"}"#).unwrap();

        let configurator = QwenConfigurator;
        let events = configurator.registered_events(Some(tmp.path())).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn provider_returns_qwen_code() {
        let configurator = QwenConfigurator;
        assert_eq!(configurator.provider(), Provider::QwenCode);
    }
}
