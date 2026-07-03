---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: minimax/MiniMax-M3
docs: https://github.com/earendil-works/pi/tree/main/packages/coding-agent
system_prompt_docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts
append_support: native
replace_support: native
cli_params:
  - flag: --system-prompt <text-or-file>
    mode: replace
    value_shape: "string | file path"
    description: "Replaces the default system prompt for the current invocation. If the value resolves to an existing file path, the file contents are loaded; otherwise it is used as inline text."
    example: "pi --system-prompt ./prompts/researcher.md -p \"Summarize the repo\""
    notes: "Precedence: --system-prompt > .pi/SYSTEM.md (project, if trusted) > ~/.pi/agent/SYSTEM.md (global) > built-in default. When set, AGENTS.md / CLAUDE.md context files are still appended under <project_context> along with skills and date/cwd."
  - flag: --append-system-prompt <text-or-file>
    mode: append
    value_shape: "string | file path (repeatable)"
    description: "Appends text or file contents to the default system prompt. May be passed multiple times; each occurrence accumulates in array order."
    example: "pi --append-system-prompt ./prompts/persona.md --append-system-prompt \"Always answer in English.\""
    notes: "Repeats are appended in array order. Each value is independently resolved as a file when it exists on disk. Always appended AFTER the built-in default, so it cannot override the 'You are an expert coding assistant' identity (see issue #6127)."
  - flag: --no-context-files / -nc
    mode: disable
    value_shape: "boolean"
    description: "Disables discovery and loading of AGENTS.md and CLAUDE.md context files for this invocation."
    example: "pi -nc -p \"Quick question\""
    notes: "Does not disable SYSTEM.md or APPEND_SYSTEM.md resolution; those are still loaded if --system-prompt / --append-system-prompt are unset."
  - flag: --no-skills / -ns
    mode: disable
    value_shape: "boolean"
    description: "Disables skill discovery and loading. The <available_skills> block is omitted from the system prompt."
    example: "pi -ns -p \"Review this PR\""
    notes: "Explicit --skill paths still load. Skills only render in the prompt when the read tool is active (see Quirks)."
  - flag: --no-extensions / -ne
    mode: disable
    value_shape: "boolean"
    description: "Disables extension auto-discovery. Explicit -e paths still load."
    example: "pi -ne -e ./scratch.ts -p \"Test\""
    notes: "Affects all extension surfaces including before_agent_start system-prompt overrides and getSystemPromptOptions. Useful for isolating whether an extension is rewriting the effective prompt."
  - flag: --approve / -a
    mode: modify
    value_shape: "boolean"
    description: "Trusts project-local files for this run. Required in non-interactive modes to load .pi/SYSTEM.md, .pi/APPEND_SYSTEM.md, and project .pi/AGENTS.md."
    example: "pi -a -p \"Audit\""
    notes: "Without this in -p / --mode json / --mode rpc runs, the defaultProjectTrust setting ('ask' | 'always' | 'never') controls whether project-local resources are loaded."
  - flag: --no-approve / -na
    mode: modify
    value_shape: "boolean"
    description: "Ignores project-local files for this run. Suppresses .pi/SYSTEM.md, .pi/APPEND_SYSTEM.md, project skills, and project AGENTS.md even when trust is otherwise granted."
    example: "pi -na -p \"Run from user config only\""
    notes: "Mirrors the 'never' defaultProjectTrust behavior at the CLI layer."
