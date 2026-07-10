---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://opencode.ai
docs: https://opencode.ai/docs
plugin_docs: https://opencode.ai/docs/plugins

support: partial

locations:
  - os: macos
    scope: user
    path: ~/.config/opencode/plugins/
    notes: Global plugin directory. TypeScript/JavaScript files here are auto-loaded at startup.
  - os: linux
    scope: user
    path: ~/.config/opencode/plugins/
    notes: Global plugin directory.
  - os: windows
    scope: user
    path: "%APPDATA%\\opencode\\plugins\\"
    notes: Windows equivalent; exact path not verified in docs, inferred from XDG-style conventions.
  - os: macos
    scope: repo
    path: .opencode/plugins/
    notes: Project-level plugin directory. Observed no plugins here on this host.
  - os: linux
    scope: repo
    path: .opencode/plugins/
    notes: Project-level plugin directory.
  - os: windows
    scope: repo
    path: ".opencode\\plugins\\"
    notes: Project-level plugin directory.
  - os: macos
    scope: user
    path: ~/.cache/opencode/node_modules/
    notes: Bun install cache for npm plugins referenced in opencode.json.
  - os: linux
    scope: user
    path: ~/.cache/opencode/node_modules/
    notes: Bun install cache for npm plugins.
  - os: windows
    scope: user
    path: "%LOCALAPPDATA%\\opencode\\node_modules\\"
    notes: Inferred Windows cache location; not explicitly documented.
  - os: macos
    scope: user
    path: ~/.config/opencode/opencode.json
    notes: Global config file; the `plugin` array lists npm packages to load.
  - os: linux
    scope: user
    path: ~/.config/opencode/opencode.json
    notes: Global config file.
  - os: windows
    scope: user
    path: "%APPDATA%\\opencode\\opencode.json"
    notes: Global config file.
  - os: macos
    scope: repo
    path: opencode.json
    notes: Project config file; can also list plugins. OpenCode searches upward to the git worktree.
  - os: linux
    scope: repo
    path: opencode.json
    notes: Project config file.
  - os: windows
    scope: repo
    path: opencode.json
    notes: Project config file.

manifest:
  file_names: []
  format: other
  required_fields: []
  optional_fields: []
  package_layout: |
    OpenCode plugins have no dedicated manifest. Local plugins are individual `.ts` or `.js` files placed
    in `.opencode/plugins/` or `~/.config/opencode/plugins/`. npm plugins are standard npm packages whose
    `package.json` `name` is referenced in the `plugin` array of `opencode.json`. The plugin entry point
    exports a function matching the `Plugin` type from `@opencode-ai/plugin` and returns a `Hooks` object.
  notes: |
    The `plugin` field in `opencode.json` accepts either a string (npm package name) or a two-element
    tuple `[string, PluginOptions]` for per-plugin options. There is no equivalent to Claude Code's
    `.claude-plugin/plugin.json` container manifest. The local host's `~/.config/opencode/opencode.jsonc`
    contains `"plugin": ["opencode-openai-codex-auth"]`, and `~/.config/opencode/plugin/` contains a
    hand-written `claudine-bridge.ts` file plus a `.disabled` copy.

lifecycle:
  install: |
    npm plugins are installed automatically by OpenCode using Bun at startup. The CLI also exposes
    `opencode plugin <module>` (alias `opencode plug <module>`) with `--global` and `--force` flags.
    Local file plugins require no install step; placing a `.ts`/`.js` file in a plugin directory is enough.
  update: |
    Updates are implicit for npm plugins when the referenced version range resolves to a newer version and
    OpenCode runs `bun install` at startup. There is no documented `plugin update` subcommand or version
    pinning mechanism for plugins.
  remove: |
    Remove the plugin name from the `plugin` array in `opencode.json` or delete the local file from the
    plugin directory. No dedicated `plugin uninstall` command is documented.
  enable_disable: |
    Plugins are enabled by being present in a plugin directory or listed in `opencode.json`. The `--pure`
    global flag disables external plugins for a single invocation. There is no per-plugin enable/disable
    state file.
  trust: |
    Trust is established by the user adding the plugin to config or the plugin directory. There is no
    documented signature verification, sandboxing, approval prompt, or trust model specific to plugins.
  versioning: |
    npm plugins follow the version resolved by Bun from the referenced range. Local file plugins are
    unversioned. There is no plugin-level version field or pinning mechanism.
  notes: |
    Lifecycle is largely implicit and config-driven. The `opencode plugin <module>` command updates the
    config file, but the actual load/install happens at the next startup.

