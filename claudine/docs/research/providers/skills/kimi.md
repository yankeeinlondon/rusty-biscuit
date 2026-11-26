---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://www.kimi.com/code
docs: https://moonshotai.github.io/kimi-cli/en/
skills_docs: https://moonshotai.github.io/kimi-cli/en/customization/skills.html

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.kimi/skills/<skill-name>/SKILL.md
    notes: Kimi brand user skills directory. Highest-priority brand directory; participates in the brand group.
  - os: linux
    scope: user
    path: ~/.kimi/skills/<skill-name>/SKILL.md
    notes: Kimi brand user skills directory. Highest-priority brand directory; participates in the brand group.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kimi\\skills\\<skill-name>\\SKILL.md"
    notes: Kimi brand user skills directory. Highest-priority brand directory; participates in the brand group.
  - os: macos
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: Claude brand user skills directory; loaded as a brand-group fallback or alongside ~/.kimi/skills/ depending on merge_all_available_skills.
  - os: linux
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: Claude brand user skills directory; loaded as a brand-group fallback or alongside ~/.kimi/skills/ depending on merge_all_available_skills.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills\\<skill-name>\\SKILL.md"
    notes: Claude brand user skills directory; loaded as a brand-group fallback or alongside ~/.kimi/skills/ depending on merge_all_available_skills.
  - os: macos
    scope: user
    path: ~/.codex/skills/<skill-name>/SKILL.md
    notes: Codex brand user skills directory; lowest-priority brand user directory. Merged when merge_all_available_skills = true.
  - os: linux
    scope: user
    path: ~/.codex/skills/<skill-name>/SKILL.md
    notes: Codex brand user skills directory; lowest-priority brand user directory. Merged when merge_all_available_skills = true.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\skills\\<skill-name>\\SKILL.md"
    notes: Codex brand user skills directory; lowest-priority brand user directory. Merged when merge_all_available_skills = true.
  - os: macos
    scope: user
    path: ~/.config/agents/skills/<skill-name>/SKILL.md
    notes: Generic cross-tool user skills directory. Recommended canonical location; searched independently of the brand group.
  - os: linux
    scope: user
    path: ~/.config/agents/skills/<skill-name>/SKILL.md
    notes: Generic cross-tool user skills directory. Recommended canonical location; searched independently of the brand group.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\agents\\skills\\<skill-name>\\SKILL.md"
    notes: Generic cross-tool user skills directory. Recommended canonical location; searched independently of the brand group.
  - os: macos
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: Generic fallback user skills directory. Searched when ~/.config/agents/skills/ is absent.
  - os: linux
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: Generic fallback user skills directory. Searched when ~/.config/agents/skills/ is absent.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<skill-name>\\SKILL.md"
    notes: Generic fallback user skills directory. Searched when ~/.config/agents/skills/ is absent.
  - os: macos
    scope: repo
    path: .kimi/skills/<skill-name>/SKILL.md
    notes: Project-level Kimi brand skills. Resolved relative to project root (nearest .git ancestor of work directory, or work directory itself when not in a git repo).
  - os: linux
    scope: repo
    path: .kimi/skills/<skill-name>/SKILL.md
    notes: Project-level Kimi brand skills. Resolved relative to project root (nearest .git ancestor of work directory, or work directory itself when not in a git repo).
  - os: windows
    scope: repo
    path: ".kimi\\skills\\<skill-name>\\SKILL.md"
    notes: Project-level Kimi brand skills. Resolved relative to project root (nearest .git ancestor of work directory, or work directory itself when not in a git repo).
  - os: macos
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: Project-level Claude-compatible brand skills. Same .git-anchored project-root resolution.
  - os: linux
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: Project-level Claude-compatible brand skills. Same .git-anchored project-root resolution.
  - os: windows
    scope: repo
    path: ".claude\\skills\\<skill-name>\\SKILL.md"
    notes: Project-level Claude-compatible brand skills. Same .git-anchored project-root resolution.
  - os: macos
    scope: repo
    path: .codex/skills/<skill-name>/SKILL.md
    notes: Project-level Codex-compatible brand skills. Same .git-anchored project-root resolution.
  - os: linux
    scope: repo
    path: .codex/skills/<skill-name>/SKILL.md
    notes: Project-level Codex-compatible brand skills. Same .git-anchored project-root resolution.
  - os: windows
    scope: repo
    path: ".codex\\skills\\<skill-name>\\SKILL.md"
    notes: Project-level Codex-compatible brand skills. Same .git-anchored project-root resolution.
  - os: macos
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: Project-level generic cross-tool skills. Same .git-anchored project-root resolution.
  - os: linux
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: Project-level generic cross-tool skills. Same .git-anchored project-root resolution.
  - os: windows
    scope: repo
    path: ".agents\\skills\\<skill-name>\\SKILL.md"
    notes: Project-level generic cross-tool skills. Same .git-anchored project-root resolution.
  - os: macos
    scope: other
    path: <extra_skill_dirs>/<skill-name>/SKILL.md
    notes: Additive paths from extra_skill_dirs in ~/.kimi/config.toml. Tilde expands to $HOME; relative paths resolve against the .git-anchored project root. Non-existent entries are silently dropped.
  - os: linux
    scope: other
    path: <extra_skill_dirs>/<skill-name>/SKILL.md
    notes: Additive paths from extra_skill_dirs in ~/.kimi/config.toml. Tilde expands to $HOME; relative paths resolve against the .git-anchored project root. Non-existent entries are silently dropped.
  - os: windows
    scope: other
    path: "<extra_skill_dirs>\\<skill-name>\\SKILL.md"
    notes: Additive paths from extra_skill_dirs in ~/.kimi/config.toml. Tilde expands to $HOME; relative paths resolve against the .git-anchored project root. Non-existent entries are silently dropped.
  - os: macos
    scope: other
    path: <skills-dir>/<skill-name>/SKILL.md
    notes: Directories appended via the --skills-dir flag (repeatable). When set, replaces auto-discovered user and project directories; built-in skills still load when supported.
  - os: linux
    scope: other
    path: <skills-dir>/<skill-name>/SKILL.md
    notes: Directories appended via the --skills-dir flag (repeatable). When set, replaces auto-discovered user and project directories; built-in skills still load when supported.
  - os: windows
    scope: other
    path: "<skills-dir>\\<skill-name>\\SKILL.md"
    notes: Directories appended via the --skills-dir flag (repeatable). When set, replaces auto-discovered user and project directories; built-in skills still load when supported.
  - os: macos
    scope: other
    path: ~/.kimi/plugins/<plugin>/SKILL.md
    notes: Skills bundled under installed plugins. The plugin install root is always added as an "extra"-scoped root regardless of extra_skill_dirs.
  - os: linux
    scope: other
    path: ~/.kimi/plugins/<plugin>/SKILL.md
    notes: Skills bundled under installed plugins. The plugin install root is always added as an "extra"-scoped root regardless of extra_skill_dirs.
  - os: windows
    scope: other
    path: "%USERPROFILE%\\.kimi\\plugins\\<plugin>\\SKILL.md"
    notes: Skills bundled under installed plugins. The plugin install root is always added as an "extra"-scoped root regardless of extra_skill_dirs.
  - os: macos
    scope: system
    path: <python-site-packages>/kimi_cli/skills/<skill-name>/SKILL.md
    notes: Shipped with the CLI (kimi-cli-help, skill-creator). Lowest priority. Loaded only when the active KAOS backend is "local" or "acp".
  - os: linux
    scope: system
    path: <python-site-packages>/kimi_cli/skills/<skill-name>/SKILL.md
    notes: Shipped with the CLI (kimi-cli-help, skill-creator). Lowest priority. Loaded only when the active KAOS backend is "local" or "acp".
  - os: windows
    scope: system
    path: "<python-site-packages>\\kimi_cli\\skills\\<skill-name>\\SKILL.md"
    notes: Shipped with the CLI (kimi-cli-help, skill-creator). Lowest priority. Loaded only when the active KAOS backend is "local" or "acp".

