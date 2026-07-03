---
$schema: ./_schema.yaml
created: 2025-04-13
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://www.geminicli.com/docs/tools/mcp-server/
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, sse, streamable_http]
  lifecycle: |
    MCP servers are discovered once at Gemini CLI startup. Stdio servers are
    spawned as local subprocesses; SSE and Streamable HTTP clients open remote
    connections. Servers that fail to connect are marked DISCONNECTED and are not
    automatically reconnected. The public docs do not describe dynamic
    list_changed capability refresh.
  notes: |
    Transport is selected by the presence of config keys: `command` for stdio,
    `url` for SSE, and `httpUrl` for Streamable HTTP. An optional `type` field
    accepts `"stdio"`, `"sse"`, or `"http"`. The settings schema also contains a
    `tcp` property, but the official MCP documentation only describes the three
    transports above. The docs do not state an explicit MCP protocol version;
    the only version reference is a link to the 2025-06-18 schema in the
    "Instructions" section.
config_files:
  - os: all
    scope: user
    path: "~/.gemini/settings.json"
    format: json
    notes: |
      User-scope MCP servers live in the top-level `mcpServers` object. The same
      file holds general Gemini CLI settings.
  - os: all
    scope: repo
    path: ".gemini/settings.json"
    format: json
    notes: |
      Project-scope MCP servers. Loaded only when the workspace is trusted
      (when Folder Trust is enabled) or when trust is bypassed.
  - os: linux
    scope: system
    path: "/etc/gemini-cli/system-defaults.json"
    format: json
    notes: |
      System-wide baseline defaults. Lowest precedence for single-value
      settings; objects like `mcpServers` are merged.
  - os: macos
    scope: system
    path: "/Library/Application Support/GeminiCli/system-defaults.json"
    format: json
    notes: |
      macOS system-wide baseline defaults.
  - os: windows
    scope: system
    path: "C:\\ProgramData\\gemini-cli\\system-defaults.json"
    format: json
    notes: |
      Windows system-wide baseline defaults.
  - os: linux
    scope: managed
    path: "/etc/gemini-cli/settings.json"
    format: json
    notes: |
      System-wide override settings. Highest precedence for single-value
      settings; `mcpServers` objects are merged.
  - os: macos
    scope: managed
    path: "/Library/Application Support/GeminiCli/settings.json"
    format: json
    notes: |
      macOS system-wide override settings.
  - os: windows
    scope: managed
    path: "C:\\ProgramData\\gemini-cli\\settings.json"
    format: json
    notes: |
      Windows system-wide override settings.
  - os: all
    scope: plugin
    path: "<extension-root>/gemini-extension.json"
    format: json
    notes: |
      Extensions can bundle MCP servers under their `mcpServers` object. User
      and project `settings.json` can override extension server definitions.
  - os: all
    scope: user
    path: "~/.gemini/mcp-server-enablement.json"
    format: json
    notes: |
      Tracks per-server enabled/disabled state, including `--session` changes.
  - os: all
    scope: user
    path: "~/.gemini/mcp-oauth-tokens.json"
    format: json
    notes: |
      Stores OAuth tokens for remote MCP servers.
  - os: all
    scope: user
    path: "~/.gemini/trustedFolders.json"
    format: json
    notes: |
      Records trusted/untrusted folder decisions when Folder Trust is enabled.
cli_params:
  - flag: "gemini mcp add [options] <name> <commandOrUrl> [args...]"
    description: "Adds a persistent MCP server to user or project settings.json."
    example: "gemini mcp add --transport http secure-api https://api.example.com/mcp/"
  - flag: "gemini mcp list"
    description: "Lists configured MCP servers, their configuration, and connection status."
  - flag: "gemini mcp remove <name>"
    description: "Removes an MCP server from settings.json."
    example: "gemini mcp remove my-server --scope user"
  - flag: "gemini mcp enable <name> [--session]"
    description: "Enables a disabled MCP server. --session applies only to the current run."
  - flag: "gemini mcp disable <name> [--session]"
    description: "Disables an MCP server. --session applies only to the current run."
  - flag: "--allowed-mcp-server-names <names>"
    description: "Comma-separated list of server names allowed to connect for this run."
    example: "gemini --allowed-mcp-server-names fs,github -p 'use tools'"
  - flag: "/mcp auth [server-name]"
    description: "Interactive slash command to authenticate with an OAuth-enabled MCP server."
  - flag: "/mcp reload"
    description: "Reloads all MCP servers and re-discovers available tools."
  - flag: "/mcp list | /mcp desc | /mcp schema"
    description: "Interactive slash commands to inspect servers, tools, and schemas."
  - flag: "--skip-trust"
    description: "Bypasses the folder-trust dialog in headless/automated environments."
  - flag: "--sandbox / GEMINI_SANDBOX"
    description: "Enables sandboxing for built-in tools; does not sandbox MCP stdio servers."
