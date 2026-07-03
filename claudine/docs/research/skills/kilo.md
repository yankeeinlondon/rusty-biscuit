---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default
homepage: https://kilo.ai/
docs: https://kilo.ai/docs/code-with-ai/platforms/cli
skills_docs: https://kilo.ai/docs/customize/skills
support: first_class
locations:
  - os: macos
    scope: user
    path: "~/.kilo/skills"
    notes: "Canonical user skill directory documented by Kilo; marketplace global installs also target this directory."
  - os: linux
    scope: user
    path: "~/.kilo/skills"
    notes: "Canonical user skill directory documented by Kilo; marketplace global installs also target this directory."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kilo\\skills"
    notes: "Canonical user skill directory documented by Kilo."
  - os: macos
    scope: user
    path: "~/.kilocode/skills"
    notes: "Legacy user skill directory accepted by the source implementation before ~/.kilo."
  - os: linux
    scope: user
    path: "~/.kilocode/skills"
    notes: "Legacy user skill directory accepted by the source implementation before ~/.kilo."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kilocode\\skills"
    notes: "Legacy user skill directory accepted by the source implementation before ~/.kilo."
  - os: macos
    scope: user
    path: "~/.agents/skills"
    notes: "Compatibility directory loaded by default unless external skills are disabled."
  - os: linux
    scope: user
    path: "~/.agents/skills"
    notes: "Compatibility directory loaded by default unless external skills are disabled."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills"
    notes: "Compatibility directory loaded by default unless external skills are disabled."
  - os: macos
    scope: user
    path: "~/.claude/skills"
    notes: "Claude Code compatibility directory loaded unless external skills or Claude Code compatibility skills are disabled."
  - os: linux
    scope: user
    path: "~/.claude/skills"
    notes: "Claude Code compatibility directory loaded unless external skills or Claude Code compatibility skills are disabled."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills"
    notes: "Claude Code compatibility directory loaded unless external skills or Claude Code compatibility skills are disabled."
  - os: macos
    scope: user
    path: "~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code/skills"
    notes: "VS Code extension global storage location included in Kilo skill directory discovery when the directory exists."
  - os: linux
    scope: user
    path: "~/.config/Code/User/globalStorage/kilocode.kilo-code/skills"
    notes: "VS Code extension global storage location included in Kilo skill directory discovery when the directory exists."
  - os: windows
    scope: user
    path: "%APPDATA%\\Code\\User\\globalStorage\\kilocode.kilo-code\\skills"
    notes: "VS Code extension global storage location included in Kilo skill directory discovery when the directory exists."
  - os: macos
    scope: user
    path: "$XDG_CONFIG_HOME/kilo/{skill,skills}"
    notes: "Additional global config-root skill directories; defaults to ~/.config/kilo/{skill,skills} when XDG_CONFIG_HOME is unset."
  - os: linux
    scope: user
    path: "$XDG_CONFIG_HOME/kilo/{skill,skills}"
    notes: "Additional global config-root skill directories; defaults to ~/.config/kilo/{skill,skills} when XDG_CONFIG_HOME is unset."
  - os: windows
    scope: user
    path: "%LOCALAPPDATA%\\kilo\\{skill,skills}"
    notes: "Additional global config-root skill directories from xdg-basedir's Windows config root."
  - os: macos
    scope: repo
    path: ".kilo/{skill,skills}"
    notes: "Project config directory scanned from the launch directory up to the git worktree root; project entries load after user entries."
  - os: linux
    scope: repo
    path: ".kilo/{skill,skills}"
    notes: "Project config directory scanned from the launch directory up to the git worktree root; project entries load after user entries."
  - os: windows
    scope: repo
    path: ".kilo\\{skill,skills}"
    notes: "Project config directory scanned from the launch directory up to the git worktree root; project entries load after user entries."
  - os: macos
    scope: repo
    path: ".kilocode/{skill,skills}"
    notes: "Legacy project config directory scanned before .kilo at each level, so .kilo wins on duplicate names."
  - os: linux
    scope: repo
    path: ".kilocode/{skill,skills}"
    notes: "Legacy project config directory scanned before .kilo at each level, so .kilo wins on duplicate names."
  - os: windows
    scope: repo
    path: ".kilocode\\{skill,skills}"
    notes: "Legacy project config directory scanned before .kilo at each level, so .kilo wins on duplicate names."
  - os: macos
    scope: repo
    path: ".agents/skills"
    notes: "Compatibility directory found by walking from launch directory to worktree root when external skills are enabled."
  - os: linux
    scope: repo
    path: ".agents/skills"
    notes: "Compatibility directory found by walking from launch directory to worktree root when external skills are enabled."
  - os: windows
    scope: repo
    path: ".agents\\skills"
    notes: "Compatibility directory found by walking from launch directory to worktree root when external skills are enabled."
  - os: macos
    scope: repo
    path: ".claude/skills"
    notes: "Compatibility directory found by walking from launch directory to worktree root when external and Claude Code skills are enabled."
  - os: linux
    scope: repo
    path: ".claude/skills"
    notes: "Compatibility directory found by walking from launch directory to worktree root when external and Claude Code skills are enabled."
  - os: windows
    scope: repo
    path: ".claude\\skills"
    notes: "Compatibility directory found by walking from launch directory to worktree root when external and Claude Code skills are enabled."
  - os: macos
    scope: other
    path: "kilo.jsonc skills.paths entries"
    notes: "Each configured path is scanned for **/SKILL.md; absolute, ~/ home-relative, and project-root-relative paths are accepted."
  - os: linux
    scope: other
    path: "kilo.jsonc skills.paths entries"
    notes: "Each configured path is scanned for **/SKILL.md; absolute, ~/ home-relative, and project-root-relative paths are accepted."
  - os: windows
    scope: other
    path: "kilo.jsonc skills.paths entries"
    notes: "Each configured path is scanned for **/SKILL.md; absolute, ~/ home-relative, and project-root-relative paths are accepted."
  - os: macos
    scope: other
    path: "$XDG_CACHE_HOME/kilo/skills"
    notes: "Cache root for skills downloaded from skills.urls; defaults to ~/.cache/kilo/skills."
  - os: linux
    scope: other
    path: "$XDG_CACHE_HOME/kilo/skills"
    notes: "Cache root for skills downloaded from skills.urls; defaults to ~/.cache/kilo/skills."
  - os: windows
    scope: other
    path: "%LOCALAPPDATA%\\kilo\\Cache\\skills"
    notes: "Cache root for skills downloaded from skills.urls, subject to xdg-basedir's Windows cache mapping."
  - os: macos
    scope: system
    path: "builtin:kilo-config"
    notes: "Built-in skill bundled inside the CLI binary; user, project, configured, or URL skills with the same name override it."
  - os: linux
    scope: system
    path: "builtin:kilo-config"
    notes: "Built-in skill bundled inside the CLI binary; user, project, configured, or URL skills with the same name override it."
  - os: windows
    scope: system
    path: "builtin:kilo-config"
    notes: "Built-in skill bundled inside the CLI binary; user, project, configured, or URL skills with the same name override it."
