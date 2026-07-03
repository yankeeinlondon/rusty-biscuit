---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://github.com/openai/codex
docs: https://developers.openai.com/codex/cli/
hooks_docs: https://developers.openai.com/codex/hooks
hooks:
  - native_event: SessionStart
    claudine_event: start
    timing: pre
    blocking: false
    payload_schema: "Common stdin JSON (session_id, transcript_path, cwd, hook_event_name, model, permission_mode) plus source (startup|resume|clear|compact)."
    return_contract: "Exit 0 continues; stdout plain text or JSON hookSpecificOutput.additionalContext adds developer context; common output fields (continue/stopReason/systemMessage/suppressOutput) supported. Exit 2 shows stderr to user only; session still starts."
    notes: "Matcher applies to source. Fires at thread scope, including on every compaction (source=compact). Local ~/.codex/config.toml had no hooks and no ~/.codex/hooks.json existed on the research host."
  - native_event: UserPromptSubmit
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "Common stdin JSON plus turn_id and prompt."
    return_contract: "Exit 0 continues; stdout plain text or JSON hookSpecificOutput.additionalContext adds developer context; top-level {decision: 'block', reason} blocks prompt submission. Exit 2 with stderr blocks."
    notes: "Configured matcher is ignored."
  - native_event: PreToolUse
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "Common stdin JSON plus turn_id, tool_name, tool_use_id, and tool_input."
    return_contract: "Exit 0 continues unless JSON denies/blocks; hookSpecificOutput.permissionDecision=deny blocks; permissionDecision=allow with updatedInput can rewrite supported tool input; legacy top-level {decision: 'block', reason} or exit 2 with stderr blocks."
    notes: "Matcher applies to tool_name and aliases (Bash, apply_patch, Edit, Write, MCP names). Coverage is explicitly incomplete: unified_exec shell paths, WebSearch, and arbitrary non-shell/non-MCP tools may bypass interception."
  - native_event: PermissionRequest
    claudine_event: permission
    timing: pre
    blocking: true
    payload_schema: "Common stdin JSON plus turn_id, tool_name, tool_input, and optional tool_input.description."
    return_contract: "JSON hookSpecificOutput.decision.behavior allow approves; behavior deny rejects with message; no decision falls through to normal approval flow; any deny wins across matching hooks."
    notes: "Runs only when Codex is about to ask for approval. updatedInput, updatedPermissions, and interrupt are reserved/fail-closed today."
  - native_event: PostToolUse
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "Common stdin JSON plus turn_id, tool_name, tool_use_id, tool_input, and tool_response."
    return_contract: "Exit 0 continues; JSON additionalContext is model-visible; top-level {decision: 'block', reason} or exit 2 replaces the model-visible tool result with hook feedback; continue: false stops normal processing of original result."
    notes: "Cannot undo tool side effects. Matching support mirrors PreToolUse and includes non-zero Bash exits. updatedMCPToolOutput and suppressOutput are parsed but unsupported."
  - native_event: PreCompact
    claudine_event: none
    timing: pre
    blocking: true
    payload_schema: "Common stdin JSON plus turn_id and trigger; trigger is manual or auto."
    return_contract: "Exit 0 continues; JSON common output fields are supported; continue: false stops before compacting."
    notes: "No direct Claudine lifecycle equivalent for conversation compaction."
  - native_event: PostCompact
    claudine_event: none
    timing: post
    blocking: true
    payload_schema: "Common stdin JSON plus turn_id and trigger; trigger is manual or auto."
    return_contract: "Exit 0 continues; JSON common output fields are supported; continue: false stops after compacting."
    notes: "No direct Claudine lifecycle equivalent for conversation compaction."
  - native_event: SubagentStart
    claudine_event: subagent_start
    timing: pre
    blocking: false
    payload_schema: "Common stdin JSON plus turn_id, agent_id, agent_type, and permission_mode."
    return_contract: "Exit 0 continues; stdout text or JSON hookSpecificOutput.additionalContext adds developer context for the subagent; continue: false is parsed but does not stop the subagent."
    notes: "Matcher applies to agent_type."
  - native_event: SubagentStop
    claudine_event: subagent_stop
    timing: post
    blocking: true
    payload_schema: "Common stdin JSON plus turn_id, agent_id, agent_type, agent_transcript_path, stop_hook_active, and last_assistant_message."
    return_contract: "Exit 0 expects JSON; top-level {decision: 'block', reason} or exit 2 asks Codex to continue the subagent flow; continue: false takes precedence over continuation."
    notes: "Matcher applies to agent_type. Plain text stdout is invalid. Hooks must check stop_hook_active to avoid infinite loops."
  - native_event: Stop
    claudine_event: success
    timing: post
    blocking: true
    payload_schema: "Common stdin JSON plus turn_id, stop_hook_active, and last_assistant_message."
    return_contract: "Exit 0 expects JSON; top-level {decision: 'block', reason} or exit 2 tells Codex to continue by creating a new prompt from the reason; continue: false takes precedence."
    notes: "Configured matcher is ignored. This is turn completion, not necessarily process/session finalization. The generated input schema includes turn_id. Hooks must check stop_hook_active to avoid infinite loops."
