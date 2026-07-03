---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://goose-docs.ai/
docs: https://goose-docs.ai/docs/
plugin_docs: https://goose-docs.ai/docs/guides/context-engineering/plugins

support: partial

locations:
  - os: macos
    scope: user
    path: "~/.agents/plugins/<plugin-name>/"
    notes: "Canonical user plugin directory. `goose plugin install` clones git-backed plugins here; manually copied plugins are also discovered."
  - os: linux
    scope: user
    path: "~/.agents/plugins/<plugin-name>/"
    notes: "Same user plugin directory as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\plugins\\<plugin-name>\\"
    notes: "Inferred from the `~/.agents/plugins/` template; Goose docs do not explicitly document a Windows plugin path."
  - os: macos
    scope: repo
    path: ".agents/plugins/<plugin-name>/"
    notes: "Project-scoped plugins, available only when goose is launched from that project."
  - os: linux
    scope: repo
    path: ".agents/plugins/<plugin-name>/"
    notes: "Project-scoped plugins."
  - os: windows
    scope: repo
    path: ".agents\\plugins\\<plugin-name>\\"
    notes: "Project-scoped plugins; Windows path inferred from the cross-platform template."
  - os: macos
    scope: user
    path: "~/.config/goose/settings.json"
    notes: "User settings file; `disabledPlugins` disables plugins globally."
  - os: linux
    scope: user
    path: "~/.config/goose/settings.json"
    notes: "User settings file for `disabledPlugins`."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\settings.json"
    notes: "Windows user settings file for `disabledPlugins`."
  - os: macos
    scope: repo
    path: ".config/goose/settings.json"
    notes: "Project-level settings; shared with teammates."
  - os: linux
    scope: repo
    path: ".config/goose/settings.json"
    notes: "Project-level settings."
  - os: windows
    scope: repo
    path: ".config\\goose\\settings.json"
    notes: "Project-level settings."
  - os: macos
    scope: repo
    path: ".config/goose/settings.local.json"
    notes: "Local gitignored overrides for disabling plugins."
  - os: linux
    scope: repo
    path: ".config/goose/settings.local.json"
    notes: "Local gitignored overrides."
  - os: windows
    scope: repo
    path: ".config\\goose\\settings.local.json"
    notes: "Local gitignored overrides."

manifest:
  file_names:
    - plugin.json
    - .plugin/plugin.json
    - .goose-plugin/plugin.json
    - gemini-extension.json
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
    - logo
    - keywords
    - commands
    - agents
    - skills
    - rules
    - hooks
    - mcpServers
    - lspServers
    - outputStyles
  package_layout: |
    Open Plugins standard layout:
    - plugin.json, .plugin/plugin.json, or .goose-plugin/plugin.json at the plugin root (metadata directory)
    - skills/<skill-name>/SKILL.md for Agent Skills
    - hooks/hooks.json for lifecycle hooks
    - scripts/ for hook helper scripts
    - optional standard directories: commands/, agents/, rules/, assets/
    - optional config files: .mcp.json, .lsp.json
    Gemini-style plugins use gemini-extension.json plus a skills/ directory.
  notes: |
    The manifest is optional for Open Plugins; when absent the plugin name is derived from the directory name. Goose docs only confirm skills and hooks as plugin contents; the broader Open Plugins component directories (commands, agents, rules, MCP, LSP) are not documented as consumed by Goose. Hook-only plugins can be discovered from hooks/hooks.json without a manifest. Relative paths must start with `./` and cannot traverse above the plugin root.

