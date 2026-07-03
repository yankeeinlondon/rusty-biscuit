---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
hooks_docs: https://geminicli.com/docs/hooks/
hooks:
  - native_event: SessionStart
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "Base JSON plus source: startup | resume | clear."
    return_contract: "May return systemMessage and hookSpecificOutput.additionalContext; continue and decision are ignored."
    notes: "Runs when a session starts, resumes, or clears. Official docs say startup is never blocked."
  - native_event: SessionEnd
    claudine_event: finalize
    timing: async
    blocking: false
    payload_schema: "Base JSON plus reason: exit | clear | logout | prompt_input_exit | other."
    return_contract: "Best effort; systemMessage may display during shutdown; flow-control fields are ignored."
    notes: "Official reference says Gemini CLI will not wait for this hook to complete."
  - native_event: BeforeAgent
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "Base JSON plus prompt string."
    return_contract: "decision deny or exit 2 aborts the turn and removes the prompt from context; continue false blocks the turn but saves the message to history; hookSpecificOutput.additionalContext appends turn-local context."
    notes: "Closest Claudine mapping is prompt submission before provider planning."
  - native_event: AfterAgent
    claudine_event: success
    timing: post
    blocking: true
    payload_schema: "Base JSON plus prompt, prompt_response, and stop_hook_active boolean."
    return_contract: "decision deny or exit 2 rejects the final response and triggers an automatic retry using reason/stderr; continue false stops without retry; hookSpecificOutput.clearContext may clear conversation history."
    notes: "Maps to success when the final agent response is produced, but Gemini can turn a denial into a retry rather than a terminal failure."
  - native_event: BeforeModel
    claudine_event: unknown
    timing: pre
    blocking: true
    payload_schema: "Base JSON plus llm_request with model, messages, config, and toolConfig."
    return_contract: "hookSpecificOutput.llm_request modifies the outgoing request; hookSpecificOutput.llm_response supplies a synthetic response and skips the LLM call; decision deny or exit 2 blocks the turn."
    notes: "Claudine has no first-class LLM request lifecycle event in this schema."
  - native_event: AfterModel
    claudine_event: unknown
    timing: post
    blocking: true
    payload_schema: "Base JSON plus llm_request and llm_response; fires for each streamed response chunk."
    return_contract: "hookSpecificOutput.llm_response replaces the current response chunk; decision deny or exit 2 aborts the turn and discards model output; continue false stops the agent loop."
    notes: "Claudine has no first-class LLM response chunk lifecycle event in this schema."
  - native_event: BeforeToolSelection
    claudine_event: unknown
    timing: pre
    blocking: false
    payload_schema: "Base JSON plus llm_request in stable hook format."
    return_contract: "hookSpecificOutput.toolConfig can set mode AUTO | ANY | NONE and allowedFunctionNames; decision, continue, and systemMessage are not supported."
    notes: "Filters or forces tool availability before model tool choice; no direct Claudine lifecycle equivalent."
  - native_event: BeforeTool
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "Base JSON plus tool_name, tool_input, optional mcp_context, and optional original_request_name."
    return_contract: "decision deny/block or exit 2 prevents execution; reason/stderr is sent to the agent as tool error; hookSpecificOutput.tool_input merges into and overrides arguments; continue false stops the agent loop."
    notes: "Matchers are regular expressions against tool_name, with invalid regex falling back to exact match."
  - native_event: AfterTool
    claudine_event: tool_result
    timing: post
    blocking: true
    payload_schema: "Base JSON plus tool_name, tool_input, tool_response, optional mcp_context, and optional original_request_name."
    return_contract: "decision deny or exit 2 hides the real result and sends reason/stderr to the model; hookSpecificOutput.additionalContext appends context; hookSpecificOutput.tailToolCallRequest can run another tool and replace the original response."
    notes: "The underlying tool has already executed; blocking only controls what the model sees afterward."
  - native_event: PreCompress
    claudine_event: none
    timing: async
    blocking: false
    payload_schema: "Base JSON plus trigger: auto | manual."
    return_contract: "May return systemMessage; flow-control fields are ignored and compression cannot be blocked or modified."
    notes: "Fires before context summarization/compression; useful for telemetry or state saving."
  - native_event: Notification
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "Base JSON plus notification_type, message, and details object."
    return_contract: "May return systemMessage; cannot block alerts or grant permissions; flow-control fields are ignored."
    notes: "Currently documented notification_type is ToolPermission."
