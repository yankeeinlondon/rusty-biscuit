---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://geminicli.com/
docs: https://geminicli.com/docs/
skills_docs: https://geminicli.com/docs/cli/skills/

support: first_class

locations:
  - os: macos
    scope: system
    path: "<install-prefix>/lib/node_modules/@google/gemini-cli/dist/src/skills/"
    notes: "Built-in skills shipped inside the `@google/gemini-cli` npm package. Pre-approved; do not require per-session consent. Not addressable as user files; not observed locally on this host."
  - os: linux
    scope: system
    path: "<install-prefix>/lib/node_modules/@google/gemini-cli/dist/src/skills/"
    notes: "Built-in skills shipped inside the npm package. Same packaging as macOS."
  - os: windows
    scope: system
    path: "%ProgramFiles%\\nodejs\\node_modules\\@google\\gemini-cli\\dist\\src\\skills\\"
    notes: "Built-in skills shipped inside the npm package. Same packaging as macOS/Linux."
  - os: macos
    scope: user
    path: ~/.gemini/skills/<skill-name>/SKILL.md
    notes: "User-scope skills available across all projects. Also discovered at the `~/.agents/skills/` alias. Symlinks are followed (observed: this host contains 176 symlinks under `~/.gemini/skills/` pointing at `~/.claude/skills/`)."
  - os: linux
    scope: user
    path: ~/.gemini/skills/<skill-name>/SKILL.md
    notes: "User-scope skills available across all projects. Also discovered at the `~/.agents/skills/` alias."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\skills\\<skill-name>\\SKILL.md"
    notes: "User-scope skills. Also discovered at `%USERPROFILE%\\.agents\\skills\\`."
  - os: macos
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: "Interoperable alias for the Agent Skills open standard; takes precedence over `~/.gemini/skills/` within the user scope."
  - os: linux
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: "Interoperable alias; takes precedence over `~/.gemini/skills/` within the user scope."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<skill-name>\\SKILL.md"
    notes: "Interoperable alias; takes precedence over `%USERPROFILE%\\.gemini\\skills\\` within the user scope."
  - os: macos
    scope: repo
    path: .gemini/skills/<skill-name>/SKILL.md
    notes: "Workspace-scope skills committed to the repository and shared via version control. Only loaded when the workspace folder is trusted (`security.folderTrust.enabled`); bypass with `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`."
  - os: linux
    scope: repo
    path: .gemini/skills/<skill-name>/SKILL.md
    notes: "Workspace-scope skills. Same trust requirement as macOS."
  - os: windows
    scope: repo
    path: ".gemini\\skills\\<skill-name>\\SKILL.md"
    notes: "Workspace-scope skills. Same trust requirement as macOS/Linux."
  - os: macos
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: "Interoperable workspace alias; takes precedence over `.gemini/skills/` within the workspace scope when both are present."
  - os: linux
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: "Interoperable workspace alias; takes precedence over `.gemini/skills/` within the workspace scope."
  - os: windows
    scope: repo
    path: ".agents\\skills\\<skill-name>\\SKILL.md"
    notes: "Interoperable workspace alias; takes precedence over `.gemini\\skills\\` within the workspace scope."
  - os: macos
    scope: extension
    path: ~/.gemini/extensions/<extension>/skills/<skill-name>/SKILL.md
    notes: "Skills bundled inside installed extensions. Each extension is a directory under `<home>/.gemini/extensions/` containing `gemini-extension.json`. Discovered only when the parent extension is enabled. Other extension install locations: per-project extensions alongside the workspace."
  - os: linux
    scope: extension
    path: ~/.gemini/extensions/<extension>/skills/<skill-name>/SKILL.md
    notes: "Skills bundled inside installed extensions. Same discovery rules as macOS."
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.gemini\\extensions\\<extension>\\skills\\<skill-name>\\SKILL.md"
    notes: "Skills bundled inside installed extensions. Same discovery rules as macOS/Linux."

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
    A skill is a directory containing `SKILL.md` as the required entry point. Recommended
    layout: `scripts/` (executable helpers), `references/` (deep-dive docs loaded on demand),
    `assets/` (templates and non-executable resources). `SKILL.md` must be at the root of a
    skill directory or one directory deep (`<skills-dir>/<skill-name>/SKILL.md`); deeper
    nesting is not discovered. The filename is case-sensitive (`skill.md`/`Skill.md` are
    ignored on case-sensitive filesystems). Frontmatter must be the very first content of the
    file — no H1, comment, or blank line may precede the opening `---`. Either `name` or
    `description` missing silently skips the file. The skill name is taken from the `name:`
    field, **not** the directory name; the characters `: \ / < > * ? " |` in `name` are
    replaced with `-`. Gemini CLI follows the Agent Skills open standard; provider-specific
    frontmatter (e.g. Claude-style `tools`, `hooks`, `paths`) is ignored at runtime and
    should not be relied on.

