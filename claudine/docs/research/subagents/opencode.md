---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://opencode.ai/
docs: https://opencode.ai/docs/
subagent_docs: https://opencode.ai/docs/agents/

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.config/opencode/agents/<name>.md
    notes: "Global user-scope agent definitions. Markdown files in `~/.config/opencode/agents/`. The filename stem becomes the agent name. Singular `agent/` is also accepted for backwards compatibility. Symlinks to files outside this directory are followed (observed on host: entries in this directory are symlinks to `~/.claude/agents/*.md`)."
  - os: linux
    scope: user
    path: ~/.config/opencode/agents/<name>.md
    notes: "Global user-scope agent definitions; same shape as macOS. Resolved under the XDG config root, which `OPENCODE_CONFIG_DIR` can override."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\agents\\<name>.md"
    notes: "Global user-scope agent definitions on Windows; backslash path form, otherwise identical to macOS/Linux."
  - os: macos
    scope: repo
    path: .opencode/agents/<name>.md
    notes: "Per-project agent directory. Every `.opencode/` directory from CWD up to the git worktree is scanned; singular `.opencode/agent/` is also accepted."
  - os: linux
    scope: repo
    path: .opencode/agents/<name>.md
    notes: "Per-project agent directory. Same walk-up discovery as macOS."
  - os: windows
    scope: repo
    path: ".opencode\\agents\\<name>.md"
    notes: "Per-project agent directory on Windows."
  - os: macos
    scope: other
    path: opencode.json (or opencode.jsonc) `agent.<name>` key
    notes: "Inline JSON agent definitions inside the config file. The same schema (`AgentConfig`) is shared with the Markdown form; key name becomes agent name. Project, global, custom (`OPENCODE_CONFIG`), and inline (`OPENCODE_CONFIG_CONTENT`) configs all accept `agent` blocks and are merged together."
  - os: linux
    scope: other
    path: opencode.json (or opencode.jsonc) `agent.<name>` key
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: "opencode.json (or opencode.jsonc) `agent.<name>` key"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: other
    path: $OPENCODE_CONFIG_DIR/agents/<name>.md
    notes: "Custom config directory. Set via `OPENCODE_CONFIG_DIR`; searched for `agents/`, `commands/`, `modes/`, and `plugins/` like a standard `.opencode` directory. Loaded after global and standard project config directories."
  - os: linux
    scope: other
    path: $OPENCODE_CONFIG_DIR/agents/<name>.md
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: "$OPENCODE_CONFIG_DIR\\agents\\<name>.md"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: system
    path: /Library/Application Support/opencode/opencode.json
    notes: "macOS managed-config directory. Admin-controlled `opencode.json` or `opencode.jsonc` dropped here; same precedence as other managed config sources. Higher than `OPENCODE_CONFIG_CONTENT` inline overlays."
  - os: linux
    scope: system
    path: /etc/opencode/opencode.json
    notes: "Linux managed-config directory. Admin-controlled file; highest config precedence."
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.json"
    notes: "Windows managed-config directory. Admin-controlled file; highest config precedence."
  - os: macos
    scope: system
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    notes: "macOS MDM-deployed preferences. `PayloadType=ai.opencode.managed` is the dedicated channel; payload keys map directly to `opencode.json` fields. Same precedence as the file-based managed config."
  - os: linux
    scope: system
    path: .well-known/opencode
    notes: "Remote organizational config fetched automatically when authenticating with a provider that supports it. Loaded first in the config precedence order; everything else can override it."
  - os: windows
    scope: system
    path: .well-known/opencode
    notes: "Remote organizational config; same behavior as Linux."
  - os: macos
    scope: extension
    path: <plugin>/agents/<name>.md (or plugin-contributed `agent` config)
    notes: "Plugins can contribute agents through the V2 plugin API. Bundled default plugins ship extra agents at startup (toggle with `OPENCODE_DISABLE_DEFAULT_PLUGINS`); user-installed plugins contribute agents at load time."
  - os: linux
    scope: extension
    path: <plugin>/agents/<name>.md (or plugin-contributed `agent` config)
    notes: "Same as macOS."
  - os: windows
    scope: extension
    path: "<plugin>\\agents\\<name>.md"
    notes: "Same as macOS/Linux."

format:
  file_names:
    - "<name>.md (file basename becomes the agent name)"
    - "opencode.json / opencode.jsonc `agent.<name>` (JSON key becomes the agent name)"
  frontmatter: true
  required_fields:
    - "description (routing signal; required by the OpenCode loader, per the official agents doc)"
  optional_fields:
    - "mode (`primary` | `subagent` | `all`; defaults to `all`)"
    - "model (`provider/model`; overrides the agent's default model when set)"
    - "prompt (`{file:./prompts/build.txt}` shorthand OR inline text OR the Markdown body for `.md` form)"
    - "temperature (0.0-1.0)"
    - "top_p (0.0-1.0)"
    - "variant (provider-specific reasoning effort, paired with `model`)"
    - "tools (legacy boolean map; deprecated in v1.1.1, use `permission` instead)"
    - "permission (object: `edit`/`bash`/`webfetch`/`doom_loop`/`external_directory`/`websearch`/`skill`/`question`/`read`/`glob`/`grep`/`list`/`task`/`todowrite`/`lsp`; each value is `\"allow\" | \"ask\" | \"deny\"` or a glob→action map; `bash` and `read`/`edit`/`glob`/`grep` accept the object form for pattern matching)"
    - "disable (`true` to skip the agent entirely)"
    - "hidden (`true` to remove from `@` autocomplete; only meaningful when `mode: subagent`)"
    - "color (hex `#RRGGBB` or theme name: `primary`/`secondary`/`accent`/`success`/`warning`/`error`/`info`)"
    - "steps (max agentic iterations before forcing text-only response; replaces legacy `maxSteps`)"
    - "options (object passed straight through to the provider as model options)"
  body_format: markdown
  notes: |
    Two equivalent declaration surfaces:
    (1) Markdown files with YAML frontmatter. Frontmatter carries the AgentConfig schema; the body becomes the system prompt for the Markdown-form agent (in place of a `prompt:` field). The filename stem becomes the agent name, e.g. `~/.config/opencode/agents/review.md` registers `review`.
    (2) JSON entries under the `agent` key in `opencode.json` / `opencode.jsonc` (and via `OPENCODE_CONFIG_CONTENT`). The JSON key becomes the agent name; the `prompt` field carries the system-prompt text inline or via `{file:./prompts/build.txt}` shorthand. The same `AgentConfig` shape is enforced by the OpenCode JSON schema (`https://opencode.ai/config.json`) and validated by editors via `$schema`.
    Frontmatter `description` is the only field treated as required by the loader — it controls automatic delegation. `mode` defaults to `all`, so a definition is invokable as both a primary agent (cycle with **Tab** / `switch_agent` keybind) and as a subagent (via `@mention` or the `task` tool) unless restricted.
    The Markdown body becomes the agent's prompt only when `prompt` is not explicitly set in frontmatter. When both exist, `prompt` wins. `{file:./relative/path}` and `{env:VAR}` substitutions are honored in the prompt field and in the config itself.

