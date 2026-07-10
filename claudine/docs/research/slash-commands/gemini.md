---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
slash_docs: https://geminicli.com/docs/cli/custom-commands/
support: first_class
locations:
  - os: macos
    scope: user
    path: ~/.gemini/commands/**/*.toml
    notes: User-defined custom slash commands. Available in any project. Discovered recursively; subdirectories become namespaces.
  - os: linux
    scope: user
    path: ~/.gemini/commands/**/*.toml
    notes: Same behavior as macOS; ~ resolves to $HOME.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\commands\\**\\*.toml"
    notes: Same behavior; subdirectories become colon-namespaced commands.
  - os: macos
    scope: repo
    path: .gemini/commands/**/*.toml
    notes: Project-specific custom commands. Require a trusted workspace. Win on name conflicts against user commands.
  - os: linux
    scope: repo
    path: .gemini/commands/**/*.toml
    notes: Same behavior as macOS.
  - os: windows
    scope: repo
    path: ".gemini\\commands\\**\\*.toml"
    notes: Same behavior as macOS.
  - os: macos
    scope: extension
    path: "<extension-path>/commands/**/*.toml"
    notes: Commands bundled with installed Gemini CLI extensions. Loaded alongside user and project commands.
  - os: linux
    scope: extension
    path: "<extension-path>/commands/**/*.toml"
    notes: Same as macOS.
  - os: windows
    scope: extension
    path: "<extension-path>\\commands\\**\\*.toml"
    notes: Same as macOS.
  - os: macos
    scope: user
    path: ~/.gemini/skills/
    notes: User Agent Skills. Directory-based with a SKILL.md entry point.
  - os: linux
    scope: user
    path: ~/.gemini/skills/
    notes: Same as macOS.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\skills\\"
    notes: Same as macOS.
  - os: macos
    scope: user
    path: ~/.agents/skills/
    notes: Interoperable alias for user skills. Takes precedence over ~/.gemini/skills/ within the same tier.
  - os: linux
    scope: user
    path: ~/.agents/skills/
    notes: Same as macOS.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\"
    notes: Same as macOS.
  - os: macos
    scope: repo
    path: .gemini/skills/
    notes: Workspace Agent Skills. Require a trusted workspace.
  - os: linux
    scope: repo
    path: .gemini/skills/
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".gemini\\skills\\"
    notes: Same as macOS.
  - os: macos
    scope: repo
    path: .agents/skills/
    notes: Interoperable alias for workspace skills. Takes precedence over .gemini/skills/ within the same tier.
  - os: linux
    scope: repo
    path: .agents/skills/
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".agents\\skills\\"
    notes: Same as macOS.
  - os: macos
    scope: extension
    path: "<extension-path>/skills/"
    notes: Agent Skills bundled with installed extensions.
  - os: linux
    scope: extension
    path: "<extension-path>/skills/"
    notes: Same as macOS.
  - os: windows
    scope: extension
    path: "<extension-path>\\skills\\"
    notes: Same as macOS.
  - os: macos
    scope: system
    path: /Library/Application Support/GeminiCli/system-defaults.json
    notes: System-wide default settings; lowest precedence. Path overridable via GEMINI_CLI_SYSTEM_DEFAULTS_PATH.
  - os: linux
    scope: system
    path: /etc/gemini-cli/system-defaults.json
    notes: Same as macOS.
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\system-defaults.json"
    notes: Same as macOS.
  - os: macos
    scope: system
    path: /Library/Application Support/GeminiCli/settings.json
    notes: System-wide override settings; highest file precedence. Path overridable via GEMINI_CLI_SYSTEM_SETTINGS_PATH.
  - os: linux
    scope: system
    path: /etc/gemini-cli/settings.json
    notes: Same as macOS.
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    notes: Same as macOS.
format:
  file_names:
    - "*.toml"
  frontmatter: false
  required_fields:
    - prompt
  optional_fields:
    - description
  argument_syntax: |
    {{args}} is replaced with the raw argument string typed after the command name. When {{args}} appears inside a shell injection block !{...}, it is shell-escaped before substitution. If {{args}} is absent, the full user input is appended to the prompt separated by two newlines.
  body_format: toml
  notes: |
    Custom commands are TOML v1 files with a required `prompt` string and optional `description`. The command name is derived from the relative path under the commands directory; path separators become colons (e.g., git/commit.toml → /git:commit).
    Agent Skills use a separate Markdown-based format: a directory containing SKILL.md with YAML frontmatter. Required frontmatter for skills: `name`, `description`. Skills are discovered at SKILL.md or */SKILL.md under a skills directory, with optional scripts/, references/, and assets/ subdirectories.
