use crate::provider::{Provider, provider_info};

use super::model::{Agent, AgentCapabilities};

/// Thin facade that re-exposes the Claude Code agent capability descriptor
/// served by [`provider_info`].
///
/// Phase 2 of the centralized providers refactor moves the data construction
/// into [`crate::provider::claude`]; this struct now carries no per-instance
/// state.
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeAgent;

impl ClaudeCodeAgent {
    pub fn new() -> Self {
        Self
    }
}

impl Agent for ClaudeCodeAgent {
    fn id(&self) -> Provider {
        Provider::Claude
    }

    fn capabilities(&self) -> &AgentCapabilities {
        provider_info(Provider::Claude).agent_capabilities()
    }
}
