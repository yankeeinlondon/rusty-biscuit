---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
homepage: https://antigravity.google/
docs: https://antigravity.google/docs
plugin_docs: https://antigravity.google/docs/plugins
support: partial
locations:
  - os: macos
    scope: user
    path: "~/.gemini/config/plugins/<plugin-name>/"
    notes: "Global Antigravity 2.0 and IDE plugin location documented for manually added plugins."
  - os: linux
    scope: user
    path: "~/.gemini/config/plugins/<plugin-name>/"
    notes: "Same documented home-relative global plugin path; exact XDG behavior is not documented."
  - os: windows
    scope: user
    path: "~/.gemini/config/plugins/<plugin-name>/"
    notes: "Documentation uses a Unix-style home path; Windows expansion behavior is not documented."
  - os: macos
    scope: user
    path: "~/.gemini/antigravity-cli/plugins/<plugin_name>/"
    notes: "Antigravity CLI stages installed or imported plugins here."
  - os: linux
    scope: user
    path: "~/.gemini/antigravity-cli/plugins/<plugin_name>/"
    notes: "Same documented home-relative CLI plugin path; exact XDG behavior is not documented."
  - os: windows
    scope: user
    path: "~/.gemini/antigravity-cli/plugins/<plugin_name>/"
    notes: "Documentation uses a Unix-style home path; Windows expansion behavior is not documented."
  - os: macos
    scope: repo
    path: "<workspace-root>/.agents/plugins/<plugin-name>/"
    notes: "Workspace plugin path scanned from the opened workspace root."
  - os: linux
    scope: repo
    path: "<workspace-root>/.agents/plugins/<plugin-name>/"
    notes: "Workspace plugin path scanned from the opened workspace root."
  - os: windows
    scope: repo
    path: "<workspace-root>/.agents/plugins/<plugin-name>/"
    notes: "Workspace plugin path scanned from the opened workspace root."
  - os: macos
    scope: repo
    path: "<workspace-root>/_agents/plugins/<plugin-name>/"
    notes: "Alternative workspace plugin path documented for Antigravity 2.0 and IDE."
  - os: linux
    scope: repo
    path: "<workspace-root>/_agents/plugins/<plugin-name>/"
    notes: "Alternative workspace plugin path documented for Antigravity 2.0 and IDE."
  - os: windows
    scope: repo
    path: "<workspace-root>/_agents/plugins/<plugin-name>/"
    notes: "Alternative workspace plugin path documented for Antigravity 2.0 and IDE."
  - os: macos
    scope: extension
    path: "~/.antigravity/extensions/"
    notes: "Observed local VS Code-style IDE extensions; this is separate from Antigravity agent plugins."
manifest:
  file_names:
    - plugin.json
    - mcp_config.json
    - hooks.json
  format: json
  required_fields:
    - name
  optional_fields:
    - $schema
    - description
  package_layout: "Plugin root contains plugin.json. Optional contained resources are mcp_config.json, hooks.json, skills/<skill-name>/SKILL.md, rules/<rule-name>.md, and in CLI docs agents/ for subagent definition templates."
  notes: "Antigravity 2.0 and IDE docs say plugin.json only marks the directory and name is optional with directory-name fallback. CLI docs and the published schema require name, allow optional description, and reject additional manifest properties."
lifecycle:
  install: "Antigravity 2.0/IDE: add Google-built bundled plugins through the Customizations UI or manually place plugin folders in the documented global/workspace plugin directories. CLI docs describe agy plugin install <target>; local CLI help says install supports plugin@marketplace."
  update: "No plugin update command is documented. The CLI has a general agy update command for the CLI itself, not plugin updates."
  remove: "CLI docs and help provide agy plugin uninstall <name>. Antigravity 2.0/IDE docs do not document a remove flow for manually placed plugin folders beyond filesystem removal."
  enable_disable: "CLI docs and help provide agy plugin enable <name> and agy plugin disable <name>. Antigravity 2.0/IDE docs do not document enable/disable controls for whole plugins."
  trust: "No plugin-specific trust, signing, review, or safe-mode lifecycle is documented. Trust behavior appears to come from the normal permissions, sandbox, MCP, and hook execution controls."
  versioning: "No plugin version field, lockfile, pinning command, or update channel is documented in plugin.json."
  notes: "Top-level local CLI help exposes plugin import, validate, and link commands in addition to the docs. Per-subcommand --help is not implemented consistently; some subcommands treat --help as an operand."
