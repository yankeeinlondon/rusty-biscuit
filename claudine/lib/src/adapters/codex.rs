use std::collections::HashMap;

use serde_json::Value;

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

pub(crate) struct CodexAdapter;

impl ProviderAdapter for CodexAdapter {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let kind = event_kind(raw).ok_or(AdapterError::MissingField("type"))?;

        let (event, notification_type) = map_event(kind, raw)?;

        let hook_event = raw.get("hook_event");
        let item = raw.get("item");
        let mut meta = EventMeta::new(Provider::Codex, event);
        meta.session_id = str_field(raw, "thread_id")
            .or_else(|| str_field(raw, "thread-id"))
            .or_else(|| str_field(raw, "session_id"));
        meta.cwd = str_field(raw, "cwd");
        meta.tool_name = item
            .and_then(|value| value.get("name"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| str_field(hook_event.unwrap_or(raw), "tool_name"));
        meta.tool_input = item
            .and_then(|value| value.get("input"))
            .cloned()
            .or_else(|| {
                hook_event
                    .and_then(|value| value.get("tool_input"))
                    .cloned()
            });
        meta.tool_response = item
            .and_then(|value| value.get("output"))
            .cloned()
            .or_else(|| {
                hook_event
                    .and_then(|value| value.get("output_preview"))
                    .cloned()
            });
        meta.error = raw
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        meta.prompt = str_field(raw, "prompt");
        meta.notification_type = notification_type;

        for key in ["thread_id", "thread-id", "session_id", "triggered_at"] {
            if let Some(value) = raw.get(key) {
                meta.extra.insert(key.to_string(), value.clone());
            }
        }

        // Capture and normalize usage from both documented field names.
        // Codex `turn.completed` uses `usage`; some payloads use `token_usage`.
        capture_codex_usage(&mut meta.extra, raw);

        if let Some(value) = hook_event {
            meta.extra.insert("hook_event".to_string(), value.clone());
        }

        if let Some(item_value) = item {
            if let Some(item_type) = item_value.get("type") {
                meta.extra
                    .insert("item_type".to_string(), item_type.clone());
            }
            if let Some(item_id) = item_value.get("id") {
                meta.extra.insert("item_id".to_string(), item_id.clone());
            }
        }

        Ok((event, meta))
    }

    fn can_block(&self, _event: &AgenticEvent) -> bool {
        false
    }

    fn non_blocking_ack(&self) -> Option<Value> {
        None // Codex uses notify endpoints, not stdout
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

fn map_event(kind: &str, raw: &Value) -> Result<(AgenticEvent, Option<String>), AdapterError> {
    match kind {
        "agent-turn-complete" | "AfterAgent" | "turn.completed" | "TurnCompleted" => {
            Ok((AgenticEvent::TurnComplete, None))
        }
        "after_tool_use" | "AfterToolUse" | "item.completed" | "ItemCompleted" => {
            Ok((AgenticEvent::AfterTool, None))
        }
        "ThreadStarted" | "thread.started" => Ok((AgenticEvent::SessionStart, None)),
        "TurnStarted" | "turn.started" => Ok((AgenticEvent::BeforePrompt, None)),
        "TurnFailed" | "turn.failed" | "Error" | "error" => Ok((AgenticEvent::TurnError, None)),
        "ItemStarted" | "item.started" => {
            if is_tool_item(raw) {
                Ok((AgenticEvent::BeforeTool, None))
            } else {
                Err(AdapterError::UnknownEvent(kind.to_string()))
            }
        }
        "ItemUpdated" | "item.updated" => {
            let item_type = raw
                .get("item")
                .and_then(|item| item.get("type"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);

            let event = match item_type.as_deref() {
                Some("reasoning") | Some("web_search") | Some("plan_update") => {
                    AgenticEvent::Notification
                }
                _ => AgenticEvent::AfterModel,
            };
            Ok((event, item_type))
        }
        other => Err(AdapterError::UnknownEvent(other.to_string())),
    }
}

fn is_tool_item(raw: &Value) -> bool {
    matches!(
        raw.get("item")
            .and_then(|item| item.get("type"))
            .and_then(Value::as_str),
        Some("command_execution") | Some("file_change") | Some("mcp_tool_call")
    ) || raw
        .get("hook_event")
        .and_then(|event| event.get("tool_name"))
        .is_some()
}

fn event_kind(raw: &Value) -> Option<&str> {
    raw.get("type")
        .or_else(|| raw.get("event"))
        .or_else(|| raw.get("event_type"))
        .or_else(|| {
            raw.get("hook_event")
                .and_then(|value| value.get("event_type"))
        })
        .and_then(Value::as_str)
}

/// Capture Codex usage fields and normalize into `token_usage`.
///
/// Codex documents two usage shapes:
/// - `usage` on `turn.completed`: `{ input_tokens, cached_input_tokens, output_tokens }`
/// - `token_usage` (legacy/alternative): opaque object preserved as-is
///
/// When `usage` is present, we normalize into the shared `token_usage` shape and
/// also preserve the raw `usage` object for future reference.
fn capture_codex_usage(extra: &mut HashMap<String, Value>, raw: &Value) {
    if let Some(usage) = raw.get("usage") {
        // Preserve raw provider-native object
        extra.insert("usage".to_string(), usage.clone());

        let input = usage.get("input_tokens").and_then(Value::as_u64);
        let cached = usage.get("cached_input_tokens").and_then(Value::as_u64);
        let output = usage.get("output_tokens").and_then(Value::as_u64);

        // Codex docs say input_tokens already includes cached_input_tokens,
        // so total = input_tokens + output_tokens.
        let total = match (input, output) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };

        let normalized = serde_json::json!({
            "total": total,
            "input": input,
            "output": output,
            "cache_read": cached,
        });
        extra.insert("token_usage".to_string(), normalized);
    } else if let Some(token_usage) = raw.get("token_usage") {
        // Legacy/alternative shape — preserve as-is
        extra.insert("token_usage".to_string(), token_usage.clone());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_item_started_tool_to_before_tool() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "item.started",
            "item": { "type": "command_execution", "name": "shell" }
        });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
    }

