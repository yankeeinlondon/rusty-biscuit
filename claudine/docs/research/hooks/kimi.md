---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://www.kimi.com/code/
docs: https://moonshotai.github.io/kimi-cli/en/
hooks_docs: https://moonshotai.github.io/kimi-cli/en/customization/hooks.md

hooks:
  - native_event: PreToolUse
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "session_id, cwd, hook_event_name, tool_name, tool_input, tool_call_id"
    return_contract: "Exit 0 allows. Exit 2 blocks and uses stderr as the reason. Exit 0 + JSON {hookSpecificOutput:{permissionDecision:'deny',permissionDecisionReason:'...'}} also blocks."
    notes: "Matcher regex is applied to tool_name. Blocking returns a ToolError to the agent loop."
  - native_event: PostToolUse
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, tool_name, tool_input, tool_output, tool_call_id"
    return_contract: "Informational only; the tool has already executed. The engine still parses JSON deny but the action is not used to reverse the call."
    notes: "Fire-and-forget. tool_output is truncated to 2000 characters in the source."
  - native_event: PostToolUseFailure
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, tool_name, tool_input, error, tool_call_id"
    return_contract: "Informational only."
    notes: "Fire-and-forget. error is str(e) from the tool runtime."
  - native_event: UserPromptSubmit
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "session_id, cwd, hook_event_name, prompt"
    return_contract: "Exit 0 allows. Exit 2 or JSON deny blocks; the reason is rendered as a user-visible turn response."
    notes: "Skips non-string user input (e.g., injected system reminders). Matcher value is the full prompt text, but the matcher field is still applied as a regex."
  - native_event: Stop
    claudine_event: finalize
    timing: post
    blocking: true
    payload_schema: "session_id, cwd, hook_event_name, stop_hook_active"
    return_contract: "Exit 2 or JSON deny blocks; the reason is injected as a new user message so the agent runs one more turn. Only one re-trigger is permitted."
    notes: "Fires after a turn completes. The engine sets stop_hook_active=true while re-running the turn to prevent infinite loops."
  - native_event: StopFailure
    claudine_event: failure
    timing: post
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, error_type, error_message"
    return_contract: "Informational only; output is not fed back to the model."
    notes: "Matcher regex is applied to error_type, which is the exception class name."
  - native_event: SessionStart
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, source"
    return_contract: "Informational only."
    notes: "source is startup or resume, matching the matcher_value passed by the CLI. Fires after session initialization succeeds but before the main interaction loop."
  - native_event: SessionEnd
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, reason"
    return_contract: "Informational only; the session is already ending."
    notes: "The source hardcodes reason='exit' and matcher_value='exit'. The CLI waits up to 5 seconds for this hook."
  - native_event: SubagentStart
    claudine_event: subagent_start
    timing: pre
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, agent_name, prompt"
    return_contract: "Informational only."
    notes: "Matcher regex is applied to agent_name (the agent type name). prompt is truncated to 500 characters."
  - native_event: SubagentStop
    claudine_event: subagent_stop
    timing: post
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, agent_name, response"
    return_contract: "Observed to be fire-and-forget; the result is not awaited, so block decisions do not affect the parent turn."
    notes: "response is truncated to 500 characters. The current implementation uses fire_and_forget_trigger, which contradicts the documented ability to block."
  - native_event: PreCompact
    claudine_event: notification
    timing: pre
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, trigger, token_count"
    return_contract: "Informational only; cannot block compaction."
    notes: "trigger is manual or auto."
  - native_event: PostCompact
    claudine_event: notification
    timing: post
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, trigger, estimated_token_count"
    return_contract: "Informational only."
    notes: "Fire-and-forget counterpart of PreCompact."
  - native_event: Notification
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "session_id, cwd, hook_event_name, sink, notification_type, title, body, severity"
    return_contract: "Informational only."
    notes: "Fire-and-forget. Matcher regex is applied to notification_type. In the current source only sink='llm' is observed; background tasks emit types such as task.finished and task.failed."

config_files:
  - os: macos
    scope: user
    path: "~/.kimi/config.toml"
    format: toml
    notes: "Default user configuration documented at the researched site. The observed host file contains hooks = []."
  - os: linux
    scope: user
    path: "~/.kimi/config.toml"
    format: toml
    notes: "Same default user path as macOS."
  - os: windows
    scope: user
    path: '%USERPROFILE%\.kimi\config.toml'
    format: toml
    notes: "Windows equivalent of the default user configuration path."
  - os: macos
    scope: user
    path: "~/.kimi-code/config.toml"
    format: toml
    notes: "Observed on host for the migrated kimi-code binary; no hooks key is present. This path is not documented at the researched site."
  - os: linux
    scope: user
    path: "~/.kimi-code/config.toml"
    format: toml
    notes: "Observed on host for the migrated kimi-code binary; no hooks key is present. This path is not documented at the researched site."
  - os: windows
    scope: user
    path: '%USERPROFILE%\.kimi-code\config.toml'
    format: toml
    notes: "Likely Windows equivalent for the migrated kimi-code binary; not confirmed by inspection."