config_sources:
  - os: macos
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Global replacement file on macOS. Loaded only when no --system-prompt CLI flag is given and no project .pi/SYSTEM.md exists."
  - os: linux
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Global replacement file on Linux. Loaded only when no --system-prompt CLI flag is given and no project .pi/SYSTEM.md exists."
  - os: windows
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Global replacement file on Windows. Loaded only when no --system-prompt CLI flag is given and no project .pi/SYSTEM.md exists."
  - os: macos
    scope: repo
    path: "./.pi/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Project replacement file (macOS). Loaded only when the project is trusted (--approve, saved trust, or defaultProjectTrust=always). Project path takes precedence over global."
  - os: linux
    scope: repo
    path: "./.pi/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Project replacement file (Linux). Loaded only when the project is trusted. Project path takes precedence over global."
  - os: windows
    scope: repo
    path: "./.pi/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Project replacement file (Windows). Loaded only when the project is trusted. Project path takes precedence over global."
  - os: macos
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Global append file on macOS. Used only when --append-system-prompt is unset and no project .pi/APPEND_SYSTEM.md exists."
  - os: linux
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Global append file on Linux. Used only when --append-system-prompt is unset and no project .pi/APPEND_SYSTEM.md exists."
  - os: windows
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Global append file on Windows. Used only when --append-system-prompt is unset and no project .pi/APPEND_SYSTEM.md exists."
  - os: macos
    scope: repo
    path: "./.pi/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Project append file (macOS). Trusted projects only. Project path takes precedence over global."
  - os: linux
    scope: repo
    path: "./.pi/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Project append file (Linux). Trusted projects only. Project path takes precedence over global."
  - os: windows
    scope: repo
    path: "./.pi/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Project append file (Windows). Trusted projects only. Project path takes precedence over global."
  - os: macos
    scope: user
    path: "~/.pi/agent/AGENTS.md"
    mode: append
    format: markdown
    notes: "First context file loaded on macOS. One file is chosen per directory by case-insensitive filename match against AGENTS.md / AGENTS.MD / CLAUDE.md / CLAUDE.MD."
  - os: linux
    scope: user
    path: "~/.pi/agent/AGENTS.md"
    mode: append
    format: markdown
    notes: "First context file loaded on Linux. One file is chosen per directory by case-insensitive filename match against AGENTS.md / AGENTS.MD / CLAUDE.md / CLAUDE.MD."
  - os: windows
    scope: user
    path: "~/.pi/agent/AGENTS.md"
    mode: append
    format: markdown
    notes: "First context file loaded on Windows. One file is chosen per directory by case-insensitive filename match against AGENTS.md / AGENTS.MD / CLAUDE.md / CLAUDE.MD."
  - os: macos
    scope: repo
    path: "./AGENTS.md (cwd and ancestor directories up to filesystem root)"
    mode: append
    format: markdown
    notes: "Project context files on macOS. Walked parent-first from cwd to filesystem root, then concatenated into <project_context> with each file as <project_instructions path=\"...\">. Disabled by --no-context-files / -nc."
  - os: linux
    scope: repo
    path: "./AGENTS.md (cwd and ancestor directories up to filesystem root)"
    mode: append
    format: markdown
    notes: "Project context files on Linux. Walked parent-first from cwd to filesystem root, then concatenated into <project_context> with each file as <project_instructions path=\"...\">. Disabled by --no-context-files / -nc."
  - os: windows
    scope: repo
    path: ".\\AGENTS.md (cwd and ancestor directories up to filesystem root)"
    mode: append
    format: markdown
    notes: "Project context files on Windows. Walked parent-first from cwd to filesystem root, then concatenated into <project_context> with each file as <project_instructions path=\"...\">. Disabled by --no-context-files / -nc."
  - os: macos
    scope: user
    path: "~/.pi/agent/skills/ and ~/.agents/skills/"
    mode: append
    format: markdown
    notes: "Skills on macOS are summarized into <available_skills> XML blocks per the Agent Skills standard; only metadata (name, description, location) is in the prompt unless the model uses /skill:name or reads SKILL.md on demand."
  - os: linux
    scope: user
    path: "~/.pi/agent/skills/ and ~/.agents/skills/"
    mode: append
    format: markdown
    notes: "Skills on Linux are summarized into <available_skills> XML blocks per the Agent Skills standard; only metadata (name, description, location) is in the prompt unless the model uses /skill:name or reads SKILL.md on demand."
  - os: windows
    scope: user
    path: "~/.pi/agent/skills/ and ~/.agents/skills/"
    mode: append
    format: markdown
    notes: "Skills on Windows are summarized into <available_skills> XML blocks per the Agent Skills standard; only metadata (name, description, location) is in the prompt unless the model uses /skill:name or reads SKILL.md on demand."
  - os: macos
    scope: repo
    path: "./.pi/skills/ and ./.agents/skills/ (cwd up to git root)"
    mode: append
    format: markdown
    notes: "Project skills on macOS (trusted projects only). .agents/skills/ in ancestor directories also picked up. Same <available_skills> XML emission."
  - os: linux
    scope: repo
    path: "./.pi/skills/ and ./.agents/skills/ (cwd up to git root)"
    mode: append
    format: markdown
    notes: "Project skills on Linux (trusted projects only). .agents/skills/ in ancestor directories also picked up. Same <available_skills> XML emission."
  - os: windows
    scope: repo
    path: ".\\.pi\\skills\\ and .\\.agents\\skills\\ (cwd up to git root)"
    mode: append
    format: markdown
    notes: "Project skills on Windows (trusted projects only). .agents/skills/ in ancestor directories also picked up. Same <available_skills> XML emission."
  - os: macos
    scope: extension
    path: "~/.pi/agent/extensions/*.ts and ./.pi/extensions/*.ts"
    mode: modify
    format: other
    notes: "TypeScript extension modules on macOS. Use pi.on('before_agent_start', ...) to rewrite event.systemPrompt or append; later handlers see earlier mutations. Use ctx.getSystemPrompt() / ctx.getSystemPromptOptions() to inspect."
  - os: linux
    scope: extension
    path: "~/.pi/agent/extensions/*.ts and ./.pi/extensions/*.ts"
    mode: modify
    format: other
    notes: "TypeScript extension modules on Linux. Use pi.on('before_agent_start', ...) to rewrite event.systemPrompt or append; later handlers see earlier mutations. Use ctx.getSystemPrompt() / ctx.getSystemPromptOptions() to inspect."
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.pi\\agent\\extensions\\*.ts and .\\.pi\\extensions\\*.ts"
    mode: modify
    format: other
    notes: "TypeScript extension modules on Windows. Use pi.on('before_agent_start', ...) to rewrite event.systemPrompt or append; later handlers see earlier mutations. Use ctx.getSystemPrompt() / ctx.getSystemPromptOptions() to inspect."
