---
$schema: ./_schema.yaml
created: '2026-07-02'
last_updated: '2026-07-03'
agent: 'opencode'
model: 'minimax/MiniMax-M3'
docs: 'https://opencode.ai/docs'
system_prompt_docs: 'https://opencode.ai/docs/rules'
append_support: config
replace_support: agent_spec
cli_params:
  - flag: '--agent <NAME>'
    mode: modify
    value_shape: agent name
    description: 'Selects a primary or built-in agent. Custom agents with a `prompt` field use replacement semantics for slot 0 of the assembled system text; built-in `build`/`plan` use the stock provider/model prompt.'
    example: 'opencode run --agent review "Audit this PR"'
    notes: 'Accepts any agent name resolved from `opencode.json#agent.*`, `~/.config/opencode/agents/*.md`, or `.opencode/agents/*.md`. Built-in names: `build`, `plan` (primary), `general`, `explore`, `scout` (subagents), `compaction`, `title`, `summary` (hidden primary). Per issue #34721 a custom agent `prompt` drops the stock per-model coding prompt; this is the only shape currently exposed for replacing the system prompt.'
  - flag: '--model <PROVIDER/MODEL>'
    mode: other
    value_shape: provider/model string
    description: 'Selects the model. The model ID determines which embedded provider prompt file (`prompt/anthropic.txt`, `prompt/gpt.txt`, `prompt/codex.txt`, `prompt/gemini.txt`, `prompt/kimi.txt`, `prompt/trinity.txt`, `prompt/beast.txt`, or `prompt/default.txt`) is loaded via `SystemPrompt.provider(model)` in `packages/opencode/src/session/system.ts`.'
    example: 'opencode run -m anthropic/claude-sonnet-4-5 "Explain closures"'
    notes: 'Not a prompt manipulation flag, but model ID substring routes the stock prompt (`gpt-4*`/`o1*`/`o3*` → beast; `gpt*` → gpt; `gpt*codex*` → codex; `gemini-*` → gemini; `claude*` → anthropic; `kimi*` → kimi; `trinity*` → trinity; fallback → default). Header `x-opencode-session` is also set when the provider ID starts with `opencode`.'
  - flag: --pure
    mode: disable
    value_shape: boolean
    description: 'Skips external plugins loaded via the `plugin` config and `opencode plugin install`. Does not disable `AGENTS.md`, `instructions`, AGENTS, MCP servers, or skills.'
    example: 'opencode run --pure "Hello"'
    notes: 'Plugin bypass only. The system prompt layers (provider prompt, agent prompt, instructions, AGENTS.md, MCP) are unaffected.'
  - flag: --auto
    mode: modify
    value_shape: boolean
    description: 'Auto-approves permissions that are not explicitly denied (open-policy prompt-bypassing mode for unattended runs).'
    example: 'opencode run --auto "Refactor auth"'
    notes: 'Execution-policy switch, not a prompt-content switch. Same effect as the persistent auto-mode in the TUI.'
  - flag: '--prompt <TEXT>'
    mode: other
    value_shape: string
    description: 'Initial user prompt when starting the TUI (`opencode --prompt "..."`). Distinct from `--system-prompt`; feeds the first user message, not the system prompt.'
    example: 'opencode --prompt "Audit the repo"'
    notes: 'Available on `opencode` and `opencode attach`; not on `opencode run`, whose positional message args are equivalent.'