discovery:
  mechanism: |
    At session start Gemini CLI scans four discovery tiers and injects each enabled skill's
    `name` + `description` into the system prompt (progressive disclosure — the `SKILL.md`
    body is only loaded after activation). Tiers are scanned in this order: built-in skills
    (bundled in the npm package), extension skills (per `~/.gemini/extensions/<ext>/skills/`),
    user skills (`~/.gemini/skills/` and `~/.agents/skills/`), workspace skills
    (`.gemini/skills/` and `.agents/skills/` relative to the launch directory). Within a tier,
    the `.agents/skills/` alias wins over `.gemini/skills/`. Discovery requires either a
    `SKILL.md` at the root of the skills directory or at `<skills-dir>/<skill-name>/SKILL.md`
    one level deep; deeper paths are ignored. `/skills reload` (alias `/skills refresh`) and
    `gemini skills enable --all` / `gemini skills disable --all` rescan or bulk-toggle
    without restarting. `gemini skills install` reads from a Git URL, a local path, or a
    `.skill` zip archive; `gemini skills link <path>` creates a symlink under the relevant
    scope's skills directory.
  precedence: |
    Lowest to highest precedence on a name collision: built-in < extension < user <
    workspace. Within the user tier: `~/.agents/skills/` beats `~/.gemini/skills/`. Within
    the workspace tier: `.agents/skills/` beats `.gemini/skills/`. Across tiers, the
    higher-precedence directory's `SKILL.md` entirely replaces the lower-precedence one;
    there is no merging.
  enable_disable: |
    Per-skill: `gemini skills disable <name> [--scope user|workspace]` and
    `gemini skills enable <name> [--scope user|workspace]` (in-session: `/skills disable
    <name>`, `/skills enable <name>`). `/skills disable` and `/skills enable` default to the
    `user` scope — use `--scope workspace` to manage workspace toggles. Bulk: `gemini skills
    enable --all`, `gemini skills disable --all`. Per-activation consent is also required at
    runtime (see notes). Global: `skills.enabled` in `settings.json` toggles Agent Skills
    entirely (default `true`). Workspace skills are also gated by `security.folderTrust`
    — see env/CLI.
  notes: |
    Activation is consent-gated: the model proposes a skill via the `activate_skill` tool,
    the user is shown a confirmation prompt with the skill name, purpose, and target
    directory, and only after approval are the `SKILL.md` body and bundled resources injected
    into the conversation. Built-in skills skip the prompt (pre-approved). After approval,
    the skill's directory is added to the agent's allowed file paths so the model can read
    bundled assets. Discovery trusts no environment variables for path roots; the home
    directory is the user-scope root in all three OSes. Workspace-scope skills are listed in
    the trust dialog's "Skills" line; an untrusted workspace does not load them, but
    `~/.gemini/skills/` is unaffected by trust. Bundled-skill toggles (`built-in only`) are
    not exposed; the only way to hide a built-in is to disable the relevant user/workspace
    override if one exists.

portability:
  portable: true
  non_portable_assets:
    - "Bundled executable scripts in `scripts/` — depend on host language runtimes (Node, Python) and binaries available on PATH"
    - "Path and project-layout assumptions embedded in `SKILL.md` body text"
    - "References to Gemini-specific concepts: `activate_skill` tool name, `/skills` slash commands, `gemini skills` CLI subcommand, `.agents/skills` precedence quirk"
    - "Extension-bundled and built-in skills that are not file-based"
    - "Provider-specific frontmatter (e.g. Claude-style `tools`, `disable-model-invocation`, `user-invocable`) — silently ignored by Gemini CLI but flagged by strict linters"
    - "Permission allowlists in `allowed-tools` — Gemini CLI uses the policy engine, not a Claude-style allowlist syntax"
  rewrite_needed: true
  notes: |
    The `SKILL.md` Markdown body and the Agent Skills standard frontmatter (`name`,
    `description`, `license`, `compatibility`, `metadata`, `allowed-tools`) are portable to
    any tool that implements the open standard. Scripts in `scripts/`, OS-specific commands,
    references to Gemini-only tool/command names, and any Claude-specific frontmatter need
    rewriting or host gating. The activation consent flow and the `.agents/skills`/
    `.gemini/skills` precedence rule are Gemini-CLI-specific and do not translate directly.

