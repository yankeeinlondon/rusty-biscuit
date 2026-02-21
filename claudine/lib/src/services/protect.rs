use std::collections::{HashMap, VecDeque};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ClaudineError, Result};
use crate::events::{AgenticEvent, EventMeta, Provider};

const GLOBAL_SESSION_KEY: &str = "__global__";

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

    /// Resolve the effective policy for a specific provider.
    pub fn resolve_policy_for_provider(&self, provider: Provider) -> ProtectConfig {
        let mut resolved = self.config.clone();
        if let Some(override_cfg) = self.config.providers.get(&provider) {
            resolved = resolved.merge_provider_override(override_cfg);
        }
        resolved
    }

    /// Evaluate one protection input and return a normalized decision.
    pub fn evaluate(&mut self, input: &ProtectInput) -> ProtectDecision {
        let policy = self.resolve_policy_for_provider(input.provider);

        let mut decision = if !policy.enabled {
            ProtectDecision::allow("protect.disabled")
        } else {
            self.evaluate_enabled(input, &policy)
        };

        decision = self.apply_completion_retry_policy(input, &policy, decision);

        self.state.record(input, &decision);

        // Keep rolling forensic context bounded for long-running sessions.
        while self.state.recent.len() > policy.max_recent_decisions as usize {
            self.state.recent.pop_front();
        }

        // Preserve an explicit reason when no downgrade occurred.
        if decision.reason.is_empty() {
            decision.reason = "protect.default".to_string();
        }

        decision
    }

    /// Build and evaluate one protect input from normalized event metadata.
    pub fn evaluate_from_event(
        &mut self,
        provider: Provider,
        event: AgenticEvent,
        meta: &EventMeta,
    ) -> Option<ProtectDecision> {
        let input = ProtectInput::from_event_meta(provider, event, meta)?;
        Some(self.evaluate(&input))
    }

    /// Redact MCP text payload using provider-effective policy.
    pub fn redact_mcp_text(&self, provider: Provider, text: &str) -> Result<McpTextRedaction> {
        let policy = self.resolve_policy_for_provider(provider);
        redact_text_with_policy(text, &policy.mcp, &policy.rules.secret_patterns)
    }

    /// Redact MCP JSON payload using provider-effective policy.
    pub fn redact_mcp_json(&self, provider: Provider, value: &Value) -> Result<McpJsonRedaction> {
        let policy = self.resolve_policy_for_provider(provider);
        redact_json_with_policy(value, &policy.mcp, &policy.rules.secret_patterns)
    }

    /// Snapshot decision records for reporting/auditing.
    pub fn snapshot_records(&self) -> Vec<ProtectDecisionRecord> {
        self.state.snapshot_records()
    }

    /// Export full state snapshot for reports/telemetry.
    pub fn export_state(&self) -> ProtectStateExport {
        self.state.export_state()
    }

    /// Export state records as JSONL for log sinks.
    pub fn export_records_jsonl(&self) -> Result<String> {
        self.state.export_records_jsonl()
    }

    /// Read-only access to state snapshots useful for telemetry/reporting.
    pub fn state(&self) -> &ProtectState {
        &self.state
    }

    fn evaluate_enabled(&self, input: &ProtectInput, policy: &ProtectConfig) -> ProtectDecision {
        let posture = policy.posture;
        let capability = self.profiles.capabilities(input.provider);

        let desired = desired_outcome(input, policy, posture);
        let desired_reason = desired.reason_code(input.runtime_mode, posture);

        if input.runtime_mode == ProtectRuntimeMode::Yolo
            && policy.yolo.force_advisory_for_medium_risk
            && matches!(input.risk, RiskLevel::Medium)
            && matches!(desired.outcome, ProtectOutcome::AskThenAllowOrStop { .. })
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
            && !policy.yolo.allow_critical_blocking
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

    fn apply_completion_retry_policy(
        &mut self,
        input: &ProtectInput,
        policy: &ProtectConfig,
        mut decision: ProtectDecision,
    ) -> ProtectDecision {
        if input.phase != ProtectPhase::Completion || !policy.completion.enabled {
            return decision;
        }

        let session_key = input
            .session_id
            .clone()
            .unwrap_or_else(|| GLOBAL_SESSION_KEY.to_string());

        if matches!(
            decision.outcome,
            ProtectOutcome::Allow | ProtectOutcome::AllowWithRedaction { .. }
        ) {
            self.state
                .completion_retries_by_session
                .remove(&session_key);
            return decision;
        }

        let retry_count = self
            .state
            .completion_retries_by_session
            .entry(session_key)
            .or_default();
        *retry_count = retry_count.saturating_add(1);

        if *retry_count > policy.completion.max_retries {
            decision = ProtectDecision {
                outcome: ProtectOutcome::StopSession {
                    reason: "completion.loop-protection.max-retries".to_string(),
                },
                degraded_from: Some(decision.outcome),
                degraded: true,
                reason: "protect.completion.loop-protection".to_string(),
                capability: decision.capability,
            };
        }

        decision
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

fn desired_outcome(
    input: &ProtectInput,
    policy: &ProtectConfig,
    posture: ProtectPosture,
) -> DesiredDecision {
    let mut candidates = Vec::new();

    if let Some(privilege) = evaluate_privilege_policy(input, &policy.privilege, posture) {
        candidates.push(privilege);
    }

    if let Some(rule_outcome) = evaluate_rule_policy(input, &policy.rules, posture) {
        candidates.push(rule_outcome);
    }

    if let Some(mcp_outcome) = evaluate_mcp_policy(input, &policy.mcp, &policy.rules, posture) {
        candidates.push(mcp_outcome);
    }

    candidates.push(fallback_risk_outcome(input.risk, posture));

    let outcome = select_precedence(candidates);
    DesiredDecision { outcome }
}

fn evaluate_privilege_policy(
    input: &ProtectInput,
    privilege: &PrivilegePolicy,
    posture: ProtectPosture,
) -> Option<ProtectOutcome> {
    if privilege.deny_when_root_without_sandbox
        && input.runtime_is_root
        && matches!(input.runtime_has_sandbox, Some(false))
    {
        return Some(stop_outcome_for_posture(
            posture,
            "privilege.root-without-sandbox",
        ));
    }

    if privilege.require_ask_for_network_writes && input.network_write {
        return Some(ProtectOutcome::AskThenAllowOrStop {
            reason: "privilege.network-write".to_string(),
        });
    }

    if privilege.require_ask_for_broad_fs_writes && input.broad_fs_write {
        return Some(ProtectOutcome::AskThenAllowOrStop {
            reason: "privilege.broad-fs-write".to_string(),
        });
    }

    None
}

fn evaluate_rule_policy(
    input: &ProtectInput,
    rules: &ProtectRules,
    posture: ProtectPosture,
) -> Option<ProtectOutcome> {
    let command_blob = command_blob(input);
    let text_blob = text_blob(input);

    let blocked_matches =
        match_patterns(&rules.blocked_command_patterns, &command_blob).unwrap_or_default();
    let ask_matches =
        match_patterns(&rules.ask_command_patterns, &command_blob).unwrap_or_default();

    let protected_paths = input
        .paths
        .iter()
        .filter(|path| path_matches(path, &rules.protected_paths))
        .cloned()
        .collect::<Vec<_>>();

    let secret_matches = match_patterns(&rules.secret_patterns, &text_blob).unwrap_or_default();

    let has_deny = !blocked_matches.is_empty() || !secret_matches.is_empty();
    let has_ask = !ask_matches.is_empty() || !protected_paths.is_empty();

    if has_deny {
        let reason = if has_ask {
            "rules.conflict-prefer-deny"
        } else if !secret_matches.is_empty() {
            "rules.secret-pattern"
        } else {
            "rules.blocked-command"
        };
        return Some(stop_outcome_for_posture(posture, reason));
    }

    if has_ask {
        return Some(ProtectOutcome::AskThenAllowOrStop {
            reason: if !protected_paths.is_empty() {
                "rules.protected-path".to_string()
            } else {
                "rules.ask-command".to_string()
            },
        });
    }

    None
}

fn evaluate_mcp_policy(
    input: &ProtectInput,
    mcp: &McpPolicy,
    rules: &ProtectRules,
    posture: ProtectPosture,
) -> Option<ProtectOutcome> {
    if input.phase != ProtectPhase::McpResponse {
        return None;
    }

    if let Some(server_id) = input.mcp_server_id.as_deref() {
        if !mcp.allowlist.is_empty() && !mcp.allowlist.iter().any(|allowed| allowed == server_id) {
            return Some(ProtectOutcome::AskThenAllowOrStop {
                reason: "mcp.server-not-allowlisted".to_string(),
            });
        }
        if mcp.denylist.iter().any(|denied| denied == server_id) {
            return Some(stop_outcome_for_posture(posture, "mcp.server-denylisted"));
        }
    }

    if let Some(payload_text) = input.payload_text.as_deref() {
        if mcp.block_instruction_payloads && contains_instruction_payload(payload_text) {
            return Some(stop_outcome_for_posture(
                posture,
                "mcp.instruction-payload-blocked",
            ));
        }

        let redaction = redact_text_with_policy(payload_text, mcp, &rules.secret_patterns).ok()?;
        if redaction.redacted {
            return Some(ProtectOutcome::AllowWithRedaction {
                reason: "mcp.redacted-text".to_string(),
            });
        }
    }

    if let Some(payload_json) = input.payload_json.as_ref() {
        let redaction = redact_json_with_policy(payload_json, mcp, &rules.secret_patterns).ok()?;

        if redaction.blocked_instruction_payload {
            return Some(stop_outcome_for_posture(
                posture,
                "mcp.instruction-payload-blocked",
            ));
        }

        if redaction.redacted {
            return Some(ProtectOutcome::AllowWithRedaction {
                reason: "mcp.redacted-json".to_string(),
            });
        }
    }

    None
}

fn fallback_risk_outcome(risk: RiskLevel, posture: ProtectPosture) -> ProtectOutcome {
    match risk {
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
        RiskLevel::Critical => stop_outcome_for_posture(posture, "critical-risk"),
    }
}

fn select_precedence(mut outcomes: Vec<ProtectOutcome>) -> ProtectOutcome {
    outcomes
        .drain(..)
        .max_by_key(outcome_priority)
        .unwrap_or(ProtectOutcome::Allow)
}

fn outcome_priority(outcome: &ProtectOutcome) -> u8 {
    match outcome {
        ProtectOutcome::StopSession { .. } => 6,
        ProtectOutcome::StopCurrent { .. } => 5,
        ProtectOutcome::AskThenAllowOrStop { .. } => 4,
        ProtectOutcome::AllowWithRedaction { .. } => 3,
        ProtectOutcome::AdvisoryOnly { .. } => 2,
        ProtectOutcome::Allow => 1,
    }
}

fn stop_outcome_for_posture(posture: ProtectPosture, reason: &str) -> ProtectOutcome {
    match posture {
        ProtectPosture::Advisory => ProtectOutcome::AdvisoryOnly {
            reason: reason.to_string(),
        },
        ProtectPosture::Balanced => ProtectOutcome::StopCurrent {
            reason: reason.to_string(),
        },
        ProtectPosture::Strict => ProtectOutcome::StopSession {
            reason: reason.to_string(),
        },
    }
}

fn command_blob(input: &ProtectInput) -> String {
    [
        input.command.as_deref(),
        input.summary.as_deref(),
        input.tool_name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

fn text_blob(input: &ProtectInput) -> String {
    let mut parts = vec![
        input.command.as_deref(),
        input.summary.as_deref(),
        input.prompt.as_deref(),
        input.payload_text.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(ToOwned::to_owned)
    .collect::<Vec<_>>();

    if let Some(payload) = input.payload_json.as_ref() {
        parts.push(payload.to_string());
    }

    parts.join("\n")
}

fn path_matches(path: &str, protected_paths: &[String]) -> bool {
    protected_paths.iter().any(|protected| {
        path == protected || path.starts_with(protected) || path.contains(protected)
    })
}

fn match_patterns(patterns: &[String], text: &str) -> Result<Vec<String>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let mut matched = Vec::new();
    for pattern in patterns {
        let regex = Regex::new(pattern).map_err(|source| ClaudineError::ProtectRuleParse {
            pattern: pattern.clone(),
            source,
        })?;

        if regex.is_match(text) {
            matched.push(pattern.clone());
        }
    }

    Ok(matched)
}

fn redact_text_with_policy(
    text: &str,
    mcp: &McpPolicy,
    secret_patterns: &[String],
) -> Result<McpTextRedaction> {
    let mut output = text.to_string();
    let mut redactions_applied = 0u32;
    let mut blocked_instruction_payload = false;

    if mcp.block_instruction_payloads && contains_instruction_payload(text) {
        blocked_instruction_payload = true;
        output = "[instruction-payload-removed]".to_string();
    }

    for pattern in mcp.redact_patterns.iter().chain(secret_patterns.iter()) {
        let regex = Regex::new(pattern).map_err(|source| ClaudineError::ProtectRuleParse {
            pattern: pattern.clone(),
            source,
        })?;

        let count = regex.find_iter(&output).count() as u32;
        if count > 0 {
            output = regex.replace_all(&output, "[REDACTED]").to_string();
            redactions_applied = redactions_applied.saturating_add(count);
        }
    }

    Ok(McpTextRedaction {
        text: output,
        redacted: redactions_applied > 0 || blocked_instruction_payload,
        blocked_instruction_payload,
        redactions_applied,
    })
}

fn redact_json_with_policy(
    value: &Value,
    mcp: &McpPolicy,
    secret_patterns: &[String],
) -> Result<McpJsonRedaction> {
    let mut redactions_applied = 0u32;
    let mut blocked_instruction_payload = false;

    fn visit(
        value: &Value,
        mcp: &McpPolicy,
        secret_patterns: &[String],
        redactions_applied: &mut u32,
        blocked_instruction_payload: &mut bool,
    ) -> Result<Value> {
        match value {
            Value::String(text) => {
                let redaction = redact_text_with_policy(text, mcp, secret_patterns)?;
                if redaction.redactions_applied > 0 {
                    *redactions_applied =
                        redactions_applied.saturating_add(redaction.redactions_applied);
                }
                if redaction.blocked_instruction_payload {
                    *blocked_instruction_payload = true;
                }
                Ok(Value::String(redaction.text))
            }
            Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(visit(
                        item,
                        mcp,
                        secret_patterns,
                        redactions_applied,
                        blocked_instruction_payload,
                    )?);
                }
                Ok(Value::Array(out))
            }
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (key, item) in map {
                    out.insert(
                        key.clone(),
                        visit(
                            item,
                            mcp,
                            secret_patterns,
                            redactions_applied,
                            blocked_instruction_payload,
                        )?,
                    );
                }
                Ok(Value::Object(out))
            }
            Value::Bool(_) | Value::Null | Value::Number(_) => Ok(value.clone()),
        }
    }

    let value = visit(
        value,
        mcp,
        secret_patterns,
        &mut redactions_applied,
        &mut blocked_instruction_payload,
    )?;

    Ok(McpJsonRedaction {
        value,
        redacted: redactions_applied > 0 || blocked_instruction_payload,
        blocked_instruction_payload,
        redactions_applied,
    })
}

fn contains_instruction_payload(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    [
        "ignore previous instructions",
        "system prompt",
        "developer instructions",
        "do not reveal",
        "tool instructions",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
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

    /// Allow repository-level config to weaken stricter user posture.
    #[serde(default)]
    pub allow_repo_posture_downgrade: bool,

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
            allow_repo_posture_downgrade: false,
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

impl ProtectConfig {
    /// Validate semantic constraints and regex configuration.
    pub fn validate(&self) -> Result<()> {
        if self.max_recent_decisions == 0 {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.max_recent_decisions must be > 0".to_string(),
            ));
        }

        if self.max_recent_decisions > 10_000 {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.max_recent_decisions must be <= 10000".to_string(),
            ));
        }

        if self.completion.enabled && self.completion.max_retries == 0 {
            return Err(ClaudineError::ProtectInvalidPolicy(
                "settings.protect.completion.max_retries must be > 0 when completion is enabled"
                    .to_string(),
            ));
        }

        validate_patterns(&self.rules.blocked_command_patterns)?;
        validate_patterns(&self.rules.ask_command_patterns)?;
        validate_patterns(&self.rules.secret_patterns)?;
        validate_patterns(&self.mcp.redact_patterns)?;

        for (provider, override_cfg) in &self.providers {
            if let Some(rules) = override_cfg.rules.as_ref() {
                if let Some(patterns) = rules.blocked_command_patterns.as_ref() {
                    validate_patterns(patterns)?;
                }
                if let Some(patterns) = rules.ask_command_patterns.as_ref() {
                    validate_patterns(patterns)?;
                }
                if let Some(patterns) = rules.secret_patterns.as_ref() {
                    validate_patterns(patterns)?;
                }
            }
            if let Some(mcp) = override_cfg.mcp.as_ref() {
                if let Some(patterns) = mcp.redact_patterns.as_ref() {
                    validate_patterns(patterns)?;
                }
            }
            if let Some(completion) = override_cfg.completion.as_ref()
                && completion.enabled.unwrap_or(false)
                && completion
                    .max_retries
                    .unwrap_or(default_completion_max_retries())
                    == 0
            {
                return Err(ClaudineError::ProtectInvalidPolicy(format!(
                    "settings.protect.providers.{provider}.completion.max_retries must be > 0 when completion is enabled"
                )));
            }
        }

        Ok(())
    }

    /// Merge another protect config on top of this config.
    pub fn merge_with(&self, overlay: &ProtectConfig) -> ProtectConfig {
        let mut merged = self.clone();
        merged.enabled = overlay.enabled;
        merged.posture = overlay.posture;
        merged.allow_repo_posture_downgrade = overlay.allow_repo_posture_downgrade;
        merged.yolo = overlay.yolo.clone();
        merged.rules = overlay.rules.clone();
        merged.completion = overlay.completion.clone();
        merged.mcp = overlay.mcp.clone();
        merged.subagents = overlay.subagents.clone();
        merged.privilege = overlay.privilege.clone();
        merged.max_recent_decisions = overlay.max_recent_decisions;

        for (provider, provider_override) in &overlay.providers {
            merged
                .providers
                .insert(*provider, provider_override.clone());
        }

        merged
    }

    /// Merge one provider override onto this config.
    pub fn merge_provider_override(
        &self,
        provider_override: &ProviderProtectOverride,
    ) -> ProtectConfig {
        let mut merged = self.clone();

        if let Some(enabled) = provider_override.enabled {
            merged.enabled = enabled;
        }
        if let Some(posture) = provider_override.posture {
            merged.posture = posture;
        }

        if let Some(yolo) = provider_override.yolo.as_ref() {
            merged.yolo = merge_yolo_policy(&merged.yolo, yolo);
        }
        if let Some(rules) = provider_override.rules.as_ref() {
            merged.rules = merge_rules_policy(&merged.rules, rules);
        }
        if let Some(completion) = provider_override.completion.as_ref() {
            merged.completion = merge_completion_policy(&merged.completion, completion);
        }
        if let Some(mcp) = provider_override.mcp.as_ref() {
            merged.mcp = merge_mcp_policy(&merged.mcp, mcp);
        }
        if let Some(subagents) = provider_override.subagents.as_ref() {
            merged.subagents = merge_subagent_policy(&merged.subagents, subagents);
        }
        if let Some(privilege) = provider_override.privilege.as_ref() {
            merged.privilege = merge_privilege_policy(&merged.privilege, privilege);
        }

        merged
    }

    /// Build provider-aware defaults for init flows.
    pub fn provider_aware_defaults(
        installed: &[Provider],
        posture: ProtectPosture,
    ) -> ProtectConfig {
        let mut config = ProtectConfig {
            posture,
            ..ProtectConfig::default()
        };

        for provider in installed {
            if matches!(
                provider,
                Provider::Codex | Provider::Goose | Provider::QwenCode
            ) {
                config.providers.insert(
                    *provider,
                    ProviderProtectOverride {
                        posture: Some(ProtectPosture::Advisory),
                        completion: Some(CompletionPolicyOverride {
                            enabled: Some(false),
                            ..CompletionPolicyOverride::default()
                        }),
                        ..ProviderProtectOverride::default()
                    },
                );
            }
        }

        config
    }
}

