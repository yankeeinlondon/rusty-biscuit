use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;

use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};

use super::{AdapterError, ProviderAdapter};

pub(crate) struct RooAdapter;

impl ProviderAdapter for RooAdapter {
    fn provider(&self) -> Provider {
        Provider::RooCode
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
            provider: Provider::RooCode,
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
            "agent_loop_state",
            "required_action",
            "cline_ask",
            "cost_info",
            "task_id",
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

fn map_event(event_name: &str) -> Result<AgenticEvent, AdapterError> {
    match event_name {
        "WaitingForInput" => Ok(AgenticEvent::HumanInTheLoop),
        "TaskCompleted" => Ok(AgenticEvent::TurnComplete),
        "Error" => Ok(AgenticEvent::TurnError),
        "StreamingStarted" => Ok(AgenticEvent::BeforeModel),
        "StreamingEnded" => Ok(AgenticEvent::AfterModel),
        "ToolUseOutput" => Ok(AgenticEvent::BeforeTool),
        "ToolResultOutput" => Ok(AgenticEvent::AfterTool),
        "TaskCreated" => Ok(AgenticEvent::SessionStart),
        "TaskAborted" => Ok(AgenticEvent::SessionEnd),
        "TaskSpawned" => Ok(AgenticEvent::SubagentStart),
        "TaskDelegationCompleted" => Ok(AgenticEvent::SubagentStop),
        "TaskToolFailed" => Ok(AgenticEvent::ToolError),
        "ModeChanged" => Ok(AgenticEvent::Notification),
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
    fn waiting_for_input_maps_human_in_loop() {
        let adapter = RooAdapter;
        let raw = json!({ "event_name": "WaitingForInput", "task_id": "t1" });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::HumanInTheLoop);
        assert_eq!(meta.extra["task_id"], json!("t1"));
    }

    #[test]
    fn never_blocks() {
        let adapter = RooAdapter;
        assert!(!adapter.can_block(&AgenticEvent::TurnComplete));
    }
}
