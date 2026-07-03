---
$schema: ./_schema.yaml
created: "2026-07-03"
last_updated: "2026-07-03"
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://goose-docs.ai/
docs: https://goose-docs.ai/docs/category/guides
hooks_docs: https://goose-docs.ai/docs/guides/context-engineering/hooks

hooks:
  - native_event: SessionStart
    claudine_event: initialize
    timing: pre
    blocking: false
    payload_schema: "event, session_id"
    return_contract: "Exit 0 = no effect. Non-zero exits are logged but do not stop the session."
    notes: "Fires only on the first user turn of a session (is_first_turn guard). No matcher support."
  - native_event: SessionEnd
    claudine_event: finalize
    timing: post
    blocking: false
    payload_schema: "event, session_id"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires when a CLI session ends. Not guaranteed for every server-side session path."
  - native_event: UserPromptSubmit
    claudine_event: prompt
    timing: pre
    blocking: false
    payload_schema: "event, session_id, matcher_context, message"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires before slash-command resolution and before the reply loop. matcher_context mirrors message."
  - native_event: PreToolUse
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "event, session_id, matcher_context, tool_name, tool_input, working_dir"
    return_contract: "Exit 0 = allow tool call. Exit 2 with reason on stderr, or stdout JSON {\"decision\":\"block\",\"reason\":\"...\"} = permanent policy denial."
    notes: "Matcher tests against tool_name. Denial returns an internal error to the model with 'Do not retry; this is a policy denial'."
  - native_event: PostToolUse
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "event, session_id, matcher_context, tool_name, tool_input, working_dir"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires after a successful tool call. The tool_output field is declared but not populated by observed code."
  - native_event: PostToolUseFailure
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "event, session_id, matcher_context, tool_name, tool_input, working_dir"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires after a failed tool call (is_error = true or the tool returned an error)."
  - native_event: BeforeReadFile
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "event, session_id, matcher_context, tool_name, tool_input, working_dir"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires after PreToolUse for read-category tools (local tool names: read/view/cat/read_file). matcher_context is the file path from tool_input.path|file|file_path."
  - native_event: BeforeShellExecution
    claudine_event: tool_call
    timing: pre
    blocking: false
    payload_schema: "event, session_id, matcher_context, tool_name, tool_input, working_dir"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires after PreToolUse for shell-category tools (local tool names: shell/bash/exec/run). matcher_context is the command string from tool_input.command."
  - native_event: AfterFileEdit
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "event, session_id, matcher_context, tool_name, tool_input, working_dir"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires after successful write-category tools (local tool names: write/edit/patch/write_file/edit_file). matcher_context is the file path."
  - native_event: AfterShellExecution
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "event, session_id, matcher_context, tool_name, tool_input, working_dir"
    return_contract: "No decision semantics; output is ignored."
    notes: "Fires after successful shell-category tools. matcher_context is the command string."
  - native_event: Stop
    claudine_event: finalize
    timing: pre
    blocking: true
    payload_schema: "event, session_id, last_assistant_message"
    return_contract: "Exit 0 = allow turn to end. Exit 2 with reason on stderr, or stdout JSON {\"decision\":\"block\",\"reason\":\"...\"} = keep working."
    notes: "Default cap of 8 consecutive blocks (GOOSE_STOP_HOOK_BLOCK_CAP). When the session exits without a final_output tool, the same Stop hook may fire non-blocking as a post event."

