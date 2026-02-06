use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::agentric_event::AgenticEvent;
use super::environment::EnvironmentContext;
use super::provider::Provider;

/// Normalized metadata attached to every fired event.
///
/// Provider adapters populate this from their native event payloads.
/// The `extra` map carries provider-specific fields that don't fit
/// the common schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMeta {
    /// Which agent provider fired the event.
    pub provider: Provider,

    /// The shared event that was matched.
    pub event: AgenticEvent,

    /// UTC timestamp of when the event was received.
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,

    /// Session or thread identifier (provider-dependent format).
    #[serde(default)]
    pub session_id: Option<String>,

    /// Current working directory at the time of the event.
    #[serde(default)]
    pub cwd: Option<String>,

    /// Tool name, if the event is tool-related.
    #[serde(default)]
    pub tool_name: Option<String>,

    /// Tool input/arguments, if the event is tool-related.
    #[serde(default)]
    pub tool_input: Option<Value>,

    /// Tool output/response, if the event is a post-tool event.
    #[serde(default)]
    pub tool_response: Option<Value>,

    /// Error message, if the event represents a failure.
    #[serde(default)]
    pub error: Option<String>,

    /// The user's prompt text, for prompt-related events.
    #[serde(default)]
    pub prompt: Option<String>,

    /// Agent/subagent type or identifier.
    #[serde(default)]
    pub agent_type: Option<String>,

    /// Notification type string.
    #[serde(default)]
    pub notification_type: Option<String>,

    /// Notification message text.
    #[serde(default)]
    pub notification_message: Option<String>,

    /// Provider-specific fields that don't map to common fields.
    #[serde(default)]
    pub extra: HashMap<String, Value>,

    /// Snapshot of the host and repository environment.
    ///
    /// Populated once at session start via `sniff` and reused
    /// for all events in the session.
    #[serde(default)]
    pub env: EnvironmentContext,
}

impl EventMeta {
    /// Create a minimal EventMeta with only environment context populated.
    ///
    /// Useful for resolving context variables without a real event.
    pub fn dummy_with_env(env: EnvironmentContext) -> Self {
        Self {
            provider: Provider::Claude,
            event: AgenticEvent::SessionStart,
            timestamp: Utc::now(),
            session_id: None,
            cwd: None,
            tool_name: None,
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_meta() -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: AgenticEvent::BeforeTool,
            timestamp: Utc::now(),
            session_id: Some("abc123".to_string()),
            cwd: Some("/tmp".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: Some(serde_json::json!({"command": "npm test"})),
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    #[test]
    fn round_trip_json() {
        let meta = sample_meta();
        let json = serde_json::to_string(&meta).unwrap();
        let back: EventMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider, Provider::Claude);
        assert_eq!(back.event, AgenticEvent::BeforeTool);
        assert_eq!(back.session_id.as_deref(), Some("abc123"));
        assert_eq!(back.tool_name.as_deref(), Some("Bash"));
    }

    #[test]
    fn event_field_serializes_as_snake_case() {
        let meta = sample_meta();
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["event"], "before_tool");
        assert_eq!(json["provider"], "claude");
    }

    #[test]
    fn extra_is_not_flattened() {
        let mut meta = sample_meta();
        meta.extra
            .insert("custom_key".to_string(), serde_json::json!("custom_value"));
        let json = serde_json::to_value(&meta).unwrap();
        // extra should be a nested object, not flattened into the top level
        assert!(json["extra"].is_object());
        assert_eq!(json["extra"]["custom_key"], "custom_value");
    }

    #[test]
    fn deserialize_minimal() {
        let json = serde_json::json!({
            "provider": "gemini",
            "event": "session_start"
        });
        let meta: EventMeta = serde_json::from_value(json).unwrap();
        assert_eq!(meta.provider, Provider::Gemini);
        assert_eq!(meta.event, AgenticEvent::SessionStart);
        assert!(meta.session_id.is_none());
        assert!(meta.tool_name.is_none());
        assert!(meta.extra.is_empty());
    }
}
