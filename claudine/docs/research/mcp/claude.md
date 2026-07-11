---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://code.claude.com/docs/en/mcp
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http, http_sse, sse, websocket, custom]
  lifecycle: |
    Stdio servers are spawned as local subprocesses at session start and are not
    reconnected automatically if they exit. HTTP, SSE, WebSocket, and claude.ai
    connector servers connect at startup and auto-reconnect with exponential
    backoff (up to five attempts for mid-session disconnects; up to three retries
    for transient initial-connection errors at startup). Capability discovery
    requests (tools/list, prompts/list, resources/list) retry transient errors
    up to three times. Dynamic capability updates are accepted via MCP
    list_changed notifications. As of v2.1.142, MCP startup is non-blocking by
    default; servers connect in the background unless `alwaysLoad: true` is set,
    in which case startup blocks until that server connects (capped at 5s).
    Worktrees started by v2.1.142 also skip the MCP wait entirely when
    `MCP_CONNECTION_NONBLOCKING=true` is set, with `--mcp-config` server
    connections bounded at 5s rather than blocking on the slowest server.
  notes: |
    Claude Code accepts `type: "streamable-http"` as an alias for `"http"`. SSE is
    documented as deprecated in favor of HTTP. WebSocket (`type: "ws"`) is supported
    for servers that push events, but cannot be added with `--transport ws`; it must
    be configured via JSON. The `claude mcp add --transport` flag explicitly does
    not accept `ws`. An in-process `type: "sdk"` exists for Agent SDK hosts and is
    the only form that survives `disableSideloadFlags` (managed-only) — managed
    deployments block `--mcp-config`, `--plugin-dir`, `--plugin-url`, and
    `--agents` from being passed, but in-process SDK servers still load. The docs
    do not state an explicit MCP protocol version date; observed feature
    generation includes elicitation (form + URL mode), channels as a vendor
    extension (`claude/channel` capability), and tool annotations
    (`anthropic/requiresUserInteraction`, `anthropic/alwaysLoad`,
    `anthropic/maxResultSizeChars`).
config_files:
  - os: macos
    scope: user
    path: "~/.claude.json"
    format: json
    notes: |
      User-scoped MCP servers live in the top-level `mcpServers` object. The same
      file also stores per-project local-scoped servers under
      `projects/<path>/mcpServers`. This is distinct from `~/.claude/settings.json`,
      which does NOT hold MCP servers.
  - os: linux
    scope: user
    path: "~/.claude.json"
    format: json
    notes: "User-scoped MCP servers — same `~/.claude.json` layout as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude.json"
    format: json
    notes: "User-scoped MCP servers — same `~/.claude.json` layout as macOS/Linux."
  - os: macos
    scope: repo
    path: ".mcp.json"
    format: json
    notes: |
      Project-scoped servers intended for version control. Servers from this file
      are pending until the workspace is trusted and the user approves them via
      `claude mcp list` / `claude mcp get` or the `/mcp` panel.
  - os: linux
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project-scoped servers — same `.mcp.json` layout as macOS."
  - os: windows
    scope: repo
    path: ".mcp.json"
    format: json
    notes: "Project-scoped servers — same `.mcp.json` layout as macOS/Linux."
  - os: macos
    scope: local
    path: "~/.claude.json"
    format: json
    notes: |
      Local-scoped servers are stored inside `~/.claude.json` under the current
      project's `projects/<path>/mcpServers` entry. This differs from general local
      settings in `.claude/settings.local.json`.
  - os: linux
    scope: local
    path: "~/.claude.json"
    format: json
    notes: "Local-scoped servers — same `~/.claude.json` layout as macOS."
  - os: windows
    scope: local
    path: "%USERPROFILE%\\.claude.json"
    format: json
    notes: "Local-scoped servers — same `~/.claude.json` layout as macOS/Linux."
  - os: macos
    scope: plugin
    path: "<plugin-root>/.mcp.json"
    format: json
    notes: |
      Plugins can bundle MCP servers in a `.mcp.json` file at the plugin root or
      inline under `mcpServers` in `plugin.json`. Plugin servers start
      automatically when the plugin is enabled. Server name `--channels
      plugin:<name>@<marketplace>` opts channel-capable plugins into push
      messaging.
  - os: linux
    scope: plugin
    path: "<plugin-root>/.mcp.json"
    format: json
    notes: "Plugins — same `<plugin-root>/.mcp.json` layout as macOS."
  - os: windows
    scope: plugin
    path: "<plugin-root>\\.mcp.json"
    format: json
    notes: "Plugins — same `<plugin-root>\\.mcp.json` layout as macOS/Linux."
  - os: macos
    scope: system
    path: "/Library/Application Support/ClaudeCode/managed-mcp.json"
    format: json
    notes: |
      Managed/enterprise fixed server set. If present, it takes exclusive control
      and blocks user-, project-, and plugin-added servers (and claude.ai
      connectors, unless `allowAllClaudeAiMcps` is set in managed settings).
  - os: linux
    scope: system
    path: "/etc/claude-code/managed-mcp.json"
    format: json
    notes: "Linux/WSL path for the managed `managed-mcp.json` file."
  - os: windows
    scope: system
    path: "C:\\Program Files\\ClaudeCode\\managed-mcp.json"
    format: json
    notes: "Windows path for the managed `managed-mcp.json` file."
  - os: macos
    scope: managed
    path: "~/.claude/settings.json / .claude/settings.json / managed-settings.json / MDM / registry"
    format: json
    notes: |
      Allowlists/denylists, `allowManagedMcpServersOnly`, `allowAllClaudeAiMcps`,
      `channelsEnabled`, `allowedChannelPlugins`, `disableClaudeAiConnectors`,
      `disabledMcpjsonServers`, `strictPluginOnlyCustomization`, and
      `disableSideloadFlags` (managed-only — rejects `--mcp-config` and
      related flags from CLI surfaces).
  - os: linux
    scope: managed
    path: "~/.claude/settings.json / .claude/settings.json / managed-settings.json / MDM / registry"
    format: json
    notes: "Managed policy files — same union of paths as macOS."
  - os: windows
    scope: managed
    path: "%USERPROFILE%\\.claude\\settings.json / .claude\\settings.json / HKLM registry / managed-settings.json"
    format: json
    notes: "Managed policy files — Windows registry + HKLM are the equivalent of macOS MDM."
