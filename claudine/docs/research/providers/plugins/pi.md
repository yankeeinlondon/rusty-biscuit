---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default

homepage: https://pi.dev/
docs: https://pi.dev/docs/latest
plugin_docs: https://pi.dev/docs/latest/packages

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.pi/agent/settings.json
    notes: Global Pi settings. User-scope package installs are recorded in the packages array here.
  - os: linux
    scope: user
    path: ~/.pi/agent/settings.json
    notes: Global Pi settings. The active agent directory can be moved with PI_CODING_AGENT_DIR.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\settings.json"
    notes: Global Pi settings on Windows.
  - os: macos
    scope: repo
    path: .pi/settings.json
    notes: Project settings. Project packages are recorded here and load only after project trust.
  - os: linux
    scope: repo
    path: .pi/settings.json
    notes: Project settings. Project packages are recorded here and load only after project trust.
  - os: windows
    scope: repo
    path: ".pi\\settings.json"
    notes: Project settings. Project packages are recorded here and load only after project trust.
  - os: macos
    scope: user
    path: ~/.pi/agent/extensions/
    notes: Global auto-discovered extension files and extension directories.
  - os: linux
    scope: user
    path: ~/.pi/agent/extensions/
    notes: Global auto-discovered extension files and extension directories.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\extensions\\"
    notes: Global auto-discovered extension files and extension directories.
  - os: macos
    scope: repo
    path: .pi/extensions/
    notes: Project-local extensions. Loaded only after project trust.
  - os: linux
    scope: repo
    path: .pi/extensions/
    notes: Project-local extensions. Loaded only after project trust.
  - os: windows
    scope: repo
    path: ".pi\\extensions\\"
    notes: Project-local extensions. Loaded only after project trust.
  - os: macos
    scope: user
    path: ~/.pi/agent/skills/
    notes: Global Pi skill directory.
  - os: linux
    scope: user
    path: ~/.pi/agent/skills/
    notes: Global Pi skill directory.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\skills\\"
    notes: Global Pi skill directory.
  - os: macos
    scope: user
    path: ~/.agents/skills/
    notes: Cross-harness global Agent Skills directory also scanned by Pi.
  - os: linux
    scope: user
    path: ~/.agents/skills/
    notes: Cross-harness global Agent Skills directory also scanned by Pi.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\"
    notes: Cross-harness global Agent Skills directory also scanned by Pi.
  - os: macos
    scope: repo
    path: .pi/skills/
    notes: Project-local Pi skills. Loaded only after project trust.
  - os: linux
    scope: repo
    path: .pi/skills/
    notes: Project-local Pi skills. Loaded only after project trust.
  - os: windows
    scope: repo
    path: ".pi\\skills\\"
    notes: Project-local Pi skills. Loaded only after project trust.
  - os: macos
    scope: repo
    path: .agents/skills/
    notes: Cross-harness project skills in cwd and ancestors up to git root, loaded only after project trust.
  - os: linux
    scope: repo
    path: .agents/skills/
    notes: Cross-harness project skills in cwd and ancestors up to git root, loaded only after project trust.
  - os: windows
    scope: repo
    path: ".agents\\skills\\"
    notes: Cross-harness project skills in cwd and ancestors up to git root, loaded only after project trust.
  - os: macos
    scope: user
    path: ~/.pi/agent/prompts/
    notes: Global prompt templates.
  - os: linux
    scope: user
    path: ~/.pi/agent/prompts/
    notes: Global prompt templates.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\prompts\\"
    notes: Global prompt templates.
  - os: macos
    scope: repo
    path: .pi/prompts/
    notes: Project prompt templates. Loaded only after project trust.
  - os: linux
    scope: repo
    path: .pi/prompts/
    notes: Project prompt templates. Loaded only after project trust.
  - os: windows
    scope: repo
    path: ".pi\\prompts\\"
    notes: Project prompt templates. Loaded only after project trust.
  - os: macos
    scope: user
    path: ~/.pi/agent/themes/
    notes: Global JSON themes.
  - os: linux
    scope: user
    path: ~/.pi/agent/themes/
    notes: Global JSON themes.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\themes\\"
    notes: Global JSON themes.
  - os: macos
    scope: repo
    path: .pi/themes/
    notes: Project JSON themes. Loaded only after project trust.
  - os: linux
    scope: repo
    path: .pi/themes/
    notes: Project JSON themes. Loaded only after project trust.
  - os: windows
    scope: repo
    path: ".pi\\themes\\"
    notes: Project JSON themes. Loaded only after project trust.
  - os: macos
    scope: user
    path: ~/.pi/agent/git/<host>/<path>/
    notes: Global git package clones.
  - os: linux
    scope: user
    path: ~/.pi/agent/git/<host>/<path>/
    notes: Global git package clones.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\git\\<host>\\<path>\\"
    notes: Global git package clones.
  - os: macos
    scope: repo
    path: .pi/git/<host>/<path>/
    notes: Project git package clones.
  - os: linux
    scope: repo
    path: .pi/git/<host>/<path>/
    notes: Project git package clones.
  - os: windows
    scope: repo
    path: ".pi\\git\\<host>\\<path>\\"
    notes: Project git package clones.
  - os: macos
    scope: repo
    path: .pi/npm/node_modules/<package>/
    notes: Project npm package installs.
  - os: linux
    scope: repo
    path: .pi/npm/node_modules/<package>/
    notes: Project npm package installs.
  - os: windows
    scope: repo
    path: ".pi\\npm\\node_modules\\<package>\\"
    notes: Project npm package installs.
  - os: macos
    scope: user
    path: "<global npm root>/<package>/"
    notes: User npm packages install into the package manager's global node_modules root; with Bun this is derived from bun pm bin -g.
  - os: linux
    scope: user
    path: "<global npm root>/<package>/"
    notes: User npm packages install into the package manager's global node_modules root.
  - os: windows
    scope: user
    path: "<global npm root>\\<package>\\"
    notes: User npm packages install into the package manager's global node_modules root.
  - os: macos
    scope: other
    path: /tmp/pi-extensions/
    notes: Temporary package material used by --extension/-e runs; exact root comes from the OS temp directory.
  - os: linux
    scope: other
    path: /tmp/pi-extensions/
    notes: Temporary package material used by --extension/-e runs; exact root comes from the OS temp directory.
  - os: windows
    scope: other
    path: "%TEMP%\\pi-extensions\\"
    notes: Temporary package material used by --extension/-e runs.
  - os: macos
    scope: marketplace
    path: https://pi.dev/packages
    notes: Official package gallery indexes npm packages tagged for Pi and exposes install commands.
  - os: linux
    scope: marketplace
    path: https://pi.dev/packages
    notes: Official package gallery indexes npm packages tagged for Pi and exposes install commands.
  - os: windows
    scope: marketplace
    path: https://pi.dev/packages
    notes: Official package gallery indexes npm packages tagged for Pi and exposes install commands.