config_sources:
  - os: macos
    scope: repo
    path: 'AGENTS.md'
    mode: append
    format: markdown
    notes: 'Project-level instructions. OpenCode traverses up from the CWD (stops at the git worktree per Skills docs precedence) and loads the first AGENTS.md it finds; if none, falls back to ./CLAUDE.md for Claude-compat repos.'
  - os: linux
    scope: repo
    path: 'AGENTS.md'
    mode: append
    format: markdown
    notes: 'Linux-equivalent of the macOS repository AGENTS.md path; same discovery rules.'
  - os: windows
    scope: repo
    path: 'AGENTS.md'
    mode: append
    format: markdown
    notes: 'Windows-equivalent. Windows shell semantics only affect bash tool timeouts; AGENTS.md discovery is OS-agnostic.'
  - os: macos
    scope: user
    path: '~/.config/opencode/AGENTS.md'
    mode: append
    format: markdown
    notes: 'Global personal rules applied across every session that does not have a closer AGENTS.md. Documented as the personal-only layer; intended to be kept out of git.'
  - os: linux
    scope: user
    path: '~/.config/opencode/AGENTS.md'
    mode: append
    format: markdown
    notes: 'Linux path equivalent (XDG config home).'
  - os: windows
    scope: user
    path: '%USERPROFILE%\.config\opencode\AGENTS.md'
    mode: append
    format: markdown
    notes: 'OpenCode resolves its config dir home-relative on every OS — on Windows the literal `.config\opencode` directory under the user profile, not %APPDATA%.'
  - os: macos
    scope: user
    path: '~/.claude/CLAUDE.md'
    mode: append
    format: markdown
    notes: 'Legacy Claude-compat fallback for global rules. Loads only when ~/.config/opencode/AGENTS.md is absent; disable with OPENCODE_DISABLE_CLAUDE_CODE_PROMPT=1.'
  - os: linux
    scope: user
    path: '~/.claude/CLAUDE.md'
    mode: append
    format: markdown
    notes: 'Linux equivalent of the Claude-compat global-rules fallback.'
  - os: windows
    scope: user
    path: '%USERPROFILE%\.claude\CLAUDE.md'
    mode: append
    format: markdown
    notes: 'Windows equivalent. OPENCODE_DISABLE_CLAUDE_CODE=1 disables every Claude-compat file.'
  - os: macos
    scope: repo
    path: 'CLAUDE.md'
    mode: append
    format: markdown
    notes: 'Legacy Claude-compat project fallback used when no project AGENTS.md exists.'
  - os: linux
    scope: repo
    path: 'CLAUDE.md'
    mode: append
    format: markdown
    notes: 'Linux equivalent; same fallback rules as macOS.'
  - os: windows
    scope: repo
    path: 'CLAUDE.md'
    mode: append
    format: markdown
    notes: 'Windows equivalent; same fallback rules.'
  - os: macos
    scope: repo
    path: 'opencode.json'
    mode: append
    format: jsonc
    notes: 'Project config. The `instructions` array (file paths, globs, or remote URLs with a 5s timeout) is appended to the system text after the agent/provider prompt slot.'
  - os: linux
    scope: repo
    path: 'opencode.json'
    mode: append
    format: jsonc
    notes: 'Linux equivalent.'
  - os: windows
    scope: repo
    path: 'opencode.json'
    mode: append
    format: jsonc
    notes: 'Windows equivalent.'
  - os: macos
    scope: user
    path: '~/.config/opencode/opencode.json'
    mode: append
    format: jsonc
    notes: 'Global config: `instructions`, `agent.*` (with optional `prompt` replacement), permissions, MCP, theme, etc.'
  - os: linux
    scope: user
    path: '~/.config/opencode/opencode.json'
    mode: append
    format: jsonc
    notes: 'Linux equivalent.'
  - os: windows
    scope: user
    path: '%USERPROFILE%\.config\opencode\opencode.json'
    mode: append
    format: jsonc
    notes: 'Windows equivalent.'
  - os: macos
    scope: agent
    path: '.opencode/agents/*.md'
    mode: replace
    format: markdown
    notes: 'Project agent definitions. Frontmatter description/mode/permission fields configure behavior; the markdown body OR frontmatter `prompt` field becomes the agent system prompt (both forms accepted).'
  - os: linux
    scope: agent
    path: '.opencode/agents/*.md'
    mode: replace
    format: markdown
    notes: 'Linux equivalent.'
  - os: windows
    scope: agent
    path: '.opencode/agents/*.md'
    mode: replace
    format: markdown
    notes: 'Windows equivalent.'
  - os: macos
    scope: agent
    path: '~/.config/opencode/agents/*.md'
    mode: replace
    format: markdown
    notes: 'User-level agent definitions. Same body-or-frontmatter prompt semantics; same replacement-of-stock semantics.'
  - os: linux
    scope: agent
    path: '~/.config/opencode/agents/*.md'
    mode: replace
    format: markdown
    notes: 'Linux equivalent.'
  - os: windows
    scope: agent
    path: '%USERPROFILE%\.config\opencode\agents\*.md'
    mode: replace
    format: markdown
    notes: 'Windows equivalent.'
  - os: macos
    scope: agent
    path: 'opencode.json (agent.<name>.prompt)'
    mode: replace
    format: jsonc
    notes: 'Inline agent definition alongside other config keys. `prompt` accepts either inline text or `{file:./path}`; with `{file:...}` the path is resolved relative to the opencode.json''s directory.'
  - os: linux
    scope: agent
    path: 'opencode.json (agent.<name>.prompt)'
    mode: replace
    format: jsonc
    notes: 'Linux equivalent.'
  - os: windows
    scope: agent
    path: 'opencode.json (agent.<name>.prompt)'
    mode: replace
    format: jsonc
    notes: 'Windows equivalent.'
  - os: macos
    scope: agent
    path: '.opencode/agents/<name>/prompt.txt'
    mode: replace
    format: text
    notes: 'Optional companion file referenced from the Markdown agent''s frontmatter, or from `prompt: "{file:./prompts/x.txt}"` in opencode.json. The path is resolved relative to the opencode.json / agent file''s directory.'
  - os: linux
    scope: agent
    path: '.opencode/agents/<name>/prompt.txt'
    mode: replace
    format: text
    notes: 'Linux equivalent. The path syntax `{file:...}` is also accepted by the global opencode.json substitution engine.'
  - os: windows
    scope: agent
    path: '.opencode/agents/<name>/prompt.txt'
    mode: replace
    format: text
    notes: 'Windows equivalent.'
  - os: macos
    scope: system
    path: '/Library/Application Support/opencode/opencode.json(c)'
    mode: modify
    format: jsonc
    notes: 'Managed file-based override. Loaded with elevated precedence (only OPENCODE_CONFIG_CONTENT and managed preferences sit above it). `instruction` and `agent.*.prompt` entries apply to every user on the host.'
  - os: linux
    scope: system
    path: '/etc/opencode/opencode.json(c)'
    mode: modify
    format: jsonc
    notes: 'Linux managed config directory; same precedence rules as macOS managed path.'
  - os: windows
    scope: system
    path: '%ProgramData%\opencode\opencode.json(c)'
    mode: modify
    format: jsonc
    notes: 'Windows managed directory; requires admin write.'
  - os: macos
    scope: system
    path: '/Library/Managed Preferences/ai.opencode.managed.plist'
    mode: modify
    format: jsonc
    notes: 'macOS MDM-deployed managed preferences under the ai.opencode.managed domain. Plist keys mirror opencode.json fields. Highest priority tier, cannot be overridden by user or project config.'