cli_params:
  - flag: "--config-file <path>"
    description: "Load a TOML or JSON configuration file from an arbitrary path, including any hooks array."
    example: "kimi --config-file ./kimi-config.toml"
  - flag: "--config '<content>'"
    description: "Pass complete configuration inline as JSON or TOML, including hooks."
    example: "kimi --config '{\"hooks\":[{\"event\":\"PreToolUse\",\"command\":\"...\"}]}'"
  - flag: "/hooks"
    description: "In-session slash command that lists configured hooks grouped by event with matcher and command."
    example: "/hooks"
  - flag: "--debug"
    description: "Enable TRACE-level logging, which includes hook execution and HookTriggered/HookResolved telemetry."
    example: "kimi --debug"

payload_fields:
  - native_event: "(common)"
    field: session_id
    type: string
    meaning: "Current session identifier, included in every event."
  - native_event: "(common)"
    field: cwd
    type: string
    meaning: "Working directory at the time the event was triggered, included in every event."
  - native_event: "(common)"
    field: hook_event_name
    type: string
    meaning: "Name of the native event, included in every event."
  - native_event: PreToolUse
    field: tool_name
    type: string
    meaning: "Name of the tool about to be called; drives matcher filtering."
  - native_event: PreToolUse
    field: tool_input
    type: object
    meaning: "Arguments passed to the tool; schema varies by tool_name."
  - native_event: PreToolUse
    field: tool_call_id
    type: string
    meaning: "Unique identifier for this tool call, used to correlate with PostToolUse/PostToolUseFailure."
  - native_event: PostToolUse
    field: tool_output
    type: string
    meaning: "String rendering of the tool result, truncated to 2000 characters by the source."
  - native_event: PostToolUseFailure
    field: error
    type: string
    meaning: "Error message from the failed tool execution."
  - native_event: UserPromptSubmit
    field: prompt
    type: string
    meaning: "The user-submitted text. Empty for non-string inputs."
  - native_event: Stop
    field: stop_hook_active
    type: boolean
    meaning: "Always false on first trigger; the engine uses an internal flag to prevent more than one re-trigger."
  - native_event: StopFailure
    field: error_type
    type: string
    meaning: "Exception class name (e.g., APIConnectionError)."
  - native_event: StopFailure
    field: error_message
    type: string
    meaning: "String rendering of the exception."
  - native_event: SessionStart
    field: source
    type: string
    meaning: "startup or resume."
  - native_event: SessionEnd
    field: reason
    type: string
    meaning: "Observed to always be exit in the current source."
  - native_event: SubagentStart
    field: agent_name
    type: string
    meaning: "Agent type name; drives matcher filtering."
  - native_event: SubagentStart
    field: prompt
    type: string
    meaning: "Subagent prompt, truncated to 500 characters."
  - native_event: SubagentStop
    field: response
    type: string
    meaning: "Subagent final response, truncated to 500 characters."
  - native_event: PreCompact
    field: trigger
    type: string
    meaning: "manual or auto."
  - native_event: PreCompact
    field: token_count
    type: integer
    meaning: "Context token count before compaction."
  - native_event: PostCompact
    field: estimated_token_count
    type: integer
    meaning: "Estimated context token count after compaction."
  - native_event: Notification
    field: sink
    type: string
    meaning: "Target sink, e.g., llm, wire, or shell. Current source only observes llm."
  - native_event: Notification
    field: notification_type
    type: string
    meaning: "Arbitrary type string; drives matcher filtering. Background tasks use task.<reason> types."
  - native_event: Notification
    field: severity
    type: string
    meaning: "One of info, success, warning, or error."

response_actions:
  - action: allow
    native_value: "Exit 0; any non-2 exit without a JSON deny payload; hook timeout or subprocess exception"
    effect: "Proceed with the action. stdout is captured by the engine but is not currently added to the model context."
  - action: block
    native_value: "Exit 2, or exit 0 + JSON {hookSpecificOutput:{permissionDecision:'deny',permissionDecisionReason:'...'}}"
    effect: "Cancel the pending tool call or prompt; stderr or permissionDecisionReason is shown to the user or returned to the LLM as feedback."
  - action: other
    native_value: "Timeout/CancelledError or unhandled exception in run_hook"
    effect: "Engine logs a warning and treats the result as allow (fail-open)."

