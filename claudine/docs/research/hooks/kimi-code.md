---
homepage: https://www.kimi.com/code
docs: https://moonshotai.github.io/kimi-cli/en/
hooks: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
---

# Kimi Code CLI hooks and events

## Scope

This document covers the hook and event system available in Kimi Code CLI (Moonshot AI's agentic coding CLI). Kimi Code CLI does **not** have a file-based hook system like Claude Code or Gemini CLI. Instead, it exposes its event surface through **Wire mode** (`kimi --wire`), a JSON-RPC 2.0 bidirectional protocol over stdin/stdout. External programs can observe events (notifications), respond to blocking requests, and control the agent turn lifecycle. A companion programmatic SDK (`kimi-agent-sdk`) wraps Wire mode for Go, Node.js, and Python.

## Home Page

https://www.kimi.com/code

## Documentation

https://moonshotai.github.io/kimi-cli/en/

## Configuration

Kimi Code CLI does not use a settings/hooks configuration file for event hooks. All event interaction happens over the Wire protocol at runtime.

### Main config file

| Location | Format | Purpose |
|----------|--------|---------|
| `~/.kimi/config.toml` | TOML (or JSON legacy) | Providers, models, services, loop control, MCP settings |
| `~/.kimi/mcp.json` | JSON | MCP server definitions |
| `~/.kimi/kimi.json` | JSON | Runtime metadata (working dirs, thinking mode) |

The configuration file manages providers, models, and runtime parameters but has no `hooks` key or equivalent. If `~/.kimi/config.toml` is absent but `~/.kimi/config.json` exists, the CLI auto-migrates to TOML and backs up the original.

Override with `--config-file /path/to/config.toml` or inline with `--config '{...}'`.

### Data locations

| Path | Purpose |
|------|---------|
| `~/.kimi/` | Root data directory (override with `KIMI_SHARE_DIR`) |
| `~/.kimi/sessions/<hash>/<id>/context.jsonl` | Message history |
| `~/.kimi/sessions/<hash>/<id>/wire.jsonl` | Wire event log (for `replay`) |
| `~/.kimi/logs/kimi.log` | Runtime log |
| `~/.kimi/credentials/<provider>.json` | OAuth credentials (mode 600) |
| `~/.kimi/user-history/<hash>.jsonl` | Input history per working directory |

### No user-scoped or repo-scoped hook config

Unlike Claude Code (which merges `~/.claude/settings.json` + `.claude/settings.json` + `.claude/settings.local.json`), Kimi Code has no equivalent hook config at either scope. The Wire protocol is the sole mechanism for intercepting events programmatically.

### Example: launching Wire mode

```bash
# Start Kimi in Wire mode (JSON-RPC 2.0 over stdin/stdout)
kimi --wire

# Or with a specific model
kimi --wire --model kimi-k2.5

# Or in auto-approve mode
kimi --wire --yolo
```

### Example: config.toml (no hooks section)

```toml
default_model = "kimi-k2.5"
default_thinking = false
default_yolo = false

[providers.kimi]
type = "kimi"
base_url = "https://api.moonshot.cn/v1"
api_key = "sk-..."

[models.kimi-k2.5]
provider = "kimi"
model = "kimi-k2.5"
max_context_size = 262144
capabilities = ["thinking", "image_input"]

[loop_control]
max_steps_per_turn = 100
max_retries_per_step = 3
reserved_context_size = 50000

[mcp.client]
tool_call_timeout_ms = 60000
```

## Wire Protocol Overview

Wire mode uses JSON-RPC 2.0 with line-delimited messages. Protocol version: **1.3**. Each message is a single JSON line on stdin or stdout.

### Message categories

| Category | Direction | Has `id` | Response required |
|----------|-----------|----------|-------------------|
| Client requests | client -> agent | Yes | Yes (from agent) |
| Agent events | agent -> client | No | No (notification) |
| Agent requests | agent -> client | Yes | Yes (from client) |
| Responses | either | Yes (matches request) | N/A |

### Standard JSON-RPC error codes

| Code | Meaning |
|------|---------|
| -32700 | Invalid JSON (parse error) |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid parameters |
| -32603 | Internal error |
| -32000 | Turn already in progress |
| -32001 | LLM not configured |
| -32002 | Specified LLM unsupported |
| -32003 | LLM service error |

## Hook Events

Kimi Code exposes two categories of Wire messages that serve as hooks:

1. **Event notifications** (agent -> client): fire-and-forget, no response required.
2. **Agent requests** (agent -> client): blocking, the agent pauses until the client responds.

All event notifications are delivered via JSON-RPC method `event`:

```json
{ "jsonrpc": "2.0", "method": "event", "params": { "type": "<EventType>", "payload": { ... } } }
```

All agent requests are delivered via JSON-RPC method `request`:

```json
{ "jsonrpc": "2.0", "method": "request", "id": "<unique-id>", "params": { "type": "<RequestType>", "payload": { ... } } }
```

### Client-to-Agent Methods

These are methods the client sends to control the agent.

#### `initialize`

Optional handshake for protocol negotiation and external tool registration.

**Direction:** client -> agent

**Parameters:**

```json
{
  "protocol_version": "1.3",
  "client": { "name": "my-client", "version": "1.0.0" },
  "external_tools": [
    {
      "name": "my_tool",
      "description": "Does something",
      "parameters": { "type": "object", "properties": {} }
    }
  ]
}
```

**Response (`InitializeResult`):**

```json
{
  "protocol_version": "1.3",
  "server": { "name": "kimi-cli", "version": "1.12.0" },
  "slash_commands": [
    { "name": "help", "description": "Show help", "aliases": ["h", "?"] }
  ],
  "external_tools": {
    "accepted": ["my_tool"],
    "rejected": []
  }
}
```

**Flow impact:** Enables protocol negotiation and registers external tools the agent can invoke via `ToolCallRequest`. If the server does not support `initialize`, it returns `-32601 method not found`; the client should fall back to sending `prompt` directly.

**Gotchas:**
- `initialize` is optional. Older or lightweight servers (e.g., `kimi-agent-rs`) may not support it. Always handle `-32601` gracefully.
- `external_tools` in the response shows which tools were accepted vs rejected. Check the `rejected` array to detect registration failures.

---

#### `prompt`

Starts an agent turn.

**Direction:** client -> agent

**Parameters:**

```json
{
  "user_input": "Fix the bug in main.rs"
}
```

`user_input` can be a plain string or an array of `ContentPart` objects (text, image_url, audio_url, video_url).

**Response (`PromptResult`):**

```json
{
  "status": "finished",
  "steps": 5
}
```

| Status | Meaning |
|--------|---------|
| `"finished"` | Turn completed normally |
| `"cancelled"` | Turn was cancelled via `cancel` |
| `"max_steps_reached"` | Hit the step limit; `steps` field is present |

**Flow impact:** While the turn runs, the agent streams event notifications and may send blocking requests. The `prompt` response only arrives after the turn ends. Real-time UI or state must be built on events, not the prompt response.

**Error codes:**
- `-32000`: Turn already in progress
- `-32001`: LLM not configured
- `-32002`: Specified LLM unsupported
- `-32003`: LLM service error

**Gotchas:**
- Only one turn can run at a time. Sending a second `prompt` while one is active returns `-32000`.
- The response arrives only at turn completion. Do not use it for real-time progress.

---

#### `replay`

Replays a recorded session from the `wire.jsonl` log file. Added in Wire protocol 1.3.

**Direction:** client -> agent

**Parameters:** Empty object `{}` or omitted.

**Response:**

```json
{
  "status": "finished",
  "events": 42,
  "requests": 3
}
```

**Flow impact:** Streams historical events and requests from the session log. Useful for replaying a session in a new client without re-executing.

---

#### `cancel`

Cancels the in-progress `prompt` or `replay`.

**Direction:** client -> agent

**Parameters:** Empty object `{}` or omitted.

**Response:** `{}`

**Flow impact:** The running `prompt` response resolves with `status: "cancelled"`. Returns `-32000` if no turn is in progress.

---

### Event Notifications (agent -> client)

These are fire-and-forget notifications. No response is required or expected.

#### `TurnBegin`

Signals the start of a new agent turn.

**Event payload:**

```json
{
  "type": "TurnBegin",
  "payload": {
    "user_input": "Fix the bug in main.rs"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `user_input` | `string \| ContentPart[]` | The user input that started this turn |

**Event response:** None (notification).

**Gotchas:**
- `user_input` can be a string or an array of content parts. Parse defensively.

---

#### `TurnEnd`

Signals the clean end of a turn. Added in Wire protocol 1.2.

**Event payload:**

```json
{
  "type": "TurnEnd",
  "payload": {}
}
```

No additional fields.

**Event response:** None (notification).

**Gotchas:**
- May be omitted if the turn is interrupted or cancelled. Always treat the `prompt` response status as the authoritative end-of-turn signal, not `TurnEnd`.

---

#### `StepBegin`

Marks the beginning of a step within the current turn.

**Event payload:**

```json
{
  "type": "StepBegin",
  "payload": {
    "n": 1
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `n` | `number` | Step index, starting at 1 |

**Event response:** None (notification).

**Gotchas:**
- Step numbering starts at 1, not 0.

---

#### `StepInterrupted`

Indicates the current step was interrupted.

**Event payload:**

```json
{
  "type": "StepInterrupted",
  "payload": {}
}
```

No additional fields.

**Event response:** None (notification).

---

#### `CompactionBegin`

Signals the start of context compaction (history compression).

**Event payload:**

```json
{
  "type": "CompactionBegin",
  "payload": {}
}
```

No additional fields.

**Event response:** None (notification).

---

#### `CompactionEnd`

Signals the end of context compaction.

**Event payload:**

```json
{
  "type": "CompactionEnd",
  "payload": {}
}
```

No additional fields.

**Event response:** None (notification).

**Gotchas:**
- Compaction can happen mid-turn when the context window fills. UI clients should show a progress indicator between `CompactionBegin` and `CompactionEnd`.

---

#### `StatusUpdate`

Reports telemetry about context usage and token consumption.

**Event payload:**

```json
{
  "type": "StatusUpdate",
  "payload": {
    "context_usage": 0.73,
    "token_usage": {
      "input_other": 1200,
      "output": 350,
      "input_cache_read": 800,
      "input_cache_creation": 100
    },
    "message_id": "msg_abc123"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `context_usage` | `number?` | 0-1 float representing context window utilization |
| `token_usage` | `TokenUsage?` | Token consumption breakdown |
| `token_usage.input_other` | `number` | Non-cached input tokens |
| `token_usage.output` | `number` | Output tokens |
| `token_usage.input_cache_read` | `number` | Tokens read from cache |
| `token_usage.input_cache_creation` | `number` | Tokens written to cache |
| `message_id` | `string?` | Provider message identifier |

**Event response:** None (notification).

**Gotchas:**
- All fields may be `null` or absent. Defensive parsing is required.
- `context_usage` approaching 1.0 typically precedes a `CompactionBegin` event.

---

#### `ContentPart`

Streams a fragment of the agent's response. This is the primary hook for rendering real-time output.

**Event payload (text):**

```json
{
  "type": "ContentPart",
  "payload": {
    "type": "text",
    "text": "Here is the fix..."
  }
}
```

**Event payload (thinking):**

```json
{
  "type": "ContentPart",
  "payload": {
    "type": "think",
    "think": "Let me analyze the code...",
    "encrypted": null
  }
}
```

**Event payload (media):**

```json
{
  "type": "ContentPart",
  "payload": {
    "type": "image_url",
    "image_url": { "url": "data:image/png;base64,...", "id": "img_001" }
  }
}
```

| Variant | Fields | Description |
|---------|--------|-------------|
| `text` | `text: string` | Plain text fragment |
| `think` | `think: string`, `encrypted?: string` | Thinking/reasoning content |
| `image_url` | `image_url: { url, id? }` | Image content |
| `audio_url` | `audio_url: { url, id? }` | Audio content |
| `video_url` | `video_url: { url, id? }` | Video content |

**Event response:** None (notification).

**Gotchas:**
- Text arrives as streaming fragments. Clients must buffer and concatenate parts to reconstruct the full response.
- `think` parts may contain the agent's chain-of-thought reasoning. Clients can surface or hide these based on user preference.
- The `encrypted` field on `think` parts is used by the provider for encrypted reasoning; clients can typically ignore it.

---

#### `ToolCall`

Signals that a tool call is being formed.

**Event payload:**

```json
{
  "type": "ToolCall",
  "payload": {
    "type": "function",
    "id": "tc_abc123",
    "function": {
      "name": "Shell",
      "arguments": "{\"command\": \"cargo test\"}"
    },
    "extras": {}
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | `"function"` | Always `"function"` |
| `id` | `string` | Unique tool call identifier |
| `function.name` | `string` | Tool name |
| `function.arguments` | `string?` | JSON-encoded arguments (may be absent during streaming) |
| `extras` | `object?` | Provider-specific metadata |

**Event response:** None (notification).

**Gotchas:**
- `arguments` may be absent or incomplete when `ToolCall` is first emitted. The full arguments arrive via subsequent `ToolCallPart` events. Buffer and concatenate before parsing.

---

#### `ToolCallPart`

Streams a fragment of tool call arguments.

**Event payload:**

```json
{
  "type": "ToolCallPart",
  "payload": {
    "arguments_part": "{\"comma"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `arguments_part` | `string?` | Fragment of JSON-encoded arguments |

**Event response:** None (notification).

**Gotchas:**
- Fragments are not valid JSON on their own. Concatenate all `ToolCallPart` payloads to reconstruct the full `arguments` JSON string, then parse.

---

#### `ToolResult`

Reports the result of a tool execution.

**Event payload:**

```json
{
  "type": "ToolResult",
  "payload": {
    "tool_call_id": "tc_abc123",
    "return_value": {
      "is_error": false,
      "output": "All 42 tests passed.",
      "message": "Tests passed successfully",
      "display": [
        {
          "type": "shell",
          "language": "sh",
          "command": "cargo test"
        }
      ],
      "extras": {}
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `tool_call_id` | `string` | Matches the `id` from `ToolCall` |
| `return_value.is_error` | `boolean` | Whether the tool execution failed |
| `return_value.output` | `string \| ContentPart[]` | Tool output content |
| `return_value.message` | `string` | Human-readable summary |
| `return_value.display` | `DisplayBlock[]` | Rich display blocks for UI rendering |
| `return_value.extras` | `object?` | Provider-specific metadata |

**Event response:** None (notification).

---

#### `ApprovalResponse`

Notifies that an approval request was resolved.

**Event payload:**

```json
{
  "type": "ApprovalResponse",
  "payload": {
    "request_id": "req_abc123",
    "response": "approve"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `request_id` | `string` | Matches the `id` from the `ApprovalRequest` |
| `response` | `"approve" \| "approve_for_session" \| "reject"` | Resolution |

**Event response:** None (notification).

**Gotchas:**
- This event was renamed from `ApprovalRequestResolved` in Wire protocol 1.1. The old name is still accepted for backward compatibility. Support both names when parsing.

---

#### `SubagentEvent`

Forwards a nested event from a subagent (spawned via the Task tool).

**Event payload:**

```json
{
  "type": "SubagentEvent",
  "payload": {
    "task_tool_call_id": "tc_task_001",
    "event": {
      "type": "ContentPart",
      "payload": {
        "type": "text",
        "text": "Subagent output..."
      }
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `task_tool_call_id` | `string` | The tool call ID of the Task that spawned this subagent |
| `event` | `WireMessage` | A nested Wire event (any event type) |

**Event response:** None (notification).

**Gotchas:**
- Subagent events are recursive: a `SubagentEvent` can itself contain another `SubagentEvent` if subagents spawn their own subagents.
- The `task_tool_call_id` links back to the parent Task tool call, enabling tree-structured progress tracking.

---

### Blocking Requests (agent -> client)

These are requests the agent sends that **must** be answered. The agent pauses until the client returns a JSON-RPC success response with the expected payload. Failure to respond causes the agent to stall indefinitely.

#### `ApprovalRequest`

The agent requests permission to execute a tool action.

**Request payload:**

```json
{
  "jsonrpc": "2.0",
  "method": "request",
  "id": "req_abc123",
  "params": {
    "type": "ApprovalRequest",
    "payload": {
      "id": "req_abc123",
      "tool_call_id": "tc_abc123",
      "sender": "Shell",
      "action": "execute_command",
      "description": "Run: cargo test",
      "display": [
        {
          "type": "shell",
          "language": "sh",
          "command": "cargo test"
        }
      ]
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Request identifier (use as `request_id` in response) |
| `tool_call_id` | `string` | The tool call being approved |
| `sender` | `string` | Tool name that triggered the approval |
| `action` | `string` | Action description |
| `description` | `string` | Human-readable explanation |
| `display` | `DisplayBlock[]?` | Optional rich preview (diffs, commands, etc.) |

**Expected response (`ApprovalResponse`):**

```json
{
  "jsonrpc": "2.0",
  "id": "req_abc123",
  "result": {
    "request_id": "req_abc123",
    "response": "approve"
  }
}
```

| Response value | Effect |
|----------------|--------|
| `"approve"` | Proceed with this tool call |
| `"approve_for_session"` | Proceed and auto-approve similar operations for the rest of the session |
| `"reject"` | Cancel the tool call; the agent adjusts its plan |

**Flow impact:** The agent halts until a response is received. This is the primary mechanism for human-in-the-loop control over dangerous operations (file writes, shell commands).

**Gotchas:**
- If the client does not respond, the agent stalls forever. Implement explicit timeouts and surface a clear UI prompt.
- `approve_for_session` is a convenience for interactive use but can be dangerous in automated pipelines. Use `approve` for per-action control.
- In `--yolo` mode, all approvals are auto-granted and `ApprovalRequest` is never sent.

---

#### `ToolCallRequest`

The agent invokes an external tool registered during `initialize`.

**Request payload:**

```json
{
  "jsonrpc": "2.0",
  "method": "request",
  "id": "req_tool_001",
  "params": {
    "type": "ToolCallRequest",
    "payload": {
      "id": "tc_ext_001",
      "name": "my_custom_tool",
      "arguments": "{\"query\": \"rust async patterns\"}"
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Tool call identifier |
| `name` | `string` | Name of the registered external tool |
| `arguments` | `string?` | JSON-encoded arguments |

**Expected response (`ToolResult`):**

```json
{
  "jsonrpc": "2.0",
  "id": "req_tool_001",
  "result": {
    "tool_call_id": "tc_ext_001",
    "return_value": {
      "is_error": false,
      "output": "Results here...",
      "message": "Search completed",
      "display": [],
      "extras": {}
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `tool_call_id` | `string` | Must match the request `id` |
| `return_value.is_error` | `boolean` | Whether the tool execution failed |
| `return_value.output` | `string \| ContentPart[]` | Tool output |
| `return_value.message` | `string` | Human-readable summary |
| `return_value.display` | `DisplayBlock[]` | Rich display blocks |
| `return_value.extras` | `object?` | Optional metadata |

**Flow impact:** The agent halts until the tool result is provided. Returning `is_error: true` informs the model the tool failed, which changes its planning and next steps.

**Gotchas:**
- `arguments` is a JSON string, not a parsed object. Parse it on the client side.
- The agent expects a response for every `ToolCallRequest`. Failing to respond causes a permanent stall.
- External tools are only invoked if they were registered and accepted during `initialize`.

---

### Display Block Types

Both `ToolResult` and `ApprovalRequest` can carry `display` blocks for rich UI rendering.

| Type | Fields | Description |
|------|--------|-------------|
| `brief` | `text: string` | Simple text content |
| `diff` | `path: string`, `old_text: string`, `new_text: string` | File modification preview |
| `todo` | `items: [{ title, status }]` | Task list (statuses: `pending`, `in_progress`, `done`) |
| `shell` | `language: string`, `command: string` | Command preview |
| (unknown) | `type: string`, `data: object` | Fallback for unrecognized block types |

## Matcher System

Kimi Code CLI does **not** have a matcher system. Unlike Claude Code (which uses regex matchers to filter hooks by tool name, session source, or notification type), Kimi Code's Wire protocol delivers all events unconditionally. Filtering must be implemented on the client side.

To implement matcher-like behavior, clients should:

1. Parse the `type` field of each incoming `event` or `request` message.
2. For tool-specific filtering, inspect `payload.function.name` on `ToolCall` events or `payload.sender` on `ApprovalRequest`.
3. For subagent filtering, check `payload.task_tool_call_id` on `SubagentEvent`.

Example client-side filtering (pseudocode):

```python
if msg["method"] == "event":
    event_type = msg["params"]["type"]
    if event_type == "ToolCall":
        tool_name = msg["params"]["payload"]["function"]["name"]
        if tool_name == "Shell":
            # Handle shell tool calls specifically
            pass
    elif event_type == "ApprovalRequest":
        # Handle all approval requests
        pass
```

## Kimi Agent SDK

The [Kimi Agent SDK](https://github.com/MoonshotAI/kimi-agent-sdk) provides language-native wrappers around Wire mode for Go, Node.js, and Python. The SDKs handle JSON-RPC serialization, event streaming, and approval/tool-call response plumbing, making it easier to build Wire clients without raw protocol handling.

| Language | Package |
|----------|---------|
| Go | `go get github.com/MoonshotAI/kimi-agent-sdk/go` |
| Node.js | `npm install @moonshot-ai/kimi-agent-sdk` |
| Python | `pip install kimi-agent-sdk` |

## Kimi Agent (Rust)

[`kimi-agent-rs`](https://github.com/MoonshotAI/kimi-agent-rs) is a lightweight Rust implementation that speaks the same Wire protocol but only supports Kimi providers. It omits Shell/Print/ACP UIs, non-Kimi providers, account login, and SSH support. Uses separate MCP credential storage at `~/.kimi/credentials/mcp_auth.json`.

## Known Gotchas and Workarounds

### 1. `initialize` is optional

**Problem:** Some servers (including `kimi-agent-rs`) return `-32601` for `initialize`.

**Solution:** Optimistically call `initialize`, then fall back to direct `prompt` on `method not found`. Do not treat this as a fatal error.

### 2. `TurnEnd` may be missing

**Problem:** The docs note `TurnEnd` can be omitted if a turn is interrupted or cancelled.

**Solution:** Always treat the `prompt` response status (`finished`, `cancelled`, `max_steps_reached`) as the authoritative end-of-turn signal. Use `TurnEnd` only as a supplementary hint.

### 3. Optional/null fields are pervasive

**Problem:** `StatusUpdate` and other payloads may omit fields or send `null`. Strict schema validation will fail.

**Solution:** Use tolerant/defensive parsing. Default missing numeric fields to 0, missing strings to empty, missing objects to `null`.

### 4. Tool arguments stream incrementally

**Problem:** `ToolCall` arguments can be absent or incomplete while `ToolCallPart` streams fragments. Attempting to parse `arguments` on the initial `ToolCall` event may fail.

**Solution:** Buffer all `ToolCallPart` fragments for a given tool call, concatenate them, then parse the complete JSON string.

### 5. Approval event rename (Wire 1.1)

**Problem:** `ApprovalResponse` replaced `ApprovalRequestResolved` in Wire protocol 1.1, but the old name is still accepted by some implementations.

**Solution:** Support both event names when parsing. Key on the `request_id` field for correlation rather than the event type name.

### 6. Blocking requests stall the agent

**Problem:** If the client does not respond to `ApprovalRequest` or `ToolCallRequest`, the agent pauses indefinitely.

**Solution:** Implement explicit timeouts on the client side. Surface a clear UI prompt so the operator can respond quickly. For automated pipelines, consider `--yolo` mode to bypass approvals entirely, or implement auto-approve logic in the client.

### 7. Single-line JSON-RPC is mandatory

**Problem:** Each Wire message must be a single JSON line terminated by a newline. Multi-line JSON or pretty-printed output breaks the protocol.

**Solution:** Always serialize outgoing messages with no internal newlines. When reading, split on newlines and parse each line independently.

### 8. No pre-tool interception (no PreToolUse equivalent)

**Problem:** Unlike Claude Code's `PreToolUse` hook which can block or modify tool calls before execution, Kimi's Wire protocol does not offer a pre-execution interception point for built-in tools. `ToolCall` is a notification (no response expected), so you cannot prevent or modify the call.

**Solution:** Use `ApprovalRequest` for gating. All built-in tools that require approval will send an `ApprovalRequest` before executing. Reject the approval to prevent execution. For tools that do not require approval (in `--yolo` mode or auto-approved tools), there is no interception mechanism.

### 9. No session lifecycle events

**Problem:** Kimi Code CLI does not emit `SessionStart` or `SessionEnd` events over Wire. You cannot detect when a session begins or ends through the protocol.

**Solution:** Infer session start from the process launch (when the Wire connection opens). Infer session end from process exit or connection close.

### 10. Only one turn at a time

**Problem:** Sending a second `prompt` while a turn is active returns error `-32000`.

**Solution:** Wait for the current `prompt` response before sending another. Use `cancel` if you need to abort the current turn first.

## Sources

- Wire mode documentation: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
- Kimi Code CLI docs: https://moonshotai.github.io/kimi-cli/en/
- Kimi Code homepage: https://www.kimi.com/code
- Configuration files: https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html
- Data locations: https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html
- Environment variables: https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.html
- Kimi Agent SDK: https://github.com/MoonshotAI/kimi-agent-sdk
- Kimi Agent (Rust): https://github.com/MoonshotAI/kimi-agent-rs
- Kimi CLI GitHub: https://github.com/MoonshotAI/kimi-cli
- Skills documentation: https://moonshotai.github.io/kimi-cli/en/customization/skills.html
- Agents documentation: https://moonshotai.github.io/kimi-cli/en/customization/agents.html
