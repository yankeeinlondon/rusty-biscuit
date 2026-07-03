---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default

homepage: https://kilo.ai/
docs: https://kilo.ai/docs
plugin_docs: https://kilo.ai/docs/automate/extending/plugins

support: partial

locations:
  - os: macos
    scope: user
    path: ~/.config/kilo/plugin/
    notes: Global runtime plugin directory; `.ts` and `.js` files are auto-registered at startup.
  - os: linux
    scope: user
    path: ~/.config/kilo/plugin/
    notes: Global runtime plugin directory.
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\plugin\\"
    notes: Template config-root path; Kilo docs do not give an exact Windows runtime-plugin path.
  - os: macos
    scope: repo
    path: .kilo/plugin/
    notes: Project runtime plugin directory; legacy `.kilocode/plugin/` is also scanned.
  - os: linux
    scope: repo
    path: .kilo/plugin/
    notes: Project runtime plugin directory.
  - os: windows
    scope: repo
    path: ".kilo\\plugin\\"
    notes: Project runtime plugin directory.
  - os: macos
    scope: user
    path: ~/.config/kilo/kilo.jsonc
    notes: Global config can contain `plugin` entries for npm or file plugins.
  - os: linux
    scope: user
    path: ~/.config/kilo/kilo.jsonc
    notes: Global config can contain `plugin` entries.
  - os: windows
    scope: user
    path: "%APPDATA%\\kilo\\kilo.jsonc"
    notes: Template path; exact Windows config root was not verified in official docs.
  - os: macos
    scope: repo
    path: .kilo/opencode.jsonc
    notes: Local `kilo plugin` installs write server plugin entries here when no existing project config file is chosen.
  - os: linux
    scope: repo
    path: .kilo/opencode.jsonc
    notes: Local `kilo plugin` installs write server plugin entries here.
  - os: windows
    scope: repo
    path: ".kilo\\opencode.jsonc"
    notes: Local `kilo plugin` installs write server plugin entries here.
  - os: macos
    scope: user
    path: ~/.cache/opencode/packages/
    notes: npm runtime plugin package cache used by the CLI; affected by `XDG_CACHE_HOME`.
  - os: linux
    scope: user
    path: ~/.cache/opencode/packages/
    notes: npm runtime plugin package cache used by the CLI; affected by `XDG_CACHE_HOME`.
  - os: windows
    scope: user
    path: "%LOCALAPPDATA%\\opencode\\packages\\"
    notes: Template cache path; exact Windows path was not verified.
  - os: macos
    scope: marketplace
    path: https://github.com/Kilo-Org/kilo-marketplace
    notes: Official catalog repository for Marketplace skills, MCP servers, and agents.
  - os: linux
    scope: marketplace
    path: https://github.com/Kilo-Org/kilo-marketplace
    notes: Official catalog repository for Marketplace skills, MCP servers, and agents.
  - os: windows
    scope: marketplace
    path: https://github.com/Kilo-Org/kilo-marketplace
    notes: Official catalog repository for Marketplace skills, MCP servers, and agents.

