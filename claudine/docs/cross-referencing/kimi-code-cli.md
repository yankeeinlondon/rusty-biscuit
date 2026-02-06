# Kimi Code CLI: Slash Commands, Skills, Agents

Sources:
- https://moonshotai.github.io/kimi-cli/en/customization/skills.html
- https://moonshotai.github.io/kimi-cli/en/customization/agents.html
- https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html

## Skills (Agent Skills)

Kimi Code CLI supports the Agent Skills open format (a directory with a `SKILL.md`). Skills are discovered at startup; the agent can decide whether to open and read a skill based on the task. The list of skill names/paths/descriptions is injected into the system prompt.

### Directory discovery (user and repo scope)

User-level (first existing directory wins):
- `~/.config/agents/skills/` (recommended)
- `~/.agents/skills/`
- `~/.kimi/skills/`
- `~/.claude/skills/`
- `~/.codex/skills/`

Project-level (first existing directory wins):
- `.agents/skills/` (recommended)
- `.kimi/skills/`
- `.claude/skills/`
- `.codex/skills/`

Override:
- `--skills-dir /path/to/skills` skips user/project discovery.
- `KIMI_SHARE_DIR` does not affect skills discovery; skills paths are intentionally separate.

### Skill directory structure

Minimal:
- `<skill-name>/SKILL.md`

Recommended layout:
```
~/.config/agents/skills/
└── my-skill/
    ├── SKILL.md
    ├── scripts/
    ├── references/
    └── assets/
```

This is the only explicit place Kimi docs recommend putting scripts/executables (inside `scripts/` under a skill directory).

### Skill metadata (frontmatter)

`SKILL.md` begins with YAML frontmatter, then Markdown content. Supported fields:
- `name`: 1-64 chars; lowercase letters, numbers, hyphens only; defaults to directory name if omitted.
- `description`: 1-1024 chars; shown in skill list; defaults to "No description provided."
- `license`: license name or file reference.
- `compatibility`: environment requirements (<= 500 chars).
- `metadata`: additional key-value attributes.

Flow skill extension:
- `type: flow` enables flow skills.
- Content must include a Mermaid or D2 flow diagram.
- Diagram must have `BEGIN` and `END` nodes; decision nodes require the agent to emit `<choice>branch</choice>`.
- Run with `/flow:<name>`, or load as plain prompt with `/skill:<name>`.

### Skill invocation

- `/skill:<name>` loads the skill prompt (and can append extra user text).
- `/flow:<name>` executes flow skills as multi-step automations.

## Slash Commands

Kimi Code CLI documents built-in slash commands only. There is no documented mechanism for custom slash command files.

### Built-in slash commands

Help and info:
- `/help` (aliases: `/h`, `/?`)
- `/version`
- `/changelog` (alias: `/release-notes`)
- `/feedback`

Account and configuration:
- `/login` (alias: `/setup`) (only works with default config file)
- `/logout`
- `/model` (only works with default config file)
- `/reload`
- `/debug`
- `/usage` (alias: `/status`, Kimi Code platform only)
- `/mcp`

Session management:
- `/sessions` (alias: `/resume`)
- `/clear` (alias: `/reset`)
- `/compact`

Skills:
- `/skill:<name>`
- `/flow:<name>`

Other:
- `/init` (generate `AGENTS.md`)
- `/yolo`

Shell mode availability:
- `/help`, `/exit`, `/version`, `/changelog`, `/feedback` are available in shell mode.

## Agents and Subagents

Kimi Code CLI supports built-in agents and custom agents defined in YAML and loaded via `--agent-file`.

### Built-in agents

- `default`: general-purpose; tools include Task, Shell, file ops, glob/grep, web search/fetch.
- `okabe`: experimental; adds `SendDMail`.

### Agent file format

Agent files are YAML with `version: 1` and an `agent` object. Key fields:

Required (unless inherited):
- `name`
- `system_prompt_path` (path to Markdown template; relative to agent file)
- `tools` (list of tool paths, format `module:ClassName`)

Optional:
- `extend` (inherit from built-in `default` or another agent file)
- `system_prompt_args` (key-values for `${VAR}` expansion)
- `exclude_tools` (remove tools from inherited list)
- `subagents` (map of subagents)

