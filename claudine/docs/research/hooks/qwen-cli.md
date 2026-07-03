---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://github.com/QwenLM/qwen-code
docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
hooks_docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/

hooks:
  - native_event: PreToolUse
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, permission_mode, tool_name, tool_input, tool_use_id, tool_call_id"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {hookEventName, permissionDecision: allow|deny|ask, permissionDecisionReason, updatedInput, additionalContext}}. Exit 2: stderr fed to model as feedback."
    notes: "Matcher: tool name regex. permissionDecision is REQUIRED in the official interface. updatedInput mutates tool input before execution."

  - native_event: PostToolUse
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, permission_mode, tool_name, tool_input, tool_response, tool_use_id, tool_call_id"
    return_contract: "Exit 0 + JSON {decision: allow|deny|block, reason, hookSpecificOutput: {additionalContext}}. Defaults to allow. Tool already executed; block only warns/injects context."
    notes: "Matcher: tool name regex. Blocking here does not undo the tool call; it only influences downstream model behavior."

  - native_event: PostToolUseFailure
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, permission_mode, tool_use_id, tool_call_id, tool_name, tool_input, error, is_interrupt"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {additionalContext}}. Exit 2 stderr becomes feedback. Cannot reverse failure."
    notes: "Matcher: tool name regex. Observation/logging only."

  - native_event: UserPromptSubmit
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, prompt"
    return_contract: "Exit 0 + JSON {decision: allow|deny|block|ask, reason, hookSpecificOutput: {additionalContext}}. Exit 2: stderr fed to model as feedback."
    notes: "No matcher. Fires on every user prompt. additionalContext appends context to the prompt."

  - native_event: SessionStart
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, permission_mode, source, model, agent_type"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {additionalContext}}. Cannot block session start."
    notes: "Matcher: source regex (startup|resume|clear|compact). additionalContext is made available in the session."

  - native_event: SessionEnd
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, reason"
    return_contract: "No decision control; output is informational only."
    notes: "Matcher: reason regex (clear|logout|prompt_input_exit|bypass_permissions_disabled|other). Cleanup/observation only."

  - native_event: Stop
    claudine_event: finalize
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, stop_hook_active, last_assistant_message, context_usage, context_limit, input_tokens"
    return_contract: "Exit 0 + JSON {decision: allow|deny|block|ask, reason, stopReason, continue: false, hookSpecificOutput: {additionalContext}}. Exit 2: stderr fed to model as feedback."
    notes: "No matcher. Fires when Qwen prepares to conclude a response. continue: false stops execution entirely. Hooks should check stop_hook_active to avoid loops."

  - native_event: StopFailure
    claudine_event: failure
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, error, error_details, last_assistant_message"
    return_contract: "All hook output and exit codes are ignored. Fire-and-forget only."
    notes: "Matcher: error field regex (rate_limit|authentication_failed|billing_error|invalid_request|server_error|max_output_tokens|unknown). Purely observational."

  - native_event: SubagentStart
    claudine_event: subagent_start
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, permission_mode, agent_id, agent_type"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {additionalContext}}. Cannot block subagent launch."
    notes: "Matcher: agent_type regex. additionalContext seeds initial context for the subagent."

  - native_event: SubagentStop
    claudine_event: subagent_stop
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, permission_mode, stop_hook_active, agent_id, agent_type, agent_transcript_path, last_assistant_message"
    return_contract: "Exit 0 + JSON {decision: allow|deny|block|ask, reason}. Exit 2: stderr fed to model as feedback."
    notes: "Matcher: agent_type regex. Hooks MUST check stop_hook_active to avoid infinite loops."

  - native_event: PreCompact
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, trigger, custom_instructions"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {additionalContext}}. Cannot block compaction."
    notes: "Matcher: exact trigger (manual|auto). custom_instructions carries /compact text for manual compaction."

  - native_event: PostCompact
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, trigger, compact_summary"
    return_contract: "Decision fields logged only; no control effect."
    notes: "Matcher: exact trigger (manual|auto). Post-compaction observation only."

  - native_event: Notification
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, message, title, notification_type"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {additionalContext}}. No blocking."
    notes: "Matcher: exact notification_type (permission_prompt|idle_prompt|auth_success). elicitation_dialog type is defined but not implemented."

  - native_event: PermissionRequest
    claudine_event: permission
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, permission_mode, tool_name, tool_input, permission_suggestions"
    return_contract: "Exit 0 + JSON {hookSpecificOutput: {decision: {behavior: allow|deny, updatedInput, updatedPermissions, message, interrupt}}}."
    notes: "Matcher: tool name regex. Automates permission decisions when the permission dialog would be shown."

  - native_event: TodoCreated
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, todo_id, todo_content, todo_status, all_todos, phase"
    return_contract: "Validation phase: exit 0 + JSON {decision: allow|block|deny, reason} allows or blocks creation. postWrite phase: block/deny ignored."
    notes: "No matcher. phase is validation|postWrite. Derived from the todo_write tool lifecycle. Exact event names not observed in the v0.15.6 binary strings; sourced from docs."

  - native_event: TodoCompleted
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, todo_id, todo_content, previous_status, all_todos, phase"
    return_contract: "Validation phase: exit 0 + JSON {decision: allow|block|deny, reason} allows or blocks completion. postWrite phase: block/deny ignored."
    notes: "No matcher. phase is validation|postWrite. Derived from the todo_write tool lifecycle. Exact event names not observed in the v0.15.6 binary strings; sourced from docs."

