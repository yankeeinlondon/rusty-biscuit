---
homepage: https://www.anthropic.com/claude-code
docs: https://code.claude.com/docs/en/overview
skills: https://code.claude.com/docs/en/skills
agent: https://code.claude.com/docs/en/sub-agents
slash: https://code.claude.com/docs/en/slash-commands
scripts: https://code.claude.com/docs/en/hooks
---

# Claude Code Cross Referencing

## Skills

Claude Code supports agent skills and follows the Agent Skills open standard, with Claude-specific extensions (invocation controls, subagent execution, dynamic context injection).

### Scope and filesystem locations

| Scope | Location | Notes |
|---|---|---|
| Enterprise | Managed settings | Organization-wide distribution |
| User | `~/.claude/skills/<skill-name>/SKILL.md` | Available across all projects |
| Project | `.claude/skills/<skill-name>/SKILL.md` | Repo-scoped, team-shareable |
| Plugin | `<plugin>/skills/<skill-name>/SKILL.md` | Available when plugin is enabled |

Precedence for same skill name is documented as: enterprise > personal > project. Plugin skills are namespaced as `plugin-name:skill-name` to avoid collisions.

Claude Code also auto-discovers nested project skills (for example, `packages/frontend/.claude/skills/...`) and loads skills from `--add-dir` directories.

### Prerequisites

- No plugin is required for personal or project skills.
- Plugin skills require that plugin to be installed/enabled.
- Enterprise skills require managed settings/admin distribution.

### Best practices explicitly documented

- Write a strong `description` so Claude can route correctly.
- Keep `SKILL.md` focused and under 500 lines; move detail to supporting files.
- Put reusable references/examples/scripts beside `SKILL.md` and link to them from `SKILL.md`.
- Use `disable-model-invocation: true` for side-effect commands you only want to run manually.
- Use `user-invocable: false` for background knowledge that should not appear in the `/` menu.

### Skill frontmatter fields

Current Claude Code docs describe skill frontmatter as configurable with no hard-required field; `description` is recommended.

| Field | Required | Purpose |
|---|---|---|
| `name` | No | Display name; defaults to directory name if omitted |
| `description` | Recommended | Routing signal for automatic invocation |
| `argument-hint` | No | Autocomplete hint for expected arguments |
| `disable-model-invocation` | No | Prevent model auto-invocation (`true`) |
| `user-invocable` | No | Hide from user slash menu when `false` |
| `allowed-tools` | No | Tool allowlist while skill is active |
| `model` | No | Model override while skill is active |
| `context` | No | `fork` runs in forked subagent context |
| `agent` | No | Subagent type used with `context: fork` |
| `hooks` | No | Hook handlers scoped to this skill |

### First introduction (date/version)

- Agent Skills were publicly introduced on **October 16, 2025**.
- Claude Code docs now treat skills as first-class and note that custom slash commands were merged into skills.
- The current public docs/changelog do not clearly publish a single "first Claude Code version number" for skills in one canonical place.

Because this file is about Claude Code itself, the "does it read Claude directories" compatibility note is not applicable.

## Slash Commands

Claude Code supports built-in slash commands and custom slash commands. Current docs frame custom slash commands as part of the skills system.

### Built-in slash commands

Built-in commands are documented in Interactive Mode (examples include `/help`, `/compact`, `/model`, `/permissions`, `/memory`, `/agents`, `/status`, `/tasks`, `/usage`).

### Custom slash commands and directories

- Current guidance: use skills (`.claude/skills/<name>/SKILL.md`).
- Backward compatibility: `.claude/commands/*.md` still works and uses the same frontmatter model.
- Legacy slash-command scope is project and user (project `.claude/commands/`, user `~/.claude/commands/`), with user-level subdirectory behavior explicitly referenced in changelog fixes.
- Legacy command namespacing from subdirectories is documented in changelog history:
  - `.claude/commands/frontend/component.md` -> `/frontend:component`
  - user-level command subdirectory handling was explicitly fixed in later releases.

### Subdirectory behavior

- For skills: nested `.claude/skills/` directories are auto-discovered when working inside subdirectories.
- For legacy `.claude/commands/`: namespaced command behavior via subdirectories is documented in changelog entries; newer docs focus on skills as the recommended path.

