---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://www.anthropic.com/claude-code
docs: https://code.claude.com/docs/en/overview
plugin_docs: https://code.claude.com/docs/en/plugins

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/
    notes: Installed marketplace plugins are copied here. Observed on this host. Windows resolves ~ to %USERPROFILE%.
  - os: linux
    scope: user
    path: ~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/
    notes: Versioned plugin cache. Each update creates a new directory; orphaned versions are removed after 7 days.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\plugins\\cache\\<marketplace>\\<plugin>\\<version>\\"
    notes: Windows plugin cache.
  - os: macos
    scope: user
    path: ~/.claude/plugins/data/<plugin-id>/
    notes: Persistent plugin data directory (${CLAUDE_PLUGIN_DATA}). Survives updates; deleted on uninstall unless --keep-data.
  - os: linux
    scope: user
    path: ~/.claude/plugins/data/<plugin-id>/
    notes: Persistent plugin data directory.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\plugins\\data\\<plugin-id>\\"
    notes: Persistent plugin data directory.
  - os: macos
    scope: user
    path: ~/.claude/plugins/marketplaces/<marketplace-name>/
    notes: Cloned marketplace catalogs. Observed claude-plugins-official, claude-code-plugins, and sentrux-marketplace.
  - os: linux
    scope: user
    path: ~/.claude/plugins/marketplaces/<marketplace-name>/
    notes: Cloned marketplace catalogs.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\plugins\\marketplaces\\<marketplace-name>\\"
    notes: Cloned marketplace catalogs.
  - os: macos
    scope: user
    path: ~/.claude/plugins/installed_plugins.json
    notes: Registry of installed plugin versions, scopes, install paths, and git commit SHAs.
  - os: linux
    scope: user
    path: ~/.claude/plugins/installed_plugins.json
    notes: Registry of installed plugin versions.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\plugins\\installed_plugins.json"
    notes: Registry of installed plugin versions.
  - os: macos
    scope: user
    path: ~/.claude/plugins/known_marketplaces.json
    notes: Registry of added marketplace sources and clone locations.
  - os: linux
    scope: user
    path: ~/.claude/plugins/known_marketplaces.json
    notes: Registry of added marketplace sources.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\plugins\\known_marketplaces.json"
    notes: Registry of added marketplace sources.
  - os: macos
    scope: user
    path: ~/.claude/skills/<name>/
    notes: Skills-directory plugins (source @skills-dir). Discovered automatically; no marketplace install step.
  - os: linux
    scope: user
    path: ~/.claude/skills/<name>/
    notes: Skills-directory plugins.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\skills\\<name>\\"
    notes: Skills-directory plugins.
  - os: macos
    scope: repo
    path: .claude/skills/<name>/
    notes: Project-scoped skills-directory plugins. Loaded only from launch directory (does not walk up to repo root). Require workspace trust.
  - os: linux
    scope: repo
    path: .claude/skills/<name>/
    notes: Project-scoped skills-directory plugins.
  - os: windows
    scope: repo
    path: ".claude\\skills\\<name>\\"
    notes: Project-scoped skills-directory plugins.
  - os: macos
    scope: user
    path: ~/.claude/settings.json
    notes: Stores enabledPlugins, pluginConfigs, and extraKnownMarketplaces at user scope.
  - os: linux
    scope: user
    path: ~/.claude/settings.json
    notes: User-scope plugin settings.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\settings.json"
    notes: User-scope plugin settings.
  - os: macos
    scope: repo
    path: .claude/settings.json
    notes: Project-scope plugin settings; shared with collaborators.
  - os: linux
    scope: repo
    path: .claude/settings.json
    notes: Project-scope plugin settings.
  - os: windows
    scope: repo
    path: ".claude\\settings.json"
    notes: Project-scope plugin settings.
  - os: macos
    scope: repo
    path: .claude/settings.local.json
    notes: Local gitignored overrides.
  - os: linux
    scope: repo
    path: .claude/settings.local.json
    notes: Local gitignored overrides.
  - os: windows
    scope: repo
    path: ".claude\\settings.local.json"
    notes: Local gitignored overrides.
  - os: macos
    scope: system
    path: /Library/Application Support/ClaudeCode/managed-settings.json
    notes: Managed settings can force-enable plugins and restrict marketplaces via strictKnownMarketplaces, blockedMarketplaces, disableSideloadFlags.
  - os: linux
    scope: system
    path: /etc/claude-code/managed-settings.json
    notes: Managed settings for plugins.
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\managed-settings.json"
    notes: Managed settings for plugins.

