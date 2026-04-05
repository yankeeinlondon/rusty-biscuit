use serde_json::{Map, Value, json};

use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EventMeta, Provider, ToolName};
use crate::permissions::query::{CommandQuery, PathQuery};
use crate::services::protect::intent::ProtectIntent;
use crate::services::protect::observe::default_observe_protect;

use super::{
    AdapterError, ProviderAdapter, extract_tool_input_path, replace_intents_preserving_completion,
    str_field,
};

pub(crate) struct ClaudeAdapter;

impl ProviderAdapter for ClaudeAdapter {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event_name = raw
            .get("hook_event_name")
            .or_else(|| raw.get("hookEventName"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("hook_event_name"))?;

        let mut event = map_event(event_name)?;
        let tool_name = str_field(raw, "tool_name").or_else(|| str_field(raw, "toolName"));

        // Adapter-level remapping: PreToolUse with AskUserQuestion → HumanInTheLoop
        if event == AgenticEvent::BeforeTool && tool_name.as_deref() == Some("AskUserQuestion") {
            event = AgenticEvent::HumanInTheLoop;
        }

        let mut meta = EventMeta::new(Provider::Claude, event);
        meta.session_id = str_field(raw, "session_id");
        meta.cwd = str_field(raw, "cwd");
        meta.tool_name = tool_name;
        meta.tool_input = raw.get("tool_input").cloned();
        meta.tool_response = raw.get("tool_response").cloned();
        meta.error = str_field(raw, "error");
        meta.prompt = str_field(raw, "prompt");
        meta.agent_type = str_field(raw, "agent_type");
        meta.notification_type = raw
            .get("notification")
            .and_then(|n| n.get("type"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        meta.notification_message = raw
            .get("notification")
            .and_then(|n| n.get("message"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);

        for key in [
            "permission_mode",
            "stop_hook_active",
            "transcript_path",
            "tool_use_id",
            "model",
            "is_interrupt",
            "permission_suggestions",
            "teammate_name",
            "team_name",
            "task_id",
            "task_subject",
            "task_description",
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
    ) -> Option<crate::services::ProtectObservation> {
        // Start with default observation, then refine for Claude-specific tools.
        let mut obs = default_observe_protect(event, meta)?;

        // Claude-specific tool intent extraction
        if let Some(tool_name) = meta.tool_name.as_deref() {
            let lowered = tool_name.to_ascii_lowercase();

            // Clear default intents and rebuild with Claude-specific knowledge.
            let mut intents = Vec::new();

            match lowered.as_str() {
                "write" | "write_file" => {
                    if let Some(path) = extract_tool_input_path(meta) {
                        intents.push(ProtectIntent::WritePath(PathQuery::file(path)));
                    }
                }
                "edit" | "edit_file" => {
                    if let Some(path) = extract_tool_input_path(meta) {
                        intents.push(ProtectIntent::WritePath(PathQuery::file(path)));
                    }
                }
                "read" | "read_file" | "read_directory" => {
                    if let Some(path) = extract_tool_input_path(meta) {
                        intents.push(ProtectIntent::ReadPath(PathQuery::unknown(path)));
                    }
                }
                "bash" | "execute_command" => {
                    if let Some(cmd) = extract_claude_command(meta) {
                        intents.push(ProtectIntent::ExecuteCommand(CommandQuery::from_raw(cmd)));
                    }
                }
                name if ToolName(name.to_owned()).is_mcp_tool() => {
                    if let Some((server_name, tool_name)) =
                        ToolName(name.to_owned()).mcp_components()
                    {
                        let server = server_name.to_owned();
                        intents.push(ProtectIntent::UseMcpServer {
                            server: server.clone(),
                        });
                        intents.push(ProtectIntent::UseMcpTool {
                            server,
                            tool: tool_name.to_owned(),
                        });
                    }
                }
                _ => {
                    // Keep default intents for unknown tools.
                    return Some(obs);
                }
            }

            replace_intents_preserving_completion(&mut obs, intents);
        }

        Some(obs)
    }

    fn can_block(&self, event: &AgenticEvent) -> bool {
        matches!(
            event,
            AgenticEvent::BeforePrompt
                | AgenticEvent::BeforeTool
                | AgenticEvent::AfterTool
                | AgenticEvent::PermissionRequest
                | AgenticEvent::SubagentStop
                | AgenticEvent::TurnComplete
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
            AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => {
                let permission = decision_to_permission(response.decision);
                let mut hook_output = Map::new();
                hook_output.insert(
                    "hookEventName".to_string(),
                    Value::String("PreToolUse".to_string()),
                );
                hook_output.insert(
                    "permissionDecision".to_string(),
                    Value::String(permission.to_string()),
                );
                if let Some(reason) = &response.reason {
                    hook_output.insert(
                        "permissionDecisionReason".to_string(),
                        Value::String(reason.clone()),
                    );
                }
                Ok(json!({ "hookSpecificOutput": hook_output }))
            }
            AgenticEvent::TurnComplete | AgenticEvent::SubagentStop => {
                let mut body = Map::new();
                if matches!(
                    response.decision,
                    Some(HookDecision::Continue | HookDecision::Deny)
                ) {
                    body.insert("decision".to_string(), Value::String("block".to_string()));
                }
                if let Some(reason) = &response.reason {
                    body.insert("reason".to_string(), Value::String(reason.clone()));
                }
                Ok(Value::Object(body))
            }
            AgenticEvent::AfterTool => {
                // Post-tool responses carry redacted content back to the provider.
                let mut body = Map::new();
                if matches!(response.decision, Some(HookDecision::Deny)) {
                    body.insert("decision".to_string(), Value::String("block".to_string()));
                }
                if let Some(reason) = &response.reason {
                    body.insert("reason".to_string(), Value::String(reason.clone()));
                }
                if let Some(ref ctx) = response.additional_context {
                    body.insert("updatedToolResult".to_string(), Value::String(ctx.clone()));
                }
                if let Some(ref input) = response.updated_input {
                    body.insert("updatedToolResult".to_string(), input.clone());
                }
                if body.is_empty() {
                    Ok(Value::Null)
                } else {
                    Ok(Value::Object(body))
                }
            }
            _ => Ok(Value::Null),
        }
    }

    fn exit_code(&self, event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        if self.can_block(event) { Some(0) } else { None }
    }
}

fn map_event(event_name: &str) -> Result<AgenticEvent, AdapterError> {
    Provider::Claude
        .event_from_shared_native_name(event_name)
        .ok_or_else(|| AdapterError::UnknownEvent(event_name.to_string()))
}

fn extract_claude_command(meta: &EventMeta) -> Option<String> {
    meta.tool_input.as_ref().and_then(|v| {
        v.get("command")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| v.as_str().map(ToOwned::to_owned))
    })
}

fn decision_to_permission(decision: Option<HookDecision>) -> &'static str {
    match decision.unwrap_or(HookDecision::Allow) {
        HookDecision::Allow => "allow",
        HookDecision::Deny => "deny",
        HookDecision::Ask => "ask",
        HookDecision::Continue => "allow",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_maps_turn_complete_aliases() {
        let adapter = ClaudeAdapter;
        let raw = json!({ "hook_event_name": "TaskCompleted", "session_id": "s1" });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
        assert_eq!(meta.session_id.as_deref(), Some("s1"));
    }

    #[test]
    fn parse_captures_extra_fields() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "permission_mode": "acceptEdits",
            "stop_hook_active": true
        });

        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(meta.extra["permission_mode"], json!("acceptEdits"));
        assert_eq!(meta.extra["stop_hook_active"], json!(true));
    }

