---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://developers.openai.com/codex/mcp/
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http]
  lifecycle: |
    Stdio and streamable-HTTP servers are spawned or connected in parallel at
    session start (one JoinSet per server) and reported through
    `McpStartupUpdateEvent` / `McpStartupCompleteEvent`. Codex's `RmcpClient`
    retries transient `tools/list`, `resources/list`, and
    `resources/templates/list` errors with a short backoff schedule
    (`STREAMABLE_HTTP_RETRY_DELAYS_MS`) inside the per-operation timeout;
    authentication and 4xx failures are not retried. A streamable-HTTP session
    that returns 404 is treated as expired and the client transparently
    re-initializes (one concurrent recovery per server). The OAuth token
    refresh runs on a 30s skew before expiry, after every successful call, and
    on startup when tokens are loaded. Server `instructions` are read once
    during the MCP handshake and re-attached alongside tools; the docs
    recommend keeping the first 512 characters self-contained.
  notes: |
    Codex implements the protocol on top of `rmcp = "1.8.0"` and accepts two
    transports: `stdio` (local subprocess) and `streamable_http` (RMCP's
    Streamable HTTP). Legacy SSE and HTTP+SSE are not documented or accepted.
    There is also an in-process `TransportRecipe::InProcess` for embedding
    Codex itself; it is not an end-user MCP transport. No explicit MCP protocol
    version date is published, but the implementation supports elicitation
    (form + URL modes plus a vendor `openai/form` extension) and a vendor
    Apps extension (`_meta.ui.visibility` filtering, connector metadata in
    `_meta.connector_*`).
config_files:
  - os: macos
    scope: user
    path: "$CODEX_HOME/config.toml (default ~/.codex/config.toml)"
    format: toml
    notes: |
      User-scoped MCP servers live under `[mcp_servers.<id>]`. The CLI, the
      Codex IDE extension, and the app-server share this file. On Windows the
      default is `%USERPROFILE%\.codex\config.toml`.
  - os: linux
    scope: user
    path: "$CODEX_HOME/config.toml (default ~/.codex/config.toml)"
    format: toml
    notes: |
      Same user-scoped MCP server table as macOS. The CLI, the IDE extension,
      and the app-server all read this file.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\config.toml"
    format: toml
    notes: |
      Same `[mcp_servers.<id>]` shape; Windows default home is
      `%USERPROFILE%\.codex`.
  - os: macos
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: |
      Project-scoped MCP overrides. Loaded only when the project is trusted.
      Closest-to-cwd wins when multiple files appear along the path.
  - os: linux
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: "Same project-scoped table and trust gate as macOS."
  - os: windows
    scope: repo
    path: ".codex\\config.toml"
    format: toml
    notes: "Same project-scoped table and trust gate as macOS/Linux."
  - os: macos
    scope: managed
    path: "/etc/codex/managed_config.toml"
    format: toml
    notes: |
      Managed defaults that merge on top of user config. May include MCP
      defaults such as `[plugins.<plugin>.mcp_servers.<server>]` policy.
  - os: linux
    scope: managed
    path: "/etc/codex/managed_config.toml"
    format: toml
    notes: "Same managed defaults file as macOS."
  - os: windows
    scope: managed
    path: "%USERPROFILE%\\.codex\\managed_config.toml"
    format: toml
    notes: |
      Windows uses the user home for managed defaults (no `/etc/codex`).
      Backwards-compatible `approval_policy` and `sandbox_mode` here are
      also interpreted as requirements.
  - os: macos
    scope: managed
    path: "/etc/codex/requirements.toml"
    format: toml
    notes: |
      Admin-enforced requirements including an `[mcp_servers]` allowlist that
      disables any configured server whose name and identity (stdin `command`
      or HTTP `url`) do not match.
  - os: linux
    scope: managed
    path: "/etc/codex/requirements.toml"
    format: toml
    notes: "Same requirements file as macOS."
  - os: windows
    scope: managed
    path: "%ProgramData%\\OpenAI\\Codex\\requirements.toml"
    format: toml
    notes: |
      Windows system requirements location. `mcp_servers` allowlist rules use
      the same `[mcp_servers.<id>]` shape and match on `command` (stdio) or
      `url` (streamable HTTP).
  - os: macos
    scope: system
    path: "com.openai.codex MDM plist (`config_toml_base64`, `requirements_toml_base64`)"
    format: toml
    notes: |
      macOS MDM-managed preferences: `config_toml_base64` provides managed
      defaults (highest precedence), `requirements_toml_base64` provides
      managed requirements. Delivered by Jamf/Fleet/Kandji or similar.
  - os: macos
    scope: system
    path: "Cloud-managed requirements (ChatGPT Business/Enterprise)"
    format: toml
    notes: |
      Codex can also fetch admin-enforced requirements from the Codex
      service. The local cache is signed; missing or expired caches trigger
      a fetch with retries. Effective even when the system
      `requirements.toml` is absent.
  - os: macos
    scope: plugin
    path: "$CODEX_HOME/plugins/cache/<marketplace>/<plugin>/<sha>/.mcp.json or `plugin.json` mcpServers block"
    format: json
    notes: |
      Installed plugins can bundle MCP servers. Verified locally:
      `~/.codex/plugins/cache/openai-curated/github/<sha>/.mcp.json`
      stores the bundled server definition. User policy lives at
      `[plugins."<plugin>@<marketplace>".mcp_servers.<server>]` in
      `config.toml`.
  - os: linux
    scope: plugin
    path: "$CODEX_HOME/plugins/cache/<marketplace>/<plugin>/<sha>/.mcp.json or `plugin.json` mcpServers block"
    format: json
    notes: "Same plugin-bundled MCP server shape as macOS."
  - os: windows
    scope: plugin
    path: "%USERPROFILE%\\.codex\\plugins\\cache\\<marketplace>\\<plugin>\\<sha>\\.mcp.json or `plugin.json` mcpServers block"
    format: json
    notes: "Same plugin-bundled MCP server shape as macOS/Linux."