    #[test]
    fn parse_item_updated_reasoning_to_notification() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "item.updated",
            "item": { "type": "reasoning", "id": "i1" }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::Notification);
        assert_eq!(meta.notification_type.as_deref(), Some("reasoning"));
    }

    #[test]
    fn parse_adds_extra_fields() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "thread.started",
            "thread_id": "t1",
            "token_usage": { "in": 1, "out": 2 }
        });

        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(meta.extra["thread_id"], json!("t1"));
        assert_eq!(meta.extra["token_usage"]["in"], json!(1));
    }

    #[test]
    fn parse_legacy_notify_payload_uses_kebab_case_thread_id() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "agent-turn-complete",
            "thread-id": "t1",
            "turn-id": "turn-1",
            "cwd": "/tmp"
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);
        assert_eq!(meta.session_id.as_deref(), Some("t1"));
        assert_eq!(meta.extra["thread-id"], json!("t1"));
    }

    #[test]
    fn parse_nested_after_tool_use_payload() {
        let adapter = CodexAdapter;
        let raw = json!({
            "session_id": "ses_123",
            "cwd": "/tmp/project",
            "triggered_at": "2026-02-11T00:00:00Z",
            "hook_event": {
                "event_type": "after_tool_use",
                "tool_name": "local_shell",
                "tool_input": {
                    "input_type": "local_shell",
                    "params": { "command": ["cargo", "fmt"] }
                },
                "output_preview": "ok"
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::AfterTool);
        assert_eq!(meta.session_id.as_deref(), Some("ses_123"));
        assert_eq!(meta.tool_name.as_deref(), Some("local_shell"));
        assert_eq!(meta.tool_response, Some(json!("ok")));
        assert_eq!(
            meta.extra["hook_event"]["event_type"],
            json!("after_tool_use")
        );
    }

    #[test]
    fn parse_turn_completed_with_usage_normalizes_token_usage() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "turn.completed",
            "usage": {
                "input_tokens": 1500,
                "cached_input_tokens": 1000,
                "output_tokens": 200
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnComplete);

        // Raw usage preserved
        assert_eq!(meta.extra["usage"]["input_tokens"], json!(1500));
        assert_eq!(meta.extra["usage"]["cached_input_tokens"], json!(1000));
        assert_eq!(meta.extra["usage"]["output_tokens"], json!(200));

        // Normalized token_usage
        assert_eq!(meta.extra["token_usage"]["input"], json!(1500));
        assert_eq!(meta.extra["token_usage"]["output"], json!(200));
        assert_eq!(meta.extra["token_usage"]["cache_read"], json!(1000));
        assert_eq!(meta.extra["token_usage"]["total"], json!(1700));
    }

    #[test]
    fn parse_legacy_token_usage_preserved_as_is() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "turn.completed",
            "token_usage": { "in": 100, "out": 50 }
        });

        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(meta.extra["token_usage"]["in"], json!(100));
        assert_eq!(meta.extra["token_usage"]["out"], json!(50));
        // No "usage" key when only token_usage present
        assert!(!meta.extra.contains_key("usage"));
    }

    #[test]
    fn parse_usage_preferred_over_token_usage() {
        let adapter = CodexAdapter;
        let raw = json!({
            "type": "turn.completed",
            "usage": { "input_tokens": 500, "output_tokens": 100 },
            "token_usage": { "in": 1, "out": 2 }
        });

        let (_, meta) = adapter.parse_event(&raw).unwrap();
        // When both present, `usage` wins and gets normalized
        assert_eq!(meta.extra["token_usage"]["input"], json!(500));
        assert_eq!(meta.extra["token_usage"]["output"], json!(100));
        assert_eq!(meta.extra["usage"]["input_tokens"], json!(500));
    }

    #[test]
    fn never_blocks() {
        let adapter = CodexAdapter;
        assert!(!adapter.can_block(&AgenticEvent::BeforeTool));
    }
}
