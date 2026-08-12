---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
homepage: https://kilo.ai/
docs: https://kilo.ai/docs/code-with-ai/platforms/cli
skills_docs: https://kilo.ai/docs/customize/skills
support: first_class
locations:
  - os: macos
    scope: user
    path: "~/.kilo/skills"
    notes: "Canonical user skill directory documented by Kilo; VS Code marketplace global installs and the canonical `Skill.fmt` prompt listing resolve here."
  - os: linux
    scope: user
    path: "~/.kilo/skills"
    notes: "Canonical user skill directory documented by Kilo; VS Code marketplace global installs target this directory."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kilo\\skills"
    notes: "Canonical user skill directory documented by Kilo (Windows users typically use %USERPROFILE%\\.kilo\\skills; the kilo-cli `KilocodePaths.globalDirs()` helper resolves `$HOME`/`$USERPROFILE`/`os.homedir()`)."
  - os: macos
    scope: user
    path: "~/.kilocode/skills"
    notes: "Legacy user directory accepted by `KilocodePaths.globalDirs()` before `~/.kilo`; `ConfigPaths.directories()` scans `~/.kilocode` before `~/.kilo`, but the registry is last-one-wins by frontmatter name so `~/.kilo/<name>` wins on duplicate names."
  - os: linux
    scope: user
    path: "~/.kilocode/skills"
    notes: "Legacy user directory accepted by `KilocodePaths.globalDirs()` before `~/.kilo`; same last-one-wins precedence as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kilocode\\skills"
    notes: "Legacy user directory accepted by `KilocodePaths.globalDirs()` before `~/.kilo`; same last-one-wins precedence as macOS/Linux."
  - os: macos
    scope: user
    path: "~/.agents/skills"
    notes: "Open Agent Skills compatibility directory. Loaded by default unless `KILO_DISABLE_EXTERNAL_SKILLS` is set; matches pattern `skills/**/SKILL.md`."
  - os: linux
    scope: user
    path: "~/.agents/skills"
    notes: "Open Agent Skills compatibility directory. Loaded by default unless `KILO_DISABLE_EXTERNAL_SKILLS` is set."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills"
    notes: "Open Agent Skills compatibility directory. Loaded by default unless `KILO_DISABLE_EXTERNAL_SKILLS` is set."
  - os: macos
    scope: user
    path: "~/.claude/skills"
    notes: "Claude Code compatibility directory. Loaded only when `KILO_DISABLE_CLAUDE_CODE` and `KILO_DISABLE_CLAUDE_CODE_SKILLS` are both unset."
  - os: linux
    scope: user
    path: "~/.claude/skills"
    notes: "Claude Code compatibility directory. Loaded only when `KILO_DISABLE_CLAUDE_CODE` and `KILO_DISABLE_CLAUDE_CODE_SKILLS` are both unset."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills"
    notes: "Claude Code compatibility directory. Loaded only when both flags are unset."
  - os: macos
    scope: user
    path: "~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code/skills"
    notes: "VS Code extension global storage. Included when the directory exists; this is where the kilo-vscode marketplace installer writes globally-installed skills."
  - os: linux
    scope: user
    path: "~/.config/Code/User/globalStorage/kilocode.kilo-code/skills"
    notes: "VS Code extension global storage on Linux; included when the directory exists."
  - os: windows
    scope: user
    path: "%APPDATA%\\Code\\User\\globalStorage\\kilocode.kilo-code\\skills"
    notes: "VS Code extension global storage on Windows; included when the directory exists."
  - os: macos
    scope: user
    path: "$XDG_CONFIG_HOME/kilo/{skill,skills}"
    notes: "Global XDG config root as `Global.Path.config`. Glob is `{skill,skills}/**/SKILL.md`; defaults to `~/.config/kilo/{skill,skills}` when `XDG_CONFIG_HOME` is unset."
  - os: linux
    scope: user
    path: "$XDG_CONFIG_HOME/kilo/{skill,skills}"
    notes: "Global XDG config root; glob is `{skill,skills}/**/SKILL.md`; defaults to `~/.config/kilo/{skill,skills}` when `XDG_CONFIG_HOME` is unset."
  - os: windows
    scope: user
    path: "%LOCALAPPDATA%\\kilo\\{skill,skills}"
    notes: "Windows equivalent via xdg-basedir mapping; the kilo-cli CLI itself reads `Global.Path.config` from `@opencode-ai/core/global` and applies xdg-basedir fallbacks."
  - os: macos
    scope: repo
    path: ".kilo/{skill,skills}"
    notes: "Project config directory scanned from the launch directory up to the git worktree root. Both singular `skill/` and plural `skills/` are accepted by the glob. Project entries load after `Global.Path.config` but before the home `.kilocode`/`.kilo`, so they override global skill entries by name."
  - os: linux
    scope: repo
    path: ".kilo/{skill,skills}"
    notes: "Same as macOS; glob is `{skill,skills}/**/SKILL.md`; project entries override home entries."
  - os: windows
    scope: repo
    path: ".kilo\\{skill,skills}"
    notes: "Same as macOS/Linux; glob is `{skill,skills}/**/SKILL.md`; project entries override home entries."
  - os: macos
    scope: repo
    path: ".kilocode/{skill,skills}"
    notes: "Legacy project config directory scanned before `.kilo` at each level. The registry is last-one-wins by frontmatter name, so `.kilo/<name>` still wins on duplicate names."
  - os: linux
    scope: repo
    path: ".kilocode/{skill,skills}"
    notes: "Legacy project config directory scanned before `.kilo` at each level; last-one-wins precedence."
  - os: windows
    scope: repo
    path: ".kilocode\\{skill,skills}"
    notes: "Legacy project config directory scanned before `.kilo` at each level; last-one-wins precedence."
  - os: macos
    scope: repo
    path: ".agents/skills"
    notes: "Compatibility directory found by walking from the launch directory to the worktree root. Loaded when `KILO_DISABLE_EXTERNAL_SKILLS` is unset."
  - os: linux
    scope: repo
    path: ".agents/skills"
    notes: "Compatibility directory found by walking from the launch directory to the worktree root. Loaded when `KILO_DISABLE_EXTERNAL_SKILLS` is unset."
  - os: windows
    scope: repo
    path: ".agents\\skills"
    notes: "Compatibility directory found by walking from the launch directory to the worktree root."
  - os: macos
    scope: repo
    path: ".claude/skills"
    notes: "Claude Code compatibility directory found by walking from the launch directory to the worktree root. Loaded only when both `KILO_DISABLE_CLAUDE_CODE` and `KILO_DISABLE_CLAUDE_CODE_SKILLS` are unset."
  - os: linux
    scope: repo
    path: ".claude/skills"
    notes: "Claude Code compatibility directory found by walking from the launch directory to the worktree root. Loaded only when both Claude Code flags are unset."
  - os: windows
    scope: repo
    path: ".claude\\skills"
    notes: "Claude Code compatibility directory found by walking from the launch directory to the worktree root."
  - os: macos
    scope: repo
    path: ".kilo/skills/<id> (marketplace installer)"
    notes: "VS Code extension marketplace installer writes project-scope skills here as tarballs; the discovery loop picks them up on the next session. This is the same `~/.kilo/skills` location as user canonical."
  - os: linux
    scope: repo
    path: ".kilo/skills/<id> (marketplace installer)"
    notes: "VS Code extension marketplace installer writes project-scope skills here; same path as user canonical on Linux."
  - os: windows
    scope: repo
    path: ".kilo\\skills\\<id> (marketplace installer)"
    notes: "VS Code extension marketplace installer writes project-scope skills here; same path as user canonical on Windows."
  - os: macos
    scope: other
    path: "kilo.jsonc skills.paths entries"
    notes: "Each configured path is scanned for `**/SKILL.md` (no `skill{,s}/` prefix). Absolute paths, `~/` home-relative paths, and project-root-relative paths are accepted."
  - os: linux
    scope: other
    path: "kilo.jsonc skills.paths entries"
    notes: "Each configured path is scanned for `**/SKILL.md`. Absolute paths, `~/` home-relative paths, and project-root-relative paths are accepted."
  - os: windows
    scope: other
    path: "kilo.jsonc skills.paths entries"
    notes: "Each configured path is scanned for `**/SKILL.md`. Absolute paths, `~/` home-relative paths, and project-root-relative paths are accepted."
  - os: macos
    scope: other
    path: "$XDG_CACHE_HOME/kilo/skills"
    notes: "Cache root for skills downloaded from `skills.urls` via the `Discovery.pull` helper; defaults to `~/.cache/kilo/skills`. Each URL is fetched once per URL and cached under `$XDG_CACHE_HOME/kilo/skills/<skill-name>/`."
  - os: linux
    scope: other
    path: "$XDG_CACHE_HOME/kilo/skills"
    notes: "Cache root for skills downloaded from `skills.urls`; defaults to `~/.cache/kilo/skills`."
  - os: windows
    scope: other
    path: "%LOCALAPPDATA%\\kilo\\Cache\\skills"
    notes: "Windows equivalent via xdg-basedir mapping; URL-backed skills are cached here."
  - os: macos
    scope: system
    path: "builtin:kilo-config"
    notes: "Built-in skill bundled inside the CLI binary (`packages/opencode/src/kilocode/skills/builtin.ts`). Seeded before discovery, so any user, project, configured, or URL skill named `kilo-config` overrides it."
  - os: linux
    scope: system
    path: "builtin:kilo-config"
    notes: "Built-in skill bundled inside the CLI binary. Seeded before discovery; user/project/URL skills named `kilo-config` override it."
  - os: windows
    scope: system
    path: "builtin:kilo-config"
    notes: "Built-in skill bundled inside the CLI binary. Seeded before discovery; user/project/URL skills named `kilo-config` override it."
