---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://www.anthropic.com/claude-code
docs: https://code.claude.com/docs/en/overview
skills_docs: https://code.claude.com/docs/en/skills

support: first_class

locations:
  - os: macos
    scope: system
    path: /Library/Application Support/ClaudeCode/managed-skills/
    notes: System-wide managed skills directory. Skipped when CLAUDE_CODE_DISABLE_POLICY_SKILLS=1. Not observed on this host.
  - os: linux
    scope: system
    path: /etc/claude-code/managed-skills/
    notes: System-wide managed skills directory on Linux and WSL. Skipped when CLAUDE_CODE_DISABLE_POLICY_SKILLS=1.
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\managed-skills\\"
    notes: System-wide managed skills directory on Windows. Skipped when CLAUDE_CODE_DISABLE_POLICY_SKILLS=1.
  - os: macos
    scope: system
    path: /Library/Application Support/ClaudeCode/managed-settings.json
    notes: File-based managed policy settings. Also supports a managed-settings.d/ drop-in directory beside it.
  - os: linux
    scope: system
    path: /etc/claude-code/managed-settings.json
    notes: File-based managed policy settings. Also supports a managed-settings.d/ drop-in directory beside it.
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\managed-settings.json"
    notes: File-based managed policy settings. Also supports a managed-settings.d/ drop-in directory beside it. Legacy C:\\ProgramData\ClaudeCode\\ path removed in v2.1.75.
  - os: macos
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: Personal skills available across all projects. Symlinks to skill directories are followed. Observed on this host.
  - os: linux
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: Personal skills available across all projects. Symlinks to skill directories are followed.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills\\<skill-name>\\SKILL.md"
    notes: Personal skills available across all projects. Symlinks to skill directories are followed.
  - os: macos
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: Project-scoped, team-shareable. Loaded from the launch directory and every parent up to the repository root; also discovered in nested subdirectories on demand. Requires accepting the workspace trust dialog for permission-related frontmatter to take effect.
  - os: linux
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: Project-scoped, team-shareable. Loaded from the launch directory and every parent up to the repository root; also discovered in nested subdirectories on demand. Requires accepting the workspace trust dialog for permission-related frontmatter to take effect.
  - os: windows
    scope: repo
    path: ".claude\\skills\\<skill-name>\\SKILL.md"
    notes: Project-scoped, team-shareable. Loaded from the launch directory and every parent up to the repository root; also discovered in nested subdirectories on demand. Requires accepting the workspace trust dialog for permission-related frontmatter to take effect.
  - os: macos
    scope: repo
    path: .claude/commands/<command>.md
    notes: Legacy custom slash commands. Still supported and share the same frontmatter model; a skill takes precedence if both share a name.
  - os: linux
    scope: repo
    path: .claude/commands/<command>.md
    notes: Legacy custom slash commands. Still supported and share the same frontmatter model; a skill takes precedence if both share a name.
  - os: windows
    scope: repo
    path: ".claude\\commands\\<command>.md"
    notes: Legacy custom slash commands. Still supported and share the same frontmatter model; a skill takes precedence if both share a name.
  - os: macos
    scope: extension
    path: <plugin>/skills/<skill-name>/SKILL.md
    notes: Plugin skills are namespaced as plugin-name:skill-name and load only when the plugin is enabled.
  - os: linux
    scope: extension
    path: <plugin>/skills/<skill-name>/SKILL.md
    notes: Plugin skills are namespaced as plugin-name:skill-name and load only when the plugin is enabled.
  - os: windows
    scope: extension
    path: "<plugin>\\skills\\<skill-name>\\SKILL.md"
    notes: Plugin skills are namespaced as plugin-name:skill-name and load only when the plugin is enabled.

