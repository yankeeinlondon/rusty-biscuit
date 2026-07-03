---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://www.kimi.com/code
docs: https://moonshotai.github.io/kimi-cli/en/
skills_docs: https://moonshotai.github.io/kimi-cli/en/customization/skills.html

support: first_class

locations:
  - os: all
    scope: user
    path: ~/.kimi/skills/<skill-name>/SKILL.md
    notes: Kimi brand user skills directory. Highest-priority brand directory when merge_all_available_skills is true or false.
  - os: all
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: Claude-compatible brand user skills directory. Loaded and merged when merge_all_available_skills = true; used as fallback when merge_all_available_skills = false and ~/.kimi/skills/ is absent.
  - os: all
    scope: user
    path: ~/.codex/skills/<skill-name>/SKILL.md
    notes: Codex-compatible brand user skills directory. Lower priority than ~/.kimi/skills/ and ~/.claude/skills/.
  - os: all
    scope: user
    path: ~/.config/agents/skills/<skill-name>/SKILL.md
    notes: Recommended generic cross-tool user skills directory. Searched independently from brand group.
  - os: all
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: Generic fallback user skills directory.
  - os: all
    scope: repo
    path: .kimi/skills/<skill-name>/SKILL.md
    notes: Project-level Kimi brand skills. Resolved relative to project root (nearest .git ancestor of work directory, or work directory itself when not in a git repo).
  - os: all
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: Project-level Claude-compatible brand skills.
  - os: all
    scope: repo
    path: .codex/skills/<skill-name>/SKILL.md
    notes: Project-level Codex-compatible brand skills.
  - os: all
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: Project-level generic cross-tool skills.
  - os: all
    scope: other
    path: <extra_skill_dirs>/<skill-name>/SKILL.md
    notes: Additional directories declared via extra_skill_dirs in ~/.kimi/config.toml. Tilde paths expand to $HOME; relative paths resolve against project root.
  - os: all
    scope: other
    path: <skills-dir>/<skill-name>/SKILL.md
    notes: Directories appended via --skills-dir flag. Overrides auto-discovered user/project directories.
  - os: all
    scope: system
    path: Built-in skills (kimi-cli-help, skill-creator)
    notes: Shipped with the CLI; lowest precedence.

format:
  file_names:
    - SKILL.md
    - "*.md (flat skills directly inside a skills directory)"
  frontmatter: true
  required_fields: []
  optional_fields:
    - name
    - description
    - license
    - compatibility
    - metadata
    - type
  body_format: markdown
  notes: |
    Kimi implements the Agent Skills open standard. `name` and `description` are optional; `name` defaults to the directory name (or filename without `.md` for flat skills), and `description` falls back to the first non-empty body line or a default string. Flow skills add `type: flow` and embed a Mermaid or D2 diagram. Supporting directories `scripts/`, `references/`, and `assets/` are conventional.

discovery:
  mechanism: |
    Layered scan at session startup. Roots are discovered in priority order: Project > User > Extra > Built-in. Within user and project scopes, two independent groups are scanned:
    - Brand group: `.kimi/skills/`, `.claude/skills/`, `.codex/skills/` (in that priority order).
    - Generic group: `.config/agents/skills/`, `.agents/skills/` (in that priority order).
    When `merge_all_available_skills = true` (default), every existing brand directory is loaded and merged; same-name conflicts resolve by brand priority (kimi > claude > codex). The generic group is always merged independently. When `merge_all_available_skills = false`, only the first existing brand directory is used.
    Discovered skill metadata (name, path, description) is injected into the system prompt grouped by origin scope so the model can decide which `SKILL.md` to read. Skills can also be explicitly invoked with `/skill:<name>` or, for flow skills, `/flow:<name>`.
  precedence: |
    Project > User > Extra > Built-in. Within brand directories: kimi > claude > codex. Brand group wins over generic group when names collide. `--skills-dir` directories take precedence over auto-discovered user/project directories. `extra_skill_dirs` are additive and ranked below project/user auto-discovery.
  enable_disable: |
    No per-skill enable/disable flag. All discovered skills are available. Session-level control is via `--skills-dir` (replace discovery with explicit dirs), `extra_skill_dirs` (add dirs), or `merge_all_available_skills = false` (restrict brand group to the single highest-priority existing directory). There is no equivalent of Claude's `disable-model-invocation` or `user-invocable`.
  notes: |
    Skill paths are independent of `KIMI_SHARE_DIR`; `KIMI_SHARE_DIR` affects runtime data only, not skill search paths. Project skill roots are resolved against the project root (nearest `.git` ancestor), so monorepo subdirectories still surface repo-level skills.