packaged_resources:
  skills: full
  scripts: partial
  slash_commands: partial
  subagents: partial
  mcp_servers: full
  hooks: full
  prompts: partial
  config: partial
  assets: partial
  other:
    - rules
    - workflows through rules/customizations outside the documented plugin layout
    - VS Code-style IDE extensions outside the agent plugin system
discovery:
  mechanism: "Antigravity scans documented plugin directories automatically. CLI docs say installed/imported plugins are staged under ~/.gemini/antigravity-cli/plugins/<plugin_name>/; Antigravity 2.0 and IDE docs say manually added workspace and global plugin folders are scanned and loaded."
  precedence: "Not documented. The docs identify workspace-level and global-level plugin locations but do not specify merge order, built-in versus plugin order, or plugin versus non-plugin resource precedence."
  namespacing: "Plugins are described as namespaced bundles. CLI manifest name is the unique machine-readable name used in CLI commands. Exact runtime prefixes for packaged skills, rules, hooks, MCP servers, and agents are not documented."
  conflicts: "Not documented beyond the CLI manifest name uniqueness requirement and name pattern. Behavior for duplicate plugin names or contained resource name collisions is unknown."
  notes: "Skills outside plugins are discovered separately from .agents/skills and global skills directories. Skills become slash commands in the CLI, but the docs do not describe whether plugin-contained skills use plugin prefixes or shadow ordinary skills."
security:
  trust_model: "No dedicated plugin trust model is documented. Plugin-provided hooks can execute shell commands, MCP definitions can start local stdio processes or connect to remote servers, and skills/rules influence the agent prompt. Those resources should be treated as trusted code/config only after user review."
  permissions: "The CLI permissions engine evaluates sensitive operations through allow, ask, and deny lists with Deny > Ask > Allow precedence. MCP tool calls are covered by mcp(server/tool) permissions. Hook PreToolUse handlers can return allow, deny, ask, or force_ask decisions plus permissionOverrides."
  sandbox_interaction: "CLI terminal sandboxing is a global/session setting, not plugin-specific. When enabled, local execution commands run under nsjail on Linux, sandbox-exec on macOS, and AppContainer on Windows; approval prompts can run a specific command without sandbox restrictions."
  credential_access: "Plugin MCP configurations may include env values, headers, OAuth client credentials, and Google ADC auth provider settings. Hook commands receive transcript paths, workspace paths, and artifact directories over stdin and run as local commands, so they can access credentials available to that process unless blocked by OS/user policy."
  update_risk: "Manual plugin folders and CLI-installed plugin directories can change behavior when files are edited or reinstalled. No signature, provenance, lockfile, pinning, or audit command is documented."
  notes: "Local inspection found no imported agent plugins. ~/.antigravity/extensions contained VS Code-style IDE extensions with package.json/.vsixmanifest files, but those are not the documented Antigravity agent plugin container."
distribution:
  marketplace: true
  registry_url: https://antigravity.google/docs/build-with-google
  source_types:
    - bundled Google plugins through UI
    - local folder
    - remote target
    - plugin@marketplace
    - imported Gemini configuration
    - imported Claude configuration
  publishing: "No public publishing workflow, review policy, ownership model, or package upload command is documented. The UI exposes Google-built bundled plugins; CLI help exposes link <mp> <target> but docs do not explain marketplace publishing."
  private_distribution: "Manual workspace/global folders and CLI local/remote install targets imply private sharing by copying folders or pointing the CLI at a target, but private registry behavior is not documented."
  notes: "The CLI docs mention local or remote install. Local CLI help adds plugin@marketplace install syntax and import from gemini or claude."
