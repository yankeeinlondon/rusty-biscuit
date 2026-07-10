---
$schema: ./_schema.yaml
created: "2026-07-03"
last_updated: "2026-07-03"
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://opencode.ai
docs: https://opencode.ai/docs
hooks_docs: https://opencode.ai/docs/plugins/
hooks:
  - native_event: config
    claudine_event: initialize
    timing: post
    blocking: false
    payload_schema: "merged_opencode_config_object"
    return_contract: "Return value ignored; observation/validation only."
    notes: "Fires once after all plugins are loaded with the fully merged config."
  - native_event: dispose
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "none"
    return_contract: "Return value ignored; cleanup only."
    notes: "Fires on shutdown."
  - native_event: event
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "{ event: { id, type, properties } }"
    return_contract: "Fire-and-forget; return value/throw has no effect."
    notes: "Catch-all event bus listener. Filtered to the plugin's directory."
  - native_event: session.created
    claudine_event: initialize
    timing: post
    blocking: false
    payload_schema: "sessionID, info { id, slug, projectID, directory, title, agent, model, time.created }"
    return_contract: "Observation only."
    notes: "Fires after a session is created."
  - native_event: session.updated
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "sessionID, info { ...SessionInfo }"
    return_contract: "Observation only."
    notes: "Fires when session metadata changes."
  - native_event: session.deleted
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "sessionID, info { ...SessionInfo }"
    return_contract: "Observation only."
    notes: "Fires when a session is deleted."
  - native_event: session.error
    claudine_event: failure
    timing: post
    blocking: false
    payload_schema: "sessionID?, error { name, data }"
    return_contract: "Observation only."
    notes: "Fires on session-level errors."
  - native_event: session.idle
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "sessionID"
    return_contract: "Observation only; deprecated in favor of session.status."
    notes: "Fires when a session becomes idle."
  - native_event: session.status
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "sessionID, status { type: idle|busy|retry, attempt?, message?, action?, next? }"
    return_contract: "Observation only."
    notes: "Status change notifications."
  - native_event: session.compacted
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "sessionID"
    return_contract: "Observation only."
    notes: "Fires after context compaction completes."
  - native_event: message.updated
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "sessionID, info { role, id, time, ... }"
    return_contract: "Observation only."
    notes: "Fires when a message is created or updated."
  - native_event: message.removed
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "sessionID, messageID"
    return_contract: "Observation only."
    notes: "Fires when a message is removed."
  - native_event: message.part.updated
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "sessionID, part { id, type, text|... }, time"
    return_contract: "Observation only."
    notes: "Fires when a message part is updated."
  - native_event: message.part.removed
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "sessionID, messageID, partID"
    return_contract: "Observation only."
    notes: "Fires when a message part is removed."
  - native_event: permission.asked
    claudine_event: permission
    timing: pre
    blocking: false
    payload_schema: "id, sessionID, permission, patterns[], metadata, always[], tool { messageID, callID }"
    return_contract: "Observation only on event bus; use dedicated permission.ask hook to influence."
    notes: "Fires when a permission prompt is shown to the user."
  - native_event: permission.replied
    claudine_event: permission
    timing: post
    blocking: false
    payload_schema: "sessionID, requestID, reply, message?"
    return_contract: "Observation only."
    notes: "Fires after the user replies to a permission prompt."
  - native_event: command.executed
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "(schema in legacy-event.ts)"
    return_contract: "Observation only."
    notes: "Fires after a TUI command is executed."
  - native_event: file.edited
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "file: string"
    return_contract: "Observation only."
    notes: "Fires after a file is edited."
  - native_event: file.watcher.updated
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "(filesystem-watcher schema)"
    return_contract: "Observation only."
    notes: "Fires on filesystem watcher updates."
  - native_event: tui.prompt.append
    claudine_event: prompt
    timing: pre
    blocking: false
    payload_schema: "text: string"
    return_contract: "Observation only."
    notes: "Fires when text is appended to the TUI prompt."
  - native_event: tui.command.execute
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "command: string|enum"
    return_contract: "Observation only."
    notes: "Fires when a TUI command is invoked."
  - native_event: tui.toast.show
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "title?, message, variant, duration"
    return_contract: "Observation only."
    notes: "Fires when a toast notification is shown."
  - native_event: chat.message
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "sessionID, agent?, model?, messageID?, variant?"
    return_contract: "Throwing aborts processing; mutating output.message/parts changes the user message."
    notes: "Typed in Hooks interface; runtime call site not confirmed in current source."
  - native_event: chat.params
    claudine_event: start
    timing: pre
    blocking: false
    payload_schema: "sessionID, agent, model, provider, message"
    return_contract: "Mutate output.temperature, topP, topK, maxOutputTokens, options."
    notes: "Fires before each LLM request."
  - native_event: chat.headers
    claudine_event: start
    timing: pre
    blocking: false
    payload_schema: "sessionID, agent, model, provider, message"
    return_contract: "Mutate output.headers."
    notes: "Fires before each LLM request; headers merged into provider request."
  - native_event: experimental.chat.system.transform
    claudine_event: prompt
    timing: pre
    blocking: false
    payload_schema: "sessionID?, model"
    return_contract: "Mutate output.system string array."
    notes: "Fires before system prompts are finalized."
  - native_event: experimental.chat.messages.transform
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "{}"
    return_contract: "Mutate output.messages array before compaction model call."
    notes: "Fires during compaction."
  - native_event: experimental.session.compacting
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "sessionID"
    return_contract: "Mutate output.context (append) or output.prompt (replace)."
    notes: "Fires before the compaction LLM call."
  - native_event: experimental.compaction.autocontinue
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "sessionID, agent, model, provider, message, overflow"
    return_contract: "Set output.enabled to false to skip synthetic continue turn."
    notes: "Fires after compaction, before auto-continue."
  - native_event: experimental.text.complete
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "sessionID, messageID, partID"
    return_contract: "Mutate output.text."
    notes: "Fires after assistant text stream ends."
  - native_event: tool.execute.before
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "tool, sessionID, callID"
    return_contract: "Throwing aborts the tool call; mutating output.args changes tool input."
    notes: "Fires before every tool execution, including MCP tools."
  - native_event: tool.execute.after
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "tool, sessionID, callID, args"
    return_contract: "Throwing aborts/turns into error; mutating output.title/output/metadata changes result."
    notes: "Fires after every tool execution."
  - native_event: shell.env
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "cwd, sessionID?, callID?"
    return_contract: "Mutate output.env to inject environment variables into shell subprocess."
    notes: "Fires before each bash/shell tool invocation."
  - native_event: tool.definition
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "toolID"
    return_contract: "Mutate output.description and output.parameters (JSON schema)."
    notes: "Fires per tool when building the tool list sent to the LLM."
  - native_event: permission.ask
    claudine_event: permission
    timing: pre
    blocking: true
    payload_schema: "Permission request object"
    return_contract: "Set output.status to allow|deny|ask; throwing aborts."
    notes: "Typed in Hooks interface; runtime call site not confirmed in current source."
  - native_event: command.execute.before
    claudine_event: notification
    timing: pre
    blocking: true
    payload_schema: "command, sessionID, arguments"
    return_contract: "Mutate output.parts; throwing aborts."
    notes: "Typed in Hooks interface; runtime call site not confirmed in current source."
  - native_event: provider
    claudine_event: none
    timing: unknown
    blocking: false
    payload_schema: "none (returns models() catalog function at init)"
    return_contract: "Return model catalog extensions."
    notes: "Not a lifecycle event; extends provider model catalogs."
  - native_event: auth
    claudine_event: none
    timing: unknown
    blocking: false
    payload_schema: "none (returns auth method definitions at init)"
    return_contract: "Return auth method definitions."
    notes: "Not a lifecycle event; registers authentication methods."
  - native_event: tool
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "none (returns custom tool definitions at init)"
    return_contract: "Return custom tool definitions keyed by tool ID."
    notes: "Not a per-call event; registers tools when plugin loads."
