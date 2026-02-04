use std::path::Path;

use crate::error::Result;
use crate::events::{HookerConfig, Provider};

use super::trait_def::{AgentConfigurator, RegistrationResult, SkipReason};

pub(crate) struct RooConfigurator;

impl AgentConfigurator for RooConfigurator {
    fn provider(&self) -> Provider {
        Provider::RooCode
    }

    fn register(&self, _config: &HookerConfig, _config_dir: Option<&Path>) -> Result<RegistrationResult> {
        Ok(RegistrationResult::Skipped(SkipReason::WrapperOnly {
            guidance: "Roo Code uses a wrapper-only approach. See claudine docs for setup.".into(),
        }))
    }

    fn deregister(&self, _config_dir: Option<&Path>) -> Result<()> {
        Ok(())
    }

    fn is_registered(&self, _config_dir: Option<&Path>) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::events::{AgenticEvent, EventBinding, GlobalSettings};

    use super::*;

    fn test_config() -> HookerConfig {
        let mut events = HashMap::new();
        events.insert(
            AgenticEvent::TurnComplete,
            EventBinding {
                enabled: true,
                actions: vec![],
                matcher: None,
                overrides: HashMap::new(),
            },
        );
        HookerConfig {
            version: "1.0".to_string(),
            settings: GlobalSettings::default(),
            events,
        }
    }

    #[test]
    fn returns_wrapper_only_skip() {
        let configurator = RooConfigurator;
        let config = test_config();
        let result = configurator.register(&config, None).unwrap();
        match result {
            RegistrationResult::Skipped(SkipReason::WrapperOnly { guidance }) => {
                assert!(guidance.contains("wrapper"));
            }
            _ => panic!("expected WrapperOnly skip"),
        }
    }

    #[test]
    fn deregister_is_noop() {
        let configurator = RooConfigurator;
        configurator.deregister(None).unwrap();
    }

    #[test]
    fn is_registered_always_false() {
        let configurator = RooConfigurator;
        assert!(!configurator.is_registered(None).unwrap());
    }
}
