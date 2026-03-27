# Codex Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All hook/event types supported by Codex CLI.
///
/// Codex has a fundamentally different hooks architecture than Claude Code.
/// Rather than a rich blocking-hooks system driven by config-file entries,
/// Codex provides:
///
/// 1. A **notify hook** (`config.toml` `notify` key) that fires a
///    fire-and-forget command after each agent turn completes.
/// 2. An **internal AfterToolUse hook** (not yet user-configurable) that
///    fires after each tool execution.
/// 3. A **JSONL event stream** (`codex exec --json`) for non-interactive
///    automation and CI/CD pipelines.
///
/// The JSONL stream events are not "hooks" in the Claude Code sense (they
/// cannot block or steer the agent), but they are the primary observability
/// surface for Codex automation and map to `AgenticEvent` variants.
///
/// Variant names use Codex's native event naming conventions:
/// - Hook events: PascalCase (`AfterAgent`, `AfterToolUse`)
/// - JSONL stream events: dot-separated (`ThreadStarted`, `TurnStarted`, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodexEvent {
    // -- Hook events (fire-and-forget, outbound only) --

    /// Fires after the agent finishes processing a complete turn.
    /// This is the only user-configurable hook via `notify` in config.toml.
    AfterAgent,

    /// Fires after any tool execution completes (success or failure).
    /// Internal only as of CLI 0.102.0 -- not yet wired to user config.
    AfterToolUse,

    // -- JSONL stream events (codex exec --json) --

    /// Emitted once at the beginning of a run. Provides the thread_id
    /// correlation key for all subsequent events.
    ThreadStarted,

    /// Marks the beginning of a new agent turn.
    TurnStarted,

    /// Signals successful completion of a turn with token usage.
    TurnCompleted,

    /// Signals a failed turn. Includes error details.
    TurnFailed,

    /// An item has begun processing (tool call, message, etc.).
    ItemStarted,

    /// An item has been partially updated (incremental data).
    ItemUpdated,

    /// An item has finished processing.
    ItemCompleted,

    /// Fatal or recoverable error event in the stream.
    Error,
}

impl CodexEvent {
    /// All variants in logical order: hooks first, then stream lifecycle.
    pub const ALL: [CodexEvent; 10] = [
        Self::AfterAgent,
        Self::AfterToolUse,
        Self::ThreadStarted,
        Self::TurnStarted,
        Self::TurnCompleted,
        Self::TurnFailed,
        Self::ItemStarted,
        Self::ItemUpdated,
        Self::ItemCompleted,
        Self::Error,
    ];

