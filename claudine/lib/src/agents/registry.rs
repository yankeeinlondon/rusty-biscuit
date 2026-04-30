use std::sync::OnceLock;

use crate::provider::{PROVIDERS_DISPLAY_ORDER, Provider, provider_info};

use super::model::Agent;

static ALL_AGENTS: OnceLock<Vec<&'static dyn Agent>> = OnceLock::new();

/// Returns the [`Agent`] descriptor for the given [`Provider`].
///
/// Since Phase 8 of the centralized providers refactor, this is a thin
/// forwarding layer over [`provider_info`]: every `ProviderInfo` implements
/// [`Agent`], so the same `'static` reference satisfies both abstractions.
pub fn agent_for(provider: Provider) -> &'static dyn Agent {
    provider_info(provider)
}

pub fn all_agents() -> &'static [&'static dyn Agent] {
    ALL_AGENTS
        .get_or_init(|| PROVIDERS_DISPLAY_ORDER.into_iter().map(agent_for).collect())
        .as_slice()
}

pub fn parse_agent_id(input: &str) -> Option<Provider> {
    Provider::parse_cli_name(input)
}