env_vars:
  - name: OPENCODE_CONFIG_CONTENT
    effect: 'Inline JSON config blob applied for the running session; wins against user/project configs but loses against managed settings. Carries `instructions` (append) and `agent.*.prompt` (replace) payloads.'
    mode: modify
  - name: OPENCODE_CONFIG
    effect: 'Path to a custom opencode.json file. Loaded between global and project in the precedence chain.'
    mode: modify
  - name: OPENCODE_CONFIG_DIR
    effect: 'Path to a custom config directory scanned for `agents/`, `commands/`, `modes/`, `plugins/`. Loaded after global and `.opencode` directories.'
    mode: modify
  - name: OPENCODE_TUI_CONFIG
    effect: 'Path to a custom tui.json file.'
    mode: modify
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: 'Disables every Claude-compat surface (AGENTS.md/CLAUDE.md prompt AND .claude/skills).'
    mode: disable
  - name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
    effect: 'Disables loading ~/.claude/CLAUDE.md (project CLAUDE.md still loads via AGENTS.md fallback rules).'
    mode: disable
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: 'Disables loading .claude/skills/<name>/SKILL.md.'
    mode: disable
  - name: OPENCODE_DISABLE_AUTOCOMPACT
    effect: 'Disables automatic context compaction (does not directly alter the prompt but affects the messages that the system prompt is paired with).'
    mode: modify
  - name: OPENCODE_DISABLE_DEFAULT_PLUGINS
    effect: 'Disables default plugins shipped by OpenCode, bypassing the `experimental.chat.system.transform` hook if any default plugins register it.'
    mode: disable
  - name: OPENCODE_DISABLE_MODELS_FETCH
    effect: 'Skips the periodic `models.dev` refresh; useful for fully offline or air-gapped runs (does not alter the prompt directly).'
    mode: modify
  - name: OPENCODE_PERMISSION
    effect: 'Inline JSON permissions overlay (applies session-wide, layered with opencode.json `permission` keys).'
    mode: modify
  - name: OPENCODE_ENABLE_EXPERIMENTAL_MODELS
    effect: 'Toggles experimental models; not a prompt-payload switch.'
    mode: modify
  - name: OPENCODE_CLIENT
    effect: 'Free-form client identifier propagated into the `x-opencode-client` header; not a prompt-content switch.'
    mode: other
prompt_layers:
  - source: Stock provider/model prompt
    mode: replace
    scope:
      - builtin
    order_notes: 'Slot 0. Selected at request time from packages/opencode/src/session/system.ts via `SystemPrompt.provider(model)`. One of: PROMPT_ANTHROPIC, PROMPT_GPT, PROMPT_CODEX, PROMPT_GEMINI, PROMPT_KIMI, PROMPT_TRINITY, PROMPT_BEAST, PROMPT_DEFAULT, chosen by model ID substring. Not published verbatim.'
    notes: 'Drops out as soon as a custom agent `prompt` is set; per issue #34721 this is the current replacement semantics.'
  - source: Custom agent prompt (agent.<name>.prompt)
    mode: replace
    scope:
      - session
      - subagent
    order_notes: 'Slot 0 when defined, displacing the stock provider prompt entirely. Inline string OR `{file:path}` reference.'
    notes: 'Markdown agents use the file body as the prompt. The replacement only covers slot 0; everything below still appends.'
  - source: opencode.json instructions (file paths, globs, remote URLs)
    mode: append
    scope:
      - session
      - subagent
    order_notes: 'Slot 1. Documents explicitly state "All instruction files are combined with your AGENTS.md files"; in request.ts the array is folded into `input.system` and joined before the user.system block.'
    notes: 'Each entry is a file reference, not inline text. Glob expansion is supported; remote URLs fetched with a 5s timeout.'
  - source: AGENTS.md (project walk, with CLAUDE.md fallback)
    mode: append
    scope:
      - repo
      - user
    order_notes: 'Slot 1, merged with the `instructions` array. Discovered by walking up from the CWD (stops at the git worktree) and falling back to global rules in this order: project AGENTS.md → ~/.config/opencode/AGENTS.md → ~/.claude/CLAUDE.md (unless disabled).'
    notes: 'First matching per category wins. Block-level HTML comments are not stripped (unlike Claude Code).'
  - source: SystemPrompt.environment
    mode: append
    scope:
      - builtin
    order_notes: 'Slot 1, emitted from `system.ts` `environment(model)`. Identifies the running model, the working directory, the git-worktree root, the platform, the date, and any discovered project references.'
    notes: 'Recomputed per request; contains per-machine dynamic context.'
  - source: SystemPrompt.skills
    mode: append
    scope:
      - session
      - subagent
    order_notes: 'Slot 1, sourced from `system.ts` `skills(agent)`. Lists skill name + description via `<available_skills>`; full SKILL.md bodies load only on demand via the `skill` tool.'
    notes: 'Hidden entirely when `permission.skill` denies the agent''s access.'
  - source: SystemPrompt.mcp
    mode: append
    scope:
      - session
      - subagent
    order_notes: 'Slot 1, sourced from `system.ts` `mcp(agent)`. Each MCP server contributing at least one allowed tool emits a `<mcp_instructions>` block.'
    notes: 'Filtered to servers whose tools are not fully disabled by the agent''s merged permission ruleset.'
  - source: User instruction layer (input.user.system)
    mode: append
    scope:
      - session
    order_notes: 'Last slot in the v2 request prep; joined after all the slots above.'
    notes: 'Carries per-session user-level instructions supplied via the runtime session API (e.g. plugins or the SDK).'
  - source: Plugin transform hook (experimental.chat.system.transform)
    mode: modify
    scope:
      - session
      - subagent
    order_notes: 'Called after the static assembly; plugins can mutate the `system` array before it is sent to the provider. OpenAI OAuth paths promote the assembled system to `options.instructions` instead of a system message.'
    notes: 'External surface; only effective when a plugin registers the hook.'
agent_prompting:
  supported: true
  definition_surface: "Markdown files with YAML frontmatter in `~/.config/opencode/agents/`, `.opencode/agents/`, or `.opencode/agents/<name>/prompt.txt` (companion file). Equivalent JSON entries under `agent.<name>` inside any opencode.json (project, global, OPENCODE_CONFIG_CONTENT, or managed). The `prompt` field accepts inline text or `{file:path}` substitution."
  inheritance: "Stock provider prompt → if the active agent defines `prompt`, the stock prompt is dropped and the agent prompt takes slot 0. Subagents inherit the invoking agent's permission ruleset and the same prompt slot 0 (each subagent retains its own prompt if defined). Without an agent, the session resolves to the global `default_agent` (falls back to `build` if unset)."
  isolation: "Each subagent runs in its own isolated child session, mounted on its own `sessionID` and `x-session-affinity` header. Subagents cannot directly read the parent's AGENTS.md or instructions unless explicitly configured. Only the final assistant summary returns to the parent."
  limitations: "Replacement semantics on slot 0 (issue #34721); no documented additive `systemMode` field. No native `--append-system-prompt` CLI. The `prompt` field's replacement can drop the stock per-model coding instructions, observable as narration leaks, degraded tool-loop behavior, or prompt-cache churn across model switches. Plugin transforms (`experimental.chat.system.transform`) are an indirect way to add layers but require writing a plugin."
