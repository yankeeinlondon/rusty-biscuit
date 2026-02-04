use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use tracing::warn;

use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

use super::ProviderAdapter;

pub(crate) struct RooAdapter;

impl ProviderAdapter for RooAdapter {
    fn provider(&self) -> Provider {
        Provider::RooCode
    }

    fn parse_event(
        &self,
        raw: &Value,
        env: &EnvironmentContext,
    ) -> Option<(AgenticEvent, EventMeta)> {
        let event_name = match raw.get("type").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                warn!("Roo adapter: missing or non-string 'type' field");
                return None;
            }
        };

        let event = match event_name {
            "taskCompleted" => AgenticEvent::TurnComplete,
            "taskToolFailed" => AgenticEvent::ToolError,
            "taskSpawned" => AgenticEvent::SubagentStart,
            "taskDelegationCompleted" => AgenticEvent::SubagentStop,
            "error" => AgenticEvent::TurnError,
            "tool_result" => AgenticEvent::AfterTool,
            _ => {
                warn!(event_name, "Roo adapter: unknown type field");
                return None;
            }
        };

        let meta = EventMeta {
            provider: Provider::RooCode,
            event: event.clone(),
            timestamp: Utc::now(),
            session_id: str_field(raw, "session_id"),
            cwd: str_field(raw, "cwd"),
            tool_name: str_field(raw, "tool_name"),
            tool_input: raw.get("tool_input").cloned(),
            tool_response: raw.get("tool_response").cloned(),
            error: str_field(raw, "error"),
            prompt: str_field(raw, "prompt"),
            agent_type: str_field(raw, "agent_type"),
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: env.clone(),
        };

        Some((event, meta))
    }
}

/// Extract a string field from a JSON value.
fn str_field(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(|v| v.as_str()).map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_env() -> EnvironmentContext {
        EnvironmentContext::default()
    }

    #[test]
    fn maps_task_completed_to_turn_complete() {
        let adapter = RooAdapter;
        let raw = json!({
            "type": "taskCompleted",
            "session_id": "roo-001"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
        assert_eq!(meta.session_id.as_deref(), Some("roo-001"));
        assert_eq!(meta.provider, Provider::RooCode);
    }

    #[test]
    fn maps_task_tool_failed_to_tool_error() {
        let adapter = RooAdapter;
        let raw = json!({
            "type": "taskToolFailed",
            "tool_name": "apply_diff",
            "error": "Patch conflict"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::ToolError);
        assert_eq!(meta.tool_name.as_deref(), Some("apply_diff"));
        assert_eq!(meta.error.as_deref(), Some("Patch conflict"));
    }

    #[test]
    fn maps_task_spawned_to_subagent_start() {
        let adapter = RooAdapter;
        let raw = json!({
            "type": "taskSpawned",
            "agent_type": "code_review"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::SubagentStart);
        assert_eq!(meta.agent_type.as_deref(), Some("code_review"));
    }

    #[test]
    fn maps_task_delegation_completed_to_subagent_stop() {
        let adapter = RooAdapter;
        let raw = json!({"type": "taskDelegationCompleted"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::SubagentStop);
    }

    #[test]
    fn maps_error_to_turn_error() {
        let adapter = RooAdapter;
        let raw = json!({
            "type": "error",
            "error": "API timeout"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(meta.error.as_deref(), Some("API timeout"));
    }

    #[test]
    fn maps_tool_result_to_after_tool() {
        let adapter = RooAdapter;
        let raw = json!({
            "type": "tool_result",
            "tool_name": "read_file",
            "tool_response": {"content": "file data"}
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::AfterTool);
        assert_eq!(meta.tool_name.as_deref(), Some("read_file"));
        assert_eq!(
            meta.tool_response,
            Some(json!({"content": "file data"}))
        );
    }

    #[test]
    fn unknown_type_returns_none() {
        let adapter = RooAdapter;
        let raw = json!({"type": "taskProgress"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn missing_type_returns_none() {
        let adapter = RooAdapter;
        let raw = json!({"session_id": "abc"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn malformed_json_empty_object_returns_none() {
        let adapter = RooAdapter;
        let raw = json!({});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn non_string_type_returns_none() {
        let adapter = RooAdapter;
        let raw = json!({"type": true});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn provider_returns_roo_code() {
        let adapter = RooAdapter;
        assert_eq!(adapter.provider(), Provider::RooCode);
    }
}
