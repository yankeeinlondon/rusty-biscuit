# Roo Code Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All events emitted by Roo Code across its three event surfaces.
///
/// Roo Code emits events through three independent surfaces:
/// 1. CLI programmatic events (`ExtensionClient` / `ClientEventMap`)
/// 2. CLI structured output events (`--output-format stream-json`)
/// 3. VS Code extension API events (`RooCodeAPI` / `RooCodeEventName`)
///
/// Unlike Claude Code, Roo Code events are **observational only** -- listener
/// return values are ignored. Flow control requires calling explicit client
/// methods (`approve()`, `reject()`, `respond()`, etc.).
///
/// Variant names use Roo Code's native camelCase event names converted to
/// PascalCase to follow Rust conventions and align with the source TypeScript
/// definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RooCodeEvent {
    // -- CLI programmatic events (Surface 1) --
    /// Agent loop state transition.
    StateChange,
    /// New message arrived in conversation.
    Message,
    /// Existing message modified (e.g., streaming partial -> complete).
    MessageUpdated,
    /// Agent needs user input to proceed.
    WaitingForInput,
    /// Agent resumed execution after waiting/idle.
    ResumedRunning,
    /// LLM response streaming began.
    StreamingStarted,
    /// LLM response streaming ended.
    StreamingEnded,
    /// Task completed (subset of idle states).
    TaskCompleted,
    /// Current task cleared.
    TaskCleared,
    /// Operational mode changed.
    ModeChanged,
    /// Processing error occurred.
    Error,

    // -- CLI structured output events (Surface 2) --
    /// System/lifecycle message (`type: "system"`).
    SystemOutput,
    /// Assistant text response chunk (`type: "assistant"`).
    AssistantOutput,
    /// Echoed user input (`type: "user"`).
    UserOutput,
    /// Reasoning/thinking content (`type: "thinking"`).
    ThinkingOutput,
    /// Tool invocation (`type: "tool_use"`).
    ToolUseOutput,
    /// Tool execution result (`type: "tool_result"`).
    ToolResultOutput,
    /// Error notification in output stream (`type: "error"`).
    ErrorOutput,
    /// Final task result (`type: "result"`).
    ResultOutput,

    // -- VS Code extension API events (Surface 3) --
    /// New task created.
    TaskCreated,
    /// Task execution began.
    TaskStarted,
    /// Task cancelled.
    TaskAborted,
    /// Task gained UI focus.
    TaskFocused,
    /// Task lost UI focus.
    TaskUnfocused,
    /// Task is actively running (state notification).
    TaskActive,
    /// Task requires user interaction (state notification).
    TaskInteractive,
    /// Task is paused but resumable (state notification).
    TaskResumable,
    /// Task is idle (state notification).
    TaskIdle,
    /// Parent task paused for subtask.
    TaskPaused,
    /// Parent task resumed after subtask.
    TaskUnpaused,
    /// Subtask spawned.
    TaskSpawned,
    /// Task delegated to subtask.
    TaskDelegated,
    /// Delegation finished.
    TaskDelegationCompleted,
    /// Delegation control returned to parent.
    TaskDelegationResumed,
    /// Mode switched within a task.
    TaskModeSwitched,
    /// User responded to an ask.
    TaskAskResponded,
    /// User sent a message.
    TaskUserMessage,
    /// Message queue changed.
    QueuedMessagesUpdated,
    /// Token consumption updated.
    TaskTokenUsageUpdated,
    /// Tool execution failed (extension-level).
    TaskToolFailed,
    /// API provider profile switched.
    ProviderProfileChanged,
    /// Response to commands query.
    CommandsResponse,
    /// Response to modes query.
    ModesResponse,
    /// Response to models query.
    ModelsResponse,
    /// Evaluation passed.
    EvalPass,
    /// Evaluation failed.
    EvalFail,
}

impl RooCodeEvent {
    /// All variants in logical grouping order (CLI programmatic, CLI output,
    /// VS Code extension API).
    pub const ALL: [RooCodeEvent; 42] = [
        // Surface 1: CLI programmatic
        Self::StateChange,
        Self::Message,
        Self::MessageUpdated,
        Self::WaitingForInput,
        Self::ResumedRunning,
        Self::StreamingStarted,
        Self::StreamingEnded,
        Self::TaskCompleted,
        Self::TaskCleared,
        Self::ModeChanged,
        Self::Error,
        // Surface 2: CLI structured output
        Self::SystemOutput,
        Self::AssistantOutput,
        Self::UserOutput,
        Self::ThinkingOutput,
        Self::ToolUseOutput,
        Self::ToolResultOutput,
        Self::ErrorOutput,
        Self::ResultOutput,
        // Surface 3: VS Code extension API
        Self::TaskCreated,
        Self::TaskStarted,
        Self::TaskAborted,
        Self::TaskFocused,
        Self::TaskUnfocused,
        Self::TaskActive,
        Self::TaskInteractive,
        Self::TaskResumable,
        Self::TaskIdle,
        Self::TaskPaused,
        Self::TaskUnpaused,
        Self::TaskSpawned,
        Self::TaskDelegated,
        Self::TaskDelegationCompleted,
        Self::TaskDelegationResumed,
        Self::TaskModeSwitched,
        Self::TaskAskResponded,
        Self::TaskUserMessage,
        Self::QueuedMessagesUpdated,
        Self::TaskTokenUsageUpdated,
        Self::TaskToolFailed,
        Self::ProviderProfileChanged,
        // Query/response and eval events omitted from ALL since they are
        // not lifecycle events -- include them via ALL_WITH_QUERIES if needed.
    ];

    /// All variants including query-response and eval events.
    pub const ALL_WITH_QUERIES: [RooCodeEvent; 47] = [
        Self::StateChange,
        Self::Message,
        Self::MessageUpdated,
        Self::WaitingForInput,
        Self::ResumedRunning,
        Self::StreamingStarted,
        Self::StreamingEnded,
        Self::TaskCompleted,
        Self::TaskCleared,
        Self::ModeChanged,
        Self::Error,
        Self::SystemOutput,
        Self::AssistantOutput,
        Self::UserOutput,
        Self::ThinkingOutput,
        Self::ToolUseOutput,
        Self::ToolResultOutput,
        Self::ErrorOutput,
        Self::ResultOutput,
        Self::TaskCreated,
        Self::TaskStarted,
        Self::TaskAborted,
        Self::TaskFocused,
        Self::TaskUnfocused,
        Self::TaskActive,
        Self::TaskInteractive,
        Self::TaskResumable,
        Self::TaskIdle,
        Self::TaskPaused,
        Self::TaskUnpaused,
        Self::TaskSpawned,
        Self::TaskDelegated,
        Self::TaskDelegationCompleted,
        Self::TaskDelegationResumed,
        Self::TaskModeSwitched,
        Self::TaskAskResponded,
        Self::TaskUserMessage,
        Self::QueuedMessagesUpdated,
        Self::TaskTokenUsageUpdated,
        Self::TaskToolFailed,
        Self::ProviderProfileChanged,
        Self::CommandsResponse,
        Self::ModesResponse,
        Self::ModelsResponse,
        Self::EvalPass,
        Self::EvalFail,
    ];

