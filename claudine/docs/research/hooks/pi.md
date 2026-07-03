---
$schema: ./_schema.yaml
created: "2026-07-03"
last_updated: "2026-07-03"
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://pi.dev/
docs: https://pi.dev/docs/latest
hooks_docs: https://github.com/earendil-works/pi/tree/main/packages/coding-agent/docs/extensions.md

hooks:
  - native_event: resources_discover
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "type, cwd, reason"
    return_contract: "{ skillPaths?: string[], promptPaths?: string[], themePaths?: string[] } — paths are merged across extensions; no cancel/block semantics."
    notes: "Fired after session_start to let extensions declare additional skill/prompt/theme resource directories. Reason is startup or reload."
  - native_event: session_start
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "type, reason, previousSessionFile?"
    return_contract: "No return value observed."
    notes: "Fired when a session is started, loaded, resumed, forked, or reloaded. Reason ∈ {startup, reload, new, resume, fork}."
  - native_event: session_before_switch
    claudine_event: unknown
    timing: pre
    blocking: true
    payload_schema: "type, reason, targetSessionFile?"
    return_contract: "{ cancel?: boolean } — returning cancel:true prevents the switch."
    notes: "Fired before switching to another session (reason new|resume). First handler returning cancel:true wins."
  - native_event: session_before_fork
    claudine_event: unknown
    timing: pre
    blocking: true
    payload_schema: "type, entryId, position"
    return_contract: "{ cancel?: boolean, skipConversationRestore?: boolean }"
    notes: "Fired before forking a session. position ∈ {before, at}."
  - native_event: session_before_compact
    claudine_event: notification
    timing: pre
    blocking: true
    payload_schema: "type, preparation, branchEntries, customInstructions?, signal"
    return_contract: "{ cancel?: boolean, compaction?: CompactionResult } — cancel prevents compaction; compaction replaces the default compaction result."
    notes: "Fired before context compaction. Extensions can customize or abort compaction."
  - native_event: session_compact
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "type, compactionEntry, fromExtension"
    return_contract: "No return value."
    notes: "Fired after context compaction."
  - native_event: session_shutdown
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "type, reason, targetSessionFile?"
    return_contract: "No return value."
    notes: "Fired before an extension runtime is torn down due to quit, reload, new, resume, or fork."
  - native_event: session_before_tree
    claudine_event: unknown
    timing: pre
    blocking: true
    payload_schema: "type, preparation, signal"
    return_contract: "{ cancel?: boolean, summary?, customInstructions?, replaceInstructions?, label? }"
    notes: "Fired before navigating the session tree (/tree)."
  - native_event: session_tree
    claudine_event: unknown
    timing: post
    blocking: false
    payload_schema: "type, newLeafId, oldLeafId, summaryEntry?, fromExtension?"
    return_contract: "No return value."
    notes: "Fired after navigating the session tree."
  - native_event: context
    claudine_event: prompt
    timing: pre
    blocking: false
    payload_schema: "type, messages"
    return_contract: "{ messages?: AgentMessage[] } — returning messages replaces the message list passed to the LLM."
    notes: "Fired before each LLM call. Allows mutation/replacement of the full message context."
  - native_event: before_provider_request
    claudine_event: prompt
    timing: pre
    blocking: false
    payload_schema: "type, payload"
    return_contract: "unknown — handler return replaces the provider request payload."
    notes: "Fired before a provider request is sent. Return value becomes the new payload."
  - native_event: after_provider_response
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "type, status, headers"
    return_contract: "No return value."
    notes: "Fired after a provider response is received and before the response stream is consumed."
  - native_event: before_agent_start
    claudine_event: prompt
    timing: pre
    blocking: false
    payload_schema: "type, prompt, images?, systemPrompt, systemPromptOptions"
    return_contract: "{ message?, systemPrompt? } — messages are collected; systemPrompt values are chained (last wins)."
    notes: "Fired after user submits a prompt but before the agent loop begins."
  - native_event: agent_start
    claudine_event: start
    timing: pre
    blocking: false
    payload_schema: "type"
    return_contract: "No return value."
    notes: "Fired when an agent loop starts."
  - native_event: agent_end
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "type, messages"
    return_contract: "No return value."
    notes: "Fired when an agent loop ends."
  - native_event: turn_start
    claudine_event: loop
    timing: pre
    blocking: false
    payload_schema: "type, turnIndex, timestamp"
    return_contract: "No return value."
    notes: "Fired at the start of each assistant turn."
  - native_event: turn_end
    claudine_event: loop
    timing: post
    blocking: false
    payload_schema: "type, turnIndex, message, toolResults"
    return_contract: "No return value."
    notes: "Fired at the end of each assistant turn."
  - native_event: message_start
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "type, message"
    return_contract: "No return value."
    notes: "Fired when any message starts (user, assistant, or toolResult)."
  - native_event: message_update
    claudine_event: notification
    timing: around
    blocking: false
    payload_schema: "type, message, assistantMessageEvent"
    return_contract: "No return value."
    notes: "Fired during assistant message streaming with token-by-token updates."
  - native_event: message_end
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "type, message"
    return_contract: "{ message?: AgentMessage } — replacement must keep the original role."
    notes: "Fired when a message ends. Handler can replace the finalized message in-place."
  - native_event: tool_execution_start
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "type, toolCallId, toolName, args"
    return_contract: "No return value."
    notes: "Observation-only event fired when the executor starts running a tool."
  - native_event: tool_execution_update
    claudine_event: tool_result
    timing: around
    blocking: false
    payload_schema: "type, toolCallId, toolName, args, partialResult"
    return_contract: "No return value."
    notes: "Observation-only event for streaming/partial tool output."
  - native_event: tool_execution_end
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "type, toolCallId, toolName, result, isError"
    return_contract: "No return value."
    notes: "Observation-only event fired when tool execution finishes."
  - native_event: model_select
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "type, model, previousModel, source"
    return_contract: "No return value."
    notes: "Fired when a new model is selected. source ∈ {set, cycle, restore}."
  - native_event: thinking_level_select
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "type, level, previousLevel"
    return_contract: "No return value."
    notes: "Fired when thinking level changes."
  - native_event: user_bash
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "type, command, excludeFromContext, cwd"
    return_contract: "{ operations?: BashOperations, result?: BashResult } — returning result fully replaces execution."
    notes: "Fired when user executes a bash command via ! or !! prefix. Extension can override execution."
  - native_event: input
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "type, text, images?, source"
    return_contract: "{ action: continue | transform | handled, text?, images? } — handled short-circuits; transform mutates the prompt."
    notes: "Fired when user input is received before agent processing. source ∈ {interactive, rpc, extension}."
  - native_event: tool_call
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "type, toolCallId, toolName, input"
    return_contract: "{ block?: boolean, reason?: string } — block:true cancels the tool call. Input is mutated in place to modify arguments."
    notes: "Fired before a tool executes. Built-in tools have typed inputs (bash, read, edit, write, grep, find, ls); custom tools use Record<string, unknown>."
  - native_event: tool_result
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "type, toolCallId, toolName, input, content, isError, details"
    return_contract: "{ content?, details?, isError? } — returned fields replace the values seen by the agent."
    notes: "Fired after a tool executes. Allows modification of result content, details, and error flag."

