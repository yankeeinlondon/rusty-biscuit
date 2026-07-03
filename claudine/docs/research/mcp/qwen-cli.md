---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp/
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http, sse]
  lifecycle: |
    Qwen Code discovers MCP servers progressively. In interactive mode the UI
    appears immediately and an MCP status pill shows `N/M MCP servers ready`;
    tools become available to the model within ~16 ms of each server
    completing its discovery handshake. In non-interactive mode (`--prompt`,
    stream-json, ACP) the CLI waits for MCP discovery to settle before
    sending the first prompt, so scripted invocations see the same complete
    tool set the legacy synchronous behavior produced. The default
    discovery-only timeout is 30 s for stdio servers and 5 s for remote
    HTTP/SSE servers; override per server with `discoveryTimeoutMs`. Set
    `QWEN_CODE_LEGACY_MCP_BLOCKING=1` to restore the old blocking discovery
    behavior. As of v0.19.5 the capability discovery requests (tools/list,
    prompts/list, resources/list) retry transient network errors with
    backoff (PR #6158). HTTP and SSE servers connect at startup; stdio
    servers are spawned as local subprocesses at session start. The MCP
    runtime hot-reload design (Issue #3696 sub-task 3) adds settings-driven
    incremental reconnect, and the daemon mode `qwen serve` runtime adds a
    shared MCP transport pool (F2 design) with workspace-scoped entries,
    per-session injection via `newSession({mcpServers})`, an idle-cap /
    drain-grace lifecycle, and a `POST /workspace/mcp/:server/restart`
    route.
  notes: |
    The documentation does not state an explicit MCP protocol version date
    (for example `2024-11-05`, `2025-06-18`, or `2025-11-25`). HTTP
    (`httpUrl`) is the recommended remote transport; SSE (`url`) is
    described as legacy/deprecated. Streamable HTTP and stdio are the two
    actively supported paths.
config_files:
  - os: macos
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: |
      Default user scope. MCP servers live in the top-level `mcpServers`
      object. The base directory defaults to `~/.qwen` and can be
      overridden with the `QWEN_HOME` environment variable. Negative probe
      on this host (2026-07-03, Qwen Code 0.15.6): no top-level
      `mcpServers` key is present.

  - os: linux
    scope: user
    path: "~/.qwen/settings.json"
    format: json
    notes: |
      Same as macOS user scope. `~/.qwen` resolves to the home directory
      on Linux too.

  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\settings.json"
    format: json
    notes: |
      Windows user scope. The home directory is read from
      `%USERPROFILE%`.

  - os: macos
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: |
      Project-scoped MCP servers. Loaded only when the workspace is
      trusted (`security.folderTrust.enabled` true and the folder is in
      `~/.qwen/trustedFolders.json` as `TRUST_FOLDER` or
      `TRUST_PARENT`). Project settings override user settings.

  - os: linux
    scope: repo
    path: ".qwen/settings.json"
    format: json
    notes: "Project-scoped MCP servers on Linux; same gate as macOS."

  - os: windows
    scope: repo
    path: ".qwen\\settings.json"
    format: json
    notes: "Project-scoped MCP servers on Windows; same gate as macOS."

  - os: macos
    scope: system
    path: "/Library/Application Support/QwenCode/system-defaults.json"
    format: json
    notes: |
      System-wide defaults with the lowest precedence. Path can be
      overridden with `QWEN_CODE_SYSTEM_DEFAULTS_PATH`.

  - os: linux
    scope: system
    path: "/etc/qwen-code/system-defaults.json"
    format: json
    notes: "Linux system-wide defaults path; override via `QWEN_CODE_SYSTEM_DEFAULTS_PATH`."

  - os: windows
    scope: system
    path: "C:\\ProgramData\\qwen-code\\system-defaults.json"
    format: json
    notes: "Windows system-wide defaults path; override via `QWEN_CODE_SYSTEM_DEFAULTS_PATH`."

  - os: macos
    scope: managed
    path: "/Library/Application Support/QwenCode/settings.json"
    format: json
    notes: |
      System-wide override settings with the highest file precedence. Path
      can be overridden with `QWEN_CODE_SYSTEM_SETTINGS_PATH`.

  - os: linux
    scope: managed
    path: "/etc/qwen-code/settings.json"
    format: json
    notes: "Linux system-wide override path; override via `QWEN_CODE_SYSTEM_SETTINGS_PATH`."

  - os: windows
    scope: managed
    path: "C:\\ProgramData\\qwen-code\\settings.json"
    format: json
    notes: "Windows system-wide override path; override via `QWEN_CODE_SYSTEM_SETTINGS_PATH`."

  - os: macos
    scope: user
    path: "~/.qwen/trustedFolders.json"
    format: json
    notes: |
      Stores folder trust decisions when `security.folderTrust.enabled` is
      true. Untrusted folders ignore `.qwen/settings.json` (which also
      disables project MCP servers), `.env` files, extensions, and
      auto-acceptance.

  - os: linux
    scope: user
    path: "~/.qwen/trustedFolders.json"
    format: json
    notes: "Linux path; same behavior as macOS."

  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\trustedFolders.json"
    format: json
    notes: "Windows path; same behavior as macOS/Linux."

  - os: macos
    scope: user
    path: "~/.qwen/mcp-oauth-tokens.json"
    format: json
    notes: |
      Plaintext OAuth token store (mode 0600) used by default. Set
      `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true` to switch to
      keychain-backed storage or to `~/.qwen/mcp-oauth-tokens-v2.json`
      with AES-256-GCM encryption.

  - os: linux
    scope: user
    path: "~/.qwen/mcp-oauth-tokens.json"
    format: json
    notes: "Linux plaintext OAuth token store; same default behavior as macOS."

  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\mcp-oauth-tokens.json"
    format: json
    notes: "Windows plaintext OAuth token store; same default behavior."

  - os: macos
    scope: user
    path: "~/.qwen/mcp-oauth-tokens-v2.json"
    format: json
    notes: |
      Encrypted OAuth token store (AES-256-GCM) used when
      `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true` and no OS keychain
      is available.

  - os: linux
    scope: user
    path: "~/.qwen/mcp-oauth-tokens-v2.json"
    format: json
    notes: "Linux encrypted OAuth token store; same behavior as macOS."

  - os: windows
    scope: user
    path: "%USERPROFILE%\\.qwen\\mcp-oauth-tokens-v2.json"
    format: json
    notes: "Windows encrypted OAuth token store; same behavior as macOS/Linux."
cli_params:
  - flag: "qwen mcp add [options] <name> <commandOrUrl> [args...]"
    description: |
      Adds a persistent MCP server to user or project settings. The
      `--oauth-*` flags are rejected when combined with `--transport stdio`
      and apply only to `--transport sse` and `--transport http`.
    example: "qwen mcp add --transport http my-server http://localhost:3000/mcp"

  - flag: "qwen mcp remove <name>"
    description: "Removes a configured MCP server from settings."
    example: "qwen mcp remove my-server"

  - flag: "qwen mcp list"
    description: |
      Lists all configured MCP servers. A diagnostic context: a
      `DISCONNECTED` server means the URL/command is wrong or the
      discovery timeout fired.
    example: "qwen mcp list"

  - flag: "qwen mcp reconnect [server-name]"
    description: |
      Reconnect one MCP server (or all of them if no name is given).
      Documented subcommand added alongside the daemon-mode MCP transport
      pool (F2) — not present in the public MCP docs page; verified
      locally on Qwen Code 0.15.6.
    example: "qwen mcp reconnect my-server"

  - flag: "-s, --scope user|project"
    description: "Target scope for `qwen mcp add` and `qwen mcp remove`. Default `user`."
    example: "qwen mcp add --scope project --transport sse my-server https://example.com/sse"

  - flag: "-t, --transport stdio|sse|http"
    description: "Selects the MCP transport when adding a server. Default `stdio`."
    example: "qwen mcp add --transport stdio fs -- npx -y @modelcontextprotocol/server-filesystem ."

  - flag: "-e, --env KEY=VALUE"
    description: "Sets an environment variable on a stdio server (repeatable)."
    example: "qwen mcp add my-server -e API_KEY=abc -- /path/to/server"

  - flag: "-H, --header 'Name: Value'"
    description: "Sets a static HTTP header on an SSE or HTTP server (repeatable)."
    example: "qwen mcp add --transport http secure https://api.example.com/mcp -H 'Authorization: Bearer token'"

  - flag: "--timeout <ms>"
    description: "Per-server request/connection timeout in milliseconds."
    example: "qwen mcp add --transport http slow https://example.com/mcp --timeout 10000"

  - flag: "--trust"
    description: "Bypasses all tool-call confirmation prompts for the server."
    example: "qwen mcp add --trust my-server -- /path/to/server"

  - flag: "--description 'text'"
    description: "Sets the server description as it appears in the `/mcp` dialog."
    example: "qwen mcp add --description 'Local tools' my-server -- /path/to/server"

  - flag: "--include-tools <names>"
    description: "Comma-separated allowlist of tool names from the server."
    example: "qwen mcp add --include-tools read,search my-server -- /path/to/server"

  - flag: "--exclude-tools <names>"
    description: "Comma-separated denylist of tool names from the server. Takes precedence over `--include-tools`."
    example: "qwen mcp add --exclude-tools dangerous my-server -- /path/to/server"

  - flag: "--oauth-client-id, --oauth-client-secret, --oauth-redirect-uri, --oauth-authorization-url, --oauth-token-url, --oauth-scopes"
    description: "OAuth options for SSE/HTTP servers (default redirect URI `http://localhost:7777/oauth/callback`)."
    example: "qwen mcp add --transport sse oauth https://api.example.com/sse --oauth-client-id id --oauth-redirect-uri https://example.com/callback"

  - flag: "--safe-mode"
    description: |
      Disable all customizations for troubleshooting: context files, hooks,
      extensions, skills, MCP servers, custom subagents, permission rules,
      settings-sourced approval mode overrides, memory features, and
      sandbox settings. `--yolo` and `--approval-mode` still take effect.
      Also settable via `QWEN_CODE_SAFE_MODE=true`. Added in v0.19.5 (PR
      #4943); NOT present in this host's installed `qwen 0.15.6` help.
    example: "qwen -p '...' --safe-mode"

  - flag: "--telemetry (and --telemetry-* family)"
    description: |
      Telemetry gate flag exposed on every `qwen mcp` subcommand.
      Deprecated in favor of `telemetry.enabled` in `settings.json`; will
      be removed in a future version. Observed locally on v0.15.6.
    example: "qwen mcp list --telemetry false"
env_vars:
  - name: QWEN_HOME
    effect: |
      Customizes the global configuration directory (default `~/.qwen`).
      Affects where `~/.qwen/settings.json` and OAuth token files live.

  - name: QWEN_CODE_SYSTEM_DEFAULTS_PATH
    effect: "Overrides the path to the system defaults settings file (lowest precedence)."

  - name: QWEN_CODE_SYSTEM_SETTINGS_PATH
    effect: "Overrides the path to the system override settings file (highest file precedence)."

  - name: QWEN_CODE_LEGACY_MCP_BLOCKING
    effect: |
      Set to `1` to make the CLI wait synchronously for every configured
      MCP server's discovery handshake before returning from `Config.initialize()`.

  - name: QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE
    effect: |
      When set to `true`, OAuth tokens are stored in the OS keychain where
      available, or in `~/.qwen/mcp-oauth-tokens-v2.json` with AES-256-GCM
      encryption. Default storage is plaintext at
      `~/.qwen/mcp-oauth-tokens.json` with mode 0600.

  - name: QWEN_CODE_SAFE_MODE
    effect: |
      Equivalent to `--safe-mode`: disables MCP servers and other
      customizations for the run. Added in v0.19.5; not present in
      v0.15.6.

  - name: QWEN_SANDBOX
    effect: |
      Enables sandboxing for the session. MCP server executables must be
      available inside the chosen sandbox environment; the sandbox does
      not isolate MCP subprocesses directly.
server_schema:
  transports: ["stdio", "streamable_http", "sse"]
  command_fields:
    [
      "command",
      "args",
      "env",
      "cwd",
      "timeout",
      "discoveryTimeoutMs",
      "trust",
      "includeTools",
      "excludeTools",
      "description",
    ]
  http_fields:
    [
      "httpUrl",
      "url",
      "headers",
      "timeout",
      "discoveryTimeoutMs",
      "trust",
      "includeTools",
      "excludeTools",
      "description",
      "oauth",
      "authProviderType",
      "targetAudience",
      "targetServiceAccount",
    ]
  env_shape: |
    `env` is an object mapping variable names to string values. Values
    support `$VAR_NAME` and `${VAR_NAME}` expansion from the process
    environment when settings are loaded.
  auth_shape: |
    HTTP/SSE servers support OAuth 2.0 via an `oauth` object
    (`enabled`, `clientId`, `clientSecret`, `authorizationUrl`,
    `tokenUrl`, `scopes`, `redirectUri`, `tokenParamName`, `audiences`)
    or via `authProviderType` (`dynamic_discovery`,
    `google_credentials`, `service_account_impersonation`). Static
    authentication can also be provided through `headers`. stdio servers
    receive credentials only through the per-server `env` object or
    environment expansion. Auto-discovery: when the server responds with
    401 Unauthorized, the CLI looks for OAuth endpoints from server
    metadata and performs dynamic client registration if supported.
  notes: |
    Server id is the map key under `mcpServers`. At least one of
    `command`, `url`, or `httpUrl` must be provided. When multiple are
    specified, the order of precedence is `httpUrl`, then `url`, then
    `command`. Tool name conflicts are resolved by prefixing the later
    server name (`serverName__toolName`). `timeout` is a tool-call
    timeout (default 600,000 ms = 10 minutes); `discoveryTimeoutMs` is
    a separate handshake timeout (30 s stdio, 5 s remote by default).
    `excludeTools` takes precedence over `includeTools`.
server_capabilities:
  tools: full
  resources: partial
  prompts: full
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: true
  prompt_list_changed: true
  notes: |
    Tools are fully exposed to the model under the namespaced callable
    `serverName__toolName`. Resources are user-selectable via the `/mcp`
    dialog or `@server:uri` references; the model cannot autonomously
    read arbitrary resources. Prompts are surfaced as slash commands
    (labelled `MCP: <server>`) and are user-invoked only. The docs are
    lenient about the declared capability: Qwen Code calls `prompts/list`
    and `resources/list` even when the server omits the capability from
    its `initialize` response, so lenient servers still surface those
    surfaces. `qwen serve` daemon mode advertises an updated capability
    tag set atomically as of v0.16 (per the F2 design doc); the F2 pool
    fans `tools/list_changed`, `resources/list_changed`, and
    `prompts/list_changed` out to attached sessions.
client_capabilities:
  roots: full
  sampling: unknown
  elicitation: unknown
  notes: |
    The Qwen Code MCP docs page does not surface `roots/list` or
    `sampling/createMessage`, but the F2 shared-transport-pool design
    defines a `WorkspaceContext` + `ListRoots` path with
    `roots/list_changed` fan-up. Treat this as advertised by the daemon
    runtime but undocumented in the user docs; standalone `qwen`
    invocations may differ. Sampling and elicitation are not described
    anywhere in the public documentation or the F2 design — record as
    `unknown`.
tool_surface:
  discovery: |
    `discoverMcpTools()` iterates configured servers, connects via the
    chosen transport, calls the MCP tool listing endpoint, sanitizes
    schemas for API compatibility (strips `$schema` and
    `additionalProperties`, removes `anyOf` defaults, truncates names to
    63 characters), and registers tools in the global registry with
    `serverName__toolName` conflict resolution. Discovery is
    progressive in interactive mode and blocking in non-interactive
    mode. As of v0.19.5 (PR #6158) the capability discovery requests
    retry transient errors with backoff.
  filtering: |
    Per-server `includeTools` / `excludeTools` restrict the exposed tool
    surface; `excludeTools` takes precedence. Global `mcp.allowed` /
    `mcp.excluded` (with glob support: `*` matches any sequence, `?`
    matches a single character) and `mcp.serverCommand` filter which
    servers are connected at all. Folder trust, when enabled, gates
    project-scoped `.qwen/settings.json` servers.
  approval: |
    MCP tools use the same confirmation model as built-in tools unless
    the server is marked `trust: true` or the user chooses "Always
    allow this tool/server" after a prompt. In v0.19.5 (PR #6177) MCP
    approval dialogs are skipped in YOLO mode.
  result_handling: |
    Text, image, audio, `resource`, and `resource_link` content blocks
    are processed and passed to the model. Tool errors surface through
    the normal function-response path. Rich content (text + image, etc.)
    is supported via the standard MCP content blocks.
  annotations_trusted: |
    Not documented. Tool annotations are not described as a trusted
    policy surface in the user or developer docs.
  notes: |
    Tool schemas are sanitized for API compatibility. Resource reads
    are disabled in untrusted folders. The `qwen serve` daemon mode
    adds budget guardrails and a transport pool lifecycle that surfaces
    server status via `mcp-client.ts:serverStatuses` and typed
    `DaemonMcpServerRestart*` events; the pool also handles
    `statusChangeListener` notifications and stale-handler guards via a
    generation counter.
resource_surface:
  supported: true
  uri_schemes: ["file://"]
  templates: false
  subscriptions: false
  exposure_model: |
    Resources are discovered per server and exposed in the `/mcp`
    management dialog (browse server resources view). Users can inject
    a resource by typing `@server:uri`; the content is read and
    appended to the message (text inline, binary blobs as
    attachments). The `server` prefix must match a configured MCP
    server — otherwise the token is treated as a normal file path.
    Resource reads are disabled in untrusted folders.
  notes: |
    The documentation does not describe URI templates, subscription
    support (`resources/subscribe`), or which URI schemes are accepted
    beyond the `file://` example shown in the UI. The
    `resources/list_changed` notification is honored at the daemon
    transport pool layer (F2 design), but `resources/subscribe` push
    semantics are not advertised by the client.
prompt_surface:
  supported: true
  invocation: |
    MCP prompts appear as slash commands prefixed with `MCP: <server>`.
    They are invoked as `/promptName --arg=value` or with positional
    arguments; `/promptName help` shows the prompt's declared arguments.
  arguments: |
    Arguments are passed on the command line; `/promptName help` shows
    the declared arguments.
  exposure_model: |
    User-initiated only. The model does not invoke prompts
    autonomously. The slash command form may include the server alias
    for disambiguation when the same prompt name appears under multiple
    servers.
  notes: |
    Qwen Code attempts `prompts/list` even when the server does not
    declare the `prompts` capability, so lenient servers still surface
    prompts. The `prompts/list_changed` notification is honored at the
    daemon transport pool layer.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: deep
  notes: |
    Claudine can read `~/.qwen/settings.json` and `.qwen/settings.json`
    and normalize the `mcpServers` object into its catalog. Export
    must rewrite the whole settings file preserving unrelated top-level
    settings (model, security, env, modelProviders, etc.). Apply is
    possible through `qwen mcp add` / `qwen mcp remove`. Settings merge
    across scopes (system defaults → user → project → system override
    → env → CLI args) with higher precedence sources overriding lower;
    per-server entries from a higher scope replace matching entries
    from lower scopes. Global `mcp.allowed` / `mcp.excluded` apply
    after the per-server set is loaded.
runtime_injection:
  supported: false
  mechanism: |
    No documented one-run CLI flag for MCP injection. The legacy
    `qwen 0.15.6` install does not expose `--mcp-config`, `--bare`, or
    `--allowed-mcp-server-names`. The daemon mode (`qwen serve`) does
    add a per-session injection path via the SDK
    `newSession({mcpServers})` (F2 design), and a mid-session
    `/mcp disable <server>` command, but these are daemon-only.
  limitations: |
    There is no `--mcp-config` or inline-config equivalent for the
    standalone CLI. The closest alternatives are to write a temporary
    settings file and point `QWEN_CODE_SYSTEM_DEFAULTS_PATH` /
    `QWEN_CODE_SYSTEM_SETTINGS_PATH` at it, to mutate user/project
    settings before launch, or to drive the daemon mode SDK. None of
    these is safe for wrapper-style one-run use without persistent
    side effects.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens are stored in `~/.qwen/mcp-oauth-tokens.json` (plaintext,
    mode 0600) by default. If `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true`,
    Qwen Code uses keychain-backed storage where available, or
    `~/.qwen/mcp-oauth-tokens-v2.json` with AES-256-GCM encryption.
  token_scope: |
    Per configured remote server. Tokens are refreshed automatically
    when a refresh token is available and validated before each
    connection attempt. Auto-discovery is triggered on a 401 response:
    the CLI finds OAuth endpoints from server metadata and performs
    dynamic client registration if supported.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` object or via
    `$VAR` / `${VAR}` expansion sourced from the process environment.
    The process environment is otherwise inherited.
  notes: |
    Static `headers.Authorization` is supported but credentials end up
    in config files. OAuth requires a browser for the default
    `http://localhost:7777/oauth/callback` redirect URI and does not
    work in headless or remote deployments unless a publicly
    accessible `redirectUri` is configured. Google credentials and
    service-account impersonation are also supported for IAP-protected
    services (`authProviderType: google_credentials` /
    `service_account_impersonation`).
security:
  tool_filtering: |
    Per-server `includeTools` / `excludeTools`; global `mcp.allowed` /
    `mcp.excluded` (with `*` and `?` glob support, `excluded` wins on
    conflict); folder trust, when enabled, gates project-scoped servers
    in `.qwen/settings.json`; `mcp.serverCommand` provides a global
    command override.
  server_trust: |
    Per-server `trust: true` bypasses all confirmation prompts for that
    server. Project `.qwen/settings.json` is ignored when folder trust
    is enabled and the folder is untrusted. `QWEN_CODE_SAFE_MODE` /
    `--safe-mode` disables MCP servers entirely along with hooks,
    extensions, skills, custom subagents, permission rules, settings-
    sourced approval-mode overrides, memory features, and sandbox
    settings.
  env_sanitization: |
    Each stdio server receives its explicit `env` map plus inherited
    process environment. Values support `$VAR` / `${VAR}` expansion. No
    documented credential-scrubbing pass is applied specifically to MCP
    subprocesses. The YOLO no-sandbox warning is printed by `qwen` at
    startup when `tools.approvalMode: yolo` is set with no sandbox,
    but it does not strip cloud credentials from MCP subprocesses.
  sandbox_interaction: |
    Sandboxing applies to built-in shell/write/edit tools. MCP server
    subprocesses run separately and may fail or need to be available
    inside the sandbox environment; the Docker/Podman sandbox mounts
    the workspace and `~/.qwen` into the container, while macOS
    Seatbelt may restrict server executable paths. There is no
    documented OS-level sandbox boundary that wraps stdio MCP servers.
  response_filtering: |
    No native MCP response sanitization is documented. Tool schemas are
    sanitized for API compatibility, but rich content returned by
    tools (text, image, audio, resource_link) is passed through to the
    model.
  notes: |
    OAuth tokens are plaintext by default; enable encrypted storage on
    shared machines with `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true`.
    Administrators can lock down the MCP surface with system-level
    settings, project `.qwen/settings.json` allowlists/denylists, and
    folder trust.
gaps:
  - |
    No explicit MCP protocol version date is documented (for example
    `2024-11-05`, `2025-06-18`, or `2025-11-25`).
  - |
    No one-run CLI injection path. `--mcp-config`, `--bare`, and
    `--allowed-mcp-server-names` are not present in `qwen 0.15.6 --help`
    (verified locally); the daemon mode SDK
    `newSession({mcpServers})` is the only documented per-session
    injection.
  - |
    `sampling/createMessage` is not described anywhere in the user docs,
    developer docs, or the F2 transport-pool design. Treat as `unknown`.
  - |
    Elicitation (form-mode or URL-mode) is not documented. Treat as
    `unknown`.
  - |
    `resources/subscribe` push semantics are not advertised by the
    client; subscriptions are not described in the docs.
  - |
    Default OAuth token storage is plaintext at
    `~/.qwen/mcp-oauth-tokens.json`; users must opt into encrypted
    storage via `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true`.
  - |
    Interaction between MCP servers and the sandbox is only partially
    documented — the docs note MCP subprocesses are not isolated by the
    sandbox but do not describe a recommended configuration.
  - |
    The `qwen mcp reconnect` subcommand is verified locally on v0.15.6
    but is not mentioned in the public user or developer MCP docs.
  - |
    The F2 design describes `roots/list_changed` fan-up and a
    `WorkspaceContext` `ListRoots` handler at the daemon transport
    pool, but standalone `qwen` (no `serve`) does not document
    `roots/list`. Behavior may differ between the standalone CLI and
    `qwen serve`.
  - |
    No first-class `allowedMcpServers` / `deniedMcpServers` admin
    policy in settings.json. Filtering is via `mcp.allowed` /
    `mcp.excluded` glob lists only.
changes:
  - "Added new `qwen mcp reconnect [server-name]` subcommand. Verified locally on v0.15.6 (not present in the prior research's CLI surface)."
  - "Added `--safe-mode` flag and `QWEN_CODE_SAFE_MODE=true` env var (v0.19.5, PR #4943). Disables MCP servers along with hooks, extensions, skills, custom subagents, permission rules, settings-sourced approval-mode overrides, memory features, and sandbox settings."
  - "Added per-server `discoveryTimeoutMs` field (separate from `timeout`). Defaults: 30 s for stdio, 5 s for remote HTTP/SSE. Documented in the user MCP page; prior research did not separate handshake from tool-call timeouts."
  - "Added `authProviderType` field with values `dynamic_discovery` (default), `google_credentials`, and `service_account_impersonation`. The auto-discovery path triggers on 401 responses and uses dynamic client registration when supported."
  - "Added `mcp.serverCommand` global setting (string): a global command to start an MCP server, alongside `mcp.allowed` and `mcp.excluded`."
  - "Added PR #6158 (v0.19.5): capability discovery requests (`tools/list`, `prompts/list`, `resources/list`) retry transient errors with backoff."
  - "Added PR #6177 (v0.19.5): MCP approval dialogs are skipped in YOLO mode."
  - "Added PR #5879 (v0.19.4 line): web-shell can browse MCP server resources in the `/mcp` dialog."
  - "Added F2 design: shared MCP transport pool for `qwen serve` daemon mode with workspace-scoped entries, per-session injection via `newSession({mcpServers})`, `WorkspaceContext` + `ListRoots` + `roots/list_changed` fan-up, `POST /workspace/mcp/:server/restart`, mid-session `/mcp disable <server>`, and `mcpPoolActive` capability tag (default-on since v0.16)."
  - "Documented `--telemetry` and `--telemetry-*` flags exposed on every `qwen mcp` subcommand (deprecated in favor of `settings.json` `telemetry.enabled`). Verified locally on v0.15.6."
  - "Documented encrypted OAuth token path `~/.qwen/mcp-oauth-tokens-v2.json` (AES-256-GCM) used when `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true` and no OS keychain is available."
  - "Verified locally on the host (2026-07-03, `qwen --version` = `0.15.6`): `~/.qwen/settings.json` contains no top-level `mcpServers` key, `~/.qwen/.mcp.json` does not exist, `.qwen/settings.json` does not exist in the repo, and `qwen mcp list` prints `No MCP servers configured.`"
  - "Verified locally: `--safe-mode`, `--mcp-config`, `--bare`, and `--allowed-mcp-server-names` are NOT present in `qwen --help` on v0.15.6 (these flags belong to v0.19.x). `--telemetry` is exposed on every `qwen mcp` subcommand."
  - "Elevated `client_capabilities.roots` from `unknown` to `full` based on the F2 design doc (`WorkspaceContext` + `ListRoots` + `roots/list_changed`); standalone `qwen` still has no documented `roots/list` surface, so record as advertised by daemon mode only."
requires_claudine_update: true
reason: |
  Qwen Code's MCP surface has expanded since the prior research. The `mcp`
  module catalog and sync layer should:

  1. Treat Qwen as `import_sync` against `~/.qwen/settings.json` and
     `.qwen/settings.json`, preserving non-MCP top-level keys when
     rewriting the file.
  2. Recognize the new `qwen mcp reconnect` subcommand for apply
     operations and the `qwen mcp list` JSON-style status output.
  3. Surface the new server-schema fields the prior research missed:
     `discoveryTimeoutMs`, `authProviderType`, and the OAuth auto-
     discovery path on 401. Without these, Claudine's normalized
     catalog will silently drop provider-specific config on export.
  4. Honor the `QWEN_CODE_SAFE_MODE` / `--safe-mode` switch as an MCP
     deny-all (matches the prior Claude Code research note for
     `--safe-mode`).
  5. Honor `QWEN_CODE_LEGACY_MCP_BLOCKING` as the toggle that flips
     between blocking and progressive discovery at session start.
  6. Treat the F2 daemon-mode transport pool as out of scope for the
     standalone wrapper path, but record `qwen serve` SDK injection via
     `newSession({mcpServers})` so Claudine's daemon adapters can use
     it for one-run injection on hosts that opt in.
  7. Recognize `mcp.allowed` / `mcp.excluded` glob filters and
     `mcp.serverCommand` global override so the catalog does not
     normalize them away.
  8. Defensively scan MCP tool results in the `protect` layer; Qwen
     does not provide native response sanitization.
---

# MCP Support in Qwen CLI

## Overview

Qwen CLI (`qwen`) supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/introduction) as a persistent, settings-driven integration. MCP servers are declared under the top-level `mcpServers` object in `settings.json` and may also be managed through `qwen mcp add` / `qwen mcp remove` / `qwen mcp list` / `qwen mcp reconnect`. Once discovered, MCP tools become available to the model as namespaced callables (`serverName__toolName`), prompts surface as slash commands labelled `MCP: <server>`, and resources can be injected via `@server:uri` references or browsed through the `/mcp` dialog.

The strongest integration path for Claudine is **`import_sync`** — the persistent `mcpServers` object can be read, normalized, and written back, with `qwen mcp` providing a supported apply path. The daemon mode (`qwen serve`) adds a parallel shared transport pool with per-session injection via `newSession({mcpServers})`, but that path is daemon-only and not addressable from the standalone CLI.

Surface inventory:

- **Tools** — exposed: full tool list after discovery, namespaced under `serverName__toolName`. Discovery retries transient errors with backoff (v0.19.5, PR #6158); tool-search and YOLO-mode approval skipping (v0.19.5, PR #6177).
- **Resources** — exposed partially: per-server discovery surfaced through the `/mcp` dialog and `@server:uri` autocomplete. No `resources/subscribe` push; no template UI.
- **Prompts** — exposed: slash commands labelled `MCP: <server>`. Discovery is lenient (Qwen Code calls `prompts/list` even when the server omits the capability from `initialize`).
- **Roots** — exposed: `WorkspaceContext` + `ListRoots` + `roots/list_changed` fan-up in the F2 shared transport pool (daemon mode). Standalone `qwen` does not document `roots/list`.
- **Sampling** — unknown: not described in any of the user docs, developer docs, or the F2 design.
- **Elicitation** — unknown: not described in any of the documentation.
- **Channels / vendor extensions** — none documented for the MCP surface.

## Protocol and Transports

Qwen CLI supports three MCP transports:

| Transport | JSON field | Status |
| :-------- | :--------- | :----- |
| stdio | `command` (+ `args`) | Local subprocess |
| Streamable HTTP | `httpUrl` | Recommended for remote |
| SSE | `url` | Legacy / deprecated |

The docs recommend HTTP for remote servers and note that SSE is legacy. No explicit MCP protocol version date is stated. The implementation observes the modern feature generation: capability discovery (`tools/list`, `prompts/list`, `resources/list`) with retry-with-backoff (PR #6158), `list_changed` fan-out, OAuth 2.0 dynamic discovery with dynamic client registration, and an `authProviderType` field for Google credentials and service-account impersonation.

Discovery is progressive in interactive mode and blocking in non-interactive mode. Per-server `discoveryTimeoutMs` is documented separately from the per-server `timeout` (which is the per-`tools/call` timeout, default 10 minutes). Default discovery timeouts: 30 s for stdio servers, 5 s for remote HTTP/SSE. Set `QWEN_CODE_LEGACY_MCP_BLOCKING=1` to restore the old synchronous behavior.

The `qwen serve` daemon mode (F2 design, v0.16+) wraps the per-session transport in a workspace-scoped shared pool with an idle cap, drain grace, hot-config reload, and a `POST /workspace/mcp/:server/restart` route.

## Configuration

MCP servers live inside the general `settings.json` hierarchy:

| Scope | File | Precedence |
| :---- | :--- | :--------- |
| System defaults | `/etc/qwen-code/system-defaults.json`, `C:\ProgramData\qwen-code\system-defaults.json`, `/Library/Application Support/QwenCode/system-defaults.json` | Lowest |
| User | `~/.qwen/settings.json` | Overrides system defaults |
| Project | `.qwen/settings.json` | Overrides user |
| System override | `/etc/qwen-code/settings.json`, etc. | Highest file precedence |
| Environment / CLI | env vars and flags | Overrides files |

Project-scoped MCP servers are gated by the optional folder-trust feature: with `security.folderTrust.enabled: true` and a folder marked as `Don't trust`, `.qwen/settings.json`, `.env` files, extensions, and tool auto-acceptance are all suppressed. Trust decisions live in `~/.qwen/trustedFolders.json`.

OAuth token storage is a separate user-scope file:

| File | Storage | Trigger |
| :--- | :------ | :------ |
| `~/.qwen/mcp-oauth-tokens.json` | Plaintext, mode 0600 | Default |
| OS keychain | Encrypted | `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true` (when available) |
| `~/.qwen/mcp-oauth-tokens-v2.json` | AES-256-GCM | `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true` (no keychain) |

Negative probe on this host (2026-07-03, `qwen --version` ⇒ `0.15.6`): `~/.qwen/settings.json` exists, has no top-level `mcpServers` key, and `qwen mcp list` prints `No MCP servers configured.`

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
      "discoveryTimeoutMs": 30000,
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
      "timeout": 5000,
      "discoveryTimeoutMs": 5000
    }
  }
}
```

An OAuth-secured SSE server with Google service-account impersonation:

```json
{
  "mcpServers": {
    "iapServer": {
      "url": "https://my-iap-service.run.app/sse",
      "authProviderType": "service_account_impersonation",
      "targetAudience": "1234567890.apps.googleusercontent.com",
      "targetServiceAccount": "mcp-runner@my-project.iam.gserviceaccount.com",
      "oauth": {
        "scopes": ["https://www.googleapis.com/auth/cloud-platform"]
      }
    }
  }
}
```

Per-server `includeTools` / `excludeTools` filter the tool surface; `excludeTools` takes precedence. The `mcp.serverCommand`, `mcp.allowed`, and `mcp.excluded` settings provide global filters above the per-server set.

## Tools, Resources, and Prompts

### Tools

Tools are fully exposed to the model. The discovery layer (`mcp-client.ts:discoverMcpTools`) iterates configured servers, connects via the chosen transport, calls `tools/list`, sanitizes schemas (`$schema` and `additionalProperties` removed, `anyOf` defaults stripped, names truncated to 63 characters), and registers tools in the global registry under the `serverName__toolName` form. As of v0.19.5 (PR #6158) the discovery requests retry transient errors with backoff.

Per-server `includeTools` and `excludeTools` restrict which tool callables are registered. MCP tools use the same confirmation model as built-in tools, unless the server is marked `trust: true` or the user chooses "Always allow this tool/server" after a prompt. In v0.19.5 (PR #6177) the dialog is skipped in YOLO mode.

Tool results (text, image, audio, `resource`, and `resource_link` blocks) are passed through to the model.

### Resources

Resources are discovered per server and exposed in the `/mcp` management dialog. Users attach them by typing `@server:uri`; the content is read and appended to the message (text inline, binary blobs as attachments). The `server` prefix must match a configured MCP server, otherwise the token is treated as a normal file path. Resource reads are disabled in untrusted folders.

The docs do not document URI templates, `resources/subscribe`, or the accepted URI schemes beyond the `file://` example in the UI. `resources/list_changed` is honored at the daemon transport-pool layer.

### Prompts

MCP prompts become slash commands prefixed with `MCP: <server>`. They are user-invoked only — the model does not autonomously invoke prompts. Arguments are passed on the command line; `/promptName help` shows the declared arguments. Discovery is lenient about the declared `prompts` capability: Qwen Code attempts `prompts/list` even when the server omits the capability from its `initialize` response, so lenient servers still surface prompts.

## Roots, Sampling, and Elicitation

### Roots

The F2 design doc defines a `WorkspaceContext` + `ListRoots` path in the daemon transport pool with `roots/list_changed` fan-up — so `roots/list` is implemented at the daemon runtime. The standalone CLI does not document `roots/list` in either the user or developer MCP docs; record the surface as `full` for `qwen serve` and `unknown` for the standalone CLI until the public docs surface the standalone behavior.

### Sampling

Not documented in any of the user docs, developer docs, or the F2 design. Treat as `unknown` — there is no evidence Qwen Code acts as a sampling client.

### Elicitation

Not documented in any of the available references. Treat as `unknown`.

## Import, Export, and Sync

Claudine can treat Qwen CLI as an `import_sync` provider:

- **Import** — read `~/.qwen/settings.json` and `.qwen/settings.json`, normalize the `mcpServers` object into the MCP catalog. Surface the new server-schema fields (`discoveryTimeoutMs`, `authProviderType`, `oauth` sub-object) without flattening.
- **Export** — write provider-shaped JSON back, preserving unrelated top-level settings (`model`, `security`, `env`, `modelProviders`, etc.). Settings file rewrites must respect the file scope (`user` vs `project`).
- **Apply** — use `qwen mcp add`, `qwen mcp remove`, and `qwen mcp reconnect` to mutate configuration through the supported CLI. `qwen mcp reconnect` lets callers force a re-handshake without restarting the session.

Settings merge across scopes (system defaults → user → project → system override → env → CLI args) with higher precedence sources overriding lower; per-server entries from a higher scope replace matching entries from lower scopes. Global `mcp.allowed` / `mcp.excluded` filters are applied after the per-server set is loaded.

## Runtime Injection

The standalone CLI does not support one-run MCP injection. There is no `--mcp-config` or inline-config equivalent. The legacy `qwen 0.15.6` install (verified locally) does not expose `--mcp-config`, `--bare`, or `--allowed-mcp-server-names`. The closest alternatives:

- write a temporary settings file and point `QWEN_CODE_SYSTEM_DEFAULTS_PATH` / `QWEN_CODE_SYSTEM_SETTINGS_PATH` at it
- mutate user/project settings before launch
- drive the daemon mode SDK via `newSession({mcpServers})` and `qwen serve`

None of these are safe for wrapper-style one-run injection without persistent side effects.

The daemon mode (`qwen serve`, F2 design) supports per-session injection via the SDK: `newSession({mcpServers})` accepts an inline server set for that session, and `POST /workspace/mcp/:server/restart` forces a single-server reconnect. A mid-session `/mcp disable <server>` command lets the user turn off a server without leaving the session.

## Authorization and Credentials

- **OAuth 2.0** — supported for HTTP/SSE servers with `dynamic_discovery` (default), `google_credentials`, or `service_account_impersonation`. The default redirect URI is `http://localhost:7777/oauth/callback`; remote deployments must configure a public `redirectUri` via `--oauth-redirect-uri` or the `oauth.redirectUri` setting. Discovery: a 401 response triggers OAuth endpoint discovery from server metadata, then dynamic client registration when supported.
- **Token storage** — default plaintext at `~/.qwen/mcp-oauth-tokens.json` (mode 0600). Set `QWEN_CODE_FORCE_ENCRYPTED_FILE_STORAGE=true` to switch to OS keychain-backed storage or `~/.qwen/mcp-oauth-tokens-v2.json` with AES-256-GCM encryption.
- **Static headers** — supported via `headers.Authorization` but discouraged for shared repo config.
- **stdio secrets** — delivered through the per-server `env` map or `$VAR` / `${VAR}` expansion.

OAuth requires a browser for the default localhost callback and will not work in headless mode without pre-authenticated tokens or static headers. The CLI validates tokens before each connection attempt and refreshes them when a refresh token is available.

## Security Model

- **Trust** — per-server `trust: true` bypasses all confirmation prompts for that server. Folder trust, when enabled, gates project-level MCP servers (project `.qwen/settings.json` is ignored in an untrusted folder).
- **Filtering** — `includeTools` / `excludeTools` per server; `mcp.allowed` / `mcp.excluded` glob filters (with `excluded` winning on conflict); `mcp.serverCommand` global override.
- **Sandboxing** — applies to built-in shell/write/edit tools. MCP server processes run separately and may fail or need to be available inside the chosen sandbox environment; the Docker/Podman sandbox mounts the workspace and `~/.qwen` into the container, while macOS Seatbelt may restrict server executable paths. There is no documented OS-level sandbox boundary that wraps stdio MCP servers.
- **Response handling** — no native MCP response sanitization is documented; rich content is passed to the model.
- **Safe mode** — `--safe-mode` / `QWEN_CODE_SAFE_MODE=true` (v0.19.5+) disables MCP servers entirely along with hooks, extensions, skills, custom subagents, permission rules, settings-sourced approval-mode overrides, memory features, and sandbox settings. The CLI flags `--yolo` and `--approval-mode` still take effect.
- **YOLO without sandbox** — Qwen Code prints a one-line warning to stderr at startup when `--yolo` (or `--approval-mode=yolo`) is set with no sandbox configured. Suppress with `QWEN_CODE_SUPPRESS_YOLO_WARNING=1`.

## Mode-Specific Behavior

- **Interactive mode** — `/mcp` opens a management dialog with server status, resource browsing, prompt listing, and OAuth re-authentication. Progressive discovery shows `N/M MCP servers ready` in the bottom-right status pill.
- **Non-interactive mode** (`qwen -p ...`) — waits for MCP discovery to settle before the first prompt. OAuth flows cannot complete without a browser. Capability discovery retries transient errors with backoff (PR #6158). YOLO mode skips MCP approval dialogs (PR #6177).
- **Safe mode** (`--safe-mode` / `QWEN_CODE_SAFE_MODE=true`) — disables MCP servers entirely along with the other customizations listed under Security Model.
- **ACP / daemon mode** (`qwen serve`) — daemon architecture shares the MCP transport pool across sessions with per-session `mcpServers` overrides, workspace-scoped roots, and typed restart/budget events. Channel plugins (Telegram, WeChat, DingTalk, Feishu, QQ Bot) are a separate feature track and do not interact with MCP directly.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Server fails to start | Marked `DISCONNECTED` in `qwen mcp list`; verify URL/command, increase `timeout` or `discoveryTimeoutMs`. |
| Stdio server discovery slow | Default 30 s discovery timeout; override per server with `discoveryTimeoutMs`. |
| Remote server discovery slow / flaky | Default 5 s discovery timeout; override per server with `discoveryTimeoutMs`. |
| Tool call timeout | Per-server `timeout` (default 600,000 ms = 10 minutes). |
| Capability discovery transient error | Retried with backoff (v0.19.5, PR #6158); auth/4xx/request-timeout errors are not retried. |
| Untrusted project | Project MCP servers ignored; resource reads disabled. |
| OAuth in headless | Fails unless pre-authenticated tokens or static headers are used. |
| MCP approval prompts in CI | Pass `--yolo` (v0.19.5+, PR #6177) to skip MCP approval dialogs. |
| Stdio subprocess in sandbox | May fail or need to be available inside the sandbox image; no documented isolation wrapper. |

## Gaps

- Explicit MCP protocol version is not stated (no `2024-11-05`, `2025-06-18`, or `2025-11-25` reference).
- No documented `--mcp-config` one-run injection on the standalone CLI. `--safe-mode`, `--mcp-config`, `--bare`, and `--allowed-mcp-server-names` are not present in `qwen 0.15.6 --help` (verified locally).
- `sampling/createMessage` is not described in any of the user docs, developer docs, or the F2 design.
- Elicitation (form-mode or URL-mode) is not documented.
- `resources/subscribe` push semantics are not advertised by the client.
- Default OAuth token storage is plaintext; users must opt into encrypted storage.
- Sandbox interaction with MCP servers is only partially documented.
- The `qwen mcp reconnect` subcommand is verified locally but not mentioned in the public MCP docs.
- `roots/list` is documented for the daemon transport pool (F2) but not for the standalone CLI.
- No first-class `allowedMcpServers` / `deniedMcpServers` admin policy; filtering is via `mcp.allowed` / `mcp.excluded` glob lists only.

## Claudine Integration Notes

- Treat Qwen CLI as `support: import_sync`.
- Read and write `~/.qwen/settings.json` and `.qwen/settings.json`, preserving all non-MCP top-level settings. The local Qwen 0.15.6 install stores its `model`, `modelProviders`, `env`, and `security` blocks at the top level — overwriting them is a Claudine bug.
- Use `qwen mcp add` / `remove` / `reconnect` for apply operations. Record `qwen mcp list` output for diagnostics.
- Do not attempt runtime wrapper injection via `--mcp-config` on the standalone CLI; it does not exist. For daemon-mode injection, use the SDK `newSession({mcpServers})` path documented in the F2 design.
- Honor folder trust: do not assume `.qwen/settings.json` MCP servers are active until the workspace is trusted.
- Honor `QWEN_CODE_SAFE_MODE` / `--safe-mode` as an MCP deny-all.
- Honor `QWEN_CODE_LEGACY_MCP_BLOCKING` as the toggle that flips between blocking and progressive discovery.
- Defensively scan MCP tool results in the `protect` layer; Qwen does not provide native response sanitization.
- Surface the new server-schema fields (`discoveryTimeoutMs`, `authProviderType`, the OAuth auto-discovery path) in the catalog. Flattening them on export will silently lose provider-specific config.
- Recognize `mcp.allowed` / `mcp.excluded` glob filters and `mcp.serverCommand` global override so the catalog does not normalize them away.

## Sources

- [Connect Qwen Code to tools via MCP](https://qwenlm.github.io/qwen-code-docs/en/users/features/mcp/) — primary user-facing MCP documentation. Last updated 2026-07-02.
- [MCP servers with Qwen Code (developer guide)](https://qwenlm.github.io/qwen-code-docs/en/developers/tools/mcp-server/) — `mcp-client.ts` and `mcp-tool.ts` architecture, OAuth configuration, schema sanitization, conflict resolution.
- [Qwen Code Configuration](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/) — settings file hierarchy, scopes, and `mcp` settings keys.
- [Trusted Folders](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/trusted-folders/) — `~/.qwen/trustedFolders.json`, folder trust gate for project settings, `security.folderTrust.enabled`.
- [Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/) — `--safe-mode`, `--max-wall-time`, `--yolo`, persistent retry, MCP discovery behavior in non-interactive mode.
- [Sandboxing](https://qwenlm.github.io/qwen-code-docs/en/users/features/sandbox/) — `--sandbox`, `QWEN_SANDBOX`, sandbox image mounting, MCP subprocess notes.
- [F2: Shared MCP Transport Pool — Design v2.2](https://qwenlm.github.io/qwen-code-docs/en/design/f2-mcp-transport-pool/) — daemon-mode transport pool, `WorkspaceContext` / `ListRoots` / `roots/list_changed`, per-session injection via `newSession({mcpServers})`, restart route.
- [MCP Runtime Hot-Reload Design](https://qwenlm.github.io/qwen-code-docs/en/design/hot-reload/mcp-runtime-reinitialization/) — settings-driven incremental reconnect (Issue #3696 sub-task 3).
- [Qwen Code Releases](https://github.com/QwenLM/qwen-code/releases) — PR #6158 (capability discovery retry, v0.19.5), PR #6177 (skip MCP approval in YOLO, v0.19.5), PR #5879 (MCP resource browser), PR #4943 (`--safe-mode` flag, v0.19.5).
- [Qwen Code GitHub repository](https://github.com/QwenLM/qwen-code) — source code, design docs, release notes.
- Local observation (2026-07-03): `qwen --version` ⇒ `0.15.6`; `qwen mcp` subcommands are `add`, `remove`, `list`, `reconnect`; `qwen mcp list` prints `No MCP servers configured.`; `~/.qwen/settings.json` contains no top-level `mcpServers` key; `--safe-mode`, `--mcp-config`, `--bare`, and `--allowed-mcp-server-names` are NOT present in `qwen --help`; the `--telemetry` flag is exposed on every `qwen mcp` subcommand (deprecated).