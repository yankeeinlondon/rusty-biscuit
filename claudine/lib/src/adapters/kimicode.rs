use std::collections::HashMap;

use chrono::Utc;
use serde_json::{Value, json};

use crate::events::{
    AgenticEvent, EnvironmentContext, EventMeta, HookDecision, HookResponse, Provider,
};

use super::{AdapterError, ProviderAdapter};

pub(crate) struct KimiCodeAdapter;

impl ProviderAdapter for KimiCodeAdapter {
    fn provider(&self) -> Provider {
        Provider::KimiCode
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event_name = raw
            .get("event_name")
            .or_else(|| raw.get("event"))
            .or_else(|| raw.get("method"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("event_name"))?;

        let event = map_event(event_name, raw)?;
        let mut meta = EventMeta {
            provider: Provider::KimiCode,
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
            "step_number",
            "approval_id",
            "display_blocks",
            "content_variant",
            "subagent_nested_event_type",
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
            AgenticEvent::BeforeTool
                | AgenticEvent::PermissionRequest
                | AgenticEvent::HumanInTheLoop
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

        let decision = match response.decision.unwrap_or(HookDecision::Allow) {
            HookDecision::Allow | HookDecision::Continue => "approve",
            HookDecision::Deny => "reject",
            HookDecision::Ask => "ask",
        };

        Ok(json!({
            "decision": decision,
            "reason": response.reason,
        }))
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        None
    }
}

fn map_event(event_name: &str, raw: &Value) -> Result<AgenticEvent, AdapterError> {
    match event_name {
        "TurnBegin" => Ok(AgenticEvent::BeforePrompt),
        "TurnEnd" => Ok(AgenticEvent::TurnComplete),
        "StepBegin" => Ok(AgenticEvent::Notification),
        "StepInterrupted" => Ok(AgenticEvent::TurnError),
        "CompactionBegin" => Ok(AgenticEvent::BeforeCompact),
        "CompactionEnd" => Ok(AgenticEvent::Notification),
        "StatusUpdate" => Ok(AgenticEvent::Notification),
        "ContentPart" => Ok(AgenticEvent::AfterModel),
        "ToolCall" | "ToolCallPart" | "ToolCallRequest" => Ok(AgenticEvent::BeforeTool),
        "ToolResult" => {
            if raw
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                Ok(AgenticEvent::ToolError)
            } else {
                Ok(AgenticEvent::AfterTool)
            }
        }
        "ApprovalRequest" | "ApprovalResponse" => Ok(AgenticEvent::PermissionRequest),
        "SubagentEvent" => {
            let nested_type = raw
                .get("subagent_nested_event_type")
                .or_else(|| raw.get("nested_event_type"))
                .and_then(Value::as_str);
            match nested_type {
                Some("done") | Some("stop") => Ok(AgenticEvent::SubagentStop),
                _ => Ok(AgenticEvent::SubagentStart),
            }
        }
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
    fn approval_request_maps_permission_request() {
        let adapter = KimiCodeAdapter;
        let raw = json!({ "event_name": "ApprovalRequest", "approval_id": "a1" });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::PermissionRequest);
        assert_eq!(meta.extra["approval_id"], json!("a1"));
    }

    #[test]
    fn subagent_done_maps_stop() {
        let adapter = KimiCodeAdapter;
        let raw = json!({
            "event_name": "SubagentEvent",
            "subagent_nested_event_type": "done"
        });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::SubagentStop);
    }

    #[test]
    fn can_block_matches_matrix() {
        let adapter = KimiCodeAdapter;
        assert!(adapter.can_block(&AgenticEvent::BeforeTool));
        assert!(adapter.can_block(&AgenticEvent::PermissionRequest));
        assert!(!adapter.can_block(&AgenticEvent::AfterTool));
    }

    #[test]
    fn format_response_approve() {
        let adapter = KimiCodeAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Allow),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::PermissionRequest, &response)
            .unwrap();
        assert_eq!(body["decision"], "approve");
    }
}
