---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://moonshotai.github.io/kimi-code/
docs: https://moonshotai.github.io/kimi-code/
plugin_docs: https://moonshotai.github.io/kimi-code/en/customization/plugins

support: partial

locations:
  - os: macos
    scope: user
    path: "$KIMI_CODE_HOME/plugins/"
    notes: Default ~/.kimi-code/plugins/. Contains installed.json and managed/<id>/. This host had no plugins directory because no plugins are installed.
  - os: linux
    scope: user
    path: "$KIMI_CODE_HOME/plugins/"
    notes: Default ~/.kimi-code/plugins/.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.kimi-code\plugins\'
    notes: Default Windows user plugin root.
  - os: macos
    scope: user
    path: "$KIMI_CODE_HOME/plugins/installed.json"
    notes: Records installed plugins, enabled state, and MCP server capability toggles.
  - os: linux
    scope: user
    path: "$KIMI_CODE_HOME/plugins/installed.json"
    notes: Plugin registry.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.kimi-code\plugins\installed.json'
    notes: Plugin registry.
  - os: macos
    scope: user
    path: "$KIMI_CODE_HOME/plugins/managed/<id>/"
    notes: Managed copy of each installed plugin. Local source directories are copied here; the CLI runs from this copy.
  - os: linux
    scope: user
    path: "$KIMI_CODE_HOME/plugins/managed/<id>/"
    notes: Managed plugin copy.
  - os: windows
    scope: user
    path: '%USERPROFILE%\.kimi-code\plugins\managed\<id>\'
    notes: Managed plugin copy.
  - os: macos
    scope: marketplace
    path: "https://code.kimi.com/kimi-code/plugins/marketplace.json"
    notes: Default marketplace URL. Observed to redirect to cdn.kimi.com. Override with KIMI_CODE_PLUGIN_MARKETPLACE_URL.
  - os: linux
    scope: marketplace
    path: "https://code.kimi.com/kimi-code/plugins/marketplace.json"
    notes: Default marketplace URL.
  - os: windows
    scope: marketplace
    path: "https://code.kimi.com/kimi-code/plugins/marketplace.json"
    notes: Default marketplace URL.

manifest:
  file_names:
    - kimi.plugin.json
    - .kimi-plugin/plugin.json
  format: json
  required_fields:
    - name
  optional_fields:
    - version
    - description
    - keywords
    - author
    - homepage
    - license
    - interface
    - interface.displayName
    - interface.shortDescription
    - interface.longDescription
    - interface.developerName
    - interface.websiteURL
    - skills
    - sessionStart
    - sessionStart.skill
    - skillInstructions
    - mcpServers
    - hooks
    - commands
  package_layout: |
    Plugin root contains one of the manifest files and component directories/files at the root:
    - skills/<skill-name>/SKILL.md for bundled Agent Skills; root SKILL.md is treated as a single skill when skills is omitted
    - commands/<command-name>.md for slash commands
    - hooks/ or inline hook rules referenced by the manifest
    - MCP server declarations in the manifest mcpServers field
    - README.md and LICENSE as optional documentation
    Manifest paths for skills and commands must be relative to the plugin root, start with ./, and stay within the plugin root.
  notes: |
    When both kimi.plugin.json and .kimi-plugin/plugin.json exist, kimi.plugin.json takes precedence. The plugin id must match [a-z0-9][a-z0-9_-]{0,63}. Unsupported runtime fields such as tools, apps, inject, and configFile appear as diagnostics in /plugins info and are ignored.

