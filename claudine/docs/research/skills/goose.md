---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://block.github.io/goose/
docs: https://goose-docs.ai/docs/
skills_docs: https://goose-docs.ai/docs/guides/context-engineering/using-skills/

support: first_class

locations:
  - os: all
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: |
      Canonical global skills directory. Recommended standard location. On Windows resolves to %USERPROFILE%\.agents\skills\.
  - os: all
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: |
      Canonical project-level skills directory. Intended to be shared via version control.
  - os: all
    scope: extension
    path: ~/.agents/plugins/<plugin-name>/
    notes: |
      Installed git-backed or Open Plugin skills. Open-plugin skill names are namespaced as my-plugin:review.
  - os: all
    scope: user
    path: ~/.config/goose/skills/<skill-name>/SKILL.md
    notes: Backward-compatible platform-specific config location.
  - os: all
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: |
      Backward compatibility with Claude Code personal skills. Discovered after canonical ~/.agents/skills/.
  - os: all
    scope: repo
    path: .goose/skills/<skill-name>/SKILL.md
    notes: Backward-compatible project location; lower precedence than .agents/skills/.
  - os: all
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: |
      Backward compatibility with Claude Code project skills. Discovered after .goose/skills/ and .agents/skills/.

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
    A skill is a directory with SKILL.md as the required entry point. Supporting files (scripts/, references/, assets/) are loaded on demand after activation. Goose follows the Agent Skills open standard; name and description are the only required frontmatter fields. Plugin-provided skills may carry additional plugin metadata but are not guaranteed portable.

discovery:
  mechanism: |
    At session start the built-in Summon extension scans built-in, user, project, and plugin skill tiers and injects each skill's name, path, and description into the system prompt. The model then activates a skill automatically when the user's request matches its description, or the user can load one explicitly with "/skills <name>" or a natural-language request such as "Use the code-review skill".
  precedence: |
    Lowest to highest: built-in skills < plugin skills < user skills (~/.agents/skills/, ~/.config/goose/skills/, ~/.claude/skills/) < project skills (.goose/skills/, .claude/skills/, .agents/skills/). Within the project tier, .agents/skills/ takes precedence over .goose/skills/ and .claude/skills/; within the user tier, ~/.agents/skills/ takes precedence over the backward-compatible paths.
  enable_disable: |
    No per-skill disable toggle in the CLI; activation is consent-gated (the model proposes, user approves) except for built-in skills which are pre-approved. Users can decline individual activations. Entire skill discovery can be bypassed by launching without the Summon extension ("--with-builtin" can add or omit builtins), and custom environments can relocate all state via GOOSE_PATH_ROOT.
  notes: |
    The "/skills" slash command lists available skills and can load one or more by name (e.g., "/skills code-review edge-case-finder"). Built-in skills are pre-approved; plugin skills may be namespaced.

portability:
  portable: true
  non_portable_assets:
    - "Scripts in scripts/ — depend on host language runtimes and installed binaries"
    - "Path and project-layout assumptions embedded in SKILL.md"
    - "Plugin-bundled and built-in skills that are not backed by a simple file tree"
    - "Provider-specific frontmatter such as Claude-style tools or allowed-tools syntax"
    - "Goose-specific activation semantics and plugin namespacing"
    - "Recipe-based slash commands and goosehints (.goosehints, AGENTS.md) — separate systems with their own formats"
  rewrite_needed: true
  notes: |
    The Markdown body and Agent Skills standard frontmatter (name, description, license, compatibility, metadata, allowed-tools) are portable across tools that implement the open standard. Any bundled scripts, OS-specific commands, provider-only frontmatter, or Goose-specific activation/plugin mechanics need rewriting or host gating when moving to another provider.

