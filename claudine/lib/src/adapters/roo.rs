use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;

use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};
use crate::permissions::query::{CommandQuery, PathQuery};
use crate::services::protect::intent::ProtectIntent;
use crate::services::protect::observe::default_observe_protect;
use crate::services::{ProtectDecision, ProtectObservation, ProtectOutcome};

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

    fn observe_protect(
        &self,
        event: &AgenticEvent,
        meta: &EventMeta,
    ) -> Option<ProtectObservation> {
        let mut obs = default_observe_protect(event, meta)?;

        if let Some(tool_name) = meta.tool_name.as_deref() {
            let lowered = tool_name.to_ascii_lowercase();
            let mut intents = Vec::new();
            let mut replaced = true;

            match lowered.as_str() {
                "execute_command" | "shell" => {
                    let cmd = meta.tool_input.as_ref().and_then(|v| {
                        v.get("command")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .or_else(|| v.as_str().map(ToOwned::to_owned))
                    });
                    if let Some(cmd) = cmd {
                        intents.push(ProtectIntent::ExecuteCommand(CommandQuery::from_raw(&cmd)));
                    }
                }
                "write_to_file" | "replace_in_file" | "insert_code_block"
                | "search_and_replace" => {
                    if let Some(path) = roo_tool_input_path(meta) {
                        intents.push(ProtectIntent::WritePath(PathQuery::file(&path)));
                    }
                }
                "read_file" | "list_files" | "search_files" | "list_code_definition_names" => {
                    if let Some(path) = roo_tool_input_path(meta) {
                        intents.push(ProtectIntent::ReadPath(PathQuery::unknown(&path)));
                    }
                }
                "use_mcp_tool" => {
                    if let Some(server) = meta
                        .tool_input
                        .as_ref()
                        .and_then(|v| v.get("server_name"))
                        .and_then(Value::as_str)
                    {
                        intents.push(ProtectIntent::UseMcpServer {
                            server: server.to_owned(),
                        });
                        if let Some(tool) = meta
                            .tool_input
                            .as_ref()
                            .and_then(|v| v.get("tool_name"))
                            .and_then(Value::as_str)
                        {
                            intents.push(ProtectIntent::UseMcpTool {
                                server: server.to_owned(),
                                tool: tool.to_owned(),
                            });
                        }
                    }
                }
                "switch_mode" => {
                    let target = meta
                        .tool_input
                        .as_ref()
                        .and_then(|v| v.get("mode_slug"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned);
                    intents.push(ProtectIntent::SwitchMode { target });
                }
                _ => {
                    replaced = false;
                }
            }

            if replaced {
                if obs
                    .intents
                    .iter()
                    .any(|i| matches!(i, ProtectIntent::CompletionOutputScan))
                {
                    intents.push(ProtectIntent::CompletionOutputScan);
                }
                obs.intents = intents;
            }
        }

        // Roo ModeChanged notification → SwitchMode intent.
        if matches!(event, AgenticEvent::Notification)
            && let Some(Value::String(mode)) = meta.extra.get("required_action")
            && mode.contains("mode")
        {
            obs.intents.push(ProtectIntent::SwitchMode {
                target: Some(mode.clone()),
            });
        }

        Some(obs)
    }

    fn can_block(&self, _event: &AgenticEvent) -> bool {
        false
    }

    fn non_blocking_ack(&self) -> Option<Value> {
        None // Roo Code hooks are fire-and-forget
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

    fn map_protect_outcome(
        &self,
        _event: &AgenticEvent,
        decision: &ProtectDecision,
    ) -> Result<HookResponse, AdapterError> {
        let mut reason = match &decision.outcome {
            ProtectOutcome::Allow => None,
            ProtectOutcome::AskThenAllowOrStop { reason }
            | ProtectOutcome::StopCurrent { reason }
            | ProtectOutcome::StopSession { reason }
            | ProtectOutcome::AllowWithRedaction { reason }
            | ProtectOutcome::AdvisoryOnly { reason } => Some(reason.clone()),
        };

        if decision.degraded {
            reason = Some(format!(
                "{} (roo: CLI hook channel cannot enforce block directly)",
                reason.unwrap_or_else(|| "protect decision".to_string())
            ));
        }

        Ok(HookResponse {
            decision: Some(crate::actions::HookDecision::Continue),
            reason,
            ..HookResponse::default()
        })
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

fn roo_tool_input_path(meta: &EventMeta) -> Option<String> {
    meta.tool_input
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|map| {
            map.get("path")
                .or_else(|| map.get("file_path"))
                .or_else(|| map.get("file"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
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