lifecycle:
  install: |
    `goose plugin install <git-url>` clones a git repository into `~/.agents/plugins/<plugin-name>/`. Manual copy into `~/.agents/plugins/` or `.agents/plugins/` is also discovered. There is no documented marketplace-based install command.
  update: |
    `goose plugin update <plugin-name>` fetches the latest copy from the original git source and replaces the installed directory, preserving the `--auto-update` flag. `goose plugin install --auto-update <git-url>` enables automatic update checks before plugin skills are loaded; these checks are rate-limited and failures are logged without blocking the session. Manually copied plugins are not managed by `goose plugin update`.
  remove: |
    No documented CLI uninstall/remove command. Remove a plugin by deleting its directory under `.agents/plugins/` and removing its name from `disabledPlugins` if present.
  enable_disable: |
    Disable a plugin by adding its name to the `disabledPlugins` array in `~/.config/goose/settings.json`, `.config/goose/settings.json`, or `.config/goose/settings.local.json`. There is no explicit `enable` command; removing the name from `disabledPlugins` re-enables discovery.
  trust: |
    No signature, sandbox, or install-time trust prompt is documented. Trust is established by choosing to install or copy a plugin from a source. Hooks execute local shell commands, so Goose warns users to install trusted plugins only.
  versioning: |
    The `version` field is optional and should follow Semantic Versioning. Git-backed plugins update from source; the version field is not enforced as a pin.
  notes: |
    Lifecycle is CLI-driven for install/update and config-driven for disable. Auto-update only applies to git-backed plugins installed with the `--auto-update` flag.

packaged_resources:
  skills: full
  scripts: partial
  slash_commands: none
  subagents: none
  mcp_servers: none
  hooks: full
  prompts: none
  config: partial
  assets: partial
  other: []

discovery:
  mechanism: |
    Goose scans `~/.agents/plugins/<name>/` and `<project>/.agents/plugins/<name>/` at session startup, skipping any plugin listed in `disabledPlugins`. Hook-only plugins are discovered via `hooks/hooks.json`. Plugin skills are added to the session instructions and loaded automatically when the task matches or when explicitly requested with `/skills <name>`.
  precedence: |
    Project plugins are available only when goose is launched from that project; user plugins are available across projects. Disabled plugins are skipped entirely. Goose docs do not specify precedence rules for name collisions between user and project plugins or between plugins and standalone skills.
  namespacing: |
    For Open Plugins, imported skill names are namespaced as `plugin-name:skill-name` (e.g. `my-plugin:review`). Gemini extension skills keep the original `SKILL.md` name and are not prefixed with the extension name. Hook commands are not namespaced in the same way; they reference `${PLUGIN_ROOT}`.
  conflicts: |
    Not documented. The Open Plugins spec requires component namespacing to prevent conflicts, but Goose does not state how exact-name collisions between plugins or between plugin and standalone skills are resolved.
  notes: |
    Standalone skills live in `~/.agents/skills/` and `.agents/skills/` and are loaded independently of plugins. The Summon extension is required for skill loading.

security:
  trust_model: |
    Plugins are trusted by source and install action. There is no code signing, certificate pinning, or install-time audit. Goose checks external extensions for known malware before activation, but this is an extension feature, not a plugin feature.
  permissions: |
    Plugin skills add instructions to the session. Hooks run shell commands via `sh -c` with user privileges and a default 30-second timeout. There is no additional permission prompt before a hook runs. The user must approve installation of a plugin; after that, hooks execute automatically for matching events.
  sandbox_interaction: |
    Hook scripts and commands are not sandboxed. They run with the user's environment and OS permissions. Goose Desktop supports an optional macOS App Sandbox via `GOOSE_SANDBOX`, but this is a global Desktop feature, not plugin-specific isolation.
  credential_access: |
    Hook commands inherit the user's shell environment and can read environment variables. Goose sets `PLUGIN_ROOT` so hooks can reference files inside the plugin. There is no plugin-specific credential store; secrets are stored in the system keyring or `~/.config/goose/secrets.yaml` when keyring is unavailable.
  update_risk: |
    Auto-updating git-backed plugins can pull new code before skills are loaded, silently changing behavior. The optional `version` field is informational; Goose does not block loading if the source has diverged.
  notes: |
    Failures and timeouts in hooks are logged but do not crash goose or the tool that triggered the hook. Hook scripts should be executable and use absolute paths or `${PLUGIN_ROOT}` for portability.