portability:
  portable: true
  non_portable_assets:
    - "Flow skills (`type: flow`) with Mermaid/D2 diagrams — execution semantics are Kimi-specific"
    - "`scripts/` files — language, executable availability, and OS/shell assumptions vary"
    - "References to Kimi-specific tools (Shell, StrReplaceFile, Agent, etc.) or paths"
    - "Project-root-relative paths and assumptions about `.git` ancestry"
    - "`metadata` contents that encode provider-specific behavior"
  rewrite_needed: true
  notes: |
    Standard Agent Skills frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`) and the Markdown body are portable to other Agent Skills implementations (Claude Code, Codex, etc.). Kimi-specific flow skills, inline script dependencies, and tool-name references need rewriting or host gating when moving to another provider. Because Kimi merges `.claude/skills/` and `.codex/skills/` directly, skills placed in those directories are already expected to be cross-tool compatible.

cli_params:
  - flag: --skills-dir PATH
    description: Append additional skills directories. Repeatable. These directories override auto-discovered user/project skill directories.
    example: kimi --skills-dir /path/to/my-skills --skills-dir /path/to/more-skills
  - flag: --agent NAME
    description: Use a built-in agent (default or okabe). Affects system prompt generation and available tools, but not skill discovery itself.
    example: kimi --agent okabe
  - flag: --agent-file PATH
    description: Load a custom agent YAML file. Mutually exclusive with --agent.
    example: kimi --agent-file ./my-agent.yaml
  - flag: --work-dir PATH / -w PATH
    description: Set the working directory, which determines the project root used for repo-level skill discovery.
    example: kimi -w /path/to/project
  - flag: --add-dir PATH
    description: Add additional workspace directories. Does not change skill discovery roots.
    example: kimi --add-dir ../shared-lib
  - flag: --config-file PATH
    description: Load an alternative TOML/JSON config file where extra_skill_dirs and merge_all_available_skills can be set.
    example: kimi --config-file ./kimi.toml
  - flag: --config STRING
    description: Pass configuration inline. Mutually exclusive with --config-file.
    example: kimi --config '{merge_all_available_skills = false}'

env_vars:
  - name: KIMI_SHARE_DIR
    effect: Overrides the runtime share directory (default ~/.kimi). Does NOT affect Agent Skills search paths; skills and runtime data are intentionally separated.

changes: []

requires_claudine_update: true
reason: |
  Claudine's linking module should model Kimi's multi-root skill discovery: brand directories (.kimi/skills/, .claude/skills/, .codex/skills/) merged by priority plus generic directories (.config/agents/skills/, .agents/skills/) at both user and project scopes; the `merge_all_available_skills` config toggling between merge-everything and first-match-only behavior; project roots resolved against the nearest `.git` ancestor; flat `.md` skills; explicit `--skills-dir` and `extra_skill_dirs` additive paths; and Kimi-specific flow skills (`type: flow` with Mermaid/D2 diagrams). It should also record that Kimi reads Claude/Codex skill directories directly, making those paths high-value portable linking targets.
---

# Kimi Code CLI Skills

## Overview

Kimi Code CLI (Moonshot AI's agentic coding CLI) supports [Agent Skills](https://agentskills.io/), an open format for adding specialized knowledge and workflows to AI agents. A skill is a directory containing a `SKILL.md` entry point with optional YAML frontmatter and Markdown instructions. At startup, Kimi discovers skills, injects their names, paths, and descriptions into the system prompt, and lets the model decide when to read a given `SKILL.md`. Users can also invoke skills explicitly with `/skill:<name>`.

Kimi distinguishes **skills** (knowledge guidance the AI reads) from **plugins** (executable tools declared via `plugin.json`). This document covers skills only.

## Locations

Skill resources are stored by scope and by cross-tool compatibility group:

| Scope | Path | Notes |
|---|---|---|
| User — Kimi brand | `~/.kimi/skills/<skill-name>/SKILL.md` | Highest-priority user brand directory. |
| User — Claude brand | `~/.claude/skills/<skill-name>/SKILL.md` | Loaded when `merge_all_available_skills` is true, or as fallback when false. |
| User — Codex brand | `~/.codex/skills/<skill-name>/SKILL.md` | Lowest-priority brand user directory. |
| User — generic | `~/.config/agents/skills/<skill-name>/SKILL.md` | Recommended cross-tool location; searched independently from brand group. |
| User — generic fallback | `~/.agents/skills/<skill-name>/SKILL.md` | Generic fallback. |
| Project — Kimi brand | `.kimi/skills/<skill-name>/SKILL.md` | Resolved relative to project root (nearest `.git` ancestor). |
| Project — Claude brand | `.claude/skills/<skill-name>/SKILL.md` | Same root resolution. |
| Project — Codex brand | `.codex/skills/<skill-name>/SKILL.md` | Same root resolution. |
| Project — generic | `.agents/skills/<skill-name>/SKILL.md` | Same root resolution. |
| Extra | Paths from `extra_skill_dirs` in `~/.kimi/config.toml` | Additive; tilde expands to `$HOME`, relative paths resolve against project root. |
| Explicit | Paths from `--skills-dir` | Replaces auto-discovered user/project directories. |
| Built-in | `kimi-cli-help`, `skill-creator` | Shipped with the CLI; lowest precedence. |

On all platforms `~` resolves to the user's home directory. Project skill roots use the nearest `.git` ancestor of the working directory (or the working directory itself when not inside a git repo), so launching from a monorepo subdirectory still surfaces repo-level skills.

## File Format

A skill is either:

- A subdirectory with a `SKILL.md` entry point (canonical layout):

```text
my-skill/
├── SKILL.md          # Required metadata + instructions
├── scripts/          # Optional executable scripts
├── references/       # Optional reference documents
└── assets/           # Optional other resources
```

- A flat `.md` file placed directly inside a skills directory; its `name` defaults to the filename without `.md`.

`SKILL.md` uses YAML frontmatter followed by Markdown content:

```markdown
---
name: code-style
description: My project's code style guidelines
---

