---
$schema: ./_schema.yaml
created: 2026-04-02
last_updated: 2026-07-02
agent: opencode
model: k2p7
docs: https://geminicli.com/docs/
system_prompt_docs: https://geminicli.com/docs/cli/system-prompt/
append_support: config
replace_support: env
cli_params:
  - flag: --approval-mode
    mode: modify
    value_shape: string
    description: Sets the approval mode for tool execution. Choices are default, auto_edit, plan, and yolo. Plan mode changes how the agent behaves but does not inject custom system prompt text.
    example: gemini --approval-mode=plan
    notes: Affects execution policy, not the literal system prompt. YOLO mode is equivalent to --approval-mode=yolo.
  - flag: --sandbox / -s
    mode: modify
    value_shape: boolean
    description: Run the session inside a sandboxed environment. The CLI injects sandbox-aware instructions into the effective prompt.
    example: gemini -s
    notes: Indirectly shapes the prompt by adding sandbox-specific guidance and restricting available tools.
  - flag: --extensions / -e
    mode: modify
    value_shape: array
    description: Enable specific extensions. Extensions can register tools and may contribute skill metadata that appears in prompt variable substitution.
    example: gemini -e my-extension
    notes: Indirectly affects the prompt through ${AgentSkills}, ${SubAgents}, and ${AvailableTools} variables when a custom system prompt is used.
  - flag: --allowed-mcp-server-names
    mode: modify
    value_shape: array
    description: Restrict which MCP servers are available. Changes the set of tools that can appear in ${AvailableTools} substitution.
    example: gemini --allowed-mcp-server-names=github,slack
    notes: No direct prompt override; only changes the tool surface exposed to variable substitution.
  - flag: --include-directories
    mode: modify
    value_shape: array
    description: Add directories to the workspace. GEMINI.md files in these directories may be discovered as part of the hierarchical context.
    example: gemini --include-directories ../shared
    notes: Indirect append path; could be used to include a temporary directory containing a GEMINI.md, but this is a workaround rather than a native append flag.
  - flag: --model / -m
    mode: other
    value_shape: string
    description: Select the model alias or concrete model name for the session.
    example: gemini -m pro
    notes: Does not change prompt text directly, but model-specific config overrides may alter generation behavior.
