# Qwen CLI Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All 7 hook-like events available in Qwen Code (v0.10.x).
///
/// Qwen Code does NOT have a user-facing lifecycle hook system like Claude
/// Code. Instead, it exposes a collection of disparate integration surfaces:
/// SDK permission callbacks, internal subagent hooks, and headless stream
/// events. This enum normalizes all of those surfaces into a single
/// discriminant so that Claudine can treat them uniformly.
///
/// Variant names use Qwen's native naming where one exists (e.g.,
/// `CanUseTool`, `SubagentPreToolUse`). For headless stream events that
/// lack an official hook name, we use descriptive PascalCase names prefixed
/// by `Stream` to distinguish them from true lifecycle hooks.
///
/// When Qwen Code ships its official hook system (roadmap P2), this enum
/// should be revised to match the finalized event names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QwenCliEvent {
    /// SDK `canUseTool` permission callback.
    CanUseTool,
    /// Internal subagent hook: before tool execution.
    SubagentPreToolUse,
    /// Internal subagent hook: after tool execution.
    SubagentPostToolUse,
    /// Internal subagent hook: subagent terminated.
    SubagentStop,
    /// Headless stream: session started (`system` event, `session_start`).
    StreamSessionStart,
    /// Headless stream: assistant message emitted.
    StreamAssistantMessage,
    /// Headless stream: final result (success or error).
    StreamResult,
}

impl QwenCliEvent {
    /// All variants in approximate lifecycle order.
    pub const ALL: [QwenCliEvent; 7] = [
        Self::StreamSessionStart,
        Self::CanUseTool,
        Self::SubagentPreToolUse,
        Self::SubagentPostToolUse,
        Self::StreamAssistantMessage,
        Self::SubagentStop,
        Self::StreamResult,
    ];

