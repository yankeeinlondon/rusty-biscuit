//! Typed event models for Kimi Code's `--wire` JSON-RPC 2.0 protocol.
//!
//! `KimiEnvelope` plus the `KimiWireEvent` / `KimiWireRequest` enums and
//! their typed payload structs model the protocol Kimi exposes via
//! `kimi --wire`. All structs derive `Deserialize` with `#[serde(default)]`
//! on every field; unknown `event.type` / `request.type` discriminants fail
//! typed deserialization so the semantic parser can fall back to a raw
//! `ProviderExtension` event.
//!
//! Wire 1.10 also introduced on-disk session surfaces — background-task
//! directories (`tasks/`) and compaction snapshots (`context_{N}.jsonl`) —
//! that are known but deliberately unmodeled here (deferred; they are
//! storage artifacts, not wire envelopes).

use serde::{Deserialize, Serialize};
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

/// Wire protocol versions this Claudine release can drive. The CLI wiring
/// advertises the newest entry on its `initialize` request; the semantic
/// parser accepts any response version in this window and emits a terminal
/// remediation-bearing `Configuration` error for anything else.
///
/// The Wire protocol version is an axis independent of both product version
/// lines — the legacy Python kimi-cli 1.x (state under `~/.kimi`) and the
/// TypeScript Kimi Code binary 0.x (state under `~/.kimi-code` /
/// `KIMI_CODE_HOME`) each negotiate their own Wire revision.
pub const SUPPORTED_WIRE_PROTOCOL_VERSIONS: &[&str] = &["1.9", "1.10"];

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
        Self::from_raw(raw)
    }

    /// Classify a raw JSON line directly into a typed wire envelope.
    ///
    /// This avoids the intermediate `serde_json::Value` DOM allocation that
    /// [`Self::classify`] requires, making it suitable for the hot path in
    /// the semantic stream parser.
    pub fn classify_str(line: &str) -> Option<Self> {
        let raw: KimiRawEnvelope = serde_json::from_str(line).ok()?;
        Self::from_raw(raw)
    }

    fn from_raw(raw: KimiRawEnvelope) -> Option<Self> {
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

    /// Returns the raw kind string for this envelope, matching the format
    /// produced by the legacy `envelope_raw_kind(Value)` helper.
    pub fn raw_kind(&self) -> String {
        match self {
            KimiEnvelope::Notification(params) => {
                format!("event:{}", params.event_type)
            }
            KimiEnvelope::Request { params, .. } => {
                format!("request:{}", params.request_type)
            }
            KimiEnvelope::SuccessResponse { .. } => "response".into(),
            KimiEnvelope::ErrorResponse { .. } => "error_response".into(),
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
///
/// The Wire 1.10 additions (`StepRetry`, `StatusUpdate.mcp_status`, the
/// richer `Notification` payload) are sourced from the Python kimi-cli Wire
/// source (`wire/types.py`); parity for the TypeScript kimi-code binary is
/// unconfirmed — research marks its wire envelope coverage partial.
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
    #[serde(rename = "StepRetry")]
    StepRetry(KimiStepRetry),
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

/// `StepRetry` payload (Wire 1.10) — an API call inside a step failed and is
/// being retried. `wait_s` is the backoff delay in float seconds;
/// `error_type` is the originating exception class (e.g.
/// `APIEmptyResponseError`); `status_code` is the HTTP status when one was
/// observed.
#[derive(Debug, Default, Deserialize)]
pub struct KimiStepRetry {
    #[serde(default)]
    pub n: Option<u64>,
    #[serde(default)]
    pub next_attempt: Option<u64>,
    #[serde(default)]
    pub max_attempts: Option<u64>,
    #[serde(default)]
    pub wait_s: Option<f64>,
    #[serde(default)]
    pub error_type: Option<String>,
    #[serde(default)]
    pub status_code: Option<i64>,
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
    pub mcp_status: Option<KimiMcpStatusSnapshot>,
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

/// `mcp_status` block on `StatusUpdate` (Wire 1.10) — aggregate MCP server
/// connection state. Derives `Serialize` (unlike sibling payload structs)
/// so the parser can project the snapshot into event `extra`.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct KimiMcpStatusSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loading: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<KimiMcpServerSnapshot>>,
}

/// Per-server entry in `mcp_status.servers`. `status` is one of `pending`,
/// `connecting`, `connected`, `failed`, `unauthorized`.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct KimiMcpServerSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
}

/// `Notification` event payload — server-emitted notice.
///
/// Decodes both notification shapes additively: the Wire 1.9 fields
/// (`level`/`title`/`message`/`source` — also the shape Claudine's own
/// synthetic warning envelopes use) and the Wire 1.10 replacement payload
/// (`id`/`category`/`type`/`source_kind`/`source_id`/`title`/`body`/
/// `severity`/`created_at`/`payload`). The parser resolves the message as
/// `message` → `body` → `title` and the warning level as `level` →
/// `severity` so both revisions render.
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
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default, rename = "type")]
    pub notification_type: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    /// Float Unix timestamp; unit/timezone are not formally specified
    /// upstream.
    #[serde(default)]
    pub created_at: Option<f64>,
    #[serde(default)]
    pub payload: Option<Value>,
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
/// auto-responds with a synthetic empty answer keyed on the JSON-RPC
/// envelope `id` (not the payload `id` or `tool_call_id`).
///
/// Two wire shapes coexist: since Wire 1.4 the payload is
/// `{id, tool_call_id, questions: [...]}`; older wires sent a flat
/// `{id, question, options}`. Both field sets are kept so either shape
/// deserializes; use [`Self::primary_question`] instead of reading the
/// fields directly.
#[derive(Debug, Default, Deserialize)]
pub struct KimiQuestionRequest {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// Current wire (>= 1.4): list of question items.
    #[serde(default)]
    pub questions: Option<Vec<KimiQuestionItem>>,
    /// Legacy flat shape: single question text.
    #[serde(default)]
    pub question: Option<String>,
    /// Legacy flat shape: option list.
    #[serde(default)]
    pub options: Option<Value>,
}

impl KimiQuestionRequest {
    /// First question text across both wire shapes: the first non-empty
    /// `questions[].question` when present, else the legacy flat `question`.
    pub fn primary_question(&self) -> Option<&str> {
        self.questions
            .as_ref()
            .and_then(|items| items.iter().find_map(|item| item.question.as_deref()))
            .or(self.question.as_deref())
    }
}

/// One entry of `QuestionRequest.questions` (Wire >= 1.4).
#[derive(Debug, Default, Deserialize)]
pub struct KimiQuestionItem {
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub header: Option<String>,
    /// `[{label, description}]` per the wire schema; kept open-shaped since
    /// Claudine never renders options (it auto-answers empty).
    #[serde(default)]
    pub options: Option<Value>,
    #[serde(default)]
    pub multi_select: Option<bool>,
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
    /// Step count accompanying the terminal status; observed alongside
    /// `max_steps_reached`, where it carries the configured loop limit.
    #[serde(default)]
    pub steps: Option<u64>,
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
mod tests;