config_files:
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    format: json
    notes: "System-wide defaults; lowest precedence. Path overridable via QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    format: json
    notes: "System-wide defaults; lowest precedence. Path overridable via QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    format: json
    notes: "System-wide defaults; lowest precedence. Path overridable via QWEN_CODE_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: "User settings. Base directory overridable via QWEN_HOME. Local observation: file exists on host and contains no hooks key."
  - os: linux
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: "User settings. Base directory overridable via QWEN_HOME."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\settings.json"
    format: json
    notes: "User settings. Base directory overridable via QWEN_HOME."
  - os: macos
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: "Project-level settings; override user settings. Requires trusted-folder status for project-level hooks."
  - os: linux
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: "Project-level settings; override user settings. Requires trusted-folder status for project-level hooks."
  - os: windows
    scope: repo
    path: ".qwen\\settings.json"
    format: json
    notes: "Project-level settings; override user settings. Requires trusted-folder status for project-level hooks."
  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/settings.json"
    format: json
    notes: "System-wide override settings; highest file precedence. Path overridable via QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: linux
    scope: system
    path: "/etc/qwen-code/settings.json"
    format: json
    notes: "System-wide override settings; highest file precedence. Path overridable via QWEN_CODE_SYSTEM_SETTINGS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    format: json
    notes: "System-wide override settings; highest file precedence. Path overridable via QWEN_CODE_SYSTEM_SETTINGS_PATH."

cli_params:
  - flag: "--bare"
    description: "Minimal mode: skip implicit startup auto-discovery. Disables context files, hooks, extensions, skills, MCP servers, custom subagents, permission rules, memory, and sandbox settings for the session."
    example: "qwen --bare -p 'query'"
  - flag: "--safe-mode"
    description: "Disable all customizations including hooks. CLI flags --yolo and --approval-mode still take effect. Also settable via QWEN_CODE_SAFE_MODE=true."
    example: "qwen --safe-mode -p 'query'"
  - flag: "--approval-mode"
    description: "Set the active permission mode (plan|default|auto-edit|auto|yolo); reflected in hook payloads as permission_mode."
    example: "qwen --approval-mode auto-edit -p 'query'"
  - flag: "--yolo / -y"
    description: "Auto-approve all actions (YOLO mode). Equivalent to --approval-mode yolo."
    example: "qwen -y -p 'query'"
  - flag: "--allowed-tools"
    description: "Comma-separated tool names that bypass confirmation; merged with permissions.allow rules."
    example: "qwen --allowed-tools 'Bash(git status)' -p 'query'"
  - flag: "--disabled-slash-commands"
    description: "Slash command names to hide/disable; unrelated to hooks but part of the customization layer."
    example: "qwen --disabled-slash-commands 'memory,plan'"
  - flag: "--debug / -d"
    description: "Enable debug mode; emits detailed hook matching and execution information."
    example: "qwen -d -p 'query'"
  - flag: "disableAllHooks (settings key)"
    description: "Top-level settings.json boolean that disables all hooks without deleting their configuration."
    example: '{ "disableAllHooks": true, "hooks": { ... } }'
  - flag: "allowedUrls (settings key)"
    description: "HTTP-hook URL allowlist pattern array under settings; empty or undefined behavior is not fully documented."
    example: '{ "allowedUrls": ["https://hooks.example.com/*"] }'
  - flag: "sequential (hook group key)"
    description: "Set to true inside a hook matcher group to run its handlers sequentially instead of in parallel."
    example: '{ "matcher": "Bash", "sequential": true, "hooks": [...] }'

