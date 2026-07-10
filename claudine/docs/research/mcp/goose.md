---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://goose-docs.ai/docs/getting-started/using-extensions
support: import_sync
protocol:
  versions: ["2025-03-26"]
  transports: [stdio, streamable_http]
  lifecycle: |
    Stdio extensions are spawned as local subprocesses at session start. The
    `Sse` variant is deserialized for backward compatibility only — `is_tool_available`
    rejects it (`SSE is unsupported`). Streamable HTTP extensions connect to a
    remote URL at session start. Reconnect / retry semantics are not exposed in
    the user-facing API; the rmcp client uses standard MCP `notifications/cancelled`
    on request timeout and supports session-level call cancellation but no
    application-level reconnect is documented. Capability refresh is handled
    through the standard MCP `list_changed` notification path; the client
    subscribes to server notifications via `subscribe()` but documented
    behavior for re-query on `list_changed` is not published.
  notes: |
    The advertised protocol version on the wire is `ProtocolVersion::V_2025_03_26`
    (the rmcp crate constant); the public docs link MCP Roots to the
    `2025-06-18` client/roots spec and MCP Sampling/Elicitation to the draft
    spec. Legacy HTTP+SSE is preserved in deserialization but actively
    rejected when a tool is requested (extension `is_tool_available` returns
    false for `Sse` variants). Goose does not document a WebSocket transport.