    /// The native event name string as it appears in Codex payloads.
    ///
    /// Hook events use their internal names. JSONL stream events use the
    /// dot-separated `type` field values from the stream protocol.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::AfterAgent => "agent-turn-complete",
            Self::AfterToolUse => "after_tool_use",
            Self::ThreadStarted => "thread.started",
            Self::TurnStarted => "turn.started",
            Self::TurnCompleted => "turn.completed",
            Self::TurnFailed => "turn.failed",
            Self::ItemStarted => "item.started",
            Self::ItemUpdated => "item.updated",
            Self::ItemCompleted => "item.completed",
            Self::Error => "error",
        }
    }

    /// Codex-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::AfterAgent => {
                "Fires after the agent finishes a complete turn; \
                 the only user-configurable hook via config.toml notify"
            }
            Self::AfterToolUse => {
                "Fires after any tool execution completes; \
                 internal only, not yet exposed to user configuration"
            }
            Self::ThreadStarted => {
                "Emitted once at stream start; provides the thread_id \
                 correlation key for the entire session"
            }
            Self::TurnStarted => "Marks the beginning of a new agent turn in the JSONL stream",
            Self::TurnCompleted => {
                "Signals successful turn completion with token usage statistics"
            }
            Self::TurnFailed => "Signals a failed turn with error details",
            Self::ItemStarted => {
                "An item (tool call, message, file change, etc.) has begun processing"
            }
            Self::ItemUpdated => {
                "Incremental update to an in-progress item; \
                 must be merged with prior state keyed by item.id"
            }
            Self::ItemCompleted => "An item has finished processing with final state",
            Self::Error => "Fatal or recoverable error event in the stream",
        }
    }

    /// Whether this event is a user-configurable hook.
    ///
    /// Only `AfterAgent` is currently configurable via `config.toml`.
    /// `AfterToolUse` exists internally but is not user-facing yet.
    pub fn is_hook(&self) -> bool {
        matches!(self, Self::AfterAgent)
    }

    /// Whether this event exists as an internal hook (present in the
    /// Codex runtime but not exposed to user configuration).
    pub fn is_internal_hook(&self) -> bool {
        matches!(self, Self::AfterToolUse)
    }

    /// Whether this event appears in the JSONL stream (`codex exec --json`).
    pub fn is_stream_event(&self) -> bool {
        matches!(
            self,
            Self::ThreadStarted
                | Self::TurnStarted
                | Self::TurnCompleted
                | Self::TurnFailed
                | Self::ItemStarted
                | Self::ItemUpdated
                | Self::ItemCompleted
                | Self::Error
        )
    }

    /// Whether this event can influence the agent's behavior.
    ///
    /// Codex hooks are fundamentally **outbound-only and fire-and-forget**.
    /// The `notify` hook (AfterAgent) cannot block, deny, or modify anything.
    ///
    /// The internal `AfterToolUse` hook has a `FailedAbort` result that can
    /// terminate the tool call pipeline, but this is not user-configurable.
    ///
    /// JSONL stream events are purely observational.
    pub fn can_block(&self) -> bool {
        // AfterToolUse can abort internally, but this is not user-facing.
        // From the user's perspective, no Codex event can block.
        false
    }

    /// Whether the internal `AfterToolUse` hook can abort the operation.
    ///
    /// This is distinct from `can_block()` because it applies only to the
    /// internal hook API (not user-configurable). Developers building on
    /// the Codex Rust library directly should be aware of this.
    pub fn can_abort_internally(&self) -> bool {
        matches!(self, Self::AfterToolUse)
    }

    /// The delivery mechanism for this event.
    pub fn delivery(&self) -> DeliveryMechanism {
        match self {
            Self::AfterAgent => DeliveryMechanism::CliArgument,
            Self::AfterToolUse => DeliveryMechanism::InternalCallback,
            _ => DeliveryMechanism::JsonlStream,
        }
    }

    /// The typed payload structure this event provides.
    pub fn payload_type(&self) -> PayloadType {
        match self {
            Self::AfterAgent => PayloadType::AgentTurnComplete,
            Self::AfterToolUse => PayloadType::AfterToolUse,
            Self::ThreadStarted => PayloadType::ThreadStarted,
            Self::TurnStarted => PayloadType::TurnStarted,
            Self::TurnCompleted => PayloadType::TurnCompleted,
            Self::TurnFailed => PayloadType::TurnFailed,
            Self::ItemStarted | Self::ItemUpdated | Self::ItemCompleted => PayloadType::Item,
            Self::Error => PayloadType::Error,
        }
    }

    /// The response type this event expects.
    ///
    /// Codex hooks are fire-and-forget. The `notify` hook ignores all
    /// output (stdout, stderr, exit code). The internal `AfterToolUse`
    /// hook returns a `HookResult` enum. JSONL stream events have no
    /// response mechanism at all.
    pub fn response_type(&self) -> ResponseType {
        match self {
            Self::AfterAgent => ResponseType::Ignored,
            Self::AfterToolUse => ResponseType::HookResult,
            _ => ResponseType::None,
        }
    }
}

impl fmt::Display for CodexEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native event names
// ---------------------------------------------------------------------------

