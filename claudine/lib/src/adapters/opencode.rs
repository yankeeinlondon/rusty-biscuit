use std::collections::HashMap;

use serde_json::{Value, json};
use tracing::debug;

use super::{AdapterError, ProviderAdapter, str_field};
use crate::actions::{HookDecision, HookResponse};
use crate::events::{AgenticEvent, EventMeta};
use crate::provider::Provider;

pub(crate) struct OpenCodeAdapter;

impl ProviderAdapter for OpenCodeAdapter {
    fn provider(&self) -> Provider {
        Provider::OpenCode
    }

    fn parse_event(&self, raw: &Value) -> Result<(AgenticEvent, EventMeta), AdapterError> {
        let event_type = raw
            .get("event_type")
            .or_else(|| raw.get("eventType"))
            .or_else(|| raw.get("type"))
            .or_else(|| raw.get("event"))
            .and_then(Value::as_str)
            .ok_or(AdapterError::MissingField("event_type"))?;

        let event = map_event(event_type)?;
        let properties = raw.get("properties");
        let event_context = properties.unwrap_or(raw);
        let info = event_context.get("info");
        let part = event_context.get("part");
        let mut meta = EventMeta::new(Provider::OpenCode, event);
        meta.session_id = str_field(raw, "session_id")
            .or_else(|| str_field(raw, "sessionId"))
            .or_else(|| str_field(raw, "sessionID"))
            .or_else(|| str_field(event_context, "session_id"))
            .or_else(|| str_field(event_context, "sessionId"))
            .or_else(|| str_field(event_context, "sessionID"))
            .or_else(|| str_field(info.unwrap_or(&Value::Null), "sessionID"))
            .or_else(|| str_field(info.unwrap_or(&Value::Null), "id"));
        meta.cwd = str_field(raw, "cwd")
            .or_else(|| value_path_string(raw, &["path", "cwd"]))
            .or_else(|| str_field(event_context, "directory"))
            .or_else(|| value_path_string(info.unwrap_or(&Value::Null), &["path", "cwd"]))
            .or_else(|| str_field(info.unwrap_or(&Value::Null), "directory"));
        meta.tool_name = str_field(raw, "tool_name")
            .or_else(|| str_field(raw, "toolName"))
            .or_else(|| str_field(raw, "tool"))
            .or_else(|| str_field(event_context, "tool_name"))
            .or_else(|| str_field(event_context, "toolName"))
            .or_else(|| str_field(event_context, "tool"));
        meta.tool_input = raw
            .get("tool_input")
            .cloned()
            .or_else(|| raw.get("args").cloned())
            .or_else(|| event_context.get("tool_input").cloned())
            .or_else(|| event_context.get("args").cloned());
        meta.tool_response = raw
            .get("tool_response")
            .cloned()
            .or_else(|| raw.get("output").cloned())
            .or_else(|| event_context.get("tool_response").cloned())
            .or_else(|| event_context.get("output").cloned());
        meta.error = extract_error(raw, event_context, event);
        meta.prompt = str_field(raw, "prompt");
        meta.agent_type = str_field(raw, "agent_type");
        meta.notification_type = if event == AgenticEvent::Notification {
            Some(event_type.to_string())
        } else {
            None
        };
        meta.notification_message =
            str_field(raw, "message").or_else(|| str_field(event_context, "message"));

        // Capture well-known structured fields into extra with semantic keys.
        if let Some(value) = properties {
            meta.extra.insert("properties".to_string(), value.clone());
        }

        if let Some(value) = info {
            meta.extra.insert("message_info".to_string(), value.clone());
            capture_usage_fields(&mut meta.extra, value);
        }

        if let Some(value) = part {
            meta.extra.insert("message_part".to_string(), value.clone());
            capture_usage_fields(&mut meta.extra, value);
        }

        // Capture all remaining raw keys into extra so we never lose data
        // from fields we don't explicitly extract.
        let extracted_keys: &[&str] = &[
            "event_type",
            "eventType",
            "type",
            "event",
            "session_id",
            "sessionId",
            "sessionID",
            "cwd",
            "tool_name",
            "toolName",
            "tool",
            "tool_input",
            "args",
            "tool_response",
            "output",
            "error",
            "prompt",
            "message",
            "agent_type",
            "properties",
            "path",
        ];
        if let Some(obj) = raw.as_object() {
            for (key, value) in obj {
                if !extracted_keys.contains(&key.as_str()) && !meta.extra.contains_key(key) {
                    meta.extra.insert(key.to_string(), value.clone());
                }
            }
        }

        Ok((event, meta))
    }