config_files:
  - os: macos
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    notes: "Global user settings. The `extensions` array lists explicit extension file/dir paths; `packages` array lists npm/git packages to load."
  - os: linux
    scope: user
    path: "~/.pi/agent/settings.json"
    format: json
    notes: "Global user settings. Same as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\settings.json"
    format: json
    notes: "Global user settings. Same content as macOS/Linux."
  - os: macos
    scope: repo
    path: ".pi/settings.json"
    format: json
    notes: "Project-local settings; overrides global settings. Subject to project trust prompt in interactive mode."
  - os: linux
    scope: repo
    path: ".pi/settings.json"
    format: json
    notes: "Project-local settings; overrides global settings."
  - os: windows
    scope: repo
    path: ".pi\\settings.json"
    format: json
    notes: "Project-local settings; overrides global settings."
  - os: macos
    scope: repo
    path: ".pi/extensions/"
    format: other
    notes: "Project-local auto-discovered extensions: direct .ts/.js files, subdirs with index.ts/js, or package.json with `pi.extensions`."
  - os: linux
    scope: repo
    path: ".pi/extensions/"
    format: other
    notes: "Project-local auto-discovered extensions."
  - os: windows
    scope: repo
    path: ".pi\\extensions\\"
    format: other
    notes: "Project-local auto-discovered extensions."
  - os: macos
    scope: user
    path: "~/.pi/agent/extensions/"
    format: other
    notes: "Global auto-discovered extensions. Same discovery rules as project dir."
  - os: linux
    scope: user
    path: "~/.pi/agent/extensions/"
    format: other
    notes: "Global auto-discovered extensions."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\extensions\\"
    format: other
    notes: "Global auto-discovered extensions."

