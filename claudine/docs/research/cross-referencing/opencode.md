---
homepage: https://opencode.ai
docs: https://opencode.ai/docs/
skills: https://opencode.ai/docs/skills/
agent: https://opencode.ai/docs/agents/
slash: https://opencode.ai/docs/commands/
scripts: https://opencode.ai/docs/custom-tools/
---

# OpenCode

OpenCode is an open-source AI coding agent by [Anomaly](https://github.com/anomalyco). It is written in TypeScript and provides a TUI, a desktop app, and an IDE extension. The GitHub repository is [anomalyco/opencode](https://github.com/anomalyco/opencode).

Configuration lives in `opencode.json` (project root) and `~/.config/opencode/opencode.json` (global). Both JSON and JSONC are accepted, with a `$schema` reference to `https://opencode.ai/config.json`.

The `.opencode/` and `~/.config/opencode/` directories use **plural** subdirectory names: `agents/`, `commands/`, `modes/`, `plugins/`, `skills/`, `tools/`, `themes/`. Singular names (e.g. `agent/`) are accepted for backward compatibility.

## Skills

OpenCode supports the Agent Skills open standard (`SKILL.md` in a named directory). Skills were introduced in **v1.0.190** (December 22, 2025) via the native `skill` tool with a permission system.

### Directory Locations

**Project-local** (searched from cwd up to git worktree root):

- `.opencode/skills/<name>/SKILL.md`
- `.claude/skills/<name>/SKILL.md`
- `.agents/skills/<name>/SKILL.md`

**Global**:

- `~/.config/opencode/skills/<name>/SKILL.md`
- `~/.claude/skills/<name>/SKILL.md`
- `~/.agents/skills/<name>/SKILL.md`

OpenCode reads Claude Code's skill directories (both `~/.claude/skills/` and `.claude/skills/`) natively. This can be disabled with `OPENCODE_DISABLE_CLAUDE_CODE=1`.

### How Skills Work

The agent has a built-in `skill` tool. All discovered skill names and descriptions are listed in the tool's description. When the agent decides a skill is relevant, it calls `skill({ name: "skill-name" })` to load the full SKILL.md content on demand (progressive disclosure).

Skills are **not** automatically loaded just by being present -- the agent must explicitly choose to call the `skill` tool based on the description match.

### Frontmatter Properties

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | 1--64 chars, lowercase alphanumeric with single hyphens (`^[a-z0-9]+(-[a-z0-9]+)*$`). Must match directory name. |
| `description` | Yes | 1--1024 chars. Used by the agent to decide when to load the skill. |
| `license` | No | SPDX license identifier (e.g. `MIT`). |
| `compatibility` | No | 1--500 chars describing environment requirements. |
| `metadata` | No | String-to-string map for custom key-value pairs. |

### Permissions

Skill access is controlled via pattern-based permissions in `opencode.json`:

```json
{
  "permission": {
    "skill": {
      "*": "allow",
      "internal-*": "deny",
      "dangerous-deploy": "ask"
    }
  }
}
```

Values are `allow` (immediate access), `deny` (hidden from agent), or `ask` (user approval required).

### Best Practices

- Keep `SKILL.md` under 500 lines; move detailed content to linked reference files.
- Write descriptions in the format: `<what it does>. Use when <specific triggers>`.
- Skill names must be unique across all discovery locations.

## Slash Commands

OpenCode supports custom slash commands. Custom commands have existed since at least **v0.0.49** (May 2025).

### Directory Locations

- **Project**: `.opencode/commands/*.md`
- **Global**: `~/.config/opencode/commands/*.md`

The filename becomes the command name (e.g. `test.md` -> `/test`).

### Claude Code Compatibility

OpenCode does **not** natively discover `.claude/commands/` directories. This is tracked in [GitHub issue #6985](https://github.com/anomalyco/opencode/issues/6985) (still open). Users migrating from Claude Code must copy or symlink their command files:

```bash
cp -r ~/.claude/commands/* ~/.config/opencode/commands/
```

There are also frontmatter incompatibilities between the two platforms (Claude Code uses `argument-hint` and `allowed-tools` which OpenCode does not recognize).

### Frontmatter Properties

| Field | Required | Description |
|-------|----------|-------------|
| `description` | No | Shown in TUI autocomplete. |
| `template` | No | Prompt template sent to the LLM. If omitted, the Markdown body is used. |
| `agent` | No | Which agent executes the command (e.g. `build`, `plan`). |
| `model` | No | Override default model for this command. |
| `subtask` | No | Forces subagent invocation. |

### Prompt Placeholders

- `$ARGUMENTS` -- all arguments passed after the command name
- `$1`, `$2`, `$3` -- individual positional arguments
- `` !`command` `` -- inline bash command output
- `@filename` -- file content inclusion

### JSON Alternative

Commands can also be defined in `opencode.json`:

```json
{
  "command": {
    "test": {
      "template": "Run the test suite and report failures.",
      "description": "Run tests"
    }
  }
}
```

### Built-in Slash Commands

| Command | Aliases | Shortcut | Description |
|---------|---------|----------|-------------|
| `/compact` | `/summarize` | `ctrl+x c` | Condense the current session |
| `/connect` | | | Add a provider and configure API keys |
| `/details` | | `ctrl+x d` | Toggle tool execution detail display |
| `/editor` | | `ctrl+x e` | Open external editor for message composition |
| `/exit` | `/quit`, `/q` | `ctrl+x q` | Exit OpenCode |
| `/export` | | `ctrl+x x` | Export conversation to Markdown |
| `/help` | | `ctrl+x h` | Show help dialog |
| `/init` | | `ctrl+x i` | Create or update `AGENTS.md` |
| `/models` | | `ctrl+x m` | List available models |
| `/new` | `/clear` | `ctrl+x n` | Start a new session |
| `/redo` | | `ctrl+x r` | Restore undone message and file changes |
| `/sessions` | `/resume`, `/continue` | `ctrl+x l` | List and switch between sessions |
| `/share` | | `ctrl+x s` | Share current session |
| `/themes` | | `ctrl+x t` | List available themes |
| `/thinking` | | | Toggle model reasoning block visibility |
| `/undo` | | `ctrl+x u` | Remove last message and revert file changes |
| `/unshare` | | | Stop sharing current session |

Slash commands are TUI-only; they cannot be used via `opencode run` on the command line.

## Agents / Subagents

OpenCode has a rich agent system. The task tool for subagent delegation has existed since at least **v0.1.99** (June 2025). Custom agent definitions (markdown-based) have been available since early versions, with `AGENTS.md` support present since **v0.1.55** (June 2025).

### Vernacular

OpenCode uses the terms **agent** (for primary agents) and **subagent** (for delegated workers). Primary agents are switched with the `Tab` key. Subagents are invoked via the `task` tool or `@mention` syntax.

### Directory Locations

- **Project**: `.opencode/agents/*.md`
- **Global**: `~/.config/opencode/agents/*.md`

The filename becomes the agent identifier.

### Built-in Agents

| Agent | Mode | Description |
|-------|------|-------------|
| **Build** | primary | Full tool access for development work (default) |
| **Plan** | primary | Restricted mode for planning without modifications |
| **General** | subagent | Multi-step research with broad capabilities |
| **Explore** | subagent | Fast, read-only codebase exploration |
| **Compaction** | hidden | System agent for session compaction |
| **Title** | hidden | System agent for session title generation |
| **Summary** | hidden | System agent for summarization |

### Frontmatter Properties

| Field | Required | Description |
|-------|----------|-------------|
| `description` | Yes | Brief purpose statement. |
| `mode` | No | `primary`, `subagent`, or `all` (default: `all`). |
| `model` | No | Override model (e.g. `anthropic/claude-sonnet-4-20250514`). |
| `temperature` | No | Sampling temperature (0.0--1.0). |
| `top_p` | No | Response diversity (0.0--1.0). |
| `tools` | No | Object mapping tool names to booleans (e.g. `write: false`). |
| `permission` | No | Pattern-based permissions (`ask`, `allow`, `deny`). |
| `steps` | No | Maximum agentic iterations. |
| `color` | No | Hex color or theme name for UI display. |
| `hidden` | No | Boolean -- hide from `@` autocomplete. |
| `disable` | No | Boolean -- deactivate the agent entirely. |
| `prompt` | No | Custom system prompt (file path or inline text). |

### Interaction Model: Orchestrator to Subagent

1. **Primary agent** receives user request and decides to delegate.
2. **Delegation**: primary agent calls the `task` tool with a target subagent name and prompt.
3. **Subagent execution**: OpenCode creates a new isolated child session. The subagent has:
   - A fresh context (no access to parent conversation history).
   - Its own system prompt, tools, and model configuration.
   - Independent tool permissions.
4. **Result return**: the subagent's final text output is returned to the parent as the `task` tool result.
5. **Continuation**: the primary agent integrates the result.

Each subagent invocation is **stateless** -- you cannot send follow-up messages to a running subagent.

### Concurrency

OpenCode supports **parallel subagent execution** by issuing multiple `task` tool calls in a single assistant message. There is no true "fire-and-forget" background execution (child sessions are awaited).

Navigate between parent and child sessions using:

- `Leader+Right`: cycle forward (parent -> child1 -> child2 -> parent)
- `Leader+Left`: cycle backward

### Task Permissions

```json
{
  "permission": {
    "task": {
      "*": "deny",
      "code-reviewer": "allow",
      "deploy-*": "ask"
    }
  }
}
```

Rules evaluate in order; the **last match wins**. Users can manually invoke any subagent via `@mention` regardless of task permissions.

### Differences from Claude Code

| Aspect | OpenCode | Claude Code |
|--------|----------|-------------|
| **Agent definition** | Markdown files in `agents/` or JSON in `opencode.json` | Markdown files in `agents/` directory |
| **Mode system** | Explicit `mode` field (`primary` / `subagent` / `all`) | Agents are always subagents invoked via Task tool |
| **Primary agent switching** | Tab key cycles between primary agents | Single primary agent (Claude itself) |
| **Delegation tool** | `task` tool (same concept) | `Task` tool |
| **Context isolation** | Child sessions with independent context | Same isolation model |
| **Built-in agents** | Build, Plan, General, Explore + hidden system agents | No built-in named agents |

## Scripts

OpenCode does not have a single dedicated "scripts" directory convention. Instead, it provides three extensibility mechanisms for executable code:

### 1. Custom Tools (`tools/` directory)

The primary mechanism for executable code. Tools are defined as TypeScript or JavaScript files that can invoke scripts in any language.

- **Project**: `.opencode/tools/`
- **Global**: `~/.config/opencode/tools/`

The filename becomes the tool name. Multi-export files use `<filename>_<exportname>` naming.

### 2. Plugins (`plugins/` directory)

Plugins are JS/TS modules that hook into OpenCode's event lifecycle (command, file, message, permission, session, tool events, etc.).

- **Project**: `.opencode/plugins/`
- **Global**: `~/.config/opencode/plugins/`

Plugins can also be installed from npm via `opencode.json` configuration.

### 3. Skill-bundled Scripts

Skills can include a `scripts/` subdirectory alongside `SKILL.md`. The agent discovers these via the skill content and can execute them through the `bash` tool.

```
my-skill/
  SKILL.md
  scripts/
    deploy.sh
    validate.py
```

## Sources

- [OpenCode Homepage](https://opencode.ai/)
- [OpenCode Documentation](https://opencode.ai/docs/)
- [OpenCode GitHub Repository](https://github.com/anomalyco/opencode)
- [Agent Skills Documentation](https://opencode.ai/docs/skills/)
- [Agents Documentation](https://opencode.ai/docs/agents/)
- [Commands Documentation](https://opencode.ai/docs/commands/)
- [Custom Tools Documentation](https://opencode.ai/docs/custom-tools/)
- [Plugins Documentation](https://opencode.ai/docs/plugins/)
- [Tools Documentation](https://opencode.ai/docs/tools/)
- [Config Documentation](https://opencode.ai/docs/config/)
- [Rules Documentation](https://opencode.ai/docs/rules/)
- [TUI Documentation](https://opencode.ai/docs/tui/)
- [CLI Documentation](https://opencode.ai/docs/cli/)
- [OpenCode Changelog](https://opencode.ai/changelog)
- [GitHub Issue #6985 -- .claude/commands/ compatibility](https://github.com/anomalyco/opencode/issues/6985)
- [GitHub Issue #12604 -- Disable Claude Code sync](https://github.com/anomalyco/opencode/issues/12604)
- [GitHub Issue #3235 -- Skills support request](https://github.com/anomalyco/opencode/issues/3235)
- [Release v1.0.190 -- Native skill tool introduced](https://github.com/anomalyco/opencode/releases/tag/v1.0.190)