    /// The native event name as it appears in Roo Code's TypeScript source.
    pub fn native_name(&self) -> &'static str {
        match self {
            // Surface 1
            Self::StateChange => "stateChange",
            Self::Message => "message",
            Self::MessageUpdated => "messageUpdated",
            Self::WaitingForInput => "waitingForInput",
            Self::ResumedRunning => "resumedRunning",
            Self::StreamingStarted => "streamingStarted",
            Self::StreamingEnded => "streamingEnded",
            Self::TaskCompleted => "taskCompleted",
            Self::TaskCleared => "taskCleared",
            Self::ModeChanged => "modeChanged",
            Self::Error => "error",
            // Surface 2
            Self::SystemOutput => "system",
            Self::AssistantOutput => "assistant",
            Self::UserOutput => "user",
            Self::ThinkingOutput => "thinking",
            Self::ToolUseOutput => "tool_use",
            Self::ToolResultOutput => "tool_result",
            Self::ErrorOutput => "error",
            Self::ResultOutput => "result",
            // Surface 3
            Self::TaskCreated => "taskCreated",
            Self::TaskStarted => "taskStarted",
            Self::TaskAborted => "taskAborted",
            Self::TaskFocused => "taskFocused",
            Self::TaskUnfocused => "taskUnfocused",
            Self::TaskActive => "taskActive",
            Self::TaskInteractive => "taskInteractive",
            Self::TaskResumable => "taskResumable",
            Self::TaskIdle => "taskIdle",
            Self::TaskPaused => "taskPaused",
            Self::TaskUnpaused => "taskUnpaused",
            Self::TaskSpawned => "taskSpawned",
            Self::TaskDelegated => "taskDelegated",
            Self::TaskDelegationCompleted => "taskDelegationCompleted",
            Self::TaskDelegationResumed => "taskDelegationResumed",
            Self::TaskModeSwitched => "taskModeSwitched",
            Self::TaskAskResponded => "taskAskResponded",
            Self::TaskUserMessage => "taskUserMessage",
            Self::QueuedMessagesUpdated => "queuedMessagesUpdated",
            Self::TaskTokenUsageUpdated => "taskTokenUsageUpdated",
            Self::TaskToolFailed => "taskToolFailed",
            Self::ProviderProfileChanged => "providerProfileChanged",
            Self::CommandsResponse => "commandsResponse",
            Self::ModesResponse => "modesResponse",
            Self::ModelsResponse => "modelsResponse",
            Self::EvalPass => "evalPass",
            Self::EvalFail => "evalFail",
        }
    }

    /// Roo Code-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::StateChange => {
                "Fires on any agent loop state transition; compares previous \
                 and current AgentStateInfo"
            }
            Self::Message => {
                "Fires when a new message arrives in the conversation \
                 (only the last message in a batch)"
            }
            Self::MessageUpdated => {
                "Fires when an existing message is modified (e.g., streaming \
                 partial becomes complete)"
            }
            Self::WaitingForInput => {
                "Fires when the agent transitions to a state requiring user \
                 input (tool approval, followup, etc.)"
            }
            Self::ResumedRunning => {
                "Fires when the agent resumes execution after waiting or idle"
            }
            Self::StreamingStarted => {
                "Fires when the agent begins streaming a response from the LLM"
            }
            Self::StreamingEnded => {
                "Fires when LLM response streaming completes"
            }
            Self::TaskCompleted => {
                "Fires when the agent determines a task has completed \
                 (completion_result, api_req_failed, or mistake_limit_reached)"
            }
            Self::TaskCleared => {
                "Fires when the current task is explicitly cleared via clearTask()"
            }
            Self::ModeChanged => {
                "Fires when the operational mode changes (e.g., code -> architect)"
            }
            Self::Error => {
                "Fires when an error occurs during message processing"
            }
            Self::SystemOutput => {
                "CLI JSON output: system/lifecycle message (init, etc.)"
            }
            Self::AssistantOutput => {
                "CLI JSON output: assistant text response (may be partial)"
            }
            Self::UserOutput => {
                "CLI JSON output: echoed user input"
            }
            Self::ThinkingOutput => {
                "CLI JSON output: reasoning/thinking content from the model"
            }
            Self::ToolUseOutput => {
                "CLI JSON output: tool invocation with name and input"
            }
            Self::ToolResultOutput => {
                "CLI JSON output: tool execution result (output or error)"
            }
            Self::ErrorOutput => {
                "CLI JSON output: error notification"
            }
            Self::ResultOutput => {
                "CLI JSON output: final task completion with success, content, and cost"
            }
            Self::TaskCreated => {
                "VS Code API: new task created"
            }
            Self::TaskStarted => {
                "VS Code API: task execution began"
            }
            Self::TaskAborted => {
                "VS Code API: task cancelled"
            }
            Self::TaskFocused => {
                "VS Code API: task gained UI focus"
            }
            Self::TaskUnfocused => {
                "VS Code API: task lost UI focus"
            }
            Self::TaskActive => {
                "VS Code API: task is actively running"
            }
            Self::TaskInteractive => {
                "VS Code API: task requires user interaction"
            }
            Self::TaskResumable => {
                "VS Code API: task is paused but resumable"
            }
            Self::TaskIdle => {
                "VS Code API: task is idle"
            }
            Self::TaskPaused => {
                "VS Code API: parent task paused for subtask execution"
            }
            Self::TaskUnpaused => {
                "VS Code API: parent task resumed after subtask completion"
            }
            Self::TaskSpawned => {
                "VS Code API: subtask created from parent task"
            }
            Self::TaskDelegated => {
                "VS Code API: task delegated to a subtask"
            }
            Self::TaskDelegationCompleted => {
                "VS Code API: delegation finished with completion summary"
            }
            Self::TaskDelegationResumed => {
                "VS Code API: delegation control returned to parent"
            }
            Self::TaskModeSwitched => {
                "VS Code API: operational mode switched within a task"
            }
            Self::TaskAskResponded => {
                "VS Code API: user responded to an ask"
            }
            Self::TaskUserMessage => {
                "VS Code API: user sent a message"
            }
            Self::QueuedMessagesUpdated => {
                "VS Code API: message queue changed"
            }
            Self::TaskTokenUsageUpdated => {
                "VS Code API: token/tool usage metrics updated"
            }
            Self::TaskToolFailed => {
                "VS Code API: tool execution failed within a task"
            }
            Self::ProviderProfileChanged => {
                "VS Code API: API provider profile switched"
            }
            Self::CommandsResponse => {
                "VS Code API: response to a commands query"
            }
            Self::ModesResponse => {
                "VS Code API: response to a modes query"
            }
            Self::ModelsResponse => {
                "VS Code API: response to a models query"
            }
            Self::EvalPass => {
                "VS Code API: evaluation passed"
            }
            Self::EvalFail => {
                "VS Code API: evaluation failed"
            }
        }
    }

    /// Which event surface this event originates from.
    pub fn surface(&self) -> EventSurface {
        match self {
            Self::StateChange
            | Self::Message
            | Self::MessageUpdated
            | Self::WaitingForInput
            | Self::ResumedRunning
            | Self::StreamingStarted
            | Self::StreamingEnded
            | Self::TaskCompleted
            | Self::TaskCleared
            | Self::ModeChanged
            | Self::Error => EventSurface::CliProgrammatic,

            Self::SystemOutput
            | Self::AssistantOutput
            | Self::UserOutput
            | Self::ThinkingOutput
            | Self::ToolUseOutput
            | Self::ToolResultOutput
            | Self::ErrorOutput
            | Self::ResultOutput => EventSurface::CliStructuredOutput,

            Self::TaskCreated
            | Self::TaskStarted
            | Self::TaskAborted
            | Self::TaskFocused
            | Self::TaskUnfocused
            | Self::TaskActive
            | Self::TaskInteractive
            | Self::TaskResumable
            | Self::TaskIdle
            | Self::TaskPaused
            | Self::TaskUnpaused
            | Self::TaskSpawned
            | Self::TaskDelegated
            | Self::TaskDelegationCompleted
            | Self::TaskDelegationResumed
            | Self::TaskModeSwitched
            | Self::TaskAskResponded
            | Self::TaskUserMessage
            | Self::QueuedMessagesUpdated
            | Self::TaskTokenUsageUpdated
            | Self::TaskToolFailed
            | Self::ProviderProfileChanged
            | Self::CommandsResponse
            | Self::ModesResponse
            | Self::ModelsResponse
            | Self::EvalPass
            | Self::EvalFail => EventSurface::VsCodeExtensionApi,
        }
    }

    /// Whether this event can influence agent flow.
    ///
    /// Roo Code events are **all observational** -- listener return values
    /// are ignored. However, some events signal that the agent is blocked
    /// and waiting for explicit client action (approve/reject/respond).
    /// These are "actionable" events.
    pub fn is_actionable(&self) -> bool {
        matches!(
            self,
            Self::WaitingForInput | Self::TaskInteractive | Self::TaskCompleted
        )
    }

    /// Whether this event carries a meaningful data payload.
    ///
    /// Events returning `false` fire with `void` (no data).
    pub fn has_payload(&self) -> bool {
        match self {
            // void payloads
            Self::ResumedRunning
            | Self::StreamingStarted
            | Self::StreamingEnded
            | Self::TaskCleared
            | Self::EvalPass
            | Self::EvalFail => false,
            _ => true,
        }
    }

    /// The typed payload this event provides to listeners.
    pub fn payload_type(&self) -> RooCodePayloadType {
        match self {
            Self::StateChange => RooCodePayloadType::AgentStateChange,
            Self::Message => RooCodePayloadType::ClineMessage,
            Self::MessageUpdated => RooCodePayloadType::ClineMessage,
            Self::WaitingForInput => RooCodePayloadType::WaitingForInput,
            Self::ResumedRunning => RooCodePayloadType::Void,
            Self::StreamingStarted => RooCodePayloadType::Void,
            Self::StreamingEnded => RooCodePayloadType::Void,
            Self::TaskCompleted => RooCodePayloadType::TaskCompleted,
            Self::TaskCleared => RooCodePayloadType::Void,
            Self::ModeChanged => RooCodePayloadType::ModeChanged,
            Self::Error => RooCodePayloadType::Error,
            Self::SystemOutput => RooCodePayloadType::JsonOutputEvent,
            Self::AssistantOutput => RooCodePayloadType::JsonOutputEvent,
            Self::UserOutput => RooCodePayloadType::JsonOutputEvent,
            Self::ThinkingOutput => RooCodePayloadType::JsonOutputEvent,
            Self::ToolUseOutput => RooCodePayloadType::JsonOutputEvent,
            Self::ToolResultOutput => RooCodePayloadType::JsonOutputEvent,
            Self::ErrorOutput => RooCodePayloadType::JsonOutputEvent,
            Self::ResultOutput => RooCodePayloadType::JsonOutputEvent,
            Self::TaskCreated
            | Self::TaskStarted
            | Self::TaskAborted
            | Self::TaskFocused
            | Self::TaskUnfocused
            | Self::TaskActive
            | Self::TaskInteractive
            | Self::TaskResumable
            | Self::TaskIdle => RooCodePayloadType::TaskId,
            Self::TaskPaused | Self::TaskUnpaused => RooCodePayloadType::TaskId,
            Self::TaskSpawned
            | Self::TaskDelegated
            | Self::TaskDelegationResumed => RooCodePayloadType::ParentChildTaskIds,
            Self::TaskDelegationCompleted => RooCodePayloadType::DelegationCompleted,
            Self::TaskModeSwitched => RooCodePayloadType::TaskModeSwitched,
            Self::TaskAskResponded | Self::TaskUserMessage => RooCodePayloadType::TaskId,
            Self::QueuedMessagesUpdated => RooCodePayloadType::QueuedMessages,
            Self::TaskTokenUsageUpdated => RooCodePayloadType::TokenUsageUpdated,
            Self::TaskToolFailed => RooCodePayloadType::TaskToolFailed,
            Self::ProviderProfileChanged => RooCodePayloadType::ProviderProfile,
            Self::CommandsResponse => RooCodePayloadType::CommandsList,
            Self::ModesResponse => RooCodePayloadType::ModesList,
            Self::ModelsResponse => RooCodePayloadType::ModelsMap,
            Self::EvalPass | Self::EvalFail => RooCodePayloadType::Void,
        }
    }

    /// The response type for this event.
    ///
    /// All Roo Code events return `void` -- listener return values are
    /// ignored by the EventEmitter. Flow control is achieved by calling
    /// explicit client methods, not by returning values from listeners.
    pub fn response_type(&self) -> RooCodeResponseType {
        // All events are observational-only in Roo Code.
        RooCodeResponseType::Void
    }

    /// The client actions available when this event fires.
    ///
    /// Returns which explicit client methods are meaningful to call in
    /// response to this event. Empty slice means no action is expected.
    pub fn available_actions(&self) -> &'static [ClientAction] {
        match self {
            Self::WaitingForInput => &[
                ClientAction::Approve,
                ClientAction::Reject,
                ClientAction::Respond,
                ClientAction::CancelTask,
                ClientAction::ContinueTerminal,
                ClientAction::AbortTerminal,
            ],
            Self::TaskCompleted => &[
                ClientAction::Respond,
                ClientAction::NewTask,
                ClientAction::ClearTask,
            ],
            Self::TaskInteractive => &[
                ClientAction::Approve,
                ClientAction::Reject,
                ClientAction::Respond,
            ],
            Self::Error => &[
                ClientAction::RetryApiRequest,
                ClientAction::CancelTask,
            ],
            Self::TaskResumable => &[
                ClientAction::ResumeTask,
                ClientAction::CancelTask,
            ],
            Self::TaskIdle => &[
                ClientAction::NewTask,
                ClientAction::ClearTask,
                ClientAction::ResumeTask,
            ],
            _ => &[],
        }
    }
}

