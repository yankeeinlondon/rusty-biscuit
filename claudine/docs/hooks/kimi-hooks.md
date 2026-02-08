---
prompt: |-
    Do online research on the hooks/event which can be leveraged when using the Agentic Kimi Code CLI (https://www.kimi.com/code). Describe each hook, what data it returns, what kind of return type is expected and how that return type effects the agentic flow. in the end describe any known gotcha in working with this event/hook model and how people have gotten around these quirks or shortcomings.
---

# Kimi Code CLI Hook/Event Model (Wire Mode)

Kimi Code CLI exposes its most complete event/hook surface through **Wire mode** (`kimi --wire`). Wire uses a JSON-RPC 2.0, line-delimited protocol where the CLI sends **event notifications** (no response required) and **request messages** (client must respond) while a turn is running. The agent only returns a `prompt` response when the turn completes, so the “hooks” you can leverage are the Wire events and requests described below. Source: [Wire mode docs](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html).

## Transport-Level Hooks (turn control)

### `initialize` (optional handshake)

- **Direction**: client → agent
- **Return**: `InitializeResult` (JSON-RPC success response)
- **Return shape**: protocol version, server info, slash command list, and optional external tool registration results.
- **Flow impact**: If supported, enables protocol negotiation and registering external tools. If the server does not support it, it returns `-32601 method not found` and the client should fall back to sending `prompt` directly.

### `prompt` (start a turn)

- **Direction**: client → agent
- **Return**: `PromptResult`
- **Return shape**: `{ status: "finished" | "cancelled" | "max_steps_reached", steps?: number }`
- **Flow impact**: Starts an agent turn. While it runs, the agent streams **events** and **requests**. The `prompt` response only arrives after the turn ends, so real-time UI/state must be built on events.

### `cancel` (stop a turn)

- **Direction**: client → agent
- **Return**: empty object `{}`
- **Flow impact**: Cancels the in-progress `prompt`; the `prompt` response then resolves with `status: "cancelled"`.

## Event Hooks (agent → client notifications)

All events are delivered via JSON-RPC method `event` with:

```
{ "jsonrpc": "2.0", "method": "event", "params": { "type": "...", "payload": { ... } } }
```

Events do **not** require a response. They are the primary “hooks” for UI updates, progress tracking, streaming content, and instrumentation.

### `TurnBegin`

- **Data**: `user_input: string | ContentPart[]`
- **Return type expected**: none (notification)
- **Flow impact**: Signals a new turn has begun; useful for UI resets or logging.

### `TurnEnd` (Wire 1.2+)

- **Data**: none
- **Return type expected**: none
- **Flow impact**: Signals a clean end of a turn, after all other events. May be omitted if the turn is interrupted.

### `StepBegin`

- **Data**: `{ n: number }` step index starting at 1
- **Return type expected**: none
- **Flow impact**: Step-level progress. Useful for progress bars or step-aware tracing.

### `StepInterrupted`

- **Data**: none
- **Return type expected**: none
- **Flow impact**: Indicates the current step was interrupted.

### `CompactionBegin` / `CompactionEnd`

- **Data**: none
- **Return type expected**: none
- **Flow impact**: Signals context compaction start/end; useful for UI hints or logs when the agent compresses history.

### `StatusUpdate`

- **Data**: `{ context_usage?, token_usage?, message_id? }` where fields may be `null` or absent
- **Return type expected**: none
- **Flow impact**: Streaming telemetry for token usage and context window utilization.

### `ContentPart`

- **Data**: a single content part; `payload.type` distinguishes:
    - `text`: `{ text: string }`
    - `think`: `{ think: string, encrypted?: string | null }`
    - `image_url` / `audio_url` / `video_url`: `{ url: string, id?: string | null }`
- **Return type expected**: none
- **Flow impact**: Streamed agent output. Clients typically append text parts to the assistant message, optionally surface `think` or ignore it.

### `ToolCall`

- **Data**: `{ type: "function", id, function: { name, arguments? }, extras? }`
- **Return type expected**: none
- **Flow impact**: Signals that a tool call is being formed/issued. Often accompanied by `ToolCallPart` (streamed arguments) and then a `request` for approval or external tool execution.

### `ToolCallPart`

- **Data**: `{ arguments_part?: string | null }`
- **Return type expected**: none
- **Flow impact**: Streaming fragments of tool arguments; clients may buffer these to reconstruct the full arguments payload.

### `ToolResult`

- **Data**: `{ tool_call_id, return_value: ToolReturnValue }`
- **Return type expected**: none
- **Flow impact**: Emits tool execution results, typically after a `ToolCallRequest` has been fulfilled.

### `ApprovalResponse`

- **Data**: `{ request_id, response: "approve" | "approve_for_session" | "reject" }`
- **Return type expected**: none
- **Flow impact**: Notifies that an approval request was resolved. (Renamed from `ApprovalRequestResolved` in Wire 1.1; old name still accepted.)

### `SubagentEvent`

- **Data**: `{ task_tool_call_id, event: { type, payload } }`
- **Return type expected**: none
- **Flow impact**: Nested event forwarding from subagents, enabling multi-agent progress streaming.

## Request Hooks (agent → client, blocking)

Requests are delivered via JSON-RPC method `request` and **must** be answered. The agent **pauses** until the client returns a successful response. The message wrapper is:

```
{ "jsonrpc": "2.0", "method": "request", "id": "...", "params": { "type": "ApprovalRequest" | "ToolCallRequest", "payload": { ... } } }
```

### `ApprovalRequest`

- **Data**: `{ id, tool_call_id, sender, action, description, display? }`
- **Expected response type**: `ApprovalResponse` (as JSON-RPC result)
    - `{ request_id, response: "approve" | "approve_for_session" | "reject" }`
- **Flow impact**: The agent halts until approval is returned. `approve_for_session` allows auto-approving similar operations in the same session.

### `ToolCallRequest`

- **Data**: `{ id, name, arguments? }` where `arguments` is a JSON string
- **Expected response type**: `ToolResult` (as JSON-RPC result)
    - `{ tool_call_id, return_value: { is_error, output, message, display, extras? } }`
- **Flow impact**: The agent halts until the tool is executed and the result is provided. Returning `is_error: true` informs the model the tool failed, which changes its planning/next steps.

## Display Blocks (aux data for UI)

`ToolResult` and `ApprovalRequest` can carry `display` blocks that the UI can render. Types include `brief`, `diff`, `todo`, and `shell`, plus a generic fallback type.

## Known Gotchas and Workarounds

- **`initialize` is optional**: Some servers return `-32601` for `initialize`. Clients should optimistically call it, then fall back to direct `prompt` on `method not found`.
- **`TurnEnd` may be missing**: The docs note it can be omitted if a turn is interrupted. Always treat the `prompt` response status as the authoritative end-of-turn signal.
- **Optional/null fields are common**: `StatusUpdate` and other payloads may omit fields or send `null`. Defensive parsing is required.
- **Tool arguments may stream**: `ToolCall` arguments can be absent while `ToolCallPart` streams fragments. Buffer and concatenate parts before parsing JSON.
- **Approval event rename**: `ApprovalResponse` replaced `ApprovalRequestResolved` in Wire 1.1, but old names are still accepted. Support both for compatibility.
- **Requests block the agent**: If the client does not respond to `ApprovalRequest` or `ToolCallRequest`, the agent stalls. Workaround: implement explicit timeouts and surface a clear UI prompt so the operator can respond quickly.
- **Single-line JSON-RPC**: Each message is a single JSON line. Avoid emitting multi-line JSON; always serialize into one line with a trailing newline.

## Sources

- Kimi Code CLI Wire mode documentation: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