format:
  file_names: ["SKILL.md"]
  frontmatter: true
  required_fields: ["name"]
  optional_fields: ["description", "license", "compatibility", "metadata"]
  body_format: markdown
  notes: "A skill is a directory containing `SKILL.md`. YAML frontmatter is required and parsed via `ConfigMarkdown.parse`. The current `isSkillFrontmatter` guard enforces `name` as a string and accepts `description` as an optional string; the official docs/spec require both `name` and `description` but the loader is permissive. The `Skill.fmt` prompt formatter filters out skills whose `description` is undefined, so `description`-less skills load into the registry but are not visible to the model. The docs additionally claim `name` must match the parent directory name, but `state.skills[md.data.name] = ...` keys by the frontmatter `name` without checking the directory name; the `SkillNameMismatchError` class exists but is not raised by the current `add()` function. Bundled sibling files and folders (`scripts/`, `references/`, `assets/`, templates, examples) are surfaced to the agent as files relative to the skill base directory."
discovery:
  mechanism: "On session initialization, Kilo scans compatibility directories (`.claude/skills/` and `.agents/skills/`) at user and project scope, walks each Kilo config directory (XDG config root, project `.kilocode`/`.kilo`, home `.kilocode`/`.kilo`, `KILO_CONFIG_DIR`) with the glob `{skill,skills}/**/SKILL.md`, expands and scans each `skills.paths` entry with the glob `**/SKILL.md`, and pulls configured `skills.urls` into the cache and scans them with the same glob. Skill metadata is read into the registry keyed by frontmatter `name`, then the `skill` tool loads the full body on demand."
  precedence: "Last-one-wins by frontmatter `name`. `BUILTIN_SKILLS` (`kilo-config`) is seeded first, then external compatibility dirs are scanned (`.claude/skills` when Claude Code skills are enabled, then `.agents/skills`), then Kilo config directories in this order: `Global.Path.config` → primary-worktree mirror (`.kilocode`/`.kilo` from the primary checkout) → project `.kilocode`/`.kilo` from launch directory to worktree root → home `.kilocode`/`.kilo` → `KILO_CONFIG_DIR`. Configured `skills.paths` are scanned next, then URL-cached `skills.urls` last, so URL skills can override both global and project skills by name. Legacy `.kilocode` directories are scanned before `.kilo` at the same level, but the registry's last-one-wins resolution makes `.kilo/<name>` win on duplicates."
  enable_disable: "Disable a skill by removing or renaming its `SKILL.md`, deleting the source directory, removing the source path or URL from `skills.paths`/`skills.urls`, or denying the `skill` tool (or a specific skill name) in Kilo permission policy. `KILO_DISABLE_EXTERNAL_SKILLS=1` disables both `.claude/skills` and `.agents/skills` discovery. `KILO_DISABLE_CLAUDE_CODE=1` and `KILO_DISABLE_CLAUDE_CODE_SKILLS=1` disable only the `.claude/skills` compatibility directory. `KILO_DISABLE_PROJECT_CONFIG=1` skips project `.kilocode`/`.kilo` config directories and therefore any project-level skills in those directories; it does not affect compatibility directory discovery. The VS Code extension exposes a discovered-skills list with a remove action for non-built-in filesystem skills; the marketplace installer refuses built-in and URL-backed cached skills. Kilo permission rules can deny the `skill` tool globally (`permission.skill`) or per-skill (`permission.skill.<name>`)."
  notes: "Skills are re-scanned for each new session. Live file-system watching is enabled via `KILO_EXPERIMENTAL_FILEWATCHER` but discovery is still trigger-based on session start. The `Skill.available` accessor filters by `Permission.evaluate(\"skill\", name, agent.permission)` so denied skills are hidden from the per-agent prompt listing. There is no documented skill-specific workspace trust prompt; repo-scope Kilo config directories can be skipped with `KILO_DISABLE_PROJECT_CONFIG`, but external compatibility directory discovery is controlled by `KILO_DISABLE_EXTERNAL_SKILLS` / `KILO_DISABLE_CLAUDE_CODE_SKILLS` rather than by that project-config flag. `KILO_PURE` (and `--pure`) only disables external plugins, not skill discovery. No evidence was found that extension/plugin packages contribute skills directly outside of the VS Code marketplace installer writing into the normal skill directories."
