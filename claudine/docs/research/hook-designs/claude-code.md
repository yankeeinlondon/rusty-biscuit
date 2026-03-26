# Claude Code Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All 14 hook events supported by Claude Code.
///
/// Each variant corresponds to a named lifecycle hook that Claude Code
/// fires at a specific point during an agentic session. The variant names
/// use Claude Code's native PascalCase naming (e.g., `PreToolUse`, not
/// `before_tool`) so that matcher logic and debug output align with the
/// official documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaudeCodeEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    PermissionRequest,
    Notification,
    SubagentStart,
    SubagentStop,
    Stop,
    TeammateIdle,
    TaskCompleted,
    PreCompact,
}

impl ClaudeCodeEvent {
    /// All variants in lifecycle order.
    pub const ALL: [ClaudeCodeEvent; 14] = [
        Self::SessionStart,
        Self::UserPromptSubmit,
        Self::PreToolUse,
        Self::PermissionRequest,
        Self::PostToolUse,
        Self::PostToolUseFailure,
        Self::Notification,
        Self::SubagentStart,
        Self::SubagentStop,
        Self::Stop,
        Self::TeammateIdle,
        Self::TaskCompleted,
        Self::PreCompact,
        Self::SessionEnd,
    ];

    /// The native event name string as it appears in `hook_event_name`.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PostToolUseFailure => "PostToolUseFailure",
            Self::PermissionRequest => "PermissionRequest",
            Self::Notification => "Notification",
            Self::SubagentStart => "SubagentStart",
            Self::SubagentStop => "SubagentStop",
            Self::Stop => "Stop",
            Self::TeammateIdle => "TeammateIdle",
            Self::TaskCompleted => "TaskCompleted",
            Self::PreCompact => "PreCompact",
        }
    }

    /// Claude Code-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::SessionStart => {
                "Fires on new session, resumed session, /clear command, or post-compaction"
            }
            Self::SessionEnd => "Fires when the session terminates",
            Self::UserPromptSubmit => {
                "Fires when the user submits a prompt, before Claude processes it"
            }
            Self::PreToolUse => {
                "Fires after Claude creates tool parameters, before tool execution"
            }
            Self::PostToolUse => "Fires immediately after a tool completes successfully",
            Self::PostToolUseFailure => "Fires when a tool execution fails with an error",
            Self::PermissionRequest => {
                "Fires when a permission dialog is about to appear to the user"
            }
            Self::Notification => "Fires when Claude Code sends a notification",
            Self::SubagentStart => "Fires when a subagent is spawned via the Task tool",
            Self::SubagentStop => "Fires when a subagent finishes responding",
            Self::Stop => {
                "Fires when the main Claude agent finishes responding (not on user interrupts)"
            }
            Self::TeammateIdle => {
                "Fires when an agent team teammate is about to go idle after its turn"
            }
            Self::TaskCompleted => "Fires when a task is being marked as completed",
            Self::PreCompact => "Fires before context compaction occurs",
        }
    }

    /// Whether this event can block the action it intercepts.
    ///
    /// Blocking events support exit code 2 to prevent the action, or
    /// JSON decision fields to deny/block. Non-blocking events are
    /// informational only; exit code 2 shows stderr but cannot stop
    /// the action.
    pub fn can_block(&self) -> bool {
        matches!(
            self,
            Self::UserPromptSubmit
                | Self::PreToolUse
                | Self::PermissionRequest
                | Self::Stop
                | Self::SubagentStop
                | Self::TeammateIdle
                | Self::TaskCompleted
        )
    }

    /// Whether this event supports regex matchers for filtering.
    ///
    /// Events that return `false` fire on every occurrence; any `matcher`
    /// field is silently ignored.
    pub fn supports_matcher(&self) -> bool {
        matches!(
            self,
            Self::SessionStart
                | Self::SessionEnd
                | Self::PreToolUse
                | Self::PostToolUse
                | Self::PostToolUseFailure
                | Self::PermissionRequest
                | Self::Notification
                | Self::SubagentStart
                | Self::SubagentStop
                | Self::PreCompact
        )
    }

    /// The field in the input JSON that the matcher regex runs against.
    ///
    /// Returns `None` for events that do not support matchers.
    pub fn matcher_field(&self) -> Option<MatcherField> {
        match self {
            Self::PreToolUse
            | Self::PostToolUse
            | Self::PostToolUseFailure
            | Self::PermissionRequest => Some(MatcherField::ToolName),
            Self::SessionStart => Some(MatcherField::Source),
            Self::SessionEnd => Some(MatcherField::Reason),
            Self::Notification => Some(MatcherField::NotificationType),
            Self::SubagentStart | Self::SubagentStop => Some(MatcherField::AgentType),
            Self::PreCompact => Some(MatcherField::Trigger),
            // These events do not support matchers
            Self::UserPromptSubmit
            | Self::Stop
            | Self::TeammateIdle
            | Self::TaskCompleted => None,
        }
    }

    /// Which hook handler types are supported for this event.
    ///
    /// Most events support all three types (command, prompt, agent).
    /// `TeammateIdle` only supports command hooks.
    pub fn supported_handler_types(&self) -> &'static [HookHandlerType] {
        match self {
            Self::TeammateIdle => &[HookHandlerType::Command],
            _ => &[
                HookHandlerType::Command,
                HookHandlerType::Prompt,
                HookHandlerType::Agent,
            ],
        }
    }

    /// The decision control pattern used by this event's response.
    pub fn decision_pattern(&self) -> DecisionPattern {
        match self {
            Self::PreToolUse => DecisionPattern::HookSpecificPermission,
            Self::PermissionRequest => DecisionPattern::HookSpecificBehavior,
            Self::UserPromptSubmit
            | Self::PostToolUse
            | Self::PostToolUseFailure
            | Self::Stop
            | Self::SubagentStop => DecisionPattern::TopLevelDecision,
            Self::TeammateIdle | Self::TaskCompleted => DecisionPattern::ExitCodeOnly,
            Self::SessionStart
            | Self::SessionEnd
            | Self::Notification
            | Self::SubagentStart
            | Self::PreCompact => DecisionPattern::Informational,
        }
    }

    /// The typed payload structure this event provides on stdin.
    ///
    /// All events also include the common fields from `CommonInputFields`.
    pub fn payload_type(&self) -> PayloadType {
        match self {
            Self::SessionStart => PayloadType::SessionStart,
            Self::SessionEnd => PayloadType::SessionEnd,
            Self::UserPromptSubmit => PayloadType::UserPromptSubmit,
            Self::PreToolUse => PayloadType::PreToolUse,
            Self::PostToolUse => PayloadType::PostToolUse,
            Self::PostToolUseFailure => PayloadType::PostToolUseFailure,
            Self::PermissionRequest => PayloadType::PermissionRequest,
            Self::Notification => PayloadType::Notification,
            Self::SubagentStart => PayloadType::SubagentStart,
            Self::SubagentStop => PayloadType::SubagentStop,
            Self::Stop => PayloadType::Stop,
            Self::TeammateIdle => PayloadType::TeammateIdle,
            Self::TaskCompleted => PayloadType::TaskCompleted,
            Self::PreCompact => PayloadType::PreCompact,
        }
    }

    /// The typed response structure this event expects on stdout.
    pub fn response_type(&self) -> ResponseType {
        match self {
            Self::SessionStart => ResponseType::ContextInjection,
            Self::SessionEnd => ResponseType::CleanupOnly,
            Self::UserPromptSubmit => ResponseType::PromptDecision,
            Self::PreToolUse => ResponseType::PermissionDecision,
            Self::PostToolUse => ResponseType::PostToolFeedback,
            Self::PostToolUseFailure => ResponseType::ContextOnly,
            Self::PermissionRequest => ResponseType::PermissionBehavior,
            Self::Notification => ResponseType::ContextOnly,
            Self::SubagentStart => ResponseType::ContextOnly,
            Self::SubagentStop => ResponseType::StopDecision,
            Self::Stop => ResponseType::StopDecision,
            Self::TeammateIdle => ResponseType::ExitCodeOnly,
            Self::TaskCompleted => ResponseType::ExitCodeOnly,
            Self::PreCompact => ResponseType::Informational,
        }
    }
}

