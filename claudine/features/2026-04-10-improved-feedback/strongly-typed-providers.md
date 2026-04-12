# Strongly Typed Provider Protocol Models — Technical Design

## Overview

Replace manual `serde_json::Value` traversal patterns in the six stream parsers (`claude`, `codex`, `gemini`, `opencode`, `qwen`, `kimi`) with serde-derived typed structs and enums. The goal: eliminate `.get().and_then().and_then()` chains, catch contract drift at deserialization time, and make provider protocol shapes explicit and reviewable.

## Current State

Every parser follows the same pattern:

1. Parse the raw line into `serde_json::Value` (`stream/claude.rs:313`)
2. Extract the event type with `obj.get("type").and_then(|t| t.as_str())` (`stream/claude.rs:323`)
3. Dispatch to a handler method that receives `&Value`
4. Extract fields with chains like `obj.get("error").and_then(|e| e.get("type")).and_then(|t| t.as_str())` (`stream/claude.rs:141-150`)
5. Accumulate into parser-local state fields
6. Produce `StreamExecutionSummary` on `finish()`

This pattern is repeated across all six parsers with minor variations per provider. The total `Value`-based field extraction code is approximately:

| Parser | Event types handled | Approx. Value extraction sites |
|--------|-------------------:|-------------------------------:|
| `claude.rs` | 9 | ~45 |
| `codex.rs` | 8 | ~35 |
| `gemini.rs` | 6 | ~35 |
| `opencode.rs` | 8 | ~50 |
| `qwen.rs` | 7 | ~30 |
| `kimi.rs` | 7 | ~35 |

**Total: ~230 manual `.get()` extraction sites** across 45 event-type handlers.

### Risks of the Current Approach

- **Silent contract drift**: If a provider renames or removes a field, the `.and_then()` chain silently returns `None` with no diagnostic
- **No compile-time guarantees**: There is no structural coupling between the provider's JSON contract and the parser code
- **Hard to review**: It is difficult to see what the full expected JSON shape is from reading the extraction chains
- **Duplicated patterns**: Error extraction, tool metadata extraction, and usage extraction are structurally identical across providers but duplicated with slight naming variations

## Proposed Architecture

### New Module: `stream/protocol/`

```
claudine/lib/src/stream/
├── protocol/
│   ├── mod.rs              # Re-exports, shared types
│   ├── claude.rs           # Claude Code typed events
│   ├── codex.rs            # Codex CLI typed events
│   ├── gemini.rs           # Gemini CLI typed events
│   ├── opencode.rs         # OpenCode typed events
│   ├── qwen.rs             # Qwen CLI typed events
│   └── kimi.rs             # Kimi Code typed events
├── claude.rs               # Existing parser (migrated)
├── codex.rs                # Existing parser (migrated)
├── gemini.rs               # Existing parser (migrated)
├── opencode.rs             # Existing parser (migrated)
├── qwen.rs                 # Existing parser (migrated)
├── kimi.rs                 # Existing parser (migrated)
├── parser.rs               # StreamParser trait (unchanged)
├── summary.rs              # StreamExecutionSummary (unchanged)
└── token_usage.rs          # NormalizedTokenUsage (unchanged)
```

The `protocol/` module contains only the typed event structs/enums and their deserialization logic. The existing parser files remain the parsing orchestrators but consume typed events instead of raw `Value`.

### Shared Protocol Types

```rust
// stream/protocol/mod.rs

/// The envelope error returned when a typed event cannot be deserialized
/// from a provider line that was expected to match a specific type tag.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Unknown event type: {0}")]
    UnknownEventType(String),

    #[error("Deserialization failed for {event_type}: {reason}")]
    DeserializationFailed {
        event_type: String,
        reason: String,
    },
}
```

### Per-Provider Typed Events

Each provider module defines a tagged enum that represents the complete set of event types that parser handles. Unknown types are explicitly represented rather than silently dropped.

#### Claude (`protocol/claude.rs`)

