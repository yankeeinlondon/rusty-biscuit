# Gemini CLI Event Design

```rust
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Core event enum
// ---------------------------------------------------------------------------

/// All 11 hook events supported by Gemini CLI.
///
/// Each variant corresponds to a named lifecycle hook that Gemini CLI fires
/// at a specific point during an agentic session. The variant names use
/// Gemini CLI's native PascalCase naming (e.g., `BeforeTool`, `AfterAgent`)
/// so that matcher logic, debug output, and serialization align with the
/// official documentation.
///
/// Gemini CLI differs from Claude Code in several important ways:
/// - It has model-level hooks (`BeforeModel`, `AfterModel`) absent in Claude Code
/// - It has a tool selection hook (`BeforeToolSelection`) with no Claude Code equivalent
/// - It has `AfterAgent` for response validation with automatic retry semantics
/// - It lacks Claude Code's `PermissionRequest`, subagent, teammate, and task events
/// - It supports only `command` hook handlers (no `prompt` or `agent` types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeminiCliEvent {
    SessionStart,
    SessionEnd,
    BeforeAgent,
    AfterAgent,
    BeforeModel,
    AfterModel,
    BeforeToolSelection,
    BeforeTool,
    AfterTool,
    PreCompress,
    Notification,
}

impl GeminiCliEvent {
    /// All variants in lifecycle order.
    pub const ALL: [GeminiCliEvent; 11] = [
        Self::SessionStart,
        Self::BeforeAgent,
        Self::BeforeModel,
        Self::BeforeToolSelection,
        Self::BeforeTool,
        Self::AfterTool,
        Self::AfterModel,
        Self::AfterAgent,
        Self::PreCompress,
        Self::Notification,
        Self::SessionEnd,
    ];

    /// The native event name string as it appears in `hook_event_name`.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::BeforeAgent => "BeforeAgent",
            Self::AfterAgent => "AfterAgent",
            Self::BeforeModel => "BeforeModel",
            Self::AfterModel => "AfterModel",
            Self::BeforeToolSelection => "BeforeToolSelection",
            Self::BeforeTool => "BeforeTool",
            Self::AfterTool => "AfterTool",
            Self::PreCompress => "PreCompress",
            Self::Notification => "Notification",
        }
    }

    /// Gemini CLI-specific description of what this event represents.
    pub fn description(&self) -> &'static str {
        match self {
            Self::SessionStart => {
                "Fires on application startup, resuming a session, or after /clear"
            }
            Self::SessionEnd => {
                "Fires when the CLI exits or a session is cleared (best-effort, not awaited)"
            }
            Self::BeforeAgent => {
                "Fires after a user submits a prompt, before the agent begins planning"
            }
            Self::AfterAgent => {
                "Fires after the agent generates its final response; supports automatic retry"
            }
            Self::BeforeModel => {
                "Fires before sending a request to the LLM; can override request or inject synthetic response"
            }
            Self::AfterModel => {
                "Fires per streaming chunk after receiving LLM response; used for redaction/PII filtering"
            }
            Self::BeforeToolSelection => {
                "Fires before the LLM decides which tools to call; controls tool availability"
            }
            Self::BeforeTool => {
                "Fires before a tool executes; used for argument validation and security checks"
            }
            Self::AfterTool => {
                "Fires after a tool executes; can redact output but cannot undo the action"
            }
            Self::PreCompress => {
                "Fires before context compression/summarization (async, advisory only)"
            }
            Self::Notification => {
                "Fires when the CLI emits a system alert (e.g., tool permissions)"
            }
        }
    }

    /// Whether this event can block the action it intercepts.
    ///
    /// Blocking events support `decision: "deny"` or exit code 2.
    /// Non-blocking events are informational only.
    pub fn can_block(&self) -> bool {
        matches!(
            self,
            Self::BeforeAgent
                | Self::AfterAgent
                | Self::BeforeModel
                | Self::AfterModel
                | Self::BeforeTool
                | Self::AfterTool
        )
    }

    /// Whether this event supports regex matchers for filtering.
    ///
    /// Tool events use regex matchers against the tool name.
    /// Lifecycle events (`SessionStart`, `SessionEnd`, `PreCompress`)
    /// use exact string matchers against their source/reason/trigger.
    /// All other events fire on every occurrence.
    pub fn supports_matcher(&self) -> bool {
        matches!(
            self,
            Self::SessionStart
                | Self::SessionEnd
                | Self::BeforeTool
                | Self::AfterTool
                | Self::PreCompress
        )
    }

    /// The field in the input JSON that the matcher runs against.
    ///
    /// Returns `None` for events that do not support matchers.
    pub fn matcher_field(&self) -> Option<GeminiMatcherField> {
        match self {
            Self::BeforeTool | Self::AfterTool => Some(GeminiMatcherField::ToolName),
            Self::SessionStart => Some(GeminiMatcherField::Source),
            Self::SessionEnd => Some(GeminiMatcherField::Reason),
            Self::PreCompress => Some(GeminiMatcherField::Trigger),
            // These events do not support matchers
            Self::BeforeAgent
            | Self::AfterAgent
            | Self::BeforeModel
            | Self::AfterModel
            | Self::BeforeToolSelection
            | Self::Notification => None,
        }
    }

    /// The matcher strategy used for this event.
    ///
    /// Gemini CLI uses regex for tool events and exact string matching
    /// for lifecycle events. This differs from Claude Code which uses
    /// regex for all matchable events.
    pub fn matcher_strategy(&self) -> Option<MatcherStrategy> {
        match self {
            Self::BeforeTool | Self::AfterTool => Some(MatcherStrategy::Regex),
            Self::SessionStart | Self::SessionEnd | Self::PreCompress => {
                Some(MatcherStrategy::ExactString)
            }
            _ => None,
        }
    }

    /// Which hook handler types are supported for this event.
    ///
    /// Gemini CLI only supports command hooks. Unlike Claude Code,
    /// there are no prompt or agent handler types.
    pub fn supported_handler_types(&self) -> &'static [GeminiHookHandlerType] {
        &[GeminiHookHandlerType::Command]
    }

    /// The decision control pattern used by this event's response.
    pub fn decision_pattern(&self) -> GeminiDecisionPattern {
        match self {
            Self::BeforeAgent => GeminiDecisionPattern::PromptDecision,
            Self::AfterAgent => GeminiDecisionPattern::RetryDecision,
            Self::BeforeModel => GeminiDecisionPattern::ModelOverride,
            Self::AfterModel => GeminiDecisionPattern::ChunkReplacement,
            Self::BeforeToolSelection => GeminiDecisionPattern::ToolConfig,
            Self::BeforeTool => GeminiDecisionPattern::ToolDecision,
            Self::AfterTool => GeminiDecisionPattern::ResultRedaction,
            Self::SessionStart => GeminiDecisionPattern::ContextInjection,
            Self::SessionEnd | Self::PreCompress | Self::Notification => {
                GeminiDecisionPattern::Informational
            }
        }
    }

    /// The typed payload structure this event provides on stdin.
    ///
    /// All events also include the common fields from `GeminiCommonInputFields`.
    pub fn payload_type(&self) -> GeminiPayloadType {
        match self {
            Self::SessionStart => GeminiPayloadType::SessionStart,
            Self::SessionEnd => GeminiPayloadType::SessionEnd,
            Self::BeforeAgent => GeminiPayloadType::BeforeAgent,
            Self::AfterAgent => GeminiPayloadType::AfterAgent,
            Self::BeforeModel => GeminiPayloadType::BeforeModel,
            Self::AfterModel => GeminiPayloadType::AfterModel,
            Self::BeforeToolSelection => GeminiPayloadType::BeforeToolSelection,
            Self::BeforeTool => GeminiPayloadType::BeforeTool,
            Self::AfterTool => GeminiPayloadType::AfterTool,
            Self::PreCompress => GeminiPayloadType::PreCompress,
            Self::Notification => GeminiPayloadType::Notification,
        }
    }

    /// The typed response structure this event expects on stdout.
    pub fn response_type(&self) -> GeminiResponseType {
        match self {
            Self::SessionStart => GeminiResponseType::ContextInjection,
            Self::SessionEnd => GeminiResponseType::BestEffort,
            Self::BeforeAgent => GeminiResponseType::PromptDecision,
            Self::AfterAgent => GeminiResponseType::RetryDecision,
            Self::BeforeModel => GeminiResponseType::ModelOverride,
            Self::AfterModel => GeminiResponseType::ChunkReplacement,
            Self::BeforeToolSelection => GeminiResponseType::ToolConfig,
            Self::BeforeTool => GeminiResponseType::ToolDecision,
            Self::AfterTool => GeminiResponseType::ResultRedaction,
            Self::PreCompress => GeminiResponseType::Informational,
            Self::Notification => GeminiResponseType::Informational,
        }
    }

    /// How multiple hooks for this event are aggregated.
    pub fn aggregation_strategy(&self) -> AggregationStrategy {
        match self {
            Self::BeforeTool
            | Self::AfterTool
            | Self::BeforeAgent
            | Self::AfterAgent
            | Self::SessionStart => AggregationStrategy::OrDecision,
            Self::BeforeModel | Self::AfterModel => AggregationStrategy::FieldReplacement,
            Self::BeforeToolSelection => AggregationStrategy::Union,
            Self::SessionEnd | Self::PreCompress | Self::Notification => {
                AggregationStrategy::SimpleMerge
            }
        }
    }

    /// The effect of exit code 2 for this event.
    pub fn exit_code_2_effect(&self) -> ExitCode2Effect {
        match self {
            Self::BeforeTool => ExitCode2Effect::BlockToolContinueTurn,
            Self::AfterTool => ExitCode2Effect::HideResultContinueTurn,
            Self::BeforeAgent => ExitCode2Effect::AbortTurnErasePrompt,
            Self::AfterAgent => ExitCode2Effect::RejectAndRetry,
            Self::BeforeModel => ExitCode2Effect::AbortTurnSkipLlm,
            Self::AfterModel => ExitCode2Effect::AbortTurnDiscardOutput,
            Self::SessionStart | Self::SessionEnd | Self::PreCompress | Self::Notification => {
                ExitCode2Effect::Advisory
            }
            Self::BeforeToolSelection => ExitCode2Effect::NotSupported,
        }
    }
}

impl fmt::Display for GeminiCliEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.native_name())
    }
}

// ---------------------------------------------------------------------------
// TryFrom<&str> for parsing native event names
// ---------------------------------------------------------------------------

impl TryFrom<&str> for GeminiCliEvent {
    type Error = UnknownGeminiEventError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "SessionStart" => Ok(Self::SessionStart),
            "SessionEnd" => Ok(Self::SessionEnd),
            "BeforeAgent" => Ok(Self::BeforeAgent),
            "AfterAgent" => Ok(Self::AfterAgent),
            "BeforeModel" => Ok(Self::BeforeModel),
            "AfterModel" => Ok(Self::AfterModel),
            "BeforeToolSelection" => Ok(Self::BeforeToolSelection),
            "BeforeTool" => Ok(Self::BeforeTool),
            "AfterTool" => Ok(Self::AfterTool),
            "PreCompress" => Ok(Self::PreCompress),
            "Notification" => Ok(Self::Notification),
            _ => Err(UnknownGeminiEventError(name.to_string())),
        }
    }
}

/// Error returned when a string does not match any known Gemini CLI event.
#[derive(Debug, Clone, thiserror::Error)]
#[error("unknown Gemini CLI event: {0}")]
pub struct UnknownGeminiEventError(pub String);

// ---------------------------------------------------------------------------
// Mapping to the unified AgenticEvent
// ---------------------------------------------------------------------------

impl From<GeminiCliEvent> for AgenticEvent {
    fn from(event: GeminiCliEvent) -> Self {
        match event {
            GeminiCliEvent::SessionStart => AgenticEvent::SessionStart,
            GeminiCliEvent::SessionEnd => AgenticEvent::SessionEnd,
            GeminiCliEvent::BeforeAgent => AgenticEvent::BeforePrompt,
            GeminiCliEvent::AfterAgent => AgenticEvent::TurnComplete,
            GeminiCliEvent::BeforeModel => AgenticEvent::BeforeModel,
            GeminiCliEvent::AfterModel => AgenticEvent::AfterModel,
            GeminiCliEvent::BeforeToolSelection => AgenticEvent::BeforeModel,
            GeminiCliEvent::BeforeTool => AgenticEvent::BeforeTool,
            GeminiCliEvent::AfterTool => AgenticEvent::AfterTool,
            GeminiCliEvent::PreCompress => AgenticEvent::BeforeCompact,
            GeminiCliEvent::Notification => AgenticEvent::Notification,
        }
    }
}

impl TryFrom<AgenticEvent> for GeminiCliEvent {
    type Error = &'static str;

    /// Best-effort reverse mapping from unified to Gemini CLI event.
    ///
    /// Events that map 1:1 succeed. Events that have no Gemini CLI
    /// equivalent (e.g., `PermissionRequest`, `SubagentStart`) return `Err`.
    /// Events where multiple Gemini CLI events map to the same unified
    /// event (e.g., `BeforeModel` <- `BeforeModel` + `BeforeToolSelection`)
    /// return the primary mapping.
    fn try_from(event: AgenticEvent) -> Result<Self, Self::Error> {
        match event {
            AgenticEvent::SessionStart => Ok(GeminiCliEvent::SessionStart),
            AgenticEvent::SessionEnd => Ok(GeminiCliEvent::SessionEnd),
            AgenticEvent::BeforePrompt => Ok(GeminiCliEvent::BeforeAgent),
            AgenticEvent::BeforeTool => Ok(GeminiCliEvent::BeforeTool),
            AgenticEvent::AfterTool => Ok(GeminiCliEvent::AfterTool),
            AgenticEvent::TurnComplete => Ok(GeminiCliEvent::AfterAgent),
            AgenticEvent::BeforeModel => Ok(GeminiCliEvent::BeforeModel),
            AgenticEvent::AfterModel => Ok(GeminiCliEvent::AfterModel),
            AgenticEvent::BeforeCompact => Ok(GeminiCliEvent::PreCompress),
            AgenticEvent::Notification => Ok(GeminiCliEvent::Notification),
            AgenticEvent::ToolError
            | AgenticEvent::PermissionRequest
            | AgenticEvent::SubagentStart
            | AgenticEvent::SubagentStop
            | AgenticEvent::TurnError
            | AgenticEvent::HumanInTheLoop => {
                Err("Gemini CLI does not support this event")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums
// ---------------------------------------------------------------------------

/// The field that the matcher runs against for a given event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiMatcherField {
    /// `tool_name` -- BeforeTool, AfterTool
    ToolName,
    /// `source` -- SessionStart (startup, resume, clear)
    Source,
    /// `reason` -- SessionEnd (exit, clear, logout, etc.)
    Reason,
    /// `trigger` -- PreCompress (auto, manual)
    Trigger,
}

/// Matcher strategies used by Gemini CLI.
///
/// Gemini CLI uses two different matching strategies depending on event
/// type. Claude Code uses regex for all matchable events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatcherStrategy {
    /// Regex pattern matching (tool events).
    /// Falls back to exact string comparison if the pattern is not valid regex.
    Regex,
    /// Exact string comparison (lifecycle events).
    ExactString,
}

/// Hook handler types supported by Gemini CLI.
///
/// Unlike Claude Code which supports command, prompt, and agent handlers,
/// Gemini CLI only supports command hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiHookHandlerType {
    /// Shell command. Receives JSON on stdin, returns JSON on stdout.
    Command,
}

/// How the event communicates decisions back to Gemini CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiDecisionPattern {
    /// BeforeAgent: `decision: "deny"` erases prompt from context,
    /// `continue: false` preserves prompt but stops the loop.
    PromptDecision,
    /// AfterAgent: `decision: "deny"` triggers retry with `reason` as
    /// correction instructions. Must check `stop_hook_active`.
    RetryDecision,
    /// BeforeModel: can override `llm_request` fields or inject a
    /// synthetic `llm_response` to skip the LLM call entirely.
    ModelOverride,
    /// AfterModel: can replace the streaming chunk via `llm_response`.
    /// `decision: "deny"` discards the chunk and blocks the turn.
    ChunkReplacement,
    /// BeforeToolSelection: configures `toolConfig.mode` and
    /// `toolConfig.allowedFunctionNames`. No flow control.
    ToolConfig,
    /// BeforeTool: `decision: "deny"` prevents tool execution.
    /// `reason` becomes the tool error message the agent sees.
    ToolDecision,
    /// AfterTool: `decision: "deny"` hides real output from agent.
    /// `reason` replaces the tool result.
    ResultRedaction,
    /// SessionStart: `additionalContext` injected into conversation.
    ContextInjection,
    /// Cannot influence agent behavior. Informational only.
    Informational,
}

/// How multiple hooks for the same event are aggregated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// Any single "deny" blocks. Messages/contexts concatenated.
    /// Used by: BeforeTool, AfterTool, BeforeAgent, AfterAgent, SessionStart.
    OrDecision,
    /// Later hooks override earlier hooks' fields.
    /// Used by: BeforeModel, AfterModel.
    FieldReplacement,
    /// `allowedFunctionNames` are unioned. `NONE` wins over all;
    /// `ANY` wins over `AUTO`.
    /// Used by: BeforeToolSelection.
    Union,
    /// Later outputs override earlier ones.
    /// Used by: SessionEnd, PreCompress, Notification.
    SimpleMerge,
}

/// Discriminant for the event-specific input payload type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiPayloadType {
    SessionStart,
    SessionEnd,
    BeforeAgent,
    AfterAgent,
    BeforeModel,
    AfterModel,
    BeforeToolSelection,
    BeforeTool,
    AfterTool,
    PreCompress,
    Notification,
}

/// Discriminant for the event-specific response type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiResponseType {
    /// SessionStart: can inject context via `additionalContext`.
    ContextInjection,
    /// SessionEnd: best-effort, not awaited by CLI.
    BestEffort,
    /// BeforeAgent: can block prompt, inject context.
    PromptDecision,
    /// AfterAgent: can trigger retry, clear context.
    RetryDecision,
    /// BeforeModel: can override request or inject synthetic response.
    ModelOverride,
    /// AfterModel: can replace streaming chunk.
    ChunkReplacement,
    /// BeforeToolSelection: only `toolConfig` is applied.
    ToolConfig,
    /// BeforeTool: can block tool, override input.
    ToolDecision,
    /// AfterTool: can redact output, inject context.
    ResultRedaction,
    /// PreCompress, Notification: informational only.
    Informational,
}

/// The effect of exit code 2 for each Gemini CLI event.
///
/// This differs significantly from Claude Code where exit code 2 has
/// a uniform "block" semantic. In Gemini CLI, exit code 2 has different
/// effects depending on the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode2Effect {
    /// Blocks tool execution; turn continues.
    BlockToolContinueTurn,
    /// Hides tool result; turn continues.
    HideResultContinueTurn,
    /// Aborts turn; erases prompt from context.
    AbortTurnErasePrompt,
    /// Rejects response; triggers automatic retry using stderr as feedback.
    RejectAndRetry,
    /// Aborts turn; skips LLM call.
    AbortTurnSkipLlm,
    /// Aborts turn; discards model output.
    AbortTurnDiscardOutput,
    /// Advisory only; startup/shutdown/compression is never blocked.
    Advisory,
    /// Not supported; exit code 2 has no effect.
    NotSupported,
}

// ---------------------------------------------------------------------------
// Common input fields (present on ALL events)
// ---------------------------------------------------------------------------

/// Fields present in the JSON input of every Gemini CLI hook event.
///
/// All event-specific payload structs include these fields via
/// `#[serde(flatten)]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCommonInputFields {
    /// Current session identifier.
    pub session_id: String,

    /// Path to the session transcript JSON file.
    pub transcript_path: String,

    /// Current working directory when the hook was invoked.
    pub cwd: String,

    /// Name of the event that fired (matches `GeminiCliEvent::native_name`).
    pub hook_event_name: String,

    /// ISO 8601 timestamp of when the event fired.
    ///
    /// This field is unique to Gemini CLI; Claude Code does not include
    /// timestamps in hook input.
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Event-specific input payloads
// ---------------------------------------------------------------------------