cli_params:
  - flag: "codex mcp add <name> -- <command> [args...]"
    description: "Add a persistent stdio MCP server to `$CODEX_HOME/config.toml`."
    example: "codex mcp add context7 -- npx -y @upstash/context7-mcp"
  - flag: "codex mcp add <name> --url <url>"
    description: "Add a persistent streamable-HTTP MCP server."
    example: "codex mcp add figma --url https://mcp.figma.com/mcp"
  - flag: "codex mcp add --env KEY=VALUE"
    description: "Set an environment variable on a stdio server (repeatable)."
  - flag: "codex mcp add --bearer-token-env-var ENV_VAR"
    description: "Environment variable whose value is sent as a bearer token for HTTP servers."
  - flag: "codex mcp add --oauth-client-id ID"
    description: "Pre-register an OAuth client id for a streamable-HTTP server."
  - flag: "codex mcp add --oauth-resource RESOURCE"
    description: "OAuth resource parameter sent during `codex mcp login`."
  - flag: "codex mcp list"
    description: "List configured MCP servers in a tabular view (Name, Url, Bearer Token Env Var, Status, Auth)."
  - flag: "codex mcp get <name> --json"
    description: "Print a single server as JSON. Output includes transport, URL, headers, enabled/disabled tool filters, startup/tool timeouts, and auth_status."
    example: "codex mcp get github --json"
  - flag: "codex mcp remove <name>"
    description: "Delete a stored MCP server definition."
  - flag: "codex mcp login <name> --scopes scope1,scope2"
    description: "Start OAuth login for a streamable-HTTP server. Browser is launched by default; the callback server binds to localhost on an ephemeral port (override with `mcp_oauth_callback_port`)."
  - flag: "codex mcp logout <name>"
    description: "Remove stored OAuth credentials (keyring + `$CODEX_HOME/.credentials.json` + `$CODEX_HOME/secrets/mcp_oauth.age` when the Secrets backend is in use)."
  - flag: "-c mcp_servers.<name>.<key>=<value>"
    description: "Override a single MCP setting for one run. Dotted-path TOML value."
    example: "codex -c 'mcp_servers.playwright.enabled=false'"
  - flag: "--profile <name>"
    description: "Layer `$CODEX_HOME/<name>.config.toml` on top of the base user config. Supported by `codex`, `codex exec`, `codex review`, `codex resume`, `codex archive`, `codex delete`, `codex unarchive`, `codex fork`, `codex mcp`, `codex sandbox`, and `codex debug prompt-input`."
  - flag: "--ignore-user-config"
    description: "Skip `$CODEX_HOME/config.toml` for one run; auth still reads `$CODEX_HOME`. Useful for clean automation envs."
  - flag: "--strict-config"
    description: "Error out when `config.toml` contains fields unknown to this Codex version."
  - flag: "codex mcp-server"
    description: "Run Codex itself as a stdio MCP server exposing its built-in tools."
env_vars:
  - name: CODEX_HOME
    effect: |
      Root for Codex state, including `config.toml`, auth, OAuth credentials,
      plugins, prompts, sessions, and standalone package metadata. Defaults
      to `~/.codex`. Claudine wrapper injection uses a shadow `CODEX_HOME`
      containing a generated `config.toml` so the user state is preserved.
  - name: CODEX_SQLITE_HOME
    effect: |
      Overrides the SQLite state directory. Defaults to `$CODEX_HOME`. The
      `sqlite_home` config option takes precedence. Relative paths resolve
      from the current working directory.
  - name: CODEX_CONNECTORS_TOKEN
    effect: |
      Debug override that supplies runtime auth for the built-in Codex Apps
      MCP server and bypasses the shared tools cache.
  - name: CODEX_API_KEY
    effect: |
      API key for a single `codex exec` run. Set inline, not job-wide, when
      running repo-controlled code.
  - name: CODEX_ACCESS_TOKEN
    effect: |
      ChatGPT or Codex access token for trusted automation. For persisted
      login, pipe to `codex login --with-access-token`.
  - name: CODEX_CA_CERTIFICATE
    effect: |
      Path to a PEM CA bundle for environments with corporate TLS
      interception. Takes precedence over `SSL_CERT_FILE`.
  - name: SSL_CERT_FILE
    effect: |
      Fallback PEM CA bundle path when `CODEX_CA_CERTIFICATE` is unset.
  - name: CODEX_NON_INTERACTIVE
    effect: |
      Set to `1`, `true`, or `yes` for the installer to skip prompts.
  - name: CODEX_INSTALL_DIR
    effect: |
      Override the visible `codex` install location (defaults to
      `~/.local/bin` on Unix, `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin` on
      Windows). The standalone package cache still lives under
      `$CODEX_HOME/packages/standalone`.
  - name: RUST_LOG
    effect: |
      Controls Rust log filtering and verbosity. `codex exec` defaults to
      `error` unless overridden.
server_schema:
  transports: ["stdio", "streamable_http", "in_process"]
  command_fields: ["command", "args", "env", "env_vars", "cwd", "experimental_environment"]
  http_fields: ["url", "bearer_token_env_var", "http_headers", "env_http_headers", "oauth_resource", "scopes"]
  env_shape: |
    `env` is an inline TOML table mapping variable names to string values.
    `env_vars` accepts plain names (read from local env) or objects of
    `{ name = "...", source = "local" | "remote" }`. `source = "remote"`
    reads from a remote executor environment and requires remote MCP stdio.
  auth_shape: |
    Streamable HTTP servers support OAuth 2.0 (via `codex mcp login`,
    `scopes`, `oauth_resource`, optional `--oauth-client-id`) and bearer
    tokens via `bearer_token_env_var`. Static `http_headers` and
    `env_http_headers` are also accepted. OAuth credentials are stored
    according to `mcp_oauth_credentials_store` (`auto`, `file`, or
    `keyring`) and `mcp_oauth_keyring_backend` (`direct` or `secrets`); they
    are never written to `config.toml`. Top-level `mcp_oauth_callback_port`
    and `mcp_oauth_callback_url` configure the OAuth redirect listener.
    If the server advertises `scopes_supported`, Codex prefers those scopes
    during `codex mcp login`; otherwise it falls back to `config.toml`.
  notes: |
    Server id is the TOML table key `[mcp_servers.<id>]`. Codex infers
    transport from the presence of `command` (stdio) versus `url`
    (streamable HTTP); there is no explicit `type` field in the user TOML.
    Plugin-bundled servers share the same key shape but live in the
    plugin's `.mcp.json` (JSON, with an explicit `type: "http"` field) or
    in `plugin.json` under `mcpServers`. User policy for plugin servers
    lives at `[plugins."<plugin>@<marketplace>".mcp_servers.<server>]`.
    The in-process transport exists for embedded SDK use and is not
    user-facing.
server_capabilities:
  tools: full
  resources: partial
  prompts: none
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: true
  prompt_list_changed: false
  notes: |
    Tools: `McpConnectionManager.list_all_tools` enumerates every server's
    tool list and runs them through `ToolFilter` (`enabled_tools` /
    `disabled_tools`) and the MCP Apps visibility filter
    (`_meta.ui.visibility` must contain `"model"` to be visible). The
    resulting tools are normalized for the Responses API, prefixed
    `mcp__<server>__<tool>` (or unprefixed when `prefix_mcp_tool_names`
    is false), and exposed to the model. `list_changed` notifications are
    honored — `RmcpClient` refreshes its tool cache as servers push
    updates. Resources: `McpConnectionManager.list_all_resources`,
    `list_all_resource_templates`, and `McpResourceClient.read_resource`
    implement the protocol-level surfaces; `list_changed` is honored and
    paged retrieval is supported. Codex does not surface resources through
    the `MentionType` autocomplete (`Plugin`/`Skill`/`File`/`Directory` are
    the only categories), and there is no `/resources` slash command —
    resources are reachable only via the internal Codex Apps link view and
    the connector-aware read paths. Prompts: not implemented.
    `RmcpClient` exposes no `list_prompts` / `get_prompt` methods, and
    there is no slash command, palette entry, or model-facing tool for
    MCP prompts.
