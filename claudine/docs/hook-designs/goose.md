# Goose Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All events observable in Goose (Block).
///
/// Goose has a fundamentally different hook architecture from Claude Code:
/// it provides one fire-and-forget status hook (`GOOSE_STATUS_HOOK`) plus
/// a set of streaming JSON events emitted via `--output-format stream-json`.
/// None of these events support a return channel -- they are all outbound-only.
///
/// The variant names use Goose's native snake_case naming (matching the
/// `type` field in stream-json events) so that debug output and serde
/// deserialization align with the wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GooseEvent {
    /// Status hook fires with "waiting" -- agent is idle, awaiting user input.
    StatusWaiting,
    /// Status hook fires with "thinking" -- agent is processing.
    StatusThinking,
    /// Stream event: assistant message or tool request/response produced.
    Message,
    /// Stream event: MCP extension or subagent log/progress notification.
    Notification,
    /// Stream event: active model or operating mode changed.
    ModelChange,
    /// Stream event: error occurred in the agent loop.
    Error,
    /// Stream event: run completed (always the final event in a stream).
    Complete,
}

impl GooseEvent {
    /// All variants in lifecycle order.
    pub const ALL: [GooseEvent; 7] = [
        Self::StatusWaiting,
        Self::StatusThinking,
        Self::Message,
        Self::Notification,
        Self::ModelChange,
        Self::Error,
        Self::Complete,
    ];

    /// The native event identifier as it appears on the wire.
    ///
    /// For status hook events, this is the argument passed to the hook
    /// command. For stream events, this is the `type` field in the JSON.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::StatusWaiting => "waiting",
            Self::StatusThinking => "thinking",
            Self::Message => "message",
            Self::Notification => "notification",
            Self::ModelChange => "model_change",
            Self::Error => "error",
            Self::Complete => "complete",
        }
    }

    /// Goose-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::StatusWaiting => {
                "Status hook fires when the agent becomes idle and awaits user input"
            }
            Self::StatusThinking => {
                "Status hook fires when the agent begins processing after user input"
            }
            Self::Message => {
                "Stream event emitted when the agent produces an assistant message, \
                 tool request, or tool response"
            }
            Self::Notification => {
                "Stream event emitted when an MCP extension or subagent sends a \
                 log or progress notification"
            }
            Self::ModelChange => {
                "Stream event emitted when the active model or operating mode changes \
                 (e.g., lead vs worker model)"
            }
            Self::Error => {
                "Stream event emitted when an error occurs in the agent loop; \
                 always followed by a Complete event"
            }
            Self::Complete => {
                "Stream event marking the end of a goose run; always the final event \
                 in the stream"
            }
        }
    }

    /// The delivery mechanism for this event.
    pub fn delivery(&self) -> DeliveryMechanism {
        match self {
            Self::StatusWaiting | Self::StatusThinking => DeliveryMechanism::StatusHook,
            Self::Message
            | Self::Notification
            | Self::ModelChange
            | Self::Error
            | Self::Complete => DeliveryMechanism::StreamJson,
        }
    }

    /// Whether this event can block, modify, or influence the agent loop.
    ///
    /// In Goose, the answer is always `false`. All events are outbound-only.
    pub fn can_block(&self) -> bool {
        false
    }

    /// Whether there is a return channel for this event.
    ///
    /// In Goose, the answer is always `false`. Status hook output is
    /// suppressed; stream events have no inbound channel.
    pub fn has_return_channel(&self) -> bool {
        false
    }

    /// The typed payload structure this event provides.
    pub fn payload_type(&self) -> PayloadType {
        match self {
            Self::StatusWaiting | Self::StatusThinking => PayloadType::Status,
            Self::Message => PayloadType::Message,
            Self::Notification => PayloadType::Notification,
            Self::ModelChange => PayloadType::ModelChange,
            Self::Error => PayloadType::Error,
            Self::Complete => PayloadType::Complete,
        }
    }

    /// The response type this event expects.
    ///
    /// All Goose events return `ResponseType::None` because Goose has
    /// no inbound return channel.
    pub fn response_type(&self) -> ResponseType {
        ResponseType::None
    }

    /// Whether this event is emitted via the status hook mechanism.
    pub fn is_status_hook(&self) -> bool {
        matches!(self, Self::StatusWaiting | Self::StatusThinking)
    }

    /// Whether this event is emitted via the stream-json mechanism.
    pub fn is_stream_event(&self) -> bool {
        !self.is_status_hook()
    }
}

