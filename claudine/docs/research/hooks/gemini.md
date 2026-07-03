---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
hooks_docs: https://geminicli.com/docs/hooks/
hooks:
  - native_event: SessionStart
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, source: startup | resume | clear"
    return_contract: "Exit 0 + JSON {hookSpecificOutput.additionalContext, systemMessage}. continue/decision are ignored. Startup is never blocked."
    notes: "Fires on application startup, session resume, and /clear. In non-interactive mode additionalContext is wrapped in <hook_context> and prepended to the prompt."
  - native_event: SessionEnd
    claudine_event: finalize
    timing: async
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, reason: exit | clear | logout | prompt_input_exit | other"
    return_contract: "Best effort; CLI does not wait for completion. systemMessage may display during shutdown. Flow-control fields ignored."
    notes: "Observational only. Fires on graceful exit via cleanup handler."
  - native_event: BeforeAgent
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, prompt"
    return_contract: "Exit 0 + JSON {hookSpecificOutput.additionalContext, decision: deny, continue: false, reason}. decision: deny discards the user message from history. continue: false keeps it in history but blocks the turn. Exit 2 is equivalent to decision: deny with stderr as reason."
    notes: "Fires after the user submits a prompt but before the agent begins planning."
  - native_event: AfterAgent
    claudine_event: success
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, prompt, prompt_response, stop_hook_active"
    return_contract: "Exit 0 + JSON {decision: deny, reason, continue: false, stopReason, hookSpecificOutput.clearContext}. decision: deny rejects the response and triggers an automatic retry with reason as feedback. continue: false stops without retry. clearContext: true clears conversation history. Exit 2 triggers retry using stderr as feedback."
    notes: "stop_hook_active indicates the hook is already running as part of a retry sequence; check it to avoid infinite loops."
  - native_event: BeforeModel
    claudine_event: unknown
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, llm_request {model, messages, config, toolConfig}"
    return_contract: "Exit 0 + JSON {hookSpecificOutput.llm_request, hookSpecificOutput.llm_response, decision: deny, reason}. llm_request merges with outgoing request. llm_response synthesizes a response and skips the model call. Exit 2 aborts the turn."
    notes: "Claudine has no unified event for LLM request interception."
  - native_event: AfterModel
    claudine_event: unknown
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, llm_request, llm_response {candidates, usageMetadata}"
    return_contract: "Exit 0 + JSON {hookSpecificOutput.llm_response, decision: deny, continue: false, reason}. llm_response replaces the current streamed chunk. decision: deny discards the chunk and aborts the turn. continue: false kills the agent loop. Exit 2 aborts the turn."
    notes: "Fires for every streamed response chunk. Modifying only affects the current chunk."
  - native_event: BeforeToolSelection
    claudine_event: unknown
    timing: pre
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, llm_request {model, messages, config, toolConfig}"
    return_contract: "Exit 0 + JSON {hookSpecificOutput.toolConfig {mode: AUTO | ANY | NONE, allowedFunctionNames}}. decision, continue, systemMessage are not supported."
    notes: "Filters available tools before the model chooses. Multiple hooks union allowedFunctionNames; NONE wins, then ANY. No Claudine equivalent."
  - native_event: BeforeTool
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, tool_name, tool_input, mcp_context?, original_request_name?"
    return_contract: "Exit 0 + JSON {decision: deny/block, reason, hookSpecificOutput.tool_input, continue: false, stopReason}. tool_input merges with/overrides model arguments. Exit 2 blocks the tool and sends stderr as a tool error; the turn continues."
    notes: "Matcher is a RegExp against tool_name; invalid regex falls back to exact match. mcp_context present only for MCP tools."
  - native_event: AfterTool
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, tool_name, tool_input, tool_response {llmContent, returnDisplay, error}, mcp_context?, original_request_name?"
    return_contract: "Exit 0 + JSON {decision: deny, reason, hookSpecificOutput.additionalContext, hookSpecificOutput.tailToolCallRequest, continue: false}. decision: deny hides the real result and sends reason as replacement. tailToolCallRequest runs another tool whose result replaces the original. Exit 2 hides the result using stderr as replacement."
    notes: "Tool has already executed; blocking only controls what the model sees."
  - native_event: PreCompress
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, trigger: auto | manual"
    return_contract: "Exit 0 + JSON {systemMessage}. Flow-control fields ignored; compression cannot be blocked or modified."
    notes: "Fire-and-forget advisory hook before context summarization."
  - native_event: Notification
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "session_id, transcript_path, cwd, hook_event_name, timestamp, notification_type: ToolPermission, message, details"
    return_contract: "Exit 0 + JSON {systemMessage}. Cannot block alerts or grant permissions; flow-control fields ignored."
    notes: "Observability only. Currently documented notification_type is ToolPermission."
