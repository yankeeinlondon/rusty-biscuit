use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::events::Provider;
use crate::permissions::PolicyWarning;
use crate::permissions::query::QueryResult;

use super::intent::ProtectIntent;
use super::redact::ProtectRedactionPlan;

/// Whether the evaluation used effective (CLI-resolved) or configured policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectPolicyMode {
    /// Full effective policy resolved with CLI args.
    Effective,
    /// Configured policy used as fallback (no CLI context available).
    ConfiguredFallback,
}

/// Structured result of a protect evaluation.
#[derive(Debug, Clone)]
pub struct ProtectEvaluation {
    pub decision: ProtectDecision,
    pub policy_mode: ProtectPolicyMode,
    pub findings: Vec<ProtectFinding>,
    pub redaction: Option<ProtectRedactionPlan>,
    pub warnings: Vec<PolicyWarning>,
}

/// One finding produced during protect evaluation.
#[derive(Debug, Clone)]
pub struct ProtectFinding {
    pub intent: ProtectIntent,
    pub result: QueryResult,
    pub severity: ProtectSeverity,
    pub source: ProtectFindingSource,
}

/// Where a finding originated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectFindingSource {
    PolicyQuery,
    RuntimeGuard,
    McpRedaction,
    CompletionLoop,
}

/// Severity level for a protect finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectSeverity {
    Info,
    Medium,
    High,
    Critical,
}

/// Normalized outcome produced by Protect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtectOutcome {
    Allow,
    AskThenAllowOrStop { reason: String },
    StopCurrent { reason: String },
    StopSession { reason: String },
    AllowWithRedaction { reason: String },
    AdvisoryOnly { reason: String },
}

/// Final decision with downgrade metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectDecision {
    pub outcome: ProtectOutcome,
    pub desired_outcome: ProtectOutcome,
    pub degraded: bool,
    pub reason: String,
    #[serde(default)]
    pub capability: Option<GateCapability>,
}

impl ProtectDecision {
    pub(crate) fn allow(reason: &str) -> Self {
        Self {
            outcome: ProtectOutcome::Allow,
            desired_outcome: ProtectOutcome::Allow,
            degraded: false,
            reason: reason.to_string(),
            capability: None,
        }
    }

    pub(crate) fn degraded(
        outcome: ProtectOutcome,
        desired: ProtectOutcome,
        reason: String,
    ) -> Self {
        Self {
            degraded: outcome != desired,
            outcome,
            desired_outcome: desired,
            reason,
            capability: None,
        }
    }

    pub(crate) fn new(
        outcome: ProtectOutcome,
        desired: ProtectOutcome,
        reason: String,
        capability: Option<GateCapability>,
    ) -> Self {
        Self {
            degraded: outcome != desired,
            outcome,
            desired_outcome: desired,
            reason,
            capability,
        }
    }
}

/// Capability levels for provider-native gate control.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateCapability {
    #[default]
    None,
    Influence,
    Guarantee,
}

impl GateCapability {
    /// Can the provider ask the user and enforce deny/allow?
    pub fn can_ask_user(self) -> bool {
        !matches!(self, GateCapability::None)
    }

    /// Can the provider mutate response/input content before model consumption?
    pub fn can_modify(self) -> bool {
        !matches!(self, GateCapability::None)
    }

    /// Can the provider reliably stop only the current operation/turn?
    pub fn can_stop_current(self) -> bool {
        matches!(self, GateCapability::Influence | GateCapability::Guarantee)
    }

    /// Can the provider reliably end the entire run/session?
    pub fn can_stop_session(self) -> bool {
        matches!(self, GateCapability::Guarantee)
    }
}

/// Degree of subagent event visibility.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityLevel {
    #[default]
    None,
    Partial,
    Full,
}

/// Provider-specific control surface summary used by Protect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProtectCapabilities {
    pub pre_tool_gate: GateCapability,
    pub user_prompt_gate: GateCapability,
    pub mcp_response_gate: GateCapability,
    pub completion_gate: GateCapability,
    pub subagent_visibility: VisibilityLevel,
    pub subagent_policy_control: bool,
    pub sandbox_available: bool,
    pub bypass_mode_available: bool,
}

