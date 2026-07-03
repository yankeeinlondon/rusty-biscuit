---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://code.claude.com/docs/en/mcp
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http, http_sse, sse, ws]
  lifecycle: |
    Stdio servers are spawned as local subprocesses at session start and are not
    reconnected automatically if they exit. HTTP, SSE, and WebSocket servers connect
    at startup and auto-reconnect with exponential backoff (up to five attempts for
    mid-session disconnects; up to three retries for transient initial-connection
    errors). Capability discovery requests (tools/list, prompts/list, resources/list)
    retry transient errors up to three times. Dynamic capability updates are accepted
    via MCP list_changed notifications.
  notes: |
    Claude Code accepts `type: "streamable-http"` as an alias for `"http"`. SSE is
    documented as deprecated in favor of HTTP. WebSocket (`type: "ws"`) is supported
    for servers that push events, but cannot be added with `--transport ws`; it must
    be configured via JSON. The docs do not state an explicit MCP protocol version
    date; behavior matches the specification at https://modelcontextprotocol.io.
config_files:
  - os: all
    scope: user
    path: "~/.claude.json"
    format: json
    notes: |
      User-scoped MCP servers live in the top-level `mcpServers` object. The same file
      also stores per-project local-scoped servers under `projects/<path>/mcpServers`.
      This is distinct from `~/.claude/settings.json`, which does NOT hold MCP servers.
  - os: all
    scope: repo
    path: ".mcp.json"
    format: json
    notes: |
      Project-scoped servers intended for version control. Servers from this file are
      not connected until the workspace is trusted and the user approves them.
  - os: all
    scope: local
    path: "~/.claude.json"
    format: json
    notes: |
      Local-scoped servers are stored inside `~/.claude.json` under the current
      project's `projects/<path>/mcpServers` entry. This differs from general local
      settings in `.claude/settings.local.json`.
  - os: all
    scope: plugin
    path: "<plugin-root>/.mcp.json"
    format: json
    notes: |
      Plugins can bundle MCP servers in a `.mcp.json` file at the plugin root or inline
      under `mcpServers` in `plugin.json`. Plugin servers start automatically when the
      plugin is enabled.
  - os: macos
    scope: system
    path: "/Library/Application Support/ClaudeCode/managed-mcp.json"
    format: json
    notes: |
      Managed/enterprise fixed server set. If present, it takes exclusive control and
      blocks user-, project-, and plugin-added servers.
  - os: linux
    scope: system
    path: "/etc/claude-code/managed-mcp.json"
    format: json
    notes: |
      Linux/WSL path for the managed `managed-mcp.json` file.
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\managed-mcp.json"
    format: json
    notes: |
      Windows path for the managed `managed-mcp.json` file.
  - os: all
    scope: managed
    path: "~/.claude/settings.json / .claude/settings.json / managed-settings.json / MDM / registry"
    format: json
    notes: |
      Allowlists and denylists (`allowedMcpServers`, `deniedMcpServers`) and related
      policy flags (`allowManagedMcpServersOnly`, `disableClaudeAiConnectors`, etc.).
