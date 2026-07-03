---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp/
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http, sse]
  lifecycle: |
    Qwen Code discovers MCP servers progressively. In interactive mode the UI
    appears immediately and an MCP status pill shows how many servers have
    finished their discovery handshake; tools become available to the model as
    each server settles. In non-interactive mode (--prompt, stream-json, ACP)
    the CLI waits for MCP discovery to settle before sending the first prompt.
    stdio servers are spawned as local subprocesses at session start. HTTP and
    SSE transports connect at startup. Set QWEN_CODE_LEGACY_MCP_BLOCKING=1 to
    restore the old blocking discovery behavior.
  notes: |
    The documentation does not state an explicit MCP protocol version date.
    HTTP (`httpUrl`) is the recommended remote transport; SSE (`url`) is
    described as legacy/deprecated. The implementation supports stdio, SSE, and
    streamable HTTP.
config_files:
  - os: all
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: |
      Default user scope. MCP servers live in the top-level `mcpServers`
      object. The base directory defaults to `~/.qwen` and can be overridden
      with the `QWEN_HOME` environment variable.

  - os: all
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: |
      Project-scoped MCP servers. Loaded only when the workspace is trusted
      (if `security.folderTrust.enabled` is true). Project settings override
      user settings.

  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    format: json
    notes: |
      System-wide defaults with the lowest precedence. Path can be overridden
      with `QWEN_CODE_SYSTEM_DEFAULTS_PATH`.

  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    format: json
    notes: |
      Linux system-wide defaults. Path can be overridden with
      `QWEN_CODE_SYSTEM_DEFAULTS_PATH`.

  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    format: json
    notes: |
      Windows system-wide defaults. Path can be overridden with
      `QWEN_CODE_SYSTEM_DEFAULTS_PATH`.

  - os: macos
    scope: managed
    path: "/Library/Application Support/QwenCode/settings.json"
    format: json
    notes: |
      System-wide override settings with highest file precedence. Path can be
      overridden with `QWEN_CODE_SYSTEM_SETTINGS_PATH`.

  - os: linux
    scope: managed
    path: "/etc/qwen-code/settings.json"
    format: json
    notes: |
      Linux system-wide override settings. Path can be overridden with
      `QWEN_CODE_SYSTEM_SETTINGS_PATH`.

  - os: windows
    scope: managed
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    format: json
    notes: |
      Windows system-wide override settings. Path can be overridden with
      `QWEN_CODE_SYSTEM_SETTINGS_PATH`.

  - os: all
    scope: user
    path: "~/.qwen/trustedFolders.json"
    format: json
    notes: |
      Stores folder trust decisions when `security.folderTrust.enabled` is
      true. Untrusted folders ignore project `.qwen/settings.json`, which
      disables project MCP servers.