portability:
  portable: false
  non_portable_assets: ["Kilo-specific `builtin:kilo-config` skill (bundled Markdown content for Kilo config questions)", "Kilo permission examples referencing the `skill` tool or skill-name permission keys", "`kilo.jsonc` `skills.paths` and `skills.urls` configuration", "Remote `index.json` manifest format (Kilo's URL cache layout under `$XDG_CACHE_HOME/kilo/skills`)", "VS Code marketplace staging/install metadata and tarball workflow", "Provider-specific instructions that assume Kilo-only tools, commands, or config surfaces"]
  rewrite_needed: true
  notes: "Plain Agent Skills directories containing `SKILL.md` plus relative bundled assets are portable to other Agent Skills implementations when the frontmatter `name` is unique and the body does not assume Kilo-only tools. Compatibility directories such as `.agents/skills` and `.claude/skills` can be linked as-is only when the target provider accepts the same Agent Skills structure. The docs require `name` and `description`, but Kilo's loader does not enforce `description`; Claudine should treat `description`-less skills as effectively invisible to Kilo's prompt formatter (and other providers may also require it). The frontmatter `name` does not need to match the parent directory name at runtime despite the docs saying so, so Claudine should preserve each skill's authored name when linking."
cli_params:
  - flag: "--pure"
    description: "Sets `KILO_PURE=1` and disables external plugins; source flags this as plugin-only with no direct skill-discovery effect."
    example: "kilo --pure"
  - flag: "--agent"
    description: "Selects the active agent; `Skill.available` filters the prompt listing by the agent's permission policy, so a denied skill hides only when this agent is active."
    example: "kilo run --agent code \"use api-design\""
  - flag: "--dangerously-skip-permissions"
    description: "Auto-approves permissions that are not explicitly denied; affects the `skill` tool runtime prompt, not discovery."
    example: "kilo run --dangerously-skip-permissions \"use my-skill\""
  - flag: "--auto"
    description: "Auto-approves all permissions for autonomous runs; affects the `skill` tool runtime prompt, not discovery."
    example: "kilo run --auto \"use my-skill\""