config_files:
  - os: macos
    scope: user
    path: "~/.agents/plugins/<plugin-name>/hooks/hooks.json"
    format: json
    notes: "User-scoped Open Plugins hook container. Auto-discovered at startup."
  - os: linux
    scope: user
    path: "~/.agents/plugins/<plugin-name>/hooks/hooks.json"
    format: json
    notes: "User-scoped Open Plugins hook container. Auto-discovered at startup."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\plugins\\<plugin-name>\\hooks\\hooks.json"
    format: json
    notes: "User-scoped Open Plugins hook container. Auto-discovered at startup."
  - os: macos
    scope: repo
    path: "<project>/.agents/plugins/<plugin-name>/hooks/hooks.json"
    format: json
    notes: "Project-scoped hook container; loaded when goose starts from that project."
  - os: linux
    scope: repo
    path: "<project>/.agents/plugins/<plugin-name>/hooks/hooks.json"
    format: json
    notes: "Project-scoped hook container; loaded when goose starts from that project."
  - os: windows
    scope: repo
    path: "<project>\\.agents\\plugins\\<plugin-name>\\hooks\\hooks.json"
    format: json
    notes: "Project-scoped hook container; loaded when goose starts from that project."
  - os: macos
    scope: managed
    path: "<plugin-install-dir>/<plugin-name>/hooks/hooks.json"
    format: json
    notes: "Plugins installed via 'goose plugin install' land in the user plugin install directory (default ~/.agents/plugins on all OSes)."
  - os: linux
    scope: managed
    path: "<plugin-install-dir>/<plugin-name>/hooks/hooks.json"
    format: json
    notes: "Plugins installed via 'goose plugin install' land in the user plugin install directory (default ~/.agents/plugins on all OSes)."
  - os: windows
    scope: managed
    path: "<plugin-install-dir>\\<plugin-name>\\hooks\\hooks.json"
    format: json
    notes: "Plugins installed via 'goose plugin install' land in the user plugin install directory (default ~/.agents/plugins on all OSes)."
  - os: macos
    scope: user
    path: "~/.config/goose/settings.json"
    format: json
    notes: "Plugin allow/block list (disabledPlugins / enabledPlugins) that gates hook discovery."
  - os: linux
    scope: user
    path: "~/.config/goose/settings.json"
    format: json
    notes: "Plugin allow/block list (disabledPlugins / enabledPlugins) that gates hook discovery."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\goose\\settings.json"
    format: json
    notes: "Plugin allow/block list (disabledPlugins / enabledPlugins) that gates hook discovery."
  - os: macos
    scope: repo
    path: "<project>/.config/goose/settings.json"
    format: json
    notes: "Project-level disabledPlugins / enabledPlugins. Precedence: local > project > user."
  - os: linux
    scope: repo
    path: "<project>/.config/goose/settings.json"
    format: json
    notes: "Project-level disabledPlugins / enabledPlugins. Precedence: local > project > user."
  - os: windows
    scope: repo
    path: "<project>\\.config\\goose\\settings.json"
    format: json
    notes: "Project-level disabledPlugins / enabledPlugins. Precedence: local > project > user."
  - os: macos
    scope: repo
    path: "<project>/.config/goose/settings.local.json"
    format: json
    notes: "Local project plugin settings; highest precedence among project settings."
  - os: linux
    scope: repo
    path: "<project>/.config/goose/settings.local.json"
    format: json
    notes: "Local project plugin settings; highest precedence among project settings."
  - os: windows
    scope: repo
    path: "<project>\\.config\\goose\\settings.local.json"
    format: json
    notes: "Local project plugin settings; highest precedence among project settings."
  - os: macos
    scope: user
    path: "~/Library/Application Support/Block/goose/config/config.yaml"
    format: yaml
    notes: "Main goose config. The 'plugins' map persists per-plugin enabled: true/false state; disabling a plugin here also disables its hooks."
  - os: linux
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    notes: "Main goose config. The 'plugins' map persists per-plugin enabled: true/false state; disabling a plugin here also disables its hooks."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    format: yaml
    notes: "Main goose config. The 'plugins' map persists per-plugin enabled: true/false state; disabling a plugin here also disables its hooks."