manifest:
  file_names:
    - package.json
    - SKILL.md
    - AGENT_DEFINITION.md
    - MCP.yaml
    - marketplace.yaml
  format: other
  required_fields:
    - package.json name for npm plugins
    - default export with id and server or tui for local module plugins
    - SKILL.md frontmatter name and description for skills
    - AGENT_DEFINITION.md frontmatter description for agents
    - MCP.yaml id, name, description, category, and content for MCP marketplace entries
  optional_fields:
    - package.json exports["./server"]
    - package.json exports["./tui"]
    - package.json main
    - package.json oc-themes
    - package.json engines.opencode
    - plugin options tuple in config
    - skill metadata.source
    - requirements
    - prerequisites
    - tags
    - author
    - url
  package_layout: |
    Runtime plugins are TypeScript or JavaScript modules. Local plugins live as `.ts` or `.js` files under
    `plugin/` or `plugins/` inside any Kilo config directory. npm plugins are standard npm packages named in
    the `plugin` array of `kilo.jsonc`, `kilo.json`, `opencode.jsonc`, or `opencode.json`.

    npm plugin packages expose server plugins through `exports["./server"]` or `main`; TUI plugins through
    `exports["./tui"]`; and theme-only TUI packages through the `oc-themes` package field.

    Marketplace resources are separate catalog entries, not one plugin directory. The official repository uses
    `skills/<id>/SKILL.md`, `agents/<id>/AGENT_DEFINITION.md`, `mcps/<id>/MCP.yaml`, and generated
    `*/marketplace.yaml` indexes. Skill marketplace entries can point at release tarballs.
  notes: |
    Kilo's runtime plugin manifest is standard npm `package.json` plus module exports, or no manifest for a
    single local file plugin. Kilo's Marketplace has resource manifests, but installing a Marketplace item writes
    the contained resource to normal Kilo locations instead of preserving an installed plugin container.

lifecycle:
  install: |
    Runtime npm plugins can be installed with `kilo plugin <module>` or by adding an entry to a config file's
    `plugin` array. Local runtime plugins install by placing a `.ts` or `.js` file in a scanned plugin directory.
    Marketplace items are installed from the Kilo sidebar Marketplace UI, which writes skills, agents, or MCP
    config entries into project or global locations.
  update: |
    Pinned npm plugin specs such as `name@1.2.3` stay pinned. Bare npm package names resolve to `latest` and can
    refresh when Kilo's cache is stale. Marketplace docs do not document a CLI update command for marketplace
    items; the public marketplace repository generates release assets for skills.
  remove: |
    Runtime plugins are removed manually by deleting local plugin files or removing `plugin` array entries. The
    Marketplace UI removes marketplace-managed entries per scope. `kilo --help` showed no runtime-plugin list,
    remove, enable, disable, audit, or trust subcommand on this host.
  enable_disable: |
    Runtime plugin enablement is presence-based: listed in config, present in a plugin directory, or skipped by
    running with `--pure`/`KILO_PURE=1`. Marketplace resources use normal resource enablement, such as MCP
    `enabled` fields or agent `disable` fields, rather than plugin-container enablement.
  trust: |
    Runtime plugins are trusted when the user adds the package or file. Marketplace installs show the destination
    before changing files and warn users to review author, source, prerequisites, parameters, and tools. No
    signature verification or sandboxed plugin trust boundary was found.
  versioning: |
    Runtime npm plugins use npm package versions. Local file plugins and Marketplace-installed resources are
    unversioned after extraction unless the resource itself records provenance or metadata.
  notes: |
    Local inspection found `kilo` 7.3.45 installed, `~/.config/kilo/kilo.jsonc` containing only the schema URL,
    and no `~/.kilo` directory. No local Kilo plugin, skill, agent, command, or MCP marketplace resources were
    present under the checked `~/.kilo` path.

packaged_resources:
  skills: partial
  scripts: partial
  slash_commands: partial
  subagents: partial
  mcp_servers: partial
  hooks: full
  prompts: partial
  config: full
  assets: partial
  other:
    - TUI plugins
    - custom tools
    - auth providers
    - model providers
    - workspace adapters
    - TUI themes
    - shell environment hooks