execution:
  shell: "Platform default shell used by asyncio.create_subprocess_shell: /bin/sh on macOS/Linux, %COMSPEC% (typically cmd.exe) on Windows. No per-hook shell override field exists."
  cwd: "Session work_dir passed to HookEngine at creation (app.py). Each event payload also carries cwd=str(Path.cwd()) at trigger time."
  env: "Inherits the Kimi CLI process environment. No hook-specific environment variables are injected."
  timeout: "Default 30 seconds per hook; configurable via the timeout field with a hard maximum of 600 seconds. Fail-open on timeout."
  stdin: "A single JSON document containing the event payload."
  stdout: "On exit 0, parsed as JSON to detect hookSpecificOutput.permissionDecision == 'deny'. Plain stdout is otherwise unused by the engine, despite documentation stating it is added to context."
  stderr: "On exit 2, stripped and used as the block reason. On other non-zero exits it is logged but does not block."
  notes: "Matching server-side hooks run in parallel; identical commands are deduplicated by command string. Client-side wire subscriptions registered during ACP/wire initialize run alongside server hooks and return allow/block via JSON-RPC HookRequest/HookResponse."

gaps:
  - "Hooks are marked Beta and the implementation differs from the user-facing docs in several places (e.g., stdout context injection, SubagentStop blocking)."
  - "No documented environment variable disables hooks globally; there is no --bare or --safe-mode equivalent."
  - "No repository-scope or managed-scope hook configuration exists in the current source; hooks are only loaded from the user config file (or inline --config/--config-file)."
  - "SubagentStop is implemented as fire-and-forget, so block decisions are not awaited and have no effect on the parent turn."
  - "Plain stdout on exit 0 is documented as being added to context, but the engine only inspects it for JSON permissionDecision deny."
  - "The set of notification_type values is not enumerated; only background task types (task.finished, task.failed, etc.) are observed in source, while the docs use permission_prompt as an example."
  - "SessionEnd always carries reason='exit' in the source, contrary to the documented generic Reason matcher."
  - "The migrated kimi-code binary on the host uses ~/.kimi-code/config.toml, which is not covered by the researched documentation."
  - "Wire/client-side hook subscriptions exist in the source and are exercised by ACP/wire clients, but they are not documented in the public hooks page."

changes: []

requires_claudine_update: true
reason: "Claudine needs a Kimi-specific provider adapter that maps the 13 native events (plus wire subscriptions) to the unified 16-event lifecycle model, honors the allow/block-only return contract, implements the 30-second default timeout and fail-open semantics, and correctly models the source-level fire-and-forget behavior of SubagentStop/Notification/PostCompact."
---

# Kimi Code CLI hooks

## Overview

Kimi Code CLI provides a Beta server-side hook system plus an undocumented client-side wire hook system. Server hooks are shell commands declared in the user `config.toml` under a `[[hooks]]` array. They fire at 13 lifecycle points, receive a JSON payload on stdin, and return a decision via process exit code (and optionally a small JSON envelope). Client-side wire subscriptions are registered during ACP/wire initialize and forward the same payloads to the connected client as JSON-RPC `HookRequest` messages.

Capability summary:

- Handler kinds: shell commands (server) and JSON-RPC client callbacks (wire).
- Hooks can **block** pending tool calls and user prompts.
- Hooks can **observe** post-event, pre-compaction, session, subagent, and notification lifecycle.
- Hooks **cannot** mutate tool input, replace tool output, or switch permission modes.
- All hook failures (timeouts, crashes, invalid regex) are **fail-open**.

## Native Hooks

| Event | Timing | Blocking | Matcher target | Notes |
|-------|--------|----------|----------------|-------|
| `PreToolUse` | pre | yes | `tool_name` regex | Cancels the tool call on block. |
| `PostToolUse` | post | no | `tool_name` regex | Tool already ran; informational only. |
| `PostToolUseFailure` | post | no | `tool_name` regex | Tool failed; informational only. |
| `UserPromptSubmit` | pre | yes | prompt text regex | Blocks before the prompt is processed. |
| `Stop` | post | yes | none | Can continue the conversation for one extra turn. |
| `StopFailure` | post | no | `error_type` regex | Fires when a turn ends because of an error. |
| `SessionStart` | pre | no | `source` (`startup`/`resume`) | Fires after session init, before the main loop. |
| `SessionEnd` | post | no | `reason` (observed `exit`) | Runs in the session cleanup finally block. |
| `SubagentStart` | pre | no | `agent_name` regex | Fires before the subagent runs. |
| `SubagentStop` | post | no* | `agent_name` regex | *Implemented fire-and-forget; block is not awaited. |
| `PreCompact` | pre | no | `trigger` (`manual`/`auto`) | Informational; cannot block compaction. |
| `PostCompact` | post | no | `trigger` (`manual`/`auto`) | Fire-and-forget counterpart. |
| `Notification` | async | no | `notification_type` regex | Fire-and-forget notification delivery hook. |