manifest:
  file_names: ["package.json"]
  format: json
  required_fields: []
  optional_fields: ["name", "keywords", "dependencies", "bundledDependencies", "peerDependencies", "pi.extensions", "pi.skills", "pi.prompts", "pi.themes", "pi.video", "pi.image"]
  package_layout: "A Pi package is an npm, git, URL, or local directory package root. Resources may be declared in package.json under pi.extensions, pi.skills, pi.prompts, and pi.themes, or discovered from conventional extensions/, skills/, prompts/, and themes/ directories. Manifest paths are relative to the package root; arrays support glob patterns and ! exclusions. Settings object-form package filters can further narrow extensions, skills, prompts, and themes with exact +path/-path entries."
  notes: "The pi-package npm keyword is recommended for gallery discoverability, not required for loading. If no pi manifest exists, Pi auto-discovers extensions/*.ts|*.js, skills containing SKILL.md plus top-level .md skills in Pi skill locations, prompts/*.md, and themes/*.json."

lifecycle:
  install: "CLI-driven with pi install <source>; use -l/--local for project settings. Sources include npm:<pkg>[@version], git:<url-or-shorthand>[@ref], protocol git URLs, raw HTTPS GitHub URLs, absolute paths, and relative paths. Project packages missing on startup are installed automatically after project trust."
  update: "pi update updates Pi itself and packages; pi update --extensions updates packages only; pi update --all updates Pi, packages, and reconciles pinned git refs; pi update <source> or pi update --extension <source> updates one package. Versioned npm specs and pinned git refs do not float."
  remove: "pi remove <source> or pi uninstall <source> removes the source from settings and removes npm/git package material for that scope; local path sources are only removed from settings."
  enable_disable: "pi config opens a TUI for enabling or disabling extensions, skills, prompt templates, and themes from installed packages and local directories in global and project scopes. Settings object-form package filters also disable resource classes with [] or exclude files with !pattern/-path."
  trust: "Project-local settings, project packages, project extensions, project skills, project prompts, project themes, and project system prompt files load only after project trust. Interactive startup can ask; non-interactive modes use defaultProjectTrust unless --approve/-a or --no-approve/-na overrides for one run."
  versioning: "npm package versions can be pinned in the source string. Git package refs can be pinned to a tag or commit; updates reconcile to that configured ref and do not move pinned refs."
  notes: "There is no separate signed-plugin audit command in the documented CLI. Pi package installation can run npm install and extension code executes as the user."

