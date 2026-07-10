---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://kilocode.ai/docs/automate/mcp/overview
support: runtime_injection
protocol:
  versions: ["2024-11-05"]
  transports: [stdio, streamable_http, http_sse]
  lifecycle: |
    The MCP service iterates the merged `mcp` map at session start and
    spawns one `Client` per enabled server using the upstream
    `@modelcontextprotocol/sdk`. Local servers run as stdio child
    processes with `StdioClientTransport`; the runtime sets `cwd` to the
    instance launch directory and `env` to `{ ...process.env,
    ...mcp.environment }`. Remote servers try `StreamableHTTPClientTransport`
    first; on non-auth transport errors the service falls back to the
    legacy `SSEClientTransport`. Auth errors short-circuit the loop and
    surface `needs_auth` / `needs_client_registration` status that points
    the user at `kilo mcp auth <name>`. The `kilo mcp debug <name>`
    command sends an explicit `initialize` with `protocolVersion:
    "2024-11-05"`, the version the client advertises. `tools/list_changed`
    notifications trigger an in-place refetch and a `mcp.tools.changed`
    bus event. SSE was deprecated by the upstream MCP specification on
    2025-03-26; Streamable HTTP is the recommended remote transport.
  notes: |
    Kilo's MCP code lives under `packages/opencode/src/mcp/` and is
    forked from OpenCode. User-facing config uses two transport names —
    `"local"` (stdio) and `"remote"` (HTTP/SSE) — and the runtime picks
    the upstream SDK transport based on those. There is no Kilo-level
    WebSocket transport and no documented interactive protocol-version
    negotiation beyond the hard-coded `2024-11-05` initialize message.
