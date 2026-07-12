//! Behavior-trait implementations for the OpenCode provider.

use std::path::{Path, PathBuf};

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::mcp::export::ExportServer;
use crate::mcp::inject::{McpInjector, OpenCodeInjector};
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

#[derive(Debug)]
pub(super) struct OpenCodeProvider;

pub(super) static OPENCODE_PROVIDER: OpenCodeProvider = OpenCodeProvider;

impl ProviderBehavior for OpenCodeProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        <Self as AdapterBehavior>::detect(self, raw)
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::OpenCode, sink, config)
    }
}
impl McpBehavior for OpenCodeProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::OpenCode
    }

    fn runtime_injector(&self) -> Option<Box<dyn McpInjector>> {
        Some(Box::new(OpenCodeInjector))
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_opencode_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_opencode_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".config").join("opencode").join("opencode.json"),
            Scope::Repo(root) => root.join("opencode.json"),
        })
    }

    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        crate::mcp::export::read_existing_opencode_mcp_servers(config_path)
    }

    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        crate::mcp::export::write_opencode_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for OpenCodeProvider {
    fn detect(&self, raw: &serde_json::Value) -> bool {
        // OpenCode payloads always include either snake_case `event_type`
        // or camelCase `eventType` at the top level. Codex's `event_type`
        // is constrained to a small set of Codex-specific values which it
        // claims first via `PROVIDERS_DISPLAY_ORDER` ordering.
        raw.get("event_type").is_some() || raw.get("eventType").is_some()
    }

    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::OPENCODE_ADAPTER
    }
}
impl ConfiguratorBehavior for OpenCodeProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::OpenCodeConfigurator)
    }
}
