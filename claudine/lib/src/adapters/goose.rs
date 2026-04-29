use serde_json::Value;

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

pub(crate) struct GooseAdapter;

impl ProviderAdapter for GooseAdapter {
    fn provider(&self) -> Provider {
        Provider::Goose
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let kind = raw
            .get("type")
            .or_else(|| raw.get("event"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("type"))?;

        let event = map_event(kind, raw)?;
        let mut meta = EventMeta::new(Provider::Goose, event);
        meta.session_id = str_field(raw, "session_id");
        meta.cwd = str_field(raw, "cwd");
        meta.tool_name = str_field(raw, "tool_name");
        meta.tool_input = raw.get("tool_input").cloned();
        meta.tool_response = raw.get("tool_response").cloned();
        meta.error = str_field(raw, "error");
        meta.prompt = str_field(raw, "prompt");
        meta.agent_type = str_field(raw, "agent_type");
        meta.notification_type = str_field(raw, "notification_type");
        meta.notification_message = str_field(raw, "message");

        for key in [
            "goose_mode",
            "message_content_type",
            "total_tokens",
            "subagent_notification_type",
        ] {
            if let Some(value) = raw.get(key) {
                meta.extra.insert(key.to_string(), value.clone());
            }
        }

        Ok((event, meta))
    }

    fn can_block(&self, _event: &AgenticEvent) -> bool {
        false
    }

    fn non_blocking_ack(&self) -> Option<Value> {
        None // Goose hooks are fire-and-forget
    }

    fn format_response(
        &self,
        _event: &AgenticEvent,
        _response: &HookResponse,
    ) -> Result<Value, AdapterError> {
        Ok(Value::Null)
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        None
    }
}

fn map_event(kind: &str, raw: &Value) -> Result<AgenticEvent, AdapterError> {
    match kind {
        "StatusWaiting" | "status.waiting" => Ok(AgenticEvent::TurnComplete),
        "StatusThinking" | "status.thinking" => Ok(AgenticEvent::BeforeModel),
        "Message" | "message" => Ok(AgenticEvent::AfterModel),
        "Notification" | "notification" => {
            match raw
                .get("notification_type")
                .or_else(|| raw.get("subagent_notification_type"))
                .and_then(Value::as_str)
            {
                Some("subagent_tool_request") => Ok(AgenticEvent::SubagentStart),
                Some("tasks_complete") => Ok(AgenticEvent::SubagentStop),
                _ => Ok(AgenticEvent::Notification),
            }
        }
        "ModelChange" | "model.change" => Ok(AgenticEvent::Notification),
        "Error" | "error" => Ok(AgenticEvent::TurnError),
        "Complete" | "complete" => Ok(AgenticEvent::SessionEnd),
        other => Err(AdapterError::UnknownEvent(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_status_waiting() {
        let adapter = GooseAdapter;
        let raw = json!({ "type": "StatusWaiting" });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
    }

    #[test]
    fn notification_subagent_start() {
        let adapter = GooseAdapter;
        let raw = json!({
            "type": "Notification",
            "notification_type": "subagent_tool_request"
        });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::SubagentStart);
    }

    #[test]
    fn never_blocks() {
        let adapter = GooseAdapter;
        assert!(!adapter.can_block(&AgenticEvent::TurnComplete));
    }
}
