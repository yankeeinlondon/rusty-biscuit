---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://opencode.ai/
docs: https://opencode.ai/docs/
skills_docs: https://opencode.ai/docs/skills/

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.config/opencode/skills/<name>/SKILL.md
    notes: Documented global skill path; canonical plural form. Replaced by `OPENCODE_CONFIG_DIR` when set. Local host uses singular alias (next row) and not this plural form.
  - os: linux
    scope: user
    path: ~/.config/opencode/skills/<name>/SKILL.md
    notes: Documented global skill path; canonical plural form. Replaced by `OPENCODE_CONFIG_DIR` when set.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\skills\\<name>\\SKILL.md"
    notes: Windows form of the documented global config path. `OPENCODE_CONFIG_DIR` replaces the config root.
  - os: macos
    scope: user
    path: ~/.config/opencode/skill/<name>/SKILL.md
    notes: Backwards-compatible singular alias under the same OpenCode config root; source glob is `{skill,skills}/**/SKILL.md`. Locally observed with 81 symlinks on this host.
  - os: linux
    scope: user
    path: ~/.config/opencode/skill/<name>/SKILL.md
    notes: Backwards-compatible singular alias under the same OpenCode config root; source glob is `{skill,skills}/**/SKILL.md`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\skill\\<name>\\SKILL.md"
    notes: Backwards-compatible singular alias under the same OpenCode config root; source glob is `{skill,skills}/**/SKILL.md`.
  - os: macos
    scope: user
    path: ~/.claude/skills/<name>/SKILL.md
    notes: Claude-compatible global external skill path. Scanned only when `OPENCODE_DISABLE_EXTERNAL_SKILLS` and `OPENCODE_DISABLE_CLAUDE_CODE[_SKILLS]` are unset. Observed locally with 82 entries.
  - os: linux
    scope: user
    path: ~/.claude/skills/<name>/SKILL.md
    notes: Claude-compatible global external skill path; gated by `OPENCODE_DISABLE_EXTERNAL_SKILLS` and `OPENCODE_DISABLE_CLAUDE_CODE[_SKILLS]`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills\\<name>\\SKILL.md"
    notes: Claude-compatible global external skill path; gated by `OPENCODE_DISABLE_EXTERNAL_SKILLS` and `OPENCODE_DISABLE_CLAUDE_CODE[_SKILLS]`.
  - os: macos
    scope: user
    path: ~/.agents/skills/<name>/SKILL.md
    notes: Agent-compatible global external skill path. Scanned only when `OPENCODE_DISABLE_EXTERNAL_SKILLS` is unset. Locally has 1 entry.
  - os: linux
    scope: user
    path: ~/.agents/skills/<name>/SKILL.md
    notes: Agent-compatible global external skill path; gated by `OPENCODE_DISABLE_EXTERNAL_SKILLS`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<name>\\SKILL.md"
    notes: Agent-compatible global external skill path; gated by `OPENCODE_DISABLE_EXTERNAL_SKILLS`.
  - os: macos
    scope: repo
    path: .opencode/skills/<name>/SKILL.md
    notes: Documented project skill path. Discovered along the current-directory-to-git-worktree walk.
  - os: linux
    scope: repo
    path: .opencode/skills/<name>/SKILL.md
    notes: Documented project skill path. Discovered along the current-directory-to-git-worktree walk.
  - os: windows
    scope: repo
    path: .opencode\\skills\\<name>\\SKILL.md
    notes: Documented project skill path. Discovered along the current-directory-to-git-worktree walk.
  - os: macos
    scope: repo
    path: .opencode/skill/<name>/SKILL.md
    notes: Backwards-compatible singular alias under the project config root; source glob matches both.
  - os: linux
    scope: repo
    path: .opencode/skill/<name>/SKILL.md
    notes: Backwards-compatible singular alias under the project config root; source glob matches both.
  - os: windows
    scope: repo
    path: .opencode\\skill\\<name>\\SKILL.md
    notes: Backwards-compatible singular alias under the project config root; source glob matches both.
  - os: macos
    scope: repo
    path: .claude/skills/<name>/SKILL.md
    notes: Claude-compatible project external skill path, scanned while walking up from CWD to the git worktree.
  - os: linux
    scope: repo
    path: .claude/skills/<name>/SKILL.md
    notes: Claude-compatible project external skill path, scanned while walking up from CWD to the git worktree.
  - os: windows
    scope: repo
    path: .claude\\skills\\<name>\\SKILL.md
    notes: Claude-compatible project external skill path, scanned while walking up from CWD to the git worktree.
  - os: macos
    scope: repo
    path: .agents/skills/<name>/SKILL.md
    notes: Agent-compatible project external skill path, scanned while walking up from CWD to the git worktree.
  - os: linux
    scope: repo
    path: .agents/skills/<name>/SKILL.md
    notes: Agent-compatible project external skill path, scanned while walking up from CWD to the git worktree.
  - os: windows
    scope: repo
    path: .agents\\skills\\<name>\\SKILL.md
    notes: Agent-compatible project external skill path, scanned while walking up from CWD to the git worktree.
  - os: macos
    scope: other
    path: configured-path/**/SKILL.md
    notes: From `skills.paths[]` entries in config. `~/` expands to home, absolute paths used as-is, other paths resolve relative to the launch directory.
  - os: linux
    scope: other
    path: configured-path/**/SKILL.md
    notes: From `skills.paths[]` entries in config. `~/` expands to home, absolute paths used as-is, other paths resolve relative to the launch directory.
  - os: windows
    scope: other
    path: configured-path\\**\\SKILL.md
    notes: From `skills.paths[]` entries in config. `~/` expands to home, absolute paths used as-is, other paths resolve relative to the launch directory.
  - os: macos
    scope: other
    path: <opencode-cache>/skills/<name>/SKILL.md
    notes: From `skills.urls[]` entries in config. Each URL serves an `<url>/index.json`; listed files are downloaded under `<opencode-cache>/skills/<name>/` with atomic versioned staging.
  - os: linux
    scope: other
    path: <xdg-cache>/opencode/skills/<name>/SKILL.md
    notes: From `skills.urls[]` entries in config. Files cached under the OpenCode cache directory.
  - os: windows
    scope: other
    path: "%LOCALAPPDATA%\\opencode\\skills\\<name>\\SKILL.md"
    notes: From `skills.urls[]` entries in config. Files cached under the OpenCode cache directory.
  - os: macos
    scope: system
    path: /Library/Application Support/opencode/opencode.json(c)
    notes: System managed config directory on macOS. Skills are not stored here directly, but `permission.skill` and other settings in these files affect skill discovery and visibility.
  - os: linux
    scope: system
    path: /etc/opencode/opencode.json(c)
    notes: System managed config directory on Linux. Skills are not stored here directly; managed settings in these files affect skill discovery and visibility.
  - os: windows
    scope: system
    path: "C:\\ProgramData\\opencode\\opencode.json(c)"
    notes: System managed config directory on Windows (`%ProgramData%\opencode`). Skills are not stored here directly; managed settings in these files affect skill discovery and visibility.
  - os: macos
    scope: system
    path: /Library/Managed Preferences/<user>/ai.opencode.managed.plist
    notes: macOS-only MDM-deployed plist for the `ai.opencode.managed` preference domain. Highest-priority config; not used as a skill directory but controls skill visibility via permission settings.
  - os: macos
    scope: other
    path: <built-in>
    notes: Built-in `customize-opencode` skill ships in the binary. Registered before disk discovery so a user-disk skill with the same name can override it.
  - os: linux
    scope: other
    path: <built-in>
    notes: Built-in `customize-opencode` skill ships in the binary. Registered before disk discovery so a user-disk skill with the same name can override it.
  - os: windows
    scope: other
    path: <built-in>
    notes: Built-in `customize-opencode` skill ships in the binary. Registered before disk discovery so a user-disk skill with the same name can override it.

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
  body_format: markdown
  notes: |
    Documented contract: one directory per skill name containing `SKILL.md` with YAML frontmatter.
    Docs require `name` to match the regex `^[a-z0-9]+(-[a-z0-9]+)*$` (1-64 chars, no leading/trailing/consecutive hyphens) and to equal the directory name; `description` must be 1-1024 chars. Unknown frontmatter fields are ignored.
    Current source is more permissive: the loader accepts any string `name`, treats `description` as optional, does not enforce the name regex, and does not require the directory to match `name`. Skills without `description` are stored internally but filtered out of `<available_skills>` model guidance by `Skill.fmt()` ("No skills are currently available.").
    The `skill` tool returns `SKILL.md` content plus a sampled list of up to 10 non-`SKILL.md` files inside the skill directory (via ripgrep); it tells the model the directory is the base for relative `scripts/` or `references/` references.
    URL sources use a separate index format: `{ "skills": [ { "name": string, "files": string[], "version"?: string } ] }` served at `<url>/index.json`. Each entry must include `SKILL.md` in `files`; otherwise OpenCode logs a warning and skips it.

