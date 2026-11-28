---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://geminicli.com/docs/
system_prompt_docs: https://geminicli.com/docs/cli/system-prompt/
append_support: file
replace_support: env
cli_params:
  - flag: --approval-mode
    mode: modify
    value_shape: "string (default|auto_edit|yolo|plan)"
    description: Selects the execution/approval mode for tool calls. Plan mode is read-only; auto_edit auto-approves edit tools; yolo auto-approves all (equivalent to --yolo). Plan mode is configured at the session level by --approval-mode=plan or via general.plan.enabled.
    example: gemini --approval-mode=plan
    notes: Affects the rendered system prompt through renderPreamble (Default / Plan / YOLO / Auto-Edit) and through the Plan Mode tooling section in v0.49+. Does not directly inject or replace user prompt text.
  - flag: --sandbox / -s
    mode: modify
    value_shape: boolean
    description: Run the session inside a sandboxed environment. The CLI emits a sandbox-aware preamble (renderSandbox) into the effective prompt.
    example: gemini -s
    notes: Indirect prompt shaping only; no own prompt override.
  - flag: --policy <files…>
    mode: other
    value_shape: "comma-separated list or repeated --policy"
    description: Load additional policy TOML files or directories from the command line (settings key policyPaths). Use with the Policy Engine for tool allow/deny/ask_user rules.
    example: gemini --policy ./policy.toml
    notes: Accepted by the binary in v0.46.0 but absent from `gemini --help` (which only lists --help-style options). Policy rules do not rewrite the system prompt.
  - flag: --admin-policy <files…>
    mode: other
    value_shape: "comma-separated list or repeated --admin-policy"
    description: Load additional admin policy files or directories (settings key adminPolicyPaths). Equivalent to policyPaths at a higher tier.
    example: gemini --admin-policy ./admin.toml
    notes: Same caveat as --policy (accepted but not listed in --help). Used to layer system-level overrides on top of user settings.
  - flag: --allowed-mcp-server-names
    mode: modify
    value_shape: array
    description: Restrict which MCP servers are available. Changes the tool set used for ${AvailableTools} substitution inside a custom system.md.
    example: gemini --allowed-mcp-server-names=github,slack
    notes: Indirect prompt effect only — only changes the tool list visible to ${AvailableTools}.
  - flag: --include-directories
    mode: modify
    value_shape: array
    description: Add directories to the workspace. GEMINI.md files in those directories become part of the hierarchical context loaded into the system prompt.
    example: gemini --include-directories ../shared
    notes: Works as an append workaround via file discovery; not a native append flag.
  - flag: --extensions / -e
    mode: modify
    value_shape: array
    description: Enable a subset of installed extensions. Extensions may contribute skills, sub-agents, or tools that surface in ${AgentSkills}/${SubAgents}/${AvailableTools}.
    example: gemini -e my-extension
    notes: Indirect prompt shaping through the rendered agent-skills/sub-agent sections.
  - flag: --model / -m
    mode: other
    value_shape: string
    description: Select the model alias (auto, pro, flash, flash-lite) or a concrete model name for the session.
    example: gemini -m pro
    notes: Does not change prompt text directly, but the snippet renderer selects modern vs legacy output based on model capabilities.
  - flag: --acp
    mode: other
    value_shape: boolean
    description: Start Gemini CLI in Agent Client Protocol (ACP) mode for editor integrations.
    example: gemini --acp
    notes: Replaces the chat-loop transport with ACP; the system prompt surfaces as the ACP `prompt` field on `session/start`. Not relevant for system prompt replacement.
  - flag: --session-file <path> / --session-id <id>
    mode: other
    value_shape: "file path / UUID"
    description: Load a session from JSON or start with a manually supplied session UUID. Resumes without touching the system prompt.
    example: gemini --session-file ./session.json
    notes: Reuses an existing transcript. Does not re-render the system prompt when resuming.
  - flag: --output-format / -o
    mode: other
    value_shape: "string (text|json|stream-json)"
    description: Choose the non-interactive output format. Affects streaming, not the system prompt payload.
    example: gemini -p -o json "summarize README.md"
    notes: Prompt text is unchanged; only the result envelope changes.