packaged_resources:
  skills: full
  scripts: partial
  slash_commands: full
  subagents: none
  mcp_servers: partial
  hooks: full
  prompts: full
  config: partial
  assets: full
  other: ["TypeScript extensions", "custom tools", "event handlers", "custom UI components", "themes", "custom providers", "extension-registered CLI flags"]

discovery:
  mechanism: "At startup or /reload, Pi resolves packages from project settings first and user settings second, deduplicates identical packages so the project entry wins, installs missing trusted project packages when allowed, collects package resources from package.json pi entries or conventional directories, then layers settings paths and auto-discovered local resources. Explicit CLI -e/--extension package sources are resolved as temporary resources for that run."
  precedence: "Observed source code in the locally installed Pi 0.72.1 ranks resources as: project settings entries, project auto-discovered entries, user settings entries, user auto-discovered entries, then package resources. Project package entries also dedupe over user package entries for the same identity. This means local project/user resources can shadow plugin/package resources when resource names collide."
  namespacing: "Pi package resources are not automatically namespaced by package name. Extension commands register the name supplied to pi.registerCommand and appear as /name. Skills register as /skill:name when skill commands are enabled. Prompt templates use the Markdown filename as /name. This differs from Claude Code, where plugin skills are always invoked as /plugin-name:skill-name."
  conflicts: "Skill name collisions warn and keep the first skill found. Prompt and theme deduplication also keep the first resource after precedence sorting. Extension conflicts for tools, commands, and flags are reported as diagnostics while all extensions remain loaded; runtime precedence follows extension load order."
  notes: "Resource disable flags such as --no-extensions, --no-skills, --no-prompt-templates, and --no-themes suppress discovered resources but still allow explicit CLI paths for the matching resource type."

security:
  trust_model: "Pi uses project trust for project-local settings, resources, packages, and extensions, but user/global packages and explicit CLI -e extensions are already inside the user's trust boundary. Package docs warn that packages run with full system access and should be reviewed before installation."
  permissions: "There is no built-in permission popup system. Extensions can implement their own gates by intercepting lifecycle events such as tool_call, and CLI --tools/--no-tools/--no-builtin-tools controls tool availability for built-in, extension, and custom tools."
  sandbox_interaction: "Pi has no built-in sandbox. The docs recommend running the whole pi process in a container or using an extension to route built-in tools into an isolated environment. Extensions run wherever the pi process runs."
  credential_access: "Extensions execute TypeScript in the Pi process with user privileges and can access process environment, filesystem paths, provider credentials visible to the process, and any configured package dependencies. Git package installs use configured SSH keys and git credential helpers for SSH/HTTPS sources."
  update_risk: "Unpinned npm and git package updates can change behavior when pi update, pi update --extensions, or pi update --all runs. Project settings can cause missing project packages to install automatically after trust. Pinned npm versions and pinned git refs reduce update drift."
  notes: "Local inspection found ~/.pi/agent/settings.json with model defaults only, no local package entries, and no ~/.pi/agent/extensions, skills, prompts, themes, npm, or git package directories. This repository has no .pi directory in the scanned root."

distribution:
  marketplace: true
  registry_url: https://pi.dev/packages
  source_types: ["npm", "git", "GitHub shorthand", "HTTPS URL", "SSH git URL", "local folder", "local file", "private npm registry via npm config", "private git repository via git credentials"]
  publishing: "Publish an npm package or git repository with a package.json pi manifest or conventional resource directories. Add the pi-package keyword for the Pi gallery. The gallery displays image and video metadata from the package.json pi block."
  private_distribution: "Private distribution uses the underlying source mechanism: private npm registries through npm configuration and private git repositories through SSH keys or HTTPS credential helpers. Pi docs also mention npmCommand in settings to pin npm operations to a wrapper such as mise or asdf."
  notes: "The official package gallery is an index of packages, not a Claude-style marketplace catalog that must be added before installing individual plugins."

portability:
  link_plugin_as_unit: true
  extract_resources: true
  portable_resources: ["skills", "prompt templates", "themes", "scripts referenced by skills", "assets referenced by skills"]
  non_portable_assets: ["TypeScript extensions", "extension dependencies", "extension-registered tools", "extension-registered commands", "extension-registered flags", "provider-specific package.json pi metadata", "npm package identity", "git source refs", "credentials", "MCP adapter extension configuration"]
  rewrite_needed: true
  notes: "Claudine can preserve a Pi package as a unit for Pi by linking or recording its package source. For other providers, extract standard Agent Skills, prompt Markdown, and passive assets; do not blindly execute or translate TypeScript extensions. Pi prompt templates can map to slash commands by filename, but Pi's /skill:name command convention differs from Claude Code's plugin namespace and should be rewritten or documented during export."

