pub(crate) mod model;
mod registry;

pub use model::{
    ActivationStyle, Agent, AgentCapabilities, AgentDefinitionFormat, AgentDocs, AgentMeta,
    AreaConfidence, BillingCapabilities, BillingModel, CapabilityStatus, CommandFormat, Confidence,
    ConfidenceProfile, ConfigCapabilities, ConfigFormat, FrontmatterContract, InvocationStyle,
    LoggingCapabilities, ModelCapabilities, NonInteractiveCapabilities, PathDiscovery,
    PermissionCapabilities, ReasoningCapabilities, ReasoningStyle, RuntimeCapabilities,
    ScriptCapabilities, SkillsCapabilities, SlashCommandCapabilities, SubagentCapabilities,
    SystemPromptCapabilities,
};
pub use registry::{agent_for, all_agents, parse_agent_id};

/// Deprecated alias for [`crate::provider::Provider`].
///
/// `AgentId` was collapsed into `Provider` during Phase 0 of the centralized
/// providers refactor (see `claudine/features/2026-04-26-centralized-providers/design.md`
/// §6.1). This alias survives one release cycle so external consumers can
/// update without breakage; removal is scheduled for the post-Phase-8 cleanup
/// release.
#[deprecated(
    since = "0.9.0",
    note = "AgentId merged into `claudine::provider::Provider` during the centralized providers migration; use `Provider` directly"
)]
pub type AgentId = crate::provider::Provider;

#[cfg(test)]
mod tests;