env_vars:
  - name: "KILO_DISABLE_EXTERNAL_SKILLS"
    effect: "When true, skips `.agents/skills` and `.claude/skills` compatibility directories at both user and project scope."
  - name: "KILO_DISABLE_CLAUDE_CODE"
    effect: "Broad Claude Code compatibility disable; also disables `.claude/skills` discovery."
  - name: "KILO_DISABLE_CLAUDE_CODE_SKILLS"
    effect: "Disables `.claude/skills` discovery without disabling `.agents/skills`."
  - name: "KILO_CONFIG"
    effect: "Loads an additional `kilo.json`/`kilo.jsonc` file; its `skills.paths` or `skills.urls` can add skill sources."
  - name: "KILO_CONFIG_DIR"
    effect: "Adds an explicit config directory to the search list; `{skill,skills}/**/SKILL.md` under that directory are scanned."
  - name: "KILO_CONFIG_CONTENT"
    effect: "Inline JSON config with high precedence; can define `skills.paths` and `skills.urls`."
  - name: "KILO_DISABLE_PROJECT_CONFIG"
    effect: "Skips project `kilo.json`/`kilo.jsonc` and project `.kilo`/`.kilocode` config directories, which also skips repo skills in those Kilo config directories; compatibility directory discovery has separate external-skill flags."
  - name: "KILO_PURE"
    effect: "Disables external plugins; no direct skill-discovery effect."
  - name: "KILO_CLIENT"
    effect: "Selects client mode (`cli`, `vscode`, `jetbrains`, `acp`); no direct discovery effect, but client mode changes available tools and UI surfaces."
  - name: "HOME"
    effect: "Primary home root used for `~/.kilo`, `~/.kilocode`, `~/.agents`, and `~/.claude` skill directories. `KilocodePaths.globalDirs()` resolves `HOME || USERPROFILE || os.homedir()`."
  - name: "USERPROFILE"
    effect: "Fallback home root in `KilocodePaths.globalDirs()` when `HOME` is unavailable."
  - name: "XDG_CONFIG_HOME"
    effect: "Changes the global config root used for `$XDG_CONFIG_HOME/kilo/{skill,skills}` and the global `kilo.jsonc` skills configuration."
  - name: "XDG_CACHE_HOME"
    effect: "Changes the cache root used for URL-downloaded skills under `$XDG_CACHE_HOME/kilo/skills`."
  - name: "APPDATA"
    effect: "Changes the Windows VS Code globalStorage base used by `KilocodePaths.vscodeGlobalStorage()` for marketplace/global-storage skills."
