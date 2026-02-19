# Kimi Code Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All 14 Wire protocol events exposed by Kimi Code CLI.
///
/// Kimi Code does not use file-based hook configuration. Instead, events are
/// delivered over a JSON-RPC 2.0 bidirectional protocol ("Wire mode") via
/// stdin/stdout. Events fall into two categories:
///
/// - **Notifications** (agent -> client): fire-and-forget, no response required.
/// - **Requests** (agent -> client): blocking, the agent pauses until a
///   JSON-RPC response is returned by the client.
///
/// Variant names use Kimi's native PascalCase event type strings so that
/// serialization and debug output align directly with the Wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KimiCodeEvent {
    // -- Notifications (fire-and-forget) --
    TurnBegin,
    TurnEnd,
    StepBegin,
    StepInterrupted,
    CompactionBegin,
    CompactionEnd,
    StatusUpdate,
    ContentPart,
    ToolCall,
    ToolCallPart,
    ToolResult,
    ApprovalResponse,
    SubagentEvent,

    // -- Blocking requests --
    ApprovalRequest,
    ToolCallRequest,
}

impl KimiCodeEvent {
    /// All variants in lifecycle order (notifications first, then requests).
    pub const ALL: [KimiCodeEvent; 15] = [
        Self::TurnBegin,
        Self::TurnEnd,
        Self::StepBegin,
        Self::StepInterrupted,
        Self::CompactionBegin,
        Self::CompactionEnd,
        Self::StatusUpdate,
        Self::ContentPart,
        Self::ToolCall,
        Self::ToolCallPart,
        Self::ToolResult,
        Self::ApprovalResponse,
        Self::SubagentEvent,
        Self::ApprovalRequest,
        Self::ToolCallRequest,
    ];

    /// The native event type string as it appears in the Wire protocol
    /// `params.type` field.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::TurnBegin => "TurnBegin",
            Self::TurnEnd => "TurnEnd",
            Self::StepBegin => "StepBegin",
            Self::StepInterrupted => "StepInterrupted",
            Self::CompactionBegin => "CompactionBegin",
            Self::CompactionEnd => "CompactionEnd",
            Self::StatusUpdate => "StatusUpdate",
            Self::ContentPart => "ContentPart",
            Self::ToolCall => "ToolCall",
            Self::ToolCallPart => "ToolCallPart",
            Self::ToolResult => "ToolResult",
            Self::ApprovalResponse => "ApprovalResponse",
            Self::SubagentEvent => "SubagentEvent",
            Self::ApprovalRequest => "ApprovalRequest",
            Self::ToolCallRequest => "ToolCallRequest",
        }
    }

    /// Kimi Code-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::TurnBegin => "Signals the start of a new agent turn",
            Self::TurnEnd => {
                "Signals the clean end of a turn (may be omitted on cancel/interrupt)"
            }
            Self::StepBegin => "Marks the beginning of a step within the current turn",
            Self::StepInterrupted => "Indicates the current step was interrupted",
            Self::CompactionBegin => "Signals the start of context compaction",
            Self::CompactionEnd => "Signals the end of context compaction",
            Self::StatusUpdate => {
                "Reports telemetry about context usage and token consumption"
            }
            Self::ContentPart => {
                "Streams a fragment of the agent's response (text, thinking, or media)"
            }
            Self::ToolCall => "Signals that a tool call is being formed",
            Self::ToolCallPart => "Streams a fragment of tool call arguments",
            Self::ToolResult => "Reports the result of a tool execution",
            Self::ApprovalResponse => {
                "Notifies that a prior ApprovalRequest was resolved"
            }
            Self::SubagentEvent => {
                "Forwards a nested event from a subagent spawned via the Task tool"
            }
            Self::ApprovalRequest => {
                "Agent requests permission to execute a tool action (blocking)"
            }
            Self::ToolCallRequest => {
                "Agent invokes an external tool registered during initialize (blocking)"
            }
        }
    }

    /// Whether this event is a blocking request (requires a JSON-RPC response)
    /// or a fire-and-forget notification.
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::ApprovalRequest | Self::ToolCallRequest)
    }

    /// Whether this event is a notification (no response required).
    pub fn is_notification(&self) -> bool {
        !self.is_blocking()
    }

    /// The JSON-RPC method name used to deliver this event.
    ///
    /// Notifications use `"event"`, blocking requests use `"request"`.
    pub fn wire_method(&self) -> &'static str {
        if self.is_blocking() {
            "request"
        } else {
            "event"
        }
    }

    /// The typed payload structure this event provides.
    pub fn payload_type(&self) -> PayloadType {
        match self {
            Self::TurnBegin => PayloadType::TurnBegin,
            Self::TurnEnd => PayloadType::Empty,
            Self::StepBegin => PayloadType::StepBegin,
            Self::StepInterrupted => PayloadType::Empty,
            Self::CompactionBegin => PayloadType::Empty,
            Self::CompactionEnd => PayloadType::Empty,
            Self::StatusUpdate => PayloadType::StatusUpdate,
            Self::ContentPart => PayloadType::ContentPart,
            Self::ToolCall => PayloadType::ToolCall,
            Self::ToolCallPart => PayloadType::ToolCallPart,
            Self::ToolResult => PayloadType::ToolResult,
            Self::ApprovalResponse => PayloadType::ApprovalResponse,
            Self::SubagentEvent => PayloadType::SubagentEvent,
            Self::ApprovalRequest => PayloadType::ApprovalRequest,
            Self::ToolCallRequest => PayloadType::ToolCallRequest,
        }
    }

    /// The typed response structure this event expects from the client.
    ///
    /// Returns `None` for notification events (no response expected).
    pub fn response_type(&self) -> Option<ResponseType> {
        match self {
            Self::ApprovalRequest => Some(ResponseType::ApprovalResponse),
            Self::ToolCallRequest => Some(ResponseType::ToolResult),
            _ => None,
        }
    }

    /// The Wire protocol version that introduced this event.
    pub fn introduced_in(&self) -> &'static str {
        match self {
            Self::TurnEnd => "1.2",
            _ => "1.0",
        }
    }

    /// Whether this event carries streaming content that must be buffered
    /// and concatenated before use.
    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::ContentPart | Self::ToolCallPart)
    }

    /// Whether this event relates to tool execution.
    pub fn is_tool_related(&self) -> bool {
        matches!(
            self,
            Self::ToolCall
                | Self::ToolCallPart
                | Self::ToolResult
                | Self::ApprovalRequest
                | Self::ToolCallRequest
        )
    }
}

