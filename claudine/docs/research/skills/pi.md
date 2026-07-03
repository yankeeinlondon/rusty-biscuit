---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: codex
model: default

homepage: https://pi.dev/
docs: https://pi.dev/docs/latest
skills_docs: https://pi.dev/docs/latest/skills

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.pi/agent/skills/
    notes: Default user skill directory. The actual config root can be replaced with `PI_CODING_AGENT_DIR`.
  - os: linux
    scope: user
    path: ~/.pi/agent/skills/
    notes: Default user skill directory. The actual config root can be replaced with `PI_CODING_AGENT_DIR`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\skills\\"
    notes: Default user skill directory. The actual config root can be replaced with `PI_CODING_AGENT_DIR`.
  - os: macos
    scope: user
    path: ~/.agents/skills/
    notes: Agent-compatible global skill directory. Root `.md` files are ignored here; directories containing `SKILL.md` are discovered recursively.
  - os: linux
    scope: user
    path: ~/.agents/skills/
    notes: Agent-compatible global skill directory. Root `.md` files are ignored here; directories containing `SKILL.md` are discovered recursively.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\"
    notes: Agent-compatible global skill directory. Root `.md` files are ignored here; directories containing `SKILL.md` are discovered recursively.
  - os: macos
    scope: repo
    path: .pi/skills/
    notes: Project skill directory loaded only after the project is trusted. Root `.md` files and nested `SKILL.md` directories are discovered.
  - os: linux
    scope: repo
    path: .pi/skills/
    notes: Project skill directory loaded only after the project is trusted. Root `.md` files and nested `SKILL.md` directories are discovered.
  - os: windows
    scope: repo
    path: .pi\\skills\\
    notes: Project skill directory loaded only after the project is trusted. Root `.md` files and nested `SKILL.md` directories are discovered.
  - os: macos
    scope: repo
    path: .agents/skills/
    notes: Project agent-compatible skill directory discovered in the current directory and ancestors up to the git root, or filesystem root outside a git repo. Loaded only after project trust.
  - os: linux
    scope: repo
    path: .agents/skills/
    notes: Project agent-compatible skill directory discovered in the current directory and ancestors up to the git root, or filesystem root outside a git repo. Loaded only after project trust.
  - os: windows
    scope: repo
    path: .agents\\skills\\
    notes: Project agent-compatible skill directory discovered in the current directory and ancestors up to the git root, or filesystem root outside a git repo. Loaded only after project trust.
  - os: macos
    scope: user
    path: ~/.pi/agent/settings.json
    notes: Global `skills` array can name skill files, skill directories, globs, exclusions, installed packages, or package filters.
  - os: linux
    scope: user
    path: ~/.pi/agent/settings.json
    notes: Global `skills` array can name skill files, skill directories, globs, exclusions, installed packages, or package filters.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\settings.json"
    notes: Global `skills` array can name skill files, skill directories, globs, exclusions, installed packages, or package filters.
  - os: macos
    scope: repo
    path: .pi/settings.json
    notes: Project `skills` array and project package entries load only after project trust.
  - os: linux
    scope: repo
    path: .pi/settings.json
    notes: Project `skills` array and project package entries load only after project trust.
  - os: windows
    scope: repo
    path: .pi\\settings.json
    notes: Project `skills` array and project package entries load only after project trust.
  - os: macos
    scope: user
    path: ~/.pi/agent/npm/<package>/skills/ and ~/.pi/agent/git/<host>/<repo>/skills/
    notes: Installed user package resources; package `pi.skills` manifest entries can point elsewhere inside the package.
  - os: linux
    scope: user
    path: ~/.pi/agent/npm/<package>/skills/ and ~/.pi/agent/git/<host>/<repo>/skills/
    notes: Installed user package resources; package `pi.skills` manifest entries can point elsewhere inside the package.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\npm\\<package>\\skills\\ and %USERPROFILE%\\.pi\\agent\\git\\<host>\\<repo>\\skills\\"
    notes: Installed user package resources; package `pi.skills` manifest entries can point elsewhere inside the package.
  - os: macos
    scope: repo
    path: .pi/npm/<package>/skills/ and .pi/git/<host>/<repo>/skills/
    notes: Installed project package resources; loaded only after project trust.
  - os: linux
    scope: repo
    path: .pi/npm/<package>/skills/ and .pi/git/<host>/<repo>/skills/
    notes: Installed project package resources; loaded only after project trust.
  - os: windows
    scope: repo
    path: .pi\\npm\\<package>\\skills\\ and .pi\\git\\<host>\\<repo>\\skills\\
    notes: Installed project package resources; loaded only after project trust.
  - os: macos
    scope: extension
    path: <extension-discovered path>
    notes: Extensions can contribute skill paths at runtime through resource discovery APIs; SDK callers can also replace the discovered skill list.
  - os: linux
    scope: extension
    path: <extension-discovered path>
    notes: Extensions can contribute skill paths at runtime through resource discovery APIs; SDK callers can also replace the discovered skill list.
  - os: windows
    scope: extension
    path: <extension-discovered path>
    notes: Extensions can contribute skill paths at runtime through resource discovery APIs; SDK callers can also replace the discovered skill list.
  - os: macos
    scope: other
    path: --skill <path>
    notes: Repeatable session-only skill file or directory path. It is additive even when `--no-skills` is set.
  - os: linux
    scope: other
    path: --skill <path>
    notes: Repeatable session-only skill file or directory path. It is additive even when `--no-skills` is set.
  - os: windows
    scope: other
    path: --skill <path>
    notes: Repeatable session-only skill file or directory path. It is additive even when `--no-skills` is set.