impl fmt::Display for GooseEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native event names
// ---------------------------------------------------------------------------

impl TryFrom<&str> for GooseEvent {
    type Error = UnknownEventError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "waiting" => Ok(Self::StatusWaiting),
            "thinking" => Ok(Self::StatusThinking),
            "message" => Ok(Self::Message),
            "notification" => Ok(Self::Notification),
            "model_change" => Ok(Self::ModelChange),
            "error" => Ok(Self::Error),
            "complete" => Ok(Self::Complete),
            _ => Err(UnknownEventError(name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known Goose event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown Goose event: {0}")]
pub struct UnknownEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

impl From<GooseEvent> for AgenticEvent {
    fn from(event: GooseEvent) -> Self {
        match event {
            // StatusWaiting maps loosely to TurnComplete -- the agent has
            // finished its turn and is now idle.
            GooseEvent::StatusWaiting => AgenticEvent::TurnComplete,

            // StatusThinking maps to BeforeModel -- the agent has accepted
            // input and is about to begin processing (model inference).
            GooseEvent::StatusThinking => AgenticEvent::BeforeModel,

            // Message is a composite event. It can contain assistant text,
            // tool requests, and tool responses in a single payload. The
            // closest unified mapping is AfterModel since the message
            // represents model output.
            GooseEvent::Message => AgenticEvent::AfterModel,

            // Notification maps 1:1.
            GooseEvent::Notification => AgenticEvent::Notification,

            // ModelChange has no direct unified equivalent; Notification
            // is the best fit since it is informational.
            GooseEvent::ModelChange => AgenticEvent::Notification,

            // Error maps to TurnError.
            GooseEvent::Error => AgenticEvent::TurnError,

            // Complete maps to SessionEnd since it marks the end of the
            // entire run.
            GooseEvent::Complete => AgenticEvent::SessionEnd,
        }
    }
}

impl TryFrom<AgenticEvent> for GooseEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified to Goose event.
    ///
    /// Many unified events have no Goose equivalent because Goose does
    /// not expose pre/post tool hooks, permission requests, session start,
    /// subagent events, or compaction hooks.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::TurnComplete => Ok(GooseEvent::StatusWaiting),
            AgenticEvent::BeforeModel => Ok(GooseEvent::StatusThinking),
            AgenticEvent::AfterModel => Ok(GooseEvent::Message),
            AgenticEvent::Notification => Ok(GooseEvent::Notification),
            AgenticEvent::TurnError => Ok(GooseEvent::Error),
            AgenticEvent::SessionEnd => Ok(GooseEvent::Complete),

            // Events Goose does not support
            AgenticEvent::SessionStart
            | AgenticEvent::BeforePrompt
            | AgenticEvent::BeforeTool
            | AgenticEvent::AfterTool
            | AgenticEvent::ToolError
            | AgenticEvent::PermissionRequest
            | AgenticEvent::HumanInTheLoop
            | AgenticEvent::SubagentStart
            | AgenticEvent::SubagentStop
            | AgenticEvent::BeforeCompact => Err("Goose does not support this event"),
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// How the event is delivered to consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMechanism {
    /// Delivered via the `GOOSE_STATUS_HOOK` shell command.
    /// Fire-and-forget: stdout/stderr suppressed, exit code ignored.
    StatusHook,
    /// Delivered as a line in the stream-json output
    /// (`goose run --output-format stream-json`).
    /// Outbound-only: no return channel.
    StreamJson,
}

/// Discriminant for the event-specific input payload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    /// Status hook events: a single status string argument.
    Status,
    /// Message events: full Goose Message struct (role + content array).
    Message,
    /// Notification events: log or progress variant from an MCP extension.
    Notification,
    /// Model change events: model identifier and mode.
    ModelChange,
    /// Error events: error description string.
    Error,
    /// Complete events: total token count.
    Complete,
}

/// Discriminant for the event-specific response type.
///
/// Goose has only one response type: None. All events are outbound-only
/// with no mechanism to influence agent behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    /// No response supported. Goose events are fire-and-forget or
    /// stream-only with no return channel.
    None,
}

