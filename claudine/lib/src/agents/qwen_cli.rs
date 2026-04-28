use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the Qwen Code CLI agent capability
/// descriptor served by [`provider_info`].
#[derive(Debug, Clone, Default)]
pub struct QwenCliAgent;

impl QwenCliAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for QwenCliAgent {
    fn id(&self) -> Provider {
        Provider::QwenCode
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::QwenCode).agent_capabilities()
    }
}