env_vars:
  - name: GEMINI_CLI_HOME
    effect: |
      Relocates the entire Gemini CLI state directory (default `~/.gemini`).
      Useful for isolating state in CI or shared environments.
  - name: GEMINI_CLI_SYSTEM_SETTINGS_PATH
    effect: |
      Overrides the path to the system-wide override settings file.
  - name: GEMINI_CLI_SYSTEM_DEFAULTS_PATH
    effect: |
      Overrides the path to the system-wide baseline defaults file.
  - name: GEMINI_CLI_TRUSTED_FOLDERS_PATH
    effect: |
      Overrides the path to the trusted-folders JSON file.
  - name: GEMINI_CLI_TRUST_WORKSPACE
    effect: |
      Set to `true` to trust the current workspace for the session without an
      interactive dialog.
  - name: GEMINI_SANDBOX
    effect: |
      Enables tool sandboxing (e.g., `true`, `docker`, `runsc`). Applies to
      built-in tool execution, not to MCP server subprocesses.
  - name: SANDBOX_MOUNTS
    effect: |
      Comma-separated extra host mounts when sandboxing is enabled.
  - name: SANDBOX_FLAGS
    effect: |
      Extra flags passed to the container runtime when sandboxing is enabled.
server_schema:
  transports: ["stdio", "sse", "streamable_http"]
  command_fields: ["command", "args", "env", "cwd", "timeout", "trust", "description", "includeTools", "excludeTools"]
  http_fields: ["url", "httpUrl", "headers", "timeout", "trust", "description", "includeTools", "excludeTools", "oauth", "authProviderType", "targetAudience", "targetServiceAccount"]
  env_shape: |
    `env` is an object mapping variable names to string values. Values support
    `$VAR`, `${VAR}`, `${VAR:-default}` expansion on all platforms, and `%VAR%`
    on Windows. Undefined variables without a default expand to an empty string.
  auth_shape: |
    Remote servers support OAuth 2.0 (`oauth` object with dynamic discovery,
    pre-registered `clientId`/`clientSecret`, `scopes`, `redirectUri`, etc.),
    Google Application Default Credentials (`authProviderType: google_credentials`),
    service-account impersonation for IAP (`authProviderType: service_account_impersonation`),
    or static `headers`. OAuth tokens are stored in
    `~/.gemini/mcp-oauth-tokens.json`.
  notes: |
    Server id is the map key under `mcpServers`. Transport is normally inferred
    from the presence of `command`, `url`, or `httpUrl`; `type` may be provided
    explicitly. The `includeTools` and `excludeTools` arrays use camelCase in
    the settings schema and official docs (`includeTools`/`excludeTools`), not
    hyphenated names.
server_capabilities:
  tools: full
  resources: partial
  prompts: full
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    Tools are fully exposed to the model with FQNs like `mcp_{serverName}_{toolName}`.
    Resources are discovered and surfaced in `/mcp` and can be referenced with
    `@server://resource/path`; the model can also invoke built-in
    `list_mcp_resources` and `read_mcp_resource`. Prompts are exposed as slash
    commands. Dynamic `list_changed` refresh is not documented.
client_capabilities:
  roots: none
  sampling: none
  elicitation: none
  notes: |
    The public documentation does not describe support for MCP roots/list,
    sampling/createMessage, or elicitation/completion.
