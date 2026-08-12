---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://qwenlm.github.io/qwen-code-docs/en/users/overview
docs: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/
slash_docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/
support: first_class
locations:
  - os: macos
    scope: user
    path: ~/.qwen/commands/<name>.md
    notes: Personal custom slash commands available in every project. Subdirectories become colon-namespaced commands, e.g. ~/.qwen/commands/git/commit.md → /git:commit.
  - os: linux
    scope: user
    path: ~/.qwen/commands/<name>.md
    notes: Same as macOS; ~ resolves to $HOME.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\commands\\<name>.md"
    notes: Personal custom commands on Windows.
  - os: macos
    scope: repo
    path: .qwen/commands/<name>.md
    notes: Project custom commands. Loaded when running from the project; checked in with the repo. Require folder trust when security.folderTrust.enabled is true.
  - os: linux
    scope: repo
    path: .qwen/commands/<name>.md
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".qwen\\commands\\<name>.md"
    notes: Project custom commands on Windows.
  - os: macos
    scope: user
    path: ~/.qwen/skills/<name>/SKILL.md
    notes: Personal Agent Skills. Skills are model-invoked by default but also user-invokable via /<name> or /skills <name> unless user-invocable is false.
  - os: linux
    scope: user
    path: ~/.qwen/skills/<name>/SKILL.md
    notes: Same as macOS.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\skills\\<name>\\SKILL.md"
    notes: Personal skills on Windows.
  - os: macos
    scope: repo
    path: .qwen/skills/<name>/SKILL.md
    notes: Project skills. Loaded from the project and shared via git. Subject to folder trust when enabled.
  - os: linux
    scope: repo
    path: .qwen/skills/<name>/SKILL.md
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".qwen\\skills\\<name>\\SKILL.md"
    notes: Project skills on Windows.
  - os: macos
    scope: extension
    path: "<extension>/skills/<name>/SKILL.md"
    notes: Extension-provided skills. Declared in the extension's qwen-extension.json. Loaded when the extension is enabled.
  - os: linux
    scope: extension
    path: "<extension>/skills/<name>/SKILL.md"
    notes: Same as macOS.
  - os: windows
    scope: extension
    path: "<extension>\\skills\\<name>\\SKILL.md"
    notes: Extension skills on Windows.
format:
  file_names:
    - "*.md"
    - "*.toml"
  frontmatter: true
  required_fields: []
  optional_fields:
    - description
    - name
    - priority
    - paths
    - user-invocable
    - disable-model-invocation
  argument_syntax: |
    {{args}} is replaced with the raw argument string after the command name.
    If {{args}} is absent, arguments are appended to the prompt body separated by two line breaks.
    Shell command injection uses !{command} and is expanded before {{args}}.
    File content injection uses @{file path} and is expanded before shell commands.
  body_format: markdown
  notes: |
    Custom commands are Markdown files under .qwen/commands/ or ~/.qwen/commands/.
    Deprecated TOML files are still parsed but migration to Markdown is recommended.
    Skills are directories containing SKILL.md plus optional supporting files.
    Subdirectory path separators (/ or \) become colons in the command name.