    /// The native event/hook name as it appears in Qwen Code source.
    ///
    /// For SDK callbacks and internal hooks, this is the function/method
    /// name. For stream events, this is the JSON `type` field value.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::CanUseTool => "canUseTool",
            Self::SubagentPreToolUse => "preToolUse",
            Self::SubagentPostToolUse => "postToolUse",
            Self::SubagentStop => "onStop",
            Self::StreamSessionStart => "system",
            Self::StreamAssistantMessage => "assistant",
            Self::StreamResult => "result",
        }
    }

    /// Qwen Code-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::CanUseTool => {
                "SDK permission callback invoked when a tool execution requires \
                 confirmation (skipped in yolo mode or for allowedTools)"
            }
            Self::SubagentPreToolUse => {
                "Internal subagent hook fired after TOOL_CALL event is emitted \
                 but before tool execution begins (notification only, cannot block)"
            }
            Self::SubagentPostToolUse => {
                "Internal subagent hook fired after tool execution completes, \
                 providing timing and success/error status (notification only)"
            }
            Self::SubagentStop => {
                "Internal subagent hook fired when a subagent terminates, \
                 providing termination reason and summary (notification only)"
            }
            Self::StreamSessionStart => {
                "Headless stream event emitted at session start with session ID, \
                 model, and configuration metadata"
            }
            Self::StreamAssistantMessage => {
                "Headless stream event containing the full assistant message \
                 with content blocks and token usage"
            }
            Self::StreamResult => {
                "Headless stream event emitted at session end with final status, \
                 duration, usage stats, and summary"
            }
        }
    }

    /// The integration surface this event belongs to.
    ///
    /// Qwen Code's hook-like capabilities span three distinct surfaces,
    /// each with different access patterns and capabilities.
    pub fn surface(&self) -> QwenSurface {
        match self {
            Self::CanUseTool => QwenSurface::SdkCallback,
            Self::SubagentPreToolUse
            | Self::SubagentPostToolUse
            | Self::SubagentStop => QwenSurface::InternalSubagentHook,
            Self::StreamSessionStart
            | Self::StreamAssistantMessage
            | Self::StreamResult => QwenSurface::HeadlessStream,
        }
    }

    /// Whether this event can block or modify agent behavior.
    ///
    /// Only `CanUseTool` can block (deny a tool call) or modify (rewrite
    /// tool input via `updatedInput`). All subagent hooks and stream
    /// events are strictly informational.
    pub fn can_block(&self) -> bool {
        matches!(self, Self::CanUseTool)
    }

    /// Whether this event can modify tool input before execution.
    ///
    /// Only `CanUseTool` supports `updatedInput` in its response.
    pub fn can_modify_input(&self) -> bool {
        matches!(self, Self::CanUseTool)
    }

    /// Whether this event is currently user-configurable.
    ///
    /// As of v0.10.x, none of these events are configurable via
    /// `settings.json`. `CanUseTool` is configurable via the SDK only.
    /// Subagent hooks are internal. Stream events are output-only.
    pub fn is_user_configurable(&self) -> bool {
        // CanUseTool is configurable via SDK, but not via settings.json
        matches!(self, Self::CanUseTool)
    }

    /// Whether this event's callback is awaited by the runtime.
    ///
    /// `SubagentPreToolUse` is fire-and-forget (errors are swallowed).
    /// All other hooks are awaited.
    pub fn is_awaited(&self) -> bool {
        !matches!(self, Self::SubagentPreToolUse)
    }

    /// The typed payload structure this event provides.
    pub fn payload_type(&self) -> QwenPayloadType {
        match self {
            Self::CanUseTool => QwenPayloadType::CanUseTool,
            Self::SubagentPreToolUse => QwenPayloadType::SubagentPreToolUse,
            Self::SubagentPostToolUse => QwenPayloadType::SubagentPostToolUse,
            Self::SubagentStop => QwenPayloadType::SubagentStop,
            Self::StreamSessionStart => QwenPayloadType::StreamSessionStart,
            Self::StreamAssistantMessage => QwenPayloadType::StreamAssistantMessage,
            Self::StreamResult => QwenPayloadType::StreamResult,
        }
    }

    /// The typed response structure this event expects.
    pub fn response_type(&self) -> QwenResponseType {
        match self {
            Self::CanUseTool => QwenResponseType::PermissionResult,
            Self::SubagentPreToolUse
            | Self::SubagentPostToolUse
            | Self::SubagentStop => QwenResponseType::Void,
            Self::StreamSessionStart
            | Self::StreamAssistantMessage
            | Self::StreamResult => QwenResponseType::OutputOnly,
        }
    }
}

impl fmt::Display for QwenCliEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native event names
// ---------------------------------------------------------------------------

