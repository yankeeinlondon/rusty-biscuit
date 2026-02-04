# Roo Code Agentic CLI Hooks and Events

## Scope
Roo Code exposes multiple event surfaces that are relevant when you run it via the CLI or automate it:

1) CLI programmatic hooks (`ExtensionClient` in `@roo-code/cli`).
2) CLI structured output events (`--output-format json|stream-json`).
3) Roo Code extension API events (VS Code extension internal API, used by tools like Agent Maestro).

This document lists each hook, payload, return type expectations, and how those returns affect agent flow.

Sources:

- CLI event map and emission logic: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/src/agent/events.ts
- Message processing + transitions: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/src/agent/message-processor.ts
- Agent state model: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/src/agent/agent-state.ts
- CLI agent loop doc: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/docs/AGENT_LOOP.md
- CLI JSON event types: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/src/types/json-events.ts
- CLI JSON emitter: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/apps/cli/src/agent/json-event-emitter.ts
- Extension API + events: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/18c4d1ac410dd07cb950d11806bf17a1256f6f8a/src/extension/api.ts
- Event definitions: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/packages/types/src/events.ts
- Message model: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/packages/types/src/message.ts
- API type: https://raw.githubusercontent.com/RooCodeInc/Roo-Code/main/packages/types/src/api.ts
- Medium overview (mentions event hooks): https://r.jina.ai/http://medium.com/@justinduy/how-to-build-your-own-remote-code-agent-with-roocode-for-cloud-workflows-0db9027cff51

## 1) CLI programmatic hooks (ExtensionClient)
These are the core event hooks when you embed or extend the CLI agent loop. They are defined by `ClientEventMap` and emitted by the CLI message processor.

### Payload types used by the hooks

**ClineMessage** (abridged)

- `ts: number`
- `type: "ask" | "say"`
- `ask?: ClineAsk` (if `type: "ask"`)
- `say?: ClineSay` (if `type: "say"`)
- `text?: string`
- `partial?: boolean` (streaming)
- `reasoning?: string`
- plus optional fields like `images`, `progressStatus`, `contextCondense`, `contextTruncation`, etc.

**AgentStateInfo** (abridged)

- `state: AgentLoopState` (`no_task | running | streaming | waiting_for_input | idle | resumable`)
- `isWaitingForInput: boolean`
- `isRunning: boolean`
- `isStreaming: boolean`
- `currentAsk?: ClineAsk`
- `requiredAction: string` (e.g., `approve`, `answer`, `retry_or_new_task`)
- `lastMessage?: ClineMessage`
- `description: string`

### Event hooks and payloads

- `stateChange: AgentStateChangeEvent`
    - Payload: `{ previousState, currentState, isSignificantChange }`
    - Return: `void` (listener return value is ignored).
    - Flow impact: Observational. Use `currentState` to decide whether to call `approve`, `reject`, or `respond`.

- `message: ClineMessage`
    - Payload: Last message received in the current state update.
    - Return: `void`.
    - Flow impact: Observational only.

- `messageUpdated: ClineMessage`
    - Payload: Updated message (e.g., `partial` -> complete).
    - Return: `void`.
    - Flow impact: Observational only.

- `waitingForInput: WaitingForInputEvent`
    - Payload: `{ ask, stateInfo, message }`
    - Return: `void`.
    - Flow impact: Signals that the agent loop is blocked until you respond.

- `resumedRunning: void`
    - Payload: none.
    - Return: `void`.
    - Flow impact: Observational transition only.

- `streamingStarted: void`
- `streamingEnded: void`
    - Payload: none.
    - Return: `void`.
    - Flow impact: Observational transition only.

- `taskCompleted: TaskCompletedEvent`
    - Payload: `{ success, stateInfo, message? }`
    - Return: `void`.
    - Flow impact: Observational; the agent can still be waiting for feedback (`completion_result` is an ask type).

- `taskCleared: void`
    - Payload: none.
    - Return: `void`.
    - Flow impact: Observational; you can start a new task.

- `modeChanged: ModeChangedEvent`
    - Payload: `{ previousMode, currentMode }`
    - Return: `void`.
    - Flow impact: Observational.

- `error: Error`
    - Payload: Error object.
    - Return: `void`.
    - Flow impact: Observational; you may want to cancel or retry.

### Return types and how they affect flow
All `ExtensionClient` event listeners return `void`; any return value is ignored by the EventEmitter. To influence the agentic flow, you must **send explicit responses** through the client:

- `approve()` / `reject()` for approvals.
- `respond(text, images?)` for follow-ups.
- `newTask(text, images?)`, `cancelTask()`, `clearTask()` for task control.
- `resumeTask()` or `retryApiRequest()` for resumable or failed states.
- `continueTerminal()` / `abortTerminal()` for command output asks.

## 2) CLI structured output events (`--output-format json|stream-json`)
The CLI can emit JSON events to stdout. This is the simplest hook for shell automation or external tools.

### Event types (NDJSON in `stream-json` mode)
Each line is a JSON object with a `type` discriminator. Core event types:

- `system` (e.g., init message)
- `assistant` (assistant text)
- `user` (user echoes)
- `thinking` (reasoning content)
- `tool_use` (tool invocation)
- `tool_result` (tool output)
- `error`
- `result` (final task result)

Common fields:

