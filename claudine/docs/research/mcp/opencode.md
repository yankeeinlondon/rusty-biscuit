---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://opencode.ai/docs/mcp-servers/
support: runtime_injection
protocol:
  versions: ["unknown"]
  transports: [stdio, http, sse]
  lifecycle: |
    Local MCP servers are spawned as child processes at session start. Remote MCP
    servers connect to HTTP endpoints. The docs do not describe mid-session
    reconnect, auto-retry, or dynamic capability refresh behavior, nor do they
    name a specific MCP protocol version date.
  notes: |
    OpenCode documentation uses provider-native transport names: `local` for
    command-based servers (stdio) and `remote` for HTTP-based servers. It does
    not mention Streamable HTTP, WebSocket, or legacy HTTP+SSE as distinct
    transports, nor does it cite a protocol generation date.
config_files:
  - os: all
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    notes: |
      Primary user-level OpenCode config. MCP servers live under the top-level
      `mcp` object. OpenCode also supports JSONC.
  - os: all
    scope: repo
    path: "opencode.json"
    format: json
    notes: |
      Project-level config. Same schema as global config. Safe to commit to Git.
  - os: macos
    scope: system
    path: "/Library/Application Support/opencode/opencode.json"
    format: json
    notes: |
      Managed/enterprise config. Highest precedence among file-based sources.
  - os: linux
    scope: system
    path: "/etc/opencode/opencode.json"
    format: json
    notes: |
      Linux managed config path.
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.json"
    format: json
    notes: |
      Windows managed config path.
  - os: macos
    scope: managed
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    format: other
    notes: |
      macOS MDM-deployed managed preferences; highest precedence overall.
  - os: all
    scope: other
    path: "OPENCODE_CONFIG"
    format: json
    notes: |
      Environment variable pointing to a custom config file loaded between
      global and project configs.
  - os: all
    scope: other
    path: "OPENCODE_CONFIG_CONTENT"
    format: json
    notes: |
      Inline JSON config content. Loaded last among standard sources (before
      managed settings) and is the mechanism Claudine uses for runtime
      injection.
  - os: all
    scope: other
    path: "~/.local/share/opencode/mcp-auth.json"
    format: json
    notes: |
      OAuth credential store for MCP servers, not the server definitions file.
cli_params:
  - flag: "opencode mcp add"
    description: "Interactive wizard to add a local or remote MCP server."
    example: "opencode mcp add"
  - flag: "opencode mcp list"
    description: "List configured MCP servers and their connection/auth status."
    example: "opencode mcp list"
  - flag: "opencode mcp ls"
    description: "Alias for `opencode mcp list`."
  - flag: "opencode mcp auth [name]"
    description: "Authenticate with an OAuth-enabled MCP server."
    example: "opencode mcp auth sentry"
  - flag: "opencode mcp auth list"
    description: "List OAuth-capable MCP servers and their auth status."
  - flag: "opencode mcp logout [name]"
    description: "Remove stored OAuth credentials for an MCP server."
    example: "opencode mcp logout sentry"
  - flag: "opencode mcp debug <name>"
    description: "Debug OAuth connectivity and discovery for a server."
    example: "opencode mcp debug sentry"
  - flag: "--config"
    description: "Point to an alternate config file (not MCP-specific)."
  - flag: "OPENCODE_CONFIG=<file>"
    description: "Use a custom config file path for this run."
    example: "OPENCODE_CONFIG=./custom.json opencode run \"hello\""
env_vars:
  - name: OPENCODE_CONFIG
    effect: |
      Path to a custom OpenCode config file. Loaded between global and project
      configs and can include an `mcp` section.
  - name: OPENCODE_CONFIG_CONTENT
    effect: |
      Inline JSON config content. Used by Claudine to inject MCP servers for a
      single run without mutating persistent config.
  - name: OPENCODE_CONFIG_DIR
    effect: |
      Path to a custom config directory used like `~/.config/opencode`.
  - name: OPENCODE_PERMISSION
    effect: |
      Inline JSON permissions config; can affect MCP tool approval policy when
      MCP tools are matched by permission keys.
server_schema:
  transports: ["local", "remote"]
  command_fields: ["type", "command", "environment", "cwd", "enabled", "timeout"]
  http_fields: ["type", "url", "headers", "oauth", "enabled", "timeout"]
  env_shape: |
    `environment` is an object mapping variable names to string values.
  auth_shape: |
    Remote servers support OAuth via an `oauth` object (with optional
    `clientId`, `clientSecret`, `scope`) or static `headers`. Static header
    values and OAuth client secrets can use `{env:VARIABLE_NAME}` substitution.
    Tokens are stored in `~/.local/share/opencode/mcp-auth.json`.
  notes: |
    Server id is the map key under `mcp`. Local servers use `"type": "local"`
    and a `command` array (command + args). Remote servers use
    `"type": "remote"` and a `url`. OpenCode does not distinguish stdio from
    SSE as separate type values; remote implies HTTP-based.