packaged_resources:
  skills: none
  scripts: partial
  slash_commands: none
  subagents: none
  mcp_servers: none
  hooks: full
  prompts: none
  config: partial
  assets: none
  other:
    - custom_tools
    - auth_providers
    - provider_model_hooks
    - tui_extensions

discovery:
  mechanism: |
    OpenCode discovers plugins from three sources at startup: the `plugin` array in `opencode.json`
    (global and project), the global plugin directory `~/.config/opencode/plugins/`, and the project
    plugin directory `.opencode/plugins/`.
  precedence: |
    Config sources load in this order: remote `.well-known/opencode`, global config, `OPENCODE_CONFIG`,
    project `opencode.json`, `.opencode/` directories, `OPENCODE_CONFIG_CONTENT`, managed settings, and
    macOS managed preferences. Plugin directories load after config files in the order: global plugins
    directory, then project plugins directory. Later sources override earlier ones for conflicting keys.
  namespacing: |
    No namespacing is documented for plugins. Local and npm plugins with similar names are loaded
    separately. Custom tools exported from a plugin are keyed by tool name and can override built-in
    tools if the name collides.
  conflicts: |
    Custom tools with the same name as a built-in tool take precedence. Duplicate npm packages with the
    same name and version are loaded once. Exact conflict rules for multiple plugins registering the same
    hook are not documented.
  notes: |
    Unlike Claude Code, OpenCode does not namespace plugin resources as `plugin-name:resource-name`.
    Plugin hooks, tools, and TUI extensions are registered into shared global registries.

security:
  trust_model: |
    Plugins are trusted by placement. Adding a plugin to `opencode.json` or the plugins directory grants
    it full runtime access. There is no documented signature verification, marketplace curation, or
    explicit install approval prompt.
  permissions: |
    Plugins participate in OpenCode's permission system only where the underlying operation is governed
    by it (e.g., tools called by the agent still respect `permission` settings). Plugin code itself can
    bypass those controls by executing shell commands or using the SDK client directly.
  sandbox_interaction: |
    Plugins run unsandboxed inside the OpenCode process with the user's OS privileges. The `PluginInput`
    object provides `client`, `$` (Bun shell), `directory`, `worktree`, and other capabilities. Custom
    tools receive a `ToolContext` with `abort`, `directory`, `worktree`, and an `ask` helper for
    permission-style prompts.
  credential_access: |
    Plugins can read environment variables, access the filesystem, and make network requests. The
    `shell.env` hook explicitly allows plugins to inject environment variables into shell executions.
    The `auth` hook lets plugins implement custom provider authentication flows and receive tokens.
  update_risk: |
    High for npm plugins because version ranges resolve at startup and auto-install silently. Local file
    plugins only change when the file changes. There is no documented pin or lockfile for plugin versions.
  notes: |
    The official docs warn only that custom tools can override built-in tools. There is no guidance on
    auditing, blocking, or sandboxing third-party plugin code.

distribution:
  marketplace: false
  source_types:
    - local folder
    - local file
    - npm package
    - scoped npm package
  publishing: |
    Plugins are published as standard npm packages. The ecosystem page lists community plugins by linking
    to their GitHub repositories, but there is no central marketplace or submission process.
  private_distribution: |
    Private npm packages work through normal npm authentication. Local plugins can be committed to a
    repository under `.opencode/plugins/` or distributed as files.
  notes: |
    There is no official OpenCode marketplace. Discovery is via the community-curated ecosystem page and
    awesome-opencode/opencode.cafe lists. npm is the de facto distribution channel.

