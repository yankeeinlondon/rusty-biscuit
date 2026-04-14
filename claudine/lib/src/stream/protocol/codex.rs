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
///
/// Top-level shorthand events (`item.tool_use`, `tool_use`, `item.tool_result`,
/// `tool_result`) carry tool fields directly, without a nested item type tag,
/// so they use [`CodexToolItemFields`] rather than the tagged [`CodexItem`]
/// enum.
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
    #[serde(rename = "item.updated")]
    ItemUpdated(CodexItemEnvelope),
    #[serde(rename = "item.tool_use")]
    ItemToolUse(CodexToolItemFields),
    #[serde(rename = "tool_use")]
    ToolUse(CodexToolItemFields),
    #[serde(rename = "item.tool_result")]
    ItemToolResult(CodexToolItemFields),
    #[serde(rename = "tool_result")]
    ToolResult(CodexToolItemFields),
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

/// Tagged enum over the `item.type` discriminator that appears inside
/// `item.started` / `item.completed` envelopes. Tool variants share
/// [`CodexToolItemFields`]; permission variants share [`CodexPermissionItem`];
/// the `Reasoning` variant is a separate shape because Codex emits an
/// optional `summary` block for it. Unknown item types fall into [`Unknown`]
/// so a single bad variant can't fail the whole envelope.
///
/// [`Unknown`]: CodexItem::Unknown
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexItem {
    AgentMessage(CodexAgentMessage),
    ToolUse(CodexToolItemFields),
    ToolCall(CodexToolItemFields),
    McpToolCall(CodexToolItemFields),
    WebSearch(CodexToolItemFields),
    CommandExec(CodexToolItemFields),
    PatchApply(CodexToolItemFields),
    ImageGeneration(CodexToolItemFields),
    ViewImage(CodexToolItemFields),
    PermissionRequest(CodexPermissionItem),
    ApprovalRequest(CodexPermissionItem),
    UserInputRequest(CodexPermissionItem),
    Reasoning(CodexReasoning),
    FileChange(CodexFileChange),
    PlanUpdate(CodexPlanUpdate),
    TodoList(CodexPlanUpdate),
    #[serde(other)]
    Unknown,
}

/// Typed `file_change` item emitted by Codex when a command or patch modifies
/// files on disk. Shape is intentionally tolerant of field drift — real Codex
/// builds have used both `path` and `file_path`, and `kind` variously named
/// `change_kind` or `operation`.
#[derive(Debug, Default, Deserialize)]
pub struct CodexFileChange {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub change_kind: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub operation: Option<String>,
}

impl CodexFileChange {
    pub fn resolved_path(&self) -> Option<&str> {
        self.path.as_deref().or(self.file_path.as_deref())
    }

    pub fn resolved_kind(&self) -> Option<&str> {
        self.change_kind
            .as_deref()
            .or(self.kind.as_deref())
            .or(self.operation.as_deref())
    }
}

/// Typed `plan_update` / `todo_list` item. Codex emits plan-tracking data under
/// various shapes; this struct accepts the common fields and lets anything else
/// survive through `extra` on the resulting [`super::super::semantic::SemanticEvent::PlanUpdate`].
#[derive(Debug, Default, Deserialize)]
pub struct CodexPlanUpdate {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

impl CodexPlanUpdate {
    pub fn resolved_message(&self) -> Option<String> {
        self.message
            .clone()
            .or_else(|| self.summary.clone())
            .or_else(|| self.title.clone())
    }
}

impl CodexItem {
    /// Whether this variant carries a tool item payload (any of the
    /// `tool_use`, `tool_call`, `mcp_tool_call`, etc. shapes).
    pub fn is_tool_item(&self) -> bool {
        matches!(
            self,
            CodexItem::ToolUse(_)
                | CodexItem::ToolCall(_)
                | CodexItem::McpToolCall(_)
                | CodexItem::WebSearch(_)
                | CodexItem::CommandExec(_)
                | CodexItem::PatchApply(_)
                | CodexItem::ImageGeneration(_)
                | CodexItem::ViewImage(_)
        )
    }

    /// Whether this variant carries a permission/approval/user-input
    /// payload that should fan out to the permission sink hook.
    pub fn is_permission_item(&self) -> bool {
        matches!(
            self,
            CodexItem::PermissionRequest(_)
                | CodexItem::ApprovalRequest(_)
                | CodexItem::UserInputRequest(_)
        )
    }

    /// Borrow the tool fields if this variant is a tool item.
    pub fn as_tool_fields(&self) -> Option<&CodexToolItemFields> {
        match self {
            CodexItem::ToolUse(f)
            | CodexItem::ToolCall(f)
            | CodexItem::McpToolCall(f)
            | CodexItem::WebSearch(f)
            | CodexItem::CommandExec(f)
            | CodexItem::PatchApply(f)
            | CodexItem::ImageGeneration(f)
            | CodexItem::ViewImage(f) => Some(f),
            _ => None,
        }
    }