config_files:
  - os: macos
    scope: user
    path: "~/.config/opencode/plugins/*.{ts,js}"
    format: other
    notes: "Global local plugins. Singular plugin/ also scanned."
  - os: linux
    scope: user
    path: "~/.config/opencode/plugins/*.{ts,js}"
    format: other
    notes: "Global local plugins. Singular plugin/ also scanned."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\plugins\\*.{ts,js}"
    format: other
    notes: "Global local plugins. Singular plugin/ also scanned."
  - os: macos
    scope: repo
    path: ".opencode/plugins/*.{ts,js}"
    format: other
    notes: "Project local plugins. Singular plugin/ also scanned."
  - os: linux
    scope: repo
    path: ".opencode/plugins/*.{ts,js}"
    format: other
    notes: "Project local plugins. Singular plugin/ also scanned."
  - os: windows
    scope: repo
    path: ".opencode\\plugins\\*.{ts,js}"
    format: other
    notes: "Project local plugins. Singular plugin/ also scanned."
  - os: macos
    scope: user
    path: "~/.config/opencode/opencode.json{c}"
    format: jsonc
    notes: "User config; plugin array loads npm plugins."
  - os: linux
    scope: user
    path: "~/.config/opencode/opencode.json{c}"
    format: jsonc
    notes: "User config; plugin array loads npm plugins."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.json{c}"
    format: jsonc
    notes: "User config; plugin array loads npm plugins."
  - os: macos
    scope: repo
    path: "opencode.json{c}"
    format: jsonc
    notes: "Project config; plugin array loads npm plugins."
  - os: linux
    scope: repo
    path: "opencode.json{c}"
    format: jsonc
    notes: "Project config; plugin array loads npm plugins."
  - os: windows
    scope: repo
    path: "opencode.json{c}"
    format: jsonc
    notes: "Project config; plugin array loads npm plugins."
  - os: macos
    scope: repo
    path: ".opencode/package.json"
    format: json
    notes: "Optional plugin dependencies for local plugins."
  - os: linux
    scope: repo
    path: ".opencode/package.json"
    format: json
    notes: "Optional plugin dependencies for local plugins."
  - os: windows
    scope: repo
    path: ".opencode\\package.json"
    format: json
    notes: "Optional plugin dependencies for local plugins."
  - os: macos
    scope: managed
    path: "/Library/Application Support/opencode/opencode.json{c}"
    format: jsonc
    notes: "Managed admin config; overrides user/project."
  - os: linux
    scope: managed
    path: "/etc/opencode/opencode.json{c}"
    format: jsonc
    notes: "Managed admin config; overrides user/project."
  - os: windows
    scope: managed
    path: "%ProgramData%\\opencode\\opencode.json{c}"
    format: jsonc
    notes: "Managed admin config; overrides user/project."
  - os: macos
    scope: managed
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    format: other
    notes: "MDM-delivered managed preferences; highest priority."
  - os: macos
    scope: managed
    path: "/Library/Managed Preferences/ai.opencode.managed.plist"
    format: other
    notes: "MDM-delivered managed preferences; highest priority."
