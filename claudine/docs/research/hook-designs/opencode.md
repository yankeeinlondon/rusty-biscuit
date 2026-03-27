# OpenCode Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All 14 hook events supported by OpenCode.
///
/// OpenCode uses a JavaScript/TypeScript plugin system where plugins export
/// named hook functions. The hook names use dot-separated identifiers
/// (e.g., `tool.execute.before`). The variant names here use Rust PascalCase
/// but preserve the original semantics. The `native_name()` method returns
/// the exact dot-separated string OpenCode expects.
///
/// OpenCode hooks follow an input/output mutation pattern: the plugin
/// receives a read-only `input` and a mutable `output` object, mutates
/// `output` in place, and returns `Promise<void>`. This is fundamentally
/// different from Claude Code's stdin-JSON/stdout-JSON/exit-code protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpenCodeEvent {
    /// Catch-all event bus subscriber. Receives every system bus event.
    Event,
    /// Fires before a tool call executes. Can modify args or throw to block.
    ToolExecuteBefore,
    /// Fires after a tool call completes successfully. Can modify output.
    ToolExecuteAfter,
    /// Fires when tool definitions are assembled for the LLM.
    ToolDefinition,
    /// Injects environment variables into all shell executions.
    ShellEnv,
    /// Fires when a user message is prepared for the LLM.
    ChatMessage,
    /// Modifies LLM request parameters (temperature, topP, topK, options).
    ChatParams,
    /// Adds custom HTTP headers to outgoing LLM API requests.
    ChatHeaders,
    /// Fires when the permission system evaluates a tool call to "ask".
    PermissionAsk,
    /// Fires before a custom slash command executes.
    CommandExecuteBefore,
    /// Fires once at startup after configuration is loaded.
    Config,
    /// Provides custom authentication flows for LLM providers.
    Auth,
    /// (Experimental) Modifies the system prompt before sending to the LLM.
    ExperimentalChatSystemTransform,
    /// (Experimental) Modifies the entire message history before sending.
    ExperimentalChatMessagesTransform,
    /// (Experimental) Fires before session compaction starts.
    ExperimentalSessionCompacting,
    /// (Experimental) Fires when a text generation part completes.
    ExperimentalTextComplete,
}

impl OpenCodeEvent {
    /// All variants in logical grouping order.
    pub const ALL: [OpenCodeEvent; 16] = [
        // Lifecycle
        Self::Config,
        Self::Auth,
        Self::Event,
        // Tool lifecycle
        Self::ToolDefinition,
        Self::ToolExecuteBefore,
        Self::ToolExecuteAfter,
        // Shell
        Self::ShellEnv,
        // Chat / LLM
        Self::ChatMessage,
        Self::ChatParams,
        Self::ChatHeaders,
        // Permission
        Self::PermissionAsk,
        // Commands
        Self::CommandExecuteBefore,
        // Experimental
        Self::ExperimentalChatSystemTransform,
        Self::ExperimentalChatMessagesTransform,
        Self::ExperimentalSessionCompacting,
        Self::ExperimentalTextComplete,
    ];