### Matcher behavior

- The `matcher` field is a Python `re.search` regex.
- An empty or omitted `matcher` matches everything.
- For tool events the regex is tested against `tool_name`.
- For subagent events it is tested against `agent_name`.
- For `StopFailure` it is tested against `error_type`.
- For `Notification` it is tested against `notification_type`.
- For `SessionStart`/`PreCompact`/`PostCompact` it is tested against the documented discriminator (`source`/`trigger`).

## Configuration

### File locations

| Scope | macOS / Linux | Windows | Format |
|-------|---------------|---------|--------|
| User | `~/.kimi/config.toml` | `%USERPROFILE%\.kimi\config.toml` | TOML (or JSON) |

The observed host also has a migrated `~/.kimi-code/config.toml` used by the `kimi-code` binary; the researched documentation does not mention this path.

### Hook shape

```toml
[[hooks]]
event = "PreToolUse"
matcher = "Shell"
command = ".kimi/hooks/safety-check.sh"
timeout = 10
```

Supported fields per hook:

| Field | Required | Default | Range | Meaning |
|-------|----------|---------|-------|---------|
| `event` | yes | — | one of 13 events | When the hook fires. |
| `command` | yes | — | shell command | Receives JSON on stdin. |
| `matcher` | no | `""` | regex | Filters when the hook fires. |
| `timeout` | no | `30` | 1–600 seconds | Fail-open on timeout. |

### CLI switches and commands

- `kimi --config-file <path>` — load an alternate TOML/JSON config, including its hooks.
- `kimi --config '<json-or-toml>'` — pass configuration inline, including hooks.
- `/hooks` — in-session slash command that lists configured hooks by event, matcher, and command.
- `kimi --debug` — enables TRACE-level logs that include hook execution.

### Environment variables

No environment variable dedicated to disabling hooks was found in the researched documentation or source. `KIMI_SHARE_DIR` relocates runtime data but the documented config file path remains `~/.kimi/config.toml`. The `--yolo` and `--auto` flags change approval behavior but do not disable hooks.

## Payloads and Responses

### Common payload

Every event includes:

```json
{
  "session_id": "...",
  "cwd": "/current/working/dir",
  "hook_event_name": "PreToolUse"
}
```

### Per-event payload fields

| Event | Additional fields |
|-------|-------------------|
| `PreToolUse` | `tool_name`, `tool_input`, `tool_call_id` |
| `PostToolUse` | `tool_name`, `tool_input`, `tool_output` (truncated to 2000 chars), `tool_call_id` |
| `PostToolUseFailure` | `tool_name`, `tool_input`, `error`, `tool_call_id` |
| `UserPromptSubmit` | `prompt` |
| `Stop` | `stop_hook_active` |
| `StopFailure` | `error_type`, `error_message` |
| `SessionStart` | `source` (`startup`/`resume`) |
| `SessionEnd` | `reason` (observed `exit`) |
| `SubagentStart` | `agent_name`, `prompt` (truncated to 500 chars) |
| `SubagentStop` | `agent_name`, `response` (truncated to 500 chars) |
| `PreCompact` | `trigger` (`manual`/`auto`), `token_count` |
| `PostCompact` | `trigger` (`manual`/`auto`), `estimated_token_count` |
| `Notification` | `sink`, `notification_type`, `title`, `body`, `severity` |

### Response contract

| Exit code / output | Effect |
|--------------------|--------|
| `0` with empty or non-JSON stdout | Allow. |
| `0` with JSON `{hookSpecificOutput:{permissionDecision:"deny",permissionDecisionReason:"..."}}` | Block; reason is shown/returned. |
| `2` | Block; stderr is used as the reason. |
| Timeout, crash, invalid regex | Allow (fail-open). |

The engine only recognizes `permissionDecision: deny`; there is no support for `allow`, `ask`, `defer`, input mutation, or output replacement.

## Execution Semantics