cli_params:
  - flag: /skills [name ...]
    description: In-session command to list available skills or load one or more by name.
    example: /skills code-review edge-case-finder
  - flag: goose plugin install [--auto-update] <git-url>
    description: Install a plugin that may provide skills.
    example: goose plugin install https://github.com/example/my-goose-plugin.git
  - flag: goose plugin update <name>
    description: Update an installed git-backed plugin.
    example: goose plugin update my-plugin
  - flag: goose run --with-builtin <id>[,...]
    description: Enable or disable built-in extensions for the run. Omitting summon disables skill discovery.
    example: goose run --with-builtin developer
  - flag: goose run --recipe <file> [--params KEY=VALUE ...]
    description: Load a reusable YAML recipe. Recipes are Goose's workflow mechanism and can expose themselves as slash commands.
    example: goose run --recipe deploy.yaml --params env=production
  - flag: goose recipe list [--format json] [--verbose]
    description: List available recipes from local directories and configured GitHub repositories.
    example: goose recipe list --format json

env_vars:
  - name: GOOSE_PATH_ROOT
    effect: |
      Overrides the root directory for all goose data, config, and state files, which relocates skill directories under <root>/config/ and <root>/data/. Useful for isolated environments and CI/CD.
  - name: CONTEXT_FILE_NAMES
    effect: |
      JSON array of filenames used for context/hint files (default [".goosehints"]). Not a skill-loading toggle, but determines which persistent instruction files are discovered alongside skills.
  - name: GOOSE_SHELL
    effect: |
      Overrides the shell used by the Developer extension for shell commands and for executing scripts referenced by skills.
  - name: GOOSE_SEARCH_PATHS
    effect: |
      JSON array of additional directories prepended to PATH when extensions run commands. Helps skill scripts find custom binaries.
  - name: GOOSE_MODE
    effect: |
      Tool execution mode (auto, approve, chat, smart_approve). Affects whether skills that invoke tools can run without user approval.

changes: []

requires_claudine_update: true
reason: |
  Claudine's linking module should recognize Goose CLI's canonical skill locations (~/.agents/skills/ and .agents/skills/) as well as its backward-compatible paths (.goose/skills/, .claude/skills/, ~/.claude/skills/, ~/.config/goose/skills/). It also needs to model plugin-provided skills under ~/.agents/plugins/<plugin-name>/ with namespacing, the Summon-extension discovery mechanism, the /skills activation command, and the distinction between portable Agent Skills content and Goose-specific assets such as recipes, goosehints, and plugin metadata.
---

# Goose CLI Skills

## Overview