impl fmt::Display for KimiCodeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native event type strings
// ---------------------------------------------------------------------------

impl TryFrom<&str> for KimiCodeEvent {
    type Error = UnknownEventError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "TurnBegin" => Ok(Self::TurnBegin),
            "TurnEnd" => Ok(Self::TurnEnd),
            "StepBegin" => Ok(Self::StepBegin),
            "StepInterrupted" => Ok(Self::StepInterrupted),
            "CompactionBegin" => Ok(Self::CompactionBegin),
            "CompactionEnd" => Ok(Self::CompactionEnd),
            "StatusUpdate" => Ok(Self::StatusUpdate),
            "ContentPart" => Ok(Self::ContentPart),
            "ToolCall" => Ok(Self::ToolCall),
            "ToolCallPart" => Ok(Self::ToolCallPart),
            "ToolResult" => Ok(Self::ToolResult),
            // Support both the current and legacy name (Wire 1.1 rename)
            "ApprovalResponse" | "ApprovalRequestResolved" => Ok(Self::ApprovalResponse),
            "SubagentEvent" => Ok(Self::SubagentEvent),
            "ApprovalRequest" => Ok(Self::ApprovalRequest),
            "ToolCallRequest" => Ok(Self::ToolCallRequest),
            _ => Err(UnknownEventError(name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known Kimi Code event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown Kimi Code event: {0}")]
pub struct UnknownEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

impl From<KimiCodeEvent> for AgenticEvent {
    fn from(event: KimiCodeEvent) -> Self {
        match event {
            KimiCodeEvent::TurnBegin => AgenticEvent::BeforePrompt,
            KimiCodeEvent::TurnEnd => AgenticEvent::TurnComplete,
            KimiCodeEvent::StepBegin => AgenticEvent::BeforeModel,
            KimiCodeEvent::StepInterrupted => AgenticEvent::TurnError,
            KimiCodeEvent::CompactionBegin => AgenticEvent::BeforeCompact,
            // CompactionEnd has no direct unified equivalent; AfterModel is
            // the closest post-processing signal.
            KimiCodeEvent::CompactionEnd => AgenticEvent::AfterModel,
            KimiCodeEvent::StatusUpdate => AgenticEvent::Notification,
            KimiCodeEvent::ContentPart => AgenticEvent::AfterModel,
            KimiCodeEvent::ToolCall => AgenticEvent::BeforeTool,
            // ToolCallPart is streaming fragments of tool arguments; maps
            // to BeforeTool since it is still pre-execution.
            KimiCodeEvent::ToolCallPart => AgenticEvent::BeforeTool,
            KimiCodeEvent::ToolResult => AgenticEvent::AfterTool,
            KimiCodeEvent::ApprovalResponse => AgenticEvent::PermissionRequest,
            KimiCodeEvent::SubagentEvent => AgenticEvent::SubagentStart,
            KimiCodeEvent::ApprovalRequest => AgenticEvent::PermissionRequest,
            KimiCodeEvent::ToolCallRequest => AgenticEvent::BeforeTool,
        }
    }
}

impl TryFrom<AgenticEvent> for KimiCodeEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified to Kimi Code event.
    ///
    /// Events that have a clear primary Kimi Code equivalent succeed.
    /// Events that have no Kimi Code equivalent return `Err`.
    /// Events where multiple Kimi Code events map to the same unified
    /// event return the primary (most semantically important) mapping.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::BeforePrompt => Ok(KimiCodeEvent::TurnBegin),
            AgenticEvent::TurnComplete => Ok(KimiCodeEvent::TurnEnd),
            AgenticEvent::BeforeModel => Ok(KimiCodeEvent::StepBegin),
            AgenticEvent::TurnError => Ok(KimiCodeEvent::StepInterrupted),
            AgenticEvent::BeforeCompact => Ok(KimiCodeEvent::CompactionBegin),
            AgenticEvent::AfterModel => Ok(KimiCodeEvent::ContentPart),
            AgenticEvent::Notification => Ok(KimiCodeEvent::StatusUpdate),
            AgenticEvent::BeforeTool => Ok(KimiCodeEvent::ToolCall),
            AgenticEvent::AfterTool => Ok(KimiCodeEvent::ToolResult),
            AgenticEvent::PermissionRequest => Ok(KimiCodeEvent::ApprovalRequest),
            AgenticEvent::ToolError => Ok(KimiCodeEvent::ToolResult),
            AgenticEvent::SubagentStart => Ok(KimiCodeEvent::SubagentEvent),
            // Kimi Code does not support these unified events
            AgenticEvent::SessionStart
            | AgenticEvent::SessionEnd
            | AgenticEvent::SubagentStop
            | AgenticEvent::HumanInTheLoop => {
                Err("Kimi Code does not support this event")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// Discriminant for the event-specific payload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    TurnBegin,
    StepBegin,
    StatusUpdate,
    ContentPart,
    ToolCall,
    ToolCallPart,
    ToolResult,
    ApprovalResponse,
    SubagentEvent,
    ApprovalRequest,
    ToolCallRequest,
    /// Events with no payload fields (TurnEnd, StepInterrupted,
    /// CompactionBegin, CompactionEnd).
    Empty,
}

/// Discriminant for the response type expected from the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    /// ApprovalRequest expects an ApprovalResponse with approve/reject.
    ApprovalResponse,
    /// ToolCallRequest expects a ToolResult with the tool's output.
    ToolResult,
}

/// Approval decision values for responding to an `ApprovalRequest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Proceed with this tool call.
    Approve,
    /// Proceed and auto-approve similar operations for the session.
    ApproveForSession,
    /// Cancel the tool call; the agent adjusts its plan.
    Reject,
}

impl ApprovalDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::ApproveForSession => "approve_for_session",
            Self::Reject => "reject",
        }
    }
}