config_files:
  - os: macos
    scope: user
    path: "~/Library/Application Support/Block/goose/config.yaml"
    format: yaml
    notes: |
      Primary user config on macOS. Holds the `extensions` map, provider / model
      settings, and `GOOSE_*` options. The directory is `Block/goose`
      intentionally (vs `goose/`) so existing Block-era installs continue to
      resolve with the etcetera `AppStrategy` they were created under.
      Docs at `/docs/guides/config-files` quote `~/.config/goose/config.yaml`
      for macOS/Linux; that is *incorrect* relative to the etcetera
      `choose_app_strategy` invocation in `config/paths.rs` (which uses
      `top_level_domain="Block"`, `author="Block"`, `app_name="goose"`).
  - os: linux
    scope: user
    path: "~/.config/Block/goose/config.yaml"
    format: yaml
    notes: |
      Primary user config on Linux. etcetera resolves this through the XDG
      config directory (`$XDG_CONFIG_HOME/Block/goose/`, default
      `~/.config/Block/goose/`). Data lives at `~/.local/share/Block/goose/`
      and state at the same path. Override the entire tree with
      `GOOSE_PATH_ROOT`.
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config.yaml"
    format: yaml
    notes: |
      Primary user config on Windows. etcetera resolves to
      `%APPDATA%\Block\goose\` (i.e. `C:\Users\<user>\AppData\Roaming\Block\goose`).
      The docs page lists this as `%APPDATA%\Block\goose\config\config.yaml`,
      which disagrees with the source.
  - os: macos
    scope: system
    path: "/etc/goose/config.yaml"
    format: yaml
    notes: |
      System-wide config file. Loaded first (lowest precedence) by `Config::default`
      via `system_config_path()`; merged into the in-memory config layer before
      user / additional layers so user values shadow it.
  - os: linux
    scope: system
    path: "/etc/goose/config.yaml"
    format: yaml
    notes: "Linux equivalent of the system `/etc/goose/config.yaml` path."
  - os: windows
    scope: system
    path: "%PROGRADATA%\\goose\\config.yaml"
    format: yaml
    notes: "Windows equivalent: `%PROGRAMDATA%\\goose\\config.yaml` (a \"goose\" path, *not* under `Block\\`)."
  - os: macos
    scope: user
    path: "~/Library/Application Support/Block/goose/secrets.yaml"
    format: yaml
    notes: |
      File-based fallback for API keys and secrets when the system keyring is
      unavailable or `GOOSE_DISABLE_KEYRING` is set (any non-empty value).
      Secrets are stored as a single YAML map under the keyring service
      `goose` / username `secrets` when the keyring is healthy; writes
      automatically fall back to this file when a keyring failure is
      classified as a "keyring unavailable" error.
  - os: linux
    scope: user
    path: "~/.config/Block/goose/secrets.yaml"
    format: yaml
    notes: "Linux equivalent of the secrets fallback file."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\secrets.yaml"
    format: yaml
    notes: "Windows equivalent of the secrets fallback file."
  - os: macos
    scope: user
    path: "~/Library/Application Support/Block/goose/permission.yaml"
    format: yaml
    notes: |
      Per-tool permission table managed via `goose configure` → Tool
      Permission. Schema is a map of permission categories (`user`,
      `smart_approve`, …) → `PermissionConfig { always_allow, ask_before,
      never_allow }` arrays of `<extension>__<tool>` strings. The agent
      auto-caches `read_only_hint == Some(false)` tool annotations into
      `smart_approve.ask_before`. The runtime store `permissions/tool_permissions.json`
      (auto-managed) is the runtime decision log.
  - os: linux
    scope: user
    path: "~/.config/Block/goose/permission.yaml"
    format: yaml
    notes: "Linux equivalent of the permission.yaml file."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\permission.yaml"
    format: yaml
    notes: "Windows equivalent of the permission.yaml file."
  - os: macos
    scope: user
    path: "~/Library/Application Support/Block/goose/adversary.md"
    format: text
    notes: |
      Optional adversarial reviewer rules file. Plain-Markdown / YAML frontmatter
      with `tools:` line listing `<extension>__<tool>` principals. File
      presence turns Adversary Mode on; absence turns it off. Reviewed
      principals default to `shell` and `computercontroller__automation_script`.
  - os: linux
    scope: user
    path: "~/.config/Block/goose/adversary.md"
    format: text
    notes: "Linux equivalent of the adversary.md file."
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\adversary.md"
    format: text
    notes: "Windows equivalent of the adversary.md file."
  - os: macos
    scope: managed
    path: "URL referenced by GOOSE_ALLOWLIST environment variable"
    format: yaml
    notes: |
      Admin-controlled extension allowlist. YAML list of `{id, command}`
      records fetched from a URL on demand and cached; refetched on every
      goose restart. When set, extension install is rejected if the proposed
      command does not match an entry exactly.
  - os: linux
    scope: managed
    path: "URL referenced by GOOSE_ALLOWLIST environment variable"
    format: yaml
    notes: "Linux equivalent of the GOOSE_ALLOWLIST URL allowlist."
  - os: windows
    scope: managed
    path: "URL referenced by GOOSE_ALLOWLIST environment variable"
    format: yaml
    notes: "Windows equivalent of the GOOSE_ALLOWLIST URL allowlist."
cli_params:
  - flag: "goose configure"
    description: |
      Interactive TUI for adding / removing / toggling extensions (MCP
      servers), the provider, tool permissions, and tool output settings.
      This is the only Claude-Code-style `claude mcp add` analogue.
    example: "goose configure"
  - flag: "goose session --with-extension <command>"
    description: "Enable a stdio MCP extension for the current session only."
    example: |
      goose session --with-extension "npx -y @modelcontextprotocol/server-memory"
  - flag: "goose session --with-streamable-http-extension <url>"
    description: "Enable a remote Streamable HTTP MCP extension for the current session only. May be repeated."
    example: |
      goose session --with-streamable-http-extension "https://example.com/mcp"
  - flag: "goose session --with-builtin <id>"
    description: "Enable a built-in MCP extension for the current session only. Comma-separated IDs (`developer,computercontroller`) or repeated flag."
    example: |
      goose session --with-builtin developer --with-builtin computercontroller
  - flag: "goose session --with-extension / --with-streamable-http-extension / --with-builtin"
    description: |
      Same flags accepted by `goose run` for non-interactive one-shot
      invocations. Runtime extensions are not persisted.
    example: |
      goose run --with-extension "npx -y @modelcontextprotocol/server-github" -t "list my repos"
  - flag: "goose run --no-session"
    description: |
      Run a recipe in headless mode without creating or storing a session
      file. Useful for CI.
    example: "goose run --no-session -i instructions.txt"
  - flag: "goose mcp <name>"
    description: "Run an already-enabled MCP extension as a stdio MCP server by name (without the goose runtime)."
    example: "goose mcp github"
  - flag: "goose acp"
    description: "Run goose itself as an Agent Client Protocol (ACP) agent server over stdio for ACP-compatible clients (e.g. Zed)."
    example: "goose acp"
  - flag: "/extension <command>"
    description: "Add a stdio extension mid-session (slash command)."
    example: "/extension npx -y @modelcontextprotocol/server-memory"
  - flag: "/builtin <names>"
    description: "Enable built-in extensions mid-session (slash command, comma-separated)."
    example: "/builtin developer,computercontroller"
  - flag: "/prompts [--extension <name>]"
    description: "List MCP prompt templates, optionally filtered by extension."
    example: "/prompts --extension developer"
  - flag: "/prompt <n> [--info] [key=value...]"
    description: "Run an MCP prompt template by numeric index, optionally printing `--info` first, with optional `key=value` arguments."
    example: "/prompt 1 name=my-project"
  - flag: "/mode <auto|approve|smart_approve|chat>"
    description: "Change the permission mode mid-session."
    example: "/mode approve"
  - flag: "--debug (also /r)"
    description: "Show full tool parameters and responses without truncation; essential for MCP troubleshooting."
    example: "goose run --debug --with-extension ..."
  - flag: "--container <container_id>"
    description: "Run stdio MCP extensions inside a Docker container instead of the host."
    example: "goose session --container devcontainer-1 --with-extension ..."
env_vars:
  - name: GOOSE_PATH_ROOT
    effect: |
      Overrides the etcetera-resolved config / data / state directories. When
      set, goose creates `<root>/config`, `<root>/data`, `<root>/state`,
      `<root>/.agents/plugins`, and `<root>/.agents/agents`. Useful for
      isolated test / CI environments.
  - name: GOOSE_ADDITIONAL_CONFIG_FILES
    effect: |
      OS-path-separator-separated list of additional YAML config files to
      load. Loaded after `/etc/goose/config.yaml` (system) and before the
      user config so later layers take precedence.
  - name: GOOSE_ALLOWLIST
    effect: |
      URL to a YAML allowlist of permitted extension install commands.
      Fetched on demand, cached, and re-fetched on each goose restart. When
      set, only extensions whose install command matches an entry are
      installable.
  - name: GOOSE_MODE
    effect: |
      Default permission mode: `auto`, `approve`, `smart_approve`, or `chat`.
      Governs whether MCP tool calls require user approval. (Docs default
      value is `auto` in the permissions page; `smart_approve` is shown
      elsewhere.)
  - name: GOOSE_DISABLE_KEYRING
    effect: |
      Any non-empty value. Disables system keyring secret storage and forces
      plaintext `secrets.yaml`. Also auto-set if a keyring operation returns
      a "keyring unavailable" error.
  - name: GOOSE_SEARCH_PATHS
    effect: |
      JSON array of extra directories prepended to `PATH` when spawning
      stdio extension processes.
  - name: GOOSE_OAUTH_CALLBACK_PORT
    effect: |
      Pin the local OAuth callback port to a fixed value. Required by some
      identity providers for MCP server OAuth and Databricks OAuth.
  - name: GOOSE_MAX_TOOL_RESPONSE_SIZE
    effect: |
      Maximum character count for a single tool response before it is written
      to a temporary file (and referenced from the conversation) instead of
      being inlined.
  - name: GOOSE_SHELL
    effect: |
      Overrides the shell used by the Developer extension's `shell` tool and
      indirect stdio extension command invocations.
  - name: GOOSE_TOOLSHIM
    effect: |
      Enable tool-call interpretation; can change how MCP tool results are
      summarized or presented to the model.
  - name: GOOSE_FAST_MODEL
    effect: |
      Override the provider's default fast model used for auxiliary calls
      (tool selection, classification, session naming).
  - name: GOOSE_DISABLE_SESSION_NAMING
    effect: |
      Disable automatic AI-generated session naming; skips a background
      model call and keeps "CLI Session" / "New Chat".
  - name: GOOSE_DISABLE_TOOL_CALL_SUMMARY
    effect: |
      Disable the per-tool-call AI-generated summary title, saving one
      provider call per tool invocation.
  - name: GOOSE_CONTEXT_STRATEGY
    effect: |
      How goose handles context-limit-exceeded situations: `summarize`,
      `truncate`, `clear`, `prompt`. Default differs between interactive
      (`prompt`) and headless (`summarize`).
  - name: GOOSE_GATEWAY_MAX_TURNS
    effect: |
      Maximum turns for gateway sessions (e.g. Telegram). Defaults to 5
      and falls back to `GOOSE_MAX_TURNS`.
  - name: GOOSE_SUBAGENT_MAX_TURNS
    effect: |
      Maximum turns a subagent may run before timeout. Default 25,
      overridable per-recipe via `settings.max_turns`.
  - name: GOOSE_MAX_BACKGROUND_TASKS
    effect: |
      Maximum number of concurrent background subagent tasks. Default 5.
  - name: GOOSE_MAX_CODE_BLOCK_LINES / GOOSE_TRUNCATED_SHOW_LINES / GOOSE_NO_CODE_TRUNCATION
    effect: |
      Code block rendering in CLI output. Defaults 50 lines truncated to 20;
      `GOOSE_NO_CODE_TRUNCATION=1` disables truncation entirely.
  - name: GOOSE_AUTO_COMPACT_THRESHOLD
    effect: |
      Float in `[0, 1]` triggering automatic session compaction. Default 0.8;
      0 disables.
  - name: GOOSE_TOOL_CALL_CUTOFF
    effect: |
      Keep `N` most recent tool calls in full detail before summarizing
      older tool outputs. Default 10.
  - name: GOOSE_CLI_THEME / GOOSE_CLI_LIGHT_THEME / GOOSE_CLI_DARK_THEME / GOOSE_CLI_SHOW_THINKING / GOOSE_CLI_NEWLINE_KEY / GOOSE_CLI_SHOW_COST / GOOSE_CLI_TOOL_PARAMS_TRUNCATION_MAX_LENGTH / GOOSE_RANDOM_THINKING_MESSAGES / GOOSE_PROMPT_EDITOR
    effect: |
      CLI presentation env vars. `GOOSE_CLI_SHOW_THINKING` toggles reasoning
      output (DeepSeek-R1, Kimi, Gemini); `GOOSE_PROMPT_EDITOR=vim` (etc.)
      routes prompt input through an external editor.
  - name: GOOSE_MOIM_MESSAGE_TEXT / GOOSE_MOIM_MESSAGE_FILE
    effect: |
      Inject persistent text into goose's working memory every turn. The
      file variant is capped at 64 KB and supports `~/`. Used for behavioral
      guardrails or persistent reminders.
  - name: GOOSE_RECIPE_PATH / GOOSE_RECIPE_GITHUB_REPO / GOOSE_RECIPE_RETRY_TIMEOUT_SECONDS / GOOSE_RECIPE_ON_FAILURE_TIMEOUT_SECONDS
    effect: |
      Recipe discovery and scheduling configuration. `GOOSE_RECIPE_PATH`
      extends search directories. `GOOSE_RECIPE_GITHUB_REPO=org/repo`
      points at the GitHub recipe repository. Recipe success / failure
      timeouts come from these globals when not set per-recipe.
  - name: GOOSE_EDITOR_API_KEY / GOOSE_EDITOR_HOST / GOOSE_EDITOR_MODEL
    effect: |
      Enable the Developer extension's AI-powered `str_replace` enhanced
      code editor. All three must be set and non-empty.
  - name: GOOSE_HOST / GOOSE_PORT / GOOSE_TLS / GOOSE_SERVER__SECRET_KEY
    effect: |
      Configure the `goosed` server process. `GOOSE_HOST=0.0.0.0` makes
      it reachable from other machines; `GOOSE_TLS=true` enables TLS with a
      self-signed cert; `GOOSE_SERVER__SECRET_KEY` is the shared secret
      required in the `X-Secret-Key` header on every client request.
  - name: GOOSE_SANDBOX
    effect: |
      Enable the macOS-only sandbox for goose Desktop, using Apple's
      `sandbox-exec`. Restricts file / network / process access for the
      desktop app process.
  - name: HTTP_PROXY / HTTPS_PROXY / NO_PROXY
    effect: |
      Standard HTTP proxy env vars, honored by goose's HTTP client for
      remote MCP servers and Databricks OAuth.
  - name: SECURITY_PROMPT_ENABLED
    effect: |
      Enable pattern-based prompt injection detection on tool calls,
      including MCP tool calls (patterns live in
      `crates/goose/src/security/patterns.rs`).
  - name: SECURITY_PROMPT_THRESHOLD
    effect: |
      Sensitivity threshold (0.01-1.0) for prompt injection detection.
  - name: SECURITY_PROMPT_CLASSIFIER_ENABLED / SECURITY_PROMPT_CLASSIFIER_ENDPOINT / SECURITY_PROMPT_CLASSIFIER_TOKEN
    effect: |
      Enable ML-based prompt injection detection. Endpoint follows the
      Hugging Face Inference API schema. Token is sent in the
      Authorization header.
  - name: CONTEXT_FILE_NAMES
    effect: |
      JSON array of filenames to load as goose context / hint files.
      Default `[".goosehints"]`.
  - name: GOOSE_MCP_CLIENT_VERSION
    effect: |
      Read by Goose's MCP client when constructing `Implementation`
      identity; falls back to the binary's `CARGO_PKG_VERSION` when unset.
  - name: BEDROCK_* and DATABRICKS_* retry knobs
    effect: |
      `BEDROCK_MAX_RETRIES`, `BEDROCK_INITIAL_RETRY_INTERVAL_MS`,
      `BEDROCK_BACKOFF_MULTIPLIER`, `BEDROCK_MAX_RETRY_INTERVAL_MS`,
      and the equivalent `DATABRICKS_*` knobs control provider-level
      retry loops. Defaults differ per provider.
  - name: OTEL_EXPORTER_OTLP_ENDPOINT / OTEL_EXPORTER_OTLP_* / OTEL_SIGNAL_EXPORTER / OTEL_SERVICE_NAME
    effect: |
      OpenTelemetry signal export. `OTEL_SDK_DISABLED=true` disables
      export; per-signal exporters (`OTEL_TRACES_EXPORTER`,
      `OTEL_METRICS_EXPORTER`, `OTEL_LOGS_EXPORTER`) accept `otlp`,
      `console`, or `none`.
server_schema:
  transports: [stdio, streamable_http, builtin, platform, frontend, inline_python]
  command_fields: ["type", "name", "cmd", "args", "description", "envs", "env_keys", "timeout", "cwd", "bundled", "enabled", "available_tools"]
  http_fields: ["type", "name", "uri", "description", "envs", "env_keys", "headers", "timeout", "socket", "bundled", "enabled", "available_tools"]
  env_shape: |
    `envs` is a JSON object mapping variable names to string values; the
    parser filters out 31 disallowed keys (PATH, PATHEXT, SystemRoot,
    windir, LD_LIBRARY_PATH, LD_PRELOAD, LD_AUDIT, LD_DEBUG, LD_BIND_NOW,
    LD_ASSUME_KERNEL, DYLD_LIBRARY_PATH, DYLD_INSERT_LIBRARIES,
    DYLD_FRAMEWORK_PATH, PYTHONPATH, PYTHONHOME, NODE_OPTIONS, RUBYOPT,
    GEM_PATH, GEM_HOME, CLASSPATH, GO111MODULE, GOROOT, APPINIT_DLLS,
    SESSIONNAME, ComSpec, TEMP, TMP, LOCALAPPDATA, USERPROFILE, HOMEDRIVE,
    HOMEPATH). `env_keys` lists variables that should be resolved from
    the keyring / `secrets.yaml` / process environment at spawn time and
    substituted into `headers`, `uri`, `socket`, and `cwd` via
    `${VAR}`-style expansion. The full keyed set is then merged into the
    final env map.
  auth_shape: |
    Streamable HTTP supports arbitrary `headers: {name: value}` plus
    `${VAR}` substitution (so secrets stored in the keyring can be
    injected without writing them into `config.yaml`). Goose uses a local
    callback server on `GOOSE_OAUTH_CALLBACK_PORT` (default random) for
    OAuth-style auth; docs do not enumerate a provider-native OAuth
    client registry. Per-server OAuth callback URL form is
    `http://127.0.0.1:<port>/oauth_callback`. The Developer extension
    shell inherits the user's shell environment and adds
    `AGENT_SESSION_ID`, `GOOSE_TERMINAL=1`, `AGENT=goose`.
  notes: |
    The YAML type tag is required (`type: stdio` / `type: streamable_http`
    / `type: builtin` / `type: platform` / `type: frontend` / `type:
    inline_python`). The `sse` variant is parsed for compatibility but
    `is_tool_available` always returns false, so SSE extensions are
    effectively deprecated even if they still appear in some files. The
    `inline_python` variant carries a `code` field and is executed via
    `uvx`. The `platform` variant is in-process and exposes goose
    internals (Extension Manager, Apps, Chat Recall, Code Mode, Summon,
    Todo, Top of Mind). The `frontend` variant lets the desktop or web
    UI register tools directly. The `streamable_http` variant supports
    `socket: "@name"` (Linux abstract socket) or an absolute Unix-domain
    socket path; the HTTP request is routed through that socket while
    `uri` is used for the Host header and path.