discovery:
  mechanism: |
    Runtime plugins are discovered from the merged `plugin` config array and from `{plugin,plugins}/*.{ts,js}`
    inside scanned config directories. npm plugins are installed and imported at startup. Marketplace items are
    discovered after installation through the normal Kilo resource scanners for skills, agents, MCP config, and
    commands.
  precedence: |
    Runtime plugin load order is internal built-ins, global config plugin array, global plugin directory, project
    config plugin array, then project plugin directories. Duplicates are deduplicated by package identity or file
    URL and hooks run sequentially. For skills, project-level skills take precedence over global skills when names
    collide. MCP project config takes precedence over global config.
  namespacing: |
    Runtime hooks and tools are not exposed with a plugin-name namespace. Custom tool names share the tool
    namespace; if a custom tool uses a built-in tool name, the custom tool wins. Marketplace skills, agents, and
    commands use their normal Kilo names after extraction.
  conflicts: |
    Runtime plugin duplicate config entries are deduplicated. Custom tool name conflicts intentionally allow the
    custom tool to override a built-in. Skill name conflicts prefer project over global. Marketplace docs do not
    describe a plugin-container-level conflict resolver because installed items are regular resources.
  notes: |
    This differs from Claude Code, where plugin resources are intentionally namespaced, for example
    `/plugin-name:hello`, to avoid conflicts between plugins.

security:
  trust_model: |
    Runtime plugins execute as trusted local code with the user's privileges. Marketplace resources are trusted
    by installing them into Kilo's normal config/resource paths. Marketplace MCP docs warn users to review source
    and required parameters before installation.
  permissions: |
    Runtime plugins can register tools, intercept tool calls, auto-answer permission prompts, inject shell
    environment variables, and mutate chat headers. MCP tools follow Kilo's `allow`, `ask`, and `deny`
    permission rules. Agent files can also declare per-tool permissions.
  sandbox_interaction: |
    No separate plugin sandbox was found. npm install scripts are disabled for runtime npm plugins, but imported
    plugin code runs inside the Kilo process. MCP server processes run according to their configured command or
    remote URL and are governed at tool-call time by Kilo permissions.
  credential_access: |
    Runtime plugins receive a local SDK client, project/worktree paths, server URL, and Bun shell access; hooks
    can inject environment variables or modify request headers. Config plugin options may include `{env:VAR}`
    substitutions. MCP configs can include environment values and OAuth/API credentials.
  update_risk: |
    Bare npm plugin names can resolve to newer `latest` versions when the cache refreshes. Marketplace resource
    updates can change instructions or MCP commands before extraction. Pinned npm specs reduce runtime plugin
    update risk.
  notes: |
    Kilo's documented security controls are operational controls around resource permissions and disabled npm
    lifecycle scripts, not a hard security boundary for plugin code.

distribution:
  marketplace: true
  registry_url: https://github.com/Kilo-Org/kilo-marketplace
  source_types:
    - npm
    - local file
    - local folder
    - GitHub repository
    - GitHub release archive
    - Marketplace UI
    - remote skill URL
  publishing: |
    Runtime plugins are distributed as npm packages or local files/folders. Marketplace resources are submitted
    through pull requests to the Kilo Marketplace repository. Contributed skills are expected to reference an
    external source repository through `metadata.source`; marketplace tooling imports and packages them.
  private_distribution: |
    Runtime plugins can be private npm packages or private local files if the user's npm/auth and filesystem can
    resolve them. Kilo docs do not describe a private Marketplace registry equivalent to Claude Code team
    marketplaces.
  notes: |
    The Kilo Marketplace repository is a catalog of skills, MCP servers, and agents. The Kilo docs also note that
    Marketplace items are configuration and instruction files, not VS Code extensions.

portability:
  link_plugin_as_unit: false
  extract_resources: true
  portable_resources:
    - skills
    - agents
    - slash commands
    - prompts
    - MCP server definitions after review
  non_portable_assets:
    - runtime JavaScript or TypeScript plugin modules
    - npm package references
    - TUI plugin modules
    - package.json export metadata
    - MCP credentials and environment variables
    - shell environment hooks
    - provider/auth hooks
    - marketplace IDs and generated release URLs
  rewrite_needed: true
  notes: |
    Claudine should treat Kilo Marketplace entries as extracted resource candidates, not as a portable plugin unit.
    Agent Skills can be linked as skill directories after preserving `SKILL.md` and bundled references/assets.
    Agents and commands require provider-specific frontmatter review. MCP entries should be imported into
    Claudine's MCP catalog only after stripping credentials and preserving transport/command metadata. Runtime
    Kilo plugins should be marked non-portable unless Claudine grows an equivalent JavaScript hook runtime.