System prompt built-in variables:
- `${KIMI_NOW}`, `${KIMI_WORK_DIR}`, `${KIMI_WORK_DIR_LS}`, `${KIMI_AGENTS_MD}`, `${KIMI_SKILLS}`

### Subagent definitions

Subagents are declared in the main agent file:
```
subagents:
  coder:
    path: ./coder-sub.yaml
    description: "Handle coding tasks"
```

Subagent files are standard agent YAML, typically:
- `extend` the main agent
- add role-specific `system_prompt_args`
- exclude `Task` to avoid nested subagent spawning

### Interaction model and concurrency

Execution model:
- Subagents run via the `Task` tool in isolated contexts.
- They do not share the main agent's conversation history; prompts must be self-contained.
- Results are returned to the orchestrator when complete.

Concurrency best practices (from docs + implied by model):
- Use separate subagents for independent tasks to run in parallel.
- Keep subagent prompts concise and self-sufficient (include all relevant context).
- Avoid nested `Task` calls (exclude `Task` from subagents) to prevent runaway recursion.
- Use targeted system prompts and reduced toolsets for subagents.

## Differences vs Anthropic/Claude Code

### Skills

Key differences:
- Kimi explicitly supports Agent Skills discovery across multiple tool-compatible directories and uses a “first existing directory wins” rule for user and project scopes; Claude Code commonly uses `~/.claude/skills/` and `.claude/skills/` without the multi-path priority system.
- Kimi defines a clear frontmatter schema (`name`, `description`, `license`, `compatibility`, `metadata`) and enforces naming constraints; Claude Code’s skill frontmatter is less strictly specified and typically focuses on `description`.
- Kimi adds “flow skills” (`type: flow`) with `/flow:<name>` execution, Mermaid/D2 diagrams, and explicit `BEGIN/END` nodes; Claude Code does not document a flow-execution mode for skills.
- Kimi allows `--skills-dir` to override discovery and explicitly ignores `KIMI_SHARE_DIR` for skills; Claude Code uses its own config roots and does not share this flag.

Common gotchas and mitigations (Kimi-specific):
- Only the first discovered skills directory is used per scope; if multiple exist, the later ones are ignored. Consolidate skills into the highest-priority directory or pass `--skills-dir`.
- Skill names must be lowercase alnum/hyphen; mismatches can make `/skill:<name>` fail. Match the directory name or set `name` explicitly.
- Flow skills require `BEGIN`/`END` and `<choice>` outputs; invalid diagrams cause flow execution to fail. Validate diagrams in Mermaid/D2 playgrounds.

### Slash commands

Key differences:
- Kimi’s slash commands are built-in and documented as session/config/debug controls; it does not document a custom slash-command file system.
- Claude Code supports user-defined slash commands (commonly via `~/.claude/commands/` and project `.claude/commands/`), which act as prompt templates.

Common gotchas and mitigations (Kimi-specific):
- `/login` and `/model` are unavailable when using `--config`/`--config-file`; use the default config file for these commands.
- Some commands are only available in interactive (non-shell) UI; in shell mode only a subset is supported.

## Where to put scripts or executables

Kimi docs only explicitly recommend storing scripts under a skill directory, e.g. `scripts/` inside `<skills-dir>/<skill-name>/`. There is no separate global “scripts” directory documented outside of the Agent Skills structure.

## Appendix: Claude Code directory conventions (from Anthropic docs)

Sources:
- https://code.claude.com/docs/en/skills
- https://code.claude.com/docs/en/sub-agents

### Skills and commands

Personal scope:
- `~/.claude/skills/<skill-name>/SKILL.md`

Project scope:
- `.claude/skills/<skill-name>/SKILL.md`

Enterprise scope:
- Managed settings (not a filesystem path; configured by org admins)

Plugin scope:
- `<plugin>/skills/<skill-name>/SKILL.md`

Slash commands in Claude Code:
- Custom slash commands are merged into skills.
- Legacy command files still work: `.claude/commands/<command>.md`.
- If a command and a skill share the same name, the skill takes precedence.

Nested discovery:
- Claude Code auto-discovers nested `.claude/skills/` from subdirectories (useful for monorepos).

### Subagents (agents)

User-level:
- `~/.claude/agents/` (Markdown files with YAML frontmatter)

Project-level:
- `.claude/agents/`

Session-level:
- `--agents` CLI flag (JSON, ephemeral)

Plugin-level:
- `<plugin>/agents/`