cli_params:
  - flag: "qwen mcp add [options] <name> <commandOrUrl> [args...]"
    description: "Adds a persistent MCP server to user or project settings."
    example: "qwen mcp add --transport http my-server http://localhost:3000/mcp"

  - flag: "qwen mcp remove <name>"
    description: "Removes a configured MCP server from settings."
    example: "qwen mcp remove my-server"

  - flag: "--scope user|project"
    description: "Target scope for `qwen mcp add` and `qwen mcp remove`."
    example: "qwen mcp add --scope project --transport sse my-server https://example.com/sse"

  - flag: "--transport stdio|sse|http"
    description: "Selects the MCP transport when adding a server."
    example: "qwen mcp add --transport stdio fs -- npx -y @modelcontextprotocol/server-filesystem ."

  - flag: "-e, --env KEY=VALUE"
    description: "Sets an environment variable on a stdio server."
    example: "qwen mcp add my-server -e API_KEY=abc -- /path/to/server"

  - flag: "-H, --header 'Name: Value'"
    description: "Sets a static HTTP header on an SSE or HTTP server."
    example: "qwen mcp add --transport http secure https://api.example.com/mcp -H 'Authorization: Bearer token'"

  - flag: "--timeout <ms>"
    description: "Per-server request/connection timeout in milliseconds."
    example: "qwen mcp add --transport http slow https://example.com/mcp --timeout 10000"

  - flag: "--trust"
    description: "Bypasses all tool-call confirmation prompts for the server."
    example: "qwen mcp add --trust my-server -- /path/to/server"

  - flag: "--include-tools <names>"
    description: "Comma-separated allowlist of tool names from the server."
    example: "qwen mcp add --include-tools read,search my-server -- /path/to/server"

  - flag: "--exclude-tools <names>"
    description: "Comma-separated denylist of tool names from the server."
    example: "qwen mcp add --exclude-tools dangerous my-server -- /path/to/server"

  - flag: "--oauth-client-id, --oauth-client-secret, --oauth-redirect-uri, --oauth-authorization-url, --oauth-token-url, --oauth-scopes"
    description: "OAuth options for SSE/HTTP servers."
    example: "qwen mcp add --transport sse oauth https://api.example.com/sse --oauth-client-id id --oauth-redirect-uri https://example.com/callback"

  - flag: "--safe-mode"
    description: "Disables MCP servers (along with hooks, extensions, skills, etc.) for the run."
    example: "qwen -p '...' --safe-mode"

  - flag: "--allowed-mcp-server-names <names>"
    description: "Comma-separated allowlist of MCP server names to connect to. Overrides `mcp.allowed`/`mcp.excluded`."
    example: "qwen --allowed-mcp-server-names fs,github"

env_vars:
  - name: QWEN_HOME
    effect: |
      Customizes the global configuration directory (default `~/.qwen`).
      Affects where `~/.qwen/settings.json` and OAuth token files are located.

  - name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
    effect: |
      Overrides the path to the system defaults settings file.

  - name: QWEN_CODE_SYSTEM_SETTINGS_PATH
    effect: |
      Overrides the path to the system override settings file.

  - name: QWEN_CODE_LEGACY_MCP_BLOCKING
    effect: |
      Set to `1` to make the CLI wait synchronously for every configured MCP
      server's discovery handshake before returning from `Config.initialize()`.

  - name: QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE
    effect: |
      When set to `true`, OAuth tokens are stored in the OS keychain where
      available, or in `~/.qwen/mcp-oauth-tokens-v2.json` with AES-256-GCM
      encryption. Default storage is plaintext with mode 0600.

  - name: QWEN_CODE_SAFE_MODE
    effect: |
      Set to `true` to disable MCP servers (and other customizations) for the
      run.

  - name: QWEN_SANDBOX
    effect: |
      Enables sandboxing for the session. MCP server executables must be
      available inside the chosen sandbox environment.

server_schema:
  transports: ["stdio", "streamable_http", "sse"]
  command_fields: ["command", "args", "env", "cwd", "timeout", "trust", "includeTools", "excludeTools", "description"]
  http_fields: ["httpUrl", "url", "headers", "timeout", "trust", "includeTools", "excludeTools", "description", "oauth", "authProviderType", "targetAudience", "targetServiceAccount"]
  env_shape: |
    `env` is an object mapping variable names to string values. Values support
    `$VAR_NAME` and `${VAR_NAME}` expansion from the process environment when
    settings are loaded.
  auth_shape: |
    HTTP/SSE servers support OAuth 2.0 via an `oauth` object
    (`enabled`, `clientId`, `clientSecret`, `authorizationUrl`, `tokenUrl`,
    `scopes`, `redirectUri`, `tokenParamName`, `audiences`) or via
    `authProviderType` (`dynamic_discovery`, `google_credentials`,
    `service_account_impersonation`). Static authentication can also be
    provided through `headers`. stdio servers receive credentials only through
    the per-server `env` object or environment expansion.
  notes: |
    Server id is the map key under `mcpServers`. At least one of `command`,
    `url`, or `httpUrl` must be provided. If multiple are specified the order
    of precedence is `httpUrl`, then `url`, then `command`. Tool name
    conflicts are resolved by prefixing the later server name
    (`serverName__toolName`).

