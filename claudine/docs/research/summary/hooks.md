---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  Lifecycle hooks are how an external system observes and steers an agentic session — they are the foundation of Claudine's normalized 16-event lifecycle model. Providers differ in which events they expose, what payloads those events carry, whether a hook can block or mutate the action, and how hooks are configured.

  ## Task

  Your task is to report on lifecycle hook support across the Agentic CLI providers Claudine supports.

  - your report should start by outlining why hooks matter to a wrapper like Claudine (event normalization, blocking/mutating actions, observability)
  - and then shift its focus to how providers differ: which events exist, payload shapes, blocking versus observe-only capability, return-value semantics, and hook configuration format
  - close with a point of view on how each provider's events map onto Claudine's canonical event model and where the notable gaps are

  As background material we have hooks research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/hooks/*.md`.

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/hooks.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining provider's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining provider has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/hooks.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
hash: 41b6946c05e281e3-788f1857e7eef7dc
last_updated: 2026-07-03
---
# Lifecycle Hook Support Across Claudine Providers

Lifecycle hooks matter to Claudine because they are where a provider-specific agent session becomes observable and steerable by an external wrapper. Without hooks, Claudine can still watch process output, parse streams, and infer progress, but it cannot reliably see the provider's own lifecycle boundaries or intervene before important actions happen.

Claudine's normalized lifecycle model exists to make those provider differences usable. The canonical lifecycle model names 16 lifecycle moments:

| Canonical event      | Meaning                                                |
|----------------------|--------------------------------------------------------|
| `session_start`      | Session started, resumed, or cleared                   |
| `session_end`        | Session ended or terminated                            |
| `before_prompt`      | User prompt submitted before the agent processes it    |
| `before_tool`        | Tool call created before execution                     |
| `after_tool`         | Tool call completed successfully                       |
| `tool_error`         | Tool call failed                                       |
| `permission_request` | Provider is asking for permission                      |
| `human_in_the_loop`  | Provider is asking the user for input or clarification |
| `turn_complete`      | Agent turn completed                                   |
| `turn_error`         | Agent turn failed                                      |
| `subagent_start`     | Subagent spawned                                       |
| `subagent_stop`      | Subagent finished                                      |
| `before_model`       | Before sending a request to the model                  |
| `after_model`        | After receiving a model response                       |
| `before_compact`     | Before context compaction or summarization             |
| `notification`       | Provider-specific notification                         |

For a wrapper like Claudine, hooks matter in three ways.

First, hooks provide event normalization. Claude Code, Codex, Gemini, Goose, Kimi, OpenCode, Qwen, and Roo do not agree on event names, timing, or payload vocabulary. Claudine needs to turn `PreToolUse`, `BeforeTool`, `tool.execute.before`, and tool-related stream events into the same `before_tool` concept while preserving provider-specific fields in `extra`.

Second, hooks determine whether Claudine can block or mutate an action. A pre-tool hook that can deny or rewrite tool input is qualitatively different from a post-tool hook that can only warn the model after side effects have already happened. Permission hooks are different again: some providers let hooks approve, deny, or ask; others only expose the fact that a permission prompt appeared.

Third, hooks are the cleanest observability boundary. Session starts, compactions, subagent lifecycles, turn failures, notifications, and tool failures are often not visible as stable terminal text. Native hook payloads usually include correlation IDs, current working directory, tool names, model names, permission mode, transcript path, or event-specific status. That data is what makes Claudine's logs, reports, policies, and lifecycle actions reliable.

This report separates compiled Claudine provider support from broader research. Claude Code, Codex, Gemini, Goose, Kimi, OpenCode, Qwen, and Roo are the compiled provider roster. Pi and Kilo have refreshed hook research, but are not compiled Claudine providers yet. Roo is compiled, but the current refreshed `claudine/docs/research/hooks/*.md` corpus does not include a current Roo hook document, so Roo capability claims remain limited.

## Provider Differences

The provider hook landscape is uneven. The most important differences are event inventory, payload shape, whether hooks can control execution, return-value semantics, configuration format, and whether the provider is currently compiled into Claudine.

## Claude Code

Claude Code has the broadest and most mature native hook surface. It exposes session, prompt, prompt-expansion, tool, permission, notification, subagent, task, stop, config/file/worktree, compaction, elicitation, and session-end events.

Claude's payloads are rich and consistent. Most events carry `session_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, and often `model`. Tool events add `tool_name`, `tool_use_id`, `tool_input`, and either `tool_response` or error data. Subagent events add `agent_id`, `agent_type`, transcript path, and stop-loop guards. Notification and file/config events include their own typed discriminator fields.

Claude is also one of the strongest providers for control:

| Capability                  | Claude support                                    |
|-----------------------------|---------------------------------------------------|
| Block prompt                | Yes, `UserPromptSubmit` and `UserPromptExpansion` |
| Block tool before execution | Yes, `PreToolUse`                                 |
| Mutate tool input           | Yes, `updatedInput` for supported schema fields   |
| Approve or deny permission  | Yes, `PermissionRequest`                          |
| Retry denied permission     | Yes, `PermissionDenied` retry path                |
| Replace post-tool output    | Limited, mainly MCP output replacement            |
| Continue after turn stop    | Yes, `Stop` can block stopping and continue work  |
| Continue subagent           | Yes, `SubagentStop` can block stopping            |
| Observe compaction          | Yes                                               |
| Observe session end         | Yes, cleanup only                                 |

Return semantics are event-specific. Exit `0` generally means continue, but stdout may be interpreted as plain additional context or JSON. Exit `2` usually blocks or feeds stderr back to Claude. Modern Claude hook responses prefer `hookSpecificOutput` with event-specific fields such as `permissionDecision`, `updatedInput`, or nested permission decisions. Top-level `decision: "block"` still appears in some flows but is not the preferred modern shape for tool permission decisions.

Configuration is JSON under Claude settings files, with user, project, local project, and managed-policy scopes. On macOS/Linux/Windows, the common user path is `~/.claude/settings.json` or `%USERPROFILE%\.claude\settings.json`, with project `.claude/settings.json` and `.claude/settings.local.json`. Managed configuration also exists through platform-specific files, MDM/plist, Windows policy registry, or server-managed policy.

Claudine mapping is strong. Claude covers nearly every canonical event except model-level `before_model` / `after_model`. Notable collisions are many native events mapping to `notification`, `before_tool`, `after_tool`, or `session_start`. Claudine must preserve the native event name because `InstructionsLoaded`, `ConfigChange`, `CwdChanged`, `FileChanged`, `PreCompact`, and `Notification` are all materially different despite sharing broad canonical buckets.

## Codex CLI

Codex now has a first-class hook system and is no longer accurately described as notify-only. It supports `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, and `Stop`.

Codex payloads share a common shape: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `model`, and often `permission_mode` and `turn_id`. Tool events include `tool_name`, `tool_use_id`, `tool_input`, and `tool_response`. Subagent events include `agent_id`, `agent_type`, transcript path, and `stop_hook_active`.

Control is strong but narrower than Claude. `UserPromptSubmit` can block prompt submission. `PreToolUse` can deny supported tool calls and can rewrite supported inputs through `updatedInput`. `PermissionRequest` can allow or deny the normal approval flow. `PostToolUse` can add context or replace model-visible feedback after side effects have already occurred. `Stop` and `SubagentStop` treat blocking as continuation: the agent or subagent is asked to keep working.

A key caveat is coverage. Codex explicitly does not intercept every possible tool path; richer shell paths, `WebSearch`, and arbitrary non-shell/non-MCP tools may bypass `PreToolUse`/`PostToolUse`.

Codex configuration can be JSON (`hooks.json`) or inline TOML under `config.toml`, with user, project, system, managed, and plugin-bundled hook sources. Project hooks require trust. Hook trust is a first-class concern: `/hooks` lets users inspect sources, review trust, and disable non-managed hooks. Feature flags can enable or disable hooks for one invocation or persistently.

Claudine mapping is straightforward for most events:

| Codex event         | Canonical event      |
|---------------------|----------------------|
| `SessionStart`      | `session_start`      |
| `UserPromptSubmit`  | `before_prompt`      |
| `PreToolUse`        | `before_tool`        |
| `PermissionRequest` | `permission_request` |
| `PostToolUse`       | `after_tool`         |
| `SubagentStart`     | `subagent_start`     |
| `SubagentStop`      | `subagent_stop`      |
| `Stop`              | `turn_complete`      |
| `PreCompact`        | `before_compact`     |
| `PostCompact`       | `notification`       |

The main gap is compaction. Claudine has `before_compact`, but not a distinct `after_compact`. Codex's `PostCompact` has to collapse into `notification` or provider-specific extra data.

## Gemini CLI

Gemini exposes a compact but unusually powerful lifecycle surface. It supports session start/end, agent-turn hooks, model request/response hooks, tool-selection hooks, tool hooks, compression hooks, and notifications.

Its distinctive feature is model-level interception. `BeforeModel` receives an `llm_request` with model, messages, config, and tool config; it can mutate the request or synthesize an `llm_response` to skip the model call. `AfterModel` receives streamed response chunks and can replace the current chunk. `BeforeToolSelection` can filter the tool set before the model chooses tools.

Gemini's prompt and turn hooks also have strong control. `BeforeAgent` can deny or stop a prompt before planning. `AfterAgent` can reject the response and trigger retry, stop without retry, or clear context. `BeforeTool` can block or mutate tool input. `AfterTool` can hide the real result, inject context, or trigger a tail tool call whose result replaces the original.

Payloads consistently include `session_id`, `transcript_path`, `cwd`, `hook_event_name`, and `timestamp`. Tool payloads include `tool_name`, `tool_input`, `tool_response`, MCP context, and original request names for tail calls. Model payloads expose stable request/response objects.

Configuration is JSON under Gemini settings: system defaults, user settings, project settings, system override settings, and extension-bundled hooks. Project hooks require trusted folders and name/command fingerprinting. Hook controls are exposed through `/hooks` subcommands and `hooksConfig.enabled` / `hooksConfig.disabled`.

Claudine maps Gemini well, but Gemini reveals a canonical gap:

| Gemini event          | Canonical event  | Fit                 |
|-----------------------|------------------|---------------------|
| `SessionStart`        | `session_start`  | Good                |
| `SessionEnd`          | `session_end`    | Good, async cleanup |
| `BeforeAgent`         | `before_prompt`  | Good                |
| `AfterAgent`          | `turn_complete`  | Good                |
| `BeforeTool`          | `before_tool`    | Good                |
| `AfterTool`           | `after_tool`     | Good                |
| `BeforeModel`         | `before_model`   | Good                |
| `AfterModel`          | `after_model`    | Good                |
| `BeforeToolSelection` | none             | Gap                 |
| `PreCompress`         | `before_compact` | Good                |
| `Notification`        | `notification`   | Good                |

The important gap is tool selection. Claudine can model before-tool execution, but not the earlier phase where a provider mutates the tool list sent to the model.

## Goose

Goose implements hooks through Open Plugins hook containers. Hooks are command actions declared in plugin `hooks/hooks.json` files, discovered from user, project, and installed plugin directories. The surface is much smaller than Claude, Codex, Gemini, or Qwen, and the observed implementation only runs Open Plugins actions of `type: "command"`; `prompt` and `agent` action types from the Open Plugins spec are ignored.

Goose events include session start/end, prompt submit, generic pre/post tool, post-tool failure, read-file and shell pre-events, file-edit and shell post-events, and stop. Payloads are intentionally small. Common fields are `event` and `session_id`. Prompt hooks add `message`. Tool-shaped events add `matcher_context`, `tool_name`, `tool_input`, and `working_dir`. `Stop` adds `last_assistant_message` when assistant output exists. A `tool_output` field is declared in the context type, but observed post-tool hook code does not populate it.

Only two Goose events are blockable:

| Goose event  | Control                                                          |
|--------------|------------------------------------------------------------------|
| `PreToolUse` | Can permanently deny a tool call                                 |
| `Stop`       | Can prevent a turn from ending and ask the agent to keep working |

Everything else is observe-only, including `UserPromptSubmit`, `BeforeReadFile`, `BeforeShellExecution`, `AfterFileEdit`, and `AfterShellExecution`. Goose cannot mutate tool input, replace tool output, inject model context, or approve permission prompts through hooks. Hook failures are fail-open: non-2 exits, timeouts, spawn failures, and most stderr output are logged but do not stop the pending action.

Return semantics are simple. For blocking hooks, exit `0` allows. Exit `2` blocks and uses stderr as the reason, with a default denial reason if stderr is empty. Stdout JSON `{"decision":"block","reason":"..."}` also blocks for blockable events. Any other non-zero exit, timeout, or spawn failure is treated as allow and logged. For `PreToolUse`, a block becomes an internal tool error telling the model not to retry because it is a policy denial. For `Stop`, a block injects a system notification and hidden user message so the agent continues; after 8 consecutive stop blocks, `GOOSE_STOP_HOOK_BLOCK_CAP` overrides the hook and allows the turn to end.

Configuration is plugin-scoped rather than a central provider settings hook map. Hook files live under `<plugin-root>/hooks/hooks.json`, with common roots such as `~/.agents/plugins/<plugin-name>/` and `<project>/.agents/plugins/<plugin-name>/` on macOS/Linux, and `%USERPROFILE%\.agents\plugins\<plugin-name>\` or `<project>\.agents\plugins\<plugin-name>\` on Windows. Plugin allow/block state is controlled by `disabledPlugins` and `enabledPlugins` in `.config/goose/settings.json`, plus the main Goose config `plugins` map in `config.yaml` (`~/Library/Application Support/Block/goose/config/config.yaml` on macOS, `~/.config/goose/config.yaml` on Linux, and `%APPDATA%\Block\goose\config\config.yaml` on Windows). There is no dedicated CLI to list, test, or validate hooks, and no global environment variable was found to disable all hooks. Goose runs hook commands as `sh -c <command>` on all platforms, so Windows requires a POSIX `sh` such as Git Bash or MSYS2.

Claudine can map Goose events, but many collapse into the same canonical events:

| Goose event                                            | Canonical event |
|--------------------------------------------------------|-----------------|
| `SessionStart`                                         | `session_start` |
| `SessionEnd`                                           | `session_end`   |
| `UserPromptSubmit`                                     | `before_prompt` |
| `PreToolUse`, `BeforeReadFile`, `BeforeShellExecution` | `before_tool`   |
| `PostToolUse`, `AfterFileEdit`, `AfterShellExecution`  | `after_tool`    |
| `PostToolUseFailure`                                   | `tool_error`    |
| `Stop`                                                 | `turn_complete` |

The gaps are permission, subagent, model, compaction, human-in-the-loop, prompt blocking, and rich mutation. `SubagentStart` and `SubagentStop` appear in the Open Plugins spec and Goose docs note they are not currently emitted, so Claudine should not treat them as active Goose hook events. Because read/shell/file-edit events overlap generic tool events, Claudine must preserve `event` and `matcher_context` to distinguish `PreToolUse` from read/shell-specific observations.

## Kimi Code

Kimi Code has a Beta server-side hook system with 13 lifecycle events, plus an undocumented client-side wire hook mechanism. Server hooks are shell commands declared in user configuration under a `[[hooks]]` array. Wire hooks are registered during ACP/wire initialization and receive JSON-RPC `HookRequest` messages carrying the same event payloads.

Kimi supports pre-tool, post-tool, tool failure, prompt submit, stop, stop failure, session start/end, subagent start/stop, pre/post compact, and notification. Every payload includes `session_id`, `cwd`, and `hook_event_name`. Tool events add `tool_name`, `tool_input`, `tool_call_id`, and either `tool_output` or `error`; `tool_output` is truncated to 2000 characters. Prompt events add `prompt`. Session start adds `source` (`startup` or `resume`), while session end currently carries `reason: "exit"`. Subagent fields include `agent_name` plus truncated `prompt` or `response` fields. Compaction events include `trigger` and token counts. Notifications include `sink`, `notification_type`, `title`, `body`, and `severity`.

Kimi's control model is allow/block only. `PreToolUse` and `UserPromptSubmit` can block. `Stop` can block turn completion and inject the reason as a new user message so the agent runs one extra turn; `stop_hook_active` prevents repeated re-triggering. Most other events are informational. The current implementation treats `SubagentStop` as fire-and-forget despite documentation suggesting blocking, so Claudine should model it as observe-only unless a later version changes the source behavior.

Kimi does not mutate tool input, replace tool output, approve permissions, or inject plain stdout as model context in the observed implementation. Exit `2` blocks where supported and uses stderr as the reason. Exit `0` plus JSON `{"hookSpecificOutput":{"permissionDecision":"deny","permissionDecisionReason":"..."}}` also blocks. Exit `0` with empty, plain, or unrelated stdout allows; the engine currently inspects stdout only for the JSON deny envelope. Timeouts, crashes, invalid regexes, cancelled hook tasks, and other failures fail open. Matching server-side hooks run in parallel, and identical command strings are deduplicated.

Configuration is narrower than most providers. The documented default file is `~/.kimi/config.toml` on macOS/Linux and `%USERPROFILE%\.kimi\config.toml` on Windows, with TOML or JSON accepted. `--config-file <path>` loads an alternate config file, and `--config '<content>'` supplies inline JSON or TOML. `/hooks` lists configured hooks in-session, and `--debug` enables trace-level hook telemetry. A migrated `~/.kimi-code/config.toml` path exists on the observed host but is not covered by the public docs. No repository-scope, managed-scope, or global hook-disable mechanism was found.

Claudine mapping is reasonable but mostly observational:

| Kimi event                    | Canonical event  |
|-------------------------------|------------------|
| `SessionStart`                | `session_start`  |
| `SessionEnd`                  | `session_end`    |
| `UserPromptSubmit`            | `before_prompt`  |
| `PreToolUse`                  | `before_tool`    |
| `PostToolUse`                 | `after_tool`     |
| `PostToolUseFailure`          | `tool_error`     |
| `Stop`                        | `turn_complete`  |
| `StopFailure`                 | `turn_error`     |
| `SubagentStart`               | `subagent_start` |
| `SubagentStop`                | `subagent_stop`  |
| `PreCompact`                  | `before_compact` |
| `PostCompact`, `Notification` | `notification`   |

The main gaps are mutation, permission approval, effective subagent-stop blocking, and after-compaction fidelity. Claudine should also preserve Kimi-specific discriminator fields such as `tool_call_id`, `source`, `reason`, `agent_name`, `trigger`, `notification_type`, and `sink`, because many Kimi events collapse into `after_tool`, `notification`, or turn/session boundaries.

## OpenCode

OpenCode is structurally different from the command-hook providers. Its hook support is a Bun-loaded JavaScript/TypeScript plugin API, not a settings-file registry of shell commands. A plugin exports a function that receives OpenCode context and returns hook functions, tool definitions, auth methods, or provider model catalog extensions.

OpenCode has two distinct hook mechanisms:

| Mechanism             | Shape                                                                                                                                                     | Control                                                 |
|-----------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------|
| Dedicated named hooks | In-process functions such as `tool.execute.before`, `chat.params`, `shell.env`, and `tool.definition` receive typed inputs plus a mutable `output` object | Can mutate pending behavior; some can block by throwing |
| Catch-all event hook  | `event({ event: { id, type, properties } })` receives event-bus notifications                                                                             | Observe-only and fire-and-forget                        |

That distinction is the most important portability fact. The event bus can report lifecycle activity, but it cannot deny a tool call or change a model request. Preventive control and mutation live in dedicated hooks.

OpenCode's event inventory is large. The event bus exposes config/dispose-adjacent lifecycle, session created/updated/deleted/error/status/idle/compacted events, message and message-part updates/removals, permission asked/replied events, file edits, file-watcher updates, TUI prompt/command/toast events, and command execution notifications. Dedicated hooks cover user-message shaping, model parameters and headers, system/message transforms, compaction transforms, auto-continue behavior, final text completion, tool execution before/after, shell environment injection, tool-definition mutation, permission ask, and command execution. Some typed dedicated hooks, notably `chat.message`, `permission.ask`, and `command.execute.before`, are present in the interface but did not have confirmed runtime call sites in the current source research.

Payloads are provider-native TypeScript objects rather than stdin JSON. Event-bus payloads use `{ event: { id, type, properties } }`, where `properties` holds the event-specific data. Session events carry `sessionID` and `info` fields such as session id, title, project directory, agent, model id, provider id, and creation time. Permission events carry permission keys, patterns, metadata, and associated tool message/call IDs. Tool hooks receive `tool`, `sessionID`, and `callID`; post-tool hooks also receive `args`. Model hooks receive session, agent, model, provider, and message objects. Shell hooks receive `cwd` and optional session/call IDs.

OpenCode's mutating surfaces are broad:

| Hook                                   | Canonical fit                    | Mutation / control                                                                       |
|----------------------------------------|----------------------------------|------------------------------------------------------------------------------------------|
| `chat.message`                         | `before_prompt`                  | Mutates `output.message` / `output.parts`; throwing may abort, call site unconfirmed     |
| `chat.params`                          | `before_model`                   | Mutates temperature, topP, topK, max output tokens, and provider options                 |
| `chat.headers`                         | `before_model`                   | Mutates provider request headers                                                         |
| `experimental.chat.system.transform`   | `before_model` / `before_prompt` | Mutates system prompt array                                                              |
| `tool.definition`                      | no exact event                   | Mutates tool description and parameter schema before tools are sent to the model         |
| `tool.execute.before`                  | `before_tool`                    | Mutates `output.args`; throwing aborts the tool call                                     |
| `shell.env`                            | `before_tool`                    | Injects environment variables into shell subprocesses                                    |
| `tool.execute.after`                   | `after_tool`                     | Mutates result title, output, and metadata; throwing turns the result into an error path |
| `permission.ask`                       | `permission_request`             | Sets `output.status` to `allow`, `deny`, or `ask`; call site unconfirmed                 |
| `experimental.session.compacting`      | `before_compact`                 | Appends context or replaces the compaction prompt                                        |
| `experimental.compaction.autocontinue` | `notification`                   | Can disable synthetic continue after compaction                                          |
| `experimental.text.complete`           | `after_model`                    | Mutates final assistant text                                                             |
| `command.execute.before`               | `notification`                   | Mutates command parts or throws; call site unconfirmed                                   |

Return semantics are in-process rather than process-based. There is no stdin/stdout hook protocol, no `exit 2`, and no documented per-hook timeout. Dedicated hooks run sequentially in plugin load order and receive the same mutable `output` object. Returning a value is usually not the control channel; mutation of `output` is. Throwing from a dedicated hook can abort the current action, but exact error handling varies by hook site and surrounding Effect code. The catch-all `event` hook is fire-and-forget; return values and throws do not make it a blocking hook.

Configuration is split between local plugin files and JSONC config that loads plugins. User plugins live under `~/.config/opencode/plugins/*.{ts,js}` on macOS/Linux and `%USERPROFILE%\.config\opencode\plugins\*.{ts,js}` on Windows, with singular `plugin` directories also scanned. Project plugins live under `.opencode/plugins/*.{ts,js}` or `.opencode\plugins\*.{ts,js}`, with `.opencode/package.json` available for plugin dependencies. User and project `opencode.json{c}` files can load npm plugins through a plugin array. Managed config exists at `/Library/Application Support/opencode/opencode.json{c}` on macOS, `/etc/opencode/opencode.json{c}` on Linux, `%ProgramData%\opencode\opencode.json{c}` on Windows, and macOS managed-preference plist paths. `OPENCODE_PURE`, `OPENCODE_DISABLE_DEFAULT_PLUGINS`, `OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG_CONTENT`, and `OPENCODE_EXPERIMENTAL` materially affect which hooks load.

Claudine mapping is broad but lossy:

| OpenCode native area                                                           | Canonical event                                                                 |
|--------------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| `config` plugin hook                                                           | `session_start` or provider-specific initialization metadata                    |
| `dispose` plugin hook                                                          | `session_end` or provider-specific finalization metadata                        |
| `session.created`                                                              | `session_start`                                                                 |
| `session.deleted`, `session.idle`                                              | `session_end` or `notification`, depending on wrapper context                   |
| `session.error`                                                                | `turn_error` or `session_end` with error metadata                               |
| `session.status`                                                               | `notification`                                                                  |
| `session.compacted`                                                            | `notification`, because it is post-compaction                                   |
| `message.updated`, `message.part.updated`, `experimental.text.complete`        | `after_model` or `notification`                                                 |
| `chat.message`, `tui.prompt.append`                                            | `before_prompt` when tied to a submitted user message; otherwise `notification` |
| `chat.params`, `chat.headers`, system/message transforms                       | `before_model`                                                                  |
| `tool.execute.before`, `shell.env`                                             | `before_tool`                                                                   |
| `tool.execute.after`                                                           | `after_tool` or `tool_error` if the hook/action fails                           |
| `permission.ask`, `permission.asked`, `permission.replied`                     | `permission_request` with pre/post distinction in `extra`                       |
| `experimental.session.compacting`                                              | `before_compact`                                                                |
| File watcher, TUI toast, command, status, and miscellaneous event-bus messages | `notification`                                                                  |

OpenCode exposes concepts Claudine's 16-event model only partially captures: request header mutation, tool definition mutation, shell environment injection, compaction prompt replacement, auto-continue suppression, plugin initialization/finalization, and final text mutation. These should remain provider-specific extras rather than being flattened away. Claudine should also distinguish observation-only event-bus bridging from full OpenCode plugin semantics: observing the event bus is useful for logs, but it does not exercise OpenCode's mutating or blocking hook contract.

## Qwen Code

Qwen Code closely resembles the Claude/Gemini family but adds handler diversity and todo-specific lifecycle events. As of the refreshed research, the documented hook surface has four handler types: shell `command` hooks, `http` POST hooks, LLM `prompt` hooks, and internal `function` hooks used by the Skill system rather than exposed as a public registration API.

The native event set includes pre/post tool, post-tool failure, prompt submit, session start/end, stop, stop failure, subagent start/stop, pre/post compact, notification, permission request, todo created, and todo completed. Matcher behavior is event-specific: tool, subagent, session, and stop-failure matchers use JavaScript regular expressions; notification and compact events use exact-string matchers; `TodoCreated`, `TodoCompleted`, `UserPromptSubmit`, and `Stop` do not support matchers. Empty, omitted, or `"*"` matchers match all events of that type.

Payloads include common fields (`session_id`, `transcript_path`, `cwd`, `hook_event_name`, `timestamp`) plus event-specific fields. Tool and permission events include `tool_name`, `tool_input`, `tool_use_id`, `tool_call_id`, and `permission_mode`. Session events include `source`, `model`, `agent_type`, `permission_mode`, or end `reason`. Stop events include `stop_hook_active`, `last_assistant_message`, `context_usage`, `context_limit`, and `input_tokens`. Subagent events include `agent_id`, `agent_type`, `agent_transcript_path`, and permission context. Compact events include `trigger`, `custom_instructions`, or `compact_summary`. Todo events include `todo_id`, `todo_content`, status fields, `all_todos`, and `phase`.

Qwen supports several control channels:

| Capability                     | Qwen support                                                             |
|--------------------------------|--------------------------------------------------------------------------|
| Block prompt                   | Yes, via top-level `decision` on `UserPromptSubmit`                      |
| Block or deny pre-tool         | Yes, via `hookSpecificOutput.permissionDecision` or exit `2`             |
| Mutate tool input              | Yes, via `updatedInput`                                                  |
| Ask/allow/deny permission      | Yes, via `PermissionRequest` decision behavior and permission updates    |
| Block post-tool                | Limited; tool already ran, so this is downstream warning/context control |
| Continue/stop turn             | Yes, via `Stop`; top-level `continue: false` stops Qwen entirely         |
| Block subagent stop            | Yes, via `SubagentStop` decision                                         |
| Block todo creation/completion | Yes during the todo validation phase                                     |
| Inject context                 | Yes, through `hookSpecificOutput.additionalContext` and prompt hooks     |
| Async background command hooks | Yes, but async hooks cannot return decisions                             |

Return semantics vary by handler type. Command hooks receive the event JSON on stdin and return an exit code plus optional stdout JSON. HTTP hooks receive the same event JSON as the POST body and return JSON in the response body. Prompt hooks return `{ok, reason, additionalContext}` from a single model call. Function hooks are internal-only.

For command hooks, exit `0` means success and stdout is parsed as JSON. Exit `2` is a blocking error for blockable events; stdout is ignored and stderr is passed to the model as feedback. Other non-zero exits are non-blocking errors and are shown only in debug mode. Common output fields include top-level `continue`, `decision`, `reason`, `stopReason`, `suppressOutput`, `systemMessage`, and `hookSpecificOutput.additionalContext`.

The per-event decision contract is not one uniform field:

| Qwen event                                                                                                                      | Decision carrier                        | Effect                                                                                       |
|---------------------------------------------------------------------------------------------------------------------------------|-----------------------------------------|----------------------------------------------------------------------------------------------|
| `PreToolUse`                                                                                                                    | `hookSpecificOutput.permissionDecision` | `deny` cancels the tool; `allow`/`ask` affect permission flow; `updatedInput` mutates input  |
| `PermissionRequest`                                                                                                             | `hookSpecificOutput.decision.behavior`  | `allow` or `deny`; may carry `updatedInput`, `updatedPermissions`, `message`, or `interrupt` |
| `PostToolUse`                                                                                                                   | top-level `decision`                    | `block` is post-action feedback only; it cannot undo the tool                                |
| `UserPromptSubmit`, `Stop`, `SubagentStop`                                                                                      | top-level `decision`                    | `block` with `reason` stops the pending action                                               |
| `TodoCreated`, `TodoCompleted`                                                                                                  | top-level `decision`                    | `block`/`deny` only takes effect during `validation` phase                                   |
| `SessionStart`, `SessionEnd`, `SubagentStart`, `PreCompact`, `PostCompact`, `Notification`, `PostToolUseFailure`, `StopFailure` | no effective decision control           | observe and context-injection only, with `StopFailure` output ignored                        |

Top-level `continue: false` overrides event-specific decisions and stops Qwen entirely. Async command hooks run in the background and cannot decide; their output can be delivered on the next conversation turn through `systemMessage` or additional context.

Configuration is JSON under a top-level `hooks` key in `settings.json`. Each event contains matcher groups; each group has an optional `matcher`, optional `sequential`, and a `hooks` array. Matching handlers normally run in parallel. `sequential: true` serializes them, and earlier handlers can modify input seen by later handlers. Configuration precedence is hardcoded defaults, system defaults, user settings, project settings, system override settings, environment variables, then command-line arguments. The common user path is `~/.qwen/settings.json` or `%USERPROFILE%\.qwen\settings.json`, with project `.qwen/settings.json`. System defaults live under `/Library/Application Support/QwenCode/system-defaults.json`, `/etc/qwen-code/system-defaults.json`, or `C:\ProgramData\qwen-code\system-defaults.json`; system overrides use the same platform roots with `settings.json`. `QWEN_HOME`, `QWEN_CODE_SYSTEM_DEFAULTS_PATH`, and `QWEN_CODE_SYSTEM_SETTINGS_PATH` can redirect those locations. Project hooks require trusted-folder status.

Qwen does not provide dedicated hook-management subcommands; the `qwen hooks` command only prints help. Interactive `/hooks` can list configured hooks. `disableAllHooks: true` disables all hooks without deleting configuration. `--bare` skips hooks and other startup customizations. `--safe-mode` disables hooks and customizations, with `QWEN_CODE_SAFE_MODE=true` documented as an equivalent. `--approval-mode` and `--yolo` change the `permission_mode` reflected in payloads. `--debug` emits hook matching and execution details.

Claudine mapping is strong, with two important lossy areas: post-compaction has no first-class canonical event, and todo lifecycle events are not first-class lifecycle events.

| Qwen event           | Canonical event      | Notes                                                                                            |
|----------------------|----------------------|--------------------------------------------------------------------------------------------------|
| `SessionStart`       | `session_start`      | Preserve `source` (`startup`, `resume`, `clear`, `compact`)                                      |
| `SessionEnd`         | `session_end`        | Preserve `reason`                                                                                |
| `UserPromptSubmit`   | `before_prompt`      | Blockable; preserve `prompt`                                                                     |
| `PreToolUse`         | `before_tool`        | Blockable and mutable; preserve `tool_name`, IDs, and `tool_input`                               |
| `PermissionRequest`  | `permission_request` | Can allow/deny and may mutate input or permissions                                               |
| `PostToolUse`        | `after_tool`         | Post-action; blocking cannot undo side effects                                                   |
| `PostToolUseFailure` | `tool_error`         | Preserve tool fields, error, and interrupt status when present                                   |
| `Stop`               | `turn_complete`      | Pre-stop check; blocking means continue work, while `continue: false` stops Qwen                 |
| `StopFailure`        | `turn_error`         | Output ignored; preserve typed error fields                                                      |
| `SubagentStart`      | `subagent_start`     | Preserve `agent_id`, `agent_type`, and permission mode                                           |
| `SubagentStop`       | `subagent_stop`      | Blockable; preserve `stop_hook_active` and `agent_transcript_path`                               |
| `PreCompact`         | `before_compact`     | Preserve `trigger` and `custom_instructions`                                                     |
| `PostCompact`        | `notification`       | Preserve `trigger` and `compact_summary` because Claudine has no `after_compact`                 |
| `Notification`       | `notification`       | Preserve `notification_type`, `title`, and `message`                                             |
| `TodoCreated`        | `before_tool`        | Best fit when treated as validation around todo write behavior; preserve todo fields and `phase` |
| `TodoCompleted`      | `after_tool`         | Best fit when treated as todo status-change result; preserve todo fields and `phase`             |

The notable gaps and uncertainties are todo fidelity, exact `PostToolUse` blocking effect, multi-layer hook merge semantics, public function-hook registration, HTTP URL allowlist details, live reload behavior, exhaustive `StopFailure` error values, and the exact difference between `--bare` and `--safe-mode` for managed/system hooks.

## Roo Code

Roo Code is present in Claudine's compiled provider roster, but the current refreshed `claudine/docs/research/hooks/*.md` set does not include a current Roo hook research document. Older skill reference material maps Roo mostly through non-hook task, streaming, tool, waiting, and mode-change events rather than a mature native hook configuration surface.

For this report, Roo should be treated as a compiled provider with incomplete current hook research. Claudine can still normalize Roo-like task events into canonical lifecycle events where stream or task payloads expose them, but the refreshed research set does not justify claims about native hook blocking, mutation, return contracts, or configuration format. The gap is not just event mapping; it is native hook capability evidence.

## Research-Only Providers

Pi and Kilo have refreshed hook research but are not compiled Claudine providers today. They should inform future adapter work, not current supported-provider claims.

Pi exposes an in-process TypeScript Extension API rather than a declarative hook file. Modules register handlers with `pi.on(eventName, handler)`, and extensions can be loaded from settings, extension directories, package resources, or repeated `--extension` flags. Its events cover resource discovery, session start/shutdown, session switch/fork/tree operations, compaction, context preparation, provider request/response, agent start/end, turn start/end, message streaming, model selection, user input, user bash, tool call, and tool result events. It can block or mutate selected tool calls, transform user input, replace context, replace system prompts, cancel some session actions, and replace tool results. It does not have a separate native permission hook in the current research, and it explicitly has no built-in subagents.

Kilo is closer to OpenCode. It uses TypeScript/JavaScript plugin functions whose `server` function returns a `Hooks` object. Named hooks are awaited sequentially, often receive immutable `input` plus mutable `output`, and can mutate pending behavior or abort by throwing. Its catch-all `event` hook is not awaited and is observe-only. Kilo covers prompt/message preparation, model params and headers, slash-command execution, permission decisions, tool definition, tool execution before/after, shell environment injection, message-history transforms, system-message transforms, compaction prompt/context transforms, compaction auto-continue, and final assistant text completion.

Both Pi and Kilo reinforce the same design lesson as OpenCode: rich in-process hook APIs do not look like external command hooks. Supporting them in Claudine would require provider-specific extension/plugin bridges, trust handling, timeout posture, and clear adapter metadata. Their hook richness should not be counted as current Claudine provider support until the compiled provider enum and adapter layer include them.

## Cross-Provider Patterns

The providers and researched candidates fall into five capability tiers.

| Tier                               | Providers / candidates      | Shape                                                                            |
|------------------------------------|-----------------------------|----------------------------------------------------------------------------------|
| Rich blocking and mutation         | Claude, Gemini, Qwen, Codex | Native hooks can block prompts/tools/turns and sometimes mutate tool/model input |
| Simple block/observe command hooks | Goose, Kimi                 | Shell hooks, mostly allow/block, little or no mutation                           |
| In-process plugin extension        | OpenCode                    | Mutable TypeScript plugin hooks plus event bus observation                       |
| Research-only plugin candidates    | Pi, Kilo                    | Rich in-process hooks, but not compiled Claudine providers yet                   |
| Incomplete current evidence        | Roo                         | Compiled Claudine provider, but refreshed hook research is missing               |

Payload shapes vary by provider family. Claude/Codex/Gemini/Qwen converge on JSON objects with session IDs, cwd, transcript paths, event names, and tool-specific fields. Goose and Kimi use smaller payloads and rely on matcher context or provider-local names. OpenCode, Pi, and Kilo expose typed plugin objects and mutable output parameters instead of stdin/stdout contracts.

Blocking semantics are not portable without provider-specific interpretation:

| Provider | Important blocking nuance                                                                         |
|----------|---------------------------------------------------------------------------------------------------|
| Claude   | `Stop` block means continue; post-tool block cannot undo side effects                             |
| Codex    | `Stop`/`SubagentStop` block means continue; tool hook coverage is incomplete                      |
| Gemini   | `AfterAgent` deny retries; `AfterTool` can hide result but not undo execution                     |
| Goose    | Only `PreToolUse` and `Stop` block; failures fail open                                            |
| Kimi     | Block is mostly exit `2` or JSON deny; failures fail open; `SubagentStop` is fire-and-forget      |
| OpenCode | Dedicated hooks can throw or mutate; event bus listeners cannot block                             |
| Qwen     | Multiple handler types; top-level `continue: false` can stop execution; async hooks cannot decide |
| Roo      | Current blocking semantics not established by refreshed research                                  |
| Pi       | Research-only; in-process handlers can cancel or mutate selected events                           |
| Kilo     | Research-only; awaited named hooks can mutate or abort, catch-all bus events observe only         |

Return-value semantics are likewise provider-specific. Some use process exit codes (`0`, `2`), some use JSON on stdout, some use nested `hookSpecificOutput`, some use prompt-hook `{ok:false}`, some mutate an in-process `output` object, and some throw exceptions. Claudine should normalize the effect, not the raw return shape: allow, deny/block, ask, mutate, inject context, continue/retry, stop, or observe.

Configuration formats also vary:

| Provider | Configuration model                                                                                  |
|----------|------------------------------------------------------------------------------------------------------|
| Claude   | JSON settings files across user/project/local/managed scopes                                         |
| Codex    | `hooks.json`, inline TOML, managed config, plugin-bundled hooks, trust review                        |
| Gemini   | JSON settings plus extension hooks and `/hooks` controls                                             |
| Goose    | Open Plugins `hooks/hooks.json` inside plugin directories                                            |
| Kimi     | User TOML/JSON config, alternate/inline config, no repo/managed scope found                          |
| OpenCode | TypeScript/JavaScript plugins plus JSONC config plugin arrays                                        |
| Qwen     | JSON settings across system-default/user/project/system-override scopes                              |
| Roo      | Current refreshed hook configuration evidence missing                                                |
| Pi       | TypeScript extensions via settings, package resources, extension dirs, or CLI flags; researched only |
| Kilo     | TypeScript/JavaScript plugins via JSONC config and plugin dirs; researched only                      |

## Mapping Point of View

Claudine's canonical 16-event model is the right abstraction for the common lifecycle. It cleanly captures sessions, prompts, tools, permissions, turns, subagents, model calls, compaction, and notifications. The research supports keeping this model as the shared user-facing action surface.

The model should, however, be treated as a normalization layer, not as a lossless provider schema. Provider adapters must preserve native event names and important discriminators in `extra`, especially where many native events collapse into one canonical event.

The strongest canonical mappings are:

| Canonical event                    | Provider support quality                                                            |
|------------------------------------|-------------------------------------------------------------------------------------|
| `before_tool`                      | Strong across nearly all researched providers                                       |
| `after_tool`                       | Strong, but post-tool blocking means feedback/replacement, not undo                 |
| `tool_error`                       | Common in Goose, Kimi, Qwen, and tool-result payloads elsewhere                     |
| `before_prompt`                    | Strong in Claude, Codex, Gemini, Kimi, Qwen; observe-only in Goose                  |
| `turn_complete`                    | Strong, but stop/block means "continue" in several providers                        |
| `session_start` / `session_end`    | Common, but timing and cleanup guarantees vary                                      |
| `permission_request`               | Strong in Claude, Codex, Qwen, OpenCode; absent or weak elsewhere                   |
| `subagent_start` / `subagent_stop` | Strong in Claude/Qwen/Codex, inconsistent in Kimi, absent in Goose hooks            |
| `before_model` / `after_model`     | Strong in Gemini and OpenCode, limited elsewhere                                    |
| `before_compact`                   | Present in Claude, Codex, Gemini, Kimi, Qwen, OpenCode; post-compaction has no peer |
| `notification`                     | Broad catch-all, necessarily lossy                                                  |

The notable gaps are:

1. **After compaction**: Several providers expose `PostCompact`, `session.compacted`, or equivalent post-compaction events, but Claudine only has `before_compact` plus generic `notification`.
2. **Tool selection and tool definition mutation**: Gemini's `BeforeToolSelection`, OpenCode's `tool.definition`, and Kilo's research-only `tool.definition` happen before a concrete tool call exists. Mapping them to `before_tool` is wrong because no tool invocation has been chosen yet.
3. **Model request mutation detail**: `before_model` exists, but provider-specific mutation surfaces differ: Gemini can replace the request or synthesize a response; OpenCode can mutate params and headers; Pi and Kilo research shows still more context/request mutation shapes.
4. **Plugin initialization/finalization**: OpenCode's `config`, `dispose`, provider, auth, and tool plugin hooks affect the environment around a session, but they are not agent-turn lifecycle events in the same sense as `session_start` or `before_tool`.
5. **Task/todo lifecycle**: Claude task events and Qwen todo events can be mapped to tool or notification events, but that loses the distinction between an agent task artifact and a normal tool call.
6. **Human-in-the-loop**: The canonical model has `human_in_the_loop`, but current provider hook docs more often expose permission prompts, elicitation, notifications, or waiting states rather than one uniform HITL hook.
7. **Blocking semantics after side effects**: `after_tool`, `PostToolUse`, and similar events may be marked blocking by providers, but the only thing they can block is downstream model visibility or turn continuation. Claudine should label these as post-action feedback/replacement, not preventive control.
8. **Provider status and confidence**: Pi and Kilo are researched candidates, not compiled Claudine providers. Roo is compiled, but current refreshed hook research is missing. Claudine should not present Pi, Kilo, or Roo native hook support with the same confidence as researched-and-compiled providers until provider metadata and adapter work catch up.

The practical design stance is: Claudine should expose the 16 canonical events as the stable user contract, but every event dispatch should retain provider-native metadata, timing, capability flags, and provider support status. A lifecycle action configured for `before_tool` should know whether it is running before execution with mutation authority, before execution with deny-only authority, before execution with observe-only semantics, after provider permission selection, as a best-effort observation, or only as a researched future integration. That distinction is the difference between a policy engine, an observability log, and a misleading wrapper abstraction.
