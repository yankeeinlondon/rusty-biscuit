---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://pi.dev/
docs: https://pi.dev/docs/latest/
subagent_docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/subagent/README.md

support: convention_only

locations:
  - os: macos
    scope: user
    path: ~/.pi/agent/agents/*.md
    notes: "User-scope subagent definitions consumed by the `subagent` example extension. Only read when that extension is installed (it is not in the default install). Default `agentScope: \"user\"`; loaded eagerly by `discoverAgents()`. Discovered via `path.join(getAgentDir(), \"agents\")`."
  - os: linux
    scope: user
    path: ~/.pi/agent/agents/*.md
    notes: "Same as macOS. `getAgentDir()` resolves `$PI_CODING_AGENT_DIR` first and falls back to `~/.pi/agent` on Linux."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\agents\\*.md"
    notes: "Same as macOS/Linux; backslash form. Resolved by `getAgentDir()` the same way; default `~/.pi/agent` on Windows is `%USERPROFILE%\\.pi\\agent`."
  - os: macos
    scope: repo
    path: .pi/agents/*.md
    notes: "Project-scope subagent definitions. Only loaded when the `subagent` extension is installed AND `agentScope` is `\"project\"` or `\"both\"`. Default scope is `\"user\"` for security. Resolved by `findNearestProjectAgentsDir()` which walks up from cwd to the filesystem root looking for the first `.pi/agents/` directory."
  - os: linux
    scope: repo
    path: .pi/agents/*.md
    notes: "Same as macOS; walk-up discovery."
  - os: windows
    scope: repo
    path: ".pi\\agents\\*.md"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: user
    path: ~/.pi/agent/presets.json
    notes: "User-scope named session presets for the `preset` example extension. Each top-level key is a preset name; values are JSON objects with `provider`, `model`, `thinkingLevel`, `tools`, `instructions`. Only read when the preset extension is installed."
  - os: linux
    scope: user
    path: ~/.pi/agent/presets.json
    notes: "Same as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\presets.json"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: repo
    path: .pi/presets.json
    notes: "Project-scope named session presets; merged with global presets with project keys winning (shallow `{ ...global, ...project }` merge in `loadPresets()`)."
  - os: linux
    scope: repo
    path: .pi/presets.json
    notes: "Same as macOS."
  - os: windows
    scope: repo
    path: ".pi\\presets.json"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: extension
    path: ~/.pi/agent/extensions/*/index.ts
    notes: "Global extensions. Subagent-style behavior is built via custom tools registered with `pi.registerTool({ name: \"subagent\", ... })`. Extensions are TypeScript modules loaded by jiti; they are not portable agent definition files."
  - os: linux
    scope: extension
    path: ~/.pi/agent/extensions/*/index.ts
    notes: "Same as macOS."
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.pi\\agent\\extensions\\*\\index.ts"
    notes: "Same as macOS/Linux; backslash form. Single-file extensions (`*.ts`) are also accepted; directory forms require an `index.ts` entry point."
  - os: macos
    scope: repo
    path: .pi/extensions/*/index.ts
    notes: "Project-scope extensions. Project extensions only load after `project_trust` resolves; trust is required for any `.pi/` content beyond settings.json."
  - os: linux
    scope: repo
    path: .pi/extensions/*/index.ts
    notes: "Same as macOS."
  - os: windows
    scope: repo
    path: ".pi\\extensions\\*\\index.ts"
    notes: "Same as macOS/Linux."

format:
  file_names:
    - "agents/*.md"
    - "agents/<name>.md"
    - "presets.json"
    - "extensions/*.ts"
    - "extensions/*/index.ts"
  frontmatter: true
  required_fields:
    - name (Markdown agent files; missing `name` or `description` skips the file in `loadAgentsFromDir`)
    - description (Markdown agent files)
  optional_fields:
    - "tools (Markdown agent files; comma-separated string parsed to string array, e.g. `read, grep, find, ls`; default is unset, child inherits parent's defaults)"
    - "model (Markdown agent files; passed as `--model <value>` to the spawned `pi` subprocess)"
    - "provider (presets.json; must combine with `model` to apply)"
    - "model (presets.json; must combine with `provider`)"
    - "thinkingLevel (presets.json; one of `off|minimal|low|medium|high|xhigh`)"
    - "tools (presets.json; array of tool names; replaces the active tool set when non-empty)"
    - "instructions (presets.json; appended to system prompt via `before_agent_start` hook)"
  body_format: markdown
  notes: |
    Agent markdown files are parsed by `parseFrontmatter()` from `@earendil-works/pi-coding-agent`; the YAML block is between `---` markers and the remainder is the body, which becomes the child subagent's system prompt. The body is written to a per-invocation temp file (`os.tmpdir()/pi-subagent-<name>/prompt-<name>.md`, mode 0600) and passed to the spawned `pi` subprocess via `--append-system-prompt <tmpfile>` so the child retains its default system prompt while appending the per-agent instructions. The body is otherwise plain Markdown — no further substitution, no template expansion, no field references.

    Presets are JSON objects in a single file. Tools in `presets.json` is an array of strings (e.g. `[\"read\", \"bash\", \"edit\", \"write\"]`); the preset extension filters out names not in `pi.getAllTools()` and warns about unknown entries. The active preset's name is persisted as a custom session entry (`{ type: \"custom\", customType: \"preset-state\", data: { name } }`) at every `turn_start`, restored at `session_start` so a `/resume` keeps the same preset.

    Extensions are TypeScript modules (default-exported function `export default function (pi: ExtensionAPI) { ... }`). They are not agent definitions themselves; they are runtime modules that register tools, commands, shortcuts, flags, providers, and lifecycle hooks via the `ExtensionAPI` surface.