format:
  file_names:
    - SKILL.md
    - "*.md (flat single-file skills placed directly in a skills directory; SKILL.md at the top of a skills dir is treated as a stray marker)"
  frontmatter: true
  required_fields: []
  optional_fields:
    - name
    - description
    - license
    - compatibility
    - metadata
    - type
  body_format: markdown
  notes: |
    Kimi Code CLI implements the Agent Skills open standard. No frontmatter field
    is strictly required. `name` defaults to the directory name (or the `.md`
    filename stem for flat skills), accepts 1–64 lowercase letters, numbers,
    and hyphens, and matches case-insensitively during discovery. `description`
    defaults to the first non-empty body line (truncated to 240 characters) and
    finally to the literal string "No description provided.". `type` selects
    "standard" (default) or "flow" (a flow skill that embeds a Mermaid or D2
    diagram). The body is Markdown; supporting `scripts/`, `references/`, and
    `assets/` directories are conventional. Kimi does not recognize Claude-Code-
    specific frontmatter extensions such as `allowed-tools`, `disable-model-
    invocation`, or `user-invocable`. A bare `SKILL.md` placed directly at the
    top of a skills directory is treated as a stray marker and is not loaded as
    a skill; flat skills must be named `<name>.md` where `<name>` becomes the
    default skill name.