config_files:
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/system-defaults.json"
    format: json
    notes: "Lowest precedence system-wide defaults. Override path with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/system-defaults.json"
    format: json
    notes: "Lowest precedence system-wide defaults. Override path with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\system-defaults.json"
    format: json
    notes: "Lowest precedence system-wide defaults. Override path with GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    notes: "User-level settings; hooks live under top-level hooks and hooksConfig keys."
  - os: linux
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    notes: "User-level settings; hooks live under top-level hooks and hooksConfig keys."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\settings.json"
    format: json
    notes: "User-level settings; hooks live under top-level hooks and hooksConfig keys."
  - os: macos
    scope: repo
    path: ".gemini/settings.json"
    format: json
    notes: "Project-level settings. Project hooks only run when the folder is trusted; fingerprinting warns on name/command changes."
  - os: linux
    scope: repo
    path: ".gemini/settings.json"
    format: json
    notes: "Project-level settings. Project hooks only run when the folder is trusted; fingerprinting warns on name/command changes."
  - os: windows
    scope: repo
    path: ".gemini\\settings.json"
    format: json
    notes: "Project-level settings. Project hooks only run when the folder is trusted; fingerprinting warns on name/command changes."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/settings.json"
    format: json
    notes: "Highest precedence settings file (system override). Override path with GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/settings.json"
    format: json
    notes: "Highest precedence settings file (system override). Override path with GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    format: json
    notes: "Highest precedence settings file (system override). Override path with GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: macos
    scope: other
    path: "<extension_dir>/hooks/hooks.json"
    format: json
    notes: "Extension-bundled hooks; loaded only when the extension is active. Extension hooks use the same shape as settings hooks."
  - os: linux
    scope: other
    path: "<extension_dir>/hooks/hooks.json"
    format: json
    notes: "Extension-bundled hooks; loaded only when the extension is active. Extension hooks use the same shape as settings hooks."
  - os: windows
    scope: other
    path: "<extension_dir>\\hooks\\hooks.json"
    format: json
    notes: "Extension-bundled hooks; loaded only when the extension is active. Extension hooks use the same shape as settings hooks."
cli_params:
  - flag: "/hooks panel"
    description: "In-session slash command that opens a read-only panel of registered hooks."
    example: "/hooks panel"
  - flag: "/hooks enable-all"
    description: "In-session slash command that enables all currently disabled hooks."
    example: "/hooks enable-all"
  - flag: "/hooks disable-all"
    description: "In-session slash command that disables all currently enabled hooks."
    example: "/hooks disable-all"
  - flag: "/hooks enable <name>"
    description: "In-session slash command that enables a hook by its configured name."
    example: "/hooks enable security-check"
  - flag: "/hooks disable <name>"
    description: "In-session slash command that disables a hook by its configured name."
    example: "/hooks disable security-check"
  - flag: "gemini hooks migrate"
    description: "CLI subcommand to migrate hooks from Claude Code to Gemini CLI format."
    example: "gemini hooks migrate"
  - flag: "hooksConfig.enabled"
    description: "Settings key that globally enables or disables the hook system (default true)."
    example: '{"hooksConfig": {"enabled": false}}'
  - flag: "hooksConfig.disabled"
    description: "Settings array of hook names to disable even when configured."
    example: '{"hooksConfig": {"disabled": ["security-check"]}}'