cli_params:
  - flag: "claude mcp add <name> -- <command> [args...]"
    description: "Add a persistent stdio MCP server."
    example: "claude mcp add fs -- npx -y @modelcontextprotocol/server-filesystem ."
  - flag: "claude mcp add --transport http <name> <url>"
    description: "Add a remote HTTP MCP server (streamable-http).
      Not a constant time; the transport name is `http`.
      Type alias `streamable-http` is accepted in JSON config."
    example: "claude mcp add --transport http sentry https://mcp.sentry.dev/mcp"
  - flag: "claude mcp add --transport sse <name> <url>"
    description: "Add a remote SSE MCP server (deprecated)."
    example: "claude mcp add --transport sse asana https://mcp.asana.com/sse"
  - flag: "claude mcp add-json <name> '<json>'"
    description: "Add a server from raw JSON, useful for WebSocket, OAuth, and
      sdk in-process configs."
    example: "claude mcp add-json ws '{\"type\":\"ws\",\"url\":\"wss://...\"}'"
  - flag: "claude mcp add-from-claude-desktop"
    description: "Import servers from Claude Desktop (macOS and WSL only)."
    example: "claude mcp add-from-claude-desktop --scope user"
  - flag: "claude mcp list"
    description: "List configured servers; health-checks connected servers.
      Honors project-trust gate as of v2.1.196; `.mcp.json` servers in untrusted
      repos are shown as `⏸ Pending approval`."
  - flag: "claude mcp get <name>"
    description: "Show details for a specific server; pending and rejected
      statuses are surfaced distinctly."
  - flag: "claude mcp remove <name>"
    description: "Remove a configured server."
    example: "claude mcp remove sentry --scope user"
  - flag: "claude mcp login <name>"
    description: "Run the OAuth flow for a server from the shell (v2.1.186+).
      Auto-detects SSH/headless and prints the redirect URL when no local
      browser is available; `--no-browser` forces the URL prompt."
  - flag: "claude mcp logout <name>"
    description: "Clear stored OAuth credentials for a server."
  - flag: "claude mcp reset-project-choices"
    description: "Reset approval choices for project-scoped `.mcp.json` servers."
  - flag: "claude mcp serve"
    description: "Run Claude Code itself as a stdio MCP server so external
      clients can call its tools (Bash/Read/Edit/etc.)."
  - flag: "--scope user|project|local"
    description: "Target scope for `add`/`add-json`/`remove`/"
  - flag: "--env KEY=VALUE"
    description: "Set an environment variable on a stdio server."
  - flag: "--header 'Name: Value'"
    description: "Set a static header on an HTTP/SSE server."
  - flag: "--callback-port <port>"
    description: "Pin the OAuth callback port to match a pre-registered redirect URI."
  - flag: "--client-id <id> / --client-secret"
    description: "Use pre-configured OAuth credentials instead of Dynamic Client
      Registration."
  - flag: "--no-browser"
    description: "Force `claude mcp login` to print the authorization URL instead
      of opening a browser (headless or SSH)."
  - flag: "--mcp-config <file-or-json>"
    description: "Load MCP servers for a single run (one or more file paths or
      inline JSON strings, space-separated). Persistent config still loads
      unless paired with `--strict-mcp-config`."
    example: "claude --mcp-config ./mcp.json --bare -p 'prompt'"
  - flag: "--strict-mcp-config"
    description: "When set with `--mcp-config`, ignore every other MCP
      configuration source. Without `--mcp-config`, it acts as a deny-all for
      MCP servers."
  - flag: "--channels plugin:<name>@<marketplace>"
    description: "Opt specific channel-capable MCP plugins into push messaging
      (research preview; requires Anthropic-authenticated session, v2.1.80+).
      Passing a plugin not on the active allowlist starts Claude normally but
      registers no channel."
  - flag: "--dangerously-load-development-channels"
    description: "Enable channels not on the allowlist, for local development;
      prompts for confirmation."
  - flag: "--bare"
    description: "Skip auto-discovery of hooks, skills, plugins, MCP servers,
      and CLAUDE.md. Equivalent to setting `CLAUDE_CODE_SIMPLE=1`. Auto-discovery
      is still off but `--mcp-config` servers continue to load."
  - flag: "--safe-mode"
    description: "Disable every user customization (CLAUDE.md, skills, plugins,
      hooks, MCP servers, custom agents/themes). Sets `CLAUDE_CODE_SAFE_MODE=1`.
      Managed policy still applies."
env_vars:
  - name: MCP_TIMEOUT
    effect: |
      Startup timeout in milliseconds per MCP server (default 30000). Distinct
      from `MCP_CONNECT_TIMEOUT_MS`, which bounds the blocking-connection batch
      before the initial tool list snapshots.
  - name: MCP_TOOL_TIMEOUT
    effect: |
      Default per-tool wall-clock timeout in milliseconds (default `100000000`,
      about 28 hours). Overridden by a per-server `timeout` field. Values below
      1000 in the env var are floored to 1s; below 1000 in the per-server field
      are ignored.
  - name: MCP_CONNECTION_NONBLOCKING
    effect: |
      As of v2.1.142, MCP startup is non-blocking by default. Set to `0` to
      restore the blocking 5-second connection wait. Servers with `alwaysLoad:
      true` still block startup regardless so their tools are present at the
      first prompt.
  - name: MCP_CONNECT_TIMEOUT_MS
    effect: |
      How long blocking MCP startup waits (in ms) for the connection batch
      before snapshotting the tool list (default 5000). Applies when
      `MCP_CONNECTION_NONBLOCKING=0` or for `alwaysLoad` servers. Servers still
      pending at the deadline keep connecting in the background.
  - name: MAX_MCP_OUTPUT_TOKENS
    effect: |
      Default maximum MCP tool output tokens (default 25000). Claude Code warns
      at 10000 tokens unless the server declares
      `_meta["anthropic/maxResultSizeChars"]`, in which case text content uses
      that character limit (up to 500000); image content still honors
      `MAX_MCP_OUTPUT_TOKENS`.
  - name: CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT
    effect: |
      Idle timeout (ms) for remote HTTP/SSE/WebSocket/claude.ai connector tool
      calls that send no response or progress notification (default 300000, or
      5 minutes; v2.1.187+). Set to `0` to disable. Values below 1000 are
      raised to 1s; value is capped at the effective `MCP_TOOL_TIMEOUT`.
      Stdio and IDE servers are exempt.
  - name: CLAUDE_CODE_SUBPROCESS_ENV_SCRUB
    effect: |
      When set, strips Anthropic and cloud-provider credentials from every
      subprocess Claude Code spawns, including stdio MCP servers. On Linux it
      also isolates Bash subprocesses in a private PID namespace. `claude-code-action`
      sets this automatically when `allowed_non_write_users` is configured.
  - name: CLAUDE_CODE_SCRIPT_CAPS
    effect: |
      JSON object capping per-session invocation counts for scripts matched by
      substring. Applies only when `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` is set.
  - name: ENABLE_TOOL_SEARCH
    effect: |
      Tool search defers MCP tools until Claude needs them. Enabled by default;
      disabled on Vertex AI and when `ANTHROPIC_BASE_URL` points at a non-
      first-party host. Accepts `true`, `false`, `auto`, or `auto:N` (custom
      context-window percentage threshold, default `auto:10`).
  - name: ENABLE_CLAUDEAI_MCP_SERVERS
    effect: |
      Set to `false` to disable claude.ai connectors for the session. Equivalent
      to the `disableClaudeAiConnectors` setting; servers passed explicitly via
      `--mcp-config` are unaffected.
  - name: MCP_CLIENT_SECRET
    effect: |
      Provides a pre-configured OAuth client secret non-interactively when running
      `claude mcp add --client-secret`. Avoids the interactive prompt in CI.
  - name: CLAUDE_PROJECT_DIR
    effect: |
      Set by Claude Code in the environment of stdio MCP servers to the project
      root, so servers can resolve project-relative paths. Plugin-provided MCP
      configs substitute `${CLAUDE_PROJECT_DIR}` directly; project/user-scoped
      configs need `${CLAUDE_PROJECT_DIR:-.}` because this var is only set in the
      server process env, not Claude's own env.
  - name: CLAUDE_CODE_MCP_SERVER_NAME
    effect: |
      Set when running a `headersHelper` command; names the MCP server being
      connected. Lets one helper script serve multiple servers.
  - name: CLAUDE_CODE_MCP_SERVER_URL
    effect: |
      Set when running a `headersHelper` command; provides the server's URL.
  - name: CLAUDE_PLUGIN_ROOT
    effect: |
      Set when running a `headersHelper` for a plugin-provided server; points to
      the plugin root directory so relative `headersHelper` paths resolve inside
      the plugin (v2.1.195+).
  - name: CLAUDE_CODE_SAFE_MODE
    effect: |
      Read when set to `1`; behaves as `--safe-mode` was passed. Disables
      user-discovered customizations including MCP servers; managed policy still
      applies.
  - name: CLAUDE_CODE_SIMPLE
    effect: |
      Read when set to `1`; equivalent to `--bare`. MCP servers from
      `--mcp-config` still load; auto-discovered MCP servers do not.
  - name: DISABLE_MCP_FOR_PROJECT
    effect: "unknown — not documented; observed absence."
