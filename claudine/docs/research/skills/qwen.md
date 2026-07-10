---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
docs: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/
skills_docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/

support: first_class

locations:
  - os: macos
    scope: user
    path: "~/.qwen/skills/<skill-name>/SKILL.md"
    notes: "Personal skills. The user config root can be redirected with QWEN_HOME; when set, this becomes $QWEN_HOME/skills/<skill-name>/SKILL.md."
  - os: linux
    scope: user
    path: "~/.qwen/skills/<skill-name>/SKILL.md"
    notes: "Personal skills. The user config root can be redirected with QWEN_HOME; when set, this becomes $QWEN_HOME/skills/<skill-name>/SKILL.md."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\skills\\<skill-name>\\SKILL.md"
    notes: "Personal skills. The user config root can be redirected with QWEN_HOME; when set, this becomes %QWEN_HOME%\\skills\\<skill-name>\\SKILL.md."
  - os: macos
    scope: user
    path: "~/.agents/skills/<skill-name>/SKILL.md"
    notes: "Source-confirmed interoperable alias scanned in addition to ~/.qwen/skills. QWEN_HOME does not redirect this alias."
  - os: linux
    scope: user
    path: "~/.agents/skills/<skill-name>/SKILL.md"
    notes: "Source-confirmed interoperable alias scanned in addition to ~/.qwen/skills. QWEN_HOME does not redirect this alias."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<skill-name>\\SKILL.md"
    notes: "Source-confirmed interoperable alias scanned in addition to %USERPROFILE%\\.qwen\\skills. QWEN_HOME does not redirect this alias."
  - os: macos
    scope: repo
    path: ".qwen/skills/<skill-name>/SKILL.md"
    notes: "Project skills under the resolved project root."
  - os: linux
    scope: repo
    path: ".qwen/skills/<skill-name>/SKILL.md"
    notes: "Project skills under the resolved project root."
  - os: windows
    scope: repo
    path: ".qwen\\skills\\<skill-name>\\SKILL.md"
    notes: "Project skills under the resolved project root."
  - os: macos
    scope: repo
    path: ".agents/skills/<skill-name>/SKILL.md"
    notes: "Source-confirmed project alias scanned in addition to .qwen/skills."
  - os: linux
    scope: repo
    path: ".agents/skills/<skill-name>/SKILL.md"
    notes: "Source-confirmed project alias scanned in addition to .qwen/skills."
  - os: windows
    scope: repo
    path: ".agents\\skills\\<skill-name>\\SKILL.md"
    notes: "Source-confirmed project alias scanned in addition to .qwen\\skills."
  - os: macos
    scope: extension
    path: "~/.qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md"
    notes: "User-scope extension skills. The extension manifest may override the skills directory with qwen-extension.json skills."
  - os: linux
    scope: extension
    path: "~/.qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md"
    notes: "User-scope extension skills. The extension manifest may override the skills directory with qwen-extension.json skills."
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.qwen\\extensions\\<extension-name>\\skills\\<skill-name>\\SKILL.md"
    notes: "User-scope extension skills. The extension manifest may override the skills directory with qwen-extension.json skills."
  - os: macos
    scope: extension
    path: ".qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md"
    notes: "Project-scope extension skills, active only when the extension is enabled for the workspace."
  - os: linux
    scope: extension
    path: ".qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md"
    notes: "Project-scope extension skills, active only when the extension is enabled for the workspace."
  - os: windows
    scope: extension
    path: ".qwen\\extensions\\<extension-name>\\skills\\<skill-name>\\SKILL.md"
    notes: "Project-scope extension skills, active only when the extension is enabled for the workspace."
  - os: macos
    scope: system
    path: "<qwen-code package>/bundled/<skill-name>/SKILL.md"
    notes: "Built-in bundled skills shipped with the installed qwen-code package. Local Homebrew 0.15.6 stores them under /opt/homebrew/Cellar/qwen-code/0.15.6/libexec/lib/node_modules/@qwen-code/qwen-code/bundled/."
  - os: linux
    scope: system
    path: "<qwen-code package>/bundled/<skill-name>/SKILL.md"
    notes: "Built-in bundled skills shipped with the installed qwen-code package; exact package path depends on npm, pnpm, distro, or manual installation."
  - os: windows
    scope: system
    path: "<qwen-code package>\\bundled\\<skill-name>\\SKILL.md"
    notes: "Built-in bundled skills shipped with the installed qwen-code package; exact package path depends on npm, pnpm, or manual installation."

