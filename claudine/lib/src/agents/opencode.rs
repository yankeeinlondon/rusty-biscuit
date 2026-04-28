use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the OpenCode agent capability descriptor
/// served by [`provider_info`].
#[derive(Debug, Clone, Default)]
pub struct OpenCodeAgent;

impl OpenCodeAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for OpenCodeAgent {
    fn id(&self) -> Provider {
        Provider::OpenCode
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::OpenCode).agent_capabilities()
    }
}
