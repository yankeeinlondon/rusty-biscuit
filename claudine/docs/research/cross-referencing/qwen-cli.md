---
homepage: https://github.com/QwenLM/qwen-code
docs: https://qwenlm.github.io/qwen-code-docs/
skills: https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/
agent: https://qwenlm.github.io/qwen-code-docs/en/users/features/sub-agents/
slash: https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/
scripts: n/a (no dedicated scripts directory; scripts live inside skill directories)
---

# Qwen Code CLI: Skills, Slash Commands, Agents

Qwen Code is an open-source terminal-based AI coding agent developed by the Qwen team (Alibaba Cloud). It is forked from Google Gemini CLI and adapted with customized prompts and function-calling protocols optimized for Qwen3-Coder models. Installed via npm (`@qwen-code/qwen-code`), the binary is invoked as `qwen`.

Qwen Code uses `QWEN.md` (configurable via `context.fileName`) as its memory/instructions file, analogous to Claude Code's `CLAUDE.md`. It does **not** read `CLAUDE.md` or any Claude Code directories.

## Skills

Qwen Code supports the Agent Skills open format (a directory containing a `SKILL.md` file). Skills were introduced as an experimental feature in v0.6.0 (December 2025, PR #1314) behind the `--experimental-skills` CLI flag. As of the current documentation, the official skills docs page no longer mentions the flag, and the `/skills` built-in slash command lists and invokes available skills. The TypeScript SDK enables skills by default.

Skills are "model-invoked": the model autonomously decides when to load and use a skill based on the task and the skill's description. Users can also explicitly invoke a skill via `/skills <name>`.

### Directory discovery

User-level (personal):
- `~/.qwen/skills/`

Project-level:
- `.qwen/skills/`

Extension-level:
- Provided by installed Qwen Code extensions (discovered automatically when extension is enabled)

Precedence: project > user > extension.

There is an open request (issue #1695) to also read from `~/.agents/skills/` and `.agents/skills/` (the cross-tool agentskills convention), but this is not yet implemented natively. Qwen Code does **not** read `.claude/skills/` or `~/.claude/skills/`.

### Skill directory structure

Minimal:
- `<skill-name>/SKILL.md`

Recommended layout:
```
my-skill/
├── SKILL.md          (required)
├── reference.md      (optional)
├── examples.md       (optional)
├── scripts/
│   └── helper.py
└── templates/
    └── template.txt
```

This is the only documented place for putting scripts/executables: inside a skill directory (e.g. `scripts/` under `<skill-name>/`).

### Skill metadata (frontmatter)

`SKILL.md` begins with YAML frontmatter, then Markdown content.

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Non-empty string; lowercase with hyphens recommended |
| `description` | Yes | Non-empty string; describes what the skill does and when to use it |

No other frontmatter fields are currently documented. Unlike some other tools (Kimi, OpenCode), Qwen Code does not document `license`, `compatibility`, or `metadata` fields.

### Best practices (from official docs)

- Keep each skill focused on one capability.
- Write specific descriptions including keywords users would naturally mention (this drives model-invoked discovery).
- Reference supporting files from SKILL.md using relative paths.
- Test with your team before deployment.
- Project-level skills can be committed to git for team sharing.

## Slash Commands

Qwen Code supports both built-in slash commands and user-defined custom commands.

### Built-in slash commands

Session and project management:
- `/init` - analyze current directory and create initial context file
- `/summary` - generate project summary from conversation history
- `/compress` - replace chat history with summary to save tokens
- `/resume` - resume a previous conversation session
- `/restore` - restore files to pre-execution state
- `/save` - save conversation with a tag

Interface and workspace:
- `/clear` (Ctrl+L) - clear terminal screen
- `/theme` - change visual theme
- `/vim` - toggle Vim editing mode
- `/directory` - manage multi-directory workspace
- `/editor` - select editor

Language settings:
- `/language` - view or change language settings
- `/language ui [lang]` - set UI language (zh-CN, en-US, ru-RU, de-DE)
- `/language output [lang]` - set LLM output language

Tools and models:
- `/mcp` - list configured MCP servers and tools
- `/tools` - display available tool list
- `/skills` - list and run available skills
- `/approval-mode` - control approval mode (plan, default, auto-edit, yolo)
- `/model` - switch model for current session
- `/extensions` - list active extensions
- `/memory` - manage instruction context (loaded from QWEN.md by default)

Information and settings:
- `/help` (alias: `/?`) - display help
- `/about` - show version
- `/stats` - session statistics
- `/settings` - open settings editor
- `/auth` - change authentication method
- `/bug` - submit issue
- `/copy` - copy last output to clipboard
- `/quit` (alias: `/exit`) - exit

Agents:
- `/agents create` - guided wizard for creating a new subagent
- `/agents manage` - interactive dialog for managing existing subagents

Other prefix commands:
- `@<file>` or `@<dir>` - inject file/directory content into conversation
- `!<command>` - execute shell command directly (sets `QWEN_CODE=1`)

### Custom commands

Custom commands are user-defined prompt shortcuts stored as files.

Directories (project overrides user when names collide):
- User (global): `~/.qwen/commands/`
- Project: `<project-root>/.qwen/commands/`

File format (recommended): Markdown with optional YAML frontmatter:

```markdown
---
description: Optional description (shown in /help)
---
Your prompt content here. Use {{args}} for parameter injection.
```

File format (deprecated, still supported): TOML:

```toml
prompt = "Prompt content with {{args}} for parameters"
description = "Optional description"
```

Frontmatter properties for custom commands:

| Field | Required | Description |
|-------|----------|-------------|
| `description` | No | Shown in `/help` and command completion |

The body content becomes the prompt template.

Naming convention: file path separators (`/` or `\`) convert to colons. For example:
- `~/.qwen/commands/test.md` becomes `/test`
- `.qwen/commands/git/commit.md` becomes `/git:commit`

Parameter mechanisms:
- `{{args}}` - context-aware injection with automatic shell escaping
- Default (no `{{args}}`) - parameters appended to prompt end
- `!{command}` - execute shell command inline (requires user confirmation)
- `@{path}` - embed file content (respects .gitignore for directories)

Qwen Code does **not** read `.claude/commands/` or `~/.claude/commands/`.

## Agents / Subagents

Qwen Code supports subagents: specialized AI assistants with their own system prompts, tool access, and behaviors. The term used is "SubAgents" (or "subagents").

### Directory locations

| Scope | Path | Priority |
|-------|------|----------|
| Project | `.qwen/agents/` | Highest |
| User | `~/.qwen/agents/` | Medium |
| Extension | Extension's `agents/` directory | Lowest |

Qwen Code does **not** read `.claude/agents/` or `~/.claude/agents/`.

### Agent file format

Subagents are Markdown files with YAML frontmatter:

```yaml
---
name: testing-expert
description: Creates comprehensive unit tests and integration tests
tools:
  - read_file
  - write_file
  - read_many_files
  - run_shell_command
---

You are a testing specialist. Your task is to create thorough, maintainable tests.

Focus on:
- Happy path scenarios
- Edge cases and error conditions
- Integration points
```

Frontmatter properties:

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier for the agent |
| `description` | Yes | Describes specialization; helps orchestrator decide when to delegate |
| `tools` | No | Array of tools the agent can access |
| `color` | No | Display color (set automatically by `/agents create`) |

The body content is the agent's system prompt.

### Interaction model

Subagents are invoked through the Task tool. The orchestrator (main Qwen Code agent) delegates tasks to subagents based on:

1. **Automatic delegation**: The main AI identifies an appropriate subagent based on the task description and the agent's description. Descriptions containing phrases like "use PROACTIVELY" or "MUST BE USED" encourage automatic selection.
2. **Explicit invocation**: The user requests a specific agent by name (e.g., "Have the testing-expert create unit tests").

Execution flow:
1. Orchestrator analyzes the request and selects a subagent
2. Subagent receives the task with relevant context
3. Subagent works independently using its configured tools
4. Subagent returns results and execution summary to the main conversation

Subagents have their own conversation context (isolated from the orchestrator). There is no documented support for nested subagent delegation (subagents calling other subagents).

### Management commands

- `/agents create` - guided step-by-step wizard to create a new subagent
- `/agents manage` - interactive dialog for viewing and managing existing subagents

### Differences from Claude Code

- Qwen Code uses Markdown files with YAML frontmatter for agent definitions; Claude Code uses the same format but with a slightly different field set.
- Qwen Code stores agents in `.qwen/agents/` and `~/.qwen/agents/`; Claude Code uses `.claude/agents/` and `~/.claude/agents/`.
- Qwen Code agents specify tools as an array of tool names in frontmatter; Claude Code agents use `allowed-tools` with glob patterns (e.g., `Bash(git *)`).
- Qwen Code provides `/agents create` and `/agents manage` built-in commands for agent lifecycle management; Claude Code does not have equivalent built-in commands.
- Both use the Task tool for delegation with isolated context per subagent.

### Gotcha: project-level tool restrictions

A `.qwen/settings.json` with a `coreTools` restriction (e.g., `"coreTools": ["run_shell_command(myapp)"]`) will override agent-level tool declarations, causing "tool not found" errors. Remove or expand the project-level restriction if agents need broader tool access (see issue #792).

## Scripts

Qwen Code does not have a dedicated `.qwen/scripts/` directory convention. Scripts are stored inside individual skill directories:

```
my-skill/
├── SKILL.md
└── scripts/
    └── helper.sh
```

For project automation outside the agent, Qwen Code supports headless mode (`qwen -p "prompt"`) suitable for CI/CD and shell scripting. The environment variable `QWEN_CODE=1` is set during `!` shell command execution.

## Sources

- [GitHub: QwenLM/qwen-code](https://github.com/QwenLM/qwen-code)
- [Official Documentation](https://qwenlm.github.io/qwen-code-docs/)
- [Skills Documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/)
- [SubAgents Documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/sub-agents/)
- [Commands Documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- [Settings / Configuration](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [npm: @qwen-code/qwen-code](https://www.npmjs.com/package/@qwen-code/qwen-code)
- [Issue #1512: Global skills directory detection bug](https://github.com/QwenLM/qwen-code/issues/1512)
- [Issue #1562: VS Code experimental-skills flag](https://github.com/QwenLM/qwen-code/issues/1562)
- [Issue #1695: .agents/skills support request](https://github.com/QwenLM/qwen-code/issues/1695)
- [Issue #792: Subagent tool not found](https://github.com/QwenLM/qwen-code/issues/792)
- [Discussion #1431: Agent skills in Qwen](https://github.com/QwenLM/qwen-code/discussions/1431)
- [Release v0.6.0: Experimental skills introduced](https://github.com/QwenLM/qwen-code/releases/tag/v0.6.0)
- [DeepWiki: Custom Commands](https://deepwiki.com/QwenLM/qwen-code/6.5-custom-commands)
- [Qwen3-Coder Blog Post](https://qwenlm.github.io/blog/qwen3-coder/)