payload_fields:
  - native_event: "*"
    field: session_id
    type: string
    meaning: "Unique ID for the current session."
  - native_event: "*"
    field: transcript_path
    type: string
    meaning: "Absolute path to the session transcript JSON, or empty string if unavailable."
  - native_event: "*"
    field: cwd
    type: string
    meaning: "Current working directory used for hook execution."
  - native_event: "*"
    field: hook_event_name
    type: string
    meaning: "Provider-native event name that fired."
  - native_event: "*"
    field: timestamp
    type: string
    meaning: "ISO 8601 hook execution timestamp."
  - native_event: SessionStart
    field: source
    type: enum(startup,resume,clear)
    meaning: "What triggered the session start."
  - native_event: SessionEnd
    field: reason
    type: enum(exit,clear,logout,prompt_input_exit,other)
    meaning: "Why the session is ending."
  - native_event: BeforeAgent
    field: prompt
    type: string
    meaning: "Original user prompt text before agent planning."
  - native_event: AfterAgent
    field: prompt
    type: string
    meaning: "Original user request for the completed turn."
  - native_event: AfterAgent
    field: prompt_response
    type: string
    meaning: "Final text generated by the agent."
  - native_event: AfterAgent
    field: stop_hook_active
    type: boolean
    meaning: "True when the hook is already running as part of a retry sequence."
  - native_event: BeforeModel
    field: llm_request.model
    type: string
    meaning: "Stable hook model ID before the LLM request is sent."
  - native_event: BeforeModel
    field: llm_request.messages
    type: array
    meaning: "Stable hook message list with user/model/system roles and text content."
  - native_event: BeforeModel
    field: llm_request.config
    type: object
    meaning: "Stable hook generation parameters such as temperature."
  - native_event: BeforeModel
    field: llm_request.toolConfig
    type: object
    meaning: "Tool selection config included in the stable request format."
  - native_event: BeforeToolSelection
    field: llm_request.toolConfig
    type: object
    meaning: "Current tool selection config before the model chooses tools."
  - native_event: AfterModel
    field: llm_response.candidates
    type: array
    meaning: "Model response candidates for the current chunk."
  - native_event: AfterModel
    field: llm_response.usageMetadata
    type: object
    meaning: "Usage metadata such as totalTokenCount for the current chunk."
  - native_event: BeforeTool
    field: tool_name
    type: string
    meaning: "Provider-native tool name, including MCP tools."
  - native_event: BeforeTool
    field: tool_input
    type: object
    meaning: "Raw tool arguments generated by the model."
  - native_event: BeforeTool
    field: mcp_context.server_name
    type: string
    meaning: "MCP server name for MCP-backed tools."
  - native_event: BeforeTool
    field: mcp_context.tool_name
    type: string
    meaning: "Original tool name from the MCP server."
  - native_event: BeforeTool
    field: mcp_context.command
    type: string
    meaning: "stdio transport command for the MCP server, when applicable."
  - native_event: BeforeTool
    field: mcp_context.args
    type: array
    meaning: "stdio transport args for the MCP server, when applicable."
  - native_event: BeforeTool
    field: mcp_context.cwd
    type: string
    meaning: "stdio transport cwd for the MCP server, when applicable."
  - native_event: BeforeTool
    field: mcp_context.url
    type: string
    meaning: "SSE/HTTP transport URL for the MCP server, when applicable."
  - native_event: BeforeTool
    field: mcp_context.tcp
    type: string
    meaning: "WebSocket transport address for the MCP server, when applicable."
  - native_event: BeforeTool
    field: original_request_name
    type: string
    meaning: "Original tool request name when the call is a tail tool call."
  - native_event: AfterTool
    field: tool_response.llmContent
    type: unknown
    meaning: "Tool result content intended for the model."
  - native_event: AfterTool
    field: tool_response.returnDisplay
    type: unknown
    meaning: "Tool result content intended for display."
  - native_event: AfterTool
    field: tool_response.error
    type: unknown
    meaning: "Tool error payload when execution failed."
  - native_event: PreCompress
    field: trigger
    type: enum(auto,manual)
    meaning: "Whether compression was automatic or user-triggered."
  - native_event: Notification
    field: notification_type
    type: enum(ToolPermission)
    meaning: "Kind of system alert."
  - native_event: Notification
    field: message
    type: string
    meaning: "Human-readable notification summary."
  - native_event: Notification
    field: details
    type: object
    meaning: "Alert-specific metadata such as tool name or file path."