tool_surface:
  discovery: |
    `tools/list` is called for each configured server at startup. Tool names are
    sanitized to Gemini API rules and namespaced as `mcp_{serverName}_{toolName}`.
    Conflicts across servers are resolved by last-registration-wins.
  filtering: |
    Per-server `includeTools`/`excludeTools`; global `mcp.allowed` and
    `mcp.excluded` server lists; `--allowed-mcp-server-names` at runtime; and
    Policy Engine TOML rules using `mcpName` or FQN wildcards. A `deny` policy
    removes the tool from the model's memory entirely.
  approval: |
    Per-server `trust: true` bypasses confirmation. Otherwise the default policy
    engine prompts the user. In non-interactive mode an `ask_user` decision is
    treated as `deny`.
  result_handling: |
    Tool results are formatted into `llmContent` for the model and
    `returnDisplay` for the user. No native MCP result sanitization or size
    limits are documented.
  annotations_trusted: |
    Tool annotations are not described as trusted policy inputs.
  notes: |
    Server names should avoid underscores because the policy parser splits FQNs
    on the first underscore after the `mcp_` prefix.
resource_surface:
  supported: true
  uri_schemes: []
  templates: unknown
  subscriptions: false
  exposure_model: |
    Resources discovered via `resources/list` appear in `/mcp`. Users reference
    them with `@server://resource/path`, which triggers `resources/read`. The
    model can also use the built-in `list_mcp_resources` and `read_mcp_resource`
    tools.
  notes: |
    Resource templates and subscriptions are not documented.
prompt_surface:
  supported: true
  invocation: |
    Discovered prompts appear as slash commands (e.g., `/poem-writer`). Arguments
    are supplied as named flags (`--title="X"`) or positionally.
  arguments: |
    Named flags or positional values are passed to the server's `prompts/get`.
  exposure_model: |
    User-invoked slash commands only; prompts are not autonomously invoked by
    the model.
  notes: |
    Prompt list changes require restarting the session or running `/mcp reload`.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: deep
  notes: |
    Claudine can read `~/.gemini/settings.json` and `.gemini/settings.json`,
    normalize server definitions, and write them back. `mcpServers` objects are
    merged across system defaults, user, workspace, and system overrides; when
    the same server name appears at multiple levels, the highest-precedence
    definition wins (system overrides > workspace > user > system defaults).
    `gemini mcp add/remove/enable/disable` provide a supported apply path.
runtime_injection:
  supported: false
  mechanism: |
    Gemini CLI has no native flag or environment variable for loading MCP
    servers for a single run without mutating persistent config. Claudine
    wrappers can emulate one-run injection by launching `gemini` with a shadow
    HOME directory containing `.gemini/settings.json`.
  limitations: |
    A shadow HOME replaces the user's normal `~/.gemini` for that process, so
    sidecars such as `mcp-oauth-tokens.json` and `mcp-server-enablement.json`
    must be copied into the shadow home if they are needed. OAuth flows still
    require an interactive browser.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens are stored in `~/.gemini/mcp-oauth-tokens.json` and refreshed
    automatically when refresh tokens are available.
  token_scope: |
    Per remote server URL. Client secrets may be supplied via the `oauth` object
    or CLI flags.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` object or via env-var
    expansion inside config values. The host process environment is inherited
    but sensitive patterns (`*TOKEN*`, `*SECRET*`, `*PASSWORD*`, `*KEY*`,
    `*AUTH*`, `*CREDENTIAL*`) are redacted unless explicitly listed in `env`.
  notes: |
    OAuth flows require a local browser and redirect listener on
    `http://localhost:<port>/oauth/callback`; they do not work in headless
    environments. Static `headers.Authorization` is supported but discouraged
    for shared repo config.
