---
homepage: https://opencode.ai
docs: https://opencode.ai/docs
hooks: https://opencode.ai/docs/plugins
---

# OpenCode Agentic CLI Hooks & Events

This document provides a comprehensive reference for the hooks and events available in OpenCode (the open-source AI coding agent by Anomaly). OpenCode uses a JavaScript/TypeScript plugin system where plugins export hook functions that intercept, modify, and respond to various agentic operations. The current release is v1.2.x.

## Home Page

https://opencode.ai

## Documentation

https://opencode.ai/docs

## Configuration

OpenCode plugins are configured through the `plugin` key in OpenCode's JSON/JSONC configuration files and by placing plugin files in designated directories.

### Config file locations (lowest to highest precedence)

| # | Location | Scope | Description |
|---|----------|-------|-------------|
| 1 | Remote `.well-known/opencode` | Organization | Organizational defaults fetched at startup |
| 2 | `~/.config/opencode/opencode.json` | Global (user) | User-wide preferences |
| 3 | `OPENCODE_CONFIG` env var path | Custom | Explicit override via environment variable |
| 4 | `opencode.json` (project root) | Project | Per-project configuration |
| 5 | `.opencode/opencode.json` | Project | Alternate project-level config (also agents, commands, plugins) |
| 6 | `OPENCODE_CONFIG_CONTENT` env var | Inline | Raw JSON content override |
| 7 | Managed config directory | Enterprise | Admin-controlled, highest priority override |

Managed config directories by OS:
- macOS: `/Library/Application Support/opencode`
- Linux: `/etc/opencode`
- Windows: `%ProgramData%\opencode`

Configuration files are **merged**, not replaced. Later sources override only conflicting keys. The `plugin` and `instructions` arrays are **concatenated** across sources (deduplicated).

### Plugin loading locations

Plugins load from two sources:

1. **Local files**: TypeScript/JavaScript files in plugin directories
   - Project: `.opencode/plugins/*.{ts,js}` (also `plugin/*.{ts,js}`)
   - Global: `~/.config/opencode/plugins/*.{ts,js}` (also `~/.opencode/plugins/`)
2. **npm packages**: Specified in the `plugin` array in config, installed automatically via Bun at startup

Packages are cached in `~/.cache/opencode/node_modules/`.

**Load order**: Global config plugins, project config plugins, global directory plugins, project directory plugins.

### Example: npm plugin in config

```jsonc
// opencode.json
{
  "plugin": [
    "my-opencode-plugin@1.0.0",
    "file:///absolute/path/to/local-plugin.ts"
  ]
}
```

### Example: local plugin file

```typescript
// .opencode/plugins/my-hooks.ts
import type { Plugin } from "@opencode-ai/plugin"

export const MyPlugin: Plugin = async ({ client, project, $, directory, worktree }) => {
  return {
    "tool.execute.before": async (input, output) => {
      if (input.tool === "bash" && output.args.command?.includes("rm -rf")) {
        throw new Error("Blocked dangerous command")
      }
    },
    event: async ({ event }) => {
      if (event.type === "session.idle") {
        await $`osascript -e 'display notification "Done!" with title "opencode"'`
      }
    },
  }
}
```

### Example: custom tool plugin

```typescript
// .opencode/plugins/my-tools.ts
import type { Plugin } from "@opencode-ai/plugin"
import { tool } from "@opencode-ai/plugin"

export const MyToolPlugin: Plugin = async () => {
  return {
    tool: {
      query_db: tool({
        description: "Query the project database",
        args: {
          query: tool.schema.string().describe("SQL query to execute"),
        },
        async execute(args, context) {
          // context provides: sessionID, messageID, agent, directory, worktree, abort, metadata(), ask()
          return `Result: ${args.query}`
        },
      }),
    },
  }
}
```

Custom tools can also be defined as standalone files in `.opencode/tools/` or `~/.config/opencode/tools/`. The filename becomes the tool name. For files with multiple exports, naming follows `<filename>_<exportname>`.

