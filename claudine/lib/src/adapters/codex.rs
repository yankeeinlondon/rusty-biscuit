use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use tracing::warn;

use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

use super::ProviderAdapter;

pub(crate) struct CodexAdapter;

impl ProviderAdapter for CodexAdapter {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn parse_event(
        &self,
        raw: &Value,
        env: &EnvironmentContext,
    ) -> Option<(AgenticEvent, EventMeta)> {
        let type_field = match raw.get("type").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                warn!("Codex adapter: missing or non-string 'type' field");
                return None;
            }
        };

        let event = match type_field {
            "thread.started" => AgenticEvent::SessionStart,
            "turn.completed" => AgenticEvent::TurnComplete,
            "turn.failed" => AgenticEvent::TurnError,
            "item.completed" => {
                // Only map to AfterTool if the item is a command_execution
                let item_type = raw
                    .get("item")
                    .and_then(|item| item.get("type"))
                    .and_then(|v| v.as_str());
                if item_type == Some("command_execution") {
                    AgenticEvent::AfterTool
                } else {
                    warn!(
                        ?item_type,
                        "Codex adapter: item.completed with non-command_execution type"
                    );
                    return None;
                }
            }
            "error" => AgenticEvent::TurnError,
            _ => {
                warn!(type_field, "Codex adapter: unknown type field");
                return None;
            }
        };

        let meta = EventMeta {
            provider: Provider::Codex,
            event: event.clone(),
            timestamp: Utc::now(),
            session_id: str_field(raw, "thread_id"),
            cwd: str_field(raw, "cwd"),
            tool_name: raw
                .get("item")
                .and_then(|item| item.get("name"))
                .and_then(|v| v.as_str())
                .map(String::from),
            tool_input: raw.get("item").and_then(|item| item.get("input")).cloned(),
            tool_response: raw.get("item").and_then(|item| item.get("output")).cloned(),
            error: raw
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .map(String::from),
            prompt: str_field(raw, "prompt"),
            agent_type: None,
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
    fn maps_thread_started_to_session_start() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "thread.started",
            "thread_id": "thread-abc-123"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::SessionStart);
        assert_eq!(meta.session_id.as_deref(), Some("thread-abc-123"));
        assert_eq!(meta.provider, Provider::Codex);
    }

    #[test]
    fn maps_turn_completed() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "turn.completed",
            "thread_id": "thread-001"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
    }

    #[test]
    fn maps_turn_failed_to_turn_error() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "turn.failed",
            "thread_id": "thread-001"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
    }

    #[test]
    fn maps_item_completed_command_execution_to_after_tool() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "item.completed",
            "thread_id": "thread-002",
            "item": {
                "type": "command_execution",
                "name": "shell",
                "input": {"command": "cargo test"},
                "output": {"exit_code": 0}
            }
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::AfterTool);
        assert_eq!(meta.tool_name.as_deref(), Some("shell"));
        assert_eq!(meta.tool_input, Some(json!({"command": "cargo test"})));
        assert_eq!(meta.tool_response, Some(json!({"exit_code": 0})));
    }

    #[test]
    fn item_completed_non_command_returns_none() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "item.completed",
            "item": {
                "type": "message",
                "content": "Hello"
            }
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn item_completed_missing_item_type_returns_none() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "item.completed",
            "item": {
                "name": "shell"
            }
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn item_completed_missing_item_returns_none() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "item.completed"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn maps_error_to_turn_error() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "error",
            "error": {
                "message": "Rate limit exceeded"
            }
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(meta.error.as_deref(), Some("Rate limit exceeded"));
    }

    #[test]
    fn unknown_type_returns_none() {
        let adapter = CodexAdapter;
        let raw = json!({"type": "stream.delta"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn missing_type_field_returns_none() {
        let adapter = CodexAdapter;
        let raw = json!({"thread_id": "abc"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn malformed_json_empty_object_returns_none() {
        let adapter = CodexAdapter;
        let raw = json!({});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn non_string_type_returns_none() {
        let adapter = CodexAdapter;
        let raw = json!({"type": 123});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn provider_returns_codex() {
        let adapter = CodexAdapter;
        assert_eq!(adapter.provider(), Provider::Codex);
    }
}
