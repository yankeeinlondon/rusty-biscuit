mod claude_code;
mod codex;
mod gemini_cli;
mod goose;
mod kimi_code;
mod model;
mod opencode;
mod qwen_cli;
mod registry;
mod roo_code;

pub use claude_code::ClaudeCodeAgent;
pub use codex::CodexAgent;
pub use gemini_cli::GeminiCliAgent;
pub use goose::GooseAgent;
pub use kimi_code::KimiCodeAgent;
pub use model::{
    ActivationStyle, Agent, AgentCapabilities, AgentDefinitionFormat, AgentDocs, AgentMeta,
    AreaConfidence, BillingCapabilities, BillingModel, CapabilityStatus, CommandFormat, Confidence,
    ConfidenceProfile, ConfigCapabilities, ConfigFormat, FrontmatterContract, InvocationStyle,
    LoggingCapabilities, ModelCapabilities, NonInteractiveCapabilities, PathDiscovery,
    PermissionCapabilities, ReasoningCapabilities, ReasoningStyle, RuntimeCapabilities,
    ScriptCapabilities, SkillsCapabilities, SlashCommandCapabilities, SubagentCapabilities,
    SystemPromptCapabilities,
};

/// Deprecated alias for [`crate::events::Provider`].
///
/// `AgentId` was the legacy provider identifier in the agents module before
/// it was unified with [`Provider`](crate::events::Provider) in the centralized
/// providers refactor. Use `Provider` directly in new code.
#[deprecated(
    since = "0.1.0",
    note = "use `claudine::events::Provider` (re-exported from `claudine::provider` after Phase 1) instead"
)]
pub type AgentId = crate::events::Provider;
pub use opencode::OpenCodeAgent;
pub use qwen_cli::QwenCliAgent;
pub use registry::{agent_for, all_agents, parse_agent_id};
pub use roo_code::RooCodeAgent;

#[cfg(test)]
mod tests;
