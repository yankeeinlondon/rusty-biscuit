//! Behavior-trait implementations for the Goose provider.

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::provider::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;

#[derive(Debug)]
pub(super) struct GooseProvider;

pub(super) static GOOSE_PROVIDER: GooseProvider = GooseProvider;

impl ProviderBehavior for GooseProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        let _ = raw;
        // Goose has no representative raw hook payload shape in the catalog yet.
        false
    }
}
impl McpBehavior for GooseProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::Goose
    }
}
impl AdapterBehavior for GooseProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::GOOSE_ADAPTER
    }
}
impl ConfiguratorBehavior for GooseProvider {
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::GooseConfigurator)
    }
}
