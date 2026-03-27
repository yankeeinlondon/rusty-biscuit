---
homepage: https://github.com/QwenLM/qwen-code
docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
hooks: https://github.com/QwenLM/qwen-code/issues/268
---

# Qwen Code Hooks and Events (2026-02)

## Scope

This document covers the hook and event surfaces available in Qwen Code
(Alibaba/QwenLM's open-source agentic CLI for Qwen3-Coder). As of v0.10.5
(February 2026), Qwen Code does **not** ship a Claude-style lifecycle hook
system (PreToolUse, PostToolUse, SessionStart, etc.) in the CLI or settings
files. The roadmap lists hooks as "In Progress" (Phase 2, P2 priority), and the
official response to issue #1708 confirms hooks are "still in development,
expected to go live within the next few weeks." The extension system also does
not yet expose hook registration for plugins.

The primary hook-like integration surfaces today are:

1. **SDK permission callback** (`canUseTool`) that gates tool execution
2. **Internal subagent hooks** (`SubagentHooks` interface) for subagent tool
   lifecycle monitoring
3. **MCP tool handlers** (external or SDK-embedded) that define tool outputs
4. **Headless stream-json events** for monitoring and automation
5. **Extension system** for commands, skills, subagents, and MCP servers (no
   lifecycle hooks)

**Home page:** https://github.com/QwenLM/qwen-code

**Documentation:** https://qwenlm.github.io/qwen-code-docs/en/users/overview/

## Configuration

### Settings file locations

Qwen Code uses JSON settings files. There is no `hooks` key in the settings
schema today.

| Priority | Location | Scope | Shareable |
|:---------|:---------|:------|:----------|
| 1 (highest) | CLI flags / env vars | Invocation | No |
| 2 | `.qwen/settings.json` | Project/workspace | Yes (committed) |
| 3 | `~/.qwen/settings.json` | User (all projects) | No (local) |
| 4 (lowest) | System defaults | Built-in | N/A |

Project settings override user settings. CLI flags and environment variables
override both.

### Extension locations

Extensions provide commands, skills, subagents, and MCP servers but do **not**
currently support lifecycle hooks.

| Location | Discovery |
|:---------|:----------|
| `~/.qwen/extensions/<name>/qwen-extension.json` | Global (all projects) |
| Extension install via `/extensions install <source>` | Runtime command |

### What a future hooks configuration might look like

The Claude-to-Qwen extension converter (`claude-converter.ts`) recognizes a
`hooks` field in Claude plugin configs but logs a warning: "Hooks are not yet
supported." The converter preserves hooks on agent configs as opaque `unknown`
data without processing them. This suggests the eventual hook configuration
will likely follow the Claude/Gemini pattern (a `hooks` key in
`settings.json`), but no schema is finalized.

```json
{
  "hooks": {
    "NOTE": "This key is NOT yet recognized by Qwen Code settings (v0.10.x)"
  }
}
```

## Hook Events

Qwen Code does not expose user-facing hook events in the CLI. The surfaces
documented below are the closest equivalents available today.

### 1. SDK Permission Callback: `canUseTool`

**Where it lives:** `@qwen-code/sdk` TypeScript SDK (`query()` options) and
Java SDK.

**Trigger:** Called when a tool execution requires confirmation (based on
permission mode and tool classification).

**Event Payload:**

```typescript
type CanUseTool = (
  toolName: string,
  input: Record<string, unknown>,
  options: {
    signal: AbortSignal;
    suggestions?: PermissionSuggestion[] | null;
  },
) => Promise<PermissionResult>;
```

- `toolName`: string identifier of the tool being called
- `input`: tool arguments object (key-value pairs)
- `options.signal`: AbortSignal for timeout/cancellation
- `options.suggestions`: optional array of permission suggestions

**Event Response:**

```typescript
type PermissionResult =
  | { behavior: 'allow'; updatedInput: Record<string, unknown> }
  | { behavior: 'deny'; message: string; interrupt?: boolean };
```

- `allow`: executes the tool. `updatedInput` replaces the original arguments
  before execution.
- `deny`: blocks execution. `message` is surfaced to the model as the denial
  reason. If `interrupt` is true, the entire session is interrupted.

**Flow impact:**

The callback is only invoked when the permission system requires confirmation.
The following mechanisms can bypass it entirely:

- `permissionMode: 'yolo'` auto-approves everything
- Tools listed in `allowedTools` are auto-approved
- Tools listed in `excludeTools` are auto-denied before the callback runs
- `permissionMode: 'plan'` blocks all non-read-only tools without invoking the
  callback

**Priority chain:**

1. `excludeTools` (absolute block)
2. `permissionMode: 'plan'` (non-read-only block)
3. `permissionMode: 'yolo'` (universal auto-approve)
4. `allowedTools` (auto-approve for matching tools)
5. `canUseTool` callback (custom logic)
6. Default denial

**SDK runtime controls:**

The SDK `Query` instance also provides mid-session methods:

- `setPermissionMode()`: change approval rules during execution
- `setModel()`: switch AI models dynamically
- `interrupt()`: halt current operations

**Gotchas:**

- The callback must resolve within 60 seconds or the tool call is auto-denied.
- If `permissionMode` is `yolo` or a tool is in `allowedTools`, the callback
  is never invoked.
- The `suggestions` parameter is optional and may be null; always check before
  iterating.

### 2. Internal Subagent Hooks: `SubagentHooks`

**Where it lives:** `packages/core/src/subagents/subagent-hooks.ts` in the
Qwen Code source. Used internally by the `Subagent` class and
`SubagentManager`.

**Trigger:** These hooks fire during subagent tool execution and lifecycle
events. They are **internal SDK hooks**, not user-configurable via settings
files.

#### `preToolUse`

**Event Payload:**

```typescript
interface PreToolUsePayload {
  subagentId: string;
  name: string;          // subagent name
  toolName: string;
  args: Record<string, unknown>;
  timestamp: number;
}
```

**Event Response:** `Promise<void> | void`

**Flow impact:** Notification only. The hook fires after the `TOOL_CALL` event
is emitted but before execution begins. It cannot block or modify the tool
call. Intended for instrumentation, logging, and telemetry.

#### `postToolUse`

**Event Payload:**

```typescript
interface PostToolUsePayload extends PreToolUsePayload {
  success: boolean;
  durationMs: number;
  errorMessage?: string;
}
```

**Event Response:** `Promise<void> | void`

**Flow impact:** Notification only. Fires in the `onAllToolCallsComplete`
callback after tool execution finishes. Provides timing and success/error
status for metrics collection. Cannot modify the tool result.

#### `onStop`

**Event Payload:**

```typescript
interface SubagentStopPayload {
  subagentId: string;
  name: string;          // subagent name
  terminateReason: string;
  summary: Record<string, unknown>;
  timestamp: number;
}
```

**Event Response:** `Promise<void> | void`

**Flow impact:** Notification only. Fires in the `finally` block after subagent
execution terminates. Provides the termination reason and a summary object for
cleanup, reporting, or analytics. Cannot prevent the subagent from stopping.

**Gotchas:**

- These hooks are internal to the `packages/core` module and are not exposed
  through user-facing configuration or the public SDK.
- All three hooks are fire-and-forget notification patterns. None can block,
  modify, or control agentic flow.
- The `preToolUse` hook is called with `void` (not awaited), while
  `postToolUse` and `onStop` are awaited. This means `preToolUse` errors are
  silently swallowed.

### 3. MCP Tool Handlers

**Where it lives:** MCP server implementation (stdio/SSE/HTTP) or SDK
`tool()`/`createSdkMcpServer()`.

**Trigger:** Model calls a tool registered by an MCP server.

**Event Payload:**

- `args`: JSON object validated against the tool's JSON Schema
- (SDK) `handler(args, extra)` receives parsed args plus context

**Event Response:** MCP `CallToolResult`:

```json
{
  "content": [
    { "type": "text", "text": "..." },
    { "type": "image", "data": "base64...", "mimeType": "image/png" },
    { "type": "resource", "uri": "..." }
  ],
  "isError": false
}
```

**Flow impact:**

- `content` is injected into the model context as tool output, split into
  `llmContent` (for the model) and `returnDisplay` (for the user).
- `isError: true` marks a tool failure and nudges the model to recover or
  choose an alternative approach.

**MCP tool filtering:**

MCP servers can be configured with `includeTools` and `excludeTools` arrays
in the `mcpServers` section of `settings.json`. The `trust` boolean bypasses
all confirmation dialogs for a server's tools.

**Gotchas:**

- Tool names can be sanitized and prefixed when conflicts arise between
  servers.
- Certain JSON Schema features may be stripped for API compatibility (e.g.,
  `additionalProperties`).
- The `timeout` setting on MCP server config controls request timeout in
  milliseconds.

### 4. MCP Prompt Handlers (Slash Command Prompts)

**Where it lives:** MCP server `registerPrompt`.

**Trigger:** User invokes a prompt name as a slash command.

**Event Payload:**

- Prompt arguments validated by the prompt schema.

**Event Response:**

```typescript
{
  messages: [
    {
      role: 'user',
      content: { type: 'text', text: '...' }
    }
  ]
}
```

**Flow impact:** The CLI converts the returned messages into the model prompt
for the next turn, effectively acting as a macro that expands into a new user
message.

### 5. Headless Stream-JSON Events

**Where it lives:** CLI headless mode (`qwen -p "..." --output-format stream-json`).

**Trigger:** Emitted during execution for real-time monitoring and automation.

**Event types:**

| Event type | Subtype / fields | Description |
|:-----------|:-----------------|:------------|
| `system` | `subtype: "session_start"` | Session metadata: session ID, model, configuration |
| `assistant` | `model`, `role`, `content`, `usage` | Full assistant message with content blocks and token usage |
| `result` | `subtype: "success"` or error | Final status with `duration_ms`, usage stats, and summary |

**Partial message events** (with `--include-partial-messages`):

| Event | Description |
|:------|:------------|
| `message_start` | Message initialization begins |
| `content_block_delta` | Incremental content update (streaming token) |

**Output format:** One JSON object per line (line-delimited JSON).

**Flow impact:** None on the model. This is an output-only stream for external
consumers. There is no input channel for feeding events back into the agent.

**Gotchas:**

- The `--input-format stream-json` flag exists but is not fully stable. Prefer
  the SDK for bidirectional automation until the protocol is finalized.
- Partial message events do not have a fully documented stable schema. Treat
  them as best-effort telemetry and implement tolerant parsing.

### 6. Approval Mode (CLI Tool Gating)

**Where it lives:** CLI settings (`settings.json`) and keyboard shortcuts.

While not a hook system, the four approval modes provide tool-gating behavior
that covers some hook use cases:

| Mode | File edits | Shell commands | Use case |
|:-----|:-----------|:---------------|:---------|
| Plan | Blocked | Blocked | Read-only analysis |
| Default | Manual approval | Manual approval | Balanced safety |
| Auto-Edit | Auto-approved | Manual approval | Routine development |
| YOLO | Auto-approved | Auto-approved | Trusted environments |

Switch modes interactively with **Shift+Tab** (or **Tab** on Windows) or set
`defaultMode` in `settings.json`.

## Matcher System

Qwen Code does not currently have a matcher system. No hook events exist in
user-facing configuration, so there is nothing to match against. The internal
`SubagentHooks` fire unconditionally for all tool calls within a subagent.

When the hooks system ships (per roadmap), it will likely adopt a regex-based
matcher pattern similar to Claude Code and Gemini CLI, given that the
`claude-converter.ts` already recognizes the `hooks` configuration shape from
those tools.

## Extension System (Current Extensibility Model)

Instead of lifecycle hooks, Qwen Code's primary extensibility is through the
extension system:

| Extension capability | Description |
|:---------------------|:------------|
| **MCP servers** | Custom tools via Model Context Protocol |
| **Custom commands** | Slash commands from markdown files in `commands/` |
| **Custom skills** | AI-invocable skills from `skills/` directories |
| **Custom subagents** | Specialized AI assistants from `agents/` directory |
| **Context files** | Persistent instructions via `QWEN.md` |

Extensions are packaged in directories with a `qwen-extension.json` manifest
and installed into `~/.qwen/extensions/`. Cross-platform compatibility is
supported: extensions from Gemini CLI (`gemini-extension.json`) and Claude Code
(`claude-plugin.json`) are auto-converted at import time, though Claude hooks
are dropped with a warning.

## Gotchas and Workarounds

### 1. No official lifecycle hook system (yet)

**Problem:** Qwen Code does not support Claude-style hook events
(PreToolUse, PostToolUse, SessionStart, Stop, etc.) in `settings.json` or the
CLI. Users attempting to add a `hooks` key to `settings.json` will find it
silently ignored (issue #1708).

**Evidence:** Feature request issue #268 is open. The roadmap lists hooks as
"In Progress" (P2). The maintainers confirmed in issue #1708 that hooks are
"still in development."

**Workaround:** Use the SDK `canUseTool` callback for tool gating. For
monitoring, consume the headless stream-json output. For policy enforcement,
use the approval modes and MCP tool filtering (`includeTools`/`excludeTools`).

### 2. `canUseTool` only runs when confirmation is required

**Problem:** If `permissionMode` is `yolo` or a tool is in `allowedTools`, the
`canUseTool` callback is skipped entirely.

**Workaround:** Use `excludeTools` or a stricter permission mode to force the
callback to run. Or use `permissionMode: 'default'` and handle all decisions
in the callback.

### 3. `canUseTool` timeout auto-denies

**Problem:** The callback must resolve within 60 seconds or the tool call is
auto-denied. There is no configuration to change this timeout.

**Workaround:** Precompute policy decisions or proxy to a fast local approval
service. Avoid network calls to external systems from within the callback.

### 4. SubagentHooks are internal only

**Problem:** The `SubagentHooks` interface (`preToolUse`, `postToolUse`,
`onStop`) exists in the core package but is not exposed through any
user-facing configuration or public SDK API.

**Workaround:** To observe subagent behavior, consume the headless stream-json
output which includes subagent events, or implement custom MCP tools that log
their own invocations.

### 5. Claude hooks in imported extensions are silently dropped

**Problem:** When importing a Claude Code extension (`claude-plugin.json`) that
contains hooks, Qwen Code's converter logs a warning and drops the hooks. The
extension installs successfully but hooks do not function.

**Workaround:** Reimplement the hook behavior using MCP tools, approval modes,
or the SDK `canUseTool` callback until native hooks ship.

### 6. Stream-json input mode is not fully stable

**Problem:** The `--input-format stream-json` flag is documented but not fully
stable. It must be paired with `--output-format stream-json`.

**Workaround:** Prefer the TypeScript or Java SDK for bidirectional automation
until the headless protocol is finalized.

### 7. MCP tool name sanitization and conflicts

**Problem:** When multiple MCP servers register tools with the same name, tools
can be renamed with server prefixes. Certain JSON Schema features are stripped
for compatibility.

**Workaround:** Use unique tool names across MCP servers. Avoid relying on
schema features that may be stripped (e.g., `additionalProperties`).

### 8. No extension hooks for lifecycle events

**Problem:** The extension system (`qwen-extension.json`) supports MCP servers,
commands, skills, and subagents but does not provide lifecycle hook
registration. There is no way for an extension to intercept tool calls, session
events, or model interactions.

**Workaround:** Use MCP tools within extensions to create tool-level
interception points. For broader lifecycle control, the SDK is the only option.

## Sources

- Qwen Code GitHub repository: https://github.com/QwenLM/qwen-code
- Qwen Code documentation: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
- Qwen Code settings reference: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/
- Qwen Code headless mode docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
- Qwen Code approval mode docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/
- Qwen Code extensions docs: https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/
- Qwen Code SDK TypeScript docs: https://raw.githubusercontent.com/QwenLM/qwen-code/main/docs/developers/sdk-typescript.md
- Qwen Code roadmap: https://raw.githubusercontent.com/QwenLM/qwen-code/main/docs/developers/roadmap.md
- SubagentHooks source: https://raw.githubusercontent.com/QwenLM/qwen-code/main/packages/core/src/subagents/subagent-hooks.ts
- Claude converter source (hooks handling): https://raw.githubusercontent.com/QwenLM/qwen-code/main/packages/core/src/extension/claude-converter.ts
- Hook feature request (issue #268): https://github.com/QwenLM/qwen-code/issues/268
- Hooks not working report (issue #1708): https://github.com/QwenLM/qwen-code/issues/1708
- Qwen Code MCP server docs: https://raw.githubusercontent.com/QwenLM/qwen-code/main/docs/developers/tools/mcp-server.md