impl fmt::Display for ClaudeCodeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native event names
// ---------------------------------------------------------------------------

impl TryFrom<&str> for ClaudeCodeEvent {
    type Error = UnknownEventError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "SessionStart" => Ok(Self::SessionStart),
            "SessionEnd" => Ok(Self::SessionEnd),
            "UserPromptSubmit" => Ok(Self::UserPromptSubmit),
            "PreToolUse" => Ok(Self::PreToolUse),
            "PostToolUse" => Ok(Self::PostToolUse),
            "PostToolUseFailure" => Ok(Self::PostToolUseFailure),
            "PermissionRequest" => Ok(Self::PermissionRequest),
            "Notification" => Ok(Self::Notification),
            "SubagentStart" => Ok(Self::SubagentStart),
            "SubagentStop" => Ok(Self::SubagentStop),
            "Stop" => Ok(Self::Stop),
            "TeammateIdle" => Ok(Self::TeammateIdle),
            "TaskCompleted" => Ok(Self::TaskCompleted),
            "PreCompact" => Ok(Self::PreCompact),
            _ => Err(UnknownEventError(name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known Claude Code event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown Claude Code event: {0}")]
pub struct UnknownEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

impl From<ClaudeCodeEvent> for AgenticEvent {
    fn from(event: ClaudeCodeEvent) -> Self {
        match event {
            ClaudeCodeEvent::SessionStart => AgenticEvent::SessionStart,
            ClaudeCodeEvent::SessionEnd => AgenticEvent::SessionEnd,
            ClaudeCodeEvent::UserPromptSubmit => AgenticEvent::BeforePrompt,
            ClaudeCodeEvent::PreToolUse => AgenticEvent::BeforeTool,
            ClaudeCodeEvent::PostToolUse => AgenticEvent::AfterTool,
            ClaudeCodeEvent::PostToolUseFailure => AgenticEvent::ToolError,
            ClaudeCodeEvent::PermissionRequest => AgenticEvent::PermissionRequest,
            ClaudeCodeEvent::Notification => AgenticEvent::Notification,
            ClaudeCodeEvent::SubagentStart => AgenticEvent::SubagentStart,
            ClaudeCodeEvent::SubagentStop => AgenticEvent::SubagentStop,
            ClaudeCodeEvent::Stop => AgenticEvent::TurnComplete,
            // TeammateIdle and TaskCompleted have no unified equivalent yet;
            // they map to TurnComplete as the closest semantic match, but
            // callers should prefer working with ClaudeCodeEvent directly
            // for these Claude-specific events.
            ClaudeCodeEvent::TeammateIdle => AgenticEvent::TurnComplete,
            ClaudeCodeEvent::TaskCompleted => AgenticEvent::TurnComplete,
            ClaudeCodeEvent::PreCompact => AgenticEvent::BeforeCompact,
        }
    }
}

impl TryFrom<AgenticEvent> for ClaudeCodeEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified to Claude Code event.
    ///
    /// Events that map 1:1 succeed. Events that have no Claude Code
    /// equivalent (e.g., `BeforeModel`, `AfterModel`) return `Err`.
    /// Events where multiple Claude Code events map to the same unified
    /// event (e.g., `TurnComplete` -> `Stop` vs `TeammateIdle`) return
    /// the primary mapping.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::SessionStart => Ok(ClaudeCodeEvent::SessionStart),
            AgenticEvent::SessionEnd => Ok(ClaudeCodeEvent::SessionEnd),
            AgenticEvent::BeforePrompt => Ok(ClaudeCodeEvent::UserPromptSubmit),
            AgenticEvent::BeforeTool => Ok(ClaudeCodeEvent::PreToolUse),
            AgenticEvent::AfterTool => Ok(ClaudeCodeEvent::PostToolUse),
            AgenticEvent::ToolError => Ok(ClaudeCodeEvent::PostToolUseFailure),
            AgenticEvent::PermissionRequest => Ok(ClaudeCodeEvent::PermissionRequest),
            AgenticEvent::Notification => Ok(ClaudeCodeEvent::Notification),
            AgenticEvent::SubagentStart => Ok(ClaudeCodeEvent::SubagentStart),
            AgenticEvent::SubagentStop => Ok(ClaudeCodeEvent::SubagentStop),
            AgenticEvent::TurnComplete => Ok(ClaudeCodeEvent::Stop),
            AgenticEvent::BeforeCompact => Ok(ClaudeCodeEvent::PreCompact),
            AgenticEvent::BeforeModel
            | AgenticEvent::AfterModel
            | AgenticEvent::TurnError
            | AgenticEvent::HumanInTheLoop => {
                Err("Claude Code does not support this event")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// The field that the matcher regex runs against for a given event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatcherField {
    /// `tool_name` -- PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest
    ToolName,
    /// `source` -- SessionStart (startup, resume, clear, compact)
    Source,
    /// `reason` -- SessionEnd (clear, logout, prompt_input_exit, etc.)
    Reason,
    /// `notification_type` -- Notification (permission_prompt, idle_prompt, etc.)
    NotificationType,
    /// `agent_type` -- SubagentStart, SubagentStop (Bash, Explore, Plan, etc.)
    AgentType,
    /// `trigger` -- PreCompact (manual, auto)
    Trigger,
}

/// Hook handler types supported by Claude Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookHandlerType {
    /// Shell command. Receives JSON on stdin, returns JSON/text on stdout.
    Command,
    /// Single LLM call. `$ARGUMENTS` is replaced with input JSON.
    Prompt,
    /// Multi-turn agent with tool access (up to 50 turns).
    Agent,
}

/// How the event communicates decisions back to Claude Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionPattern {
    /// Uses `hookSpecificOutput.permissionDecision` (allow/deny/ask) and
    /// `hookSpecificOutput.permissionDecisionReason`. PreToolUse only.
    HookSpecificPermission,
    /// Uses `hookSpecificOutput.decision.behavior` (allow/deny).
    /// PermissionRequest only.
    HookSpecificBehavior,
    /// Uses top-level `decision: "block"` and `reason`.
    /// UserPromptSubmit, PostToolUse, PostToolUseFailure, Stop, SubagentStop.
    TopLevelDecision,
    /// Exit code 2 is the only blocking mechanism; JSON decision fields
    /// have no effect. TeammateIdle, TaskCompleted.
    ExitCodeOnly,
    /// Cannot block. Response is informational or context-injection only.
    /// SessionStart, SessionEnd, Notification, SubagentStart, PreCompact.
    Informational,
}

/// Discriminant for the event-specific input payload type.
///
/// Use this with `ClaudeCodeEvent::payload_type()` to determine which
/// payload struct to deserialize the stdin JSON into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    PermissionRequest,
    Notification,
    SubagentStart,
    SubagentStop,
    Stop,
    TeammateIdle,
    TaskCompleted,
    PreCompact,
}