/// The `status` field values in a `prompt` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptStatus {
    /// Turn completed normally.
    Finished,
    /// Turn was cancelled via `cancel`.
    Cancelled,
    /// Hit the step limit.
    MaxStepsReached,
}

// ---------------------------------------------------------------------------
// ContentPart variants
// ---------------------------------------------------------------------------

/// Content part type for the `ContentPart` event.
///
/// The agent streams response fragments of various media types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPartVariant {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "think")]
    Think {
        think: String,
        #[serde(default)]
        encrypted: Option<String>,
    },

    #[serde(rename = "image_url")]
    ImageUrl { image_url: MediaUrl },

    #[serde(rename = "audio_url")]
    AudioUrl { audio_url: MediaUrl },

    #[serde(rename = "video_url")]
    VideoUrl { video_url: MediaUrl },
}

/// URL reference for media content parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaUrl {
    pub url: String,
    #[serde(default)]
    pub id: Option<String>,
}

// ---------------------------------------------------------------------------
// Display block types (shared by ToolResult and ApprovalRequest)
// ---------------------------------------------------------------------------

/// Rich display blocks for UI rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DisplayBlock {
    #[serde(rename = "brief")]
    Brief { text: String },

    #[serde(rename = "diff")]
    Diff {
        path: String,
        old_text: String,
        new_text: String,
    },

    #[serde(rename = "todo")]
    Todo { items: Vec<TodoItem> },

    #[serde(rename = "shell")]
    Shell {
        language: String,
        command: String,
    },

    /// Fallback for unrecognized block types.
    #[serde(other)]
    Unknown,
}

/// A single item in a `Todo` display block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub title: String,
    pub status: TodoStatus,
}

/// Status values for todo items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
}

// ---------------------------------------------------------------------------
// Event-specific input payloads
// ---------------------------------------------------------------------------

/// TurnBegin payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnBeginPayload {
    /// The user input that started this turn. Can be a string or an
    /// array of ContentPart objects (text, image_url, audio_url, video_url).
    pub user_input: Value,
}

/// StepBegin payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepBeginPayload {
    /// Step index, starting at 1.
    pub n: u32,
}

