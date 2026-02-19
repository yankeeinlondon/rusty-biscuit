use std::collections::HashMap;

use chrono::Utc;
use serde_json::{Value, json};

use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

use super::{AdapterError, ProviderAdapter};

pub(crate) struct QwenAdapter;

impl ProviderAdapter for QwenAdapter {
    fn provider(&self) -> Provider {
        Provider::QwenCode
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event_name = raw
            .get("event_name")
            .or_else(|| raw.get("type"))
            .or_else(|| raw.get("event"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("event_name"))?;

        let event = map_event(event_name)?;
        let mut meta = EventMeta {
            provider: Provider::QwenCode,
            event,
            timestamp: Utc::now(),
            session_id: str_field(raw, "session_id"),
            cwd: str_field(raw, "cwd"),
            tool_name: str_field(raw, "tool_name"),
            tool_input: raw.get("tool_input").cloned(),
            tool_response: raw.get("tool_response").cloned(),
            error: str_field(raw, "error"),
            prompt: str_field(raw, "prompt"),
            agent_type: str_field(raw, "agent_type"),
            notification_type: str_field(raw, "notification_type"),
            notification_message: str_field(raw, "message"),
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        };

        for key in [
            "approval_mode",
            "permission_priority",
            "subagent_id",
            "can_use_tool_timeout_secs",
        ] {
            if let Some(value) = raw.get(key) {
                meta.extra.insert(key.to_string(), value.clone());
            }
        }

        Ok((event, meta))
    }

    fn can_block(&self, event: &AgenticEvent) -> bool {
        matches!(event, AgenticEvent::PermissionRequest)
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

        let allow = !matches!(response.decision, Some(HookDecision::Deny));
        Ok(json!({
            "decision": if allow { "allow" } else { "deny" },
            "reason": response.reason,
        }))
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        None
    }
}

fn map_event(event_name: &str) -> Result<AgenticEvent, AdapterError> {
    match event_name {
        "CanUseTool" => Ok(AgenticEvent::PermissionRequest),
        "SubagentPreToolUse" => Ok(AgenticEvent::BeforeTool),
        "SubagentPostToolUse" => Ok(AgenticEvent::AfterTool),
        "SubagentStop" => Ok(AgenticEvent::SubagentStop),
        "StreamSessionStart" => Ok(AgenticEvent::SessionStart),
        "StreamAssistantMessage" => Ok(AgenticEvent::AfterModel),
        "StreamResult" => Ok(AgenticEvent::SessionEnd),
        other => Err(AdapterError::UnknownEvent(other.to_string())),
    }
}

fn str_field(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn can_use_tool_maps_permission_request() {
        let adapter = QwenAdapter;
        let raw = json!({
            "event_name": "CanUseTool",
            "can_use_tool_timeout_secs": 60
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::PermissionRequest);
        assert_eq!(meta.extra["can_use_tool_timeout_secs"], json!(60));
    }

    #[test]
    fn only_permission_is_blocking() {
        let adapter = QwenAdapter;
        assert!(adapter.can_block(&AgenticEvent::PermissionRequest));
        assert!(!adapter.can_block(&AgenticEvent::AfterTool));
    }

    #[test]
    fn format_response_permission() {
        let adapter = QwenAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("blocked".to_string()),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::PermissionRequest, &response)
            .unwrap();
        assert_eq!(body["decision"], "deny");
    }
}
