---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://github.com/openai/codex
docs: https://developers.openai.com/codex/cli
subagent_docs: https://developers.openai.com/codex/subagents

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.codex/agents/
    notes: "Personal custom-agent definitions. Each *.toml file in this directory is loaded as a config layer for the named agent. Resolved relative to $CODEX_HOME (default ~/.codex)."
  - os: linux
    scope: user
    path: ~/.codex/agents/
    notes: "Personal custom-agent definitions. Same as macOS; resolved via $CODEX_HOME."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\agents\\"
    notes: "Personal custom-agent definitions. Resolved relative to $CODEX_HOME which defaults to %USERPROFILE%\\.codex on Windows."
  - os: macos
    scope: repo
    path: .codex/agents/
    notes: "Project-scoped custom agents. Codex walks up from the working directory to the project root (default marker: .git) and loads every .codex/agents/*.toml file it finds. Loads only when the project is trusted; otherwise the project .codex/ layer is ignored."
  - os: linux
    scope: repo
    path: .codex/agents/
    notes: "Same as macOS. Project trust gates load; user-level agents from ~/.codex/agents/ remain available even in untrusted projects."
  - os: windows
    scope: repo
    path: ".codex\\agents\\"
    notes: "Same as macOS / Linux on native Windows. On WSL the project .codex/agents/ is read from the WSL-side path."
  - os: macos
    scope: system
    path: "<managed config layer>/agents/"
    notes: "No fixed system agents/ directory documented; managed subagents ship via the enterprise managed-configuration layer (see Codex Managed configuration docs) rather than a fixed /etc or /Library path."
  - os: linux
    scope: system
    path: "<managed config layer>/agents/"
    notes: "Same as macOS. Enterprise admins ship agent definitions through managed config layers (not a documented filesystem path)."
  - os: windows
    scope: system
    path: "<managed config layer>\\agents\\"
    notes: "Same as macOS / Linux. No fixed managed agents/ directory is documented; managed layers are distributed through MDM / requirements.toml."
  - os: macos
    scope: extension
    path: "<plugin>/agents/<name>.toml"
    notes: "Plugins can bundle custom agents via their .codex-plugin/plugin.json manifest (see Codex Plugins Build docs). Codex auto-loads enabled plugins at session start; bundled agents become available under their `name`."
  - os: linux
    scope: extension
    path: "<plugin>/agents/<name>.toml"
    notes: "Same as macOS."
  - os: windows
    scope: extension
    path: "<plugin>\\agents\\<name>.toml"
    notes: "Same as macOS / Linux. Plugin manifest hooks/lifecycle rules apply; .codex-plugin/plugin.json is the entry point."
  - os: macos
    scope: other
    path: "inline config-file path via [agents.<name>.config_file]"
    notes: "An existing custom-agent file can be referenced by an absolute or config-relative path via the agents.<name>.config_file key in config.toml. This is how a custom-agent definition can be loaded from a managed or out-of-tree location without copying it into ~/.codex/agents/ or .codex/agents/."
  - os: linux
    scope: other
    path: "inline config-file path via [agents.<name>.config_file]"
    notes: "Same as macOS."
  - os: windows
    scope: other
    path: "inline config-file path via [agents.<name>.config_file]"
    notes: "Same as macOS / Linux."