cli_params:
  - flag: gemini skills list [--all]
    description: List discovered skills. `--all` includes built-in skills.
    example: gemini skills list --all
  - flag: gemini skills install <source> [--consent] [--scope user|workspace] [--path <subdir>]
    description: Install a skill from a Git URL, local path, or `.skill` archive. `--consent` skips the install security prompt.
    example: gemini skills install https://github.com/user/repo.git --consent
  - flag: gemini skills link <path> [--scope user|workspace]
    description: Symlink a local skill directory into the target scope's skills directory for development.
    example: gemini skills link ./my-skill --scope workspace
  - flag: gemini skills uninstall <name> [--scope user|workspace]
    description: Remove an installed or linked skill.
    example: gemini skills uninstall my-skill --scope workspace
  - flag: gemini skills enable <name> [--scope user|workspace]
    description: Re-enable a previously disabled skill.
    example: gemini skills enable my-skill
  - flag: gemini skills disable <name> [--scope user|workspace]
    description: Prevent a skill from being triggered.
    example: gemini skills disable my-skill
  - flag: gemini skills enable --all
    description: Bulk-enable every discovered skill.
    example: gemini skills enable --all
  - flag: gemini skills disable --all
    description: Bulk-disable every discovered skill.
    example: gemini skills disable --all
  - flag: /skills list [all] [nodesc]
    description: In-session list of discovered skills. `all` includes built-ins; `nodesc` hides descriptions.
    example: /skills list all nodesc
  - flag: /skills disable <name> [--scope user|workspace]
    description: In-session per-skill disable (default scope `user`).
    example: /skills disable my-skill
  - flag: /skills enable <name> [--scope user|workspace]
    description: In-session per-skill re-enable (default scope `user`).
    example: /skills enable my-skill --scope workspace
  - flag: /skills link <path> [--scope user|workspace]
    description: In-session variant of `gemini skills link`.
    example: /skills link ./my-skill --scope workspace
  - flag: /skills reload
    description: "Rescan all discovery tiers without restarting. Alias: `/skills refresh`."
    example: /skills reload
  - flag: --skip-trust
    description: Trust the current workspace for this session, bypassing the folder trust dialog and therefore loading workspace-scope skills in untrusted folders.
    example: gemini --skip-trust

env_vars:
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: Set to `true` to trust the current workspace for the duration of the session (equivalent to `--skip-trust`). Indirectly affects skills by gating workspace-scope skills in folders that are otherwise untrusted.
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: Override the path to the `trustedFolders.json` file (default `~/.gemini/trustedFolders.json`). Indirectly affects skills by changing which workspaces are trusted.
  - name: GEMINI_SYSTEM_MD
    effect: Override the core system prompt (`1`/`true` or a file path). Not a skill-loading toggle, but it changes how skill metadata is presented to the model.
  - name: GEMINI_WRITE_SYSTEM_MD
    effect: Export the built-in system prompt to `./.gemini/system.md` (or a custom path). Useful for inspecting how skills are introduced to the model.

changes:
  - "Split location records from `os: all` into separate macOS, Linux, and Windows entries per schema requirements."
  - "Added explicit per-OS records for the built-in, user, workspace, and extension tiers (the previous version collapsed them)."
  - "Documented the `skills.enabled` global toggle in `settings.json` and the `security.folderTrust.enabled` workspace trust gate."
  - "Documented the per-OS Windows path conventions (`%USERPROFILE%\\.gemini\\`, `%USERPROFILE%\\.agents\\`, `%ProgramFiles%\\nodejs\\node_modules\\@google\\gemini-cli\\`)."
  - "Documented the built-in skills location (npm package internals) — was previously described only as 'bundled with Gemini CLI'."
  - "Confirmed the `.agents/skills/` alias precedence quirk and the case-sensitive `SKILL.md` discovery rule from the official get-started tutorial."
  - "Confirmed the workspace trust requirement for workspace-scope skills from the trusted-folders docs and the get-started tutorial."
  - "Documented the bulk `--all` flags on `gemini skills enable|disable` and added `--skip-trust` and `GEMINI_CLI_*` env vars."
  - "Verified symlink-following behavior locally: `~/.gemini/skills/` on this host contains 176 symlinks pointing to `~/.claude/skills/` directories, and those are recognized as discoverable skills."
  - "Verified the `~/.agents/.skill-lock.json` install tracking used by the cross-tool `npx skills` package manager."
  - "Updated model field to `minimax/MiniMax-M3` and bumped `last_updated` to `2026-07-03`."