/// StatusUpdate payload. All fields are optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusUpdatePayload {
    /// 0-1 float representing context window utilization.
    #[serde(default)]
    pub context_usage: Option<f64>,

    /// Token consumption breakdown.
    #[serde(default)]
    pub token_usage: Option<TokenUsage>,

    /// Provider message identifier.
    #[serde(default)]
    pub message_id: Option<String>,
}

/// Token consumption breakdown within a StatusUpdate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Non-cached input tokens.
    #[serde(default)]
    pub input_other: u64,

    /// Output tokens.
    #[serde(default)]
    pub output: u64,

    /// Tokens read from cache.
    #[serde(default)]
    pub input_cache_read: u64,

    /// Tokens written to cache.
    #[serde(default)]
    pub input_cache_creation: u64,
}

/// ContentPart payload (wraps the variant).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPartPayload(pub ContentPartVariant);

/// ToolCall payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPayload {
    /// Always `"function"`.
    #[serde(rename = "type")]
    pub call_type: String,

    /// Unique tool call identifier.
    pub id: String,

    /// Function name and arguments.
    pub function: ToolCallFunction,

    /// Provider-specific metadata.
    #[serde(default)]
    pub extras: Option<Value>,
}

/// The `function` object within a ToolCall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    /// Tool name (e.g., "Shell", "FileWrite", "Grep").
    pub name: String,

    /// JSON-encoded arguments. May be absent or incomplete during streaming.
    #[serde(default)]
    pub arguments: Option<String>,
}

/// ToolCallPart payload (streaming argument fragment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPartPayload {
    /// Fragment of JSON-encoded arguments. Not valid JSON on its own.
    #[serde(default)]
    pub arguments_part: Option<String>,
}

/// ToolResult payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPayload {
    /// Matches the `id` from the originating ToolCall.
    pub tool_call_id: String,

    /// The tool execution result.
    pub return_value: ToolReturnValue,
}

/// The `return_value` object within a ToolResult.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolReturnValue {
    /// Whether the tool execution failed.
    pub is_error: bool,

    /// Tool output content (string or ContentPart array).
    #[serde(default)]
    pub output: Value,

    /// Human-readable summary.
    #[serde(default)]
    pub message: Option<String>,

    /// Rich display blocks for UI rendering.
    #[serde(default)]
    pub display: Vec<DisplayBlock>,

    /// Provider-specific metadata.
    #[serde(default)]
    pub extras: Option<Value>,
}

/// ApprovalResponse notification payload (resolution of a prior request).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponsePayload {
    /// Matches the `id` from the originating ApprovalRequest.
    pub request_id: String,

    /// How the approval was resolved.
    pub response: ApprovalDecision,
}

/// SubagentEvent payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentEventPayload {
    /// The tool call ID of the Task that spawned this subagent.
    pub task_tool_call_id: String,

    /// A nested Wire event (recursive -- can itself be SubagentEvent).
    pub event: Box<KimiCodeWireMessage>,
}

/// ApprovalRequest payload (blocking).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequestPayload {
    /// Request identifier (use as `request_id` in response).
    pub id: String,

    /// The tool call being approved.
    pub tool_call_id: String,

    /// Tool name that triggered the approval.
    pub sender: String,

    /// Action description.
    pub action: String,

    /// Human-readable explanation of what will happen.
    pub description: String,

    /// Optional rich preview (diffs, commands, etc.).
    #[serde(default)]
    pub display: Vec<DisplayBlock>,
}

/// ToolCallRequest payload (blocking -- external tool invocation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequestPayload {
    /// Tool call identifier.
    pub id: String,

    /// Name of the registered external tool.
    pub name: String,

    /// JSON-encoded arguments.
    #[serde(default)]
    pub arguments: Option<String>,
}

// ---------------------------------------------------------------------------
// Wire message wrapper
// ---------------------------------------------------------------------------

/// A complete Wire protocol message from the Kimi agent.
///
/// Used for parsing incoming JSON-RPC messages and for the recursive
/// `SubagentEvent.event` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiCodeWireMessage {
    /// The event or request type.
    #[serde(rename = "type")]
    pub event_type: String,

    /// The event/request payload.
    pub payload: Value,
}

impl KimiCodeWireMessage {
    /// Parse the event type string into a strongly-typed event.
    pub fn event(&self) -> Result<KimiCodeEvent, UnknownEventError> {
        KimiCodeEvent::try_from(self.event_type.as_str())
    }
}

// ---------------------------------------------------------------------------
// Unified input payload enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any Kimi Code event's input payload.
///
/// Constructed by deserializing the Wire message payload based on the
/// `type` field. Gives callers access to strongly-typed event-specific
/// fields without manual JSON wrangling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum KimiCodeInput {
    TurnBegin(TurnBeginPayload),
    TurnEnd,
    StepBegin(StepBeginPayload),
    StepInterrupted,
    CompactionBegin,
    CompactionEnd,
    StatusUpdate(StatusUpdatePayload),
    ContentPart(ContentPartPayload),
    ToolCall(ToolCallPayload),
    ToolCallPart(ToolCallPartPayload),
    ToolResult(ToolResultPayload),
    ApprovalResponse(ApprovalResponsePayload),
    SubagentEvent(SubagentEventPayload),
    ApprovalRequest(ApprovalRequestPayload),
    ToolCallRequest(ToolCallRequestPayload),
}