    /// The native hook name string as it appears in plugin exports.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::ToolExecuteBefore => "tool.execute.before",
            Self::ToolExecuteAfter => "tool.execute.after",
            Self::ToolDefinition => "tool.definition",
            Self::ShellEnv => "shell.env",
            Self::ChatMessage => "chat.message",
            Self::ChatParams => "chat.params",
            Self::ChatHeaders => "chat.headers",
            Self::PermissionAsk => "permission.ask",
            Self::CommandExecuteBefore => "command.execute.before",
            Self::Config => "config",
            Self::Auth => "auth",
            Self::ExperimentalChatSystemTransform => "experimental.chat.system.transform",
            Self::ExperimentalChatMessagesTransform => "experimental.chat.messages.transform",
            Self::ExperimentalSessionCompacting => "experimental.session.compacting",
            Self::ExperimentalTextComplete => "experimental.text.complete",
        }
    }

    /// OpenCode-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Event => {
                "Catch-all system bus subscriber; receives every event and must filter by event.type"
            }
            Self::ToolExecuteBefore => {
                "Fires before a tool call executes; can mutate args or throw to block"
            }
            Self::ToolExecuteAfter => {
                "Fires after a tool call completes successfully; can modify output shown to agent"
            }
            Self::ToolDefinition => {
                "Fires when tool definitions are assembled for the LLM; can modify description and parameter schema"
            }
            Self::ShellEnv => {
                "Injects environment variables into all shell executions (bash tool, PTY, subprocesses)"
            }
            Self::ChatMessage => {
                "Fires when a user message is prepared; can modify message content and parts"
            }
            Self::ChatParams => {
                "Modifies LLM request parameters (temperature, topP, topK) before sending"
            }
            Self::ChatHeaders => {
                "Adds custom HTTP headers to outgoing LLM API requests"
            }
            Self::PermissionAsk => {
                "Fires when permission evaluates to 'ask'; can programmatically allow or deny"
            }
            Self::CommandExecuteBefore => {
                "Fires before a custom slash command executes; can modify output parts"
            }
            Self::Config => {
                "Fires once at startup after configuration is loaded; read-only access to merged config"
            }
            Self::Auth => {
                "Provides custom authentication flows for LLM providers (OAuth, API key)"
            }
            Self::ExperimentalChatSystemTransform => {
                "(Experimental) Modifies the system prompt array before sending to the LLM"
            }
            Self::ExperimentalChatMessagesTransform => {
                "(Experimental) Modifies the entire message history before sending to the LLM"
            }
            Self::ExperimentalSessionCompacting => {
                "(Experimental) Fires before session compaction; can customize the compaction prompt"
            }
            Self::ExperimentalTextComplete => {
                "(Experimental) Fires when a text generation part completes; can post-process text"
            }
        }
    }

    /// Whether this hook can block or prevent the intercepted action.
    ///
    /// In OpenCode, blocking is done by throwing an error (for tool hooks)
    /// or by mutating the output status (for permission hooks). This is
    /// fundamentally different from Claude Code's exit-code-2 protocol.
    pub fn can_block(&self) -> bool {
        matches!(
            self,
            Self::ToolExecuteBefore | Self::PermissionAsk
        )
    }

    /// Whether this hook can mutate its output to affect agent behavior.
    ///
    /// Most OpenCode hooks follow the input/output mutation pattern. Hooks
    /// that return `false` are either fire-and-forget (`Event`) or
    /// read-only (`Config`).
    pub fn can_mutate_output(&self) -> bool {
        !matches!(self, Self::Event | Self::Config)
    }

    /// Whether this hook is marked experimental and may change.
    pub fn is_experimental(&self) -> bool {
        matches!(
            self,
            Self::ExperimentalChatSystemTransform
                | Self::ExperimentalChatMessagesTransform
                | Self::ExperimentalSessionCompacting
                | Self::ExperimentalTextComplete
        )
    }

    /// The typed payload (input) structure this event provides.
    pub fn payload_type(&self) -> PayloadType {
        match self {
            Self::Event => PayloadType::BusEvent,
            Self::ToolExecuteBefore => PayloadType::ToolExecuteBeforeInput,
            Self::ToolExecuteAfter => PayloadType::ToolExecuteAfterInput,
            Self::ToolDefinition => PayloadType::ToolDefinitionInput,
            Self::ShellEnv => PayloadType::ShellEnvInput,
            Self::ChatMessage => PayloadType::ChatMessageInput,
            Self::ChatParams => PayloadType::ChatParamsInput,
            Self::ChatHeaders => PayloadType::ChatHeadersInput,
            Self::PermissionAsk => PayloadType::PermissionAskInput,
            Self::CommandExecuteBefore => PayloadType::CommandExecuteBeforeInput,
            Self::Config => PayloadType::ConfigInput,
            Self::Auth => PayloadType::AuthHook,
            Self::ExperimentalChatSystemTransform => PayloadType::SystemTransformInput,
            Self::ExperimentalChatMessagesTransform => PayloadType::MessagesTransformInput,
            Self::ExperimentalSessionCompacting => PayloadType::SessionCompactingInput,
            Self::ExperimentalTextComplete => PayloadType::TextCompleteInput,
        }
    }

    /// The typed response (output) structure this event expects.
    pub fn response_type(&self) -> ResponseType {
        match self {
            Self::Event => ResponseType::FireAndForget,
            Self::ToolExecuteBefore => ResponseType::MutableArgs,
            Self::ToolExecuteAfter => ResponseType::MutableToolOutput,
            Self::ToolDefinition => ResponseType::MutableToolDefinition,
            Self::ShellEnv => ResponseType::MutableEnv,
            Self::ChatMessage => ResponseType::MutableMessage,
            Self::ChatParams => ResponseType::MutableParams,
            Self::ChatHeaders => ResponseType::MutableHeaders,
            Self::PermissionAsk => ResponseType::PermissionDecision,
            Self::CommandExecuteBefore => ResponseType::MutableParts,
            Self::Config => ResponseType::FireAndForget,
            Self::Auth => ResponseType::AuthRegistration,
            Self::ExperimentalChatSystemTransform => ResponseType::MutableSystemPrompt,
            Self::ExperimentalChatMessagesTransform => ResponseType::MutableMessages,
            Self::ExperimentalSessionCompacting => ResponseType::MutableCompactionPrompt,
            Self::ExperimentalTextComplete => ResponseType::MutableText,
        }
    }

    /// The flow control pattern used by this event.
    pub fn flow_pattern(&self) -> FlowPattern {
        match self {
            Self::Event | Self::Config => FlowPattern::Informational,
            Self::ToolExecuteBefore => FlowPattern::MutateOrThrow,
            Self::PermissionAsk => FlowPattern::StatusDecision,
            Self::Auth => FlowPattern::Registration,
            Self::ToolExecuteAfter
            | Self::ToolDefinition
            | Self::ShellEnv
            | Self::ChatMessage
            | Self::ChatParams
            | Self::ChatHeaders
            | Self::CommandExecuteBefore
            | Self::ExperimentalChatSystemTransform
            | Self::ExperimentalChatMessagesTransform
            | Self::ExperimentalSessionCompacting
            | Self::ExperimentalTextComplete => FlowPattern::MutateOnly,
        }
    }
}

impl fmt::Display for OpenCodeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native hook names
// ---------------------------------------------------------------------------