format:
  file_names:
    - SKILL.md
    - "*.md"
  frontmatter: true
  required_fields:
    - description
  optional_fields:
    - name
    - license
    - compatibility
    - metadata
    - allowed-tools
    - disable-model-invocation
  body_format: markdown
  notes: |
    Pi implements the Agent Skills standard and documents `name` and `description` as standard-required fields, but the implementation is lenient. `description` is the only hard load requirement; a skill with no description produces a warning and is not loaded. If `name` is absent, Pi uses the parent directory name. Invalid names, names longer than 64 characters, and descriptions longer than 1024 characters warn but still load. Pi does not require `name` to match the parent directory.

    A directory containing `SKILL.md` is treated as a skill root and Pi does not recurse deeper inside that root for additional skills. Otherwise, Pi recursively searches subdirectories for `SKILL.md`. Direct root `.md` files are accepted as individual skills only in Pi-native skill roots such as `~/.pi/agent/skills/`, `.pi/skills/`, package `skills/`, configured skill directories, or explicit directory paths. Direct root `.md` files are ignored in `~/.agents/skills/` and project `.agents/skills/`.

    A skill directory can contain arbitrary sibling files, including scripts, references, and assets. Pi tells the model to resolve relative paths against the skill directory and to use absolute paths in tool commands. Directory scans honor `.gitignore`, `.ignore`, and `.fdignore`, skip dot-prefixed entries, skip `node_modules`, follow valid symlinks, and silently skip broken symlinks or unreadable paths.