```rust
use serde::Deserialize;

/// Typed envelope for Claude Code `stream-json` events.
///
## References
/// * Claude Code structured output: `claude --output-format stream-json`
/// * Event types observed: init, system, assistant, content_block_start,
///   content_block_delta, error, assistant.error, result, rate_limit_event,
///   tool_use, tool_result
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeEvent {
    Init(ClaudeInit),
    System(ClaudeInit),
    Assistant(ClaudeAssistant),
    ContentBlockStart(ClaudeContentBlockStart),
    ContentBlockDelta(ClaudeContentBlockDelta),
    #[serde(rename = "error")]
    Error(ClaudeErrorEvent),
    #[serde(rename = "assistant.error")]
    AssistantError(ClaudeErrorEvent),
    Result(ClaudeResult),
    RateLimitEvent(ClaudeRateLimit),
    ToolUse(ClaudeToolUse),
    ToolResult(ClaudeToolResult),
}

#[derive(Debug, Deserialize)]
pub struct ClaudeInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeAssistant {
    /// Claude Code wraps content under a "message" key.
    /// The simplified test format places it at the top level.
    pub message: Option<ClaudeMessage>,
    /// Top-level content array (simplified format).
    pub content: Option<Vec<ClaudeContentBlock>>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeMessage {
    pub content: Vec<ClaudeContentBlock>,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeContentBlock {
    Text { text: String },
    ToolUse(ClaudeToolUseContent),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeToolUseContent {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeContentBlockStart {
    pub content_block: Option<ClaudeToolUseContent>,
    #[serde(default)]
    pub index: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeContentBlockDelta {
    pub delta: ClaudeDelta,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeDelta {
    TextDelta { text: String },
    ThinkingDelta { thinking: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
pub struct ClaudeErrorEvent {
    pub error: ClaudeError,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeError {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeResult {
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u64>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub usage: Option<ClaudeUsage>,
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
    #[serde(default)]
    pub skills: Option<serde_json::Value>,
    #[serde(default)]
    pub agents: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp_servers: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeRateLimit {
    #[serde(default)]
    pub is_throttled: Option<bool>,
    #[serde(default)]
    pub retry_after_ms: Option<u64>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeToolUse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub content_block: Option<ClaudeToolUseContent>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeToolResult {
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
}
```

#### Codex (`protocol/codex.rs`)

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexEvent {
    #[serde(rename = "thread.created")]
    ThreadCreated(CodexThreadMeta),
    #[serde(rename = "thread.started")]
    ThreadStarted(CodexThreadMeta),
    #[serde(rename = "turn.started")]
    TurnStarted(CodexEmpty),
    #[serde(rename = "turn.completed")]
    TurnCompleted(CodexTurnCompleted),
    #[serde(rename = "turn.failed")]
    TurnFailed(CodexErrorEnvelope),
    Error(CodexErrorEnvelope),
    #[serde(rename = "turn.error")]
    TurnError(CodexErrorEnvelope),
    #[serde(rename = "stream.error")]
    StreamError(CodexErrorEnvelope),
    #[serde(rename = "item.started")]
    ItemStarted(CodexItemEnvelope),
    #[serde(rename = "item.completed")]
    ItemCompleted(CodexItemEnvelope),
    #[serde(rename = "item.tool_use")]
    ItemToolUse(CodexToolItem),
    #[serde(rename = "tool_use")]
    ToolUse(CodexToolItem),
    #[serde(rename = "item.tool_result")]
    ItemToolResult(CodexToolItem),
    #[serde(rename = "tool_result")]
    ToolResult(CodexToolItem),
    // Agent messages from streaming
    #[serde(rename = "agent_message")]
    AgentMessage(CodexAgentMessage),
    // Reasoning events
    Reasoning(CodexReasoning),
}