impl TryFrom<&str> for OpenCodeEvent {
    type Error = UnknownEventError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "event" => Ok(Self::Event),
            "tool.execute.before" => Ok(Self::ToolExecuteBefore),
            "tool.execute.after" => Ok(Self::ToolExecuteAfter),
            "tool.definition" => Ok(Self::ToolDefinition),
            "shell.env" => Ok(Self::ShellEnv),
            "chat.message" => Ok(Self::ChatMessage),
            "chat.params" => Ok(Self::ChatParams),
            "chat.headers" => Ok(Self::ChatHeaders),
            "permission.ask" => Ok(Self::PermissionAsk),
            "command.execute.before" => Ok(Self::CommandExecuteBefore),
            "config" => Ok(Self::Config),
            "auth" => Ok(Self::Auth),
            "experimental.chat.system.transform" => Ok(Self::ExperimentalChatSystemTransform),
            "experimental.chat.messages.transform" => Ok(Self::ExperimentalChatMessagesTransform),
            "experimental.session.compacting" => Ok(Self::ExperimentalSessionCompacting),
            "experimental.text.complete" => Ok(Self::ExperimentalTextComplete),
            _ => Err(UnknownEventError(name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known OpenCode event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown OpenCode event: {0}")]
pub struct UnknownEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

/// Maps an OpenCode event to the closest unified `AgenticEvent`.
///
/// OpenCode has several hooks with no unified equivalent (tool definition
/// modification, shell env injection, LLM parameter tuning, HTTP header
/// injection, auth registration, command hooks). These return `None`.
///
/// The `Event` hook is special: it is a catch-all bus that can represent
/// *any* unified event depending on the `event.type` field. This mapping
/// returns `None` for `Event` because the correct mapping depends on the
/// inner event type, which requires inspecting the payload.
impl OpenCodeEvent {
    /// Maps this OpenCode event to the closest unified `AgenticEvent`,
    /// or `None` if no reasonable mapping exists.
    pub fn to_agentic_event(&self) -> Option<AgenticEvent> {
        match self {
            Self::ToolExecuteBefore => Some(AgenticEvent::BeforeTool),
            Self::ToolExecuteAfter => Some(AgenticEvent::AfterTool),
            Self::PermissionAsk => Some(AgenticEvent::PermissionRequest),
            Self::ChatMessage => Some(AgenticEvent::BeforePrompt),
            Self::ChatParams | Self::ChatHeaders => Some(AgenticEvent::BeforeModel),
            Self::ExperimentalChatSystemTransform => Some(AgenticEvent::BeforeModel),
            Self::ExperimentalChatMessagesTransform => Some(AgenticEvent::BeforeModel),
            Self::ExperimentalSessionCompacting => Some(AgenticEvent::BeforeCompact),
            Self::ExperimentalTextComplete => Some(AgenticEvent::AfterModel),
            // These have no unified equivalent
            Self::Event
            | Self::ToolDefinition
            | Self::ShellEnv
            | Self::CommandExecuteBefore
            | Self::Config
            | Self::Auth => None,
        }
    }
}

/// Maps a bus event type string (from the `event` hook) to a unified
/// `AgenticEvent`.
///
/// The `event` hook is a catch-all that receives 40+ different event types
/// on the system bus. Only a subset maps cleanly to `AgenticEvent` variants.
pub fn bus_event_type_to_agentic(event_type: &str) -> Option<AgenticEvent> {
    match event_type {
        "session.created" => Some(AgenticEvent::SessionStart),
        "session.deleted" => Some(AgenticEvent::SessionEnd),
        "session.idle" | "session.status" => Some(AgenticEvent::TurnComplete),
        "session.error" => Some(AgenticEvent::TurnError),
        "session.compacted" => Some(AgenticEvent::BeforeCompact),
        "permission.asked" => Some(AgenticEvent::PermissionRequest),
        "question.asked" => Some(AgenticEvent::HumanInTheLoop),
        "file.edited" => Some(AgenticEvent::Notification),
        "tui.toast.show" => Some(AgenticEvent::Notification),
        _ => None,
    }
}

impl TryFrom<AgenticEvent> for OpenCodeEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified to OpenCode event.
    ///
    /// Many unified events have no single OpenCode hook equivalent because
    /// OpenCode distributes functionality across its plugin system
    /// differently. For example, `SessionStart` and `SessionEnd` are only
    /// available through the catch-all `event` bus, not as dedicated hooks.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::BeforeTool => Ok(OpenCodeEvent::ToolExecuteBefore),
            AgenticEvent::AfterTool => Ok(OpenCodeEvent::ToolExecuteAfter),
            AgenticEvent::PermissionRequest => Ok(OpenCodeEvent::PermissionAsk),
            AgenticEvent::BeforePrompt => Ok(OpenCodeEvent::ChatMessage),
            AgenticEvent::BeforeModel => Ok(OpenCodeEvent::ChatParams),
            AgenticEvent::AfterModel => Ok(OpenCodeEvent::ExperimentalTextComplete),
            AgenticEvent::BeforeCompact => Ok(OpenCodeEvent::ExperimentalSessionCompacting),
            // These unified events can only be observed via the catch-all Event bus
            AgenticEvent::SessionStart
            | AgenticEvent::SessionEnd
            | AgenticEvent::TurnComplete
            | AgenticEvent::TurnError
            | AgenticEvent::Notification
            | AgenticEvent::HumanInTheLoop => {
                Err("only available via OpenCode's catch-all event bus, not a dedicated hook")
            }
            // These have no OpenCode equivalent at all
            AgenticEvent::ToolError => {
                Err("OpenCode has no tool error hook; tool.execute.after only fires on success")
            }
            AgenticEvent::SubagentStart | AgenticEvent::SubagentStop => {
                Err("OpenCode does not expose subagent lifecycle as dedicated hooks")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// How the hook controls agent flow.
///
/// OpenCode's flow control is fundamentally different from Claude Code.
/// Instead of exit codes and JSON responses, OpenCode uses in-process
/// mutation and exceptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowPattern {
    /// Cannot affect flow. Fire-and-forget observation only.
    /// Used by: `event`, `config`.
    Informational,
    /// Mutate the output object to change behavior. Cannot block.
    /// Used by most hooks (tool.execute.after, chat.*, shell.env, etc.).
    MutateOnly,
    /// Mutate the output object OR throw an Error to block the action.
    /// The error message is returned to the agent as feedback.
    /// Used by: `tool.execute.before`.
    MutateOrThrow,
    /// Mutate a status field to allow/deny/ask.
    /// Used by: `permission.ask`.
    StatusDecision,
    /// Structured registration (not a simple input/output hook).
    /// Used by: `auth`.
    Registration,
}

/// Discriminant for the event-specific input payload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    /// Discriminated union of 40+ system bus events.
    BusEvent,
    /// Tool name, session ID, call ID (read-only).
    ToolExecuteBeforeInput,
    /// Tool name, session ID, call ID, args (read-only).
    ToolExecuteAfterInput,
    /// Tool ID (read-only).
    ToolDefinitionInput,
    /// Current working directory (read-only).
    ShellEnvInput,
    /// Session ID, agent, model, message ID, variant (read-only).
    ChatMessageInput,
    /// Session ID, agent, model, provider, message (read-only).
    ChatParamsInput,
    /// Session ID, agent, model, provider, message (read-only).
    ChatHeadersInput,
    /// Permission request details: tool name, patterns, metadata (read-only).
    PermissionAskInput,
    /// Command name, session ID, arguments (read-only).
    CommandExecuteBeforeInput,
    /// Full merged OpenCode configuration object.
    ConfigInput,
    /// Structured auth hook definition (provider, methods, loader).
    AuthHook,
    /// Session ID and model (read-only).
    SystemTransformInput,
    /// Empty input (read-only).
    MessagesTransformInput,
    /// Session ID (read-only).
    SessionCompactingInput,
    /// Session ID, message ID, part ID (read-only).
    TextCompleteInput,
}

/// Discriminant for the event-specific response (output) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    /// No output processed. Observation only.
    FireAndForget,
    /// Mutable `{ args: any }` - tool arguments before execution.
    MutableArgs,
    /// Mutable `{ title, output, metadata }` - tool result after execution.
    MutableToolOutput,
    /// Mutable `{ description, parameters }` - tool definition for LLM.
    MutableToolDefinition,
    /// Mutable `{ env: Record<string, string> }` - environment variables.
    MutableEnv,
    /// Mutable `{ message, parts }` - user message content.
    MutableMessage,
    /// Mutable `{ temperature, topP, topK, options }` - LLM params.
    MutableParams,
    /// Mutable `{ headers: Record<string, string> }` - HTTP headers.
    MutableHeaders,
    /// Mutable `{ status: "ask" | "deny" | "allow" }` - permission decision.
    PermissionDecision,
    /// Mutable `{ parts: Part[] }` - command output parts.
    MutableParts,
    /// Structured auth provider registration.
    AuthRegistration,
    /// Mutable `{ system: string[] }` - system prompt strings.
    MutableSystemPrompt,
    /// Mutable `{ messages }` - full conversation history.
    MutableMessages,
    /// Mutable `{ context: string[], prompt?: string }` - compaction config.
    MutableCompactionPrompt,
    /// Mutable `{ text: string }` - completed text output.
    MutableText,
}

// ---------------------------------------------------------------------------
// Event-specific input payloads
// ---------------------------------------------------------------------------

/// Input for the `event` hook -- a discriminated union of bus events.
///
/// The `type` field identifies which of the 40+ event types this is.
/// The `properties` field contains event-specific data whose shape varies
/// by type. Callers must filter by `event_type` before interpreting
/// `properties`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEventPayload {
    /// Discriminator for the event type (e.g., "session.created",
    /// "file.edited", "permission.asked").
    #[serde(rename = "type")]
    pub event_type: String,