env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: Overrides the config root (default ~/.pi/agent). Changes where global SYSTEM.md, APPEND_SYSTEM.md, AGENTS.md, skills, extensions, and themes are discovered.
    mode: other
  - name: PI_PACKAGE_DIR
    effect: Overrides the package directory. Useful for Nix/Guix where store paths tokenize poorly.
    mode: other
  - name: PI_OFFLINE
    effect: Disables startup network operations (update check, package update check, install telemetry). Indirectly reduces noise during wrapper runs.
    mode: disable
  - name: PI_SKIP_VERSION_CHECK
    effect: Skips the pi.dev version check at startup. Faster, more deterministic startup for CI / wrappers.
    mode: disable
  - name: PI_TELEMETRY
    effect: Overrides install/update telemetry flag. Accepts 1/0/true/false/yes/no.
    mode: other
  - name: PI_CODING_AGENT
    effect: Set automatically by the CLI ('true') at process start; signals to extensions that pi-coding-agent is the host.
    mode: other
  - name: PI_CACHE_RETENTION
    effect: When 'long', requests extended prompt-cache windows (Anthropic 1h, OpenAI 24h). Affects provider cache reuse of the system prompt across sessions.
    mode: other
  - name: PI_EXPERIMENTAL
    effect: Enables experimental first-run setup (theme picker, opt-in analytics). Not a prompt-control surface.
    mode: other
  - name: VISUAL / EDITOR
    effect: External editor fallback for Ctrl+G when externalEditor is unset. Does not affect the system prompt.
    mode: other
prompt_layers:
  - source: built-in default system prompt
    mode: replace
    scope: ["builtin"]
    order_notes: "Lowest layer; replaced when --system-prompt is set or SYSTEM.md is discovered. Anchors the 'You are an expert coding assistant operating inside pi' identity."
    notes: "Includes role description, dynamic tool list, dynamic guidelines based on active tools, and Pi documentation references. Tool list is filtered to entries with toolSnippets."
  - source: --system-prompt <text|file> (CLI override)
    mode: replace
    scope: ["session"]
    order_notes: "Highest replace-precedence source. Wins over both SYSTEM.md files."
    notes: "File paths auto-resolved when they exist on disk. Does not disable later layers (project_context, skills, date, cwd)."
  - source: ./.pi/SYSTEM.md (project)
    mode: replace
    scope: ["repo"]
    order_notes: "Loaded only when the project is trusted. Wins over the global SYSTEM.md."
    notes: "Same effect as --system-prompt: replaces the built-in identity paragraph. Context files and skills still appended."
  - source: ~/.pi/agent/SYSTEM.md (global)
    mode: replace
    scope: ["user"]
    order_notes: "Used only when --system-prompt is unset and no project SYSTEM.md is discovered."
    notes: "Replaces the built-in identity paragraph for every session that does not override it."
  - source: --append-system-prompt (CLI, repeatable)
    mode: append
    scope: ["session"]
    order_notes: "Appended in argument order after the default (or after --system-prompt / SYSTEM.md) and before project_context and skills."
    notes: "Always sits AFTER the 'You are a coding assistant' identity, so identity-level changes are not honored (issue #6127). Use --system-prompt or an extension for persona work."
  - source: ./.pi/APPEND_SYSTEM.md (project)
    mode: append
    scope: ["repo"]
    order_notes: "Trusted projects only. Wins over the global APPEND_SYSTEM.md."
    notes: "Identical layering to --append-system-prompt; useful as a checked-in repo-level contract."
  - source: ~/.pi/agent/APPEND_SYSTEM.md (global)
    mode: append
    scope: ["user"]
    order_notes: "Used only when --append-system-prompt is unset and no project APPEND_SYSTEM.md is discovered."
    notes: "Machine-wide supplement to the default prompt."
  - source: "<project_context> (AGENTS.md / CLAUDE.md chain)"
    mode: append
    scope: ["user", "repo"]
    order_notes: "Loaded after the prompt body and append sections. Files are emitted as <project_instructions path=\"...\">...</project_instructions> under a single <project_context> wrapper."
    notes: "Global AGENTS.md first, then ancestor directories up to filesystem root, then cwd last. Disabled by --no-context-files / -nc."
  - source: "<available_skills> (skill metadata)"
    mode: append
    scope: ["user", "repo", "extension"]
    order_notes: "Appended after <project_context>, before date/cwd. Only emitted when the read tool is active."
    notes: "Skill bodies are NOT in the prompt; only name, description, and location. The model uses the read tool or /skill:name to load SKILL.md on demand (progressive disclosure)."
  - source: Current date and working directory footer
    mode: append
    scope: ["session"]
    order_notes: "Last block of every prompt."
    notes: "Two plain lines ('Current date: YYYY-MM-DD' and 'Current working directory: ...'). Injects machine-specific data, so prompt-cache reuse across machines requires care."
  - source: Extension before_agent_start systemPrompt rewrite
    mode: modify
    scope: ["session", "extension"]
    order_notes: "Runs after the prompt body is assembled, before agent_start. Extensions chain in load order; later handlers see earlier rewrites."
    notes: "Mutations only affect the current turn's prompt. Also available: pi.sendMessage({ customType, content }) to inject a persistent user-side context message."