impl fmt::Display for RooCodeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native event names
// ---------------------------------------------------------------------------

impl TryFrom<&str> for RooCodeEvent {
    type Error = UnknownEventError;

    /// Parse a native Roo Code event name into the enum variant.
    ///
    /// Handles both Surface 1/3 camelCase names and Surface 2 snake_case
    /// type discriminators. For the ambiguous `"error"` name (which appears
    /// in both Surface 1 as `Error` and Surface 2 as `ErrorOutput`), this
    /// returns `Error` (the CLI programmatic event). Use
    /// `try_from_output_type()` to parse Surface 2 type discriminators
    /// unambiguously.
    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            // Surface 1
            "stateChange" => Ok(Self::StateChange),
            "message" => Ok(Self::Message),
            "messageUpdated" => Ok(Self::MessageUpdated),
            "waitingForInput" => Ok(Self::WaitingForInput),
            "resumedRunning" => Ok(Self::ResumedRunning),
            "streamingStarted" => Ok(Self::StreamingStarted),
            "streamingEnded" => Ok(Self::StreamingEnded),
            "taskCompleted" => Ok(Self::TaskCompleted),
            "taskCleared" => Ok(Self::TaskCleared),
            "modeChanged" => Ok(Self::ModeChanged),
            "error" => Ok(Self::Error),
            // Surface 3
            "taskCreated" => Ok(Self::TaskCreated),
            "taskStarted" => Ok(Self::TaskStarted),
            "taskAborted" => Ok(Self::TaskAborted),
            "taskFocused" => Ok(Self::TaskFocused),
            "taskUnfocused" => Ok(Self::TaskUnfocused),
            "taskActive" => Ok(Self::TaskActive),
            "taskInteractive" => Ok(Self::TaskInteractive),
            "taskResumable" => Ok(Self::TaskResumable),
            "taskIdle" => Ok(Self::TaskIdle),
            "taskPaused" => Ok(Self::TaskPaused),
            "taskUnpaused" => Ok(Self::TaskUnpaused),
            "taskSpawned" => Ok(Self::TaskSpawned),
            "taskDelegated" => Ok(Self::TaskDelegated),
            "taskDelegationCompleted" => Ok(Self::TaskDelegationCompleted),
            "taskDelegationResumed" => Ok(Self::TaskDelegationResumed),
            "taskModeSwitched" => Ok(Self::TaskModeSwitched),
            "taskAskResponded" => Ok(Self::TaskAskResponded),
            "taskUserMessage" => Ok(Self::TaskUserMessage),
            "queuedMessagesUpdated" => Ok(Self::QueuedMessagesUpdated),
            "taskTokenUsageUpdated" => Ok(Self::TaskTokenUsageUpdated),
            "taskToolFailed" => Ok(Self::TaskToolFailed),
            "providerProfileChanged" => Ok(Self::ProviderProfileChanged),
            "commandsResponse" => Ok(Self::CommandsResponse),
            "modesResponse" => Ok(Self::ModesResponse),
            "modelsResponse" => Ok(Self::ModelsResponse),
            "evalPass" => Ok(Self::EvalPass),
            "evalFail" => Ok(Self::EvalFail),
            _ => Err(UnknownEventError(name.to_string())),
        }
    }
}

