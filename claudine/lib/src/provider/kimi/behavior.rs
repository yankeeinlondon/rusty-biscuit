//! Behavior-trait implementations for the Kimi Code provider.

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

#[derive(Debug)]
pub(super) struct KimiProvider;

pub(super) static KIMI_PROVIDER: KimiProvider = KimiProvider;

impl ProviderBehavior for KimiProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        <Self as AdapterBehavior>::detect(self, raw)
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::KimiCode, sink, config)
    }
}
impl McpBehavior for KimiProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::KimiCode
    }
}
impl AdapterBehavior for KimiProvider {
    fn detect(&self, raw: &serde_json::Value) -> bool {
        // Kimi's payloads are JSON-RPC framed and always carry a `method`
        // field. No other provider in the catalog uses the same shape, so
        // detection is the simple field-presence check.
        raw.get("method").is_some()
    }

    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::KIMI_ADAPTER
    }
}
impl ConfiguratorBehavior for KimiProvider {
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::KimiCodeConfigurator)
    }
}
