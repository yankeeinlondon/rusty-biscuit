---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
homepage: https://antigravity.google/product/antigravity-cli
docs: https://antigravity.google/docs/cli/overview
hooks_docs: https://antigravity.google/docs/ide/hooks
hooks:
  - native_event: PreToolUse
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "Common camelCase stdin JSON (conversationId, workspacePaths, transcriptPath, artifactDirectoryPath) plus toolCall.name, toolCall.args, and stepIdx."
    return_contract: "Stdout JSON decision is required: allow auto-allows, deny hard-blocks, ask prompts while respecting cached Always Allow settings, force_ask always prompts. Optional reason is shown to the agent or user; optional permissionOverrides adds resource permission strings."
    notes: "Matcher is a regular expression over toolCall.name. Empty string or * matches all tools. Handler type is command only; timeout defaults to 30 seconds."
  - native_event: PostToolUse
    claudine_event: tool_result
    timing: post
    blocking: false
    payload_schema: "Common camelCase stdin JSON plus stepIdx and optional error string; documentation does not include toolCall or tool result fields for this event."
    return_contract: "Stdout JSON is expected to be an empty object {}. No documented blocking, mutation, or feedback action."
    notes: "Matcher is a regular expression over tool name. Runs after a tool completes, so side effects have already happened."
  - native_event: PreInvocation
    claudine_event: prompt
    timing: pre
    blocking: true
    payload_schema: "Common camelCase stdin JSON plus invocationNum and initialNumSteps."
    return_contract: "Stdout JSON may include injectSteps, an array of steps to inject before the model call. Injected step variants are toolCall, userMessage, and ephemeralMessage."
    notes: "Matcher is ignored. Around-model-call semantics do not expose the pending model request body in the documented payload."
  - native_event: PostInvocation
    claudine_event: loop
    timing: post
    blocking: true
    payload_schema: "Same documented input as PreInvocation: common fields plus invocationNum and initialNumSteps."
    return_contract: "Stdout JSON may include injectSteps and terminationBehavior. terminationBehavior force_continue forces the loop to continue; terminate forces the loop to terminate; empty or omitted uses default behavior."
    notes: "Matcher is ignored. The docs describe it as after tool calls finish, which makes this a loop-control event rather than a final session event."
  - native_event: Stop
    claudine_event: finalize
    timing: post
    blocking: true
    payload_schema: "Common camelCase stdin JSON plus executionNum, terminationReason, optional error, and required fullyIdle boolean."
    return_contract: "Stdout JSON decision is required. decision=continue prevents stopping and re-enters the execution loop; optional reason is injected as a system message. Any other decision value allows the stop."
    notes: "Matcher is ignored. fullyIdle distinguishes final idle termination from stops while background commands or tasks remain active."