cli_params:
  - flag: "-e, --extension <path>"
    description: "Load an extension file or directory. Repeatable."
    example: "pi -e ./permission-gate.ts"
  - flag: "--no-extensions, -ne"
    description: "Disable extension discovery from standard locations. Explicit -e paths still load."
    example: "pi --no-extensions -e ./my-ext.ts"
  - flag: "--no-tools, -nt"
    description: "Disable all tools by default (built-in and extension)."
    example: "pi -nt -p 'hello'"
  - flag: "--no-builtin-tools, -nbt"
    description: "Disable built-in tools by default but keep extension/custom tools."
    example: "pi -nbt"
  - flag: "--tools, -t <tools>"
    description: "Allowlist specific tool names across built-in, extension, and custom tools."
    example: "pi -t read,grep,find,ls -p 'review code'"
  - flag: "--approve, -a"
    description: "Trust project-local settings/resources for this run (bypasses trust prompt)."
    example: "pi -a -e ./.pi/extensions/guard.ts"
  - flag: "--no-approve, -na"
    description: "Ignore project-local settings/resources for this run."
    example: "pi -na"
  - flag: "--mode json"
    description: "Output session events as JSON lines. Does not change extension event behavior."
    example: "pi --mode json 'list files'"
  - flag: "--mode rpc"
    description: "RPC mode over stdin/stdout. Extensions still run normally."
    example: "pi --mode rpc"
  - flag: "pi install <source>"
    description: "Install a Pi package (npm or git) that may include extensions."
    example: "pi install npm:@foo/pi-tools"
  - flag: "pi list"
    description: "List installed Pi packages/resources."
    example: "pi list"
  - flag: "pi config"
    description: "Open TUI to enable/disable package resources (extensions, skills, prompts, themes)."
    example: "pi config"

payload_fields:
  - native_event: resources_discover
    field: "reason"
    type: string
    meaning: "Why discovery is running: startup | reload."
  - native_event: session_start
    field: "reason"
    type: string
    meaning: "Why the session started: startup | reload | new | resume | fork."
  - native_event: session_start
    field: "previousSessionFile"
    type: string
    meaning: "Previously active session file, present for new/resume/fork."
  - native_event: session_before_fork
    field: "position"
    type: string
    meaning: "Fork position: before | at."
  - native_event: session_before_compact
    field: "preparation"
    type: object
    meaning: "Compaction preparation data including context entries to compact."
  - native_event: session_before_compact
    field: "customInstructions"
    type: string
    meaning: "User-provided custom compaction instructions."
  - native_event: session_compact
    field: "fromExtension"
    type: boolean
    meaning: "True if the compaction result came from an extension."
  - native_event: context
    field: "messages"
    type: AgentMessage[]
    meaning: "Full message list about to be sent to the LLM."
  - native_event: before_provider_request
    field: "payload"
    type: unknown
    meaning: "Provider-specific request payload."
  - native_event: after_provider_response
    field: "status"
    type: number
    meaning: "HTTP status code from provider response."
  - native_event: before_agent_start
    field: "prompt"
    type: string
    meaning: "Raw user prompt text after expansion."
  - native_event: before_agent_start
    field: "images"
    type: ImageContent[]
    meaning: "Images attached to the user prompt."
  - native_event: before_agent_start
    field: "systemPrompt"
    type: string
    meaning: "Fully assembled system prompt string."
  - native_event: before_agent_start
    field: "systemPromptOptions"
    type: object
    meaning: "Structured options describing which resources were loaded into the system prompt."
  - native_event: turn_start
    field: "turnIndex"
    type: number
    meaning: "Zero-based turn counter."
  - native_event: turn_end
    field: "toolResults"
    type: ToolResultMessage[]
    meaning: "Tool results produced during the turn."
  - native_event: message_update
    field: "assistantMessageEvent"
    type: object
    meaning: "Provider streaming event (text_delta, tool_call, etc.)."
  - native_event: tool_execution_start
    field: "toolCallId"
    type: string
    meaning: "Stable identifier for this tool call."
  - native_event: tool_execution_start
    field: "args"
    type: any
    meaning: "Arguments passed to the tool."
  - native_event: tool_execution_end
    field: "isError"
    type: boolean
    meaning: "Whether tool execution ended in error."
  - native_event: model_select
    field: "source"
    type: string
    meaning: "How the model was selected: set | cycle | restore."
  - native_event: input
    field: "source"
    type: string
    meaning: "Where input came from: interactive | rpc | extension."
  - native_event: input
    field: "text"
    type: string
    meaning: "User input text."
  - native_event: input
    field: "images"
    type: ImageContent[]
    meaning: "Images attached to input."
  - native_event: tool_call
    field: "toolCallId"
    type: string
    meaning: "Stable identifier for the tool call."
  - native_event: tool_call
    field: "toolName"
    type: string
    meaning: "Tool name (bash, read, edit, write, grep, find, ls, or custom)."
  - native_event: tool_call
    field: "input"
    type: object
    meaning: "Tool input arguments; type narrowed per built-in tool."
  - native_event: tool_result
    field: "content"
    type: TextContent[] | ImageContent[]
    meaning: "Tool result content blocks."
  - native_event: tool_result
    field: "details"
    type: unknown
    meaning: "Tool-specific metadata (e.g., BashToolDetails)."
  - native_event: user_bash
    field: "excludeFromContext"
    type: boolean
    meaning: "True for !! prefix commands, which are excluded from LLM context."

