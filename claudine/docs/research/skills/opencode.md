---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default

homepage: https://opencode.ai/
docs: https://opencode.ai/docs/
skills_docs: https://opencode.ai/docs/skills/

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.config/opencode/skills/<name>/SKILL.md
    notes: Native global OpenCode skill location documented as global config; current source also scans singular `skill/`.
  - os: linux
    scope: user
    path: ~/.config/opencode/skills/<name>/SKILL.md
    notes: Native global OpenCode skill location documented as global config; current source also scans singular `skill/`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\skills\\<name>\\SKILL.md"
    notes: Windows form of the documented global config path; current source uses the XDG config root and `OPENCODE_CONFIG_DIR` can replace it.
  - os: macos
    scope: user
    path: ~/.config/opencode/skill/<name>/SKILL.md
    notes: Undocumented but implemented native global alias via `{skill,skills}/**/SKILL.md`.
  - os: linux
    scope: user
    path: ~/.config/opencode/skill/<name>/SKILL.md
    notes: Undocumented but implemented native global alias via `{skill,skills}/**/SKILL.md`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\skill\\<name>\\SKILL.md"
    notes: Undocumented but implemented native global alias via `{skill,skills}/**/SKILL.md`.
  - os: macos
    scope: user
    path: ~/.claude/skills/<name>/SKILL.md
    notes: Claude-compatible global external skill path, skipped by `OPENCODE_DISABLE_EXTERNAL_SKILLS` or `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`.
  - os: linux
    scope: user
    path: ~/.claude/skills/<name>/SKILL.md
    notes: Claude-compatible global external skill path, skipped by `OPENCODE_DISABLE_EXTERNAL_SKILLS` or `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills\\<name>\\SKILL.md"
    notes: Claude-compatible global external skill path, skipped by `OPENCODE_DISABLE_EXTERNAL_SKILLS` or `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`.
  - os: macos
    scope: user
    path: ~/.agents/skills/<name>/SKILL.md
    notes: Agent-compatible global external skill path, skipped by `OPENCODE_DISABLE_EXTERNAL_SKILLS`.
  - os: linux
    scope: user
    path: ~/.agents/skills/<name>/SKILL.md
    notes: Agent-compatible global external skill path, skipped by `OPENCODE_DISABLE_EXTERNAL_SKILLS`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<name>\\SKILL.md"
    notes: Agent-compatible global external skill path, skipped by `OPENCODE_DISABLE_EXTERNAL_SKILLS`.
  - os: macos
    scope: repo
    path: .opencode/skills/<name>/SKILL.md
    notes: Native project skill path; source scans every discovered `.opencode` config directory from CWD up to the worktree.
  - os: linux
    scope: repo
    path: .opencode/skills/<name>/SKILL.md
    notes: Native project skill path; source scans every discovered `.opencode` config directory from CWD up to the worktree.
  - os: windows
    scope: repo
    path: .opencode\\skills\\<name>\\SKILL.md
    notes: Native project skill path; source scans every discovered `.opencode` config directory from CWD up to the worktree.
  - os: macos
    scope: repo
    path: .opencode/skill/<name>/SKILL.md
    notes: Undocumented but implemented native project alias via `{skill,skills}/**/SKILL.md`.
  - os: linux
    scope: repo
    path: .opencode/skill/<name>/SKILL.md
    notes: Undocumented but implemented native project alias via `{skill,skills}/**/SKILL.md`.
  - os: windows
    scope: repo
    path: .opencode\\skill\\<name>\\SKILL.md
    notes: Undocumented but implemented native project alias via `{skill,skills}/**/SKILL.md`.
  - os: macos
    scope: repo
    path: .claude/skills/<name>/SKILL.md
    notes: Claude-compatible project external skill path, discovered while walking from CWD to the git worktree.
  - os: linux
    scope: repo
    path: .claude/skills/<name>/SKILL.md
    notes: Claude-compatible project external skill path, discovered while walking from CWD to the git worktree.
  - os: windows
    scope: repo
    path: .claude\\skills\\<name>\\SKILL.md
    notes: Claude-compatible project external skill path, discovered while walking from CWD to the git worktree.
  - os: macos
    scope: repo
    path: .agents/skills/<name>/SKILL.md
    notes: Agent-compatible project external skill path, discovered while walking from CWD to the git worktree.
  - os: linux
    scope: repo
    path: .agents/skills/<name>/SKILL.md
    notes: Agent-compatible project external skill path, discovered while walking from CWD to the git worktree.
  - os: windows
    scope: repo
    path: .agents\\skills\\<name>\\SKILL.md
    notes: Agent-compatible project external skill path, discovered while walking from CWD to the git worktree.
  - os: macos
    scope: other
    path: configured-path/SKILL.md
    notes: skills.paths entries in config are expanded relative to the launch directory unless absolute or home-prefixed.
  - os: linux
    scope: other
    path: configured-path/SKILL.md
    notes: skills.paths entries in config are expanded relative to the launch directory unless absolute or home-prefixed.
  - os: windows
    scope: other
    path: configured-path\\SKILL.md
    notes: skills.paths entries in config are expanded relative to the launch directory unless absolute or home-prefixed.
  - os: macos
    scope: system
    path: <built-in>
    notes: OpenCode registers the bundled `customize-opencode` skill before disk discovery; a disk skill with the same name can override it.
  - os: linux
    scope: system
    path: <built-in>
    notes: OpenCode registers the bundled `customize-opencode` skill before disk discovery; a disk skill with the same name can override it.
  - os: windows
    scope: system
    path: <built-in>
    notes: OpenCode registers the bundled `customize-opencode` skill before disk discovery; a disk skill with the same name can override it.
  - os: macos
    scope: extension
    path: <plugin-provided source>
    notes: V2 plugin APIs can add directory, URL, or embedded skill sources; the current CLI also has config-driven URL sources.
  - os: linux
    scope: extension
    path: <plugin-provided source>
    notes: V2 plugin APIs can add directory, URL, or embedded skill sources; the current CLI also has config-driven URL sources.
  - os: windows
    scope: extension
    path: <plugin-provided source>
    notes: V2 plugin APIs can add directory, URL, or embedded skill sources; the current CLI also has config-driven URL sources.

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
    Official docs require one skill directory per skill name, an uppercase `SKILL.md`, YAML frontmatter, `name`, and `description`.
    Docs say `name` must match the parent directory, be 1-64 lowercase alphanumeric characters with single hyphen separators, and `description` must be 1-1024 characters.
    Current OpenCode 1.17.13 source is more permissive: it accepts any string `name`, treats `description` as optional in the loader, ignores unknown frontmatter, and stores the Markdown body after frontmatter as `content`.
    Skills without a description load internally but are omitted from model-facing available-skill guidance.
    The skill tool returns the body plus the skill base directory and a sampled file list from sibling files under the same skill directory, excluding `SKILL.md`.

discovery:
  mechanism: |
    At instance startup OpenCode builds a skill registry. It scans global external directories (`~/.claude/skills`, `~/.agents/skills`), project external directories while walking from the current directory to the git worktree, native OpenCode config directories, configured `skills.paths[]`, configured `skills.urls[]`, and the built-in `customize-opencode` skill.
    Native OpenCode directories are scanned with `{skill,skills}/**/SKILL.md`; external directories are scanned with `skills/**/SKILL.md`; configured paths are scanned recursively with `**/SKILL.md`.
    URL sources fetch `<url>/index.json`, expect `skills: [{ name, files, version? }]`, download listed files under each skill name into the OpenCode cache, and require each entry to include `SKILL.md`.
    The model does not receive full skill bodies up front. It receives `<available_skills>` entries with name and description through system prompt guidance, then calls the native `skill` tool with `{ name }` to load content.
  precedence: |
    Duplicate skill names are last-writer-wins in the in-memory registry, with a warning. Built-in `customize-opencode` is registered first specifically so disk skills can override it.
    Source scan order in current source is external global (`~/.claude`, then `~/.agents`), external project walk results, native config directories, `skills.paths[]`, then `skills.urls[]`.
    Native config directory order follows OpenCode's config directory discovery and merge process; project config is skipped when `OPENCODE_DISABLE_PROJECT_CONFIG=1`.
    The implementation does not merge same-name skills. The final loaded item for a name replaces the earlier item.
  enable_disable: |
    Removing or renaming the skill directory disables that skill. Per-agent permissions can hide, deny, allow, or ask for skill loading by pattern.
    `tools.skill: false` on a custom agent or built-in agent disables the skill tool entirely for that agent; then the `<available_skills>` section is omitted.
    `OPENCODE_DISABLE_EXTERNAL_SKILLS=1` skips `.claude` and `.agents` external scans. `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS=1` skips only `.claude` external scans. `OPENCODE_DISABLE_CLAUDE_CODE=1` also disables Claude-compatible skill scans.
  notes: |
    There is no documented trust prompt specific to skill discovery. Skill execution is mediated by OpenCode's normal permission system: denied skill names are hidden from the agent and rejected; `ask` prompts before loading.
    The global `--pure` flag / `OPENCODE_PURE=1` disables external plugins, not disk skill discovery. It can indirectly affect plugin-contributed skill sources.

portability:
  portable: false
  non_portable_assets:
    - "OpenCode-only built-in `customize-opencode` skill"
    - "OpenCode `skills.paths[]` and `skills.urls[]` config wiring"
    - "URL skill indexes served as `<url>/index.json` with `skills[].files` and optional `version`"
    - "Permission rules under `permission.skill` and agent `tools.skill`"
    - "Provider-specific instructions that mention OpenCode's `skill` tool output, `opencode.json(c)`, `.opencode/`, plugins, MCP config, or permission names"
    - "Sibling files, scripts, references, and assets that depend on OpenCode's base-directory message or host-specific executables"
  rewrite_needed: true
  notes: |
    The Markdown body of a simple `SKILL.md` is mostly portable, and OpenCode intentionally reads Claude-compatible `.claude/skills` and agent-compatible `.agents/skills`.
    Claudine should still classify OpenCode skills as requiring review/rewrite rather than as-is portable because OpenCode's implemented metadata contract is not identical to Claude or Codex, descriptions are the runtime routing signal, and permissions/config/URL sources are provider-specific.
    A safe rewrite should preserve `name`, `description`, Markdown body, and same-directory assets; drop or map OpenCode-specific config and permission surfaces.

cli_params:
  - flag: --agent <agent>
    description: Selects the active agent; agent permissions and `tools.skill` determine whether skills are listed or loadable.
    example: opencode run --agent plan "review this"
  - flag: --dir <path>
    description: Sets the working directory for `run`, `attach`, or ACP contexts; affects repo-local skill discovery and relative `skills.paths[]`.
    example: opencode run --dir ./packages/api "use the repo skill"
  - flag: --pure
    description: Runs without external plugins. Does not disable normal disk skills, but can prevent plugin-contributed skill sources.
    example: opencode --pure run "explain config"
  - flag: opencode debug skill
    description: Lists all currently registered skills as JSON, including built-in and file-backed skills.
    example: opencode debug skill
  - flag: opencode agent create --permissions
    description: Creates an agent with an allow-list of tools; omitting `skill` denies the skill tool for that agent.
    example: opencode agent create --description "Review only" --mode primary --permissions read,grep,glob

env_vars:
  - name: OPENCODE_CONFIG_DIR
    effect: Replaces the global OpenCode config directory, which changes the native global `skill/` and `skills/` scan root.
  - name: OPENCODE_CONFIG
    effect: Loads an additional explicit config file; can add `skills.paths`, `skills.urls`, permissions, agents, or plugin settings.
  - name: OPENCODE_CONFIG_CONTENT
    effect: Injects inline JSON config as a final local-scope merge; can add skill sources or override permissions for the session.
  - name: OPENCODE_DISABLE_PROJECT_CONFIG
    effect: Skips local project config files, which can remove project `skills.paths`, `skills.urls`, permissions, and some `.opencode` config-directory discovery.
  - name: OPENCODE_DISABLE_EXTERNAL_SKILLS
    effect: Skips `.claude/skills` and `.agents/skills` scans at both global and project scopes.
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: Skips `.claude/skills` scans while leaving `.agents/skills` external scans enabled.
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: Broad Claude-compatibility disable flag; also disables `.claude/skills` scans.
  - name: OPENCODE_PURE
    effect: Disables external plugins; can indirectly remove plugin-provided skill sources but does not disable normal disk skills.
  - name: OPENCODE_PERMISSION
    effect: Merges inline JSON permissions into config; can deny, ask, or allow `skill` patterns.

changes: []

requires_claudine_update: true
reason: |
  Claudine should add OpenCode as a first-class Agent Skills target. The linker should recognize native `.opencode/skills` and implemented `.opencode/skill`, Claude-compatible `.claude/skills`, agent-compatible `.agents/skills`, configured `skills.paths`, remote `skills.urls`, and the permission/env gates above.
  Generated metadata should classify OpenCode resources as rewrite-needed because OpenCode has provider-specific config, URL indexes, permission rules, and a built-in skill.
---

# OpenCode CLI Agent Skills

## Overview

OpenCode CLI has a first-class Agent Skills implementation. Official docs describe Agent Skills as reusable behavior packaged as `SKILL.md` definitions discovered from a repository or home directory and loaded on demand through OpenCode's native `skill` tool.

The runtime model is two-stage. At startup, OpenCode scans skill locations and builds a registry keyed by frontmatter `name`. During prompt assembly, the active agent receives an `<available_skills>` list containing names and descriptions that survive permission filtering. The agent loads the body only by calling the `skill` tool with the selected name. The tool then returns the Markdown body, the skill base directory, and a sampled list of sibling files under the skill directory.

Current source and tests were inspected from OpenCode `1.17.13`, matching the locally installed `opencode --version` output. Local inspection in this non-interactive session found `HOME=/Users/ken/.claudine`, `~/.config/opencode/opencode.jsonc` containing only the schema reference, no `~/.config/opencode/skill(s)/**/SKILL.md`, and no local `~/.agents/skills/**/SKILL.md`. Running `opencode debug skill` returned only the built-in `customize-opencode` skill.

## Locations

OpenCode recognizes native OpenCode skill directories, Claude-compatible skill directories, agent-compatible skill directories, configured directory sources, URL sources, and a built-in skill.

| Scope | macOS / Linux | Windows | Notes |
|---|---|---|---|
| User native | `~/.config/opencode/skills/<name>/SKILL.md` | `%USERPROFILE%\.config\opencode\skills\<name>\SKILL.md` | Documented global config skill path. `OPENCODE_CONFIG_DIR` replaces the config root. |
| User native alias | `~/.config/opencode/skill/<name>/SKILL.md` | `%USERPROFILE%\.config\opencode\skill\<name>\SKILL.md` | Implemented by source via `{skill,skills}/**/SKILL.md`; not highlighted in the public docs. |
| User Claude-compatible | `~/.claude/skills/<name>/SKILL.md` | `%USERPROFILE%\.claude\skills\<name>\SKILL.md` | External scan; can be disabled by external-skill env vars. |
| User agent-compatible | `~/.agents/skills/<name>/SKILL.md` | `%USERPROFILE%\.agents\skills\<name>\SKILL.md` | External scan; can be disabled by `OPENCODE_DISABLE_EXTERNAL_SKILLS`. |
| Repo native | `.opencode/skills/<name>/SKILL.md` | `.opencode\skills\<name>\SKILL.md` | Project config skill path discovered along the current directory to worktree path. |
| Repo native alias | `.opencode/skill/<name>/SKILL.md` | `.opencode\skill\<name>\SKILL.md` | Implemented alias. |
| Repo Claude-compatible | `.claude/skills/<name>/SKILL.md` | `.claude\skills\<name>\SKILL.md` | Discovered while walking from CWD up to the git worktree. |
| Repo agent-compatible | `.agents/skills/<name>/SKILL.md` | `.agents\skills\<name>\SKILL.md` | Discovered while walking from CWD up to the git worktree. |
| Configured directories | `<configured-path>/**/SKILL.md` | `<configured-path>\**\SKILL.md` | From `skills.paths[]` in config. Relative paths resolve against the launch directory; `~/` expands to home. |
| Configured URLs | `<url>/index.json` and listed files | `<url>/index.json` and listed files | From `skills.urls[]` in config. Files are cached under OpenCode's cache directory. |
| Built-in | `<built-in>` | `<built-in>` | `customize-opencode`, registered before disk discovery so disk can override the same name. |
| Plugin / extension | directory, URL, or embedded source | directory, URL, or embedded source | V2 plugin skill hooks can contribute sources; `--pure` / `OPENCODE_PURE` disables external plugins. |

OpenCode's source uses `xdg-basedir` for config/cache/data/state roots and `os.homedir()` for `~` external paths. The public skills docs use `~/.config/opencode` for global config, so Claudine should render that canonical path in user-facing linking output unless `OPENCODE_CONFIG_DIR` is explicitly set.

## File Format

The documented on-disk shape is one directory per skill:

```text
<name>/
└── SKILL.md
```

`SKILL.md` is Markdown with YAML frontmatter. Public docs say the recognized frontmatter keys are:

| Key | Required | Notes |
|---|---:|---|
| `name` | Yes | Docs require 1-64 characters, lowercase alphanumeric with single hyphen separators, no leading/trailing/consecutive hyphens, and an exact match with the containing directory name. |
| `description` | Yes | Docs require 1-1024 characters and recommend enough specificity for routing. |
| `license` | No | Recognized by documentation. |
| `compatibility` | No | Recognized by documentation. |
| `metadata` | No | Documented as a string-to-string map. |

Current implementation is more permissive than the docs. The loader accepts a parsed Markdown file when frontmatter has `name: string` and optional `description: string`; it ignores unknown fields, does not enforce the documented name regex, and does not enforce that `name` matches the parent directory. Tests confirm a skill without a description is discovered internally, but `Skill.fmt()` returns "No skills are currently available" when every skill lacks `description`, so such skills are not exposed in model-facing guidance.

The skill tool treats sibling files as part of the skill package. When a skill is loaded, it uses the directory containing `SKILL.md` as the base directory, tells the model that relative paths such as `scripts/` or `reference/` are relative to that base, and samples up to 10 non-`SKILL.md` files under the directory. Claudine should therefore treat same-directory assets as associated resources, even though `SKILL.md` is the only required file.

Configured URL sources use a separate index format:

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

Each listed remote skill must include `SKILL.md`. OpenCode downloads listed files under the skill name into cache. If `version` changes, it refreshes atomically only when the new download includes `SKILL.md`; otherwise it keeps the prior cached copy.

## Discovery and Precedence

Discovery starts from the active instance's directory and worktree. The current source does the following:

1. Register the built-in `customize-opencode` skill.
2. Unless `OPENCODE_DISABLE_EXTERNAL_SKILLS` is true, scan global `~/.claude` and `~/.agents` external directories. `~/.claude` is skipped when `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` or `OPENCODE_DISABLE_CLAUDE_CODE` is true.
3. Unless external skills are disabled, walk up from the current directory to the git worktree and scan matching `.claude/skills/**/SKILL.md` and `.agents/skills/**/SKILL.md` along the way.
4. Scan OpenCode config directories for `{skill,skills}/**/SKILL.md`.
5. Load `skills.paths[]` from the merged OpenCode config and scan each directory recursively for `**/SKILL.md`.
6. Load `skills.urls[]`, fetch each remote index, cache listed files, and scan the resulting cache directories.

Duplicate names are not merged. The registry is a map from `name` to one skill, and later additions replace earlier entries while logging a duplicate warning. Because the built-in skill is inserted before disk discovery, a file-backed `customize-opencode` skill can shadow the built-in one.

Permissions are applied when building model-facing guidance and when the tool loads a skill. Top-level config can contain pattern-based `permission.skill` entries such as `"*": "allow"`, `"internal-*": "deny"`, or `"experimental-*": "ask"`. Agent-level permission overrides can change that for a specific agent. `deny` hides matching skills and rejects access; `ask` prompts before loading; `allow` loads immediately. `tools.skill: false` disables the skill tool for an agent and omits the entire `<available_skills>` section.

The public CLI docs expose `--agent`, `--dir`, and global `--pure`. `--agent` matters because selected-agent permissions decide skill visibility. `--dir` matters for repo-local discovery and relative `skills.paths[]`. `--pure` disables external plugins, not ordinary disk skills, but it can remove plugin-contributed skill sources. `opencode debug skill` is an implementation-facing diagnostic command that lists the current registry as JSON.

Relevant environment variables are:

| Variable | Effect |
|---|---|
| `OPENCODE_CONFIG_DIR` | Replaces the global config directory and therefore the native global skill scan root. |
| `OPENCODE_CONFIG` | Loads an additional config file that can add skill sources or permissions. |
| `OPENCODE_CONFIG_CONTENT` | Loads inline JSON config as a final local-scope merge. |
| `OPENCODE_DISABLE_PROJECT_CONFIG` | Skips project config files and their skill source settings. |
| `OPENCODE_DISABLE_EXTERNAL_SKILLS` | Skips `.claude/skills` and `.agents/skills` scans. |
| `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS` | Skips only `.claude/skills` scans. |
| `OPENCODE_DISABLE_CLAUDE_CODE` | Broadly disables Claude-compatible prompt/skill surfaces, including `.claude/skills`. |
| `OPENCODE_PURE` | Disables external plugins; can indirectly remove plugin-provided skill sources. |
| `OPENCODE_PERMISSION` | Merges inline JSON permissions, including `skill` rules. |

No separate trust gate specific to Agent Skills was found in the official docs or inspected source. Skill loading is controlled by discovery roots, config/env gates, selected-agent tool availability, and permissions.

## Portability

OpenCode is intentionally compatible with common `SKILL.md` layouts: it reads `.claude/skills`, `.agents/skills`, and native `.opencode/skill(s)`. A minimal skill with `name`, `description`, and provider-neutral Markdown can be copied or linked into another provider's expected directory and likely retain most semantics.

Claudine should still mark OpenCode skills as `rewrite_needed` for provider-to-provider linking. The content may mention the OpenCode `skill` tool, `opencode.json(c)`, `.opencode/`, OpenCode permissions, plugins, or MCP config. OpenCode's URL index format and `skills.paths[]` / `skills.urls[]` config are not portable skill artifacts. Agent permissions and `tools.skill` settings are OpenCode-specific policy, not skill metadata. The built-in `customize-opencode` skill is OpenCode-specific and should not be exported as a generic shared skill unless the target provider is also being taught to edit OpenCode configuration.

Assets are conditionally portable. Same-directory Markdown references, scripts, and media should be kept with the skill directory, but Claudine should flag scripts and host-dependent references for review. The OpenCode tool tells the model the base directory and sampled file list at load time; other providers may not provide the same runtime hint.

## Claudine Linking Notes

Claudine should scan and link the following OpenCode targets:

- Native user and repo targets: `skill/` and `skills/` under the effective OpenCode config directories.
- Compatibility targets: `.claude/skills` and `.agents/skills` at user and repo scopes.
- Configured sources: `skills.paths[]` as directory roots and `skills.urls[]` as remote catalogs, with the remote catalog classified as provider-specific metadata rather than a plain skill directory.
- Extension/plugin sources when provider metadata exposes them; do not assume `--pure` sessions can see plugin-provided skills.

The linker should preserve `SKILL.md` plus sibling assets as one package. It should not rely on public-doc-only validation when reading OpenCode sources: current OpenCode accepts a description-less skill internally and does not enforce name/directory matching in source. For outbound links, however, Claudine should produce the documented stricter form because that is what OpenCode tells users to create and what model guidance needs.

The generated portability metadata should classify OpenCode as `first_class`, `portable: false`, and `rewrite_needed: true`. It should explicitly record the env gates `OPENCODE_DISABLE_EXTERNAL_SKILLS`, `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`, `OPENCODE_DISABLE_CLAUDE_CODE`, `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG`, `OPENCODE_CONFIG_CONTENT`, `OPENCODE_DISABLE_PROJECT_CONFIG`, `OPENCODE_PURE`, and `OPENCODE_PERMISSION`.

## Sources

- [OpenCode Agent Skills documentation](https://opencode.ai/docs/skills/)
- [OpenCode CLI documentation](https://opencode.ai/docs/cli/)
- [OpenCode config documentation](https://opencode.ai/docs/config/)
- [OpenCode source: `packages/opencode/src/skill/index.ts`](https://github.com/anomalyco/opencode/blob/main/packages/opencode/src/skill/index.ts)
- [OpenCode source: `packages/opencode/src/tool/skill.ts`](https://github.com/anomalyco/opencode/blob/main/packages/opencode/src/tool/skill.ts)
- [OpenCode source: `packages/opencode/src/skill/discovery.ts`](https://github.com/anomalyco/opencode/blob/main/packages/opencode/src/skill/discovery.ts)
- [OpenCode source: `packages/opencode/src/effect/runtime-flags.ts`](https://github.com/anomalyco/opencode/blob/main/packages/opencode/src/effect/runtime-flags.ts)
- [OpenCode source: `packages/opencode/src/config/config.ts`](https://github.com/anomalyco/opencode/blob/main/packages/opencode/src/config/config.ts)
- [OpenCode source: `packages/core/src/global.ts`](https://github.com/anomalyco/opencode/blob/main/packages/core/src/global.ts)
- [OpenCode tests: `packages/opencode/test/skill/skill.test.ts`](https://github.com/anomalyco/opencode/blob/main/packages/opencode/test/skill/skill.test.ts)
- [OpenCode tests: `packages/opencode/test/skill/discovery.test.ts`](https://github.com/anomalyco/opencode/blob/main/packages/opencode/test/skill/discovery.test.ts)