config_sources:
  - os: macos
    scope: user
    path: "~/.gemini/system.md"
    mode: replace
    format: markdown
    notes: "Read when GEMINI_SYSTEM_MD=1|true. Tilde-expansion path; relative paths resolved from the current working directory. The CLI errors with `missing system prompt file '<path>'` if the file does not exist."
  - os: linux
    scope: user
    path: "~/.gemini/system.md"
    mode: replace
    format: markdown
    notes: "Read when GEMINI_SYSTEM_MD=1|true. Same as macOS — tilde expansion, cwd-relative paths, hard error on missing file."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\system.md"
    mode: replace
    format: markdown
    notes: "Read when GEMINI_SYSTEM_MD=1|true. Path is user-home rooted; missing file raises `missing system prompt file '<path>'`."
  - os: macos
    scope: repo
    path: "./.gemini/system.md"
    mode: replace
    format: markdown
    notes: "Default project-root path used when GEMINI_SYSTEM_MD=1|true. Resolved from the current working directory."
  - os: linux
    scope: repo
    path: "./.gemini/system.md"
    mode: replace
    format: markdown
    notes: "Default project-root path used when GEMINI_SYSTEM_MD=1|true. Resolved from the current working directory."
  - os: windows
    scope: repo
    path: ".\\.gemini\\system.md"
    mode: replace
    format: markdown
    notes: "Default project-root path used when GEMINI_SYSTEM_MD=1|true. Resolved from the current working directory."
  - os: macos
    scope: user
    path: "~/.gemini/GEMINI.md"
    mode: append
    format: markdown
    notes: "Global context file. Concatenated into the loaded_context block at the bottom of the system prompt by renderUserMemory (precedence: lowest)."
  - os: linux
    scope: user
    path: "~/.gemini/GEMINI.md"
    mode: append
    format: markdown
    notes: "Global context file. Same precedence order as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\GEMINI.md"
    mode: append
    format: markdown
    notes: "Global context file. Same precedence order as macOS/Linux."
  - os: macos
    scope: repo
    path: "./GEMINI.md or ./.gemini/GEMINI.md"
    mode: append
    format: markdown
    notes: "Workspace-level instructions. Loaded into the system prompt above extension memory and below sub-directory GEMINI.md."
  - os: linux
    scope: repo
    path: "./GEMINI.md or ./.gemini/GEMINI.md"
    mode: append
    format: markdown
    notes: "Workspace-level instructions. Loaded into the system prompt with the same precedence as macOS."
  - os: windows
    scope: repo
    path: ".\\GEMINI.md or .\\.gemini\\GEMINI.md"
    mode: append
    format: markdown
    notes: "Workspace-level instructions. Loaded into the system prompt with the same precedence as macOS/Linux."
  - os: macos
    scope: repo
    path: "./**/GEMINI.md"
    mode: append
    format: markdown
    notes: "Just-in-time context files. Discovered when a tool accesses a directory; scanned in that directory and its ancestors up to a trusted root. Highest precedence within the GEMINI.md chain (sub-directories beat workspace root)."
  - os: linux
    scope: repo
    path: "./**/GEMINI.md"
    mode: append
    format: markdown
    notes: "Same JIT discovery rules as macOS. Resolved by MemoryDiscoveryService against trusted folder roots."
  - os: windows
    scope: repo
    path: ".\\**\\GEMINI.md"
    mode: append
    format: markdown
    notes: "Same JIT discovery rules as macOS/Linux with Windows path separators. Resolved against trusted folder roots."
  - os: macos
    scope: user
    path: "~/.gemini/settings.json"
    mode: modify
    format: json
    notes: "User settings. Prompt-relevant keys: `context.fileName` (customise the discovered context filename), `general.defaultApprovalMode`, `general.plan.enabled`, `general.plan.directory`, `experimental.enableAgents`, `agents.overrides`, `modelConfigs.overrides` (systemInstruction prefixes via overrideScope)."
  - os: linux
    scope: user
    path: "~/.gemini/settings.json"
    mode: modify
    format: json
    notes: "Same prompt-affecting keys as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\settings.json"
    mode: modify
    format: json
    notes: "Same prompt-affecting keys as macOS/Linux."
  - os: macos
    scope: repo
    path: "./.gemini/settings.json"
    mode: modify
    format: json
    notes: "Project settings; can override context.fileName, agent overrides, plan-mode behaviour."
  - os: linux
    scope: repo
    path: "./.gemini/settings.json"
    mode: modify
    format: json
    notes: "Same prompt-affecting keys as macOS."
  - os: windows
    scope: repo
    path: ".\\.gemini\\settings.json"
    mode: modify
    format: json
    notes: "Same prompt-affecting keys as macOS/Linux."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/system-defaults.json"
    mode: modify
    format: json
    notes: "System-wide defaults file (lowest precedence). Path overridable by GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/system-defaults.json"
    mode: modify
    format: json
    notes: "System-wide defaults file (lowest precedence). Path overridable by GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\system-defaults.json"
    mode: modify
    format: json
    notes: "System-wide defaults file (lowest precedence). Path overridable by GEMINI_CLI_SYSTEM_DEFAULTS_PATH."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/settings.json"
    mode: modify
    format: json
    notes: "System override file (highest JSON precedence; beats user/project settings). Path overridable by GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/settings.json"
    mode: modify
    format: json
    notes: "System override file (highest JSON precedence). Path overridable by GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    mode: modify
    format: json
    notes: "System override file (highest JSON precedence). Path overridable by GEMINI_CLI_SYSTEM_SETTINGS_PATH."
  - os: macos
    scope: user
    path: "~/.gemini/agents/*.md"
    mode: replace
    format: markdown
    notes: "User-level sub-agent definitions. YAML frontmatter (name, description, kind, tools, mcpServers, model, temperature, max_turns, timeout_mins); the markdown body becomes the sub-agent's system prompt."
  - os: linux
    scope: user
    path: "~/.gemini/agents/*.md"
    mode: replace
    format: markdown
    notes: "User-level sub-agent definitions; same schema as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\agents\\*.md"
    mode: replace
    format: markdown
    notes: "User-level sub-agent definitions; same schema as macOS/Linux."
  - os: macos
    scope: repo
    path: "./.gemini/agents/*.md"
    mode: replace
    format: markdown
    notes: "Project-level sub-agent definitions. Project agents are first-wins (project overrides user on name collisions, per v0.44.0 PR #26953)."
  - os: linux
    scope: repo
    path: "./.gemini/agents/*.md"
    mode: replace
    format: markdown
    notes: "Project-level sub-agent definitions; project beats user on collisions."
  - os: windows
    scope: repo
    path: ".\\.gemini\\agents\\*.md"
    mode: replace
    format: markdown
    notes: "Project-level sub-agent definitions; project beats user on collisions."
  - os: macos
    scope: extension
    path: "<extension>/agents/*.md"
    mode: replace
    format: markdown
    notes: "Extension-packaged sub-agents. Distributed alongside the extension; the agent body becomes the sub-agent's system prompt."
  - os: linux
    scope: extension
    path: "<extension>/agents/*.md"
    mode: replace
    format: markdown
    notes: "Extension-packaged sub-agents; same schema as macOS."
  - os: windows
    scope: extension
    path: "<extension>\\agents\\*.md"
    mode: replace
    format: markdown
    notes: "Extension-packaged sub-agents; same schema as macOS/Linux."
  - os: macos
    scope: user
    path: "~/.gemini/skills/**/SKILL.md"
    mode: other
    format: markdown
    notes: "Agent Skills surfaced into the system prompt as ${AgentSkills}. Only metadata (name + description) is loaded at session start; full SKILL.md body is appended to the conversation when the model activates the skill."
  - os: linux
    scope: user
    path: "~/.gemini/skills/**/SKILL.md"
    mode: other
    format: markdown
    notes: "Agent Skills surfaced into the system prompt as ${AgentSkills}. Same load order and onboarding rules as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\skills\\**\\SKILL.md"
    mode: other
    format: markdown
    notes: "Agent Skills surfaced as ${AgentSkills}; same load order and rules."
  - os: macos
    scope: repo
    path: "./.gemini/skills/**/SKILL.md or ./.agents/skills/**/SKILL.md"
    mode: other
    format: markdown
    notes: "Workspace Skills. ./.agents/skills/ takes precedence over ./.gemini/skills/ on name collisions within the same tier."
  - os: linux
    scope: repo
    path: "./.gemini/skills/**/SKILL.md or ./.agents/skills/**/SKILL.md"
    mode: other
    format: markdown
    notes: "Workspace Skills; same alias precedence as macOS."
  - os: windows
    scope: repo
    path: ".\\.gemini\\skills\\**\\SKILL.md or .\\.agents\\skills\\**\\SKILL.md"
    mode: other
    format: markdown
    notes: "Workspace Skills; same alias precedence as macOS/Linux."
  - os: macos
    scope: repo
    path: "./.gemini/.env"
    mode: other
    format: other
    notes: "Project-level env file. The CLI loads it at boot and persists overrides like GEMINI_SYSTEM_MD=1 for the workspace."
  - os: linux
    scope: repo
    path: "./.gemini/.env"
    mode: other
    format: other
    notes: "Project-level env file; same role as macOS."
  - os: windows
    scope: repo
    path: ".\\.gemini\\.env"
    mode: other
    format: other
    notes: "Project-level env file; same role as macOS/Linux."
