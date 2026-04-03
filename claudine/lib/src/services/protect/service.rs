use std::sync::Arc;

use serde_json::Value;

use crate::adapters::ProviderAdapter;
use crate::error::Result;
use crate::events::{AgenticEvent, EventMeta, Provider};
use crate::permissions::PolicyEngine;

use super::config::{ProtectConfig, ProtectPhase};
use super::decision::{
    ProtectDecision, ProtectEvaluation, ProtectOutcome, ProviderProtectCapabilities,
    ProviderProtectProfiles,
};
use super::downgrade::capability_for_phase;
use super::evaluate::evaluate_request;
use super::redact::{
    McpJsonRedaction, McpTextRedaction, redact_json_with_policy, redact_text_with_policy,
};
use super::request::{ProtectRequest, ProtectSessionContext};
use super::state::{GLOBAL_SESSION_KEY, ProtectDecisionRecord, ProtectState, ProtectStateExport};

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
    last_evaluation: Option<ProtectEvaluation>,
}

impl ProtectService {
    /// Build a Protect service with a policy engine and default profiles.
    pub fn new(engine: Arc<PolicyEngine>, config: ProtectConfig) -> Self {
        Self {
            engine,
            config,
            profiles: ProviderProtectProfiles::defaults(),
            state: ProtectState::default(),
            last_evaluation: None,
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
            last_evaluation: None,
        }
    }