### Plugin dependencies

Add a `package.json` to `.opencode/` with npm dependencies. OpenCode runs `bun install` at startup and caches packages. The `@opencode-ai/plugin` package version is validated and upgraded automatically if outdated.

### Plugin function context

The plugin function receives a context object:

| Field | Type | Description |
|-------|------|-------------|
| `client` | `ReturnType<typeof createOpencodeClient>` | OpenCode SDK client for API interactions |
| `project` | `Project` | Current project information |
| `directory` | `string` | Current working directory |
| `worktree` | `string` | Git worktree path |
| `serverUrl` | `URL` | Local OpenCode server URL |
| `$` | `BunShell` | Bun's shell API for executing commands |

---

## Hook Events

OpenCode hooks follow an input/output pattern. Most hooks receive an `input` object (read-only context) and an `output` object (mutable). The plugin mutates the output object in place; the final mutated output is what OpenCode uses. Hooks return `Promise<void>`.

When multiple plugins define the same hook, they execute sequentially in plugin load order. Each plugin sees mutations from previous plugins.

### `event`

Subscribes to all system bus events. Unlike other hooks, this is a single catch-all handler that receives every event; the plugin must filter by `event.type`.

**Signature:**

```typescript
event?: (input: { event: Event }) => Promise<void>
```

**Data received:**

- `event` - A discriminated union object. The `type` field identifies the event; `properties` contains event-specific data.

**Return:** `Promise<void>` (fire-and-forget; cannot affect flow)

**Available event types:**

| Event Type | Properties | Description |
|------------|------------|-------------|
| `command.executed` | `{ name: string, sessionID: string, arguments: string, messageID: string }` | Custom command executed |
| `file.edited` | `{ file: string }` | File was edited by the agent |
| `file.watcher.updated` | `{ file: string, event: "add" \| "change" \| "unlink" }` | File system watcher event |
| `ide.installed` | `{ ide: string }` | IDE extension installed |
| `installation.updated` | `{ version: string }` | OpenCode installation updated to new version |
| `installation.update-available` | `{ version: string }` | New version available for update |
| `lsp.client.diagnostics` | `{ serverID: string, path: string }` | LSP diagnostics received for a file |
| `lsp.updated` | `{}` | LSP server configuration updated |
| `mcp.tools.changed` | `{ server: string }` | MCP server tools changed |
| `mcp.browser.open.failed` | `{ mcpName: string, url: string }` | MCP browser failed to open |
| `message.updated` | `{ info: Message }` | Message added or changed |
| `message.removed` | `{ sessionID: string, messageID: string }` | Message deleted |
| `message.part.updated` | `{ part: Part }` | Message part updated |
| `message.part.delta` | `{ sessionID: string, messageID: string, partID: string, field: string, delta: string }` | Incremental text delta for a message part |
| `message.part.removed` | `{ sessionID: string, messageID: string, partID: string }` | Message part deleted |
| `permission.asked` | `PermissionRequest` | Permission requested from user |
| `permission.replied` | `{ sessionID: string, requestID: string, reply: "once" \| "always" \| "reject" }` | User responded to permission request |
| `project.updated` | `Project` | Project configuration changed |
| `pty.created` | `{ info: Pty }` | PTY session created |
| `pty.updated` | `{ info: Pty }` | PTY session updated |
| `pty.exited` | `{ id: string, exitCode: number }` | PTY process exited |
| `pty.deleted` | `{ id: string }` | PTY session deleted |
| `question.asked` | `QuestionRequest` | Question asked to user |
| `question.replied` | `{ sessionID: string, requestID: string, answers: Array<QuestionAnswer> }` | User answered question |
| `question.rejected` | `{ sessionID: string, requestID: string }` | Question was rejected |
| `server.connected` | `{}` | Connected to OpenCode server |
| `server.instance.disposed` | `{ directory: string }` | Server instance shut down |
| `global.disposed` | `{}` | Global state disposed |
| `session.created` | `{ info: Session }` | New session created |
| `session.updated` | `{ info: Session }` | Session updated |
| `session.deleted` | `{ info: Session }` | Session deleted |
| `session.status` | `{ sessionID: string, status: SessionStatus }` | Session status changed |
| `session.idle` | `{ sessionID: string }` | Session became idle (deprecated; prefer `session.status`) |
| `session.compacted` | `{ sessionID: string }` | Session context compacted |
| `session.diff` | `{ sessionID: string, diff: Array<FileDiff> }` | Session diff generated |
| `session.error` | `{ sessionID?: string, error?: ProviderAuthError \| UnknownError \| ... }` | Session error occurred |
| `todo.updated` | `{ sessionID: string, todos: Array<Todo> }` | Todo list updated |
| `tui.prompt.append` | `{ text: string }` | Text appended to TUI prompt |
| `tui.command.execute` | `{ command: string }` | TUI command executed (values: `session.list`, `session.new`, `session.share`, `session.interrupt`, `session.compact`, `session.page.up`, `session.page.down`, `session.line.up`, `session.line.down`, `session.half.page.up`, `session.half.page.down`, `session.first`, `session.last`, `prompt.clear`, `prompt.submit`, `agent.cycle`, or any string) |
| `tui.toast.show` | `{ title?: string, message: string, variant: "info" \| "success" \| "warning" \| "error", duration?: number }` | Toast notification shown |
| `tui.session.select` | `{ sessionID: string }` | Session selected in TUI |
| `vcs.branch.updated` | `{ branch?: string }` | Git branch changed |
| `worktree.ready` | `{ name: string, branch: string }` | Worktree ready |
| `worktree.failed` | `{ message: string }` | Worktree operation failed |