discovery:
  mechanism: |
    Kimi Code CLI loads skills in a single layered scan at session startup via
    `resolve_skills_roots` (in `src/kimi_cli/skill/__init__.py`). Roots are
    resolved in this order; the first occurrence of any normalized skill name
    across the resolved list wins, so the priority order is the reverse of
    insertion order:

    1. **`--skills-dir` (CLI override, repeatable)** — when set, replaces the
       user and project auto-discovery blocks entirely. Built-in skills still
       load when supported.
    2. **Project brand group** — `.kimi/skills/`, `.claude/skills/`,
       `.codex/skills/` resolved against the .git-anchored project root. With
       `merge_all_available_skills = true` (default), all existing brand
       directories are included in `kimi → claude → codex` priority order; with
       `false`, only the first existing one is used.
    3. **Project generic group** — `.agents/skills/` at the project root. If
       `.config/agents/skills/` were ever added at the project level it would
       precede it (none is currently scanned; only `.agents/skills` is in the
       project-level candidate list).
    4. **User brand group** — `~/.kimi/skills/`, `~/.claude/skills/`,
       `~/.codex/skills/` with the same `merge_all_available_skills` semantics
       as the project group.
    5. **User generic group** — `~/.config/agents/skills/` (preferred),
       `~/.agents/skills/`. The generic group is **always** searched, even when
       empty, and its results are merged independently of the brand group.
       When a skill name exists in both groups, the brand-group version wins
       because the brand roots are inserted first in the resolved list.
    6. **Extra directories** — every entry in `extra_skill_dirs` from
       `~/.kimi/config.toml`. Tilde prefixes expand against `$HOME`; relative
       paths resolve against the project root; non-existent or unreadable
       entries are silently logged and skipped.
    7. **Plugin directory** — `~/.kimi/plugins/` (overridable via
       `KIMI_SHARE_DIR`) is always added as a single "extra"-scoped root.
    8. **Built-in** — `<python-site-packages>/kimi_cli/skills/` is appended last
       and is **only loaded when the active KAOS backend is `local` or `acp`**
       (i.e. not in hosted/multi-tenant contexts).

    Each resolved root is tagged with a `SkillScope` of `project`, `user`,
    `extra`, or `builtin`. `discover_skills_from_roots` walks the resolved
    roots in order, and for each root runs two passes: (1) subdirectory
    `<name>/SKILL.md` discovery, (2) flat `<name>.md` discovery. Subdirectory
    skills shadow flat skills of the same name in the same directory with a
    warning. Same-name skills across roots resolve by first occurrence in the
    ordered root list, so the priority order is `project > user > extra >
    builtin`.

    The system-prompt renderer (`format_skills_for_prompt`) groups skills by
    scope into `### Project` / `### User` / `### Extra` / `### Built-in`
    headings so the model can distinguish a project's own skills from
    user-level or bundled ones. Skills can be explicitly invoked with the
    `/skill:<name>` slash command (or `/flow:<name>` for flow skills).
  precedence: |
    Highest priority first (winner of name collision):

    1. `--skills-dir` (replaces user and project auto-discovery).
    2. Project brand group, in directory priority: `.kimi/skills` > `.claude/skills` > `.codex/skills`. With `merge_all_available_skills = true`, all existing brand directories contribute; with `false`, only the first existing one does.
    3. Project generic group: `.agents/skills`.
    4. User brand group, in directory priority: `~/.kimi/skills` > `~/.claude/skills` > `~/.codex/skills`. Same `merge_all_available_skills` semantics as the project brand group.
    5. User generic group: `~/.config/agents/skills` > `~/.agents/skills`. Always searched, even when empty.
    6. `extra_skill_dirs` config entries (additive).
    7. `~/.kimi/plugins` (always added as an "extra" root).
    8. Built-in skills at `<python-site-packages>/kimi_cli/skills/`. Loaded only when the active KAOS backend is `local` or `acp`.

    Brand-group roots are always inserted before generic-group roots at the
    same scope level, so a same-name skill in a brand directory shadows the
    generic-directory version. Within the brand group, the same-name priority
    order is `kimi > claude > codex`.
  enable_disable: |
    No per-skill enable or disable flag. All discovered skills are available to
    the model. Session-level control is via:

    - `--skills-dir PATH` (repeatable) — replaces user and project auto-
      discovery with the listed directories.
    - `extra_skill_dirs` in `~/.kimi/config.toml` — adds additional directories
      on top of the auto-discovered set.
    - `merge_all_available_skills = false` — restricts the brand group at
      every scope to the first existing directory in priority order.
    - The KAOS backend in use (built-in skills load only on `local`/`acp`).

    Kimi has no equivalent of Claude Code's `disable-model-invocation` or
    `user-invocable` frontmatter fields, and no `skillOverrides` mechanism.
  notes: |
    Skill paths are deliberately independent of `KIMI_SHARE_DIR`. `KIMI_SHARE_DIR`
    relocates configuration, sessions, logs, and the plugin install root
    (`$KIMI_SHARE_DIR/plugins`) but does **not** affect the brand or generic
    skill search paths. The `--skills-dir` flag and `extra_skill_dirs` config
    are the only documented ways to relocate skill discovery.

    The repo-level `find_project_root` walker anchors project-scope skill
    discovery to the nearest directory containing `.git`, falling back to the
    working directory itself when no `.git` marker is found; this is what
    allows launching from a monorepo subdirectory to still surface skills
    defined at the repository root.

    Symlinks, `..` segments, and trailing slashes are normalized before roots
    are inserted into the resolved list, so listing the same directory twice
    (or via a symlink to an already-discovered target) does not render the
    same skill twice in the system prompt.

    `OSError` from `is_dir` or `iterdir` on any candidate root is logged and
    skipped rather than aborting the entire discovery pass, so a permission
    failure on one `extra_skill_dirs` entry does not block the rest.

