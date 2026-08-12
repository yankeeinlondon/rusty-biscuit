---
$schema: ./_schema.yaml
created: 2025-04-13
last_updated: 2026-07-03
agent: codex
model: default
docs: https://www.geminicli.com/docs/tools/mcp-server/
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, sse, streamable_http]
  lifecycle: |
    Gemini CLI connects configured MCP servers during startup or explicit reload, tracks CONNECTING/CONNECTED/DISCONNECTED state, and can restart all or one server through its MCP client manager. Source inspection of v0.46.0 shows it registers server notification handlers for tools, resources, and prompts list changes, refreshes the relevant registry, coalesces overlapping refreshes, and sends roots/list_changed notifications when workspace directories change. Failed servers are marked disconnected and can be retried through /mcp reload, mcp manager restart paths, or config/extension reload.
  notes: |
    The public docs do not state a protocol version date. The bundled MCP SDK contains schemas through roots, sampling/createMessage, and elicitation/create, while Gemini CLI's active client registers only roots as a client capability. Transport is inferred from `httpUrl`, `url`, or `command`; `type` can be `stdio`, `sse`, or `http`. The settings schema contains a `tcp` field, but neither docs nor active transport selection use it as a supported provider transport.
config_files:
  - os: macos
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    notes: "Top-level `mcpServers` and `mcp` policy. Local host probe on 2026-07-03 found no `mcpServers` key in `/Users/ken/.gemini/settings.json`."
  - os: linux
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    notes: "Top-level `mcpServers` and `mcp` policy."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\settings.json"
    format: json
    notes: "Top-level `mcpServers` and `mcp` policy."
  - os: macos
    scope: repo
    path: ".gemini/settings.json"
    format: json
    notes: "Project-scope settings. Project MCP servers are gated by folder trust."
  - os: linux
    scope: repo
    path: ".gemini/settings.json"
    format: json
    notes: "Project-scope settings. Project MCP servers are gated by folder trust."
  - os: windows
    scope: repo
    path: ".gemini\\settings.json"
    format: json
    notes: "Project-scope settings. Project MCP servers are gated by folder trust."
  - os: macos
    scope: local
    path: "~/.gemini/config/mcp_config.json"
    format: json
    notes: "Standard MCP-client or remote-skill MCP config file referenced by the official MCP docs. Local host probe found the file present but empty."
  - os: linux
    scope: local
    path: "~/.gemini/config/mcp_config.json"
    format: json
    notes: "Standard MCP-client or remote-skill MCP config file referenced by the official MCP docs."
  - os: windows
    scope: local
    path: "%USERPROFILE%\\.gemini\\config\\mcp_config.json"
    format: json
    notes: "Standard MCP-client or remote-skill MCP config file referenced by the official MCP docs."
  - os: macos
    scope: user
    path: "~/.gemini/mcp-server-enablement.json"
    format: json
    notes: "Per-server enabled/disabled state; local host probe found it missing, meaning no persisted enablement overrides."
  - os: linux
    scope: user
    path: "~/.gemini/mcp-server-enablement.json"
    format: json
    notes: "Per-server enabled/disabled state."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\mcp-server-enablement.json"
    format: json
    notes: "Per-server enabled/disabled state."
  - os: macos
    scope: user
    path: "~/.gemini/mcp-oauth-tokens.json"
    format: json
    notes: "Remote MCP OAuth token store; local host probe found it missing."
  - os: linux
    scope: user
    path: "~/.gemini/mcp-oauth-tokens.json"
    format: json
    notes: "Remote MCP OAuth token store."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\mcp-oauth-tokens.json"
    format: json
    notes: "Remote MCP OAuth token store."
  - os: macos
    scope: user
    path: "~/.gemini/trustedFolders.json"
    format: json
    notes: "Folder-trust decisions; local host probe found five entries."
  - os: linux
    scope: user
    path: "~/.gemini/trustedFolders.json"
    format: json
    notes: "Folder-trust decisions."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.gemini\\trustedFolders.json"
    format: json
    notes: "Folder-trust decisions."
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/system-defaults.json"
    format: json
    notes: "System baseline defaults; lower precedence than user and repo settings."
  - os: linux
    scope: system
    path: "/etc/gemini-cli/system-defaults.json"
    format: json
    notes: "System baseline defaults; lower precedence than user and repo settings."
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\system-defaults.json"
    format: json
    notes: "System baseline defaults; lower precedence than user and repo settings."
  - os: macos
    scope: managed
    path: "/Library/Application Support/GeminiCli/settings.json"
    format: json
    notes: "System override settings; highest precedence among local settings files."
  - os: linux
    scope: managed
    path: "/etc/gemini-cli/settings.json"
    format: json
    notes: "System override settings; highest precedence among local settings files."
  - os: windows
    scope: managed
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    format: json
    notes: "System override settings; highest precedence among local settings files."
  - os: macos
    scope: plugin
    path: "<extension-root>/gemini-extension.json"
    format: json
    notes: "Extensions can bundle `mcpServers`; all MCP server options are supported except `cwd`."
  - os: linux
    scope: plugin
    path: "<extension-root>/gemini-extension.json"
    format: json
    notes: "Extensions can bundle `mcpServers`; all MCP server options are supported except `cwd`."
  - os: windows
    scope: plugin
    path: "<extension-root>\\gemini-extension.json"
    format: json
    notes: "Extensions can bundle `mcpServers`; all MCP server options are supported except `cwd`."
