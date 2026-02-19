use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unified response that a hook can return to influence agent behavior.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HookResponse {
    /// The decision to communicate back to the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<HookDecision>,

    /// Human-readable reason for the decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Modified tool input to substitute before execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<Value>,

    /// Additional context string to inject into the agent context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,

    /// Raw provider-specific response fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
}

/// Decisions a hook can communicate back to the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HookDecision {
    /// Allow the action to proceed.
    Allow,

    /// Block/deny the action.
    Deny,

    /// Show the user a permission dialog when supported.
    Ask,

    /// Continue processing instead of stopping.
    Continue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_opinion() {
        let response = HookResponse::default();
        assert_eq!(response.decision, None);
        assert_eq!(response.reason, None);
        assert_eq!(response.updated_input, None);
        assert_eq!(response.additional_context, None);
        assert_eq!(response.raw, None);
    }

    #[test]
    fn decision_serializes_snake_case() {
        let json = serde_json::to_value(HookDecision::Continue).unwrap();
        assert_eq!(json, serde_json::json!("continue"));
    }
}