portability:
  portable: true
  non_portable_assets:
    - "Flow skills (`type: flow`) — Mermaid/D2 diagram execution and the `/flow:<name>` command are Kimi-specific."
    - "Scripts in `scripts/` — language, interpreter availability, and OS/shell assumptions vary by host."
    - "References to Kimi-specific tools (`Shell`, `StrReplaceFile`, `Agent`, `ReadFile`, `WriteFile`, `Glob`, `Grep`, `WebFetch`, ...) — schemas and tool names differ across providers."
    - "Project-root-relative paths and assumptions about `.git` ancestry — only Kimi walks up to the nearest `.git` to anchor repo-scope discovery."
    - "`extra_skill_dirs` config entries — Kimi-specific config key; Claude Code and Codex have no equivalent."
    - "`merge_all_available_skills` config — Kimi-specific toggle; other providers do not scan multiple brand directories."
    - "Kimi-specific frontmatter keys or `metadata` values — Kimi ignores Claude-Code extensions, so linking a Kimi skill that uses `allowed-tools`, `disable-model-invocation`, or `user-invocable` will not be portable."
    - "Built-in skills `kimi-cli-help` and `skill-creator` — Kimi-specific; should not be linked as user-level resources elsewhere."
  rewrite_needed: true
  notes: |
    The portable part of a Kimi skill is the Agent Skills standard frontmatter
    (`name`, `description`, `license`, `compatibility`, `metadata`) and the
    Markdown body. Because Kimi explicitly reads `.claude/skills/` and
    `.codex/skills/` at both user and project scope (with the same
    `SKILL.md` / flat `.md` contract), skills placed in those directories are
    expected to be cross-tool compatible. Skills placed in `~/.kimi/skills/`,
    `.kimi/skills/`, `~/.config/agents/skills/`, or `.agents/skills/` are also
    portable for the same reason, but the directory names signal a Kimi-side
    primary owner; the canonical cross-tool directory is
    `~/.config/agents/skills/` (user) and `.agents/skills/` (project).

    Kimi has no direct equivalent of Claude Code's `skillOverrides`, `disable-
    model-invocation`, `user-invocable`, managed skills, or plugin namespacing.
    Kimi plugins are a different feature from Claude Code plugins; a Kimi
    plugin's `SKILL.md` (if any) is loaded through the same `extra`-scoped
    path that user-config and `--skills-dir` entries use, not through a
    separate plugin manifest.

cli_params:
  - flag: --skills-dir PATH
    description: Append additional skills directories; repeatable. When set, replaces user/project auto-discovery. Built-in skills still load when supported.
    example: kimi --skills-dir /path/to/my-skills --skills-dir /path/to/more-skills
  - flag: --work-dir PATH / -w PATH
    description: Set the working directory, which determines the .git-anchored project root used for repo-level skill discovery.
    example: kimi -w /path/to/project
  - flag: --add-dir PATH
    description: Add additional workspace directories for file tools; does not change skill discovery roots.
    example: kimi --add-dir ../shared-lib
  - flag: --config-file PATH
    description: Load an alternative TOML/JSON config file; extra_skill_dirs and merge_all_available_skills are read from this file.
    example: kimi --config-file ./kimi.toml
  - flag: --config STRING
    description: Pass configuration inline as a TOML or JSON string. Mutually exclusive with --config-file.
    example: "kimi --config '{\"merge_all_available_skills\": false, \"extra_skill_dirs\": [\"./my-skills\"]}'"
  - flag: --agent NAME
    description: Use a built-in agent (`default` or `okabe`). Affects system-prompt generation and tool set, not skill discovery.
    example: kimi --agent okabe
  - flag: --agent-file PATH
    description: Load a custom agent YAML file. Mutually exclusive with --agent.
    example: kimi --agent-file ./my-agent.yaml
  - flag: --model NAME / -m NAME
    description: Specify the LLM model; overrides the config-file default. No effect on skill discovery.
    example: kimi -m kimi-for-coding

env_vars:
  - name: KIMI_SHARE_DIR
    effect: Relocates configuration, sessions, logs, credentials, and the plugin install root (default ~/.kimi). Does NOT affect Agent Skills search paths. Plugin-scope skill roots follow the share dir.
  - name: KIMI_CLI_NO_AUTO_UPDATE
    effect: When set to a truthy value (1/true/t/yes/y, case-insensitive), disables update checks and the blocking update gate. No effect on skill discovery.
  - name: KIMI_CLI_PASTE_CHAR_THRESHOLD
    effect: Character threshold for folding pasted text into a placeholder (default 1000). No effect on skill discovery.
  - name: KIMI_CLI_PASTE_LINE_THRESHOLD
    effect: Line threshold for folding pasted text into a placeholder (default 15). No effect on skill discovery.

changes:
  - "Split location records from `os: all` into separate macOS, Linux, and Windows entries per schema requirements."
  - "Documented the two-group discovery model (brand + generic) and the --skills-dir override that replaces user/project auto-discovery."
  - "Added the KAOS backend gate for built-in skills (built-ins load only when the active backend is `local` or `acp`)."
  - "Added the always-on plugin root (~/.kimi/plugins) as an extra-scoped discovery root."
  - "Added the two-pass discovery model (subdirectory then flat .md) and the subdirectory-shadows-flat tie-break with warning."
  - "Confirmed the description fallback chain (frontmatter → first body line capped at 240 chars → 'No description provided.') and the .git-anchored project-root resolution."
  - "Verified on this host: ~/.kimi/config.toml has merge_all_available_skills = false and extra_skill_dirs = []; ~/.kimi/skills/ does not exist; ~/.claude/skills/ contains 85+ skill entries (real directories plus symlinks to skill libraries). kimi CLI v0.14.0 installed locally."

requires_claudine_update: true
reason: |
  Claudine's linking module should model Kimi's layered two-group skill
  discovery: brand directories (`~/.kimi/skills/`, `~/.claude/skills/`,
  `~/.codex/skills/`) merged by priority plus generic directories
  (`~/.config/agents/skills/`, `~/.agents/skills/`) at both user and project
  scope; the `merge_all_available_skills` config toggling between
  merge-everything and first-match-only behavior; project roots resolved
  against the nearest `.git` ancestor (with the working directory as a
  fallback); flat `.md` skills; the `--skills-dir` flag replacing user/project
  auto-discovery while keeping built-ins; `extra_skill_dirs` additive paths;
  the always-on `~/.kimi/plugins` extra root; and Kimi-specific flow skills
  (`type: flow` with Mermaid/D2 diagrams). It should also record that Kimi
  reads Claude and Codex brand skill directories directly, making those paths
  high-value portable linking targets, and that `KIMI_SHARE_DIR` is *not* a
  skill-path knob.
---

# Kimi Code CLI Skills

## Overview

Kimi Code CLI (Moonshot AI's agentic coding CLI) supports [Agent Skills](https://agentskills.io/), an open format for adding specialized knowledge and workflows to AI agents. A skill is a directory containing a `SKILL.md` entry point with optional YAML frontmatter and Markdown instructions, or — in the flat form — a single `<name>.md` file placed directly in a skills directory. At session startup, Kimi discovers skills across multiple roots, deduplicates them by normalized name (first match wins), and injects their name, path, and description into the system prompt under `### Project` / `### User` / `### Extra` / `### Built-in` scope headings so the model can decide when to read a given `SKILL.md`. Users can also invoke a skill explicitly with `/skill:<name>`; flow skills additionally expose `/flow:<name>`.