server_capabilities:
  tools: full
  resources: partial
  prompts: full
  tool_list_changed: unknown
  resource_subscribe: unknown
  resource_list_changed: unknown
  prompt_list_changed: unknown
  notes: |
    Tools are fully exposed to the model. Resources are user-selectable via
    the `/mcp` dialog or `@server:uri` references, but the model cannot
    autonomously read arbitrary resources. Prompts are exposed as slash
    commands. The documentation does not mention dynamic `list_changed`
    handling, subscriptions, or capability-strict discovery (Qwen Code
    attempts `prompts/list` and `resources/list` even when the capability is
    not declared).

client_capabilities:
  roots: unknown
  sampling: unknown
  elicitation: unknown
  notes: |
    The public documentation does not describe `roots/list`,
    `sampling/createMessage`, or elicitation/completion support.

tool_surface:
  discovery: |
    `discoverMcpTools()` iterates configured servers, connects via the chosen
    transport, calls the MCP tool listing endpoint, sanitizes schemas, and
    registers tools in the global registry. Discovery is progressive in
    interactive mode and blocking in non-interactive mode.
  filtering: |
    Per-server `includeTools`/`excludeTools` restrict the exposed tool surface.
    Global `mcp.allowed`/`mcp.excluded` (with glob support) and the
    `--allowed-mcp-server-names` flag filter which configured servers are
    connected at all. `permissions.deny` can also block tools by the
    `mcp__<server>` pattern.
  approval: |
    MCP tools use the same confirmation model as built-in tools unless the
    server is marked `trust: true` or the user chooses "Always allow this
    tool/server" after a prompt.
  result_handling: |
    Text, image, audio, `resource`, and `resource_link` content blocks are
    processed and passed to the model. Tool errors surface through the normal
    function-response path. Large tool outputs from built-in tools are
    truncated according to `tools.truncateToolOutputThreshold`, but MCP tool
    results are passed through as context.
  annotations_trusted: |
    Not documented. Tool annotations are not described as a trusted policy
    surface.
  notes: |
    Tool schemas are sanitized for API compatibility (`$schema` and
    `additionalProperties` removed, `anyOf` defaults stripped, names truncated
    to 63 characters). Resource reads are disabled in untrusted folders.

resource_surface:
  supported: true
  uri_schemes: ["unknown"]
  templates: unknown
  subscriptions: unknown
  exposure_model: |
    Resources are discovered per server and exposed in the `/mcp` management
    dialog. Users can inject a resource by typing `@server:uri`; the content
    is read and appended to the message. Resource reads are disabled in
    untrusted folders.
  notes: |
    The documentation does not describe URI templates, subscription support,
    or which URI schemes are accepted beyond the examples shown in the UI.

prompt_surface:
  supported: true
  invocation: |
    MCP prompts appear as slash commands prefixed with `MCP: <server>`. They
    are invoked as `/promptName --arg=value` or with positional arguments.
  arguments: |
    Arguments are passed on the command line; `/promptName help` shows the
    declared arguments.
  exposure_model: |
    User-initiated only. The model does not invoke prompts autonomously.
  notes: |
    Qwen Code attempts `prompts/list` even when the server does not declare
    the `prompts` capability, so lenient servers still surface prompts.

sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: deep
  notes: |
    Claudine can read `~/.qwen/settings.json` and `.qwen/settings.json`,
    normalize the `mcpServers` object into its catalog, and write provider-
    shaped JSON back. Changes can also be applied through `qwen mcp add` and
    `qwen mcp remove`. Because MCP servers are stored inside the general
    `settings.json` file, Claudine must preserve unrelated top-level settings
    when rewriting the file. Settings merge across scopes (system defaults,
    user, project, system override, env, CLI args) with higher precedence
    sources overriding lower precedence; per-server entries from a higher
    scope replace matching entries from lower scopes.