manifest:
  file_names:
    - .claude-plugin/plugin.json
  format: json
  required_fields:
    - name
  optional_fields:
    - $schema
    - displayName
    - version
    - description
    - author
    - homepage
    - repository
    - license
    - keywords
    - defaultEnabled
    - skills
    - commands
    - agents
    - hooks
    - mcpServers
    - lspServers
    - outputStyles
    - experimental.themes
    - experimental.monitors
    - userConfig
    - channels
    - dependencies
  package_layout: |
    Plugin root contains .claude-plugin/plugin.json plus component directories at the root level:
    - skills/<name>/SKILL.md for Agent Skills
    - commands/*.md for legacy flat slash-command files
    - agents/*.md for subagent definitions
    - hooks/hooks.json for event handlers
    - .mcp.json for MCP server configs
    - .lsp.json for LSP server configs
    - output-styles/ for output style files
    - themes/ for color themes
    - monitors/monitors.json for background monitors
    - bin/ for executables added to the Bash tool PATH
    - settings.json for plugin default settings (agent / subagentStatusLine)
    A single-skill plugin may place SKILL.md at the root instead.
  notes: |
    Manifest is optional; when omitted Claude auto-discovers components in default locations and derives the plugin name from the directory. Path fields are relative to the plugin root and must start with ./. The skills field adds to the default skills/ scan; commands, agents, outputStyles, experimental.themes, and experimental.monitors replace their default directories unless listed explicitly. hooks, mcpServers, and lspServers merge by their own rules. The JSON Schema URL is https://json.schemastore.org/claude-code-plugin-manifest.json.

lifecycle:
  install: |
    Marketplace plugins are installed via /plugin install <plugin>[@<marketplace>] or claude plugin install <plugin> --scope user|project|local. The plugin source is cloned/copied into ~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/ and an entry is written to enabledPlugins in the chosen settings scope. Session-only plugins can be loaded with claude --plugin-dir <dir|zip> or claude --plugin-url <url>. Project .claude/settings.json entries from external sources prompt each user to install and trust the plugin before it runs (v2.1.195+).
  update: |
    /plugin update <plugin> or claude plugin update <plugin> fetches the latest version. Official marketplaces auto-update by default; third-party and local marketplaces do not. Updates create a new versioned cache directory; the previous version remains for ~7 days. Use DISABLE_AUTOUPDATER=1 to disable all auto-updates, or FORCE_AUTOUPDATE_PLUGINS=1 to keep plugin auto-updates while disabling Claude Code auto-updates.
  remove: |
    /plugin uninstall <plugin> or claude plugin uninstall <plugin> removes the enabledPlugins entry for the scope and deletes the plugin cache. --keep-data preserves ~/.claude/plugins/data/<id>/. --prune removes auto-installed dependencies no longer required.
  enable_disable: |
    Plugins are enabled/disabled per scope in enabledPlugins. /plugin enable|disable <plugin> or claude plugin enable|disable <plugin> --scope <scope> toggles state. defaultEnabled in plugin.json controls first install only; an explicit enabledPlugins entry persists across updates. Disabling fails if another enabled plugin depends on the target.
  trust: |
    No separate signature or sandbox trust layer. Trust is established by explicit install/enable, workspace trust for project-scope @skills-dir plugins, and managed-settings restrictions (strictKnownMarketplaces, blockedMarketplaces, disableSideloadFlags). Each user must approve installation of project-recommended external plugins.
  versioning: |
    Version resolves from plugin.json version, then marketplace entry version, then git commit SHA. Setting version pins the plugin; omitting it treats every commit as a new version. Cache paths include the resolved version string.
  notes: |
    Lifecycle is primarily CLI/UI driven. Settings files are watched and reloaded, but newly installed/enabled plugins require /reload-plugins or a restart to take effect. Managed plugins are read-only and cannot be uninstalled by users.

packaged_resources:
  skills: full
  scripts: partial
  slash_commands: full
  subagents: full
  mcp_servers: full
  hooks: full
  prompts: none
  config: partial
  assets: full
  other:
    - lsp_servers
    - monitors
    - themes
    - output_styles
    - channels

discovery:
  mechanism: |
    Claude Code scans enabledPlugins in settings.json at startup, loads matching plugins from ~/.claude/plugins/cache/, discovers @skills-dir plugins in ~/.claude/skills/ and .claude/skills/, and loads session plugins from --plugin-dir/--plugin-url. /reload-plugins rescans without restarting. Plugin components are registered in the in-session plugin inventory shown by /plugin and claude plugin details.
  precedence: |
    Session-only plugins (--plugin-dir/--plugin-url) override installed marketplace plugins with the same name for that session. Project .claude/skills/ agents override same-named plugin agents; user/repo skills and commands do not collide with plugin ones because plugin skills/commands are namespaced. Managed settings override user/project settings.
  namespacing: |
    Plugin skills and agents are exposed as plugin-name:resource-name, e.g. /superpowers:tdd or @superpowers:debugger. Plugin slash commands from commands/*.md follow the same namespacing. Plugin MCP and LSP servers are prefixed with the plugin identifier in tool listings.
  conflicts: |
    Exact name conflicts within the same namespace are rejected. Marketplace plugin names must be unique per marketplace. A plugin name collision between --plugin-dir and an installed plugin is resolved in favor of the session-only --plugin-dir copy.
  notes: |
    Project-scope @skills-dir plugins are discovered only from the launch working directory, not by walking up to the repository root. Disabled plugins are ignored without deleting files. Managed plugins and --plugin-dir plugins are never flagged as unused.

security:
  trust_model: |
    Plugins are highly trusted: installation is the trust boundary. Anthropic does not sign or sandbox plugin code. Users must trust the source (marketplace, Git repo, local path, or URL). Managed settings can whitelist/blacklist marketplaces and block sideload flags.
  permissions: |
    Plugin MCP servers require the same per-server approval as user MCP servers. Project-scope plugin MCP/LSP/monitor loading is gated by workspace trust. Hooks run commands with the user's shell privileges. The plugin manifest can declare userConfig options, including sensitive values stored in the system keychain or ~/.claude/.credentials.json.
  sandbox_interaction: |
    Plugin-provided hooks, MCP servers, LSP servers, monitors, and bin/ executables run unsandboxed with user privileges, not inside a Claude Code sandbox. They share the user's environment and can access the filesystem and network subject to normal OS permissions.
  credential_access: |
    Plugins can reference environment variables via ${ENV_VAR} substitution in MCP/LSP configs, hook commands, and monitor commands. userConfig sensitive values are stored in the keychain/credentials.json and exported as CLAUDE_PLUGIN_OPTION_<KEY>. Plugins can read the plugin root, plugin data dir, and project dir via ${CLAUDE_PLUGIN_ROOT}, ${CLAUDE_PLUGIN_DATA}, and ${CLAUDE_PROJECT_DIR}.
  update_risk: |
    High. Official marketplaces auto-update by default, so a plugin can change behavior silently when its source updates. Version pinning via the version field mitigates this but is optional; without it, every new commit becomes a new version.
  notes: |
    The local blocklist file ~/.claude/plugins/blocklist.json stores user-blocked plugins. This host observed a blocklist with test entries. Plugins can execute arbitrary code; only install from trusted sources.

distribution:
  marketplace: true
  registry_url: https://claude.com/plugins
  source_types:
    - marketplace (official and community)
    - github repo
    - git repo
    - git-subdir
    - npm package
    - local folder
    - remote URL to marketplace.json
    - zip archive via --plugin-dir
    - zip archive via --plugin-url
  publishing: |
    Official marketplace is curated by Anthropic; no application process. Community marketplace submissions go through Anthropic review at claude.ai/admin-settings/directory/submissions/plugins/new or platform.claude.com/plugins/submit. Third parties can create private marketplaces by hosting a .claude-plugin/marketplace.json file in a Git repo or local directory. Plugins can also be distributed as npm packages or zip archives.
  private_distribution: |
    Private Git repositories work via existing git credential helpers for manual operations, but background auto-updates require GITHUB_TOKEN/GH_TOKEN, GITLAB_TOKEN/GL_TOKEN, or BITBUCKET_TOKEN. Team marketplaces can be pre-configured in .claude/settings.json via extraKnownMarketplaces and enabledPlugins. Container images can pre-populate plugins with CLAUDE_CODE_PLUGIN_SEED_DIR and CLAUDE_CODE_PLUGIN_CACHE_DIR.
  notes: |
    The official marketplace (claude-plugins-official) is registered automatically on first interactive launch; non-interactive sessions must add it explicitly with claude plugin marketplace add anthropics/claude-plugins-official. The community marketplace (anthropics/claude-plugins-community) must be added manually. Marketplace names are reserved and cannot impersonate Anthropic.

portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources:
    - skills
    - slash_commands
    - subagents
    - themes
    - output_styles
    - assets
  non_portable_assets:
    - plugin manifest (.claude-plugin/plugin.json)
    - marketplace entry metadata
    - MCP server configs and credentials
    - LSP server binary paths
    - userConfig values and keychain credentials
    - hook executable commands and scripts/
    - bin/ executables
    - ${CLAUDE_PLUGIN_ROOT} path references
    - dependencies field and npm packages
    - monitors and channels
  rewrite_needed: true
  notes: |
    The portable parts are the Markdown-based resources (SKILL.md, commands/*.md, agents/*.md, output-styles/*.md, themes/*.json) and static assets. The plugin manifest, MCP/LSP configs, hook commands, userConfig prompts, and any ${CLAUDE_PLUGIN_ROOT}/${CLAUDE_PLUGIN_DATA} references are Claude Code-specific and must be rewritten or omitted when linking to another provider. Scripts in scripts/ and bin/ are executable and host-dependent. Marketplace IDs and dependency graphs do not travel across providers.

cli_params:
  - flag: --plugin-dir <dir|zip>
    description: Load a plugin from a directory or .zip archive for this session only. Repeatable.
    example: claude --plugin-dir ./my-plugin --plugin-dir ./other.zip
  - flag: --plugin-url <url>
    description: Fetch a plugin .zip archive from a URL for this session only. Repeatable or space-separated in one quoted value.
    example: claude --plugin-url https://example.com/plugin.zip
  - flag: --bare
    description: Minimal mode. Skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md.
    example: claude --bare -p "query"
  - flag: --safe-mode
    description: Disables CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands/agents, output styles, workflows, themes, keybindings, status line, file-suggestion commands, LSP, and auto-memory.
    example: claude --safe-mode
  - flag: --disable-slash-commands
    description: Disables all skills and commands for the session, including plugin skills.
    example: claude --disable-slash-commands
  - flag: --setting-sources <scopes>
    description: Restrict which settings scopes load. Can prevent project/local plugin settings from applying.
    example: claude --setting-sources user,project
  - flag: --settings <file-or-json>
    description: Session-only settings overlay, including enabledPlugins and pluginConfigs.
    example: claude --settings ./ci-settings.json
  - flag: --channels <plugin:name@marketplace,...>
    description: Load channel plugins for message injection. Requires Claude.ai authentication.
    example: claude --channels plugin:my-notifier@my-marketplace
  - flag: claude plugin init <name>
    description: Scaffold a new plugin at ~/.claude/skills/<name>/ as a @skills-dir plugin.
    example: claude plugin init my-tool --with skills hooks
  - flag: claude plugin install <plugin> --scope <scope>
    description: Install a plugin from available marketplaces.
    example: claude plugin install code-review@claude-plugins-official --scope user
  - flag: claude plugin uninstall <plugin> --scope <scope> --keep-data --prune -y
    description: Remove an installed plugin; --keep-data preserves data dir; --prune cleans dependencies.
    example: claude plugin uninstall my-plugin --prune -y
  - flag: claude plugin enable|disable <plugin> --scope <scope>
    description: Enable or disable a plugin without installing/uninstalling.
    example: claude plugin enable my-plugin@my-marketplace
  - flag: claude plugin update <plugin> --scope <scope>
    description: Update a plugin to the latest version.
    example: claude plugin update my-plugin
  - flag: claude plugin list --json --available
    description: List installed plugins; --json --available includes marketplace-available plugins.
    example: claude plugin list --json
  - flag: claude plugin details <name>
    description: Show a plugin's component inventory and projected token cost.
    example: claude plugin details rust-analyzer-lsp@claude-plugins-official
  - flag: claude plugin validate <path> --strict
    description: Validate a plugin or marketplace manifest. --strict treats warnings as errors.
    example: claude plugin validate ./my-plugin --strict
  - flag: claude plugin prune --scope <scope> --dry-run -y
    description: Remove auto-installed plugin dependencies no longer required.
    example: claude plugin prune --dry-run
  - flag: claude plugin tag --push --dry-run --force
    description: Create a release git tag for the plugin in the current directory.
    example: claude plugin tag --push
  - flag: claude plugin marketplace add <source> --scope <scope> --sparse <paths...>
    description: Add a marketplace from GitHub owner/repo, git URL, local path, or remote marketplace.json URL.
    example: claude plugin marketplace add anthropics/claude-plugins-community
  - flag: claude plugin marketplace list --json
    description: List configured marketplaces.
    example: claude plugin marketplace list --json
  - flag: claude plugin marketplace remove <name>
    description: Remove a marketplace and uninstall its plugins.
    example: claude plugin marketplace remove my-marketplace
  - flag: claude plugin marketplace update <name>
    description: Refresh a marketplace catalog from its source.
    example: claude plugin marketplace update claude-plugins-official

env_vars:
  - name: CLAUDE_CODE_PLUGIN_CACHE_DIR
    effect: Override the plugins root directory (~/.claude/plugins). Marketplaces and plugin cache live in subdirectories under this path.
  - name: CLAUDE_CODE_PLUGIN_SEED_DIR
    effect: Path to one or more read-only plugin seed directories (colon-separated on Unix, semicolon on Windows). Used to pre-populate marketplaces and plugin caches in containers.
  - name: CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS
    effect: Timeout in milliseconds for git operations when installing or updating plugins (default 120000).
  - name: CLAUDE_CODE_PLUGIN_KEEP_MARKETPLACE_ON_FAILURE
    effect: Set to 1 to keep the existing marketplace cache when a git pull fails instead of wiping and re-cloning.
  - name: CLAUDE_CODE_PLUGIN_PREFER_HTTPS
    effect: Set to 1 to clone GitHub owner/repo shorthand sources over HTTPS instead of SSH.
  - name: CLAUDE_CODE_SYNC_PLUGIN_INSTALL
    effect: Set to 1 in non-interactive mode (-p) to wait for plugin installation to complete before the first query.
  - name: CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS
    effect: Timeout in milliseconds for synchronous plugin installation when CLAUDE_CODE_SYNC_PLUGIN_INSTALL is set.
  - name: CLAUDE_CODE_ENABLE_BACKGROUND_PLUGIN_REFRESH
    effect: Set to 1 to refresh plugin state at turn boundaries in non-interactive mode after a background install completes.
  - name: FORCE_AUTOUPDATE_PLUGINS
    effect: Set to 1 to force plugin auto-updates even when the main auto-updater is disabled via DISABLE_AUTOUPDATER.
  - name: DISABLE_AUTOUPDATER
    effect: Set to 1 to disable all automatic updates for both Claude Code and plugins.
  - name: CLAUDE_CODE_SAFE_MODE
    effect: Set to 1 to disable plugins along with most other customizations. Equivalent to --safe-mode.
  - name: CLAUDE_CODE_SIMPLE
    effect: Set to 1 to disable auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. Equivalent to --bare.
  - name: GITHUB_TOKEN / GH_TOKEN
    effect: Personal access token for private GitHub marketplace/plugin auto-updates.
  - name: GITLAB_TOKEN / GL_TOKEN
    effect: Personal or project token for private GitLab marketplace/plugin auto-updates.
  - name: BITBUCKET_TOKEN
    effect: App password or repository access token for private Bitbucket marketplace/plugin auto-updates.

gaps:
  - No documented code signing or reproducible-build verification for plugins.
  - No public API to query the plugin inventory or enable state outside of claude plugin list --json.
  - Exact merge semantics for plugin settings.json with user/project settings are not fully specified beyond agent and subagentStatusLine.
  - The exact contents of the installation/trust prompt for project-recommended external plugins are not documented.

changes: []

requires_claudine_update: true
reason: |
  Claudine should model Claude Code's first-class plugin container: ~/.claude/plugins cache and data directories, marketplace registry files, enabledPlugins/pluginConfigs settings, @skills-dir plugins, and session-only --plugin-dir/--plugin-url loading. Linking should extract portable Markdown resources (skills, commands, agents, output styles, themes, assets) while flagging non-portable assets (plugin manifest, MCP/LSP configs, hook commands, userConfig credentials, bin/ executables, and ${CLAUDE_PLUGIN_ROOT} references) as requiring rewrite or omission. The linker also needs to respect managed-settings restrictions and versioned cache directories.
---

# Claude Code Plugins

## Overview

Claude Code has a first-class plugin system that packages multiple extension assets into a single installable, versioned, namespaced unit. A plugin is a directory containing a `.claude-plugin/plugin.json` manifest and zero or more component directories: `skills/`, `commands/`, `agents/`, `hooks/`, `.mcp.json`, `.lsp.json`, `output-styles/`, `themes/`, `monitors/`, and `bin/`. Plugins can be loaded from a marketplace, a local directory, a Git repository, an npm package, or a zip archive, and they can be scoped per user, project, or local session.

Plugins are distinct from standalone `.claude/` configuration. Standalone resources live directly in `.claude/skills/`, `.claude/agents/`, `.claude/commands/`, or `settings.json`, are not namespaced, and are best for personal or project-specific workflows. Plugins are namespaced, versioned, and designed for sharing across projects and teams.

Plugins are loaded at session startup based on `enabledPlugins` entries in settings, `@skills-dir` plugins discovered in skills directories, and session-only `--plugin-dir`/`--plugin-url` flags. Newly installed or enabled plugins require `/reload-plugins` or a restart to take effect.

## Installation and Locations

Claude Code stores plugin state under `~/.claude/plugins/` (or `%USERPROFILE%\.claude\plugins\` on Windows). The local layout observed on this host is:

| Directory | Purpose |
|---|---|
| `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/` | Copied plugin contents. Each installed version is a separate directory. Orphaned versions are kept for ~7 days. |
| `~/.claude/plugins/data/<plugin-id>/` | Persistent plugin data directory referenced by `${CLAUDE_PLUGIN_DATA}`. |
| `~/.claude/plugins/marketplaces/<marketplace-name>/` | Cloned marketplace catalogs containing `.claude-plugin/marketplace.json`. |
| `~/.claude/plugins/installed_plugins.json` | Registry of installed plugins, scopes, versions, install paths, and git SHAs. |
| `~/.claude/plugins/known_marketplaces.json` | Registry of added marketplace sources. |
| `~/.claude/plugins/blocklist.json` | User-blocked plugins. This host contained test entries. |

Plugins can also live as `@skills-dir` plugins inside `~/.claude/skills/<name>/` or `.claude/skills/<name>/` without a marketplace install step. Project-scope `@skills-dir` plugins require accepting the workspace trust dialog and are discovered only from the launch directory.

Installation scopes mirror Claude Code's settings scopes:

| Scope | Settings file | Use case |
|---|---|---|
| User | `~/.claude/settings.json` | Personal plugins across all projects (default). |
| Project | `.claude/settings.json` | Team-shared plugins committed to version control. |
| Local | `.claude/settings.local.json` | Gitignored project-specific plugins. |
| Managed | `managed-settings.json` | Read-only enterprise-enforced plugins. |

When a plugin is installed to a scope, Claude Code writes its identifier to `enabledPlugins` in that scope's settings file. The same plugin can be installed to multiple scopes.

## Manifest and Package Format

The plugin manifest lives at `.claude-plugin/plugin.json` relative to the plugin root. The manifest is optional; when omitted Claude Code auto-discovers components in default locations and derives the name from the directory.

Required field:

- `name` — kebab-case plugin identifier used for namespacing.

Optional metadata fields:

- `$schema`, `displayName`, `version`, `description`, `author`, `homepage`, `repository`, `license`, `keywords`, `defaultEnabled`.

Optional component path fields:

- `skills`, `commands`, `agents`, `hooks`, `mcpServers`, `lspServers`, `outputStyles`.
- `experimental.themes`, `experimental.monitors`.
- `userConfig`, `channels`, `dependencies`.

Package layout at the plugin root:

```text
my-plugin/
├── .claude-plugin/
│   └── plugin.json
├── skills/
│   └── my-skill/
│       └── SKILL.md
├── commands/
│   └── my-command.md
├── agents/
│   └── reviewer.md
├── hooks/
│   └── hooks.json
├── .mcp.json
├── .lsp.json
├── output-styles/
├── themes/
├── monitors/
│   └── monitors.json
├── bin/
├── settings.json
└── README.md
```

`skills/` always adds to the default scan, while `commands/`, `agents/`, `outputStyles/`, `experimental.themes`, and `experimental.monitors` replace their default directories when specified. `hooks` and `mcpServers` merge inline configs with files. All custom paths must be relative and start with `./`.

A single-skill plugin can place `SKILL.md` directly at the plugin root instead of using a `skills/` directory.

## Packaged Resources

Claude Code plugins can contain the following resource types:

| Resource | Support | Location | Notes |
|---|---|---|---|
| Agent Skills | Full | `skills/<name>/SKILL.md` | Namespaced as `plugin-name:skill-name`. Can also use root `SKILL.md` for single-skill plugins. |
| Slash commands | Full | `commands/*.md` | Legacy flat Markdown command files; namespaced. |
| Subagents | Full | `agents/*.md` | Namespaced as `plugin-name:agent-name`. `hooks`, `mcpServers`, and `permissionMode` are not allowed in plugin agents for security. |
| MCP servers | Full | `.mcp.json` or inline `mcpServers` | Start automatically when the plugin is enabled. |
| Hooks | Full | `hooks/hooks.json` or inline `hooks` | Respond to Claude Code lifecycle events. |
| LSP servers | Full | `.lsp.json` or inline `lspServers` | Require the language-server binary to be installed separately. |
| Background monitors | Full | `monitors/monitors.json` or inline `experimental.monitors` | Run shell commands for the session lifetime. Experimental. |
| Themes | Full | `themes/` | Color themes; selecting one persists `custom:<plugin>:<slug>`. Experimental. |
| Output styles | Full | `output-styles/` | Default response style files. |
| Channels | Full | `channels` in plugin.json + plugin MCP server | Message injection channels. |
| Scripts / executables | Partial | `scripts/`, `bin/` | `bin/` is added to the Bash tool PATH; `scripts/` files are referenced by hooks/MCP. No automatic discovery of scripts as standalone commands. |
| Config | Partial | `settings.json` at plugin root | Only `agent` and `subagentStatusLine` are supported. `userConfig` prompts for values at enable time. |
| Prompts | None | — | Claude Code does not have a dedicated prompt container; skills and commands serve this role. |
| Assets | Full | `assets/`, `README.md`, etc. | Static files referenced by skills or hooks. |

## Lifecycle and Trust

Install:

- Marketplace: `/plugin install <plugin>[@<marketplace>]` or `claude plugin install <plugin> --scope user|project|local`.
- Session-only: `claude --plugin-dir <dir|zip>` or `claude --plugin-url <url>`.
- Project `.claude/settings.json` entries for external plugins prompt each user to install and trust before running (v2.1.195+).

Update:

- `/plugin update <plugin>` or `claude plugin update <plugin>`.
- Official marketplaces auto-update by default; third-party and local marketplaces do not.
- Updates create a new versioned cache directory; the previous version remains for ~7 days for concurrent sessions.

Remove:

- `/plugin uninstall <plugin>` or `claude plugin uninstall <plugin> [--keep-data] [--prune] [-y]`.
- `--keep-data` preserves `~/.claude/plugins/data/<id>/`.
- `--prune` removes auto-installed dependencies no longer required.

Enable / disable:

- `/plugin enable|disable <plugin>` or `claude plugin enable|disable <plugin> --scope <scope>`.
- `defaultEnabled` in `plugin.json` only controls first install; an explicit `enabledPlugins` entry persists across updates.
- Disabling fails if another enabled plugin depends on the target.

Trust:

- No code signing or sandboxing. Trust is established by explicit install/enable action.
- Workspace trust is required for project-scope `@skills-dir` plugins.
- Managed settings can restrict sources via `strictKnownMarketplaces`, `blockedMarketplaces`, and `disableSideloadFlags`.
- The local blocklist file `~/.claude/plugins/blocklist.json` stores user-blocked plugins.

Versioning:

- Version resolves from `plugin.json` `version`, then marketplace entry `version`, then git commit SHA.
- Setting `version` pins the plugin; omitting it treats every new commit as a new version.

## Discovery and Precedence

Discovery mechanism:

1. Read `enabledPlugins` from user, project, local, and managed settings.
2. Load matching plugins from `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/`.
3. Discover `@skills-dir` plugins in `~/.claude/skills/` and `.claude/skills/`.
4. Load session-only plugins from `--plugin-dir` and `--plugin-url`.
5. `/reload-plugins` rescans without a full restart.

Precedence:

- Session-only plugins (`--plugin-dir`/`--plugin-url`) take precedence over installed marketplace plugins with the same name.
- Project `.claude/skills/` agents override same-named plugin agents.
- User/project standalone skills and commands do not collide with plugin resources because plugin skills/commands are namespaced.
- Managed settings override user/project settings.

Namespacing:

- Plugin skills and agents use `plugin-name:resource-name`, e.g. `/superpowers:tdd` or `@pr-review-toolkit:reviewer`.
- Plugin MCP and LSP servers are prefixed with the plugin identifier in tool listings.

Conflicts:

- Exact name conflicts within the same namespace are rejected.
- A plugin loaded via `--plugin-dir` overrides an installed plugin with the same name for that session, unless managed settings force-enable or force-disable it.

## Security and Runtime Behavior

Plugins are highly trusted components that run with user privileges:

- Hooks, MCP servers, LSP servers, monitors, and `bin/` executables run unsandboxed.
- Plugin MCP servers require the same per-server approval as user MCP servers.
- Project-scope plugin MCP, LSP, and monitors are gated by workspace trust.
- Plugins can reference environment variables via `${ENV_VAR}` substitution in configs and commands.
- `userConfig` values, including sensitive ones, are stored in `settings.json` (non-sensitive) or the system keychain / `~/.claude/.credentials.json` (sensitive).
- Plugins receive `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}`, and `${CLAUDE_PROJECT_DIR}` path variables.
- Auto-updates are enabled by default for official marketplaces, creating silent update risk. Pin `version` to mitigate.

## Distribution

Official distribution channels:

- Official marketplace: `claude-plugins-official`, curated by Anthropic, registered automatically on first interactive launch. Catalog at [claude.com/plugins](https://claude.com/plugins).
- Community marketplace: `anthropics/claude-plugins-community`, added manually via `claude plugin marketplace add anthropics/claude-plugins-community`.
- Demo marketplace: `anthropics/claude-code`.

Source types supported for marketplaces and plugins:

| Source | Example |
|---|---|
| GitHub shorthand | `anthropics/claude-plugins-official` |
| Git URL | `https://gitlab.com/company/plugins.git` |
| Git subdirectory | `git-subdir` with `url` and `path` |
| npm package | `@company/claude-plugin` |
| Local directory | `./my-marketplace` |
| Remote marketplace.json URL | `https://example.com/marketplace.json` |
| Zip archive | `--plugin-dir ./plugin.zip` or `--plugin-url https://example.com/plugin.zip` |

Publishing:

- Official marketplace inclusion is at Anthropic's discretion.
- Community submissions use review forms at [claude.ai/admin-settings/directory/submissions/plugins/new](https://claude.ai/admin-settings/directory/submissions/plugins/new) or [platform.claude.com/plugins/submit](https://platform.claude.com/plugins/submit).
- Private teams can host their own marketplace by publishing a `.claude-plugin/marketplace.json` file in a Git repo or local directory.
- Plugins can be published as npm packages or zip archives.

Private distribution:

- Private Git repositories work for manual install via existing git credential helpers.
- Background auto-updates for private repos require `GITHUB_TOKEN`/`GH_TOKEN`, `GITLAB_TOKEN`/`GL_TOKEN`, or `BITBUCKET_TOKEN`.
- Container images can pre-populate plugins via `CLAUDE_CODE_PLUGIN_SEED_DIR` and `CLAUDE_CODE_PLUGIN_CACHE_DIR`.

## Portability

Claudine should not link Claude Code plugins as intact units to other providers. Instead, it should extract the portable Markdown-based resources and flag Claude-specific assets for rewrite or omission.

Portable resources:

- `skills/<name>/SKILL.md` and root `SKILL.md` (Agent Skills standard frontmatter and body).
- `commands/*.md` flat command files.
- `agents/*.md` subagent definitions (minus Claude-specific security-blocked fields).
- `output-styles/*.md` and `themes/*.json`.
- Static assets in `assets/` and `README.md`.

Non-portable assets:

- `.claude-plugin/plugin.json` manifest.
- Marketplace entry metadata and IDs.
- `.mcp.json` / inline `mcpServers` configs and credentials.
- `.lsp.json` / inline `lspServers` binary paths.
- `userConfig` prompts and stored credentials.
- `hooks/hooks.json` hook command definitions.
- `scripts/` and `bin/` executables.
- `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}`, and `${CLAUDE_PROJECT_DIR}` references.
- `dependencies` and npm package references.
- `monitors/` and `channels` declarations.

Rewrite is needed for path variables, Claude-specific frontmatter, executable commands, and any provider-specific metadata. Scripts should be treated as host-dependent and linked only with explicit OS gating or replacement.

## Claudine Linking Notes

- Treat `~/.claude/plugins/` as the canonical plugin state root; respect `CLAUDE_CODE_PLUGIN_CACHE_DIR` if set.
- Parse `installed_plugins.json` and `known_marketplaces.json` to discover installed plugins and their source marketplaces.
- Read `enabledPlugins` from user, project, local, and managed settings to determine which plugins are active.
- For each enabled plugin, scan its cache directory for `skills/`, `commands/`, `agents/`, `output-styles/`, `themes/`, and `assets/`.
- Extract portable Markdown resources and rewrite namespaced invocation names (`plugin-name:skill-name`) into the target provider's format.
- Do not extract or link non-portable assets without explicit user confirmation and host-aware rewriting.
- Account for session-only plugins loaded via `--plugin-dir`/`--plugin-url` only when Claudine is wrapping a specific Claude Code invocation that includes those flags.
- Respect managed-settings restrictions (`strictKnownMarketplaces`, `blockedMarketplaces`, `disableSideloadFlags`) and do not suggest linking plugins from blocked sources.
- Consider versioned cache directories when determining which plugin version is currently active.

## Sources

- [Claude Code — Create plugins](https://code.claude.com/docs/en/plugins)
- [Claude Code — Plugins reference](https://code.claude.com/docs/en/plugins-reference)
- [Claude Code — Discover and install plugins](https://code.claude.com/docs/en/discover-plugins)
- [Claude Code — Create and distribute a plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces)
- [Claude Code — CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code — Settings](https://code.claude.com/docs/en/settings)
- [Claude Code — Environment variables](https://code.claude.com/docs/en/env-vars)
- [Claude Code — Subagents](https://code.claude.com/docs/en/sub-agents)
- [Claude Code — Hooks](https://code.claude.com/docs/en/hooks)
- [Claude Code — MCP](https://code.claude.com/docs/en/mcp)
- [Claude Code product homepage](https://www.anthropic.com/claude-code)
