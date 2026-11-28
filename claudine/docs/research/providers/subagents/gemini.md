---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
subagent_docs: https://geminicli.com/docs/core/subagents/

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.gemini/agents/
    notes: "Personal subagents loaded for every session. Each *.md file with a valid YAML frontmatter is parsed; identity comes from the `name` field, not the filename. Reload at runtime via `/agents reload`."
  - os: linux
    scope: user
    path: ~/.gemini/agents/
    notes: "Same as macOS. `$HOME` resolves the same way under WSL and Linux."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\agents\\"
    notes: "Same as macOS / Linux on native Windows; resolves relative to %USERPROFILE%. WSL reads the Linux-side path inside the distro."
  - os: macos
    scope: repo
    path: ".gemini/agents/"
    notes: "Project-scoped subagents, checked into the repo so a team can share them. Loaded alongside user agents on session start. Same precedence rules apply when both directories define the same `name`."
  - os: linux
    scope: repo
    path: ".gemini/agents/"
    notes: "Same as macOS. Project trust (see trustedFolders) is not required to load the directory, but untrusted projects otherwise refuse to run mutating tools."
  - os: windows
    scope: repo
    path: ".gemini\\agents\\"
    notes: "Same as macOS / Linux on native Windows. On WSL the project directory is read from the WSL-side path."
  - os: macos
    scope: extension
    path: "<extension>/agents/<name>.md"
    notes: "Extension-bundled subagents. Loaded automatically when the extension is enabled (`gemini extensions enable <name>`). Identities are flat (no namespace); a custom local agent with the same `name` collides on precedence rules."
  - os: linux
    scope: extension
    path: "<extension>/agents/<name>.md"
    notes: "Same as macOS. Extensions themselves live under `~/.gemini/extensions/<name>/` (user) or `<project>/.gemini/extensions/<name>/` (workspace)."
  - os: windows
    scope: extension
    path: "<extension>\\agents\\<name>.md"
    notes: "Same as macOS / Linux. The `gemini-extension.json` manifest plus the bundled `agents/` directory are the entry points."
  - os: macos
    scope: system
    path: "<admin-managed extensions>/agents/<name>.md"
    notes: "No fixed system agents/ directory is documented. Enterprise-managed agent definitions ship via admin-managed extensions or the policy engine, not via a fixed /etc or /Library path."
  - os: linux
    scope: system
    path: "<admin-managed extensions>/agents/<name>.md"
    notes: "Same as macOS. Gemini CLI does not define an /etc/gemini-cli/agents/ directory; system agents arrive via admin extensions."
  - os: windows
    scope: system
    path: "<admin-managed extensions>\\agents\\<name>.md"
    notes: "Same as macOS / Linux. No fixed ProgramData agents/ directory documented."
  - os: macos
    scope: other
    path: inline `agents.overrides` and `agents.browser` blocks in settings.json
    notes: "Global overrides (enable/disable, runConfig, modelConfig) live under `agents.overrides.<agent_name>` in `~/.gemini/settings.json`, `.gemini/settings.json`, or `/Library/Application Support/GeminiCli/settings.json`. They are not definition files; they retune built-in agents without redeclaring their system prompt."
  - os: linux
    scope: other
    path: inline `agents.overrides` and `agents.browser` blocks in settings.json
    notes: "Same as macOS. Override blocks merge into the agent's resolved definition at load time; per-agent `enabled`, `modelConfig`, and `runConfig` are the canonical knobs."
  - os: windows
    scope: other
    path: inline `agents.overrides` and `agents.browser` blocks in settings.json
    notes: "Same as macOS / Linux. The `agents.browser` block only configures the `browser_agent` (sessionMode, headless, profilePath, visualModel, allowedDomains, disableUserInput, maxActionsPerTask, confirmSensitiveActions, blockFileUploads)."

format:
  file_names:
    - "*.md"
  frontmatter: true
  required_fields:
    - name (slug — lowercase letters, numbers, hyphens, underscores; identity carrier, not the filename)
    - description (short text shown to the main agent to drive automatic routing)
  optional_fields:
    - "kind (`local` default | `remote` — selects the local loader or the Agent2Agent client)"
    - "tools (array of tool names with wildcards `*`, `mcp_*`, `mcp_<server>_*`; omitted → inherit all parent tools)"
    - "mcpServers (object of inline MCP server definitions isolated to this agent; stdio/http/sse shape matches the global MCP catalog)"
    - "model (Gemini model id or alias; defaults to `inherit` — uses the parent session model)"
    - "temperature (number 0.0–2.0; default 1)"
    - "max_turns (number; default 30)"
    - "timeout_mins (number; default 10)"
    - "agent_card_url (remote subagents — A2A card endpoint URL; required if `agent_card_json` is absent)"
    - "agent_card_json (remote subagents — inline A2A card JSON string; required if `agent_card_url` is absent)"
    - "auth (remote subagents only — `{ type, key|token|username|password|value, name?, scheme?, scopes? }`; `type` ∈ apiKey | http | google-credentials | oauth)"
    - "Body (Markdown — becomes the agent's system prompt verbatim)"
  body_format: markdown
  notes: |
    Local agents are Markdown files starting with YAML frontmatter between triple-dash markers; the body is the agent's system prompt. Identity comes from the `name` field, not the filename. The loader does NOT prepend the main agent's system prompt — subagents receive only their own body plus a small environment summary.

    A single file can declare multiple remote subagents via a YAML list of `name:`/`agent_card_url:` records (a "multi-subagent" file). Mixed local/remote lists and multiple local agents in one file are explicitly NOT supported. Multiple local agents must each live in their own `.md` file.

    The schema is enforced by the loader; invalid `name` slugs, missing required fields, or unknown `tools` values cause the file to be skipped with a warning rather than aborting the session. See the body for full local, remote, and multi-remote definition examples.

    Built-in agent names that ship with Gemini CLI (defaults; tunable via `agents.overrides`):

    | Name | Default | Purpose |
    |---|---|---|
    | `codebase_investigator` | enabled | Read-only codebase mapping / dependency analysis |
    | `cli_help` | enabled | Gemini CLI itself, its commands, configuration |
    | `generalist` | enabled | Generic full-tool subagent for resource-heavy subtasks |
    | `browser_agent` | disabled | Chrome via the bundled `chrome-devtools-mcp` |

    Default `agents.overrides.<name>` keys documented: `enabled`, `modelConfig.model`, `runConfig.maxTurns`, `runConfig.maxTimeMinutes`. A custom user/repo agent whose `name` matches a built-in shadows the built-in.