lifecycle:
  install: |
    TUI slash command: /plugins install <path-or-url>, or browse /plugins tabs (Installed / Official / Third-party / Custom). Sources include marketplace entries, GitHub repository URLs, zip URLs, and local directories. The source is copied to $KIMI_CODE_HOME/plugins/managed/<id>/ and recorded in installed.json. Third-party installs show a confirmation prompt that defaults to cancel.
  update: |
    The Installed tab in /plugins shows an update badge when a newer marketplace version exists; pressing Enter installs it. There is no documented standalone CLI update subcommand. Individual plugins such as Kimi Datasource do not auto-update.
  remove: |
    TUI slash command: /plugins remove <id>. Removes the installation record from installed.json; the managed copy on disk is not deleted.
  enable_disable: |
    TUI slash command: /plugins enable|disable <id>, or Space in the Installed tab. State is written to installed.json. Changes require /reload or a new session to take effect.
  trust: |
    No separate signature or sandbox layer. Trust is established by the install action and a source tier badge: kimi-official, curated, or third-party. Third-party sources require an explicit confirmation. Paths in the manifest are resolved and must remain inside the plugin root after symlink resolution.
  versioning: |
    The manifest may declare a version string. Marketplace entries include a version. GitHub installs support branch, tag, release, and commit URLs for pinning. installed.json records the installed state.
  notes: |
    Plugin management is TUI-only via slash commands; the host CLI (v0.14.0) has no kimi plugin subcommand. Plugins are installed per-user and apply to all projects; project-scope installation is not supported.

packaged_resources:
  skills: full
  scripts: partial
  slash_commands: full
  subagents: none
  mcp_servers: full
  hooks: full
  prompts: none
  config: partial
  assets: full
  other:
    - session_start_skill
    - skill_instructions
    - interface_metadata

discovery:
  mechanism: |
    Kimi Code reads $KIMI_CODE_HOME/plugins/installed.json at startup, loads enabled plugins from plugins/managed/<id>/, and registers skills, commands, MCP servers, hooks, and sessionStart skill. /reload or starting a new session refreshes the plugin inventory.
  precedence: |
    Plugin slash commands are namespaced as <plugin>:<command>. Plugin MCP servers are merged with user MCP servers; project-level .kimi-code/mcp.json overrides user-level declarations. There is no documented shadowing rule for plugin skills versus user skills.
  namespacing: |
    Slash commands are registered as <plugin>:<command> and invoked with /<plugin>:<command>. Skills are exposed by their declared name field and invoked with /skill:<name>; the docs do not state an automatic plugin prefix for skill names.
  conflicts: |
    Plugin ids must match the documented grammar and must be unique within installed.json. Command name collisions are prevented by the <plugin>: prefix. Skill name collisions are not explicitly documented.
  notes: |
    Plugin changes are not hot-reloaded in the current session; /reload or /new is required. Broken manifests and unsafe paths appear in /plugins info diagnostics and do not crash the session.

security:
  trust_model: |
    Plugins are highly trusted once installed. Kimi does not sign or sandbox plugin bundles. The trust boundary is the install/enable action plus the tier badge and third-party confirmation prompt.
  permissions: |
    Plugin MCP servers require the same per-server approval as user MCP servers and can be disabled via /plugins mcp disable. Hooks run with the user's OS privileges. sessionStart.skill only injects text; it does not execute code.
  sandbox_interaction: |
    Plugin hooks and MCP servers run with the user's OS privileges, not inside a Kimi Code sandbox. Plugin slash commands are prompt templates, not executables. The security model explicitly states that command-type plugin tools and legacy runtimes are not executed.
  credential_access: |
    Plugin hooks receive KIMI_CODE_HOME and KIMI_PLUGIN_ROOT environment variables. MCP server configs can reference bearerTokenEnvVar to read a user environment variable. Plugins do not have direct access to Kimi Code credentials stored in credentials/.
  update_risk: |
    Medium. The marketplace shows update badges but plugins do not silently auto-update by default. Pinning a GitHub install to a commit or tag reduces silent-change risk.
  notes: |
    The security model confines manifest paths to the plugin root after symlink resolution. Broken manifests are surfaced as diagnostics rather than blocking the session.

