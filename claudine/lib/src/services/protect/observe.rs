use serde_json::Value;

use crate::events::{AgenticEvent, EventMeta};
use crate::permissions::query::{CommandQuery, PathQuery};

use super::config::ProtectPhase;
use super::intent::ProtectIntent;

/// Observed context extracted from an event for protect evaluation.
#[derive(Debug, Clone)]
pub struct ProtectObservation {
    pub summary: Option<String>,
    pub intents: Vec<ProtectIntent>,
    pub runtime: RuntimeFacts,
    pub payload: Option<ProtectPayload>,
}

/// Runtime environment facts relevant to protect decisions.
#[derive(Debug, Clone, Default)]
pub struct RuntimeFacts {
    pub is_root: bool,
    pub has_sandbox: Option<bool>,
    pub bypass_mode: bool,
}

/// MCP payload attached to the observation.
#[derive(Debug, Clone)]
pub enum ProtectPayload {
    McpText(String),
    McpJson(Value),
}

/// Default observation logic that mirrors the current `ProtectInput::from_event_meta()`.
///
/// Produces a `ProtectObservation` with `ProtectIntent` variants instead of flat
/// fields. Provider-specific adapters can override for more precise extraction.
pub fn default_observe_protect(
    event: &AgenticEvent,
    meta: &EventMeta,
) -> Option<ProtectObservation> {
    // Map event to phase — return None for events we don't evaluate.
    let _phase = match event {
        AgenticEvent::BeforePrompt => ProtectPhase::BeforePrompt,
        AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest => ProtectPhase::BeforeTool,
        AgenticEvent::AfterTool | AgenticEvent::ToolError => ProtectPhase::AfterTool,
        AgenticEvent::TurnComplete => ProtectPhase::Completion,
        AgenticEvent::SubagentStart => ProtectPhase::SubagentStart,
        AgenticEvent::SubagentStop => ProtectPhase::SubagentStop,
        AgenticEvent::AfterModel => ProtectPhase::McpResponse,
        _ => return None,
    };

    let mut intents = Vec::new();

    // Extract command intent
    let command = meta
        .tool_input
        .as_ref()
        .and_then(extract_command_string)
        .or_else(|| meta.prompt.clone());

    if let Some(ref cmd) = command {
        intents.push(ProtectIntent::ExecuteCommand(CommandQuery::from_raw(cmd)));
    }

    // Extract path intents
    if let Some(Value::Object(map)) = meta.tool_input.as_ref() {
        for key in ["path", "file", "target"] {
            if let Some(path) = map.get(key).and_then(Value::as_str) {
                let is_write = is_write_event(event, meta);
                if is_write {
                    intents.push(ProtectIntent::WritePath(PathQuery::unknown(path)));
                } else {
                    intents.push(ProtectIntent::ReadPath(PathQuery::unknown(path)));
                }
            }
        }
    }

    // MCP server intent
    if let Some(server_id) = meta.extra.get("mcp_server_id").and_then(Value::as_str) {
        intents.push(ProtectIntent::UseMcpServer {
            server: server_id.to_owned(),
        });
    }

    // Subagent intent
    if matches!(event, AgenticEvent::SubagentStart) {
        intents.push(ProtectIntent::SpawnSubagent {
            name: meta.agent_type.clone(),
        });
    }

    // Completion scan intent
    if matches!(event, AgenticEvent::TurnComplete) {
        intents.push(ProtectIntent::CompletionOutputScan);
    }

    // Runtime facts
    let runtime = RuntimeFacts {
        is_root: is_probably_root(meta),
        has_sandbox: infer_sandbox_state(meta),
        bypass_mode: detect_bypass_mode(meta),
    };

    // Payload
    let payload = meta.tool_response.as_ref().map(|v| {
        if v.is_string() {
            ProtectPayload::McpText(v.as_str().unwrap_or_default().to_owned())
        } else {
            ProtectPayload::McpJson(v.clone())
        }
    });

    Some(ProtectObservation {
        summary: meta.notification_message.clone(),
        intents,
        runtime,
        payload,
    })
}

fn extract_command_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map
            .get("command")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        _ => None,
    }
}

fn is_write_event(event: &AgenticEvent, meta: &EventMeta) -> bool {
    let tool = meta.tool_name.as_deref().unwrap_or("");
    let lowered = tool.to_ascii_lowercase();
    matches!(event, AgenticEvent::BeforeTool)
        && (lowered.contains("write")
            || lowered.contains("edit")
            || lowered.contains("create")
            || lowered.contains("delete"))
}

fn is_probably_root(meta: &EventMeta) -> bool {
    meta.extra
        .get("uid")
        .and_then(Value::as_u64)
        .is_some_and(|uid| uid == 0)
        || meta
            .extra
            .get("is_root")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn infer_sandbox_state(meta: &EventMeta) -> Option<bool> {
    if let Some(value) = meta.extra.get("sandbox_enabled").and_then(Value::as_bool) {
        return Some(value);
    }
    meta.extra
        .get("sandbox_mode")
        .and_then(Value::as_str)
        .map(|mode| {
            let lowered = mode.to_ascii_lowercase();
            !(lowered.contains("none") || lowered.contains("off") || lowered.contains("danger"))
        })
}

fn detect_bypass_mode(meta: &EventMeta) -> bool {
    let keys = [
        "permission_mode",
        "approval_mode",
        "sandbox_mode",
        "execution_mode",
    ];
    for key in keys {
        if let Some(value) = meta.extra.get(key).and_then(Value::as_str) {
            let lowered = value.to_ascii_lowercase();
            if lowered.contains("yolo") || lowered.contains("bypass") || lowered.contains("danger")
            {
                return true;
            }
        }
    }
    false
}