/// SessionStart input. Includes `source`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiSessionStartPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// How the session started.
    pub source: GeminiSessionStartSource,
}

/// The `source` field values for SessionStart.
///
/// Note: Gemini CLI does not have a "compact" source (Claude Code fires
/// SessionStart after context compaction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeminiSessionStartSource {
    Startup,
    Resume,
    Clear,
}

impl GeminiSessionStartSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Resume => "resume",
            Self::Clear => "clear",
        }
    }
}

/// SessionEnd input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiSessionEndPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// Why the session ended.
    pub reason: GeminiSessionEndReason,
}

/// The `reason` field values for SessionEnd.
///
/// Gemini CLI has "exit" where Claude Code has "logout" and
/// "bypass_permissions_disabled".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeminiSessionEndReason {
    Exit,
    Clear,
    Logout,
    PromptInputExit,
    Other,
}

impl GeminiSessionEndReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exit => "exit",
            Self::Clear => "clear",
            Self::Logout => "logout",
            Self::PromptInputExit => "prompt_input_exit",
            Self::Other => "other",
        }
    }
}

/// BeforeAgent input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiBeforeAgentPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// The text the user submitted.
    pub prompt: String,
}

/// AfterAgent input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiAfterAgentPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// The user's original request.
    pub prompt: String,

    /// The final text generated by the agent.
    pub prompt_response: String,

    /// `true` if this hook is already running as part of a retry sequence.
    /// **Must check this to prevent infinite loops.**
    pub stop_hook_active: bool,
}