    fn can_block(&self, event: &AgenticEvent) -> bool {
        matches!(
            event,
            AgenticEvent::BeforeTool | AgenticEvent::PermissionRequest
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
            AgenticEvent::BeforeTool => {
                if matches!(response.decision, Some(HookDecision::Deny)) {
                    Ok(json!({
                        "__action": "throw",
                        "message": response.reason.clone().unwrap_or_else(|| "blocked by policy".to_string())
                    }))
                } else {
                    Ok(json!({
                        "__action": "mutate",
                        "status": "allow"
                    }))
                }
            }
            AgenticEvent::PermissionRequest => {
                let status = match response.decision.unwrap_or(HookDecision::Allow) {
                    HookDecision::Allow => "allow",
                    HookDecision::Deny => "deny",
                    HookDecision::Ask => "ask",
                    HookDecision::Continue => "allow",
                };
                Ok(json!({
                    "__action": "mutate",
                    "status": status,
                    "reason": response.reason,
                }))
            }
            _ => Ok(Value::Null),
        }
    }

    fn exit_code(&self, _event: &AgenticEvent, _response: &HookResponse) -> Option<i32> {
        None
    }
}

fn map_event(event_type: &str) -> Result<AgenticEvent, AdapterError> {
    if let Some(event) = Provider::OpenCode.event_from_shared_native_name(event_type) {
        return Ok(event);
    }

    match event_type {
        "chat.message" => Ok(AgenticEvent::BeforePrompt),
        "tool.execute.before" => Ok(AgenticEvent::BeforeTool),
        "tool.execute.after" => Ok(AgenticEvent::AfterTool),
        "message.updated" => Ok(AgenticEvent::AfterModel),
        "chat.params"
        | "chat.headers"
        | "experimental.chat.system.transform"
        | "experimental.chat.messages.transform" => Ok(AgenticEvent::BeforeModel),
        "experimental.text.complete" => Ok(AgenticEvent::AfterModel),
        other => Err(AdapterError::UnknownEvent(other.to_string())),
    }
}

/// Extract error details from an OpenCode event payload.
///
/// OpenCode `session.error` events carry a structured error object:
/// ```json
/// { "properties": { "error": { "name": "UnknownError", "data": { "message": "..." } } } }
/// ```
///
/// This function tries multiple paths to find error text, logging the raw
/// payload at DEBUG level for turn errors to aid future debugging.
fn extract_error(raw: &Value, event_context: &Value, event: AgenticEvent) -> Option<String> {
    if event == AgenticEvent::TurnError {
        debug!(
            raw_payload = %raw,
            event_context = %event_context,
            "OpenCode turn error payload"
        );
    }

    // 1. Simple top-level string: raw["error"]
    if let Some(error) = str_field(raw, "error") {
        return Some(error);
    }

    // 2. Structured error object: properties.error.data.message (OpenCode SDK format)
    let error_obj = event_context.get("error");
    if let Some(error_obj) = error_obj {
        let error_name = error_obj
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Error");
        let error_message = error_obj
            .get("data")
            .and_then(|data| data.get("message"))
            .and_then(Value::as_str);

        if let Some(message) = error_message {
            return Some(format!("{error_name}: {message}"));
        }

        // Fallback: error name only (e.g. MessageOutputLengthError has no message)
        if error_name != "Error" {
            return Some(error_name.to_string());
        }

        // Last resort: stringify the error object if it's non-null
        if !error_obj.is_null() {
            return Some(error_obj.to_string());
        }
    }

    // 3. Legacy path: properties.error.message (flat structure)
    if let Some(msg) = value_path_string(event_context, &["error", "message"]) {
        return Some(msg);
    }

    // 4. Simple string in event_context: properties.error (string)
    if let Some(error) = str_field(event_context, "error") {
        return Some(error);
    }

    None
}

fn value_path_string(raw: &Value, path: &[&str]) -> Option<String> {
    let mut current = raw;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(ToOwned::to_owned)
}

fn capture_usage_fields(extra: &mut HashMap<String, Value>, value: &Value) {
    if let (Some(provider_id), Some(model_id)) =
        (str_field(value, "providerID"), str_field(value, "modelID"))
    {
        extra.insert(
            "model".to_string(),
            Value::String(format!("{provider_id}/{model_id}")),
        );
        extra.insert("provider_id".to_string(), Value::String(provider_id));
        extra.insert("model_id".to_string(), Value::String(model_id));
    }

    if let Some(cost) = value.get("cost") {
        extra.insert("cost_usd".to_string(), cost.clone());
    }

    if let Some(finish) = value.get("finish") {
        extra.insert("finish".to_string(), finish.clone());
    }

    if let Some(tokens) = value.get("tokens") {
        extra.insert("token_usage".to_string(), normalize_token_usage(tokens));
    }
}

