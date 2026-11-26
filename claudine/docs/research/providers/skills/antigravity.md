---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
homepage: https://antigravity.google/product/antigravity-cli
docs: https://antigravity.google/docs/cli/reference
skills_docs: https://antigravity.google/docs/skills
support: first_class
locations:
  - os: macos
    scope: user
    path: "~/.gemini/config/skills"
    notes: "Global user skills. A skill is stored as ~/.gemini/config/skills/<name>/SKILL.md."
  - os: linux
    scope: user
    path: "~/.gemini/config/skills"
    notes: "Global user skills. A skill is stored as ~/.gemini/config/skills/<name>/SKILL.md."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\skills"
    notes: "Global user skills. A skill is stored as %USERPROFILE%\\.gemini\\config\\skills\\<name>\\SKILL.md."
  - os: macos
    scope: user
    path: "~/.gemini/config/skills.json"
    notes: "Optional declared user skill config; entries point at additional skill directories."
  - os: linux
    scope: user
    path: "~/.gemini/config/skills.json"
    notes: "Optional declared user skill config; entries point at additional skill directories."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\skills.json"
    notes: "Optional declared user skill config; entries point at additional skill directories."
  - os: macos
    scope: repo
    path: ".agents/skills"
    notes: "Default workspace customization root; alternatives .agent/skills, _agents/skills, and _agent/skills are also scanned."
  - os: linux
    scope: repo
    path: ".agents/skills"
    notes: "Default workspace customization root; alternatives .agent/skills, _agents/skills, and _agent/skills are also scanned."
  - os: windows
    scope: repo
    path: ".agents\\skills"
    notes: "Default workspace customization root; alternatives .agent\\skills, _agents\\skills, and _agent\\skills are also scanned."
  - os: macos
    scope: repo
    path: ".agents/skills.json"
    notes: "Optional declared workspace skill config; paths resolve from the repository root unless absolute or ~/."
  - os: linux
    scope: repo
    path: ".agents/skills.json"
    notes: "Optional declared workspace skill config; paths resolve from the repository root unless absolute or ~/."
  - os: windows
    scope: repo
    path: ".agents\\skills.json"
    notes: "Optional declared workspace skill config; paths resolve from the repository root unless absolute or ~/."
  - os: macos
    scope: extension
    path: ".agents/plugins/<plugin_name>/skills"
    notes: "Workspace plugin skills load when the plugin is discovered and enabled."
  - os: linux
    scope: extension
    path: ".agents/plugins/<plugin_name>/skills"
    notes: "Workspace plugin skills load when the plugin is discovered and enabled."
  - os: windows
    scope: extension
    path: ".agents\\plugins\\<plugin_name>\\skills"
    notes: "Workspace plugin skills load when the plugin is discovered and enabled."
  - os: macos
    scope: extension
    path: "~/.gemini/config/plugins/<plugin_name>/skills"
    notes: "Global plugin skills load when the plugin is discovered and enabled."
  - os: linux
    scope: extension
    path: "~/.gemini/config/plugins/<plugin_name>/skills"
    notes: "Global plugin skills load when the plugin is discovered and enabled."
  - os: windows
    scope: extension
    path: "%USERPROFILE%\\.gemini\\config\\plugins\\<plugin_name>\\skills"
    notes: "Global plugin skills load when the plugin is discovered and enabled."
  - os: macos
    scope: system
    path: "~/.gemini/antigravity-cli/builtin/skills"
    notes: "Bundled Antigravity CLI skills observed locally in agy 1.1.0."
  - os: linux
    scope: system
    path: "~/.gemini/antigravity-cli/builtin/skills"
    notes: "Bundled Antigravity CLI skills; same home-relative application data root is used by the CLI docs and release notes."
  - os: windows
    scope: system
    path: "%USERPROFILE%\\.gemini\\antigravity-cli\\builtin\\skills"
    notes: "Expected Windows equivalent of the home-relative built-in skill root used by the CLI."
format:
  file_names: ["SKILL.md", "skills.json"]
  frontmatter: true
  required_fields: ["name", "description"]
  optional_fields: []
  body_format: markdown
  notes: "A skill is a directory under a skills folder containing SKILL.md. SKILL.md starts with YAML frontmatter containing name and description, followed by Markdown instructions. Optional sibling directories such as scripts/, examples/, resources/, and references/ may hold helper scripts, reference implementations, templates, manuals, and other assets. skills.json is not a skill file; it is an optional JSON registry with entries/inherits and include_only/exclude filters for non-standard skill directories."
