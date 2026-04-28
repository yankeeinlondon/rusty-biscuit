use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the Codex agent capability descriptor served
/// by [`provider_info`].
#[derive(Debug, Clone, Default)]
pub struct CodexAgent;

impl CodexAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for CodexAgent {
    fn id(&self) -> Provider {
        Provider::Codex
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::Codex).agent_capabilities()
    }
}
