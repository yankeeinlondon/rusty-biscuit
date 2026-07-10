---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://kilo.ai/
docs: https://kilocode.ai/docs/code-with-ai/platforms/cli
subagent_docs: https://kilocode.ai/docs/customize/custom-subagents

support: first_class

locations:
  - os: macos
    scope: user
    path: "~/.config/kilo/agent/<name>.md"
    notes: "User-scope Markdown agent. The directory `agent/` (singular) and `agents/` (plural) are both accepted. The basename (without `.md`) becomes the agent name unless an explicit `name:` is set in frontmatter. Nested directories are not honored at the user-scope level (the docs page names only the immediate children). The filename is not part of the identity; the frontmatter `name` field is. Resolved through the same `Global.Path.config` XDG path the rest of the Kilo config uses."
  - os: linux
    scope: user
    path: "~/.config/kilo/agent/<name>.md"
    notes: "Same as macOS. Resolved under XDG (`$XDG_CONFIG_HOME/kilo/agent/<name>.md`, default `~/.config/kilo/agent/<name>.md`). Confirmed via the `kilo agent create` interactive prompt and the source `packages/opencode/src/cli/cmd/agent.ts`."
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\agent\\<name>.md"
    notes: "Windows path form. Resolved through `Global.Path.config` which on Windows is `%APPDATA%\\kilo\\`. Both `agent\\` and `agents\\` accepted."
  - os: macos
    scope: user
    path: "~/.config/kilo/agents/<name>.md"
    notes: "Plural `agents/` directory — alternative to singular `agent/`. Both are walked at session start. Nested directories (e.g. `agents/backend/sql.md`) register as `backend/sql` per the docs and source."
  - os: linux
    scope: user
    path: "~/.config/kilo/agents/<name>.md"
    notes: "Same as macOS."
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\agents\\<name>.md"
    notes: "Same as macOS/Linux; backslash form."
  - os: macos
    scope: repo
    path: ".kilo/agent/<name>.md"
    notes: "Project-scope Markdown agent. The directory `.kilo/agent/` (singular) and `.kilo/agents/` (plural) are both accepted. `kilo agent create` defaults to `<worktree>/.kilo/agents/<name>.md` in a git repo, and falls back to `<Global.Path.config>/agents/` in non-git directories. Nested directories register as `<dir>/<name>`."
  - os: linux
    scope: repo
    path: ".kilo/agent/<name>.md"
    notes: "Same as macOS. Resolved under the session's project worktree."
  - os: windows
    scope: repo
    path: ".kilo\\agent\\<name>.md"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: repo
    path: ".kilo/agents/<name>.md"
    notes: "Plural `agents/` form of the project-scope agent directory. Default target for `kilo agent create` when scope is `project` and the worktree is a git repo."
  - os: linux
    scope: repo
    path: ".kilo/agents/<name>.md"
    notes: "Same as macOS."
  - os: windows
    scope: repo
    path: ".kilo\\agents\\<name>.md"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: repo
    path: ".kilocode/agents/<name>.md"
    notes: "Legacy `.kilocode/` location. Still read by Kilo CLI 1.0 for compatibility with the pre-CLI VS Code extension. `ModesMigrator` (see `kilocode/agent/index.ts`) reads `.kilocodemodes` YAML from the same directory and from `~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code/settings/custom_modes.yaml`."
  - os: linux
    scope: repo
    path: ".kilocode/agents/<name>.md"
    notes: "Same as macOS."
  - os: windows
    scope: repo
    path: ".kilocode\\agents\\<name>.md"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: other
    path: "kilo.jsonc (or kilo.json) `agent.<name>` key"
    notes: "Inline JSON agent definition inside the config file. The same `AgentConfig` schema is shared with the Markdown form; the JSON key becomes the agent name. Project, global, custom (`KILO_CONFIG`), and inline (`KILO_CONFIG_CONTENT`) configs all accept `agent` blocks and are merged together. Inheritance is field-level (an entry with the same name in a higher-precedence source overrides only the listed fields, leaving other fields at the merged value)."
  - os: linux
    scope: other
    path: "kilo.jsonc (or kilo.json) `agent.<name>` key"
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: "kilo.json[c] `agent.<name>` key"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: other
    path: "$KILO_CONFIG_DIR/agents/<name>.md"
    notes: "Custom config directory. Set via `KILO_CONFIG_DIR`; searched for `agents/` (and `commands/`, `plugins/`, `skills/`) like a standard `.kilo` directory. Loaded after global and project configs, so it can override their settings."
  - os: linux
    scope: other
    path: "$KILO_CONFIG_DIR/agents/<name>.md"
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: "$KILO_CONFIG_DIR\\agents\\<name>.md"
    notes: "Same as macOS/Linux."
  - os: macos
    scope: system
    path: "/Library/Application Support/kilo/opencode.json"
    notes: "macOS managed-config directory. Admin-controlled `opencode.json` or `.jsonc` dropped here (the path was forked from OpenCode's `/Library/Application Support/opencode/`); the `kilo` directory name is the new prefix. Highest file-based precedence for the `agent.<name>` block; not user-overridable. Sources: `packages/opencode/src/config/managed.ts` and the MCP research `claudine/docs/research/mcp/kilo.md`."
  - os: linux
    scope: system
    path: "/etc/kilo/opencode.json"
    notes: "Linux managed-config path. Admin-controlled file; highest config precedence. (`/etc/kilo/opencode.json` was the OpenCode path renamed to `kilo/`.)"
  - os: windows
    scope: system
    path: "%ProgramData%\\kilo\\opencode.json"
    notes: "Windows managed-config path. Admin-controlled file; highest config precedence."
  - os: macos
    scope: system
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    notes: "macOS MDM-deployed preferences. `PayloadType=ai.opencode.managed` is the dedicated channel; payload keys map 1:1 to `kilo.json[c]` fields, so an admin can ship a managed `agent` allowlist. Same precedence as the file-based managed config. (Kilo kept the `ai.opencode.managed` plist domain after forking from OpenCode.)"
  - os: linux
    scope: system
    path: ".well-known/opencode"
    notes: "Remote organizational config fetched automatically when authenticating with a remote provider (e.g. Kilo Gateway) that supplies a `type: wellknown` credential entry. Loaded first in the config precedence order; can ship default `agent` entries. (Verified by MCP research: same precedence order as OpenCode.)"
  - os: windows
    scope: system
    path: ".well-known/opencode"
    notes: "Remote organizational config; same behavior as Linux."
  - os: macos
    scope: extension
    path: "kilo plugin <npm-module> contributes `agent.<name>` blocks"
    notes: "Plugins can contribute agents by registering them in their own config contribution. Bundled default plugins (`@kilocode/kilo-indexing`, `@kilocode/plugin-atomic-chat` shipped with the host's `kilo.jsonc` `plugin` array) ship extra agents. Toggle bundled defaults with `KILO_DISABLE_DEFAULT_PLUGINS`. Organization-managed agents distributed through the cloud arrive as `source: \"organization\"` on the resolved `Info`; users cannot remove them."
  - os: linux
    scope: extension
    path: "kilo plugin <npm-module> contributes `agent.<name>` blocks"
    notes: "Same as macOS."
  - os: windows
    scope: extension
    path: "kilo plugin <npm-module> contributes `agent.<name>` blocks"
    notes: "Same as macOS/Linux."

