---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
docs: https://antigravity.google/docs/mcp
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, sse, streamable_http]
  lifecycle: |
    Antigravity connects configured MCP servers when the CLI language-server backend starts and when MCP configuration is reloaded from the interactive MCP surface. Release notes state that MCP server initialization was parallelized in 1.0.4, configurable launch timeout was added in 1.0.7, the default connection timeout was increased to 60 seconds in 1.0.15, and `/mcp`/MCP settings can reload configuration dynamically. Bundled documentation says successful servers are queried for tools, injected into the agent toolset, and routed through the language server for execution.
  notes: |
    No official protocol version date is published. Bundled Antigravity docs describe stdio and SSE, while the public docs/search snippets and 1.0.5 release notes show `url`/`serverUrl` support for direct remote URL configuration; issue and integration evidence indicate `serverUrl` targets remote Streamable HTTP or SSE. Legacy SSE remains documented and tolerated. The bundled docs and binary symbols expose tool, resource, prompt, OAuth, and MCP reload/state surfaces, but list_changed notification handling is not documented.
config_files:
  - os: macos
    scope: user
    path: "~/.gemini/config/mcp_config.json"
    format: json
    notes: "Global shared MCP config. Host probe on 2026-07-08 found `/Users/ken/.claudine/.gemini/config/mcp_config.json` present but empty."
  - os: linux
    scope: user
    path: "~/.gemini/config/mcp_config.json"
    format: json
    notes: "Global shared MCP config."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\config\\mcp_config.json"
    format: json
    notes: "Global shared MCP config."
  - os: macos
    scope: repo
    path: ".agents/mcp_config.json"
    format: json
    notes: "Workspace MCP config documented by official search snippets and customization discovery. Antigravity also discovers `.agent/`, `_agents/`, and `_agent/` customization roots, but MCP-specific docs name `.agents/mcp_config.json`."
  - os: linux
    scope: repo
    path: ".agents/mcp_config.json"
    format: json
    notes: "Workspace MCP config."
  - os: windows
    scope: repo
    path: ".agents\\mcp_config.json"
    format: json
    notes: "Workspace MCP config."
  - os: macos
    scope: plugin
    path: ".agents/plugins/<plugin_name>/mcp_config.json or ~/.gemini/config/plugins/<plugin_name>/mcp_config.json"
    format: json
    notes: "Plugin-scoped MCP servers load only while the plugin is enabled."
  - os: linux
    scope: plugin
    path: ".agents/plugins/<plugin_name>/mcp_config.json or ~/.gemini/config/plugins/<plugin_name>/mcp_config.json"
    format: json
    notes: "Plugin-scoped MCP servers load only while the plugin is enabled."
  - os: windows
    scope: plugin
    path: ".agents\\plugins\\<plugin_name>\\mcp_config.json or %USERPROFILE%\\.gemini\\config\\plugins\\<plugin_name>\\mcp_config.json"
    format: json
    notes: "Plugin-scoped MCP servers load only while the plugin is enabled."
  - os: macos
    scope: user
    path: "~/.gemini/mcp-oauth-tokens-v2.json"
    format: other
    notes: "Observed encrypted/token-like OAuth store on host. Contents were not decoded."
  - os: linux
    scope: user
    path: "~/.gemini/mcp-oauth-tokens-v2.json"
    format: other
    notes: "Remote MCP OAuth token store inferred from host and binary symbols."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\mcp-oauth-tokens-v2.json"
    format: other
    notes: "Remote MCP OAuth token store inferred from host and binary symbols."
cli_params:
  - flag: "/mcp"
    description: "Interactive MCP manager for inspecting active servers/tools, reloading configuration, toggling servers, and initiating OAuth authentication."
  - flag: "Additional Options (...) > MCP Servers"
    description: "Documented UI path for inspecting active MCP servers and their tools."
  - flag: "agy --sandbox"
    description: "Runs the session in sandbox mode; relevant to MCP because subprocess reachability and terminal restrictions can affect configured local servers."
  - flag: "agy --dangerously-skip-permissions"
    description: "Auto-approves tool permission requests, including MCP tool calls that use the common permission surface."
