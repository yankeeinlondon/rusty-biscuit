use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use tracing::warn;

use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

use super::ProviderAdapter;

pub(crate) struct OpenCodeAdapter;

impl ProviderAdapter for OpenCodeAdapter {
    fn provider(&self) -> Provider {
        Provider::OpenCode
    }

    fn parse_event(
        &self,
        raw: &Value,
        env: &EnvironmentContext,
    ) -> Option<(AgenticEvent, EventMeta)> {
        let event_type = match raw.get("event_type").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                warn!("OpenCode adapter: missing or non-string 'event_type'");
                return None;
            }
        };

        let event = match event_type {
            "session.created" => AgenticEvent::SessionStart,
            "session.deleted" => AgenticEvent::SessionEnd,
            "session.idle" => AgenticEvent::TurnComplete,
            "session.error" => AgenticEvent::TurnError,
            "session.compacted" | "experimental.session.compacting" => AgenticEvent::BeforeCompact,
            "permission.asked" => AgenticEvent::PermissionRequest,
            "chat.message" => AgenticEvent::BeforePrompt,
            "tool.execute.before" => AgenticEvent::BeforeTool,
            "tool.execute.after" => AgenticEvent::AfterTool,
            "chat.params" => AgenticEvent::BeforeModel,
            _ => {
                warn!(event_type, "OpenCode adapter: unknown event_type");
                return None;
            }
        };

        let meta = EventMeta {
            provider: Provider::OpenCode,
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
    fn maps_session_created_to_session_start() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "event_type": "session.created",
            "session_id": "oc-sess-001"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::SessionStart);
        assert_eq!(meta.session_id.as_deref(), Some("oc-sess-001"));
        assert_eq!(meta.provider, Provider::OpenCode);
    }

    #[test]
    fn maps_session_deleted_to_session_end() {
        let adapter = OpenCodeAdapter;
        let raw = json!({"event_type": "session.deleted"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::SessionEnd);
    }

    #[test]
    fn maps_session_idle_to_turn_complete() {
        let adapter = OpenCodeAdapter;
        let raw = json!({"event_type": "session.idle"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
    }

    #[test]
    fn maps_session_error_to_turn_error() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "event_type": "session.error",
            "error": "context limit exceeded"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(meta.error.as_deref(), Some("context limit exceeded"));
    }

    #[test]
    fn maps_session_compacted_to_before_compact() {
        let adapter = OpenCodeAdapter;
        let raw = json!({"event_type": "session.compacted"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeCompact);
    }

    #[test]
    fn maps_experimental_compacting_to_before_compact() {
        let adapter = OpenCodeAdapter;
        let raw = json!({"event_type": "experimental.session.compacting"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeCompact);
    }

    #[test]
    fn maps_permission_asked_to_permission_request() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "event_type": "permission.asked",
            "tool_name": "file_write"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::PermissionRequest);
        assert_eq!(meta.tool_name.as_deref(), Some("file_write"));
    }

    #[test]
    fn maps_chat_message_to_before_prompt() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "event_type": "chat.message",
            "prompt": "Refactor this function"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforePrompt);
        assert_eq!(meta.prompt.as_deref(), Some("Refactor this function"));
    }

    #[test]
    fn maps_tool_execute_before_to_before_tool() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "event_type": "tool.execute.before",
            "tool_name": "bash",
            "tool_input": {"command": "npm test"}
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
        assert_eq!(meta.tool_name.as_deref(), Some("bash"));
        assert_eq!(meta.tool_input, Some(json!({"command": "npm test"})));
    }

    #[test]
    fn maps_tool_execute_after_to_after_tool() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "event_type": "tool.execute.after",
            "tool_name": "bash",
            "tool_response": {"output": "tests passed"}
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::AfterTool);
        assert_eq!(meta.tool_response, Some(json!({"output": "tests passed"})));
    }

    #[test]
    fn maps_chat_params_to_before_model() {
        let adapter = OpenCodeAdapter;
        let raw = json!({"event_type": "chat.params"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeModel);
    }

    #[test]
    fn unknown_event_type_returns_none() {
        let adapter = OpenCodeAdapter;
        let raw = json!({"event_type": "session.unknown"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn missing_event_type_returns_none() {
        let adapter = OpenCodeAdapter;
        let raw = json!({"session_id": "abc"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn malformed_json_empty_object_returns_none() {
        let adapter = OpenCodeAdapter;
        let raw = json!({});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn provider_returns_open_code() {
        let adapter = OpenCodeAdapter;
        assert_eq!(adapter.provider(), Provider::OpenCode);
    }
}
