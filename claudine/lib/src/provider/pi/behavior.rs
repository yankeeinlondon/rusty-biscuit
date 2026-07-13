//! Behavior-trait implementations for the Pi provider.

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

#[derive(Debug)]
pub(super) struct PiProvider;

pub(super) static PI_PROVIDER: PiProvider = PiProvider;

impl ProviderBehavior for PiProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        let _ = raw;
        // Pi has no native hooks, so it never delivers a raw hook payload for
        // shape-based detection; the wrapper path always knows the provider
        // from the `claudine pi` subcommand. (Consistent with
        // `representative_payload_for` returning `None`.)
        false
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::Pi, sink, config)
    }
}
impl McpBehavior for PiProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::Pi
    }
}
impl AdapterBehavior for PiProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::PI_ADAPTER
    }
}
impl ConfiguratorBehavior for PiProvider {
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::PiConfigurator)
    }
}