portability:
  link_plugin_as_unit: false
  extract_resources: false
  portable_resources: []
  non_portable_assets:
    - plugin source code
    - "@opencode-ai/plugin API bindings"
    - Bun shell invocations
    - OpenCode SDK client usage
    - custom tool definitions
    - auth provider hooks
    - provider model hooks
    - TUI extensions
    - shell.env hook implementations
  rewrite_needed: false
  notes: |
    OpenCode plugins are executable code modules tightly coupled to the OpenCode runtime and SDK, not
    declarative resource containers. Claudine should not attempt to link or rewrite them for other
    providers. The observed `claudine-bridge.ts` plugin is an example of an OpenCode-specific bridge that
    forwards OpenCode events to Claudine; it is non-portable in the opposite direction as well.

cli_params:
  - flag: opencode plugin <module>
    description: Install an npm plugin and add it to the config.
    example: opencode plugin opencode-helicone-session
  - flag: opencode plug <module>
    description: Alias for opencode plugin.
    example: opencode plug opencode-helicone-session
  - flag: --global / -g
    description: Install the plugin into the global config instead of the project config.
    example: opencode plugin opencode-wakatime --global
  - flag: --force / -f
    description: Replace an existing plugin version in the config.
    example: opencode plugin opencode-wakatime --force
  - flag: --pure
    description: Global flag that disables external plugins for this invocation.
    example: opencode run --pure "hello"
  - flag: opencode debug config
    description: Show the resolved configuration, including loaded plugin references.
    example: opencode debug config

env_vars:
  - name: OPENCODE_CONFIG
    effect: Path to a custom config file; loaded between global and project configs.
  - name: OPENCODE_CONFIG_DIR
    effect: Path to a custom config directory that is searched for plugins like `.opencode/`.
  - name: OPENCODE_CONFIG_CONTENT
    effect: Inline JSON config content; loaded after `.opencode/` directories.
  - name: OPENCODE_DISABLE_DEFAULT_PLUGINS
    effect: Set to disable default plugins.
  - name: OPENCODE_DISABLE_AUTOUPDATE
    effect: Disable automatic update checks for OpenCode itself.
  - name: OPENCODE_DISABLE_CLAUDE_CODE
    effect: Disable reading `.claude/` prompts and skills.
  - name: OPENCODE_DISABLE_CLAUDE_CODE_PROMPT
    effect: Disable reading `~/.claude/CLAUDE.md`.
  - name: OPENCODE_DISABLE_CLAUDE_CODE_SKILLS
    effect: Disable loading `.claude/skills`.
  - name: OPENCODE_PURE
    effect: Not documented as an environment variable; use the `--pure` CLI flag.

gaps:
  - No dedicated plugin manifest or container format; plugins are raw JS/TS modules.
  - No documented trust model, code signing, sandbox, or approval flow for plugins.
  - No documented plugin list, enable/disable, or uninstall subcommands beyond `opencode plugin <module>`.
  - No documented versioning or lockfile behavior for npm plugins.
  - Exact precedence when multiple plugins register the same hook is undocumented.
  - Official docs do not state whether plugin code runs in the main process or a separate worker.
  - The local `opencode --help` output (observed on this host) did not list a `plugin` subcommand,
    although the online CLI reference does; the command may depend on OpenCode version.

changes: []

requires_claudine_update: false
reason: |
  OpenCode plugins are code modules bound to the OpenCode SDK and plugin API. They do not package
  portable resources such as Agent Skills, slash commands, or subagents in a way Claudine can extract
  and share with other providers. Claudine should ignore OpenCode plugin files when linking resources
  across providers, treating them as non-portable runtime extensions. The existing Claudine bridge
  plugin is already a per-provider integration point, not a container to be linked.