// ---------------------------------------------------------------------------
// Status hook payload
// ---------------------------------------------------------------------------

/// The status argument passed to `GOOSE_STATUS_HOOK`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GooseStatus {
    /// Agent is idle, awaiting user input.
    Waiting,
    /// Agent is processing (thinking).
    Thinking,
}

impl GooseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Thinking => "thinking",
        }
    }

    /// Convert to the corresponding `GooseEvent`.
    pub fn to_event(&self) -> GooseEvent {
        match self {
            Self::Waiting => GooseEvent::StatusWaiting,
            Self::Thinking => GooseEvent::StatusThinking,
        }
    }
}

impl From<GooseStatus> for GooseEvent {
    fn from(status: GooseStatus) -> Self {
        status.to_event()
    }
}

impl TryFrom<&str> for GooseStatus {
    type Error = UnknownEventError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "waiting" => Ok(Self::Waiting),
            "thinking" => Ok(Self::Thinking),
            _ => Err(UnknownEventError(s.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// Stream-JSON event payloads
// ---------------------------------------------------------------------------

/// A single content item within a Goose message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    /// Plain text from the assistant.
    Text {
        text: String,
    },
    /// Tool invocation request from the assistant.
    ToolRequest {
        id: String,
        tool_name: String,
        arguments: Value,
    },
    /// Result of a tool invocation.
    ToolResponse {
        id: String,
        output: Value,
    },
    /// The agent needs user input to continue.
    ActionRequired {
        prompt: String,
    },
    /// Model thinking/reasoning block (chain-of-thought).
    Thinking {
        text: String,
    },
}

/// A Goose message (assistant or tool response turn).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooseMessage {
    /// The role that produced this message (typically "assistant").
    pub role: String,

    /// Content items: text, tool requests, tool responses, etc.
    pub content: Vec<MessageContent>,
}

/// Payload for the `message` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub message: GooseMessage,
}

/// Log notification data from an MCP extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogNotification {
    pub message: String,
}

/// Progress notification data from an MCP extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    pub progress: f64,
    pub total: f64,
    #[serde(default)]
    pub message: Option<String>,
}

/// Payload for the `notification` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Which MCP extension or subagent produced this notification.
    pub extension_id: String,

    /// Log variant (mutually exclusive with `progress`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<LogNotification>,

    /// Progress variant (mutually exclusive with `log`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressNotification>,
}

/// Payload for the `model_change` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChangePayload {
    /// Model identifier (e.g., "claude-4.5-sonnet").
    pub model: String,

    /// Operating mode (e.g., "lead", "worker").
    pub mode: String,
}

/// Payload for the `error` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    /// Human-readable error description.
    pub error: String,
}

/// Payload for the `complete` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletePayload {
    /// Total tokens consumed during the run. `None` if unavailable.
    pub total_tokens: Option<u64>,
}

// ---------------------------------------------------------------------------
// Unified stream event enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any Goose stream-json event.
///
/// Constructed by deserializing a line of stream-json output. The
/// `type` field in the JSON serves as the discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GooseStreamEvent {
    Message(MessagePayload),
    Notification(NotificationPayload),
    ModelChange(ModelChangePayload),
    Error(ErrorPayload),
    Complete(CompletePayload),
}

impl GooseStreamEvent {
    /// Returns the `GooseEvent` variant for this stream event.
    pub fn event(&self) -> GooseEvent {
        match self {
            Self::Message(_) => GooseEvent::Message,
            Self::Notification(_) => GooseEvent::Notification,
            Self::ModelChange(_) => GooseEvent::ModelChange,
            Self::Error(_) => GooseEvent::Error,
            Self::Complete(_) => GooseEvent::Complete,
        }
    }