requires_claudine_update: true
reason: |
  Claudine's linking module should recognize Gemini CLI's first-class Agent Skills layout
  (`~/.gemini/skills/` and `.agents/skills/` for user, `.gemini/skills/` and `.agents/skills/`
  for workspace), the extension-bundled `~/.gemini/extensions/<ext>/skills/` tier, and the
  built-in npm-package tier (not user-linkable). It must model the
  `.agents/skills` > `.gemini/skills` alias precedence within each tier, the
  workspace-scope trust gate (`security.folderTrust` + `--skip-trust` +
  `GEMINI_CLI_TRUST_WORKSPACE`), the `skills.enabled` global toggle, and per-skill
  enable/disable state under the `user` (default) or `workspace` scope. Portability
  classification must distinguish portable Agent Skills content (`SKILL.md` + standard
  frontmatter + generic Markdown body) from Gemini-specific activation semantics
  (`activate_skill` tool, per-session consent flow, `.skill` archive packaging), extension
  bundling, and `scripts/` that depend on host binaries. The `SKILL.md`-only convention,
  case-sensitive filename, and `name`-field-over-directory-name rule should be enforced by
  the linker validator when syncing into a Gemini target.
---

# Gemini CLI Skills

## Overview

Gemini CLI has first-class **Agent Skills** that follow the [Agent Skills](https://agentskills.io) open standard. A skill is a self-contained directory whose entry point is a `SKILL.md` file containing YAML frontmatter and Markdown body. Skills are distinct from the persistent workspace context provided by [`GEMINI.md`](https://geminicli.com/docs/cli/gemini-md/) files, from the TOML-based [custom slash commands](https://geminicli.com/docs/cli/custom-commands/), and from the cross-tool `npx skills` package manager (which targets the `.agents/skills/` interoperable path).

Activation follows a **progressive disclosure** model: at session start, Gemini CLI injects only each skill's `name` and `description` into the system prompt. When the model decides a task matches a description, it calls the `activate_skill` tool, the user is prompted to confirm, and only then is the `SKILL.md` body and the bundled resource directory attached to the conversation.

## Locations

Skills live in one of four tiers. Each OS has independent filesystem paths; on Windows the user-scope root resolves to `%USERPROFILE%\.gemini\` and `%USERPROFILE%\.agents\`.

| Scope | macOS | Linux / WSL | Windows | Notes |
|---|---|---|---|---|
| Built-in | `<install-prefix>/lib/node_modules/@google/gemini-cli/dist/src/skills/` | same | `%ProgramFiles%\nodejs\node_modules\@google\gemini-cli\dist\src\skills\` | Shipped inside the npm package. Pre-approved, no per-session consent. Not addressable as user files. |
| User | `~/.gemini/skills/<skill-name>/SKILL.md` and `~/.agents/skills/<skill-name>/SKILL.md` | same | `%USERPROFILE%\.gemini\skills\` and `%USERPROFILE%\.agents\skills\` | Cross-project. `.agents/skills/` wins on collision. Symlinks are followed (observed locally: 176 symlinks to `~/.claude/skills/` are recognized). |
| Workspace | `.gemini/skills/<skill-name>/SKILL.md` and `.agents/skills/<skill-name>/SKILL.md` | same | `.gemini\skills\` and `.agents\skills\` | Repository-scoped, shareable via VCS. Gated by folder trust (`security.folderTrust`); bypass with `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`. |
| Extension | `~/.gemini/extensions/<extension>/skills/<skill-name>/SKILL.md` | same | `%USERPROFILE%\.gemini\extensions\<extension>\skills\` | Loaded only while the parent extension is enabled. Extension root contains `gemini-extension.json`. |

Locally observed on this host (macOS, July 2026):

- `~/.gemini/skills/` contains 176 symlinks, almost all pointing to `~/.claude/skills/<skill>/` directories (e.g. `claude`, `darkmatter`, `biscuit-terminal`, `cli`, `axum`, `rust`). Confirms that user-scope symlinks are followed during discovery.
- `~/.agents/skills/find-skills/` exists as a real directory (installed by the cross-tool `npx skills add vercel-labs/skills`); its lock file at `~/.agents/.skill-lock.json` records source `https://github.com/vercel-labs/skills.git`, the path `skills/find-skills/SKILL.md`, and the install/updated timestamps.

## File Format

A skill is a directory containing `SKILL.md` as the required entry point. The recommended layout matches the Agent Skills open standard:

```text
my-skill/
├── SKILL.md       (Required) Instructions + YAML frontmatter
├── scripts/       (Optional) Executable helpers (Node, Python, etc.)
├── references/    (Optional) Deep docs, loaded on demand
└── assets/        (Optional) Templates and non-executable resources
```

Discovery rules enforced by Gemini CLI:

1. The filename must be exactly `SKILL.md` (case-sensitive on case-sensitive filesystems).
2. `SKILL.md` must be either at the root of the skills directory (`.gemini/skills/SKILL.md`) or one directory deep (`.gemini/skills/<skill-name>/SKILL.md`). Files nested deeper than one level are not discovered.
3. The opening `---` frontmatter delimiter must be the very first line of the file — no H1, comment, or blank line may precede it.
4. Frontmatter must include both `name` and `description` (matching the Agent Skills open standard). Either missing → file is silently skipped.
5. The skill name comes from the `name:` field, not the directory name. Characters `: \ / < > * ? " |` in `name` are replaced with `-`.

Frontmatter template (verbatim from the official tutorial):

```yaml
---
name: code-reviewer
description: Expertise in reviewing code changes for correctness, security, and style. Use when the user asks to "review" their code or a PR.
---
```

After approval to activate, the body is injected into the conversation and the skill's directory is added to the agent's allowed file paths. The model is instructed to prioritize the skill's procedural guidance within reason.

## Discovery and Precedence

Discovery order, from lowest to highest precedence:

1. **Built-in skills** — bundled in the npm package, pre-approved.
2. **Extension skills** — under `~/.gemini/extensions/<ext>/skills/`, loaded only while the parent extension is enabled.
3. **User skills** — `~/.gemini/skills/` and the `~/.agents/skills/` alias.
4. **Workspace skills** — `.gemini/skills/` and `.agents/skills/` (relative to the launch directory).

Within the user tier, `~/.agents/skills/` wins over `~/.gemini/skills/`. Within the workspace tier, `.agents/skills/` wins over `.gemini/skills/`. Across tiers, the higher-precedence directory's `SKILL.md` entirely replaces the lower-precedence one — there is no merging.

Enable/disable mechanisms:

- **Per-skill, per-scope**: `gemini skills enable <name> [--scope user|workspace]` / `gemini skills disable <name> [--scope user|workspace]`, plus the in-session `/skills enable|disable <name>` variants. The in-session commands default to the `user` scope.
- **Bulk**: `gemini skills enable --all` and `gemini skills disable --all`.
- **Global**: `skills.enabled` in `settings.json` toggles Agent Skills entirely (default `true`).
- **Workspace trust gate**: `.gemini/skills/` and `.agents/skills/` are only loaded when the workspace is trusted (`security.folderTrust.enabled`, default `true`). Bypass with `gemini --skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`. User-scope skills are unaffected by folder trust.
- **Live rescan**: `/skills reload` (alias `/skills refresh`) rescans all tiers without restarting the CLI.

Activation consent is the runtime gate: every non-built-in activation triggers a confirmation prompt that names the skill, summarises its purpose, and reveals the target directory. The skill is only attached to the conversation after the user approves.

## Portability

Portable across any tool that implements the Agent Skills open standard:

- The `SKILL.md` Markdown body.
- The Agent Skills standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`).

Non-portable assets that need rewriting or host gating when moving to another provider:

- Bundled executable scripts in `scripts/` (host runtimes and binaries).
- Path and project-layout assumptions embedded in the body.
- References to Gemini-specific concepts: the `activate_skill` tool name, the `/skills` slash command, the `gemini skills` CLI subcommand, the `.agents/skills` precedence quirk, the `activate_skill` consent flow, the `.skill` zip packaging format.
- Extension-bundled and built-in skills that are not file-based (no `SKILL.md` on the user's disk).
- Provider-specific frontmatter (Claude-style `tools`, `disable-model-invocation`, `user-invocable`, `hooks`, `paths`, `context`, `agent`) — Gemini CLI silently ignores these but strict linters and other providers may flag or misinterpret them.
- `allowed-tools` allowlists — Gemini CLI uses the Policy Engine, not a Claude-style `Skill(...)` allowlist.

## Claudine Linking Notes

For cross-provider resource linking into a Gemini target:

- Use `~/.gemini/skills/<name>/SKILL.md` for the user scope and `.gemini/skills/<name>/SKILL.md` for the workspace scope as the canonical destinations.
- Recognise `~/.agents/skills/<name>/SKILL.md` and `.agents/skills/<name>/SKILL.md` as interoperable Agent Skills aliases; when both are present in the same scope, prefer the `.agents/skills/` destination so the linked resource wins against any pre-existing `.gemini/skills/` copy.
- Place extension-bundled skills under `~/.gemini/extensions/<extension>/skills/<name>/SKILL.md` and require the parent extension's `gemini-extension.json` to declare the extension. Do not link extension skills into user or workspace scopes — they would lose their extension-level lifecycle binding.
- Built-in skills are not addressable from the filesystem; do not try to link them.
- Symlinks are followed: linking a Claude Code skill into Gemini CLI is as simple as `ln -s ~/.claude/skills/<name> ~/.gemini/skills/<name>`, and Gemini CLI will discover it (confirmed locally).
- Enforce on the validator side: case-sensitive `SKILL.md`, leading-`---` frontmatter on line 1, both `name` and `description` present, no characters from `: \ / < > * ? " |` in `name`.
- When syncing into an untrusted workspace, the link is correct but the skill will not load until the folder is trusted or the user passes `--skip-trust` / sets `GEMINI_CLI_TRUST_WORKSPACE=true`. Surface this as a runtime warning.
- Per-skill enable/disable state is per-scope (`user` vs `workspace`) and is not stored in the `SKILL.md` itself; treat it as session metadata rather than file content.
- The bulk `--all` flags and the `skills.enabled` setting in `~/.gemini/settings.json` are user-controlled toggles outside the file format; do not rewrite them as part of a sync.

## Changelog

- **2026-07-03** — Refreshed to match the current official documentation (last updated 2026-04-30 for skills pages, 2026-06-18 for the CLI commands page, repository at v0.49.0). Replaced `os: all` records with per-OS rows for every tier, including the built-in npm-package tier and the Windows path conventions. Added the `skills.enabled` global toggle, the `security.folderTrust` workspace trust gate, the `--skip-trust` flag, and the `GEMINI_CLI_TRUST_WORKSPACE` / `GEMINI_CLI_TRUSTED_FOLDERS_PATH` env vars. Confirmed the case-sensitive `SKILL.md` filename rule, the leading-`---` frontmatter rule, and the `name`-field-over-directory-name rule from the get-started tutorial. Confirmed locally that `~/.gemini/skills/` contains 176 symlinks pointing at `~/.claude/skills/` and that Gemini CLI follows them. Confirmed locally that `~/.agents/skills/find-skills/` (installed via `npx skills add vercel-labs/skills`) is recognized via the `~/.agents/.skill-lock.json` track. Updated model field to `minimax/MiniMax-M3` and bumped `last_updated` to `2026-07-03`.
- **2026-07-02** — Initial research document.

## Sources

- [Gemini CLI — Agent Skills overview](https://geminicli.com/docs/cli/skills/)
- [Gemini CLI — Get started with Agent Skills](https://geminicli.com/docs/cli/tutorials/skills-getting-started/)
- [Gemini CLI — Creating Agent Skills](https://geminicli.com/docs/cli/creating-skills/)
- [Gemini CLI — Managing Agent Skills](https://geminicli.com/docs/cli/using-agent-skills/)
- [Gemini CLI — Skill best practices](https://geminicli.com/docs/cli/skills-best-practices/)
- [Gemini CLI — CLI commands reference](https://geminicli.com/docs/reference/commands/)
- [Gemini CLI cheatsheet (skills management)](https://geminicli.com/docs/cli/cli-reference/#skills-management)
- [Gemini CLI — Settings reference](https://geminicli.com/docs/cli/settings/)
- [Gemini CLI — Trusted Folders](https://geminicli.com/docs/cli/trusted-folders/)
- [Gemini CLI — Extensions overview](https://geminicli.com/docs/extensions/)
- [Gemini CLI — Build Gemini CLI extensions (Agent skills section)](https://github.com/google-gemini/gemini-cli/blob/main/docs/extensions/writing-extensions.md)
- [Gemini CLI — Extension reference](https://github.com/google-gemini/gemini-cli/blob/main/docs/extensions/reference.md)
- [Gemini CLI — `docs/cli/skills.md` (source of truth)](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/skills.md)
- [Gemini CLI homepage](https://geminicli.com/)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
- [Agent Skills open specification](https://agentskills.io/specification)