client_capabilities:
  roots: none
  sampling: none
  elicitation: full
  notes: |
    Roots: Codex does not advertise `roots/list`. There is no client-side
    `ListRoots` service method on the RMCP client used by Codex. Sampling:
    Codex does not implement `sampling/createMessage` or
    `completion/complete` as a client; servers cannot request LLM
    completions through Codex. Elicitation: `ElicitationClientService`
    handles both standard MCP `CreateElicitationRequest` modes (form + URL)
    and a vendor extension called `openai/form` (form-style with an
    extended schema, including OpenAI image-picker fields). Requests flow
    to the UI through `ElicitationRequestEvent`. Auto-accept: schema
    forms whose `properties` are empty and whose effective permission
    profile is auto-approve are accepted with `{"action":"accept",
    "content":{}}`. Auto-deny: `AskForApproval::Never` or `Granular`
    without `mcp_elicitations` set; the `ElicitationRequestManager` also
    exposes an `auto_deny` toggle.
tool_surface:
  discovery: |
    `tools/list` is called at server startup for every connected server,
    cached, and refreshed on `list_changed`. Codex also implements
    streamable-HTTP retry on transient `tools/list` errors
    (`STREAMABLE_HTTP_RETRY_DELAYS_MS`). Tool descriptions and server
    instructions are truncated to 2 KB before reaching the model.
  filtering: |
    Per-server `enabled_tools` / `disabled_tools` (allow/deny by exact tool
    name), the MCP Apps `_meta.ui.visibility` filter (a tool is hidden when
    its visibility array does not include `"model"`), per-tool
    `tools.<tool>.approval_mode` overrides, and managed
    `requirements.toml` `[mcp_servers]` allowlists (matched by `command`
    for stdio servers and by `url` for streamable HTTP servers; argument
    structure can be matched via `command.executable` +
    `command.args[]`). Plugin-bundled servers gain the same filters under
    `[plugins.<plugin>.mcp_servers.<server>]`.
  approval: |
    Per-server `default_tools_approval_mode` and per-tool
    `tools.<tool>.approval_mode` accept `auto`, `prompt`, or `approve`.
    Codex routes MCP calls through the same approval policy as native
    tools; the granular `mcp_elicitations` flag controls elicitation
    prompts independently of tool approval.
  result_handling: |
    Tool text and image results are passed to the model. Tool errors are
    surfaced with `isError` and routed through `mcp_init_error_display`,
    which produces a tailored hint for known failure modes (e.g. GitHub
    MCP without OAuth, startup timeouts, auth-required errors). No native
    output sanitization or truncation is documented for MCP results.
  annotations_trusted: |
    The MCP Apps visibility annotation (`_meta.ui.visibility`) is honored
    as policy for model visibility. Connector metadata
    (`_meta.connector_id`, `_meta.connector_name`,
    `_meta.connector_description`) is read for telemetry and the Codex
    Apps link view. Other `tool.annotations` values are treated as hints
    only.
  notes: |
    The server's `instructions` field is read during initialization and
    prepended to the model-visible tool guidance. The docs recommend
    keeping the first 512 characters self-contained. Required servers
    (`required = true`) cause `codex exec` to exit if they fail to
    initialize; non-required servers report failure but the session
    continues.