env_vars: []
server_schema:
  transports: ["stdio", "sse", "streamable_http"]
  command_fields: ["command", "args", "env", "disabled", "trust", "timeout", "tools"]
  http_fields: ["serverUrl", "url", "headers", "oauth", "disabled", "trust", "timeout", "tools"]
  env_shape: "`env` is an object mapping variable names to string values for stdio subprocesses."
  auth_shape: "Remote servers can use `serverUrl`/`url`; bundled and binary symbols expose `headers`, `oauth`, `oauth_config`, and OAuth completion/disconnect flows. Google Workspace codelabs show `oauth: { enabled, clientId, clientSecret, scopes }`; host storage uses `~/.gemini/mcp-oauth-tokens-v2.json`."
  notes: "A server definition is the value under `mcpServers.<server_id>`. Bundled docs require `command` for stdio or `serverUrl` for SSE; release notes added `url` compatibility in 1.0.5. Binary schema symbols include `McpServerTools`, with `tools.background` and `tools.eager` fields visible in strings, but per-tool include/exclude keys were not verified."
server_capabilities:
  tools: full
  resources: partial
  prompts: partial
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: "Bundled docs explicitly say MCP servers expose custom tools, resources, and prompts, but the described agent flow only covers tool discovery/injection/execution. Binary symbols expose `ListMcpResources`, `ListMcpPrompts`, `GetMcpPrompt`, and `GetMcpServerStates`; no documented resource subscriptions or list_changed notification handling was found."
client_capabilities:
  roots: unknown
  sampling: unknown
  elicitation: partial
  notes: "Provider docs do not describe MCP client capabilities returned to servers. Binary strings include `client does not support CreateMessage`, suggesting sampling/createMessage is not generally available, and include elicitation/schema-default errors, suggesting some structured user-interaction support exists. `roots/list` behavior was not verified."
tool_surface:
  discovery: "On successful connection, Antigravity queries server tools, injects them into the agent toolset, and lists them in the system prompt. Binary strings include `McpListTools` and prompt text for listing MCP servers and available tools."
  filtering: "Server-level enable/disable is exposed through `/mcp`/ToggleMcpServer and `disabled`-style config. Plugin servers are scoped by plugin enablement. Per-tool include/exclude filtering is not documented; binary schema exposes a `tools` block but not enough to verify filtering semantics."
  approval: "MCP tool calls appear to use the same permission layer as native tools: Antigravity has `/permissions`, `--dangerously-skip-permissions`, request-review/default modes, and release-note fixes for permission integration. No separate MCP approval policy was documented."
  result_handling: "The language server routes `tools/call` results back to the agent. Binary strings include MCP tool conversion errors, max-output truncation messages, and tool cancellation handling, but no MCP-specific prompt-injection scanner was documented."
  annotations_trusted: "Unknown. No source documented use of MCP tool annotations as trusted security metadata."
  notes: "Release notes mention improved resilience to unresponsive MCP servers and Chrome DevTools MCP troubleshooting, but not per-tool result sanitization."
resource_surface:
  supported: true
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: "Application/UI-controlled listing is exposed by language-server APIs (`ListMcpResources`) and docs say servers can expose resources. No evidence showed resources as automatic model-controlled tools."
  notes: "Resource URI schemes, templates, and subscriptions were not documented."
prompt_surface:
  supported: true
  invocation: "Language-server APIs expose `ListMcpPrompts` and `GetMcpPrompt`; UI invocation model is not documented."
  arguments: "Unknown. Binary symbols include `McpPromptArgument`, but no public argument contract was found."
  exposure_model: "Prompts are recognized as MCP server content but not documented as automatic model tools."
  notes: "No evidence found that MCP prompts become slash commands in Antigravity CLI."
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: unknown
  notes: "Claudine can read and write provider-shaped JSON in global, workspace, and plugin `mcp_config.json` files. Antigravity has no non-interactive MCP add/list/remove CLI. Bundled customization docs state workspace customizations override declared configs, global discovery, built-ins, and global declared configs; MCP-specific same-name merge/shadow behavior was not verified."
runtime_injection:
  supported: false
  mechanism: "No MCP-specific one-run flag, inline config flag, or config-path environment variable was found in `agy --help` or docs."
  limitations: "A temporary HOME could redirect `~/.gemini/config/mcp_config.json`, but it also redirects authentication, project cache, settings, plugin state, and OAuth stores, so it is not a safe one-run MCP injection mechanism."
authorization:
  oauth: true
  credential_storage: "MCP OAuth state is stored per user under `~/.gemini/mcp-oauth-tokens-v2.json`; Antigravity CLI account auth uses the system keyring and browser/URL sign-in."
  token_scope: "Remote MCP OAuth scopes are supplied by the server config; Google Workspace examples require per-surface authentication and OAuth redirect `https://antigravity.google/oauth-callback`."
  stdio_secret_delivery: "Stdio secrets should be passed through explicit `env` entries or the child process's own credential store; Antigravity's HTTP OAuth flow is for remote servers, not local subprocesses."
  notes: "Host probe found an encrypted/token-like `mcp-oauth-tokens-v2.json`; Claudine should not parse or rewrite it."
