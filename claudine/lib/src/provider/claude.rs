//! Claude Code provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

/// Zero-sized provider behavior implementor used as the trait-object value
/// for all four behavior trait fields on `CLAUDE_INFO`.
#[derive(Debug)]
pub(super) struct ClaudeProvider;

pub(super) static CLAUDE_PROVIDER: ClaudeProvider = ClaudeProvider;

impl ProviderBehavior for ClaudeProvider {}
impl McpBehavior for ClaudeProvider {
    fn supported(&self) -> bool {
        true
    }
}
impl AdapterBehavior for ClaudeProvider {}
impl ConfiguratorBehavior for ClaudeProvider {
    fn hooks_supported(&self) -> bool {
        true
    }
}

pub(super) static CLAUDE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Claude,
    display_name: "Claude",
    slug: "claude",
    binary: "claude",
    agent_offset: ".claude",
    cli_aliases: &["claude"],
    docs_url: "https://docs.anthropic.com/en/docs/claude-code",
    usage_dashboard_url: Some("https://console.anthropic.com/settings/billing"),
    sniff_binding: AiCli::Claude,
    supports_skills: true,
    behavior: &CLAUDE_PROVIDER,
    mcp: &CLAUDE_PROVIDER,
    adapter: &CLAUDE_PROVIDER,
    configurator: &CLAUDE_PROVIDER,
};