agent_prompting:
  supported: true
  definition_surface: "TypeScript extension modules; can also use Skill (Markdown SKILL.md with frontmatter) registered as /skill:<name>"
  inheritance: "Extensions do not nest. There is no built-in subagent primitive; Pi explicitly omits sub-agents per its philosophy. Sub-agents can be built as extensions (see examples/extensions/subagent/) or by spawning external pi instances via tmux."
  isolation: "Extension handlers are session-scoped, not agent-scoped. A before_agent_start rewrite only mutates that turn's prompt. No recursive child agent execution exists in the core."
  limitations: "No built-in agent spec format; agents are ad-hoc extensions or external pi processes. before_agent_start changes do not persist across compaction unless re-injected. Prompt layering across multiple extensions is order-dependent and not documented as deterministic."
claudine_delivery:
  append_strategy: file_flag
  replace_strategy: file_flag
  temp_file_required: true
  argv_limit: "Node/Bun argv limits (~1 MB on Linux, 256 KB on Windows by default, larger on macOS) bound inline --append-system-prompt / --system-prompt text. Long prompts should be written to a temp file and passed as the flag value; pi auto-detects file paths and reads them."
  notes: "Both --system-prompt and --append-system-prompt accept either inline text or an existing file path (resolvePromptInput checks existsSync first). Use a temp file under ~/.claudine/tmp/ (or <repo>/.claudine/tmp/) so user config and global SYSTEM.md / APPEND_SYSTEM.md are never mutated. Set PI_CODING_AGENT_DIR to a shadow config root if you also want to suppress global AGENTS.md / skills / extensions — though that is usually unnecessary since file-flag delivery is already non-mutating. To bypass project trust gating in non-interactive modes, pass --approve or set defaultProjectTrust=always in the user's settings.json. The 'replace' path cannot add Pi's default tool guidance automatically; structure the replacement prompt accordingly (see Format Recommendations)."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Both SYSTEM.md and APPEND_SYSTEM.md are documented as plain Markdown and Pi's default prompt is Markdown. Skills and context files are wrapped in XML automatically by the framework (<project_context>, <project_instructions>, <available_skills>) per the Agent Skills standard, so wrapping hand-authored Markdown in additional XML tags adds tokens without documented benefit. For replacement, pure Markdown is sufficient unless the wrapper wants to mirror the framework's XML structure for its own rules/context/constraints sections."
recent_changes:
  - date: "2026-06-30"
    version: "0.80.3"
    change: "Fixed extension tool changes to apply before the next provider request in the same agent run without dropping before_agent_start system-prompt overrides (#6162)."
    impact: "Extensions can mutate the active tool set without losing per-turn before_agent_start system prompt rewrites — important when an extension swaps tools mid-session and still wants a custom prompt."
  - date: "2026-06-08"
    version: "0.79.0"
    change: "Project trust for local inputs. --approve / --no-approve gates project-local SYSTEM.md, APPEND_SYSTEM.md, AGENTS.md, skills, and packages in non-interactive modes."
    impact: "Wrappers that rely on .pi/SYSTEM.md or project AGENTS.md must either pass --approve, pre-populate ~/.pi/agent/trust.json, or set defaultProjectTrust=always in the user config."
  - date: "2026-06-04"
    version: "0.78.1"
    change: "Added ctx.getSystemPromptOptions() and ctx.mode for extensions; richer system-prompt inspection and mode-aware behaviors."
    impact: "Extensions can now read the structured system-prompt inputs (customPrompt, selectedTools, promptGuidelines, appendSystemPrompt, contextFiles, skills) instead of just the rendered string, enabling smarter system-prompt surgery."
  - date: "2026-06-27"
    version: "0.80.x"
    change: "Closed issue #6127 confirming --append-system-prompt cannot override the default coding-agent identity; --system-prompt is required for a custom persona."
    impact: "Documented as a deliberate limitation. Wrappers that need a non-coding persona must use --system-prompt (replacement) plus an extension that re-supplies tool guidance, OR a before_agent_start rewrite."