response_actions:
  - action: allow
    native_value: "exit 0 with decision: allow/approve, or no decision object"
    effect: "Allows the provider action to continue. Default when no blocking/ask decision is returned."
  - action: deny
    native_value: "exit 0 with JSON decision: deny/block and reason"
    effect: "Blocks or rejects the event-specific action. For AfterTool the real result is hidden and reason replaces it; for BeforeTool/BeforeAgent/BeforeModel/AfterModel the turn is aborted; for AfterAgent the response is rejected and a retry is triggered."
  - action: block
    native_value: "exit 2"
    effect: "System block. The target action is aborted and stderr is used as the rejection reason. For BeforeTool/AfterTool the turn continues."
  - action: ask
    native_value: "exit 0 with JSON decision: ask"
    effect: "Recognized by the decision union but primarily meaningful for permission-style hooks; public reference does not define full ask semantics."
  - action: stop
    native_value: "exit 0 with JSON continue: false and optional stopReason"
    effect: "Stops the entire agent loop immediately. Ignored by advisory hooks (SessionStart, SessionEnd, PreCompress, Notification)."
  - action: modify
    native_value: "hookSpecificOutput.tool_input"
    effect: "For BeforeTool, merges with and overrides the model's arguments before execution."
  - action: modify
    native_value: "hookSpecificOutput.llm_request"
    effect: "For BeforeModel, overrides parts of the outgoing request (shallow merge)."
  - action: modify
    native_value: "hookSpecificOutput.toolConfig"
    effect: "For BeforeToolSelection, sets mode and allowedFunctionNames."
  - action: replace
    native_value: "hookSpecificOutput.llm_response"
    effect: "For BeforeModel, synthesizes a response and skips the model call. For AfterModel, replaces the current streamed chunk."
  - action: replace
    native_value: "hookSpecificOutput.tailToolCallRequest"
    effect: "For AfterTool, requests an immediate tail tool call whose result replaces the original tool response."
  - action: other
    native_value: "systemMessage"
    effect: "Displayed immediately to the user in the terminal."
  - action: other
    native_value: "suppressOutput: true"
    effect: "Hides internal hook metadata from logs/telemetry (any true wins across hooks)."
  - action: other
    native_value: "hookSpecificOutput.additionalContext"
    effect: "Appended to the prompt (BeforeAgent), session context (SessionStart), or tool result (AfterTool)."
  - action: other
    native_value: "hookSpecificOutput.clearContext: true"
    effect: "For AfterAgent, clears conversation history while preserving UI display."
execution:
  shell: "On macOS/Linux the hook command is passed to bash -c. On Windows it is passed to PowerShell (pwsh.exe preferred, then powershell.exe; ComSpec is checked first). Commands are spawned with spawn(executable, [...argsPrefix, command], { shell: false })."
  cwd: "Hook child process cwd is the input.cwd value, which is also exported as GEMINI_CWD."
  env: "Starts from sanitizeEnvironment(process.env), then sets GEMINI_PROJECT_DIR=input.cwd, GEMINI_PLANS_DIR, GEMINI_CWD=input.cwd, GEMINI_SESSION_ID, CLAUDE_PROJECT_DIR=input.cwd (compatibility alias), plus any hookConfig.env overrides."
  timeout: "Default 60000 ms. Per-handler timeout field in milliseconds overrides. On timeout Unix sends SIGTERM then SIGKILL after 5 seconds; Windows uses taskkill /f /t."
  stdin: "Receives the event input as a single JSON object on stdin, then stdin is closed."
  stdout: "Must contain only the final JSON object. Non-JSON stdout (or stderr fallback if stdout is empty) is converted to a structured HookOutput by the runner."
  stderr: "Used for logs and feedback. On exit 2 it becomes the rejection reason. The runner may parse stderr as fallback if stdout is empty."
  notes: "Matching hooks are deduplicated by name:command. If any matching hook definition has sequential: true, all matching hooks for that event run sequentially; otherwise they run in parallel. In sequential mode BeforeAgent, BeforeModel, and BeforeTool can pass modified input to later hooks. Aggregation uses OR for blocking decisions, ANY true for suppressOutput/clearContext, later-field replacement for BeforeModel/AfterModel, and UNION for BeforeToolSelection toolConfig."