- `id?: number` (message id / timestamp)
- `content?: string`
- `done?: boolean` (true on final chunk for a message)
- `subtype?: string` (tool/mcp/browser, etc)
- `tool_use?: { name: string, input?: object }`
- `tool_result?: { name: string, output?: string, error?: string }`
- `success?: boolean` and `cost?: { totalCost, inputTokens, outputTokens, cacheWrites, cacheReads }` on `result`

### `json` mode
Outputs a single JSON object at the end:

```json
{
  "type": "result",
  "success": true,
  "content": "...",
  "cost": { ... },
  "events": [ ... ]
}
```

### Return types and flow impact
The output stream is **read-only**. Consuming these events does not affect agent flow. To advance the agent, you still need to supply input (interactive mode) or use CLI options like `-y` (auto-approve) and send messages via the CLI or extension API.

## 3) Roo Code extension API events (internal VS Code API)
Roo Code exposes an internal EventEmitter API (`RooCodeAPI`). This is what the Medium article references when it mentions `onDidTaskEnd` and `onDidMessageReceived`; in current code, these map to `RooCodeEventName.TaskCompleted` and `RooCodeEventName.Message`.

### Event hooks and payloads
All events are defined by `RooCodeEventName` + `RooCodeEvents` tuples:

**Task lifecycle**

- `taskCreated`: `[taskId]`
- `taskStarted`: `[taskId]`
- `taskCompleted`: `[taskId, tokenUsage, toolUsage, { isSubtask: boolean }]`
- `taskAborted`: `[taskId]`
- `taskFocused`: `[taskId]`
- `taskUnfocused`: `[taskId]`
- `taskActive`: `[taskId]`
- `taskInteractive`: `[taskId]`
- `taskResumable`: `[taskId]`
- `taskIdle`: `[taskId]`

**Subtask lifecycle**

- `taskPaused`: `[taskId]`
- `taskUnpaused`: `[taskId]`
- `taskSpawned`: `[parentTaskId, childTaskId]`
- `taskDelegated`: `[parentTaskId, childTaskId]`
- `taskDelegationCompleted`: `[parentTaskId, childTaskId, completionResultSummary]`
- `taskDelegationResumed`: `[parentTaskId, childTaskId]`

**Task execution**

- `message`: `[{ taskId, action: "created" | "updated", message: ClineMessage }]`
- `taskModeSwitched`: `[taskId, mode]`
- `taskAskResponded`: `[taskId]`
- `taskUserMessage`: `[taskId]`
- `queuedMessagesUpdated`: `[taskId, queuedMessages[]]`

**Analytics and config**

- `taskTokenUsageUpdated`: `[taskId, tokenUsage, toolUsage]`
- `taskToolFailed`: `[taskId, toolName, error]`
- `modeChanged`: `[mode]`
- `providerProfileChanged`: `[{ name, provider }]`

### Return types and flow impact
Listener return values are ignored (standard Node `EventEmitter`). To affect flow you must call API methods such as:

- `sendMessage(text, images?)`
- `pressPrimaryButton()` / `pressSecondaryButton()`
- `startNewTask(...)`, `cancelTask(...)`, `resumeTask(...)`

## Gotchas and practical workarounds

1) **`message` only emits the last message in a state update.**
   - Source: CLI `MessageProcessor.emitNewMessageEvents`.
   - Workaround: track all messages via `stateChange` + `getMessages()` or use `messageUpdated` to process streaming/partial updates.

2) **`taskCompleted` only fires for a subset of idle asks.**
   - The helper `taskCompleted()` only recognizes `completion_result`, `api_req_failed`, and `mistake_limit_reached`.
   - Workaround: treat `AgentStateInfo.state === "idle"` as completion, and inspect `currentAsk` for other idle cases like `auto_approval_max_req_reached` or `resume_completed_task`.

3) **`waitingForInput` only fires on transitions.**
   - If you attach after the CLI is already waiting, you will not get the event.
   - Workaround: call `getAgentState()` immediately after constructing the client and handle `isWaitingForInput`.

4) **`completion_result` is an `ask` type.**
   - The agent can be both "completed" and still waiting for feedback.
   - Workaround: after `taskCompleted`, check `stateInfo.isWaitingForInput` and send a response or start a new task.

5) **`stream-json` is delta-based.**
   - Partial updates emit only the delta; the same `id` can appear multiple times and the final chunk has `done: true`.
   - Workaround: accumulate content per `id` (and separate `thinking` streams), or use `--output-format json` for final aggregation.

6) **Some internal message types are skipped in JSON output.**
   - The JSON emitter suppresses certain `say` types (e.g., `api_req_finished`, `checkpoint_saved`, `condense_context`, `sliding_window_truncation`).
   - Workaround: use `ExtensionClient` hooks or the extension API `message` event if you need raw message fidelity.

7) **Listener errors are not swallowed.**
   - The underlying Node EventEmitter does not guard against exceptions in listeners.
   - Workaround: wrap handlers in try/catch and report via your own logging.

8) **`taskCleared` is emitted only when you call `clearTask()`.**
   - External clears that skip `notifyTaskCleared()` will not emit this event.
   - Workaround: monitor `stateChange` and look for transitions to `no_task`.

## Notes on naming drift
The Medium article references `onDidTaskEnd` and `onDidMessageReceived`, but the current extension API exposes events via `RooCodeEventName` (e.g., `taskCompleted`, `message`). If you follow older examples, map those to the current event names.