runtime:
  invocation: |
    Primary agents are selected by the user via the **Tab** key or the configured `switch_agent` keybind inside the TUI; the CLI equivalent is `opencode --agent <name>` (also `opencode run --agent <name>`). Subagents are invoked by name in one of three ways: (a) the primary agent calls the native `task` tool with `agent: "<name>"`, `description: "<short label>"`, and `prompt: "<task>"`; (b) the user types `@<name>` to autocomplete a subagent in the prompt and route the message directly to it; (c) the model autonomously picks a subagent based on the `description` field of every loaded subagent. Hidden subagents (`hidden: true`) are excluded from the user's `@` autocomplete menu but are still callable through (a) and (c). The TUI can navigate between parent and child sessions via `session_child_cycle` (**Right** / **Left**) and `session_parent` (**Up**). Subagents can be made uninvokable by an agent with `permission.task` glob rules; a `deny` removes the subagent from the task-tool description entirely (the model cannot even attempt it).
  parent_child_context: |
    Each subagent runs in its own isolated session. The parent session gets a `Part` of type `subtask` (`{ prompt, description, agent }`) recording the invocation; the child gets a fresh session whose `parentID` field points back at the parent. The OpenCode HTTP API surfaces this through `GET /session/:id/children`. The child session starts with its own system prompt (Markdown body or `prompt` field), the agent's `mode` (`primary`/`subagent`/`all`), the agent's `permission` overrides merged on top of the global `permission` config, the agent's `tools` / `permission` allowlist/denylist, and any agent-specific `model`, `temperature`, `top_p`, `variant`, and `options`. AGENTS.md / CLAUDE.md and the resolved `instructions` array are *not* reloaded for the child unless they were carried through `OPENCODE_CONFIG_CONTENT`; the system-prompt research (`docs/research/system-prompt/opencode.md`) confirms that `OPENCODE_CONFIG_CONTENT` propagates to child sessions, but the project `AGENTS.md` discovery walk does not re-execute for child sessions. The child's only return value to the parent is the final assistant text — the rest of the child transcript stays inside the child session and is queryable through `GET /session/:id/message`.
  permissions_inheritance: |
    The base layer is the global `permission` block in `opencode.json` (shorthand `"allow" | "ask" | "deny"` per key, or a glob→action map for `bash` and the file-pattern keys). When the agent definition supplies a `permission` block, OpenCode merges it on top: object-form entries for `bash`, `read`, `edit`, `glob`, `grep`, `list`, and `lsp` are unioned with rule-level merge (`last matching rule wins`); shorthand entries override the matching global rule. `--auto` (and the `auto` toggle in the TUI's command palette) flips every `ask` to `allow` but never downgrades an explicit `deny`. There is no concept of "trust mode" or sandbox inheritance — every tool call still hits the standard permission resolver, and per-agent `permission` entries can narrow the tool surface further (e.g. `code-reviewer` typically sets `edit: deny`). The legacy `tools` boolean map (deprecated in v1.1.1, replaced by `permission`) was equivalent to `{ "*": "allow" | "deny" }` and was applied per-agent on top of the global `tools`.
  model_inheritance: |
    Primary agents use the global `model` setting from `opencode.json` when no `model` field is set; the same is true of subagents per the official docs ("If you don't specify a model, primary agents use the [model globally configured](/docs/config#models) while subagents will use the model of the primary agent that invoked the subagent"). Subagents therefore inherit the *invoking primary agent's* model by default, not the global default. Per-agent `model` overrides both. `variant` is independent of model inheritance and applies only when the agent's own `model` field is in use. Provider-specific `options` (e.g. `reasoningEffort`, `textVerbosity`) and any unrecognized frontmatter fields are passed through to the provider request as model options.
  tool_inheritance: |
    The default is every tool the parent has access to, then narrowed by `permission` entries (`edit`/`bash`/`webfetch`/`websearch`/`skill`/`question`/`read`/`glob`/`grep`/`list`/`task`/`todowrite`/`lsp`/`external_directory`/`doom_loop`). The `experimental.primary_tools` config lists tool IDs that should only be available to primary agents — these are *removed* from subagents entirely (a stronger rule than a per-agent `deny`). MCP tools participate through the same permission keys: `"mymcp_*": "deny"` removes every tool from `mymcp`, `"mymcp_search": "ask"` targets one. The legacy `tools` map (deprecated) used boolean allow/deny per tool name; `tools: { "write": false, "edit": false }` was the canonical read-only agent before v1.1.1.
  max_turns: |
    Optional `steps` (replaces the deprecated `maxSteps`) caps the number of agentic iterations before the agent is forced to respond with text only. Per the docs: "Control the maximum number of agentic iterations an agent can perform before being forced to respond with text only... If this is not set, the agent will continue to iterate until the model chooses to stop or the user interrupts the session." The WebSearch-style `tool_call` model capability and the per-message `MessageOutputLengthError` are separate upper bounds enforced by the provider, not by the agent config. There is no documented hard limit on nested delegation depth; the `permission.task` rule on the parent agent is the practical way to bound it.
  notes: |
    Concurrency: multiple subagents can run in parallel because each invocation opens a new child session; the parent waits for child completions before continuing (unless `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=1` is enabled, which decouples the spawn from the parent's tool-loop step). Nesting: a subagent can in turn call the `task` tool itself, producing a grandchild session whose `parentID` is the child session ID — `GET /session/<grandparent>/children` walks one level only. The `experimental.primary_tools` flag is the way to draw a hard line between primary and subagent tool sets (the parent-only tools simply don't exist for subagents). Selection: an agent with `mode: all` is selectable both ways; an agent with `mode: subagent` is invokable only via `task` or `@`; an agent with `mode: primary` is only reachable via the cycle keybind / `--agent` flag. Disabling: `disable: true` removes the agent from every discovery list, equivalent to deleting the file. The interactive `opencode agent create` command walks the user through writing a new agent Markdown file with the right frontmatter and a permission preset.

observability:
  stream_events:
    - "message.part.updated with `part.type == \"subtask\"` ({ prompt, description, agent }) is emitted on the parent session when a subagent is invoked"
    - "session.created with `parentID` set on the new session payload identifies a child/subagent session"
    - "session.status changes per session (idle / busy / retry) reflect subagent lifecycle in the parent's session list"
    - "session.idle fires per session when a subagent finishes; use `parentID` on each session to correlate"
    - "message.updated / message.part.updated fire on the *child* session, identifiable by `sessionID` matching the child"
    - "GET /session/:id/children returns the full list of child sessions for any session"
  hook_events: []
  session_ids: true
  notes: |
    OpenCode does not expose a dedicated `subagent_start` / `subagent_stop` stream event. Wrapper consumers detect subagent lifecycle by watching for either (a) the `subtask` message part emitted on the parent session when the parent calls the `task` tool, or (b) a new `session.created` event whose payload has a non-empty `parentID` (the child's session ID stays stable for the lifetime of the subagent). Subagent completion is signaled by `session.idle` for the child session, not by any subagent-specific event. The bus exposes the same `Event` union over SSE on `/global/event` and `/event`; both `/session` endpoints (`POST /session`, `POST /session/:id/init`, `POST /session/:id/fork`, `POST /session/:id/abort`, `POST /session/:id/permissions/:permissionID`) are session-level, not subagent-level. There is no hook script system analogous to Claude Code hooks; the experimental `experimental.hook.file_edited` and `experimental.hook.session_completed` config blocks (introduced 2026) cover file-edit and session-completion shell hooks but no per-subagent hook. Permission prompts over a subagent tool call still surface as `permission.updated` events on the *parent* session and resolve through `POST /session/:id/permissions/:permissionID`, because the child session is its own session ID. TUI navigation uses `session_child_cycle` / `session_child_cycle_reverse` / `session_parent` keybinds (Right/Left/Up by default) to walk between parent and child sessions.
    Local `~/.config/opencode/agents/` on this host contains three symlinks to `~/.claude/agents/*.md` files; the OpenCode loader follows the symlinks and registers the agents as `mode: all` (the default). The Claude-authored files declare only `name` and `description` — they have no `mode`, no `tools`, and no `prompt`/`permission` block — yet `opencode agent list` reports them with mode `all`, confirming that OpenCode accepts partial AgentConfig entries and only `description` is required.

portability:
  portable: false
  non_portable_assets:
    - "OpenCode-only `mode` (`primary`/`subagent`/`all`) — no direct equivalent in Claude/Codex/Goose/Kimi/Qwen"
    - "OpenCode `permission` keys (`edit`/`bash`/`webfetch`/`doom_loop`/`external_directory`/`skill`/`question`/`todowrite`/`read`/`glob`/`grep`/`list`/`task`/`lsp`) with their `ask`/`allow`/`deny` triad — provider-specific vocabulary"
    - "OpenCode-only `tools` (legacy boolean map, deprecated)"
    - "OpenCode `color` (`#RRGGBB` or `primary`/`secondary`/`accent`/`success`/`warning`/`error`/`info`) — different palette than Claude's red/blue/green/yellow/purple/orange/pink/cyan"
    - "OpenCode-only `hidden` (`true` to remove from `@` autocomplete)"
    - "OpenCode `variant` (provider-specific reasoning effort)"
    - "OpenCode `options` (provider-passthrough) — every additional field is opaque to other providers"
    - "OpenCode `steps` (max agentic iterations before text-only response) — replaced legacy `maxSteps`"
    - "OpenCode `prompt` field with `{file:./prompts/build.txt}` shorthand and `{env:VAR}` substitution — syntax not portable"
    - "OpenCode `permission.task` glob rule (the dedicated channel for gating which subagent types an agent may spawn)"
    - "OpenCode `experimental.primary_tools` (hard primary-only tool allowlist)"
    - "Markdown body that references OpenCode-only tool names (`bash`, `edit`, `apply_patch`, `task`, `todowrite`, `webfetch`, `websearch`, `skill`, `lsp`, `question`), the `@<subagent>` mention syntax, or `$OPENCODE_*` env-var references"
  rewrite_needed: true
  notes: |
    The Markdown body of a simple OpenCode agent (description + role-played instructions) is the most portable piece and lifts cleanly to Claude Code subagents, Codex, or Gemini once the frontmatter is rewritten. The required `description` field is provider-neutral and carries the routing intent across implementations; it is the only field that survives a verbatim copy.
    A safe cross-provider rewrite preserves `description`, the body Markdown, the agent's identifier name (subject to the target provider's identifier grammar), and the `model` choice (when the target provider has the same model ID). It must drop or remap `mode`, the entire `permission` block, `tools`, `color`, `hidden`, `variant`, `options`, `steps`/`maxSteps`, `prompt` substitutions, and any `permission.task` glob rules. Body text that references OpenCode tool names (`bash`/`edit`/`task`/`todowrite`/`webfetch`/`skill`/`lsp`/`question`) must be rewritten to the target provider's vocabulary.
    The OpenCode JSON schema (`https://opencode.ai/config.json`) is the source of truth for the AgentConfig shape and is published at the schema URL used by every editor `$schema` reference.

cli_params:
  - flag: --agent <name>
    description: "Select a primary agent for the session. Must reference an existing agent name from JSON `agent.<name>` or `.opencode/agents/<name>.md` / `~/.config/opencode/agents/<name>.md`. If the agent has its own `prompt`, that prompt replaces the provider-specific stock prompt for the session."
    example: "opencode run \"Refactor auth\" --agent review"
  - flag: --pure
    description: "Run without external plugins (default plugins still load). Affects plugin-contributed agents only; does not affect agents declared in `opencode.json` or Markdown files."
    example: "opencode run --pure \"Hello\""
  - flag: --auto
    description: "Auto-approve permissions that are not explicitly denied. Equivalent to flipping every `ask` to `allow` in the merged `permission` config; affects both primary and child sessions."
    example: "opencode run --auto \"Refactor auth\""
  - flag: --model / -m <provider/model>
    description: "Override the global model for the session. Distinct from the per-agent `model` field; applies to whichever primary agent is selected unless that agent specifies its own `model`."
    example: "opencode run -m anthropic/claude-sonnet-4-5 \"Explain closures\""
  - flag: --continue / -c (and --session / -s <id>, --fork)
    description: "Continue or resume a prior session. Sessions are resumed by ID and retain their primary agent choice; --fork starts a new session branched at the chosen message."
    example: "opencode --continue --agent review"
  - flag: --hostname / --port / --mdns / --mdns-domain / --cors (serve, web, acp, attach)
    description: "Network exposure flags for the OpenCode server (`serve`), web (`web`), ACP server (`acp`), and TUI attach (`attach`). Do not affect agent discovery."
    example: "opencode serve --port 4096 --hostname 0.0.0.0"
  - flag: opencode agent create [--path <dir>] [--description <text>] [--mode <all|primary|subagent>] [--permissions <csv>] [--model <provider/model>]
    description: "Interactive (or non-interactive with all flags) agent scaffolder. Writes a Markdown file with the right frontmatter; `permissions` is a comma-separated allowlist (`bash`,`read`,`edit`,`glob`,`grep`,`webfetch`,`task`,`todowrite`,`websearch`,`lsp`,`skill`); anything omitted is set to `deny`. Alias: `--tools`."
    example: "opencode agent create --path .opencode/agents --description \"Reviews code for style\" --mode subagent --permissions read,grep --model anthropic/claude-haiku-4-5"
  - flag: opencode agent list
    description: "List every loaded agent with its resolved mode (one of `primary`, `subagent`, `all`). The default output is plain text, one agent per line, followed by a JSON dump of the merged `permission` rules; there is no `--format json` option as of v1.17.13."
    example: "opencode agent list"

env_vars:
  - name: OPENCODE_CONFIG
    effect: "Path to a custom `opencode.json` file loaded between global and project configs (config precedence step 3). Carries `agent.<name>` blocks and can override agents from disk."
  - name: OPENCODE_CONFIG_CONTENT
    effect: "Raw JSON config applied session-wide, including to child/subagent sessions (highest non-managed precedence). Carries `agent.<name>` blocks; ideal surface for wrappers that want to inject an agent without writing a file."
  - name: OPENCODE_CONFIG_DIR
    effect: "Custom directory searched for `agents/`, `commands/`, `modes/`, and `plugins/`. Loaded after global and `.opencode` directories, so it can override their settings."
  - name: OPENCODE_PERMISSION
    effect: "Inlined JSON `permission` block, applied to the merged permission config without touching disk."
  - name: OPENCODE_DISABLE_DEFAULT_PLUGINS
    effect: "Disables the bundled default plugins (which can ship extra agents). Does not affect agents in `opencode.json` or `.opencode/agents/`."
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: "Disable all `.claude` support (prompt + skills). Indirectly disables Claude-symlinked agents if those are discovered via the Claude compatibility layer."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
    effect: "Disable reading `~/.claude/CLAUDE.md` and per-project `CLAUDE.md` files. Independent of agent discovery."
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: "Disable loading `.claude/skills`. Independent of agent discovery."
  - name: OPENCODE_EXPERIMENTAL_SCOUT
    effect: "Enable the built-in `scout` subagent (read-only external-doc/dependency research). Without this flag, `scout` is omitted from the loaded agent list — observed locally on v1.17.13."
  - name: OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS
    effect: "Decouple subagent spawns from the parent's tool-loop step so multiple subagents can run in parallel without blocking the parent."
  - name: OPENCODE_EXPERIMENTAL_PLAN_MODE
    effect: "Toggle the experimental plan-mode surface. Affects the `plan` primary agent's prompt; not directly about user-defined subagents."
  - name: OPENCODE_EXPERIMENTAL_WORKSPACES
    effect: "Workspace support; may affect how `.opencode` directories are walked and merged, which in turn changes which project-scope agents are loaded."
  - name: OPENCODE_EXPERIMENTAL_EVENT_SYSTEM
    effect: "Use the experimental event system for the SSE bus (`/event`); affects how wrappers observe subagent lifecycle events but does not change the event schema."
  - name: OPENCODE_AUTO_SHARE
    effect: "Automatically share sessions. Indirect: shared sessions expose the same subagent child sessions on `/session/:id/children`."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `claudine agents` link command does not yet recognize OpenCode's `agents/` discovery surface (`~/.config/opencode/agents/<name>.md`, `.opencode/agents/<name>.md`, `$OPENCODE_CONFIG_DIR/agents/<name>.md`, and the JSON `agent.<name>` block in `opencode.json`/`opencode.jsonc`/`OPENCODE_CONFIG_CONTENT`). A future OpenCode agent-linker entry must:
  1. Walk the four filesystem scopes (global, project walk-up, custom config dir, managed/system) and parse Markdown frontmatter using the canonical `AgentConfig` schema from `https://opencode.ai/config.json`.
  2. Read the JSON `agent` block from every config file in precedence order and merge it with the Markdown definitions.
  3. Apply same-name resolution rules (later scopes win; managed > custom > global > project by precedence) and distinguish `mode: primary` / `mode: subagent` / `mode: all`.
  4. Surface the resolved model, prompt file reference (`{file:./prompts/build.txt}` shorthand), and permission block; flag assets that depend on `permission.task` glob rules, `experimental.primary_tools`, `color`, `hidden`, or the deprecated `tools`/`maxSteps` fields as needing rewrite when linking to another provider.
  5. Update the linking-classification table to record that OpenCode agents carry a provider-neutral `description` plus a provider-specific `permission`/`mode`/`color`/`hidden`/`tools`/`options`/`variant` set — portable only on the body, never on the frontmatter.
  For lifecycle `proxy`/`resume`: OpenCode does not emit a `subagent_start`/`subagent_stop` pair. The stable signal is the child session's `parentID` on a `session.created` event, plus the `subtask` message part on the parent when the `task` tool fires. A wrapper that wants to address a specific subagent should keep the `sessionID` of the child, and use `GET /session/:id/children` to enumerate. Per-message tool parts on the child carry the same `sessionID`, so a transcript-shaped resume is possible if the wrapper joins on `sessionID`. There is no `agent_id`/`agent_type` field analogous to Claude Code; the agent identity lives in the session title and in the `subtask` part's `agent` string. Resume of an interrupted subagent is `opencode run --session <child-id> --agent <name>` or `POST /session/<child-id>/message`.

---

# OpenCode Subagents

## Overview

OpenCode treats user-defined **agents** as a first-class feature with two concrete flavors — **primary agents** (the main assistants the user interacts with directly) and **subagents** (specialized assistants that a primary agent invokes via the `task` tool or that the user invokes via `@mention`). The provider calls the whole feature "agents"; "subagent" is the documented term for the second flavor, and every primary agent can also be invoked as a subagent unless `mode: primary` is set. Two declaration surfaces are accepted: YAML-frontmatter Markdown files in `.opencode/agents/` or `~/.config/opencode/agents/`, and JSON `agent.<name>` blocks in any config file (`opencode.json`, `opencode.jsonc`, the custom-path file pointed to by `OPENCODE_CONFIG`, the directory pointed to by `OPENCODE_CONFIG_DIR`, or inline JSON in `OPENCODE_CONFIG_CONTENT`). The `AgentConfig` schema is shared between both surfaces and is published at `https://opencode.ai/config.json`; editors pick it up automatically via `$schema`.

OpenCode ships built-in primary agents `build` and `plan` (plus the hidden system agents `compaction`, `title`, `summary`) and built-in subagents `general`, `explore`, and `scout` (with `scout` gated behind `OPENCODE_EXPERIMENTAL_SCOUT=1`). All eight can be overridden by a user-scope or project-scope definition with the same name. Support is `first_class`: there are documented scopes (system/managed, user, project, extension/plugin, inline), a stable frontmatter schema, runtime delegation semantics via the `task` tool and `@mention`, isolated child sessions with a `parentID` lineage, and a stable HTTP API surface for wrappers.

This topic is the *definition* of subagents — where the files live, what frontmatter they accept, how the parent picks one, what context and permissions the child gets, and how a wrapper can observe starts/stops. The hooks topic owns lifecycle event semantics; this document records only which stream/hook events expose agent lifecycle.

## Locations

Definitions are stored by scope. The config-precedence order from the official Config doc applies (later sources override earlier ones for conflicting keys; non-conflicting settings are merged):

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| Managed / system (file) | `/Library/Application Support/opencode/opencode.json` | `/etc/opencode/opencode.json` | `%ProgramData%\opencode\opencode.json` | Admin-controlled `opencode.json` / `.jsonc`. Highest precedence among file-based configs. |
| Managed / system (MDM) | `/Library/Managed Preferences/<user>/ai.opencode.managed.plist` | n/a | n/a | macOS MDM `PayloadType=ai.opencode.managed`. Payload keys map 1:1 to `opencode.json` fields. |
| Remote | `.well-known/opencode` | `.well-known/opencode` | `.well-known/opencode` | Fetched when authenticating with a provider that supports it. Lowest precedence. |
| Custom config dir | `$OPENCODE_CONFIG_DIR/agents/<name>.md` and `$OPENCODE_CONFIG_DIR/opencode.json` | same | same | Loaded after global and `.opencode` directories, so it can override their settings. |
| Global | `~/.config/opencode/agents/<name>.md` and `~/.config/opencode/opencode.json` | `~/.config/opencode/agents/<name>.md` and `~/.config/opencode/opencode.json` | `%USERPROFILE%\.config\opencode\agents\<name>.md` and `%USERPROFILE%\.config\opencode\opencode.json` | Personal user-scope. Symlinks to files outside this directory are followed. |
| Project | `.opencode/agents/<name>.md` and `opencode.json` (or `opencode.jsonc`) in the project root | same | `.opencode\agents\<name>.md` | Every `.opencode` directory walked from CWD up to the git worktree. Loaded via `OPENCODE_CONFIG_DIR` semantics. |
| Inline overlay | `OPENCODE_CONFIG_CONTENT` env var (JSON string) | same | same | Session-only. Propagates to child sessions. Highest precedence short of managed configs. |
| Plugin | `<plugin>/agents/<name>.md` (or plugin-contributed JSON `agent.<name>`) | same | same | Plugins can ship agent definitions; toggle bundled defaults with `OPENCODE_DISABLE_DEFAULT_PLUGINS`. |

OpenCode's loader walks each scope for both Markdown files (`{agents,agent}/<name>.md`) and JSON config files, then merges them by name. Singular `agent/` is also accepted as a documented backwards-compatibility alias.

On this host (macOS), `~/.config/opencode/agents/` contains three symlinks to `~/.claude/agents/*.md` (the Claude Code `feature-tester-rust.md`, `feature-tester-typescript.md`, and `tester-agent.md` files). The OpenCode loader follows the symlinks and registers each as `mode: all` (the default), even though the files declare only `name` and `description` and lack any `prompt` / `permission` block — confirming that the loader accepts partial AgentConfig entries and that `description` is the only required field. `opencode agent list` on this host returns the following loaded agents, in line with the source:

```
build (primary)
compaction (primary)
explore (subagent)
general (subagent)
plan (primary)
summary (primary)
title (primary)
CLI Developer (all)
code-simplifier (all)
Documenter (subagent)
feature-tester-rust (all)
feature-tester-typescript (all)
just-scripter (all)
rust-designer (all)
rust-developer (all)
spec-writer (all)
tester-agent (all)
```

`scout` is absent from this list because `OPENCODE_EXPERIMENTAL_SCOUT` is not set on this host.

## Definition Format

The two declaration surfaces are equivalent. JSON entries map to the same `AgentConfig` shape as Markdown frontmatter.

### Markdown form

A Markdown agent is a single file with YAML frontmatter. The filename stem becomes the agent name (e.g. `review.md` registers `review`).

```markdown
---
description: Reviews code for quality and best practices
mode: subagent
model: anthropic/claude-sonnet-4-20250514
temperature: 0.1
permission:
  edit: deny
  bash: deny
  webfetch: deny
color: accent
---

You are in code review mode. Focus on:
- Code quality and best practices
- Potential bugs and edge cases
- Performance implications
- Security considerations

Provide constructive feedback without making direct changes.
```

When the agent is also the active primary agent for the session, the Markdown body becomes the system prompt in place of the provider's stock prompt; the system-prompt topic documents the replacement semantics in detail.

### JSON form

Inline in `opencode.json` (or `.jsonc`):

```json
{
  "$schema": "https://opencode.ai/config.json",
  "agent": {
    "build": {
      "mode": "primary",
      "model": "anthropic/claude-sonnet-4-20250514",
      "prompt": "{file:./prompts/build.txt}",
      "permission": { "edit": "allow", "bash": "allow" }
    },
    "plan": {
      "mode": "primary",
      "model": "anthropic/claude-haiku-4-20250514",
      "permission": { "edit": "deny", "bash": "deny" }
    },
    "code-reviewer": {
      "description": "Reviews code for best practices and potential issues",
      "mode": "subagent",
      "model": "anthropic/claude-sonnet-4-20250514",
      "prompt": "You are a code reviewer. Focus on security, performance, and maintainability.",
      "permission": { "edit": "deny" }
    }
  }
}
```

`prompt` accepts inline text, `{file:./relative/path}` shorthand (resolved relative to the config file's directory), or `{env:VAR}` substitution. Unrecognized frontmatter fields fall through to the provider as model options (e.g. `reasoningEffort`, `textVerbosity`).

### Recognized fields

- Required: `description` (routing signal for automatic delegation; required by the loader per the official agents doc).
- Mode: `mode` (`primary` | `subagent` | `all`; default `all`).
- Prompt and model: `model` (`provider/model`), `prompt` (system prompt text or `{file:...}` shorthand), `variant` (provider-specific reasoning effort, paired with `model`), `temperature` (0.0-1.0), `top_p` (0.0-1.0).
- Permissions: `permission` (object; key set is `edit`, `bash`, `webfetch`, `websearch`, `websearch`, `doom_loop`, `external_directory`, `skill`, `question`, `read`, `glob`, `grep`, `list`, `task`, `todowrite`, `lsp`; values are `"allow" | "ask" | "deny"` or, for `bash`/`read`/`edit`/`glob`/`grep`/`list`/`lsp`/`skill`, a glob→action map).
- Tool surface (legacy): `tools` (boolean map; `@deprecated` since v1.1.1, use `permission` instead).
- Display: `color` (hex `#RRGGBB` or theme name: `primary`/`secondary`/`accent`/`success`/`warning`/`error`/`info`).
- Visibility: `hidden` (`true` removes from `@` autocomplete; only meaningful when `mode: subagent`).
- Lifecycle: `disable` (`true` removes the agent from discovery), `steps` (max agentic iterations before forced text-only response; replaces `maxSteps`).
- Passthrough: `options` (object passed through to the provider as model options).

The body of a Markdown agent file is itself stored as the agent's system prompt when `prompt` is not set; setting both keeps `prompt` and discards the body.

## Runtime Behavior

A subagent is invoked by the primary agent with the native `task` tool. The tool's parameters are `agent` (the agent name), `description` (a short label for the parent's transcript), and `prompt` (the task text). The parent's session then emits a `subtask` message part recording the invocation; OpenCode creates a child session with a fresh `parentID`, and the child's first user message is the parent's `prompt` text. The child runs to completion and the final assistant text is the only thing that returns to the parent.

Three other invocation paths reach a subagent:

1. **User `@mention`**. Typing `@<name>` in the TUI prompt injects the subagent's prompt as a `@file`-style reference and routes the message to it. Hidden subagents (`hidden: true`) are excluded from autocomplete but still callable through paths 2 and 3.
2. **Autonomous selection**. The primary agent picks a subagent when the user's prompt matches one of the loaded `description` fields.
3. **CLI direct**. `opencode run --agent <name>` (or `--continue --agent <name>`) starts the session as the named agent; this works for any agent whose `mode` includes `primary` or `all`, and *not* for agents declared `mode: subagent`.

Selection is gated by `permission.task` on the calling agent. A glob rule like `permission.task: { "*": "deny", "code-reviewer": "ask" }` removes the denied agents from the task-tool description entirely (the model can't even attempt them) and surfaces an `ask` prompt for the others. Last matching rule wins, and the user can always bypass the gate by typing `@<name>` manually.

Each child runs in its own session with its own message history, permission resolver, and tool allowlist. The child does **not** reload `AGENTS.md`/`CLAUDE.md` or re-walk the project `.opencode` discovery tree; the system-prompt research confirms that only `OPENCODE_CONFIG_CONTENT` propagates into child sessions. The child's resolved permission is the global `permission` config merged with the agent's `permission` block (object entries are unioned; shorthand entries override). `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=1` lets the parent fire multiple subagent calls in parallel without waiting for each one.

The TUI exposes parent/child navigation via three keybinds: `session_child_first` (default **Leader+Down**) enters the first child, `session_child_cycle` (**Right**) / `session_child_cycle_reverse` (**Left**) walks siblings, and `session_parent` (**Up**) returns to the parent. These keybinds target session IDs and use the same `parentID` lineage that the HTTP API exposes through `GET /session/:id/children`.

## Observability

OpenCode does not emit a dedicated `subagent_start` / `subagent_stop` pair. Wrappers detect subagent lifecycle through three coordinated signals:

1. **Subtask part on the parent**. When the parent calls the `task` tool, the parent session emits a `message.part.updated` event whose `part.type` is `"subtask"` and whose payload is `{ prompt, description, agent }`. This is the "subagent started" signal that is directly attached to the parent's transcript.
2. **Child session creation**. A `session.created` event whose payload's `Session.parentID` is non-empty identifies a new child/subagent session. The child session's `Session.id` stays stable for the lifetime of the subagent and is the same ID used in `/session/:id`, `/session/:id/message`, and `/session/:id/abort` calls. The child session's `Session.title` and the `subtask` part's `agent` string carry the agent identity.
3. **Per-session status and idle**. Each session has its own `session.status` (`idle` / `busy` / `retry`) and fires its own `session.idle` when it stops. Wrappers identify the child's idle by joining on `sessionID` from the `session.created` payload.

There is no hook-script system for subagents in v1.17.13. The experimental `experimental.hook.file_edited` and `experimental.hook.session_completed` config blocks fire shell commands on file edits and session completion but are not subagent-scoped. Permission prompts raised by a subagent's tool call still surface on the *parent* session as `permission.updated` events (because OpenCode lifts the prompt to the user, not the child session), and are resolved with `POST /session/:id/permissions/:permissionID` on the parent session ID.

The full `Event` union is published in the OpenAPI spec (`GET /doc`) and in the SDK types at `packages/sdk/js/src/gen/types.gen.ts`. The relevant types for subagent observability are:

| Type | Trigger | Subagent relevance |
|---|---|---|
| `EventMessagePartUpdated` (`message.part.updated`) | A message part changes | The parent's `part.type == "subtask"` is the "started" signal |
| `EventSessionCreated` (`session.created`) | A new session is created | Non-empty `parentID` indicates a child session |
| `EventSessionStatus` (`session.status`) | A session's status changes | Track the child's `busy`/`idle` transitions |
| `EventSessionIdle` (`session.idle`) | A session finishes | Subagent completion when the sessionID matches a child |
| `EventSessionUpdated` (`session.updated`) | A session's metadata changes | `parentID` field also shows on updates for live children |
| `EventSessionError` (`session.error`) | A session errored | Surfaces on the child session ID |
| `EventPermissionUpdated` (`permission.updated`) | A tool needs approval | Per-session; lifted to the parent for subagent tool calls |

Local `opencode agent list` on this host confirms the loader's partial-acceptance behavior: Claude-authored agents with only `name` and `description` are registered as `mode: all` and surfaced in the list with no warning, even though they lack a `prompt`/`model`/`permission` block.

## Portability

OpenCode agents are **not portable** across providers as-is. The body Markdown is the most portable piece and lifts cleanly to Claude Code subagents, Codex, or Gemini once the frontmatter is rewritten. The required `description` field is provider-neutral and carries the routing intent across implementations. The rest of the frontmatter is provider-specific.

| Field | Portable? | Rewrite target |
|---|---|---|
| `description` | yes | Carries the routing signal across providers. |
| `mode` | no | Remap to the target provider's mode/role vocabulary (`subagent` ↔ subagent / `--agent`, `primary` ↔ main-thread replacement). |
| `model` | depends | Same `provider/model` ID only if the target provider also exposes it; otherwise map to the target's model identifier. |
| `prompt` | depends | Strip `{file:...}` / `{env:...}` substitutions; verify body text doesn't reference OpenCode-only tool names. |
| `temperature`, `top_p`, `variant` | depends | Most providers accept these names; `variant` is OpenCode-only. |
| `permission` | no | Remap each key to the target provider's permission vocabulary and the `ask`/`allow`/`deny` triad. Drop `permission.task` (OpenCode-only), `external_directory`, `doom_loop`, and `skill`. |
| `tools` (legacy) | no | Remap boolean keys to the target provider's tool identifier shape. |
| `color` | no | Remap to the target provider's color palette. |
| `hidden` | no | Drop or remap; some providers have no `@mention` UI. |
| `steps` / `maxSteps` | no | Remap to the target provider's turn-limit field (e.g. Claude's `maxTurns`). |
| `options` | no | Provider-specific passthrough; review each key. |
| `disable` | depends | Some providers have an `enabled`/`disable` flag; otherwise drop. |
| Body Markdown | partial | Body is portable when it references standard Markdown; rewrite references to OpenCode-only tools (`bash`, `edit`, `apply_patch`, `task`, `todowrite`, `webfetch`, `websearch`, `skill`, `lsp`, `question`) and to `$OPENCODE_*` env-var names. |

The OpenCode JSON schema (`https://opencode.ai/config.json`) is the authoritative source for the `AgentConfig` shape and is what every editor's `$schema` reference points to. Plugins that ship agents through their V2 plugin API are even less portable because they couple the agent to plugin-only hooks/tools.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy`/`resume` actions:

- **Discovery surfaces to enumerate** for the OpenCode agent linker:
  1. `~/.config/opencode/agents/<name>.md` (user-scope, follows symlinks).
  2. Every `.opencode/agents/<name>.md` from CWD up to the git worktree (project-scope).
  3. `$OPENCODE_CONFIG_DIR/agents/<name>.md` (custom config dir).
  4. The JSON `agent.<name>` block in `~/.config/opencode/opencode.json`, `opencode.json` / `opencode.jsonc` in the project, `$OPENCODE_CONFIG_DIR/opencode.json`, and the file pointed to by `OPENCODE_CONFIG`.
  5. The `agent.<name>` block inside `OPENCODE_CONFIG_CONTENT` (session-only overlay; effectively ephemeral).
  6. The managed-system files at `/Library/Application Support/opencode/opencode.json` (macOS), `/etc/opencode/opencode.json` (Linux), `%ProgramData%\opencode\opencode.json` (Windows), plus the macOS MDM `.mobileconfig` plist at `/Library/Managed Preferences/ai.opencode.managed.plist`.
  7. Plugin-contributed agents (toggle with `OPENCODE_DISABLE_DEFAULT_PLUGINS`).
- **Listing command**: `opencode agent list` returns every loaded agent with its resolved mode (one of `primary` / `subagent` / `all`). Use this as the listing source instead of re-walking the filesystem twice; it already accounts for config merging and plugin contributions.
- **Portability classification**: classify OpenCode agents as **non-portable**. The Markdown body carries most of the agent's *purpose* and may be lifted as-is to another provider's body, but the frontmatter is OpenCode-specific. Flag assets that depend on `mode`, `permission`, `permission.task`, `tools` (legacy), `color`, `hidden`, `variant`, `options`, `steps`/`maxSteps`, `prompt` substitutions, or `experimental.primary_tools` as needing rewrite, stripping, or host gating before they can land elsewhere.
- **Lifecycle `proxy`/`resume`**: there is no `agent_id` / `agent_type` pair analogous to Claude Code. The stable identity is the child session's `Session.id`, paired with the parent's `subtask` part's `agent` string and the `parentID` field on the child session. To address a specific subagent from a wrapper:
  - Capture the `sessionID` from the `session.created` event whose payload has a non-empty `parentID` (this is the child).
  - Resume the child directly with `opencode run --session <child-id> --agent <name>` or `POST /session/<child-id>/message` (with `agent: "<name>"` in the body).
  - Use `GET /session/<child-id>/children` to walk the child's grandchildren when nesting.
  - For tool-call permission prompts raised by a subagent, look for `permission.updated` on the parent session (the prompt is lifted to the parent, not the child), and resolve with `POST /session/<parent-id>/permissions/<permissionID>`.
- **Wrappers that need `OPENCODE_CONFIG_CONTENT`**: the inline JSON overlay already used for MCP injection and YOLO permissions is the natural surface for an OpenCode agent linker. Agents injected here propagate to child sessions automatically, so a wrapper that wants to scope a custom agent to one run can use it without touching the project filesystem. The same surface can carry `permission.task` rules to gate which subagents the parent can call.
- **Model resolution**: subagents inherit the invoking primary agent's model unless the agent's own `model` field is set; wrappers should not assume a subagent uses the global default. The four-tier resolution is: (1) agent's `model` field, (2) invoking primary agent's model, (3) `opencode.json` global `model`, (4) `--model` CLI override on the parent run.

## Sources

- [OpenCode — Agents](https://opencode.ai/docs/agents/)
- [OpenCode — Config](https://opencode.ai/docs/config/)
- [OpenCode — CLI](https://opencode.ai/docs/cli/)
- [OpenCode — Permissions](https://opencode.ai/docs/permissions/)
- [OpenCode — TUI](https://opencode.ai/docs/tui/)
- [OpenCode — Server / HTTP API](https://opencode.ai/docs/server/)
- [OpenCode JSON schema (`https://opencode.ai/config.json`)](https://opencode.ai/config.json)
- [OpenCode TypeScript SDK types — `packages/sdk/js/src/gen/types.gen.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/sdk/js/src/gen/types.gen.ts)
- [OpenCode product homepage](https://opencode.ai/)
- [OpenCode on GitHub](https://github.com/anomalyco/opencode)