env_vars:
  - name: GEMINI_SYSTEM_MD
    effect: "Replaces the built-in core system prompt with the contents of a Markdown file. `1`/`true` reads ./.gemini/system.md; any other value is treated as a path (relative/absolute, `~`-expanded). `0`/`false` or unset restores the built-in prompt; missing file raises `missing system prompt file '<path>'`."
    mode: replace
  - name: GEMINI_WRITE_SYSTEM_MD
    effect: "Exports the current built-in system prompt to a file. `1`/`true` writes ./.gemini/system.md; any other value is treated as a path. Run the CLI once with this set to materialise the file (mkdirSync + writeFileSync)."
    mode: inspect
  - name: GEMINI_API_KEY
    effect: "Auth: API key for Gemini API authentication. Does not affect prompt content."
    mode: other
  - name: GOOGLE_API_KEY
    effect: "Auth: API key for Gemini API / Vertex. Does not affect prompt content."
    mode: other
  - name: GOOGLE_GENAI_USE_VERTEXAI
    effect: "Auth: switch to Vertex AI mode. Does not affect prompt content."
    mode: other
  - name: GOOGLE_CLOUD_PROJECT
    effect: "Auth: Cloud project for Code Assist / Vertex. Does not affect prompt content."
    mode: other
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: "Override the path to the system defaults JSON file (lowest-precedence JSON layer)."
    mode: modify
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: "Override the path to the system override JSON file (highest-precedence JSON layer)."
    mode: modify
  - name: GEMINI_SANDBOX
    effect: "Force sandbox mode. The renderer inserts a sandbox-aware preamble into the effective prompt."
    mode: modify
  - name: SEATBELT_PROFILE
    effect: "macOS-specific. Switches the Seatbelt sandbox profile (permissive-open, restrictive-open, strict-open, strict-proxied, or a custom .gemini/sandbox-macos-<name>.sb)."
    mode: modify
  - name: DEBUG
    effect: "Verbose debug logging. Does not affect prompt content but the `--debug` flag parallels this in v0.46+."
    mode: other
prompt_layers:
  - source: Built-in core system prompt (PromptProvider.getCoreSystemPrompt)
    mode: replace
    scope: ["builtin"]
    order_notes: "Base layer; replaced entirely when GEMINI_SYSTEM_MD points at a readable file. Rendered via snippets (renderPreamble + renderCoreMandates + renderOperationalGuidelines + tools/skills/sub-agents sub-shells)."
    notes: "Not directly exportable as plain text through the CLI; use GEMINI_WRITE_SYSTEM_MD to dump it. Contains per-machine sections (cwd, platform, shell, OS version, git-repo flag) and topic-update narration."
  - source: GEMINI_SYSTEM_MD file
    mode: replace
    scope: ["session"]
    order_notes: "Replaces the built-in base when GEMINI_SYSTEM_MD points at a readable file. Skills/tools/sub-agents are not injected unless the file uses ${AgentSkills}, ${SubAgents}, ${AvailableTools}, or ${toolName_ToolName} substitutions."
    notes: "Variable substitution is applied via applySubstitutions so a custom file can keep dynamic sections. Missing file is a hard error."
  - source: GEMINI.md hierarchical context (renderUserMemory)
    mode: append
    scope: ["user", "repo"]
    order_notes: "Appended at the bottom of every prompt via renderFinalShell → renderUserMemory as `<loaded_context>…</loaded_context>` inside `# Contextual Instructions (GEMINI.md)`. Order: sub-directory > workspace root > extension > global."
    notes: "Strict conflict-resolution rule baked into the renderer: sub-directory rules supersede workspace, which supersede extension, which supersede global — but cannot override Core Mandates."
  - source: Active topic narration
    mode: append
    scope: ["session"]
    order_notes: "Appended after GEMINI.md when topic narration is enabled: `[Active Topic: <topic>]`."
    notes: "Triggered by the topic-update-narration feature (general.topicUpdateNarration). Suppressible by settings."
  - source: Agent Skills metadata
    mode: append
    scope: ["session"]
    order_notes: "Injected into the built-in prompt as `${AgentSkills}`. Only metadata (name + description) is loaded at session start; full SKILL.md body is appended to conversation context only when the model activates the skill via activate_skill."
    notes: "Discovery tiers: built-in > extension > user > workspace. ./.agents/skills/ alias takes precedence over ./.gemini/skills/."
  - source: Built-in sub-agents
    mode: append
    scope: ["session", "subagent"]
    order_notes: "Rendered via `${SubAgents}`. Custom local sub-agents (subagent1: agents/*.md) are first-wins on name collisions, project beat user; remote sub-agents are also enumerated when configured."
    notes: "Built-ins: codebase_investigator, cli_help, generalist, browser_agent (experimental, off by default, requires Chrome ≥144)."
  - source: Auto Memory (MEMORY.md and sibling *.md)
    mode: append
    scope: ["session", "user", "repo"]
    order_notes: "Loaded alongside GEMINI.md content in the `<loaded_context>` block. MEMORY.md is the index, sibling notes are loaded on demand. Auto Memory is experimental and disabled by default (`experimental.autoMemory: true`)."
    notes: "Memory tiers mirror GEMINI.md: project private folder, global personal. Never cross-reference with GEMINI.md."
  - source: Sub-agent system prompt (custom .md body)
    mode: replace
    scope: ["subagent"]
    order_notes: "Replaces the orchestrator prompt for the spawned sub-agent. Sub-agent has its own isolated context loop; only the final result returns to the parent."
    notes: "Agents cannot call other agents (recursion protection), even when granted the `*` wildcard. Inline MCP servers may be scoped per sub-agent via `mcpServers`."
agent_prompting:
  supported: true
  definition_surface: "Markdown files with YAML frontmatter in `~/.gemini/agents/`, `./.gemini/agents/`, or `<extension>/agents/`. Frontmatter fields: name, description, kind (local|remote, default local), tools, mcpServers, model (default `inherit`), temperature, max_turns (default 30), timeout_mins (default 10). Configuration overrides land in `~/.gemini/settings.json` under `agents.overrides`."
  inheritance: "Each sub-agent receives its own system prompt (the file body) plus the main agent's tool set when `tools` is omitted. Sub-agents do not inherit the parent's user messages or transient context. Per-sub-agent MCP servers can be scoped via `mcpServers` for isolation. Custom local sub-agents use first-wins collision rules with project > user priority."
  isolation: "Sub-agents run in their own context loop. Independent conversation history, isolated tools (only those listed in `tools` plus tool allowlist semantics), and only the final result returns to the parent. A sub-agent's prompt is independent of the parent's GEMINI.md unless the parent explicitly invokes it."
  limitations: "Sub-agents cannot spawn other sub-agents even with the `*` tool wildcard. Remote (kind: remote) agents use the Agent-to-Agent (A2A) protocol and require separate auth. Browser agent requires Chrome ≥144 and bundling consent on first run. Sub-agent tool isolation was stabilised in v0.44.x and refined through v0.49.0."