format:
  file_names:
    - SKILL.md
    - "*.md (legacy .claude/commands/)"
  frontmatter: true
  required_fields:
    - name (Agent Skills open standard)
    - description (Agent Skills open standard)
  optional_fields:
    - name (Claude Code allows omission; defaults to directory name)
    - description
    - when_to_use
    - argument-hint
    - arguments
    - disable-model-invocation
    - user-invocable
    - allowed-tools
    - disallowed-tools
    - model
    - effort
    - context
    - agent
    - hooks
    - paths
    - shell
    - license (Agent Skills standard)
    - compatibility (Agent Skills standard)
    - metadata (Agent Skills standard)
  body_format: markdown
  notes: |
    Claude Code implements the Agent Skills open standard but relaxes it: no field is strictly required and `description` is only recommended. `SKILL.md` is the required entry point; supporting files live beside it and are referenced from the body. Frontmatter strings may include `${CLAUDE_SKILL_DIR}`, `${CLAUDE_PROJECT_DIR}`, and argument substitutions. A skill directory that also contains `.claude-plugin/plugin.json` loads as a skills-directory plugin named `<name>@skills-dir` and can bundle agents, hooks, MCP servers, and output styles.

discovery:
  mechanism: |
    File-system watcher scans managed, user, project, nested project, and --add-dir `.claude/skills/` directories at startup and watches them for changes. Skills are loaded progressively: `name`/`description` metadata is always in context; the full body loads when the skill is invoked by the user (`/skill-name`) or by the model (unless `disable-model-invocation: true`). Legacy `.claude/commands/*.md` are discovered the same way. Creating a top-level skills directory that did not exist when the session started requires a restart so the new directory can be watched.
  precedence: |
    Enterprise/managed > personal (`~/.claude/skills/`) > project (`.claude/skills/`) > plugin (namespaced). A user/project skill with the same name as a bundled skill overrides the bundled version. Nested project skills with clashing names are qualified by subdirectory path, e.g. `apps/web:deploy`, and the variant matching the working files is preferred. `skillOverrides` in settings can demote or hide a skill regardless of scope.
  enable_disable: |
    Per-skill: `disable-model-invocation: true` blocks the model from auto-invoking and preloading into subagents; `user-invocable: false` hides it from the `/` menu; `skillOverrides` can set a skill to `on`, `name-only`, `user-invocable-only`, or `off`. Global toggles: `--disable-slash-commands` disables all skills/commands for the session; `--bare` / `CLAUDE_CODE_SIMPLE` and `--safe-mode` / `CLAUDE_CODE_SAFE_MODE` skip skill discovery; `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS` disables only bundled skills; `CLAUDE_CODE_DISABLE_POLICY_SKILLS` skips managed skills.
  notes: |
    Plugin skills are managed through `/plugin` and are not affected by `skillOverrides`. Live change detection covers `SKILL.md` text only; plugin-side `hooks/`, `.mcp.json`, `agents/`, and `output-styles/` require `/reload-plugins`. Project skills require workspace trust before permission-related frontmatter such as `allowed-tools` takes effect.