/// BeforeModel input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiBeforeModelPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// Stable, SDK-agnostic request object.
    pub llm_request: LlmRequest,
}

/// AfterModel input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiAfterModelPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// The original request.
    pub llm_request: LlmRequest,

    /// The model's response (or a single streaming chunk).
    pub llm_response: LlmResponse,
}

/// BeforeToolSelection input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiBeforeToolSelectionPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// Same format as BeforeModel.
    pub llm_request: LlmRequest,
}

/// BeforeTool input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiBeforeToolPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// Name of the tool being called.
    pub tool_name: String,

    /// Raw arguments generated by the model.
    pub tool_input: Value,

    /// Present only for MCP tools. Contains server and connection info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_context: Option<McpContext>,
}

/// AfterTool input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiAfterToolPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// Name of the tool that executed.
    pub tool_name: String,

    /// Original arguments.
    pub tool_input: Value,

    /// Result containing `llmContent`, `returnDisplay`, and optional `error`.
    pub tool_response: Value,

    /// Present only for MCP tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_context: Option<McpContext>,
}

/// PreCompress input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiPreCompressPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// How compression was triggered.
    pub trigger: GeminiCompressTrigger,
}

/// CompressTrigger values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeminiCompressTrigger {
    Auto,
    Manual,
}

impl GeminiCompressTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }
}

