//! Kimi Code adapter for the legacy stdin-fed hook surface (`claudine
//! handle`). Kimi's `--wire` JSON-RPC mode has its own dispatch path
//! ([`crate::dispatch`] called from the CLI's `wire_io` module) that
//! resolves canonical events through `wire_io::map_kimi_hook_event`. The
//! two paths share the canonical [`AgenticEvent`] surface but stay
//! independent because:
//!
//! - This adapter parses Kimi *event* names (`TurnBegin`, `ToolCall`,
//!   `ToolResult`, …) emitted by Kimi's hook system on stdin.
//! - The wire-mode dispatch parses Kimi *hook* event names
//!   (`PreToolUse`, `PostToolUse`, `Stop`, …) carried inside a wire
//!   `HookRequest` envelope's `payload.event` field.
//!
//! The canonical event names overlap deliberately so a Kimi hook payload
//! routed through either entry point reaches the same dispatch
//! pipeline. See `wire_io::map_kimi_hook_event` for the wire-mode
//! mapping.

use std::collections::HashMap;

use serde_json::{Value, json};

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

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
        let mut meta = EventMeta::new(Provider::KimiCode, event);
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

        // Capture usage from StatusUpdate events (flat or nested under `payload`).
        let usage_source = raw.get("payload").filter(|p| p.is_object()).unwrap_or(raw);
        capture_kimi_usage(&mut meta.extra, usage_source);

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
        // Kimi wire/stream event names.
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
        // Kimi hook event names — payloads delivered through
        // `claudine handle` from the legacy hook surface use the same
        // names that wire mode's `HookRequest.payload.event` carries.
        // Mapping them here keeps the two transport paths aligned on a
        // single canonical event surface.
        "PreToolUse" => Ok(AgenticEvent::BeforeTool),
        "PostToolUse" => Ok(AgenticEvent::AfterTool),
        "UserPromptSubmit" => Ok(AgenticEvent::BeforePrompt),
        "Stop" => Ok(AgenticEvent::TurnComplete),
        "SessionStart" => Ok(AgenticEvent::SessionStart),
        "SessionEnd" => Ok(AgenticEvent::SessionEnd),
        "Notification" => Ok(AgenticEvent::Notification),
        "SubagentStart" => Ok(AgenticEvent::SubagentStart),
        "SubagentStop" => Ok(AgenticEvent::SubagentStop),
        other => Err(AdapterError::UnknownEvent(other.to_string())),
    }
}

/// Capture and normalize Kimi Code usage fields.
///
/// Kimi `StatusUpdate` may include:
/// - `context_usage`: context-window utilization (promoted to top-level extra)
/// - `token_usage`: `{ input_other, output, input_cache_read, input_cache_creation }`
///
/// Both raw objects are preserved, and `token_usage` is additionally normalized
/// into the shared shape.
fn capture_kimi_usage(extra: &mut HashMap<String, Value>, source: &Value) {
    if let Some(context_usage) = source.get("context_usage") {
        extra.insert("context_usage".to_string(), context_usage.clone());
    }

    if let Some(token_usage) = source.get("token_usage") {
        // Preserve raw provider-native object
        extra.insert("raw_token_usage".to_string(), token_usage.clone());

        let input_other = token_usage.get("input_other").and_then(Value::as_u64);
        let output = token_usage.get("output").and_then(Value::as_u64);
        let cache_read = token_usage.get("input_cache_read").and_then(Value::as_u64);
        let cache_write = token_usage
            .get("input_cache_creation")
            .and_then(Value::as_u64);

        // Compute input as sum of all input components when all are present
        let input = match (input_other, cache_read, cache_write) {
            (Some(o), Some(cr), Some(cw)) => Some(o + cr + cw),
            _ => input_other,
        };

        // Compute total only when input and output are both known
        let total = match (input, output) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };

        let normalized = json!({
            "total": total,
            "input": input,
            "output": output,
            "cache_read": cache_read,
            "cache_write": cache_write,
        });
        extra.insert("token_usage".to_string(), normalized);
    }
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
    fn status_update_captures_token_and_context_usage() {
        let adapter = KimiCodeAdapter;
        let raw = json!({
            "event_name": "StatusUpdate",
            "context_usage": { "used": 8000, "total": 128000 },
            "token_usage": {
                "input_other": 500,
                "output": 300,
                "input_cache_read": 7000,
                "input_cache_creation": 200
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::Notification);

        // Context usage promoted to top-level
        assert_eq!(meta.extra["context_usage"]["used"], json!(8000));
        assert_eq!(meta.extra["context_usage"]["total"], json!(128000));

        // Raw token_usage preserved
        assert_eq!(meta.extra["raw_token_usage"]["input_other"], json!(500));

        // Normalized token_usage
        assert_eq!(meta.extra["token_usage"]["output"], json!(300));
        assert_eq!(meta.extra["token_usage"]["cache_read"], json!(7000));
        assert_eq!(meta.extra["token_usage"]["cache_write"], json!(200));
        // input = input_other + cache_read + cache_creation = 500 + 7000 + 200
        assert_eq!(meta.extra["token_usage"]["input"], json!(7700));
        // total = input + output = 7700 + 300
        assert_eq!(meta.extra["token_usage"]["total"], json!(8000));
    }

    #[test]
    fn status_update_nested_payload_captures_usage() {
        let adapter = KimiCodeAdapter;
        let raw = json!({
            "event_name": "StatusUpdate",
            "payload": {
                "context_usage": { "used": 4000, "total": 64000 },
                "token_usage": {
                    "input_other": 100,
                    "output": 50,
                    "input_cache_read": 3000,
                    "input_cache_creation": 100
                }
            }
        });

        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(meta.extra["context_usage"]["used"], json!(4000));
        assert_eq!(meta.extra["token_usage"]["output"], json!(50));
        assert_eq!(meta.extra["token_usage"]["cache_read"], json!(3000));
    }

    #[test]
    fn no_cost_usd_without_provider_cost() {
        let adapter = KimiCodeAdapter;
        let raw = json!({
            "event_name": "StatusUpdate",
            "token_usage": { "input_other": 100, "output": 50 }
        });

        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert!(!meta.extra.contains_key("cost_usd"));
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

    #[test]
    fn hook_event_names_resolve_to_canonical_events() {
        // Phase 5 contract: Kimi hook event names (carried inside wire
        // `HookRequest.payload.event` and on the legacy stdin hook
        // surface) must resolve to the same canonical [`AgenticEvent`]
        // variants the wire-mode dispatch path uses. Keeps both
        // transports aligned so dispatch policy doesn't depend on which
        // entry point the hook came through.
        let adapter = KimiCodeAdapter;
        for (name, expected) in [
            ("PreToolUse", AgenticEvent::BeforeTool),
            ("PostToolUse", AgenticEvent::AfterTool),
            ("UserPromptSubmit", AgenticEvent::BeforePrompt),
            ("Stop", AgenticEvent::TurnComplete),
            ("SessionStart", AgenticEvent::SessionStart),
            ("SessionEnd", AgenticEvent::SessionEnd),
            ("Notification", AgenticEvent::Notification),
            ("SubagentStart", AgenticEvent::SubagentStart),
            ("SubagentStop", AgenticEvent::SubagentStop),
        ] {
            let raw = json!({ "event_name": name });
            let (event, _) = adapter.parse_event(&raw).unwrap_or_else(|err| {
                panic!("expected {name} to map to a canonical event, got {err:?}")
            });
            assert_eq!(event, expected, "{name} should map to {expected:?}");
        }
    }
}