config_files:
  - os: macos
    scope: user
    path: "~/.codex/hooks.json"
    format: json
    notes: "User hook file discovered next to the user config layer. Not present on the research host."
  - os: linux
    scope: user
    path: "~/.codex/hooks.json"
    format: json
    notes: "User hook file discovered next to the user config layer."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\hooks.json"
    format: json
    notes: "User hook file under CODEX_HOME/default home on Windows."
  - os: macos
    scope: user
    path: "~/.codex/config.toml"
    format: toml
    notes: "Inline [hooks] tables may be placed in config.toml. Research host file contained only model, projects, features, and plugins; no hooks table."
  - os: linux
    scope: user
    path: "~/.codex/config.toml"
    format: toml
    notes: "Inline [hooks] tables may be placed in config.toml."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\config.toml"
    format: toml
    notes: "Inline [hooks] tables may be placed in config.toml."
  - os: macos
    scope: repo
    path: ".codex/hooks.json"
    format: json
    notes: "Project-local hooks load only when the project .codex layer is trusted."
  - os: linux
    scope: repo
    path: ".codex/hooks.json"
    format: json
    notes: "Project-local hooks load only when the project .codex layer is trusted."
  - os: windows
    scope: repo
    path: ".codex\\hooks.json"
    format: json
    notes: "Project-local hooks load only when the project .codex layer is trusted."
  - os: macos
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: "Project-local inline [hooks] tables; trusted projects only."
  - os: linux
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: "Project-local inline [hooks] tables; trusted projects only."
  - os: windows
    scope: repo
    path: ".codex\\config.toml"
    format: toml
    notes: "Project-local inline [hooks] tables; trusted projects only."
  - os: macos
    scope: system
    path: "/etc/codex/hooks.json"
    format: json
    notes: "Inferred from hook discovery next to active config layers and documented Unix system config."
  - os: linux
    scope: system
    path: "/etc/codex/hooks.json"
    format: json
    notes: "Inferred from hook discovery next to active config layers and documented Unix system config."
  - os: macos
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    notes: "Documented Unix system config layer; inline [hooks] tables may be used."
  - os: linux
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    notes: "Documented Unix system config layer; inline [hooks] tables may be used."
  - os: windows
    scope: system
    path: unknown
    format: toml
    notes: "No Windows system config path was found in the official hook/config docs."
  - os: macos
    scope: managed
    path: "requirements.toml"
    format: toml
    notes: "Enterprise managed config can define inline [hooks], pin features.hooks, and set managed_dir."
  - os: linux
    scope: managed
    path: "requirements.toml"
    format: toml
    notes: "Enterprise managed config can define inline [hooks], pin features.hooks, and set managed_dir."
  - os: windows
    scope: managed
    path: "requirements.toml"
    format: toml
    notes: "Enterprise managed config can define inline [hooks] and windows_managed_dir."
  - os: macos
    scope: other
    path: "<plugin-root>/hooks/hooks.json"
    format: json
    notes: "Default plugin-bundled hook file when a plugin is enabled. Observed on host: ~/.codex/.tmp/plugins/plugins/figma/hooks.json and replayio/hooks.json."
  - os: linux
    scope: other
    path: "<plugin-root>/hooks/hooks.json"
    format: json
    notes: "Default plugin-bundled hook file when a plugin is enabled."
  - os: windows
    scope: other
    path: "<plugin-root>\\hooks\\hooks.json"
    format: json
    notes: "Default plugin-bundled hook file when a plugin is enabled."