command_model:
  invocation: |
    Custom commands are invoked interactively by typing /name, e.g. /commit or /git:commit. Agent Skills are not directly slash-invoked; they are discovered at session start and activated by the model calling the `activate_skill` tool, or managed via /skills list, /skills disable, /skills enable, and /skills reload.
  namespacing: |
    Custom commands share the / namespace with built-in commands. Subdirectories under commands/ produce colon-separated namespaces. Project commands override user commands with the same name. Built-in commands win if a custom command collides with an existing built-in. Agent Skills use a flat name from SKILL.md frontmatter with precedence tiers: workspace > user > extension > built-in; within the same tier, .agents/skills/ aliases take precedence over .gemini/skills/.
  arguments: |
    Custom commands receive the raw text after the command name. Use {{args}} to inject it inside the prompt; otherwise the CLI appends the typed command to the prompt after two newlines. There is no positional or named argument parsing beyond the raw string. Multi-word arguments can be quoted by the user but are delivered as a single raw string to {{args}}. Agent Skills do not accept runtime arguments at activation.
  output_handling: |
    For custom commands, the resolved `prompt` string is sent to the model as a user prompt after argument substitution, shell command execution (!{...}), and file injection (@{...}). For Agent Skills, activation injects the SKILL.md body and folder structure into context and adds the skill directory to allowed file paths after user consent.
  disabled_mechanism: |
    Remove or rename the .toml file or SKILL.md directory. Use /commands reload or /skills reload to refresh discovery without restarting. Skills can be disabled per name with /skills disable <name> or gemini skills disable <name>, and re-enabled with /skills enable <name> or gemini skills enable <name>. Settings can disable all skills (skills.enabled: false), list disabled skill names (skills.disabled), or disable extension-provided commands/skills (admin.extensions.enabled: false).
  notes: |
    Workspace/project custom commands and workspace skills are gated by folder trust. If the folder is untrusted, workspace commands return [] and workspace skills are skipped. Shell commands inside !{...} trigger a confirmation dialog showing the exact resolved command before execution. File injection @{...} is processed before shell commands and argument substitution and respects .gitignore/.geminiignore.
portability:
  portable: false
  non_portable_assets:
    - "TOML file format for custom commands"
    - "{{args}} placeholder"
    - "!{...} shell injection syntax"
    - "@{...} file injection syntax"
    - "Colon (:) namespace syntax derived from directory paths"
    - "Extension command/skill hooks"
    - "SKILL.md activation via activate_skill tool and consent dialog"
    - "Trust model and trustedFolders.json"
  rewrite_needed: true
  notes: |
    The prose body of a custom command or SKILL.md is portable after rewriting metadata and placeholders. A TOML custom command must be converted to the target provider's command format (often Markdown with YAML frontmatter) and {{args}} mapped to the target's argument grammar. !{...} shell blocks and @{...} file injections have no direct equivalent in most providers and must be expanded or removed. Agent Skills follow the agentskills.io directory layout, so their structure is more portable than TOML custom commands, but activation semantics and frontmatter are still provider-specific.