payload_fields:
  - native_event: PreToolUse
    field: "tool_name"
    type: string
    meaning: "Name of the tool about to execute; drives matcher regex."
  - native_event: PreToolUse
    field: "tool_input"
    type: object
    meaning: "Arguments passed to the tool; schema varies by tool."
  - native_event: PreToolUse
    field: "tool_use_id"
    type: string
    meaning: "Internal tool-use identifier (e.g., toolu_xxx)."
  - native_event: PreToolUse
    field: "tool_call_id"
    type: string
    meaning: "Original LLM provider call ID (e.g., call_xxx); optional."
  - native_event: PreToolUse
    field: "permission_mode"
    type: string
    meaning: "Active approval mode: plan|default|auto-edit|yolo."
  - native_event: PostToolUse
    field: "tool_response"
    type: object
    meaning: "Tool output returned by the tool."
  - native_event: PostToolUseFailure
    field: "error"
    type: string
    meaning: "Error message describing the failure."
  - native_event: PostToolUseFailure
    field: "is_interrupt"
    type: boolean
    meaning: "Whether the failure was caused by user interruption; optional."
  - native_event: UserPromptSubmit
    field: "prompt"
    type: string
    meaning: "The user-submitted prompt text."
  - native_event: SessionStart
    field: "source"
    type: string
    meaning: "Why the session started: startup|resume|clear|compact."
  - native_event: SessionStart
    field: "model"
    type: string
    meaning: "Model selected for the session."
  - native_event: SessionStart
    field: "agent_type"
    type: string
    meaning: "Agent type if applicable; optional."
  - native_event: SessionEnd
    field: "reason"
    type: string
    meaning: "Why the session ended: clear|logout|prompt_input_exit|bypass_permissions_disabled|other."
  - native_event: Stop
    field: "stop_hook_active"
    type: boolean
    meaning: "true when a stop hook has already kept Qwen working; check to avoid loops."
  - native_event: Stop
    field: "last_assistant_message"
    type: string
    meaning: "The last assistant message before the stop decision."
  - native_event: Stop
    field: "context_usage"
    type: number
    meaning: "Ratio of context window used; may exceed 1."
  - native_event: Stop
    field: "context_limit"
    type: number
    meaning: "Context window size in tokens."
  - native_event: Stop
    field: "input_tokens"
    type: number
    meaning: "Prompt token count."
  - native_event: StopFailure
    field: "error"
    type: string
    meaning: "Typed error discriminator: rate_limit|authentication_failed|billing_error|invalid_request|server_error|max_output_tokens|unknown."
  - native_event: StopFailure
    field: "error_details"
    type: string
    meaning: "Detailed error message; optional."
  - native_event: SubagentStart
    field: "agent_id"
    type: string
    meaning: "Identifier for the subagent instance."
  - native_event: SubagentStart
    field: "agent_type"
    type: string
    meaning: "Type of subagent; drives matcher regex."
  - native_event: SubagentStop
    field: "agent_transcript_path"
    type: string
    meaning: "Path to the subagent's transcript."
  - native_event: SubagentStop
    field: "last_assistant_message"
    type: string
    meaning: "Last message from the subagent."
  - native_event: PreCompact
    field: "trigger"
    type: string
    meaning: "manual or auto."
  - native_event: PreCompact
    field: "custom_instructions"
    type: string
    meaning: "Text passed to /compact for manual trigger."
  - native_event: PostCompact
    field: "compact_summary"
    type: string
    meaning: "Summary generated by compaction."
  - native_event: Notification
    field: "notification_type"
    type: string
    meaning: "permission_prompt|idle_prompt|auth_success; exact-match matcher target."
  - native_event: Notification
    field: "title"
    type: string
    meaning: "Notification title; optional."
  - native_event: PermissionRequest
    field: "permission_suggestions"
    type: array
    meaning: "Suggested permission options shown to the user; optional."
  - native_event: TodoCreated
    field: "todo_id"
    type: string
    meaning: "Unique todo identifier."
  - native_event: TodoCreated
    field: "todo_content"
    type: string
    meaning: "Todo description."
  - native_event: TodoCreated
    field: "todo_status"
    type: string
    meaning: "pending|in_progress|completed."
  - native_event: TodoCreated
    field: "all_todos"
    type: array
    meaning: "All todos in the current list."
  - native_event: TodoCreated
    field: "phase"
    type: string
    meaning: "validation (pre-persistence) or postWrite (post-persistence)."
  - native_event: TodoCompleted
    field: "previous_status"
    type: string
    meaning: "Status before completion."
  - native_event: (common)
    field: "session_id"
    type: string
    meaning: "Current session identifier; present on every event."
  - native_event: (common)
    field: "transcript_path"
    type: string
    meaning: "Path to the conversation JSONL transcript; present on every event."
  - native_event: (common)
    field: "cwd"
    type: string
    meaning: "Working directory when the hook fired; present on every event."
  - native_event: (common)
    field: "hook_event_name"
    type: string
    meaning: "Name of the native event that fired; present on every event."
  - native_event: (common)
    field: "timestamp"
    type: string
    meaning: "Event timestamp; present on every event."