config_files:
  - os: macos
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    notes: |
      Primary global config. The CLI also accepts `kilo.json`,
      `opencode.jsonc`, `opencode.json`, and `config.json` in the same
      directory (first existing wins; `kilo.jsonc` is the default seed
      with a `$schema` reference). The legacy `opencode.json[c]` paths
      remain supported for backward compatibility.
  - os: linux
    scope: user
    path: "~/.config/kilo/kilo.jsonc"
    format: jsonc
    notes: |
      Same XDG config home as macOS. `$XDG_CONFIG_HOME/kilo/kilo.jsonc`
      overrides the default when set.
  - os: windows
    scope: user
    path: "C:\\Users\\<user>\\.config\\kilo\\kilo.jsonc"
    format: jsonc
    notes: |
      Windows user config lives under `%USERPROFILE%\.config\kilo\`,
      not `%APPDATA%` (confirmed by the settings page).
  - os: macos
    scope: repo
    path: "<project>/kilo.jsonc"
    format: jsonc
    notes: |
      Project-level config discovered by walking upward from the launch
      directory until reaching the worktree root. `kilo.json`, plus the
      legacy `opencode.json[c]` files, are also accepted.
  - os: linux
    scope: repo
    path: "<project>/kilo.jsonc"
    format: jsonc
    notes: |
      Same upward-walking discovery as macOS. `.kilocode/` and `.kilo/`
      config directories are also discovered by the same upward walk.
  - os: windows
    scope: repo
    path: "<project>\\kilo.jsonc"
    format: jsonc
    notes: |
      Same upward-walking discovery; project config wins over the user
      config when both define the same key.
  - os: macos
    scope: local
    path: "<project>/.kilo/"
    format: jsonc
    notes: |
      `.kilo/` config directory discovered alongside `.kilocode/`. The
      Marketplace installer writes MCP entries to `.kilo/kilo.json` for
      project scope. `.kilo/kilo.jsonc`, `.kilo/kilo.json`,
      `.kilo/opencode.jsonc`, and `.kilo/opencode.json` are all loaded.
  - os: linux
    scope: local
    path: "<project>/.kilo/"
    format: jsonc
    notes: "Same `.kilo/` discovery as macOS."
  - os: windows
    scope: local
    path: "<project>\\.kilo\\"
    format: jsonc
    notes: "Same `.kilo/` discovery as macOS/Linux."
  - os: macos
    scope: system
    path: "/Library/Application Support/kilo/kilo.jsonc"
    format: jsonc
    notes: |
      File-based managed config directory (path renamed from OpenCode's
      `/Library/Application Support/opencode/`). `kilo.json`,
      `kilo.jsonc`, `opencode.json`, and `opencode.jsonc` are all loaded.
      Not user-overridable.
  - os: linux
    scope: system
    path: "/etc/kilo/kilo.jsonc"
    format: jsonc
    notes: |
      Linux/WSL managed config directory (renamed from OpenCode's
      `/etc/opencode/`). Admin-only.
  - os: windows
    scope: system
    path: "C:\\ProgramData\\kilo\\kilo.jsonc"
    format: jsonc
    notes: |
      Windows managed config directory under `%ProgramData%\kilo\`
      (renamed from OpenCode's `%ProgramData%\opencode\`).
  - os: macos
    scope: managed
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    format: other
    notes: |
      macOS MDM-deployed preferences using the OpenCode plist domain
      (`ai.opencode.managed` — not renamed to kilo). Read with
      `plutil -convert json`. Highest precedence overall.
  - os: macos
    scope: other
    path: "KILO_CONFIG"
    format: jsonc
    notes: |
      Environment variable pointing to a custom config file path. Loaded
      between the global and project config sources. Applies to all OSes.
  - os: linux
    scope: other
    path: "KILO_CONFIG"
    format: jsonc
    notes: |
      Environment variable pointing to a custom config file path. Loaded
      between the global and project config sources. Applies to all OSes.
  - os: windows
    scope: other
    path: "KILO_CONFIG"
    format: jsonc
    notes: |
      Environment variable pointing to a custom config file path. Loaded
      between the global and project config sources. Applies to all OSes.
  - os: macos
    scope: other
    path: "KILO_CONFIG_CONTENT"
    format: jsonc
    notes: |
      Inline JSON config for one-run injection. The runtime tags the
      source as `local` and merges it above project configs but below
      managed sources. This is the runtime injection mechanism Claudine
      should use (analogous to OpenCode's `OPENCODE_CONFIG_CONTENT`).
      Applies to all OSes.
  - os: linux
    scope: other
    path: "KILO_CONFIG_CONTENT"
    format: jsonc
    notes: |
      Inline JSON config for one-run injection. The runtime tags the
      source as `local` and merges it above project configs but below
      managed sources. This is the runtime injection mechanism Claudine
      should use (analogous to OpenCode's `OPENCODE_CONFIG_CONTENT`).
      Applies to all OSes.
  - os: windows
    scope: other
    path: "KILO_CONFIG_CONTENT"
    format: jsonc
    notes: |
      Inline JSON config for one-run injection. The runtime tags the
      source as `local` and merges it above project configs but below
      managed sources. This is the runtime injection mechanism Claudine
      should use (analogous to OpenCode's `OPENCODE_CONFIG_CONTENT`).
      Applies to all OSes.
  - os: macos
    scope: other
    path: "KILO_CONFIG_DIR"
    format: jsonc
    notes: |
      Alternate config directory layered over the discovered `.kilo/`
      directories. Applies to all OSes.
  - os: linux
    scope: other
    path: "KILO_CONFIG_DIR"
    format: jsonc
    notes: |
      Alternate config directory layered over the discovered `.kilo/`
      directories. Applies to all OSes.
  - os: windows
    scope: other
    path: "KILO_CONFIG_DIR"
    format: jsonc
    notes: |
      Alternate config directory layered over the discovered `.kilo/`
      directories. Applies to all OSes.
  - os: macos
    scope: other
    path: "KILO_TEST_MANAGED_CONFIG_DIR / KILO_TEST_HOME"
    format: jsonc
    notes: |
      Test-only overrides for the managed-config dir and the data home.
      Not for production use. Applies to all OSes.
  - os: linux
    scope: other
    path: "KILO_TEST_MANAGED_CONFIG_DIR / KILO_TEST_HOME"
    format: jsonc
    notes: |
      Test-only overrides for the managed-config dir and the data home.
      Not for production use. Applies to all OSes.
  - os: windows
    scope: other
    path: "KILO_TEST_MANAGED_CONFIG_DIR / KILO_TEST_HOME"
    format: jsonc
    notes: |
      Test-only overrides for the managed-config dir and the data home.
      Not for production use. Applies to all OSes.
  - os: macos
    scope: remote
    path: "<provider>/.well-known/opencode"
    format: jsonc
    notes: |
      Well-known endpoint fetched when an auth credential has
      `type: "wellknown"`. Remote config is loaded first as the base
      layer; entries may ship `mcp` keys with `enabled: false` so the
      user opts in. Applies to all OSes.
  - os: linux
    scope: remote
    path: "<provider>/.well-known/opencode"
    format: jsonc
    notes: |
      Well-known endpoint fetched when an auth credential has
      `type: "wellknown"`. Remote config is loaded first as the base
      layer; entries may ship `mcp` keys with `enabled: false` so the
      user opts in. Applies to all OSes.
  - os: windows
    scope: remote
    path: "<provider>/.well-known/opencode"
    format: jsonc
    notes: |
      Well-known endpoint fetched when an auth credential has
      `type: "wellknown"`. Remote config is loaded first as the base
      layer; entries may ship `mcp` keys with `enabled: false` so the
      user opts in. Applies to all OSes.
  - os: macos
    scope: other
    path: "<Global.Path.data>/mcp-auth.json"
    format: jsonc
    notes: |
      OAuth credential store, separate from server definitions. Resolved
      via XDG: `~/.local/share/kilo/mcp-auth.json` on macOS. File mode
      `0o600`, flock-locked.
  - os: linux
    scope: other
    path: "<Global.Path.data>/mcp-auth.json"
    format: jsonc
    notes: |
      OAuth credential store, separate from server definitions. Resolved
      via XDG: `~/.local/share/kilo/mcp-auth.json` on Linux. File mode
      `0o600`, flock-locked.
  - os: windows
    scope: other
    path: "<Global.Path.data>/mcp-auth.json"
    format: jsonc
    notes: |
      OAuth credential store, separate from server definitions. Resolved
      via `%LOCALAPPDATA%\kilo\mcp-auth.json` on Windows. File mode
      `0o600`, flock-locked.
cli_params:
  - flag: "kilo mcp add"
    description: |
      Interactive wizard to add a local (stdio) or remote (HTTP/SSE) MCP
      server. Prompts for scope (Current project vs Global when the
      project is a git repo), name, type, command/URL, and OAuth
      configuration, then merges the entry into the chosen JSONC file via
      `jsonc-parser`. The command accepts no CLI flags; only the
      interactive path is supported.
    example: "kilo mcp add"
  - flag: "kilo mcp list"
    description: |
      List all configured MCP servers with per-server status: `connected`,
      `disabled`, `needs authentication`, `needs client registration`,
      `failed` (with error), or `not initialized`.
    example: "kilo mcp list"
  - flag: "kilo mcp ls"
    description: "Alias for `kilo mcp list`."
    example: "kilo mcp ls"
  - flag: "kilo mcp auth [name]"
    description: |
      Run the OAuth flow against a remote server. Without `name`, an
      interactive picker of OAuth-capable servers is shown. The default
      callback port is `19876` and the callback path is
      `/mcp/oauth/callback` (overridable via `oauth.callbackPort` /
      `oauth.redirectUri`); the CLI binds the port only after the
      authorization redirect is captured. Re-running for an
      already-authenticated server confirms before re-authenticating.
      Stores tokens in `<Global.Path.data>/mcp-auth.json` (mode `0o600`).
    example: "kilo mcp auth sentry"
  - flag: "kilo mcp auth list"
    description: "List OAuth-capable MCP servers and their auth status."
    example: "kilo mcp auth list"
  - flag: "kilo mcp logout [name]"
    description: |
      Remove stored OAuth credentials for one server. Without `name`, an
      interactive picker is shown. Drops the stored `tokens` and
      `clientInfo` for that name.
    example: "kilo mcp logout sentry"
  - flag: "kilo mcp debug <name>"
    description: |
      Diagnose a server's HTTP and OAuth health. Sends an explicit
      `initialize` request with `protocolVersion: "2024-11-05"`, reports
      the response status, prints the `WWW-Authenticate` header if
      present, and reports stored access/refresh/client secret state.
      Only works for remote servers.
    example: "kilo mcp debug sentry"
env_vars:
  - name: KILO_CONFIG
    effect: |
      Path to a custom config file. Loaded between the global and
      project sources in the precedence order.
  - name: KILO_CONFIG_CONTENT
    effect: |
      Inline JSON config applied for the current run. Tagged as a `local`
      source. The cleanest runtime injection path for Claudine wrappers.
  - name: KILO_CONFIG_DIR
    effect: |
      Path to an alternate config directory layered over the discovered
      `.kilo/` directories for agents/commands/plugins/skills.
  - name: KILO_TUI_CONFIG
    effect: "Path to a custom TUI config file."
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: |
      When set to a truthy value, Kilo skips loading `kilo.json[c]` and
      `.kilo/` from the project hierarchy. Useful for sandboxed or
      wrapper-driven runs that want the inline source to be authoritative.
  - name: KILO_PERMISSION
    effect: "Inlined JSON permissions merged over the loaded config."
  - name: KILO_DISABLE_DEFAULT_PLUGINS
    effect: "Disable the default plugin set Kilo ships with."
  - name: KILO_PURE
    effect: |
      Equivalent to the `--pure` CLI flag. Skips loading external
      plugins; does not disable MCP servers.
  - name: KILO_EXPERIMENTAL
    effect: |
      Umbrella flag that flips the unstable defaults on (LSP tool, plan
      mode, scout, parallel tool calls, etc.).
  - name: KILO_EXPERIMENTAL_LSP_TOOL
    effect: |
      Enables the experimental LSP tool surface (not MCP, but worth
      noting alongside `KILO_EXPERIMENTAL`).
  - name: KILO_CLIENT
    effect: |
      Identifies the calling client (defaults to `cli`); used in
      telemetry tags.
  - name: KILO_BWRAP_PATH
    effect: |
      Path to the bubblewrap binary used by the experimental sandbox on
      Linux. Does not change MCP transport behavior but matters for
      whether the agent itself runs sandboxed.
  - name: KILO_TEST_HOME / KILO_TEST_MANAGED_CONFIG_DIR
    effect: |
      Test-only overrides for the data home and the managed config dir;
      not intended for production use.
server_schema:
  transports: ["local", "remote"]
  command_fields: ["type", "command", "environment", "enabled", "timeout"]
  http_fields: ["type", "url", "headers", "oauth", "enabled", "timeout"]
  env_shape: |
    `environment` is an object mapping variable names to string values.
    The runtime merges these over `process.env` and inherits the rest
    (including `BUN_BE_BUN=1` when the command is `opencode`). Variable
    substitution follows the same `{env:NAME}` and `{file:path}` rules
    used elsewhere in `kilo.json[c]`. The active config field used to
    disable a server is `enabled: false` (the legacy form recognized by
    the schema union); `disabled: true` is only accepted through the
    unused v2 `ConfigMCP` Effect schema. Prefer `enabled: true|false`
    for both new and legacy compatibility.
  auth_shape: |
    Remote servers support OAuth 2.0 with PKCE. The `oauth` field accepts
    `false` to opt out of OAuth and force static `headers`, or an object
    with `clientId` (optional pre-registered id), `clientSecret` (optional
    secret), `scope` (RFC 6749 §3.3 space-separated string), `callbackPort`
    (1–65535; default `19876`), and `redirectUri` (full URL overrides the
    callback-port shorthand). Dynamic Client Registration is attempted
    automatically; the resulting client info is stored under
    `mcp-auth.json`'s `clientInfo` field. Token refresh is automatic
    when the access token expires; `isTokenExpired` is consulted before
    each call.
  notes: |
    Server id is the map key under `mcp`. The `type` field accepts
    `"local"` or `"remote"` — there is no Kilo-level `stdio`,
    `streamable-http`, or `sse` value. The SDK calls the older
    `SSEClientTransport` as a runtime fallback when `Streamable HTTP`
    fails (other than on auth error). The runtime patches `docker run` /
    `podman run` commands to inject `--rm` automatically so stopped MCP
    containers do not accumulate on the host.
server_capabilities:
  tools: full
  resources: partial
  prompts: partial
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: true
  prompt_list_changed: true
  notes: |
    Tools are auto-discovered via `tools/list` and presented to the
    model with permission keys namespaced `<sanitizedServer>_<sanitizedTool>`
    (e.g. `github_create_pull_request`) so they plug into the same
    `allow`/`ask`/`deny` system as built-in tools. Sanitization replaces
    any character outside `[A-Za-z0-9_-]` with `_`. Resources and prompts
    are listed by the SDK via `fetchFromClient`, but only a subset is
    wired into the UI: tools get the rich permission surface; prompts and
    resources get listed in the TUI toggle panel and are reachable
    through the SDK, but Kilo does not document a Kilo-native slash
    command or picker for them. Treat both surfaces as `partial`.
client_capabilities:
  roots: unknown
  sampling: none
  elicitation: unknown
  notes: |
    The MCP service exposes `prompts`, `resources`, `tools`, `add`,
    `connect`, `disconnect`, `getPrompt`, `readResource`, and auth
    helpers, but does not implement `roots/list`, `sampling/createMessage`,
    or elicitation from the upstream MCP servers' perspective. Roots are
    not surfaced; sampling is not advertised; elicitation is not
    documented.
tool_surface:
  discovery: |
    At session start, the MCP service iterates the merged `mcp` map,
    creates one `Client` per enabled server, calls `connect(transport)`
    with the per-server `timeout` (default `30_000` ms), and calls
    `tools/list`. When the client receives a `notifications/tools/list_changed`,
    the service refetches the tool list, updates the cached `defs`, and
    publishes a `mcp.tools.changed` bus event the agent and TUI react to.
    `tools/list` errors that stem from invalid `outputSchema` references
    fall back to a tolerant schema (omitting `outputSchema`).
  filtering: |
    Tool-level filtering happens through the permission system: rules
    like `<server>_<tool>` or `<server>_*` under `permission.<tool>` map
    to `allow`/`ask`/`deny`, so individual tools can be hidden from the
    model without removing the server. Server-level filtering happens
    through `enabled: false` in config, through the legacy `tools` map
    (e.g. `tools: {"my-mcp": false}` → `deny`), or via per-agent
    `agent.<name>.tools` / `agent.<name>.permission` entries.
  approval: |
    MCP tools obey the same `permission` engine as built-in tools. Each
    call is evaluated as `<server>_<tool>` against the matched rule
    (with `*`-globbing); unmatched requests default to `ask` for actions
    like `bash` and to `allow` for most others. The TUI's "Always run"
    options append new rules to the user's global config.
  result_handling: |
    Results are returned to the model via `CallToolResultSchema` with
    `resetTimeoutOnProgress: true` (progress notifications keep the
    idle timer happy). The runtime clamps the `inputSchema` to
    `additionalProperties: false` and forces `type: "object"` before
    surfacing tools to the AI SDK. Errors during `tools/list` that stem
    from invalid `outputSchema` references fall back to a tolerant
    schema; other errors are surfaced per server in `kilo mcp list`.
    There is no documented per-call output-token cap or persistence-to-
    disk step; large outputs flow back to the model verbatim.
  annotations_trusted: |
    The runtime forwards tool definitions without applying tool-side
    annotations beyond the input/JSON-schema layer. Kilo-specific
    `anthropic/*` annotations such as `requiresUserInteraction` are not
    advertised as policy inputs; only the namespaced permission rule
    controls visibility. A Kilo-specific sandbox wrapper
    (`SandboxNetwork.remote`) is applied to remote MCP tools when the
    experimental network-restricted sandbox is enabled.
  notes: |
    There is no documented per-argument approval policy for MCP tools.
    Approval prompts are the standard "Run" / "Deny" pair; the runtime
    auto-approve toggle in the TUI affects both native and MCP tools in
    lockstep.
resource_surface:
  supported: true
  uri_schemes: ["depends on server"]
  templates: true
  subscriptions: false
  exposure_model: |
    Resources are listed by the SDK (`fetchFromClient` calls
    `client.listResources()`) and stored under
    `<sanitizedClient>:<sanitizedName>` keys, so the underlying transport
    decides which URI schemes appear. Kilo does not document a
    user-visible picker for resources; whether they appear in the chat
    UI depends on the surrounding IDE/extension (VS Code/JetBrains) and
    on prompts that ask the model to call `readResource`. There is no
    documented `resources/subscribe` story and no subscriptions URI list.
prompt_surface:
  supported: true
  invocation: |
    Prompts are listed by the SDK and addressed by `<server>:<prompt>`.
    The VS Code UI surfaces MCP prompts (e.g. through McpEditView and
    the per-prompt `getPrompt` helper), but Kilo's docs do not document a
    Kilo-native slash command for prompts. Treat slash-command/palette
    exposure as `unknown` until proved otherwise.
  arguments: |
    `mcp.getPrompt(clientName, name, args?)` accepts an `args` map; UI
    argument collection depends on the front-end (the JetBrains
    `McpEditDialog` and VS Code `McpEditView` implement parameter forms).
  exposure_model: |
    Prompts are reachable through the SDK but there is no documented
    Kilo-native slash command or palette action that injects an MCP
    prompt directly. Treat them as `partial`: discovered and invokable,
    not surfaced as first-class slash commands.
  notes: |
    `fetchFromClient` is shared with resources, so capability drift
    (e.g. a dropped `prompts/list_changed` notification) is observable
    through the same client connection.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: shallow
  notes: |
    Claudine can read both `kilo.json[c]` / `.kilo/kilo.json[c]` and the
    legacy `opencode.json[c]` paths and normalize server entries back
    to the runtime shape (`enabled` field, OAuth in camelCase). Merge
    at load time uses Kilo's `mergeConfigConcatArrays` (arrays
    concatenated; objects deep-merged; scalars overwritten by later
    sources). There is no documented non-interactive apply path —
    `kilo mcp add` only runs interactively and merges its result into
    the selected config file via `jsonc-parser` while preserving
    comments. There is also **no `kilo mcp remove` command** — to
    remove a server, edit the config file directly (or write a
    `jsonc-parser` edit that drops the `mcp.<name>` entry). Apply via
    `kilo mcp add` is therefore unsafe for non-TTY runs.
runtime_injection:
  supported: true
  mechanism: |
    Set `KILO_CONFIG_CONTENT` to an inline JSON document containing the
    `mcp` map before launching `kilo`, `kilo run`, or any other
    subcommand. The runtime treats the inline source as `local`
    precedence (above project configs, below managed sources and
    MDM). The companion `KILO_CONFIG` / `KILO_CONFIG_DIR` vars let
    Claudine point Kilo at a temp file or directory written from the
    MCP catalog instead.
  limitations: |
    `KILO_CONFIG_CONTENT` does NOT preserve the user's persistent merge
    semantics by itself — it is concatenated with remote + managed
    sources, but if it shares keys with the user's project config, the
    last-loaded source (managed > inline > project > custom > global >
    remote) wins. Claudine should build the full effective server list
    itself for `-p` / `run` use, set `KILO_DISABLE_PROJECT_CONFIG` if
    user config is to be omitted, and rely on `KILO_CONFIG_CONTENT` for
    the body. OAuth flows cannot complete in non-interactive `kilo
    run` mode; pre-authenticated servers or static `headers` are
    required for autonomous runs.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens and Dynamic Client Registration state are written to
    `<Global.Path.data>/mcp-auth.json`, resolved via XDG
    (`~/.local/share/kilo/mcp-auth.json` on Linux/macOS and
    `%LOCALAPPDATA%\kilo\mcp-auth.json` on Windows; `KILO_TEST_HOME`
    overrides in tests). The file is flock-locked and written at file
    mode `0o600`. There is no use of an OS keychain for MCP OAuth
    credentials. Each entry is keyed by the user-chosen server name and
    carries an optional `serverUrl` so credentials can be tied to one
    remote URL.
  token_scope: |
    One entry per remote server URL (`serverUrl` field in the entry),
    keyed by config-side server name. Refresh tokens are stored when
    the server returns them; `isTokenExpired` is consulted before each
    call so re-auth can be triggered when tokens lapse. Client secret
    expiry is also tracked; an expired secret triggers re-registration.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `environment` object
    or via `{env:VAR}` / `{file:path}` substitution in `kilo.json[c]`.
    Process environment is otherwise inherited, including any platform-
    specific credentials the user has on PATH. The experimental sandbox
    explicitly does NOT isolate local MCP servers from network access
    (the network-restriction covers model-originated commands and
    first-party HTTP tools only).
  notes: |
    `oauth: false` opts a remote server out of the OAuth auto-detect
    path and forces header-based auth. For pre-registered clients,
    supply `clientId` (and optionally `clientSecret`) under `oauth`.
    Re-running `kilo mcp auth` with already-valid credentials confirms
    before re-authenticating. There is no per-run `--client-secret`
    flag analogue; secrets must already exist in env or be supplied via
    the OAuth flow. CSRF protection uses a per-server `oauthState` and
    a mismatched state raises a typed error.
security:
  tool_filtering: |
    Three complementary layers: (1) config-side `enabled: false` (or
    legacy `{enabled: false}`) on a single server; (2) the `tools` /
    `permission` map with `<server>_*` glob entries that yield
    `allow`/`ask`/`deny` for every tool of a server; (3) per-agent
    `agent.<name>.tools` / `agent.<name>.permission` overrides. The
    Marketplace install dialog shows the source author and requested
    parameters, and `kilo uninstall --keep-config` leaves the user's
    choices in place.
  server_trust: |
    Project-level `kilo.json[c]` / `.kilo/kilo.json[c]` is auto-loaded;
    there is no documented trust gate like Claude Code's per-workspace
    approval. Marketplace installs surface an install-dialog preview
    before writing, and removed items can be re-removed without deleting
    the rest of the config. macOS/Linux/Windows managed-config
    directories are admin-only and take the highest file-based
    precedence.
  env_sanitization: |
    Each stdio MCP server receives only the entries in its `environment`
    object plus inherited `process.env`. There is no documented
    subprocess env scrub analogous to Claude Code's
    `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`; anything in the user's
    environment that is not overridden is reachable by a local MCP
    command. The experimental sandbox restricts network from the
    model's bash / web tools, but explicitly excludes local MCP
    servers.
  sandbox_interaction: |
    The experimental sandbox (macOS `sandbox-exec` / Linux `bwrap`,
    Windows is not supported) applies to agent shell commands and
    file-write tools. It is documented to NOT cover local MCP servers
    and plugin hooks. The `experimental.sandbox` and
    `experimental.sandbox_restrict_network` settings affect only
    model-originated commands and first-party HTTP tools; MCP stdio
    traffic remains outside the bubblewrap/seatbelt confinement.
    Kilo-specific `SandboxNetwork.remote()` wraps remote MCP tools so
    that *remote* MCP traffic does flow through the network-restricted
    sandbox when it is enabled — but local stdio servers do not.
  response_filtering: |
    No native MCP response sanitization is documented. Tool outputs go
    straight to the model after `tools/call`. Claudine's `protect` layer
    should treat MCP tool results as untrusted and scan them for
    injection-style patterns, as it does for other agentic CLIs.
  notes: |
    OAuth tokens live in `mcp-auth.json` at `0o600`, not in an OS
    keychain. The .well-known remote source can ship default `mcp`
    entries with `enabled: false`; admin and managed directories add
    hard allowlist/denylist capability.
gaps:
  - |
    The docs do not name an explicit MCP protocol version date beyond
    the SSE-deprecation callout (`2025-03-26`). The hard-coded
    `protocolVersion: "2024-11-05"` in `kilo mcp debug` is the
    strongest evidence available.
  - |
    `roots/list`, `sampling/createMessage`, and elicitation are not
    surfaced and are not documented as client capabilities.
  - |
    Resources and prompts have no documented Kilo-native slash command
    or picker; treat them as `partial` rather than `full`.
  - |
    `resources/subscribe` is not advertised; there is no documented
    push-update story for MCP resources.
  - |
    There is no `kilo mcp remove` subcommand — removing a server
    requires editing the config file directly. Verify by running
    `kilo mcp --help` (CLI 7.3.45).
  - |
    The v2 `ConfigMCP` Effect schema in `packages/core/src/config/mcp.ts`
    declares `disabled: boolean` plus snake_case OAuth keys
    (`client_id`, `client_secret`, `callback_port`, `redirect_uri`).
    The active runtime in `packages/opencode/src/mcp/index.ts` reads
    `enabled: boolean` and camelCase OAuth keys (`clientId`,
    `clientSecret`, `callbackPort`, `redirectUri`); `kilo mcp add`
    writes camelCase. Treat the camelCase form as the live contract
    and the v2 schema as a parallel spec that is not yet active.
  - |
    Managed `mcp` schema for organization-wide allowlists is proposed
    but not shipped (see "Enterprise MCP Controls" contribution doc).
  - |
    Sandbox does NOT cover local MCP servers — confirmed in source
    (`experimental.sandbox_restrict_network` description and the
    `SandboxNetwork` wrapper) and the public docs.
  - |
    Kilo does not currently advertise an OpenCode-style TS extension
    for MCP servers; MCP marketplace entries are static definitions plus
    `{env:VAR}` substitution only.
  - |
    The `tools/list` filter pattern (excluding tools by annotation) is
    not documented; only server-level disabling and namespaced
    permission rules are.
  - |
    The OAuth callback port default (`19876`) and callback path
    (`/mcp/oauth/callback`) are documented only in source code.
changes:
  - "CLI version is now `7.3.45` (was previously characterized as 1.0); the kilocode monorepo `releases` tab shows 443 releases with the latest `v7.4.1` on 2026-07-03."
  - "MCP documentation URLs moved from `https://kilocode.ai/docs/features/mcp/...` to `https://kilocode.ai/docs/automate/mcp/...`; the using-mcp page (`using-in-kilo-code`), server-transports page, and overview page are now under `/automate/mcp/`."
  - "No `kilo mcp remove` subcommand exists in `kilo mcp --help`; the only MCP subcommands are `add`, `list` (alias `ls`), `auth` (+ `auth list`), `logout`, and `debug`. Server removal must be done by editing the config file directly."
  - "Active config disable field is `enabled: false`; the legacy `{enabled: false}` form is recognized alongside the v2 `ConfigMCP` Effect schema's `disabled` field, but `kilo mcp add` writes `enabled: true`."
  - "OAuth default callback port is `19876` (was previously characterized as `4096`); callback path is `/mcp/oauth/callback`. Both are overridable via `oauth.callbackPort` and `oauth.redirectUri`."
  - "Default MCP `timeout` is `30_000` ms (was previously characterized as 5s/15s); confirmed in `packages/opencode/src/mcp/index.ts` (`DEFAULT_TIMEOUT = 30_000`). Per-server `timeout` overrides via the JSON config."
  - "Hard-coded `protocolVersion: \"2024-11-05\"` in the `initialize` message sent by `kilo mcp debug <name>`; SSE deprecated by MCP spec on `2025-03-26`."
  - "Managed config directory is `/Library/Application Support/kilo` (macOS), `/etc/kilo` (Linux), and `%ProgramData%\\kilo` (Windows) — renamed from OpenCode's `opencode` paths. MDM plist domain is still `ai.opencode.managed`."
  - "Windows user config lives under `%USERPROFILE%\\.config\\kilo\\kilo.jsonc`, not `%APPDATA%` as previously characterized."
  - "Remote MCP tools are wrapped with `SandboxNetwork.remote()` when the experimental network-restricted sandbox is enabled (Kilo-specific). Local MCP stdio servers remain outside the sandbox; confirmed by both source code and the public docs."
  - "OAuth state CSRF mismatch raises a typed `OAuth state mismatch - potential CSRF attack` error and aborts the flow."
  - "OAuth `clientSecret` expiry is tracked and triggers re-registration; `getForUrl` validates that stored credentials match the current server URL."
  - "OAuth credential store is flock-locked with `EffectFlock.withLock(lockKey)`; file mode `0o600`; per-server `oauthState` and `codeVerifier` are persisted alongside `tokens` and `clientInfo`."
  - "Marketplace MCP install destinations are `.kilo/kilo.json` (project scope) and `~/.config/kilo/kilo.json` (user scope); the install dialog previews the destination."
  - "Project config discovery walks upward from the launch directory looking for `.kilocode/` or `.kilo/` config directories and `kilo.json[c]` / `opencode.json[c]` files. `.kilocode/` is recognized as a legacy directory in addition to `.kilo/`."
  - "`KILO_PURE` / `--pure` flag exists but does not disable MCP servers; it only skips loading external plugins."
  - "Verified locally: `~/.config/kilo/kilo.jsonc` exists with only `$schema: https://app.kilo.ai/config.json`, no `mcp` key configured. `kilo mcp list` reports `No MCP servers configured`. kilo CLI version `7.3.45` is installed at `/Users/ken/.nvm/versions/node/v22.20.0/bin/kilo`."
requires_claudine_update: true
reason: |
  Three behaviors are now provable rather than guessed: (1) there is no
  `kilo mcp remove` subcommand — Claudine's `mcp sync` and apply layers
  must edit the config file via `jsonc-parser` (matching what
  `kilo mcp add` does) rather than expect an interactive apply path;
  (2) the active disable field is `enabled: false` (not `disabled`), and
  the OAuth shape uses camelCase (`clientId`/`clientSecret`/
  `callbackPort`/`redirectUri`), so the catalog normalizer should map
  to that form; (3) the OAuth default callback port is `19876` (not
  `4096`), so the runtime injector must reserve `127.0.0.1:19876`
  (overridable via `oauth.callbackPort`) instead of `4096`. Remote
  MCP tools flow through `SandboxNetwork.remote()` when the experimental
  network-restricted sandbox is on, so the wrapper can rely on the
  sandbox to gate remote MCP egress but must not rely on it for local
  MCP. The CLI has moved from the v1.x family (where the prior research
  was anchored) to v7.3.45 and the MCP docs have moved from
  `/docs/features/mcp/` to `/docs/automate/mcp/`; both should be
  reflected in the provider metadata and skill docs.

---

# MCP Support in Kilo Code

## Overview

Kilo Code supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io) as a first-class extension point for its VS Code, JetBrains, and CLI surfaces. The CLI binary `kilo` (currently v7.3.45, branched from [OpenCode](https://github.com/anomalyco/opencode) and shipped from the [Kilo-Org/kilocode](https://github.com/Kilo-Org/kilocode) monorepo) shares a single core with the IDE extensions; the docs, settings page, and CLI reference all read and write the same JSONC config files. The CLI exposes `kilo mcp add` (interactive), `kilo mcp list`, `kilo mcp auth` / `logout` / `debug` — there is no `kilo mcp remove` subcommand. Tools are auto-discovered via `tools/list` and presented to the model under namespaced `<server>_<tool>` permission keys; the existing `permission` engine treats them as first-class tools. A [Kilo Marketplace](https://github.com/Kilo-Org/kilo-marketplace) curates shared Skills, MCP servers, and Agents that install at project or global scope.

For Claudine, the strongest integration path is **runtime injection** via `KILO_CONFIG_CONTENT`, paired with direct JSONC edits (with `jsonc-parser`, matching what `kilo mcp add` does) for persistent setup. Importing the user's `~/.config/kilo/kilo.json[c]` and the merged effective config remains feasible; applying changes through the CLI is not safe because `kilo mcp add` is interactive-only and there is no `remove`.

Surface inventory (one-line):

- **Tools** — exposed: `<server>_<tool>` callable names, with `tools/list_changed` refresh and the same `allow`/`ask`/`deny` engine as native tools.
- **Resources** — exposed partially: listed by the SDK; no `resources/subscribe` push and no documented Kilo-native picker.
- **Prompts** — exposed partially: listed by the SDK; no documented Kilo-native slash command or palette action.
- **Roots / Sampling / Elicitation** — not advertised by the Kilo client.
- **Plugin hooks / Marketplace** — separate from MCP servers; Marketplace MCP entries install into `kilo.json[c]` via the same `jsonc-parser` edit path.

## Protocol and Transports

Kilo documents two user-visible transport categories — **Local** and **Remote** — but the underlying runtime uses the upstream `@modelcontextprotocol/sdk` transports:

| User-facing `type` | Runtime transport(s) | How it is added |
| :----------------- | :------------------- | :-------------- |
| `"local"` | `StdioClientTransport` | `kilo mcp add` (interactive) |
| `"remote"` | `StreamableHTTPClientTransport`, falling back to `SSEClientTransport` | `kilo mcp add` (interactive) |

Lifecycle behavior:

- **Local (stdio)** servers are spawned as child processes at session start with the launch CWD as `cwd` and `{ ...process.env, ...mcp.environment }` as `env`. Failures surface as `failed` status in `kilo mcp list` and are not auto-retried mid-session. Stderr from the child is logged via the MCP service (`mcp stderr: ...`) but is not streamed to the user.
- **Remote** servers try Streamable HTTP first; on connection error (other than auth) they fall back to legacy SSE within the configured `timeout` (default `30_000` ms). Auth errors break the loop and surface a `needs_auth` or `needs_client_registration` status that points the user at `kilo mcp auth <name>`.
- **Dynamic capability updates**: servers may send `tools/list_changed`; the MCP service refetches the tool list, updates the cached `defs`, and publishes a `mcp.tools.changed` bus event so the TUI and agent refresh without a session restart.
- **Protocol version**: `kilo mcp debug <name>` sends an explicit `initialize` with `protocolVersion: "2024-11-05"`, the version the Kilo client advertises. The upstream MCP specification deprecated SSE on `2025-03-26`.

## Configuration

Kilo's MCP servers live in the same `kilo.json[c]` config file as the rest of the configuration. The `mcp` key is a map of server name to server definition.

### File layout

| Scope | File | Notes |
| :---- | :--- | :---- |
| User | `~/.config/kilo/kilo.jsonc` | XDG config home on Linux/macOS, `%USERPROFILE%\.config\` on Windows. Legacy `opencode.json[c]` and `config.json` are also read. |
| Project (root) | `<project>/kilo.json[c]` | Discovered by walking upward from the launch directory until the worktree root. Project overrides user. |
| Project (in-dir) | `<project>/.kilo/kilo.json[c]` (preferred) or `<project>/.kilocode/kilo.json[c]` (legacy) | Same precedence; Marketplace installs write to `.kilo/kilo.json` for project scope. |
| Plugin | `<plugin-root>/.kilocode/mcp.json` (or any MCP entry installed by Marketplace into the chosen config) | Bundled by some marketplace entries; project or global scope selected at install. |
| Managed | `/Library/Application Support/kilo/kilo.jsonc` (macOS), `/etc/kilo/kilo.jsonc` (Linux), `C:\ProgramData\kilo\kilo.jsonc` (Windows); plus the `ai.opencode.managed` MDM plist (macOS) | Admin-controlled, highest file-based precedence. |
| Remote | `<provider>/.well-known/opencode` | Fetched when an auth credential has `type: "wellknown"`; loaded first as the base layer; may ship `mcp` entries with `enabled: false` to opt out. |

### Precedence order

Files are merged, not replaced (arrays concatenated; objects deep-merged; scalars overwritten by later sources). The combined precedence from lowest to highest:

1. Remote well-known config (`<provider>/.well-known/opencode`)
2. Global config (`~/.config/kilo/...`)
3. Custom config (`KILO_CONFIG` env var)
4. Project config (`<project>/kilo.json[c]` / `.kilo/kilo.json[c]`)
5. `.kilo` / `.kilocode` directories (agents, commands, plugins, skills)
6. Inline config (`KILO_CONFIG_CONTENT`) — tagged `local`, above project configs
7. Active-org config (Kilo Cloud) — when authenticated
8. File-based managed config directory (`/Library/Application Support/kilo/`, `/etc/kilo/`, `%ProgramData%\kilo\`)
9. macOS managed preferences (`.mobileconfig` via MDM)

`KILO_DISABLE_PROJECT_CONFIG=true` skips tiers 4 and 5, which is useful when Claudine wants to ship a clean effective config via inline injection.

## Server Definition Shape

A server definition under `mcp.<name>` looks like one of:

```jsonc
{
  "mcp": {
    "my-local-server": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem"],
      "environment": { "API_KEY": "..." },
      "enabled": true,
      "timeout": 30000
    },
    "my-remote-server": {
      "type": "remote",
      "url": "https://example.com/mcp",
      "headers": { "Authorization": "Bearer {env:MY_API_KEY}" },
      "oauth": false,
      "enabled": true,
      "timeout": 30000
    }
  }
}
```

### Field reference

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `type` | both | `"local"` or `"remote"` |
| `command` | local | Array of strings forming the executable and arguments |
| `environment` | local | Object mapping variable names to string values |
| `url` | remote | Endpoint URL |
| `headers` | remote | Static header map |
| `oauth` | remote | OAuth object (camelCase), or `false` to opt out and force header-based auth |
| `enabled` | both | Set to `false` to suppress the server without removing the entry (legacy form `{enabled: false}` is also accepted) |
| `timeout` | both | Per-server connection + tool-call timeout in ms (default `30_000`) |

### Environment and variable substitution

- `{env:VARIABLE_NAME}` expands a process env var.
- `{file:path/to/file}` reads a file's contents; paths can be absolute (`/`, `~`) or relative to the config directory.

If a substitution target is unset and has no default, the value is replaced with the empty string for env vars and the file is read at load.

### OAuth shape (camelCase — runtime contract)

```jsonc
{
  "oauth": {
    "clientId": "{env:MY_MCP_CLIENT_ID}",
    "clientSecret": "{env:MY_MCP_CLIENT_SECRET}",
    "scope": "tools:read tools:execute",
    "callbackPort": 19876,
    "redirectUri": "http://127.0.0.1:19876/mcp/oauth/callback"
  }
}
```

If `oauth` is omitted, Dynamic Client Registration is attempted; if the server rejects it, the MCP service marks the server as `needs_client_registration` and prints the JSON to add `clientId`/`clientSecret` to the config. The default callback port is `19876` and the default callback path is `/mcp/oauth/callback`. `kilo mcp auth` binds the port only after the authorization redirect is captured, so the port is free for the user to repurpose until OAuth is actually required.

The v2 `ConfigMCP` Effect schema in `packages/core/src/config/mcp.ts` declares OAuth with snake_case keys (`client_id`, `client_secret`, `callback_port`, `redirect_uri`) and uses a `disabled: boolean` field — these are not the live contract. The runtime reads camelCase and `enabled`; `kilo mcp add` writes camelCase and `enabled`. Treat the v2 schema as a parallel specification that is not yet active.

## Tools, Resources, and Prompts

### Tools

All MCP tools are auto-discovered and presented to the model under namespaced names. The Kilo CLI / JetBrains / VS Code surfaces use the convention `{server}_{tool}` in permission rules (sanitized so non-`[A-Za-z0-9_-]` characters become `_`), e.g.:

```jsonc
{
  "permission": {
    "github_create_pull_request": "allow",
    "github_*": "ask"
  }
}
```

Discovery:

- `tools/list` is called once on connect with the per-server `timeout` (default `30_000` ms).
- A `notifications/tools/list_changed` handler refetches the list, updates the cached `defs`, and emits `mcp.tools.changed`.
- Errors during `tools/list` that stem from invalid `outputSchema` references fall back to a tolerant schema (omitting `outputSchema`).

Approval:

- Tools respect the same `allow` / `ask` / `deny` engine as built-in tools. The TUI's runtime auto-approve toggle covers MCP calls in lockstep.
- The Marketplace MCP install dialog displays the source author, requested parameters, and any platform-specific requirements before writing to the chosen config.

Result handling:

- `tools/call` responses are validated against `CallToolResultSchema` with `resetTimeoutOnProgress: true`, so progress notifications keep the idle timer happy.
- The runtime clamps `inputSchema` to `additionalProperties: false` and forces `type: "object"` before surfacing tools to the AI SDK.
- There is no documented per-call output-token cap or persistence-to-disk step; large outputs flow back to the model verbatim.

### Resources and prompts

The MCP source code lists both `resources` and `prompts` via `fetchFromClient`, so they are reachable from any Kilo front end that asks the model to call `readResource` / `getPrompt`. The Marketplace install path can also surface them in the VS Code `McpEditView` / JetBrains `McpEditDialog`. Kilo's primary docs do not describe a Kilo-native slash command or palette action that lists MCP prompts as first-class commands, so Claudine should treat prompts and resources as **partial** until a future doc pass promotes them.

## Roots, Sampling, and Elicitation

Kilo's MCP service does not implement the client-side capabilities of `roots/list`, `sampling/createMessage`, or elicitation from the perspective of an MCP server. There is no:

- public way for an MCP server to ask Kilo "what are your filesystem boundaries";
- documented support for an MCP server asking the model to make a nested LLM call; or
- elicitation path for an MCP server to collect structured user input through Kilo's UI.

Treat all three as `unknown` until proven otherwise; in practice this means an MCP server running against Kilo must either complete its work using only tools the server itself exposes, or rely on the host application (VS Code / JetBrains) for filesystem context.

## Import, Export, and Sync

Claudine can treat Kilo Code as a `runtime_injection` candidate (with `import_sync` surface available for catalog reading), with caveats:

- **Import**: read the merged `kilo.json[c]` config(s) — including the XDG-resolved paths, `KILO_CONFIG`, `KILO_CONFIG_CONTENT`, and managed directories — and normalize the `mcp` key into Claudine's MCP catalog. Also consume the legacy `opencode.json[c]` paths Kilo still reads.
- **Export**: write provider-shaped JSON back to those files (`.kilo/kilo.json[c]` for project scope, `~/.config/kilo/kilo.jsonc` for user scope). Claudine should preserve comments by writing `.jsonc` only when the source was `.jsonc`, and use `jsonc-parser` (which `kilo mcp add` already does) for surgical edits.
- **Apply**: there is **no documented non-interactive apply path**. `kilo mcp add` is interactive only; it does not accept flags. There is also **no `kilo mcp remove` subcommand** — removing a server requires editing the config file directly. Claudine should therefore:
  - write to the config file directly with `jsonc-parser`, atomic-rename style; then
  - call `kilo mcp auth` only when OAuth setup is required (and only when a TTY is available).

Merge semantics:

- Arrays are concatenated across scopes (e.g. `plugin`, `instructions`).
- Objects deep-merge with later sources winning conflicts.
- Scalar values are overwritten by the higher-precedence source.
- `enabled: false` (or `{enabled: false}`) wins for individual server entries regardless of where they appear.

## Runtime Injection

For one-run injection without mutating persistent config, Kilo accepts the same trio of env vars as OpenCode:

| Var | Effect |
| :-- | :----- |
| `KILO_CONFIG` | Load a custom config file at a custom path; sits between global and project configs in the precedence order. |
| `KILO_CONFIG_CONTENT` | Apply an inline JSON document for this run only; tagged as a `local` source. |
| `KILO_CONFIG_DIR` | Layer an alternate config directory (agents/commands/plugins/skills) over `.kilo`. |
| `KILO_DISABLE_PROJECT_CONFIG=true` | Skip project-level configs entirely — useful when the inline source is authoritative. |

Example one-shot run:

```bash
KILO_CONFIG_CONTENT='{"mcp":{"fs":{"type":"local","command":["npx","-y","@modelcontextprotocol/server-filesystem","."]}}}' \
  kilo run --auto "Summarize the repo"
```

Limitations:

- `KILO_CONFIG_CONTENT` does not preserve the normal user/project merge semantics — it is concatenated with remote + managed sources, but later sources override scalar keys.
- OAuth flows cannot complete in non-interactive `kilo run` mode; pre-authenticated servers or static `headers` are required for `run`.
- The runtime cannot tell the difference between `KILO_CONFIG_CONTENT` and a project config of the same keys at the same precedence tier; if you want strict control, also set `KILO_DISABLE_PROJECT_CONFIG=1`.
- There is no `--bare` analogue for Kilo; the closest is `KILO_DISABLE_PROJECT_CONFIG=1` + no explicit project file.

## Authorization and Credentials

OAuth 2.0 with PKCE is the default for remote MCP servers that advertise it. Flow:

1. The MCP service detects the 401 response.
2. Dynamic Client Registration (RFC 7591) is attempted unless the config supplies `clientId`/`clientSecret`.
3. The Kilo CLI opens the browser via the `open` package at the provider's authorization URL.
4. The MCP OAuth callback server (embedded in the CLI) catches the redirect at `http://127.0.0.1:19876/mcp/oauth/callback` (or the `oauth.callbackPort` / `oauth.redirectUri` override).
5. Tokens + refresh token + DCR state are written to `<Global.Path.data>/mcp-auth.json` at file mode `0o600`, flock-locked.

Per-server UX:

- `kilo mcp auth [name]` — run the OAuth flow for one server.
- `kilo mcp auth list` — list OAuth-capable servers and their status (`authenticated` / `expired` / `not authenticated`).
- `kilo mcp logout [name]` — remove stored credentials.
- `kilo mcp debug <name>` — diagnose auth/HTTP issues. Sends `initialize` with `protocolVersion: "2024-11-05"`, prints the response status and `WWW-Authenticate` header if present, and reports stored access/refresh/client secret state.

For stdio servers, secrets should be supplied via the per-server `environment` map (or via `{env:VAR}` substitution in `command` / `headers`); process env is otherwise inherited. There is no documented env scrub for MCP subprocesses — anything in the user's shell environment is reachable by a local MCP command unless the user cleanses it manually.

## Security Model

### Trust and allowlisting

- Project config is auto-loaded without a per-workspace trust gate. Marketplace installs surface an install-dialog preview before writing.
- Managed-config directories (`/Library/Application Support/kilo/`, `/etc/kilo/`, `%ProgramData%\kilo\`) are admin-only and take the highest file-based precedence.
- macOS MDM preferences (`ai.opencode.managed` plist, not renamed to kilo) are the highest-precedence configuration source overall.
- Per-tool policy lives in the `permission` engine: `<server>_<tool>` rules or `<server>_*` globs are honored everywhere MCP surfaces touch the model.
- Remote `well-known/opencode` entries may ship with `enabled: false` and require the user to opt in by name.

### Environment and sandboxing

- Each stdio MCP server receives only its `environment` map plus inherited `process.env`. There is no documented scrub equivalent to `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`.
- The experimental sandbox (`experimental.sandbox` / `experimental.sandbox_restrict_network`) applies to model-originated shell commands and first-party HTTP tools (webfetch/websearch). It is explicitly documented to **NOT** isolate local MCP servers or plugin hooks. On Linux this is `bwrap`, on macOS `sandbox-exec`; Windows has no backend.
- A Kilo-specific `SandboxNetwork.remote()` wrapper is applied to remote MCP tools, so **remote** MCP traffic does flow through the network-restricted sandbox when enabled — but local stdio servers do not.
- A stdio `command` of `docker run ...` (or `podman run ...`) is patched to inject `--rm` automatically (`ensureDockerRm`) so that stopped MCP containers do not accumulate on the host.

### Response handling

- No native MCP result sanitization is documented.
- There is no documented output-size persistence/turndown step for MCP tool responses; large outputs are passed back to the model verbatim.
- Claudine's `protect` layer should treat MCP tool results as untrusted and scan them for prompt-injection patterns.

### Credential storage

- OAuth credentials live in `mcp-auth.json` on disk, not in the OS keychain. The file is `0o600`, flock-locked, on a XDG-aware path (`~/.local/share/kilo/mcp-auth.json` on Linux/macOS, `%LOCALAPPDATA%\kilo\mcp-auth.json` on Windows).
- Client secrets supplied in `oauth.clientSecret` are read at flow time; they are not written back to `kilo.json[c]` verbatim if you substitute them via `{env:VAR}`.

## Mode-Specific Behavior

### Interactive TUI / VS Code / JetBrains

- `kilo mcp add` runs an interactive wizard that prompts for scope (project vs global), name, transport, command/URL, optional OAuth details, and so on.
- The MCP toggle panel in the TUI lets the user enable/disable configured servers.
- The `McpEditView` (VS Code) and `McpEditDialog` (JetBrains) forms collect OAuth and parameter inputs and write through to the same JSONC files.
- Marketplace installs show a preview dialog, then write the entry into the chosen config.

### Non-interactive (`kilo run`, autonomous)

- `kilo run --auto` lets the agent proceed autonomously; OAuth flows cannot complete, so any `oauth: { ... }` server that needs interactive consent will report `needs_auth` and stay unavailable.
- `kilo run` (no `--auto`) defers permission prompts to the user; MCP tool prompts behave like any other permission-prompted action.
- Tool calls flow through the MCP service's `tools/call` path with `resetTimeoutOnProgress: true`; long-running servers keep their idle window open via progress notifications.

### Headless server (`kilo serve`, `kilo web`)

- The HTTP server exposes the same MCP-backed tool surface to remote TUI / browser clients.
- Authentication is governed by `KILO_SERVER_PASSWORD` / `KILO_SERVER_USERNAME` for basic auth.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Server fails to start (local) | Reported as `failed` in `kilo mcp list`; stderr is logged but not surfaced to the model |
| `tools/list` returns invalid `outputSchema` | Falls back to a tolerant schema (omits `outputSchema`) and retries once |
| `tools/list` returns transport error | Server is marked `failed`; `kilo mcp debug <name>` can probe further |
| Remote Streamable HTTP fails first | Falls back to SSE within the `timeout` window |
| Remote returns 401 | Dynamic Client Registration or stored-token refresh attempted; if both fail, the server is marked `needs_auth` |
| Remote requires pre-registered `clientId` | Server is marked `needs_client_registration` with a JSON snippet showing how to add `clientId` to config |
| OAuth state mismatch | Raises `OAuth state mismatch - potential CSRF attack` and aborts the flow |
| OAuth token expired | `kilo mcp auth <name>` reports "expired credentials" and re-authenticates |
| OAuth client secret expired | `clientInformation()` returns `undefined`; re-registration is triggered |
| Output too large | No documented turndown behavior; the full `tools/call` payload is returned to the model |
| Project config missing | Falls back to global + remote config; the MCP toggle panel shows whatever is enabled |

## Claudine Integration Notes

- Treat Kilo as `support: runtime_injection`. The strongest path is `KILO_CONFIG_CONTENT` for non-interactive use and direct `kilo.json[c]` edits via `jsonc-parser` (matching `kilo mcp add`'s own approach) for persistent setup.
- Map Claudine's normalized catalog to Kilo's `mcp` object shape: use the runtime contract (`enabled` boolean, camelCase OAuth keys, `type` of `local` / `remote`). The v2 `ConfigMCP` Effect schema's `disabled` field and snake_case OAuth keys are not the live contract — they are a parallel schema that is not yet wired to the runtime.
- For one-run wrappers, prefer `KILO_CONFIG_CONTENT` (with `KILO_DISABLE_PROJECT_CONFIG=1` for strict control) over mutating `.kilo/kilo.json[c]` — Claudine should construct the full effective server list itself.
- The OAuth credential store at `<Global.Path.data>/mcp-auth.json` is plain-text JSON at `0o600`, flock-locked. Claudine should avoid reading or writing this file and instead rely on `kilo mcp auth` for interactive OAuth setup.
- The default OAuth callback port is `19876` and the callback path is `/mcp/oauth/callback`. Reserve `127.0.0.1:19876` for the wrapper, or override via `oauth.callbackPort` / `oauth.redirectUri` in the injected config.
- The Marketplace install dialog is the closest analog to a guided "add server" UI; Claudine does not have direct access to it but can replicate its layout when presenting a server to the user.
- Treat MCP outputs as untrusted; the experimental sandbox does not isolate stdio MCP servers, and there is no native response sanitizer. Remote MCP traffic **does** flow through `SandboxNetwork.remote()` when the network-restricted sandbox is on, but local stdio traffic does not.
- Surfacing resources and prompts to the model is partially supported through the SDK but no Kilo-native UI is documented; expect to expose them via VS Code / JetBrains fronts rather than the CLI.
- Kilo CLI is a fork of OpenCode, so most Claudine code that handles OpenCode config (`OPENCODE_CONFIG`, `OPENCODE_CONFIG_CONTENT`) applies with `KILO_*` renames — but the `KILO_CONFIG_CONTENT` precedence is between the project and managed sources (not above managed), so a wrapper that wants to be authoritative should also set `KILO_DISABLE_PROJECT_CONFIG=1`.
- Because `kilo mcp add` is interactive-only and there is **no `kilo mcp remove`**, Claudine should not call either from non-TTY contexts; use `jsonc-parser` edits to `kilo.jsonc` directly and rely on the user running `kilo mcp auth` themselves when OAuth is needed.

## Sources

- [Kilo Code MCP overview](https://kilocode.ai/docs/automate/mcp/overview)
- [Using MCP in Kilo Code](https://kilocode.ai/docs/automate/mcp/using-in-kilo-code)
- [MCP server transports (STDIO/SSE)](https://kilocode.ai/docs/automate/mcp/server-transports)
- [What is MCP](https://kilocode.ai/docs/automate/mcp/what-is-mcp)
- [MCP vs API](https://kilocode.ai/docs/automate/mcp/mcp-vs-api)
- [Kilo Marketplace](https://kilocode.ai/docs/customize/marketplace)
- [Kilo CLI reference](https://kilocode.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo CLI installation](https://kilocode.ai/docs/code-with-ai/platforms/cli)
- [Kilo settings](https://kilocode.ai/docs/getting-started/settings)
- [Auto-approving actions](https://kilocode.ai/docs/getting-started/settings/auto-approving-actions)
- [Sandboxing (experimental)](https://kilocode.ai/docs/getting-started/settings/sandboxing)
- [What's new in Kilo Code](https://kilocode.ai/docs/code-with-ai/platforms/vscode/whats-new)
- [OpenCode MCP servers (parent project)](https://opencode.ai/docs/mcp-servers)
- [OpenCode config (parent project)](https://opencode.ai/docs/config)
- [OpenCode permissions (parent project)](https://opencode.ai/docs/permissions)
- [Kilo-Org/kilocode repository](https://github.com/Kilo-Org/kilocode)
- [Kilo Marketplace repository](https://github.com/Kilo-Org/kilo-marketplace)
- [`packages/opencode/src/mcp/index.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/mcp/index.ts) — runtime MCP service
- [`packages/opencode/src/mcp/auth.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/mcp/auth.ts) — OAuth token store (`mcp-auth.json` schema)
- [`packages/opencode/src/mcp/oauth-provider.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/mcp/oauth-provider.ts) — OAuth callback port (`19876`) and path (`/mcp/oauth/callback`)
- [`packages/opencode/src/cli/cmd/mcp.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/cli/cmd/mcp.ts) — `kilo mcp` CLI source (no `remove` subcommand)
- [`packages/opencode/src/config/config.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/config.ts) — config precedence and merging
- [`packages/opencode/src/config/paths.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/paths.ts) — project config discovery
- [`packages/opencode/src/config/managed.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/managed.ts) — managed-config paths and MDM plist domain
- [`packages/core/src/config/mcp.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/config/mcp.ts) — Effect v2 MCP schema (snake_case OAuth, `disabled` field — not yet live)
- [`packages/core/src/flag/flag.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/flag/flag.ts) — `KILO_*` env vars
- [Enterprise MCP Controls (proposal)](https://kilocode.ai/docs/contributing/features/enterprise-mcp-controls)
- Local observation: `kilo --version` ⇒ `7.3.45`; `kilo mcp list` reports `No MCP servers configured`; `kilo mcp --help` shows only `add` / `list` (`ls`) / `auth` (+ `auth list`) / `logout` / `debug` (no `remove`); `~/.config/kilo/kilo.jsonc` contains only `$schema: https://app.kilo.ai/config.json` with no `mcp` key.