format:
  file_names: ["SKILL.md"]
  frontmatter: true
  required_fields: ["name"]
  optional_fields: ["description", "license", "compatibility", "metadata"]
  body_format: markdown
  notes: "A skill is a directory containing SKILL.md. YAML frontmatter is parsed; implementation currently accepts name plus optional description, while docs and the Agent Skills spec describe name and description as required. Kilo stores the Markdown body separately as the loadable skill content. Optional sibling files and directories such as scripts/, references/, assets/, templates, and examples may be bundled and are surfaced to the agent as files relative to the skill directory."
discovery:
  mechanism: "On session startup, Kilo scans compatibility directories, Kilo config directories, configured local paths, and configured remote URL caches; it reads frontmatter metadata first and adds available skills to the system prompt, then the model invokes the skill tool by name to load the full SKILL.md body and a sample of sibling files."
  precedence: "The implementation is last-one-wins by frontmatter name. Built-in skills are seeded first, then discovered skills override them. External compatibility dirs are scanned before Kilo config dirs. Kilo config directories start with the global XDG config root, then project .kilocode/.kilo directories from launch directory to worktree root, then home .kilocode/.kilo, then KILO_CONFIG_DIR. Legacy .kilocode is before .kilo, so .kilo wins at the same level. Configured skills.paths and downloaded skills.urls are scanned after config directories, so they can override earlier skills by name."
  enable_disable: "Disable a skill by removing or renaming its SKILL.md or deleting it from skills.paths/skills.urls. VS Code has a remove action for discovered non-built-in skills; CLI/backend removal deletes only SKILL.md for filesystem skills and refuses built-in and URL-backed cached skills. KILO_DISABLE_EXTERNAL_SKILLS disables .agents and .claude compatibility directories. KILO_DISABLE_CLAUDE_CODE or KILO_DISABLE_CLAUDE_CODE_SKILLS disables only .claude/skills while leaving .agents/skills enabled. Agent permission rules can deny the skill tool or individual skill names."
  notes: "Skills are re-scanned for each new session. The prompt lists skills sorted by name, but duplicate resolution happens during load order. There is no documented trust prompt specific to repo skills; project config can be skipped with KILO_DISABLE_PROJECT_CONFIG, and ordinary Kilo permission policy controls whether the skill tool may load a named skill. No evidence was found that extension/plugin packages contribute skills directly, apart from the VS Code extension marketplace installer writing files into normal skill directories."