/// Notification input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiNotificationPayload {
    #[serde(flatten)]
    pub common: GeminiCommonInputFields,

    /// Currently only `"ToolPermission"`.
    pub notification_type: String,

    /// Summary of the alert.
    pub message: String,

    /// Alert-specific metadata (e.g., tool name, file path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

// ---------------------------------------------------------------------------
// LLM request/response types (Gemini Stable Model API)
// ---------------------------------------------------------------------------

/// Stable, SDK-agnostic LLM request structure.
///
/// Used by BeforeModel, AfterModel, and BeforeToolSelection.
/// This structure is versioned independently from the underlying Gemini SDK.
/// Non-text parts (images, function calls, etc.) are filtered out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Model identifier.
    pub model: String,

    /// Conversation messages (text content only).
    pub messages: Vec<LlmMessage>,

    /// Generation configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<LlmConfig>,

    /// Tool configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
}

/// A single message in the LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// Message role.
    pub role: LlmRole,

    /// Text content (non-text parts are filtered out).
    pub content: String,
}

/// Message roles in the LLM API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    User,
    Model,
    System,
}

/// LLM generation configuration parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
}

/// Tool selection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Tool selection mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ToolSelectionMode>,

    /// Whitelist of tool names the LLM may call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Tool selection modes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ToolSelectionMode {
    /// Default: LLM decides whether to call tools.
    Auto,
    /// Force at least one tool call.
    Any,
    /// Disable all tools.
    None,
}

