//! Typed event models for Kimi Code's `--wire` JSON-RPC 2.0 protocol.
//!
//! `KimiEnvelope` plus the `KimiWireEvent` / `KimiWireRequest` enums and
//! their typed payload structs model the protocol Kimi exposes via
//! `kimi --wire`. All structs derive `Deserialize` with `#[serde(default)]`
//! on every field; unknown `event.type` / `request.type` discriminants fail
//! typed deserialization so the semantic parser can fall back to a raw
//! `ProviderExtension` event.

use serde::Deserialize;
use serde_json::Value;

// ----------------------------------------------------------------------------
// Wire mode (JSON-RPC 2.0) protocol models
// ----------------------------------------------------------------------------
//
// Kimi exposes a richer protocol surface via `kimi --wire`. Each line on
// stdout is a JSON-RPC 2.0 envelope of one of four shapes:
//
//   1. Notification  — `{"jsonrpc":"2.0","method":"event","params":{...}}`
//   2. Server request — `{"jsonrpc":"2.0","method":"request","id":"…","params":{...}}`
//   3. Success response — `{"jsonrpc":"2.0","id":"…","result":{...}}`
//   4. Error response — `{"jsonrpc":"2.0","id":"…","error":{"code":<i32>,"message":"…"}}`
//
// `KimiEnvelope` is the single seam between the line reader and downstream
// dispatch: `KimiEnvelope::classify(value)` accepts a parsed `serde_json::Value`
// and returns either a typed envelope variant or `None` for envelopes that
// don't match any known shape (which the parser may then surface as a raw
// fallback). Notification params are decoded into `KimiNotificationParams`
// whose `type` discriminates the typed `KimiWireEvent`. Request params are
// decoded into `KimiRequestParams` whose `type` discriminates the typed
// `KimiWireRequest`.

/// Classified JSON-RPC envelope received from `kimi --wire`.
#[derive(Debug)]
pub enum KimiEnvelope {
    /// `{"method":"event", "params": {...}}` — a server-initiated notification
    /// that requires no response. Payload is decoded into typed event variants
    /// via `params.into_event()`.
    Notification(KimiNotificationParams),
    /// `{"method":"request", "id": "...", "params": {...}}` — a server-initiated
    /// request requiring a response keyed by `id`.
    Request {
        id: Value,
        params: KimiRequestParams,
    },
    /// `{"id": "...", "result": {...}}` — successful response to a previously-sent
    /// client request.
    SuccessResponse { id: Value, result: Value },
    /// `{"id": "...", "error": {...}}` — error response to a previously-sent
    /// client request.
    ErrorResponse { id: Value, error: KimiJsonRpcError },
}

impl KimiEnvelope {
    /// Classify a parsed `serde_json::Value` into a typed wire envelope.
    ///
    /// Returns `None` when the value does not match any known envelope shape
    /// (e.g. missing both `method` and `result`/`error`). Callers should treat
    /// `None` as a malformed/unknown envelope and surface it via the raw
    /// fallback path the way provider parsers do for unknown event types.
    pub fn classify(value: Value) -> Option<Self> {
        let raw = serde_json::from_value::<KimiRawEnvelope>(value).ok()?;
        match (raw.method.as_deref(), raw.id, raw.result, raw.error) {
            (Some("event"), _id, _r, _e) => {
                let params = raw.params.unwrap_or(Value::Null);
                let params: KimiNotificationParams = serde_json::from_value(params).ok()?;
                Some(KimiEnvelope::Notification(params))
            }
            (Some("request"), Some(id), _r, _e) => {
                let params = raw.params.unwrap_or(Value::Null);
                let params: KimiRequestParams = serde_json::from_value(params).ok()?;
                Some(KimiEnvelope::Request { id, params })
            }
            (None, Some(id), Some(result), None) => {
                Some(KimiEnvelope::SuccessResponse { id, result })
            }
            (None, Some(id), None, Some(error)) => Some(KimiEnvelope::ErrorResponse { id, error }),
            _ => None,
        }
    }
}

/// Internal raw shape used by [`KimiEnvelope::classify`]. Every field is
/// optional so the same struct deserializes any of the four envelope shapes
/// without rejecting unknown extras. The `jsonrpc` marker is accepted but
/// not inspected — clients trust the canonical Kimi wire transport.
#[derive(Debug, Default, Deserialize)]
struct KimiRawEnvelope {
    #[serde(default)]
    #[allow(dead_code)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<KimiJsonRpcError>,
}

/// JSON-RPC error object returned in `{"error":{...}}` response envelopes.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct KimiJsonRpcError {
    #[serde(default)]
    pub code: i32,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

impl KimiJsonRpcError {
    /// Standard JSON-RPC error codes plus Kimi-specific extensions. These map
    /// to [`crate::stream::semantic::SemanticErrorKind`] in Phase 2.
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    /// `AUTH_EXPIRED` — Kimi-specific code observed on `prompt` request
    /// responses when the OAuth credentials need re-authentication via
    /// `kimi login`.
    pub const AUTH_EXPIRED: i32 = -32004;
    /// `CHAT_PROVIDER_ERROR` — Kimi-specific code surfaced when the upstream
    /// chat provider returns an error (rate limits, billing, upstream API).
    pub const CHAT_PROVIDER_ERROR: i32 = -32005;
}

/// `params` payload of a `method: "event"` notification envelope. Wraps a
/// discriminator (`type`) and an open `payload` value that further typed
/// dispatch reads from.
#[derive(Debug, Default, Deserialize)]
pub struct KimiNotificationParams {
    #[serde(default, rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub payload: Value,
}

impl KimiNotificationParams {
    /// Decode the params into a typed [`KimiWireEvent`]. Returns `None` for
    /// unknown discriminants — caller should then expose the raw shape via the
    /// parser fallback.
    pub fn into_event(self) -> Option<KimiWireEvent> {
        let event_type = self.event_type.clone();
        let payload = self.payload;
        // Reconstruct a `{type, payload}` value so a `#[serde(tag = "type",
        // content = "payload")]` enum can dispatch on the discriminator while
        // tolerating missing/empty payloads.
        let envelope = serde_json::json!({
            "type": event_type,
            "payload": payload,
        });
        serde_json::from_value(envelope).ok()
    }
}