impl TryFrom<&str> for CodexEvent {
    type Error = UnknownEventError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "agent-turn-complete" => Ok(Self::AfterAgent),
            "after_tool_use" => Ok(Self::AfterToolUse),
            "thread.started" => Ok(Self::ThreadStarted),
            "turn.started" => Ok(Self::TurnStarted),
            "turn.completed" => Ok(Self::TurnCompleted),
            "turn.failed" => Ok(Self::TurnFailed),
            "item.started" => Ok(Self::ItemStarted),
            "item.updated" => Ok(Self::ItemUpdated),
            "item.completed" => Ok(Self::ItemCompleted),
            "error" => Ok(Self::Error),
            _ => Err(UnknownEventError(name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known Codex event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown Codex event: {0}")]
pub struct UnknownEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

impl From<CodexEvent> for AgenticEvent {
    fn from(event: CodexEvent) -> Self {
        match event {
            CodexEvent::AfterAgent => AgenticEvent::TurnComplete,
            CodexEvent::AfterToolUse => AgenticEvent::AfterTool,
            CodexEvent::ThreadStarted => AgenticEvent::SessionStart,
            CodexEvent::TurnStarted => AgenticEvent::BeforePrompt,
            CodexEvent::TurnCompleted => AgenticEvent::TurnComplete,
            CodexEvent::TurnFailed => AgenticEvent::TurnError,
            // ItemStarted maps to BeforeTool for command_execution items,
            // but items can also be agent_message, file_change, etc.
            // BeforeTool is the closest semantic match since it signals
            // "an action is about to happen."
            CodexEvent::ItemStarted => AgenticEvent::BeforeTool,
            // ItemUpdated has no direct unified equivalent; AfterModel is
            // the closest since updates often carry incremental model output.
            CodexEvent::ItemUpdated => AgenticEvent::AfterModel,
            // ItemCompleted covers tool results, messages, file changes.
            // AfterTool is the closest match for the common case.
            CodexEvent::ItemCompleted => AgenticEvent::AfterTool,
            CodexEvent::Error => AgenticEvent::TurnError,
        }
    }
}

impl TryFrom<AgenticEvent> for CodexEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified to Codex event.
    ///
    /// Many unified events have no Codex equivalent. Events where
    /// multiple Codex events map to the same unified event return
    /// the primary (most common/useful) mapping.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::SessionStart => Ok(CodexEvent::ThreadStarted),
            AgenticEvent::BeforePrompt => Ok(CodexEvent::TurnStarted),
            AgenticEvent::BeforeTool => Ok(CodexEvent::ItemStarted),
            AgenticEvent::AfterTool => Ok(CodexEvent::ItemCompleted),
            AgenticEvent::TurnComplete => Ok(CodexEvent::TurnCompleted),
            AgenticEvent::TurnError => Ok(CodexEvent::TurnFailed),
            AgenticEvent::AfterModel => Ok(CodexEvent::ItemUpdated),
            // No Codex equivalent for these events
            AgenticEvent::SessionEnd
            | AgenticEvent::ToolError
            | AgenticEvent::PermissionRequest
            | AgenticEvent::Notification
            | AgenticEvent::SubagentStart
            | AgenticEvent::SubagentStop
            | AgenticEvent::BeforeModel
            | AgenticEvent::BeforeCompact
            | AgenticEvent::HumanInTheLoop => Err("Codex does not support this event"),
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// How the event payload is delivered to consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMechanism {
    /// Passed as the final CLI argument to the `notify` command.
    /// stdout, stderr, and stdin are all connected to /dev/null.
    CliArgument,
    /// Internal Rust callback within the Codex runtime.
    /// Not user-configurable.
    InternalCallback,
    /// Newline-delimited JSON on stdout via `codex exec --json`.
    JsonlStream,
}

/// Discriminant for the event-specific input payload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    /// The `agent-turn-complete` payload from the notify hook.
    AgentTurnComplete,
    /// The internal `after_tool_use` payload.
    AfterToolUse,
    /// `thread.started` stream event.
    ThreadStarted,
    /// `turn.started` stream event.
    TurnStarted,
    /// `turn.completed` stream event with usage data.
    TurnCompleted,
    /// `turn.failed` stream event with error details.
    TurnFailed,
    /// `item.started`, `item.updated`, or `item.completed` stream event.
    Item,
    /// `error` stream event.
    Error,
}

/// Discriminant for the event-specific response type.
///
/// Unlike Claude Code, Codex hooks are almost entirely fire-and-forget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    /// No response mechanism. JSONL stream events are read-only.
    None,
    /// Output is completely ignored. stdout/stderr/exit code discarded.
    /// Used by the `notify` hook (AfterAgent).
    Ignored,
    /// Internal `HookResult` enum (Success/FailedContinue/FailedAbort).
    /// Used by the internal AfterToolUse hook only.
    HookResult,
}

/// Item types that appear in the JSONL stream.
///
/// Each item in `item.started`/`item.updated`/`item.completed` events
/// has a `type` field identifying what kind of item it represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// Assistant text response.
    AgentMessage,
    /// Model reasoning/thinking (may be suppressed by config).
    Reasoning,
    /// Shell command execution.
    CommandExecution,
    /// File modification.
    FileChange,
    /// MCP server tool invocation.
    McpToolCall,
    /// Web search performed.
    WebSearch,
    /// Agent plan modification.
    PlanUpdate,
}

impl ItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AgentMessage => "agent_message",
            Self::Reasoning => "reasoning",
            Self::CommandExecution => "command_execution",
            Self::FileChange => "file_change",
            Self::McpToolCall => "mcp_tool_call",
            Self::WebSearch => "web_search",
            Self::PlanUpdate => "plan_update",
        }
    }
}

/// Tool kind for the internal AfterToolUse hook payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Function-call style tool.
    Function,
    /// Custom tool (e.g., `apply_patch`).
    Custom,
    /// Shell command execution.
    LocalShell,
    /// MCP server tool call.
    Mcp,
}