/// Stable LLM response structure.
///
/// Used by AfterModel (per streaming chunk) and BeforeModel (synthetic
/// response injection).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmResponse {
    /// Convenience text field (first candidate's first part).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Response candidates.
    pub candidates: Vec<LlmCandidate>,

    /// Token usage metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

/// A single response candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmCandidate {
    /// Candidate content.
    pub content: LlmCandidateContent,

    /// Why generation stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,

    /// Candidate index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,

    /// Safety ratings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

/// Content of a response candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCandidateContent {
    pub role: LlmRole,
    pub parts: Vec<String>,
}

/// Finish reasons for LLM generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinishReason {
    Stop,
    MaxTokens,
    Safety,
    Recitation,
    Other,
}

/// Safety rating for a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
}

/// Token usage metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<u32>,
}

// ---------------------------------------------------------------------------
// MCP context (present on tool events for MCP tools)
// ---------------------------------------------------------------------------

/// MCP tool context, present on BeforeTool and AfterTool for MCP tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContext {
    /// MCP server name.
    pub server_name: String,

    /// MCP tool name.
    pub tool_name: String,

    /// Connection details (stdio, SSE/HTTP, or WebSocket).
    #[serde(flatten)]
    pub connection: McpConnection,
}

/// MCP connection types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpConnection {
    /// stdio transport.
    Stdio {
        command: String,
        #[serde(default)]
        args: Option<Vec<String>>,
        #[serde(default)]
        cwd: Option<String>,
    },
    /// SSE or HTTP transport.
    Http {
        url: String,
    },
    /// WebSocket transport.
    WebSocket {
        tcp: String,
    },
}

// ---------------------------------------------------------------------------
// Unified input payload enum (for dispatch)
// ---------------------------------------------------------------------------

/// Type-safe wrapper for any Gemini CLI event's input payload.
///
/// Constructed by deserializing the hook's stdin JSON based on the
/// `hook_event_name` field. Gives callers access to the strongly-typed
/// event-specific fields without manual JSON wrangling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum GeminiCliInput {
    SessionStart(GeminiSessionStartPayload),
    SessionEnd(GeminiSessionEndPayload),
    BeforeAgent(GeminiBeforeAgentPayload),
    AfterAgent(GeminiAfterAgentPayload),
    BeforeModel(GeminiBeforeModelPayload),
    AfterModel(GeminiAfterModelPayload),
    BeforeToolSelection(GeminiBeforeToolSelectionPayload),
    BeforeTool(GeminiBeforeToolPayload),
    AfterTool(GeminiAfterToolPayload),
    PreCompress(GeminiPreCompressPayload),
    Notification(GeminiNotificationPayload),
}

impl GeminiCliInput {
    /// Returns the event variant for this input payload.
    pub fn event(&self) -> GeminiCliEvent {
        match self {
            Self::SessionStart(_) => GeminiCliEvent::SessionStart,
            Self::SessionEnd(_) => GeminiCliEvent::SessionEnd,
            Self::BeforeAgent(_) => GeminiCliEvent::BeforeAgent,
            Self::AfterAgent(_) => GeminiCliEvent::AfterAgent,
            Self::BeforeModel(_) => GeminiCliEvent::BeforeModel,
            Self::AfterModel(_) => GeminiCliEvent::AfterModel,
            Self::BeforeToolSelection(_) => GeminiCliEvent::BeforeToolSelection,
            Self::BeforeTool(_) => GeminiCliEvent::BeforeTool,
            Self::AfterTool(_) => GeminiCliEvent::AfterTool,
            Self::PreCompress(_) => GeminiCliEvent::PreCompress,
            Self::Notification(_) => GeminiCliEvent::Notification,
        }
    }

    /// Returns the common input fields shared by all events.
    pub fn common(&self) -> &GeminiCommonInputFields {
        match self {
            Self::SessionStart(p) => &p.common,
            Self::SessionEnd(p) => &p.common,
            Self::BeforeAgent(p) => &p.common,
            Self::AfterAgent(p) => &p.common,
            Self::BeforeModel(p) => &p.common,
            Self::AfterModel(p) => &p.common,
            Self::BeforeToolSelection(p) => &p.common,
            Self::BeforeTool(p) => &p.common,
            Self::AfterTool(p) => &p.common,
            Self::PreCompress(p) => &p.common,
            Self::Notification(p) => &p.common,
        }
    }

    /// Returns the tool name if this is a tool-related event.
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::BeforeTool(p) => Some(&p.tool_name),
            Self::AfterTool(p) => Some(&p.tool_name),
            _ => None,
        }
    }

    /// Returns the matcher value -- the string the matcher should
    /// test against for this event.
    pub fn matcher_value(&self) -> Option<&str> {
        match self {
            Self::BeforeTool(p) => Some(&p.tool_name),
            Self::AfterTool(p) => Some(&p.tool_name),
            Self::SessionStart(p) => Some(p.source.as_str()),
            Self::SessionEnd(p) => Some(p.reason.as_str()),
            Self::PreCompress(p) => Some(p.trigger.as_str()),
            // Events with no matcher support
            Self::BeforeAgent(_)
            | Self::AfterAgent(_)
            | Self::BeforeModel(_)
            | Self::AfterModel(_)
            | Self::BeforeToolSelection(_)
            | Self::Notification(_) => None,
        }
    }

    /// Whether the stop_hook_active flag is set (AfterAgent only).
    ///
    /// Returns `false` for all other events. Callers should check this
    /// before blocking AfterAgent to prevent infinite retry loops.
    pub fn is_stop_hook_active(&self) -> bool {
        match self {
            Self::AfterAgent(p) => p.stop_hook_active,
            _ => false,
        }
    }

    /// Returns the LLM request if this is a model-related event.
    pub fn llm_request(&self) -> Option<&LlmRequest> {
        match self {
            Self::BeforeModel(p) => Some(&p.llm_request),
            Self::AfterModel(p) => Some(&p.llm_request),
            Self::BeforeToolSelection(p) => Some(&p.llm_request),
            _ => None,
        }
    }

    /// Returns the LLM response if this is an AfterModel event.
    pub fn llm_response(&self) -> Option<&LlmResponse> {
        match self {
            Self::AfterModel(p) => Some(&p.llm_response),
            _ => None,
        }
    }

    /// Returns the MCP context if this is an MCP tool event.
    pub fn mcp_context(&self) -> Option<&McpContext> {
        match self {
            Self::BeforeTool(p) => p.mcp_context.as_ref(),
            Self::AfterTool(p) => p.mcp_context.as_ref(),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Response types (what hooks return on stdout)
// ---------------------------------------------------------------------------

/// Common response fields supported by most events on exit 0.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiCommonResponseFields {
    /// Setting `false` stops the entire agent loop immediately.
    #[serde(default, rename = "continue", skip_serializing_if = "Option::is_none")]
    pub should_continue: Option<bool>,

    /// Message shown to user when `continue` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// When true, suppresses hook metadata from logs/telemetry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,

    /// Warning/message displayed immediately to the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,

    /// `"allow"`, `"deny"` (alias `"block"`), or `"ask"` / `"approve"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    /// Feedback message when decision is deny/block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Event-specific output fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<Value>,
}

/// BeforeAgent response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiBeforeAgentOutput {
    #[serde(flatten)]
    pub common: GeminiCommonResponseFields,
}