discovery:
  mechanism: |
    At instance startup, OpenCode builds a skill registry keyed by frontmatter `name`. Discovery runs in this order: (1) register the built-in `customize-opencode` skill; (2) when external skills are not disabled, scan `~/.claude/skills/` (unless Claude skills are disabled) and `~/.agents/skills/`, then walk up from the current directory to the git worktree and scan matching `.claude/skills/` and `.agents/skills/` along the way; (3) scan every OpenCode config directory for `{skill,skills}/**/SKILL.md`; (4) load `skills.paths[]` entries and recursively scan each; (5) load `skills.urls[]`, fetch each remote index, cache the listed files, and scan the resulting cache directories.
    Source confirms the scan glob differs by source family: external directories use `skills/**/SKILL.md`; OpenCode config directories use `{skill,skills}/**/SKILL.md`; configured paths use `**/SKILL.md` recursively.
    Models do not receive full skill bodies up front. They see `<available_skills>` entries with `name` and `description` only; the model loads a body on demand by calling the native `skill` tool with `{ name }`. The tool resolves the skill through the registry, performs a permission check via `ctx.ask({ permission: "skill", patterns: [name], always: [name] })`, then returns the content plus base-directory hint and sampled file list.
  precedence: |
    Duplicate names are last-writer-wins in the in-memory registry, with a `WARN` log line for each collision ("duplicate skill name" with `existing`/`duplicate` paths). The built-in `customize-opencode` skill is registered first so a disk skill of the same name overrides it.
    Source scan order is: built-in → external global (`~/.claude` then `~/.agents`) → external project walk → native config dirs → `skills.paths[]` → `skills.urls[]`. Inside each scan step, file order is determined by `Glob.scan`. The implementation does not merge same-name skills; the final loaded entry replaces earlier ones.
    Permission visibility: agent-level `permission.skill` rules apply on top of discovery. `deny` hides matching skills from the agent's `<available_skills>` and rejects access; `ask` prompts before loading; `allow` loads immediately. `tools.skill: false` disables the skill tool for an agent and omits the entire `<available_skills>` section.
  enable_disable: |
    Per-agent `permission.skill` (built-in or custom agents) can `allow`, `deny`, or `ask` per glob pattern. `tools.skill: false` disables the skill tool entirely for an agent. Top-level `permission.skill` in `opencode.json(c)` applies to built-in agents; custom agents can override via their own `permission:` frontmatter or `agent.<name>.permission` in config.
    Globals: removing or renaming a skill directory disables that skill. `OPENCODE_DISABLE_EXTERNAL_SKILLS=1` skips both `.claude/skills/` and `.agents/skills/` external scans. `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1` (or the broad `OPENCODE_DISABLE_CLAUDE_CODE=1`) skips `.claude/skills/` only. `OPENCODE_DISABLE_PROJECT_CONFIG=1` skips project config files (and therefore project-level `skills.paths[]`, `skills.urls[]`, permissions, and `.opencode/skills/`). `OPENCODE_CONFIG_DIR` replaces the global OpenCode config root, which changes where native global skills are scanned.
    `--pure` / `OPENCODE_PURE=1` disables external plugins (not disk skills); it can indirectly remove plugin-contributed skill sources. There is no dedicated `--no-skills` flag; the closest global disable is `OPENCODE_DISABLE_EXTERNAL_SKILLS`.
    There is no documented trust prompt specific to skill discovery. Skill loading is mediated by OpenCode's permission system: denied skill names are hidden from the agent and rejected; `ask` prompts the user before loading.
  notes: |
    `opencode debug skill` lists the current registry as JSON, including `name`, `description`, `location`, and `content` for each registered skill. Verified on this host: the command emitted dozens of `WARN duplicate skill name` lines because both `~/.config/opencode/skill/` and `~/.claude/skills/` (plus `.opencode/skill/` and `.claude/skills/` inside this worktree) expose overlapping symlinks to the same source skills. This is direct evidence of last-writer-wins precedence and of the scanner walking both native and external roots.
    Managed MDM plist and `/etc/opencode/` directories don't host `SKILL.md` files themselves; they only override `permission` (including `skill`) and other settings.

