use tracing::warn;

use crate::actions::{HookDecision, HookResponse};
use crate::protect::catalog::{RuleGroup, ScanSurface};
use crate::protect::decision::{ProtectDecision, ProtectMatch};
use crate::protect::observe::ProtectObservation;
use crate::protect::report::format_blocked_message;

pub(super) fn map_protect_block(decision: &ProtectDecision) -> HookResponse {
    let reason = decision
        .blocked
        .as_ref()
        .map(format_blocked_message)
        .unwrap_or_else(|| "protect: blocked".to_string());

    HookResponse {
        decision: Some(HookDecision::Deny),
        reason: Some(reason),
        updated_input: None,
        additional_context: None,
        raw: Some(serde_json::json!({
            "protect": {
                "outcome": "block",
                "group": decision.blocked.as_ref().map(|m| m.group.config_key()),
                "rule_id": decision.blocked.as_ref().map(|m| &m.rule_id),
            }
        })),
    }
}

/// Evaluate a protect observation. Normal requests flow through the service.
/// `Unparsed` command- or write-shaped observations are blocked defensively
/// with a warning, reflecting the best-effort posture.
pub(super) fn evaluate_protect_observation(
    service: &crate::protect::service::ProtectService,
    observation: ProtectObservation<'_>,
    tool_name: &str,
) -> Option<ProtectDecision> {
    match observation {
        ProtectObservation::Request(request) => {
            let decision = service.evaluate(&request);
            if decision.is_blocked() {
                Some(decision)
            } else {
                None
            }
        }
        ProtectObservation::Unparsed { surface, reason } => {
            warn!(
                tool_name,
                ?surface,
                reason,
                "protect could not parse command/write-shaped tool; blocking defensively"
            );
            Some(synthetic_unparsed_block(surface, reason))
        }
        ProtectObservation::NoOpinion => None,
    }
}

fn synthetic_unparsed_block(surface: ScanSurface, reason: &'static str) -> ProtectDecision {
    let rule_id = match surface {
        ScanSurface::BashCommand => "unparsed_bash_command",
        ScanSurface::WritePath => "unparsed_write_path",
        ScanSurface::McpResponse => "unparsed_mcp_response",
    };
    ProtectDecision::blocked(ProtectMatch {
        group: RuleGroup::Custom,
        rule_id: rule_id.to_string(),
        pattern: String::new(),
        matched_text: reason.to_string(),
        surface,
        target_path: None,
        config_key: "protect.rules.custom".to_string(),
    })
}
