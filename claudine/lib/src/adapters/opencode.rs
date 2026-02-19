use std::collections::HashMap;

use chrono::Utc;
use serde_json::{Value, json};

use crate::events::{
    AgenticEvent, EnvironmentContext, EventMeta, HookDecision, HookResponse, Provider,
};

use super::{AdapterError, ProviderAdapter};

pub(crate) struct OpenCodeAdapter;

impl ProviderAdapter for OpenCodeAdapter {
    fn provider(&self) -> Provider {
        Provider::OpenCode
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event_type = raw
            .get("event_type")
            .or_else(|| raw.get("eventType"))
            .or_else(|| raw.get("type"))
            .or_else(|| raw.get("event"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("event_type"))?;

        let event = map_event(event_type)?;
        let mut meta = EventMeta {
            provider: Provider::OpenCode,
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
            notification_type: if event == AgenticEvent::Notification {
                Some(event_type.to_string())
            } else {
                None
            },
            notification_message: str_field(raw, "message"),
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        };

        for key in ["bus_event_type", "plugin_context", "auth_method"] {
            if let Some(value) = raw.get(key) {
                meta.extra.insert(key.to_string(), value.clone());
            }
        }

        Ok((event, meta))
    }

    fn can_block(&self, event: &AgenticEvent) -> bool {
        matches!(
            event,
            AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest
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

        match event {
            AgenticEvent::BeforeTool => {
                if matches!(response.decision, Some(HookDecision::Deny)) {
                    Ok(json!({
                        "__action": "throw",
                        "message": response.reason.clone().unwrap_or_else(|| "blocked by policy".to_string())
                    }))
                } else {
                    Ok(json!({
                        "__action": "mutate",
                        "status": "allow"
                    }))
                }
            }
            AgenticEvent::PermissionRequest => {
                let status = match response.decision.unwrap_or(HookDecision::Allow) {
                    HookDecision::Allow => "allow",
                    HookDecision::Deny => "deny",
                    HookDecision::Ask => "ask",
                    HookDecision::Continue => "allow",
                };
                Ok(json!({
                    "__action": "mutate",
                    "status": status,
                    "reason": response.reason,
                }))
            }
            _ => Ok(Value::Null),
        }
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        None
    }
}

fn map_event(event_type: &str) -> Result<AgenticEvent, AdapterError> {
    if let Some(event) = Provider::OpenCode.event_from_shared_native_name(event_type) {
        return Ok(event);
    }

    match event_type {
        "chat.message" => Ok(AgenticEvent::BeforePrompt),
        "tool.execute.before" => Ok(AgenticEvent::BeforeTool),
        "tool.execute.after" => Ok(AgenticEvent::AfterTool),
        "chat.params"
        | "chat.headers"
        | "experimental.chat.system.transform"
        | "experimental.chat.messages.transform" => Ok(AgenticEvent::BeforeModel),
        "experimental.text.complete" => Ok(AgenticEvent::AfterModel),
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
    fn parse_permission_ask_is_blockable() {
        let adapter = OpenCodeAdapter;
        let raw = json!({ "event_type": "permission.ask" });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::PermissionRequest);
        assert!(adapter.can_block(&event));
    }

    #[test]
    fn parse_permission_asked_maps_to_human_in_loop() {
        let adapter = OpenCodeAdapter;
        let raw = json!({ "event_type": "permission.asked" });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::HumanInTheLoop);
    }

    #[test]
    fn deny_before_tool_throws() {
        let adapter = OpenCodeAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("blocked by policy".to_string()),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::BeforeTool, &response)
            .unwrap();
        assert_eq!(body["__action"], "throw");
        assert_eq!(body["message"], "blocked by policy");
    }

    #[test]
    fn allow_permission_mutates_status() {
        let adapter = OpenCodeAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Allow),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::PermissionRequest, &response)
            .unwrap();
        assert_eq!(body["__action"], "mutate");
        assert_eq!(body["status"], "allow");
    }
}