runtime:
  invocation: |
    Three surfaces invoke a subagent:

    (1) **Automatic delegation** — the main agent's system prompt instructs it to call the subagent's tool when the task matches the `description`. e.g. "How does auth work?" routes to `codebase_investigator`. There is no separate router; the routing decision lives in the prompt itself, weighted by the `description` field.

    (2) **Force via `@` syntax** — type `@<agent-name>` at the start of a prompt to skip the router and call that subagent directly. Example: `@codebase_investigator Map out the relationship between AgentRegistry and LocalAgentExecutor.` The CLI injects a system note that nudges the primary model to use that specific subagent tool immediately.

    (3) **`/agents` slash command** — `/agents list`, `/agents reload` (alias `/agents refresh`), `/agents enable <name>`, `/agents disable <name>`, `/agents config <name>` (open a model/temperature/runConfig dialog). This is the discovery and lifecycle-management surface, not a runtime invocation surface — `/agents` does not itself delegate.

    No CLI flag (`--agent <name>` equivalent) launches a session directly as a specific subagent; the main thread is always the default Gemini agent, and delegation is mid-session via tool call.

    Subagents are exposed to the main agent as **tools of the same name** in its tool registry. The main agent's call into `codebase_investigator` is recorded as a `tool_use` event; the subagent's final assistant text becomes the `tool_result`.
  parent_child_context: |
    Each subagent runs in an independent context loop:

    - **Focused context** — the subagent's conversation history does not bloat the main agent's context window. The main agent sees only the subagent's final assistant message (as a tool result).
    - **Isolated tools** — the subagent only sees tools granted by its `tools:` list (or the `*` wildcard). MCP servers from the parent session are not automatically inherited; the subagent must redeclare them via `mcpServers:` or rely on the parent's MCP tools passing through the `*` / `mcp_*` wildcards.
    - **Independent system prompt** — the loader does NOT prepend Gemini CLI's main system prompt. Subagents receive only their Markdown body as the system prompt, plus a short environment summary (CWD, platform).
    - **Return state** — the subagent's final assistant text becomes the parent tool result. The parent sees that text plus the standard tool envelope (start line, latency, error or success flag) and nothing else from the subagent's internal turns.

    There is no equivalent to Claude Code's "background subagent" — Gemini CLI runs every subagent synchronously from the parent's perspective. A subagent cannot be sent messages after it returns.
  permissions_inheritance: |
    Gemini CLI's policy engine treats each subagent name as a virtual tool alias. Rules can target a subagent's permission directly:

    - **Governing the subagent itself** — set `toolName = "<agent-name>"` (or `toolName = ["<agent-name>", ...]`) with `decision = "allow" | "deny" | "ask_user"`. A `deny` rule on a subagent tool completely hides that subagent from the model; the model never sees the tool as an option.
    - **Governing tool calls the subagent makes** — set `toolName = "<actual-tool>"` and add `subagent = "<agent-name>"`. The rule only fires when that specific subagent invokes the tool.
    - **No `subagent` field** — the rule applies universally across all agents (main + every subagent).
    - **Backward compatibility** — rules written against the historical 1:1 subagent tool names still match transparently.

    Tiered precedence (final_priority = tier_base + toml_priority/1000): Default=1, Extension=2, Workspace=3 (currently disabled, see issue #18186), User=4, Admin=5. Admin always wins; user overrides workspace and default.

    Approval modes interact: `default` prompts for write tools; `autoEdit` auto-approves certain edits; `plan` is read-only; `yolo` auto-approves everything. Rules can scope themselves to modes via `modes = [...]`. Persistent approvals granted in `plan` mode flow to all modes; approvals in `default` flow to `default`/`autoEdit`/`yolo`; approvals in `yolo` stay in `yolo` only.

    Inherited behavior: the subagent runs inside its own approval context; the parent session's approval mode does NOT automatically apply. A subagent's tool calls are evaluated against the policy rules as if it were the calling session, but the rule's `subagent` field must match the subagent name (or be absent) for it to fire.

    The default policy for agent delegation is `ask_user` to ensure remote agents can prompt for confirmation; local sub-agent actions are checked individually per tool call.
  model_inheritance: |
    Resolution order (highest precedence first):

    1. **`agents.overrides.<name>.modelConfig.model`** in settings.json (system / user / project).
    2. **`modelConfigs.overrides`** entry whose `match.overrideScope` equals the agent's name.
    3. **Frontmatter `model`** on the agent definition file.
    4. **`inherit`** — fall back to the parent session's model.

    Model IDs are Gemini model names or aliases (`gemini-3-flash-preview`, `gemini-2.5-pro`, `flash`, `pro`, `auto`, ...). Aliases resolve via `modelConfigs.aliases`. The `model` field accepts `inherit` as an explicit value, meaning "use the parent's model"; the omission default is also `inherit`.

    Per-agent `temperature` defaults to `1` (not 0). `max_turns` defaults to `30`. `timeout_mins` defaults to `10`. `modelConfigs.overrides` can also inject `generateContentConfig` (temperature, topP, topK, thinkingConfig/thinkingBudget/thinkingLevel, safetySettings) keyed by `overrideScope` so a single model alias can carry different generation configs per subagent.
  tool_inheritance: |
    By default, a subagent inherits every tool the parent has access to (including all MCP tools). The agent definition can narrow or replace that set:

    - **`tools:` array** — explicit allowlist. Names are matched exactly against the global tool registry (built-ins like `read_file`, `grep_search`, `glob`, `run_shell_command`, `write_file`, `replace`, plus MCP-tool FQNs like `mcp_<server>_<tool>`).
    - **Wildcards** — `*` (all tools, built-in and MCP), `mcp_*` (every MCP tool from every server), `mcp_<server>_*` (every tool from one named server).
    - **`mcpServers:`** — defines inline MCP servers isolated to this agent. These are launched for the subagent only; they do not leak into the parent session.
    - **No denylist** — Gemini CLI does not expose a `disallowedTools:` analogue. To deny a tool for a subagent, write a policy rule with `subagent = "<name>"` and `decision = "deny"`.

    **Recursion protection is built in.** Subagents cannot call other subagents. Even when the subagent is granted the `*` wildcard, agent-to-agent tool calls are filtered out — the subagent simply does not see the other agent tools. This is enforced by the loader, not by configuration.

    The Policy Engine is the canonical way to scope which tools a subagent can call. Policy rules with `subagent = "<name>"` apply only to that subagent's tool invocations; without `subagent`, the rule applies to every caller.
  max_turns: |
    Per-agent `max_turns` frontmatter field (number; default 30). "Maximum number of conversation turns allowed for this agent before it must return." Beyond the limit, the subagent is terminated and the parent sees an error in the tool result.

    Related but separate: `timeout_mins` (default 10) bounds wall-clock time, and `agents.overrides.<name>.runConfig.maxTimeMinutes` overrides it globally. The Policy Engine can also apply a turn cap per session via the headless exit-code 53 ("Turn limit exceeded").

    No global maximum-recursion-depth knob is documented — recursion is not possible because subagents cannot invoke other subagents.
  notes: |
    Built-in precedence rules when the same `name` appears in multiple sources: user-scope wins over extension-scope, project-scope wins over user-scope when both define the same `name`. A custom agent whose name matches a built-in (e.g. `codebase_investigator`) shadows the built-in entirely; the built-in is replaced, not merged.

    Wildcard merging: when a subagent inherits `tools: *` from the parent, MCP tools are included. The `*` wildcard does NOT, by itself, disable the agent-to-agent recursion filter — subagent tool calls remain invisible.

    Failure behavior: a subagent that hits `max_turns` or `timeout_mins` returns an error tool result to the parent; the parent's session continues. The subagent does NOT mark the parent session as failed. A subagent that fails to start (bad frontmatter, missing `name`, invalid `kind`) is silently dropped at load time with a warning.

    The `codebase_investigator` agent is the canonical read-only mapping subagent — its system prompt limits it to read tools, so it cannot perform mutations even if the parent session is in `yolo`.

observability:
  stream_events:
    - "init (session metadata — session id, model)"
    - "message (user/assistant chunks)"
    - "tool_use (tool call request — used for subagent invocations; the subagent's `name` appears as `tool_use.name`)"
    - "tool_result (subagent final response; error or success envelope carries the subagent's exit reason)"
    - "error (non-fatal warnings / system errors)"
    - "result (final outcome with aggregated stats; per-model token usage breakdowns)"
  hook_events:
    - "BeforeTool (matcher can target the subagent's tool name — fires when the main agent calls a subagent)"
    - "AfterTool (matcher can target the subagent's tool name — fires when the subagent returns; `tool_response` carries the final assistant text and any error)"
    - "BeforeAgent / AfterAgent (fire on the main agent loop, NOT on subagent invocation; cannot be scoped to a specific subagent by name)"
    - "BeforeModel / AfterModel (fire on every LLM call; can observe a subagent's model traffic indirectly by adding `hookSpecificOutput.llm_response` or by reading `llm_request.model` and `llm_request.messages`)"
    - "SessionStart / SessionEnd / Notification / PreCompress (lifecycle hooks; SessionEnd fires on session exit, not per subagent)"
  session_ids: true
  notes: |
    Subagent invocations are visible to a wrapper via three coordinated surfaces:

    1. **Stream JSON output** (`--output-format stream-json` or `json`): every subagent call is a `tool_use` event with `name = <subagent-name>` and `tool_input = { prompt, ... }`; the matching `tool_result` event carries the final assistant text. There is NO dedicated `subagent_start` / `subagent_stop` event type in the stream — subagent lifecycle IS the tool call lifecycle.

    2. **Hooks**: `BeforeTool` / `AfterTool` hooks can match on the subagent name to wrap each call. `tool_input.prompt` carries the parent's delegated prompt; `tool_response.llmContent` carries the subagent's final assistant text; `tool_response.error` carries any failure. Common fields on every hook: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `timestamp`. Subagent-internal tool calls (the subagent using `read_file`, `grep_search`, etc.) ALSO fire `BeforeTool` / `AfterTool` with the subagent name available via the policy rule's `subagent` field — but the hook itself does not currently expose `subagent` in its payload, so wrappers must match via the call site (the parent's `AfterTool` for the subagent's invocation vs the subagent's own `BeforeTool` for read_file).

    3. **Transcripts**: every session writes a JSONL transcript at the path reported in `session_id` and `transcript_path`. There is no separate per-subagent transcript (no equivalent to Claude Code's `~/.claude/projects/.../subagents/agent-{id}.jsonl`); the subagent's intermediate tool calls are inline in the parent's transcript, indented under the parent tool_use entry.

    Watch the `tool_use` envelope for subagent identity: `tool_use.name` carries the subagent slug (e.g. `codebase_investigator`). Pair it with `tool_result.error` to detect failures. Headless exit code 53 indicates the session hit its turn cap before any subagent returned.

    Wrapper-based resume: Gemini CLI has `/resume <tag>` and `--resume <index>` for restoring the parent's session; it does not document a per-subagent resume handle. Subagent state is not independently resumable.

portability:
  portable: false
  non_portable_assets:
    - "`name` slug (lowercase letters, numbers, hyphens, underscores — different from Claude Code's no-underscore rule and from Codex's TOML-friendly any-string rule)"
    - "`tools:` array — Gemini CLI tool names (`read_file`, `grep_search`, `glob`, `run_shell_command`, `write_file`, `replace`, ...); MCP FQNs use `mcp_<server>_<tool>` (single underscore delimiter)"
    - "`mcpServers:` inline MCP server definitions (Gemini's MCP shape — `command`/`args`/`cwd` for stdio, `url`/`headers` for HTTP/SSE; differs from Claude Code's `stdio`/`http`/`sse`/`ws` discriminated union)"
    - "`temperature`, `max_turns`, `timeout_mins` (Gemini-specific; not all providers share turn-cap semantics — Claude Code's `maxTurns` is in turns, Codex has per-session cap knobs, Gemini's `timeout_mins` is wall-clock)"
    - "`kind: local | remote` (Gemini-specific discriminator; Claude Code uses an `agent` parameter, Codex uses a name match in `~/.codex/agents/`)"
    - "`agent_card_url`, `agent_card_json`, `auth.*` (Agent2Agent-protocol fields; meaningless outside A2A-compatible clients)"
    - "`model` aliases (`flash`, `pro`, `auto`, `gemini-3-flash-preview`) — Gemini-specific model catalog; `inherit` is the Gemini default and must be rewritten when targeting providers that lack an inherit-equivalent"
    - "Built-in agent names `codebase_investigator`, `cli_help`, `generalist`, `browser_agent` (no equivalents on other providers)"
    - "`agents.overrides.<name>` block in settings.json (Gemini-specific; the per-agent model/temperature/runConfig knobs do not map 1:1 to other providers)"
    - "Body prompt text that references Gemini-only tools (`read_file`, `grep_search`, `run_shell_command`), env vars (`$GEMINI_API_KEY`, `$GOOGLE_API_KEY`), or environment markers"
    - "Tool wildcards `mcp_*` and `mcp_<server>_*` (Gemini-specific; Claude Code uses `mcp__server` / `mcp__server__*` patterns)"
  rewrite_needed: true
  notes: |
    The Markdown + YAML-frontmatter shape is close to Claude Code's, but the frontmatter vocabulary does not overlap enough for a verbatim link.

    Cross-provider rewrite map (Claude Code → Gemini CLI):

    | Claude Code field | Gemini CLI equivalent | Notes |
    |---|---|---|
    | `name` | `name` | Slug rule differs (Claude disallows underscores; Gemini allows them) |
    | `description` | `description` | Verbatim |
    | `tools` (e.g. `Read`, `Glob`, `Grep`, `Bash`, `Write`, `Edit`) | `tools` (e.g. `read_file`, `glob`, `grep_search`, `run_shell_command`, `write_file`, `replace`) | **Tool names must be remapped** |
    | `disallowedTools` | (no direct field) | Rewrite to a Policy Engine TOML rule with `subagent = "<name>"` and `decision = "deny"` |
    | `model` | `model` | Claude aliases (`sonnet`, `opus`) → Gemini aliases (`pro`, `flash`) |
    | `maxTurns` | `max_turns` | Rename only |
    | `permissionMode` | (no direct field) | Gemini's approval mode is session-wide via `--approval-mode`; per-subagent approval must go through Policy Engine rules |
    | `mcpServers` | `mcpServers` | Shape is similar but Gemini uses flat `command`/`args`/`cwd` instead of Claude's `stdio` discriminated union |
    | `hooks` (subagent-scoped) | (no direct field) | Hooks are global (in settings.json), not per-subagent; rewrite to global hooks that match on the subagent's tool name |
    | `skills` (preload list) | (no equivalent) | Gemini's skills are activated by the model, not preloaded; convert to `GEMINI.md` references |
    | `memory` | (no equivalent) | Gemini has no per-agent MEMORY.md scoping; persistent context is hierarchical GEMINI.md |
    | `background` / `isolation` / `effort` / `color` | (no equivalents) | Drop |
    | `initialPrompt` | (no equivalent) | Convert to a `/<name>` slash command or a `prompt:` field on a custom command |

    Cross-provider rewrite map (Codex → Gemini CLI):

    | Codex field | Gemini CLI equivalent | Notes |
    |---|---|---|
    | `name` (TOML) | `name` (YAML) | Slug rule differs; Codex allows free strings, Gemini enforces slug shape |
    | `description` | `description` | Verbatim |
    | `developer_instructions` | Body Markdown | Codex's TOML field moves into the Markdown body (the file body IS the system prompt) |
    | `model` | `model` | Map Codex `gpt-*` ids to Gemini model ids; `inherit` is Gemini's default |
    | `model_reasoning_effort` | `modelConfigs.overrides` | Use the `overrideScope` field to scope reasoning effort to the agent name |
    | `sandbox_mode` | Policy Engine `decision` | Rewrite to policy rules restricting mutating tools |
    | `approval_policy` | `--approval-mode` (session) | Approval is session-scoped; per-agent overrides go through Policy Engine |
    | `mcp_servers.<id>` | `mcpServers.<id>` | Shape is similar; Gemini CLI also has a global MCP catalog |
    | `[[skills.config]]` | (no equivalent) | Gemini has a separate Skills system; link via the Skill tool, not as agent metadata |
    | `nickname_candidates` | (no equivalent) | Drop |

    Portable bits (link-as-is):

    - The Markdown body when it does not reference provider-specific tools / env vars.
    - The `description` field's routing intent — copy across, possibly rewrite to the target provider's router language.

    Local agents bundled inside extensions: when an extension's `agents/<name>.md` is the definition source, the extension manifest also matters; the linker must decide whether to keep the agent in the extension or extract it.

cli_params:
  - flag: --approval-mode <default|auto_edit|yolo|plan>
    description: "Set the session approval mode (also drives the policy tier the session evaluates tools against). Subagents do not inherit approval mode per se; their tool calls are checked individually against Policy Engine rules. Combine with `subagent`-scoped policy rules for fine-grained control."
    example: "gemini --approval-mode yolo"
  - flag: --policy <file-or-directory> [...]
    description: "Additional policy files or directories to load (comma-separated or repeated). Use this to ship per-subagent policy rules (`subagent = \"<name>\"`) alongside the agent definition."
    example: "gemini --policy ./policies"
  - flag: --admin-policy <file-or-directory> [...]
    description: "Admin-tier policy files or directories (highest precedence; same tier as the system policy directory). Use for enterprise deployment to enforce subagent allow/deny."
    example: "gemini --admin-policy ./admin-policies"
  - flag: --allowed-mcp-server-names <names...>
    description: "Restrict MCP servers by name. Applies to the parent session and (via the `*` / `mcp_*` inheritance) to subagents that use the `*` wildcard. Inline `mcpServers:` declared on a subagent definition are unaffected."
    example: "gemini --allowed-mcp-server-names chrome-devtools-mcp"
  - flag: --allowed-tools <names...>
    description: "DEPRECATED: use the Policy Engine (`toolName` allowlist with `decision = \"allow\"`) instead. Persists tools that should run without confirmation; useful for pre-approving the read tools a subagent will use."
    example: "gemini --allowed-tools read_file grep_search"
  - flag: --extensions <names...>
    description: "Limit which extensions are active for this session (comma-separated list). Removing an extension strips its bundled subagents."
    example: "gemini --extensions workspace,security-audit"
  - flag: --list-extensions
    description: "List all available extensions and exit. Subagents bundled inside extensions are listed indirectly (via the extension name)."
    example: "gemini --list-extensions"
  - flag: --include-directories <dirs...>
    description: "Add directories to the workspace; mirrors `/directory add` mid-session. If an added directory contains `.gemini/agents/`, those agents are loaded for this session."
    example: "gemini --include-directories ../shared-lib"
  - flag: --output-format <text|json|stream-json>
    description: "Stream-json output exposes every subagent invocation as `tool_use` (`name = <subagent>`) and the response as `tool_result`. JSON collapses the run into a single response object. Use stream-json for subagent lifecycle observation."
    example: "gemini --output-format stream-json"
  - flag: --resume <index-or-tag>
    description: "Resume a previous session (auto-saved by index or manually checkpointed by tag). Resumes the parent's session; subagent state is not independently resumable."
    example: "gemini --resume 5"
  - flag: --list-sessions
    description: "List available sessions for the current project and exit."
    example: "gemini --list-sessions"
  - flag: --session-id <uuid>
    description: "Start a new session with a manually provided UUID. Useful for deterministic log correlation in wrapper-driven runs."
    example: "gemini --session-id 11111111-2222-3333-4444-555555555555"
  - flag: --acp / --experimental-acp
    description: "Start Gemini CLI in ACP (Agent Client Protocol) mode. ACP is the cross-provider integration surface; it exposes session and prompt primitives that wrap the same subagent delegation model."
    example: "gemini --acp"
  - flag: --raw-output / --accept-raw-output-risk
    description: "Disable sanitization of model output (allow ANSI escape sequences). Affects how the parent's `tool_result` for a subagent is rendered; the wrapper may need to enable this for terminal-side visualization."
    example: "gemini --output-format stream-json --raw-output --accept-raw-output-risk"
  - flag: --skip-trust
    description: "Trust the current workspace for this session. Project `.gemini/agents/` is loaded regardless, but untrusted workspaces otherwise refuse mutating tools; this flag avoids the interactive trust prompt."
    example: "gemini --skip-trust"
  - flag: -s, --sandbox / -y, --yolo
    description: "Sandbox flag (`-s`) runs the session sandboxed; `-y` enables YOLO mode (auto-approve all actions, including subagent tool calls). Sandboxing affects subagents too — they inherit the parent's sandbox."
    example: "gemini -s -y"
  - flag: -m, --model <model>
    description: "Set the parent session model. Subagents with `model: inherit` (the default) use this value. Subagents with an explicit `model:` override it."
    example: "gemini -m gemini-2.5-pro"

env_vars:
  - name: GEMINI_API_KEY
    effect: "API key for the Gemini Developer API. Drives OAuth-free authentication for the parent session and (via inheritance) subagents."
  - name: GOOGLE_API_KEY
    effect: "API key for the Gemini Developer API (alternative name). Mutually exclusive with `GEMINI_API_KEY` at runtime."
  - name: GOOGLE_GENAI_USE_VERTEXAI
    effect: "When `true`, switch the parent session to Vertex AI authentication. Subagents inherit this choice."
  - name: GOOGLE_CLOUD_PROJECT
    effect: "Google Cloud project ID for paid Code Assist licenses or Vertex AI routing."
  - name: GOOGLE_APPLICATION_CREDENTIALS
    effect: "Path to a service account key for ADC. Required when a remote subagent uses `auth.type = \"google-credentials\"` and the local environment is not already authenticated."
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: "Override the default path to the system defaults settings file. Defaults: `/etc/gemini-cli/system-defaults.json` (Linux), `C:\\ProgramData\\gemini-cli\\system-defaults.json` (Windows), `/Library/Application Support/GeminiCli/system-defaults.json` (macOS)."
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: "Override the default path to the system overrides settings file. Defaults: `/etc/gemini-cli/settings.json` (Linux), `C:\\ProgramData\\gemini-cli\\settings.json` (Windows), `/Library/Application Support/GeminiCli/settings.json` (macOS)."
  - name: GEMINI_SANDBOX
    effect: "Select the sandbox backend (e.g. `docker`). When set, the subagent's tool calls run inside the same sandbox as the parent."
  - name: SANDBOX_PORTS
    effect: "Comma-separated list of host ports to forward into the sandbox (e.g. `9222` for the browser agent's Chrome DevTools connection)."
  - name: HTTP_PROXY / HTTPS_PROXY
    effect: "Standard HTTP/HTTPS proxy variables. Used by remote (A2A) subagents when `general.proxy` is not set."
  - name: GEMINI_CLI=1
    effect: "Set by Gemini CLI on subprocesses invoked via `!` or shell mode. Allows scripts to detect they are running inside a Claude/Gemini CLI session. Not subagent-specific but worth noting."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `linking` module does not currently recognize Gemini CLI's user-level `~/.gemini/agents/*.md` and project-level `.gemini/agents/*.md` directories. The `claudine agents` command needs a Gemini-aware row to enumerate these files alongside the existing Claude Code and Codex CLI rows.

  Five concrete Claudine follow-ups:

  1. **Add a Gemini CLI agent linker** that mirrors the Claude Code / Codex walkers for `~/.gemini/agents/*.md`, `.gemini/agents/*.md`, and bundled `<extension>/agents/*.md`. The walker should read YAML frontmatter (not TOML), extract the `name` / `description` / `kind` / `tools` / `model` / `mcpServers` fields, and treat remote agents (`kind: remote`) as a distinct category because they ship an `agent_card_url` / `agent_card_json` that links to an external service.

  2. **Adapt the field-mapping table** to translate Gemini's frontmatter (`max_turns`, `timeout_mins`, `temperature`, `mcpServers`, `kind`, `agent_card_url`, `auth`) into Claudine's portable metadata model. `max_turns` and `timeout_mins` map cleanly to a "limits" row; `kind: remote` is a Claudine-only discriminator and needs a new "kind" field on the agent metadata struct.

  3. **Tool-name remapping table**: when linking a Gemini agent into Claude Code or Codex, `tools: [read_file, grep_search, run_shell_command, write_file, replace]` must become `[Read, Grep, Bash, Write, Edit]` (Claude) or the Codex equivalent. This is a Claudine-internal rewrite step; the linker should not store Gemini tool names verbatim in the destination.

  4. **Update `linking-strategy.md` and `non-portable-assets.md`** with the Gemini-specific non-portable assets (tool name vocabulary, MCP shape, A2A auth blocks, built-in subagent names, `agents.overrides` block) and the corresponding rewrite notes.

  5. **Add Gemini CLI rows to `claudine providers`** so `claudine agents` reports Gemini agent discovery state in the same way it reports Claude Code and Codex.

  A lifecycle `proxy` action has nothing to do here: Gemini CLI does not expose per-subagent resume or session handles. A wrapper that wants to address a specific subagent must use stream-json `tool_use` / `tool_result` events with `name = <agent>` (no `agent_id` analogue) and rely on the parent session's `transcript_path` to replay context.
---

# Gemini CLI Subagents

## Overview

Gemini CLI treats user-defined **subagents** as a first-class feature: durable Markdown files with YAML frontmatter that change who does work inside a session. The provider calls them "subagents" — every `codebase_investigator`, every `generalist`, every `@security-auditor` invocation is built from a subagent definition. Support is `first_class`: there are named scopes (user, project, extension), a documented frontmatter schema, an interactive `/agents` management command, a Policy Engine integration that scopes permissions per subagent, and an Agent2Agent (A2A) protocol extension that lets remote subagents ship as Markdown just like local ones.

This topic is the *definition* of subagents — where files live, what frontmatter they accept, how the parent picks one, what context and permissions the child gets, and how a wrapper can observe start/stop. Hook event semantics (`BeforeTool` / `AfterTool` payload shape, matcher rules) live in the hooks topic; this document records only **which** events expose agent lifecycle. Remote subagents are documented as their own definition flavor (Markdown files with `kind: remote`) because the semantics differ at the wire (A2A protocol over HTTPS) but not at the storage layer. Plugins / extensions are addressed by the plugins topic; bundled `<extension>/agents/*.md` definitions keep their semantics here.

## Locations

Gemini CLI loads subagent files from three on-disk scopes; the runtime merge order (lowest → highest precedence) is: extension-bundled → user → project. The system-wide tier (admin-managed extensions / admin policies) lives in the `Admin` policy tier and the system settings file rather than a dedicated `agents/` directory.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| User | `~/.gemini/agents/*.md` | `~/.gemini/agents/*.md` | `%USERPROFILE%\.gemini\agents\` | Personal subagents. Loaded for every session regardless of working directory. Reload without restart via `/agents reload`. |
| Project | `.gemini/agents/*.md` | `.gemini/agents/*.md` | `.gemini\agents\` | Project-shared subagents, typically checked into the repo. Loaded alongside user agents. Same-`name` collisions resolve to the project file (highest precedence among on-disk scopes). |
| Extension | `<extension>/agents/<name>.md` | `<extension>/agents/<name>.md` | `<extension>\agents\<name>.md` | Bundled subagents distributed via `gemini extensions install`. Enabled when the extension is enabled; flat identifier (no namespace) so a custom local agent with the same `name` collides on precedence rules. |
| Admin (system) | n/a | n/a | n/a | No fixed `/etc/gemini-cli/agents/` documented; admin-managed agent definitions ship via admin-managed extensions or `~/.gemini/policies/*.toml` rules. |
| Settings overrides | `~/.gemini/settings.json`, `.gemini/settings.json`, `/Library/Application Support/GeminiCli/settings.json` | `~/.gemini/settings.json`, `.gemini/settings.json`, `/etc/gemini-cli/settings.json` | `%USERPROFILE%\.gemini\settings.json`, `.gemini\settings.json`, `C:\ProgramData\gemini-cli\settings.json` | `agents.overrides.<name>` block tunes `enabled`, `modelConfig`, and `runConfig` for any agent (built-in or custom). Not a definition file; retunes an existing agent without redeclaring its body. |
| Remote (A2A) | (HTTPS endpoint) | (HTTPS endpoint) | (HTTPS endpoint) | Declared with `kind: remote` in any of the local Markdown files; the local file is the binding record, the A2A card lives at `agent_card_url`. |

On this host (macOS), observed: `~/.gemini/agents/` exists and contains **28 symlinks** pointing at `~/.claude/agents/*.md` (a manually-bridged setup). The symlinked files do not declare `kind: local`, but they parse successfully because every other required field (`name`, `description`) is present and the `kind` field defaults to `local`. A Gemini-aware linker can read these symlinks and extract the same metadata it would extract from a native Gemini agent — but the resulting metadata must be re-categorized as "Claude-originated" for portability analysis, and the tool list (`Read`, `Bash`, `Grep`) needs the Claude → Gemini rewrite (`read_file`, `run_shell_command`, `grep_search`) before it is usable as a native Gemini agent.

## Definition Format

A local subagent file is a Markdown file with YAML frontmatter between `---` markers, where the body becomes the subagent's system prompt. Identity comes from the `name` field, not the filename or directory path. The loader does NOT prepend Gemini CLI's main system prompt — subagents receive only their own body plus a short environment summary (working directory, platform).

```markdown
---
name: security-auditor
description: Specialized in finding security vulnerabilities in code.
kind: local
tools:
  - read_file
  - grep_search
model: gemini-3-flash-preview
temperature: 0.2
max_turns: 10
---

You are a ruthless Security Auditor. Your job is to analyze code for potential
vulnerabilities. Focus on:
1. SQL Injection
2. XSS (Cross-Site Scripting)
3. Hardcoded credentials
4. Unsafe file operations

When you find a vulnerability, explain it clearly and suggest a fix. Do not fix
it yourself; just report it.
```

A remote (Agent2Agent) subagent uses the same shape with `kind: remote` and an `agent_card_url` (or inline `agent_card_json`):

```markdown
---
kind: remote
name: my-remote-agent
agent_card_url: https://example.com/agent-card
auth:
  type: apiKey
  key: $MY_API_KEY
---
```

A single Markdown file may declare **multiple remote** subagents via a YAML list — but mixed local/remote lists and multi-local files are explicitly NOT supported.

Recognized frontmatter fields:

- **Required**: `name` (slug — lowercase letters, numbers, hyphens, underscores; **identity**), `description` (routing signal shown to the main agent).
- **Kind**: `kind` (`local` default | `remote`).
- **Tooling / permissions**: `tools` (allowlist with `*` / `mcp_*` / `mcp_<server>_*` wildcards; omitted = inherit all parent tools), `mcpServers` (inline MCP server definitions isolated to this agent).
- **Model / execution**: `model` (Gemini model id or alias; `inherit` default), `temperature` (0.0–2.0; default 1), `max_turns` (default 30), `timeout_mins` (default 10).
- **Remote only**: `agent_card_url` (A2A card endpoint URL), `agent_card_json` (inline A2A card JSON), `auth` (`{ type, key|token|username|password|value, name?, scheme?, scopes? }` where `type` ∈ `apiKey` | `http` | `google-credentials` | `oauth`).
- **Body**: Markdown — verbatim system prompt, no provider-managed preamble.

Built-in agent names that ship with Gemini CLI (defaults; tunable via `agents.overrides`):

| Name | Default | Purpose |
|---|---|---|
| `codebase_investigator` | enabled | Read-only codebase mapping / dependency analysis |
| `cli_help` | enabled | Gemini CLI itself, its commands, configuration |
| `generalist` | enabled | Generic full-tool subagent for resource-heavy subtasks |
| `browser_agent` | disabled | Chrome via the bundled `chrome-devtools-mcp` |

A custom user/repo agent whose `name` matches a built-in shadows the built-in entirely (replaced, not merged). The loader silently drops malformed files with a warning — invalid slugs, missing required fields, unknown `tools` values, or unsupported `kind` values do not abort the session.

## Runtime Behavior

A subagent is delegated when the main agent's tool registry invokes it as a function call: `codebase_investigator({ prompt: "Map the auth flow" })`. The main agent's system prompt instructs it to call the subagent when the task matches the subagent's `description`; no separate router exists. The `@<agent-name>` syntax at the start of a user prompt skips the router by injecting a system note that nudges the primary model to call that specific subagent tool immediately.

The child receives:

- Its Markdown body as the system prompt (no Gemini-CLI-default preamble).
- A short environment summary (working directory, platform).
- The delegated prompt composed by the parent.
- Only the tools granted by `tools:` (with the `*` / `mcp_*` / `mcp_<server>_*` wildcards), or all of the parent's tools if `tools:` is omitted.
- Inline MCP servers declared in `mcpServers:` — these are scoped to the agent; the parent's MCP servers do NOT auto-inherit.

It does **not** receive the parent's conversation history, the parent's GEMINI.md context, or any sibling subagent tools (subagent-to-subagent recursion is blocked at the loader). Each subagent runs in an independent context loop whose internal turns stay invisible to the parent.

The child's returned state to the parent:

- **Final assistant text** as a normal `tool_result` payload.
- **`error` field** in the tool envelope if the subagent hit `max_turns`, `timeout_mins`, or an internal failure.
- Nothing else — the subagent's intermediate tool calls are inline in the parent's transcript (visible as `tool_use` events for read_file / grep_search etc.) but the subagent's full message stream is not.

The parent continues normally after a subagent returns. A failed subagent does NOT mark the parent session as failed; it surfaces only as a failed `tool_result` for the parent to handle (or fail and re-prompt, depending on the parent's tool error handling).

The Policy Engine governs everything around this delegation:

- `toolName = "<subagent-name>"` rules allow/deny/ask-user for the delegation itself.
- `subagent = "<subagent-name>"` rules scope tool-call permissions to one subagent.
- Approval modes (`default`, `autoEdit`, `yolo`, `plan`) interact via rule `modes = [...]`.

Subagent lifecycle is also visible through three coordinated surfaces:

1. **Stream JSON output** (`--output-format stream-json`): every subagent call is a `tool_use` event with `name = <agent-name>` and `tool_input.prompt` carrying the delegated prompt; the matching `tool_result` event carries the final assistant text and any error envelope. There is no dedicated `subagent_start` / `subagent_stop` event — subagent lifecycle IS the tool call lifecycle.
2. **Hooks**: `BeforeTool` / `AfterTool` hooks match on the subagent name to wrap each call. `tool_input.prompt` carries the parent's delegated prompt; `tool_response.llmContent` carries the subagent's final assistant text; `tool_response.error` carries any failure. Common fields on every hook: `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `timestamp`.
3. **Transcripts**: every session writes a JSONL transcript at the path reported in `session_id` and `transcript_path`. There is no separate per-subagent transcript (no equivalent to Claude Code's `~/.claude/projects/.../subagents/agent-{id}.jsonl`); the subagent's intermediate tool calls are inline in the parent's transcript, indented under the parent tool_use entry.

Subagent state is not independently resumable: `/resume <index-or-tag>` restores the parent's session, and the next prompt will re-delegate to the same subagent from scratch.

## Observability

Subagent starts and stops are visible to wrappers through three coordinated surfaces:

1. **Stream output**: the regular stream-json output (`--output-format stream-json`) records every subagent invocation as a `tool_use` event keyed by the subagent's `name` plus `tool_input.prompt`; the matching `tool_result` event carries the final assistant text and any error envelope. There is no dedicated `subagent_start` / `subagent_stop` event type — subagent lifecycle IS the tool call lifecycle.

2. **Hook events**: `BeforeTool` (when the subagent is called) and `AfterTool` (when the subagent returns) fire as configurable lifecycle events. The matcher targets the subagent's `toolName`. Subagent-internal tool calls (the subagent using `read_file`, `grep_search`, etc.) ALSO fire `BeforeTool` / `AfterTool`, but the hook payload does not currently expose `subagent` as a field — wrappers must correlate via the call site (parent's `AfterTool` for the subagent vs the subagent's own `BeforeTool` for read_file).

   `BeforeAgent` / `AfterAgent` hook events exist but fire on the **main agent loop only**, NOT on subagent invocation. They cannot be scoped to a specific subagent by name. To wrap a specific subagent, use `BeforeTool` / `AfterTool` with a `matcher` on the agent's tool name.

   Common fields on every hook (via `stdin`): `session_id`, `transcript_path`, `cwd`, `hook_event_name`, `timestamp`. `BeforeTool` adds `tool_name` and `tool_input`; `AfterTool` adds `tool_response` (with `llmContent`, `returnDisplay`, and optional `error`).

3. **Transcripts**: the session writes a JSONL transcript at `session_id`/`transcript_path`. Subagent intermediate turns appear inline in the parent's transcript under the parent tool_use entry. There is no separate per-subagent transcript (no equivalent to Claude Code's `~/.claude/projects/.../subagents/agent-{id}.jsonl`); subagent state cannot be independently inspected or resumed.

The `codebase_investigator` built-in is a useful canary: its system prompt limits it to read tools, so when the parent's stream-json shows a `codebase_investigator` `tool_use` followed by a stream of `read_file`/`grep_search` `tool_use` events with a final `tool_result` for `codebase_investigator`, the wrapper has full visibility into the agent's run without parsing the transcript.

## Portability

Gemini CLI subagents are **not portable** across providers as-is. The Markdown body is provider-neutral when it does not reference Gemini-only tools or env vars, but the frontmatter vocabulary does not overlap enough for a verbatim link.

| Field | Portable? | Rewrite target |
|---|---|---|
| `name` | depends | Slug rule differs (Gemini allows underscores; Claude Code disallows them; Codex accepts free strings). |
| `description` | yes | Carries the routing signal across providers. |
| `tools` | no | Gemini tool names (`read_file`, `grep_search`, `run_shell_command`, `write_file`, `replace`, ...) → Claude Code names (`Read`, `Grep`, `Bash`, `Write`, `Edit`) or Codex's equivalents. |
| `tools` wildcards (`*`, `mcp_*`, `mcp_<server>_*`) | no | Claude Code uses `mcp__server` / `mcp__server__*` patterns. Codex uses no wildcards. |
| `mcpServers` | partial | Gemini's `command`/`args`/`cwd` flat shape vs Claude Code's `stdio`/`http`/`sse`/`ws` discriminated union. |
| `model` | partial | Gemini aliases (`flash`, `pro`, `auto`) → Claude aliases (`haiku`, `sonnet`, `opus`) or Codex models. |
| `temperature`, `max_turns`, `timeout_mins` | partial | Claude Code accepts `maxTurns` only; Codex has no direct equivalents (per-session knobs are config-layer, not per-agent). |
| `kind: local \| remote` | no | Gemini-specific discriminator. |
| `agent_card_url`, `agent_card_json`, `auth.*` | no | A2A-protocol fields; meaningless outside A2A-compatible clients. |
| `agents.overrides.<name>` block | no | Gemini-specific tuning knobs. |
| Built-in names (`codebase_investigator`, `cli_help`, `generalist`, `browser_agent`) | no | No equivalents on other providers. |
| Body prompt | partial | Verbatim is fine when it doesn't reference Gemini-only tools or env vars. |

Extension-bundled agents ship with the extension manifest; if the destination provider has its own extension model (e.g. Claude Code plugins), the linker must also translate the manifest packaging — not just the agent file.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy`/`resume` actions, what matters about Gemini CLI subagents:

- Treat `~/.gemini/agents/*.md` and `.gemini/agents/*.md` as the canonical user- and repo-scope subagent locations. Extension-bundled `<extension>/agents/*.md` files register agents with flat identifiers (no namespace); collisions with local agents resolve to the local file (higher precedence).
- Walk the on-disk locations with a YAML frontmatter parser (not TOML, not JSON). Use `name` for identity (not the filename) and capture `kind` to distinguish local vs remote agents.
- A linked Gemini subagent is portable when its body uses only standard Markdown and its frontmatter carries only `name`, `description`, plus an optional `tools:` list and `model` that already target the destination provider's vocabulary. Flag assets that depend on `kind: remote` + `agent_card_url` / `agent_card_json` + `auth.*` (A2A-only), `mcpServers` (provider-specific shape), `temperature` / `max_turns` / `timeout_mins` (partially portable), built-in names (`codebase_investigator`, `cli_help`, `generalist`, `browser_agent`), or `agents.overrides` blocks (Gemini-specific).
- Tool-name remapping table (Gemini → Claude Code): `read_file` → `Read`, `grep_search` → `Grep`, `glob` → `Glob`, `run_shell_command` → `Bash`, `write_file` → `Write`, `replace` → `Edit`. MCP FQN delimiter differs (`mcp_<server>_<tool>` vs `mcp__<server>__<tool>`).
- Model alias remapping: `flash` → `haiku`, `pro` → `sonnet`, `auto` → (no Claude equivalent — drop or pin), `gemini-3-flash-preview` → `claude-haiku-*` or omit for inherit. `inherit` is Gemini's default; Claude Code's default is also `inherit`; Codex accepts no equivalent and falls back to the parent session model.
- For lifecycle `proxy`/`resume`: Gemini CLI does not expose per-subagent resume handles. A wrapper that wants to address a specific subagent must use stream-json `tool_use` / `tool_result` events with `name = <agent>` (no `agent_id` analogue) and rely on the parent session's `transcript_path` to replay context. `/resume <index-or-tag>` restores the parent session; the next prompt will re-delegate to the same subagent from scratch.
- Permission policy: the parent's `--approval-mode` (default / autoEdit / plan / yolo) sets the parent's policy tier but does NOT auto-apply to subagents. To scope a permission to a single subagent, write a Policy Engine TOML rule with `subagent = "<agent-name>"` (or use `toolName = "<agent-name>"` to govern the delegation itself). `policyPaths` / `--policy` / `--admin-policy` load these rules; load them at wrapper preflight so the Policy Engine has them when the parent's first tool call fires.
- Whenever Claudine's wrapper code grows a Gemini-aware `claudine agents` row or a `--subagent <name>` resolution path, the model resolution ladder is: `agents.overrides.<name>.modelConfig.model` → `modelConfigs.overrides[].match.overrideScope == <name>` → frontmatter `model` → `inherit` (parent session model).
- Default observability: `tool_use` (start) and `tool_result` (end) in stream-json output; `BeforeTool` / `AfterTool` hooks with a matcher on the subagent's tool name for wrappers that prefer hooks over stream parsing. The wrapper does NOT need to parse a separate per-subagent transcript because none exists — the subagent's intermediate turns are inline in the parent's transcript under the parent `tool_use` entry.

## Sources

- [Gemini CLI — Subagents (canonical)](https://geminicli.com/docs/core/subagents/)
- [Gemini CLI — Remote subagents (Agent2Agent)](https://geminicli.com/docs/core/remote-agents/)
- [Gemini CLI — Configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI — Hooks reference](https://geminicli.com/docs/hooks/reference/)
- [Gemini CLI — Policy engine](https://geminicli.com/docs/reference/policy-engine/)
- [Gemini CLI — Command reference (incl. `/agents`)](https://geminicli.com/docs/reference/commands/)
- [Gemini CLI — Extensions overview](https://geminicli.com/docs/extensions/)
- [Gemini CLI — Build extensions](https://geminicli.com/docs/extensions/writing-extensions/)
- [Gemini CLI — Headless mode (stream-json output)](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI — homepage](https://geminicli.com/)
- [Gemini CLI — GitHub repository](https://github.com/google-gemini/gemini-cli)
- [Agent2Agent (A2A) protocol specification](https://a2a-protocol.org/latest/specification/)