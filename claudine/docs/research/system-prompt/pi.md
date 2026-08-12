---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/usage.md
system_prompt_docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts
append_support: native
replace_support: native
cli_params:
  - flag: "--system-prompt <text>"
    mode: replace
    value_shape: "inline text or file path"
    description: "Replaces Pi's built-in coding-agent system prompt for the invocation. If the value names an existing file, Pi reads that file and uses its contents."
    example: "pi --system-prompt /tmp/claudine-pi-system.md -p \"Summarize this repo\""
    notes: "Highest replacement precedence. It replaces only the base prompt; append prompts, AGENTS.md/CLAUDE.md context files, skills, current date, and current working directory are still layered after it."
  - flag: "--append-system-prompt <text>"
    mode: append
    value_shape: "inline text or file path; repeatable"
    description: "Appends text or file contents to the system prompt for the invocation."
    example: "pi --append-system-prompt /tmp/claudine-pi-append.md --append-system-prompt \"Prefer concise answers.\" -p \"Review src\""
    notes: "Repeatable since 0.67.2. Pi joins multiple resolved append values with blank-line separation before adding later context and skill layers."
  - flag: "--no-context-files, -nc"
    mode: disable
    value_shape: "boolean switch"
    description: "Disables AGENTS.md and CLAUDE.md context-file discovery and loading."
    example: "pi --no-context-files --append-system-prompt /tmp/append.md -p \"Answer without repo instructions\""
    notes: "Does not disable SYSTEM.md or APPEND_SYSTEM.md. It only removes the context-file layer."
  - flag: "--no-skills, -ns"
    mode: disable
    value_shape: "boolean switch"
    description: "Disables automatic skill discovery and loading."
    example: "pi --no-skills -p \"No skill metadata in prompt\""
    notes: "Explicit --skill paths still load. Skills appear in the system prompt only when the read tool is active and the skill is not hidden with disable-model-invocation."
  - flag: "--skill <path>"
    mode: modify
    value_shape: "file or directory path; repeatable"
    description: "Adds skill files or directories, causing visible skill metadata to be listed in the prompt's available-skills block."
    example: "pi --skill ~/.claude/skills/review/SKILL.md -p \"Use the review skill\""
    notes: "Additive even when --no-skills is used. Full skill bodies are loaded on demand by the model via read or by /skill:name."
  - flag: "--extension <path>, -e <path>"
    mode: modify
    value_shape: "TypeScript extension file or directory path; repeatable"
    description: "Loads an extension that can mutate or inspect the prompt through before_agent_start and before_provider_request hooks."
    example: "pi -e ./prompt-customizer.ts -p \"Run with extension prompt logic\""
    notes: "Explicit CLI extensions still load when --no-extensions is set. Extensions execute with local user permissions."
  - flag: "--no-extensions, -ne"
    mode: disable
    value_shape: "boolean switch"
    description: "Disables extension discovery, while preserving explicit -e/--extension paths."
    example: "pi --no-extensions --append-system-prompt /tmp/append.md -p \"Isolate prompt behavior\""
    notes: "Useful when wrapper behavior must avoid user/global or project extension prompt rewrites."
  - flag: "--approve, -a"
    mode: modify
    value_shape: "boolean switch"
    description: "Trusts project-local resources for this run, including .pi/SYSTEM.md, .pi/APPEND_SYSTEM.md, project .pi/extensions, .pi/skills, and .agents/skills."
    example: "pi --approve -p \"Use trusted project prompt files\""
    notes: "Important for non-interactive wrapper runs because print, JSON, and RPC modes do not prompt for project trust."
  - flag: "--no-approve, -na"
    mode: disable
    value_shape: "boolean switch"
    description: "Ignores project-local trust-gated resources for this run."
    example: "pi --no-approve --append-system-prompt /tmp/append.md -p \"Ignore project .pi resources\""
    notes: "Does not disable AGENTS.md/CLAUDE.md context files; use --no-context-files for that."
  - flag: "--tools <tools>, -t <tools>"
    mode: modify
    value_shape: "comma-separated tool names"
    description: "Restricts the active tool set; the active tools control tool snippets, tool-specific prompt guidelines, and whether skills are prompt-visible."
    example: "pi --tools read,grep,find,ls -p \"Read-only review\""
    notes: "If read is absent, Pi omits the skills layer because the model cannot load skill files."
  - flag: "--exclude-tools <tools>, -xt <tools>"
    mode: modify
    value_shape: "comma-separated tool names"
    description: "Disables selected tools; this can remove tool prompt snippets, related guidelines, and skill visibility."
    example: "pi --exclude-tools bash -p \"No shell access\""
    notes: "Applies to built-in, extension, and custom tools."
  - flag: "--no-tools, -nt"
    mode: disable
    value_shape: "boolean switch"
    description: "Disables all tools by default, affecting the built-in prompt's available-tools and skill layers."
    example: "pi --no-tools -p \"Answer without tool use\""
    notes: "Extensions can still affect prompt text unless extension loading is disabled."
  - flag: "--no-builtin-tools, -nbt"
    mode: disable
    value_shape: "boolean switch"
    description: "Disables built-in tools while preserving extension/custom tools, changing the prompt's tool list and guidelines."
    example: "pi --no-builtin-tools -e ./tools.ts -p \"Use only extension tools\""
    notes: "Custom tools can add promptSnippet and promptGuidelines that feed the default system prompt."