discovery:
  mechanism: |
    On resource-loader startup or reload, Pi resolves enabled resources from global settings, project settings, package manifests, conventional package directories, built-in user/project directories, `.agents/skills`, explicit CLI paths, and extension-contributed paths. It parses only frontmatter metadata up front. Visible skills are appended to the system prompt as `<available_skills>` entries containing `name`, `description`, and `location`, but only when the `read` tool is available. The model is instructed to use the `read` tool to load the full skill file on demand.

    Interactive, RPC, and SDK command expansion also supports `/skill:<name> [args]`. A skill command reads the skill file immediately, strips frontmatter, wraps the body in a `<skill name="..." location="...">` block, adds `References are relative to <baseDir>.`, and appends user arguments after the block. `disable-model-invocation: true` hides the skill from the system prompt but does not remove the `/skill:name` command.
  precedence: |
    Name collisions are first-writer-wins. Pi records a collision diagnostic with the winning and losing paths, keeps the first skill already in the map, and does not merge same-name resources.

    The resource accumulator adds project `.pi/skills` first when trusted, then trusted project `.agents/skills` ancestor directories, then user `~/.pi/agent/skills`, then user `~/.agents/skills`. CLI `--skill` paths and extension resources are merged into the effective path list before loading, and path de-duplication uses canonical paths. Package identity de-duplication is separate: if the same package appears in global and project settings, the project package entry wins.

    Project settings override global settings by normal settings merge rules. Resource arrays support glob patterns and explicit `!`, `+`, and `-` overrides. In settings, paths in `~/.pi/agent/settings.json` resolve relative to `~/.pi/agent`, while paths in `.pi/settings.json` resolve relative to `.pi`; absolute paths and `~` are supported.
  enable_disable: |
    `--no-skills` / `-ns` disables normal skill discovery and loading, but explicit `--skill <path>` entries still load. Removing or excluding a path disables that skill. `enableSkillCommands: false` disables `/skill:name` registration but does not remove model-visible skills. `disable-model-invocation: true` hides a skill from the system prompt and requires explicit `/skill:name` invocation.

    Project-local `.pi/skills`, project `.agents/skills`, project settings, and project package resources require project trust. Interactive mode asks according to `defaultProjectTrust`; non-interactive modes do not prompt. `--approve` / `-a` trusts project-local resources for one run, and `--no-approve` / `-na` ignores them for one run. Saved trust decisions live in `~/.pi/agent/trust.json` and the nearest current-or-parent path decision applies.
  notes: |
    Pi has no built-in permission sandbox. Trust controls whether project resources are loaded; it does not restrict what loaded skills, extensions, tools, or model output can ask the process to do. Package resources can be enabled or disabled through `pi config`, global/project settings, or package filters. Extensions can add skill paths dynamically, and SDK callers can filter, merge, or replace the skill list via `skillsOverride`.

portability:
  portable: true
  non_portable_assets:
    - "Pi-native direct root `*.md` skills; many Agent Skills consumers require `SKILL.md` inside a directory."
    - "`disable-model-invocation` behavior; other providers may use a different auto-invocation control or none."
    - "Experimental `allowed-tools` semantics and tool names."
    - "Pi settings/package wiring: `settings.json` `skills`, package `pi.skills`, package filters, `pi config` enabled/disabled state, and installed package roots under `.pi` or `~/.pi/agent`."
    - "Project trust state in `~/.pi/agent/trust.json` and one-shot trust flags."
    - "Extension-discovered skill paths and SDK `skillsOverride` virtual skills."
    - "Relative scripts/assets that assume Pi's prompt wording, built-in `read`/`bash` tools, local executable availability, or package-installed dependencies."
  rewrite_needed: true
  notes: |
    A directory skill with `SKILL.md`, YAML frontmatter, Markdown body, `name`, and `description` is linkable as an Agent Skills artifact. Claudine can preserve sibling files and map the directory into another provider's skill root.

    Claudine should rewrite Pi-only direct `.md` skills into `<name>/SKILL.md` before linking to providers that require directory skills. It should preserve `name`, `description`, `license`, `compatibility`, and `metadata`; retain unknown fields only when the destination accepts them; and treat `allowed-tools`, `disable-model-invocation`, package manifests, settings arrays, trust files, and extension/SDK-provided skills as provider-specific.