cli_params:
  - flag: "/hooks"
    description: "Interactive CLI command to inspect hook sources, review/trust changed hooks, and disable individual non-managed hooks."
    example: "codex /hooks"
  - flag: "--dangerously-bypass-hook-trust"
    description: "Runs enabled hooks for one invocation without requiring persisted trust."
    example: "codex --dangerously-bypass-hook-trust \"run the repo checks\""
  - flag: "--disable hooks"
    description: "Disables the hooks feature for one invocation through the feature flag surface."
    example: "codex --disable hooks \"summarize this repository\""
  - flag: "--enable hooks"
    description: "Enables the hooks feature for one invocation through the feature flag surface."
    example: "codex --enable hooks \"run the test plan\""
  - flag: "-c features.hooks=false"
    description: "Disables hooks for one invocation with a direct config override."
    example: "codex -c features.hooks=false \"explain this file\""
  - flag: "features disable hooks"
    description: "Persists a disabled hooks feature flag in CODEX_HOME/config.toml."
    example: "codex features disable hooks"
  - flag: "features enable hooks"
    description: "Persists an enabled hooks feature flag in CODEX_HOME/config.toml."
    example: "codex features enable hooks"
  - flag: "--strict-config"
    description: "Errors when config.toml contains unrecognized fields, which can catch hook schema typos early."
    example: "codex --strict-config \"hello\""
payload_fields:
  - native_event: "*"
    field: session_id
    type: string
    meaning: "Current Codex session id; subagent hooks use the parent session id."
  - native_event: "*"
    field: transcript_path
    type: "string | null"
    meaning: "Path to the session transcript file if available; transcript format is not stable."
  - native_event: "*"
    field: cwd
    type: string
    meaning: "Working directory for the session and hook command."
  - native_event: "*"
    field: hook_event_name
    type: string
    meaning: "Native hook event name."
  - native_event: "*"
    field: model
    type: string
    meaning: "Active Codex model slug."
  - native_event: "*"
    field: permission_mode
    type: string
    meaning: "Current permission mode for supported events: default, acceptEdits, plan, dontAsk, or bypassPermissions (per generated JSON schema)."
  - native_event: SessionStart
    field: source
    type: string
    meaning: "How the session started: startup, resume, clear, or compact."
  - native_event: UserPromptSubmit
    field: turn_id
    type: string
    meaning: "Active Codex turn id."
  - native_event: UserPromptSubmit
    field: prompt
    type: string
    meaning: "User prompt about to be submitted."
  - native_event: PreToolUse
    field: tool_name
    type: string
    meaning: "Canonical hook tool name, such as Bash, apply_patch, or mcp__server__tool."
  - native_event: PreToolUse
    field: tool_use_id
    type: string
    meaning: "Tool-call id for this invocation."
  - native_event: PreToolUse
    field: tool_input
    type: JSON value
    meaning: "Tool-specific input; Bash/apply_patch use tool_input.command and MCP tools send arguments."
  - native_event: PermissionRequest
    field: tool_input.description
    type: "string | null"
    meaning: "Human-readable approval reason when Codex has one."
  - native_event: PostToolUse
    field: tool_response
    type: JSON value
    meaning: "Tool-specific output; for MCP tools, the MCP call result."
  - native_event: PreCompact
    field: trigger
    type: string
    meaning: "Compaction trigger: manual or auto."
  - native_event: PostCompact
    field: trigger
    type: string
    meaning: "Compaction trigger: manual or auto."
  - native_event: SubagentStart
    field: agent_id
    type: string
    meaning: "Identifier for the subagent."
  - native_event: SubagentStart
    field: agent_type
    type: string
    meaning: "Subagent type or profile."
  - native_event: SubagentStop
    field: agent_transcript_path
    type: "string | null"
    meaning: "Path to the subagent transcript file if available."
  - native_event: SubagentStop
    field: stop_hook_active
    type: boolean
    meaning: "Whether this subagent was already continued by a stop hook."
  - native_event: SubagentStop
    field: last_assistant_message
    type: "string | null"
    meaning: "Latest subagent assistant message if available."
  - native_event: Stop
    field: stop_hook_active
    type: boolean
    meaning: "Whether this turn was already continued by Stop."
  - native_event: Stop
    field: last_assistant_message
    type: "string | null"
    meaning: "Latest assistant message text if available."
  - native_event: Stop
    field: turn_id
    type: string
    meaning: "Active Codex turn id; present per generated stop.command.input.schema.json."