fn validate_patterns(patterns: &[String]) -> Result<()> {
    for pattern in patterns {
        Regex::new(pattern).map_err(|source| ClaudineError::ProtectRuleParse {
            pattern: pattern.clone(),
            source,
        })?;
    }
    Ok(())
}

fn merge_yolo_policy(base: &YoloPolicy, overlay: &YoloPolicyOverride) -> YoloPolicy {
    YoloPolicy {
        allow_critical_blocking: overlay
            .allow_critical_blocking
            .unwrap_or(base.allow_critical_blocking),
        force_advisory_for_medium_risk: overlay
            .force_advisory_for_medium_risk
            .unwrap_or(base.force_advisory_for_medium_risk),
        collect_forensic_trail: overlay
            .collect_forensic_trail
            .unwrap_or(base.collect_forensic_trail),
    }
}

fn merge_rules_policy(base: &ProtectRules, overlay: &ProtectRulesOverride) -> ProtectRules {
    ProtectRules {
        blocked_command_patterns: overlay
            .blocked_command_patterns
            .clone()
            .unwrap_or_else(|| base.blocked_command_patterns.clone()),
        ask_command_patterns: overlay
            .ask_command_patterns
            .clone()
            .unwrap_or_else(|| base.ask_command_patterns.clone()),
        protected_paths: overlay
            .protected_paths
            .clone()
            .unwrap_or_else(|| base.protected_paths.clone()),
        secret_patterns: overlay
            .secret_patterns
            .clone()
            .unwrap_or_else(|| base.secret_patterns.clone()),
    }
}