Kimi distinguishes **skills** (knowledge guidance the AI reads) from **plugins** (executable tools declared via `plugin.json`). Skills are not plugins, and the same skill can be used independently of any plugin install. This document covers skills only.

## Locations

Skill roots are grouped into four scope buckets: `user`, `project`, `extra`, and `builtin`. The brand group and the generic group are searched independently; the brand group is always inserted before the generic group, so a same-name skill in a brand directory shadows the generic version. Within the brand group, the directory priority is `kimi > claude > codex`.

| Scope | macOS / Linux | Windows | Notes |
|---|---|---|---|
| User — Kimi brand | `~/.kimi/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.kimi\skills\<skill-name>\SKILL.md` | Highest-priority brand directory. |
| User — Claude brand | `~/.claude/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.claude\skills\<skill-name>\SKILL.md` | Loaded as brand-group fallback or alongside `~/.kimi/skills/` depending on `merge_all_available_skills`. |
| User — Codex brand | `~/.codex/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.codex\skills\<skill-name>\SKILL.md` | Lowest-priority brand directory. |
| User — generic (recommended) | `~/.config/agents/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.config\agents\skills\<skill-name>\SKILL.md` | Canonical cross-tool location. |
| User — generic fallback | `~/.agents/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.agents\skills\<skill-name>\SKILL.md` | Searched when `~/.config/agents/skills/` is absent. |
| Project — Kimi brand | `.kimi/skills/<skill-name>/SKILL.md` | `.kimi\skills\<skill-name>\SKILL.md` | Resolved against the `.git`-anchored project root. |
| Project — Claude brand | `.claude/skills/<skill-name>/SKILL.md` | `.claude\skills\<skill-name>\SKILL.md` | Same root resolution. |
| Project — Codex brand | `.codex/skills/<skill-name>/SKILL.md` | `.codex\skills\<skill-name>\SKILL.md` | Same root resolution. |
| Project — generic | `.agents/skills/<skill-name>/SKILL.md` | `.agents\skills\<skill-name>\SKILL.md` | Same root resolution. |
| Extra (config) | Paths from `extra_skill_dirs` | Paths from `extra_skill_dirs` | Additive; `~` expands to `$HOME`; relative entries resolve against the project root. |
| Extra (CLI) | Paths from `--skills-dir` | Paths from `--skills-dir` | Replaces user and project auto-discovery. Built-ins still load. |
| Extra (plugins) | `~/.kimi/plugins/<plugin>/SKILL.md` | `%USERPROFILE%\.kimi\plugins\<plugin>\SKILL.md` | Always added as a single extra-scoped root (follows `KIMI_SHARE_DIR`). |
| Built-in | `<python-site-packages>/kimi_cli/skills/<name>/SKILL.md` | `<python-site-packages>\kimi_cli\skills\<name>\SKILL.md` | Shipped with the CLI: `kimi-cli-help`, `skill-creator`. Loaded only when the active KAOS backend is `local` or `acp`. |

