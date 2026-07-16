//! Behavior-trait implementations for the Claude Code provider.

use std::path::{Path, PathBuf};

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::mcp::export::ExportServer;
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

use super::data::CLAUDE_EVENT_MAPPING;

/// Zero-sized provider behavior implementor used as the trait-object value
/// for all four behavior trait fields on `CLAUDE_INFO`.
#[derive(Debug)]
pub(super) struct ClaudeProvider;

pub(super) static CLAUDE_PROVIDER: ClaudeProvider = ClaudeProvider;

impl ProviderBehavior for ClaudeProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        <Self as AdapterBehavior>::detect(self, raw)
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::Claude, sink, config)
    }
}
impl McpBehavior for ClaudeProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::Claude
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_claude_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_claude_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".claude.json"),
            Scope::Repo(root) => root.join(".mcp.json"),
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
        crate::mcp::export::write_claude_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for ClaudeProvider {
    fn detect(&self, raw: &serde_json::Value) -> bool {
        // Claude payloads always carry a `hook_event_name` whose value is one
        // of the native hook names in the Claude event mapping table. The
        // `PROVIDERS_DISPLAY_ORDER` walk in `Provider::detect_from_payload`
        // visits Claude before Gemini, so any name shared with Gemini is
        // attributed to Claude — Gemini's `detect` guards against the
        // shared names explicitly.
        let Some(name) = raw.get("hook_event_name").and_then(|v| v.as_str()) else {
            return false;
        };
        CLAUDE_EVENT_MAPPING.event_from_native_name(name).is_some()
    }

    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::CLAUDE_ADAPTER
    }
}
impl ConfiguratorBehavior for ClaudeProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::ClaudeConfigurator)
    }
}