    /// Extract tool requests from a message event, if any.
    ///
    /// Returns an empty slice for non-message events. For message
    /// events, returns only the `ToolRequest` content items.
    pub fn tool_requests(&self) -> Vec<&MessageContent> {
        match self {
            Self::Message(payload) => payload
                .message
                .content
                .iter()
                .filter(|c| matches!(c, MessageContent::ToolRequest { .. }))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Extract tool responses from a message event, if any.
    pub fn tool_responses(&self) -> Vec<&MessageContent> {
        match self {
            Self::Message(payload) => payload
                .message
                .content
                .iter()
                .filter(|c| matches!(c, MessageContent::ToolResponse { .. }))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Extract text content from a message event, if any.
    pub fn text_content(&self) -> Vec<&str> {
        match self {
            Self::Message(payload) => payload
                .message
                .content
                .iter()
                .filter_map(|c| match c {
                    MessageContent::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Returns the extension ID for notification events.
    pub fn extension_id(&self) -> Option<&str> {
        match self {
            Self::Notification(payload) => Some(&payload.extension_id),
            _ => None,
        }
    }

    /// Returns the total token count for complete events.
    pub fn total_tokens(&self) -> Option<u64> {
        match self {
            Self::Complete(payload) => payload.total_tokens,
            _ => None,
        }
    }
}

/// Union type for any Goose event input, covering both delivery mechanisms.
///
/// Status hook events carry only a status string. Stream events carry
/// their full JSON payload.
#[derive(Debug, Clone)]
pub enum GooseInput {
    /// A status hook invocation with the status argument.
    Status(GooseStatus),
    /// A stream-json event with its typed payload.
    Stream(GooseStreamEvent),
}

impl GooseInput {
    /// Returns the `GooseEvent` variant for this input.
    pub fn event(&self) -> GooseEvent {
        match self {
            Self::Status(status) => status.to_event(),
            Self::Stream(stream) => stream.event(),
        }
    }
}

impl From<GooseStatus> for GooseInput {
    fn from(status: GooseStatus) -> Self {
        Self::Status(status)
    }
}

impl From<GooseStreamEvent> for GooseInput {
    fn from(stream: GooseStreamEvent) -> Self {
        Self::Stream(stream)
    }
}

// ---------------------------------------------------------------------------
// Batch JSON output (non-streaming)
// ---------------------------------------------------------------------------

/// The complete output from `goose run --output-format json`.
///
/// This is a post-run summary, not a streaming event. Included for
/// completeness since it is the other structured output mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooseBatchOutput {
    /// Full conversation history.
    pub messages: Vec<GooseMessage>,

    /// Run metadata.
    pub metadata: BatchMetadata,
}

/// Metadata from a batch JSON output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    /// Total tokens consumed. `None` if unavailable.
    pub total_tokens: Option<u64>,

    /// Run status (e.g., "completed", "error").
    pub status: String,
}

// ---------------------------------------------------------------------------
// Configuration types
// ---------------------------------------------------------------------------

/// Goose configuration relevant to hooks and event delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooseHookConfig {
    /// Path to the status hook command.
    /// Configured via `GOOSE_STATUS_HOOK` in config.yaml or env var.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_hook: Option<String>,

    /// Tool execution mode. Determines how tool approvals work.
    /// Goose has no hook-based tool approval; this is config-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<GooseMode>,
}

/// Goose operating modes for tool approval.
///
/// These control how Goose handles tool execution permission. Unlike
/// Claude Code, there is no hook mechanism to approve/deny individual
/// tool calls -- these are static configuration modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GooseMode {
    /// All tools execute without approval.
    Auto,
    /// All tools require user approval.
    Approve,
    /// Chat-only mode, no tool execution.
    Chat,
    /// Intelligent approval: safe tools auto-approve, risky tools prompt.
    SmartApprove,
}

impl GooseMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Approve => "approve",
            Self::Chat => "chat",
            Self::SmartApprove => "smart_approve",
        }
    }
}

// ---------------------------------------------------------------------------
// Exit code semantics
// ---------------------------------------------------------------------------

/// Exit code semantics for Goose status hooks.
///
/// Unlike Claude Code where exit codes control blocking behavior,
/// Goose ignores exit codes entirely. This type exists for API
/// symmetry with other provider designs but always returns `Ignored`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// All exit codes are ignored by Goose.
    Ignored,
}

impl ExitCode {
    /// Interpret a raw process exit code. Always returns `Ignored`.
    pub fn from_code(_code: i32) -> Self {
        Self::Ignored
    }