gaps:
  - "Only command hooks are officially supported. The source also defines an internal runtime hook type used programmatically; HTTP/MCP/prompt/agent hook types do not exist in the public surface."
  - "SessionEnd is best-effort and not awaited; the exact async boundary relative to process exit is not documented."
  - "SessionStart hook execution path differs between interactive and non-interactive modes in the source."
  - "The runner's plain-text-to-structured-output fallback is implementation behavior not covered by the public hook reference."
  - "Claudine has no unified lifecycle events for BeforeModel, AfterModel, or BeforeToolSelection."
  - "No dedicated CLI flag disables hooks for a single session; the canonical toggle is the hooksConfig.enabled setting."
  - "Project hooks require the workspace folder to be trusted; untrusted folders silently skip project hooks."
  - "The ask decision value is present in the type union but its observable behavior is not fully documented."
changes:
  - "2026-07-03 — Refreshed against Gemini CLI 0.46.0 and official docs. Corrected hook handler types: only command is publicly supported; removed unsupported http/mcp_tool/prompt/agent claims."
  - "Narrowed hook configuration fields to type, command, name, description, timeout, env; removed args, shell, async, asyncRewake, statusMessage, once, if, allowedEnvVars, url, headers."
  - "Added hooksConfig.enabled and hooksConfig.disabled as the canonical hook on/off controls; corrected disabled hook location from hooks.disabled to hooksConfig.disabled."
  - "Added system-defaults.json config layer and GEMINI_CLI_SYSTEM_SETTINGS_PATH / GEMINI_CLI_SYSTEM_DEFAULTS_PATH env-var overrides."
  - "Corrected exit code semantics: exit 1 is a non-blocking warning, exit 2+ is blocking; public docs still label exit 2 as System Block."
  - "Documented source-level shell selection (bash on Unix, PowerShell on Windows), sequential execution, input chaining, deduplication, and aggregation rules."
  - "Added mcp_context fields and original_request_name to payload fields."
  - "Updated response_actions to reflect allow/approve/deny/block/ask union and clearContext/suppressOutput."
requires_claudine_update: true
reason: "The adapter and generated metadata must drop the unsupported http/mcp_tool/prompt/agent handler types and non-existent fields (args, shell, async, asyncRewake, statusMessage, once, if, allowedEnvVars). They must read hooks from hooksConfig.enabled/hooksConfig.disabled rather than the old hooks.disabled location, respect project-folder trust before installing or running project hooks, and model the GEMINI_CLI_SYSTEM_SETTINGS_PATH / GEMINI_CLI_SYSTEM_DEFAULTS_PATH path overrides plus the system-defaults.json layer. The plain-text stdout fallback, sequential chaining, and event-specific aggregation rules should also be reflected if the adapter reimplements hook execution semantics."
---

# Gemini CLI Hooks

## Overview

Gemini CLI provides a command-hook system that runs external shell commands at fixed lifecycle points. As of v0.46.0 the only publicly supported handler type is `command`; the source also contains an internal `runtime` type used programmatically by the CLI itself. Hooks receive a JSON payload on stdin and return a JSON object on stdout. Exit code `0` is the normal structured path (including intentional blocks), exit code `1` is a non-blocking warning, and exit code `2` (or any other non-zero code) blocks the action.

Hooks can add context, validate tool arguments, block tool calls and turns, modify outgoing LLM requests and tool inputs, synthesize LLM responses, replace streamed response chunks, request tail tool calls, and stop the agent loop. They cannot make SessionStart/SessionEnd/PreCompress/Notification block or modify the underlying provider action.

