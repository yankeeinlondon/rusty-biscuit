---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://github.com/QwenLM/qwen-code
docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
plugin_docs: https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.qwen/extensions/<name>/
    notes: Installed extension root. Contains qwen-extension.json, .qwen-extension-install.json, .env, commands/, skills/, agents/, hooks/, and any bundled executables or QWEN.md. Observed host has no installed extensions.
  - os: linux
    scope: user
    path: ~/.qwen/extensions/<name>/
    notes: Same layout as macOS; confirmed by Qwen Code source and documentation.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\extensions\\<name>\\"
    notes: Windows resolves ~ to %USERPROFILE%; layout mirrors Unix.
  - os: macos
    scope: user
    path: ~/.qwen/extensions/extension-enablement.json
    notes: Enable/disable overrides per extension and per scope. Defaults to enabled on install.
  - os: linux
    scope: user
    path: ~/.qwen/extensions/extension-enablement.json
    notes: Enablement registry.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\extensions\\extension-enablement.json"
    notes: Enablement registry.
  - os: macos
    scope: user
    path: ~/.qwen/extensions/<name>/.env
    notes: Non-sensitive extension setting values at user scope. Sensitive values are stored in the OS keychain.
  - os: linux
    scope: user
    path: ~/.qwen/extensions/<name>/.env
    notes: User-scope extension settings.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\extensions\\<name>\\.env"
    notes: User-scope extension settings.
  - os: macos
    scope: repo
    path: .qwen/.env
    notes: Workspace-scoped extension setting values when --scope workspace/project is used. Sensitive values still go to the keychain.
  - os: linux
    scope: repo
    path: .qwen/.env
    notes: Workspace-scoped extension settings.
  - os: windows
    scope: repo
    path: ".qwen\\.env"
    notes: Workspace-scoped extension settings.
  - os: macos
    scope: repo
    path: .qwen/
    notes: Project-level Qwen Code configuration directory. Extensions themselves are not stored here; only workspace-scoped enablement and settings are referenced from here.
  - os: linux
    scope: repo
    path: .qwen/
    notes: Project-level Qwen Code configuration directory.
  - os: windows
    scope: repo
    path: .qwen\\
    notes: Project-level Qwen Code configuration directory.