format:
  file_names:
    - "*.toml"
  frontmatter: false
  required_fields:
    - name (string; identity carrier; matches by value, not filename)
    - description (string; routing signal shown to Codex when picking the agent)
    - developer_instructions (string; the core instructions defining the agent's behavior)
  optional_fields:
    - "nickname_candidates (string[]; presentation-only display pool)"
    - "model (alias or full model ID, e.g. gpt-5.5, gpt-5.4, gpt-5.4-mini, gpt-5.3-codex-spark)"
    - "model_reasoning_effort (minimal | low | medium | high | xhigh)"
    - "sandbox_mode (read-only | workspace-write | danger-full-access)"
    - "approval_policy (untrusted | on-request | never | granular table)"
    - "mcp_servers.<id>.* (inline MCP server definitions; see Codex MCP docs)"
    - "[[skills.config]] (path, enabled) — skill enable/disable list for this agent"
    - "approval_policy.granular.* (sandbox_approval, rules, mcp_elicitations, request_permissions, skill_approval)"
    - "approvals_reviewer (user | auto_review)"
    - "web_search (cached | live | disabled)"
    - "personality (pragmatic | friendly | none)"
    - "developer_instructions append via inline `developer_instructions`"
    - "Any other supported config.toml key (custom-agent files load as configuration layers, so they can override most session settings)"
  body_format: toml
  notes: |
    A custom-agent file is a single TOML document. The file extension is *.toml; frontmatter is not parsed. Identity is carried by the `name` field, not the filename; matching the filename to the agent name is the simplest convention but the docs explicitly note that `name` is the source of truth.

    A custom-agent file can include **any** recognized config.toml key (model, sandbox_mode, approval_policy, mcp_servers, skills.config, web_search, personality, developer_instructions, and so on). Codex loads the file as a configuration layer for the spawned session, so custom agents can override the same settings as a normal Codex session config. Optional fields inherit from the parent session when omitted.

    Built-in precedence: if a custom-agent name matches a built-in agent such as `explorer`, the custom agent takes precedence over the built-in.

    Example custom-agent definition from the official docs:

    ```toml
    # ~/.codex/agents/reviewer.toml
    name = "reviewer"
    description = "PR reviewer focused on correctness, security, and missing tests."
    model = "gpt-5.4"
    model_reasoning_effort = "high"
    sandbox_mode = "read-only"
    developer_instructions = """
    Review code like an owner.
    Prioritize correctness, security, behavior regressions, and missing test coverage.
    Lead with concrete findings, include reproduction steps when possible, and avoid style-only comments unless they hide a real bug.
    """
    nickname_candidates = ["Atlas", "Delta", "Echo"]
    ```

    Example using MCP servers (also from the official docs):

    ```toml
    # .codex/agents/docs-researcher.toml
    name = "docs_researcher"
    description = "Documentation specialist that uses the docs MCP server to verify APIs and framework behavior."
    model = "gpt-5.4-mini"
    model_reasoning_effort = "medium"
    sandbox_mode = "read-only"
    developer_instructions = """
    Use the docs MCP server to confirm APIs, options, and version-specific behavior.
    Return concise answers with links or exact references when available.
    Do not make code changes.
    """

    [mcp_servers.openaiDeveloperDocs]
    url = "https://developers.openai.com/mcp"
    ```

runtime:
  invocation: |
    Subagents are **never** spawned automatically. Codex only spawns a new agent when the user explicitly asks for parallel or delegated work, e.g. "spawn one agent per point" or "delegate this work in parallel". The CLI exposes a `/agent` slash command for switching between active agent threads and inspecting the ongoing thread.

    Three invocation surfaces:
    (1) **Natural-language instruction** in the prompt — "Spawn one subagent for security risks, one for test gaps, and one for maintainability. Wait for all three, then summarize the findings by category with file references." The main agent orchestrates spawning, routing, waiting, and closing the agent threads.
    (2) **Multi-agent tool family** exposed to the main agent when `features.multi_agent = true`: `spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, and `close_agent`. These tools are how the parent actually creates a subagent thread.
    (3) **`spawn_agents_on_csv` (experimental CSV fan-out)** — a single tool call that reads a CSV, spawns one worker per row, waits for the full batch to finish, and exports the combined results to a CSV. Each worker must call `report_agent_job_result` exactly once; a worker that exits without reporting a result is marked with an error in the exported CSV.

    There is no CLI flag or `--agent <name>` equivalent that launches a Codex session directly as a custom agent. The session's main thread is always the default Codex agent; custom agents only come into play once the parent has been asked to delegate.

    Concurrency: the parent can keep multiple subagent threads open in parallel. `agents.max_threads` (default 6) caps how many agent threads stay open concurrently.

  parent_child_context: |
    Each subagent is its own Codex session: a fresh context that receives its `developer_instructions` as its system-prompt core, plus the parent-supplied task prompt and any inherited config. Subagents do not see the parent's conversation history; their context is built from the custom-agent file (the config-layer override) plus what the parent passes in.

    The parent waits for results across parallel subagents, then returns a consolidated response. When many agents are running, Codex waits until all requested results are available. Codex handles orchestration end-to-end: spawning new subagents, routing follow-up instructions, waiting for results, and closing agent threads.

    A subagent can call `send_input` to receive follow-up direction from the parent while it is running. The parent can ask Codex directly to steer a running subagent, stop it, or close completed agent threads.

  permissions_inheritance: |
    Subagents inherit the parent's current sandbox policy and the parent's live runtime overrides. The Codex docs explicitly call out: "Codex reapplies the parent turn's live runtime overrides when it spawns a child. That includes sandbox and approval choices you set interactively during the session, such as `/permissions` changes or `--yolo`, even if the selected custom agent file sets different defaults."

    Within a custom-agent file, `sandbox_mode` and `approval_policy` can **narrow** or **widen** the parent's setting (e.g. an exploration agent can pin `sandbox_mode = "read-only"` even when the parent is in `workspace-write`). A custom agent cannot override a live `/permissions` change — the live override wins.

    Approval-routing rule: approval requests can surface from inactive agent threads while the user is looking at the main thread. The approval overlay shows the source thread label, and `o` opens that thread before approving, rejecting, or answering. In non-interactive flows (or whenever a run cannot surface a fresh approval), an action that needs new approval fails and the error is surfaced back to the parent workflow.

    `approvals_reviewer` can be `user` or `auto_review` (the auto-review subagent routes eligible interactive approvals automatically); the per-agent value inherits from the parent if omitted.

  model_inheritance: |
    Resolution order, top wins:
    1. `--model <id>` on the parent CLI invocation (or `model = ...` in the parent's config layer).
    2. The custom-agent file's `model` field (alias or full ID; examples in the docs include `gpt-5.5`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.3-codex-spark`).
    3. The parent's session model (when the custom-agent file omits `model`).

    When the custom-agent file omits `model_reasoning_effort`, the parent's reasoning setting is used. The docs note that if a model and reasoning effort are not pinned, Codex can choose a setup that balances intelligence, speed, and price for the task.

  tool_inheritance: |
    Default: the subagent inherits the parent's tool surface minus any tools restricted by the custom-agent file's config layer. Because custom-agent files load as configuration layers, the same `approval_policy`, sandbox, and feature overrides that work in `config.toml` also work in a custom-agent file.

    MCP servers: a custom agent can attach its own MCP servers inline (`[mcp_servers.<id>]`). The parent agent's MCP servers are not automatically inherited; the custom-agent file must redeclare any it wants. Server definitions match the standard `mcp_servers.<id>` schema (command/args for stdio, url for HTTP, plus startup_timeout_sec, env_http_headers, http_headers, env, etc.).

    Skills: a custom agent can enable or disable skills via `[[skills.config]]` blocks. Path resolution is the same as in `config.toml`. A custom agent can disable skills the parent has enabled, but skills are not "preloaded" the way they are in some other providers — they are activated as the agent runs.

    Per-tool overrides: approval_mode, enabled_tools, disabled_tools, and `default_tools_approval_mode` work the same way in a custom-agent file as in the parent config.

  max_turns: |
    No documented per-agent `max_turns` field. The relevant caps are:
    - `agents.max_threads` (number): concurrent open agent thread cap, default 6.
    - `agents.max_depth` (number): spawned agent nesting depth (root session starts at depth 0), default 1 — a direct child agent can spawn but deeper nesting is blocked. Raising this allows recursive delegation; the docs warn that recursive delegation can turn broad instructions into repeated fan-out and increases token usage, latency, and local resource consumption. `agents.max_threads` still caps concurrent open threads even at deeper recursion.
    - `agents.job_max_runtime_seconds` (number): default per-worker timeout for `spawn_agents_on_csv` jobs; when unset, the tool falls back to 1800 seconds per worker. A per-call `max_runtime_seconds` override on `spawn_agents_on_csv` takes precedence.

    Hooks carry a per-handler `timeout` (seconds, default 600); hook timeouts do not bound agent turns but bound individual hook runs.

  notes: |
    Selection: there is no automatic router that picks a subagent — Codex only delegates when the user explicitly asks for it. The custom-agent file's `description` is what Codex reads when deciding which agent fits a request, so good descriptions matter for routing.

    Built-in agents Codex ships: `default` (general-purpose fallback), `worker` (execution-focused, implementation/fixes), `explorer` (read-heavy codebase exploration). Custom agent names that collide with a built-in name take precedence over the built-in.

    Concurrency: multiple subagents run in parallel. Default cap is 6 concurrent open threads (`agents.max_threads`). The fan-out tool `spawn_agents_on_csv` adds `max_concurrency` per call as an additional cap.

    Nesting: `agents.max_depth` defaults to 1 (root = depth 0). Default prevents deeper recursion; raising it enables recursive delegation.

    Failure: a worker that exits without calling `report_agent_job_result` is marked with an error in the exported CSV. Subagent threads can be closed by the parent or steered via `send_input`.

    Switching: the `/agent` slash command lets the user switch between active agent threads and inspect the ongoing thread.

    Plugin-bundled agents ship via the plugin manifest; the `agents.<name>.config_file` key can also reference an existing TOML file by absolute or config-relative path, which is how an external or managed agent definition can be loaded without copying it into `~/.codex/agents/` or `.codex/agents/`.