response_actions:
  - action: allow
    native_value: "{hookSpecificOutput.permissionDecision: 'allow', permissionDecisionReason}"
    effect: "PreToolUse/PermissionRequest: proceeds without interactive prompt."
  - action: deny
    native_value: "{hookSpecificOutput.permissionDecision: 'deny', permissionDecisionReason}"
    effect: "PreToolUse: cancels the tool call and feeds the reason to the model. PermissionRequest: denies permission."
  - action: ask
    native_value: "{hookSpecificOutput.permissionDecision: 'ask', permissionDecisionReason}"
    effect: "PreToolUse: surfaces the normal permission dialog to the user."
  - action: block
    native_value: "Exit 2 OR {decision: 'block', reason}"
    effect: "PreToolUse/PostToolUse/UserPromptSubmit/Stop/SubagentStop/TodoCreated/TodoCompleted: stops or prevents the action; reason shown to model."
  - action: modify
    native_value: "{hookSpecificOutput.updatedInput: {...}}"
    effect: "PreToolUse/PermissionRequest: mutates tool input before execution."
  - action: continue
    native_value: "{continue: true|false, stopReason, suppressOutput, systemMessage}"
    effect: "Top-level flag valid for any event. continue: false stops Qwen entirely; systemMessage shown to user; suppressOutput hides hook progress."
  - action: other
    native_value: "{hookSpecificOutput.additionalContext: '...'}"
    effect: "Injects context into the conversation/session/subagent."
  - action: other
    native_value: "Prompt hook {ok: true|false, reason, additionalContext}"
    effect: "ok: false blocks and uses reason as feedback; additionalContext injects context when allowing."

execution:
  shell: "Command hooks use the session shell by default; the 'shell' field can pin 'bash' or 'powershell'. Prompt hooks run the current model via LLM. HTTP hooks POST JSON to the target URL. Function hooks are internal-only JavaScript/TypeScript calls used by the Skill system."
  cwd: "Command hooks run in the Qwen Code session's current working directory (the cwd field in the payload)."
  env: "Qwen Code's environment is exported to command hooks; extra env can be supplied per hook via the 'env' field. HTTP hooks support ${VAR} interpolation in URL/headers only for variables listed in allowedEnvVars."
  timeout: "Default 60 seconds for command hooks (milliseconds in config), 600 seconds for HTTP hooks, 30 seconds for prompt hooks. Per-hook 'timeout' overrides."
  stdin: "Command hooks receive the JSON event payload on stdin. HTTP hooks receive the same JSON as the POST body. Prompt hooks receive input via the $ARGUMENTS placeholder."
  stdout: "Exit 0 stdout must be valid JSON. Plain text is not documented as a valid decision format; output must be JSON."
  stderr: "Exit 2: stderr is passed to the model as error feedback for blockable events. Other non-zero exits: stderr shown only in debug mode, execution continues."
  notes: "Hooks execute in parallel by default. Set 'sequential: true' on a matcher group to serialize handlers. Only command hooks support 'async: true' for fire-and-forget background execution; async hooks cannot return decisions and their output is delivered on the next turn via systemMessage/additionalContext. Project-level hooks require the project folder to be trusted. HTTP hooks have SSRF protection (private IPs blocked, loopback allowed) and DNS validation."

