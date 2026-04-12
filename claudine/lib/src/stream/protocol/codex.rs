//! Typed event models for Codex CLI's `exec --json` JSONL format.
//!
//! Codex uses dotted event names (`thread.started`, `turn.completed`, etc.)
//! which serde handles through `#[serde(rename = "...")]` on each variant.
//! The parser falls back to `Value`-based skipping for any event type that
//! is not enumerated here.

use serde::Deserialize;
use serde_json::Value;

/// Tagged enum over all Codex CLI stream event variants dispatched by the
/// parser. Unknown event types fail typed deserialization and are handled by
/// the parser's fallback arm.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum CodexEvent {
    #[serde(rename = "thread.created")]
    ThreadCreated(CodexThreadMeta),
    #[serde(rename = "thread.started")]
    ThreadStarted(CodexThreadMeta),
    #[serde(rename = "turn.started")]
    TurnStarted(CodexTurnStarted),
    #[serde(rename = "turn.completed")]
    TurnCompleted(CodexTurnCompleted),
    #[serde(rename = "error")]
    Error(CodexErrorEnvelope),
    #[serde(rename = "turn.error")]
    TurnError(CodexErrorEnvelope),
    #[serde(rename = "turn.failed")]
    TurnFailed(CodexErrorEnvelope),
    #[serde(rename = "stream.error")]
    StreamError(CodexErrorEnvelope),
    #[serde(rename = "item.started")]
    ItemStarted(CodexItemEnvelope),
    #[serde(rename = "item.completed")]
    ItemCompleted(CodexItemEnvelope),
    #[serde(rename = "item.tool_use")]
    ItemToolUse(CodexItem),
    #[serde(rename = "tool_use")]
    ToolUse(CodexItem),
    #[serde(rename = "item.tool_result")]
    ItemToolResult(CodexItem),
    #[serde(rename = "tool_result")]
    ToolResult(CodexItem),
}

/// `thread.created` / `thread.started` payload. Some Codex builds emit
/// `thread_id`, others use `id`.
#[derive(Debug, Default, Deserialize)]
pub struct CodexThreadMeta {
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

impl CodexThreadMeta {
    pub fn resolved_id(self) -> Option<String> {
        self.thread_id.or(self.id)
    }
}

/// Placeholder struct for `turn.started` events. Empty today so that unknown
/// fields are silently tolerated.
#[derive(Debug, Default, Deserialize)]
pub struct CodexTurnStarted {}

/// `turn.completed` event — carries token usage, duration, and stop reason.
#[derive(Debug, Default, Deserialize)]
pub struct CodexTurnCompleted {
    #[serde(default)]
    pub usage: Option<CodexUsage>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

impl CodexTurnCompleted {
    pub fn provider_status(&self) -> Option<&str> {
        self.status.as_deref().or(self.stop_reason.as_deref())
    }
}

/// Token usage block reported by `turn.completed`. Codex builds differ on
/// whether they send `cached_input_tokens` or `cache_read_input_tokens`; both
/// are captured and the parser selects the first populated value.
#[derive(Debug, Default, Deserialize)]
pub struct CodexUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cached_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
}

impl CodexUsage {
    pub fn cache_read(&self) -> Option<u64> {
        self.cached_input_tokens.or(self.cache_read_input_tokens)
    }
}

/// Error envelope. Codex builds emit errors either with flat
/// `error_type`/`error_message` fields or with a nested `error` object.
#[derive(Debug, Default, Deserialize)]
pub struct CodexErrorEnvelope {
    #[serde(default)]
    pub error_type: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub error: Option<CodexErrorDetail>,
}

impl CodexErrorEnvelope {
    pub fn resolved_kind(&self) -> Option<String> {
        self.error_type
            .clone()
            .or_else(|| self.error.as_ref().and_then(|e| e.kind.clone()))
    }