cli_params:
  - flag: kilo plugin <module>
    description: Installs an npm runtime plugin and patches local plugin config.
    example: kilo plugin my-plugin
  - flag: kilo plugin <module> --global
    description: Installs an npm runtime plugin and patches global plugin config.
    example: kilo plugin my-plugin --global
  - flag: kilo plugin <module> --force
    description: Replaces an existing plugin entry for the same package identity.
    example: kilo plugin my-plugin@1.2.3 --force
  - flag: kilo --pure
    description: Runs without external runtime plugins.
    example: kilo --pure
  - flag: kilo mcp add
    description: Adds an MCP server configuration; relevant because Marketplace MCP items become normal MCP config.
    example: kilo mcp add
  - flag: kilo mcp list
    description: Lists MCP servers and status after config or Marketplace installation.
    example: kilo mcp list
  - flag: kilo mcp auth [name]
    description: Authenticates an OAuth-enabled MCP server.
    example: kilo mcp auth github
  - flag: kilo mcp logout [name]
    description: Removes OAuth credentials for an MCP server.
    example: kilo mcp logout github
  - flag: kilo agent create
    description: Creates an agent file interactively or with flags; Marketplace agents install into the same resource surface.
    example: kilo agent create --path .kilo --description "Reviews code" --mode subagent
  - flag: kilo agent list
    description: Lists available agents after file or Marketplace installation.
    example: kilo agent list

env_vars:
  - name: KILO_PURE
    effect: Skips external runtime plugins; built-in plugins still load.
  - name: KILO_CONFIG
    effect: Loads an additional config file after global config.
  - name: KILO_CONFIG_DIR
    effect: Adds an additional config directory to the scanned config/plugin/resource locations.
  - name: KILO_CONFIG_CONTENT
    effect: Supplies inline JSON config with high precedence.
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: Skips project-level config and resource discovery.
  - name: KILO_DISABLE_DEFAULT_PLUGINS
    effect: Disables bundled/default Kilo plugins such as indexing or Atomic Chat.
  - name: KILO_DISABLE_EXTERNAL_SKILLS
    effect: Disables external skill loading.
  - name: KILO_PERMISSION
    effect: Applies a runtime permission overlay from JSON.
  - name: XDG_CONFIG_HOME
    effect: Can influence the global Kilo config root on XDG platforms.
  - name: XDG_CACHE_HOME
    effect: Changes the CLI cache root used for npm plugin packages.

gaps:
  - Marketplace install/update/remove behavior appears UI-driven; no Marketplace CLI list/install/remove command was found in `kilo --help`.
  - Official docs do not give exact Windows config/cache/plugin paths for all runtime plugin surfaces.
  - Runtime plugin removal, per-plugin disablement, audit, signing, and trust prompts are not documented as dedicated lifecycle operations.
  - Marketplace resource conflict behavior is documented per extracted resource type, not per original marketplace item.
  - Private marketplace or registry behavior for Kilo Marketplace resources was not found.
  - Local `~/.kilo` did not exist on this host; only `~/.config/kilo/kilo.jsonc` with the schema URL was observed.

changes: []

requires_claudine_update: true
reason: |
  Claudine should model Kilo runtime plugins separately from Marketplace resource extraction. Kilo runtime plugins
  are non-portable JavaScript/TypeScript code, while Marketplace skills, agents, commands, and MCP definitions can
  be linked only after extraction and provider-specific metadata review.
---

# Kilo Code Plugin Research

## Overview

Kilo has two extension mechanisms that use plugin-like language but behave differently.

The first mechanism is the documented Kilo runtime plugin system. These plugins are TypeScript or JavaScript modules loaded at startup by the Kilo CLI and the VS Code extension. They hook into events, add custom tools, register auth and model providers, mutate chat parameters and headers, customize compaction, and inject shell environment variables. Runtime plugins are configured in the `plugin` array of Kilo config files, placed in scanned `plugin/` or `plugins/` directories, or installed from npm with `kilo plugin <module>`.