cli_params:
  - flag: "gemini mcp add [options] <name> <commandOrUrl> [args...]"
    description: "Adds a persistent MCP server to user or project config; `--scope user|project` chooses destination and `--transport/--type stdio|sse|http` chooses transport."
    example: "gemini mcp add --scope user --transport http api https://example.com/mcp"
  - flag: "gemini mcp remove <name> --scope user|project"
    description: "Removes a persistent MCP server from the selected settings file."
  - flag: "gemini mcp list"
    description: "Lists configured MCP servers and status."
  - flag: "gemini mcp enable <name> [--session]"
    description: "Enables a server; `--session` clears a session-only disable without editing persistent config."
  - flag: "gemini mcp disable <name> [--session]"
    description: "Disables a server; `--session` applies only to the current process."
  - flag: "--allowed-mcp-server-names"
    description: "Comma-separated or repeated server-name allowlist for the current session; it overrides configured MCP allow/exclude lists for connection filtering."
    example: "gemini --allowed-mcp-server-names github,docs -p \"summarize\""
  - flag: "--skip-trust"
    description: "Bypasses the folder-trust prompt for the current session, allowing project MCP config in headless or automated runs."
  - flag: "--acp"
    description: "Starts ACP mode; MCP behavior is separate from persistent settings and is driven by the ACP client/server path."
  - flag: "/mcp auth [server-name]"
    description: "Interactive OAuth authentication for remote MCP servers."
  - flag: "/mcp reload"
    description: "Restarts MCP clients and re-discovers tools, prompts, and resources."
  - flag: "/mcp list | /mcp desc | /mcp schema"
    description: "Interactive MCP inspection commands for servers, descriptions, and schemas."
env_vars:
  - name: GEMINI_CLI_HOME
    effect: "Relocates Gemini CLI user state root; Gemini stores `.gemini` under this directory. This enables one-run MCP injection by pointing a process at a temporary home containing generated config."
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: "Overrides the managed/system settings file path."
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the system defaults file path."
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: "Overrides the trusted-folders file path used by folder trust."
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: "When `true`, trusts the current workspace for the session and allows project MCP servers to connect."
  - name: GEMINI_SANDBOX
    effect: "Enables Gemini CLI tool sandboxing. It changes overall execution posture and can affect MCP server reachability, but docs warn MCP servers are separate processes and must be sandbox-compatible."
  - name: SANDBOX_MOUNTS
    effect: "Extra host mounts for sandboxed Gemini CLI execution; relevant when MCP servers need files reachable from sandboxed built-in tools."
  - name: SANDBOX_FLAGS
    effect: "Additional flags for sandbox command setup; relevant to MCP only through sandbox compatibility."
server_schema:
  transports: ["stdio", "sse", "streamable_http"]
  command_fields: ["command", "args", "env", "cwd", "timeout", "trust", "description", "includeTools", "excludeTools", "type", "extension"]
  http_fields: ["url", "httpUrl", "headers", "timeout", "trust", "description", "includeTools", "excludeTools", "oauth", "authProviderType", "targetAudience", "targetServiceAccount", "type"]
  env_shape: "`env` is an object mapping variable names to string values. Values support `$VAR`, `${VAR}`, and Windows `%VAR%`; undefined variables resolve to an empty string. The docs describe expansion primarily for the `env` block."
  auth_shape: "Remote `url` and `httpUrl` servers can use static `headers`, OAuth dynamic discovery or explicit `oauth` settings, `authProviderType: google_credentials`, or `authProviderType: service_account_impersonation` with `targetAudience` and `targetServiceAccount`. OAuth tokens are stored in `~/.gemini/mcp-oauth-tokens.json`."
  notes: "A server definition is the value under `mcpServers.<name>`. One of `command`, `url`, or `httpUrl` is required for active connection. `type` values are `stdio`, `sse`, and `http`. `excludeTools` takes precedence over `includeTools`. The schema contains `tcp`, but active transport docs and source do not expose it as supported."