portability:
  portable: false
  non_portable_assets:
    - "OpenCode-only built-in `customize-opencode` skill (binary-internal; not exportable)"
    - "`skills.paths[]` and `skills.urls[]` config wiring (schema fields; not portable across providers)"
    - "URL skill indexes served as `<url>/index.json` with `skills[].files[]` and optional `version`"
    - "Permission rules under `permission.skill` and agent `tools.skill` (OpenCode-specific schema)"
    - "OpenCode-only env vars: `OPENCODE_DISABLE_EXTERNAL_SKILLS`, `OPENCODE_DISABLE_CLAUDE_CODE[_SKILLS]`, `OPENCODE_PURE`, `OPENCODE_DISABLE_PROJECT_CONFIG`, `OPENCODE_CONFIG_DIR`, `OPENCODE_PERMISSION`"
    - "Provider-specific instructions that mention OpenCode's `skill` tool, `opencode.json(c)`, `.opencode/`, plugins, MCP config, or permission names"
    - "Sibling files, scripts, references, and assets that depend on OpenCode's base-directory hint or host-specific executables"
    - "OpenCode-only skill tool return shape (`<skill_content>`, `<skill_files>`, base-directory hint)"
  rewrite_needed: true
  notes: |
    The Markdown body of a simple `SKILL.md` is largely portable, and OpenCode intentionally reads Claude-compatible `.claude/skills` and agent-compatible `.agents/skills`. A minimal skill with `name`, `description`, and provider-neutral Markdown can be copied or linked into another provider's expected directory with most semantics intact.
    Claudine should still classify OpenCode skills as requiring review/rewrite rather than as-is portable because: (1) OpenCode's implemented metadata contract is more permissive than Claude's docs but OpenCode tells users to write the stricter form, so provider targets differ; (2) `description` is the runtime routing signal — without it the skill is hidden from the model; (3) `permission.skill`, `tools.skill`, `skills.paths`, `skills.urls`, and the URL index are OpenCode-specific; (4) the built-in `customize-opencode` skill is OpenCode-only and must not be exported as a generic skill.
    A safe rewrite preserves `name`, `description`, the Markdown body, and same-directory assets; drops or maps OpenCode-specific config and permission surfaces; rewrites provider-specific instructions (opencode.json, .opencode/, plugins, MCP config, skill tool name) to the target provider's terminology.