format:
  file_names:
    - SKILL.md
  frontmatter: true
  required_fields:
    - name
    - description
  optional_fields:
    - allowedTools
    - hooks
    - model
    - argument-hint
    - when_to_use
    - disable-model-invocation
    - user-invocable
    - paths
    - priority
  body_format: markdown
  notes: "A skill is a directory containing SKILL.md plus optional adjacent files. Qwen normalizes BOM/CRLF before parsing, requires YAML frontmatter bounded by --- markers, trims the Markdown body, and allows sibling references such as reference.md, scripts/, templates/, examples/, references/, and assets/. Skill names must match /^[\\p{L}\\p{N}_:.-]+$/u. paths entries must be project-root-relative globs; absolute paths, drive letters, and .. segments are rejected."

discovery:
  mechanism: "SkillManager scans project, user, extension, and bundled tiers. It watches skill directories in normal mode, caches parsed metadata, injects active model-invocable skills into <available_skills> system reminders, and loads the full body through the Skill tool or direct /<skill-name> slash invocation. .qwen and .agents provider directories are both scanned for user and project skills."
  precedence: "Project > user > extension > bundled. Duplicate names are shadowed by the first higher-precedence tier. Within a tier, returned skills are sorted alphabetically for model-facing listings; priority affects the /skills listing display only."
  enable_disable: "Per-skill frontmatter can set user-invocable: false to hide direct slash invocation, or disable-model-invocation: true to hide the skill from the model. settings.json skills.disabled hides matching skill names case-insensitively from <available_skills>, /<name> slash commands, /skills listings, and completion; this array is union-merged across settings scopes. Removing or fixing the directory disables/enables a file-backed skill. Extensions must be enabled for their skills to participate."
  notes: "Safe mode loads only bundled skills. Bare mode skips skill commands and watchers, and refreshes an empty skill cache because every level is skipped. paths-gated skills are hidden from the model until a tool call touches a matching project-root-relative file; activation lasts for the current skill-cache lifetime and resets on a new session or skill-cache refresh. Trusted Folders is disabled by default; when enabled and a workspace is untrusted, Qwen enters restricted safe mode and project customizations are not loaded."

portability:
  portable: true
  non_portable_assets:
    - "allowedTools permission rules using Qwen tool names or Qwen permission syntax"
    - "hooks frontmatter, including Qwen hook event names and command/http hook definitions"
    - "model frontmatter using Qwen model selector syntax"
    - "paths frontmatter semantics and project-root-relative activation state"
    - "priority sorting in the Qwen /skills listing"
    - "Extension qwen-extension.json metadata, conversion from Claude/Gemini extension packages, and extension enablement state"
    - "Bundled skills shipped inside the qwen-code package"
    - "Local scripts, templates, examples, references, and assets that assume Qwen's skill base-directory reminder or host-specific tools"
  rewrite_needed: true
  notes: "The SKILL.md Markdown body, required name, required description, and simple adjacent reference files can usually be linked as Agent Skills. Claudine should preserve Qwen placement but classify Qwen-only frontmatter and extension packaging as rewrite-needed. The .agents/skills alias is more portable than .qwen/skills, but Qwen-specific metadata still needs filtering or mapping before sharing with providers that do not implement those keys."