security:
  tool_filtering: "Server enable/disable, plugin enablement, and common permission rules are documented or observed. Per-tool filters are unknown."
  server_trust: "Workspace customizations are project-scoped and project permissions can override global permissions; no MCP-specific repo-trust prompt was documented. `/mcp` can toggle servers."
  env_sanitization: "Stdio servers receive configured `env`; whether they inherit the full user environment is unknown."
  sandbox_interaction: "`agy --sandbox` enables terminal restrictions, and release notes include sandbox hardening. No source showed MCP servers running inside a separate container or being isolated from the host beyond normal subprocess boundaries."
  response_filtering: "No MCP-specific prompt-injection or secret scanner was documented. README warns generally about data exfiltration, prompt injection, and supply-chain risks."
  notes: "Roots, sampling, and elicitation consent are not documented clearly enough to treat them as security boundaries."
gaps:
  - "No official MCP protocol version date is published."
  - "No source documented whether Antigravity handles `notifications/tools/list_changed`, `notifications/resources/list_changed`, or `notifications/prompts/list_changed`."
  - "Resource templates, resource subscriptions, URI schemes, and resource selection UI are not documented."
  - "MCP prompt invocation and prompt argument behavior are not documented."
  - "Client capabilities for roots, sampling, and elicitation are not documented; binary strings only provide weak evidence."
  - "MCP-specific merge behavior for user, workspace, and plugin servers with the same id is not documented."
  - "No one-run MCP runtime injection mechanism was found."
changes: []
requires_claudine_update: true
reason: "Antigravity is a new MCP-capable provider with import/export via shared JSON config but no safe runtime injection; Claudine provider metadata/catalog support would need a new provider entry before sync can target it."
---

# Antigravity MCP Support

## Overview

Antigravity CLI has persistent MCP configuration through shared JSON files, so the strongest Claudine integration path is import/export/sync against provider-shaped config. It does not expose a non-interactive `agy mcp add/list/remove` command or a one-run MCP injection flag, so Claudine should treat file sync as the primary path and avoid runtime injection.

Surface inventory: tools are explicitly discovered and injected into the agent toolset; resources and prompts are recognized by docs and language-server APIs but their UI/model exposure is only partially documented; roots, sampling, and elicitation are mostly undocumented from the server-facing client-capability side.

## Protocol and Transports

Antigravity does not publish an MCP protocol version date. The installed bundled guide describes MCP as an open standard and says configuring MCP servers exposes custom tools, resources, and prompts to the agent.

The bundled guide documents two transport mechanisms:

| Transport | Evidence | Notes |
| --- | --- | --- |
| stdio | `command`, `args`, and `env` in `mcp_config.json` | Local subprocess JSON-RPC over stdin/stdout; the language server spawns the process. |
| SSE | `serverUrl` example ending in `/sse` | Legacy remote SSE remains documented. |
| Streamable HTTP | Official docs snippets and integration examples describe `serverUrl` or `url` for remote HTTP endpoints | Antigravity 1.0.5 added `url` in `mcp_config.json`; docs snippets say `serverUrl` is for remote Streamable HTTP or SSE. |

Session lifecycle is startup-oriented and reloadable. The language-server backend loads MCP config at session start, and the bundled guide says it queries tools, injects them into the toolset, and routes executions through the language server. The changelog says MCP server initialization was parallelized in 1.0.4, launch timeout became configurable in 1.0.7, and the connection timeout was raised to 60 seconds in 1.0.15. `/mcp` and the MCP Servers UI can reload/toggle servers. I found no provider documentation for MCP `list_changed` notifications.

## Configuration

Persistent MCP server definitions live in JSON `mcp_config.json` files:

| OS | Scope | Path | Notes |
| --- | --- | --- | --- |
| macOS | user | `~/.gemini/config/mcp_config.json` | Host probe found this file present but empty under `/Users/ken/.claudine/.gemini/config/mcp_config.json`. |
| Linux | user | `~/.gemini/config/mcp_config.json` | Same shared global path shape. |
| Windows | user | `%USERPROFILE%\.gemini\config\mcp_config.json` | Windows path from official/integration docs. |
| macOS/Linux | repo | `.agents/mcp_config.json` | Official docs snippets name this workspace path. |
| Windows | repo | `.agents\mcp_config.json` | Same workspace file with Windows separators. |
| all | plugin | `plugins/<plugin_name>/mcp_config.json` under a customization root | Loaded only when the plugin is enabled. |