server_schema:
  transports: ["stdio", "http", "streamable-http", "sse", "ws", "sdk"]
  command_fields: ["type", "command", "args", "env", "cwd", "timeout", "alwaysLoad"]
  http_fields: ["type", "url", "headers", "headersHelper", "oauth", "timeout", "alwaysLoad"]
  env_shape: |
    `env` is an object mapping variable names to string values. Values in `env`,
    plus `command`, `args`, and `url`, support `${VAR}` and `${VAR:-default}`
    expansion sourced from the user's process environment. Plugin-provided
    configs also expand `${CLAUDE_PROJECT_DIR}` (no default required); user/
    project configs must use `${CLAUDE_PROJECT_DIR:-.}` because the variable is
    only set inside the server's environment.
  auth_shape: |
    HTTP/SSE servers support OAuth 2.0 (Dynamic Client Registration, Client ID
    Metadata Document discovery, or pre-configured `oauth.clientId` +
    `--client-secret`), static `headers`, or a `headersHelper` command that
    emits a JSON object of headers at connect time. WebSocket servers support
    header-only auth (`headers` or `headersHelper`); OAuth is not supported over
    WebSocket. `oauth.scopes` pins requested scopes (RFC 6749 §3.3 space-
    separated string); `authServerMetadataUrl` overrides the default RFC 9728 +
    RFC 8414 metadata discovery chain. OAuth tokens are stored in the macOS
    Keychain when available, or in a credentials file on Windows/Linux — never
    in config files. The in-process `type: "sdk"` carries no auth in JSON; the
    host SDK supplies credentials internally.
  notes: |
    Server id is the map key under `mcpServers`. The `type` field accepts
    `"stdio"`, `"http"` (alias `"streamable-http"`), `"sse"`, `"ws"`, or
    `"sdk"`. The reserved server name `workspace` is skipped at load time with
    a warning asking the user to rename it. Project-scoped servers from
    `.mcp.json` require user approval before they connect (v2.1.196 also moved
    that check into `claude mcp list` / `claude mcp get`).
server_capabilities:
  tools: full
  resources: partial
  prompts: full
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: true
  prompt_list_changed: true
  notes: |
    Tools are fully exposed to the model, refreshed on `list_changed`, and
    optionally exempted from tool-search deferral via the per-server
    `alwaysLoad: true` field or the per-tool `_meta["anthropic/alwaysLoad"]: true`
    annotation. Resources are surfaced via `@` autocomplete in the prompt as
    `server:protocol://resource/path` references and are auto-fetched and
    attached to the conversation when referenced; the protocol-level
    `resources/list` and `resources/list_changed` notifications are honored,
    but server-pushed resource updates are not subscribed to
    (`resources/subscribe` is not advertised by Claude Code). Prompts are
    surfaced as slash commands with the form `/mcp__<server>__<prompt>`, take
    positional arguments, and are dynamically discovered and refreshed on
    `list_changed`.
client_capabilities:
  roots: partial
  sampling: unknown
  elicitation: full
  notes: |
    Roots: every stdio MCP server sees `CLAUDE_PROJECT_DIR` set to the launch
    directory, and any MCP server may call `roots/list` to retrieve it; the same
    path is passed to hooks. The boundary is therefore exactly the project
    root — there is no concept of additional roots, no per-tool boundary, and no
    wildcard. Sampling: `sampling/createMessage` and `completion/complete` are
    not described in the documentation; Claude Code does not advertise itself
    as a sampling client. Elicitation: the docs describe both form-mode and
    URL-mode elicitation. Dialogs appear automatically when a server requests
    them; auto-response is available via the `Elicitation` hook, and the URL
    mode supports OAuth-style deep links for two-step auth flows.
tool_surface:
  discovery: |
    `tools/list` is called at server startup and refreshed on `list_changed`.
    With tool search enabled (default), only tool names and server instructions
    load upfront; full schemas are discovered on demand. Without tool search
    (Vertex AI, non-first-party `ANTHROPIC_BASE_URL`, or `ENABLE_TOOL_SEARCH=false`),
    Claude Code uses the `WaitForMcpServers` tool instead so the model waits
    for needed servers before continuing.
  filtering: |
    Per-server filtering is available via managed `allowedMcpServers` /
    `deniedMcpServers` (matching `serverUrl`, `serverCommand`, or `serverName`).
    Permission rules such as `mcp__<server>__<tool>` or `mcp__<server>__*` apply
    in `permissions.allow` / `deny`. CLI `--allowedTools` / `--disallowedTools`
    also apply; a scoped rule leaves the tool available and denies matching
    calls while a bare `mcp__*` removes every MCP tool from the model's context.
    The `disabledMcpjsonServers` setting can reject project servers by name;
    `disableSideloadFlags` (managed-only) rejects `--mcp-config` from the CLI.
  approval: |
    MCP tool calls use the same permission model as native tools. An
    `anthropic/requiresUserInteraction` annotation on a tool forces a prompt on
    every call, even in `bypassPermissions`/`auto`/`acceptEdits` modes, and
    `bypassPermissions`-equivalent `--permission-prompt-tool` approvers are
    converted to deny with `MCP tool requires user interaction; not supported
    via --permission-prompt-tool`. The Agent SDK's `canUseTool` callback can
    still approve these calls.
  result_handling: |
    Text, image, and resource_link results are passed to the model. Tool errors
    are surfaced with `isError`. Outputs above `MAX_MCP_OUTPUT_TOKENS` are
    persisted to disk and replaced with a file reference; servers can raise
    their own text limit via `_meta["anthropic/maxResultSizeChars"]` up to
    500,000 characters (image data still honors `MAX_MCP_OUTPUT_TOKENS`). Root-
    level `anyOf`/`oneOf`/`allOf` schema combinators are flattened before the
    API call in v2.1.195+ — earlier versions skipped the tool entirely.
  annotations_trusted: |
    Three `anthropic/*` annotations are honored: `requiresUserInteraction`
    forces per-call prompts (overrides allow rules), `alwaysLoad` opts a tool
    out of tool-search deferral, and `maxResultSizeChars` raises the per-server
    output threshold. Other tool annotations are treated as hints.
  notes: |
    Project-scoped servers from `.mcp.json` must be approved before their tools
    become available. There is no documented per-argument approval policy.
    Plugin-bundled tools use the form `mcp__plugin_<plugin-name>_<server-name>__<tool-name>`
    with non-`[A-Za-z0-9_-]` characters replaced by `_`.
resource_surface:
  supported: true
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    User-selected via `@` autocomplete; resources appear alongside files in the
    menu and are referenced as `@server:protocol://resource/path`. Once
    referenced, Claude Code auto-fetches and attaches them to the conversation
    as context — there is no model-discoverable `resources/list` tool exposed
    to the model, but Claude Code provides internal tools to list and read MCP
    resources when servers advertise them. Templates and subscriptions are not
    described; schemes are server-defined.
  notes: |
    `resources/list` and `resources/list_changed` are part of capability
    discovery. `resources/subscribe` is not advertised by the Claude Code
    client, so push-based resource updates from a server are not supported.
prompt_surface:
  supported: true
  invocation: |
    Slash commands of the form `/mcp__<server>__<prompt>` appear in the `/`
    autocomplete palette alongside built-in commands. Typing the slash command
    executes the prompt; arguments are passed space-separated and parsed by
    the prompt's declared `arguments` schema.
  arguments: |
    The prompt's declared arguments (per the `prompts/list` response) are
    parsed positionally from the trailing tokens of the slash command. Server
    and prompt names are normalized (spaces converted to underscores) when
    forming the slash-command name.
  exposure_model: |
    User-controlled via slash command palette. Prompts are not auto-invoked by
    the model — the user must type the slash command to inject the prompt's
    contents into the conversation. Dynamically discovered and refreshed via
    `prompts/list` + `prompts/list_changed`.
  notes: |
    Prompt results are injected directly into the conversation once the user
    submits the slash command. Plugin-bundled prompts follow the standard
    `/mcp__<server>__<prompt>` form, with the server key being the
    `<plugin-name>_<server-name>` pair.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: replace
  notes: |
    Claudine can read `~/.claude.json` and `.mcp.json` and normalize server
    definitions into the MCP catalog, then write them back. Server entries are
    replaced whole per scope (highest-precedence source wins the full entry);
    fields are not merged across scopes. Allow/deny policy arrays
    (`allowedMcpServers`, `deniedMcpServers`) merge across settings sources
    unless `allowManagedMcpServersOnly` is set, in which case only the managed
    allowlist applies. The `claude mcp add`, `claude mcp add-json`,
    `claude mcp remove`, and `claude mcp add-from-claude-desktop` commands
    provide a supported apply path. `claude mcp serve` itself is not a sync
    primitive — it runs Claude as a server.
