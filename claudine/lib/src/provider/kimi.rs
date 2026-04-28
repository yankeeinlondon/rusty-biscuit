//! Kimi Code provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

#[derive(Debug)]
pub(super) struct KimiProvider;

pub(super) static KIMI_PROVIDER: KimiProvider = KimiProvider;

impl ProviderBehavior for KimiProvider {}
impl McpBehavior for KimiProvider {}
impl AdapterBehavior for KimiProvider {}
impl ConfiguratorBehavior for KimiProvider {}

pub(super) static KIMI_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::KimiCode,
    display_name: "Kimi Code",
    slug: "kimi_code",
    binary: "kimi",
    agent_offset: ".kimi",
    cli_aliases: &["kimi", "kimicode", "kimi_code", "kimi-code"],
    docs_url: "https://moonshotai.github.io/kimi-cli/en/",
    usage_dashboard_url: Some("https://platform.moonshot.cn/console/account"),
    sniff_binding: AiCli::KimiCli,
    supports_skills: false,
    behavior: &KIMI_PROVIDER,
    mcp: &KIMI_PROVIDER,
    adapter: &KIMI_PROVIDER,
    configurator: &KIMI_PROVIDER,
};