Project skill roots are anchored to the project root, defined as the nearest directory containing `.git` above the working directory; the working directory itself is used as the project root when no `.git` marker is found. Launching from a monorepo subdirectory therefore still surfaces skills defined at the repository root.

The user's home directory on Windows is `%USERPROFILE%`; the `~` token used in the docs and the `extra_skill_dirs` config is expanded against `$HOME` (`%USERPROFILE%` on Windows) at discovery time.

## File Format

A skill is either:

- A subdirectory with a `SKILL.md` entry point (canonical layout):

  ```text
  my-skill/
  ├── SKILL.md          # Required metadata + instructions
  ├── scripts/          # Optional executable scripts
  ├── references/       # Optional reference documents
  └── assets/           # Optional other resources
  ```

- A flat `.md` file placed directly in a skills directory; its `name` defaults to the filename without the `.md` extension, and `description` follows the same three-step fallback as subdirectory skills.

  ```text
  ~/my-skills-collection/
  ├── demo-ui-components.md    # flat: name = "demo-ui-components"
  └── deploy/                   # subdirectory: name = "deploy"
      └── SKILL.md
  ```

  A bare `SKILL.md` placed directly at the top of a skills directory is treated as a stray marker and is not loaded as a skill. If a flat `<name>.md` and a subdirectory `<name>/SKILL.md` share the same name in the same directory, the subdirectory wins and a warning is logged.

`SKILL.md` uses YAML frontmatter between `---` markers followed by Markdown content:

```markdown
---
name: code-style
description: My project's code style guidelines
---

## Code Style

- Use 4-space indentation
- Variable names use camelCase
- Function names use snake_case
- Every function needs a docstring
- Lines should not exceed 100 characters
```

Frontmatter fields recognized by Kimi:

| Field | Required | Description |
|---|---|---|
| `name` | No | 1–64 lowercase letters, numbers, hyphens. Defaults to the directory name (or the `.md` filename stem for flat skills). Matched case-insensitively during discovery. |
| `description` | No | 1–1024 characters. Falls back to the first non-empty body line (truncated to 240 characters) and finally to the literal string `"No description provided."`. |
| `license` | No | License name or file reference. |
| `compatibility` | No | Environment requirements, up to 500 characters. |
| `metadata` | No | Additional key-value attributes. |
| `type` | No | `standard` (default) or `flow` for flow skills. |

The body is Markdown. Kimi does not currently recognize Claude-Code-specific frontmatter extensions such as `allowed-tools`, `disallowed-tools`, `disable-model-invocation`, `user-invocable`, `context`, `agent`, `hooks`, `paths`, or shell-injection blocks.

### Flow skills

Flow skills embed an Agent Flow diagram and are invoked via `/flow:<name>`:

```markdown
---
name: code-review
description: Code review workflow
type: flow
---

```mermaid
flowchart TD
A([BEGIN]) --> B[Analyze code changes]
B --> C{Is quality acceptable?}
C -->|Yes| D[Generate report]
C -->|No| E[List issues]
E --> B
D --> F([END])
```
```

