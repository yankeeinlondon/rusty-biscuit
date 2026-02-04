# OpenCode Agentic CLI Hooks & Events

This document provides a comprehensive reference for the hooks and events available in the OpenCode Agentic CLI plugin system. These hooks allow you to intercept, modify, and respond to various agentic operations.

## Overview

OpenCode plugins are JavaScript/TypeScript modules that export a plugin function. This function receives a context object and returns a `Hooks` object containing hook implementations:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const MyPlugin: Plugin = async ({ client, project, $, directory, worktree }) => {
  return {
    // Hook implementations go here
  }
}
```

**Plugin Function Context:**

- `client` - OpenCode SDK client for API interactions
- `project` - Current project information
- `directory` - Current working directory
- `worktree` - Git worktree path
- `$` - Bun's shell API for executing commands

---

## Hook Categories

### 1. Event Hooks

The `event` hook allows subscribing to system events. Unlike other hooks, it receives all events and must filter by `event.type`.

**Hook Signature:**

```typescript
event?: (input: { event: Event }) => Promise<void>
```

**Data Received:**

- `event` - An Event object with discriminated union type based on `event.type`

**Return Type:** `Promise<void>`

**Flow Impact:** Events are notifications; returning void does not affect flow. Use for side effects like logging, notifications, or state tracking.

**Available Events (v2 SDK):**

| Event Type | Properties | Description |
|------------|------------|-------------|
| `installation.updated` | `{ version: string }` | OpenCode installation updated |
| `installation.update-available` | `{ version: string }` | New version available |
| `project.updated` | `Project` | Project configuration changed |
| `server.instance.disposed` | `{ directory: string }` | Server instance shut down |
| `server.connected` | `{ [key: string]: unknown }` | Connected to server |
| `global.disposed` | `{ [key: string]: unknown }` | Global state disposed |
| `lsp.client.diagnostics` | `{ serverID: string, path: string }` | LSP diagnostics received |
| `lsp.updated` | `{ [key: string]: unknown }` | LSP server updated |
| `file.edited` | `{ file: string }` | File was edited |
| `message.updated` | `{ info: Message }` | Message added/changed |
| `message.removed` | `{ sessionID: string, messageID: string }` | Message deleted |
| `message.part.updated` | `{ part: Part, delta?: string }` | Message part updated |
| `message.part.removed` | `{ sessionID: string, messageID: string, partID: string }` | Message part deleted |
| `permission.asked` | `PermissionRequest` | Permission requested from user |
| `permission.replied` | `{ sessionID: string, requestID: string, reply: "once" \| "always" \| "reject" }` | User responded to permission |
| `session.status` | `{ sessionID: string, status: SessionStatus }` | Session status changed |
| `session.idle` | `{ sessionID: string }` | Session became idle |
| `question.asked` | `QuestionRequest` | Question asked to user |
| `question.replied` | `{ sessionID: string, requestID: string, answers: Array<QuestionAnswer> }` | User answered question |
| `question.rejected` | `{ sessionID: string, requestID: string }` | Question was rejected |
| `session.compacted` | `{ sessionID: string }` | Session context compacted |
| `file.watcher.updated` | `{ file: string, event: "add" \| "change" \| "unlink" }` | File watcher event |
| `todo.updated` | `{ sessionID: string, todos: Array<Todo> }` | Todo list updated |
| `tui.prompt.append` | `{ text: string }` | Text appended to TUI prompt |
| `tui.command.execute` | `{ command: string }` | TUI command executed |
| `tui.toast.show` | `{ title?: string, message: string, variant: "info" \| "success" \| "warning" \| "error", duration?: number }` | Toast notification shown |
| `tui.session.select` | `{ sessionID: string }` | Session selection in TUI |
| `mcp.tools.changed` | `{ server: string }` | MCP tools changed |
| `mcp.browser.open.failed` | `{ mcpName: string, url: string }` | MCP browser failed to open |
| `command.executed` | `{ name: string, sessionID: string, arguments: string, messageID: string }` | Custom command executed |
| `session.created` | `{ info: Session }` | New session created |
| `session.updated` | `{ info: Session }` | Session updated |
| `session.deleted` | `{ info: Session }` | Session deleted |
| `session.diff` | `{ sessionID: string, diff: Array<FileDiff> }` | Session diff generated |
| `session.error` | `{ sessionID?: string, error?: ProviderAuthError \| UnknownError \| ... }` | Session error occurred |
| `vcs.branch.updated` | `{ branch?: string }` | Git branch changed |
| `pty.created` | `{ info: Pty }` | PTY created |
| `pty.updated` | `{ info: Pty }` | PTY updated |
| `pty.exited` | `{ id: string, exitCode: number }` | PTY process exited |
| `pty.deleted` | `{ id: string }` | PTY deleted |
| `worktree.ready` | `{ name: string, branch: string }` | Worktree ready |
| `worktree.failed` | `{ message: string }` | Worktree operation failed |

**Example:**

```typescript
event: async ({ event }) => {
  if (event.type === "session.idle") {
    await $`osascript -e 'display notification "Session completed!" with title "opencode"'`
  }
}
```

---

### 2. Tool Execution Hooks

These hooks intercept tool calls before and after execution, allowing modification or blocking.

#### `tool.execute.before`

**Hook Signature:**

```typescript
"tool.execute.before"?: (
  input: { tool: string; sessionID: string; callID: string },
  output: { args: any }
) => Promise<void>
```

**Data Received:**

- `input.tool` - Tool name (e.g., "bash", "edit", "write")
- `input.sessionID` - Current session ID
- `input.callID` - Unique tool call ID
- `output.args` - Mutable arguments object for the tool

**Return Type:** `Promise<void>`

**Flow Impact:**

- Modify `output.args` to change tool arguments before execution
- Throw an error to block the tool call entirely
- If blocked, the tool never executes and an error is returned to the agent

**Example:**

```typescript
"tool.execute.before": async (input, output) => {
  // Block reading .env files
  if (input.tool === "read" && output.args.filePath.includes(".env")) {
    throw new Error("Do not read .env files")
  }

  // Escape bash commands
  if (input.tool === "bash") {
    output.args.command = escape(output.args.command)
  }
}
```

#### `tool.execute.after`

**Hook Signature:**

```typescript
"tool.execute.after"?: (
  input: { tool: string; sessionID: string; callID: string },
  output: { title: string; output: string; metadata: any }
) => Promise<void>
```

**Data Received:**

- `input.tool` - Tool name
- `input.sessionID` - Current session ID
- `input.callID` - Tool call ID
- `output.title` - Mutable title for the tool result
- `output.output` - Mutable output string
- `output.metadata` - Mutable metadata object

**Return Type:** `Promise<void>`

**Flow Impact:**

- Modify the tool's displayed output or metadata
- Cannot block/retry after execution (use `before` for that)
- Useful for logging, formatting, or post-processing

**Example:**

```typescript
"tool.execute.after": async (input) => {
  if (input.tool === "edit") {
    console.log(`File edited: ${input.args.filePath}`)
  }
}
```

---

### 3. Shell Environment Hook

#### `shell.env`

**Hook Signature:**

```typescript
"shell.env"?: (
  input: { cwd: string },
  output: { env: Record<string, string> }
) => Promise<void>
```

**Data Received:**

- `input.cwd` - Current working directory
- `output.env` - Mutable environment variables object

**Return Type:** `Promise<void>`

**Flow Impact:**

- Inject or modify environment variables for ALL shell executions
- Affects both AI tool calls and user terminal sessions
- Variables are merged with existing environment

**Example:**

```typescript
"shell.env": async (input, output) => {
  output.env.MY_API_KEY = "secret"
  output.env.PROJECT_ROOT = input.cwd
}
```

---

### 4. Chat/LLM Hooks

These hooks allow modifying LLM interactions.

#### `chat.message`

**Hook Signature:**

```typescript
"chat.message"?: (
  input: {
    sessionID: string
    agent?: string
    model?: { providerID: string; modelID: string }
    messageID?: string
    variant?: string
  },
  output: { message: UserMessage; parts: Part[] }
) => Promise<void>
```

**Data Received:**

- Input contains session and model information
- `output.message` - Mutable user message object
- `output.parts` - Mutable message parts array

**Return Type:** `Promise<void>`

**Flow Impact:**

- Modify the message before it's sent to the LLM
- Add or modify message parts
- Cannot block the message (use permissions for that)

#### `chat.params`

**Hook Signature:**

```typescript
"chat.params"?: (
  input: { sessionID: string; agent: string; model: Model; provider: ProviderContext; message: UserMessage },
  output: { temperature: number; topP: number; topK: number; options: Record<string, any> }
) => Promise<void>
```

**Data Received:**

- `output.temperature` - Mutable temperature setting
- `output.topP` - Mutable top-p setting
- `output.topK` - Mutable top-k setting
- `output.options` - Mutable additional options object

**Return Type:** `Promise<void>`

**Flow Impact:**

- Modify LLM parameters dynamically
- Adjust temperature, sampling, or provider-specific options
- Changes affect the current request only

#### `chat.headers`

**Hook Signature:**

```typescript
"chat.headers"?: (
  input: { sessionID: string; agent: string; model: Model; provider: ProviderContext; message: UserMessage },
  output: { headers: Record<string, string> }
) => Promise<void>
```

**Data Received:**

- `output.headers` - Mutable HTTP headers object

**Return Type:** `Promise<void>`

**Flow Impact:**

- Add custom HTTP headers to LLM API requests
- Useful for routing, tracking, or authentication

---

### 5. Permission Hook

#### `permission.ask`

**Hook Signature:**

```typescript
"permission.ask"?: (
  input: Permission,
  output: { status: "ask" | "deny" | "allow" }
) => Promise<void>
```

**Data Received:**

- `input` - Permission request details (tool name, patterns, metadata)
- `output.status` - Mutable status, defaults to "ask"

**Return Type:** `Promise<void>`

**Flow Impact:**

- Set `output.status = "allow"` to auto-approve the permission
- Set `output.status = "deny"` to auto-reject the permission
- Keep `"ask"` to prompt the user
- **Critical:** This hook only fires if the permission system evaluates to "ask" - it cannot override "deny" rules

**Example:**

```typescript
"permission.ask": async (input, output) => {
  // Auto-approve safe bash commands
  if (input.permission === "bash") {
    const cmd = input.metadata?.command ?? ""
    if (isSafeCommand(cmd)) {
      output.status = "allow"
    }
  }
}
```

---

### 6. Command Hooks

#### `command.execute.before`

**Hook Signature:**

```typescript
"command.execute.before"?: (
  input: { command: string; sessionID: string; arguments: string },
  output: { parts: Part[] }
) => Promise<void>
```

**Data Received:**

- `input.command` - Command name
- `input.sessionID` - Session ID
- `input.arguments` - Command arguments as string
- `output.parts` - Mutable parts array for command output

**Return Type:** `Promise<void>`

**Flow Impact:**

- Modify command arguments or inject output parts
- Can be used to short-circuit command execution

---

### 7. System Prompt Hooks (Experimental)

#### `experimental.chat.system.transform`

**Hook Signature:**

```typescript
"experimental.chat.system.transform"?: (
  input: { sessionID?: string; model: Model },
  output: { system: string[] }
) => Promise<void>
```

**Data Received:**

- `input.sessionID` - Optional session ID
- `input.model` - Model information
- `output.system` - Mutable array of system prompt strings

**Return Type:** `Promise<void>`

**Flow Impact:**

- Inject custom context into the system prompt
- Push strings to `output.system` to add context
- Useful for project-specific rules or persistent context

**Example:**

```typescript
"experimental.chat.system.transform": async (input, output) => {
  output.system.push(`<custom-context>
    Important project rules go here.
  </custom-context>`)
}
```

#### `experimental.chat.messages.transform`

**Hook Signature:**

```typescript
"experimental.chat.messages.transform"?: (
  input: {},
  output: {
    messages: {
      info: Message
      parts: Part[]
    }[]
  }
) => Promise<void>
```

**Data Received:**

- `output.messages` - Mutable array of message objects

**Return Type:** `Promise<void>`

**Flow Impact:**

- Modify the entire message history before sending to LLM
- Can add, remove, or modify messages
- Use with caution - can break conversation flow

---

### 8. Session Compaction Hook (Experimental)

#### `experimental.session.compacting`

**Hook Signature:**

```typescript
"experimental.session.compacting"?: (
  input: { sessionID: string },
  output: { context: string[]; prompt?: string }
) => Promise<void>
```

**Data Received:**

- `input.sessionID` - Session being compacted
- `output.context` - Mutable array of context strings
- `output.prompt` - Optional replacement for entire compaction prompt

**Return Type:** `Promise<void>`

**Flow Impact:**

- Push strings to `output.context` to add domain-specific context to the compaction summary
- Set `output.prompt` to completely replace the compaction prompt (ignores `output.context`)
- Critical for preserving important state across compaction

**Example:**

```typescript
"experimental.session.compacting": async (input, output) => {
  // Add context that should persist across compaction
  output.context.push(`
## Custom Context
Include any state that should persist:
- Current task status
- Important decisions made
- Files being actively worked on
`)
}
```

---

### 9. Text Completion Hook (Experimental)

#### `experimental.text.complete`

**Hook Signature:**

```typescript
"experimental.text.complete"?: (
  input: { sessionID: string; messageID: string; partID: string },
  output: { text: string }
) => Promise<void>
```

**Data Received:**

- Input identifies the specific message part
- `output.text` - Mutable completed text

**Return Type:** `Promise<void>`

**Flow Impact:**

- Modify or post-process generated text
- Can transform, filter, or enhance LLM output

---

## Hook Execution Order

When multiple plugins define the same hook, they execute sequentially in plugin load order:

1. Global config (`~/.config/opencode/opencode.json`)
2. Project config (`opencode.json`)
3. Global plugin directory (`~/.config/opencode/plugins/`)
4. Project plugin directory (`.opencode/plugins/`)

**Important:** For hooks that mutate output objects (like `tool.execute.before`), each plugin sees the mutations from previous plugins. The final mutated output is what gets used.

---

## Known Gotchas and Workarounds

### 1. **Plugin Hooks Don't Intercept Subagent Tool Calls**

**Problem:** Hooks defined in `tool.execute.before` successfully block tool calls from the primary agent but **do not intercept tool calls from subagents** spawned via the `task` tool. This allows security policies to be bypassed.

**Impact:** Any agent can bypass plugin restrictions by delegating work to a subagent.

**Workaround:**

- Configure tool restrictions per-agent in `opencode.json`:

```json
{
  "agent": {
    "my-agent": {
      "tools": {
        "grep": false,
        "glob": false
      }
    }
  }
}
```

- Must manually configure every built-in subagent
- Does not work for custom subagents created dynamically
- Duplicates policy logic between plugins and agent configs

**Status:** Issue #5894 - Under investigation

---

### 2. **Permission Hook Only Fires for "Ask" Decisions**

**Problem:** The `permission.ask` hook only fires when the permission system evaluates a rule to `"ask"`. It **cannot override "deny" rules** - if the permission config denies a tool, the hook never fires.

**Impact:** Cannot use plugins to create exceptions to deny rules.

**Workaround:**

- Set permissions to `"ask"` in config, then use the hook to auto-approve/deny programmatically:

```typescript
"permission.ask": async (input, output) => {
  if (shouldAutoAllow(input)) {
    output.status = "allow"
  } else if (shouldAutoDeny(input)) {
    output.status = "deny"
  }
  // Otherwise keep "ask" to prompt user
}
```

---

### 3. **Plugin Package Uses Outdated SDK Types (v1 vs v2)**

**Problem:** The `@opencode-ai/plugin` package imports types from `@opencode-ai/sdk` (v1), but OpenCode v1.1.x+ uses `@opencode-ai/sdk/v2` with different type structures:

- v1: Nested `path`/`body`/`query` parameters
- v2: Flattened parameters (e.g., `sessionID` instead of `path.id`)
- New event types like `permission.asked` don't exist in v1 types

**Impact:** TypeScript errors when using newer event types or v2 API patterns.

**Workaround:**
Cast events to v2 types:

```typescript
import type { Plugin } from "@opencode-ai/plugin"
import type { Event, EventSessionStatus } from "@opencode-ai/sdk/v2"