    /// Event-specific data. Shape varies by `event_type`.
    #[serde(default)]
    pub properties: Value,
}

impl BusEventPayload {
    /// Returns the bus event category (the part before the first dot).
    ///
    /// ## Examples
    ///
    /// - `"session.created"` -> `"session"`
    /// - `"file.watcher.updated"` -> `"file"`
    /// - `"tui.toast.show"` -> `"tui"`
    pub fn category(&self) -> &str {
        self.event_type
            .split('.')
            .next()
            .unwrap_or(&self.event_type)
    }

    /// Maps this bus event to the closest unified `AgenticEvent`.
    pub fn to_agentic_event(&self) -> Option<AgenticEvent> {
        bus_event_type_to_agentic(&self.event_type)
    }
}

/// All known bus event type strings in OpenCode.
///
/// These are the `event.type` values that the `event` hook can receive.
/// This enum is non-exhaustive because OpenCode may add new event types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BusEventType {
    // Command
    CommandExecuted,
    // File
    FileEdited,
    FileWatcherUpdated,
    // IDE
    IdeInstalled,
    // Installation
    InstallationUpdated,
    InstallationUpdateAvailable,
    // LSP
    LspClientDiagnostics,
    LspUpdated,
    // MCP
    McpToolsChanged,
    McpBrowserOpenFailed,
    // Message
    MessageUpdated,
    MessageRemoved,
    MessagePartUpdated,
    MessagePartDelta,
    MessagePartRemoved,
    // Permission
    PermissionAsked,
    PermissionReplied,
    // Project
    ProjectUpdated,
    // PTY
    PtyCreated,
    PtyUpdated,
    PtyExited,
    PtyDeleted,
    // Question (human-in-the-loop)
    QuestionAsked,
    QuestionReplied,
    QuestionRejected,
    // Server
    ServerConnected,
    ServerInstanceDisposed,
    GlobalDisposed,
    // Session
    SessionCreated,
    SessionUpdated,
    SessionDeleted,
    SessionStatus,
    SessionIdle,
    SessionCompacted,
    SessionDiff,
    SessionError,
    // Todo
    TodoUpdated,
    // TUI
    TuiPromptAppend,
    TuiCommandExecute,
    TuiToastShow,
    TuiSessionSelect,
    // VCS
    VcsBranchUpdated,
    // Worktree
    WorktreeReady,
    WorktreeFailed,
}

impl BusEventType {
    /// The dot-separated event type string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CommandExecuted => "command.executed",
            Self::FileEdited => "file.edited",
            Self::FileWatcherUpdated => "file.watcher.updated",
            Self::IdeInstalled => "ide.installed",
            Self::InstallationUpdated => "installation.updated",
            Self::InstallationUpdateAvailable => "installation.update-available",
            Self::LspClientDiagnostics => "lsp.client.diagnostics",
            Self::LspUpdated => "lsp.updated",
            Self::McpToolsChanged => "mcp.tools.changed",
            Self::McpBrowserOpenFailed => "mcp.browser.open.failed",
            Self::MessageUpdated => "message.updated",
            Self::MessageRemoved => "message.removed",
            Self::MessagePartUpdated => "message.part.updated",
            Self::MessagePartDelta => "message.part.delta",
            Self::MessagePartRemoved => "message.part.removed",
            Self::PermissionAsked => "permission.asked",
            Self::PermissionReplied => "permission.replied",
            Self::ProjectUpdated => "project.updated",
            Self::PtyCreated => "pty.created",
            Self::PtyUpdated => "pty.updated",
            Self::PtyExited => "pty.exited",
            Self::PtyDeleted => "pty.deleted",
            Self::QuestionAsked => "question.asked",
            Self::QuestionReplied => "question.replied",
            Self::QuestionRejected => "question.rejected",
            Self::ServerConnected => "server.connected",
            Self::ServerInstanceDisposed => "server.instance.disposed",
            Self::GlobalDisposed => "global.disposed",
            Self::SessionCreated => "session.created",
            Self::SessionUpdated => "session.updated",
            Self::SessionDeleted => "session.deleted",
            Self::SessionStatus => "session.status",
            Self::SessionIdle => "session.idle",
            Self::SessionCompacted => "session.compacted",
            Self::SessionDiff => "session.diff",
            Self::SessionError => "session.error",
            Self::TodoUpdated => "todo.updated",
            Self::TuiPromptAppend => "tui.prompt.append",
            Self::TuiCommandExecute => "tui.command.execute",
            Self::TuiToastShow => "tui.toast.show",
            Self::TuiSessionSelect => "tui.session.select",
            Self::VcsBranchUpdated => "vcs.branch.updated",
            Self::WorktreeReady => "worktree.ready",
            Self::WorktreeFailed => "worktree.failed",
        }
    }
}