server_capabilities:
  tools: full
  resources: partial
  prompts: partial
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: true
  notes: |
    Tools are fully exposed to the model and refreshed on `list_changed`
    (server notifications are plumbed via `subscribe()` to internal
    channels; per the source the call returns a tokio mpsc receiver but
    automatic re-query on `list_changed` is implementation-internal and
    not externally documented). Tool names appear to the model in the form
    `<extension>__<tool>` (e.g. `developer__shell`,
    `computercontroller__automation_script`). The Extension Manager
    extension adds `search_available_extensions` and `manage_extensions`
    so the runtime set can change mid-session. Resources are surfaced
    indirectly via `Extension Manager.list_resources` / `read_resource`
    tools — there is no native MCP resource picker, no
    `resources/subscribe`, no documented `resources/templates/list` and
    no native surface that listens for `resources/list_changed`. Prompts
    are surfaced through `/prompts` and `/prompt <n>` slash commands,
    backed by the standard `prompts/list` + `get_prompt` MCP requests;
    `prompts/list_changed` is honored by re-listing but no UI pushes
    updates.
client_capabilities:
  roots: full
  sampling: full
  elicitation: full
  notes: |
    Goose advertises roots, sampling, and elicitation on every MCP
    initialization. The capability builder chain is
    `enable_roots().enable_extensions_with(extensions).enable_sampling().enable_elicitation()`.
    Roots: a single root URI is exposed — the current session working
    directory as a `file://` URL named `working_directory`. Updates flow
    through `notify_roots_list_changed` whenever the session CWD changes.
    Sampling: server `create_message` requests are routed through the
    active goose LLM (`GOOSE_PROVIDER` + `GOOSE_MODEL`), support both
    text and image `SamplingMessageContent`, and return
    `stop_reason=STOP_REASON_END_TURN`. Elicitation: supports both
    `FormElicitationParams` (rendered as a form in the CLI/Desktop) and
    `UrlElicitationParams` (browser URL). Form-mode requests time out
    after 300 seconds (5 minutes) through `ActionRequiredManager`. There
    is no in-source idempotency gate that blocks sampling or elicitation
    by default — both are automatically enabled.
tool_surface:
  discovery: |
    `tools/list` is called at extension startup. The Extension Manager
    extension keeps the session working set bounded to ~5 extensions /
    50 tools (recommendation, not a hard cap) by enabling extensions
    on-demand and disabling unused ones. Tool schemas are loaded in full
    at startup (no tool-search deferral documented for goose).
  filtering: |
    Two layers of filtering:
      1. Per-extension `available_tools: [names]`. Empty means "all";
         otherwise only the listed tools load (see `is_tool_available`).
      2. Per-tool `<extension>__<tool>` keys in `permission.yaml` under
         the relevant permission category (`user`, `smart_approve`).
         Values are `always_allow`, `ask_before`, or `never_allow`.
         The agent auto-caches `read_only_hint == Some(false)` tool
         annotations into `smart_approve.ask_before`. The server
         allowlist (`GOOSE_ALLOWLIST`) blocks extension install before
         tools are even loaded. There is no documented managed
         allow/deny-mcpServers equivalent at the CLI / config level.
  approval: |
    MCP tools use the same `GOOSE_MODE` permission model as built-in
    tools. The granular override per tool sits on top:
      • `always_allow` — no prompt, even in `approve` mode.
      • `ask_before` — prompts on every call, even in `auto` mode.
      • `never_allow` — hard denial; LLM does not see the tool.
    In `smart_approve` mode LLM classification decides which tools
    fall under `ask_before` automatically. The Developer `shell` and
    `computercontroller__automation_script` tools are also subject to
    the optional Adversary reviewer if `~/.config/goose/adversary.md`
    exists. The Adversary reviewer is fail-open.
  result_handling: |
    Text, image, and resource-link results are passed to the model.
    Outputs above `GOOSE_MAX_TOOL_RESPONSE_SIZE` are persisted to a
    temp file and replaced by a file reference in the conversation
    (matching Claude Code's `MAX_MCP_OUTPUT_TOKENS` pattern, but for
    Goose the threshold is a single character count).
  annotations_trusted: |
    Tool annotations are read for two effects only: `read_only_hint ==
    Some(false)` triggers an automatic `AskBefore` cache in
    `smart_approve` mode, and "destructive tools" defined in
    `security/patterns.rs` are flagged for the prompt-injection
    detector. There is no documented `requiresUserInteraction`-style
    per-call gate driven by annotations — `Apply Before` is driven by
    `permission.yaml`.
  notes: |
    Tool naming uses `<extension>__<tool>`; built-in extension tools
    follow the same convention (e.g. `developer__shell`,
    `developer__text_editor`, `developer__analyze`,
    `developer__screen_capture`, `developer__image_processor`).
    Channel-capable MCP servers are not described — goose has no
    `claude mcp serve` analogue for an event push surface.