portability:
  portable: false
  non_portable_assets: ["Kilo-specific builtin:kilo-config skill", "permission policy for tool name skill", "kilo.jsonc skills.paths and skills.urls configuration", "remote index.json cache layout", "VS Code marketplace install metadata and tarball workflow", "provider-specific instructions that assume Kilo tools or Kilo config"]
  rewrite_needed: true
  notes: "Plain Agent Skills directories containing SKILL.md plus relative bundled assets are portable to other Agent Skills implementations when the frontmatter name and description are valid. Claudine should rewrite or drop Kilo-specific config surfaces: skills.paths/skills.urls, URL cache artifacts, Kilo permission examples, and references to Kilo-only tools or commands. Compatibility directories such as .claude/skills and .agents/skills can be linked as-is only when the target provider accepts the same Agent Skills structure."
cli_params:
  - flag: "--pure"
    description: "Sets KILO_PURE=1 and disables external plugins; no source evidence that it disables skill discovery directly."
    example: "kilo --pure"
  - flag: "--agent"
    description: "Selects the active agent; that agent's permission policy can hide or deny skills."
    example: "kilo run --agent code \"use api-design\""
  - flag: "--dangerously-skip-permissions"
    description: "Auto-approves permissions that are not explicitly denied; affects the skill tool permission prompt at runtime, not discovery."
    example: "kilo run --dangerously-skip-permissions \"use my-skill\""
  - flag: "--auto"
    description: "Auto-approves all permissions for autonomous runs; affects the skill tool permission prompt at runtime, not discovery."
    example: "kilo run --auto \"use my-skill\""