/// Discriminant for the event-specific response type.
///
/// Use this with `ClaudeCodeEvent::response_type()` to determine which
/// response struct to serialize for stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    /// SessionStart: can inject context, use CLAUDE_ENV_FILE.
    ContextInjection,
    /// SessionEnd: cleanup only, cannot prevent termination.
    CleanupOnly,
    /// UserPromptSubmit: can block with decision, add context.
    PromptDecision,
    /// PreToolUse: three-level permission (allow/deny/ask), updatedInput.
    PermissionDecision,
    /// PostToolUse: feedback only, decision "block" is advisory (can't undo).
    PostToolFeedback,
    /// PostToolUseFailure, Notification, SubagentStart: additionalContext only.
    ContextOnly,
    /// PermissionRequest: allow/deny behavior, updatedInput, updatedPermissions.
    PermissionBehavior,
    /// Stop, SubagentStop: decision "block" + reason continues conversation.
    StopDecision,
    /// TeammateIdle, TaskCompleted: exit code 2 blocks, stderr is feedback.
    ExitCodeOnly,
    /// PreCompact: information only, no output processed.
    Informational,
}

// ---------------------------------------------------------------------------
// Common input fields (present on ALL events)
// ---------------------------------------------------------------------------

/// Fields present in the JSON input of every Claude Code hook event.
///
/// All event-specific payload structs include these fields via
/// `#[serde(flatten)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonInputFields {
    /// Current session identifier.
    pub session_id: String,

    /// Path to the conversation JSONL transcript.
    pub transcript_path: String,

    /// Current working directory when the hook was invoked.
    pub cwd: String,

    /// Current permission mode.
    pub permission_mode: PermissionMode,

    /// Name of the event that fired (matches `ClaudeCodeEvent::native_name`).
    pub hook_event_name: String,
}