Goose CLI has first-class **Agent Skills** based on the [Agent Skills](https://agentskills.io) open standard. A skill is a self-contained directory with a `SKILL.md` entry point; it packages instructions and optional resources (`scripts/`, `references/`, `assets/`) into a discoverable, on-demand capability. Skills are discovered at session start by the built-in Summon extension and activated either automatically by the model (when the user's request matches the skill's `description`) or explicitly via the `/skills` slash command or natural-language requests such as "Use the code-review skill".

Goose also maintains related but distinct reuse systems: **recipes** (YAML reusable workflows that can register as slash commands), **goosehints** / `AGENTS.md` (persistent context injection), and **plugins** (git-backed extension bundles that can include skills). This document focuses on skills; the related systems are noted only where they affect linking or portability.

## Locations

Skill resources are stored by scope:

| Scope | Location | Notes |
|---|---|---|
| Built-in | Bundled with the `goose` package via the Summon extension | Pre-approved; listed with `/skills`. |
| Plugin | `~/.agents/plugins/<plugin-name>/` | Open-plugin skill names are namespaced as `my-plugin:review`. |
| User (canonical) | `~/.agents/skills/<skill-name>/SKILL.md` | Recommended global location. |
| Project (canonical) | `.agents/skills/<skill-name>/SKILL.md` | Shareable via version control. |
| User (backward compat) | `~/.config/goose/skills/<skill-name>/SKILL.md` | Platform-specific config location. |
| User (Claude compat) | `~/.claude/skills/<skill-name>/SKILL.md` | Reads Claude Code's personal skills. |
| Project (backward compat) | `.goose/skills/<skill-name>/SKILL.md` | Older Goose project location. |
| Project (Claude compat) | `.claude/skills/<skill-name>/SKILL.md` | Reads Claude Code's project skills. |

On Windows, `~/.agents` resolves to `%USERPROFILE%\.agents`, `~/.config/goose` resolves to `%APPDATA%\Block\goose\config`, and `~/.claude` resolves to `%USERPROFILE%\.claude`.

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
- `description` — trigger phrase describing what the skill does and when to use it.

Optional standard fields include `license`, `compatibility`, `metadata`, and `allowed-tools`. Goose tolerates additional frontmatter, but provider-specific extensions (for example, Claude-style `tools`) are not guaranteed to be honored and should be treated as non-portable.

## Discovery and Precedence

Discovery order, from lowest to highest precedence:

1. Built-in skills (Summon extension).
2. Plugin skills (`~/.agents/plugins/<plugin-name>/`).
3. User skills:
   - `~/.agents/skills/`
   - `~/.config/goose/skills/`
   - `~/.claude/skills/`
4. Project skills:
   - `.agents/skills/`
   - `.goose/skills/`
   - `.claude/skills/`

Within the same tier, canonical `.agents/skills/` paths take precedence over backward-compatible paths. If a project skill has the same `name` as a user skill, the project copy wins.

Activation is consent-gated for non-built-in skills: the model may propose a skill, but the user must approve before the full `SKILL.md` body and bundled files are loaded. Built-in skills are pre-approved. The `/skills` slash command lists available skills and can load one or more by name without waiting for the model to propose them.

## Portability

Portable assets:

- `SKILL.md` Markdown body.
- Agent Skills standard frontmatter: `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`.

Assets that need rewriting or host gating:

- Scripts in `scripts/` (language/runtime availability).
- Path assumptions and project-layout references in the body.
- Provider-specific frontmatter such as Claude-style `tools`.
- Built-in and plugin-bundled skills that are not backed by a simple file tree.
- Goose-specific activation consent and plugin namespacing mechanics.
- Related but separate systems: recipes (YAML), goosehints (`AGENTS.md`, `.goosehints`), and plugin metadata.

## Claudine Linking Notes

For cross-provider linking:

- Treat `~/.agents/skills/<name>/SKILL.md` and `.agents/skills/<name>/SKILL.md` as the canonical user and project skill locations.
- Also recognize backward-compatible paths `.goose/skills/`, `.claude/skills/`, `~/.claude/skills/`, and `~/.config/goose/skills/` for import/discovery, but classify them as secondary to `.agents/skills/`.
- Recognize plugin skills under `~/.agents/plugins/<plugin-name>/` as namespaced, provider-managed assets.
- Classify built-in and plugin-bundled skills as non-portable unless they are mirrored by a simple file-tree skill.
- Mark skills containing `scripts/`, OS-specific commands, or provider-specific frontmatter as needing rewrite.
- Account for Goose's consent-gated activation when deciding whether a linked skill is "active" by default.

## Sources

- [Goose homepage](https://block.github.io/goose/)
- [Goose documentation](https://goose-docs.ai/docs/)
- [Agent Skills guide](https://goose-docs.ai/docs/guides/context-engineering/using-skills/)
- [CLI commands reference](https://goose-docs.ai/docs/guides/goose-cli-commands/)
- [Configuration files](https://goose-docs.ai/docs/guides/config-files/)
- [Environment variables](https://goose-docs.ai/docs/guides/environment-variables/)
- [Custom slash commands](https://goose-docs.ai/docs/guides/context-engineering/slash-commands/)
- [Using goosehints](https://goose-docs.ai/docs/guides/context-engineering/using-goosehints/)
- [Plugins](https://goose-docs.ai/docs/guides/context-engineering/plugins/)
- [Agent Skills open standard](https://agentskills.io/specification)
- [Goose GitHub repository](https://github.com/aaif-goose/goose)