### Slash/custom-command frontmatter

Because custom slash commands are merged into skills and `.claude/commands` "supports the same frontmatter", these are the active fields:

| Field | Required | Purpose |
|---|---|---|
| `name` | No | Command/skill name (defaults from path) |
| `description` | Recommended | Primary routing and menu metadata |
| `argument-hint` | No | UI hint for arguments |
| `disable-model-invocation` | No | Block model-side invocation |
| `user-invocable` | No | Hide from slash menu |
| `allowed-tools` | No | Tool allowlist while active |
| `model` | No | Model override |
| `context` | No | Forked execution context |
| `agent` | No | Subagent target for forked context |
| `hooks` | No | Lifecycle hooks |

## Agent / Subagents

Claude Code supports subagents and distinguishes them from agent teams.

### Vernacular and model

- **Subagents**: specialized assistants within one Claude Code session.
- **Agent teams**: multiple agents across separate sessions for more persistent parallel collaboration.

### Scope and locations

| Scope | Location | Priority |
|---|---|---|
| Session-only | `--agents` CLI JSON | Highest |
| Project | `.claude/agents/` | High |
| User | `~/.claude/agents/` | Lower |
| Plugin | `<plugin>/agents/` | Lowest |

When names collide, higher-priority scope wins.

### Orchestration behavior

- Claude can delegate automatically based on user task + subagent `description`.
- You can explicitly request a specific subagent.
- Subagents run foreground (blocking) or background (concurrent).
- Claude can run parallel independent subagent investigations and synthesize results.
- Subagents cannot spawn subagents; for multi-agent communication across separate sessions, use agent teams.

### Subagent frontmatter

Subagent docs explicitly require two fields.

| Field | Required | Purpose |
|---|---|---|
| `name` | Yes | Unique subagent identifier |
| `description` | Yes | Delegation trigger description |
| `tools` | No | Tool allowlist |
| `disallowedTools` | No | Tool denylist |
| `model` | No | `sonnet`, `opus`, `haiku`, or `inherit` |
| `permissionMode` | No | Permission mode override |
| `maxTurns` | No | Max agentic turns |
| `skills` | No | Skills preloaded into subagent context |
| `mcpServers` | No | MCP server availability for this subagent |
| `hooks` | No | Hook lifecycle handlers |
| `memory` | No | Persistent memory scope (`user`/`project`/`local`) |

## Scripts

Claude Code does not document one single global "scripts folder" feature. Instead, scripts appear in two primary patterns:

1. Skill-local scripts

- Place scripts under a skill directory (for example, `<skill>/scripts/`).
- Reference them from `SKILL.md`.
- These scripts are executed as utilities; they are not auto-injected as prompt text.

2. Hook commands (automation scripts)

- Configure shell commands in hooks via settings files and/or component frontmatter hooks.
- Typical project pattern is `.claude/hooks/...` scripts invoked from hook config.
- `CLAUDE_PROJECT_DIR` is provided for reliable project-relative script paths.

Hook/script scope can be user, project, local project, managed policy, plugin, or skill/agent-scoped hooks.

## Sources

- [Claude Code product homepage](https://www.anthropic.com/claude-code)
- [Claude Code docs overview](https://code.claude.com/docs/en/overview)
- [Skills documentation](https://code.claude.com/docs/en/skills)
- [Slash commands documentation URL (currently merged with skills docs)](https://code.claude.com/docs/en/slash-commands)
- [Interactive mode (built-in command reference)](https://code.claude.com/docs/en/interactive-mode)
- [Subagents documentation](https://code.claude.com/docs/en/sub-agents)
- [Hooks reference](https://code.claude.com/docs/en/hooks)
- [Common workflows (subagent usage examples)](https://code.claude.com/docs/en/tutorials)
- [Claude Code changelog (GitHub)](https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md)
- [Claude Code changelog (raw)](https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md)
- [Introducing Agent Skills (Claude blog, October 16, 2025)](https://claude.com/blog/skills)
- [Claude Developer Platform release notes (Agent Skills launch entry)](https://platform.claude.com/docs/en/release-notes/overview)
- [Agent Skills specification](https://agentskills.io/specification)