cli_params:
  - flag: "opencode run [message..]"
    description: "Run OpenCode non-interactively; plugin hooks still fire for the executed session."
    example: "opencode run 'explain this code'"
  - flag: "opencode debug config"
    description: "Show resolved config including plugin origins and managed settings."
    example: "opencode debug config"
  - flag: "OPENCODE_PURE=true"
    description: "Environment variable that disables loading of all external plugins."
    example: "OPENCODE_PURE=1 opencode"
  - flag: "OPENCODE_DISABLE_DEFAULT_PLUGINS=true"
    description: "Environment variable that skips the built-in auth/provider plugins."
    example: "OPENCODE_DISABLE_DEFAULT_PLUGINS=1 opencode"
  - flag: "OPENCODE_CONFIG=<file>"
    description: "Load an alternate config file between global and project configs."
    example: "OPENCODE_CONFIG=./ci-opencode.json opencode run 'test'"
  - flag: "OPENCODE_CONFIG_DIR=<dir>"
    description: "Use a custom config directory scanned for agents, commands, modes, and plugins."
    example: "OPENCODE_CONFIG_DIR=./custom opencode"
  - flag: "OPENCODE_CONFIG_CONTENT=<json>"
    description: "Inline JSON config override loaded after project config and before managed config."
    example: "OPENCODE_CONFIG_CONTENT='{\"plugin\":[]}' opencode"
  - flag: "OPENCODE_EXPERIMENTAL=true"
    description: "Enables many experimental flags at once, which may change which plugin hooks are active."
    example: "OPENCODE_EXPERIMENTAL=1 opencode"