---

# OpenCode CLI Plugins

## Overview

OpenCode implements plugins as **executable JavaScript/TypeScript modules** rather than as declarative,
manifest-driven containers. A plugin is a file that exports a function conforming to the `Plugin` type
from `@opencode-ai/plugin`; that function receives a `PluginInput` context and returns a `Hooks` object
that registers callbacks for lifecycle events, custom tools, authentication flows, provider model
overrides, TUI extensions, and more.

This design differs sharply from [Claude Code's plugin system](https://code.claude.com/docs/en/plugins),
which treats a plugin as a directory containing a `.claude-plugin/plugin.json` manifest and discrete
resource folders (`skills/`, `commands/`, `agents/`, `hooks/`, `.mcp.json`, etc.). Claude Code plugins
are versioned, namespaced, and installable from marketplaces; OpenCode plugins are code-first modules
loaded from the filesystem or npm, with no manifest, no namespacing, and no marketplace.

Plugins are loaded at OpenCode startup. The official docs list three loading mechanisms:

1.  npm packages named in the `plugin` array of `opencode.json`.
2.  Local `.ts`/`.js` files in the global plugin directory `~/.config/opencode/plugins/`.
3.  Local `.ts`/`.js` files in the project plugin directory `.opencode/plugins/`.

The local host's `~/.config/opencode/opencode.jsonc` references one npm plugin
(`"plugin": ["opencode-openai-codex-auth"]`), and `~/.config/opencode/plugin/` contains a hand-written
`claudine-bridge.ts` file plus a `.disabled` copy.

## Installation and Locations

OpenCode stores plugin-related state in the user's config directory and in a Bun package cache.

| Location | Purpose |
|---|---|
| `~/.config/opencode/plugins/` | Global plugin directory. Any `.ts` or `.js` file is auto-loaded. |
| `.opencode/plugins/` | Project plugin directory. Loaded after the global directory. |
| `~/.cache/opencode/node_modules/` | Bun install cache for npm plugins referenced in `opencode.json`. |
| `opencode.json` / `~/.config/opencode/opencode.json` | Config files where the `plugin` array lists npm packages to load. |

The `plugin` config field accepts either a string package name or a tuple `[packageName, options]` for
per-plugin options:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": [
    "opencode-helicone-session",
    ["@my-org/custom-plugin", { "team": "platform" }]
  ]
}
```

Config sources are merged, not replaced. The documented precedence order is:

1. Remote config from `.well-known/opencode`
2. Global config (`~/.config/opencode/opencode.json`)
3. `OPENCODE_CONFIG`
4. Project config (`opencode.json`)
5. `.opencode/` directories
6. `OPENCODE_CONFIG_CONTENT`
7. Managed settings (`/Library/Application Support/opencode/`, `/etc/opencode/`, `%ProgramData%\opencode`)
8. macOS managed preferences via MDM

## Manifest and Package Format

OpenCode plugins have **no dedicated manifest file**. Local plugins are single source files; npm plugins
are standard npm packages. The only metadata OpenCode reads is the npm package name from `package.json`
when resolving the `plugin` array.

A local plugin file looks like this:

```typescript
// .opencode/plugins/example.ts
import type { Plugin } from "@opencode-ai/plugin";