security:
  tool_filtering: |
    Per-server `includeTools`/`excludeTools`; global `mcp.allowed`/`mcp.excluded`;
    `--allowed-mcp-server-names`; and Policy Engine rules using `mcpName` or
    FQN wildcards.
  server_trust: |
    When Folder Trust is enabled, project-scope MCP servers do not connect in
    untrusted folders. `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true`
    bypasses the dialog for the session. System-level `mcpServers` and
    `mcp.allowed` can enforce a corporate catalog.
  env_sanitization: |
    Host environment variables matching sensitive patterns are redacted from
    MCP stdio server subprocesses. Only variables explicitly listed in a
    server's `env` object are guaranteed to be passed through.
  sandbox_interaction: |
    MCP stdio server subprocesses run as ordinary local processes and are not
    isolated by Gemini CLI's tool sandbox, which applies to built-in tools such
    as shell and file operations.
  response_filtering: |
    No native MCP response sanitization is documented. Claudine's `protect`
    layer should treat MCP tool results as untrusted.
  notes: |
    OAuth tokens are stored in the user home, not the OS keychain. Admin
    policies in `/etc/gemini-cli/policies` (Linux), `/Library/Application
    Support/GeminiCli/policies` (macOS), or `C:\ProgramData\gemini-cli\policies`
    (Windows) can enforce organization-wide rules.
gaps:
  - |
    The official docs do not state which MCP protocol version Gemini CLI
    implements, only an indirect link to the 2025-06-18 schema.
  - |
    Dynamic capability refresh (`list_changed` notifications) is not documented.
  - |
    Auto-reconnect behavior for failed SSE/HTTP servers is not documented.
  - |
    Resource URI templates and subscriptions are not documented.
  - |
    Roots, sampling, and elicitation support are not documented.
  - |
    Gemini CLI provides no native one-run MCP config flag or environment
    variable.
  - |
    Whether MCP stdio server subprocesses are included in tool sandboxing is
    not explicitly stated.
  - |
    Native MCP response size limits and prompt-injection filtering are not
    documented.
changes: []
requires_claudine_update: true
reason: |
  Claudine's Gemini import/export currently uses non-standard hyphenated field
  names (`include-tools`/`exclude-tools`) and omits documented server fields
  such as `cwd`, `headers`, `oauth`, `authProviderType`, `httpUrl`, `type`,
  `description`, `timeout`, and `trust`. The importer also reads a
  non-documented `transport` key instead of the documented `type`/`url`/`httpUrl`
  inference. Update import/export/runtime injection to match the Gemini
  settings.schema.json and MCP server docs.
---

# MCP Support in Gemini CLI

## Overview

Gemini CLI supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io) as a first-class extension mechanism. MCP servers add external tools, resources, and prompt templates to the Gemini model. Servers can be local stdio processes, remote SSE endpoints, or Streamable HTTP endpoints. Configuration is persisted in JSON settings files, managed through both `gemini mcp` CLI commands and interactive `/mcp` slash commands.

This document maps Gemini CLI's MCP behavior to the schema used by Claudine's MCP catalog and provider wrappers.

## Protocol and Transports

Gemini CLI supports three MCP transports:

| Transport | Configuration keys | Status |
| :-------- | :----------------- | :----- |
| `stdio` | `command` (and `args`, `env`, `cwd`) | Primary |
| `sse` | `url` | Supported |
| `streamable_http` | `httpUrl` | Supported |

Transport selection is normally inferred from the presence of `command`, `url`, or `httpUrl`. An optional `type` field can be set to `"stdio"`, `"sse"`, or `"http"` to be explicit.

Lifecycle behavior:

- Servers are discovered once at startup.
- Stdio servers are spawned as local child processes.
- SSE and Streamable HTTP servers open remote connections.
- Failed connections mark the server as `DISCONNECTED`.
- The public documentation does not describe automatic reconnection or dynamic `list_changed` refresh.