The second mechanism is Kilo Marketplace. Marketplace items are reusable extensions in the product UI, but Kilo's own docs describe them as configuration and instruction files rather than VS Code extensions. Installing an item writes a skill, agent, or MCP configuration into project or user locations, and Kilo then discovers those files through its normal resource scanners. That means Kilo Marketplace is not a self-contained installed plugin container in the Claude Code sense; it is closer to a catalog and extractor for individual resources.

This is the core difference from Claude Code. Claude Code documents plugins as self-contained directories with component schemas and a `.claude-plugin/plugin.json` manifest, and it explicitly lists skills, agents, hooks, MCP servers, LSP servers, and monitors as plugin components. Claude also namespaces plugin resources such as `/plugin-name:hello`, provides marketplace install scopes, and exposes installed-plugin enable, disable, uninstall, list, detail, update, and reload flows. Kilo has a first-class runtime module API, but Marketplace-installed skills, agents, and MCP servers are extracted into normal Kilo resource locations rather than remaining attached to a plugin container.

## Installation and Locations

Runtime plugins can be installed three ways:

1. Add entries to the `plugin` array in a Kilo config file.
2. Drop `.ts` or `.js` files into `plugin/` or `plugins/` under any scanned config directory.
3. Run `kilo plugin <module>`, which resolves an npm package, reads its `package.json`, and writes plugin entries to local or global config.

Config-file plugin entries accept npm package names, pinned npm specs such as `package@1.2.3`, two-element tuples such as `["package", { "apiKey": "{env:MY_API_KEY}" }]`, relative local plugin paths, and absolute `file:` URLs. Relative local plugin paths are resolved relative to the config file that declared them.

Documented runtime plugin directories include:

| Scope | Path |
|---|---|
| Global | `~/.config/kilo/plugin/` |
| Project | `.kilo/plugin/` |
| Legacy project | `.kilocode/plugin/` |

Source inspection confirms Kilo scans both singular and plural forms, so `plugin/` and `plugins/` work. Source inspection also confirms Kilo scans `kilo.jsonc`, `kilo.json`, `opencode.jsonc`, and `opencode.json` in config directories, with `kilo` names preferred for new Kilo configuration.

`kilo plugin <module>` writes server plugin entries to `.kilo/opencode.jsonc` or `~/.config/kilo/opencode.jsonc`, and TUI plugin entries to `.kilo/tui.jsonc` or `~/.config/kilo/tui.jsonc`, preserving JSONC comments. This is a runtime-plugin command, not a Marketplace command.

Marketplace installation writes extracted resources to normal locations:

| Marketplace type | Project destination | Global destination |
|---|---|---|
| Agent | `.kilo/agents/<name>.md` | `~/.config/kilo/agents/<name>.md` |
| Skill | `.kilo/skills/<name>/` | `~/.kilo/skills/<name>/` |
| MCP server | `.kilo/kilo.json` | `~/.config/kilo/kilo.json` |

MCP configuration also supports global `~/.config/kilo/kilo.jsonc` and project `kilo.jsonc`, `kilo.json`, `.kilo/kilo.jsonc`, or `.kilo/kilo.json`, with project configuration taking precedence over global configuration.

Local inspection on 2026-07-03 found:

| Path | Observation |
|---|---|
| `~/.kilo` | Did not exist. |
| `~/.config/kilo` | Existed. |
| `~/.config/kilo/kilo.jsonc` | Contained only `{ "$schema": "https://app.kilo.ai/config.json" }`. |
| Local `kilo` binary | Installed at version 7.3.45 through npm. |

## Manifest and Package Format

Runtime plugins use JavaScript module shape plus npm package metadata, not a dedicated `plugin.json`.

A local runtime plugin can be a single `.ts` or `.js` file:

