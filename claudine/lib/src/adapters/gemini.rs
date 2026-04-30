use serde_json::{Value, json};

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

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
        let mut meta = EventMeta::new(Provider::Gemini, event);
        meta.session_id = str_field(raw, "session_id");
        meta.cwd = str_field(raw, "cwd");
        meta.tool_name = str_field(raw, "tool_name").or_else(|| str_field(raw, "toolName"));
        meta.tool_input = raw.get("tool_input").cloned();
        meta.tool_response = raw.get("tool_response").cloned();
        meta.error = str_field(raw, "error");
        meta.prompt = str_field(raw, "prompt");
        meta.agent_type = str_field(raw, "agent_type");
        meta.notification_type = raw
            .get("notification")
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        meta.notification_message = raw
            .get("notification")
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

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

        // Include redacted content in AfterTool responses.
        if matches!(event, AgenticEvent::AfterTool) {
            let mut body = serde_json::Map::new();
            if matches!(response.decision, Some(HookDecision::Deny)) {
                body.insert(
                    "error".to_string(),
                    Value::String(
                        response
                            .reason
                            .clone()
                            .unwrap_or_else(|| "blocked".to_string()),
                    ),
                );
            }
            if let Some(ref ctx) = response.additional_context {
                body.insert("updatedToolResult".to_string(), Value::String(ctx.clone()));
            }
            if let Some(ref input) = response.updated_input {
                body.insert("updatedToolResult".to_string(), input.clone());
            }
            if body.is_empty() {
                return Ok(Value::Object(Default::default()));
            }
            return Ok(Value::Object(body));
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
