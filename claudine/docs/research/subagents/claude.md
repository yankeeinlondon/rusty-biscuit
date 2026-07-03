---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://www.anthropic.com/claude-code
docs: https://code.claude.com/docs/en/overview
subagent_docs: https://code.claude.com/docs/en/sub-agents

support: first_class

locations:
  - os: macos
    scope: system
    path: /Library/Application Support/ClaudeCode/.claude/agents/
    notes: "Managed / organization-wide subagents. Highest precedence. Same directory also contains managed-settings.json and is read with admin privileges."
  - os: linux
    scope: system
    path: /etc/claude-code/.claude/agents/
    notes: "Managed / organization-wide subagents on Linux and WSL. Highest precedence."
  - os: windows
    scope: system
    path: C:\Program Files\ClaudeCode\.claude\agents\
    notes: "Managed / organization-wide subagents on Windows. Highest precedence; requires admin to write."
  - os: macos
    scope: user
    path: ~/.claude/agents/
    notes: "Personal subagents applied across all projects. Recursively scanned (subfolders are scanned as well but the directory path does not affect the agent's `name`)."
  - os: linux
    scope: user
    path: ~/.claude/agents/
    notes: "Personal subagents applied across all projects. Subfolders are scanned but do not affect identification."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\agents\\"
    notes: "Personal subagents on Windows. Resolved from %USERPROFILE%; `--bare` and `--disable-slash-commands` sessions do not watch the directory."
  - os: macos
    scope: repo
    path: .claude/agents/
    notes: "Project-scoped; merged via recursion from the launch directory up to the repo root. Also discovered inside any directory added with `--add-dir`."
  - os: linux
    scope: repo
    path: .claude/agents/
    notes: "Same as macOS — launch-dir-to-repo-root recursion plus `--add-dir` lookup. Same-scope collisions resolve to the closest-to-CWD definition."
  - os: windows
    scope: repo
    path: ".claude\\agents\\"
    notes: "Same as macOS / Linux. On WSL the project `.claude/agents/` is read from the WSL-side path; on native Windows the path uses backslashes."
  - os: macos
    scope: extension
    path: <plugin>/agents/
    notes: "Plugin subagents. Namespaced as `plugin-name:agent`; nested plugin subfolders register as `plugin-name:folder:agent`. `hooks`, `mcpServers`, and `permissionMode` frontmatter are ignored for security."
  - os: linux
    scope: extension
    path: <plugin>/agents/
    notes: "Same as macOS."
  - os: windows
    scope: extension
    path: "<plugin>\\agents\\"
    notes: "Same as macOS / Linux. Plugins are loaded from `.claude-plugin/plugin.json` manifest."
  - os: macos
    scope: other
    path: inline JSON via `claude --agents '...'`
    notes: "Session-only CLI-defined subagents. Not persisted to disk; highest precedence after managed settings for the lifetime of the session. Same field shape as file frontmatter plus a `prompt` field."
  - os: linux
    scope: other
    path: inline JSON via `claude --agents '...'`
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: inline JSON via `claude --agents '...'`
    notes: "Same as macOS / Linux. Quote PowerShell-safe; the value shape is identical."