server_capabilities:
  tools: full
  resources: partial
  prompts: full
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: true
  prompt_list_changed: true
  notes: "Tools are model-controlled and registered as `mcp_{serverName}_{toolName}`. Resources are discovered, listed in `/mcp`, available through `@server://...`, and exposed through built-in `list_mcp_resources` and `read_mcp_resource` tools; resource templates and subscriptions are not implemented in the documented surface. Prompts are registered in the prompt registry and exposed as slash commands. Source inspection shows list-changed handlers for tools, resources, and prompts."
client_capabilities:
  roots: full
  sampling: none
  elicitation: none
  notes: "Gemini CLI v0.46.0 registers MCP client capabilities with `roots: { listChanged: true }`, answers `roots/list` from workspace directories, and emits `notifications/roots/list_changed` on directory changes. The MCP SDK includes sampling and elicitation schemas, but active Gemini MCP client setup does not register those capabilities or handlers."
tool_surface:
  discovery: "On startup, `/mcp reload`, extension load, or MCP manager restart, Gemini connects each allowed/trusted server, calls `tools/list`, transforms nullable schema shapes for Gemini API compatibility, and registers tools as FQNs."
  filtering: "Per-server `includeTools`/`excludeTools`, top-level `mcp.allowed` and `mcp.excluded`, session `--allowed-mcp-server-names`, disabled-server state, extension activation, folder trust, and Policy Engine rules using `mcpName` or FQN/wildcards."
  approval: "Server `trust: true` bypasses prompts. Otherwise MCP tools use the same policy/confirmation pipeline as native tools; non-interactive mode cannot ask and treats required confirmation as denial. Read-only annotations are passed into policy metadata but should not be treated as a security boundary."
  result_handling: "MCP tool calls use `tools/call` with a progress token. Success and error responses are returned as function responses for the model; errors are wrapped as `isError: true`. The CLI can truncate/mask general tool output through its context-management settings, but no MCP-specific prompt-injection scanner is documented."
  annotations_trusted: "Tool annotations are ingested, including `readOnlyHint`, and policy code can match annotation metadata. They come from the server and should be treated as hints, not trusted proof."
  notes: "Server names should avoid underscores because FQN policy parsing splits after the `mcp_` prefix."
resource_surface:
  supported: true
  uri_schemes: ["server-qualified MCP resource URIs such as @server://resource/path"]
  templates: false
  subscriptions: false
  exposure_model: "Application-controlled context. Users can reference MCP resources with `@server://...`; the model can discover/read resources through built-in `list_mcp_resources` and `read_mcp_resource` tools."
  notes: "Source lists resources through paginated `resources/list` and reads exact URIs through `resources/read`; resource templates and subscribe/unsubscribe are not described in docs or active source paths."
prompt_surface:
  supported: true
  invocation: "Discovered MCP prompts are registered as slash commands."
  arguments: "Arguments are accepted as named flags or positional command arguments, converted to strings, and sent to `prompts/get`."
  exposure_model: "User-controlled templates. The user invokes a slash command; prompts are not automatic model tools."
  notes: "Prompt list changes can refresh dynamically through `notifications/prompts/list_changed` or manually with `/mcp reload`."
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: shallow
  notes: "Claudine can import from and export to user/project `settings.json` and should also recognize `~/.gemini/config/mcp_config.json` for standard MCP-client/remote-skill config. `gemini mcp add/remove/enable/disable` is the supported apply path for normal settings. Gemini's settings schema marks `mcpServers` as shallow-merge across settings layers; the MCP manager then securely merges same-name extension/settings definitions with includeTools intersection, excludeTools union, and env object merge. Managed admin remote settings can inject allowed and required MCP config."
runtime_injection:
  supported: true
  mechanism: "Set `GEMINI_CLI_HOME` to a temporary directory containing generated `.gemini/settings.json` and launch `gemini` for that run; optionally combine with `--allowed-mcp-server-names` and `GEMINI_CLI_TRUST_WORKSPACE=true` or `--skip-trust`."
  limitations: "This relocates all Gemini CLI state for the process, so OAuth tokens, enablement state, trusted folders, auth state, and other sidecars must be copied or deliberately omitted. There is no narrow `--mcp-config` flag for a single injected server. OAuth browser flows remain unsuitable for headless one-run injection."
