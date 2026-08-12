//! Behavior-trait implementations for the Antigravity provider.

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

#[derive(Debug)]
pub(super) struct AntigravityProvider;

pub(super) static ANTIGRAVITY_PROVIDER: AntigravityProvider = AntigravityProvider;

impl ProviderBehavior for AntigravityProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        let _ = raw;
        // agy delivers no raw hook payload for shape-based detection; the
        // wrapper path always knows the provider from the `claudine antigravity`
        // subcommand. (Consistent with `representative_payload_for` → `None`.)
        false
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::Antigravity, sink, config)
    }
}
impl McpBehavior for AntigravityProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::Antigravity
    }
}
impl AdapterBehavior for AntigravityProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::ANTIGRAVITY_ADAPTER
    }
}
impl ConfiguratorBehavior for AntigravityProvider {
    fn hooks_supported(&self) -> bool {
        // agy has a real file-based hook system (~/.gemini/config/hooks.json)
        // whose subsystem loads during `--print` runs; AntigravityConfigurator
        // registers Claudine's handlers there.
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::AntigravityConfigurator)
    }
}