**Example:**

```typescript
event: async ({ event }) => {
  if (event.type === "session.idle") {
    console.log("Session idle:", event.properties.sessionID)
  }
}
```

**Gotchas:**

- The event hook receives **every** system event. Always filter by `event.type` using early returns or switch statements to avoid performance issues.
- Event properties vary by type. Use TypeScript discriminated unions or cast to specific event types for type safety.
- Events are fire-and-forget notifications. Returning void does not affect agent flow.

---

### `tool.execute.before`

Fires before a tool call executes. Can modify arguments or block execution.

**Signature:**

```typescript
"tool.execute.before"?: (
  input: { tool: string; sessionID: string; callID: string },
  output: { args: any }
) => Promise<void>
```

**Data received:**

- `input.tool` - Tool name (e.g., `"bash"`, `"edit"`, `"write"`, `"read"`, `"glob"`, `"grep"`, `"task"`, or custom tool IDs)
- `input.sessionID` - Current session ID
- `input.callID` - Unique tool call ID
- `output.args` - Mutable arguments object for the tool

**Return:** `Promise<void>`

**Flow impact:**

- Mutate `output.args` to change tool arguments before execution
- **Throw an error** to block the tool call entirely; the error message is returned to the agent

**Example:**

```typescript
"tool.execute.before": async (input, output) => {
  if (input.tool === "read" && output.args.filePath?.includes(".env")) {
    throw new Error("Reading .env files is not allowed")
  }
  if (input.tool === "bash") {
    output.args.command = sanitize(output.args.command)
  }
}
```

**Gotchas:**

- This hook fires for **all** tool calls including the `task` tool (subagent spawning). Filter by `input.tool` to target specific tools.
- The `args` object shape varies by tool. There is no compile-time type narrowing based on tool name.

---

### `tool.execute.after`

Fires after a tool call completes successfully. Can modify the output shown to the agent.

**Signature:**

```typescript
"tool.execute.after"?: (
  input: { tool: string; sessionID: string; callID: string; args: any },
  output: { title: string; output: string; metadata: any }
) => Promise<void>
```

**Data received:**