discovery:
  mechanism: "Antigravity discovers customization roots from the current workspace hierarchy, global ~/.gemini/config, explicit skills.json entries/inherits, enabled plugin directories, and bundled built-in skills. For skills, only name and description are injected initially; the full SKILL.md body and linked references are loaded on demand when the model or user activates the skill. The /skills panel lists available skills."
  precedence: "Documented highest-to-lowest order is workspace project discovery, declared workspace configurations, global discovery under ~/.gemini/config, built-in customizations, then global declared configurations. Naming conflicts are resolved by the higher-priority customization overriding the lower-priority one."
  enable_disable: "A plain skill is enabled by placing a valid skills/<name>/SKILL.md under a discovered root or by declaring its directory in skills.json; it is disabled by removal or exclusion. skills.json entries support include_only and exclude regex filters. Plugin skills load only when their containing plugin is enabled; agy plugin enable and agy plugin disable affect those plugin-contributed skills."
  notes: "Workspace roots are .agents, .agent, _agents, and _agent at the project root. The agent walks from the current working directory toward the repository root to find workspace customizations. agy --add-dir adds workspace directories for the current session and the 1.0.8 changelog says custom skills are dynamically rediscovered after conversation switches or /add-dir. Local trust state exists in ~/.gemini/trustedFolders.json, and the codelab notes that Antigravity may ask whether the user trusts a folder, but the public skill docs do not specify a skill-only trust bypass or per-skill trust metadata."
portability:
  portable: false
  non_portable_assets: ["skills.json registry and regex filters", "plugins/<name>/plugin.json packaging", "Antigravity-specific paths under ~/.gemini/config and .agents", "Antigravity-specific slash commands or tool names inside SKILL.md", "bundled built-in skills under ~/.gemini/antigravity-cli/builtin/skills"]
  rewrite_needed: true
  notes: "Simple skills that are only skills/<name>/SKILL.md with name/description frontmatter and relative references are close to the open Agent Skills convention and can usually be linked or copied after changing destination path. Claudine must rewrite or drop skills.json, plugin packaging, and Antigravity-specific tool/slash-command instructions when exporting to providers that do not understand those surfaces."
cli_params:
  - flag: "--add-dir"
    description: "Adds a directory to the active workspace; custom skills are rediscovered when workspace directories change."
    example: "agy --add-dir /path/to/workspace"
  - flag: "--sandbox"
    description: "Runs the session with terminal restrictions enabled; affects execution of helper scripts or commands a skill asks the agent to run, not discovery."
    example: "agy --sandbox"
  - flag: "--dangerously-skip-permissions"
    description: "Auto-approves tool permission requests; affects runtime execution requested by skills, not whether skills load."
    example: "agy --dangerously-skip-permissions"
  - flag: "plugin install <target>"
    description: "Installs a plugin into the shared configuration directory so plugin skills can be discovered."
    example: "agy plugin install ./my-plugin"
  - flag: "plugin import [source]"
    description: "Imports plugins from supported sources such as gemini or claude."
    example: "agy plugin import gemini"
  - flag: "plugin enable <name>"
    description: "Enables an installed plugin, making its skills available."
    example: "agy plugin enable team-developer-kit"
  - flag: "plugin disable <name>"
    description: "Disables an installed plugin, removing its contributed skills from the active customization set."
    example: "agy plugin disable team-developer-kit"
  - flag: "plugin validate [path]"
    description: "Validates a plugin directory containing plugin.json and optional skills."
    example: "agy plugin validate ./my-plugin"
env_vars: []
changes: []
requires_claudine_update: true
reason: "Claudine's linker should add Antigravity as a skills-capable provider with .agents/.agent/_agents/_agent workspace roots, ~/.gemini/config user roots, skills.json declared roots, plugin-contributed skills, and Antigravity-specific portability rules."
---

# Antigravity Agent Skills

## Overview

Antigravity CLI implements Agent Skills as first-class customization resources. A skill is a directory inside a `skills/` folder with a required `SKILL.md` file. The `SKILL.md` file contains YAML frontmatter used for discovery and a Markdown body used as the agent's procedural or knowledge payload.