impl KimiCodeInput {
    /// Returns the event variant for this input payload.
    pub fn event(&self) -> KimiCodeEvent {
        match self {
            Self::TurnBegin(_) => KimiCodeEvent::TurnBegin,
            Self::TurnEnd => KimiCodeEvent::TurnEnd,
            Self::StepBegin(_) => KimiCodeEvent::StepBegin,
            Self::StepInterrupted => KimiCodeEvent::StepInterrupted,
            Self::CompactionBegin => KimiCodeEvent::CompactionBegin,
            Self::CompactionEnd => KimiCodeEvent::CompactionEnd,
            Self::StatusUpdate(_) => KimiCodeEvent::StatusUpdate,
            Self::ContentPart(_) => KimiCodeEvent::ContentPart,
            Self::ToolCall(_) => KimiCodeEvent::ToolCall,
            Self::ToolCallPart(_) => KimiCodeEvent::ToolCallPart,
            Self::ToolResult(_) => KimiCodeEvent::ToolResult,
            Self::ApprovalResponse(_) => KimiCodeEvent::ApprovalResponse,
            Self::SubagentEvent(_) => KimiCodeEvent::SubagentEvent,
            Self::ApprovalRequest(_) => KimiCodeEvent::ApprovalRequest,
            Self::ToolCallRequest(_) => KimiCodeEvent::ToolCallRequest,
        }
    }

    /// Returns the tool name if this is a tool-related event.
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::ToolCall(p) => Some(&p.function.name),
            Self::ToolResult(p) => {
                // ToolResult only has tool_call_id, not tool name directly.
                // Return None; callers should correlate via tool_call_id.
                let _ = p;
                None
            }
            Self::ApprovalRequest(p) => Some(&p.sender),
            Self::ToolCallRequest(p) => Some(&p.name),
            _ => None,
        }
    }

    /// Returns the tool call ID if this event carries one.
    pub fn tool_call_id(&self) -> Option<&str> {
        match self {
            Self::ToolCall(p) => Some(&p.id),
            Self::ToolResult(p) => Some(&p.tool_call_id),
            Self::ApprovalRequest(p) => Some(&p.tool_call_id),
            Self::ApprovalResponse(p) => Some(&p.request_id),
            Self::ToolCallRequest(p) => Some(&p.id),
            _ => None,
        }
    }

    /// Returns true if this event reports a tool error.
    pub fn is_tool_error(&self) -> bool {
        matches!(self, Self::ToolResult(p) if p.return_value.is_error)
    }

    /// Returns the subagent task tool call ID if this is a SubagentEvent.
    pub fn subagent_task_id(&self) -> Option<&str> {
        match self {
            Self::SubagentEvent(p) => Some(&p.task_tool_call_id),
            _ => None,
        }
    }

    /// Whether this input is from a blocking request (requires a response).
    pub fn requires_response(&self) -> bool {
        self.event().is_blocking()
    }
}

// ---------------------------------------------------------------------------
// Response types (what clients return for blocking requests)
// ---------------------------------------------------------------------------

/// Response to an `ApprovalRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponseOutput {
    /// Must match the `id` from the ApprovalRequest.
    pub request_id: String,

    /// The approval decision.
    pub response: ApprovalDecision,
}

/// Response to a `ToolCallRequest` (external tool result).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponseOutput {
    /// Must match the `id` from the ToolCallRequest.
    pub tool_call_id: String,

    /// The tool execution result.
    pub return_value: ToolReturnValue,
}

// ---------------------------------------------------------------------------
// Client-to-agent methods
// ---------------------------------------------------------------------------

/// Client methods that can be sent to the Kimi agent.
///
/// These are JSON-RPC methods the client sends to control the agent,
/// distinct from the agent's event/request messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KimiClientMethod {
    /// Optional handshake for protocol negotiation and external tool
    /// registration.
    Initialize,
    /// Starts an agent turn with user input.
    Prompt,
    /// Replays a recorded session from wire.jsonl.
    Replay,
    /// Cancels the in-progress prompt or replay.
    Cancel,
}

impl KimiClientMethod {
    pub fn method_name(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Prompt => "prompt",
            Self::Replay => "replay",
            Self::Cancel => "cancel",
        }
    }
}

impl fmt::Display for KimiClientMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.method_name())
    }
}

// ---------------------------------------------------------------------------
// Initialize handshake types
// ---------------------------------------------------------------------------