/// Claude Code permission modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    Default,
    Plan,
    AcceptEdits,
    DontAsk,
    BypassPermissions,
}

// ---------------------------------------------------------------------------
// Event-specific input payloads
// ---------------------------------------------------------------------------

/// SessionStart input. Includes `source`, `model`, optional `agent_type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// How the session started.
    pub source: SessionStartSource,

    /// Model identifier being used.
    pub model: String,

    /// Present only when started with `claude --agent <name>`.
    #[serde(default)]
    pub agent_type: Option<String>,
}

/// The `source` field values for SessionStart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStartSource {
    Startup,
    Resume,
    Clear,
    Compact,
}

/// SessionEnd input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Why the session ended.
    pub reason: SessionEndReason,
}

/// The `reason` field values for SessionEnd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Clear,
    Logout,
    PromptInputExit,
    BypassPermissionsDisabled,
    Other,
}

/// UserPromptSubmit input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// The text the user submitted.
    pub prompt: String,
}

/// PreToolUse input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUsePayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Name of the tool about to execute.
    pub tool_name: String,

    /// Unique identifier for this tool call.
    pub tool_use_id: String,

    /// Tool-specific arguments (schema varies by tool).
    pub tool_input: Value,
}

/// PostToolUse input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUsePayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Name of the tool that executed.
    pub tool_name: String,

    /// Unique identifier for this tool call.
    pub tool_use_id: String,

    /// Arguments that were sent to the tool.
    pub tool_input: Value,

    /// Result returned by the tool (schema varies by tool).
    pub tool_response: Value,
}

/// PostToolUseFailure input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseFailurePayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Name of the tool that failed.
    pub tool_name: String,

    /// Unique identifier for this tool call.
    pub tool_use_id: String,

    /// Arguments that were sent to the tool.
    pub tool_input: Value,

    /// Description of what went wrong.
    pub error: String,

    /// Whether the failure was caused by user interruption.
    #[serde(default)]
    pub is_interrupt: bool,
}

/// PermissionRequest input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// The tool requesting permission.
    pub tool_name: String,

    /// Same structure as PreToolUse (no `tool_use_id`).
    pub tool_input: Value,

    /// "Always allow" options the user would see in the dialog.
    #[serde(default)]
    pub permission_suggestions: Vec<PermissionSuggestion>,
}

/// A single permission suggestion entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    /// Suggestion type (e.g., "toolAlwaysAllow").
    #[serde(rename = "type")]
    pub suggestion_type: String,

    /// The tool this suggestion applies to.
    #[serde(default)]
    pub tool: Option<String>,
}

/// Notification input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Notification text.
    pub message: String,

    /// Notification title.
    #[serde(default)]
    pub title: Option<String>,

    /// Which type fired (used for matcher filtering).
    pub notification_type: NotificationType,
}

/// Known notification types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    PermissionPrompt,
    IdlePrompt,
    AuthSuccess,
    ElicitationDialog,
}

/// SubagentStart input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStartPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Unique identifier for the subagent.
    pub agent_id: String,

    /// Agent name (used for matcher filtering).
    pub agent_type: String,
}