## Code Style

- Use 4-space indentation
- Variable names use camelCase
```

Recognized frontmatter fields:

| Field | Required | Description |
|---|---|---|
| `name` | No | 1–64 lowercase letters, numbers, hyphens. Defaults to directory or filename. |
| `description` | No | 1–1024 characters; shown in skill listings. Falls back to first non-empty body line or a default. |
| `license` | No | License name or file reference. |
| `compatibility` | No | Environment requirements, up to 500 characters. |
| `metadata` | No | Additional key-value attributes. |
| `type` | No | Set to `flow` for flow skills. |

The body is Markdown. Kimi does not currently support Claude-specific frontmatter extensions such as `allowed-tools`, `disallowed-tools`, `context`, `agent`, `hooks`, `paths`, or shell-injection blocks.

### Flow skills

Flow skills embed an Agent Flow diagram and are invoked via `/flow:<name>`:

```markdown
---
name: code-review
description: Code review workflow
type: flow
---

```mermaid
flowchart TD
A([BEGIN]) --> B[Analyze code changes]
B --> C{Is quality acceptable?}
C -->|Yes| D[Generate report]
C -->|No| E[List issues]
E --> B
D --> F([END])
```
```

Flow diagrams must contain one `BEGIN` and one `END` node and may use Mermaid or D2 syntax. Decision nodes require the agent to output `<choice>branch name</choice>` to select the next step.

## Discovery and Precedence

Kimi loads skills in layers:

```text
Project > User > Extra > Built-in
```

Within the **user** and **project** scopes, two independent groups are scanned:

1. **Brand group** (mutually exclusive by default):
   - `.kimi/skills/`
   - `.claude/skills/`
   - `.codex/skills/`

2. **Generic group** (mutually exclusive):
   - `.config/agents/skills/`
   - `.agents/skills/`

The `merge_all_available_skills` config key controls brand-group behavior:

- `true` (default): every existing brand directory is loaded and merged; same-name conflicts resolve by brand priority (`kimi > claude > codex`).
- `false`: only the first existing brand directory is used.

The generic group is always merged independently. When a skill name exists in both brand and generic groups, the brand group wins.

Discovered skill metadata is injected into the system prompt grouped by origin scope (`Project`, `User`, `Extra`, `Built-in`), letting the model distinguish project-specific from user-level skills. Skills are automatically considered by the model during conversation and can be explicitly loaded with `/skill:<name>`. Flow skills can be loaded as a standard skill with `/skill:<name>` or executed as a flow with `/flow:<name>`.

There is no per-skill enable/disable flag. To control skill loading, use:

- `--skills-dir` to replace discovery with explicit directories.
- `extra_skill_dirs` in config to add directories.
- `merge_all_available_skills = false` to restrict the brand group to the single highest-priority existing directory.

## Portability

The portable parts of a Kimi skill are the Agent Skills standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`) and the Markdown body. Because Kimi explicitly reads `.claude/skills/` and `.codex/skills/`, skills placed in those directories are expected to be cross-tool compatible.