    /// Build a Protect service with a single provider's capabilities.
    ///
    /// Preferred over `with_profiles` when only one provider is active,
    /// which is the common dispatch case. Avoids maintaining a parallel
    /// capabilities map.
    pub fn with_capabilities(
        engine: Arc<PolicyEngine>,
        config: ProtectConfig,
        provider: Provider,
        capabilities: ProviderProtectCapabilities,
    ) -> Self {
        let mut profiles = ProviderProtectProfiles::defaults();
        profiles.insert(provider, capabilities);
        Self {
            engine,
            config,
            profiles,
            state: ProtectState::default(),
            last_evaluation: None,
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
    pub fn evaluate_structured(&mut self, request: &ProtectRequest) -> Result<ProtectEvaluation> {
        let policy = self.resolve_policy_for_provider(request.provider);

        // Fast path: protect disabled for this provider
        if !policy.enabled {
            let eval = ProtectEvaluation {
                decision: ProtectDecision::allow("protect.disabled"),
                policy_mode: super::decision::ProtectPolicyMode::ConfiguredFallback,
                findings: Vec::new(),
                redaction: None,
                warnings: Vec::new(),
            };
            return Ok(eval);
        }

        let mut eval = evaluate_request(&self.engine, &policy, request)?;

        // Apply capability downgrade
        let capabilities = self.profiles.capabilities(request.provider);
        let gate = capability_for_phase(request.phase, &capabilities);
        if let Some(degraded) = downgrade_outcome_for_capability(
            &eval.decision.desired_outcome,
            request.phase,
            capabilities,
        ) {
            eval.decision = ProtectDecision::degraded(
                degraded,
                eval.decision.desired_outcome.clone(),
                eval.decision.reason.clone(),
            );
            eval.decision.capability = Some(gate);
        } else {
            eval.decision.capability = Some(gate);
        }

        // Apply completion retry policy
        eval = self.apply_completion_retry_policy_structured(request, &policy, eval);

        // Record the evaluation
        self.state.record_evaluation(
            request.provider,
            request.phase,
            &eval,
            request.session.session_id.as_deref(),
        );

        // Bound rolling records
        while self.state.recent.len() > policy.max_recent_decisions as usize {
            self.state.recent.pop_front();
        }

        // Cache for explain_last()
        self.last_evaluation = Some(eval.clone());

        Ok(eval)
    }

    /// Convenience entry point for dispatch: observe + evaluate.
    pub fn evaluate_event_structured(
        &mut self,
        provider: Provider,
        event: AgenticEvent,
        _meta: &EventMeta,
        ctx: &ProtectSessionContext,
        adapter: &dyn ProviderAdapter,
    ) -> Result<Option<ProtectEvaluation>> {
        let observation = adapter.observe_protect(&event, _meta);
        let Some(obs) = observation else {
            return Ok(None);
        };

        let phase = phase_from_event(event);
        let request = ProtectRequest {
            provider,
            event,
            phase,
            session: ctx.clone(),
            observation: obs,
        };

        self.evaluate_structured(&request).map(Some)
    }

    /// Redact MCP text payload using provider-effective policy.
    ///
    /// **Deprecated:** Redaction is now part of `ProtectEvaluation.redaction`.
    /// This method remains for backward compatibility.
    #[deprecated(note = "Redaction is now part of ProtectEvaluation.redaction")]
    pub fn redact_mcp_text(&self, provider: Provider, text: &str) -> Result<McpTextRedaction> {
        let policy = self.resolve_policy_for_provider(provider);
        redact_text_with_policy(text, &policy.mcp, &policy.rules.secret_patterns)
    }

    /// Redact MCP JSON payload using provider-effective policy.
    ///
    /// **Deprecated:** Redaction is now part of `ProtectEvaluation.redaction`.
    /// This method remains for backward compatibility.
    #[deprecated(note = "Redaction is now part of ProtectEvaluation.redaction")]
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

    /// Render an explanation for the most recently evaluated request.
    ///
    /// Returns `None` if no evaluations have been performed yet, or if
    /// the last evaluation has no cached data. This is a convenience method;
    /// callers can also call [`explain::explain_evaluation`] directly on
    /// a `ProtectEvaluation`.
    pub fn explain_last(&self) -> Option<super::explain::ProtectExplanation> {
        self.last_evaluation
            .as_ref()
            .map(super::explain::explain_evaluation)
    }

    fn apply_completion_retry_policy_structured(
        &mut self,
        request: &ProtectRequest,
        policy: &ProtectConfig,
        mut eval: ProtectEvaluation,
    ) -> ProtectEvaluation {
        if request.phase != ProtectPhase::Completion || !policy.completion.enabled {
            return eval;
        }

        let session_key = request
            .session
            .session_id
            .clone()
            .unwrap_or_else(|| GLOBAL_SESSION_KEY.to_string());

        if matches!(
            eval.decision.outcome,
            ProtectOutcome::Allow | ProtectOutcome::AllowWithRedaction { .. }
        ) {
            self.state
                .completion_retries_by_session
                .remove(&session_key);
            return eval;
        }

        let retry_count = self
            .state
            .completion_retries_by_session
            .entry(session_key)
            .or_default();
        *retry_count = retry_count.saturating_add(1);

        if *retry_count > policy.completion.max_retries {
            eval.decision = ProtectDecision::degraded(
                ProtectOutcome::StopSession {
                    reason: "completion.loop-protection.max-retries".to_string(),
                },
                eval.decision.desired_outcome.clone(),
                "protect.completion.loop-protection".to_string(),
            );
        }

        eval
    }
}

fn downgrade_outcome_for_capability(
    desired: &ProtectOutcome,
    phase: ProtectPhase,
    capabilities: ProviderProtectCapabilities,
) -> Option<ProtectOutcome> {
    let gate = capability_for_phase(phase, &capabilities);

    match desired {
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

fn phase_from_event(event: AgenticEvent) -> ProtectPhase {
    match event {
        AgenticEvent::BeforePrompt => ProtectPhase::BeforePrompt,
        AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => ProtectPhase::BeforeTool,
        AgenticEvent::AfterTool | AgenticEvent::ToolError => ProtectPhase::AfterTool,
        AgenticEvent::TurnComplete => ProtectPhase::Completion,
        AgenticEvent::SubagentStart => ProtectPhase::SubagentStart,
        AgenticEvent::SubagentStop => ProtectPhase::SubagentStop,
        AgenticEvent::AfterModel => ProtectPhase::McpResponse,
        _ => ProtectPhase::Runtime,
    }
}
