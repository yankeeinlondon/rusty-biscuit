use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::provider::Provider;
use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};

pub(crate) struct GooseConfigurator;

impl AgentConfigurator for GooseConfigurator {
    fn provider(&self) -> Provider {
        Provider::Goose
    }

    fn register(
        &self,
        _plan: &ProviderHookPlan,
        config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        let config_path = config_path(config_dir);
        if !config_path.exists() {
            return Ok(RegistrationResult::Skipped(SkipReason::NotDetected));
        }

        // Goose supports hooks via the GOOSE_STATUS_HOOK environment variable,
        // but this requires setting an env var before running goose, not modifying
        // a config file. A wrapper script approach would be needed.
        //
        // TODO: Implement wrapper-based hook registration for Goose.
        // The wrapper would:
        // 1. Set GOOSE_STATUS_HOOK to point to a claudine handler script
        // 2. Run goose with the user's arguments
        //
        // For now, we skip registration with guidance.
        Ok(RegistrationResult::Skipped(SkipReason::WrapperOnly {
            guidance: "Run goose with GOOSE_STATUS_HOOK=claudine-goose-hook".to_string(),
        }))
    }

    fn deregister(&self, _config_dir: Option<&Path>) -> Result<()> {
        // Nothing to deregister - Goose uses env vars, not config files
        Ok(())
    }

    fn is_registered(&self, _config_dir: Option<&Path>) -> Result<bool> {
        // Can't detect env var registration from config file
        Ok(false)
    }

    fn registered_events(&self, _config_dir: Option<&Path>) -> Result<Vec<String>> {
        // No events registered via config file
        Ok(vec![])
    }

    fn supports_config_registration(&self) -> bool {
        false // Goose uses GOOSE_STATUS_HOOK env var, not config file
    }
}

/// Resolve the config.yaml path for Goose.
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("config.yaml"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".config").join("goose").join("config.yaml")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn register_skips_when_not_detected() {
        let tmp = TempDir::new().unwrap();
        let configurator = GooseConfigurator;
        let plan = ProviderHookPlan {
            events: vec![],
            canonical_for: None,
        };

        let result = configurator.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NotDetected)
        ));
    }

    #[test]
    fn register_returns_wrapper_only_when_detected() {
        let tmp = TempDir::new().unwrap();
        let config_path = tmp.path().join("config.yaml");
        fs::write(&config_path, "provider: openai\n").unwrap();

        let configurator = GooseConfigurator;
        let plan = ProviderHookPlan {
            events: vec![],
            canonical_for: None,
        };

        let result = configurator.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::WrapperOnly { .. })
        ));
    }

    #[test]
    fn provider_returns_goose() {
        let configurator = GooseConfigurator;
        assert_eq!(configurator.provider(), Provider::Goose);
    }
}