payload_fields:
  - native_event: event
    field: "event.id"
    type: string
    meaning: "Unique event identifier."
  - native_event: event
    field: "event.type"
    type: string
    meaning: "Event type string (e.g. session.created)."
  - native_event: event
    field: "event.properties"
    type: object
    meaning: "Event-specific schema data (same as event.data internally)."
  - native_event: session.created
    field: "sessionID"
    type: string
    meaning: "Session identifier."
  - native_event: session.created
    field: "info.id"
    type: string
    meaning: "Session ID (same as sessionID)."
  - native_event: session.created
    field: "info.title"
    type: string
    meaning: "Session title."
  - native_event: session.created
    field: "info.directory"
    type: string
    meaning: "Project directory associated with the session."
  - native_event: session.created
    field: "info.agent"
    type: string
    meaning: "Agent name for the session."
  - native_event: session.created
    field: "info.model.id"
    type: string
    meaning: "Model ID."
  - native_event: session.created
    field: "info.model.providerID"
    type: string
    meaning: "Provider ID."
  - native_event: session.error
    field: "error.name"
    type: string
    meaning: "Error discriminator (e.g. ProviderAuthError, APIError)."
  - native_event: session.error
    field: "error.data"
    type: object
    meaning: "Error-specific payload."
  - native_event: session.status
    field: "status.type"
    type: string
    meaning: "idle | busy | retry"
  - native_event: session.status
    field: "status.attempt"
    type: number
    meaning: "Retry attempt number when status.type=retry."
  - native_event: message.updated
    field: "info.role"
    type: string
    meaning: "user | assistant"
  - native_event: message.updated
    field: "info.id"
    type: string
    meaning: "Message ID."
  - native_event: message.updated
    field: "info.time.completed"
    type: number
    meaning: "Completion timestamp for assistant messages."
  - native_event: permission.asked
    field: "permission"
    type: string
    meaning: "Permission key being requested (e.g. bash, edit)."
  - native_event: permission.asked
    field: "patterns"
    type: array
    meaning: "Patterns the permission applies to."
  - native_event: permission.asked
    field: "tool.messageID"
    type: string
    meaning: "Message ID when triggered by a tool call."
  - native_event: permission.asked
    field: "tool.callID"
    type: string
    meaning: "Tool call ID when triggered by a tool call."
  - native_event: permission.replied
    field: "reply"
    type: string
    meaning: "once | always | reject"
  - native_event: file.edited
    field: "file"
    type: string
    meaning: "Path of the edited file."
  - native_event: tui.prompt.append
    field: "text"
    type: string
    meaning: "Text appended to the prompt."
  - native_event: tui.toast.show
    field: "message"
    type: string
    meaning: "Toast message body."
  - native_event: tui.toast.show
    field: "variant"
    type: string
    meaning: "info | success | warning | error"
  - native_event: chat.params
    field: "sessionID"
    type: string
    meaning: "Current session ID."
  - native_event: chat.params
    field: "agent"
    type: string
    meaning: "Agent name."
  - native_event: chat.params
    field: "model"
    type: object
    meaning: "Provider model object."
  - native_event: chat.params
    field: "provider"
    type: object
    meaning: "Provider context with source, info, options."
  - native_event: chat.params
    field: "message"
    type: object
    meaning: "User message object."
  - native_event: tool.execute.before
    field: "tool"
    type: string
    meaning: "Tool ID being executed."
  - native_event: tool.execute.before
    field: "sessionID"
    type: string
    meaning: "Current session ID."
  - native_event: tool.execute.before
    field: "callID"
    type: string
    meaning: "Tool call ID."
  - native_event: tool.execute.before
    field: "output.args"
    type: object
    meaning: "Mutable tool arguments."
  - native_event: tool.execute.after
    field: "output.title"
    type: string
    meaning: "Mutable tool result title."
  - native_event: tool.execute.after
    field: "output.output"
    type: string
    meaning: "Mutable tool result output."
  - native_event: tool.execute.after
    field: "output.metadata"
    type: object
    meaning: "Mutable tool result metadata."
  - native_event: shell.env
    field: "cwd"
    type: string
    meaning: "Working directory for the shell invocation."
  - native_event: shell.env
    field: "output.env"
    type: object
    meaning: "Mutable environment variables to inject."
  - native_event: experimental.session.compacting
    field: "output.context"
    type: array
    meaning: "Additional context strings appended to default compaction prompt."
  - native_event: experimental.session.compacting
    field: "output.prompt"
    type: string
    meaning: "If set, replaces the entire compaction prompt."
response_actions:
  - action: block
    native_value: "throw new Error(...) in a dedicated hook"
    effect: "Aborts the current action (tool call, LLM request, etc.). Event bus event handler cannot block."
  - action: modify
    native_value: "mutate output.args (tool.execute.before)"
    effect: "Changes the arguments passed to the tool before execution."
  - action: modify
    native_value: "mutate output.title/output.output/output.metadata (tool.execute.after)"
    effect: "Changes the tool result displayed to the model and user."
  - action: modify
    native_value: "mutate output.env (shell.env)"
    effect: "Injects environment variables into the shell subprocess."
  - action: modify
    native_value: "mutate output.context or output.prompt (experimental.session.compacting)"
    effect: "Appends context to or replaces the compaction prompt."
  - action: modify
    native_value: "mutate output.temperature/topP/topK/maxOutputTokens/options (chat.params)"
    effect: "Changes LLM request parameters."
  - action: modify
    native_value: "mutate output.headers (chat.headers)"
    effect: "Adds headers to the provider HTTP request."
  - action: modify
    native_value: "mutate output.system (experimental.chat.system.transform)"
    effect: "Changes system prompts sent to the model."
  - action: modify
    native_value: "mutate output.description/output.parameters (tool.definition)"
    effect: "Changes tool definition sent to the model."
  - action: modify
    native_value: "mutate output.enabled (experimental.compaction.autocontinue)"
    effect: "Set to false to skip the synthetic continue turn after compaction."
  - action: modify
    native_value: "mutate output.text (experimental.text.complete)"
    effect: "Overrides the final assistant text part."
  - action: modify
    native_value: "mutate output.messages (experimental.chat.messages.transform)"
    effect: "Alters the message list used as compaction input."
  - action: modify
    native_value: "mutate output.parts (chat.message, command.execute.before)"
    effect: "Changes message parts; runtime call sites unconfirmed."
  - action: allow
    native_value: "output.status = 'allow' (permission.ask)"
    effect: "Grants the permission; runtime call site unconfirmed."
  - action: deny
    native_value: "output.status = 'deny' (permission.ask)"
    effect: "Denies the permission; runtime call site unconfirmed."
  - action: other
    native_value: "return custom tools from tool hook"
    effect: "Registers custom tools available to the model."
  - action: other
    native_value: "return provider/models function from provider hook"
    effect: "Extends the provider model catalog."
  - action: other
    native_value: "return auth methods from auth hook"
    effect: "Registers authentication methods."