resource_surface:
  supported: true
  uri_schemes: []
  templates: true
  subscriptions: false
  exposure_model: |
    `McpConnectionManager.list_all_resources` and
    `list_all_resource_templates` paginate every connected server and
    cache the union. `McpResourceClient` exposes `list_resources(server,
    cursor)` and `read_resource(server, uri)` to internal callers (notably
    the Codex Apps link view for connectors). The user-facing
    `MentionType` autocomplete does not include a `Resource` variant — the
    TUI mentions catalog only contains `Plugin`, `Skill`, `File`, and
    `Directory` — and there is no `/resources` slash command. Resources
    attached inside MCP tool results may still be rendered; resource
    subscriptions (`resources/subscribe`) are not advertised by Codex.
  notes: |
    `list_changed` notifications are honored (resource refreshes on
    `McpResourceClient`'s `cache_key` change), but no UI surface is
    exposed. Templates are listed and usable; subscriptions are not.
prompt_surface:
  supported: false
  invocation: ""
  arguments: ""
  exposure_model: |
    MCP prompts are not exposed in any surface. There is no
    `list_prompts`/`get_prompt` implementation in Codex's `RmcpClient`, no
    slash command, no palette entry, and no model-facing prompt tool.
  notes: |
    The Codex plugin system provides plugin-scoped prompts (visible under
    `$PROMPT` and `/prompts`), but these are local prompts and are not
    the same as MCP prompts.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: deep
  notes: |
    Claudine can read `$CODEX_HOME/config.toml` (and the project-scoped
    `.codex/config.toml` in trusted projects), normalize the
    `[mcp_servers.<id>]` tables plus
    `[plugins.<plugin>.mcp_servers.<server>]` policy into the catalog,
    and write them back through either file edits or the supported
    `codex mcp add` / `codex mcp remove` CLI. Plugin `.mcp.json` files
    live under `$CODEX_HOME/plugins/cache/<marketplace>/<plugin>/<sha>/`
    and can also be read for import; they are managed by the Codex
    installer and should not be written by a third party. Per-server
    tables replace by id when the same id appears in multiple layers;
    general TOML tables merge deeply. Cloud-managed requirements take
    precedence over local `requirements.toml`, which takes precedence
    over `config.toml` for security-sensitive settings.
runtime_injection:
  supported: true
  mechanism: |
    Set `CODEX_HOME` to a temporary directory containing a generated
    `.codex/config.toml` with the desired `[mcp_servers]` tables, then
    launch `codex` normally. This is how Claudine's wrapper performs
    one-run injection without editing the user's persistent config.
  limitations: |
    There is no native one-run MCP flag. A shadow `CODEX_HOME` does not
    inherit saved auth, OAuth tokens, history, sessions, or other
    non-config state, so the caller must re-supply anything needed.
    Project-scoped `.codex/config.toml` may still load if the project is
    trusted. OAuth flows cannot complete in non-interactive `codex exec`;
    pre-authenticated servers or bearer tokens are required.
authorization:
  oauth: true
  credential_storage: |
    OAuth credentials are stored according to `mcp_oauth_credentials_store`
    (`auto`, `file`, or `keyring`) and `mcp_oauth_keyring_backend` (`direct`
    or `secrets`). On Linux the Direct backend prefers
    `linux-native-async-persistent` (DBus Secret Service + kernel
    keyutils); on macOS it uses the Keychain; on Windows it uses
    Credential Manager. The Secrets backend writes to
    `$CODEX_HOME/secrets/mcp_oauth.age` (encrypted via `SecretsManager`).
    `auto` falls back to `$CODEX_HOME/.credentials.json` when the keyring
    is unavailable; the fallback file is created with `0o600` perms on
    Unix. Bearer tokens for HTTP servers are referenced by
    `bearer_token_env_var` and are never written to `config.toml`.
  token_scope: |
    Per streamable-HTTP server URL and per `scopes` value when set.
    Refresh tokens are stored and refreshed automatically with a 30s
    skew before `expires_in`. `oauth_resource` is sent as the OAuth
    resource parameter during `codex mcp login`.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` table or the
    `env_vars` whitelist. Process environment inheritance is governed by
    `shell_environment_policy`. Stdio servers do not receive an HTTP
    auth header; auth is exclusively through env.
  notes: |
    OAuth discovery follows RFC 9728 then RFC 8414; Codex falls back to a
    stored bearer token when the server does not advertise OAuth
    metadata. Local callback URLs (`localhost`) bind on the local
    interface; non-local callback URLs (set via `mcp_oauth_callback_url`)
    bind on `0.0.0.0` so the callback can reach the host. Register the
    full derived `redirect_uri` (including the appended server-specific
    callback ID) with the OAuth provider.
security:
  tool_filtering: |
    Per-server `enabled_tools` / `disabled_tools`, per-tool
    `tools.<tool>.approval_mode`, and the MCP Apps visibility filter
    (`_meta.ui.visibility`). Managed `requirements.toml` provides an
    `[mcp_servers]` allowlist matched on `command` (with optional
    `executable` + positional `args[]`) for stdio and on `url` for streamable
    HTTP; unmatched servers are disabled.
  server_trust: |
    Project `.codex/config.toml` MCP servers load only for trusted
    projects. Managed `requirements.toml` can disable mismatched servers
    and pin feature flags. macOS MDM `config_toml_base64` /
    `requirements_toml_base64` provide managed defaults and requirements
    with the highest precedence.
  env_sanitization: |
    `shell_environment_policy` controls which environment variables are
    forwarded to subprocesses (including stdio MCP servers), with a
    default that excludes secret-like keys. Per-server `env` and
    `env_vars` give explicit, opt-in control. OAuth bearer tokens never
    land in `config.toml` — they are referenced through
    `bearer_token_env_var`.
  sandbox_interaction: |
    MCP stdio servers run as ordinary local processes and are not
    isolated by Codex's command sandbox (which applies to model-generated
    shell commands). Plugin-bundled MCP servers inherit the same
    sandboxing as user-configured servers.
  response_filtering: |
    No native MCP response sanitization is documented. Large outputs are
    not truncated by Codex itself; the `protect` layer in Claudine
    should treat MCP tool results as untrusted content.
  notes: |
    OAuth tokens are stored in the OS credential store (keyring) with
    `$CODEX_HOME/.credentials.json` and `$CODEX_HOME/secrets/mcp_oauth.age`
    fallbacks. Enterprises should deploy `requirements.toml` with an
    explicit `[mcp_servers]` allowlist and pin the granular approval
    policy and feature flags.
gaps:
  - |
    The official docs do not state which MCP protocol version date Codex
    implements (e.g. 2024-11-05, 2025-06-18, or 2025-11-25). The
    observable surface (elicitation, MCP Apps visibility, connector meta)
    lines up with the 2025-06-18 and later drafts.
  - |
    Tool result size limits, response sanitization, and native output
    truncation are not documented.
  - |
    Resources are implemented in the protocol layer but not surfaced in
    the TUI mention catalog or via a `/resources` slash command.
    Documentation does not describe the intended exposure model beyond
    Codex Apps link surfaces.
  - |
    No first-class OS-level sandbox or container boundary for stdio MCP
    servers is documented; isolation must come from the surrounding shell
    environment or the Codex command sandbox for shell commands.
  - |
    Codex does not advertise a sampling or roots capability; the docs do
    not explicitly call out the absence, but the RMCP client surface
    (no `create_message`, no `ListRoots` service) is the source of
    truth.
  - |
    The exact precedence between `mcp_servers` allowlists in
    `requirements.toml` (admin-enforced) and matching `enabled = false`
    in `config.toml` (user) is not documented; the source treats
    `requirements.toml` as the requirements layer that can constrain or
    disable server identity.
changes:
  - "Resources: previously `none`; now `partial` — `McpConnectionManager` and `McpResourceClient` implement `list_resources`, `list_resource_templates`, and `read_resource`, and `list_changed` notifications are honored. The user-facing `MentionType` autocomplete does not include `Resource`, however, and there is no `/resources` slash command."
  - "Prompts: previously `none`; confirmed `none` from source (no `list_prompts`/`get_prompt` in `RmcpClient`)."
  - "Sampling: previously `unknown`; now `none` — Codex does not implement `sampling/createMessage` and does not advertise itself as a sampling client."
  - "Roots: previously `unknown`; now `none` — no `roots/list` service method is implemented in Codex's RMCP client."
  - "Elicitation: previously `full`; confirmed `full` — Form + URL MCP elicitation plus a vendor `openai/form` extension are handled by `ElicitationClientService` and surfaced through `ElicitationRequestEvent`. Auto-accept/auto-deny are gated on `AskForApproval` plus the active `PermissionProfile`."
  - "Added Codex Apps connector model: built-in `codex_apps` MCP server, connector metadata extracted from tool `_meta.connector_id` / `_meta.connector_name` / `_meta.connector_description`, and an auth-failure meta shape (`_codex_apps.connector_auth_failure.*`). Linked to the `enable_mcp_apps` feature flag."
  - "Added tool visibility filter via `_meta.ui.visibility` (MCP Apps spec, 2026-01-26). Tools whose visibility array does not include `\"model\"` are hidden from the model; tools without the metadata remain visible."
  - "Added plugin-bundled MCP server shape: plugin `.mcp.json` (JSON, verified locally for `github@openai-curated`) holds the transport; user policy lives at `[plugins.\"<plugin>@<marketplace>\".mcp_servers.<server>]` in `config.toml`."
  - "Verified the on-host Codex CLI version: 0.142.5; `codex mcp list` shows the GitHub MCP server (added via the bundled plugin) with `Bearer token` auth using `GITHUB_PAT_TOKEN`."
  - "Added official OAuth storage detail: keyring (`direct` or `secrets` backend) with `$CODEX_HOME/.credentials.json` and `$CODEX_HOME/secrets/mcp_oauth.age` fallback paths; modes are `auto` / `file` / `keyring`."
  - "Added `CODEX_SQLITE_HOME` env var (SQLite state directory override; the `sqlite_home` config option takes precedence)."
  - "Added `CODEX_CONNECTORS_TOKEN` env var: debug override that supplies Codex Apps runtime auth and bypasses the shared tools cache."
  - "Added `CODEX_INSTALL_DIR` and `CODEX_NON_INTERACTIVE` env vars for the standalone installer."
  - "Added `--ignore-user-config` flag (`codex exec`): skip `$CODEX_HOME/config.toml` for one run; auth still reads `$CODEX_HOME`."
  - "Added `--strict-config` flag: error out on unknown `config.toml` keys."
  - "Added explicit `--profile` support list (`codex`, `codex exec`, `codex review`, `codex resume`, `codex archive`, `codex delete`, `codex unarchive`, `codex fork`, `codex mcp`, `codex sandbox`, `codex debug prompt-input`)."
  - "Added `codex mcp get <name> --json` JSON output that includes transport, URL, headers, enabled/disabled filters, timeouts, and auth_status (verified locally)."
  - "Added `codex mcp login --scopes` (comma-separated) and the related `--oauth-client-id` / `--oauth-resource` flags from the CLI source."
  - "Added `mcp_oauth_callback_port` and `mcp_oauth_callback_url` top-level config: localhost binds to the local interface, non-local binds to `0.0.0.0`; Codex appends a server-specific callback ID to the base URL before sending the `redirect_uri`."
  - "Added streamable-HTTP retry/reconnect detail: transient `tools/list`/`resources/list`/`resources/templates/list` errors retry on `STREAMABLE_HTTP_RETRY_DELAYS_MS`; auth and 4xx failures are not retried; 404 from the session is treated as expired and the client auto-reinitializes."
  - "Added startup event model: `McpStartupUpdateEvent` per server (`Starting`/`Ready`/`Failed`/`Cancelled`), `McpStartupCompleteEvent` summary at session start."
  - "Added the manager-level OAuth scope resolution: `McpOAuthScopesSource` (`ServerAdvertised` vs `Configured`) and `should_retry_without_scopes` for fallback behavior."
requires_claudine_update: true
reason: |
  Three behaviors are now provable rather than guessed and require Claudine
  metadata updates: resources are `partial` (the protocol layer implements
  them, but Codex does not surface them through the TUI mention catalog or a
  slash command); sampling and roots are `none` (not `unknown`); the MCP
  Apps visibility filter (`_meta.ui.visibility`) is an enforceable model-side
  policy that the catalog should surface alongside `enabled_tools` and
  `disabled_tools`. OAuth credential storage has more nuance than the prior
  research captured: `mcp_oauth_credentials_store` has three modes
  (`auto`/`file`/`keyring`) and `mcp_oauth_keyring_backend` adds a `secrets`
  option that writes `$CODEX_HOME/secrets/mcp_oauth.age`. The Claudine MCP
  catalog should record both values and the fallback path. New env vars
  (`CODEX_SQLITE_HOME`, `CODEX_CONNECTORS_TOKEN`) and managed-defaults sources
  (`/etc/codex/managed_config.toml`, macOS MDM `config_toml_base64`,
  cloud-managed requirements) need to be reflected in the provider-state
  metadata so the wrapper knows where each layer lives. Plugin-bundled MCP
  servers (`~/.codex/plugins/cache/<marketplace>/<plugin>/<sha>/.mcp.json`)
  should be read for import but not written; the import path needs to record
  plugin provenance. The runtime injector should keep using a shadow
  `CODEX_HOME`, but `--ignore-user-config` plus `--strict-config` is a useful
  pair for CI/scripted runs and should appear in the wrapper notes.
---

# MCP Support in Codex CLI

## Overview

Codex CLI supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io)
as a tool- and context-extension mechanism. MCP servers can be local stdio
processes or remote streamable-HTTP endpoints; OAuth or bearer-token
authentication is supported for HTTP servers. Configuration lives in TOML
files alongside other Codex settings, with per-project overrides, managed
defaults, and managed requirements layered on top. The same configuration is
shared with the Codex IDE extension and the Codex app-server. For Claudine,
Codex is a strong `import_sync` target: persistent config files
(`$CODEX_HOME/config.toml`, project `.codex/config.toml`, plugin `.mcp.json`)
can be read and normalized into Claudine's MCP catalog, written back through
either file edits or `codex mcp add` / `codex mcp remove`, and one-run
injection is supported through a shadow `CODEX_HOME`.