impl TryFrom<&str> for QwenCliEvent {
    type Error = UnknownQwenEventError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "canUseTool" => Ok(Self::CanUseTool),
            "preToolUse" => Ok(Self::SubagentPreToolUse),
            "postToolUse" => Ok(Self::SubagentPostToolUse),
            "onStop" => Ok(Self::SubagentStop),
            "system" => Ok(Self::StreamSessionStart),
            "assistant" => Ok(Self::StreamAssistantMessage),
            "result" => Ok(Self::StreamResult),
            _ => Err(UnknownQwenEventError(name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known Qwen Code event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown Qwen Code event: {0}")]
pub struct UnknownQwenEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

impl From<QwenCliEvent> for AgenticEvent {
    fn from(event: QwenCliEvent) -> Self {
        match event {
            QwenCliEvent::CanUseTool => AgenticEvent::PermissionRequest,
            QwenCliEvent::SubagentPreToolUse => AgenticEvent::BeforeTool,
            QwenCliEvent::SubagentPostToolUse => AgenticEvent::AfterTool,
            QwenCliEvent::SubagentStop => AgenticEvent::SubagentStop,
            QwenCliEvent::StreamSessionStart => AgenticEvent::SessionStart,
            QwenCliEvent::StreamAssistantMessage => AgenticEvent::AfterModel,
            QwenCliEvent::StreamResult => AgenticEvent::SessionEnd,
        }
    }
}

impl TryFrom<AgenticEvent> for QwenCliEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified to Qwen Code event.
    ///
    /// Because Qwen Code's surfaces are fragmented and incomplete, many
    /// unified events have no Qwen equivalent. Events that map 1:1 succeed.
    /// Events with no Qwen representation return `Err`.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::PermissionRequest => Ok(QwenCliEvent::CanUseTool),
            AgenticEvent::BeforeTool => Ok(QwenCliEvent::SubagentPreToolUse),
            AgenticEvent::AfterTool => Ok(QwenCliEvent::SubagentPostToolUse),
            AgenticEvent::SubagentStop => Ok(QwenCliEvent::SubagentStop),
            AgenticEvent::SessionStart => Ok(QwenCliEvent::StreamSessionStart),
            AgenticEvent::AfterModel => Ok(QwenCliEvent::StreamAssistantMessage),
            AgenticEvent::SessionEnd => Ok(QwenCliEvent::StreamResult),
            AgenticEvent::BeforePrompt
            | AgenticEvent::ToolError
            | AgenticEvent::TurnComplete
            | AgenticEvent::TurnError
            | AgenticEvent::SubagentStart
            | AgenticEvent::BeforeModel
            | AgenticEvent::BeforeCompact
            | AgenticEvent::Notification
            | AgenticEvent::HumanInTheLoop => {
                Err("Qwen Code does not support this event")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// The integration surface a Qwen Code event belongs to.
///
/// Unlike Claude Code where all events flow through a single hook system,
/// Qwen Code's hook-like capabilities are spread across three independent
/// surfaces with different access patterns, capabilities, and audiences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QwenSurface {
    /// SDK-level callback (`canUseTool`). Requires programmatic SDK usage.
    /// This is the only surface that can block or modify agent behavior.
    SdkCallback,
    /// Internal hooks on the `SubagentHooks` interface. Not user-facing.
    /// Fire-and-forget notification pattern only.
    InternalSubagentHook,
    /// Headless stream-json output events. Output-only (no input channel).
    /// Consumed via `--output-format stream-json`.
    HeadlessStream,
}

impl QwenSurface {
    /// Whether this surface supports bidirectional communication.
    pub fn is_bidirectional(&self) -> bool {
        matches!(self, Self::SdkCallback)
    }

    /// Whether this surface is accessible to end users (vs internal only).
    pub fn is_user_facing(&self) -> bool {
        matches!(self, Self::SdkCallback | Self::HeadlessStream)
    }
}

/// Discriminant for the event-specific input payload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QwenPayloadType {
    CanUseTool,
    SubagentPreToolUse,
    SubagentPostToolUse,
    SubagentStop,
    StreamSessionStart,
    StreamAssistantMessage,
    StreamResult,
}

/// Discriminant for the event-specific response type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QwenResponseType {
    /// `CanUseTool`: returns allow (with updatedInput) or deny (with message).
    PermissionResult,
    /// Subagent hooks: `Promise<void> | void` -- no return value processed.
    Void,
    /// Stream events: output-only, no response channel exists.
    OutputOnly,
}

// ---------------------------------------------------------------------------
// Qwen permission modes (maps to approval mode)
// ---------------------------------------------------------------------------

/// Qwen Code approval modes that affect whether `CanUseTool` is invoked.
///
/// These determine tool gating behavior. In `Yolo` mode, `CanUseTool`
/// is never called. In `Plan` mode, non-read-only tools are blocked
/// before the callback is reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QwenApprovalMode {
    /// Read-only analysis. All tool execution blocked.
    Plan,
    /// Manual approval for file edits and shell commands.
    Default,
    /// File edits auto-approved, shell commands need manual approval.
    AutoEdit,
    /// Everything auto-approved. `CanUseTool` is never invoked.
    Yolo,
}