format:
  file_names:
    - "*.md"
  frontmatter: true
  required_fields:
    - name (lowercase letters and hyphens; identity is carried by this field, not the filename)
    - description (routing signal for automatic delegation)
  optional_fields:
    - tools (allowlist of tool names or `Agent(agent_type1, agent_type2)`)
    - disallowedTools (denylist; MCP tool patterns `mcp__server` or `mcp__server__*`)
    - "model (alias sonnet/opus/haiku/fable, full model ID, or inherit)"
    - permissionMode (`default`/`acceptEdits`/`auto`/`dontAsk`/`bypassPermissions`/`plan`)
    - maxTurns
    - skills (list of skill names whose full content is preloaded into the subagent's context at startup)
    - mcpServers (inline definition or string reference by server name)
    - hooks (subagent-scoped hook definitions; `PreToolUse`/`PostToolUse`/`Stop` most common)
    - memory (`user`/`project`/`local` — persistent MEMORY.md scope)
    - background (`true` to force background execution)
    - effort (`low`/`medium`/`high`/`xhigh`/`max` — overrides session effort)
    - isolation (`worktree` — runs in a temporary git worktree)
    - "color (display color in task list: red/blue/green/yellow/purple/orange/pink/cyan)"
    - initialPrompt (auto-submitted when this agent is the main session agent via `--agent` or the `agent` setting)
  body_format: markdown
  notes: |
    The file is YAML frontmatter between `---` markers followed by Markdown body. The body becomes the subagent's system prompt — Claude Code does NOT prepend its full system prompt; subagents "receive only this system prompt plus basic environment details like the working directory". Identifiers in the body may not be needed for the agent to be discovered; identity comes from the `name` field. File names and folder paths (e.g. `agents/review/security.md`) are not part of the identifier, except for plugin subfolders which become `<plugin>:<folder>:<name>`. Same-scope name collisions load one of the two; from v2.1.196 `/doctor` reports which is active.

runtime:
  invocation: |
    Subagents are invoked explicitly through any of four escalating patterns:
    (1) **natural language**: name the subagent in the prompt; the parent decides whether to delegate (e.g. `Use the test-runner subagent to fix failing tests`).
    (2) **`@-mention`**: `@"code-reviewer (agent)"` forces a specific subagent for one task; plugin agents appear under their scoped name (e.g. `@agent-my-plugin:code-reviewer`).
    (3) **Built-in types**: Claude Code ships `Explore`, `Plan`, `general-purpose`, `statusline-setup`, and `claude-code-guide`. Custom definitions named `Explore` or `Plan` override the built-ins.
    (4) **Tool call by the parent**: the parent model's `Agent` (formerly `Task`) tool spawns subagents with parameters `subagent_type`, `prompt`, `description`, optional `model`, optional `run_in_background`, and (for agent teams) optional `name`. Renamed from `Task` to `Agent` in v2.1.63; `Task(...)` is still accepted as an alias in settings and agent definitions.
    (5) **Whole-session primary agent**: `claude --agent <name>` (or `agent:` in `settings.json`) makes the main thread itself the subagent — the subagent's system prompt replaces Claude Code's default prompt but `CLAUDE.md` files still flow through normally.
    (6) **Background agent view**: `claude --bg "..."` (or `claude agents`) launches the session as a background task; `--bg --agent <name>` combines both. As of v2.1.198 subagents spawned via the Agent tool default to background unless Claude needs the result synchronously, and v2.1.199 lets `claude --bg` also accept `--agent` for session-level background launches.
  parent_child_context: |
    Each subagent starts with a fresh, isolated context window and only sees:
    - the subagent's own system prompt (frontmatter body) plus basic environment details;
    - the delegated task prompt composed by the parent;
    - the full memory hierarchy the main conversation loaded (`~/.claude/CLAUDE.md`, project `.claude/CLAUDE.md`, `.claude/rules/*.md`, `CLAUDE.local.md`, managed policy files) — with the exception of the built-in `Explore` and `Plan`, which deliberately skip CLAUDE.md and git status;
    - the full content of any skill named in the `skills` frontmatter (preloaded at startup; maximum scope: 200 lines / 25 KB of `MEMORY.md` plus system-prompt memory instructions).
    Subagents do NOT see the parent's conversation history, prior skill invocations, or files already read by the parent. Foreground subagents return a final assistant message to the parent when they stop; background subagents keep running and the parent can `SendMessage` them with a `to: <agent_id|name>` later. When `CLAUDE_CODE_FORK_SUBAGENT=1`, the `fork` subagent type replaces the fresh-context behavior with full inheritance of the parent conversation.
  permissions_inheritance: |
    Permission mode inherits but may be overridden, with one important asymmetry:
    - A child `bypassPermissions` or `acceptEdits` parent cannot be downgraded; the parent's mode takes precedence and the child frontmatter `permissionMode` is ignored.
    - A child `auto`-mode parent forces the child into auto mode too; the child frontmatter `permissionMode` is ignored.
    - Otherwise the child's frontmatter `permissionMode` (`default`/`acceptEdits`/`auto`/`dontAsk`/`bypassPermissions`/`plan`) overrides the inherited mode.
    - Background subagents surface every permission prompt in the parent's session and name the subagent that is asking; Esc denies the one call without killing the subagent. Before v2.1.186 the background path auto-denied any prompting tool.
    - Only the parent's `permissions.allow`/`permissions.deny` and the user's own messages can grant approval — no agent's `SendMessage` message is treated as permission.
    - Tools the child cannot see (`AskUserQuestion`, `EnterPlanMode`, `ExitPlanMode` unless `permissionMode: plan`, `ScheduleWakeup`, `WaitForMcpServers`) are always unavailable regardless of `tools:` frontmatter.
  model_inheritance: |
    Resolution order, top wins:
    1. `CLAUDE_CODE_SUBAGENT_MODEL` environment variable (alias or full ID; `inherit` is treated as unset from v2.1.196).
    2. Per-invocation `model` parameter passed by the parent (Agent tool's `model` argument).
    3. Subagent definition's `model` frontmatter (`sonnet`/`opus`/`haiku`/`fable`/full ID/`inherit`; default if omitted is `inherit`).
    4. The main conversation's model.
    All four are checked against the `availableModels` organization allowlist; an excluded value is skipped and falls back to `inherit`. As of v2.1.198 subagents also inherit the main conversation's extended-thinking configuration — if extended thinking is on in the session, it is on for the subagent, and vice versa.
  tool_inheritance: |
    Default: every tool the parent has access to, including MCP tools. Inheritance is then narrowed:
    - `tools:` is an allowlist; if set, the child only sees those tools (with `Agent(agent_type1, agent_type2)` enforcing an agent-spawn allowlist for `--agent` main-thread agents).
    - `disallowedTools:` is a denylist; `mcp__server` or `mcp__server__*` removes MCP-server tool groups, `mcp__*` removes all MCP tools.
    - When both are set, `disallowedTools` is applied first and then `tools` is resolved against the remainder.
    Skill content is NOT inherited as a tool — list preloaded skills in `skills:`. Subagents can still invoke other project, user, and plugin skills through the Skill tool.
  max_turns: |
    Optional `maxTurns` frontmatter field; "Maximum number of agentic turns before the subagent stops". If omitted, the subagent can continue until it returns, hits an error, or exceeds the parent's `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`-gated auto-compaction. The conversation doc does not document a hardcoded turn limit for default subagents.
  notes: |
    Concurrency: multiple subagents can run in parallel via the Agent tool's `run_in_background: true` (or the v2.1.198+ default behavior of running in background unless synchronously required). Tool-call concurrency inside a single subagent is the same as the parent's.
    Nesting: subagents can spawn nested subagents via `tools: Agent` (the parenthesized `Agent(worker, researcher)` allowlist syntax is **ignored** in nested definitions). Depth limit is fixed at five: a subagent at depth five does not receive the Agent tool, and on v2.1.187+ a background subagent's depth is fixed at first spawn and does not change on resume. A `fork` subagent cannot spawn further forks.
    Selection: automatic delegation is driven by the task prompt plus the subagent's `description` field. To make a subagent a strong candidate include phrases like "use proactively".
    Disabling: a parent can block specific agent types via `permissions.deny: ["Agent(Explore)", "Agent(name)"]` or `claude --disallowedTools "Agent(Explore)"`. The built-in `Explore` and `Plan` are removed by setting `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS=1` (v2.1.198+); all built-ins are removed in the Agent SDK / `-p` mode by `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1`.
    Failure: foreground API errors return partial output with a note or `Agent terminated early due to an API error`; background agents are marked failed with the last assistant message included.
    Resume: a stopped subagent can be resumed either by the parent invoking it again or via `SendMessage` with the agent's ID / name as `to`; `SendMessage` checks (v2.1.199+) that the name still resolves to the same agent and refuses the send if a re-spawned agent has reused it. The check is scoped to the current conversation and resets on `/clear`.

observability:
  stream_events:
    - "SubagentStart (lifecycle event; emit it as a hook event, not as a stream-output event in `claude -p` by default)"
    - "SubagentStop (lifecycle event; hook counterpart of `Stop` when the agent finishes)"
    - "agent_id (in the JSON input of SubagentStart / SubagentStop hooks)"
    - "agent_type (in the JSON input of SubagentStart / SubagentStop hooks; value is the agent's frontmatter `name`, or the plugin-scoped `plugin:agent` form)"
    - "agent_transcript_path (SubagentStop only; points to the subagent's own JSONL transcript)"
    - "last_assistant_message (SubagentStop only; carries the final assistant text)"
    - "background_tasks and session_crons (SubagentStop only since v2.1.145; arrays scoped to the parent session)"
    - "compact_boundary (system event in the subagent's transcript when auto-compaction fires)"
    - "(stream-json mode; `--include-hook-events` adds hook lifecycle events including SubagentStart/SubagentStop to the output stream)"
  hook_events:
    - "SubagentStart (matcher targets agent_type: built-in name, custom agent name, or plugin-scoped name)"
    - "SubagentStop (matcher targets agent_type the same way)"
    - "PreToolUse / PostToolUse / PostToolUseFailure / PermissionRequest / PermissionDenied / UserPromptSubmit (fires inside subagent runs when scoped hooks are defined in the subagent frontmatter)"
    - "Stop (in subagent frontmatter, automatically converted to SubagentStop at runtime)"
  session_ids: true
  notes: |
    Subagents get a stable `agent_id` and `agent_type` and write their own JSONL transcript under `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl` (the parent's `transcript_path` is also included in `SubagentStop` as the main-session transcript for comparison). Transcript retention is controlled by `cleanupPeriodDays`, defaulting to 30 days. `SendMessage` resume relies on this same `agent_id`/`name` pair.

    Because SubagentStart/SubagentStop are hook lifecycle events rather than model-output events, they appear in the stream-json output only when `--include-hook-events` is enabled alongside `--output-format stream-json`. Wrapper-based consumers should listen to those hook-name events (or to the `agent_id` / `agent_type` fields on the regular lifecycle stream) for starts/stops. The local `~/.claude/agents/catalog.json` (auto-generated when the user runs `/agents` or when the catalog needs refresh) lists every user-scope agent with `source`, `model`, `tools`, `description`, and absolute `path` — useful when a wrapper wants to enumerate subagents without spawning a session.

portability:
  portable: false
  non_portable_assets:
    - "`name` field (must be lowercase letters and hyphens; provider-specific identifier shape)"
    - "`tools` / `disallowedTools` lists (Claude Code tool names; other providers use different tool identifiers)"
    - "`Agent(agent_type1, agent_type2)` allowlist syntax (Claude Code only)"
    - "`mcpServers` references (Claude Code MCP server-shape: `stdio`/`http`/`sse`/`ws` inline definitions)"
    - "`permissionMode` values: `default`/`acceptEdits`/`auto`/`dontAsk`/`bypassPermissions`/`plan` (Claude Code-specific)"
    - "`model` aliases: `sonnet`/`opus`/`haiku`/`fable` (Claude-specific; providers use their own aliases)"
    - "`background` / `effort` / `isolation: worktree` / `color` (Claude Code-only frontmatter fields)"
    - "`memory` frontmatter with `user` / `project` / `local` scopes and `MEMORY.md` truncation rule (Claude Code-specific)"
    - "`hooks` block scoped to subagent lifecycle (Claude Code hook JSON schema; plugin subagents lose hooks by policy)"
    - "`initialPrompt` (auto-submitted when the agent is the main session)"
    - "Built-in agent types `Explore`, `Plan`, `general-purpose`, `statusline-setup`, `claude-code-guide` (no equivalents on other providers)"
    - "Body prompt text that references Claude Code-specific tools, environment variables (${CLAUDE_*}), or environment markers (CLAUDE_CODE_REMOTE)"
  rewrite_needed: true
  notes: |
    The Markdown body carries most of the agent's *purpose*, and the `description` field's routing intent can be carried across providers when re-targeted to their router language. Translation work for a portable copy:
    - rename `name` to the target provider's identifier shape;
    - remap tool names in `tools`/`disallowedTools` and any `Agent(...)` allowlist;
    - replace `permissionMode` with the target's permission vocabulary;
    - replace `mcpServers` with the target provider's MCP shape (and review whether inline-MCP works the same way);
    - drop or rewrite `model` aliases / `memory` / `hooks` / `initialPrompt` / `isolation: worktree` / `background` / `effort` / `color`;
    - rebuild the body in the target provider's system-prompt style.
    `claude --agents` inline JSON has no on-disk shape — it must be rewritten into the target provider's session-only definition mechanism. Plugin-packaged subagents are particularly hard to port because Claude Code forces `hooks`, `mcpServers`, and `permissionMode` to be stripped at load time for security, so providers that allow those fields inside their plugin equivalents still need a translation pass.

cli_params:
  - flag: --agent <name>
    description: "Run the entire session as the named subagent (its system prompt replaces Claude Code's default; CLAUDE.md still flows through). Accepts plugin-scoped names and folder-qualified names, e.g. `claude --agent my-plugin:review:security`."
    example: "claude --agent code-reviewer"
  - flag: --agents '<json>'
    description: "Define one or more session-only subagents inline as JSON. Same fields as file frontmatter plus `prompt` for the system prompt. Highest precedence after managed settings. Subject to `--strict-mcp-config` differently from file-defined subagents."
    example: "claude --agents '{\"reviewer\":{\"description\":\"Reviews code\",\"prompt\":\"You are a code reviewer\",\"tools\":[\"Read\",\"Grep\",\"Glob\"],\"model\":\"sonnet\"}}'"
  - flag: --disallowedTools "Agent(Explore)" (also --disallowed-tools)
    description: "Block a specific built-in or custom agent type from being invoked this session. Multiple types can be comma-separated."
    example: "claude --disallowedTools \"Agent(Explore),Agent(my-custom-agent)\""
  - flag: --allowedTools / --allowed-tools (with Agent allowlist in frontmatter)
    description: "Pre-allow tools or `Agent(agent_type)` allowlists so prompts skip permission check. Most relevant when an agent definition wants to gate which subagent types another subagent can spawn."
    example: "\"Agent(worker, researcher), Read, Bash\" (in a subagent's frontmatter)"
  - flag: --bg / --background
    description: "Launch the session itself as a background task and return immediately with the session id. Combine with `--agent` to background a specific subagent. Cannot be combined with `-p`/`--print`."
    example: "claude --bg --agent code-reviewer \"review my changes\""
  - flag: --model <alias-or-id>
    description: "Pick the model for the session with an alias (`sonnet`/`opus`/`haiku`/`fable`) or a full model ID. The same values are accepted by subagent `model` frontmatter and the Agent tool's `model` parameter; the Agent tool's per-invocation value still loses to `CLAUDE_CODE_SUBAGENT_MODEL`."
    example: "claude --model claude-sonnet-5"
  - flag: --continue / -c (with --agent / --fork-session)
    description: "Resume the latest conversation; combine with `--fork-session` to start a new session id and `--agent` to resume the session as a particular subagent."
    example: "claude --continue --agent code-reviewer"
  - flag: --resume / -r (with --agent / --fork-session)
    description: "Resume a specific session by id or name; combine with `--agent` to resume as that subagent. Background sessions appear in the picker marked `bg` (v2.1.144+)."
    example: "claude --resume auth-refactor --agent code-reviewer --fork-session"
  - flag: --setting-sources user,project,local
    description: "Restrict which settings scopes are loaded. Removing `project` or `user` can prevent repo- or user-scope subagent files from loading."
    example: "claude --setting-sources user,project"
  - flag: --settings <file-or-json>
    description: "Overlay a JSON settings file (or inline JSON string) for the session. Useful for embedding a `permissions.deny` block with `Agent(...)` entries."
    example: "claude --settings ./ci-settings.json"
  - flag: --safe-mode / CLAUDE_CODE_SAFE_MODE=1
    description: "Disable CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands AND custom agents/agents-md, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP servers, and auto-memory. Custom subagents are skipped entirely."
    example: "claude --safe-mode"
  - flag: --bare / CLAUDE_CODE_SIMPLE=1
    description: "Minimal mode: skips auto-discovery of hooks, skills, plugins, MCP servers, auto-memory, and CLAUDE.md. Custom agents inside `.claude/agents/` and `~/.claude/agents/` are not loaded either."
    example: "claude --bare -p \"summarize\""
  - flag: --permission-mode <default|acceptEdits|auto|dontAsk|bypassPermissions|plan>
    description: "Start the session (and inherited by child subagents unless overridden) in the chosen permission mode. Same set accepted by subagent frontmatter `permissionMode`."
    example: "claude --permission-mode plan"
  - flag: --include-hook-events (paired with --output-format stream-json)
    description: "Includes lifecycle hook events (including SubagentStart and SubagentStop) in the stream-json output so wrappers can observe subagent starts/stops without parsing the transcript."
    example: "claude --output-format stream-json --include-hook-events -p \"...\""
  - flag: --strict-mcp-config (and --bare)
    description: "Strict MCP server filtering; affects which `mcpServers` declared in a subagent frontmatter are honored. Inline `--agents` and SDK `agents` options are exempt because they are explicit caller input."
    example: "claude --strict-mcp-config"
  - flag: --add-dir <dir> [...]
    description: "Add a working directory; if it contains a `.claude/agents/` subdirectory those agents load alongside project agents."
    example: "claude --add-dir ../shared"
  - flag: --plugin-dir / --plugin-url
    description: "Load a plugin from a directory, archive, or URL for this session; plugin `agents/` ships subagents that appear in @-mentions under their scoped names."
    example: "claude --plugin-dir ./my-plugin"
  - flag: --allow-dangerously-skip-permissions / --dangerously-skip-permissions
    description: "Adds or starts in `bypassPermissions` mode. Both descend into any spawned subagent (parent mode wins over frontmatter overrides)."
    example: "claude --dangerously-skip-permissions"

env_vars:
  - name: CLAUDE_CODE_SUBAGENT_MODEL
    effect: "Override the model used for every subagent (alias or full ID). Position 1 of the four-tier subagent model resolution. Set to `inherit` to use normal resolution (since v2.1.196; earlier versions used `inherit` to force-override on the main-conversation model)."
  - name: CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS
    effect: "When `1` (v2.1.198+), the built-in `Explore` and `Plan` subagents are removed in interactive sessions. Custom subagents named `Explore` or `Plan` are unaffected. Use `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1` for the Agent SDK / non-interactive mode."
  - name: CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS
    effect: "Removes every built-in subagent (`Explore`, `Plan`, `general-purpose`, `statusline-setup`, `claude-code-guide`) from the Agent SDK and `claude -p` mode so only user-provided subagents are available."
  - name: CLAUDE_CODE_FORK_SUBAGENT
    effect: "`1` to enable fork mode (the `fork` subagent type inherits the full parent context instead of starting fresh; all subagent spawns become background regardless of `background:` frontmatter); `0` to disable and override any server-side rollout. A fork cannot spawn further forks."
  - name: CLAUDE_CODE_DISABLE_BACKGROUND_TASKS
    effect: "`1` disables all background-task functionality, keeping subagent spawns in the foreground. Takes precedence over `CLAUDE_CODE_FORK_SUBAGENT=1`."
  - name: CLAUDE_AUTOCOMPACT_PCT_OVERRIDE
    effect: "Override the auto-compaction threshold percentage; applies to subagents as well as the parent conversation."
  - name: CLAUDE_CODE_SAFE_MODE
    effect: "`1` disables custom agents/agents-md along with skills, hooks, plugins, MCP servers, CLAUDE.md, and other customizations for one session. Equivalent to `--safe-mode`."
  - name: CLAUDE_CODE_SIMPLE
    effect: "`1` skips auto-discovery of hooks, skills, plugins, MCP servers, auto-memory, and CLAUDE.md (and therefore also skips discovery of subagents defined in those locations). Equivalent to `--bare`."
  - name: CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS
    effect: "Adjacent flag for the experimental agent-teams protocol (which can consume subagent definitions). Not a subagent-loading flag itself; listed here because agent-teams consume the same `agents/` files."
  - name: ANTHROPIC_MODEL
    effect: "Sets the main-conversation model. Used by subagent model resolution position 4 (`inherit` default)."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `linking` module currently recognizes Claude Code skills, slash commands, and AGENTS.md, but no entry covers user-defined subagents (`~/.claude/agents/`, `.claude/agents/`, plugin `agents/`, managed `.claude/agents/`, CLI `--agents`). The agent-listing feature (`claudine agents`) and any future `claudine run --agent <name>` will need a sibling that enumerates Claude Code subagent files the same way the skills linker enumerates `SKILL.md` files. The new model resolution ladder (env var → per-invocation → frontmatter → parent) is also load-bearing for the planned lifecycle `proxy` / `resume` actions: any wrapper that wants to resume a subagent must recognize `agent_id` + `agent_type` and the corresponding `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl` transcript.
---

# Claude Code Subagents

## Overview

Claude Code treats user-defined **subagents** as a first-class feature: durable Markdown files with YAML frontmatter that change who does work inside a session. The provider calls them "subagents" — every Claude Code `Agent` (formerly `Task`) tool call, every plugin `my-plugin:agent` registration, every `--agent` whole-session replacement, and every `/agents` library entry is built from a subagent definition. Support is `first_class`: there are named scopes (managed/system, personal, project, CLI session, plugin), a documented frontmatter schema, runtime delegation semantics, and a fresh `agent_id` / `agent_type` lifecycle that hooks and stream-json consumers can observe.

This topic's scope is the *definition* of subagents — where files live, what frontmatter they accept, how the parent picks one, what context and permissions the child gets, and how a wrapper can observe start/stop. Hook event semantics (`SubagentStart` / `SubagentStop` payload shape, matcher rules, exit-code behavior) live in the hooks topic; this document records only **which** events expose agent lifecycle. Agent teams and the `Agent` tool's parameter shape are mentioned only as they affect subagent loading — they are separate, larger topics owned by their research areas. The hooks topic's `enabledPlugins` / packaging boundary still applies: an agent definition bundled in a plugin keeps its semantics here; the packaging is recorded by the plugins topic.

## Locations

Claude Code loads subagent files from five scopes; the order below is the precedence order with the highest-priority source on top.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| Managed / system | `/Library/Application Support/ClaudeCode/.claude/agents/` | `/etc/claude-code/.claude/agents/` | `C:\Program Files\ClaudeCode\.claude/agents/` | Deployed by organization admins; same conventions as other Claude Code managed resources. Highest precedence. |
| CLI session | inline JSON via `claude --agents '{...}'` | same | same | Session-only; not persisted. Forms the second tier alongside managed settings. |
| Project | `.claude/agents/` (recursive from launch dir to repo root) | same | `.claude\agents\` | Walked upward from the launch directory; `--add-dir` directories are scanned too. Same-scope collisions resolve to the closest definition (v2.1.178+). |
| Personal | `~/.claude/agents/` | `~/.claude/agents/` | `%USERPROFILE%\.claude\agents\` | Personal subagents applied across all projects. Subfolders are scanned but do not change the agent's identifier. |
| Plugin | `<plugin>/agents/` | same | `<plugin>\agents\` | Namespaced as `plugin-name:agent`; nested plugin subfolders register as `plugin-name:folder:agent`. `hooks`, `mcpServers`, `permissionMode` are stripped for security. |

The watcher reloads `*.md` changes within a few seconds for directories that existed at session start. Two edge cases still require a restart: the watcher does not notice *new* `agents/` directories created mid-session, and sessions launched with `--disable-slash-commands` never watch the directory at all. `--bare` and `--safe-mode` skip the entire discovery path so no subagents load.

On this host (macOS), observed: `~/.claude/agents/` contains 28 user-scope subagent files; Claude Code auto-generates `~/.claude/agents/catalog.json` summarizing them with `{name, path, description, source: "user", model, tools}` per entry. The host's `.claude/agents/` repo path (`/Volumes/coding/personal/rusty-biscuit/.claude/agents`) is recorded in `.claude/settings.json` and is exposed via the same `catalog.json` API.

## Definition Format

A subagent file is a single Markdown file with YAML frontmatter between `---` markers, where the body becomes the subagent's system prompt. Identity comes from the `name` field, not the filename or directory path.

```markdown
---
name: code-reviewer
description: Reviews code for quality and best practices. Use proactively after non-trivial code changes.
tools: Read, Glob, Grep, Bash
model: sonnet
permissionMode: default
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./scripts/block-destructive.sh"
---

You are a code reviewer focused on quality, security, and best practices.
For each issue, explain the problem, quote the offending code, and propose a
minimal patch. Prefer small, focused notes over broad rewrites.
```

The same shape is also supported inline via `claude --agents` (and the Agent SDK `agents` parameter) by replacing the body with a `prompt` string:

```bash
claude --agents '{
  "code-reviewer": {
    "description": "Reviews code for quality and best practices.",
    "prompt": "You are a code reviewer focused on quality, security, and best practices.",
    "tools": ["Read", "Glob", "Grep", "Bash"],
    "model": "sonnet",
    "permissionMode": "default"
  }
}'
```

Recognized frontmatter fields:

- Required: `name` (lowercase letters and hyphens; **identity**), `description` (routing signal).
- Tooling / permissions: `tools`, `disallowedTools`, `permissionMode`, `mcpServers`, `maxTurns`, `memory`, `skills`.
- Model / execution: `model`, `effort`, `background`, `isolation`, `initialPrompt`, `color`.
- Lifecycle: `hooks` (subagent-scoped lifecycle hooks).

The body inherits any system-prompt content from `${CLAUDE_SKILL_DIR}`, `${CLAUDE_PROJECT_DIR}`, and the standard `${CLAUDE_SESSION_ID}`-style substitutions, but Claude Code's full default system prompt is *not* prepended — subagents "receive only this system prompt plus basic environment details like the working directory".

## Runtime Behavior

A subagent is delegated via Claude's `Agent` tool (renamed from `Task` in v2.1.63; aliases still work). The parent's parameters — `subagent_type`, `prompt`, `description`, `run_in_background`, `model` (optional) — pick the definition and shape the task. `subagent_type` matches either a built-in name (`Explore`, `Plan`, `general-purpose`, `statusline-setup`, `claude-code-guide`) or a custom `name`; plugin agents use the scoped form `plugin-name:agent` (or `plugin-name:folder:agent` for nested plugin subfolders). As of v2.1.198 the default concurrency is background — Claude only forces a subagent to run in the foreground when it needs the result synchronously. `claude --bg --agent <name>` backgrounded the entire session around the chosen subagent on v2.1.199+.

The child receives:

- Its own frontmatter body as the system prompt.
- A short environment summary (working directory, platform); not Claude Code's full system prompt.
- The task prompt composed by the parent.
- The full memory hierarchy the parent loaded (`~/.claude/CLAUDE.md`, project rules, `CLAUDE.local.md`, managed policies), unless the child is one of the built-in `Explore` or `Plan`.
- The full body of any skill listed in the `skills` frontmatter (preloaded at startup; preloading still respects `disable-model-invocation: true`, skipping skills that opted out).
- All tools the parent has access to (minus the always-unavailable `AskUserQuestion`, `EnterPlanMode`, `ExitPlanMode`-without-`plan`-mode, `ScheduleWakeup`, `WaitForMcpServers`), then narrowed by `disallowedTools` and `tools`.

It does **not** receive the parent's conversation history, prior skill invocations, or files the parent has already read — except via the experimental `fork` subagent type (`CLAUDE_CODE_FORK_SUBAGENT=1`), which inherits the parent's conversation instead of starting fresh. A fork cannot spawn further forks, and nested delegation depth is capped at five: a depth-five subagent does not receive the Agent tool at all. A background subagent's depth is fixed at first spawn (v2.1.187+) and is preserved on resume.

The child's returned state to the parent is:

- **Foreground subagent**: the final assistant message (also delivered to the `SubagentStop` `last_assistant_message` field).
- **Background subagent**: nothing immediate; the parent can `SendMessage` with `to: <agent_id|name>` to deliver follow-up direction. A stopped subagent auto-resumes in the background when it receives a `SendMessage`.

Permission mode is inherited but may be overridden with one important asymmetry: when the parent uses `bypassPermissions`, `acceptEdits`, or `auto`, the parent mode wins and the child's `permissionMode` frontmatter is ignored. For all other modes the child's frontmatter overrides. Child subagents cannot escalate permission approvals by sending messages to the parent — only the parent's `permissions.allow`/`permissions.deny` and the user's own messages can grant approval. The `Agent(agent_type1, agent_type2)` allowlist in a subagent's frontmatter only matters when that subagent is the *main-thread* agent (launched with `claude --agent`); inside a nested subagent `Agent(worker, researcher)` collapses to plain `Agent`.

Built-in subagents can be disabled in two distinct ways. To remove only `Explore` and `Plan`, set `CLAUDE_CODE_DISABLE_EXPLORE_PLAN_AGENTS=1` (v2.1.198+, interactive sessions); custom subagents named `Explore` or `Plan` keep working. To remove every built-in type in the Agent SDK or `claude -p` mode, set `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS=1`. A specific agent type — built-in or custom — can be blocked anywhere with `permissions.deny: ["Agent(name)"]` or `claude --disallowedTools "Agent(name)"`.

## Observability

Subagent starts and stops are visible to wrappers through three coordinated surfaces:

1. **Hook events**: `SubagentStart` (when the subagent is spawned) and `SubagentStop` (when the subagent completes) fire as configurable lifecycle events. The matcher targets `agent_type`: built-in names (`Explore`, `Plan`, `general-purpose`, `statusline-setup`, `claude-code-guide`), custom `name` values, or plugin-scoped identifiers (`my-plugin:agent` or `my-plugin:folder:agent`). On Claude Code v2.1.195+ a hyphenated matcher like `db-agent` matches *exactly*; before that it is unanchored regex and also fires for `prod-db-agent`. The full JSON input schema adds to the shared fields:
   - `agent_id` — stable unique identifier for the subagent (assigned when the subagent is spawned).
   - `agent_type` — the matcher target (the agent's frontmatter `name`, or the plugin-scoped form).
   - For `SubagentStop`: also `agent_transcript_path` (the subagent's own JSONL under `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl`), `last_assistant_message` (the final assistant text), and since v2.1.145 also `background_tasks` + `session_crons` (scoped to the parent). The parent's own `transcript_path` is also delivered so the hook can correlate.
   - `SubagentStop` decision control mirrors `Stop` — `decision: "block"` keeps the subagent running and feeds `reason` back as its next instruction; to inject context into the parent, use a `PostToolUse` hook on the `Agent` tool instead.

2. **Stream output**: the regular stream-json output records the subagent's tool calls in the parent's session as `PreToolUse`/`PostToolUse` events keyed by tool `Agent` plus `agent_id`/`agent_type` fields; run with `--include-hook-events --output-format stream-json` to also receive `SubagentStart`/`SubagentStop` events over stdout.

3. **Transcripts**: the subagent writes its own JSONL transcript to `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl` (claude-code-clean removes it after `cleanupPeriodDays`, default 30 days). Auto-compaction emits a `compact_boundary` system event with `compactMetadata: { trigger: "auto", preTokens }`.

## Portability

Subagents are **not portable** across providers as-is. The body prompt — the bulk of the agent's *purpose* — is provider-neutral Markdown and can be lifted verbatim, but the frontmatter is almost entirely provider-specific:

| Field | Portable? | Rewrite target |
|---|---|---|
| `name` | depends | Must match the target provider's identifier rule. |
| `description` | yes | Carries the routing signal across providers. |
| `tools` / `disallowedTools` | no | Remap tool identifiers to the target provider's vocabulary. |
| `Agent(agent_type1, agent_type2)` | no | Only meaningful for `--agent` main-thread agents in Claude Code. |
| `permissionMode` | no | Remap to the target provider's permission mode set. |
| `mcpServers` | partial | The MCP shape is shared but Claude Code adds `mcp__*` patterns and security-mandated stripping for plugins. |
| `model`, `effort`, `isolation`, `background`, `color` | no | Provider-specific. |
| `memory` | no | The `MEMORY.md` truncation rule plus `user`/`project`/`local` scopes are Claude-specific. |
| `hooks`, `initialPrompt` | no | Claude Code-only. |
| Body prompt | partial | Verbatim is fine, but references to Claude Code-only tools, `${CLAUDE_*}` env vars, or `CLAUDE_CODE_REMOTE` markers need rewriting. |

`claude --agents` inline JSON has no on-disk shape and must be rewritten into the target provider's session-only definition mechanism. Plugin-packaged subagents are particularly hard to port because Claude Code forces `hooks`, `mcpServers`, and `permissionMode` to be stripped at load time — providers that allow those fields inside their plugin equivalents still need a translation pass.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy`/`resume` actions, what matters about Claude Code subagents:

- Treat `~/.claude/agents/*.md` and `.claude/agents/*.md` (and `.claude/agents/**/*.md` via recursive scan) as the canonical user- and repo-scope subagent locations. Plugin `agents/` directories register agents under namespaced names; nested plugin subfolders register as `plugin-name:folder:agent`. Managed-system subagents live under `/Library/Application Support/ClaudeCode/.claude/agents/` (macOS), `/etc/claude-code/.claude/agents/` (Linux/WSL), and `C:\Program Files\ClaudeCode\.claude/agents` (Windows).
- The `claude agents` catalog (auto-summarized in `~/.claude/agents/catalog.json` and exposed via `claude agents --json`) lists every user-, repo-, and managed-scope agent with `{name, path, description, source, model, tools}` — use that as the listing source instead of re-walking the filesystem twice.
- A linked subagent is portable when its `body` uses only standard Markdown and its frontmatter carries only `name`, `description`, plus optional `model`, `permissionMode`, and a `tools` list that already targets the destination provider's vocabulary. Flag assets that depend on `hooks`, `mcpServers`, `permissionMode`, `memory`, `isolation`, `effort`, `background`, `color`, `initialPrompt`, or `claude --agents` inline JSON — they need rewriting, stripping, or host gating before they can land elsewhere.
- For lifecycle `proxy`/`resume`: the wrapper must capture and replay `agent_id` + `agent_type` for the subagent it wants to address. The subagent's stable transcript at `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl` is the source of truth for resume; the parent transcript's `transcript_path` plus the child's `agent_transcript_path` come paired on `SubagentStop` so a wrapper can correlate them without guessing. `SendMessage`-based resume needs the same `agent_id`/`name` after v2.1.199 introduced a same-conversation collision check.
- Permission policy: when `bypassPermissions`, `acceptEdits`, or `auto` is in force at the parent, the child inherits it; otherwise the child's `permissionMode` frontmatter can narrow it. A wrapper that pre-loads its own approval policy should treat the parent's effective permission mode as the ceiling.
- Whenever Claudine's wrapper code grows a `claude agents` command or a `--agent` resolution path, model resolution must follow the four-tier ladder (`CLAUDE_CODE_SUBAGENT_MODEL` → Agent tool `model` parameter → frontmatter `model` → main-conversation model). The default since v2.1.198 is background concurrency; `--bg` keeps the user prompt out of the foreground and `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1` overrides it.

## Sources

- [Claude Code — Sub-agents](https://code.claude.com/docs/en/sub-agents)
- [Claude Code — Hooks reference](https://code.claude.com/docs/en/hooks)
- [Claude Code — Permission modes](https://code.claude.com/docs/en/permission-modes)
- [Claude Code — Permissions](https://code.claude.com/docs/en/permissions)
- [Claude Code — Settings](https://code.claude.com/docs/en/settings)
- [Claude Code — Environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code — CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code — Plugins](https://code.claude.com/docs/en/plugins)
- [Claude Code — Plugin components reference](https://code.claude.com/docs/en/plugins-reference)
- [Claude Code — Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview)
- [Claude Code — Model configuration](https://code.claude.com/docs/en/model-config)
- [Claude Code — Agent view (background sessions)](https://code.claude.com/docs/en/agent-view)
- [Claude Code — Memory & CLAUDE.md](https://code.claude.com/docs/en/memory)
- [Claude Code product homepage](https://www.anthropic.com/claude-code)
