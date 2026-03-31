#![allow(deprecated)] // ProtectInput is deprecated but still the active evaluation path.

use std::sync::Arc;

use serde_json::Value;

use crate::adapters::ProviderAdapter;
use crate::error::Result;
use crate::events::{AgenticEvent, EventMeta, Provider};
use crate::permissions::PolicyEngine;

use super::config::{ProtectConfig, ProtectPhase, ProtectRuntimeMode, RiskLevel};
use super::decision::{ProtectDecision, ProtectEvaluation, ProtectOutcome, ProviderProtectProfiles};
use super::downgrade::{capability_for_phase, downgrade_for_capability};
#[allow(deprecated)]
use super::evaluate::{desired_outcome, resolve_snapshot, ProtectInput};
use super::redact::{
    redact_json_with_policy, redact_text_with_policy, McpJsonRedaction, McpTextRedaction,
};
use super::request::{ProtectRequest, ProtectSessionContext};
use super::state::{ProtectDecisionRecord, ProtectState, ProtectStateExport, GLOBAL_SESSION_KEY};

/// Central policy actor for Protect decisions.
///
/// The service is capability-aware: it computes a normalized decision first,
/// then downgrades when a provider cannot enforce that decision natively.
#[derive(Debug, Clone)]
pub struct ProtectService {
    engine: Arc<PolicyEngine>,
    config: ProtectConfig,
    profiles: ProviderProtectProfiles,
    state: ProtectState,
}

impl ProtectService {
    /// Build a Protect service with a policy engine and default profiles.
    pub fn new(engine: Arc<PolicyEngine>, config: ProtectConfig) -> Self {
        Self {
            engine,
            config,
            profiles: ProviderProtectProfiles::defaults(),
            state: ProtectState::default(),
        }
    }

    /// Build a Protect service with explicit provider capability profiles.
    pub fn with_profiles(
        engine: Arc<PolicyEngine>,
        config: ProtectConfig,
        profiles: ProviderProtectProfiles,
    ) -> Self {
        Self {
            engine,
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

    /// Return a reference to the policy engine.
    pub fn engine(&self) -> &PolicyEngine {
        &self.engine
    }

    /// Full structured evaluation from a pre-built request.
    ///
    /// Delegates to the legacy evaluation path internally in this phase.
    /// Phase 4 will replace the internals with policy-backed evaluation.
    pub fn evaluate_structured(
        &mut self,
        request: &ProtectRequest,
    ) -> Result<ProtectEvaluation> {
        // Resolve snapshot (called for side effects / caching; results not yet
        // used for decisions until Phase 4).
        let (_snapshot, policy_mode) =
            resolve_snapshot(&self.engine, &request.session);

        // Phase 4 will use the request's observation/intents directly.
        // For now, return a no-op evaluation since we can't translate
        // ProtectRequest back to ProtectInput without EventMeta.
        let decision = ProtectDecision::allow("protect.structured-stub");

        Ok(ProtectEvaluation {
            decision,
            policy_mode,
            findings: Vec::new(),
            redaction: None,
            warnings: Vec::new(),
        })
    }

    /// Convenience entry point for dispatch.
    pub fn evaluate_event_structured(
        &mut self,
        provider: Provider,
        event: AgenticEvent,
        meta: &EventMeta,
        ctx: &ProtectSessionContext,
        _adapter: &dyn ProviderAdapter,
    ) -> Result<Option<ProtectEvaluation>> {
        let input = ProtectInput::from_event_meta(provider, event, meta);
        let Some(input) = input else {
            return Ok(None);
        };

        let (_snapshot, policy_mode) = resolve_snapshot(&self.engine, ctx);

        let decision = self.evaluate_legacy(&input);

        Ok(Some(ProtectEvaluation {
            decision,
            policy_mode,
            findings: Vec::new(),
            redaction: None,
            warnings: Vec::new(),
        }))
    }

    /// Legacy evaluation path — will be replaced in Phase 4.
    pub fn evaluate_legacy(&mut self, input: &ProtectInput) -> ProtectDecision {
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

    /// Evaluate one protection input and return a normalized decision.
    ///
    /// Delegates to [`evaluate_legacy`] for now. Phase 4 will replace the
    /// internals with policy-backed evaluation.
    pub fn evaluate(&mut self, input: &ProtectInput) -> ProtectDecision {
        self.evaluate_legacy(input)
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