// ---------------------------------------------------------------------------
// Event-specific input payloads
// ---------------------------------------------------------------------------

/// `CanUseTool` callback payload.
///
/// Delivered when the permission system requires confirmation for a tool
/// call. Skipped entirely when `permissionMode` is `yolo`, the tool is
/// in `allowedTools`, or the tool was already blocked by `excludeTools`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanUseToolPayload {
    /// Name of the tool requesting permission.
    pub tool_name: String,

    /// Tool arguments as key-value pairs.
    pub input: Value,

    /// AbortSignal for timeout/cancellation (60-second deadline).
    /// Represented as a boolean indicating whether the signal has fired.
    #[serde(default)]
    pub aborted: bool,

    /// Optional permission suggestions (e.g., "always allow this tool").
    #[serde(default)]
    pub suggestions: Option<Vec<QwenPermissionSuggestion>>,
}

/// A permission suggestion entry for `CanUseTool`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QwenPermissionSuggestion {
    /// Suggestion type.
    #[serde(rename = "type")]
    pub suggestion_type: String,

    /// Additional metadata for the suggestion.
    #[serde(flatten)]
    pub extra: Value,
}

/// `SubagentPreToolUse` internal hook payload.
///
/// Notification-only. Fires after `TOOL_CALL` event but before tool
/// execution. Errors in this callback are silently swallowed because
/// it is not awaited.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentPreToolUsePayload {
    /// Unique identifier of the subagent.
    pub subagent_id: String,

    /// Human-readable name of the subagent.
    pub name: String,

    /// Name of the tool about to execute.
    pub tool_name: String,

    /// Tool arguments.
    pub args: Value,

    /// Unix timestamp (milliseconds) when the event was created.
    pub timestamp: u64,
}

/// `SubagentPostToolUse` internal hook payload.
///
/// Notification-only. Extends `SubagentPreToolUse` with execution results.
/// This callback IS awaited, unlike `preToolUse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentPostToolUsePayload {
    /// Unique identifier of the subagent.
    pub subagent_id: String,

    /// Human-readable name of the subagent.
    pub name: String,

    /// Name of the tool that executed.
    pub tool_name: String,

    /// Tool arguments that were sent.
    pub args: Value,

    /// Unix timestamp (milliseconds) when the event was created.
    pub timestamp: u64,

    /// Whether the tool execution succeeded.
    pub success: bool,

    /// Execution duration in milliseconds.
    pub duration_ms: u64,

    /// Error message if `success` is false.
    #[serde(default)]
    pub error_message: Option<String>,
}

/// `SubagentStop` internal hook payload.
///
/// Notification-only. Fires in the `finally` block after subagent
/// execution terminates. This callback IS awaited.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentStopPayload {
    /// Unique identifier of the subagent.
    pub subagent_id: String,

    /// Human-readable name of the subagent.
    pub name: String,

    /// Why the subagent terminated.
    pub terminate_reason: String,

    /// Summary object with subagent execution details.
    pub summary: Value,

    /// Unix timestamp (milliseconds) when the event was created.
    pub timestamp: u64,
}

/// `StreamSessionStart` headless event payload.
///
/// Emitted as the first event in a headless stream-json session.
/// The `type` field is `"system"` and `subtype` is `"session_start"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamSessionStartPayload {
    /// Always `"system"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Always `"session_start"`.
    pub subtype: String,

    /// Session identifier.
    #[serde(default)]
    pub session_id: Option<String>,

    /// Model being used.
    #[serde(default)]
    pub model: Option<String>,

    /// Additional configuration metadata.
    #[serde(flatten)]
    pub extra: Value,
}

