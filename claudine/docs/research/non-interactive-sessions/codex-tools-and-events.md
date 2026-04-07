---
title: Codex CLI Tools and JSONL Event Types
last_updated: 2026-04-06
source: https://github.com/openai/codex (commit HEAD as of 2026-04-06)
---

# Codex CLI: Built-in Tools and JSONL Event Taxonomy

Source material drawn from the Codex CLI Rust source code at `codex-rs/` in the
[openai/codex](https://github.com/openai/codex) repository, cross-referenced
with the public documentation at
[developers.openai.com/codex](https://developers.openai.com/codex).

---

## 1. Built-in Tools

The Codex CLI provides tools to the model as OpenAI Responses API `function`
tools, OpenAI `custom` (freeform) tools, or special built-in tool types
(`web_search`, `local_shell`, `tool_search`, `image_generation`).

### 1.1 Shell / Command Execution Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `exec_command` | function | Runs a command in a PTY. Returns output or a session ID for ongoing interaction. Supports `cmd`, `workdir`, `shell`, `tty`, `yield_time_ms`, `max_output_tokens`, and optional sandbox/approval parameters. |
| `write_stdin` | function | Writes characters to an existing `exec_command` session and returns recent output. Used for interactive PTY sessions. |
| `shell` | function | Runs a command via `execvp()` (Unix) or `CreateProcessW` (Windows). Takes a `command` array, `workdir`, and `timeout_ms`. |
| `shell_command` | function | Runs a shell script string in the user's default shell. Takes `command` (string), `workdir`, `timeout_ms`. |
| `local_shell` | built-in | A special Responses API tool type `{ "type": "local_shell" }`. Minimal wrapper. |

The `exec_command` tool has a structured output schema:

```json
{
  "wall_time_seconds": 1.2,
  "exit_code": 0,
  "session_id": 42,
  "original_token_count": 500,
  "output": "command output text"
}
```

### 1.2 File Editing Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `apply_patch` (freeform) | custom/grammar | A Lark-grammar freeform tool for GPT-5 class models. Sends raw patch text (not JSON-wrapped). Supports `*** Add File`, `*** Delete File`, `*** Update File` operations with context-aware hunks. |
| `apply_patch` (JSON) | function | A JSON-wrapped variant for other models. Takes `{ "input": "<patch text>" }`. Same patch language. |

The patch language uses a custom diff format:

```
*** Begin Patch
*** Add File: hello.txt
+Hello world
*** Update File: src/app.py
*** Move to: src/main.py
@@ def greet():
-print("Hi")
+print("Hello, world!")
*** Delete File: obsolete.txt
*** End Patch
```

### 1.3 Web Search Tool

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `web_search` | built-in | A special Responses API tool type. Supports `external_web_access` (cached vs live), `filters` (allowed domains), `user_location`, `search_context_size`, and `search_content_types` (text, image). |

### 1.4 Image Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `view_image` | function | Reads a local image file and returns it as a data URL. Takes `path` (required) and optional `detail` ("original" for full resolution). |
| `image_generation` | built-in | A special Responses API tool type `{ "type": "image_generation", "output_format": "..." }`. |

The `view_image` output schema:

```json
{
  "image_url": "data:image/png;base64,...",
  "detail": "original"
}
```

### 1.5 JavaScript REPL Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `js_repl` | custom/grammar | A Lark-grammar freeform tool. Runs JavaScript in a persistent Node kernel with top-level await. Sends raw JS source, optionally with a pragma line like `// codex-js-repl: timeout_ms=15000`. |
| `js_repl_reset` | function | Restarts the js_repl kernel and clears persisted top-level bindings. Takes no parameters. |

### 1.6 Plan / Todo Tool

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `update_plan` | function | Updates the agent's task plan. Takes `explanation` (optional) and `plan` (array of `{ step, status }` objects where status is `pending`, `in_progress`, or `completed`). At most one step can be `in_progress` at a time. |

### 1.7 Directory Listing Tool

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `list_dir` | function | Lists entries in a local directory with 1-indexed entry numbers. Takes `dir_path` (required), `offset`, `limit`, `depth`. |

### 1.8 Multi-Agent (Collaboration) Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `spawn_agent` | function | Spawns a sub-agent for a well-scoped task. Takes `message`, `agent_type`, `model`, `reasoning_effort`, `fork_context` (v1) or `fork_turns` (v2), `task_name` (v2). Returns `agent_id`, `nickname`, and optionally `task_name`. |
| `send_input` | function | Sends a message to an existing agent. Takes `target` (agent ID), `message` or `items`, `interrupt`. |
| `send_message` | function | Adds a message to an agent without triggering a new turn. Takes `target`, `message`. |
| `followup_task` | function | Adds a message to a non-root agent and triggers a turn. Takes `target`, `message`, `interrupt`. |
| `wait_agent` | function | Waits for agents to reach a final status (v1) or a mailbox update (v2). Takes `targets` (v1) or nothing (v2), plus `timeout_ms`. Returns status map and `timed_out` boolean. |
| `close_agent` | function | Closes an agent and any open descendants. Takes `target`. Returns `previous_status`. |
| `resume_agent` | function | Resumes a previously closed agent. Takes `id`. Returns `status`. |
| `list_agents` | function | Lists live agents in the current root thread tree. Takes optional `path_prefix`. Returns array of agents with status. |

### 1.9 Batch Agent Job Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `spawn_agents_on_csv` | function | Processes a CSV by spawning one worker sub-agent per row. Takes `csv_path`, `instruction` (template with `{column}` placeholders), `id_column`, `output_csv_path`, `max_concurrency`, `max_runtime_seconds`, `output_schema`. Blocks until all rows finish. |
| `report_agent_job_result` | function | Worker-only tool to report a result for a job item. Takes `job_id`, `item_id`, `result` (object), and optional `stop` (cancel remaining items). |

### 1.10 MCP Tools (Dynamic)

MCP tools are not built-in; they are dynamically registered from configured MCP
servers. Each MCP server tool is parsed into a `ToolDefinition` with:

- `name` from `tool.name`
- `description` from `tool.description`
- `input_schema` from `tool.input_schema` (with `properties` normalized to `{}` if absent)
- `output_schema` wrapping the MCP `CallToolResult` structure

MCP tools appear in the Responses API via a namespace mechanism. The model sees
them as `<namespace><tool_name>` (e.g., `mcp_server_name__tool_name`). The CLI
routes calls back to the appropriate MCP server.

### 1.11 MCP Resource Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `list_mcp_resources` | function | Lists resources from MCP servers. Takes optional `server` and `cursor`. |
| `list_mcp_resource_templates` | function | Lists parameterized resource templates from MCP servers. Takes optional `server` and `cursor`. |
| `read_mcp_resource` | function | Reads a specific resource from an MCP server. Takes `server` and `uri` (both required). |

### 1.12 Tool Discovery Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `tool_search` | tool_search | Searches over apps/connectors tool metadata with BM25 and exposes matching tools for the next model call. Takes `query` (required) and `limit`. Returns tool definitions in namespace groups. |
| `tool_suggest` | function | Suggests a missing connector or plugin when the user needs a capability not in the active tools list. Takes `tool_type`, `action_type`, `tool_id`, `suggest_reason`. |

### 1.13 Permission / User Input Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `request_permissions` | function | Requests additional filesystem or network permissions from the user. Takes `permissions` (with `network.enabled`, `file_system.read[]`, `file_system.write[]`) and optional `reason`. |
| `request_user_input` | function | Requests user input for 1-3 short questions. Takes `questions` array, each with `id`, `header`, `question`, and `options` (2-3 choices with labels and descriptions). |

### 1.14 Code Mode Tools

| Tool Name | Format | Description |
|-----------|--------|-------------|
| `exec` (code mode) | custom/grammar | A Lark-grammar freeform tool for code-mode execution. Sends raw source with optional `// @exec:` pragma. Wraps nested tool calls into a unified execution model. |
| `wait` (code mode) | function | Waits on a yielded code-mode `exec` cell and returns new output or completion. Takes `cell_id`, `yield_time_ms`, `max_tokens`, `terminate`. |

---

## 2. JSONL Event Type Taxonomy

When `codex exec --json` is used, stdout receives a stream of newline-delimited
JSON objects. Each object has a `"type"` field that discriminates the event kind.

### 2.1 Source Definition

The authoritative Rust definition is in `codex-rs/exec/src/exec_events.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type")]
pub enum ThreadEvent {
    #[serde(rename = "thread.started")]
    ThreadStarted(ThreadStartedEvent),

    #[serde(rename = "turn.started")]
    TurnStarted(TurnStartedEvent),

    #[serde(rename = "turn.completed")]
    TurnCompleted(TurnCompletedEvent),

    #[serde(rename = "turn.failed")]
    TurnFailed(TurnFailedEvent),

    #[serde(rename = "item.started")]
    ItemStarted(ItemStartedEvent),

    #[serde(rename = "item.updated")]
    ItemUpdated(ItemUpdatedEvent),

    #[serde(rename = "item.completed")]
    ItemCompleted(ItemCompletedEvent),

    #[serde(rename = "error")]
    Error(ThreadErrorEvent),
}
```

### 2.2 Complete Event Type Table

| Event Type | Description | When Emitted |
|------------|-------------|--------------|
| `thread.started` | First event in every session. Contains `thread_id` (UUID). | Session begins |
| `turn.started` | A new turn begins (prompt sent to model). | Each model invocation |
| `turn.completed` | Turn finished successfully. Contains `usage` (tokens). | Turn succeeds |
| `turn.failed` | Turn ended with an error. Contains `error.message`. | Turn fails |
| `item.started` | A new item begins processing. Contains `item` with typed payload. | Tool call begins, plan created |
| `item.updated` | An existing item is updated (e.g., plan step changes). | Plan steps change state |
| `item.completed` | An item has reached terminal state. Contains `item` with final payload. | Tool completes, message finishes |
| `error` | Unrecoverable stream-level error. Contains `message`. | Fatal errors |

### 2.3 Item Type Taxonomy

Each `item.started`, `item.updated`, and `item.completed` event contains an
`item` object with `id`, `type`, and type-specific fields. The `type` field is
one of:

| Item Type | Description | Key Fields |
|-----------|-------------|------------|
| `agent_message` | Agent's natural language response or structured output. | `text` |
| `reasoning` | Agent's reasoning summary. | `text` |
| `command_execution` | A command executed by the agent. | `command`, `aggregated_output`, `exit_code`, `status` |
| `file_change` | A set of file changes (patch applied). | `changes[]` (each with `path`, `kind`), `status` |
| `mcp_tool_call` | A call to an MCP tool. | `server`, `tool`, `arguments`, `result`, `error`, `status` |
| `collab_tool_call` | A call to a collaboration/multi-agent tool. | `tool`, `sender_thread_id`, `receiver_thread_ids`, `prompt`, `agents_states`, `status` |
| `web_search` | A web search request. | `id`, `query`, `action` |
| `todo_list` | Agent's running to-do/plan list. | `items[]` (each with `text`, `completed`) |
| `error` | A non-fatal error surfaced as an item. | `message` |

### 2.4 Status Enums

#### CommandExecutionStatus

| Value | Meaning |
|-------|---------|
| `in_progress` | Command is running |
| `completed` | Command finished successfully |
| `failed` | Command failed (non-zero exit or error) |
| `declined` | Command was declined by user/policy |

#### PatchApplyStatus

| Value | Meaning |
|-------|---------|
| `in_progress` | Patch is being applied |
| `completed` | Patch applied successfully |
| `failed` | Patch application failed or was declined |

#### PatchChangeKind

| Value | Meaning |
|-------|---------|
| `add` | New file created |
| `delete` | File deleted |
| `update` | File modified in place |

#### McpToolCallStatus

| Value | Meaning |
|-------|---------|
| `in_progress` | MCP tool call dispatched, waiting for result |
| `completed` | MCP server returned success |
| `failed` | MCP server returned error |

#### CollabToolCallStatus

| Value | Meaning |
|-------|---------|
| `in_progress` | Collaboration tool call dispatched |
| `completed` | Call completed |
| `failed` | Call failed |

#### CollabTool (multi-agent tool types)

| Value | Meaning |
|-------|---------|
| `spawn_agent` | Creating a new sub-agent |
| `send_input` | Sending input to an existing agent |
| `wait` | Waiting for agent(s) to complete |
| `close_agent` | Closing an agent |

#### CollabAgentStatus

| Value | Meaning |
|-------|---------|
| `pending_init` | Agent created but not yet running |
| `running` | Agent is actively processing |
| `interrupted` | Agent was interrupted |
| `completed` | Agent finished successfully |
| `errored` | Agent encountered an error |
| `shutdown` | Agent was shut down |
| `not_found` | Agent ID not found |

#### WebSearchAction

| Variant | Fields | Meaning |
|---------|--------|---------|
| `Search` | `query`, `queries` | Standard web search |
| `OpenPage` | `url` | Opening a specific URL |
| `FindInPage` | `url`, `pattern` | Searching within a page |
| `Other` | (none) | Unknown/unclassified action |

---

## 3. Example JSONL Snippets

### 3.1 Session Lifecycle

```jsonl
{"type":"thread.started","thread_id":"0199a213-81c0-7800-8aa1-bbab2a035a53"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"Hello! How can I help?"}}
{"type":"turn.completed","usage":{"input_tokens":24763,"cached_input_tokens":24448,"output_tokens":122}}
```

### 3.2 Command Execution

```jsonl
{"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"cargo test -p mylib","aggregated_output":"","exit_code":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"cargo test -p mylib","aggregated_output":"running 12 tests\ntest result: ok. 12 passed","exit_code":0,"status":"completed"}}
```

### 3.3 File Change (Patch Apply)

```jsonl
{"type":"item.completed","item":{"id":"item_2","type":"file_change","changes":[{"path":"src/main.rs","kind":"update"},{"path":"src/config.rs","kind":"add"}],"status":"completed"}}
```

### 3.4 MCP Tool Call

```jsonl
{"type":"item.started","item":{"id":"item_3","type":"mcp_tool_call","server":"my-mcp-server","tool":"search_docs","arguments":{"query":"authentication flow"},"result":null,"error":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_3","type":"mcp_tool_call","server":"my-mcp-server","tool":"search_docs","arguments":{"query":"authentication flow"},"result":{"content":[{"type":"text","text":"Found 3 matching documents..."}],"structured_content":null},"error":null,"status":"completed"}}
```

### 3.5 MCP Tool Call (Failed)

```jsonl
{"type":"item.started","item":{"id":"item_4","type":"mcp_tool_call","server":"external-api","tool":"create_issue","arguments":{"title":"Bug fix"},"result":null,"error":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_4","type":"mcp_tool_call","server":"external-api","tool":"create_issue","arguments":{"title":"Bug fix"},"result":null,"error":{"message":"user rejected MCP tool call"},"status":"failed"}}
```

### 3.6 Web Search

```jsonl
{"type":"item.completed","item":{"id":"item_5","type":"web_search","id":"ws_abc123","query":"rust async patterns 2025","action":{"Search":{"query":"rust async patterns 2025"}}}}
```

### 3.7 Collaboration / Multi-Agent

```jsonl
{"type":"item.started","item":{"id":"item_6","type":"collab_tool_call","tool":"spawn_agent","sender_thread_id":"thread-main","receiver_thread_ids":["thread-worker-1"],"prompt":"Search for all uses of FooBar","agents_states":{"thread-worker-1":{"status":"pending_init","message":null}},"status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_6","type":"collab_tool_call","tool":"spawn_agent","sender_thread_id":"thread-main","receiver_thread_ids":["thread-worker-1"],"prompt":"Search for all uses of FooBar","agents_states":{"thread-worker-1":{"status":"completed","message":"Found FooBar in 3 files"}},"status":"completed"}}
```

### 3.8 Todo / Plan Updates

```jsonl
{"type":"item.started","item":{"id":"item_7","type":"todo_list","items":[{"text":"Read the codebase","completed":false},{"text":"Implement the feature","completed":false},{"text":"Write tests","completed":false}]}}
{"type":"item.updated","item":{"id":"item_7","type":"todo_list","items":[{"text":"Read the codebase","completed":true},{"text":"Implement the feature","completed":false},{"text":"Write tests","completed":false}]}}
{"type":"item.completed","item":{"id":"item_7","type":"todo_list","items":[{"text":"Read the codebase","completed":true},{"text":"Implement the feature","completed":true},{"text":"Write tests","completed":true}]}}
```

### 3.9 Reasoning

```jsonl
{"type":"item.completed","item":{"id":"item_8","type":"reasoning","text":"The user wants to refactor the error handling. I should first understand the current error types..."}}
```

### 3.10 Error Events

```jsonl
{"type":"error","message":"rate limit exceeded, please try again later"}
```

```jsonl
{"type":"turn.failed","error":{"message":"turn failed (model returned invalid response)"}}
```

Non-fatal errors appear as items:

```jsonl
{"type":"item.completed","item":{"id":"item_9","type":"error","message":"model rerouted: gpt-5.4 -> gpt-5.3-codex (capacity)"}}
```

### 3.11 Usage Data

The `turn.completed` event always includes token usage:

```jsonl
{"type":"turn.completed","usage":{"input_tokens":36398,"cached_input_tokens":32000,"output_tokens":1450}}
```

---

## 4. Internal Event Model (Core Protocol)

The exec-level JSONL events (`ThreadEvent`) are derived from a richer internal
event model (`EventMsg` in `codex-rs/protocol/src/protocol.rs`). The internal
model has many more event types that are used by the TUI and app server but are
collapsed or filtered for the exec JSONL output.

### 4.1 Internal Events Mapped to JSONL

| Internal Event | JSONL Mapping |
|----------------|---------------|
| `SessionConfigured` | `thread.started` (thread_id = session_id) |
| `TurnStarted` | `turn.started` |
| `TurnCompleted` (success) | `turn.completed` + reconcile unfinished items |
| `TurnCompleted` (failed) | `turn.failed` |
| `ItemStarted` | `item.started` (for tool items only; messages/reasoning suppressed) |
| `ItemCompleted` | `item.completed` |
| `TurnPlanUpdated` | `item.started` (first), `item.updated` (subsequent), `item.completed` (at turn end) |
| `ThreadTokenUsageUpdated` | Accumulated; emitted with `turn.completed` |
| `Error` | `error` |
| `ConfigWarning` | `item.completed` (as error item) |
| `DeprecationNotice` | `item.completed` (as error item) |
| `ModelRerouted` | `item.completed` (as error item with reroute message) |

### 4.2 Internal Events NOT in JSONL

These internal events are silently dropped by the JSONL output processor:

| Internal Event | Why Not Exposed |
|----------------|-----------------|
| `AgentMessageDelta` | High-volume streaming delta |
| `AgentReasoningDelta` | High-volume streaming delta |
| `AgentReasoningRawContent*` | Internal reasoning content |
| `ExecCommandBegin` / `OutputDelta` / `End` | Replaced by item.started/completed |
| `PatchApplyBegin` / `End` | Replaced by item.completed (file_change) |
| `McpToolCallBegin` / `End` | Replaced by item.started/completed (mcp_tool_call) |
| `WebSearchBegin` / `End` | Replaced by item.completed (web_search) |
| `ViewImageToolCall` | Not surfaced |
| `ExecApprovalRequest` | Interactive-only |
| `RequestPermissions` | Interactive-only |
| `RequestUserInput` | Interactive-only |
| `ApplyPatchApprovalRequest` | Interactive-only |
| `GuardianAssessment` | Internal safety monitoring |
| `HookStarted` / `HookCompleted` | Not surfaced in JSONL |
| `TurnDiff` / `TurnDiffUpdated` | Not surfaced |
| `StreamError` | Not surfaced (internal retry) |
| `UndoStarted` / `UndoCompleted` | Not surfaced |
| `BackgroundEvent` | Not surfaced |
| `ContextCompacted` | Not surfaced |
| `ThreadRolledBack` | Not surfaced |
| `RawResponseItem` | Not surfaced |
| Various delta events | Not surfaced |

---

## 5. MCP Tools in the JSONL Stream

### 5.1 Naming Convention

MCP tools appear in the stream with the server name and tool name as separate
fields in the `mcp_tool_call` item type:

```json
{
  "type": "mcp_tool_call",
  "server": "my-mcp-server",
  "tool": "search_docs",
  "arguments": { "query": "..." },
  "status": "in_progress"
}
```

This is different from Claude Code, which uses a `mcp__<server>__<tool>` naming
convention in `tool_use` events. Codex CLI keeps server and tool as separate
structured fields.

### 5.2 MCP Tool Result Structure

On success:

```json
{
  "result": {
    "content": [
      { "type": "text", "text": "result text" }
    ],
    "structured_content": null
  },
  "error": null,
  "status": "completed"
}
```

On failure:

```json
{
  "result": null,
  "error": { "message": "error description" },
  "status": "failed"
}
```

### 5.3 MCP Tool Approval

MCP tool calls may require user approval based on tool annotations:

- `destructive_hint: true` -- always requires approval
- `read_only_hint: true` -- no approval needed
- Neither hint set -- defaults to requiring approval

Approval decisions: Accept, AcceptForSession (remembered), AcceptAndRemember
(persisted to config), Decline, Cancel.

In non-interactive mode with `--dangerously-bypass-approvals-and-sandbox`, all
approvals are skipped.

### 5.4 MCP Tool Telemetry

Each MCP tool call emits OpenTelemetry spans with:

- `mcp.server.name`
- `mcp.server.origin` (stdio or streamable_http)
- `mcp.transport`
- `mcp.connector.id` / `mcp.connector.name` (for Codex Apps)
- `tool.name`
- `tool.call_id`

---

## 6. Serialization Details

### 6.1 Output Method

The JSONL processor (`EventProcessorWithJsonOutput`) serializes each
`ThreadEvent` to JSON with `serde_json::to_string()` and writes it to stdout
via `println!()`. One JSON object per line, no trailing comma, no array wrapper.

If serialization fails, a fallback error event is emitted:

```json
{"type":"error","message":"failed to serialize exec json event: <serde error>"}
```

### 6.2 Item ID Generation

Item IDs are monotonically incrementing: `item_0`, `item_1`, `item_2`, etc.
They are local to the exec session and do not correspond to any internal IDs
from the core protocol.

### 6.3 Final Message

The last `agent_message` text from the final turn is captured and can be:

- Written to a file via `--output-last-message <path>` / `-o <path>`
- Retrieved programmatically after the session completes

On `turn.failed`, the final message is cleared and the output file is not
written (preserving any previous content).

### 6.4 Tag-Based Discrimination

All events use `serde(tag = "type")` for top-level discrimination. Item
details use `serde(tag = "type", rename_all = "snake_case")` for the item type
field. This means each JSONL line has a top-level `"type"` (event type) and
item events have a nested `"type"` (item type) within the `"item"` object.

### 6.5 TypeScript Type Generation

The Rust types derive `ts_rs::TS`, meaning TypeScript type definitions can be
generated from the Rust source. The generated types would be available in a
TypeScript SDK package (similar to how the Codex SDK works).

---

## 7. CLI Invocation Reference

### 7.1 Key Flags for Non-Interactive Mode

| Flag | Description |
|------|-------------|
| `--json` | Print events to stdout as JSONL |
| `-o <FILE>` / `--output-last-message <FILE>` | Write final agent message to file |
| `--output-schema <FILE>` | JSON Schema for structured response validation |
| `--ephemeral` | Don't persist session files |
| `--full-auto` | Broader permissions (workspace-write sandbox) |
| `--sandbox <MODE>` | Sandbox policy: `workspace-write`, `danger-full-access`, etc. |
| `-m <MODEL>` | Model to use |
| `-C <DIR>` / `--cd <DIR>` | Working directory |
| `--skip-git-repo-check` | Allow running outside a git repo |
| `--add-dir <DIR>` | Additional writable directories |
| `--color <MODE>` | Color output: `auto`, `always`, `never` |

### 7.2 Subcommands

| Subcommand | Description |
|------------|-------------|
| (none) | Run a new non-interactive session with a prompt |
| `resume` | Resume a previous session by ID, name, or `--last` |
| `review` | Run a code review (`--uncommitted`, `--base <BRANCH>`, `--commit <SHA>`) |

### 7.3 Recommended Invocation for Claudine

```bash
codex exec --json --ephemeral -o /tmp/last-message.txt "your prompt here"
```

This gives:
- JSONL events on stdout for metadata parsing
- Final message captured to a temp file for user-facing output
- No persistent session files cluttering disk
