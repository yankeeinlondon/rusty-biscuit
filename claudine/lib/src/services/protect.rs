use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::events::Provider;

/// Central policy actor for Protect decisions.
///
/// The service is capability-aware: it computes a normalized decision first,
/// then downgrades when a provider cannot enforce that decision natively.
#[derive(Debug, Clone)]
pub struct ProtectService {
    config: ProtectConfig,
    profiles: ProviderProtectProfiles,
    state: ProtectState,
}

impl ProtectService {
    /// Build a Protect service with default provider capability profiles.
    pub fn new(config: ProtectConfig) -> Self {
        Self {
            config,
            profiles: ProviderProtectProfiles::defaults(),
            state: ProtectState::default(),
        }
    }

    /// Build a Protect service with explicit provider capability profiles.
    pub fn with_profiles(config: ProtectConfig, profiles: ProviderProtectProfiles) -> Self {
        Self {
            config,
            profiles,
            state: ProtectState::default(),
        }
    }

    /// Return the active protect configuration.
    pub fn config(&self) -> &ProtectConfig {
        &self.config
    }

    /// Return the capability profile map in use.
    pub fn profiles(&self) -> &ProviderProtectProfiles {
        &self.profiles
    }

    /// Evaluate one protection input and return a normalized decision.
    pub fn evaluate(&mut self, input: &ProtectInput) -> ProtectDecision {
        let mut decision = if !self.config.enabled {
            ProtectDecision::allow("protect.disabled")
        } else {
            self.evaluate_enabled(input)
        };

        self.state.record(input, &decision);

        // Keep rolling forensic context bounded for long-running sessions.
        while self.state.recent.len() > self.config.max_recent_decisions as usize {
            self.state.recent.pop_front();
        }

        // Preserve an explicit reason when no downgrade occurred.
        if decision.reason.is_empty() {
            decision.reason = "protect.default".to_string();
        }

        decision
    }

    /// Read-only access to state snapshots useful for telemetry/reporting.
    pub fn state(&self) -> &ProtectState {
        &self.state
    }

    fn evaluate_enabled(&self, input: &ProtectInput) -> ProtectDecision {
        let posture = self.effective_posture(input.provider);
        let capability = self.profiles.capabilities(input.provider);

        let desired = desired_outcome(input, posture);
        let desired_reason = desired.reason_code(input.runtime_mode, posture);

        if input.runtime_mode == ProtectRuntimeMode::Yolo
            && self.config.yolo.force_advisory_for_medium_risk
            && matches!(input.risk, RiskLevel::Medium)
        {
            return ProtectDecision::degraded(
                ProtectOutcome::AdvisoryOnly {
                    reason: "yolo.medium-risk-advisory".to_string(),
                },
                desired.outcome,
                "protect.yolo.medium-risk-advisory".to_string(),
            );
        }

        if input.runtime_mode == ProtectRuntimeMode::Yolo
            && !self.config.yolo.allow_critical_blocking
            && matches!(
                desired.outcome,
                ProtectOutcome::StopCurrent { .. } | ProtectOutcome::StopSession { .. }
            )
        {
            return ProtectDecision::degraded(
                ProtectOutcome::AdvisoryOnly {
                    reason: "yolo.blocking-disabled".to_string(),
                },
                desired.outcome,
                "protect.yolo.blocking-disabled".to_string(),
            );
        }

        if let Some(degraded) = downgrade_for_capability(desired.outcome.clone(), input, capability)
        {
            return ProtectDecision::degraded(degraded, desired.outcome, desired_reason);
        }

        ProtectDecision {
            outcome: desired.outcome,
            degraded_from: None,
            degraded: false,
            reason: desired_reason,
            capability: Some(capability_for_phase(input.phase, &capability)),
        }
    }

    fn effective_posture(&self, provider: Provider) -> ProtectPosture {
        self.config
            .providers
            .get(&provider)
            .and_then(|override_cfg| override_cfg.posture)
            .unwrap_or(self.config.posture)
    }
}

#[derive(Debug, Clone)]
struct DesiredDecision {
    outcome: ProtectOutcome,
}

impl DesiredDecision {
    fn reason_code(&self, mode: ProtectRuntimeMode, posture: ProtectPosture) -> String {
        format!("protect.{}.{posture}", mode.as_str())
    }
}

