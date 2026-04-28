//! Roo Code provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

#[derive(Debug)]
pub(super) struct RooProvider;

pub(super) static ROO_PROVIDER: RooProvider = RooProvider;

impl ProviderBehavior for RooProvider {}
impl McpBehavior for RooProvider {
    fn supported(&self) -> bool {
        true
    }
}
impl AdapterBehavior for RooProvider {}
impl ConfiguratorBehavior for RooProvider {}

pub(super) static ROO_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::RooCode,
    display_name: "Roo Code",
    slug: "roo_code",
    binary: "roo",
    agent_offset: ".roo",
    cli_aliases: &["roo", "roocode", "roo_code", "roo-code"],
    docs_url: "https://github.com/RooVetGit/Roo-Code",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Roo,
    supports_skills: true,
    behavior: &ROO_PROVIDER,
    mcp: &ROO_PROVIDER,
    adapter: &ROO_PROVIDER,
    configurator: &ROO_PROVIDER,
};