config_files:
  - os: macos
    scope: user
    path: "~/.gemini/config/hooks.json"
    format: json
    notes: "Official hook docs say hooks.json lives in the global customization directory ~/.gemini/config/. Not present on this host."
  - os: linux
    scope: user
    path: "~/.gemini/config/hooks.json"
    format: json
    notes: "Official hook docs use the same Unix home-relative path for global customization. Linux path is documented separately here because OS path syntax differs elsewhere."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\hooks.json"
    format: json
    notes: "Windows form inferred from the documented ~/.gemini/config customization root; no official Windows-specific hook path was found."
  - os: macos
    scope: user
    path: "~/.gemini/antigravity-cli/settings.json"
    format: json
    notes: "CLI plugins docs say hooks can be configured in the primary settings.json. The real host file had enableTelemetry, model, and trustedWorkspaces, but no hooks key."
  - os: linux
    scope: user
    path: "~/.gemini/antigravity-cli/settings.json"
    format: json
    notes: "Primary Antigravity CLI settings path documented for CLI preferences; hook schema inside settings.json is not documented."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\settings.json"
    format: json
    notes: "Windows form inferred from the documented ~/.gemini/antigravity-cli/settings.json path; no official Windows-specific hook path was found."
  - os: macos
    scope: repo
    path: ".agents/hooks.json"
    format: json
    notes: "Official hook docs name .agents/ as the workspace customization directory. No repo-local .agents/hooks.json was present in this workspace."
  - os: linux
    scope: repo
    path: ".agents/hooks.json"
    format: json
    notes: "Workspace hook config path. Docs also mention older .agent support for skills, but did not verify .agent/hooks.json compatibility."
  - os: windows
    scope: repo
    path: ".agents\\hooks.json"
    format: json
    notes: "Windows workspace hook config path, rendered separately to avoid mixing path syntaxes."
  - os: macos
    scope: other
    path: "~/.gemini/antigravity-cli/plugins/<plugin_name>/hooks.json"
    format: json
    notes: "CLI plugin docs show installed/imported plugin bundles staged here with optional hooks.json. No plugin hook files were present on this host."
  - os: linux
    scope: other
    path: "~/.gemini/antigravity-cli/plugins/<plugin_name>/hooks.json"
    format: json
    notes: "Plugin-scoped hook container path for CLI-managed plugins."
  - os: windows
    scope: other
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\plugins\\<plugin_name>\\hooks.json"
    format: json
    notes: "Windows plugin-scoped hook container path inferred from the documented Unix staging root."
  - os: macos
    scope: other
    path: "~/.gemini/config/plugins/<plugin_name>/hooks.json"
    format: json
    notes: "IDE plugin docs say manually-added global plugins live under ~/.gemini/config/plugins/ and may contain hooks.json."
  - os: linux
    scope: other
    path: "~/.gemini/config/plugins/<plugin_name>/hooks.json"
    format: json
    notes: "Global plugin customization path from IDE plugin docs."
  - os: windows
    scope: other
    path: "%USERPROFILE%\\.gemini\\config\\plugins\\<plugin_name>\\hooks.json"
    format: json
    notes: "Windows form inferred from documented ~/.gemini/config/plugins/ global plugin root."
  - os: macos
    scope: other
    path: ".agents/plugins/<plugin_name>/hooks.json"
    format: json
    notes: "Workspace plugin hook container from IDE plugin docs."
  - os: linux
    scope: other
    path: ".agents/plugins/<plugin_name>/hooks.json"
    format: json
    notes: "Workspace plugin hook container from IDE plugin docs."
  - os: windows
    scope: other
    path: ".agents\\plugins\\<plugin_name>\\hooks.json"
    format: json
    notes: "Windows workspace plugin hook container path."
cli_params:
  - flag: "/hooks"
    description: "Interactive TUI slash command to browse loaded and active hooks."
    example: "type /hooks inside agy"
  - flag: "agy plugin install <target>"
    description: "Installs a plugin bundle that may contain hooks.json."
    example: "agy plugin install /path/to/local/plugin"
  - flag: "agy plugin import [source]"
    description: "Imports plugins from gemini or claude; imported bundles may include hooks.json."
    example: "agy plugin import gemini"
  - flag: "agy plugin enable <name>"
    description: "Enables a plugin bundle and therefore any hook container it carries."
    example: "agy plugin enable my-plugin"
  - flag: "agy plugin disable <name>"
    description: "Disables a plugin bundle, suspending hooks shipped by that plugin."
    example: "agy plugin disable my-plugin"
  - flag: "agy plugin validate [path]"
    description: "Validates a plugin bundle before installation; hook file validation depth is not documented."
    example: "agy plugin validate /path/to/plugin"