- `input.tool` - Tool name
- `input.sessionID` - Current session ID
- `input.callID` - Tool call ID
- `input.args` - The arguments that were passed to the tool (read-only at this point)
- `output.title` - Mutable display title for the tool result
- `output.output` - Mutable output string shown to the agent
- `output.metadata` - Mutable metadata object

**Return:** `Promise<void>`

**Flow impact:**

- Modify the tool's displayed output, title, or metadata
- Cannot block or retry after execution
- Useful for logging, formatting, or post-processing results

**Example:**

```typescript
"tool.execute.after": async (input, output) => {
  if (input.tool === "bash") {
    // Redact sensitive output
    output.output = output.output.replace(/API_KEY=\S+/g, "API_KEY=***")
  }
}
```

**Gotchas:**

- There is no `tool.execute.error` hook. This hook only fires on successful completion. Use the `event` hook to listen for `session.error` events if you need error handling.

---

### `tool.definition`

Fires when tool definitions are being assembled to send to the LLM. Allows modifying a tool's description and parameter schema.

**Signature:**

```typescript
"tool.definition"?: (
  input: { toolID: string },
  output: { description: string; parameters: any }
) => Promise<void>
```

**Data received:**

- `input.toolID` - The tool identifier
- `output.description` - Mutable tool description sent to the LLM
- `output.parameters` - Mutable JSON Schema parameters object sent to the LLM

**Return:** `Promise<void>`

**Flow impact:**

- Modify how tools are presented to the LLM
- Can change descriptions to guide model behavior or restrict parameter schemas

**Example:**

```typescript
"tool.definition": async (input, output) => {
  if (input.toolID === "bash") {
    output.description += "\nIMPORTANT: Never use sudo."
  }
}
```

---

### `shell.env`

Injects environment variables into all shell executions (bash tool, PTY sessions, and spawned processes).

**Signature:**

```typescript
"shell.env"?: (
  input: { cwd: string },
  output: { env: Record<string, string> }
) => Promise<void>
```

**Data received:**

- `input.cwd` - Current working directory
- `output.env` - Mutable environment variables object (starts empty)

**Return:** `Promise<void>`

**Flow impact:**

- Variables added to `output.env` are merged with `process.env` for all shell executions
- Affects the bash tool, PTY processes, and spawned subprocesses

**Example:**

```typescript
"shell.env": async (input, output) => {
  output.env.MY_API_KEY = "secret"
  output.env.PROJECT_ROOT = input.cwd
}
```

**Gotchas:**

- This hook fires on every shell invocation (bash tool, PTY creation, subprocesses). Keep it fast.
- Environment variables are merged in order: `process.env` -> caller-provided env -> `shell.env` hook output. Hook values win.

---

### `chat.message`

Fires when a new user message is being prepared for the LLM. Allows modifying the message content and parts.

**Signature:**

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

**Data received:**

- `input.sessionID` - Current session ID
- `input.agent` - Agent name (if applicable)
- `input.model` - Model provider and ID (if known)
- `input.messageID` - Message ID (if applicable)
- `input.variant` - Message variant (if applicable)
- `output.message` - Mutable user message object
- `output.parts` - Mutable message parts array

**Return:** `Promise<void>`

**Flow impact:**

- Modify the user message before it is sent to the LLM
- Add, modify, or remove message parts
- Cannot block the message

---

### `chat.params`

Modifies LLM request parameters dynamically before sending.

**Signature:**

```typescript
"chat.params"?: (
  input: {
    sessionID: string
    agent: string
    model: Model
    provider: ProviderContext
    message: UserMessage
  },
  output: { temperature: number; topP: number; topK: number; options: Record<string, any> }
) => Promise<void>
```

**Data received:**

- `input` - Session, agent, model, provider, and message context
- `output.temperature` - Mutable temperature setting
- `output.topP` - Mutable top-p setting
- `output.topK` - Mutable top-k setting
- `output.options` - Mutable additional provider-specific options

**Return:** `Promise<void>`

**Flow impact:**

- Adjust LLM sampling parameters per-request
- Changes affect only the current request

---

### `chat.headers`