gaps:
  - "TodoCreated and TodoCompleted event names and payloads are documented on the official hooks page but were not found as quoted strings in the local v0.15.6 binary; they should be treated as sourced from docs, not observed locally."
  - "QWEN_CODE_SAFE_MODE is documented in the headless-mode page but the literal string was not observed in the v0.15.6 binary; QWEN_CODE_SIMPLE was observed."
  - "The exact precedence and merge semantics when hooks are defined in multiple settings layers (system defaults, user, project, system override) are not explicitly documented."
  - "Whether PostToolUse decision: block actually prevents downstream model processing or merely warns is ambiguous from the docs; treat as observational block (cannot undo execution)."
  - "No dedicated CLI subcommands exist to list, test, or validate hooks; configuration is hand-edited JSON plus interactive /hooks."
  - "Function hooks are described as internal-only and not exposed as a public API; their exact registration contract is undocumented."
  - "HTTP hook allowedUrls semantics (empty array vs undefined vs wildcard) are not fully specified."
  - "Live reload behavior for hooks when settings.json changes mid-session is not explicitly documented."
  - "StopFailure exact error value set and matcher behavior are not exhaustively documented."
  - "The difference between --bare and --safe-mode with respect to managed/system hooks is not documented."

changes: []

requires_claudine_update: true
reason: "Qwen Code is a new provider in Claudine's roster. The adapter needs a native-event to unified-event mapping (16 events across pre/post/async timing), support for four handler types (command/http/prompt/function), regex vs exact matcher resolution per event, prompt-hook {ok,reason,additionalContext} parsing, todo_write lifecycle mapping with validation/postWrite phases, and session-gate modeling for --bare/--safe-mode/QWEN_CODE_SAFE_MODE/QWEN_CODE_SIMPLE/disableAllHooks."
---

# Qwen Code Hooks and Events

## Overview

Qwen Code ships a hook system that fires user-defined handlers at specific points in the agent lifecycle. As of v0.15.6 the documented surface supports **four handler types**: shell `command` hooks, `http` POST hooks, LLM `prompt` hooks, and internal `function` hooks (used by the Skill system and not exposed as a public API).

A hook configuration is a JSON object nested under a top-level `hooks` key in a `settings.json` file. Each event holds a list of *matcher groups*; each group has an optional `matcher` (regex or exact string) and an array of handlers that fire when the matcher matches. By default all matching handlers in a group run in parallel; `sequential: true` serializes them.

Hooks can **block**, **allow/deny/ask**, **modify tool input**, **inject context**, and **stop execution**. They cannot reverse already-executed tool calls, terminate the session directly, or override system-level policy. Project-level hooks additionally require the project folder to be marked as trusted.

## Native Hooks

### Event inventory

| Event | Timing | Matcher target | Can block |
|-------|--------|----------------|-----------|
| `PreToolUse` | `pre` | Tool name regex | yes |
| `PostToolUse` | `post` | Tool name regex | yes (warning only) |
| `PostToolUseFailure` | `post` | Tool name regex | no |
| `UserPromptSubmit` | `pre` | none | yes |
| `SessionStart` | `pre` | Source regex | no |
| `SessionEnd` | `post` | Reason regex | no |
| `Stop` | `pre` | none | yes |
| `StopFailure` | `post` | Error regex | no |
| `SubagentStart` | `pre` | Agent type regex | no |
| `SubagentStop` | `post` | Agent type regex | yes |
| `PreCompact` | `pre` | Exact trigger | no |
| `PostCompact` | `post` | Exact trigger | no |
| `Notification` | `async` | Exact notification_type | no |
| `PermissionRequest` | `pre` | Tool name regex | yes |
| `TodoCreated` | `pre` | none | yes (validation phase only) |
| `TodoCompleted` | `post` | none | yes (validation phase only) |

### Matcher resolution rules

| Matcher value | Evaluated as |
|---------------|--------------|
| `""` or `"*"` or omitted | match all events of that type |
| Tool / subagent / session / stop-failure events | standard JavaScript regex (`RegExp.prototype.test`) |
| `Notification`, `PreCompact`, `PostCompact` | exact string match |
| `TodoCreated`, `TodoCompleted`, `UserPromptSubmit`, `Stop` | no matcher support |

Examples: `"^Bash$"` matches only Bash; `"Write.*"` matches `WriteFile` and `WriteDir`; `"(WriteFile|Edit)"` matches either.

### Handler types

| Type | Transport | Default timeout | Returns |
|------|-----------|-----------------|---------|
| `command` | shell child process | 60 s | exit code + stdout JSON |
| `http` | POST request | 600 s | HTTP response body JSON |
| `prompt` | single LLM call | 30 s | `{ok, reason, additionalContext}` |
| `function` | registered JS/TS function | unknown | internal-only |