cli_params:
  - flag: --agent <agent>
    description: Selects the active agent; agent `permission.skill` and `tools.skill` decide whether skills appear in `<available_skills>` and load.
    example: opencode run --agent plan "review this"
  - flag: --dir <path>
    description: Sets the launch directory for `run`, `attach`, and ACP sessions; affects repo-local `.opencode/skills`, `.claude/skills`, `.agents/skills` walks, and relative `skills.paths[]` entries.
    example: opencode run --dir ./packages/api "use the repo skill"
  - flag: --pure
    description: Runs without external plugins. Does not disable normal disk skills; can remove plugin-contributed skill sources.
    example: opencode --pure run "explain config"
  - flag: opencode debug skill
    description: Lists all currently registered skills as JSON, including the built-in `customize-opencode` and file-backed skills.
    example: opencode debug skill

env_vars:
  - name: OPENCODE_CONFIG_DIR
    effect: Replaces the global OpenCode config directory (and therefore the native global `skill/` and `skills/` scan roots).
  - name: OPENCODE_CONFIG
    effect: Loads an additional explicit config file; can add `skills.paths`, `skills.urls`, `permission.skill`, agents, or plugin settings.
  - name: OPENCODE_CONFIG_CONTENT
    effect: Injects inline JSON config as a final local-scope merge; can add skill sources or override permissions for the session.
  - name: OPENCODE_DISABLE_PROJECT_CONFIG
    effect: Skips local project config files, which can remove project `skills.paths`, `skills.urls`, `permission.skill`, and some `.opencode/` discovery.
  - name: OPENCODE_DISABLE_EXTERNAL_SKILLS
    effect: Skips both `.claude/skills` and `.agents/skills` scans at global and project scopes.
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: Skips only `.claude/skills` scans; leaves `.agents/skills` external scans enabled. OR-ed with `OPENCODE_DISABLE_CLAUDE_CODE` so either flag disables Claude skills.
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: Broad Claude-compatibility disable flag; combines with `OPENCODE_DISABLE_CLAUDE_CODE_PROMPT` for prompt disabling and with `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` for skill scanning.
  - name: OPENCODE_PURE
    effect: Disables external plugins; can indirectly remove plugin-provided skill sources but does not disable normal disk skills.
  - name: OPENCODE_PERMISSION
    effect: Merges inline JSON permissions into config; can deny, ask, or allow `skill` patterns.
  - name: OPENCODE_TEST_MANAGED_CONFIG_DIR
    effect: Test-only override for the system managed config directory (used by `ConfigManaged.managedConfigDir()`). Not user-facing.