execution:
  shell: "Bun runtime (plugins are JS/TS modules imported by Bun)."
  cwd: "Plugin runs inside the OpenCode process; PluginInput.directory is the project directory and PluginInput.worktree is the git worktree root."
  env: "process.env of the OpenCode process. shell.env hook can inject additional environment variables into bash tool subprocesses."
  timeout: "No documented per-hook timeout. Plugin code runs within the timeout of the action it hooks (e.g. shell tool timeout, LLM request timeout)."
  stdin: "No stdin channel for plugin hooks. Tool arguments and payloads arrive as function arguments."
  stdout: "Use client.app.log() for structured logging; console.log also works but is less structured."
  stderr: "Plugin errors are captured in the OpenCode debug log and may surface as session.error events."
  notes: "Dedicated hooks run sequentially in plugin load order, all receiving the same mutable output object. The catch-all event hook is fire-and-forget and filtered to the plugin's project directory. NPM plugins are installed automatically into ~/.cache/opencode/node_modules/."
gaps:
  - "No dedicated 'Hooks' documentation page exists; event and hook documentation lives under /docs/plugins/."
  - "Some entries in the Hooks TypeScript interface (chat.message, permission.ask, command.execute.before) have no confirmed runtime call site in the current source tree."
  - "Exact payload shapes for every event bus event (especially session.next.* granular events, MCP events, and LSP events) are defined in schema files but not all are documented for plugin authors."
  - "Plugin error handling policy (whether a throwing plugin aborts the whole action or is caught) is determined by surrounding Effect code and varies by hook site."
  - "No documented timeout, retry, or sandbox policy for plugin hook execution."
  - "Permission decisions cannot be made from the permission.asked event bus event; the dedicated permission.ask hook is typed but its invocation status is unclear."
  - "The observed local bridge plugin at ~/.config/opencode/plugin/claudine-bridge.ts only observes events and does not mutate outputs or block actions, so it cannot exercise the full hook contract."
changes: []
requires_claudine_update: true
reason: "OpenCode's plugin hook model is structurally different from settings-file hook systems. Claudine's existing OpenCode bridge is observation-only, maps only a subset of events, and the disabled legacy bridge used an API that no longer matches OpenCode's V1 plugin contract. Supporting OpenCode's full hook semantics requires Claudine to emit lifecycle events from both the event bus and dedicated plugin hooks, wire plugin output mutations back into the action pipeline, handle blocking via thrown errors for dedicated hooks, and regenerate the bridge plugin to use the current V1 API."
---

# OpenCode CLI hooks and events

## Overview

OpenCode CLI implements hooks as a **plugin system** rather than a settings-file hook registry. A hook is a JavaScript or TypeScript module that exports a plugin function. The function receives a context object and returns a `Hooks` object whose keys are hook names. OpenCode supports two hook mechanisms:

1. **Dedicated named hooks** — functions with an `(input, output)` signature where `output` is a mutable object. These hooks can observe, mutate, and in many cases block the action they wrap by throwing an error.
2. **Catch-all event bus hook** — an `event` handler that receives `{ event: { id, type, properties } }`. It is fire-and-forget and cannot block.

Plugins can also register **custom tools**, **authentication methods**, and **provider model catalog extensions** by returning `tool`, `auth`, and `provider` objects at initialization.

Capability summary:

| Capability | Supported? | Mechanism |
|---|---|---|
| Observe lifecycle | Yes | `event` hook and dedicated hooks |
| Mutate pending action | Yes | Mutate `output` object in dedicated hooks |
| Block pending action | Yes | Throw in dedicated hooks |
| Replace result | Yes | Mutate `output.output`, `output.text`, etc. |
| Inject env vars | Yes | `shell.env` hook |
| Custom tools | Yes | `tool` registration |
| Async/background | Partial | `event` hook is fire-and-forget |

## Native Hooks

### Dedicated named hooks

These hooks are called synchronously/sequentially in plugin load order. Each matching hook receives the same mutable `output` object.