cli_params:
  - flag: "goose plugin install [--auto-update] <URL>"
    description: "Install a git-backed plugin that may contain hooks/hooks.json."
    example: "goose plugin install https://github.com/example/my-goose-plugin.git"
  - flag: "goose plugin update <NAME>"
    description: "Update an installed git-backed plugin by name."
    example: "goose plugin update my-plugin"
  - flag: "disabledPlugins (settings key)"
    description: "List of plugin names to skip during discovery; disables all hooks from those plugins."
    example: '"disabledPlugins": ["session-logger"]'
  - flag: "enabledPlugins (settings key)"
    description: "When present, only listed plugins are loaded (explicit allowlist)."
    example: '"enabledPlugins": ["session-logger"]'

payload_fields:
  - native_event: "(common)"
    field: "event"
    type: string
    meaning: "Native event name, e.g. PreToolUse or Stop."
  - native_event: "(common)"
    field: "session_id"
    type: string
    meaning: "Current goose session identifier."
  - native_event: "(tool events)"
    field: "matcher_context"
    type: string
    meaning: "String the rule's regex is tested against: tool_name for Pre/Post/Failure; file path for BeforeReadFile/AfterFileEdit; command string for Before/AfterShellExecution; prompt text for UserPromptSubmit."
  - native_event: "(tool events)"
    field: "tool_name"
    type: string
    meaning: "Full tool name (e.g. developer__shell)."
  - native_event: "(tool events)"
    field: "tool_input"
    type: object
    meaning: "Arguments passed to the tool as a JSON object."
  - native_event: "(tool events)"
    field: "working_dir"
    type: string
    meaning: "Session working directory at the time of the tool call."
  - native_event: UserPromptSubmit
    field: "message"
    type: string
    meaning: "The user-submitted prompt text."
  - native_event: Stop
    field: "last_assistant_message"
    type: string
    meaning: "Final assistant text for the turn, present when the assistant produced output before stopping."

response_actions:
  - action: allow
    native_value: "Exit 0"
    effect: "Proceed with the pending action (tool call or turn end)."
  - action: block
    native_value: 'Exit 2 with reason on stderr, OR stdout JSON {"decision":"block","reason":"..."}'
    effect: "PreToolUse: deny the tool call permanently (model is told not to retry). Stop: prevent the turn from ending; a system notification and hidden user message are injected, and the agent continues. After 8 consecutive Stop blocks the cap overrides."
  - action: other
    native_value: "Any non-zero exit except 2, or timeout, or spawn failure"
    effect: "Treated as Allow for blocking hooks; logged as a warning but never crashes the host tool or pending action."

execution:
  shell: "Hooks run as `sh -c <command>` on all platforms (flatpak builds use flatpak-spawn with `sh -c`). On Windows, `sh` must be available via Git Bash/MSYS2."
  cwd: "No explicit working directory is set; the child inherits goose's current working directory, which is the session working directory."
  env: "The hook process inherits goose's environment. PLUGIN_ROOT is always set to the plugin's root directory. On goose Desktop, PATH may be augmented with the user's login-shell PATH. GOOSE_TERMINAL=1 and AGENT=goose are set for shell commands invoked by goose, but not for hook commands themselves."
  timeout: "Default 30 seconds per hook; overridden by the 'timeout' field in hooks.json."
  stdin: "A single JSON document (HookContext) is written to stdin."
  stdout: "For blocking hooks, if trimmed stdout starts with '{' and parses to {\"decision\":\"block\",\"reason\"}, it blocks. For non-blocking hooks stdout is ignored."
  stderr: "On exit 2 for blocking hooks, stderr becomes the block reason. Other stderr is logged but not shown to the user."
  notes: "Rules within an event are evaluated in load order; all matching rules run. Actions within a rule run in array order. Multiple plugins can match the same event; there is no deduplication. Hook failures are logged and never propagate."