payload_fields:
  - native_event: "*"
    field: conversationId
    type: string
    meaning: "UUID of the active agent conversation; useful for correlation."
  - native_event: "*"
    field: workspacePaths
    type: array<string>
    meaning: "Absolute paths for mounted workspaces; useful for path policy and routing."
  - native_event: "*"
    field: transcriptPath
    type: string
    meaning: "Absolute path to transcript.jsonl conversation logs under the Antigravity app data brain directory."
  - native_event: "*"
    field: artifactDirectoryPath
    type: string
    meaning: "Absolute path to the directory containing conversation artifacts and screenshots."
  - native_event: PreToolUse
    field: toolCall
    type: object
    meaning: "Proposed tool call details."
  - native_event: PreToolUse
    field: toolCall.name
    type: string
    meaning: "Tool name matched by PreToolUse matcher regex."
  - native_event: PreToolUse
    field: toolCall.args
    type: object
    meaning: "Tool arguments; field names depend on the tool, such as CommandLine and Cwd for run_command."
  - native_event: PreToolUse
    field: stepIdx
    type: integer
    meaning: "Zero-based index of the current trajectory step."
  - native_event: PostToolUse
    field: stepIdx
    type: integer
    meaning: "Zero-based index of the completed trajectory step."
  - native_event: PostToolUse
    field: error
    type: string
    meaning: "Detailed runtime error message if the tool failed; empty if successful."
  - native_event: PreInvocation
    field: invocationNum
    type: integer
    meaning: "Zero-based model invocation sequence number."
  - native_event: PreInvocation
    field: initialNumSteps
    type: integer
    meaning: "Number of trajectory steps before the invocation."
  - native_event: PostInvocation
    field: invocationNum
    type: integer
    meaning: "Zero-based model invocation sequence number."
  - native_event: PostInvocation
    field: initialNumSteps
    type: integer
    meaning: "Number of trajectory steps at the invocation boundary."
  - native_event: Stop
    field: executionNum
    type: integer
    meaning: "Execution attempt sequence number."
  - native_event: Stop
    field: terminationReason
    type: string
    meaning: "Reason execution is stopping, such as model_stop, max_steps_exceeded, or error."
  - native_event: Stop
    field: error
    type: string
    meaning: "Optional error message when termination was caused by a system error."
  - native_event: Stop
    field: fullyIdle
    type: boolean
    meaning: "True when the agent is completely finished and all background commands or asynchronous tasks have completed."
response_actions:
  - action: allow
    native_value: "PreToolUse stdout JSON: {\"decision\":\"allow\"}"
    effect: "Automatically allows the pending tool execution."
  - action: deny
    native_value: "PreToolUse stdout JSON: {\"decision\":\"deny\",\"reason\":\"...\"}"
    effect: "Hard-blocks the pending tool execution immediately."
  - action: ask
    native_value: "PreToolUse stdout JSON: {\"decision\":\"ask\",\"reason\":\"...\"}"
    effect: "Prompts the user, while respecting cached Always Allow settings."
  - action: ask
    native_value: "PreToolUse stdout JSON: {\"decision\":\"force_ask\",\"reason\":\"...\"}"
    effect: "Always prompts the user and ignores cached permissions."
  - action: modify
    native_value: "PreToolUse stdout JSON: {\"permissionOverrides\":[\"read_file(/path)\",\"command(args)\"]}"
    effect: "Overrides default tool permissions for the pending tool decision; no tool argument rewrite is documented."
  - action: continue
    native_value: "PreInvocation stdout JSON: {\"injectSteps\":[...]}"
    effect: "Injects toolCall, userMessage, or ephemeralMessage steps before the model call."
  - action: continue
    native_value: "PostInvocation stdout JSON: {\"injectSteps\":[...],\"terminationBehavior\":\"force_continue\"}"
    effect: "Injects steps after the invocation and forces the loop to continue."
  - action: stop
    native_value: "PostInvocation stdout JSON: {\"terminationBehavior\":\"terminate\"}"
    effect: "Forces the execution loop to terminate after the invocation."
  - action: continue
    native_value: "PostInvocation stdout JSON: {\"terminationBehavior\":\"\"} or omitted"
    effect: "Uses Antigravity's default post-invocation loop behavior."
  - action: continue
    native_value: "Stop stdout JSON: {\"decision\":\"continue\",\"reason\":\"...\"}"
    effect: "Prevents stopping, injects the reason as a system message, and re-enters the execution loop."
  - action: allow
    native_value: "Stop stdout JSON: any decision other than \"continue\""
    effect: "Allows the execution loop to stop."
  - action: continue
    native_value: "PostToolUse stdout JSON: {}"
    effect: "Acknowledges post-tool observation; no documented effect on execution."