manifest:
  file_names:
    - qwen-extension.json
    - gemini-extension.json
    - .claude-plugin/plugin.json
    - .claude-plugin/marketplace.json
  format: json
  required_fields:
    - name
    - version
  optional_fields:
    - mcpServers
    - lspServers
    - hooks
    - channels
    - contextFileName
    - commands
    - skills
    - agents
    - settings
    - excludeTools
  package_layout: |
    Native Qwen Code extension root contains qwen-extension.json plus component directories:
    - commands/*.md (or legacy *.toml) for slash commands
    - skills/<skill-name>/SKILL.md for Agent Skills
    - agents/*.md or *.yaml for subagent definitions
    - hooks/hooks.json for lifecycle hooks (or an inline hooks object in qwen-extension.json)
    - QWEN.md or a file named by contextFileName for persistent extension context
    - compiled JS/TS entry points referenced by mcpServers or channels
    - .env for non-sensitive settings (written by Qwen Code after install)
    - .qwen-extension-install.json for install metadata (source, type, version, marketplace info)
    For Claude plugin installs, .claude-plugin/plugin.json and marketplace.json are converted to qwen-extension.json and the resources above are copied/rewritten. For Gemini extension installs, gemini-extension.json is converted similarly.
  notes: |
    Manifest paths for commands/skills/agents can be a single relative path or an array of paths; defaults are commands/, skills/, and agents/. Qwen Code substitutes ${extensionPath}, ${workspacePath}, ${/}, and ${pathSeparator} in qwen-extension.json. The contextFileName field defaults to QWEN.md if a QWEN.md exists. Extension names must match /^[a-zA-Z0-9-_.]+$/.

lifecycle:
  install: |
    qwen extensions install <source> [--ref <git-ref>] [--auto-update] [--pre-release] [--registry <url>] [--consent] [--scope user|project]. Source may be a git URL, GitHub owner/repo shorthand, local path, local .zip/.tar.gz archive, archive URL, scoped npm package (@scope/name[@version]), or a Claude marketplace source (marketplace-url:plugin-name). The CLI clones/copies/extracts the source into ~/.qwen/extensions/<name>/, writes .qwen-extension-install.json, prompts for any declared settings, and enables the extension at user scope by default (or project scope if requested).
  update: |
    qwen extensions update [<name>] [--all]. Git, npm, archive-URL, and local-path extensions can be updated. npm extensions pinned to an exact version are considered up-to-date; dist-tag pins track that tag. Linked extensions do not need updating. Auto-update is opt-in via --auto-update at install time.
  remove: |
    qwen extensions uninstall <name> deletes the extension directory under ~/.qwen/extensions/, removes its enablement entry, and refreshes the tool registry.
  enable_disable: |
    qwen extensions enable|disable <name> [--scope user|workspace]. Enablement is stored in ~/.qwen/extensions/extension-enablement.json as path overrides. By default an installed extension is enabled everywhere; workspace scope can restrict it to the current project. System scope is not supported.
  trust: |
    Trust is established by explicit install/enable action. The CLI requires --consent or an interactive confirmation. There is no code signing or separate sandbox boundary for extensions. Workspace trust is required to install from a local path. Extension MCP server configs have their trust field stripped, so they require the same per-server approval as user MCP servers.
  versioning: |
    Version is read from qwen-extension.json version. npm sources resolve the requested dist-tag/version; git sources respect --ref; otherwise HEAD is used. The install metadata records the resolved version/releaseTag. No automatic updates occur unless --auto-update was set.
  notes: |
    Interactive /extensions slash commands support hot-reload, while qwen extensions CLI changes require restarting the Qwen Code session. The local host (qwen 0.15.6) has no installed extensions.

packaged_resources:
  skills: full
  scripts: partial
  slash_commands: full
  subagents: full
  mcp_servers: full
  hooks: full
  prompts: none
  config: partial
  assets: partial
  other:
    - lsp_servers
    - channels
    - excludeTools

discovery:
  mechanism: |
    On startup the ExtensionManager scans ~/.qwen/extensions/ subdirectories, reads qwen-extension.json, resolves active state from extension-enablement.json, and honors the top-level --extensions/-e override list. In bare mode (--bare or QWEN_CODE_SIMPLE) only explicitly requested extensions are loaded. Loaded extensions register commands, skills, agents, hooks, MCP servers, LSP servers, and channels.
  precedence: |
    User/project custom commands override extension commands. User/project agents override extension agents (loadSubagent checks user, project, extension, builtin in that order). User settings.json MCP servers override extension mcpServers by name. Hooks run in source priority order: project (1), user (2), system (3), extensions (4). LSP configs apply extension configs first, then user configs, with user overriding by name.
  namespacing: |
    Extension slash commands keep their natural name when no conflict exists; when a user/project command has the same name, the extension command is renamed to extensionName.commandName (e.g. gcp.deploy). Skills are tagged with their extension source label but are not namespaced by default. Agents and MCP/LSP servers are not prefixed; conflicts are resolved by precedence or override.
  conflicts: |
    Command conflicts are resolved by renaming the extension command. Skill/agent conflicts are resolved by first-found-wins (user before project before extension). MCP/LSP conflicts are resolved by same-name override, with user config winning. Exact extension names must be unique in ~/.qwen/extensions/.
  notes: |
    The --extensions/-e flag restricts which extensions load. --disabled-slash-commands and QWEN_DISABLED_SLASH_COMMANDS can hide specific commands. --bare disables implicit extension discovery.

security:
  trust_model: |
    Extensions are highly trusted: installation/enablement is the trust boundary. Qwen Code does not sign or verify extension bundles. Users must trust the source (git repo, npm package, local path, archive, or Claude/Gemini marketplace). The install command requires explicit consent.
  permissions: |
    Extension commands become slash commands that execute within the agent loop. Extension MCP servers are subject to Qwen Code's approval mode and cannot pre-set trust. Hooks can invoke shell commands and HTTP callbacks. excludeTools can disable tools for the session. Settings declared in qwen-extension.json are prompted at install; sensitive values go to the OS keychain.
  sandbox_interaction: |
    If Qwen Code is launched with --sandbox or QWEN_SANDBOX, model-generated shell commands run inside sandbox-exec (macOS) or a Docker/Podman container, which also applies to spawned MCP/LSP/hook processes. By default no sandbox is used. There is no plugin-specific sandbox.
  credential_access: |
    Extension settings can request sensitive values that are stored in the OS keychain and passed as environment variables to MCP servers. Extension manifests can reference ${extensionPath}, ${workspacePath}, and path separators. Spawned MCP/LSP/hook commands inherit the user's environment, so extensions can read env vars indirectly.
  update_risk: |
    Low to medium by default because updates are explicit (qwen extensions update). Extensions installed with --auto-update or linked extensions can change behavior silently. npm dist-tag pins and git refs can limit drift; exact npm version pins are treated as up-to-date.
  notes: |
    No signature verification or reproducible-build checks. Marketplace plugins converted from Claude Code or Gemini inherit the code of the upstream plugin. Workspace trust is required before installing from local paths.

distribution:
  marketplace: true
  registry_url: https://claudemarketplaces.com/
  source_types:
    - git repository
    - github owner/repo shorthand
    - github release
    - local folder
    - local .zip/.tar.gz archive
    - archive URL
    - scoped npm package
    - claude marketplace (marketplace-url:plugin-name)
    - gemini cli extension git URL
  publishing: |
    Extensions are published as public Git repositories, GitHub Releases, or scoped npm packages. Qwen Code has no dedicated publishing portal of its own; it consumes extensions from Git/npm and can convert Claude Code and Gemini CLI extensions. The getting-started guide recommends GitHub Releases for distribution.
  private_distribution: |
    Private Git repositories use GITHUB_TOKEN for HTTPS clones. Private npm registries use NPM_TOKEN or registry-specific _authToken entries in .npmrc; scoped registry configuration is also supported. Local paths and archives work for internal sharing without publishing.
  notes: |
    Qwen Code can browse plugins from the Claude Code Marketplace and Gemini CLI Extensions Gallery, but no persistent local marketplace registry file was observed in CLI 0.15.6. The docs reference qwen extensions sources add/list/update/remove, but those subcommands are not present in the 0.15.6 CLI help.

portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources:
    - skills
    - slash_commands
    - subagents
    - QWEN.md context
  non_portable_assets:
    - qwen-extension.json manifest
    - mcpServers configuration and executables
    - lspServers configuration
    - hooks/hooks.json
    - channels JavaScript entry points
    - settings .env and keychain entries
    - .qwen-extension-install.json metadata
    - ${extensionPath} references
    - excludeTools
  rewrite_needed: true
  notes: |
    Qwen Code can ingest Claude plugins and Gemini extensions by conversion, but the qwen-extension.json container is Qwen-specific. Claudine should extract Markdown-based resources (skills, commands, agents, QWEN.md) and treat executable/config resources as requiring rewrite or omission. Command names may need adjustment because Qwen only prefixes extension commands when they conflict, unlike Claude Code which always namespaces plugin commands.

cli_params:
  - flag: qwen extensions install <source>
    description: Install an extension from git, local path, archive, npm, or a Claude marketplace. Options --ref, --auto-update, --pre-release, --registry, --consent, --scope.
    example: qwen extensions install https://github.com/qwen-cli-extensions/security --consent
  - flag: qwen extensions uninstall <name>
    description: Remove an installed extension.
    example: qwen extensions uninstall qwen-cli-security
  - flag: qwen extensions list
    description: List installed extensions.
    example: qwen extensions list
  - flag: qwen extensions update [<name>] [--all]
    description: Update a named extension or all extensions.
    example: qwen extensions update --all
  - flag: qwen extensions enable <name> [--scope user|workspace]
    description: Enable an extension globally or for the current workspace.
    example: qwen extensions enable my-ext --scope workspace
  - flag: qwen extensions disable <name> [--scope user|workspace]
    description: Disable an extension globally or for the current workspace.
    example: qwen extensions disable my-ext --scope workspace
  - flag: qwen extensions link <path>
    description: Link a local extension directory so changes are reflected immediately.
    example: qwen extensions link ./my-extension
  - flag: qwen extensions new <path> [template]
    description: Create a new extension from a built-in template.
    example: qwen extensions new my-extension mcp-server
  - flag: qwen extensions settings set <name> <setting> [--scope user|workspace]
    description: Set an extension setting value.
    example: qwen extensions settings set my-ext API_KEY --scope user
  - flag: qwen extensions settings list <name>
    description: List all settings and current values for an extension.
    example: qwen extensions settings list my-ext
  - flag: qwen --extensions <names> / -e <names>
    description: Limit which extensions load for the session.
    example: qwen -e my-ext,other-ext
  - flag: qwen --bare
    description: Minimal mode that skips implicit startup discovery, including extensions.
    example: qwen --bare
  - flag: qwen --disabled-slash-commands <names>
    description: Hide/disable specific slash command names.
    example: qwen --disabled-slash-commands deploy,gcp:sync

env_vars:
  - name: QWEN_CODE_SIMPLE
    effect: Equivalent to --bare; disables implicit extension discovery.
  - name: QWEN_SANDBOX
    effect: Enables sandbox-exec/Docker/Podman for shell commands and spawned MCP/LSP/hook processes.
  - name: QWEN_SANDBOX_IMAGE
    effect: Docker/Podman image to use when sandboxing.
  - name: GITHUB_TOKEN
    effect: Authentication token for private GitHub repositories and marketplace fetches.
  - name: NPM_TOKEN
    effect: Authentication token for npm registry access; also respects .npmrc _authToken entries.
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: Comma-separated list of slash command names to disable.
  - name: QWEN_RUNTIME_DIR
    effect: Overrides runtime output base directory; does not move the extensions install root.
  - name: CI
    effect: Indicates non-interactive environment; may affect consent prompts.

gaps:
  - The qwen extensions sources add/list/update/remove CLI subcommands are documented but not present in qwen 0.15.6 help.
  - No official Qwen-specific marketplace registry; plugin discovery relies on Claude Code and Gemini CLI ecosystems.
  - No code signing or reproducible-build verification for extensions.
  - Project-scoped extension installs do not use a separate project extensions directory; they reuse the user directory and apply workspace enablement overrides.
  - The exact symlink behavior of qwen extensions link is described in docs but not fully visible in the 0.15.6 bundled source.
  - No dedicated safe-mode flag; --bare is the closest equivalent.

changes: []

requires_claudine_update: false
reason: This document is research-only. No Claudine code, schemas, generated metadata, or linking rules need to change.
---

# Qwen Code Extensions (Plugins)

## Overview

Qwen Code calls its plugin container an **extension**. An extension bundles prompts (slash commands), Agent Skills, subagents, MCP servers, LSP servers, hooks, channels, and configuration into a single installable directory with a `qwen-extension.json` manifest. Extensions are designed to be shared through Git repositories, npm packages, local paths, archives, or by converting plugins from the Claude Code Marketplace and Gemini CLI Extensions Gallery.

Compared with Claude Code, the shape is similar—both use a JSON manifest plus component directories for skills, commands, agents, hooks, and MCP servers—but the packaging and discovery mechanics differ. Qwen Code stores extensions under `~/.qwen/extensions/<name>/` with the manifest at the root (`qwen-extension.json`), whereas Claude Code uses `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/` with a manifest inside `.claude-plugin/plugin.json` ([Qwen Code Extensions](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/), [Claude Code Plugins](https://code.claude.com/docs/en/plugins)). Qwen Code also does not maintain its own marketplace registry; it consumes Claude and Gemini marketplaces directly.

This research was done against Qwen Code CLI 0.15.6 installed on the host. The local `~/.qwen/extensions/` directory does not exist because no extensions are installed.

## Installation and Locations

Extensions are installed into a user-level directory regardless of scope. Project scope only affects enablement and workspace-scoped settings.

| OS | Scope | Path | Purpose |
|---|---|---|---|
| macOS / Linux | user | `~/.qwen/extensions/<name>/` | Installed extension root: manifest, install metadata, settings, commands, skills, agents, hooks, executables, and `QWEN.md`. |
| Windows | user | `%USERPROFILE%\.qwen\extensions\<name>\` | Same layout as macOS/Linux. |
| macOS / Linux | user | `~/.qwen/extensions/extension-enablement.json` | Enable/disable overrides per extension and per scope. |
| Windows | user | `%USERPROFILE%\.qwen\extensions\extension-enablement.json` | Enablement overrides. |
| macOS / Linux | user | `~/.qwen/extensions/<name>/.env` | Non-sensitive extension setting values at user scope. |
| Windows | user | `%USERPROFILE%\.qwen\extensions\<name>\.env` | User-scope extension settings. |
| macOS / Linux | repo | `.qwen/.env` | Workspace-scoped extension setting values when `--scope workspace` is used. |
| Windows | repo | `.qwen\.env` | Workspace-scoped extension settings. |
| macOS / Linux / Windows | repo | `.qwen/` | Project-level Qwen Code configuration. Extensions are not stored here; only workspace enablement and settings are referenced. |

The host had no installed extensions and no `extension-enablement.json` file at the time of research.

## Manifest and Package Format

A native Qwen Code extension is a directory containing `qwen-extension.json` at its root. Qwen Code can also install extensions that expose `gemini-extension.json` or a Claude Code `.claude-plugin/plugin.json` plus `marketplace.json`; those are converted into the native format during installation.

Required manifest fields:

- `name` — lowercase/number/dash/dot/underscore identifier that must match the directory name.
- `version` — extension version string.

Recognized optional fields:

- `mcpServers` — map of MCP server configs.
- `lspServers` — map or path to LSP server configs.
- `hooks` — inline hooks object or path to a hooks JSON file.
- `channels` — map of channel adapter entries (`entry`, optional `displayName`).
- `contextFileName` — context file to load from the extension root; defaults to `QWEN.md` if present.
- `commands` — directory of slash-command Markdown files (default `commands/`).
- `skills` — directory of Agent Skill folders (default `skills/`).
- `agents` — directory of subagent files (default `agents/`).
- `settings` — array of user-prompted settings (`name`, `description`, `envVar`, `sensitive`).
- `excludeTools` — array of tool names to disable for the session.

A typical native layout:

```text
my-extension/
├── qwen-extension.json
├── QWEN.md
├── .env
├── .qwen-extension-install.json
├── commands/
│   ├── deploy.md
│   └── gcs/
│       └── sync.md
├── skills/
│   └── pdf-processor/
│       └── SKILL.md
├── agents/
│   └── testing-expert.md
├── hooks/
│   └── hooks.json
└── dist/
    └── server.js
```

Variable substitution is supported in `qwen-extension.json`:

| Variable | Value |
|---|---|
| `${extensionPath}` | Absolute path to the extension directory. |
| `${workspacePath}` | Absolute path to the current workspace. |
| `${/}` or `${pathSeparator}` | OS path separator. |

For Claude plugin conversion, Qwen Code reads `.claude-plugin/marketplace.json` to find the plugin entry, merges it with `.claude-plugin/plugin.json`, copies `commands/`, `skills/`, and `agents/`, converts agent files to Qwen subagent format, and writes a `qwen-extension.json` containing the merged `mcpServers`, `lspServers`, and `hooks` ([Qwen Code Extensions](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/)). Gemini extensions are converted from `gemini-extension.json`, with TOML commands migrated to Markdown.

## Packaged Resources

| Resource | Support | Location | Notes |
|---|---|---|---|
| Agent Skills | Full | `skills/<name>/SKILL.md` | Loaded as extension-level skills. |
| Slash commands | Full | `commands/**/*.md` (legacy `.toml` also supported) | Become `/command-name` or `/group:command-name`. |
| Subagents | Full | `agents/*.md`, `agents/*.yaml` | Appear under Extension Agents. |
| MCP servers | Full | `mcpServers` in manifest or `.mcp.json` equivalent | Extension `trust` field is stripped; user approval still required. |
| Hooks | Full | `hooks/hooks.json` or inline `hooks` | Lifecycle event handlers. |
| LSP servers | Full | `lspServers` in manifest or JSON path | User LSP configs override by name. |
| Channels | Full | `channels` in manifest with JS `entry` | Custom chat-platform adapters. |
| Scripts / executables | Partial | `dist/`, root-level `.js`, MCP server binaries | No dedicated `scripts/` container, but arbitrary executables are allowed. |
| Config | Partial | `settings`, `contextFileName`, `excludeTools` | Settings produce `.env`/keychain entries. |
| Prompts | None | — | Prompts are represented as slash commands. |
| Assets | Partial | `QWEN.md`, icons, README, etc. | No formal `assets/` directory, but static files may be referenced. |

## Lifecycle and Trust

Install:

- `qwen extensions install <source>` accepts Git URLs, `owner/repo`, local paths/archives, archive URLs, scoped npm packages, and Claude marketplace sources in `marketplace:plugin-name` form. Options include `--ref`, `--auto-update`, `--pre-release`, `--registry`, `--consent`, and `--scope user|project`.
- The source is copied/extracted into `~/.qwen/extensions/<name>/`, install metadata is written to `.qwen-extension-install.json`, settings are prompted, and the extension is enabled by default.

Update:

- `qwen extensions update [<name>] [--all]` refreshes Git, npm, archive-URL, and local-path extensions. Exact npm version pins are considered up-to-date; dist-tag pins track the tag. Linked extensions do not need updating. Auto-update is opt-in.

Remove:

- `qwen extensions uninstall <name>` deletes the extension directory and its enablement entry.

Enable / disable:

- `qwen extensions enable|disable <name> [--scope user|workspace]` writes path overrides into `~/.qwen/extensions/extension-enablement.json`. Workspace scope is an alias for project scope. System scope is not supported.

Trust:

- No code signing or sandbox boundary is specific to extensions. Trust is established by the explicit install/enable action, which requires `--consent` or interactive confirmation. Workspace trust is required for local-path installs. Extension MCP servers cannot pre-declare trust; they are subject to the same approval flow as user MCP servers.

Versioning:

- Version comes from `qwen-extension.json`. npm resolves tags/versions, git respects `--ref`, and otherwise HEAD is used. The install metadata records the resolved version.

## Discovery and Precedence

Discovery:

- At startup Qwen Code scans `~/.qwen/extensions/`, reads each `qwen-extension.json`, resolves active state from `extension-enablement.json`, and applies any `--extensions`/`-e` CLI override. In bare mode only explicitly listed extensions load.

Precedence:

- **Commands**: user/project commands take precedence; extension commands are renamed to `extensionName.commandName` only when a conflict exists.
- **Agents**: `loadSubagent` checks user, project, extension, builtin in that order, so user/project agents shadow extension agents.
- **Skills**: loaded from user, project, and extension levels; the command loader tags them by source but does not namespace them unless the command service renames them on conflict.
- **MCP servers**: user `settings.json` configs override extension `mcpServers` by name.
- **Hooks**: executed in priority order project (1), user (2), system (3), extensions (4).
- **LSP servers**: extension configs are applied first, then user configs override by name.

Namespacing:

- Extension slash commands use their natural name unless they conflict, in which case they become `extensionName.commandName`. Skills and agents are not namespaced by default. MCP/LSP server names are not prefixed.

Conflicts:

- Command conflicts are resolved by renaming. Skill/agent conflicts are resolved by first-found-wins. MCP/LSP conflicts are resolved by same-name override.

## Security and Runtime Behavior

Trust model:

- Extensions are highly trusted. Installation/enablement is the trust boundary. There is no signature verification.

Permissions:

- Extension commands become slash commands executed in the agent loop. MCP servers require per-server approval. Hooks can run shell commands and HTTP callbacks. `excludeTools` can disable tools session-wide.

Sandbox interaction:

- When Qwen Code is launched with `--sandbox` or `QWEN_SANDBOX`, shell commands and spawned MCP/LSP/hook processes run under sandbox-exec (macOS) or Docker/Podman. By default no sandbox is used.

Credential access:

- Extension settings can declare sensitive values that are stored in the OS keychain and passed as environment variables to MCP servers. Non-sensitive values go to `.env`. Spawned commands inherit the user's environment, so extensions can read environment variables indirectly.

Update risk:

- Default risk is low because updates are explicit. Risk increases for extensions installed with `--auto-update` or linked extensions, which reflect source changes immediately.

## Distribution

Qwen Code has no dedicated marketplace of its own. It can install extensions from:

- Public or private Git repositories.
- GitHub Releases.
- Local directories, `.zip`, or `.tar.gz` archives.
- Archive URLs.
- Scoped npm packages, including private registries.
- The Claude Code Marketplace (`marketplace-url:plugin-name`).
- Gemini CLI Extensions Gallery via Git URL.

Publishing is done by sharing a Git repository, publishing a GitHub Release, or publishing a scoped npm package. Private Git repos use `GITHUB_TOKEN`; private npm registries use `NPM_TOKEN` or `.npmrc` `_authToken` entries.

## Portability

Claudine should not link Qwen Code extensions as intact units to other providers. The `qwen-extension.json` manifest, MCP/LSP configs, hooks, channel entry points, settings `.env`, and install metadata are Qwen-specific.

Portable resources:

- `skills/<name>/SKILL.md`
- `commands/*.md` slash commands
- `agents/*.md` subagent definitions
- `QWEN.md` context file

Non-portable assets:

- `qwen-extension.json`
- MCP/LSP server configs and binaries
- `hooks/hooks.json`
- Channel JavaScript entry points
- `.env` and keychain settings
- `.qwen-extension-install.json`
- `${extensionPath}` references
- `excludeTools`

Rewrite is needed for provider-specific manifest fields, command naming (Qwen only prefixes on conflict while Claude Code always namespaces), and path variables.

## Claudine Linking Notes

- Treat `~/.qwen/extensions/<name>/` as the canonical extension root and `~/.qwen/extensions/extension-enablement.json` as the enablement source.
- For each enabled extension, scan `skills/`, `commands/`, `agents/`, and `QWEN.md` for portable resources.
- Extract skills and commands to the target provider's format, adjusting command names when they conflict with user/project resources.
- Do not extract or link `mcpServers`, `lspServers`, `hooks`, `channels`, `settings`, `excludeTools`, or arbitrary executables without explicit rewrite and host-aware validation.
- Respect `--extensions`/`-e` and `--bare`/`QWEN_CODE_SIMPLE` when wrapping a specific Qwen Code invocation.
- Note that Qwen Code can consume Claude plugins and Gemini extensions, so a plugin source may already be a converted copy of another provider's bundle.

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code Extensions (user guide)](https://qwenlm.github.io/qwen-code-docs/en/users/extension/introduction/)
- [Qwen Code Extensions (developer guide)](https://qwenlm.github.io/qwen-code-docs/en/developers/extensions/extension/)
- [Getting Started with Qwen Code Extensions](https://qwenlm.github.io/qwen-code-docs/en/developers/extensions/getting-started-extensions/)
- [Extension Releasing Guide](https://qwenlm.github.io/qwen-code-docs/en/developers/extensions/extension-releasing/)
- [Claude Code Plugins](https://code.claude.com/docs/en/plugins)
- [Claude Code Plugins Reference](https://code.claude.com/docs/en/plugins-reference)
- [Qwen Code CLI source, v0.15.6](/opt/homebrew/Cellar/qwen-code/0.15.6/libexec/lib/node_modules/@qwen-code/qwen-code/cli.js)