Adds custom HTTP headers to LLM API requests.

**Signature:**

```typescript
"chat.headers"?: (
  input: {
    sessionID: string
    agent: string
    model: Model
    provider: ProviderContext
    message: UserMessage
  },
  output: { headers: Record<string, string> }
) => Promise<void>
```

**Data received:**

- `input` - Session, agent, model, provider, and message context
- `output.headers` - Mutable HTTP headers object

**Return:** `Promise<void>`

**Flow impact:**

- Inject custom headers into outgoing LLM API requests
- Useful for routing, tracking, audit trails, or custom authentication

---

### `permission.ask`

Fires when the permission system evaluates a tool call to "ask" (prompt the user). Allows programmatic auto-approval or denial.

**Signature:**

```typescript
"permission.ask"?: (
  input: Permission,
  output: { status: "ask" | "deny" | "allow" }
) => Promise<void>
```

**Data received:**

- `input` - Permission request details (tool name, patterns, metadata)
- `output.status` - Mutable status, defaults to `"ask"`

**Return:** `Promise<void>`

**Flow impact:**

- Set `output.status = "allow"` to auto-approve the permission
- Set `output.status = "deny"` to auto-reject (throws `RejectedError`)
- Keep `"ask"` to prompt the user as normal

**Example:**

```typescript
"permission.ask": async (input, output) => {
  if (input.permission === "bash") {
    const cmd = input.metadata?.command ?? ""
    if (cmd.startsWith("git ")) {
      output.status = "allow"
    }
  }
}
```

**Gotchas:**

- This hook **only fires** when the permission system evaluates a rule to `"ask"`. If the permission config already sets a tool to `"deny"` or `"allow"`, this hook never fires for that tool.
- To use this hook effectively, set permissions to `"ask"` in config, then use the hook for programmatic decisions.

---

### `command.execute.before`

Fires before a custom slash command executes.

**Signature:**

```typescript
"command.execute.before"?: (
  input: { command: string; sessionID: string; arguments: string },
  output: { parts: Part[] }
) => Promise<void>
```

**Data received:**

- `input.command` - Command name
- `input.sessionID` - Session ID
- `input.arguments` - Command arguments as string
- `output.parts` - Mutable parts array for command output

**Return:** `Promise<void>`

**Flow impact:**

- Modify command arguments or inject output parts before the command runs
- Can be used to augment commands with additional context

---

### `config`

Fires once at startup after configuration is loaded. Receives the full merged config object.

**Signature:**

```typescript
config?: (input: Config) => Promise<void>
```

**Data received:**

- `input` - The full merged OpenCode configuration object

**Return:** `Promise<void>`

**Flow impact:**

- Read-only access to configuration at startup
- Useful for plugin initialization based on config values

---

### `auth`

Provides custom authentication flows for LLM providers. This is a structured hook (not a simple function) used by built-in plugins like Codex auth, Copilot auth, and GitLab auth.

**Signature:**

```typescript
auth?: AuthHook
```

**AuthHook structure:**

```typescript
{
  provider: string            // Provider name this auth handles
  loader?: (auth, provider) => Promise<Record<string, any>>  // Custom auth loading
  methods: Array<OAuthMethod | ApiKeyMethod>  // Authentication methods
}
```

**Flow impact:**

- Registers custom authentication providers in the OpenCode auth system
- Supports OAuth flows (with auto-callback or code-based) and API key flows
- Methods can include interactive prompts (text inputs, select dropdowns) for gathering user credentials

---

### `experimental.chat.system.transform`

Modifies the system prompt before sending to the LLM. Experimental; may change.

**Signature:**

```typescript
"experimental.chat.system.transform"?: (
  input: { sessionID?: string; model: Model },
  output: { system: string[] }
) => Promise<void>
```

**Data received:**

- `input.sessionID` - Optional session ID
- `input.model` - Model information
- `output.system` - Mutable array of system prompt strings

**Return:** `Promise<void>`

**Flow impact:**