impl ToolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Custom => "custom",
            Self::LocalShell => "local_shell",
            Self::Mcp => "mcp",
        }
    }
}

/// Internal hook result for AfterToolUse.
///
/// This is returned by the internal hook callback, not by user-facing hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    /// Hook completed, continue normally.
    Success,
    /// Hook failed, log warning, continue with remaining hooks and operation.
    FailedContinue,
    /// Hook failed, skip remaining hooks, abort the operation with a fatal error.
    FailedAbort,
}

// ---------------------------------------------------------------------------
// Event-specific input payloads
// ---------------------------------------------------------------------------

/// AfterAgent (notify hook) payload.
///
/// Delivered as a JSON string in the final CLI argument.
/// Uses kebab-case field names (legacy format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterAgentPayload {
    /// Always `"agent-turn-complete"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Session identifier (UUID).
    #[serde(rename = "thread-id")]
    pub thread_id: String,

    /// Turn identifier within the session.
    #[serde(rename = "turn-id")]
    pub turn_id: String,

    /// Current working directory when the turn completed.
    pub cwd: String,

    /// User messages that initiated the turn.
    #[serde(rename = "input-messages")]
    pub input_messages: Vec<String>,

    /// Final assistant response text, or null if none.
    #[serde(rename = "last-assistant-message")]
    pub last_assistant_message: Option<String>,
}

/// AfterToolUse (internal hook) payload.
///
/// Uses snake_case and a nested `hook_event` structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterToolUsePayload {
    /// Session identifier (UUID).
    pub session_id: String,

    /// Current working directory.
    pub cwd: String,

    /// ISO 8601 timestamp of when the hook was triggered.
    pub triggered_at: String,

    /// The tool execution details.
    pub hook_event: ToolUseEvent,
}

/// The nested `hook_event` in the AfterToolUse payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseEvent {
    /// Always `"after_tool_use"`.
    pub event_type: String,

    /// Turn within the session.
    pub turn_id: String,

    /// Unique identifier for this tool call.
    pub call_id: String,

    /// Name of the tool (e.g., `local_shell`, `apply_patch`).
    pub tool_name: String,

    /// Kind of tool.
    pub tool_kind: ToolKind,

    /// Tool-specific input (tagged union by `input_type`).
    pub tool_input: ToolInput,

    /// Whether the tool actually ran.
    pub executed: bool,

    /// Whether the tool completed successfully.
    pub success: bool,

    /// Wall-clock execution time in milliseconds.
    pub duration_ms: u64,

    /// Whether the tool modifies state.
    pub mutating: bool,

    /// Sandbox mode used (e.g., `"none"`, `"seatbelt"`).
    pub sandbox: String,

    /// Sandbox policy name.
    pub sandbox_policy: String,

    /// Truncated/serialized tool output.
    pub output_preview: String,
}

/// Tool input for the internal AfterToolUse payload.
///
/// Tagged union discriminated by `input_type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "input_type")]
pub enum ToolInput {
    /// Function-call style tool input.
    #[serde(rename = "function")]
    Function {
        /// JSON-encoded arguments string.
        arguments: String,
    },

    /// Custom tool input (e.g., `apply_patch`).
    #[serde(rename = "custom")]
    Custom {
        /// Input string.
        input: String,
    },

    /// Shell command execution.
    #[serde(rename = "local_shell")]
    LocalShell {
        /// Shell execution parameters.
        params: LocalShellParams,
    },

    /// MCP server tool call.
    #[serde(rename = "mcp")]
    Mcp {
        /// MCP server name.
        server: String,
        /// Tool name within the server.
        tool: String,
        /// JSON-encoded arguments string.
        arguments: String,
    },
}

/// Parameters for a `local_shell` tool input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalShellParams {
    /// The command as an argv array.
    pub command: Vec<String>,

    /// Working directory for the command.
    #[serde(default)]
    pub workdir: Option<String>,

    /// Timeout in milliseconds.
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Sandbox permission mode.
    #[serde(default)]
    pub sandbox_permissions: Option<String>,

    /// Justification for the command (for approval flows).
    #[serde(default)]
    pub justification: Option<String>,

    /// Prefix rule applied (if any).
    #[serde(default)]
    pub prefix_rule: Option<String>,
}

// ---------------------------------------------------------------------------
// JSONL stream event payloads
// ---------------------------------------------------------------------------

/// `thread.started` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadStartedPayload {
    /// Always `"thread.started"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Session-scoped correlation key (UUID).
    pub thread_id: String,
}