changes:
  - "Verified against official docs page `https://opencode.ai/docs/skills/` (last updated 2026-07-03) and the current source at `packages/opencode/src/skill/{index,discovery}.ts`, `packages/opencode/src/tool/skill.ts`, `packages/opencode/src/config/{config,managed}.ts`, and `packages/opencode/src/effect/runtime-flags.ts` on the `dev` branch."
  - "Confirmed that both `~/.config/opencode/skills/` (plural, canonical) and `~/.config/opencode/skill/` (singular, backwards-compatible) are scanned via the `{skill,skills}/**/SKILL.md` glob in the source. Locally observed 81 entries in the singular alias on this host."
  - "Verified `opencode debug skill` against OpenCode 1.17.13 — it logs `WARN duplicate skill name` for every name collision across `~/.config/opencode/skill/`, `~/.claude/skills/`, `.opencode/skill/`, and `.claude/skills/`, confirming last-writer-wins precedence and the explicit scan order."
  - "Confirmed system managed config locations from `managed.ts`: `/Library/Application Support/opencode` (macOS), `/etc/opencode` (Linux), `%ProgramData%\\opencode` (Windows), plus macOS-only `/Library/Managed Preferences[/<user>]/ai.opencode.managed.plist`."
  - "Captured the runtime-flag wiring: `disableClaudeCodeSkills` is the OR of `OPENCODE_DISABLE_CLAUDE_CODE` (broad) and `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` (direct); the same OR-pattern gates `disableClaudeCodePrompt`."
  - "Captured the URL index schema `{ skills: [{ name, files: string[], version?: string }] }` from `packages/opencode/src/skill/discovery.ts`, including the version-mismatch staging/rename and the missing-`SKILL.md` skip rule."
  - "Added per-OS `os: other` records for configured `skills.paths[]` and `skills.urls[]` to make those provider-specific sources explicit instead of only mentioned in prose."
  - "Added system-scope entries for the managed config directories with notes that they don't host `SKILL.md` directly but control skill visibility via `permission.skill`."
  - "Updated frontmatter `model` to match the running model and `last_updated` to 2026-07-03."

requires_claudine_update: true
reason: |
  Claudine should recognize OpenCode as a first-class Agent Skills target. The linker should: (1) scan the documented native user paths `~/.config/opencode/skills/` and the backwards-compatible singular alias `~/.config/opencode/skill/`; (2) scan the project paths `.opencode/skills/` and `.opencode/skill/`; (3) honor the Claude-compatible `.claude/skills/` and agent-compatible `.agents/skills/` external scans at both global and project scopes; (4) treat configured `skills.paths[]` as directory sources and `skills.urls[]` as remote catalogs (provider-specific metadata, not portable); (5) classify OpenCode resources as `first_class` with `rewrite_needed: true` because of provider-specific config, URL index schema, permission rules, and the built-in `customize-opencode` skill; (6) track the env-var gates `OPENCODE_DISABLE_EXTERNAL_SKILLS`, `OPENCODE_DISABLE_CLAUDE_CODE[_SKILLS]`, `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG`, `OPENCODE_CONFIG_CONTENT`, `OPENCODE_DISABLE_PROJECT_CONFIG`, `OPENCODE_PURE`, and `OPENCODE_PERMISSION` so the linker can warn when a target session suppresses the discovery.
---

# OpenCode CLI Agent Skills

## Overview

OpenCode CLI has a first-class Agent Skills implementation. Official docs describe Agent Skills as reusable behavior packaged as `SKILL.md` files, discovered from project and home directories and loaded on demand through OpenCode's native `skill` tool.

The runtime model is two-stage. At startup, OpenCode scans skill locations and builds a registry keyed by frontmatter `name`. During prompt assembly, the active agent receives an `<available_skills>` list containing names and descriptions that survive permission filtering. The agent loads the body only by calling the `skill` tool with the selected name. The tool resolves the skill, performs a permission check (`ctx.ask({ permission: "skill", patterns: [name], always: [name] })`), then returns the Markdown body, the skill base directory, and a sampled list of up to 10 sibling files under the skill directory.

The current research targets OpenCode **1.17.13**, matching the locally installed `opencode --version` output. Local inspection in this non-interactive session found:

- `~/.config/opencode/opencode.jsonc` and `~/.config/opencode/config.json` present but minimal.
- `~/.config/opencode/skill/` (singular, backwards-compatible) populated with **81 symlinks** into `~/.research/library/<name>/skill/`.
- `~/.config/opencode/skills/` (plural, canonical) does **not** exist on this host.
- `~/.claude/skills/` populated with **82 entries** (mix of directories and symlinks into the same library).
- `~/.agents/skills/` populated with **1 entry** (`find-skills`).
- `opencode debug skill` returned only the built-in `customize-opencode` skill — the per-skill entries shown were truncated by the JSON parse but the WARN lines enumerate every duplicate name across `~/.config/opencode/skill/`, `~/.claude/skills/`, `.opencode/skill/`, and `.claude/skills/`, confirming last-writer-wins precedence and the global-then-project scan order.

## Locations

OpenCode recognizes four discovery root families plus a built-in skill. Managed system directories do not host `SKILL.md` directly; they override `permission.skill` and other settings.