/// SubagentStop input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// `true` when subagent is already continuing due to a stop hook.
    /// **Must check this to prevent infinite loops.**
    pub stop_hook_active: bool,

    /// Unique identifier for the subagent.
    pub agent_id: String,

    /// Agent name (used for matcher filtering).
    pub agent_type: String,

    /// Path to the subagent's own transcript.
    pub agent_transcript_path: String,
}

/// Stop input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// `true` when Claude is already continuing as a result of a stop hook.
    /// **Must check this to prevent infinite loops.**
    pub stop_hook_active: bool,
}

/// TeammateIdle input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeammateIdlePayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Name of the teammate about to go idle.
    pub teammate_name: String,

    /// Name of the team.
    pub team_name: String,
}

/// TaskCompleted input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletedPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// Identifier of the task being completed.
    pub task_id: String,

    /// Title of the task.
    pub task_subject: String,

    /// Detailed description of the task.
    #[serde(default)]
    pub task_description: Option<String>,

    /// Name of the teammate completing the task.
    #[serde(default)]
    pub teammate_name: Option<String>,

    /// Name of the team.
    #[serde(default)]
    pub team_name: Option<String>,
}

/// PreCompact input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactPayload {
    #[serde(flatten)]
    pub common: CommonInputFields,

    /// How compaction was triggered.
    pub trigger: CompactTrigger,

    /// For `manual`, contains what the user passed to `/compact`.
    /// For `auto`, empty string.
    #[serde(default)]
    pub custom_instructions: String,
}

/// CompactTrigger values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactTrigger {
    Manual,
    Auto,
}

// ---------------------------------------------------------------------------
// Unified input payload enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any Claude Code event's input payload.
///
/// Constructed by deserializing the hook's stdin JSON based on the
/// `hook_event_name` field. Gives callers access to the strongly-typed
/// event-specific fields without manual JSON wrangling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum ClaudeCodeInput {
    SessionStart(SessionStartPayload),
    SessionEnd(SessionEndPayload),
    UserPromptSubmit(UserPromptSubmitPayload),
    PreToolUse(PreToolUsePayload),
    PostToolUse(PostToolUsePayload),
    PostToolUseFailure(PostToolUseFailurePayload),
    PermissionRequest(PermissionRequestPayload),
    Notification(NotificationPayload),
    SubagentStart(SubagentStartPayload),
    SubagentStop(SubagentStopPayload),
    Stop(StopPayload),
    TeammateIdle(TeammateIdlePayload),
    TaskCompleted(TaskCompletedPayload),
    PreCompact(PreCompactPayload),
}

impl ClaudeCodeInput {
    /// Returns the event variant for this input payload.
    pub fn event(&self) -> ClaudeCodeEvent {
        match self {
            Self::SessionStart(_) => ClaudeCodeEvent::SessionStart,
            Self::SessionEnd(_) => ClaudeCodeEvent::SessionEnd,
            Self::UserPromptSubmit(_) => ClaudeCodeEvent::UserPromptSubmit,
            Self::PreToolUse(_) => ClaudeCodeEvent::PreToolUse,
            Self::PostToolUse(_) => ClaudeCodeEvent::PostToolUse,
            Self::PostToolUseFailure(_) => ClaudeCodeEvent::PostToolUseFailure,
            Self::PermissionRequest(_) => ClaudeCodeEvent::PermissionRequest,
            Self::Notification(_) => ClaudeCodeEvent::Notification,
            Self::SubagentStart(_) => ClaudeCodeEvent::SubagentStart,
            Self::SubagentStop(_) => ClaudeCodeEvent::SubagentStop,
            Self::Stop(_) => ClaudeCodeEvent::Stop,
            Self::TeammateIdle(_) => ClaudeCodeEvent::TeammateIdle,
            Self::TaskCompleted(_) => ClaudeCodeEvent::TaskCompleted,
            Self::PreCompact(_) => ClaudeCodeEvent::PreCompact,
        }
    }

    /// Returns the common input fields shared by all events.
    pub fn common(&self) -> &CommonInputFields {
        match self {
            Self::SessionStart(p) => &p.common,
            Self::SessionEnd(p) => &p.common,
            Self::UserPromptSubmit(p) => &p.common,
            Self::PreToolUse(p) => &p.common,
            Self::PostToolUse(p) => &p.common,
            Self::PostToolUseFailure(p) => &p.common,
            Self::PermissionRequest(p) => &p.common,
            Self::Notification(p) => &p.common,
            Self::SubagentStart(p) => &p.common,
            Self::SubagentStop(p) => &p.common,
            Self::Stop(p) => &p.common,
            Self::TeammateIdle(p) => &p.common,
            Self::TaskCompleted(p) => &p.common,
            Self::PreCompact(p) => &p.common,
        }
    }