cli_params:
  - flag: "--bare"
    description: "Minimal mode; skips implicit startup discovery. Source shows skill commands are skipped and all skill levels are skipped during cache refresh."
    example: "qwen --bare -p \"summarize\""
  - flag: "--safe-mode"
    description: "Disables customizations including context files, hooks, extensions, skills, and MCP servers. Source shows only bundled skills are considered in safe mode."
    example: "qwen --safe-mode"
  - flag: "--extensions <name>[,<name>...]"
    description: "Restricts the session to named extensions. This can change which extension skills are active."
    example: "qwen --extensions my-extension"
  - flag: "--list-extensions"
    description: "Lists available extensions and exits; useful for seeing extension sources that may contribute skills."
    example: "qwen --list-extensions"
  - flag: "--include-directories <dir> / --add-dir <dir>"
    description: "Adds workspace directories for context discovery. Source-confirmed skill discovery itself is rooted at the project root, not these include directories."
    example: "qwen --include-directories ../shared"
  - flag: "qwen extensions install <source> [--scope user|project|workspace] [--registry <url>] [--consent] [--ref <ref>] [--auto-update] [--pre-release]"
    description: "Installs an extension that may contribute skills. Project/workspace scope enables it only for the current workspace; --consent skips the security confirmation."
    example: "qwen extensions install @scope/my-extension --scope project --consent"
  - flag: "qwen extensions uninstall <name>"
    description: "Removes an installed extension and therefore any skills it contributed."
    example: "qwen extensions uninstall my-extension"
  - flag: "qwen extensions update [<name>] [--all]"
    description: "Updates extension contents; updated extensions may add, remove, or change skills."
    example: "qwen extensions update --all"
  - flag: "/skills [<skill-name>]"
    description: "Interactive slash command for listing available user-invocable skills or directly invoking a skill body."
    example: "/skills code-analyzer"
  - flag: "/extensions install|manage|explore"
    description: "Interactive extension management; changes hot-reload and can enable or disable extension-provided skills without restarting."
    example: "/extensions manage"
  - flag: "/trust"
    description: "Interactive trust management when folder trust is enabled. Untrusted workspaces run in restricted safe mode."
    example: "/trust"

env_vars:
  - name: QWEN_HOME
    effect: "Overrides the global Qwen config directory, including the canonical user skills directory from ~/.qwen/skills to $QWEN_HOME/skills. Does not redirect ~/.agents/skills."
  - name: QWEN_RUNTIME_DIR
    effect: "Overrides runtime output storage. It does not relocate skill definitions, but can affect adjacent runtime files used while skills run."
  - name: QWEN_CODE_SAFE_MODE
    effect: "Truthy value enables safe mode, which disables customizations and limits skill loading to bundled skills."
  - name: QWEN_CODE_SYSTEM_SETTINGS_PATH
    effect: "Overrides the system settings file path. System settings can include skills.disabled entries that hide skills."
  - name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the system defaults settings file path. System defaults can include skills.disabled entries that lower scopes cannot remove because the list is union-merged."
  - name: NPM_TOKEN
    effect: "Used by npm extension installation. It can affect access to private extension packages that contribute skills."

changes:
  - "Refreshed 2026-07-03: re-verified against Qwen Code 0.15.6 bundled skills shipped under /opt/homebrew/Cellar/qwen-code/0.15.6/libexec/lib/node_modules/@qwen-code/qwen-code/bundled/, the qc-helper/docs/features/skills.md shipped with that package, and the current SkillManager source on QwenLM/qwen-code main (1215 lines)."
  - "Confirmed SKILL_PROVIDER_CONFIG_DIRS = ['.qwen', '.agents'] continues to scan both user/project roots; .qwen uses Storage.getGlobalQwenDir() (so QWEN_HOME redirects it), while .agents stays anchored to the OS home directory."
  - "Confirmed cross-level precedence (project > user > extension > bundled) and the alphabetical model-facing sort; priority only re-orders the /skills display layer."
  - "Confirmed safe mode limits levels to ['bundled'] and bare mode skips every level (startWatching still refreshes the cache, but listSkillsAtLevel returns [])."
  - "Confirmed path-activation (paths: frontmatter) gates a skill from model listing until a tool invocation touches a picomatch-matching project-root-relative file; disable-model-invocation skills are excluded from the activation registry to avoid misleading reminders."
  - "Confirmed allowedTools uses camelCase in Qwen (other providers use allowed-tools kebab-case); current bundled review and qc-helper skills both follow this convention."
  - "Confirmed watcher depth is intentionally shallow (WATCHER_MAX_DEPTH = 2) so node_modules-style subtrees do not exhaust file descriptors."
  - "Confirmed bundled skill directory is resolved at runtime via resolveBundleDir(import.meta.url) + 'bundled', so the absolute path varies by install method (npm, pnpm, Homebrew, manual)."