quirks:
  - "resolvePromptInput checks existsSync before treating the value as text. So --append-system-prompt / --system-prompt doubles as either an inline-text flag or a file-flag — no separate --append-system-prompt-file / --system-prompt-file is provided."
  - "AGENTS.md and CLAUDE.md are equivalent (case-insensitive); the loader picks the first match per directory in the order AGENTS.md / AGENTS.MD / CLAUDE.md / CLAUDE.MD."
  - "Context file walking goes global -> ancestors (parent-first) -> cwd last, so the most specific instructions appear last and have natural priority by ordering."
  - "Skills only appear in the system prompt if the `read` tool is active. --no-tools or a --tools allowlist that excludes read will silently drop <available_skills>."
  - "Skills with disable-model-invocation: true are excluded from the prompt (visible only via /skill:name); they still exist on disk."
  - "Project-local resources (.pi/SYSTEM.md, .pi/APPEND_SYSTEM.md, .pi/skills, .pi/AGENTS.md, .pi/extensions) require trust. In interactive mode pi prompts; in -p / --mode json / --mode rpc the defaultProjectTrust setting decides unless --approve / --no-approve is passed."
  - "Hostname / working-directory / current-date are injected last. These break prompt-cache reuse across machines; use PI_CACHE_RETENTION=long only when the same prompt must be reused across sessions."
  - "Compaction summarization uses its own internal system prompt ('Use neutral AI assistant wording for non-coding agents', per 0.79.0 #5401), separate from the user's prompt — not directly user-overridable without an extension."
  - "The default system prompt references pi's own documentation paths (README, docs/, examples/) so the model can self-serve pi-internals questions; this content cannot be redacted via --append / --system-prompt without a replacement."
  - "--append-system-prompt is repeatable; values accumulate in array order. Useful for layering persona + per-task guardrails + audit policy from multiple sources."
  - "before_agent_start extensions see `event.systemPromptOptions` (structured inputs) and `event.systemPrompt` (the rendered string). Both chain across extensions; later handlers see earlier rewrites."
  - "before_provider_request is the LAST chance to rewrite the system message that goes to the provider, but its payload-level changes are NOT reflected by ctx.getSystemPrompt() — useful for cache key shaping only."
  - "No `/inspect-prompt`, no `--show-system-prompt`, no dedicated export for the effective prompt. Inspection is via extensions (ctx.getSystemPrompt / ctx.getSystemPromptOptions) or by intercepting the prompt in a debugger."
  - "Default thinking level, default model, and default provider are NOT in the system prompt; they are independent settings. Changing them does not require --append-system-prompt."
gaps:
  - "The full default system-prompt text is not published as documentation; only the build code in src/core/system-prompt.ts reveals it. Token count and exact section ordering can drift across releases."
  - "No CLI flag or command dumps the effective resolved prompt for inspection. Wrappers that need verification must rely on extensions or by reading the session's exported HTML (no first-party `pi --show-system-prompt` exists)."
  - "The order of multi-extension before_agent_start rewrites is load order, which depends on settings.json `extensions` array and `packages` resolution. Not officially documented as deterministic; package updates could re-order."
  - "Hostname/cwd injection at the bottom of the prompt is documented but the exact placement of pi documentation references relative to date/cwd may shift across releases; documented behavior covers only the function-level structure."
  - "No documented official way to inspect AGENTS.md discovery order or to disable individual files (e.g. exclude a parent directory's CLAUDE.md while keeping the cwd one)."
  - "PI_CODING_AGENT_DIR is environment-only; no CLI equivalent for one-off overrides."
changes: []
requires_claudine_update: false
reason: "Pi's --system-prompt and --append-system-prompt flags already accept inline text or an existing file path, and that file path auto-loads the file's contents. Claudine's existing file-flag delivery maps cleanly to both modes without needing a new wrapper strategy. The existing 'append_system_prompt = file_flag, replace_system_prompt = file_flag' shape in claudine_delivery aligns with Pi's native behavior."
---

# Pi System Prompt Research

## Overview