distribution:
  marketplace: true
  registry_url: https://code.kimi.com/kimi-code/plugins/marketplace.json
  source_types:
    - marketplace
    - github repo
    - zip url
    - local directory
    - custom marketplace json
  publishing: |
    No documented self-serve publishing flow. The official marketplace is curated by Kimi. Teams can host a custom marketplace by publishing a JSON file with version and plugins entries and pointing KIMI_CODE_PLUGIN_MARKETPLACE_URL at it.
  private_distribution: |
    GitHub URLs and custom marketplace JSON can reference private repositories or internal URLs, but the docs do not specify required tokens or credential helpers for private GitHub installs. Local path installs work without network access.
  notes: |
    Observed marketplace.json uses version "1" and plugins entries with id, tier, displayName, version, description, keywords, and source. source can be a relative path such as ./official/kimi-datasource.zip or a GitHub URL.

portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources:
    - skills
    - slash_commands
    - assets
    - README.md
  non_portable_assets:
    - kimi.plugin.json manifest
    - .kimi-plugin/plugin.json manifest
    - mcpServers declarations
    - hooks and hook commands
    - sessionStart.skill reference
    - skillInstructions
    - interface metadata
    - KIMI_PLUGIN_ROOT path references
  rewrite_needed: true
  notes: |
    Claudine should extract Markdown-based skills and commands and rewrite the slash-command namespace (<plugin>:<command>) into the target provider's format. MCP server declarations, hook commands, sessionStart.skill, skillInstructions, and KIMI_PLUGIN_ROOT references are Kimi-specific and must be rewritten or omitted.

cli_params:
  - flag: /plugins
    description: Open the interactive plugin manager in the TUI. Not a shell CLI subcommand.
    example: /plugins
  - flag: /plugins list
    description: List installed plugins.
    example: /plugins list
  - flag: /plugins install <path-or-url>
    description: Install from a local directory, zip URL, or GitHub repository URL.
    example: /plugins install https://github.com/obra/superpowers
  - flag: /plugins marketplace [source]
    description: Browse the official marketplace or a custom marketplace JSON path/URL.
    example: /plugins marketplace
  - flag: /plugins info <id>
    description: Show plugin details and diagnostics.
    example: /plugins info kimi-datasource
  - flag: /plugins enable|disable <id>
    description: Enable or disable an installed plugin.
    example: /plugins enable kimi-datasource
  - flag: /plugins remove <id>
    description: Remove a plugin from installed.json.
    example: /plugins remove kimi-datasource
  - flag: /plugins reload
    description: Reload installed.json and all plugin manifests.
    example: /plugins reload
  - flag: /plugins mcp enable|disable <id> <server>
    description: Enable or disable an MCP server declared by a plugin.
    example: /plugins mcp disable kimi-datasource finance
  - flag: /reload
    description: Apply plugin changes in the current or new session.
    example: /reload

env_vars:
  - name: KIMI_CODE_HOME
    effect: Overrides the data root directory (default ~/.kimi-code). All plugin paths (plugins/, skills/, AGENTS.md) move with this variable.
  - name: KIMI_CODE_PLUGIN_MARKETPLACE_URL
    effect: Override the plugin marketplace JSON loaded by /plugins. Accepts https://, http://, file:// URLs, and local paths.

gaps:
  - No kimi plugin shell subcommand exists in v0.14.0; management is TUI-only via slash commands.
  - Project-scope or repo-scope plugin installation is not supported.
  - No documented standalone CLI update command for plugins.
  - No documented deconfliction behavior for skill-name collisions between plugins or between plugin and user skills.
  - No code signing, sandboxing, or reproducible-build documentation.
  - No documented credential helper requirements for private GitHub plugin installs.

changes: []