    /// Whether JSON output should be parsed from stdout. Always `false`.
    pub fn should_parse_json(&self) -> bool {
        false
    }

    /// Whether this exit code blocks the action. Always `false`.
    pub fn is_blocking(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Environment and configuration lookup
// ---------------------------------------------------------------------------

/// Environment variables and config keys relevant to Goose hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GooseConfigKey {
    /// `GOOSE_STATUS_HOOK` -- shell command for status transitions.
    StatusHook,
    /// `GOOSE_MODE` -- tool execution mode (auto/approve/chat/smart_approve).
    Mode,
    /// `GOOSE_PROVIDER` -- LLM provider name.
    Provider,
    /// `GOOSE_MODEL` -- model identifier.
    Model,
    /// `GOOSE_CLI_MIN_PRIORITY` -- minimum priority for CLI output rendering.
    CliMinPriority,
}

impl GooseConfigKey {
    /// The environment variable name for this config key.
    pub fn env_var(&self) -> &'static str {
        match self {
            Self::StatusHook => "GOOSE_STATUS_HOOK",
            Self::Mode => "GOOSE_MODE",
            Self::Provider => "GOOSE_PROVIDER",
            Self::Model => "GOOSE_MODEL",
            Self::CliMinPriority => "GOOSE_CLI_MIN_PRIORITY",
        }
    }

    /// The YAML key in `~/.config/goose/config.yaml`.
    pub fn yaml_key(&self) -> &'static str {
        match self {
            Self::StatusHook => "GOOSE_STATUS_HOOK",
            Self::Mode => "GOOSE_MODE",
            Self::Provider => "GOOSE_PROVIDER",
            Self::Model => "GOOSE_MODEL",
            Self::CliMinPriority => "GOOSE_CLI_MIN_PRIORITY",
        }
    }
}
```

## Design Considerations

- **Observe-only architecture.** Goose's hook and event system is fundamentally outbound-only. Unlike Claude Code's bidirectional stdin/stdout JSON protocol, Goose provides no return channel for any event. The `can_block()` and `has_return_channel()` methods always return `false`, making this constraint explicit in the API rather than requiring callers to discover it through documentation.

- **Two delivery mechanisms modeled separately.** Goose delivers events through two independent mechanisms: the `GOOSE_STATUS_HOOK` shell command (fire-and-forget, stdout/stderr suppressed, exit code ignored) and stream-json output (line-delimited JSON events in `goose run --output-format stream-json` mode). The `DeliveryMechanism` enum and the `GooseInput` union type make this duality first-class, allowing the dispatch layer to handle both sources through a single entry point.

- **Composite message events.** Goose's `message` event is a composite that can contain text, tool requests, tool responses, action-required prompts, and thinking blocks in a single payload. Rather than splitting this into separate events (which would misrepresent the wire format), the design keeps it as one `Message` variant with helper methods (`tool_requests()`, `tool_responses()`, `text_content()`) for extracting specific content types. This is a deliberate departure from Claude Code where tool calls are separate `PreToolUse`/`PostToolUse` events.

- **Notification variant union.** Goose notifications can be either log messages or progress updates, distinguished by the presence of a `log` or `progress` field. The `NotificationPayload` models this as two optional fields rather than an inner enum, matching the actual JSON wire format where both fields could theoretically appear (though in practice they are mutually exclusive).

- **Status hook carries no structured payload.** The `GOOSE_STATUS_HOOK` receives only a positional string argument (`waiting` or `thinking`). This is modeled as the `GooseStatus` enum with conversions to/from `GooseEvent`. There is no JSON payload, no session ID, no tool context -- just a bare status string.

- **GooseMode captures tool approval configuration.** Since Goose has no hook-based tool approval mechanism, the `GooseMode` enum documents the four static approval modes (`auto`, `approve`, `chat`, `smart_approve`). This is included because Claudine may need to represent what tool approval capabilities a provider offers, even when they are config-only rather than hook-driven.

- **ExitCode always ignored.** For API symmetry with the Claude Code design, an `ExitCode` type is provided, but it has a single `Ignored` variant. This makes the behavioral difference explicit: calling `is_blocking()` or `should_parse_json()` always returns `false`.

- **Batch output included for completeness.** The `GooseBatchOutput` type models the `--output-format json` mode (single JSON object after run completes). This is not a streaming event but is the other structured output mode Goose supports, and may be useful for CI/pipeline integrations in Claudine.

- **Forward-compatible serde.** All payload structs use `Option` fields with `skip_serializing_if` where the schema may evolve. The `GooseStreamEvent` enum uses `#[serde(tag = "type")]` matching Goose's wire format, so unknown event types will produce deserialization errors that callers can handle gracefully (or a future `Unknown(Value)` variant could be added).

