---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://openai.com/codex
docs: https://developers.openai.com/codex
plugin_docs: https://developers.openai.com/codex/plugins

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/
    notes: Installed plugin cache. Each marketplace/plugin/version combination gets a separate directory. Observed on this host for github@openai-curated and gmail@openai-curated.
  - os: linux
    scope: user
    path: ~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/
    notes: Same versioned cache layout as macOS; confirmed by CLI help and documentation.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\plugins\\cache\\<marketplace>\\<plugin>\\<version>\\"
    notes: Windows resolves ~ to %USERPROFILE%; cache layout mirrors Unix.
  - os: macos
    scope: user
    path: ~/.codex/config.toml
    notes: Stores [plugins."<name>@<marketplace>"] enabled = true/false and plugin-scoped MCP server policy.
  - os: linux
    scope: user
    path: ~/.codex/config.toml
    notes: User-level plugin enablement and MCP policy.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\config.toml"
    notes: User-level plugin enablement and MCP policy.
  - os: macos
    scope: repo
    path: .agents/plugins/marketplace.json
    notes: Repo-scoped marketplace catalog. Legacy-compatible path .claude-plugin/marketplace.json is also supported.
  - os: linux
    scope: repo
    path: .agents/plugins/marketplace.json
    notes: Repo marketplace catalog.
  - os: windows
    scope: repo
    path: ".agents\\plugins\\marketplace.json"
    notes: Repo marketplace catalog.
  - os: macos
    scope: user
    path: ~/.agents/plugins/marketplace.json
    notes: Personal marketplace catalog.
  - os: linux
    scope: user
    path: ~/.agents/plugins/marketplace.json
    notes: Personal marketplace catalog.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\plugins\\marketplace.json"
    notes: Personal marketplace catalog.
  - os: macos
    scope: marketplace
    path: ~/.codex/.tmp/plugins/.agents/plugins/marketplace.json
    notes: Snapshot of a Git-backed marketplace on this host. The CLI reports the marketplace root as ~/.codex/.tmp/plugins for openai-curated.
  - os: linux
    scope: marketplace
    path: ~/.codex/.tmp/plugins/.agents/plugins/marketplace.json
    notes: Marketplace snapshot root observed on this host.
  - os: windows
    scope: marketplace
    path: "%USERPROFILE%\\.codex\\.tmp\\plugins\\.agents\\plugins\\marketplace.json"
    notes: Marketplace snapshot root on Windows.

manifest:
  file_names:
    - .codex-plugin/plugin.json
  format: json
  required_fields:
    - name
  optional_fields:
    - version
    - description
    - author
    - homepage
    - repository
    - license
    - keywords
    - skills
    - apps
    - mcpServers
    - hooks
    - interface
    - interface.displayName
    - interface.shortDescription
    - interface.longDescription
    - interface.developerName
    - interface.category
    - interface.capabilities
    - interface.websiteURL
    - interface.privacyPolicyURL
    - interface.termsOfServiceURL
    - interface.defaultPrompt
    - interface.brandColor
    - interface.composerIcon
    - interface.logo
    - interface.screenshots
  package_layout: |
    Plugin root contains .codex-plugin/plugin.json and component directories/files at the root:
    - skills/<skill-name>/SKILL.md for bundled Agent Skills
    - agents/<agent-name>.md for subagent definitions
    - commands/<command-name>.md for slash commands
    - hooks/hooks.json for lifecycle hooks (or hooks/ directory with a default hooks.json)
    - .app.json for app/connector mappings
    - .mcp.json for MCP server configuration
    - assets/ for icons, logos, screenshots
    - scripts/ for executable scripts referenced by hooks
    - bin/ for executables added to the tool PATH
    - README.md and LICENSE as optional documentation
    Only plugin.json belongs inside .codex-plugin/; every other file lives at the plugin root.
  notes: |
    All component paths in plugin.json must be relative to the plugin root and start with "./". If hooks is omitted, Codex checks hooks/hooks.json automatically. .mcp.json may contain either a direct server map or a wrapped "mcp_servers" object. apps points to .app.json which maps plugin-local app IDs to ChatGPT connector IDs. The marketplace entry (marketplace.json) duplicates some metadata and adds policy.installation (AVAILABLE, INSTALLED_BY_DEFAULT, NOT_AVAILABLE), policy.authentication (ON_INSTALL, ON_FIRST_USE), and category.

