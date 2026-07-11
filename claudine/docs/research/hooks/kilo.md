---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
homepage: https://kilo.ai/
docs: https://kilo.ai/docs
hooks_docs: https://kilo.ai/docs/automate/extending/plugins
hooks:
  - native_event: config
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "input: resolved Kilo Config object, with plugin omitted from the public Config type except as plugin?: Array<string | [string, PluginOptions]>."
    return_contract: "Promise<void>; return value is ignored. Mutation is not documented and the docs describe this hook as read-only."
    notes: "Called once after plugin loading to inspect the fully resolved config. Source catches and logs config hook failures, so it is not a runtime blocker."
  - native_event: event
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "input.event: SDK Event object with id, type, and properties. Documented event.type examples include session, message, tool, permission, file, shell, command, LSP, todo, server, and installation events."
    return_contract: "Promise<void>; return value is ignored. The bus subscriber calls hook.event without awaiting the returned promise."
    notes: "Catch-all observer for Kilo's internal bus. It cannot mutate or block the publishing action."
  - native_event: chat.message
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input: sessionID, optional agent, optional model.providerID/model.modelID, optional messageID, optional variant. output: message and parts."
    return_contract: "Promise<void>; mutate output.message or output.parts in place to change the persisted user message. Throwing/rejecting aborts the trigger path."
    notes: "Fires after Kilo resolves prompt parts and before it validates and saves the message."
  - native_event: chat.params
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input: sessionID, agent, model, provider, message. output: temperature, topP, topK, maxOutputTokens, options."
    return_contract: "Promise<void>; mutate output fields in place. Throwing/rejecting aborts request preparation."
    notes: "Runs while preparing the LLM request, after system/messages are assembled and before provider call parameters are finalized."
  - native_event: chat.headers
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input: sessionID, agent, model, provider, message. output.headers: Record<string,string>."
    return_contract: "Promise<void>; mutate output.headers in place. Throwing/rejecting aborts request preparation."
    notes: "Runs immediately after chat.params while preparing the LLM request."
  - native_event: command.execute.before
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input: command, sessionID, arguments. output.parts: Part[]."
    return_contract: "Promise<void>; mutate output.parts in place. Throwing/rejecting aborts command handling."
    notes: "Intercepts slash command execution before the resulting parts are consumed."
  - native_event: permission.ask
    claudine_event: permission
    timing: pre
    blocking: true
    payload_schema: "input: Permission request. output.status: ask | deny | allow."
    return_contract: "Promise<void>; set output.status to allow, deny, or ask. Throwing/rejecting aborts permission evaluation."
    notes: "Auto-resolves permission prompts before user interaction. A deny blocks the requested action; allow skips prompting; ask preserves the normal prompt."
  - native_event: tool.definition
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "input.toolID. output.description, output.parameters, output.jsonSchema."
    return_contract: "Promise<void>; mutate output description/schema in place before tool definitions are sent to the model. Throwing/rejecting aborts tool registry preparation."
    notes: "The public typings document description and parameters; source also initializes jsonSchema on the output object."
  - native_event: tool.execute.before
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "input: tool, sessionID, callID. output.args: any."
    return_contract: "Promise<void>; mutate output.args in place before the tool or MCP tool executes. Throwing/rejecting prevents execution."
    notes: "Applies to built-in tools, MCP tools, and the task/subagent tool."
  - native_event: tool.execute.after
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "input: tool, sessionID, callID, args. output: title, output, metadata; source may also carry attachments/content for some tools."
    return_contract: "Promise<void>; mutate output fields in place before the result is returned to Kilo/model state. Throwing/rejecting aborts the post-processing path."
    notes: "For MCP tools, the hook sees the raw MCP result before Kilo converts content items into text, attachments, and truncation metadata."
  - native_event: shell.env
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "input: cwd, optional sessionID, optional callID. output.env: Record<string,string>."
    return_contract: "Promise<void>; mutate output.env in place. Throwing/rejecting aborts shell environment assembly."
    notes: "Runs before agent shell commands and PTY shell startup; returned values are merged over process.env for shell tool execution."
  - native_event: experimental.chat.messages.transform
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input: empty object. output.messages: Array<{ info: Message, parts: Part[] }>."
    return_contract: "Promise<void>; mutate output.messages in place before conversion to model messages. Throwing/rejecting aborts the current prompt or compaction path."
    notes: "Experimental. Runs before ordinary model requests and compaction summary requests; can rewrite full history."
  - native_event: experimental.chat.system.transform
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input: optional sessionID, model. output.system: string[]."
    return_contract: "Promise<void>; mutate output.system in place before system messages are sent. Throwing/rejecting aborts request preparation."
    notes: "Experimental. Runs during LLM request preparation after Kilo assembles base system text."
  - native_event: experimental.session.compacting
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input.sessionID. output.context: string[], output.prompt?: string."
    return_contract: "Promise<void>; append context or set output.prompt to replace Kilo's compaction prompt. Throwing/rejecting aborts compaction."
    notes: "Experimental. Fires before session compaction builds the summary prompt."
  - native_event: experimental.compaction.autocontinue
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "input: sessionID, agent, model, provider, message, overflow. output.enabled: boolean."
    return_contract: "Promise<void>; set output.enabled=false to stop the synthetic continue turn. Throwing/rejecting aborts the auto-continue decision."
    notes: "Experimental. Fires after compaction succeeds, before Kilo adds its synthetic user continue message."
  - native_event: experimental.text.complete
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "input: sessionID, messageID, partID. output.text: string."
    return_contract: "Promise<void>; mutate output.text to rewrite the completed assistant text part. Throwing/rejecting aborts text-end processing."
    notes: "Experimental. Fires at text-end for final assistant text parts."
