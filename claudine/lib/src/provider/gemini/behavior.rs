//! Behavior-trait implementations for the Gemini CLI provider.

use std::path::{Path, PathBuf};

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::mcp::export::ExportServer;
use crate::mcp::inject::{GeminiInjector, McpInjector};
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

use super::data::GEMINI_EVENT_MAPPING;

#[derive(Debug)]
pub(super) struct GeminiProvider;

pub(super) static GEMINI_PROVIDER: GeminiProvider = GeminiProvider;

impl ProviderBehavior for GeminiProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        <Self as AdapterBehavior>::detect(self, raw)
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::Gemini, sink, config)
    }
}
impl McpBehavior for GeminiProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::Gemini
    }

    fn runtime_injector(&self) -> Option<Box<dyn McpInjector>> {
        Some(Box::new(GeminiInjector))
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_gemini_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_gemini_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".gemini").join("settings.json"),
            Scope::Repo(root) => root.join(".gemini").join("settings.json"),
        })
    }

    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        crate::mcp::export::read_existing_json_mcp_servers(config_path)
    }

    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        crate::mcp::export::write_gemini_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for GeminiProvider {
    fn detect(&self, raw: &serde_json::Value) -> bool {
        // Gemini emits two payload shapes: the standard `hook_event_name`
        // shape (where the name must be Gemini-only — Claude already owns
        // shared names like `Stop`/`PreToolUse` via display order) and a
        // legacy `event_name` field used by some Gemini integrations.
        if let Some(name) = raw.get("hook_event_name").and_then(|v| v.as_str()) {
            let claude_table = &crate::provider::claude::CLAUDE_EVENT_MAPPING;
            return GEMINI_EVENT_MAPPING.event_from_native_name(name).is_some()
                && claude_table.event_from_native_name(name).is_none();
        }
        raw.get("event_name").is_some()
    }

    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::GEMINI_ADAPTER
    }
}
impl ConfiguratorBehavior for GeminiProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::GeminiConfigurator)
    }
}
