---
prompt: |-
    do online research on the hooks/event which can be leveraged when using the Agentic Qwen Code CLI (https://github.com/QwenLM/qwen-code). Describe each hook, what data it returns, what kind of return type is expected and how that return type effects the agentic flow. in the end describe any known gotcha in working with this event/hook model and how people have gotten around these quirks or shortcomings.
---

# Qwen Code hook/event surfaces (2026-02)

## Quick take

Qwen Code does not currently ship a Claude-style hook system (pre-tool, post-tool, or injection hooks) in the CLI. The primary hook-like surfaces today are:

- SDK permission callback (`canUseTool`) that gates tool execution
- MCP tool handlers (external or SDK-embedded) that define tool outputs
- MCP prompt handlers (slash-command prompts) that generate messages
- Headless stream-json output events for monitoring and automation

An open issue requests Claude-like hooks, which indicates the feature is not yet available in the official CLI.

## Hook/event surfaces

### 1) SDK permission hook: `canUseTool`

**Where it lives:** `@qwen-code/sdk` (`query()` options)

**Trigger:** Called when a tool execution requires confirmation.

**Inputs (from SDK docs):**

- `toolName`: string (tool identifier)
- `input`: tool arguments object
- context with `signal` (AbortSignal for timeout/cancel)

**Return type (from SDK docs):**

```ts
{
  behavior: 'allow' | 'deny';
  updatedInput?: unknown;
  message?: string;
}
```

**Flow impact:**

- `allow` executes the tool; `updatedInput` replaces original arguments.
- `deny` blocks execution; `message` is surfaced to the model as the reason.
- If the callback does not respond within 60 seconds, the tool call is auto-denied.

**Notes:** `allowedTools`, `excludeTools`, and `permissionMode` can bypass or short-circuit this hook, so it is not always invoked.

### 2) MCP tool handler (external MCP server or SDK-embedded server)

**Where it lives:** MCP server implementation (stdio/SSE/HTTP) or SDK `tool()`/`createSdkMcpServer()`.

**Trigger:** Model calls a tool registered by an MCP server.

**Inputs:**

- `args`: JSON object validated against the tool schema.
- (SDK) `handler(args, extra)` receives the parsed args plus extra context.

**Return type:** MCP `CallToolResult` with `content` blocks and optional `isError`:

```json
{
  "content": [
    { "type": "text", "text": "..." },
    { "type": "image", "data": "...", "mimeType": "image/png" },
    { "type": "resource", "uri": "..." }
  ],
  "isError": false
}
```

**Flow impact:**

- `content` is injected into the model context as tool output.
- `isError: true` marks a tool failure and nudges the model to recover or choose another path.

### 3) MCP prompt handler (slash command prompts)

**Where it lives:** MCP server `registerPrompt`.

**Trigger:** User invokes a prompt name as a slash command (`/poem-writer ...`).

**Inputs:**

- Prompt args validated by the prompt schema.

**Return type:**

```ts
{
  messages: [
    {
      role: 'user',
      content: { type: 'text', text: '...' }
    }
  ]
}
```

**Flow impact:** The CLI converts the returned messages into the model prompt for the next turn, effectively acting as a macro that expands into a new user message.

### 4) Headless stream-json events

**Where it lives:** CLI headless mode (`--output-format stream-json`).

**Trigger:** Emitted during execution; useful for observers/automation clients.

**Event payloads (documented types):**

- `system` (e.g., `session_start`): session metadata, model, ids.
- `assistant`: full assistant message with content blocks and usage.
- `result`: final status (`success`/`error`), duration, usage, and summary.

**Return type:** One JSON object per line (line-delimited JSON).

**Flow impact:** None on the model by itself. This is an output stream for external consumers.

### 5) Partial message events (stream-json + partials)

**Where it lives:** CLI headless mode with `--include-partial-messages` or SDK `includePartialMessages`.

**Trigger:** Emitted while the model is still generating.

**Payloads:** The docs mention partial events (e.g., `message_start`, `content_block_delta`), but do not define a stable schema.

**Flow impact:** None on the model. These are incremental updates for UIs and loggers.

## Gotchas and workarounds

1) **No official Claude-style hook system (yet).**
   - Evidence: Feature request for Claude-like hooks is open in the Qwen Code repo.
   - Workaround: Use the SDK `canUseTool` callback for tool gating, or wrap Qwen Code with a stream-json consumer that enforces policy externally.

2) **`canUseTool` only runs when confirmation is required.**
   - If `permissionMode` is `yolo` or a tool is in `allowedTools`, the callback is skipped.
   - Workaround: Use `excludeTools` or a stricter permission mode to force the callback to run.

3) **`canUseTool` timeouts auto-deny.**
   - The callback must resolve within 60 seconds or the tool call is denied.
   - Workaround: Precompute policy decisions or proxy to a fast local approval service.

4) **MCP tool name changes and schema sanitation.**
   - Tool names can be sanitized and prefixed on conflicts; certain schema fields are stripped for compatibility.
   - Workaround: Use unique tool names and avoid relying on schema features that get removed (e.g., `additionalProperties`).

5) **Stream-json input mode is under construction.**
   - Docs indicate `--input-format stream-json` is not fully stable and must be paired with stream-json output.
   - Workaround: Prefer the SDK for bidirectional automation until the protocol is finalized.

## Sources

- Qwen Code SDK docs: https://raw.githubusercontent.com/QwenLM/qwen-code/main/docs/developers/sdk-typescript.md
- Qwen Code headless mode docs: https://raw.githubusercontent.com/QwenLM/qwen-code/main/docs/users/features/headless.md
- Qwen Code MCP server docs: https://raw.githubusercontent.com/QwenLM/qwen-code/main/docs/developers/tools/mcp-server.md
- Hook feature request (issue #268): https://api.github.com/repos/QwenLM/qwen-code/issues/268