authorization:
  oauth: true
  credential_storage: "Remote MCP OAuth tokens are stored in `~/.gemini/mcp-oauth-tokens.json`; Google credentials and service-account impersonation use Application Default Credentials. Static headers live in config."
  token_scope: "Token records are keyed per MCP server name and use discovered or configured client/audience/scope data; service account impersonation binds to target audience and target service account."
  stdio_secret_delivery: "Stdio secrets should be passed via explicit per-server `env` values, usually with environment-variable expansion. Gemini redacts sensitive inherited host environment variables unless explicitly configured."
  notes: "OAuth requires a browser and localhost redirect listener. Claudine can avoid reading or writing secrets by preserving opaque sidecar files and generating env references rather than literal secret values."
security:
  tool_filtering: "Per-server tool include/exclude, server allow/exclude lists, session allowlist, enablement state, extension activity, and Policy Engine allow/deny/ask rules."
  server_trust: "Folder trust gates project MCP servers; `--skip-trust` and `GEMINI_CLI_TRUST_WORKSPACE=true` bypass for a session. Admin controls can disable MCP, allowlist MCP config, or inject required remote MCP servers."
  env_sanitization: "MCP stdio subprocesses inherit a sanitized environment with sensitive patterns removed; explicit `env` entries are trusted and passed through."
  sandbox_interaction: "MCP servers run as separate local processes or remote transports. Gemini docs call out sandbox compatibility problems; sandboxing is not documented as a containment boundary around stdio MCP subprocesses."
  response_filtering: "No MCP-specific prompt-injection filter is documented. General tool output truncation/masking may apply before context insertion, but Claudine should treat all MCP responses as untrusted."
  notes: "Roots are advertised to servers and reflect workspace directories, but roots are not a filesystem sandbox. Sampling and elicitation are not offered, so no user-consent gate for those MCP client capabilities is present."
gaps:
  - "The public docs do not state the MCP protocol version date implemented by Gemini CLI."
  - "The settings schema includes `tcp`, but no official doc or active transport branch proves TCP/WebSocket support."
  - "Resource templates and resource subscriptions are not documented or observed in active v0.46.0 source paths."
  - "The precise maximum MCP tool-result size and interaction with general truncation/masking settings is not documented as MCP-specific behavior."
  - "ACP-mode MCP exposure is under-documented in the public Gemini CLI docs; source/docs distinguish ACP mode but not a complete MCP import/export surface for Claudine."
  - "Admin remote settings can inject MCP config, but the external distribution mechanism and local persistence are not fully documented in public sources."
changes:
  - "Corrected runtime injection from unsupported shadow-HOME emulation to supported `GEMINI_CLI_HOME` one-run state relocation."
  - "Reclassified client roots support from none to full for `roots/list` plus `roots/list_changed` based on v0.46.0 source inspection."
  - "Updated dynamic notification support: tools, resources, and prompts list-changed notifications are handled and refresh registries."
  - "Added `~/.gemini/config/mcp_config.json` based on bundled docs and local host probe."
  - "Replaced invalid `os: all` records with per-OS config file records."
  - "Recorded local host probes: real user settings contain no `mcpServers`, `mcp_config.json` is empty, enablement and MCP OAuth token files are absent, and trusted folders exist."
requires_claudine_update: true
reason: |
  Claudine's Gemini MCP metadata should be updated to support native one-run injection through `GEMINI_CLI_HOME`, client roots capability, dynamic list_changed refresh flags, and the additional `~/.gemini/config/mcp_config.json` import surface. Existing generated metadata that says runtime injection is unsupported or roots/list_changed are absent would now be stale.
---

# Gemini CLI MCP Support

## Overview

Gemini CLI has first-class MCP support through persistent JSON settings, `gemini mcp` management commands, interactive `/mcp` commands, extension-bundled servers, managed/admin config, and one-run state relocation through `GEMINI_CLI_HOME`. For Claudine, the strongest integration path is `import_sync`: read/write Gemini-shaped config, prefer `gemini mcp` commands where practical, and use `GEMINI_CLI_HOME` only for wrapper-time injection.

Surface inventory: tools and prompts are exposed strongly; resources are exposed through user `@server://...` references and built-in resource tools; roots are implemented as a client capability; sampling and elicitation are not offered by Gemini CLI's active MCP client setup.