server_capabilities:
  tools: full
  resources: unknown
  prompts: unknown
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    MCP tools are exposed to the LLM alongside built-in tools. The docs do not
    describe dynamic tool-list refresh, resources, or prompts as user/model
    surfaces.
client_capabilities:
  roots: unknown
  sampling: unknown
  elicitation: unknown
  notes: |
    OpenCode does not document `roots/list`, `sampling/createMessage`, or
    elicitation support for MCP servers.
tool_surface:
  discovery: |
    MCP tools are fetched at server startup and become available to the LLM.
    Server-specific tool discovery and refresh semantics are not documented.
  filtering: |
    MCP tools can be enabled/disabled globally or per-agent using the `tools`
    config key or `permission` entries with glob patterns such as
    `mymcp_*` or `mymcp_search`. `tools` is deprecated in favor of `permission`.
  approval: |
    MCP tool calls are governed by the same permission system as built-in tools
    (`allow`, `ask`, `deny`). No MCP-specific approval policy is documented.
  result_handling: |
    Tool results are passed to the model. No native MCP response sanitization
    is documented.
  annotations_trusted: |
    Not documented. OpenCode does not describe handling of MCP tool
    annotations.
  notes: |
    MCP tools are registered with the server name as prefix (e.g.
    `mymcp_toolname`).
resource_surface:
  supported: false
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    MCP resources are not documented as a user-selectable or model-discoverable
    feature in OpenCode.
  notes: |
    The docs focus exclusively on MCP tools.
prompt_surface:
  supported: false
  invocation: ""
  arguments: ""
  exposure_model: |
    MCP prompts are not documented as slash commands, palette entries, or
    automatic model tools.
  notes: |
    OpenCode does not describe a prompt catalog surface for MCP servers.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: deep
  notes: |
    Claudine can read `~/.config/opencode/opencode.json` and project
    `opencode.json`, normalize the `mcp` object into its catalog, and write it
    back. Config files are merged, with later sources overriding conflicting
    keys; non-conflicting keys are preserved. OpenCode does not provide a CLI
    command to atomically add/remove a single MCP server definition, so apply
    must happen by rewriting the `mcp` object.
runtime_injection:
  supported: true
  mechanism: |
    Set `OPENCODE_CONFIG_CONTENT` to inline JSON containing an `mcp` object
    before launching `opencode`. Claudine merges this overlay with any existing
    `OPENCODE_CONFIG_CONTENT` value so that system prompts, permissions, and
    other keys are preserved.
  limitations: |
    The injected `mcp` object is merged shallowly on top of prior inline config.
    It does not replicate the full persistent-config precedence chain; user,
    project, and managed file configs are still loaded by OpenCode unless
    bypassed. OAuth flows cannot complete in non-interactive `opencode run`
    mode.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens are stored in `~/.local/share/opencode/mcp-auth.json`. Client
    secrets can reference `{env:VAR_NAME}` and are not persisted in plain text
    in config files.
  token_scope: |
    Per remote MCP server URL. Dynamic Client Registration is attempted
    automatically; explicit `scope` can be configured in the `oauth` object.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `environment` object or via
    `{env:VAR_NAME}` substitution anywhere in config. Process environment is
    otherwise inherited.
  notes: |
    `oauth: false` disables automatic OAuth for a remote server, allowing
    API-key style `headers` instead.
security:
  tool_filtering: |
    MCP tools can be filtered globally or per-agent using `permission` glob
    patterns (e.g. `mymcp_*` or `mymcp_search`). The legacy `tools` key offers
    similar enable/disable semantics.
  server_trust: |
    Project-level `opencode.json` MCP servers are loaded like any other config
    key; OpenCode does not document a separate trust gate for repo-level MCP
    servers. Managed config (MDM / system directories) takes precedence and
    cannot be overridden by users.
  env_sanitization: |
    Each local server receives its explicit `environment` map plus inherited
    process env. No documented env-scrubbing mode exists specifically for MCP
    servers.
  sandbox_interaction: |
    MCP server subprocesses run as ordinary local processes and are not
    described as isolated by any OS-level sandbox or container.
  response_filtering: |
    No native MCP response sanitization is documented. Claudine's `protect`
    layer should treat MCP tool results as untrusted.
  notes: |
    OAuth credentials are stored in a local JSON file. Organizations should use
    managed config to enforce an approved MCP server surface.
