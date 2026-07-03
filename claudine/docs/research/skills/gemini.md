---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
skills_docs: https://www.geminicli.com/docs/cli/skills/

support: first_class

locations:
  - os: all
    scope: system
    path: Built-in skills bundled with Gemini CLI
    notes: Pre-approved foundational skills. Listed with `/skills list all` or `gemini skills list --all`.
  - os: all
    scope: extension
    path: Skills bundled in installed Gemini CLI extensions
    notes: Loaded only while the parent extension is enabled.
  - os: all
    scope: user
    path: ~/.gemini/skills/<skill-name>/SKILL.md
    notes: |
      Also discovers the `~/.agents/skills/` alias as an interoperable path.
      On Windows this resolves to `%USERPROFILE%\.gemini\skills\` and `%USERPROFILE%\.agents\skills\`.
  - os: all
    scope: repo
    path: .gemini/skills/<skill-name>/SKILL.md
    notes: |
      Also discovers the `.agents/skills/` alias. Workspace skills are intended to be shared via version control.

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
  body_format: markdown
  notes: |
    A skill is a directory with `SKILL.md` as the required entry point. Supporting folders (`scripts/`, `references/`, `assets/`) are loaded on demand after activation. Gemini CLI follows the Agent Skills open standard, so `name` and `description` are the only required frontmatter fields. Extra fields (for example, Claude-style `tools`) are generally ignored by Gemini CLI but are not portable.

discovery:
  mechanism: |
    At session start Gemini CLI scans built-in, extension, user, and workspace skill tiers. Only the `name` and `description` of each discovered skill are injected into the system prompt. When the model decides a task matches a skill, it calls the `activate_skill` tool; the user must approve before the full `SKILL.md` body and bundled resources are loaded.
  precedence: |
    Lowest to highest: built-in skills < extension skills < user skills (`~/.gemini/skills/`, `~/.agents/skills/`) < workspace skills (`.gemini/skills/`, `.agents/skills/`). Within the same tier, the `.agents/skills/` alias takes precedence over the `.gemini/skills/` directory. A workspace skill with the same name as a user skill wins.
  enable_disable: |
    Per-skill: `/skills disable <name>` and `/skills enable <name>` (default `user` scope; use `--scope workspace` for project-specific toggles). Terminal equivalents: `gemini skills disable <name>` and `gemini skills enable <name>`. `/skills reload` or `/skills refresh` rescans all tiers without restarting.
  notes: |
    Every skill activation requires explicit user consent during the session, except for built-in skills which are pre-approved. Installed/linked skills persist on disk; enable/disable state is managed separately.

portability:
  portable: true
  non_portable_assets:
    - "Scripts in `scripts/` — depend on host language runtimes and installed binaries"
    - "Path and project-layout assumptions embedded in `SKILL.md`"
    - "Extension-bundled and built-in skills that are not file-based"
    - "Provider-specific frontmatter such as Claude-style `tools` or `allowed-tools` syntax"
    - "Gemini-specific activation consent and extension namespacing mechanics"
  rewrite_needed: true
  notes: |
    The Markdown body and Agent Skills standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`) are portable across tools that implement the open standard. Any bundled scripts, OS-specific commands, or provider-only frontmatter need rewriting or host gating when moving to another provider.

cli_params:
  - flag: gemini skills list [--all]
    description: Lists discovered agent skills. `--all` includes built-in skills.
    example: gemini skills list --all
  - flag: gemini skills install <source> [--scope user|workspace] [--path <subdir>] [--consent]
    description: Installs a skill from a Git repository URL or local path.
    example: gemini skills install https://github.com/user/repo.git --scope workspace
  - flag: gemini skills link <path> [--scope user|workspace]
    description: Links a local skill directory for development.
    example: gemini skills link ./my-skill --scope workspace
  - flag: gemini skills uninstall <name> [--scope user|workspace]
    description: Removes an installed or linked skill.
    example: gemini skills uninstall my-skill
  - flag: gemini skills enable <name> [--scope user|workspace]
    description: Re-enables a previously disabled skill.
    example: gemini skills enable my-skill
  - flag: gemini skills disable <name> [--scope user|workspace]
    description: Prevents a skill from being triggered.
    example: gemini skills disable my-skill
  - flag: /skills list [all] [nodesc]
    description: In-session command to list discovered skills. `all` includes built-ins; `nodesc` hides descriptions.
    example: /skills list all nodesc
  - flag: /skills reload
    description: Rescans skill tiers without restarting the CLI.
    example: /skills reload

env_vars:
  - name: GEMINI_SYSTEM_MD
    effect: |
      Overrides the core system prompt. Set to `1` or `true` to use `./.gemini/system.md`, or provide a file path. This can change how skill metadata is presented to the model, but it is not a skill-loading toggle.
  - name: GEMINI_WRITE_SYSTEM_MD
    effect: |
      Exports the built-in system prompt to `./.gemini/system.md` or a custom path. Useful for inspecting how skills are introduced to the model.

changes: []