gaps:
  - "The Open Plugins spec defines prompt and agent hook action types, but Goose source only implements command and silently ignores the others as of the observed code."
  - "tool_output is a declared field in HookContext but is not populated by the observed post-tool hook code."
  - "There is no CLI command to list, test, or validate hooks; the only hook-related CLI surface is plugin installation and the disabledPlugins/enabledPlugins settings keys."
  - "There is no environment variable to disable all hooks globally; hooks are gated only by plugin enable/disable state (settings.json / config.yaml plugins map)."
  - "SubagentStart and SubagentStop are listed in the Open Plugins spec and the Goose docs note they are 'not currently emitted'; no subagent lifecycle hooks fire."
  - "Stop hooks can fire via emit_blocking (final_output / exit_chat paths) or emit non-blocking (other exit paths). The same hooks.json Stop rules participate in both, which means a Stop hook may run non-blocking if the session ends without a final_output tool."
  - "On Windows, hook command execution relies on a POSIX sh in PATH; the documented Windows installers include Git Bash/MSYS2 but this is a portability caveat."
  - "The user plugin settings.json path is hardcoded to .config/goose/settings.json under the home directory on all OSes in discovery.rs, whereas config.yaml uses etcetera platform paths. This is an observed inconsistency."

changes: []
requires_claudine_update: true
reason: "Goose's hook surface uses a different event set and blocking contract than Claude Code. Claudine needs a Goose provider adapter that maps the 11 native events (plus the spec-only SubagentStart/Stop) into the unified lifecycle, preserves the event field as a provider discriminator for tool_call/tool_result collisions, handles the two blocking events (PreToolUse and Stop) with their native deny formats (exit 2 or stdout JSON decision:block), and respects the non-blocking fire-and-forget semantics for all other events. It also needs to know that only command actions are functional and that plugin discovery paths differ from Claude Code's settings.json model."
---

# Goose CLI Hooks

## Overview