/// `StreamAssistantMessage` headless event payload.
///
/// Contains a complete assistant message with content blocks and
/// token usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamAssistantMessagePayload {
    /// Always `"assistant"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Model that generated the message.
    #[serde(default)]
    pub model: Option<String>,

    /// Role (always "assistant").
    #[serde(default)]
    pub role: Option<String>,

    /// Content blocks (text, tool_use, etc.).
    #[serde(default)]
    pub content: Vec<Value>,

    /// Token usage statistics.
    #[serde(default)]
    pub usage: Option<Value>,
}

/// `StreamResult` headless event payload.
///
/// Final event in a headless stream-json session. Contains overall
/// execution status and metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResultPayload {
    /// Always `"result"`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// `"success"` or an error description.
    #[serde(default)]
    pub subtype: Option<String>,

    /// Total execution duration in milliseconds.
    #[serde(default)]
    pub duration_ms: Option<u64>,

    /// Aggregated token usage across the session.
    #[serde(default)]
    pub usage: Option<Value>,

    /// Summary of execution.
    #[serde(default)]
    pub summary: Option<String>,

    /// Additional result metadata.
    #[serde(flatten)]
    pub extra: Value,
}

// ---------------------------------------------------------------------------
// Unified input payload enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any Qwen Code event's input payload.
///
/// Because Qwen Code's events come from three different surfaces,
/// deserialization depends on the surface being consumed. The `event()`
/// method returns the appropriate `QwenCliEvent` discriminant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QwenCliInput {
    CanUseTool(CanUseToolPayload),
    SubagentPreToolUse(SubagentPreToolUsePayload),
    SubagentPostToolUse(SubagentPostToolUsePayload),
    SubagentStop(SubagentStopPayload),
    StreamSessionStart(StreamSessionStartPayload),
    StreamAssistantMessage(StreamAssistantMessagePayload),
    StreamResult(StreamResultPayload),
}

impl QwenCliInput {
    /// Returns the event variant for this input payload.
    pub fn event(&self) -> QwenCliEvent {
        match self {
            Self::CanUseTool(_) => QwenCliEvent::CanUseTool,
            Self::SubagentPreToolUse(_) => QwenCliEvent::SubagentPreToolUse,
            Self::SubagentPostToolUse(_) => QwenCliEvent::SubagentPostToolUse,
            Self::SubagentStop(_) => QwenCliEvent::SubagentStop,
            Self::StreamSessionStart(_) => QwenCliEvent::StreamSessionStart,
            Self::StreamAssistantMessage(_) => QwenCliEvent::StreamAssistantMessage,
            Self::StreamResult(_) => QwenCliEvent::StreamResult,
        }
    }