requires_claudine_update: true
reason: "Claudine's linker should add Qwen Code first-class skill locations for ~/.qwen/skills, .qwen/skills, the source-confirmed ~/.agents/skills and .agents/skills aliases, extension skills, and bundled package skills. Portability rules should mark Qwen-specific allowedTools, hooks, model, paths, priority, settings skills.disabled, and qwen-extension.json packaging as rewrite-needed or provider-managed."
---

# Qwen Code Agent Skills

## Overview

Qwen Code has first-class Agent Skills. A skill is a durable directory containing a required `SKILL.md` file with YAML frontmatter and Markdown instructions, plus optional adjacent supporting files. Qwen exposes skills through two runtime paths:

- Model invocation: the model sees active skills in an `<available_skills>` system reminder and calls the `Skill` tool with a skill name.
- User invocation: a user can run a skill directly through `/skills <skill-name>` or the generated `/<skill-name>` slash command, unless the skill opts out with `user-invocable: false`.

The implementation is file-system based and source-confirmed in `SkillManager`, `SkillTool`, and `SkillCommandLoader`. Qwen discovers four tiers: project, user, extension, and bundled. Project and user tiers scan both Qwen-specific directories and the interoperable `.agents/skills` alias. Extension skills are parsed from enabled extensions. Bundled skills are shipped inside the installed `qwen-code` package and behave as the lowest-precedence built-in tier.

Local inspection on this macOS host (refreshed 2026-07-03) found `/opt/homebrew/bin/qwen` installed as Homebrew `qwen-code` 0.15.6, with bundled skills under `/opt/homebrew/Cellar/qwen-code/0.15.6/libexec/lib/node_modules/@qwen-code/qwen-code/bundled/`. The shipped bundles are `batch/`, `loop/`, `qc-helper/`, and `review/`; `qc-helper` carries a complete `docs/` mirror of the user documentation, including `docs/features/skills.md` which restates `~/.qwen/skills/`, `.qwen/skills/`, and the extension `skills` field as the public surface.

A `~/.qwen` directory does exist on this host, populated with `skills/`, `commands/`, `debug/`, `output-language.md`, `projects/`, `settings.json`, and `oauth_creds.json` — but every entry under `~/.qwen/skills/` (174 directories) is a symlink into `~/.claude/skills/`, so the user skills tree is effectively empty of original Qwen-authored content here. The `~/.agents` directory contains a single `find-skills` skill plus a `.skill-lock.json` from the Vercel Labs skills installer. These observations confirm the source-confirmed `.qwen` and `.agents` aliases are active but also show real-world users commonly populate them by symlinking rather than by copy.