Goose CLI implements lifecycle hooks through the [Open Plugins hooks specification](https://open-plugins.com/agent-builders/components/hooks). Hooks are *command* actions declared inside a plugin's `hooks/hooks.json` file. They are discovered automatically from user, project, and installed plugin directories at startup, and they receive a JSON event payload on stdin.

As of the observed source, Goose only runs `type: "command"` hooks. The Open Plugins spec also defines `prompt` and `agent` action types, but Goose deserializes and ignores them. Hooks can **block** two actions (`PreToolUse` and `Stop`) and are **fire-and-forget observers** for everything else. They cannot mutate tool input, replace tool output, or auto-approve pending actions.

## Native Hooks

| Native event | Timing | Blocking | Matcher target | Notes |
|--------------|--------|----------|----------------|-------|
| `SessionStart` | pre | no | none | Fires on the first user turn only. |
| `SessionEnd` | post | no | none | Fires when a CLI session ends. |
| `UserPromptSubmit` | pre | no | prompt text | Fires before command resolution and the reply loop. |
| `PreToolUse` | pre | yes | tool name | Can permanently deny a tool call. |
| `PostToolUse` | post | no | tool name | Fires after successful tool calls. |
| `PostToolUseFailure` | post | no | tool name | Fires after failed tool calls. |
| `BeforeReadFile` | pre | no | file path | Fires after `PreToolUse` for read-category tools. |
| `BeforeShellExecution` | pre | no | command string | Fires after `PreToolUse` for shell-category tools. |
| `AfterFileEdit` | post | no | file path | Fires after successful write-category tools. |
| `AfterShellExecution` | post | no | command string | Fires after successful shell-category tools. |
| `Stop` | pre | yes | none | Can block the turn from ending; overridden after 8 consecutive blocks. |

The matcher is a regular expression. If omitted or empty, the rule matches every event of that type. Regex syntax follows the Rust `regex` crate; invalid regexes cause the rule to be skipped at load time.

Only `PreToolUse` and `Stop` use `emit_blocking`. All other events use `emit`, which logs failures but never propagates them. A misbehaving hook cannot crash the host tool or the action that triggered it.

## Configuration

Hooks live inside a plugin directory under `<plugin-root>/hooks/hooks.json`. Plugins are discovered from:

| Scope | macOS | Linux | Windows |
|-------|-------|-------|---------|
| User | `~/.agents/plugins/<plugin-name>/` | `~/.agents/plugins/<plugin-name>/` | `%USERPROFILE%\.agents\plugins\<plugin-name>\` |
| Project | `<project>/.agents/plugins/<plugin-name>/` | `<project>/.agents/plugins/<plugin-name>/` | `<project>\.agents\plugins\<plugin-name>\` |
| Installed | `~/.agents/plugins/<plugin-name>/` (default) | `~/.agents/plugins/<plugin-name>/` (default) | `%USERPROFILE%\.agents\plugins\<plugin-name>\` (default) |

The `disabledPlugins` and `enabledPlugins` settings keys control which discovered plugins are active:

| Scope | macOS | Linux | Windows |
|-------|-------|-------|---------|
| User | `~/.config/goose/settings.json` | `~/.config/goose/settings.json` | `%USERPROFILE%\.config\goose\settings.json` |
| Project | `<project>/.config/goose/settings.json` | `<project>/.config/goose/settings.json` | `<project>\.config\goose\settings.json` |
| Local project | `<project>/.config/goose/settings.local.json` | `<project>/.config/goose/settings.local.json` | `<project>\.config\goose\settings.local.json` |

Precedence is local project > project > user. In addition, the main config file persists a `plugins` map with per-plugin `enabled: true/false` entries:

| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/Block/goose/config/config.yaml` |
| Linux | `~/.config/goose/config.yaml` |
| Windows | `%APPDATA%\Block\goose\config\config.yaml` |

There are no CLI commands dedicated to listing, testing, or disabling hooks. The closest CLI surface is `goose plugin install <URL>` and `goose plugin update <NAME>`, which install or update git-backed plugins that may contain hooks.

## Payloads and Responses

Every hook receives a JSON object on stdin. The common fields are:

- `event` — the native event name.
- `session_id` — the current session identifier.

Tool events add:

- `matcher_context` — the string the regex matcher tests.
- `tool_name` — the full tool name, e.g. `developer__shell`.
- `tool_input` — the tool arguments as a JSON object.
- `working_dir` — the session working directory.

`UserPromptSubmit` adds `message` (the prompt text). `Stop` adds `last_assistant_message` when the assistant produced output. The `tool_output` field is declared in the context struct but is not populated by the observed post-tool hook code.

For blocking events, the response contract is:

| Exit / stdout | Meaning |
|---------------|---------|
| Exit 0 | Allow the pending action. |
| Exit 2, reason on stderr | Block; stderr becomes the reason. |
| Exit 2 with empty stderr | Block with default reason "denied by plugin hook". |
| Stdout JSON `{"decision":"block","reason":"..."}` | Block with the supplied reason. |
| Any other non-zero exit, timeout, or spawn failure | Treated as Allow and logged as a warning. |

For `PreToolUse`, a block returns an internal error to the model instructing it not to retry. For `Stop`, a block injects a system notification and a hidden user message telling the model to address the issue before stopping again; after 8 consecutive blocks the turn ends anyway unless `GOOSE_STOP_HOOK_BLOCK_CAP` is raised.

## Execution Semantics

- **Shell**: Goose spawns `sh -c <command>` on all platforms. Flatpak builds use `flatpak-spawn` with `sh -c`. On Windows, `sh` must be on PATH, which the documented installers provide via Git Bash or MSYS2.
- **Working directory**: No explicit `current_dir` is set; the hook inherits Goose's current working directory (the session working directory).
- **Environment**: The hook inherits Goose's environment. `PLUGIN_ROOT` is always set to the plugin's root directory. On Goose Desktop, PATH may be augmented with the user's login-shell PATH.
- **Timeout**: Default 30 seconds per hook; overridden by the `timeout` field in `hooks.json`.
- **Stdin**: One JSON `HookContext` document.
- **Stdout**: Parsed only for blocking hooks; if it starts with `{` and contains `{"decision":"block",...}`, the action is blocked. Otherwise stdout is ignored.
- **Stderr**: Becomes the block reason on exit 2 for blocking hooks; otherwise logged but not shown to the user.
- **Ordering**: Rules are evaluated in load order and all matching rules run. Actions inside a rule run in array order. There is no deduplication across plugins.

## Claudine Mapping

| Native event | Claudine event | Provider-specific payload to preserve |
|--------------|----------------|---------------------------------------|
| `SessionStart` | `initialize` | none |
| `SessionEnd` | `finalize` | none |
| `UserPromptSubmit` | `prompt` | `message` |
| `PreToolUse` | `tool_call` | `tool_name`, `tool_input`, `working_dir` |
| `BeforeReadFile` | `tool_call` | `event_kind="BeforeReadFile"`, `matcher_context` |
| `BeforeShellExecution` | `tool_call` | `event_kind="BeforeShellExecution"`, `matcher_context` |
| `PostToolUse` | `tool_result` | `tool_name`, `tool_input`, `working_dir` |
| `PostToolUseFailure` | `tool_result` | `tool_name`, `tool_input`, `working_dir` |
| `AfterFileEdit` | `tool_result` | `event_kind="AfterFileEdit"`, `matcher_context` |
| `AfterShellExecution` | `tool_result` | `event_kind="AfterShellExecution"`, `matcher_context` |
| `Stop` | `finalize` | `last_assistant_message` |

Because several native events map to the same Claudine event, Claudine must preserve the native `event` name (and `matcher_context` where present) as a provider discriminator. `PreToolUse` and `Stop` are the only events that can block; all others are observer-only.

## Gaps

- The Open Plugins spec defines `prompt` and `agent` hook action types, but Goose source only implements `command` and silently ignores the others.
- `tool_output` is declared in the payload struct but is not populated by the observed post-tool hook code.
- There is no CLI command to list, test, or validate hooks.
- There is no environment variable to disable all hooks globally.
- `SubagentStart` and `SubagentStop` are listed in the Open Plugins spec but Goose does not emit them.
- `Stop` hooks can fire via the blocking path (when a turn ends through the `final_output` tool or `exit_chat`) or the non-blocking path (other exits), so the same `Stop` rule may behave as either pre or post depending on how the turn ends.
- On Windows, hook execution depends on a POSIX `sh` being available in PATH.
- The plugin settings.json path is hardcoded to `.config/goose/settings.json` under the home directory on all OSes, while `config.yaml` uses platform-specific etcetera paths.

## Sources

- Goose hooks documentation: <https://goose-docs.ai/docs/guides/context-engineering/hooks>
- Open Plugins hooks spec: <https://open-plugins.com/agent-builders/components/hooks>
- Goose hooks blog post: <https://goose-docs.ai/blog/2026/05/14/goose-hooks>
- Goose CLI commands reference: <https://goose-docs.ai/docs/guides/goose-cli-commands>
- Goose environment variables: <https://goose-docs.ai/docs/guides/environment-variables>
- Goose configuration files: <https://goose-docs.ai/docs/guides/config-files>
- Goose source, hooks implementation: <https://github.com/aaif-goose/goose/blob/main/crates/goose/src/hooks/mod.rs>
- Goose source, hook event definitions and payload: <https://github.com/aaif-goose/goose/blob/main/crates/goose/src/hooks/mod.rs#L50-L172>
- Goose source, blocking hook execution: <https://github.com/aaif-goose/goose/blob/main/crates/goose/src/hooks/mod.rs#L364-L452>
- Goose source, PreToolUse blocking call site: <https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/agent.rs#L1050-L1080>
- Goose source, Stop hook blocking call site and cap: <https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/agent.rs#L1939-L1962>
- Goose source, plugin discovery and settings paths: <https://github.com/aaif-goose/goose/blob/main/crates/goose/src/plugins/discovery.rs>
- Goose source, paths resolver: <https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs>
- Example hello-hooks plugin: <https://github.com/aaif-goose/goose/blob/main/examples/plugins/hello-hooks/hooks/hooks.json>