The documentation does not explicitly state an MCP protocol version date; the only version reference is a link to the [2025-06-18 schema](https://modelcontextprotocol.io/specification/2025-06-18/schema#initializeresult) in the "Instructions" section.

## Configuration

MCP servers are configured in the top-level `mcpServers` object of a `settings.json` file. There are four configuration layers:

| Layer | File | Precedence |
| :---- | :--- | :--------- |
| System defaults | `/etc/gemini-cli/system-defaults.json` (Linux), `/Library/Application Support/GeminiCli/system-defaults.json` (macOS), `C:\ProgramData\gemini-cli\system-defaults.json` (Windows) | Lowest |
| User | `~/.gemini/settings.json` | Overrides system defaults |
| Workspace (project) | `.gemini/settings.json` | Overrides user settings |
| System overrides | `/etc/gemini-cli/settings.json` (Linux), `/Library/Application Support/GeminiCli/settings.json` (macOS), `C:\ProgramData\gemini-cli\settings.json` (Windows) | Highest |

For single-value settings, higher layers override lower layers. For the `mcpServers` object, definitions are merged; if the same server name appears at multiple layers, the highest-precedence definition wins. The `GEMINI_CLI_SYSTEM_SETTINGS_PATH` and `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` environment variables can relocate the system files.

Additional sidecar files:

- `~/.gemini/mcp-server-enablement.json` — per-server enabled/disabled state.
- `~/.gemini/mcp-oauth-tokens.json` — stored OAuth tokens for remote servers.
- `~/.gemini/trustedFolders.json` — folder-trust decisions.

Global MCP policy can be set with the top-level `mcp` object:

- `mcp.serverCommand` — a global command to start a server.
- `mcp.allowed` — server-name allowlist.
- `mcp.excluded` — server-name denylist.

## Server Definition Shape

A server definition under `mcpServers` looks like:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
      "env": { "SOME_VAR": "$SOME_VAR" },
      "cwd": "./mcp-servers",
      "timeout": 600000,
      "trust": false,
      "includeTools": ["read_file"],
      "excludeTools": ["delete_file"]
    },
    "remote": {
      "httpUrl": "https://api.example.com/mcp",
      "headers": { "Authorization": "Bearer ${API_TOKEN}" },
      "oauth": { "scopes": ["read"] },
      "timeout": 30000
    }
  }
}
```

### Field reference

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `command` | stdio | Executable to spawn |
| `args` | stdio | Argument array |
| `env` | stdio | Environment variable map |
| `cwd` | stdio | Working directory |
| `url` | sse | SSE endpoint URL |
| `httpUrl` | streamable_http | Streamable HTTP endpoint URL |
| `headers` | remote | Static HTTP header map |
| `timeout` | all | Request timeout in ms (default 600,000) |
| `trust` | all | Bypass tool-call confirmation prompts |
| `description` | all | Human-readable description |
| `includeTools` | all | Tool allowlist |
| `excludeTools` | all | Tool denylist (takes precedence over `includeTools`) |
| `oauth` | remote | OAuth configuration object |
| `authProviderType` | remote | `dynamic_discovery`, `google_credentials`, or `service_account_impersonation` |
| `targetAudience` | remote | IAP OAuth client ID |
| `targetServiceAccount` | remote | Service account email to impersonate |

### Environment variable expansion

String values in `env`, `command`, `args`, `url`, `httpUrl`, and `headers` support `$VAR`, `${VAR}`, and `${VAR:-default}` expansion on all platforms, and `%VAR%` on Windows. Undefined variables without a default expand to an empty string.

## Tools, Resources, and Prompts

### Tools

MCP tools are fully exposed to the Gemini model. Each tool is registered with a fully qualified name:

```
mcp_{serverName}_{toolName}
```

Tool discovery:

- `tools/list` is called for each server at startup.
- Tool schemas are sanitized for the Gemini API.
- Name collisions are resolved by last-registration-wins.

Tool approval:

- `trust: true` on a server bypasses confirmation.
- The Policy Engine can `allow`, `deny`, or `ask_user` per server/tool.
- In non-interactive mode, `ask_user` is treated as `deny`.

Tool filtering:

- Per-server `includeTools`/`excludeTools`.
- Global `mcp.allowed`/`mcp.excluded` server lists.
- `--allowed-mcp-server-names` at runtime.
- Policy Engine TOML rules using `mcpName` or FQN wildcards such as `mcp_server_*`.

### Resources

Gemini CLI discovers and surfaces MCP resources:

- `resources/list` results appear in `/mcp`.
- Users reference resources with `@server://resource/path` syntax.
- The CLI calls `resources/read` and injects the content into the conversation.
- Built-in tools `list_mcp_resources` and `read_mcp_resource` are also available to the model.

Resource templates and subscriptions are not documented.

### Prompts

MCP prompts are exposed as slash commands:

- A prompt named `poem-writer` is invoked as `/poem-writer`.
- Arguments are passed as named flags (`--title="X" --mood="Y"`) or positionally.
- The CLI calls `prompts/get` and sends the resulting prompt to the model.