    pub fn resolved_message(&self) -> Option<String> {
        self.error_message
            .clone()
            .or_else(|| self.error.as_ref().and_then(|e| e.message.clone()))
            .or_else(|| self.message.clone())
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct CodexErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Envelope around a nested `item` for `item.started` / `item.completed`.
#[derive(Debug, Default, Deserialize)]
pub struct CodexItemEnvelope {
    #[serde(default)]
    pub item: Option<CodexItem>,
}

/// Flattened representation of a Codex item (agent_message, tool_use,
/// tool_result, permission_request, etc.). The parser branches on `kind` for
/// dispatch and accepts multiple aliases for tool fields.
#[derive(Debug, Default, Deserialize)]
pub struct CodexItem {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    /// Direct text used by `agent_message` items.
    #[serde(default)]
    pub text: Option<String>,
    /// Either an array of `{text: ...}` parts (agent_message) or a tool
    /// response payload (tool_result). The parser branches on `kind` to
    /// decide how to interpret it.
    #[serde(default)]
    pub content: Option<Value>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub arguments: Option<Value>,
    #[serde(default)]
    pub parameters: Option<Value>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
}

impl CodexItem {
    pub fn is_tool_item_kind(kind: &str) -> bool {
        matches!(
            kind,
            "tool_use"
                | "tool_call"
                | "mcp_tool_call"
                | "web_search"
                | "command_exec"
                | "patch_apply"
                | "image_generation"
                | "view_image"
        )
    }

    pub fn is_permission_item_kind(kind: &str) -> bool {
        matches!(
            kind,
            "permission_request" | "approval_request" | "user_input_request"
        )
    }

    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref().or(self.name.as_deref())
    }

    pub fn resolved_input(&self) -> Option<&Value> {
        self.input
            .as_ref()
            .or(self.arguments.as_ref())
            .or(self.parameters.as_ref())
    }

    pub fn resolved_output(&self) -> Option<&Value> {
        self.output
            .as_ref()
            .or(self.result.as_ref())
            .or(self.content.as_ref())
    }

    /// Fold a previously-seen `item.started` record into a
    /// corresponding `item.completed` record. Any field missing on
    /// `self` (the completed event) is inherited from `started`.
    pub fn merge_started(mut self, started: CodexItem) -> CodexItem {
        if self.kind.is_none() {
            self.kind = started.kind;
        }
        if self.id.is_none() {
            self.id = started.id;
        }
        if self.text.is_none() {
            self.text = started.text;
        }
        if self.content.is_none() {
            self.content = started.content;
        }
        if self.name.is_none() {
            self.name = started.name;
        }
        if self.tool_name.is_none() {
            self.tool_name = started.tool_name;
        }
        if self.input.is_none() {
            self.input = started.input;
        }
        if self.arguments.is_none() {
            self.arguments = started.arguments;
        }
        if self.parameters.is_none() {
            self.parameters = started.parameters;
        }
        if self.output.is_none() {
            self.output = started.output;
        }
        if self.result.is_none() {
            self.result = started.result;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> CodexEvent {
        serde_json::from_str(line).expect("valid CodexEvent")
    }

    #[test]
    fn codex_thread_started_deserializes() {
        let event = parse(r#"{"type":"thread.started","thread_id":"thrd-1"}"#);
        let CodexEvent::ThreadStarted(meta) = event else {
            panic!("expected ThreadStarted");
        };
        assert_eq!(meta.resolved_id(), Some("thrd-1".into()));
    }

    #[test]
    fn codex_thread_created_alias() {
        let event = parse(r#"{"type":"thread.created","id":"thrd-2"}"#);
        let CodexEvent::ThreadCreated(meta) = event else {
            panic!("expected ThreadCreated");
        };
        assert_eq!(meta.resolved_id(), Some("thrd-2".into()));
    }

    #[test]
    fn codex_turn_started_accepts_empty() {
        let event = parse(r#"{"type":"turn.started"}"#);
        assert!(matches!(event, CodexEvent::TurnStarted(_)));
    }

    #[test]
    fn codex_turn_completed_with_usage() {
        let event = parse(
            r#"{"type":"turn.completed","usage":{"input_tokens":200,"output_tokens":100,"cached_input_tokens":50},"duration_ms":5000,"status":"completed"}"#,
        );
        let CodexEvent::TurnCompleted(tc) = event else {
            panic!("expected TurnCompleted");
        };
        assert_eq!(tc.duration_ms, Some(5000));
        assert_eq!(tc.provider_status(), Some("completed"));
        let usage = tc.usage.expect("usage");
        assert_eq!(usage.input_tokens, Some(200));
        assert_eq!(usage.output_tokens, Some(100));
        assert_eq!(usage.cache_read(), Some(50));
    }

    #[test]
    fn codex_error_flat_fields() {
        let event = parse(
            r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#,
        );
        let CodexEvent::Error(err) = event else {
            panic!("expected Error");
        };
        assert_eq!(err.resolved_kind(), Some("rate_limit".into()));
        assert_eq!(err.resolved_message(), Some("Too many requests".into()));
    }

    #[test]
    fn codex_error_nested_object() {
        let event = parse(
            r#"{"type":"stream.error","error":{"type":"network","message":"socket closed"}}"#,
        );
        let CodexEvent::StreamError(err) = event else {
            panic!("expected StreamError");
        };
        assert_eq!(err.resolved_kind(), Some("network".into()));
        assert_eq!(err.resolved_message(), Some("socket closed".into()));
    }

    #[test]
    fn codex_item_started_agent_message() {
        let event = parse(
            r#"{"type":"item.started","item":{"id":"item_0","type":"agent_message","text":"hi"}}"#,
        );
        let CodexEvent::ItemStarted(env) = event else {
            panic!("expected ItemStarted");
        };
        let item = env.item.expect("item");
        assert_eq!(item.kind.as_deref(), Some("agent_message"));
        assert_eq!(item.text.as_deref(), Some("hi"));
    }

    #[test]
    fn codex_item_completed_tool_use() {
        let event = parse(
            r#"{"type":"item.completed","item":{"id":"tu-1","type":"tool_use","tool_name":"bash","input":{"command":"ls"},"output":"ok"}}"#,
        );
        let CodexEvent::ItemCompleted(env) = event else {
            panic!("expected ItemCompleted");
        };
        let item = env.item.expect("item");
        assert!(CodexItem::is_tool_item_kind(item.kind.as_deref().unwrap()));
        assert_eq!(item.resolved_tool_name(), Some("bash"));
        assert_eq!(
            item.resolved_input()
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str),
            Some("ls")
        );
        assert_eq!(item.resolved_output().and_then(Value::as_str), Some("ok"));
    }

    #[test]
    fn codex_top_level_tool_use_deserializes() {
        let event = parse(r#"{"type":"item.tool_use","name":"bash"}"#);
        let CodexEvent::ItemToolUse(item) = event else {
            panic!("expected ItemToolUse");
        };
        assert_eq!(item.name.as_deref(), Some("bash"));
    }

    #[test]
    fn codex_top_level_tool_result_deserializes() {
        let event = parse(r#"{"type":"item.tool_result","status":"ok"}"#);
        assert!(matches!(event, CodexEvent::ItemToolResult(_)));
    }

    #[test]
    fn codex_merge_started_populates_missing_fields() {
        let started = CodexItem {
            kind: Some("tool_use".into()),
            id: Some("tu-1".into()),
            name: Some("bash".into()),
            input: Some(serde_json::json!({"command": "ls"})),
            ..Default::default()
        };
        let completed = CodexItem {
            kind: Some("tool_use".into()),
            id: Some("tu-1".into()),
            output: Some(Value::String("clean".into())),
            ..Default::default()
        };
        let merged = completed.merge_started(started);
        assert_eq!(merged.name.as_deref(), Some("bash"));
        assert_eq!(
            merged
                .resolved_input()
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str),
            Some("ls")
        );
        assert_eq!(
            merged.resolved_output().and_then(Value::as_str),
            Some("clean")
        );
    }

    #[test]
    fn codex_unknown_event_type_fails_typed() {
        let err = serde_json::from_str::<CodexEvent>(r#"{"type":"session.not_a_real_event"}"#);
        assert!(err.is_err());
    }
}