/// `params` payload of a `method: "request"` envelope. Same `type`/`payload`
/// shape as notifications but discriminates a different enum.
#[derive(Debug, Default, Deserialize)]
pub struct KimiRequestParams {
    #[serde(default, rename = "type")]
    pub request_type: String,
    #[serde(default)]
    pub payload: Value,
}

impl KimiRequestParams {
    /// Decode the params into a typed [`KimiWireRequest`]. Returns `None` for
    /// unknown discriminants.
    pub fn into_request(self) -> Option<KimiWireRequest> {
        let request_type = self.request_type.clone();
        let payload = self.payload;
        let envelope = serde_json::json!({
            "type": request_type,
            "payload": payload,
        });
        serde_json::from_value(envelope).ok()
    }
}

/// Tagged enum over Kimi wire event variants observed in captured fixtures
/// plus the spec catalog. Uses `#[serde(tag = "type", content = "payload")]`
/// so it dispatches on the discriminator while consuming the payload as a
/// strongly-typed struct. Unknown event types fail typed deserialization and
/// the parser surfaces them via the raw fallback path.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum KimiWireEvent {
    #[serde(rename = "TurnBegin")]
    TurnBegin(KimiTurnBegin),
    #[serde(rename = "TurnEnd")]
    TurnEnd(KimiTurnEnd),
    #[serde(rename = "StepBegin")]
    StepBegin(KimiStepBegin),
    #[serde(rename = "StepInterrupted")]
    StepInterrupted(KimiStepInterrupted),
    #[serde(rename = "SteerInput")]
    SteerInput(KimiSteerInput),
    #[serde(rename = "CompactionBegin")]
    CompactionBegin(KimiCompactionBegin),
    #[serde(rename = "CompactionEnd")]
    CompactionEnd(KimiCompactionEnd),
    #[serde(rename = "MCPLoadingBegin")]
    McpLoadingBegin(KimiMcpLoadingBegin),
    #[serde(rename = "MCPLoadingEnd")]
    McpLoadingEnd(KimiMcpLoadingEnd),
    #[serde(rename = "StatusUpdate")]
    StatusUpdate(KimiWireStatusUpdate),
    #[serde(rename = "Notification")]
    Notification(KimiWireNotification),
    #[serde(rename = "PlanDisplay")]
    PlanDisplay(KimiPlanDisplay),
    #[serde(rename = "ContentPart")]
    ContentPart(KimiContentPart),
    #[serde(rename = "ToolCall")]
    ToolCall(KimiToolCall),
    #[serde(rename = "ToolCallPart")]
    ToolCallPart(KimiToolCallPart),
    #[serde(rename = "ToolResult")]
    ToolResult(KimiToolResult),
    #[serde(rename = "ApprovalResponse")]
    ApprovalResponse(KimiApprovalResponseEvent),
    #[serde(rename = "SubagentEvent")]
    SubagentEvent(Box<KimiSubagentEvent>),
    #[serde(rename = "BtwBegin")]
    BtwBegin(KimiBtwBegin),
    #[serde(rename = "BtwEnd")]
    BtwEnd(KimiBtwEnd),
    /// `HookTriggered` — declared in Kimi's wire types module but only emitted
    /// when the client subscribes to `hooks` capabilities. Modeled here so
    /// future hook plumbing can dispatch typed payloads.
    #[serde(rename = "HookTriggered")]
    HookTriggered(KimiHookTriggered),
    /// `HookResolved` — companion to `HookTriggered`.
    #[serde(rename = "HookResolved")]
    HookResolved(KimiHookResolved),
    /// `DiffDisplayBlock` — modeled per spec; not observed in current
    /// fixtures but reserved so additive protocol drift is forward-compatible.
    #[serde(rename = "DiffDisplayBlock")]
    DiffDisplayBlock(KimiDiffDisplayBlock),
}

/// Tagged enum over Kimi wire request variants. Same dispatch shape as
/// `KimiWireEvent`. Each variant requires a response (`{"jsonrpc":"2.0","id":
/// "<original-id>","result":{...}}` or matching `error`).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum KimiWireRequest {
    #[serde(rename = "ApprovalRequest")]
    Approval(KimiApprovalRequest),
    #[serde(rename = "QuestionRequest")]
    Question(KimiQuestionRequest),
    #[serde(rename = "ToolCallRequest")]
    ToolCall(KimiToolCallRequest),
    #[serde(rename = "HookRequest")]
    Hook(KimiHookRequest),
}

// --- Event payload structs --------------------------------------------------

/// `TurnBegin` payload — first event of a turn, carries the user prompt.
#[derive(Debug, Default, Deserialize)]
pub struct KimiTurnBegin {
    /// Either a `String` or a `[ContentPart]` array per the wire schema. Kept
    /// as `Value` so both shapes round-trip without dropping data.
    #[serde(default)]
    pub user_input: Option<Value>,
}

impl KimiTurnBegin {
    /// Best-effort plain-text rendering of `user_input`. Returns `None` when
    /// the field is absent or carries an unsupported shape.
    pub fn user_input_text(&self) -> Option<String> {
        match &self.user_input {
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Array(parts)) => {
                let mut collected = String::new();
                for part in parts {
                    if let Some(text) = part.get("text").and_then(Value::as_str) {
                        collected.push_str(text);
                    }
                }
                if collected.is_empty() {
                    None
                } else {
                    Some(collected)
                }
            }
            _ => None,
        }
    }
}

/// `TurnEnd` payload — observed as `{}` in current captures. Status arrives on
/// the `prompt` request response (`KimiPromptResult`), not on `TurnEnd`.
#[derive(Debug, Default, Deserialize)]
pub struct KimiTurnEnd {
    /// Reserved — older protocol revisions may carry a status here.
    #[serde(default)]
    pub status: Option<String>,
}

/// `StepBegin` payload — counter for the model step within a turn.
#[derive(Debug, Default, Deserialize)]
pub struct KimiStepBegin {
    #[serde(default)]
    pub n: Option<u64>,
}

/// `StepInterrupted` payload — emitted when a step is cut short.
#[derive(Debug, Default, Deserialize)]
pub struct KimiStepInterrupted {
    #[serde(default)]
    pub reason: Option<String>,
}

/// `SteerInput` payload — out-of-band steering directive from the user.
#[derive(Debug, Default, Deserialize)]
pub struct KimiSteerInput {
    #[serde(default)]
    pub user_input: Option<Value>,
}

/// `CompactionBegin` payload — context compaction phase started.
#[derive(Debug, Default, Deserialize)]
pub struct KimiCompactionBegin {
    #[serde(default)]
    pub focus: Option<String>,
}

