---
homepage: https://www.kimi.com/code
docs: https://moonshotai.github.io/kimi-cli/en/
skills: https://moonshotai.github.io/kimi-cli/en/customization/skills.html
agent: https://moonshotai.github.io/kimi-cli/en/customization/agents.html
slash: https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html
scripts: Not a standalone feature; scripts live inside skill directories under scripts/
---

# Kimi Code CLI: Skills, Slash Commands, Agents, Scripts

Kimi Code CLI (by Moonshot AI) is a Python-based terminal agent for software development tasks. It is open source (Apache-2.0) at [MoonshotAI/kimi-cli](https://github.com/MoonshotAI/kimi-cli). As of February 2026, the latest release is **v1.12.0** (2026-02-11).

Kimi Code CLI has strong cross-tool compatibility: its skill discovery system intentionally checks directories belonging to Claude Code, Codex, and a shared `.agents/` convention, so skills authored for one tool are often found by the others without modification.

---

## Skills

**Supported:** Yes. Kimi Code CLI supports the Agent Skills open format (a directory containing a `SKILL.md` entry point). Skills are discovered at startup; their names, paths, and descriptions are injected into the system prompt. The agent autonomously decides whether to open and read a skill based on the current task.

**First introduced:** v0.75 (2026-01-09) added built-in skills (`kimi-cli-help`). v0.79 (2026-01-19) added project-level skill discovery and unified layered loading across built-in, user, and project scopes.

### Directory discovery

Skills use a **layered loading** model. Later layers override same-named skills from earlier layers.

**Layer 1 -- Built-in skills** (shipped with the package):
- `kimi-cli-help` -- CLI usage and configuration reference
- `skill-creator` -- guidance for authoring new skills

**Layer 2 -- User-level skills** (first existing directory wins):
1. `~/.config/agents/skills/` (recommended)
2. `~/.agents/skills/`
3. `~/.kimi/skills/`
4. `~/.claude/skills/`
5. `~/.codex/skills/`

**Layer 3 -- Project-level skills** (first existing directory wins):
1. `.agents/skills/` (recommended)
2. `.kimi/skills/`
3. `.claude/skills/`
4. `.codex/skills/`

**Override:** `--skills-dir /path/to/skills` skips user-level and project-level discovery entirely.

`KIMI_SHARE_DIR` (which changes the runtime data directory from `~/.kimi/`) does **not** affect skill discovery paths. Skill paths are intentionally separate from runtime data.

### Cross-tool compatibility

Because Kimi checks `~/.claude/skills/` and `.claude/skills/`, it **does** read Claude Code's skill directories. It also checks Codex directories. The "first existing directory wins" rule means that if `~/.config/agents/skills/` exists, `~/.claude/skills/` at the user level is skipped. To ensure Kimi reads Claude Code's directories, either:
- Do not create a higher-priority directory, or
- Use `--skills-dir ~/.claude/skills` to force it, or
- Symlink or consolidate skills into the recommended `~/.config/agents/skills/` directory.

### Skill directory structure

Minimal:
```
<skill-name>/
└── SKILL.md
```

Recommended:
```
<skill-name>/
├── SKILL.md        # Required entry point
├── scripts/        # Optional executable scripts
├── references/     # Optional reference documents
└── assets/         # Optional supporting files
```

### Frontmatter properties

`SKILL.md` begins with YAML frontmatter followed by Markdown content. All fields are optional:

| Field           | Required | Constraints                              | Default                     |
|-----------------|----------|------------------------------------------|-----------------------------|
| `name`          | No       | 1-64 chars; lowercase letters, numbers, hyphens only | Directory name              |
| `description`   | No       | 1-1024 chars; shown in the skill list    | "No description provided."  |
| `license`       | No       | License name or file reference           | --                          |
| `compatibility` | No       | Environment requirements; max 500 chars  | --                          |
| `metadata`      | No       | Arbitrary key-value pairs                | --                          |
| `type`          | No       | Set to `flow` for flow skills            | (standard skill)            |

### Skill invocation

- `/skill:<name>` -- loads the SKILL.md content as a prompt. Additional text can be appended: `/skill:<name> refactor the auth module`.
- `/flow:<name>` -- executes a flow skill as a multi-step automation (see below). The same skill can also be loaded as a plain prompt via `/skill:<name>`.

### Flow skills

Introduced in v0.81 (2026-01-20). Flow skills embed a Mermaid or D2 flow diagram in `SKILL.md` and set `type: flow` in frontmatter.

Requirements:
- Diagram must have exactly one `BEGIN` node and one `END` node.
- Decision nodes require the agent to emit `<choice>branch name</choice>` to select the next step.
- Regular node text is sent to the agent as a prompt at each step.

v0.81 was a **breaking change**: the `--prompt-flow` CLI option and `/begin` slash command were removed in favor of `/flow:<name>`.

### Best practices

- Consolidate skills into the highest-priority directory per scope to avoid confusion about which directory is active.
- Match the directory name to the `name` frontmatter field (or omit `name` to auto-derive from the directory).
- Keep `SKILL.md` compact and link to supporting documents in subdirectories.
- Validate Mermaid/D2 diagrams in a playground before using them as flow skills.

---

## Slash Commands

**Custom slash commands:** Not supported. Kimi Code CLI does not document a mechanism for user-defined slash command files (no `commands/` directory convention). Skills with `/skill:<name>` serve a similar role to Claude Code's custom slash commands.

**Built-in slash commands:**

Help and info:
- `/help` (aliases: `/h`, `/?`) -- shows keyboard shortcuts, slash commands, and loaded skills
- `/version` -- displays CLI version
- `/changelog` (alias: `/release-notes`) -- shows recent version changelog
- `/feedback` -- opens GitHub Issues

Account and configuration:
- `/login` (alias: `/setup`) -- log in or configure an API platform (only with default config file)
- `/logout` -- clear stored credentials
- `/model` -- switch models and thinking mode (only with default config file)
- `/reload` -- reload configuration without exiting
- `/debug` -- show context debug info (messages, tokens, checkpoints, history)
- `/usage` (alias: `/status`) -- API quota and usage (Kimi Code platform only)
- `/mcp` -- show connected MCP servers and loaded tools

Session management:
- `/sessions` (alias: `/resume`) -- list and switch sessions in current directory
- `/clear` (alias: `/reset`) -- clear context, start new conversation
- `/compact` -- manually compact context to reduce token usage

Skills:
- `/skill:<name>` -- load a skill as a prompt
- `/flow:<name>` -- execute a flow skill

Other:
- `/init` -- analyze project and generate `AGENTS.md`
- `/yolo` -- toggle auto-approve mode

**Shell mode** (entered via Ctrl-X) only supports: `/help`, `/exit`, `/version`, `/changelog`, `/feedback`.

**First introduced:** `/version` appeared in v0.33 (2025-10-21). Skill-related commands (`/skill:<name>`) added in v0.76 (2026-01-12).

### Comparison with Claude Code

Claude Code supports user-defined slash commands via `~/.claude/commands/*.md` and `.claude/commands/*.md`, where each Markdown file becomes a `/command-name`. Kimi Code CLI has no equivalent -- its `/skill:<name>` invocation is the closest analog. If you need a quick prompt template without the full skill directory structure, Kimi does not currently offer that shortcut; you must create a skill directory with a `SKILL.md`.

---

## Agents / Subagents

**Supported:** Yes. Kimi Code CLI has a YAML-based agent definition system with support for both built-in and custom agents, including subagent delegation.

**First introduced:** v0.23 (2025-10-09) added custom agent YAML files and `AGENTS.md` support. v0.62 (2025-12-08) added built-in agents (`default`, `okabe`) selectable via `--agent`.

### Vernacular

Kimi uses **"agent"** for the top-level orchestrator and **"subagent"** for delegated workers. The coordination layer is called the **LaborMarket**. Subagents are spawned via the **Task** tool. There is also an optional **CreateSubagent** tool for dynamic runtime subagent creation.

### Built-in agents

- **`default`** -- general-purpose agent with tools: Task, Shell, ReadFile, WriteFile, SearchWeb, FetchURL, Glob, Grep, and others.
- **`okabe`** -- experimental agent extending `default` with `SendDMail`.

Load with: `kimi --agent default` or `kimi --agent okabe`.

### Custom agent file format

Load with: `kimi --agent-file /path/to/my-agent.yaml`

```yaml
version: 1
agent:
  name: my-agent
  system_prompt_path: ./system.md     # Relative to agent file
  tools:
    - "kimi_cli.tools.shell:Shell"
    - "kimi_cli.tools.file:ReadFile"
    - "kimi_cli.tools.file:WriteFile"
```

**Required fields** (unless inherited via `extend`):

| Field                | Description                                            |
|----------------------|--------------------------------------------------------|
| `name`               | Agent identifier                                       |
| `system_prompt_path` | Path to Markdown system prompt template                |
| `tools`              | List of tool paths in `module:ClassName` format        |

**Optional fields:**

| Field                | Description                                            |
|----------------------|--------------------------------------------------------|
| `extend`             | Inherit from `default` or a relative agent file path   |
| `system_prompt_args` | Key-value pairs for `${VAR}` expansion in the prompt   |
| `exclude_tools`      | Tools to remove from an inherited list                 |
| `subagents`          | Map of named subagent definitions                      |

### System prompt variables

Built-in variables available in prompt templates:

| Variable              | Description                              |
|-----------------------|------------------------------------------|
| `${KIMI_NOW}`         | Current time (ISO format)                |
| `${KIMI_WORK_DIR}`    | Working directory path                   |
| `${KIMI_WORK_DIR_LS}` | File listing of working directory        |
| `${KIMI_AGENTS_MD}`   | Content of `AGENTS.md` (if it exists)    |
| `${KIMI_SKILLS}`      | Loaded skills list                       |

Custom variables are defined via `system_prompt_args` and referenced as `${MY_VAR}`.

### Subagent definitions

Subagents are declared inside the main agent file:

```yaml
subagents:
  coder:
    path: ./coder-sub.yaml
    description: "Handle coding tasks"
  reviewer:
    path: ./reviewer-sub.yaml
    description: "Code review expert"
```

Subagent files are standard agent YAML. Common patterns:
- `extend` the main agent to inherit its configuration.
- Add role-specific `system_prompt_args`.
- Exclude `Task` from `exclude_tools` to prevent nested subagent spawning (avoid recursion).

### Dynamic subagent creation

The `CreateSubagent` tool (not enabled by default) allows runtime subagent definition:

```yaml
agent:
  tools:
    - "kimi_cli.tools.multiagent:CreateSubagent"
```

Parameters: `name` (unique identifier), `system_prompt` (role definition), `tools` (tool paths). Dynamic subagents share the main agent's LaborMarket, so they can delegate to other subagents including fixed ones. Fixed subagents receive isolated runtimes.

### Execution model

- Subagents run via the **Task** tool in **isolated contexts**.
- They do **not** share the main agent's conversation history; prompts must be self-contained.
- Results are returned to the orchestrator when complete.
- Independent subagents can run in **parallel** (separate Task calls).

### Comparison with Claude Code

| Aspect                | Kimi Code CLI                                    | Claude Code                                     |
|-----------------------|--------------------------------------------------|--------------------------------------------------|
| Definition format     | YAML files with `version: 1` + `agent` object    | Markdown files with YAML frontmatter             |
| Agent location        | Passed via `--agent-file` flag                   | `~/.claude/agents/` and `.claude/agents/`        |
| Subagent invocation   | Task tool (explicit)                             | Task tool (explicit)                             |
| Dynamic creation      | CreateSubagent tool (opt-in)                     | Not supported                                    |
| Prompt templating     | `${VAR}` expansion with built-in + custom vars   | No built-in template variable system             |
| Tool specification    | Explicit `module:ClassName` paths                | Implicit (tools are managed by the runtime)      |
| Inheritance           | `extend` field for agent composition             | Not supported                                    |

---

## Scripts

**Dedicated scripts directory:** No. Kimi Code CLI does not document a standalone scripts directory outside of the skill structure.

The only documented location for scripts is inside a skill directory:

```
<skills-dir>/<skill-name>/scripts/
```

Scripts placed here can be referenced from `SKILL.md` and executed by the agent via the Shell tool. There is no global `~/.kimi/scripts/` or project-level `.kimi/scripts/` convention.

For automation outside of skills, Kimi supports **print mode** (`--print` flag) which enables non-interactive execution suitable for CI/CD pipelines and shell scripts. Print mode implicitly enables `--yolo` (auto-approve) and outputs to stdout. Options include `--output-format=stream-json` for machine-readable output and `--input-format=stream-json` for JSONL input.

---

## Sources

- [Kimi Code CLI homepage](https://www.kimi.com/code)
- [Kimi Code CLI documentation](https://moonshotai.github.io/kimi-cli/en/)
- [Agent Skills documentation](https://moonshotai.github.io/kimi-cli/en/customization/skills.html)
- [Agents and Subagents documentation](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Slash Commands reference](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html)
- [Changelog](https://moonshotai.github.io/kimi-cli/en/release-notes/changelog.html)
- [Breaking Changes](https://moonshotai.github.io/kimi-cli/en/release-notes/breaking-changes.html)
- [Data Locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Print Mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [Getting Started](https://moonshotai.github.io/kimi-cli/en/guides/getting-started.html)
- [GitHub repository](https://github.com/MoonshotAI/kimi-cli)
- [AGENTS.md in repository](https://github.com/MoonshotAI/kimi-cli/blob/main/AGENTS.md)