command_model:
  invocation: |
    Type /name in an interactive session, e.g. /commit or /git:commit.
    Skills are normally model-invoked, but can also be run explicitly via /<skill-name> or /skills <skill-name>.
    Built-in slash commands, custom commands, and skills share the / namespace.
  namespacing: |
    Custom command files map basename to command name; nested directories produce colon-namespaced commands.
    Project commands override user commands when names collide.
    Skills have their own name declared in frontmatter; project skills override personal skills on name collision.
    Exact precedence between a custom command and a skill with the same name is not documented.
  arguments: |
    Everything after the command name is passed as one raw string to {{args}}.
    When {{args}} is omitted, arguments are appended to the rendered prompt with two line breaks of separation.
    Multi-word arguments should be quoted; shell escaping is applied automatically when {{args}} appears inside !{...}.
  output_handling: |
    The rendered Markdown body is sent to the model as a user prompt.
    Expansion order is: @{file} first, then !{shell} (after user confirmation), then {{args}}.
    Dynamic shell output replaces the !{...} marker; file references replace the @{...} marker.
  disabled_mechanism: |
    Remove or rename the command/skill file or directory.
    Set user-invocable: false on a skill to hide it from / and /skills while keeping it model-invokable.
    Set disable-model-invocation: true to hide a skill from the model while keeping it user-invokable.
    Use --disabled-slash-commands, the slashCommands.disabled setting, or QWEN_DISABLED_SLASH_COMMANDS to hide specific slash command names case-insensitively.
  notes: |
    Project-level .qwen/ resources require folder trust when security.folderTrust.enabled is true.
    Skills support path gating via paths: glob patterns; activation persists for the rest of the session once a matching file is touched.
    Skills may include supporting files (scripts, templates) referenced with relative links.
portability:
  portable: false
  non_portable_assets:
    - "{{args}} placeholder and default appending behavior"
    - "!{...} shell command injection syntax"
    - "@{...} file content injection syntax"
    - "Colon namespace syntax from nested command directories"
    - "Skill frontmatter: name, priority, paths, user-invocable, disable-model-invocation"
    - "Qwen-specific settings and trust model"
  rewrite_needed: true
  notes: |
    The prose Markdown body is mostly portable, but every execution-facing and namespacing construct must be rewritten.
    Map {{args}} to the target provider's argument placeholder (e.g. $ARGUMENTS).
    Expand !{...} shell injection and @{...} file references for providers that do not support them natively.
    Map or strip skill frontmatter fields; many have no universal equivalent.
cli_params:
  - flag: --disabled-slash-commands
    description: Slash command names to hide/disable. Merged with slashCommands.disabled and QWEN_DISABLED_SLASH_COMMANDS. Case-insensitive match against the final command name.
    example: qwen --disabled-slash-commands "auth,mcp,extensions"
  - flag: --bare
    description: Minimal mode; skip implicit startup auto-discovery and honor only explicitly provided inputs.
    example: qwen --bare
  - flag: --extensions / -e
    description: Enable only listed extensions for the session. Use qwen -e none to disable all extensions.
    example: qwen -e my-extension
  - flag: --debug
    description: Run in debug mode to surface skill/command loading errors.
    example: qwen --debug
  - flag: --include-directories
    description: Add additional workspace directories; may affect project resource discovery.
    example: qwen --include-directories /path/to/project1
env_vars:
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: Comma-separated slash command names to hide/disable. Unioned with --disabled-slash-commands and slashCommands.disabled.
  - name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
    effect: Override the path to the system defaults settings file.
  - name: QWEN_CODE_SYSTEM_SETTINGS_PATH
    effect: Override the path to the system override settings file.
changes: []
requires_claudine_update: false
reason: Research confirms Qwen Code has first-class custom slash commands and skills. Existing Claudine portability classification (non-portable, rewrite needed) remains accurate.
---

# Qwen CLI Slash Commands and Skills

## Overview

Qwen Code provides two user-defined command surfaces:

* **Custom Commands** — Markdown files stored in `.qwen/commands/` (project) or `~/.qwen/commands/` (user). These are **user-invoked** slash commands: type `/name` to run the prompt stored in the file.
* **Agent Skills** — directories containing `SKILL.md`, stored in `.qwen/skills/` or `~/.qwen/skills/`. Skills are **model-invoked** by default — the model decides when to use them based on the `description` — but they are also directly invokable via `/<skill-name>` or `/skills <skill-name>` unless marked `user-invocable: false`.

Support is **first class**: users can define commands and skills at user, project, and extension scopes; pass arguments; inject shell output and file content; and disable individual entries via frontmatter, settings, CLI flags, or environment variables. Built-in slash commands, custom commands, and skills all share the same `/` namespace.

## Locations