The implementation is part of Antigravity's broader customization system. The shipped `agy-customizations` built-in skill describes the customization types as rules, skills, plugins, hooks, and MCP servers. Skills are the on-demand, progressive surface: Antigravity initially exposes only a skill's `name` and `description` to the model, then loads the full `SKILL.md` and any referenced files when the model or user activates the skill. The interactive `/skills` panel lists available skills, and the Google codelab demonstrates asking `/skills` after creating a project-local skill.

This is not the same surface as generic `GEMINI.md` or `AGENTS.md` rules. Antigravity loads those as hierarchical rules, while Agent Skills require the `skills/<name>/SKILL.md` package shape.

Local observation on macOS with `agy 1.1.0`:

- `agy` is installed at `~/.local/bin/agy`.
- No `~/.antigravity` directory exists on this host.
- No local `~/.gemini/config/skills` directory exists on this host.
- Antigravity CLI built-in skills exist under `~/.gemini/antigravity-cli/builtin/skills`, including `agy-customizations` and `antigravity_guide`.
- `~/.gemini/trustedFolders.json` exists and records trusted workspace folders, but the skill docs inspected do not define per-skill trust metadata.

## Locations

Antigravity uses customization roots. A `skills/` directory under a root contains skill directories.

| OS | Scope | Path | Notes |
| --- | --- | --- | --- |
| macOS | user | `~/.gemini/config/skills/<name>/SKILL.md` | Global skills for all projects. Documented by the built-in customization guide and supported by release notes that corrected global customization paths to `~/.gemini/config/`. |
| Linux | user | `~/.gemini/config/skills/<name>/SKILL.md` | Same home-relative global path as macOS. |
| Windows | user | `%USERPROFILE%\.gemini\config\skills\<name>\SKILL.md` | Windows equivalent of the same home-relative `.gemini/config` root. The Windows installer uses `%LOCALAPPDATA%` for the binary, but Antigravity customization docs and changelog use the home `.gemini` tree for shared config. |
| macOS | repo | `.agents/skills/<name>/SKILL.md` | Default workspace root; discovered from the current working directory up to repository root. |
| Linux | repo | `.agents/skills/<name>/SKILL.md` | Same repository-relative layout. |
| Windows | repo | `.agents\skills\<name>\SKILL.md` | Same repository-relative layout with Windows separators. |
| macOS | repo | `.agent/skills/<name>/SKILL.md`, `_agents/skills/<name>/SKILL.md`, `_agent/skills/<name>/SKILL.md` | Documented alternate workspace customization roots. |
| Linux | repo | `.agent/skills/<name>/SKILL.md`, `_agents/skills/<name>/SKILL.md`, `_agent/skills/<name>/SKILL.md` | Documented alternate workspace customization roots. |
| Windows | repo | `.agent\skills\<name>\SKILL.md`, `_agents\skills\<name>\SKILL.md`, `_agent\skills\<name>\SKILL.md` | Documented alternate workspace customization roots. |
| macOS | repo | `.agents/skills.json` | Optional declared workspace registry for skill directories in non-standard locations. |
| Linux | repo | `.agents/skills.json` | Same registry format. |
| Windows | repo | `.agents\skills.json` | Same registry format. |
| macOS | user | `~/.gemini/config/skills.json` | Optional declared global registry. |
| Linux | user | `~/.gemini/config/skills.json` | Optional declared global registry. |
| Windows | user | `%USERPROFILE%\.gemini\config\skills.json` | Optional declared global registry. |
| macOS | extension | `.agents/plugins/<plugin_name>/skills/<name>/SKILL.md` and `~/.gemini/config/plugins/<plugin_name>/skills/<name>/SKILL.md` | Plugin-contributed skills load when the plugin is enabled. |
| Linux | extension | `.agents/plugins/<plugin_name>/skills/<name>/SKILL.md` and `~/.gemini/config/plugins/<plugin_name>/skills/<name>/SKILL.md` | Same plugin layout. |
| Windows | extension | `.agents\plugins\<plugin_name>\skills\<name>\SKILL.md` and `%USERPROFILE%\.gemini\config\plugins\<plugin_name>\skills\<name>\SKILL.md` | Same plugin layout. |
| macOS | system | `~/.gemini/antigravity-cli/builtin/skills/<name>/SKILL.md` | Observed locally. Built-ins are lower priority than workspace and global discovery. |
| Linux | system | `~/.gemini/antigravity-cli/builtin/skills/<name>/SKILL.md` | Expected equivalent based on home-relative CLI application data. |
| Windows | system | `%USERPROFILE%\.gemini\antigravity-cli\builtin\skills\<name>\SKILL.md` | Expected equivalent based on home-relative CLI application data. |