response_actions:
  - action: block
    native_value: "{ block: true, reason?: string }"
    effect: "tool_call: cancels the tool call. input: action 'handled' with optional notification prevents agent processing. user_bash: extension handles execution."
  - action: continue
    native_value: "{ action: 'continue' }"
    effect: "input event: proceed with current text/images unchanged."
  - action: modify
    native_value: "In-place mutation of event.input (tool_call) or returning { messages } (context) or { systemPrompt } (before_agent_start) or { text, images? } (input transform)"
    effect: "Mutate pending action/context before it reaches the agent/provider."
  - action: replace
    native_value: "{ message?: AgentMessage } (message_end), { result?: BashResult } (user_bash), { content?, details?, isError? } (tool_result)"
    effect: "Replace a finalized message, bash result, or tool result."
  - action: other
    native_value: "{ cancel?: boolean }"
    effect: "session_before_switch / session_before_fork / session_before_compact / session_before_tree: abort the session operation."
  - action: other
    native_value: "{ skillPaths?: string[], promptPaths?: string[], themePaths?: string[] }"
    effect: "resources_discover: append additional resource directories."

execution:
  shell: "N/A — extension handlers are in-process TypeScript functions running inside the pi Node.js/Bun process."
  cwd: "The pi session's current working directory; exposed as ctx.cwd."
  env: "The host process environment is inherited. PI_CODING_AGENT_DIR overrides the config root; PI_CODING_AGENT_SESSION_DIR overrides session storage; PI_PACKAGE_DIR overrides package asset resolution; PI_OFFLINE disables startup network ops."
  timeout: "No per-handler timeout documented. Handlers run synchronously/awaited inline; a hanging handler blocks the event pipeline."
  stdin: "N/A — events are JavaScript objects passed directly to handler functions."
  stdout: "N/A — handlers use ctx.ui.notify, pi.sendMessage, console.log, or process.stdout directly."
  stderr: "Uncaught errors are caught by ExtensionRunner, converted to ExtensionError objects, and forwarded to registered error listeners (logged to debug log). They do not terminate pi."
  notes: "Handlers run sequentially: per extension in load order, and per event handler in registration order. Multiple extensions can observe the same event. Cancellation/blocking events short-circuit on the first handler that returns cancel/block. Mutation events chain (context messages, before_provider_request payload, before_agent_start systemPrompt, message_end replacement, tool_result fields)."

gaps:
  - "Pi does not have a traditional hooks file format (no JSON/YAML hooks config). The only hook mechanism is the TypeScript Extension API, which is in-process and imperative rather than declarative."
  - "The package.json exports a './hooks' subpath, but the installed npm package (0.73.1) has no dist/core/hooks/ directory. It is unclear whether this is a deprecated planned feature or a build artifact."
  - "Return contract for before_provider_request is typed as unknown in source; exact replacement semantics are not documented beyond 'handler return replaces payload'."
  - "No native shell-command, HTTP endpoint, or LLM-evaluator hook handler kinds exist; Claudine would need to wrap TypeScript extensions to implement those action types."
  - "No built-in permission event separate from tool_call; permissions are implemented by extensions via the tool_call event (e.g., permission-gate.ts)."
  - "No subagent lifecycle events; Pi explicitly has no built-in sub-agents."
  - "Event handler execution has no documented timeout or sandboxing; a malicious or buggy extension runs with full process access."
  - "Project trust prompt behavior means project-local extensions may not load in non-interactive mode unless --approve is passed or defaultProjectTrust is always."
  - "The many-to-one Claudine mapping contains several provider-specific events (session_before_switch/fork/tree) with no clear unified equivalent."