config_sources:
  - os: all
    scope: repo
    path: ./.gemini/system.md
    mode: replace
    format: markdown
    notes: Default path used when GEMINI_SYSTEM_MD is set to 1 or true. Replaces the entire built-in system prompt.
  - os: all
    scope: user
    path: ~/.gemini/GEMINI.md
    mode: append
    format: markdown
    notes: Global project-context instructions loaded for every session.
  - os: all
    scope: repo
    path: ./GEMINI.md
    mode: append
    format: markdown
    notes: Project-level instructions loaded from the workspace root.
  - os: all
    scope: repo
    path: ./.gemini/GEMINI.md
    mode: append
    format: markdown
    notes: Alternative project-level context file inside the .gemini directory.
  - os: all
    scope: repo
    path: "./**/GEMINI.md"
    mode: append
    format: markdown
    notes: Just-in-time context files discovered when a tool accesses a file or directory. Scanned in that directory and its ancestors up to a trusted root.
  - os: all
    scope: user
    path: ~/.gemini/settings.json
    mode: modify
    format: json
    notes: Can set context.fileName to customize the discovered context filename(s), plus model configs, agent overrides, and plan-mode settings.
  - os: all
    scope: repo
    path: ./.gemini/settings.json
    mode: modify
    format: json
    notes: Project-scoped settings; can override context.fileName, agent configs, and model configs.
  - os: all
    scope: user
    path: ~/.gemini/agents/*.md
    mode: replace
    format: markdown
    notes: User-level subagent definitions. The markdown body becomes the subagent system prompt.
  - os: all
    scope: repo
    path: ./.gemini/agents/*.md
    mode: replace
    format: markdown
    notes: Project-level subagent definitions. The markdown body becomes the subagent system prompt.
  - os: all
    scope: repo
    path: ./.gemini/.env
    mode: modify
    format: other
    notes: Project-level environment file loaded by the CLI. Can persist GEMINI_SYSTEM_MD or other env overrides.
env_vars:
  - name: GEMINI_SYSTEM_MD
    effect: Replaces the built-in system prompt with the contents of a Markdown file.
    mode: replace
  - name: GEMINI_WRITE_SYSTEM_MD
    effect: Exports the current built-in system prompt to a file. Set to 1/true to write to ./.gemini/system.md, or to a path for a custom location.
    mode: inspect
  - name: GEMINI_API_KEY
    effect: Provides a Gemini API key for authentication. Does not affect prompt content.
    mode: other
  - name: GOOGLE_API_KEY
    effect: Provides an API key when using Vertex AI or API-key authentication. Does not affect prompt content.
    mode: other
  - name: GOOGLE_GENAI_USE_VERTEXAI
    effect: Switches the CLI to Vertex AI mode. Does not affect prompt content.
    mode: other
  - name: GOOGLE_CLOUD_PROJECT
    effect: Sets the Google Cloud project for Code Assist or Vertex authentication. Does not affect prompt content.
    mode: other
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the path to the system defaults JSON file.
    mode: modify
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the path to the system override JSON file.
    mode: modify
  - name: GEMINI_SANDBOX
    effect: Forces sandbox mode. May inject sandbox-specific instructions into the effective prompt.
    mode: modify
prompt_layers:
  - source: Built-in core system prompt
    mode: replace
    scope:
      - session
    order_notes: Base layer; replaced entirely when GEMINI_SYSTEM_MD is active.
    notes: Contains tool-use guidance, safety instructions, and execution policies. Not published verbatim, but can be exported with GEMINI_WRITE_SYSTEM_MD.
  - source: GEMINI_SYSTEM_MD file
    mode: replace
    scope:
      - session
    order_notes: Replaces the built-in core system prompt when the environment variable is set.
    notes: The caller must include required variables such as ${AvailableTools} if tool guidance is still needed.
  - source: GEMINI.md hierarchy
    mode: append
    scope:
      - user
      - repo
    order_notes: Global GEMINI.md first, then workspace GEMINI.md, then JIT files discovered at tool-access time.
    notes: Loaded as project context rather than into the system prompt itself; concatenated and sent with every prompt.
  - source: Agent Skills metadata
    mode: append
    scope:
      - session
    order_notes: Skill names and descriptions injected at startup; full SKILL.md loaded when selected.
    notes: Skills live in ~/.gemini/skills and ./.gemini/skills.
  - source: Auto Memory
    mode: append
    scope:
      - session
    order_notes: Persistent memories loaded when the memory feature is enabled.
    notes: Managed through the /memory command and memory tool.
  - source: Subagent system prompt
    mode: replace
    scope:
      - subagent
    order_notes: Custom system prompt for the spawned subagent; runs in an isolated context loop.
    notes: Defined in .gemini/agents/*.md or ~/.gemini/agents/*.md; the markdown body becomes the prompt.
agent_prompting:
  supported: true
  definition_surface: Markdown files with YAML frontmatter in ~/.gemini/agents/ or ./.gemini/agents/
  inheritance: Subagents inherit the parent session model when omitted; tools default to parent set unless a tools list is provided. MCP servers can be isolated per subagent.
  isolation: Each subagent runs in its own context loop with independent history. Only the final result returns to the parent.
  limitations: Subagents cannot call other subagents, even when granted the * tool wildcard. Built-in agents include codebase_investigator, cli_help, generalist, and browser_agent.
claudine_delivery:
  append_strategy: unsupported
  replace_strategy: env_var_file
  temp_file_required: true
  argv_limit: Not applicable; Gemini CLI has no native system-prompt argv flags.
  notes: For replace, write the resolved prompt to a temporary file and set GEMINI_SYSTEM_MD=<tmp> for the wrapped invocation. For append, there is no native per-invocation mechanism; GEMINI.md is the documented append surface but requires file creation in the workspace hierarchy. Claudine should treat append as unsupported or document the GEMINI.md workaround.
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: Appended context via GEMINI.md works best as plain Markdown headers and lists that blend with the hierarchical context chain. Replacement via GEMINI_SYSTEM_MD should still be Markdown, but must include variable substitutions such as ${AgentSkills}, ${SubAgents}, and ${AvailableTools} to preserve tool and skill discovery.
recent_changes:
  - date: "2026-06-03"
    version: "0.45.0"
    change: Context Manager Simplification completed, refactoring how context files and memory are loaded.
    impact: Affects reliability and ordering of GEMINI.md and memory layers.
  - date: "2026-06-18"
    version: unknown
    change: Google announced that unpaid-tier and Google One users of Gemini CLI will be transitioned to Antigravity CLI.
    impact: Long-term provider viability is uncertain; Antigravity CLI may use different system-prompt mechanisms.
  - date: "2026-05"
    version: "0.44.x"
    change: Subagent tool isolation and markdown-based agent definitions stabilized.
    impact: Subagents now carry distinct system prompts with isolated tools and MCP servers.
  - date: "2026-04"
    version: "0.43.x"
    change: Browser agent added and model routing defaults updated.
    impact: New built-in subagent and tool variables available for substitution.
quirks:
  - Gemini CLI has no --append-system-prompt, --system-prompt, or equivalent CLI flag. Prompt overrides are env-var or file-based only.
  - GEMINI.md files are loaded as project context, not into the system prompt itself, so they are softer than a true system-prompt append.
  - Replacing the system prompt with GEMINI_SYSTEM_MD removes all built-in safety and tool-use guidance unless the custom file reintroduces it via variable substitution.
  - The CLI shows a |⌐■_■| indicator in the UI when GEMINI_SYSTEM_MD is active.
  - Plan mode is configured via general.plan.enabled and --approval-mode=plan; it is not a mandatory pre-execution phase for all sessions.
  - Subagents cannot recursively spawn other subagents, even with the * tool wildcard.
  - Persisting GEMINI_SYSTEM_MD in ./.gemini/.env makes the override durable for the project, which may surprise users who expected a one-off change.
  - The default GEMINI.md filename can be customized via context.fileName in settings.json, but only one filename list is active at a time.
gaps:
  - No documented native mechanism to append to the system prompt for a single invocation without creating or modifying GEMINI.md files.
  - No CLI flag exists for replacing the system prompt; GEMINI_SYSTEM_MD is env-var only.
  - It is unclear whether GEMINI.md context is still loaded when GEMINI_SYSTEM_MD replaces the core system prompt.
  - The Antigravity CLI transition may invalidate these findings for users on the unpaid tier after June 18, 2026.
  - No public API exposes the effective built-in prompt as plain text except via GEMINI_WRITE_SYSTEM_MD export.
changes:
  - "Updated claudine_delivery to classify replace as env_var_file and append as unsupported."
  - "Corrected earlier claims about mandatory plan mode; plan mode is configurable, not mandatory."
  - "Added GEMINI_WRITE_SYSTEM_MD as the documented export/inspection mechanism."
  - "Refreshed sources and recent changes against geminicli.com docs and local Gemini CLI v0.46.0 inspection."
requires_claudine_update: true
reason: Gemini CLI lacks a native per-invocation append mechanism and has no CLI flag for system-prompt replacement. Claudine's provider metadata should reflect that replace is delivered via GEMINI_SYSTEM_MD env var pointing to a temp file, while append is unsupported or requires the GEMINI.md file workaround.
---

## Overview

Gemini CLI distinguishes between two ways to influence instructions: the **core system prompt** (firmware) and the **project context** (strategy). The core system prompt can be replaced entirely through the `GEMINI_SYSTEM_MD` environment variable, while project context is appended through a hierarchy of `GEMINI.md` files. There are no dedicated CLI flags such as `--append-system-prompt` or `--system-prompt`; manipulation is env-var and file-discovery based. Subagents carry their own isolated system prompts defined in Markdown files.

## CLI Parameters

Gemini CLI does not expose flags that directly append or replace system prompt text. The options that touch the effective prompt do so indirectly:

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--approval-mode <mode>` | Modify | Changes execution policy (`default`, `auto_edit`, `plan`, `yolo`). Plan mode is read-only during planning. |
| `--sandbox` / `-s` | Modify | Enables sandbox mode; injects sandbox-aware instructions and restricts tools. |
| `--extensions <names>` / `-e` | Modify | Enables extensions that contribute tools and may appear in prompt variable substitution. |
| `--allowed-mcp-server-names <names>` | Modify | Restricts MCP servers, changing the tool surface available for `${AvailableTools}` substitution. |
| `--include-directories <dirs>` | Modify | Adds workspace directories; `GEMINI.md` files inside them may be discovered as appended context. |
| `--model <alias>` / `-m` | Other | Selects the model; does not change prompt text. |

There are no inline or file-based `--system-prompt` or `--append-system-prompt` equivalents.

## Configuration and Discovery

### Core system prompt replacement (`GEMINI_SYSTEM_MD`)

The core system prompt is replaced by setting the `GEMINI_SYSTEM_MD` environment variable:

- `GEMINI_SYSTEM_MD=1` or `GEMINI_SYSTEM_MD=true` reads `./.gemini/system.md`.
- `GEMINI_SYSTEM_MD=/path/to/system.md` reads the specified file.
- Relative paths and tilde expansion are supported.
- `GEMINI_SYSTEM_MD=0`, `false`, or unset restores the built-in prompt.

If the variable points to a missing file, the CLI errors with `missing system prompt file '<path>'`.

### Project context (`GEMINI.md`)

`GEMINI.md` files provide persistent project instructions. The CLI discovers and concatenates them in this order:

1. `~/.gemini/GEMINI.md` (global user context)
2. Workspace `GEMINI.md` files from configured workspace directories and their parents
3. Just-in-time `GEMINI.md` files in directories accessed by tools, scanned up to a trusted root

The footer shows the count of loaded context files. Use `/memory show` to inspect the concatenated context and `/memory reload` to rescan.

### Imports and custom filenames

- Use `@./path/to/file.md` inside a `GEMINI.md` to import other Markdown files.
- Customize the discovered filename(s) via `context.fileName` in `settings.json`.

### Agent definitions

Custom subagents are Markdown files with YAML frontmatter in `~/.gemini/agents/*.md` or `./.gemini/agents/*.md`. The file body becomes the agent's system prompt. Fields include `name`, `description`, `tools`, `model`, `temperature`, `max_turns`, `timeout_mins`, and `mcpServers`.

## Prompt Layers and Precedence

```mermaid
graph TD
    A[Built-in core system prompt] --> B{GEMINI_SYSTEM_MD set?}
    B -- yes --> C[Contents of system.md file]
    B -- no --> D[Built-in core system prompt]
    C --> E[GEMINI.md hierarchy]
    D --> E
    E --> F[Agent skills metadata]
    F --> G[Auto memory]
    G --> H[User prompt]
```

Notes on precedence:

- `GEMINI_SYSTEM_MD` replaces the built-in core system prompt entirely.
- `GEMINI.md` files append project context on top of whichever core prompt is active.
- Skills, memory, and subagents add further layers.
- Subagents use their own system prompt and do not inherit the parent `GEMINI.md` context automatically.

## Agents and Subagents

Gemini CLI supports built-in subagents (`codebase_investigator`, `cli_help`, `generalist`, `browser_agent`) and user-defined agents. Each subagent has its own system prompt and tool set.

Key behaviors:

- Custom agents are defined as Markdown files with YAML frontmatter; the body is the system prompt.
- Subagents run in isolated context loops with independent history.
- Only the final result returns to the parent session.
- Subagents cannot call other subagents, even with the `*` tool wildcard.
- Tool access is restricted by the `tools` list; if omitted, the subagent inherits the parent's tools.
- Inline MCP servers can be scoped to a subagent via `mcpServers` in the frontmatter.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append (`GEMINI.md`) | Pure Markdown | Headers, lists, and short paragraphs blend with the concatenated context chain. |
| Replace (`GEMINI_SYSTEM_MD`) | Markdown with variable substitution | The custom file must reintroduce dynamic content such as `${AgentSkills}`, `${SubAgents}`, and `${AvailableTools}` to keep tools, skills, and agents usable. |

For replacements, the file should explicitly include any safety rules, tool protocols, and workflow instructions the task still needs, because the built-in prompt is removed entirely.

## Recent Changes

- **v0.45.0 (2026-06-03)**: Context Manager Simplification refactor completed, changing how context files and memory are loaded and ordered.
- **Antigravity CLI transition (2026-06-18)**: Google announced that unpaid-tier and Google One Gemini CLI users will move to Antigravity CLI. This may change system-prompt behavior for affected users.
- **Subagent tool isolation (2026-05)**: Markdown-based agent definitions with isolated tools and MCP servers became stable.
- **Browser agent (2026-04)**: Added a built-in browser automation subagent and updated default model routing.

## Quirks and Workarounds

- There is no CLI flag for system prompt append or replace; use `GEMINI_SYSTEM_MD` for replace and `GEMINI.md` for append.
- `GEMINI.md` is project context, not a hard system-prompt layer, so it can be overridden by strong instructions in a custom `system.md`.
- When `GEMINI_SYSTEM_MD` is active, the CLI displays a `|⌐■_■|` indicator.
- Export the built-in prompt first with `GEMINI_WRITE_SYSTEM_MD=1 gemini` before editing it, so required variables and safety rules are preserved.
- Plan mode is not mandatory for all sessions; it is governed by `general.plan.enabled` and `--approval-mode=plan`.
- Persisting `GEMINI_SYSTEM_MD` in `./.gemini/.env` makes it durable; remove or set it to `0` to restore defaults.

## Claudine Delivery Notes

- **Replace**: Write the resolved replacement prompt to a temporary Markdown file and invoke Gemini CLI with `GEMINI_SYSTEM_MD=<tmp>` set in the environment. This is a per-invocation change that does not mutate user `settings.json` or persistent `GEMINI.md` files.
- **Append**: Gemini CLI has no native per-invocation append mechanism. The documented append surface is the `GEMINI.md` hierarchy, which requires creating or modifying files in the workspace. Claudine should treat append as unsupported for this provider unless it implements a temporary-file workaround.
- **Export/inspect**: Use `GEMINI_WRITE_SYSTEM_MD=1 gemini` to export the current built-in prompt to `./.gemini/system.md` for review.

## Changelog

- Updated `claudine_delivery` to classify replace as `env_var_file` and append as `unsupported`.
- Corrected earlier claims about mandatory plan mode and version-specific firmware changes; plan mode is configurable, not mandatory.
- Added `GEMINI_WRITE_SYSTEM_MD` as the documented export/inspection mechanism.
- Refreshed sources and recent changes against geminicli.com docs and local Gemini CLI v0.46.0 inspection.

## Sources

- [Gemini CLI documentation](https://geminicli.com/docs/)
- [System Prompt Override (GEMINI_SYSTEM_MD)](https://geminicli.com/docs/cli/system-prompt/)
- [Provide context with GEMINI.md files](https://geminicli.com/docs/cli/gemini-md/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Subagents](https://geminicli.com/docs/core/subagents/)
- [CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/)
- [Latest stable release notes](https://geminicli.com/docs/changelogs/latest/)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