lifecycle:
  install: |
    CLI: codex plugin add <plugin[@marketplace]> [--marketplace NAME] [--json]. The plugin is copied into ~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/ and enabled in ~/.codex/config.toml as [plugins."<name>@<marketplace>"] enabled = true. Marketplace plugins can also be installed through the TUI plugin browser (/plugins or codex then browse). Apps bundled with a plugin may prompt for ChatGPT authentication during install or first use.
  update: |
    Git-backed marketplaces are refreshed with codex plugin marketplace upgrade [NAME]; plugin contents are re-copied into a new versioned cache directory. There is no documented standalone "codex plugin update" command. Version resolves from plugin.json version, then marketplace entry version, then git commit SHA. Local plugins are reloaded by restarting Codex after updating the source directory.
  remove: |
    CLI: codex plugin remove <plugin[@marketplace]> [--marketplace NAME] [--json]. Removes the enabled entry from config.toml and deletes the plugin cache. Apps installed through ChatGPT remain installed until managed there separately.
  enable_disable: |
    Set [plugins."<name>@<marketplace>"] enabled = true/false in ~/.codex/config.toml, then restart Codex. There is no dedicated CLI enable/disable subcommand; toggling is config-driven. The TUI plugin browser also allows Space to toggle enabled state.
  trust: |
    Trust is established by explicit install/enable action and by approving the plugin in the TUI/app. Plugin-bundled lifecycle hooks are non-managed hooks, so Codex skips them until the user reviews and trusts the current hook definition. There is no code-signing or sandbox boundary specific to plugins.
  versioning: |
    Cache paths include a version string derived from plugin.json version, marketplace entry version, or git SHA. For local sources the version is "local". Pinning a Git marketplace with --ref or a git-subdir entry with ref/sha controls update behavior.
  notes: |
    Lifecycle is CLI/UI/config driven. Restart is required after adding, removing, enabling, disabling, or updating marketplace snapshots. There is no documented "safe mode" flag that disables only plugins, though disabling the hooks feature or individual plugins achieves similar results.

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
    - apps
    - app_connectors
    - commands

discovery:
  mechanism: |
    Codex reads configured marketplace snapshots at startup, loads enabled plugins from ~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/, and registers bundled skills, apps, MCP servers, hooks, commands, and agents. The CLI reports installed plugins via codex plugin list --json. Discovery requires a marketplace file (.agents/plugins/marketplace.json) or a marketplace added with codex plugin marketplace add.
  precedence: |
    User-level ~/.codex/config.toml plugin enablement overrides defaults. Project-scoped config (.codex/config.toml) can only override non-host-owned keys; plugin enablement appears to be user-scope only on this host. There is no documented plugin shadowing of user skills or commands.
  namespacing: |
    Plugin skills and agents are invoked with @plugin-name or @plugin-name:skill-name. Commands are exposed as slash commands scoped under the plugin. MCP servers bundled in a plugin are namespaced under plugins.<plugin>.mcp_servers.<server> in config.toml.
  conflicts: |
    Exact plugin IDs (name@marketplace) must be unique. The marketplace file controls ordering and visibility. If a marketplace entry cannot be resolved, Codex skips that entry rather than failing the whole marketplace.
  notes: |
    The official openai-curated marketplace is configured automatically on this host. Local plugins require a marketplace entry pointing source.path to the plugin directory with a "./"-prefixed relative path.

security:
  trust_model: |
    Plugins are highly trusted: installation/enablement is the trust boundary. OpenAI does not sign or sandbox plugin bundles. Users must trust the marketplace source (official, community, repo, personal, or Git). Plugin hooks require an additional trust review before they run.
  permissions: |
    Plugin skills inherit the user's approval_policy. Plugin MCP servers require the same per-server approval as user MCP servers and can be gated via plugins.<plugin>.mcp_servers.<server>.identity allowlists. Plugin apps/connectors follow ChatGPT app permissions and OAuth flows.
  sandbox_interaction: |
    Plugin-provided hooks, scripts, MCP servers, and bin/ executables run with the user's OS privileges, not inside Codex's sandbox. The sandbox policy (--sandbox) applies to model-generated shell commands, not to plugin-owned executables.
  credential_access: |
    Plugins can reference environment variables via standard shell expansion in hook commands and MCP config. MCP bearer tokens are sourced from env vars declared in .mcp.json (e.g. bearer_token_env_var). Plugin hooks receive PLUGIN_ROOT, PLUGIN_DATA, CLAUDE_PLUGIN_ROOT, and CLAUDE_PLUGIN_DATA.
  update_risk: |
    Medium to high. Git-backed marketplaces auto-update when codex plugin marketplace upgrade runs, and the official marketplace snapshot may refresh in the background. A new commit produces a new versioned cache directory. Version pinning via --ref, ref, or sha mitigates silent changes.
  notes: |
    Installing a plugin makes its workflows available but does not bypass the user's approval settings. Data sent through a plugin's app/connector is subject to that app's terms and privacy policy.

