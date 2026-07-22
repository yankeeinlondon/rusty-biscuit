//! Typed event models for Codex CLI's `exec --json` JSONL format.
//!
//! Codex uses dotted event names (`thread.started`, `turn.completed`, etc.)
//! which serde handles through `#[serde(rename = "...")]` on each variant.
//! The parser falls back to `Value`-based skipping for any event type that
//! is not enumerated here.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Tagged enum over all Codex CLI stream event variants dispatched by the
/// parser. Unknown event types fail typed deserialization and are handled by
/// the parser's fallback arm.
///
/// Top-level shorthand events (`item.tool_use`, `tool_use`, `item.tool_result`,
/// `tool_result`) carry tool fields directly, without a nested item type tag,
/// so they use [`CodexToolItemFields`] rather than the tagged [`CodexItem`]
/// enum.
#[derive(Debug, Deserialize, Serialize)]
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

impl CodexEvent {
    /// Returns the JSON `type` discriminator for this event variant.
    pub const fn type_str(&self) -> &'static str {
        match self {
            CodexEvent::ThreadCreated(_) => "thread.created",
            CodexEvent::ThreadStarted(_) => "thread.started",
            CodexEvent::TurnStarted(_) => "turn.started",
            CodexEvent::TurnCompleted(_) => "turn.completed",
            CodexEvent::Error(_) => "error",
            CodexEvent::TurnError(_) => "turn.error",
            CodexEvent::TurnFailed(_) => "turn.failed",
            CodexEvent::StreamError(_) => "stream.error",
            CodexEvent::ItemStarted(_) => "item.started",
            CodexEvent::ItemCompleted(_) => "item.completed",
            CodexEvent::ItemUpdated(_) => "item.updated",
            CodexEvent::ItemToolUse(_) => "item.tool_use",
            CodexEvent::ToolUse(_) => "tool_use",
            CodexEvent::ItemToolResult(_) => "item.tool_result",
            CodexEvent::ToolResult(_) => "tool_result",
        }
    }
}

/// `thread.created` / `thread.started` payload. Some Codex builds emit
/// `thread_id`, others use `id`.
#[derive(Debug, Default, Deserialize, Serialize)]
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
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodexTurnStarted {}

/// `turn.completed` event — carries token usage, duration, and stop reason.
#[derive(Debug, Default, Deserialize, Serialize)]
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
    /// Dynamic fallback for unknown fields so the raw payload can be
    /// reconstructed without a second parse.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

impl CodexTurnCompleted {
    pub fn provider_status(&self) -> Option<&str> {
        self.status.as_deref().or(self.stop_reason.as_deref())
    }
}

/// Token usage block reported by `turn.completed`. Codex builds differ on
/// whether they send `cached_input_tokens` or `cache_read_input_tokens`; both
/// are captured and the parser selects the first populated value.
#[derive(Debug, Default, Deserialize, Serialize)]
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
#[derive(Debug, Default, Deserialize, Serialize)]
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodexErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Envelope around a nested `item` for `item.started` / `item.completed`.
#[derive(Debug, Default, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexItem {
    AgentMessage(CodexAgentMessage),
    ToolUse(CodexToolItemFields),
    ToolCall(CodexToolItemFields),
    McpToolCall(CodexToolItemFields),
    WebSearch(CodexToolItemFields),
    #[serde(alias = "command_execution")]
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
/// builds emit the paths inside a `changes: [{path, kind}]` array on
/// completion, while older/legacy builds have used flat `path` / `file_path`
/// and `change_kind` / `operation` fields. All shapes are captured; callers
/// iterate [`resolved_entries`] to get one entry per touched path.
///
/// [`resolved_entries`]: CodexFileChange::resolved_entries
#[derive(Debug, Default, Deserialize, Serialize)]
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
    /// Array of per-path change entries — the canonical shape Codex emits
    /// on `item.completed` for patch applies.
    #[serde(default)]
    pub changes: Option<Vec<CodexFileChangeEntry>>,
    /// Completion status (e.g. `completed`, `failed`, `declined`) carried
    /// alongside the `changes` array.
    #[serde(default)]
    pub status: Option<String>,
}

/// Single entry inside [`CodexFileChange::changes`].
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodexFileChangeEntry {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub change_kind: Option<String>,
    #[serde(default)]
    pub operation: Option<String>,
}

impl CodexFileChangeEntry {
    pub fn resolved_path(&self) -> Option<&str> {
        self.path.as_deref().or(self.file_path.as_deref())
    }

    pub fn resolved_kind(&self) -> Option<&str> {
        self.kind
            .as_deref()
            .or(self.change_kind.as_deref())
            .or(self.operation.as_deref())
    }
}

impl CodexFileChange {
    /// First flat-field resolution — kept for callers that only need a
    /// single path. Prefers the `changes[0]` entry when present.
    pub fn resolved_path(&self) -> Option<&str> {
        if let Some(entries) = self.changes.as_ref()
            && let Some(p) = entries.iter().find_map(|e| e.resolved_path())
        {
            return Some(p);
        }
        self.path.as_deref().or(self.file_path.as_deref())
    }

    /// First flat-field resolution for the change kind. Prefers the
    /// `changes[0]` entry when present.
    pub fn resolved_kind(&self) -> Option<&str> {
        if let Some(entries) = self.changes.as_ref()
            && let Some(k) = entries.iter().find_map(|e| e.resolved_kind())
        {
            return Some(k);
        }
        self.change_kind
            .as_deref()
            .or(self.kind.as_deref())
            .or(self.operation.as_deref())
    }