cli_params:
  - flag: "pi install <source>"
    description: "Install a package source and add it to user settings."
    example: "pi install npm:@foo/bar"
  - flag: "pi install <source> -l, --local"
    description: "Install a package source and add it to .pi/settings.json."
    example: "pi install git:github.com/user/repo -l"
  - flag: "pi remove <source>"
    description: "Remove a package source from user settings and remove installed npm/git package material."
    example: "pi remove npm:@foo/bar"
  - flag: "pi uninstall <source>"
    description: "Alias for pi remove."
    example: "pi uninstall npm:@foo/bar"
  - flag: "pi remove <source> -l, --local"
    description: "Remove a package source from project settings."
    example: "pi remove npm:@foo/bar -l"
  - flag: "pi list"
    description: "List installed packages from user and project settings."
    example: "pi list"
  - flag: "pi update"
    description: "Update Pi and installed packages."
    example: "pi update"
  - flag: "pi update --all"
    description: "Update Pi, update packages, and reconcile pinned git refs."
    example: "pi update --all"
  - flag: "pi update --extensions"
    description: "Update installed packages only."
    example: "pi update --extensions"
  - flag: "pi update --self"
    description: "Update Pi itself only."
    example: "pi update --self"
  - flag: "pi update --self --force"
    description: "Reinstall Pi even if current."
    example: "pi update --self --force"
  - flag: "pi update <source>"
    description: "Update one configured package source."
    example: "pi update npm:@foo/bar"
  - flag: "pi update --extension <source>"
    description: "Update one configured package source."
    example: "pi update --extension npm:@foo/bar"
  - flag: "pi config"
    description: "Open the TUI to enable or disable package and local resources."
    example: "pi config"
  - flag: "pi -e, --extension <path-or-source>"
    description: "Load an extension file or package source for the current run only; can be repeated."
    example: "pi -e npm:@foo/bar"
  - flag: "pi --no-extensions, -ne"
    description: "Disable extension discovery; explicit -e paths still load."
    example: "pi --no-extensions -e ./my-extension.ts"
  - flag: "pi --skill <path>"
    description: "Load a skill file or directory; repeatable and additive even with --no-skills."
    example: "pi --skill ./skills/review"
  - flag: "pi --no-skills, -ns"
    description: "Disable skill discovery and loading, except explicit --skill paths."
    example: "pi --no-skills --skill ./one-skill"
  - flag: "pi --prompt-template <path>"
    description: "Load a prompt template file or directory; repeatable."
    example: "pi --prompt-template ./prompts/review.md"
  - flag: "pi --no-prompt-templates, -np"
    description: "Disable prompt template discovery and loading, except explicit --prompt-template paths."
    example: "pi --no-prompt-templates"
  - flag: "pi --theme <path>"
    description: "Load a theme file or directory; repeatable."
    example: "pi --theme ./themes/team.json"
  - flag: "pi --no-themes"
    description: "Disable theme discovery and loading, except explicit --theme paths."
    example: "pi --no-themes"
  - flag: "pi --approve, -a"
    description: "Trust project-local settings and resources for one run."
    example: "pi --approve"
  - flag: "pi --no-approve, -na"
    description: "Ignore project-local settings and resources for one run."
    example: "pi --no-approve"
  - flag: "pi --offline"
    description: "Disable startup network operations, same as PI_OFFLINE=1."
    example: "pi --offline"

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: "Overrides the Pi agent configuration directory, whose default is ~/.pi/agent."
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: "Overrides session storage directory; --session-dir overrides it for a run."
  - name: PI_PACKAGE_DIR
    effect: "Overrides the package directory for Nix/Guix-style store paths in the installed Pi CLI help."
  - name: PI_OFFLINE
    effect: "When set to 1, true, or yes, disables startup network operations and package update/install reconciliation paths that check offline mode."
  - name: PI_TELEMETRY
    effect: "Overrides install telemetry when set to 1/true/yes or 0/false/no; relevant to package/install provenance rather than resource loading."
  - name: GIT_TERMINAL_PROMPT
    effect: "For git package sources, setting 0 disables credential prompts in non-interactive runs."
  - name: GIT_SSH_COMMAND
    effect: "For git package sources, can force non-interactive SSH behavior such as BatchMode and short connect timeouts."