config_sources:
  - os: macos
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Global base-prompt replacement. Used when --system-prompt is absent and no trusted project .pi/SYSTEM.md exists."
  - os: linux
    scope: user
    path: "~/.pi/agent/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Global base-prompt replacement. Used when --system-prompt is absent and no trusted project .pi/SYSTEM.md exists."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Global base-prompt replacement. Used when --system-prompt is absent and no trusted project .pi/SYSTEM.md exists."
  - os: macos
    scope: repo
    path: ".pi/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Trusted project base-prompt replacement; takes precedence over the global file."
  - os: linux
    scope: repo
    path: ".pi/SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Trusted project base-prompt replacement; takes precedence over the global file."
  - os: windows
    scope: repo
    path: ".pi\\SYSTEM.md"
    mode: replace
    format: markdown
    notes: "Trusted project base-prompt replacement; takes precedence over the global file."
  - os: macos
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Global append file. Used when --append-system-prompt is absent and no trusted project .pi/APPEND_SYSTEM.md exists."
  - os: linux
    scope: user
    path: "~/.pi/agent/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Global append file. Used when --append-system-prompt is absent and no trusted project .pi/APPEND_SYSTEM.md exists."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Global append file. Used when --append-system-prompt is absent and no trusted project .pi/APPEND_SYSTEM.md exists."
  - os: macos
    scope: repo
    path: ".pi/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Trusted project append file; takes precedence over the global append file when no append CLI flag is present."
  - os: linux
    scope: repo
    path: ".pi/APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Trusted project append file; takes precedence over the global append file when no append CLI flag is present."
  - os: windows
    scope: repo
    path: ".pi\\APPEND_SYSTEM.md"
    mode: append
    format: markdown
    notes: "Trusted project append file; takes precedence over the global append file when no append CLI flag is present."
  - os: macos
    scope: user
    path: "~/.pi/agent/AGENTS.md or ~/.pi/agent/CLAUDE.md"
    mode: append
    format: markdown
    notes: "Global context file. Loaded into the project_context layer unless --no-context-files is set."
  - os: linux
    scope: user
    path: "~/.pi/agent/AGENTS.md or ~/.pi/agent/CLAUDE.md"
    mode: append
    format: markdown
    notes: "Global context file. Loaded into the project_context layer unless --no-context-files is set."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\AGENTS.md or %USERPROFILE%\\.pi\\agent\\CLAUDE.md"
    mode: append
    format: markdown
    notes: "Global context file. Loaded into the project_context layer unless --no-context-files is set."
  - os: macos
    scope: repo
    path: "AGENTS.md, AGENTS.MD, CLAUDE.md, or CLAUDE.MD in cwd and ancestors"
    mode: append
    format: markdown
    notes: "One matching context file per directory is loaded, parent-first, into XML project_instructions blocks. Context files are not project-trust gated."
  - os: linux
    scope: repo
    path: "AGENTS.md, AGENTS.MD, CLAUDE.md, or CLAUDE.MD in cwd and ancestors"
    mode: append
    format: markdown
    notes: "One matching context file per directory is loaded, parent-first, into XML project_instructions blocks. Context files are not project-trust gated."
  - os: windows
    scope: repo
    path: "AGENTS.md, AGENTS.MD, CLAUDE.md, or CLAUDE.MD in cwd and ancestors"
    mode: append
    format: markdown
    notes: "One matching context file per directory is loaded, parent-first, into XML project_instructions blocks. Context files are not project-trust gated."
  - os: macos
    scope: user
    path: "~/.pi/agent/settings.json"
    mode: modify
    format: json
    notes: "Global settings can set defaultProjectTrust, extension paths, skill paths, prompts, themes, tool/user behavior, and related defaults. It does not directly contain a system prompt string."
  - os: linux
    scope: user
    path: "~/.pi/agent/settings.json"
    mode: modify
    format: json
    notes: "Global settings can set defaultProjectTrust, extension paths, skill paths, prompts, themes, tool/user behavior, and related defaults. It does not directly contain a system prompt string."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\settings.json"
    mode: modify
    format: json
    notes: "Global settings can set defaultProjectTrust, extension paths, skill paths, prompts, themes, tool/user behavior, and related defaults. It does not directly contain a system prompt string."
  - os: macos
    scope: repo
    path: ".pi/settings.json"
    mode: modify
    format: json
    notes: "Project settings are trust-gated and override/merge with global settings. They can enable project extensions and skills that affect the prompt."
  - os: linux
    scope: repo
    path: ".pi/settings.json"
    mode: modify
    format: json
    notes: "Project settings are trust-gated and override/merge with global settings. They can enable project extensions and skills that affect the prompt."
  - os: windows
    scope: repo
    path: ".pi\\settings.json"
    mode: modify
    format: json
    notes: "Project settings are trust-gated and override/merge with global settings. They can enable project extensions and skills that affect the prompt."
  - os: macos
    scope: extension
    path: "~/.pi/agent/extensions/*.ts and ~/.pi/agent/extensions/*/index.ts"
    mode: modify
    format: other
    notes: "Global TypeScript extensions can return a replacement systemPrompt from before_agent_start or rewrite the serialized provider payload in before_provider_request."
  - os: linux
    scope: extension
    path: "~/.pi/agent/extensions/*.ts and ~/.pi/agent/extensions/*/index.ts"
    mode: modify
    format: other
    notes: "Global TypeScript extensions can return a replacement systemPrompt from before_agent_start or rewrite the serialized provider payload in before_provider_request."
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.pi\\agent\\extensions\\*.ts and %USERPROFILE%\\.pi\\agent\\extensions\\*\\index.ts"
    mode: modify
    format: other
    notes: "Global TypeScript extensions can return a replacement systemPrompt from before_agent_start or rewrite the serialized provider payload in before_provider_request."
  - os: macos
    scope: extension
    path: ".pi/extensions/*.ts and .pi/extensions/*/index.ts"
    mode: modify
    format: other
    notes: "Project TypeScript extensions are trust-gated and can mutate or inspect prompts."
  - os: linux
    scope: extension
    path: ".pi/extensions/*.ts and .pi/extensions/*/index.ts"
    mode: modify
    format: other
    notes: "Project TypeScript extensions are trust-gated and can mutate or inspect prompts."
  - os: windows
    scope: extension
    path: ".pi\\extensions\\*.ts and .pi\\extensions\\*\\index.ts"
    mode: modify
    format: other
    notes: "Project TypeScript extensions are trust-gated and can mutate or inspect prompts."
  - os: macos
    scope: user
    path: "~/.pi/agent/skills/, ~/.agents/skills/, and configured skill paths"
    mode: append
    format: markdown
    notes: "Visible skill metadata is rendered into an XML available_skills block when read is active."
  - os: linux
    scope: user
    path: "~/.pi/agent/skills/, ~/.agents/skills/, and configured skill paths"
    mode: append
    format: markdown
    notes: "Visible skill metadata is rendered into an XML available_skills block when read is active."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\skills\\, %USERPROFILE%\\.agents\\skills\\, and configured skill paths"
    mode: append
    format: markdown
    notes: "Visible skill metadata is rendered into an XML available_skills block when read is active."
  - os: macos
    scope: repo
    path: ".pi/skills/ and .agents/skills/ in cwd and ancestors"
    mode: append
    format: markdown
    notes: "Project skills are trust-gated and affect the prompt through skill metadata."
  - os: linux
    scope: repo
    path: ".pi/skills/ and .agents/skills/ in cwd and ancestors"
    mode: append
    format: markdown
    notes: "Project skills are trust-gated and affect the prompt through skill metadata."
  - os: windows
    scope: repo
    path: ".pi\\skills\\ and .agents\\skills\\ in cwd and ancestors"
    mode: append
    format: markdown
    notes: "Project skills are trust-gated and affect the prompt through skill metadata."
  - os: macos
    scope: agent
    path: "~/.pi/agent/agents/*.md"
    mode: append
    format: markdown
    notes: "Used by the example subagent extension, not Pi core. Agent file bodies are appended to child Pi processes with --append-system-prompt."
  - os: linux
    scope: agent
    path: "~/.pi/agent/agents/*.md"
    mode: append
    format: markdown
    notes: "Used by the example subagent extension, not Pi core. Agent file bodies are appended to child Pi processes with --append-system-prompt."
  - os: windows
    scope: agent
    path: "%USERPROFILE%\\.pi\\agent\\agents\\*.md"
    mode: append
    format: markdown
    notes: "Used by the example subagent extension, not Pi core. Agent file bodies are appended to child Pi processes with --append-system-prompt."
