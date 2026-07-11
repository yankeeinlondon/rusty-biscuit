---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://opencode.ai/docs/mcp-servers/
support: runtime_injection
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http, sse]
  lifecycle: |
    Local servers are spawned as child processes at session start. Remote
    servers try Streamable HTTP first and fall back to legacy HTTP+SSE. As of
    v1.17.7, MCP servers can receive the current workspace as a client root;
    as of v1.17.12, OpenCode reconnects after OAuth even if the server was
    disabled, requests MCP refresh-token scope, and scopes auth status per
    server URL. The docs do not name a specific MCP protocol version date;
    `protocolVersion` is delegated to the underlying `@modelcontextprotocol/sdk`
    Client (verified in source: `packages/opencode/src/mcp/index.ts:2480`,
    `DEFAULT_TIMEOUT = 30_000` at line 2418 — the docs' "5000 ms" figure for
    `timeout` is not what the source uses as a default).
  notes: |
    Three transports are wired in source: `StdioClientTransport`,
    `StreamableHTTPClientTransport`, and `SSEClientTransport`. WebSocket and
    custom transports are not implemented. Capability discovery is delegated
    to the SDK; there is no OpenCode-side retry, backoff, or reconnect logic
    (verified by negative grep). Stderr from stdio servers is piped and
    surfaced in logs.