runtime_injection:
  supported: true
  mechanism: |
    `--mcp-config <file-or-json>` accepts one or more file paths or inline
    JSON strings (space-separated) for a single invocation. Persistent
    configuration still loads unless `--strict-mcp-config` is also passed, in
    which case every other MCP source (user, project, plugin, claude.ai) is
    ignored. Combine with `--bare` for fast scripted calls; auto-discovery is
    off but `--mcp-config` servers continue to load. OAuth flows cannot
    complete in non-interactive `-p` mode; pre-authenticated servers or static
    headers are required, and the model is told when servers need sign-in.
  limitations: |
    `--mcp-config` is documented primarily for `--bare` and headless use. It
    does not preserve the user/project/local merge semantics — Claudine must
    build the desired effective config itself when wrapping. Managed
    `disableSideloadFlags` rejects `--mcp-config` from the CLI by design, but
    accepts a `--mcp-config` whose servers are all in-process `type: "sdk"`
    entries so the Agent SDK and VS Code extension keep working. `--channels`
    plugins pushed through `--mcp-config` are still subject to the active
    allowlist; out-of-list plugins do not register.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens are stored in the macOS Keychain when available, or in a
    credentials file on Windows/Linux. Client secrets passed via
    `--client-secret` (or pre-existing client secrets) are stored in the same
    credential store, never in config files. Static `headers.Authorization` is
    supported but discouraged for shared repo config.
  token_scope: |
    Per remote server URL (and per the `oauth.scopes` value when set).
    Refresh tokens are stored by Claude Code and refreshed automatically;
    `offline_access` is appended to pinned scopes when the server advertises
    it. Refresh failures trigger a startup notice pointing at `/mcp` (v2.1.195+)
    with a Re-authenticate menu entry.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` object or via `${VAR}`
    expansion sourced from the user's process environment. Plugin-provided
    configs substitute `${CLAUDE_PROJECT_DIR}` directly; user/project configs
    need `${CLAUDE_PROJECT_DIR:-.}` because the variable is only set inside the
    server's environment. The process environment is otherwise inherited, so
    `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` is the recommended mitigation for
    stranding secrets in plain-env stdio servers.
  notes: |
    OAuth discovery follows RFC 9728 first (`/.well-known/oauth-protected-
    resource`), then falls back to RFC 8414 (`/.well-known/oauth-authorization-
    server`). Set `oauth.authServerMetadataUrl` to override. Dynamic Client
    Registration is the default; Client ID Metadata Document (CIMD) URLs are
    also discovered automatically when present. When `oauth.scopes` is unset,
    the requested scope comes from the server's `WWW-Authenticate` header or
    its protected resource metadata; the full `scopes_supported` catalog is
    no longer requested from automatically discovered metadata in v2.1.196+,
    so admins should pin `oauth.scopes` for hostile IdPs.
security:
  tool_filtering: |
    Managed `allowedMcpServers` / `deniedMcpServers` filter by `serverUrl`
    (`*` wildcards supported anywhere, hostname case-insensitive, trailing FQDN
    dot ignored), `serverCommand` (exact match including arg order), or
    `serverName` (exact match; `serverName` is a label the user picks, so it is
    not a security control — prefer `serverUrl`/`serverCommand` for
    enforcement). Permission rules support `mcp__<server>__<tool>` and
    `mcp__<server>__*` patterns. `disabledMcpjsonServers` rejects project
    servers by name. `disableSideloadFlags` (managed-only) rejects
    `--plugin-dir`, `--plugin-url`, `--agents`, and `--mcp-config` from CLI
    surfaces but still accepts in-process `type: "sdk"` entries.
  server_trust: |
    Project-scoped `.mcp.json` servers are pending until the workspace is
    trusted and the user approves them via `/mcp` or `claude mcp list` /
    `claude mcp get` (v2.1.196+). Committing `enableAllProjectMcpServers` or
    `enabledMcpjsonServers` to `.claude/settings.json` does NOT bypass trust
    for a freshly cloned repo. Managed `managed-mcp.json` takes exclusive
    control and blocks other servers (and claude.ai connectors unless
    `allowAllClaudeAiMcps` is set in managed settings). Channel-capable MCP
    servers still connect and their tools work, but channel messages do not
    arrive until a user opts them in with `--channels` and the plugin is on
    the active allowlist (`channelsEnabled` / `allowedChannelPlugins`).
  env_sanitization: |
    Each stdio server receives only its explicit `env` map plus inherited
    process env, with `CLAUDE_PROJECT_DIR` added. `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1`
    strips Anthropic and cloud-provider credentials from all subprocesses
    Claude Code spawns, including stdio MCP servers. On Linux it also isolates
    Bash in a private PID namespace so the server cannot read the host
    `/proc`. The Bash sandbox's `sandbox.credentials` can deny or mask env vars,
    but only for sandboxed Bash commands, not MCP servers.
  sandbox_interaction: |
    MCP servers run as ordinary local processes and are NOT isolated by
    Claude Code's built-in Bash sandbox, which applies only to Bash tool
    subprocesses. There is no documented OS-level sandbox or container
    boundary around stdio MCP servers; container isolation must come from the
    surrounding shell environment.
  response_filtering: |
    No native MCP response sanitization is documented. Large tool outputs are
    persisted to disk and summarized by the file-reference replacement. The
    `protect` layer in Claudine should treat MCP tool results as untrusted
    and scan them for prompt-injection patterns. Plugin-bundled MCP servers
    receive no separate input/output filtering.
  notes: |
    OAuth tokens and client secrets live in the OS credential store
    (Keychain / credentials file). Administrators should deploy
    `managed-mcp.json`, `allowedMcpServers`/`deniedMcpServers`, and a managed
    settings tier to enforce an organization-wide server surface. Plugin
    marketplace restrictions (`strictKnownMarketplaces`,
    `strictPluginOnlyCustomization: ["mcp"]`) and `disableSideloadFlags` are
    the supported levers for limiting where MCP servers can come from.
gaps:
  - |
    The official docs do not state which MCP protocol version date Claude
    Code implements (e.g., 2024-11-05, 2025-06-18, or 2025-11-25).
  - |
    Sampling (`sampling/createMessage`) is not documented as supported by
    Claude Code as a client; the docs are silent on whether servers can request
    LLM completions through Claude Code.
  - |
    `resources/subscribe` is not advertised by the Claude Code client; there
    is no documented UI or mechanism for a server to push resource updates.
  - |
    No first-class sandbox/container boundary for stdio MCP servers is
    described; isolation must come from the host environment.
  - |
    Exact precedence and merge behavior of `--mcp-config` versus persistent
    config when `--strict-mcp-config` is NOT passed is not fully specified —
    docs explain the strict variant, but the additive behavior is described
    primarily by example rather than with an explicit ordering rule.
  - |
    The `DISABLE_MCP_FOR_PROJECT` environment variable is undocumented; only
    `CLAUDE_CODE_SAFE_MODE=1` / `--safe-mode` and the
    `disableClaudeAiConnectors` setting are documented as MCP-disabling
    switches, and the docs explicitly warn that managed deployments cannot
    be bypassed by a project-level `false`.
changes:
  - "Prompts: previously `unknown`; now `full` — surfaced as slash commands `/mcp__<server>__<prompt>` with positional argument parsing and `prompts/list_changed` refresh."
  - "Resources: previously `unknown`; now `partial` — surfaced via `@server:protocol://path` autocomplete references and auto-fetched as context attachments; no `resources/subscribe` push and no `resources/templates` documented."
  - "Elicitation: previously `unknown`; now `full` — both form-mode and URL-mode dialogs appear automatically; auto-response is available via the `Elicitation` hook."
  - "Added new CLI flag `--strict-mcp-config`, which pairs with `--mcp-config` to ignore every other MCP configuration source; without `--mcp-config` it acts as an MCP deny-all."
  - "Added new env var `MCP_CONNECTION_NONBLOCKING` (default non-blocking since v2.1.142; set to `0` to restore the blocking 5s wait)."
  - "Added new env var `MCP_CONNECT_TIMEOUT_MS` (default 5000; bounds the blocking-connection batch snapshot, separate from `MCP_TIMEOUT`)."
  - "Added new env var `CLAUDE_CODE_MCP_SERVER_NAME` / `CLAUDE_CODE_MCP_SERVER_URL` (set inside `headersHelper` commands so one helper can serve multiple servers)."
  - "Added new env var `CLAUDE_CODE_SAFE_MODE` / `--safe-mode` and `CLAUDE_CODE_SIMPLE` / `--bare` (MCP-aware: both disable auto-discovered MCP servers but allow `--mcp-config` servers through)."
  - "Added new managed setting `disableSideloadFlags` (v2.1.193+) which rejects `--plugin-dir`, `--plugin-url`, `--agents`, and `--mcp-config` from the CLI by default; only in-process `type: \"sdk\"` entries still load."
  - "Added new in-process `type: \"sdk\"` server kind used by the Agent SDK and VS Code extension; not addressable through the `claude mcp add --transport` flag (use `claude mcp add-json`)."
  - "Added OAuth Client ID Metadata Document (CIMD) discovery alongside Dynamic Client Registration; `oauth.authServerMetadataUrl` overrides the default RFC 9728 → RFC 8414 metadata chain (v2.1.64+)."
  - "Added `oauth.scopes` behavior: when unset, the scope comes from the server's `WWW-Authenticate` header or its protected resource metadata; v2.1.196+ no longer requests the full `scopes_supported` catalog from automatically discovered metadata."
  - "Added `--no-browser` flag on `claude mcp login` for SSH / headless flows (v2.1.186+, auto-detected when no local browser is available)."
  - "Added `--channels plugin:<name>@<marketplace>` and `--dangerously-load-development-channels` flags (research preview in v2.1.80+; channels are gated by `channelsEnabled` and `allowedChannelPlugins` in managed settings)."
  - "Added new CLI command `claude mcp login <name>` (v2.1.186+) which runs the OAuth flow from the shell; when a token refresh fails (v2.1.195+) the `/mcp` panel offers a Re-authenticate menu entry."
  - "Documented v2.1.196 enforcement of project trust in `claude mcp list` / `claude mcp get`: `.mcp.json` approvals from settings files that ARE checked into source control are ignored in an untrusted folder, and pending servers stay shown as `⏸ Pending approval` rather than being connected or health-checked."
  - "Documented v2.1.195 root-level `anyOf`/`oneOf`/`allOf` schema flattening for tool input schemas; earlier versions skip tools whose schema has a root-level combinator."
  - "Documented v2.1.191 capability discovery retries: `tools/list`, `prompts/list`, and `resources/list` retry transient network and server errors up to three times with short backoff; auth/4xx/request-timeout errors are not retried."
  - "Reconnect budget: clarified that the \"up to five attempts for mid-session disconnects\" is exponential backoff starting at 1s and doubling, and that startup retries transient errors (5xx, connection refused, timeout) up to three times (v2.1.121+). Auth and 4xx errors are not retried."
  - "Reliability state correction: idle timeout default is 300000 ms (5 min), not just \"5 min\" — explicit. Stdio and IDE servers exempt; values below 1000 raise to 1s and cap at `MCP_TOOL_TIMEOUT`."
  - "Added `anthropic/alwaysLoad` per-tool annotation: equivalent to per-server `alwaysLoad: true` for that one tool, opts it out of tool-search deferral."
  - "Added BLAKE3-style fact under `Server Definition Shape`: plugin-bundled MCP tool callable names normalize non-`[A-Za-z0-9_-]` characters to `_`."
  - "Verified Claude Code version locally: 2.1.200 (native install method); `claude mcp list` shows claude.ai connectors `claude.ai Gmail`, `Google Drive`, `Google Calendar`, and `Hugging Face` as the active set on this host."
  - "Verified locally: `~/.claude.json` does not currently contain a top-level `mcpServers` key (no user-scoped servers configured on this host); there is no `~/.claude/.mcp.json`; `~/.claude/settings.json` contains no MCP-related keys."