/// Parameters for the `initialize` client method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,

    #[serde(default)]
    pub client: Option<ClientInfo>,

    #[serde(default)]
    pub external_tools: Vec<ExternalToolDef>,
}

/// Client identification sent during initialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// External tool definition registered during initialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Option<Value>,
}

/// Result of the `initialize` handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub server: ServerInfo,

    #[serde(default)]
    pub slash_commands: Vec<SlashCommand>,

    #[serde(default)]
    pub external_tools: Option<ExternalToolsResult>,
}

/// Server identification returned during initialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Slash command reported by the server during initialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Result of external tool registration during initialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolsResult {
    pub accepted: Vec<String>,
    pub rejected: Vec<String>,
}

// ---------------------------------------------------------------------------
// JSON-RPC error codes specific to Kimi Wire protocol
// ---------------------------------------------------------------------------

/// Kimi-specific JSON-RPC error codes beyond the standard set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KimiErrorCode {
    /// Standard: Invalid JSON (parse error).
    ParseError,
    /// Standard: Invalid request structure.
    InvalidRequest,
    /// Standard: Method not found.
    MethodNotFound,
    /// Standard: Invalid parameters.
    InvalidParams,
    /// Standard: Internal error.
    InternalError,
    /// Kimi: A turn is already in progress.
    TurnAlreadyInProgress,
    /// Kimi: LLM provider not configured.
    LlmNotConfigured,
    /// Kimi: Specified LLM model is unsupported.
    LlmUnsupported,
    /// Kimi: LLM service returned an error.
    LlmServiceError,
}

impl KimiErrorCode {
    pub fn code(&self) -> i32 {
        match self {
            Self::ParseError => -32700,
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
            Self::InternalError => -32603,
            Self::TurnAlreadyInProgress => -32000,
            Self::LlmNotConfigured => -32001,
            Self::LlmUnsupported => -32002,
            Self::LlmServiceError => -32003,
        }
    }

    /// Parse a numeric error code into the enum variant.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            -32700 => Some(Self::ParseError),
            -32600 => Some(Self::InvalidRequest),
            -32601 => Some(Self::MethodNotFound),
            -32602 => Some(Self::InvalidParams),
            -32603 => Some(Self::InternalError),
            -32000 => Some(Self::TurnAlreadyInProgress),
            -32001 => Some(Self::LlmNotConfigured),
            -32002 => Some(Self::LlmUnsupported),
            -32003 => Some(Self::LlmServiceError),
            _ => None,
        }
    }

    /// Whether this is a Kimi-specific error (vs standard JSON-RPC).
    pub fn is_kimi_specific(&self) -> bool {
        matches!(
            self,
            Self::TurnAlreadyInProgress
                | Self::LlmNotConfigured
                | Self::LlmUnsupported
                | Self::LlmServiceError
        )
    }
}

// ---------------------------------------------------------------------------
// Known built-in tool names
// ---------------------------------------------------------------------------

/// Known built-in Kimi Code tool names.
///
/// Kimi uses different tool names than Claude Code. These are the tools
/// that appear in `ToolCall.function.name` and `ApprovalRequest.sender`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KimiBuiltinTool {
    Shell,
    FileWrite,
    FileRead,
    Grep,
    Glob,
    WebFetch,
    Task,
}

impl KimiBuiltinTool {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Shell" => Some(Self::Shell),
            "FileWrite" => Some(Self::FileWrite),
            "FileRead" => Some(Self::FileRead),
            "Grep" => Some(Self::Grep),
            "Glob" => Some(Self::Glob),
            "WebFetch" => Some(Self::WebFetch),
            "Task" => Some(Self::Task),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Shell => "Shell",
            Self::FileWrite => "FileWrite",
            Self::FileRead => "FileRead",
            Self::Grep => "Grep",
            Self::Glob => "Glob",
            Self::WebFetch => "WebFetch",
            Self::Task => "Task",
        }
    }
}
```

## Design Considerations

- **Wire protocol, not file-based hooks.** Kimi Code's event system is fundamentally different from Claude Code's. Claude Code uses file-based hook configuration (`settings.json`) with shell commands, prompts, and agent handlers. Kimi uses a JSON-RPC 2.0 bidirectional protocol over stdin/stdout. This means there is no matcher system, no hook handler types, no exit code semantics, and no `CommonInputFields` equivalent -- all concepts that are central to Claude Code's design. The Kimi design models the Wire protocol natively rather than forcing Claude Code's patterns onto it.

- **Only two blocking events.** Where Claude Code has 7 events that `can_block()`, Kimi Code has exactly 2 blocking requests (`ApprovalRequest` and `ToolCallRequest`). Everything else is a fire-and-forget notification. This is captured via `is_blocking()` / `is_notification()` rather than Claude's richer `DecisionPattern` and `ExitCode` systems, which would be meaningless here.

- **No common input fields.** Claude Code events all share `session_id`, `transcript_path`, `cwd`, `permission_mode`, and `hook_event_name` via `CommonInputFields`. Kimi Code events have no common fields -- each event type has its own bespoke payload (or no payload at all for events like `TurnEnd`, `StepInterrupted`, `CompactionBegin`, and `CompactionEnd`). The `KimiCodeInput` enum uses unit variants for empty-payload events rather than forcing a common struct.

- **Streaming events are first-class.** Kimi Code has two streaming event types (`ContentPart` and `ToolCallPart`) that deliver incremental fragments requiring client-side buffering. The `is_streaming()` method makes this discoverable. Claude Code does not have streaming events in its hook system.

- **ContentPart is a rich discriminated union.** The `ContentPart` event carries text, thinking, images, audio, or video. This is modeled as `ContentPartVariant` with `#[serde(tag = "type")]` for clean deserialization. Claude Code has no equivalent -- its hooks do not expose model response streaming.