response_actions:
  - action: continue
    native_value: "exit 0 with no stdout"
    effect: "Marks the hook successful and lets Codex continue."
  - action: continue
    native_value: '{"continue": true}'
    effect: "Lets Codex continue; supported where common output fields apply."
  - action: block
    native_value: "exit 2 with stderr"
    effect: "Blocks or redirects event flow with stderr as the reason; exact effect varies by event."
  - action: block
    native_value: '{"decision":"block","reason":"..."}'
    effect: "Blocks UserPromptSubmit, blocks PreToolUse through the legacy shape, gives PostToolUse feedback, and continues Stop/SubagentStop with the reason as continuation text."
  - action: deny
    native_value: '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"..."}}'
    effect: "Denies a supported tool call before it runs."
  - action: allow
    native_value: '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}'
    effect: "Approves an approval request without showing the normal prompt."
  - action: deny
    native_value: '{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"..."}}}'
    effect: "Denies an approval request; any deny wins across matching hooks."
  - action: modify
    native_value: '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":{"command":"..."}}}'
    effect: "Rewrites supported Bash/apply_patch command input or MCP arguments before the tool runs."
  - action: other
    native_value: '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"..."}}'
    effect: "Adds model-visible developer context for SessionStart, UserPromptSubmit, SubagentStart, or PostToolUse."
  - action: stop
    native_value: '{"continue": false, "stopReason":"..."}'
    effect: "Stops or redirects processing for supported events; unsupported on PreToolUse/PermissionRequest and non-blocking on SubagentStart."
execution:
  shell: "Command hooks use a command string; examples use shell substitution. Windows can override with commandWindows/command_windows."
  cwd: "Commands run with the session cwd as their working directory."
  env: "Inherits the Codex process environment; plugin hooks additionally receive PLUGIN_ROOT, PLUGIN_DATA, CLAUDE_PLUGIN_ROOT, and CLAUDE_PLUGIN_DATA."
  timeout: "timeout is seconds; default is 600 seconds when omitted."
  stdin: "Every command hook receives one JSON object on stdin."
  stdout: "Plain text or JSON semantics are event-specific; empty stdout with exit 0 is success."
  stderr: "For exit 2, stderr is used as the blocking/feedback reason."
  notes: "Matching hooks from multiple files all run. Multiple matching command hooks for the same event launch concurrently. Hooks are enabled by default; async, prompt, and agent handlers are parsed but skipped today. Non-managed hooks require trust review unless bypassed. Generated JSON schemas confirm permission_mode enum is default|acceptEdits|plan|dontAsk|bypassPermissions and PreToolUse permissionDecision enum is allow|deny|ask."
gaps:
  - "Official docs do not document a Windows system config path equivalent to /etc/codex."
  - "The exact shell used to execute command strings is not specified in the public hook docs; command substitution examples imply shell evaluation."
  - "PreToolUse/PostToolUse coverage is explicitly incomplete for some shell paths and does not cover WebSearch or arbitrary non-shell/non-MCP tool paths."
  - "Transcript file format is explicitly not stable and should not be used as an adapter contract."
  - "Async command hooks, prompt handlers, agent handlers, suppressOutput, PreToolUse ask/continue/stopReason, PermissionRequest updatedPermissions/interrupt, and PostToolUse updatedMCPToolOutput are parsed but unsupported or fail today."
  - "Local inspection found no user hooks configured (~/.codex/hooks.json absent, ~/.codex/config.toml had no [hooks] table), so observed behavior is limited to plugin-bundled examples."