/// `turn.started` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStartedPayload {
    /// Always `"turn.started"`.
    #[serde(rename = "type")]
    pub event_type: String,
}

/// `turn.completed` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnCompletedPayload {
    /// Always `"turn.completed"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Token usage statistics for this turn.
    pub usage: TokenUsage,
}

/// Token usage statistics reported in `turn.completed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Total input tokens consumed.
    pub input_tokens: u64,

    /// Input tokens served from cache.
    pub cached_input_tokens: u64,

    /// Output tokens generated.
    pub output_tokens: u64,
}

/// `turn.failed` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnFailedPayload {
    /// Always `"turn.failed"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Error details.
    pub error: StreamError,
}

/// Error details in stream events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamError {
    /// Human-readable error message.
    pub message: String,
}

/// Item lifecycle event payload (`item.started`, `item.updated`, `item.completed`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemPayload {
    /// One of `"item.started"`, `"item.updated"`, `"item.completed"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// The item data. Shape varies by `item.type`.
    pub item: ItemData,
}

/// Data for a single item in the JSONL stream.
///
/// The `item_type` field determines which additional fields are present.
/// Unknown fields are captured in `extra` for forward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemData {
    /// Unique item identifier (stable across started/updated/completed).
    pub id: String,

    /// Item type discriminant.
    #[serde(rename = "type")]
    pub item_type: String,

    /// Item status (e.g., `"in_progress"`, `"completed"`).
    #[serde(default)]
    pub status: Option<String>,

    // -- Fields present depending on item_type --

    /// Text content (agent_message, reasoning).
    #[serde(default)]
    pub text: Option<String>,

    /// Shell command string (command_execution).
    #[serde(default)]
    pub command: Option<String>,

    /// Exit code (command_execution, when completed).
    #[serde(default)]
    pub exit_code: Option<i32>,

    /// Additional type-specific fields not covered above.
    #[serde(flatten)]
    pub extra: Value,
}

/// `error` stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    /// Always `"error"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Error details.
    pub error: StreamError,
}

// ---------------------------------------------------------------------------
// Unified input payload enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any Codex event's input payload.
///
/// For hook events, constructed from the CLI argument JSON or internal
/// callback data. For stream events, constructed by parsing JSONL lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CodexInput {
    /// Notify hook payload (AfterAgent).
    #[serde(rename = "agent-turn-complete")]
    AgentTurnComplete(AfterAgentPayload),

    /// JSONL stream: thread started.
    #[serde(rename = "thread.started")]
    ThreadStarted(ThreadStartedPayload),

    /// JSONL stream: turn started.
    #[serde(rename = "turn.started")]
    TurnStarted(TurnStartedPayload),

    /// JSONL stream: turn completed.
    #[serde(rename = "turn.completed")]
    TurnCompleted(TurnCompletedPayload),

    /// JSONL stream: turn failed.
    #[serde(rename = "turn.failed")]
    TurnFailed(TurnFailedPayload),

    /// JSONL stream: item started.
    #[serde(rename = "item.started")]
    ItemStarted(ItemPayload),

    /// JSONL stream: item updated (incremental).
    #[serde(rename = "item.updated")]
    ItemUpdated(ItemPayload),

    /// JSONL stream: item completed.
    #[serde(rename = "item.completed")]
    ItemCompleted(ItemPayload),

    /// JSONL stream: error.
    #[serde(rename = "error")]
    Error(ErrorPayload),
}

impl CodexInput {
    /// Returns the event variant for this input payload.
    pub fn event(&self) -> CodexEvent {
        match self {
            Self::AgentTurnComplete(_) => CodexEvent::AfterAgent,
            Self::ThreadStarted(_) => CodexEvent::ThreadStarted,
            Self::TurnStarted(_) => CodexEvent::TurnStarted,
            Self::TurnCompleted(_) => CodexEvent::TurnCompleted,
            Self::TurnFailed(_) => CodexEvent::TurnFailed,
            Self::ItemStarted(_) => CodexEvent::ItemStarted,
            Self::ItemUpdated(_) => CodexEvent::ItemUpdated,
            Self::ItemCompleted(_) => CodexEvent::ItemCompleted,
            Self::Error(_) => CodexEvent::Error,
        }
    }

    /// Returns the thread/session ID if available.
    ///
    /// Present in `AgentTurnComplete` (as `thread-id`) and
    /// `ThreadStarted` (as `thread_id`).
    pub fn thread_id(&self) -> Option<&str> {
        match self {
            Self::AgentTurnComplete(p) => Some(&p.thread_id),
            Self::ThreadStarted(p) => Some(&p.thread_id),
            _ => None,
        }
    }

