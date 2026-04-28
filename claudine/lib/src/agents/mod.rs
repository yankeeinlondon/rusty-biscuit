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

#[cfg(test)]
mod tests;