fn desired_outcome(input: &ProtectInput, posture: ProtectPosture) -> DesiredDecision {
    let outcome = match input.risk {
        RiskLevel::Low => ProtectOutcome::Allow,
        RiskLevel::Medium => match posture {
            ProtectPosture::Advisory => ProtectOutcome::AdvisoryOnly {
                reason: "medium-risk".to_string(),
            },
            ProtectPosture::Balanced | ProtectPosture::Strict => {
                ProtectOutcome::AskThenAllowOrStop {
                    reason: "medium-risk".to_string(),
                }
            }
        },
        RiskLevel::High => match posture {
            ProtectPosture::Advisory => ProtectOutcome::AdvisoryOnly {
                reason: "high-risk".to_string(),
            },
            ProtectPosture::Balanced | ProtectPosture::Strict => {
                ProtectOutcome::AskThenAllowOrStop {
                    reason: "high-risk".to_string(),
                }
            }
        },
        RiskLevel::Critical => match posture {
            ProtectPosture::Advisory => ProtectOutcome::AdvisoryOnly {
                reason: "critical-risk".to_string(),
            },
            ProtectPosture::Balanced => ProtectOutcome::StopCurrent {
                reason: "critical-risk".to_string(),
            },
            ProtectPosture::Strict => ProtectOutcome::StopSession {
                reason: "critical-risk".to_string(),
            },
        },
    };

    DesiredDecision { outcome }
}

fn downgrade_for_capability(
    outcome: ProtectOutcome,
    input: &ProtectInput,
    capability: ProviderProtectCapabilities,
) -> Option<ProtectOutcome> {
    let gate = capability_for_phase(input.phase, &capability);

    match outcome {
        ProtectOutcome::StopCurrent { .. } if !gate.can_stop_current() => {
            Some(ProtectOutcome::AdvisoryOnly {
                reason: "capability.no-stop-current".to_string(),
            })
        }
        ProtectOutcome::StopSession { .. } if !gate.can_stop_session() => {
            Some(ProtectOutcome::AdvisoryOnly {
                reason: "capability.no-stop-session".to_string(),
            })
        }
        ProtectOutcome::AskThenAllowOrStop { .. } if !gate.can_ask_user() => {
            Some(ProtectOutcome::AdvisoryOnly {
                reason: "capability.no-ask".to_string(),
            })
        }
        ProtectOutcome::AllowWithRedaction { .. } if !gate.can_modify() => {
            Some(ProtectOutcome::AdvisoryOnly {
                reason: "capability.no-redaction".to_string(),
            })
        }
        _ => None,
    }
}

fn capability_for_phase(
    phase: ProtectPhase,
    capabilities: &ProviderProtectCapabilities,
) -> GateCapability {
    match phase {
        ProtectPhase::BeforeTool => capabilities.pre_tool_gate,
        ProtectPhase::BeforePrompt => capabilities.user_prompt_gate,
        ProtectPhase::McpResponse => capabilities.mcp_response_gate,
        ProtectPhase::Completion => capabilities.completion_gate,
        ProtectPhase::SubagentStart | ProtectPhase::SubagentStop => {
            match capabilities.subagent_visibility {
                VisibilityLevel::None => GateCapability::None,
                VisibilityLevel::Partial => GateCapability::Influence,
                VisibilityLevel::Full => GateCapability::Guarantee,
            }
        }
        ProtectPhase::Runtime | ProtectPhase::AfterTool => GateCapability::Influence,
    }
}

/// Serializable top-level configuration for the Protect service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectConfig {
    /// Global on/off switch for protect evaluations.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Baseline policy strictness.
    #[serde(default)]
    pub posture: ProtectPosture,

    /// Behavior overrides when the provider is in bypass/YOLO posture.
    #[serde(default)]
    pub yolo: YoloPolicy,

    /// Generic command/path/secret rules.
    #[serde(default)]
    pub rules: ProtectRules,

    /// Completion gate behavior.
    #[serde(default)]
    pub completion: CompletionPolicy,

    /// MCP trust and redaction policy.
    #[serde(default)]
    pub mcp: McpPolicy,

    /// Subagent policy controls.
    #[serde(default)]
    pub subagents: SubagentPolicy,

    /// Privilege/runtime hardening controls.
    #[serde(default)]
    pub privilege: PrivilegePolicy,

    /// Optional provider-specific overrides.
    #[serde(default)]
    pub providers: HashMap<Provider, ProviderProtectOverride>,

    /// Max in-memory decision records retained for forensic inspection.
    #[serde(default = "default_max_recent_decisions")]
    pub max_recent_decisions: u16,
}

impl Default for ProtectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            posture: ProtectPosture::Balanced,
            yolo: YoloPolicy::default(),
            rules: ProtectRules::default(),
            completion: CompletionPolicy::default(),
            mcp: McpPolicy::default(),
            subagents: SubagentPolicy::default(),
            privilege: PrivilegePolicy::default(),
            providers: HashMap::default(),
            max_recent_decisions: default_max_recent_decisions(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_max_recent_decisions() -> u16 {
    256
}

/// Posture controls baseline aggressiveness for medium/high risk actions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectPosture {
    /// Never block in-line. Only collect findings and recommend next actions.
    Advisory,
    /// Ask for risky actions, block critical actions.
    #[default]
    Balanced,
    /// Prefer hard stops for any high-confidence dangerous behavior.
    Strict,
}