## Native Hooks

Gemini CLI exposes 11 provider-native hook events:

| Event | Timing | Can block | Matcher |
|-------|--------|-----------|---------|
| `SessionStart` | pre | no | exact source string (`startup`, `resume`, `clear`) or `*` |
| `SessionEnd` | async | no | exact reason string or `*` |
| `BeforeAgent` | pre | yes | none (fires on every prompt) |
| `AfterAgent` | post | yes | none |
| `BeforeModel` | pre | yes | none |
| `AfterModel` | post | yes | none |
| `BeforeToolSelection` | pre | no | none |
| `BeforeTool` | pre | yes | regex against `tool_name` |
| `AfterTool` | post | yes | regex against `tool_name` |
| `PreCompress` | async | no | exact trigger (`auto`, `manual`) or `*` |
| `Notification` | async | no | exact `notification_type` or `*` |

### Matcher rules

- Tool events (`BeforeTool`, `AfterTool`): `matcher` is evaluated as a JavaScript RegExp against `tool_name`. Invalid regex falls back to exact string equality.
- Lifecycle events (`SessionStart`, `SessionEnd`, `PreCompress`, `Notification`): `matcher` is an exact string match against the trigger/source/reason/type field.
- `*` or empty string matches all occurrences.
- Built-in tools use names like `read_file`, `run_shell_command`, `write_file`. MCP tools use names like `mcp_<server_name>_<tool_name>`.

### Handler configuration

A hook definition groups one or more handlers under a matcher:

```json
{
  "hooks": {
    "BeforeTool": [
      {
        "matcher": "write_file|replace",
        "sequential": false,
        "hooks": [
          {
            "type": "command",
            "name": "security-check",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/security.sh",
            "timeout": 5000,
            "description": "Checks write operations",
            "env": { "MODE": "strict" }
          }
        ]
      }
    ]
  }
}
```

Supported command-hook fields are `type`, `command`, `name`, `description`, `timeout`, and `env`. The `sequential` flag on the definition controls the whole matching group: if any matching definition sets `sequential: true`, all matching hooks for that event run sequentially; otherwise they run in parallel.

## Configuration

Hooks are configured in JSON settings files. Gemini CLI loads settings in precedence order (lowest to highest): system defaults, user settings, project settings, system overrides. Extension hooks are loaded from active extensions and take part in the registry with their own source priority.

### File locations

| Scope | macOS | Linux | Windows |
|-------|-------|-------|---------|
| System defaults | `/Library/Application Support/GeminiCli/system-defaults.json` | `/etc/gemini-cli/system-defaults.json` | `C:\ProgramData\gemini-cli\system-defaults.json` |
| User | `~/.gemini/settings.json` | `~/.gemini/settings.json` | `%USERPROFILE%\.gemini\settings.json` |
| Project | `.gemini/settings.json` | `.gemini/settings.json` | `.gemini\settings.json` |
| System override | `/Library/Application Support/GeminiCli/settings.json` | `/etc/gemini-cli/settings.json` | `C:\ProgramData\gemini-cli\settings.json` |
| Extension | `<extension_dir>/hooks/hooks.json` | `<extension_dir>/hooks/hooks.json` | `<extension_dir>\hooks\hooks.json` |

The system paths can be redirected via `GEMINI_CLI_SYSTEM_SETTINGS_PATH` and `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`.

### Enabling and disabling hooks

- `hooksConfig.enabled` (boolean, default `true`): global on/off switch for the hook system.
- `hooksConfig.disabled` (string array): list of hook names that are disabled even when configured.
- `hooksConfig.notifications` (boolean, default `true`): show visual indicators while hooks run.
- In-session slash commands: `/hooks panel`, `/hooks enable-all`, `/hooks disable-all`, `/hooks enable <name>`, `/hooks disable <name>`.
- CLI command: `gemini hooks migrate` migrates hooks from Claude Code format.

### Trust and security

Project-level hooks in `.gemini/settings.json` are only executed when the current folder is trusted. Gemini CLI fingerprints project hooks; if a hook's name or command changes, the user is warned before it executes.