portability:
  portable: true
  non_portable_assets:
    - "Inline shell commands (`!`command`` and ` ```! ` blocks) — OS/shell dependent"
    - "Scripts in `scripts/` — language, path, and executable availability vary by host"
    - "`allowed-tools` / `disallowed-tools` rules using OS-specific tool names or paths"
    - "`paths` globs and `${CLAUDE_PROJECT_DIR}` references tied to repository layout"
    - "`hooks`, `agents/`, `.mcp.json`, and plugin metadata bundled in a skills-directory plugin"
  rewrite_needed: true
  notes: |
    The Markdown body and the Agent Skills frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`) are portable across tools that implement the open standard. Claude-specific extensions (`context`, `agent`, `model`, `effort`, `hooks`, `paths`, `shell`) and any local scripts or shell-injection commands need rewriting or host gating when moving to another provider. `skillOverrides` and managed-policy skills are Claude-specific and do not travel.

cli_params:
  - flag: --add-dir <dir> [...]
    description: Add working directories. `.claude/skills/` inside an added directory is discovered; other `.claude/` config is not.
    example: claude --add-dir ../shared
  - flag: --bare
    description: Minimal mode. Skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Sets CLAUDE_CODE_SIMPLE.
    example: claude --bare -p "summarize"
  - flag: --safe-mode
    description: Disables CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP, and auto-memory.
    example: claude --safe-mode
  - flag: --disable-slash-commands
    description: Disables all skills and commands for the session.
    example: claude --disable-slash-commands
  - flag: --plugin-dir <path>
    description: Load a plugin from a directory or zip for this session, including its skills.
    example: claude --plugin-dir ./my-plugin
  - flag: --plugin-url <url>
    description: Fetch and load a plugin zip from a URL for this session.
    example: claude --plugin-url https://example.com/plugin.zip
  - flag: --setting-sources user,project,local
    description: Restrict which settings scopes load. Can prevent project/local skill-related settings from applying.
    example: claude --setting-sources user,project
  - flag: --settings <file-or-json>
    description: Session-only settings overlay, including skillOverrides.
    example: claude --settings ./ci-settings.json
  - flag: --allowedTools / --allowed-tools
    description: Permission rules that can allow the Skill tool or specific skills, e.g. Skill(commit) or Skill(deploy *).
    example: claude --allowedTools "Skill(commit)"
  - flag: --disallowedTools / --disallowed-tools
    description: Permission rules that can deny the Skill tool or specific skills, e.g. Skill(deploy *).
    example: claude --disallowedTools "Skill(deploy *)"

env_vars:
  - name: CLAUDE_CODE_DISABLE_BUNDLED_SKILLS
    effect: Set to 1 to disable bundled skills/workflows. Custom and plugin skills are unaffected.
  - name: CLAUDE_CODE_DISABLE_POLICY_SKILLS
    effect: Set to 1 to skip loading skills from the system-wide managed skills directory.
  - name: CLAUDE_CODE_SAFE_MODE
    effect: Set to 1 to disable skills along with most other customizations. Equivalent to --safe-mode.
  - name: CLAUDE_CODE_SIMPLE
    effect: Set to 1 to disable auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Equivalent to --bare.
  - name: CLAUDE_CODE_SYNC_SKILLS
    effect: Set to 1 to download enabled claude.ai skills into ~/.claude/skills/ before the first query and resync every 10 minutes. Non-interactive -p only.
  - name: CLAUDE_CODE_SYNC_SKILLS_INSTALL_TIMEOUT_MS
    effect: Timeout for mid-session skills resync when CLAUDE_CODE_SYNC_SKILLS is set (default 30000).
  - name: CLAUDE_CODE_SYNC_SKILLS_WAIT_TIMEOUT_MS
    effect: Timeout for the first query to wait on initial skills sync (default 5000).
  - name: CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD
    effect: Set to 1 to also load memory files from --add-dir directories. Does not change skill discovery, which already loads .claude/skills/ from added dirs.
  - name: SLASH_COMMAND_TOOL_CHAR_BUDGET
    effect: Override the character budget for skill metadata shown to the Skill tool.

changes:
  - "Split location records from 'os: all' into separate macOS, Linux, and Windows entries per schema requirements."
  - "Verified official docs for skills, settings, env vars, CLI reference, and the Agent Skills open specification."
  - "Confirmed managed settings drop-in directory behavior, managed-skills trust requirements, and symlink following for user/project skills."

requires_claudine_update: true
reason: |
  Claudine's linking module should recognize Claude Code's first-class Agent Skills layout (`~/.claude/skills/` and `.claude/skills/`), legacy `.claude/commands/`, nested monorepo skills with directory-qualified names, plugin namespacing, and managed/system skill directories. It also needs to model Claude-specific portability concerns: `skillOverrides`, `disable-model-invocation`/`user-invocable`, `context`/`agent` subagent execution, shell-injection blocks, and bundled/managed skill toggles.
---

# Claude Code Skills

## Overview

Claude Code has first-class, file-system-based **skills** that follow the [Agent Skills](https://agentskills.io) open standard with Claude-specific extensions. A skill is a directory containing a `SKILL.md` entry point with YAML frontmatter and Markdown instructions. Skills can be invoked explicitly (`/skill-name`), loaded automatically by the model when the user's prompt matches the `description`, or preloaded into subagents. Custom slash commands have been merged into the skills system; existing `.claude/commands/*.md` files continue to work and share the same frontmatter model.

Skills are also a packaging boundary: a skill directory can contain supporting reference files, templates, scripts, and (if it includes `.claude-plugin/plugin.json`) agents, hooks, MCP servers, and output styles.

## Locations

Skill resources are stored by scope:

| Scope | macOS | Linux / WSL | Windows | Notes |
|---|---|---|---|---|
| Managed / system | `/Library/Application Support/ClaudeCode/managed-skills/` | `/etc/claude-code/managed-skills/` | `C:\Program Files\ClaudeCode\managed-skills\` | System-wide managed skills. Can be skipped with `CLAUDE_CODE_DISABLE_POLICY_SKILLS=1`. Not observed locally. |
| Managed policy | `/Library/Application Support/ClaudeCode/managed-settings.json` + optional `managed-settings.d/` | `/etc/claude-code/managed-settings.json` + optional `managed-settings.d/` | `C:\Program Files\ClaudeCode\managed-settings.json` + optional `managed-settings.d/` | Organization-wide policy. Highest precedence. Legacy `C:\ProgramData\ClaudeCode\managed-settings.json` removed in v2.1.75. |
| Personal | `~/.claude/skills/<skill-name>/SKILL.md` | `~/.claude/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.claude\skills\<skill-name>\SKILL.md` | Applies across all projects. Local host observed real skills and symlinks to external skill directories. |
| Project | `.claude/skills/<skill-name>/SKILL.md` | `.claude/skills/<skill-name>/SKILL.md` | `.claude\skills\<skill-name>\SKILL.md` | Shared with repository collaborators. Discovered from launch directory and every parent up to repo root, plus nested subdirectories on demand. Requires workspace trust. |
| Legacy commands | `.claude/commands/<command>.md` | `.claude/commands/<command>.md` | `.claude\commands\<command>.md` | Still discovered; same frontmatter; lower precedence than a skill with the same name. |
| Plugin | `<plugin>/skills/<skill-name>/SKILL.md` | `<plugin>/skills/<skill-name>/SKILL.md` | `<plugin>\skills\<skill-name>\SKILL.md` | Namespaced as `plugin-name:skill-name`; requires the plugin to be enabled. |

On Windows, `~/.claude` resolves to `%USERPROFILE%\.claude`. A `<skill-name>` entry in the personal or project locations may be a symlink to a directory elsewhere on disk; Claude Code follows the symlink and reads `SKILL.md` from the target, deduplicating if the same target is reachable from more than one location. Plugin skills handle symlinks differently.

## File Format

A skill is a directory with `SKILL.md` as the required entry point:

```text
my-skill/
├── SKILL.md           # Required metadata + instructions
├── reference.md       # Optional deep reference
├── examples.md        # Optional examples
└── scripts/
    └── helper.sh      # Optional executable script
```

`SKILL.md` contains YAML frontmatter between `---` markers followed by Markdown content. The Agent Skills specification requires `name` and `description`, but Claude Code treats all frontmatter fields as optional and only recommends `description`.

Commonly recognized frontmatter fields:

| Field | Purpose |
|---|---|
| `name` | Display name; defaults to directory name. |
| `description` | Routing signal for automatic invocation. |
| `when_to_use` | Extra trigger phrases, counted toward the 1,536-character skill-listing cap. |
| `argument-hint` | Autocomplete hint for arguments. |
| `arguments` | Named positional arguments for `$name` substitution. |
| `disable-model-invocation` | When `true`, only the user can invoke the skill and it is not preloaded into subagents. |
| `user-invocable` | When `false`, hides the skill from the `/` menu. |
| `allowed-tools` | Tool allowlist while the skill is active. |
| `disallowed-tools` | Tools removed from the model's pool while the skill is active. |
| `model` / `effort` | Model or effort override for the current turn. |
| `context` | Set to `fork` to run in a subagent. |
| `agent` | Subagent type used with `context: fork`. |
| `hooks` | Lifecycle hooks scoped to the skill. |
| `paths` | Glob patterns limiting auto-activation to matching files. |
| `shell` | `bash` (default) or `powershell` for inline `!` commands. |

The body supports string substitutions such as `$ARGUMENTS`, `$0`..`$N`, `$name`, `${CLAUDE_SESSION_ID}`, `${CLAUDE_EFFORT}`, `${CLAUDE_SKILL_DIR}`, and `${CLAUDE_PROJECT_DIR}`. Dynamic context can be injected with `` !`command` `` inline or ` ```! ` fenced blocks.

A skill folder that also contains `.claude-plugin/plugin.json` loads as a skills-directory plugin named `<name>@skills-dir`, allowing it to bundle agents, hooks, MCP servers, and output styles. For project `.claude/skills/`, this requires accepting the workspace trust dialog.

## Discovery and Precedence

Claude Code watches configured skill directories at startup and reloads `SKILL.md` changes during a session. Discovery order and precedence are:

1. Managed / enterprise settings and system managed-skills directory.
2. Personal `~/.claude/skills/`.
3. Project `.claude/skills/` (including parent-directory discovery and nested variants).
4. Plugin skills (namespaced).
5. Legacy `.claude/commands/`.

A user or project skill with the same name as a bundled skill overrides the bundled version. Nested skills with clashing names are both available under directory-qualified names. Plugin skills cannot collide because they carry a `plugin-name:` prefix.

Enable/disable mechanisms:

- `disable-model-invocation: true` — blocks the model from auto-invoking the skill and prevents preloading into subagents.
- `user-invocable: false` — hides the skill from the user menu but lets the model use it.
- `skillOverrides` in settings — per-skill states: `on`, `name-only`, `user-invocable-only`, `off`.
- `--disable-slash-commands` — disables all skills/commands for the session.
- `--bare` / `CLAUDE_CODE_SIMPLE=1` and `--safe-mode` / `CLAUDE_CODE_SAFE_MODE=1` — skip skill discovery.
- `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS=1` — disables only bundled skills.
- `CLAUDE_CODE_DISABLE_POLICY_SKILLS=1` — skips system-wide managed skills.
- Permission rules such as `Skill(deploy *)` can allow or deny the Skill tool globally or per skill.

Plugin skills are not affected by `skillOverrides`. Live change detection covers `SKILL.md` text only; plugin-side `hooks/`, `.mcp.json`, `agents/`, and `output-styles/` require `/reload-plugins`.

## Portability

Skills are highly portable across tools that implement the Agent Skills standard. The portable parts are:

- `SKILL.md` Markdown body.
- Standard frontmatter: `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`.

Assets that need rewriting or host gating when moving to another provider include:

- Inline shell commands and `scripts/` files (OS/shell differences, missing binaries).
- Claude-specific extensions (`context`, `agent`, `model`, `effort`, `hooks`, `paths`, `shell`).
- `${CLAUDE_*}` substitutions and project-path assumptions.
- Permission rules in `allowed-tools`/`disallowed-tools` that reference Claude-specific tools or paths.
- Skills-directory plugin metadata (`hooks/`, `agents/`, `.mcp.json`, `output-styles/`).

`skillOverrides`, managed skills, bundled-skill overrides, and plugin namespacing are Claude Code-specific and do not map directly to other providers.

## Claudine Linking Notes

For Claudine's cross-provider resource linking:

- Treat `~/.claude/skills/<name>/SKILL.md` and `.claude/skills/<name>/SKILL.md` as the canonical user and repo skill locations.
- Treat `.claude/commands/*.md` as legacy slash commands with the same frontmatter; link them as skill equivalents.
- Recognize nested project skills by their directory-qualified name (`<subdir>:<skill>`).
- Classify each linked asset as portable when the body uses only standard Agent Skills frontmatter and Markdown; flag assets containing `!` shell injection, `scripts/`, `hooks`, `agents/`, `.mcp.json`, or Claude-specific frontmatter as needing rewrite.
- Account for `skillOverrides` and `disable-model-invocation`/`user-invocable` when deciding whether a linked skill is active or visible in a target provider.
- Bundled skills (`/code-review`, `/debug`, `/loop`, etc.) are replaceable by user/project skills with the same name; link them only if a custom override exists.
- Managed/system skills should be excluded from user-controlled sync unless `CLAUDE_CODE_DISABLE_POLICY_SKILLS` is considered.

## Changelog

- **2026-07-03** — Split location records from `os: all` into per-OS records (macOS, Linux, Windows) to satisfy the schema contract. Verified paths and behavior against current official documentation. Added explicit managed-policy location records and skills-directory plugin notes. Confirmed local `~/.claude/skills/` contains real skills and symlinks to external directories.

## Sources

- [Claude Code — Skills](https://code.claude.com/docs/en/skills)
- [Claude Code — Slash commands](https://code.claude.com/docs/en/slash-commands)
- [Claude Code — Settings](https://code.claude.com/docs/en/settings)
- [Claude Code — Environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code — CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code — Sub-agents](https://code.claude.com/docs/en/sub-agents)
- [Claude Code — Hooks](https://code.claude.com/docs/en/hooks)
- [Agent Skills specification](https://agentskills.io/specification)
- [Claude Code product homepage](https://www.anthropic.com/claude-code)
