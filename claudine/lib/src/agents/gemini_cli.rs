use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the Gemini CLI agent capability descriptor
/// served by [`provider_info`].
#[derive(Debug, Clone, Default)]
pub struct GeminiCliAgent;

impl GeminiCliAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for GeminiCliAgent {
    fn id(&self) -> Provider {
        Provider::Gemini
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::Gemini).agent_capabilities()
    }
}