impl TryFrom<&str> for BusEventType {
    type Error = UnknownBusEventError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "command.executed" => Ok(Self::CommandExecuted),
            "file.edited" => Ok(Self::FileEdited),
            "file.watcher.updated" => Ok(Self::FileWatcherUpdated),
            "ide.installed" => Ok(Self::IdeInstalled),
            "installation.updated" => Ok(Self::InstallationUpdated),
            "installation.update-available" => Ok(Self::InstallationUpdateAvailable),
            "lsp.client.diagnostics" => Ok(Self::LspClientDiagnostics),
            "lsp.updated" => Ok(Self::LspUpdated),
            "mcp.tools.changed" => Ok(Self::McpToolsChanged),
            "mcp.browser.open.failed" => Ok(Self::McpBrowserOpenFailed),
            "message.updated" => Ok(Self::MessageUpdated),
            "message.removed" => Ok(Self::MessageRemoved),
            "message.part.updated" => Ok(Self::MessagePartUpdated),
            "message.part.delta" => Ok(Self::MessagePartDelta),
            "message.part.removed" => Ok(Self::MessagePartRemoved),
            "permission.asked" => Ok(Self::PermissionAsked),
            "permission.replied" => Ok(Self::PermissionReplied),
            "project.updated" => Ok(Self::ProjectUpdated),
            "pty.created" => Ok(Self::PtyCreated),
            "pty.updated" => Ok(Self::PtyUpdated),
            "pty.exited" => Ok(Self::PtyExited),
            "pty.deleted" => Ok(Self::PtyDeleted),
            "question.asked" => Ok(Self::QuestionAsked),
            "question.replied" => Ok(Self::QuestionReplied),
            "question.rejected" => Ok(Self::QuestionRejected),
            "server.connected" => Ok(Self::ServerConnected),
            "server.instance.disposed" => Ok(Self::ServerInstanceDisposed),
            "global.disposed" => Ok(Self::GlobalDisposed),
            "session.created" => Ok(Self::SessionCreated),
            "session.updated" => Ok(Self::SessionUpdated),
            "session.deleted" => Ok(Self::SessionDeleted),
            "session.status" => Ok(Self::SessionStatus),
            "session.idle" => Ok(Self::SessionIdle),
            "session.compacted" => Ok(Self::SessionCompacted),
            "session.diff" => Ok(Self::SessionDiff),
            "session.error" => Ok(Self::SessionError),
            "todo.updated" => Ok(Self::TodoUpdated),
            "tui.prompt.append" => Ok(Self::TuiPromptAppend),
            "tui.command.execute" => Ok(Self::TuiCommandExecute),
            "tui.toast.show" => Ok(Self::TuiToastShow),
            "tui.session.select" => Ok(Self::TuiSessionSelect),
            "vcs.branch.updated" => Ok(Self::VcsBranchUpdated),
            "worktree.ready" => Ok(Self::WorktreeReady),
            "worktree.failed" => Ok(Self::WorktreeFailed),
            _ => Err(UnknownBusEventError(s.to_string())),
        }
    }
}

/// Error returned when a string does not match any known bus event type.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown OpenCode bus event type: {0}")]
pub struct UnknownBusEventError(pub String);

// ---------------------------------------------------------------------------
// Input payload structs for dedicated hooks
// ---------------------------------------------------------------------------

/// Input for `tool.execute.before`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecuteBeforeInput {
    /// Tool name (e.g., "bash", "edit", "write", "read", "glob",
    /// "grep", "task", or custom tool IDs).
    pub tool: String,

    /// Current session ID.
    #[serde(rename = "sessionID")]
    pub session_id: String,

    /// Unique tool call ID.
    #[serde(rename = "callID")]
    pub call_id: String,
}

/// Mutable output for `tool.execute.before`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecuteBeforeOutput {
    /// Mutable tool arguments. Shape varies by tool.
    pub args: Value,
}

/// Input for `tool.execute.after`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecuteAfterInput {
    /// Tool name.
    pub tool: String,

    /// Current session ID.
    #[serde(rename = "sessionID")]
    pub session_id: String,

    /// Tool call ID.
    #[serde(rename = "callID")]
    pub call_id: String,

    /// The arguments that were passed to the tool (read-only at this point).
    pub args: Value,
}

/// Mutable output for `tool.execute.after`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecuteAfterOutput {
    /// Display title for the tool result.
    pub title: String,

    /// Output string shown to the agent.
    pub output: String,

    /// Additional metadata.
    pub metadata: Value,
}

/// Input for `tool.definition`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionInput {
    /// The tool identifier.
    #[serde(rename = "toolID")]
    pub tool_id: String,
}

/// Mutable output for `tool.definition`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionOutput {
    /// Tool description sent to the LLM.
    pub description: String,

    /// JSON Schema parameters object sent to the LLM.
    pub parameters: Value,
}

/// Input for `shell.env`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEnvInput {
    /// Current working directory.
    pub cwd: String,
}

/// Mutable output for `shell.env`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEnvOutput {
    /// Environment variables to inject. Merged with `process.env`; hook
    /// values override existing variables.
    pub env: std::collections::HashMap<String, String>,
}

/// Input for `chat.message`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageInput {
    /// Current session ID.
    #[serde(rename = "sessionID")]
    pub session_id: String,

    /// Agent name, if applicable.
    #[serde(default)]
    pub agent: Option<String>,

    /// Model provider and ID, if known.
    #[serde(default)]
    pub model: Option<ModelRef>,

    /// Message ID, if applicable.
    #[serde(default, rename = "messageID")]
    pub message_id: Option<String>,

    /// Message variant, if applicable.
    #[serde(default)]
    pub variant: Option<String>,
}

/// Model reference with provider and model ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: String,

    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// Mutable output for `chat.message`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageOutput {
    /// The user message object.
    pub message: Value,

    /// Message parts array.
    pub parts: Vec<Value>,
}

