pub mod config;
pub mod decision;
mod downgrade;
pub mod evaluate;
mod explain;
pub mod intent;
pub mod observe;
pub mod redact;
pub mod request;
pub mod service;
pub mod state;

// Re-exports for public API surface
pub use config::{
    CompletionPolicy, CompletionPolicyOverride, McpPolicy, McpPolicyOverride, PrivilegePolicy,
    PrivilegePolicyOverride, ProtectConfig, ProtectPhase, ProtectPosture, ProtectRules,
    ProtectRulesOverride, ProtectRuntimeMode, ProviderProtectOverride, RiskLevel, SubagentPolicy,
    SubagentPolicyOverride, YoloPolicy, YoloPolicyOverride,
};

pub use decision::{
    GateCapability, ProtectDecision, ProtectEvaluation, ProtectOutcome, ProtectPolicyMode,
    ProviderProtectCapabilities, ProviderProtectProfiles, VisibilityLevel,
};

#[allow(deprecated)]
pub use evaluate::ProtectInput;

pub use intent::ProtectIntent;

pub use observe::{ProtectObservation, ProtectPayload, RuntimeFacts};

pub use redact::{McpJsonRedaction, McpTextRedaction};

pub use request::{ProtectCliContext, ProtectRequest, ProtectSessionContext};

pub use service::ProtectService;

pub use state::{ProtectDecisionRecord, ProtectState, ProtectStateExport};

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::events::Provider;
    use crate::permissions::PolicyEngine;

    use super::config::*;
    use super::decision::*;
    use super::evaluate::*;
    use super::redact::*;
    use super::service::*;
    use super::state::*;

    fn test_engine() -> Arc<PolicyEngine> {
        Arc::new(PolicyEngine::new())
    }

    fn matrix_input(
        provider: Provider,
        risk: RiskLevel,
        posture: ProtectPosture,
        mode: ProtectRuntimeMode,
    ) -> ProtectDecision {
        let mut service = ProtectService::new(test_engine(), ProtectConfig {
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
        let mut service = ProtectService::new(test_engine(), ProtectConfig {
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

        let mut service = ProtectService::new(test_engine(), ProtectConfig {
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
        let mut service = ProtectService::new(test_engine(), ProtectConfig {
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

        let service = ProtectService::new(test_engine(), config);

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
        let mut service = ProtectService::new(test_engine(), ProtectConfig::default());
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