config_files:
  - os: macos
    scope: user
    path: ~/.config/kilo/kilo.jsonc
    format: jsonc
    notes: "Primary global config documented by Kilo settings. Also supports kilo.json, opencode.jsonc, opencode.json, and config.json in the same directory."
  - os: linux
    scope: user
    path: ~/.config/kilo/kilo.jsonc
    format: jsonc
    notes: "Primary global config documented by Kilo settings. Respects XDG_CONFIG_HOME through the runtime global path layer."
  - os: windows
    scope: user
    path: C:\Users\<username>\.config\kilo\kilo.jsonc
    format: jsonc
    notes: "Primary Windows global config path documented by Kilo settings."
  - os: macos
    scope: repo
    path: ./.kilo/kilo.jsonc
    format: jsonc
    notes: "Project config directory. Plugin arrays here can register hooks; .kilocode is supported as legacy."
  - os: linux
    scope: repo
    path: ./.kilo/kilo.jsonc
    format: jsonc
    notes: "Project config directory. Plugin arrays here can register hooks; .kilocode is supported as legacy."
  - os: windows
    scope: repo
    path: .\.kilo\kilo.jsonc
    format: jsonc
    notes: "Project config directory. Plugin arrays here can register hooks; .kilocode is supported as legacy."
  - os: macos
    scope: repo
    path: ./kilo.jsonc
    format: jsonc
    notes: "Project-root config; plugin arrays here can register hooks."
  - os: linux
    scope: repo
    path: ./kilo.jsonc
    format: jsonc
    notes: "Project-root config; plugin arrays here can register hooks."
  - os: windows
    scope: repo
    path: .\kilo.jsonc
    format: jsonc
    notes: "Project-root config; plugin arrays here can register hooks."
  - os: macos
    scope: user
    path: ~/.config/kilo/plugin/
    format: other
    notes: "Directory of auto-loaded .ts/.js server plugin files. The docs also allow plugins/."
  - os: linux
    scope: user
    path: ~/.config/kilo/plugin/
    format: other
    notes: "Directory of auto-loaded .ts/.js server plugin files. The docs also allow plugins/."
  - os: windows
    scope: user
    path: C:\Users\<username>\.config\kilo\plugin\
    format: other
    notes: "Directory of auto-loaded .ts/.js server plugin files. The docs also allow plugins/."
  - os: macos
    scope: repo
    path: ./.kilo/plugin/
    format: other
    notes: "Directory of auto-loaded project .ts/.js server plugin files. Legacy .kilocode/plugin/ is supported."
  - os: linux
    scope: repo
    path: ./.kilo/plugin/
    format: other
    notes: "Directory of auto-loaded project .ts/.js server plugin files. Legacy .kilocode/plugin/ is supported."
  - os: windows
    scope: repo
    path: .\.kilo\plugin\
    format: other
    notes: "Directory of auto-loaded project .ts/.js server plugin files. Legacy .kilocode\\plugin\\ is supported."
  - os: macos
    scope: other
    path: $KILO_CONFIG_DIR/
    format: other
    notes: "Runtime env-provided extra config directory; source includes it in config-directory discovery."
  - os: linux
    scope: other
    path: $KILO_CONFIG_DIR/
    format: other
    notes: "Runtime env-provided extra config directory; source includes it in config-directory discovery."
  - os: windows
    scope: other
    path: "%KILO_CONFIG_DIR%\\"
    format: other
    notes: "Runtime env-provided extra config directory; source includes it in config-directory discovery."
cli_params:
  - flag: kilo plugin <specifier>
    description: "Installs an npm plugin and patches the current project's plugin config."
    example: "kilo plugin my-plugin"
  - flag: kilo plugin <specifier> --global
    description: "Installs an npm plugin and patches global plugin config."
    example: "kilo plugin my-plugin --global"
  - flag: kilo plugin <specifier> --force
    description: "Replaces an existing plugin entry while installing."
    example: "kilo plugin my-plugin --force"
  - flag: --pure
    description: "CLI global option that sets KILO_PURE=1 and disables external plugins."
    example: "kilo --pure run \"inspect\""