requires_claudine_update: true
reason: |
  Three Claudine behaviors are now provable rather than guessed: prompts and
  elicitation are `full` (not `unknown`), resources are `partial` (not
  `unknown`), and `--strict-mcp-config` is the documented way to run an
  exclusive MCP set without mutating user config. The `mcp` module catalog and
  sync must surface prompt and resource surfaces distinctly from tools, and
  the runtime injector should pair `--mcp-config` with `--strict-mcp-config`
  by default for one-run wrappers so injected servers do not silently merge
  with persisted user defaults. New env vars (`MCP_CONNECTION_NONBLOCKING`,
  `MCP_CONNECT_TIMEOUT_MS`) and managed-only knobs (`disableSideloadFlags`,
  `allowAllClaudeAiMcps`) should be reflected in provider metadata so the
  wrapper layer can warn or refuse when set. The `claude mcp list` honors
  project trust (v2.1.196+) — Claudine's trust detection should treat
  `⏸ Pending approval` as a known state, not an error.

---

# MCP Support in Claude Code

## Overview

Claude Code supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io) as a first-class integration path. MCP servers extend Claude Code with external tools, resources, prompts, and event channels. Servers can be local stdio processes, remote HTTP/SSE/WebSocket endpoints, in-process SDK handles, or bundled with plugins. Configuration is scope-based, persistent, and can be administered through managed policy files. For Claudine, Claude Code is a strong `import_sync` target: persistent config files (`~/.claude.json`, `.mcp.json`, plugin `.mcp.json`) can be read and normalized into Claudine's MCP catalog, written back through either file edits or `claude mcp add`/`add-json`/`remove`/`add-from-claude-desktop` calls, and `--mcp-config` provides a one-run injection path that pairs cleanly with `--strict-mcp-config` to keep the wrapper's injected set authoritative without mutating user config.

Surface inventory (one-line):

- **Tools** — exposed: `mcp__<server>__<tool>` callable names, with tool-search deferral by default and per-server `alwaysLoad` opt-outs.
- **Resources** — exposed partially: `@server:protocol://resource/path` autocomplete references; auto-fetched as context attachments when referenced; no `resources/subscribe` push and no template UI.
- **Prompts** — exposed: slash commands `/mcp__<server>__<prompt>` with positional argument parsing.
- **Roots** — exposed: `CLAUDE_PROJECT_DIR` is set in the env, and `roots/list` returns the launch directory.
- **Sampling** — unknown: docs do not describe `sampling/createMessage` server-to-client flows.
- **Elicitation** — exposed: form-mode and URL-mode dialogs appear automatically; auto-response via the `Elicitation` hook.
- **Channels (vendor extension)** — exposed: research-preview push messaging via `type: "ws"` MCP servers that declare the `claude/channel` capability, opt-in per session with `--channels`.

## Protocol and Transports

Claude Code accepts the modern MCP transport set plus an in-process SDK kind and a vendor WebSocket channel kind:

| Transport | Status | How it is added | Notes |
| :-------- | :----- | :-------------- | :---- |
| `stdio` | Primary | `claude mcp add <name> -- <command>` | Local subprocess; `CLAUDE_PROJECT_DIR` injected; no auto-reconnect on exit. |
| `http` (`streamable-http` alias) | Recommended for remote | `claude mcp add --transport http <name> <url>` | Also accepted as JSON `"type": "streamable-http"` or `"type": "http"`. |
| `sse` | Deprecated | `claude mcp add --transport sse <name> <url>` | Legacy only. |
| `ws` | Supported for push events | JSON config only (`claude mcp add-json`) | `claude mcp add --transport` does NOT accept `ws`. Used by channels. |
| `sdk` | In-process | JSON config (`claude mcp add-json ... type: "sdk"`) | Agent SDK and VS Code extension; the only form that survives `disableSideloadFlags`. |
| claude.ai connectors | Cloud | Admin-provisioned at claude.ai or by org admin | Listed by `/mcp` when authenticated via claude.ai; fetched only when no first-party host is set. |

Lifecycle behavior differs by transport:

- **stdio** servers are spawned as local child processes at session start. If they exit, they are **not** automatically reconnected.
- **HTTP/SSE/WebSocket/claude.ai-connector** servers connect at startup and reconnect automatically with exponential backoff — starting at 1 second and doubling, up to **five attempts** for mid-session disconnects. Initial startup retries transient errors (5xx, connection refused, timeout) up to **three times** since v2.1.121; auth and 4xx errors are not retried because they need a configuration change.
- **Capability discovery** — `tools/list`, `prompts/list`, and `resources/list` retry transient network and server errors up to three times with short backoff (v2.1.191+); auth errors, 4xx responses, and request timeouts are not retried.
- **List updates** — servers can send `list_changed` notifications for tools, prompts, and resources, and Claude Code refreshes the local capability set automatically without a session restart.
- **Startup blocking** — as of v2.1.142, MCP startup is non-blocking by default; servers connect in the background and their tools become available as they finish. Servers with `alwaysLoad: true` still block startup (capped at the 5-second `MCP_CONNECT_TIMEOUT_MS`); restore the old blocking behavior with `MCP_CONNECTION_NONBLOCKING=0`. The `--mcp-config` set in `-p` mode is bounded at 5 s since v2.1.142 and `MCP_CONNECTION_NONBLOCKING=true` skips the wait entirely.

The documentation does not name an explicit MCP protocol version date. Observed feature generation includes: elicitation (form + URL mode), tool-search (`tool_reference` blocks), channels (`claude/channel` capability), `anthropic/*` tool annotations (`requiresUserInteraction`, `alwaysLoad`, `maxResultSizeChars`), and `WWW-Authenticate`-driven OAuth discovery. Claudine should treat the implemented version as observed rather than pinned.

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

Duplicate detection matches by name for the three primary scopes and by endpoint (URL or command) for plugins and connectors. Within each scope, `claude mcp list` / `claude mcp get` honor the project-trust gate as of v2.1.196: a `.mcp.json` server is shown as `⏸ Pending approval` (and not connected or health-checked) until the user accepts the workspace trust dialog interactively.

### Project trust

Servers in `.mcp.json` are not loaded until the user runs `claude` interactively and accepts the workspace trust dialog. `claude mcp list` shows them as `⏸ Pending approval` until then. Approvals sources that **do** apply in an untrusted folder:

- your own user `~/.claude/settings.json`
- managed settings
- settings passed with `--settings`
- `.claude/settings.local.json`, as long as git does not track it

A `disabledMcpjsonServers` entry in any settings file still rejects the server regardless of trust state.

### Managed configuration

Administrators can deploy `managed-mcp.json` to system paths:

- macOS: `/Library/Application Support/ClaudeCode/managed-mcp.json`
- Linux/WSL: `/etc/claude-code/managed-mcp.json`
- Windows: `C:\Program Files\ClaudeCode\managed-mcp.json`

When present, `managed-mcp.json` takes **exclusive control**: users cannot add, modify, or run any other MCP servers, including plugin-provided servers and claude.ai connectors (unless `allowAllClaudeAiMcps: true` is set in a managed settings source, v2.1.149+). To load claude.ai connectors alongside `managed-mcp.json`, the `allowAllClaudeAiMcps` setting must live in admin-controlled policy tiers (server-managed settings, MDM-deployed plist / HKLM registry key, or system `managed-settings.json`); users cannot re-enable them.

Policy controls in settings files include:

- `allowedMcpServers` — allowlist by `serverUrl` (with `*` wildcards), `serverCommand` (exact match including arg order), or `serverName` (label, not a security control)
- `deniedMcpServers` — denylist (merges from all scopes, takes precedence over allowlist)
- `allowManagedMcpServersOnly` — locks the allowlist to managed settings
- `disableClaudeAiConnectors` — disables cloud connectors (`true` in any source wins)
- `disabledMcpjsonServers` — rejects project servers by name in any settings file
- `strictPluginOnlyCustomization: ["mcp"]` — allows MCP servers only from plugins or managed settings
- `channelsEnabled` (v2.1.80+) — master switch for `--channels` push messaging
- `allowedChannelPlugins` (v2.1.80+) — replaces the Anthropic-maintained plugin allowlist when set
- `disableSideloadFlags` (v2.1.193+, managed-only) — rejects `--plugin-dir`, `--plugin-url`, `--agents`, `--mcp-config` from CLI surfaces at startup; in-process `type: "sdk"` MCP entries are still accepted

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
      "timeout": 600000,
      "alwaysLoad": false
    },
    "github": {
      "type": "http",
      "url": "https://api.githubcopilot.com/mcp/",
      "headers": { "Authorization": "Bearer ${GITHUB_PAT}" },
      "oauth": {
        "clientId": "your-client-id",
        "scopes": "repo read:user",
        "authServerMetadataUrl": "https://auth.example.com/.well-known/openid-configuration"
      },
      "timeout": 600000,
      "alwaysLoad": false
    }
  }
}
```

### Field reference

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `type` | all | `stdio`, `http`, `streamable-http`, `sse`, `ws`, or `sdk` |
| `command` | stdio | Executable to spawn |
| `args` | stdio | Argument array |
| `env` | stdio | Map of environment variables (overlaid on inherited env + `CLAUDE_PROJECT_DIR`) |
| `cwd` | stdio | Working directory for the server process |
| `url` | http / sse / ws | Endpoint URL |
| `headers` | http / sse / ws | Static header map |
| `headersHelper` | http / sse / ws | Shell command that emits a JSON header map (10-second timeout) |
| `oauth` | http / sse | `clientId`, `scopes`, `callbackPort`, `authServerMetadataUrl` |
| `timeout` | all | Per-server wall-clock timeout (ms); per-server `timeout` < 1000 falls through to `MCP_TOOL_TIMEOUT` since v2.1.162 |
| `alwaysLoad` | all (v2.1.121+) | Skip tool-search deferral; blocks startup until that one server connects |

### Environment variable expansion

Values in `command`, `args`, `env`, `url`, and `headers` support:

- `${VAR}` — expand to the value of `VAR`
- `${VAR:-default}` — expand to `VAR` or `default` if unset

Plugin-provided configs substitute `${CLAUDE_PROJECT_DIR}` directly; user- and project-scoped configs need `${CLAUDE_PROJECT_DIR:-.}` because the variable is only set inside the server's environment, not Claude Code's own environment. If a required variable is unset and has no default, config parsing fails.

## Tools, Resources, and Prompts

### Tools

Claude Code exposes MCP tools to the model. Tool names appear to the model as `mcp__<server>__<tool>`; plugin-bundled tools use `mcp__plugin_<plugin-name>_<server-name>__<tool-name>`, with any character outside `[A-Za-z0-9_-]` replaced by `_`.

Tool discovery and loading:

- `tools/list` is called at server startup and refreshed on `list_changed`.
- **Tool search** (the default since v2.1.121) defers full tool schemas; only tool names and server instructions load upfront and Claude discovers tools through the `ToolSearch` call. Set `ENABLE_TOOL_SEARCH=auto` to load schemas upfront when they fit within 10% (or `auto:N` for a custom percentage) of the context window, `ENABLE_TOOL_SEARCH=false` to defer nothing, or `=true` to force deferred loading everywhere (including Vertex AI).
- Vertex AI, a non-first-party `ANTHROPIC_BASE_URL`, or `ENABLE_TOOL_SEARCH=false` falls back to the `WaitForMcpServers` tool that pauses the request until needed servers connect. Tool search also needs `tool_reference`-capable models — Haiku does not support it; on Vertex AI it is supported for Sonnet 4.5+ and Opus 4.5+.
- A server or tool can opt out of tool-search deferral with `alwaysLoad: true` or `_meta["anthropic/alwaysLoad"]: true` respectively; `alwaysLoad` also blocks startup until that server connects (capped at the 5-second `MCP_CONNECT_TIMEOUT_MS`).

Tool input schemas:

- Claude Code truncates each tool description and server-instructions block at 2 KB.
- Root-level `anyOf` / `oneOf` / `allOf` JSON Schema combinators are flattened before being sent to the API (v2.1.195+); earlier versions skip tools with root-level combinators. Combinators nested inside `properties` are sent unchanged.

Tool approval:

- MCP tools use the same permission model as native tools. Allow rules like `mcp__github__get_issue` or wildcards such as `mcp__github__*` apply normally.
- An `_meta["anthropic/requiresUserInteraction"]: true` annotation forces a prompt on every call, even in `bypassPermissions` / `auto` / `acceptEdits`, and bypass-equivalent `--permission-prompt-tool` approvals convert to deny with `MCP tool requires user interaction; not supported via --permission-prompt-tool`. The Agent SDK's `canUseTool` callback can still approve these. Requires Claude Code v2.1.199+; earlier versions ignore the annotation.
- In Remote Control or SDK-host sessions, Claude Code marks the request as requiring user interaction so the host shows the permission prompt rather than a one-tap approve.

Tool output:

- Warning at 10,000 tokens; default hard cap is 25,000 tokens (`MAX_MCP_OUTPUT_TOKENS`).
- Per-server override via `_meta["anthropic/maxResultSizeChars"]` up to 500,000 characters (text only — images still honor `MAX_MCP_OUTPUT_TOKENS`).
- Outputs above the cap are persisted to disk and replaced with a file reference.
- Remote HTTP / SSE / WebSocket / claude.ai-connector tool calls abort after `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT` (default 5 min, v2.1.187+) of no response and no progress notification. Stdio and IDE servers are exempt.

### Resources

MCP resources are exposed as application-controlled context that the user attaches by `@`-mentioning them. In the prompt, type `@` to see resources from all connected MCP servers; pick `@server:protocol://resource/path` to attach it. Claude Code auto-fetches the resource and inserts it as a context attachment, but does not advertise `resources/subscribe` to servers — push updates are not supported. Claude Code also provides internal tools to list and read MCP resources when servers advertise them (so models can navigate resources on demand even without an explicit `@` mention). Templates (`resources/templates/list`) and template expansion are not described in the docs.