| Hook | Timing | Can block? | Input | Mutable output | Notes |
|---|---|---|---|---|---|
| `config` | post | no | merged config | none | Runs once after all plugins load. |
| `dispose` | post | no | none | none | Runs on shutdown. |
| `event` | async | no | `{ event: { id, type, properties } }` | none | Fire-and-forget; filtered to plugin directory. |
| `tool` | pre | no | none | `{ [toolID]: ToolDefinition }` | Registers custom tools at load time. |
| `auth` | pre | no | none | `AuthHook` | Registers auth methods at load time. |
| `provider` | pre | no | none | `ProviderHook` | Extends provider model catalogs. |
| `chat.message` | pre | yes | `sessionID`, `agent?`, `model?`, `messageID?`, `variant?` | `{ message, parts }` | Runtime call site unconfirmed. |
| `chat.params` | pre | no | `sessionID`, `agent`, `model`, `provider`, `message` | `{ temperature, topP, topK, maxOutputTokens, options }` | Before each LLM request. |
| `chat.headers` | pre | no | `sessionID`, `agent`, `model`, `provider`, `message` | `{ headers }` | Added to provider HTTP request. |
| `permission.ask` | pre | yes | `Permission` request | `{ status: ask|deny|allow }` | Runtime call site unconfirmed. |
| `command.execute.before` | pre | yes | `command`, `sessionID`, `arguments` | `{ parts }` | Runtime call site unconfirmed. |
| `tool.execute.before` | pre | yes | `tool`, `sessionID`, `callID` | `{ args }` | Before every tool call, including MCP. |
| `shell.env` | pre | no | `cwd`, `sessionID?`, `callID?` | `{ env }` | Injects env vars into shell tool subprocess. |
| `tool.execute.after` | post | yes | `tool`, `sessionID`, `callID`, `args` | `{ title, output, metadata }` | After every tool call. |
| `experimental.chat.system.transform` | pre | no | `sessionID?`, `model` | `{ system: string[] }` | Mutate system prompts. |
| `experimental.chat.messages.transform` | pre | no | `{}` | `{ messages }` | During compaction. |
| `experimental.provider.small_model` | pre | no | `{ provider }` | `{ model? }` | Override small model selection. |
| `experimental.session.compacting` | pre | no | `{ sessionID }` | `{ context: string[], prompt?: string }` | Inject or replace compaction prompt. |
| `experimental.compaction.autocontinue` | pre | no | `sessionID`, `agent`, `model`, `provider`, `message`, `overflow` | `{ enabled }` | Defaults to `true`; set `false` to skip auto-continue. |
| `experimental.text.complete` | post | no | `sessionID`, `messageID`, `partID` | `{ text }` | Override final assistant text. |
| `tool.definition` | pre | no | `{ toolID }` | `{ description, parameters }` | Modify tool definition sent to LLM. |

### Event bus events

These are observed through the `event` hook. They are fire-and-forget and cannot block.

| Category | Events |
|---|---|
| Command | `command.executed` |
| File | `file.edited`, `file.watcher.updated` |
| Installation | `installation.updated`, `installation.update-available` |
| LSP | `lsp.updated` |
| Message | `message.updated`, `message.removed`, `message.part.updated`, `message.part.removed`, `message.part.delta` |
| Permission | `permission.asked`, `permission.replied` (and v2 variants `permission.v2.asked`, `permission.v2.replied`) |
| Server | `server.connected`, `global.disposed` |
| Session | `session.created`, `session.updated`, `session.deleted`, `session.diff`, `session.error`, `session.status`, `session.idle`, `session.compacted` |
| Session next | `session.next.agent.switched`, `session.next.model.switched`, `session.next.moved`, `session.next.prompted`, `session.next.context.updated`, `session.next.synthetic`, `session.next.shell.started/ended`, `session.next.step.started/ended/failed`, `session.next.text.started/delta/ended`, `session.next.reasoning.started/delta/ended`, `session.next.tool.input.started/delta/ended`, `session.next.tool.called`, `session.next.tool.progress`, `session.next.tool.success/failed`, `session.next.retried`, `session.next.compaction.started/delta/ended`, `session.next.revert.staged/cleared/committed` |
| Todo | `todo.updated` |
| TUI | `tui.prompt.append`, `tui.command.execute`, `tui.toast.show`, `tui.session.select` |
| MCP | `mcp.tools.changed`, `mcp.browser.open.failed` |

The schema manifest at `packages/schema/src/event-manifest.ts` is the authoritative inventory. The public docs list a stable subset.

## Configuration

### Plugin sources and load order

Plugins are loaded from:

1. **Built-in plugins** — hard-coded auth/provider plugins (Codex, Copilot, GitLab, Poe, Cloudflare, Azure, DigitalOcean, Snowflake, xAI).
2. **NPM plugins** — declared in the `plugin` array of `opencode.json` / `opencode.jsonc`.
3. **Local file plugins** — `*.{ts,js}` files in `.opencode/plugins/` (project) or `~/.config/opencode/plugins/` (global). Singular `plugin/` is also scanned.

Config precedence:

