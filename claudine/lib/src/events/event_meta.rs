use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::agentic_event::AgenticEvent;
use super::environment::EnvironmentContext;
use crate::provider::Provider;

/// Normalized metadata attached to every fired event.
///
/// Provider adapters populate this from their native event payloads.
/// The `extra` map carries provider-specific fields that don't fit
/// the common schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Immediate child PID returned by the wrapper spawn operation.
    ///
    /// `None` until a successful provider spawn. Raw JSONL records omit
    /// the key entirely when unavailable (`skip_serializing_if`), while
    /// reporting DTOs and SQL columns expose a stable nullable field
    /// where `null` means "no provider child PID was available for that
    /// row".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_pid: Option<u32>,
}

/// Structured wrapper for provider tool names.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolName(pub String);

impl ToolName {
    /// Returns whether this tool name follows the `mcp__<server>__<tool>` pattern.
    pub fn is_mcp_tool(&self) -> bool {
        self.0.starts_with("mcp__")
    }

    /// Returns the `(server, tool)` segments for MCP tools.
    pub fn mcp_components(&self) -> Option<(&str, &str)> {
        let remainder = self.0.strip_prefix("mcp__")?;
        let (server, tool) = remainder.split_once("__")?;
        Some((server, tool))
    }
}

impl EventMeta {
    /// Create a new event metadata record with all optional fields empty.
    pub fn new(provider: Provider, event: AgenticEvent) -> Self {
        Self {
            provider,
            event,
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
            env: EnvironmentContext::default(),
            agent_pid: None,
        }
    }

    /// Create a minimal EventMeta with only environment context populated.
    ///
    /// Useful for resolving context variables without a real event.
    pub fn dummy_with_env(env: EnvironmentContext) -> Self {
        let mut meta = Self::new(Provider::Claude, AgenticEvent::SessionStart);
        meta.env = env;
        meta
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
            agent_pid: None,
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

    /// Phase 3 — `agent_pid` MUST be omitted from JSONL when `None`.
    ///
    /// This keeps raw stream records compact when the wrapper has not yet
    /// spawned the provider (or when the record originates from a hook
    /// handler that has no way to know the agent PID).
    #[test]
    fn agent_pid_omitted_when_none() {
        let meta = sample_meta();
        let json = serde_json::to_value(&meta).unwrap();
        assert!(
            json.get("agent_pid").is_none(),
            "agent_pid must be omitted when None; got: {json}"
        );
    }

    /// Phase 3 — `agent_pid` MUST appear as a number when populated.
    #[test]
    fn agent_pid_serialized_when_some() {
        let mut meta = sample_meta();
        meta.agent_pid = Some(42_345);
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["agent_pid"], 42_345);
    }

    /// Phase 3 — legacy JSONL without `agent_pid` MUST round-trip to `None`.
    #[test]
    fn agent_pid_defaults_to_none_on_deserialize() {
        let json = serde_json::json!({
            "provider": "claude",
            "event": "before_tool"
        });
        let meta: EventMeta = serde_json::from_value(json).unwrap();
        assert!(meta.agent_pid.is_none());
    }
}