gaps:
  - |
    OpenCode does not state which MCP protocol version or generation it
    implements.
  - |
    Whether resources and prompts are exposed to the user or model is not
    documented.
  - |
    Roots, sampling, and elicitation support are not documented.
  - |
    Transport names are provider-native (`local`/`remote`); the mapping to MCP
    stdio/Streamable HTTP/SSE is not explicitly specified.
  - |
    Dynamic capability refresh, mid-session reconnect, and startup retry
    behavior are not documented.
  - |
    No OS-level sandbox or credential-scrubbing boundary for MCP servers is
    described.
changes: []
requires_claudine_update: false
reason: ""
---

# MCP Support in OpenCode CLI

## Overview

OpenCode supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io) to extend the agent's available tools. MCP servers can be local command-based processes or remote HTTP-based services. Configuration lives inside OpenCode's unified JSON/JSONC config, and the CLI provides a small set of `opencode mcp` commands for management.

This document maps OpenCode's MCP behavior to the schema used by Claudine's MCP catalog and provider wrappers.

## Protocol and Transports

OpenCode's documentation does not cite a specific MCP protocol version date. It uses provider-native transport names rather than MCP-standard transport names:

| OpenCode type | MCP equivalent | How it is configured |
| :------------ | :------------- | :------------------- |
| `local` | stdio | `type: "local"` with a `command` array |
| `remote` | HTTP/SSE | `type: "remote"` with a `url` |

The docs do not describe startup retry, mid-session reconnect, or dynamic capability refresh. They also do not mention Streamable HTTP, WebSocket, or legacy HTTP+SSE as distinct transports.

## Configuration

MCP servers are configured under the top-level `mcp` key in OpenCode config files.

### Config file locations and precedence

OpenCode merges multiple config sources. Later sources override earlier ones for conflicting keys:

1. Remote config (`.well-known/opencode`) — organizational defaults
2. Global user config (`~/.config/opencode/opencode.json`)
3. Custom config (`OPENCODE_CONFIG` env var)
4. Project config (`opencode.json` in project root)
5. `.opencode` directories (agents, commands, plugins)
6. Inline config (`OPENCODE_CONFIG_CONTENT` env var)
7. Managed config files or macOS MDM preferences — highest precedence

| Scope | Path | Notes |
| :---- | :--- | :---- |
| User | `~/.config/opencode/opencode.json` | Also supports `.jsonc` |
| Repo | `<project>/opencode.json` | Safe to commit to Git |
| Managed (macOS) | `/Library/Application Support/opencode/opencode.json` | Admin-controlled |
| Managed (Linux) | `/etc/opencode/opencode.json` | Admin-controlled |
| Managed (Windows) | `%ProgramData%\opencode\opencode.json` | Admin-controlled |
| Managed (macOS MDM) | `/Library/Managed Preferences/.../ai.opencode.managed.plist` | Highest precedence |