Local inspection on 2026-07-03 used Gemini CLI `0.46.0`. The real `/Users/ken/.gemini/settings.json` has no `mcpServers` key, `/Users/ken/.gemini/config/mcp_config.json` exists but is empty, `/Users/ken/.gemini/mcp-server-enablement.json` and `/Users/ken/.gemini/mcp-oauth-tokens.json` are absent, and `/Users/ken/.gemini/trustedFolders.json` exists.

## Protocol and Transports

Gemini CLI documents three MCP transports:

| Transport | Config keys | Notes |
| --- | --- | --- |
| stdio | `command`, `args`, `env`, `cwd` | Local subprocess JSON-RPC over stdin/stdout. |
| SSE | `url`, optional `type: "sse"` | Legacy remote Server-Sent Events transport. |
| Streamable HTTP | `httpUrl`, optional `type: "http"` | Modern remote HTTP streaming transport. |

Transport selection is inferred from `httpUrl`, then `url`, then `command`; `type` can make the intent explicit. The settings schema includes `tcp`, but the public docs and active source paths do not show it as a supported transport.

The docs do not state a protocol version date. Source inspection shows the bundled MCP SDK contains schemas for modern client/server features including roots, sampling, and elicitation, but Gemini CLI registers only roots as an active client capability.

Session lifecycle:

- Configured servers connect at startup when MCP is enabled and the folder is trusted.
- `/mcp reload` and MCP manager restart paths disconnect and rediscover servers.
- Extension load/unload can start or stop extension-owned MCP servers.
- Failed connections are marked disconnected and diagnostics are deduplicated unless the user explicitly opens MCP status.
- Gemini listens for `notifications/tools/list_changed`, `notifications/resources/list_changed`, and `notifications/prompts/list_changed`, then refreshes the corresponding registry.
- Gemini advertises `roots.listChanged` and emits `notifications/roots/list_changed` when workspace directories change.

## Configuration

Persistent MCP config lives primarily in JSON settings files:

| OS | Scope | Path | Notes |
| --- | --- | --- | --- |
| macOS | user | `~/.gemini/settings.json` | Top-level `mcpServers` and `mcp`. |
| Linux | user | `~/.gemini/settings.json` | Same shape. |
| Windows | user | `%USERPROFILE%\.gemini\settings.json` | Same shape. |
| macOS | repo | `.gemini/settings.json` | Project scope; folder-trust gated. |
| Linux | repo | `.gemini/settings.json` | Project scope; folder-trust gated. |
| Windows | repo | `.gemini\settings.json` | Project scope; folder-trust gated. |
| macOS | local | `~/.gemini/config/mcp_config.json` | Standard MCP-client or remote-skill MCP config. |
| Linux | local | `~/.gemini/config/mcp_config.json` | Same shape. |
| Windows | local | `%USERPROFILE%\.gemini\config\mcp_config.json` | Same shape. |
| macOS | system defaults | `/Library/Application Support/GeminiCli/system-defaults.json` | Low precedence. |
| Linux | system defaults | `/etc/gemini-cli/system-defaults.json` | Low precedence. |
| Windows | system defaults | `C:\ProgramData\gemini-cli\system-defaults.json` | Low precedence. |
| macOS | managed | `/Library/Application Support/GeminiCli/settings.json` | Highest local settings precedence. |
| Linux | managed | `/etc/gemini-cli/settings.json` | Highest local settings precedence. |
| Windows | managed | `C:\ProgramData\gemini-cli\settings.json` | Highest local settings precedence. |
| all | plugin | `<extension-root>/gemini-extension.json` | Extension `mcpServers`; all options except `cwd`. |

Sidecars:

- `~/.gemini/mcp-server-enablement.json` stores per-server enabled/disabled state.
- `~/.gemini/mcp-oauth-tokens.json` stores remote MCP OAuth tokens.
- `~/.gemini/trustedFolders.json` stores folder-trust decisions.

MCP-specific commands and switches:

- `gemini mcp add [options] <name> <commandOrUrl> [args...]`
- `gemini mcp remove <name> --scope user|project`
- `gemini mcp list`
- `gemini mcp enable <name> [--session]`
- `gemini mcp disable <name> [--session]`
- `--allowed-mcp-server-names`
- `--skip-trust`
- `/mcp auth [server-name]`
- `/mcp reload`
- `/mcp list`, `/mcp desc`, `/mcp schema`

MCP-affecting environment variables:

- `GEMINI_CLI_HOME`
- `GEMINI_CLI_SYSTEM_SETTINGS_PATH`
- `GEMINI_CLI_SYSTEM_DEFAULTS_PATH`
- `GEMINI_CLI_TRUSTED_FOLDERS_PATH`
- `GEMINI_CLI_TRUST_WORKSPACE`
- `GEMINI_SANDBOX`, `SANDBOX_MOUNTS`, `SANDBOX_FLAGS` for sandbox posture and compatibility.

## Server Definition Shape

A single server definition is the object under `mcpServers.<serverName>`:

```json
{
  "command": "node",
  "args": ["server.js"],
  "env": {
    "API_KEY": "$API_KEY"
  },
  "cwd": "./mcp",
  "timeout": 30000,
  "trust": false,
  "includeTools": ["search"],
  "excludeTools": ["delete"],
  "description": "Example stdio server"
}
```

Remote examples use either `url` for SSE or `httpUrl` for Streamable HTTP:

```json
{
  "httpUrl": "https://example.com/mcp",
  "headers": {
    "Authorization": "Bearer ${MCP_TOKEN}"
  },
  "oauth": {
    "enabled": true,
    "scopes": ["read"]
  },
  "authProviderType": "dynamic_discovery",
  "timeout": 30000,
  "trust": false
}
```

Required active fields: one of `command`, `url`, or `httpUrl`. Optional fields include `type`, `args`, `headers`, `env`, `cwd`, `timeout`, `trust`, `description`, `includeTools`, `excludeTools`, `oauth`, `authProviderType`, `targetAudience`, `targetServiceAccount`, and extension metadata. `excludeTools` wins over `includeTools`.

Environment values support `$VAR`, `${VAR}`, and Windows `%VAR%`; undefined variables resolve to an empty string. The docs recommend env references instead of hardcoded secrets.

## Tools, Resources, and Prompts

Tools are model-controlled. Gemini calls `tools/list`, filters tools, sanitizes Gemini API schema details, and registers each tool as:

```text
mcp_{serverName}_{toolName}
```

Filtering happens before model exposure through:

- `includeTools` and `excludeTools`.
- `mcp.allowed` and `mcp.excluded`.
- `--allowed-mcp-server-names`.
- Per-server enablement state.
- Folder trust and extension activation.
- Policy Engine rules using `mcpName` or FQN/wildcard matching.

Approval uses the same confirmation and policy path as native tools. `trust: true` bypasses confirmation for that server. In non-interactive mode, a tool that needs confirmation cannot ask the user and is denied. Tool annotations, including `readOnlyHint`, are passed into metadata and can inform policy, but they originate from the server and should be treated as hints.

Tool results are sent back as function responses. Server-side call errors are wrapped with `isError: true`. The source passes MCP progress tokens and maps progress notifications to Gemini events. No MCP-specific prompt-injection scanner is documented.

Resources are application-controlled context. Gemini calls paginated `resources/list`, registers resources, displays them in `/mcp`, offers them in `@` completion, and reads exact URIs with `resources/read`. Users can write `@server://resource/path`; the model can also call built-in `list_mcp_resources` and `read_mcp_resource`. Resource templates and subscriptions are not documented or observed.

Prompts are user-controlled templates. Gemini calls `prompts/list`, registers prompts as slash commands, and invokes them through `prompts/get`. Arguments are converted to strings and passed by name. Prompts are not automatic model tools.

Gemini handles `tools/list_changed`, `resources/list_changed`, and `prompts/list_changed` notifications and refreshes the affected registry.

## Roots, Sampling, and Elicitation

Roots are supported. Gemini registers:

```json
{
  "roots": {
    "listChanged": true
  }
}
```

It answers `roots/list` from the current workspace context directories, returning file URLs with basename labels. It emits `notifications/roots/list_changed` when the workspace directory set changes.

Sampling is not supported in the active Gemini MCP client. The SDK schema is bundled, but the client does not register `sampling` capability or a `sampling/createMessage` handler.

Elicitation is not supported in the active Gemini MCP client. The SDK schema is bundled, but the client does not register `elicitation` capability or a handler. Because sampling and elicitation are absent, there is no Gemini consent prompt for those MCP client capabilities.

## Import, Export, and Sync

Claudine can import Gemini MCP config from:

- `~/.gemini/settings.json`
- project `.gemini/settings.json`
- `~/.gemini/config/mcp_config.json` where relevant
- extension manifests for read-only catalog awareness

