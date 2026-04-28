use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the Kimi Code agent capability descriptor
/// served by [`provider_info`].
#[derive(Debug, Clone, Default)]
pub struct KimiCodeAgent;

impl KimiCodeAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for KimiCodeAgent {
    fn id(&self) -> Provider {
        Provider::KimiCode
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::KimiCode).agent_capabilities()
    }
}