portability:
  link_plugin_as_unit: true
  extract_resources: true
  portable_resources:
    - skills
    - rules
    - MCP server definitions after credential review
    - hooks after executable review
    - subagent templates when their schema is known
  non_portable_assets:
    - plugin.json provider metadata
    - hooks that execute provider-specific commands
    - MCP credentials and OAuth client secrets
    - marketplace IDs or plugin@marketplace targets
    - VS Code-style IDE extensions under ~/.antigravity/extensions
    - unknown agents/ template schema
  rewrite_needed: true
  notes: "Claudine can preserve an Antigravity plugin as a unit for Antigravity targets, but cross-provider linking should extract known resources. Skills use the Agent Skills folder/SKILL.md shape and are the most portable. Rules, hooks, MCP configs, and CLI agents/ metadata require provider-aware rewrite or should be marked non-portable until their target provider semantics are known."
cli_params:
  - flag: "agy plugin list"
    description: "List imported plugins."
    example: "agy plugin list"
  - flag: "agy plugins list"
    description: "Alias form for agy plugin list."
    example: "agy plugins list"
  - flag: "agy plugin install <target>"
    description: "Install a plugin from a local or remote target; local help says plugin@marketplace is supported."
    example: "agy plugin install /path/to/local/plugin"
  - flag: "agy plugin uninstall <name>"
    description: "Uninstall a plugin."
    example: "agy plugin uninstall my-plugin"
  - flag: "agy plugin enable <name>"
    description: "Enable a disabled plugin."
    example: "agy plugin enable my-plugin"
  - flag: "agy plugin disable <name>"
    description: "Disable a plugin without deleting its assets."
    example: "agy plugin disable my-plugin"
  - flag: "agy plugin import [source]"
    description: "Import plugins from gemini or claude according to local CLI help; not explained in the published plugin docs."
    example: "agy plugin import claude"
  - flag: "agy plugin validate [path]"
    description: "Validate a plugin directory according to local CLI help."
    example: "agy plugin validate /path/to/plugin"
  - flag: "agy plugin link <mp> <target>"
    description: "Generate a link to a marketplace according to local CLI help; publishing semantics are undocumented."
    example: "agy plugin link <marketplace> <target>"
env_vars: []
gaps:
  - "No plugin precedence, conflict, or shadowing rules are documented."
  - "No plugin-specific trust, signing, safe mode, provenance, audit, or pinning behavior is documented."
  - "Antigravity 2.0/IDE plugin docs and CLI plugin docs disagree on whether plugin.json name is optional or required."
  - "The CLI docs omit plugin import, validate, link, marketplace target syntax, and any detailed help for those subcommands."
  - "No OS-specific Windows or XDG Linux path expansion is documented for home-relative plugin paths."
  - "No public plugin publishing workflow is documented for Google-built bundled plugins or marketplace targets."
  - "The schema and docs do not define the shape of plugin-contained agents/ subagent templates."
  - "Local CLI per-subcommand --help is unreliable because some plugin subcommands treat --help as an operand."
changes: []
requires_claudine_update: true
reason: "Claudine should add Antigravity as a plugin-aware provider with support for linking plugin directories as units for Antigravity, extracting portable Agent Skills, and marking hooks, MCP credentials, rules, and agents/ templates with provider-specific portability notes."
---

# Antigravity Plugin Containers

## Overview

Antigravity has a documented agent plugin container, but the implementation is only partially specified. The official Antigravity 2.0 and IDE plugin pages define plugins as namespaced bundles that group skills, rules, MCP servers, and hooks into one package. The Antigravity CLI plugin page expands that list to custom skills, background subagents, linting rules, MCP definitions, and event hooks.