All handlers accept `name`, `description`, `timeout`, `statusMessage`. Command hooks additionally accept `command`, `args`, `env`, `shell` (`"bash"` or `"powershell"`), and `async`. HTTP hooks accept `url`, `headers`, `allowedEnvVars`, and `once`. Prompt hooks accept `prompt` (with `$ARGUMENTS` placeholder) and `model`.

### Per-event decision contract

| Event | Decision carrier | Blocking behavior |
|-------|------------------|-------------------|
| `PreToolUse` | `hookSpecificOutput.permissionDecision` | `deny` cancels tool; `allow`/`ask` affect prompt flow; `updatedInput` mutates input |
| `PermissionRequest` | `hookSpecificOutput.decision.behavior` | `allow`/`deny`; may carry `updatedInput`, `updatedPermissions`, `message`, `interrupt` |
| `PostToolUse` | top-level `decision` | `block` warns only; tool already ran |
| `UserPromptSubmit`, `Stop`, `SubagentStop` | top-level `decision` | `block` with `reason` stops action |
| `TodoCreated`, `TodoCompleted` | top-level `decision` | `block`/`deny` blocks only during `validation` phase |
| `SessionStart`, `SessionEnd`, `SubagentStart`, `PreCompact`, `PostCompact`, `Notification`, `PostToolUseFailure`, `StopFailure` | (no decision control) | observation / context injection only |

Top-level `continue: false` overrides every event-specific decision and stops Qwen entirely.

## Configuration

### Settings file locations and precedence

Configuration is applied in order of precedence (later overrides earlier):

1. Hardcoded defaults
2. System defaults file
3. User settings file
4. Project settings file
5. System settings file
6. Environment variables
7. Command-line arguments

| Scope | macOS | Linux | Windows | Notes |
|-------|-------|-------|---------|-------|
| System defaults | `/Library/Application Support/QwenCode/system-defaults.json` | `/etc/qwen-code/system-defaults.json` | `C:\ProgramData\qwen-code\system-defaults.json` | Lowest file precedence; path overridable via `QWEN_CODE_SYSTEM_DEFAULTS_PATH` |
| User | `~/.qwen/settings.json` | `~/.qwen/settings.json` | `%USERPROFILE%\.qwen\settings.json` | Base directory overridable via `QWEN_HOME` |
| Project | `.qwen/settings.json` | `.qwen/settings.json` | `.qwen\settings.json` | Requires trusted-folder status for hooks to run |
| System override | `/Library/Application Support/QwenCode/settings.json` | `/etc/qwen-code/settings.json` | `C:\ProgramData\qwen-code\settings.json` | Highest file precedence; path overridable via `QWEN_CODE_SYSTEM_SETTINGS_PATH` |

Local observation: the host's `~/.qwen/settings.json` exists and contains no `hooks` key.

### Top-level switch

Set `disableAllHooks: true` at the root of any `settings.json` to disable all hooks without deleting their configuration:

```json
{
  "disableAllHooks": true,
  "hooks": { "PreToolUse": [] }
}
```

