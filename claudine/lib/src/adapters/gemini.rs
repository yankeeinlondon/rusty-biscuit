use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use tracing::warn;

use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

use super::ProviderAdapter;

pub(crate) struct GeminiAdapter;

impl ProviderAdapter for GeminiAdapter {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    fn parse_event(
        &self,
        raw: &Value,
        env: &EnvironmentContext,
    ) -> Option<(AgenticEvent, EventMeta)> {
        let event_name = match raw.get("hook_event_name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                warn!("Gemini adapter: missing or non-string 'hook_event_name'");
                return None;
            }
        };

        let event = match event_name {
            "SessionStart" => AgenticEvent::SessionStart,
            "SessionEnd" => AgenticEvent::SessionEnd,
            "BeforeAgent" => AgenticEvent::BeforePrompt,
            "BeforeTool" => AgenticEvent::BeforeTool,
            "AfterTool" => AgenticEvent::AfterTool,
            "AfterAgent" => AgenticEvent::TurnComplete,
            "BeforeModel" => AgenticEvent::BeforeModel,
            "AfterModel" => AgenticEvent::AfterModel,
            "PreCompress" => AgenticEvent::BeforeCompact,
            "Notification" => AgenticEvent::Notification,
            _ => {
                warn!(event_name, "Gemini adapter: unknown hook_event_name");
                return None;
            }
        };

        let notification = raw.get("notification");
        let meta = EventMeta {
            provider: Provider::Gemini,
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
        let adapter = GeminiAdapter;
        let raw = json!({
            "hook_event_name": "SessionStart",
            "session_id": "gem-001"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::SessionStart);
        assert_eq!(meta.session_id.as_deref(), Some("gem-001"));
        assert_eq!(meta.provider, Provider::Gemini);
    }

    #[test]
    fn maps_session_end() {
        let adapter = GeminiAdapter;
        let raw = json!({"hook_event_name": "SessionEnd"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::SessionEnd);
    }

    #[test]
    fn maps_before_agent_to_before_prompt() {
        let adapter = GeminiAdapter;
        let raw = json!({
            "hook_event_name": "BeforeAgent",
            "prompt": "Analyze this code"
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforePrompt);
        assert_eq!(meta.prompt.as_deref(), Some("Analyze this code"));
    }

    #[test]
    fn maps_before_tool_with_metadata() {
        let adapter = GeminiAdapter;
        let raw = json!({
            "hook_event_name": "BeforeTool",
            "tool_name": "search",
            "tool_input": {"query": "rust async"}
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
        assert_eq!(meta.tool_name.as_deref(), Some("search"));
        assert_eq!(meta.tool_input, Some(json!({"query": "rust async"})));
    }

    #[test]
    fn maps_after_tool() {
        let adapter = GeminiAdapter;
        let raw = json!({
            "hook_event_name": "AfterTool",
            "tool_name": "edit",
            "tool_response": {"status": "success"}
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::AfterTool);
        assert_eq!(meta.tool_response, Some(json!({"status": "success"})));
    }

    #[test]
    fn maps_after_agent_to_turn_complete() {
        let adapter = GeminiAdapter;
        let raw = json!({"hook_event_name": "AfterAgent"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
    }

    #[test]
    fn maps_before_model() {
        let adapter = GeminiAdapter;
        let raw = json!({"hook_event_name": "BeforeModel"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeModel);
    }

    #[test]
    fn maps_after_model() {
        let adapter = GeminiAdapter;
        let raw = json!({"hook_event_name": "AfterModel"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::AfterModel);
    }

    #[test]
    fn maps_pre_compress_to_before_compact() {
        let adapter = GeminiAdapter;
        let raw = json!({"hook_event_name": "PreCompress"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, _) = result.unwrap();
        assert_eq!(event, AgenticEvent::BeforeCompact);
    }

    #[test]
    fn maps_notification() {
        let adapter = GeminiAdapter;
        let raw = json!({
            "hook_event_name": "Notification",
            "notification": {
                "type": "warning",
                "message": "Rate limit approaching"
            }
        });
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_some());
        let (event, meta) = result.unwrap();
        assert_eq!(event, AgenticEvent::Notification);
        assert_eq!(meta.notification_type.as_deref(), Some("warning"));
        assert_eq!(
            meta.notification_message.as_deref(),
            Some("Rate limit approaching")
        );
    }

    #[test]
    fn unknown_event_returns_none() {
        let adapter = GeminiAdapter;
        let raw = json!({"hook_event_name": "UnknownGeminiEvent"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn missing_hook_event_name_returns_none() {
        let adapter = GeminiAdapter;
        let raw = json!({"session_id": "abc"});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn malformed_json_empty_object_returns_none() {
        let adapter = GeminiAdapter;
        let raw = json!({});
        let result = adapter.parse_event(&raw, &default_env());
        assert!(result.is_none());
    }

    #[test]
    fn provider_returns_gemini() {
        let adapter = GeminiAdapter;
        assert_eq!(adapter.provider(), Provider::Gemini);
    }
}
