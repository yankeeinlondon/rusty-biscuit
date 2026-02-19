use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::Result;
use crate::events::{HookerConfig, Provider};

use super::trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

pub(crate) struct RooConfigurator;

impl AgentConfigurator for RooConfigurator {
    fn provider(&self) -> Provider {
        Provider::RooCode
    }

    fn register(
        &self,
        _config: &HookerConfig,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let settings_path = config_path(config_dir);
        if !settings_path.exists() {
            return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
        }

        // Roo Code currently exposes observational events only.
        // Flow-control uses client actions rather than hook return channels.
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
            .and_then(|value| value.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn registered_events(&self, _config_dir: Option<&Path>) -> Result<Vec<String>> {
        Ok(vec![])
    }

    fn supports_config_registration(&self) -> bool {
        false
    }
}

fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("settings.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".roo").join("settings.json")
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn register_skips_when_not_detected() {
        let tmp = TempDir::new().unwrap();
        let configurator = RooConfigurator;
        let config = HookerConfig {
            version: "1.0".to_string(),
            settings: Default::default(),
            providers: Default::default(),
        };

        let result = configurator.register(&config, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NotDetected)
        ));
    }

    #[test]
    fn provider_returns_roo_code() {
        let configurator = RooConfigurator;
        assert_eq!(configurator.provider(), Provider::RooCode);
    }
}