config_files:
  - os: macos
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    notes: "User-level hooks under the top-level hooks object."
  - os: linux
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    notes: "User-level hooks under the top-level hooks object."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\settings.json"
    format: json
    notes: "User-level hooks under the top-level hooks object."
  - os: macos
    scope: repo
    path: ".gemini/settings.json"
    format: json
    notes: "Project-level hook configuration; Gemini CLI fingerprints project hooks and warns when name or command changes."
  - os: linux
    scope: repo
    path: ".gemini/settings.json"
    format: json
    notes: "Project-level hook configuration; Gemini CLI fingerprints project hooks and warns when name or command changes."
  - os: windows
    scope: repo
    path: ".gemini\\settings.json"
    format: json
    notes: "Project-level hook configuration; Gemini CLI fingerprints project hooks and warns when name or command changes."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/settings.json"
    format: json
    notes: "System override settings path from official configuration docs."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/settings.json"
    format: json
    notes: "System override settings path from official configuration docs."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    format: json
    notes: "System override settings path from official configuration docs."
  - os: macos
    scope: other
    path: "<extension_dir>/hooks/hooks.json"
    format: json
    notes: "Extension hook definitions; extension docs say hooks are not declared in gemini-extension.json."
  - os: linux
    scope: other
    path: "<extension_dir>/hooks/hooks.json"
    format: json
    notes: "Extension hook definitions; extension docs say hooks are not declared in gemini-extension.json."
  - os: windows
    scope: other
    path: "<extension_dir>\\hooks\\hooks.json"
    format: json
    notes: "Extension hook definitions; extension docs say hooks are not declared in gemini-extension.json."
cli_params:
  - flag: "/hooks list"
    description: "Displays all registered hooks with their status; aliases are /hooks show and /hooks panel."
    example: "/hooks list"
  - flag: "/hooks disable-all"
    description: "Disables all enabled hooks."
    example: "/hooks disable-all"
  - flag: "/hooks enable-all"
    description: "Enables all disabled hooks."
    example: "/hooks enable-all"
  - flag: "/hooks disable <hook-name>"
    description: "Disables a named hook."
    example: "/hooks disable security-check"
  - flag: "/hooks enable <hook-name>"
    description: "Enables a named hook."
    example: "/hooks enable security-check"
payload_fields:
  - native_event: "*"
    field: session_id
    type: string
    meaning: "Unique ID for the current session."
  - native_event: "*"
    field: transcript_path
    type: string
    meaning: "Absolute path to the session transcript JSON when available; source initializes it to an empty string if unavailable."
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
    meaning: "Session start trigger."
  - native_event: SessionEnd
    field: reason
    type: enum(exit,clear,logout,prompt_input_exit,other)
    meaning: "Why the session is ending."
  - native_event: BeforeAgent
    field: prompt
    type: string
    meaning: "Original user prompt before agent planning."
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
    meaning: "Indicates that the hook is already running as part of a retry sequence."
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
    meaning: "Stable hook generation parameters."
  - native_event: BeforeToolSelection
    field: llm_request.toolConfig
    type: object
    meaning: "Current tool selection config before the model chooses tools."
  - native_event: AfterModel
    field: llm_response.candidates
    type: array
    meaning: "Stable hook model response candidates for the current chunk."
  - native_event: AfterModel
    field: llm_response.usageMetadata
    type: object
    meaning: "Stable hook usage metadata such as totalTokenCount."
  - native_event: BeforeTool
    field: tool_name
    type: string
    meaning: "Provider-native tool name, including MCP tool names."
  - native_event: BeforeTool
    field: tool_input
    type: object
    meaning: "Raw tool arguments generated by the model."
  - native_event: BeforeTool
    field: mcp_context.server_name
    type: string
    meaning: "MCP server name for MCP-backed tools."
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
    native_value: "exit 0 with no blocking decision, or JSON decision: allow/approve"
    effect: "Allows the provider action to continue; stdout JSON is parsed on exit 0."
  - action: deny
    native_value: "exit 0 with JSON decision: deny/block"
    effect: "Blocks or rejects the event-specific action while preserving structured output handling."
  - action: block
    native_value: "exit 2"
    effect: "System block; target action is aborted and stderr is used as the rejection reason. Event effect varies by hook."
  - action: ask
    native_value: "exit 0 with JSON decision: ask"
    effect: "Supported by the source type union and aggregation logic, but not fully described in the public reference for every event."
  - action: stop
    native_value: "exit 0 with JSON continue: false and optional stopReason"
    effect: "Requests that Gemini stop the current agent loop/session path; ignored by advisory hooks."
  - action: modify
    native_value: "hookSpecificOutput.tool_input"
    effect: "Merges with and overrides BeforeTool arguments before execution."
  - action: modify
    native_value: "hookSpecificOutput.llm_request"
    effect: "Overrides parts of the outgoing BeforeModel request."
  - action: replace
    native_value: "hookSpecificOutput.llm_response"
    effect: "BeforeModel can synthesize a response and skip the model call; AfterModel can replace the current response chunk."
  - action: modify
    native_value: "hookSpecificOutput.toolConfig"
    effect: "BeforeToolSelection can set tool selection mode and allowedFunctionNames."
  - action: replace
    native_value: "hookSpecificOutput.tailToolCallRequest"
    effect: "AfterTool can request an immediate tail tool call whose result replaces the original tool response."
  - action: other
    native_value: "systemMessage"
    effect: "Displays a user-facing system message without giving it to the model."
  - action: other
    native_value: "suppressOutput: true"
    effect: "Suppresses internal hook metadata/system-message display where supported."