Antigravity does not use `~/.antigravity` for the skill resources observed on this host. The active Antigravity CLI configuration and built-ins are under `~/.gemini/`.

## File Format

The required artifact shape is:

```text
skills/<skill_name>/
├── SKILL.md
├── scripts/
├── examples/
├── resources/
└── references/
```

Only `SKILL.md` is required. The optional directories are conventional locations for helper scripts, example implementations, templates, resources, and detailed references. Relative links from `SKILL.md` are expected; progressive disclosure means large manuals should live in `references/` and be linked from the main skill file rather than pasted into the initial body.

`SKILL.md` must begin with YAML frontmatter:

```markdown
---
name: my-specialized-skill
description: Use this skill when the user asks for the team's specialized workflow.
---

# My Specialized Skill

1. Run the preparation command.
2. Inspect the generated output.
3. Verify the result with the documented check.
```

Recognized skill frontmatter:

| Key | Required | Notes |
| --- | --- | --- |
| `name` | yes | Unique skill identifier. The built-in guide recommends lowercase hyphenated names. |
| `description` | yes | Discovery text. Antigravity injects this into the model so it can decide whether to activate the skill. |

No additional skill-frontmatter keys were documented in the official codelab, built-in skill guide, local built-in `SKILL.md` files, CLI help, or public changelog reviewed for this research.

`skills.json` is a JSON registry, not a skill. It can live in a customization root such as `.agents/skills.json` or `~/.gemini/config/skills.json` and supports:

| Key | Type | Purpose |
| --- | --- | --- |
| `entries` | array | Additional directories to scan for skills. |
| `inherits` | array | Other registry files to merge in order. |
| `path` | string | Required inside each entry/inherit object. Absolute paths stay absolute, `~/` resolves from the user's home, and relative paths resolve from the repository root. |
| `include_only` | array of strings | Regex filters; only matching customization directory names load. |
| `exclude` | array of strings | Regex filters; matching customization directory names are skipped. |

Plugin skills use the same `SKILL.md` format but are nested under a plugin:

```text
plugins/<plugin_name>/
├── plugin.json
└── skills/
    └── <skill_name>/
        └── SKILL.md
```

`plugin.json` is the marker file for a plugin. Its `name` field is optional and defaults to the plugin directory name when omitted.

## Discovery and Precedence

Discovery happens through five surfaces:

1. Workspace customization roots: `.agents/`, `.agent/`, `_agents/`, and `_agent/` at the project root.
2. Declared workspace registries: `skills.json` entries and inherited registries.
3. Global discovery: `~/.gemini/config/`.
4. Built-in customizations bundled with the CLI.
5. Global declared registries.

The built-in customization guide documents this highest-to-lowest precedence:

| Rank | Source | Effect |
| --- | --- | --- |
| 1 | Workspace project discovery | Highest priority; project-local skills override lower-priority skills with the same name. |
| 2 | Declared workspace configurations | Skills listed through workspace `skills.json`. |
| 3 | Global discovery | Skills under `~/.gemini/config/skills`. |
| 4 | Built-in customizations | Bundled Antigravity skills such as `antigravity_guide`. |
| 5 | Global declared configurations | Lowest priority declared global entries. |

When names conflict, the higher-priority customization overrides the lower-priority one. Antigravity also deduplicates customizations by resolved file path so a resource discovered through multiple routes is not injected twice.

Skills are progressive. They are not loaded into the context window by default; only the name and description are exposed initially. The full `SKILL.md` body and linked references are loaded after activation. The codelab demonstrates that a `.agents/skills/my-favorite-things/SKILL.md` skill is discoverable immediately when Antigravity CLI starts in that project, appears in `/skills`, and may require user approval before use.

CLI and configuration effects:

- `agy --add-dir` adds another workspace directory. The `1.0.8` changelog says custom skills are dynamically rediscovered after conversation switches or `/add-dir`.
- `agy plugin install`, `agy plugin import`, `agy plugin enable`, and `agy plugin disable` affect plugin-contributed skills.
- `agy --sandbox` and `agy --dangerously-skip-permissions` affect runtime command/tool approval if a skill asks the agent to run scripts or commands; they do not define alternate discovery roots.
- `~/.gemini/trustedFolders.json` stores folder trust decisions locally. The codelab notes that the CLI may ask whether the user trusts a folder. No official source reviewed specified per-skill trust metadata or a separate CLI flag to load untrusted workspace skills.
- No environment variable that changes skill discovery roots, disables skills, or selects a skill profile was found in `agy --help`, plugin command help, the built-in skill docs, or the public changelog reviewed. Documented `AGY_CLI_*` variables found in release notes affect display behavior rather than skill loading.

## Portability

Plain Antigravity skills are close to the open Agent Skills convention: `skills/<name>/SKILL.md` with `name` and `description` frontmatter plus a Markdown body. That simple form can usually be linked or copied into another provider that accepts the same package shape after changing only the destination path.

The full Antigravity implementation is not fully portable as-is:

- `skills.json` is Antigravity-specific registry glue. Other providers may not honor `entries`, `inherits`, `include_only`, or `exclude`.
- Plugin packaging under `plugins/<name>/plugin.json` is Antigravity-specific. A plugin's nested `skills/` may be extractable, but the plugin manifest, MCP config, hooks, and activation state require provider-specific handling.
- Built-in skills under `~/.gemini/antigravity-cli/builtin/skills` are Antigravity product assets and should not be linked into another provider by default.
- Skill bodies may mention Antigravity-only slash commands such as `/skills`, plugin commands, or Antigravity-specific permission and artifact flows. Those instructions need content rewriting for another provider.
- Helper scripts and resources are portable only when relative links remain valid and the target provider allows the agent to read or execute them.

For Claudine, classify a simple `SKILL.md` directory as structurally portable but provider-scoped. Classify Antigravity registries, plugin bundles, built-in skills, and Antigravity-specific runtime instructions as requiring rewrite or exclusion.

## Claudine Linking Notes

Claudine should model Antigravity as a first-class skills provider with these link targets:

- User skills: `~/.gemini/config/skills/<name>/SKILL.md`.
- Repo skills: `.agents/skills/<name>/SKILL.md`, with `.agent`, `_agents`, and `_agent` as accepted alternate roots.
- Declared roots: `.agents/skills.json` and `~/.gemini/config/skills.json`.
- Plugin skills: `plugins/<plugin_name>/skills/<name>/SKILL.md` under workspace or global customization roots.
- System skills: `~/.gemini/antigravity-cli/builtin/skills/<name>/SKILL.md`, read-only and not a normal sync target.

The linker should avoid:

- Treating `GEMINI.md`, `AGENTS.md`, or `.agents/rules/*.md` as Agent Skills. They are rules.
- Treating slash-command behavior as skills. Skill-derived slash command UI details belong to slash-command research.
- Writing into `~/.gemini/antigravity-cli/builtin/skills`.
- Exporting `skills.json` or `plugin.json` to providers that do not support Antigravity's registry/plugin model.
- Assuming `~/.antigravity` is a skills root. It was absent locally and not used by the inspected docs.

This research implies Claudine generated metadata or linking code should add Antigravity skill paths and portability rules. In particular, Antigravity should not be mapped to Gemini CLI's older `~/.gemini/skills` global path; the Antigravity CLI customization system uses `~/.gemini/config/skills`.

## Sources

- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Antigravity skills documentation](https://antigravity.google/docs/skills)
- [Antigravity CLI reference](https://antigravity.google/docs/cli/reference)
- [Google codelab: How to use AI Agent Skills with Antigravity CLI](https://codelabs.developers.google.com/antigravity/how-to-create-agent-skills-for-antigravity-cli)
- [google-antigravity/antigravity-cli README](https://github.com/google-antigravity/antigravity-cli)
- [google-antigravity/antigravity-cli CHANGELOG](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- Local `agy 1.1.0 --help`, `agy plugin --help`, and local built-in docs under `~/.gemini/antigravity-cli/builtin/skills/agy-customizations/` observed on macOS on 2026-07-08.