cli_params:
  - flag: "claude mcp add <name> -- <command> [args...]"
    description: "Add a persistent stdio MCP server."
    example: "claude mcp add fs -- npx -y @modelcontextprotocol/server-filesystem ."
  - flag: "claude mcp add --transport http <name> <url>"
    description: "Add a remote HTTP MCP server."
    example: "claude mcp add --transport http sentry https://mcp.sentry.dev/mcp"
  - flag: "claude mcp add --transport sse <name> <url>"
    description: "Add a remote SSE MCP server (deprecated)."
    example: "claude mcp add --transport sse asana https://mcp.asana.com/sse"
  - flag: "claude mcp add-json <name> '<json>'"
    description: "Add a server from raw JSON, useful for WebSocket and OAuth configs."
    example: "claude mcp add-json ws '{\"type\":\"ws\",\"url\":\"wss://...\"}'"
  - flag: "claude mcp add-from-claude-desktop"
    description: "Import servers from Claude Desktop (macOS and WSL only)."
    example: "claude mcp add-from-claude-desktop --scope user"
  - flag: "claude mcp list"
    description: "List configured servers and their connection status."
  - flag: "claude mcp get <name>"
    description: "Show details for a specific server."
  - flag: "claude mcp remove <name>"
    description: "Remove a configured server."
  - flag: "claude mcp login <name>"
    description: "Run OAuth flow for a server from the shell."
  - flag: "claude mcp logout <name>"
    description: "Clear stored OAuth credentials for a server."
  - flag: "claude mcp reset-project-choices"
    description: "Reset approval choices for project-scoped .mcp.json servers."
  - flag: "claude mcp serve"
    description: "Run Claude Code itself as a stdio MCP server."
  - flag: "--scope user|project|local"
    description: "Target scope for add/add-json/remove."
  - flag: "--env KEY=VALUE"
    description: "Set an environment variable on a stdio server."
  - flag: "--header 'Name: Value'"
    description: "Set a static header on an HTTP/SSE server."
  - flag: "--callback-port <port>"
    description: "Pin the OAuth callback port."
  - flag: "--client-id <id> / --client-secret"
    description: "Use pre-configured OAuth credentials."
  - flag: "--mcp-config <file-or-json>"
    description: "Load MCP servers for a single run without mutating user config."
  - flag: "--channels plugin:<name>@<marketplace>"
    description: "Opt specific channel-capable MCP plugins into push messaging."
  - flag: "--bare"
    description: "Skip auto-discovery of hooks, skills, plugins, MCP servers, etc."
env_vars:
  - name: MCP_TIMEOUT
    effect: |
      Startup timeout for MCP servers in milliseconds (default not stated; example
      uses 10000 for 10 seconds).
  - name: MCP_TOOL_TIMEOUT
    effect: |
      Default per-server tool execution timeout in milliseconds. Overridden by a
      per-server `timeout` field.
  - name: MAX_MCP_OUTPUT_TOKENS
    effect: |
      Default maximum MCP tool output tokens (default 25000). Claude Code warns at
      10000 tokens unless the server declares `_meta["anthropic/maxResultSizeChars"]`.
  - name: CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT
    effect: |
      Idle timeout in milliseconds for remote HTTP/SSE/WebSocket/claude.ai tool calls
      that send no response or progress notification. Defaults to 5 minutes; set to 0
      to disable.
  - name: CLAUDE_CODE_SUBPROCESS_ENV_SCRUB
    effect: |
      When set, strips Anthropic and cloud-provider credentials from all subprocesses
      spawned by Claude Code, including stdio MCP servers.
  - name: ENABLE_TOOL_SEARCH
    effect: |
      Tool search is enabled by default and is disabled when using a custom
      ANTHROPIC_BASE_URL, Vertex AI, or other non-first-party providers unless set to
      true explicitly.
  - name: ENABLE_CLAUDEAI_MCP_SERVERS
    effect: |
      Set to "false" to disable claude.ai connectors for the session. Equivalent to
      the `disableClaudeAiConnectors` setting.
  - name: MCP_CLIENT_SECRET
    effect: |
      Provides a pre-configured OAuth client secret non-interactively when running
      `claude mcp add --client-secret`.
  - name: CLAUDE_PROJECT_DIR
    effect: |
      Set by Claude Code in the environment of stdio MCP servers to the project root,
      so servers can resolve project-relative paths.
  - name: CLAUDE_CODE_MCP_SERVER_NAME
    effect: |
      Set when running a `headersHelper` command; names the MCP server being connected.
  - name: CLAUDE_CODE_MCP_SERVER_URL
    effect: |
      Set when running a `headersHelper` command; provides the server's URL.
  - name: CLAUDE_PLUGIN_ROOT
    effect: |
      Set when running a `headersHelper` for a plugin-provided server; points to the
      plugin root directory.
server_schema:
  transports: ["stdio", "http", "streamable-http", "sse", "ws"]
  command_fields: ["type", "command", "args", "env", "cwd", "timeout", "alwaysLoad"]
  http_fields: ["type", "url", "headers", "headersHelper", "oauth", "timeout", "alwaysLoad"]
  env_shape: |
    `env` is an object mapping variable names to string values. Values in `env`, plus
    `command`, `args`, and `url`, support `${VAR}` and `${VAR:-default}` expansion
    sourced from the user's process environment.
  auth_shape: |
    HTTP/SSE servers support OAuth 2.0 (dynamic client registration or pre-configured
    clientId/clientSecret inside an `oauth` object), static `headers`, or a
    `headersHelper` command that emits a JSON object of headers at connect time.
    WebSocket servers support header-only auth. OAuth tokens are stored in the system
    keychain or a credentials file, not in config.
  notes: |
    Server id is the map key under `mcpServers`. The `type` field accepts `"stdio"`,
    `"http"`, `"streamable-http"`, `"sse"`, or `"ws"`. The reserved name `workspace`
    is skipped. Project-scoped servers from `.mcp.json` require user approval before
    they load.