impl RooCodeEvent {
    /// Parse a `stream-json` output `type` discriminator into the
    /// corresponding event.
    ///
    /// This resolves the `"error"` ambiguity by always returning
    /// `ErrorOutput` for Surface 2 parsing.
    pub fn try_from_output_type(type_name: &str) -> Result<Self, UnknownEventError> {
        match type_name {
            "system" => Ok(Self::SystemOutput),
            "assistant" => Ok(Self::AssistantOutput),
            "user" => Ok(Self::UserOutput),
            "thinking" => Ok(Self::ThinkingOutput),
            "tool_use" => Ok(Self::ToolUseOutput),
            "tool_result" => Ok(Self::ToolResultOutput),
            "error" => Ok(Self::ErrorOutput),
            "result" => Ok(Self::ResultOutput),
            _ => Err(UnknownEventError(type_name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known Roo Code event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown Roo Code event: {0}")]
pub struct UnknownEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

/// Roo Code events do not map 1:1 to AgenticEvent. Many Roo Code events
/// have no unified equivalent (UI focus, eval, query-response, etc.).
/// This `From` impl maps the subset that has a meaningful correspondence;
/// `TryFrom` goes the other direction.
impl TryFrom<RooCodeEvent> for AgenticEvent {
    type Error = &'static str;

    fn try_from(event: RooCodeEvent) -> Result<Self, Self::Error> {
        match event {
            // Direct semantic mappings
            RooCodeEvent::WaitingForInput => Ok(AgenticEvent::HumanInTheLoop),
            RooCodeEvent::TaskCompleted => Ok(AgenticEvent::TurnComplete),
            RooCodeEvent::TaskCleared => Ok(AgenticEvent::SessionEnd),
            RooCodeEvent::ModeChanged => Ok(AgenticEvent::Notification),
            RooCodeEvent::Error => Ok(AgenticEvent::TurnError),
            RooCodeEvent::StreamingStarted => Ok(AgenticEvent::BeforeModel),
            RooCodeEvent::StreamingEnded => Ok(AgenticEvent::AfterModel),

            // Surface 2 tool events
            RooCodeEvent::ToolUseOutput => Ok(AgenticEvent::BeforeTool),
            RooCodeEvent::ToolResultOutput => Ok(AgenticEvent::AfterTool),
            RooCodeEvent::ErrorOutput => Ok(AgenticEvent::TurnError),
            RooCodeEvent::ResultOutput => Ok(AgenticEvent::TurnComplete),

            // Surface 3 task lifecycle
            RooCodeEvent::TaskCreated => Ok(AgenticEvent::SessionStart),
            RooCodeEvent::TaskStarted => Ok(AgenticEvent::SessionStart),
            RooCodeEvent::TaskAborted => Ok(AgenticEvent::SessionEnd),
            RooCodeEvent::TaskToolFailed => Ok(AgenticEvent::ToolError),
            RooCodeEvent::TaskSpawned => Ok(AgenticEvent::SubagentStart),
            RooCodeEvent::TaskDelegationCompleted => Ok(AgenticEvent::SubagentStop),

            // No unified equivalent
            RooCodeEvent::StateChange
            | RooCodeEvent::Message
            | RooCodeEvent::MessageUpdated
            | RooCodeEvent::ResumedRunning
            | RooCodeEvent::SystemOutput
            | RooCodeEvent::AssistantOutput
            | RooCodeEvent::UserOutput
            | RooCodeEvent::ThinkingOutput
            | RooCodeEvent::TaskFocused
            | RooCodeEvent::TaskUnfocused
            | RooCodeEvent::TaskActive
            | RooCodeEvent::TaskInteractive
            | RooCodeEvent::TaskResumable
            | RooCodeEvent::TaskIdle
            | RooCodeEvent::TaskPaused
            | RooCodeEvent::TaskUnpaused
            | RooCodeEvent::TaskDelegated
            | RooCodeEvent::TaskDelegationResumed
            | RooCodeEvent::TaskModeSwitched
            | RooCodeEvent::TaskAskResponded
            | RooCodeEvent::TaskUserMessage
            | RooCodeEvent::QueuedMessagesUpdated
            | RooCodeEvent::TaskTokenUsageUpdated
            | RooCodeEvent::ProviderProfileChanged
            | RooCodeEvent::CommandsResponse
            | RooCodeEvent::ModesResponse
            | RooCodeEvent::ModelsResponse
            | RooCodeEvent::EvalPass
            | RooCodeEvent::EvalFail => Err("Roo Code event has no unified equivalent"),
        }
    }
}

impl TryFrom<AgenticEvent> for RooCodeEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified event to Roo Code.
    ///
    /// Returns the most semantically appropriate Roo Code event. Many
    /// unified events have no Roo Code equivalent because Roo Code does
    /// not support shell-based interception hooks.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::SessionStart => Ok(RooCodeEvent::TaskCreated),
            AgenticEvent::SessionEnd => Ok(RooCodeEvent::TaskCleared),
            AgenticEvent::BeforeTool => Ok(RooCodeEvent::ToolUseOutput),
            AgenticEvent::AfterTool => Ok(RooCodeEvent::ToolResultOutput),
            AgenticEvent::ToolError => Ok(RooCodeEvent::TaskToolFailed),
            AgenticEvent::TurnComplete => Ok(RooCodeEvent::TaskCompleted),
            AgenticEvent::TurnError => Ok(RooCodeEvent::Error),
            AgenticEvent::SubagentStart => Ok(RooCodeEvent::TaskSpawned),
            AgenticEvent::SubagentStop => Ok(RooCodeEvent::TaskDelegationCompleted),
            AgenticEvent::BeforeModel => Ok(RooCodeEvent::StreamingStarted),
            AgenticEvent::AfterModel => Ok(RooCodeEvent::StreamingEnded),
            AgenticEvent::HumanInTheLoop => Ok(RooCodeEvent::WaitingForInput),
            AgenticEvent::Notification => Ok(RooCodeEvent::ModeChanged),
            // No Roo Code equivalent
            AgenticEvent::BeforePrompt
            | AgenticEvent::PermissionRequest
            | AgenticEvent::BeforeCompact => {
                Err("Roo Code does not support this event")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// The event surface a Roo Code event originates from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventSurface {
    /// CLI programmatic events via `ExtensionClient` (`ClientEventMap`).
    CliProgrammatic,
    /// CLI structured output events via `--output-format stream-json`.
    CliStructuredOutput,
    /// VS Code extension API events via `RooCodeAPI` (`RooCodeEventName`).
    VsCodeExtensionApi,
}

impl EventSurface {
    /// Whether this surface is available in headless/CLI mode.
    pub fn available_in_cli(&self) -> bool {
        matches!(
            self,
            Self::CliProgrammatic | Self::CliStructuredOutput
        )
    }
}

impl fmt::Display for EventSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CliProgrammatic => f.write_str("CLI Programmatic"),
            Self::CliStructuredOutput => f.write_str("CLI Structured Output"),
            Self::VsCodeExtensionApi => f.write_str("VS Code Extension API"),
        }
    }
}

/// Explicit client actions that can be called to influence agent flow.
///
/// Roo Code events are observational; these actions are the mechanism
/// by which external code controls the agent loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientAction {
    /// Approve a tool/command/MCP ask.
    Approve,
    /// Reject a tool/command/MCP ask.
    Reject,
    /// Send a text (and optional images) response.
    Respond,
    /// Start a new task.
    NewTask,
    /// Cancel the running task.
    CancelTask,
    /// Clear the current task.
    ClearTask,
    /// Resume a paused/resumable task.
    ResumeTask,
    /// Retry after API failure.
    RetryApiRequest,
    /// Continue after command output.
    ContinueTerminal,
    /// Abort a running command.
    AbortTerminal,
}

impl fmt::Display for ClientAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approve => f.write_str("approve()"),
            Self::Reject => f.write_str("reject()"),
            Self::Respond => f.write_str("respond()"),
            Self::NewTask => f.write_str("newTask()"),
            Self::CancelTask => f.write_str("cancelTask()"),
            Self::ClearTask => f.write_str("clearTask()"),
            Self::ResumeTask => f.write_str("resumeTask()"),
            Self::RetryApiRequest => f.write_str("retryApiRequest()"),
            Self::ContinueTerminal => f.write_str("continueTerminal()"),
            Self::AbortTerminal => f.write_str("abortTerminal()"),
        }
    }
}

