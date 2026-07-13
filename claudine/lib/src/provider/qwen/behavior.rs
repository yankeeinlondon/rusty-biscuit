//! Behavior-trait implementations for the Qwen Code provider.

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

#[derive(Debug)]
pub(super) struct QwenProvider;

pub(super) static QWEN_PROVIDER: QwenProvider = QwenProvider;

impl ProviderBehavior for QwenProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        let _ = raw;
        // Qwen has no representative raw hook payload shape in the catalog yet.
        false
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::QwenCode, sink, config)
    }
}
impl McpBehavior for QwenProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::QwenCode
    }
}
impl AdapterBehavior for QwenProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::QWEN_ADAPTER
    }
}
impl ConfiguratorBehavior for QwenProvider {
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::QwenConfigurator)
    }
}