env_vars:
  - name: "KILO_DISABLE_EXTERNAL_SKILLS"
    effect: "When true, skips .agents/skills and .claude/skills compatibility directories at both user and project scope."
  - name: "KILO_DISABLE_CLAUDE_CODE"
    effect: "Broad Claude Code compatibility disable; also disables .claude/skills discovery."
  - name: "KILO_DISABLE_CLAUDE_CODE_SKILLS"
    effect: "Disables .claude/skills discovery without disabling .agents/skills."
  - name: "KILO_CONFIG"
    effect: "Loads an additional kilo.json/kilo.jsonc file; its skills.paths or skills.urls can add skill sources."
  - name: "KILO_CONFIG_DIR"
    effect: "Adds an explicit config directory to the search list; {skill,skills}/**/SKILL.md under that directory are scanned."
  - name: "KILO_CONFIG_CONTENT"
    effect: "Inline JSON config with high precedence; can define skills.paths and skills.urls."
  - name: "KILO_DISABLE_PROJECT_CONFIG"
    effect: "Skips project config files and project .kilo/.kilocode config directories, which also skips repo skills in those Kilo config directories; compatibility directory discovery has separate external-skill flags."
  - name: "KILO_PURE"
    effect: "Disables external plugins; no direct skill discovery effect found, but it can prevent plugin-provided behavior around skills."
  - name: "KILO_CLIENT"
    effect: "Selects client mode such as cli, vscode, jetbrains, or acp; no direct discovery effect found, but client mode changes available tools and UI surfaces."
  - name: "HOME"
    effect: "Primary home root used for ~/.kilo, ~/.kilocode, ~/.agents, and ~/.claude skill directories."
  - name: "USERPROFILE"
    effect: "Fallback home root on Windows when HOME is unavailable in legacy path helper code."
  - name: "XDG_CONFIG_HOME"
    effect: "Changes the global config root used for $XDG_CONFIG_HOME/kilo/{skill,skills} and global kilo.jsonc skills configuration."
  - name: "XDG_CACHE_HOME"
    effect: "Changes the cache root used for URL-downloaded skills under $XDG_CACHE_HOME/kilo/skills."
  - name: "APPDATA"
    effect: "Changes the Windows VS Code globalStorage base used by the helper path for marketplace/global-storage skills."
changes: []
requires_claudine_update: true
reason: "Claudine's Kilo linker should include Kilo as first-class skills support, add .kilo/.kilocode skill and skills directories plus compatibility directories, classify URL-backed and built-in skills as non-portable, and model Kilo's last-one-wins duplicate handling."
---

# Kilo Code Agent Skills

## Overview

Kilo Code implements Agent Skills as durable directories containing a `SKILL.md` file with YAML frontmatter and Markdown instructions. The official documentation describes the workflow as metadata discovery at session start, prompt inclusion of relevant skill names and descriptions, and on-demand loading when the model calls the `skill` tool. Source code matches that shape: `Skill.state()` builds a registry, `SystemPrompt.skills()` injects an `<available_skills>` block, and `tool/skill.ts` loads the selected skill body and reports the base directory for sibling files.

The implementation is first-class and provider-specific in these ways:

- It has a dedicated `skill` tool.
- It has documented user, project, compatibility, configured-path, and remote-URL sources.
- It ships at least one built-in system skill, `kilo-config`.
- It applies Kilo permission policy to the `skill` tool and to skill names.

The local host inspection found no `~/.kilo` directory and no local `~/.kilo/skills` resources. A local `kilo` and `kilocode` CLI were installed at version `7.3.45`; `kilo --help`, `kilo run --help`, and `kilo serve --help` showed no skill-specific CLI path flags. They did expose runtime flags such as `--agent`, `--auto`, `--dangerously-skip-permissions`, and `--pure`, which affect agent selection, permission prompting, or plugin loading rather than skill discovery paths.

## Locations