Claudine can export provider-shaped config to user or project settings. For normal apply operations, prefer `gemini mcp add`, `gemini mcp remove`, `gemini mcp enable`, and `gemini mcp disable` where they cover the desired mutation. Direct JSON writes are still needed for fields the command surface does not expose.

Gemini settings mark `mcpServers` as shallow-merge across settings layers. Higher-precedence same-name server definitions replace scalar fields. At runtime, the MCP manager has additional secure merge behavior for extension/settings overlaps: `includeTools` is intersected, `excludeTools` is unioned, and `env` objects are merged.

Admin and managed settings can disable MCP, provide allowlisted config, or inject required remote MCP servers.

## Runtime Injection

Gemini has a native state-root override:

```bash
GEMINI_CLI_HOME=/tmp/claudine-gemini-home gemini --allowed-mcp-server-names docs -p "..."
```

For one-run injection, Claudine can create a temporary directory, write `.gemini/settings.json`, set `GEMINI_CLI_HOME`, and launch Gemini. Add `GEMINI_CLI_TRUST_WORKSPACE=true` or `--skip-trust` when project config must be trusted in a non-interactive run.

Limitations:

- The override relocates all Gemini state, not only MCP config.
- OAuth tokens, enablement state, trusted folders, auth state, and other sidecars must be copied or intentionally omitted.
- There is no narrow `--mcp-config` flag for a single run.
- Browser OAuth is still not headless-safe.

## Authorization and Credentials

Remote MCP authorization differs by transport:

| Pattern | Config | Storage |
| --- | --- | --- |
| Static header | `headers` | Provider config file. |
| OAuth dynamic discovery | `oauth.enabled` or 401-triggered discovery | `~/.gemini/mcp-oauth-tokens.json`. |
| OAuth explicit client | `oauth.clientId`, `oauth.clientSecret`, endpoints, scopes | Token sidecar plus config. |
| Google ADC | `authProviderType: "google_credentials"` | Application Default Credentials. |
| Service account impersonation | `authProviderType: "service_account_impersonation"`, `targetAudience`, `targetServiceAccount` | Application Default Credentials and generated token. |

OAuth opens a browser and receives a localhost callback. It is not suitable for headless runs unless already authenticated and sidecars are available.

Stdio servers should receive secrets through explicit `env` entries and environment expansion. Gemini sanitizes inherited environment variables matching sensitive patterns, while explicit `env` entries are treated as user consent to share those values.

## Security Model

Gemini's MCP security posture is layered:

- Server filtering: `mcp.allowed`, `mcp.excluded`, session `--allowed-mcp-server-names`, enablement state, extension activation, and admin settings.
- Tool filtering: `includeTools`, `excludeTools`, and Policy Engine rules.
- Trust gates: project MCP config does not connect unless the folder is trusted or trust is bypassed.
- Approval: MCP tool calls ride the same permission model as native tools; `trust: true` bypasses prompts for a server.
- Environment: stdio MCP subprocesses receive sanitized inherited env plus explicit `env`.
- Admin controls: managed settings can disable MCP, define allowed config, or require remote MCP servers.
- Roots: servers can ask for workspace roots, but roots do not sandbox file access by themselves.
- Sandbox: Gemini's tool sandbox is not documented as containing stdio MCP subprocesses. Docs warn MCP servers are separate processes and may fail when sandboxing changes the environment.
- Responses: no MCP-specific prompt-injection filtering is documented; tool results should be treated as untrusted content.

## Mode-Specific Behavior

Interactive mode exposes `/mcp` commands, OAuth auth flows, status/details/schema inspection, enable/disable commands, and reload.

Non-interactive mode (`-p` or `--prompt`) can use configured MCP servers. It cannot complete OAuth browser flows or interactive confirmations; approvals that require asking the user are denied. Use `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` for trusted project config in automation.

ACP mode is a separate provider mode. The CLI exposes `--acp`, but public docs do not describe a complete Claudine-style import/export surface for MCP inside ACP mode. Treat ACP MCP behavior as separate from persistent `settings.json` sync.

IDE integration uses MCP over HTTP for the IDE companion path. That is an integration channel, not the normal user `mcpServers` catalog.

Extension mode can add and remove extension-owned MCP servers dynamically. If a user/project setting defines the same server name as an extension, Gemini merges/overrides according to source precedence and extension ownership rules.

## Failure Modes