runtime:
  invocation: |
    Pi has no built-in subagent feature. Delegation happens three documented ways, none of which ship enabled by default:

    1. **Tool call through the `subagent` extension** (the canonical reference implementation). The LLM invokes the custom tool registered by the extension as `subagent` with one of three shapes:
       - Single: `{ agent: \"<name>\", task: \"<text>\" }`
       - Parallel: `{ tasks: [{ agent, task, cwd? }, ...] }` (max 8 tasks, 4 concurrent; per-task output capped at 50 KB before going to the parent model)
       - Chain: `{ chain: [{ agent, task, cwd? }, ...] }` (sequential; `{previous}` placeholder in the `task` string is replaced by the prior step's final assistant text; chain stops at first failure with `{ isError: true }`)

       Additional parameters: `agentScope: \"user\" | \"project\" | \"both\"` (default `\"user\"`), `confirmProjectAgents: boolean` (default `true`; only effective when `ctx.hasUI`), and per-task `cwd`.

    2. **Preset extension `/preset` slash command** or `--preset <name>` CLI flag. The active preset reconfigures the current session's `model`, `thinkingLevel`, `tools`, and appends `instructions` to the system prompt — it does **not** spawn a child process.

    3. **External process orchestration** — the documented philosophy fallback: \"Spawn pi instances via tmux, or build your own with extensions, or install a package that does it your way.\" The `handoff` example extension demonstrates the latter by extracting conversation context into a generated prompt for a new focused session (`ctx.newSession({ parentSession, withSession })`).
  parent_child_context: |
    For the `subagent` extension, the child runs in a fresh `pi` subprocess launched with `spawn(command, [...args], { stdio: [\"ignore\", \"pipe\", \"pipe\"] })` and the following flag set: `--mode json -p --no-session`. The child therefore:
    - receives **no** inherited conversation history, parent tokens, or compaction state;
    - gets its own working directory (`cwd ?? defaultCwd`), its own model and tool set, and a system prompt that is the default `--system-prompt` plus the per-agent body appended via a temp file;
    - emits its own JSON-line event stream on stdout which the parent parses line-by-line (`message_end` and `tool_result_end` events are accumulated into the parent's tool result).

    What returns to the parent is the final assistant text content (last `AssistantMessage.text` block across the child's messages), collapsed for display and Markdown-rendered on expand. On failure the parent sees the child's `errorMessage`, `stderr`, or final text in that order. Parallel mode returns a `### [agent] status` summary per task (output truncated at 50 KB for model visibility; full output preserved in the tool's `details` JSON).
  permissions_inheritance: |
    Pi has no built-in permission system — extensions like `permission-gate.ts` demonstrate how to add one, but defaults are inherited from the host process. For the `subagent` extension:
    - The child inherits nothing about the parent's permission posture because it is a fresh subprocess with the same UID, environment, and filesystem access as the parent — there is no sandbox, capability mask, or approval state passed across the boundary.
    - The `confirmProjectAgents` parameter gates only the parent-side confirmation prompt when `agentScope` includes `\"project\"` (default `true`). Project agents are repo-controlled prompts that can instruct the model to read files and run bash; only continue for trusted repositories.
    - The plan-mode example extension does narrow tool access (disables `edit`/`write`; gates `bash` through an allowlist of read-only commands) but it operates inside the current session, not across a subprocess boundary.
    - The preset extension does not change permissions; it only swaps model, thinking level, and tool set.
  model_inheritance: |
    Subagent extension: the child's `--model` flag is set from the agent's `model` frontmatter when present; otherwise the child uses whatever the spawning `pi` process uses as its default model. There is no implicit inheritance of the parent's currently-selected model — it is whatever the parent's `defaultProvider`/`defaultModel` resolves to at spawn time (read from `~/.pi/agent/models.json` or settings).

    Preset extension: explicit — `provider` and `model` are applied via `pi.setModel()` only when **both** are set. Missing model triggers a warning notification and the change is skipped.
  tool_inheritance: |
    Subagent extension: the child receives `--tools <comma-separated list>` only when the agent frontmatter defines `tools`; otherwise it falls back to the parent's default tool set. There is no incremental allowlist/denylist semantics — the child's tools are either explicitly set by the agent definition or fully default.

    Preset extension: the preset's `tools` array is applied via `pi.setActiveTools(validTools)` after filtering against `pi.getAllTools()`; unknown tool names trigger a `ctx.ui.notify(\"Unknown tools: ...\", \"warning\")` and are dropped. The applied set replaces the previous active tool set, not adds to it.
  max_turns: "none documented"
  notes: |
    Concurrency: parallel mode in the subagent extension is hard-capped at `MAX_PARALLEL_TASKS = 8` and `MAX_CONCURRENCY = 4`; chain mode is inherently sequential. The child's tool calls inside its own subprocess are governed by pi's normal parallel tool execution defaults.

    Nesting: the subagent tool itself is not recursive — there is no documented mechanism for the child's spawned `pi` process to also invoke the `subagent` tool, because the child runs without extensions loaded (`--no-extensions` is not passed but the spawn path uses `--mode json -p --no-session` and the tool is only registered on the parent). In practice nesting requires installing and enabling the same extension in each level manually.

    Failure: exit code != 0, `stopReason` of `\"error\"`, or `\"aborted\"` (user Ctrl+C) are all surfaced as `{ isError: true }` on the parent's tool result. The parent kills the child subprocess with SIGTERM and a 5 s SIGKILL fallback when its `ctx.signal` aborts. Chain mode short-circuits on the first failure; parallel mode runs all tasks to completion and reports the success count in the result text.

    Discovery is repeated on every subagent tool call (`discoverAgents()` reads the directory fresh), so editing an agent file between calls is picked up without `/reload`. Tools themselves are cached in the parent's `pi.getAllTools()` registry until `/reload` (or `ctx.reload()`).

observability:
  stream_events:
    - "agent_start"
    - "agent_end"
    - "turn_start"
    - "turn_end"
    - "message_start"
    - "message_update"
    - "message_end"
    - "tool_execution_start"
    - "tool_execution_update"
    - "tool_execution_end"
    - "queue_update"
    - "compaction_start"
    - "compaction_end"
    - "auto_retry_start"
    - "auto_retry_end"
    - "(subagent tool is itself just a custom tool; the parent's stream sees one `tool_execution_*` cycle per invocation, NOT child lifecycle events)"
  hook_events:
    - "tool_call (fires for the parent's `subagent` tool call; extensions can block or mutate arguments before the child is spawned)"
    - "tool_result (fires for the parent's `subagent` tool result; extensions can patch the aggregated child output)"
    - "session_start (fires for the parent session; not for child sessions because they run with `--no-session`)"
    - "session_shutdown"
    - "(no `subagent_start` / `subagent_stop` hook event exists in the ExtensionAPI; child lifecycle is observable only via the parent's tool_execution_* cycle and the child subprocess's own stdout JSON stream captured by the extension)"
  session_ids: false
  notes: |
    The child subprocess writes no session file (`--no-session`), so there is no transcript on disk to tail and no stable child session ID — the only handle to a child is its exit code and its in-memory `SingleResult` aggregation in the parent's `tool_result.details`. The parent's own JSON event stream (`--mode json`) reflects the parent's lifecycle only; the child's lifecycle is invisible to it.

    The `subagent` extension captures the child's stdout JSON output into the tool's `details` payload, which is persisted on the parent's session as a tool-call result. A wrapper can re-derive the child's start/stop timing from `proc.spawn` time and `proc.on('close')` time but those are not part of the documented stream or hook surface.

    The preset extension's active name is persisted as a session entry (`customType: \"preset-state\"`) — a wrapper reading the parent's session JSONL can recover the active preset at any turn.

portability:
  portable: false
  non_portable_assets:
    - "frontmatter `tools:` values are pi tool names (`read`, `bash`, `edit`, `write`, `grep`, `find`, `ls`) — other providers use different identifier sets"
    - "frontmatter `model:` is passed verbatim as `--model <value>`; pi resolves it via its own `ModelRegistry`, so aliases like `claude-haiku-4-5` / `claude-sonnet-4-5` are pi-specific"
    - "the child is a real `pi` subprocess, so the agent's body runs through pi's full system prompt / tool plumbing / event stack — there is no portable \"agent runtime\" to target"
    - "the chain/parallel orchestration is implemented in the `subagent` extension source; agents that rely on `{previous}` placeholders or `tasks: [...]` arrays need to be rewritten for the target provider's delegation API"
    - "`presets.json` fields `thinkingLevel` and `instructions` are pi-specific session-prompt controls; the `--preset` flag and Ctrl+Shift+U shortcut are pi CLI surface"
    - "active-preset persistence uses a custom session entry (`customType: \"preset-state\"`) — other providers have no equivalent slot"
  rewrite_needed: true
  notes: |
    The agent body (Markdown system prompt) is the most portable asset — it can usually be lifted verbatim as long as it does not reference pi-specific tools, env vars (`PI_*`), or session files. The `name` and `description` frontmatter is provider-neutral routing metadata; it can carry across. Everything else (`tools`, `model`, the `{previous}` placeholder) requires a per-provider rewrite to map to that provider's tool identifiers, model aliases, and delegation API.

    The presets.json mechanism has no portable equivalent outside pi; remapping to another provider means rewriting into that provider's session-mode or prompt-template format (e.g. OpenCode `mode` blocks, Claude Code custom slash commands, Codex `agents/*.toml`).

cli_params:
  - flag: --extension / -e <source>
    description: "Load an extension from a path, npm, or git source. The `subagent`, `preset`, `plan-mode`, and `handoff` examples are all loaded this way (or via auto-discovery from `~/.pi/agent/extensions/` and `.pi/extensions/`). Repeatable."
    example: "pi -e examples/extensions/subagent/index.ts"
  - flag: --no-extensions
    description: "Disable extension auto-discovery from the global and project extension directories. Equivalent to omitting the auto-discovery layer but does not affect explicit `-e <path>` flags."
    example: "pi --no-extensions"
  - flag: --no-context-files / -nc
    description: "Disable AGENTS.md / CLAUDE.md / SYSTEM.md discovery from `~/.pi/agent/AGENTS.md` and the walk-up directory chain. Not directly related to agents but commonly paired with subagent tests because context-file content is appended to the child's system prompt via `--append-system-prompt`."
    example: "pi --no-context-files"
  - flag: --no-builtin-tools / -nbt
    description: "Disable the four built-in tools (`read`, `bash`, `edit`, `write`) by default; extension-registered and custom tools remain enabled. Useful when an agent definition needs a stripped-down tool set without writing the `tools:` list explicitly."
    example: "pi --no-builtin-tools"
  - flag: --no-tools / -nt
    description: "Disable all tools (built-in, extension, and custom) by default. Agents can still re-enable specific tools via the `tools:` frontmatter which is passed as `--tools t1,t2,t3` to the child subprocess."
    example: "pi --no-tools"
  - flag: --preset <name>
    description: "Preset name registered by an installed preset extension. Activates the preset at session start. Only meaningful when the preset extension is loaded."
    example: "pi --preset implement"
  - flag: --plan
    description: "Start in plan mode (registered by the plan-mode example extension). Disables edit/write tools and applies the read-only bash allowlist. Only meaningful when the plan-mode extension is loaded."
    example: "pi --plan"
  - flag: --no-session
    description: "Ephemeral mode: do not save the session to `~/.pi/agent/sessions/`. The subagent extension always passes this to child subprocesses so child runs do not pollute the session list."
    example: "pi --no-session -p \"one-off task\""
  - flag: -p / --print
    description: "Print mode: respond and exit without entering the interactive TUI. The subagent extension always passes `-p` to child subprocesses (combined with `--mode json`) so each child is a one-shot print invocation that streams structured events."
    example: "pi -p \"summarize\""
  - flag: --mode json
    description: "Output every session event as a JSON line on stdout (see https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md). The subagent extension's child subprocesses always run with this mode so their lifecycle can be parsed back into the parent's tool result."
    example: "pi --mode json \"task\""
  - flag: --tools <list> / -t <list>
    description: "Allowlist of tool names (built-in, extension, and custom). Passed verbatim to the child subprocess by the subagent extension when the agent definition specifies a `tools:` list."
    example: "pi --tools read,grep,find,ls -p \"recon\""
  - flag: --model <pattern>
    description: "Provider/model pattern (supports `provider/id` form and optional `:<thinking>` suffix). Passed verbatim to the child subprocess when the agent definition specifies a `model:` field."
    example: "pi --model anthropic/claude-haiku-4-5 -p \"task\""
  - flag: --append-system-prompt <text>
    description: "Append text to the default system prompt. The subagent extension writes each agent's body to a temp file and passes the file path here so the child retains pi's defaults plus the agent's instructions."
    example: "pi --append-system-prompt /tmp/pi-subagent-scout/prompt-scout.md"
  - flag: --provider / --api-key / --thinking / --list-models / --models
    description: "Model selection flags honored by the parent session; the subagent extension does not pass these to the child, so the child resolves its own provider/model from `--model` and the active `~/.pi/agent/models.json`."
    example: "pi --provider anthropic --model claude-sonnet-4-5 --thinking high"
  - flag: --add-dir / --session / --fork / --resume / --session-dir / -c / -r
    description: "Session control flags. None of them are passed to child subagent subprocesses; the child always gets `--no-session` and a fresh working directory."
    example: "pi -c  # continue the most recent session"

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: "Override the entire agent config directory (default `~/.pi/agent`). Honored by `getAgentDir()`; changes where `agents/`, `extensions/`, `presets.json`, `prompts/`, `themes/`, `skills/`, `sessions/`, `models.json`, and `settings.json` are resolved. Useful when packaging or sandboxing pi into a per-project bundle."
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: "Override the session storage directory (default `~/.pi/agent/sessions`). Child subagent subprocesses always use `--no-session` so they never write here regardless of this override."
  - name: PI_PACKAGE_DIR
    effect: "Override the package installation directory (useful for Nix/Guix where store paths tokenize poorly)."
  - name: PI_OFFLINE
    effect: "Disable all startup network operations, including update checks, package update checks, and install/update telemetry. Does not affect the subagent extension's local subprocess spawn."
  - name: PI_SKIP_VERSION_CHECK
    effect: "Skip the `https://pi.dev/api/latest-version` startup check only; does not disable telemetry or package update checks."
  - name: PI_TELEMETRY
    effect: "Override install/update telemetry (`1/true/yes` enables; `0/false/no` disables)."
  - name: PI_CACHE_RETENTION
    effect: "`long` enables extended prompt cache (Anthropic: 1 h, OpenAI: 24 h). Useful for repeating the same agent definition body across many invocations."
  - name: PI_ALLOW_LOCKFILE_CHANGE
    effect: "Pre-commit hook override that allows accidental lockfile commits; not relevant to agent loading but listed because it appears in the same supply-chain hardening section of the repo README."

changes: []

requires_claudine_update: true
reason: |
  Pi is research-only — `claudine/docs/providers.yaml` has no Pi entry yet, the `Provider` enum in `claudine/provider_id.rs` does not include it, and there is no `claudine pi` wrapper or `claudine agents` discovery path for `~/.pi/agent/agents/*.md`. If Claudine decides to wire Pi support, three things need to land:

  1. **Agent linking must be conditional on extension presence.** Claudine's `linking` module cannot just symlink `~/.pi/agent/agents/*.md` and expect them to work — the files are inert until the user installs the `subagent` extension (either via the README's symlink steps or by installing a third-party pi package that depends on it). The linker should emit a soft warning when an agent is copied without the corresponding extension being enabled, or pair the copy with the extension install command.

  2. **Runtime delegation semantics are extension-shaped, not protocol-shaped.** There is no first-class `claudine pi subagent <name>` analog. A wrapper targeting Pi must either invoke the `subagent` tool through a normal interactive or print session (relying on the LLM to call the tool) or shell out to a fresh `pi --mode json -p --no-session --model ... --tools ... --append-system-prompt <tmpfile>` process mirroring the extension's spawn path. The latter matches what the extension does and avoids requiring the extension to be installed in the wrapper's session.

  3. **No `resume` / `proxy` affordance for child subagents.** Because the child runs with `--no-session`, there is no child session file to resume and no stable child session ID. Claudine's lifecycle `proxy` action cannot forward a hook to a child; the wrapper must track child PIDs if it wants to forward signals, and only via the parent's `tool_call`/`tool_result` cycle.

  4. **Portability for body prompts is high, but frontmatter needs full rewrite.** A linked agent file's `tools:` list and `model:` field must be remapped to the destination provider's vocabulary. Linking Claude Code's `tools: Read, Glob, Grep, Bash` directly into `~/.pi/agent/agents/<name>.md` would crash the child at spawn because pi's tool registry has no `Read`, `Glob`, or `Grep` names (pi uses `read`, `grep`, `find`, `ls`).

  Also, `presets.json` is the closest analogue to a "mode" file in pi and could be linked to OpenCode's `mode` blocks or Claude Code's custom slash commands, but the field set (`thinkingLevel`, `instructions` appended via `before_agent_start`) does not map 1:1 to either target.
---

# Pi Subagents

## Overview

Pi is a minimal terminal coding harness that ships with no built-in agent, subagent, mode, persona, or worker feature. The README is explicit: **"No sub-agents. There's many ways to do this. Spawn pi instances via tmux, or build your own with [extensions](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md), or install a package that does it your way."** The README's customization section lists Skills, Prompt Templates, Extensions, Themes, and Pi Packages, and the only "agent-like" affordances are example extensions and JSON config files that ship in `packages/coding-agent/examples/extensions/` and require manual installation.

Because user-defined agent behavior must be built on top of pi's extension API, support is classified as `convention_only`. Three documented patterns can act as agent definitions Claudine could link:

| Pattern | Definition location | Requires | Span |
|---|---|---|---|
| `subagent` extension (Markdown files + tool) | `~/.pi/agent/agents/*.md`, `.pi/agents/*.md` | Subagent extension installed | Cross-process (spawns `pi` subprocesses) |
| `preset` extension (named JSON session configs) | `~/.pi/agent/presets.json`, `.pi/presets.json` | Preset extension installed | In-session (swaps model/thinking/tools/system prompt) |
| `plan-mode` extension (read-only mode + UI widget) | none — extracted from `Plan:` headers | Plan-mode extension installed | In-session (gates tools and bash commands) |

This topic covers all three because they are the documented reusable persona/mode files the Claudine agent linker can target, and they share the same parent-process / subprocess semantics that drive runtime delegation. None of them are first-class in pi's core; the schemas and paths below are derived from the source under `packages/coding-agent/examples/extensions/`.

## Locations

Pi's config root is `~/.pi/agent/` on every platform, resolved by `getAgentDir()` from `@earendil-works/pi-coding-agent`. The `$PI_CODING_AGENT_DIR` env var redirects it. The agent-related directories inside that root are:

| Scope | macOS / Linux | Windows | Discovery |
|---|---|---|---|
| User agents | `~/.pi/agent/agents/*.md` | `%USERPROFILE%\.pi\agent\agents\*.md` | Loaded by `subagent` extension via `path.join(getAgentDir(), \"agents\")` |
| Project agents | `<cwd>/.pi/agents/*.md` (walking up) | `<cwd>\.pi\agents\*.md` | Walk-up via `findNearestProjectAgentsDir()` |
| User presets | `~/.pi/agent/presets.json` | `%USERPROFILE%\.pi\agent\presets.json` | Loaded by `preset` extension at `session_start` |
| Project presets | `<cwd>/.pi/presets.json` | `<cwd>\.pi\presets.json` | Loaded by `preset` extension at `session_start` |
| User extensions | `~/.pi/agent/extensions/*.ts` or `*/index.ts` | `%USERPROFILE%\.pi\agent\extensions\*.ts` | Auto-discovered unless `--no-extensions` |
| Project extensions | `<cwd>/.pi/extensions/*.ts` or `*/index.ts` | `<cwd>\.pi\extensions\*.ts` | Auto-discovered only after `project_trust` resolves |

On this host (macOS), observed: `~/.pi/agent/` contains `auth.json`, `models.json`, `settings.json`, plus `sessions/` and timestamped backups of the JSON files. There is **no** `agents/`, `extensions/`, `presets.json`, or `prompts/` directory present, and no subagent, preset, or plan-mode extension is installed. The default `pi` install (0.73.1, located at `/Users/ken/.bun/bin/pi`) ships the example extensions only inside the npm tarball under `examples/extensions/`; they are not symlinked into the user's config directory.

The `~/.pi/agent/settings.json` on this host contains:

```json
{
  "lastChangelogVersion": "0.73.0",
  "defaultProvider": "omlx",
  "defaultModel": "Qwen3.6-35B-A3B-oQ6"
}
```

`defaultProvider` and `defaultModel` are the values the subagent extension's child subprocess picks up when no `model:` frontmatter is set on the agent definition. None of these fields are agent-definition locations themselves — they configure the runtime the children run in.

## Definition Format

### Agent markdown files (`*.md` under `agents/`)

A subagent agent file is a single Markdown file with YAML frontmatter between `---` markers. Parsed by `parseFrontmatter()` from `@earendil-works/pi-coding-agent`. The body becomes the child subagent's system prompt.

```markdown
---
name: scout
description: Fast codebase recon that returns compressed context for handoff to other agents
tools: read, grep, find, ls, bash
model: claude-haiku-4-5
---

You are a scout. Quickly investigate a codebase and return structured findings
that another agent can use without re-reading everything.

## Files Retrieved
1. `path/to/file.ts` (lines 10-50) - Description of what's here
2. ...

## Key Code
...
```

Recognized frontmatter fields:

- **Required**: `name` (becomes the agent identifier used in `subagent` tool calls), `description` (routing signal).
- **Optional**: `tools` (comma-separated string parsed to a `string[]`; passed as `--tools t1,t2,...` to the child), `model` (passed verbatim as `--model <value>` to the child).

Files missing either `name` or `description` are silently skipped by `loadAgentsFromDir()`. Filename and folder paths do not contribute to identity — the `name` field is the only key. Directory scanning is non-recursive: the extension reads only the immediate `agents/*.md` entries, not nested folders.

### Preset JSON files (`presets.json`)

```json
{
  "plan": {
    "provider": "anthropic",
    "model": "claude-sonnet-4-5",
    "thinkingLevel": "high",
    "tools": ["read", "bash", "edit", "write"],
    "instructions": "You are in PLAN MODE. Your job is to deeply understand the problem..."
  },
  "implement": {
    "provider": "anthropic",
    "model": "claude-sonnet-4-5",
    "thinkingLevel": "high",
    "tools": ["read", "bash", "edit", "write"],
    "instructions": "You are in IMPLEMENTATION MODE. Your job is to make focused, correct changes..."
  }
}
```

Each top-level key is a preset name (referenced by `--preset <name>` or `/preset <name>`). Recognized fields:

- **Optional**: `provider` (must combine with `model`), `model` (must combine with `provider`), `thinkingLevel` (`off|minimal|low|medium|high|xhigh`), `tools` (string array; replaces the active tool set when non-empty), `instructions` (appended to the system prompt at `before_agent_start`).

There is no preset-level frontmatter or description; presets are activated by name. The active preset name is persisted to the session as `{ type: \"custom\", customType: \"preset-state\", data: { name } }` at every `turn_start`, restored at `session_start`. The `(none)` pseudo-preset clears the active preset and restores the original model/thinking/tools snapshot.

### Extension TypeScript modules (`*.ts` under `extensions/`)

Extensions are TypeScript modules loaded by jiti. They are not agent definitions per se — they are the runtime modules that register custom tools, commands, flags, and lifecycle hooks. The agent definition (Markdown file) only becomes useful after the extension that consumes it is installed. The minimum shape:

```typescript
import type { ExtensionAPI } from \"@earendil-works/pi-coding-agent\";
import { Type } from \"typebox\";

export default function (pi: ExtensionAPI) {
  pi.registerTool({
    name: \"subagent\",
    label: \"Subagent\",
    description: \"Delegate tasks to specialized subagents with isolated context.\",
    parameters: Type.Object({ /* schema */ }),
    async execute(_toolCallId, params, signal, onUpdate, ctx) {
      // spawn child pi subprocess, parse JSON output, return aggregated result
    },
  });
}
```

Extensions can also register commands (`pi.registerCommand(\"preset\", ...)`), shortcuts (`pi.registerShortcut(...)`), flags (`pi.registerFlag(\"preset\", ...)`), providers (`pi.registerProvider(...)`), and subscribe to lifecycle events (`pi.on(\"before_agent_start\", ...)` etc.).

## Runtime Behavior

### `subagent` extension (cross-process delegation)

The LLM in the parent session calls the registered `subagent` tool with one of three shapes:

- **Single**: `{ agent: \"<name>\", task: \"<text>\" }` — spawns one child and returns its final assistant text or error.
- **Parallel**: `{ tasks: [{ agent, task, cwd? }, ...] }` — spawns up to 8 children with at most 4 in flight at any time. Returns a `### [agent] status` summary per task; per-task model-visible output is capped at 50 KB but the full output remains in the tool `details` JSON.
- **Chain**: `{ chain: [{ agent, task, cwd? }, ...] }` — sequential; `{previous}` placeholder in the `task` string is replaced by the prior step's final assistant text. Stops at the first failure with `{ isError: true }`.

Optional flags:

- `agentScope: \"user\" | \"project\" | \"both\"` (default `\"user\"`) — controls whether project-local agents are visible.
- `confirmProjectAgents: boolean` (default `true`) — when `agentScope` includes `\"project\"` and `ctx.hasUI` is true, the parent shows a confirmation dialog before running any project-scope agent.
- `cwd` — per-task working directory override.

The child is spawned via `node:child_process.spawn(command, [...args], { stdio: [\"ignore\", \"pipe\", \"pipe\"] })`. The arg list is constructed as:

```
--mode json -p --no-session [--model <agent.model>] [--tools <agent.tools>] [--append-system-prompt <tmpfile>] "Task: <task>"
```

The system prompt temp file is written by `writePromptToTempFile(agent.name, agent.systemPrompt)` to `os.tmpdir()/pi-subagent-<safeName>/prompt-<safeName>.md` with mode 0600 and removed in the `finally` block. The extension reads the child's stdout as LF-delimited JSON lines and accumulates `message_end` and `tool_result_end` events into the parent's tool `details`.

The child inherits nothing about the parent except:

- the same UID, environment, and filesystem access (no sandbox or capability boundary);
- the parent's default provider/model when no `model:` is set on the agent definition (resolved via `~/.pi/agent/models.json` `defaultProvider`/`defaultModel`);
- the parent's default tool set when no `tools:` is set on the agent definition.

What returns to the parent:

- **Single mode success**: the final assistant text, or `"(no output)"` if the child produced no text.
- **Single mode failure**: `errorMessage`, then `stderr`, then final text. Tool result has `isError: true`.
- **Parallel mode**: a text block of the form `Parallel: N/M succeeded\n\n### [agent] status\n\n<output>\n\n---\n\n...`. Each task's output is truncated at 50 KB for the model-visible text; the full output is preserved under `details.results[i].messages`.
- **Chain mode**: the final step's output, or `Chain stopped at step N (agent): <error>` on failure.

The extension renders the result in the TUI with a custom `renderResult` that shows ✓/✗/⏳ status icons, the last 10 items (collapsible, Ctrl+O to expand all), Markdown rendering of the final text, and per-task usage stats (`turns ↑input ↓output RcacheRead WcacheWrite $cost ctx:contextTokens model`).

Ctrl+C in the parent triggers the child's `AbortSignal`; the extension sends `SIGTERM` to the child PID, then `SIGKILL` after 5 s if the child is still alive.

### `preset` extension (in-session mode switch)

The active preset reconfigures the current session, not a child. At `session_start`, presets are loaded from both `~/.pi/agent/presets.json` and `<cwd>/.pi/presets.json` (project overrides global with a shallow merge). If the `--preset <name>` flag was passed, the preset is applied immediately. On resume, the persisted `preset-state` custom entry is restored without re-applying model/tools (so a resumed session keeps the same preset label even though the model selection is whatever the user last picked).

Applying a preset calls, in order:

1. `pi.setModel(model)` (when both `provider` and `model` resolve in `ctx.modelRegistry`) — warns via `ctx.ui.notify(\"Preset ...: No API key for ...\")` if the model has no key.
2. `pi.setThinkingLevel(preset.thinkingLevel)` — only when defined.
3. `pi.setActiveTools(validTools)` — only when `preset.tools` is non-empty; unknown tool names trigger a warning and are dropped.
4. The active preset's `instructions` are appended to the system prompt via the `before_agent_start` hook (`return { systemPrompt: event.systemPrompt + \"\\n\\n\" + preset.instructions }`).
5. A footer status indicator `preset:<name>` is set via `ctx.ui.setStatus(\"preset\", ...)`.
6. The preset name is persisted to the session at every `turn_start`.

`/preset` with no argument shows a themed selector; `/preset <name>` applies directly. Ctrl+Shift+U cycles through `(none)` and the defined presets. Choosing `(none)` clears the active preset and restores the snapshot taken on first activation.

### `plan-mode` extension (in-session mode gate)

Plan mode is the closest pi gets to a built-in "mode". `/plan` toggles between read-only and normal tool sets:

- **Plan mode active**: `read`, `bash` (allowlist only), `grep`, `find`, `ls`, `questionnaire`; `edit` and `write` are disabled; bash is filtered through `isSafeCommand()` (an allowlist of read-only commands).
- **Normal mode**: `read`, `bash`, `edit`, `write` plus whatever else was active.
- **Execution mode** (after the user accepts a plan): full tool access; assistant is expected to emit `[DONE:n]` markers; a footer widget shows progress (`📋 completed/total`); completion sends a `plan-complete` message and clears state.

State is persisted as `{ customType: \"plan-mode\", enabled, todos, executing, toolsBeforePlanMode }` and restored at `session_start`. On resume, messages after the last `plan-mode-execute` entry are re-scanned for `[DONE:n]` markers to rebuild completion state.

## Observability

Pi has no built-in hook or stream event named `subagent_start` or `subagent_stop`. The subagent extension is just a custom tool, so the parent's surface sees only one `tool_execution_*` cycle per invocation. The child's lifecycle is invisible to the parent's `--mode json` stream and to all `ExtensionAPI` hooks.

The full set of `AgentSessionEvent` types emitted by `pi --mode json` is:

| Event | Payload | Notes |
|---|---|---|
| `session` | `{ version, id, timestamp, cwd }` | First line; one per session |
| `agent_start` | `{}` | One per user prompt |
| `agent_end` | `{ messages }` | One per user prompt |
| `turn_start` | `{ turnIndex, timestamp }` | One per LLM response cycle |
| `turn_end` | `{ turnIndex, message, toolResults }` | One per LLM response cycle |
| `message_start` | `{ message }` | user/assistant/toolResult |
| `message_update` | `{ message, assistantMessageEvent }` | Streaming assistant deltas |
| `message_end` | `{ message }` | Finalized message |
| `tool_execution_start` | `{ toolCallId, toolName, args }` | One per tool call |
| `tool_execution_update` | `{ toolCallId, toolName, args, partialResult }` | Streaming progress |
| `tool_execution_end` | `{ toolCallId, toolName, result, isError }` | Tool call result |
| `queue_update` | `{ steering, followUp }` | Pending message queues |
| `compaction_start` | `{ reason }` | `manual` / `threshold` / `overflow` |
| `compaction_end` | `{ reason, result, aborted, willRetry, errorMessage? }` | Compaction finished |
| `auto_retry_start` | `{ attempt, maxAttempts, delayMs, errorMessage }` | Provider auto-retry |
| `auto_retry_end` | `{ success, attempt, finalError? }` | Provider auto-retry finished |

Extension hook events that can observe subagent-adjacent activity:

- `before_agent_start`, `agent_start`, `agent_end` — fires once per user prompt on the parent. Use `before_agent_start` to inject context (the `subagent` extension itself does not subscribe to these).
- `turn_start`, `turn_end` — fires per LLM turn.
- `tool_call`, `tool_result`, `tool_execution_start`, `tool_execution_update`, `tool_execution_end` — fires per tool call. The `subagent` tool's `tool_call` event carries the agent name(s) in `event.input`; the `tool_result` event carries the aggregated child output.
- `session_start`, `session_shutdown`, `session_info_changed`, `session_before_switch`, `session_before_fork`, `session_before_compact`, `session_compact`, `session_before_tree`, `session_tree` — session lifecycle.

There is no stable child session ID. The child runs with `--no-session`, so no session JSONL is written, and the parent's session records only the `subagent` tool call's aggregated result. A wrapper can re-derive approximate child start/stop times from `proc.spawn` time and the `tool_execution_start` / `tool_execution_end` events on the parent, but those events are scoped to the parent's tool registry, not the child's.

The active preset's name is recoverable from the parent's session JSONL by reading entries with `type === \"custom\" && customType === \"preset-state\"`. The active plan-mode state is recoverable from entries with `customType === \"plan-mode\"`.

## Portability

The agent body (Markdown system prompt) is the most portable asset and usually lifts verbatim as long as it does not reference pi-specific tool names (`read`, `grep`, `find`, `ls`), env vars (`PI_*`), or session files. The `name` and `description` frontmatter fields are provider-neutral routing metadata and can carry across. Everything else requires a per-provider rewrite:

| Asset | Portable as-is? | Rewrite target |
|---|---|---|
| Body prompt | partial | Strip pi tool references; rewrite provider-specific env / file paths |
| `name` | yes | Remap identifier shape if the target has rules (lowercase letters + hyphens etc.) |
| `description` | yes | Carries as routing signal |
| `tools:` | no | Remap to target provider's tool identifiers (e.g. `read` → `Read`, `grep` → `Grep`, `find` → `Glob`, `ls` → `LS`) |
| `model:` | no | Remap to target provider's model aliases (e.g. `claude-haiku-4-5` → `haiku`) |
| Chain / parallel orchestration (`chain: [...]`, `tasks: [...]`) | no | Rewrite into target provider's delegation API (Task tool, Agent tool, etc.) |
| `{previous}` placeholder | no | Rewrite into target provider's chain mechanism |

Linking Claude Code's `tools: Read, Glob, Grep, Bash` into `~/.pi/agent/agents/<name>.md` would crash the child at spawn because pi's tool registry has no `Read`, `Glob`, or `Grep` names — pi uses `read`, `grep`, `find`, `ls`. Similarly, pi's `model: claude-haiku-4-5` is passed verbatim as `--model claude-haiku-4-5`, which pi's `ModelRegistry.find(\"anthropic\", \"claude-haiku-4-5\")` resolves; other providers expect their own identifier forms.

`presets.json` has no portable equivalent outside pi. The `thinkingLevel` enum is pi's session setting, and the `instructions` field is appended to the system prompt via pi's `before_agent_start` hook — both are pi-specific. Remapping to OpenCode's `mode` blocks or Claude Code's custom slash commands requires a structural rewrite.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy`/`resume` actions:

- **Pi is not yet wired.** There is no `pi` entry in `claudine/docs/providers.yaml`, no `Provider::Pi` variant in `claudine/provider_id.rs`, and no `claudine pi` wrapper or subagent discovery command. Pi is research-only at this stage. Any linking rule must be opt-in via an extension-presence check, not a default `~/.pi/agent/agents/*.md` walk.

- **Agent definition discovery is conditional on extension installation.** Claudine's agent linker cannot assume `~/.pi/agent/agents/*.md` is meaningful — the files are inert without the `subagent` extension installed. The linker should either (a) check for the extension by looking at `~/.pi/agent/extensions/*/index.ts` or the active `settings.json` packages list, (b) install the extension automatically as part of the copy, or (c) copy the agent files together with the extension source and emit a clear "you also need to symlink the subagent extension" hint. Recommended: pair the copy with the symlink steps from the extension README.

- **Runtime delegation has no first-class wrapper command.** `claudine pi subagent <name>` has no direct analog. A wrapper has two choices: (a) run an interactive `pi` session that depends on the LLM calling the `subagent` tool on its own, or (b) shell out to a one-shot `pi --mode json -p --no-session --model <m> --tools <t> --append-system-prompt <tmpfile> "Task: ..."` subprocess, mirroring the extension's spawn path. Option (b) avoids requiring the extension to be installed in the wrapper's session and matches the documented semantics exactly.

- **No `resume` / `proxy` affordance for child subagents.** Children run with `--no-session`, so there is no child session JSONL to resume and no stable child session ID. A wrapper that wants to forward a hook or send follow-up instructions to a child must track child PIDs in memory (the parent extension does this only for the duration of the tool call) and can only forward signals via `proc.kill()` — there is no `SendMessage` analog. The child's transcript survives only as the aggregated `details` payload on the parent's `tool_result` for that one tool call.

- **Active preset is recoverable.** For wrappers that want to know "which preset is active?", the answer is the last `customType: \"preset-state\"` entry in the parent's session JSONL. The plan-mode state is similarly recoverable as the last `customType: \"plan-mode\"` entry.

- **Body prompts are mostly portable.** A linked Claude Code `code-reviewer.md` body can usually drop into `~/.pi/agent/agents/code-reviewer.md` with no rewrites, but the `tools:` list must be remapped to pi's vocabulary and any `${CLAUDE_*}` env var references must be rewritten to `${PI_*}` or stripped. Linking is recommended for body-only content; frontmatter-level linking should be treated as a rewrite-needed case.

## Sources

- [Pi — homepage](https://pi.dev/)
- [Pi — documentation root](https://pi.dev/docs/latest/)
- [Pi — package README on GitHub](https://github.com/earendil-works/pi)
- [Pi — README on npm (@earendil-works/pi-coding-agent)](https://www.npmjs.com/package/@earendil-works/pi-coding-agent)
- [Pi — extensions documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md)
- [Pi — JSON event stream mode](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md)
- [Pi — settings documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/settings.md)
- [Pi — sessions documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sessions.md)
- [Pi — extension examples index](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions)
- [Pi — subagent extension README](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/subagent/README.md)
- [Pi — subagent extension entry point (`index.ts`)](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/subagent/index.ts)
- [Pi — subagent agent discovery (`agents.ts`)](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/subagent/agents.ts)
- [Pi — sample agent definitions (`agents/scout.md`, `agents/worker.md`, etc.)](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/subagent/agents)
- [Pi — sample workflow prompts (`prompts/implement.md`, etc.)](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/subagent/prompts)
- [Pi — preset extension source](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/preset.ts)
- [Pi — plan-mode extension README](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/plan-mode/README.md)
- [Pi — plan-mode extension entry point](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/plan-mode/index.ts)
- [Pi — handoff extension source](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/handoff.ts)
- [Pi — philosophy blog post (\"What if you don't need MCP?\")](https://mariozechner.at/posts/2025-11-02-what-if-you-dont-need-mcp/)
- [Pi — \"Pi coding agent\" launch post](https://mariozechner.at/posts/2025-11-30-pi-coding-agent/)