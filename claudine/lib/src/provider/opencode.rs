//! OpenCode provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

#[derive(Debug)]
pub(super) struct OpenCodeProvider;

pub(super) static OPENCODE_PROVIDER: OpenCodeProvider = OpenCodeProvider;

impl ProviderBehavior for OpenCodeProvider {}
impl McpBehavior for OpenCodeProvider {
    fn supported(&self) -> bool {
        true
    }
}
impl AdapterBehavior for OpenCodeProvider {}
impl ConfiguratorBehavior for OpenCodeProvider {
    fn hooks_supported(&self) -> bool {
        true
    }
}

pub(super) static OPENCODE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::OpenCode,
    display_name: "OpenCode",
    slug: "open_code",
    binary: "opencode",
    agent_offset: ".opencode",
    cli_aliases: &["opencode", "open_code", "open-code"],
    docs_url: "https://github.com/opencode-ai/opencode",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Opencode,
    supports_skills: true,
    behavior: &OPENCODE_PROVIDER,
    mcp: &OPENCODE_PROVIDER,
    adapter: &OPENCODE_PROVIDER,
    configurator: &OPENCODE_PROVIDER,
};