/// Discriminant for the event-specific payload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RooCodePayloadType {
    /// No data (`void`).
    Void,
    /// `AgentStateChangePayload` with previous/current state.
    AgentStateChange,
    /// `ClineMessagePayload` -- a single conversation message.
    ClineMessage,
    /// `WaitingForInputPayload` with ask type, state info, and message.
    WaitingForInput,
    /// `TaskCompletedPayload` with success flag and state info.
    TaskCompleted,
    /// `ModeChangedPayload` with previous and current mode.
    ModeChanged,
    /// Standard `Error` object.
    Error,
    /// `JsonOutputEventPayload` from `stream-json` output.
    JsonOutputEvent,
    /// Single `task_id: String`.
    TaskId,
    /// Parent and child task IDs.
    ParentChildTaskIds,
    /// Delegation completed with summary.
    DelegationCompleted,
    /// Task mode switched with task ID and new mode.
    TaskModeSwitched,
    /// Queued messages update.
    QueuedMessages,
    /// Token and tool usage update.
    TokenUsageUpdated,
    /// Tool failure with task ID, tool name, and error.
    TaskToolFailed,
    /// Provider profile with name and provider.
    ProviderProfile,
    /// List of commands.
    CommandsList,
    /// List of modes.
    ModesList,
    /// Map of model IDs to model info.
    ModelsMap,
}

/// Response type for Roo Code events.
///
/// All Roo Code events are fire-and-forget from the listener's perspective.
/// This enum exists for API symmetry with Claude Code's `ResponseType`,
/// but currently has a single variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RooCodeResponseType {
    /// No return value processed (all Roo Code events).
    Void,
}

// ---------------------------------------------------------------------------
// Agent state types
// ---------------------------------------------------------------------------

/// Roo Code agent loop states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLoopState {
    /// No active task.
    NoTask,
    /// Task is running.
    Running,
    /// LLM response is streaming.
    Streaming,
    /// Agent is blocked waiting for user input.
    WaitingForInput,
    /// Agent is idle (task finished or paused).
    Idle,
    /// Task can be resumed.
    Resumable,
}

/// What kind of user action is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredAction {
    /// No action needed.
    None,
    /// User must approve an operation.
    Approve,
    /// User must answer a question.
    Answer,
    /// User can retry or start a new task.
    RetryOrNewTask,
    /// User can proceed or start a new task.
    ProceedOrNewTask,
    /// User should start a task.
    StartTask,
    /// User can resume or abandon.
    ResumeOrAbandon,
    /// User should start a new task.
    StartNewTask,
    /// User can continue or abort.
    ContinueOrAbort,
}

/// The type of "ask" the agent is presenting to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineAsk {
    /// Follow-up question.
    Followup,
    /// Command execution approval.
    Command,
    /// Command produced output.
    CommandOutput,
    /// Task completion result.
    CompletionResult,
    /// Tool usage approval.
    Tool,
    /// API request failed.
    ApiReqFailed,
    /// Resume a previous task.
    ResumeTask,
    /// Resume a completed task.
    ResumeCompletedTask,
    /// Mistake limit reached.
    MistakeLimitReached,
    /// MCP server usage approval.
    UseMcpServer,
    /// Auto-approval max requests reached.
    AutoApprovalMaxReqReached,
}

