---
last_updated: "2026-04-29"
has_official_schema: true
schema_url: "https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/types.ts"
---

# Qwen CLI Logging

## Introduction to Qwen CLI Logging

Qwen CLI (also known as Qwen Code, `qwen-code`, npm package `@qwen-code/qwen-code`) is a TypeScript-based agentic coding CLI. It was originally forked from Google's Gemini CLI and retains much of its telemetry infrastructure. Logging in Qwen CLI operates across three distinct surfaces: session transcripts (JSONL chat history), OpenTelemetry-based telemetry (logs + metrics), and hook events (lifecycle callbacks). There is no SQLite database; all persistent data is file-based.

### Log Locations

Qwen CLI stores its data under `~/.qwen/` with the following structure:

| Path | Description |
|------|-------------|
| `~/.qwen/settings.json` | User-global configuration (including telemetry settings) |
| `~/.qwen/projects/<sanitized-cwd>/chats/<session-id>.jsonl` | Session transcripts (JSONL), one file per session |
| `~/.qwen/tmp/<project-hash>/logs.json` | Lightweight per-project session logs (JSON array) |
| `~/.qwen/tmp/<project-hash>/shell_history` | Shell command history for the project |
| `.qwen/telemetry.log` | OpenTelemetry file output (when `target: "local"`, configurable path) |
| `~/.qwen/tmp/<project-hash>/otel/collector.log` | OTLP collector output (advanced collector-based export) |

The `<sanitized-cwd>` in the projects directory is derived from the working directory where `qwen` was launched, with path separators replaced by dashes (e.g., `-Users-ken--claudine-worktrees-rusty-biscuit-sniff-sniff`). The `<project-hash>` is a SHA-256 hash of the project root path.

Session transcript files are **not archived or rotated** by default -- they accumulate indefinitely under `chats/`. There is no built-in log rotation or compaction for the JSONL transcript files.

### Log File Formats

**Session Transcripts (JSONL):** Each line is a self-contained JSON object. The format uses a linked-list structure with `uuid` and `parentUuid` fields:

```json
{
  "uuid": "45814d3d-...",
  "parentUuid": null,
  "sessionId": "0c78c02b-...",
  "timestamp": "2026-04-21T22:41:05.285Z",
  "type": "system",
  "cwd": "/Users/ken/...",
  "version": "0.9.0",
  "gitBranch": "main",
  "subtype": "slash_command",
  "systemPayload": {
    "phase": "invocation",
    "rawCommand": "/auth"
  }
}
```

Observed `type` values include: `system`, `user`, `assistant`, `tool`, `result`. Observed `subtype` values include: `slash_command`, `session_start`, `success`, `error`.

**OpenTelemetry Logs:** When telemetry is enabled with file output, logs are written as pretty-printed JSON (2-space indented), one JSON object per write batch. The format follows the OpenTelemetry `ReadableLogRecord` structure with `body` (string) and `attributes` (key-value map) fields.

**Per-Project Session Logs (`logs.json`):** A simple JSON array of objects with `sessionId`, `messageId`, `type`, `message`, and `timestamp` fields.

### Database Usage

Qwen CLI does **not** use SQLite or any other database. All state is stored as flat files (JSON, JSONL) on disk.

### Major Log Event Types