cli_params:
  - flag: --skip-trust
    description: Treat the current workspace as trusted for this session (sets GEMINI_CLI_TRUST_WORKSPACE=true).
    example: gemini --skip-trust
  - flag: --extensions, -e
    description: Restrict loaded extensions, which also affects extension commands and skills.
    example: gemini --extensions my-ext
  - flag: --sandbox, -s
    description: Run in sandbox mode; may restrict tool execution available to commands.
    example: gemini -s
  - flag: --yolo, -y
    description: Auto-approve all tools for the session; non-default approval modes are forced back to default in untrusted folders.
    example: gemini -y
  - flag: --approval-mode
    description: Set approval mode (default, auto_edit, yolo, plan); yolo requires trust.
    example: gemini --approval-mode plan
  - flag: gemini skills list [--all]
    description: List discovered agent skills; --all includes built-in skills.
    example: gemini skills list --all
  - flag: gemini skills disable <name> [--scope user|workspace]
    description: Disable a skill by name.
    example: gemini skills disable my-skill --scope workspace
  - flag: gemini skills enable <name>
    description: Re-enable a disabled skill.
    example: gemini skills enable my-skill
  - flag: gemini skills install <source> [--scope user|workspace] [--path <subdir>]
    description: Install a skill from a Git repo or local path.
    example: gemini skills install https://github.com/user/repo.git --scope user
  - flag: gemini skills link <path>
    description: Link a skill from a local path for development.
    example: gemini skills link ./my-skill
  - flag: gemini skills uninstall <name> [--scope user|workspace]
    description: Remove an installed or linked skill.
    example: gemini skills uninstall my-skill
  - flag: gemini extensions list
    description: List installed extensions, whose commands/skills may be loaded.
    example: gemini extensions list
  - flag: gemini extensions disable <name> [--scope user|workspace]
    description: Disable an extension, removing its commands and skills.
    example: gemini extensions disable my-ext
  - flag: gemini extensions enable <name>
    description: Re-enable a disabled extension.
    example: gemini extensions enable my-ext
  - flag: /commands list
    description: In-session listing of custom command .toml files from all sources.
  - flag: /commands reload
    description: In-session reload of custom command definitions from all sources.
  - flag: /skills list [all] [nodesc]
    description: In-session listing of discovered skills.
  - flag: /skills reload
    description: In-session refresh of skill discovery from all tiers.
  - flag: /skills disable <name>
    description: In-session disable of a skill by name.
  - flag: /skills enable <name>
    description: In-session re-enable of a disabled skill.
env_vars:
  - name: GEMINI_CLI_HOME
    effect: Overrides the user home directory, shifting ~/.gemini/commands, ~/.gemini/skills, ~/.gemini/trustedFolders.json, and related user paths.
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: If set to "true", treats the current workspace as trusted for the session, equivalent to --skip-trust.
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: Overrides the path to the trustedFolders.json file that records per-folder trust decisions.
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: Overrides the system-wide override settings.json path.
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: Overrides the system-wide system-defaults.json path.
changes: []
requires_claudine_update: false
reason: Research confirms Gemini CLI has first-class user-defined custom commands and Agent Skills; existing non-portable classification with required rewrites remains accurate.
---

# Gemini CLI Slash Commands and Reusable Commands

## Overview

Gemini CLI provides two distinct but overlapping mechanisms for user-defined, reusable command resources:

1. **Custom commands** — user-defined slash commands stored as `.toml` files. These are the closest equivalent to the slash commands found in Claude Code or Codex.
2. **Agent Skills** — directory-based expertise bundles following the [agentskills.io](https://agentskills.io) open standard, discovered automatically and activated by the model via a tool call.

Support is **first class**: users can define custom commands at user, project, and extension scopes; invoke them with `/`; pass arguments; and reload them without restarting. Agent Skills add a higher-level packaging model with bundled scripts, references, and assets, plus explicit enable/disable management.

Built-in slash commands (such as `/help`, `/commands`, `/skills`, `/plan`) and user-defined custom commands share the same `/` namespace.

## Locations

### Custom commands

Custom commands are discovered recursively from `.toml` files in up to three locations:

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.gemini/commands/**/*.toml` | Available in any project. |
| Windows | User | `%USERPROFILE%\.gemini\commands\**\*.toml` | Available in any project. |
| macOS / Linux | Repo | `<project-root>/.gemini/commands/**/*.toml` | Project-specific; requires workspace trust. |
| Windows | Repo | `<project-root>\.gemini\commands\**\*.toml` | Project-specific; requires workspace trust. |
| All | Extension | `<extension-path>/commands/**/*.toml` | Bundled with active extensions. |

If a project command has the same namespace-qualified name as a user command, the project command wins. Built-in commands win if a custom command would collide with an existing built-in name.

### Agent Skills

Skills are discovered from directory trees containing a `SKILL.md` entry point:

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.gemini/skills/` or `~/.agents/skills/` | Alias takes precedence within the user tier. |
| Windows | User | `%USERPROFILE%\.gemini\skills\` or `%USERPROFILE%\.agents\skills\` | Alias takes precedence within the user tier. |
| macOS / Linux | Repo | `<project-root>/.gemini/skills/` or `<project-root>/.agents/skills/` | Workspace skills require trust. |
| Windows | Repo | `<project-root>\.gemini\skills\` or `<project-root>\.agents\skills\` | Workspace skills require trust. |
| All | Extension | `<extension-path>/skills/` | Bundled with active extensions. |
| All | Built-in | Bundled with the CLI | Lowest precedence, pre-approved. |

Precedence order (lowest to highest) is built-in → extension → user → workspace. Within the same tier, the `.agents/skills/` alias overrides `.gemini/skills/`.

### Local observations

On this machine:

- `~/.gemini/commands/` does **not** exist.
- `~/.gemini/skills/` exists and contains symlinks to `~/.claude/skills/`, indicating the user's setup is sharing Claude Code skills into Gemini's discovery path.
- The current repository has `.gemini/skills/` with symlinks to `.claude/skills/`, and `.gemini/agents/`, but no `.gemini/commands/` directory.
- `~/.gemini/trustedFolders.json` exists.
- `~/.gemini/settings.json` exists with `general.previewFeatures: true` and hooks/tools/security sections.

## File Format

### Custom commands

A custom command is a single TOML file. The file path under the commands directory determines the command name; subdirectories produce colon-separated namespaces.

Examples:

- `~/.gemini/commands/commit.toml` → `/commit`
- `<project>/.gemini/commands/git/commit.toml` → `/git:commit`

Required and optional fields:

| Field | Required | Purpose |
| :---- | :------- | :------ |
| `prompt` | Yes | The prompt sent to the model when the command runs. |
| `description` | No | Shown in `/help`; auto-generated from the filename if omitted. |

Example `git/commit.toml`:

```toml
# Invoked via: /git:commit
description = "Generates a Git commit message based on staged changes."
prompt = """Please generate a Conventional Commit message based on the following git diff:
```diff
!{git diff --staged}
```
"""
```

### Agent Skills

A skill is a directory with a `SKILL.md` entry point and optional supporting folders:

```text
my-skill/
├── SKILL.md       (required)
├── scripts/       (optional)
├── references/    (optional)
└── assets/        (optional)
```

`SKILL.md` uses YAML frontmatter and a Markdown body:

| Field | Required | Purpose |
| :---- | :------- | :------ |
| `name` | Yes | Unique identifier; should match the directory name. |
| `description` | Yes | Used by the model to decide when to activate the skill. |

Example `code-reviewer/SKILL.md`:

```markdown
---
name: code-reviewer
description: Expertise in reviewing code changes for correctness, security, and style. Use when the user asks to "review" their code or a PR.
---

# Code Reviewer Instructions

You act as a senior software engineer specialized in code quality. When this skill is active:

1. Analyze the provided code for logical errors, security vulnerabilities, and style violations.
2. Use the bundled `scripts/review.js` utility when appropriate.
3. Provide constructive feedback, distinguishing critical issues from minor improvements.
```

### Argument and dynamic-content syntax

Custom commands support three substitution and execution constructs:

| Syntax | Meaning |
| :----- | :------ |
| `{{args}}` | Raw text typed after the command name. |
| `!{command}` | Execute a shell command and inject its stdout. |
| `@{path}` | Inject file or directory contents (multimodal-aware). |

Argument handling details:

- If `{{args}}` is present in `prompt`, it is replaced with the user's raw input.
- If `{{args}}` appears inside `!{...}`, it is shell-escaped before substitution.
- If `{{args}}` is absent, the full typed command (e.g. `/changelog 1.2.0 added "New feature"`) is appended to the prompt after two newlines.
- `@{...}` is processed before `!{...}` and `{{args}}`.
- `!{...}` requires balanced braces; unbalanced commands should be moved to a script file.

## Invocation Model

### Custom commands

Type `/` followed by the command name at the start of an interactive message:

```text
> /commit
> /git:fix "Button is misaligned"
> /refactor:pure
```

The CLI resolves the command, performs substitutions and shell/file injections, then sends the final prompt to the model as a user message.

### Agent Skills

Skills are not directly invoked by name. Instead:

1. At session start, the CLI scans discovery tiers and injects skill `name`/`description` metadata into the system prompt.
2. When the model decides a task matches a skill's description, it calls the `activate_skill` tool.
3. The UI shows a consent dialog with the skill name, purpose, and directory path.
4. After approval, the `SKILL.md` body and folder structure are added to context, and the skill directory is added to the agent's allowed file paths.

Management slash commands for skills:

| Slash command | Action |
| :------------ | :----- |
| `/skills list [all] [nodesc]` | List discovered skills. |
| `/skills reload` / `/skills refresh` | Rescan skills without restarting. |
| `/skills disable <name>` | Disable a skill by name. |
| `/skills enable <name>` | Re-enable a disabled skill. |
| `/skills link <path> [--scope user|workspace]` | Link a local skill directory. |

Equivalent terminal commands are available under `gemini skills`.

### Trust and consent

Workspace/project custom commands and workspace skills require folder trust. Trust resolution order is:

1. `GEMINI_CLI_TRUST_WORKSPACE=true` (or `--skip-trust`) → trusted.
2. `security.folderTrust.enabled: false` in settings → trusted.
3. IDE workspace trust signal → trusted/untrusted.
4. `~/.gemini/trustedFolders.json` → trusted/untrusted.

If the workspace is untrusted, project custom commands are not loaded and project skills are skipped. User-level commands and skills still load.

Skill activation always requires per-session user consent, even in trusted workspaces. Installing a skill from a remote URL also requires confirmation unless `--consent` is passed.

## Portability

Gemini CLI custom commands and Agent Skills are **not portable** to other agentic CLIs without rewriting.

What can be reused with transformation:

- The prose Markdown body of a `SKILL.md`.
- The prompt prose inside a custom command `.toml` file.

What is provider-specific and must be rewritten or removed:

- TOML file format (most providers use Markdown with YAML frontmatter).
- `{{args}}` placeholder grammar.
- `!{...}` shell injection and confirmation model.
- `@{...}` file injection.
- Colon namespace syntax derived from directory paths.
- Extension hook structure.
- `activate_skill` tool activation and consent flow.
- `trustedFolders.json` trust model.

Agent Skills follow the [agentskills.io](https://agentskills.io) directory layout, so their physical structure is more portable than TOML custom commands, but metadata and activation behavior still vary across providers.

## Claudine Linking Notes

- Classify Gemini CLI as **first-class slash/skill support** with **non-portable** assets.
- Do not symlink Gemini custom command `.toml` files directly to another provider. Extract the `prompt` body and rewrite it into the target format, mapping `{{args}}` to the target placeholder grammar.
- For Agent Skills, the `SKILL.md` body can be mapped, but frontmatter fields and the activation model require provider-specific handling. The directory layout is closer to a portable standard than TOML commands.
- When syncing from Gemini to another provider, expand `!{...}` shell blocks and `@{...}` file injections into static content or remove them unless the target provider supports equivalent dynamic preprocessing.
- Preserve the namespace distinction: custom commands use `:` from directory paths, while skills use flat names with tier precedence.
- Trust gating is Gemini-specific; cross-provider linking should map workspace/project resources to the target provider's trust/opt-in mechanism rather than assuming `trustedFolders.json` semantics.
- The local machine already links `~/.gemini/skills/` and `.gemini/skills/` to Claude Code skills; this is a user-managed bridge, not native Gemini format equivalence.

## Sources

- [Gemini CLI homepage](https://geminicli.com/)
- [Gemini CLI documentation](https://geminicli.com/docs/)
- [Custom commands documentation](https://geminicli.com/docs/cli/custom-commands/)
- [Agent Skills overview](https://geminicli.com/docs/cli/skills/)
- [Creating Agent Skills](https://geminicli.com/docs/cli/creating-skills/)
- [Using Agent Skills](https://geminicli.com/docs/cli/using-agent-skills/)
- [Command reference](https://geminicli.com/docs/reference/commands/)
- [Configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI source: `packages/cli/src/services/FileCommandLoader.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/services/FileCommandLoader.ts)
- [Gemini CLI source: `packages/core/src/skills/skillManager.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/skills/skillManager.ts)
- [Gemini CLI source: `packages/core/src/skills/skillLoader.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/skills/skillLoader.ts)
- [Gemini CLI source: `packages/core/src/utils/trust.ts`](https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/utils/trust.ts)
- Local inspection of `~/.gemini/`, `~/.gemini/skills/`, `~/.gemini/settings.json`, and `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/.gemini/`
- Local `gemini --help`, `gemini skills --help`, and `gemini extensions --help` output (version 0.46.0)