#[derive(Debug, Deserialize)]
pub struct CodexThreadMeta {
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexEmpty {}

#[derive(Debug, Deserialize)]
pub struct CodexTurnCompleted {
    #[serde(default)]
    pub usage: Option<CodexUsage>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cached_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CodexErrorEnvelope {
    #[serde(default)]
    pub error_type: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    pub error: Option<CodexErrorDetail>,
}

#[derive(Debug, Deserialize)]
pub struct CodexErrorDetail {
    #[serde(default)]
    #[serde(rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexItemEnvelope {
    pub item: Option<CodexItem>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodexItem {
    AgentMessage {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        content: Option<Vec<CodexContentPart>>,
    },
    ToolUse(CodexToolItemFields),
    ToolCall(CodexToolItemFields),
    McpToolCall(CodexToolItemFields),
    WebSearch(CodexToolItemFields),
    CommandExec(CodexToolItemFields),
    PatchApply(CodexToolItemFields),
    ImageGeneration(CodexToolItemFields),
    ViewImage(CodexToolItemFields),
    PermissionRequest(CodexPermissionItem),
    ApprovalRequest(CodexPermissionItem),
    UserInputRequest(CodexPermissionItem),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct CodexToolItemFields {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CodexPermissionItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexContentPart {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodexToolItem {
    #[serde(flatten)]
    pub fields: CodexToolItemFields,
}

#[derive(Debug, Deserialize)]
pub struct CodexAgentMessage {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CodexReasoning {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub summary: Option<serde_json::Value>,
}
```

#### Gemini (`protocol/gemini.rs`)

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GeminiEvent {
    Init(GeminiInit),
    System(GeminiInit),
    Message(GeminiMessage),
    Error(GeminiErrorEvent),
    Result(GeminiResult),
    ToolUse(GeminiToolUse),
    ToolResult(GeminiToolResult),
}

#[derive(Debug, Deserialize)]
pub struct GeminiInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiMessage {
    #[serde(default)]
    pub role: Option<String>,
    /// Gemini emits content as a plain string, not an array.
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub delta: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiErrorEvent {
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiResult {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub error: Option<GeminiResultError>,
    #[serde(default)]
    pub stats: Option<GeminiStats>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiResultError {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiStats {
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cached: Option<u64>,
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub tool_calls: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiToolUse {
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GeminiToolResult {
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<String>,
}
```

#### OpenCode (`protocol/opencode.rs`)

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenCodeEvent {
    Init(OpenCodeInit),
    SessionStart(OpenCodeInit),
    StepStart(OpenCodeStepStart),
    Text(OpenCodeText),
    TextDelta(OpenCodeText),
    #[serde(rename = "assistant_text")]
    AssistantText(OpenCodeText),
    StepFinish(OpenCodeStepFinish),
    StepComplete(OpenCodeStepComplete),
    #[serde(rename = "turn_complete")]
    TurnComplete(OpenCodeStepComplete),
    Error(OpenCodeError),
    #[serde(rename = "step_error")]
    StepError(OpenCodeError),
    ToolUse(OpenCodeToolEvent),
    #[serde(rename = "tool_start")]
    ToolStart(OpenCodeToolEvent),
    ToolResult(OpenCodeToolResult),
    #[serde(rename = "tool_end")]
    ToolEnd(OpenCodeToolResult),
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeStepStart {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(rename = "sessionID", default)]
    pub session_id_alt: Option<String>,
    #[serde(default)]
    pub part: Option<serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeText {
    /// Real format: nested under "part"
    #[serde(default)]
    pub part: Option<OpenCodeTextPart>,
    /// Legacy format: flat "text" field
    #[serde(default)]
    pub text: Option<String>,
    /// Fallback: "content" field
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub timestamp: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeTextPart {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeStepFinish {
    #[serde(default)]
    pub part: Option<OpenCodeStepFinishPart>,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeStepFinishPart {
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub tokens: Option<OpenCodeTokens>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeTokens {
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
    #[serde(default)]
    pub reasoning: Option<u64>,
    #[serde(default)]
    pub cache: Option<OpenCodeCache>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeCache {
    #[serde(default)]
    pub read: Option<u64>,
    #[serde(default)]
    pub write: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeStepComplete {
    #[serde(default)]
    pub usage: Option<OpenCodeUsage>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeError {
    #[serde(default)]
    pub error_type: Option<String>,
    pub error: Option<OpenCodeErrorDetail>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeToolEvent {
    /// OpenCode nests most fields under "part"
    #[serde(default)]
    pub part: Option<OpenCodeToolPart>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(rename = "toolName", default)]
    pub tool_name_camel: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeToolPart {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(rename = "toolName", default)]
    pub tool_name_camel: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "tool_use_id", default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct OpenCodeToolResult {
    #[serde(flatten)]
    pub event: OpenCodeToolEvent,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}
```

#### Qwen (`protocol/qwen.rs`)

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QwenEvent {
    Init(QwenInit),
    System(QwenSystem),
    Message(QwenMessage),
    #[serde(rename = "assistant_message")]
    AssistantMessage(QwenMessage),
    #[serde(rename = "assistant")]
    Assistant(QwenMessage),
    Error(QwenErrorEvent),
    Result(QwenResult),
    #[serde(rename = "summary")]
    Summary(QwenResult),
    #[serde(rename = "tool_use")]
    ToolUse(QwenToolUse),
    #[serde(rename = "tool_call")]
    ToolCall(QwenToolUse),
    #[serde(rename = "tool_result")]
    ToolResult(QwenToolResult),
    #[serde(rename = "tool_response")]
    ToolResponse(QwenToolResult),
}

#[derive(Debug, Deserialize)]
pub struct QwenInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QwenSystem {
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QwenMessage {
    #[serde(default)]
    pub role: Option<String>,
    /// Content as array of text blocks (Gemini-style)
    #[serde(default)]
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QwenErrorEvent {
    pub error: Option<QwenErrorDetail>,
}

#[derive(Debug, Deserialize)]
pub struct QwenErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QwenResult {
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u64>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub stats: Option<QwenUsage>,
    #[serde(default)]
    pub usage: Option<QwenUsage>,
    #[serde(rename = "token_usage", default)]
    pub token_usage: Option<QwenUsage>,
}

#[derive(Debug, Deserialize)]
pub struct QwenUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct QwenToolUse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
    #[serde(default)]
    pub args: Option<serde_json::Value>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QwenToolResult {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}
```

#### Kimi (`protocol/kimi.rs`)

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KimiEvent {
    Init(KimiInit),
    System(KimiInit),
    #[serde(rename = "assistant")]
    Assistant(KimiContent),
    #[serde(rename = "message")]
    Message(KimiContent),
    #[serde(rename = "content")]
    Content(KimiContent),
    #[serde(rename = "ContentPart")]
    ContentPart(KimiContent),
    #[serde(rename = "StatusUpdate")]
    StatusUpdateCapital(KimiStatusUpdate),
    #[serde(rename = "status_update")]
    StatusUpdate(KimiStatusUpdate),
    #[serde(rename = "status")]
    Status(KimiStatusUpdate),
    Error(KimiErrorEvent),
    ToolUse(KimiToolUse),
    ToolResult(KimiToolResult),
}

#[derive(Debug, Deserialize)]
pub struct KimiInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KimiContent {
    /// Content array with text parts
    #[serde(default)]
    pub content: Option<Vec<KimiTextPart>>,
    /// Direct text field
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KimiTextPart {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KimiStatusUpdate {
    #[serde(default)]
    pub usage: Option<KimiUsage>,
    #[serde(rename = "token_usage", default)]
    pub token_usage: Option<KimiUsage>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u64>,
    #[serde(default)]
    pub context_usage: Option<KimiContextUsage>,
    #[serde(default)]
    pub context: Option<KimiContextUsage>,
}

#[derive(Debug, Deserialize)]
pub struct KimiUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct KimiContextUsage {
    #[serde(default)]
    pub used: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub percent: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct KimiErrorEvent {
    pub error: Option<KimiErrorDetail>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KimiErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KimiToolUse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
    #[serde(default)]
    pub args: Option<serde_json::Value>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct KimiToolResult {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}
```

### Integration with Existing Parsers

The typed event structs are deserialized *first*, then the parser maps the typed data into its accumulated state. This is a thin layer — the parser retains all its existing logic for state accumulation, sink dispatch, and summary production.

The `feed_line()` method in each parser changes from:

```rust
// BEFORE
let obj: Value = serde_json::from_str(line)?;
let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
match event_type {
    "init" | "system" => {
        self.handle_init(&obj);
        Ok(None)
    }
    "error" | "assistant.error" => {
        self.handle_error(&obj);
        Ok(None)
    }
    // ... more arms with manual .get() chains
}
```

To:

```rust
// AFTER
use super::protocol::claude::ClaudeEvent;

let event: Result<ClaudeEvent, _> = serde_json::from_str(line);
match event {
    Ok(ClaudeEvent::Init(init)) | Ok(ClaudeEvent::System(init)) => {
        self.handle_init(init);
        Ok(None)
    }
    Ok(ClaudeEvent::Error(err)) | Ok(ClaudeEvent::AssistantError(err)) => {
        self.handle_error(&err.error);
        Ok(None)
    }
    // ... typed arms
    Err(_) => {
        // Fallback: parse as raw Value for unknown/future event types
        // This preserves the current behavior where unknown events are silently skipped
        if let Ok(obj) = serde_json::from_str::<Value>(line) {
            let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
            trace!("unknown Claude event type: {event_type}");
        }
        Ok(None)
    }
}
```

The handler methods change signature from `fn handle_init(&mut self, obj: &Value)` to `fn handle_init(&mut self, init: ClaudeInit)` and use direct field access instead of `.get()` chains:

```rust
// BEFORE
fn handle_init(&mut self, obj: &Value) {
    self.session_id = obj.get("session_id").and_then(|v| v.as_str()).map(String::from);
    self.model = obj.get("model").and_then(|v| v.as_str()).map(String::from);
}

// AFTER
fn handle_init(&mut self, init: ClaudeInit) {
    self.session_id = init.session_id;
    self.model = init.model;
}
```

### Malformed Line Handling Strategy

The existing parsers use `Value` as an intermediate step, which means `serde_json::from_str` never fails for valid JSON — only for truly malformed lines. With typed deserialization, there are **two** failure modes:

1. **Truly malformed JSON** — not valid JSON at all (e.g. `this is not json {{{`)
2. **Valid JSON but unknown/changed event shape** — valid JSON that doesn't match any typed variant

The strategy:

- Mode 1 is handled identically to today (return `StreamParseError::MalformedLine` for most parsers, `StreamParseError::Fatal` for Codex which uses a different error variant)
- Mode 2 is handled by the `Err(_)` fallback arm in `feed_line()`, which re-parses as `Value` for tracing and then returns `Ok(None)` (skipped silently, same as today)

This ensures **zero behavior change** for events that don't match the typed model — they are still silently skipped, preserving backward compatibility.

### Important: No `#[serde(deny_unknown_fields)]`

All typed structs use `#[serde(default)]` on every field instead of `deny_unknown_fields`. This is critical because:

1. Provider output formats evolve independently of Claudine releases
2. Adding a new field to Claude's `result` event should not break deserialization
3. The typed structs are a *lower bound* on the fields we care about, not a strict schema

## Migration Strategy

### Phase 1: Foundation (Claude + Codex)

**Why these two first**: Claude and Codex have the most stable, well-documented JSON contracts. Claude's `stream-json` format is the primary target. Codex's JSONL format is the second most commonly used.

**Steps**:

1. Create `stream/protocol/mod.rs` with `ProtocolError`
2. Create `stream/protocol/claude.rs` with the `ClaudeEvent` enum and all sub-structs
3. Migrate `stream/claude.rs` to use `ClaudeEvent` deserialization in `feed_line()`
4. Run existing tests (all 18 tests in `claude.rs`) to confirm zero behavior change
5. Create `stream/protocol/codex.rs` with the `CodexEvent` enum
6. Migrate `stream/codex.rs` to use `CodexEvent` deserialization
7. Run existing tests to confirm zero behavior change
8. Add new tests verifying typed deserialization of all event variants (see Testing section)

### Phase 2: Gemini + OpenCode

1. Create `stream/protocol/gemini.rs` and migrate `stream/gemini.rs`
2. Create `stream/protocol/opencode.rs` and migrate `stream/opencode.rs`
3. OpenCode is the **highest risk parser** due to nested `part` objects (fields appear at top-level OR nested under `part`), camelCase aliases (`toolName`, `sessionID`), accumulation-based usage semantics, and ~50 extraction sites

### Phase 3: Qwen + Kimi

1. Create `stream/protocol/qwen.rs` and migrate `stream/qwen.rs`
2. Create `stream/protocol/kimi.rs` and migrate `stream/kimi.rs`
3. Both Qwen and Kimi tool helpers check 5 keys for input (`input`/`parameters`/`arguments`/`args`/`params`) — all 5 aliases must be present in typed structs

### Phase 4: Cleanup

1. Remove any remaining `use serde_json::Value` imports from parsers that no longer need them
2. Audit for any `Value` extraction sites that were missed
3. Update `mod.rs` to re-export protocol types if needed by consumers
4. Consider extracting shared tool-use structs across providers (all 6 parsers track tool_id, tool_name, tool_input with the same semantics)

## Testing Strategy

### Existing Tests (Must Pass Unchanged)

All existing parser tests must pass without modification. These are the primary regression guard:

| Parser | Test count | Key scenarios |
|--------|----------:|---------------|
| `claude.rs` | 18 | Happy path, error, rate limit, tool events, content block delta, thinking, malformed recovery, tool_calls None, total_cost_usd, nested message key |
| `codex.rs` | 4 | Thread lifecycle, error handling, tool counting, text accumulation |
| `gemini.rs` | 8 | Real format, user messages filtered, error severity, result error status, tool correlation, stats override, content as array |
| `opencode.rs` | 9 | Usage accumulation across steps, real NDJSON, tool extraction with nested part, step boundaries, step failure |
| `qwen.rs` | 5 | Happy path, Qwen-specific names, session_start subtype, string content, tool contract |
| `kimi.rs` | 5 | Status update last-snapshot, context pressure warning, no warning below threshold, missing fields, tool contract |

### New Tests: Typed Deserialization

Add a `stream/protocol/` test module that verifies each provider's typed enums deserialize correctly from real provider output samples:

```rust
#[test]
fn claude_error_event_deserializes() {
    let json = r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#;
    let event: ClaudeEvent = serde_json::from_str(json).unwrap();
    match event {
        ClaudeEvent::Error(err) => {
            assert_eq!(err.error.kind.as_deref(), Some("billing_error"));
            assert_eq!(err.error.message.as_deref(), Some("Insufficient credits"));
        }
        _ => panic!("Expected Error variant"),
    }
}
```

### New Tests: Unknown Event Graceful Degradation

```rust
#[test]
fn claude_unknown_event_type_falls_through() {
    let json = r#"{"type":"future_event_type","data":"something"}"#;
    let result: Result<ClaudeEvent, _> = serde_json::from_str(json);
    // Should fail to deserialize (unknown variant), not panic
    assert!(result.is_err());
}
```

### New Tests: Field Evolution Tolerance

```rust
#[test]
fn claude_init_tolerates_unknown_fields() {
    let json = r#"{"type":"init","session_id":"s1","model":"claude-4","new_field_from_future":"value"}"#;
    let event: ClaudeEvent = serde_json::from_str(json).unwrap();
    match event {
        ClaudeEvent::Init(init) => {
            assert_eq!(init.session_id.as_deref(), Some("s1"));
        }
        _ => panic!("Expected Init variant"),
    }
}
```

## Scope Boundaries

### In Scope

- Typed event enums/structs for all 6 stream parsers
- Deserialization layer between raw JSON and parser handler methods
- Graceful degradation for unknown event types
- Full test coverage of typed deserialization
- Preservation of all existing parser behavior

### Out of Scope

- Changes to `StreamParser` trait or `StreamEventSink`
- Changes to `StreamExecutionSummary` or `NormalizedTokenUsage`
- Changes to `EventMeta` or `StreamChunk`
- Renaming or restructuring the existing parser files
- Adding new event types not currently handled
- Changes to the `output.rs` stderr rendering (covered by the Session Badge feature)
- Protocol-level validation beyond serde deserialization (e.g. semantic checks on field values)

## File Change Summary

| File | Action | Description |
|------|--------|-------------|
| `stream/protocol/mod.rs` | Create | Module root, `ProtocolError`, shared re-exports |
| `stream/protocol/claude.rs` | Create | `ClaudeEvent` enum + all sub-structs |
| `stream/protocol/codex.rs` | Create | `CodexEvent` enum + all sub-structs |
| `stream/protocol/gemini.rs` | Create | `GeminiEvent` enum + all sub-structs |
| `stream/protocol/opencode.rs` | Create | `OpenCodeEvent` enum + all sub-structs |
| `stream/protocol/qwen.rs` | Create | `QwenEvent` enum + all sub-structs |
| `stream/protocol/kimi.rs` | Create | `KimiEvent` enum + all sub-structs |
| `stream/mod.rs` | Edit | Add `pub mod protocol;` |
| `stream/claude.rs` | Edit | Use `ClaudeEvent` in `feed_line()`, update handler signatures |
| `stream/codex.rs` | Edit | Use `CodexEvent` in `feed_line()`, update handler signatures |
| `stream/gemini.rs` | Edit | Use `GeminiEvent` in `feed_line()`, update handler signatures |
| `stream/opencode.rs` | Edit | Use `OpenCodeEvent` in `feed_line()`, update handler signatures |
| `stream/qwen.rs` | Edit | Use `QwenEvent` in `feed_line()`, update handler signatures |
| `stream/kimi.rs` | Edit | Use `KimiEvent` in `feed_line()`, update handler signatures |

**Total: 7 new files, 7 edited files.**

## Estimated Effort

| Phase | Providers | Estimated LOC (protocol) | Estimated LOC (parser edits) | Effort |
|-------|-----------|------------------------:|----------------------------:|--------|
| 1 | Claude + Codex | ~350 | ~200 | Medium-High |
| 2 | Gemini + OpenCode | ~350 | ~250 | High (OpenCode is riskiest) |
| 3 | Qwen + Kimi | ~280 | ~150 | Medium |
| 4 | Cleanup | 0 | ~50 | Low |

The work is mechanical but the scale is larger than initially estimated: ~230 extraction sites (not ~134) across 49 existing tests. OpenCode requires the most care due to its nested `part` pattern and camelCase aliases.

## Future Benefits

Once this is in place:

- **Contract drift detection**: If a provider changes their JSON format, `serde` will fail at deserialization time, producing a clear error instead of silently returning `None`
- **Easier event addition**: Adding support for a new event type is adding a struct variant and a match arm, not writing a new handler with `.get()` chains
- **Cross-provider refactoring**: The typed structs make it possible to extract shared patterns (e.g., all providers have error events with kind+message, all have tool events with id+name+input)
- **Documentation value**: The struct definitions serve as living documentation of each provider's JSON contract
- **Foundation for Session Badge**: The `ClaudeError.kind` field being a named enum variant makes it trivial to match `billing_error` → badge severity `Error` → remediation URL from `Provider::usage_dashboard_url()`