    /// Returns the tool name if this is a tool-related event.
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::PreToolUse(p) => Some(&p.tool_name),
            Self::PostToolUse(p) => Some(&p.tool_name),
            Self::PostToolUseFailure(p) => Some(&p.tool_name),
            Self::PermissionRequest(p) => Some(&p.tool_name),
            _ => None,
        }
    }

    /// Returns the matcher value -- the string the regex matcher should
    /// test against for this event.
    pub fn matcher_value(&self) -> Option<&str> {
        match self {
            Self::PreToolUse(p) => Some(&p.tool_name),
            Self::PostToolUse(p) => Some(&p.tool_name),
            Self::PostToolUseFailure(p) => Some(&p.tool_name),
            Self::PermissionRequest(p) => Some(&p.tool_name),
            Self::SessionStart(p) => Some(p.source.as_str()),
            Self::SessionEnd(p) => Some(p.reason.as_str()),
            Self::Notification(p) => Some(p.notification_type.as_str()),
            Self::SubagentStart(p) => Some(&p.agent_type),
            Self::SubagentStop(p) => Some(&p.agent_type),
            Self::PreCompact(p) => Some(p.trigger.as_str()),
            // Events with no matcher support
            Self::UserPromptSubmit(_)
            | Self::Stop(_)
            | Self::TeammateIdle(_)
            | Self::TaskCompleted(_) => None,
        }
    }

    /// Whether the stop_hook_active flag is set (Stop and SubagentStop only).
    ///
    /// Returns `false` for all other events. Callers should check this
    /// before blocking Stop/SubagentStop events to prevent infinite loops.
    pub fn is_stop_hook_active(&self) -> bool {
        match self {
            Self::Stop(p) => p.stop_hook_active,
            Self::SubagentStop(p) => p.stop_hook_active,
            _ => false,
        }
    }
}

// as_str() helpers for enum types used in matcher_value()

impl SessionStartSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Resume => "resume",
            Self::Clear => "clear",
            Self::Compact => "compact",
        }
    }
}

impl SessionEndReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Logout => "logout",
            Self::PromptInputExit => "prompt_input_exit",
            Self::BypassPermissionsDisabled => "bypass_permissions_disabled",
            Self::Other => "other",
        }
    }
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PermissionPrompt => "permission_prompt",
            Self::IdlePrompt => "idle_prompt",
            Self::AuthSuccess => "auth_success",
            Self::ElicitationDialog => "elicitation_dialog",
        }
    }
}

impl CompactTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Auto => "auto",
        }
    }
}

// ---------------------------------------------------------------------------
// Response types (what hooks return on stdout)
// ---------------------------------------------------------------------------

/// Common response fields supported by all events on exit 0.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonResponseFields {
    /// Setting `false` stops Claude entirely, regardless of event-specific
    /// decision fields. Takes precedence over everything.
    #[serde(default, rename = "continue", skip_serializing_if = "Option::is_none")]
    pub should_continue: Option<bool>,

    /// Message shown to user when `continue` is false.
    /// Not shown to Claude.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// When true, suppresses system messages about this hook's changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,

    /// Warning shown to user (not added to Claude's context).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
}

/// PreToolUse response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseOutput {
    #[serde(flatten)]
    pub common: CommonResponseFields,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<PreToolUseHookOutput>,
}

/// The `hookSpecificOutput` for PreToolUse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookOutput {
    /// Must be "PreToolUse".
    pub hook_event_name: String,

    /// `"allow"` bypasses permission, `"deny"` prevents tool call,
    /// `"ask"` shows the permission dialog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<PreToolUseDecision>,

    /// For `allow`/`ask`: shown to user, not Claude.
    /// For `deny`: shown to Claude.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,

    /// Modifies tool input before execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<Value>,

    /// String added to Claude's context before tool executes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Three-level permission decision for PreToolUse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreToolUseDecision {
    /// Bypass permission system entirely.
    Allow,
    /// Prevent the tool call.
    Deny,
    /// Show user the permission dialog.
    Ask,
}

/// PermissionRequest response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestOutput {
    #[serde(flatten)]
    pub common: CommonResponseFields,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<PermissionRequestHookOutput>,
}

/// The `hookSpecificOutput` for PermissionRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestHookOutput {
    /// Must be "PermissionRequest".
    pub hook_event_name: String,

    /// The permission decision.
    pub decision: PermissionBehaviorDecision,
}

/// The nested `decision` object in PermissionRequest response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionBehaviorDecision {
    /// `"allow"` grants permission, `"deny"` denies it.
    pub behavior: PermissionBehavior,

    /// For `allow` only: modifies tool input before execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<Value>,

    /// For `allow` only: applies permission rule updates (equivalent to
    /// user selecting "always allow").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_permissions: Option<Vec<Value>>,

    /// For `deny` only: tells Claude why permission was denied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// For `deny` only: if `true`, stops Claude.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<bool>,
}

/// Permission behavior values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    Allow,
    Deny,
}

/// Response for events using top-level decision pattern:
/// UserPromptSubmit, PostToolUse, PostToolUseFailure, Stop, SubagentStop.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopLevelDecisionOutput {
    #[serde(flatten)]
    pub common: CommonResponseFields,

    /// `"block"` prevents the action. Omit to allow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    /// Explanation shown to Claude (Stop/SubagentStop) or user
    /// (UserPromptSubmit) when blocking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Event-specific hook output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<GenericHookOutput>,
}