Surface inventory (one-line):

- **Tools** — exposed: every server's tool list reaches the model with the
  `mcp__<server>__<tool>` prefix (or unprefixed when `prefix_mcp_tool_names`
  is false). MCP Apps visibility metadata is honored.
- **Resources** — partially exposed: the protocol-level surface
  (`list_resources`, `list_resource_templates`, `read_resource`) is
  implemented and used internally by Codex Apps; the user-facing TUI mention
  catalog does not include a `Resource` variant, and there is no
  `/resources` slash command.
- **Prompts** — not exposed: no `list_prompts` / `get_prompt` implementation,
  no slash command, no model-facing prompt tool.
- **Roots** — not advertised: Codex does not implement `roots/list`.
- **Sampling** — not advertised: Codex does not implement
  `sampling/createMessage` or `completion/complete`.
- **Elicitation** — exposed: Form + URL MCP elicitation plus a vendor
  `openai/form` extension. Auto-accept and auto-deny are gated on
  `AskForApproval` plus the active `PermissionProfile`.
- **Codex Apps (vendor)** — exposed: built-in `codex_apps` MCP server with
  connector metadata in tool `_meta` and an auth-failure meta shape; gated on
  the `enable_mcp_apps` feature flag.

## Protocol and Transports

Codex accepts two transports and exposes a third in-process kind for
embedded SDK use:

| Transport | Status | How it is added |
| :-------- | :----- | :-------------- |
| `stdio` | Primary | `codex mcp add <name> -- <command>` |
| `streamable_http` | Supported | `codex mcp add <name> --url <url>` |
| `in_process` | SDK embed | `RmcpClient::new_in_process_client` only |

Legacy HTTP+SSE is not documented and not accepted. The implementation is
built on `rmcp = "1.8.0"`, so any new RMCP transport added upstream can land
in Codex without a user-visible API change.

Lifecycle behavior:

- **Startup**: every enabled server is spawned (stdio) or connected
  (streamable HTTP) in parallel; per-server `McpStartupUpdateEvent`
  (`Starting` → `Ready`/`Failed`/`Cancelled`) and a final
  `McpStartupCompleteEvent` summary are emitted. Servers marked
  `required = true` must reach `Ready`; otherwise `codex exec` exits with an
  error.
- **Tool/resource/template listing**: `RmcpClient` retries transient
  transport errors with the schedule in `STREAMABLE_HTTP_RETRY_DELAYS_MS`
  inside the per-operation timeout; auth errors and 4xx failures are not
  retried. Successful calls persist OAuth tokens when they were refreshed
  during the call.
- **Session recovery**: a streamable-HTTP session that returns `404` is
  treated as expired; the client transparently re-initializes the
  connection (one concurrent recovery per server).
- **OAuth token refresh**: refresh tokens are stored and refreshed
  automatically with a 30-second skew before `expires_in`. The refresh
  runs after every successful call and on startup if the loaded tokens are
  already past the skew window.
- **Server `instructions`**: read once during the MCP handshake and
  prepended to the model-visible tool guidance. The docs recommend keeping
  the first 512 characters self-contained.

The documentation does not name an explicit MCP protocol version date. The
observable feature generation includes elicitation (Form + URL + the vendor
`openai/form` extension), MCP Apps visibility filtering
(`_meta.ui.visibility`), and connector metadata in tool `_meta`.

## Configuration

MCP servers are configured under `[mcp_servers.<id>]` tables in `config.toml`.

### Scopes