/// Input for `chat.params`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatParamsInput {
    /// Current session ID.
    #[serde(rename = "sessionID")]
    pub session_id: String,

    /// Agent name.
    pub agent: String,

    /// Model information.
    pub model: Value,

    /// Provider context.
    pub provider: Value,

    /// The user message.
    pub message: Value,
}

/// Mutable output for `chat.params`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatParamsOutput {
    /// LLM temperature.
    pub temperature: f64,

    /// Top-p sampling parameter.
    #[serde(rename = "topP")]
    pub top_p: f64,

    /// Top-k sampling parameter.
    #[serde(rename = "topK")]
    pub top_k: f64,

    /// Additional provider-specific options.
    #[serde(default)]
    pub options: std::collections::HashMap<String, Value>,
}

/// Mutable output for `chat.headers`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHeadersOutput {
    /// HTTP headers to inject into the LLM API request.
    pub headers: std::collections::HashMap<String, String>,
}

/// Input for `permission.ask`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAskInput {
    /// The permission type / tool name.
    pub permission: String,

    /// Additional metadata (tool-specific patterns, command text, etc.).
    #[serde(default)]
    pub metadata: Value,
}

/// Mutable output for `permission.ask`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionAskOutput {
    /// Permission decision. Defaults to "ask".
    pub status: PermissionStatus,
}

/// Three-level permission status for `permission.ask`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionStatus {
    /// Prompt the user for a decision (default).
    Ask,
    /// Auto-approve the permission.
    Allow,
    /// Auto-reject the permission (throws RejectedError to agent).
    Deny,
}

/// Input for `command.execute.before`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecuteBeforeInput {
    /// Command name.
    pub command: String,

    /// Current session ID.
    #[serde(rename = "sessionID")]
    pub session_id: String,

    /// Command arguments as a string.
    pub arguments: String,
}

/// Mutable output for `command.execute.before`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecuteBeforeOutput {
    /// Mutable parts array for command output.
    pub parts: Vec<Value>,
}

/// Input for `experimental.chat.system.transform`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTransformInput {
    /// Optional session ID.
    #[serde(default, rename = "sessionID")]
    pub session_id: Option<String>,

    /// Model information.
    pub model: Value,
}

/// Mutable output for `experimental.chat.system.transform`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTransformOutput {
    /// System prompt strings. Push to add context; if emptied entirely,
    /// OpenCode restores the original system prompt (safety mechanism).
    pub system: Vec<String>,
}

/// Mutable output for `experimental.chat.messages.transform`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesTransformOutput {
    /// Full conversation history. Each entry has `info` (Message) and
    /// `parts` (Part[]).
    pub messages: Vec<MessageEntry>,
}

/// A single message entry in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    /// Message metadata.
    pub info: Value,

    /// Message parts.
    pub parts: Vec<Value>,
}

/// Input for `experimental.session.compacting`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCompactingInput {
    /// Session being compacted.
    #[serde(rename = "sessionID")]
    pub session_id: String,
}

/// Mutable output for `experimental.session.compacting`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCompactingOutput {
    /// Context strings appended to the default compaction prompt.
    pub context: Vec<String>,

    /// If set, replaces the default compaction prompt entirely
    /// (ignores `context`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

/// Input for `experimental.text.complete`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextCompleteInput {
    /// Session ID.
    #[serde(rename = "sessionID")]
    pub session_id: String,

    /// Message ID.
    #[serde(rename = "messageID")]
    pub message_id: String,

    /// Part ID within the message.
    #[serde(rename = "partID")]
    pub part_id: String,
}

/// Mutable output for `experimental.text.complete`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextCompleteOutput {
    /// The completed text (already trimmed). Mutable for post-processing.
    pub text: String,
}

// ---------------------------------------------------------------------------
// Auth hook structures
// ---------------------------------------------------------------------------

/// The structured auth hook definition.
///
/// Unlike other hooks, `auth` is not a simple input/output function.
/// It is a structured object that registers authentication methods with
/// the OpenCode auth system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthHookDef {
    /// Provider name this auth handles.
    pub provider: String,

    /// Authentication methods (OAuth or API key flows).
    pub methods: Vec<AuthMethod>,
}

/// An authentication method supported by the auth hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    /// OAuth flow (auto-callback or code-based).
    #[serde(rename = "oauth")]
    OAuth {
        /// Display name for this method.
        name: String,
        /// OAuth configuration details.
        #[serde(flatten)]
        config: Value,
    },
    /// API key flow with optional interactive prompts.
    #[serde(rename = "api_key")]
    ApiKey {
        /// Display name for this method.
        name: String,
        /// API key configuration details.
        #[serde(flatten)]
        config: Value,
    },
}

// ---------------------------------------------------------------------------
// Unified input payload enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any OpenCode hook's input payload.
///
/// Because OpenCode hooks are JavaScript function calls (not stdin/stdout),
/// this enum represents the Rust-side modeling of what each hook receives.
/// In practice, Claudine intercepts these at the bridge layer between
/// the OpenCode plugin runtime and the unified event system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_hook_type")]
pub enum OpenCodeInput {
    Event(BusEventPayload),
    ToolExecuteBefore {
        input: ToolExecuteBeforeInput,
        output: ToolExecuteBeforeOutput,
    },
    ToolExecuteAfter {
        input: ToolExecuteAfterInput,
        output: ToolExecuteAfterOutput,
    },
    ToolDefinition {
        input: ToolDefinitionInput,
        output: ToolDefinitionOutput,
    },
    ShellEnv {
        input: ShellEnvInput,
        output: ShellEnvOutput,
    },
    ChatMessage {
        input: ChatMessageInput,
        output: ChatMessageOutput,
    },
    ChatParams {
        input: ChatParamsInput,
        output: ChatParamsOutput,
    },
    ChatHeaders {
        input: ChatParamsInput,
        output: ChatHeadersOutput,
    },
    PermissionAsk {
        input: PermissionAskInput,
        output: PermissionAskOutput,
    },
    CommandExecuteBefore {
        input: CommandExecuteBeforeInput,
        output: CommandExecuteBeforeOutput,
    },
    Config(Value),
    Auth(AuthHookDef),
    ExperimentalSystemTransform {
        input: SystemTransformInput,
        output: SystemTransformOutput,
    },
    ExperimentalMessagesTransform {
        output: MessagesTransformOutput,
    },
    ExperimentalSessionCompacting {
        input: SessionCompactingInput,
        output: SessionCompactingOutput,
    },
    ExperimentalTextComplete {
        input: TextCompleteInput,
        output: TextCompleteOutput,
    },
}