execution:
  shell: "Command hooks run through bash -c on macOS/Linux. On Windows they run through PowerShell, preferring ComSpec when it is pwsh.exe/powershell.exe, then pwsh.exe, then powershell.exe; commands are spawned with shell:false."
  cwd: "Hook child process cwd is the hook input cwd, sourced from config.getWorkingDir(). Official env docs call GEMINI_PROJECT_DIR the project root, but source sets GEMINI_PROJECT_DIR and GEMINI_CWD to input.cwd."
  env: "Starts from sanitizeEnvironment(process.env), then sets GEMINI_PROJECT_DIR, GEMINI_PLANS_DIR, GEMINI_CWD, GEMINI_SESSION_ID, CLAUDE_PROJECT_DIR, and any hookConfig.env overrides."
  timeout: "Default 60000 ms; hook configuration timeout is milliseconds. On timeout, Unix sends SIGTERM then SIGKILL after 5 seconds; Windows uses taskkill /f /t."
  stdin: "Receives the event input as a single JSON object on stdin, then stdin is closed."
  stdout: "Expected to contain only final JSON on success. Source trims stdout first, then stderr if stdout is empty; non-JSON text is converted into a systemMessage/deny shape."
  stderr: "Documented for logs and feedback; exit 2 uses stderr as rejection reason. Source may parse stderr as fallback JSON/text if stdout is empty."
  notes: "Matching hooks are deduplicated by name+command. If any matching hook definition has sequential true, all matching hooks for that event run sequentially; otherwise they run in parallel. Sequential hooks can pass modified input to later hooks for BeforeAgent, BeforeModel, and BeforeTool."
gaps:
  - "Public docs say hooks are merged from project, user, system, and extensions. The inspected registry processes the already-merged main config as Project plus Extensions, so source-level attribution of user/system hook entries is not explicit at the registry boundary."
  - "Public docs describe stdout as JSON-only and stderr as logs, but current source falls back to parsing stderr if stdout is empty and converts plain text output into structured HookOutput. Adapters should not rely on this leniency unless intentionally modeling implementation behavior."
  - "BeforeToolSelection has no direct Claudine lifecycle equivalent; it is a tool-availability planning hook rather than a tool call, permission prompt, or model event in Claudine's current schema."
  - "BeforeModel and AfterModel expose LLM request/response lifecycle events that Claudine's current hook schema cannot represent directly."
  - "Source type union includes ask/approve decisions and runtime hooks; public hook docs emphasize command hooks and allow/deny/block. The durable public contract for ask/approve is unclear."
  - "The UI component inspected displays hook timeout with an s suffix, while docs, types, and runner all use milliseconds."
changes: []
requires_claudine_update: false
reason: ""
---

# Gemini CLI Hooks

## Overview

Gemini CLI has a first-class hook system with 11 provider-native events. Hooks are configured under `hooks` in `settings.json` or bundled in an extension `hooks/hooks.json`. Command hooks receive JSON on stdin and return JSON on stdout; exit code `0` is the normal structured path, exit code `2` is a system block, and other non-zero codes are warnings.

The public docs state that hooks run synchronously as part of the agent loop, but several events are explicitly advisory or best-effort: `SessionEnd` is not awaited, `PreCompress` is asynchronous, and `Notification` cannot control the alert or permission decision.

## Native Hooks

Gemini documents these hook events: `SessionStart`, `SessionEnd`, `BeforeAgent`, `AfterAgent`, `BeforeModel`, `AfterModel`, `BeforeToolSelection`, `BeforeTool`, `AfterTool`, `PreCompress`, and `Notification`.

`BeforeTool` and `AfterTool` are the closest fit for Claudine `tool_call` and `tool_result`. `BeforeAgent` maps cleanly to Claudine's prompt submission phase. `AfterAgent` is close to a success/final response event, but a denial causes a Gemini retry rather than simply reporting failure. `BeforeModel`, `AfterModel`, and `BeforeToolSelection` are important adapter gaps because they expose lower-level LLM request/response and tool-planning surfaces not represented in the current Claudine hook schema.

## Configuration

Hooks use the top-level `hooks` object in Gemini settings:

```json
{
  "hooks": {
    "BeforeTool": [
      {
        "matcher": "write_file|replace",
        "hooks": [
          {
            "name": "security-check",
            "type": "command",
            "command": "$GEMINI_PROJECT_DIR/.gemini/hooks/security.sh",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

Hook definitions support `matcher`, `sequential`, and `hooks`. Hook configs support `type`, `command`, `name`, `timeout`, and `description`. Public docs say only `command` hooks are supported; source also has a programmatic `runtime` hook type.

Tool-event matchers are regular expressions against the tool name, with invalid regex falling back to exact match in source. Lifecycle matchers are exact trigger/source strings. `*` and `""` match all. Project hooks are fingerprinted and warned on change; untrusted folders block project hook execution in source.

## Payloads and Responses

Every hook receives `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `timestamp`. Event-specific fields are listed in frontmatter.

Common output fields are `systemMessage`, `suppressOutput`, `continue`, `stopReason`, `decision`, `reason`, and event-specific `hookSpecificOutput`. The most important event-specific outputs are:

- `hookSpecificOutput.tool_input` for `BeforeTool` argument rewrite.
- `hookSpecificOutput.additionalContext` for `SessionStart`, `BeforeAgent`, and `AfterTool`.
- `hookSpecificOutput.tailToolCallRequest` for `AfterTool` tail-call replacement.
- `hookSpecificOutput.llm_request` and `hookSpecificOutput.llm_response` for `BeforeModel`.
- `hookSpecificOutput.llm_response` for `AfterModel` chunk replacement.
- `hookSpecificOutput.toolConfig` for `BeforeToolSelection`.

Gemini's stable hook model API represents requests as `model`, `messages`, `config`, and `toolConfig`, and responses as `candidates` plus `usageMetadata`.

## Execution Semantics

Command hooks run with stdin/stdout/stderr pipes. The source spawns a shell executable directly with `shell: false`: `bash -c` on macOS/Linux and PowerShell on Windows. Windows prefers a PowerShell-like `ComSpec`, then `pwsh.exe`, then `powershell.exe`.

The default timeout is 60,000 ms. Timeout termination is platform-specific: Unix receives SIGTERM then SIGKILL after 5 seconds; Windows uses `taskkill /f /t`.

Matching hooks normally run in parallel. If any matching hook definition sets `sequential: true`, the whole execution plan for that event is sequential. Source-level sequential chaining applies selected output changes to the next hook input for `BeforeAgent`, `BeforeModel`, and `BeforeTool`.

## Claudine Mapping

Gemini can support Claudine's prompt, tool-call, tool-result, notification, initialize, success, and finalize concepts with caveats. It also exposes lower-level model and tool-selection hooks that Claudine cannot currently model faithfully.

For blocking behavior, Claudine should prefer structured JSON on exit `0` when installing hooks because Gemini documents it as the preferred path, including intentional blocks. Exit `2` should be reserved for hard system blocks where stderr is the model/user feedback.

For hook installation, Claudine should write command hooks into `.gemini/settings.json` or `~/.gemini/settings.json` and use `name` values stable enough for `/hooks enable|disable <name>`. Extension hooks are portable but require packaging as a Gemini extension.

## Gaps

The main modeling gaps are `BeforeModel`, `AfterModel`, and `BeforeToolSelection`; they are materially useful but do not map to Claudine's current lifecycle event names. There is also a contract gap between public docs and source for stderr fallback parsing/plain-text conversion, and between public command-only docs and the source `runtime` hook type.

Session attribution is another implementation detail to treat carefully: settings docs list user, project, and system paths, while the inspected hook registry sees a merged main config plus extension configs. Claudine should install to documented settings files rather than depend on registry source labels.

## Sources

- [Gemini CLI hooks overview](https://geminicli.com/docs/hooks/)
- [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI slash commands reference](https://geminicli.com/docs/reference/commands/)
- [Gemini CLI extension reference](https://geminicli.com/docs/extensions/reference/)
- [google-gemini/gemini-cli `docs/hooks/reference.md`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/docs/hooks/reference.md)
- [google-gemini/gemini-cli `packages/core/src/hooks/types.ts`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/hooks/types.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookRunner.ts`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/hooks/hookRunner.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookRegistry.ts`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/hooks/hookRegistry.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookPlanner.ts`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/hooks/hookPlanner.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookAggregator.ts`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/hooks/hookAggregator.ts)
- [google-gemini/gemini-cli `packages/core/src/hooks/hookEventHandler.ts`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/hooks/hookEventHandler.ts)
- [google-gemini/gemini-cli `packages/core/src/utils/shell-utils.ts`](https://github.com/google-gemini/gemini-cli/blob/f7af4e5180cf92eea8190e383fd5daeeb2578c2d/packages/core/src/utils/shell-utils.ts)