changes:
  - "2026-07-03: Refreshed against Codex CLI 0.142.5, official hooks docs, and generated JSON schemas. Added local inspection findings (no user hooks present; plugin examples observed). Corrected permission_mode enum to schema values (no auto). Confirmed Stop carries turn_id per generated schema. Added --strict-config as a hook-affecting CLI control."
requires_claudine_update: true
reason: "Claudine's current Codex configurator still registers only the legacy notify turn_complete hook, while current Codex supports first-class hooks.json/config.toml events with blocking, mutation, permission, subagent, compaction, and Stop semantics."
---

# Codex CLI Hooks and Events

## Overview

Codex ships a first-class hook system documented at `https://developers.openai.com/codex/hooks`. Hooks are enabled by default and can be disabled with `[features].hooks = false`; `codex_hooks` remains a deprecated alias. The current system is materially different from the older `notify` integration: command hooks receive JSON on `stdin`, can run at several lifecycle points, and some events can block, modify, or continue execution.

Runtime details that matter for Claudine:

- Hooks are discovered from `hooks.json`, inline `[hooks]` tables in `config.toml`, managed `requirements.toml`, and enabled plugin bundles.
- Matching hooks from all active sources run; higher-precedence config layers do not replace lower-precedence hooks.
- Multiple matching command hooks for the same event launch concurrently.
- Non-managed hooks require trust review through `/hooks`; managed hooks are trusted by policy.
- Only `type = "command"` handlers run today.

## Native Hooks

Codex documents ten native hook events:

- `SessionStart`: start/resume/clear/compact scope. Can add developer context.
- `UserPromptSubmit`: pre-prompt hook. Can add context or block prompt submission.
- `PreToolUse`: pre-tool hook for supported Bash, `apply_patch`, and MCP calls. Can deny or rewrite supported input.
- `PermissionRequest`: approval-path hook. Can allow, deny, or decline to decide.
- `PostToolUse`: post-tool hook. Can add context, replace the model-visible result with feedback, or stop normal result processing.
- `PreCompact` and `PostCompact`: conversation compaction hooks.
- `SubagentStart` and `SubagentStop`: subagent lifecycle hooks.
- `Stop`: turn-completion hook. A block decision asks Codex to continue with a new prompt built from the reason.

The tool hooks are useful but not a complete security boundary. The docs explicitly say interception is incomplete for richer shell paths and non-shell/non-MCP tools such as `WebSearch`.

## Configuration