requires_claudine_update: true
reason: |
  Claudine's linking module should recognize Gemini CLI's canonical skill locations (`~/.gemini/skills/` and `.gemini/skills/`) and the interoperable `.agents/skills/` alias, and it should model the built-in/extension/user/workspace precedence tiers. Portability classification must distinguish portable Agent Skills content from Gemini-specific activation semantics, extension-bundled skills, and any bundled scripts.
---

# Gemini CLI Skills

## Overview

Gemini CLI has first-class **Agent Skills** based on the [Agent Skills](https://agentskills.io) open standard. A skill is a self-contained directory with a `SKILL.md` entry point; it packages instructions and optional resources (`scripts/`, `references/`, `assets/`) into a discoverable, on-demand capability. Skills are distinct from the persistent workspace context provided by [`GEMINI.md`](https://www.geminicli.com/docs/cli/gemini-md/) files and from the TOML-based [custom slash commands](https://www.geminicli.com/docs/cli/custom-commands/).

## Locations

Skill resources are stored by scope:

| Scope | Location | Notes |
|---|---|---|
| Built-in | Bundled with the `gemini` package | Pre-approved; listed with `/skills list all`. |
| Extension | Inside installed Gemini CLI extensions | Loaded only when the extension is enabled. |
| User | `~/.gemini/skills/<skill-name>/SKILL.md` | Also discovers `~/.agents/skills/` alias. |
| Workspace | `.gemini/skills/<skill-name>/SKILL.md` | Also discovers `.agents/skills/` alias; shareable via version control. |

On Windows, `~/.gemini` resolves to `%USERPROFILE%\.gemini` and `~/.agents` resolves to `%USERPROFILE%\.agents`.

## File Format

A skill is a directory with `SKILL.md` as the required entry point:

```text
my-skill/
├── SKILL.md       # Required metadata + instructions
├── scripts/       # Optional executable helpers
├── references/    # Optional documentation
└── assets/        # Optional templates/resources
```

`SKILL.md` contains YAML frontmatter between `---` markers followed by Markdown content. The Agent Skills standard requires only:

- `name` — unique identifier (matches directory name).
- `description` — trigger phrase used by the model to decide when to activate the skill.

Optional standard fields include `license`, `compatibility`, `metadata`, and `allowed-tools`. Gemini CLI tolerates additional frontmatter, but provider-specific extensions (for example, Claude-style `tools`) are not guaranteed to be honored and should be treated as non-portable.

## Discovery and Precedence

Discovery order, from lowest to highest precedence:

1. Built-in skills.
2. Extension skills.
3. User skills (`~/.gemini/skills/` and `~/.agents/skills/`).
4. Workspace skills (`.gemini/skills/` and `.agents/skills/`).

Within the same tier, the `.agents/skills/` alias takes precedence over the `.gemini/skills/` directory. If a workspace skill has the same `name` as a user skill, the workspace copy wins.

Activation is consent-gated: the model may propose a skill, but the user must approve before the full `SKILL.md` body and bundled files are injected. Built-in skills are pre-approved. Enable/disable state is managed with `/skills enable|disable <name>` or `gemini skills enable|disable <name>`. `/skills reload` rescans all tiers without restarting.

## Portability

Portable assets:

- `SKILL.md` Markdown body.
- Agent Skills standard frontmatter: `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`.

Assets that need rewriting or host gating:

- Scripts in `scripts/` (language/runtime availability).
- Path assumptions and project-layout references in the body.
- Provider-specific frontmatter such as Claude-style `tools`.
- Extension-bundled and built-in skills that are not backed by a simple file tree.
- Gemini-specific activation consent and extension namespacing.

## Claudine Linking Notes

For cross-provider linking:

- Treat `~/.gemini/skills/<name>/SKILL.md` and `.gemini/skills/<name>/SKILL.md` as the canonical user and workspace skill locations.
- Also recognize the `.agents/skills/` alias in both scopes as an interoperable Agent Skills path.
- Classify built-in and extension skills as non-portable, provider-managed assets.
- Mark skills containing `scripts/`, OS-specific commands, or provider-specific frontmatter as needing rewrite.
- Account for per-skill enable/disable state and the fact that Gemini requires per-session activation consent.

## Changelog

- 2026-07-02 — Initial research document. No implementation changes yet.

## Sources

- [Gemini CLI homepage](https://geminicli.com/)
- [Gemini CLI documentation](https://geminicli.com/docs/)
- [Agent Skills overview](https://www.geminicli.com/docs/cli/skills/)
- [Creating Agent Skills](https://www.geminicli.com/docs/cli/creating-skills/)
- [Using Agent Skills](https://www.geminicli.com/docs/cli/using-agent-skills/)
- [Agent Skills best practices](https://www.geminicli.com/docs/cli/skills-best-practices/)
- [Custom commands](https://www.geminicli.com/docs/cli/custom-commands/)
- [Project context (GEMINI.md)](https://www.geminicli.com/docs/cli/gemini-md/)
- [System prompt override](https://www.geminicli.com/docs/cli/system-prompt/)
- [CLI commands reference](https://www.geminicli.com/docs/reference/commands/)
- [Agent Skills open standard](https://agentskills.io/specification)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