### Prompts

MCP prompts are exposed as slash commands in the `/` autocomplete palette. The form is `/mcp__<server>__<prompt>` and arguments are passed space-separated after the slash command; the server parses them according to its own prompt argument schema. Server and prompt names are normalized (spaces → underscores). Results are injected directly into the conversation, not sent through the model as an autocomplete. Prompts are dynamically discovered via `prompts/list` and refreshed on `prompts/list_changed`.

## Roots, Sampling, and Elicitation

### Roots

Claude Code provides a single root boundary to every stdio MCP server:

- The environment variable `CLAUDE_PROJECT_DIR` is set to the project root inside the spawned server's environment.
- Servers can call `roots/list` and receive the directory Claude Code was launched from.
- This is the same directory hooks receive in `CLAUDE_PROJECT_DIR`. There is no documented mechanism for additional roots or per-server boundaries.

### Sampling

The documentation does not describe `sampling/createMessage` or `completion/complete`. Claudine should treat these as **unknown** for Claude Code until documentation proves otherwise — there is no evidence Claude Code acts as a sampling client.

### Elicitation

Claude Code supports MCP elicitation in both supported modes:

- **Form mode** — Claude Code shows a dialog with form fields defined by the server and passes the user's response back to the server.
- **URL mode** — Claude Code opens a browser URL for authentication or approval; the user completes the flow in the browser and confirms in the CLI.