/// Capability profiles keyed by provider.
#[derive(Debug, Clone, Default)]
pub struct ProviderProtectProfiles {
    by_provider: HashMap<Provider, ProviderProtectCapabilities>,
}

impl ProviderProtectProfiles {
    /// Build provider profiles based on current researched hook/control surfaces.
    pub fn defaults() -> Self {
        let mut by_provider = HashMap::new();

        by_provider.insert(
            Provider::Claude,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::Guarantee,
                user_prompt_gate: GateCapability::Guarantee,
                mcp_response_gate: GateCapability::Guarantee,
                completion_gate: GateCapability::Guarantee,
                subagent_visibility: VisibilityLevel::Full,
                subagent_policy_control: true,
                sandbox_available: true,
                bypass_mode_available: true,
            },
        );

        by_provider.insert(
            Provider::Codex,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::None,
                user_prompt_gate: GateCapability::None,
                mcp_response_gate: GateCapability::None,
                completion_gate: GateCapability::None,
                subagent_visibility: VisibilityLevel::None,
                subagent_policy_control: true,
                sandbox_available: true,
                bypass_mode_available: true,
            },
        );

        by_provider.insert(
            Provider::Gemini,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::Guarantee,
                user_prompt_gate: GateCapability::Guarantee,
                mcp_response_gate: GateCapability::Guarantee,
                completion_gate: GateCapability::Guarantee,
                subagent_visibility: VisibilityLevel::Partial,
                subagent_policy_control: true,
                sandbox_available: true,
                bypass_mode_available: true,
            },
        );

        by_provider.insert(
            Provider::Goose,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::None,
                user_prompt_gate: GateCapability::None,
                mcp_response_gate: GateCapability::None,
                completion_gate: GateCapability::None,
                subagent_visibility: VisibilityLevel::Partial,
                subagent_policy_control: true,
                sandbox_available: true,
                bypass_mode_available: true,
            },
        );

        by_provider.insert(
            Provider::KimiCode,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::Guarantee,
                user_prompt_gate: GateCapability::Influence,
                mcp_response_gate: GateCapability::None,
                completion_gate: GateCapability::None,
                subagent_visibility: VisibilityLevel::Partial,
                subagent_policy_control: true,
                sandbox_available: false,
                bypass_mode_available: true,
            },
        );

        by_provider.insert(
            Provider::OpenCode,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::Guarantee,
                user_prompt_gate: GateCapability::Influence,
                mcp_response_gate: GateCapability::Influence,
                completion_gate: GateCapability::Influence,
                subagent_visibility: VisibilityLevel::Partial,
                subagent_policy_control: true,
                sandbox_available: false,
                bypass_mode_available: false,
            },
        );

        by_provider.insert(
            Provider::QwenCode,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::Influence,
                user_prompt_gate: GateCapability::None,
                mcp_response_gate: GateCapability::None,
                completion_gate: GateCapability::None,
                subagent_visibility: VisibilityLevel::None,
                subagent_policy_control: true,
                sandbox_available: true,
                bypass_mode_available: true,
            },
        );

        by_provider.insert(
            Provider::RooCode,
            ProviderProtectCapabilities {
                pre_tool_gate: GateCapability::Guarantee,
                user_prompt_gate: GateCapability::None,
                mcp_response_gate: GateCapability::None,
                completion_gate: GateCapability::Guarantee,
                subagent_visibility: VisibilityLevel::Full,
                subagent_policy_control: true,
                sandbox_available: false,
                bypass_mode_available: true,
            },
        );

        Self { by_provider }
    }

    /// Insert or replace capabilities for a provider.
    pub fn insert(&mut self, provider: Provider, capabilities: ProviderProtectCapabilities) {
        self.by_provider.insert(provider, capabilities);
    }

    /// Fetch capabilities for a provider, falling back to an empty profile.
    pub fn capabilities(&self, provider: Provider) -> ProviderProtectCapabilities {
        self.by_provider.get(&provider).copied().unwrap_or_default()
    }
}
