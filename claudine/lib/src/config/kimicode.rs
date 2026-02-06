use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::events::{HookerConfig, Provider};

use super::trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

pub(crate) struct KimiCodeConfigurator;

impl AgentConfigurator for KimiCodeConfigurator {
    fn provider(&self) -> Provider {
        Provider::KimiCode
    }

    fn register(
        &self,
        _config: &HookerConfig,
        _config_dir: Option<&Path>,
    ) -> Result<RegistrationResult> {
        // Kimi Code supports hooks via "wire mode" (--wire flag), which is a
        // bidirectional JSON-RPC protocol. The CLI sends events and waits for
        // responses, enabling blocking hooks like ApprovalRequest and ToolCallRequest.
        //
        // This requires a wrapper approach where:
        // 1. A middleware process intercepts JSON-RPC messages
        // 2. Routes claudine events appropriately
        // 3. Handles ApprovalRequest/ToolCallRequest blocking hooks
        //
        // TODO: Implement wire-mode proxy for Kimi Code.
        // For now, we skip registration with guidance.
        Ok(RegistrationResult::Skipped(SkipReason::WrapperOnly {
            guidance: "Use `kimi --wire` with a claudine JSON-RPC proxy".to_string(),
        }))
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
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn register_returns_wrapper_only() {
        // KimiCode always returns WrapperOnly since it requires wire-mode proxy
        let tmp = TempDir::new().unwrap();
        let configurator = KimiCodeConfigurator;
        let config = crate::events::HookerConfig {
            version: "1.0".to_string(),
            settings: Default::default(),
            providers: Default::default(),
        };

        let result = configurator.register(&config, Some(tmp.path())).unwrap();
        assert!(matches!(
            result,
            RegistrationResult::Skipped(SkipReason::WrapperOnly { .. })
        ));
    }

    #[test]
    fn provider_returns_kimicode() {
        let configurator = KimiCodeConfigurator;
        assert_eq!(configurator.provider(), Provider::KimiCode);
    }
}