| Failure | Observed or documented behavior |
| --- | --- |
| Missing active fields | Server is skipped because no `command`, `url`, or `httpUrl` is present. |
| Connection failure | Status becomes `DISCONNECTED`; diagnostic is logged and usually surfaced as a hint unless the user is interacting with `/mcp`. |
| 401 remote auth | Gemini attempts OAuth discovery/flow when possible; otherwise it asks for `/mcp auth`. |
| No prompts or tools | Legacy discovery path treats a server with no prompts or tools as an error; manager/resource-aware paths still maintain resource registries. |
| `tools/list` method missing | Treated as no tools rather than fatal. |
| `resources/list` or `prompts/list` missing | Treated as absent where method-not-found is detected. |
| Tool call error | Returned to the model as an MCP function response with `isError: true`. |
| Tool timeout | Uses per-server `timeout`, default 600,000 ms. |
| Stdio stderr | Captured and logged; docs say INFO messages are filtered. |
| List-change burst | Refreshes are coalesced; if a refresh is already running, Gemini marks a pending refresh and loops once complete. |
| Untrusted folder | Configured project MCP servers do not connect. |

## Gaps

- The public docs do not state Gemini CLI's MCP protocol version date.
- `tcp` exists in the settings schema, but no official docs or active transport branch confirm it as supported.
- Resource templates and subscriptions are not documented or observed in active source paths.
- MCP-specific maximum output size and exact interaction with general truncation/masking are not documented.
- ACP-mode MCP behavior is under-documented for Claudine import/export/sync.
- Managed remote admin MCP settings are visible in source, but the public docs do not fully describe local persistence or distribution mechanics.

## Claudine Integration Notes

Claudine should import and export Gemini's native camelCase server fields exactly: `includeTools`, `excludeTools`, `httpUrl`, `authProviderType`, `targetAudience`, and `targetServiceAccount`. Do not generate hyphenated field names.

For runtime wrappers, prefer `GEMINI_CLI_HOME` with a temporary state root over mutating real user or repo config. Preserve or copy OAuth and enablement sidecars only when explicitly needed. Avoid reading token contents unless the user asks for credential migration.

Catalog metadata should mark tools, prompts, roots, and list_changed notifications as supported; mark resource templates, resource subscriptions, sampling, and elicitation as unsupported or unknown as described above.

Treat MCP tool results as untrusted. Gemini's `readOnlyHint` and other annotations are useful for policy UX, but Claudine should not treat them as proof of safety.

## Changelog

- 2026-07-03: Refreshed against Gemini CLI `0.46.0`, official docs, bundled docs/source, `gemini mcp --help`, and local `/Users/ken/.gemini` probes. Corrected runtime injection, roots, list_changed notifications, `mcp_config.json`, per-OS config records, and host-observed empty/missing MCP sidecars.

## Sources

- [Gemini CLI MCP server docs](https://www.geminicli.com/docs/tools/mcp-server/)
- [Gemini CLI configuration docs](https://www.geminicli.com/docs/reference/configuration/)
- [Gemini CLI commands reference](https://www.geminicli.com/docs/reference/commands/)
- [Gemini CLI policy engine docs](https://www.geminicli.com/docs/reference/policy-engine/)
- [Gemini CLI extensions reference](https://www.geminicli.com/docs/extensions/reference/)
- [Gemini CLI enterprise controls](https://www.geminicli.com/docs/admin/enterprise-controls/)
- [Gemini CLI trusted folders docs](https://www.geminicli.com/docs/cli/trusted-folders/)
- [Gemini CLI repository](https://github.com/google-gemini/gemini-cli)
- Observed on host: `gemini --version` returned `0.46.0`; `gemini mcp --help`, `gemini mcp add --help`, `gemini mcp list --help`, `gemini mcp remove --help`, `gemini mcp enable --help`, and `gemini mcp disable --help`.
- Observed on host: `/Users/ken/.gemini/settings.json`, `/Users/ken/.gemini/config/mcp_config.json`, `/Users/ken/.gemini/trustedFolders.json`, and missing `/Users/ken/.gemini/mcp-server-enablement.json` plus `/Users/ken/.gemini/mcp-oauth-tokens.json`.
- Observed in installed package: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/docs/tools/mcp-server.md`.
- Observed in installed package: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/docs/tools/mcp-resources.md`.
- Observed in installed package: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/docs/reference/configuration.md`.
- Observed in installed package: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-G33JEOEV.js` for roots, notification handlers, discovery, OAuth storage, resources, prompts, tool calls, and MCP manager behavior.
- Observed in installed package: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-MKQJU6N7.js` for settings schema and merge strategy.
- Observed in installed package: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/gemini-LOO67E54.js` for `--allowed-mcp-server-names`, trust, and sandbox-related CLI behavior.
