---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://goose-docs.ai/
docs: https://goose-docs.ai/docs/
subagent_docs: https://goose-docs.ai/docs/guides/context-engineering/custom-agents

support: first_class

locations:
  - os: macos
    scope: user
    path: "~/.agents/agents/"
    notes: "Primary global agent directory (Path::home_dir().join('.agents').join('agents')). Discovered by both `sources::list_agent_sources` and the summon platform extension. Each `*.md` file is parsed as a Markdown + YAML-frontmatter agent definition; `name` (required) and `description` (optional) drive identity and routing."
  - os: linux
    scope: user
    path: "~/.agents/agents/"
    notes: "Same XDG `~/.agents/agents/` location; matches `dirs::home_dir()` semantics."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\agents\\"
    notes: "Same `dirs::home_dir()`-relative location; backslashes on native Windows."
  - os: macos
    scope: repo
    path: ".agents/agents/"
    notes: "Project-scoped agent directory joined to the launch working directory. Walked before the user scope by `list_agent_dirs` so project agents shadow user agents with the same `name`."
  - os: linux
    scope: repo
    path: ".agents/agents/"
    notes: "Same project-scope location as macOS."
  - os: windows
    scope: repo
    path: ".agents\\agents\\"
    notes: "Same project-scope location as macOS / Linux, using Windows path separators."
  - os: macos
    scope: repo
    path: ".goose/agents/"
    notes: "Compatibility alias scanned alongside `.agents/agents/` and `.claude/agents/` for project agents."
  - os: linux
    scope: repo
    path: ".goose/agents/"
    notes: "Same compatibility alias as macOS."
  - os: windows
    scope: repo
    path: ".goose\\agents\\"
    notes: "Same compatibility alias as macOS / Linux."
  - os: macos
    scope: repo
    path: ".claude/agents/"
    notes: "Compatibility alias scanned in addition to `.agents/agents/` and `.goose/agents/`. Lets Goose pick up Claude Code-authored agent files (e.g. on this host, `~/.claude/agents/*.md`) and surface them to the summon extension."
  - os: linux
    scope: repo
    path: ".claude/agents/"
    notes: "Same compatibility alias as macOS."
  - os: windows
    scope: repo
    path: ".claude\\agents\\"
    notes: "Same compatibility alias as macOS / Linux."
  - os: macos
    scope: user
    path: "~/.goose/agents/"
    notes: "Legacy / compatibility global directory scanned by the summon extension's `discover_filesystem_sources`. Lower precedence than the `.agents/agents/` home directory and the goose-specific config directory."
  - os: linux
    scope: user
    path: "~/.goose/agents/"
    notes: "Same legacy / compatibility location."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.goose\\agents\\"
    notes: "Same legacy / compatibility location."
  - os: macos
    scope: user
    path: "~/.claude/agents/"
    notes: "Compatibility directory scanned globally so Goose can re-use Claude Code-authored agent files written to the Claude home directory."
  - os: linux
    scope: user
    path: "~/.claude/agents/"
    notes: "Same compatibility directory as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\agents\\"
    notes: "Same compatibility directory as macOS / Linux."
  - os: macos
    scope: system
    path: "~/Library/Application Support/Block/goose/agents/"
    notes: "Goose's primary on-disk config + data root (`Paths::config_dir().join('agents')`). Discovered via `list_agent_dirs`; on macOS the docs and source both resolve `config_dir` to `~/Library/Application Support/Block/goose/` (etcetera `Block/goose` strategy). Treated as user-global even though the path is system-style."
  - os: linux
    scope: system
    path: "~/.config/goose/agents/"
    notes: "Goose's `Paths::config_dir().join('agents')` on Linux. Docs page [Configuration Files](https://goose-docs.ai/docs/guides/config-files) anchors at `~/.config/goose/config.yaml`, so this directory sits one level above the documented config file."
  - os: windows
    scope: system
    path: "%APPDATA%\\Block\\goose\\config\\agents\\"
    notes: "Goose's `Paths::config_dir().join('agents')` on Windows; the docs page resolves the same config file to `%APPDATA%\\Block\\goose\\config\\config.yaml`."
  - os: macos
    scope: other
    path: "GOOSE_PATH_ROOT/.agents/agents/"
    notes: "When `GOOSE_PATH_ROOT` is set, every `Paths::*` lookup resolves under `<root>/config`, `<root>/data`, `<root>/state`, or `<root>/.agents/{agents,plugins}`. The Agents directory becomes `<root>/.agents/agents/`. Useful for tests, ephemeral CI sandboxes, and the `--help`-style path overrides that ship with the binary."
  - os: linux
    scope: other
    path: "GOOSE_PATH_ROOT/.agents/agents/"
    notes: "Same `GOOSE_PATH_ROOT` override as macOS."
  - os: windows
    scope: other
    path: "GOOSE_PATH_ROOT/.agents/agents/"
    notes: "Same `GOOSE_PATH_ROOT` override as macOS / Linux, using Windows path separators."
  - os: macos
    scope: extension
    path: "<recipe>/sub_recipes/*.yaml (recipe-defined)"
    notes: "Recipes (defined inline or via `GOOSE_RECIPE_PATH` directories) can carry a `sub_recipes:` array whose entries are surfaced as `SourceType::Subrecipe` agents. They participate in `load`/`delegate` but are not free-standing agent files; their identity comes from the recipe's `sub_recipes[].name` and the body comes from `load_local_recipe_file(path).content`."
  - os: linux
    scope: extension
    path: "<recipe>/sub_recipes/*.yaml (recipe-defined)"
    notes: "Same recipe-defined subrecipe entry point as macOS."
  - os: windows
    scope: extension
    path: "<recipe>/sub_recipes/*.yaml (recipe-defined)"
    notes: "Same recipe-defined subrecipe entry point as macOS / Linux."
  - os: macos
    scope: extension
    path: "<external-MCP-server> subagent stdio entry in config.yaml"
    notes: "External subagents are MCP stdio servers declared under `config.yaml`'s `subagent:` block (e.g. `cmd: codex`, `args: [\"mcp-server\"]`, `bundled: true`, `env_keys: [OPENAI_API_KEY]`). Goose wraps them and exposes them as `SourceType::Agent` with `path` set to the MCP entry's config key."
  - os: linux
    scope: extension
    path: "<external-MCP-server> subagent stdio entry in config.yaml"
    notes: "Same external-subagent mechanism as macOS."
  - os: windows
    scope: extension
    path: "<external-MCP-server> subagent stdio entry in config.yaml"
    notes: "Same external-subagent mechanism as macOS / Linux."

format:
  file_names:
    - "*.md"
  frontmatter: true
  required_fields:
    - "name (slugs-ish — `validate_agent_name` rejects empty, >80 chars, or any `/` / `\\`; `slugify_agent_name` lowercases + replaces non-alphanumeric with `-` to a 64-char max, defaulting to `agent` on empty)"
    - "description (recommended; surfaced in `load()` discovery and the @-mention picker; defaults to `\"Agent (<model>)\"` when omitted)"
  optional_fields:
    - "model (preferred model id or alias; goose does not currently propagate this into the subagent's runtime — the parent's `GOOSE_PROVIDER`/`GOOSE_MODEL` wins unless `delegate` is called with an explicit `model:` override)"
    - "any other YAML key — stored verbatim under `properties` in the `SourceEntry` and surfaced via ACP `source/properties`)"
  body_format: markdown
  notes: |
    The file is YAML frontmatter between `---` markers followed by a Markdown body. Identity comes from the `name` field (not the filename); filename collisions are deduplicated by lowercased `name`. The body becomes the agent's instructions verbatim and is loaded via `load(source: \"<name>\")` (added to the parent's context) or `delegate(source: \"<name>\", instructions: ...)` (run as an isolated subagent). The markdown parser (`parse_frontmatter`) returns `None` when the file is missing frontmatter and silently skips files whose YAML deserializes without an error but yields no `name` (e.g. random config files like `package.json` would NOT trigger this path — only `*.md` files are scanned, but `*.json` recipe files are scanned in adjacent directories).

    Goose's `validate_agent_name` caps names at 80 characters (slugified cap is 64). Names may contain spaces (e.g. `Backend TypeScript Developer`); the slugified filename and the filesystem `name` are independent of the on-disk identifier.

    Plugin agents (the goose plugin system is layered separately) and recipe-defined `sub_recipes` are surfaced under the same `SourceType::Agent` enumeration even though they live at different paths; the plugins topic owns the packaging boundary.