runtime_injection:
  supported: false
  mechanism: |
    None documented. Qwen Code loads MCP servers only from persistent
    `settings.json` files (`mcpServers`) and from `qwen mcp add/remove`.
  limitations: |
    There is no equivalent to `--mcp-config` or inline config content. The
    closest alternatives are to write a temporary settings file and point
    `QWEN_CODE_SYSTEM_DEFAULTS_PATH`/`QWEN_CODE_SYSTEM_SETTINGS_PATH` at it,
    or to mutate user/project settings before launch; both require file I/O
    and do not provide safe one-run injection.

authorization:
  oauth: true
  credential_storage: |
    OAuth tokens are stored in `~/.qwen/mcp-oauth-tokens.json` (plaintext,
    mode 0600) by default. If `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true`,
    Qwen Code uses keychain-backed storage where available, or
    `~/.qwen/mcp-oauth-tokens-v2.json` with AES-256-GCM encryption.
  token_scope: |
    Per configured remote server. Tokens are refreshed automatically when a
    refresh token is available and validated before each connection attempt.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` object or via `$VAR`
    expansion sourced from the process environment. Process environment is
    otherwise inherited.
  notes: |
    Static `headers.Authorization` is supported but credentials end up in
    config files. OAuth requires a browser and does not work in headless or
    remote deployments unless a publicly accessible `redirectUri` is
    configured. Google credentials and service-account impersonation are also
    supported for IAP-protected services.

security:
  tool_filtering: |
    Per-server `includeTools`/`excludeTools`; global `mcp.allowed`/
    `mcp.excluded` (with globs); `--allowed-mcp-server-names`; and
    `permissions.deny` rules such as `mcp__<server>` or
    `mcp__<server>__<tool>`.
  server_trust: |
    Per-server `trust: true` bypasses all tool-call confirmations. Project
    `.qwen/settings.json` is ignored when folder trust is enabled and the
    folder is untrusted. `QWEN_CODE_SAFE_MODE` disables MCP servers entirely.
  env_sanitization: |
    Each stdio server receives its explicit `env` map plus inherited process
    environment. Values support `$VAR`/`${VAR}` expansion. No documented
    credential-scrubbing pass is applied specifically to MCP subprocesses.
  sandbox_interaction: |
    Sandboxing applies to built-in shell/write/edit tools. MCP server
    subprocesses run separately and may fail or need to be available inside
    the sandbox; the Docker sandbox mounts the workspace and `~/.qwen`, while
    macOS Seatbelt may restrict server executable paths.
  response_filtering: |
    No native MCP response sanitization is documented. Tool schemas are
    sanitized for API compatibility, but rich content returned by tools is
    passed to the model.
  notes: |
    OAuth tokens are plaintext by default; enable encrypted storage on shared
    machines. Administrators can lock down the MCP surface with system-level
    settings and `--allowed-mcp-server-names`.

gaps:
  - |
    No explicit MCP protocol version date is documented.
  - |
    No runtime injection mechanism for one-run MCP server configuration.
  - |
    `roots/list`, `sampling/createMessage`, and elicitation support are not
    documented.
  - |
    Dynamic capability notifications (`tools/list_changed`, etc.) and
    resource subscriptions are not described.
  - |
    Default OAuth token storage is plaintext; users must opt into encrypted
    storage.
  - |
    Interaction between MCP servers and the sandbox is only partially
    documented.

changes: []
requires_claudine_update: true
reason: |
  Qwen Code has graduated from "no MCP support" to a full persistent-config
  MCP implementation. Claudine's MCP catalog should classify Qwen as
  `import_sync`, add import/export/apply paths for `~/.qwen/settings.json`
  and `.qwen/settings.json`, and update wrapper guidance accordingly.
---

# MCP Support in Qwen CLI

## Overview

Qwen CLI (`qwen`) supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/introduction) as a persistent configuration-driven integration. MCP servers are defined under the `mcpServers` object in `settings.json` and can be managed either by editing the JSON file directly or through the `qwen mcp add/remove` commands. Once discovered, MCP tools become available to the model, prompts surface as slash commands, and resources can be injected via `@server:uri` references.

This document maps Qwen CLI's MCP behavior to the schema used by Claudine's MCP catalog and provider wrappers.

## Protocol and Transports

Qwen CLI supports three MCP transports:

| Transport | JSON field | Status |
| :-------- | :--------- | :----- |
| stdio | `command` (+ `args`) | Local process |
| Streamable HTTP | `httpUrl` | Recommended for remote |
| SSE | `url` | Legacy/deprecated |

The docs recommend HTTP for remote servers and note that SSE is legacy. No explicit MCP protocol version date is stated.

Discovery is progressive: in interactive mode the UI appears before all servers have connected, and a status pill shows `N/M MCP servers ready`; in non-interactive mode the CLI waits for discovery to settle before the first prompt. Set `QWEN_CODE_LEGACY_MCP_BLOCKING=1` to restore synchronous discovery.

## Configuration

MCP servers are configured inside the general `settings.json` hierarchy:

| Scope | File | Precedence |
| :---- | :--- | :--------- |
| System defaults | `/etc/qwen-code/system-defaults.json`, `C:\ProgramData\qwen-code\system-defaults.json`, `/Library/Application Support/QwenCode/system-defaults.json` | Lowest |
| User | `~/.qwen/settings.json` | Overrides system defaults |
| Project | `.qwen/settings.json` | Overrides user |
| System override | `/etc/qwen-code/settings.json`, etc. | Highest file precedence |
| Environment / CLI | env vars and flags | Overrides files |

Project-scoped MCP servers are gated by the optional folder-trust feature: if `security.folderTrust.enabled` is true, an untrusted folder ignores `.qwen/settings.json`, `.env` files, extensions, and auto-acceptance, which disables project MCP servers.

## Server Definition Shape

A minimal stdio server:

```json
{
  "mcpServers": {
    "pythonTools": {
      "command": "python",
      "args": ["-m", "my_mcp_server", "--port", "8080"],
      "cwd": "./mcp-servers/python",
      "env": {
        "DATABASE_URL": "$DB_CONNECTION_STRING",
        "API_KEY": "${EXTERNAL_API_KEY}"
      },
      "timeout": 15000,
      "trust": false
    }
  }
}
```

A minimal HTTP server:

```json
{
  "mcpServers": {
    "httpServer": {
      "httpUrl": "http://localhost:3000/mcp",
      "headers": {
        "Authorization": "Bearer your-api-token"
      },
      "timeout": 5000
    }
  }
}
```

Per-server `includeTools`/`excludeTools` filter the tool surface; `excludeTools` takes precedence.

## Tools, Resources, and Prompts

- **Tools** are fully exposed to the model. Tool schemas are sanitized for API compatibility and name conflicts are resolved by prefixing with the server alias (`serverName__toolName`).
- **Resources** are discovered per server and shown in the `/mcp` dialog. Users inject them with `@server:uri`; resource reads are disabled in untrusted folders.
- **Prompts** are exposed as slash commands (`/promptName`) and are user-invoked only.

## Roots, Sampling, and Elicitation

The public documentation does not describe MCP `roots/list`, `sampling/createMessage`, or elicitation/completion support. Claudine should treat these as unknown.

## Import, Export, and Sync

Claudine can treat Qwen CLI as an `import_sync` provider:

- **Import**: read `~/.qwen/settings.json` and `.qwen/settings.json` and normalize `mcpServers` into the MCP catalog.
- **Export**: write provider-shaped JSON back, preserving unrelated top-level settings.
- **Apply**: use `qwen mcp add` and `qwen mcp remove` to mutate configuration through the supported CLI.

Because MCP servers live inside the general settings file, Claudine must rewrite only the `mcpServers` object and preserve other settings.

## Runtime Injection

Qwen CLI does not support one-run MCP injection. There is no `--mcp-config` or inline config environment variable. The closest alternatives require mutating persistent files or pointing `QWEN_CODE_SYSTEM_DEFAULTS_PATH`/`QWEN_CODE_SYSTEM_SETTINGS_PATH` at a temporary file, neither of which is safe for wrapper-style one-run use.

## Authorization and Credentials

- **OAuth 2.0**: supported for HTTP/SSE servers with dynamic discovery, Google credentials, or service-account impersonation. The default redirect URI is `http://localhost:7777/oauth/callback`; remote deployments must configure a public `redirectUri`.
- **Token storage**: default plaintext `~/.qwen/mcp-oauth-tokens.json` (mode 0600); enable encrypted storage with `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true`.
- **Static headers**: supported via `headers` but discouraged for shared repo config.
- **stdio secrets**: delivered through the per-server `env` map or `$VAR` expansion.