Qwen Code discovers command and skill resources from several scopes. On Windows, `~/.qwen` resolves to `%USERPROFILE%\.qwen`.

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.qwen/commands/<name>.md` | Personal custom commands available in every project. |
| Windows | User | `%USERPROFILE%\.qwen\commands\<name>.md` | Personal custom commands. |
| macOS / Linux | Repo | `.qwen/commands/<name>.md` | Project custom commands, version-controllable. |
| Windows | Repo | `.qwen\commands\<name>.md` | Project custom commands. |
| macOS / Linux | User | `~/.qwen/skills/<name>/SKILL.md` | Personal Agent Skills. |
| Windows | User | `%USERPROFILE%\.qwen\skills\<name>\SKILL.md` | Personal skills. |
| macOS / Linux | Repo | `.qwen/skills/<name>/SKILL.md` | Project skills, shared via git. |
| Windows | Repo | `.qwen\skills\<name>\SKILL.md` | Project skills. |
| All | Extension | `<extension>/skills/<name>/SKILL.md` | Extension-provided skills declared in `qwen-extension.json`. |

### Local observations

On this machine, `~/.qwen/commands/` exists and contains Markdown files such as `commit.md`, `clarify.md`, and nested directories (`fix/`, `local/`, `meta/`, `review/`, `vue/`). Many files are symlinks into `~/.claude/commands/`, indicating the user is sharing command prose across providers. `~/.qwen/skills/` contains skill directories (e.g. `claude/`, `darkmatter/`, `rust/`) that are also symlinks into `~/.claude/skills/`. The repository has no project-level `.qwen/commands/` or `.qwen/skills/` observed.

## File Format

### Custom Commands

A custom command is a single Markdown file with optional YAML frontmatter:

```markdown
---
description: Generate a commit message from staged changes
---

Please generate a conventional commit message for these staged changes:

```diff
!{git diff --staged}
```

User hint: {{args}}
```

Field usage:

| Field | Required | Purpose |
| :---- | :------- | :------ |
| `description` | No | Shown in `/help` and command listings. |
| Body | Yes | Markdown prompt content sent to the model. |

TOML files are **deprecated** but still parsed for backwards compatibility.

### Agent Skills

A skill is a directory whose name is usually the command name, with a required `SKILL.md` entry point:

```text
my-skill/
├── SKILL.md
├── reference.md
├── examples.md
└── scripts/
    └── helper.py
```

Skills use YAML frontmatter with stricter validation:

| Field | Required | Purpose |
| :---- | :------- | :------ |
| `name` | Yes | Command name. Must match `/^[\p{L}\p{N}_:.-]+$/u`. |
| `description` | Yes | Used by the model to decide invocation and shown in listings. |
| `priority` | No | Number; higher values sort earlier in `/skills` listings only. |
| `paths` | No | Glob patterns that gate model discovery until a matching file is touched. |
| `user-invocable` | No | `false` hides the skill from `/` and `/skills` but keeps it model-invokable. |
| `disable-model-invocation` | No | `true` hides the skill from the model but keeps it user-invokable. |

### Argument and injection syntax

Custom commands recognize three expansion tokens:

| Token | Expansion order | Meaning |
| :---- | :-------------- | :------ |
| `@{file path}` | First | Inject file, image, PDF, or directory content. |
| `!{command}` | Second | Execute a shell command (after user confirmation) and inject stdout. |
| `{{args}}` | Last | Replace with the raw argument string after the command name. |

If the body does **not** contain `{{args}}`, any arguments typed after the command are appended to the prompt body separated by two line breaks. When `{{args}}` appears inside `!{...}`, shell escaping is applied automatically.

Example:

```markdown
---
description: Review code against project standards
---

Review {{args}}, reference standards:

@{docs/code-standards.md}
```

## Invocation Model

### How commands are invoked

In an interactive session, type `/` followed by the command name:

* `/commit`
* `/git:commit Message`
* `/my-skill`
* `/skills my-skill`

Subdirectory separators in the command path become colons, so `.qwen/commands/git/commit.md` produces `/git:commit`.

### Namespacing and precedence

Built-in slash commands, custom commands, and skills share a single `/` namespace. Documented precedence:

1. Project custom commands override user custom commands on exact name conflicts.
2. Project skills override personal skills on exact name conflicts.
3. Extension skills are loaded from the enabled extension.

The exact resolution rule when a custom command and a skill share the same name is **not documented** in the official docs; Claudine should treat such collisions as provider-specific and avoid relying on a universal rule.

### Arguments

Everything after the command name is passed as one raw string to `{{args}}`. Multi-word arguments should be quoted. If the body lacks `{{args}}`, the arguments are appended to the rendered prompt with two line breaks of separation.

### Output handling

The rendered Markdown body is sent to the model as a user prompt. Expansion happens in three phases:

1. `@{...}` file references are replaced with file/directory content.
2. `!{...}` shell commands execute after user confirmation and their stdout replaces the marker.
3. `{{args}}` is replaced with the raw argument string (or arguments are appended if absent).

### Disable mechanisms

* Delete, rename, or move the file/directory.
* For skills, set `user-invocable: false` to hide from slash invocation, or `disable-model-invocation: true` to hide from model invocation.
* Use `--disabled-slash-commands` to hide specific names for the session.
* Use the `slashCommands.disabled` setting in any settings scope.
* Set `QWEN_DISABLED_SLASH_COMMANDS` for a session-wide denylist.

All denylist sources are unioned and matched case-insensitively against the final command name.

### Trust and permissions

Project-level `.qwen/` resources are gated by **Trusted Folders** when `security.folderTrust.enabled` is `true`. In an untrusted folder, Qwen Code runs in a restricted safe mode: project settings, `.env` files, extension installation, automatic memory loading, and tool auto-acceptance are disabled. Project commands and skills in `.qwen/` are therefore effectively unavailable until the folder is trusted. User-level `~/.qwen/` resources are not subject to folder trust.

## Portability

Qwen Code custom commands and skills are **not portable** to other agentic CLIs without rewriting.

What can be linked after rewriting:

* The prose Markdown body, after mapping placeholders and expanding dynamic injections.

What is provider-specific and must be rewritten or removed:

* `{{args}}` placeholder and the default two-line-break append behavior.
* `!{...}` shell command injection.
* `@{...}` file content injection.
* Colon namespace syntax produced by nested command directories.
* Skill frontmatter fields: `name`, `priority`, `paths`, `user-invocable`, `disable-model-invocation`.
* Folder trust gating and Qwen-specific settings.

Because the placeholder grammar, injection syntax, and namespace model are Qwen-specific, Claudine should classify these assets as **rewrite needed** rather than linkable as-is.

## Claudine Linking Notes

* Classify Qwen Code as **first-class slash/skill support** with **non-portable** commands.
* Do not symlink Qwen command or skill files directly to another provider; extract the Markdown body and rewrite placeholders/frontmatter.
* For cross-provider sync, map `{{args}}` to the target provider's argument grammar (e.g. `$ARGUMENTS` for Claude Code).
* Expand `!{...}` shell injection and `@{...}` file references for providers that do not support them natively.
* Map skill frontmatter fields individually; many (e.g. `paths`, `priority`, `disable-model-invocation`) have no universal equivalent.
* Project-level resources require folder trust when `security.folderTrust.enabled` is enabled; other providers may need their own opt-in mechanism for repo-level commands.
* The command namespace collision rules (project > user) should be preserved if Claudine builds a unified command index, but collisions between a custom command and a skill with the same name should be treated as undefined/avoided.

## Sources

* [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
* [Commands documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
* [Agent Skills documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/)
* [Settings reference](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
* [Trusted Folders documentation](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/trusted-folders/)
* Local inspection of `~/.qwen/commands/` and `~/.qwen/skills/`
* `qwen --help` output for CLI flags