server_capabilities:
  tools: full
  resources: unknown
  prompts: unknown
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: true
  prompt_list_changed: true
  notes: |
    Tools are fully exposed to the model and refreshed when the server sends a
    `list_changed` notification. Resources and prompts are part of the MCP protocol,
    but the Claude Code documentation only describes dynamic updates for tools,
    prompts, and resources together; it does not state whether resources or prompts
    are surfaced in the UI or offered to the model autonomously.
client_capabilities:
  roots: partial
  sampling: unknown
  elicitation: unknown
  notes: |
    Stdio servers receive `CLAUDE_PROJECT_DIR` and can call `roots/list`, which
    returns the directory Claude Code was launched from. The documentation does not
    describe `sampling/createMessage` or elicitation support.
tool_surface:
  discovery: |
    Claude Code calls `tools/list` at server startup and refreshes the list when a
    `list_changed` notification is received. With tool search enabled, it waits for
    needed servers before continuing.
  filtering: |
    Per-server filtering is available via managed `allowedMcpServers`/`deniedMcpServers`
    and via permission rules such as `mcp__<server>__<tool>` or `mcp__<server>__*` in
    `permissions.allow`/`deny`. CLI `--allowedTools`/`--disallowedTools` also apply,
    and the `disabledMcpjsonServers` setting can reject project servers by name.
  approval: |
    MCP tool calls use the same permission model as native tools. An
    `anthropic/requiresUserInteraction` annotation on a tool forces a prompt on every
    call, even in bypass/auto/acceptEdits modes.
  result_handling: |
    Text, image, and resource_link results are passed to the model. Tool errors are
    surfaced with `isError`. Outputs above `MAX_MCP_OUTPUT_TOKENS` are persisted to
    disk and replaced with a file reference; servers can raise their own text limit
    via `_meta["anthropic/maxResultSizeChars"]` up to 500,000 characters.
  annotations_trusted: |
    The `anthropic/requiresUserInteraction` annotation is enforced, but annotations are
    otherwise treated as hints, not as trusted policy.
  notes: |
    Project-scoped servers from `.mcp.json` must be approved before their tools become
    available. There is no documented per-argument approval policy.
resource_surface:
  supported: false
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    Not documented. Claude Code focuses on tool exposure; resources are not described
    as user-selectable or model-discoverable in the public docs.
  notes: |
    The protocol-level `resources/list` and `resources/list_changed` capabilities are
    mentioned only in the context of dynamic capability discovery, not as a surfaced
    feature.
prompt_surface:
  supported: false
  invocation: ""
  arguments: ""
  exposure_model: |
    Not documented. MCP prompts are not described as slash commands, palette entries,
    or automatic model tools.
  notes: |
    The protocol-level `prompts/list` and `prompts/list_changed` capabilities are
    mentioned only in the context of dynamic capability discovery.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: deep
  notes: |
    Claudine can read `~/.claude.json` and `.mcp.json`, normalize them into its MCP
    catalog, and write them back. Server definitions are replaced whole per scope
    (highest-precedence source wins the full entry), while allow/deny policy arrays
    merge across sources. The `claude mcp add`, `claude mcp add-json`, and
    `claude mcp remove` commands provide a supported apply path.