cli_params:
  - flag: --skill <path>
    description: Load a skill file or directory for the current session. Repeatable and additive even with `--no-skills`.
    example: pi --skill ./skills/review/SKILL.md "review this repo"
  - flag: --no-skills, -ns
    description: Disable normal skill discovery and loading. Explicit `--skill` paths still load.
    example: pi --no-skills --skill ./one-off/SKILL.md -p "use only this skill"
  - flag: --approve, -a
    description: Trust project-local resources for this run, including `.pi/skills`, project `.agents/skills`, project settings, and project package skills.
    example: pi --approve "use project skills"
  - flag: --no-approve, -na
    description: Ignore project-local resources for this run, including project skills.
    example: pi --no-approve -p "summarize safely"
  - flag: --extension <path>, -e <path>
    description: Load a session-only extension. Extensions can contribute skill paths at runtime through resource discovery.
    example: pi -e ./extensions/resources.ts "run with extension resources"
  - flag: --no-extensions, -ne
    description: Disable extension discovery. This can indirectly prevent extension-contributed skill paths, while explicit `-e` paths still work.
    example: pi --no-extensions "ignore installed extension resources"
  - flag: --tools <tools>, -t <tools>
    description: Tool allowlist. If `read` is absent, Pi does not append available skills to the system prompt.
    example: pi --tools read,bash "load matching skills"
  - flag: --no-tools, -nt
    description: Disable all tools by default. This removes `read`, so model-visible skill listings are not appended.
    example: pi --no-tools -p "answer without tools or skills"
  - flag: --no-builtin-tools, -nbt
    description: Disable built-in tools by default. This removes built-in `read` unless an extension provides a replacement, so skill prompt exposure can be affected.
    example: pi --no-builtin-tools -e ./read-extension.ts "use extension tools"
  - flag: --exclude-tools <tools>, -xt <tools>
    description: Tool denylist. Excluding `read` prevents available-skill prompt injection.
    example: pi --exclude-tools read "do not expose skill metadata"
  - flag: pi install <source> [-l]
    description: Install a Pi package that can contribute skills through `skills/` or package `pi.skills`; `-l` writes project settings.
    example: pi install npm:@scope/package
  - flag: pi remove <source> [-l]
    description: Remove a package source from settings, disabling its packaged skills for that scope.
    example: pi remove npm:@scope/package
  - flag: pi update [source|self|pi]
    description: Update Pi and/or installed packages; package updates can change packaged skill files.
    example: pi update --extensions
  - flag: pi config
    description: Open the TUI for enabling or disabling package and local resources, including skills.
    example: pi config

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: Replaces the global agent config directory, moving user settings, trust store, packages, and the default `skills/` directory away from `~/.pi/agent`.
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: Replaces session storage only; it does not directly affect skill discovery.
  - name: PI_OFFLINE
    effect: Disables startup network operations and package/update checks. Existing local skill discovery still works, but package install/update network activity is affected.
  - name: PI_PACKAGE_DIR
    effect: Overrides the package directory used to locate the installed Pi package itself; useful for packaged/Nix/Guix layouts, not a user skill root.

changes: []

requires_claudine_update: true
reason: |
  Claudine should add Pi as a first-class skill-linking target/source once provider metadata is generated: recognize `~/.pi/agent/skills`, `.pi/skills`, `~/.agents/skills`, project `.agents/skills` ancestor discovery, settings/package skill paths, `--skill` session paths, first-writer-wins collision behavior, project-trust gating, and Pi's direct-root-`*.md` compatibility case. Linking should rewrite direct `.md` skills into directory `SKILL.md` form for stricter providers and should not try to link virtual SDK override skills or extension-generated resources without a real file path.
---

# Pi Agent Skills

## Overview

Pi has first-class Agent Skills support. The official skills documentation describes skills as reusable capability packages loaded on demand, and current source implements a dedicated `Skill` model, scanner, frontmatter parser, system-prompt formatter, `/skill:name` command expander, and SDK/resource-loader override path.

The runtime is intentionally progressive-disclosure based. At startup or reload, Pi scans skill locations and reads frontmatter metadata. Skills visible to model invocation are rendered into the system prompt as XML-like `<available_skills>` entries with `name`, `description`, and `location`. The full `SKILL.md` body is not automatically injected. The model is told to use the `read` tool when a task matches a skill description. Users can force a skill through `/skill:<name>`, which reads the skill file immediately and injects its body into the user message.