```ts
import type { Plugin } from "@kilocode/plugin"

const server: Plugin = async () => ({
  event: async ({ event }) => {
    // hook implementations
  },
})

export default { id: "my-plugin", server }
```

For local-file plugins, `id` is required. For npm plugins, Kilo can infer identity from `package.json#name`.

Published npm runtime plugins should use `package.json` fields:

| Field | Meaning |
|---|---|
| `exports["./server"]` | Server plugin entry point. |
| `exports["./tui"]` | TUI plugin entry point. |
| `main` | Server-only fallback when `exports` is absent. |
| `oc-themes` | TUI theme package paths. |
| `engines.opencode` | Optional CLI compatibility range; incompatible plugins are skipped with a warning. |

Kilo disables npm lifecycle scripts such as `install` and `postinstall` for runtime plugin package installation. Local plugins can import npm dependencies by placing a `package.json` in the same config directory; Kilo runs `bun install` at startup so imports resolve.

Marketplace manifests are resource manifests:

| Resource | Source format | Generated index |
|---|---|---|
| Skill | `skills/<id>/SKILL.md` plus optional bundled files | `skills/marketplace.yaml` |
| Agent | `agents/<id>/AGENT_DEFINITION.md` | `agents/marketplace.yaml` |
| MCP server | `mcps/<id>/MCP.yaml` | `mcps/marketplace.yaml` |

The marketplace repository also has legacy `modes/marketplace.yaml`, but the repository README says current clients should use agents and that modes remain for legacy 5.x clients.

## Packaged Resources

Runtime plugins can contain executable hook logic, TUI extensions, tools, auth providers, model providers, workspace adapters, theme metadata, and arbitrary package assets. They are not a container for Agent Skills, commands, subagents, or MCP server definitions in the same way Claude Code plugins are.

Marketplace entries cover:

| Resource | Kilo coverage | Notes |
|---|---|---|
| Agent Skills | Partial | Marketplace skills install to `.kilo/skills/<name>/` or `~/.kilo/skills/<name>/`; skills can include `scripts/`, `references/`, and `assets/`. |
| Scripts | Partial | Scripts can be bundled inside skills or npm runtime plugin packages, but there is no generic script resource type. |
| Slash commands / workflows | Partial | Kilo workflows are Markdown slash commands in `.kilo/commands/` or `~/.config/kilo/commands/`; current Marketplace docs list agents, skills, and MCP servers, not commands as marketplace item types. |
| Subagents | Partial | Marketplace agents and custom agent Markdown files can define primary or subagent behavior. |
| MCP servers | Partial | Marketplace MCP entries install MCP config; normal Kilo MCP permission and auth behavior applies. |
| Hooks | Full | Runtime plugins are hook modules. |
| Prompts | Partial | Agent files and commands carry prompts; skills carry instructions. |
| Config | Full | Runtime plugins can read config at startup and `kilo plugin` patches config. Marketplace items write config/resource files. |
| Assets | Partial | Skills and npm packages can include assets; Marketplace handling is resource-specific. |

## Lifecycle and Trust

Runtime plugin install is config-driven. `kilo plugin <module>` is a convenience command that resolves an npm package, reads package entrypoints, and patches config. It does not expose a complete lifecycle manager. On this host, `kilo plugin --help` showed only `--global` and `--force`; there were no runtime-plugin `list`, `remove`, `enable`, `disable`, `trust`, or `audit` subcommands.

Runtime plugin update behavior follows the npm spec:

| Spec | Update behavior |
|---|---|
| `package-name` | Resolves to `latest`; can refresh when the cached copy is stale. |
| `package-name@1.2.3` | Installs that exact version. |
| Local file path | No versioning. |

Marketplace install and removal are UI-driven from the Kilo sidebar. The install dialog shows the destination before changing files. Removing an item deletes its marketplace-managed entry from the selected scope, and project/global copies are removed independently. Kilo reloads affected configuration after install or removal; running sessions may be interrupted so they do not continue with stale agents, skills, or tools.