- **Recursive SubagentEvent.** Kimi's `SubagentEvent` contains a nested `KimiCodeWireMessage` that can itself be any event type, including another `SubagentEvent`. This is handled via `Box<KimiCodeWireMessage>` to break the recursive type size. Claude Code's subagent events (`SubagentStart`/`SubagentStop`) are flat and non-recursive.

- **Client-to-agent methods modeled separately.** Kimi Code has 4 client-initiated methods (`initialize`, `prompt`, `replay`, `cancel`) that are distinct from agent events. These are modeled as `KimiClientMethod` rather than being mixed into the event enum, since they flow in the opposite direction and have different semantics.

- **Initialize handshake enables external tools.** Kimi's `initialize` method registers external tools that the agent can later invoke via `ToolCallRequest`. Claude Code has no equivalent -- its tools are built-in or come from MCP servers. The `InitializeParams`, `ExternalToolDef`, and `InitializeResult` types capture this handshake.

- **No session lifecycle events.** Kimi Code does not emit `SessionStart` or `SessionEnd` over Wire. Session boundaries must be inferred from process launch/exit. This is a significant gap compared to Claude Code and is noted in the `TryFrom<AgenticEvent>` implementation which returns `Err` for `SessionStart` and `SessionEnd`.

- **No matcher system.** Claude Code has a rich matcher system where hooks can be filtered by tool name, session source, notification type, etc. Kimi delivers all events unconditionally; filtering is the client's responsibility. The design omits `MatcherField` and related types entirely.

- **ApprovalResponse legacy name support.** The `TryFrom<&str>` implementation accepts both `"ApprovalResponse"` (current) and `"ApprovalRequestResolved"` (pre-1.1 name) for backward compatibility with older Wire implementations.

- **JSON-RPC error codes as a first-class type.** Kimi extends standard JSON-RPC error codes with 4 custom codes (-32000 through -32003). These are modeled as `KimiErrorCode` with `from_code()` parsing and `is_kimi_specific()` classification, since error handling is integral to the Wire protocol.

- **Display blocks shared across events.** Both `ToolResult` and `ApprovalRequest` carry `DisplayBlock` arrays for rich UI rendering (diffs, shell commands, todos). These are modeled as a shared `DisplayBlock` enum with `#[serde(tag = "type")]` and an `Unknown` fallback variant for forward compatibility.

- **Defensive Option fields throughout.** Following the Kimi documentation's explicit warning that "optional/null fields are pervasive," payload structs use `Option<T>` and `#[serde(default)]` liberally. `StatusUpdatePayload` defaults all fields to `None`. `ToolCallFunction.arguments` is `Option<String>` since it may be absent during streaming.

## Claude Code Mapping

- **TurnBegin** -- Same trigger as Claude Code's `UserPromptSubmit` (a user prompt starts a turn), but different payload: Kimi provides `user_input` (which can be multimodal), Claude provides `prompt` (always a string) plus `session_id`, `cwd`, etc. Also, TurnBegin is a notification (no response), while UserPromptSubmit is blocking (can reject the prompt). Maps to `AgenticEvent::BeforePrompt`.

- **TurnEnd** -- Closest to Claude Code's `Stop`, which fires when the main agent finishes responding. Key difference: Kimi's TurnEnd is a notification (cannot block), while Claude's Stop is blocking (can force the agent to continue). Also, TurnEnd may be omitted on cancellation. Maps to `AgenticEvent::TurnComplete`.

- **StepBegin** -- Distinct from anything in Claude Code. Claude Code has no concept of numbered steps within a turn. The closest analog is `BeforeModel` in the unified enum since it represents the start of a model inference cycle. Maps to `AgenticEvent::BeforeModel`.

- **StepInterrupted** -- Distinct from anything in Claude Code. Claude Code has no equivalent for mid-step interruption. Maps to `AgenticEvent::TurnError` as the closest semantic match.