/// Generic hook-specific output for events that use additionalContext
/// and/or other event-specific fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericHookOutput {
    /// Must match the event name.
    pub hook_event_name: String,

    /// String added to Claude's context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,

    /// For PostToolUse with MCP tools: replaces tool output Claude sees.
    #[serde(
        default,
        rename = "updatedMCPToolOutput",
        skip_serializing_if = "Option::is_none"
    )]
    pub updated_mcp_tool_output: Option<String>,
}

/// Response for context-injection events (SessionStart).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextInjectionOutput {
    #[serde(flatten)]
    pub common: CommonResponseFields,

    /// Context string injected into the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

// ---------------------------------------------------------------------------
// Exit code semantics
// ---------------------------------------------------------------------------

/// Exit code semantics for Claude Code hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Success. Action proceeds. JSON output is processed.
    Success = 0,
    /// Blocking error. Action prevented. Stderr becomes feedback.
    /// JSON on stdout is ignored.
    Block = 2,
    // Any other code: non-blocking error. Stderr shown in verbose mode.
}

impl ExitCode {
    /// Interpret a raw process exit code.
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Success,
            2 => Self::Block,
            _ => Self::Success, // Non-blocking error, treat as success path
        }
    }

    /// Whether JSON output should be parsed from stdout.
    pub fn should_parse_json(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Whether this exit code blocks the action (for blocking events).
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Block)
    }
}

// ---------------------------------------------------------------------------
// Environment variables available in hooks
// ---------------------------------------------------------------------------

/// Environment variables Claude Code sets for hook processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEnvVar {
    /// `CLAUDE_PROJECT_DIR` -- absolute path to project root (all hooks).
    ProjectDir,
    /// `CLAUDE_PLUGIN_ROOT` -- plugin directory (plugin hooks only).
    PluginRoot,
    /// `CLAUDE_ENV_FILE` -- file path for persisting env vars to subsequent
    /// Bash calls (SessionStart only).
    EnvFile,
    /// `CLAUDE_CODE_REMOTE` -- `"true"` in web/remote mode (all hooks).
    CodeRemote,
}

impl HookEnvVar {
    pub fn var_name(&self) -> &'static str {
        match self {
            Self::ProjectDir => "CLAUDE_PROJECT_DIR",
            Self::PluginRoot => "CLAUDE_PLUGIN_ROOT",
            Self::EnvFile => "CLAUDE_ENV_FILE",
            Self::CodeRemote => "CLAUDE_CODE_REMOTE",
        }
    }
}

// ---------------------------------------------------------------------------
// Tool input shapes (for PreToolUse / PostToolUse / PermissionRequest)
// ---------------------------------------------------------------------------

/// Known built-in tool names and their expected input schemas.
///
/// MCP tools follow the pattern `mcp__<server>__<tool>` and have
/// free-form input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinTool {
    Bash,
    Write,
    Edit,
    Read,
    Glob,
    Grep,
    WebFetch,
    WebSearch,
    Task,
}

/// Strongly-typed tool input for the Bash tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolInput {
    pub command: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub run_in_background: Option<bool>,
}

/// Strongly-typed tool input for the Write tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteToolInput {
    pub file_path: String,
    pub content: String,
}

/// Strongly-typed tool input for the Edit tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditToolInput {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: Option<bool>,
}

/// Strongly-typed tool input for the Read tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadToolInput {
    pub file_path: String,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

/// Strongly-typed tool input for the Glob tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobToolInput {
    pub pattern: String,
    #[serde(default)]
    pub path: Option<String>,
}

/// Strongly-typed tool input for the Grep tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepToolInput {
    pub pattern: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub glob: Option<String>,
    #[serde(default)]
    pub output_mode: Option<String>,
    #[serde(default, rename = "-i")]
    pub case_insensitive: Option<bool>,
    #[serde(default)]
    pub multiline: Option<bool>,
}

/// Strongly-typed tool input for the WebFetch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchToolInput {
    pub url: String,
    pub prompt: String,
}

/// Strongly-typed tool input for the WebSearch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchToolInput {
    pub query: String,
    #[serde(default)]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(default)]
    pub blocked_domains: Option<Vec<String>>,
}

/// Strongly-typed tool input for the Task tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToolInput {
    pub prompt: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub subagent_type: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Parsed tool input, dispatched by tool name.
///
/// Provides strongly-typed access to tool arguments when the tool name
/// is known. Falls back to `Other` with raw JSON for MCP tools and any
/// future built-in tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypedToolInput {
    Bash(BashToolInput),
    Write(WriteToolInput),
    Edit(EditToolInput),
    Read(ReadToolInput),
    Glob(GlobToolInput),
    Grep(GrepToolInput),
    WebFetch(WebFetchToolInput),
    WebSearch(WebSearchToolInput),
    Task(TaskToolInput),
    Other(Value),
}