## Payloads and Responses

### Common input fields

Every hook receives:

```json
{
  "session_id": "...",
  "transcript_path": "...",
  "cwd": "...",
  "hook_event_name": "...",
  "timestamp": "..."
}
```

### Common output fields

```json
{
  "systemMessage": "...",
  "suppressOutput": false,
  "continue": true,
  "stopReason": "...",
  "decision": "allow",
  "reason": "...",
  "hookSpecificOutput": { }
}
```

- `systemMessage`: displayed immediately to the user.
- `suppressOutput`: hides internal hook metadata.
- `continue: false`: stops the entire agent loop.
- `decision`: `allow`, `approve`, `deny`, `block`, or `ask`.
- `reason`: required when blocking or denying.

### Per-event payloads and response contracts

- **SessionStart**: adds `source: startup | resume | clear`. Returns `additionalContext` and `systemMessage`; flow-control fields are ignored.
- **SessionEnd**: adds `reason: exit | clear | logout | prompt_input_exit | other`. Best-effort; flow-control fields ignored.
- **BeforeAgent**: adds `prompt`. Can inject `additionalContext`, block the turn with `decision: deny` or exit 2, or keep history with `continue: false`.
- **AfterAgent**: adds `prompt`, `prompt_response`, `stop_hook_active`. `decision: deny` rejects the response and forces a retry. `continue: false` stops. `clearContext: true` clears history.
- **BeforeModel**: adds `llm_request {model, messages, config, toolConfig}`. Can override the request via `llm_request`, synthesize a response via `llm_response`, or block with `decision: deny`/exit 2.
- **AfterModel**: adds `llm_request` and `llm_response {candidates, usageMetadata}`. Can replace the current chunk via `llm_response`, block the turn, or stop the loop.
- **BeforeToolSelection**: adds `llm_request`. Can only set `hookSpecificOutput.toolConfig {mode, allowedFunctionNames}`; `decision`, `continue`, and `systemMessage` are not supported.
- **BeforeTool**: adds `tool_name`, `tool_input`, optional `mcp_context`, optional `original_request_name`. Can block with `decision: deny`/exit 2, modify arguments via `tool_input`, or stop the loop.
- **AfterTool**: adds `tool_name`, `tool_input`, `tool_response {llmContent, returnDisplay, error}`, optional `mcp_context`, optional `original_request_name`. Can hide the result, append context, request a tail tool call, or stop the loop.
- **PreCompress**: adds `trigger: auto | manual`. Advisory only.
- **Notification**: adds `notification_type`, `message`, `details`. Advisory only.

### Exit codes

| Exit code | Meaning | Effect |
|-----------|---------|--------|
| 0 | Success | stdout parsed as JSON |
| 1 | Warning | non-blocking; proceeds with a warning |
| 2 or other non-zero | Block/error | blocks the action; stderr becomes the reason |

## Execution Semantics

Command hooks are spawned with `shell: false`. On macOS and Linux the executable is `bash` with argument `-c`. On Windows the executable is PowerShell: `ComSpec` is checked first if it points to a PowerShell executable, otherwise `pwsh.exe` is preferred over `powershell.exe`. PowerShell is invoked with `-NoProfile -NonInteractive -Command`.

The child receives the JSON payload on stdin. The working directory is the session's current working directory. The environment is sanitized from the CLI process, then augmented with `GEMINI_PROJECT_DIR`, `GEMINI_PLANS_DIR`, `GEMINI_CWD`, `GEMINI_SESSION_ID`, and `CLAUDE_PROJECT_DIR`, plus any per-hook `env` overrides.

The default timeout is 60 seconds. Timeouts on Unix send `SIGTERM` followed by `SIGKILL` after 5 seconds; on Windows `taskkill /f /t` is used.

Matching hooks are deduplicated by `name:command`. If any matching definition sets `sequential: true`, the whole set runs sequentially; otherwise hooks run in parallel. In sequential execution, `BeforeAgent`, `BeforeModel`, and `BeforeTool` can pass modified inputs to later hooks.