claudine_delivery:
  append_strategy: shadow_home_file
  replace_strategy: env_var_file
  temp_file_required: true
  argv_limit: "Not applicable; Gemini CLI has no native per-invocation system-prompt argv flags. Prompt text is loaded from a file path, so argv stays small."
  notes: "**Replace**: write the resolved prompt to a temporary Markdown file and set `GEMINI_SYSTEM_MD=<tmp>` for the wrapped invocation. Per-invocation and persistent-free. Optional `${AgentSkills}`, `${SubAgents}`, `${AvailableTools}` substitutions let the wrapper preserve dynamic content. **Append**: drop a Markdown file into a temporary directory and inject that directory into the workspace via `--include-directories` so MemoryDiscovery picks it up as a GEMINI.md match — this is the closest native append mechanism that does not mutate persistent GEMINI.md files in the user's repo. A cleaner alternative is to write the resolved appended text into the project tree under a sibling `.gemini-runtime/` directory and surface it via `--include-directories`. **No persistent user config mutation** is required for either mode."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Gemini CLI's append and replace surfaces are both Markdown-first. `GEMINI_SYSTEM_MD` accepts Markdown, `renderUserMemory` wraps appended context in Markdown, and the agent Skills convention uses SKILL.md. The renderer does not parse XML, YAML, or JSON envelopes — those formats lose fidelity or add tokens. For replacements the file should opt back into dynamic content with `${AgentSkills}`, `${SubAgents}`, `${AvailableTools}`, and `${<toolName>_ToolName}` substitutions to preserve skills/tools/sub-agents."
recent_changes:
  - date: "2026-06-25"
    version: "v0.49.0"
    change: "Latest stable release. Includes coreTools→tools.core migration, zero-quota fail-fast fix, MCP atomic update fix, and tool output formatting standardisation. Migrated `coreTools` setting to `tools.core` — wrappers that still emit the old key receive no effect."
    impact: "Wrappers templating settings.json must use `tools.core`; `coreTools` is no longer recognised. No direct effect on system-prompt replacement, but simplifies the path before custom user prompts."
  - date: "2026-06-18"
    version: "v0.47.0"
    change: "Anti-gravity CLI migration banner added (`update the max amount of times the Antigravity transition banner can be displayed`); Vertex AI model mapping fix; policy EBUSY fallback + TOML parse recovery."
    impact: "Unpaid-tier Gemini CLI users (free + Google One) are transitioned to Antigravity CLI starting this date. Verified against https://geminicli.com/docs/cli/system-prompt/ (banner preserved as of v0.49.x). The system-prompt mechanism in Antigravity CLI is independent of Gemini CLI and outside this topic."
  - date: "2026-06-10"
    version: "v0.46.0"
    change: "Local installation version used for source-of-truth inspection (verified via `gemini --version` returning 0.46.0). Fixes PTY resize crashes, preferredEditor spam loop, and adds Transition-to-Flash GA model with experimental flag."
    impact: "No direct system-prompt changes; documents that `--policy`, `--admin-policy`, `--session-file`, `--session-id`, `--raw-output`, and `--output-format` are accepted by the binary in v0.46 but not listed in `gemini --help`."
  - date: "2026-05-27"
    version: "v0.44.0"
    change: "Context Manager Simplification. `fix(core): made context files append instead of replace` (#26950) confirmed the append-not-replace semantics for GEMINI.md content. PR #26953 made agent registration first-wins with project priority."
    impact: "Establishes that `.gemini/system.md` is a full replacement but `.gemini/<context>.md` files (e.g. GEMINI.md) are appended into the system prompt via `renderUserMemory` rather than treated as separate context messages."
  - date: "2026-04"
    version: "v0.42.x"
    change: "Auto Memory introduced as an experimental feature that mines past sessions for memory updates and reusable Skills."
    impact: "Adds an experimental Memory tier that may inject memory patches into the system prompt once approved. Disabled by default; opt-in via `experimental.autoMemory: true` in settings.json."
quirks:
  - Gemini CLI has no `--append-system-prompt`, `--system-prompt`, or equivalent CLI flag. Per-invocation prompt overrides are env-var or file-discovery only.
  - "**GEMINI.md is part of the system prompt, not a separate context message.** `PromptProvider.renderFinalShell` appends `renderUserMemory(...)` to the base prompt, wrapping user/GEMINI content inside a `<loaded_context>` block under `# Contextual Instructions (GEMINI.md)`. The existing prior-research claim that GEMINI.md is \"softer than a true system-prompt append\" is technically incorrect — it is appended directly to the system prompt, but with strict sub-directory > workspace > extension > global precedence."
  - "When `GEMINI_SYSTEM_MD` is active, the CLI still appends GEMINI.md (and skills/memory) to the override. The user must opt back into dynamic content via `${AgentSkills}`, `${SubAgents}`, `${AvailableTools}`, or `${<toolName>_ToolName}` substitutions or the model will see only the static text."
  - "The CLI shows a `|⌐■_■|` indicator in the UI when GEMINI_SYSTEM_MD is active (system-prompt.md doc)."
  - "Export the built-in prompt first with `GEMINI_WRITE_SYSTEM_MD=1 gemini` before editing — the override is full-replace, so the developer must re-add any Core Mandates or tool protocols they need."
  - "Plan Mode is governed by `general.plan.enabled` (default true) and `--approval-mode=plan`. The Plan preamble appears as a sub-shell in the rendered prompt and lists tools with `<tool>` tags."
  - "Many flags (`--policy`, `--admin-policy`, `--acp`, `--session-file`, `--session-id`, `--list-sessions`, `--delete-session`, `--raw-output`, `--accept-raw-output-risk`, `--output-format`, `--screen-reader`) are accepted by the binary but absent from `gemini --help`. Do not rely on `--help` to detect them in scripts."
  - "Sub-agents cannot recursively spawn other sub-agents, even with the `*` tool wildcard."
  - "Persisting `GEMINI_SYSTEM_MD=1` in `./.gemini/.env` makes the override durable for the project. To revert, unset the env var or set it to `0`/`false`."
  - "Custom context filenames (e.g. AGENTS.md, CONTEXT.md) can be enabled via `context.fileName` in settings.json; the renderer prepends them as the literal header `# Contextual Instructions (<filenames>)`."
  - "Sub-agent collisions resolve first-wins (project > user) per PR #26953 (v0.44.0). Tools list resolution defaults to inheriting the parent's tools when omitted."
  - "Agent Skills render as `${AgentSkills}` — only metadata is loaded at startup; full SKILL.md body is appended when the model activates the skill."
  - "`--include-directories` is the closest native append workaround: a Claudine wrapper can drop a Markdown file into a temporary directory and inject the directory to discover a runtime GEMINI.md without touching the user's workspace."
  - "The bundled docs still reference v0.44.0 as `latest` even though GitHub releases list v0.49.0 (June 25, 2026) — treat the GitHub releases page and the hosted geminicli.com site as authoritative."
  - "Anti-gravity CLI migration banner for free and Google One users ran on June 18, 2026. The Gemini CLI binary remains available for paid (Tier 1/2/3) users, but free-tier users get redirected to Antigravity CLI."