| Scope | macOS / Linux | Windows | Notes |
|---|---|---|---|
| User native (plural) | `~/.config/opencode/skills/<name>/SKILL.md` | `%USERPROFILE%\.config\opencode\skills\<name>\SKILL.md` | Documented canonical path. Replaced by `OPENCODE_CONFIG_DIR` when set. |
| User native (singular) | `~/.config/opencode/skill/<name>/SKILL.md` | `%USERPROFILE%\.config\opencode\skill\<name>\SKILL.md` | Backwards-compatible alias; source uses `{skill,skills}/**/SKILL.md`. Locally observed. |
| User Claude-compatible | `~/.claude/skills/<name>/SKILL.md` | `%USERPROFILE%\.claude\skills\<name>\SKILL.md` | External scan; gated by `OPENCODE_DISABLE_EXTERNAL_SKILLS` and `OPENCODE_DISABLE_CLAUDE_CODE[_SKILLS]`. |
| User agent-compatible | `~/.agents/skills/<name>/SKILL.md` | `%USERPROFILE%\.agents\skills\<name>\SKILL.md` | External scan; gated by `OPENCODE_DISABLE_EXTERNAL_SKILLS`. |
| Repo native (plural) | `.opencode/skills/<name>/SKILL.md` | `.opencode\skills\<name>\SKILL.md` | Discovered while walking from CWD up to the git worktree. |
| Repo native (singular) | `.opencode/skill/<name>/SKILL.md` | `.opencode\skill\<name>\SKILL.md` | Backwards-compatible alias; source glob matches both. |
| Repo Claude-compatible | `.claude/skills/<name>/SKILL.md` | `.claude\skills\<name>\SKILL.md` | Discovered while walking from CWD up to the git worktree. |
| Repo agent-compatible | `.agents/skills/<name>/SKILL.md` | `.agents\skills\<name>\SKILL.md` | Discovered while walking from CWD up to the git worktree. |
| Configured directories | `<configured-path>/**/SKILL.md` | `<configured-path>\**\SKILL.md` | From `skills.paths[]` in config. `~/` expands to home; absolute paths used as-is; other paths resolve relative to the launch directory. |
| Configured URLs | `<opencode-cache>/skills/<name>/SKILL.md` | `%LOCALAPPDATA%\opencode\skills\<name>\SKILL.md` | From `skills.urls[]`. Each URL serves `<url>/index.json`; listed files are downloaded into the cache under the skill name. |
| System managed (file) | `/Library/Application Support/opencode/opencode.json(c)` (macOS) · `/etc/opencode/opencode.json(c)` (Linux) | `%ProgramData%\opencode\opencode.json(c)` | Does not host `SKILL.md` directly; controls `permission.skill` and other settings. |
| System managed (plist) | `/Library/Managed Preferences[/<user>]/ai.opencode.managed.plist` | — | macOS-only MDM-deployed plist. Highest-priority config; controls skill visibility via permission settings. |
| Built-in | `<built-in>` | `<built-in>` | `customize-opencode`, registered before disk discovery so a disk skill of the same name overrides it. |

OpenCode's source uses `xdg-basedir` for cache/data/state roots on Linux, `os.homedir()` for `~` expansion, and `process.env.ProgramData` on Windows. The public skills docs use `~/.config/opencode` for global config, so Claudine should render that canonical path in user-facing linking output unless `OPENCODE_CONFIG_DIR` is explicitly set.

## File Format

The documented on-disk shape is one directory per skill:

```text
<name>/
└── SKILL.md
```

`SKILL.md` is Markdown with YAML frontmatter. Public docs say the recognized frontmatter keys are:

| Key | Required | Notes |
|---|---:|---|
| `name` | Yes | Docs require 1-64 chars, lowercase alphanumeric with single hyphen separators (`^[a-z0-9]+(-[a-z0-9]+)*$`), and an exact match with the containing directory name. |
| `description` | Yes | Docs require 1-1024 chars and recommend enough specificity for routing. |
| `license` | No | Recognized by documentation. |
| `compatibility` | No | Recognized by documentation. |
| `metadata` | No | Documented as a string-to-string map. |

The current source is more permissive than the docs. The loader (`packages/opencode/src/skill/index.ts`) accepts any string `name`, treats `description` as optional, ignores unknown frontmatter fields, and does not enforce that `name` equals the directory. Skills without `description` are still added to the registry but `Skill.fmt()` filters them out of model-facing guidance and returns `"No skills are currently available."` if no described skill exists.

The skill tool treats sibling files as part of the skill package. When a skill is loaded, the tool resolves the directory containing `SKILL.md`, tells the model that relative paths such as `scripts/` or `reference/` are relative to that base, and samples up to 10 non-`SKILL.md` files under the directory (via ripgrep with `hidden: true`, `follow: false`, `limit: 10`). Claudine should treat same-directory assets as associated resources, even though `SKILL.md` is the only required file.