/// AfterAgent response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiAfterAgentOutput {
    #[serde(flatten)]
    pub common: GeminiCommonResponseFields,
}

/// AfterAgent hook-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterAgentHookOutput {
    /// If `true`, clears conversation history while preserving UI display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_context: Option<bool>,
}

/// BeforeModel response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiBeforeModelOutput {
    #[serde(flatten)]
    pub common: GeminiCommonResponseFields,
}

/// BeforeModel hook-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeModelHookOutput {
    /// Partial override of the outgoing LLM request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_request: Option<Value>,

    /// Synthetic response. If provided, CLI skips the LLM call entirely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_response: Option<LlmResponse>,
}

/// AfterModel response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiAfterModelOutput {
    #[serde(flatten)]
    pub common: GeminiCommonResponseFields,
}

/// AfterModel hook-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterModelHookOutput {
    /// Replacement for the model's streaming chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_response: Option<Value>,
}

/// BeforeToolSelection response output.
///
/// This event does NOT support `decision`, `continue`, or `systemMessage`.
/// Only `toolConfig` is applied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiBeforeToolSelectionOutput {
    /// The tool configuration to apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<BeforeToolSelectionHookOutput>,
}

/// BeforeToolSelection hook-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeToolSelectionHookOutput {
    /// Tool configuration to apply.
    pub tool_config: ToolConfig,
}

/// BeforeTool response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiBeforeToolOutput {
    #[serde(flatten)]
    pub common: GeminiCommonResponseFields,
}

/// BeforeTool hook-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeToolHookOutput {
    /// Merges with and overrides the model's arguments before execution.
    /// Shallow merge; cannot add fields the tool does not expect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,
}

/// AfterTool response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiAfterToolOutput {
    #[serde(flatten)]
    pub common: GeminiCommonResponseFields,
}

/// AfterTool hook-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterToolHookOutput {
    /// Appended to the tool result for the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// SessionStart response output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSessionStartOutput {
    /// Warning/message shown at start of session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<SessionStartHookOutput>,
}

/// SessionStart hook-specific output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartHookOutput {
    /// Interactive mode: injected as first turn.
    /// Non-interactive mode: prepended to user prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

// ---------------------------------------------------------------------------
// Exit code semantics
// ---------------------------------------------------------------------------

/// Exit code semantics for Gemini CLI hooks.
///
/// Same three-way protocol as Claude Code, but exit code 2 behavior
/// varies significantly by event (see `ExitCode2Effect`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiExitCode {
    /// Success. Action proceeds. JSON output is processed.
    Success = 0,
    /// System block. Behavior varies by event (see `ExitCode2Effect`).
    /// Stderr is used as the rejection/feedback reason.
    Block = 2,
    // Any other code: non-fatal warning. Action proceeds.
}

impl GeminiExitCode {
    /// Interpret a raw process exit code.
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => Self::Success,
            2 => Self::Block,
            _ => Self::Success, // Non-fatal warning, treat as success path
        }
    }

    /// Whether JSON output should be parsed from stdout.
    pub fn should_parse_json(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Whether this exit code triggers the event-specific block behavior.
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Block)
    }
}

// ---------------------------------------------------------------------------
// Environment variables available in hooks
// ---------------------------------------------------------------------------

/// Environment variables Gemini CLI sets for hook processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiHookEnvVar {
    /// `GEMINI_PROJECT_DIR` -- absolute path to project root.
    ProjectDir,
    /// `GEMINI_SESSION_ID` -- unique ID for the current session.
    SessionId,
    /// `GEMINI_CWD` -- current working directory.
    Cwd,
    /// `CLAUDE_PROJECT_DIR` -- alias for `GEMINI_PROJECT_DIR` (compatibility).
    ClaudeProjectDir,
}

impl GeminiHookEnvVar {
    pub fn var_name(&self) -> &'static str {
        match self {
            Self::ProjectDir => "GEMINI_PROJECT_DIR",
            Self::SessionId => "GEMINI_SESSION_ID",
            Self::Cwd => "GEMINI_CWD",
            Self::ClaudeProjectDir => "CLAUDE_PROJECT_DIR",
        }
    }

    /// All environment variables set for hook processes.
    pub const ALL: [GeminiHookEnvVar; 4] = [
        Self::ProjectDir,
        Self::SessionId,
        Self::Cwd,
        Self::ClaudeProjectDir,
    ];
}

// ---------------------------------------------------------------------------
// Tool input shapes (for BeforeTool / AfterTool)
// ---------------------------------------------------------------------------

/// Known built-in tool names for Gemini CLI.
///
/// Gemini CLI uses snake_case tool names (e.g., `write_file`,
/// `run_shell_command`), unlike Claude Code's PascalCase (e.g., `Write`,
/// `Bash`). MCP tools follow the pattern `mcp__<server>__<tool>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeminiBuiltinTool {
    ListDirectory,
    ReadFile,
    ReadManyFiles,
    WriteFile,
    Glob,
    SearchFileContent,
    Replace,
    RunShellCommand,
    AskUser,
    SaveMemory,
    WriteTodos,
    ActivateSkill,
    GetInternalDocs,
    WebFetch,
    GoogleWebSearch,
}