/// `CompactionEnd` payload — context compaction phase ended.
#[derive(Debug, Default, Deserialize)]
pub struct KimiCompactionEnd {
    #[serde(default)]
    pub tokens_saved: Option<u64>,
}

/// `MCPLoadingBegin` payload — MCP server initialization started.
#[derive(Debug, Default, Deserialize)]
pub struct KimiMcpLoadingBegin {
    #[serde(default)]
    pub server: Option<String>,
}

/// `MCPLoadingEnd` payload — MCP server initialization ended.
#[derive(Debug, Default, Deserialize)]
pub struct KimiMcpLoadingEnd {
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

/// `StatusUpdate` payload — token usage, message id, context window, plan
/// mode, MCP status. Fields are all optional so additive evolution is safe.
#[derive(Debug, Default, Deserialize)]
pub struct KimiWireStatusUpdate {
    /// Pre-computed context fraction in `[0.0, 1.0]`. Wire mode emits this as
    /// a float, NOT a `{used,total,percent}` block — `effective_context_*`
    /// helpers normalize the two representations.
    #[serde(default)]
    pub context_usage: Option<f64>,
    #[serde(default)]
    pub context_tokens: Option<u64>,
    #[serde(default)]
    pub max_context_tokens: Option<u64>,
    #[serde(default)]
    pub token_usage: Option<KimiWireTokenUsage>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub plan_mode: Option<bool>,
    #[serde(default)]
    pub mcp_status: Option<Value>,
}

impl KimiWireStatusUpdate {
    /// Resolve the context-usage percent, computing it from token counters
    /// when only the fraction is missing.
    pub fn computed_context_percent(&self) -> Option<f64> {
        if let Some(frac) = self.context_usage {
            return Some(frac * 100.0);
        }
        match (self.context_tokens, self.max_context_tokens) {
            (Some(used), Some(total)) if total > 0 => Some((used as f64 / total as f64) * 100.0),
            _ => None,
        }
    }
}

/// Token-usage block on `StatusUpdate`. The wire-mode field set differs from
/// the legacy stream-json model: inputs are split into `input_other`,
/// `input_cache_read`, and `input_cache_creation`; output is a single field.
#[derive(Debug, Default, Deserialize)]
pub struct KimiWireTokenUsage {
    #[serde(default)]
    pub input_other: Option<u64>,
    #[serde(default)]
    pub input_cache_read: Option<u64>,
    #[serde(default)]
    pub input_cache_creation: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
}

impl KimiWireTokenUsage {
    /// Total input tokens across all sub-buckets.
    pub fn total_input(&self) -> Option<u64> {
        let parts = [
            self.input_other,
            self.input_cache_read,
            self.input_cache_creation,
        ];
        let mut sum: u64 = 0;
        let mut any = false;
        for v in parts.into_iter().flatten() {
            sum = sum.saturating_add(v);
            any = true;
        }
        any.then_some(sum)
    }

    /// Cache-read input tokens, surfaced separately for token-budget badges.
    pub fn cache_read_input(&self) -> Option<u64> {
        self.input_cache_read
    }
}

/// `Notification` event payload — server-emitted info notice.
#[derive(Debug, Default, Deserialize)]
pub struct KimiWireNotification {
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

/// `PlanDisplay` event payload — Kimi plan-mode rendering surface.
#[derive(Debug, Default, Deserialize)]
pub struct KimiPlanDisplay {
    #[serde(default)]
    pub plan: Option<Value>,
    #[serde(default)]
    pub display: Option<Value>,
}

/// `ContentPart` event payload — assistant reasoning or assistant text streamed
/// as deltas. Discriminator is the inner `type` field which is one of
/// `think`, `text`, `image_url`, `audio_url`, `video_url`.
#[derive(Debug, Default, Deserialize)]
pub struct KimiContentPart {
    #[serde(default, rename = "type")]
    pub part_type: String,
    /// Reasoning text when `part_type == "think"`.
    #[serde(default)]
    pub think: Option<String>,
    /// Optional encrypted reasoning blob accompanying `think`.
    #[serde(default)]
    pub encrypted: Option<Value>,
    /// Assistant text when `part_type == "text"`.
    #[serde(default)]
    pub text: Option<String>,
    /// Image URL when `part_type == "image_url"`.
    #[serde(default)]
    pub image_url: Option<Value>,
    /// Audio URL when `part_type == "audio_url"`.
    #[serde(default)]
    pub audio_url: Option<Value>,
    /// Video URL when `part_type == "video_url"`.
    #[serde(default)]
    pub video_url: Option<Value>,
}

impl KimiContentPart {
    /// `true` when this part carries reasoning text.
    pub fn is_thinking(&self) -> bool {
        self.part_type == "think"
    }

    /// `true` when this part carries assistant text.
    pub fn is_text(&self) -> bool {
        self.part_type == "text"
    }

    /// Resolved text body for the part regardless of `part_type`. Returns
    /// `None` for media parts that don't carry inline text.
    pub fn resolved_text(&self) -> Option<&str> {
        if self.is_thinking() {
            self.think.as_deref()
        } else if self.is_text() {
            self.text.as_deref()
        } else {
            None
        }
    }
}

/// `ToolCall` event payload — emitted once per tool invocation. Arguments
/// arrive as a JSON-encoded *string* on `function.arguments` (per the OpenAI
/// tool-call convention) and are usually empty here, with subsequent
/// `ToolCallPart` events streaming `arguments_part` deltas. The parser is
/// responsible for accumulating those deltas and JSON-decoding the final
/// argument blob via [`KimiToolCall::parse_arguments_string`].
#[derive(Debug, Default, Deserialize)]
pub struct KimiToolCall {
    #[serde(default, rename = "type")]
    pub call_type: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub function: Option<KimiToolCallFunction>,
    #[serde(default)]
    pub extras: Option<Value>,
}

impl KimiToolCall {
    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.function.as_ref().and_then(|f| f.name.as_deref())
    }

    /// Take the raw argument string from `function.arguments`. Useful for
    /// concatenation with subsequent `ToolCallPart` deltas.
    pub fn take_arguments_string(&mut self) -> Option<String> {
        self.function.as_mut().and_then(|f| f.arguments.take())
    }