resource_surface:
  supported: true
  uri_schemes: ["file", "config", "cached", "memory", and any server-defined URI"]
  templates: false
  subscriptions: false
  exposure_model: |
    User / model selection happens through the Extension Manager
    extension's `list_resources` and `read_resource` tools. There is no
    native UI picker (no `@`-menu equivalent), no URI template support,
    and no subscription push. Scripts and the model can call
    `read_resource` directly. URI schemes are server-defined; goose
    does not advertise `resources/templates/list`.
  notes: |
    `resources/list_changed` is part of the protocol but no consumer
    listens for it; listing is on-demand.
prompt_surface:
  supported: true
  invocation: |
    `/prompts [--extension <name>]` lists available prompts;
    `/prompt <n> [--info] [key=value...]` runs a prompt by index with
    optional arguments. Both are session-time slash commands.
  arguments: |
    Trailing `key=value` tokens on `/prompt` are parsed by the prompt's
    declared arguments schema. `prompts/list_changed` refreshes the
    index shown by `/prompts`.
  exposure_model: |
    User-only via slash commands. The model does not autonomously invoke
    prompts; it can however call `prompts/get` directly.
  notes: |
    Implementation uses standard MCP `ListPromptsRequest` /
    `GetPromptRequest`; slash-command styling is goose-specific
    presentation on top.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: deep
  notes: |
    Claudine can treat Goose as `import_sync`. The user-managed files
    are `config.yaml` (extensions + settings), `permission.yaml`
    (per-tool permissions), `secrets.yaml` (when the keyring is
    disabled), and `adversary.md` (optional). On disk precedence is
    `[system → additional → user]` for config, with environment
    variables winning on a per-key basis. `merge_config_values` is
    *deep* for the `extensions:` and `providers:` sub-trees — extension
    entries merge field-by-field when both layers define the same
    name — but for all other top-level keys later entries fully
    replace earlier ones. Secrets are layered separately: env → keyring
    → `secrets.yaml` fallback file. Goose has no dedicated non-interactive
    CLI subcommand for adding / listing / removing MCP servers outside
    the `goose configure` TUI, so `apply_supported` is `false` and
    Claudine must edit `config.yaml` directly with care to preserve
    field-level merge semantics.
runtime_injection:
  supported: true
  mechanism: |
    Pass `--with-extension <cmd>`, `--with-streamable-http-extension
    <url>`, or `--with-builtin <id>` (each repeatable) to `goose
    session` or `goose run` to enable MCP extensions for the current
    session without mutating `config.yaml`. Built-in ids are comma-
    separated (`developer,computercontroller`) when given as a single
    arg. Sessions also accept `/extension <cmd>` and `/builtin
    <names>` mid-flight. The `--container <id>` flag routes the stdio
    subprocesses into a Docker container.
  limitations: |
    Runtime extensions are not persisted and do not participate in the
    `config.yaml` merge; Claudine must build the desired flags itself
    (including any per-extension `envs` value as inline
    `KEY=VALUE <command>` prefixes on `--with-extension`). Non-
    interactive `goose run` cannot complete OAuth interactive flows;
    pre-authenticated servers or env-resolved secrets are required.
    `GOOSE_ALLOWLIST` is consulted at install time, but runtime
    injection does not invoke "install" so the allowlist does *not*
    block `--with-extension`. There is no `--mcp-config <file>` analogue
    that loads an external JSON / YAML server set; the runtime
    injection is entirely per-flag.
authorization:
  oauth: true
  credential_storage: |
    Secrets are stored in the system keyring under service `goose`,
    username `secrets`, as a single JSON object containing every key.
    When the keyring is unavailable (headless server, CI, container,
    or `GOOSE_DISABLE_KEYRING` is set) or a keyring operation returns
    one of `No entry found`, `No matching entry found`, or a
    `Keyring unavailable`-style error, goose automatically falls back
    to `<config_dir>/secrets.yaml` (mode `0600` on Unix). The secrets
    file is YAML. Some platforms (Databricks) require a fixed
    callback port (`GOOSE_OAUTH_CALLBACK_PORT`).
  token_scope: unknown
  stdio_secret_delivery: |
    Per-extension `envs` map plus inherited user environment plus
    goose-set variables (`AGENT_SESSION_ID`, `GOOSE_TERMINAL=1`,
    `AGENT=goose`). The 31 disallowed env vars are filtered out
    automatically (`PATH`, `LD_PRELOAD`, `DYLD_INSERT_LIBRARIES`,
    `NODE_OPTIONS`, etc.). `env_keys` listed in `config.yaml` are
    resolved from the keyring / `secrets.yaml` / process env and
    substituted into `headers`, `uri`, `socket`, and `cwd` via
    `${VAR}` expansion *before* the extension is started, so secret
    values never live as plaintext in `config.yaml`.
  notes: |
    Token refresh and audience scoping per remote server are not
    documented; OAuth support assumes interactive auth using a local
    callback server. Static `headers.Authorization` is the documented
    escape hatch.
security:
  tool_filtering: |
    Three filters compose in this order:
      1. Server allowlist (`GOOSE_ALLOWLIST`) — extension install is
         matched against the YAML by exact command; runtime-injected
         servers bypass this check.
      2. Per-extension `available_tools` — tool-level allowlist applied
         before the LLM sees the tool.
      3. Per-tool `permission.yaml` — overrides the mode-level default
         on a per-`<extension>__<tool>` basis.
    The system auto-caches `read_only_hint == Some(false)` tool
    annotations into `smart_approve.ask_before`. There is no managed
    allow/deny policy file equivalent to `~/.claude/settings.json`;
    allowlists are limited to the `GOOSE_ALLOWLIST` URL.
  server_trust: |
    `goose` itself checks external extensions for known malware via a
    blocklist before activation. There is no documented project-trust
    gate because there is no repo-level MCP config file in goose.
    Built-in platform extensions (`Platform` variant) run in-process
    and have direct access to the agent — they are *not* isolated by
    the macOS sandbox.
  env_sanitization: |
    Stdio extensions receive the explicit `envs` map (with 31
    disallowed keys filtered) plus inherited process environment,
    plus goose-set variables (`AGENT_SESSION_ID`, `GOOSE_TERMINAL=1`,
    `AGENT=goose`). There is no `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`
    analogue that strips provider credentials from subprocess env.
    A `GOOSE_DISABLED_*` style scrub is **not** documented. The
    Developer extension's `shell` tool inherits the same env.
  sandbox_interaction: |
    Stdio MCP servers run as ordinary local processes. Goose Desktop
    supports an optional macOS sandbox via `GOOSE_SANDBOX` (Apple
    `sandbox-exec`) but it does *not* describe stdio extension
    subprocesses as running inside that sandbox — sandbox scope is
    unclear and not verified in this research. The `--container` flag
    runs stdio extensions inside a Docker container when set;
    isolation otherwise comes from the host environment.
  response_filtering: |
    SECURITY_PROMPT_ENABLED does pattern + optional ML classifier
    detection on tool call *parameters before execution* (not on tool
    results). Adversary mode is an LLM-based review pass on tool
    calls before execution; it returns ALLOW / BLOCK, fail-open. There
    is no documented scanning of MCP tool *result/output* content for
    prompt-injection patterns.
  notes: |
    Note: `secret` keys stored in the keyring cannot be safely copied
    across machines; secrets.yaml is plaintext. Administrators should
    deploy `GOOSE_ALLOWLIST` and (optionally) a managed `GOOSE_MODE=approve`
    baseline to enforce an organization-wide extension surface.
gaps:
  - |
    The public configuration-files docs page lists
    `~/.config/goose/config.yaml` as the macOS/Linux user path,
    contradicting the source code (etcetera `Block` author → macOS uses
    `~/Library/Application Support/Block/goose/`, Linux uses
    `~/.config/Block/goose/`, Windows uses `%APPDATA%\Block\goose\`).
    Treat the source as authoritative.
  - |
    The docs page lists Windows user config as
    `%APPDATA%\Block\goose\config\config.yaml`; the source uses
    `%APPDATA%\Block\goose\config.yaml` (no `config/` subdirectory).
  - |
    The MCP protocol version on the wire is `V_2025_03_26` per the
    rmcp crate constant; the docs cite `2025-06-18` for roots and
    the draft spec for sampling / elicitation. Claudine should treat
    goose as `2025-03-26`-era from a protocol-feature perspective.
  - |
    `resources/templates/list`, `resources/subscribe`, and
    `resources/list_changed` notification handling are not exposed in
    any documented UI surface; clients may opt-in via direct protocol
    calls but no goose UX pushes updates.
  - |
    OAuth token scope, refresh, and storage semantics for remote
    Streamable-HTTP MCP servers are not fully documented past "uses a
    local callback server".
  - |
    There is no `goose mcp add` / `goose mcp remove` CLI pair — only
    `goose configure` is interactive and `goose mcp <name>` re-runs
    an already-enabled extension. Programmatic apply must edit
    `config.yaml` directly.
  - |
    There is no `--mcp-config <file-or-json>` analogue equivalent to
    Claude Code's flag, so runtime injection must be done flag-by-flag
    (`--with-extension`, `--with-streamable-http-extension`,
    `--with-builtin`).
  - |
    `GOOSE_SANDBOX` scope is described only for goose Desktop; whether
    macOS `sandbox-exec` covers the *stdio MCP extension* subprocesses
    is not verified in source.
  - |
    Goose is not installed on this host (`~/Library/Application Support/Block/goose/` and
    `~/.config/goose/` do not exist; `which goose` reports not found).
    All shape and merge findings are from official docs and source
    code at `aaif-goose/goose`, not from on-host inspection.
changes:
  - "2026-07-03 — Corrected the user-config paths. The docs say `~/.config/goose/config.yaml` on macOS/Linux, but the source uses etcetera `Block` author — macOS `~/Library/Application Support/Block/goose/config.yaml`, Linux `~/.config/Block/goose/config.yaml`, Windows `%APPDATA%\\Block\\goose\\config.yaml` (also fixed the Windows `\\config\\config.yaml` extra subdir bug)."
  - "2026-07-03 — Confirmed the protocol version advertised on the wire is `2025-03-26` (`ProtocolVersion::V_2025_03_26` in `crates/goose/src/agents/mcp_client.rs`); the doc references to 2025-06-18 roots and draft sampling/elicitation are external spec links, not the rmcp constant."
  - "2026-07-03 — Documented `Sse` as deserialized for compatibility only; `is_tool_available` always returns `false` for it (`SSE is unsupported`). Effective removal of legacy SSE even though the enum variant survives."
  - "2026-07-03 — Added `Platform`, `Frontend`, and `InlinePython` as first-class `ExtensionConfig` variants. The `streamable_http` variant gained an optional `socket` field for HTTP-over-UDS transport (`@name` for Linux abstract sockets, otherwise an absolute Unix-domain socket path)."
  - "2026-07-03 — Added `Envs::DISALLOWED_KEYS` (31 hard-coded var names) that get silently filtered from `envs`; `${VAR}` substitution resolves into `headers` / `uri` / `socket` / `cwd` from keyring or process env."
  - "2026-07-03 — Changed merge semantics claim from `shallow` to `deep` for the `extensions:` and `providers:` sub-trees in `config.yaml` (field-level merge when the same extension id appears at multiple paths)."
  - "2026-07-03 — Added system-wide config path `/etc/goose/config.yaml` (Unix) / `%PROGRAMDATA%\\goose\\config.yaml` (Windows) and the new env var `GOOSE_ADDITIONAL_CONFIG_FILES` for layered config files."
  - "2026-07-03 — Added new env vars observed in current docs/source: `GOOSE_FAST_MODEL`, `GOOSE_GATEWAY_MAX_TURNS`, `GOOSE_SUBAGENT_MAX_TURNS`, `GOOSE_MAX_BACKGROUND_TASKS`, `GOOSE_AUTO_COMPACT_THRESHOLD`, `GOOSE_TOOL_CALL_CUTOFF`, `GOOSE_DISABLE_SESSION_NAMING`, `GOOSE_DISABLE_TOOL_CALL_SUMMARY`, `GOOSE_CONTEXT_STRATEGY`, `GOOSE_CLI_SHOW_THINKING`, `GOOSE_RECIPE_PATH`, `GOOSE_RECIPE_GITHUB_REPO`, `GOOSE_EDITOR_*`, `GOOSE_MOIM_MESSAGE_*`, `GOOSE_HOST/PORT/TLS/SERVER__SECRET_KEY`, `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY`, `OTEL_*`, `LANGFUSE_*`, `CONTEXT_FILE_NAMES`, `BEDROCK_*`/`DATABRICKS_*`, `GOOSE_MCP_CLIENT_VERSION`."
  - "2026-07-03 — Documented `goose acp`, `goose plugin install/update`, `goose schedule`, `goose recipe (deeplink/list/open/validate)`, and `goose session diagnostics` subcommands; `goose mcp <name>` re-runs an enabled extension without the goose runtime."
  - "2026-07-03 — Documented that `smart_approve` permission cache is auto-populated from tool annotations: `read_only_hint == Some(false)` → `smart_approve.ask_before`."
  - "2026-07-03 — Documented Adversary Mode location (`<config_dir>/adversary.md`), default reviewed tools (`shell`, `computercontroller__automation_script`), and fail-open behavior."
  - "2026-07-03 — Documented MCP Elicitation 5-minute timeout (`Duration::from_secs(300)`) and that URL mode is supported alongside form mode."
  - "2026-07-03 — Documented MCP Sampling routes through the active goose LLM (text + image content, `stop_reason = STOP_REASON_END_TURN`) — i.e. servers share the user's main provider for sampling."
  - "2026-07-03 — Noted goose has moved from `block/goose` to `aaif-goose/goose` (AAIF / Linux Foundation) as of April 2026; docs site redirects to `goose-docs.ai`; current version v1.41.0 (Jul 3 2026)."
requires_claudine_update: true
reason: |
  Claudine's skill and provider metadata currently classify Goose as
  "no MCP support yet" (per the project overview), but the underlying
  reality is `import_sync`: persistent `config.yaml` / `permission.yaml`
  files plus runtime injection via `--with-extension`,
  `--with-streamable-http-extension`, `--with-builtin`, plus built-in
  platform extensions in-process. Claudine needs:
    (a) provider metadata that flags Goose as `import_sync` with
        `apply_supported: false` and points to etcetera's actual paths
        rather than `~/.config/goose/`,
    (b) sync logic that performs deep field-level merges on the
        `extensions:` and `providers:` sub-trees while shallow-replacing
        top-level keys,
    (c) runtime injection that emits per-flag injection rather than
        the `--mcp-config <file>` pattern,
    (d) secrets handling that prefers the keyring but falls back to
        `<config_dir>/secrets.yaml`,
    (e) recognition of the new env vars listed above (`GOOSE_*`),
    (f) capability flags for Platform / Frontend / InlinePython
        extension variants and the Streamable-HTTP `socket` knob,
    (g) leaves MCP Roots, Sampling, and Elicitation as `full` (matches
        the prior research), but tightens the protocol version to
        2025-03-26.
---

# MCP Support in Goose

## Overview

[Goose](https://goose-docs.ai/) (now maintained by the Agentic AI
Foundation at the Linux Foundation after the April-2026 move from
`block/goose` to [`aaif-goose/goose`](https://github.com/aaif-goose/goose))
is a Rust-built general-purpose AI agent with a desktop app, CLI, server
(`goosed`), and ACP bridge. It was one of the earliest MCP adopters and
ships 70+ documented MCP extensions plus seven platform extensions
(`extensionmanager`, `apps`, `chatrecall`, `codemode`, `summon`, `todo`,
`tom`) that run in-process. Extensions provide tools, resources, and
prompts using a YAML config plus stdio or Streamable HTTP transports.
This document maps Goose's MCP behavior to Claudine's MCP catalog,
import/export, runtime injection, and provider security posture.

For Claudine, Goose is an `import_sync` target with `apply_supported:
false`. Persistent YAML files (`config.yaml`, `permission.yaml`, optional
`secrets.yaml`, optional `adversary.md`) can be read and normalized into
the catalog and written back, but there is no Claude-Code-style
`goose mcp add` CLI — only the interactive `goose configure` TUI. One-run
injection is well-supported through `--with-extension`,
`--with-streamable-http-extension`, and `--with-builtin` on `goose
session` and `goose run`.

Surface inventory (one-line):

- **Tools** — exposed: tool names take the form `<extension>__<tool>`;
  per-extension `available_tools` whitelist plus `permission.yaml` per-tool
  override on top of `GOOSE_MODE`.
- **Resources** — exposed partially: surfaced only through the
  Extension Manager's `list_resources` / `read_resource` tools, no native
  picker, no `resources/subscribe` push, no `resources/templates/list`.
- **Prompts** — exposed partially: surfaced through slash commands
  `/prompts [--extension]` and `/prompt <n> [key=value...]`; the LLM can
  call `prompts/get` directly.
- **Roots** — exposed: single root = current session `cwd` as `file://`
  with the name `working_directory`; updated on session-cwd changes via
  `notifications/roots/list_changed`.
- **Sampling** — exposed: server `create_message` requests route through
  goose's active LLM (text + image); no per-server policy gate by default.
- **Elicitation** — exposed: both form-mode (CLI/Desktop form) and
  URL-mode dialogs; 5-minute timeout (300 s) before cancel.

## Protocol and Transports

Goose speaks MCP over two documented transports plus four in-process /
internal variants:

| Transport | Documented | Status | How it is added |
| :-------- | :--------- | :----- | :-------------- |
| `stdio`   | Yes | Primary | `goose configure` → Command-Line Extension, or `--with-extension` |
| `streamable_http` (`streamable-http`) | Yes | Primary | `goose configure` → Remote Extension, or `--with-streamable-http-extension` |
| `sse`     | Deprecated compatibility only | Deserialized in `config.yaml` for backwards compatibility but `is_tool_available` always returns `false`; not addressable from CLI/deeplink surfaces |
| `builtin` | Yes | Internal | Bundled MCP servers shipped in-process with goose |
| `platform` | Yes | Internal | Platform extensions that share the agent process (`extensionmanager`, `apps`, `chatrecall`, `codemode`, `summon`, `todo`, `tom`) |
| `frontend` | Yes | Internal | Tools provided by the desktop / web UI shell |
| `inline_python` | Yes | Internal | Inline Python executed via `uvx`, with optional `dependencies` |

- The MCP client in `crates/goose/src/agents/mcp_client.rs` advertises
  `ProtocolVersion::V_2025_03_26` on the wire (the rmcp crate constant
  for `2025-03-26`).
- The [MCP Roots guide](https://goose-docs.ai/docs/guides/mcp-roots)
  references the [2025-06-18 client/roots
  spec](https://modelcontextprotocol.io/specification/2025-06-18/client/roots);
  [MCP Sampling](https://goose-docs.ai/docs/guides/mcp-sampling) and
  [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation)
  link to the draft spec. Treat the wire spec as `2025-03-26`-era.
- The `streamable_http` variant supports a non-standard `socket` field
  for HTTP-over-UDS transport (`@name` for Linux abstract sockets,
  otherwise an absolute Unix-domain socket path). When set, the HTTP
  request is routed through that socket while `uri` provides the Host
  header and path.
- Legacy SSE / HTTP+SSE transport is no longer documented in the public
  guides; the `sse` enum variant survives in `ExtensionConfig` only for
  backwards compatibility when reading older `config.yaml` files.

Lifecycle behavior is described only at a high level: stdio extensions
are spawned as local subprocesses with `cmd` + `args` + resolved `envs`
(and the security-filtered env map) at session start; Streamable HTTP
extensions connect to a remote URL over the rmcp transport. Per-request
cancellation is supported via MCP `notifications/cancelled` (the client
sends it on timeout or `Cancel`); mid-session reconnect / retry semantics
for crash or transient connection failures are not exposed in the user-
facing API. Capability refresh is handled through the standard MCP
`list_changed` notification path; the client subscribes to server
notifications via `subscribe()` but documented behavior on
`tools/list_changed` / `prompts/list_changed` is implementation-internal.

## Configuration

Goose uses YAML configuration files for persistent settings. The base
directory comes from `etcetera::choose_app_strategy` with
`top_level_domain="Block"`, `author="Block"`, `app_name="goose"`. The
docs page at `/docs/guides/config-files` quotes an older path layout
(`~/.config/goose/config.yaml`) that does not match the source — the
etcetera-derived paths are the truth.

### Primary user config file

| OS | Path |
| :- | :--- |
| macOS | `~/Library/Application Support/Block/goose/config.yaml` |
| Linux | `~/.config/Block/goose/config.yaml` |
| Windows | `%APPDATA%\Block\goose\config.yaml` |

### System and overlay paths

| Path (purpose) | Notes |
| :--- | :--- |
| `/etc/goose/config.yaml` (Unix) / `%PROGRAMDATA%\goose\config.yaml` (Windows) | System-wide `config.yaml`. Loaded first; merged into a `Mapping` along with each additional layer. |
| `GOOSE_ADDITIONAL_CONFIG_FILES` (env var) | OS-path-separator list of additional YAML files. Loaded after system, before user. |
| `<config_dir>/config.yaml` | User config. Loaded last among files; environment variables still beat it. |

The `extensions` key in `config.yaml` defines MCP servers:

```yaml
extensions:
  github:
    name: GitHub
    cmd: npx
    args: [-y, @modelcontextprotocol/server-github]
    enabled: true
    envs:
      GITHUB_PERSONAL_ACCESS_TOKEN: "<token>"
    type: stdio
    timeout: 300
```

### Related files in `<config_dir>`

- `permission.yaml` — tool permission levels configured through
  `goose configure` → Tool Permission. Schema:

  ```yaml
  user:
    always_allow: [developer__analyze]
    ask_before: [developer__shell]
    never_allow: []
  smart_approve:
    ask_before: [<auto-cached write tools>]
  ```

- `secrets.yaml` — file-based fallback for API keys and secrets (when
  the keyring is unavailable, or `GOOSE_DISABLE_KEYRING` is set).
  Stored as a single YAML map.
- `permissions/tool_permissions.json` — runtime permission decisions
  (auto-managed).
- `adversary.md` — optional Adversary Mode rules file (YAML frontmatter
  + Markdown). Presence turns Adversary Mode on; absence turns it off.

### Precedence

Configuration values are loaded in this order (later wins on disk):

1. System: `/etc/goose/config.yaml` (Unix) /
   `%PROGRAMDATA%\goose\config.yaml` (Windows)
2. Additional files from `GOOSE_ADDITIONAL_CONFIG_FILES`
3. User: `<config_dir>/config.yaml`
4. Environment variables (highest priority; per-key upper-case match)

Two sub-trees are merged *deeply* (`merge_nested_entries`): the
`extensions:` map and the `providers:` map. For all other top-level
keys, later sources fully replace earlier ones. Secrets are layered
separately: env → keyring → `secrets.yaml` fallback file.

There is no documented repo-scoped MCP config file.

### Reload behavior

Direct edits to `config.yaml` usually require a `goose` restart to take
effect for an existing session. `goose info -v` reports the active
settings and resolved paths; `goose2` provider credentials (per Goose
Desktop) refresh provider inventory without an app restart.

## Server Definition Shape

A single extension under `extensions` in `config.yaml` matches one of
these `ExtensionConfig` variants (Rust enum, tagged on `type`):

### Common fields

| Field | Required | Description |
| :---- | :------- | :---------- |
| `type` | yes | One of `stdio`, `streamable_http`, `builtin`, `platform`, `frontend`, `inline_python`. The `sse` variant is parse-only, never active. |
| `name` | yes | Internal name; primary identifier. |
| `description` | no (default `""`) | Description shown in UI. |
| `bundled` | no | Whether the extension ships with goose. |
| `enabled` | not part of extension map | Provided at use-site only; not persisted on every entry shape. |
| `timeout` | no | Per-extension tool call timeout (seconds). Defaults to `DEFAULT_EXTENSION_TIMEOUT`. |
| `available_tools` | no (`[]` = all) | Whitelist of tool names; `is_tool_available` filters against this. |

### `type: stdio`

| Field | Type | Description |
| :---- | :--- | :---------- |
| `cmd` | string | Executable to spawn. |
| `args` | array of strings | Process arguments. |
| `envs` | object (`HashMap<String,String>`) | Process environment overlay. 31 disallowed keys (`PATH`, `LD_PRELOAD`, `DYLD_INSERT_LIBRARIES`, `NODE_OPTIONS`, etc.) are filtered before the process starts. |
| `env_keys` | array of strings | Variable names resolved from keyring / secrets.yaml / process env and merged into `envs` *and* substituted via `${VAR}` into the streamable-http fields below when present. |
| `cwd` | string | Working directory of the stdio subprocess; supports `${VAR}`. |

### `type: streamable_http`

| Field | Type | Description |
| :---- | :--- | :---------- |
| `uri` | string | Streamable-HTTP endpoint URL; supports `${VAR}`. |
| `envs` | object | Unused at runtime beyond auth lookups; subject to `${VAR}` substitution. |
| `env_keys` | array | Names to resolve and substitute into `headers` / `uri` / `socket`. |
| `headers` | object (`HashMap`) | Static HTTP headers; each value supports `${VAR}` substitution (so secrets stored in the keyring or `secrets.yaml` flow into headers). |
| `socket` | string (optional) | Unix-domain socket path for HTTP-over-UDS transport, or `@name` for Linux abstract sockets. |
| `timeout` | number | Per-call timeout (seconds). |

### `type: builtin` / `type: platform`

Built-in extensions additionally carry an optional `display_name`
(`platform` always does); built-ins declared in goose itself include
`developer`, `computercontroller`, `memory`, `tutorial`,
`autovisualiser`, `extensionmanager`, `apps`, `chatrecall`, `codemode`,
`summon`, `todo`, `tom`.

### `type: frontend`

```yaml
type: frontend
name: <server_id>
description: <text>
tools: <inline MCP Tool[]>
instructions: <text>
bundled: <true|false>
available_tools: <list>
```

### `type: inline_python`

```yaml
type: inline_python
name: <id>
description: <text>
code: |
  # python code goes here
  print("hi from the extension")
dependencies: ["requests", "rich"]   # optional; pip-installed via uvx
timeout: 300
available_tools: <list>
```

## Tools, Resources, and Prompts

### Tools

Goose exposes MCP tools to the model. Tool names are namespaced by
extension (e.g. `developer__shell`, `github__list_repos`,
`computercontroller__automation_script`).

Tool discovery:

- `tools/list` is called at extension startup.
- The Extension Manager extension can search for and enable additional
  extensions mid-session (`search_available_extensions`,
  `manage_extensions`).
- The recommended ceiling is 5 active extensions / 50 total tools; the
  Extension Manager helps stay below it by enabling task-specific
  extensions only when needed.

Tool filtering (three layers, in this order):

1. **Server allowlist** (`GOOSE_ALLOWLIST`): extension *install* is
   rejected if the proposed command does not match a YAML entry; does
   not apply to `--with-extension` runtime injection.
2. **`available_tools`**: per-extension allowlist applied before the
   LLM sees the tool.
3. **`permission.yaml`**: per-tool `<extension>__<tool>` override under
   `user` and `smart_approve` permission categories with values
   `always_allow`, `ask_before`, `never_allow`. The system
   auto-caches `read_only_hint == Some(false)` tool annotations into
   `smart_approve.ask_before`.

### Resources

Goose does not expose a native MCP resource picker. Resources are
surfaced indirectly through the Extension Manager's `list_resources` and
`read_resource` tools (the latter is documented as only present when at
least one enabled extension supports resources). A user or the model can
ask Goose to read a resource by URI, but there is no URI template
support, no `resources/subscribe`, and no documented
`resources/list_changed` consumer. URI schemes are server-defined and
typically include `file:`, `config:`, `cached:`, `memory:`, and any
custom scheme the server advertises.

### Prompts

MCP prompts are exposed through slash commands in interactive sessions:

- `/prompts [--extension <name>]` — list available prompts, optionally
  filtered by extension.
- `/prompt <n> [--info] [key=value...]` — execute a prompt by numeric
  index with arguments.

Prompts are user-selected only; the LLM can call `prompts/get` directly
but the slash command is the human-facing trigger. `prompts/list_changed`
refreshes `/prompts` output. The LLM sees a single-string prompt
injection rather than a structured prompt result schema.

## Roots, Sampling, and Elicitation

### Roots

Goose advertises roots support during MCP initialization
(`enable_roots()` on `ClientCapabilities`). The root list contains a
single entry: the current session working directory, expressed as a
`file://` URL with the name `working_directory`. When the session CWD
changes (mid-session `cd`, resume in a new directory, manual switch in
Desktop), Goose updates the root and calls `notify_roots_list_changed`
on every connected extension.

### Sampling

[MCP Sampling](https://goose-docs.ai/docs/guides/mcp-sampling) is
automatically enabled. MCP servers can ask Goose's configured LLM
(`GOOSE_PROVIDER` + `GOOSE_MODEL`) for completions, including multimodal
requests (`text` and `image` `SamplingMessageContent`). The response
returns `stop_reason: STOP_REASON_END_TURN` and exposes
`model` (the resolved model name) to the server. There is no
per-server policy gate; sampling is automatic.

### Elicitation

[MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation) is
automatically enabled and supports both modes:

- **Form mode** — Goose renders a form in the CLI/Desktop.
- **URL mode** — Goose opens a browser URL for OAuth / external flows.

Form-mode requests time out after 300 seconds (5 minutes); the
`ActionRequiredManager` cancels the request and surfaces the cancellation
to the server. The `Elicitation` hook on the desktop/web hosts can
auto-respond programmatically; the action decisions `accept`, `decline`,
and `cancel` are mapped 1:1 from `ElicitationOutcome`.

## Import, Export, and Sync

Claudine can treat Goose as an `import_sync` provider with
`apply_supported: false`:

- **Import**: read `<config_dir>/config.yaml` (system + additional +
  user layers) and normalize the `extensions` map into the MCP catalog.
  Optional side files: `permission.yaml`, `adversary.md`,
  `secrets.yaml` (only when the keyring is disabled).
- **Export**: write provider-shaped YAML back to `config.yaml` and
  optionally `permission.yaml` / `adversary.md`. The YAML `extensions:`
  shape is reconstructed; field-level deep merge semantics must be
  respected or installed extension configs may regress.
- **Apply**: Goose has no dedicated non-interactive CLI for
  add/list/remove/import/export servers; the only supported mutation
  path outside file editing is the interactive `goose configure` TUI.
  Do **not** try to discover a `goose mcp add` command — it does not
  exist. Use file edits or run `goose configure` in a terminal where
  human input is possible.

Merge semantics:

- File-level config layers are merged into a single `Mapping`, with
  *deep* merging of the `extensions:` and `providers:` sub-trees
  (`merge_nested_entries` in `crates/goose/src/config/base.rs`) —
  when the same extension name appears at multiple paths, fields are
  merged by key.
- All other top-level keys are shallow-replaced (later source wins).
- Environment variables win over every file for a given upper-case
  config key.
- Secrets are layered env → keyring → `secrets.yaml` fallback.

## Runtime Injection

For one-run injection without mutating `config.yaml`, Goose provides
per-flag injection on both interactive and non-interactive entry points:

- `--with-extension <cmd>` — add a stdio extension for the session /
  run. Repeated flag. Commands may include inline `KEY=VALUE` prefixes
  for env vars (`VAR=value npx -y …`).
- `--with-streamable-http-extension <url>` — add a remote Streamable
  HTTP extension. Repeated flag.
- `--with-builtin <id>` — enable a built-in extension.
  Comma-separated ids (`developer,computercontroller`) or repeated flag.

```bash
goose run \
  --with-extension "GITHUB_PERSONAL_ACCESS_TOKEN=$TOKEN npx -y @modelcontextprotocol/server-github" \
  -t "list my repositories"
```

Mid-session slash equivalents:

- `/extension <command>` — add a stdio extension in-flight.
- `/builtin <names>` — enable built-in extensions in-flight.

Container variant:

- `--container <id>` runs the stdio extension subprocesses inside a
  Docker container instead of the host.

Limitations:

- Runtime extensions are **not persisted**; `--with-extension` does
  not touch `config.yaml`.
- They do not participate in the `merge_config_values` file merge;
  Claudine must build the desired flags itself (including per-extension
  `envs` values as inline shell-key prefixes).
- Non-interactive `goose run` cannot complete OAuth interactive flows;
  pre-authenticated servers or env-resolved secrets are required.
- There is no `--mcp-config <file>` equivalent: a single command line
  flag that loads a complete server set as a JSON / YAML blob is not
  available. Build the flags yourself or stage a `config.yaml` and use
  a custom `GOOSE_PATH_ROOT`.
- The `GOOSE_ALLOWLIST` is checked at install time, not at runtime-
  injection time, so `--with-extension` bypasses the install gate.

## Authorization and Credentials

Goose supports multiple credential patterns for extensions:

| Pattern | Where configured | Storage |
| :------ | :--------------- | :------ |
| Stdio env var | `envs` map in `config.yaml` | In config or inherited env |
| Stdio env var | `env_keys` referencing a keyring / `secrets.yaml` key | System keyring, then `secrets.yaml` fallback |
| Streamable HTTP header (static) | `headers: {Name: value}` in `config.yaml` | Config or substituted from keyring via `env_keys` |
| Streamable HTTP header (dynamic) | Same `headers` field, with `${VAR}` substitution at spawn time | ${VAR} resolves against `env_keys` |
| Streamable HTTP OAuth | `GOOSE_OAUTH_CALLBACK_PORT` (host); per-server auth completion is interactive | Token cache location not documented |

For stdio extensions, secrets should be passed through the `envs`
object or referenced indirectly via `env_keys`. Inline shell
`KEY=value` prefixes on `--with-extension` accept env from the calling
shell.

Per-server OAuth callback URL form is
`http://127.0.0.1:<port>/oauth_callback`. The callback port is
negotiated by default but can be pinned with `GOOSE_OAUTH_CALLBACK_PORT`.

## Security Model

### Trust and allowlisting

- Goose itself checks external extensions for known malware before
  activation.
- `GOOSE_ALLOWLIST` restricts which extension install commands are
  permitted. Runtime-injected extensions bypass this.
- There is no documented project-trust gate because there is no repo-
  level MCP config file.
- Built-in platform extensions run in the goose process and have direct
  access to the agent internals; treat them as part of the trusted
  computing base, not as user-extensible MCP servers.

### Tool filtering and permissions

- Per-extension `available_tools` and per-tool `permission.yaml`
  (configured via `goose configure`) provide two layers of filtering.
- `GOOSE_MODE` (`auto` / `approve` / `smart_approve` / `chat`) governs
  the default approval flow.
- `smart_approve` mode additionally consults the optional Adversary
  reviewer (`<config_dir>/adversary.md`); the reviewer uses the same
  provider/model as the main agent (no separate creds) and is fail-open.

### Environment and sandboxing

- Stdio MCP servers inherit the user's process environment plus their
  explicit `envs` map (with 31 disallowed keys filtered automatically).
  Goose-set variables `AGENT_SESSION_ID`, `GOOSE_TERMINAL=1`, `AGENT=goose`
  are added.
- There is **no** `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` analog: secrets
  that exist in the parent env are visible to MCP subprocesses by
  default. Treat the parent env as authoritative for secret hygiene.
- Goose Desktop supports an optional macOS sandbox via `GOOSE_SANDBOX`
  using Apple's `sandbox-exec`. The macOS sandbox's scope over the
  stdio MCP extension subprocesses is not documented as a guarantee.

### Prompt-injection scanning

- Pattern-based detection (`SECURITY_PROMPT_ENABLED` /
  `SECURITY_PROMPT_THRESHOLD`) intercepts every tool call *before
  execution* and matches against the patterns in
  `crates/goose/src/security/patterns.rs` (recognition covers
  filesystem destruction, remote code execution, data exfiltration,
  network access, process manipulation, privilege escalation, command
  injection, encoded commands, container escape, kernel module
  manipulation, etc.).
- ML-based detection (`SECURITY_PROMPT_CLASSIFIER_*`) sends the tool
  call to a Hugging Face Inference API-compatible endpoint for
  semantic classification; the endpoint URL and bearer token must be
  configured.
- Adversary Mode is a separate LLM-based reviewer pass run before
  tool execution; fail-open.
- **No** scanning of MCP tool *result/output* content is documented;
  Claudine's `protect` layer should treat MCP tool results as
  untrusted user content.

## Mode-Specific Behavior

### Interactive mode (`goose session`, `goose run --interactive`)

- Extensions can be added / toggled via `goose configure` or slash
  commands (`/extension`, `/builtin`).
- OAuth flows complete through the UI.
- MCP prompts are available via `/prompts` and `/prompt <n>`.
- Working-directory changes update MCP roots.
- Channel-capable MCP servers (e.g. council-style extension sets) are
  not described in goose.

### Non-interactive / headless mode (`goose run`, `goose run --no-session`)

- OAuth interactive flows cannot complete; pre-authenticated servers
  or env-based secrets are required.
- Runtime extension injection works through `--with-extension`,
  `--with-streamable-http-extension`, `--with-builtin`.
- `GOOSE_CONTEXT_STRATEGY` defaults to `summarize` (vs `prompt` in
  interactive).
- `--debug` and `/r` show full tool parameters and responses.

### Goose-as-MCP-server

`goose mcp <name>` runs an already-enabled MCP extension by name
without the goose runtime — useful for exposing goose-bundled MCP
servers to other agentic CLIs. `goose acp` runs Goose itself as an
ACP agent server over stdio for ACP-compatible clients (e.g. Zed).

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Extension fails to start | Error shown in session; that extension's tools unavailable |
| Streamable HTTP unreachable | Connection error at startup |
| Tool timeout | Per-extension `timeout` (and per-call `MCP_TIMEOUT` if set) |
| Large tool output | Written to temp file when response exceeds `GOOSE_MAX_TOOL_RESPONSE_SIZE` |
| Elicitation timeout | Cancelled after 300 seconds |
| Prompt injection detected | Security alert with Allow / Deny choice (configurable threshold) |
| Adversary reviewer unreachable | Fail-open: tool call proceeds |
| Disallowed `envs` key (`PATH`, `LD_PRELOAD`, …) | Silently skipped (`warn!`); `validate()` returns an error if the strict path is used |
| OAuth callback blocked by IdP (wildcard-port rejected) | Set `GOOSE_OAUTH_CALLBACK_PORT` to a registered port and re-run |
| Unknown tool during `tools/call` | Error surfaced to the model with `isError` |

## Gaps

- The public configuration-files docs page lists
  `~/.config/goose/config.yaml` as the macOS/Linux user path,
  contradicting the source code (etcetera `Block` author → macOS uses
  `~/Library/Application Support/Block/goose/`, Linux uses
  `~/.config/Block/goose/`, Windows uses `%APPDATA%\Block\goose\`).
- The docs page lists Windows user config as
  `%APPDATA%\Block\goose\config\config.yaml`; the source uses
  `%APPDATA%\Block\goose\config.yaml` (no `config/` subdirectory).
- `resources/templates/list`, `resources/subscribe`, and
  `resources/list_changed` notification handling are not exposed in any
  documented UI surface.
- OAuth token scope, refresh, and storage semantics for remote
  Streamable-HTTP MCP servers are not fully documented past "uses a
  local callback server".
- There is no `goose mcp add` / `goose mcp remove` CLI pair — only
  `goose configure` (interactive TUI) and `goose mcp <name>` (re-runs
  an enabled extension).
- There is no `--mcp-config <file-or-json>` analogue to Claude Code's
  flag; runtime injection is exclusively flag-based.
- `GOOSE_SANDBOX` scope is described only for goose Desktop; whether
  macOS `sandbox-exec` covers the *stdio MCP extension* subprocesses
  is not verified in source.
- Goose is not installed on this host (`~/Library/Application Support/Block/goose/`
  and `~/.config/goose/` do not exist; `which goose` reports not found).
  All shape and merge findings are from official docs and source code at
  `aaif-goose/goose`, not from on-host inspection.

## Claudine Integration Notes

- Treat Goose as `support: import_sync`. Map the catalog to the
  Goose `extensions:` map shape and merge per-extension entries
  deep when both layers define the same name, otherwise shallow replace.
- Honor the etcetera paths exactly: macOS
  `~/Library/Application Support/Block/goose/config.yaml`, Linux
  `~/.config/Block/goose/config.yaml`, Windows
  `%APPDATA%\Block\goose\config.yaml`. Do *not* write to
  `~/.config/goose/config.yaml` on macOS/Linux — that path does not
  exist in this build.
- For one-run wrappers, emit per-flag injection
  (`--with-extension`, `--with-streamable-http-extension`, `--with-builtin`)
  rather than the `--mcp-config <file>` pattern. There is no equivalent
  to load a complete server set as a single blob.
- Write `secrets.yaml` only when `GOOSE_DISABLE_KEYRING` is set or the
  keyring is unavailable — otherwise the keyring service `goose` /
  username `secrets` is the canonical storage.
- Treat secrets as user-scoped; rely on `env_keys` + `${VAR}` so secret
  values never live as plaintext in `config.yaml`.
- Map `permission.yaml` categories (user / smart_approve) to per-tool
  permission rules in Claudine's permission engine. Surface the
  `available_tools` allowlist per extension as a filter.
- Goose currently has no managed-policy file equivalent; there is no
  `~/.claude/settings.json`-style allowlist directory for admins.
  Recommend `GOOSE_ALLOWLIST` as the deployment-time hardener.
- Defensively scan MCP tool results; Goose provides prompt-injection
  detection on *tool-call parameters before execution* but no documented
  result sanitizer.
- Update Claudine's provider metadata to:
    • drop the "Goose has no MCP" categorization,
    • flag Goose as `import_sync` with `apply_supported: false`,
    • record protocol `2025-03-26` (not 2025-06-18),
    • replace the user-config paths with the correct etcetera paths,
    • surface the new env vars listed above.

## Changelog

- **2026-07-02** — Initial research. Discovered that Goose has
  first-class MCP support (stdio and Streamable HTTP), persistent YAML
  extension config, runtime injection flags, roots, sampling, and
  elicitation. Corrects the prior "no MCP support" classification.
- **2026-07-03** — Current revision. Tightens the protocol version to
  `2025-03-26`; corrects the actual etcetera user-config paths; flags
  `sse` as a compatibility-only enum; documents `platform`,
  `frontend`, `inline_python`, and the `streamable_http.socket` UDS
  field; promotes `Envs` filtering to a known fact (31 disallowed
  keys); promotes the deep merge in `extensions:` / `providers:` over
  the previously claimed shallow merge; adds the system-wide
  `/etc/goose/config.yaml`, `GOOSE_ADDITIONAL_CONFIG_FILES`, and the
  long tail of new env vars; notes goose's April-2026 move from
  `block/goose` to `aaif-goose/goose` at AAIF / Linux Foundation and
  the latest version v1.41.0; records the on-host absence of `goose`
  as a verification gap.

## Sources

- [Goose docs home](https://goose-docs.ai/)
- [Using Extensions](https://goose-docs.ai/docs/getting-started/using-extensions)
- [Configuration Files](https://goose-docs.ai/docs/guides/config-files)
- [Environment Variables](https://goose-docs.ai/docs/guides/environment-variables)
- [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [MCP Roots](https://goose-docs.ai/docs/guides/mcp-roots)
- [MCP Sampling](https://goose-docs.ai/docs/guides/mcp-sampling)
- [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation)
- [Extension Manager](https://goose-docs.ai/docs/mcp/extension-manager-mcp)
- [Developer Extension](https://goose-docs.ai/docs/mcp/developer-mcp)
- [Extension Allowlist](https://goose-docs.ai/docs/guides/allowlist)
- [goose Permission Modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions)
- [Managing Tool Permissions](https://goose-docs.ai/docs/guides/managing-tools/tool-permissions)
- [Prompt Injection Detection](https://goose-docs.ai/docs/guides/security/prompt-injection-detection)
- [Adversary Mode](https://goose-docs.ai/docs/guides/security/adversary-mode)
- [macOS Sandbox for goose Desktop](https://goose-docs.ai/docs/guides/sandbox)
- [Running goose in Docker](https://goose-docs.ai/docs/tutorials/goose-in-docker)
- [Figma Extension (remote Streamable HTTP example)](https://goose-docs.ai/docs/mcp/figma-mcp)
- [GitHub repo (block → aaif-goose transition)](https://github.com/aaif-goose/goose)
- [MCP client source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/mcp_client.rs)
- [Extension config source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/agents/extension.rs)
- [Config base source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/base.rs)
- [Config paths source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs)
- [Permission manager source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/permission.rs)
- [Security pattern source](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/security/patterns.rs)
- [MCP Roots spec (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18/client/roots)
- Local probe: `ls ~/Library/Application Support/Block/goose/` /
  `ls ~/.config/goose/` → both `No such file or directory`;
  `which goose` → not found on this host.