The installed CLI help for `agy` lists no MCP subcommand. MCP management is interactive through `/mcp` and the UI path **Additional Options (...) > MCP Servers**. MCP-affecting launch flags are indirect: `--sandbox` changes tool execution restrictions, and `--dangerously-skip-permissions` auto-approves tool permission requests.

No MCP-specific environment variable was verified. `HOME` changes the `~/.gemini` location, but it also moves unrelated auth/settings/cache state and is not a provider-supported MCP config override.

## Server Definition Shape

A single server definition is the object under `mcpServers.<server_id>`:

```json
{
  "command": "sqlite-mcp-server",
  "args": ["/path/to/database.db"],
  "env": {
    "DB_READONLY": "true"
  }
}
```

For remote servers:

```json
{
  "serverUrl": "https://mcp.mycompany.com/sse"
}
```

Observed and documented native keys include `command`, `args`, `env`, `serverUrl`, and `url`. Binary symbols and integration docs also show `headers`, `oauth`, `disabled`, `trust`, `timeout`, and a `tools` block, but Antigravity's public docs do not fully describe those fields. `env` is an object of string key/value pairs injected into stdio server subprocesses. OAuth credentials are stored separately from server definitions.

## Tools, Resources, and Prompts

Tools are model-controlled. Antigravity queries configured servers for available tools, injects discovered tools into the agent toolset, lists them in the system prompt, and routes calls through the language server. Active servers and tools are inspectable through `/mcp` or **Additional Options (...) > MCP Servers**.

Filtering is server-scoped in verified sources: servers can be enabled/disabled through the MCP UI, and plugin servers only load when the parent plugin is enabled. Per-tool include/exclude filters were not documented. MCP approvals appear to ride the same permission model as native tools: Antigravity has request-review behavior, `/permissions`, `--dangerously-skip-permissions`, and shared permission integration in the changelog. Tool annotations were not documented as trusted.

Resources are application-controlled context by MCP definition. Antigravity's bundled docs say servers can expose resources, and binary symbols expose `ListMcpResources`, but I found no public contract for URI schemes, templates, subscriptions, or who selects resources.

Prompts are user-controlled templates. Bundled docs say servers can expose prompts, and binary symbols expose `ListMcpPrompts`, `GetMcpPrompt`, and `McpPromptArgument`. I found no evidence that MCP prompts are exposed as slash commands or automatic model tools.

## Roots, Sampling, and Elicitation

Provider docs do not describe client capabilities returned to MCP servers.

Roots are unknown. Antigravity has workspace/project resources in `~/.gemini/config/projects/*.json`, and the host project file contains a `file:///Users/ken/.claudine/worktrees/rusty-biscuit/claudine` workspace resource, but I did not verify any `roots/list` MCP handler.

Sampling is likely unsupported or constrained, but not proven. Binary strings include `client does not support CreateMessage`, which is consistent with no general sampling support, but this is only local binary evidence.

Elicitation is partial/unknown. Binary strings include elicitation-related schema-default errors and user-interaction machinery, but no public docs describe MCP elicitation support or consent gates.

## Import, Export, and Sync

Claudine can import Antigravity MCP config by reading JSON `mcp_config.json` files and normalizing `mcpServers`. It can export by writing provider-shaped JSON to the same files. There is no non-interactive provider CLI/API apply path; reload happens inside the running UI via `/mcp`.

The customization guide gives general precedence: workspace project customizations outrank declared workspace configs, global discovery, built-ins, and global declared configs. Plugin MCP servers are scoped to enabled plugins. MCP-specific duplicate server-id merge/shadow behavior was not verified, so Claudine should avoid writing duplicate ids across scopes unless the user explicitly asks.

## Runtime Injection

No safe single-run MCP injection mechanism was found. `agy --help` exposes no MCP config flag, inline config flag, or alternate MCP config path.

The closest alternative is launching with a synthetic `HOME` containing `~/.gemini/config/mcp_config.json`, but that also redirects Antigravity auth, OAuth token storage, project cache, settings, plugins, logs, and built-in state. That is not safe as a one-run MCP overlay.

## Authorization and Credentials

Authorization differs by transport. Stdio servers receive secrets through explicit environment variables or their own credential stores. Remote servers can use `serverUrl`/`url`, static headers, and OAuth-style config. Google Workspace examples use OAuth client id/secret/scopes and redirect through `https://antigravity.google/oauth-callback`.

On this host, `~/.gemini/mcp-oauth-tokens-v2.json` exists as an encrypted/token-like blob. Antigravity CLI account authentication uses the system keyring and browser/URL sign-in. Claudine should not read, parse, or rewrite Antigravity MCP OAuth token stores.