| Scope | File | Shared | Trust gate |
| :---- | :--- | :----- | :--------- |
| User | `$CODEX_HOME/config.toml` (default `~/.codex/config.toml`) | No | None |
| Project | `.codex/config.toml` | Yes (git) | Requires trusted project |
| Managed defaults | `/etc/codex/managed_config.toml` (Unix); `%USERPROFILE%\.codex\managed_config.toml` (Windows/non-Unix) | Organization-wide | Admin-controlled |
| Requirements | `/etc/codex/requirements.toml` (Unix); `%ProgramData%\OpenAI\Codex\requirements.toml` (Windows) | Organization-wide | Admin-enforced |
| macOS MDM preferences | `com.openai.codex` plist (`config_toml_base64`, `requirements_toml_base64`) | Organization-wide | Highest precedence |
| Cloud requirements | Codex service (ChatGPT Business/Enterprise) — cached locally | Organization-wide | Highest precedence |
| Plugin | `<plugin-root>/.mcp.json` or `plugin.json` `mcpServers` block (JSON); user policy under `plugins.<plugin>.mcp_servers.<server>` in `config.toml` | With plugin | Plugin trust |

### Precedence

Managed layers override user config. For requirements, precedence is cloud
managed → macOS MDM → system `requirements.toml` → user config. For ordinary
settings (managed defaults), precedence is MDM → `managed_config.toml` →
`config.toml` → CLI `--config`. Per-server TOML tables replace by id when the
same id appears in multiple layers; general tables merge deeply.

### Project trust

Untrusted projects ignore project-local `.codex/config.toml` (and any other
project-scoped config). User, managed, MDM, and cloud-managed layers still
load. This matters for Claudine wrappers: repo-level MCP servers should not
be assumed active until the user has trusted the project.

## Server Definition Shape

A stdio server:

```toml
[mcp_servers.context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp"]
env_vars = ["LOCAL_TOKEN"]

[mcp_servers.context7.env]
MY_ENV_VAR = "MY_ENV_VALUE"
```

```toml
[mcp_servers.context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp"]
env_vars = [
  "LOCAL_TOKEN",
  { name = "REMOTE_TOKEN", source = "remote" },
]
```

A streamable HTTP server:

```toml
[mcp_servers.figma]
url = "https://mcp.figma.com/mcp"
bearer_token_env_var = "FIGMA_OAUTH_TOKEN"
http_headers = { "X-Figma-Region" = "us-east-1" }
scopes = ["files:read"]
oauth_resource = "https://mcp.figma.com/"
```

OAuth callback overrides:

```toml
mcp_oauth_callback_port = 5555
mcp_oauth_callback_url = "https://devbox.example.internal/callback"
```

Per-tool approval and filter knobs:

```toml
[mcp_servers.chrome_devtools]
url = "http://localhost:3000/mcp"
enabled_tools = ["open", "screenshot"]
disabled_tools = ["screenshot"] # applied after enabled_tools
default_tools_approval_mode = "prompt"
startup_timeout_sec = 20
tool_timeout_sec = 45
enabled = true

[mcp_servers.chrome_devtools.tools.open]
approval_mode = "approve"
```

### Common fields

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `command` | stdio | Executable to spawn |
| `args` | stdio | Argument array |
| `env` | stdio | Inline table of environment variables |
| `env_vars` | stdio | Variable names to whitelist (strings or `{ name, source }` objects; `source = "remote"` requires remote MCP stdio) |
| `cwd` | stdio | Working directory for the server process |
| `experimental_environment` | stdio | `local` or `remote` placement |
| `url` | HTTP | Endpoint URL |
| `bearer_token_env_var` | HTTP | Env var supplying a bearer token |
| `http_headers` | HTTP | Static header map |
| `env_http_headers` | HTTP | Header names mapped to env var names |
| `scopes` | HTTP | OAuth scopes to request |
| `oauth_resource` | HTTP | OAuth resource parameter |
| `startup_timeout_sec` | all | Server startup timeout (default 10s) |
| `tool_timeout_sec` | all | Per-tool timeout (default 60s) |
| `enabled` | all | Set `false` to disable without deleting |
| `required` | all | Fail startup if the enabled server cannot initialize |
| `enabled_tools` | all | Tool allow list |
| `disabled_tools` | all | Tool deny list (applied after `enabled_tools`) |
| `default_tools_approval_mode` | all | Default approval behavior for this server's tools (`auto`, `prompt`, `approve`) |
| `tools.<tool>.approval_mode` | all | Per-tool approval override |

The server id is the TOML table key. Transport is inferred from the
presence of `command` (stdio) versus `url` (streamable HTTP); there is no
explicit `type` field. Plugin-bundled servers use a JSON `.mcp.json` shape
with an explicit `type: "http"` field and live alongside the plugin's other
assets.

## Tools, Resources, and Prompts

### Tools

Codex exposes MCP tools to the model. Tool exposure is controlled by:

- `enabled_tools` / `disabled_tools` — exact tool name allow/deny.
- `default_tools_approval_mode` and `tools.<tool>.approval_mode` — `auto`,
  `prompt`, or `approve`.
- The MCP Apps visibility filter — a tool is hidden from the model unless
  `_meta.ui.visibility` contains `"model"`. Tools without visibility
  metadata remain visible.
- Managed `requirements.toml` `[mcp_servers]` allowlists — matched on
  `command` (with optional structured `executable` + `args[]`) for stdio
  servers and on `url` for streamable HTTP servers; unmatched servers are
  disabled. Plugin-bundled MCP servers share the same identity shapes
  under `plugins.<plugin>.mcp_servers.<server>`.

Approval modes flow through Codex's overall approval policy; the granular
`mcp_elicitations` flag controls elicitation prompts independently of tool
approval. The server `instructions` field is read during initialization and
used as server-wide guidance (first 512 characters should be self-contained).

### Resources

The protocol-level surface is implemented. `McpConnectionManager`
aggregates `list_all_resources` and `list_all_resource_templates` across
servers, and `McpResourceClient` exposes `list_resources(server, cursor)`
and `read_resource(server, uri)` to internal callers (notably the Codex
Apps link view for connectors). `list_changed` notifications refresh the
manager's cache.

The TUI mention catalog does not include a `Resource` variant (`Plugin`,
`Skill`, `File`, `Directory` are the only categories in
`MentionType::label`), and there is no `/resources` slash command.
Resources attached inside MCP tool results may still be rendered by the
existing tool-result surface, but the user has no direct picker.
Subscriptions (`resources/subscribe`) are not advertised by Codex.

### Prompts

The public documentation does not describe a user-facing prompt catalog,
and the implementation does not include `list_prompts` or `get_prompt` in
the `RmcpClient` API. Codex consumes server `instructions` for guidance,
but that is distinct from exposing MCP prompts. Claudine should assume
prompts are not surfaced.

## Roots, Sampling, and Elicitation

| Capability | Status | Notes |
| :--------- | :----- | :---- |
| Roots | none | No `roots/list` service method in Codex's RMCP client. |
| Sampling | none | No `sampling/createMessage` or `completion/complete`. |
| Elicitation | full | Form + URL MCP elicitation plus the `openai/form` vendor extension. Routed to the UI through `ElicitationRequestEvent`. |

Elicitation flows through `ElicitationClientService` to the protocol layer.
The three request shapes are:

- `Mcp(...FormElicitationParams { message, requested_schema, meta })` —
  rendered as a form, with the schema's `properties` driving the UI
  controls.
- `Mcp(...UrlElicitationParams { message, url, elicitation_id, meta })` —
  rendered as a link the user opens in a browser.
- `OpenAiForm { message, requested_schema, meta }` — the `openai/form`
  vendor extension with extended schema (including OpenAI image-picker
  fields), for Codex Apps connectors.

Auto-accept behavior: when `mcp_permission_prompt_is_auto_approved(...)`
returns true and the schema's `properties` is empty, the request is
answered with `{"action":"accept","content":{}}` without surfacing it.
Auto-deny behavior: `AskForApproval::Never` or a granular policy without
`mcp_elicitations`, plus an `auto_deny` toggle on the manager, declines
immediately. Codex surfaces both auto-accept and auto-deny through
existing approval-policy controls; no separate flag is exposed.

## Import, Export, and Sync

Claudine can treat Codex as an `import_sync` provider:

- **Import**: read `$CODEX_HOME/config.toml` and the project-scoped
  `.codex/config.toml` (in trusted projects); normalize `[mcp_servers.<id>]`
  tables into the catalog. Plugin-bundled servers can be read from
  `$CODEX_HOME/plugins/cache/<marketplace>/<plugin>/<sha>/.mcp.json` for
  visibility, but should not be rewritten (the Codex installer manages
  those files).
- **Export**: write provider-shaped TOML back to those files using the
  schema above.
- **Apply**: use `codex mcp add`, `codex mcp add --url`,
  `codex mcp login`, `codex mcp logout`, and `codex mcp remove` to mutate
  configuration through the supported CLI.

Merge semantics:

- Per-server TOML tables replace by id when the same id appears in
  multiple layers.
- General TOML tables merge deeply.
- Managed `requirements.toml` can disable servers whose name and identity
  do not match an allowlist (matched on `command` for stdio, `url` for
  streamable HTTP).
- Cloud-managed requirements take precedence over local `requirements.toml`,
  which takes precedence over `config.toml` for security-sensitive
  settings.

## Runtime Injection

Codex has no official one-run MCP flag, but injection is straightforward by
redirecting `CODEX_HOME`:

1. Create a temporary directory.
2. Write `.codex/config.toml` inside it with the desired `[mcp_servers]`
   tables.
3. Launch `codex` with `CODEX_HOME` pointing at the temporary directory.

This is the mechanism Claudine uses for wrapper-level runtime injection. It
does not mutate the user's persistent `$CODEX_HOME/config.toml`.

Limitations:

- The shadow home does not inherit saved authentication, OAuth tokens,
  history, sessions, or other non-config state, so the caller must
  re-supply anything needed.
- If the working directory is a trusted project, project-scoped
  `.codex/config.toml` may still load.
- For one-run scripted execution, pair the shadow home with
  `--ignore-user-config` to bypass `$CODEX_HOME/config.toml` cleanly and
  with `--strict-config` to surface unknown keys as errors.
- OAuth flows cannot complete in non-interactive `codex exec`; pre-authenticated
  servers, bearer tokens, or pre-registered OAuth client IDs are required.

## Authorization and Credentials

| Pattern | Where configured | Credential storage |
| :------ | :--------------- | :----------------- |
| Static header | `http_headers` | In config file (not recommended for shared repos) |
| Header from env | `env_http_headers` | Env var referenced at runtime |
| Bearer token | `bearer_token_env_var` | Env var referenced at runtime |
| OAuth 2.0 | `scopes`, `oauth_resource`, `--oauth-client-id` | `mcp_oauth_credentials_store` (`auto`/`file`/`keyring`) + `mcp_oauth_keyring_backend` (`direct`/`secrets`) |

For stdio servers, secrets should be passed through the `env` table or
`env_vars` whitelist rather than committed in config files.

OAuth storage:

- `auto` (default) reads from the OS keyring with fallback to
  `$CODEX_HOME/.credentials.json`.
- `file` writes/reads only `$CODEX_HOME/.credentials.json` (mode `0o600` on
  Unix).
- `keyring` writes/reads only the OS keyring.
- `direct` keyring backend uses the standard keyring crate (Keychain /
  Credential Manager / DBus Secret Service + kernel keyutils on Linux).
- `secrets` keyring backend writes encrypted blobs to
  `$CODEX_HOME/secrets/mcp_oauth.age` via `SecretsManager`.

OAuth callback:

- `mcp_oauth_callback_port` pins the redirect URI port; default is
  ephemeral.
- `mcp_oauth_callback_url` overrides the base redirect URL (useful for
  remote Devbox ingresses); Codex appends a server-specific callback ID
  before sending `redirect_uri`. Register the full derived URL with the
  OAuth provider.
- Localhost URLs bind on the local interface; non-local URLs bind on
  `0.0.0.0` so the callback can reach the host.

## Security Model

### Trust and allowlisting

- Project `.codex/config.toml` servers load only for trusted projects.
- Managed `requirements.toml` provides admin-enforced `[mcp_servers]`
  allowlists matched by command identity for stdio and by URL for streamable
  HTTP.
- Managed defaults (`managed_config.toml`, MDM `config_toml_base64`,
  cloud-managed requirements) layer on top of user config.
- Feature flags in `requirements.toml` can disable MCP-related surfaces
  entirely (for example `multi_agent = false`, `plugins = false`).

### Environment and sandboxing

- MCP stdio servers inherit process environment according to
  `shell_environment_policy`, which can exclude secret-like keys by
  default.
- Per-server `env` and `env_vars` give explicit control over variables
  forwarded to a server.
- Codex's command sandbox applies to model-generated shell commands, not
  to MCP server processes. There is no documented OS-level sandbox around
  stdio MCP servers.

### Response handling

- No native MCP response sanitization is documented.
- Claudine's `protect` layer should treat MCP tool results as untrusted
  content.
- Tool errors are surfaced with `isError` and routed through
  `mcp_init_error_display`, which produces tailored hints for known
  failure modes (e.g. GitHub MCP without OAuth, startup timeouts,
  auth-required errors).

## Mode-Specific Behavior

### Interactive TUI

- `/mcp` lists configured MCP tools. Per the `SlashCommand::Mcp` help
  text: "list configured MCP tools; use `/mcp verbose` for details."
- `/mcp` is part of the chat-wide slash-command dispatch and follows the
  same input flow as other commands.
- Project servers from `.codex/config.toml` are available only after the
  project is trusted.

### Non-interactive `codex exec`

- Reuses saved CLI authentication by default; use `CODEX_API_KEY` for
  API-key automation, or `CODEX_ACCESS_TOKEN` for ChatGPT/Codex access
  tokens.
- OAuth login flows cannot run interactively; use bearer tokens, static
  headers, or pre-registered OAuth configs.
- If an enabled MCP server has `required = true` and fails to initialize,
  the command exits with an error.
- `--ignore-user-config` skips `$CODEX_HOME/config.toml` for one run;
  `--strict-config` errors on unknown `config.toml` keys.