claudine_delivery:
  append_strategy: env_var_file
  replace_strategy: agent_spec
  temp_file_required: true
  argv_limit: "Not applicable; `opencode run` has no `--system-prompt` / `--append-system-prompt` flag. For append, the wrapper must inject a temporary file path into `OPENCODE_CONFIG_CONTENT.instructions`. For replace, the wrapper injects an agent definition (plus prompt file) into `OPENCODE_CONFIG_CONTENT` and passes `--agent <name>`."
  notes: "Append path: write the resolved prompt body to a temp file (e.g. `/tmp/claudine-system.md`), then export `OPENCODE_CONFIG_CONTENT='{\"instructions\":[\"<tmp>\"]}'` before running `opencode run`. Replace path: write the prompt to a temp file, export `OPENCODE_CONFIG_CONTENT='{\"agent\":{\"<name>\":{\"mode\":\"primary\",\"description\":\"...\",\"prompt\":\"{file:<tmp>}\"}}}'`, and pass `--agent <name>` to `opencode run`. Because `OPENCODE_CONFIG_CONTENT` is also used by Claudine's MCP injection and YOLO permission overlay, the existing `merge_overlay` helper in `claudine/lib/src/opencode_config.rs` should compose the three payloads under one `OPENCODE_CONFIG_CONTENT` blob rather than overwriting it. No user config (`~/.config/opencode/opencode.json` or `AGENTS.md`) is mutated."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Both layers eventually render as plain text inside a single system message joined with `\\n`. The official `AGENTS.md` examples use Markdown headers and bullets, and the built-in provider prompts are Markdown-shaped XML (`<env>`, `<available_references>`, `<mcp_instructions>`, `<available_skills>`). XML-wrapped Markdown buys no model-side advantage because OpenCode itself embeds the wider instructions as Markdown. Inline-append text and `<file>` content share the same prompt body."
recent_changes:
  - date: '2026-07-01'
    version: v1.17.13
    change: 'Latest stable release. No new CLI flags affecting the system prompt; v2 prompt-assembly code in `packages/opencode/src/session/llm/request.ts` is unchanged from #34721''s snapshot.'
    impact: 'Confirms the surface is still config/agent-driven; OpenCode is the only Claudine-wrapped provider without a `--system-prompt` (inline or file) flag.'
  - date: '2026-07-01'
    version: issue #34721
    change: 'Documented that custom agent `prompt` values use replacement semantics at slot 0 of the request assembly. Submitted a feature request for an additive `systemMode` field targeting the 2.0 branch.'
    impact: 'Closes the ambiguity about `--agent review` vs `OPENCODE_CONFIG_CONTENT`-injected agents: both lose the stock per-model coding prompt unless no `prompt` is set. Tracked in recent Claudine system-prompt research as the reason `--agent <wrapper-agent>` does not give an additive layer.'
  - date: '2026-06-24'
    version: v1.17.10
    change: "Added MCP server instructions to session context (changelog: `Added MCP server instructions to session context`). Now emitted under the `<mcp_instructions>` block by `SystemPrompt.mcp` in `system.ts`."
    impact: 'MCP servers can ship `instructions` to the model directly; this is one of slot 1''s append sources and part of every effective system prompt.'
  - date: '2026-06-05'
    version: v1.16.2
    change: "Changelog: `Sessions now persist system context updates during long-running conversations`. Subagents can be sent to the background mid-run."
    impact: 'Minor surface change: the system prompt is preserved across session resumption and compaction, so injected layers (instructions, AGENTS.md, MCP) stay attached without re-resolution.'
  - date: '2026-06-05'
    version: v1.16.0
    change: "Added skill discovery and file-based agent loading."
    impact: 'Skills and agents transitioned to filesystem discovery so `OPENCODE_CONFIG_CONTENT` no longer needs to enumerate them; the system prompt layers now load from `.opencode/skills/`, `~/.config/opencode/skills/`, `.opencode/agents/`, and `~/.config/opencode/agents/`.'
  - date: '2026-04-02'
    version: issue #20695
    change: 'Memory megathread opened by `@thdxr`. Covers cross-session memory, AGENTS.md loading, and instruction-file scope.'
    impact: 'No prompt-surface change yet, but the megathread is the umbrella for future file-driven prompt injection.'