Configured URL sources use a separate index format served at `<url>/index.json`:

```json
{
  "skills": [
    {
      "name": "example-skill",
      "version": "1",
      "files": ["SKILL.md", "references/example.md"]
    }
  ]
}
```

Each listed remote skill must include `SKILL.md`. OpenCode downloads the listed files into the cache under the skill name. When `version` is provided, OpenCode writes `.opencode-version` to the cache and, on a version mismatch, stages the new download in a sibling directory and renames atomically only if the new download contains `SKILL.md`; otherwise it keeps the prior cached copy and logs `ERROR failed to refresh skill`.

## Discovery and Precedence

Discovery starts from the active instance's directory and worktree. The current source (`packages/opencode/src/skill/index.ts`) does the following:

1. Register the built-in `customize-opencode` skill in the registry first.
2. Unless `OPENCODE_DISABLE_EXTERNAL_SKILLS` is true, scan global `~/.claude` and `~/.agents` external directories. `~/.claude` is skipped when `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` or `OPENCODE_DISABLE_CLAUDE_CODE` is true (the runtime flag combines both with OR).
3. Walk up from the current directory to the git worktree and scan matching `.claude/skills/**/SKILL.md` and `.agents/skills/**/SKILL.md` along the way.
4. For every OpenCode config directory (global `~/.config/opencode` plus project `.opencode` directories, plus `OPENCODE_CONFIG_DIR` when set), scan `{skill,skills}/**/SKILL.md`.
5. Load `skills.paths[]` from the merged OpenCode config and scan each path recursively with `**/SKILL.md`. `~/` expands to home, absolute paths are used as-is, and other paths resolve relative to the launch directory.
6. Load `skills.urls[]`, fetch each remote index, cache listed files, and scan the resulting cache directories with `**/SKILL.md`.

Duplicate names are not merged. The registry is a `Record<string, Info>`; later additions replace earlier entries and log a `WARN duplicate skill name` line with `existing` and `duplicate` paths. Because the built-in skill is inserted before disk discovery, a file-backed `customize-opencode` skill can shadow the built-in one.

Permissions are applied when building model-facing guidance and when the tool loads a skill. Top-level config can contain pattern-based `permission.skill` entries such as `"*": "allow"`, `"internal-*": "deny"`, or `"experimental-*": "ask"`. Agent-level `permission.skill` overrides apply to the agent. `deny` hides matching skills from the agent's `<available_skills>` and rejects access; `ask` prompts the user before loading; `allow` loads immediately. `tools.skill: false` disables the skill tool for an agent and omits the entire `<available_skills>` section.

The public CLI docs expose `--agent`, `--dir`, and global `--pure`. `--agent` matters because selected-agent permissions decide skill visibility. `--dir` matters for repo-local discovery and relative `skills.paths[]`. `--pure` disables external plugins, not ordinary disk skills, but it can remove plugin-contributed skill sources. `opencode debug skill` is an implementation-facing diagnostic command that lists the current registry as JSON.

Relevant environment variables are:

| Variable | Effect |
|---|---|
| `OPENCODE_CONFIG_DIR` | Replaces the global config directory and therefore the native global skill scan root. |
| `OPENCODE_CONFIG` | Loads an additional config file that can add skill sources or permissions. |
| `OPENCODE_CONFIG_CONTENT` | Loads inline JSON config as a final local-scope merge. |
| `OPENCODE_DISABLE_PROJECT_CONFIG` | Skips project config files and their skill source settings. |
| `OPENCODE_DISABLE_EXTERNAL_SKILLS` | Skips `.claude/skills` and `.agents/skills` external scans. |
| `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` | Skips only `.claude/skills` scans. |
| `OPENCODE_DISABLE_CLAUDE_CODE` | Broad Claude-compatibility disable; combines with `_SKILLS` and `_PROMPT` siblings via OR. |
| `OPENCODE_PURE` | Disables external plugins; can indirectly remove plugin-provided skill sources. |
| `OPENCODE_PERMISSION` | Merges inline JSON permissions, including `skill` rules. |
| `OPENCODE_TEST_MANAGED_CONFIG_DIR` | Test-only override for the system managed config directory. Not user-facing. |

No separate trust gate specific to Agent Skills was found in the official docs or inspected source. Skill loading is controlled by discovery roots, config/env gates, selected-agent tool availability, and permissions.

## Portability

OpenCode is intentionally compatible with common `SKILL.md` layouts: it reads `.claude/skills`, `.agents/skills`, and native `.opencode/skill(s)`. A minimal skill with `name`, `description`, and provider-neutral Markdown can be copied or linked into another provider's expected directory and likely retain most semantics.

