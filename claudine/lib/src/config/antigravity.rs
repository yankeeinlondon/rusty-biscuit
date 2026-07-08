use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};
use crate::error::Result;
use crate::provider::Provider;

/// Configurator for Antigravity.
///
/// agy exposes no headless hook-registration surface (its file-based hooks.json
/// belongs to the interactive/IDE surfaces and was unverified on host), so from
/// Claudine's print-mode perspective it is a no-hook provider: registration
/// always reports [`SkipReason::NoHookSupport`]. This is what keeps agy's
/// catalog consistent (no `Hook`-level `event_mapping`, absent from
/// `init --quick`), mirroring the Pi/Qwen configurators.
pub(crate) struct AntigravityConfigurator;

impl AgentConfigurator for AntigravityConfigurator {
    fn provider(&self) -> Provider {
        Provider::Antigravity
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
        // No headless hook contract is confirmed for agy; Claudine derives every
        // lifecycle event from the `--output-format json` envelope instead.
        Ok(RegistrationResult::Skipped(SkipReason::NoHookSupport))
    }

    fn deregister(&self, _config_dir: Option<&Path>) -> Result<()> {
        Ok(())
    }

    fn is_registered(&self, config_dir: Option<&Path>) -> Result<bool> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(false);
        }
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
        false // agy uses `--output-format json` envelope parsing, not config-file hooks.
    }
}

/// Resolve the settings.json path, using an override dir for tests.
///
/// agy stores CLI state under `~/.gemini/antigravity-cli/`.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("settings.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".gemini")
                .join("antigravity-cli")
                .join("settings.json")
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
        let configurator = AntigravityConfigurator;
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
        fs::write(tmp.path().join("settings.json"), r#"{"model": "Gemini 3.1 Pro (High)"}"#)
            .unwrap();
        let configurator = AntigravityConfigurator;
        let plan = test_plan(vec![AgenticEvent::TurnComplete]);
        let result = configurator.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NoHookSupport)
        ));
    }

    #[test]
    fn is_registered_reflects_claudine_marker() {
        let tmp = TempDir::new().unwrap();
        let settings = tmp.path().join("settings.json");
        let configurator = AntigravityConfigurator;
        assert!(!configurator.is_registered(Some(tmp.path())).unwrap());
        fs::write(&settings, r#"{"model": "x"}"#).unwrap();
        assert!(!configurator.is_registered(Some(tmp.path())).unwrap());
        fs::write(&settings, r#"{"claudine": {"enabled": true}}"#).unwrap();
        assert!(configurator.is_registered(Some(tmp.path())).unwrap());
    }

    #[test]
    fn provider_returns_antigravity() {
        assert_eq!(AntigravityConfigurator.provider(), Provider::Antigravity);
    }
}