Qwen CLI distinguishes the following categories of log events (from the OpenTelemetry telemetry layer, defined in [`packages/core/src/telemetry/constants.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/constants.ts)):

| Event Name | Category | Description |
|------------|----------|-------------|
| `qwen-code.config` | Session | CLI configuration at startup |
| `qwen-code.user_prompt` | User Input | User submits a prompt |
| `qwen-code.user_retry` | User Input | User retries a prompt |
| `qwen-code.tool_call` | Tool Usage | Each tool/function call |
| `qwen-code.file_operation` | Tool Usage | File create/read/update |
| `qwen-code.tool_output_truncated` | Tool Usage | Tool output exceeded threshold |
| `qwen-code.api_request` | API | Request to LLM API |
| `qwen-code.api_response` | API | Response from LLM API |
| `qwen-code.api_error` | API | API request failure |
| `qwen-code.api_cancel` | API | API request cancelled |
| `qwen-code.malformed_json_response` | API | Unparseable JSON from API |
| `qwen-code.flash_fallback` | Fallback | Switched to flash model |
| `qwen-code.ripgrep_fallback` | Fallback | Switched from ripgrep to grep |
| `qwen-code.slash_command` | Command | Slash command execution |
| `qwen-code.chat_compression` | Session | Context compression |
| `qwen-code.conversation_finished` | Session | Conversation ended |
| `qwen-code.subagent_execution` | Agent | Subagent start/stop |
| `qwen-code.hook_call` | Hooks | Hook execution |
| `qwen-code.auth` | Auth | Authentication event |
| `qwen-code.skill_launch` | Features | Skill invocation |
| `qwen-code.prompt_suggestion` | UX | Prompt suggestion outcome |
| `qwen-code.speculation` | UX | Speculative execution |
| `qwen-code.memory.extract` | Memory | Auto-memory extraction |
| `qwen-code.memory.dream` | Memory | Memory deduplication |
| `qwen-code.memory.recall` | Memory | Memory retrieval |
| `qwen-code.arena_session_started` | Arena | Arena session start |
| `qwen-code.arena_agent_completed` | Arena | Arena agent completed |
| `qwen-code.arena_session_ended` | Arena | Arena session ended |
| `qwen-code.ide_connection` | IDE | IDE connection event |
| `qwen-code.extension_*` | Extensions | Extension lifecycle events |
| `qwen-code.user_feedback` | Feedback | User rating event |
| `qwen-code.loop_detected` | Debug | Loop detection triggered |

## Logging Schema

### Official Schema

Qwen CLI has an **official, source-code-level schema** defined in TypeScript in the Qwen Code repository. The authoritative source is:

- [`packages/core/src/telemetry/types.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/types.ts) -- All telemetry event class definitions
- [`packages/core/src/telemetry/constants.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/constants.ts) -- Event name string constants
- [`packages/core/src/telemetry/loggers.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/loggers.ts) -- Logging functions that emit OTel records and record metrics
- [`packages/core/src/telemetry/metrics.ts`](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/metrics.ts) -- Metric definitions

There is no standalone JSON Schema or OpenAPI-style schema document. The schema exists as TypeScript class definitions that are serialized to JSON attributes via the OpenTelemetry SDK. The official documentation at [Telemetry](https://qwenlm.github.io/qwen-code-docs/en/developers/development/telemetry/) provides a human-readable listing of log events and metrics with their attribute schemas.

### Representative Rust Schema

Below is a representative Rust schema derived from the official TypeScript types. Each event shares a common `BaseTelemetryEvent` structure with `event_name` and `event_timestamp`.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseTelemetryEvent {
    #[serde(rename = "event.name")]
    pub event_name: String,
    #[serde(rename = "event.timestamp")]
    pub event_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub session_id: String,
    pub model: String,
    pub sandbox_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_tools_enabled: Option<String>,
    pub approval_mode: String,
    pub debug_enabled: bool,
    pub truncate_tool_output_threshold: u64,
    pub truncate_tool_output_lines: u64,
    pub mcp_servers: String,
    pub telemetry_enabled: bool,
    pub file_filtering_respect_git_ignore: bool,
    pub mcp_servers_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_tools_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_tools: Option<String>,
    pub output_format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<String>,
    pub ide_enabled: bool,
    pub interactive_shell_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagents: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub prompt_length: u64,
    pub prompt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Success,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallDecision {
    Accept,
    Reject,
    AutoAccept,
    Modify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Native,
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub function_name: String,
    #[serde(default)]
    pub function_args: serde_json::Value,
    pub duration_ms: u64,
    pub status: ToolCallStatus,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<ToolCallDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    pub prompt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    pub tool_type: ToolType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_length: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponseEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub response_id: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_code: Option<serde_json::Value>,
    pub duration_ms: u64,
    pub input_token_count: u64,
    pub output_token_count: u64,
    pub cached_content_token_count: u64,
    pub thoughts_token_count: u64,
    pub tool_token_count: u64,
    pub total_token_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_text: Option<String>,
    pub prompt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    pub model: String,
    pub duration_ms: u64,
    pub prompt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,
    pub error_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_code: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subagent_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommandEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subcommand: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<SlashCommandStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlashCommandStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOperationEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub tool_name: String,
    pub operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mimetype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub programming_language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentExecutionEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub subagent_name: String,
    pub status: SubagentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminate_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentStatus {
    Started,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCallEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub hook_event_name: String,
    pub hook_type: HookTelemetryType,
    pub hook_name: String,
    pub hook_input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hook_output: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTelemetryType {
    Command,
    Http,
    Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompressionEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub tokens_before: u64,
    pub tokens_after: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_input_token_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_output_token_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEvent {
    #[serde(flatten)]
    pub base: BaseTelemetryEvent,
    pub auth_type: String,
    pub action_type: AuthActionType,
    pub status: AuthStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthActionType {
    Auto,
    Manual,
    CodingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    Success,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event.name")]
pub enum QwenTelemetryEvent {
    #[serde(rename = "qwen-code.config")]
    Config(StartSessionEvent),
    #[serde(rename = "qwen-code.user_prompt")]
    UserPrompt(UserPromptEvent),
    #[serde(rename = "qwen-code.tool_call")]
    ToolCall(ToolCallEvent),
    #[serde(rename = "qwen-code.api_response")]
    ApiResponse(ApiResponseEvent),
    #[serde(rename = "qwen-code.api_error")]
    ApiError(ApiErrorEvent),
    #[serde(rename = "qwen-code.slash_command")]
    SlashCommand(SlashCommandEvent),
    #[serde(rename = "qwen-code.file_operation")]
    FileOperation(FileOperationEvent),
    #[serde(rename = "qwen-code.subagent_execution")]
    SubagentExecution(SubagentExecutionEvent),
    #[serde(rename = "qwen-code.hook_call")]
    HookCall(HookCallEvent),
    #[serde(rename = "qwen-code.chat_compression")]
    ChatCompression(ChatCompressionEvent),
    #[serde(rename = "qwen-code.auth")]
    Auth(AuthEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTranscriptEntry {
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_uuid: Option<String>,
    #[serde(rename = "parentUuid", default, skip_serializing_if = "Option::is_none")]
    pub parent_uuid_legacy: Option<String>,
    pub session_id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_payload: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
```

## Informational Content versus Hook Events

### When Filesystem Logs Are Better

Filesystem logs (session transcripts and telemetry files) are the superior data source when you need:

- **Complete historical data**: Session transcripts capture every message, tool call, and system event for the full session lifetime. Hook events only fire at discrete lifecycle points.
- **Post-hoc analysis**: Reading JSONL transcripts after a session has ended enables cost analysis, token usage auditing, error trend identification, and replay debugging.
- **Token and cost metrics**: The OpenTelemetry telemetry layer records granular `input_token_count`, `output_token_count`, `cached_content_token_count`, `thoughts_token_count`, and `tool_token_count` per API response. Hook events do not expose token counts.
- **API latency and error details**: The `api_response`, `api_error`, and `api_cancel` telemetry events contain `duration_ms`, `status_code`, `error_type`, and `error_message` -- data not available through hooks.
- **Tool call metadata**: Telemetry captures `duration_ms`, `decision` (accept/reject/auto_accept/modify), `tool_type` (native vs MCP), `content_length`, and diff stats (`model_added_lines`, `user_added_lines`, etc.) per tool call. Hooks receive the input but not the full outcome metadata.
- **Metrics aggregation**: OpenTelemetry counters and histograms (`qwen-code.tool.call.count`, `qwen-code.api.request.latency`, `qwen-code.token.usage`) enable time-series analysis that raw hook events cannot provide.

### When Hook Events Are Better

Hook events are the superior data source when you need:

- **Real-time intervention**: Hooks fire synchronously during the agentic loop, allowing you to block, modify, or approve actions before they execute (`PreToolUse` with `permissionDecision: "deny"`). Filesystem logs are write-only and provide no control.
- **Input modification**: The `PreToolUse` hook can return `updatedInput` to rewrite tool parameters before execution. This is impossible with log file analysis.
- **Context injection**: `UserPromptSubmit`, `SessionStart`, `SubagentStart`, and `Stop` hooks can inject `additionalContext` into the conversation dynamically. No log file can achieve this.
- **Permission automation**: The `PermissionRequest` hook enables programmatic approval/denial of tool calls based on policy. This is an active control plane, not a passive observability plane.
- **Zero-overhead observability**: For providers like Claudine that need to react to session lifecycle without parsing large JSONL files, hooks provide a lightweight, event-driven interface.

### Additional Data Sources for Enrichment

Beyond filesystem logs and hook events, these sources can enrich the data Claudine collects:

1. **`stream-json` output format**: Running Qwen CLI with `--output-format stream-json` emits line-delimited JSON events in real-time (`system`, `assistant`, `result` message types) on stdout. This provides a structured, real-time event stream that includes session IDs, model names, usage metadata, and tool call details -- all without requiring filesystem access or telemetry configuration.

2. **`--output-format json`**: Buffered JSON output at session end, containing the complete message array. Useful for post-session analysis when you control the CLI invocation.

3. **Session resume (`--continue` / `--resume`)**: Session transcripts are stored as JSONL under `~/.qwen/projects/<sanitized-cwd>/chats/<session-id>.jsonl`. These can be parsed for full conversation history, including tool inputs/outputs and assistant reasoning.

4. **OpenTelemetry collector integration**: When `telemetry.useCollector` is `true`, Qwen CLI exports to an OTLP collector (Jaeger, Prometheus, etc.), enabling centralized observability with dashboards and alerting. This is the enterprise-grade path.

5. **`/stats` and `/bug` slash commands**: In interactive mode, `/stats` exposes current session token usage and model information. The `/bug` command captures session state for issue reporting.

6. **`model.enableOpenAILogging` + `model.openAILoggingDir`**: When enabled, raw OpenAI-compatible API request/response JSON files are written to a configurable directory, providing the complete LLM protocol exchange for debugging.

## Sources

- [Qwen Code Official Documentation - Overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code Hooks Documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/)
- [Qwen Code Telemetry (OpenTelemetry)](https://qwenlm.github.io/qwen-code-docs/en/developers/development/telemetry/)
- [Qwen Code Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen Code Configuration](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [QwenLM/qwen-code GitHub Repository](https://github.com/QwenLM/qwen-code)
- [Telemetry Types (source)](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/types.ts)
- [Telemetry Constants (source)](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/constants.ts)
- [Telemetry Loggers (source)](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/loggers.ts)
- [Telemetry File Exporters (source)](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/telemetry/file-exporters.ts)