Trust is mostly source trust. Runtime plugins execute code in the Kilo process. Marketplace MCP servers can run local commands or connect to remote services; installing an MCP server makes its tools available but does not automatically approve each tool call. MCP tools follow Kilo's `allow`, `ask`, and `deny` permission rules.

Claude Code is more explicit at the container lifecycle layer. Its docs describe an official marketplace, install scopes, installed-plugin list, enable, disable, uninstall, plugin details, `/reload-plugins`, auto-update controls, and a warning that plugins and marketplaces are highly trusted components that can execute arbitrary code with user privileges. Kilo documents similar runtime risk for the capabilities indirectly, but the lifecycle surface is thinner.

## Discovery and Precedence

Runtime plugin load order is documented as:

1. Internal built-ins.
2. Global config plugin array, for example `~/.config/kilo/kilo.json`.
3. Global plugin directory, for example `~/.config/kilo/plugin/`.
4. Project config plugin array, for example `kilo.json` or `opencode.json`.
5. Project plugin directories, for example `.kilo/plugin/`.

Duplicates are deduplicated. Hooks from multiple plugins run sequentially in load order. Source inspection shows deduplication uses package identity for npm specs and exact file URL for local file specs, with later merged origins winning during reverse-order deduplication.

For extracted resources, normal Kilo precedence applies:

| Resource | Precedence |
|---|---|
| Skills | Project `.kilo/skills/` wins over global `~/.kilo/skills/` for same names; compatibility directories and configured paths load alongside them. |
| MCP | Project MCP config takes precedence over global MCP config. |
| Agents | Agents can be defined in config or Markdown files; built-in agents can be customized or disabled by name. |
| Commands | Command files load from global and project command directories; file name without `.md` is the slash command name. |

There is no plugin namespace for Kilo runtime hooks or tools. If a custom tool uses a built-in tool name, Kilo documents that the custom tool wins. By contrast, Claude Code recommends plugins when users are comfortable with namespaced skills such as `/my-plugin:hello`, specifically to prevent conflicts between plugins.

## Security and Runtime Behavior

Runtime plugins are executable code. A plugin receives project metadata, current directory, worktree root, a local Kilo SDK client, local server URL, and Bun's shell API. Hooks can inspect events, register tools, modify tool arguments and outputs, auto-allow or auto-deny permission prompts, modify LLM request headers, and inject shell environment variables into every shell command Kilo runs.

Kilo reduces npm-package installation risk by blocking npm lifecycle scripts for runtime plugins. That does not sandbox imported plugin code. Once loaded, a runtime plugin runs with the user's privileges inside the Kilo process.

Marketplace security depends on the installed resource:

| Resource | Runtime risk |
|---|---|
| Skill | Instructions may direct the agent to read files, use tools, or execute bundled scripts if the agent chooses to follow them and permissions allow. |
| Agent | Agent frontmatter can set permissions such as read, edit, bash, MCP, and task delegation. |
| MCP server | Local MCP servers run configured commands as child processes; remote MCP servers send requests to services. |
| Command/workflow | Markdown commands can leverage built-in tools, bash, web fetch, and MCP tools. |

Kilo's permission model uses `allow`, `ask`, and `deny`, with last matching rule winning for patterns. Marketplace docs warn users to keep credentials out of version control because project `.kilo/kilo.json` may be committed.

## Distribution

Runtime plugins are distributed through npm packages, local files, or local folders. Private npm distribution should work when the local npm/Bun environment can resolve the package, but Kilo's docs do not describe a private plugin registry flow beyond normal npm behavior.

Kilo Marketplace distribution is centered on `Kilo-Org/kilo-marketplace`. The repository is a curated collection of skills, MCP servers, and agents. It uses source folders plus generated `marketplace.yaml` indexes. Contributed skills must reference an external source repository in `metadata.source`; marketplace tooling imports the skill, records the upstream commit, and packages skill release archives.