impl std::fmt::Display for ProtectPosture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtectPosture::Advisory => write!(f, "advisory"),
            ProtectPosture::Balanced => write!(f, "balanced"),
            ProtectPosture::Strict => write!(f, "strict"),
        }
    }
}

/// Runtime mode for a single evaluation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectRuntimeMode {
    #[default]
    Normal,
    Yolo,
}

impl ProtectRuntimeMode {
    fn as_str(self) -> &'static str {
        match self {
            ProtectRuntimeMode::Normal => "normal",
            ProtectRuntimeMode::Yolo => "yolo",
        }
    }
}

/// High-level event phase where a protect decision is being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectPhase {
    BeforePrompt,
    BeforeTool,
    AfterTool,
    McpResponse,
    Completion,
    SubagentStart,
    SubagentStop,
    Runtime,
}

/// Risk level assigned by upstream detection logic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Input envelope for evaluating one potential protection decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectInput {
    pub provider: Provider,
    pub phase: ProtectPhase,
    #[serde(default)]
    pub runtime_mode: ProtectRuntimeMode,
    #[serde(default)]
    pub risk: RiskLevel,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
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
#[serde(deny_unknown_fields)]
pub struct ProtectDecision {
    pub outcome: ProtectOutcome,
    #[serde(default)]
    pub degraded_from: Option<ProtectOutcome>,
    pub degraded: bool,
    pub reason: String,
    #[serde(default)]
    pub capability: Option<GateCapability>,
}

impl ProtectDecision {
    fn allow(reason: &str) -> Self {
        Self {
            outcome: ProtectOutcome::Allow,
            degraded_from: None,
            degraded: false,
            reason: reason.to_string(),
            capability: None,
        }
    }

    fn degraded(outcome: ProtectOutcome, original: ProtectOutcome, reason: String) -> Self {
        Self {
            outcome,
            degraded_from: Some(original),
            degraded: true,
            reason,
            capability: None,
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

/// Optional provider-specific overrides layered on top of [`ProtectConfig`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProtectOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub posture: Option<ProtectPosture>,
    #[serde(default)]
    pub yolo: Option<YoloPolicy>,
    #[serde(default)]
    pub rules: Option<ProtectRules>,
    #[serde(default)]
    pub completion: Option<CompletionPolicy>,
    #[serde(default)]
    pub mcp: Option<McpPolicy>,
    #[serde(default)]
    pub subagents: Option<SubagentPolicy>,
    #[serde(default)]
    pub privilege: Option<PrivilegePolicy>,
}

/// Controls how policy behaves in bypass/YOLO runtime mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YoloPolicy {
    #[serde(default = "default_allow_critical_blocking")]
    pub allow_critical_blocking: bool,
    #[serde(default = "default_force_advisory_for_medium_risk")]
    pub force_advisory_for_medium_risk: bool,
    #[serde(default = "default_collect_forensic_trail")]
    pub collect_forensic_trail: bool,
}

impl Default for YoloPolicy {
    fn default() -> Self {
        Self {
            allow_critical_blocking: default_allow_critical_blocking(),
            force_advisory_for_medium_risk: default_force_advisory_for_medium_risk(),
            collect_forensic_trail: default_collect_forensic_trail(),
        }
    }
}

fn default_allow_critical_blocking() -> bool {
    true
}

fn default_force_advisory_for_medium_risk() -> bool {
    true
}

fn default_collect_forensic_trail() -> bool {
    true
}

/// Shared tool/prompt risk matching patterns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectRules {
    #[serde(default)]
    pub blocked_command_patterns: Vec<String>,
    #[serde(default)]
    pub ask_command_patterns: Vec<String>,
    #[serde(default)]
    pub protected_paths: Vec<String>,
    #[serde(default)]
    pub secret_patterns: Vec<String>,
}

/// Completion validation controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionPolicy {
    #[serde(default = "default_true_value")]
    pub enabled: bool,
    #[serde(default = "default_completion_max_retries")]
    pub max_retries: u8,
    #[serde(default)]
    pub check_commands: Vec<String>,
    #[serde(default = "default_true_value")]
    pub secret_scan: bool,
}

impl Default for CompletionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: default_completion_max_retries(),
            check_commands: Vec::new(),
            secret_scan: true,
        }
    }
}

fn default_completion_max_retries() -> u8 {
    3
}

/// MCP trust and response handling policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpPolicy {
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub denylist: Vec<String>,
    #[serde(default)]
    pub redact_patterns: Vec<String>,
    #[serde(default = "default_true_value")]
    pub block_instruction_payloads: bool,
}