impl ClineAsk {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Followup => "followup",
            Self::Command => "command",
            Self::CommandOutput => "command_output",
            Self::CompletionResult => "completion_result",
            Self::Tool => "tool",
            Self::ApiReqFailed => "api_req_failed",
            Self::ResumeTask => "resume_task",
            Self::ResumeCompletedTask => "resume_completed_task",
            Self::MistakeLimitReached => "mistake_limit_reached",
            Self::UseMcpServer => "use_mcp_server",
            Self::AutoApprovalMaxReqReached => "auto_approval_max_req_reached",
        }
    }
}

/// The type of output `stream-json` tool events carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutputSubtype {
    /// Standard tool invocation.
    Tool,
    /// Shell command execution.
    Command,
    /// MCP server tool invocation.
    Mcp,
}

// ---------------------------------------------------------------------------
// Event-specific payloads
// ---------------------------------------------------------------------------

/// Payload for `StateChange` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateChangePayload {
    pub previous_state: AgentStateInfo,
    pub current_state: AgentStateInfo,
    pub is_significant_change: bool,
}

/// Snapshot of the agent's current state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateInfo {
    pub state: AgentLoopState,
    pub is_waiting_for_input: bool,
    pub is_running: bool,
    pub is_streaming: bool,
    #[serde(default)]
    pub current_ask: Option<ClineAsk>,
    pub required_action: RequiredAction,
    #[serde(default)]
    pub last_message_ts: Option<u64>,
    #[serde(default)]
    pub last_message: Option<ClineMessagePayload>,
    pub description: String,
}

/// Payload for `Message` and `MessageUpdated` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineMessagePayload {
    pub ts: u64,
    /// `"ask"` or `"say"`.
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub ask: Option<ClineAsk>,
    #[serde(default)]
    pub say: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub partial: Option<bool>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub images: Option<Vec<String>>,
    #[serde(default)]
    pub progress_status: Option<String>,
}

/// Payload for `WaitingForInput` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitingForInputPayload {
    pub ask: ClineAsk,
    pub state_info: AgentStateInfo,
    pub message: ClineMessagePayload,
}

/// Payload for `TaskCompleted` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletedPayload {
    pub success: bool,
    pub state_info: AgentStateInfo,
    #[serde(default)]
    pub message: Option<ClineMessagePayload>,
}

/// Payload for `ModeChanged` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeChangedPayload {
    #[serde(default)]
    pub previous_mode: Option<String>,
    pub current_mode: String,
}

/// Payload for `stream-json` output events (Surface 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonOutputEventPayload {
    /// Event type discriminator (`system`, `assistant`, `tool_use`, etc.).
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub done: Option<bool>,
    #[serde(default)]
    pub subtype: Option<String>,
    /// Present on `tool_use` events.
    #[serde(default)]
    pub tool_use: Option<ToolUseInfo>,
    /// Present on `tool_result` events.
    #[serde(default)]
    pub tool_result: Option<ToolResultInfo>,
    /// Present on `result` events.
    #[serde(default)]
    pub success: Option<bool>,
    /// Present on `result` events.
    #[serde(default)]
    pub cost: Option<CostInfo>,
}

/// Tool invocation details in `tool_use` output events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseInfo {
    pub name: String,
    #[serde(default)]
    pub input: Option<Value>,
}

/// Tool result details in `tool_result` output events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultInfo {
    pub name: String,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Cost information in `result` output events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    #[serde(default)]
    pub total_cost: Option<f64>,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_writes: Option<u64>,
    #[serde(default)]
    pub cache_reads: Option<u64>,
}

/// Payload for `TaskDelegationCompleted` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationCompletedPayload {
    pub parent_task_id: String,
    pub child_task_id: String,
    #[serde(default)]
    pub completion_result_summary: Option<String>,
}

/// Payload for `TaskModeSwitched` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskModeSwitchedPayload {
    pub task_id: String,
    pub mode: String,
}

/// Payload for `TaskTokenUsageUpdated` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageUpdatedPayload {
    pub task_id: String,
    pub token_usage: Value,
    pub tool_usage: Value,
}

/// Payload for `TaskToolFailed` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToolFailedPayload {
    pub task_id: String,
    pub tool_name: String,
    pub error: String,
}

/// Payload for `ProviderProfileChanged` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfilePayload {
    pub name: String,
    pub provider: String,
}

/// Payload for VS Code extension API `message` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMessagePayload {
    pub task_id: String,
    /// `"created"` or `"updated"`.
    pub action: String,
    pub message: ClineMessagePayload,
}

// ---------------------------------------------------------------------------
// Unified input payload enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any Roo Code event payload.
///
/// Since Roo Code events come from three different surfaces with different
/// serialization formats, this enum normalizes them into a single dispatch
/// type. Surface 2 events are parsed from NDJSON lines; Surface 1 and 3
/// events are received programmatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum RooCodeInput {
    StateChange(AgentStateChangePayload),
    Message(ClineMessagePayload),
    MessageUpdated(ClineMessagePayload),
    WaitingForInput(WaitingForInputPayload),
    ResumedRunning,
    StreamingStarted,
    StreamingEnded,
    TaskCompleted(TaskCompletedPayload),
    TaskCleared,
    ModeChanged(ModeChangedPayload),
    Error { error: String },
    JsonOutput(JsonOutputEventPayload),
    Task { task_id: String },
    ParentChild { parent_task_id: String, child_task_id: String },
    DelegationCompleted(DelegationCompletedPayload),
    TaskModeSwitched(TaskModeSwitchedPayload),
    QueuedMessages { task_id: String, messages: Vec<Value> },
    TokenUsageUpdated(TokenUsageUpdatedPayload),
    TaskToolFailed(TaskToolFailedPayload),
    ProviderProfileChanged(ProviderProfilePayload),
    ExtensionMessage(ExtensionMessagePayload),
    CommandsList { commands: Vec<Value> },
    ModesList { modes: Vec<Value> },
    ModelsMap { models: Value },
    EvalPass,
    EvalFail,
}

impl RooCodeInput {
    /// Returns the event variant for this input payload.
    pub fn event(&self) -> RooCodeEvent {
        match self {
            Self::StateChange(_) => RooCodeEvent::StateChange,
            Self::Message(_) => RooCodeEvent::Message,
            Self::MessageUpdated(_) => RooCodeEvent::MessageUpdated,
            Self::WaitingForInput(_) => RooCodeEvent::WaitingForInput,
            Self::ResumedRunning => RooCodeEvent::ResumedRunning,
            Self::StreamingStarted => RooCodeEvent::StreamingStarted,
            Self::StreamingEnded => RooCodeEvent::StreamingEnded,
            Self::TaskCompleted(_) => RooCodeEvent::TaskCompleted,
            Self::TaskCleared => RooCodeEvent::TaskCleared,
            Self::ModeChanged(_) => RooCodeEvent::ModeChanged,
            Self::Error { .. } => RooCodeEvent::Error,
            Self::JsonOutput(p) => {
                RooCodeEvent::try_from_output_type(&p.event_type)
                    .unwrap_or(RooCodeEvent::SystemOutput)
            }
            Self::Task { .. } => RooCodeEvent::TaskCreated, // ambiguous; caller should track
            Self::ParentChild { .. } => RooCodeEvent::TaskSpawned,
            Self::DelegationCompleted(_) => RooCodeEvent::TaskDelegationCompleted,
            Self::TaskModeSwitched(_) => RooCodeEvent::TaskModeSwitched,
            Self::QueuedMessages { .. } => RooCodeEvent::QueuedMessagesUpdated,
            Self::TokenUsageUpdated(_) => RooCodeEvent::TaskTokenUsageUpdated,
            Self::TaskToolFailed(_) => RooCodeEvent::TaskToolFailed,
            Self::ProviderProfileChanged(_) => RooCodeEvent::ProviderProfileChanged,
            Self::ExtensionMessage(_) => RooCodeEvent::Message,
            Self::CommandsList { .. } => RooCodeEvent::CommandsResponse,
            Self::ModesList { .. } => RooCodeEvent::ModesResponse,
            Self::ModelsMap { .. } => RooCodeEvent::ModelsResponse,
            Self::EvalPass => RooCodeEvent::EvalPass,
            Self::EvalFail => RooCodeEvent::EvalFail,
        }
    }