impl TypedToolInput {
    /// Attempt to parse raw JSON tool_input into the appropriate typed
    /// variant based on the tool name.
    pub fn from_tool_name(tool_name: &str, input: &Value) -> Self {
        match tool_name {
            "Bash" => serde_json::from_value(input.clone())
                .map(TypedToolInput::Bash)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "Write" => serde_json::from_value(input.clone())
                .map(TypedToolInput::Write)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "Edit" => serde_json::from_value(input.clone())
                .map(TypedToolInput::Edit)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "Read" => serde_json::from_value(input.clone())
                .map(TypedToolInput::Read)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "Glob" => serde_json::from_value(input.clone())
                .map(TypedToolInput::Glob)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "Grep" => serde_json::from_value(input.clone())
                .map(TypedToolInput::Grep)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "WebFetch" => serde_json::from_value(input.clone())
                .map(TypedToolInput::WebFetch)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "WebSearch" => serde_json::from_value(input.clone())
                .map(TypedToolInput::WebSearch)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            "Task" => serde_json::from_value(input.clone())
                .map(TypedToolInput::Task)
                .unwrap_or_else(|_| TypedToolInput::Other(input.clone())),
            _ => TypedToolInput::Other(input.clone()),
        }
    }
}
```

## Design Considerations

- **Native naming preserved.** The enum variants use Claude Code's exact PascalCase event names (`PreToolUse`, `PostToolUseFailure`, `SubagentStop`, etc.) rather than normalized names. This eliminates a translation layer in matcher logic, debug output, and serialization. The mapping to `AgenticEvent` is handled explicitly via `From`/`TryFrom` implementations.

- **All 14 events captured.** The design accounts for all 14 hook events documented in Claude Code, including the two newer team-oriented events (`TeammateIdle` and `TaskCompleted`) that have no direct equivalent in the existing `AgenticEvent` unified enum. These are mapped to `TurnComplete` as a fallback but flagged with a comment that callers should prefer the provider-specific type.

- **Rich metadata on each variant.** Rather than a single `description()` method returning a string, the enum exposes six distinct metadata accessors: `description()`, `can_block()`, `supports_matcher()`, `matcher_field()`, `supported_handler_types()`, and `decision_pattern()`. This avoids callers needing to hardcode knowledge about event capabilities.

- **Strongly-typed payloads.** Each event has a dedicated payload struct with fields matching the documented JSON schema. The `ClaudeCodeInput` tagged enum (discriminated on `hook_event_name`) allows deserialization from raw stdin JSON into the correct typed variant in a single step. Common fields are shared via `#[serde(flatten)]` on `CommonInputFields`.

- **Strongly-typed responses.** Response types are differentiated by event category. `PreToolUse` uses a three-level permission decision (`allow`/`deny`/`ask`), `PermissionRequest` uses a behavior decision with nested fields, and most blocking events use a top-level `decision`/`reason` pattern. `TeammateIdle` and `TaskCompleted` use exit codes only. This is captured via the `DecisionPattern` enum and concrete response structs.

- **Typed tool inputs.** The `TypedToolInput` enum provides strongly-typed access to all 9 built-in Claude Code tools (Bash, Write, Edit, Read, Glob, Grep, WebFetch, WebSearch, Task). The `from_tool_name()` constructor dispatches by tool name and falls back to `Other(Value)` for MCP tools and any unrecognized tools, making the design forward-compatible.

- **Exit code semantics.** The `ExitCode` type encodes Claude Code's three-way exit code protocol: 0 (success, parse JSON), 2 (block, use stderr), other (non-blocking error). The `should_parse_json()` and `is_blocking()` methods provide clean dispatch for hook runners.

- **Gotcha awareness baked into the API.** The `is_stop_hook_active()` method on `ClaudeCodeInput` makes the infinite-loop prevention pattern discoverable. The `supports_matcher()` method prevents wasted configuration on events that silently ignore matchers. The `supported_handler_types()` method prevents misconfiguring `TeammateIdle` with prompt/agent handlers.

- **Matcher system modeled explicitly.** The `MatcherField` enum captures which JSON field each event's matcher regex runs against, and `ClaudeCodeInput::matcher_value()` extracts the concrete value for regex matching. This allows the dispatch layer to be generic over the matcher field without per-event special casing.

- **`PermissionMode` as enum.** The permission mode string (`"default"`, `"plan"`, `"acceptEdits"`, `"dontAsk"`, `"bypassPermissions"`) is captured as a proper enum rather than a raw string, enabling exhaustive matching in hook logic.

- **Environment variables documented in-type.** The `HookEnvVar` enum enumerates the four environment variables Claude Code sets for hook processes, including the `CLAUDE_ENV_FILE` variable that is only available in `SessionStart` hooks.

- **Forward compatibility.** Serde's `skip_serializing_if` annotations keep serialized output clean. The `TypedToolInput::Other(Value)` fallback handles unknown tools gracefully. The `CommonResponseFields` struct with `Option` fields means hooks only need to set the fields they care about.

- **`From<ClaudeCodeEvent> for AgenticEvent` is total.** Every Claude Code event maps to some unified event (even if lossy for `TeammateIdle`/`TaskCompleted`). The reverse `TryFrom<AgenticEvent>` is partial, returning `Err` for events Claude Code does not support (`BeforeModel`, `AfterModel`, `TurnError`, `HumanInTheLoop`).
