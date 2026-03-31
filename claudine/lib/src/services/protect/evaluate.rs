#![allow(deprecated)] // ProtectInput is deprecated but still the active evaluation path.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{ClaudineError, Result};
use crate::events::{AgenticEvent, EventMeta, Provider};
use crate::permissions::{CliPolicyInput, PolicyEngine};

use super::config::{
    McpPolicy, PrivilegePolicy, ProtectConfig, ProtectPhase, ProtectPosture, ProtectRules,
    ProtectRuntimeMode, RiskLevel,
};
use super::decision::{ProtectOutcome, ProtectPolicyMode};
use super::redact::{contains_instruction_payload, redact_json_with_policy, redact_text_with_policy};
use super::request::{ProtectCliContext, ProtectSessionContext};

/// Input envelope for evaluating one potential protection decision.
///
/// **Deprecated:** Use `ProtectRequest` with `ProtectObservation` instead.
/// This type will be removed after Phase 4 completes.
#[deprecated(note = "Use ProtectRequest with ProtectObservation instead")]
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

// --- Snapshot resolution ---

/// Resolve a policy snapshot from the engine, returning the policy mode
/// that indicates whether we got effective or configured-fallback policy.
///
/// Results are not yet used for decisions in this phase. Phase 4 will wire
/// snapshot query results into the evaluation pipeline.
pub(crate) fn resolve_snapshot(
    engine: &PolicyEngine,
    session: &ProtectSessionContext,
) -> (Option<()>, ProtectPolicyMode) {
    let provider = session.provider;
    let ctx = &session.policy_context;

    match &session.cli {
        ProtectCliContext::Argv(args) => {
            match engine.effective(provider, ctx, CliPolicyInput::Argv(args)) {
                Ok(_snapshot) => {
                    debug!(%provider, "Resolved effective policy snapshot from argv");
                    (Some(()), ProtectPolicyMode::Effective)
                }
                Err(err) => {
                    debug!(%provider, %err, "Failed to resolve effective snapshot, using configured fallback");
                    try_configured_fallback(engine, provider, ctx)
                }
            }
        }
        ProtectCliContext::Parsed(overrides) => {
            match engine.effective(provider, ctx, CliPolicyInput::Parsed(overrides.as_ref())) {
                Ok(_snapshot) => {
                    debug!(%provider, "Resolved effective policy snapshot from parsed overrides");
                    (Some(()), ProtectPolicyMode::Effective)
                }
                Err(err) => {
                    debug!(%provider, %err, "Failed to resolve effective snapshot, using configured fallback");
                    try_configured_fallback(engine, provider, ctx)
                }
            }
        }
        ProtectCliContext::None => try_configured_fallback(engine, provider, ctx),
    }
}

fn try_configured_fallback(
    engine: &PolicyEngine,
    provider: Provider,
    ctx: &crate::permissions::PolicyContext,
) -> (Option<()>, ProtectPolicyMode) {
    match engine.configured(provider, ctx) {
        Ok(_snapshot) => {
            debug!(%provider, "Resolved configured policy snapshot (fallback)");
            (Some(()), ProtectPolicyMode::ConfiguredFallback)
        }
        Err(err) => {
            debug!(%provider, %err, "Failed to resolve any policy snapshot");
            (None, ProtectPolicyMode::ConfiguredFallback)
        }
    }
}

// --- Evaluation pipeline ---

#[derive(Debug, Clone)]
pub(crate) struct DesiredDecision {
    pub outcome: ProtectOutcome,
}

impl DesiredDecision {
    pub fn reason_code(&self, mode: ProtectRuntimeMode, posture: ProtectPosture) -> String {
        format!("protect.{}.{posture}", mode.as_str())
    }
}

pub(crate) fn desired_outcome(
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

pub(crate) fn select_precedence(mut outcomes: Vec<ProtectOutcome>) -> ProtectOutcome {
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

pub(crate) fn stop_outcome_for_posture(posture: ProtectPosture, reason: &str) -> ProtectOutcome {
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