observability:
  stream_events:
    - "thread.started (codex exec --json, end-of-turn framing event for the session)"
    - "turn.started (codex exec --json, marks the start of a turn)"
    - "turn.completed (codex exec --json, marks the end of a turn)"
    - "item.* (codex exec --json, per-item events for assistant text, reasoning, tool calls, etc.)"
    - "SubagentStart hook input (lifecycle event; fires when a subagent is spawned; delivered to command hooks over stdin as JSON)"
    - "SubagentStop hook input (lifecycle event; fires when a subagent finishes; delivered to command hooks over stdin as JSON; the stop hook can return continue: false to stop the agent — though the docs note continue: false is parsed for compatibility but does not stop the subagent)"
    - "agent_id (input field on SubagentStart and SubagentStop hooks; Codex-specific extension naming a stable subagent identifier)"
    - "agent_type (input field on SubagentStart and SubagentStop hooks; Codex-specific extension equal to the agent's `name`, or to the parent's chosen nickname)"
    - "agent_transcript_path (SubagentStop only; path to the subagent's own transcript file when one exists)"
    - "turn_id (Codex-specific extension on the SubagentStart/SubagentStop hook input)"
    - "permission_mode (Codex-specific extension on SubagentStart/SubagentStop hook input: default | acceptEdits | plan | dontAsk | bypassPermissions)"
    - "session_id (Codex session id, present on every hook; SubagentStart/SubagentStop use the parent session id)"
    - "transcript_path (path to the parent session transcript on every hook input)"
    - "model (Codex-specific extension on every hook input: active model slug)"
    - "stop_hook_active (boolean on SubagentStop: whether the subagent was already continued)"
  hook_events:
    - "SubagentStart (matcher is applied to agent_type — built-in names like default/worker/explorer, custom-agent name values, or nicknames)"
    - "SubagentStop (matcher is applied to agent_type; same value space as SubagentStart)"
    - "PreToolUse / PostToolUse / PermissionRequest (fired inside the parent session, scoped by tool_name; the SubagentStart/SubagentStop pair covers the subagent-boundary cases)"
  session_ids: true
  notes: |
    Subagent starts and stops are surfaced through the SubagentStart and SubagentStop hook events. The matcher targets `agent_type`, which carries the custom-agent `name` (or the parent's chosen display nickname when nicknames are used). Each subagent has a stable `agent_id` that is delivered on both events.

    On SubagentStop the hook receives `agent_transcript_path`, the path to the subagent's own JSONL transcript; the parent transcript's `transcript_path` is also delivered so a wrapper can correlate the two. Transcript retention follows the parent session's history policy (`history.persistence` and the SQLite-backed state under `$CODEX_SQLITE_HOME` / `sqlite_home`).

    Non-interactive (`codex exec --json`) emits the standard `thread.started`, `turn.started`, `turn.completed`, `item.*` event stream but does **not** itself emit dedicated subagent lifecycle events. Wrappers that need subagent start/stop in `codex exec` should listen for SubagentStart/SubagentStop hook output (when hooks are configured) or read `agent_id` / `agent_type` from the regular item.* stream.

    Hook command output: SubagentStop accepts the same output shape as Stop (continue, stopReason, systemMessage, suppressOutput). `continue: false` on SubagentStop is parsed for compatibility but does not stop the subagent from continuing.

    SQLite-backed state: `sqlite_home` (or `$CODEX_SQLITE_HOME`) controls where Codex stores the SQLite-backed state used for `spawn_agents_on_csv` jobs and their exported results. OTel exporters can stream subagent run events when telemetry is enabled.

portability:
  portable: false
  non_portable_assets:
    - "`name` (string identity field; portable as a concept but the exact value is consumed by Codex's spawn_agent / send_input / close_agent tool family and cannot be directly imported into another provider)"
    - "`description` (routing signal; portable as a concept — Claude Code's `description` and Codex's `description` carry similar semantics but the surrounding router reads each provider's vocabulary)"
    - "`developer_instructions` (provider-agnostic prose; verbatim reuse is fine, but reference to Codex-specific surface like `spawn_agent`, `send_input`, `report_agent_job_result`, or the Codex tool family has no equivalent in Claude Code)"
    - "TOML format (every Codex definition is TOML; Claude Code subagents are Markdown+YAML frontmatter — automatic migration requires a TOML→frontmatter parser)"
    - "Config-layer overrides (`sandbox_mode`, `approval_policy`, `mcp_servers`, `[[skills.config]]`, `web_search`, `personality`, etc.) — these are full `config.toml` keys and have no equivalent in the Claude Code subagent frontmatter shape; on the Claude Code side they would map to `permissionMode`, `mcpServers`, `skills`, and tool allowlists, plus model/effort vocabulary"
    - "`nickname_candidates` (presentation-only display pool; Codex-specific UX)"
    - "`agents.max_threads`, `agents.max_depth`, `agents.job_max_runtime_seconds` (global caps under `[agents]`; Claude Code has no documented concurrency / depth ceiling in the same place)"
    - "`features.multi_agent` flag (stable, on by default; Claude Code subagents are always on)"
    - "`spawn_agents_on_csv` fan-out tool and `report_agent_job_result` worker call (experimental Codex-only workflow)"
    - "`/agent` slash command (Claude Code has no equivalent session-thread switcher)"
    - "`agents.<name>.config_file` indirection (lets a Codex custom-agent definition be loaded from an external path; no equivalent in Claude Code)"
    - "Hook events `SubagentStart` / `SubagentStop` (Claude Code exposes the same hooks but with a different payload contract — agent_type is a free-form string for both providers, but field shape and output capabilities differ)"
    - "`mcp_servers.<id>` block (Codex's MCP shape is similar but not identical to Claude Code's `mcpServers` block; OAuth, headers, and transport names may diverge)"
  rewrite_needed: true
  notes: |
    The body of `developer_instructions` is plain prose and can usually be lifted verbatim into Claude Code's `prompt` field or frontmatter body. The surrounding schema, however, is provider-specific:

    - **Codex → Claude Code**: rewrite the TOML into a Markdown file with YAML frontmatter. `name` becomes the `name` frontmatter field (lowercase letters and hyphens); `description` maps to `description`; `developer_instructions` becomes the body; `model` maps to `model` (and Codex's aliases like `gpt-5.4-mini` must be remapped to Claude aliases `sonnet` / `opus` / `haiku` or a full Claude model ID); `model_reasoning_effort` has no direct Claude Code equivalent (drop or rewrite as part of the body prompt); `sandbox_mode` maps to Claude Code's `permissionMode` (`read-only` → no edits, `workspace-write` → `default`, `danger-full-access` → `bypassPermissions`); `mcp_servers` becomes Claude Code's `mcpServers` (verify OAuth and header shape); `[[skills.config]]` becomes Claude Code's `skills` array of skill names; `nickname_candidates` has no Claude equivalent and should be dropped.

    - **Claude Code → Codex**: rewrite the Markdown+YAML frontmatter into a TOML file. The body becomes the inline `developer_instructions = """ ... """` triple-quoted string. `name` becomes `name`. `description` becomes `description`. Claude Code's `model` aliases (`sonnet`/`opus`/`haiku`/`fable`) do not work in Codex — replace with a full Codex model ID. `tools` (an allowlist) has no Codex equivalent — drop the field and rely on Codex's default tool surface plus the custom-agent file's `sandbox_mode` / `approval_policy` to narrow. `permissionMode` maps to Codex's `sandbox_mode` + `approval_policy` pair. `mcpServers` becomes `mcp_servers` (verify shape). `hooks`, `memory`, `isolation: worktree`, `effort`, `background`, `color`, `initialPrompt` are Claude Code-only and must be dropped.

    - **Plugin-packaged custom agents** survive the round trip only if the consumer understands the Codex plugin manifest. A Codex plugin's `.codex-plugin/plugin.json` does not document a public agents/ directory; agent definitions bundled with a plugin are loaded by their manifest entry (the `agents.<name>.config_file` indirection, when present, can point at any TOML file). On the Claude Code side, plugin agents register as `plugin-name:agent` and are scoped differently. Cross-provider translation of a Codex-plugin-bundled agent is therefore not lossless.

    - **CSV fan-out workflows** (`spawn_agents_on_csv`, `report_agent_job_result`) are Codex-specific and have no portable equivalent.

cli_params:
  - flag: -c, --config <key=value>
    description: "Override a single configuration value for one run (dotted-path notation; value parsed as TOML). Useful for one-off overrides of features.multi_agent, sandbox_mode, or individual agent settings via -c agents.max_threads=12."
    example: "codex -c features.multi_agent=true -c agents.max_depth=2"
  - flag: --enable <FEATURE>
    description: "Enable a feature flag for one run. Equivalent to -c features.<name>=true. The relevant flag for subagents is `multi_agent`."
    example: "codex --enable multi_agent"
  - flag: --disable <FEATURE>
    description: "Disable a feature flag for one run. Use to turn the multi_agent tool family off entirely (subagent definitions still load but the parent cannot spawn them)."
    example: "codex --disable multi_agent"
  - flag: -m, --model <MODEL>
    description: "Pick the parent session model. Subagents without their own `model` field inherit this value; subagents with a custom-agent file override use that file's value."
    example: "codex -m gpt-5.5"
  - flag: -p, --profile <CONFIG_PROFILE_V2>
    description: "Layer $CODEX_HOME/<name>.config.toml on top of the base user config. The profile file is a regular config.toml layer — it can include its own [agents] section or `agents.<name>.config_file` indirection to load a specific custom-agent definition."
    example: "codex --profile deep-review"
  - flag: --strict-config
    description: "Error out when config.toml contains fields that are not recognized by this version of Codex. Useful when validating a custom-agent file's TOML layer against an unknown schema version."
    example: "codex --strict-config"
  - flag: -s, --sandbox <SANDBOX_MODE>
    description: "Set the parent session's sandbox policy. Values: read-only, workspace-write, danger-full-access. Subagents inherit this; live overrides via /permissions or --yolo are reapplied to children."
    example: "codex -s workspace-write"
  - flag: --dangerously-bypass-approvals-and-sandbox
    description: "Skip all confirmation prompts and execute commands without sandboxing. EXTREMELY DANGEROUS. Propagates to subagents because the live runtime override is reapplied on child spawn."
    example: "codex --dangerously-bypass-approvals-and-sandbox"
  - flag: --dangerously-bypass-hook-trust
    description: "Run enabled hooks (including SubagentStart / SubagentStop) without requiring persisted hook trust for one invocation. DANGEROUS. Intended only for automation that already vets hook sources."
    example: "codex --dangerously-bypass-hook-trust"
  - flag: -a, --ask-for-approval <APPROVAL_POLICY>
    description: "Parent session approval policy (untrusted | on-request | never | granular table). Inherited by subagents unless the custom-agent file overrides."
    example: "codex -a on-request"
  - flag: -C, --cd <DIR>
    description: "Tell the agent to use the specified directory as its working root. Codex walks from this directory upward to find the project root and load .codex/agents/."
    example: "codex -C ./services/auth"
  - flag: --add-dir <DIR>
    description: "Additional directories that should be writable alongside the primary workspace. Inherited by subagents spawned from the session."
    example: "codex --add-dir ../shared"
  - flag: codex plugin <add|list|marketplace|remove>
    description: "Manage Codex plugins. Plugins can bundle custom agents via their .codex-plugin/plugin.json manifest."
    example: "codex plugin list"
  - flag: codex exec --json
    description: "Run Codex non-interactively with newline-delimited JSON events (thread.started, turn.started, turn.completed, item.*). Subagent lifecycle is not in this stream directly; SubagentStart/SubagentStop arrive via hook output when hooks are configured."
    example: "codex exec --json \"summarize the repo\""
  - flag: codex features <list|enable|disable>
    description: "Inspect or toggle feature flags at the user config level. multi_agent, hooks, plugins are all gated by [features] flags."
    example: "codex features list"
  - flag: codex doctor
    description: "Diagnose local Codex installation, config, auth, and runtime health. Useful for confirming that custom-agent files loaded cleanly."
    example: "codex doctor"
  - flag: /agent (in-session slash command)
    description: "Switch between active agent threads and inspect the ongoing thread. Available in the interactive CLI and TUI."
    example: "/agent"
  - flag: /hooks (in-session slash command)
    description: "Inspect hook sources, review new or changed hooks, trust hooks, or disable individual non-managed hooks. Hook sources include SubagentStart and SubagentStop."
    example: "/hooks"

env_vars:
  - name: CODEX_HOME
    effect: "Root for Codex state (config, auth, logs, sessions, skills, plugins, agents). Defaults to ~/.codex on macOS/Linux and %USERPROFILE%\\.codex on Windows. Setting it to a new path moves ~/.codex/agents/, .codex/agents/ discovery, and the entire config tree."
  - name: CODEX_SQLITE_HOME
    effect: "Where SQLite-backed state is stored. Used for spawn_agents_on_csv job state, exported CSV results, and agent thread bookkeeping. The sqlite_home config key takes precedence. Defaults to $CODEX_HOME."
  - name: CODEX_NON_INTERACTIVE
    effect: "Installer behavior flag. Skip installer prompts for unattended installs/upgrades. Does not affect subagent behavior at runtime."
  - name: CODEX_API_KEY
    effect: "Provides an API key for a single non-interactive run. Used by `codex exec` only. Subagents spawned from a `codex exec` parent inherit auth via the parent session."
  - name: CODEX_ACCESS_TOKEN
    effect: "ChatGPT or Codex access token for trusted automation. For persisted login, pipe to `codex login --with-access-token`."
  - name: CODEX_CA_CERTIFICATE
    effect: "PEM CA bundle for HTTPS / WebSocket clients when corporate TLS interception or private root CAs are in play. Precedence over SSL_CERT_FILE."
  - name: RUST_LOG
    effect: "Controls Rust log filtering and verbosity. `codex exec` defaults to error output unless a more verbose value is set. Useful for surfacing SubagentStart/SubagentStop hook events when wrapping Codex from a wrapper."
  - name: OPENAI_BASE_URL (provider config)
    effect: "Not a Codex env var directly; it is the env var named by `[model_providers.<id>].env_key` for the openai provider's API key. Changing the base URL is done via the `openai_base_url` config key, not via env var."

changes: []

requires_claudine_update: true
reason: |
  Claudine's `linking` module currently recognizes Codex skills, slash commands, hooks, MCP servers, and AGENTS.md, but no entry covers Codex's user-defined **custom-agent definitions** (`~/.codex/agents/*.toml`, `.codex/agents/*.toml`, plugin-bundled agents, and `agents.<name>.config_file` indirection). The agent-listing feature (`claudine agents`) needs a sibling that enumerates Codex `*.toml` agent files from both scopes and surfaces their `name`, `description`, `model`, and `sandbox_mode`. Lifecycle `proxy` / `resume` actions will also need to know the Codex-specific SubagentStart / SubagentStop payload shape — `agent_id`, `agent_type`, `agent_transcript_path` (SubagentStop only), `permission_mode`, `session_id`, `turn_id`, `transcript_path`, `model`, and `stop_hook_active` — and the SQLite-backed `agent_transcript_path` is the source of truth for subagent resume. Concurrency / depth ceiling awareness (`agents.max_threads`, `agents.max_depth`) is needed for the `proxy` action to avoid fanning out past the parent's cap.
---

# Codex CLI Subagents

## Overview

Codex treats user-defined **custom agents** as a first-class feature: durable TOML files with a strict three-field minimum (`name`, `description`, `developer_instructions`) that override the same `config.toml` keys as a normal Codex session. Support is `first_class`: there are named scopes (managed/system, personal, project, plugin, and inline indirection through `agents.<name>.config_file`), a documented field schema, runtime delegation semantics through a multi-agent tool family (`spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, `close_agent`), built-in agents (`default`, `worker`, `explorer`) that custom names can override, an experimental CSV fan-out workflow (`spawn_agents_on_csv` + `report_agent_job_result`), and a fresh `agent_id` / `agent_type` lifecycle that surfaces in `SubagentStart` / `SubagentStop` hook events.

This topic's scope is the **definition** of custom agents — where files live, what fields they accept, how the parent picks one, what context and permissions the child gets, and how a wrapper can observe start/stop. Hook event semantics (the full SubagentStart / SubagentStop payload shape, matcher rules, exit-code behavior) live in the hooks topic; this document records only **which** events expose agent lifecycle and what fields a wrapper can rely on for resume and proxy. The plugins topic owns plugin packaging rules; an agent definition bundled in a plugin keeps its semantics here and its packaging there.

On the host (macOS, Codex CLI 0.142.5) the local `~/.codex/agents/` contains symlinks to `~/.claude/agents/*.md` files rather than native Codex `*.toml` definitions. The host's `[features].multi_agent = true` is in `~/.codex/config.toml` and the `multi_agent` feature flag is reported `stable` / `true` by `codex features list`.

## Locations

Codex loads custom-agent files from six surfaces; the order below is the precedence order with the highest-priority source on top.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| User | `~/.codex/agents/` | `~/.codex/agents/` | `%USERPROFILE%\.codex\agents\` | Personal custom-agent definitions. Each `*.toml` file is loaded as a config layer for the named agent. Resolved via `$CODEX_HOME` (defaults above). |
| Project | `.codex/agents/` (recursive from cwd to project root) | same | `.codex\agents\` | Walked upward from the working directory; `.git` is the default project root marker (overridable via `project_root_markers`). Project `.codex/` layer loads only when the project is **trusted**; user-level definitions remain available in untrusted projects. |
| Managed / system | (managed config layer) | (managed config layer) | (managed config layer) | No fixed `/etc` or `/Library` path is documented; admins ship agent definitions through the Codex managed-configuration layer (MDM / `requirements.toml`) rather than a fixed system path. |
| Extension / plugin | `<plugin>/agents/<name>.toml` (loaded via the plugin manifest) | same | `<plugin>\agents\<name>.toml` | Plugins bundle custom agents via `.codex-plugin/plugin.json`. Plugin-bundled agents become available by `name` when the plugin is enabled. |
| Other | inline `agents.<name>.config_file = "..."` in any `config.toml` | same | same | An existing TOML can be referenced by absolute or config-relative path. This is how a managed or out-of-tree definition is loaded without copying it into `~/.codex/agents/` or `.codex/agents/`. |
| Global settings | inline `[agents]` table in any `config.toml` | same | same | `agents.max_threads`, `agents.max_depth`, `agents.job_max_runtime_seconds`, plus per-role `agents.<name>.{description, nickname_candidates, config_file}`. |

Project `.codex/config.toml` cannot override auth / profile / telemetry keys; the same restriction applies to project `.codex/agents/*.toml` files because they are loaded as config layers.

On this host (macOS), observed: `~/.codex/agents/` is a directory but its contents are symlinks to `~/.claude/agents/*.md` Markdown files rather than native Codex `*.toml` definitions. `codex features list` reports `multi_agent` as `stable` and `true`; the local `~/.codex/config.toml` sets `[features].multi_agent = true` explicitly. The Codex docs do not auto-parse Claude Code's Markdown subagent shape into a Codex `*.toml` agent file — symlinks are a host-side bridging decision, not a Codex feature.

## Definition Format

A custom-agent file is a single TOML document. Identity comes from the `name` field, not the filename; matching the filename to the agent name is the simplest convention but the docs explicitly note that `name` is the source of truth.

```toml
# ~/.codex/agents/reviewer.toml
name = "reviewer"
description = "PR reviewer focused on correctness, security, and missing tests."
model = "gpt-5.4"
model_reasoning_effort = "high"
sandbox_mode = "read-only"
developer_instructions = """
Review code like an owner.
Prioritize correctness, security, behavior regressions, and missing test coverage.
Lead with concrete findings, include reproduction steps when possible, and avoid style-only comments unless they hide a real bug.
"""
nickname_candidates = ["Atlas", "Delta", "Echo"]
```

A custom-agent file can include **any** recognized `config.toml` key (model, `model_reasoning_effort`, `sandbox_mode`, `approval_policy`, `mcp_servers`, `[[skills.config]]`, `web_search`, `personality`, `developer_instructions`, granular approval toggles, `approvals_reviewer`, and so on). The same shape is also available via the `agents.<name>.config_file` indirection:

```toml
# in any config.toml layer
[agents.reviewer]
description = "PR reviewer focused on correctness, security, and missing tests."
nickname_candidates = ["Atlas", "Delta", "Echo"]
config_file = "/etc/codex/agents/reviewer.toml"
```

Recognized fields for a standalone custom-agent file (or for `agents.<name>` in `config.toml`):

- **Required**: `name` (string; identity carrier; matches by value, not filename), `description` (string; routing signal shown to Codex when picking the agent), `developer_instructions` (string; the core instructions defining the agent's behavior).
- **Presentation**: `nickname_candidates` (string[]; non-empty list of unique names; each nickname may use ASCII letters, digits, spaces, hyphens, and underscores; presentation-only — identity is still `name`).
- **Model**: `model` (alias or full model ID; e.g. `gpt-5.5`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.3-codex-spark`); `model_reasoning_effort` (`minimal` | `low` | `medium` | `high` | `xhigh`).
- **Sandbox / approval**: `sandbox_mode` (`read-only` | `workspace-write` | `danger-full-access`); `approval_policy` (`untrusted` | `on-request` | `never` | granular table); `approval_policy.granular.{sandbox_approval, rules, mcp_elicitations, request_permissions, skill_approval}`; `approvals_reviewer` (`user` | `auto_review`).
- **Tools / integrations**: `[mcp_servers.<id>]` blocks (full MCP server shape: command/args for stdio, url for HTTP, plus `startup_timeout_sec`, `env_http_headers`, `http_headers`, `env`, `oauth_resource`, `scopes`, etc.); `[[skills.config]]` (`path`, `enabled`) for skill enable/disable lists.
- **Surface**: `web_search` (`cached` | `live` | `disabled`); `personality` (`pragmatic` | `friendly` | `none`); `developer_instructions` (inline override).
- **Session knobs**: any other supported `config.toml` key — custom-agent files load as configuration layers.

Built-in precedence: a custom agent name that matches a built-in agent (`default`, `worker`, `explorer`) takes precedence over the built-in.

## Runtime Behavior

Subagents are delegated through Codex's multi-agent tool family. The parent model issues tool calls (`spawn_agent`, `send_input`, `resume_agent`, `wait_agent`, `close_agent`) once the user has asked for parallel or delegated work. Codex handles orchestration end-to-end: spawning new subagents, routing follow-up instructions, waiting for results, and closing agent threads.

Each subagent is its own Codex session. The child receives:

- Its own `developer_instructions` as the system-prompt core.
- The task prompt composed by the parent.
- The full set of overrides from the custom-agent file (model, sandbox, approval, MCP servers, skills, etc.).
- Whatever context the parent passes in — subagents do **not** see the parent's full conversation history.

The parent waits for results across parallel subagents, then returns a consolidated response. When many agents are running, Codex waits until all requested results are available. The parent can keep multiple subagent threads open in parallel; `agents.max_threads` (default 6) caps how many stay open concurrently.

Subagents inherit the parent's current sandbox policy and the parent's live runtime overrides. The Codex docs explicitly call out: "Codex reapplies the parent turn's live runtime overrides when it spawns a child. That includes sandbox and approval choices you set interactively during the session, such as `/permissions` changes or `--yolo`, even if the selected custom agent file sets different defaults." Within a custom-agent file, `sandbox_mode` and `approval_policy` can narrow or widen the parent's setting, but a custom agent cannot override a live `/permissions` change — the live override wins.

Approval-routing rule: approval requests can surface from inactive agent threads while the user is looking at the main thread. The approval overlay shows the source thread label, and `o` opens that thread before approving, rejecting, or answering. In non-interactive flows (or whenever a run cannot surface a fresh approval), an action that needs new approval fails and the error is surfaced back to the parent workflow.

Model resolution order, top wins:

1. `--model <id>` on the parent CLI invocation (or `model = ...` in the parent's config layer).
2. The custom-agent file's `model` field (alias or full ID).
3. The parent's session model (when the custom-agent file omits `model`).

When the custom-agent file omits `model_reasoning_effort`, the parent's reasoning setting is used.

The default concurrency ceiling is 6 (`agents.max_threads`); the default nesting depth is 1 (`agents.max_depth`, root session starts at 0), which allows a direct child agent to spawn but prevents deeper recursion. Raising `agents.max_depth` allows recursive delegation; the docs warn that recursive delegation can turn broad instructions into repeated fan-out and increases token usage, latency, and local resource consumption. `agents.max_threads` still caps concurrent open threads even at deeper recursion.

There is no documented per-agent `max_turns` field. The relevant turn/timeout caps are `agents.max_threads` (concurrency), `agents.max_depth` (recursion), and `agents.job_max_runtime_seconds` (default per-worker timeout for `spawn_agents_on_csv` jobs, default 1800 seconds when unset, with a per-call `max_runtime_seconds` override taking precedence).

The CSV fan-out workflow (`spawn_agents_on_csv`) reads a CSV, spawns one worker subagent per row, waits for the full batch to finish, and exports the combined results to a CSV. The tool accepts `csv_path`, `instruction` (with `{column_name}` placeholders), `id_column` (for stable item ids from a specific column), `output_schema` (a fixed JSON shape), `output_csv_path`, `max_concurrency`, and `max_runtime_seconds`. Each worker must call `report_agent_job_result` exactly once; a worker that exits without reporting a result is marked with an error in the exported CSV.

Selection is never automatic — Codex only delegates when the user explicitly asks for it. The custom-agent file's `description` is what Codex reads when deciding which agent fits a request, so good descriptions matter for routing.

Disabling: the entire multi-agent feature is gated on `features.multi_agent` (default `true`, `codex features disable multi_agent` to turn off); the tool family disappears from the parent. There is no per-agent disable flag analogous to Claude Code's `permissions.deny: ["Agent(name)"]` — to disable a specific custom agent, rename its file out of `~/.codex/agents/` or `.codex/agents/`, or set its entry to `enabled = false` if it came from a plugin.

## Observability

Subagent starts and stops are visible to wrappers through three coordinated surfaces:

1. **Hook events**: `SubagentStart` fires when a subagent is spawned; `SubagentStop` fires when the subagent finishes. The matcher targets `agent_type`, which carries the custom-agent `name` (or the parent's chosen display nickname when nicknames are used). Built-in names (`default`, `worker`, `explorer`) and any custom `name` value are valid matchers. The full JSON input schema adds to the shared fields:
   - `agent_id` — stable identifier for the subagent (Codex-specific extension).
   - `agent_type` — the custom-agent `name`, or a chosen display nickname.
   - `agent_transcript_path` (SubagentStop only) — path to the subagent's own transcript file when one exists.
   - `turn_id` (Codex-specific extension on both events).
   - `permission_mode` (Codex-specific extension: `default` | `acceptEdits` | `plan` | `dontAsk` | `bypassPermissions`).
   - `session_id` — parent session id (present on every hook; SubagentStart/SubagentStop use the parent session id).
   - `transcript_path` — parent session transcript (always present on hooks; useful for correlating the subagent's `agent_transcript_path` against the parent).
   - `model` — active model slug on every hook input.
   - `stop_hook_active` (SubagentStop only) — whether the subagent was already continued.

   Hook command output: SubagentStop accepts the same output shape as Stop (`continue`, `stopReason`, `systemMessage`, `suppressOutput`). `continue: false` is parsed for compatibility but does **not** stop the subagent from continuing.

2. **Stream output**: `codex exec --json` produces a JSONL stream of `thread.started`, `turn.started`, `turn.completed`, and per-item `item.*` events. The JSONL stream does **not** itself emit dedicated subagent lifecycle events; wrappers that need subagent start/stop in `codex exec` should listen for SubagentStart / SubagentStop hook output (when hooks are configured) or read `agent_id` / `agent_type` from the regular item stream.

3. **Transcripts**: the subagent writes its own JSONL transcript at the path delivered in SubagentStop's `agent_transcript_path`. Transcript retention follows the parent session's history policy (`history.persistence`, `history.max_bytes`) and the SQLite-backed state under `$CODEX_SQLITE_HOME` / `sqlite_home`. `spawn_agents_on_csv` exports a CSV that includes the original row data plus metadata such as `job_id`, `item_id`, `status`, `last_error`, and `result_json`.

## Portability

Custom agents are **not portable** across providers as-is. The body of `developer_instructions` is plain prose and can usually be lifted verbatim into another provider's system-prompt slot, but the surrounding schema is provider-specific.

| Field | Portable? | Rewrite target |
|---|---|---|
| `name` | depends | The identity carrier; the exact value is consumed by Codex's `spawn_agent` / `send_input` / `close_agent` tool family and cannot be directly imported into another provider. |
| `description` | partial | Carries the routing signal across providers, but the surrounding router reads each provider's vocabulary. |
| `developer_instructions` | partial | Verbatim reuse is fine, but references to Codex-specific surface like `spawn_agent`, `send_input`, `report_agent_job_result`, or the Codex tool family have no equivalent in Claude Code. |
| TOML format | no | Every Codex definition is TOML; Claude Code subagents are Markdown+YAML frontmatter — automatic migration requires a TOML→frontmatter parser. |
| `model` / `model_reasoning_effort` | no | Codex aliases like `gpt-5.4-mini` must be remapped to Claude aliases `sonnet` / `opus` / `haiku` or a full Claude model ID. Codex's `model_reasoning_effort` has no direct Claude Code equivalent (drop or rewrite as part of the body prompt). |
| `sandbox_mode` / `approval_policy` | partial | Maps to Claude Code's `permissionMode` (`read-only` → no edits, `workspace-write` → `default`, `danger-full-access` → `bypassPermissions`); the granular table form is Codex-specific. |
| `mcp_servers` | partial | The MCP shape is similar but not identical to Claude Code's `mcpServers`; OAuth, headers, and transport names may diverge. |
| `[[skills.config]]` | partial | Maps to Claude Code's `skills` array of skill names; per-skill `enabled` toggles are not exposed. |
| `nickname_candidates` | no | Presentation-only display pool; Codex-specific UX. |
| `[agents]` global caps (`max_threads`, `max_depth`, `job_max_runtime_seconds`) | no | No equivalent on the Claude Code side. |
| `features.multi_agent` flag | no | Stable, on by default; Claude Code subagents are always on. |
| `spawn_agents_on_csv` fan-out + `report_agent_job_result` worker call | no | Experimental Codex-only workflow. |
| `/agent` slash command | no | Claude Code has no equivalent session-thread switcher. |
| `agents.<name>.config_file` indirection | no | Codex-specific; lets a definition be loaded from an external path. |
| `SubagentStart` / `SubagentStop` hook events | partial | Claude Code exposes the same hooks but with a different payload contract — `agent_type` is a free-form string for both providers, but field shape and output capabilities differ. |

A Codex-plugin-bundled custom agent does not round-trip cleanly across providers because the Codex plugin manifest (`.codex-plugin/plugin.json`) does not document a public `agents/` directory; bundled agents are loaded by their manifest entry or via the `agents.<name>.config_file` indirection. On the Claude Code side, plugin agents register as `plugin-name:agent` and are scoped differently — cross-provider translation of a Codex-plugin-bundled agent is therefore not lossless.

## Claudine Linking Notes

For Claudine's `linking` module and the planned lifecycle `proxy` / `resume` actions, what matters about Codex custom agents:

- Treat `~/.codex/agents/*.toml` and `.codex/agents/*.toml` (recursive from cwd to project root) as the canonical user- and repo-scope agent locations. Plugin-bundled agents load via the plugin manifest; managed / out-of-tree agents load via `agents.<name>.config_file` in any `config.toml` layer.
- For each file, parse the TOML and surface `name`, `description`, `model`, `sandbox_mode`, `mcp_servers` (as identifiers, not raw server definitions), and `nickname_candidates`. The `developer_instructions` body is portable prose and can be embedded into a cross-provider catalog entry verbatim.
- A linked custom agent is portable when its `developer_instructions` uses only generic prose and the file declares only `name`, `description`, optional `model`, optional `mcp_servers` (with provider-neutral URLs), and optional `nickname_candidates`. Flag assets that depend on `agents.<name>.config_file` indirection (managed or out-of-tree), plugin-bundled packaging, `spawn_agents_on_csv` fan-out, `/agent` thread switching, or Codex-specific tool references — they need rewriting, stripping, or host gating before they can land elsewhere.
- For lifecycle `proxy` / `resume`: the wrapper must capture and replay `agent_id` + `agent_type` for the subagent it wants to address. The subagent's stable transcript at the `agent_transcript_path` delivered on `SubagentStop` is the source of truth for resume; the parent transcript's `transcript_path` is also delivered so a wrapper can correlate. Hook output rules differ between SubagentStart and SubagentStop — `continue: false` does not stop a subagent, only the parent's standard `Stop` event honors it as a hard stop.
- Permission policy: the parent session's live `/permissions` change or `--yolo` flag wins over the custom-agent file's `sandbox_mode` / `approval_policy`. A wrapper that pre-loads its own approval policy should treat the parent's live override as the ceiling and apply the custom-agent file's narrowing after.
- Model resolution must follow the documented ladder: parent CLI `--model` → custom-agent file `model` → parent session model. `model_reasoning_effort` falls back the same way. `features.multi_agent` is the gate for the spawn tool family; if a wrapper is targeting a child but `multi_agent` is off, the spawn path is unavailable and the wrapper must surface that explicitly.
- Concurrency / depth: `agents.max_threads` (default 6) is the ceiling on concurrent open agent threads. `agents.max_depth` (default 1) caps how deep delegation can recurse. A `proxy` action that wants to spawn more threads must read these from the active `config.toml` layer and respect them; raising either is a config-layer edit, not a per-call override.
- Whenever Claudine's wrapper code grows a `codex agents` enumeration command or a `--agent <name>` resolution path, it must walk both the user- and project-scope `agents/` directories, parse TOML, and identify each agent by its `name` field (not its filename). The local Codex install on this host (`codex-cli 0.142.5`) is current with the documented `multi_agent = stable` / `true` behavior; older Codex versions may not have the `spawn_agents_on_csv` tool or the full SubagentStart / SubagentStop hook shape.

## Sources

- [Codex — Subagents](https://developers.openai.com/codex/subagents)
- [Codex — Subagent concepts](https://developers.openai.com/codex/concepts/subagents)
- [Codex — Configuration Reference](https://developers.openai.com/codex/config-reference)
- [Codex — Advanced Configuration](https://developers.openai.com/codex/config-advanced)
- [Codex — Hooks](https://developers.openai.com/codex/hooks)
- [Codex — Plugins](https://developers.openai.com/codex/plugins)
- [Codex — Build plugins](https://developers.openai.com/codex/plugins/build)
- [Codex — Environment Variables](https://developers.openai.com/codex/environment-variables)
- [Codex — MCP](https://developers.openai.com/codex/mcp)
- [Codex — Permissions](https://developers.openai.com/codex/permissions)
- [Codex — Sandbox and approvals](https://developers.openai.com/codex/agent-approvals-security)
- [Codex — Feature Maturity](https://developers.openai.com/codex/feature-maturity)
- [Codex — Command Line Options](https://developers.openai.com/codex/cli/reference)
- [Codex — Slash commands](https://developers.openai.com/codex/cli/slash-commands)
- [Codex — Non-interactive Mode](https://developers.openai.com/codex/noninteractive)
- [Codex — Changelog](https://developers.openai.com/codex/changelog)
- [Codex CLI repository](https://github.com/openai/codex)
- [Codex CLI configuration reference (repo docs/config.md)](https://github.com/openai/codex/blob/main/docs/config.md)