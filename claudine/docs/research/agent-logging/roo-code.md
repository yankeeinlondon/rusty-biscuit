---
last_updated: "2026-04-29"
has_official_schema: true
schema_url: https://github.com/RooCodeInc/Roo-Code/tree/main/packages/types/src
---

# Roo Code Logging

## Introduction to Roo Code Logging

Roo Code maintains task history and conversation data through **JSON file storage** in the VS Code extension global state directory. It does not use SQLite or any database. Roo Code also provides a structured JSON stream protocol for CLI consumers. There is no dedicated "log file" in the traditional sense; instead, task history, API conversations, and UI messages are persisted as discrete JSON files per task.

### Log Locations

Roo Code stores all task data in the VS Code extension's global storage directory. The exact path depends on the operating system:

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/` |
| Linux | `~/.config/Code/User/globalStorage/rooveterinaryinc.roo-cline/` |
| Windows | `%APPDATA%/Code/User/globalStorage/rooveterinaryinc.roo-cline/` |

Additional global configuration and custom tools/rules live under `~/.roo/`.

### Organization and Structure

```
globalStorage/rooveterinaryinc.roo-cline/
├── tasks/
│   ├── _index.json                          # Task index (all tasks metadata)
│   ├── {uuid}/                              # Per-task directory
│   │   ├── api_conversation_history.json    # Raw API messages (user/assistant turns)
│   │   ├── ui_messages.json                 # ClineMessage array (UI-level events)
│   │   ├── history_item.json                # Task metadata (HistoryItem schema)
│   │   ├── task_metadata.json               # File tracking metadata
│   │   └── checkpoints/                     # Git checkpoint data
│   └── ...
├── cache/                                   # Codebase index cache
├── settings/                                # Extension settings
└── puppeteer/                               # Browser automation data
```

### How Logs Are Organized, Split, and Archived

- **Task Index** (`_index.json`): A single flat JSON file containing a `version` field, `updatedAt` timestamp, and an `entries` array with one `HistoryItem` per task. On the host machine this file contained 383 entries. There is no archival or rotation mechanism; the index grows indefinitely.
- **Per-Task Directories**: Each task is stored in a UUID-named directory. Tasks are never merged or split; each task is an independent unit.
- **No Log Rotation**: Roo Code does not implement log rotation, archival, or compaction. Historical tasks accumulate until manually deleted from the VS Code UI.

### Log File Format

All storage files are JSON:

| File | Format | Description |
|---|---|---|
| `_index.json` | `{"version": 1, "updatedAt": number, "entries": [HistoryItem...]}` | Global task index |
| `api_conversation_history.json` | `[{role, content: [{type, text}]}]` | OpenAI-style API conversation turns |
| `ui_messages.json` | `[ClineMessage...]` | All UI-level messages for the task |
| `history_item.json` | `HistoryItem` | Single task metadata object |
| `task_metadata.json` | `{files_in_context: [{path, record_state, record_source, ...}]}` | File tracking |

### Database Usage

Roo Code does **not** use SQLite or any database engine. All persistent state is stored as flat JSON files on disk.

### Major Message Types

Roo Code classifies messages into two top-level categories, each with sub-variants:

**Ask messages** (`type: "ask"`) — require user interaction or approval:

| Sub-type | Description |
|---|---|
| `followup` | Clarifying question |
| `command` | Permission to execute a terminal command |
| `command_output` | Permission to read command output |
| `completion_result` | Task completed, awaiting feedback |
| `tool` | Permission to use a file/tool operation |
| `api_req_failed` | API failure, retry prompt |
| `resume_task` | Resume a paused task |
| `resume_completed_task` | Resume a completed task |
| `mistake_limit_reached` | Error limit reached |
| `use_mcp_server` | Permission for MCP server use |
| `auto_approval_max_req_reached` | Auto-approve limit reached |

**Say messages** (`type: "say"`) — informational from the assistant:

| Sub-type | Description |
|---|---|
| `error` | General error |
| `api_req_started` | API request initiated |
| `api_req_finished` | API request completed |
| `api_req_retried` | API request retry |
| `api_req_retry_delayed` | Retry delayed |
| `api_req_rate_limit_wait` | Rate limit wait |
| `api_req_deleted` | Request cancelled |
| `text` | General text response |
| `image` | Image content |
| `reasoning` | Model reasoning/thinking |
| `completion_result` | Final result |
| `user_feedback` | User feedback text |
| `user_feedback_diff` | Diff-formatted feedback |
| `command_output` | Command execution output |
| `shell_integration_warning` | Shell integration issue |
| `mcp_server_request_started` | MCP server request started |
| `mcp_server_response` | MCP server response |
| `subtask_result` | Subtask completion result |
| `checkpoint_saved` | Git checkpoint saved |
| `rooignore_error` | .rooignore processing error |
| `diff_error` | Diff application error |
| `condense_context` | Context condensation event |
| `condense_context_error` | Condensation failure |
| `sliding_window_truncation` | Context truncation event |
| `codebase_search_result` | Codebase search results |
| `user_edit_todos` | Todo list edit |
| `too_many_tools_warning` | Too many MCP tools warning |
| `tool` | Tool usage information |

## Logging Schema

### Official Schema

Roo Code defines official Zod schemas in the `@roo-code/types` package. The authoritative source files are:

| Schema | File | URL |
|---|---|---|
| `ClineMessage`, `ClineAsk`, `ClineSay` | `packages/types/src/message.ts` | [message.ts](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/message.ts) |
| `HistoryItem` | `packages/types/src/history.ts` | [history.ts](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/history.ts) |
| `RooCliStreamEvent`, `RooCliCost`, `RooCliToolUse`, `RooCliToolResult` | `packages/types/src/cli.ts` | [cli.ts](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/cli.ts) |
| `TokenUsage`, `QueuedMessage`, `ToolProgressStatus` | `packages/types/src/message.ts` | [message.ts](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/message.ts) |
| `TaskProviderEvents`, `TaskEvents` | `packages/types/src/task.ts` | [task.ts](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/task.ts) |
| `RooCodeEventName` | `packages/types/src/events.ts` | [events.ts](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/events.ts) |

### Rust Schema Representation

The following Rust structs and enums model Roo Code's logging schema, derived from the official Zod schemas:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub root_task_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub number: u64,
    pub ts: u64,
    pub task: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cache_writes: Option<u64>,
    pub cache_reads: Option<u64>,
    pub total_cost: f64,
    pub size: Option<u64>,
    pub workspace: Option<String>,
    pub mode: Option<String>,
    pub api_config_name: Option<String>,
    pub status: Option<TaskStatus>,
    pub delegated_to_id: Option<String>,
    pub child_ids: Option<Vec<String>>,
    pub awaiting_child_id: Option<String>,
    pub completed_by_child_id: Option<String>,
    pub completion_result_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    active,
    completed,
    delegated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskIndex {
    pub version: u64,
    pub updated_at: u64,
    pub entries: Vec<HistoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClineMessage {
    pub ts: u64,
    #[serde(rename = "type")]
    pub message_type: MessageDirection,
    pub ask: Option<ClineAsk>,
    pub say: Option<ClineSay>,
    pub text: Option<String>,
    pub images: Option<Vec<String>>,
    pub partial: Option<bool>,
    pub reasoning: Option<String>,
    pub conversation_history_index: Option<u64>,
    pub checkpoint: Option<serde_json::Value>,
    pub progress_status: Option<ToolProgressStatus>,
    pub context_condense: Option<ContextCondense>,
    pub context_truncation: Option<ContextTruncation>,
    pub is_protected: Option<bool>,
    pub api_protocol: Option<ApiProtocol>,
    pub is_answered: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Ask,
    Say,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineAsk {
    Followup,
    Command,
    CommandOutput,
    CompletionResult,
    Tool,
    ApiReqFailed,
    ResumeTask,
    ResumeCompletedTask,
    MistakeLimitReached,
    UseMcpServer,
    AutoApprovalMaxReqReached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClineSay {
    Error,
    ApiReqStarted,
    ApiReqFinished,
    ApiReqRetried,
    ApiReqRetryDelayed,
    ApiReqRateLimitWait,
    ApiReqDeleted,
    Text,
    Image,
    Reasoning,
    CompletionResult,
    UserFeedback,
    UserFeedbackDiff,
    CommandOutput,
    ShellIntegrationWarning,
    McpServerRequestStarted,
    McpServerResponse,
    SubtaskResult,
    CheckpointSaved,
    RooignoreError,
    DiffError,
    CondenseContext,
    CondenseContextError,
    SlidingWindowTruncation,
    CodebaseSearchResult,
    UserEditTodos,
    TooManyToolsWarning,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressStatus {
    pub icon: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextCondense {
    pub cost: f64,
    pub prev_context_tokens: u64,
    pub new_context_tokens: u64,
    pub summary: String,
    pub condense_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTruncation {
    pub truncation_id: String,
    pub messages_removed: u64,
    pub prev_context_tokens: u64,
    pub new_context_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiProtocol {
    openai,
    anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_cache_writes: Option<u64>,
    pub total_cache_reads: Option<u64>,
    pub total_cost: f64,
    pub context_tokens: u64,
}
```