env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: "Overrides the user agent config directory. This changes where global SYSTEM.md, APPEND_SYSTEM.md, AGENTS.md/CLAUDE.md, settings, extensions, skills, and sessions are discovered."
    mode: other
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: "Overrides session storage and lookup directory. It does not directly change prompt text but can affect resumed sessions."
    mode: other
  - name: PI_CODING_AGENT
    effect: "Set to true by Pi at CLI/RPC startup so extensions can detect the host."
    mode: other
  - name: PI_PACKAGE_DIR
    effect: "Overrides the package directory used for resolving Pi docs/examples embedded in the built-in prompt."
    mode: modify
  - name: PI_OFFLINE
    effect: "Disables startup network operations. Useful for deterministic wrapper runs; no direct system-prompt effect."
    mode: disable
  - name: PI_SKIP_VERSION_CHECK
    effect: "Disables startup version checks. No direct system-prompt effect."
    mode: disable
  - name: PI_TELEMETRY
    effect: "Controls install/update telemetry and some attribution behavior. No direct system-prompt effect."
    mode: other
  - name: PI_CACHE_RETENTION
    effect: "When set to long, requests longer provider prompt-cache retention on supported transports; affects provider caching of the system prompt, not prompt construction."
    mode: other
  - name: HOME
    effect: "Used by Node os.homedir() and Pi package-manager helpers; in wrappers, a shadow HOME or PI_CODING_AGENT_DIR can isolate global prompt/config discovery."
    mode: other