Pi documents that it implements the Agent Skills standard but remains lenient. The most important practical difference is that Pi does not require the `name` field to match the parent directory. It also still supports direct root `.md` files as Pi-native skills in Pi-managed skill directories, while agent-compatible `.agents/skills` directories only accept nested `SKILL.md` skills.

Local inspection for this run found Pi installed as `/Users/ken/.bun/bin/pi`. The process `HOME` was `/Users/ken/.claudine`, so Pi's default agent directory resolved to `/Users/ken/.claudine/.pi/agent`. That directory contained `sessions/` and `auth.json`, but no observed `skills/` directory. No `/Users/ken/.claudine/.agents/skills` directory existed. This is host evidence only; portable paths above use `~`.

## Locations

Pi's default user config root is `~/.pi/agent`, computed from the current app name as `PI_CODING_AGENT_DIR` when overridden. Under that root, `skills/` is the native user skill directory. Pi also scans `~/.agents/skills/` as a global agent-compatible skill directory.

Project skill loading is trust-gated. After a project is trusted, Pi loads `.pi/skills/` from the current working directory and project `.agents/skills/` directories from the current directory and ancestors. Ancestor scanning stops at the git repository root when one is found; outside a git repo it continues to the filesystem root. The global `~/.agents/skills/` path is explicitly excluded from the project trust scan even when the current directory is the user's home.

Settings and packages are additional durable sources:

| Scope | File or Directory | Behavior |
|---|---|---|
| User settings | `~/.pi/agent/settings.json` | `skills` can name skill files/directories and patterns. Paths resolve relative to `~/.pi/agent` unless absolute or `~`-prefixed. |
| Project settings | `.pi/settings.json` | Same `skills` array, loaded only after project trust. Paths resolve relative to `.pi`. |
| User npm packages | `~/.pi/agent/npm/<package>/` | Conventional `skills/` directory or `package.json` `pi.skills` entries. |
| User git packages | `~/.pi/agent/git/<host>/<repo>/` | Same package resource rules as npm packages. |
| Project npm packages | `.pi/npm/<package>/` | Loaded only after project trust. |
| Project git packages | `.pi/git/<host>/<repo>/` | Loaded only after project trust. |
| CLI | `--skill <path>` | Session-only explicit file or directory. Loads even with `--no-skills`. |
| Extension / SDK | Runtime-provided paths or `skillsOverride` | Can add, filter, or replace skills without a stable provider-owned filesystem location. |

Package directories support two discovery styles. If `package.json` contains a `pi.skills` manifest array, those paths are used relative to the package root. If no `pi` manifest is present, a conventional `skills/` directory is auto-discovered. Package and settings filters can include glob patterns, `!pattern` exclusions, `+path` force-includes, and `-path` force-excludes.

## File Format

The durable skill artifact is either a directory containing `SKILL.md` plus optional sibling files, or a Pi-native root `.md` skill file in locations that allow direct files.

```text
my-skill/
├── SKILL.md
├── scripts/
│   └── process.sh
├── references/
│   └── api-reference.md
└── assets/
    └── template.json
```

`SKILL.md` is Markdown with YAML frontmatter. Pi recognizes these frontmatter fields:

| Field | Required by Pi loader | Behavior |
|---|---:|---|
| `description` | Yes | Routing text. Missing or blank descriptions warn and prevent the skill from loading. Descriptions longer than 1024 characters warn but still load. |
| `name` | No | Skill name. Defaults to the parent directory name when omitted. Invalid names warn but still load. |
| `license` | No | Standard Agent Skills metadata; Pi preserves it as frontmatter but does not use it for runtime behavior. |
| `compatibility` | No | Standard Agent Skills metadata; Pi preserves it as frontmatter but does not use it for runtime behavior. |
| `metadata` | No | Standard arbitrary metadata; Pi preserves it as frontmatter but does not use it for runtime behavior. |
| `allowed-tools` | No | Documented as experimental. Treat as Pi/provider-specific unless a destination provider explicitly supports the same semantics. |
| `disable-model-invocation` | No | When `true`, the skill is omitted from `<available_skills>` and can still be used via `/skill:name`. |

