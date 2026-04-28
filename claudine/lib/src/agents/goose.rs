use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the Goose agent capability descriptor served
/// by [`provider_info`].
#[derive(Debug, Clone, Default)]
pub struct GooseAgent;

impl GooseAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for GooseAgent {
    fn id(&self) -> Provider {
        Provider::Goose
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::Goose).agent_capabilities()
    }
}