Flow diagrams must contain one `BEGIN` and one `END` node and may use Mermaid (preferred) or D2 syntax. Decision nodes require the agent to output `<choice>branch name</choice>` to select the next step. If a flow skill's diagram fails to parse, the loader falls back to `type: "standard"` and emits an error log; the skill remains usable as a regular skill.

## Discovery and Precedence

Kimi loads skills in a single layered scan at session startup via `resolve_skills_roots` in `src/kimi_cli/skill/__init__.py`. The first occurrence of any normalized skill name across the resolved root list wins, so the priority order is the reverse of insertion order.

```text
1. --skills-dir entries (CLI override, repeatable; replaces user/project discovery)
2. Project brand group   — .kimi/skills > .claude/skills > .codex/skills
3. Project generic group — .agents/skills
4. User brand group      — ~/.kimi/skills > ~/.claude/skills > ~/.codex/skills
5. User generic group    — ~/.config/agents/skills > ~/.agents/skills
6. extra_skill_dirs entries (additive, config-driven)
7. ~/.kimi/plugins       (always-on extra root, follows KIMI_SHARE_DIR)
8. Built-in skills       (only when KAOS backend is local or acp)
```

Within the brand group, two behaviors are supported:

- `merge_all_available_skills = true` (default since 1.39.0): every existing brand directory is included in `kimi > claude > codex` priority order.
- `merge_all_available_skills = false`: only the first existing brand directory is used; later brand directories are skipped.

The generic group is always searched independently of the brand group, even when its directory is empty (a regression fix in 1.30.0 prevents an empty `~/.config/agents/skills/` from hiding brand-group skills). When a skill name exists in both groups, the brand-group version wins because brand roots are inserted into the resolved list before generic roots.

For each resolved root, the loader runs two passes:

1. **Subdirectory pass** — entries of the form `<name>/SKILL.md` are loaded as subdirectory skills, with `<name>` as the default skill name.
2. **Flat pass** — entries of the form `<name>.md` (case-insensitive) are loaded as flat skills with `<name>` as the default skill name. A flat skill whose name collides with a subdirectory skill in the same directory is shadowed with a warning.

Discovered skill metadata is rendered into the system prompt under four scope headings, in priority order: `### Project`, `### User`, `### Extra`, `### Built-in`. Each entry lists the skill name, its path, and its description. Empty scope groups are omitted. This grouping lets the model distinguish a project-scope skill from a user-scope one when responding to prompts like "the skill in this project".

Skills are considered automatically by the model during conversation. Users can also load a skill explicitly with `/skill:<name> [<extra prompt>]`; flow skills accept `/flow:<name>` to execute the flow from `BEGIN` to `END`.

There is no per-skill enable/disable flag. Session-level control is via:

- `--skills-dir` to replace user and project auto-discovery with explicit directories.
- `extra_skill_dirs` in config to add directories.
- `merge_all_available_skills = false` to restrict the brand group to the highest-priority existing directory.
- The active KAOS backend (built-ins load only on `local`/`acp`).

There is no equivalent of Claude Code's `disable-model-invocation` or `user-invocable` frontmatter, and no `skillOverrides` mechanism. Errors during discovery (failed `is_dir`, permission denials, missing `SKILL.md`) are logged and skipped rather than aborting the whole scan.

## Portability

