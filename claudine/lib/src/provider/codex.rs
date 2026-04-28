//! Codex CLI provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

#[derive(Debug)]
pub(super) struct CodexProvider;

pub(super) static CODEX_PROVIDER: CodexProvider = CodexProvider;

impl ProviderBehavior for CodexProvider {}
impl McpBehavior for CodexProvider {
    fn supported(&self) -> bool {
        true
    }
}
impl AdapterBehavior for CodexProvider {}
impl ConfiguratorBehavior for CodexProvider {
    fn hooks_supported(&self) -> bool {
        true
    }
}

pub(super) static CODEX_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Codex,
    display_name: "Codex",
    slug: "codex",
    binary: "codex",
    agent_offset: ".codex",
    cli_aliases: &["codex"],
    docs_url: "https://github.com/openai/codex",
    usage_dashboard_url: Some("https://platform.openai.com/usage"),
    sniff_binding: AiCli::Codex,
    supports_skills: true,
    behavior: &CODEX_PROVIDER,
    mcp: &CODEX_PROVIDER,
    adapter: &CODEX_PROVIDER,
    configurator: &CODEX_PROVIDER,
};