    /// Parse a serialized JSON argument string into a structured `Value`.
    ///
    /// Returns `Ok(None)` for an empty input. On JSON parse failure, returns
    /// `Err` with the original string so callers can fall back to a string
    /// passthrough representation rather than dropping the call.
    pub fn parse_arguments_string(raw: &str) -> Result<Option<Value>, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        serde_json::from_str(trimmed)
            .map(Some)
            .map_err(|_| raw.to_string())
    }
}

/// `function` block on `ToolCall`. Holds the tool name and the JSON-encoded
/// argument string.
#[derive(Debug, Default, Deserialize)]
pub struct KimiToolCallFunction {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

/// `ToolCallPart` event payload — incremental argument-string delta for the
/// most recent `ToolCall` envelope. Concatenate `arguments_part` values until
/// the next non-`ToolCallPart` event arrives, then JSON-decode the collected
/// string via [`KimiToolCall::parse_arguments_string`].
#[derive(Debug, Default, Deserialize)]
pub struct KimiToolCallPart {
    #[serde(default)]
    pub arguments_part: Option<String>,
}

/// `ToolResult` event payload — tool execution outcome. `return_value` carries
/// the structured result fields; `tool_call_id` correlates with the originating
/// `ToolCall.id`.
#[derive(Debug, Default, Deserialize)]
pub struct KimiToolResult {
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub return_value: Option<KimiToolReturnValue>,
}

impl KimiToolResult {
    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.tool_call_id.as_deref()
    }

    pub fn is_error(&self) -> bool {
        self.return_value
            .as_ref()
            .map(|rv| rv.is_error)
            .unwrap_or(false)
    }

    pub fn take_output(&mut self) -> Option<Value> {
        self.return_value.as_mut().and_then(|rv| rv.take_output())
    }

    /// Status string derived from `return_value.is_error` for compatibility
    /// with the existing `SemanticEvent::ToolResult.status` field.
    pub fn derived_status(&self) -> &'static str {
        if self.is_error() { "error" } else { "success" }
    }
}

/// `return_value` block on `ToolResult`. Output is canonically a string but
/// the field is typed as `Value` to tolerate JSON tool outputs.
#[derive(Debug, Default, Deserialize)]
pub struct KimiToolReturnValue {
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub display: Option<Value>,
    #[serde(default)]
    pub extras: Option<Value>,
}

impl KimiToolReturnValue {
    pub fn take_output(&mut self) -> Option<Value> {
        self.output.take()
    }
}

/// `ApprovalResponse` event payload — server echo of the client's approval
/// decision after the matching `ApprovalRequest` was answered.
#[derive(Debug, Default, Deserialize)]
pub struct KimiApprovalResponseEvent {
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub response: Option<String>,
    #[serde(default)]
    pub feedback: Option<String>,
}

/// `SubagentEvent` event payload — wraps a nested wire event emitted by a
/// subagent run. The nested event arrives in `event` and is itself a
/// `{"type":"...","payload":{...}}` shape. Approvals for tool calls inside
/// the subagent arrive un-nested on the parent stream as `ApprovalRequest`
/// envelopes (carrying `agent_id` / `subagent_type` for correlation).
#[derive(Debug, Default, Deserialize)]
pub struct KimiSubagentEvent {
    #[serde(default)]
    pub parent_tool_call_id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub subagent_type: Option<String>,
    /// Nested wire event. Stored as `Value` here so a separate decode pass can
    /// walk it through [`KimiNotificationParams::into_event`] when the
    /// semantic parser handles the variant.
    #[serde(default)]
    pub event: Option<Value>,
}

impl KimiSubagentEvent {
    /// Decode the nested `event` payload into a typed [`KimiWireEvent`].
    pub fn nested_event(&self) -> Option<KimiWireEvent> {
        let raw = self.event.clone()?;
        let params: KimiNotificationParams = serde_json::from_value(raw).ok()?;
        params.into_event()
    }
}

/// `BtwBegin` event payload — Kimi "by the way" sidebar start marker.
#[derive(Debug, Default, Deserialize)]
pub struct KimiBtwBegin {
    #[serde(default)]
    pub topic: Option<String>,
}

/// `BtwEnd` event payload — companion to `BtwBegin`.
#[derive(Debug, Default, Deserialize)]
pub struct KimiBtwEnd {}

/// `HookTriggered` event payload — informational notice that a hook event
/// fired. Distinct from a `HookRequest` (which requires a response).
#[derive(Debug, Default, Deserialize)]
pub struct KimiHookTriggered {
    #[serde(default)]
    pub hook_name: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
}

/// `HookResolved` event payload — companion to `HookTriggered`.
#[derive(Debug, Default, Deserialize)]
pub struct KimiHookResolved {
    #[serde(default)]
    pub hook_name: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
}

/// `DiffDisplayBlock` event payload — surface for inline diff rendering.
#[derive(Debug, Default, Deserialize)]
pub struct KimiDiffDisplayBlock {
    #[serde(default)]
    pub is_summary: Option<bool>,
    #[serde(default)]
    pub diff: Option<String>,
}

// --- Request payload structs ------------------------------------------------

/// `ApprovalRequest` request payload — server prompts the client to approve a
/// tool action before running it. Auto-approved by Claudine in v1; surfaced
/// via a visible `Info { kind: "auto_approved" }` semantic event in Phase 2.
#[derive(Debug, Default, Deserialize)]
pub struct KimiApprovalRequest {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub subagent_type: Option<String>,
    #[serde(default)]
    pub source_description: Option<String>,
    /// Per-display-channel render hints. For shell tools, contains
    /// `[{"type":"shell","language":"bash","command":"..."}]`.
    #[serde(default)]
    pub display: Option<Value>,
}

impl KimiApprovalRequest {
    /// Best-effort shell-command extraction from the first `display` entry of
    /// `type == "shell"`. Returns `None` for non-shell approvals.
    pub fn shell_command(&self) -> Option<String> {
        let entries = self.display.as_ref()?.as_array()?;
        for entry in entries {
            if entry.get("type").and_then(Value::as_str) == Some("shell")
                && let Some(cmd) = entry.get("command").and_then(Value::as_str)
            {
                return Some(cmd.to_string());
            }
        }
        None
    }
}

/// `QuestionRequest` request payload — server-initiated user-input prompt.
/// Should not arrive in v1 (Claudine declares `supports_question: false` on
/// initialize). If it does, the parser surfaces a `Warning` and the IO loop
/// auto-responds with a synthetic empty answer.
#[derive(Debug, Default, Deserialize)]
pub struct KimiQuestionRequest {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub options: Option<Value>,
}