payload_fields:
  - native_event: event
    field: event.id
    type: string
    meaning: "Internal bus event identifier."
  - native_event: event
    field: event.type
    type: string
    meaning: "Bus event name such as session.created, session.idle, message.updated, permission.asked, file.edited, command.executed, server.connected, or installation.updated."
  - native_event: event
    field: event.properties
    type: object
    meaning: "Event-specific payload. Shape depends on event.type."
  - native_event: chat.message
    field: input.sessionID
    type: string
    meaning: "Session receiving the user message."
  - native_event: chat.message
    field: input.agent
    type: string | undefined
    meaning: "Agent/mode selected for the message."
  - native_event: chat.message
    field: input.model.providerID
    type: string | undefined
    meaning: "Selected model provider when available."
  - native_event: chat.message
    field: input.model.modelID
    type: string | undefined
    meaning: "Selected model ID when available."
  - native_event: chat.message
    field: input.messageID
    type: string | undefined
    meaning: "Message correlation ID."
  - native_event: chat.message
    field: output.message
    type: UserMessage
    meaning: "Mutable message info persisted after the hook."
  - native_event: chat.message
    field: output.parts
    type: Part[]
    meaning: "Mutable user message parts persisted after the hook."
  - native_event: chat.params
    field: output.temperature
    type: number | undefined
    meaning: "Mutable LLM sampling temperature."
  - native_event: chat.params
    field: output.topP
    type: number
    meaning: "Mutable nucleus sampling value."
  - native_event: chat.params
    field: output.topK
    type: number
    meaning: "Mutable top-k sampling value."
  - native_event: chat.params
    field: output.maxOutputTokens
    type: number | undefined
    meaning: "Mutable output token cap."
  - native_event: chat.params
    field: output.options
    type: Record<string, any>
    meaning: "Mutable provider-specific options."
  - native_event: chat.headers
    field: output.headers
    type: Record<string, string>
    meaning: "Mutable HTTP headers added to the model provider call."
  - native_event: permission.ask
    field: input.id
    type: string
    meaning: "Permission request ID."
  - native_event: permission.ask
    field: input.sessionID
    type: string
    meaning: "Session requesting permission."
  - native_event: permission.ask
    field: input.permission
    type: string
    meaning: "Permission category/tool name being evaluated."
  - native_event: permission.ask
    field: input.patterns
    type: string[]
    meaning: "Patterns under evaluation for the permission request."
  - native_event: permission.ask
    field: input.metadata
    type: Record<string, unknown>
    meaning: "Provider metadata for display, security decisions, and tool context."
  - native_event: permission.ask
    field: input.always
    type: string[]
    meaning: "Patterns eligible for persistent approval."
  - native_event: permission.ask
    field: input.tool.messageID
    type: string | undefined
    meaning: "Tool message correlation ID when the permission belongs to a tool call."
  - native_event: permission.ask
    field: input.tool.callID
    type: string | undefined
    meaning: "Tool call correlation ID when the permission belongs to a tool call."
  - native_event: permission.ask
    field: output.status
    type: ask | deny | allow
    meaning: "Mutable permission decision."
  - native_event: command.execute.before
    field: input.command
    type: string
    meaning: "Slash command name."
  - native_event: command.execute.before
    field: input.arguments
    type: string
    meaning: "Slash command argument string."
  - native_event: command.execute.before
    field: output.parts
    type: Part[]
    meaning: "Mutable parts produced by the command."
  - native_event: tool.definition
    field: input.toolID
    type: string
    meaning: "Tool definition being prepared for the model."
  - native_event: tool.definition
    field: output.description
    type: string
    meaning: "Mutable tool description."
  - native_event: tool.definition
    field: output.parameters
    type: any
    meaning: "Mutable tool parameter schema object."
  - native_event: tool.definition
    field: output.jsonSchema
    type: any
    meaning: "Mutable JSON Schema observed in source, though not in public typings."
  - native_event: tool.execute.before
    field: input.tool
    type: string
    meaning: "Tool name about to execute."
  - native_event: tool.execute.before
    field: input.sessionID
    type: string
    meaning: "Session containing the tool call."
  - native_event: tool.execute.before
    field: input.callID
    type: string
    meaning: "Tool call correlation ID."
  - native_event: tool.execute.before
    field: output.args
    type: any
    meaning: "Mutable tool arguments."
  - native_event: tool.execute.after
    field: input.args
    type: any
    meaning: "Arguments used for the completed tool call."
  - native_event: tool.execute.after
    field: output.title
    type: string
    meaning: "Mutable display title for the tool result."
  - native_event: tool.execute.after
    field: output.output
    type: string
    meaning: "Mutable textual result returned to Kilo/model state."
  - native_event: tool.execute.after
    field: output.metadata
    type: any
    meaning: "Mutable result metadata."
  - native_event: shell.env
    field: input.cwd
    type: string
    meaning: "Working directory for the shell or PTY."
  - native_event: shell.env
    field: output.env
    type: Record<string, string>
    meaning: "Environment variables merged into shell execution."
  - native_event: experimental.chat.messages.transform
    field: output.messages[].info
    type: Message
    meaning: "Mutable message metadata in the outgoing history."
  - native_event: experimental.chat.messages.transform
    field: output.messages[].parts
    type: Part[]
    meaning: "Mutable message parts in the outgoing history."
  - native_event: experimental.chat.system.transform
    field: output.system
    type: string[]
    meaning: "Mutable system prompt array."
  - native_event: experimental.session.compacting
    field: output.context
    type: string[]
    meaning: "Extra context appended to the default compaction prompt."
  - native_event: experimental.session.compacting
    field: output.prompt
    type: string | undefined
    meaning: "Replacement compaction prompt when set."
  - native_event: experimental.compaction.autocontinue
    field: input.overflow
    type: boolean
    meaning: "Whether compaction was triggered by context overflow."
  - native_event: experimental.compaction.autocontinue
    field: output.enabled
    type: boolean
    meaning: "Whether Kilo should add the synthetic continue message."
  - native_event: experimental.text.complete
    field: output.text
    type: string
    meaning: "Mutable completed assistant text."
response_actions:
  - action: continue
    native_value: "Promise<void> resolves without mutating output"
    effect: "Kilo continues with the provider's original data."
  - action: modify
    native_value: "Promise<void> resolves after mutating output.*"
    effect: "Kilo continues with mutated message parts, tool args/results, LLM params, headers, environment variables, system text, or compaction data."
  - action: allow
    native_value: "permission.ask output.status = \"allow\""
    effect: "Permission is granted without asking the user."
  - action: deny
    native_value: "permission.ask output.status = \"deny\""
    effect: "Permission is denied and the requested action is blocked."
  - action: ask
    native_value: "permission.ask output.status = \"ask\""
    effect: "Kilo keeps the normal user permission prompt path."
  - action: replace
    native_value: "provider.models returns Record<string, ModelV2>"
    effect: "Provider model catalog is supplied or replaced by the provider hook."
  - action: stop
    native_value: "experimental.compaction.autocontinue output.enabled = false"
    effect: "Kilo does not add the synthetic continue turn after compaction."
  - action: block
    native_value: "Hook throws or returned Promise rejects"
    effect: "Awaited trigger paths fail before the provider action continues. The config hook catches/logs failures and the event hook is not awaited."
