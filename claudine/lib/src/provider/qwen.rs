//! Qwen Code provider definition.

use sniff::programs::AiCli;

use super::behavior::{
    AdapterBehavior, ConfiguratorBehavior, McpBehavior, ProviderBehavior,
};
use super::identity::Provider;
use super::ProviderInfo;

#[derive(Debug)]
pub(super) struct QwenProvider;

pub(super) static QWEN_PROVIDER: QwenProvider = QwenProvider;

impl ProviderBehavior for QwenProvider {}
impl McpBehavior for QwenProvider {}
impl AdapterBehavior for QwenProvider {}
impl ConfiguratorBehavior for QwenProvider {}

pub(super) static QWEN_INFO: ProviderInfo = ProviderInfo {
    provider: Provider::QwenCode,
    display_name: "Qwen Code",
    slug: "qwen_code",
    binary: "qwen",
    agent_offset: ".qwen",
    cli_aliases: &["qwen", "qwencode", "qwen_code", "qwen-code"],
    docs_url: "https://qwenlm.github.io/qwen-code-docs/",
    usage_dashboard_url: Some("https://bailian.console.aliyun.com/"),
    sniff_binding: AiCli::QwenCli,
    supports_skills: true,
    behavior: &QWEN_PROVIDER,
    mcp: &QWEN_PROVIDER,
    adapter: &QWEN_PROVIDER,
    configurator: &QWEN_PROVIDER,
};