gaps:
  - "The official docs do not publish a full JSON schema for package.json pi fields."
  - "The official docs document npm and git install locations for macOS/Linux-style paths but do not give explicit Windows global npm root expansion."
  - "The exact current source-code precedence was checked against locally installed Pi 0.72.1 under the older @mariozechner package name; official hosted docs now use @earendil-works/pi-coding-agent."
  - "No local package installation was performed because this research task should not mutate user or repo Pi settings."
  - "Pi has extension packages that can implement MCP or subagents, but Pi core documentation states MCP and subagents are not built in; package containment for those surfaces is therefore extension-mediated rather than native manifest support."

changes: []

requires_claudine_update: true
reason: "Pi packages are first-class containers, but their portable subset differs from Claude Code: Pi package manifests use package.json pi fields, resources are not package-namespaced, and executable TypeScript extensions should be marked non-portable unless linked as Pi-only package sources."
---

# Pi Plugin Packages

## Overview

Pi calls its plugin container a **Pi package**. A package bundles TypeScript extensions, Agent Skills, prompt templates, and themes so they can be shared through npm, git, or a local path. The package can declare resources in `package.json` under the `pi` key, or it can rely on conventional directories such as `extensions/`, `skills/`, `prompts/`, and `themes/`.

This is meaningfully different from Claude Code. Claude Code has a dedicated plugin system whose plugins extend Claude Code with skills, agents, hooks, MCP servers, and LSP servers, are discovered through marketplaces, and are usually namespaced as `/plugin-name:resource-name`. Pi packages are closer to npm/git/local bundles of Pi resource folders plus executable TypeScript extensions. Pi does have an official package gallery, but users install the package source directly with `pi install npm:name`, `pi install git:...`, or a local path rather than first adding a marketplace catalog.

Pi keeps the core small and explicitly pushes behavior into extensions, skills, prompt templates, themes, and packages. It intentionally does not ship built-in MCP, subagents, permission popups, plan mode, to-dos, or background bash; these can be built or installed as extensions/packages. For Claudine, this means Pi package support is real, but several resource categories are implemented by executable extension code rather than by a stable native manifest surface.

## Installation and Locations

Pi settings are JSON. Global settings live at `~/.pi/agent/settings.json`; project settings live at `.pi/settings.json`. The docs say `install` and `remove` write to global settings by default and to project settings when `-l` is passed. Project settings can be shared, and Pi installs missing project packages automatically on startup after the project is trusted.

Package sources accepted by `pi install` and settings are:

| Source | Example | Storage |
| --- | --- | --- |
| npm | `npm:@scope/pkg@1.2.3`, `npm:pkg` | User installs under the global package manager node_modules root; project installs under `.pi/npm/node_modules/` |
| git | `git:github.com/user/repo@v1`, `git:git@github.com:user/repo@v1`, `https://github.com/user/repo@v1`, `ssh://git@github.com/user/repo@v1` | User clones under `~/.pi/agent/git/<host>/<path>`; project clones under `.pi/git/<host>/<path>` |
| local path | `/absolute/path/to/package`, `./relative/path/to/package` | Recorded in settings without copying; relative paths resolve against the settings file |

Local path behavior is precise: if the path is a file, Pi loads it as a single extension; if it is a directory, Pi loads resources using package rules. `pi -e` or `--extension` can load a source for the current run only; the locally installed CLI help and docs both describe this as a temporary install path rather than a persistent settings change.

Pi also has direct resource folders outside packages:

| Resource | Global | Project |
| --- | --- | --- |
| Extensions | `~/.pi/agent/extensions/*.ts`, `~/.pi/agent/extensions/*/index.ts` | `.pi/extensions/*.ts`, `.pi/extensions/*/index.ts` |
| Skills | `~/.pi/agent/skills/`, `~/.agents/skills/` | `.pi/skills/`, `.agents/skills/` in cwd and ancestors |
| Prompt templates | `~/.pi/agent/prompts/*.md` | `.pi/prompts/*.md` |
| Themes | `~/.pi/agent/themes/*.json` | `.pi/themes/*.json` |

Local inspection on this host found `pi` installed at `/Users/ken/.bun/bin/pi`, symlinked to an older `@mariozechner/pi-coding-agent` package. The current official docs use `@earendil-works/pi-coding-agent`, so the hosted docs are treated as current authority and local help/source as corroborating implementation evidence. `/Users/ken/.pi/agent/settings.json` exists and contains model defaults only. No local Pi packages, package clones, npm package installs, extensions, skills, prompts, or themes were observed under `/Users/ken/.pi/agent` in the inspected depth. This repository root has no `.pi` directory.

## Manifest and Package Format

The Pi package manifest is `package.json`. Pi recognizes a `pi` object:

```json
{
  "name": "my-package",
  "keywords": ["pi-package"],
  "pi": {
    "extensions": ["./extensions"],
    "skills": ["./skills"],
    "prompts": ["./prompts"],
    "themes": ["./themes"]
  }
}
```

Manifest paths are relative to the package root. Arrays support glob patterns and `!` exclusions. Gallery metadata is also stored inside `pi`; `video` accepts MP4 and `image` accepts PNG, JPEG, GIF, or WebP, with video taking precedence when both are set.

If no `pi` manifest is present, Pi falls back to conventional directories:

| Directory | Discovery |
| --- | --- |
| `extensions/` | Loads `.ts` and `.js` files |
| `skills/` | Recursively finds `SKILL.md` folders and loads top-level `.md` files as skills |
| `prompts/` | Loads `.md` files |
| `themes/` | Loads `.json` files |

Settings can filter package resources with object-form package entries:

```json
{
  "packages": [
    {
      "source": "npm:my-package",
      "extensions": ["extensions/*.ts", "!extensions/legacy.ts"],
      "skills": [],
      "prompts": ["prompts/review.md"],
      "themes": ["+themes/legacy.json"]
    }
  ]
}
```

Omitting a key loads all resources of that type. `[]` loads none. `!pattern` excludes matches. `+path` force-includes an exact path and `-path` force-excludes an exact path, relative to the package root. Filters layer on top of the package manifest, so they narrow what the package itself exposes.

Dependencies belong in normal npm `dependencies`. When Pi installs npm or git packages, it runs `npm install`. Pi core packages used by extensions and skills should be listed in `peerDependencies` with a `"*"` range and not bundled: `@earendil-works/pi-ai`, `@earendil-works/pi-agent-core`, `@earendil-works/pi-coding-agent`, `@earendil-works/pi-tui`, and `typebox`. Other Pi packages referenced from `node_modules/` paths must be included in `dependencies` and `bundledDependencies`.

Claude Code is different here: a Claude Code plugin normally has `.claude-plugin/plugin.json` inside the plugin directory, with fields such as `name`, `description`, `version`, and component definitions. Claude Code marketplaces use `.claude-plugin/marketplace.json`; marketplace entries can also define advanced component paths and strictness behavior. Pi does not use `.claude-plugin/` files.

## Packaged Resources

Pi packages natively package:

| Resource | Pi package support | Notes |
| --- | --- | --- |
| Agent Skills | Full | `skills/` or `pi.skills`; Pi implements Agent Skills and loads full `SKILL.md` on demand |
| scripts | Partial | Freeform files may be bundled, especially inside skills or extension packages; no native `scripts` manifest key was found |
| slash commands | Full | Prompt templates become `/name`; extensions can register commands with `pi.registerCommand()` |
| subagents | None in core | Pi intentionally does not ship subagents; packages can implement them as extensions |
| MCP servers | Partial | Pi core intentionally does not ship MCP; packages can implement MCP support as extensions, such as a Pi MCP adapter |
| hooks | Full through extensions | Extensions subscribe to lifecycle and tool events with `pi.on(...)` |
| prompts | Full | `prompts/` or `pi.prompts`; filenames become slash commands |
| config | Partial | Package sources and resource filters live in settings; packages can include package-level config files consumed by extensions |
| assets | Full | Packages may include arbitrary assets used by extensions, themes, prompts, or skills |
| themes | Full | `themes/` or `pi.themes`; JSON color themes |

Extensions are Pi's most powerful packaged resource. They are TypeScript modules that can register tools callable by the LLM, intercept events, block or modify tool calls, prompt users through the TUI, register commands, add renderers, define shortcuts, register additional CLI flags, and register providers. This is more executable than Claude Code's plugin manifest resources. Claude Code plugins can include hooks and MCP servers that execute commands, but the plugin container itself is described and loaded through Claude's plugin/marketplace model rather than through arbitrary TypeScript module loading as the primary extension surface.

## Lifecycle and Trust

The documented package lifecycle is:

```bash
pi install npm:@foo/bar@1.0.0
pi install git:github.com/user/repo@v1
pi install https://github.com/user/repo
pi install /absolute/path/to/package
pi install ./relative/path/to/package
pi remove npm:@foo/bar
pi list
pi update
pi update --all
pi update --extensions
pi update --self
pi update --self --force
pi update npm:@foo/bar
pi update --extension npm:@foo/bar
```

By default, install and remove operate on global settings. `-l` or `--local` writes project settings instead. `pi remove` also has the alias `pi uninstall`.