format:
  file_names:
    - "<name>.md (file basename or nested path becomes the agent name when no frontmatter `name` is set)"
    - "kilo.json[c] `agent.<name>` (JSON key becomes the agent name)"
  frontmatter: true
  required_fields:
    - "description (routing signal; the docs explicitly state the orchestrator uses the description to decide which subagent to invoke)"
  optional_fields:
    - "mode (`primary` | `subagent` | `all`; defaults to `all` for custom agents)"
    - "model (`provider/model-id`; overrides the agent's default model when set)"
    - "prompt (system prompt — for Markdown form the body IS the prompt; for JSON form `prompt` is inline text or `{file:./path}` shorthand)"
    - "temperature (0.0-1.0)"
    - "top_p (0.0-1.0)"
    - "permission (object; values are `\"allow\" | \"ask\" | \"deny\"` or a glob→action map; known keys include `bash`, `read`, `edit`, `glob`, `grep`, `webfetch`, `websearch`, `task`, `todowrite`, `todoread`, `lsp`, `skill`, `doom_loop`, `external_directory`, `question`, `suggest`, `interactive_terminal`, `plan_enter`, `plan_exit`, `repo_clone`, `repo_overview`, `recall`, `codebase_search`, `semantic_search`)"
    - "color (hex `#RRGGBB` or theme name: `primary`/`secondary`/`accent`/`success`/`warning`/`error`/`info`)"
    - "hidden (boolean; removes the agent from the `@` autocomplete menu; only meaningful when `mode: subagent`)"
    - "disable (boolean; removes the agent from every discovery list, equivalent to deleting the file)"
    - "steps (positive integer; max agentic iterations before the agent is forced to respond with text only)"
    - "variant (provider-specific reasoning effort, paired with `model`; e.g. `high`, `max`, `minimal`)"
    - "options (object passed through to the provider as model options; e.g. `reasoningEffort`, `textVerbosity`)"
    - "name (explicit identifier; overrides the filename-derived name — confirmed by the source `kilocode/agent/index.ts:processConfigItem` which reads `value.name ?? item.name`)"
    - "displayName (Kilo-specific; human-readable name shown in the agent picker for org-supplied modes — read from frontmatter or lifted from `options.displayName` then stripped from `options`)"
    - "source (Kilo-specific; one of `organization` | `global` | `project` — marks origin and gates the removal guard in `kilocode/agent/index.ts:remove`)"
    - "deprecated (boolean; marks an org-supplied mode as deprecated)"
    - "requirements (Kilo-specific; a `kilocode/agent-requirements` schema describing optional prerequisites that gate agent invocation)"
  body_format: markdown
  notes: |
    Two equivalent declaration surfaces:
    (1) **Markdown files with YAML frontmatter** under `.kilo/{agent,agents}/<name>.md` (or `.kilocode/agents/<name>.md` for legacy). The filename stem becomes the agent name unless the frontmatter carries an explicit `name:`. The body becomes the system prompt.
    (2) **JSON entries under the `agent` key** in `kilo.json[c]` (or `opencode.json[c]` for compatibility) and in the env-var overlays `KILO_CONFIG` and `KILO_CONFIG_CONTENT`. The JSON key becomes the agent name; the `prompt` field carries the system prompt inline or via `{file:./prompts/build.txt}` shorthand. The JSON `prompt` field is honored in the Markdown form too (it overrides the body when both are set).

    Frontmatter `description` is the only field treated as required by the loader — it is the routing signal the orchestrator uses to decide which subagent to invoke. The `permission` block is *last-match-wins* on glob patterns (per the CLI permissions docs and the source `packages/opencode/src/permission/permission.ts`).

    When `prompt` is set, the Markdown body is ignored. `{file:./relative/path}` and `{env:VAR}` substitutions are honored in the prompt field and elsewhere in the config; `{env:VAR}` falls back to empty string when unset. The legacy `.kilocodemodes` / `custom_modes.yaml` format is auto-migrated on first launch: `slug` → `name`, `roleDefinition` + `customInstructions` → `prompt`, `groups` (e.g. `["read", "edit", "browser"]`) → `permission` rules, `whenToUse` / `description` → `description`, `mode` is set to `primary`. Default legacy slugs (`code`, `build`, `architect`, `ask`, `debug`, `orchestrator`) are skipped during migration because they map to built-ins.

    The merged config precedence from lowest to highest, per the Custom Subagents docs and the upstream config source:
    1. Built-in (native) agent defaults
    2. Global config (`~/.config/kilo/kilo.jsonc`)
    3. Project config (`kilo.jsonc` at project root)
    4. Global Markdown files (`~/.config/kilo/agents/*.md`)
    5. Project Markdown files (`.kilo/agents/*.md`)

runtime:
  invocation: |
    Three documented invocation paths reach a custom agent:
    1. **Automatic delegation via the `task` tool** — a primary agent (the docs name Code, Plan, and Debug) calls the `task` tool with `subagent_type: "<name>"`, `description: "<short label>"`, `prompt: "<task>"`, optional `background: true`, optional `task_id: "<session-id>"` to resume. The TaskTool source at `packages/opencode/src/tool/task.ts` describes the parameters and the resume hint that flows back to the parent. The `description` frontmatter is the only required field for the orchestrator to pick the agent; the schema also allows `task` to be restricted to specific subagent types via `permission.task` globs (e.g. `permission.task: { "*": "deny", "code-reviewer": "allow" }`).
    2. **Manual `@<name>` mention** in the TUI prompt injects the subagent as the message target. The agent's `description` and `prompt` are honored; `hidden: true` excludes the agent from the autocomplete menu but it remains callable through the `task` tool and direct CLI invocations.
    3. **CLI direct** — `kilo --agent <name> <project>` starts the TUI as the named primary agent; `kilo run --agent <name> "..."` runs it non-interactively; `kilo agent create` walks the user through authoring a new agent; `kilo agent list` enumerates every loaded agent with its resolved mode and permission ruleset; `kilo debug agent <name>` shows one agent's resolved config.

    The default primary agent on launch is `code` (formerly `build`; the legacy key is still accepted and remapped to `code` by `KiloAgent.preprocessConfig` / `KiloAgent.resolveKey` in `kilocode/agent/index.ts`). Set `default_agent` in `kilo.json[c]` to override. Primary agents with `mode: "primary"` appear in the agent dropdown; agents with `mode: "subagent"` do not.

    Concurrency: a single tool call can spawn multiple subagents (the parent can call `task` repeatedly in one assistant turn). Per-agent concurrency is bounded by the `task` tool's foreground/background choice — background mode (`KILO_EXPERIMENTAL_BACKGROUND_SUBAGENTS=1`, `background: true`) decouples the child from the parent's tool-loop step and lets the parent fire multiple in parallel.
  parent_child_context: |
    Each subagent invocation runs in its own **session** with its own conversation history. The parent session keeps a record of the delegation through `Session.Info.parentID` set to the parent's `SessionID`; the child is a fresh session created via `sessions.create({ parentID: ctx.sessionID, title: "<description> (@<name> subagent)", ... })` in `packages/opencode/src/tool/task.ts`. The child session's `agent` field carries the child's agent name; the parent's `agent` field carries the parent's. The child's only return value to the parent is the final assistant text (wrapped in a `<task id="<sessionID>" state="completed"><task_result>...</task_result></task>` envelope) plus, on failure, a `resumeHint` text instructing the parent how to re-invoke with `task_id` to continue the same subagent session.

    The child receives:
    - its own frontmatter body (or `prompt` field) as the system prompt;
    - a fresh tool registry (the parent builds the child's tool map in `runTask` with `question: false`, `interactive_terminal: false`, `todowrite: false` when the child does not have a `todowrite` permission, and `task: false` when the child does not have a `task` permission);
    - the merged `permission` ruleset from `deriveSubagentSessionPermission` (parent session's `external_directory` and `deny` rules plus the parent agent's `edit` deny rules plus default `todowrite`/`task` denies);
    - the model resolved from the child's `model` frontmatter, the parent's `model` (when the child omits `model`), or the agent's own model passed via the `task` call (resolution handled by `KiloTask.resolveModel` in `kilocode/tool/task.ts`).

    The child does NOT receive the parent's conversation history, prior skill invocations, or files already read by the parent. There is no fork-mode analog of Claude Code's `CLAUDE_CODE_FORK_SUBAGENT`; the `task` tool always opens a fresh session.
  permissions_inheritance: |
    The child's permission ruleset is built by `deriveSubagentSessionPermission` (`packages/opencode/src/agent/subagent-permissions.ts`) from:
    1. the parent **agent's** `edit`-class deny rules (this is what makes Plan mode's `edit: "*": "deny"` propagate into subagents; Plan mode is enforced on the agent ruleset, not the session ruleset, so a subagent that only inherited the session's permission would silently bypass it);
    2. the parent **session's** `deny` rules and `external_directory` rules (forwarded as-is from the parent);
    3. default `todowrite: deny` and `task: deny` when the subagent's own ruleset does not already permit those.

    On top of that base, the child gets its own `permission` block from the agent definition (merged per `Permission.merge` — last matching rule wins). The subagent's `permission.task` block controls which subagent types the agent itself can spawn; `permission.task: { "*": "deny" }` removes the `task` tool from the child entirely, blocking nested delegation.

    `--auto` (and the TUI's auto-approve toggle) flips every `ask` to `allow` for the parent, which the child inherits. There is no concept of "trust mode" or sandbox inheritance by default — the optional experimental sandbox (`experimental.sandbox`, `experimental.sandbox_restrict_network`) covers model-originated shell commands and first-party HTTP tools but explicitly does NOT cover local MCP servers or plugin hooks.
  model_inheritance: |
    Resolution order, top wins:
    1. Per-call override on the parent's `task` invocation (e.g. `task(subagent_type="explore", model="anthropic/claude-haiku-4-5")` if the tool schema exposes a `model` parameter — Kilo's current `TaskTool` does not pass a `model` parameter through; the model is taken from the agent definition or the parent's model).
    2. Subagent definition's `model` frontmatter (`provider/model-id`).
    3. Invoking primary agent's model (the parent's `Session.Info.model`).
    4. Global `model` in `kilo.json[c]`.
    5. `--model` CLI override on the parent run.

    `variant` is independent of model resolution and is applied only when the agent's own `model` field is in use (per `KiloTask.resolveModel`). Provider-specific `options` (e.g. `reasoningEffort`, `textVerbosity`) are passed through to the model request verbatim.
  tool_inheritance: |
    Default: every tool the parent has access to, then narrowed by the child's merged `permission` ruleset. The child's `permission` block is an allowlist/denylist, not a tool-name allowlist; the keys are *permission classes* (e.g. `bash`, `edit`, `read`), not the underlying tool identifier. Wildcards and `*` are glob patterns matched against the tool's call input (e.g. for `bash`, the command string; for `edit`, the path glob). The default `task: false` and `interactive_terminal: false` keys apply to every child — subagents cannot ask the user a `question` or take over the user's terminal directly. MCP tools participate through the same permission classes with namespaced identifiers (`<server>_<tool>`).

    The `experimental.primary_tools` setting is the way to draw a hard line between primary and subagent tool sets: a list of permission keys listed in `experimental.primary_tools` is *removed* from every subagent's tool map entirely, regardless of what `permission` says. (Source: `packages/opencode/src/tool/task.ts` build step `Object.fromEntries((cfg.experimental?.primary_tools ?? []).map((item) => [item, false]))`.)
  max_turns: |
    Optional `steps` (positive integer) caps the number of agentic iterations before the agent is forced to respond with text only. When omitted, the agent iterates until it returns, hits an error, or exceeds the model's `MessageOutputLengthError`. There is no documented hard limit on nested delegation depth; `permission.task: { "*": "deny" }` on the parent agent is the practical way to bound it. Kilo's patches also add `KiloTask.nestedTask()` (a Kilo-only guard that blocks a subagent from spawning further subagents through the `task` tool); this is the practical max-depth control.
  notes: |
    Selection: automatic delegation is driven by the task prompt plus the subagent's `description` field. To make a subagent a strong candidate include phrases like "use proactively" or "must be used" in the description (parallels Claude Code's routing intent).

    Disabling: `disable: true` removes the agent from every discovery list, equivalent to deleting the file. Built-in agents cannot be removed by the user; organization-managed agents cannot be removed by the user (the `remove` function in `kilocode/agent/index.ts` raises `RemoveError("cannot remove organization agent — manage it from the cloud dashboard")` for `source === "organization"`).

    Failure: a non-zero exit, `stopReason: "error"`, or abort surface on the `TaskTool` result as `<task_error>` text. A child session that is `Busy` returns `SessionBusyError`. The TaskTool's `output` envelope (`<task id="<sessionID>" state="completed">...<task_result>...</task_result></task>`) is the wire format the parent parses; in the tool result it's the `output` field plus the metadata `{ parentSessionId, sessionId, model, variant, ... }`.

    Resume: a stopped or failed subagent can be resumed by re-invoking the `task` tool with `task_id: "<child-sessionID>"` and a follow-up prompt. The child session's permission ruleset is rebuilt at resume (Kilo's `merge` is idempotent and `SandboxPolicy.inherit` re-seeds the confinement from the parent); the resume re-uses the same child session ID, message thread, and any background processes. The `resumeHint` text appended to failed task outputs is a machine-readable signal: `This subagent session can be resumed: call the task tool again with task_id="<id>" and a prompt describing how to continue or recover.`

    Cost propagation: the child session's cost is snapshot before the task runs and propagated as a delta to the parent's session after the task completes (Kilo patch in `kilocode/session/cost-propagation.ts`); this works on foreground tasks, background tasks, and resume.

    Permission prompts raised by a subagent's tool call flow through the standard `kilo.permission.ask` surface on the *parent* session (the child does not have its own permission UI); the TUI's runtime auto-approve toggle covers subagent tool calls in lockstep with the parent's.