Assets that need rewriting or host gating when moving to another provider:

- **Flow skills** (`type: flow`) — the Mermaid/D2 execution engine and `/flow:<name>` command are Kimi-specific.
- **`scripts/` files** — depend on installed interpreters, OS, and shell.
- **Tool-name references** — Kimi tools such as `Shell`, `StrReplaceFile`, `Agent`, etc., may not exist or may have different schemas elsewhere.
- **Project-root-relative paths** — tied to Kimi's `.git`-root discovery.
- **`metadata` contents** — may encode provider-specific behavior.

Kimi has no direct equivalent of Claude Code's `skillOverrides`, `disable-model-invocation`, `user-invocable`, managed skills, or plugin namespacing.

## Claudine Linking Notes

For Claudine's cross-provider resource linking:

- Treat Kimi-brand, Claude-brand, Codex-brand, and generic skill directories at both user and project scopes as linkable locations. Because Kimi merges `.kimi/skills/`, `.claude/skills/`, and `.codex/skills/`, a skill placed in `.claude/skills/` is usable by both Claude Code and Kimi.
- Classify standard Agent Skills frontmatter + Markdown body as portable.
- Flag flow skills (`type: flow`) and any `scripts/` dependencies as non-portable / requiring rewrite.
- Account for `merge_all_available_skills` when deciding which brand directories are effective; when `false`, only the highest-priority existing brand directory contributes.
- Recognize flat `.md` skills as first-class skill entries whose `name` derives from the filename.
- `--skills-dir` and `extra_skill_dirs` add explicit or additive linking targets that may sit outside the conventional layout.

## Sources

- [Kimi Code CLI — Agent Skills](https://moonshotai.github.io/kimi-cli/en/customization/skills.html)
- [Kimi Code CLI — Agents and Subagents](https://moonshotai.github.io/kimi-cli/en/customization/agents.html)
- [Kimi Code CLI — Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Kimi Code CLI — Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.html)
- [Kimi Code CLI — Data Locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Kimi Code CLI — `kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Kimi Code CLI — Slash Commands](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html)
- [Agent Skills specification](https://agentskills.io/specification)
- [Kimi Code homepage](https://www.kimi.com/code)
- [Kimi CLI GitHub repository](https://github.com/MoonshotAI/kimi-cli)