Hook configuration has three levels: event, matcher group, and hook handler. In JSON:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "^Bash$",
        "hooks": [
          {
            "type": "command",
            "command": "/usr/bin/python3 \"$(git rev-parse --show-toplevel)/.codex/hooks/pre_tool_use_policy.py\"",
            "timeout": 30,
            "statusMessage": "Checking Bash command"
          }
        ]
      }
    ]
  }
}
```

The same shape can be written as inline TOML under `[[hooks.<Event>]]` and `[[hooks.<Event>.hooks]]`. Project hooks load only for trusted projects. Managed hooks can be delivered through `requirements.toml`, can pin `[features].hooks = true`, and can set `allow_managed_hooks_only = true`.

Matchers are regex strings. `PermissionRequest`, `PreToolUse`, and `PostToolUse` match tool names and aliases; compaction hooks match `manual` or `auto`; `SessionStart` matches `startup`, `resume`, `clear`, or `compact`; subagent hooks match agent type. `UserPromptSubmit` and `Stop` ignore matchers.

## Payloads and Responses

Every command hook receives one JSON object on `stdin` with common fields: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `model`. Most turn-scoped events add `turn_id`; several events also include `permission_mode`.

Codex response parsing is event-specific:

- Exit `0` with no stdout means success/continue.
- Exit `2` plus stderr is a blocking or feedback path.
- JSON `systemMessage` can surface a warning/status message.
- `hookSpecificOutput.additionalContext` can add model-visible developer context for several events.
- `PreToolUse` can deny through `permissionDecision = "deny"` and can rewrite input through `permissionDecision = "allow"` plus `updatedInput`.
- `PermissionRequest` uses a nested decision object with `behavior = "allow"` or `behavior = "deny"`.
- `Stop` and `SubagentStop` reinterpret `decision = "block"` as continuation, not rejection.

Unsupported-but-parsed fields matter for fail-closed behavior: `async`, `prompt`, and `agent` handlers are skipped; several future response fields are parsed but not supported and may mark a hook run failed while Codex continues the underlying operation.

## Execution Semantics

Command hooks run in the session `cwd`. `timeout` is in seconds and defaults to `600`. `commandWindows` / `command_windows` can provide a Windows-specific command string. Repo-local hooks should avoid fragile relative paths because Codex may be launched from a subdirectory.

For plugin-bundled hooks, Codex provides `PLUGIN_ROOT`, `PLUGIN_DATA`, `CLAUDE_PLUGIN_ROOT`, and `CLAUDE_PLUGIN_DATA`. The public docs do not specify the exact shell used to evaluate command strings, but official examples use command substitution, so Claudine should treat shell semantics as provider-defined and verify source before emitting portable commands.

## Claudine Mapping

The current Claudine Codex implementation is behind the provider. Local inspection shows `claudine/lib/src/config/codex.rs` still treats Codex as notify-only and registers only `turn_complete` through `notify`. That does not match the current first-class hook surface.

Suggested native-to-Claudine mapping:

| Codex event | Claudine event | Notes |
| --- | --- | --- |
| `SessionStart` | `start` | Can inject developer context; source distinguishes startup/resume/clear/compact. |
| `UserPromptSubmit` | `prompt` | Pre-prompt and blocking. |
| `PreToolUse` | `tool_call` | Blocking and input mutation for supported tools. |
| `PermissionRequest` | `permission` | Approval decision hook. |
| `PostToolUse` | `tool_result` | Post-tool feedback/context; cannot undo side effects. |
| `PreCompact` | `none` | No direct Claudine event. |
| `PostCompact` | `none` | No direct Claudine event. |
| `SubagentStart` | `subagent_start` | Adds context but does not block start. |
| `SubagentStop` | `subagent_stop` | Can continue subagent flow. |
| `Stop` | `success` | Turn completion; block means continue with a new prompt. |

Claudine's blocking model must account for Codex's event-specific meanings. In particular, `Stop`/`SubagentStop` "block" is a continuation request, `PostToolUse` "block" is feedback/result replacement after side effects, and `PermissionRequest` has deny-wins fan-in across concurrent matching hooks.

## Gaps

Open adapter questions:

- Windows system config location is undocumented.
- Exact command-shell semantics are not specified in the public docs.
- Tool-hook coverage is intentionally incomplete.
- Transcript contents are not stable.
- Some parsed fields are future-reserved and currently fail or skip.
- Local inspection found no configured user hooks; observed examples come from plugin bundles.
- Claudine needs a migration plan from legacy `notify` wrapper registration to `hooks.json`/inline hooks, including hook trust implications.

## Changelog

- 2026-07-03: Refreshed against Codex CLI 0.142.5, official hooks docs, and generated JSON schemas. Added local inspection findings, corrected permission_mode enum, confirmed Stop turn_id, and added `--strict-config`.
- 2026-07-02: Replaced legacy notify-only research with the current first-class Codex hooks surface from official docs.

## Sources

- [Codex hooks documentation](https://developers.openai.com/codex/hooks)
- [Codex config basics](https://developers.openai.com/codex/config-basic)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex command line options](https://developers.openai.com/codex/cli/reference)
- [Codex managed configuration](https://developers.openai.com/codex/enterprise/managed-configuration)
- [Codex hook JSON schemas](https://github.com/openai/codex/tree/main/codex-rs/hooks/schema/generated)
- [OpenAI Codex repository](https://github.com/openai/codex)