fn normalize_token_usage(tokens: &Value) -> Value {
    json!({
        "total": tokens.get("total").and_then(Value::as_u64),
        "input": tokens.get("input").and_then(Value::as_u64),
        "output": tokens.get("output").and_then(Value::as_u64),
        "reasoning": tokens.get("reasoning").and_then(Value::as_u64),
        "cache_read": tokens
            .get("cache")
            .and_then(|cache| cache.get("read"))
            .and_then(Value::as_u64),
        "cache_write": tokens
            .get("cache")
            .and_then(|cache| cache.get("write"))
            .and_then(Value::as_u64),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_permission_ask_is_blockable() {
        let adapter = OpenCodeAdapter;
        let raw = json!({ "event_type": "permission.ask" });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::PermissionRequest);
        assert!(adapter.can_block(&event));
    }

    #[test]
    fn parse_permission_asked_maps_to_human_in_loop() {
        let adapter = OpenCodeAdapter;
        let raw = json!({ "event_type": "permission.asked" });

        let (event, _) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::HumanInTheLoop);
    }

    #[test]
    fn deny_before_tool_throws() {
        let adapter = OpenCodeAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Deny),
            reason: Some("blocked by policy".to_string()),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::BeforeTool, &response)
            .unwrap();
        assert_eq!(body["__action"], "throw");
        assert_eq!(body["message"], "blocked by policy");
    }

    #[test]
    fn allow_permission_mutates_status() {
        let adapter = OpenCodeAdapter;
        let response = HookResponse {
            decision: Some(HookDecision::Allow),
            ..HookResponse::default()
        };

        let body = adapter
            .format_response(&AgenticEvent::PermissionRequest, &response)
            .unwrap();
        assert_eq!(body["__action"], "mutate");
        assert_eq!(body["status"], "allow");
    }

    #[test]
    fn parse_tool_bridge_payload_uses_tool_and_args_fields() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "event_type": "tool.execute.before",
            "session_id": "ses_123",
            "tool": "bash",
            "args": { "command": "git status" }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::BeforeTool);
        assert_eq!(meta.session_id.as_deref(), Some("ses_123"));
        assert_eq!(meta.tool_name.as_deref(), Some("bash"));
        assert_eq!(meta.tool_input, Some(json!({ "command": "git status" })));
    }

    #[test]
    fn parse_message_updated_extracts_usage_fields() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "type": "message.updated",
            "properties": {
                "info": {
                    "role": "assistant",
                    "sessionID": "ses_456",
                    "providerID": "minimax",
                    "modelID": "MiniMax-M2.5-highspeed",
                    "path": { "cwd": "/tmp/project" },
                    "cost": 0.00409764,
                    "finish": "stop",
                    "tokens": {
                        "total": 49712,
                        "input": 583,
                        "output": 294,
                        "reasoning": 0,
                        "cache": {
                            "read": 48479,
                            "write": 356
                        }
                    }
                }
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::AfterModel);
        assert_eq!(meta.session_id.as_deref(), Some("ses_456"));
        assert_eq!(meta.cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(meta.extra["model"], json!("minimax/MiniMax-M2.5-highspeed"));
        assert_eq!(meta.extra["cost_usd"], json!(0.00409764));
        assert_eq!(meta.extra["finish"], json!("stop"));
        assert_eq!(meta.extra["token_usage"]["total"], json!(49712));
        assert_eq!(meta.extra["token_usage"]["cache_read"], json!(48479));
        assert_eq!(meta.extra["token_usage"]["cache_write"], json!(356));
    }

    #[test]
    fn parse_session_error_extracts_structured_error() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "type": "session.error",
            "properties": {
                "sessionID": "ses_err_789",
                "error": {
                    "name": "UnknownError",
                    "data": {
                        "message": "something went wrong"
                    }
                }
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(meta.session_id.as_deref(), Some("ses_err_789"));
        assert_eq!(
            meta.error.as_deref(),
            Some("UnknownError: something went wrong")
        );
    }

    #[test]
    fn parse_session_error_api_error_with_status() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "type": "session.error",
            "properties": {
                "error": {
                    "name": "APIError",
                    "data": {
                        "message": "rate limit exceeded",
                        "statusCode": 429,
                        "isRetryable": true
                    }
                }
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(meta.error.as_deref(), Some("APIError: rate limit exceeded"));
    }

    #[test]
    fn parse_session_error_name_only_no_message() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "type": "session.error",
            "properties": {
                "error": {
                    "name": "MessageOutputLengthError",
                    "data": {}
                }
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(meta.error.as_deref(), Some("MessageOutputLengthError"));
    }

    #[test]
    fn parse_session_error_with_provider_auth_error() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "type": "session.error",
            "properties": {
                "error": {
                    "name": "ProviderAuthError",
                    "data": {
                        "providerID": "anthropic",
                        "message": "invalid API key"
                    }
                }
            }
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(
            meta.error.as_deref(),
            Some("ProviderAuthError: invalid API key")
        );
    }

    #[test]
    fn parse_session_error_with_no_error_field() {
        let adapter = OpenCodeAdapter;
        let raw = json!({
            "type": "session.error",
            "properties": {}
        });

        let (event, meta) = adapter.parse_event(&raw).unwrap();
        assert_eq!(event, AgenticEvent::TurnError);
        assert_eq!(meta.error, None);
    }
}