prompt_layers:
  - source: "built-in base system prompt"
    mode: replace
    scope: ["builtin"]
    order_notes: "Lowest base layer when no replacement source is present."
    notes: "Includes Pi coding-agent identity, active tool snippets, default and tool-specific guidelines, and absolute paths to Pi documentation/examples."
  - source: "--system-prompt value or file"
    mode: replace
    scope: ["session"]
    order_notes: "Overrides .pi/SYSTEM.md, ~/.pi/agent/SYSTEM.md, and the built-in base prompt."
    notes: "Auto-resolves an existing path to file contents; otherwise treats the value as inline text."
  - source: ".pi/SYSTEM.md"
    mode: replace
    scope: ["repo"]
    order_notes: "Used when the project is trusted and --system-prompt is absent; takes precedence over the global SYSTEM.md."
    notes: "Trust-gated by project trust, including --approve/--no-approve in non-interactive modes."
  - source: "~/.pi/agent/SYSTEM.md"
    mode: replace
    scope: ["user"]
    order_notes: "Used when no CLI or trusted project replacement exists."
    notes: "File content becomes customPrompt, so later layers still apply."
  - source: "--append-system-prompt values"
    mode: append
    scope: ["session"]
    order_notes: "Added immediately after the selected base prompt and before context files, skills, date, and cwd."
    notes: "Repeatable; values are resolved as files when paths exist and joined with blank-line separation."
  - source: ".pi/APPEND_SYSTEM.md"
    mode: append
    scope: ["repo"]
    order_notes: "Used only when --append-system-prompt is absent and the project is trusted; takes precedence over global APPEND_SYSTEM.md."
    notes: "Project append prompt is not combined with the global append file by the built-in loader."
  - source: "~/.pi/agent/APPEND_SYSTEM.md"
    mode: append
    scope: ["user"]
    order_notes: "Used only when no append CLI flag and no trusted project append file exist."
    notes: "One global append file is loaded."
  - source: "AGENTS.md / CLAUDE.md context files"
    mode: append
    scope: ["user", "repo"]
    order_notes: "Added after append-system-prompt text in a project_context XML block. Global file is first, then ancestor files parent-first down to cwd."
    notes: "Disabled by --no-context-files. These files are not project-trust gated."
  - source: "skills"
    mode: append
    scope: ["user", "repo"]
    order_notes: "Added after context files in an available_skills XML block."
    notes: "Only metadata is included. Hidden skills and all skills when read is unavailable are omitted."
  - source: "current date and current working directory"
    mode: append
    scope: ["session"]
    order_notes: "Always appended last by buildSystemPrompt."
    notes: "Date is formatted YYYY-MM-DD and cwd normalizes Windows backslashes to forward slashes."
  - source: "before_agent_start extension hook"
    mode: modify
    scope: ["extension"]
    order_notes: "Runs after the base prompt is built and before the agent loop. Handlers run in extension load order and see prior prompt mutations."
    notes: "Returning systemPrompt replaces the whole current prompt for that turn."
  - source: "before_provider_request extension hook"
    mode: modify
    scope: ["extension"]
    order_notes: "Runs after provider payload serialization, just before the request."
    notes: "Can rewrite or remove provider-level system instructions. ctx.getSystemPrompt does not include these payload-level rewrites."