    /// Returns the item ID if this is an item lifecycle event.
    pub fn item_id(&self) -> Option<&str> {
        match self {
            Self::ItemStarted(p) => Some(&p.item.id),
            Self::ItemUpdated(p) => Some(&p.item.id),
            Self::ItemCompleted(p) => Some(&p.item.id),
            _ => None,
        }
    }

    /// Returns the item type string if this is an item lifecycle event.
    pub fn item_type(&self) -> Option<&str> {
        match self {
            Self::ItemStarted(p) => Some(&p.item.item_type),
            Self::ItemUpdated(p) => Some(&p.item.item_type),
            Self::ItemCompleted(p) => Some(&p.item.item_type),
            _ => None,
        }
    }

    /// Returns the error message if this is an error-carrying event.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::TurnFailed(p) => Some(&p.error.message),
            Self::Error(p) => Some(&p.error.message),
            _ => None,
        }
    }

    /// Returns token usage if this is a TurnCompleted event.
    pub fn usage(&self) -> Option<&TokenUsage> {
        match self {
            Self::TurnCompleted(p) => Some(&p.usage),
            _ => None,
        }
    }

    /// Returns the last assistant message if this is an AfterAgent event.
    pub fn last_assistant_message(&self) -> Option<&str> {
        match self {
            Self::AgentTurnComplete(p) => p.last_assistant_message.as_deref(),
            _ => None,
        }
    }
}

// Note: CodexInput does not include AfterToolUse because its payload
// uses a different structure (nested `hook_event`) that is not tagged
// by the top-level `type` field. It must be handled separately:

impl AfterToolUsePayload {
    /// Returns the event variant.
    pub fn event(&self) -> CodexEvent {
        CodexEvent::AfterToolUse
    }

    /// Returns the tool name from the nested hook_event.
    pub fn tool_name(&self) -> &str {
        &self.hook_event.tool_name
    }

    /// Returns whether the tool executed successfully.
    pub fn succeeded(&self) -> bool {
        self.hook_event.executed && self.hook_event.success
    }

    /// Returns whether the tool modifies state.
    pub fn is_mutating(&self) -> bool {
        self.hook_event.mutating
    }

    /// Returns the execution duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.hook_event.duration_ms
    }
}

// ---------------------------------------------------------------------------
// TUI notification events (separate from hooks)
// ---------------------------------------------------------------------------

/// TUI notification event types that can be filtered in config.toml.
///
/// These control terminal desktop alerts, not external hook invocations.
/// Configured via `tui.notifications` as either a boolean or a filtered
/// list of event type strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TuiNotificationEvent {
    /// Agent turn completed.
    #[serde(rename = "agent-turn-complete")]
    AgentTurnComplete,
    /// User approval is requested.
    #[serde(rename = "approval-requested")]
    ApprovalRequested,
}

impl TuiNotificationEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AgentTurnComplete => "agent-turn-complete",
            Self::ApprovalRequested => "approval-requested",
        }
    }
}

/// TUI notification configuration value.
///
/// Can be either a boolean (all on/off) or a list of specific events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TuiNotifications {
    /// Enable or disable all notifications.
    All(bool),
    /// Enable notifications for specific events only.
    Filtered(Vec<TuiNotificationEvent>),
}

// ---------------------------------------------------------------------------
// OpenTelemetry event types (observability, not hooks)
// ---------------------------------------------------------------------------

/// Representative telemetry event types exported via OpenTelemetry.
///
/// These are not hooks and cannot influence agent behavior. They exist
/// for structured observability when `otel.exporter = "otlp"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryEvent {
    /// Session initiated.
    ConversationStarts,
    /// API call made.
    ApiRequest,
    /// Server-sent event received.
    SseEvent,
    /// User prompt submitted (redacted by default).
    UserPrompt,
    /// Tool call decision made.
    ToolDecision,
    /// Tool execution result.
    ToolResult,
}

