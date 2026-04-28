//! Goose provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

#[derive(Debug)]
pub(super) struct GooseProvider;

pub(super) static GOOSE_PROVIDER: GooseProvider = GooseProvider;

impl ProviderBehavior for GooseProvider {}
impl McpBehavior for GooseProvider {}
impl AdapterBehavior for GooseProvider {}
impl ConfiguratorBehavior for GooseProvider {}

pub(super) static GOOSE_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::Goose,
    display_name: "Goose",
    slug: "goose",
    binary: "goose",
    agent_offset: ".goose",
    cli_aliases: &["goose"],
    docs_url: "https://block.github.io/goose/",
    usage_dashboard_url: None,
    sniff_binding: AiCli::Goose,
    supports_skills: false,
    behavior: &GOOSE_PROVIDER,
    mcp: &GOOSE_PROVIDER,
    adapter: &GOOSE_PROVIDER,
    configurator: &GOOSE_PROVIDER,
};
