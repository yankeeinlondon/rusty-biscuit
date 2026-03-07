---
homepage: https://openai.com/codex/
docs: https://developers.openai.com/codex/cli/
skills: https://developers.openai.com/codex/skills/
agent: https://developers.openai.com/codex/multi-agent
slash: https://developers.openai.com/codex/cli/slash-commands/
scripts: N/A (scripts live inside skill directories; no dedicated scripts mechanism)
---

# OpenAI Codex CLI

OpenAI Codex CLI is an open-source, Rust-based coding agent that runs locally in your terminal. It can read, change, and run code on your machine. Codex is included with ChatGPT Plus, Pro, Business, Edu, and Enterprise plans. Configuration lives in `~/.codex/config.toml` (TOML format).

Repository: https://github.com/openai/codex

---

## Skills

Codex CLI supports the [open agent skills standard](https://agentskills.io/specification). A skill is a directory containing a required `SKILL.md` file plus optional supporting resources. Skills were first introduced experimentally on **2 December 2025** via [PR #7412](https://github.com/openai/codex/pull/7412), initially behind a `--enable skills` feature flag. The feature request originated in [issue #5291](https://github.com/openai/codex/issues/5291) (opened 17 October 2025). As of early 2026, skills are enabled by default and no longer require a feature flag.

### Directory discovery

Codex scans for skills at multiple scope levels. Within each scope, it looks for directories containing a `SKILL.md` file.

| Scope | Path | Description |
|-------|------|-------------|
| REPO (CWD) | `$CWD/.agents/skills/` | Folder-specific skills |
| REPO (parent) | `$CWD/../.agents/skills/` | Parent directory skills |
| REPO (root) | `$REPO_ROOT/.agents/skills/` | Repository-wide skills |
| USER | `$HOME/.agents/skills/` | Personal global skills |
| ADMIN | `/etc/codex/skills/` | System-level administrator skills |
| SYSTEM | Bundled with Codex | Built-in skills from OpenAI (e.g., `plan`, `skill-creator`) |

**Legacy path**: `~/.codex/skills/` was the original user-scope path and is still supported for backward compatibility. The current documentation standardizes on `~/.agents/skills/` for user scope.

When naming conflicts occur across scopes, both skills appear in the skill selector without merging.

### Does Codex read Claude Code skill directories?

No. Codex does **not** scan `.claude/skills/` at either user or repo scope. It only scans `.agents/skills/` (repo scope) and `~/.agents/skills/` or `~/.codex/skills/` (user scope). To share skills between Claude Code and Codex, use symlinks:

```bash
# Symlink a Claude Code skill into the Codex user skills directory
ln -s ~/.claude/skills/my-skill ~/.agents/skills/my-skill
```

Codex supports symlinked skill **directories** (not individual files). The entire skill directory must be symlinked, not just the `SKILL.md` file.

### Skill directory structure

```
my-skill/
├── SKILL.md           # Required: instructions with YAML frontmatter
├── scripts/           # Optional: executable utilities
├── references/        # Optional: supporting documentation
├── assets/            # Optional: images, templates, data files
└── agents/
    └── openai.yaml    # Optional: UI metadata and policy
```

### SKILL.md frontmatter

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Skill identifier (1-64 chars, lowercase alphanumeric with hyphens) |
| `description` | Yes | When the skill should/shouldn't trigger (1-1024 chars). Critical for implicit invocation |

No other frontmatter fields are documented as supported by Codex. The open agent skills standard defines additional optional fields (`license`, `compatibility`, `metadata`) but Codex's documentation only requires `name` and `description`.

**Example:**

```yaml
---
name: git-release
description: Create consistent releases and changelogs. Use when the user asks to cut a release, draft release notes, or bump versions.
---

## Instructions
- Draft release notes from merged PRs
- Propose a semantic version bump
- Generate CHANGELOG entries
```

### Optional UI metadata (agents/openai.yaml)

This file provides UI integration and behavioral policy for the Codex app and IDE extension:

```yaml
interface:
  display_name: "User-facing name"
  short_description: "Brief description"
  icon_small: "./assets/small-logo.svg"
  icon_large: "./assets/large-logo.png"
  brand_color: "#3B82F6"
  default_prompt: "Optional surrounding prompt"

policy:
  allow_implicit_invocation: false  # default: true

dependencies:
  tools:
    - type: "mcp"
      value: "toolName"
      description: "Tool description"
```

Setting `allow_implicit_invocation: false` prevents automatic triggering while preserving explicit `$skill-name` invocation.

### Loading mechanism (progressive disclosure)

1. At startup, Codex loads only metadata: `name`, `description`, file path, and optional `openai.yaml` data
2. Full `SKILL.md` instructions are loaded only when Codex decides to use a skill
3. Codex detects new skills automatically; restart if changes to existing skills don't appear

### Invocation

- **Explicit**: Use `$skill-name` syntax in the composer, or the `/skills` command to browse
- **Implicit**: Codex automatically selects skills when user tasks match the skill's `description` field

### Disabling a skill (without deleting it)

```toml
# ~/.codex/config.toml
[[skills.config]]
path = "/path/to/skill-directory"
enabled = false
```

### Built-in system skills

Codex ships with two system skills in `~/.codex/skills/.system/`:

- **plan**: Manages implementation planning for multi-step work
- **skill-creator**: Guided skill builder (invoke via `$skill-creator`)

An additional `$skill-installer` command can install skills from external repositories.

### Key differences from Claude Code skills

| Aspect | Codex CLI | Claude Code |
|--------|-----------|-------------|
| Repo skill path | `.agents/skills/` | `.claude/skills/` |
| User skill path | `~/.agents/skills/` (or `~/.codex/skills/`) | `~/.claude/skills/` |
| Cross-reading | Does not read `.claude/skills/` | Does not read `.agents/skills/` |
| Required frontmatter | `name` + `description` | `description` only |
| UI metadata | `agents/openai.yaml` sidecar | Not applicable |
| Implicit invocation control | `allow_implicit_invocation` policy | Always implicit |
| Skill disabling | Via `config.toml` `[[skills.config]]` | Not supported natively |

---

## Slash Commands

Codex CLI provides built-in slash commands accessed by typing `/` in the TUI composer. Custom slash commands were previously supported via the "custom prompts" mechanism (`~/.codex/prompts/*.md`), but **custom prompts are now deprecated** in favor of skills.

### Built-in slash commands

| Command | Description |
|---------|-------------|
| `/model` | Switch models and reasoning effort levels |
| `/personality` | Change communication style (friendly, pragmatic, none) |
| `/permissions` | Adjust approval requirements (Auto, Read Only, etc.) |
| `/plan` | Enter plan mode with optional inline prompts |
| `/experimental` | Toggle experimental features (e.g., multi-agents) |
| `/status` | Display active model, approval policy, writable roots, token usage |
| `/debug-config` | Print config layer diagnostics and policy sources |
| `/statusline` | Customize TUI footer items |
| `/diff` | Show Git diff including untracked files |
| `/review` | Request working-tree analysis |
| `/compact` | Summarize conversation to conserve context tokens |
| `/mention` | Attach specific files/folders to conversation |
| `/mcp` | List Model Context Protocol tools |
| `/apps` | Browse available connectors/apps |
| `/ps` | Show background terminal status and recent output |
| `/new` | Start fresh conversation within same session |
| `/resume` | Reload previously saved conversation |
| `/fork` | Clone current conversation into new thread |
| `/agent` | Switch between active agent threads |
| `/init` | Generate `AGENTS.md` scaffold |
| `/feedback` | Submit diagnostics and logs |
| `/logout` | Clear local credentials |
| `/quit` / `/exit` | Exit the CLI |

Recent additions include `/m_update` and `/m_drop` for memory management.

### Does Codex read Claude Code's slash command directories?

No. Codex does not read `.claude/commands/` or `~/.claude/commands/`. There is no built-in mechanism for custom slash commands beyond the deprecated prompts feature (see below).

### Deprecated: Custom prompts (formerly custom slash commands)

Custom prompts were Markdown files stored in `~/.codex/prompts/` with YAML frontmatter. They are **deprecated** in favor of skills.

Current Codex documentation says custom prompts live in the local Codex home directory and **are not shared through the repository**. In practice, this means project-scoped `.codex/prompts/` files should not be relied on for discovery in current Codex builds.

**Frontmatter fields (deprecated):**

| Field | Description |
|-------|-------------|
| `description` | Shown in the command menu |
| `argument-hint` | Documents expected parameters (e.g., `[PARAM=<value>]`) |

**Argument placeholders (deprecated):**

- `$1` through `$9`: Positional arguments
- `$UPPERCASE_NAME`: Named arguments from `KEY=value` pairs
- `$ARGUMENTS`: All supplied values
- `$$`: Literal dollar sign

### Differences from Claude Code slash commands

| Aspect | Codex CLI | Claude Code |
|--------|-----------|-------------|
| Custom commands | Deprecated (use skills instead) | Supported via `.claude/commands/*.md` |
| Built-in commands | ~23 built-in commands | Built-in commands |
| File format | Was Markdown with YAML frontmatter (deprecated) | Markdown with YAML frontmatter |
| Storage scope | User-scoped `~/.codex/prompts/` only | User- and repo-scoped `.claude/commands/` |
| Subdirectory support | Deprecated prompt files may still use nested paths under user scope | Supported (subdirectories create namespaces) |
| Cross-reading | Does not read `.claude/commands/` | N/A |

---

## Agents / Subagents

Codex CLI supports multi-agent workflows as an **experimental** feature. When enabled, Codex can spawn specialized sub-agents in parallel and collect their results. Codex uses the term "agents" or "agent roles" rather than "subagents".

### Enabling multi-agent

Multi-agent must be explicitly enabled:

```toml
# ~/.codex/config.toml
[features]
multi_agent = true
```

Or toggle at runtime via `/experimental` in the TUI.

### How orchestration works

Codex handles orchestration automatically:

1. **Spawning**: Codex determines when to spawn sub-agents (or the user can request it explicitly)
2. **Parallel execution**: Multiple agents can run simultaneously on independent tasks
3. **Result collection**: Codex waits for all agents to finish, then consolidates results into one response
4. **Thread management**: Each agent runs in its own thread; use `/agent` to navigate between active threads

### Agent role configuration

Agent roles are defined in the `[agents]` section of `config.toml`:

```toml
# ~/.codex/config.toml

[agents]
max_threads = 4

[agents.explorer]
description = "Read-only codebase exploration agent"
sandbox_mode = "read-only"
model = "gpt-4.1-mini"

[agents.worker]
description = "Implementation agent for making code changes"
config_file = "worker-config.toml"  # Relative to the config file's directory
```

**Supported role fields:**

| Field | Description |
|-------|-------------|
| `description` | Guidance text for when to use this role |
| `config_file` | Path to a TOML config layer for the role |
| `model` | Model to use for this agent |
| `model_reasoning_effort` | Reasoning effort level |
| `sandbox_mode` | Access restrictions (read-only, workspace-write, etc.) |
| `developer_instructions` | Role-specific directive text |

Built-in roles include `default`, `worker`, and `explorer`. Custom roles override built-in definitions with the same name.

### Sandbox and approval behavior

- Sub-agents inherit the parent session's sandbox policy
- Sub-agents run with **non-interactive approvals**: if an action would require user approval, it fails and the error surfaces to the parent
- Individual agent roles can override sandbox settings (e.g., read-only for explorers)

### Key differences from Claude Code agents

| Aspect | Codex CLI | Claude Code |
|--------|-----------|-------------|
| Feature status | Experimental (requires `multi_agent = true`) | Stable (Task tool) |
| Configuration | TOML in `config.toml` `[agents]` section | Markdown files in `agents/` directory |
| Vernacular | "Agent roles" and "threads" | "Sub-agents" via Task tool |
| Definition format | TOML key-value pairs | Markdown with YAML frontmatter |
| Repo-level agents | Not yet supported (requested in [issue #11701](https://github.com/openai/codex/issues/11701)) | `.claude/agents/*.md` supported |
| Orchestration | Automatic (Codex decides when to spawn) | Manual (agent calls Task tool explicitly) |
| Parallel execution | Native parallel spawning | Multiple Task tool calls in one turn |
| Thread navigation | `/agent` command to switch threads | N/A (one-off delegations) |

### Current limitations

- Agent roles can only be configured globally in `~/.codex/config.toml`, not per-repository (requested in [issue #11701](https://github.com/openai/codex/issues/11701))
- No Markdown-based agent definition files (unlike Claude Code's `agents/*.md`)
- The feature is experimental and may change

---

## Scripts

Codex CLI does not have a dedicated scripts directory or discovery mechanism at the top level. Scripts are expected to live **inside skill directories** under a `scripts/` subdirectory:

```
my-skill/
├── SKILL.md
└── scripts/
    ├── build.sh
    └── validate.py
```

The `SKILL.md` file references these scripts, and Codex executes them via its shell tool when instructed by the skill's content.

### Notification hooks

Codex does support a `notify` configuration for triggering external programs on specific events:

```toml
# ~/.codex/config.toml
notify = ["python3", "/path/to/notify.py"]
```

The script receives a JSON payload with fields including `type`, `thread-id`, `turn-id`, `cwd`, `input-messages`, and `last-assistant-message`. The currently supported event is `agent-turn-complete`.

### Custom instructions

Codex uses `AGENTS.md` files (not scripts) for project-level instructions. The discovery hierarchy is:

1. **Global**: `~/.codex/AGENTS.override.md` then `~/.codex/AGENTS.md`
2. **Project**: Walks from Git root to current directory, checking `AGENTS.override.md` then `AGENTS.md` at each level
3. Fallback filenames configurable via `project_doc_fallback_filenames` in `config.toml`
4. Combined size capped by `project_doc_max_bytes` (default 32 KiB)

---

## Sources

- [OpenAI Codex CLI homepage](https://developers.openai.com/codex/cli/)
- [Agent Skills documentation](https://developers.openai.com/codex/skills/)
- [Slash commands reference](https://developers.openai.com/codex/cli/slash-commands/)
- [Multi-agent documentation](https://developers.openai.com/codex/multi-agent)
- [Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md/)
- [Configuration reference](https://developers.openai.com/codex/config-reference)
- [Advanced configuration](https://developers.openai.com/codex/config-advanced/)
- [Custom prompts (deprecated)](https://developers.openai.com/codex/custom-prompts/)
- [Codex CLI features](https://developers.openai.com/codex/cli/features/)
- [Codex changelog](https://developers.openai.com/codex/changelog?type=codex-cli)
- [GitHub repository](https://github.com/openai/codex)
- [PR #7412: Experimental skills support](https://github.com/openai/codex/pull/7412)
- [Issue #5291: SKILL.md support request](https://github.com/openai/codex/issues/5291)
- [Issue #9365: Symlinked SKILL.md](https://github.com/openai/codex/issues/9365)
- [Issue #11701: Subagent configuration and orchestration](https://github.com/openai/codex/issues/11701)
- [Issue #2604: Subagent support](https://github.com/openai/codex/issues/2604)
- [OpenAI Skills catalog](https://github.com/openai/skills)
- [Agent Skills open standard](https://agentskills.io/)
- [Agent Skills specification](https://agentskills.io/specification)
- [Blog: Skills in OpenAI Codex (fsck.com)](https://blog.fsck.com/2025/12/19/codex-skills/)
- [Blog: OpenAI quietly adopting skills (Simon Willison)](https://simonwillison.net/2025/Dec/12/openai-skills/)