Claudine should still mark OpenCode skills as `rewrite_needed` for provider-to-provider linking. The body may mention the OpenCode `skill` tool, `opencode.json(c)`, `.opencode/`, OpenCode permissions, plugins, or MCP config. OpenCode's URL index format and `skills.paths[]` / `skills.urls[]` config are not portable skill artifacts. Agent permissions and `tools.skill` settings are OpenCode-specific policy, not skill metadata. The built-in `customize-opencode` skill is OpenCode-specific and should not be exported as a generic shared skill unless the target provider is also being taught to edit OpenCode configuration.

Assets are conditionally portable. Same-directory Markdown references, scripts, and media should be kept with the skill directory, but Claudine should flag scripts and host-dependent references for review. The OpenCode tool tells the model the base directory and sampled file list at load time; other providers may not provide the same runtime hint.

## Claudine Linking Notes

Claudine should scan and link the following OpenCode targets:

- Native user and repo targets: `skill/` and `skills/` under the effective OpenCode config directories.
- Compatibility targets: `.claude/skills` and `.agents/skills` at user and repo scopes.
- Configured sources: `skills.paths[]` as directory roots and `skills.urls[]` as remote catalogs, with the remote catalog classified as provider-specific metadata rather than a plain skill directory.
- Extension/plugin sources when provider metadata exposes them; do not assume `--pure` sessions can see plugin-provided skills.

The linker should preserve `SKILL.md` plus sibling assets as one package. It should not rely on public-doc-only validation when reading OpenCode sources: current OpenCode accepts a description-less skill internally and does not enforce name/directory matching in source. For outbound links, however, Claudine should produce the documented stricter form because that is what OpenCode tells users to create and what model guidance needs.

The generated portability metadata should classify OpenCode as `first_class`, `portable: false`, and `rewrite_needed: true`. It should explicitly record the env gates `OPENCODE_DISABLE_EXTERNAL_SKILLS`, `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`, `OPENCODE_DISABLE_CLAUDE_CODE`, `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG`, `OPENCODE_CONFIG_CONTENT`, `OPENCODE_DISABLE_PROJECT_CONFIG`, `OPENCODE_PURE`, and `OPENCODE_PERMISSION` so the linker can warn when a target session suppresses the discovery.

## Changelog

- **2026-07-03 (current run)** — Verified against `https://opencode.ai/docs/skills/` (last updated 2026-07-03) and current `dev`-branch source for `packages/opencode/src/skill/{index,discovery}.ts`, `packages/opencode/src/tool/skill.ts`, `packages/opencode/src/config/{config,managed}.ts`, and `packages/opencode/src/effect/runtime-flags.ts`. Confirmed that both `~/.config/opencode/skills/` (plural, canonical) and `~/.config/opencode/skill/` (singular, backwards-compatible) are scanned via the `{skill,skills}/**/SKILL.md` glob. Observed 81 entries in the singular alias on this host; `~/.config/opencode/skills/` does not exist locally. Ran `opencode debug skill` on 1.17.13 and confirmed last-writer-wins precedence via the per-collision `WARN duplicate skill name` lines emitted across `~/.config/opencode/skill/`, `~/.claude/skills/`, `.opencode/skill/`, and `.claude/skills/`. Captured the URL index schema and the version-mismatch staging/rename behavior from `discovery.ts`. Captured the `OPENCODE_DISABLE_CLAUDE_CODE[_SKILLS|_PROMPT]` OR-pattern from `runtime-flags.ts`. Added per-OS records for `skills.paths[]` (configured dirs) and `skills.urls[]` (cache locations) and for the system managed config directories.
- **2026-07-02 (prior)** — First research entry.

## Sources

- [OpenCode Agent Skills documentation](https://opencode.ai/docs/skills/)
- [OpenCode CLI documentation](https://opencode.ai/docs/cli/)
- [OpenCode config documentation](https://opencode.ai/docs/config/)
- [OpenCode permissions documentation](https://opencode.ai/docs/permissions/)
- [OpenCode schema](https://opencode.ai/config.json)
- [OpenCode repository home](https://github.com/anomalyco/opencode)
- [OpenCode source: `packages/opencode/src/skill/index.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/skill/index.ts)
- [OpenCode source: `packages/opencode/src/skill/discovery.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/skill/discovery.ts)
- [OpenCode source: `packages/opencode/src/tool/skill.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/tool/skill.ts)
- [OpenCode source: `packages/opencode/src/config/config.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/config.ts)
- [OpenCode source: `packages/opencode/src/config/managed.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/config/managed.ts)
- [OpenCode source: `packages/opencode/src/effect/runtime-flags.ts`](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/effect/runtime-flags.ts)