quirks:
  - "`opencode run` exposes no `--system-prompt` or `--append-system-prompt` flag; the only CLI parameter that swaps slot 0 of the system text is `--agent <name>`."
  - "A custom agent's `prompt` **replaces** the provider/model stock prompt, not the entire effective system prompt. Slot 1 (instructions, AGENTS.md, environment, skills, MCP) still appends."
  - "The `prompt` field accepts two shapes: inline text (`\"prompt\": \"...\"`) and `{file:./path}` substitution (`\"prompt\": \"{file:./prompts/build.txt}\"`). Both produce slot-0 text. Paths resolve relative to the opencode.json / agent-file directory."
  - "AGENTS.md is discovered by walking up from the CWD until the git worktree is reached; project files closer to CWD take precedence. No per-run flag exists to skip AGENTS.md auto-discovery."
  - "The `instructions` array accepts file paths, glob patterns, and remote URLs (with a 5-second timeout). It does not accept inline strings; the wrapper must always write to a temp file first."
  - "`OPENCODE_CONFIG_CONTENT` propagates to subagent sessions, so any wrapper-injected prompt layer reaches every child session, including those spawned by hidden agents (compaction/title/summary) and the scout / explore / general subagents."
  - "`OPENCODE_DISABLE_CLAUDE_CODE=1` disables both `~/.claude/CLAUDE.md` and `.claude/skills/`. Without it, OpenCode silently treats Claude-compat directories as fallbacks."
  - "`--pure` only disables external plugins loaded via `plugin` config and `opencode plugin install`. It does not touch `AGENTS.md`, `instructions`, agent prompts, MCP, or skills."
  - "The provider prompt selection in `system.ts` is a string-substring switch on `model.api.id`; `claude-haiku-4-5` and `claude-sonnet-4-5` both return `PROMPT_ANTHROPIC`, but `claude-trinity` returns `PROMPT_TRINITY`, and any `kimi` substring returns `PROMPT_KIMI`. Renaming a model can silently swap prompt files."
  - "The `prompt:` field in `opencode.json` and Markdown agent frontmatter is parsed by config substitution. The `{file:...}` path uses the same variable-substitution engine as the rest of the config (`{env:VAR}`, `{file:path}`), so the resolved prompt file can itself contain unresolved substitutions if the file-load happens before the substitution engine runs."
  - "OpenCode does not publish any inspect/export surface for the effective built-in system prompt. `opencode debug config` shows the resolved config tree but not the assembled prompt text; `opencode export --sanitize` emits the session transcript but issue #26376 (open since 2026-05-08) still has not landed system-prompt persistence."
  - "Built-in `compaction`, `title`, and `summary` agents are hidden primaries that auto-run; custom `--agent <name>` overrides apply only when the named agent resolves. They are not a way to slot a custom prompt into compaction passes."
  - "Plugin hook `experimental.chat.system.transform` lets third-party plugins mutate the assembled `system` array before it is sent; OpenCode itself does not expose a stable public API for this. The OpenAI-Codex OAuth path promotes the system text into `options.instructions` instead of a system message (`request.ts:60`)."
gaps:
  - "No documented way to inspect or export the built-in provider prompt files (`prompt/anthropic.txt`, `prompt/gpt.txt`, etc.). The shape can be inferred from `system.ts:provider()` and the slot ordering confirmed from `request.ts:prepare()`, but the exact text is not published."
  - "Whether the assembled `system` array is delivered as a single string or as a multi-segment system message in non-OpenAI-OAuth providers is undocumented; the OpenAI OAuth path merges them into `options.instructions`."
  - "The exact order in which multiple `instructions` entries (across nested `opencode.json`s and `OPENCODE_CONFIG_CONTENT`) merge is undocumented beyond 'combined'."
  - "Whether the v2 (dev) request prep captures the same slot ordering as v1.17.13 or has any subagent-specific reorderings is not pinned in release notes; the dev-branch source matches the issue #34721 quote, but v2 is described as 'improved fallback but still replacement semantics'."
  - "The `OPENCODE_CONFIG_CONTENT` payload's maximum practical size (argv length, JSON parse time) is undocumented; long append prompts should still go through a temp file."
  - "No documented knob tells OpenCode to skip AGENTS.md discovery for a single run (no `--no-agents-md` / `--no-rules`)."
  - "Behavior when both an inline `prompt` and a `{file:...}` substitution sit in the same agent definition is not explicit; the docs show only one or the other."
  - "Whether the `default_agent` setting honors the wrapper-injected custom agent (so `--agent <name>` is even necessary) is undocumented; issue #34721's reproduction script uses `--agent <name>` explicitly."
changes:
  - '2026-07-03 refresh: switched `created` / `agent` / `model` frontmatter; expanded `cli_params` to include every documented `opencode run` flag (--agent, --model, --pure, --auto, --prompt) with verified 1.17.13 help output; expanded `config_sources` to per-OS records (macos/linux/windows); split `os: all` records that previously conflated macOS/Linux/Windows; added managed-config system paths for all three OSes (Library/Application Support/opencode, /etc/opencode, %ProgramData%\opencode) plus the macOS MDM `ai.opencode.managed` plist domain; added a long `env_vars` table that documents every `OPENCODE_*` env var affecting prompt discovery, agent config, MCP, model fetches, or the system layer per the CLI docs Environment variables section; rebuilt `prompt_layers` to mirror the v2 source-of-truth assembly in `packages/opencode/src/session/llm/request.ts` and `packages/opencode/src/session/system.ts`; updated `agent_prompting` to reflect that the agent prompt replaces only slot 0 (not the full effective system prompt) per issue #34721; updated `recent_changes` with v1.17.13, v1.17.10 (MCP server instructions to session context), v1.16.2 (system context updates persist), v1.16.0 (skill discovery and file-based agent loading), and issue #20695 (memory megathread); added quirks describing the v2 substitution engine, the `claude-haiku-4-5`-vs-`claude-sonnet-4-5` ID-switch behavior, and the OpenAI-OAuth `options.instructions` promotion; updated `claudine_delivery.append_strategy` from `unsupported` to `env_var_file` and added a notes paragraph showing how to inject the temp file via `OPENCODE_CONFIG_CONTENT.instructions` while keeping MCP/YOLO permission overlays intact; updated `format_recommendations` to recommend plain Markdown for both append and replace based on the actual provider-prompt file shape (Markdown-flavored XML, not pure XML-wrapped Markdown).'
requires_claudine_update: true
reason: "OpenCode still lacks a native `--system-prompt` / `--append-system-prompt` flag, so Claudine's current `SystemPromptSpec` for OpenCode must use `OPENCODE_CONFIG_CONTENT` for append (with a temp file) and a wrapper-named agent definition for replace. Both paths compose with the existing MCP / YOLO-permission `merge_overlay` helper, but the wrapper needs a new OpenCode delivery variant to surface a `--append-system-prompt` / `--replace-system-prompt` flag without permanently mutating user config."
---

# System Prompt Handling in OpenCode CLI

## Overview