1. Remote config from `.well-known/opencode`
2. Global config `~/.config/opencode/opencode.json{c}`
3. `OPENCODE_CONFIG` custom config file
4. Project config `opencode.json{c}` (looked up from cwd to nearest git root)
5. Global and project plugin directories
6. `OPENCODE_CONFIG_CONTENT` inline JSON
7. Managed config (`/Library/Application Support/opencode/`, `/etc/opencode/`, `%ProgramData%\opencode\`)
8. macOS MDM managed preferences (`ai.opencode.managed`)

NPM plugins are installed automatically by Bun into `~/.cache/opencode/node_modules/`. Local plugin dependencies can be declared in `.opencode/package.json`.

### Hook-affecting environment variables

| Variable | Effect |
|---|---|
| `OPENCODE_PURE=true` | Do not load any external plugins. |
| `OPENCODE_DISABLE_DEFAULT_PLUGINS=true` | Skip built-in auth/provider plugins. |
| `OPENCODE_CONFIG=<file>` | Use an alternate config file. |
| `OPENCODE_CONFIG_DIR=<dir>` | Use a custom config directory. |
| `OPENCODE_CONFIG_CONTENT=<json>` | Inline config JSON. |
| `OPENCODE_EXPERIMENTAL=true` | Enables many experimental flags at once. |
| `OPENCODE_DISABLE_PROJECT_CONFIG=true` | Skip project-level `opencode.json{c}`. |

### Local plugin entrypoint

A local plugin file is a JS/TS module that default-exports (or named-exports) a function:

```ts
import type { Plugin } from "@opencode-ai/plugin"

const MyPlugin: Plugin = async ({ client, project, directory, worktree, $ }) => {
  return {
    "tool.execute.before": async (input, output) => {
      // inspect or mutate output.args
    },
    event: async ({ event }) => {
      // observe event bus
    },
  }
}

export default MyPlugin
```

V1 plugins default-export `{ server: async (input, options) => Hooks }`.

## Payloads and Responses

### Dedicated hooks

Dedicated hooks receive two positional arguments: `input` and `output`. `input` carries event-specific data; `output` is a mutable object whose shape depends on the hook.

Common input fields:

| Field | Type | Meaning |
|---|---|---|
| `sessionID` | string | Current session identifier. |
| `agent` | string | Agent name. |
| `model` | object | Provider model object. |
| `tool` | string | Tool ID. |
| `callID` | string | Tool call ID. |
| `args` | object | Tool arguments. |

Mutable output objects:

| Hook | Output fields | Effect of mutation |
|---|---|---|
| `tool.execute.before` | `args` | Changes tool input. |
| `tool.execute.after` | `title`, `output`, `metadata` | Changes tool result. |
| `shell.env` | `env` | Injects env vars into shell subprocess. |
| `chat.params` | `temperature`, `topP`, `topK`, `maxOutputTokens`, `options` | Changes LLM request params. |
| `chat.headers` | `headers` | Adds HTTP headers. |
| `experimental.chat.system.transform` | `system` | Changes system prompts. |
| `experimental.chat.messages.transform` | `messages` | Changes compaction input messages. |
| `experimental.session.compacting` | `context`, `prompt` | Appends to or replaces compaction prompt. |
| `experimental.compaction.autocontinue` | `enabled` | Controls synthetic continue turn. |
| `experimental.text.complete` | `text` | Overrides final assistant text. |
| `tool.definition` | `description`, `parameters` | Changes tool definition. |
| `permission.ask` | `status` | Grants/denies permission. |

Blocking contract:

- Throwing an error inside a dedicated hook aborts the action being wrapped.
- `tool.execute.before` aborts the tool call.
- `tool.execute.after` turns the tool result into an error.
- `chat.message` aborts message processing.
- `permission.ask` denies the permission.
- `command.execute.before` aborts command execution.
- Event bus `event` handlers cannot block.

### Event bus payload

The `event` hook receives:

```ts
{
  event: {
    id: string,
    type: string,
    properties: object
  }
}
```

`properties` is the event-specific schema data. The hook is filtered by project directory; plugins only receive events whose `event.location.directory` matches the plugin's directory.

## Execution Semantics

- **Runtime**: Plugins execute inside the Bun runtime alongside OpenCode.
- **Load order**: Plugins load in config-precedence order; hooks run sequentially in that order.
- **Shared output**: The same `output` object is passed to every plugin for a given hook invocation, so later plugins can overwrite earlier mutations.
- **Directory filtering**: The `event` hook only receives events for its own project directory.
- **No explicit timeout**: Plugin hook code runs within the timeout of the action it intercepts.
- **Stderr/errors**: Errors are logged to the OpenCode debug log and may surface as `session.error` events.
- **NPM install**: NPM plugins are installed with Bun on startup into `~/.cache/opencode/node_modules/`.

## Claudine Mapping

| OpenCode native hook/event | Claudine event | Notes |
|---|---|---|
| `config` | `initialize` | Post-load config notification. |
| `session.created` | `initialize` | New session started. |
| `session.updated` | `notification` | Session metadata changed. |
| `session.deleted` | `finalize` | Session ended/deleted. |
| `session.error` | `failure` | Session-level error. |
| `session.idle` | `finalize` | Session became idle. |
| `session.compacted` | `notification` | Context compaction completed. |
| `session.status` | `notification` | Status change event. |
| `message.updated` | `notification` | Message created/updated. |
| `message.part.updated` | `notification` | Streaming part update. |
| `permission.asked` | `permission` | Permission prompt shown. |
| `permission.replied` | `permission` | User replied to permission prompt. |
| `permission.ask` | `permission` | Dedicated permission gate (runtime unconfirmed). |
| `chat.message` | `prompt` | User message received. |
| `chat.params` / `chat.headers` | `start` | LLM request preparation. |
| `experimental.chat.system.transform` | `prompt` | System prompt mutation. |
| `tui.prompt.append` | `prompt` | Prompt input. |
| `tool.execute.before` | `tool_call` | Before tool execution. |
| `tool.execute.after` | `tool_result` | After tool execution. |
| `tool.definition` | `tool_call` | Tool definition sent to LLM. |
| `shell.env` | `tool_call` | Shell env injection. |
| `experimental.session.compacting` | `notification` | Compaction prompt hook. |
| `experimental.compaction.autocontinue` | `notification` | Post-compaction continue decision. |
| `experimental.text.complete` | `notification` | Assistant text completion. |
| `command.executed` / `tui.command.execute` | `notification` | TUI command lifecycle. |
| `file.edited` / `file.watcher.updated` | `notification` | File system events. |
| `tui.toast.show` | `notification` | Notification toast. |
| `event` catch-all | `notification` | Generic observation channel. |
| `provider` / `auth` / `tool` registrations | `none` | Capability extensions, not lifecycle events. |
| `dispose` | `finalize` | Shutdown cleanup. |

Provider-specific payload fields Claudine should preserve on the unified payload include `sessionID`, `callID`, `tool`, `agent`, `model`, `provider`, `overflow`, `variant`, and event-type discriminators.

## Gaps

1. OpenCode has no dedicated "Hooks" documentation page; event/hook docs are under `/docs/plugins/`.
2. The Hooks TypeScript interface declares `chat.message`, `permission.ask`, and `command.execute.before`, but no runtime `plugin.trigger(...)` call site for these was found in the current source.
3. Exact payload shapes for every event bus event, especially the granular `session.next.*` events, MCP events, and LSP events, are only fully available in the schema source.
4. Plugin hook error handling and timeout policies are implicit in the surrounding Effect code rather than documented.
5. Permission decisions cannot be made from the `permission.asked` event bus event; the dedicated `permission.ask` hook's invocation status is unclear.
6. The observed local Claudine bridge plugin (`~/.config/opencode/plugin/claudine-bridge.ts`) is observation-only and does not exercise mutation or blocking contracts.
7. Plugin execution order when multiple plugins mutate the same `output` field is sequential by load order, but precedence/conflict rules are not documented.

## Sources

- OpenCode homepage: <https://opencode.ai>
- OpenCode docs: <https://opencode.ai/docs>
- Plugins and events docs: <https://opencode.ai/docs/plugins/>
- Config docs: <https://opencode.ai/docs/config/>
- Agents docs: <https://opencode.ai/docs/agents/>
- Config JSON schema: <https://opencode.ai/config.json>
- GitHub repository: <https://github.com/anomalyco/opencode>
- Plugin types: `packages/plugin/src/index.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/plugin/src/index.ts>
- Plugin manager: `packages/opencode/src/plugin/index.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/plugin/index.ts>
- Plugin loader: `packages/opencode/src/plugin/loader.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/plugin/loader.ts>
- Plugin shared/entrypoint: `packages/opencode/src/plugin/shared.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/plugin/shared.ts>
- Local plugin scanning: `packages/opencode/src/config/plugin.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/plugin.ts>
- Event manifest: `packages/schema/src/event-manifest.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/schema/src/event-manifest.ts>
- Tool execution hooks: `packages/opencode/src/session/tools.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/tools.ts>
- Shell env hook: `packages/opencode/src/tool/shell.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/tool/shell.ts>
- LLM request hooks: `packages/opencode/src/session/llm/request.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/llm/request.ts>
- Compaction hooks: `packages/opencode/src/session/compaction.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/compaction.ts>
- Runtime flags/env vars: `packages/opencode/src/effect/runtime-flags.ts` — <https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/effect/runtime-flags.ts>
- Observed local plugin: `~/.config/opencode/plugin/claudine-bridge.ts`
