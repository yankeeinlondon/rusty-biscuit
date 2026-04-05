use serde::de::{self, Deserializer};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::services::ProtectOutcome;

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

    /// Optional protect context attached by protect-aware action execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<ProtectCallContext>,
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

/// Context attached to responses when Protect influenced call action execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectCallContext {
    /// Protect outcome.
    pub outcome: ProtectOutcome,
    /// Protect reason code or message.
    pub reason: String,
    /// Whether execution was short-circuited before running the call action.
    pub short_circuited: bool,
}

impl Serialize for ProtectCallContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ProtectCallContext", 3)?;
        state.serialize_field("outcome", protect_outcome_slug(&self.outcome))?;
        state.serialize_field("reason", &self.reason)?;
        state.serialize_field("short_circuited", &self.short_circuited)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ProtectCallContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ProtectCallContextWire {
            outcome: String,
            reason: String,
            #[serde(default)]
            short_circuited: bool,
        }

        let wire = ProtectCallContextWire::deserialize(deserializer)?;
        let outcome =
            protect_outcome_from_slug(&wire.outcome, &wire.reason).map_err(de::Error::custom)?;
        Ok(Self {
            outcome,
            reason: wire.reason,
            short_circuited: wire.short_circuited,
        })
    }
}

fn protect_outcome_slug(outcome: &ProtectOutcome) -> &'static str {
    match outcome {
        ProtectOutcome::Allow => "allow",
        ProtectOutcome::AskThenAllowOrStop { .. } => "ask_then_allow_or_stop",
        ProtectOutcome::StopCurrent { .. } => "stop_current",
        ProtectOutcome::StopSession { .. } => "stop_session",
        ProtectOutcome::AllowWithRedaction { .. } => "allow_with_redaction",
        ProtectOutcome::AdvisoryOnly { .. } => "advisory_only",
    }
}

fn protect_outcome_from_slug(slug: &str, reason: &str) -> Result<ProtectOutcome, String> {
    let reason = reason.to_string();
    match slug {
        "allow" => Ok(ProtectOutcome::Allow),
        "ask_then_allow_or_stop" => Ok(ProtectOutcome::AskThenAllowOrStop { reason }),
        "stop_current" => Ok(ProtectOutcome::StopCurrent { reason }),
        "stop_session" => Ok(ProtectOutcome::StopSession { reason }),
        "allow_with_redaction" => Ok(ProtectOutcome::AllowWithRedaction { reason }),
        "advisory_only" => Ok(ProtectOutcome::AdvisoryOnly { reason }),
        _ => Err(format!("unknown protect outcome `{slug}`")),
    }
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
        assert_eq!(response.protect, None);
    }

    #[test]
    fn decision_serializes_snake_case() {
        let json = serde_json::to_value(HookDecision::Continue).unwrap();
        assert_eq!(json, serde_json::json!("continue"));
    }

    #[test]
    fn protect_context_serializes_as_outcome_slug() {
        let context = ProtectCallContext {
            outcome: ProtectOutcome::StopCurrent {
                reason: "blocked".to_string(),
            },
            reason: "blocked".to_string(),
            short_circuited: true,
        };

        let json = serde_json::to_value(&context).unwrap();
        assert_eq!(json["outcome"], "stop_current");
        assert_eq!(json["reason"], "blocked");
        assert_eq!(json["short_circuited"], true);
    }
}
