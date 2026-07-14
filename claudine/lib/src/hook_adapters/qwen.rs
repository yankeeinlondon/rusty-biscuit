use std::collections::HashMap;

use serde_json::{Value, json};

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

pub(crate) struct QwenAdapter;

impl ProviderAdapter for QwenAdapter {
    fn provider(&self) -> Provider {
        Provider::QwenCode
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event_name = raw
            .get("event_name")
            .or_else(|| raw.get("type"))
            .or_else(|| raw.get("event"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("event_name"))?;

        let event = map_event(event_name)?;
        let mut meta = EventMeta::new(Provider::QwenCode, event);
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
            "approval_mode",
            "permission_priority",
            "subagent_id",
            "can_use_tool_timeout_secs",
        ] {
            if let Some(value) = raw.get(key) {
                meta.extra.insert(key.to_string(), value.clone());
            }
        }

        // Capture usage from assistant-message and result-style payloads.
        // Qwen may nest usage at root or under a child object.
        capture_qwen_usage(&mut meta.extra, raw);

        Ok((event, meta))
    }

    fn can_block(&self, event: &AgenticEvent) -> bool {
        matches!(event, AgenticEvent::PermissionRequest)
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

        let allow = !matches!(response.decision, Some(HookDecision::Deny));
        Ok(json!({
            "decision": if allow { "allow" } else { "deny" },
            "reason": response.reason,
        }))
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        None
    }
}

fn map_event(event_name: &str) -> Result<AgenticEvent, AdapterError> {
    match event_name {
        "CanUseTool" => Ok(AgenticEvent::PermissionRequest),
        "SubagentPreToolUse" => Ok(AgenticEvent::BeforeTool),
        "SubagentPostToolUse" => Ok(AgenticEvent::AfterTool),
        "SubagentStop" => Ok(AgenticEvent::SubagentStop),
        "StreamSessionStart" => Ok(AgenticEvent::SessionStart),
        "StreamAssistantMessage" => Ok(AgenticEvent::AfterModel),
        "StreamResult" => Ok(AgenticEvent::SessionEnd),
        other => Err(AdapterError::UnknownEvent(other.to_string())),
    }
}

/// Capture and normalize Qwen usage fields.
///
/// Qwen headless stream emits `usage` on both `assistant` and `result` events.
/// The field may be at the root or nested under a child object. We preserve the
/// raw `usage` object and normalize into the shared `token_usage` shape.
///
/// Only writes `cost_usd` when the provider includes an actual cost value.
fn capture_qwen_usage(extra: &mut HashMap<String, Value>, raw: &Value) {
    let usage = raw
        .get("usage")
        .or_else(|| raw.get("result").and_then(|r| r.get("usage")));

    let Some(usage) = usage else { return };

    // Preserve raw provider-native object
    extra.insert("usage".to_string(), usage.clone());

    // Normalize — Qwen field names may vary; try common shapes
    let input = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .and_then(Value::as_u64);
    let output = usage
        .get("output_tokens")
        .or_else(|| usage.get("completion_tokens"))
        .and_then(Value::as_u64);
    let total = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .or_else(|| match (input, output) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        });
    let cache_read = usage
        .get("cached_tokens")
        .or_else(|| usage.get("cache_read_tokens"))
        .and_then(Value::as_u64);

    let normalized = json!({
        "total": total,
        "input": input,
        "output": output,
        "cache_read": cache_read,
    });
    extra.insert("token_usage".to_string(), normalized);

    // Only write cost_usd when provider gives us a real value
    if let Some(cost) = usage.get("cost").or_else(|| raw.get("cost"))
        && cost.is_number()
    {
        extra.insert("cost_usd".to_string(), cost.clone());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn can_use_tool_maps_permission_request() {
        let adapter = QwenAdapter;
        let raw = json!({
            "event_name": "CanUseTool",
            "can_use_tool_timeout_secs": 60
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::PermissionRequest);
        assert_eq!(meta.extra["can_use_tool_timeout_secs"], json!(60));
    }

    #[test]
    fn parse_assistant_message_with_usage_normalizes_token_usage() {
        let adapter = QwenAdapter;
        let raw = json!({
            "event_name": "StreamAssistantMessage",
            "usage": {
                "input_tokens": 2000,
                "output_tokens": 500,
                "total_tokens": 2500
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::AfterModel);

        // Raw usage preserved
        assert_eq!(meta.extra["usage"]["input_tokens"], json!(2000));

        // Normalized token_usage
        assert_eq!(meta.extra["token_usage"]["input"], json!(2000));
        assert_eq!(meta.extra["token_usage"]["output"], json!(500));
        assert_eq!(meta.extra["token_usage"]["total"], json!(2500));
    }

    #[test]
    fn parse_result_with_nested_usage() {
        let adapter = QwenAdapter;
        let raw = json!({
            "event_name": "StreamResult",
            "result": {
                "usage": {
                    "prompt_tokens": 1000,
                    "completion_tokens": 300,
                    "cached_tokens": 800
                }
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::SessionEnd);

        assert_eq!(meta.extra["token_usage"]["input"], json!(1000));
        assert_eq!(meta.extra["token_usage"]["output"], json!(300));
        assert_eq!(meta.extra["token_usage"]["cache_read"], json!(800));
        assert_eq!(meta.extra["token_usage"]["total"], json!(1300));
    }

    #[test]
    fn qwen_cost_usd_only_when_provider_gives_it() {
        let adapter = QwenAdapter;

        // Without cost
        let raw_no_cost = json!({
            "event_name": "StreamAssistantMessage",
            "usage": { "input_tokens": 100, "output_tokens": 50 }
        });
        let (_, meta) = adapter.parse_event(&raw_no_cost).unwrap();
        assert!(!meta.extra.contains_key("cost_usd"));

        // With cost
        let raw_with_cost = json!({
            "event_name": "StreamAssistantMessage",
            "usage": { "input_tokens": 100, "output_tokens": 50, "cost": 0.003 }
        });
        let (_, meta) = adapter.parse_event(&raw_with_cost).unwrap();
        assert_eq!(meta.extra["cost_usd"], json!(0.003));
    }

    #[test]
    fn qwen_no_usage_no_extra_keys() {
        let adapter = QwenAdapter;
        let raw = json!({ "event_name": "StreamSessionStart" });

        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert!(!meta.extra.contains_key("usage"));
        assert!(!meta.extra.contains_key("token_usage"));
    }

    #[test]
    fn only_permission_is_blocking() {
        let adapter = QwenAdapter;
        assert!(adapter.can_block(&AgenticEvent::PermissionRequest));
        assert!(!adapter.can_block(&AgenticEvent::AfterTool));
    }

    #[test]
    fn format_response_permission() {
        let adapter = QwenAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("blocked".to_string()),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::PermissionRequest, &response)
            .unwrap();
        assert_eq!(body["decision"], "deny");
    }
}
