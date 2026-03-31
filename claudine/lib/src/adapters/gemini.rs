use std::collections::HashMap;

use chrono::Utc;
use serde_json::{Value, json};

use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EnvironmentContext, EventMeta, Provider};
use crate::permissions::query::{CommandQuery, DomainQuery, PathQuery};
use crate::services::protect::intent::ProtectIntent;
use crate::services::protect::observe::default_observe_protect;
use crate::services::ProtectObservation;

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

    fn observe_protect(
        &self,
        event: &AgenticEvent,
        meta: &EventMeta,
    ) -> Option<ProtectObservation> {
        let mut obs = default_observe_protect(event, meta)?;

        // Gemini-specific: extract MCP context and refine tool intents.
        let mut intents = Vec::new();
        let mut replaced = false;

        if let Some(tool_name) = meta.tool_name.as_deref() {
            let lowered = tool_name.to_ascii_lowercase();
            replaced = true;

            match lowered.as_str() {
                "shell" | "execute_command" | "run_command" => {
                    if let Some(cmd) = meta
                        .tool_input
                        .as_ref()
                        .and_then(|v| v.get("command"))
                        .and_then(Value::as_str)
                    {
                        intents.push(ProtectIntent::ExecuteCommand(CommandQuery::from_raw(cmd)));
                    }
                }
                "write_file" | "edit_file" | "create_file" | "patch_file" => {
                    if let Some(path) = tool_input_path(meta) {
                        intents.push(ProtectIntent::WritePath(PathQuery::file(&path)));
                    }
                }
                "read_file" | "list_directory" => {
                    if let Some(path) = tool_input_path(meta) {
                        intents.push(ProtectIntent::ReadPath(PathQuery::unknown(&path)));
                    }
                }
                "google_search" | "web_search" => {
                    if let Some(query) = meta
                        .tool_input
                        .as_ref()
                        .and_then(|v| v.get("query"))
                        .and_then(Value::as_str)
                    {
                        intents.push(ProtectIntent::AccessDomain(DomainQuery::new(query)));
                    }
                }
                _ => {
                    replaced = false;
                }
            }
        }

        // Extract MCP server/tool from mcp_context extra.
        if let Some(mcp) = meta.extra.get("mcp_context")
            && let Some(server) = mcp.get("server_id").and_then(Value::as_str)
        {
            intents.push(ProtectIntent::UseMcpServer {
                server: server.to_owned(),
            });
            if let Some(tool) = mcp.get("tool_name").and_then(Value::as_str) {
                intents.push(ProtectIntent::UseMcpTool {
                    server: server.to_owned(),
                    tool: tool.to_owned(),
                });
            }
            replaced = true;
        }

        if replaced {
            // Preserve completion scan intent if present.
            if obs.intents.iter().any(|i| matches!(i, ProtectIntent::CompletionOutputScan)) {
                intents.push(ProtectIntent::CompletionOutputScan);
            }
            obs.intents = intents;
        }

        Some(obs)
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
                body.insert(
                    "updatedToolResult".to_string(),
                    Value::String(ctx.clone()),
                );
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

fn str_field(raw: &Value, key: &str) -> Option<String> {
    raw.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn tool_input_path(meta: &EventMeta) -> Option<String> {
    meta.tool_input
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|map| {
            map.get("file_path")
                .or_else(|| map.get("path"))
                .or_else(|| map.get("file"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
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