/// `ToolCallRequest` request payload — server asks the client to execute a
/// tool externally. Out of scope for v1; the IO loop returns a JSON-RPC
/// `METHOD_NOT_FOUND` error and the parser surfaces a `Warning`.
#[derive(Debug, Default, Deserialize)]
pub struct KimiToolCallRequest {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_call: Option<Value>,
}

/// `HookRequest` request payload — server asks Claudine to run hook actions
/// for a lifecycle event. Routed through `crate::dispatch` in Phase 5.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct KimiHookRequest {
    #[serde(default)]
    pub id: Option<String>,
    /// Canonical Kimi hook event name (e.g. `PreToolUse`, `Stop`).
    #[serde(default)]
    pub event: Option<String>,
    /// Free-form context body forwarded to the dispatch payload.
    #[serde(default)]
    pub context: Option<Value>,
}

// --- Response payload helpers -----------------------------------------------

/// Decoded shape of a successful `prompt` request response. Status is one of
/// `finished`, `cancelled`, `max_steps_reached`, `steered`. This is the
/// canonical end-of-turn signal — `TurnEnd.payload` is `{}` in current
/// captures.
#[derive(Debug, Default, Deserialize)]
pub struct KimiPromptResult {
    #[serde(default)]
    pub status: Option<String>,
}

impl KimiPromptResult {
    pub const STATUS_FINISHED: &'static str = "finished";
    pub const STATUS_CANCELLED: &'static str = "cancelled";
    pub const STATUS_MAX_STEPS_REACHED: &'static str = "max_steps_reached";
    pub const STATUS_STEERED: &'static str = "steered";
}

/// Decoded shape of a successful `initialize` request response. Carries the
/// negotiated protocol version, server identity, slash-command catalog,
/// supported hooks, and the negotiated capability set.
#[derive(Debug, Default, Deserialize)]
pub struct KimiInitializeResult {
    #[serde(default)]
    pub protocol_version: Option<String>,
    #[serde(default)]
    pub server: Option<KimiServerInfo>,
    #[serde(default)]
    pub slash_commands: Option<Value>,
    #[serde(default)]
    pub hooks: Option<Value>,
    #[serde(default)]
    pub capabilities: Option<KimiServerCapabilities>,
}

/// Server identity reported by `initialize`.
#[derive(Debug, Default, Deserialize)]
pub struct KimiServerInfo {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// Capability set the server advertises in its `initialize` response. Only
/// `supports_question` and `supports_plan_mode` are currently observed —
/// other hypothetical fields are silently ignored by the server.
#[derive(Debug, Default, Deserialize)]
pub struct KimiServerCapabilities {
    #[serde(default)]
    pub supports_question: Option<bool>,
    #[serde(default)]
    pub supports_plan_mode: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Wire-mode (JSON-RPC 2.0) protocol tests
    // ------------------------------------------------------------------

    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("src/stream/protocol/fixtures/kimi");
        p.push(name);
        p
    }

    fn load_fixture_lines(name: &str) -> Vec<Value> {
        let raw = std::fs::read_to_string(fixture_path(name))
            .unwrap_or_else(|e| panic!("read fixture {name}: {e}"));
        let mut out = Vec::new();
        for (idx, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(trimmed)
                .unwrap_or_else(|e| panic!("parse {name} line {}: {e}", idx + 1));
            out.push(value);
        }
        out
    }

    fn classify_lines(name: &str) -> Vec<KimiEnvelope> {
        load_fixture_lines(name)
            .into_iter()
            .filter_map(KimiEnvelope::classify)
            .collect()
    }