Versioning is source-specific. Versioned npm specs are pinned and skipped by package updates. Git refs are pinned tags or commits; package updates reconcile existing clones to the configured ref but do not move them to a newer ref. To move a git package to another pinned ref, install the same source with the new ref.

Project trust is the central trust gate. Interactive Pi asks before trusting a project folder that has project settings, project resources, project `.agents/skills`, or project system prompt files and no saved trust decision. Trust decisions are stored in `~/.pi/agent/trust.json`. Trusting allows Pi to load `.pi/settings.json`, project `.pi` resources, missing project packages, and project package-managed extensions. Non-interactive modes do not prompt; they use `defaultProjectTrust`, with `--approve` and `--no-approve` as one-run overrides.

Claude Code has a richer plugin lifecycle. Its `/plugin` UI and commands install plugins from marketplaces, list installed plugins, enable, disable, uninstall, reload plugins with `/reload-plugins`, update marketplaces, and configure auto-updates. Claude Code also shows what a plugin will install, including commands, agents, skills, hooks, MCP servers, and LSP servers. Pi has `pi config` for enabling/disabling package resources, but there is no documented Pi equivalent of Claude Code's marketplace-add workflow, plugin detail inventory, or `/reload-plugins` token-cost warning. Pi extension directories can be hot-reloaded with `/reload`, according to the extension docs.

## Discovery and Precedence

Pi resolves package and resource sources in several layers:

1. Project and user `packages` settings are read; project package entries come first.
2. Package identities are deduplicated so the project entry wins over the user entry for the same npm package name, git repository URL without ref, or resolved local absolute path.
3. Package resources are collected from the `pi` manifest or conventional directories.
4. Project then user resource arrays in settings are resolved.
5. Project and user auto-discovered resources are added.
6. The final resource list is sorted by precedence and canonical duplicate paths are removed.

The locally installed Pi 0.72.1 source ranks resources in this order: project settings entries, project auto-discovered entries, user settings entries, user auto-discovered entries, then package resources. Since name collision handling keeps the first resource in several loaders, local project/user resources can shadow package resources.

Name and command exposure is not package-namespaced by default:

| Resource | Pi name behavior |
| --- | --- |
| Extension commands | The extension chooses the command name with `pi.registerCommand("name", ...)`, exposed as `/name` |
| Skills | Registered as `/skill:name` when skill commands are enabled |
| Prompt templates | Filename without `.md`, exposed as `/name` |
| Themes | Theme file/name selected in settings |

This is an important portability difference from Claude Code. Claude Code plugin skills are always namespaced by plugin name, such as `/commit-commands:commit`, specifically to prevent conflicts. Pi's packages rely on resource precedence, diagnostics, and author naming discipline instead of a package namespace.

Conflict behavior depends on resource type. Pi skill validation warns on name collisions and keeps the first skill found. Extension conflicts for tools, commands, and flags are reported as diagnostics while all extensions remain loaded and precedence follows load order. Package identity deduplication uses npm package name, git repository URL without ref, or resolved absolute local path.

## Security and Runtime Behavior

Pi packages are high-trust. The package docs warn that packages run with full system access: extensions execute arbitrary code, and skills can instruct the model to perform any action including running executables. The extension docs repeat that extensions run with full user permissions and can execute arbitrary code.

Pi itself has no built-in sandbox. The security docs state that Pi runs with the permissions of the user account that starts it and treats files writable by that user as inside the same local trust boundary. The containerization docs recommend either running the whole `pi` process in an isolated environment or running Pi on the host while routing tool execution into an isolated environment. They also note that extensions run wherever the Pi process runs; if a host Pi uses a tool-routing extension, other custom extension tools still run on the host unless they also delegate their operations.

There is no built-in permission popup system. The home page explicitly lists permission popups among features Pi did not build; users can run in a container or build confirmation flows with extensions. Extensions can intercept `tool_call` and block dangerous operations, which is powerful but means the permission mechanism is package code, not a provider-enforced sandbox.

Credential exposure follows process privileges. TypeScript extensions run in-process and can read environment variables and files available to the user. Git package sources use configured SSH keys and git credentials. The Pi package docs recommend `GIT_TERMINAL_PROMPT=0` and `GIT_SSH_COMMAND` settings for non-interactive git installs so credential prompts fail fast.

Claude Code is also explicit that plugins and marketplaces are highly trusted and can execute arbitrary code with user privileges. However, Claude Code adds marketplace policy controls such as `strictKnownMarketplaces` and `strictPluginOnlyCustomization`, while Pi's docs do not describe an equivalent organizational allowlist for package sources.

## Distribution

Pi distribution is source-based:

| Channel | Behavior |
| --- | --- |
| Official gallery | `https://pi.dev/packages` indexes packages and shows install commands such as `pi install npm:pi-web-access` |
| npm | Public npm packages; private registries are available through npm configuration |
| git | GitHub shorthand, HTTPS, SSH, and protocol URLs |
| local | Local files or directories, useful for development and private sharing |

The gallery supports filtering by package type such as extension, skill, theme, prompt, and package. It is not a marketplace that must be registered in the CLI. Publishing is normal npm/git publishing plus a Pi package shape; the docs recommend adding the `pi-package` keyword for discoverability and optional `pi.video` or `pi.image` metadata for gallery previews.

Claude Code distribution is marketplace-centric. A marketplace is a catalog users add before installing individual plugins; sources can be GitHub repositories, other git hosts, local paths, remote marketplace JSON URLs, npm packages, or private/internal repositories. Claude Code's official marketplace is automatically available, its community marketplace is manually added, and organizations can configure marketplace restrictions and auto-installs. Pi's gallery is simpler: it points users at package source strings consumed directly by `pi install`.

## Portability

For Claudine, a Pi package should be treated as two things:

1. A Pi-native unit that can be linked or recorded as a source string for Pi.
2. A bundle that may contain portable subresources.

Portable subresources include Agent Skills, passive skill assets, prompt template Markdown, theme JSON only when the target provider supports comparable themes, and scripts/assets that are inert until a skill asks the model to use them. Non-portable or high-risk resources include TypeScript extensions, extension dependencies, custom tools, event handlers, extension-registered commands, extension-registered CLI flags, provider-specific package metadata, package source identities, credentials, and any extension-mediated MCP or subagent implementation.

Claudine should not blindly extract a Pi extension and present it as a safe cross-provider script. Pi extensions are executable code with access to the Pi runtime API and user environment. If Claudine links Pi resources to Claude Code, Codex, or another provider, it should either preserve the whole Pi package as Pi-only metadata or extract only standards-shaped resources and mark the extension code as non-portable.

Rewrites are needed for command naming. Pi prompt templates become `/filename`; Pi skills can be invoked through `/skill:name` when skill commands are enabled; Claude Code plugin skills use `/plugin-name:skill-name`. A Claudine exporter should not assume Pi package resources carry a namespace.

## Claudine Linking Notes

Implementation guidance:

- Add Pi package containers as a first-class provider resource type distinct from Claude Code plugins.
- Capture package source strings, scope, filters, and install roots from `settings.json` rather than assuming every resource exists under `~/.pi/agent`.
- Preserve `package.json` and the `pi` manifest when linking a Pi package as a Pi unit.
- Extract `skills/`, `prompts/`, and `themes/` only when the target provider has a compatible resource type.
- Mark TypeScript extensions and extension dependencies as non-portable executable assets.
- Model precedence so project/local resources can shadow package resources; do not import Claude Code's plugin namespace rules into Pi.
- Respect project trust: do not read or link project `.pi` package resources as trusted runtime resources without recording that they are project-trust-gated.
- For npm/git sources, record whether the source is pinned. Unpinned package links carry update risk.

This research implies Claudine metadata changes are needed if Pi is added to plugin linking: the schema should represent `package.json` `pi` manifests, source strings (`npm:`, `git:`, local path), package filters, and the distinction between Pi-native package linking and portable resource extraction.

## Sources

- [Pi Documentation](https://pi.dev/docs/latest)
- [Pi Packages](https://pi.dev/docs/latest/packages)
- [Pi Extensions](https://pi.dev/docs/latest/extensions)
- [Pi Skills](https://pi.dev/docs/latest/skills)
- [Pi Prompt Templates](https://pi.dev/docs/latest/prompt-templates)
- [Pi Themes](https://pi.dev/docs/latest/themes)
- [Pi Settings](https://pi.dev/docs/latest/settings)
- [Pi Security](https://pi.dev/docs/latest/security)
- [Pi Containerization](https://pi.dev/docs/latest/containerization)
- [Pi Package Catalog](https://pi.dev/packages)
- [Claude Code: Discover and install prebuilt plugins](https://code.claude.com/docs/en/discover-plugins)
- [Claude Code: Create plugins](https://code.claude.com/docs/en/plugins)
- [Claude Code: Create and distribute a plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces)
- [Claude Code Settings](https://code.claude.com/docs/en/settings)
- Local inspection: `pi --help`, `pi install --help`, `pi update --help`, `pi remove --help`, `pi list --help`, `/Users/ken/.pi/agent/settings.json`, and the locally installed Pi 0.72.1 package source under `/Users/ken/.bun/install/cache/@mariozechner/pi-coding-agent@0.72.1@@@1/`.