execution:
  shell: "No shell hook runner. Hooks are TypeScript/JavaScript plugin functions executed in Kilo's Bun/Node runtime. Plugin context exposes Bun.$ when Bun is available."
  cwd: "Plugin input includes directory and worktree. The BunShell helper can set cwd; shell.env receives the command cwd."
  env: "Plugin functions inherit the Kilo process environment. shell.env can inject env vars for agent/user shell commands. KILO_PURE=1 skips external plugins; KILO_DISABLE_DEFAULT_PLUGINS skips internal plugins; KILO_CONFIG, KILO_CONFIG_DIR, KILO_CONFIG_CONTENT, and KILO_DISABLE_PROJECT_CONFIG alter config/plugin discovery."
  timeout: "No per-hook timeout was found in public docs or source. Shell tool timeout defaults to 120000 ms and can be changed with KILO_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS, but that is for shell commands, not hook execution."
  stdin: "No stdin contract for hooks. Hooks receive typed input/output arguments."
  stdout: "No stdout response protocol. Plugins may log to process stdout/stderr, but hook effects are through output-object mutation or thrown errors."
  stderr: "No stderr response protocol. Hook load/config/dispose errors are logged; awaited hook errors propagate through the triggering provider path."
  notes: "External plugins load after internal built-ins, global config plugin array, global plugin directory, project config plugin array, and project plugin directories. Named hooks run sequentially in load order and are awaited. The catch-all event hook is fire-and-forget. Local evidence on 2026-07-03: /Users/ken/.kilo was absent; /Users/ken/.config/kilo/kilo.jsonc and /Users/ken/.claudine/.config/kilo/kilo.jsonc contained only the schema URL and no plugin/hook entries; /Users/ken/.config/kilo/node_modules/@kilocode/plugin supplied installed public typings."
gaps:
  - "No official page gives exhaustive payload schemas for every internal bus event delivered through the catch-all event hook; SDK generated typings expose many event shapes, but the docs list only common examples."
  - "No provider-native shell command, HTTP endpoint, or LLM-evaluator hook runner was found; Kilo's public hook mechanism is plugin functions."
  - "No per-hook timeout, cancellation, or stderr/stdout display contract was found for plugin hooks."
  - "The public docs say plugin directories can be named plugin/ or plugins/, while the inspected config-path source uses config directories and plugin loader code was not fully traced to confirm both directory names on every OS."
  - "The public typings for tool.definition omit output.jsonSchema, but the inspected source initializes and consumes it."
changes: []
requires_claudine_update: true
reason: "Kilo's hook model is plugin-function based and includes mutable pre/post hooks plus async bus events; Claudine needs generated provider metadata and an adapter design distinct from shell-hook providers before Kilo can be supported."
---

# Kilo Code Hook and Event Semantics

## Overview

Kilo Code exposes hooks through TypeScript or JavaScript plugins, not through a Claude-style `hooks.json` shell runner. A plugin is a module whose `server` function receives Kilo runtime context and returns a `Hooks` object. Hook handlers are ordinary async functions. They are loaded for both the CLI and VS Code extension.

The public hook system supports these handler kinds:

- Plugin functions for lifecycle, chat, permission, tool, provider/auth, shell environment, and experimental compaction/text transforms.
- Custom tools registered by plugins, which are model-callable tools rather than lifecycle hooks.
- Provider/auth hooks, which can add auth flows and model catalogs but are not direct lifecycle events.

The main capability model is mutable objects. Most named hooks receive an immutable `input` object and a mutable `output` object. Kilo awaits each hook sequentially in plugin load order, then continues with the final mutated output. These hooks can mutate pending actions, replace parts of requests/results, and block by throwing or returning a rejected promise. `permission.ask` has an explicit decision field: `allow`, `deny`, or `ask`.

The catch-all `event` hook is different. It subscribes to Kilo's internal event bus and receives `{ event }`, but source calls it without awaiting the returned promise. It is an async observer and cannot block or mutate the event being published.

Local host inspection on 2026-07-03 found no `/Users/ken/.kilo` directory. The active user config file at `/Users/ken/.config/kilo/kilo.jsonc` and Claudine shadow config at `/Users/ken/.claudine/.config/kilo/kilo.jsonc` both contained only:

```json
{
  "$schema": "https://app.kilo.ai/config.json"
}
```

No local hook/plugin entries or local plugin scripts were installed in those config files. The host did have installed package typings under `/Users/ken/.config/kilo/node_modules/@kilocode/plugin/`, which matched the source hook interface.

## Native Hooks

### `config`

- Timing: `pre`
- Blocking: no for normal provider execution. Source catches and logs this hook's failures.
- Mutation: no documented mutation. The docs call it read-only.
- Matcher/filter: none.

`config` receives the fully resolved Kilo config at startup. It is useful for inspection and diagnostics after plugin loading. It does not configure a provider action directly.

### `event`

- Timing: `async`
- Blocking: no.
- Mutation: no.
- Matcher/filter: none in Kilo. Plugins filter by checking `event.type`.

`event` receives every event on Kilo's internal bus as `{ event }`. The docs list common event types:

- Session: `session.created`, `session.updated`, `session.idle`, `session.error`, `session.deleted`, `session.compacted`, `session.diff`, `session.status`
- Message: `message.updated`, `message.removed`, `message.part.updated`, `message.part.removed`
- Tool: `tool.execute.before`, `tool.execute.after`
- Permission: `permission.asked`, `permission.replied`
- File: `file.edited`, `file.watcher.updated`
- Shell: `shell.env`
- Command: `command.executed`
- LSP: `lsp.updated`, `lsp.client.diagnostics`
- Todo: `todo.updated`
- Server: `server.connected`
- Installation: `installation.updated`