changes:
  - "Verified discovery order against current `packages/opencode/src/skill/index.ts`, `skill/discovery.ts`, `kilocode/paths.ts`, `kilocode/primary-worktree.ts`, and `kilocode/skills/builtin.ts` on `main`."
  - "Reclassified `required_fields` to `[name]` to match the actual `isSkillFrontmatter` loader guard; documented that the spec/docs say `name` + `description` and that the prompt formatter filters out `description`-less skills."
  - "Documented the `{skill,skills}/**/SKILL.md` glob for Kilo config dirs, `skills/**/SKILL.md` for compatibility dirs, and `**/SKILL.md` for `skills.paths` and URL cache dirs."
  - "Noted that the docs' `name`-must-match-directory rule is not enforced by the loader (`state.skills[md.data.name] = ...`); `SkillNameMismatchError` is declared but not currently raised."
  - "Added the `KilocodePaths.globalDirs()` order (`~/.kilocode` before `~/.kilo`) and the `primaryPaths()` linked-worktree mirror that contributes extra `.kilocode`/`.kilo` paths from the primary checkout."
  - "Added the kilo-vscode marketplace installer target (`.kilo/skills/<id>` project and `~/.kilo/skills/<id>` global) to the locations list."
  - "Observed locally that `~/.agents/skills/find-skills/SKILL.md` is present and was installed via the Vercel `vercel-labs/skills` GitHub source, confirming the `.agents` compatibility directory is actively loaded by the installed `kilo 7.3.45` CLI."
  - "Refreshed the metadata block: `agent: open_code`, `model: minimax/MiniMax-M3`, `last_updated: 2026-07-03`."
requires_claudine_update: true
reason: "Claudine's Kilo linker should include Kilo as first-class skill support, add `.kilo/{skill,skills}/` and `.kilocode/{skill,skills}/` directories plus the `.agents/skills` and `.claude/skills` compatibility directories, classify URL-backed and built-in skills as non-portable, model Kilo's last-one-wins duplicate handling by frontmatter `name`, and recognize that Kilo's prompt formatter filters out skills without `description` even when the loader accepts them."
---

# Kilo Code Agent Skills

## Overview

