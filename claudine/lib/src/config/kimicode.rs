use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::events::Provider;

use super::trait_def::{AgentConfigurator, ProviderHookPlan, RegistrationResult, SkipReason};

pub(crate) struct KimiCodeConfigurator;

impl AgentConfigurator for KimiCodeConfigurator {
    fn provider(&self) -> Provider {
        Provider::KimiCode
    }

    fn register(
        &self,
        _plan: &ProviderHookPlan,
        _config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        // Kimi Code doesn't support hooks via config files.
        // It uses a "wire mode" (--wire flag) JSON-RPC protocol that would
        // require a wrapper/proxy approach - not currently implemented.
        Ok(RegistrationResult::Skipped(SkipReason::NoHookSupport))
    }

    fn deregister(&self, _config_dir: Option<&Path>) -> Result<()> {
        // Nothing to deregister - Kimi Code uses wire mode, not config-based hooks
        Ok(())
    }

    fn is_registered(&self, _config_dir: Option<&Path>) -> Result<bool> {
        // Can't detect wire-mode registration from config file
        Ok(false)
    }

    fn registered_events(&self, _config_dir: Option<&Path>) -> Result<Vec<String>> {
        // No events registered via config file
        Ok(vec![])
    }

    fn supports_config_registration(&self) -> bool {
        false // KimiCode uses wire-mode JSON-RPC, not config file
    }
}

/// Resolve the config.json path for Kimi Code.
#[allow(dead_code)] // Reserved for future wire-mode implementation
fn config_path(config_dir: Option<&Path>) -> PathBuf {
    match config_dir {
        Some(dir) => dir.join("config.json"),
        None => {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(".kimi").join("config.json")
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn register_returns_no_hook_support() {
        // KimiCode always returns NoHookSupport since it doesn't support config-based hooks
        let tmp = TempDir::new().unwrap();
        let configurator = KimiCodeConfigurator;
        let plan = ProviderHookPlan {
            events: vec![],
            canonical_for: None,
        };

        let result = configurator.register(&plan, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::NoHookSupport)
        ));
    }

    #[test]
    fn provider_returns_kimicode() {
        let configurator = KimiCodeConfigurator;
        assert_eq!(configurator.provider(), Provider::KimiCode);
    }
}