runtime_injection:
  supported: true
  mechanism: |
    Pass `--mcp-config <file-or-json>` to a `claude` invocation (commonly with `--bare`
    in headless mode) to load MCP servers for that run without mutating
    `~/.claude.json` or `.mcp.json`. `--bare` additionally skips auto-discovery of user
    and project MCP servers.
  limitations: |
    `--mcp-config` is primarily documented for bare/headless use. It is not a complete
    substitute for persistent config merge semantics; Claudine should merge desired
    servers into the supplied JSON if it needs to preserve user defaults for the run.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens are stored in the macOS Keychain when available, or in a credentials
    file on Windows/Linux. Client secrets added via `--client-secret` are stored the
    same way, not in config files.
  token_scope: |
    Per remote server URL. Refresh tokens are stored by Claude Code and refreshed
    automatically.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` object or via `${VAR}` expansion
    sourced from the user's process environment. Process environment is otherwise
    inherited.
  notes: |
    Static `headers.Authorization` is supported but discouraged for shared repo config.
    The `headersHelper` command can generate short-lived tokens at connect time.
security:
  tool_filtering: |
    Managed `allowedMcpServers`/`deniedMcpServers` filter by `serverUrl`,
    `serverCommand`, or `serverName`. Permission rules support `mcp__<server>__<tool>`
    patterns. `disabledMcpjsonServers` rejects project servers by name. The
    `anthropic/requiresUserInteraction` annotation forces per-call approval.
  server_trust: |
    Project-scoped `.mcp.json` servers are pending until the workspace is trusted and
    the user approves. `enableAllProjectMcpServers`/`enabledMcpjsonServers` committed to
    `.claude/settings.json` are ignored in an untrusted folder. Managed
    `managed-mcp.json` takes exclusive control and blocks other servers.
  env_sanitization: |
    Each stdio server receives only its explicit `env` map plus inherited process env.
    `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` strips Anthropic/cloud credentials from all
    subprocesses. The Bash sandbox's `sandbox.credentials` can deny or mask env vars,
    but only for sandboxed Bash commands, not MCP servers.
  sandbox_interaction: |
    MCP servers run as ordinary local processes and are not isolated by Claude Code's
    built-in Bash sandbox, which applies only to Bash tool subprocesses. There is no
    documented OS-level sandbox or container boundary around stdio MCP servers.
  response_filtering: |
    No native MCP response sanitization is documented. Large tool outputs are persisted
    to disk and summarized. The `protect` layer in Claudine should treat MCP results as
    untrusted and scan them for prompt-injection patterns.
  notes: |
    OAuth tokens are stored in the OS credential store. Administrators should deploy
    `managed-mcp.json` and `allowedMcpServers`/`deniedMcpServers` to enforce an
    organization-wide server surface.
gaps:
  - |
    The official docs do not state which MCP protocol version date Claude Code
    implements (e.g., 2024-11-05 or 2025-06-18).
  - |
    Whether resources and prompts are exposed to the user or model is not documented.
  - |
    Sampling (server-requested LLM calls) and elicitation (server-requested user input)
    are not documented.
  - |
    No first-class sandbox/container boundary for stdio MCP servers is described.
  - |
    Exact precedence and merge behavior of `--mcp-config` versus persistent config is
    not fully specified.
changes: []
requires_claudine_update: false
reason: ""
---

# MCP Support in Claude Code

## Overview

Claude Code supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io) as a first-class integration path. MCP servers extend Claude Code with external tools, databases, APIs, and event channels. Servers can be local stdio processes, remote HTTP/SSE/WebSocket endpoints, or bundled with plugins. Configuration is scope-based, persistent, and can be administered through managed policy files.

This document maps Claude Code's MCP behavior to the schema used by Claudine's MCP catalog and provider wrappers.

## Protocol and Transports

Claude Code speaks several MCP transports:

| Transport | Status | How it is added |
| :-------- | :----- | :-------------- |
| `stdio` | Primary | `claude mcp add <name> -- <command>` |
| `http` (`streamable-http` alias) | Recommended for remote | `claude mcp add --transport http <name> <url>` |
| `sse` | Deprecated | `claude mcp add --transport sse <name> <url>` |
| `ws` | Supported for push | Only via JSON config (`type: "ws"`) |

The docs recommend HTTP for remote servers and note that SSE is deprecated. WebSocket is intended for servers that push events into the session (see [channels](#mode-specific-behavior)).

Lifecycle behavior differs by transport:

- **stdio** servers are spawned as local child processes at session start. If they exit, they are **not** automatically reconnected.
- **HTTP/SSE/WebSocket** servers connect at startup and reconnect automatically with exponential backoff (up to five attempts) if they disconnect mid-session. Initial connections also retry transient errors (5xx, connection refused, timeout) up to three times.
- Capability discovery requests (`tools/list`, `prompts/list`, `resources/list`) retry transient network and server errors up to three times.
- Servers can send `list_changed` notifications to update available capabilities without restarting the session.

The documentation does not name a specific MCP protocol version date, so Claudine should treat the implemented version as observed rather than pinned.

## Configuration

MCP servers are configured in a hierarchy of files. The most important distinction is that `~/.claude.json` holds user and local MCP servers, while general Claude Code settings live in `~/.claude/settings.json`.

### Scopes

| Scope | File | Shared | Trust gate |
| :---- | :--- | :----- | :--------- |
| User | `~/.claude.json` top-level `mcpServers` | No | None |
| Local | `~/.claude.json` `projects/<path>/mcpServers` | No | None |
| Project | `.mcp.json` | Yes (git) | Requires workspace trust + user approval |
| Plugin | plugin `.mcp.json` or `plugin.json` inline | With plugin | None beyond plugin trust |
| Managed | system `managed-mcp.json` | Organization-wide | Admin-controlled |
| Cloud | claude.ai connectors | Via claude.ai org | Anthropic authentication |

### Scope precedence

When the same server is defined in multiple places, the entire definition from the highest-precedence source is used (fields are **not** merged across scopes):

1. Local scope
2. Project scope
3. User scope
4. Plugin-provided servers
5. claude.ai connectors

Duplicate detection matches by name for the three primary scopes and by endpoint (URL or command) for plugins and connectors.

### Project trust

Servers in `.mcp.json` are not loaded until the user runs `claude` interactively and accepts the workspace trust dialog. `claude mcp list` shows them as `⏸ Pending approval` until then. Committing `enableAllProjectMcpServers` or `enabledMcpjsonServers` to `.claude/settings.json` does **not** bypass trust for a freshly cloned repo.

### Managed configuration

Administrators can deploy `managed-mcp.json` to system paths:

- macOS: `/Library/Application Support/ClaudeCode/managed-mcp.json`
- Linux/WSL: `/etc/claude-code/managed-mcp.json`
- Windows: `C:\Program Files\ClaudeCode\managed-mcp.json`

When present, `managed-mcp.json` takes **exclusive control**: users cannot add, modify, or run any other MCP servers, including plugin-provided servers and claude.ai connectors (unless `allowAllClaudeAiMcps` is set in managed settings).

Policy controls in settings files include:

- `allowedMcpServers` — allowlist by `serverUrl`, `serverCommand`, or `serverName`
- `deniedMcpServers` — denylist (merges from all scopes, takes precedence over allowlist)
- `allowManagedMcpServersOnly` — locks the allowlist to managed settings
- `disableClaudeAiConnectors` — disables cloud connectors
- `strictPluginOnlyCustomization: ["mcp"]` — allows MCP servers only from plugins or managed settings

## Server Definition Shape

A server definition in `~/.claude.json` or `.mcp.json` looks like:

```json
{
  "mcpServers": {
    "filesystem": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
      "env": { "KEY": "value" },
      "cwd": "/path/to/working/dir",
      "timeout": 600000
    },
    "github": {
      "type": "http",
      "url": "https://api.githubcopilot.com/mcp/",
      "headers": { "Authorization": "Bearer ${GITHUB_PAT}" },
      "oauth": { "scopes": "repo" },
      "timeout": 600000
    }
  }
}
```

### Field reference

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `type` | all | `stdio`, `http`, `streamable-http`, `sse`, or `ws` |
| `command` | stdio | Executable to spawn |
| `args` | stdio | Argument array |
| `env` | stdio | Map of environment variables |
| `cwd` | stdio | Working directory for the server process |
| `url` | http/sse/ws | Endpoint URL |
| `headers` | http/sse/ws | Static header map |
| `headersHelper` | http/sse/ws | Shell command that emits a JSON header map |
| `oauth` | http/sse | OAuth configuration object |
| `timeout` | all | Per-server tool execution timeout in ms |
| `alwaysLoad` | all | Whether to load the server eagerly |

### Environment variable expansion

Values in `command`, `args`, `env`, `url`, and `headers` support:

- `${VAR}` — expand to the value of `VAR`
- `${VAR:-default}` — expand to `VAR` or `default` if unset

If a required variable is unset and has no default, config parsing fails.

## Tools, Resources, and Prompts

### Tools

Claude Code exposes MCP tools to the model. Tool names appear to the model as `mcp__<server>__<tool>`. Plugin-bundled tools use the form `mcp__plugin_<plugin-name>_<server-name>__<tool-name>`.

Tool discovery:

- `tools/list` is called at server startup.
- Lists are refreshed when the server sends a `list_changed` notification.
- Claude Code waits for a server to finish connecting if the current request needs its tools.

Tool approval:

- MCP tools follow the same permission model as native tools.
- Users can write permission rules such as `mcp__github__get_issue` or `mcp__github__*`.
- A tool can force per-call approval by setting `_meta["anthropic/requiresUserInteraction"]: true`.

Tool output:

- A warning is shown when output exceeds 10,000 tokens.
- The default hard limit is 25,000 tokens (`MAX_MCP_OUTPUT_TOKENS`).
- Servers can raise their own text limit via `_meta["anthropic/maxResultSizeChars"]` up to 500,000 characters.
- Large outputs are persisted to disk and replaced with a file reference in the conversation.

### Resources and prompts

The public documentation does not describe a user-facing resource picker or prompt catalog. The protocol-level `resources/list` and `prompts/list` capabilities are mentioned only as part of dynamic `list_changed` refresh, not as surfaced features. Claudine should assume **resources and prompts are not exposed to users or the model** until documentation proves otherwise.

## Roots, Sampling, and Elicitation

### Roots

Claude Code provides a limited root boundary to stdio servers:

- The environment variable `CLAUDE_PROJECT_DIR` is set to the project root.
- Servers can call MCP `roots/list`, which returns the directory Claude Code was launched from.
- This is the same directory hooks receive in `CLAUDE_PROJECT_DIR`.

### Sampling and elicitation

The documentation does not describe support for MCP `sampling/createMessage` (server-requested LLM calls) or `completion/complete` (elicitation). Claudine should treat these as **unknown** for Claude Code.

## Import, Export, and Sync

Claudine can treat Claude Code as an `import_sync` provider:

- **Import**: read `~/.claude.json` and `.mcp.json` and normalize server definitions into the MCP catalog.
- **Export**: write provider-shaped JSON back to those files.
- **Apply**: use `claude mcp add`, `claude mcp add-json`, and `claude mcp remove` to mutate configuration through the supported CLI.

Merge semantics:

- Server entries are replaced whole from the highest-precedence scope; fields are not merged across scopes.
- `allowedMcpServers` and `deniedMcpServers` arrays merge across settings sources.
- When `allowManagedMcpServersOnly` is true, only the managed allowlist applies (denylists still merge).

 Claudine should be careful not to overwrite the non-MCP contents of `~/.claude.json` when rewriting the `mcpServers` section.

## Runtime Injection

For one-run injection without mutating persistent config, Claude Code offers:

- `--mcp-config <file-or-json>` — load MCP servers for the current invocation only.
- `--bare` — skip auto-discovery of user/project/plugin MCP servers, skills, hooks, etc.

Typical headless usage:

```bash
claude --bare -p "Summarize this project" \
  --mcp-config '{"mcpServers":{"fs":{"type":"stdio","command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","."]}}}'