    #[test]
    fn can_block_matches_matrix() {
        let adapter = ClaudeAdapter;
        assert!(adapter.can_block(&AgenticEvent::BeforeTool));
        assert!(adapter.can_block(&AgenticEvent::TurnComplete));
        assert!(!adapter.can_block(&AgenticEvent::Notification));
    }

    #[test]
    fn parse_pretooluse_ask_user_question_remaps_to_human_in_the_loop() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "AskUserQuestion",
            "session_id": "s1"
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::HumanInTheLoop);
        assert_eq!(meta.event, AgenticEvent::HumanInTheLoop);
        assert_eq!(meta.tool_name.as_deref(), Some("AskUserQuestion"));
    }

    #[test]
    fn parse_pretooluse_bash_stays_before_tool() {
        let adapter = ClaudeAdapter;
        let raw = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "session_id": "s1"
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
        assert_eq!(meta.event, AgenticEvent::BeforeTool);
        assert_eq!(meta.tool_name.as_deref(), Some("Bash"));
    }

    #[test]
    fn format_permission_response() {
        let adapter = ClaudeAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("blocked".to_string()),
            ..HookResponse::default()
        };

        let value = adapter
            .format_response(&AgenticEvent::PermissionRequest, &response)
            .unwrap();

        assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "deny");
        assert_eq!(
            value["hookSpecificOutput"]["permissionDecisionReason"],
            "blocked"
        );
    }
}