Generated SDK typings and source show additional bus events, including `message.part.delta`, `question.*`, `session.network.*`, `session.turn.open`, `session.turn.close`, background-process events, interactive-terminal events, workspace/worktree events, and suggestion events. These are observable through `event` when published, but they are not individually configured as plugin hook names.

### `chat.message`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, `output.message` and `output.parts`.
- Matcher/filter: none.

`chat.message` fires when a new user message arrives, after Kilo resolves the incoming parts and before it validates and saves the message. A plugin can inspect or rewrite the persisted message and parts.

### `chat.params`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, LLM request parameters.
- Matcher/filter: none.

`chat.params` fires during LLM request preparation. It can mutate `temperature`, `topP`, `topK`, `maxOutputTokens`, and provider-specific `options`.

### `chat.headers`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, HTTP headers.
- Matcher/filter: none.

`chat.headers` fires during LLM request preparation after `chat.params`. It can add or replace headers for the model provider request.

### `permission.ask`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, decision field.
- Matcher/filter: none.

`permission.ask` receives a permission request and mutable `{ status }`. Setting `status` to `allow` grants the action without a user prompt. Setting `deny` blocks the action. Leaving or setting `ask` preserves the ordinary prompt.

### `command.execute.before`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, command result parts.
- Matcher/filter: none.

`command.execute.before` intercepts slash command execution and can mutate the `parts` that the command contributes to the session.

### `tool.definition`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, tool description/schema.
- Matcher/filter: none.

`tool.definition` runs while Kilo prepares tool definitions for the model. It can mutate the tool `description` and `parameters`. Source also initializes `jsonSchema` in the output object and consumes it after the hook, although the installed public typings only document `description` and `parameters`.

### `tool.execute.before`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, tool arguments.
- Matcher/filter: none.

`tool.execute.before` fires before a built-in tool, MCP tool, or task/subagent tool executes. It can mutate `output.args`. If a hook throws, the pending tool execution does not continue.

### `tool.execute.after`

- Timing: `post`
- Blocking: yes.
- Mutation: yes, tool result.
- Matcher/filter: none.

`tool.execute.after` fires after a tool returns and before Kilo finishes result processing. It can rewrite `output.title`, `output.output`, and `output.metadata`. For MCP tools, the hook is called before Kilo converts MCP content into final text, attachments, and truncation metadata.

### `shell.env`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, environment variables.
- Matcher/filter: none.

`shell.env` injects environment variables into shell commands Kilo runs. Source call sites include the shell tool, shell restoration inside prompt flow, PTY creation, and interactive-terminal tooling. Returned env values are merged into the shell execution environment.

### `experimental.chat.messages.transform`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, complete message history.
- Matcher/filter: none.

This experimental hook rewrites the message history before it is sent to the model. Source calls it in ordinary prompt flow and during compaction summary generation.

### `experimental.chat.system.transform`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, system prompt array.
- Matcher/filter: none.

This experimental hook mutates the system prompt array during LLM request preparation.

### `experimental.session.compacting`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, compaction prompt/context.
- Matcher/filter: none.

This experimental hook fires before session compaction builds its summary prompt. It can append extra `context` strings or set `prompt` to replace the default compaction prompt.

### `experimental.compaction.autocontinue`

- Timing: `pre`
- Blocking: yes.
- Mutation: yes, auto-continue decision.
- Matcher/filter: none.

This experimental hook fires after compaction succeeds and before Kilo adds the synthetic user "continue" turn. Setting `output.enabled` to `false` stops that synthetic turn.

### `experimental.text.complete`

- Timing: `post`
- Blocking: yes.
- Mutation: yes, assistant text.
- Matcher/filter: none.

This experimental hook fires when an assistant text part ends. It can rewrite the final text before Kilo stores and publishes the text-end state.

## Configuration

Hooks are configured by loading plugins. There is no standalone native `hooks.json` file found in docs or source.

On macOS:

- User config: `~/.config/kilo/kilo.jsonc`; also supports `kilo.json`, `opencode.jsonc`, `opencode.json`, and `config.json` in the same directory.
- User plugin directory: `~/.config/kilo/plugin/`; docs also say `plugins/`.
- Project config: `./kilo.jsonc`, `./kilo.json`, `./.kilo/kilo.jsonc`, `./.kilo/kilo.json`; legacy `.kilocode/` directories are supported.
- Project plugin directory: `./.kilo/plugin/`; legacy `./.kilocode/plugin/` is supported.

On Linux:

- User config: `~/.config/kilo/kilo.jsonc`; the runtime respects XDG-style global paths, so `XDG_CONFIG_HOME` can move this root.
- User plugin directory: `~/.config/kilo/plugin/`; docs also say `plugins/`.
- Project config: `./kilo.jsonc`, `./kilo.json`, `./.kilo/kilo.jsonc`, `./.kilo/kilo.json`; legacy `.kilocode/` directories are supported.
- Project plugin directory: `./.kilo/plugin/`; legacy `./.kilocode/plugin/` is supported.

On Windows:

- User config: `C:\Users\<username>\.config\kilo\kilo.jsonc`, as documented by Kilo settings.
- User plugin directory: `C:\Users\<username>\.config\kilo\plugin\`; docs also say `plugins/`.
- Project config: `.\kilo.jsonc`, `.\kilo.json`, `.\.kilo\kilo.jsonc`, `.\.kilo\kilo.json`; legacy `.\.kilocode\` directories are supported.
- Project plugin directory: `.\.kilo\plugin\`; legacy `.\.kilocode\plugin\` is supported.

Plugin configuration can be an array in config:

```json
{
  "$schema": "https://app.kilo.ai/config.json",
  "plugin": [
    "@your-org/your-plugin",
    "your-plugin@1.2.3",
    ["your-plugin", { "apiKey": "{env:MY_API_KEY}" }],
    "./plugins/local.ts",
    "file:///abs/path/plugin.ts"
  ]
}
```

The `kilo plugin` command installs npm plugins and patches config:

- `kilo plugin my-plugin`
- `kilo plugin my-plugin --global`
- `kilo plugin my-plugin --force`

The docs say the command writes server plugin entries to `.kilo/opencode.jsonc` or `~/.config/kilo/opencode.jsonc`, preserving JSONC comments.

Hook-affecting environment variables observed in docs or source:

- `KILO_PURE=1`: skips all external plugins; built-in internal plugins still load.
- `--pure`: CLI option that sets `KILO_PURE=1`.
- `KILO_DISABLE_DEFAULT_PLUGINS`: source runtime flag that skips internal built-in plugins.
- `KILO_CONFIG`: source runtime flag that loads an explicit config file.
- `KILO_CONFIG_DIR`: source runtime flag that adds an extra config directory and also affects instruction discovery.
- `KILO_CONFIG_CONTENT`: source runtime path for inline config content.
- `KILO_DISABLE_PROJECT_CONFIG`: source runtime flag that skips project config discovery.
- `XDG_CONFIG_HOME`: affects the global config root on Unix-like systems through the runtime global path layer.

## Payloads and Responses

Kilo's native response contract is not stdout JSON or process exit codes. Hook handlers are async functions. For named hooks, the native value is the returned promise plus mutations to the provided output object.

The general response contract is:

- Promise resolves and output is unchanged: allow/continue with original data.
- Promise resolves after output mutation: continue with modified data.
- Promise rejects or handler throws: awaited trigger paths fail and the provider action does not continue normally.
- `event` hook promise: not awaited; return value and rejection do not block the bus publisher.
- `config` hook promise: awaited during notification, but failures are caught and logged.

Meaningful payload fields:

| Hook | Field | Meaning |
|---|---|---|
| `event` | `event.id` | Internal bus event identifier. |
| `event` | `event.type` | Event name. Plugins route on this field. |
| `event` | `event.properties` | Event-specific payload. |
| `chat.message` | `input.sessionID` | Session receiving the user message. |
| `chat.message` | `input.agent` | Agent/mode, when present. |
| `chat.message` | `input.model.providerID` | Provider ID, when present. |
| `chat.message` | `input.model.modelID` | Model ID, when present. |
| `chat.message` | `input.messageID` | Message correlation ID, when present. |
| `chat.message` | `input.variant` | Model/message variant, when present. |
| `chat.message` | `output.message` | Mutable `UserMessage`. |
| `chat.message` | `output.parts` | Mutable `Part[]`. |
| `chat.params` | `input.sessionID` | Session preparing a model call. |
| `chat.params` | `input.agent` | Agent name. |
| `chat.params` | `input.model` | Full model metadata. |
| `chat.params` | `input.provider.source` | Provider source: `env`, `config`, `custom`, or `api`. |
| `chat.params` | `input.provider.info` | Provider metadata. |
| `chat.params` | `input.provider.options` | Provider options. |
| `chat.params` | `input.message` | User message driving the call. |
| `chat.params` | `output.temperature` | Mutable sampling temperature. |
| `chat.params` | `output.topP` | Mutable top-p. |
| `chat.params` | `output.topK` | Mutable top-k. |
| `chat.params` | `output.maxOutputTokens` | Mutable output token cap. |
| `chat.params` | `output.options` | Mutable provider options. |
| `chat.headers` | `output.headers` | Mutable model request headers. |
| `permission.ask` | `input.id` | Permission request ID. |
| `permission.ask` | `input.sessionID` | Session requesting permission. |
| `permission.ask` | `input.permission` | Permission category/tool. |
| `permission.ask` | `input.patterns` | Requested permission patterns. |
| `permission.ask` | `input.metadata` | Request metadata for display/security decisions. |
| `permission.ask` | `input.always` | Patterns eligible for persistent approval. |
| `permission.ask` | `input.tool.messageID` | Tool message ID, when present. |
| `permission.ask` | `input.tool.callID` | Tool call ID, when present. |
| `permission.ask` | `output.status` | Native decision: `ask`, `deny`, or `allow`. |
| `command.execute.before` | `input.command` | Slash command name. |
| `command.execute.before` | `input.sessionID` | Session executing the command. |
| `command.execute.before` | `input.arguments` | Command argument string. |
| `command.execute.before` | `output.parts` | Mutable command output parts. |
| `tool.definition` | `input.toolID` | Tool definition ID. |
| `tool.definition` | `output.description` | Mutable description sent to the model. |
| `tool.definition` | `output.parameters` | Mutable parameter schema. |
| `tool.definition` | `output.jsonSchema` | Mutable JSON Schema observed in source. |
| `tool.execute.before` | `input.tool` | Tool name about to execute. |
| `tool.execute.before` | `input.sessionID` | Session containing the tool call. |
| `tool.execute.before` | `input.callID` | Tool call ID. |
| `tool.execute.before` | `output.args` | Mutable tool args. |
| `tool.execute.after` | `input.tool` | Completed tool name. |
| `tool.execute.after` | `input.sessionID` | Session containing the tool call. |
| `tool.execute.after` | `input.callID` | Tool call ID. |
| `tool.execute.after` | `input.args` | Args used for execution. |
| `tool.execute.after` | `output.title` | Mutable display title. |
| `tool.execute.after` | `output.output` | Mutable textual output. |
| `tool.execute.after` | `output.metadata` | Mutable result metadata. |
| `shell.env` | `input.cwd` | Shell working directory. |
| `shell.env` | `input.sessionID` | Session ID, when shell belongs to a session. |
| `shell.env` | `input.callID` | Tool call ID, when shell belongs to a tool call. |
| `shell.env` | `output.env` | Mutable env var map merged into command execution. |
| `experimental.chat.messages.transform` | `output.messages[].info` | Mutable message metadata in outgoing history. |
| `experimental.chat.messages.transform` | `output.messages[].parts` | Mutable message parts in outgoing history. |
| `experimental.chat.system.transform` | `input.sessionID` | Session ID, when available. |
| `experimental.chat.system.transform` | `input.model` | Model metadata. |
| `experimental.chat.system.transform` | `output.system` | Mutable system prompt array. |
| `experimental.session.compacting` | `input.sessionID` | Session being compacted. |
| `experimental.session.compacting` | `output.context` | Extra context appended to default compaction prompt. |
| `experimental.session.compacting` | `output.prompt` | Replacement prompt when set. |
| `experimental.compaction.autocontinue` | `input.sessionID` | Compacted session. |
| `experimental.compaction.autocontinue` | `input.agent` | Agent from the user message. |
| `experimental.compaction.autocontinue` | `input.model` | Model metadata. |
| `experimental.compaction.autocontinue` | `input.provider` | Provider context. |
| `experimental.compaction.autocontinue` | `input.message` | User message. |
| `experimental.compaction.autocontinue` | `input.overflow` | Whether compaction was overflow-triggered. |
| `experimental.compaction.autocontinue` | `output.enabled` | Whether to add synthetic continue. |
| `experimental.text.complete` | `input.sessionID` | Session containing text. |
| `experimental.text.complete` | `input.messageID` | Assistant message ID. |
| `experimental.text.complete` | `input.partID` | Text part ID. |
| `experimental.text.complete` | `output.text` | Mutable final assistant text. |

Provider-native response values:

| Native value | Effect |
|---|---|
| `Promise<void>` resolves | Continue. |
| Mutated `output.args` | Replace pending tool args before execution. |
| Mutated `output.parts` | Replace or modify pending message/command parts. |
| Mutated `output.output`, `output.title`, or `output.metadata` | Rewrite tool result. |
| Mutated `output.headers` | Add/replace LLM request headers. |
| Mutated `output.temperature`, `output.topP`, `output.topK`, `output.maxOutputTokens`, `output.options` | Rewrite LLM request parameters. |
| Mutated `output.env` | Inject shell command environment variables. |
| `output.status = "allow"` | Allow permission without asking the user. |
| `output.status = "deny"` | Deny permission and block the action. |
| `output.status = "ask"` | Preserve normal permission prompt behavior. |
| `output.prompt = "..."` | Replace compaction prompt. |
| `output.enabled = false` | Stop synthetic continue after compaction. |
| `output.text = "..."` | Replace final assistant text part. |
| Hook throws or rejects | Block/fail the awaited provider path, except `config` failures are caught and `event` is not awaited. |

## Execution Semantics

Kilo hook execution is in-process plugin execution. There is no discovered shell-command hook runner, HTTP hook endpoint runner, LLM evaluator hook runner, process exit-code contract, or stdout/stderr JSON response protocol.

External plugins are loaded after internal built-ins. The documented order is:

1. Internal built-ins.
2. Global config plugin array.
3. Global plugin directory.
4. Project config plugin array.
5. Project plugin directory.

Duplicate package/version entries are deduplicated. Source explicitly keeps plugin execution sequential so registration and hook execution order are deterministic. The named hook trigger loops through hooks in order and awaits each handler. Later hooks see mutations made by earlier hooks because they share the same mutable output object.

The catch-all `event` hook is subscribed to the internal bus. Source loops over hooks and calls `void hook["event"]?.({ event })`; it does not await promises. Treat it as fire-and-forget observation.

The plugin context includes:

- `client`: Kilo SDK client.
- `project`: current project.
- `directory`: current project directory.
- `worktree`: current worktree root.
- `serverUrl`: local server URL.
- `$`: Bun shell helper when Bun is available.
- `experimental_workspace.register(...)`: workspace adapter registration.

No hook-specific timeout was found. Shell commands run by the shell tool have their own default timeout of 120 seconds, controlled by runtime flags and `KILO_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS`, but that does not bound plugin hook execution.

Platform caveats:

- Kilo plugins are TypeScript/JavaScript modules. Local plugin portability depends on runtime APIs and imports used by the plugin author.
- Windows paths in config are distinct from Unix-like paths; docs explicitly use `C:\Users\<username>\.config\kilo\kilo.jsonc`.
- Source and docs both retain OpenCode-compatible names such as `opencode.jsonc`, while Kilo-specific config prefers `kilo.jsonc`/`kilo.json`.
- Plugin hooks are not covered by Kilo's sandbox/network restriction docs; Kilo sandboxing docs state local MCP servers and plugin hooks are outside the restriction.

## Claudine Mapping

| Kilo native event | Claudine event | Notes |
|---|---|---|
| `config` | `initialize` | Startup inspection hook. No mutation/blocking contract useful for provider lifecycle beyond initialization. |
| `event` | `notification` | Catch-all async bus observer. Preserve `event.type`, `event.id`, and `event.properties`. |
| `chat.message` | `prompt` | Pre-persistence user message hook. Preserve `agent`, `model`, `variant`, `messageID`, and mutable `parts`. |
| `chat.params` | `prompt` | Pre-LLM request parameter mutation. Preserve provider/model/options. |
| `chat.headers` | `prompt` | Pre-LLM request header mutation. Preserve provider/model/options and headers. |
| `command.execute.before` | `prompt` | Slash commands produce prompt parts. Preserve command and argument string. |
| `permission.ask` | `permission` | Native decision maps directly to allow/deny/ask. Preserve permission, patterns, metadata, always, and tool IDs. |
| `tool.definition` | `tool_call` | Pre-model tool definition mutation, not a tool execution. Claudine should distinguish definition-time mutation from call-time mutation. |
| `tool.execute.before` | `tool_call` | Pre-execution mutation/blocking for built-in, MCP, and task/subagent tools. Preserve tool, sessionID, callID, args. |
| `tool.execute.after` | `tool_result` | Post-execution result mutation. Preserve title/output/metadata and any source-only attachments/content when present. |
| `shell.env` | `tool_call` | Pre-shell environment injection. Preserve cwd/sessionID/callID. |
| `experimental.chat.messages.transform` | `prompt` | Full-history pre-request rewrite. Many-to-one collision with `chat.message`, `chat.params`, and `chat.headers`. |
| `experimental.chat.system.transform` | `prompt` | System prompt rewrite. Preserve system array and model. |
| `experimental.session.compacting` | `prompt` | Compaction prompt rewrite. Claudine has no exact compaction lifecycle event; use `prompt` until a compaction event exists. |
| `experimental.compaction.autocontinue` | `prompt` | Stops synthetic continue, which Claudine cannot fully model today except as provider-specific flow control. |
| `experimental.text.complete` | `tool_result` | Closest existing event is `tool_result`, but semantically this is assistant text completion. Claudine may need a text-output completion event to avoid overloading tool results. |

Many Kilo hooks collapse into Claudine `prompt`. A Kilo adapter must preserve provider-specific subphase names or Claudine will lose ordering and mutation semantics:

- `chat.message`: user message and parts before persistence.
- `experimental.chat.messages.transform`: full outgoing history.
- `experimental.chat.system.transform`: system prompt array.
- `chat.params`: model parameters.
- `chat.headers`: provider HTTP headers.
- `experimental.session.compacting`: compaction prompt construction.
- `experimental.compaction.autocontinue`: synthetic continuation control.

`tool.execute.before` covers normal tools, MCP tools, and the task/subagent tool. Claudine should preserve `tool` names and not infer subagent lifecycle solely from the hook name; the task tool's arguments include subagent-shaped fields such as `subagent_type`, `prompt`, `description`, and `command`.

## Gaps

- No official page gives exhaustive payload schemas for every internal bus event delivered through the catch-all `event` hook. SDK generated typings expose many event shapes, but the docs list only common examples.
- No provider-native shell command, HTTP endpoint, or LLM-evaluator hook runner was found. Kilo's public hook mechanism is plugin functions.
- No per-hook timeout, cancellation, or stdout/stderr display contract was found for plugin hooks.
- The public docs say plugin directories can be named `plugin/` or `plugins/`, while the inspected config-path source uses config directories and the plugin loader path handling was not fully traced to confirm both directory names on every OS.
- The public typings for `tool.definition` omit `output.jsonSchema`, but the inspected source initializes and consumes it.
- The source contains an internal Effect-based v2 plugin layer with hooks such as `catalog.transform`, `account.switched`, `aisdk.language`, and `aisdk.sdk`. These are internal provider/model extension hooks, not documented user lifecycle hooks; Claudine should not treat them as user hook events without more evidence.

## Sources

- [Kilo homepage](https://kilo.ai/)
- [Kilo documentation](https://kilo.ai/docs)
- [Kilo plugins documentation](https://kilo.ai/docs/automate/extending/plugins)
- [Kilo settings documentation](https://kilo.ai/docs/getting-started/settings)
- [Kilo MCP config locations](https://kilo.ai/docs/automate/mcp/using-in-kilo-code)
- [Kilo sandboxing documentation](https://kilo.ai/docs/getting-started/settings/sandboxing)
- [Kilo repository](https://github.com/Kilo-Org/kilocode)
- [Plugin public hook typings, `packages/plugin/src/index.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/plugin/src/index.ts)
- [Plugin trigger implementation, `packages/opencode/src/plugin/index.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/plugin/index.ts)
- [LLM request hook call sites, `packages/opencode/src/session/llm/request.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/session/llm/request.ts)
- [Prompt hook call sites, `packages/opencode/src/session/prompt.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/session/prompt.ts)
- [Tool execution hook call sites, `packages/opencode/src/session/tools.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/session/tools.ts)
- [Permission events and request schema, `packages/opencode/src/permission/index.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/permission/index.ts)
- [Shell environment hook call site, `packages/opencode/src/tool/shell.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/tool/shell.ts)
- [Tool definition hook call site, `packages/opencode/src/tool/registry.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/tool/registry.ts)
- [Compaction hook call sites, `packages/opencode/src/session/compaction.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/session/compaction.ts)
- [Config loading, `packages/opencode/src/config/config.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/config/config.ts)
- [Config path discovery, `packages/opencode/src/config/paths.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/config/paths.ts)
- [Runtime flags, `packages/opencode/src/effect/runtime-flags.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/effect/runtime-flags.ts)
- [Plugin install command, `packages/opencode/src/cli/cmd/plug.ts`](https://github.com/Kilo-Org/kilocode/blob/2799ba638f9f5157f7fc3fc5783e67f026bcbcc3/packages/opencode/src/cli/cmd/plug.ts)
- Observed on host 2026-07-03: `/Users/ken/.kilo` absent; `/Users/ken/.config/kilo/kilo.jsonc` and `/Users/ken/.claudine/.config/kilo/kilo.jsonc` contain only the Kilo config schema URL; `/Users/ken/.config/kilo/node_modules/@kilocode/plugin/dist/index.d.ts` contains the installed hook interface.
