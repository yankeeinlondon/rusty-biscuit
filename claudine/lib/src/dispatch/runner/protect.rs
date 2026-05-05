use crate::actions::{HookDecision, HookResponse};
use crate::protect::decision::ProtectDecision;

pub(super) fn should_short_circuit_call(decision: Option<&ProtectDecision>) -> bool {
    decision.is_some_and(|d| d.is_blocked())
}

pub(super) fn decision_for_short_circuit(_decision: &ProtectDecision) -> HookDecision {
    HookDecision::Deny
}

pub(super) fn attach_protect_context(
    response: HookResponse,
    _protect_decision: Option<&ProtectDecision>,
) -> HookResponse {
    // Simplified: new protect doesn't attach context to responses
    response
}