    /// Extract a tool name if this is a tool-related event.
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::JsonOutput(p) => p
                .tool_use
                .as_ref()
                .map(|t| t.name.as_str())
                .or_else(|| p.tool_result.as_ref().map(|t| t.name.as_str())),
            Self::TaskToolFailed(p) => Some(&p.tool_name),
            _ => None,
        }
    }

    /// Extract a task ID if this payload carries one.
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::Task { task_id } => Some(task_id),
            Self::ParentChild { parent_task_id, .. } => Some(parent_task_id),
            Self::DelegationCompleted(p) => Some(&p.parent_task_id),
            Self::TaskModeSwitched(p) => Some(&p.task_id),
            Self::QueuedMessages { task_id, .. } => Some(task_id),
            Self::TokenUsageUpdated(p) => Some(&p.task_id),
            Self::TaskToolFailed(p) => Some(&p.task_id),
            Self::ExtensionMessage(p) => Some(&p.task_id),
            _ => None,
        }
    }

    /// Whether the agent is currently waiting for user input.
    ///
    /// Returns `true` for `WaitingForInput` events and for `StateChange`
    /// events where the current state indicates waiting.
    pub fn is_waiting_for_input(&self) -> bool {
        match self {
            Self::WaitingForInput(_) => true,
            Self::StateChange(p) => p.current_state.is_waiting_for_input,
            _ => false,
        }
    }

    /// Extract the ask type if this is an ask-related event.
    pub fn ask_type(&self) -> Option<ClineAsk> {
        match self {
            Self::WaitingForInput(p) => Some(p.ask),
            Self::StateChange(p) => p.current_state.current_ask,
            Self::Message(p) | Self::MessageUpdated(p) => p.ask,
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Roo Code operational modes
// ---------------------------------------------------------------------------

/// Known Roo Code operational modes.
///
/// Roo Code supports both built-in modes and user-defined custom modes.
/// This enum covers the built-in set; custom modes are represented as
/// strings in the payload types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuiltinMode {
    Code,
    Architect,
    Ask,
    Debug,
}

impl BuiltinMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Architect => "architect",
            Self::Ask => "ask",
            Self::Debug => "debug",
        }
    }
}

// ---------------------------------------------------------------------------
// CLI flags relevant to event behavior
// ---------------------------------------------------------------------------

/// CLI output format modes that affect which event surfaces are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Default human-readable text. Surface 2 events are not emitted.
    Text,
    /// Single JSON object at task completion.
    Json,
    /// NDJSON stream of events throughout execution.
    StreamJson,
}

impl OutputFormat {
    /// Whether Surface 2 (structured output) events are emitted.
    pub fn emits_json_events(&self) -> bool {
        matches!(self, Self::Json | Self::StreamJson)
    }