OAuth requires a browser and will not work in headless mode without pre-authenticated tokens or static headers.

## Security Model

- **Trust**: per-server `trust: true` bypasses all confirmation prompts. Folder trust, when enabled, gates project-level MCP servers.
- **Filtering**: `includeTools`/`excludeTools`, global `mcp.allowed`/`mcp.excluded`, `--allowed-mcp-server-names`, and `permissions.deny` rules.
- **Sandboxing**: applies to built-in tools; MCP server processes may need to be available inside the chosen sandbox environment.
- **Response handling**: no native MCP response sanitization is documented; rich content is passed to the model.

## Mode-Specific Behavior

- **Interactive mode**: `/mcp` opens a management dialog; progressive discovery lets the UI appear immediately.
- **Headless mode** (`qwen -p ...`): waits for MCP discovery to settle before the first prompt. OAuth flows cannot complete without a browser.
- **Safe mode** (`--safe-mode` or `QWEN_CODE_SAFE_MODE=true`): disables MCP servers entirely.
- **ACP / daemon mode** (`qwen serve`): MCP discovery is also used; the shared MCP transport pool and budget guardrails are part of the daemon architecture.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Server fails to start | Status becomes `DISCONNECTED`; shown in `/mcp` and logs |
| Slow stdio server | Default 30 s discovery timeout; override with `discoveryTimeoutMs` |
| Slow/flaky remote server | Default 5 s discovery timeout; override with `discoveryTimeoutMs` |
| Tool call timeout | Per-server `timeout` (default 10 min) |
| Untrusted project | Project MCP servers ignored; resource reads disabled |
| OAuth in headless | Fails unless pre-authenticated tokens or static headers are used |

## Gaps

- Explicit MCP protocol version is not stated.
- No documented runtime injection path.
- No documented `roots`, `sampling`, or `elicitation` support.
- Default OAuth token storage is plaintext.
- Sandbox interaction with MCP servers is only partially documented.

## Claudine Integration Notes

- Treat Qwen CLI as `support: import_sync`.
- Read and write `~/.qwen/settings.json` and `.qwen/settings.json`, preserving all non-MCP settings.
- Use `qwen mcp add`/`remove` for apply operations.
- Do not attempt runtime wrapper injection; direct users to export/sync persistent config instead.
- Honor folder trust: do not assume `.qwen/settings.json` MCP servers are active until the workspace is trusted.
- Defensively scan MCP tool results in the `protect` layer.

## Sources

- [Connect Qwen Code to tools via MCP](https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp/)
- [MCP servers with Qwen Code (developer guide)](https://qwenlm.github.io/qwen-code-docs/en/developers/tools/mcp-server/)
- [Qwen Code Configuration](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Trusted Folders](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/trusted-folders/)
- [Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Sandboxing](https://qwenlm.github.io/qwen-code-docs/en/users/features/sandbox/)