fn merge_completion_policy(
    base: &CompletionPolicy,
    overlay: &CompletionPolicyOverride,
) -> CompletionPolicy {
    CompletionPolicy {
        enabled: overlay.enabled.unwrap_or(base.enabled),
        max_retries: overlay.max_retries.unwrap_or(base.max_retries),
        check_commands: overlay
            .check_commands
            .clone()
            .unwrap_or_else(|| base.check_commands.clone()),
        secret_scan: overlay.secret_scan.unwrap_or(base.secret_scan),
    }
}

fn merge_mcp_policy(base: &McpPolicy, overlay: &McpPolicyOverride) -> McpPolicy {
    McpPolicy {
        allowlist: overlay
            .allowlist
            .clone()
            .unwrap_or_else(|| base.allowlist.clone()),
        denylist: overlay
            .denylist
            .clone()
            .unwrap_or_else(|| base.denylist.clone()),
        redact_patterns: overlay
            .redact_patterns
            .clone()
            .unwrap_or_else(|| base.redact_patterns.clone()),
        block_instruction_payloads: overlay
            .block_instruction_payloads
            .unwrap_or(base.block_instruction_payloads),
    }
}

fn merge_subagent_policy(
    base: &SubagentPolicy,
    overlay: &SubagentPolicyOverride,
) -> SubagentPolicy {
    SubagentPolicy {
        enabled: overlay.enabled.unwrap_or(base.enabled),
        tighten_permissions: overlay
            .tighten_permissions
            .unwrap_or(base.tighten_permissions),
        default_profile: overlay.default_profile.unwrap_or(base.default_profile),
    }
}