agent_prompting:
  supported: true
  definition_surface: "No built-in core subagent API; the official example extension defines Markdown agent files under ~/.pi/agent/agents/*.md and .pi/agents/*.md."
  inheritance: "The example subagent extension launches separate pi subprocesses and appends each agent file body with --append-system-prompt, so child agents keep Pi's base prompt unless the extension is changed to use --system-prompt."
  isolation: "Each example subagent invocation is a separate Pi process with isolated context. Final output, usage, and diagnostics are returned through the parent tool result."
  limitations: "This is extension-provided behavior, not a first-class Pi core agent spec. The sample agent prompt appends to the child base prompt rather than replacing it; project-local agents require explicit agentScope and trust."
claudine_delivery:
  append_strategy: file_flag
  replace_strategy: file_flag
  temp_file_required: true
  argv_limit: "Use temporary files for wrapper-generated prompts. Inline text is supported but risks shell/argv length limits and quoting issues."
  notes: "For append, pass --append-system-prompt <tempfile>. For replace, pass --system-prompt <tempfile>. Add --no-extensions when user/global extension prompt rewrites would violate wrapper isolation. Add --no-context-files only when Claudine explicitly needs to suppress AGENTS.md/CLAUDE.md. Use PI_CODING_AGENT_DIR or a shadow HOME only when isolating global config is required; the CLI flags avoid permanent mutation."
format_recommendations:
  append_format: xml_wrapped_markdown
  replace_format: markdown
  rationale: "Pi accepts arbitrary text and its user-facing prompt files are Markdown, so replacement should be a complete Markdown system prompt. For append, plain Markdown works, but XML-wrapped Markdown is the safer wrapper format because Pi itself now uses XML boundaries for project context and skills; a Claudine block such as <claudine_append_system_prompt>...</claudine_append_system_prompt> creates an explicit boundary without needing JSON/YAML."
recent_changes:
  - date: "2026-06-30"
    version: "0.80.3"
    change: "Fixed extension tool changes so they apply before the next provider request without dropping before_agent_start system-prompt overrides."
    impact: "Prompt-mutating extensions should be more reliable when tools change during a run."
  - date: "2026-05-22"
    version: "0.74.0"
    change: "Added ctx.getSystemPromptOptions() for extension commands to inspect current base prompt inputs."
    impact: "Extensions can inspect structured prompt inputs outside before_agent_start."
  - date: "2026-05-13"
    version: "0.71.0"
    change: "Changed system prompt and context file boundaries to explicit XML tags instead of Markdown headings."
    impact: "XML-wrapped appended sections align with Pi's own prompt-boundary style."
  - date: "2026-04-20"
    version: "0.68.0"
    change: "Added systemPromptOptions to before_agent_start extension events."
    impact: "Extensions can inspect custom prompt, active tools, prompt guidelines, append prompt text, cwd, loaded context files, and loaded skills without re-discovery."
  - date: "2026-04-14"
    version: "0.67.2"
    change: "Added support for multiple --append-system-prompt flags."
    impact: "Wrappers can pass multiple append files or strings; Pi joins them with blank-line separation."
quirks:
  - "The current installed local binary was 0.73.1, while upstream source on 2026-07-03 was 0.80.3. This research uses upstream behavior and observed local config, not the stale binary's behavior."
  - "Project .pi/SYSTEM.md and .pi/APPEND_SYSTEM.md are ignored in non-interactive mode unless project trust is already saved, defaultProjectTrust is always, or --approve is passed."
  - "AGENTS.md/CLAUDE.md context files are loaded regardless of project trust; use --no-context-files when wrapper isolation requires suppressing them."
  - "A replacement prompt is not the final complete provider prompt: Pi still appends append prompts, context files, skills, date, and cwd."
  - "Append file discovery is exclusive: a project APPEND_SYSTEM.md suppresses the global APPEND_SYSTEM.md, and any --append-system-prompt flag suppresses both discovered append files."
  - "Extensions can rewrite the prompt after CLI/file delivery. Use --no-extensions for deterministic wrapper tests, or pass an explicit safe extension set with -e."
  - "before_provider_request can mutate the serialized provider payload in ways ctx.getSystemPrompt() cannot inspect."
  - "Skills only appear in the prompt as metadata and only when read is active. The model may fail to read the full SKILL.md unless prompted or invoked via /skill:name."
  - "The official subagent example appends delegated agent prompts instead of replacing the child system prompt, so agent files are layered instructions rather than isolated base identities."
  - "Local config inspection found no SYSTEM.md, APPEND_SYSTEM.md, AGENTS.md, trust.json, or extensions under the current session HOME (/Users/ken/.claudine/.pi/agent). The user's real /Users/ken/.pi/agent had settings.json with defaultProvider/defaultModel and no prompt files."