export const MyPlugin: Plugin = async () => {
  return {
    event: async ({ event: _event }) => {
      const event = _event as Event  // Cast to v2 Event

      if (event.type === "permission.asked") {
        // Now TypeScript knows the type
      }
    }
  }
}
```

**Status:** Issue #7147 and #7641 - PRs in progress to migrate plugin system to SDK v2

---

### 4. **Permission Hook Defined But Not Triggered**

**Problem:** The `permission.ask` hook type exists in the plugin SDK but was **not actually triggered** by the permission system until recently. The hook definition existed without implementation.

**Impact:** Plugins defining `permission.ask` hooks would appear to work but never actually fire.

**Workaround:**

- Ensure you're on OpenCode v1.1.2+ where this was fixed (PR #7077)
- Verify hook is firing by adding logging

---

### 5. **Event Hook Receives All Events - Must Filter**

**Problem:** Unlike other hooks that target specific operations, the `event` hook receives **every single system event**. You must manually filter by `event.type`.

**Impact:** Easy to accidentally process the wrong events or create performance issues.

**Workaround:**
Always use early returns or switch statements:

```typescript
event: async ({ event }) => {
  // Early return pattern
  if (event.type !== "session.idle") return

  // Process idle event
}
```

---

### 6. **Hooks Run Sequentially - Mutation Order Matters**

**Problem:** When multiple plugins define the same hook, they run in sequence. Each plugin sees the mutations from previous ones. This can lead to confusing interactions.

**Impact:** One plugin's modifications may conflict with another's expectations.

**Workaround:**

- Keep plugins focused and composable
- Document hook interactions in plugin READMEs
- Use unique namespaces for custom data to avoid collisions

---

### 7. **Context Destructuring Gotcha**

**Problem:** The plugin function receives a **context object**, not individual parameters. A common mistake is treating the context as the client directly.

**Incorrect:**

```typescript
export const MyPlugin: Plugin = async (client) => {
  await client.session.prompt({ ... })  // FAILS
}
```

**Correct:**

```typescript
export const MyPlugin: Plugin = async ({ client }) => {
  await client.session.prompt({ ... })  // Works
}
```

---

### 8. **Stop Hook Not Documented in Official Docs**

**Problem:** The `stop` hook (for intercepting session stop attempts) works but is **not documented** in the official OpenCode documentation.

**Impact:** Undocumented feature that may change behavior.

**Usage:**

```typescript
stop: async (input) => {
  const sessionId = input.sessionID || input.session_id

  // Check if work is complete
  if (!workComplete) {
    // Prompt agent to continue
    await client.session.prompt({
      path: { id: sessionId },
      body: {
        parts: [{ type: "text", text: "Please complete X before stopping." }]
      }
    })
  }
}
```

---

### 9. **State Persistence Across Hooks**

**Problem:** Plugin functions are initialized once at startup. State variables defined in the plugin function scope persist across all hook invocations but are **not persisted across OpenCode restarts**.

**Impact:** Cannot rely on state surviving application restart.

**Workaround:**

- Use session-keyed Maps for per-session state:

```typescript
const sessions = new Map<string, SessionState>()