impl TelemetryEvent {
    pub fn otel_name(&self) -> &'static str {
        match self {
            Self::ConversationStarts => "codex.conversation_starts",
            Self::ApiRequest => "codex.api_request",
            Self::SseEvent => "codex.sse_event",
            Self::UserPrompt => "codex.user_prompt",
            Self::ToolDecision => "codex.tool_decision",
            Self::ToolResult => "codex.tool_result",
        }
    }
}
```

## Design Considerations

- **Fundamentally different architecture from Claude Code.** Codex hooks are outbound-only and fire-and-forget. There is no mechanism for hooks to block, deny, modify, or steer the agent. This is the single most important distinction from Claude Code's rich bidirectional hook system. The design reflects this by having `can_block()` return `false` for all events, and `ResponseType` having only `None`, `Ignored`, and `HookResult` variants (no permission decisions, no blocking patterns, no decision fields).

- **Three distinct event surfaces captured in one enum.** Codex exposes events through three mechanisms: (1) the user-configurable `notify` hook, (2) the internal `AfterToolUse` hook, and (3) the JSONL stream. Rather than splitting these into separate types, the design unifies them under `CodexEvent` with discriminator methods (`is_hook()`, `is_internal_hook()`, `is_stream_event()`) and a `DeliveryMechanism` enum. This keeps the mapping to `AgenticEvent` clean while preserving the distinction.

- **Only 2 actual hooks, vs Claude Code's 14.** Codex has `AfterAgent` (user-configurable) and `AfterToolUse` (internal). The remaining 8 `CodexEvent` variants represent JSONL stream events, not hooks. The design explicitly names and documents this to prevent confusion. Claude Code's hook-centric patterns (matchers, handler types, decision patterns) have no Codex equivalent and are intentionally omitted.

- **No matcher system.** Codex hooks fire unconditionally. There is no filtering by tool name, event subtype, or pattern. The `MatcherField` enum from Claude Code's design has no counterpart here. The TUI notification filter (`tui.notifications`) is modeled separately as `TuiNotifications` since it only controls desktop alerts, not hook invocation.

- **kebab-case vs snake_case payload asymmetry.** The `notify` hook payload uses legacy kebab-case field names (`thread-id`, `turn-id`, `input-messages`), while the internal `AfterToolUse` payload uses snake_case. The JSONL stream uses snake_case for most fields but kebab-case is not present. Serde `rename` attributes handle this cleanly.

- **Payload delivery via CLI argument, not stdin.** The `notify` hook receives its JSON payload as the last CLI argument, not on stdin. This is fundamentally different from Claude Code (which pipes JSON to stdin). The `DeliveryMechanism::CliArgument` variant captures this, and the `AfterAgentPayload` struct is designed to be parsed from `argv[1]`.

- **AfterToolUse excluded from the tagged union.** The `CodexInput` tagged enum dispatches on the `type` field, which works for the notify payload and all JSONL stream events. However, the `AfterToolUsePayload` uses a nested `hook_event` structure without a top-level `type` discriminator, so it is handled separately with its own convenience methods. This avoids forcing an awkward untagged variant into the enum.

- **Item lifecycle as a first-class concept.** Codex's JSONL stream revolves around items (tool calls, messages, file changes) that go through started/updated/completed phases. The `ItemType` enum captures the 7 known item types. The `ItemData` struct uses a flat structure with optional fields plus a `#[serde(flatten)]` catch-all for forward compatibility, reflecting the documented advice to use tolerant JSON parsing since the schema is experimental.

- **Internal abort capability documented but not exposed.** The `AfterToolUse` hook can return `FailedAbort` which produces a fatal error. This is captured in `HookResult` and documented via `can_abort_internally()`, but kept separate from `can_block()` to clearly signal that this is not a user-facing capability.

- **TUI notifications and OTel telemetry modeled as separate concerns.** These are not hooks but are part of Codex's event surface. `TuiNotificationEvent` and `TelemetryEvent` are included for completeness but kept structurally separate from `CodexEvent` since they serve different purposes (desktop alerts and structured observability, respectively).

- **Forward compatibility baked in.** `ItemData` uses `#[serde(flatten)]` for unknown fields. The `--json` flag is documented as experimental (`--experimental-json`), so the design uses `Option` fields liberally and avoids hard-coding exact payload shapes beyond the documented `type`, `thread_id`, and `item.id` fields.

- **`From<CodexEvent> for AgenticEvent` is total but lossy.** Every Codex event maps to some unified event, but the mappings are semantically approximate. `ItemStarted` maps to `BeforeTool` (closest match for "action beginning"), `ItemUpdated` maps to `AfterModel` (incremental output), and both `AfterAgent` and `TurnCompleted` map to `TurnComplete`. Comments document the semantic gaps. The reverse `TryFrom` is partial, returning `Err` for the 9 unified events Codex does not support.

## Claude Code Mapping

- **`CodexEvent::AfterAgent`** maps to **`ClaudeCodeEvent::Stop`** (both fire when the agent finishes responding). Key difference: Claude Code's `Stop` is a blocking hook that can continue the conversation via `decision: "block"`. Codex's `AfterAgent` is fire-and-forget with no response mechanism. Both map to `AgenticEvent::TurnComplete`.