execution:
  shell: "Configured command handler string; exact shell/interpreter selection is not documented. Handler type defaults to command, and command is the only documented type."
  cwd: "Unknown. Docs do not state whether commands run from the workspace root, process cwd, hook file directory, or another directory."
  env: "Unknown. No hook-specific environment variables or config-root override variables were found in official docs or local config."
  timeout: "Per-handler timeout integer in seconds; defaults to 30 seconds. No event-specific timeout differences documented."
  stdin: "Hook receives camelCase JSON payload on stdin."
  stdout: "Hook returns JSON on stdout. PreToolUse requires decision; PostToolUse returns {}; PreInvocation and PostInvocation may return injectSteps; Stop requires decision."
  stderr: "Unknown. Official docs do not state whether stderr is displayed to the user, fed to the model, logged only, or ignored."
  notes: "Multiple handlers appear in hooks arrays, but docs do not specify sequential versus parallel execution, aggregation order, invalid JSON behavior, non-zero exit behavior, or whether hooks run in print mode. Local probes used agy --help and settings inspection only; no live hook execution was performed."
gaps:
  - "The official CLI hooks page is not separate from IDE hooks; CLI docs point to /hooks and settings/plugin containers but do not repeat the full hook payload contract."
  - "No official Windows-specific hook paths were found; Windows paths are inferred from documented Unix home-relative roots."
  - "The primary settings.json hook schema is undocumented. Host ~/.gemini/antigravity-cli/settings.json had no hooks key; legacy ~/.gemini/settings.json had an empty hooks object."
  - "No ~/.gemini/config/hooks.json, .agents/hooks.json, or plugin hooks.json was present on this host, so installed hook behavior could not be observed."
  - "The actual shell, cwd, environment, stderr handling, non-zero exit-code behavior, invalid JSON behavior, and multiple-handler ordering are undocumented."
  - "The CLI repository does not include Go source for hook execution; it only provided changelog evidence about /hooks writing to ~/.gemini/config/hooks.json."
  - "Whether .agent/hooks.json is still accepted, analogous to documented .agent/skills backward compatibility, is unknown."
changes: []
requires_claudine_update: true
reason: "Antigravity is not currently one of Claudine's compiled provider adapters, and its native hook model differs from the existing Gemini/Claude-style event set by using PreInvocation/PostInvocation plus camelCase payloads and stdout JSON loop-control actions."
---

# Antigravity Hook Event Semantics

## Overview

Antigravity's hook system runs command handlers at fixed points in the agent execution loop. The official hooks documentation describes only one handler kind: shell command handlers declared with `"type": "command"` and a `"command"` string. Hooks receive JSON on stdin and return JSON on stdout. HTTP endpoints, LLM evaluator hooks, and non-command handler kinds are not documented.

The native hook surface can block some actions, inject new trajectory steps, alter permission behavior, or observe completed work. `PreToolUse` gates a pending tool call and can allow, deny, ask, or force a user prompt. `PreInvocation` and `PostInvocation` can inject trajectory steps around model calls, and `PostInvocation` can force the loop to continue or terminate. `Stop` can prevent termination and continue the loop. `PostToolUse` is observational in the documented contract.

Local inspection matters because Antigravity stores data in multiple similarly named trees. On this host, `/Users/ken/.antigravity` exists but contains IDE/editor extension state, not CLI hook configuration. The active CLI/user state is under `/Users/ken/.gemini` and, for this non-interactive run's overridden home, `/Users/ken/.claudine/.gemini`. No `hooks.json` file was found in `/Users/ken/.gemini/config`, `/Users/ken/.gemini/antigravity-cli`, `/Users/ken/.claudine/.gemini/config`, this workspace's `.agents`, or plugin directories. `/Users/ken/.gemini/settings.json` contains `"hooks": {}`, while `/Users/ken/.gemini/antigravity-cli/settings.json` contains `enableTelemetry`, `model`, and `trustedWorkspaces` only.

## Native Hooks

### PreToolUse

`PreToolUse` fires before a tool is executed. It is a `pre` event and is blocking. It can allow the tool, deny it, ask the user, force a fresh prompt, or add permission override strings. It cannot rewrite the tool arguments in the documented contract.

The event supports a `matcher` regular expression over the tool name. `""` and `"*"` match all tools, `"run_command"` matches that exact tool, `"run_command|view_file"` matches either, and expressions such as `"browser_.*"` match a family.

### PostToolUse