    /// Iterate all (path, kind) pairs this file_change event reports. When
    /// the canonical `changes[]` array is present, yields one pair per
    /// entry; otherwise falls back to the flat fields. Entries whose path
    /// is both missing and empty are filtered out so callers never see a
    /// meaningless empty rendering.
    pub fn resolved_entries(&self) -> Vec<(Option<String>, Option<String>)> {
        if let Some(entries) = self.changes.as_ref()
            && !entries.is_empty()
        {
            return entries
                .iter()
                .map(|e| {
                    (
                        e.resolved_path().map(str::to_string),
                        e.resolved_kind().map(str::to_string),
                    )
                })
                .filter(|(p, k)| p.as_deref().is_some_and(|s| !s.is_empty()) || k.is_some())
                .collect();
        }
        let path = self.resolved_path().map(str::to_string);
        let kind = self.resolved_kind().map(str::to_string);
        if path.as_deref().is_some_and(|s| !s.is_empty()) || kind.is_some() {
            vec![(path, kind)]
        } else {
            Vec::new()
        }
    }
}

/// Typed `plan_update` / `todo_list` item. Codex emits plan-tracking data under
/// various shapes; this struct accepts the common fields and lets anything else
/// survive through `extra` on the resulting [`super::super::semantic::SemanticEvent::PlanUpdate`].
#[derive(Debug, Default, Deserialize, Serialize)]
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
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodexContentPart {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

/// Typed `agent_message` body. Either `text` (legacy / synthesized form)
/// or `content` (the canonical array of [`CodexContentPart`]) carries the
/// assistant text. The parser concatenates whichever is populated.
#[derive(Debug, Default, Deserialize, Serialize)]
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
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodexPermissionItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Reasoning item shape. Codex emits a `text` field plus an optional
/// `summary` payload that is opaque today. The parser does not currently
/// surface reasoning traces beyond logging.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodexReasoning {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub summary: Option<Value>,
}

/// Shared tool item payload reused by every tool-bearing variant of
/// [`CodexItem`] and by the top-level `item.tool_use` / `tool_use` /
/// `item.tool_result` / `tool_result` event variants on [`CodexEvent`].
#[derive(Debug, Default, Deserialize, Serialize)]
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
    /// Raw shell command string emitted by Codex's `command_execution` items
    /// (both `item.started` and `item.completed`). Exposed as a synthesized
    /// `{"command": ...}` fallback via [`resolved_input`].
    ///
    /// [`resolved_input`]: CodexToolItemFields::resolved_input
    #[serde(default)]
    pub command: Option<String>,
    /// Aggregated stdout+stderr buffer emitted by Codex's `command_execution`
    /// items on completion. Exposed as a string fallback via
    /// [`resolved_output`].
    ///
    /// [`resolved_output`]: CodexToolItemFields::resolved_output
    #[serde(default)]
    pub aggregated_output: Option<String>,
}

impl CodexToolItemFields {
    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.tool_name
            .as_deref()
            .or(self.name.as_deref())
            .or_else(|| {
                self.command
                    .as_deref()
                    .map(|cmd| detect_shell_from_command(cmd).unwrap_or("shell"))
            })
    }

    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn resolved_input(&self) -> Option<Value> {
        if let Some(source) = self
            .input
            .as_ref()
            .or(self.arguments.as_ref())
            .or(self.parameters.as_ref())
        {
            return Some(source.clone());
        }
        if let Some(cmd) = self.command.as_deref() {
            let trimmed = strip_shell_path_prefix(cmd);
            return Some(serde_json::json!({ "command": trimmed }));
        }
        None
    }

    pub fn resolved_output(&self) -> Option<Value> {
        if let Some(source) = self
            .output
            .as_ref()
            .or(self.result.as_ref())
            .or(self.content.as_ref())
        {
            return Some(source.clone());
        }
        self.aggregated_output
            .as_ref()
            .map(|agg| Value::String(agg.clone()))
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
        if self.command.is_none() {
            self.command = started.command;
        }
        if self.aggregated_output.is_none() {
            self.aggregated_output = started.aggregated_output;
        }
    }
}

/// Detect the shell binary name from the leading token of a `command` string
/// emitted by Codex's `command_execution` items.
///
/// Codex wraps every shell command in an absolute invocation such as
/// `/bin/zsh -lc '<script>'`. Returning `"zsh"` (rather than the generic
/// `"shell"`) gives the live surface an accurate label and lets
/// [`tool_display`](crate::stream::tool_display) treat the command as a
/// shell tool with the right prefix. Returns `None` when the leading token
/// does not look like a shell path we recognize.
pub(crate) fn detect_shell_from_command(command: &str) -> Option<&'static str> {
    let first = command.split_whitespace().next()?;
    let basename = first.rsplit('/').next().unwrap_or(first);
    match basename {
        "zsh" => Some("zsh"),
        "bash" => Some("bash"),
        "sh" => Some("sh"),
        "fish" => Some("fish"),
        "dash" => Some("dash"),
        "ksh" => Some("ksh"),
        _ => None,
    }
}

/// Strip a leading `/path/to/<shell>` token from a command string so the
/// rendered summary reads as `zsh -lc '…'` rather than
/// `zsh /bin/zsh -lc '…'` after `tool_display` prepends the detected shell
/// name. When the first token does not name a recognized shell the command
/// is returned unchanged.
pub(crate) fn strip_shell_path_prefix(command: &str) -> String {
    let trimmed = command.trim_start();
    let Some((first, rest)) = trimmed.split_once(char::is_whitespace) else {
        return command.to_string();
    };
    let basename = first.rsplit('/').next().unwrap_or(first);
    if detect_shell_from_command(basename).is_some() && first.contains('/') {
        rest.trim_start().to_string()
    } else {
        command.to_string()
    }
}

#[cfg(test)]
mod tests;