    /// Mutable borrow of the tool fields, used by the parser to merge
    /// `item.started` into `item.completed`.
    pub fn as_tool_fields_mut(&mut self) -> Option<&mut CodexToolItemFields> {
        match self {
            CodexItem::ToolUse(f)
            | CodexItem::ToolCall(f)
            | CodexItem::McpToolCall(f)
            | CodexItem::WebSearch(f)
            | CodexItem::CommandExec(f)
            | CodexItem::PatchApply(f)
            | CodexItem::ImageGeneration(f)
            | CodexItem::ViewImage(f) => Some(f),
            _ => None,
        }
    }

    /// Borrow the permission item if applicable.
    pub fn as_permission(&self) -> Option<&CodexPermissionItem> {
        match self {
            CodexItem::PermissionRequest(p)
            | CodexItem::ApprovalRequest(p)
            | CodexItem::UserInputRequest(p) => Some(p),
            _ => None,
        }
    }

    /// Borrow the agent message if applicable.
    pub fn as_agent_message(&self) -> Option<&CodexAgentMessage> {
        match self {
            CodexItem::AgentMessage(m) => Some(m),
            _ => None,
        }
    }

    /// Resolve the item id from whichever sub-payload carries it.
    pub fn resolved_id(&self) -> Option<&str> {
        match self {
            CodexItem::AgentMessage(m) => m.id.as_deref(),
            CodexItem::PermissionRequest(p)
            | CodexItem::ApprovalRequest(p)
            | CodexItem::UserInputRequest(p) => p.id.as_deref(),
            CodexItem::Reasoning(_) | CodexItem::Unknown => None,
            _ => self.as_tool_fields().and_then(|f| f.id.as_deref()),
        }
    }

    /// Fold the started snapshot into the completed snapshot. The parser
    /// stores the started form keyed by id and merges when the matching
    /// `item.completed` arrives. Only tool variants carry merge-eligible
    /// data today; for other variants the completed snapshot is returned
    /// unchanged.
    pub fn merge_started(mut self, started: CodexItem) -> CodexItem {
        if let (Some(completed_fields), Some(started_fields)) =
            (self.as_tool_fields_mut(), started.into_tool_fields())
        {
            completed_fields.merge_started(started_fields);
        }
        self
    }

    /// Consume the item and return its tool fields, if it carries any.
    /// Used by [`merge_started`] when folding a stored started snapshot
    /// into the corresponding completed snapshot.
    ///
    /// [`merge_started`]: CodexItem::merge_started
    pub fn into_tool_fields(self) -> Option<CodexToolItemFields> {
        match self {
            CodexItem::ToolUse(f)
            | CodexItem::ToolCall(f)
            | CodexItem::McpToolCall(f)
            | CodexItem::WebSearch(f)
            | CodexItem::CommandExec(f)
            | CodexItem::PatchApply(f)
            | CodexItem::ImageGeneration(f)
            | CodexItem::ViewImage(f) => Some(f),
            _ => None,
        }
    }
}

/// Single text part inside an `agent_message.content` array. Codex emits
/// `{type: "text", text: "..."}` entries; the `type` field is preserved as
/// `kind` for diagnostics but is not required.
#[derive(Debug, Default, Deserialize)]
pub struct CodexContentPart {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

/// Typed `agent_message` body. Either `text` (legacy / synthesized form)
/// or `content` (the canonical array of [`CodexContentPart`]) carries the
/// assistant text. The parser concatenates whichever is populated.
#[derive(Debug, Default, Deserialize)]
pub struct CodexAgentMessage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub content: Option<Vec<CodexContentPart>>,
}

impl CodexAgentMessage {
    /// Concatenate all text fragments. Falls back to the top-level `text`
    /// field if no `content` parts carry text.
    pub fn collected_text(&self) -> Option<String> {
        if let Some(parts) = &self.content {
            let mut collected = String::new();
            for part in parts {
                if let Some(text) = &part.text {
                    collected.push_str(text);
                }
            }
            if !collected.is_empty() {
                return Some(collected);
            }
        }
        self.text.clone().filter(|s| !s.is_empty())
    }
}

/// Permission/approval/user-input item shape. Codex carries `id` and
/// `name` for these and the parser uses `name` as the tool name placeholder
/// when fanning out to the permission sink hook.
#[derive(Debug, Default, Deserialize)]
pub struct CodexPermissionItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Reasoning item shape. Codex emits a `text` field plus an optional
/// `summary` payload that is opaque today. The parser does not currently
/// surface reasoning traces beyond logging.
#[derive(Debug, Default, Deserialize)]
pub struct CodexReasoning {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub summary: Option<Value>,
}

/// Shared tool item payload reused by every tool-bearing variant of
/// [`CodexItem`] and by the top-level `item.tool_use` / `tool_use` /
/// `item.tool_result` / `tool_result` event variants on [`CodexEvent`].
#[derive(Debug, Default, Deserialize)]
pub struct CodexToolItemFields {
    #[serde(default)]
    pub id: Option<String>,
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
    #[serde(default)]
    pub content: Option<Value>,
    /// Command-execution status reported on `command_execution` completions
    /// (e.g. "success", "failure", "timeout").
    #[serde(default)]
    pub status: Option<String>,
    /// Process exit code for shell-tool completions.
    #[serde(default)]
    pub exit_code: Option<i32>,
}

impl CodexToolItemFields {
    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref().or(self.name.as_deref())
    }

    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.id.as_deref()
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