export const MyPlugin: Plugin = async ({ project, client, $, directory, worktree }) => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.idle") {
        // handle event
      }
    },
  };
};
```

The exported function's return type, `Hooks`, is defined in `@opencode-ai/plugin` and includes fields
such as `event`, `config`, `tool`, `auth`, `provider`, `chat.message`, `chat.params`, `chat.headers`,
`permission.ask`, `command.execute.before`, `tool.execute.before`, `tool.execute.after`, `shell.env`,
and several `experimental.*` hooks.

This is unlike Claude Code, where a plugin must declare itself in `.claude-plugin/plugin.json` with a
`name`, `version`, and explicit path mappings for each resource type. OpenCode's approach is simpler for
code extensions but provides no standardized way to package skills, commands, or agents as discrete,
shareable files.

## Packaged Resources

Because OpenCode plugins are code modules, they do not "contain" the declarative resources that Claude
Code plugins can bundle. The following table classifies what a plugin can expose through its hooks:

| Resource | Support | Notes |
|---|---|---|
| Agent Skills | none | Skills live separately in `.opencode/skills/` or `~/.claude/skills/` and are loaded by the `skill` tool. |
| Slash commands | none | Commands are Markdown files in `.opencode/commands/` or entries in `opencode.json` `command`. |
| Subagents | none | Agents are defined in `.opencode/agents/` or `opencode.json` `agent`. |
| MCP servers | none | MCP servers are configured in `opencode.json` `mcp`. |
| Hooks | full | Plugins are fundamentally hook-based; `event`, `tool.execute.before`, `shell.env`, etc. |
| Scripts | partial | Plugins can execute arbitrary shell commands via `PluginInput.$` and spawn child processes. |
| Custom tools | full | Plugins can register Zod-typed tools via the `tool` helper exported by `@opencode-ai/plugin/tool`. |
| Auth providers | full | Plugins can implement custom OAuth/API auth flows via the `auth` hook. |
| Provider models | full | Plugins can inject or override provider models via the `provider` hook. |
| TUI extensions | full | Plugins can register commands, routes, keybinds, slots, and themes via `@opencode-ai/plugin/tui`. |
| Config | partial | The `config` hook can mutate the resolved config object at runtime. |
| Prompts | none | There is no dedicated prompt container; use skills or commands instead. |
| Assets | none | No plugin-level asset directory is defined. |

The official [custom tools](https://opencode.ai/docs/custom-tools) docs show that a plugin's `tool`
hook exposes custom tools alongside built-in tools; if a custom tool shares a built-in name, it
overrides the built-in tool.

## Lifecycle and Trust

Install:

- npm plugins are installed automatically by Bun at startup when listed in `opencode.json`.
- The CLI command `opencode plugin <module>` (alias `opencode plug <module>`) adds an npm package to
  the `plugin` array with optional `--global` and `--force` flags.
- Local plugins require no install step; dropping a `.ts`/`.js` file into a plugin directory is enough.

Update:

- npm plugins update implicitly when the referenced version range resolves to a newer version and
  OpenCode runs `bun install` at startup. There is no `plugin update` command.
- Local plugins update when the file changes.

Remove:

- Delete the local plugin file or remove the package name from the `plugin` array in `opencode.json`.
- There is no `plugin uninstall` command documented.

Enable/disable:

- Plugins are enabled by presence. The only documented disable mechanism is the `--pure` global flag,
  which runs OpenCode without external plugins.

Trust:

- Trust is implicit: a plugin added to config or the plugin directory runs with the user's privileges.
- There is no documented signature verification, sandbox, approval prompt, or blocklist.
- This contrasts with Claude Code, which uses explicit `enabledPlugins` entries, workspace trust for
  project-scope plugins, managed-settings restrictions, and a `~/.claude/plugins/blocklist.json`.

## Discovery and Precedence

OpenCode discovers plugins from all three sources and loads them in a documented order. The config
sources load first; then plugin directories load in this order:

1. Global plugin directory (`~/.config/opencode/plugins/`)
2. Project plugin directory (`.opencode/plugins/`)

The overall config precedence is documented in the [config locations](https://opencode.ai/docs/config#locations)
section. Later sources override earlier ones for conflicting keys, but the docs do not specify exact
merge rules when multiple plugins register the same hook.

Duplicate npm packages with the same name and version are loaded once. A local plugin and an npm plugin
with similar names are loaded separately. Custom tools override built-in tools on name collision.

Unlike Claude Code, there is no namespacing such as `plugin-name:resource-name`. Plugin hooks and tools
are registered into shared global registries, so name collisions are possible and only partially
documented.

## Security and Runtime Behavior

Plugins run with the full privileges of the OpenCode process:

- They can execute shell commands via `PluginInput.$` (Bun shell), spawn child processes, and access the
  filesystem and network.
- The `shell.env` hook lets a plugin inject environment variables into every shell execution performed
  by OpenCode, including AI tool calls.
- The `auth` hook lets a plugin implement OAuth/API flows and receive tokens.
- Custom tools run with a `ToolContext` that exposes `directory`, `worktree`, `sessionID`, `messageID`,
  `agent`, an `ask` permission helper, and an `abort` signal.
- There is no documented sandbox, process isolation, or permission prompt specific to plugin loading.

The official [permissions](https://opencode.ai/docs/permissions) docs apply to built-in tools invoked by
the agent, but a plugin can bypass those controls by executing its own code. This is a significant
difference from Claude Code, where plugin MCP servers and hooks require the same per-server approvals as
user MCP servers and are gated by workspace trust for project-scope loading.

## Distribution

OpenCode has no official marketplace or plugin registry. Distribution channels are:

- **npm packages** — the primary distribution method. The `plugin` array in `opencode.json` references
  package names, and OpenCode installs them with Bun.
- **Local files** — committed to a repository under `.opencode/plugins/` or shared as files in
  `~/.config/opencode/plugins/`.
- **Community curation** — the [ecosystem page](https://opencode.ai/docs/ecosystem#plugins) lists
  community plugins by linking to their GitHub repos. Third-party lists such as
  [awesome-opencode](https://github.com/awesome-opencode/awesome-opencode) and
  [opencode.cafe](https://opencode.cafe) also exist.

Publishing is therefore the same as publishing any npm package. Private distribution works through
private npm registries or repository-committed files.

## Portability

OpenCode plugins are **not portable** to other agentic CLI providers. They are executable code modules
that import `@opencode-ai/plugin`, `@opencode-ai/sdk`, Bun shell APIs, and OpenCode-specific types. The
observed `claudine-bridge.ts` plugin on this host is itself an OpenCode-specific bridge that forwards
OpenCode events to Claudine; it cannot be reused in Claude Code, Codex, or other providers.

Claudine should not attempt to link OpenCode plugin files as units, nor should it try to extract
contained skills, commands, or agents from them. The appropriate integration point is the kind of
bridge plugin already present: an OpenCode plugin that calls into Claudine at runtime.

## Claudine Linking Notes

- Do not include `~/.config/opencode/plugins/` or `.opencode/plugins/` files in cross-provider resource
  linking. They are runtime code, not portable resource containers.
- Do not treat npm plugins listed in `opencode.json` `plugin` as source packages for skills, commands,
  agents, or MCP servers. They may implement hooks and tools, but those implementations are
  OpenCode-specific.
- If Claudine needs to react to OpenCode events, the supported pattern is an OpenCode plugin that
  forwards events to Claudine (as the existing `claudine-bridge.ts` does), not the reverse.
- Keep OpenCode's separate resource conventions in mind: skills live in `.opencode/skills/`, commands in
  `.opencode/commands/`, agents in `.opencode/agents/`, tools in `.opencode/tools/`, and MCP servers in
  `opencode.json` `mcp`. Those sibling topics are documented separately; this plugin research does not
  govern their semantics.

## Sources

- [OpenCode — Plugins](https://opencode.ai/docs/plugins)
- [OpenCode — Config](https://opencode.ai/docs/config)
- [OpenCode — CLI reference](https://opencode.ai/docs/cli)
- [OpenCode — Custom tools](https://opencode.ai/docs/custom-tools)
- [OpenCode — Commands](https://opencode.ai/docs/commands)
- [OpenCode — Agent Skills](https://opencode.ai/docs/skills)
- [OpenCode — Ecosystem](https://opencode.ai/docs/ecosystem)
- [OpenCode config schema](https://opencode.ai/config.json)
- [Claude Code — Plugins](https://code.claude.com/docs/en/plugins)