Kilo does not appear to have Claude Code's marketplace-container model. Claude Code marketplaces are catalogs of plugins; adding a marketplace only registers the catalog, and installing an individual plugin is a second step. Claude also supports official, third-party, local development, team, and managed marketplace flows. Kilo Marketplace is useful for distributing reusable resources, but installation extracts files into Kilo's normal configuration system.

## Portability

Claudine should not link Kilo runtime plugins as portable plugin units. They are provider-specific JavaScript/TypeScript modules targeting `@kilocode/plugin`, Kilo's hook names, Kilo's local SDK, and optionally Kilo's TUI plugin API. They can execute code, mutate requests, and access local process capabilities. Without an equivalent runtime and permission model, linking them into another provider would be unsafe and semantically wrong.

Marketplace resources should be considered for extraction:

| Kilo source | Claudine action |
|---|---|
| `skills/<id>/SKILL.md` | Link as an Agent Skill if frontmatter validates and bundled files are safe to preserve. |
| Skill `scripts/` | Preserve only with explicit non-portable/executable marking. |
| `agents/<id>/AGENT_DEFINITION.md` | Convert to provider-specific agent/subagent metadata only after reviewing frontmatter fields such as permissions, mode, model, and prompt. |
| `mcps/<id>/MCP.yaml` | Import as MCP catalog candidates after stripping credentials and preserving command/transport metadata. |
| `commands/*.md` | Link as slash commands where the target provider supports Markdown commands; rewrite frontmatter if needed. |
| Runtime `plugin/*.ts` or npm packages | Mark non-portable. |

## Claudine Linking Notes

Implementation notes for Claudine:

1. Treat `plugin` config entries and `plugin/` files as Kilo runtime plugins, not as shareable resource bundles.
2. Mark runtime plugins non-portable by default because they are executable code and Kilo-specific.
3. Scan Kilo Marketplace resources by extracted type, not by marketplace item, if Claudine chooses to ingest them.
4. Preserve skill directories intact, including `references/`, `assets/`, and `scripts/`, but flag scripts as executable risk.
5. Convert agents and commands through provider-specific rewriters; do not assume Kilo frontmatter maps directly to Claude, Codex, OpenCode, or others.
6. Import MCP definitions into the normalized MCP catalog only after removing secrets and preserving transport, command, args, environment keys, and enabled state.
7. Do not generate a Claude-style plugin manifest for Kilo Marketplace entries unless Claudine also records that this is a synthetic package, not a Kilo-native plugin.

This research implies Claudine metadata should distinguish at least three Kilo surfaces: runtime plugins, Marketplace resource catalogs, and already-extracted normal resources.

## Sources

- [Kilo Plugins documentation](https://kilo.ai/docs/automate/extending/plugins)
- [Kilo Marketplace documentation](https://kilo.ai/docs/customize/marketplace)
- [Kilo Skills documentation](https://kilo.ai/docs/customize/skills)
- [Kilo Custom Subagents documentation](https://kilo.ai/docs/customize/custom-subagents)
- [Kilo Workflows documentation](https://kilo.ai/docs/customize/workflows)
- [Kilo MCP documentation](https://kilo.ai/docs/automate/mcp/using-in-kilo-code)
- [Kilo Agent Permissions documentation](https://kilo.ai/docs/customize/agent-permissions)
- [Kilo Marketplace repository](https://github.com/Kilo-Org/kilo-marketplace)
- [Kilo Code repository](https://github.com/Kilo-Org/kilocode)
- [Kilo plugin install source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/plugin/install.ts)
- [Kilo plugin loader source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/plugin/loader.ts)
- [Kilo plugin shared source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/plugin/shared.ts)
- [Kilo plugin command source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/cli/cmd/plug.ts)
- [Claude Code plugins reference](https://code.claude.com/docs/en/plugins-reference)
- [Claude Code create plugins documentation](https://code.claude.com/docs/en/plugins)
- [Claude Code discover plugins documentation](https://code.claude.com/docs/en/discover-plugins)
