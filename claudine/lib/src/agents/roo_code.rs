use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the Roo Code agent capability descriptor
/// served by [`provider_info`].
#[derive(Debug, Clone, Default)]
pub struct RooCodeAgent;

impl RooCodeAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for RooCodeAgent {
    fn id(&self) -> Provider {
        Provider::RooCode
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::RooCode).agent_capabilities()
    }
}