`PostToolUse` fires after a tool completes. It is a `post` event and is not documented as blocking. The handler returns `{}` and cannot undo tool side effects. The event supports the same regular expression matcher over tool names.

The documented payload includes `stepIdx` and `error`, but not the full tool call or result body. That limits adapter-level routing unless Claudine also reads the transcript referenced by `transcriptPath`.

### PreInvocation

`PreInvocation` fires before Antigravity calls the model. It is a `pre` event and can mutate the trajectory by returning `injectSteps`. The matcher is ignored. The documented input identifies the invocation number and step count, but does not expose the model request body.

### PostInvocation

`PostInvocation` fires after tool calls finish. It is a `post` event with loop-control behavior. It can inject steps and can set `terminationBehavior` to `force_continue` or `terminate`. The matcher is ignored.

This event is not equivalent to session finalization. It sits inside Antigravity's execution loop and can decide whether another loop iteration happens.

### Stop

`Stop` fires when the execution loop terminates. It is a `post` event and can block termination by returning `{"decision":"continue"}`. The matcher is ignored. The `fullyIdle` field is required and distinguishes a complete idle stop from a stop while background commands or asynchronous tasks remain active.

## Configuration

Global hooks are configured in a `hooks.json` file in the customization directory. The official docs name `.agents/` for workspace customizations and `~/.gemini/config/` for global customizations. CLI plugin docs also state that hooks can be configured in a plugin's `hooks.json` or in the primary `settings.json`, but they do not document the `settings.json` hook object shape.

macOS paths:

| Scope | Path | Notes |
| :--- | :--- | :--- |
| User | `~/.gemini/config/hooks.json` | Official global customization hook path. |
| User | `~/.gemini/antigravity-cli/settings.json` | Primary CLI settings file; hook schema inside it is undocumented. |
| Repo | `.agents/hooks.json` | Official workspace customization hook path. |
| Plugin | `~/.gemini/antigravity-cli/plugins/<plugin_name>/hooks.json` | CLI-installed plugin hook container. |
| Plugin | `~/.gemini/config/plugins/<plugin_name>/hooks.json` | Global manually-added plugin hook container from IDE plugin docs. |
| Plugin | `.agents/plugins/<plugin_name>/hooks.json` | Workspace plugin hook container. |

Linux paths:

| Scope | Path | Notes |
| :--- | :--- | :--- |
| User | `~/.gemini/config/hooks.json` | Official global customization hook path. |
| User | `~/.gemini/antigravity-cli/settings.json` | Primary CLI settings file; hook schema inside it is undocumented. |
| Repo | `.agents/hooks.json` | Official workspace customization hook path. |
| Plugin | `~/.gemini/antigravity-cli/plugins/<plugin_name>/hooks.json` | CLI-installed plugin hook container. |
| Plugin | `~/.gemini/config/plugins/<plugin_name>/hooks.json` | Global manually-added plugin hook container from IDE plugin docs. |
| Plugin | `.agents/plugins/<plugin_name>/hooks.json` | Workspace plugin hook container. |

Windows paths:

| Scope | Path | Notes |
| :--- | :--- | :--- |
| User | `%USERPROFILE%\.gemini\config\hooks.json` | Inferred Windows form of the official global customization hook path. |
| User | `%USERPROFILE%\.gemini\antigravity-cli\settings.json` | Inferred Windows form of the primary CLI settings path. |
| Repo | `.agents\hooks.json` | Workspace customization hook path with Windows separators. |
| Plugin | `%USERPROFILE%\.gemini\antigravity-cli\plugins\<plugin_name>\hooks.json` | Inferred Windows form of CLI-installed plugin hook container. |
| Plugin | `%USERPROFILE%\.gemini\config\plugins\<plugin_name>\hooks.json` | Inferred Windows form of global manually-added plugin container. |
| Plugin | `.agents\plugins\<plugin_name>\hooks.json` | Workspace plugin hook container with Windows separators. |

Hook definitions use a top-level map from hook name to event configuration. A hook definition can set `enabled: false`. `PreToolUse` and `PostToolUse` contain arrays of matcher blocks, each with a `matcher` and a `hooks` array. `PreInvocation`, `PostInvocation`, and `Stop` use a direct array of handlers under the event key, and matchers are ignored.