distribution:
  marketplace: false
  source_types:
    - git repo
    - local folder
  publishing: |
    Distribute a plugin as a git repository or a local directory. Users install it with `goose plugin install <git-url>` or by copying it into `.agents/plugins/`. There is no documented publishing flow, registry upload, or package archive format.
  private_distribution: |
    Any git repository works, including private repos. Background auto-update relies on the host's git credentials (e.g. SSH keys or git credential helpers). There is no separate private-marketplace mechanism.
  notes: |
    The Goose Skills Marketplace (`/skills`) and Extensions directory (`/extensions`) are discovery surfaces, but neither is a plugin marketplace. Plugins are not listed in the Extensions directory.

portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources:
    - skills
  non_portable_assets:
    - plugin manifest (plugin.json)
    - hooks/hooks.json
    - hook scripts in scripts/
    - ${PLUGIN_ROOT} references
    - auto-update metadata and git source URLs
    - Gemini extension metadata (gemini-extension.json)
  rewrite_needed: true
  notes: |
    Claudine should extract `skills/<skill-name>/SKILL.md` files, which follow the Agent Skills standard and are already compatible with Claude Desktop and other agents that support Agent Skills. The namespaced skill name `plugin-name:skill-name` must be rewritten to the target provider's convention. Hooks and their scripts are Goose-specific (event payloads, `${PLUGIN_ROOT}`, `sh -c` execution) and should not be linked across providers without host-aware rewrite or explicit user approval. The plugin manifest, update metadata, and Gemini extension config are provider-specific and non-portable.

cli_params:
  - flag: "goose plugin install <git-url>"
    description: "Clone a git repository as a plugin into ~/.agents/plugins/."
    example: "goose plugin install https://github.com/example/my-goose-plugin.git"
  - flag: "goose plugin install --auto-update <git-url>"
    description: "Install a plugin and enable automatic update checks before skill loading."
    example: "goose plugin install --auto-update https://github.com/example/my-goose-plugin.git"
  - flag: "goose plugin update <plugin-name>"
    description: "Update an installed git-backed plugin from its original source."
    example: "goose plugin update my-plugin"
  - flag: "/skills"
    description: "List available skills, including plugin-provided skills."
    example: "/skills"
  - flag: "/skills <plugin-name>:<skill-name>"
    description: "Explicitly load a namespaced plugin skill."
    example: "/skills my-plugin:review"

env_vars: []

gaps:
  - No documented CLI command to remove/uninstall a plugin.
  - No explicit CLI command to enable or disable a plugin; disable is only via settings.json.
  - No documented plugin listing command or inventory inspection beyond `/skills`.
  - No documented trust model, code signing, or sandboxing specific to plugins.
  - No official plugin marketplace or registry.
  - Windows plugin installation path is not explicitly documented.
  - Conflict resolution and precedence rules between user/project plugins and standalone skills are not specified.
  - It is unclear whether Goose honors Open Plugins components other than skills and hooks (commands, agents, rules, MCP servers, LSP servers, output styles).

changes: []

requires_claudine_update: true
reason: |
  Claudine should add Goose plugin discovery paths (`~/.agents/plugins/` and `.agents/plugins/`) and the `disabledPlugins` settings file to its Goose provider model. The linker should extract portable Agent Skills from plugin `skills/` directories, rewrite namespaced skill references (`plugin-name:skill-name`), and treat hooks, scripts, the plugin manifest, and `${PLUGIN_ROOT}` references as non-portable Goose-specific assets.
---

# Goose CLI Plugin Research

## Overview

