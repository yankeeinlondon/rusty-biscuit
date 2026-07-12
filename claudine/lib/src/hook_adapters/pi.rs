use std::collections::HashMap;

use serde_json::{Value, json};

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::HookResponse;
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

/// Hook-dispatch adapter for Pi.
///
/// Pi has no native hook system, so this adapter is never reached through hook
/// registration — Pi's live output is consumed by
/// [`crate::stream::providers::pi::PiSemanticStreamParser`] instead. It exists to
/// satisfy the per-provider [`ProviderAdapter`] contract and to give
/// `claudine handle --provider pi` a best-effort normalization of Pi's
/// `--mode json` records into the canonical event model.
pub(crate) struct PiAdapter;

impl ProviderAdapter for PiAdapter {
    fn provider(&self) -> Provider {
        Provider::Pi
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let kind = raw
            .get("type")
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("type"))?;

        let event = map_event(kind)?;
        let mut meta = EventMeta::new(Provider::Pi, event);
        meta.session_id = str_field(raw, "id").or_else(|| str_field(raw, "session_id"));
        meta.cwd = str_field(raw, "cwd");
        meta.tool_name = str_field(raw, "toolName");
        meta.tool_input = raw.get("args").cloned();
        meta.tool_response = raw.get("result").cloned();

        // Pi normalizes provider failures into the assistant message envelope,
        // so surface the free-text errorMessage from either location.
        meta.error = str_field(raw, "errorMessage")
            .or_else(|| raw.get("message").and_then(|m| str_field(m, "errorMessage")))
            .or_else(|| str_field(raw, "finalError"));

        capture_pi_usage(&mut meta.extra, raw);

        Ok((event, meta))
    }

    fn can_block(&self, _event: &AgenticEvent) -> bool {
        // Pi exposes no permission events in JSON mode and no hook-approval path.
        false
    }

    fn non_blocking_ack(&self) -> Option<Value> {
        None
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

fn map_event(kind: &str) -> Result<AgenticEvent, AdapterError> {
    match kind {
        "session" => Ok(AgenticEvent::SessionStart),
        "tool_execution_start" => Ok(AgenticEvent::BeforeTool),
        "tool_execution_end" => Ok(AgenticEvent::AfterTool),
        "message_end" => Ok(AgenticEvent::AfterModel),
        "agent_end" => Ok(AgenticEvent::TurnComplete),
        "compaction_start" => Ok(AgenticEvent::BeforeCompact),
        other => Err(AdapterError::UnknownEvent(other.to_string())),
    }
}

/// Capture Pi's per-message usage into the shared normalized shape.
///
/// In JSON mode usage rides on the assistant message (`message.usage`) with
/// `input`, `output`, `cacheRead`, `totalTokens`, and a nested `cost.total`.
fn capture_pi_usage(extra: &mut HashMap<String, Value>, raw: &Value) {
    let usage = raw
        .get("message")
        .and_then(|m| m.get("usage"))
        .or_else(|| raw.get("usage"));

    let Some(usage) = usage else { return };

    extra.insert("usage".to_string(), usage.clone());

    let input = usage.get("input").and_then(Value::as_u64);
    let output = usage.get("output").and_then(Value::as_u64);
    let cache_read = usage.get("cacheRead").and_then(Value::as_u64);
    let total = usage
        .get("totalTokens")
        .and_then(Value::as_u64)
        .or_else(|| match (input, output) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        });

    let normalized = json!({
        "total": total,
        "input": input,
        "output": output,
        "cache_read": cache_read,
    });
    extra.insert("token_usage".to_string(), normalized);

    if let Some(cost) = usage.get("cost").and_then(|c| c.get("total"))
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
    fn session_maps_session_start() {
        let adapter = PiAdapter;
        let raw = json!({"type": "session", "id": "s-1", "cwd": "/work"});
        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::SessionStart);
        assert_eq!(meta.session_id.as_deref(), Some("s-1"));
        assert_eq!(meta.cwd.as_deref(), Some("/work"));
    }

    #[test]
    fn tool_execution_start_maps_before_tool() {
        let adapter = PiAdapter;
        let raw = json!({"type": "tool_execution_start", "toolName": "bash", "args": {"command": "ls"}});
        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
        assert_eq!(meta.tool_name.as_deref(), Some("bash"));
    }

    #[test]
    fn message_end_normalizes_usage() {
        let adapter = PiAdapter;
        let raw = json!({
            "type": "message_end",
            "message": {
                "usage": {"input": 1200, "output": 150, "cacheRead": 300, "totalTokens": 1650,
                          "cost": {"total": 0.00594}},
                "stopReason": "stop"
            }
        });
        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::AfterModel);
        assert_eq!(meta.extra["token_usage"]["input"], json!(1200));
        assert_eq!(meta.extra["token_usage"]["output"], json!(150));
        assert_eq!(meta.extra["token_usage"]["cache_read"], json!(300));
        assert_eq!(meta.extra["token_usage"]["total"], json!(1650));
        assert_eq!(meta.extra["cost_usd"], json!(0.00594));
    }

    #[test]
    fn error_message_surfaces_from_message_envelope() {
        let adapter = PiAdapter;
        let raw = json!({
            "type": "message_end",
            "message": {"stopReason": "error", "errorMessage": "Provider returned error: 503"}
        });
        let (_, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(meta.error.as_deref(), Some("Provider returned error: 503"));
    }

    #[test]
    fn nothing_is_blocking() {
        let adapter = PiAdapter;
        assert!(!adapter.can_block(&AgenticEvent::BeforeTool));
        assert!(!adapter.can_block(&AgenticEvent::PermissionRequest));
    }

    #[test]
    fn unknown_event_errors() {
        let adapter = PiAdapter;
        let raw = json!({"type": "queue_update"});
        assert!(adapter.parse_event(&raw).is_err());
    }

    #[test]
    fn provider_is_pi() {
        assert_eq!(PiAdapter.provider(), Provider::Pi);
    }
}