- Startup events surface in the JSONL event stream as
  `McpStartupUpdateEvent` and `McpStartupCompleteEvent`.

### Codex as an MCP server

`codex mcp-server` runs Codex itself as a stdio MCP server, exposing its
built-in tools to an external MCP client. The external client is
responsible for its own approval UI.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| `required` server fails to start | `codex exec` exits with an error; the failure reason is reported as `ReauthenticationRequired` when the cause is auth. |
| Non-required server fails to start | Server unavailable; session continues; status surfaces in `/mcp` and `McpStartupCompleteEvent`. |
| Stdio server exits mid-session | Not documented as auto-reconnected. |
| Streamable HTTP transient error on `tools/list` / `resources/list` / `resources/templates/list` | Retried per `STREAMABLE_HTTP_RETRY_DELAYS_MS` inside the per-operation timeout. |
| Streamable HTTP auth or 4xx error | Not retried. |
| Streamable HTTP `404` on the session | Treated as session expiry; the client auto-reinitializes. |
| OAuth token near expiry | Refreshed with a 30s skew before `expires_in`. |
| Tool timeout | Aborted after `tool_timeout_sec` (default 60s). |
| Startup timeout | Aborted after `startup_timeout_sec` (default 10s). |
| Project config untrusted | `.codex/config.toml` MCP servers are ignored. |
| `--strict-config` rejects unknown keys | Command exits with an error. |

## Gaps

- Explicit MCP protocol version date is not stated.
- Resource and prompt surfaces: protocol-level resources are implemented
  but not surfaced in the TUI mention catalog or via a `/resources`
  slash command; prompts are not implemented at all.
- `roots/list`, sampling, and dynamic capability refresh are not
  documented as supported.
- Tool result size limits, retries, and native response sanitization are
  not documented.
- No documented OS-level sandbox for stdio MCP servers.
- No official one-run MCP injection flag; injection relies on the
  `CODEX_HOME` side-effect.
- The interaction between `requirements.toml` `[mcp_servers]` allowlists
  and matching `enabled = false` in user `config.toml` is not
  documented explicitly; the source treats `requirements.toml` as the
  layer that can constrain or disable server identity.

## Claudine Integration Notes

- Treat Codex as `support: import_sync`. Map the catalog to Codex's
  `[mcp_servers.<id>]` TOML tables, preserving stdio versus streamable-HTTP
  fields, and record plugin provenance for plugin-bundled servers.
- For one-run wrappers, prefer a shadow `CODEX_HOME` with a generated
  `.codex/config.toml`; pair with `--ignore-user-config` and
  `--strict-config` for clean CI/scripted runs.
- Honor project trust: do not assume `.codex/config.toml` servers are
  active until the project is trusted.
- Record `mcp_oauth_credentials_store` and `mcp_oauth_keyring_backend`
  per server, and the OAuth credential fallback paths
  (`$CODEX_HOME/.credentials.json`,
  `$CODEX_HOME/secrets/mcp_oauth.age`).
- Treat Codex as exposing only `tools` to the model. Mark resources
  `partial` in the catalog and prompts `none`; roots and sampling are
  `none`.
- Defensively scan MCP tool results in the `protect` layer; Codex does
  not document native response sanitization.
- Plugin `.mcp.json` files under
  `$CODEX_HOME/plugins/cache/<marketplace>/<plugin>/<sha>/` should be
  read for import but never written; Claudine should record the plugin
  provenance in the catalog entry.
- Managed defaults and requirements are layered above user config; do not
  write to `/etc/codex/managed_config.toml` or
  `/etc/codex/requirements.toml` from a user-level wrapper.

## Sources

- [Codex MCP documentation](https://developers.openai.com/codex/mcp/)
- [Codex CLI reference](https://developers.openai.com/codex/cli/reference/)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference/)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables/)
- [Codex advanced configuration](https://developers.openai.com/codex/config-advanced/)
- [Codex managed configuration](https://developers.openai.com/codex/enterprise/managed-configuration/)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive/)
- [Codex plugins overview](https://developers.openai.com/codex/plugins/)
- [Codex permissions](https://developers.openai.com/codex/permissions/)
- [Model Context Protocol specification](https://modelcontextprotocol.io)
- [Codex Rust source — `rmcp-client/src/rmcp_client.rs`](https://github.com/openai/codex/blob/main/codex-rs/rmcp-client/src/rmcp_client.rs)
- [Codex Rust source — `rmcp-client/src/oauth.rs`](https://github.com/openai/codex/blob/main/codex-rs/rmcp-client/src/oauth.rs)
- [Codex Rust source — `rmcp-client/src/elicitation_client_service.rs`](https://github.com/openai/codex/blob/main/codex-rs/rmcp-client/src/elicitation_client_service.rs)
- [Codex Rust source — `rmcp-client/src/auth_status.rs`](https://github.com/openai/codex/blob/main/codex-rs/rmcp-client/src/auth_status.rs)
- [Codex Rust source — `rmcp-client/src/perform_oauth_login.rs`](https://github.com/openai/codex/blob/main/codex-rs/rmcp-client/src/perform_oauth_login.rs)
- [Codex Rust source — `codex-mcp/src/connection_manager.rs`](https://github.com/openai/codex/blob/main/codex-rs/codex-mcp/src/connection_manager.rs)
- [Codex Rust source — `codex-mcp/src/elicitation.rs`](https://github.com/openai/codex/blob/main/codex-rs/codex-mcp/src/elicitation.rs)
- [Codex Rust source — `codex-mcp/src/tools.rs`](https://github.com/openai/codex/blob/main/codex-rs/codex-mcp/src/tools.rs)
- [Codex Rust source — `codex-mcp/src/resource_client.rs`](https://github.com/openai/codex/blob/main/codex-rs/codex-mcp/src/resource_client.rs)
- [Codex Rust source — `codex-mcp/src/auth_elicitation.rs`](https://github.com/openai/codex/blob/main/codex-rs/codex-mcp/src/auth_elicitation.rs)
- [Codex Rust source — `tui/src/slash_command.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/slash_command.rs)
- [Codex Rust source — `tui/src/bottom_pane/mentions_v2/candidate.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/bottom_pane/mentions_v2/candidate.rs)
- [Codex Rust source — `tui/src/bottom_pane/mentions_v2/search_catalog.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/bottom_pane/mentions_v2/search_catalog.rs)
- [Codex Rust source — `protocol/src/mcp.rs`](https://github.com/openai/codex/blob/main/codex-rs/protocol/src/mcp.rs)
- [Codex Rust source — `exec/src/cli.rs`](https://github.com/openai/codex/blob/main/codex-rs/exec/src/cli.rs)
- [MCP Apps spec — `ext-apps`](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx)
- Local observation: `codex --version` ⇒ `codex-cli 0.142.5`; `codex mcp list`
  shows `github` (streamable HTTP, bearer token, `GITHUB_PAT_TOKEN`) coming
  from the bundled `github@openai-curated` plugin
  (`~/.codex/plugins/cache/openai-curated/github/3fdeeb49/.mcp.json`).