Goose CLI has a documented plugin system that packages reusable **skills** and **hooks** into a single installable directory. According to the [Goose Plugins guide](https://goose-docs.ai/docs/guides/context-engineering/plugins), a plugin is a directory with a manifest and optional `skills/` and `hooks/` subdirectories. The system follows the [Open Plugins specification](https://open-plugins.com/plugin-builders/specification), which defines a broader component model, but Goose only confirms support for skills and hooks.

By comparison, [Claude Code plugins](https://code.claude.com/docs/en/plugins) are first-class, versioned containers that can include skills, slash commands, subagents, MCP servers, LSP servers, hooks, themes, output styles, monitors, and channels, with a dedicated marketplace and CLI lifecycle commands. Goose plugins are narrower: they have no marketplace, no install-time trust prompt, no plugin-level MCP or LSP packaging, and only two documented lifecycle CLI commands (`install` and `update`).

## Installation and Locations

Goose discovers plugins from user-scope and project-scope directories. The canonical user location is `~/.agents/plugins/<plugin-name>/`, and the project location is `.agents/plugins/<plugin-name>/`.

| OS | Scope | Path | Notes |
|---|---|---|---|
| macOS / Linux | user | `~/.agents/plugins/<plugin-name>/` | Installed by `goose plugin install` or manual copy. |
| macOS / Linux | repo | `.agents/plugins/<plugin-name>/` | Available only when goose is launched from that project. |
| Windows | user | `%USERPROFILE%\.agents\plugins\<plugin-name>\` | Inferred from the cross-platform `~/.agents/plugins/` template; not explicitly documented. |
| Windows | repo | `.agents\plugins\<plugin-name>\` | Inferred project-scoped path. |

Plugin enable/disable state is controlled through `disabledPlugins` in Goose settings files:

| OS | Scope | Path |
|---|---|---|
| macOS / Linux | user | `~/.config/goose/settings.json` |
| Windows | user | `%APPDATA%\Block\goose\config\settings.json` |
| All | repo | `.config/goose/settings.json` |
| All | repo local | `.config/goose/settings.local.json` |

On this host there is no `~/.config/goose/` directory and no `~/.agents/plugins/` directory, so no local Goose plugin resources exist.

## Manifest and Package Format

Goose accepts several manifest locations:

- `plugin.json` at the plugin root
- `.plugin/plugin.json` (vendor-neutral Open Plugins path)
- `.goose-plugin/plugin.json`
- `gemini-extension.json` for Gemini-style extensions

The only required field is `name`. A minimal manifest looks like this ([Goose Plugins guide](https://goose-docs.ai/docs/guides/context-engineering/plugins)):

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "Reusable skills and hooks for my team"
}
```

The [Open Plugins specification](https://open-plugins.com/plugin-builders/specification) also recognizes optional metadata (`author`, `homepage`, `repository`, `license`, `logo`, `keywords`) and optional component path fields (`commands`, `agents`, `skills`, `rules`, `hooks`, `mcpServers`, `lspServers`, `outputStyles`). However, Goose documentation only confirms consumption of skills and hooks.

A typical Goose/Open Plugins layout is:

```text
my-plugin/
├── plugin.json
├── skills/
│   └── review/
│       └── SKILL.md
├── hooks/
│   └── hooks.json
└── scripts/
    └── notify.sh
```

The manifest is optional; when it is absent, Goose derives the plugin name from the directory. Hook-only plugins can be discovered from `hooks/hooks.json` even without a manifest. Relative paths must start with `./` and cannot escape the plugin directory.

## Packaged Resources

Goose plugins can contain the resources shown below. The assessment is based on the [Goose Plugins guide](https://goose-docs.ai/docs/guides/context-engineering/plugins) and the [Open Plugins specification](https://open-plugins.com/plugin-builders/specification).

| Resource | Support | Location | Notes |
|---|---|---|---|
| Agent Skills | Full | `skills/<name>/SKILL.md` | Namespaced as `plugin-name:skill-name` for Open Plugins. |
| Hooks | Full | `hooks/hooks.json` | Execute local commands on lifecycle events. |
| Scripts | Partial | `scripts/` | Referenced by hooks; not standalone commands. |
| Assets | Partial | `assets/`, supporting files in skill directories | Static files referenced by skills or hooks. |
| Config | Partial | `plugin.json`, `disabledPlugins` in settings | Manifest and disable-list only. |
| Slash commands | None | — | Slash commands are recipe-based and configured in `config.yaml`, not inside plugins. |
| Subagents / custom agents | None | — | Agents live in `~/.agents/agents/` or `.agents/agents/`, outside plugins. |
| MCP servers | None | — | Extensions/MCP servers are configured separately in `config.yaml`. |
| Prompts | None | — | No dedicated prompt container. |

The broader Open Plugins component types (`commands`, `agents`, `rules`, `mcpServers`, `lspServers`, `outputStyles`) are defined by the spec but not documented as supported by Goose.

## Lifecycle and Trust

**Install.** Plugins are installed from a git repository:

```bash
goose plugin install https://github.com/example/my-goose-plugin.git
```

The command clones the repo into `~/.agents/plugins/<plugin-name>/` and reports imported components ([Goose Plugins guide](https://goose-docs.ai/docs/guides/context-engineering/plugins)).

**Update.** Git-backed plugins are updated with:

```bash
goose plugin update my-plugin
```

The update command fetches the latest source and replaces the installed copy, preserving the `--auto-update` flag. Auto-update is enabled at install time with `goose plugin install --auto-update <url>`; Goose checks for updates before loading plugin skills, but the checks are rate-limited and failures are logged without blocking the session.

**Remove.** There is no documented `goose plugin uninstall` or `remove` command. Removal is a manual filesystem operation: delete the plugin directory and remove its name from `disabledPlugins` if present.

**Enable / disable.** Disable a plugin by adding its name to `disabledPlugins` in the appropriate `settings.json` file. There is no explicit `enable` command; removing the name from the list re-enables the plugin.

**Trust.** Goose does not document code signing, certificate validation, sandboxing, or an install-time trust prompt for plugins. The [Plugins guide](https://goose-docs.ai/docs/guides/context-engineering/plugins) warns: "Install trusted plugins only" because hooks execute local commands.

**Versioning.** The `version` field is optional and should follow Semantic Versioning. It is not enforced as a pin; git-backed plugins update from source regardless of the declared version.

## Discovery and Precedence

Goose discovers plugins by scanning `~/.agents/plugins/<name>/` and `<project>/.agents/plugins/<name>/` at session startup. Any plugin listed in `disabledPlugins` is skipped. Hook-only plugins are discovered via `hooks/hooks.json`.

Plugin skills are added to session instructions and are loaded automatically when the task matches a skill or when the user runs `/skills <name>`. Open Plugin skills are namespaced (`my-plugin:review`), while Gemini extension skills keep their original name.

Precedence and conflict rules are not documented. Project plugins are scoped to the launch directory, and user plugins are global, but Goose does not specify how collisions between plugins or between plugin and standalone skills are resolved.

## Security and Runtime Behavior

Plugin skills are instructions added to the model context. Hooks are the main runtime risk: they execute local shell commands via `sh -c`, receive event payloads on stdin, and run with the user's OS permissions and environment ([Goose Hooks guide](https://goose-docs.ai/docs/guides/context-engineering/hooks)).

- **Trust model.** Trust is established by the user choosing to install or copy the plugin. There is no signature verification or plugin-specific sandbox.
- **Permissions.** Hooks run automatically when their event/matcher matches; there is no per-hook approval prompt. The user must pre-approve by installing the plugin.
- **Sandboxing.** Hooks are not sandboxed. Goose Desktop has an optional macOS App Sandbox via `GOOSE_SANDBOX`, but it is not plugin-specific isolation.
- **Credential access.** Hook commands inherit environment variables. Goose sets `PLUGIN_ROOT` so scripts can reference files inside the plugin. Secrets are stored in the system keyring or `~/.config/goose/secrets.yaml` when the keyring is unavailable.
- **Update risk.** `goose plugin install --auto-update` can pull new code before skills load, silently changing behavior. The `version` field is informational and does not prevent updates.

Goose checks external extensions for known malware before activation ([Using Extensions guide](https://goose-docs.ai/docs/getting-started/using-extensions)), but this behavior is not described for plugins.

## Distribution

Goose has no documented plugin marketplace or registry. Plugins are distributed as git repositories or local directories. The [Skills Marketplace](https://goose-docs.ai/skills) and [Extensions directory](https://goose-docs.ai/extensions) are separate surfaces and do not appear to act as plugin marketplaces.

| Source | Supported | Example |
|---|---|---|
| Git repository | Yes | `goose plugin install https://github.com/example/my-plugin.git` |
| Local folder | Yes | Copy into `~/.agents/plugins/<name>/` |
| Marketplace | No | No plugin marketplace documented. |
| Archive / npm | No | No archive or npm package format documented. |

Private distribution works through any git repository; auto-update relies on the host's git credentials. There is no documented signing, ownership verification, or moderation workflow.

## Portability

Claudine should not link Goose plugins as intact units to other providers. The portable part is the Agent Skills content in `skills/<name>/SKILL.md`, which follows the [Agent Skills](https://agentskills.io) format and is compatible with Claude Desktop and other agents that support Agent Skills.

Non-portable assets include:

- `plugin.json` and `.goose-plugin/plugin.json`
- `hooks/hooks.json`
- `scripts/` and hook helper scripts
- `${PLUGIN_ROOT}` path references
- Auto-update metadata and git source URLs
- `gemini-extension.json`

The namespaced skill reference `my-plugin:review` should be rewritten to the target provider's namespacing convention. Hooks should be omitted or flagged for host-aware rewrite because they depend on Goose-specific event payloads, `${PLUGIN_ROOT}`, and `sh -c` execution.

## Claudine Linking Notes

- Discover plugins at `~/.agents/plugins/<name>/` and `.agents/plugins/<name>/`.
- Read `disabledPlugins` from `~/.config/goose/settings.json`, `.config/goose/settings.json`, and `.config/goose/settings.local.json` to determine which plugins are active.
- For each enabled plugin, scan `skills/<skill-name>/SKILL.md` and extract portable Agent Skills.
- Rewrite namespaced skill names (`plugin-name:skill-name`) into the target provider's format.
- Do not link `hooks/hooks.json`, `scripts/`, or `${PLUGIN_ROOT}` references across providers without explicit user approval and host-aware rewrite.
- Treat the plugin manifest, Gemini extension metadata, and auto-update source as Goose-specific and non-portable.
- Note that Goose has no marketplace or registry; plugin provenance is the git URL or local path.

## Sources

- [Goose Plugins guide](https://goose-docs.ai/docs/guides/context-engineering/plugins)
- [Goose Hooks guide](https://goose-docs.ai/docs/guides/context-engineering/hooks)
- [Goose Agent Skills guide](https://goose-docs.ai/docs/guides/context-engineering/using-skills)
- [Goose CLI Commands reference](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [Goose Configuration Files guide](https://goose-docs.ai/docs/guides/config-files)
- [Goose Environment Variables guide](https://goose-docs.ai/docs/guides/environment-variables)
- [Goose Using Extensions guide](https://goose-docs.ai/docs/getting-started/using-extensions)
- [Goose Subagents guide](https://goose-docs.ai/docs/guides/context-engineering/subagents)
- [Goose Recipes guide](https://goose-docs.ai/docs/guides/recipes/session-recipes)
- [Goose Custom Slash Commands guide](https://goose-docs.ai/docs/guides/context-engineering/slash-commands)
- [Open Plugins specification](https://open-plugins.com/plugin-builders/specification)
- [Open Plugins Hooks specification](https://open-plugins.com/agent-builders/components/hooks)
- [Claude Code Plugins reference](https://code.claude.com/docs/en/plugins)