Prompts are user-invoked only; the model does not autonomously invoke them.

## Roots, Sampling, and Elicitation

The public Gemini CLI documentation does not describe support for:

- MCP `roots/list` or workspace-root boundaries.
- MCP `sampling/createMessage` (server-requested LLM calls).
- MCP `completion/complete` (elicitation / structured user input).

Claudine should treat these client capabilities as **none** until documentation proves otherwise.

## Import, Export, and Sync

Claudine can treat Gemini CLI as an `import_sync` provider:

- **Import**: read `~/.gemini/settings.json` and `.gemini/settings.json` and normalize `mcpServers` entries into the MCP catalog.
- **Export**: write provider-shaped JSON back to those files.
- **Apply**: use `gemini mcp add`, `gemini mcp remove`, `gemini mcp enable`, and `gemini mcp disable` to mutate configuration through the supported CLI.

Merge semantics:

- `mcpServers` objects are merged across system defaults, user, workspace, and system overrides.
- Same-name servers are replaced whole from the highest-precedence layer.
- Extension-bundled servers can be overridden by user/workspace `settings.json`.

## Runtime Injection

Gemini CLI does **not** provide a native flag or environment variable for loading MCP servers for a single run without mutating persistent config. The supported non-persistent option is `gemini mcp enable <name> --session` / `gemini mcp disable <name> --session`, which only affects enablement state.

For Claudine wrappers, one-run injection can be emulated by:

1. Creating a shadow HOME directory.
2. Writing the desired `.gemini/settings.json` with the `mcpServers` object.
3. Launching `gemini` with `HOME` (or equivalent) pointing at the shadow directory.
4. Copying sidecars such as `mcp-oauth-tokens.json` and `mcp-server-enablement.json` if needed.

Limitations:

- The shadow HOME replaces the user's normal config for that process.
- OAuth authentication still requires an interactive browser.
- `--allowed-mcp-server-names` can restrict which injected servers connect.

## Authorization and Credentials

Gemini CLI supports multiple authentication patterns for remote MCP servers:

| Pattern | Where configured | Credential storage |
| :------ | :--------------- | :----------------- |
| Static header | `headers` | In config file (not recommended for shared repos) |
| OAuth dynamic discovery | `oauth` object (omit details) | `~/.gemini/mcp-oauth-tokens.json` |
| OAuth pre-registered | `oauth.clientId` + client secret | `~/.gemini/mcp-oauth-tokens.json` |
| Google credentials | `authProviderType: google_credentials` | Application Default Credentials |
| Service account impersonation | `authProviderType: service_account_impersonation` + `targetAudience`/`targetServiceAccount` | Application Default Credentials |

OAuth details:

- `/mcp auth [server-name]` runs the OAuth flow interactively.
- Tokens are stored in `~/.gemini/mcp-oauth-tokens.json` and refreshed automatically.
- OAuth requires a browser and a localhost redirect; it does not work in headless mode.

For stdio servers, secrets should be passed through the per-server `env` object or via env-var expansion, not committed in config files.

## Security Model

### Trust and allowlisting

- When Folder Trust is enabled, project-scope `.gemini/settings.json` is ignored in untrusted folders, and MCP servers do not connect.
- `gemini mcp list` shows stdio servers as `Disconnected` in untrusted folders.
- `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` bypasses the trust dialog for the session.
- `mcp.allowed` can enforce a server allowlist; omitting it allows any configured server.

### Environment and sandboxing

- Stdio servers inherit the host process environment minus sensitive patterns (`*TOKEN*`, `*SECRET*`, `*PASSWORD*`, `*KEY*`, `*AUTH*`, `*CREDENTIAL*`).
- Variables must be explicitly listed in a server's `env` object to guarantee they are passed through.
- The built-in tool sandbox (Seatbelt, Docker, gVisor, etc.) applies to built-in tool execution, not to MCP stdio server subprocesses.

### Response handling

- No native MCP response sanitization is documented.
- Claudine's `protect` layer should scan MCP tool results defensively.

## Mode-Specific Behavior

### Interactive mode

