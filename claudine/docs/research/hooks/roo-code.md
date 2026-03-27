---
homepage: https://roocode.com
docs: https://docs.roocode.com
hooks: https://github.com/RooCodeInc/Roo-Code/tree/main/apps/cli/src/agent
---

# Roo Code hooks and events

## Home Page

https://roocode.com

## Documentation

https://docs.roocode.com

## Scope

Roo Code does **not** provide a shell-level hooks system (no equivalent to Claude Code's `PreToolUse` / `PostToolUse` JSON-on-stdin hooks). Instead, Roo Code exposes three programmatic event surfaces for automation and extension:

1. **CLI programmatic events** (`ExtensionClient` in the `roo` CLI).
2. **CLI structured output events** (`--output-format json|stream-json`).
3. **VS Code extension API events** (`RooCodeAPI` internal EventEmitter).

All three surfaces are **observational only** -- listener return values are ignored. To influence the agentic flow, you must send explicit responses through the client API (approve, reject, respond, etc.).

Additionally, Roo Code provides two extensibility mechanisms that are not event hooks but can alter behavior:

- **Custom tools** (`.roo/tools/` or `~/.roo/tools/`): TypeScript/JavaScript files that define new tools the agent can invoke.
- **Custom instructions** (`.roo/rules/` or `~/.roo/rules/`): Markdown/text files that shape agent behavior via system prompt injection.
- **Auto-approve settings**: Per-tool and per-category permission configuration that controls which actions proceed without user confirmation.

## Configuration

Roo Code has no hooks configuration file. The event surfaces are accessed programmatically (via the Node.js `ExtensionClient` API or by parsing CLI stdout) rather than through a declarative configuration file.

### Custom instructions (behavioral customization)

Custom instructions are the closest equivalent to Claude Code's settings-based configuration. They are loaded in this order, with later entries taking precedence:

1. Global rules from `~/.roo/rules/` (all modes) or `~/.roo/rules-{modeSlug}/` (mode-specific)
2. Workspace rules from `.roo/rules/` (all modes) or `.roo/rules-{modeSlug}/` (mode-specific)
3. Legacy single-file fallbacks: `.roorules` or `.roorules-{modeSlug}` (used only if directories are absent or empty)

Files are read recursively in alphabetical order. Temporary/cache files (`.DS_Store`, `*.bak`, `*.log`) are automatically excluded.

Optional `AGENTS.md` (or `AGENT.md`) at workspace root provides agent-specific guidelines, loaded after mode-specific rules but before generic rules. Controlled by `roo-cline.useAgentRules` setting (default: `true`).

### Custom tools (tool-level extensibility)

Custom tools live in:

| Location | Scope | Override behavior |
|----------|-------|-------------------|
| `~/.roo/tools/` | Global (all projects) | Base layer |
| `.roo/tools/` | Project-specific | Overrides global tools with same name |

Each tool is a `.ts` or `.js` file:

```typescript
import { parametersSchema as z, defineCustomTool } from "@roo-code/types"

export default defineCustomTool({
  name: "tool_name",
  description: "What the tool does",
  parameters: z.object({ /* Zod schema */ }),
  async execute(args, context) {
    // implementation
    return "string result"  // Must return a string
  }
})
```

Custom tools are **auto-approved** when enabled -- they run without user confirmation.

### Auto-approve settings

Auto-approve controls which operations proceed without user confirmation. Toggled via `Cmd+Alt+A` (macOS) / `Ctrl+Alt+A` (Windows/Linux).

| Permission | Controls | Key details |
|------------|----------|-------------|
| Read operations | File/directory access | Workspace-only by default |
| Write operations | File creation/modification | Protected files (`.roo/`, `.rooignore`) bypass |
| Command execution | Terminal commands | Allowlist/denylist with longest-prefix matching |
| Browser usage | Headless browser | Single toggle |
| MCP tools | Third-party MCP tools | Dual-permission: global toggle + per-tool "Always allow" |
| Mode switching | Mode changes | Covers switching and creating new modes |
| Subtasks | Task creation/completion | Single toggle |
| Follow-up questions | Response automation | Configurable timeout (1-300s, default 60s) |

### CLI flags for automation

| Flag | Purpose |
|------|---------|
| `--print` / `-p` | Non-interactive mode (no TUI) |
| `--output-format <fmt>` | `text`, `json`, or `stream-json` |
| `--require-approval` / `-a` | Manual confirmation for all actions |
| `--oneshot` | Terminate after task completion |
| `--stdin-prompt-stream` | Batch process prompts from stdin (one per line) |
| `--debug` / `-d` | Verbose logging to `~/.roo/cli-debug.log` |
| `--mode <mode>` | `code`, `architect`, `ask`, `debug` |

## Hook Events

Roo Code's "hook events" are Node.js EventEmitter events, not shell-based hooks. They are organized into three surfaces described below. None of these events support matchers, shell commands, or exit-code-based flow control.

### Surface 1: CLI programmatic events (`ExtensionClient`)

These events are emitted by the `MessageProcessor` when you embed or extend the CLI agent loop. Defined by the `ClientEventMap` interface.

#### Shared payload types

**ClineMessage** (abridged):

- `ts: number` -- message timestamp
- `type: "ask" | "say"` -- message classification
- `ask?: ClineAsk` -- ask subtype (if `type` is `"ask"`). Values: `followup`, `command`, `command_output`, `completion_result`, `tool`, `api_req_failed`, `resume_task`, `resume_completed_task`, `mistake_limit_reached`, `use_mcp_server`, `auto_approval_max_req_reached`
- `say?: ClineSay` -- say subtype (if `type` is `"say"`). 28 values covering API operations, task progress, errors, MCP integration, context management.
- `text?: string` -- message content
- `partial?: boolean` -- `true` while streaming
- `reasoning?: string` -- model reasoning content
- `images?: string[]`, `progressStatus?: string`, `contextCondense?: ContextCondense`, `contextTruncation?: ContextTruncation` -- optional fields

**AgentStateInfo**:

- `state: AgentLoopState` -- one of `no_task`, `running`, `streaming`, `waiting_for_input`, `idle`, `resumable`
- `isWaitingForInput: boolean`
- `isRunning: boolean`
- `isStreaming: boolean`
- `currentAsk?: ClineAsk`
- `requiredAction: RequiredAction` -- one of `none`, `approve`, `answer`, `retry_or_new_task`, `proceed_or_new_task`, `start_task`, `resume_or_abandon`, `start_new_task`, `continue_or_abort`
- `lastMessageTs?: number`
- `lastMessage?: ClineMessage`
- `description: string`

#### Event: `stateChange`

**Description:** Fires on any state transition in the agent loop. The `MessageProcessor` compares previous and current states and emits this event when differences are detected.

**Event payload:**

```typescript
interface AgentStateChangeEvent {
  previousState: AgentStateInfo
  currentState: AgentStateInfo
  isSignificantChange: boolean
}
```

**Event response:** `void` (return value ignored).

**Flow impact:** Observational. Use `currentState` fields to decide whether to call `approve()`, `reject()`, or `respond()` through the client.

**Gotchas:** Emission frequency depends on the `emitAllStateChanges` option (default: `true`). When set to `false`, only "significant" changes emit, which may cause you to miss intermediate states.

#### Event: `message`

**Description:** Fires when a new message arrives in the conversation. Only the last message in the current state update is emitted, not all messages.

**Event payload:** `ClineMessage` (the most recent message).

**Event response:** `void`.

**Flow impact:** Observational only.

**Gotchas:** If multiple messages arrive in a single state update, only the last one triggers this event. Use `stateChange` to track the full message array, or use `messageUpdated` for streaming updates.

#### Event: `messageUpdated`

**Description:** Fires when an existing message is modified (e.g., `partial: true` becomes `partial: false` when streaming completes).

**Event payload:** `ClineMessage` (the updated message).

**Event response:** `void`.

**Flow impact:** Observational only.

**Gotchas:** None specific.

#### Event: `waitingForInput`

**Description:** Fires when the agent transitions to a state that requires user input. This is the primary event for building interactive automation.

**Event payload:**

```typescript
interface WaitingForInputEvent {
  ask: ClineAsk         // The type of ask (tool, command, followup, etc.)
  stateInfo: AgentStateInfo
  message: ClineMessage  // The ask message
}
```

**Event response:** `void`.

**Flow impact:** Signals that the agent loop is blocked until you respond. You must call one of the client response methods (`approve()`, `reject()`, `respond()`, etc.) to unblock.

**Gotchas:** Only fires on transitions. If you attach a listener after the CLI is already waiting, you will not receive the event. **Workaround:** call `getAgentState()` immediately after constructing the client and check `isWaitingForInput`.

#### Event: `resumedRunning`

**Description:** Fires when the agent resumes execution after being in a waiting or idle state.

**Event payload:** `void` (no data).

**Event response:** `void`.

**Flow impact:** Observational transition only.

**Gotchas:** None specific.

#### Event: `streamingStarted`

**Description:** Fires when the agent begins streaming a response from the LLM.

**Event payload:** `void`.

**Event response:** `void`.

**Flow impact:** Observational transition only.

**Gotchas:** None specific.

#### Event: `streamingEnded`

**Description:** Fires when streaming from the LLM completes.

**Event payload:** `void`.

**Event response:** `void`.

**Flow impact:** Observational transition only.

**Gotchas:** None specific.

#### Event: `taskCompleted`

**Description:** Fires when the agent determines a task has completed. Triggered by the `taskCompleted()` helper in the message processor.

**Event payload:**

```typescript
interface TaskCompletedEvent {
  success: boolean
  stateInfo: AgentStateInfo
  message?: ClineMessage
}
```

**Event response:** `void`.

**Flow impact:** Observational, but the agent can still be waiting for feedback because `completion_result` is an `ask` type.

**Gotchas:** Only fires for a subset of idle asks: `completion_result`, `api_req_failed`, and `mistake_limit_reached`. Other idle asks like `auto_approval_max_req_reached` or `resume_completed_task` do not trigger this event. **Workaround:** treat `AgentStateInfo.state === "idle"` as completion and inspect `currentAsk` for the specific idle reason.

#### Event: `taskCleared`

**Description:** Fires when the current task is explicitly cleared.

**Event payload:** `void`.

**Event response:** `void`.

**Flow impact:** Observational; you can start a new task after this event.

**Gotchas:** Only emitted when `clearTask()` is called through the client. External clears that skip `notifyTaskCleared()` will not emit this event. **Workaround:** monitor `stateChange` and look for transitions where `state` becomes `no_task`.

#### Event: `modeChanged`

**Description:** Fires when the operational mode changes (e.g., from `code` to `architect`).

**Event payload:**

```typescript
interface ModeChangedEvent {
  previousMode: string | undefined
  currentMode: string
}
```

**Event response:** `void`.

**Flow impact:** Observational.

**Gotchas:** None specific.

#### Event: `error`

**Description:** Fires when an error occurs during message processing.

**Event payload:** `Error` (standard JavaScript Error object).

**Event response:** `void`.

**Flow impact:** Observational. You may want to cancel or retry depending on the error.

**Gotchas:** The underlying Node EventEmitter does not guard against exceptions in listeners. If your listener throws, it will propagate and potentially crash the process. **Workaround:** wrap all event handlers in try/catch.

#### Controlling flow from event listeners

All `ExtensionClient` event listeners return `void`; any return value is ignored by the EventEmitter. To influence the agentic flow, you must send explicit responses through the client:

| Method | Purpose |
|--------|---------|
| `approve()` | Approve a tool/command/MCP ask |
| `reject()` | Reject a tool/command/MCP ask |
| `respond(text, images?)` | Answer a followup or completion_result ask |
| `newTask(text, images?)` | Start a new task |
| `cancelTask()` | Cancel the current task |
| `clearTask()` | Clear the current task |
| `resumeTask()` | Resume a paused/resumable task |
| `retryApiRequest()` | Retry after API failure |
| `continueTerminal()` | Continue after command output |
| `abortTerminal()` | Abort a running command |

### Surface 2: CLI structured output events (`--output-format`)

The CLI can emit JSON events to stdout. This is the simplest integration surface for shell automation and external tools.

#### `stream-json` mode (NDJSON)

Each line is a JSON object with a `type` discriminator. Event types:

| Type | Description | Key fields |
|------|-------------|------------|
| `system` | Init/lifecycle messages | `subtype: "init"` |
| `assistant` | Assistant text responses | `id`, `content`, `done` |
| `user` | Echoed user input | `id`, `content`, `done` |
| `thinking` | Reasoning/thinking content | `id`, `content`, `done` |
| `tool_use` | Tool invocations | `tool_use: { name, input? }`, `subtype: "tool" \| "command" \| "mcp"` |
| `tool_result` | Tool execution outcomes | `tool_result: { name, output?, error? }`, `subtype: "mcp"` (optional) |
| `error` | Error notifications | `content` |
| `result` | Final task completion | `success`, `content`, `done`, `cost: { totalCost?, inputTokens?, outputTokens?, cacheWrites?, cacheReads? }` |

Common fields on all events:

- `type: string` -- event discriminator (required)
- `id?: number` -- message identifier/timestamp
- `content?: string` -- text payload
- `done?: boolean` -- `true` on the final chunk for a message
- `subtype?: string` -- additional categorization

#### `json` mode

Outputs a single JSON object at task completion:

```json
{
  "type": "result",
  "success": true,
  "content": "...",
  "cost": { "totalCost": 0.05, "inputTokens": 1200, "outputTokens": 800 },
  "events": [ /* all accumulated events */ ]
}
```

#### Suppressed message types

The JSON emitter skips certain internal `say` types:

- `api_req_finished`, `api_req_retried`, `api_req_retry_delayed`
- `api_req_rate_limit_wait`, `api_req_deleted`
- `checkpoint_saved`
- `condense_context`, `condense_context_error`
- `sliding_window_truncation`

#### Flow impact

The output stream is **read-only**. Consuming these events does not affect agent flow. To advance the agent in non-interactive mode, use CLI flags like `--require-approval` (for interactive) or default auto-approve (for unattended).

### Surface 3: VS Code extension API events (`RooCodeAPI`)

`RooCodeAPI` extends `EventEmitter<RooCodeAPIEvents>` and is the primary integration point for other VS Code extensions. Events are defined by `RooCodeEventName` and `RooCodeEvents` tuples in `@roo-code/types`.

#### Task lifecycle events

| Event | Payload | Description |
|-------|---------|-------------|
| `taskCreated` | `[taskId: string]` | New task created |
| `taskStarted` | `[taskId: string]` | Task execution begins |
| `taskCompleted` | `[taskId: string, tokenUsage, toolUsage, { isSubtask: boolean }]` | Task finished |
| `taskAborted` | `[taskId: string]` | Task cancelled |
| `taskFocused` | `[taskId: string]` | Task gained UI focus |
| `taskUnfocused` | `[taskId: string]` | Task lost UI focus |
| `taskActive` | `[taskId: string]` | Task is actively running |
| `taskInteractive` | `[taskId: string]` | Task requires user interaction |
| `taskResumable` | `[taskId: string]` | Task is paused but resumable |
| `taskIdle` | `[taskId: string]` | Task is idle |

#### Subtask lifecycle events

| Event | Payload | Description |
|-------|---------|-------------|
| `taskPaused` | `[taskId: string]` | Parent task paused for subtask |
| `taskUnpaused` | `[taskId: string]` | Parent task resumed after subtask |
| `taskSpawned` | `[parentTaskId, childTaskId]` | Subtask created |
| `taskDelegated` | `[parentTaskId, childTaskId]` | Task delegated to subtask |
| `taskDelegationCompleted` | `[parentTaskId, childTaskId, completionResultSummary]` | Delegation finished |
| `taskDelegationResumed` | `[parentTaskId, childTaskId]` | Delegation control returned |

#### Task execution events

| Event | Payload | Description |
|-------|---------|-------------|
| `message` | `[{ taskId, action: "created" \| "updated", message: ClineMessage }]` | Message created or updated |
| `taskModeSwitched` | `[taskId, mode]` | Operational mode changed within task |
| `taskAskResponded` | `[taskId]` | User responded to an ask |
| `taskUserMessage` | `[taskId]` | User sent a message |
| `queuedMessagesUpdated` | `[taskId, queuedMessages[]]` | Message queue changed |

#### Analytics and configuration events

| Event | Payload | Description |
|-------|---------|-------------|
| `taskTokenUsageUpdated` | `[taskId, tokenUsage, toolUsage]` | Token consumption updated |
| `taskToolFailed` | `[taskId, toolName, error]` | Tool execution failed |
| `modeChanged` | `[mode: string]` | Global mode changed |
| `providerProfileChanged` | `[{ name, provider }]` | API provider profile switched |

#### Query response events

| Event | Payload | Description |
|-------|---------|-------------|
| `commandsResponse` | `[command[]]` | Response to commands query (name, source, filePath, description, argumentHint) |
| `modesResponse` | `[{ slug, name }[]]` | Response to modes query |
| `modelsResponse` | `[Record<string, ModelInfo>]` | Response to models query |

#### Eval events

| Event | Payload | Description |
|-------|---------|-------------|
| `evalPass` | `undefined` | Evaluation passed (requires taskId context) |
| `evalFail` | `undefined` | Evaluation failed (requires taskId context) |

#### Controlling flow from extension API

Listener return values are ignored (standard Node `EventEmitter`). To affect flow, call API methods:

| Method | Purpose |
|--------|---------|
| `sendMessage(text, images?)` | Send a user message to the active task |
| `pressPrimaryButton()` | Approve / continue |
| `pressSecondaryButton()` | Reject / cancel |
| `startNewTask({ task, images?, configuration?, newTab? })` | Start a new task |
| `cancelCurrentTask()` | Cancel the running task |
| `clearCurrentTask()` | Clear the task stack |
| `resumeTask(taskId)` | Resume a paused task |
| `deleteQueuedMessage(messageId)` | Remove a queued message |

## Matcher System

Roo Code does **not** have a matcher system. Unlike Claude Code (which uses regex matchers to filter hooks by tool name, session source, or notification type), Roo Code events fire unconditionally. All filtering must be performed in your event listener code by inspecting the event payload.

For example, to react only to tool-related asks in the CLI:

```typescript
client.on("waitingForInput", (event) => {
  if (event.ask === "tool") {
    // Handle tool approval
    client.approve()
  }
})
```

For the extension API, filter by inspecting the `ClineMessage` payload:

```typescript
api.on("message", ([{ taskId, action, message }]) => {
  if (message.type === "ask" && message.ask === "command") {
    // Handle command approval for this task
  }
})
```

## Gotchas

### 1. `message` only emits the last message in a state update

The CLI `MessageProcessor.emitNewMessageEvents` only emits the final message, not all messages that arrived in the update batch.

**Workaround:** Track all messages via `stateChange` events (which include the full state including message history), or use `messageUpdated` to process streaming/partial updates individually.

### 2. `taskCompleted` only fires for a subset of idle asks

The `taskCompleted()` helper only recognizes `completion_result`, `api_req_failed`, and `mistake_limit_reached` as completion triggers.

**Workaround:** Treat `AgentStateInfo.state === "idle"` as completion, and inspect `currentAsk` for other idle cases like `auto_approval_max_req_reached` or `resume_completed_task`.

### 3. `waitingForInput` only fires on transitions

If you attach your listener after the CLI is already in a waiting state, you will never receive the event.

**Workaround:** Call `getAgentState()` immediately after constructing the client and handle `isWaitingForInput` before relying on the event.

### 4. `completion_result` is an `ask` type

The agent can be both "completed" and still waiting for user feedback. This is confusing because `taskCompleted` fires but the agent is not truly done until you respond.

**Workaround:** After receiving `taskCompleted`, check `stateInfo.isWaitingForInput` and send a response or start a new task to fully close the loop.

### 5. `stream-json` is delta-based

Partial updates emit only the delta content. The same `id` can appear multiple times, and only the final chunk has `done: true`.

**Workaround:** Accumulate content per `id` (maintaining separate buffers for `thinking` and `assistant` streams), or use `--output-format json` for a single aggregated result at the end.

### 6. Some internal message types are suppressed in JSON output

The JSON emitter skips certain `say` types including `api_req_finished`, `checkpoint_saved`, `condense_context`, and `sliding_window_truncation`.

**Workaround:** Use `ExtensionClient` events or the extension API `message` event if you need full message fidelity.

### 7. Listener errors are not swallowed

The underlying Node EventEmitter does not guard against exceptions in listeners. A thrown error in any listener will propagate up the call stack.

**Workaround:** Wrap all event handlers in try/catch and report errors via your own logging mechanism.

### 8. `taskCleared` is emitted only when you call `clearTask()`

External clears that skip `notifyTaskCleared()` will not emit this event.

**Workaround:** Monitor `stateChange` and look for transitions where `state` becomes `no_task`.

### 9. No shell-level hook interception

Unlike Claude Code, Gemini CLI, or Kimi Code, Roo Code does not support shell-script-based hooks that can intercept, modify, or block tool calls via stdin/stdout JSON and exit codes. All automation must be done programmatically through the Node.js API or by parsing stdout in `stream-json` mode.

**Workaround:** For CI/CD integration, use `--output-format stream-json --print --oneshot` and parse the NDJSON output. For programmatic control, use the `ExtensionClient` API or build a VS Code extension that consumes `RooCodeAPI` events.

### 10. Naming drift from older documentation

The Medium article and some older references use `onDidTaskEnd` and `onDidMessageReceived`, but the current extension API uses `RooCodeEventName` values (`taskCompleted`, `message`). Map old names to current event names when following older tutorials.

## Sources

- Roo Code homepage: https://roocode.com
- Roo Code documentation: https://docs.roocode.com
- GitHub repository: https://github.com/RooCodeInc/Roo-Code
- CLI event map and emission logic: https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/events.ts
- CLI message processor: https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/message-processor.ts
- CLI agent state model: https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/agent-state.ts
- CLI agent loop documentation: https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/docs/AGENT_LOOP.md
- CLI JSON event types: https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/types/json-events.ts
- CLI JSON emitter: https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/src/agent/json-event-emitter.ts
- Extension API source: https://github.com/RooCodeInc/Roo-Code/blob/main/src/extension/api.ts
- Event definitions (`RooCodeEventName`): https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/events.ts
- Message model (`ClineMessage`, `ClineAsk`, `ClineSay`): https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/message.ts
- API type (`RooCodeAPI`): https://github.com/RooCodeInc/Roo-Code/blob/main/packages/types/src/api.ts
- Custom instructions docs: https://docs.roocode.com/features/custom-instructions
- Custom tools docs: https://docs.roocode.com/features/experimental/custom-tools
- Auto-approve docs: https://docs.roocode.com/features/auto-approving-actions
- Shell integration docs: https://docs.roocode.com/features/shell-integration
- CLI README: https://github.com/RooCodeInc/Roo-Code/blob/main/apps/cli/README.md
- External API deep dive: https://deepwiki.com/roovetgit/roo-code/2.3-external-api-interface
- Medium article (remote agent): https://medium.com/@justinduy/how-to-build-your-own-remote-code-agent-with-roocode-for-cloud-workflows-0db9027cff51