    /// Whether events stream incrementally or arrive as a batch.
    pub fn is_streaming(&self) -> bool {
        matches!(self, Self::StreamJson)
    }
}
```

## Design Considerations

- **Three event surfaces, one enum.** Roo Code exposes events through three fundamentally different mechanisms (CLI programmatic EventEmitter, CLI stdout NDJSON, VS Code extension API). Rather than three separate enums, a single `RooCodeEvent` with a `surface()` discriminator keeps the dispatch layer unified. The `EventSurface` enum allows callers to filter by surface when needed.

- **All events are observational.** This is the defining architectural difference from Claude Code. Roo Code event listeners cannot return values to influence flow. The `response_type()` method always returns `Void`. Instead, the design captures the _action_ pattern through `ClientAction` and `available_actions()`, which tells callers what explicit client methods are meaningful to call when a given event fires.

- **No matcher system.** Claude Code has a regex-based matcher system for filtering events by tool name, session source, etc. Roo Code has nothing equivalent -- all filtering must happen in listener code. Consequently, there are no `matcher_field()` or `supports_matcher()` methods. The `ask_type()` and `tool_name()` extractors on `RooCodeInput` serve the same filtering purpose in a programmatic way.

- **No exit code semantics.** Claude Code hooks use process exit codes (0 = success, 2 = block) to control flow. Roo Code has no shell-based hooks, so there is no `ExitCode` type. The `ClientAction` enum replaces this concept.

- **47 events vs Claude Code's 14.** Roo Code has a much larger event surface due to the VS Code extension API's granular task lifecycle events (created, started, aborted, focused, unfocused, active, interactive, resumable, idle) and subtask delegation events. Many of these have no equivalent in any other agentic CLI.

- **Surface 2 name collision.** Both Surface 1 and Surface 2 have an `"error"` event with different semantics. The `try_from_output_type()` constructor resolves this ambiguity for Surface 2 parsing, while the standard `TryFrom<&str>` defaults to the Surface 1 event.

- **`ALL` vs `ALL_WITH_QUERIES`.** The `ALL` constant excludes query-response events (`CommandsResponse`, `ModesResponse`, `ModelsResponse`) and eval events (`EvalPass`, `EvalFail`) since these are not lifecycle events. `ALL_WITH_QUERIES` includes everything for completeness.

- **Partial/streaming awareness.** The `JsonOutputEventPayload` includes `done` and `id` fields to support delta accumulation for `stream-json` mode. The `ClineMessagePayload` includes `partial` for the same reason. Callers must buffer partial events by `id` until `done: true`.

- **`AgentStateInfo` as the richest payload.** The `StateChange` event carries the most information of any Roo Code event. Rather than flattening this into the main event type, it is kept as a separate struct since many other events reference it (e.g., `WaitingForInput` and `TaskCompleted` both embed it).

- **Lossy unified mapping.** Only 17 of 47 Roo Code events map to `AgenticEvent`. The 30 unmapped events are either UI-specific (focus/unfocus), query-responses, eval signals, or granular state transitions that have no cross-provider equivalent. The `TryFrom` implementations make this lossiness explicit with `Err` returns rather than silently mapping to a "closest" variant.

- **No common input fields.** Unlike Claude Code (which has `session_id`, `transcript_path`, `cwd`, etc. on every event), Roo Code events have no shared envelope. Each surface has its own structure. The `RooCodeInput` enum handles this heterogeneity through per-variant payloads rather than a flattened common struct.

## Claude Code Mapping

- **`StateChange`** -- Distinct from anything in Claude Code. Claude Code has no concept of an aggregate state-transition event. The closest analog would be composing `SessionStart` + `Stop` + other events, but `StateChange` fires on every micro-transition in the agent loop.

- **`Message`** -- Partially overlaps with Claude Code's `Notification` (both carry text from the agent), but `Message` represents conversation messages (ask/say typed), not system notifications. Different payload, different semantics.

- **`MessageUpdated`** -- Distinct. Claude Code has no streaming message update event; tool results arrive atomically via `PostToolUse`.

- **`WaitingForInput`** -- Maps to Claude Code's `PermissionRequest` in the tool-approval case, but is much broader. It also covers followup questions, command approval, completion feedback, and MCP server approval. Same trigger category (agent needs user action) but fundamentally different payload and response model. In Claude Code, the hook can return a decision; in Roo Code, you must call `approve()`/`reject()` separately.

- **`ResumedRunning`** -- Distinct. Claude Code has no event for resuming after a pause.

- **`StreamingStarted`** -- Maps loosely to Claude Code's conceptual "before model call" (`BeforeModel` in unified), but Claude Code does not expose this as a hook. Different trigger granularity.

- **`StreamingEnded`** -- Maps loosely to `AfterModel` in the unified enum. Claude Code does not expose this.

- **`TaskCompleted`** -- Same trigger as Claude Code's `TaskCompleted`, but different payload (Roo: `{success, stateInfo, message}` vs Claude: `{task_id, task_subject, task_description}`). Roo's version also has the gotcha that the agent may still be waiting for feedback. Claude Code's `TaskCompleted` is a blocking event (exit code 2 can prevent completion); Roo's is observational only.

- **`TaskCleared`** -- Maps to Claude Code's `SessionEnd` when `reason` is `"clear"`. Same trigger, different payload (Roo: void vs Claude: `{reason}`).

- **`ModeChanged`** -- Distinct. Claude Code has no operational mode concept.

- **`Error`** -- Maps loosely to Claude Code's `PostToolUseFailure` or general error conditions, but Roo's `Error` is a catch-all for any processing error, not specifically tool failures. Different scope.

- **`SystemOutput`** -- Distinct. CLI stdout JSON surface has no Claude Code equivalent.

- **`AssistantOutput`** -- Distinct. Claude Code does not expose raw assistant text streaming as a hook event.

- **`UserOutput`** -- Distinct. Claude Code does not echo user input as an event.

- **`ThinkingOutput`** -- Distinct. Claude Code does not expose model reasoning/thinking as a hook event.

- **`ToolUseOutput`** -- Same trigger as Claude Code's `PreToolUse` (tool invocation about to happen). Different payload format (Roo: `{name, input}` in NDJSON vs Claude: full `PreToolUsePayload` on stdin). Critically different response model: Claude Code can block/allow/deny; Roo Code is read-only.

- **`ToolResultOutput`** -- Same trigger as Claude Code's `PostToolUse` (tool completed). Different payload (Roo: `{name, output, error}` vs Claude: `{tool_name, tool_use_id, tool_input, tool_response}`). Roo's version does not distinguish success from failure in the event type itself (error field is optional).

- **`ErrorOutput`** -- Overlaps with Claude Code's `PostToolUseFailure` for tool errors, but also covers non-tool errors. Broader scope.

- **`ResultOutput`** -- Maps loosely to Claude Code's `Stop` (agent finished). Different payload (Roo includes cost/token info; Claude includes `stop_hook_active` loop guard). Roo's is read-only; Claude's can block to continue the conversation.

- **`TaskCreated`** -- Maps to Claude Code's `SessionStart` with `source: "startup"`. Same trigger, different payload (Roo: `task_id` only vs Claude: `{source, model, agent_type}`).

- **`TaskStarted`** -- Maps to Claude Code's `SessionStart`. Same category but Roo distinguishes creation from execution start; Claude Code does not.

- **`TaskAborted`** -- Maps to Claude Code's `SessionEnd`. Same trigger category, different payload.

- **`TaskFocused` / `TaskUnfocused`** -- Distinct. VS Code UI events with no CLI/agentic equivalent in Claude Code.

- **`TaskActive` / `TaskInteractive` / `TaskResumable` / `TaskIdle`** -- Distinct. Fine-grained task state notifications specific to the VS Code extension API. Claude Code's closest equivalent is the implicit state communicated through event sequencing.

- **`TaskPaused` / `TaskUnpaused`** -- Distinct. Subtask-related parent pause/resume has no Claude Code equivalent.

- **`TaskSpawned`** -- Maps to Claude Code's `SubagentStart`. Same trigger (child agent created). Different payload (Roo: `{parentTaskId, childTaskId}` vs Claude: `{agent_id, agent_type}`).

- **`TaskDelegated`** -- Partially maps to Claude Code's `SubagentStart`. Roo distinguishes spawning from delegation; Claude Code does not.

- **`TaskDelegationCompleted`** -- Maps to Claude Code's `SubagentStop`. Same trigger, different payload (Roo includes `completionResultSummary`; Claude includes `stop_hook_active`, `agent_transcript_path`).

- **`TaskDelegationResumed`** -- Distinct. Claude Code has no event for delegation control transfer.

- **`TaskModeSwitched`** -- Distinct. Claude Code has no mode concept.

- **`TaskAskResponded`** -- Distinct. Claude Code does not emit an event when the user responds to an ask.

- **`TaskUserMessage`** -- Maps loosely to Claude Code's `UserPromptSubmit`. Same trigger (user sends input), but Roo's version fires after the fact (observational) while Claude's fires before processing (blocking).

- **`QueuedMessagesUpdated`** -- Distinct. Claude Code has no message queue concept.

- **`TaskTokenUsageUpdated`** -- Distinct. Claude Code does not expose token usage as a hook event.

- **`TaskToolFailed`** -- Maps to Claude Code's `PostToolUseFailure`. Same trigger (tool failed). Different payload (Roo: `{taskId, toolName, error}` vs Claude: `{tool_name, tool_use_id, tool_input, error, is_interrupt}`). Roo's is observational; Claude's can inject context.

- **`ProviderProfileChanged`** -- Distinct. Claude Code has no provider profile switching concept.

- **`CommandsResponse` / `ModesResponse` / `ModelsResponse`** -- Distinct. Query-response events with no Claude Code equivalent.

- **`EvalPass` / `EvalFail`** -- Distinct. Evaluation framework events with no Claude Code equivalent.

The file has been written to `/Volumes/coding/personal/rusty-biscuit/claudine/docs/designs/roo-code.md`.

---

**Skills used:** `claudine` (hook/event handler system design context).

Here is a summary of the design:

**File:** `/Volumes/coding/personal/rusty-biscuit/claudine/docs/designs/roo-code.md`

The design captures all 47 Roo Code events across three event surfaces in a single `RooCodeEvent` enum. The key architectural departure from the Claude Code baseline is that all Roo Code events are observational only -- there is no shell-based hook interception, no exit code semantics, and no matcher system. Instead, flow control is modeled through the `ClientAction` enum and the `available_actions()` method, which describes what explicit API calls are meaningful in response to each event.

Of the 47 events, only 17 have a meaningful mapping to `AgenticEvent`. The remaining 30 are either VS Code UI-specific (focus, unfocus, state granularity), query-response pairs, evaluation signals, or streaming/partial-update events that have no cross-provider equivalent. Both `From`/`TryFrom` directions are explicitly partial, using `Result` returns to make the lossiness transparent.