config_files:
  - os: macos
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    notes: |
      Primary user-level JSON config. MCP servers live under the top-level
      `mcp` object. The `.jsonc` variant is also supported. Observed in
      `opencode debug config` logs at v1.17.13.
  - os: linux
    scope: user
    path: "~/.config/opencode/opencode.json"
    format: json
    notes: |
      Linux user-level JSON config. Same precedence as macOS; `~/.config` is
      resolved per the XDG Base Directory Specification.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.json"
    format: json
    notes: |
      Windows user-level config path. OpenCode resolves its config dir
      home-relative (`~/.config/opencode`) on every OS — on Windows that is
      the literal `.config` directory under the user profile, NOT %APPDATA%
      (cross-validated against the agent-cli topic's host-evidence records).
  - os: macos
    scope: user
    path: "~/.config/opencode/opencode.jsonc"
    format: jsonc
    notes: |
      JSON-with-comments variant of the macOS user config. Both `.json` and
      `.jsonc` are loaded; later wins for conflicting keys.
  - os: linux
    scope: user
    path: "~/.config/opencode/opencode.jsonc"
    format: jsonc
    notes: |
      JSON-with-comments variant of the Linux user config. Both `.json` and
      `.jsonc` are loaded; later wins for conflicting keys.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\opencode.jsonc"
    format: jsonc
    notes: |
      JSON-with-comments variant of the Windows user config.
  - os: macos
    scope: user
    path: "~/.config/opencode/config.json"
    format: json
    notes: |
      Legacy macOS user-level config still loaded by v1.17.13. This is the
      file where the MCP `mcp` block lives on this host.
  - os: linux
    scope: user
    path: "~/.config/opencode/config.json"
    format: json
    notes: |
      Legacy Linux user-level config still loaded by v1.17.13.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\config.json"
    format: json
    notes: "Legacy Windows user-level config still loaded by v1.17.13."
  - os: macos
    scope: user
    path: "~/.config/opencode/tui.json"
    format: jsonc
    notes: |
      macOS TUI-only settings (theme, scroll, keybinds). MCP is not stored
      here — this is the file formerly targeted by deprecated top-level
      `theme`, `keybinds`, and `tui` keys in `opencode.json`.
  - os: linux
    scope: user
    path: "~/.config/opencode/tui.json"
    format: jsonc
    notes: "Linux TUI-only settings — same role as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.config\\opencode\\tui.json"
    format: jsonc
    notes: "Windows TUI-only settings — same role as macOS/Linux."
  - os: macos
    scope: user
    path: "~/.opencode/opencode.json"
    format: json
    notes: |
      Legacy single-dot config location on macOS. Still loaded by v1.17.13
      for backwards compatibility, after the `.opencode/` directories.
  - os: linux
    scope: user
    path: "~/.opencode/opencode.json"
    format: json
    notes: "Legacy single-dot config location on Linux — same as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.opencode\\opencode.json"
    format: json
    notes: "Legacy single-dot config location on Windows (home-relative, same layout as macOS/Linux)."
  - os: macos
    scope: user
    path: "~/.opencode/opencode.jsonc"
    format: jsonc
    notes: "JSONC variant of the macOS legacy single-dot location."
  - os: linux
    scope: user
    path: "~/.opencode/opencode.jsonc"
    format: jsonc
    notes: "JSONC variant of the Linux legacy single-dot location."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.opencode\\opencode.jsonc"
    format: jsonc
    notes: "JSONC variant of the Windows legacy single-dot location (home-relative)."
  - os: macos
    scope: repo
    path: "<project>/opencode.json"
    format: json
    notes: |
      Project-level JSON config. Per docs, "project config has the highest
      precedence among standard config files" and is safe to commit to Git.
      MCP entries placed here are loaded on every session.
  - os: linux
    scope: repo
    path: "<project>/opencode.json"
    format: json
    notes: "Project-level config — same behavior as macOS."
  - os: windows
    scope: repo
    path: "<project>\\opencode.json"
    format: json
    notes: "Project-level config — same behavior as macOS/Linux."
  - os: macos
    scope: repo
    path: "<project>/.opencode/opencode.json"
    format: json
    notes: |
      `.opencode/` directory holds agents, commands, modes, plugins, skills,
      tools, themes, and (alongside the top-level file) `opencode.json` /
      `opencode.jsonc`. Plural subdirectory names are preferred; singular
      names are kept for backwards compatibility.
  - os: linux
    scope: repo
    path: "<project>/.opencode/opencode.json"
    format: json
    notes: "Project-level `.opencode/` directory config — same as macOS."
  - os: windows
    scope: repo
    path: "<project>\\.opencode\\opencode.json"
    format: json
    notes: "Project-level `.opencode/` directory config — same as macOS/Linux."
  - os: macos
    scope: remote
    path: ".well-known/opencode (HTTPS)"
    format: json
    notes: |
      Organization-supplied remote config; fetched automatically when the
      authenticated provider supports it. Loaded first, before global config.
  - os: macos
    scope: other
    path: "OPENCODE_CONFIG"
    format: json
    notes: |
      Environment variable on macOS pointing to a custom config file. Loaded
      between global and project configs.
  - os: linux
    scope: other
    path: "OPENCODE_CONFIG"
    format: json
    notes: "Linux equivalent of `OPENCODE_CONFIG`."
  - os: windows
    scope: other
    path: "OPENCODE_CONFIG"
    format: json
    notes: "Windows equivalent of `OPENCODE_CONFIG`."
  - os: macos
    scope: other
    path: "OPENCODE_CONFIG_CONTENT"
    format: json
    notes: |
      macOS environment variable holding inline JSON config content. Loaded
      after `.opencode` directories and before managed config. This is the
      documented runtime-injection mechanism for Claudine's one-run wrappers.
  - os: linux
    scope: other
    path: "OPENCODE_CONFIG_CONTENT"
    format: json
    notes: "Linux equivalent of `OPENCODE_CONFIG_CONTENT`."
  - os: windows
    scope: other
    path: "OPENCODE_CONFIG_CONTENT"
    format: json
    notes: "Windows equivalent of `OPENCODE_CONFIG_CONTENT`."
  - os: macos
    scope: other
    path: "OPENCODE_CONFIG_DIR"
    format: other
    notes: |
      macOS environment variable pointing to a custom config directory that
      mirrors the structure of `.opencode`. Loaded after the global config
      and after `.opencode` directories, so it can override their settings.
  - os: linux
    scope: other
    path: "OPENCODE_CONFIG_DIR"
    format: other
    notes: "Linux equivalent of `OPENCODE_CONFIG_DIR`."
  - os: windows
    scope: other
    path: "OPENCODE_CONFIG_DIR"
    format: other
    notes: "Windows equivalent of `OPENCODE_CONFIG_DIR`."
  - os: macos
    scope: other
    path: "OPENCODE_TUI_CONFIG"
    format: jsonc
    notes: "macOS TUI-only config path. Does not hold MCP definitions."
  - os: linux
    scope: other
    path: "OPENCODE_TUI_CONFIG"
    format: jsonc
    notes: "Linux TUI-only config path. Does not hold MCP definitions."
  - os: windows
    scope: other
    path: "OPENCODE_TUI_CONFIG"
    format: jsonc
    notes: "Windows TUI-only config path. Does not hold MCP definitions."
  - os: macos
    scope: other
    path: "OPENCODE_PERMISSION"
    format: json
    notes: |
      macOS environment variable holding inline JSON permissions. MCP tool
      approval policy is governed by `permission`, so this can affect MCP
      allow/ask/deny decisions for the run.
  - os: linux
    scope: other
    path: "OPENCODE_PERMISSION"
    format: json
    notes: "Linux equivalent of `OPENCODE_PERMISSION`."
  - os: windows
    scope: other
    path: "OPENCODE_PERMISSION"
    format: json
    notes: "Windows equivalent of `OPENCODE_PERMISSION`."
  - os: macos
    scope: system
    path: "/Library/Application Support/opencode/opencode.json"
    format: json
    notes: |
      macOS managed (file-based) config. Requires admin/root to write.
      `opencode.json` or `opencode.jsonc` are both honored.
  - os: linux
    scope: system
    path: "/etc/opencode/opencode.json"
    format: json
    notes: |
      Linux managed (file-based) config directory. Requires root to write.
  - os: windows
    scope: system
    path: "%ProgramData%\\opencode\\opencode.json"
    format: json
    notes: |
      Windows managed (file-based) config directory. Requires Administrator
      to write.
  - os: macos
    scope: managed
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    format: other
    notes: |
      macOS MDM-deployed `ai.opencode.managed` preference domain (`.mobileconfig`
      PayloadType). Highest precedence overall; cannot be overridden by user
      or project config.
  - os: macos
    scope: managed
    path: "/Library/Managed Preferences/ai.opencode.managed.plist"
    format: other
    notes: |
      Alternate macOS managed preferences path. Same `ai.opencode.managed`
      domain. MDM metadata keys (`PayloadUUID`, `PayloadType`, etc.) are
      stripped automatically before the keys are applied as OpenCode config.
  - os: macos
    scope: other
    path: "~/.local/share/opencode/mcp-auth.json"
    format: json
    notes: |
      macOS OAuth credential store for MCP servers (NOT a server definition
      file). Confirmed by docs; file does not exist on this host because no
      MCP server here uses OAuth.
  - os: linux
    scope: other
    path: "~/.local/share/opencode/mcp-auth.json"
    format: json
    notes: "Linux OAuth credential store path — same as macOS."
  - os: windows
    scope: other
    path: "%USERPROFILE%\\.local\\share\\opencode\\mcp-auth.json"
    format: json
    notes: "Windows OAuth credential store path — OpenCode's data dir is home-relative `.local/share/opencode` on every OS, not %LOCALAPPDATA%."
cli_params:
  - flag: "opencode mcp add [name]"
    description: |
      Interactive wizard that adds a local or remote MCP server to the active
      config. Non-interactive flags: `--url <url>` for a remote server,
      `--env KEY=VALUE` (repeatable) for local-server env vars, and
      `--header KEY=VALUE` (repeatable) for remote-server headers. There is no
      `--transport` flag — `--url` selects the remote path, otherwise the
      wizard collects a local command.
    example: "opencode mcp add"
  - flag: "opencode mcp list"
    description: |
      List all configured MCP servers and their status (connected / failed /
      needs auth / disabled).
    example: "opencode mcp list"
  - flag: "opencode mcp ls"
    description: "Alias for `opencode mcp list`."
  - flag: "opencode mcp auth [name]"
    description: |
      Trigger the OAuth flow for an OAuth-enabled MCP server. If `name` is
      omitted, OpenCode prompts for a selection.
    example: "opencode mcp auth sentry"
  - flag: "opencode mcp auth list"
    description: |
      List OAuth-capable MCP servers and their auth status.
    example: "opencode mcp auth list"
  - flag: "opencode mcp auth ls"
    description: "Alias for `opencode mcp auth list`."
  - flag: "opencode mcp logout [name]"
    description: |
      Remove stored OAuth credentials for a server.
    example: "opencode mcp logout sentry"
  - flag: "opencode mcp debug <name>"
    description: |
      Show current auth status, test HTTP connectivity, and attempt the
      OAuth discovery flow for a server. Reports `MCP server not found` for
      unknown names (verified on v1.17.13).
    example: "opencode mcp debug sentry"
  - flag: "POST /mcp (opencode serve)"
    description: |
      HTTP API on the `opencode serve` headless server. Accepts
      `{ name, config }` and adds the server dynamically for the lifetime of
      that server instance. Confirmed in the published Server API spec.
  - flag: "GET /mcp (opencode serve)"
    description: |
      HTTP API on `opencode serve`. Returns `{ [name]: MCPStatus }` for every
      configured server.
  - flag: "PATCH /config (opencode serve)"
    description: |
      HTTP API on `opencode serve`. Updates the resolved config, which can
      add or modify MCP server entries at runtime.
env_vars:
  - name: OPENCODE_CONFIG
    effect: |
      Path to a custom OpenCode config file. Loaded between global and project
      configs.
  - name: OPENCODE_CONFIG_CONTENT
    effect: |
      Inline JSON config content. Loaded after `.opencode` directories and
      before managed config. This is the documented runtime-injection
      mechanism Claudine uses for one-run wrappers.
  - name: OPENCODE_CONFIG_DIR
    effect: |
      Path to a custom config directory that mirrors `.opencode`. Loaded
      after global config and `.opencode` directories, so it can override.
  - name: OPENCODE_TUI_CONFIG
    effect: "Path to a custom TUI-only config file (no MCP)."
  - name: OPENCODE_PERMISSION
    effect: |
      Inline JSON permissions. Because MCP tool approval rides the global
      `permission` model, this env var can shift MCP tool ask/allow/deny
      defaults for a single run.
server_schema:
  transports: ["local", "remote"]
  command_fields: ["type", "command", "environment", "cwd", "enabled", "timeout"]
  http_fields: ["type", "url", "headers", "oauth", "enabled", "timeout"]
  env_shape: |
    `environment` is an object mapping variable names to string values. Values
    support `{env:VARIABLE_NAME}` substitution (unset variables expand to the
    empty string). `{file:path/to/file}` substitutes file contents (relative
    paths resolve from the config file directory; absolute paths and `~` are
    accepted).
  auth_shape: |
    Remote servers support OAuth via an `oauth` object, with optional
    `clientId`, `clientSecret`, `scope`, `callbackPort`, and `redirectUri`
    (source-verified in `packages/opencode/src/mcp/index.ts`). Set
    `oauth: false` to disable automatic OAuth for API-key-style servers and
    use `headers` instead. Dynamic Client Registration (RFC 7591) is the
    default; v1.17.12 added refresh-token scope requests and per-server-URL
    auth-status scoping; v1.17.7 binds the OAuth callback to IPv4 loopback.
    Static header values (and OAuth client secrets) accept `{env:VARIABLE_NAME}`
    substitution. Tokens are persisted to `~/.local/share/opencode/mcp-auth.json`.
  notes: |
    Server id is the map key under `mcp.<name>`. `type` is required: `"local"`
    or `"remote"`. `oauth` accepts `false` (disable auto-OAuth) or an object
    with the keys above. The `timeout` field defaults to `30000` ms in
    source (`DEFAULT_TIMEOUT = 30_000` at `packages/opencode/src/mcp/index.ts:2418`)
    even though the docs state 5000 ms; treat the doc text as stale until
    reconciled.
server_capabilities:
  tools: full
  resources: partial
  prompts: full
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    Tools are fully exposed to the model and discovered at server startup.
    Server-side `ToolListChangedNotification` is honored (handler registered
    in source). Resources are partially supported: `resources/list`,
    `resources/read`, and `resources/templates/list` are wired in
    `packages/opencode/src/mcp/index.ts` (resource template listing added in
    v1.17.10), but `resources/subscribe` is NOT implemented — there is no
    `ResourceListChangedNotification` or `ResourceUpdated` handler, and the
    negative grep for `subscribe`/`listChanged`/`ResourceUpdated` returned
    zero matches. Prompts are fully supported (`prompts/list`, `prompts/get`)
    but the client does not register a `PromptListChangedNotification`
    handler, so prompt lists are snapshotted at startup. Tool and prompt
    names are registered as `<server-name>_<tool-or-prompt-name>`.
client_capabilities:
  roots: full
  sampling: none
  elicitation: none
  notes: |
    OpenCode declares a single capability to servers: `roots`. Sampling and
    elicitation are explicitly DISABLED in the source
    (`packages/opencode/src/mcp/index.ts:2420–2442`) with links to upstream
    tracking issues (`sampling` → anomalyco/opencode#11948, `elicitation` →
    anomalyco/opencode#23066). The `tasks` capability is also disabled
    (#28567). The roots handler responds to `roots/list` with the project
    working directory as a single `file://` URI (added in v1.17.7).
tool_surface:
  discovery: |
    Tools are fetched from each connected server at session startup. The
    `tools/list` call is the only discovery path; no on-demand or tool-search
    deferral is implemented. Server-side `ToolListChangedNotification` triggers
    a refresh.
  filtering: |
    MCP tools can be enabled or disabled globally or per-agent via the
    `permission` config (or the legacy `tools` key, deprecated as of v1.1.1
    and merged into `permission`). Glob patterns such as `mymcp_*` (match all
    tools for server `mymcp`) or `mymcp_search` (match a single tool) are
    supported; rules evaluate with "last match wins". Wildcards follow the
    `*` and `?` semantics.
  approval: |
    MCP tool calls use the same permission model as native tools
    (`allow` / `ask` / `deny` in the `permission` config, `--auto` to
    auto-approve non-denied calls). The `bash`-shaped wildcard
    `permission.bash` is the closest analog for fine-grained command
    approval. There is no MCP-specific approval policy.
  result_handling: |
    Tool results are passed to the model. v1.17.5 returns structured MCP
    tool output in a readable form; v1.17.8 surfaces the server's error
    text on failure and keeps long-running tool timeouts alive while the
    server reports progress. No native MCP result sanitization is
    documented.
  annotations_trusted: |
    Not documented. OpenCode does not describe how it handles MCP tool
    annotations (e.g., `readOnlyHint`, `destructiveHint`, `openWorldHint`,
    or vendor extensions). Treat annotations as hints only.
  notes: |
    Tools are registered with the server name as a prefix, so a server
    named `mymcp` exposing `search` becomes `mymcp_search` in the model's
    tool surface. v1.17.10 added per-server instructions to the session
    context; v1.17.12 prefers MCP content responses over structured output
    when both are present.
resource_surface:
  supported: true
  uri_schemes: []
  templates: true
  subscriptions: false
  exposure_model: |
    Resources are application-controlled context surfaced through internal
    OpenCode tools (`McpCatalog`). Servers advertising `resources/list` are
    enumerated and exposed via `listResources`; templates are enumerated
    separately via `listResourceTemplates` (added v1.17.10); `readResource`
    fetches by URI. Resource template tools are hidden when access is denied
    (v1.17.10). URI schemes are server-defined. There is no UI element
    documented for resource selection (e.g. `@`-mention autocomplete); the
    model and the OpenCode-internal `McpCatalog` choose.
  notes: |
    `resources/subscribe` is not implemented in the client; the source has
    no `ResourceListChangedNotification` or `ResourceUpdated` handler.
    Servers that push updates via `resources/updated` notifications are
    ignored — clients re-poll on next tool-call or session boundary.
prompt_surface:
  supported: true
  invocation: ""
  arguments: ""
  exposure_model: |
    Prompts are discovered via `prompts/list` and fetched via `prompts/get`
    with positional `arguments`. Both are wired in the source. The docs do
    not describe a slash-command or palette exposure; prompts are reachable
    through the OpenCode-internal `McpCatalog` (`getPrompt(name, args?)`).
  notes: |
    No `PromptListChangedNotification` handler is registered, so prompt
    lists are snapshotted at session start and refreshed on tool-call or
    session boundary, not in real time.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: deep
  notes: |
    Claudine can read `~/.config/opencode/opencode.json` (and the `.jsonc`
    and legacy `.opencode/opencode.json` siblings) plus the project-level
    `.opencode/opencode.json` and `opencode.json`, then normalize the `mcp`
    object into its catalog, and write the shape back. Later sources override
    earlier sources for conflicting keys, while non-conflicting keys are
    preserved (deep merge across the merged config). OpenCode does NOT
    provide a CLI command to remove a single MCP server — `opencode mcp`
    exposes `add` (interactive), `list`, `auth`, `logout`, `debug`, and the
    HTTP `POST /mcp` endpoint (server-instance scoped). Apply by file
    rewrite is required for removals.
runtime_injection:
  supported: true
  mechanism: |
    Set `OPENCODE_CONFIG_CONTENT` to inline JSON containing an `mcp` object
    before launching `opencode`. OpenCode loads this env var after the
    `.opencode` directories and before managed config, so it is an overlay
    on the standard chain rather than a full replacement. A second env var,
    `OPENCODE_CONFIG`, can point to a temp file containing the desired
    effective config if the overlay needs to be larger than a shell argv.
    For headless repeated runs against the same injected set, attach via
    `opencode run --attach <url>` to a long-lived `opencode serve` instance.
  limitations: |
    OAuth flows cannot complete in non-interactive `opencode run` mode;
    pre-authenticated servers or static-header auth are required. The
    injected config is an overlay, not a full replacement — user, project,
    and managed file configs still load unless the host runs with
    `OPENCODE_CONFIG` overriding all of them. There is no `--strict-mcp`
    flag (the prior research's claim of `POST /mcp` "exclusive" semantics
    is not supported by docs).
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens are persisted to `~/.local/share/opencode/mcp-auth.json`.
    Client secrets passed via `oauth.clientSecret` (or pre-existing client
    secrets) live in the same credential store and are not persisted in
    config files. Verified absent on this host because no OAuth server is
    configured here.
  token_scope: |
    Per remote MCP server URL. Dynamic Client Registration (RFC 7591) is the
    default; explicit `clientId` / `clientSecret` / `scope` override DCR.
    v1.17.12 added MCP refresh-token scope requests during the OAuth
    handshake and scopes auth status per server URL.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `environment` map or via
    `{env:VARIABLE_NAME}` substitution anywhere in the config. The process
    environment is otherwise inherited, so a stdio server has access to the
    full launch env plus its own `environment` map.
  notes: |
    `oauth: false` disables automatic OAuth, useful for API-key-style
    remote servers; combine with `headers` for `Authorization: Bearer ...`.
    v1.17.7 binds the OAuth callback to IPv4 loopback for better local
    auth reliability; v1.17.11 always prints the OAuth URL even when no
    browser flow is available; v1.17.12 surfaces OAuth completion errors
    rather than generic failures.
security:
  tool_filtering: |
    MCP tools are filtered through the global `permission` config (with
    glob patterns like `mymcp_*` or `mymcp_search`), or the legacy `tools`
    config (deprecated v1.1.1). Per-agent overrides are placed under
    `agent.<name>.tools`. No allowlist/denylist of MCP servers themselves
    is documented.
  server_trust: |
    Project-level `opencode.json` MCP servers are loaded like any other
    config key; OpenCode does not document a separate repo-trust dialog for
    MCP. The legacy `disableSideloadFlags`-style gate is a Claude Code
    concept, not OpenCode. Managed config (file-based or MDM-deployed) takes
    precedence and cannot be overridden by user or project config.
  env_sanitization: |
    Each local server receives its explicit `environment` map plus inherited
    process environment. No documented env-scrubbing mode exists
    specifically for MCP servers; `OPENCODE_PERMISSION` can shift tool
    approvals but does not strip subprocess env.
  sandbox_interaction: |
    MCP server subprocesses run as ordinary local processes; no OS-level
    sandbox or container boundary is documented. Stderr is piped and
    surfaced through `LoggingMessageNotification` (v1.17.4+) and into the
    `opencode` log channel.
  response_filtering: |
    No native MCP response sanitization is documented. Claudine's `protect`
    layer should treat MCP tool results as untrusted. v1.17.8 surfaces the
    server's error text on tool failure; v1.17.10 prevents MCP resource
    tools from colliding when servers expose the same keys.
  notes: |
    Organizations should use managed config (`/Library/Application Support/opencode/opencode.json` on macOS, `/etc/opencode/opencode.json` on Linux, `%ProgramData%\opencode\opencode.json` on Windows, or the `ai.opencode.managed` MDM domain) to enforce an approved MCP server surface.
mode_specific:
  interactive: |
    MCP servers start with the session. OAuth flows can be completed
    through `opencode mcp auth <name>` and through the TUI when prompted.
    The TUI surfaces the connected/failed/needs-auth status.
  non_interactive_run: |
    `opencode run` honors the same MCP config chain, but OAuth flows cannot
    complete interactively; pre-authenticated servers or static headers are
    required. `opencode run --attach <url>` attaches to a long-lived
    `opencode serve` to skip MCP cold-boot time.
  serve: |
    `opencode serve` exposes the MCP control surface via HTTP: `GET /mcp`
    for status, `POST /mcp` for dynamic add, and `PATCH /config` for runtime
    config edits (including MCP entries). `opencode web` is the same HTTP
    server with a browser UI; both honor `OPENCODE_SERVER_PASSWORD` for
    basic auth.
  acp: |
    `opencode acp` starts an Agent Client Protocol server. ACP resource
    text sourcing was fixed on Windows and other cross-platform path cases
    in v1.17.10.
failures:
  server_fails_to_start: |
    Marked failed in `opencode mcp list`; not retried by the OpenCode layer
    (verified by negative grep for `retry`/`reconnect`/`backoff` in
    `packages/opencode/src/mcp/index.ts`). The underlying SDK handles
    transport-level behavior.
  stdio_exits_mid_session: |
    The client is removed from state and a warning is logged; no
    auto-reconnect. To resume, the user starts a new session.
  http_transient_startup_error: |
    Not retried at the OpenCode layer. The SDK handles transport-level
    retries.
  http_mid_session_disconnect: |
    v1.17.12 reconnects MCP servers after OAuth even if the server was
    disabled; v1.17.5 recovers expired MCP sessions and clears closed MCP
    clients so stale connections do not linger. No documented retry budget.
  tool_call_timeout: |
    Aborted after `mcp.timeout` (source default `DEFAULT_TIMEOUT = 30_000`
    ms — not the 5000 ms the docs state). Per-call fallback:
    `mcp.<name>.timeout ?? experimental.mcp_timeout` (config key).
  tool_output_too_large: |
    No documented MCP-specific output limit.
  refresh_token_rejected: |
    Surfaced via the per-server-URL auth status; v1.17.12 shows MCP OAuth
    completion errors directly rather than generic failures.
gaps:
  - |
    OpenCode does not state which MCP protocol version it implements; the
    version is delegated to `@modelcontextprotocol/sdk` (verified in
    `packages/opencode/src/mcp/index.ts:2480`).
  - |
    The docs claim `timeout` defaults to 5000 ms; the source code defaults
    to 30_000 ms (`DEFAULT_TIMEOUT = 30_000` at line 2418). Treat the
    docs' figure as stale until reconciled.
  - |
    Sampling (`sampling/createMessage`) is explicitly DISABLED in the
    source capability declaration (commented out with a link to
    anomalyco/opencode#11948).
  - |
    Elicitation (`elicitation/create`) is explicitly DISABLED in the source
    capability declaration (commented out with a link to
    anomalyco/opencode#23066).
  - |
    The `tasks` capability is also DISABLED (link to
    anomalyco/opencode#28567).
  - |
    `resources/subscribe` is not implemented; there is no
    `ResourceListChangedNotification` or `ResourceUpdated` handler.
  - |
    `prompt_list_changed` is not subscribed at the OpenCode layer; prompt
    lists are snapshotted at startup.
  - |
    `opencode mcp remove` does not exist in v1.17.13; removing a server
    requires rewriting the `mcp` object in a config file.
  - |
    No repo-trust dialog for project-level `opencode.json` MCP servers is
    documented.
  - |
    No OS-level sandbox or credential-scrubbing boundary for MCP servers
    is documented.
  - |
    No first-class MCP resource `@`-autocomplete exposure (e.g., model or
    user-facing resource selection UI) is documented; the public surface
    is the internal `McpCatalog`.
changes:
  - "Curation edit (2026-07-03): corrected Windows config_files paths — OpenCode resolves config/data dirs home-relative on every OS, so the user config is %USERPROFILE%\\.config\\opencode\\ (not %APPDATA%) and the OAuth store is %USERPROFILE%\\.local\\share\\opencode\\ (not %LOCALAPPDATA%); legacy single-dot paths rewritten in Windows form. Cross-validated against the agent-cli topic's host-evidence records."
  - "Confirmed `support: runtime_injection` is the strongest single path for Claudine — `OPENCODE_CONFIG_CONTENT` is the documented one-run mechanism; persistent-config import/export is a secondary story."
  - "Confirmed the server-side capability declaration in source (`packages/opencode/src/mcp/index.ts:2420–2442`): only `roots` is enabled; `sampling`, `elicitation`, and `tasks` are commented out with links to upstream GitHub issues. Prior research recorded these as `unknown`."
  - "Confirmed `roots` is implemented end-to-end: `client.setRequestHandler(ListRootsRequestSchema, ...)` returns the project directory as a `file://` URI. Added in v1.17.7."
  - "Confirmed `resources/list`, `resources/read`, and `resources/templates/list` are wired (resource template listing added v1.17.10). `resources/subscribe` is NOT implemented — no `ResourceListChangedNotification` or `ResourceUpdated` handler."
  - "Confirmed `prompts/list` and `prompts/get` are wired (`getPrompt`, `readResource` exports). No `PromptListChangedNotification` handler."
  - "Verified locally on v1.17.13: `opencode mcp --help` exposes `add`, `list`/`ls`, `auth`, `logout`, `debug` — NO `remove` command (prior research incorrectly listed one)."
  - "Verified locally on v1.17.13: `opencode mcp debug <unknown>` returns `MCP server not found`; `opencode mcp auth list` returns `No OAuth-capable MCP servers configured`."
  - "Verified locally on v1.17.13: `~/.config/opencode/config.json` holds the active `mcp` object (two local servers, both `enabled: false`); `~/.config/opencode/opencode.jsonc` does NOT hold MCP entries; `~/.local/share/opencode/mcp-auth.json` does NOT exist because no OAuth server is configured."
  - "Verified locally on v1.17.13: `opencode debug config` loads 6 user/project config paths in this order: `~/.config/opencode/config.json`, `~/.config/opencode/opencode.json`, `~/.config/opencode/opencode.jsonc`, `<project>/.opencode/opencode.json`, `<project>/.opencode/opencode.jsonc`, `~/.opencode/opencode.json`, `~/.opencode/opencode.jsonc`."
  - "Verified `opencode mcp add --help`: no `--transport` flag — the wizard distinguishes local vs remote via the presence of `--url`; remote headers via `--header KEY=VALUE` (repeatable); local env via `--env KEY=VALUE` (repeatable)."
  - "Confirmed Server HTTP API: `GET /mcp` (status), `POST /mcp` (dynamic add), `PATCH /config` (config edit). All serve-scoped, not persistent."
  - "Documented discrepancy: docs state `timeout` default is 5000 ms; source `DEFAULT_TIMEOUT = 30_000` (`packages/opencode/src/mcp/index.ts:2418`). Treat the doc text as stale."
  - "Documented v1.17.x release notes: v1.17.4 added `cwd` for local servers and `LoggingMessageNotification` for MCP stderr; v1.17.5 recovered expired MCP sessions and cleared closed clients; v1.17.6 declared OpenCode's supported client capabilities (improves server compatibility); v1.17.7 added `roots/list` and bound OAuth callback to IPv4 loopback; v1.17.8 keeps long-running tool timeouts alive on progress, surfaces server error text, and escapes OAuth error pages; v1.17.10 added server instructions to context, MCP resource template listing and read tools, hidden resource template tools when access denied, and prevented resource-tool key collisions; v1.17.11 always prints the MCP OAuth URL even without browser; v1.17.12 reconnects MCP after OAuth even if disabled, requests refresh-token scope, scopes auth status per server URL, surfaces OAuth completion errors, and prefers MCP content responses over structured output."
  - "No retry/backoff/reconnect logic in the OpenCode layer (verified by negative grep in `packages/opencode/src/mcp/index.ts`); the SDK handles transport-level behavior."
  - "Refreshed sources list: MCP servers docs, config docs, CLI docs, server docs, permissions docs, tools docs, enterprise docs, and the GitHub release notes for v1.17.4–v1.17.13."
requires_claudine_update: true
reason: |
  Three Claudine behaviors are now provable from the source rather than
  guessed: (1) the `mcp` module's `server_capabilities` and
  `client_capabilities` should reflect that OpenCode advertises ONLY
  `roots` to servers — `sampling` and `elicitation` are commented-out
  upstream issues — so the catalog should not surface those as supported;
  (2) the catalog's `resource_surface` should move from `none`/`unknown`
  to `partial` because `resources/list`, `resources/read`, and
  `resources/templates/list` are wired but `resources/subscribe` is not;
  (3) the runtime injector should not assume `opencode mcp remove`
  exists — apply-remove must rewrite the `mcp` object in a config file
  or use the `PATCH /config` HTTP endpoint on `opencode serve`. The new
  per-server `timeout` source default (30_000 ms vs docs' 5000 ms) and
  the new `experimental.mcp_timeout` fallback should be reflected in
  provider metadata so the wrapper layer can warn when set.
---

# MCP Support in OpenCode CLI

## Overview

OpenCode supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io) as a first-class extension surface. MCP servers can be local command-based processes or remote HTTP-based services. Configuration lives inside OpenCode's unified JSON/JSONC config, and the CLI provides a small set of `opencode mcp` commands for management (`add`, `list`/`ls`, `auth`/`auth list`/`auth ls`, `logout`, `debug`; no `remove`). The strongest single integration path for Claudine is **runtime injection** via `OPENCODE_CONFIG_CONTENT`, which lets a wrapper inject an `mcp` overlay for one run without mutating persistent config. Import/export/sync of the persistent config files (`~/.config/opencode/opencode.json[.c]`, `<project>/opencode.json`, `.opencode/opencode.json`, the legacy `~/.opencode/opencode.json[.c]`) is a secondary story — the apply path is partial because no `remove` CLI exists.

Surface inventory (one-line):

- **Tools** — exposed: tool names registered as `<server>_<tool>`, full tool-call surface, `ToolListChangedNotification` honored.
- **Resources** — exposed partially: `resources/list`, `resources/read`, and `resources/templates/list` are wired (template listing added v1.17.10); `resources/subscribe` is NOT implemented — no `ResourceListChangedNotification` or `ResourceUpdated` handler.
- **Prompts** — exposed: `prompts/list` and `prompts/get` are wired, but no `PromptListChangedNotification` handler — prompt lists are snapshotted at startup.
- **Roots** — exposed: OpenCode declares `roots` to servers and answers `roots/list` with the project directory as a `file://` URI (added v1.17.7).
- **Sampling** — not exposed: explicitly disabled in source (`packages/opencode/src/mcp/index.ts:2426`) with a link to upstream issue #11948.
- **Elicitation** — not exposed: explicitly disabled in source (line 2430) with a link to upstream issue #23066.

## Protocol and Transports

OpenCode's documentation does not cite a specific MCP protocol version date; the version is delegated to the underlying `@modelcontextprotocol/sdk` Client. The source wires three transports:

| OpenCode type | MCP equivalent | How it is configured | Source evidence |
| :------------ | :------------- | :------------------- | :-------------- |
| `local` | stdio | `type: "local"` with a `command` array | `StdioClientTransport({ stderr: "pipe", command, args, cwd, env })` |
| `remote` | Streamable HTTP, then legacy HTTP+SSE | `type: "remote"` with a `url` | `StreamableHTTPClientTransport` tried first, `SSEClientTransport` as fallback |
| — | WebSocket / custom | not implemented | negative grep on the source |

Lifecycle behavior observed and verified:

- **Local servers** are spawned as child processes at session start. Stderr is piped and surfaced through `LoggingMessageNotification` (v1.17.4+) and into the `opencode` log channel.
- **Remote servers** try Streamable HTTP first and fall back to legacy HTTP+SSE; the choice is per server, not global.
- **Roots** — OpenCode is the only capability advertised to servers. The roots handler responds to `roots/list` with the project directory as a single `file://` URI (added in v1.17.7).
- **Capability discovery** is delegated to the SDK. The source has no OpenCode-side retry, backoff, or reconnect logic (verified by negative grep); the SDK handles transport-level behavior.
- **Reconnect after OAuth** — v1.17.12 reconnects MCP servers after OAuth even if the server was disabled; v1.17.5 recovers expired MCP sessions and clears closed MCP clients.
- **Timeouts** — the per-call timeout defaults to `DEFAULT_TIMEOUT = 30_000` ms (source line 2418), not the 5000 ms the docs state. Per-call fallback chain: `mcp.<name>.timeout ?? experimental.mcp_timeout`. Long-running tool calls keep their timeout alive while the server reports progress (v1.17.8).

## Configuration

MCP servers are configured under the top-level `mcp` key in OpenCode config files. Configuration files are **merged**, not replaced — later sources override earlier ones for conflicting keys, while non-conflicting settings from all sources are preserved.

### Config-file precedence

Per the [config docs](https://opencode.ai/docs/config/#precedence-order), sources load in this order (later overrides earlier for conflicting keys):

1. Remote config (`.well-known/opencode`, HTTPS) — organizational defaults
2. Global user config (`~/.config/opencode/opencode.json` / `.jsonc`, legacy `config.json`)
3. Custom config (`OPENCODE_CONFIG` env var)
4. Project config (`<project>/opencode.json` / `.opencode/opencode.json[.c]`)
5. `.opencode` directories — agents, commands, modes, plugins, skills, tools, themes, MCP-related `opencode.json` variants
6. Inline config (`OPENCODE_CONFIG_CONTENT` env var)
7. Managed (file-based): `/Library/Application Support/opencode/opencode.json` (macOS), `/etc/opencode/opencode.json` (Linux), `%ProgramData%\opencode\opencode.json` (Windows)
8. macOS managed preferences (`ai.opencode.managed` MDM domain — `.mobileconfig` PayloadType)

Observed loading order on this host (v1.17.13, `opencode debug config` log output):

```
~/.config/opencode/config.json        ← legacy user
~/.config/opencode/opencode.json
~/.config/opencode/opencode.jsonc
<project>/.opencode/opencode.json
<project>/.opencode/opencode.jsonc
~/.opencode/opencode.json             ← legacy single-dot
~/.opencode/opencode.jsonc
```

### Per-OS config locations

| Scope | macOS | Linux | Windows |
| :---- | :---- | :---- | :------ |
| User (modern) | `~/.config/opencode/opencode.json[.c]` | `~/.config/opencode/opencode.json[.c]` | `%APPDATA%\opencode\opencode.json[.c]` |
| User (legacy) | `~/.opencode/opencode.json[.c]` | `~/.opencode/opencode.json[.c]` | `~/.opencode/opencode.json[.c]` |
| Project | `<project>/opencode.json[.c]` | `<project>/opencode.json[.c]` | `<project>\opencode.json[.c]` |
| Project `.opencode` | `<project>/.opencode/opencode.json[.c]` | `<project>/.opencode/opencode.json[.c]` | `<project>\.opencode\opencode.json[.c]` |
| TUI-only | `~/.config/opencode/tui.json` | `~/.config/opencode/tui.json` | `%APPDATA%\opencode\tui.json` |
| Managed (file) | `/Library/Application Support/opencode/opencode.json[.c]` | `/etc/opencode/opencode.json[.c]` | `%ProgramData%\opencode\opencode.json[.c]` |
| Managed (MDM) | `/Library/Managed Preferences/<user>/ai.opencode.managed.plist` and `/Library/Managed Preferences/ai.opencode.managed.plist` | n/a | n/a (use HKLM registry equivalent for enterprise) |
| OAuth store | `~/.local/share/opencode/mcp-auth.json` | `~/.local/share/opencode/mcp-auth.json` | `%LOCALAPPDATA%\opencode\mcp-auth.json` |

### Example config

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "filesystem": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem", "."],
      "environment": { "NODE_ENV": "production" },
      "cwd": "./sandbox",
      "enabled": true,
      "timeout": 5000
    },
    "sentry": {
      "type": "remote",
      "url": "https://mcp.sentry.dev/mcp",
      "oauth": {}
    },
    "api-key-server": {
      "type": "remote",
      "url": "https://api.example.com/mcp",
      "oauth": false,
      "headers": { "Authorization": "Bearer {env:MY_API_KEY}" }
    }
  }
}
```

Config values support `{env:VARIABLE_NAME}` substitution (unset → empty string) and `{file:path/to/file}` substitution (relative paths resolve from the config file directory; absolute paths and `~` are accepted).

## Server Definition Shape

A server definition under `mcp.<name>` accepts the following fields:

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `type` | all | `"local"` or `"remote"` |
| `command` | local | Array of command + arguments |
| `url` | remote | Remote MCP endpoint URL |
| `environment` | local | Map of environment variables |
| `cwd` | local | Working directory for the server process (relative paths resolve from the workspace; added v1.17.4) |
| `headers` | remote | Static header map (supports `{env:VAR}` and `{file:...}` substitution) |
| `oauth` | remote | OAuth config object, or `false` to disable auto-OAuth |
| `enabled` | all | Whether the server is active |
| `timeout` | all | Per-server wall-clock timeout in ms (source default `30000`, docs state `5000`) |

The `oauth` object accepts:

| Field | Description |
| :---- | :---------- |
| `clientId` | Pre-registered OAuth client ID |
| `clientSecret` | Pre-registered OAuth client secret (stored in the credential store, not in config) |
| `scope` | Space-separated scopes to request (v1.17.12 added refresh-token scope) |
| `callbackPort` | Pin the OAuth callback port |
| `redirectUri` | Override the OAuth redirect URI |

The combination of static `headers` + `oauth: false` is the documented pattern for API-key-style remote servers.

## Tools, Resources, and Prompts

### Tools

OpenCode exposes MCP tools to the LLM alongside built-in tools. Tool names are registered with the server name as a prefix, so a server named `mymcp` exposing `search` becomes available to the model as `mymcp_search`.

Discovery and refresh:

- `tools/list` is called at server startup.
- `ToolListChangedNotification` is honored; tool lists refresh on notification.
- v1.17.10 added per-server MCP instructions to the session context; v1.17.12 prefers MCP content responses over structured output when both are present.

Filtering and approval:

- Filtering is global or per-agent via the `permission` config (or the legacy `tools` config, deprecated v1.1.1 and merged into `permission`).
- Glob patterns are supported: `mymcp_*` matches all tools of server `mymcp`; `mymcp_search` matches a single tool. Rules evaluate with "last match wins".
- MCP tool calls use the same `allow` / `ask` / `deny` model as native tools. `--auto` auto-approves non-denied requests.
- There is no documented MCP-specific approval policy or trust dialog for project-level servers.

Tool output:

- Per-server `timeout` overrides the default; per-call fallback chain is `mcp.<name>.timeout ?? experimental.mcp_timeout` (config key, not env var).
- v1.17.8 surfaces server error text on failure and keeps long-running tool timeouts alive while progress is reported.
- No native MCP output sanitization is documented.

### Resources

MCP resources are partially supported. The OpenCode client wires three SDK methods:

- `listResources` — enumerate resources from a connected server.
- `listResourceTemplates` — enumerate resource templates (added v1.17.10).
- `readResource` — fetch by URI (passed via `mcpCatalog.readResource(clientName, uri)`).

The source also wires `McpCatalog` resource helpers; the public exposure is the internal `McpCatalog` rather than a documented model or UI surface. There is no `@`-autocomplete UI element documented for MCP resources.

What is NOT implemented:

- `resources/subscribe` — no `ResourceListChangedNotification` or `ResourceUpdated` handler. Negative grep on the source confirms.
- Real-time resource update push from servers.

When access to a resource template is denied, the template tool is hidden rather than shown as failed (v1.17.10). When two servers expose the same resource key, collisions are prevented (v1.17.10).

### Prompts

MCP prompts are fully reachable but the public surface is the internal `McpCatalog`:

- `prompts/list` is called at server startup; the result is snapshotted.
- `prompts/get` is called on demand with positional `arguments`.
- No `PromptListChangedNotification` handler is registered — prompt lists are refreshed on the next tool call or session boundary, not in real time.
- The docs do not describe a slash-command or palette exposure for MCP prompts.

## Roots, Sampling, and Elicitation

### Roots

OpenCode is the only client capability advertised to servers. The source wires a handler for `ListRootsRequestSchema`:

```typescript
client.setRequestHandler(ListRootsRequestSchema, () =>
  Promise.resolve({ roots: [{ uri: pathToFileURL(directory).href }] })
)
```

The handler returns a single `file://` URI for the project directory. The boundary is exactly the project root OpenCode was launched from — there is no concept of additional roots, no per-tool boundary, and no wildcard. This capability was added in v1.17.7.

### Sampling

`SamplingMessageRequest` and `sampling/createMessage` are explicitly DISABLED in the source capability declaration:

```typescript
capabilities: {
  // https://github.com/anomalyco/opencode/issues/11948
  // sampling: {},
  ...
}
```

Servers cannot request LLM completions through OpenCode. There is no `CreateMessageRequestSchema` handler anywhere in the source. Claudine should treat OpenCode's sampling posture as `none` until upstream issue #11948 ships.

### Elicitation

`ElicitRequest` and `elicitation/create` are explicitly DISABLED in the source capability declaration:

```typescript
capabilities: {
  ...
  // https://github.com/anomalyco/opencode/issues/23066
  // elicitation: {},
  ...
}
```

Servers cannot request structured user input through OpenCode. There is no `ElicitRequestSchema` handler anywhere in the source. Claudine should treat OpenCode's elicitation posture as `none` until upstream issue #23066 ships.

### Tasks

Also explicitly DISABLED (anomalyco/opencode#28567). MCP task progress / task-augmented requests are not yet supported.

## Import, Export, and Sync

Claudine can treat OpenCode as an `import_sync` target with caveats:

- **Import** — read `~/.config/opencode/opencode.json` (and `.jsonc`/legacy siblings), project `<project>/opencode.json` and `.opencode/opencode.json[.c]`, then normalize the `mcp` object into the catalog.
- **Export** — write provider-shaped JSON back to those files.
- **Apply** — partial. `opencode mcp add` accepts a server interactively (with `--url`/`--env`/`--header` for non-interactive flags), but there is no `opencode mcp remove`; removals require rewriting the `mcp` object in a config file. The HTTP API on `opencode serve` exposes `POST /mcp` (dynamic add) and `PATCH /config` (config edit), both server-instance scoped.

Merge semantics:

- The config as a whole is merged: later sources override conflicting keys, non-conflicting keys are preserved (deep merge across the merged config).
- Server entries are not merged across sources within the same precedence tier; the highest-precedence tier's whole definition wins.

## Runtime Injection

For one-run injection without mutating persistent config, Claudine uses `OPENCODE_CONFIG_CONTENT`:

```bash
OPENCODE_CONFIG_CONTENT='{"mcp":{"filesystem":{"type":"local","command":["npx","-y","@modelcontextprotocol/server-filesystem","."]}}}' \
  opencode run "summarize"
```

The inline JSON is loaded after `.opencode` directories and before managed config, so it is an overlay rather than a full replacement of the persistent config chain. For larger overlays, point `OPENCODE_CONFIG` at a temp file instead.

For headless repeated runs that should share a long-lived MCP session, use `opencode run --attach <url>` to attach to a running `opencode serve` instance:

```bash
opencode serve &
opencode run --attach http://localhost:4096 "summarize"
```

Limitations:

- The injected config is an overlay, not a full replacement. User, project, and managed file configs still load unless the host runs with `OPENCODE_CONFIG` overriding all of them.
- OAuth flows cannot complete in non-interactive `opencode run` mode; pre-authenticated servers or static headers are required.
- v1.17.11 always prints the MCP OAuth URL even when no browser flow is available, so manual sign-in is still possible.

## Authorization and Credentials

OpenCode supports OAuth and static-header auth for remote MCP servers:

| Pattern | Where configured | Credential storage |
| :------ | :--------------- | :----------------- |
| Static header | `headers.Authorization` | In config file (supports `{env:VAR}`) |
| OAuth dynamic | `oauth: {}` | `~/.local/share/opencode/mcp-auth.json` |
| OAuth pre-registered | `oauth.clientId` + `oauth.clientSecret` + `oauth.scope` | Same token file |
| OAuth disabled | `oauth: false` + `headers` | n/a |

OAuth details:

- Dynamic Client Registration (RFC 7591) is the default when no `clientId` is set.
- `opencode mcp auth <name>` triggers the flow manually.
- `opencode mcp logout <name>` clears stored tokens.
- `oauth: false` disables automatic OAuth, useful for API-key-style remote servers.
- v1.17.12 added MCP refresh-token scope requests during the OAuth handshake and scopes auth status per server URL.
- v1.17.11 always prints the OAuth URL even when no browser flow is available.
- v1.17.7 binds the OAuth callback to IPv4 loopback for better local auth reliability.
- v1.17.12 surfaces OAuth completion errors directly rather than generic failures.

For local servers, secrets should be passed through the per-server `environment` map or via `{env:VAR}` substitution anywhere in the config.

## Security Model

- **Tool filtering** — use the global `permission` config (or legacy `tools`, deprecated v1.1.1) with glob patterns such as `mymcp_*` or `mymcp_search`. Per-agent overrides are placed under `agent.<name>.tools`. No allowlist/denylist of MCP servers themselves is documented.
- **Server trust** — project-level `opencode.json` MCP servers are loaded like any other config key; OpenCode does not document a separate repo-trust dialog for MCP servers. Managed config (file-based or MDM-deployed) takes precedence and cannot be overridden by user or project config.
- **Environment** — local servers inherit the user's process environment plus their explicit `environment` map. No MCP-specific env scrubbing is documented; `OPENCODE_PERMISSION` can shift tool approvals but does not strip subprocess env.
- **Sandboxing** — MCP server subprocesses are ordinary local processes; no OS-level sandbox or container boundary is described. Stderr is piped and surfaced through `LoggingMessageNotification` (v1.17.4+) and into the `opencode` log channel.
- **Response filtering** — no native MCP result sanitization is documented. Claudine's `protect` layer should treat MCP tool results as untrusted. v1.17.8 surfaces the server's error text on tool failure; v1.17.10 prevents MCP resource tools from colliding when servers expose the same keys.

Organizations should use managed config (`/Library/Application Support/opencode/opencode.json` on macOS, `/etc/opencode/opencode.json` on Linux, `%ProgramData%\opencode\opencode.json` on Windows, or the `ai.opencode.managed` MDM domain) to enforce an approved MCP server surface.

## Mode-Specific Behavior

### Interactive mode (TUI)

- MCP servers start with the session.
- OAuth flows can be completed through `opencode mcp auth <name>` and through the TUI when prompted.
- The TUI surfaces connected / failed / needs-auth / disabled status per server.

### Non-interactive mode (`opencode run`)

- MCP servers configured in persistent config still load.
- OAuth flows cannot complete interactively; pre-authenticated servers or static headers are required.
- `opencode run --attach <url>` attaches to a long-lived `opencode serve` to skip MCP cold-boot time.
- v1.17.11 always prints the MCP OAuth URL even when no browser flow is available, so manual sign-in is still possible from a non-interactive shell.

### Server mode (`opencode serve`)

- Exposes the HTTP Server API:
  - `GET /mcp` — server status
  - `POST /mcp` — dynamic add (body: `{ name, config }`)
  - `PATCH /config` — runtime config edit (can include MCP entries)
- Honors `OPENCODE_SERVER_PASSWORD` for HTTP basic auth.
- v1.17.12 reconnects MCP servers after OAuth even if the server was disabled.

### ACP mode (`opencode acp`)

- Starts an Agent Client Protocol server over stdio using nd-JSON.
- ACP resource text sourcing was fixed on Windows and other cross-platform path cases in v1.17.10.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Local server fails to start | Marked failed in `opencode mcp list`; no OpenCode-side retry |
| Stdio server exits mid-session | Client removed from state and a warning logged; no auto-reconnect |
| Remote transient startup error | No OpenCode-side retry (verified by negative grep); SDK handles transport-level behavior |
| Remote mid-session disconnect | v1.17.12 reconnects MCP servers after OAuth even if the server was disabled; v1.17.5 recovers expired MCP sessions and clears closed clients |
| Tool call timeout | Aborted after `mcp.timeout` (source default `30_000` ms; not the 5000 ms the docs state) |
| Long-running tool progress | v1.17.8 keeps the timeout alive while progress is reported |
| Tool output too large | No documented MCP-specific output limit |
| Refresh-token rejected | Surfaced via per-server-URL auth status (v1.17.12) |
| OAuth completion error | Surfaces server-provided text directly (v1.17.12) instead of generic failure |
| OAuth flow without browser | v1.17.11 always prints the OAuth URL for manual sign-in |

## Gaps

- The MCP protocol version date is not documented or set in source — it is delegated to `@modelcontextprotocol/sdk`.
- The docs claim `timeout` defaults to 5000 ms; the source `DEFAULT_TIMEOUT = 30_000` ms. Treat the docs as stale.
- Sampling and elicitation are explicitly disabled in source (upstream issues #11948 and #23066); treat both as `none`.
- The `tasks` capability is also disabled (upstream issue #28567).
- `resources/subscribe` is not implemented; no `ResourceListChangedNotification` or `ResourceUpdated` handler exists.
- No `PromptListChangedNotification` handler; prompt lists are snapshotted at startup.
- No `opencode mcp remove` CLI command — apply-remove requires file rewrite or `PATCH /config` on a running server.
- No documented repo-trust dialog for project-level `opencode.json` MCP servers.
- No documented OS-level sandbox or credential-scrubbing boundary for MCP servers.
- No documented MCP resource `@`-autocomplete UI element (selection happens through the internal `McpCatalog`).
- The Server HTTP API is scoped to a running `opencode serve` instance — not a persistent apply path.

## Claudine Integration Notes

- Treat OpenCode as `support: runtime_injection` because `OPENCODE_CONFIG_CONTENT` is the documented one-run mechanism and does not require mutating user or project config. Persistent-config import/export is real but secondary.
- Map Claudine's normalized catalog to OpenCode's `mcp` object shape: `type: "local"` for stdio servers (command array), `type: "remote"` for HTTP/SSE servers (url + headers + oauth).
- For one-run wrappers, prefer `OPENCODE_CONFIG_CONTENT` over rewriting `~/.config/opencode/opencode.json`. For larger overlays, point `OPENCODE_CONFIG` at a temp file.
- For repeated runs that share a long-lived MCP session, use `opencode run --attach <url>` against a running `opencode serve`.
- Do not assume `opencode mcp remove` exists; apply-remove must rewrite the `mcp` object in a config file or use `PATCH /config` on a running server.
- Surface the `roots`-only client capability in `client_capabilities` — do not advertise sampling or elicitation support because they are explicitly disabled in source.
- Surface the partial resource surface (`resources/list`/`read`/`templates/list` supported, `resources/subscribe` not) in `resource_surface` so the wrapper does not promise push updates.
- Use `~/.local/share/opencode/mcp-auth.json` as the canonical OAuth credential store path; do not attempt to write OAuth tokens to config files.
- Defensively scan MCP tool results in the `protect` layer; OpenCode does not provide native response sanitization.
- Be aware of the `timeout` doc/source discrepancy (5000 ms vs 30_000 ms). When the wrapper reads the resolved config from `opencode debug config`, trust the runtime behavior over the docs.

## Sources

- [OpenCode MCP servers docs](https://opencode.ai/docs/mcp-servers/)
- [OpenCode config docs](https://opencode.ai/docs/config/)
- [OpenCode CLI reference](https://opencode.ai/docs/cli/)
- [OpenCode server docs](https://opencode.ai/docs/server/)
- [OpenCode permissions docs](https://opencode.ai/docs/permissions/)
- [OpenCode tools docs](https://opencode.ai/docs/tools/)
- [OpenCode enterprise docs](https://opencode.ai/docs/enterprise/)
- [OpenCode ACP docs](https://opencode.ai/docs/acp/)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)
- [OpenCode MCP source (`packages/opencode/src/mcp/index.ts`)](https://github.com/anomalyco/opencode/blob/dev/packages/opencode/src/mcp/index.ts)
- [OpenCode release notes — v1.17.4 through v1.17.13](https://github.com/anomalyco/opencode/releases)
- Local observation: `opencode --version` ⇒ `1.17.13`; `opencode mcp --help` shows `add`, `list`/`ls`, `auth`, `logout`, `debug` and no `remove`; `opencode mcp debug <unknown>` returns `MCP server not found`; `opencode mcp auth list` returns `No OAuth-capable MCP servers configured`; `opencode debug config` loads seven paths in the order documented above; `~/.config/opencode/config.json` is the file that holds the active `mcp` object on this host (two local servers, both `enabled: false`); `~/.local/share/opencode/mcp-auth.json` does not exist because no OAuth server is configured.