impl GeminiBuiltinTool {
    /// The native tool name string.
    pub fn native_name(&self) -> &'static str {
        match self {
            Self::ListDirectory => "list_directory",
            Self::ReadFile => "read_file",
            Self::ReadManyFiles => "read_many_files",
            Self::WriteFile => "write_file",
            Self::Glob => "glob",
            Self::SearchFileContent => "search_file_content",
            Self::Replace => "replace",
            Self::RunShellCommand => "run_shell_command",
            Self::AskUser => "ask_user",
            Self::SaveMemory => "save_memory",
            Self::WriteTodos => "write_todos",
            Self::ActivateSkill => "activate_skill",
            Self::GetInternalDocs => "get_internal_docs",
            Self::WebFetch => "web_fetch",
            Self::GoogleWebSearch => "google_web_search",
        }
    }

    /// All built-in tools.
    pub const ALL: [GeminiBuiltinTool; 15] = [
        Self::ListDirectory,
        Self::ReadFile,
        Self::ReadManyFiles,
        Self::WriteFile,
        Self::Glob,
        Self::SearchFileContent,
        Self::Replace,
        Self::RunShellCommand,
        Self::AskUser,
        Self::SaveMemory,
        Self::WriteTodos,
        Self::ActivateSkill,
        Self::GetInternalDocs,
        Self::WebFetch,
        Self::GoogleWebSearch,
    ];
}