- Push strings to `output.system` to inject custom context into the system prompt
- If the array is emptied, OpenCode restores the original system prompt (safety mechanism)
- Useful for project-specific rules or persistent context

**Example:**

```typescript
"experimental.chat.system.transform": async (input, output) => {
  output.system.push(`<project-rules>
    Always run tests before committing.
    Never modify files in the vendor/ directory.
  </project-rules>`)
}
```

**Gotchas:**

- If you clear the `system` array entirely, OpenCode restores the original system prompt. This prevents accidental removal of essential system instructions.
- This hook is marked experimental and may change in future versions.

---

### `experimental.chat.messages.transform`

Modifies the entire message history before sending to the LLM.

**Signature:**

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

**Data received:**

- `output.messages` - Mutable array of message objects (each with `info` and `parts`)

**Return:** `Promise<void>`

**Flow impact:**

- Add, remove, or modify messages in the conversation history
- Use with extreme caution; can break conversation flow

**Gotchas:**

- Modifying message history can confuse the model or break tool call chains. Only use for well-understood transformations.

---

### `experimental.session.compacting`

Fires before session compaction starts. Allows customizing the compaction prompt.

**Signature:**

```typescript
"experimental.session.compacting"?: (
  input: { sessionID: string },
  output: { context: string[]; prompt?: string }
) => Promise<void>
```

**Data received:**

- `input.sessionID` - Session being compacted
- `output.context` - Mutable array of context strings appended to the default compaction prompt
- `output.prompt` - Optional; if set, **replaces** the default compaction prompt entirely (ignores `output.context`)

**Return:** `Promise<void>`

**Flow impact:**

- Push strings to `output.context` to add domain-specific context to the compaction summary
- Set `output.prompt` to completely replace the compaction prompt
- Critical for preserving important state across context compaction

**Example:**

```typescript
"experimental.session.compacting": async (input, output) => {
  output.context.push(`
## Important State
- Current deployment target: staging
- Database migration pending for users table
- Files being actively worked on: src/auth.ts, src/middleware.ts
`)
}
```

---

### `experimental.text.complete`

Fires when a text generation part completes. Allows post-processing of generated text.

**Signature:**

```typescript
"experimental.text.complete"?: (
  input: { sessionID: string; messageID: string; partID: string },
  output: { text: string }
) => Promise<void>
```

**Data received:**

- `input.sessionID`, `input.messageID`, `input.partID` - Identify the specific text part
- `output.text` - Mutable completed text (already trimmed)

**Return:** `Promise<void>`

**Flow impact:**

- Modify or post-process the LLM's generated text before it is finalized
- Can transform, filter, or enhance output

---

## Matcher System

OpenCode does **not** have a matcher/pattern system for hooks like Claude Code does. Instead, all hook filtering is done programmatically within the hook function itself:

- **Tool hooks** (`tool.execute.before`, `tool.execute.after`, `tool.definition`): Receive the tool name in `input.tool` or `input.toolID`. Filter with conditionals (e.g., `if (input.tool === "bash")`).
- **Event hook**: Receives all events. Filter by `event.type` (e.g., `if (event.type === "session.idle")`).
- **Permission hook**: Receives the full `Permission` object. Filter by `input.permission` (tool name) and `input.metadata`.

This means every hook function must implement its own filtering logic. There is no declarative way to scope a hook to specific tools or events in configuration.

---

## Known Gotchas and Workarounds

### 1. Event hook receives all events -- must filter

**Problem:** The `event` hook receives **every** system event. You must manually filter by `event.type`.

**Impact:** Easy to accidentally process the wrong events or create performance issues.

**Workaround:** Always use early returns:

```typescript
event: async ({ event }) => {
  if (event.type !== "session.idle") return
  // Process idle event
}
```

### 2. Permission hook only fires for "ask" decisions

**Problem:** The `permission.ask` hook only fires when the permission system evaluates a rule to `"ask"`. It cannot override `"deny"` or `"allow"` rules set in configuration.

