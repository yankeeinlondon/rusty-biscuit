use super::ProtectOutcome;
use super::config::ProtectPhase;
use super::decision::{GateCapability, ProviderProtectCapabilities, VisibilityLevel};

#[allow(dead_code)]
pub(crate) fn downgrade_for_capability(
    outcome: ProtectOutcome,
    phase: ProtectPhase,
    capability: ProviderProtectCapabilities,
) -> Option<ProtectOutcome> {
    let gate = capability_for_phase(phase, &capability);

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

pub(crate) fn capability_for_phase(
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
        ProtectPhase::AfterTool => capabilities.post_tool_gate,
        ProtectPhase::Runtime => GateCapability::Influence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subagent_visibility_maps_to_gate_capability() {
        let partial = ProviderProtectCapabilities {
            subagent_visibility: VisibilityLevel::Partial,
            ..ProviderProtectCapabilities::default()
        };
        let full = ProviderProtectCapabilities {
            subagent_visibility: VisibilityLevel::Full,
            ..ProviderProtectCapabilities::default()
        };

        assert_eq!(
            capability_for_phase(ProtectPhase::SubagentStart, &partial),
            GateCapability::Influence
        );
        assert_eq!(
            capability_for_phase(ProtectPhase::SubagentStop, &full),
            GateCapability::Guarantee
        );
    }

    #[test]
    fn stop_session_downgrades_when_capability_cannot_stop_session() {
        let downgraded = downgrade_for_capability(
            ProtectOutcome::StopSession {
                reason: "policy.deny".to_string(),
            },
            ProtectPhase::BeforeTool,
            ProviderProtectCapabilities::default(),
        );

        assert!(matches!(
            downgraded,
            Some(ProtectOutcome::AdvisoryOnly { reason }) if reason == "capability.no-stop-session"
        ));
    }
}