OpenCode CLI (Anomaly) has no dedicated `--system-prompt` or `--append-system-prompt` flag in v1.17.13. The only CLI parameter that influences the assembled system text is `--agent <name>`, which selects an agent whose `prompt` field replaces the stock provider/model system prompt for slot 0 of the request assembly. Every other manipulation happens through configuration files (`opencode.json`, `AGENTS.md`, `.opencode/agents/*.md`, `.opencode/skills/*`) or the `OPENCODE_CONFIG_CONTENT` environment variable.

Source of truth for the assembly is `packages/opencode/src/session/llm/request.ts:prepare()` on `dev`, which collapses the prompt into a flat string and emits it as a `system` message:

```ts
const system = [
  [
    ...(input.agent.prompt ? [input.agent.prompt] : SystemPrompt.provider(input.model)),
    ...input.system,
    ...(input.user.system ? [input.user.system] : []),
  ]
    .filter((x) => x)
    .join("\n"),
]
```

The stock provider prompt comes from `packages/opencode/src/session/system.ts:provider(model)`, which selects one of the embedded text files by substring match on `model.api.id`:

| Model substring | Prompt file |
| :--- | :--- |
| `gpt-4*`, `o1*`, `o3*` | `prompt/beast.txt` |
| `gpt*` (incl. `codex`) | `prompt/gpt.txt` or `prompt/codex.txt` |
| `gemini-*` | `prompt/gemini.txt` |
| `claude*` | `prompt/anthropic.txt` |
| `kimi*` | `prompt/kimi.txt` |
| `trinity*` | `prompt/trinity.txt` |
| fallback | `prompt/default.txt` |

