use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use tracing::warn;

use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

use super::ProviderAdapter;

pub(crate) struct ClaudeAdapter;

impl ProviderAdapter for ClaudeAdapter {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn parse_event(
        &self,
        raw: &Value,
        env: &EnvironmentContext,
    ) -> Option<(AgenticEvent, EventMeta)> {
        let event_name = match raw.get("hook_event_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                warn!("Claude adapter: missing or non-string 'hook_event_name'");
                return None;
            }
        };

        let event = match event_name {
            "SessionStart" => AgenticEvent::SessionStart,
            "SessionEnd" => AgenticEvent::SessionEnd,
            "UserPromptSubmit" => AgenticEvent::BeforePrompt,
            "PreToolUse" => AgenticEvent::BeforeTool,
            "PostToolUse" => AgenticEvent::AfterTool,
            "PostToolUseFailure" => AgenticEvent::ToolError,
            "PermissionRequest" => AgenticEvent::PermissionRequest,
            "Stop" => AgenticEvent::TurnComplete,
            "SubagentStart" => AgenticEvent::SubagentStart,
            "SubagentStop" => AgenticEvent::SubagentStop,
            "PreCompact" => AgenticEvent::BeforeCompact,
            "Notification" => AgenticEvent::Notification,
            _ => {
                warn!(event_name, "Claude adapter: unknown hook_event_name");
                return None;
            }
        };

        let notification = raw.get("notification");
        let meta = EventMeta {
            provider: Provider::Claude,
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
            notification_type: notification
                .and_then(|n| n.get("type"))
                .and_then(|v| v.as_str())
                .map(String::from),
            notification_message: notification
                .and_then(|n| n.get("message"))
                .and_then(|v| v.as_str())
                .map(String::from),
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
    fn maps_session_start() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "SessionStart",
            "session_id": "sess-001",
            "cwd": "/home/user/project"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::SessionStart);
        assert_eq!(meta.session_id.as_deref(), Some("sess-001"));
        assert_eq!(meta.cwd.as_deref(), Some("/home/user/project"));
        assert_eq!(meta.provider, Provider::Claude);
    }

    #[test]
    fn maps_before_tool_with_metadata() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "sess-002",
            "tool_name": "Bash",
            "tool_input": {"command": "ls -la"}
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
        assert_eq!(meta.tool_name.as_deref(), Some("Bash"));
        assert_eq!(meta.tool_input, Some(json!({"command": "ls -la"})));
    }

    #[test]
    fn maps_after_tool_with_response() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "PostToolUse",
            "tool_name": "Read",
            "tool_response": {"content": "file contents"}
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::AfterTool);
        assert_eq!(meta.tool_name.as_deref(), Some("Read"));
        assert_eq!(meta.tool_response, Some(json!({"content": "file contents"})));
    }

    #[test]
    fn maps_tool_error() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "PostToolUseFailure",
            "tool_name": "Bash",
            "error": "command not found"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::ToolError);
        assert_eq!(meta.error.as_deref(), Some("command not found"));
    }

    #[test]
    fn maps_permission_request() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "PermissionRequest",
            "tool_name": "Bash"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::PermissionRequest);
    }

    #[test]
    fn maps_before_prompt() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "UserPromptSubmit",
            "prompt": "Fix the bug"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforePrompt);
        assert_eq!(meta.prompt.as_deref(), Some("Fix the bug"));
    }

    #[test]
    fn maps_turn_complete() {
        let adapter = ClaudeAdapter;
        let raw = json!({"hook_event_name": "Stop"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
    }

    #[test]
    fn maps_subagent_start() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "SubagentStart",
            "agent_type": "task"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::SubagentStart);
        assert_eq!(meta.agent_type.as_deref(), Some("task"));
    }

    #[test]
    fn maps_subagent_stop() {
        let adapter = ClaudeAdapter;
        let raw = json!({"hook_event_name": "SubagentStop"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::SubagentStop);
    }

    #[test]
    fn maps_before_compact() {
        let adapter = ClaudeAdapter;
        let raw = json!({"hook_event_name": "PreCompact"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeCompact);
    }

    #[test]
    fn maps_notification_with_details() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "Notification",
            "notification": {
                "type": "info",
                "message": "Context window 80% full"
            }
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::Notification);
        assert_eq!(meta.notification_type.as_deref(), Some("info"));
        assert_eq!(
            meta.notification_message.as_deref(),
            Some("Context window 80% full")
        );
    }

    #[test]
    fn maps_session_end() {
        let adapter = ClaudeAdapter;
        let raw = json!({"hook_event_name": "SessionEnd"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::SessionEnd);
    }

    #[test]
    fn unknown_event_returns_none() {
        let adapter = ClaudeAdapter;
        let raw = json!({"hook_event_name": "SomeNewEvent"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn missing_hook_event_name_returns_none() {
        let adapter = ClaudeAdapter;
        let raw = json!({"session_id": "abc"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn malformed_json_missing_fields_returns_none() {
        let adapter = ClaudeAdapter;
        let raw = json!({});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn non_string_hook_event_name_returns_none() {
        let adapter = ClaudeAdapter;
        let raw = json!({"hook_event_name": 42});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn provider_returns_claude() {
        let adapter = ClaudeAdapter;
        assert_eq!(adapter.provider(), Provider::Claude);
    }
}