## Locations

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| User | `~/.qwen/skills/<skill-name>/SKILL.md` | `~/.qwen/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.qwen\skills\<skill-name>\SKILL.md` | Official personal skill path. `QWEN_HOME` redirects the `.qwen` config root. |
| User alias | `~/.agents/skills/<skill-name>/SKILL.md` | `~/.agents/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.agents\skills\<skill-name>\SKILL.md` | Source-confirmed alias from `SKILL_PROVIDER_CONFIG_DIRS = ['.qwen', '.agents']`; not called out in the public skill guide. |
| Project | `.qwen/skills/<skill-name>/SKILL.md` | `.qwen/skills/<skill-name>/SKILL.md` | `.qwen\skills\<skill-name>\SKILL.md` | Official project skill path under the resolved project root. |
| Project alias | `.agents/skills/<skill-name>/SKILL.md` | `.agents/skills/<skill-name>/SKILL.md` | `.agents\skills\<skill-name>\SKILL.md` | Source-confirmed project alias. |
| Extension, user | `~/.qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md` | `~/.qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.qwen\extensions\<extension-name>\skills\<skill-name>\SKILL.md` | Enabled user-scope extensions can contribute skills. |
| Extension, project | `.qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md` | `.qwen/extensions/<extension-name>/skills/<skill-name>/SKILL.md` | `.qwen\extensions\<extension-name>\skills\<skill-name>\SKILL.md` | Enabled project-scope extensions can contribute skills. |
| Bundled | `<qwen-code package>/bundled/<skill-name>/SKILL.md` | `<qwen-code package>/bundled/<skill-name>/SKILL.md` | `<qwen-code package>\bundled\<skill-name>\SKILL.md` | Built into the package; exact install root depends on npm, Homebrew, or other packaging. |

System settings files are not skill storage locations, but they can affect skill visibility through `skills.disabled`. Their default paths are macOS `/Library/Application Support/QwenCode/settings.json`, Linux `/etc/qwen-code/settings.json`, and Windows `C:\ProgramData\qwen-code\settings.json`; `QWEN_CODE_SYSTEM_SETTINGS_PATH` can override them. System defaults are adjacent `system-defaults.json` files and can be overridden by `QWEN_CODE_SYSTEM_DEFAULTS_PATH`.

## File Format

A Qwen skill directory has a fixed entry point:

```text
my-skill/
├── SKILL.md
├── reference.md
├── examples.md
├── scripts/
│   └── helper.py
└── templates/
    └── template.txt
```

`SKILL.md` must start with YAML frontmatter and then Markdown body content:

```markdown
---
name: code-analyzer
description: Analyzes code structure and suggests maintainability improvements
priority: 10
paths:
  - "src/**/*.ts"
---

# Code Analyzer

Use repository-local evidence before recommending refactors.
```

Recognized metadata:

| Field | Required | Behavior |
|---|---:|---|
| `name` | Yes | Non-empty string matching `/^[\p{L}\p{N}_:.-]+$/u`; whitespace, slashes, brackets, and other structurally unsafe characters are rejected. |
| `description` | Yes | Non-empty string used for model discovery and listings. |
| `allowedTools` | No | Array of Qwen permission-rule strings. Applied as session-scoped allow rules when the skill is invoked. This is additive and does not restrict visible tools. |
| `hooks` | No | Qwen hook configuration for project/user/bundled skills; command and HTTP hook definitions are parsed. Extension skill parsing currently does not extract `hooks`. |
| `model` | No | Skill-local model selector. Empty or `inherit` means use the session model. |
| `argument-hint` | No | Slash-command completion hint. |
| `when_to_use` | No | Additional model-facing routing text. |
| `disable-model-invocation` | No | `true` hides the skill from model invocation, while preserving user invocation unless separately disabled. |
| `user-invocable` | No | `false` hides direct user invocation and `/skills` picker entries, while preserving model visibility unless separately disabled. |
| `paths` | No | Array of project-root-relative glob patterns. The model cannot see the skill until a tool call touches a matching file. |
| `priority` | No | Finite number. Higher values sort earlier in `/skills` display only; programmatic/model listings remain alphabetical. |

The parser normalizes BOM and CRLF line endings, requires `---` frontmatter delimiters, validates required fields, and trims the Markdown body. Plain files in a skills directory are skipped; each skill must be a subdirectory. Directory symlinks can be accepted if their target resolves to a directory.

