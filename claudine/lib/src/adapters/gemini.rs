use std::collections::HashMap;

use chrono::Utc;
use serde_json::{Value, json};

use crate::events::{
    AgenticEvent, EnvironmentContext, EventMeta, HookDecision, HookResponse, Provider,
};

use super::{AdapterError, ProviderAdapter};

pub(crate) struct GeminiAdapter;

impl ProviderAdapter for GeminiAdapter {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event_name = raw
            .get("hook_event_name")
            .or_else(|| raw.get("hookEventName"))
            .or_else(|| raw.get("event_name"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("hook_event_name"))?;

        let event = map_event(event_name)?;
        let mut meta = EventMeta {
            provider: Provider::Gemini,
            event,
            timestamp: Utc::now(),
            session_id: str_field(raw, "session_id"),
            cwd: str_field(raw, "cwd"),
            tool_name: str_field(raw, "tool_name").or_else(|| str_field(raw, "toolName")),
            tool_input: raw.get("tool_input").cloned(),
            tool_response: raw.get("tool_response").cloned(),
            error: str_field(raw, "error"),
            prompt: str_field(raw, "prompt"),
            agent_type: str_field(raw, "agent_type"),
            notification_type: raw
                .get("notification")
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            notification_message: raw
                .get("notification")
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        };

        for key in [
            "llm_request",
            "llm_response",
            "tool_config",
            "aggregation_strategy",
            "mcp_context",
        ] {
            if let Some(value) = raw.get(key) {
                meta.extra.insert(key.to_string(), value.clone());
            }
        }

        Ok((event, meta))
    }

    fn can_block(&self, event: &AgenticEvent) -> bool {
        matches!(
            event,
            AgenticEvent::BeforePrompt
                | AgenticEvent::TurnComplete
                | AgenticEvent::BeforeModel
                | AgenticEvent::AfterModel
                | AgenticEvent::BeforeTool
                | AgenticEvent::AfterTool
        )
    }

    fn format_response(
        &self,
        event: &AgenticEvent,
        response: &HookResponse,
    ) -> Result<Value, AdapterError> {
        if let Some(raw) = &response.raw {
            return Ok(raw.clone());
        }

        if !self.can_block(event) {
            return Ok(Value::Null);
        }

        match response.decision.unwrap_or(HookDecision::Allow) {
            HookDecision::Allow => Ok(Value::Object(Default::default())),
            HookDecision::Deny => match event {
                AgenticEvent::TurnComplete => Ok(json!({
                    "reason": response.reason.clone().unwrap_or_else(|| "blocked".to_string()),
                    "clearContext": false
                })),
                AgenticEvent::BeforeTool => Ok(json!({
                    "error": response.reason.clone().unwrap_or_else(|| "blocked".to_string())
                })),
                _ => Ok(json!({
                    "error": response.reason.clone().unwrap_or_else(|| "blocked".to_string())
                })),
            },
            HookDecision::Continue => Ok(json!({
                "decision": "continue",
                "reason": response.reason
            })),
            HookDecision::Ask => Ok(json!({
                "decision": "ask",
                "reason": response.reason
            })),
        }
    }

    fn exit_code(&self, event: &AgenticEvent, response: &HookResponse) -> Option<i32> {
        if !self.can_block(event) {
            return None;
        }

        match response.decision.unwrap_or(HookDecision::Allow) {
            HookDecision::Deny => Some(2),
            _ => Some(0),
        }
    }
}

fn map_event(event_name: &str) -> Result<AgenticEvent, AdapterError> {
    Provider::Gemini
        .event_from_shared_native_name(event_name)
        .ok_or_else(|| AdapterError::UnknownEvent(event_name.to_string()))
}

fn str_field(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_maps_before_tool_selection() {
        let adapter = GeminiAdapter;
        let raw = json!({ "hook_event_name": "BeforeToolSelection" });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::BeforeModel);
    }

    #[test]
    fn can_block_matches_matrix() {
        let adapter = GeminiAdapter;
        assert!(adapter.can_block(&AgenticEvent::BeforePrompt));
        assert!(adapter.can_block(&AgenticEvent::AfterTool));
        assert!(!adapter.can_block(&AgenticEvent::Notification));
    }

    #[test]
    fn deny_before_tool_formats_error() {
        let adapter = GeminiAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("disallowed tool".to_string()),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::BeforeTool, &response)
            .unwrap();
        assert_eq!(body["error"], "disallowed tool");
        assert_eq!(
            adapter.exit_code(&AgenticEvent::BeforeTool, &response),
            Some(2)
        );
    }

    #[test]
    fn after_agent_retry_payload() {
        let adapter = GeminiAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("response too short".to_string()),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::TurnComplete, &response)
            .unwrap();
        assert_eq!(body["reason"], "response too short");
        assert_eq!(body["clearContext"], false);
    }
}