observability:
  stream_events:
    - "session.created (emitted on the bus when a subagent child session is created; the child's payload has parentID set to the parent's sessionID — the `Session.Info.parentID` is the primary correlation field)"
    - "session.updated (emitted on every patch to the child session; `Event.Updated` carries `info.parentID` so wrappers can still correlate to the parent)"
    - "session.deleted (emitted when the child session is closed; carries the full `Info` snapshot for the deletion event)"
    - "session.diff (bus event with the child's file diff at session close)"
    - "session.error (bus event with the child's `MessageV2.Assistant.fields.error` and any pre-message requirement failures; payload `error` is optional so a child that errored before producing a message still surfaces)"
    - "session.idle (emitted on the child when its processor loop finishes; correlates via the child's sessionID)"
    - "messageV2.Updated (emitted on every message in the child session; `Session.Event.Updated` schema carries `sessionID` + `info`)"
    - "messageV2.Removed / PartUpdated / PartRemoved / PartDelta (granular message lifecycle on the child)"
    - "background_job.started / completed / failed (when `KILO_EXPERIMENTAL_BACKGROUND_SUBAGENTS=1` and `background: true`; the job's metadata carries `parentSessionId` and `sessionId`)"
    - "(stream-json output mode) `kilo run --format json` emits the full bus event stream as raw JSON; this is the wire format wrappers can consume to observe child lifecycle in real time"
  hook_events: []
  session_ids: true
  notes: |
    Kilo does not emit a dedicated `subagent_start` / `subagent_stop` event pair. The stable signal is the child session's `parentID` on a `session.created` event. Each subagent has a stable `SessionID` (assigned at creation, descending timestamp order via `SessionID.descending()` in `session.ts`); the same ID is the `task_id` parameter used to resume the subagent. The child session is queryable through the same `/session/<id>/messages` HTTP API the parent uses (Kilo forks OpenCode's HTTP API, so `GET /session/:id/children` returns the full list of children for any session, and per-session event subscriptions over SSE identify the child via the `sessionID` field of each event payload).

    The CLI's `--format json` mode (`kilo run --format json --agent <name> "..."`) emits the raw bus event stream on stdout; this is the canonical surface for wrapper consumers. The `kilo run --debug` flag and `kilo debug info` show startup timing and resolved config including every agent.

    Permission prompts raised by a subagent's tool call flow through the parent's permission UI (the child does not have its own UI). The `AskUserQuestion` and `interactive_terminal` tools are stripped from subagents' tool maps at spawn time (`question: false`, `interactive_terminal: false`), so a subagent cannot trigger a direct user prompt — it can only call a regular tool whose permission prompt is lifted to the parent.

    Local `kilo agent list` on this host (macOS, Kilo CLI 7.3.45) returns the merged list with built-in and user-defined agents separated by their `native` flag:
    ```
    ask (primary)         # built-in (kilocode patch)
    code (primary)        # built-in (renamed from build by KiloAgent.patchAgents)
    compaction (primary)  # hidden system agent
    debug (primary)       # built-in (kilocode patch)
    Documenter (subagent) # user-defined via kilo.jsonc agent.<name>
    explore (subagent)    # built-in
    general (subagent)    # built-in
    orchestrator (primary)# built-in (deprecated=true)
    plan (primary)        # built-in
    summary (primary)     # hidden system agent
    title (primary)       # hidden system agent
    ```
    The custom agents (rust-developer, spec-writer, Documenter, feature-tester-rust, CLI Developer, rust-designer, code-simplifier, etc.) are loaded from the `kilo.jsonc` `agent` block in `~/.config/kilo/kilo.jsonc` — the host's user-scope config is a near-empty `{"$schema": "https://app.kilo.ai/config.json"}` (the `kilo agent list` output's `command` and `plugin` blocks visible in `kilo debug config` show that the user-defined agent blocks are loaded from a separate config file or an opencode-compat path; the agent listing confirms the load order in practice: built-ins are first, custom agents follow, and the merge is field-level).

    The host has symlinks in `~/.config/opencode/agents/` pointing to three Claude-authored agents (`feature-tester-rust.md`, `feature-tester-typescript.md`, `tester-agent.md`); Kilo's opencode-compat layer reads those, and `kilo agent list` reports the Claude-authored `feature-tester-rust` as a custom subagent with `mode: "all"` (its default frontmatter does not set a mode). This matches the upstream OpenCode loader's behavior of following symlinks and accepting partial AgentConfig entries.

portability:
  portable: false
  non_portable_assets:
    - "OpenCode/Kilo-only `mode` (`primary`/`subagent`/`all`) — Claude/Codex/Goose/Kimi/Qwen do not have an exact equivalent (Claude has `--agent` for whole-session replacement; the routing intent lives in `description`)"
    - "Kilo's `permission` keys (`bash`/`read`/`edit`/`glob`/`grep`/`webfetch`/`websearch`/`task`/`todowrite`/`todoread`/`lsp`/`skill`/`doom_loop`/`external_directory`/`question`/`suggest`/`interactive_terminal`/`plan_enter`/`plan_exit`/`repo_clone`/`repo_overview`/`recall`/`codebase_search`/`semantic_search`) — provider-specific vocabulary"
    - "Kilo's `permission.task` glob rule (the dedicated channel for gating which subagent types an agent may spawn)"
    - "Kilo-only `color` (`#RRGGBB` or `primary`/`secondary`/`accent`/`success`/`warning`/`error`/`info`)"
    - "Kilo-only `hidden` (removes from `@` autocomplete)"
    - "Kilo `variant` (provider-specific reasoning effort)"
    - "Kilo `options` (provider-passthrough) — every additional field is opaque to other providers"
    - "Kilo `steps` (max agentic iterations before text-only response)"
    - "Kilo `prompt` field with `{file:./prompts/build.txt}` shorthand and `{env:VAR}` substitution — syntax not portable to other providers"
    - "Kilo `experimental.primary_tools` (hard primary-only tool allowlist)"
    - "Kilo-specific Kilo-patch fields: `displayName`, `source` (one of `organization`/`global`/`project`), `deprecated`, `requirements` (Kilo-only, used to gate agent invocation on optional prerequisites)"
    - "Kilo `default_agent` setting and the `code` (formerly `build`) primary rename — provider-specific"
    - "Kilo's nested-task block (`KiloTask.nestedTask()` in `kilocode/tool/task.ts`) — subagents cannot spawn subagents in Kilo; this is a Kilo-only behavior, not present in upstream OpenCode"
    - "Built-in agent types `code`, `plan`, `ask`, `debug`, `orchestrator` (deprecated) plus hidden system agents `compaction`, `title`, `summary` — no equivalents on other providers"
    - "Built-in subagent types `general` (multi-step), `explore` (read-only), optional `scout` (experimental, gated behind `KILO_EXPERIMENTAL_SCOUT=1` or the `scout` flag)"
    - "Body prompt text that references Kilo tool names (`bash`, `edit`, `read`, `webfetch`, `task`, `todowrite`, `websearch`, `skill`, `lsp`), the `@<subagent>` mention syntax, or `$KILO_*` / `KILOCODE_*` env-var references"
    - "Auto-migrated `.kilocodemodes` / `custom_modes.yaml` legacy fields (`slug`, `roleDefinition`, `customInstructions`, `groups`, `whenToUse`) — Kilo-only, rewritten on first launch"
    - "Organization-managed agent metadata (`source: organization`, `displayName`) — Kilo Gateway only, not portable"
  rewrite_needed: true
  notes: |
    The Markdown body of a simple Kilo agent (description + role-played instructions) is the most portable piece and lifts cleanly to Claude Code subagents, Codex, or Gemini once the frontmatter is rewritten. The required `description` field is provider-neutral and carries the routing intent across implementations; it is the only field that survives a verbatim copy.

    A safe cross-provider rewrite preserves `description`, the body Markdown, the agent's identifier name (subject to the target provider's identifier grammar), and the `model` choice (when the target provider has the same model ID). It must drop or remap `mode` (translate to the target provider's mode/role vocabulary — Claude's subagents default to invocable from the Agent tool, OpenCode's `mode: all` is the analog), the entire `permission` block (remap each key to the target provider's vocabulary), `color` (remap palette), `hidden` (drop or remap to the target's `visibility` setting), `variant` (provider-specific reasoning effort), `options` (provider-passthrough; review each key), `steps` (remap to the target's turn-limit field), `prompt` substitutions, `experimental.primary_tools` (provider-specific), and any `permission.task` glob rules. Kilo-only `displayName`, `source`, `deprecated`, and `requirements` fields have no equivalent and must be stripped or mapped to the target provider's metadata keys.

    Body text that references Kilo tool names (`bash`/`edit`/`read`/`webfetch`/`task`/`todowrite`/`websearch`/`skill`/`lsp`) must be rewritten to the target provider's vocabulary — note that Claude Code's tool names are PascalCase (`Bash`, `Read`, `Edit`, `WebFetch`, `WebSearch`) and require case remapping.

    The Kilo JSON schema (`https://app.kilo.ai/config.json`) is the authoritative source for the `AgentConfig` shape and is what every editor's `$schema` reference points to (per the agent research at `claudine/docs/research/mcp/kilo.md` and the model research at `claudine/docs/research/agent-models/kilo.md`).

