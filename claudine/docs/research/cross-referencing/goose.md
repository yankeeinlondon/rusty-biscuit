---
homepage: https://block.github.io/goose/
docs: https://block.github.io/goose/docs/
skills: https://block.github.io/goose/docs/guides/context-engineering/using-skills/
agent: https://block.github.io/goose/docs/guides/subagents/
slash: https://block.github.io/goose/docs/guides/goose-cli-commands/
scripts: null # No dedicated scripts directory; scripts live inside skill directories
---

# Goose: Cross-Referencing

Goose is an open-source, extensible AI agent by Block (formerly Square). It runs locally, supports any LLM provider, and is available as both a CLI and desktop application. Written primarily in Rust with a TypeScript desktop UI. Latest release: v1.25.0 (Feb 18, 2026).

## Skills

Goose supports the **Agent Skills open standard** (directory with a `SKILL.md` file). Skills were introduced in **v1.16.0** (Dec 10, 2025) via [PR #5760](https://github.com/block/goose/pull/5760). The built-in Skills extension (enabled by default since v1.25.0; previously required the Summon extension) discovers skills at session startup and injects their names, paths, and descriptions into the system prompt. The agent then decides which skills to load based on the current task.

### Directory discovery (priority order)

Goose checks these directories in order; **later directories override earlier ones** when skill names conflict:

| Priority | Scope | Directory |
|----------|-------|-----------|
| 1 (lowest) | User | `~/.claude/skills/` |
| 2 | User | `~/.config/agents/skills/` |
| 3 | User | `~/.config/goose/skills/` |
| 4 | Repo | `./.claude/skills/` |
| 5 | Repo | `./.goose/skills/` |
| 6 (highest) | Repo | `./.agents/skills/` |

Discovery of `~/.claude/skills/` and `./.claude/skills/` means **Goose reads Claude Code's skill directories**. The `.agents/skills/` paths (cross-platform portable) were added in v1.18.0 (Dec 19, 2025).

### Skill directory structure

Minimal:
```
<skill-name>/
  SKILL.md
```

Recommended:
```
<skill-name>/
  SKILL.md
  scripts/       # Executable code or utilities
  references/    # Supporting documentation
  assets/        # Templates, images, data files
```

### Skill metadata (frontmatter)

`SKILL.md` begins with YAML frontmatter followed by Markdown content.

**Required fields:**

| Field | Rules | Example |
|-------|-------|---------|
| `name` | 1-64 chars; lowercase alphanumeric + hyphens; no consecutive hyphens; must match directory name | `code-review` |
| `description` | 1-1024 chars; describes WHAT it does AND WHEN to use it | `Review code for bugs and style. Use when the user asks for a code review` |

**Optional fields:**

| Field | Description | Example |
|-------|-------------|---------|
| `license` | License name or file reference | `Apache-2.0` |
| `compatibility` | Environment requirements (up to 500 chars) | `Requires python>=3.8, pdfplumber` |
| `metadata` | Key-value pairs for custom properties | `version: "1.0"` |
| `allowed-tools` | Space-delimited pre-approved tools (experimental) | `Bash(python:*) Read` |

### Activation model

- **Automatic**: Goose activates skills when the user's request matches the skill's `description`.
- **Explicit**: Users can say "Use the code-review skill" or ask "What skills are available?"
- **Progressive disclosure**: Discovery (name + description) happens at session start; full content loads on demand.

### Best practices (from official docs)

- Follow the `<what it does>. Use when <specific triggers>` format for `description`.
- Keep `SKILL.md` concise; move detailed content to reference files.
- The `description` field is critical -- vague descriptions lead to unreliable activation.
- Supporting files (scripts, references) are accessed via the Developer extension's file tools.

### Differences from Claude Code skills

| Aspect | Goose | Claude Code |
|--------|-------|-------------|
| Activation | Automatic based on description matching | Context-based with manual option |
| Directory priority | Later directories win | Project overrides user |
| Extra frontmatter | `allowed-tools`, `compatibility` | `description` only required |
| Cross-platform dirs | `.agents/skills/`, `~/.config/agents/skills/` | Not checked |
| Integration | Tightly coupled with recipes and subagents | Standalone knowledge units |

## Slash Commands

Goose has two categories of slash commands: **built-in session commands** and **recipe-based custom commands**.

### Built-in slash commands (in interactive session)

Available during `goose session` or `goose run --interactive`. Tab completion via `/` + `<Tab>`.

| Command | Description |
|---------|-------------|
| `/?` or `/help` | Display help menu |
| `/builtin <names>` | Add builtin extensions (comma-separated) |
| `/clear` | Clear chat history |
| `/compact` | Summarize conversation to reduce context |
| `/endplan` | Exit plan mode, return to normal mode |
| `/exit` or `/quit` | Exit the session |
| `/extension <command>` | Add stdio extension |
| `/mode <name>` | Set mode: auto, approve, chat, smart_approve |
| `/plan <message>` | Enter plan mode with optional message |
| `/prompt <n> [--info] [key=value...]` | Execute or get info about a prompt |
| `/prompts [--extension <name>]` | List available prompts |
| `/recipe [filepath]` | Generate recipe from conversation (saves as YAML) |
| `/r` | Toggle full tool output display |
| `/t` or `/t <name>` | Toggle/set theme (light, dark, ansi) |

Slash commands were formalized in **v1.18.0** (Dec 19, 2025) via [PR #5858](https://github.com/block/goose/pull/5858).

### Recipe-based custom slash commands

Recipes can be registered as slash commands in `~/.config/goose/config.yaml`. Once registered, typing `/<command-name>` in the GUI or REPL executes that recipe. Documentation for this feature was added in v1.17.0 via [PR #6075](https://github.com/block/goose/pull/6075).

### Differences from Claude Code slash commands

| Aspect | Goose | Claude Code |
|--------|-------|-------------|
| Custom commands | Recipe YAML files registered in config | Markdown files in `.claude/commands/` |
| Discovery | Config file + built-in | Directory-based (`commands/*.md`) |
| Format | YAML with extensions, parameters, retry logic | Markdown with optional frontmatter |
| Reads Claude dirs | No (does not check `.claude/commands/`) | N/A (native) |
| Parameterization | `{{ variable }}` template syntax with typed params | `$ARGUMENTS` placeholder |

## Agents / Subagents

Goose uses the term **"subagents"** for isolated agent instances that execute tasks independently. Subagents were introduced in **v1.0.30** (Jun 27, 2025) via [PR #2797](https://github.com/block/goose/pull/2797). Subrecipes (YAML-defined subagents) were unified with ad-hoc subagents in v1.13.0-v1.14.0.

### How subagents work

- Subagents are **temporary, isolated Goose instances** that run tasks in their own session.
- They **do not share** conversation history, memory, or state with the parent session.
- Results are returned to the parent when the subagent completes.
- Subagents **cannot spawn their own subagents** (prevents recursion; enforced since v1.14.0).

### Triggering subagents

**Autonomous creation** (default mode):
- Goose autonomously decides when subagents are beneficial.
- Requires `auto` permission mode (the default).
- Disabled in `approve`, `smart_approve`, and `chat` modes.

**Explicit creation** via natural language:
- "Use a code reviewer to analyze this function for security issues"
- "Create three HTML templates in parallel"
- "Use a subagent with only the developer extension to refactor main.py"

### Execution models

| Pattern | Trigger keywords | Behavior |
|---------|-----------------|----------|
| Sequential (default) | "first...then", "after" | Tasks run one after another |
| Parallel | "parallel", "simultaneously", "concurrently", "at the same time" | Tasks run simultaneously |

Failed or timed-out subagents produce no output. In parallel execution, only successful results are returned.

### Configuration

| Setting | Default | Override |
|---------|---------|---------|
| Max turns | 25 | `GOOSE_SUBAGENT_MAX_TURNS` env var or natural language |
| Timeout | 5 minutes | Natural language ("with 20-minute timeout") |
| Extensions | Inherited from parent | Natural language ("with only developer extension") |
| Return mode | Full details | "Just give me the summary" |

### Subrecipes (YAML-defined subagents)

Subrecipes are YAML recipe files referenced by a parent recipe. They provide reusable, parameterized subagent definitions.

```yaml
sub_recipes:
  - name: "code-reviewer"
    path: "./review.yaml"
    values:
      focus_area: "security"
```

Key properties of subrecipes:
- Complete isolation: no shared history, memory, or state.
- Cannot nest (subrecipes cannot define their own subrecipes).
- Parameters via `{{ variable }}` template syntax.
- Pre-set `values` take precedence over context-derived parameters.
- Parallel execution available for independent subrecipes.

### Differences from Claude Code

| Aspect | Goose | Claude Code |
|--------|-------|-------------|
| Vernacular | "Subagent" / "subrecipe" | "Sub-agent" via Task tool |
| Definition | Natural language or YAML recipe files | Markdown files in `.claude/agents/` |
| Triggering | Autonomous (in auto mode) or explicit | Explicit via Task tool only |
| Isolation | Full: separate session, no shared history | Task tool returns result to orchestrator |
| Configuration | Env vars, natural language, recipe YAML | Agent markdown with YAML frontmatter |
| Nesting | Prohibited (single level) | Allowed (can nest Task calls) |
| Parallel execution | Native keyword triggers | Via concurrent Task tool calls |

## Scripts

Goose does **not** have a dedicated global scripts directory. Scripts are stored inside skill directories.

### Script locations

| Scope | Location | Purpose |
|-------|----------|---------|
| Inside a skill | `<skill-name>/scripts/` | Skill-specific executables and utilities |
| Project | `./.goose/scripts/` (convention, not enforced) | Project-specific utilities |

The official docs recommend placing executable scripts alongside the skill that uses them:

```
deployment-skill/
  SKILL.md
  scripts/
    health-check.sh
    rollback.sh
    notify.py
```

Best practices:
- Make scripts executable (`chmod +x`).
- Use shebang lines (`#!/usr/bin/env python3`).
- Include help documentation in comments or separate `.md` files.

## Context Files (Goosehints)

While not directly equivalent to Claude Code skills or commands, Goose has its own context file system that provides persistent instructions:

| File | Scope | Purpose |
|------|-------|---------|
| `AGENTS.md` | Repo root | Project context (checked first by default) |
| `.goosehints` | Directory hierarchy | Project/directory-specific hints |
| `~/.config/goose/.goosehints` | Global | Global hints for all sessions |

Configuration:
- `CONTEXT_FILE_NAMES` env var overrides the default `["AGENTS.md", ".goosehints"]`.
- `.goosehints` files are combined hierarchically from repo root to current directory.
- `@file.md` syntax in hints auto-includes file content; plain references are optional.
- Requires the Developer extension to be enabled.

## Recipes (Reusable Workflows)

Recipes are Goose's primary reusable workflow mechanism. Introduced in **v1.0.18** (Apr 16, 2025) via [PR #2115](https://github.com/block/goose/pull/2115).

### Recipe format

YAML files (`.yaml` only; `.yml` not supported by CLI).

**Required fields:**
- `title`: Short descriptive name.
- `description`: Detailed explanation.
- At least one of `instructions` or `prompt`.

**Optional fields:**

| Field | Purpose |
|-------|---------|
| `version` | Format version (default: `"1.0.0"`) |
| `extensions` | MCP server/extension requirements |
| `parameters` | Dynamic input definitions with `{{ variable }}` syntax |
| `settings` | Provider, model, temperature, max_turns overrides |
| `response` | JSON schema for structured output validation |
| `retry` | Automatic retry with shell command validation |
| `sub_recipes` | References to subrecipe files |
| `activities` | Desktop-only clickable buttons and info boxes |

### Recipe locations

- `GOOSE_RECIPE_PATH` env var (custom directories).
- `GOOSE_RECIPE_GITHUB_REPO` (GitHub repository).
- Current working directory.
- `~/.config/goose/` (saved recipes).

## Sources

- [Goose Homepage](https://block.github.io/goose/)
- [Goose GitHub Repository](https://github.com/block/goose)
- [Using Skills](https://block.github.io/goose/docs/guides/context-engineering/using-skills/)
- [Subagents Guide](https://block.github.io/goose/docs/guides/subagents/)
- [CLI Commands Reference](https://block.github.io/goose/docs/guides/goose-cli-commands/)
- [Recipe Reference Guide](https://block.github.io/goose/docs/guides/recipes/recipe-reference/)
- [Sub-Recipes for Specialized Tasks](https://block.github.io/goose/docs/guides/recipes/subrecipes)
- [Shareable Recipes](https://block.github.io/goose/docs/guides/recipes/session-recipes/)
- [Configuration Files](https://block.github.io/goose/docs/guides/config-files/)
- [Context Engineering](https://block.github.io/goose/docs/guides/context-engineering/)
- [Providing Hints to Goose](https://block.github.io/goose/docs/guides/context-engineering/using-goosehints)
- [3 Principles for Designing Agent Skills (Block Engineering Blog)](https://engineering.block.xyz/blog/3-principles-for-designing-agent-skills)
- [GitHub Discussion #5761: Implement skills.md](https://github.com/block/goose/discussions/5761)
- [v1.16.0 Release Notes (Skills introduced)](https://github.com/block/goose/releases/tag/v1.16.0)
- [v1.0.30 Release Notes (Subagents introduced)](https://github.com/block/goose/releases/tag/v1.0.30)
- [v1.0.18 Release Notes (Recipes introduced)](https://github.com/block/goose/releases/tag/v1.0.18)
- [v1.18.0 Release Notes (Slash commands formalized)](https://github.com/block/goose/releases/tag/v1.18.0)