changes: []
requires_claudine_update: true
reason: "Pi's hook model is fundamentally different from the declarative shell/HTTP/prompt/agent hook model Claudine currently assumes. Claudine's Pi adapter would need to generate TypeScript extension modules (or embed a small runtime extension) to translate Pi's native ExtensionEvent stream into Claudine's 16 unified lifecycle events. Key gaps: (1) no native permission/subagent events — these must be synthesized from tool_call; (2) blocking and mutation semantics are event-specific and chained, not parallel like Claude Code; (3) configuration lives in settings.json extensions/packages arrays and filesystem directories, not hooks.json files; (4) execution is in-process with no shell/HTTP/LLM handler abstraction."
---

# Pi extension events (hooks)

## Overview

Pi does not ship a traditional declarative hook system. Instead, it exposes an **Extension API**: TypeScript modules loaded at startup that subscribe to lifecycle events via `pi.on(eventName, handler)`. Extensions run **in-process** inside the Pi Node.js/Bun runtime and can register event handlers, custom tools, commands, keyboard shortcuts, CLI flags, and UI components.

A hook, in Pi terms, is therefore a `(event, ctx) => Promise<result | void>` function registered by an extension. The handler receives a typed event object and an `ExtensionContext` (`ctx`) with UI, session, model, and control methods. Handler kinds are **TypeScript functions only** — there are no first-class shell-command, HTTP endpoint, or LLM-evaluator hooks.

Capability summary:

| Capability | Supported? | Notes |
|------------|------------|-------|
| Block/deny tool calls | Yes | `tool_call` handler returns `{ block: true, reason }` |
| Cancel session operations | Yes | `session_before_*` handlers return `{ cancel: true }` |
| Mutate LLM messages | Yes | `context` handler returns `{ messages }` |
| Mutate tool arguments | Yes | Mutate `event.input` in place in `tool_call` |
| Mutate tool results | Yes | `tool_result` handler returns `{ content, details, isError }` |
| Replace system prompt | Yes | `before_agent_start` handler returns `{ systemPrompt }` |
| Replace a finalized message | Yes | `message_end` handler returns `{ message }` |
| Transform user input | Yes | `input` handler returns `{ action: "transform", text }` |
| Observe only | Yes | Most events have no return contract |
| Shell-command hooks | No | Must spawn processes from inside TS handler |
| HTTP endpoint hooks | No | Must fetch from inside TS handler |
| LLM-evaluator hooks | No | Must call a model from inside TS handler |

## Native Hooks

Pi's native event inventory is the `ExtensionEvent` union in `packages/coding-agent/src/core/extensions/types.ts`. There are **29 events** grouped below by lifecycle phase.

### Resource and session lifecycle

| Event | Timing | Can block | Notes |
|-------|--------|-----------|-------|
| `resources_discover` | pre | no | Declare extra skill/prompt/theme paths after session start |
| `session_start` | pre | no | Session started/loaded/resumed/forked/reloaded |
| `session_before_switch` | pre | yes (`cancel`) | Before switching to another session |
| `session_before_fork` | pre | yes (`cancel`) | Before forking a session |
| `session_before_compact` | pre | yes (`cancel`) | Before context compaction |
| `session_compact` | post | no | After context compaction |
| `session_shutdown` | post | no | Before extension runtime teardown |
| `session_before_tree` | pre | yes (`cancel`) | Before tree navigation |
| `session_tree` | post | no | After tree navigation |

### Prompt and context lifecycle

| Event | Timing | Can block/mutate | Notes |
|-------|--------|------------------|-------|
| `input` | pre | yes (`handled`/`transform`) | User input before agent processing |
| `before_agent_start` | pre | mutate | After prompt submission, before agent loop |
| `context` | pre | mutate | Before each LLM call; can replace message list |
| `before_provider_request` | pre | mutate | Before provider request; return replaces payload |
| `after_provider_response` | post | no | After provider response received |

### Agent and turn lifecycle

| Event | Timing | Can block/mutate | Notes |
|-------|--------|------------------|-------|
| `agent_start` | pre | no | Agent loop started |
| `agent_end` | post | no | Agent loop ended |
| `turn_start` | pre | no | Assistant turn started |
| `turn_end` | post | no | Assistant turn ended |
| `message_start` | pre | no | Any message started |
| `message_update` | around | no | Streaming token update |
| `message_end` | post | mutate | Message finalized; can replace if role preserved |