Dialogs appear automatically when a server requests them. To auto-respond programmatically, attach a handler to the `Elicitation` hook. Server authors are referred to the [MCP elicitation specification](https://modelcontextprotocol.io/docs/learn/client-concepts#elicitation).

## Import, Export, and Sync

Claudine can treat Claude Code as an `import_sync` provider:

- **Import** — read `~/.claude.json` and `.mcp.json`, normalize server definitions and managed policy arrays into the catalog.
- **Export** — write provider-shaped JSON back to those files. Claudine must avoid overwriting non-MCP contents of `~/.claude.json`.
- **Apply** — use `claude mcp add`, `claude mcp add-json`, `claude mcp remove`, and `claude mcp add-from-claude-desktop` to mutate configuration through the supported CLI.

Merge semantics:

- Server entries are **replaced whole** per scope — fields are never merged across scopes.
- `allowedMcpServers` and `deniedMcpServers` arrays merge across settings sources.
- When `allowManagedMcpServersOnly: true`, only the managed allowlist applies; the denylist still merges from every source.
- `claude mcp list` / `claude mcp get` honor the project-trust gate (v2.1.196+); pending `.mcp.json` servers do not appear as connected or health-checked, only as `⏸ Pending approval`.

## Runtime Injection

For one-run injection without mutating persistent config, Claude Code offers:

- `--mcp-config <file-or-json>` — load MCP servers for the current invocation only.
- `--strict-mcp-config` — when added, ignore every other MCP configuration source (user, project, plugin, claude.ai). Without `--mcp-config` it acts as an MCP deny-all.
- `--bare` — skip auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. `--mcp-config` servers still load.

Typical headless usage:

```bash
claude --bare --strict-mcp-config -p "Summarize this project" \
  --mcp-config '{"mcpServers":{"fs":{"type":"stdio","command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","."]}}}'
```

Limitations:

- `--mcp-config` is documented primarily for `--bare` and headless mode.
- Without `--strict-mcp-config`, persistent user/project/local servers still load — Claudine must build the desired effective config itself.
- OAuth flows cannot complete in non-interactive `-p` mode; pre-authenticated servers or static headers are required, and Claude Code explicitly tells the model that the server's tools are unavailable until you sign in from an interactive session.
- Managed `disableSideloadFlags` rejects `--mcp-config` from the CLI by design — except for in-process `type: "sdk"` entries, so the Agent SDK and VS Code extension keep working.
- `--mcp-config` `-p` servers are bounded at 5 s (v2.1.142+) instead of blocking on the slowest server.

## Authorization and Credentials

Claude Code supports multiple auth patterns for remote servers:

| Pattern | Where configured | Credential storage |
| :------ | :--------------- | :----------------- |
| Static header | `headers.Authorization` | In config file (not recommended for shared repos) |
| Dynamic header | `headersHelper` command | Helper fetches at connect time; re-runs automatically on 401/403 (v2.1.193+) |
| OAuth 2.0 dynamic | `oauth` object | System keychain / credentials file |
| OAuth 2.0 pre-registered | `oauth.clientId` + `--client-secret` | System keychain / credentials file |
| OAuth 2.0 CIMD | auto-discovery of a `Client ID Metadata Document` URL | Same as DCR |
| claude.ai connector | claude.ai admin console / org provisioning | Anthropic-managed |

OAuth details:

- Dynamic Client Registration is attempted automatically; Client ID Metadata Document URLs (CIMD) are also discovered automatically when present.
- Default discovery chain is RFC 9728 Protected Resource Metadata first (`/.well-known/oauth-protected-resource`), then RFC 8414 Authorization Server Metadata (`/.well-known/oauth-authorization-server`); set `oauth.authServerMetadataUrl` to override.
- `claude mcp login <name>` runs the configured OAuth flow from the shell (v2.1.186+). Without a local browser, it prints the authorization URL automatically and lets you paste the redirect URL back; `--no-browser` forces this prompt.
- `claude mcp logout <name>` clears stored tokens.
- `--callback-port <port>` pins the redirect URI port (v2.1.64+ requires `https://` for `authServerMetadataUrl`).
- `oauth.scopes` pins requested scopes; unset scopes use the `WWW-Authenticate` header value or the protected resource metadata's scope, not the full `scopes_supported` catalog (v2.1.196+ change).
- `offline_access` is automatically appended to pinned scopes when the AS advertises it, so the access token can refresh without re-auth.
- If a configured `Authorization` header is rejected by the server, Claude Code reports the connection as failed instead of falling back to OAuth — remove the header to use the OAuth flow.
- `--dangerously-skip-permissions` / `bypassPermissions` prompts an OAuth re-auth notice (v2.1.195+) when the stored refresh token is rejected.

For stdio servers, secrets should be passed through the per-server `env` object or `${VAR}` expansion. Per-user managed deployment should never store API keys in `managed-mcp.json` — use `${VAR}` expansion or per-user headers.

## Security Model

### Trust and allowlisting

- Project `.mcp.json` servers require explicit user approval after workspace trust (`claude mcp list` honors the gate since v2.1.196).
- Managed `managed-mcp.json` provides exclusive, admin-controlled server sets.
- `allowedMcpServers` / `deniedMcpServers` filter by URL (`*` wildcards), exact command/args, or user-chosen label. `serverName` is a label and cannot act as a security control; prefer `serverUrl` / `serverCommand` entries.
- Permission rules support per-server and per-tool patterns (`mcp__<server>__<tool>`).
- `disableSideloadFlags` rejects the very CLI knobs that bypass managed `strictKnownMarketplaces` and friends.

### Environment and sandboxing

- Stdio servers inherit the user's process environment plus their explicit `env` map and `CLAUDE_PROJECT_DIR`.
- `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` strips Anthropic and cloud-provider credentials from all subprocesses Claude Code spawns, including stdio MCP servers; on Linux it also isolates Bash in a private PID namespace.
- The built-in Bash sandbox does **not** isolate MCP servers; it applies only to Bash tool subprocesses.
- There is no documented OS-level sandbox around stdio MCP servers; container isolation must come from the host environment.

### Response handling

- No native MCP result sanitization is documented.
- Large outputs are truncated or persisted to disk (the file-reference replacement).
- Channel messages are gated by sender allowlists maintained by the channel plugin; only listed senders can push into the session, and the same allowlist gates the channel's permission-relay capability.
- Claudine's `protect` layer should scan MCP tool results defensively for prompt-injection patterns; Claude Code does not provide native response sanitization.

## Mode-Specific Behavior

### Interactive mode

- `/mcp` opens a panel showing connected servers, tool counts, and connection status.
- OAuth flows complete through `/mcp`; the model can route to `/mcp` when it sees a 401/403 or a refresh failure (v2.1.195+).
- Project `.mcp.json` servers appear as pending until approved.
- Channels are opt-in per session via `--channels`.

### Non-interactive / headless mode (`-p`)

- OAuth flows cannot run; pre-authenticated servers or header-based auth are required, and the model is told when a server's tools are unavailable so it can name the server.
- `--bare` skips auto-discovery (recommended for CI), but `--mcp-config` servers continue to load.
- `--strict-mcp-config` pairs with `--mcp-config` for a clean exclusive set in `claude -p`.
- `--mcp-config` `-p` mode is bounded at 5 s (v2.1.142+) instead of blocking on the slowest server; `MCP_CONNECTION_NONBLOCKING=true` skips the wait entirely.

### Safe mode

`--safe-mode` (or `CLAUDE_CODE_SAFE_MODE=1`) disables every user customization — CLAUDE.md, skills, plugins, hooks, MCP servers, custom commands, agents, and themes — but managed policy still applies. Useful for troubleshooting broken configurations.

### Non-interactive channels

When running channels in non-interactive mode with `-p`, tools that need terminal input (multiple-choice questions, plan-mode approval) are disabled so the session cannot stall waiting on a human. The session can still be driven through channels.

### Claude Code as an MCP server

`claude mcp serve` runs Claude Code itself as a stdio MCP server so external clients can call its built-in tools. The client is responsible for its own approval UI.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Stdio server fails to start | Marked failed in `/mcp` and `claude mcp list`; not retried |
| HTTP/SSE transient startup error | Retried up to three times (v2.1.121+); exponential backoff |
| HTTP/SSE mid-session disconnect | Retried up to five times (1s start, doubling) |
| HTTP/SSE auth or 4xx error | Not retried; server marked as needing auth or failed |
| Tool discovery transient failure | Retried up to three times (v2.1.191+); auth / 4xx / request-timeout not retried |
| Stdio server exits | Not auto-reconnected; user restarts the session |
| Remote tool idle timeout | Aborted after `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT` (default 5 min) |
| Tool wall-clock timeout | Aborted after per-server `timeout` or `MCP_TOOL_TIMEOUT` |
| Output too large | Persisted to disk and replaced with a file reference |
| Project server untrusted | Remains `⏸ Pending approval` (v2.1.196+) until trusted |
| Managed `disableSideloadFlags` set and `--mcp-config` passed | CLI rejects the flag unless `--mcp-config` is in-process `type: "sdk"` only |
| Refresh-token rejected | Re-auth notice via `/mcp`; tool call fails until re-auth (v2.1.195+) |
| `managed-mcp.json` exclusive control active and `claude mcp add` used | `Cannot add MCP server: enterprise MCP configuration is active and has exclusive control over MCP servers` |
| `MAX_MCP_OUTPUT_TOKENS` exceeded and no `maxResultSizeChars` annotation | Persisted to disk and replaced with a file reference |

## Claudine Integration Notes

- Treat Claude Code as `support: import_sync`. Map the catalog to the Claude `mcpServers` shape (stdio / http / streamable-http / sse / ws / sdk) and merge policy arrays correctly across scopes.
- For one-run wrappers, prefer `--mcp-config` with `--strict-mcp-config` and `--bare`; build the effective server list in memory rather than mutating user config.
- Surface prompts (`full`) and resources (`partial`) as first-class surfaces in the catalog's `server_capabilities`. Treating both as `tools` will mislead Claudine's surface detection.
- Honor project trust: treat `⏸ Pending approval` (v2.1.196+) as a known state, not a startup error.
- Do not place MCP server definitions in `~/.claude/settings.json` or `.claude/settings.json`; those files hold policy and other settings, not MCP servers.
- Defensively scan MCP tool results in the `protect` layer; Claude Code does not provide native response sanitization.
- Channel plugins are gated by `--channels` plus `channelsEnabled` / `allowedChannelPlugins`; treat them as separate from generic MCP servers in the wrapper layer.
- Managed-only `disableSideloadFlags` should be respected — Claudine must warn and refuse in the wrapper if the host is managed and an injected `--mcp-config` contains anything other than `type: "sdk"` entries.

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
- [Channels reference](https://code.claude.com/docs/en/channels-reference)
- [Agent SDK permissions](https://code.claude.com/docs/en/agent-sdk/permissions)
- Local observation: `claude --version` ⇒ `2.1.200 (Claude Code)`; `claude mcp list` shows four `claude.ai` connectors; `claude mcp --help` shows the `add`/`add-json`/`add-from-claude-desktop`/`get`/`list`/`login`/`logout`/`remove`/`reset-project-choices`/`serve` subcommand surface.