Hook-affecting CLI surfaces are:

| Command | Effect |
| :--- | :--- |
| `/hooks` | TUI slash command to browse loaded and active hooks. |
| `agy plugin install <target>` | Installs a plugin bundle that may carry `hooks.json`. |
| `agy plugin import [source]` | Imports plugins from `gemini` or `claude`; imported bundles may carry hooks. |
| `agy plugin enable <name>` | Enables a plugin and its hook container. |
| `agy plugin disable <name>` | Disables a plugin and its hook container. |
| `agy plugin validate [path]` | Validates a plugin bundle; hook validation depth is not documented. |

No environment variable that disables hooks, redirects hook config roots, or changes hook execution was found in the official docs, `agy --help`, or local settings.

## Payloads and Responses

All documented hook payloads use camelCase JSON on stdin and include common metadata:

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `conversationId` | string | UUID of the active conversation. |
| `workspacePaths` | array of strings | Absolute mounted workspace paths. |
| `transcriptPath` | string | Absolute path to the persistent `transcript.jsonl` log. |
| `artifactDirectoryPath` | string | Absolute path to the conversation artifact/screenshot directory. |

`PreToolUse` fields:

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `toolCall` | object | Proposed tool call. |
| `toolCall.name` | string | Tool name used by matchers. |
| `toolCall.args` | object | Tool arguments, whose shape depends on `toolCall.name`. |
| `stepIdx` | integer | Zero-based trajectory step index. |

`PreToolUse` stdout JSON:

| Field | Type | Effect |
| :--- | :--- | :--- |
| `decision` | string | Required. `allow`, `deny`, `ask`, or `force_ask`. |
| `reason` | string | Optional explanation shown to the agent or user. |
| `permissionOverrides` | array of strings | Optional resource strings such as `read_file(/path)` or `command(args)`. |

`PostToolUse` fields:

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `stepIdx` | integer | Zero-based completed step index. |
| `error` | string | Runtime error message if the tool call failed; empty if successful. |

`PostToolUse` returns `{}` on stdout.

`PreInvocation` and `PostInvocation` fields:

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `invocationNum` | integer | Zero-based model invocation sequence number. |
| `initialNumSteps` | integer | Number of steps in the trajectory at the invocation boundary. |

`PreInvocation` stdout JSON may include `injectSteps`. Each injected step can contain one of:

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `toolCall` | object | A tool call to execute. |
| `userMessage` | string | A user message to inject. |
| `ephemeralMessage` | string | A transient system message. |

`PostInvocation` stdout JSON uses the same optional `injectSteps` field and also supports `terminationBehavior`:

| Native value | Effect |
| :--- | :--- |
| `"force_continue"` | Forces the execution loop to continue. |
| `"terminate"` | Forces the execution loop to terminate. |
| `""` or omitted | Uses the default behavior. |

`Stop` fields:

| Field | Type | Meaning |
| :--- | :--- | :--- |
| `executionNum` | integer | Execution attempt sequence number. |
| `terminationReason` | string | Reason for stopping, such as `model_stop`, `max_steps_exceeded`, or `error`. |
| `error` | string | Optional system error message. |
| `fullyIdle` | boolean | Required. True when the agent and all background work are finished. |

`Stop` stdout JSON:

| Field | Type | Effect |
| :--- | :--- | :--- |
| `decision` | string | Required. `"continue"` prevents stop; any other value allows stop. |
| `reason` | string | Optional. When continuing, injected as a system message. |

The official docs do not specify exit-code semantics. They say stdout JSON is the response channel, and they do not document whether exit 0 is required, whether non-zero exits block, or where stderr is routed.

## Execution Semantics

Handlers are configured as command strings:

```json
{
  "type": "command",
  "command": "./scripts/lint.sh",
  "timeout": 10
}
```

`type` is optional, currently only `"command"` is documented, and the default type is `"command"`. `timeout` is an integer number of seconds and defaults to `30`. Hooks receive JSON on stdin and return JSON on stdout.