fn merge_privilege_policy(
    base: &PrivilegePolicy,
    overlay: &PrivilegePolicyOverride,
) -> PrivilegePolicy {
    PrivilegePolicy {
        deny_when_root_without_sandbox: overlay
            .deny_when_root_without_sandbox
            .unwrap_or(base.deny_when_root_without_sandbox),
        require_ask_for_network_writes: overlay
            .require_ask_for_network_writes
            .unwrap_or(base.require_ask_for_network_writes),
        require_ask_for_broad_fs_writes: overlay
            .require_ask_for_broad_fs_writes
            .unwrap_or(base.require_ask_for_broad_fs_writes),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub payload_text: Option<String>,
    #[serde(default)]
    pub payload_json: Option<Value>,
    #[serde(default)]
    pub mcp_server_id: Option<String>,
    #[serde(default)]
    pub runtime_is_root: bool,
    #[serde(default)]
    pub runtime_has_sandbox: Option<bool>,
    #[serde(default)]
    pub runtime_bypass_mode: bool,
    #[serde(default)]
    pub network_write: bool,
    #[serde(default)]
    pub broad_fs_write: bool,
}

impl ProtectInput {
    /// Build a protect input from a normalized event metadata object.
    pub fn from_event_meta(
        provider: Provider,
        event: AgenticEvent,
        meta: &EventMeta,
    ) -> Option<ProtectInput> {
        let phase = match event {
            AgenticEvent::BeforePrompt => ProtectPhase::BeforePrompt,
            AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => ProtectPhase::BeforeTool,
            AgenticEvent::AfterTool | AgenticEvent::ToolError => ProtectPhase::AfterTool,
            AgenticEvent::TurnComplete => ProtectPhase::Completion,
            AgenticEvent::SubagentStart => ProtectPhase::SubagentStart,
            AgenticEvent::SubagentStop => ProtectPhase::SubagentStop,
            AgenticEvent::AfterModel => ProtectPhase::McpResponse,
            _ => return None,
        };

        let runtime_mode = detect_runtime_mode(meta);

        let command = meta
            .tool_input
            .as_ref()
            .and_then(extract_command_string)
            .or_else(|| meta.prompt.clone());

        let paths = collect_paths(meta);

        let network_write = command_implies_network_write(command.as_deref());
        let broad_fs_write = command_implies_broad_fs_write(command.as_deref());

        Some(ProtectInput {
            provider,
            phase,
            runtime_mode,
            risk: infer_risk(meta),
            summary: meta.notification_message.clone(),
            session_id: meta.session_id.clone(),
            tool_name: meta.tool_name.clone(),
            command,
            paths,
            prompt: meta.prompt.clone(),
            payload_text: meta
                .tool_response
                .as_ref()
                .map(Value::to_string)
                .or_else(|| meta.notification_message.clone()),
            payload_json: meta.tool_response.clone(),
            mcp_server_id: meta
                .extra
                .get("mcp_server_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            runtime_is_root: is_probably_root(meta),
            runtime_has_sandbox: infer_sandbox_state(meta),
            runtime_bypass_mode: runtime_mode == ProtectRuntimeMode::Yolo,
            network_write,
            broad_fs_write,
        })
    }
}

fn detect_runtime_mode(meta: &EventMeta) -> ProtectRuntimeMode {
    let keys = [
        "permission_mode",
        "approval_mode",
        "sandbox_mode",
        "execution_mode",
    ];

    for key in keys {
        if let Some(value) = meta.extra.get(key).and_then(Value::as_str) {
            let lowered = value.to_ascii_lowercase();
            if lowered.contains("yolo") || lowered.contains("bypass") || lowered.contains("danger")
            {
                return ProtectRuntimeMode::Yolo;
            }
        }
    }

    ProtectRuntimeMode::Normal
}

fn infer_risk(meta: &EventMeta) -> RiskLevel {
    if meta.error.is_some() {
        return RiskLevel::High;
    }

    let haystack = [
        meta.tool_name.as_deref(),
        meta.prompt.as_deref(),
        meta.notification_message.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
    .to_ascii_lowercase();

    if haystack.contains("rm -rf") || haystack.contains("drop database") {
        RiskLevel::Critical
    } else if haystack.contains("chmod") || haystack.contains("curl") {
        RiskLevel::High
    } else if haystack.contains("write") || haystack.contains("delete") {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

fn extract_command_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map
            .get("command")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        Value::Array(_) | Value::Bool(_) | Value::Null | Value::Number(_) => None,
    }
}

fn collect_paths(meta: &EventMeta) -> Vec<String> {
    let mut paths = Vec::new();

    if let Some(Value::Object(map)) = meta.tool_input.as_ref() {
        for key in ["path", "file", "target", "cwd"] {
            if let Some(path) = map.get(key).and_then(Value::as_str) {
                paths.push(path.to_string());
            }
        }
    }

    if let Some(cwd) = meta.cwd.as_ref() {
        paths.push(cwd.clone());
    }

    paths
}

fn is_probably_root(meta: &EventMeta) -> bool {
    meta.extra
        .get("uid")
        .and_then(Value::as_u64)
        .is_some_and(|uid| uid == 0)
        || meta
            .extra
            .get("is_root")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn infer_sandbox_state(meta: &EventMeta) -> Option<bool> {
    if let Some(value) = meta.extra.get("sandbox_enabled").and_then(Value::as_bool) {
        return Some(value);
    }

    meta.extra
        .get("sandbox_mode")
        .and_then(Value::as_str)
        .map(|mode| {
            let lowered = mode.to_ascii_lowercase();
            !(lowered.contains("none") || lowered.contains("off") || lowered.contains("danger"))
        })
}

fn command_implies_network_write(command: Option<&str>) -> bool {
    let Some(command) = command else {
        return false;
    };

    let lowered = command.to_ascii_lowercase();
    [
        "curl -x",
        "curl -d",
        "wget --post",
        "scp ",
        "rsync ",
        "git push",
    ]
    .iter()
    .any(|pattern| lowered.contains(pattern))
}

fn command_implies_broad_fs_write(command: Option<&str>) -> bool {
    let Some(command) = command else {
        return false;
    };

    let lowered = command.to_ascii_lowercase();
    ["rm -rf /", "chmod -r", "chown -r", "find / -delete"]
        .iter()
        .any(|pattern| lowered.contains(pattern))
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
    pub yolo: Option<YoloPolicyOverride>,
    #[serde(default)]
    pub rules: Option<ProtectRulesOverride>,
    #[serde(default)]
    pub completion: Option<CompletionPolicyOverride>,
    #[serde(default)]
    pub mcp: Option<McpPolicyOverride>,
    #[serde(default)]
    pub subagents: Option<SubagentPolicyOverride>,
    #[serde(default)]
    pub privilege: Option<PrivilegePolicyOverride>,
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

/// Partial provider override for [`YoloPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YoloPolicyOverride {
    #[serde(default)]
    pub allow_critical_blocking: Option<bool>,
    #[serde(default)]
    pub force_advisory_for_medium_risk: Option<bool>,
    #[serde(default)]
    pub collect_forensic_trail: Option<bool>,
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

/// Partial provider override for [`ProtectRules`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectRulesOverride {
    #[serde(default)]
    pub blocked_command_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub ask_command_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub protected_paths: Option<Vec<String>>,
    #[serde(default)]
    pub secret_patterns: Option<Vec<String>>,
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

/// Partial provider override for [`CompletionPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionPolicyOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub max_retries: Option<u8>,
    #[serde(default)]
    pub check_commands: Option<Vec<String>>,
    #[serde(default)]
    pub secret_scan: Option<bool>,
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

/// Partial provider override for [`McpPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpPolicyOverride {
    #[serde(default)]
    pub allowlist: Option<Vec<String>>,
    #[serde(default)]
    pub denylist: Option<Vec<String>>,
    #[serde(default)]
    pub redact_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub block_instruction_payloads: Option<bool>,
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

/// Partial provider override for [`SubagentPolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubagentPolicyOverride {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub tighten_permissions: Option<bool>,
    #[serde(default)]
    pub default_profile: Option<SubagentProfile>,
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

/// Partial provider override for [`PrivilegePolicy`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivilegePolicyOverride {
    #[serde(default)]
    pub deny_when_root_without_sandbox: Option<bool>,
    #[serde(default)]
    pub require_ask_for_network_writes: Option<bool>,
    #[serde(default)]
    pub require_ask_for_broad_fs_writes: Option<bool>,
}

fn default_true_value() -> bool {
    true
}

/// Redaction result for MCP text payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpTextRedaction {
    pub text: String,
    pub redacted: bool,
    pub blocked_instruction_payload: bool,
    pub redactions_applied: u32,
}

/// Redaction result for MCP JSON payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpJsonRedaction {
    pub value: Value,
    pub redacted: bool,
    pub blocked_instruction_payload: bool,
    pub redactions_applied: u32,
}

/// In-memory rolling state retained by the protect evaluator.
#[derive(Debug, Clone, Default)]
pub struct ProtectState {
    /// Total decisions evaluated by this service instance.
    pub decision_count: u64,
    /// Bounded rolling records for post-run audits.
    pub recent: VecDeque<ProtectDecisionRecord>,
    /// Per-session completion retries used for loop protection.
    pub completion_retries_by_session: HashMap<String, u8>,
}

impl ProtectState {
    fn record(&mut self, input: &ProtectInput, decision: &ProtectDecision) {
        self.decision_count += 1;

        let completion_retry_count = if input.phase == ProtectPhase::Completion {
            Some(
                self.completion_retries_by_session
                    .get(input.session_id.as_deref().unwrap_or(GLOBAL_SESSION_KEY))
                    .copied()
                    .unwrap_or(0),
            )
        } else {
            None
        };

        self.recent.push_back(ProtectDecisionRecord {
            provider: input.provider,
            phase: input.phase,
            mode: input.runtime_mode,
            risk: input.risk,
            outcome: decision.outcome.clone(),
            degraded: decision.degraded,
            degraded_from: decision.degraded_from.clone(),
            reason: decision.reason.clone(),
            session_id: input.session_id.clone(),
            completion_retry_count,
        });
    }

    /// Snapshot decision records as an owned list.
    pub fn snapshot_records(&self) -> Vec<ProtectDecisionRecord> {
        self.recent.iter().cloned().collect()
    }

    /// Export full protect state for telemetry/reporting.
    pub fn export_state(&self) -> ProtectStateExport {
        ProtectStateExport {
            decision_count: self.decision_count,
            records: self.snapshot_records(),
            completion_retries_by_session: self.completion_retries_by_session.clone(),
        }
    }

    /// Export records as JSON Lines.
    pub fn export_records_jsonl(&self) -> Result<String> {
        let mut out = String::new();
        for record in &self.recent {
            out.push_str(&serde_json::to_string(record)?);
            out.push('\n');
        }
        Ok(out)
    }
}

/// Export shape for protect state snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectStateExport {
    pub decision_count: u64,
    pub records: Vec<ProtectDecisionRecord>,
    pub completion_retries_by_session: HashMap<String, u8>,
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
    #[serde(default)]
    pub degraded_from: Option<ProtectOutcome>,
    pub reason: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub completion_retry_count: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix_input(
        provider: Provider,
        risk: RiskLevel,
        posture: ProtectPosture,
        mode: ProtectRuntimeMode,
    ) -> ProtectDecision {
        let mut service = ProtectService::new(ProtectConfig {
            posture,
            ..ProtectConfig::default()
        });

        service.evaluate(&ProtectInput {
            provider,
            phase: ProtectPhase::BeforeTool,
            runtime_mode: mode,
            risk,
            summary: Some("test".to_string()),
            session_id: Some("session-1".to_string()),
            tool_name: Some("Bash".to_string()),
            command: Some("echo test".to_string()),
            paths: vec!["/tmp/test".to_string()],
            prompt: None,
            payload_text: None,
            payload_json: None,
            mcp_server_id: None,
            runtime_is_root: false,
            runtime_has_sandbox: Some(true),
            runtime_bypass_mode: mode == ProtectRuntimeMode::Yolo,
            network_write: false,
            broad_fs_write: false,
        })
    }

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
    fn risk_posture_mode_matrix_is_deterministic() {
        for posture in [
            ProtectPosture::Advisory,
            ProtectPosture::Balanced,
            ProtectPosture::Strict,
        ] {
            for mode in [ProtectRuntimeMode::Normal, ProtectRuntimeMode::Yolo] {
                for risk in [
                    RiskLevel::Low,
                    RiskLevel::Medium,
                    RiskLevel::High,
                    RiskLevel::Critical,
                ] {
                    let decision = matrix_input(Provider::Claude, risk, posture, mode);
                    assert!(!decision.reason.is_empty());
                }
            }
        }
    }

    #[test]
    fn critical_risk_degrades_when_provider_cannot_block() {
        let decision = matrix_input(
            Provider::Codex,
            RiskLevel::Critical,
            ProtectPosture::Strict,
            ProtectRuntimeMode::Normal,
        );

        assert!(decision.degraded);
        assert!(matches!(
            decision.outcome,
            ProtectOutcome::AdvisoryOnly { .. }
        ));
    }

    #[test]
    fn yolo_medium_risk_is_forced_to_advisory_by_default() {
        let decision = matrix_input(
            Provider::Claude,
            RiskLevel::Medium,
            ProtectPosture::Balanced,
            ProtectRuntimeMode::Yolo,
        );

        assert!(decision.degraded);
        assert!(matches!(
            decision.outcome,
            ProtectOutcome::AdvisoryOnly { .. }
        ));
    }

    #[test]
    fn rules_conflict_prefers_deny_over_ask() {
        let mut service = ProtectService::new(ProtectConfig {
            posture: ProtectPosture::Balanced,
            rules: ProtectRules {
                blocked_command_patterns: vec!["rm -rf".to_string()],
                ask_command_patterns: vec!["rm".to_string()],
                protected_paths: vec![],
                secret_patterns: vec![],
            },
            ..ProtectConfig::default()
        });

        let decision = service.evaluate(&ProtectInput {
            provider: Provider::Claude,
            phase: ProtectPhase::BeforeTool,
            runtime_mode: ProtectRuntimeMode::Normal,
            risk: RiskLevel::Low,
            summary: Some("run command".to_string()),
            session_id: Some("s1".to_string()),
            tool_name: Some("Bash".to_string()),
            command: Some("rm -rf /tmp/foo".to_string()),
            paths: vec![],
            prompt: None,
            payload_text: None,
            payload_json: None,
            mcp_server_id: None,
            runtime_is_root: false,
            runtime_has_sandbox: Some(true),
            runtime_bypass_mode: false,
            network_write: false,
            broad_fs_write: false,
        });

        assert!(matches!(
            decision.outcome,
            ProtectOutcome::StopCurrent { .. }
        ));
    }

    #[test]
    fn provider_override_can_disable_protect() {
        let mut overrides = HashMap::new();
        overrides.insert(
            Provider::Claude,
            ProviderProtectOverride {
                enabled: Some(false),
                ..ProviderProtectOverride::default()
            },
        );

        let mut service = ProtectService::new(ProtectConfig {
            providers: overrides,
            ..ProtectConfig::default()
        });

        let decision = service.evaluate(&ProtectInput {
            provider: Provider::Claude,
            phase: ProtectPhase::BeforeTool,
            runtime_mode: ProtectRuntimeMode::Normal,
            risk: RiskLevel::Critical,
            summary: None,
            session_id: None,
            tool_name: None,
            command: None,
            paths: vec![],
            prompt: None,
            payload_text: None,
            payload_json: None,
            mcp_server_id: None,
            runtime_is_root: false,
            runtime_has_sandbox: Some(true),
            runtime_bypass_mode: false,
            network_write: false,
            broad_fs_write: false,
        });

        assert!(matches!(decision.outcome, ProtectOutcome::Allow));
        assert_eq!(decision.reason, "protect.disabled");
    }

    #[test]
    fn completion_retry_limit_triggers_loop_protection() {
        let mut service = ProtectService::new(ProtectConfig {
            completion: CompletionPolicy {
                max_retries: 1,
                ..CompletionPolicy::default()
            },
            posture: ProtectPosture::Strict,
            ..ProtectConfig::default()
        });

        let input = ProtectInput {
            provider: Provider::Claude,
            phase: ProtectPhase::Completion,
            runtime_mode: ProtectRuntimeMode::Normal,
            risk: RiskLevel::Critical,
            summary: None,
            session_id: Some("c1".to_string()),
            tool_name: None,
            command: None,
            paths: vec![],
            prompt: None,
            payload_text: None,
            payload_json: None,
            mcp_server_id: None,
            runtime_is_root: false,
            runtime_has_sandbox: Some(true),
            runtime_bypass_mode: false,
            network_write: false,
            broad_fs_write: false,
        };

        let first = service.evaluate(&input);
        assert!(matches!(first.outcome, ProtectOutcome::StopSession { .. }));

        let second = service.evaluate(&input);
        assert!(matches!(second.outcome, ProtectOutcome::StopSession { .. }));
        assert!(second.degraded);
        assert!(
            service
                .state()
                .completion_retries_by_session
                .contains_key("c1")
        );
    }

    #[test]
    fn mcp_redaction_helpers_redact_secret_patterns() {
        let config = ProtectConfig {
            rules: ProtectRules {
                secret_patterns: vec!["sk-[a-z0-9]+".to_string()],
                ..ProtectRules::default()
            },
            mcp: McpPolicy {
                redact_patterns: vec!["token=[a-z0-9]+".to_string()],
                ..McpPolicy::default()
            },
            ..ProtectConfig::default()
        };

        let service = ProtectService::new(config);

        let text = service
            .redact_mcp_text(Provider::Claude, "token=abc123 and sk-secret")
            .unwrap();
        assert!(text.redacted);
        assert!(text.text.contains("[REDACTED]"));

        let json = service
            .redact_mcp_json(
                Provider::Claude,
                &serde_json::json!({"message":"token=abc123", "secret":"sk-secret"}),
            )
            .unwrap();
        assert!(json.redacted);
        assert_eq!(json.value["message"], "[REDACTED]");
    }

    #[test]
    fn audit_export_contains_recent_records() {
        let mut service = ProtectService::new(ProtectConfig::default());
        let _ = service.evaluate(&ProtectInput {
            provider: Provider::Claude,
            phase: ProtectPhase::BeforeTool,
            runtime_mode: ProtectRuntimeMode::Normal,
            risk: RiskLevel::Low,
            summary: Some("test".to_string()),
            session_id: Some("s1".to_string()),
            tool_name: Some("Bash".to_string()),
            command: Some("echo ok".to_string()),
            paths: vec![],
            prompt: None,
            payload_text: None,
            payload_json: None,
            mcp_server_id: None,
            runtime_is_root: false,
            runtime_has_sandbox: Some(true),
            runtime_bypass_mode: false,
            network_write: false,
            broad_fs_write: false,
        });

        let snapshot = service.export_state();
        assert_eq!(snapshot.decision_count, 1);
        assert_eq!(snapshot.records.len(), 1);

        let jsonl = service.export_records_jsonl().unwrap();
        assert!(jsonl.contains("before_tool"));
    }

    #[test]
    fn validate_rejects_bad_regex() {
        let config = ProtectConfig {
            rules: ProtectRules {
                blocked_command_patterns: vec!["[unterminated".to_string()],
                ..ProtectRules::default()
            },
            ..ProtectConfig::default()
        };

        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("protect rule parse error"));
    }

    #[test]
    fn provider_aware_defaults_soften_low_control_providers() {
        let config = ProtectConfig::provider_aware_defaults(
            &[Provider::Claude, Provider::Codex, Provider::Goose],
            ProtectPosture::Balanced,
        );

        assert_eq!(config.posture, ProtectPosture::Balanced);
        assert_eq!(
            config.providers[&Provider::Codex].posture,
            Some(ProtectPosture::Advisory)
        );
        assert_eq!(
            config.providers[&Provider::Goose].posture,
            Some(ProtectPosture::Advisory)
        );
    }
}