impl Default for McpPolicy {
    fn default() -> Self {
        Self {
            allowlist: Vec::new(),
            denylist: Vec::new(),
            redact_patterns: Vec::new(),
            block_instruction_payloads: true,
        }
    }
}

/// Subagent defaults used when providers allow subagent-specific controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubagentPolicy {
    #[serde(default = "default_true_value")]
    pub enabled: bool,
    #[serde(default = "default_true_value")]
    pub tighten_permissions: bool,
    #[serde(default)]
    pub default_profile: SubagentProfile,
}

impl Default for SubagentPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            tighten_permissions: true,
            default_profile: SubagentProfile::ReadMostly,
        }
    }
}

/// Named subagent permission profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentProfile {
    #[default]
    ReadMostly,
    Default,
    Isolated,
}

/// Hardening knobs for elevated privilege and weak isolation environments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivilegePolicy {
    #[serde(default = "default_true_value")]
    pub deny_when_root_without_sandbox: bool,
    #[serde(default = "default_true_value")]
    pub require_ask_for_network_writes: bool,
    #[serde(default = "default_true_value")]
    pub require_ask_for_broad_fs_writes: bool,
}

impl Default for PrivilegePolicy {
    fn default() -> Self {
        Self {
            deny_when_root_without_sandbox: true,
            require_ask_for_network_writes: true,
            require_ask_for_broad_fs_writes: true,
        }
    }
}

fn default_true_value() -> bool {
    true
}

/// In-memory rolling state retained by the protect evaluator.
#[derive(Debug, Clone, Default)]
pub struct ProtectState {
    /// Total decisions evaluated by this service instance.
    pub decision_count: u64,
    /// Bounded rolling records for post-run audits.
    pub recent: VecDeque<ProtectDecisionRecord>,
}

impl ProtectState {
    fn record(&mut self, input: &ProtectInput, decision: &ProtectDecision) {
        self.decision_count += 1;
        self.recent.push_back(ProtectDecisionRecord {
            provider: input.provider,
            phase: input.phase,
            mode: input.runtime_mode,
            risk: input.risk,
            outcome: decision.outcome.clone(),
            degraded: decision.degraded,
            reason: decision.reason.clone(),
            session_id: input.session_id.clone(),
        });
    }
}

/// Lightweight decision log entry for telemetry/report generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectDecisionRecord {
    pub provider: Provider,
    pub phase: ProtectPhase,
    pub mode: ProtectRuntimeMode,
    pub risk: RiskLevel,
    pub outcome: ProtectOutcome,
    pub degraded: bool,
    pub reason: String,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_have_all_supported_providers() {
        let profiles = ProviderProtectProfiles::defaults();
        assert!(profiles.capabilities(Provider::Claude).sandbox_available);
        assert!(profiles.capabilities(Provider::Codex).sandbox_available);
        assert!(profiles.capabilities(Provider::Gemini).sandbox_available);
        assert!(profiles.capabilities(Provider::Goose).bypass_mode_available);
        assert!(
            profiles.capabilities(Provider::KimiCode).pre_tool_gate == GateCapability::Guarantee
        );
        assert!(
            profiles.capabilities(Provider::OpenCode).mcp_response_gate
                == GateCapability::Influence
        );
        assert!(
            profiles.capabilities(Provider::QwenCode).pre_tool_gate == GateCapability::Influence
        );
        assert!(
            profiles.capabilities(Provider::RooCode).completion_gate == GateCapability::Guarantee
        );
    }

    #[test]
    fn critical_risk_degrades_when_provider_cannot_block() {
        let mut service = ProtectService::new(ProtectConfig {
            posture: ProtectPosture::Strict,
            ..ProtectConfig::default()
        });

        let decision = service.evaluate(&ProtectInput {
            provider: Provider::Codex,
            phase: ProtectPhase::BeforeTool,
            runtime_mode: ProtectRuntimeMode::Normal,
            risk: RiskLevel::Critical,
            summary: None,
            session_id: None,
        });

        assert!(decision.degraded);
        assert!(matches!(
            decision.outcome,
            ProtectOutcome::AdvisoryOnly { .. }
        ));
    }

    #[test]
    fn yolo_medium_risk_is_forced_to_advisory_by_default() {
        let mut service = ProtectService::new(ProtectConfig::default());

        let decision = service.evaluate(&ProtectInput {
            provider: Provider::Claude,
            phase: ProtectPhase::BeforeTool,
            runtime_mode: ProtectRuntimeMode::Yolo,
            risk: RiskLevel::Medium,
            summary: None,
            session_id: Some("abc".to_string()),
        });

        assert!(decision.degraded);
        assert!(matches!(
            decision.outcome,
            ProtectOutcome::AdvisoryOnly { .. }
        ));
        assert_eq!(service.state().decision_count, 1);
    }
}