This differs from Claude Code in an important way. Claude Code publishes a marketplace install flow for prebuilt plugins in [Discover and install prebuilt plugins](https://code.claude.com/docs/en/discover-plugins), plus a marketplace creation format in [Create and distribute a plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces). Antigravity documents Google-built bundled plugins in the UI and local/remote CLI install targets, but it does not document a public marketplace publishing model, plugin trust workflow, or complete lifecycle semantics. Claude Code also documents plugin resources as a self-contained package that can contain skills, agents, hooks, MCP servers, LSP servers, and monitors in [Plugins reference](https://code.claude.com/docs/en/plugins-reference), while Antigravity documents a narrower, file-layout-driven package rooted at `plugin.json`.

Local inspection on this host found `/Users/ken/.antigravity`, `/Users/ken/.gemini/antigravity`, `/Users/ken/.gemini/antigravity-cli`, and `/Users/ken/.gemini/config`. `/Users/ken/.antigravity/extensions` contains VS Code-style IDE extensions with `package.json` and `.vsixmanifest` files; these are editor extensions, not the Antigravity agent plugin container. No imported Antigravity agent plugins were present under `/Users/ken/.gemini/antigravity-cli`, `/Users/ken/.gemini/antigravity`, or `/Users/ken/.gemini/config`. `agy plugin list` reported `No imported plugins.`

## Installation and Locations

Antigravity 2.0 and IDE support two manually scanned agent plugin scopes:

| Scope | Path | Behavior |
| --- | --- | --- |
| Workspace | `<workspace-root>/.agents/plugins/<plugin-name>/` | Available only when the workspace is open. |
| Workspace | `<workspace-root>/_agents/plugins/<plugin-name>/` | Alternative workspace plugin path documented by Antigravity. |
| Global | `~/.gemini/config/plugins/<plugin-name>/` | Active across workspaces. |

Antigravity CLI documents a different staging location for installed or imported plugin bundles:

```text
~/.gemini/antigravity-cli/plugins/<plugin_name>/
```

The CLI top-level help on this host exposes `agy plugin` and `agy plugins` as aliases. Published CLI docs list `list`, `install`, `disable`, `enable`, and `uninstall`. Local CLI help additionally lists `import [source]`, `validate [path]`, and `link <mp> <target>`, and says `install` supports `plugin@marketplace`.

Antigravity 2.0 and IDE docs also mention bundled Google-built plugins available from the Customizations UI and link to a Build with Google page. The docs do not specify where UI-installed bundled plugins are stored on disk, whether they are copied into the same global plugin folder, or how they are updated.

## Manifest and Package Format

The plugin root must contain `plugin.json`. Antigravity 2.0 and IDE docs describe it as a required marker file and say the `name` field is optional, defaulting to the directory name when omitted:

```json
{
  "name": "my-custom-plugin"
}
```

The CLI docs and the published schema are stricter. The CLI manifest schema requires `name`, allows optional `description`, and rejects additional properties:

```json
{
  "$schema": "https://antigravity.google/schemas/v1/plugin.json",
  "name": "my-plugin",
  "description": "A brief description of what my plugin does."
}
```

The schema constrains `name` to `^[a-zA-Z0-9-_]+$`. That name is used by CLI plugin commands. There is no documented `version`, `author`, `license`, `dependencies`, `engines`, `activationEvents`, permissions declaration, signature, or resource list field.

Documented Antigravity 2.0 and IDE layout:

```text
plugins/<plugin-name>/
├── plugin.json
├── mcp_config.json
├── hooks.json
├── skills/
│   └── <skill-name>/
│       └── SKILL.md
└── rules/
    └── <rule-name>.md
```

Documented CLI layout:

```text
~/.gemini/antigravity-cli/plugins/<plugin_name>/
├── plugin.json
├── mcp_config.json
├── hooks.json
├── skills/
├── agents/
└── rules/
```

The `agents/` directory appears only in CLI plugin docs. Its schema, filenames, and relation to runtime `define_subagent` or `invoke_subagent` tools are not documented.

## Packaged Resources

Antigravity plugins can package these resources:

| Resource | Support | Evidence and limits |
| --- | --- | --- |
| Agent Skills | Full | `skills/<skill-name>/SKILL.md` is documented in plugin layout. Skills follow the Agent Skills folder shape. |
| Scripts | Partial | Skills may include scripts/resources, and hooks execute shell commands, but there is no generic plugin `scripts/` resource declared in `plugin.json`. |
| Slash commands | Partial | CLI docs say registered skills become slash commands in the TUI. Workflows also run as `/workflow-name`, but workflows are not in the documented plugin layout. |
| Subagents | Partial | CLI docs list `agents/` as optional subagent templates, but no file format is documented. |
| MCP servers | Full | `mcp_config.json` at plugin root is documented. It uses the shared `mcpServers` configuration shape. |
| Hooks | Full | `hooks.json` at plugin root is documented. Hooks can run command handlers for PreToolUse, PostToolUse, PreInvocation, PostInvocation, and Stop. |
| Prompts | Partial | Skills and rules are prompt-like Markdown resources; no generic prompt package surface is documented. |
| Config | Partial | Plugins can package MCP and hook config. General settings are not documented as plugin-contained resources. |
| Assets | Partial | Skills may include resources/templates, but there is no documented top-level plugin assets directory. |
| Rules | Full | `rules/<rule-name>.md` is documented in plugin layout. |

Skills in Antigravity are close to Claude Code skills at the container level: both use a folder with `SKILL.md`, YAML frontmatter, progressive disclosure, and optional scripts/resources. Antigravity’s plugin docs explicitly include skills in plugins; Claude Code’s plugin docs also treat skills as one plugin resource type.

## Lifecycle and Trust

CLI lifecycle commands documented or observed:

| Action | Command or surface | Notes |
| --- | --- | --- |
| List | `agy plugin list` | Local CLI reported no imported plugins. |
| Install | `agy plugin install <target>` | Docs say local or remote target; local help says `plugin@marketplace` is supported. |
| Import | `agy plugin import [source]` | Local help says import from `gemini` or `claude`; published docs do not explain it. |
| Validate | `agy plugin validate [path]` | Local help lists it; published docs do not explain validation output. |
| Link | `agy plugin link <mp> <target>` | Local help lists it; publishing and marketplace semantics are undocumented. |
| Enable | `agy plugin enable <name>` | Docs say this re-enables disabled plugin tools. |
| Disable | `agy plugin disable <name>` | Docs say this suspends plugin tools without deleting assets. |
| Remove | `agy plugin uninstall <name>` | Docs say this purges the package directory and registries. |

No plugin-specific update command is documented. No plugin version pinning, lockfile, audit, signature, provenance, or safe-mode behavior is documented.

Trust is therefore resource-specific rather than plugin-specific. Hooks run commands. MCP server definitions may launch local stdio commands or connect to remote servers. Skills and rules alter prompt context. The CLI permissions engine and sandbox can gate sensitive operations, but Antigravity does not document a separate plugin install approval or signed package trust boundary.

## Discovery and Precedence

Antigravity 2.0 and IDE automatically scan the documented global and workspace plugin directories. CLI docs say installed or imported plugin bundles are staged under `~/.gemini/antigravity-cli/plugins/<plugin_name>/`.

The docs do not define load order, resource precedence, conflict resolution, or shadowing. Unknowns that matter for Claudine:

- Whether workspace plugins shadow global plugins.
- Whether plugin skills shadow global or workspace skills outside plugins.
- Whether plugin rules are ordered before or after `.agents/rules`.
- Whether duplicate plugin names are rejected, last-writer-wins, or merged.
- Whether plugin-contained skills, hooks, rules, MCP servers, and agents are exposed with a plugin prefix.

Claude Code is more explicit about plugin command namespacing: its [Create plugins](https://code.claude.com/docs/en/plugins) documentation contrasts standalone skill names with plugin-provided `/plugin-name:hello` names. Antigravity documents plugins as namespaced bundles and says the CLI manifest name references the plugin in CLI commands, but it does not document an equivalent runtime prefix for contained resources.

## Security and Runtime Behavior

Plugin-provided hooks are executable code. `hooks.json` maps hook names to event configurations and command handlers. A handler supports `type`, `command`, and `timeout`; `command` is required and `timeout` defaults to 30 seconds. Hook commands receive JSON on stdin and return JSON on stdout.

For `PreToolUse`, a hook can return:

- `allow`
- `deny`
- `ask`
- `force_ask`

It can also return `permissionOverrides`, such as `command(npm test)`. `PostToolUse` returns `{}`. `PreInvocation` and `PostInvocation` can inject steps. `Stop` can return `continue` to re-enter the loop.

The hook common input includes `conversationId`, absolute `workspacePaths`, `transcriptPath`, and `artifactDirectoryPath`. The transcript path is under `~/.gemini/antigravity/brain/<conversationId>/.system_generated/logs/transcript.jsonl` for Antigravity 2.0 and under `~/.gemini/antigravity-cli/brain/<conversationId>/.system_generated/logs/transcript.jsonl` for CLI. This means plugin hook commands can see local conversation metadata and paths.

Plugin-provided MCP servers use the shared `mcpServers` shape. Local stdio servers can declare `command`, `args`, `env`, and `cwd`. Remote servers use `serverUrl`, `headers`, `authProviderType`, and optional OAuth client credentials. Antigravity stores OAuth access tokens for IDE/Antigravity 2.0 in `~/.gemini/antigravity/mcp_oauth_tokens.json`; the docs say expired tokens refresh automatically and invalid tokens are removed.

The CLI permission engine evaluates resources as `action(target)` with Deny > Ask > Allow precedence. Relevant plugin surfaces include:

- `command(...)` for hook-triggered or agent-triggered terminal commands.
- `unsandboxed(...)` for bypassing terminal sandbox restrictions.
- `mcp(server/tool)` for MCP tools.
- `read_file(...)`, `write_file(...)`, `read_url(...)`, and `execute_url(...)` for other sensitive operations.

The CLI terminal sandbox is not plugin-specific. It can restrict local execution commands using `nsjail` on Linux, `sandbox-exec` on macOS, and `AppContainer` on Windows. Users can enable it with `enableTerminalSandbox` in `~/.gemini/antigravity-cli/settings.json` or pass `--sandbox` for a session. Approval prompts can also choose one command to run inside or outside the sandbox depending on the current sandbox state.

## Distribution

Antigravity supports at least four distribution paths:

- Google-built bundled plugins through the Antigravity 2.0/IDE Customizations UI.
- Manual workspace or global plugin folders.
- CLI local or remote install targets through `agy plugin install <target>`.
- CLI marketplace-style targets through `plugin@marketplace`, according to local help.

The CLI also exposes `agy plugin import [source]`, with local help identifying `gemini` and `claude` as sources. That makes Antigravity unusual compared with Claude Code: Antigravity appears to include a migration/import path from Claude and Gemini configuration, while Claude Code’s official plugin flow centers on discovering, adding, and installing plugins through marketplaces as documented in [Discover and install prebuilt plugins](https://code.claude.com/docs/en/discover-plugins) and [Create and distribute a plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces).

No Antigravity public publishing workflow is documented. The Build with Google page is referenced for Google-built bundled plugins, but the plugin docs do not explain how third parties publish, whether packages are moderated, whether private registries exist, or how marketplace names map to install targets.

## Portability

Claudine should treat Antigravity plugins as portable only for Antigravity unless the contained resources are extracted and rewritten.

Portable or partially portable resources:

- `skills/<skill-name>/SKILL.md`: portable to providers that support Agent Skills or can ingest skill Markdown after metadata rewrite.
- `rules/<rule-name>.md`: portable as prompt/rule text only after target-provider rule semantics are mapped.
- `mcp_config.json`: portable in structure, but credentials, environment variables, OAuth metadata, local commands, and disabled tool lists require review.
- `hooks.json`: not portable as behavior unless the target provider supports equivalent hook events, matchers, stdin/stdout contracts, and permission decisions.
- `agents/`: not portable until Antigravity documents the template format or local examples are available.

Non-portable or high-risk assets:

- `plugin.json` as provider-specific container metadata.
- Hook command scripts and executable references.
- MCP `env`, `headers`, OAuth client secrets, and local command paths.
- Marketplace identifiers or generated marketplace links.
- VS Code-style IDE extensions under `~/.antigravity/extensions`.

For Antigravity targets, Claudine can link the plugin as a unit by creating or syncing a plugin directory under the appropriate Antigravity plugin location. For other targets, Claudine should extract known resources and emit non-portability diagnostics for hooks, MCP credentials, and unknown `agents/` content.

## Claudine Linking Notes

Implementation guidance:

- Add Antigravity plugin discovery for `~/.gemini/config/plugins`, `~/.gemini/antigravity-cli/plugins`, `<workspace>/.agents/plugins`, and `<workspace>/_agents/plugins`.
- Recognize `plugin.json` as the plugin root marker.
- Accept the stricter CLI manifest schema when validating generated Antigravity CLI plugins: required `name`, optional `description`, optional `$schema`, no additional fields.
- Preserve the 2.0/IDE compatibility note that `name` may be omitted by existing manually created plugins and then defaults to the directory name.
- Extract `skills/*/SKILL.md` into Claudine’s skill linker.
- Treat `mcp_config.json` as MCP catalog input but redact or block secret-bearing `env`, `headers`, and OAuth fields unless explicitly approved.
- Treat `hooks.json` as provider-specific and non-portable by default because event names, matcher tool names, stdin/stdout payloads, and permission decisions are Antigravity-specific.
- Treat `rules/*.md` as provider-specific prompt/rule assets until Claudine has a normalized rules topic.
- Treat `agents/` as unknown provider metadata until Antigravity publishes or local inspection finds a stable schema.
- Do not treat `~/.antigravity/extensions` as agent plugins; those are IDE extensions.

The research implies a Claudine metadata/code update because Antigravity has a distinct plugin container with known paths and enough structure to discover and classify resources, even though full trust and precedence semantics are missing.

## Sources

- [Antigravity 2.0 Plugins](https://antigravity.google/docs/plugins) and underlying Markdown asset: `https://antigravity.google/assets/docs/antigravity-2-0/plugins.md`
- [Antigravity CLI Plugins & Skills](https://antigravity.google/docs/cli/plugins) and underlying Markdown asset: `https://antigravity.google/assets/docs/cli/cli-plugins.md`
- [Antigravity IDE Plugins](https://antigravity.google/docs/ide/plugins) and underlying Markdown asset: `https://antigravity.google/assets/docs/editor/ide-plugins.md`
- [Antigravity Agent Skills](https://antigravity.google/docs/skills) and underlying Markdown asset: `https://antigravity.google/assets/docs/antigravity-2-0/skills.md`
- [Antigravity Hooks](https://antigravity.google/docs/hooks) and underlying Markdown asset: `https://antigravity.google/assets/docs/antigravity-2-0/hooks.md`
- [Antigravity MCP](https://antigravity.google/docs/mcp) and underlying Markdown asset: `https://antigravity.google/assets/docs/antigravity-2-0/mcp.md`
- [Antigravity CLI Permissions](https://antigravity.google/docs/cli/permissions) and underlying Markdown asset: `https://antigravity.google/assets/docs/cli/cli-permissions.md`
- [Antigravity CLI Sandbox](https://antigravity.google/docs/cli/sandbox) and underlying Markdown asset: `https://antigravity.google/assets/docs/cli/cli-sandbox.md`
- [Antigravity CLI Settings](https://antigravity.google/docs/cli/settings) and underlying Markdown asset: `https://antigravity.google/assets/docs/cli/cli-settings.md`
- [Antigravity CLI Reference](https://antigravity.google/docs/cli/reference) and underlying Markdown asset: `https://antigravity.google/assets/docs/cli/cli-reference.md`
- [Antigravity Plugin Manifest JSON Schema](https://antigravity.google/schemas/v1/plugin.json)
- [Claude Code Create plugins](https://code.claude.com/docs/en/plugins)
- [Claude Code Discover and install prebuilt plugins](https://code.claude.com/docs/en/discover-plugins)
- [Claude Code Create and distribute a plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces)
- [Claude Code Plugins reference](https://code.claude.com/docs/en/plugins-reference)