impl OpenCodeInput {
    /// Returns the event variant for this input payload.
    pub fn event(&self) -> OpenCodeEvent {
        match self {
            Self::Event(_) => OpenCodeEvent::Event,
            Self::ToolExecuteBefore { .. } => OpenCodeEvent::ToolExecuteBefore,
            Self::ToolExecuteAfter { .. } => OpenCodeEvent::ToolExecuteAfter,
            Self::ToolDefinition { .. } => OpenCodeEvent::ToolDefinition,
            Self::ShellEnv { .. } => OpenCodeEvent::ShellEnv,
            Self::ChatMessage { .. } => OpenCodeEvent::ChatMessage,
            Self::ChatParams { .. } => OpenCodeEvent::ChatParams,
            Self::ChatHeaders { .. } => OpenCodeEvent::ChatHeaders,
            Self::PermissionAsk { .. } => OpenCodeEvent::PermissionAsk,
            Self::CommandExecuteBefore { .. } => OpenCodeEvent::CommandExecuteBefore,
            Self::Config(_) => OpenCodeEvent::Config,
            Self::Auth(_) => OpenCodeEvent::Auth,
            Self::ExperimentalSystemTransform { .. } => {
                OpenCodeEvent::ExperimentalChatSystemTransform
            }
            Self::ExperimentalMessagesTransform { .. } => {
                OpenCodeEvent::ExperimentalChatMessagesTransform
            }
            Self::ExperimentalSessionCompacting { .. } => {
                OpenCodeEvent::ExperimentalSessionCompacting
            }
            Self::ExperimentalTextComplete { .. } => OpenCodeEvent::ExperimentalTextComplete,
        }
    }

    /// Returns the session ID if available from the payload.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Self::ToolExecuteBefore { input, .. } => Some(&input.session_id),
            Self::ToolExecuteAfter { input, .. } => Some(&input.session_id),
            Self::ChatMessage { input, .. } => Some(&input.session_id),
            Self::ChatParams { input, .. } => Some(&input.session_id),
            Self::ChatHeaders { input, .. } => Some(&input.session_id),
            Self::CommandExecuteBefore { input, .. } => Some(&input.session_id),
            Self::ExperimentalSystemTransform { input, .. } => input.session_id.as_deref(),
            Self::ExperimentalSessionCompacting { input, .. } => Some(&input.session_id),
            Self::ExperimentalTextComplete { input, .. } => Some(&input.session_id),
            // Event bus may have sessionID in properties but it's not uniform
            _ => None,
        }
    }

    /// Returns the tool name if this is a tool-related hook.
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::ToolExecuteBefore { input, .. } => Some(&input.tool),
            Self::ToolExecuteAfter { input, .. } => Some(&input.tool),
            Self::ToolDefinition { input, .. } => Some(&input.tool_id),
            Self::PermissionAsk { input, .. } => Some(&input.permission),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin context (modeled for Rust-side awareness)
// ---------------------------------------------------------------------------

/// The context object passed to the plugin initialization function.
///
/// In OpenCode's TypeScript runtime, the plugin receives this when loaded.
/// For Claudine's Rust-side modeling, this captures what contextual
/// information is available during plugin initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// Current working directory.
    pub directory: String,

    /// Git worktree path.
    pub worktree: String,

    /// Local OpenCode server URL.
    #[serde(rename = "serverUrl")]
    pub server_url: String,
}
```

## Design Considerations

- **Fundamentally different execution model.** OpenCode hooks are in-process JavaScript/TypeScript plugin functions that receive mutable output objects and return `Promise<void>`. Claude Code hooks are external processes that receive JSON on stdin and return JSON on stdout with exit codes. This means OpenCode has no concept of exit codes for flow control; instead it uses exception throwing (`tool.execute.before`) and status field mutation (`permission.ask`) for blocking. The design reflects this by using `FlowPattern` instead of Claude Code's `DecisionPattern` and `ExitCode`.

- **Input/output pairs instead of single payloads.** OpenCode hooks receive two separate objects: a read-only `input` and a mutable `output`. The design models both sides as separate structs (e.g., `ToolExecuteBeforeInput` + `ToolExecuteBeforeOutput`) rather than Claude Code's single-payload approach. The `OpenCodeInput` dispatch enum captures both halves together for routing purposes.

- **The `event` hook is a catch-all bus, not a single event.** Unlike Claude Code where each event is a distinct hook, OpenCode funnels 40+ system bus events through a single `event` hook. The design models this with `BusEventPayload` (containing a `type` discriminator and freeform `properties`) plus a comprehensive `BusEventType` enum for all known event type strings. The `bus_event_type_to_agentic()` function provides the bridge from bus event types to unified events.

- **No tool error hook.** OpenCode has no equivalent to Claude Code's `PostToolUseFailure`. The `tool.execute.after` hook only fires on success. Tool errors can only be observed indirectly via the `session.error` bus event. This is an explicit gap documented in the mapping.

- **No subagent lifecycle hooks.** OpenCode does not expose subagent start/stop as dedicated hooks. The `task` tool (which spawns subagents) is interceptable via `tool.execute.before`/`tool.execute.after`, but there is no dedicated subagent lifecycle notification.

- **No session start/end hooks.** Unlike Claude Code's `SessionStart`/`SessionEnd` hooks, OpenCode only exposes session lifecycle through the bus event system (`session.created`, `session.deleted`). These are fire-and-forget observations; they cannot inject context or block.

- **Experimental hooks modeled as first-class variants.** OpenCode's four `experimental.*` hooks are included as full enum variants rather than being relegated to a separate type. The `is_experimental()` method flags them for callers that need stability guarantees.

- **Auth hook is structurally distinct.** The `auth` hook is not a simple input/output function; it is a structured registration object with provider names and authentication methods. The design models this as `AuthHookDef` with `AuthMethod` variants, acknowledging that it does not fit the standard hook pattern.

- **16 hooks vs Claude Code's 14.** OpenCode has 16 distinct hook types (including 4 experimental), compared to Claude Code's 14. However, the functional overlap is smaller than it appears because OpenCode has many hooks targeting LLM request/response shaping (chat.params, chat.headers, chat.message, system transform, messages transform) that have no Claude Code equivalent, while Claude Code has session lifecycle and subagent hooks that OpenCode lacks as dedicated hooks.

- **`to_agentic_event()` returns `Option` instead of total `From`.** Unlike Claude Code where every event maps to some `AgenticEvent` (even if lossy), OpenCode has 6 hooks with no reasonable unified mapping (`Event`, `ToolDefinition`, `ShellEnv`, `CommandExecuteBefore`, `Config`, `Auth`). Returning `Option` is more honest than forcing a lossy mapping.

- **`BusEventType` is `#[non_exhaustive]`.** OpenCode's bus event system is extensible; new event types may appear in future versions without being breaking changes. The enum is marked non-exhaustive to force callers to handle unknown variants.