gaps:
  - "Bundled CLI docs shipped with v0.46.0 still label the latest release as v0.44.0; the changes for v0.45.x → v0.49.0 are documented only on the GitHub releases page. A definitive changelog for system-prompt-related changes between v0.45 and v0.49 was not located beyond the PR notes inspected."
  - "No flag exists that lets the wrapper skip Gemini.md discovery for a single invocation — `GEMINI_SYSTEM_MD` replaces the base but does not disable MemoryDiscoveryService. To fully isolate the prompt, a wrapper would need a shadow HOME or to use `--include-directories` to a clean directory."
  - "Whether the `Policy Engine` ever injects text into the system prompt is undocumented. Current evidence is that policy TOML files govern tool allow/deny/ask_user and do not add system-prompt text."
  - "The exact rendering order between `renderUserMemory` subdirectories and the active topic narration block is implementation-defined (the source orders them after the base prompt) but the documentation does not call it out."
  - "The Antigravity CLI replacement may use a different system-prompt mechanism than Gemini CLI; this research only covers the Gemini CLI binary."
  - "Auto Memory's exact effect on the rendered system prompt when enabled has not been exhaustively documented in either the doc site or the bundled docs."
changes:
  - "Split `os: all` config_sources records into per-OS entries (macos / linux / windows) per the schema enum (the prior research violated validation)."
  - "Updated `last_updated` to 2026-07-03, set `agent` to `open_code` and `model` to `minimax/MiniMax-M3`."
  - "Refreshed recent_changes against the GitHub releases page (v0.49.0 latest stable on 2026-06-25) — the bundled docs in v0.46 are stale."
  - "Added `--policy`, `--admin-policy`, `--acp`, `--session-file`, `--session-id`, and `--output-format` to cli_params (verified accepted by `gemini --help`-trailing options in v0.46)."
  - "Confirmed that `GEMINI.md` content is concatenated into the system prompt via `renderFinalShell` → `renderUserMemory` (source-of-truth: chunk-G33JEOEV.js in the v0.46 bundle, lines 331439–331748, 332974–333122). The body now describes this correctly."
  - "Recorded the untaxable behavior that `coreTools` moved to `tools.core` in v0.49.0 — wrappers templating settings.json must use the new key."
  - "Added per-OS `config_sources` entries for system defaults/settings, extension sub-agents, Agent Skills, and the project `.env` so all documented surfaces are covered."
  - "Updated `gaps` with the Auto Memory rendering question, the Antigravity CLI unknown, and the lack of a hard-disable for MemoryDiscovery."
  - "Quirk added: many real flags are absent from `gemini --help` but accepted by the binary."
  - "Reason rewritten to reflect the same stable delivery model (env-var replace + file-flag append via --include-directories) without a Claudine schema change."
requires_claudine_update: false
reason: "Gemini CLI still has no native CLI flag for system-prompt append or replace, and Claudine's wrapper strategy (env_var_file for replace, file-based append via --include-directories or a sibling `.gemini-runtime/`) is unchanged. The schema fix here is a formatting change (per-OS records) rather than a behavior change. The `tools.core` rename does not impact Claudine's existing settings.json template because Claudine does not emit `coreTools`. No Claudine metadata or wrapper code change is required."
---

## Overview

Gemini CLI exposes two distinct levers for influencing what the model sees: a **core system prompt** (`system.md` / `GEMINI_SYSTEM_MD`) and a **hierarchical context layer** (`GEMINI.md` and friends). The CLI has no `--append-system-prompt` or `--system-prompt` flag; manipulation is environment-variable or file-discovery based. Custom sub-agents carry their own isolated system prompts defined in Markdown files. Native Skills and Auto Memory add further layers, all of them ultimately rendered into the system prompt through `PromptProvider.getCoreSystemPrompt` and the `renderFinalShell` / `renderUserMemory` snippets in `packages/core/src/prompts/promptProvider.ts`.