export const MyPlugin: Plugin = async ({ client }) => {
  return {
    event: async ({ event }) => {
      const sessionId = (event as any).session_id
      if (event.type === "session.created") {
        sessions.set(sessionId, { filesModified: [], commitMade: false })
      }
      if (event.type === "session.deleted") {
        sessions.delete(sessionId)
      }
    }
  }
}
```

- For persistent state, use files or external storage

---

### 10. **No Hook for Tool Execution Errors**

**Problem:** There is no `tool.execute.error` hook to handle tool call failures. The `tool.execute.after` hook only fires on success.

**Impact:** Cannot implement centralized error handling for tool failures.

**Workaround:**

- Check for error conditions in `tool.execute.after` by examining output
- Use `event` hook to listen for `session.error` events
- Handle errors at the agent level

**Status:** Feature request #10027

---

## Best Practices

1. **Always use TypeScript** - The plugin system is designed for TypeScript; JavaScript works but loses type safety
2. **Filter events early** - Use early returns to avoid processing irrelevant events
3. **Namespace your data** - Avoid conflicts with other plugins by using unique keys
4. **Handle errors gracefully** - Don't let plugin errors crash the agent
5. **Test with subagents** - If implementing security policies, verify they work with `@general` subagent
6. **Use structured logging** - Prefer `client.app.log()` over `console.log` for better integration
7. **Document dependencies** - If your plugin needs npm packages, include a `package.json`

---

## References

- Official Plugin Documentation: https://opencode.ai/docs/plugins
- Plugin Type Definitions: `@opencode-ai/plugin` package
- SDK Type Definitions: `@opencode-ai/sdk` (v1) and `@opencode-ai/sdk/v2` (v2)
- GitHub Issues: https://github.com/anomalyco/opencode/issues
    - #5894: Subagent tool call interception
    - #7006: Permission hook not triggered
    - #7147: SDK v2 type alignment
    - #7641: Plugin hook types not aligned
