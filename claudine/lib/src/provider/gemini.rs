//! Gemini CLI provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

#[derive(Debug)]
pub(super) struct GeminiProvider;

pub(super) static GEMINI_PROVIDER: GeminiProvider = GeminiProvider;

impl ProviderBehavior for GeminiProvider {}
impl McpBehavior for GeminiProvider {
    fn supported(&self) -> bool {
        true
    }
}
impl AdapterBehavior for GeminiProvider {}
impl ConfiguratorBehavior for GeminiProvider {
    fn hooks_supported(&self) -> bool {
        true
    }
}

pub(super) static GEMINI_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Gemini,
    display_name: "Gemini",
    slug: "gemini",
    binary: "gemini",
    agent_offset: ".gemini",
    cli_aliases: &["gemini"],
    docs_url: "https://github.com/google-gemini/gemini-cli",
    usage_dashboard_url: Some("https://aistudio.google.com/billing"),
    sniff_binding: AiCli::GeminiCli,
    supports_skills: true,
    behavior: &GEMINI_PROVIDER,
    mcp: &GEMINI_PROVIDER,
    adapter: &GEMINI_PROVIDER,
    configurator: &GEMINI_PROVIDER,
};