    #[test]
    fn envelope_classifies_notification() {
        let value = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "event",
            "params": {"type": "TurnEnd", "payload": {}},
        });
        let env = KimiEnvelope::classify(value).expect("classified");
        let KimiEnvelope::Notification(params) = env else {
            panic!("expected Notification");
        };
        assert_eq!(params.event_type, "TurnEnd");
    }

    #[test]
    fn envelope_classifies_request() {
        let value = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "request",
            "id": "req-1",
            "params": {"type": "ApprovalRequest", "payload": {"id": "a"}},
        });
        let env = KimiEnvelope::classify(value).expect("classified");
        let KimiEnvelope::Request { id, params } = env else {
            panic!("expected Request");
        };
        assert_eq!(id.as_str(), Some("req-1"));
        assert_eq!(params.request_type, "ApprovalRequest");
    }

    #[test]
    fn envelope_classifies_success_response() {
        let value = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "prompt-2",
            "result": {"status": "finished"},
        });
        let env = KimiEnvelope::classify(value).expect("classified");
        let KimiEnvelope::SuccessResponse { id, result } = env else {
            panic!("expected SuccessResponse");
        };
        assert_eq!(id.as_str(), Some("prompt-2"));
        let parsed: KimiPromptResult = serde_json::from_value(result).unwrap();
        assert_eq!(
            parsed.status.as_deref(),
            Some(KimiPromptResult::STATUS_FINISHED)
        );
    }

    #[test]
    fn envelope_classifies_error_response() {
        let value = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "prompt-2",
            "error": {"code": -32004, "message": "Auth expired", "data": null},
        });
        let env = KimiEnvelope::classify(value).expect("classified");
        let KimiEnvelope::ErrorResponse { id, error } = env else {
            panic!("expected ErrorResponse");
        };
        assert_eq!(id.as_str(), Some("prompt-2"));
        assert_eq!(error.code, KimiJsonRpcError::AUTH_EXPIRED);
        assert_eq!(error.message, "Auth expired");
    }

    #[test]
    fn envelope_returns_none_for_unknown_shape() {
        let value = serde_json::json!({"jsonrpc": "2.0"});
        assert!(KimiEnvelope::classify(value).is_none());
    }

    #[test]
    fn notification_params_decodes_typed_event() {
        let params = KimiNotificationParams {
            event_type: "StepBegin".into(),
            payload: serde_json::json!({"n": 3}),
        };
        let event = params.into_event().expect("typed event");
        let KimiWireEvent::StepBegin(payload) = event else {
            panic!("expected StepBegin");
        };
        assert_eq!(payload.n, Some(3));
    }

    #[test]
    fn notification_params_unknown_event_type_returns_none() {
        let params = KimiNotificationParams {
            event_type: "FuturisticEvent".into(),
            payload: Value::Null,
        };
        assert!(params.into_event().is_none());
    }

    #[test]
    fn request_params_decodes_typed_request() {
        let params = KimiRequestParams {
            request_type: "ApprovalRequest".into(),
            payload: serde_json::json!({
                "id": "abc",
                "tool_call_id": "tool_x",
                "sender": "Shell",
                "action": "run command",
                "description": "Run command `ls`",
                "display": [{"type": "shell", "language": "bash", "command": "ls"}]
            }),
        };
        let request = params.into_request().expect("typed request");
        let KimiWireRequest::Approval(approval) = request else {
            panic!("expected Approval");
        };
        assert_eq!(approval.id.as_deref(), Some("abc"));
        assert_eq!(approval.tool_call_id.as_deref(), Some("tool_x"));
        assert_eq!(approval.shell_command(), Some("ls".into()));
    }

    #[test]
    fn request_params_unknown_request_type_returns_none() {
        let params = KimiRequestParams {
            request_type: "FuturisticRequest".into(),
            payload: Value::Null,
        };
        assert!(params.into_request().is_none());
    }

    #[test]
    fn wire_event_unknown_type_fails_typed() {
        let envelope = serde_json::json!({"type": "FuturisticEvent", "payload": {}});
        let result: Result<KimiWireEvent, _> = serde_json::from_value(envelope);
        assert!(result.is_err());
    }

    #[test]
    fn wire_request_unknown_type_fails_typed() {
        let envelope = serde_json::json!({"type": "FuturisticRequest", "payload": {}});
        let result: Result<KimiWireRequest, _> = serde_json::from_value(envelope);
        assert!(result.is_err());
    }

    #[test]
    fn turn_begin_user_input_text_handles_string() {
        let envelope = serde_json::json!({
            "type": "TurnBegin",
            "payload": {"user_input": "Hi how are you?"},
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::TurnBegin(payload) = event else {
            panic!("expected TurnBegin");
        };
        assert_eq!(payload.user_input_text(), Some("Hi how are you?".into()));
    }

    #[test]
    fn turn_begin_user_input_text_handles_array() {
        let envelope = serde_json::json!({
            "type": "TurnBegin",
            "payload": {"user_input": [
                {"type": "text", "text": "Hi "},
                {"type": "text", "text": "Bob"}
            ]},
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::TurnBegin(payload) = event else {
            panic!("expected TurnBegin");
        };
        assert_eq!(payload.user_input_text(), Some("Hi Bob".into()));
    }

    #[test]
    fn content_part_text_resolves_text() {
        let envelope = serde_json::json!({
            "type": "ContentPart",
            "payload": {"type": "text", "text": "Hi Bob!"},
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ContentPart(part) = event else {
            panic!("expected ContentPart");
        };
        assert!(part.is_text());
        assert!(!part.is_thinking());
        assert_eq!(part.resolved_text(), Some("Hi Bob!"));
    }

    #[test]
    fn content_part_think_resolves_thinking() {
        let envelope = serde_json::json!({
            "type": "ContentPart",
            "payload": {"type": "think", "think": "The user...", "encrypted": null},
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ContentPart(part) = event else {
            panic!("expected ContentPart");
        };
        assert!(part.is_thinking());
        assert!(!part.is_text());
        assert_eq!(part.resolved_text(), Some("The user..."));
    }

    #[test]
    fn content_part_image_url_returns_no_inline_text() {
        let envelope = serde_json::json!({
            "type": "ContentPart",
            "payload": {"type": "image_url", "image_url": "https://example.test/x.png"},
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ContentPart(part) = event else {
            panic!("expected ContentPart");
        };
        assert_eq!(part.resolved_text(), None);
    }

    #[test]
    fn status_update_computes_percent_from_fraction() {
        let envelope = serde_json::json!({
            "type": "StatusUpdate",
            "payload": {
                "context_usage": 0.06,
                "context_tokens": 15969,
                "max_context_tokens": 262144,
                "token_usage": {"input_other": 10081, "output": 52, "input_cache_read": 5888, "input_cache_creation": 0},
                "message_id": "chatcmpl-abc",
                "plan_mode": false,
                "mcp_status": null
            }
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::StatusUpdate(status) = event else {
            panic!("expected StatusUpdate");
        };
        let pct = status.computed_context_percent().expect("percent");
        assert!((pct - 6.0).abs() < 0.01, "percent was {pct}");
        let usage = status.token_usage.as_ref().expect("token_usage");
        assert_eq!(usage.total_input(), Some(10081 + 5888));
        assert_eq!(usage.cache_read_input(), Some(5888));
        assert_eq!(status.message_id.as_deref(), Some("chatcmpl-abc"));
    }

    #[test]
    fn status_update_computes_percent_from_token_counters() {
        let envelope = serde_json::json!({
            "type": "StatusUpdate",
            "payload": {"context_tokens": 100, "max_context_tokens": 200}
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::StatusUpdate(status) = event else {
            panic!("expected StatusUpdate");
        };
        assert_eq!(status.computed_context_percent(), Some(50.0));
    }

    #[test]
    fn tool_call_arguments_string_round_trip() {
        let envelope = serde_json::json!({
            "type": "ToolCall",
            "payload": {
                "type": "function",
                "id": "tool_x",
                "function": {"name": "Shell", "arguments": ""},
                "extras": null
            }
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ToolCall(mut call) = event else {
            panic!("expected ToolCall");
        };
        assert_eq!(call.resolved_tool_id(), Some("tool_x"));
        assert_eq!(call.resolved_tool_name(), Some("Shell"));
        // Empty initial arguments — subsequent ToolCallPart deltas accumulate the body.
        assert_eq!(call.take_arguments_string().as_deref(), Some(""));
    }

    #[test]
    fn parse_arguments_string_decodes_valid_json() {
        let parsed = KimiToolCall::parse_arguments_string(r#"{"command":"ls"}"#).unwrap();
        let value = parsed.expect("parsed");
        assert_eq!(value.get("command").and_then(Value::as_str), Some("ls"));
    }

    #[test]
    fn parse_arguments_string_returns_none_for_empty() {
        let parsed = KimiToolCall::parse_arguments_string("").unwrap();
        assert!(parsed.is_none());
        let parsed = KimiToolCall::parse_arguments_string("   ").unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_arguments_string_passthrough_on_malformed_json() {
        let err = KimiToolCall::parse_arguments_string("{not json").unwrap_err();
        assert_eq!(err, "{not json");
    }

    #[test]
    fn tool_call_part_carries_argument_delta() {
        let envelope = serde_json::json!({
            "type": "ToolCallPart",
            "payload": {"arguments_part": "{\"command\":"}
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ToolCallPart(part) = event else {
            panic!("expected ToolCallPart");
        };
        assert_eq!(part.arguments_part.as_deref(), Some("{\"command\":"));
    }

    #[test]
    fn tool_result_returns_status_and_output() {
        let envelope = serde_json::json!({
            "type": "ToolResult",
            "payload": {
                "tool_call_id": "tool_x",
                "return_value": {
                    "is_error": false,
                    "output": "hello-from-kimi\n",
                    "message": "Command executed successfully.",
                    "display": [],
                    "extras": null
                }
            }
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ToolResult(mut result) = event else {
            panic!("expected ToolResult");
        };
        assert_eq!(result.resolved_tool_id(), Some("tool_x"));
        assert_eq!(result.derived_status(), "success");
        let output = result.take_output().expect("output");
        assert_eq!(output.as_str(), Some("hello-from-kimi\n"));
    }

    #[test]
    fn tool_result_marks_errors() {
        let envelope = serde_json::json!({
            "type": "ToolResult",
            "payload": {
                "tool_call_id": "tool_y",
                "return_value": {"is_error": true, "output": "permission denied"}
            }
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ToolResult(result) = event else {
            panic!("expected ToolResult");
        };
        assert!(result.is_error());
        assert_eq!(result.derived_status(), "error");
    }

    #[test]
    fn approval_request_extracts_shell_command() {
        let envelope = serde_json::json!({
            "type": "ApprovalRequest",
            "payload": {
                "id": "appr-1",
                "tool_call_id": "tool_x",
                "sender": "Shell",
                "action": "run command",
                "description": "Run command `echo hi`",
                "source_kind": "foreground_turn",
                "source_id": "src-1",
                "display": [{"type": "shell", "language": "bash", "command": "echo hi"}]
            }
        });
        let request: KimiWireRequest = serde_json::from_value(envelope).unwrap();
        let KimiWireRequest::Approval(approval) = request else {
            panic!("expected Approval");
        };
        assert_eq!(approval.shell_command(), Some("echo hi".into()));
        assert_eq!(approval.action.as_deref(), Some("run command"));
    }

    #[test]
    fn approval_request_no_shell_returns_none() {
        let envelope = serde_json::json!({
            "type": "ApprovalRequest",
            "payload": {
                "id": "appr-2",
                "display": [{"type": "markdown", "body": "..."}]
            }
        });
        let request: KimiWireRequest = serde_json::from_value(envelope).unwrap();
        let KimiWireRequest::Approval(approval) = request else {
            panic!("expected Approval");
        };
        assert_eq!(approval.shell_command(), None);
    }

    #[test]
    fn subagent_event_decodes_nested_event() {
        let envelope = serde_json::json!({
            "type": "SubagentEvent",
            "payload": {
                "parent_tool_call_id": "tool_p",
                "agent_id": "ab1",
                "subagent_type": "explore",
                "event": {
                    "type": "StepBegin",
                    "payload": {"n": 1}
                }
            }
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::SubagentEvent(sub) = event else {
            panic!("expected SubagentEvent");
        };
        assert_eq!(sub.subagent_type.as_deref(), Some("explore"));
        let nested = sub.nested_event().expect("nested");
        let KimiWireEvent::StepBegin(payload) = nested else {
            panic!("expected nested StepBegin");
        };
        assert_eq!(payload.n, Some(1));
    }

    #[test]
    fn approval_response_event_round_trip() {
        let envelope = serde_json::json!({
            "type": "ApprovalResponse",
            "payload": {"request_id": "abc", "response": "approve", "feedback": ""}
        });
        let event: KimiWireEvent = serde_json::from_value(envelope).unwrap();
        let KimiWireEvent::ApprovalResponse(echo) = event else {
            panic!("expected ApprovalResponse");
        };
        assert_eq!(echo.request_id.as_deref(), Some("abc"));
        assert_eq!(echo.response.as_deref(), Some("approve"));
    }

    #[test]
    fn initialize_result_decodes_capabilities() {
        let result = serde_json::json!({
            "protocol_version": "1.9",
            "server": {"name": "Kimi Code CLI", "version": "1.38.0"},
            "slash_commands": [],
            "hooks": [],
            "capabilities": {"supports_question": true, "supports_plan_mode": false}
        });
        let parsed: KimiInitializeResult = serde_json::from_value(result).unwrap();
        assert_eq!(parsed.protocol_version.as_deref(), Some("1.9"));
        let server = parsed.server.expect("server");
        assert_eq!(server.name.as_deref(), Some("Kimi Code CLI"));
        let caps = parsed.capabilities.expect("capabilities");
        assert_eq!(caps.supports_question, Some(true));
        assert_eq!(caps.supports_plan_mode, Some(false));
    }

    #[test]
    fn prompt_result_recognizes_known_statuses() {
        for status in [
            KimiPromptResult::STATUS_FINISHED,
            KimiPromptResult::STATUS_CANCELLED,
            KimiPromptResult::STATUS_MAX_STEPS_REACHED,
            KimiPromptResult::STATUS_STEERED,
        ] {
            let v = serde_json::json!({"status": status});
            let parsed: KimiPromptResult = serde_json::from_value(v).unwrap();
            assert_eq!(parsed.status.as_deref(), Some(status));
        }
    }

    // ------------------------------------------------------------------
    // Fixture replay tests
    // ------------------------------------------------------------------

    #[test]
    fn wire_greet_fixture_classifies_every_line() {
        let envelopes = classify_lines("wire-greet.jsonl");
        assert!(!envelopes.is_empty());
        let mut notif = 0;
        let mut requests = 0;
        let mut success = 0;
        let mut errors = 0;
        for env in &envelopes {
            match env {
                KimiEnvelope::Notification(_) => notif += 1,
                KimiEnvelope::Request { .. } => requests += 1,
                KimiEnvelope::SuccessResponse { .. } => success += 1,
                KimiEnvelope::ErrorResponse { .. } => errors += 1,
            }
        }
        assert!(notif > 0, "expected at least one notification");
        assert!(
            success >= 2,
            "expected initialize and prompt success responses, got {success}"
        );
        assert_eq!(requests, 0, "greet fixture should have no server requests");
        assert_eq!(errors, 0, "greet fixture should have no error responses");
    }

    #[test]
    fn wire_greet_fixture_produces_typed_events() {
        let envelopes = classify_lines("wire-greet.jsonl");
        let mut typed = 0;
        let mut think_parts = 0;
        let mut text_parts = 0;
        let mut turn_begin = 0;
        let mut turn_end = 0;
        let mut status_updates = 0;
        for env in envelopes {
            if let KimiEnvelope::Notification(params) = env
                && let Some(typed_event) = params.into_event()
            {
                typed += 1;
                match typed_event {
                    KimiWireEvent::TurnBegin(_) => turn_begin += 1,
                    KimiWireEvent::TurnEnd(_) => turn_end += 1,
                    KimiWireEvent::StatusUpdate(_) => status_updates += 1,
                    KimiWireEvent::ContentPart(part) => {
                        if part.is_thinking() {
                            think_parts += 1;
                        } else if part.is_text() {
                            text_parts += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        assert!(typed > 0);
        assert_eq!(turn_begin, 1, "expected exactly one TurnBegin");
        assert_eq!(turn_end, 1, "expected exactly one TurnEnd");
        assert!(status_updates >= 1, "expected at least one StatusUpdate");
        assert!(think_parts > 0, "expected reasoning ContentPart deltas");
        assert!(text_parts > 0, "expected assistant text ContentPart deltas");
    }

    #[test]
    fn wire_greet_fixture_prompt_status_finished() {
        let envelopes = classify_lines("wire-greet.jsonl");
        let mut last_prompt_status: Option<String> = None;
        for env in envelopes {
            if let KimiEnvelope::SuccessResponse { id, result } = env
                && id.as_str() == Some("prompt-2")
                && let Ok(parsed) = serde_json::from_value::<KimiPromptResult>(result)
            {
                last_prompt_status = parsed.status;
            }
        }
        assert_eq!(
            last_prompt_status.as_deref(),
            Some(KimiPromptResult::STATUS_FINISHED)
        );
    }

    #[test]
    fn wire_tool_shell_fixture_covers_tool_lifecycle() {
        let envelopes = classify_lines("wire-tool-shell.jsonl");
        let mut tool_calls = 0;
        let mut tool_call_parts = 0;
        let mut tool_results = 0;
        let mut approval_requests = 0;
        let mut approval_responses = 0;
        for env in envelopes {
            match env {
                KimiEnvelope::Notification(params) => {
                    if let Some(typed) = params.into_event() {
                        match typed {
                            KimiWireEvent::ToolCall(_) => tool_calls += 1,
                            KimiWireEvent::ToolCallPart(_) => tool_call_parts += 1,
                            KimiWireEvent::ToolResult(_) => tool_results += 1,
                            KimiWireEvent::ApprovalResponse(_) => approval_responses += 1,
                            _ => {}
                        }
                    }
                }
                KimiEnvelope::Request { params, .. } => {
                    if let Some(KimiWireRequest::Approval(_)) = params.into_request() {
                        approval_requests += 1;
                    }
                }
                _ => {}
            }
        }
        assert!(tool_calls >= 1, "expected at least one ToolCall");
        assert!(tool_call_parts > 0, "expected ToolCallPart deltas");
        assert!(tool_results >= 1, "expected at least one ToolResult");
        assert!(approval_requests >= 1, "expected ApprovalRequest");
        assert!(approval_responses >= 1, "expected ApprovalResponse echo");
    }

    #[test]
    fn wire_tool_shell_fixture_arguments_decode() {
        let envelopes = classify_lines("wire-tool-shell.jsonl");
        let mut arg_buffer = String::new();
        let mut tool_call_seen = false;
        for env in envelopes {
            if let KimiEnvelope::Notification(params) = env
                && let Some(typed) = params.into_event()
            {
                match typed {
                    KimiWireEvent::ToolCall(mut call) => {
                        tool_call_seen = true;
                        if let Some(initial) = call.take_arguments_string() {
                            arg_buffer.push_str(&initial);
                        }
                    }
                    KimiWireEvent::ToolCallPart(part) => {
                        if let Some(delta) = part.arguments_part {
                            arg_buffer.push_str(&delta);
                        }
                    }
                    _ => {}
                }
            }
        }
        assert!(tool_call_seen);
        let parsed = KimiToolCall::parse_arguments_string(&arg_buffer)
            .unwrap()
            .expect("parsed args");
        assert_eq!(
            parsed.get("command").and_then(Value::as_str),
            Some("echo hello-from-kimi")
        );
    }

    #[test]
    fn wire_subagent_fixture_nested_events_decode() {
        let envelopes = classify_lines("wire-subagent.jsonl");
        let mut subagent_count = 0;
        let mut nested_typed = 0;
        for env in envelopes {
            if let KimiEnvelope::Notification(params) = env
                && let Some(KimiWireEvent::SubagentEvent(sub)) = params.into_event()
            {
                subagent_count += 1;
                if sub.nested_event().is_some() {
                    nested_typed += 1;
                }
            }
        }
        assert!(subagent_count > 0, "expected at least one SubagentEvent");
        assert!(
            nested_typed > 0,
            "expected at least one nested event to decode"
        );
    }

    #[test]
    fn wire_cancelled_fixture_prompt_status_cancelled() {
        let envelopes = classify_lines("wire-cancelled.jsonl");
        let mut prompt_status: Option<String> = None;
        for env in envelopes {
            if let KimiEnvelope::SuccessResponse { id, result } = env
                && id.as_str() == Some("prompt-2")
                && let Ok(parsed) = serde_json::from_value::<KimiPromptResult>(result)
            {
                prompt_status = parsed.status;
            }
        }
        assert_eq!(
            prompt_status.as_deref(),
            Some(KimiPromptResult::STATUS_CANCELLED)
        );
    }

    #[test]
    fn wire_auth_expired_fixture_classifies_error_response() {
        let envelopes = classify_lines("wire-auth-expired.jsonl");
        let mut found_auth_expired = false;
        for env in envelopes {
            if let KimiEnvelope::ErrorResponse { id, error } = env
                && id.as_str() == Some("prompt-2")
                && error.code == KimiJsonRpcError::AUTH_EXPIRED
            {
                found_auth_expired = true;
            }
        }
        assert!(
            found_auth_expired,
            "expected an AUTH_EXPIRED error response on prompt-2"
        );
    }
}