impl TryFrom<&str> for GeminiBuiltinTool {
    type Error = ();

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        match name {
            "list_directory" => Ok(Self::ListDirectory),
            "read_file" => Ok(Self::ReadFile),
            "read_many_files" => Ok(Self::ReadManyFiles),
            "write_file" => Ok(Self::WriteFile),
            "glob" => Ok(Self::Glob),
            "search_file_content" => Ok(Self::SearchFileContent),
            "replace" => Ok(Self::Replace),
            "run_shell_command" => Ok(Self::RunShellCommand),
            "ask_user" => Ok(Self::AskUser),
            "save_memory" => Ok(Self::SaveMemory),
            "write_todos" => Ok(Self::WriteTodos),
            "activate_skill" => Ok(Self::ActivateSkill),
            "get_internal_docs" => Ok(Self::GetInternalDocs),
            "web_fetch" => Ok(Self::WebFetch),
            "google_web_search" => Ok(Self::GoogleWebSearch),
            _ => Err(()),
        }
    }
}
```

## Design Considerations

- **Native naming preserved.** The enum variants use Gemini CLI's exact PascalCase event names (`BeforeTool`, `AfterAgent`, `BeforeModel`, etc.) rather than normalized names. This keeps the design consistent with the Claude Code approach where native naming eliminates a translation layer in matcher logic and debug output.

- **All 11 events captured.** Gemini CLI supports 11 hook events compared to Claude Code's 14. The missing events (no `PermissionRequest`, no `SubagentStart`/`SubagentStop`, no `TeammateIdle`/`TaskCompleted`, no `PostToolUseFailure`) reflect genuine gaps in Gemini CLI's event model, not design omissions.

- **Model-level hooks are first-class.** Gemini CLI's `BeforeModel` and `AfterModel` hooks have no Claude Code equivalent. These require the full `LlmRequest`/`LlmResponse` stable API types, which are unique to Gemini CLI. The design includes dedicated structs for these types rather than using `serde_json::Value`, since the schema is well-defined and versioned independently from the SDK.

- **Dual matcher strategies.** Gemini CLI uses regex matching for tool events but exact string matching for lifecycle events (`SessionStart`, `SessionEnd`, `PreCompress`). This is captured via the `MatcherStrategy` enum. Claude Code uses regex for all matchable events, making this a meaningful structural difference.

- **AfterAgent retry semantics modeled explicitly.** Gemini CLI's `AfterAgent` hook has unique retry semantics: `decision: "deny"` triggers an automatic retry with `reason` as correction instructions, and `clearContext` can reset LLM memory between retries. The `stop_hook_active` field for infinite loop prevention mirrors Claude Code's `Stop`/`SubagentStop` pattern but applies to response validation rather than turn completion.

- **BeforeToolSelection is a distinct event type.** This event has no Claude Code equivalent and uses a unique aggregation strategy (union of `allowedFunctionNames`, priority ordering of modes). It intentionally does not support `decision`, `continue`, or `systemMessage`, which is captured by the `ToolConfig` decision pattern.

- **Multi-hook aggregation strategies are explicit.** Gemini CLI documents four distinct aggregation strategies for when multiple hooks match the same event. These are captured as the `AggregationStrategy` enum because the aggregation behavior meaningfully affects hook design. Claude Code does not document aggregation strategies at all.

- **Exit code 2 behavior varies by event.** Unlike Claude Code where exit code 2 uniformly means "block", Gemini CLI's exit code 2 has different effects for each event (block tool vs. abort turn vs. trigger retry vs. advisory). This is captured in the `ExitCode2Effect` enum to make the variation explicit rather than burying it in documentation.

- **MCP context is a first-class concept.** Gemini CLI's `mcp_context` field on tool events exposes MCP server connection details (stdio, HTTP/SSE, WebSocket). This is unique to Gemini CLI and modeled as an optional field on tool payloads with a structured `McpContext`/`McpConnection` type.

- **Only `command` handler type.** Gemini CLI does not support prompt or agent hook handler types. The `GeminiHookHandlerType` enum has a single variant, which is intentional rather than omitting the type entirely -- it preserves the same structural API as Claude Code for the unified layer to query.

- **`additionalContext` HTML sanitization.** Gemini CLI escapes `<` and `>` in `additionalContext` values to prevent tag injection. This is a runtime behavior rather than a type-level concern, but it affects how hook authors construct context strings. This gotcha is documented in the source hooks document and should be noted in any hook authoring guidance.

- **`CLAUDE_PROJECT_DIR` compatibility alias.** Gemini CLI provides `CLAUDE_PROJECT_DIR` as an alias for `GEMINI_PROJECT_DIR`, enabling scripts originally written for Claude Code to work without modification. This is captured in the `GeminiHookEnvVar` enum.

- **15 built-in tools enumerated.** Gemini CLI has 15 known built-in tools (vs. Claude Code's 9), using snake_case naming (`write_file` vs. `Write`, `run_shell_command` vs. `Bash`). The design catalogs these for matcher validation but does not define strongly-typed input structs for each tool, since Gemini CLI's tool input schemas are less stable and less documented than Claude Code's.

- **Forward compatibility with `Value` fallbacks.** Hook-specific output fields use `serde_json::Value` for the top-level `hook_specific_output` in `GeminiCommonResponseFields`, while dedicated response structs provide type safety for well-known events. This allows unknown future fields to pass through without deserialization failures.

## Claude Code Mapping

- **SessionStart** -- Same trigger (startup/resume/clear). Gemini CLI lacks the "compact" source value that Claude Code fires after context compaction. Gemini CLI uses exact string matching for the source value; Claude Code uses regex. Gemini CLI's output supports only `additionalContext` and `systemMessage`; Claude Code also supports the `CLAUDE_ENV_FILE` mechanism for persisting environment variables.

- **SessionEnd** -- Same trigger (session termination). Gemini CLI has "exit" as a reason where Claude Code has "bypass_permissions_disabled". Both are best-effort/non-blocking. Gemini CLI explicitly documents that the CLI does not wait for this hook to complete.

- **BeforeAgent** -- Maps to Claude Code's `UserPromptSubmit`. Both fire after user submits a prompt, before processing. Gemini CLI adds the ability to erase the prompt from context with `decision: "deny"` (vs. Claude Code's `decision: "block"` which preserves it). Gemini CLI's `additionalContext` is appended to the prompt for the turn; Claude Code does not have this field on `UserPromptSubmit`.

- **AfterAgent** -- Partially maps to Claude Code's `Stop`. Both fire after the agent completes a response. However, Gemini CLI's `AfterAgent` has unique retry semantics (`decision: "deny"` triggers retry with `reason` as correction instructions) and `clearContext` for resetting LLM memory. Claude Code's `Stop` uses `decision: "block"` + `reason` to continue conversation but does not support automatic retry or context clearing. Gemini CLI provides `prompt_response` (the agent's output) in the payload; Claude Code's `Stop` does not.

- **BeforeModel** -- Entirely distinct from Claude Code. Claude Code has no hook that fires before sending requests to the LLM. Gemini CLI's `BeforeModel` can override request parameters, inject a synthetic response to skip the LLM call entirely, or block the turn. Uses a stable `LlmRequest`/`LlmResponse` API that is versioned independently from the Gemini SDK.

- **AfterModel** -- Entirely distinct from Claude Code. Claude Code has no hook that fires after receiving LLM responses. Gemini CLI's `AfterModel` fires per streaming chunk (not once per response) and can replace chunks for real-time redaction/PII filtering. Heavy processing in this hook slows streaming.

- **BeforeToolSelection** -- Entirely distinct from Claude Code. No Claude Code equivalent exists. Controls which tools the LLM may call via `toolConfig.mode` (AUTO/ANY/NONE) and `allowedFunctionNames` whitelist. Does not support flow control (`decision`, `continue`, `systemMessage` are all ignored). Uses union aggregation across multiple hooks.

- **BeforeTool** -- Maps to Claude Code's `PreToolUse`. Both fire before tool execution and support blocking. Key differences: (1) Gemini CLI uses `decision: "deny"` with `reason` as tool error message; Claude Code uses a three-level `permissionDecision` (allow/deny/ask). (2) Gemini CLI's `tool_input` override uses shallow merge; Claude Code's `updatedInput` is a full replacement. (3) Gemini CLI provides `mcp_context` for MCP tools; Claude Code does not expose MCP connection details. (4) Gemini CLI tool names are snake_case (`write_file`); Claude Code uses PascalCase (`Write`). (5) Gemini CLI does not have a `tool_use_id` field.

- **AfterTool** -- Maps to Claude Code's `PostToolUse` (success) and `PostToolUseFailure` (failure combined). Claude Code separates success and failure into two distinct events; Gemini CLI combines them into a single `AfterTool` event where the `tool_response` may contain an `error` field. Gemini CLI supports `additionalContext` for appending to tool results; Claude Code has this on `PostToolUse` as well. Gemini CLI's `decision: "deny"` hides tool output; Claude Code's `PostToolUse` blocking is advisory (cannot undo the tool call). Gemini CLI provides `mcp_context`; Claude Code does not.

- **PreCompress** -- Maps to Claude Code's `PreCompact`. Same concept (fires before context summarization). Gemini CLI uses the command `/compress`; Claude Code uses `/compact`. Both are advisory/non-blocking with `auto`/`manual` trigger values. Gemini CLI calls it "PreCompress" while Claude Code calls it "PreCompact". Gemini CLI fires this event asynchronously; Claude Code's PreCompact supports matchers and can inject context.

- **Notification** -- Maps to Claude Code's `Notification`. Both are informational and cannot block. Gemini CLI currently has only `"ToolPermission"` as a notification type; Claude Code has `permission_prompt`, `idle_prompt`, `auth_success`, and `elicitation_dialog`. Claude Code supports matchers on `notification_type`; Gemini CLI does not support matchers for notifications. Gemini CLI includes a `details` object; Claude Code has `title` and `message`.

- **No equivalent for Claude Code's `PermissionRequest`** -- Gemini CLI does not support programmatic permission decisions. Claude Code's `PermissionRequest` hook allows hooks to automatically allow/deny permission dialogs. Gemini CLI's `Notification` with type `"ToolPermission"` is observability-only and cannot automate permission decisions.

- **No equivalent for Claude Code's `SubagentStart`/`SubagentStop`** -- Gemini CLI does not expose subagent lifecycle events. Claude Code's subagent hooks fire when the Task tool spawns and completes subagents.

- **No equivalent for Claude Code's `TeammateIdle`/`TaskCompleted`** -- Gemini CLI does not have team/teammate concepts. These are Claude Code-specific events related to multi-agent team workflows.

- **No equivalent for Claude Code's `PostToolUseFailure`** -- Gemini CLI handles tool failures within the same `AfterTool` event. Claude Code separates success (`PostToolUse`) from failure (`PostToolUseFailure`) into distinct events with different payload shapes (failure includes `error` string and `is_interrupt` boolean).