**Impact:** Cannot use plugins to create exceptions to deny rules.

**Workaround:** Set permissions to `"ask"` in config, then use the hook for programmatic decisions:

```jsonc
// opencode.json
{
  "permission": {
    "bash": "ask"
  }
}
```

```typescript
"permission.ask": async (input, output) => {
  if (input.permission === "bash" && isSafe(input.metadata?.command)) {
    output.status = "allow"
  }
}
```

### 3. Hooks run sequentially -- mutation order matters

**Problem:** When multiple plugins define the same hook, they run in sequence. Each plugin sees the mutations from previous ones. This can lead to confusing interactions between plugins.

**Impact:** One plugin's modifications may conflict with another's expectations.

**Workaround:**
- Keep plugins focused and composable
- Use unique namespaces for custom data in metadata objects
- Document hook interactions in plugin READMEs

### 4. Context destructuring gotcha

**Problem:** The plugin function receives a **context object**, not individual parameters. A common mistake is treating the context as the client directly.

**Incorrect:**

```typescript
export const MyPlugin: Plugin = async (client) => {
  await client.session.prompt({ ... })  // FAILS: client is the context object
}
```

**Correct:**

```typescript
export const MyPlugin: Plugin = async ({ client }) => {
  await client.session.prompt({ ... })  // Works: destructured from context
}
```

### 5. State persistence across hooks

**Problem:** Plugin functions are initialized once at startup. Closure variables persist across all hook invocations within a session but are **not persisted across OpenCode restarts**.

**Impact:** Cannot rely on in-memory state surviving application restart.

**Workaround:** Use session-keyed Maps for per-session state and files or external storage for persistent state:

```typescript
const sessions = new Map<string, SessionState>()

export const MyPlugin: Plugin = async () => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.created") {
        sessions.set(event.properties.info.id, { filesModified: [] })
      }
      if (event.type === "session.deleted") {
        sessions.delete(event.properties.info.id)
      }
    },
  }
}
```

### 6. No hook for tool execution errors

**Problem:** There is no `tool.execute.error` hook. The `tool.execute.after` hook only fires on successful completion.

**Impact:** Cannot implement centralized error handling for tool failures in plugins.

**Workaround:**
- Use the `event` hook to listen for `session.error` events
- Check for error conditions in `tool.execute.after` by examining output content

### 7. Experimental hooks may change

**Problem:** Hooks prefixed with `experimental.` (system transform, messages transform, session compacting, text complete) are not considered stable API.

**Impact:** Plugin code using these hooks may break on upgrades.

**Workaround:** Pin your OpenCode version in CI/CD and test upgrades before deploying. Keep experimental hook usage isolated and easy to update.

### 8. System prompt safety mechanism

**Problem:** If an `experimental.chat.system.transform` hook empties the `output.system` array, OpenCode silently restores the original system prompt.

**Impact:** You cannot use this hook to remove the system prompt entirely.

**Workaround:** This is intentional. If you need to modify the system prompt, push additional strings rather than replacing the existing ones.

---

## Sources

- OpenCode GitHub repository: https://github.com/anomalyco/opencode
- OpenCode documentation: https://opencode.ai/docs
- Plugin documentation: https://opencode.ai/docs/plugins
- Custom tools documentation: https://opencode.ai/docs/custom-tools
- Configuration documentation: https://opencode.ai/docs/config
- Permissions documentation: https://opencode.ai/docs/permissions
- Plugin type definitions (source): https://github.com/anomalyco/opencode/tree/dev/packages/plugin/src/index.ts
- Tool helper (source): https://github.com/anomalyco/opencode/tree/dev/packages/plugin/src/tool.ts
- Plugin loader (source): https://github.com/anomalyco/opencode/tree/dev/packages/opencode/src/plugin/index.ts
- Bus event system (source): https://github.com/anomalyco/opencode/tree/dev/packages/opencode/src/bus
- npm package: https://www.npmjs.com/package/@opencode-ai/plugin