## Security Model

Server trust and filtering are mostly configuration/UI based: `/mcp` can toggle custom servers, plugins scope bundled MCP servers, and permission rules cover tool calls. The default 1.1.0 mode pauses before file writes for review, while `--dangerously-skip-permissions` bypasses tool approval prompts.

Repo/workspace behavior comes through the customization system and project resources. Project-specific permissions under `~/.gemini/config/projects/` can take precedence over global settings. The bundled customization guide documents workspace discovery under `.agents/`, but no MCP-specific repo trust gate was documented.

For subprocesses, Antigravity docs say the language server spawns stdio servers and injects configured `env`. Whether subprocesses inherit the full user environment is unknown. `--sandbox` enables terminal restrictions, and release notes describe sandbox hardening, but I found no evidence that MCP servers run in their own container or that MCP results are scanned for prompt injection. The README explicitly warns about data exfiltration, prompt injection, and supply-chain risks.

Roots, sampling, and elicitation are not documented clearly enough to treat them as security boundaries.

## Mode-Specific Behavior

Interactive TUI mode exposes `/mcp` and the MCP Servers UI. Non-interactive print mode (`agy --print`/`--prompt`) starts the same backend, but it cannot use interactive approval or OAuth flows reliably; local logs show print mode attempting silent auth, then starting browser OAuth and timing out when the user is not logged in.

The CLI and Antigravity app share settings according to the README, and changelog entries say settings and permission paths moved to the shared `~/.gemini/config/` tree. I found no ACP mode or MCP behavior specific to an ACP/server mode in `agy --help`.

## Failure Modes

When an MCP server is slow or unresponsive, Antigravity no longer blocks all MCP startup: the 1.0.4 changelog says initialization was parallelized, and 1.0.15 raised MCP connection timeout to 60 seconds. The 1.0.7 changelog says launch timeout is configurable and can be disabled with `-1`.

The changelog also records fixes for MCP path mismatches, disabling custom MCP servers through the TUI, OAuth token persistence/authentication hangs, and improved resilience to unresponsive MCP servers. Binary strings show errors for uninitialized MCP client sessions, unsupported resources, nil/empty resource content, and tool-output truncation, but exact user-visible failure rendering was not verified.

## Gaps

- Protocol version date is unpublished.
- `list_changed` notification handling is undocumented.
- Resource templates, subscriptions, URI schemes, and selection model are undocumented.
- MCP prompt invocation and argument handling are undocumented.
- Roots, sampling, and elicitation client capabilities are undocumented.
- User/workspace/plugin duplicate server merge semantics are undocumented.
- No safe runtime injection mechanism was found.

## Claudine Integration Notes

Claudine should model Antigravity as a persistent-config sync provider. Import should read `mcpServers` from `~/.gemini/config/mcp_config.json`, `.agents/mcp_config.json`, and enabled plugin `mcp_config.json` files when requested. Export should write JSON in Antigravity's native shape and preserve unknown fields.

Claudine should not use `agy` CLI commands for MCP apply because no non-interactive MCP subcommand exists. It should not implement runtime injection by changing `HOME`, because that mutates too much provider state. It should never read or rewrite `mcp-oauth-tokens-v2.json`; remote credentials should remain provider-managed.

## Sources

- [Google Antigravity MCP documentation](https://antigravity.google/docs/mcp)
- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [google-antigravity/antigravity-cli README](https://github.com/google-antigravity/antigravity-cli/blob/main/README.md)
- [google-antigravity/antigravity-cli CHANGELOG](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- [Google Developer Knowledge MCP server codelab](https://codelabs.developers.google.com/developer-knowledge-mcp-antigravity)
- [Google Workspace MCP servers in Google Antigravity codelab](https://codelabs.developers.google.com/google-workspace-mcp-antigravity)
- [Antigravity CLI issue #60: project-local MCP config behavior](https://github.com/google-antigravity/antigravity-cli/issues/60)
- Observed on host: `agy --help` from `/Users/ken/.local/bin/agy`, version 1.1.0 logs under `/Users/ken/.claudine/.gemini/antigravity-cli/log/`.
- Observed on host: bundled provider docs at `/Users/ken/.claudine/.gemini/antigravity-cli/builtin/skills/agy-customizations/docs/mcp_servers.md`.
- Observed on host: actual config files `/Users/ken/.claudine/.gemini/config/mcp_config.json`, `/Users/ken/.claudine/.gemini/mcp-oauth-tokens-v2.json`, and `/Users/ken/.claudine/.gemini/config/projects/*.json`.
