use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};
use crate::error::Result;
use crate::provider::Provider;

/// Configurator for Pi.
///
/// Pi has no native hook system — its extensibility is executable TypeScript
/// extensions, not a declarative shell/HTTP/prompt hook config. So registration
/// always reports [`SkipReason::NoHookSupport`]; this is what keeps Pi's catalog
/// consistent (no `Hook`-level `event_mapping`, absent from `init --quick`).
pub(crate) struct PiConfigurator;

impl AgentConfigurator for PiConfigurator {
    fn provider(&self) -> Provider {
        Provider::Pi
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

        // Pi core has no declarative hook system; extension events are only
        // reachable via executable TypeScript extensions, not a config file.
        Ok(RegistrationResult::Skipped(SkipReason::NoHookSupport))
    }

    fn deregister(&self, _config_dir: Option<&Path>) -> Result<()> {
        // Nothing to deregister since we never registered hooks.
        Ok(())
    }

    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(false);
        }

        // Pi has no hooks; detect only an explicit claudine opt-in marker.
        let content = fs::read_to_string(&settings_path)?;
        let root: Value = serde_json::from_str(&content)?;
        Ok(root
            .get("claudine")
            .and_then(|c| c.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn registered_events(&self, _config_dir: Option<&Path>) -> Result<Vec<String>> {
        Ok(vec![])
    }

    fn supports_config_registration(&self) -> bool {
        false // Pi uses `--mode json` stream parsing, not config-file hooks.
    }
}

/// Resolve the settings.json path, using an override dir for tests.
///
/// Pi stores agent state under `~/.pi/agent/`.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("settings.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".pi").join("agent").join("settings.json")
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
        let configurator = PiConfigurator;
        let plan = test_plan(vec![AgenticEvent::TurnComplete]);

        let result = configurator.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NotDetected)
        ));
    }

    #[test]
    fn register_skips_due_to_no_hook_support() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"defaultModel": "claude-opus-4-8"}"#).unwrap();

        let configurator = PiConfigurator;
        let plan = test_plan(vec![AgenticEvent::TurnComplete]);

        let result = configurator.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NoHookSupport)
        ));
    }

    #[test]
    fn deregister_leaves_config_intact() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"defaultModel": "claude-opus-4-8"}"#).unwrap();

        let configurator = PiConfigurator;
        configurator.deregister(Some(tmp.path())).unwrap();
        assert!(settings.exists());
    }

    #[test]
    fn is_registered_reflects_claudine_marker() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");

        let configurator = PiConfigurator;
        assert!(!configurator.is_registered(Some(tmp.path())).unwrap());

        fs::write(&settings, r#"{"defaultModel": "x"}"#).unwrap();
        assert!(!configurator.is_registered(Some(tmp.path())).unwrap());

        fs::write(&settings, r#"{"claudine": {"enabled": true}}"#).unwrap();
        assert!(configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn provider_returns_pi() {
        assert_eq!(PiConfigurator.provider(), Provider::Pi);
    }
}