gaps:
  - "No dedicated CLI command was found to print or export Pi's final effective provider payload."
  - "Source inspection proves path-or-inline resolution, but no authenticated provider call was run to observe actual transmitted request payload."
  - "Windows paths are inferred from Pi's homedir/config-dir implementation and documented path shape; not executed on Windows."
  - "No current public issue thread was found that changes prompt semantics after upstream commit 23d1462611ab74b4874c35e701a43d7caa5e3de3."
changes:
  - "Refreshed Pi system-prompt research against upstream 0.80.3 source and docs on 2026-07-03."
  - "Added extension hook, subagent example, trust-gating, local config inspection, and wrapper delivery details."
requires_claudine_update: true
reason: "Pi has native append and replace CLI flags with file-path support; Claudine can implement wrapper delivery without mutating user config once Pi is added to provider metadata."
---

# Pi System Prompt Research

## Overview

Pi provides native per-invocation system-prompt replacement and append surfaces. `--system-prompt` replaces the base prompt, and repeatable `--append-system-prompt` values append to the chosen base prompt. Both flags accept inline text or a path to an existing file.

Replacement is not a total final-payload replacement. Pi still layers append text, context files, skills, the current date, and the current working directory after a custom base prompt. Extensions can then mutate the turn prompt through `before_agent_start`, and can even rewrite the final serialized provider payload through `before_provider_request`.

The best Claudine delivery path is therefore file-backed flags: write a temporary prompt file and pass `--append-system-prompt <file>` for append or `--system-prompt <file>` for replace. This avoids mutating `~/.pi/agent` or `.pi`, avoids shell quoting problems, and keeps long prompts out of argv as inline strings.

Local inspection on 2026-07-03 found:

| Location | Observation |
|----------|-------------|
| Current session `$HOME` | `/Users/ken/.claudine` |
| Current session Pi config | `/Users/ken/.claudine/.pi/agent` contained only `auth.json` and `sessions`; no prompt files, settings, trust file, skills, or extensions were observed. |
| User Pi config | `/Users/ken/.pi/agent/settings.json` existed and set `defaultProvider` and `defaultModel`; no `SYSTEM.md`, `APPEND_SYSTEM.md`, `AGENTS.md`, `trust.json`, or extensions were found in the inspected paths. |
| Installed Pi binary | `pi --version` returned `0.73.1`; upstream source inspected at `23d1462611ab74b4874c35e701a43d7caa5e3de3` reported package version `0.80.3`. |

## CLI Parameters

| Switch | Mode | Value | Wrapper relevance |
|--------|------|-------|-------------------|
| `--system-prompt <text>` | Replace | Inline text or file path | Primary Claudine replace mechanism. |
| `--append-system-prompt <text>` | Append | Inline text or file path; repeatable | Primary Claudine append mechanism. |
| `--no-context-files`, `-nc` | Disable | Boolean | Suppresses AGENTS.md/CLAUDE.md context injection, not SYSTEM.md/APPEND_SYSTEM.md. |
| `--no-skills`, `-ns` | Disable | Boolean | Suppresses automatic skill prompt metadata; explicit `--skill` still loads. |
| `--skill <path>` | Modify | File or directory path | Adds skill metadata to the system prompt when `read` is active. |
| `--extension <path>`, `-e <path>` | Modify | TypeScript file or directory | Loads extension hooks that can inspect or mutate prompts. |
| `--no-extensions`, `-ne` | Disable | Boolean | Prevents discovered extension prompt rewrites; explicit `-e` still loads. |
| `--approve`, `-a` | Modify | Boolean | Enables trust-gated project prompt files/resources for one run. |
| `--no-approve`, `-na` | Disable | Boolean | Ignores trust-gated project prompt files/resources for one run. |
| `--tools`, `--exclude-tools`, `--no-tools`, `--no-builtin-tools` | Modify/disable | Tool name sets | Changes available-tool snippets, tool guidelines, and skill visibility. |

Pi's `resolvePromptInput()` treats a prompt argument as a file only when `existsSync(input)` succeeds; otherwise the raw value is used as prompt text. If reading an existing file fails, Pi logs a warning and falls back to using the argument string as text.

## Configuration and Discovery

Pi discovers system-prompt and prompt-adjacent resources from user and project scopes:

| Source | Scope | Behavior |
|--------|-------|----------|
| `~/.pi/agent/SYSTEM.md` | User | Replaces built-in base prompt when no CLI or trusted project replacement exists. |
| `.pi/SYSTEM.md` | Project | Replaces built-in base prompt when trusted and no CLI replacement exists. |
| `~/.pi/agent/APPEND_SYSTEM.md` | User | Appends when no CLI append flag and no trusted project append file exists. |
| `.pi/APPEND_SYSTEM.md` | Project | Appends when trusted and no CLI append flag exists. |
| `~/.pi/agent/AGENTS.md` or `CLAUDE.md` | User | Appended in `<project_context>` unless context files are disabled. |
| `AGENTS.md`, `AGENTS.MD`, `CLAUDE.md`, `CLAUDE.MD` in cwd and ancestors | Repo | Appended in parent-first order in `<project_context>` unless disabled. |
| `~/.pi/agent/settings.json` | User | Can set `defaultProjectTrust`, extensions, skills, and other prompt-adjacent defaults. |
| `.pi/settings.json` | Project | Trust-gated project settings; can enable extensions and skills. |
| `~/.pi/agent/extensions/` and `.pi/extensions/` | Extension | TypeScript hooks can modify prompt text or provider payloads. |
| `~/.pi/agent/skills/`, `~/.agents/skills/`, `.pi/skills/`, `.agents/skills/` | User/repo | Skill metadata is rendered into the prompt in XML format when active. |

Project trust matters for `.pi` resources. In non-interactive modes, Pi does not prompt; it uses saved trust, `defaultProjectTrust`, or the explicit `--approve`/`--no-approve` override. Context files are the exception: AGENTS.md and CLAUDE.md load independently of project trust unless disabled.

`PI_CODING_AGENT_DIR` is the most useful environment-level isolation knob. It changes the global agent directory from `~/.pi/agent` to a wrapper-chosen directory, affecting global prompt files, settings, extensions, skills, and sessions.

## Prompt Layers and Precedence

```mermaid
flowchart TD
    A["Built-in base prompt"] --> B{"Replacement source?"}
    B -->|"--system-prompt"| C["CLI custom base"]
    B -->|".pi/SYSTEM.md trusted"| D["Project custom base"]
    B -->|"~/.pi/agent/SYSTEM.md"| E["Global custom base"]
    B -->|"none"| F["Built-in base retained"]
    C --> G["Append source"]
    D --> G
    E --> G
    F --> G
    G -->|"--append-system-prompt values"| H["CLI append text"]
    G -->|".pi/APPEND_SYSTEM.md trusted"| I["Project append text"]
    G -->|"~/.pi/agent/APPEND_SYSTEM.md"| J["Global append text"]
    G -->|"none"| K["No append text"]
    H --> L["AGENTS.md / CLAUDE.md project_context"]
    I --> L
    J --> L
    K --> L
    L --> M["available_skills XML block"]
    M --> N["Current date and cwd"]
    N --> O["before_agent_start extension chain"]
    O --> P["before_provider_request payload rewrite"]
```

Source-observed order:

1. Choose the base prompt: CLI `--system-prompt`, trusted project `.pi/SYSTEM.md`, global `~/.pi/agent/SYSTEM.md`, or Pi's built-in prompt.
2. Add append text immediately after the base prompt: CLI append values, trusted project `APPEND_SYSTEM.md`, global `APPEND_SYSTEM.md`, or nothing.
3. Add context files inside `<project_context>` and `<project_instructions path="...">` tags.
4. Add visible skills inside `<available_skills>` if `read` is active.
5. Add `Current date: YYYY-MM-DD` and `Current working directory: ...`.
6. Run `before_agent_start` extension handlers in load order; each handler sees the currently chained prompt and may return a full replacement for that turn.
7. Build the provider request; `before_provider_request` handlers may rewrite the serialized payload.

## Agents and Subagents

Pi's README says Pi ships without built-in subagents and expects extensions to provide workflows such as subagents or plan mode. The official repository includes a subagent example extension that is useful as the current provider-native pattern, but it is not a core CLI subagent feature.

The subagent example:

| Feature | Behavior |
|---------|----------|
| Agent definitions | Markdown files with YAML frontmatter and a Markdown body. |
| User location | `~/.pi/agent/agents/*.md`. |
| Project location | `.pi/agents/*.md`, enabled only with `agentScope: "project"` or `"both"`. |
| Prompt delivery | The extension writes the agent body to a temp file and launches child `pi` with `--append-system-prompt <tempfile>`. |
| Isolation | Each subagent is a separate Pi subprocess with isolated context. |
| Return path | Final output, usage, and errors return to the parent as a tool result. |