Pi is a minimal, TypeScript-first coding-agent harness from Earendil Inc. ([pi.dev](https://pi.dev/), repo [earendil-works/pi](https://github.com/earendil-works/pi)). Unlike Claude Code or Gemini, Pi ships an aggressively minimal default prompt and treats the system prompt as a programmable surface rather than a sealed artifact — extensions can rewrite it per-turn, and `SYSTEM.md` / `APPEND_SYSTEM.md` files in the global and project config roots provide first-class file-based replacement and append without a custom agent spec.

The package version verified for this research is `0.80.3` (released 2026-06-30). The CLI binary is `pi` (npm: `@earendil-works/pi-coding-agent`); the four execution modes are interactive (TUI), print (`-p`), JSON (`--mode json`), and RPC (`--mode rpc`). The system prompt is constructed in `src/core/system-prompt.ts` (`buildSystemPrompt`) and is layered, in order: an optional replacement string, the default coding-agent paragraph, an optional append string, an XML `<project_context>` block of `AGENTS.md`/`CLAUDE.md` files, an XML `<available_skills>` block, and finally `Current date` / `Current working directory` lines.

Pi's central distinction for Claudine is that **file-flag delivery is already native**: `--system-prompt` and `--append-system-prompt` both auto-detect when their value points to an existing file and read its contents (see `resolvePromptInput` in `src/core/resource-loader.ts`). Claudine does not need to invent a separate `--*-file` flag.

## CLI Parameters

The Pi CLI exposes two flags that directly control the system prompt and a handful of related ones that affect adjacent layers.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--system-prompt <text-or-file>` | Replace | Replaces the default system prompt for the invocation. |
| `--append-system-prompt <text-or-file>` | Append | Appends to the default system prompt. Repeatable; values accumulate in array order. |
| `--no-context-files` / `-nc` | Disable | Skips `AGENTS.md` / `CLAUDE.md` discovery and loading. |
| `--no-skills` / `-ns` | Disable | Skips skill discovery and the `<available_skills>` block. |
| `--no-extensions` / `-ne` | Disable | Skips extension discovery (explicit `-e` paths still load). |
| `--approve` / `-a` | Modify | Trusts project-local files for this run; required in `-p`/`json`/`rpc` modes to load `.pi/SYSTEM.md`, `.pi/APPEND_SYSTEM.md`, `.pi/AGENTS.md`, project skills. |
| `--no-approve` / `-na` | Modify | Ignores project-local files for this run regardless of saved trust. |

Key behaviors to keep in mind:

- Both `--system-prompt` and `--append-system-prompt` accept either an inline string or an existing file path. The resource loader checks `existsSync(value)` first; on hit it reads the file with `readFileSync(value, "utf-8")`, on miss it uses the value as text (and warns if reading fails).
- Replacement does not disable downstream layers. AGENTS.md/CLAUDE.md context, skills, and the date/cwd footer are still appended after `--system-prompt` (see `buildSystemPrompt`).
- Appending cannot override the "You are an expert coding assistant" identity line — that sits before any appended text and wins by ordering (issue #6127).
- The CLI also exposes `--print`/`-p`, `--mode json`, `--mode rpc`, and `--tools`/`--no-tools`. Tool selection changes the system prompt indirectly because the `<available_skills>` block only renders when `read` is active.

## Configuration and Discovery

Pi's prompt is shaped by five files and one directory tree, in this lookup order:

1. **`--system-prompt`** (CLI value, file or text). If absent, falls through.
2. **`.pi/SYSTEM.md`** in the project root (trusted projects only).
3. **`~/.pi/agent/SYSTEM.md`** (global).
4. **Built-in default prompt** (`buildSystemPrompt` in `src/core/system-prompt.ts`). Anchors the coding-agent identity.
5. **`--append-system-prompt`** values (CLI, repeatable, file or text).
6. **`.pi/APPEND_SYSTEM.md`** (project, trusted).
7. **`~/.pi/agent/APPEND_SYSTEM.md`** (global).
8. **`<project_context>`** from `AGENTS.md` / `CLAUDE.md` files (case-insensitive match per directory).
9. **`<available_skills>`** from `~/.pi/agent/skills/`, `~/.agents/skills/`, `.pi/skills/`, `.agents/skills/` (cwd and ancestors).
10. **Current date** and **current working directory** (always last).

The AGENTS.md walker in `loadProjectContextFiles` reads:

1. `~/.pi/agent/AGENTS.md` (or `AGENTS.MD`, `CLAUDE.md`, `CLAUDE.MD`) once.
2. Then walks from `cwd` up to filesystem root, prepending each directory's first-matching file to the list.
3. The cwd file ends up last in the emitted `<project_context>` block, so the most specific instructions win by ordering.

File-based replacement and append are completely opt-in. With nothing discovered, the built-in default prompt is used unmodified, plus AGENTS.md context and skills.

### Project trust

Project-local resources (anything under `./.pi/`) require trust. Interactive mode prompts via `project_trust`; non-interactive modes consult `defaultProjectTrust` (`"ask"` / `"always"` / `"never"`) in `~/.pi/agent/settings.json`. `--approve` / `-a` is the one-shot override.

### Default prompt structure

The built-in default (from `buildSystemPrompt`) reads roughly:

```
You are an expert coding assistant operating inside pi, a coding agent harness. You help users by reading files, executing commands, editing code, and writing new files.

Available tools:
- read: <snippet>
- bash: <snippet>
- edit: <snippet>
- write: <snippet>

In addition to the tools above, you may have access to other custom tools depending on the project.

Guidelines:
- <dynamic guidelines based on active tools>
- Be concise in your responses
- Show file paths clearly when working with files

Pi documentation (read only when the user asks about pi itself, its SDK, extensions, themes, skills, or TUI):
- Main documentation: <abs path>
- Additional docs: <abs path>
- Examples: <abs path>
...
```

This is followed by the append section, the `<project_context>` block, the `<available_skills>` block, and the date/cwd footer.

## Prompt Layers and Precedence

```mermaid
graph TD
    A[Built-in default prompt] --> B{--system-prompt?}
    B -- yes --> C[--system-prompt text/file]
    B -- no --> D{.pi/SYSTEM.md trusted?}
    D -- yes --> E[.pi/SYSTEM.md]
    D -- no --> F{~/.pi/agent/SYSTEM.md?}
    F -- yes --> G[~/.pi/agent/SYSTEM.md]
    F -- no --> A
    C --> H[--append-system-prompt repeatable]
    E --> H
    G --> H
    A --> H
    H --> I{APPEND file?}
    I -- yes project --> J[.pi/APPEND_SYSTEM.md]
    I -- yes global --> K[~/.pi/agent/APPEND_SYSTEM.md]
    J --> L["<project_context>"]
    K --> L
    L --> M["<available_skills> (read tool required)"]
    M --> N[Current date + cwd footer]
    N --> O[Extension before_agent_start rewrites]
    O --> P[Extension before_provider_request payload rewrite]
```

Notes:

- Replacement sources (`--system-prompt`, `SYSTEM.md`) are exclusive: whichever resolves first wins.
- Append sources are additive: CLI flag array first, then project file, then global file. All three are kept in that order before the `<project_context>` block.
- `<project_context>` and `<available_skills>` use XML tags. The default prompt body and all hand-authored `SYSTEM.md` / `APPEND_SYSTEM.md` files use plain Markdown.
- Extension `before_agent_start` runs after the entire prompt is built. Multiple extension handlers chain in load order; later handlers see earlier rewrites of `event.systemPrompt`.
- Extension `before_provider_request` is the very last chance to mutate what is actually sent over the wire; its changes are not visible to `ctx.getSystemPrompt()`.

## Agents and Subagents

Pi has **no built-in sub-agent primitive** by design — that is one of the project's "what we didn't build" decisions ([pi.dev](https://pi.dev/)). Sub-agents can be built as extensions (see [`examples/extensions/subagent/`](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions/subagent)) or by spawning external pi instances over RPC or tmux.

For Claude-Code-style agent specs (a separate system prompt per agent), Pi offers:

1. **`SYSTEM.md`** files — the most direct way to give the main thread a different persona.
2. **`APPEND_SYSTEM.md`** files — for additive customization.
3. **Skills** — SKILL.md with frontmatter; registered as `/skill:<name>` commands and summarized in the `<available_skills>` block of the prompt. A skill's full body is loaded only when the model uses `/skill:name` or reads the file on demand (progressive disclosure per the [Agent Skills standard](https://agentskills.io)).
4. **Extensions** — TypeScript modules with a `before_agent_start` handler. Each handler can mutate `event.systemPrompt` and inject persistent `pi.sendMessage({ customType, content })` context messages. Extensions also expose `ctx.getSystemPrompt()` (current rendered string) and `ctx.getSystemPromptOptions()` (structured inputs: `customPrompt`, `selectedTools`, `toolSnippets`, `promptGuidelines`, `appendSystemPrompt`, `cwd`, `contextFiles`, `skills`).

There is no `agent_spec` enum value in the sense of "separate per-agent system prompt text" — extensions *are* the agent customization layer, and they run per-turn rather than per-spawn.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append | Pure Markdown | Matches the default prompt style; no documented benefit from XML wrapping. The framework already wraps AGENTS.md in `<project_context>` automatically. |
| Replace | Pure Markdown | The framework auto-wraps downstream layers (skills, context, date/cwd) in XML; the replacement body itself should be Markdown. If you want to mirror Pi's own `<project_context>` / `<available_skills>` structure inside the replacement, you can, but it adds tokens without documented model-side benefit. |

For the **append** case, headers and bullet lists integrate cleanly. For the **replace** case, the prompt must self-supply any tool guidance you want the model to follow — Pi's default provides an explicit `Available tools:` section and a `Guidelines:` block; if you replace, replicate that structure or supply it via an extension that re-emits it in `before_agent_start`.

## Recent Changes

- **v0.80.3 (2026-06-30)** — Fixed extension tool changes to apply before the next provider request in the same agent run without dropping `before_agent_start` system-prompt overrides ([#6162](https://github.com/earendil-works/pi/issues/6162)). Important for extensions that swap tools mid-session.
- **v0.79.0 (2026-06-08)** — Added project trust for local inputs ([#5332](https://github.com/earendil-works/pi/pull/5332)). `--approve` / `--no-approve` gate project-local SYSTEM.md, APPEND_SYSTEM.md, AGENTS.md, skills, and packages in non-interactive modes.
- **v0.78.1 (2026-06-04)** — Added `ctx.getSystemPromptOptions()` and `ctx.mode` for extensions; structured system-prompt inspection ([#5306](https://github.com/earendil-works/pi/pull/5306)). Enables extensions to make informed rewrites instead of regex surgery on the rendered string.
- **2026-06-27 (issue #6127)** — Closed (Not planned) confirming `--append-system-prompt` cannot override the default coding-agent identity; `--system-prompt` or an extension is required for a custom persona. Useful as documented behavior.

## Quirks and Workarounds

- `--append-system-prompt` doubles as either an inline-text flag or a file flag because `resolvePromptInput` checks `existsSync` first. No separate `--*-file` flag exists; that is the file-flag delivery mechanism.
- `--append-system-prompt` is repeatable. Values are concatenated in array order before `<project_context>`.
- AGENTS.md / CLAUDE.md are equivalent (case-insensitive); the loader picks the first match in `AGENTS.md / AGENTS.MD / CLAUDE.md / CLAUDE.MD` order.
- Context files are emitted as `<project_instructions path="...">...</project_instructions>` under a single `<project_context>` wrapper, regardless of how many files were loaded. The cwd file is last (most-specific priority by ordering).
- Skills are wrapped in `<available_skills>` per the Agent Skills standard. Only name, description, and location are in the prompt — bodies are loaded on demand.
- Skills with `disable-model-invocation: true` are excluded from the prompt entirely (must be invoked via `/skill:name`).
- `<available_skills>` is omitted entirely if the `read` tool is not active. `--no-tools`, `--tools` allowlists without `read`, or `--no-skills` all drop the block silently.
- `--system-prompt` does NOT suppress the AGENTS.md context or skills block — those are appended after replacement.
- The default prompt ends with `Pi documentation: <abs path>` lines and a date/cwd footer. Replacing the prompt does not remove these unless you also suppress them via `--no-context-files` / `--no-skills`.
- Project-local resources (anything under `.pi/`) require trust; in non-interactive modes the default `defaultProjectTrust: "ask"` means they are silently ignored unless `--approve` is passed.
- No CLI command or flag dumps or inspects the effective resolved prompt. Inspection requires either an extension (`ctx.getSystemPrompt()` / `ctx.getSystemPromptOptions()`) or a debugger attachment.
- The default prompt anchors a coding-agent identity that survives any `--append-system-prompt`. For persona swaps, use `--system-prompt` (replacement) or a `before_agent_start` extension rewrite.
- Extension `before_agent_start` rewrites are scoped to the current turn; they do not persist across compaction unless re-injected via `pi.sendMessage({ customType, content, display: true })`.
- `before_provider_request` is a lower-level payload rewrite that does not update `ctx.getSystemPrompt()` — useful only for provider cache shaping, not for "make the model think differently".

## Claudine Delivery Notes

Claudine should map Pi onto its existing file-flag delivery model:

- **Append** — write the resolved prompt to a temp file (e.g. `<repo>/.claudine/tmp/<id>-append.md`) and invoke Pi with `pi --append-system-prompt <tmp>`. The auto-detect in `resolvePromptInput` means no `--*-file` flag is needed.
- **Replace** — same pattern with `pi --system-prompt <tmp>`. The replacement prompt must self-supply any tool guidance, or pair with a small extension that re-emits Pi's default `Available tools:` section via `before_agent_start`.
- **Multiple appends** — pass `--append-system-prompt` multiple times if the resolved prompt needs to be split (e.g. persona + per-task guardrail). Each value is independently file-or-text resolved.
- **Avoid mutating user config** — never write to `~/.pi/agent/SYSTEM.md`, `~/.pi/agent/APPEND_SYSTEM.md`, or project `.pi/SYSTEM.md`. CLI flags and temp files are sufficient. If you also want to suppress global AGENTS.md / skills, set `PI_CODING_AGENT_DIR` to a shadow config root in the wrapper — usually unnecessary, since flag delivery is already non-mutating.
- **Project trust** — in `-p` / `--mode json` / `--mode rpc` modes, either pass `--approve` to let project `.pi/SYSTEM.md` and `.pi/AGENTS.md` load, or pass `--no-approve` to skip them deterministically. Do not depend on `defaultProjectTrust: "always"` in user settings; that requires mutating user config.
- **Argv limits** — long prompts should always go through a temp file; Pi's value-as-file auto-detect makes this trivial and avoids Node/Bun argv caps on macOS/Linux/Windows.
- **Persona customization** — when the wrapper needs a non-coding identity, use `--system-prompt` (replace) plus an extension that re-supplies tool guidance, rather than fighting the built-in identity with `--append-system-prompt`. Documented as deliberate (issue #6127).

## Sources

- [Pi coding-agent README](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/README.md)
- [Pi CLI argument parser](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts) — `--system-prompt`, `--append-system-prompt`, `--no-context-files`, `--approve`
- [Pi main entry](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/main.ts) — wires CLI flags into `resourceLoaderOptions`
- [Pi system-prompt construction](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts) — `buildSystemPrompt` and default prompt text
- [Pi resource loader](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/resource-loader.ts) — `discoverSystemPromptFile`, `discoverAppendSystemPromptFile`, `loadProjectContextFiles`, `resolvePromptInput`
- [Pi skills module](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/skills.ts) — `formatSkillsForPrompt` and the Agent Skills XML format
- [Pi extensions docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md) — `before_agent_start`, `before_provider_request`, `ctx.getSystemPrompt`, `ctx.getSystemPromptOptions`
- [Pi settings docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/settings.md) — `defaultProjectTrust`, package/resource settings
- [Pi skills docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/skills.md) — skill locations, Agent Skills standard
- [Pi prompt-templates docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/prompt-templates.md) — `/name` template system
- [Pi changelog](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/CHANGELOG.md) — recent versions and notable fixes
- [Pi website](https://pi.dev/) — high-level overview and philosophy
- [Issue #6127 — `--append-system-prompt` cannot override default identity](https://github.com/earendil-works/pi/issues/6127)
- [Issue #6162 — Extension tool changes dropped `before_agent_start` system-prompt overrides](https://github.com/earendil-works/pi/issues/6162)
- [PR #5306 — `ctx.getSystemPromptOptions()` for extensions](https://github.com/earendil-works/pi/pull/5306)
- [PR #5332 — Project trust for local inputs](https://github.com/earendil-works/pi/pull/5332)
- [Agent Skills specification](https://agentskills.io/specification)