Supporting files are not eagerly loaded as separate resources. The Skill tool injects the body with a base-directory reminder and instructs the model to resolve referenced files and scripts from the skill directory. That means adjacent files are portable only when the target provider preserves directory-relative asset access.

## Discovery and Precedence

Qwen discovers skills through `SkillManager.listSkills()` in this precedence order:

1. Project skills.
2. User skills.
3. Extension skills.
4. Bundled skills.

Duplicate names are shadowed by the first higher-precedence tier. The final list is sorted alphabetically by name for model-facing consumers. `priority` does not change model listing order; it only changes the `/skills` UI display order.

Normal mode starts file watchers for skill directories, refreshes the cache on changes, and notifies runtime consumers. Watch depth is intentionally shallow because the required layout is `<skill-name>/SKILL.md`; deeper subtrees such as `node_modules` are avoided. Parse errors are stored and surfaced through debug/UI paths instead of aborting every tier.

Model visibility is narrower than disk discovery:

- `disable-model-invocation: true` removes the skill from `<available_skills>` and blocks model use.
- `skills.disabled` in settings hides matching names case-insensitively from model listings, slash commands, `/skills`, and completion.
- `paths` gates model discovery until a matching file is touched by a tool call. Matching uses `picomatch` with dotfile support against paths relative to the project root. Files outside the project root do not activate a skill. Once activated, the skill stays active until the skill cache is rebuilt or the session ends.

User visibility is controlled separately:

- Skills are user-invocable by default.
- `user-invocable: false` removes direct slash invocation and `/skills` picker visibility but does not hide the skill from the model.
- `skills.disabled` hides both model and user surfaces.

Safe mode and bare mode are important:

- `--safe-mode` or `QWEN_CODE_SAFE_MODE` disables customizations. Source shows `SkillManager.refreshCache()` limits levels to `bundled` in safe mode, so project, user, and extension skills are not loaded.
- `--bare` is stricter for skills in current source: skill commands are skipped, watchers are skipped, and each level returns no skills during cache refresh.

Trusted Folders is disabled by default. If enabled through `security.folderTrust.enabled`, an untrusted workspace runs in restricted safe mode. The trust docs explicitly say project settings and extension management are restricted; source behavior means non-bundled skills are also skipped while safe mode is active.

Extensions participate when enabled. `qwen-extension.json` has a `skills` property whose default is `skills`; each skill underneath follows the same `SKILL.md` shape. Qwen can install extensions from local paths, Git repositories, archive URLs, scoped npm packages, Claude Code marketplaces, and Gemini CLI extension galleries. Claude and Gemini extension manifests are converted into Qwen's `qwen-extension.json` format during installation. Extension command conflicts are documented as lowest-precedence with prefixed fallback names; file-backed skills use the SkillManager tier precedence above, so extension skills are below project and user skills and above bundled skills.

## Portability

Qwen's basic artifact shape is portable: a directory with `SKILL.md`, YAML frontmatter, Markdown body, and optional adjacent files is compatible with the Agent Skills pattern used by other providers. The most portable target locations are the `.agents/skills` aliases because they are not Qwen-branded, but their metadata may still be Qwen-specific.

Portable without semantic rewrite:

- `SKILL.md` body content that contains provider-neutral instructions.
- `name` and `description` when the destination provider accepts the same name grammar.
- Adjacent Markdown reference files when relative links are preserved.

Rewrite or filtering needed:

- `allowedTools` uses Qwen permission-rule strings and Qwen tool names.
- `hooks` is Qwen-specific and can run command or HTTP hooks.
- `model` uses Qwen's selector semantics.
- `paths` has Qwen-specific activation semantics tied to tool-touched files and project-root-relative `picomatch`.
- `priority`, `user-invocable`, `disable-model-invocation`, `argument-hint`, and `when_to_use` may not be honored by other providers or may require key mapping.
- Extension skills depend on `qwen-extension.json`, extension enablement state, conversion rules, marketplace metadata, and optional extension settings.
- Scripts, templates, examples, references, and assets depend on host runtimes, OS path syntax, and the destination provider's ability to expose the skill directory to the model.