cli_params:
  - flag: --agent <name>
    description: "Select a primary agent for the session. Must reference an existing agent name from JSON `agent.<name>` or `.kilo/agents/<name>.md` / `~/.config/kilo/agents/<name>.md`. Works for any agent whose `mode` includes `primary` or `all`; rejected for agents declared `mode: subagent` with a clear error. Equivalent to setting `agent` in `kilo.jsonc` or `default_agent` for the next session."
    example: "kilo run --agent code-reviewer \"refactor auth\""
  - flag: kilo agent create [--path <dir>] [--description <text>] [--mode <all|primary|subagent>] [--permissions <csv>] [--model <provider/model>]
    description: "Interactive (or non-interactive with all flags) agent scaffolder. Writes a Markdown file with the right frontmatter; `permissions` is a comma-separated allowlist (`bash`,`read`,`edit`,`glob`,`grep`,`webfetch`,`task`,`todowrite`,`websearch`,`lsp`,`skill`); anything omitted is set to `deny`. `--path` accepts the parent directory; the file is written to `<path>/agents/<name>.md`. The `description` is passed to the agent's LLM-based generator (`Agent.generate`) which produces a structured `{identifier, whenToUse, systemPrompt}` payload; `whenToUse` becomes the frontmatter `description` and the body is `systemPrompt`. The agent's chosen `mode` is honored; if the mode includes `subagent`, the doc-page source confirms the resulting file works as a subagent definition."
    example: "kilo agent create --path .kilo --description \"Reviews code for security vulnerabilities\" --mode subagent --permissions read,grep,glob --model anthropic/claude-haiku-4-5"
  - flag: kilo agent list
    description: "List every loaded agent with its resolved mode and permission ruleset. Built-in (native) agents are listed first; user-defined agents follow in alphabetical order by name. Each entry prints `<name> (<mode>)` followed by the JSON-encoded `permission` array. Does not list hidden system agents (`compaction`, `title`, `summary`)."
    example: "kilo agent list"
  - flag: kilo debug agent <name>
    description: "Show one agent's resolved config (name, description, mode, options, permission, model, variant, hidden, steps, etc.) after the full config merge. Useful for debugging why a specific agent definition is loaded or skipped."
    example: "kilo debug agent explore"
  - flag: kilo debug config
    description: "Show the resolved configuration including every merged `agent` block, `mode` block, `plugin` array, `mcp` map, and `command` map. Used to verify which agents are actually loaded after the precedence merge."
    example: "kilo debug config"
  - flag: --model / -m <provider/model>
    description: "Override the global model for the session. Distinct from the per-agent `model` field; applies to whichever primary agent is selected unless that agent specifies its own `model`."
    example: "kilo run -m anthropic/claude-sonnet-4-5 \"explain closures\""
  - flag: --variant <name>
    description: "Model variant (provider-specific reasoning effort, e.g. `high`, `max`, `minimal`). Pairs with `--model`; not all providers accept every variant value."
    example: "kilo run --model anthropic/claude-sonnet-4-5 --variant high \"refactor this\""
  - flag: --continue / -c (and --session / -s <id>, --fork)
    description: "Continue or resume a prior session. Sessions are resumed by ID and retain their primary agent choice; --fork starts a new session branched at the chosen message. Combine with `--agent` to resume a session as a specific subagent (the agent's system prompt replaces the default)."
    example: "kilo run --continue --agent code-reviewer"
  - flag: --format default|json
    description: "Output format for `kilo run`. `default` is the human-readable formatted output; `json` is the raw bus event stream (one JSON object per line on stdout), the canonical wire format for wrapper consumers observing subagent lifecycle."
    example: "kilo run --format json --agent explore \"list tsx files\""
  - flag: --auto
    description: "Auto-approve permissions that are not explicitly denied. Equivalent to flipping every `ask` to `allow` in the merged `permission` config; affects both primary and child sessions. Does NOT affect explicit `deny` rules."
    example: "kilo run --auto \"refactor auth\""
  - flag: --dangerously-skip-permissions
    description: "Alias for `permission: \"allow\"` at the session level — auto-approves all permissions, bypassing the permission engine entirely. Used in CI/automation; cascades to all spawned subagents."
    example: "kilo run --dangerously-skip-permissions \"smoke test\""
  - flag: --print-logs
    description: "Print Kilo's internal logs to stderr in addition to the log file. Useful for debugging subagent permission or model resolution issues."
    example: "kilo run --print-logs --agent explore \"find files\""
  - flag: --log-level <DEBUG|INFO|WARN|ERROR>
    description: "Set the log level for the session. The session log is written to `<data>/log/<session>/...` and `--log-level DEBUG` is required to see per-agent resolution detail."
    example: "kilo run --log-level DEBUG --agent code \"explain this\""

