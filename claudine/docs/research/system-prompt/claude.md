---
$schema: ./_schema.yaml
created: 2026-03-30
last_updated: 2026-07-02
agent: opencode
model: k2p7
docs: https://code.claude.com/docs/en/cli-reference
system_prompt_docs: https://code.claude.com/docs/en/agent-sdk/modifying-system-prompts
append_support: native
replace_support: native
cli_params:
  - flag: --append-system-prompt
    mode: append
    value_shape: string
    description: Append custom text to the end of the default system prompt.
    example: claude --append-system-prompt "Always use TypeScript"
    notes: Temporary; applies only to the current invocation. Can be combined with --system-prompt or --system-prompt-file.
  - flag: --append-system-prompt-file
    mode: append
    value_shape: path
    description: Load additional system prompt text from a file and append it to the default prompt.
    example: claude --append-system-prompt-file ./extra-rules.txt
    notes: Temporary; applies only to the current invocation. Can be combined with replacement flags.
  - flag: --system-prompt
    mode: replace
    value_shape: string
    description: Replace the entire default system prompt with custom text.
    example: claude --system-prompt "You are a Python expert"
    notes: Mutually exclusive with --system-prompt-file. Drops built-in tool guidance, safety instructions, and coding conventions.
  - flag: --system-prompt-file
    mode: replace
    value_shape: path
    description: Load a system prompt from a file, replacing the default prompt.
    example: claude --system-prompt-file ./custom-prompt.txt
    notes: Mutually exclusive with --system-prompt. The caller takes responsibility for any tool guidance and safety instructions the task still needs.
  - flag: --exclude-dynamic-system-prompt-sections
    mode: modify
    value_shape: boolean
    description: Move per-machine sections (working directory, environment info, memory paths, git-repo flag) from the system prompt into the first user message.
    example: claude -p --exclude-dynamic-system-prompt-sections "query"
    notes: Ignored when --system-prompt or --system-prompt-file is set. Improves prompt-cache reuse across users and machines.
  - flag: --safe-mode
    mode: disable
    value_shape: boolean
    description: Start Claude Code with CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, LSP servers, and auto-memory disabled.
    example: claude --safe-mode
    notes: Managed policy still applies. Equivalent to CLAUDE_CODE_SAFE_MODE=1.
  - flag: --bare
    mode: disable
    value_shape: boolean
    description: Minimal mode; skip auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Uses a minimal system prompt with Bash, file read, and file edit tools.
    example: claude --bare -p "query"
    notes: Sets CLAUDE_CODE_SIMPLE=1.