    /// Fold a previously-seen `item.started` snapshot into this completed
    /// snapshot. Any field missing on `self` (completed) is inherited from
    /// `started`.
    pub fn merge_started(&mut self, started: CodexToolItemFields) {
        if self.id.is_none() {
            self.id = started.id;
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
        if self.content.is_none() {
            self.content = started.content;
        }
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
        let msg = item.as_agent_message().expect("agent_message");
        assert_eq!(msg.id.as_deref(), Some("item_0"));
        assert_eq!(msg.text.as_deref(), Some("hi"));
        assert_eq!(msg.collected_text(), Some("hi".into()));
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
        assert!(item.is_tool_item());
        let fields = item.as_tool_fields().expect("tool fields");
        assert_eq!(fields.resolved_tool_name(), Some("bash"));
        assert_eq!(
            fields
                .resolved_input()
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str),
            Some("ls")
        );
        assert_eq!(fields.resolved_output().and_then(Value::as_str), Some("ok"));
    }

    #[test]
    fn codex_item_completed_typed_agent_message_content() {
        let event = parse(
            r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","content":[{"type":"text","text":"Hi "},{"type":"text","text":"there"}]}}"#,
        );
        let CodexEvent::ItemCompleted(env) = event else {
            panic!("expected ItemCompleted");
        };
        let item = env.item.expect("item");
        let msg = item.as_agent_message().expect("agent_message");
        let parts = msg.content.as_ref().expect("content");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].kind.as_deref(), Some("text"));
        assert_eq!(parts[0].text.as_deref(), Some("Hi "));
        assert_eq!(msg.collected_text(), Some("Hi there".into()));
    }

    #[test]
    fn codex_item_permission_request_typed() {
        let event = parse(
            r#"{"type":"item.started","item":{"id":"perm-1","type":"permission_request","name":"bash"}}"#,
        );
        let CodexEvent::ItemStarted(env) = event else {
            panic!("expected ItemStarted");
        };
        let item = env.item.expect("item");
        assert!(item.is_permission_item());
        let perm = item.as_permission().expect("permission");
        assert_eq!(perm.id.as_deref(), Some("perm-1"));
        assert_eq!(perm.name.as_deref(), Some("bash"));
    }

    #[test]
    fn codex_item_unknown_kind_falls_back() {
        let event =
            parse(r#"{"type":"item.started","item":{"id":"x","type":"some_brand_new_kind"}}"#);
        let CodexEvent::ItemStarted(env) = event else {
            panic!("expected ItemStarted");
        };
        let item = env.item.expect("item");
        assert!(matches!(item, CodexItem::Unknown));
    }

    #[test]
    fn codex_top_level_tool_use_deserializes() {
        let event = parse(r#"{"type":"item.tool_use","name":"bash"}"#);
        let CodexEvent::ItemToolUse(fields) = event else {
            panic!("expected ItemToolUse");
        };
        assert_eq!(fields.resolved_tool_name(), Some("bash"));
    }

    #[test]
    fn codex_top_level_tool_result_deserializes() {
        let event = parse(r#"{"type":"item.tool_result","status":"ok"}"#);
        assert!(matches!(event, CodexEvent::ItemToolResult(_)));
    }

    #[test]
    fn codex_merge_started_populates_missing_fields() {
        let started = CodexItem::ToolUse(CodexToolItemFields {
            id: Some("tu-1".into()),
            name: Some("bash".into()),
            input: Some(serde_json::json!({"command": "ls"})),
            ..Default::default()
        });
        let completed = CodexItem::ToolUse(CodexToolItemFields {
            id: Some("tu-1".into()),
            output: Some(Value::String("clean".into())),
            ..Default::default()
        });
        let merged = completed.merge_started(started);
        let fields = merged.as_tool_fields().expect("tool fields");
        assert_eq!(fields.name.as_deref(), Some("bash"));
        assert_eq!(
            fields
                .resolved_input()
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str),
            Some("ls")
        );
        assert_eq!(
            fields.resolved_output().and_then(Value::as_str),
            Some("clean")
        );
    }

    #[test]
    fn codex_unknown_event_type_fails_typed() {
        let err = serde_json::from_str::<CodexEvent>(r#"{"type":"session.not_a_real_event"}"#);
        assert!(err.is_err());
    }
}