Kilo has more skill locations than the short public examples imply. The canonical user directory is `~/.kilo/skills`, and the canonical project directory is `.kilo/skills`. Source also accepts singular `skill/` under Kilo config directories, legacy `.kilocode`, compatibility `.agents` and `.claude`, VS Code extension global storage, extra configured paths, remote URL cache directories, and built-ins.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| User canonical | `~/.kilo/skills` | `~/.kilo/skills` | `%USERPROFILE%\.kilo\skills` | Documented global skill directory and VS Code marketplace global install target. |
| User legacy | `~/.kilocode/skills` | `~/.kilocode/skills` | `%USERPROFILE%\.kilocode\skills` | Source helper scans before `~/.kilo`, so `~/.kilo` wins for duplicate names. |
| User compatibility | `~/.agents/skills` | `~/.agents/skills` | `%USERPROFILE%\.agents\skills` | Loaded by default unless `KILO_DISABLE_EXTERNAL_SKILLS` is true. |
| User Claude compatibility | `~/.claude/skills` | `~/.claude/skills` | `%USERPROFILE%\.claude\skills` | Loaded unless external skills or Claude Code skills are disabled. |
| VS Code global storage | `~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code/skills` | `~/.config/Code/User/globalStorage/kilocode.kilo-code/skills` | `%APPDATA%\Code\User\globalStorage\kilocode.kilo-code\skills` | Included when the directory exists. |
| Global config root | `$XDG_CONFIG_HOME/kilo/{skill,skills}` | `$XDG_CONFIG_HOME/kilo/{skill,skills}` | xdg-basedir Windows config root plus `kilo\{skill,skills}` | Defaults to `~/.config/kilo/{skill,skills}` on macOS and Linux when `XDG_CONFIG_HOME` is unset. |
| Project canonical | `.kilo/{skill,skills}` | `.kilo/{skill,skills}` | `.kilo\{skill,skills}` | Scanned from launch directory up to git worktree root. |
| Project legacy | `.kilocode/{skill,skills}` | `.kilocode/{skill,skills}` | `.kilocode\{skill,skills}` | Scanned before `.kilo` at the same level. |
| Project compatibility | `.agents/skills` | `.agents/skills` | `.agents\skills` | Found by walking up to the worktree root when external skills are enabled. |
| Project Claude compatibility | `.claude/skills` | `.claude/skills` | `.claude\skills` | Found by walking up to the worktree root when external and Claude Code skills are enabled. |
| Configured paths | `skills.paths` entries | `skills.paths` entries | `skills.paths` entries | Absolute paths, `~/` paths, and project-root-relative paths are accepted and scanned with `**/SKILL.md`. |
| Remote URL cache | `$XDG_CACHE_HOME/kilo/skills` | `$XDG_CACHE_HOME/kilo/skills` | xdg-basedir Windows cache root plus `kilo\skills` | `skills.urls` downloads declared files here, then scans cached directories. |
| Built-in | `builtin:kilo-config` | `builtin:kilo-config` | `builtin:kilo-config` | Bundled in the CLI binary and overridden by any discovered skill named `kilo-config`. |

Remote URLs are configured in `kilo.jsonc`:

```jsonc
{
  "skills": {
    "paths": ["/path/to/shared/skills", "~/my-skills", "relative/skills"],
    "urls": ["https://example.com/.well-known/skills/"]
  }
}
```

A remote URL must serve `index.json` at the URL root. Kilo fetches `{url}/index.json`, requires each skill entry to include `SKILL.md` in its `files` array, downloads each listed file from `{url}/{skill-name}/{file}`, stores it under the cache root, and scans the cached skill directory.

```json
{
  "skills": [
    { "name": "skill-name", "files": ["SKILL.md", "references/file.md"] }
  ]
}
```

## File Format

The accepted file name is exactly `SKILL.md`. Under Kilo config directories, the glob is `{skill,skills}/**/SKILL.md`; under compatibility directories the glob is `skills/**/SKILL.md`; under configured paths and URL cache directories the glob is `**/SKILL.md`.

The public Kilo docs follow the Agent Skills specification and list these frontmatter fields:

| Field | Required by docs/spec | Accepted by implementation | Notes |
|---|---:|---:|---|
| `name` | Yes | Yes | Registry key. The source implementation does not currently enforce the documented parent-directory name match. |
| `description` | Yes | Optional | Included in the system prompt only when present. Skills without descriptions load into the registry but are omitted from the visible available-skills prompt formatting. |
| `license` | No | Ignored by current loader | Portable metadata for other Agent Skills implementations. |
| `compatibility` | No | Ignored by current loader | Portable metadata for environment requirements; Kilo does not enforce it in the inspected loader. |
| `metadata` | No | Ignored by current loader | Arbitrary mapping for other consumers. |

Example:

```markdown
---
name: api-design
description: REST API design best practices and conventions.
license: Apache-2.0
metadata:
  author: example-org
  version: 1.0.0
---

# API Design Guidelines

Use plural nouns for resources, stable status codes, and consistent error shapes.
```

The loader stores `content` as the Markdown body after frontmatter, not the full file. When the `skill` tool runs, it emits the content inside `<skill_content name="...">`. For filesystem-backed skills it also includes the skill base directory as a file URL, tells the model that relative paths are relative to that base directory, and samples up to 10 sibling files found with ripgrep while excluding `SKILL.md`. This means bundled `scripts/`, `references/`, `assets/`, templates, examples, and media can be used, but the model must read or execute them through normal tools after the skill is loaded.

## Discovery and Precedence

Discovery happens at session initialization. In the CLI, Kilo rescans skills when a new TUI session starts or when `kilo run` starts a new run. In the VS Code extension, skills are loaded when the extension connects to the CLI server. Existing sessions do not automatically pick up newly edited skill files.

The source discovery order is:

1. External compatibility directories, unless `KILO_DISABLE_EXTERNAL_SKILLS` is set:
   `.claude/skills` if Claude Code skills are enabled, then `.agents/skills`, first under the home directory and then from launch directory up to the worktree root.
2. Kilo config directories from `ConfigPaths.directories()`:
   global XDG config root, project `.kilocode` and `.kilo` directories from launch directory to worktree root, home `.kilocode` and `.kilo`, then `KILO_CONFIG_DIR` when set.
3. `skills.paths` from merged Kilo config, expanded as absolute, home-relative, or project-root-relative paths.
4. `skills.urls` from merged Kilo config, downloaded into the cache and scanned.

The registry is keyed by frontmatter `name`, and duplicates are last-one-wins. Source logs a duplicate warning and overwrites the earlier entry. The built-in `kilo-config` skill is seeded before all discovery so any filesystem or URL skill named `kilo-config` overrides it. The docs state project skills take precedence over global skills; the implementation achieves this for Kilo config directories because project `.kilo` directories load after global roots. Configured `skills.paths` and URL-backed skills load even later, so they can override both global and project skills by name.

Kilo does not use separate mode-specific skill directories on the new platform. All discovered skills enter one pool and the agent decides whether a description clearly applies to the current task. Explicit user invocation by name works because the skill names are visible in the prompt.

Skills can be enabled by creating a valid `SKILL.md` in a scanned directory, adding a path to `skills.paths`, or adding a URL to `skills.urls`. They can be disabled by removing or renaming `SKILL.md`, removing the source path or URL from config, or denying the `skill` tool or a specific skill name in Kilo permission policy. VS Code exposes a discovered-skills list and a remove action for non-built-in skills; the backend removal path refuses built-ins, refuses cached URL-backed skills, and deletes only the `SKILL.md` manifest for ordinary filesystem skills.

No skill-specific workspace trust prompt was found. Repo-scope Kilo config directories can be skipped with `KILO_DISABLE_PROJECT_CONFIG`, but external compatibility directory discovery is controlled by `KILO_DISABLE_EXTERNAL_SKILLS` and `KILO_DISABLE_CLAUDE_CODE_SKILLS` rather than by that project-config flag in the inspected source. Kilo permissions still gate runtime use of the `skill` tool.

## Portability

Kilo uses the open Agent Skills shape, so the core artifact is partially portable:

- Portable as-is: a directory with `SKILL.md`, YAML `name` and `description`, Markdown instructions, and relative bundled files that do not assume Kilo-only tools.
- Needs path placement only: skills in `.agents/skills` or `.claude/skills` can be linked to providers that scan those same paths.
- Needs rewrite: `kilo.jsonc` `skills.paths` and `skills.urls`, remote `index.json` registries, Kilo permission examples, and any instructions that say to use Kilo-specific commands, tools, config files, or marketplace flows.
- Non-portable: the built-in `kilo-config` skill, VS Code marketplace staging/install metadata, URL cache directories under `$XDG_CACHE_HOME/kilo/skills`, and provider-specific runtime permission state.

The frontmatter itself is mostly portable, but Claudine should prefer the stricter docs/spec contract (`name` and `description`) over Kilo's currently looser loader. A Kilo skill with no `description` may be accepted by Kilo source, but it is not a good cross-provider link candidate because Kilo's own prompt formatter omits undescribed skills and other providers may reject them.

## Claudine Linking Notes

Claudine should model Kilo as first-class skill support with these link targets:

- Primary Kilo targets: `.kilo/skills/<name>/SKILL.md` for repo scope and `~/.kilo/skills/<name>/SKILL.md` for user scope.
- Accepted Kilo-only aliases: `.kilo/skill/<name>/SKILL.md`, `.kilocode/skills/<name>/SKILL.md`, and `.kilocode/skill/<name>/SKILL.md`.
- Compatibility targets: `.agents/skills/<name>/SKILL.md` and `.claude/skills/<name>/SKILL.md` should be shared only when the user wants multi-provider compatibility.
- Do not link built-in `builtin:kilo-config`; treat it as provider-owned system content.
- Do not link URL cache entries from `$XDG_CACHE_HOME/kilo/skills` as author-owned resources; link the source repository or downloaded skill directory only when it is intentionally materialized by the user.
- Preserve bundled sibling files with relative paths. The `skill` tool explicitly tells the model that relative paths are relative to the skill base directory.
- Warn or rewrite Kilo-specific instructions that depend on Kilo permission keys, Kilo config paths, `skills.urls`, or Kilo-only CLI commands.

This research implies Claudine metadata/linking changes. Kilo should be added to provider skill support as `first_class`; the linker should know Kilo's canonical `.kilo/skills` target, legacy `.kilocode` acceptance, singular `skill` acceptance, compatibility-directory behavior, and non-portable URL/built-in cases. Duplicate handling should be represented as last-one-wins by frontmatter name rather than directory name.

## Sources

- [Kilo Code Skills documentation](https://kilo.ai/docs/customize/skills)
- [Kilo Code CLI documentation](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo Code source: `packages/opencode/src/skill/index.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/skill/index.ts)
- [Kilo Code source: `packages/opencode/src/tool/skill.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/tool/skill.ts)
- [Kilo Code source: `packages/opencode/src/skill/discovery.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/skill/discovery.ts)
- [Kilo Code source: `packages/opencode/src/kilocode/paths.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/kilocode/paths.ts)
- [Kilo Code source: `packages/opencode/src/effect/runtime-flags.ts`](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/effect/runtime-flags.ts)
- [Kilo Code source: built-in skills registry](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/src/kilocode/skills/builtin.ts)
- [Kilo Code source: VS Code marketplace paths](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/kilo-vscode/src/services/marketplace/paths.ts)
- [Kilo Code source: VS Code marketplace installer](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/kilo-vscode/src/services/marketplace/installer.ts)
- [Kilo Code source: skill tests](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/test/skill/skill.test.ts)
- [Kilo Code source: Kilo path tests](https://github.com/Kilo-Org/kilocode/blob/4c07a1db51d121b60129c3858f035da3f12df39c/packages/opencode/test/kilocode/paths.test.ts)