config_sources:
  - os: all
    scope: system
    path: /Library/Application Support/ClaudeCode/CLAUDE.md (macOS), /etc/claude-code/CLAUDE.md (Linux/WSL), C:\Program Files\ClaudeCode\CLAUDE.md (Windows)
    mode: append
    format: markdown
    notes: Organization-wide managed CLAUDE.md; cannot be excluded by user or project settings.
  - os: all
    scope: system
    path: managed-settings.json / MDM plist / Windows registry
    mode: append
    format: json
    notes: Managed settings can include claudeMd content and enforce policy; cannot be overridden by user/project settings.
  - os: all
    scope: user
    path: ~/.claude/CLAUDE.md
    mode: append
    format: markdown
    notes: Personal preferences loaded at the start of every session.
  - os: all
    scope: repo
    path: ./CLAUDE.md or ./.claude/CLAUDE.md
    mode: append
    format: markdown
    notes: Project-level instructions; shared via source control.
  - os: all
    scope: repo
    path: ./CLAUDE.local.md
    mode: append
    format: markdown
    notes: Personal project-specific preferences; should be gitignored.
  - os: all
    scope: repo
    path: ./.claude/rules/*.md
    mode: append
    format: markdown
    notes: Path-scoped rules loaded when Claude reads files matching their paths patterns.
  - os: all
    scope: user
    path: ~/.claude/output-styles/*.md
    mode: replace
    format: markdown
    notes: User-level output styles that replace or extend the default engineering instructions.
  - os: all
    scope: repo
    path: ./.claude/output-styles/*.md
    mode: replace
    format: markdown
    notes: Project-level output styles; closest to the working directory wins when names collide.
  - os: all
    scope: user
    path: ~/.claude/agents/*.md
    mode: replace
    format: markdown
    notes: User-level subagent definitions with their own system prompt in the markdown body.
  - os: all
    scope: repo
    path: ./.claude/agents/*.md
    mode: replace
    format: markdown
    notes: Project-level subagent definitions; closest to the working directory wins on name collisions.
  - os: all
    scope: user
    path: ~/.claude/settings.json
    mode: modify
    format: json
    notes: Can set outputStyle, agent, includeGitInstructions, and claudeMdExcludes.
  - os: all
    scope: repo
    path: ./.claude/settings.json or ./.claude/settings.local.json
    mode: modify
    format: json
    notes: Project/local settings including outputStyle, agent, and claudeMdExcludes.
env_vars:
  - name: CLAUDE_CODE_DISABLE_AUTO_MEMORY
    effect: Disables loading and writing of auto memory files.
    mode: modify
  - name: CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD
    effect: Loads CLAUDE.md, .claude/CLAUDE.md, .claude/rules/*.md, and CLAUDE.local.md from directories passed via --add-dir.
    mode: modify
  - name: CLAUDE_CODE_SAFE_MODE
    effect: Equivalent to --safe-mode; disables CLAUDE.md, skills, plugins, hooks, MCP, output styles, etc.
    mode: disable
  - name: CLAUDE_CODE_SIMPLE
    effect: Equivalent to --bare; minimal system prompt with reduced tool set.
    mode: disable
  - name: CLAUDE_CODE_SIMPLE_SYSTEM_PROMPT
    effect: Uses a shorter system prompt and abbreviated tool descriptions while keeping full tool discovery.
    mode: modify
  - name: CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS
    effect: Removes built-in commit/PR workflow instructions and git status snapshot from the system prompt.
    mode: modify
  - name: CLAUDE_CODE_ATTRIBUTION_HEADER
    effect: Omits the attribution block (client version and prompt fingerprint) from the system prompt when set to 0.
    mode: modify
  - name: CLAUDE_AX_SCREEN_READER
    effect: Enables screen-reader friendly output; affects rendering, not prompt content directly.
    mode: other
prompt_layers:
  - source: Default system prompt ("claude_code" preset)
    mode: replace
    scope:
      - session
    order_notes: Base layer; replaced entirely by --system-prompt/--system-prompt-file, or extended by --append-system-prompt.
    notes: Includes tool guidance, safety instructions, coding conventions, and dynamic environment sections.
  - source: Output style
    mode: replace
    scope:
      - session
    order_notes: Applied via outputStyle setting; replaces engineering instructions unless keep-coding-instructions is true.
    notes: Part of the system prompt; rebuilt on /clear or restart.
  - source: --append-system-prompt / --append-system-prompt-file
    mode: append
    scope:
      - session
    order_notes: Appended to the end of the default system prompt after output-style instructions.
    notes: Temporary per-invocation additions.
  - source: Managed CLAUDE.md / claudeMd setting
    mode: append
    scope:
      - organization
    order_notes: Loaded before user and project CLAUDE.md.
    notes: Cannot be excluded by user settings.
  - source: User CLAUDE.md (~/.claude/CLAUDE.md)
    mode: append
    scope:
      - user
    order_notes: Loaded after managed and before project CLAUDE.md.
    notes: Injected as project context, not into the system prompt itself.
  - source: Project CLAUDE.md (./CLAUDE.md or ./.claude/CLAUDE.md)
    mode: append
    scope:
      - repo
    order_notes: Loaded after user CLAUDE.md.
    notes: Injected as project context, not into the system prompt itself.
  - source: Auto memory (MEMORY.md)
    mode: append
    scope:
      - repo
    order_notes: First 200 lines or 25KB loaded at startup.
    notes: Injected as project context, not into the system prompt itself.
  - source: Path-scoped rules (.claude/rules/*.md)
    mode: append
    scope:
      - repo
    order_notes: Loaded when Claude reads files matching paths patterns.
    notes: Injected as project context.
  - source: Subagent system prompt
    mode: replace
    scope:
      - subagent
    order_notes: Custom system prompt for the subagent; runs in isolated context window.
    notes: May include memory preload and skill preload.
agent_prompting:
  supported: true
  definition_surface: Markdown files with YAML frontmatter in ~/.claude/agents/ or ./.claude/agents/
  inheritance: Subagents receive their own system prompt (file body) plus basic environment details; parent CLAUDE.md and auto memory may load unless the agent is Explore/Plan.
  isolation: Each subagent runs in its own context window; only the final summary returns to the parent.
  limitations: Cannot spawn AskUserQuestion, EnterPlanMode, ExitPlanMode (unless plan mode), ScheduleWakeup, or WaitForMcpServers. Built-in Explore/Plan skip CLAUDE.md and parent git status.
claudine_delivery:
  append_strategy: file_flag
  replace_strategy: file_flag
  temp_file_required: true
  argv_limit: No published argv limit for --append-system-prompt/--system-prompt; use the -file variants for large prompts.
  notes: Claudine's wrapper accepts --append-system-prompt/--asp and --replace-system-prompt/--rsp as file paths. It discovers system-prompt.md from the launch-CWD hierarchy and passes the resolved content to Claude Code's native --append-system-prompt-file or --system-prompt-file flags, avoiding persistent mutation of user config.
format_recommendations:
  append_format: markdown
  replace_format: xml_wrapped_markdown
  rationale: Appending layers onto an already-structured prompt works best with plain Markdown headers and lists. Replacing the entire prompt requires self-provided structure; XML tags help the model distinguish rules, context, constraints, and examples.
recent_changes:
  - date: "2026-07-01"
    version: "2.1.198"
    change: Subagents run in the background by default and can spawn nested subagents up to 5 levels deep. Built-in Explore agent inherits the main session's model.
    impact: Changes default lifecycle and cost profile of subagents; agent prompts remain isolated.
  - date: "2026-06-24"
    version: "2.1.191"
    change: Foreground subagents now respect the same 5-level depth limit as background subagents.
    impact: Prevents unbounded nested agent recursion.
  - date: "2026-06-17"
    version: "2.1.181"
    change: /config key=value syntax added for any setting, including outputStyle, from the prompt.
    impact: Output style (a system prompt layer) can be changed mid-session via command but only takes effect after /clear or restart.
  - date: "2026-06-15"
    version: "2.1.178"
    change: Nested .claude/ directories closest to the working directory now win when output style, agent, or workflow names collide.
    impact: Allows monorepo-style prompt layering at the subdirectory level.
  - date: "2026-06-08"
    version: "2.1.169"
    change: Added --safe-mode flag and CLAUDE_CODE_SAFE_MODE environment variable to disable CLAUDE.md, output styles, skills, plugins, hooks, MCP, and auto-memory for troubleshooting.
    impact: Provides a clean way to verify whether the effective prompt is being affected by local customizations.
quirks:
  - CLAUDE.md content is injected as a user/project-context message, not into the system prompt itself, so it is softer than --append-system-prompt.
  - --append-system-prompt text is appended after output-style instructions, so a configured output style can override or precede appended rules.
  - The default system prompt embeds per-machine context (working directory, platform, shell, OS version, memory paths, git-repo flag), which invalidates prompt cache across different machines unless --exclude-dynamic-system-prompt-sections is used.
  - Replacing the system prompt drops all built-in tool guidance and safety instructions; the caller must re-implement anything the task still needs.
  - --system-prompt and --system-prompt-file are mutually exclusive, but append flags can be combined with either replacement flag.
  - Project-root CLAUDE.md survives /compact; nested subdirectory CLAUDE.md files do not reload automatically after compaction.
  - Managed policy CLAUDE.md cannot be excluded by claudeMdExcludes.
  - The /output-style command was removed in v2.1.91; use /config or edit the outputStyle setting directly.
gaps:
  - Anthropic does not publish the full default system prompt, so exact token counts and section ordering can only be inferred from documentation and /context output.
  - No documented provider API exports or inspects the effective built-in system prompt as plain text; /context shows only high-level sections and token counts.
  - It is unclear whether --append-system-prompt and --append-system-prompt-file support multi-file or repeated invocations in a single command.
changes: []
requires_claudine_update: false
reason: Claude Code's native CLI already provides direct --append-system-prompt-file and --system-prompt-file flags, which align with Claudine's file-based delivery model. No new wrapper mechanism is required.
---

## Overview

Claude Code builds the effective prompt for every session from several ordered layers. The base is the unpublished default system prompt (the `claude_code` preset), which contains tool-use guidance, safety instructions, coding conventions, and dynamic environment context. On top of that, optional output styles, per-invocation append/replace flags, and project-context files such as `CLAUDE.md` and auto memory shape what Claude knows and how it behaves. Subagents run with their own isolated system prompts and return only a summary to the parent session.

## CLI Parameters

Claude Code exposes four flags that directly manipulate the system prompt for a single invocation. They work in both interactive and non-interactive (`-p`) modes.

| Flag | Mode | Effect |
| :--- | :--- | :--- |
| `--append-system-prompt "<text>"` | Append | Adds text to the end of the default system prompt. |
| `--append-system-prompt-file <path>` | Append | Adds the contents of a file to the end of the default system prompt. |
| `--system-prompt "<text>"` | Replace | Replaces the entire default system prompt with the supplied text. |
| `--system-prompt-file <path>` | Replace | Replaces the entire default system prompt with the contents of a file. |

`--system-prompt` and `--system-prompt-file` are mutually exclusive. The append flags can be combined with either replacement flag. Replacement drops the built-in tool guidance and safety instructions, so it is only appropriate when the new prompt supplies everything the task needs. Append is the safer default because it preserves the `claude_code` preset.

Additional related flags:

| Flag | Effect |
| :--- | :--- |
| `--exclude-dynamic-system-prompt-sections` | Moves per-machine sections (working directory, environment info, memory paths, git-repo flag) out of the system prompt and into the first user message to improve cross-machine prompt-cache reuse. Ignored when replacing the system prompt. |
| `--safe-mode` | Disables `CLAUDE.md`, output styles, skills, plugins, hooks, MCP servers, custom commands/agents, workflows, themes, keybindings, status line, LSP servers, and auto-memory for troubleshooting. |
| `--bare` | Uses a minimal system prompt with only Bash, file read, and file edit tools; skips discovery of hooks, skills, plugins, MCP, auto-memory, and `CLAUDE.md`. |

## Configuration and Discovery

Beyond CLI flags, Claude Code discovers persistent instruction sources automatically.

### CLAUDE.md hierarchy

`CLAUDE.md` files are plain Markdown files that load at session start. They are injected into the conversation as project context rather than into the system prompt itself. Discovery walks up the directory tree from the working directory, loading files in this order:

1. Managed policy `CLAUDE.md` (macOS `/Library/Application Support/ClaudeCode/CLAUDE.md`, Linux/WSL `/etc/claude-code/CLAUDE.md`, Windows `C:\Program Files\ClaudeCode\CLAUDE.md`)
2. User `~/.claude/CLAUDE.md`
3. Project `./CLAUDE.md` or `./.claude/CLAUDE.md`
4. Project-local `./CLAUDE.local.md` alongside each loaded `CLAUDE.md`

Within each directory, `CLAUDE.local.md` is appended after `CLAUDE.md`. Files closer to the working directory are loaded later, so more specific instructions take precedence. Subdirectory `CLAUDE.md` files under the working directory load on demand when Claude reads files in those subdirectories.

### Output styles

Output styles are Markdown files stored in `~/.claude/output-styles/` or `./.claude/output-styles/`. They directly modify the system prompt and can replace the default engineering instructions unless their frontmatter sets `keep-coding-instructions: true`. They are activated via the `outputStyle` setting in `settings.json` or through `/config`. Because output styles are part of the system prompt, changes only take effect after `/clear` or a new session.

### Settings files

`settings.json` at user, project, or local scope can set:

- `outputStyle`: selects an output style.
- `agent`: runs the main thread as a named subagent, applying that agent's system prompt and restrictions.
- `includeGitInstructions`: includes or excludes built-in commit/PR workflow instructions and the git status snapshot.
- `claudeMdExcludes`: glob patterns that skip specific `CLAUDE.md` files.
- `claudeMd`: managed-only inline CLAUDE.md-style instructions.

### Subagent definitions

Custom subagents are Markdown files with YAML frontmatter in `~/.claude/agents/` or `./.claude/agents/`. The file body becomes the subagent's system prompt. Definitions can also be passed inline for a single session via the `--agents` CLI flag or configured through managed settings.

### Managed settings

Organizations can deploy managed `CLAUDE.md` files and `claudeMd` content through `managed-settings.json`, MDM plist, or Windows registry entries. Managed policy sources load first and cannot be excluded by user or project settings.

## Prompt Layers and Precedence

The final context for a session is assembled from the following layers, from most foundational to most specific.

```mermaid
graph TD
    A[Default claude_code system prompt] --> B{Output style configured?}
    B -- yes --> C[Output style instructions]
    B -- no --> D[Default engineering instructions]
    C --> E[--append-system-prompt / --append-system-prompt-file]
    D --> E
    E --> F[Managed CLAUDE.md]
    F --> G[User CLAUDE.md]
    G --> H[Project CLAUDE.md]
    H --> I[CLAUDE.local.md]
    I --> J[Auto memory MEMORY.md]
    J --> K[Path-scoped rules loaded on demand]
    K --> L[User prompt]
```

Notes on precedence:

- `--system-prompt` or `--system-prompt-file` replaces layers A, B, C, D, and E entirely; `CLAUDE.md` and memory still load as project context.
- `--append-system-prompt` adds to layer E after any output style.
- `CLAUDE.md` and auto memory are user/project-context messages, not part of the system prompt, so they are softer than `--append-system-prompt`.
- Managed policy `CLAUDE.md` cannot be skipped with `claudeMdExcludes`.

## Agents and Subagents

Claude Code supports custom agents defined as Markdown files with YAML frontmatter in `~/.claude/agents/` or `./.claude/agents/`. Each subagent has its own system prompt (the file body), its own tool allowlist or denylist, an optional model, permission mode, MCP servers, hooks, and memory scope.

Key behaviors:

- Subagents run in isolated context windows. Only the final summary returns to the parent.
- The main session can run as a subagent via `claude --agent <name>` or the `agent` setting; this replaces the default system prompt with the agent's system prompt, the same way `--system-prompt` does.
- Built-in subagents include Explore, Plan, and general-purpose. Explore and Plan skip `CLAUDE.md` and the parent git status to keep research fast and inexpensive.
- As of v2.1.198, subagents run in the background by default and can spawn nested subagents up to five levels deep.
- The `Agent` tool replaced the older `Task` tool in v2.1.63; `Task(...)` references still work as aliases.

## Format Recommendations

| Goal | Recommended format | Rationale |
| :--- | :--- | :--- |
| Append | Pure Markdown | Headers, bullet lists, and short paragraphs blend cleanly with the existing structured system prompt. |
| Replace | XML-wrapped Markdown | When the built-in structure is removed, XML tags such as `<rules>`, `<constraints>`, `<context>`, and `<examples>` help the model distinguish instruction categories. |

For replacements, the prompt must explicitly supply any tool-calling guidance, safety instructions, and environment context the task requires because the default `claude_code` preset is removed entirely.

## Recent Changes

- **v2.1.198 (2026-07-01)**: Subagents now run in the background by default and can spawn nested subagents up to five levels deep. The built-in Explore agent inherits the main session's model.
- **v2.1.191 (2026-06-24)**: Foreground subagents now respect the same five-level depth limit as background subagents.
- **v2.1.181 (2026-06-17)**: `/config key=value` syntax now works for any setting, including `outputStyle`, though the system prompt is only rebuilt on `/clear` or restart.
- **v2.1.178 (2026-06-15)**: Nested `.claude/` directories now resolve collisions by proximity to the working directory for output styles, agents, and workflows.
- **v2.1.169 (2026-06-08)**: Added `--safe-mode` and `CLAUDE_CODE_SAFE_MODE` to disable `CLAUDE.md`, output styles, skills, plugins, hooks, MCP, and auto-memory for troubleshooting.
- **v2.1.91 (late 2025)**: Removed the standalone `/output-style` command; use `/config` or edit `outputStyle` directly.
- **v2.1.63 (2026-02)**: Renamed the `Task` tool to `Agent`; existing `Task(...)` references continue to work as aliases.

## Quirks and Workarounds

- `CLAUDE.md` is context, not enforcement. For behavior that must run at a specific lifecycle point, use a hook such as `PreToolUse` or `PostToolUse` rather than relying on `CLAUDE.md` alone.
- If an output style is configured, its instructions are placed before `--append-system-prompt` content, so style rules can shadow appended rules.
- The default prompt includes dynamic per-machine sections that break prompt-cache reuse across machines. Use `--exclude-dynamic-system-prompt-sections` for scripted, multi-user workloads.
- Project-root `CLAUDE.md` survives `/compact`, but nested subdirectory `CLAUDE.md` files do not reload automatically; they come back when Claude next reads a file in that subdirectory.
- `--safe-mode` and `--bare` are useful for verifying whether a misbehavior is caused by local customizations.
- Replacing the system prompt is powerful but removes the built-in tool and safety guidance; most use cases are better served by `--append-system-prompt` or an output style.

## Claudine Delivery Notes

Claudine should continue using its file-based delivery path:

- Discover a `system-prompt.md` file from the launch working-directory hierarchy.
- For append mode, write the resolved content to a temporary file and invoke Claude Code with `--append-system-prompt-file <tmp>`.
- For replace mode, write the resolved content to a temporary file and invoke Claude Code with `--system-prompt-file <tmp>`.
- Both modes are temporary per-invocation changes, so no user `settings.json`, `CLAUDE.md`, or output style is permanently mutated.
- Because Claude Code natively supports both inline and file flags, the file-backed approach avoids argv-length limits and keeps the wrapper simple.

## Sources

- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Modifying system prompts (Agent SDK)](https://code.claude.com/docs/en/agent-sdk/modifying-system-prompts)
- [How Claude remembers your project](https://code.claude.com/docs/en/memory)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Create custom subagents](https://code.claude.com/docs/en/sub-agents)
- [Output styles](https://code.claude.com/docs/en/output-styles)
- [Explore the context window](https://code.claude.com/docs/en/context-window)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code changelog](https://code.claude.com/docs/en/changelog)