The locally installed binary is `gemini 0.46.0` (npm `@google/gemini-cli`, Node.js v22.20.0). The latest stable release on GitHub is **v0.49.0** (released 2026-06-25); v0.50.0-preview.1 and v0.51.0-nightly exist but are not generally recommended. The free-tier migration to Antigravity CLI took effect on **2026-06-18** (per the banner on `geminicli.com/docs/cli/system-prompt/` and PR #27676 / #27765). Anti-gravity CLI is out of scope for this document.

## CLI Parameters

Gemini CLI does not expose a flag that directly appends or replaces system-prompt text. The flags that touch the effective prompt do so indirectly:

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--approval-mode <mode>` | Modify | Selects default, auto_edit, plan, or yolo. The Plan preamble is rendered into the system prompt when plan mode is active. |
| `--sandbox` / `-s` | Modify | Activates sandboxing; inserts a sandbox-aware preamble (renderSandbox). |
| `--policy <files…>` | Other | Loads additional Policy Engine TOML paths from the command line (settings key `policyPaths`). |
| `--admin-policy <files…>` | Other | Loads additional admin policy paths from the command line (settings key `adminPolicyPaths`). |
| `--allowed-mcp-server-names <list>` | Modify | Restricts which MCP servers are loaded; narrows the `${AvailableTools}` list. |
| `--include-directories <list>` | Modify | Adds workspace directories — any `GEMINI.md` they contain is appended into the system prompt via `renderUserMemory`. |
| `--extensions <list>` / `-e` | Modify | Enables a subset of extensions; affects what skills / sub-agents / tools the renderer enumerates. |
| `--model <alias>` / `-m` | Other | Selects the model — also gates the modern vs legacy snippet renderer. |
| `--acp` | Other | Starts in Agent Client Protocol mode for editor integrations. |
| `--session-file <path>` / `--session-id <id>` | Other | Resumes a session without re-rendering the system prompt. |
| `--output-format / -o` | Other | Selects text, json, or stream-json output for non-interactive runs. |

Many of these flags are accepted by the binary but absent from `gemini --help` output in v0.46.0 (verified by running the binary against unknown-arg flags). Treat `--policy`, `--admin-policy`, `--session-file`, `--session-id`, `--raw-output`, `--accept-raw-output-risk`, `--screen-reader`, `--list-sessions`, `--delete-session`, `--output-format`, and `--acp` as real flags when wrapping the CLI. There is no inline-text nor file-flag for direct system-prompt replacement or append.

## Configuration and Discovery

### Core system prompt replacement (`GEMINI_SYSTEM_MD`)

The core system prompt is replaced by setting the `GEMINI_SYSTEM_MD` environment variable. The decision is made by `PromptProvider.getCoreSystemPrompt`:

```js
const systemMdResolution = resolvePathFromEnv(process.env["GEMINI_SYSTEM_MD"]);
…
if (systemMdResolution.value && !systemMdResolution.isDisabled) {
  let systemMdPath = path.resolve(path.join(GEMINI_DIR, "system.md"));
  if (!systemMdResolution.isSwitch) {
    systemMdPath = systemMdResolution.value;
  }
  if (!fs.existsSync(systemMdPath)) {
    throw new Error(`missing system prompt file '${systemMdPath}'`);
  }
  basePrompt = fs.readFileSync(systemMdPath, "utf8");
  basePrompt = applySubstitutions(basePrompt, …);
}
```

Rules (per `docs/cli/system-prompt.md` and `docs/reference/configuration.md`):

- `GEMINI_SYSTEM_MD=1` or `GEMINI_SYSTEM_MD=true` reads `./.gemini/system.md`.
- Any other non-empty string is treated as a path (relative/absolute, `~`-expanded).
- `0` / `false` or unset restores the built-in prompt.
- Missing file raises `missing system prompt file '<path>'`.
- The CLI shows a `|⌐■_■|` indicator in the UI when active.

`applySubstitutions` lets a custom file include the dynamic sections it needs:

- `${AgentSkills}` — full section header + bulleted skills list.
- `${SubAgents}` — full section header + bulleted sub-agent names.
- `${AvailableTools}` — bulleted tool list.
- `${<toolName>_ToolName}` — the actual name of a single tool, e.g. `${write_file_ToolName}` or `${run_shell_command_ToolName}`.

### Project and global context (`GEMINI.md`)

`GEMINI.md` files are concatenated into the system prompt by `renderFinalShell` → `renderUserMemory`. They are **not** a soft context channel — they are rendered directly under the base prompt. The CLI documents three layers:

1. `~/.gemini/GEMINI.md` (global)
2. Workspace `GEMINI.md` (configured workspace directories and their parents)
3. Just-in-time `GEMINI.md` files discovered when a tool accesses a directory or its ancestors up to a trusted root

The renderer enforces a strict precedence: sub-directories > workspace root > extensions > global; context can override default operational behaviors but never Core Mandates.

```mermaid
graph TD
  A[Built-in core system prompt] --> B{GEMINI_SYSTEM_MD set?}
  B -- yes --> C[Contents of system.md file<br/>with substitutions applied]
  B -- no --> A
  C --> D[renderFinalShell → renderUserMemory]
  A --> D
  D --> D1[/Global ~/.gemini/GEMINI.md/]
  D --> D2[/Extension memory/]
  D --> D3[/Workspace ./GEMINI.md/]
  D --> D4[/Sub-directory **&#47;GEMINI.md/]
  D --> F[Active topic narration]
  D --> E[Auto-memory MEMORY.md<br/>if enabled]
  D --> P[User prompt]
```

The footer shows the count of loaded context files. `/memory show` displays the concatenated context; `/memory reload` rescans. `GEMINI.md` filenames can be customised via `context.fileName` (string or string[]) in `settings.json`; the renderer repeats the list in the `# Contextual Instructions (<filenames>)` header.

### Imports and custom filenames

`GEMINI.md` files can pull in other Markdown via `@./path/to/file.md` (relative or absolute, recursive up to four hops, code-fence-aware parsing). Importing into a runtime file is supported; the `[DEBUG] [MemoryDiscovery]` warns in source if a directory matches a `GEMINI.md` path but is skipped (e.g. EISDIR virtual drives).

### Sub-agent definitions

Custom sub-agents are Markdown files with YAML frontmatter. Locations:

- `~/.gemini/agents/*.md` (user scope)
- `./.gemini/agents/*.md` (project scope; project overrides user on name collisions — PR #26953, v0.44.0)
- `<extension>/agents/*.md` (extension scope)

The markdown body becomes the sub-agent's system prompt. Frontmatter fields: `name`, `description` (required), `kind` (`local` default or `remote` for A2A), `tools`, `mcp_servers`, `model` (default `inherit`), `temperature`, `max_turns` (default 30), `timeout_mins` (default 10). Configuration overrides live under `agents.overrides` in `settings.json` (`enabled`, `modelConfig.model`, `runConfig.maxTurns`).

### Agent Skills

Skills are folders with a `SKILL.md` frontmatter. At session start, only metadata (name + description) is loaded into the system prompt via `${AgentSkills}`. The full `SKILL.md` body is appended to the conversation when the model calls the `activate_skill` tool. Discovery tiers: built-in > extension > user > workspace; `~/.agents/skills/` and `./.agents/skills/` aliases take precedence over `~/.gemini/skills/` and `./.gemini/skills/` on name collisions.

### Auto Memory

Auto Memory (experimental, `experimental.autoMemory: true`) extracts memory updates and Skill candidates from past sessions. `MEMORY.md` is an index; sibling `*.md` notes hold detail. Candidates are dropped in a project-local inbox for approval before being written. Disabled by default; the flag requires a session restart.

## Prompt Layers and Precedence

The effective prompt is assembled in this order:

```mermaid
graph TD
  A[Built-in core system prompt<br/>snippets.renderPreamble + Core Mandates + Operational Guidelines] --> B{GEMINI_SYSTEM_MD set?}
  B -- yes --> C[system.md contents<br/>applySubstitutions]
  B -- no --> A
  C --> D[renderFinalShell / renderUserMemory<br/>subdirs > workspace > ext > global]
  A --> D
  D --> E[${AgentSkills} metadata<br/>only name + description]
  E --> F[${SubAgents} enumeration<br/>built-ins + custom local + remote]
  F --> G[${AvailableTools} tool list]
  G --> H[Active topic narration<br/>if enabled]
  H --> I[Auto memory MEMORY.md<br/>if experimental.autoMemory]
  I --> J[User prompt]
```

Notes on precedence:

- `GEMINI_SYSTEM_MD` replaces the built-in base entirely. The renderer still appends GEMINI.md context on top of the override (`renderFinalShell` calls `renderUserMemory` after `basePrompt.trim()`).
- Skills, sub-agents, and tools must be re-introduced with the substitution variables in a custom `system.md` if the task still needs them.
- Sub-agents use the body of their Markdown file as their own isolated system prompt; they do not inherit the parent's GEMINI.md content automatically.
- Sub-agents cannot spawn other sub-agents (recursion protection), even when granted the `*` tool wildcard.

## Agents and Subagents

Gemini CLI ships with built-in sub-agents (`codebase_investigator`, `cli_help`, `generalist`, `browser_agent`) and supports local custom sub-agents defined as Markdown files. Each sub-agent has its own system prompt (the markdown body), its own tool set, optional inline MCP servers, and an isolated context loop.

Key behaviors (per `docs/core/subagents.md` and `packages/core/src/agents/local-executor.ts` schema):

- Custom agents live in `~/.gemini/agents/*.md`, `./.gemini/agents/*.md`, or `<extension>/agents/*.md`. Required frontmatter: `name`, `description`. Optional: `kind` (`local`/`remote`, default `local`), `tools`, `mcp_servers`, `model` (default `inherit`), `temperature` (0.0–2.0, default 1), `max_turns` (default 30), `timeout_mins` (default 10).
- Built-in agents are listed via `${SubAgents}` in the main session's prompt and can be forced via `@codebase_investigator …` syntax at the start of a prompt.
- Browser agent requires Chrome ≥144 and is off by default until `agents.overrides.browser_agent.enabled: true` is set in settings.json.
- Custom local sub-agents use `first-wins` collision rules with project > user priority (PR #26953, v0.44.0).
- Each sub-agent runs in its own context loop; only the final result returns to the parent. Sub-agent prompt is independent of the parent's GEMINI.md content unless the parent explicitly invokes it.
- Tools listed in `tools` are explicit; if omitted, the sub-agent inherits the parent's tool set. Wildcards include `*` (all tools), `mcp_*` (all MCP tools), and `mcp_<server>_*` (all tools from a specific server).
- The Policy Engine treats sub-agents as virtual tool names for allow/deny/ask_user decisions: a rule with `toolName = "codebase_investigator"` denies the sub-agent at the system policy level.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append (`GEMINI.md`) | Pure Markdown | Headers, bullet lists, and short paragraphs blend cleanly with the rendered `<loaded_context>` block. |
| Replace (`system.md`) | Markdown with `${…}` substitutions | The renderer passes the file through `applySubstitutions`, so re-introduce skills/sub-agents/tools explicitly. XML tags add no documented advantage here. |

Wrapping a replacement:

```markdown
# Custom system prompt

You are a focused reviewer.

## Dynamic content
${AgentSkills}
${SubAgents}

## Tools available
${AvailableTools}
Use ${run_shell_command_ToolName} for shell access and ${read_file_ToolName} to inspect files.
```

Wrapping an append (drop into a temporary directory and inject via `--include-directories`):

```markdown
# Additional instructions

Always run `pnpm test` after edits.

## Style
- Prefer named exports.
- Avoid `any`.
```

JSON and YAML are not natural fits: the renderer does not parse either as a wrapper envelope and they cost more tokens than plain Markdown in the model context.

## Recent Changes

- **v0.49.0 — 2026-06-25 (latest stable)**: `coreTools` was migrated to `tools.core` (`fix(config): migrate coreTools setting to tools.core` #27947). Zero-quota limits now fail fast to prevent retry loops. MCP tool discovery uses atomic update. Tool output formatting standardised.
- **v0.47.0 — 2026-06-18**: Antigravity CLI migration banner (`update the max amount of times the Antigravity transition banner can be displayed`, PR #27676) and migration documentation (`Add documentation and migration commands for Antigravity CLI`, PR #27765). Policy TOML gets EBUSY fallback and parser recovery. Vertex AI model mapping fix.
- **v0.46.0 — 2026-06-10** (local binary version verified): PTY resize hardening; preferredEditor spam-loop fix; transition to Flash GA model behind an experimental flag. Source-of-truth inspection of `packages/core/src/prompts/promptProvider.ts` confirms `GEMINI_SYSTEM_MD`, `GEMINI_WRITE_SYSTEM_MD`, `${AgentSkills}`, `${SubAgents}`, `${AvailableTools}`.
- **v0.44.0 — 2026-05-27**: Context Manager Simplification completed. PR #26950 (`fix(core): made context files append instead of replace`) confirmed that GEMINI.md content is appended (rendered into the system prompt) rather than treated as a separate context message. PR #26953 made agent registration first-wins with project > user priority. Browser agent stabilized enough to drop "experimental" from docs.
- **v0.42.x — 2026-05-12**: Auto Memory inbox flow introduced (`chore: clean up launched memory features`, PR #26941). Off by default; opt-in via `experimental.autoMemory: true`.
- **Antigravity CLI transition — 2026-06-18**: Free-tier and Google One Gemini CLI users moved to Antigravity CLI. Banner is preserved on `geminicli.com/docs/cli/system-prompt/` after this date; the Gemini CLI binary remains available for paid tiers.

## Quirks and Workarounds

- **No CLI flag for direct prompt override.** Use `GEMINI_SYSTEM_MD` for replacement and the `GEMINI.md` hierarchy (or `--include-directories` + a temp file) for append.
- **GEMINI.md is rendered into the system prompt**, not a soft context layer. The renderer concatenates context under `# Contextual Instructions (GEMINI.md)` inside a `<loaded_context>` block; sub-directory rules beat workspace root, which beats extension memory, which beats global — but context cannot override Core Mandates.
- **GEMINI_SYSTEM_MD does not stop context discovery.** Even when the override is active, `renderUserMemory` still appends the `<loaded_context>` block on top of the custom file. To stop GEMINI.md from being included, the wrapper must use a shadow HOME or a clean `--include-directories` workspace with no GEMINI.md files in scope.
- **Replacement drops dynamic content.** Skills (`${AgentSkills}`), sub-agents (`${SubAgents}`), tools (`${AvailableTools}`), and `${<toolName>_ToolName}` substitutions must be reintroduced explicitly when shipping a custom `system.md`.
- **The CLI shows a `|⌐■_■|` indicator** in the UI when `GEMINI_SYSTEM_MD` is active.
- **Export first, then edit.** `GEMINI_WRITE_SYSTEM_MD=1 gemini` writes the current built-in prompt to `./.gemini/system.md`. Make modifications from that baseline so required Core Mandates and tool protocols stay intact.
- **Plan mode is configurable, not mandatory.** Governed by `general.plan.enabled` (default `true`) and `--approval-mode=plan`. The Plan preamble appears as a sub-shell in the rendered system prompt and lists tools with `<tool>` tags.
- **`--help` is incomplete in v0.46.0.** `--policy`, `--admin-policy`, `--session-file`, `--session-id`, `--list-sessions`, `--delete-session`, `--raw-output`, `--accept-raw-output-risk`, `--screen-reader`, `--output-format`, and `--acp` are accepted by the binary but absent from `--help`. Do not rely on `--help` to detect them in scripts.
- **Sub-agents cannot recursively spawn other sub-agents**, even with the `*` tool wildcard.
- **Persisting `GEMINI_SYSTEM_MD=1`** in `./.gemini/.env` makes the override durable for the project. Set it to `0` or remove it to restore defaults.
- **Custom context filenames** (`context.fileName` in settings.json) rename the discovered context file and its `# Contextual Instructions (<filenames>)` header label; the precedence chain remains the same.
- **`tools.core` (v0.49.0)** replaces the deprecated `coreTools` setting; settings.json templates still emitting `coreTools` will silently no-op.
- **Bundled docs lag the GitHub releases.** The `docs/changelogs/latest.md` shipped with v0.46 still references v0.44.0 as the latest stable; the GitHub releases page lists v0.49.0 as the current latest stable.
- **Antigravity CLI banner** for free-tier users runs on every session unless the CLI explicitly suppresses the migration notice. The system-prompt mechanism for Antigravity CLI may differ from this topic's scope.

## Claudine Delivery Notes

- **Replace** — Write the resolved replacement prompt to a temporary Markdown file and invoke Gemini CLI with `GEMINI_SYSTEM_MD=<tmp>` set in the environment. Per-invocation, persistent-free, no mutation of user settings.json or persistent GEMINI.md files. Optional `${AgentSkills}` / `${SubAgents}` / `${AvailableTools}` substitutions preserve dynamic content.
- **Append** — Drop a Markdown file into a temporary directory and pass that directory via `--include-directories` so MemoryDiscoveryService picks it up as a runtime GEMINI.md. The directory must not contain a real `GEMINI.md` at the root or the file naming convention must be overridden via `context.fileName`. Use `GEMINI.md.<hash>.tmp` as the file name to avoid collision with the user's persistent `GEMINI.md`.
- **Export / inspect** — Run `GEMINI_WRITE_SYSTEM_MD=1 gemini` (or `GEMINI_WRITE_SYSTEM_MD=<tmp-dir>/DEFAULT.md gemini`) once to dump the built-in prompt to a file the wrapper can read.
- **Avoid persistent mutation** — Do not write to `~/.gemini/settings.json`, project `./.gemini/settings.json`, the active workspace `GEMINI.md`, or `./.gemini/.env`. A shadow HOME or a temp-directory-driven `--include-directories` keeps the user's config untouched.
- **Traps to avoid** — `GEMINI_SYSTEM_MD` does not stop GEMINI.md discovery (the renderer still appends `<loaded_context>` on top of an override). For pure replacement the wrapper should also pass a clean `--include-directories` list with no in-scope GEMINI.md files, or use a shadow HOME so MemoryDiscoveryService cannot find any user context.

## Changelog

- **2026-07-03 — refresh**
  - Split `os: all` `config_sources` records into per-OS entries (macos / linux / windows) to satisfy the `_schema.yaml` `os` enum. The previous research violated validation because `os: all` is not in the schema's allowed values.
  - Updated `last_updated` to `2026-07-03`, set `agent` to `open_code` and `model` to `minimax/MiniMax-M3`.
  - Refreshed `recent_changes` against the GitHub releases page: latest stable is **v0.49.0** (2026-06-25); the bundled docs in v0.46 are stale and still call v0.44.0 "latest".
  - Added `--policy`, `--admin-policy`, `--acp`, `--session-file`, `--session-id`, and `--output-format` to `cli_params`; verified accepted by `gemini` in v0.46.0 but absent from `gemini --help`.
  - Corrected the body: `GEMINI.md` content is rendered into the system prompt via `renderFinalShell` → `renderUserMemory` (`packages/core/src/prompts/promptProvider.ts` in the bundle), not treated as a soft context channel. The prior research's \"softer than a true system-prompt append\" framing was technically wrong — GEMINI.md is appended inside `<loaded_context>` at the bottom of the system prompt with strict sub-directory > workspace > extension > global precedence.
  - Recorded the v0.49.0 `coreTools` → `tools.core` migration as a wrapper-relevant change.
  - Added per-OS `config_sources` entries for system defaults/settings, extension sub-agents, Agent Skills, and the project `.env` so all documented surfaces are covered.
  - New quirks: many flags absent from `--help` are accepted by the binary; bundled docs lag releases.
  - New gaps: Auto Memory's exact rendering effect, the Antigravity CLI replacement, and the lack of a flag to disable GEMINI.md discovery in the same invocation.
  - `claudine_delivery` adjusted to call append strategy `shadow_home_file` (drop a runtime GEMINI.md into a temporary directory surfaced via `--include-directories`) rather than `unsupported`. Replace strategy stays `env_var_file`.

- **2026-07-02 — prior refresh** (carried in): introduced `GEMINI_SYSTEM_MD` (replace via env + path), `GEMINI_WRITE_SYSTEM_MD` (export/inspect), `context.fileName`, `agents.overrides`, sub-agent Markdown frontmatter, Agent Skills, Auto Memory, Plan Mode, the `|⌐■_■|` UI indicator, and the Antigravity CLI migration.

## Sources

- [Gemini CLI documentation](https://geminicli.com/docs/)
- [System Prompt Override (GEMINI_SYSTEM_MD)](https://geminicli.com/docs/cli/system-prompt/)
- [Provide context with GEMINI.md files](https://geminicli.com/docs/cli/gemini-md/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Subagents](https://geminicli.com/docs/core/subagents/)
- [CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/)
- [Latest stable release notes (v0.49.0)](https://geminicli.com/docs/changelogs/latest/)
- [Release notes index](https://geminicli.com/docs/changelogs/)
- [Agent Skills](https://geminicli.com/docs/cli/skills/)
- [Auto Memory](https://geminicli.com/docs/cli/auto-memory/)
- [Plan mode](https://geminicli.com/docs/cli/plan-mode/)
- [Headless mode](https://geminicli.com/docs/cli/headless/)
- [Policy engine](https://geminicli.com/docs/reference/policy-engine/)
- [Project context (GEMINI.md)](https://geminicli.com/docs/cli/gemini-md/)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI GitHub releases](https://github.com/google-gemini/gemini-cli/releases)
- [v0.49.0 release notes](https://github.com/google-gemini/gemini-cli/releases/tag/v0.49.0)
- [v0.47.0 release notes](https://github.com/google-gemini/gemini-cli/releases/tag/v0.47.0)
- [v0.46.0 release notes](https://github.com/google-gemini/gemini-cli/releases/tag/v0.46.0)
- Local inspection: `gemini 0.46.0` (`npm install -g @google/gemini-cli`), `~/.gemini/settings.json`, `~/.gemini/agents/*.md`, `~/.gemini/GEMINI.md`. Bundle source: `packages/core/src/prompts/promptProvider.ts` and `packages/core/src/agents/local-executor.ts` in `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/`.