Aggregation rules when multiple hooks fire:

- Blocking decisions use OR logic: one `deny`/`block` blocks.
- `suppressOutput` and `clearContext`: any true wins.
- `BeforeModel`/`AfterModel`: later hook fields replace earlier ones.
- `BeforeToolSelection`: function names are unioned; mode precedence is `NONE` > `ANY` > `AUTO`.
- `additionalContext` strings are concatenated.

## Claudine Mapping

| Gemini event | Claudine event | Notes |
|--------------|----------------|-------|
| `SessionStart` | `initialize` | `source` preserved as provider extension. |
| `SessionEnd` | `finalize` | Best-effort async; `reason` preserved. |
| `BeforeAgent` | `prompt` | Blockable prompt submission gate. |
| `AfterAgent` | `success` | Retry-on-deny maps to a Claudine `success` with provider-specific retry behavior. |
| `BeforeModel` | `unknown` | No unified LLM-request event. |
| `AfterModel` | `unknown` | No unified LLM-response-chunk event. |
| `BeforeToolSelection` | `unknown` | No unified tool-availability event. |
| `BeforeTool` | `tool_call` | Blockable; `tool_name` + `tool_input` preserved. |
| `AfterTool` | `tool_result` | Blockable result visibility; `tool_response` preserved. |
| `PreCompress` | `notification` | Async advisory. |
| `Notification` | `notification` | Async advisory; `notification_type` preserved. |

Provider-specific fields that should round-trip on the unified payload include `source`, `reason`, `stop_hook_active`, `llm_request`, `llm_response`, `toolConfig`, `mcp_context`, `original_request_name`, `trigger`, `notification_type`, and `details`.

## Gaps

1. Only `command` hooks are officially supported; `runtime` is internal. Claims about HTTP/MCP/prompt/agent hooks are unsupported by current source and docs.
2. `SessionEnd` is best-effort and the exact async boundary is not documented.
3. `SessionStart` execution path differs between interactive and non-interactive modes.
4. The runner's plain-text-to-JSON fallback is implementation behavior, not a public contract.
5. `BeforeModel`, `AfterModel`, and `BeforeToolSelection` have no Claudine unified equivalents.
6. No dedicated per-session CLI flag disables hooks; the canonical toggle is `hooksConfig.enabled`.
7. Project hooks depend on folder trust; untrusted folders silently skip them.
8. The `ask` decision value is in the type union but its observable semantics are not fully documented.

## Changelog

- **2026-07-03** — Refreshed against Gemini CLI 0.46.0 and official docs. Corrected hook handler types to command-only (plus internal runtime). Narrowed hook config fields. Added `hooksConfig.enabled`/`hooksConfig.disabled` controls. Added system-defaults.json layer and env-var path overrides. Corrected exit code semantics. Documented source-level shell selection, sequential execution, deduplication, and aggregation rules.

## Sources

- [Gemini CLI hooks overview](https://geminicli.com/docs/hooks/)
- [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [google-gemini/gemini-cli `packages/core/src/hooks/types.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/types.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookRunner.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookRunner.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookRegistry.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookRegistry.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookPlanner.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookPlanner.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookAggregator.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookAggregator.ts)
- [google-gemini/gemini-cli `packages/core/src/utils/shell-utils.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/utils/shell-utils.ts)
- [google-gemini/gemini-cli `packages/core/src/config/storage.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/config/storage.ts)
- [google-gemini/gemini-cli `packages/cli/src/config/settings.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/config/settings.ts)
- [google-gemini/gemini-cli `packages/cli/src/config/settingsSchema.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/config/settingsSchema.ts)
- [google-gemini/gemini-cli `packages/cli/src/config/config.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/config/config.ts)
- [google-gemini/gemini-cli `packages/cli/src/gemini.tsx`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/gemini.tsx)
- [google-gemini/gemini-cli `packages/cli/src/commands/hooks.tsx`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/commands/hooks.tsx)
- Host observation: `~/.gemini/settings.json` exists with `"hooks": {}` and no active hook configurations.