- **Shell**: `asyncio.create_subprocess_shell` uses the platform default (`/bin/sh` on macOS/Linux, `%COMSPEC%` on Windows). No per-hook shell override exists.
- **Working directory**: the session `work_dir` passed to `HookEngine` at creation; each payload also records `cwd=str(Path.cwd())` at trigger time.
- **Environment**: the spawned shell inherits the Kimi CLI process environment; no hook-specific variables are injected.
- **Timeout**: default `30s`, configurable per hook (`1–600s`), fail-open on timeout.
- **Stdin**: one JSON document, the event payload.
- **Stdout/stderr**: see Response contract above.
- **Parallelism**: matching server hooks run in parallel; identical `command` strings are deduplicated.
- **Fire-and-forget**: `PostToolUse`, `PostToolUseFailure`, `StopFailure`, `PostCompact`, and `Notification` are triggered without awaiting results.
- **Wire hooks**: ACP/wire clients may subscribe to events during initialize. Matching subscriptions send a JSON-RPC `HookRequest` with the same `input_data`; the client replies with `action: "allow" | "block"` and an optional `reason`.

## Claudine Mapping

| Native event | Claudine event | Provider-specific fields to preserve |
|--------------|----------------|--------------------------------------|
| `PreToolUse` | `tool_call` | `tool_name`, `tool_input`, `tool_call_id` |
| `PostToolUse` | `tool_result` | `tool_output`, `tool_call_id`, `tool_input` |
| `PostToolUseFailure` | `tool_result` | `error`, `tool_call_id`, `tool_input` |
| `UserPromptSubmit` | `prompt` | `prompt` |
| `Stop` | `finalize` | `stop_hook_active` |
| `StopFailure` | `failure` | `error_type`, `error_message` |
| `SessionStart` | `initialize` | `source` |
| `SessionEnd` | `finalize` | `reason` |
| `SubagentStart` | `subagent_start` | `agent_name`, `prompt` |
| `SubagentStop` | `subagent_stop` | `agent_name`, `response` |
| `PreCompact` | `notification` | `trigger`, `token_count` |
| `PostCompact` | `notification` | `trigger`, `estimated_token_count` |
| `Notification` | `notification` | `sink`, `notification_type`, `title`, `body`, `severity` |

Many-to-one collisions (`tool_result`, `notification`, `finalize`) require Claudine to preserve the native event name and the provider-specific discriminator fields on the unified payload.

## Gaps

1. Hooks are Beta and the implementation diverges from the user docs (e.g., stdout context injection is not implemented; `SubagentStop` is fire-and-forget).
2. No global hook-disable switch (env var or CLI flag) was found.
3. No repository-scope or managed-scope hook configuration exists; hooks live only in the user config file or inline `--config`/`--config-file`.
4. `SubagentStop` cannot effectively block because the current source does not await its result.
5. Plain stdout on exit 0 is documented as context but is not consumed by the engine.
6. The set of `notification_type` values is not enumerated; only background `task.*` types are observed, while docs use `permission_prompt` as an example.
7. `SessionEnd` always carries `reason="exit"` in the source, despite the documented generic Reason matcher.
8. The migrated `kimi-code` binary on the host uses `~/.kimi-code/config.toml`, which is outside the researched documentation.
9. Wire/client-side hook subscriptions are present in the source but undocumented in the public hooks page.

## Sources

- [Hooks (Beta) documentation](https://moonshotai.github.io/kimi-cli/en/customization/hooks.md)
- [Config Files documentation](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md)
- [Data Locations documentation](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.md)
- [Environment Variables documentation](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.md)
- [Source: `src/kimi_cli/hooks/config.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/hooks/config.py)
- [Source: `src/kimi_cli/hooks/events.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/hooks/events.py)
- [Source: `src/kimi_cli/hooks/engine.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/hooks/engine.py)
- [Source: `src/kimi_cli/hooks/runner.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/hooks/runner.py)
- [Source: `src/kimi_cli/wire/jsonrpc.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/jsonrpc.py)
- [Source: `src/kimi_cli/wire/types.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/wire/types.py)
- [Source: `src/kimi_cli/soul/toolset.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/soul/toolset.py)
- [Source: `src/kimi_cli/soul/kimisoul.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/soul/kimisoul.py)
- [Source: `src/kimi_cli/subagents/runner.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/subagents/runner.py)
- [Source: `src/kimi_cli/cli/__init__.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/cli/__init__.py)
- Observed host configuration: `~/.kimi/config.toml` (contains `hooks = []`) and `~/.kimi-code/config.toml` (no hooks key)