requires_claudine_update: true
reason: |
  Claudine should model Kimi Code's plugin container: $KIMI_CODE_HOME/plugins/installed.json, plugins/managed/<id>/, the kimi.plugin.json / .kimi-plugin/plugin.json manifest, namespaced slash commands, and plugin-declared MCP servers and hooks. Linking should extract portable Markdown resources (skills, commands, assets) while flagging non-portable assets (manifest, MCP configs, hooks, sessionStart.skill, skillInstructions, KIMI_PLUGIN_ROOT references) as requiring rewrite or omission.
---

# Kimi Code CLI Plugins

## Overview

Kimi Code CLI has a documented plugin system that packages reusable capabilities into installable units. A plugin is a directory or zip file containing a JSON manifest and optional component directories for [Agent Skills](https://moonshotai.github.io/kimi-code/en/customization/skills), slash commands, MCP servers, and lifecycle hooks. Plugins are installed per-user, managed under `$KIMI_CODE_HOME/plugins/`, and controlled through TUI slash commands such as `/plugins`.

Compared with [Claude Code](https://code.claude.com/docs/en/plugins), Kimi's plugin model is younger and smaller in scope, with important differences in command surface, scope, and packaging:

| Area | Kimi Code CLI | Claude Code |
|---|---|---|
| Manifest file | `kimi.plugin.json` or `.kimi-plugin/plugin.json` | `.claude-plugin/plugin.json` |
| Management UI | TUI slash commands (`/plugins ...`) | CLI subcommands (`claude plugin ...`) plus TUI slash commands |
| Install scope | User only | User, project, local, managed |
| Managed copy | `$KIMI_CODE_HOME/plugins/managed/<id>/` | `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/` |
| Skill namespacing | Skills exposed by declared `name`; no documented plugin prefix | Plugin skills namespaced as `plugin-name:skill-name` |
| Command namespacing | `<plugin>:<command>` | `<plugin>:<command>` |
| Project-scope plugins | Not supported | Supported via `.claude/settings.json` and `.claude/skills/` |
| Subagents in plugins | Not supported | Supported via `agents/` |

Sources: [Kimi Code Plugins](https://moonshotai.github.io/kimi-code/en/customization/plugins), [Claude Code Plugins](https://code.claude.com/docs/en/plugins), [Claude Code Plugins Reference](https://code.claude.com/docs/en/plugins-reference).

This host's installed binary is `kimi 0.14.0` at `/Users/ken/.kimi-code/bin/kimi`. Running `kimi --help` lists no `plugin` subcommand, and `kimi plugin` returns "unknown command 'plugin'". No `~/.kimi-code/plugins/` directory exists because no plugins have been installed.

## Installation and Locations

Plugin state lives under the Kimi data root. The default layout is documented in [Data locations](https://moonshotai.github.io/kimi-code/en/configuration/data-locations):

| Path | Purpose |
|---|---|
| `$KIMI_CODE_HOME/plugins/installed.json` | Registry of installed plugins, enabled state, and MCP capability overrides |
| `$KIMI_CODE_HOME/plugins/managed/<id>/` | Managed copy of each installed plugin |
| `$KIMI_CODE_HOME/skills/` | User-level Kimi-specific skills (separate from plugins) |
| `~/.agents/skills/` | Cross-tool user skills |
| `.kimi-code/skills/` | Project-level skills |

`$KIMI_CODE_HOME` defaults to `~/.kimi-code` and can be overridden per [Environment variables](https://moonshotai.github.io/kimi-code/en/configuration/env-vars). Plugins are currently installed only at user scope; project-scope installation is not supported.

## Manifest and Package Format

The plugin manifest can be placed at either `kimi.plugin.json` or `.kimi-plugin/plugin.json`; if both exist, `kimi.plugin.json` takes precedence. The only required field is `name`, which must match `[a-z0-9][a-z0-9_-]{0,63}` and serves as the plugin id.

Example from the docs:

```json
{
  "name": "kimi-finance",
  "version": "1.0.0",
  "description": "Finance data and analysis workflows for Kimi Code CLI",
  "skills": "./skills/",
  "sessionStart": {
    "skill": "using-finance"
  },
  "interface": {
    "displayName": "Kimi Finance",
    "shortDescription": "Market data and financial analysis workflows"
  }
}
```

Supported fields include metadata (`version`, `description`, `keywords`, `author`, `homepage`, `license`), `interface` display fields, `skills` paths, `sessionStart.skill`, `skillInstructions`, `mcpServers`, `hooks`, and `commands`. Component paths must be relative to the plugin root, start with `./`, and stay within the plugin root.

Unsupported runtime fields such as `tools`, `apps`, `inject`, and `configFile` are reported as diagnostics and ignored.

## Packaged Resources

Kimi plugins can contain the following resource types:

| Resource | Support | Location | Notes |
|---|---|---|---|
| Agent Skills | Full | `skills/<name>/SKILL.md` or root `SKILL.md` | Uses the same SKILL.md format as standalone skills |
| Slash commands | Full | `commands/*.md` | Added in v0.21.0; namespaced as `<plugin>:<command>` |
| MCP servers | Full | Inline `mcpServers` manifest field | Enabled by default; can be toggled per server |
| Hooks | Full | Inline `hooks` manifest field | Added in v0.20.1; active only while plugin is enabled |
| Scripts | Partial | Referenced by hook `command` or MCP `command` | No automatic script discovery; must be invoked explicitly |
| Session-start skill | Full | `sessionStart.skill` field | Loads a skill into the main agent at session start |
| Skill instructions | Full | `skillInstructions` field | Appended whenever any plugin skill is loaded |
| Assets | Full | README, LICENSE, referenced files | Static files referenced by skills or hooks |
| Subagents | None | — | Plugins cannot define subagents |
| Prompts | None | — | Skills and commands serve this role |
| Config | Partial | `interface`, `sessionStart`, `skillInstructions` | No standalone plugin config file |

## Lifecycle and Trust

Install: `/plugins install <path-or-url>` or browse the `/plugins` tabs. The source is copied to `$KIMI_CODE_HOME/plugins/managed/<id>/` and recorded in `installed.json`.

Update: The Installed tab shows an update badge when a newer marketplace version is available; pressing Enter installs it. There is no standalone CLI `update` subcommand.

Remove: `/plugins remove <id>` removes the installation record from `installed.json` but leaves the managed copy on disk.

Enable / disable: `/plugins enable|disable <id>` or `Space` in the Installed tab. State is persisted in `installed.json`. Changes require `/reload` or a new session.

Trust: Trust is established by install/enable and a source tier badge (`kimi-official`, `curated`, `third-party`). Third-party installs show a confirmation prompt that defaults to cancel. There is no code signing or sandbox layer.

Versioning: `version` is optional in the manifest. Marketplace entries include a version. GitHub installs can be pinned to a branch, tag, release, or commit.

## Discovery and Precedence

At startup Kimi Code reads `plugins/installed.json`, loads enabled plugins from `plugins/managed/<id>/`, and registers their skills, commands, MCP servers, hooks, and `sessionStart` skill. The `/reload` command refreshes the plugin inventory.

Slash commands are namespaced as `<plugin>:<command>`. Skills are exposed by their declared `name` and invoked with `/skill:<name>`; the docs do not describe an automatic plugin prefix for skill names, which differs from Claude Code's explicit `plugin-name:skill-name` namespacing.

Project-level `.kimi-code/mcp.json` overrides user-level MCP server declarations for ordinary MCP configuration; plugin MCP servers are managed separately through `/plugins`.

## Security and Runtime Behavior

Plugins are highly trusted after installation. Kimi does not sign or sandbox plugin bundles. Plugin hooks and MCP servers run with the user's OS privileges. Plugin slash commands are prompt templates, not executables.

The manifest path resolver requires that all paths remain within the plugin root after symlink resolution. Broken manifests and unsafe paths appear in `/plugins info <id>` diagnostics and do not affect other sessions.

Plugin MCP servers require the same per-server approval as user MCP servers. Hooks receive `KIMI_CODE_HOME` and `KIMI_PLUGIN_ROOT` environment variables. MCP server configs can use `bearerTokenEnvVar` to read a user environment variable.

## Distribution

Kimi Code has an official marketplace at `https://code.kimi.com/kimi-code/plugins/marketplace.json` (observed to redirect to `cdn.kimi.com`). The marketplace JSON contains a `version` field and a `plugins` array; each entry has `id`, `tier`, `displayName`, `version`, `description`, `keywords`, and `source`. `source` can be a relative path such as `./official/kimi-datasource.zip` or a GitHub URL.

Supported install sources include the official marketplace, GitHub repository URLs, zip URLs, local directories, and custom marketplace JSON files pointed to by `KIMI_CODE_PLUGIN_MARKETPLACE_URL`. There is no documented self-serve publishing process; the official marketplace appears to be curated by Kimi.

## Portability

Claudine should not link Kimi plugins as intact units. Extract portable Markdown resources and flag Kimi-specific assets for rewrite or omission.

Portable resources:
- `skills/<name>/SKILL.md` and root `SKILL.md`
- `commands/*.md`
- Static assets and `README.md`

Non-portable assets:
- `kimi.plugin.json` / `.kimi-plugin/plugin.json` manifest
- `mcpServers` declarations
- `hooks` hook rules and executable commands
- `sessionStart.skill` reference and `skillInstructions`
- `interface` metadata
- `KIMI_PLUGIN_ROOT` references

Rewrite is needed for namespaced slash commands (`<plugin>:<command>`), MCP server configs, hook commands, and any plugin-root-relative executable paths.

## Claudine Linking Notes

- Treat `$KIMI_CODE_HOME/plugins/installed.json` as the canonical plugin registry.
- For each enabled plugin, read its managed copy from `plugins/managed/<id>/`.
- Parse either `kimi.plugin.json` or `.kimi-plugin/plugin.json`, preferring `kimi.plugin.json`.
- Extract portable Markdown skills and commands; rewrite command invocations from `<plugin>:<command>` to the target provider's namespace format.
- Do not extract or link `mcpServers`, `hooks`, `sessionStart.skill`, `skillInstructions`, or `interface` metadata without provider-specific rewriting.
- Respect the marketplace source tier (`kimi-official`, `curated`, `third-party`) when deciding whether to suggest linking a plugin.
- Because plugin management is TUI-only on this CLI version, Claudine should not expect a `kimi plugin list` or similar shell subcommand.

## Sources

- [Kimi Code CLI — Plugins](https://moonshotai.github.io/kimi-code/en/customization/plugins)
- [Kimi Code CLI — Agent Skills](https://moonshotai.github.io/kimi-code/en/customization/skills)
- [Kimi Code CLI — Hooks](https://moonshotai.github.io/kimi-code/en/customization/hooks)
- [Kimi Code CLI — MCP](https://moonshotai.github.io/kimi-code/en/customization/mcp)
- [Kimi Code CLI — Data Locations](https://moonshotai.github.io/kimi-code/en/configuration/data-locations)
- [Kimi Code CLI — Environment Variables](https://moonshotai.github.io/kimi-code/en/configuration/env-vars)
- [Kimi Code CLI — kimi Command Reference](https://moonshotai.github.io/kimi-code/en/reference/kimi-command)
- [Kimi Code CLI — Changelog](https://moonshotai.github.io/kimi-code/en/release-notes/changelog)
- [Kimi Code CLI Marketplace JSON](https://code.kimi.com/kimi-code/plugins/marketplace.json)
- [Claude Code — Plugins](https://code.claude.com/docs/en/plugins)
- [Claude Code — Plugins Reference](https://code.claude.com/docs/en/plugins-reference)