env_vars:
  - name: KILO_CONFIG
    effect: "Path to a custom `kilo.json[c]` file loaded between global and project configs (config precedence step 3). Carries `agent.<name>` blocks and can override agents from disk. Also accepts `KILO_CONFIG_DIR` (the directory form)."
  - name: KILO_CONFIG_CONTENT
    effect: "Raw JSON config applied session-wide, including to child/subagent sessions (highest non-managed precedence). Carries `agent.<name>` blocks; ideal surface for wrappers that want to inject an agent without writing a file. Source is tagged as `local` precedence. (Verified: see `claudine/docs/research/mcp/kilo.md` runtime_injection section.)"
  - name: KILO_CONFIG_DIR
    effect: "Custom directory searched for `agents/`, `commands/`, `modes/`, `skills/`, and `plugins/`. Loaded after global and `.kilo` directories, so it can override their settings. (Mirrors OpenCode's `OPENCODE_CONFIG_DIR`.)"
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: "Skip loading `kilo.jsonc` and `.kilo/` from the project hierarchy. Used by sandboxed or wrapper-driven runs; preserves the global + managed + inline precedence."
  - name: KILO_PERMISSION
    effect: "Inlined JSON `permission` config, merged over the loaded config."
  - name: KILO_DISABLE_DEFAULT_PLUGINS
    effect: "Disable bundled default plugins (`@kilocode/kilo-indexing`, `@kilocode/plugin-atomic-chat`, etc.). Affects plugin-contributed agents only; does not affect agents declared in `kilo.jsonc` or Markdown files."
  - name: KILO_DISABLE_EXTERNAL_SKILLS
    effect: "Disable loading skills from outside the config tree. Independent of agent discovery."
  - name: KILO_EXPERIMENTAL_BACKGROUND_SUBAGENTS
    effect: "When set, enables the `background: true` parameter on the `task` tool, decouples subagent spawns from the parent's tool-loop step, and allows multiple subagents to run in parallel. When unset, the `task` tool's `background: true` parameter raises `Background subagents require KILO_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true`."
  - name: KILO_EXPERIMENTAL_SCOUT
    effect: "Enable the built-in `scout` subagent (external-doc/dependency research with `repo_clone` + `repo_overview` + read-only web tools). Without this flag, `scout` is omitted from the loaded agent list — observed locally on v7.3.45."
  - name: KILO_EXPERIMENTAL
    effect: "Umbrella flag that flips unstable defaults on."
  - name: KILO_EXPERIMENTAL_LSP_TOOL
    effect: "Enables the experimental LSP tool surface. Affects the `lsp` permission class available to agents."
  - name: KILO_ORG_ID
    effect: "Routes `kilo run` through the specified Kilo organization (Team/Enterprise). Used by non-interactive CI environments; persisted selection from `/teams` is used as a fallback."
  - name: KILO_PROVIDER
    effect: "Override the active provider ID (e.g. `kilo`, `anthropic`, `kilocode`). Pairs with `KILO_<FIELD>` overrides for the active provider."
  - name: KILO_<FIELD>
    effect: "Override individual config fields by name (e.g. `KILO_MODEL` sets `model`). Provider-specific."
  - name: KILOCODE_<FIELD>
    effect: "Override config fields for the `kilocode` provider specifically (e.g. `KILOCODE_MODEL`, `KILOCODE_API_KEY`)."
  - name: KILO_CLIENT
    effect: "Identifies the calling client (defaults to `cli`); used in telemetry and per-agent configuration toggles (e.g. native_notebook_tools is only enabled when `KILO_CLIENT === \"vscode\"`)."
  - name: KILO_TUI_CONFIG
    effect: "Path to a custom `tui.json[c]` file. Controls TUI-only settings (notifications, sounds, themes, keybindings)."
  - name: KILO_BWRAP_PATH
    effect: "Path to the bubblewrap binary used by the optional Linux sandbox backend. Does not affect agent discovery but matters for whether the parent (and by extension subagent) bash commands run sandboxed."
  - name: KILO_TEST_HOME
    effect: "Test-only override for `Global.Path.home`; not intended for production."
  - name: KILO_TEST_MANAGED_CONFIG_DIR
    effect: "Test-only override for the managed config directory; not intended for production."
  - name: KILO_SERVER_PASSWORD
    effect: "Basic auth password for the `kilo serve` / `kilo web` headless server."
  - name: KILO_SERVER_USERNAME
    effect: "Basic auth username for the headless server (defaults to `kilo`)."
  - name: OTEL_EXPORTER_OTLP_ENDPOINT
    effect: "If set, `kilo` exports OpenTelemetry traces and logs to that OTLP HTTP endpoint. Request spans include `http.method`, `http.path`, route params such as `session.id` and `message.id`, and internal params under the `opencode.*` namespace. Per-agent spans and the child session's tool calls show up under the child's `session.id`."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `claudine agents` link command does not yet recognize Kilo Code's `agents/` discovery surface (`~/.config/kilo/agents/<name>.md`, `.kilo/agents/<name>.md`, `$KILO_CONFIG_DIR/agents/<name>.md`, and the JSON `agent.<name>` block in `kilo.json[c]`/`opencode.json[c]`/`KILO_CONFIG_CONTENT`). A future Kilo agent-linker entry must:

  1. Walk the four filesystem scopes (global, project walk-up, custom config dir, managed/system) and parse Markdown frontmatter using the canonical `AgentConfig` schema from `https://app.kilo.ai/config.json`.
  2. Read the JSON `agent` block from every config file in precedence order and merge it with the Markdown definitions (field-level merge, not whole-block).
  3. Apply Kilo's resolution rules: the `build` key is silently remapped to `code` by `KiloAgent.preprocessConfig`; both `agent/` and `agents/` are accepted; nested directory names register as `<dir>/<name>`; `displayName`/`source` metadata in frontmatter or in the `options` object is lifted to typed fields and stripped from `options` (per `KiloAgent.processConfigItem`); `requirements` is a Kilo-specific gate field.
  4. Distinguish `mode: primary` / `mode: subagent` / `mode: all`; identify hidden system agents (`compaction`, `title`, `summary`) which are not invokable through the `task` tool.
  5. Surface the resolved `permission` ruleset, `model`, `variant`, `steps`, and the `{file:./...}` prompt substitutions; flag assets that depend on `permission.task` glob rules, `experimental.primary_tools`, `color`, `hidden`, `variant`, `options`, `displayName`, `source`, or `requirements` as needing rewrite when linking to another provider.
  6. Honor the Kilo-only hard-block on subagent nesting (`KiloTask.nestedTask()` in `kilocode/tool/task.ts`): a linked subagent's `permission.task: { "*": "deny" }` is the Kilo-side enforcement; Claudine's wrapper does not need to add its own depth check.
  7. Update the linking-classification table to record that Kilo agents carry a provider-neutral `description` plus a provider-specific `mode`/`permission`/`color`/`hidden`/`variant`/`options`/`steps` set, plus the Kilo-only `displayName`/`source`/`deprecated`/`requirements` metadata — portable only on the body, never on the frontmatter.

  For lifecycle `proxy`/`resume`: Kilo does not emit a `subagent_start`/`subagent_stop` pair. The stable signal is the child session's `parentID` on a `session.created` event, plus the `<task id="<sessionID>" state="...">` envelope the TaskTool writes to the parent's tool result. A wrapper that wants to address a specific subagent should keep the `task_id` (the child's `SessionID`) and use the `task` tool's `task_id` parameter on a subsequent call to resume that exact session. Kilo's resume is more powerful than OpenCode's: the child session's permission ruleset, model, variant, and tool map are all re-derived at resume time, so a `proxy` action can patch any of them between invocations by calling `sessions.setPermission` / `sessions.setMetadata` on the child session ID before the `task` resume. Kilo's `KiloTaskBackgroundProcess.finish(nextSession.id)` in the `Effect.ensuring` block means background processes (terminal sessions, etc.) are transferred to the parent on child exit — a `proxy` action that wants to take over a child must either stop those processes or pass them through.

  The `kilo run --format json` flag is the canonical wrapper surface. The full bus event stream is one JSON object per line on stdout; events for the child session carry `sessionID` + `parentID` so a wrapper can correlate child lifecycle to parent lifecycle without guessing.
---

# Kilo Code Subagents

## Overview

Kilo Code treats user-defined **agents** as a first-class feature with two concrete flavors — **primary agents** (the main assistants the user interacts with directly, switched with the agent dropdown or `--agent`) and **subagents** (specialized assistants that a primary agent invokes through the `task` tool or that the user invokes via `@<name>` mentions). The provider calls the whole feature "agents" or "subagents" depending on the doc page; the in-source module name is `packages/opencode/src/agent/` plus the Kilo-specific patches in `packages/opencode/src/kilocode/agent/`. The internal data model is `Agent.Info` with a strict `Schema.Struct` enforced via Effect.

Kilo CLI 1.0 ships built-in primary agents `code` (renamed from `build`), `plan`, `ask`, `debug`, `orchestrator` (deprecated, hidden behind a `deprecated: true` flag), and hidden system agents `compaction`, `title`, `summary`; built-in subagents `general` (multi-step), `explore` (read-only), and `scout` (experimental, gated behind `KILO_EXPERIMENTAL_SCOUT=1`). All built-in agents can be overridden by a user-scope or project-scope definition with the same name (the merge is field-level, not whole-block). Support is `first_class`: there are documented scopes (built-in, global, project, custom config dir, managed/system, plugin, remote `.well-known/opencode`, organization-managed via the cloud), a stable frontmatter schema, runtime delegation semantics through the `task` tool, isolated child sessions with their own `parentID` lineage, an explicit `task_id`-based resume mechanism, and observable start/stop lifecycle through the bus event stream.

This topic is the *definition* of agents and subagents — where the files live, what frontmatter they accept, how the parent picks one, what context and permissions the child gets, and how a wrapper can observe starts/stops. The hooks topic owns lifecycle event semantics (this document records only which stream/bus events expose agent lifecycle). The plugins topic's `packaged_resources` records containment only — agent definitions packaged inside plugins still have their semantics documented here.

The Kilo agent system is a fork of OpenCode's `agent` mechanism (paths renamed from `opencode` to `kilo`, the `KiloAgent.patchAgents` patch adds the Kilo-specific primary agents `code`/`plan`/`debug`/`ask`/`orchestrator` and renames `build` to `code`). The schema is intentionally byte-for-byte compatible with `opencode.json[c]` for shared fields, and the Kilo-specific `displayName`/`source`/`deprecated`/`requirements` are added as typed fields on top of the standard `AgentConfig`. The full `AgentConfig` schema is published at `https://app.kilo.ai/config.json` and the Kilo JSON schema URL is what every editor's `$schema` reference points to.

## Locations

Definitions are stored by scope. Kilo's config precedence is well-defined: remote (`.well-known/opencode`) → global (`~/.config/kilo/kilo.jsonc`) → custom (`KILO_CONFIG`) → project (`kilo.jsonc` / `.kilo/kilo.jsonc`) → `.kilo`/`.kilocode` directory walk for agents/commands/plugins/skills → inline (`KILO_CONFIG_CONTENT`) → file-based managed config (`/Library/Application Support/kilo/`, `/etc/kilo/`, `%ProgramData%\kilo\`) → macOS MDM preferences (`ai.opencode.managed` plist). For *agent* definitions specifically, the documentation orders them as: built-in → global config → project config → global Markdown → project Markdown. The Kilo-specific `.kilocode/` and `custom_modes.yaml` legacy directories are also read for auto-migration on first launch.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| Built-in (system) | hardcoded in `packages/opencode/src/agent/agent.ts` (with Kilo patches in `kilocode/agent/index.ts`) | same | same | `code`, `plan`, `ask`, `debug`, `orchestrator` (deprecated) as primary; `general`, `explore`, `scout` (experimental) as subagents; hidden system agents `compaction`, `title`, `summary`. NOT on disk in the user's environment. Lowest precedence. |
| User (config) | `~/.config/kilo/kilo.jsonc` (or `kilo.json`) | same | `%APPDATA%\kilo\kilo.json[c]` | Personal agents applied across all projects. The `agent.<name>` key carries inline JSON agent definitions. |
| User (markdown) | `~/.config/kilo/agent/<name>.md` or `~/.config/kilo/agents/<name>.md` | same | `%APPDATA%\kilo\agent\<name>.md` or `%APPDATA%\kilo\agents\<name>.md` | Personal agents applied across all projects. Both `agent/` (singular) and `agents/` (plural) are walked. Nested directories register as `<dir>/<name>`. |
| Project (markdown) | `.kilo/agent/<name>.md` or `.kilo/agents/<name>.md` | same | `.kilo\agent\<name>.md` or `.kilo\agents\<name>.md` | Project-scope agents. The path is relative to the session's project worktree. Default target for `kilo agent create` when scope is `project` and the worktree is a git repo. |
| Legacy | `.kilocode/agents/<name>.md` and `~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code/settings/custom_modes.yaml` (or `%APPDATA%\Code\User\globalStorage\...` on Windows, `~/.config/Code/...` on Linux) | same | same | Legacy VS Code extension locations; auto-migrated on first launch via `ModesMigrator`. Migrated agents become Markdown files in the standard `agents/` directory. |
| Custom config dir | `$KILO_CONFIG_DIR/agents/<name>.md` and `$KILO_CONFIG_DIR/kilo.json[c]` | same | `$KILO_CONFIG_DIR\agents\<name>.md` | Set via `KILO_CONFIG_DIR`; searched for `agents/`, `commands/`, `modes/`, `plugins/`, and `skills/` like a standard `.kilo` directory. Loaded after global and `.kilo` directories, so it can override their settings. |
| Inline overlay | `KILO_CONFIG_CONTENT` env var (JSON string) | same | same | Session-only. Tagged as `local` source (above project configs, below remote and managed sources). Carries `agent.<name>` blocks. Highest non-managed precedence. |
| Remote | `<provider>/.well-known/opencode` | same | same | Fetched when authenticating with a remote provider (e.g. Kilo Gateway) that supplies a `type: wellknown` credential entry. Loaded first as the base layer; may ship `agent` entries with `disable: true`. |
| Managed / system (file) | `/Library/Application Support/kilo/opencode.json` | `/etc/kilo/opencode.json` | `%ProgramData%\kilo\opencode.json` | Admin-controlled `opencode.json` or `.jsonc` (Kilo kept the OpenCode-style path with the `kilo` directory rename). Highest file-based precedence. |
| Managed / system (MDM) | `/Library/Managed Preferences/<user>/ai.opencode.managed.plist` | n/a | n/a | macOS MDM `PayloadType=ai.opencode.managed`. Payload keys map 1:1 to `kilo.json[c]` fields, so an admin can ship a managed `agent` allowlist. Highest precedence overall. |
| Plugin | `<plugin>/agents/<name>.md` or plugin-contributed `agent.<name>` config block | same | same | Plugins can ship agent definitions; bundled defaults include `@kilocode/kilo-indexing` and `@kilocode/plugin-atomic-chat`. Toggle bundled defaults with `KILO_DISABLE_DEFAULT_PLUGINS`. Organization-managed agents distributed through the cloud arrive as `source: "organization"` on the resolved `Info`; users cannot remove them. |

The Kilo config loader walks each scope for both Markdown files (`{agent,agents}/<name>.md` and nested paths) and JSON config files, then merges them by name. Singular `agent/` is also accepted as a documented alternative to `agents/`.

On this host (macOS, Kilo CLI 7.3.45, source at `kilo --version`):

- `kilo debug paths` reports:
  - `home` = `/Users/ken`
  - `data` = `/Users/ken/.local/share/kilo`
  - `config` = `/Users/ken/.config/kilo`
  - `tmp` = `/var/folders/l9/xdcp3xnn6s78_5l9w2_mnvtw0000gn/T/kilo`
  - `log` = `/Users/ken/.local/share/kilo/log`
  - `repos` = `/Users/ken/.local/share/kilo/repos`
  - `cache` = `/Users/ken/.cache/kilo`
  - `bin` = `/Users/ken/.cache/kilo/bin`

- `~/.config/kilo/kilo.jsonc` is a near-empty `{"$schema": "https://app.kilo.ai/config.json"}`; the user-scope agent definitions visible in `kilo agent list` (`Documenter`, `rust-developer`, `spec-writer`, `feature-tester-rust`, `CLI Developer`, `rust-designer`, `code-simplifier`, `just-scripter`, `skill-crafter`, `research-writer`, `research-consolidator`, `risk-assessor`, etc.) are loaded from the global config's `agent` block as resolved by `kilo debug config`. The agent listing confirms both the built-in and user-defined agent sets are merged and sorted with built-ins (native: true) first.

- `~/.config/opencode/agents/` contains three symlinks (`feature-tester-rust.md`, `feature-tester-typescript.md`, `tester-agent.md`) pointing to `~/.claude/agents/*.md`. Kilo's opencode-compat layer reads those and `kilo agent list` reports the Claude-authored `feature-tester-rust` as a custom subagent with `mode: "all"` (its default frontmatter does not set a mode). This matches the upstream OpenCode loader's behavior of following symlinks and accepting partial `AgentConfig` entries.

- `kilo agent list` output (mode-filtered):
  ```
  ask (primary)         # Kilo-specific built-in (kilocode/agent/index.ts:patchAgents)
  code (primary)        # Kilo rename of upstream "build"; the legacy key is silently remapped
  compaction (primary)  # hidden system agent (hard-coded `*`: "deny" permission)
  debug (primary)       # Kilo-specific built-in
  Documenter (subagent) # user-defined via kilo.jsonc agent.<name>
  explore (subagent)    # upstream built-in (Kilo adds codebase_search + conditional prompt)
  general (subagent)    # upstream built-in
  orchestrator (primary)# upstream built-in; marked deprecated: true by Kilo
  plan (primary)        # upstream built-in (Kilo adds .kilo/ paths to planEditRules)
  summary (primary)     # hidden system agent
  title (primary)       # hidden system agent
  ```
  Built-ins are listed first; custom agents follow in alphabetical order. The `scout` agent is absent because `KILO_EXPERIMENTAL_SCOUT` is not set on this host.

## Definition Format

The two declaration surfaces are equivalent. JSON entries map to the same `AgentConfig` shape as Markdown frontmatter.

### Markdown form

A Markdown agent is a single file with YAML frontmatter. The basename (without `.md`) becomes the agent name unless the frontmatter carries an explicit `name:` field. Nested directories register as `<dir>/<name>` per the docs.

```markdown
---
description: Reviews code for quality and best practices. Use proactively after non-trivial code changes.
mode: subagent
model: anthropic/claude-sonnet-4-20250514
temperature: 0.1
permission:
  edit: deny
  bash: deny
  webfetch: allow
color: accent
---

You are a code reviewer. Analyze code for:

- Code quality and best practices
- Potential bugs and edge cases
- Performance implications
- Security considerations

Provide constructive feedback without making direct changes.
```

When the agent is also the active primary agent for the session, the Markdown body becomes the system prompt in place of the provider's stock prompt; the system-prompt topic documents the replacement semantics in detail.

### JSON form

Inline in `kilo.jsonc` (or `.jsonc`):

```jsonc
{
  "$schema": "https://app.kilo.ai/config.json",
  "agent": {
    "code-reviewer": {
      "description": "Reviews code for best practices and potential issues",
      "mode": "subagent",
      "model": "anthropic/claude-sonnet-4-20250514",
      "prompt": "You are a code reviewer. Focus on security, performance, and maintainability.",
      "permission": {
        "edit": "deny",
        "bash": "deny"
      }
    },
    "explore": {
      "model": "anthropic/claude-haiku-4-5"
    }
  }
}
```

`prompt` accepts inline text, `{file:./relative/path}` shorthand (resolved relative to the config file's directory), or `{env:VAR}` substitution. Unrecognized frontmatter fields fall through to the provider as model options (e.g. `reasoningEffort`, `textVerbosity`).

### Recognized fields

| Field | Required | Behavior |
|---|---|---|
| `description` | Yes | Routing signal; the orchestrator uses the description to decide which subagent to invoke. |
| `mode` | No | One of `primary` (user-selectable), `subagent` (only invocable via `task` tool or `@<name>`), or `all` (both). Default `all` for custom agents. |
| `model` | No | `provider/model-id` string; overrides the agent's default model. Subagents without a `model` field inherit the invoking primary agent's model. |
| `prompt` | No | System prompt text or `{file:./path}` shorthand. In Markdown form, the body is the prompt when `prompt` is not set; setting both keeps `prompt` and discards the body. |
| `temperature` | No | 0.0-1.0. |
| `top_p` | No | 0.0-1.0. |
| `permission` | No | Object; values are `"allow" | "ask" | "deny"` or, for `bash`/`read`/`edit`/`glob`/`grep`/`lsp`/`skill`/`external_directory`, a glob→action map. Rules are evaluated last-match-wins. |
| `hidden` | No | Boolean; removes the agent from the `@` autocomplete menu. Only meaningful when `mode: subagent`. |
| `steps` | No | Positive integer; max agentic iterations before the agent is forced to respond with text only. |
| `color` | No | Hex `#RRGGBB` or theme name: `primary`/`secondary`/`accent`/`success`/`warning`/`error`/`info`. |
| `disable` | No | Boolean; removes the agent from every discovery list, equivalent to deleting the file. |
| `variant` | No | Provider-specific reasoning effort paired with `model` (e.g. `high`, `max`, `minimal`). |
| `options` | No | Object passed through to the provider as model options. |
| `name` | No | Explicit identifier; overrides the filename-derived name (per `KiloAgent.processConfigItem`). |
| `displayName` | No | Kilo-specific; human-readable name shown in the agent picker for org-supplied modes. Read from frontmatter or lifted from `options.displayName` then stripped from `options`. |
| `source` | No | Kilo-specific; one of `organization`/`global`/`project`. Origin marker; gates the removal guard (`RemoveError("cannot remove organization agent — manage it from the cloud dashboard")`). |
| `deprecated` | No | Kilo-specific; marks an org-supplied mode as deprecated. |
| `requirements` | No | Kilo-specific; a `kilocode/agent-requirements` schema describing optional prerequisites that gate agent invocation (e.g. required skills, MCP servers, or other agents). |

The body of a Markdown agent file is itself stored as the agent's system prompt when `prompt` is not set; setting both keeps `prompt` and discards the body. `displayName` and `source` inside the `options` object are lifted to typed fields and stripped from `options` at merge time by `KiloAgent.processConfigItem` to keep `options` provider-clean at the source.

## Runtime Behavior

A subagent is invoked by the primary agent with the native `task` tool. The tool's parameters are `subagent_type` (the agent name; also accepts `task_id` to resume, `background: true` for asynchronous mode, `command` to flag a code-review command, and `description` for the parent's transcript label). The parent's session then emits a `subtask`-like invocation through the tool registry, and Kilo's `TaskTool` source at `packages/opencode/src/tool/task.ts` creates a child session with `parentID: ctx.sessionID`, builds a fresh tool registry with `question: false` and `interactive_terminal: false`, and returns the final assistant text to the parent wrapped in a `<task id="<sessionID>" state="..."><task_result>...</task_result></task>` envelope.

Three other invocation paths reach a subagent:

1. **User `@<name>` mention**. Typing `@<name>` in the TUI prompt or chat input injects the subagent's prompt as a `@`-style reference and routes the message to it. Hidden subagents (`hidden: true`) are excluded from autocomplete but are still callable through paths 2 and 3.
2. **Autonomous selection**. The primary agent picks a subagent when the user's prompt matches one of the loaded `description` fields. The description should include phrases like "use proactively" or "must be used" for strong-candidate routing.
3. **CLI direct**. `kilo --agent <name> <project>` (or `kilo run --agent <name> "..."`) starts the session as the named agent; this works for any agent whose `mode` includes `primary` or `all`, and not for agents declared `mode: subagent`.

Selection is gated by `permission.task` on the calling agent. A glob rule like `permission.task: { "*": "deny", "code-reviewer": "allow" }` removes the denied agents from the task-tool description entirely (the model can't even attempt them) and surfaces an `ask` prompt for the others. Last matching rule wins. Kilo also adds a hard nesting block via `KiloTask.nestedTask()` in `kilocode/tool/task.ts`: a subagent that tries to call the `task` tool itself gets the `task: false` flag stripped from its tool map, so subagents cannot spawn further subagents regardless of their `permission.task` configuration.

Each child runs in its own session with its own message history, permission resolver, and tool allowlist. The child does **not** reload `agents.md` or re-walk the project `.kilo` discovery tree; the system-prompt research (`claudine/docs/research/system-prompt/opencode.md`) confirms that only `KILO_CONFIG_CONTENT` propagates into child sessions. The child's resolved permission is built by `deriveSubagentSessionPermission` from the parent session's `deny` rules and `external_directory` rules plus the parent agent's `edit`-class deny rules, then merged with the child's own `permission` block. `KILO_EXPERIMENTAL_BACKGROUND_SUBAGENTS=1` lets the parent fire multiple subagent calls in parallel without waiting for each one; the `background: true` parameter on the `task` tool creates a `BackgroundJob` that runs detached from the parent's tool-loop step.

A child session's `parentID` is the parent's `SessionID`; the child's `agent` field carries the child's agent name; the parent's `agent` field carries the parent's. The child session's `SessionID` is the `task_id` parameter used to resume the subagent in a later call — the resume re-uses the same child session ID, message thread, and any background processes. The `resumeHint` text appended to failed task outputs is a machine-readable signal that the wrapper can parse: `This subagent session can be resumed: call the task tool again with task_id="<id>" and a prompt describing how to continue or recover.`

## Observability

Kilo does not emit a dedicated `subagent_start` / `subagent_stop` pair. Wrappers detect subagent lifecycle through three coordinated signals:

1. **Subtask envelope on the parent**. When the parent calls the `task` tool, the parent session emits a tool result whose `output` field carries the `<task id="<sessionID>" state="completed">...<task_result>...</task_result></task>` envelope. The `<task_result>` is the child's final assistant text. On failure, the envelope's state is `error` and the body is the error message (with the `resumeHint` appended). On background mode, the envelope is `<task id="<sessionID>" state="running">...<summary>Background task started</summary>...<task_result>Background task started. You will be notified automatically when it finishes; do not poll for progress.</task_result></task>` and the parent's tool result carries the background job ID in metadata.
2. **Child session creation**. A `session.created` event whose payload's `Session.Info.parentID` is non-empty identifies a new child/subagent session. The child session's `Session.id` stays stable for the lifetime of the subagent and is the same ID used in `POST /session/:id/message`, `POST /session/:id/abort`, and the `task_id` parameter on resume. The child session's `agent` field carries the agent name.
3. **Per-session status and idle**. Each session has its own `session.idle` event that fires when its processor loop finishes. Wrappers identify the child's idle by joining on `sessionID` from the `session.created` payload. The `session.error` event surfaces on the child session ID.

The bus exposes the same `Event` union over the SSE bus (`/event` and `/global/event`); both `/session` endpoints (`POST /session`, `POST /session/:id/init`, `POST /session/:id/fork`, `POST /session/:id/abort`, `POST /session/:id/permissions/:permissionID`) are session-level, not subagent-level — wrappers use the *child's* session ID to address the subagent. There is no hook script system analogous to Claude Code hooks. Kilo does have the `kilo serve` / `kilo web` headless server with `kilo acp` for ACP-mode integration, but no per-subagent hook layer.

Permission prompts raised by a subagent's tool call flow through the standard `kilo.permission.ask` surface on the *parent* session (the child does not have its own permission UI); the TUI's runtime auto-approve toggle covers subagent tool calls in lockstep with the parent. The `AskUserQuestion` and `interactive_terminal` tools are stripped from subagents' tool maps at spawn time (`question: false`, `interactive_terminal: false` in the tool map), so a subagent cannot trigger a direct user prompt — it can only call a regular tool whose permission prompt is lifted to the parent.

The `kilo run --format json` flag is the canonical wrapper surface. It emits the full bus event stream as raw JSON, one object per line on stdout, including the `session.created` / `session.updated` / `session.idle` / `session.error` events with their `sessionID` and `parentID` fields. Pair with `kilo debug config` to see the resolved agent set before launching.

Local `kilo agent list` on this host confirms the loader's partial-acceptance behavior: the host's `kilo.jsonc` `agent` block defines agents with only `name` and `description` (and sometimes a long `prompt` body), and they are registered with their declared `mode` (e.g. `Documenter` registers as `subagent`; `code-simplifier` registers with no mode so defaults to `all`). The custom agents are sorted after the built-ins by alphabetical name; the listing also shows the resolved `permission` ruleset as JSON.

## Portability

Kilo agents are **not portable** across providers as-is. The body Markdown is the most portable piece and lifts cleanly to Claude Code subagents, Codex, or Gemini once the frontmatter is rewritten. The required `description` field is provider-neutral and carries the routing intent across implementations. The rest of the frontmatter is provider-specific.

| Field | Portable? | Rewrite target |
|---|---|---|
| `description` | yes | Carries the routing signal across providers. |
| `mode` | no | Remap to the target provider's mode/role vocabulary (`subagent` ↔ subagent / `--agent`, `primary` ↔ main-thread replacement, `all` ↔ both). |
| `model` | depends | Same `provider/model` ID only if the target provider also exposes it; otherwise map to the target's model identifier. |
| `prompt` | depends | Strip `{file:...}` / `{env:...}` substitutions; verify body text doesn't reference Kilo-only tool names. |
| `temperature`, `top_p` | depends | Most providers accept these names. |
| `variant` | no | Kilo-only; map to the target's reasoning-effort field. |
| `permission` | no | Remap each key to the target provider's permission vocabulary. Drop `permission.task` (Kilo-only), `external_directory`, `doom_loop`, `recall`, `repo_clone`, `repo_overview`, `codebase_search`, `semantic_search`, `interactive_terminal`, `suggest`, `plan_enter`, `plan_exit`, `question`, `skill`. |
| `color` | no | Remap to the target provider's color palette. |
| `hidden` | no | Drop or remap; some providers have no `@mention` UI. |
| `steps` | no | Remap to the target provider's turn-limit field (e.g. Claude's `maxTurns`). |
| `options` | no | Provider-specific passthrough; review each key. |
| `disable` | depends | Some providers have an `enabled`/`disable` flag; otherwise drop. |
| `name` | depends | Subject to the target provider's identifier grammar. |
| `displayName`, `source`, `deprecated`, `requirements` | no | Kilo-only; no direct equivalent on other providers. |
| Body Markdown | partial | Body is portable when it references standard Markdown; rewrite references to Kilo-only tools (`bash`, `edit`, `read`, `webfetch`, `task`, `todowrite`, `websearch`, `skill`, `lsp`) and to `$KILO_*` / `KILOCODE_*` env-var names. |

The Kilo JSON schema (`https://app.kilo.ai/config.json`) is the authoritative source for the `AgentConfig` shape and is what every editor's `$schema` reference points to. Plugins that ship agents through their `plugin` array are even less portable because they couple the agent to plugin-only hooks/tools.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy`/`resume` actions, what matters about Kilo Code agents:

- **Discovery surfaces to enumerate** for the Kilo agent linker:
  1. `~/.config/kilo/agents/<name>.md` and `~/.config/kilo/agent/<name>.md` (user-scope, follows symlinks).
  2. Every `.kilo/agents/<name>.md` and `.kilo/agent/<name>.md` from CWD up to the git worktree (project-scope).
  3. `$KILO_CONFIG_DIR/agents/<name>.md` (custom config dir).
  4. The JSON `agent.<name>` block in `~/.config/kilo/kilo.jsonc`, `kilo.jsonc` / `kilo.json` in the project, `$KILO_CONFIG_DIR/kilo.jsonc`, and the file pointed to by `KILO_CONFIG`.
  5. The `agent.<name>` block inside `KILO_CONFIG_CONTENT` (session-only overlay; effectively ephemeral).
  6. The managed-system files at `/Library/Application Support/kilo/opencode.json` (macOS), `/etc/kilo/opencode.json` (Linux), `%ProgramData%\kilo\opencode.json` (Windows), plus the macOS MDM `.mobileconfig` plist at `/Library/Managed Preferences/ai.opencode.managed.plist`.
  7. The legacy `.kilocode/agents/<name>.md` and `~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code/settings/custom_modes.yaml` (read for auto-migration only — the result is a Markdown file in the standard `agents/` directory after migration runs).
  8. Plugin-contributed agents (toggle with `KILO_DISABLE_DEFAULT_PLUGINS`; ship defaults `@kilocode/kilo-indexing` and `@kilocode/plugin-atomic-chat`).
- **Listing command**: `kilo agent list` returns every loaded agent with its resolved mode (one of `primary` / `subagent` / `all`) and the full JSON-encoded `permission` ruleset. Use this as the listing source instead of re-walking the filesystem twice; it already accounts for config merging and plugin contributions. The companion `kilo debug agent <name>` shows one agent's resolved config in detail.
- **Portability classification**: classify Kilo agents as **non-portable**. The Markdown body carries most of the agent's *purpose* and may be lifted as-is to another provider's body, but the frontmatter is Kilo-specific. Flag assets that depend on `mode`, `permission`, `permission.task`, `color`, `hidden`, `variant`, `options`, `steps`, `displayName`, `source`, `deprecated`, `requirements`, `prompt` substitutions, or `experimental.primary_tools` as needing rewrite, stripping, or host gating before they can land elsewhere.
- **Lifecycle `proxy`/`resume`**: there is no `agent_id` / `agent_type` pair analogous to Claude Code. The stable identity is the child session's `Session.id`, paired with the parent's `task` tool result envelope (`<task id="<sessionID>" state="...">`) and the `parentID` field on the child session. To address a specific subagent from a wrapper:
  - Capture the `sessionID` from the `session.created` event whose payload has a non-empty `parentID` (this is the child).
  - Resume the child directly by calling the `task` tool with `task_id: <child-id>` and a follow-up prompt — the same child session is reused with its prior context, permission ruleset, and any background processes.
  - To override the child's permission, model, or metadata before resume, call `sessions.setPermission({ sessionID, permission })`, `sessions.setMetadata({ sessionID, metadata })`, or `sessions.setTitle` on the child session ID first.
  - Use `kilo session list --search "<description>"` or `GET /session/:id/children` to walk the child's grandchildren when nesting.
  - For tool-call permission prompts raised by a subagent, look for `permission.updated` events on the parent session (the prompt is lifted to the parent, not the child), and resolve with the parent's `permissionID`.
- **Wrappers that need `KILO_CONFIG_CONTENT`**: the inline JSON overlay already used for MCP injection is the natural surface for an agent linker. Agents injected here propagate to child sessions automatically (the system-prompt research confirms that `KILO_CONFIG_CONTENT` survives the child session's prompt construction), so a wrapper that wants to scope a custom agent to one run can use it without touching the project filesystem. The same surface can carry `permission.task` rules to gate which subagents the parent can call — note that Kilo's nested-task block is enforced unconditionally, so a `permission.task: { "*": "deny" }` injected here is largely cosmetic (Kilo's `KiloTask.nestedTask()` always strips the `task` tool from subagents' tool maps regardless of the `permission` block).
- **Model resolution**: subagents inherit the invoking primary agent's model unless the agent's own `model` field is set; wrappers should not assume a subagent uses the global default. The five-tier resolution is: (1) `task` invocation override (if the tool schema exposes a `model` parameter; Kilo's current `TaskTool` does not pass a `model` parameter through), (2) agent's `model` frontmatter, (3) invoking primary agent's model, (4) `kilo.json[c]` global `model`, (5) `--model` CLI override on the parent run. `variant` is independent of model resolution and is applied only when the agent's own `model` field is in use.
- **Forced subagent-nesting block**: Kilo's `KiloTask.nestedTask()` in `kilocode/tool/task.ts` strips the `task` tool from every subagent's tool map unconditionally. A wrapper that wants subagent nesting should not rely on `permission.task` configuration; the only way to enable nested delegation is to patch `kilocode/tool/task.ts` itself. Document this as Kilo-specific behavior; upstream OpenCode does not have the same block.

## Sources

- [Kilo Code homepage](https://kilo.ai/)
- [Kilo Code documentation root](https://kilocode.ai/docs/)
- [Kilo Code — Custom Modes (legacy)](https://kilocode.ai/docs/customize/custom-modes)
- [Kilo Code — Custom Subagents](https://kilocode.ai/docs/customize/custom-subagents)
- [Kilo Code — Using Agents](https://kilocode.ai/docs/code-with-ai/agents/using-agents)
- [Kilo Code — Orchestrator Mode (Deprecated)](https://kilocode.ai/docs/code-with-ai/agents/orchestrator-mode)
- [Kilo Code — Tool Use Overview](https://kilocode.ai/docs/automate/tools)
- [Kilo Code — Chat Interface](https://kilocode.ai/docs/code-with-ai/agents/chat-interface)
- [Kilo Code — Context & Mentions](https://kilocode.ai/docs/code-with-ai/agents/context-mentions)
- [Kilo Code — Model Selection](https://kilocode.ai/docs/code-with-ai/agents/model-selection)
- [Kilo Code — CLI documentation](https://kilocode.ai/docs/code-with-ai/platforms/cli)
- [Kilo Code — CLI Command Reference](https://kilocode.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code — Settings](https://kilocode.ai/docs/getting-started/settings)
- [Kilo Code — Auto-approving actions](https://kilocode.ai/docs/getting-started/settings/auto-approving-actions)
- [Kilo Code — Sandboxing (experimental)](https://kilocode.ai/docs/getting-started/settings/sandboxing)
- [Kilo Code — Marketplace](https://kilocode.ai/docs/customize/marketplace)
- [Kilo Code — What's new in the new extension](https://kilocode.ai/docs/code-with-ai/platforms/vscode/whats-new)
- [Kilo Code — MCP overview](https://kilocode.ai/docs/features/mcp/overview)
- [Kilo Code repository (`Kilo-Org/kilocode`)](https://github.com/Kilo-Org/kilocode)
- [`packages/opencode/src/agent/agent.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/agent/agent.ts) — base agent service (built-in defaults, `Agent.Info` schema, `generate`, `hardenSystemAgents`)
- [`packages/opencode/src/agent/subagent-permissions.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/agent/subagent-permissions.ts) — `deriveSubagentSessionPermission` (parent agent/session permission merge for subagents)
- [`packages/opencode/src/tool/task.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/tool/task.ts) — `TaskTool` (the `task` tool surface, resume, background mode, child session creation, `KiloTask.*` patches)
- [`packages/opencode/src/session/session.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/session/session.ts) — `Session` service (`Session.Info.parentID`, `Session.create`, `Session.children`, `session.created`/`session.updated`/`session.deleted` events)
- [`packages/opencode/src/kilocode/agent/index.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/agent/index.ts) — Kilo-specific agent patches (`patchAgents` adds `code`/`plan`/`debug`/`ask`/`orchestrator`, `preprocessConfig` remaps `build`→`code`, `processConfigItem` lifts `displayName`/`source` from `options`, `remove` blocks org agents)
- [`packages/opencode/src/cli/cmd/agent.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/cli/cmd/agent.ts) — `kilo agent create` / `kilo agent list` CLI commands
- [`packages/opencode/src/bus/bus-event.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/bus/bus-event.ts) — bus event registry
- [`packages/opencode/src/config/managed.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/managed.ts) — managed-config paths
- [Kilo JSON schema (`https://app.kilo.ai/config.json`)](https://app.kilo.ai/config.json)
- [OpenCode Config (parent project)](https://opencode.ai/docs/config)
- [OpenCode Agents (parent project)](https://opencode.ai/docs/agents/)
- [OpenCode Permissions (parent project)](https://opencode.ai/docs/permissions/)
- [OpenCode Server / HTTP API (parent project)](https://opencode.ai/docs/server/)
- [Kilo MCP research in claudine — companion document on Kilo's MCP system](claudine/docs/research/mcp/kilo.md)
- [Kilo Model research in claudine — companion document on Kilo's model catalog](claudine/docs/research/agent-models/kilo.md)
