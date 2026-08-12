---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7
homepage: https://developers.openai.com/codex
docs: https://developers.openai.com/codex/cli/reference
slash_docs: https://developers.openai.com/codex/cli/slash-commands
support: first_class
locations:
  - os: macos
    scope: user
    path: ~/.agents/skills/<name>/SKILL.md
    notes: Current user skill location. Scanned when CODEX_HOME is unset.
  - os: linux
    scope: user
    path: ~/.agents/skills/<name>/SKILL.md
    notes: Same as macOS.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<name>\\SKILL.md"
    notes: Windows user skill location.
  - os: macos
    scope: user
    path: ~/.codex/skills/<name>/SKILL.md
    notes: Deprecated backward-compatibility path; still scanned.
  - os: linux
    scope: user
    path: ~/.codex/skills/<name>/SKILL.md
    notes: Deprecated backward-compatibility path.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\skills\\<name>\\SKILL.md"
    notes: Deprecated backward-compatibility path.
  - os: macos
    scope: repo
    path: .agents/skills/<name>/SKILL.md
    notes: Repo skills. Codex scans .agents/skills in every directory from cwd up to the repo root.
  - os: linux
    scope: repo
    path: .agents/skills/<name>/SKILL.md
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".agents\\skills\\<name>\\SKILL.md"
    notes: Same as macOS.
  - os: macos
    scope: repo
    path: .codex/skills/<name>/SKILL.md
    notes: Project config folder skills (project_config_folder/skills). Loaded only when the project is trusted.
  - os: linux
    scope: repo
    path: .codex/skills/<name>/SKILL.md
    notes: Same as macOS.
  - os: windows
    scope: repo
    path: ".codex\\skills\\<name>\\SKILL.md"
    notes: Same as macOS.
  - os: macos
    scope: system
    path: ~/.codex/skills/.system/<name>/SKILL.md
    notes: Bundled/system skills cached under CODEX_HOME.
  - os: linux
    scope: system
    path: ~/.codex/skills/.system/<name>/SKILL.md
    notes: Same as macOS.
  - os: windows
    scope: system
    path: "%USERPROFILE%\\.codex\\skills\\.system\\<name>\\SKILL.md"
    notes: Same as macOS.
  - os: macos
    scope: system
    path: /etc/codex/skills/<name>/SKILL.md
    notes: Admin/system skills on Unix.
  - os: linux
    scope: system
    path: /etc/codex/skills/<name>/SKILL.md
    notes: Same as macOS.
  - os: windows
    scope: system
    path: "<system-config-folder>\\skills\\<name>\\SKILL.md"
    notes: Windows system config layer path; exact location depends on system config resolution.
format:
  file_names:
    - SKILL.md
  frontmatter: true
  required_fields:
    - name
    - description
  optional_fields:
    - license
    - compatibility
    - metadata
    - allowed-tools
  argument_syntax: |
    The Agent Skills standard has no argument placeholder grammar. Arguments are supplied as free-form user prompt text after a $SkillName mention or after selecting the skill from the /skills picker. Some Codex skills use scripts/ that read CLI-style arguments, but that is script-level parsing, not a SKILL.md substitution syntax.
  body_format: markdown
  notes: |
    A skill is a directory named after the skill, containing a SKILL.md entry point. Optional subdirectories include scripts/, references/, assets/, and agents/. The agents/openai.yaml file inside a skill is Codex-specific UI metadata and policy, not part of the open Agent Skills core spec. The parent directory name must match the name frontmatter field.
command_model:
  invocation: |
    User-defined skills are not invoked as /name slash commands. Two explicit mechanisms exist: (1) type $SkillName in a prompt, e.g. "$skill-creator build a linter skill"; (2) open the slash picker and choose /skills to browse and select a skill. Implicit invocation occurs when Codex matches the task to a skill description.
  namespacing: |
    Built-in slash commands and user-defined skills do not share a namespace. Slash commands are a closed enum hard-coded in the TUI (e.g. /model, /skills, /clear). Skills are referenced by their name field as $SkillName. If two skills share the same name, Codex does not merge them; both can appear in skill selectors.
  arguments: |
    Arguments are passed as ordinary prompt text after the skill mention or selection. There is no positional $1 or $ARGUMENTS substitution in SKILL.md. Skills that need structured input either parse the remaining prompt text or ship executable scripts in scripts/ that accept command-line arguments.
  output_handling: |
    Codex uses progressive disclosure. At startup it loads only the name and description of every discovered skill. When a skill is selected or matched, the full SKILL.md Markdown body is loaded into the conversation context as instructions for the current turn. Scripts and reference files are loaded only when referenced.
  disabled_mechanism: |
    Delete or rename the skill directory. Per-skill enable/disable can be set in ~/.codex/config.toml with [[skills.config]] entries using path or name and enabled = false. The bundled skills group can be disabled with [skills].bundled = { enabled = false }.
  notes: |
    Project-scoped skill discovery (repo .agents/skills and .codex/skills) requires the project to be trusted. Project trust is recorded in ~/.codex/config.toml under projects."<path>".trust_level = "trusted". Codex detects skill changes automatically; restart if a new skill does not appear.