- **Unified mapping is lossy but honest.** The `From<GooseEvent> for AgenticEvent` mapping is total but lossy: `StatusWaiting` maps to `TurnComplete`, `StatusThinking` to `BeforeModel`, `Message` to `AfterModel`, `ModelChange` to `Notification`. The reverse `TryFrom` correctly rejects the 10 unified events that Goose cannot represent (tool hooks, permission, subagent, session start, compaction, human-in-the-loop).

## Claude Code Mapping

- **StatusWaiting -> TurnComplete (loose).** Goose's "waiting" status indicates the agent is idle. The closest Claude Code analog is the `Stop` event (agent finished responding), which maps to `TurnComplete` in the unified model. However, `Stop` carries a `stop_hook_active` field and supports blocking via exit code 2 or `decision: "block"` to continue the conversation. Goose's `StatusWaiting` is purely informational with no payload and no return channel.

- **StatusThinking -> BeforeModel (loose).** Goose's "thinking" status fires when the agent begins processing. Claude Code has no single equivalent -- the closest is the conceptual moment between `UserPromptSubmit` (prompt received) and the first tool/model call. Mapped to `BeforeModel` in the unified model since it represents the start of model inference. Claude Code does not expose a `BeforeModel` event at all.

- **Message -> AfterModel (partial, structural mismatch).** Goose's `message` event is a composite containing assistant text, tool requests, and tool responses in a single payload. Claude Code decomposes this into separate events: `PreToolUse` (tool request created), `PostToolUse` (tool completed), `PostToolUseFailure` (tool failed), and `Stop` (assistant finished). The Goose `message` event carries rich structured data but has no return channel; Claude Code's decomposed events each support blocking, input modification, and context injection. This is the most significant structural divergence between the two providers.

- **Notification -> Notification (partial overlap).** Both Goose and Claude Code have notification events. Goose notifications carry MCP extension/subagent log and progress data with an `extension_id`. Claude Code notifications carry a `message`, `title`, and `notification_type` (permission_prompt, idle_prompt, auth_success, elicitation_dialog). The trigger and payload structures differ substantially, but the semantic role (informational notification) is the same. Claude Code notifications support matcher filtering on `notification_type`; Goose notifications have no filtering mechanism.

- **ModelChange -> No Claude Code equivalent.** Goose emits a `model_change` event when the active model or operating mode changes (e.g., switching between lead and worker models). Claude Code has no equivalent event. The model is reported in `SessionStart` but model changes during a session are not surfaced. Mapped to `AgenticEvent::Notification` as a generic informational event.

- **Error -> TurnError (same semantic, different payload).** Both represent errors in the agent loop. Goose provides a simple `error` string. Claude Code does not have a dedicated error event; errors surface through `PostToolUseFailure` (tool-level) or are handled internally. The unified `TurnError` captures this concept. Goose's error is session-level; Claude Code's errors are tool-level.

- **Complete -> SessionEnd (same semantic, different context).** Both mark the end of a run/session. Goose's `complete` event carries `total_tokens` and is always the final event in a stream. Claude Code's `SessionEnd` carries a `reason` field (clear, logout, prompt_input_exit, etc.) and fires via the hook mechanism. Goose `complete` is outbound-only; Claude Code `SessionEnd` supports matcher filtering on the reason field.

- **No Goose equivalent for: SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, SubagentStart, SubagentStop, TeammateIdle, TaskCompleted, PreCompact.** Goose does not expose pre/post lifecycle hooks for tool calls, has no hook-based permission system, does not emit events for subagent spawn/stop, and has no compaction mechanism. These 11 Claude Code events have no Goose counterpart. Tool call information is embedded within `message` events but cannot be intercepted, modified, or blocked.