- **`CodexEvent::AfterToolUse`** maps to **`ClaudeCodeEvent::PostToolUse`** (both fire after tool execution). Key difference: Claude Code's `PostToolUse` provides strongly-typed tool response data and supports `decision: "block"` plus `additionalContext`. Codex's `AfterToolUse` is internal-only, provides rich execution metadata (duration, sandbox, mutating flag), and can return `FailedAbort` to terminate the pipeline. Both map to `AgenticEvent::AfterTool`.

- **`CodexEvent::ThreadStarted`** maps to **`ClaudeCodeEvent::SessionStart`** (both signal session beginning). Key difference: Claude Code's `SessionStart` fires on startup, resume, clear, and post-compaction, and supports context injection via `CLAUDE_ENV_FILE`. Codex's `ThreadStarted` fires once per `codex exec` run and only provides a `thread_id`. Both map to `AgenticEvent::SessionStart`.

- **`CodexEvent::TurnStarted`** maps to **`ClaudeCodeEvent::UserPromptSubmit`** (both mark the start of processing). Key difference: Claude Code's `UserPromptSubmit` provides the prompt text and can block submission. Codex's `TurnStarted` is a minimal marker event with no payload beyond the type field. Both map to `AgenticEvent::BeforePrompt`.

- **`CodexEvent::TurnCompleted`** maps to **`ClaudeCodeEvent::Stop`** (both signal turn completion). Key difference: Claude Code's `Stop` includes `stop_hook_active` for infinite-loop prevention and supports blocking. Codex's `TurnCompleted` provides `TokenUsage` statistics (input_tokens, cached_input_tokens, output_tokens) which Claude Code does not expose. Both map to `AgenticEvent::TurnComplete`.

- **`CodexEvent::TurnFailed`** has **no direct Claude Code equivalent**. Claude Code does not have a `TurnError` or `TurnFailed` event. Tool failures are captured by `PostToolUseFailure`, but session-level turn failures are not surfaced as hook events. Maps to `AgenticEvent::TurnError`.

- **`CodexEvent::ItemStarted`** partially maps to **`ClaudeCodeEvent::PreToolUse`** (for command_execution items). Key difference: Claude Code's `PreToolUse` is tool-specific, provides `tool_name`/`tool_input`/`tool_use_id`, and supports three-level permission decisions (allow/deny/ask). Codex's `ItemStarted` is a generic item lifecycle event that covers tools, messages, file changes, and more. For non-tool items (e.g., `agent_message`), there is no Claude Code equivalent. Maps to `AgenticEvent::BeforeTool`.

- **`CodexEvent::ItemUpdated`** has **no direct Claude Code equivalent**. Claude Code does not stream incremental item updates. The closest concept is the main agent producing partial output, but this is not exposed as a hook event. Maps to `AgenticEvent::AfterModel` as the closest approximation (incremental model output).

- **`CodexEvent::ItemCompleted`** partially maps to **`ClaudeCodeEvent::PostToolUse`** (for command_execution items) or **`ClaudeCodeEvent::PostToolUseFailure`** (for failed items). Key difference: Claude Code separates success and failure into distinct events with different payloads. Codex uses a single `ItemCompleted` event where `exit_code` and `status` indicate success or failure. Maps to `AgenticEvent::AfterTool`.

- **`CodexEvent::Error`** has **no direct Claude Code equivalent**. Claude Code does not expose a generic error stream event. The closest is `PostToolUseFailure` for tool-level errors, but session-level errors are not surfaced. Maps to `AgenticEvent::TurnError`.

- **Claude Code events with no Codex equivalent:**
    - `SessionEnd` -- Codex has no session end event; the process simply exits.
    - `PermissionRequest` -- Codex uses approval policies (`full_auto`, `auto_edit`, `ask_always`) configured statically, not dynamic permission hooks. The `approval-requested` TUI notification exists but is not a hook.
    - `SubagentStart` / `SubagentStop` -- Codex does not expose subagent lifecycle events.
    - `Notification` -- Codex has TUI notifications but no corresponding hook event.
    - `PreCompact` -- Codex does not expose a compaction event.
    - `UserPromptSubmit` (blocking) -- Codex's `TurnStarted` is the closest analog but carries no prompt text and cannot block.
    - `TeammateIdle` / `TaskCompleted` -- Codex has no team/task concepts.
    - `PreToolUse` (blocking/steering) -- Codex's `ItemStarted` is observational only; it cannot modify or deny tool calls.