Because the example uses append, subagent prompt files layer on top of the child Pi base prompt rather than replacing it. A wrapper or extension could use `--system-prompt` instead, but that is not the sample behavior.

## Format Recommendations

Use file-backed Markdown for both append and replace.

For replacement, use a complete Markdown prompt. Pi's `SYSTEM.md` convention and `--system-prompt` behavior treat the content as raw prompt text, and the built-in layers will still append context, skills, date, and cwd.

For append, prefer XML-wrapped Markdown for Claudine-authored blocks:

```xml
<claudine_append_system_prompt>

## Additional Instructions

Markdown instructions go here.

</claudine_append_system_prompt>
```

Plain Markdown works, but Pi itself changed prompt and context boundaries to XML tags, and skills are emitted as XML. XML wrapping makes Claudine's appended block easy to distinguish from adjacent project context and skill metadata. YAML and JSON add parsing implications that Pi does not document for system prompts.

## Recent Changes

| Date | Version | Change | Impact |
|------|---------|--------|--------|
| 2026-06-30 | 0.80.3 | Fixed extension tool changes so they apply without dropping `before_agent_start` prompt overrides. | More reliable extension prompt mutation. |
| 2026-05-22 | 0.74.0 | Added `ctx.getSystemPromptOptions()` for extension commands. | Extensions can inspect base prompt inputs outside `before_agent_start`. |
| 2026-05-13 | 0.71.0 | Switched prompt/context file boundaries to explicit XML tags. | XML-wrapped append blocks align with Pi's prompt style. |
| 2026-04-20 | 0.68.0 | Added `systemPromptOptions` to `before_agent_start`. | Extensions can inspect structured base prompt inputs. |
| 2026-04-14 | 0.67.2 | Added multiple `--append-system-prompt` flags. | Claudine can pass more than one append file/string if needed. |

Older but relevant changes include automatic `SYSTEM.md` loading, `APPEND_SYSTEM.md` support, and `--system-prompt` file-path support. These were already present before the current upstream version.

## Quirks and Workarounds

- Use `--no-extensions` for deterministic wrapper validation. Otherwise global or project extensions can modify prompts after Claudine's delivery.
- Use `--no-context-files` only when suppressing AGENTS.md/CLAUDE.md is intentional. It is separate from system-prompt append/replace.
- Use `--approve` if Claudine intentionally wants project `.pi/SYSTEM.md`, `.pi/APPEND_SYSTEM.md`, `.pi/extensions`, or `.pi/skills` to participate in non-interactive runs.
- Use `--no-approve` if Claudine wants to ignore project trust-gated prompt files while still allowing global config.
- Use `PI_CODING_AGENT_DIR` or a shadow HOME if global Pi resources must be isolated. CLI prompt flags alone do not prevent global extensions, global skills, or global context files from loading.
- `ctx.getSystemPrompt()` is not an export of the final provider payload. It misses later `before_provider_request` rewrites.
- There is no documented standalone `pi --print-effective-system-prompt` style command. Inspection requires an extension hook or source-level/SDK instrumentation.

## Claudine Delivery Notes

Recommended append delivery:

```bash
PI_OFFLINE=1 PI_SKIP_VERSION_CHECK=1 pi --append-system-prompt /tmp/claudine-pi-append.md -p "..."
```

Recommended replace delivery:

```bash
PI_OFFLINE=1 PI_SKIP_VERSION_CHECK=1 pi --system-prompt /tmp/claudine-pi-system.md -p "..."
```

For strict wrapper isolation:

```bash
PI_CODING_AGENT_DIR=/tmp/claudine-pi-agent pi --no-extensions --no-context-files --system-prompt /tmp/claudine-pi-system.md -p "..."
```

Do not permanently write `~/.pi/agent/SYSTEM.md`, `~/.pi/agent/APPEND_SYSTEM.md`, `.pi/SYSTEM.md`, or `.pi/APPEND_SYSTEM.md` for wrapper delivery. File flags are native and avoid config mutation.

## Changelog

- 2026-07-03: Rewrote the Pi system-prompt research against upstream `@earendil-works/pi-coding-agent` 0.80.3 source and docs, adding current CLI semantics, discovery paths, extension hooks, subagent behavior, local config observations, and Claudine delivery guidance.

## Sources

- [Pi usage documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/usage.md)
- [Pi README](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/README.md)
- [System prompt builder source](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts)
- [Resource loader source](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/resource-loader.ts)
- [CLI argument parser source](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts)
- [Extension documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md)
- [Skills documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/skills.md)
- [Security and project trust documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/security.md)
- [Subagent example documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/subagent/README.md)
- [Pi coding-agent changelog](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/CHANGELOG.md)