runtime:
  invocation: |
    Three surfaces invoke a Goose agent / subagent, all routed through the **Summon** platform extension (default-enabled, unprefixed-tools):

    (1) **Natural language + `@mention` picker** — type `@<agent-name>` at the start of a prompt to force the named agent. The summon extension inspects every user prompt and injects a roster of available sources into the system prompt, instructing the parent model to call `delegate(source: \"<name>\", instructions: ...)` when a name appears. `@code-reviewer review the diff` and `Delegate to code-reviewer: review the diff` both fire the same tool call.

    (2) **`delegate` tool (synchronous subagent spawn)** — fires when the parent model decides a subagent is appropriate. Parameters: `instructions` (ad-hoc task), `source` (named agent/recipe/subrecipe), `parameters` (recipe substitutions), `extensions` (allowlist; empty array disables all), `provider`, `model`, `temperature`, `max_turns`, `context` (extra system-prompt context), `working_dir` (must be inside parent's), `async` (default `false`). The parent must be in `goose_mode: auto` for the spawn to succeed; manual / smart_approve / chat-only modes disable subagents. SubAgent sessions cannot delegate further (`SessionType::SubAgent` blocks re-entry).

    (3) **`load` tool (instructions-only injection)** — `load()` lists every available source; `load(source: \"<name>\")` reads the agent's instructions into the **current** conversation's context (no spawn); `load(source: \"<task-id>\")` waits for an async background task and returns its result. For background tasks `peek: true` returns status without blocking; `cancel: true` cancels and returns partial output.

    Goose also auto-spawns internal subagents from the **direct prompt** path (\"Use 2 subagents to create hello.html\") and the **recipe** path (`Use the 'security-auditor' recipe to scan this endpoint`). Both build a `Recipe` from `delegate`'s arguments, hand it to `run_subagent_task`, and stream its `AgentEvent::Message` notifications back to the parent.

    **External subagents** (the `subagent:` block in `config.yaml`) are MCP stdio servers; goose wraps them so `delegate(source: \"<name>\")` dispatches a JSON-RPC call to the external process.

    The `goose session` CLI does not itself expose a `--agent <name>` whole-session replacement equivalent; subagents are a runtime tool, not a session configuration.
  parent_child_context: |
    Each delegated subagent runs as a new session of its own (`session_type = SessionType::SubAgent`) and receives only:

    - The rendered `subagent_system.md` template, which embeds the agent's `description`, `max_turns`, `subagent_id` (the new session id), `tool_count`, and `available_tools` list (filtered through `is_tool_visible_to_model`).
    - The recipe's `instructions` (verbatim body of the agent file for `SourceType::Agent`; full recipe body for `SourceType::Recipe` / `SourceType::Subrecipe`) with Jinja parameter substitution applied.
    - The user task assembled by the parent (`prompt` plus any optional `context`).
    - The extensions the parent filtered into the delegation (default: `extensions` parameter omitted ⇒ inherit all enabled extensions; explicit list narrows; explicit empty array disables all).

    Subagents do **not** receive the parent's conversation history, the parent's hint files, the parent's `moim_message_*` working-memory blocks, or the parent's skill selections. The subagent gets its own `session_id`, its own recipe store, and its own extension set. The parent sees only the subagent's final assistant text plus `subagent_session_id` in the tool-result `meta`.

    Return mode is documented in two flavors: by default the parent sees the **final assistant text** only (`return_last_only: true` is hardcoded for `delegate`); `delegate`'s `parameters.return_last_only` flag is reserved but not surfaced through `DelegateParams`. The summon extension can also be configured (via natural language in the parent's prompt) for **full detail** mode in the future. Parallel subagents return their per-task status via the structured `execution_summary` envelope emitted by the **direct prompt** path; recipe-driven parallel delegations return per-task results through the same `delegate` mechanism.
  permissions_inheritance: |
    Tool permission mode follows the parent's `GOOSE_MODE` setting with one explicit override:

    - Subagents are **forced into `GooseMode::Auto`** by `handle_delegate` (\"Subagents must use Auto until `get_agent_messages` forwards `ActionRequired` messages to the parent. Until then, any mode that requires approval will hang on the subagent's `confirmation_rx`.\"). The parent's mode is otherwise inherited, but `chat` / `approve` / `smart_approve` parents block subagent spawning entirely (\"subagents are disabled in manual approval, smart approval, and chat-only modes\").
    - `delegate.extensions: []` narrows the subagent to no extensions; `delegate.extensions: [\"developer\", ...]` narrows to that allowlist; omitted ⇒ inherit all parent extensions (the parent's extensions are inherited individually, not as an allowlist policy).
    - Goose's `SecurityPrompt` / prompt-injection-detection (`SECURITY_PROMPT_ENABLED`) and keyring-gated `secrets.yaml` are process-wide; they apply to subagent processes identically. `GOOSE_ALLOWLIST` is the extension allowlist enforcement knob; subagents respect it the same as the parent.
    - Subagent-internal tool calls are filtered by `is_tool_visible_to_model` (excludes admin / hidden / reserved tools). Subagents cannot enable or disable extensions (`ext_manager` is intentionally restricted for subagents per the docs' \"Security Constraints\" table — extension management is one of three blocked operations: subagent spawning, extension management, schedule management).
  model_inheritance: |
    Resolution order, top wins:

    1. **`delegate.model` + `delegate.provider` + `delegate.temperature`** — explicit per-invocation override from the parent's tool call.
    2. **`recipe.settings.{goose_provider, goose_model, temperature, max_turns}`** — recipe-level override (the `SourceType::Agent` body is converted into a recipe with these fields).
    3. **Agent frontmatter `model:`** — informational; the source code does not currently propagate it to the subagent's provider. Goose's `build_recipe_from_agent` does not read frontmatter `model` into the recipe's settings (it copies only `description` and `instructions`), so the `model` field is effectively documentation-only.
    4. **Parent session's `goose_provider` + `goose_model`** — inherited by default.

    Subagents use the parent's model unless the parent explicitly overrides via `delegate(model: ...)` or the recipe's `settings:` block. `delegate.max_turns` overrides `recipe.settings.max_turns`, which overrides `GOOSE_SUBAGENT_MAX_TURNS`, which overrides the 25-turn default.
  tool_inheritance: |
    Subagents inherit the parent's enabled extensions by default. The `delegate.extensions` parameter is an **allowlist** that narrows the set (omitted ⇒ inherit all; `[]` ⇒ none; explicit array ⇒ only those). MCP servers from the parent session flow through automatically unless restricted.

    The subagent's **system prompt** enumerates the tools it can call (`available_tools` in `subagent_system.md`), filtered by `is_tool_visible_to_model`. Hidden platform extensions (`Orchestrator`, `Chat Recall` when `default_enabled: false`) are filtered out. The `summon` extension is always present in the subagent (it provides `delegate`/`load`); recipes that define `sub_recipes` have `summon` auto-injected; recipes that explicitly enumerate `extensions` must list `summon` themselves to keep subagent support.

    Subagent-to-subagent recursion is blocked at the session-type layer (`if session.session_type == SessionType::SubAgent { return Err(\"Delegated tasks cannot spawn further delegations\") }`). A subagent therefore cannot spawn further subagents — it can `load` and `delegate` only when its parent was the original session.
  max_turns: |
    Default `DEFAULT_SUBAGENT_MAX_TURNS = 25` (constant in `subagent_task_config.rs`). Per-invocation overrides:

    - `delegate.max_turns: <int>` (overrides everything below).
    - `recipe.settings.max_turns: <int>` (overrides `GOOSE_SUBAGENT_MAX_TURNS`).
    - `GOOSE_SUBAGENT_MAX_TURNS` env var (defaults to 25).
    - 25 (built-in default).

    No hardcoded recursive depth knob exists because subagent-to-subagent recursion is blocked outright. The parent's session `max_turns` (default 1000) is independent of the subagent cap.
  notes: |
    Concurrency: `GOOSE_MAX_BACKGROUND_TASKS` (default 5) bounds the number of concurrent async `delegate(..., async: true)` calls. Background task IDs are 8-digit-date + sequence (\"20260219_1\"); they are recognized by `is_session_id`. The summon extension keeps a per-task `JoinHandle`, a `cancellation_token`, a `last_activity` epoch-millis stamp, a `notification_buffer`, and a `turns` counter; the buffer is drained back into subscribers the next time `load(source: \"<task-id>\")` is called.

    Default `GOOSE_COMPLETED_TASK_TTL_SECS = 600` (10 minutes). After the TTL the entry is dropped by `cleanup_completed_tasks`. Tasks also auto-cancel on `SummonClient::Drop`.

    Failure: a subagent that hits `max_turns` or `timeout` (5-minute wall-clock default, configurable via natural language) returns a `load()` envelope with `task_status: \"failed\"` or `\"cancelled\"`. The `delegate` tool returns an MCP `CallToolResult::error` with the failure text and a `subagent_session_id` in `meta`. The parent session continues normally; a failed subagent does NOT mark the parent session as failed.

    Recipe `sub_recipes[].sequential_when_repeated` controls whether repeated subrecipe calls in the same parent turn execute serially; the default is parallel for repeated subrecipes. `delegate.async: true` parallelizes a single parent's many delegations.

observability:
  stream_events:
    - "`subagent_tool_request` (ServerNotification / LoggingMessageNotification; fields: `subagent_id`, `tool_call.name`, `tool_call.arguments`; emitted by `create_tool_notification` from `subagent_handler.rs` on every subagent tool call)"
    - "`subagent:<id>` (logger name on the LoggingMessageNotification, used by `goose-desktop` to group tool calls visually)"
    - "`task_status` (one of `completed` / `failed` / `cancelled` / `panicked` / `running`; surfaced in `load()` meta when retrieving background task results)"
    - "`subagent_session_id` (the new session id assigned to the subagent; surfaced in `delegate` and `load` meta)"
    - "`turns_taken` (count of model+tool iterations the subagent ran before returning; surfaced in `load` meta)"
    - "`duration_secs` (wall-clock duration; surfaced in `load` meta)"
    - "`peek` mode emits the same envelope non-blockingly with `task_status: \"running\"`"
  hook_events:
    - "Goose has no documented hook event for subagent lifecycle in the public docs. Subagent starts/stops are visible through the **`server_notifications` MCP notification channel** (`LoggingMessageNotification` with logger `subagent:<id>` and structured data `type: subagent_tool_request`), not via the Goose CLI hook subsystem (`crates/goose/src/hooks/`). Hooks topic research for goose is recorded separately; this topic captures only the agent-lifecycle notification shape."
  session_ids: true
  notes: |
    Each subagent gets a **stable session id** (`SessionType::SubAgent`, generated by `session_manager.create_session`). The id is the same `subagent_session_id` returned to the parent in `delegate` meta, the same id the parent uses with `load(source: \"<id>\")` to retrieve the result, and the same id the LoggingMessageNotification logs under `subagent:<id>`. Session ids are 8-digit-date + sequence (e.g. `20260219_1`); the summon extension recognizes them via `is_session_id`.

    Subagent transcripts are written to the regular session store under the standard goose data dir (`Paths::data_dir()` — `~/Library/Application Support/Block/goose/data/` on macOS, `~/.local/share/goose/` on Linux, `%APPDATA%\\Block\\goose\\data\\` on Windows). They are first-class `Session` records, not separate per-subagent files. Goose CLI ships no public transcript path equivalent to Claude Code's `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl`; consumers replay context from the `session_id`.

    `goose session list` / `goose session export --session-id <id>` expose the subagent session as a normal session for diagnostics. `goose session --resume --session-id <id>` can re-enter the subagent session — but since subagents do not auto-resume after `delegate` returns control to the parent, this is a manual forensic step rather than an automatic resume path. The parent that started the subagent cannot transparently resume it; only a human operator with the subagent's id can.

portability:
  portable: false
  non_portable_assets:
    - "`name` slug (Goose accepts spaces and any non-`/`/`\\` text up to 80 chars; other providers apply stricter rules)"
    - "`description` field is portable as routing intent but the natural-language phrasing matters for summon extension's @mention / load() discovery"
    - "`model` frontmatter (Goose-specific; not all providers honor a frontmatter model and Goose itself does not currently propagate it to the subagent's runtime)"
    - "`properties` HashMap (Goose-specific; the frontmatter parser preserves arbitrary keys into `SourceEntry.properties` and exposes them via ACP `source/properties`; other providers do not understand this bag)"
    - "Body prompt that references goose-only tools (`developer__shell`, `todo__*`, `summon__delegate`, `summon__load`, `extensionmanager__*`), goose-only env vars (`$GOOSE_PROVIDER`, `$GOOSE_MODEL`, `$GOOSE_MODE`, `$GOOSE_SUBAGENT_MAX_TURNS`, `$AGENT_SESSION_ID`), or goose-only platform markers (`AGENT=goose`, `GOOSE_TERMINAL=1`)"
    - "Recipe-defined `sub_recipes` entries (Goose-only; other providers have no recipe layer)"
    - "External subagent MCP `subagent:` blocks in `config.yaml` (Goose-only; this is the canonical way to wrap an external CLI like Codex as a goose subagent)"
    - "The `paths.canonicalize_or_original` + `is_under_root` portability check (Goose's own source-root machinery; not exported as an API)"
    - "Goose's `summon` extension name (the tool that drives `load`/`delegate` is itself provider-specific — Claude Code has the `Agent` tool, Codex uses TOML `agents/` + a different discovery path)"
    - "Subagent sessions created via `SessionType::SubAgent` are an opaque session-type tag and do not survive a roundtrip to another provider's session store"
    - "The 5-minute default timeout (Goose-specific wall-clock default for subagents; configurable via natural language only)"
  rewrite_needed: true
  notes: |
    The Markdown body of an agent definition is the most portable bit — when it does not reference goose-only tools, env vars, or platform markers, it can be lifted verbatim across providers. The frontmatter vocabulary, however, is provider-specific enough that a verbatim link is unsafe.

    Cross-provider rewrite map (Claude Code → Goose):

    | Claude Code field | Goose equivalent | Notes |
    |---|---|---|
    | `name` | `name` | Goose allows spaces and a more relaxed charset; slugify at write-time if needed |
    | `description` | `description` | Verbatim |
    | `tools` (allowlist) | `delegate.extensions: [...]` | Per-invocation parameter; not a frontmatter field. Goose decides per-call rather than per-definition. |
    | `disallowedTools` | (no equivalent) | Goose has no per-subagent denylist; restrict by setting `delegate.extensions: []` and listing only the desired MCP server names |
    | `permissionMode` | `GOOSE_MODE` (session-wide) | Goose has no per-subagent permission override; the parent mode (forced to `Auto` for subagents) always wins |
    | `mcpServers` | `delegate.extensions: [...]` (referenced by name) | Goose uses MCP server names; the inline MCP server shape is similar but not identical |
    | `model` | `delegate.model` + `recipe.settings.goose_model` | The `model` field in goose agent frontmatter is documentation-only; set the recipe's `settings:` block instead |
    | `maxTurns` | `GOOSE_SUBAGENT_MAX_TURNS` (env) or `delegate.max_turns` | 25-turn default vs Claude's session-wide 1000 |
    | `skills` (preload list) | (no equivalent) | Goose has a Skills platform extension but preloading is not part of the agent definition |
    | `memory` (MEMORY.md scope) | (no equivalent) | Goose has the Memory MCP extension; no per-agent memory scoping |
    | `hooks` | (no equivalent) | Subagent-scoped hooks are a Claude Code concept; Goose emits `subagent_tool_request` notifications that are consumed by `goose-desktop` rather than via a configurable hook layer |
    | `background` / `effort` / `isolation` / `color` | (no equivalents) | Drop |
    | `initialPrompt` | (no equivalent) | Drop or convert to recipe `prompt:` |
    | Body prompt | body prompt | Verbatim when it does not reference Claude Code-only tools / env vars |

    Cross-provider rewrite map (Codex → Goose):

    | Codex field | Goose equivalent | Notes |
    |---|---|---|
    | `name` (TOML) | `name` (YAML) | Goose is more permissive; slugify at write-time |
    | `description` | `description` | Verbatim |
    | `developer_instructions` | Body Markdown | Goose's agent body is the system prompt |
    | `model` | frontmatter `model` (documentation-only) + recipe `settings.goose_model` | Codex's TOML field moves into the recipe's `settings:` block |
    | `model_reasoning_effort` | (no equivalent) | Goose doesn't have an agent-scoped reasoning knob |
    | `sandbox_mode` | (no equivalent) | Goose doesn't have a per-agent sandbox mode |
    | `approval_policy` | (no equivalent) | Per-subagent approval is not supported; parent's `GOOSE_MODE` wins (forced to `Auto`) |
    | `mcp_servers.<id>` | Recipe `extensions: [...]` | The MCP shape is similar but the recipe is the source of truth in Goose |

    Portable bits (link-as-is):

    - The Markdown body when it does not reference goose-only tools or env vars.
    - The `description` field's routing intent — copy across with minor rephrasing for the target provider's router language.
    - Generic MCP server configurations (any provider's MCP server can be referenced from another provider with a wrapper).

    Claude Code-authored agent files discovered via `~/.claude/agents/*.md` and `.claude/agents/*.md` work as Goose agents with two caveats: (1) Claude-only frontmatter fields (`tools`, `model: sonnet`, `permissionMode`, `mcpServers`, etc.) are stored under `properties` and ignored by Goose; (2) the body prompt usually references Claude-specific tool names (`Read`, `Bash`, `Grep`, etc.) that Goose cannot resolve, so a body rewrite is required before the agent can run as a Goose subagent.

cli_params:
  - flag: --provider <name>
    description: "Per-session LLM provider override on `goose run` / `goose session start`. Sets the parent's provider; subagents inherit by default unless `delegate(model: ..., provider: ...)` overrides."
    example: "goose run --provider anthropic -t \"summarize this PR\""
  - flag: --model <model-id-or-alias>
    description: "Per-session model override on `goose run`. Same precedence story as `--provider`."
    example: "goose run --model claude-sonnet-4-5 -t \"summarize this PR\""
  - flag: --with-builtin <extension-name> [...]
    description: "Restrict which builtin extensions are loaded for the session. Removing `summon` disables `delegate`/`load` and therefore all subagent invocation. Same flag works on `goose acp`."
    example: "goose run --with-builtin developer,platform_tools --with-builtin summon -t \"...\""
  - flag: --no-session-naming (also via `GOOSE_DISABLE_SESSION_NAMING`)
    description: "Disables AI-generated session names. Affects both parent sessions and the subagents they spawn (the summon extension passes `disable_session_naming: true` when constructing the subagent's `AgentConfig`)."
    example: "GOOSE_DISABLE_SESSION_NAMING=1 goose run -t \"...\""
  - flag: --debug (also `GOOSE_DEBUG=1`)
    description: "Toggles verbose tool-parameter output and additional logging. Useful when diagnosing subagent tool-call notifications (the LoggingMessageNotification channel is unaffected by `--debug`, but full tool-call payloads appear in the parent transcript)."
    example: "goose run --debug -t \"delegate to code-reviewer and show the tool calls\""
  - flag: --output-format <text|json|stream-json>
    description: "Output format for non-interactive `goose run`. `stream-json` exposes per-turn events on stdout; subagent lifecycle is observable via the `subagent_tool_request` MCP notification side-channel rather than the stream-json stream itself."
    example: "goose run --output-format json -t \"...\""
  - flag: --max-turns <n> (also `GOOSE_MAX_TURNS`)
    description: "Parent session turn cap. Default 1000. Distinct from the subagent-specific `GOOSE_SUBAGENT_MAX_TURNS` (default 25)."
    example: "goose run --max-turns 50 -t \"...\""
  - flag: goose recipe validate <file>
    description: "Validates a recipe (and the agent/recipe `sub_recipes` it references). Use this when authoring recipes that include `sub_recipes:` entries which will surface as `SourceType::Subrecipe` agents to the summon extension."
    example: "goose recipe validate ./code-reviewer.yaml"
  - flag: goose session --resume --session-id <id>
    description: "Resume a session by id. Useful for re-entering a subagent session for diagnostics; subagents do not auto-resume after `delegate` returns."
    example: "goose session --resume --session-id 20260219_1"
  - flag: goose session --export --session-id <id>
    description: "Export a session transcript (JSONL). Use this to inspect a subagent's turns after the fact."
    example: "goose session --export --session-id 20260219_1 --format jsonl > subagent.jsonl"
  - flag: goose skills list
    description: "List installed skills (related: skills and agents both surface through the same filesystem discovery and the summon extension's `load()` discovery)."
    example: "goose skills list"
  - flag: goose plugin install <git-url>
    description: "Install a plugin (which may bundle agent files via the plugin manifest). Plugin-packaged agents are documented by the plugins topic."
    example: "goose plugin install https://github.com/example/goose-plugin.git"
  - flag: goose configure (Choose Extensions / Configure Subagent Recipes)
    description: "Interactive editor for `config.yaml`. Use it to register an external subagent via the `subagent:` MCP stdio block."
    example: "goose configure → Configure Subagent Recipes → Enter codex → provide cmd/args/env_keys"
  - flag: --extension <name> (also `--with-extension`)
    description: "Add an extension to the session for one run; affects subagents via `delegate.extensions` inheritance."
    example: "goose run --with-extension computercontroller -t \"...\""
  - flag: --recipe <path>
    description: "Run a specific recipe. Recipes carry `sub_recipes:` which surface as `SourceType::Subrecipe` agents during the run."
    example: "goose run --recipe ./code-reviewer.yaml"
  - flag: --interactive
    description: "Switch `goose run` to interactive mode (otherwise the command exits after the first response). Interactive mode exposes the summon extension's `load()` discovery inline."
    example: "goose run --interactive"
  - flag: --scheduled-at <cron>
    description: "Schedule a recipe run. Subagents cannot be scheduled directly — schedule a recipe that delegates to the agent."
    example: "goose run --recipe ./daily-review.yaml --scheduled-at \"0 9 * * 1-5\""

env_vars:
  - name: GOOSE_PROVIDER
    effect: "Primary LLM provider. Inherited by subagents unless `delegate(provider: ...)` overrides."
  - name: GOOSE_MODEL
    effect: "Primary model. Inherited by subagents unless `delegate(model: ...)` overrides."
  - name: GOOSE_SUBAGENT_MAX_TURNS
    effect: "Default max-turns cap for delegated subagents (default 25). Overridden by `delegate.max_turns` then by `recipe.settings.max_turns`."
  - name: GOOSE_MAX_BACKGROUND_TASKS
    effect: "Maximum number of concurrent async `delegate(..., async: true)` calls (default 5)."
  - name: GOOSE_COMPLETED_TASK_TTL_SECS
    effect: "How long a completed background task stays available for `load(source: \"<task-id>\")` retrieval (default 600s = 10 minutes)."
  - name: GOOSE_MAX_TURNS
    effect: "Parent-session turn cap (default 1000). Independent of `GOOSE_SUBAGENT_MAX_TURNS`."
  - name: GOOSE_GATEWAY_MAX_TURNS
    effect: "Gateway session cap (Telegram etc.); overrides `GOOSE_MAX_TURNS` for gateway traffic only."
  - name: GOOSE_MODE
    effect: "Tool execution mode (`auto` / `approve` / `chat` / `smart_approve`). Subagents are disabled in `approve`, `chat`, `smart_approve`; only `auto` allows spawning."
  - name: GOOSE_RECIPE_PATH
    effect: "Additional recipe directories (colon-separated on Unix, semicolon-separated on Windows). Recipes in these directories are surfaced as `SourceType::Recipe` to the summon extension."
  - name: GOOSE_RECIPE_GITHUB_REPO
    effect: "GitHub repository for recipe discovery (org/repo). Requires `gh` CLI authentication."
  - name: GOOSE_PATH_ROOT
    effect: "Override the root directory for all goose config, data, state, and agent files. Useful for tests and CI sandboxes; when set, agent discovery resolves under `<root>/.agents/agents/` instead of `~/.agents/agents/`."
  - name: GOOSE_DISABLE_SESSION_NAMING
    effect: "Disables AI-generated session naming. Set on both parent and subagent sessions."
  - name: AGENT
    effect: "Goose sets `AGENT=goose` in shell subprocesses it spawns; scripts can detect goose execution."
  - name: AGENT_SESSION_ID
    effect: "Current session id, automatically set in STDIO-extension subprocess and Developer-extension shell environments. Subagents have their own session id; the parent's id is NOT exported into the subagent's shell by default."
  - name: GOOSE_TERMINAL
    effect: "Indicates that a command is being executed by goose; scripts can branch on this."
  - name: GOOSE_ALLOWLIST
    effect: "URL of the allowed-extensions allowlist. Affects both the parent and the extensions subagents inherit."
  - name: SECURITY_PROMPT_ENABLED / SECURITY_PROMPT_THRESHOLD
    effect: "Process-wide prompt-injection detection; applies to subagent runs identically."
  - name: GOOSE_TELEMETRY_ENABLED
    effect: "Toggle anonymous telemetry; applies to subagent sessions identically."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `linking` module does not currently have a Goose CLI row. The agent linking walker needs three new adapters that mirror the order documented in the goose source:

  1. **Goose custom-agent linker** — walk `~/.agents/agents/*.md` and `.agents/agents/*.md` (with compatibility aliases `.goose/agents/*.md`, `.claude/agents/*.md`, plus the goose-specific config-directory agents path) and parse YAML frontmatter. The walker must use `name` for identity (not the filename), capture `description` and the `model` frontmatter field (Goose does not currently propagate the frontmatter `model` to the subagent runtime, but the linker should still surface it as a property), and preserve the rest of the frontmatter under a `properties` bag because the goose source explicitly forwards arbitrary frontmatter keys to ACP `source/properties`.

  2. **Recipe-defined subrecipe linker** — for each `*.yaml`/`.json` recipe in `GOOSE_RECIPE_PATH` plus the working directory's `.goose/recipes/` and `.agents/recipes/`, parse the recipe and surface each `sub_recipes[].name` as a `SourceType::Subrecipe` row. Recipe `sub_recipes[].path` may be relative to the parent recipe, so the walker needs to resolve the relative path before emitting the row.

  3. **External subagent linker** — for each `subagent:` block in `~/.config/goose/config.yaml` (or its Windows equivalent), surface a single `SourceType::Agent` row whose `path` is the YAML key and whose `properties.bundled` / `properties.cmd` / `properties.args` / `properties.env_keys` carry the MCP stdio entry's config.

  For the runtime side, Claudine's planned lifecycle `proxy` / `resume` actions should:

  - Recognize the `subagent_session_id` MCP meta field on `delegate` and `load` tool results; the same id is the `load(source: "<task-id>")` argument for retrieving async results.
  - Recognize the `subagent_tool_request` LoggingMessageNotification channel (`logger = "subagent:<id>"`, `data.type = "subagent_tool_request"`, `data.tool_call.{name, arguments}`) when parsing goose CLI stream output — this is how the wrapper sees subagent tool calls in real time.
  - Recognize `is_session_id` shaped ids (8-digit-date + sequence, e.g. `20260219_1`) as background-task identifiers, not regular agents. The summon extension distinguishes them automatically; a wrapper has to do the same.
  - Skip subagent session entries (`SessionType::SubAgent`) when surfacing "primary" sessions — but keep them accessible for forensic inspection via `goose session --resume --session-id <id>`.
  - Honor the per-invocation `delegate.max_turns` override when sizing step-timeouts and runaway-volume caps, and use `GOOSE_SUBAGENT_MAX_TURNS` (default 25) as the floor for any per-subagent timeout.

  Finally, `claudine providers` should grow a Goose row reporting the same surface (filesystem discovery, MCP subagent blocks, recipe subrecipes), and `claudine agents` should be able to list user-scope, repo-scope, and goose-config-dir-scope Goose agent files alongside Claude Code's catalog output. Because Goose also accepts Claude Code-authored agent files at `~/.claude/agents/*.md` and `.claude/agents/*.md`, the linker must tag such files as "Claude-originated" for portability analysis (see the cross-provider rewrite map in the Portability section above).
---

# Goose Subagents

## Overview

Goose (now under the Agentic AI Foundation; repo at `aaif-goose/goose`, docs at `goose-docs.ai`) treats user-defined agents as a **first-class** feature: durable Markdown files with YAML frontmatter that change who does work inside a session. The provider calls them "custom agents" (`docs/guides/context-engineering/custom-agents`) for the on-disk definitions, and "subagents" (`docs/guides/context-engineering/subagents`) for the runtime delegation feature that consumes them. The two are tied together by the **Summon** platform extension, which is default-enabled, exposes unprefixed tools, and provides the `delegate` and `load` tools the parent model uses to spawn or read agent files.

Two definitions:

- **Custom agent** — a static `*.md` file with `name` / `description` / `model` frontmatter and a Markdown body that becomes the agent's instructions. Three scopes (user-global `~/.agents/agents/`, project `<cwd>/.agents/agents/`, goose-config-dir agents path) plus three compatibility aliases (`.goose/agents/`, `.claude/agents/`, and `~/.claude/agents/`, `~/.goose/agents/`).
- **Subagent** — a runtime delegation that Goose's parent agent launches via `delegate`. The default behavior is **autonomous**: goose decides when to spawn subagents (when `GOOSE_MODE=auto`); manual / smart_approve / chat-only modes disable them. Subagents can also be configured via **recipes** (YAML/JSON files with `instructions`, `prompt`, `extensions`, `parameters`, `settings.max_turns`, etc.) or via **direct prompts** (one-off natural-language tasks). External subagents are MCP stdio servers wrapped in a `config.yaml` `subagent:` block.

This topic covers both halves of the picture: where agent files live, what frontmatter they accept, how the parent picks one, what context and permissions the child gets, and how a wrapper can observe start / stop. Hook event semantics (Goose's `LoggingMessageNotification` channel and the `hooks/` crate) are documented by the hooks topic; this document records only the agent-lifecycle notification shape (`subagent_tool_request`, `subagent:<id>`, `task_status`).

## Locations

Goose discovers agent files from up to seven on-disk roots per session, in a stable order documented by `list_agent_dirs` in `crates/goose/src/sources.rs` and the parallel `discover_filesystem_sources` in `crates/goose/src/agents/platform_extensions/summon.rs`. The compatibility aliases are deliberate so that users with existing Claude Code / `.goose/`-era agent files can pick up goose without renames.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| Project `.agents` | `<cwd>/.agents/agents/` | `<cwd>/.agents/agents/` | `<cwd>\.agents\agents\` | Project-scoped (highest precedence). Walked first when a working directory is set. |
| Project `.goose` | `<cwd>/.goose/agents/` | `<cwd>/.goose/agents/` | `<cwd>\.goose\agents\` | Compatibility alias. |
| Project `.claude` | `<cwd>/.claude/agents/` | `<cwd>/.claude/agents/` | `<cwd>\.claude\agents\` | Compatibility alias. Lets goose pick up Claude Code-authored agent files in a project. |
| User `~/.agents` | `~/.agents/agents/` | `~/.agents/agents/` | `%USERPROFILE%\.agents\agents\` | Primary global directory (`Paths::agents_dir()`). |
| User `~/.goose` | `~/.goose/agents/` | `~/.goose/agents/` | `%USERPROFILE%\.goose\agents\` | Legacy / compatibility. |
| User `~/.claude` | `~/.claude/agents/` | `~/.claude/agents/` | `%USERPROFILE%\.claude\agents\` | Compatibility alias for Claude Code agents. |
| Goose config dir | `~/Library/Application Support/Block/goose/agents/` | `~/.config/goose/agents/` | `%APPDATA%\Block\goose\config\agents\` | `Paths::config_dir().join("agents")` — the goose-specific config directory. Resolved via etcetera's `Block/goose` strategy on macOS, the docs' `~/.config/goose/` on Linux, and the docs' `%APPDATA%\Block\goose\config\` on Windows. |
| `GOOSE_PATH_ROOT` | `<root>/.agents/agents/` | `<root>/.agents/agents/` | `<root>\.agents\agents\` | When `GOOSE_PATH_ROOT` is set, every path resolves under this root. |
| External MCP `subagent:` | inline in `config.yaml` | inline in `config.yaml` | inline in `config.yaml` | An MCP stdio entry declared as `subagent:` keys; surfaced as `SourceType::Agent` with `path` set to the YAML key. |
| Recipe subrecipes | `<recipe>`'s `sub_recipes[].path` | same | same | A recipe's `sub_recipes` array entries are surfaced as `SourceType::Subrecipe`; their body comes from `load_local_recipe_file(path).content`. |

All scanned directories accept only `*.md` files; non-Markdown files are silently skipped. Two further filters apply:

1. **Identity dedup** — multiple files with the same lowercased `name` field are deduplicated by `list_agent_sources` using a `seen: HashSet<String>` keyed on the lowercased name; the first file (in `list_agent_dirs` order) wins. Filenames do not participate in identity.
2. **Writability flag** — `SourceEntry.writable` is `true` for files in the standard user/repo directories, and `false` for files in `additional_roots` marked read-only (used by goose's managed / read-only-agent roots feature).

On this host (macOS), observed: `~/.claude/agents/` exists and contains 28 Claude Code-authored agent files (e.g. `agent-picker.md`, `database-expert.md`, `correctness-reviewer.md`) — Goose would surface these via the `~/.claude/agents/` compatibility path, but only the `name` / `description` / `body` fields are honored; `model: sonnet`, `tools: Read, Bash`, `skills: claude` are stored under `properties` and ignored. The host has no `~/.agents/agents/`, `~/.goose/agents/`, `~/Library/Application Support/Block/goose/agents/`, or `~/.config/goose/` directories today (no goose CLI is installed locally).

## Definition Format

A custom-agent file is a Markdown file with YAML frontmatter between `---` markers, where the body becomes the agent's instructions verbatim. Identity comes from the `name` field, not the filename or directory path.

```markdown
---
name: code-reviewer
description: Reviews code for correctness, maintainability, and risk
model: gpt-5.5
---

You are a senior code reviewer. Review changes for correctness, maintainability, security, and test coverage.
Be direct, prioritize issues by severity, and suggest concrete fixes.
```

Recognized frontmatter fields (per `summon.rs::parse_agent_content` and `sources.rs::parse_agent_frontmatter`):

- **Required**: `name` (≤80 chars, no `/` or `\`; spaces allowed; the source code slugs the filename but keeps the `name` field verbatim).
- **Recommended**: `description` (free text; surfaced in `load()` discovery and the @-mention picker).
- **Optional**: any other YAML key — stored verbatim under `properties` in the `SourceEntry`. Goose's summon extension reads only `name`, `description`, and `model` from the agent frontmatter; everything else passes through.
- **Effectively metadata-only**: `model` (Goose does not currently propagate the frontmatter `model` to the subagent runtime; the parent's `GOOSE_PROVIDER`/`GOOSE_MODEL` or an explicit `delegate(model: ...)` override wins).

The body is loaded via `load(source: "<name>")` (added to the parent's context) or `delegate(source: "<name>", instructions: ...)` (run as an isolated subagent). External subagents use the `subagent:` block in `config.yaml`:

```yaml
subagent:
  args:
    - mcp-server
  bundled: true
  cmd: codex
  description: OpenAI Codex CLI Subagent
  enabled: true
  env_keys:
    - OPENAI_API_KEY
  envs: {}
  name: subagent
  timeout: 300
  type: stdio
```

Recipe-defined subrecipes live in `*.yaml` / `*.json` recipe files (`.yml` not supported) and surface as `SourceType::Subrecipe`:

```yaml
version: "1.0.0"
title: "Code Review Assistant"
description: "Specialized subagent for code quality and security analysis"
instructions: |
  You are a code review assistant. ...
extensions:
  - type: builtin
    name: developer
    display_name: Developer
    timeout: 300
    bundled: true
parameters:
  - key: focus_area
    input_type: string
    requirement: optional
    default: "general"
settings:
  goose_provider: "anthropic"
  goose_model: "claude-sonnet-4-20250514"
  temperature: 0.7
  max_turns: 50
```

A single agent file produces one `SourceEntry`; a single recipe file produces N `SourceEntry` rows (1 recipe + N subrecipes). Mixed-files (one file declaring multiple local agents) are NOT supported by Goose (the `*.md` agent format is one-agent-per-file, in contrast to Gemini CLI's "multi-remote-agent" variant).

## Runtime Behavior

A custom agent / subagent is delegated when the parent's model calls the **Summon** extension's `delegate` tool. Summon is default-enabled and ships unprefixed (so the parent model sees `delegate` and `load` directly, not as `summon__delegate`). The tool's parameters:

- **`instructions`** — ad-hoc task instructions; required when `source` is absent.
- **`source`** — name of a recipe, subrecipe, or agent; triggers the named-definition path.
- **`parameters`** — recipe parameter substitutions (only valid with `source`).
- **`extensions`** — allowlist of MCP server names; omitted ⇒ inherit all; empty array ⇒ none.
- **`provider`** / **`model`** / **`temperature`** — per-invocation overrides.
- **`max_turns`** — overrides recipe `settings.max_turns` and `GOOSE_SUBAGENT_MAX_TURNS`.
- **`context`** — extra system-prompt context (file contents, constraints) injected into the subagent's rendered prompt.
- **`working_dir`** — must be inside the parent's working directory.
- **`async`** — run in background; returns a task id immediately.

The summon extension rejects invocations from subagent sessions themselves (`SessionType::SubAgent` ⇒ "Delegated tasks cannot spawn further delegations"), and forces subagents to `GooseMode::Auto` regardless of the parent's mode. The parent model is steered toward calling `delegate` in three documented ways:

1. **Natural language** — `Use the code-reviewer agent to review this PR` triggers automatic delegation in `auto` mode.
2. **`@mention` picker** — `@code-reviewer review the diff` forces a specific subagent. The summon extension inspects every user prompt and injects a roster of available sources into the system prompt.
3. **Direct tool call** — the parent model invokes `delegate(source: "<name>", instructions: ...)` directly when it determines the task matches a source's description.

The child receives:

- A new session of its own (`SessionType::SubAgent`) with a unique `subagent_session_id`.
- The rendered `subagent_system.md` template, which embeds `subagent_id`, `max_turns`, `task_instructions` (recipe `instructions` with Jinja parameter substitution), `tool_count`, and `available_tools` (filtered through `is_tool_visible_to_model`).
- The parent's filtered extensions (default: inherit all enabled).
- Its own conversation history; NOT the parent's.

The child does **not** receive the parent's conversation history, the parent's hint files, the parent's `moim_message_*` working-memory blocks, or the parent's skill selections. The parent's first-class ACL (`permissions.allow`/`permissions.deny`) is process-wide, not per-subagent; subagents cannot enable or disable extensions even though `ext_manager` is otherwise available to parents.

The child's returned state to the parent:

- **Synchronous `delegate`** — the final assistant text via `CallToolResult::success(content)` plus `meta.subagent_session_id` for follow-up.
- **Asynchronous `delegate(..., async: true)`** — a task id (`YYYYMMDD_<seq>`) in `meta.subagent_session_id`. The parent collects the result via `load(source: "<task-id>")`, which blocks up to 5 minutes, or `load(source: "<task-id>", peek: true)` for status, or `load(source: "<task-id>", cancel: true)` to cancel.
- **Internal direct-prompt parallel** — `{ execution_summary: { total_tasks, successful_tasks, failed_tasks, execution_time_seconds }, task_results: [{ task_id, status, result }] }` from the structured parent tool result.
- **Failure** — `CallToolResult::error(text)` with `meta.subagent_session_id` and `task_status: "failed"` / `"cancelled"` / `"panicked"` retrievable via `load()`.

The parent session continues normally after any subagent outcome; a failed subagent does NOT mark the parent session as failed.

Subagent security constraints (per the public docs and enforced by the `summon` extension):

- **Allowed**: extension discovery (read-only), resource access from enabled extensions, extension tools (as filtered by recipe / parent).
- **Restricted**: subagent spawning (blocked by `SessionType::SubAgent` check), extension management (`enable` / `disable` blocked), schedule management (creating / modifying / deleting scheduled tasks blocked).

Recipe `sub_recipes[].sequential_when_repeated` controls whether repeated subrecipe calls in the same parent turn execute serially; the default is parallel. `delegate.async: true` parallelizes a single parent's many delegations up to `GOOSE_MAX_BACKGROUND_TASKS` (default 5). Completed background tasks live in the summon extension's `completed_tasks` map for `GOOSE_COMPLETED_TASK_TTL_SECS` (default 600s) before being dropped by `cleanup_completed_tasks`.

## Observability

Subagent starts and stops are visible to wrappers through **two coordinated surfaces**:

1. **`LoggingMessageNotification` server notifications** (the canonical MCP-side channel). Every subagent tool call fires a notification whose structured `data` is `{ type: "subagent_tool_request", subagent_id: <id>, tool_call: { name: <tool>, arguments: <json> } }` with `logger = "subagent:<id>"`. This is the same MCP notification shape that `crates/goose/src/agents/subagent_handler.rs::create_tool_notification` emits; `goose-desktop` consumes it to render the inline `[subagent:<id>] <tool> | <extension>` indicator. Wrappers that speak MCP can subscribe to this channel directly.

2. **`load()` tool-result `meta`** — `subagent_session_id`, `task_status` (`completed` / `failed` / `cancelled` / `panicked` / `running`), `turns_taken`, `duration_secs`. Surfaced by the summon extension on every `load(source: "<task-id>")` call.

Goose does NOT emit a separate `subagent_start` / `subagent_stop` event type on the `goose run --output-format stream-json` stream — subagent lifecycle IS the LoggingMessageNotification channel plus the `load()` meta. There is no documented goose CLI hook event for subagent lifecycle (the `crates/goose/src/hooks/` crate exists, but no public docs page lists a `subagent_start` / `subagent_stop` event name; the hooks topic research for goose records only what the docs show).

Session IDs: each subagent gets a stable `subagent_session_id` that doubles as the `load(source: "<id>")` argument for retrieval and the `subagent:<id>` logger name on tool-call notifications. Session IDs are 8-digit-date + sequence (e.g. `20260219_1`); the summon extension recognizes them via `is_session_id`. Subagent transcripts are written to the regular session store under the standard goose data dir (`~/Library/Application Support/Block/goose/data/` on macOS, `~/.local/share/goose/` on Linux, `%APPDATA%\Block\goose\data\` on Windows) — first-class `Session` records, not separate per-subagent files. There is no goose CLI equivalent of Claude Code's `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl`.

Wrapper-based resume: `goose session --resume --session-id <subagent_session_id>` re-enters the subagent session, but subagents do NOT auto-resume after `delegate` returns control to the parent. A parent that wants to address a specific subagent after the fact must use the `subagent_session_id` from the `delegate` meta and the `load(source: "<task-id>")` / `load(source: "<task-id>", peek: true)` / `load(source: "<task-id>", cancel: true)` surface for active tasks. Subagent sessions do NOT inherit the parent's conversation; replaying context means re-running with the same instructions.

## Portability

Goose custom agents are **not portable** across providers as-is. The Markdown body is provider-neutral when it does not reference goose-only tools or env vars, but the frontmatter vocabulary and the `delegate`/`load` mechanism are entirely goose-specific.

| Field / construct | Portable? | Rewrite target |
|---|---|---|
| `name` | partial | Goose is permissive (≤80 chars, no `/` `\`); other providers apply stricter rules |
| `description` | yes | Carries the routing signal across providers |
| `model` (frontmatter) | no | Goose-specific and not propagated to the subagent runtime; set the recipe's `settings.goose_model` block instead |
| `properties` HashMap | no | Goose's arbitrary-key bag; other providers do not understand it |
| Body prompt | partial | Verbatim when it does not reference goose-only tools or env vars |
| `delegate.extensions: [...]` (per-invocation) | no | Goose-specific tool-allowlist semantics; Claude Code uses frontmatter `tools`, Codex has no equivalent |
| `delegate.max_turns` + `GOOSE_SUBAGENT_MAX_TURNS` (default 25) | no | Other providers use different cap semantics |
| `summon` extension name | no | Goose-specific (the underlying `delegate`/`load` tool is the analog of Claude Code's `Agent` tool) |
| Recipe `sub_recipes:` | no | Goose-specific; the recipe layer itself is goose-specific |
| External MCP `subagent:` block in `config.yaml` | no | Goose-specific (canonical way to wrap an external CLI like Codex as a goose subagent) |
| 5-minute default subagent wall-clock timeout | no | Goose-specific; configurable via natural language only |
| `subagent_tool_request` LoggingMessageNotification | no | Goose-specific notification channel (other providers do not emit this shape) |

Claude Code-authored agent files discovered via `~/.claude/agents/*.md` and `.claude/agents/*.md` parse as Goose agents with two caveats:

1. Claude-only frontmatter fields (`tools`, `model: sonnet`, `permissionMode`, `mcpServers`, `skills`, `hooks`, etc.) are stored under `properties` and ignored by Goose — the subagent runs with the parent's tools and parent's mode (forced to `Auto` for subagents).
2. The body prompt usually references Claude Code-only tool names (`Read`, `Bash`, `Grep`, `Edit`, `Write`) that Goose cannot resolve, so a body rewrite is required before the agent can run as a Goose subagent.

External subagents (the `subagent:` MCP stdio block in `config.yaml`) are an interesting portability case: the wrapper is goose-specific, but the wrapped tool (e.g. Codex itself) can usually run in a host-provider environment. A wrapper that wants to port a goose external subagent to another provider must rewrite the wrapper config into the target provider's MCP server syntax.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy`/`resume` actions, what matters about Goose subagents:

- Treat `~/.agents/agents/*.md` as the canonical user-scope location; treat `.agents/agents/*.md` (and the compatibility aliases `.goose/agents/*.md`, `.claude/agents/*.md`) as the canonical project-scope locations. Walk the on-disk locations with a YAML frontmatter parser. Use `name` for identity (not the filename) and capture `description` and `model` (knowing the frontmatter `model` is documentation-only on goose today). Preserve arbitrary frontmatter keys under a `properties` bag to mirror Goose's own `SourceEntry.properties`.
- Walk `~/.config/goose/config.yaml` (or its Windows equivalent) and surface each `subagent:` MCP stdio entry as a synthetic `SourceType::Agent` row whose `path` is the YAML key and whose `properties.bundled` / `properties.cmd` / `properties.args` / `properties.env_keys` carry the MCP entry's config.
- Walk recipe directories (`GOOSE_RECIPE_PATH`, working-dir `.goose/recipes/`, `.agents/recipes/`, `~/.goose/recipes/`, `~/.agents/recipes/`, `Paths::config_dir().join("recipes")`) and surface each `*.yaml` / `*.json` recipe as a `SourceType::Recipe` row plus N `SourceType::Subrecipe` rows for the recipe's `sub_recipes`. The recipe fields `instructions`, `prompt`, `extensions`, `parameters`, `settings.max_turns`, `settings.goose_provider`, `settings.goose_model`, `settings.temperature` are the per-definition metadata to capture.
- A linked Goose agent is portable when its body uses only standard Markdown and its frontmatter carries only `name`, `description`, plus optional `model` and a `properties` bag that already targets the destination provider's vocabulary. Flag assets that depend on `delegate.extensions` (per-invocation allowlist), `delegate.max_turns`, recipe `sub_recipes` entries, external `subagent:` blocks in `config.yaml`, or body prompts that reference goose-only tools (`developer__shell`, `summon__delegate`, `summon__load`, `todo__*`) or goose-only env vars (`$GOOSE_PROVIDER`, `$GOOSE_MODEL`, `$GOOSE_MODE`, `$GOOSE_SUBAGENT_MAX_TURNS`, `$AGENT_SESSION_ID`, `AGENT=goose`).
- For lifecycle `proxy`/`resume`: the wrapper must capture and replay `subagent_session_id` for the subagent it wants to address. The same id is the `load(source: "<task-id>")` argument for retrieving async background tasks. The LoggingMessageNotification channel (`logger = "subagent:<id>"`, `data.type = "subagent_tool_request"`, `data.subagent_id`, `data.tool_call.{name, arguments}`) is the canonical way a wrapper sees subagent tool calls in real time — it is NOT the regular `goose run --output-format stream-json` stream. Subagent transcripts live in the regular goose data dir as first-class `Session` records (no separate per-subagent JSONL path), so `goose session --export --session-id <id>` is the diagnostic entry point.
- Model / turn resolution: `delegate.model` → `recipe.settings.goose_model` → frontmatter `model` (documentation-only) → parent's `GOOSE_MODEL`. Turn cap: `delegate.max_turns` → `recipe.settings.max_turns` → `GOOSE_SUBAGENT_MAX_TURNS` (default 25). Mode is forced to `Auto` for subagents; subagents cannot escalate to `approve` / `chat` / `smart_approve` modes.
- Permission policy: when the parent uses `approve` / `smart_approve` / `chat`, subagents are disabled entirely. When the parent is in `auto`, subagents are forced into `auto` too; `GOOSE_ALLOWLIST` applies process-wide. The `SECURITY_PROMPT_*` family applies to subagent runs identically. Subagents cannot enable or disable extensions even though `ext_manager` is otherwise available to parents.
- Whenever Claudine's wrapper code grows a Goose-aware `claudine agents` row or a subagent addressing path, it must respect the summon extension's `is_session_id` discriminator (8-digit-date + sequence task ids) and the seven-directory discovery order (`list_agent_dirs` in `sources.rs`). The same walker must also avoid silently promoting `SourceType::Agent` to a "session-listing" surface — `SessionType::SubAgent` entries are subagents, not primary sessions, and should be marked as such in any diagnostic output.

## Sources

- [Goose — Custom Agents (canonical)](https://goose-docs.ai/docs/guides/context-engineering/custom-agents)
- [Goose — Subagents (canonical)](https://goose-docs.ai/docs/guides/context-engineering/subagents)
- [Goose — Configuration Files](https://goose-docs.ai/docs/guides/config-files)
- [Goose — Environment Variables](https://goose-docs.ai/docs/guides/environment-variables)
- [Goose — Recipe Reference Guide](https://goose-docs.ai/docs/guides/recipes/recipe-reference)
- [Goose — CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [Goose — Hooks](https://goose-docs.ai/docs/guides/context-engineering/hooks)
- [Goose — Prompt Templates](https://goose-docs.ai/docs/guides/context-engineering/prompt-templates)
- [Goose — Subrecipes](https://goose-docs.ai/docs/guides/recipes/subrecipes)
- [Goose — Multi-Model Configuration](https://goose-docs.ai/docs/guides/multi-model/)
- [Goose — Tutorial: Using Subagents](https://goose-docs.ai/docs/tutorials/subagents)
- [Goose — homepage (AAIF)](https://goose-docs.ai/)
- [Goose — moved to AAIF announcement](https://goose-docs.ai/blog/2026/04/07/goose-moves-to-aaif)
- [goose repo — `crates/goose/src/sources.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/sources.rs) (agent file discovery: `list_agent_dirs`, `is_global_agent_file`, `agent_source_entry`, `parse_agent_frontmatter`)
- [goose repo — `crates/goose/src/agents/platform_extensions/summon.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/platform_extensions/summon.rs) (delegate / load tool, `discover_filesystem_sources`, async background tasks, `is_session_id`)
- [goose repo — `crates/goose/src/agents/subagent_handler.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/subagent_handler.rs) (`SUBAGENT_TOOL_REQUEST_TYPE`, `create_tool_notification`, `run_subagent_task`, `subagent_system.md` rendering)
- [goose repo — `crates/goose/src/agents/subagent_task_config.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/subagent_task_config.rs) (`DEFAULT_SUBAGENT_MAX_TURNS = 25`, `GOOSE_SUBAGENT_MAX_TURNS` lookup)
- [goose repo — `crates/goose/src/agents/platform_extensions/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/platform_extensions/mod.rs) (Summon registration, default-enabled, unprefixed)
- [goose repo — `crates/goose/src/config/paths.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs) (`Paths::agents_dir()`, `GOOSE_PATH_ROOT` semantics)
- [goose repo — `crates/goose/src/agents/prompt_manager.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/prompt_manager.rs) (parent-system-prompt construction with `enable_subagents` flag)
- Local inspection on 2026-07-03: `ls ~/.claude/agents/` (28 Claude Code agent files visible to goose via the `~/.claude/agents/` compatibility path); `~/.agents/`, `~/.goose/`, `~/Library/Application Support/Block/goose/`, `~/.config/goose/` not present on this host (no goose CLI installed locally).