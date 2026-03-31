#![allow(deprecated)] // ProtectInput is deprecated but still used in the legacy evaluation path.

use super::config::ProtectPhase;
use super::decision::{GateCapability, ProviderProtectCapabilities, VisibilityLevel};
use super::evaluate::ProtectInput;
use super::ProtectOutcome;

pub(crate) fn downgrade_for_capability(
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
        ProtectPhase::Runtime | ProtectPhase::AfterTool => GateCapability::Influence,
    }
}