Unknown frontmatter fields are ignored by the Pi loader. Pi validates the standard name shape but treats most violations as warnings: names should be 1-64 characters, lowercase letters, digits, and hyphens only, with no leading/trailing hyphen and no consecutive hyphens.

The body is CommonMark/GFM-style Markdown instructions. Supporting files are freeform. Pi's system prompt tells the model that relative paths in a skill file are relative to the skill directory, and `/skill:name` expansion injects an explicit "References are relative to ..." line. Scripts and assets therefore remain file-backed resources beside the skill, not embedded metadata.

Discovery behavior depends on the directory type:

| Source shape | Accepted entries |
|---|---|
| Pi-native roots such as `~/.pi/agent/skills/`, `.pi/skills/`, configured directories, package `skills/`, and CLI directory paths | Direct root `*.md` files and recursive directories containing `SKILL.md`. |
| Agent-compatible roots `~/.agents/skills/` and project `.agents/skills/` | Recursive directories containing `SKILL.md`; direct root `*.md` files are ignored. |
| A directory containing `SKILL.md` | That directory is one skill root and scanning does not recurse deeper below it for more skills. |

Scans honor `.gitignore`, `.ignore`, and `.fdignore` files. Dot-prefixed entries and `node_modules` are skipped during recursion. Valid symlinks are followed; broken symlinks are skipped.

## Discovery and Precedence

Pi's `DefaultResourceLoader` is the central implementation. It resolves resources from settings, packages, default filesystem locations, CLI paths, and extension-provided paths, then calls `loadSkills`. The loader can be reloaded during an interactive session; Pi's UI describes reload as reloading keybindings, extensions, skills, prompts, and themes.

Project trust is part of discovery. A project requires trust when Pi finds `.pi/settings.json`, `.pi/extensions`, `.pi/skills`, `.pi/prompts`, `.pi/themes`, `.pi/SYSTEM.md`, `.pi/APPEND_SYSTEM.md`, or project `.agents/skills` in the current directory or an ancestor. Trusting the project allows those project resources to load. Declining trust skips protected project resources. Non-interactive modes do not prompt; with no saved decision they obey `defaultProjectTrust` from global settings, where `ask` and `never` skip resources and `always` loads them. `--approve` and `--no-approve` override the trust decision for one run.

Skill name collisions are first-writer-wins. The source keeps a `Map<string, Skill>` and records a collision diagnostic when a later skill has an already-loaded name. The later skill is not loaded and skills are not merged.

Within the implemented resource accumulator, trusted project `.pi/skills` are added before trusted project `.agents/skills` ancestor directories, then user `~/.pi/agent/skills`, then user `~/.agents/skills`. CLI and extension paths are merged into the effective path list before `loadSkills` runs, with canonical path de-duplication. Package precedence is handled before skill loading: when the same package appears in both global and project settings, the project package entry wins by package identity.

Runtime application has two paths:

1. Model-visible skills: if the `read` tool is available and a skill does not set `disable-model-invocation: true`, Pi appends a skills section to the system prompt. If `read` is disabled through `--no-tools`, `--tools` without `read`, `--exclude-tools read`, or equivalent tool selection, Pi does not append the skill list.
2. Explicit skill commands: `/skill:<name> [args]` looks up a loaded skill by exact name, reads the file, strips frontmatter, wraps the body in a `<skill>` block, and appends arguments. This path is controlled by `enableSkillCommands` in settings or `/settings` in interactive mode.

Extensions can affect skills in two ways. A CLI `-e` or installed extension can contribute resource paths through the resource discovery API, and SDK consumers can provide `skillsOverride` to filter, merge, or replace the loaded list. These are runtime/resource-loader behaviors rather than portable skill artifacts.

## Portability

Standard Pi directory skills are portable Agent Skills artifacts when they use `SKILL.md`, YAML frontmatter, Markdown body, and common fields such as `name`, `description`, `license`, `compatibility`, and `metadata`. Claudine can link those as-is into another provider's skill root and copy sibling files.