### Hook configuration shape

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "^Bash$",
        "sequential": false,
        "hooks": [
          {
            "type": "command",
            "command": "$QWEN_PROJECT_DIR/.qwen/hooks/security-check.sh",
            "name": "security-check",
            "timeout": 10000
          }
        ]
      }
    ]
  }
}
```

### CLI controls

Qwen Code does not provide dedicated hook-management subcommands. The `qwen hooks` command only prints its own help. In interactive mode, `/hooks` lists configured hooks. Hook-affecting CLI flags are:

- `--bare` — skip hooks and all other startup customizations.
- `--safe-mode` — disable hooks and all customizations; `--yolo` and `--approval-mode` still apply.
- `--approval-mode <mode>` / `--yolo` — change the `permission_mode` reflected in payloads.
- `--debug` — emit hook matching and execution details.

### Environment variables

| Variable | Effect |
|----------|--------|
| `QWEN_HOME` | Change the global config root from `~/.qwen` |
| `QWEN_RUNTIME_DIR` | Override runtime output directory (conversations, logs, todos) |
| `QWEN_CODE_SYSTEM_DEFAULTS_PATH` | Override system-defaults file path |
| `QWEN_CODE_SYSTEM_SETTINGS_PATH` | Override system-override settings file path |
| `QWEN_CODE_SAFE_MODE` | Documented equivalent of `--safe-mode`; set to `true` |
| `QWEN_CODE_SIMPLE` | Observed in v0.15.6 binary; related to minimal mode |
| `QWEN_CODE_UNATTENDED_RETRY` | Retry 429/529 errors in headless mode |

No hook-specific `QWEN_DISABLE_ALL_HOOKS` environment variable is documented; use the `disableAllHooks` settings key.

## Payloads and Responses

### Common input fields

Every hook receives:

```json
{
  "session_id": "string",
  "transcript_path": "string",
  "cwd": "string",
  "hook_event_name": "string",
  "timestamp": "string"
}
```

When running inside a subagent, `agent_id` and `agent_type` are additionally included.

### Common output fields

Hook output is JSON on stdout (command) or in the HTTP response body:

```json
{
  "continue": true,
  "decision": "allow",
  "reason": "...",
  "stopReason": "...",
  "suppressOutput": false,
  "systemMessage": "...",
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "additionalContext": "..."
  }
}
```

### Exit code behavior

| Exit code | Meaning |
|-----------|---------|
| `0` | Success; parse stdout as JSON decision |
| `2` | Blocking error; ignore stdout, pass stderr to the model as feedback |
| other | Non-blocking error; stderr shown only in debug mode, execution continues |

### Per-event input highlights

- **PreToolUse / PostToolUse / PostToolUseFailure / PermissionRequest** — `tool_name`, `tool_input`, `tool_use_id`, `tool_call_id`, `permission_mode`.
- **UserPromptSubmit** — `prompt`.
- **SessionStart** — `source`, `model`, `agent_type`, `permission_mode`.
- **SessionEnd** — `reason`.
- **Stop** — `stop_hook_active`, `last_assistant_message`, `context_usage`, `context_limit`, `input_tokens`.
- **StopFailure** — `error`, `error_details`, `last_assistant_message`.
- **SubagentStart** — `agent_id`, `agent_type`, `permission_mode`.
- **SubagentStop** — `stop_hook_active`, `agent_id`, `agent_type`, `agent_transcript_path`, `last_assistant_message`, `permission_mode`.
- **PreCompact** — `trigger`, `custom_instructions`.
- **PostCompact** — `trigger`, `compact_summary`.
- **Notification** — `message`, `title`, `notification_type`.
- **TodoCreated** — `todo_id`, `todo_content`, `todo_status`, `all_todos`, `phase`.
- **TodoCompleted** — `todo_id`, `todo_content`, `previous_status`, `all_todos`, `phase`.

### Per-event output highlights

- **PreToolUse** — `hookSpecificOutput.permissionDecision` (`allow|deny|ask`), `permissionDecisionReason`, `updatedInput`, `additionalContext`.
- **PermissionRequest** — `hookSpecificOutput.decision.behavior` (`allow|deny`), `updatedInput`, `updatedPermissions`, `message`, `interrupt`.
- **PostToolUse** — top-level `decision` (`allow|deny|block`), `reason`, `hookSpecificOutput.additionalContext`.
- **UserPromptSubmit / Stop / SubagentStop** — top-level `decision` (`allow|deny|block|ask`), `reason`, `hookSpecificOutput.additionalContext`.
- **TodoCreated / TodoCompleted** — top-level `decision` (`allow|block|deny`), `reason`; blocking only effective in `validation` phase.
- **Prompt hooks** — `{ok: true|false, reason, additionalContext}`.
- **SessionStart / SubagentStart / PreCompact / PostCompact / Notification / PostToolUseFailure** — `hookSpecificOutput.additionalContext` only.
- **StopFailure** — all output ignored.

## Execution Semantics

### Shell, cwd, environment, timeout

- **Shell** — command hooks run in the session's default shell; pin with `"shell": "bash"` or `"shell": "powershell"`.
- **cwd** — handlers run in the session's current working directory (`cwd` in payload).
- **Environment** — Qwen Code's environment is exported; per-hook `env` can add variables. HTTP hooks allow `${VAR}` interpolation only for variables listed in `allowedEnvVars`.
- **Timeout** — command hooks default to 60 seconds (configured in milliseconds), HTTP hooks to 600 seconds, prompt hooks to 30 seconds. Override per hook.

### Stdin / stdout / stderr

- **Stdin** — JSON event payload for command hooks; POST body for HTTP hooks.
- **Stdout** — on exit 0, parsed as JSON decision. Non-JSON stdout behavior is not documented.
- **Stderr** — on exit 2, passed to the model as feedback for blockable events. On other non-zero exits, shown only in debug mode.

### Parallel vs sequential

By default, handlers within a matcher group run in parallel. Set `"sequential": true` on the group to serialize them; earlier hooks can modify input seen by later ones.

### Async hooks

Only `command` hooks support `"async": true`. Async hooks run in the background and cannot return decisions. Their output is delivered on the next conversation turn via `systemMessage` or `additionalContext`.

### Security model

- Hooks run with user privileges in the user's environment.
- Project-level hooks require trusted-folder status.
- HTTP hooks have URL allowlisting (`allowedUrls`), SSRF protection (private IPs blocked, loopback allowed), and DNS validation to prevent rebinding.

## Claudine Mapping

| Qwen Code event | Claudine event | Notes |
|-----------------|----------------|-------|
| `SessionStart` | `initialize` | Carries `source` as a lifecycle discriminator (`startup|resume|clear|compact`). |
| `UserPromptSubmit` | `prompt` | Blockable; preserves `prompt` text. |
| `PreToolUse` | `tool_call` | Blockable; carries `tool_name` and polymorphic `tool_input`. |
| `PermissionRequest` | `permission` | Automates permission dialog decisions. |
| `PostToolUse` | `tool_result` | Post-event; `block` only warns/injects context. |
| `PostToolUseFailure` | `tool_result` | Provider-extension with `error` and `is_interrupt`. |
| `SubagentStart` | `subagent_start` | Carries `agent_id` and `agent_type`. |
| `SubagentStop` | `subagent_stop` | Blockable; carries `stop_hook_active` and `agent_transcript_path`. |
| `Stop` | `finalize` | Blockable pre-stop check; `continue: false` terminates. |
| `StopFailure` | `failure` | Output ignored; carries typed `error`. |
| `SessionEnd` | `finalize` | Post-event counterpart of Stop; carries `reason`. |
| `PreCompact` | `notification` | Pre-event notification. |
| `PostCompact` | `notification` | Post-event notification. |
| `Notification` | `notification` | Async; carries `notification_type` as `kind`. |
| `TodoCreated` | `tool_call` | Validation phase blocks the underlying `todo_write` write. |
| `TodoCompleted` | `tool_result` | Validation phase blocks the status change. |

Provider-specific discriminator fields to preserve: `permission_mode`, `source`, `reason`, `agent_type`, `agent_transcript_path`, `stop_hook_active`, `notification_type`, `trigger`, `compact_summary`, `context_usage`, `context_limit`, `input_tokens`, `error`, `error_details`, `is_interrupt`, `todo_id`, `todo_content`, `todo_status`, `previous_status`, `all_todos`, `phase`.

## Gaps

1. **TodoCreated / TodoCompleted** — documented but not observed as quoted event names in the local v0.15.6 binary.
2. **`QWEN_CODE_SAFE_MODE`** — documented as an env var but not observed in the v0.15.6 binary; `QWEN_CODE_SIMPLE` was observed.
3. **Multi-layer hook merge semantics** — exact precedence when hooks exist in system defaults, user, project, and system override files is not specified.
4. **PostToolUse blocking effect** — whether `decision: block` actually prevents downstream model processing or only warns is ambiguous.
5. **No hook CLI management** — no subcommands to list, test, install, or remove hooks; only interactive `/hooks` and hand-edited JSON.
6. **Function hooks** — internal-only; public registration contract is undocumented.
7. **HTTP `allowedUrls`** — exact allowlist semantics (empty array vs undefined vs wildcard) are not fully specified.
8. **Live reload** — whether settings changes are picked up mid-session is not documented.
9. **StopFailure error set** — exhaustive matcher values for `error` are not documented.
10. **`--bare` vs `--safe-mode`** — difference regarding managed/system hooks is not documented.

## Sources

- Qwen Code overview: <https://qwenlm.github.io/qwen-code-docs/en/users/overview/>
- Qwen Code Hooks documentation: <https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/>
- Qwen Code Configuration: <https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/>
- Qwen Code Headless Mode: <https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/>
- Qwen Code Approval Mode: <https://qwenlm.github.io/qwen-code-docs/en/users/features/approval-mode/>
- Qwen Code Typed Daemon Event Schema v1: <https://qwenlm.github.io/qwen-code-docs/en/developers/daemon/09-event-schema/>
- Qwen Code GitHub repository: <https://github.com/QwenLM/qwen-code>
- Local Qwen Code binary v0.15.6 at `/opt/homebrew/Cellar/qwen-code/0.15.6/bin/qwen` and user settings at `~/.qwen/settings.json`