- `/mcp` opens a panel showing connected servers, tools, resources, and prompts.
- `/mcp auth` runs OAuth flows.
- `/mcp enable`, `/mcp disable`, `/mcp reload` manage servers during the session.
- Folder Trust dialogs appear for untrusted projects.

### Headless / non-interactive mode (`-p`)

- OAuth flows cannot complete; pre-authenticated or header-based servers are required.
- If Folder Trust is enabled and the folder is untrusted, the CLI exits with `FatalUntrustedWorkspaceError` unless `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` is set.
- `ask_user` policy decisions are treated as `deny`.
- `--allowed-mcp-server-names` can restrict which configured servers connect.

### ACP mode (`--acp`)

- In ACP mode the client can register its own MCP server during the `initialize` handshake.
- Gemini CLI connects to that server, discovers tools, and exposes them to the model.
- This is a distinct path from persistent `settings.json` configuration.

## Failure Modes

| Failure | Behavior |
| :------ | :--------- |
| Server fails to start | Marked `DISCONNECTED` in `/mcp` and `gemini mcp list`; connection errors are silent at startup unless diagnostics are triggered |
| Stdio server exits | Not automatically reconnected; `/mcp reload` or restart required |
| HTTP/SSE transient error | No documented auto-retry or reconnect behavior |
| Auth error (401) | OAuth flow triggered if configured; otherwise server marked as needing auth |
| Tool timeout | Aborted after per-server `timeout` (default 10 minutes) |
| No tools discovered | Connection closed; server may show as connected but provide no tools |
| Untrusted project | MCP servers do not connect; stdio servers shown as `Disconnected` |
| STDERR | Captured and logged; INFO-level messages are filtered |

## Gaps

- Explicit MCP protocol version date is not stated.
- Dynamic `list_changed` capability refresh is not documented.
- Auto-reconnect behavior for remote servers is not documented.
- Resource templates and subscriptions are not documented.
- Roots, sampling, and elicitation support are not documented.
- No native one-run runtime injection mechanism exists.
- Whether MCP stdio servers run inside the tool sandbox is not explicit.
- Native MCP response size limits and prompt-injection filtering are not documented.

## Claudine Integration Notes

- Treat Gemini CLI as `support: import_sync`. Claudine can read and write `~/.gemini/settings.json` and `.gemini/settings.json`, and can apply changes through `gemini mcp add/remove/enable/disable`.
- Map Claudine's normalized catalog to Gemini's `mcpServers` object, preserving the documented field names:
  - Use `includeTools` and `excludeTools` (camelCase), not hyphenated names.
  - Support `cwd`, `headers`, `oauth`, `authProviderType`, `httpUrl`, `type`, `description`, `timeout`, and `trust`.
  - Do not rely on a non-documented `transport` key; infer or emit `type`/`url`/`httpUrl`/`command`.
- For one-run wrappers, use a shadow HOME containing `.gemini/settings.json`; there is no native `--mcp-config` equivalent.
- Honor folder trust: do not assume `.gemini/settings.json` servers are active until the workspace is trusted or trust is bypassed.
- Defensively scan MCP tool results in the `protect` layer; Gemini CLI does not provide native response sanitization.

## Sources

- [Gemini CLI MCP server integration](https://www.geminicli.com/docs/tools/mcp-server/)
- [Gemini CLI MCP resource tools](https://www.geminicli.com/docs/tools/mcp-resources/)
- [Gemini CLI configuration reference](https://www.geminicli.com/docs/reference/configuration/)
- [Gemini CLI command reference](https://www.geminicli.com/docs/reference/commands/)
- [Gemini CLI policy engine](https://www.geminicli.com/docs/reference/policy-engine/)
- [Gemini CLI sandboxing](https://www.geminicli.com/docs/cli/sandbox/)
- [Gemini CLI trusted folders](https://www.geminicli.com/docs/cli/trusted-folders/)
- [Gemini CLI enterprise configuration](https://www.geminicli.com/docs/cli/enterprise/)
- [Gemini CLI ACP mode](https://www.geminicli.com/docs/cli/acp-mode/)
- [Gemini CLI settings.schema.json](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json)
- [Gemini CLI source repository](https://github.com/google-gemini/gemini-cli)