The portable parts of a Kimi skill are the Agent Skills standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`) and the Markdown body. Because Kimi explicitly reads `.claude/skills/` and `.codex/skills/` at both user and project scope with the same `SKILL.md` and flat-`.md` contracts, skills placed in those directories are expected to be cross-tool compatible. The canonical cross-tool directory is `~/.config/agents/skills/` (user) and `.agents/skills/` (project); the `kimi`-brand directories are owned primarily by Kimi but their content is structurally portable.

Assets that need rewriting or host gating when moving to another provider:

- **Flow skills** (`type: flow`) — the Mermaid/D2 execution engine and `/flow:<name>` command are Kimi-specific.
- **`scripts/` files** — depend on installed interpreters, OS, and shell.
- **Tool-name references** — Kimi tools such as `Shell`, `StrReplaceFile`, `Agent`, `ReadFile`, `WriteFile`, `Glob`, `Grep`, `WebFetch`, etc. have different schemas or may not exist in other providers.
- **Project-root-relative paths** — tied to Kimi's `.git`-root discovery.
- **`extra_skill_dirs` config entries** — Kimi-specific config key; Claude Code and Codex have no equivalent.
- **`merge_all_available_skills` config** — Kimi-specific toggle; other providers do not scan multiple brand directories.
- **Kimi-specific frontmatter keys or `metadata` values** — Kimi ignores Claude-Code extensions, so linking a Kimi skill that uses `allowed-tools`, `disable-model-invocation`, or `user-invocable` will not be portable.

Kimi has no direct equivalent of Claude Code's `skillOverrides`, `disable-model-invocation`, `user-invocable`, managed skills, or plugin namespacing. Kimi plugins are a distinct feature from Claude Code plugins; a Kimi plugin's `SKILL.md` (if any) is loaded through the same `extra`-scoped path that user-config and `--skills-dir` entries use, not through a separate plugin manifest. The built-in `kimi-cli-help` and `skill-creator` skills are Kimi-specific and should not be linked as user-level resources elsewhere.

## Claudine Linking Notes

For Claudine's cross-provider resource linking:

- Treat Kimi-brand, Claude-brand, Codex-brand, and generic skill directories at both user and project scopes as linkable locations. Because Kimi merges `.kimi/skills/`, `.claude/skills/`, and `.codex/skills/`, a skill placed in `.claude/skills/` is usable by both Claude Code and Kimi. The canonical cross-tool location is `~/.config/agents/skills/` (user) and `.agents/skills/` (project).
- Classify standard Agent Skills frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`) and the Markdown body as portable.
- Flag flow skills (`type: flow`) and any `scripts/` dependencies, Kimi-specific tool-name references, and `extra_skill_dirs` or `merge_all_available_skills` config knobs as non-portable / requiring rewrite.
- Account for `merge_all_available_skills` when deciding which brand directories are effective. When `false`, only the highest-priority existing brand directory contributes; when `true`, all existing brand directories are merged in priority order.
- Account for the `.git`-anchored project root: a Kimi `.kimi/skills/` defined at the repository root is picked up from any subdirectory, so a project-scope link should target the repo-root path rather than a per-package copy.
- Recognize flat `.md` skills as first-class skill entries whose `name` derives from the filename stem; treat the absence of `name` in the frontmatter as a filename-based default.
- Recognize that the always-on `~/.kimi/plugins` extra root is a Kimi-specific extension; do not link Kimi plugin directories as portable user-scope resources elsewhere.
- Recognize that `KIMI_SHARE_DIR` does not relocate skill search paths; the brand and generic user directories are always derived from `$HOME` regardless of the share dir.
- Recognize that Kimi has no per-skill enable/disable frontmatter, so link-time gating is the only way to suppress a Kimi skill in a target provider.

## Changelog

- **2026-07-03** — Split location records from `os: all` into per-OS records (macOS, Linux, Windows) to satisfy the schema contract. Documented the two-group discovery model (brand + generic) and the always-on `~/.kimi/plugins` extra root. Added the KAOS backend gate for built-in skills (loaded only on `local`/`acp`). Added the two-pass discovery model (subdirectory then flat `.md`) and the subdirectory-shadows-flat tie-break. Verified the description fallback chain (frontmatter → first body line capped at 240 chars → `"No description provided."`) and the `.git`-anchored project-root resolution. Verified locally: `~/.kimi/config.toml` has `merge_all_available_skills = false` and `extra_skill_dirs = []`; `~/.kimi/skills/` does not exist; `~/.claude/skills/` contains 85+ skill entries (real directories plus symlinks to external skill libraries); kimi CLI v0.14.0 installed.

## Sources

- [Kimi Code CLI — Agent Skills](https://moonshotai.github.io/kimi-cli/en/customization/skills.html)
- [Kimi Code CLI — `kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.html)
- [Kimi Code CLI — Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html)
- [Kimi Code CLI — Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.html)
- [Kimi Code CLI — Data Locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Kimi Code CLI — Slash Commands](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html)
- [Kimi Code CLI — Plugins (Beta)](https://moonshotai.github.io/kimi-cli/en/customization/plugins.html)
- [Kimi Code CLI — Changelog](https://moonshotai.github.io/kimi-cli/en/release-notes/changelog.html)
- [Kimi Code CLI GitHub — `src/kimi_cli/skill/__init__.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/skill/__init__.py)
- [Kimi Code CLI GitHub — `src/kimi_cli/skill/flow/__init__.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/skill/flow/__init__.py)
- [Kimi Code CLI GitHub — `src/kimi_cli/plugin/manager.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/plugin/manager.py)
- [Kimi Code CLI GitHub — `src/kimi_cli/utils/path.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/utils/path.py)
- [Kimi Code CLI GitHub — `tests/core/test_skill.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/tests/core/test_skill.py)
- [Kimi Code CLI GitHub — `klips/klip-8-config-and-skills-layout.md`](https://github.com/MoonshotAI/kimi-cli/blob/main/klips/klip-8-config-and-skills-layout.md)
- [Kimi Code CLI GitHub — built-in `kimi-cli-help/SKILL.md`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/skills/kimi-cli-help/SKILL.md)
- [Kimi Code CLI GitHub — built-in `skill-creator/SKILL.md`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/skills/skill-creator/SKILL.md)
- [Agent Skills open specification](https://agentskills.io/specification)
- [Kimi Code homepage](https://www.kimi.com/code)
- [Kimi Code CLI GitHub repository](https://github.com/MoonshotAI/kimi-cli)