```

Limitations:

- `--mcp-config` is documented primarily for bare/headless mode.
- It does not preserve the normal user/project/local merge semantics; Claudine must build the desired effective config itself.
- OAuth flows cannot complete in non-interactive `claude -p` mode; pre-authenticated servers or static headers are required.

## Authorization and Credentials

Claude Code supports multiple auth patterns for remote servers:

| Pattern | Where configured | Credential storage |
| :------ | :--------------- | :----------------- |
| Static header | `headers.Authorization` | In config file (not recommended for shared repos) |
| Dynamic header | `headersHelper` command | Helper fetches token at connect time |
| OAuth 2.0 dynamic | `oauth` object | System keychain / credentials file |
| OAuth 2.0 pre-registered | `oauth.clientId` + `--client-secret` | System keychain / credentials file |

OAuth details:

- Dynamic Client Registration is attempted automatically.
- `claude mcp login <name>` runs the OAuth flow from the shell.
- `claude mcp logout <name>` clears stored tokens.
- `--callback-port` pins the redirect URI port.
- `oauth.scopes` restricts requested scopes.
- `authServerMetadataUrl` overrides OAuth metadata discovery.
- `MCP_CLIENT_SECRET` can supply the client secret non-interactively.

For stdio servers, secrets should be passed through the per-server `env` object or `${VAR}` expansion rather than committed in config files.

## Security Model

### Trust and allowlisting

- Project `.mcp.json` servers require explicit user approval after workspace trust.
- Managed `managed-mcp.json` provides exclusive, admin-controlled server sets.
- `allowedMcpServers`/`deniedMcpServers` filter by URL, exact command/args, or name.
- Permission rules support per-server and per-tool patterns (`mcp__<server>__<tool>`).

### Environment and sandboxing

- Stdio servers inherit the user's process environment plus explicit `env`.
- `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` strips Anthropic and cloud-provider credentials from all subprocesses.
- The built-in Bash sandbox does **not** isolate MCP servers; it applies only to Bash tool subprocesses.
- There is no documented OS-level sandbox around stdio MCP servers.

### Response handling

- No native MCP result sanitization is documented.
- Large outputs are truncated or persisted to disk.
- Claudine's `protect` layer should scan MCP tool results defensively.

## Mode-Specific Behavior

### Interactive mode

- `/mcp` opens a panel showing connected servers, tool counts, and connection status.
- OAuth flows can be completed through `/mcp`.
- Project `.mcp.json` servers appear as pending until approved.
- Channels (push messaging) are opt-in per session via `--channels`.

### Non-interactive / headless mode (`-p`)

- OAuth flows cannot run; pre-authenticated or header-based servers are required.
- If a server needs auth, its tools are reported as unavailable.
- `--bare` skips auto-discovery and is recommended for CI.
- `--mcp-config` can inject servers for the run.

### Claude Code as an MCP server

`claude mcp serve` runs Claude Code as a stdio MCP server, exposing its built-in tools to an external MCP client. The client is responsible for its own approval UI.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Server fails to start | Marked failed in `/mcp` and `claude mcp list`; stdio servers are not retried |
| HTTP/SSE transient error | Retried up to three times at startup; up to five reconnects mid-session |
| HTTP/SSE auth or 4xx error | Not retried; server marked as needing auth or failed |
| Stdio server exits | Not auto-reconnected; user must restart the session |
| Tool idle timeout (remote) | Aborted after `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT` (default 5 min) |
| Tool wall-clock timeout | Aborted after per-server `timeout` or `MCP_TOOL_TIMEOUT` |
| Output too large | Persisted to disk and replaced with a file reference |
| Project server untrusted | Remains `⏸ Pending approval` until trusted |

## Gaps

- Explicit MCP protocol version date is not stated.
- Resource and prompt surfaces are not documented as user/model-facing features.
- Sampling and elicitation support are not documented.
- No documented OS-level sandbox or container boundary for stdio MCP servers.
- Exact merge semantics of `--mcp-config` versus persistent config are unspecified.

## Claudine Integration Notes

- Treat Claude Code as `support: import_sync`. Claudine can read and write `~/.claude.json` and `.mcp.json`, and can apply changes through `claude mcp add/add-json/remove`.
- Map Claudine's normalized catalog to Claude's `mcpServers` object shape, preserving the `type` field and transport-specific fields.
- For one-run wrappers, prefer `--mcp-config` with `--bare`; build the effective server list in memory rather than mutating user config.
- Do not place MCP server definitions in `~/.claude/settings.json` or `.claude/settings.json`; those files hold policy and other settings, not MCP servers.
- Honor project trust: do not assume `.mcp.json` servers are active until the user has approved them in an interactive session.
- Defensively scan MCP tool results in the `protect` layer; Claude Code does not provide native response sanitization.

## Sources

- [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code settings](https://code.claude.com/docs/en/settings)
- [Managed MCP configuration](https://code.claude.com/docs/en/managed-mcp)
- [Environment variables](https://code.claude.com/docs/en/env-vars)
- [Headless / Agent SDK](https://code.claude.com/docs/en/headless)
- [Permissions](https://code.claude.com/docs/en/permissions)
- [Security](https://code.claude.com/docs/en/security)
- [Sandboxing](https://code.claude.com/docs/en/sandboxing)
- [Channels](https://code.claude.com/docs/en/channels)
- [Agent SDK permissions](https://code.claude.com/docs/en/agent-sdk/permissions)