### Tool lifecycle

| Event | Timing | Can block/mutate | Notes |
|-------|--------|------------------|-------|
| `tool_call` | pre | yes (`block`) + mutate args | Before tool executes |
| `tool_result` | post | mutate result | After tool executes |
| `tool_execution_start` | pre | no | Executor started running tool |
| `tool_execution_update` | around | no | Partial/streaming tool output |
| `tool_execution_end` | post | no | Tool execution finished |
| `user_bash` | pre | yes (`result` replacement) | User `!`/`!!` bash prefix |

### Model and settings lifecycle

| Event | Timing | Can block/mutate | Notes |
|-------|--------|------------------|-------|
| `model_select` | post | no | Model changed |
| `thinking_level_select` | post | no | Thinking level changed |

### Matcher/filter mechanism

There is no global matcher syntax. Extensions filter events manually inside their handlers by inspecting event fields (e.g., `event.toolName === "bash"`, `event.source === "interactive"`, regex against `event.input.command`). The first handler to return a blocking result can short-circuit the pipeline for blocking events.

## Configuration

Extensions (Pi's hook containers) are configured via settings files and filesystem discovery.

### Settings files

| Scope | macOS / Linux | Windows | Notes |
|-------|---------------|---------|-------|
| Global user | `~/.pi/agent/settings.json` | `%USERPROFILE%\.pi\agent\settings.json` | `extensions` array and `packages` array |
| Project | `.pi/settings.json` | `.pi\settings.json` | Overrides global; subject to project trust |

The `extensions` setting is an array of file or directory paths. The `packages` setting is an array of npm/git package sources (string or object with `extensions`/`skills`/`prompts`/`themes` filters).

### Auto-discovery directories

| Scope | macOS / Linux | Windows |
|-------|---------------|---------|
| Project | `.pi/extensions/` | `.pi\extensions\` |
| Global | `~/.pi/agent/extensions/` | `%USERPROFILE%\.pi\agent\extensions\` |

Discovery rules within each directory:

1. Direct `.ts` or `.js` files are loaded.
2. Subdirectories with `index.ts`/`index.js` are loaded.
3. Subdirectories with `package.json` containing a `pi.extensions` field load the declared entry points.

No recursion beyond one level.

### CLI switches and commands

| Flag / command | Effect |
|----------------|--------|
| `pi -e <path>` / `pi --extension <path>` | Load an explicit extension file or directory. Repeatable. |
| `pi --no-extensions` / `pi -ne` | Disable auto-discovery. Explicit `-e` paths still load. |
| `pi --no-tools` / `pi -nt` | Disable all tools by default. |
| `pi --no-builtin-tools` / `pi -nbt` | Disable built-in tools only. |
| `pi --tools <names>` / `pi -t <names>` | Allowlist tool names. |
| `pi -a` / `pi --approve` | Trust project-local settings/resources for this run. |
| `pi -na` / `pi --no-approve` | Ignore project-local settings/resources for this run. |
| `pi install npm:<pkg>` / `pi install git:<repo>` | Install a Pi package that may bundle extensions. |
| `pi list` | List installed packages/resources. |
| `pi config` | TUI to enable/disable package resources. |
| `pi --mode json` | JSON-line event stream; does not affect extension execution. |
| `pi --mode rpc` | RPC mode; extensions still run normally. |

### Environment variables

| Variable | Effect |
|----------|--------|
| `PI_CODING_AGENT_DIR` | Override config directory (default `~/.pi/agent`). |
| `PI_CODING_AGENT_SESSION_DIR` | Override session storage directory. |
| `PI_PACKAGE_DIR` | Override package asset resolution path. |
| `PI_OFFLINE` | Disable startup network operations. |
| `PI_SKIP_VERSION_CHECK` | Skip the pi.dev latest-version check. |
| `PI_TELEMETRY` | Override install/update telemetry. |

There is no environment variable that disables extensions globally; use `--no-extensions` or omit extension paths.

## Payloads and Responses

### Common context

Every handler receives an `ExtensionContext` (`ctx`) with:

- `ctx.ui` — UI methods (no-op in print/RPC mode; `ctx.hasUI` is false)
- `ctx.cwd` — current working directory
- `ctx.sessionManager` — read-only session manager
- `ctx.modelRegistry` — model registry
- `ctx.model` — current model
- `ctx.isIdle()` — whether the agent is idle
- `ctx.signal` — abort signal during streaming
- `ctx.abort()` — abort current operation
- `ctx.shutdown()` — graceful shutdown
- `ctx.getSystemPrompt()` — current effective system prompt

### Per-event payloads and response contracts

| Event | Payload fields | Response contract |
|-------|----------------|-------------------|
| `resources_discover` | `cwd`, `reason` | `{ skillPaths?, promptPaths?, themePaths? }` |
| `session_start` | `reason`, `previousSessionFile?` | None |
| `session_before_switch` | `reason`, `targetSessionFile?` | `{ cancel?: boolean }` |
| `session_before_fork` | `entryId`, `position` | `{ cancel?: boolean, skipConversationRestore?: boolean }` |
| `session_before_compact` | `preparation`, `branchEntries`, `customInstructions?`, `signal` | `{ cancel?: boolean, compaction?: CompactionResult }` |
| `session_compact` | `compactionEntry`, `fromExtension` | None |
| `session_shutdown` | `reason`, `targetSessionFile?` | None |
| `session_before_tree` | `preparation`, `signal` | `{ cancel?: boolean, summary?, customInstructions?, replaceInstructions?, label? }` |
| `session_tree` | `newLeafId`, `oldLeafId`, `summaryEntry?`, `fromExtension?` | None |
| `context` | `messages` | `{ messages?: AgentMessage[] }` replaces list |
| `before_provider_request` | `payload` | Return value replaces payload |
| `after_provider_response` | `status`, `headers` | None |
| `before_agent_start` | `prompt`, `images?`, `systemPrompt`, `systemPromptOptions` | `{ message?, systemPrompt? }`; messages collected, systemPrompt chained |
| `agent_start` | — | None |
| `agent_end` | `messages` | None |
| `turn_start` | `turnIndex`, `timestamp` | None |
| `turn_end` | `turnIndex`, `message`, `toolResults` | None |
| `message_start` | `message` | None |
| `message_update` | `message`, `assistantMessageEvent` | None |
| `message_end` | `message` | `{ message?: AgentMessage }`; role must stay the same |
| `tool_execution_start` | `toolCallId`, `toolName`, `args` | None |
| `tool_execution_update` | `toolCallId`, `toolName`, `args`, `partialResult` | None |
| `tool_execution_end` | `toolCallId`, `toolName`, `result`, `isError` | None |
| `model_select` | `model`, `previousModel`, `source` | None |
| `thinking_level_select` | `level`, `previousLevel` | None |
| `user_bash` | `command`, `excludeFromContext`, `cwd` | `{ operations?, result? }`; `result` fully replaces execution |
| `input` | `text`, `images?`, `source` | `{ action: "continue" | "transform" | "handled", text?, images? }` |
| `tool_call` | `toolCallId`, `toolName`, `input` | `{ block?: boolean, reason? }`; mutate `event.input` in place |
| `tool_result` | `toolCallId`, `toolName`, `input`, `content`, `isError`, `details` | `{ content?, details?, isError? }` |

### Tool input shapes

Built-in tool `tool_call` events carry typed inputs:

- `bash`: `{ command: string, description?: string, timeout?: number }`
- `read`: `{ file_path: string, offset?: number, limit?: number }`
- `write`: `{ file_path: string, content: string }`
- `edit`: `{ file_path: string, old_string: string, new_string: string }`
- `grep`: `{ pattern: string, path?: string, output_mode?: string }`
- `find`: `{ pattern: string, path?: string }`
- `ls`: `{ path: string }`
- Custom tools: `Record<string, unknown>`

## Execution Semantics

### Runtime environment

- **Runtime**: Extension code executes inside the same Node.js/Bun process as Pi.
- **Working directory**: `ctx.cwd`, the session's current working directory.
- **Environment**: Host process environment is inherited. Relevant env vars are listed in Configuration.
- **Shell**: Not applicable; handlers are TypeScript functions. They may spawn subprocesses via `ctx.exec` or Node APIs.
- **Timeout**: No per-handler timeout is documented. A hanging handler blocks the event pipeline.

### Sequential versus parallel

Handlers run **sequentially**:

- Extensions are loaded in order (project auto-discovered, then global auto-discovered, then explicit paths, then packages).
- Within one extension, handlers for an event run in registration order.
- Across extensions, the runner iterates extensions in load order.
- Blocking/short-circuit events stop at the first handler that returns a blocking result (`cancel`, `block`, or `handled`).
- Mutation events chain: later handlers see earlier mutations (e.g., `context` messages, `before_provider_request` payload, `tool_result` fields).

### Error handling

Uncaught errors in handlers are caught by `ExtensionRunner`, converted to `ExtensionError` objects (`{ extensionPath, event, error, stack? }`), and forwarded to registered error listeners. They are logged to the debug log and do **not** terminate Pi or stop subsequent handlers.

### Async hooks

All handlers may be async (`Promise<R | void>`). The runner `await`s each handler before proceeding.

## Claudine Mapping

Pi's 29 events map into Claudine's 16 normalized lifecycle events as follows. Provider-specific session/tree events have no direct Claudine equivalent and are marked `unknown`.

| Pi native event | Claudine event | Notes |
|-----------------|----------------|-------|
| `resources_discover` | `initialize` | Resource discovery at startup/reload |
| `session_start` | `initialize` | Session initialization |
| `session_shutdown` | `finalize` | Session/runtime teardown |
| `session_before_compact` | `notification` | Pre-compaction notification (can cancel) |
| `session_compact` | `notification` | Post-compaction notification |
| `context` | `prompt` | Pre-LLM message mutation |
| `before_provider_request` | `prompt` | Provider request mutation |
| `before_agent_start` | `prompt` | Pre-turn system-prompt/message mutation |
| `input` | `prompt` | User input intercept/transform |
| `agent_start` | `start` | Agent loop start |
| `agent_end` | `finalize` | Agent loop end |
| `turn_start` | `loop` | Turn iteration start |
| `turn_end` | `loop` | Turn iteration end |
| `message_start` | `notification` | Message display start |
| `message_update` | `notification` | Streaming display update |
| `message_end` | `notification` | Message finalized (can replace) |
| `tool_call` | `tool_call` | Pre-tool execution; carries permission-like blocking |
| `user_bash` | `tool_call` | User-initiated shell execution |
| `tool_result` | `tool_result` | Post-tool result mutation |
| `tool_execution_start` | `tool_call` | Observation-only tool start |
| `tool_execution_update` | `tool_result` | Observation-only partial result |
| `tool_execution_end` | `tool_result` | Observation-only tool end |
| `model_select` | `notification` | Model change notification |
| `thinking_level_select` | `notification` | Settings change notification |
| `session_before_switch` | `unknown` | Session management specific |
| `session_before_fork` | `unknown` | Session management specific |
| `session_before_tree` | `unknown` | Session tree navigation specific |
| `session_tree` | `unknown` | Session tree navigation specific |

Provider-specific payload fields Claudine should preserve on unified payloads:

- `reason` (session events, compaction)
- `turnIndex`
- `toolCallId`
- `toolName`
- `source` (input, model_select)
- `excludeFromContext` (user_bash)
- `fromExtension` (compaction/tree)

## Gaps

1. Pi has no declarative hooks file. Hook behavior is expressed only through TypeScript extensions.
2. The `package.json` exports a `./hooks` subpath, but the installed package (0.73.1) contains no `dist/core/hooks/` directory. Its purpose is unknown.
3. `before_provider_request` return semantics are typed as `unknown`; precise contract is undocumented.
4. No first-class shell-command, HTTP endpoint, or LLM-evaluator hook handlers exist.
5. No built-in permission event separate from `tool_call`; permission gates are implemented as extensions.
6. No subagent lifecycle events; Pi explicitly omits built-in sub-agents.
7. No documented per-handler timeout or sandbox; extensions run with full process access.
8. Project trust behavior can prevent project-local extensions from loading in non-interactive mode unless `--approve` is used or `defaultProjectTrust` is `always`.
9. Session-management events (`session_before_switch`, `session_before_fork`, `session_before_tree`, `session_tree`) have no clear Claudine unified equivalent.

## Sources

- Pi homepage: <https://pi.dev/>
- Pi docs: <https://pi.dev/docs/latest>
- Extension docs: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/docs/extensions.md>
- Settings docs: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/docs/settings.md>
- JSON mode docs: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/docs/json.md>
- Session format docs: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/docs/session-format.md>
- Main repo README: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent>
- Extension types source: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/src/core/extensions/types.ts>
- Extension runner source: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/src/core/extensions/runner.ts>
- Permission gate example: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/permission-gate.ts>
- Protected paths example: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/protected-paths.ts>
- Input transform example: <https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/input-transform.ts>
- Installed package inspected locally: `/Users/ken/.bun/install/global/node_modules/@mariozechner/pi-coding-agent/` v0.73.1
- Local config inspected: `~/.pi/agent/settings.json`, `~/.pi/agent/models.json`