Issue [#34721](https://github.com/anomalyco/opencode/issues/34721) confirms that `agent.prompt` uses replacement semantics at slot 0; no additive `systemMode` field is shipped in v1.17.13.

## CLI Parameters

`opencode run` (`v1.17.13`) accepts these flags. The only one that directly affects slot 0 of the assembled system text is `--agent`. There is no `--system-prompt`, `--append-system-prompt`, or `--prompt-instructions` flag.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--agent <name>` | modify | Selects an agent. If the agent defines `prompt`, that text replaces the stock per-model prompt for slot 0. |
| `--model <provider/model>` | other | Selects the model; the model ID drives `SystemPrompt.provider(model)`. |
| `--pure` | disable | Skips external plugins without changing AGENTS.md / instructions / agent prompts. |
| `--auto` | modify | Auto-approves non-`deny` permissions. Does not change prompt content. |
| `--prompt <text>` | other | Initial user prompt (TUI entrypoint only); feeds the first user message, not the system prompt. |

## Configuration and Discovery

The effective system text is assembled from several config-driven sources.

### `AGENTS.md` discovery

OpenCode walks up from the launch CWD (stopping at the git worktree) looking for the first `AGENTS.md` it finds; if none exists, it falls back to a `CLAUDE.md` for Claude-compat repos. The global layer is `~/.config/opencode/AGENTS.md`, and the Claude-compat global layer is `~/.claude/CLAUDE.md`. The first matching file per category wins.

The precedence is:

1. Project `AGENTS.md` (or `CLAUDE.md` if no `AGENTS.md`)
2. Global `~/.config/opencode/AGENTS.md`
3. Claude-compat `~/.claude/CLAUDE.md` (unless disabled)

### `opencode.json` `instructions` array

OpenCode config supports an `instructions` array of file paths and glob patterns (and remote URLs with a 5s timeout):

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "instructions": ["CONTRIBUTING.md", "docs/guidelines.md", ".cursor/rules/*.md"]
}
```

All paths, globs, and resolved URLs are concatenated with the AGENTS.md content in slot 1 of the system text. Values must be file references; there is no inline-string form.

### Agent definitions

Agents are defined two ways. The frontmatter `prompt` field (or the file body for Markdown agents) becomes the agent's system prompt. Both inline text and `{file:path}` substitution are accepted.

JSON shape (`opencode.json`):

```jsonc
{
  "agent": {
    "review": {
      "mode": "subagent",
      "description": "Reviews code",
      "prompt": "{file:./prompts/review.txt}"
    }
  }
}
```

Markdown shape (`~/.config/opencode/agents/review.md` or `.opencode/agents/review.md`):

```markdown
---
description: Reviews code
mode: subagent
---
You are a code reviewer. Focus on security, performance, and maintainability.
```

Per docs, primary agents use `mode: primary` and can be selected via `--agent <name>` (the `Tab` key cycles between them in the TUI). Subagents (`mode: subagent`) are invoked via `@<name>` or the `task` tool. `default_agent` setting chooses the primary when none is specified; falls back to `build` if unset.

### `OPENCODE_CONFIG_CONTENT`

`OPENCODE_CONFIG_CONTENT` is an inline JSON blob applied for the running session. It wins against global and project configs but loses against managed settings. It carries the same keys as `opencode.json`, including `instructions` (append) and `agent.<name>.prompt` (replace for slot 0). Because it propagates to subagent sessions, a wrapper-injected layer is visible to every child session that starts from the same run.

### Claude compatibility

OpenCode reads `CLAUDE.md` and `.claude/skills/` by default. Three env vars gate it:

- `OPENCODE_DISABLE_CLAUDE_CODE=1` — disable both prompt and skills.
- `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT=1` — disable `~/.claude/CLAUDE.md` only.
- `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1` — disable `.claude/skills/` only.

### Managed / system configs

| OS | Path |
| :--- | :--- |
| macOS | `/Library/Application Support/opencode/opencode.json(.c)` |
| Linux | `/etc/opencode/opencode.json(.c)` |
| Windows | `%ProgramData%\opencode\opencode.json(.c)` |

macOS MDM-deployed preferences under the `ai.opencode.managed` preference domain (`/Library/Managed Preferences/ai.opencode.managed.plist`) sit at the highest priority tier and cannot be overridden by user or project config.

### Configuration precedence

The official docs give this top-down order (later sources override earlier ones only for conflicting keys):

1. Remote config (`.well-known/opencode`)
2. Global config (`~/.config/opencode/opencode.json`)
3. Custom config (`OPENCODE_CONFIG`)
4. Project config (`opencode.json` in the project root)
5. `.opencode/` directories
6. Inline config (`OPENCODE_CONFIG_CONTENT`)
7. Managed config files (`/Library/Application Support/opencode/` on macOS, `/etc/opencode/` on Linux, `%ProgramData%\opencode` on Windows)
8. macOS managed preferences (`ai.opencode.managed` plist via MDM)

Note that `OPENCODE_CONFIG_CONTENT` sits higher than managed files but lower than macOS MDM preferences, so a wrapper's overlay cannot bypass managed policy.

## Prompt Layers and Precedence

The full v2 assembly (slot 0 plus slot 1 and beyond) is:

```mermaid
graph TD
    A[Model ID → SystemPrompt.provider] --> B{Prompt file}
    B -- anthropic.txt --> P1
    B -- codex.txt --> P2
    B -- gemini.txt --> P3
    B -- gpt.txt --> P4
    B -- beast.txt --> P5
    B -- kimi.txt --> P6
    B -- trinity.txt --> P7
    B -- default.txt --> P8
    P1 & P2 & P3 & P4 & P5 & P6 & P7 & P8 --> C{Active agent has prompt?}
    C -- yes --> D[agent.prompt - replaces slots 0]
    C -- no --> A
    D --> E[input.system: opencode.json instructions + AGENTS.md walks]
    A --> E
    E --> F[input.user.system: per-session user instructions]
    F --> G[plugin hook: experimental.chat.system.transform]
    G --> H{Provider is OpenAI OAuth?}
    H -- yes --> I[options.instructions = joined text]
    H -- no --> J[system message to provider]
```

The slot ordering matches `request.ts:prepare()`. Slot 1 is composed in `system.ts` of three independent services:

- `environment(model)` — working directory, workspace root, git/worktree, platform, today's date, available project references
- `skills(agent)` — verbose `<available_skills>` listing (filterable by permission)
- `mcp(agent)` — `<mcp_instructions>` block per MCP server with at least one allowed tool

Slot 0 is the only slot that supports replacement semantics today (per issue #34721). Every other layer appends.

## Agents and Subagents

OpenCode v1.17.13 ships:

| Category | Agents |
| :--- | :--- |
| Primary | `build`, `plan` |
| Subagent | `general`, `explore`, `scout` |
| Hidden primary | `compaction`, `title`, `summary` |

Custom agents are defined per session, project, or user (Markdown or JSON) and may include:

- `mode` — `primary`, `subagent`, or `all` (default)
- `prompt` — system prompt (inline text or `{file:path}`)
- `model` — overrides the primary's model for subagents
- `temperature`, `top_p` — sampling knobs
- `steps` — max-step cap (legacy `maxSteps` deprecated)
- `permission` — per-tool allow/ask/deny (with wildcard patterns)
- `tools` — deprecated, see `permission`
- `description` — required for `@mention` discovery
- `hidden` — hide from `@` autocomplete but allow programmatic invocation
- `color` — UI color
- Anything else — passed through as provider options

Subagents run in their own isolated child sessions; their own `prompt` replaces only their slot 0. The parent's instructions/AGENTS.md/MCP/skills layers do not propagate by default — each subagent resolves its own slot 1 from its own context. Only the final assistant summary returns to the parent.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append | Pure Markdown | The official `AGENTS.md` examples use Markdown headers and bullets; the embedded `prompt/*.txt` files use Markdown-flavored XML. Plain Markdown blends cleanly. |
| Replace (agent prompt) | Pure Markdown | The replacement slot 0 is read as plain text by the LLM; XML tags add no measured benefit and risk masking sections the model already covers in its stock prompt. |

For both modes the wrapper writes the prompt to a temp file and references it via `OPENCODE_CONFIG_CONTENT` (using `{file:path}` substitution or the `instructions` array).

## Recent Changes

- **v1.17.13 (2026-07-01)** — Latest stable release. No new system-prompt CLI flags; the v2 prompt-assembly code in `packages/opencode/src/session/llm/request.ts` remains unchanged from issue #34721's snapshot.
- **Issue #34721 (2026-07-01)** — Documented that custom agent `prompt` values use replacement semantics at slot 0; requested an additive `systemMode: append` field for the 2.0 branch.
- **v1.17.10 (2026-06-24)** — Added MCP server instructions to session context (now emitted via `SystemPrompt.mcp` and the `<mcp_instructions>` block).
- **v1.16.2 (2026-06-05)** — Sessions now persist system-context updates during long-running conversations; compaction preserves the assembled layers.
- **v1.16.0 (2026-06-05)** — Added skill discovery and file-based agent loading; agent definition surfaces transitioned to filesystem discovery.
- **Issue #20695 (2026-04-02)** — Memory megathread opened by `@thdxr`. Umbrella for cross-session memory and instruction-file scope; no prompt-surface change yet.

## Quirks and Workarounds

- The dedicated CLI flag for system-prompt manipulation does not exist; wrappers must rely on `OPENCODE_CONFIG_CONTENT` (with a temp file) for append and on a wrapper-named agent for replace.
- A custom agent's `prompt` only replaces slot 0; everything below (instructions, AGENTS.md, environment, skills, MCP) is appended independently and is unaffected by the replacement.
- The `prompt` field accepts both inline strings and `{file:path}` substitution; the substitution engine is shared with the rest of the config (`{env:VAR}` and `{file:path}` resolve at config-load time).
- The provider prompt selection in `system.ts:provider()` is a substring match on `model.api.id`; renaming a model (e.g. `claude-5-fable` if not `claude-…`-prefixed) can silently route to `PROMPT_DEFAULT`.
- `OPENCODE_CONFIG_CONTENT` propagates to subagent sessions, so any injected layer reaches every child session including hidden primary agents (`compaction`, `title`, `summary`).
- `--pure` only disables external `plugin` entries; it does not disable `AGENTS.md`, `instructions`, agent prompts, MCP, or skills.
- OpenAI-OAuth paths promote the assembled system text into `options.instructions` rather than sending a system message (`request.ts:60`).
- `opencode export --sanitize` exports session data as JSON, but issue [#26376](https://github.com/anomalyco/opencode/issues/26376) (open since 2026-05-08) has not yet landed a way to persist or view the effective system prompt text.
- The `default_agent` setting chooses the primary agent when none is named; setting it globally only affects subsequent sessions and does not bypass a wrapper's `--agent <name>`.
- Plugin hook `experimental.chat.system.transform` is the only documented way to mutate the assembled system array at runtime; OpenCode does not expose a stable public API for it.
- macOS MDM preferences under `ai.opencode.managed` sit at the highest priority tier, so a wrapper cannot override managed `instruction` or `agent.*.prompt` entries by setting `OPENCODE_CONFIG_CONTENT`.

## Claudine Delivery Notes

OpenCode's lack of a native `--system-prompt` flag means Claudine's existing `Unsupported` classification for append is updated to `env_var_file`, and replace remains an agent-spec delivery.

**Append** — write the resolved prompt body to a temp file (for example, `/tmp/claudine-system.md`) and export:

```sh
OPENCODE_CONFIG_CONTENT='{"instructions":["/tmp/claudine-system.md"]}' \
  opencode run "Refactor auth"
```

For large prompts, increase reliability by also passing an absolute path. Because `OPENCODE_CONFIG_CONTENT` is also used for MCP injection and YOLO-permission overlays, the wrapper's `merge_overlay` helper in `claudine/lib/src/opencode_config.rs` should compose the three payloads under one `OPENCODE_CONFIG_CONTENT` blob (deep-merge `instructions`, `mcp`, `permission`, etc.) rather than overwriting the variable.

**Replace** — write the prompt body to a temp file, then define an agent:

```sh
OPENCODE_CONFIG_CONTENT='{"agent":{"claudine":{"mode":"primary","description":"Claudine wrapper agent","prompt":"{file:/tmp/claudine-system.md}"}}}' \
  opencode run --agent claudine "Refactor auth"
```

The temporary file lives only for the duration of the spawned process; no `~/.config/opencode/opencode.json` or `AGENTS.md` is touched. Because `OPENCODE_CONFIG_CONTENT` sits below macOS MDM-managed preferences, wrapper delivery cannot bypass managed policy.

## Changelog

- **2026-07-03 curation edit** — Rewrote the four `os: windows` `config_sources` paths in Windows form (`%USERPROFILE%\.config\opencode\…`, `%USERPROFILE%\.claude\CLAUDE.md`): OpenCode resolves these dirs home-relative on every OS, so the locations were correct but unix-styled; also removed a stray `%APPDATA%` reference in the notes. Cross-validated against the agent-cli topic's host-evidence records.
- **2026-07-03 refresh** — Switched `created` / `agent` / `model` frontmatter per the prompt contract. Expanded `cli_params` to every documented `opencode run` flag with verified 1.17.13 `--help` output. Expanded `config_sources` to per-OS records (macos/linux/windows) for `AGENTS.md`, `CLAUDE.md`, `opencode.json`, `agents/*.md`, and `<name>/prompt.txt`; added managed-config paths for all three OSes plus the macOS MDM `ai.opencode.managed` plist domain. Expanded `env_vars` to cover every `OPENCODE_*` env var that touches prompt discovery, agent config, MCP, model fetches, or the system layer (per the CLI docs Environment variables section). Rebuilt `prompt_layers` to mirror the v2 source-of-truth assembly in `packages/opencode/src/session/llm/request.ts:prepare()` and the three `system.ts` services (`environment`, `skills`, `mcp`). Updated `agent_prompting` so the slot-0 replacement semantics per issue #34721 are explicit (the agent's `prompt` does not replace the entire effective system prompt, only slot 0). Updated `recent_changes` with v1.17.13, issue #34721, v1.17.10 (MCP server instructions to session context), v1.16.2 (system context updates persist), v1.16.0 (skill discovery and file-based agent loading), and issue #20695 (memory megathread). Added quirks for the v2 substitution engine, the OpenAI-OAuth `options.instructions` promotion, and the substring-match model-routing in `system.ts`. Switched `claudine_delivery.append_strategy` from `unsupported` to `env_var_file` (still `agent_spec` for replace), and added concrete `OPENCODE_CONFIG_CONTENT` invocations showing the wrapper temp-file path. Updated `format_recommendations` to recommend plain Markdown for both append and replace, because the provider prompt files (`prompt/*.txt`) are themselves Markdown-flavored XML and the model handles inline Markdown cleanly.

## Sources

- [OpenCode docs homepage](https://opencode.ai/docs)
- [OpenCode rules / AGENTS.md docs](https://opencode.ai/docs/rules)
- [OpenCode config docs](https://opencode.ai/docs/config)
- [OpenCode agents docs](https://opencode.ai/docs/agents)
- [OpenCode CLI docs](https://opencode.ai/docs/cli)
- [OpenCode skills docs](https://opencode.ai/docs/skills)
- [OpenCode permissions docs](https://opencode.ai/docs/permissions)
- [OpenCode changelog](https://opencode.ai/changelog)
- [`packages/opencode/src/session/system.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/system.ts)
- [`packages/opencode/src/session/llm/request.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/session/llm/request.ts)
- [GitHub issue #34721 — agent: support additive custom system prompts](https://github.com/anomalyco/opencode/issues/34721)
- [GitHub issue #34341 — Load AGENTS.md progressively via read-tool plugin context](https://github.com/anomalyco/opencode/issues/34341)
- [GitHub issue #26376 — Save dynamically generated system prompt to opencode.db](https://github.com/anomalyco/opencode/issues/26376)
- [GitHub issue #20695 — Memory Megathread](https://github.com/anomalyco/opencode/issues/20695)
- [OpenCode repository](https://github.com/anomalyco/opencode)