Bundled skills are provider-managed and should not be linked as user-authored resources unless a user explicitly copies them into a normal skill directory.

## Claudine Linking Notes

Claudine should recognize these Qwen skill roots:

- User: `~/.qwen/skills` and `~/.agents/skills`.
- Project: `.qwen/skills` and `.agents/skills`.
- Extension: enabled extension `skills` directories under `~/.qwen/extensions` and `.qwen/extensions`.
- Bundled: package-managed `<qwen-code package>/bundled`, classified as provider-managed.

The linker should prefer linking normal user/project skills, not bundled skills. It should preserve the skill directory as a unit, including adjacent assets, and classify Qwen-specific metadata as rewrite-needed. For cross-provider links, `.agents/skills` is the least provider-branded path, but Claudine still needs metadata inspection before declaring the skill portable.

The linker should avoid treating `QWEN.md`, ordinary commands, memories, chat history, settings files, and docs pages as Agent Skills. Settings matter only insofar as `skills.disabled`, safe mode, folder trust, `QWEN_HOME`, and extension enablement affect loading. If Claudine models provider state, it should add Qwen fields for `.agents` aliases, `skills.disabled`, safe/bare mode behavior, and extension skill participation.

`requires_claudine_update` is `true` because current linking metadata should include Qwen's first-class skill locations, the `.agents/skills` aliases, Qwen-specific frontmatter keys, and the provider-managed bundled tier.

## Changelog

- **2026-07-03** — Refresh against Qwen Code 0.15.6. Verified the bundled-skill layout (`batch`, `loop`, `qc-helper`, `review`) and the shipped `qc-helper/docs/features/skills.md` mirror of the public skills documentation. Confirmed the current `SkillManager` source (QwenLM/qwen-code `main`, 1215 lines) still exposes `SKILL_PROVIDER_CONFIG_DIRS = ['.qwen', '.agents']`, project > user > extension > bundled precedence, safe/bare-mode behavior, the `paths:` activation registry that excludes `disable-model-invocation` skills, and the camelCase `allowedTools` convention. Updated frontmatter `last_updated`, `agent`, and `model`. No new storage locations, file format fields, or CLI/env knobs surfaced since the 2026-07-02 first run.
- **2026-07-02** — Initial research. Established first-class support, four-tier discovery (project, user, extension, bundled), the `.agents/skills` aliases as Qwen-shipped-but-undocumented, source-confirmed `SKILL_PROVIDER_CONFIG_DIRS`, the camelCase `allowedTools` frontmatter, the path-activation registry, safe/bare-mode filtering, Trusted Folders gating, and the cross-extension packaging conversions from Claude/Gemini manifests.

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code Agent Skills documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/skills/)
- [Qwen Code configuration settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code extension documentation](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/)
- [Qwen Code extension getting started guide](https://qwenlm.github.io/qwen-code-docs/en/users/extension/getting-started-extensions/)
- [Qwen Code commands documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- [Qwen Code trusted folders documentation](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/trusted-folders/)
- [Qwen Code repository](https://github.com/QwenLM/qwen-code)
- [SkillManager source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/skills/skill-manager.ts)
- [Skill types source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/skills/types.ts)
- [Skill loading source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/skills/skill-load.ts)
- [Skill activation source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/skills/skill-activation.ts)
- [Skill tool source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/tools/skill.ts)
- [Skill utility source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/tools/skill-utils.ts)
- [Skill command loader source](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/services/SkillCommandLoader.ts)
- [Storage source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/config/storage.ts)
- [CLI config source](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/config/config.ts)
- [Settings schema source](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/config/settingsSchema.ts)
- [Safe mode source](https://github.com/QwenLM/qwen-code/blob/main/packages/core/src/utils/safe-mode.ts)
