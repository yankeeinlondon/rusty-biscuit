//! Behavior-trait implementations for the Codex CLI provider.

use std::path::{Path, PathBuf};

use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::mcp::export::ExportServer;
use crate::mcp::inject::{CodexInjector, McpInjector};
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::provider::behavior::{
    AdapterBehavior, BoxedSemanticEventSink, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use crate::provider::identity::Provider;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;

#[derive(Debug)]
pub(super) struct CodexProvider;

pub(super) static CODEX_PROVIDER: CodexProvider = CodexProvider;

impl ProviderBehavior for CodexProvider {
    fn detect_from_payload(&self, raw: &serde_json::Value) -> bool {
        <Self as AdapterBehavior>::detect(self, raw)
    }

    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(Provider::Codex, sink, config)
    }
}
impl McpBehavior for CodexProvider {
    fn supported(&self) -> bool {
        true
    }

    fn provider_for_error(&self) -> Provider {
        Provider::Codex
    }

    fn runtime_injector(&self) -> Option<Box<dyn McpInjector>> {
        Some(Box::new(CodexInjector))
    }

    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        crate::mcp::import::discover_codex_configs(repo_root)
    }

    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        crate::mcp::import::parse_codex_mcp(config_path)
    }

    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Some(match scope {
            Scope::User => home.join(".codex").join("config.toml"),
            Scope::Repo(root) => root.join(".codex").join("config.toml"),
        })
    }

    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        crate::mcp::export::read_existing_codex_mcp_servers(config_path)
    }

    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        crate::mcp::export::write_codex_mcp(servers, config_path, managed_names)
    }
}
impl AdapterBehavior for CodexProvider {
    fn detect(&self, raw: &serde_json::Value) -> bool {
        looks_like_codex_payload(raw)
    }

    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        &crate::hook_adapters::CODEX_ADAPTER
    }
}

/// Recognize Codex payloads by their characteristic shape.
///
/// Codex emits several shapes that no other provider produces: a
/// thread-id key (`thread_id` or `thread-id`), a nested `hook_event` block
/// with `event_type: after_tool_use`, a top-level `event_type` of
/// `after_tool_use`, or one of Codex's tagged stream events under the
/// `type` field.
fn looks_like_codex_payload(raw: &serde_json::Value) -> bool {
    use serde_json::Value;

    if raw.get("thread_id").is_some() || raw.get("thread-id").is_some() {
        return true;
    }

    if raw
        .get("hook_event")
        .and_then(|value| value.get("event_type"))
        .and_then(Value::as_str)
        .is_some_and(|kind| matches!(kind, "after_tool_use"))
    {
        return true;
    }

    raw.get("event_type")
        .and_then(Value::as_str)
        .is_some_and(|kind| matches!(kind, "after_tool_use"))
        || raw.get("type").and_then(Value::as_str).is_some_and(|kind| {
            matches!(
                kind,
                "agent-turn-complete"
                    | "thread.started"
                    | "turn.started"
                    | "turn.completed"
                    | "turn.failed"
                    | "item.started"
                    | "item.updated"
                    | "item.completed"
                    | "error"
            )
        })
}
impl ConfiguratorBehavior for CodexProvider {
    fn hooks_supported(&self) -> bool {
        true
    }

    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        Box::new(crate::config::CodexConfigurator)
    }
}