portability:
  portable: true
  non_portable_assets:
    - agents/openai.yaml (Codex-specific UI metadata, dependencies, policy)
    - Codex plugin packaging and marketplace distribution
    - Bundled skill config and system cache paths
    - Project trust gating in ~/.codex/config.toml
    - "Provider-specific frontmatter extensions (for example, Claude Code's tools: field)"
  rewrite_needed: false
  notes: |
    The core Agent Skills format (SKILL.md with name, description, optional metadata, and Markdown body) is an open standard adopted by multiple providers. A standard skill can be linked or copied to another Agent Skills-compatible provider with only path placement. Codex-specific extensions (agents/openai.yaml, plugin packaging) and provider-specific frontmatter (e.g. Claude's tools:) must be stripped or rewritten when crossing providers.
cli_params:
  - flag: --enable <FEATURE>
    description: Force-enable a feature flag, equivalent to -c features.<name>=true.
    example: codex --enable memories
  - flag: --disable <FEATURE>
    description: Force-disable a feature flag, equivalent to -c features.<name>=false.
    example: codex --disable multi_agent
  - flag: -c, --config <key=value>
    description: Override a config.toml value for this run. Values parse as TOML if possible.
    example: codex -c skills.bundled.enabled=false
  - flag: --profile <NAME>
    description: Layer $CODEX_HOME/<name>.config.toml on top of the base user config.
    example: codex --profile minimal
  - flag: --strict-config
    description: Error when config.toml contains unrecognized fields.
    example: codex --strict-config
  - flag: --dangerously-bypass-hook-trust
    description: Run enabled hooks without persisted hook trust; not a skill-specific flag.
    example: codex --dangerously-bypass-hook-trust
env_vars:
  - name: CODEX_HOME
    effect: Overrides the default ~/.codex root. Used for config, auth, logs, sessions, skills, and bundled skill cache. The directory must already exist.
  - name: CODEX_SQLITE_HOME
    effect: Overrides where SQLite-backed state is stored. Defaults to CODEX_HOME; the sqlite_home config option takes precedence.
  - name: HOME / USERPROFILE
    effect: Resolves the default ~/.codex home and ~/.agents/skills user skill paths when CODEX_HOME is unset.
changes: []
requires_claudine_update: false
reason: |
  Research confirms Codex CLI uses the open Agent Skills standard for user-defined reusable commands. Claudine's command linker should treat Codex skills as a distinct invocation model ($SkillName and /skills picker, not /name slash commands) and classify standard SKILL.md files as portable to other Agent Skills providers.
---

# Codex CLI User-Defined Commands (Skills)

## Overview

Codex CLI does not support user-defined `/name` slash commands. The `/` commands in the interactive TUI are a closed, built-in set (for example `/model`, `/skills`, `/clear`). The user-defined reusable command equivalent is **Agent Skills** — a directory with a `SKILL.md` entry point following the [open Agent Skills standard](https://agentskills.io). Codex calls these simply **skills**.

Skill support is **first class**: users can author skills at user, repo, admin, and system scopes; invoke them explicitly with a `$SkillName` mention or through the `/skills` picker; let Codex match them implicitly by description; and enable or disable individual skills in `config.toml`. Skills can also bundle executable scripts, reference documents, and Codex-specific UI metadata.

## Locations

Codex discovers skills from several scopes. On all platforms, `$CODEX_HOME` defaults to `~/.codex` unless overridden by the environment variable.

| OS | Scope | Path | Notes |
| :- | :---- | :--- | :---- |
| macOS / Linux | User | `~/.agents/skills/<name>/SKILL.md` | Current user skill location. |
| Windows | User | `%USERPROFILE%\.agents\skills\<name>\SKILL.md` | Current user skill location. |
| macOS / Linux | User (deprecated) | `~/.codex/skills/<name>/SKILL.md` | Backward-compatibility path; still scanned. |
| Windows | User (deprecated) | `%USERPROFILE%\.codex\skills\<name>\SKILL.md` | Backward-compatibility path. |
| macOS / Linux | Repo | `.agents/skills/<name>/SKILL.md` | Scanned in every directory from cwd up to the repo root. |
| Windows | Repo | `.agents\skills\<name>\SKILL.md` | Same behavior. |
| macOS / Linux | Repo | `.codex/skills/<name>/SKILL.md` | Project config folder skills; loaded only when the project is trusted. |
| Windows | Repo | `.codex\skills\<name>\SKILL.md` | Same behavior. |
| macOS / Linux | System | `~/.codex/skills/.system/<name>/SKILL.md` | Bundled/system skills cached under `CODEX_HOME`. |
| Windows | System | `%USERPROFILE%\.codex\skills\.system\<name>\SKILL.md` | Same behavior. |
| macOS / Linux | Admin | `/etc/codex/skills/<name>/SKILL.md` | Machine-wide admin skills. |
| Windows | Admin | `<system-config-folder>\skills\<name>\SKILL.md` | Windows system config layer path. |

### Local observations

On the machine used for this research, `~/.codex/skills` is a **symlink to `/Users/ken/.claude/skills`**, and `~/.codex/prompts` contains **symlinks to Claude Code command files**. These are not native Codex skill locations; they appear to be imported or manually linked from Claude Code. The current repo also has `.codex/skills` (symlink) and `.codex/prompts` with Claude command symlinks, but **no `.agents/skills` directory**, which is Codex's native repo skill path.

## File Format

A skill is a directory whose name becomes the skill identifier. The required entry point is `SKILL.md`.

```text
my-skill/
├── SKILL.md          # Required: metadata + instructions
├── scripts/          # Optional: executable code
├── references/       # Optional: documentation
├── assets/           # Optional: templates, resources
└── agents/
    └── openai.yaml   # Optional: Codex-specific UI/policy metadata
```

### Frontmatter

`SKILL.md` uses YAML frontmatter between `---` markers.

| Field | Required | Purpose | Example |
| :---- | :------- | :------ | :------ |
| `name` | Yes | Skill identifier. Must match the parent directory name. Lowercase alphanumeric and hyphens only; 1-64 characters. | `name: code-review` |
| `description` | Yes | When to use the skill. Shown in the skill list and used for implicit matching. Max 1024 characters. | `description: Review Rust code for idiomatic patterns and safety.` |
| `license` | No | License name or reference. | `license: Apache-2.0` |
| `compatibility` | No | Environment requirements. Max 500 characters. | `compatibility: Requires git and Python 3.11+.` |
| `metadata` | No | Arbitrary string-keyed map for tooling. | `metadata: { author: team-platform, version: "1.0" }` |
| `allowed-tools` | No | Experimental space-separated pre-approved tool list. | `allowed-tools: Bash(git:*) Read` |

### Argument syntax

The Agent Skills standard has **no argument placeholder grammar**. Arguments are supplied as free-form prompt text after a `$SkillName` mention or after selecting the skill from the `/skills` picker. For example:

```text
$skill-creator create a skill that reviews Rust PRs
```

Skills that need structured input can either parse the remaining prompt text or ship scripts in `scripts/` that accept their own command-line arguments.

### Body format

The body is Markdown. It contains the instructions Codex follows when the skill is active. Codex loads only the `name` and `description` at startup; the full body is loaded when the skill is selected or matched.

Example `SKILL.md`:

```markdown
---
name: summarize-diff
description: Summarize git diffs for code review. Use when the user asks for a diff summary.
---

Summarize the working tree changes concisely:

1. List files changed.
2. Highlight behavioral changes and risks.
3. Note any missing tests or documentation.
```

### Optional `agents/openai.yaml`

Inside a skill directory, `agents/openai.yaml` is Codex-specific metadata. It is not part of the open Agent Skills core spec. Supported fields include `interface.display_name`, `interface.icon_small`, `interface.icon_large`, `interface.brand_color`, `interface.default_prompt`, `policy.allow_implicit_invocation`, and `dependencies.tools`.

## Invocation Model

### How skills are invoked

User-defined skills are **not** invoked as `/name` slash commands. Two explicit mechanisms exist:

1. **Mention in a prompt**: type `$SkillName` followed by the task. Example: `$skill-creator build a skill for reviewing PRs`.
2. **Slash picker**: type `/skills` and select the skill from the picker.

Codex can also invoke a skill implicitly when the task matches the skill's `description`.

### Namespacing and conflicts

Built-in slash commands and user-defined skills do **not** share a namespace. Slash commands are a hard-coded enum in the TUI; skills are referenced by their `name`. If two skills share the same `name`, Codex does not merge them; both can appear in skill selectors.

### Arguments

Everything after a `$SkillName` mention is treated as normal prompt text. There is no shell-like quoting, positional `$1` substitution, or named argument mapping defined by the skill format. Script-based skills may implement their own argument parsing.

### Output handling

Codex uses **progressive disclosure**:

1. At startup, it loads only the `name` and `description` of every discovered skill into context (capped at roughly 2% of the model context window or 8,000 characters).
2. When a skill is selected or matched, the full `SKILL.md` Markdown body is loaded into the conversation context as instructions for that turn.
3. Files in `scripts/`, `references/`, or `assets/` are loaded only when the skill or the user references them.

### Disable mechanisms

- Delete, rename, or move the skill directory.
- Disable a specific skill in `~/.codex/config.toml`:

    ```toml
    [[skills.config]]
    name = "skill-name"
    enabled = false
    ```

- Disable bundled/system skills as a group:

    ```toml
    [skills]
    bundled = { enabled = false }
    ```

- Restart Codex after changing `config.toml` if the skill list does not update.

### Trust and permissions

Project-scoped skill discovery (`.agents/skills` and `.codex/skills` inside a project) loads only when the project is trusted. Trust is recorded in `~/.codex/config.toml`:

```toml
[projects."/path/to/repo"]
trust_level = "trusted"
```

The `allowed-tools` frontmatter is experimental and support varies by agent implementation.

## Portability

Codex skills are based on the **open Agent Skills standard**, so the core artifact is portable across providers that support the standard (Claude Code, Gemini CLI, OpenCode, Goose, and others).

What links or copies with only path placement:

- The `SKILL.md` file and its standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`).
- The Markdown body.
- `scripts/`, `references/`, and `assets/` directories.

What is Codex-specific and must be stripped or rewritten:

- `agents/openai.yaml` UI metadata and policy.
- Plugin packaging and marketplace distribution.
- Bundled skill config and system cache paths.
- Project trust gating recorded in `~/.codex/config.toml`.
- Provider-specific frontmatter extensions, such as Claude Code's `tools:` field.

Claudine should classify standard Codex skills as **portable** with a small list of non-portable extensions, rather than requiring a full rewrite.

## Claudine Linking Notes

- Classify Codex CLI as having **first-class user-defined reusable commands**, but clarify that they are **skills invoked as `$SkillName` or via `/skills`**, not `/name` slash commands.
- Do not map Codex skills to a `/` command namespace; they are a separate surface.
- Standard `SKILL.md` files can be linked or copied to other Agent Skills-compatible providers with only path placement.
- Strip or rewrite Codex-specific extensions (`agents/openai.yaml`) and provider-specific frontmatter (Claude's `tools:`) when crossing providers.
- The local `~/.codex/skills` and `.codex/prompts` symlinks observed on this machine are Claude Code artifacts, not native Codex skill locations; avoid treating them as ground truth for Codex discovery paths.
- Preserve the open standard's directory-name-equals-name constraint and the `name`/`description` required fields when generating or validating skills.

## Sources

- [Codex CLI overview](https://developers.openai.com/codex/cli)
- [Codex CLI slash commands](https://developers.openai.com/codex/cli/slash-commands)
- [Codex CLI command line options](https://developers.openai.com/codex/cli/reference)
- [Codex skills documentation](https://developers.openai.com/codex/skills)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Agent Skills specification](https://agentskills.io/specification)
- [Agent Skills overview](https://agentskills.io)
- [OpenAI Codex GitHub repository](https://github.com/openai/codex)
- Local inspection of `~/.codex/`, `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/.codex/`, and Codex CLI source (`codex-rs/core-skills/src/loader.rs`, `codex-rs/tui/src/slash_command.rs`, `codex-rs/core/src/agents_md.rs`)