Several Pi forms need provider-specific handling:

| Artifact | Classification | Claudine behavior |
|---|---|---|
| `<skill>/SKILL.md` with standard fields | Linkable | Preserve directory, body, and sibling files. |
| Direct root `*.md` skill | Rewrite needed | Convert to `<derived-or-frontmatter-name>/SKILL.md` for providers that require directory skills. |
| `disable-model-invocation` | Provider-specific metadata | Map only to destinations with an equivalent auto-invocation disable feature; otherwise preserve only as inert metadata when safe. |
| `allowed-tools` | Provider/tool-specific | Do not blindly map; tool names and permission semantics are provider-specific. |
| `settings.json` `skills` array and package filters | Non-portable wiring | Use as discovery input, not as a linked artifact. |
| `~/.pi/agent/trust.json` | Non-portable state | Do not link. It is host/project trust state. |
| Package `pi.skills` manifest entries | Provider-specific wiring | Resolve to concrete skill files when possible; do not require destination providers to understand Pi package manifests. |
| Extension-discovered or SDK virtual skills | Non-portable unless file-backed | Link only when Claudine can resolve a real source file/directory and stable sibling assets. |

The main semantic portability hazard is that Pi's system prompt includes the absolute skill file location and expects the model to use Pi's `read` tool to load it. That runtime detail should not be copied into the skill body. Skill bodies that mention Pi-specific commands, `pi config`, `PI_CODING_AGENT_DIR`, Pi packages, or Pi's built-in tool names need review before linking elsewhere.

## Claudine Linking Notes

Claudine should model Pi as a first-class skills provider with these rules:

- Discover user skills from `~/.pi/agent/skills/` and `~/.agents/skills/`, honoring `PI_CODING_AGENT_DIR` for the native Pi root.
- Discover repo skills from `.pi/skills/` and `.agents/skills/` in the current directory and ancestors up to the git root.
- Treat project Pi resources as trust-gated metadata. The linker should report project skill paths but should not imply that Pi will load them without project trust.
- Accept `SKILL.md` directory skills and Pi-native direct root `.md` skills. Normalize direct `.md` skills to directory `SKILL.md` form when exporting to stricter providers.
- Preserve sibling files when linking a skill directory. Avoid flattening references, scripts, and assets because Pi relies on relative path behavior.
- Implement collision reporting as first-writer-wins for Pi parity, not repo-over-user override. Surface both winner and loser paths when possible.
- Do not link `trust.json`, package install directories, package manager cache state, or `settings.json` itself as a skill. Use those files only to discover concrete skill paths.
- Treat extension and SDK virtual skills as non-linkable unless they have a concrete `filePath` and `baseDir`.
- Mark `allowed-tools`, `disable-model-invocation`, and Pi-specific body instructions as rewrite/review points in portability metadata.

This research implies Claudine generated provider metadata and linking rules should change when Pi is added to the compiled provider roster. No code change is required for this research document itself.

## Sources

- [Pi Skills documentation](https://pi.dev/docs/latest/skills)
- [Pi Settings documentation](https://pi.dev/docs/latest/settings)
- [Pi Security documentation](https://pi.dev/docs/latest/security)
- [Pi Packages documentation](https://pi.dev/docs/latest/packages)
- [Pi SDK documentation](https://pi.dev/docs/latest/sdk)
- [Pi repository](https://github.com/earendil-works/pi)
- [Source: `packages/coding-agent/src/core/skills.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/skills.ts)
- [Source: `packages/coding-agent/src/core/resource-loader.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/resource-loader.ts)
- [Source: `packages/coding-agent/src/core/package-manager.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/package-manager.ts)
- [Source: `packages/coding-agent/src/core/trust-manager.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/trust-manager.ts)
- [Source: `packages/coding-agent/src/core/agent-session.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts)
- [Source: `packages/coding-agent/src/core/system-prompt.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts)
- [Source: `packages/coding-agent/src/cli/args.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts)