### CLI Stream Event Schema

For the `--output-format stream-json` NDJSON protocol:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RooCliStreamEvent {
    System {
        subtype: Option<String>,
        content: Option<String>,
        schema_version: Option<u64>,
        protocol: Option<String>,
        capabilities: Option<Vec<String>>,
    },
    Control {
        subtype: ControlSubtype,
        request_id: Option<String>,
        command: Option<String>,
        task_id: Option<String>,
        content: Option<String>,
        success: Option<bool>,
        code: Option<String>,
        done: Option<bool>,
    },
    Queue {
        subtype: Option<String>,
        task_id: Option<String>,
        content: Option<String>,
        queue_depth: Option<u64>,
        queue: Option<Vec<RooCliQueueItem>>,
    },
    Assistant {
        id: Option<u64>,
        content: Option<String>,
        done: Option<bool>,
        subtype: Option<String>,
    },
    User {
        id: Option<u64>,
        content: Option<String>,
        done: Option<bool>,
        subtype: Option<String>,
    },
    ToolUse {
        id: Option<u64>,
        content: Option<String>,
        done: Option<bool>,
        subtype: Option<String>,
        tool_use: Option<RooCliToolUse>,
    },
    ToolResult {
        id: Option<u64>,
        content: Option<String>,
        done: Option<bool>,
        subtype: Option<String>,
        tool_result: Option<RooCliToolResult>,
    },
    Thinking {
        id: Option<u64>,
        content: Option<String>,
        done: Option<bool>,
    },
    Error {
        id: Option<u64>,
        content: Option<String>,
    },
    Result {
        id: Option<u64>,
        content: Option<String>,
        done: Option<bool>,
        success: Option<bool>,
        cost: Option<RooCliCost>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlSubtype {
    Ack,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RooCliQueueItem {
    pub id: String,
    pub text: Option<String>,
    pub image_count: Option<u64>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RooCliToolUse {
    pub name: String,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RooCliToolResult {
    pub name: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RooCliCost {
    pub total_cost: Option<f64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_writes: Option<u64>,
    pub cache_reads: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RooCliFinalOutput {
    #[serde(rename = "type")]
    pub output_type: String,
    pub success: bool,
    pub content: Option<String>,
    pub cost: Option<RooCliCost>,
    pub events: Vec<serde_json::Value>,
}
```

## Informational Content versus Hook Events

### When File System Logs Are a Better Source

Roo Code's file-system logs (`ui_messages.json`, `api_conversation_history.json`, `history_item.json`) are the better source when you need:

- **Full conversation fidelity**: The `ui_messages.json` file contains every `ClineMessage` including internal types suppressed from the CLI JSON stream (`api_req_finished`, `checkpoint_saved`, `condense_context`, `sliding_window_truncation`, etc.).
- **Historical analysis**: File logs persist indefinitely and allow querying past sessions without requiring real-time observation.
- **Token usage and cost tracking**: The `_index.json` aggregates `tokensIn`, `tokensOut`, `cacheWrites`, `cacheReads`, and `totalCost` across all tasks. Individual `api_req_started` messages in `ui_messages.json` provide per-request cost breakdowns.
- **Cross-session correlation**: Parent/child task relationships are stored in `history_item.json` via `rootTaskId`, `parentTaskId`, `childIds`, and `delegatedToId`.
- **Workspace and mode context**: Each `HistoryItem` records `workspace` and `mode`, enabling per-project or per-mode analytics.

### When Event Logs (Hook Events) Are a Better Source

Roo Code's event surfaces are the better source when you need:

- **Real-time observation**: CLI stream events (`--output-format stream-json`) provide immediate, line-delimited JSON as the agent executes. File logs are only written after the extension processes them.
- **Structured tool tracking**: The CLI JSON emitter parses tool use into structured `tool_use` and `tool_result` events with explicit `name`, `input`, and `output` fields, which is more convenient than parsing raw `ClineMessage.text` JSON strings from file logs.
- **Streaming deltas**: In `stream-json` mode, partial updates contain only the content delta, reducing bandwidth for high-frequency text streaming.
- **Flow control**: The `ExtensionClient` API events (`waitingForInput`, `stateChange`) provide actionable state transitions. File logs have no flow control semantics.
- **Non-interactive/CI consumption**: The CLI `--print` + `--output-format stream-json` + `--oneshot` combination is purpose-built for machine consumption in pipelines.

### Additional Enrichment Sources

Several other Roo Code surfaces can enrich logging data:

1. **`api_conversation_history.json`**: Contains the raw LLM API request/response pairs including full environment details injected by the extension (visible files, open tabs, workspace directory listing, current mode, model name). This is richer than `ui_messages.json` for understanding what context the model received.

2. **`task_metadata.json`**: Tracks which files were in context, how they were sourced (read tool, edit tool, etc.), and when they were read or edited. Useful for file-access auditing.

3. **`checkpoints/` directory**: Stores Git checkpoint data created during task execution, enabling before/after diff analysis.

4. **CLI debug log** (`~/.roo/cli-debug.log`): Available when the CLI is run with `--debug` / `-d`. Provides verbose internal logging beyond structured events.

5. **VS Code extension API** (`RooCodeAPI`): EventEmitter-based interface exposing task lifecycle events (`taskCreated`, `taskStarted`, `taskCompleted`, `taskAborted`), subtask events (`taskSpawned`, `taskDelegated`, `taskDelegationCompleted`), and per-message events. This is the most granular real-time surface but requires running inside VS Code.

6. **Telemetry properties** (`packages/types/src/telemetry.ts`): Static app properties and git properties that provide environment context alongside events.

## Sources

- [Roo Code Documentation](https://docs.roocode.com)
- [Roo Code GitHub Repository](https://github.com/RooCodeInc/Roo-Code)
- [Message Types (ClineMessage, ClineAsk, ClineSay)](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/message.ts)
- [History Item Schema](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/history.ts)
- [CLI Stream Event Schema](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/cli.ts)
- [Task Types (TaskProviderEvents, TaskEvents)](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/task.ts)
- [Event Definitions (RooCodeEventName)](https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/events.ts)
- [CLI JSON Event Emitter](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/json-event-emitter.ts)
- [CLI Extension Client](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/extension-client.ts)
- [CLI State Store](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/state-store.ts)
- [CLI README](https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/README.md)
- [Roo Code Custom Instructions](https://docs.roocode.com/features/custom-instructions)
- [Roo Code Auto-Approve Settings](https://docs.roocode.com/features/auto-approving-actions)
