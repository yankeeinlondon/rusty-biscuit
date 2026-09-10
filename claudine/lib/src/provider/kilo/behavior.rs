//! Behavior-trait implementations for the Kilo Code provider.
//!
//! Kilo Code's CLI is an OpenCode fork: its non-interactive `--format json`
//! output is OpenCode-shaped NDJSON, so the stream parser and event adapter
//! are reused verbatim from OpenCode rather than duplicated. Because the wire
//! shape is identical, raw-payload detection cannot distinguish Kilo from
//! OpenCode, so `detect_from_payload` is intentionally `false` — the wrapper
//! path always knows the provider from the `claudine kilo` subcommand.
//! Plugin-bus hooks are wired via `KiloConfigurator`, which emits an
//! OpenCode-style bridge plugin using Kilo's `@kilocode/plugin` package.
//! Native MCP is not wired yet; see the M-Kilo graduation report.

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

#[derive(Debug)]
pub(super) struct KiloProvider;

pub(super) static KILO_PROVIDER: KiloProvider = KiloProvider;

impl ProviderBehavior for KiloProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        // Kilo's payloads share OpenCode's shape (`event_type` / `eventType`),
        // so payload-shape detection cannot uniquely identify Kilo. The wrapper
        // path selects the parser from the known provider instead.
        let _ = raw;
        false
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::Kilo, sink, config)
    }
}
impl McpBehavior for KiloProvider {
    fn provider_for_error(&self) -> Provider {
        Provider::Kilo
    }
}
impl AdapterBehavior for KiloProvider {
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        // Kilo emits OpenCode-shaped events; the OpenCode adapter parses them
        // unchanged. A dedicated Kilo adapter would be pure duplication.
        &crate::hook_adapters::OPENCODE_ADAPTER
    }
}
impl ConfiguratorBehavior for KiloProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::KiloConfigurator)
    }
}