    /// Returns the tool name if this is a tool-related event.
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::CanUseTool(p) => Some(&p.tool_name),
            Self::SubagentPreToolUse(p) => Some(&p.tool_name),
            Self::SubagentPostToolUse(p) => Some(&p.tool_name),
            _ => None,
        }
    }

    /// Returns the subagent ID if this is a subagent-related event.
    pub fn subagent_id(&self) -> Option<&str> {
        match self {
            Self::SubagentPreToolUse(p) => Some(&p.subagent_id),
            Self::SubagentPostToolUse(p) => Some(&p.subagent_id),
            Self::SubagentStop(p) => Some(&p.subagent_id),
            _ => None,
        }
    }

    /// Returns the subagent name if this is a subagent-related event.
    pub fn subagent_name(&self) -> Option<&str> {
        match self {
            Self::SubagentPreToolUse(p) => Some(&p.name),
            Self::SubagentPostToolUse(p) => Some(&p.name),
            Self::SubagentStop(p) => Some(&p.name),
            _ => None,
        }
    }

    /// Returns the timestamp (millis) if the event carries one.
    pub fn timestamp(&self) -> Option<u64> {
        match self {
            Self::SubagentPreToolUse(p) => Some(p.timestamp),
            Self::SubagentPostToolUse(p) => Some(p.timestamp),
            Self::SubagentStop(p) => Some(p.timestamp),
            _ => None,
        }
    }

    /// Whether this event reports a tool execution failure.
    pub fn is_tool_error(&self) -> bool {
        match self {
            Self::SubagentPostToolUse(p) => !p.success,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// `CanUseTool` response: allow or deny the tool execution.
///
/// This is the only response type in Qwen Code that has any effect on
/// agent behavior. The 60-second timeout auto-denies if no response
/// is returned.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "lowercase")]
pub enum QwenPermissionResult {
    /// Allow tool execution. `updated_input` replaces the original
    /// arguments before the tool runs.
    Allow {
        /// Modified tool arguments. Replaces the original input entirely.
        #[serde(rename = "updatedInput")]
        updated_input: Value,
    },
    /// Deny tool execution. `message` is surfaced to the model as the
    /// denial reason. `interrupt` halts the entire session if true.
    Deny {
        /// Reason shown to the model explaining why the tool was denied.
        message: String,
        /// If true, interrupts the entire session (not just this tool call).
        #[serde(default)]
        interrupt: bool,
    },
}

impl QwenPermissionResult {
    /// Create an allow response that passes through the original input.
    pub fn allow(original_input: Value) -> Self {
        Self::Allow {
            updated_input: original_input,
        }
    }

    /// Create a deny response with a reason message.
    pub fn deny(message: impl Into<String>) -> Self {
        Self::Deny {
            message: message.into(),
            interrupt: false,
        }
    }

    /// Create a deny response that also interrupts the session.
    pub fn deny_and_interrupt(message: impl Into<String>) -> Self {
        Self::Deny {
            message: message.into(),
            interrupt: true,
        }
    }

    /// Whether this result allows the tool to proceed.
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}

// ---------------------------------------------------------------------------
// Permission priority chain
// ---------------------------------------------------------------------------

/// The priority chain that determines whether `CanUseTool` is invoked.
///
/// Steps are evaluated in order. The first matching rule short-circuits.
/// `CanUseTool` is only reached at step 5 if no earlier rule matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPriority {
    /// Tool is in `excludeTools` -- absolute block, callback never runs.
    ExcludedTool,
    /// `permissionMode` is `plan` -- non-read-only tools blocked.
    PlanMode,
    /// `permissionMode` is `yolo` -- universal auto-approve.
    YoloMode,
    /// Tool is in `allowedTools` -- auto-approve for matching tools.
    AllowedTool,
    /// `canUseTool` callback invoked for custom decision logic.
    Callback,
    /// No callback provided -- default denial.
    DefaultDeny,
}
```

## Design Considerations

- **Three distinct surfaces, not one hook system.** Unlike Claude Code where all 14 events flow through a single, unified hook configuration in `settings.json`, Qwen Code's hook-like capabilities are fragmented across three independent surfaces: an SDK callback (`canUseTool`), internal subagent hooks (`SubagentHooks` interface), and headless stream-json output events. The `QwenSurface` enum captures this fundamental architectural difference, and each event is tagged with its surface so callers understand the access pattern and limitations.

- **Only one blocking event.** `CanUseTool` is the sole event in Qwen Code that can influence agent behavior (by allowing/denying a tool call or rewriting its input). The three subagent hooks are notification-only (`void` return). The three stream events are output-only with no input channel at all. This is a major departure from Claude Code where 7 of 14 events support blocking. The `can_block()` method reflects this reality rather than optimistically modeling capabilities that do not exist.

- **Subagent hooks are internal and not user-configurable.** The `SubagentHooks` interface (`preToolUse`, `postToolUse`, `onStop`) exists in `packages/core/src/subagents/subagent-hooks.ts` but is not exposed via settings files, CLI flags, or the public SDK. They are included in this design because they are the closest thing Qwen Code has to tool lifecycle hooks and may become user-facing when the official hook system ships. The `is_user_configurable()` method makes this limitation explicit.

- **`preToolUse` is fire-and-forget; others are awaited.** Qwen Code's `SubagentPreToolUse` callback is invoked without `await`, meaning errors are silently swallowed and timing is non-deterministic. The `is_awaited()` method captures this behavioral asymmetry so the dispatch layer can handle it appropriately (e.g., skip error reporting for non-awaited hooks).

- **Stream events modeled as events, not hooks.** Headless stream-json events (`system`, `assistant`, `result`) are strictly output-side. There is no input channel to feed responses back into the agent. They are included in this design because they provide the only external visibility into session lifecycle (start/message/end) and are the primary automation surface for headless consumers. The `QwenResponseType::OutputOnly` variant makes it clear that no response processing is possible.

- **No matcher system.** Qwen Code has no matcher/filter system. All subagent hooks fire unconditionally for every tool call within a subagent. There is no per-tool filtering, no regex matching, and no event-specific filtering. Unlike the Claude Code design which has `MatcherField`, `supports_matcher()`, and `matcher_value()`, this design omits matcher infrastructure entirely.

- **No common input fields across surfaces.** Claude Code provides `CommonInputFields` (session_id, transcript_path, cwd, permission_mode, hook_event_name) on every event. Qwen Code has no equivalent shared context across its three surfaces. SDK callbacks get tool name and input. Subagent hooks get subagent ID, name, and tool details. Stream events get type-specific JSON. The `QwenCliInput` enum uses `#[serde(untagged)]` rather than `#[serde(tag = ...)]` because there is no shared discriminant field.

- **Permission result is a tagged enum, not a struct with optional fields.** The `QwenPermissionResult` uses `#[serde(tag = "behavior")]` to model the allow/deny response as a proper tagged union. This is more idiomatic than Claude Code's approach of using optional fields with comments about which fields apply to which behavior. The convenience constructors (`allow()`, `deny()`, `deny_and_interrupt()`) provide ergonomic creation.

- **Permission priority chain documented in-type.** The `PermissionPriority` enum captures the 6-step evaluation chain that determines whether `canUseTool` is even invoked. This is critical operational knowledge: if `permissionMode` is `yolo` or a tool is in `allowedTools`, the callback is silently skipped. Embedding this in the type system makes the gotcha discoverable.

- **Forward-looking design.** The roadmap lists hooks as "In Progress" (P2 priority). When Qwen Code ships its official hook system, this enum will likely need significant expansion to cover Claude-style lifecycle events (SessionStart, SessionEnd, PreToolUse at the main agent level, Stop, etc.). The current design is intentionally conservative, modeling only what exists today, with doc comments noting where expansion is expected.

- **60-second timeout on `CanUseTool`.** The SDK enforces a 60-second deadline on the permission callback. If no response is returned within that window, the tool call is auto-denied. This timeout is not configurable. The `CanUseToolPayload` includes an `aborted` field to indicate whether the abort signal has fired.

## Claude Code Mapping

- **`QwenCliEvent::CanUseTool`** maps to **`ClaudeCodeEvent::PermissionRequest`** -- same trigger (tool execution needs user/hook confirmation), similar payload (tool name + input + permission suggestions), and similar response (allow with optional input rewriting, or deny with message). Key differences: (1) Claude Code has a separate `PreToolUse` event that fires on ALL tool calls regardless of permission mode, while Qwen's `CanUseTool` is skipped entirely in yolo mode or for allowed tools; (2) Claude Code's `PermissionRequest` has `updatedPermissions` for persisting "always allow" rules, which Qwen lacks; (3) Claude Code's `PreToolUse` supports a third option `"ask"` (show the permission dialog), which has no Qwen equivalent; (4) Qwen has a hard 60-second timeout that auto-denies, while Claude Code has no documented timeout.

- **`QwenCliEvent::SubagentPreToolUse`** maps loosely to **`ClaudeCodeEvent::PreToolUse`** -- both fire before tool execution with tool name and arguments. Critical differences: (1) Qwen's version is notification-only and cannot block or modify the tool call, while Claude's `PreToolUse` can deny, allow, ask, modify input, and inject context; (2) Qwen's version only fires for subagent tool calls, not main agent calls; (3) Qwen's version is fire-and-forget (not awaited), so errors are swallowed silently; (4) Qwen includes `subagent_id`, `name`, and `timestamp` fields that Claude's `PreToolUse` does not have.

- **`QwenCliEvent::SubagentPostToolUse`** maps loosely to **`ClaudeCodeEvent::PostToolUse`** + **`ClaudeCodeEvent::PostToolUseFailure`** combined -- Qwen uses a single event with a `success` boolean and optional `error_message`, while Claude Code splits success and failure into two distinct events. Additional differences: (1) Qwen's version is notification-only (void return), while Claude's `PostToolUse` supports `decision: "block"` and `additionalContext` injection; (2) Qwen includes `duration_ms` for timing metrics, which Claude's payload does not have; (3) Qwen only fires for subagent tool calls; (4) Claude's `PostToolUseFailure` includes `is_interrupt` to indicate user interruption, which Qwen lacks.

- **`QwenCliEvent::SubagentStop`** maps to **`ClaudeCodeEvent::SubagentStop`** -- both fire when a subagent terminates. Key differences: (1) Qwen's version is notification-only (void return), while Claude's version supports `decision: "block"` to prevent the subagent from stopping and force continuation; (2) Claude's version includes `stop_hook_active` for infinite-loop prevention, which Qwen does not need since it cannot block; (3) Qwen provides `terminate_reason` and `summary` fields, while Claude provides `agent_transcript_path`; (4) Claude provides `agent_type` for matcher filtering, while Qwen provides `name` (human-readable name rather than a type classifier).

- **`QwenCliEvent::StreamSessionStart`** maps loosely to **`ClaudeCodeEvent::SessionStart`** -- both represent session initialization. Critical differences: (1) Qwen's version is an output-only stream event with no response channel, while Claude's `SessionStart` supports context injection and `CLAUDE_ENV_FILE`; (2) Qwen's version only fires in headless mode (`--output-format stream-json`), while Claude's fires on every session start; (3) Qwen provides model and configuration metadata, while Claude provides `source` (startup/resume/clear/compact) for distinguishing how the session started.

- **`QwenCliEvent::StreamAssistantMessage`** maps loosely to **`AgenticEvent::AfterModel`** -- both represent the model's response being available. There is no direct Claude Code equivalent because Claude Code does not have `BeforeModel`/`AfterModel` events. This is a Qwen-specific event that provides the complete assistant message with content blocks and token usage, which has no parallel in Claude Code's hook system.

- **`QwenCliEvent::StreamResult`** maps loosely to **`ClaudeCodeEvent::SessionEnd`** -- both represent session termination. Critical differences: (1) Qwen's version is output-only with no response channel, while Claude's `SessionEnd` is informational but fires as a proper hook with common fields; (2) Qwen provides execution metrics (`duration_ms`, usage stats) in the result event, while Claude does not include metrics in `SessionEnd`; (3) Qwen's version only fires in headless mode.

- **No Qwen equivalent for:** `ClaudeCodeEvent::UserPromptSubmit` (no prompt interception), `ClaudeCodeEvent::PostToolUseFailure` (merged into `SubagentPostToolUse`), `ClaudeCodeEvent::Notification` (no notification hook), `ClaudeCodeEvent::SubagentStart` (no subagent start hook), `ClaudeCodeEvent::Stop` (no main agent turn-complete hook), `ClaudeCodeEvent::TeammateIdle` (no team system), `ClaudeCodeEvent::TaskCompleted` (no task system), `ClaudeCodeEvent::PreCompact` (no compaction hook).