- **No matcher system needed.** OpenCode has no declarative matcher/pattern system. All filtering is done programmatically inside hook functions. The design omits any `MatcherField` equivalent because it would be meaningless for OpenCode.

## Claude Code Mapping

- **`OpenCodeEvent::ToolExecuteBefore`** -- Maps to `ClaudeCodeEvent::PreToolUse`. Same trigger (before tool execution), similar payload (tool name, call ID, args). Key difference: OpenCode blocks by throwing an error; Claude Code blocks via exit code 2 or `permissionDecision: "deny"`. OpenCode's output is mutable args; Claude Code uses `updatedInput` in the response JSON.

- **`OpenCodeEvent::ToolExecuteAfter`** -- Maps to `ClaudeCodeEvent::PostToolUse`. Same trigger (after successful tool execution). OpenCode provides mutable `title`/`output`/`metadata`; Claude Code provides `tool_response` in the payload and `additionalContext`/`updatedMCPToolOutput` in the response. OpenCode can directly rewrite what the agent sees; Claude Code can only add context.

- **`OpenCodeEvent::ToolDefinition`** -- No Claude Code equivalent. This is entirely unique to OpenCode. Allows modifying tool descriptions and parameter schemas before they are sent to the LLM.

- **`OpenCodeEvent::ShellEnv`** -- Partially maps to `ClaudeCodeEvent::SessionStart` with `CLAUDE_ENV_FILE`. Claude Code allows injecting environment variables only at session start via a file; OpenCode injects them on every shell invocation via a hook. Different trigger, different mechanism, similar intent.

- **`OpenCodeEvent::ChatMessage`** -- Maps to `ClaudeCodeEvent::UserPromptSubmit`. Same trigger (user message before LLM processing). OpenCode provides richer input (agent, model, parts) and allows mutation of message parts. Claude Code provides the prompt text and supports blocking via decision field.

- **`OpenCodeEvent::ChatParams`** -- No Claude Code equivalent. Allows per-request modification of LLM sampling parameters (temperature, topP, topK). Claude Code has no hook for this.

- **`OpenCodeEvent::ChatHeaders`** -- No Claude Code equivalent. Allows injecting custom HTTP headers into LLM API requests. Claude Code has no hook for this.

- **`OpenCodeEvent::PermissionAsk`** -- Maps to `ClaudeCodeEvent::PermissionRequest`. Same trigger (permission decision needed). Both support allow/deny. Key difference: OpenCode's hook only fires when permission evaluates to "ask" (cannot override "allow" or "deny" rules), while Claude Code's fires for all permission prompts. OpenCode uses a three-level status (`ask`/`allow`/`deny`); Claude Code uses a nested `decision.behavior` object.

- **`OpenCodeEvent::CommandExecuteBefore`** -- No Claude Code equivalent. Fires before custom slash commands. Claude Code has no hook system for slash commands.

- **`OpenCodeEvent::Config`** -- No Claude Code equivalent. Read-only access to merged configuration at startup. Claude Code has no startup config hook.

- **`OpenCodeEvent::Auth`** -- No Claude Code equivalent. Registers custom authentication providers. Claude Code handles auth entirely outside the hooks system.

- **`OpenCodeEvent::Event`** -- Partially maps to multiple Claude Code events. The catch-all bus contains events that correspond to `SessionStart` (`session.created`), `SessionEnd` (`session.deleted`), `Notification` (`tui.toast.show`, `file.edited`), and others. However, they are all fire-and-forget (cannot block or inject context), unlike Claude Code's dedicated hooks which can. The bus also contains many events with no Claude Code equivalent (LSP diagnostics, PTY lifecycle, worktree events, installation updates, etc.).

- **`OpenCodeEvent::ExperimentalChatSystemTransform`** -- No direct Claude Code equivalent. Allows modifying the system prompt. The closest Claude Code mechanism is context injection via `SessionStart` hooks, but that adds to the conversation, not the system prompt.

- **`OpenCodeEvent::ExperimentalChatMessagesTransform`** -- No Claude Code equivalent. Allows rewriting the entire message history before LLM invocation. No Claude Code hook operates at this level.

- **`OpenCodeEvent::ExperimentalSessionCompacting`** -- Maps to `ClaudeCodeEvent::PreCompact`. Same trigger (before context compaction). OpenCode allows customizing the compaction prompt and injecting context; Claude Code's `PreCompact` is informational only (cannot affect compaction). OpenCode's hook is strictly more powerful.

- **`OpenCodeEvent::ExperimentalTextComplete`** -- No direct Claude Code equivalent. Fires when text generation completes, allowing post-processing. The closest Claude Code mechanism is the `Stop` hook, but that fires at the turn level, not at the individual text-part level.

- **Claude Code events with NO OpenCode hook equivalent:**
  - `PostToolUseFailure` -- OpenCode has no tool error hook at all
  - `SubagentStart` / `SubagentStop` -- OpenCode has no subagent lifecycle hooks
  - `SessionStart` / `SessionEnd` -- Only available as fire-and-forget bus events, not as interceptable hooks
  - `Stop` -- No dedicated turn-complete hook; `session.idle` bus event is the closest
  - `TeammateIdle` / `TaskCompleted` -- No team-oriented hooks
  - `PreCompact` (as informational) -- OpenCode's compacting hook is experimental but more capable