- **CompactionBegin** -- Same trigger as Claude Code's `PreCompact` (context compaction starting), but different payload: Kimi's payload is empty, while Claude's PreCompact includes `trigger` (manual/auto), `custom_instructions`, and full `CommonInputFields`. Also, CompactionBegin is purely informational, while PreCompact supports matchers on the trigger field. Maps to `AgenticEvent::BeforeCompact`.

- **CompactionEnd** -- Distinct from anything in Claude Code. Claude Code has no post-compaction notification (it re-fires `SessionStart` with `source: "compact"` instead). Maps to `AgenticEvent::AfterModel` as a loose proxy.

- **StatusUpdate** -- Distinct from anything in Claude Code. Claude Code has no telemetry event for token usage or context window utilization. The closest Claude Code concept is `Notification`, but StatusUpdate carries structured numeric data, not a text message. Maps to `AgenticEvent::Notification`.

- **ContentPart** -- Distinct from anything in Claude Code. Claude Code hooks do not expose model response streaming. The agent's text output is not surfaced through the hook system at all. Maps to `AgenticEvent::AfterModel`.

- **ToolCall** -- Same trigger as Claude Code's `PreToolUse` (a tool call is being formed), but fundamentally different interaction pattern: Kimi's ToolCall is a notification (cannot block, cannot modify arguments), while Claude's PreToolUse is the most powerful blocking event (can allow/deny/ask, modify tool input, inject context). The payload structure also differs: Kimi uses `function.name` + JSON-encoded `function.arguments`, while Claude uses `tool_name` + structured `tool_input`. Maps to `AgenticEvent::BeforeTool`.

- **ToolCallPart** -- Distinct from anything in Claude Code. Claude Code does not stream tool arguments incrementally; by the time `PreToolUse` fires, arguments are complete. Maps to `AgenticEvent::BeforeTool` since it is still part of the pre-execution phase.

- **ToolResult** -- Same trigger as Claude Code's `PostToolUse` (tool execution completed) and `PostToolUseFailure` (tool execution failed). Kimi combines both success and failure into a single event type, distinguished by `return_value.is_error`. Claude Code separates them into distinct events with different payloads and blocking semantics. Kimi's ToolResult is a notification; Claude's PostToolUse can block (advisory). Maps to `AgenticEvent::AfterTool` (success) or `AgenticEvent::ToolError` (when `is_error` is true).

- **ApprovalResponse** -- Partially analogous to Claude Code's `PermissionRequest`, but inverted. Claude's `PermissionRequest` fires before a permission dialog appears (proactive), while Kimi's `ApprovalResponse` fires after an approval was resolved (reactive notification). The resolution values also differ: Kimi uses `approve`/`approve_for_session`/`reject`, while Claude uses `allow`/`deny` with nested behavior objects. Maps to `AgenticEvent::PermissionRequest`.

- **SubagentEvent** -- Same trigger domain as Claude Code's `SubagentStart`/`SubagentStop` (subagent lifecycle), but structurally different. Kimi wraps any nested event inside a `SubagentEvent` envelope with a `task_tool_call_id` link, while Claude Code fires discrete `SubagentStart` and `SubagentStop` events with `agent_id` and `agent_type`. Kimi's approach is recursive and fine-grained (every subagent event is forwarded); Claude's is coarse (only start/stop). Maps to `AgenticEvent::SubagentStart`.

- **ApprovalRequest** -- Same trigger as Claude Code's `PermissionRequest` (agent needs permission for an action), but different mechanism. Kimi's ApprovalRequest is a JSON-RPC blocking request with `approve`/`approve_for_session`/`reject`, while Claude's PermissionRequest is a hook event with `allow`/`deny` behavior decisions and optional `updatedInput`/`updatedPermissions`. Kimi's response cannot modify tool input; Claude's can. Maps to `AgenticEvent::PermissionRequest`.

- **ToolCallRequest** -- Distinct from anything in Claude Code. This is Kimi's mechanism for external tool invocation: the agent calls a tool registered during `initialize`, and the client must execute it and return results. Claude Code has no equivalent; its tools are either built-in or accessed via MCP servers, never delegated to the client process. Maps to `AgenticEvent::BeforeTool` since it represents a tool being called.

- **No equivalent for Claude Code's `SessionStart`** -- Kimi does not emit session lifecycle events over Wire. Session start must be inferred from process launch.

- **No equivalent for Claude Code's `SessionEnd`** -- Kimi does not emit session end events. Session end must be inferred from process exit or connection close.

- **No equivalent for Claude Code's `SubagentStop`** -- Kimi's `SubagentEvent` wraps all nested events but does not have a discrete "subagent finished" signal.

- **No equivalent for Claude Code's `TeammateIdle`** -- Kimi has no team/teammate concept in its Wire protocol.

- **No equivalent for Claude Code's `TaskCompleted`** -- Kimi has no task completion event in its Wire protocol.

- **No equivalent for Claude Code's `PreCompact` matcher/trigger fields** -- Kimi's `CompactionBegin` carries no metadata about what triggered compaction.
