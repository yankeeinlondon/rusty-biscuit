---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://block.github.io/goose/
docs: https://goose-docs.ai/docs/
skills_docs: https://goose-docs.ai/docs/guides/context-engineering/using-skills

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: |
      Canonical global skills directory. Source-of-truth for the user tier
      (`global_skills_dir()`). Observed on this host: find-skills/SKILL.md
      tracked by ~/.agents/.skill-lock.json.
  - os: linux
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: |
      Canonical global skills directory (`global_skills_dir()`,
      `dirs::home_dir().join(".agents").join("skills")`).
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<skill-name>\\SKILL.md"
    notes: |
      Canonical global skills directory (`global_skills_dir()`). Resolves via
      `dirs::home_dir()` then `.agents/skills`.
  - os: macos
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: |
      Canonical project-tier location. Loaded with `global=false` and scanned
      first in `all_skill_dirs()` so it shadows user/global skills with the
      same `name`.
  - os: linux
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: Canonical project-tier location, scanned before global dirs.
  - os: windows
    scope: repo
    path: ".agents\\skills\\<skill-name>\\SKILL.md"
    notes: Canonical project-tier location, scanned before global dirs.
  - os: macos
    scope: repo
    path: .goose/skills/<skill-name>/SKILL.md
    notes: |
      Backward-compatible project location (older goose project layout).
      Scanned after `.agents/skills/` in the project tier.
  - os: linux
    scope: repo
    path: .goose/skills/<skill-name>/SKILL.md
    notes: Backward-compatible project location, scanned after `.agents/skills/`.
  - os: windows
    scope: repo
    path: ".goose\\skills\\<skill-name>\\SKILL.md"
    notes: Backward-compatible project location, scanned after `.agents/skills/`.
  - os: macos
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: |
      Claude-Code backward-compatibility project location. Scanned after
      `.agents/skills/` and `.goose/skills/` in the project tier.
  - os: linux
    scope: repo
    path: .claude/skills/<skill-name>/SKILL.md
    notes: Claude-Code backward-compatibility project location.
  - os: windows
    scope: repo
    path: ".claude\\skills\\<skill-name>\\SKILL.md"
    notes: Claude-Code backward-compatibility project location.
  - os: macos
    scope: user
    path: ~/.config/goose/skills/<skill-name>/SKILL.md
    notes: |
      `Paths::config_dir().join("skills")` (etcetera `config_dir()`,
      `~/.config/goose/`). Loaded after `~/.agents/skills/` in the global tier.
      Backward-compatible platform-specific config location.
  - os: linux
    scope: user
    path: ~/.config/goose/skills/<skill-name>/SKILL.md
    notes: |
      `Paths::config_dir().join("skills")`. etcetera XDG strategy yields
      `~/.config/goose/` on Linux.
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\skills\\<skill-name>\\SKILL.md"
    notes: |
      `Paths::config_dir().join("skills")` under `%APPDATA%\Block\goose\config\`.
      Not observed on this host.
  - os: macos
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: |
      Claude-Code backward-compatibility global location. Observed on this
      host as a symlink-rich directory but is not a goose-native location.
      Scanned after `~/.agents/skills/` and `~/.config/goose/skills/`.
  - os: linux
    scope: user
    path: ~/.claude/skills/<skill-name>/SKILL.md
    notes: Claude-Code backward-compatibility global location.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills\\<skill-name>\\SKILL.md"
    notes: Claude-Code backward-compatibility global location.
  - os: macos
    scope: user
    path: ~/.config/agents/skills/<skill-name>/SKILL.md
    notes: |
      Optional XDG-style global location (`dirs::home_dir().join(".config").join("agents").join("skills")`).
      Not in the documented canonical list but present in `all_skill_dirs()`.
      Not observed on this host.
  - os: linux
    scope: user
    path: ~/.config/agents/skills/<skill-name>/SKILL.md
    notes: |
      Optional XDG-style global location; present in `all_skill_dirs()` but
      not the recommended canonical path.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\agents\\skills\\<skill-name>\\SKILL.md"
    notes: Optional XDG-style global location.
  - os: macos
    scope: extension
    path: ~/.agents/plugins/<plugin-name>/skills/<skill-name>/SKILL.md
    notes: |
      Open-Plugin user-scope skill bundles (Gemini extensions use a similar
      layout). Discovered via `installed_plugin_skill_dirs()` →
      `plugin_install_dir()` (= `Paths::plugins_dir()` =
      `dirs::home_dir().join(".agents").join("plugins")`). Names are
      namespaced as `<plugin>:<skill>` for Open Plugins; Gemini-extension
      skills keep the unprefixed `SKILL.md` `name`.
  - os: linux
    scope: extension
    path: ~/.agents/plugins/<plugin-name>/skills/<skill-name>/SKILL.md
    notes: Open-Plugin and Gemini-extension skill bundles.
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.agents\\plugins\\<plugin-name>\\skills\\<skill-name>\\SKILL.md"
    notes: Open-Plugin and Gemini-extension skill bundles.
  - os: macos
    scope: repo
    path: .agents/plugins/<plugin-name>/skills/<skill-name>/SKILL.md
    notes: |
      Project-scope plugin bundles. `project_plugin_dir()` =
      `<project>/.agents/plugins`. Plugin discovery must also pass the
      `disabledPlugins` / `enabledPlugins` settings check.
  - os: linux
    scope: repo
    path: .agents/plugins/<plugin-name>/skills/<skill-name>/SKILL.md
    notes: Project-scope plugin bundles.
  - os: windows
    scope: repo
    path: ".agents\\plugins\\<plugin-name>\\skills\\<skill-name>\\SKILL.md"
    notes: Project-scope plugin bundles.
  - os: macos
    scope: other
    path: bundled with `goose` binary (compile-time `include_dir!` of `crates/goose/src/skills/builtins/*.md`)
    notes: |
      Built-in skills shipped inside the goose binary
      (`builtin::get_all()`, `SourceType::BuiltinSkill`,
      `builtin://skills/<name>` synthetic path). Pre-approved; not
      user-writable; overridden by a same-`name` file-system skill because
      built-ins are appended last in `discover_skills()`.
  - os: linux
    scope: other
    path: bundled with `goose` binary (compile-time `include_dir!` of `crates/goose/src/skills/builtins/*.md`)
    notes: Built-in skills shipped inside the goose binary.
  - os: windows
    scope: other
    path: bundled with `goose` binary (compile-time `include_dir!` of `crates/goose/src/skills/builtins/*.md`)
    notes: Built-in skills shipped inside the goose binary.

format:
  file_names:
    - SKILL.md
  frontmatter: true
  required_fields:
    - name
    - description
  optional_fields:
    - metadata (agentskills.io free-form bag; carries argument-hint, arguments, license, compatibility, etc.)
    - argument-hint (read by the `/skills` slash-command UI from the `properties` map)
    - arguments (positional `$ARGUMENTS` substitution names, read from the `properties` map)
  body_format: markdown
  notes: |
    A skill is a directory whose entry point is `SKILL.md`. The body is
    CommonMark/GFM Markdown. The provider parses frontmatter with serde via
    `parse_skill_content` → `SkillFrontmatter`; the only top-level keys it
    reads are `name`, `description`, and `metadata` (a `HashMap<String, Value>`
    that the rest of the body treats as the agentskills.io
    `<https://agentskills.io/specification#frontmatter>` free-form bag — so
    `argument-hint`/`arguments`/`license`/`compatibility`/etc. survive as
    untyped metadata). Skill directories are walked recursively; VCS dirs
    (`.git`/`.hg`/`.svn`) are skipped; every non-`SKILL.md` file is recorded
    as a `supporting_file` and re-emitted as a `load_skill(name:
    "<skill>/<relative-path>")` hint when the skill is loaded. The
    `SkillsClient` MCP extension exposes `load_skill` /
    `list_skills` / etc. so the agent can fetch a supporting file by name.
    Skill `name` is validated to ≤ 64 chars, lowercase letters / digits /
    hyphens only, no leading or trailing hyphen, no `/` (which would
    collide with Open-Plugin namespacing).

discovery:
  mechanism: |
    At session start the goose binary's `discover_skills(working_dir)`
    walks a fixed list of filesystem roots (project first, then global,
    then plugin skills, then compile-time built-ins), parses each
    `SKILL.md` it finds, deduplicates by `name` in encounter order, and
    returns `SourceEntry` records. The Summon built-in MCP extension
    (v1.25.0+) exposes these records to the model via a `skills` tool and
    emits per-skill instructions into the system prompt. The agent then
    (a) auto-loads a skill when the user's request matches its
    `description`, (b) accepts an explicit "Use the X skill" prompt, or
    (c) lists/loads via the in-session `/skills` slash command or the
    `goose skills` CLI subcommand. The runtime tool that hydrates a
    skill's body and supporting files is `SkillsClient::load_skill`
    (extension name `summon`).
  precedence: |
    Source-of-truth order from `all_skill_dirs()` (project first, then
    global, then plugin skills), followed by built-ins appended last:

    1. `<working_dir>/.agents/skills`
    2. `<working_dir>/.goose/skills`
    3. `<working_dir>/.claude/skills`
    4. `~/.agents/skills`
    5. `Paths::config_dir()/skills` (macOS/Linux `~/.config/goose/skills`,
       Windows `%APPDATA%\Block\goose\config\skills`)
    6. `~/.claude/skills`
    7. `~/.config/agents/skills`
    8. Each installed plugin's `skills/` dir (Open-Plugins namespaced as
       `<plugin>:<skill>`; Gemini-extension skills keep the unprefixed
       `SKILL.md` name)
    9. Compile-time built-in skills (`builtin::get_all()` →
       `SourceType::BuiltinSkill`, synthetic path `builtin://skills/<name>`)

    Within this list the **first** entry wins on `name` collisions because
    `scan_skills_from_dir` checks a shared `seen` HashSet as it iterates.
    Therefore a project `.agents/skills/<x>` always overrides a global
    `~/.agents/skills/<x>`, which overrides a backward-compat
    `~/.claude/skills/<x>`, which overrides a built-in `<x>`. The same
    rule applies between Open-Plugin user-skills and project-skills
    (project scope is discovered before user scope in
    `discover_enabled_plugins`).
  enable_disable: |
    No per-skill disable toggle in the SKILL.md itself; activation is
    driven by the agent picking the skill from its context or by the user
    invoking it via `/skills <name>`. Whole-feature knobs:

    - **Summon extension off** (`goose configure → Toggle Extensions`,
      or `--with-builtin` omit) disables every skill — built-in, user,
      and project. The Summon extension is enabled by default for new
      users (v1.25.0+).
    - **`disabledPlugins`** in
      `~/.config/goose/settings.json` / `<project>/.config/goose/settings.json`
      (or `settings.local.json`) hides specific plugins from discovery,
      so plugin-bundled skills stop loading.
    - **`enabledPlugins`** allowlists specific plugins; any unlisted
      plugin is filtered out (and `local` scope overrides project scope,
      which overrides user scope).
    - **`plugins:`** map in `~/.config/goose/config.yaml` stores per-path
      `{enabled: bool}` entries; a `false` entry silently disables that
      plugin.
    - **`GOOSE_PATH_ROOT`** relocates the entire data/config/state tree
      (`config/`, `data/`, `state/`, `.agents/plugins/`,
      `.agents/agents/`, `.agents/`); skill discovery follows the
      relocated `Paths::config_dir()` and `Paths::plugins_dir()`.
    - There is **no documented way** to disable individual
      backward-compatibility project paths (`.goose/skills`,
      `.claude/skills`) — leaving stale directories there will continue
      to contribute skills, possibly shadowing newer locations.
  notes: |
    Discovery is per-session (no persistent watcher documented). Live
    editing requires a new session. The `/skills` slash command takes one
    or more names and loads each as a slash command (e.g.
    `/skills code-review edge-case-finder`). The `goose skills` CLI
    subcommand prints a token-counted table of installed skills with
    `name | description | description-tokens | content-tokens | location`
    columns (`crates/goose-cli/src/commands/skills.rs`).

portability:
  portable: true
  non_portable_assets:
    - "Scripts/files under a skill's supporting files — depend on host language runtimes and installed binaries"
    - "Path/project-layout assumptions embedded in `SKILL.md` prose"
    - "`argument-hint` / `arguments` frontmatter keys (Goose-specific surface via the `properties` map; ignored by other Agent-Skills consumers)"
    - "Plugin-bundled skills under `~/.agents/plugins/<plugin>/skills/` and the plugin namespacing convention `<plugin>:<skill>`"
    - "Compile-time built-in skills (`builtin://skills/<name>` synthetic paths, not backed by a file tree)"
    - "GOOSE_PATH_ROOT and the relocated `Paths::config_dir()` / `Paths::plugins_dir()` roots"
    - "Gemini-extension-style skill manifests (`gemini-extension.json`) and Open-Plugin `plugin.json` / `.plugin/plugin.json` / `.goose-plugin/plugin.json` — non-portable plugin metadata"
    - "Backward-compat user/project dirs that goose still scans but that other providers don't recognize as their own canonical layout (`.goose/skills`, `~/.config/goose/skills`)"
  rewrite_needed: true
  notes: |
    The Markdown body and the agentskills.io standard frontmatter
    (`name`, `description`, `metadata` containing `license` /
    `compatibility` / `allowed-tools` etc.) port cleanly to any other
    Agent-Skills consumer. Goose-specific frontmatter usage
    (`argument-hint`, `arguments`) and plugin-side metadata need to be
    dropped or rewritten when sharing with another provider, and any
    skill that lives only inside an Open-Plugin bundle must be unpacked
    into a plain directory tree before it can be linked elsewhere.

cli_params:
  - flag: /skills [name ...]
    description: |
      In-session slash command that lists installed skills (no args) or
      loads one or more named skills as a slash command
      (`crates/goose/src/slash_commands/skill_slash_command.rs`).
    example: /skills code-review edge-case-finder
  - flag: goose skills
    description: |
      CLI subcommand that prints a token-counted table of every skill
      `discover_skills()` returns for the current working directory
      (`crates/goose-cli/src/commands/skills.rs`). Read-only.
    example: goose skills
  - flag: goose configure
    description: |
      Interactive configuration TUI. Used to toggle built-in extensions
      (Summon controls all skill discovery), pick a provider/model, and
      edit permission/recipe settings.
    example: goose configure
  - flag: goose session --with-builtin <id>[,...]
    description: |
      Enable built-in extensions for the session. Omitting `summon` (or
      disabling it in `goose configure`) disables all skill discovery,
      including project and global file-tree skills, plus the
      `/skills` slash command.
    example: goose session --with-builtin developer,summon
  - flag: goose run --with-builtin <id>[,...]
    description: |
      Same as `session --with-builtin` but for non-interactive
      `goose run`. Omitting `summon` disables skill discovery for the
      run.
    example: goose run --with-builtin summon --recipe my-recipe.yaml
  - flag: goose plugin install [--auto-update] <git-url>
    description: |
      Clone a git repo into `Paths::plugins_dir()` /
      `<root>/.agents/plugins`, detect Open-Plugins or Gemini-extension
      format, and copy the plugin into place. Skills under
      `<plugin>/skills/` are imported and namespaced
      (`<plugin>:<skill>` for Open Plugins).
    example: goose plugin install https://github.com/example/my-goose-plugin.git
  - flag: goose plugin update <name>
    description: |
      Update a git-backed installed plugin in place; preserves
      `auto_update` and metadata. Auto-update uses a 24-hour rate limit
      (`AUTO_UPDATE_INTERVAL_HOURS`).
    example: goose plugin update my-plugin
  - flag: goose run --recipe <file> [--params KEY=VALUE ...]
    description: |
      Load a YAML recipe. Recipes are goose's reusable-workflow
      primitive (separate from skills) and can be registered as slash
      commands via `slash_commands:` in `config.yaml`.
    example: goose run --recipe deploy.yaml --params env=production
  - flag: goose recipe list [--format json] [--verbose]
    description: |
      List available recipes from local directories and the configured
      `GOOSE_RECIPE_GITHUB_REPO`. Not a skill-discovery path, but a
      related reuse surface.
    example: goose recipe list --format json

env_vars:
  - name: GOOSE_PATH_ROOT
    effect: |
      Overrides the root directory for all goose data/config/state.
      When set, `Paths::*` resolve under `<root>/config`,
      `<root>/data`, `<root>/state`, `<root>/.agents/plugins`,
      `<root>/.agents/agents`, and `<root>/.agents`. This relocates the
      config-dir skills location
      (`Paths::config_dir().join("skills")`) and the user-plugin
      location. The plugin `user_settings_path()` also follows
      `<root>/.config/goose/settings.json` under GOOSE_PATH_ROOT.
      Default locations when unset:

      - macOS: `~/Library/Application Support/Block/goose/`
      - Linux: `~/.local/share/goose/`
      - Windows: `%APPDATA%\Block\goose\`
  - name: CONTEXT_FILE_NAMES
    effect: |
      JSON array of filenames used for persistent context files
      (`goosehints` / `AGENTS.md` etc., default `[".goosehints"]`). Not a
      skill loader, but a related reuse surface that the linker must
      distinguish from skills.
  - name: GOOSE_SHELL
    effect: |
      Overrides the shell used by the Developer extension (and any
      scripts a skill instructs the agent to execute).
  - name: GOOSE_SEARCH_PATHS
    effect: |
      JSON array of directories prepended to `PATH` when extensions run
      commands. Helps skill scripts find custom binaries without
      polluting the global `PATH`.
  - name: GOOSE_MODE
    effect: |
      Tool execution mode (`auto`, `approve`, `chat`, `smart_approve`;
      default `smart_approve`). Affects whether a skill that drives
      tool calls (e.g. a deploy skill that runs shell scripts) needs
      user approval before each call.

changes:
  - "Split `os: all` location records into per-OS (macos / linux / windows) entries to satisfy the schema enum contract."
  - "Verified exact skill-discovery paths against source: `crates/goose/src/skills/mod.rs::all_skill_dirs` and `crates/goose/src/plugins/discovery.rs::discover_enabled_plugins`."
  - "Added the previously undocumented `~/.config/agents/skills/` global path (`all_skill_dirs` line 8 of the project tier)."
  - "Added the project-scope plugin path `<project>/.agents/plugins/<plugin>/skills/`."
  - "Recorded the built-in skill tier as `scope: other` with a `builtin://skills/<name>` synthetic path, sourced from `crates/goose/src/skills/builtin.rs` (`include_dir!` of `crates/goose/src/skills/builtins/*.md`)."
  - "Documented the deprecated `Skills` extension (v1.16.0–v1.24.0) and the v1.25.0+ replacement: Summon (`/docs/mcp/summon-mcp`)."
  - "Confirmed AAIF move (April 2026) — repo is now `github.com/aaif-goose/goose` and docs live at `goose-docs.ai`."
  - "Refined the frontmatter model: `name` + `description` are required; `metadata` is the agentskills.io free-form bag that carries `argument-hint` and `arguments` as untyped properties (Goose-specific surface; ignored by other providers)."
  - "Refined the precedence model to match the source-of-truth order: project → global → plugin → built-in, with first-encounter-wins by `name`."
  - "Documented the `disabledPlugins` / `enabledPlugins` JSON settings files and the per-plugin `plugins:` map in `config.yaml`."
  - "Recorded local evidence: `~/.agents/skills/find-skills/SKILL.md` and `~/.agents/.skill-lock.json` (version 3 schema, source attribution)."

requires_claudine_update: true
reason: |
  Claudine's linking module should model Goose's exact skill-tier layout
  in `all_skill_dirs()`: project (`.agents/skills` → `.goose/skills` →
  `.claude/skills`) then global (`~/.agents/skills` →
  `~/.config/goose/skills` → `~/.claude/skills` → `~/.config/agents/skills`),
  then plugin skills under `<home>/.agents/plugins/<plugin>/skills/` and
  `<project>/.agents/plugins/<plugin>/skills/`, then built-ins. It must
  distinguish the agentskills.io-standard `name` / `description` /
  `metadata` keys (portable) from Goose-specific `argument-hint` /
  `arguments` (kept inside `metadata.properties`) and plugin-bundle
  assets (non-portable). Linking should refuse to treat
  `~/.claude/skills/` as a Goose-canonical directory even though
  goose's discovery scans it for backward compatibility — link it as a
  Claude-Code export target instead. The `disabledPlugins` /
  `enabledPlugins` JSON settings and the `plugins:` map in
  `~/.config/goose/config.yaml` need to be exposed so a linked
  Goose session knows which plugin-provided skills are active.
---

# Goose CLI Agent Skills

## Overview

Goose CLI implements first-class Agent Skills as a directory containing a
`SKILL.md` entry point with YAML frontmatter and Markdown body. The
provider implements the [Agent Skills](https://agentskills.io/) open
standard: required frontmatter fields are `name` and `description`, with
the agentskills.io free-form `metadata` bag carrying
`license`/`compatibility`/`allowed-tools`/etc. Goose-specific extras
(`argument-hint`, `arguments`) ride inside the same `metadata` map and
are read as untyped `properties`.

Goose also maintains four related-but-distinct reuse systems. None of
them count as Agent Skills for this research; each has its own format
and is documented here only where it touches skill discovery or
portability:

| Surface | Format | Purpose |
|---|---|---|
| **Agent Skills** | `SKILL.md` per directory | Reusable instructions; this document. |
| **Plugins** | `~/.agents/plugins/<plugin>/` (Open Plugins or Gemini extension) | Bundles skills, hooks, and metadata; skills are namespaced `<plugin>:<skill>`. |
| **Recipes** | YAML recipes invoked via `goose run --recipe` or `/recipe` | Reusable task definitions; registered as slash commands via `slash_commands:` in `config.yaml`. |
| **goosehints** | Filenames in `CONTEXT_FILE_NAMES` (default `.goosehints`, also `AGENTS.md`) | Persistent context injected every turn — separate file type, not a skill. |
| **Built-in platform extensions** | Compiled into the `goose` binary | `Summon` is the discovery/loader for skills (v1.25.0+); the older `Skills` extension (v1.16.0–v1.24.0) is deprecated. |

Goose is hosted by the [Agentic AI Foundation (AAIF)](https://goose-docs.ai/blog/2026/04/07/goose-moves-to-aaif)
as of April 2026 — the canonical repository is
[github.com/aaif-goose/goose](https://github.com/aaif-goose/goose) and
documentation lives at [goose-docs.ai](https://goose-docs.ai/docs/).
The earlier Block-hosted `block/goose` repo redirects.

## Locations

Skill resources are stored by scope. The exact on-disk layout per
operating system is reproduced from the source code
(`crates/goose/src/skills/mod.rs::all_skill_dirs` and
`crates/goose/src/config/paths.rs`) so a Claudine linker can mirror the
discovery order bit-for-bit:

| Scope | macOS | Linux | Windows | Source-of-truth |
|---|---|---|---|---|
| Project (canonical) | `<cwd>/.agents/skills/<name>/SKILL.md` | `<cwd>/.agents/skills/<name>/SKILL.md` | `<cwd>\.agents\skills\<name>\SKILL.md` | `wd.join(".agents").join("skills")` |
| Project (back-compat) | `<cwd>/.goose/skills/<name>/SKILL.md` | same | `<cwd>\.goose\skills\<name>\SKILL.md` | `wd.join(".goose").join("skills")` |
| Project (back-compat) | `<cwd>/.claude/skills/<name>/SKILL.md` | same | `<cwd>\.claude\skills\<name>\SKILL.md` | `wd.join(".claude").join("skills")` |
| User (canonical) | `~/.agents/skills/<name>/SKILL.md` | same | `%USERPROFILE%\.agents\skills\<name>\SKILL.md` | `global_skills_dir()` |
| User (config-dir) | `~/.config/goose/skills/<name>/SKILL.md` | same | `%APPDATA%\Block\goose\config\skills\<name>\SKILL.md` | `Paths::config_dir().join("skills")` |
| User (back-compat) | `~/.claude/skills/<name>/SKILL.md` | same | `%USERPROFILE%\.claude\skills\<name>\SKILL.md` | `home.join(".claude").join("skills")` |
| User (optional) | `~/.config/agents/skills/<name>/SKILL.md` | same | `%USERPROFILE%\.config\agents\skills\<name>\SKILL.md` | `home.join(".config").join("agents").join("skills")` |
| Plugin (user) | `~/.agents/plugins/<plugin>/skills/<name>/SKILL.md` | same | `%USERPROFILE%\.agents\plugins\<plugin>\skills\<name>\SKILL.md` | `installed_plugin_skill_dirs()` ← `plugin_install_dir()` = `Paths::plugins_dir()` |
| Plugin (project) | `<cwd>/.agents/plugins/<plugin>/skills/<name>/SKILL.md` | same | `<cwd>\.agents\plugins\<plugin>\skills\<name>\SKILL.md` | `project_plugin_dir()` |
| Built-in | bundled with the `goose` binary (`builtin://skills/<name>` synthetic path) | same | same | `builtin::get_all()` ← `include_dir!("$CARGO_MANIFEST_DIR/src/skills/builtins")` |

### Platform-specific config roots (`Paths::config_dir` etc.)

These roots come from `etcetera`'s `choose_app_strategy` with
`author: "Block", top_level_domain: "Block", app_name: "goose"`, and are
overridden entirely when `GOOSE_PATH_ROOT` is set:

| Surface | macOS | Linux | Windows |
|---|---|---|---|
| `config_dir()` (skills at `config_dir/skills`) | `~/Library/Application Support/Block/goose/` | `$XDG_CONFIG_HOME/goose/` (default `~/.config/goose/`) | `%APPDATA%\Block\goose\config\` |
| `data_dir()` | `~/Library/Application Support/Block/goose/` | `~/.local/share/goose/` | `%APPDATA%\Block\goose\` |
| `state_dir()` | same as `data_dir()` | same as `data_dir()` | same as `data_dir()` |
| `plugins_dir()` (= `~/.agents/plugins`) | `~/.agents/plugins/` | `~/.agents/plugins/` | `%USERPROFILE%\.agents\plugins\` |
| `agents_home_dir()` | `~/.agents/` | `~/.agents/` | `%USERPROFILE%\.agents\` |

### Local evidence on this host

```text
~/.agents/skills/find-skills/SKILL.md      # 4635 bytes, installed from vercel-labs/skills
~/.agents/.skill-lock.json                 # version 3 schema tracking installed skill sources
~/.claude/skills/                          # 85+ symlinked Claude-Code skills, NOT a goose-native dir
~/Library/Application Support/Block/goose/ # absent — goose is not installed locally
~/.config/goose/                           # absent — goose is not installed locally
~/.config/agents/skills/                   # absent
~/.agents/plugins/                         # absent
```

The `~/.agents/.skill-lock.json` lock file (version 3) carries install
provenance (`source`, `sourceType`, `sourceUrl`, `skillPath`,
`skillFolderHash`, `installedAt`, `updatedAt`) and is written by
third-party skill installers (`npx skills add …`). Goose itself does
not consume this file — its discovery is purely filesystem-based — but
the lock file's presence is informative provenance for linking.

## File Format

A skill is a directory whose entry point is `SKILL.md`:

```text
my-skill/
├── SKILL.md           # Required frontmatter + Markdown instructions
├── setup.sh           # Optional supporting executable
├── scripts/           # Optional supporting folder
└── templates/         # Optional supporting folder
```

`SKILL.md` contains YAML frontmatter between `---` markers followed by
Markdown content. The provider's `SkillFrontmatter` struct accepts
exactly three top-level keys; everything else is preserved as a
free-form bag:

```yaml
---
name: code-review                # required; ≤ 64 chars; [a-z0-9-]; no leading/trailing '-'; no '/'
description: Comprehensive code review checklist for pull requests   # required; non-empty
metadata:
  license: MIT                   # agentskills.io-standard (free-form)
  compatibility: claude-code,goose,opencode
  allowed-tools: Read             # agentskills.io-standard (free-form)
  argument-hint: "[task]"         # Goose-specific (read as untyped `properties` value)
  arguments:                      # Goose-specific (positional names for $ARGUMENTS substitution)
    - task
---
# Code Review Checklist
…
```

When loaded, the body is wrapped with a synthetic header:

```text
# Loaded Skill: <name> (skill)

<description>

## Content

<markdown body>

## Supporting Files
Skill directory: <absolute path>
Relative paths in this skill resolve from the skill directory.
- scripts/setup.sh → /Users/…/my-skill/scripts/setup.sh (load_skill(name: "my-skill/scripts/setup.sh"))
…
```

`$ARGUMENTS` in the body is replaced with the raw args passed to the
`/skills <name> <args>` slash command (or `load_skill` tool call),
producing `loaded_skill_context_with_args`. Named positional arguments
declared in `metadata.arguments` are visible to the loader via
`skill_argument_names`.

## Discovery and Precedence

The on-disk walker in `crates/goose/src/skills/mod.rs::all_skill_dirs`
returns the discovery order **with a `(path, is_global)` flag**. The
scanner (`scan_skills_from_dir`) iterates this list and pushes each
skill into a shared `seen: HashSet<String>` keyed by `name`; the
**first** entry wins on `name` collisions. Source-of-truth order:

1. `<cwd>/.agents/skills/` — project canonical.
2. `<cwd>/.goose/skills/` — project backward-compat.
3. `<cwd>/.claude/skills/` — Claude-Code back-compat.
4. `~/.agents/skills/` — user canonical.
5. `Paths::config_dir()/skills` (`~/.config/goose/skills` on
   macOS/Linux, `%APPDATA%\Block\goose\config\skills` on Windows) —
   user config-dir back-compat.
6. `~/.claude/skills/` — Claude-Code user back-compat.
7. `~/.config/agents/skills/` — optional XDG-style global back-compat.
8. **Plugin skills** — each `<home>/.agents/plugins/<plugin>/skills/`
   and each `<cwd>/.agents/plugins/<plugin>/skills/`. Project-scope
   plugins are discovered before user-scope plugins inside
   `discover_enabled_plugins`. Open-Plugins skill names are
   namespaced as `<plugin>:<skill>`; Gemini-extension skills keep the
   unprefixed `SKILL.md` name.
9. **Built-in skills** — appended last
   (`builtin::get_all()` → `SourceType::BuiltinSkill` with
   `builtin://skills/<name>` synthetic path).

The Summon built-in MCP extension (v1.25.0+) is the discovery/loader
runtime: it exposes the resulting `SourceEntry` list to the model as a
`skills` tool and registers each non-built-in skill as a slash
command. The `/skills` slash command either lists them or, with one or
more names, loads each by name (e.g. `/skills code-review
edge-case-finder`).

Activation model:

- The model sees every skill's `name` and `description` in the
  system prompt and chooses one when the user's request matches.
- The user can force activation by saying "Use the X skill" or by
  invoking `/skills <name> [args]`.
- Built-in skills are pre-approved and do not need an extra consent
  step; user/project/plugin skills may require user approval before
  their supporting scripts run (governed by `GOOSE_MODE`).

Enable/disable mechanisms:

- **`disabledPlugins`** in `~/.config/goose/settings.json`,
  `<project>/.config/goose/settings.json`, or
  `<project>/.config/goose/settings.local.json` (JSON). Any plugin
  whose name appears here is skipped; `local` scope overrides
  project scope, which overrides user scope.
- **`enabledPlugins`** — same files. An empty `enabledPlugins`
  allowlists specific plugins; unlisted plugins are filtered.
- **`plugins:`** map in `~/.config/goose/config.yaml` —
  `{ "<absolute-plugin-path>": { enabled: bool } }`. A `false` entry
  silently disables that plugin; newly discovered plugins are added
  with `enabled: true` automatically.
- **`goose configure → Toggle Extensions`** — disable `Summon`
  entirely (kills all skill discovery for that user, including the
  `/skills` slash command).
- **`--with-builtin`** flag (without `summon`) on `goose session` /
  `goose run` — equivalent per-run disable.
- **`GOOSE_PATH_ROOT`** — relocates `Paths::config_dir()` and
  `Paths::plugins_dir()`; skill and plugin discovery follow.

There is **no per-skill disable flag in `SKILL.md`** itself and no
documented way to exclude one of the backward-compatibility paths —
removing the directory is the only opt-out.

## Portability

Portable assets:

- `SKILL.md` Markdown body.
- `SKILL.md` frontmatter: `name`, `description`, plus the
  agentskills.io-standard `metadata` entries (`license`,
  `compatibility`, `allowed-tools`, etc.).

Assets that need rewriting or host gating:

- Scripts and supporting files (host language/runtime availability,
  missing binaries, OS-specific shell).
- Project-layout assumptions in `SKILL.md` prose.
- `argument-hint` / `arguments` frontmatter (Goose-specific surface
  via the `metadata.properties` map; ignored by other Agent-Skills
  consumers).
- Plugin-bundled skills under `<plugin>/skills/` and the
  `<plugin>:<skill>` namespace convention.
- Compile-time built-in skills (`builtin://skills/<name>` synthetic
  paths, not backed by a file tree).
- `GOOSE_PATH_ROOT`-relocated config / plugin roots.
- Plugin manifests (`plugin.json`, `.plugin/plugin.json`,
  `.goose-plugin/plugin.json` for Open Plugins;
  `gemini-extension.json` for Gemini extensions).
- Backward-compat `.goose/skills` and `~/.config/goose/skills`
  layout — goose still scans them but other providers don't treat
  them as their canonical path.

The `~/.claude/skills/` and `.claude/skills/` paths are a special
case: goose discovers them only for backward compatibility with
Claude-Code exports. They are not Goose-canonical and should not be
linked from Goose-side metadata into other providers; if a Claudine
linker sees skills under these paths, it should treat them as
Claude-Code exports (link from Claude Code, not from Goose).

## Claudine Linking Notes

For cross-provider linking:

- Treat `~/.agents/skills/<name>/SKILL.md` and
  `.agents/skills/<name>/SKILL.md` as the canonical user and project
  Goose locations. Recognize (but rank below canonical) the
  back-compat `.goose/skills`, `.claude/skills`, `~/.claude/skills`,
  `~/.config/goose/skills`, and `~/.config/agents/skills` paths for
  import/discovery.
- Read `~/.agents/.skill-lock.json` for install provenance when
  present; the file is third-party (e.g. `npx skills add`) and goose
  itself does not require it.
- Recognize plugin skills under
  `~/.agents/plugins/<plugin>/skills/<skill>/SKILL.md` (user) and
  `<project>/.agents/plugins/<plugin>/skills/<skill>/SKILL.md`
  (project). Open-Plugin skill names carry a `<plugin>:` prefix;
  Gemini-extension skill names do not. Plugin discovery is gated by
  `disabledPlugins` / `enabledPlugins` JSON and the `plugins:` map in
  `~/.config/goose/config.yaml` — capture those settings alongside
  any plugin-bundled skills link.
- Classify built-in skills (`builtin://skills/<name>`) as
  non-portable unless the same skill is mirrored as a file-tree
  skill elsewhere.
- Mark skills containing `argument-hint` / `arguments` frontmatter,
  `scripts/` executables, OS-specific commands, or plugin-bundle
  metadata as needing rewrite before linking to another provider.
- Account for Summon extension availability when deciding whether a
  linked skill is "active" — a Goose session without Summon loaded
  (no `summon` in `--with-builtin`, or Summon toggled off in
  `goose configure`) sees no skills at all.
- When `GOOSE_PATH_ROOT` is set, resolve all paths under the
  relocated root; do not assume `~/.agents/`, `~/.config/goose/`, or
  `%APPDATA%\Block\goose\` defaults.

## Changelog

- **2026-07-03 (current)** — Verified skill-discovery paths against
  `crates/goose/src/skills/mod.rs::all_skill_dirs` and
  `crates/goose/src/plugins/discovery.rs`. Added the previously
  undocumented `~/.config/agents/skills/` path and the project-scope
  plugin path `<project>/.agents/plugins/<plugin>/skills/`. Added the
  built-in tier as `scope: other` with `builtin://skills/<name>`
  synthetic paths. Recorded the deprecated Skills extension
  (v1.16.0–v1.24.0) and the v1.25.0+ Summon replacement. Split all
  `os: all` records into per-OS entries to satisfy the schema
  contract. Documented `disabledPlugins` / `enabledPlugins` settings
  files and the per-plugin `plugins:` map. Updated `homepage` /
  `docs` / `skills_docs` to reflect the April 2026 AAIF move and the
  new `goose-docs.ai` documentation root. Recorded local evidence:
  `~/.agents/skills/find-skills/SKILL.md` (4635 bytes, installed from
  `vercel-labs/skills`) and `~/.agents/.skill-lock.json` (version 3
  schema).
- **2026-07-02** — First research pass. Catalogued canonical
  `.agents/skills/` user and project paths plus backward-compat
  `.goose/skills`, `.claude/skills`, `~/.claude/skills`, and
  `~/.config/goose/skills` paths. Documented the `Summon` extension,
  `/skills` slash command, and `goose plugin install` /
  `goose plugin update` flow. Confirmed Agent Skills open-standard
  frontmatter (`name`, `description`) and the optional
  `license` / `compatibility` / `metadata` / `allowed-tools` keys.

## Sources

- [Goose — Agent Skills guide](https://goose-docs.ai/docs/guides/context-engineering/using-skills)
- [Goose — Summon extension](https://goose-docs.ai/docs/mcp/summon-mcp)
- [Goose — Plugins](https://goose-docs.ai/docs/guides/context-engineering/plugins)
- [Goose — Skills extension (deprecated v1.16.0–v1.24.0)](https://goose-docs.ai/docs/mcp/skills-mcp)
- [Goose — CLI commands](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [Goose — Configuration files](https://goose-docs.ai/docs/guides/config-files)
- [Goose — Environment variables](https://goose-docs.ai/docs/guides/environment-variables)
- [Goose — Using goosehints](https://goose-docs.ai/docs/guides/context-engineering/using-goosehints)
- [Goose — Custom slash commands](https://goose-docs.ai/docs/guides/context-engineering/slash-commands)
- [Goose — Moving to AAIF (April 2026)](https://goose-docs.ai/blog/2026/04/07/goose-moves-to-aaif)
- [Goose — homepage (legacy Block URL)](https://block.github.io/goose/)
- [Goose — GitHub repository (aaif-goose)](https://github.com/aaif-goose/goose)
- [Goose — source: `crates/goose/src/skills/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/skills/mod.rs)
- [Goose — source: `crates/goose/src/skills/builtin.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/skills/builtin.rs)
- [Goose — source: `crates/goose/src/plugins/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/plugins/mod.rs)
- [Goose — source: `crates/goose/src/plugins/discovery.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/plugins/discovery.rs)
- [Goose — source: `crates/goose/src/config/paths.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs)
- [Goose — source: `crates/goose/src/slash_commands/skill_slash_command.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/slash_commands/skill_slash_command.rs)
- [Goose — source: `crates/goose-cli/src/commands/skills.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/commands/skills.rs)
- [Agent Skills open standard](https://agentskills.io/specification)