The documentation does not state the shell used to run command strings, the working directory, environment variables, stderr behavior, invalid JSON behavior, non-zero exit behavior, or whether multiple matching handlers run sequentially or in parallel. It also does not state whether hooks run in `agy --print` mode. These must be treated as unknown until observed with a live hook probe or source access.

Platform caveat: all documented examples use Unix-style paths. Windows path forms in this document are inferred from the documented home-relative paths and should be verified on a Windows host.

## Claudine Mapping

| Native event | Timing | Claudine event | Notes |
| :--- | :--- | :--- | :--- |
| `PreToolUse` | `pre` | `tool_call` | Preserve `toolCall.name`, `toolCall.args`, `stepIdx`, and Antigravity's decision vocabulary. |
| `PostToolUse` | `post` | `tool_result` | Payload lacks documented result details; preserve `stepIdx`, `error`, and `transcriptPath`. |
| `PreInvocation` | `pre` | `prompt` | Closest pre-model boundary. Not identical to user prompt submission because it can fire for later loop invocations. |
| `PostInvocation` | `post` | `loop` | Provider-specific loop-control event with `terminationBehavior`; preserve this field. |
| `Stop` | `post` | `finalize` | Can continue the loop, so Claudine should preserve the native `decision=continue` behavior if adapting. |

Many-to-one collisions are limited. `PreInvocation` and `PostInvocation` both sit around model-loop execution rather than around a user prompt; Claudine must not collapse them into a single prompt event without preserving `invocationNum`, `initialNumSteps`, and whether injection happens before or after the model call. `Stop` resembles `finalize`, but because it can restart execution it also overlaps with Claudine recovery/loop-control semantics.

Provider-specific payload fields worth preserving on the unified payload include `conversationId`, `workspacePaths`, `transcriptPath`, `artifactDirectoryPath`, `stepIdx`, `toolCall.name`, `toolCall.args`, `invocationNum`, `initialNumSteps`, `executionNum`, `terminationReason`, `fullyIdle`, `injectSteps`, `terminationBehavior`, `permissionOverrides`, and the native `decision` string.

## Gaps

- The official CLI hooks page is not separate from the IDE hook contract. CLI docs confirm hooks exist and are inspectable via `/hooks`, but the full payload/response contract is published under `ide/hooks`.
- No source code for the Go CLI hook runner was available in the public `google-antigravity/antigravity-cli` repository.
- No local hook files or plugin hook containers were installed on this host, so actual handler ordering, process environment, cwd, stderr handling, exit-code handling, and invalid JSON behavior were not observed.
- The primary `settings.json` hook schema is not documented, despite CLI docs saying hooks can be configured there.
- Windows hook paths and plugin paths are inferred and need a Windows host probe.
- Backward compatibility for `.agent/hooks.json` is unknown.

## Sources

- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Official hooks documentation](https://antigravity.google/docs/ide/hooks)
- [Raw hooks Markdown asset](https://antigravity.google/assets/docs/editor/ide-hooks.md)
- [CLI plugins documentation](https://antigravity.google/docs/cli/plugins)
- [Raw CLI plugins Markdown asset](https://antigravity.google/assets/docs/cli/cli-plugins.md)
- [CLI settings documentation](https://antigravity.google/docs/cli/settings)
- [Raw CLI settings Markdown asset](https://antigravity.google/assets/docs/cli/cli-settings.md)
- [CLI reference documentation](https://antigravity.google/docs/cli/reference)
- [Raw CLI reference Markdown asset](https://antigravity.google/assets/docs/cli/cli-reference.md)
- [Antigravity CLI repository changelog](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- [Antigravity SDK hook architecture README](https://github.com/google-antigravity/antigravity-sdk-python/blob/main/google/antigravity/hooks/README.md)
- Observed on host: `/Users/ken/.antigravity` contains IDE/editor extension state and no hook configuration files.
- Observed on host: `/Users/ken/.gemini/settings.json` contains `"hooks": {}`.
- Observed on host: `/Users/ken/.gemini/antigravity-cli/settings.json` contains no hooks key.
- Observed on host: no `hooks.json` was found under `/Users/ken/.gemini/config`, `/Users/ken/.gemini/antigravity-cli`, `/Users/ken/.claudine/.gemini/config`, this workspace's `.agents`, or plugin directories.