Kilo Code implements Agent Skills as durable directories containing a `SKILL.md` file with YAML frontmatter and Markdown instructions, in line with the open [Agent Skills specification](https://agentskills.io/specification). The official documentation describes the workflow as metadata discovery at session start, prompt inclusion of relevant skill names and descriptions, and on-demand loading when the model calls the `skill` tool. The implementation matches that shape: `Skill.discovery` scans multiple location categories, `Skill.state` seeds built-ins and loads metadata into a registry keyed by frontmatter `name`, `Skill.fmt` injects an `<available_skills>` block (or a Markdown bullet list in the non-verbose form), and `tool/skill.ts` loads the selected skill body, reports the base directory as a `file://` URL, and samples up to 10 sibling files via ripgrep.

The implementation is first-class and provider-specific in these ways:

- It has a dedicated `skill` tool whose runtime permission key is `"skill"` with `patterns: [name]`.
- It has documented user, project, compatibility, configured-path, and remote-URL sources.
- It ships at least one built-in system skill, `kilo-config`, in `packages/opencode/src/kilocode/skills/builtin.ts`.
- It applies Kilo permission policy to the `skill` tool and to individual skill names via `Permission.evaluate("skill", name, agent.permission)`.

The local host inspection found no `~/.kilo` directory and no local `~/.kilo/skills` resources, but `~/.agents/skills/find-skills/SKILL.md` is present and is installed from the Vercel `vercel-labs/skills` GitHub source (confirmed via `~/.agents/.skill-lock.json`), demonstrating that the `.agents` compatibility directory is actively loaded by the installed `kilo 7.3.45` CLI. The local `kilo` and `kilocode` CLIs (both version `7.3.45`, installed at `@kilocode/cli`) showed no skill-specific CLI flags; the documented runtime flags `--agent`, `--auto`, `--dangerously-skip-permissions`, and `--pure` affect agent selection, permission prompting, or plugin loading rather than skill discovery paths.

## Locations

Kilo has more skill locations than the short public examples imply. The canonical user directory is `~/.kilo/skills`, and the canonical project directory is `.kilo/skills`. Source also accepts singular `skill/` under Kilo config directories, legacy `.kilocode`, compatibility `.agents` and `.claude`, VS Code extension global storage, extra configured paths, remote URL cache directories, and built-ins.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| User canonical | `~/.kilo/skills` | `~/.kilo/skills` | `%USERPROFILE%\.kilo\skills` | Documented global skill directory and VS Code marketplace global install target. |
| User legacy | `~/.kilocode/skills` | `~/.kilocode/skills` | `%USERPROFILE%\.kilocode\skills` | `KilocodePaths.globalDirs()` returns `~/.kilocode` before `~/.kilo`; registry is last-one-wins by name, so `~/.kilo/<name>` wins on duplicate names. |
| User compatibility | `~/.agents/skills` | `~/.agents/skills` | `%USERPROFILE%\.agents\skills` | Open Agent Skills standard; loaded by default unless `KILO_DISABLE_EXTERNAL_SKILLS` is set. |
| User Claude compatibility | `~/.claude/skills` | `~/.claude/skills` | `%USERPROFILE%\.claude\skills` | Claude Code compatibility; loaded unless both Claude Code flags are set. |
| VS Code global storage | `~/Library/Application Support/Code/User/globalStorage/kilocode.kilo-code/skills` | `~/.config/Code/User/globalStorage/kilocode.kilo-code/skills` | `%APPDATA%\Code\User\globalStorage\kilocode.kilo-code\skills` | Included when the directory exists; marketplace installer writes globally-installed skills here. |
| Global config root | `$XDG_CONFIG_HOME/kilo/{skill,skills}` | `$XDG_CONFIG_HOME/kilo/{skill,skills}` | xdg-basedir Windows config root plus `kilo\{skill,skills}` | `Global.Path.config` from `@opencode-ai/core/global`; defaults to `~/.config/kilo/{skill,skills}` on macOS and Linux when `XDG_CONFIG_HOME` is unset. |
| Project canonical | `.kilo/{skill,skills}` | `.kilo/{skill,skills}` | `.kilo\{skill,skills}` | Scanned from launch directory up to git worktree root, plus a primary-worktree mirror via `primaryPaths()`. |
| Project legacy | `.kilocode/{skill,skills}` | `.kilocode/{skill,skills}` | `.kilocode\{skill,skills}` | Scanned before `.kilo` at each level; registry last-one-wins by name still applies. |
| Project compatibility | `.agents/skills` | `.agents/skills` | `.agents\skills` | Found by walking up to the worktree root when external skills are enabled. |
| Project Claude compatibility | `.claude/skills` | `.claude/skills` | `.claude\skills` | Found by walking up to the worktree root when both Claude Code flags are unset. |
| Configured paths | `skills.paths` entries | `skills.paths` entries | `skills.paths` entries | Absolute paths, `~/` home-relative paths, and project-root-relative paths are accepted and scanned with `**/SKILL.md`. |
| Remote URL cache | `$XDG_CACHE_HOME/kilo/skills` | `$XDG_CACHE_HOME/kilo/skills` | xdg-basedir Windows cache root plus `kilo\skills` | `skills.urls` downloads each skill directory under `$XDG_CACHE_HOME/kilo/skills/<name>/`. |
| Built-in | `builtin:kilo-config` | `builtin:kilo-config` | `builtin:kilo-config` | Bundled in the CLI binary; seeded before discovery, so any discovered skill named `kilo-config` overrides it. |

Remote URLs are configured in `kilo.jsonc` under `skills.urls`:

```jsonc
{
  "skills": {
    "paths": ["/path/to/shared/skills", "~/my-skills", "relative/skills"],
    "urls": ["https://example.com/.well-known/skills/"]
  }
}
```

A remote URL must serve `index.json` at the URL root. Kilo fetches `{url}/index.json`, requires each skill entry to include `SKILL.md` in its `files` array, downloads each listed file from `{url}/{skill-name}/{file}` to `$XDG_CACHE_HOME/kilo/skills/<skill-name>/`, and scans the cached skill directory with `**/SKILL.md`.

```json
{
  "skills": [
    { "name": "skill-name", "files": ["SKILL.md", "references/file.md"] }
  ]
}
```

The kilo-vscode marketplace installer is a separate path into the same filesystem layout. The installer downloads a tarball from the marketplace into `~/.kilo/skills/<id>` (global) or `<workspace>/.kilo/skills/<id>` (project), stages and renames it inside the target directory, and then the discovery loop picks it up on the next session. The `removeSkill` action only refuses built-in and URL-backed cached skills; ordinary filesystem skills are deleted recursively.

## File Format

The accepted file name is exactly `SKILL.md`. Under Kilo config directories, the glob is `{skill,skills}/**/SKILL.md`; under compatibility directories the glob is `skills/**/SKILL.md`; under configured paths and URL cache directories the glob is `**/SKILL.md`.

The public Kilo docs follow the Agent Skills specification and list these frontmatter fields:

| Field | Required by docs/spec | Accepted by implementation | Notes |
|---|---:|---:|---|
| `name` | Yes | Yes | Registry key. The implementation keys the registry by `md.data.name` and does not check the parent directory name despite the docs' claim. The `SkillNameMismatchError` class exists but is not currently raised. |
| `description` | Yes | Optional | Loader accepts a missing description; `Skill.fmt` filters out `description`-less skills from the prompt, so they are effectively invisible to the model. |
| `license` | No | Ignored by the current loader | Portable metadata for other Agent Skills implementations. |
| `compatibility` | No | Ignored by the current loader | Portable metadata for environment requirements; not enforced in the inspected loader. |
| `metadata` | No | Ignored by the current loader | Arbitrary mapping for other consumers. |

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

The loader stores `content` as the Markdown body after frontmatter, not the full file. When the `skill` tool runs, it emits the content inside `<skill_content name="...">`. For filesystem-backed skills it also includes the skill base directory as a `file://` URL, tells the model that relative paths are relative to that base directory, and samples up to 10 sibling files found with ripgrep while excluding `SKILL.md`. This means bundled `scripts/`, `references/`, `assets/`, templates, examples, and media can be used, but the model must read or execute them through normal tools after the skill is loaded. Built-in skills (`BUILTIN_LOCATION = "builtin"`) take a special branch in the tool that omits the base-directory section because they have no filesystem location.

## Discovery and Precedence

Discovery happens at session initialization. In the CLI, Kilo rescans skills when a new TUI session starts or when `kilo run` starts a new run. In the VS Code extension, skills are loaded when the extension connects to the CLI server. Existing sessions do not automatically pick up newly edited skill files.

The source discovery order is:

1. External compatibility directories, unless `KILO_DISABLE_EXTERNAL_SKILLS` is set:
   `.claude/skills` if Claude Code skills are enabled, then `.agents/skills`, first under the home directory and then from the launch directory up to the worktree root.
2. Kilo config directories from `ConfigPaths.directories()` (after the `primaryPaths()` mirror of the linked-worktree primary checkout):
   `Global.Path.config` (XDG), project `.kilocode` and `.kilo` from launch directory to worktree root, home `.kilocode` and `.kilo`, then `KILO_CONFIG_DIR` when set.
3. `skills.paths` from merged Kilo config, expanded as absolute, home-relative (`~/…`), or project-root-relative paths.
4. `skills.urls` from merged Kilo config, downloaded into the cache and scanned.

The registry is keyed by frontmatter `name`, and duplicates are last-one-wins. Source logs a duplicate warning and overwrites the earlier entry. The built-in `kilo-config` skill is seeded before all discovery so any filesystem or URL skill named `kilo-config` overrides it. The docs state project skills take precedence over global skills; the implementation achieves this for Kilo config directories because project `.kilocode`/`.kilo` directories load after `Global.Path.config` but before home `.kilocode`/`.kilo`. Configured `skills.paths` and URL-backed skills load even later, so they can override both global and project skills by name.

Kilo does not use separate mode-specific skill directories on the new platform. All discovered skills enter one pool and the agent decides whether a description clearly applies to the current task. Explicit user invocation by name works because the skill names are visible in the prompt.

Skills can be enabled by creating a valid `SKILL.md` in a scanned directory, adding a path to `skills.paths`, or adding a URL to `skills.urls`. They can be disabled by removing or renaming `SKILL.md`, removing the source path or URL from config, or denying the `skill` tool or a specific skill name in Kilo permission policy. The VS Code extension exposes a discovered-skills list and a remove action for non-built-in filesystem skills; the marketplace installer removal path refuses built-ins and refuses cached URL-backed skills.

No skill-specific workspace trust prompt was found. Repo-scope Kilo config directories can be skipped with `KILO_DISABLE_PROJECT_CONFIG`, but external compatibility directory discovery is controlled by `KILO_DISABLE_EXTERNAL_SKILLS` and `KILO_DISABLE_CLAUDE_CODE_SKILLS` rather than by that project-config flag in the inspected source. Kilo permissions still gate runtime use of the `skill` tool through the `permission.skill.<name>` rule and the global `permission.skill` rule.

## Portability

Kilo uses the open Agent Skills shape, so the core artifact is partially portable:

- Portable as-is: a directory with `SKILL.md`, YAML `name` and `description`, Markdown instructions, and relative bundled files that do not assume Kilo-only tools.
- Needs path placement only: skills in `.agents/skills` or `.claude/skills` can be linked to providers that scan those same paths.
- Needs rewrite: `kilo.jsonc` `skills.paths` and `skills.urls`, remote `index.json` registries, Kilo permission examples, and any instructions that say to use Kilo-specific commands, tools, config files, or marketplace flows.
- Non-portable: the built-in `kilo-config` skill, VS Code marketplace staging/install metadata, URL cache directories under `$XDG_CACHE_HOME/kilo/skills`, and provider-specific runtime permission state.

The frontmatter itself is mostly portable, but Claudine should prefer the stricter docs/spec contract (`name` and `description`) over Kilo's currently looser loader. A Kilo skill with no `description` is accepted by Kilo source and loaded into the registry, but Kilo's own prompt formatter omits it and other providers may reject it.

## Claudine Linking Notes

Claudine should model Kilo as first-class skill support with these link targets:

- Primary Kilo targets: `.kilo/skills/<name>/SKILL.md` for repo scope and `~/.kilo/skills/<name>/SKILL.md` for user scope.
- Accepted Kilo-only aliases: `.kilo/skill/<name>/SKILL.md`, `.kilocode/skills/<name>/SKILL.md`, and `.kilocode/skill/<name>/SKILL.md`.
- Compatibility targets: `.agents/skills/<name>/SKILL.md` and `.claude/skills/<name>/SKILL.md` should be shared only when the user wants multi-provider compatibility.
- Do not link built-in `builtin:kilo-config`; treat it as provider-owned system content.
- Do not link URL cache entries from `$XDG_CACHE_HOME/kilo/skills` as author-owned resources; link the source repository or downloaded skill directory only when it is intentionally materialized by the user.
- Preserve bundled sibling files with relative paths. The `skill` tool explicitly tells the model that relative paths are relative to the skill base directory.
- Warn or rewrite Kilo-specific instructions that depend on Kilo permission keys, Kilo config paths, `skills.urls`, or Kilo-only CLI commands.
- Treat the registry's last-one-wins-by-name duplicate rule as the precedence model when a skill exists under multiple Kilo scopes.

This research implies Claudine metadata/linking changes. Kilo should be added to provider skill support as `first_class`; the linker should know Kilo's canonical `.kilo/skills` target, legacy `.kilocode` acceptance, singular `skill` acceptance, compatibility-directory behavior, and non-portable URL/built-in cases. Duplicate handling should be represented as last-one-wins by frontmatter `name` rather than directory name, and the linker should warn when a candidate skill lacks a `description` so the user can choose whether to surface it.

## Changelog

- **2026-07-03** — Verified against current `main` source: `packages/opencode/src/skill/index.ts`, `skill/discovery.ts`, `kilocode/paths.ts`, `kilocode/primary-worktree.ts`, `kilocode/skills/builtin.ts`, `config/paths.ts`, `tool/skill.ts`, and `core/src/flag/flag.ts`. Updated the metadata block (`agent: open_code`, `model: minimax/MiniMax-M3`, `last_updated: 2026-07-03`). Reclassified `required_fields` to `[name]` to match the actual `isSkillFrontmatter` loader guard and clarified that the docs' `description` requirement is enforced by the prompt formatter, not the loader. Documented the `{skill,skills}/**/SKILL.md` glob for Kilo config dirs, `skills/**/SKILL.md` for compatibility dirs, and `**/SKILL.md` for `skills.paths` and URL cache dirs. Noted that the docs' `name`-must-match-directory rule is not enforced by the loader; `SkillNameMismatchError` is declared but not currently raised. Added the `KilocodePaths.globalDirs()` order (`~/.kilocode` before `~/.kilo`) and the `primaryPaths()` linked-worktree mirror. Added the kilo-vscode marketplace installer target to the locations list. Observed locally that `~/.agents/skills/find-skills/SKILL.md` is loaded by the installed `kilo 7.3.45` CLI from the Vercel `vercel-labs/skills` GitHub source. Refreshed sources to include the kilo-vscode marketplace installer and the Agent Skills specification.

## Sources

- [Kilo Code Skills documentation](https://kilo.ai/docs/customize/skills)
- [Kilo Code CLI documentation](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Agent Skills specification](https://agentskills.io/specification)
- [Kilo Code source: `packages/opencode/src/skill/index.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/skill/index.ts)
- [Kilo Code source: `packages/opencode/src/skill/discovery.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/skill/discovery.ts)
- [Kilo Code source: `packages/opencode/src/tool/skill.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/tool/skill.ts)
- [Kilo Code source: `packages/opencode/src/tool/skill.txt`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/tool/skill.txt)
- [Kilo Code source: `packages/opencode/src/kilocode/paths.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/paths.ts)
- [Kilo Code source: `packages/opencode/src/kilocode/primary-worktree.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/primary-worktree.ts)
- [Kilo Code source: `packages/opencode/src/kilocode/skills/builtin.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/skills/builtin.ts)
- [Kilo Code source: `packages/opencode/src/config/paths.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/paths.ts)
- [Kilo Code source: `packages/opencode/src/effect/runtime-flags.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/effect/runtime-flags.ts)
- [Kilo Code source: `packages/core/src/flag/flag.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/flag/flag.ts)
- [Kilo Code source: skill tests](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/test/skill/skill.test.ts)
- [Kilo Code source: kilo-vscode marketplace installer](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-vscode/src/services/marketplace/installer.ts)
- [Kilo Code source: kilo-vscode marketplace paths](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-vscode/src/services/marketplace/paths.ts)
- [Kilo Marketplace repository](https://github.com/Kilo-Org/kilo-marketplace)
- [Kilo Code homepage](https://kilo.ai/)