### Example config

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "filesystem": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem", "."],
      "environment": { "NODE_ENV": "production" },
      "enabled": true,
      "timeout": 5000
    },
    "sentry": {
      "type": "remote",
      "url": "https://mcp.sentry.dev/mcp",
      "oauth": {}
    }
  }
}
```

## Server Definition Shape

A server definition under `mcp.<name>` accepts the following fields:

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `type` | all | `"local"` or `"remote"` |
| `command` | local | Array of command + arguments |
| `url` | remote | Remote MCP endpoint URL |
| `environment` | local | Map of environment variables |
| `headers` | remote | Static header map |
| `oauth` | remote | OAuth config object, or `false` to disable auto-OAuth |
| `enabled` | all | Whether the server is active |
| `timeout` | all | Tool-fetch timeout in ms (default 5000) |
| `cwd` | local | Working directory for the server process |

Config values support `{env:VAR_NAME}` substitution. If the variable is unset, it expands to an empty string.

## Tools, Resources, and Prompts

### Tools

OpenCode exposes MCP tools to the LLM alongside built-in tools. Tool names appear with the server name as a prefix, so a server named `mymcp` with a tool `search` becomes available as something like `mymcp_search`.

Tool discovery happens at server startup. The docs do not describe dynamic tool-list refresh.

### Resources and prompts

The public documentation does not describe MCP resources or prompts as user-facing or model-facing features. Claudine should assume these surfaces are not exposed until documentation proves otherwise.

## Roots, Sampling, and Elicitation

OpenCode does not document support for MCP `roots/list`, `sampling/createMessage`, or elicitation.

## Import, Export, and Sync

Claudine can treat OpenCode as an `import_sync`-capable provider in addition to its `runtime_injection` support:

- **Import**: read `~/.config/opencode/opencode.json` and project `opencode.json`, then normalize the `mcp` object into the catalog.
- **Export**: write provider-shaped JSON back to those files.
- **Apply**: OpenCode does not offer CLI commands that atomically add or remove a single MCP server definition, so apply operations must rewrite the `mcp` object in the config file.

Merge semantics are deep for the config as a whole: later sources override conflicting keys, while non-conflicting keys are preserved.

## Runtime Injection

For one-run injection without mutating persistent config, Claudine uses `OPENCODE_CONFIG_CONTENT`:

```bash
OPENCODE_CONFIG_CONTENT='{"mcp":{"filesystem":{"type":"local","command":["npx","-y","@modelcontextprotocol/server-filesystem","."]}}}' opencode run "summarize"
```

Claudine's OpenCode injector merges the `mcp` overlay into any existing `OPENCODE_CONFIG_CONTENT` value so that system prompts, permissions, and other keys set by other producers are preserved.

Limitations:

- The injected config is an overlay, not a full replacement for the persistent config chain.
- OAuth flows cannot complete in non-interactive `opencode run` mode; pre-authenticated servers or static headers are required.

## Authorization and Credentials

OpenCode supports OAuth and static-header auth for remote MCP servers:

| Pattern | Where configured | Credential storage |
| :------ | :--------------- | :----------------- |
| Static header | `headers` | In config file (supports `{env:VAR}`) |
| OAuth dynamic | `oauth` object or `oauth: {}` | `~/.local/share/opencode/mcp-auth.json` |
| OAuth pre-registered | `oauth.clientId` + `oauth.clientSecret` | Same token file |

OAuth behavior:

- Dynamic Client Registration is attempted automatically.
- `opencode mcp auth <name>` triggers the flow manually.
- `opencode mcp logout <name>` clears stored tokens.
- `oauth: false` disables automatic OAuth, which is useful for API-key servers.

For local servers, secrets should be passed through the `environment` object or `{env:VAR}` substitution.

## Security Model

- **Tool filtering**: Use the `permission` key (or legacy `tools` key) with glob patterns such as `mymcp_*` or `mymcp_search` to allow, ask, or deny MCP tools globally or per-agent.
- **Server trust**: Project-level `opencode.json` MCP servers are not gated by a separate trust dialog. Managed config takes precedence and cannot be overridden by users.
- **Environment**: Local servers inherit the user's process environment plus the explicit `environment` map. No MCP-specific env scrubbing is documented.
- **Sandboxing**: MCP server subprocesses are ordinary local processes; no OS-level sandbox is described.
- **Response filtering**: No native MCP result sanitization is documented. Claudine's `protect` layer should scan MCP tool results defensively.

## Mode-Specific Behavior

- **TUI / interactive**: MCP servers start with the session. OAuth flows can be completed through `opencode mcp auth`.
- **`opencode run` (non-interactive)**: MCP servers configured in persistent config still load. OAuth flows cannot complete interactively.
- **`opencode serve` / server API**: The server exposes `GET /mcp` for status and `POST /mcp` to add a server dynamically.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Server fails to start | Connection error shown in `opencode mcp list`; no documented auto-retry |
| Remote auth failure | OAuth flow triggered interactively, or server marked as needing auth |
| Tool fetch timeout | Per-server `timeout` (default 5000 ms) |
| Output too large | No documented MCP-specific output limit |

## Gaps

- Explicit MCP protocol version is not stated.
- Resource and prompt surfaces are not documented.
- Roots, sampling, and elicitation support are not documented.
- Mapping from `local`/`remote` to MCP stdio/HTTP/SSE transports is implicit.
- Dynamic capability refresh, reconnect, and retry semantics are not documented.
- No documented sandbox or credential-scrubbing boundary for MCP servers.

## Claudine Integration Notes

- Treat OpenCode as `support: runtime_injection` because `OPENCODE_CONFIG_CONTENT` allows one-run MCP injection without mutating user config. Import and export of persistent config are also supported.
- Map Claudine's normalized catalog to OpenCode's `mcp` object shape, using `type: "local"` for stdio servers (command array) and `type: "remote"` for HTTP/SSE servers (url).
- For one-run wrappers, merge desired servers into `OPENCODE_CONFIG_CONTENT` while preserving existing keys.
- Do not rely on a separate repo-trust gate for `.opencode.json` MCP servers; OpenCode loads project config directly.
- Defensively scan MCP tool results in the `protect` layer; OpenCode does not provide native response sanitization.

## Sources

- [OpenCode MCP servers docs](https://opencode.ai/docs/mcp-servers/)
- [OpenCode config docs](https://opencode.ai/docs/config/)
- [OpenCode CLI reference](https://opencode.ai/docs/cli/)
- [OpenCode agents docs](https://opencode.ai/docs/agents/)
- [OpenCode server docs](https://opencode.ai/docs/server/)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)