distribution:
  marketplace: true
  registry_url: https://developers.openai.com/codex/plugins
  source_types:
    - marketplace (official openai-curated)
    - repo marketplace (.agents/plugins/marketplace.json)
    - personal marketplace (~/.agents/plugins/marketplace.json)
    - github shorthand (owner/repo)
    - git url (https/ssh)
    - git-subdir
    - local directory
  publishing: |
    Self-serve publishing to the official Plugin Directory is documented as "coming soon" (Build plugins page, July 2026). Today plugins are distributed through private marketplaces, workspace sharing in the Codex app, or local marketplace files.
  private_distribution: |
    Private teams can host a marketplace by committing .agents/plugins/marketplace.json to a Git repo and adding it with codex plugin marketplace add owner/repo --ref main. Workspace sharing inside a ChatGPT workspace is also supported from the Codex app and can be disabled by admins via requirements.toml features.plugin_sharing = false.
  notes: |
    The official marketplace (openai-curated) is pre-configured on this host. Marketplace names are identified by the top-level "name" field in marketplace.json. Codex installs plugins into the versioned cache even for local sources.

portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources:
    - skills
    - slash_commands
    - subagents
    - assets
    - README.md
  non_portable_assets:
    - .codex-plugin/plugin.json manifest
    - .app.json connector mappings
    - .mcp.json MCP configs and credentials
    - hooks/hooks.json and scripts/
    - bin/ executables
    - package.json/npm dependencies
    - marketplace entry metadata
    - PLUGIN_ROOT / PLUGIN_DATA / CLAUDE_PLUGIN_ROOT references
  rewrite_needed: true
  notes: |
    Claudine should extract Markdown-based resources (skills/<name>/SKILL.md, commands/*.md, agents/*.md) and static assets for linking to other providers. The plugin manifest, app mappings, MCP configs, hook commands, executable scripts, and any connector-specific metadata are Codex-specific and must be rewritten or omitted. Namespaced invocations like @plugin-name:skill-name need translation into the target provider's format.

cli_params:
  - flag: codex plugin add <plugin[@marketplace]>
    description: Install a plugin from a configured marketplace.
    example: codex plugin add github@openai-curated
  - flag: codex plugin add <plugin> --marketplace <NAME> --json
    description: Install with explicit marketplace and JSON output.
    example: codex plugin add github --marketplace openai-curated --json
  - flag: codex plugin list [--json] [--available]
    description: List installed plugins; --available requires --json and includes uninstalled marketplace plugins.
    example: codex plugin list --json --available
  - flag: codex plugin remove <plugin[@marketplace]>
    description: Remove an installed plugin from config and cache.
    example: codex plugin remove github@openai-curated
  - flag: codex plugin marketplace add <source> [--ref REF] [--sparse PATH]
    description: Add a marketplace from GitHub shorthand, Git URL, SSH URL, or local directory. --sparse is Git-only and repeatable.
    example: codex plugin marketplace add openai/plugins --ref main
  - flag: codex plugin marketplace list [--json]
    description: Show configured marketplaces and their resolved roots.
    example: codex plugin marketplace list --json
  - flag: codex plugin marketplace upgrade [NAME] [--json]
    description: Refresh one or all Git-backed marketplace snapshots.
    example: codex plugin marketplace upgrade openai-curated
  - flag: codex plugin marketplace remove <NAME>
    description: Remove a configured marketplace source.
    example: codex plugin marketplace remove openai-curated
  - flag: --dangerously-bypass-hook-trust
    description: Run enabled hooks without requiring persisted hook trust for this invocation.
    example: codex --dangerously-bypass-hook-trust -p "task"
  - flag: --enable hooks / --disable hooks
    description: Toggle the hooks feature flag, which affects plugin hooks too.
    example: codex --disable hooks -p "task"

env_vars:
  - name: CODEX_HOME
    effect: Root for Codex state (~/.codex), including config, auth, logs, sessions, skills, and plugin cache.
  - name: CODEX_SQLITE_HOME
    effect: Overrides where SQLite-backed state is stored; sqlite_home config option takes precedence.
  - name: CODEX_API_KEY
    effect: API key for a single non-interactive codex exec run; not plugin-specific but relevant to automation.
  - name: CODEX_ACCESS_TOKEN
    effect: ChatGPT/Codex access token for trusted automation; used for login.
  - name: RUST_LOG
    effect: Controls Rust log filtering/verbosity for CLI and app-server diagnostics.

gaps:
  - No documented JSON Schema URL for .codex-plugin/plugin.json or marketplace.json.
  - No dedicated CLI enable/disable subcommand for plugins; toggling is config-driven via ~/.codex/config.toml.
  - The config reference documents plugins.<plugin>.mcp_servers.* but not the observed [plugins."name@marketplace"] enabled = true table.
  - No documented standalone "codex plugin update" command; updates are marketplace-upgrade driven.
  - No documented safe-mode flag that disables only plugins.
  - Plugin hook trust review prompt contents are not documented in detail.

changes: []

requires_claudine_update: true
reason: |
  Claudine should model Codex's first-class plugin container: ~/.codex/plugins/cache versioned directories, marketplace snapshots under ~/.codex/.tmp/plugins, .agents/plugins/marketplace.json catalogs, and [plugins."name@marketplace"] enablement in ~/.codex/config.toml. Linking should extract portable Markdown resources (skills, commands, agents, assets) while flagging non-portable assets (plugin manifest, .app.json, .mcp.json, hooks, scripts, bin/, marketplace metadata, and PLUGIN_ROOT references) as requiring rewrite or omission. The linker also needs to respect plugin-scoped MCP allowlists and hook trust state.
---

# Codex CLI Plugins

## Overview

Codex CLI has a first-class, experimental plugin system that bundles skills, app integrations, MCP servers, lifecycle hooks, slash commands, and subagents into a single installable, namespaced unit. A plugin is a directory containing a `.codex-plugin/plugin.json` manifest and zero or more component directories or files at the plugin root: `skills/`, `agents/`, `commands/`, `hooks/`, `.app.json`, `.mcp.json`, and `assets/`.

Compared with Claude Code, Codex's plugin model is younger and smaller in scope but follows a similar container pattern. Both systems use a JSON manifest, a namespaced skill/agent invocation model (`@plugin-name:skill-name`), a versioned user-scope cache, and marketplace-based distribution. The key differences are:

| Area | Codex CLI | Claude Code |
|---|---|---|
| Manifest path | `.codex-plugin/plugin.json` | `.claude-plugin/plugin.json` |
| User cache | `~/.codex/plugins/cache/...` | `~/.claude/plugins/cache/...` |
| Enablement | `[plugins."name@marketplace"]` in `~/.codex/config.toml` | `enabledPlugins` in `~/.claude/settings.json` |
| Marketplace file | `.agents/plugins/marketplace.json` or `~/.agents/plugins/marketplace.json` | `.claude-plugin/marketplace.json` or cloned Git marketplaces under `~/.claude/plugins/marketplaces/` |
| Hook trust | Plugin hooks require an explicit trust review before running | Hooks run after enablement; trust is install/enable plus managed settings |
| App integration | Bundled apps map to ChatGPT connectors via `.app.json` | Bundled MCP/LSP servers and app connectors via `.mcp.json`/`.lsp.json` |
| Update model | Marketplace snapshots refreshed with `codex plugin marketplace upgrade` | Per-plugin update plus official-marketplace auto-update |

Sources: [Codex Plugins overview](https://developers.openai.com/codex/plugins), [Build plugins](https://developers.openai.com/codex/plugins/build), [Codex CLI reference](https://developers.openai.com/codex/cli/reference), [Claude Code Plugins reference](https://code.claude.com/docs/en/plugins-reference).

## Installation and Locations

Codex stores plugin state under `~/.codex/` (or `%USERPROFILE%\.codex\` on Windows). The layout observed on this host is:

| Path | Purpose |
|---|---|
| `~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/` | Installed plugin contents. Each version is a separate directory. |
| `~/.codex/config.toml` | Plugin enablement (`[plugins."name@marketplace"]`) and plugin-scoped MCP policy. |
| `~/.codex/.tmp/plugins/` | Snapshot root for the configured `openai-curated` Git marketplace on this host. |
| `~/.codex/.tmp/plugins/.agents/plugins/marketplace.json` | Marketplace catalog for the `openai-curated` source. |
| `~/.codex/.tmp/plugins/plugins/<plugin>/` | Source plugins referenced by the local marketplace entry. |

On this host, `codex plugin list --json` reported two installed plugins:

```json
{
  "installed": [
    {
      "pluginId": "gmail@openai-curated",
      "name": "gmail",
      "marketplaceName": "openai-curated",
      "version": "3fdeeb49",
      "installed": true,
      "enabled": true,
      "source": { "source": "local", "path": "/Users/ken/.codex/.tmp/plugins/plugins/gmail" }
    },
    {
      "pluginId": "github@openai-curated",
      "name": "github",
      "marketplaceName": "openai-curated",
      "version": "3fdeeb49",
      "installed": true,
      "enabled": true,
      "source": { "source": "local", "path": "/Users/ken/.codex/.tmp/plugins/plugins/github" }
    }
  ]
}
```

The corresponding config.toml entries are:

```toml
[plugins."gmail@openai-curated"]
enabled = true

[plugins."github@openai-curated"]
enabled = true
```

Marketplaces are configured at the repo or user level via `.agents/plugins/marketplace.json` (legacy-compatible path `.claude-plugin/marketplace.json` also supported). The marketplace root can be a Git repo, a local directory, or a Git subdirectory.

## Manifest and Package Format

The plugin manifest lives at `.codex-plugin/plugin.json` relative to the plugin root. The only required field is `name`, which must be stable kebab-case and serves as the plugin identifier and namespace.

Observed manifest from the GitHub plugin:

```json
{
  "name": "github",
  "version": "0.1.6",
  "description": "Inspect repositories, triage pull requests and issues, debug CI, and publish changes through a hybrid GitHub connector and CLI workflow.",
  "author": { "name": "OpenAI", "email": "support@openai.com", "url": "https://openai.com/" },
  "homepage": "https://github.com/",
  "repository": "https://github.com/openai/plugins",
  "license": "MIT",
  "keywords": ["github", "pull-request", "code-review", "issues", "ci", "actions"],
  "skills": "./skills/",
  "apps": "./.app.json",
  "mcpServers": "./.mcp.json",
  "interface": {
    "displayName": "GitHub",
    "shortDescription": "Triage PRs, issues, CI, and publish flows",
    "longDescription": "...",
    "developerName": "OpenAI",
    "category": "Developer Tools",
    "capabilities": ["Interactive", "Write"],
    "websiteURL": "https://github.com/",
    "privacyPolicyURL": "...",
    "termsOfServiceURL": "...",
    "defaultPrompt": ["Inspect PRs, triage issues, debug failing checks, and prepare code changes for review"],
    "composerIcon": "./assets/github-small.svg",
    "logo": "./assets/logo.png",
    "screenshots": [],
    "brandColor": "#24292F"
  }
}
```

The `interface` object is for install-surface metadata only; it does not affect runtime loading.

Package layout at the plugin root:

```text
my-plugin/
├── .codex-plugin/
│   └── plugin.json
├── skills/
│   └── my-skill/
│       └── SKILL.md
├── agents/
│   └── reviewer.md
├── commands/
│   └── deploy.md
├── hooks/
│   └── hooks.json
├── .app.json
├── .mcp.json
├── assets/
├── scripts/
├── bin/
├── README.md
└── LICENSE
```

Path rules:

- Manifest component paths (`skills`, `apps`, `mcpServers`, `hooks`, interface asset paths) must be relative to the plugin root and start with `./`.
- If `hooks` is omitted, Codex checks `hooks/hooks.json` automatically.
- `.mcp.json` may contain either a direct server map or `{ "mcp_servers": { ... } }`.
- `.app.json` maps plugin-local app names to ChatGPT connector IDs, as observed in the GitHub plugin:

```json
{
  "apps": {
    "github": {
      "id": "connector_76869538009648d5b282a4bb21c3d157"
    }
  }
}
```

## Packaged Resources

Codex plugins can contain the following resource types:

| Resource | Support | Location | Notes |
|---|---|---|---|
| Agent Skills | Full | `skills/<name>/SKILL.md` | Invoked with `@plugin-name` or `@plugin-name:skill-name`. |
| Subagents | Full | `agents/<name>.md` | Plugin agent definitions. |
| Slash commands | Full | `commands/<name>.md` | Exposed as slash commands scoped to the plugin. |
| MCP servers | Full | `.mcp.json` or inline `mcpServers` | Start when the plugin is enabled; policy can be overridden per server. |
| Hooks | Full | `hooks/hooks.json` or manifest `hooks` | Plugin hooks require trust review before running. |
| Apps / connectors | Full | `.app.json` | Maps plugin apps to ChatGPT connectors. |
| Scripts | Partial | `scripts/`, `bin/` | Referenced by hooks or added to the tool PATH; no automatic discovery as standalone commands. |
| Config | Partial | `[plugins."name@marketplace"]` in `~/.codex/config.toml` | Only enablement and MCP server policy are documented/observed. |
| Prompts | None | — | Skills serve this role. |
| Assets | Full | `assets/`, `README.md`, `LICENSE` | Icons, logos, screenshots, documentation. |

The GitHub plugin bundles four skills (`github`, `gh-address-comments`, `gh-fix-ci`, `yeet`) and one MCP server pointing to `https://api.githubcopilot.com/mcp/` with a `bearer_token_env_var` of `GITHUB_PAT_TOKEN`. The Figma plugin bundles skills, an `.mcp.json`, and a `hooks.json` that runs `./scripts/post_write_figma_parity_check.sh` on `PostToolUse` write events.

## Lifecycle and Trust

Install:

- CLI: `codex plugin add <plugin>[@<marketplace>]` or `codex plugin add <plugin> --marketplace <NAME>`.
- TUI: run `codex`, then `/plugins`, browse, and install.
- The plugin is copied into `~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/` and enabled in `~/.codex/config.toml`.

Update:

- Git-backed marketplaces are refreshed with `codex plugin marketplace upgrade [NAME]`.
- A new version produces a new cache directory; the previous version remains until removed.
- Local plugins pick up changes after updating the source directory and restarting Codex.

Remove:

- CLI: `codex plugin remove <plugin>[@<marketplace>]`.
- Removes the cache and the config.toml entry. Apps installed via ChatGPT are not removed.

Enable / disable:

- Edit `~/.codex/config.toml`: `[plugins."name@marketplace"]` `enabled = true/false`.
- TUI plugin browser: Space toggles enabled state.
- Restart Codex after changing enablement.

Trust:

- Installation/enablement is the primary trust boundary.
- Plugin hooks are non-managed hooks and are skipped until the user explicitly trusts the current hook definition.
- There is no plugin code signing or sandbox boundary.

## Discovery and Precedence

Discovery mechanism:

1. Read configured marketplace snapshots (from `codex plugin marketplace add` or `.agents/plugins/marketplace.json`).
2. Load enabled plugins from `~/.codex/plugins/cache/<marketplace>/<plugin>/<version>/`.
3. Register bundled skills, agents, commands, apps, MCP servers, and hooks.
4. Plugin hooks remain inactive until trusted.

Precedence:

- User config.toml enablement overrides marketplace defaults.
- Plugin MCP server policy can be overridden via `plugins.<plugin>.mcp_servers.<server>.*`.
- There is no documented shadowing between plugin resources and user/repo resources; plugins are namespaced by plugin ID.

Namespacing:

- Plugin ID format: `name@marketplace`.
- Skill/agent invocation: `@plugin-name` or `@plugin-name:skill-name`.
- MCP server config key: `plugins.<plugin>.mcp_servers.<server>`.

Conflicts:

- Duplicate plugin IDs within the same marketplace are not allowed.
- Unresolvable marketplace entries are skipped rather than failing the whole catalog.

## Security and Runtime Behavior

Plugins run with high trust and user privileges:

- Plugin hooks, scripts, MCP servers, and `bin/` executables run outside Codex's sandbox.
- They share the user's environment and can access the filesystem and network subject to normal OS permissions.
- Plugin MCP servers require the same approval as user MCP servers; administrators can restrict them with identity allowlists.
- Plugin apps authenticate through ChatGPT OAuth; data handled by those apps is subject to the app's terms and privacy policy.
- Hooks receive `PLUGIN_ROOT`, `PLUGIN_DATA`, `CLAUDE_PLUGIN_ROOT`, and `CLAUDE_PLUGIN_DATA` environment variables.

Update risk is medium to high because Git-backed marketplaces refresh in the background and official snapshots may update silently. Pinning with `--ref`, `ref`, or `sha` mitigates this.

## Distribution

Official distribution channels:

- Official curated marketplace (`openai-curated`), pre-configured on this host.
- Self-serve public directory is documented as "coming soon" as of July 2026.

Source types supported for marketplaces and plugins:

| Source | Example |
|---|---|
| GitHub shorthand | `openai/plugins` |
| Git URL | `https://github.com/example/plugins.git` |
| SSH Git URL | `git@github.com:example/plugins.git` |
| Git subdirectory | `git-subdir` with `url` and `path` |
| Local directory | `./local-marketplace-root` |

Publishing:

- Public directory submissions are not yet open.
- Teams can publish private marketplaces by hosting `.agents/plugins/marketplace.json` in a Git repo.
- Workspace sharing is available in the Codex app for ChatGPT workspace members and can be disabled by admins via `requirements.toml`.

Marketplace file example:

```json
{
  "name": "local-repo",
  "interface": { "displayName": "Local Repo Plugins" },
  "plugins": [
    {
      "name": "my-plugin",
      "source": { "source": "local", "path": "./plugins/my-plugin" },
      "policy": { "installation": "AVAILABLE", "authentication": "ON_INSTALL" },
      "category": "Productivity"
    }
  ]
}
```

## Portability

Claudine should not link Codex plugins as intact units to other providers. Instead, it should extract the portable Markdown-based resources and flag Codex-specific assets for rewrite or omission.

Portable resources:

- `skills/<name>/SKILL.md` (Agent Skills standard frontmatter and body).
- `agents/<name>.md` (subagent definitions).
- `commands/<name>.md` (slash command files).
- Static assets in `assets/` and documentation such as `README.md`.

Non-portable assets:

- `.codex-plugin/plugin.json` manifest.
- `.app.json` ChatGPT connector mappings.
- `.mcp.json` / inline `mcpServers` configs and credentials.
- `hooks/hooks.json` hook command definitions.
- `scripts/` and `bin/` executables.
- `PLUGIN_ROOT`, `PLUGIN_DATA`, `CLAUDE_PLUGIN_ROOT`, and `CLAUDE_PLUGIN_DATA` references.
- Marketplace entry metadata and IDs.
- npm/package.json dependencies.

Rewrite is needed for namespaced invocation names (`@plugin-name:skill-name`), Codex-specific frontmatter, hook commands, and any connector-specific metadata.

## Claudine Linking Notes

- Treat `~/.codex/plugins/cache/` as the canonical installed-plugin root; respect `CODEX_HOME` if set.
- Parse `~/.codex/config.toml` for `[plugins."name@marketplace"]` enablement tables.
- Discover configured marketplaces with `codex plugin marketplace list --json` and read their `.agents/plugins/marketplace.json` catalogs.
- For each enabled plugin, scan its cache directory for `skills/`, `agents/`, `commands/`, and `assets/`.
- Extract portable Markdown resources and rewrite namespaced invocation names into the target provider's format.
- Do not extract or link `.app.json`, `.mcp.json`, hook definitions, scripts, or `bin/` without explicit user confirmation and host-aware rewriting.
- Respect plugin hook trust state: do not link hook-driven resources from plugins whose hooks have not been trusted.
- Account for versioned cache directories when determining which plugin version is active.

## Sources

- [Codex — Plugins overview](https://developers.openai.com/codex/plugins)
- [Codex — Build plugins](https://developers.openai.com/codex/plugins/build)
- [Codex — Command line options](https://developers.openai.com/codex/cli/reference)
- [Codex — Configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex — Environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex — Hooks](https://developers.openai.com/codex/hooks)
- [Codex — MCP](https://developers.openai.com/codex/mcp)
- [Codex product homepage](https://openai.com/codex)
- [Codex GitHub repository](https://github.com/openai/codex)
- [Claude Code — Plugins reference](https://code.claude.com/docs/en/plugins-reference)
