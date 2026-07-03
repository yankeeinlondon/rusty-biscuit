---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: minimax/MiniMax-M3
docs: https://kilocode.ai/docs/features/mcp/overview
support: runtime_injection
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http, http_sse]
  lifecycle: |
    Local (stdio) MCP servers are spawned as child processes at session start;
    failures are surfaced in the `/mcps` toggle and `kilo mcp list` output and
    are not auto-retried mid-session. Remote servers try Streamable HTTP first
    and fall back to legacy SSE within the configured `timeout` window. Tool
    lists are refreshed on `tools/list_changed` notifications, and a capability
    list is fetched from each server (tools/list, prompts/list, resources/list)
    during connect. The docs do not name a specific MCP protocol version date;
    behavior matches the spec at https://modelcontextprotocol.io.
  notes: |
    Kilo's config schema is forked from OpenCode and uses provider-native
    transport names: `local` for command-based stdio servers and `remote` for
    HTTP-based servers. The runtime uses the three upstream transports
    (StdioClientTransport, StreamableHTTPClientTransport, SSEClientTransport)
    directly, so Streamable HTTP is preferred for new servers and SSE is a
    documented fallback. There is no Kilo-level WebSocket transport, and
    interactive protocol-version negotiation is not exposed.
config_files:
  - os: all
    scope: user
    path: "~/.config/kilo/kilo.json[c]"
    format: json
    notes: |
      Primary user-level config. MCP servers live under the top-level `mcp`
      object. The legacy `opencode.json[c]` is also read for compatibility.
      JSONC comments are preserved when the file ends in `.jsonc`.
  - os: all
    scope: repo
    path: "kilo.json[c]"
    format: json
    notes: |
      Project-level config at the repo root. Same schema as user config. Safe
      to commit to Git; per Marketplace guidance, avoid committing API keys.
      Project config overrides the user config.
  - os: all
    scope: repo
    path: ".kilo/kilo.json[c] / .kilocode/kilo.json[c]"
    format: json
    notes: |
      Alternative project-level location. `.kilo/kilo.json[c]` takes priority
      over root-level files when both exist; `.kilocode/kilo.json[c]` is
      supported as a legacy location.
  - os: all
    scope: plugin
    path: "<plugin-root>/.kilocode/mcp.json"
    format: json
    notes: |
      Docs site shows a `packages/kilo-docs/.kilocode/mcp.json` example.
      Bundled MCP marketplace entries install an `mcp` key into the chosen
      config (global `~/.config/kilo/kilo.json` or project
      `./.kilo/kilo.json`).
  - os: macos
    scope: system
    path: "/Library/Application Support/kilo/opencode.json"
    format: json
    notes: |
      File-based managed config dir for Kilo (forked from OpenCode's path,
      renamed to `kilo`). Highest file-based precedence; not user-overridable.
  - os: linux
    scope: system
    path: "/etc/kilo/opencode.json"
    format: json
    notes: |
      Linux/WSL managed config path.
  - os: windows
    scope: system
    path: "%ProgramData%\\kilo\\opencode.json"
    format: json
    notes: |
      Windows managed config path. Confirmed in
      `packages/opencode/src/config/managed.ts`.
  - os: macos
    scope: managed
    path: "/Library/Managed Preferences/<user>/ai.opencode.managed.plist"
    format: other
    notes: |
      macOS MDM-deployed preferences using the `ai.opencode.managed` plist
      domain (carried over from OpenCode). The plist's PayloadContent keys map
      directly to Kilo config fields, so an admin can ship a managed `mcp`
      allowlist. Highest precedence overall.
  - os: all
    scope: other
    path: "KILO_CONFIG"
    format: json
    notes: |
      Environment variable pointing to a custom config file. Loaded between
      global and project configs in the precedence order.
  - os: all
    scope: other
    path: "KILO_CONFIG_CONTENT"
    format: json
    notes: |
      Inline JSON config content for one-run injection. Loaded as a `local`
      source (above project configs but below remote and managed sources).
      This is the runtime injection mechanism Claudine uses, by analogy with
      OpenCode's `OPENCODE_CONFIG_CONTENT`.
  - os: all
    scope: other
    path: "KILO_CONFIG_DIR"
    format: json
    notes: |
      Environment variable naming an alternate config directory that supplies
      agents/commands/plugins/skills, layerable over `.kilo`.
  - os: all
    scope: remote
    path: "<provider>/.well-known/opencode"
    format: json
    notes: |
      Well-known endpoint fetched when a remote provider (e.g. Kilo Gateway)
      returns a `type: wellknown` credential entry. Remote config is loaded
      first as the base layer; remote `mcp` entries may be `enabled: false`
      until the user opts in.
  - os: all
    scope: other
    path: "<Global.Path.data>/mcp-auth.json"
    format: json
    notes: |
      OAuth credential store for MCP servers, separate from server
      definitions. Resolved via XDG: `~/.local/share/kilo/mcp-auth.json` on
      Linux/macOS and `%LOCALAPPDATA%\kilo\mcp-auth.json` on Windows. File
      mode `0o600`.
cli_params:
  - flag: "kilo mcp add"
    description: |
      Interactive wizard to add a local (stdio) or remote (Streamable HTTP/SSE)
      MCP server; prompts for scope (Current project vs Global when the
      directory is a git repo), name, and per-server fields, then merges the
      entry into the chosen `kilo.json[c]` via `jsonc-parser`. Command-line
      flags are not documented; only the interactive path is supported.
    example: "kilo mcp add"
  - flag: "kilo mcp list"
    description: |
      List all configured MCP servers with per-server status: `connected`,
      `disabled`, `needs authentication`, `needs client registration`, or
      `failed` (with the error message).
    example: "kilo mcp list"
  - flag: "kilo mcp ls"
    description: "Alias for `kilo mcp list`."
    example: "kilo mcp ls"
  - flag: "kilo mcp auth [name]"
    description: |
      Run the OAuth flow against a remote server. If no name is given, an
      interactive picker of OAuth-capable servers is shown. The OAuth browser
      redirect is opened automatically; the MCP callback path is
      `OAUTH_CALLBACK_PATH` (handled by an embedded HTTP callback server).
      Re-running for an already-authenticated server confirms before
      re-authenticating.
    example: "kilo mcp auth sentry"
  - flag: "kilo mcp auth list"
    description: "List OAuth-capable MCP servers and their auth status."
    example: "kilo mcp auth list"
  - flag: "kilo mcp logout [name]"
    description: |
      Remove stored OAuth credentials for a server. If no name is given, an
      interactive picker is shown.
    example: "kilo mcp logout sentry"
  - flag: "kilo mcp debug <name>"
    description: |
      Diagnose a server's OAuth/HTTP issues; shows current auth status, tests
      HTTP connectivity, and attempts OAuth discovery.
    example: "kilo mcp debug sentry"
env_vars:
  - name: KILO_CONFIG
    effect: |
      Path to a custom config file. Loaded between global and project
      precedence tiers. Used by some installations to point at a per-project
      override.
  - name: KILO_CONFIG_CONTENT
    effect: |
      Inline JSON config applied for the current run (analogous to OpenCode's
      `OPENCODE_CONFIG_CONTENT`). Sources are tagged as `local`. The cleanest
      runtime injection path for Claudine wrappers.
  - name: KILO_CONFIG_DIR
    effect: |
      Path to an alternate config directory for agents/commands/plugins,
      layered over `.kilo`.
  - name: KILO_TUI_CONFIG
    effect: "Path to a custom TUI config file."
  - name: KILO_DISABLE_PROJECT_CONFIG
    effect: |
      When set, Kilo skips loading `kilo.json` / `.kilo/` from the project
      hierarchy. Useful for sandboxed or wrapper-driven runs.
  - name: KILO_PERMISSION
    effect: "Inlined JSON permissions config, merged over the loaded config."
  - name: KILO_DISABLE_DEFAULT_PLUGINS
    effect: "Disable default plugins that ship with the build."
  - name: KILO_DISABLE_EXTERNAL_SKILLS
    effect: "Disable loading skills from outside the config tree."
  - name: KILO_EXPERIMENTAL
    effect: "Umbrella flag that flips the unstable defaults on."
  - name: KILO_EXPERIMENTAL_LSP_TOOL
    effect: "Enables the experimental LSP tool surface (matters for skills, not MCP)."
  - name: KILO_CLIENT
    effect: |
      Identifies the calling client (defaults to `cli`); used in telemetry and
      some user-agent strings.
  - name: KILO_BWRAP_PATH
    effect: |
      Path to the bubblewrap binary used by the optional sandbox backend on
      Linux. Does not change MCP transport behavior but matters for whether
      the agent itself runs sandboxed.
  - name: KILO_TEST_HOME / KILO_TEST_MANAGED_CONFIG_DIR
    effect: |
      Test-only overrides for `Global.Path.home` and the managed config dir;
      not intended for production.
server_schema:
  transports: ["local", "remote"]
  command_fields: ["type", "command", "environment", "disabled", "timeout"]
  http_fields: ["type", "url", "headers", "oauth", "disabled", "timeout"]
  env_shape: |
    `environment` is an object mapping variable names to string values. The
    runtime merges these over `process.env` and inherits the rest. The newer
    runtime schema accepts either a v2 entry (`{type, command, environment,
    disabled?, timeout?}`) or a legacy `{enabled: false}` form for entries
    used purely to disable a server; per the runbook, prefer the v2 fields
    and use `disabled: true` to suppress a server without removing the entry.
    Env-var and file substitution follow OpenCode's
    `{env:NAME}` / `{file:path}` syntax.
  auth_shape: |
    Remote servers support OAuth 2.0 with PKCE under `oauth` (an object) or
    `oauth: false` to opt out and use static `headers` instead. The OAuth
    object accepts `clientId`, `clientSecret`, `scope`, `callbackPort` (1-65535),
    and `redirectUri`. Dynamic Client Registration (RFC 7591) is attempted
    automatically; pre-registered `clientId`/`clientSecret` can be supplied.
    Tokens are written to `<Global.Path.data>/mcp-auth.json`.
  notes: |
    Server id is the map key under `mcp`. The `type` field accepts `"local"`
    or `"remote"`. There is no Kilo-level `stdio` or `streamable-http` value
    in the schema — the SDK calls the older SSEClientTransport as a runtime
    fallback when Streamable HTTP fails. Stdio servers have an
    `ensureDockerRm` codepath that injects `--rm` into `docker run` /
    `podman run` invocations to avoid accumulating stopped containers.
server_capabilities:
  tools: full
  resources: partial
  prompts: partial
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: true
  prompt_list_changed: true
  notes: |
    Tools are auto-discovered via `tools/list` and presented to the model
    with permission keys namespaced `{server}_{tool}` (e.g.
    `github_create_pull_request`) so they plug into the same
    `allow`/`ask`/`deny` system as built-in tools. Resources and prompts are
    listed by the source (`fetchFromClient`), but only a subset is wired into
    the UI: tools get the rich `mcp__*__*` permission surface; prompts and
    resources get listed in the toggle panel and are reachable through the
    SDK, but Kilo does not document slash-command or picker exposure for
    them.
client_capabilities:
  roots: unknown
  sampling: none
  elicitation: unknown
  notes: |
    The MCP Service exposes `prompts`, `resources`, `tools`, `add`,
    `connect`, `disconnect`, `getPrompt`, `readResource`, and auth helpers,
    but does not implement `roots/list`, `sampling/createMessage`, or
    elicitation from the upstream MCP servers' perspective. Roots are not
    surfaced, sampling is not advertised, and elicitation requests are not
    documented. The JetBrains and VS Code fronts add their own UI on top but
    do not change these protocol-level capabilities.
tool_surface:
  discovery: |
    At session start, Kilo's MCP service iterates the config map, creates
    one `Client` per enabled server, calls `connect(transport)` with the
    per-server `timeout`, and calls `tools/list`. When the client receives a
    `notifications/tools/list_changed`, the service refetches the tool list
    and publishes a `mcp.tools.changed` bus event the agent and TUI react to.
  filtering: |
    Tool-level filtering happens through the permission system: rules like
    `my_server_*` under `permission.<tool>` map to `allow`/`ask`/`deny`, so
    individual tools can be hidden from the model without removing the
    server. Server-level filtering happens through `disabled: true` /
    `enabled: false` in config, through the `tools` legacy field (e.g.
    `tools: {"my-mcp*": false}`), or via per-agent `agent.<name>.tools` /
    `agent.<name>.permission` entries.
  approval: |
    MCP tools obey the same `permission` engine as built-in tools. Each
    call is evaluated as `<server>_<tool>` against the matched rule (with
    `*`-globbing); unmatched requests default to `ask` for actions like
    `bash` and to `allow` for most others. The TUI's "Always run" options
    append new rules to the user's global config, and the Marketplace card
    preview describes how `allow`/`ask`/`deny` interact with each server.
  result_handling: |
    Results are converted to the AI SDK `Tool` shape and returned to the
    model. `tools/call` responses are returned via
    `CallToolResultSchema` with `resetTimeoutOnProgress: true`. Errors during
    `tools/list` that stem from invalid `outputSchema` references fall back
    to a tolerant schema (omitting `outputSchema`); other errors are
    surfaced per server in `kilo mcp list` and the `/mcps` toggle.
  annotations_trusted: |
    The runtime forwards tool definitions without applying tool-side
    annotations beyond the input/JSON-schema layer. `toolAnnotations` are
    not advertised as policy inputs; only the namespaced permission rule
    controls visibility.
  notes: |
    There is no documented per-argument approval policy for MCP tools.
    Approval prompts are the standard "Run" / "Deny" pair; the Runtime
    Approval auto-approve toggle in the TUI affects both native and MCP
    tools in lockstep.
resource_surface:
  supported: true
  uri_schemes: ["depends on server"]
  templates: true
  subscriptions: false
  exposure_model: |
    Resources are listed by the SDK (`fetchFromClient` calls
    `client.listResources()`) and stored under
    `<sanitizedClient>:<sanitizedName>` keys, so the underlying transport
    decides which URI schemes appear. Kilo does not document a user-visible
    picker for resources; whether they appear in the chat UI depends on the
    surrounding IDE/extension (VS Code/JetBrains) and on prompts that ask
    the model to call `readResource`. There is no documented
    `resources/subscribe` story and no subscriptions URI list.
prompt_surface:
  supported: true
  invocation: |
    Prompts are listed by the SDK and addressed by `<server>:<prompt>`. The
    VS Code UI surfaces MCP prompts (e.g. through McpEditView and the
    per-prompt `getPrompt` helper), but Kilo's docs do not document a
    Kilo-native slash command for prompts. Treat slash-command/palette
    exposure as `unknown` until proved otherwise.
  arguments: |
    `mcp.getPrompt(clientName, name, args?)` accepts an `args` map; UI
    argument collection depends on the front-end (the JetBrains
    `McpEditDialog` and VS Code `McpEditView` implement parameter forms).
  exposure_model: |
    Prompts are reachable through the SDK but there is no documented
    Kilo-native slash command or palette action that injects an MCP prompt
    directly. Treat them as `partial`: discovered and invokable, not
    surfaced as first-class slash commands.
  notes: |
    `fetchFromClient` is shared with resources, so capability drift (e.g. a
    dropped `prompts/list_changed` notification) is observable through the
    same client connection.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: shallow
  notes: |
    Claudine can read both `kilo.json[c]` / `.kilo/kilo.json[c]` and the
    legacy `opencode.json[c]` and normalize server entries back to the v2
    shape. Merge at load time uses Kilo's `mergeConfigConcatArrays` (arrays
    concatenated; objects deep-merged; scalars overwritten by later
    sources). There is no documented non-interactive apply path —
    `kilo mcp add` only runs interactively and merges its result into the
    selected config file via `jsonc-parser` while preserving comments.
    Apply via `kilo mcp add` is therefore unsafe for non-TTY runs.
runtime_injection:
  supported: true
  mechanism: |
    Set `KILO_CONFIG_CONTENT` to an inline JSON document containing the
    `mcp` map before launching `kilo`, `kilo run`, or any other
    subcommand. The runtime treats the inline source as `local` precedence
    (above project configs, below remote and managed sources). The
    companion `KILO_CONFIG` / `KILO_CONFIG_DIR` vars let Claudine point Kilo
    at a temp file or directory written from the MCP catalog instead.
  limitations: |
    `KILO_CONFIG_CONTENT` does NOT preserve the user's persistent merge
    semantics by itself — it is concatenated with remote + managed sources,
    but if it shares keys with the user's project config, the last-loaded
    source (managed > inline > project > custom > global > remote) wins.
    Claudine should build the full effective server list itself for `-p` /
    `run` use, set `KILO_DISABLE_PROJECT_CONFIG` if user config is to be
    omitted, and rely on `KILO_CONFIG_CONTENT` for the body.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens and Dynamic Client Registration state are written to
    `<Global.Path.data>/mcp-auth.json`, resolved via XDG (`~/.local/share/kilo/`
    on Linux/macOS; `%LOCALAPPDATA%\kilo\` on Windows; `KILO_TEST_HOME`
    overrides in tests). Stored at file mode `0o600`. The data dir is
    excluded from macOS Spotlight by default. There is no use of an OS
    keychain for MCP OAuth credentials (unlike some other providers).
  token_scope: |
    One entry per remote server URL (`serverUrl` field in the entry),
    keyed by config-side server name. Refresh tokens are stored when the
    server returns them; `isTokenExpired` is consulted before each call so
    re-auth can be triggered when tokens lapse.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `environment` object or
    via `{env:VAR}` / `{file:path}` substitution in `kilo.json[c]`. Process
    environment is otherwise inherited, including any platform-specific
    credentials the user has on PATH. The experimental sandbox explicitly
    does NOT isolate local MCP servers from network access.
  notes: |
    `oauth: false` opts a remote server out of the OAuth auto-detect path
    and forces header-based auth. For pre-registered clients, supply
    `clientId` (and optionally `clientSecret`) under `oauth`. Re-running
    `kilo mcp auth` with already-valid credentials confirms before
    re-authenticating. There is no per-run `--client-secret` flag analogue;
    secrets must already exist in env or be supplied via the OAuth flow.
security:
  tool_filtering: |
    Three complementary layers: (1) config-side `disabled: true` (v2) /
    `enabled: false` (legacy) on a single server; (2) the `tools` /
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
    the rest of the config. macOS/Linux/Windows managed-config directories
    are admin-only.
  env_sanitization: |
    Each stdio MCP server receives only the entries in its `environment`
    object plus inherited `process.env`. There is no documented subprocess
    env scrub analogous to Claude Code's
    `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`; anything in the user's environment
    that is not overridden is reachable by a local MCP command. The
    experimental sandbox restricts network from the model's bash / web
    tools, but explicitly excludes local MCP servers.
  sandbox_interaction: |
    The sandbox (macOS `sandbox-exec` / Linux `bwrap`, Windows is not
    supported) applies to agent shell commands and file-write tools; it is
    documented to NOT cover local MCP servers and plugin hooks. The
    `experimental.sandbox` and `experimental.sandbox_restrict_network`
    settings affect only model-originated commands and first-party HTTP
    tools; MCP stdio traffic remains outside the bubblewrap/seatbelt
    confinement.
  response_filtering: |
    No native MCP response sanitization is documented. Tool outputs go
    straight to the model after `tools/call`. Claudine's `protect` layer
    should treat MCP tool results as untrusted and scan them for
    injection-style patterns, as it does for other agentic CLIs.
  notes: |
    OAuth tokens live in `mcp-auth.json` at `0o600`, not in an OS
    keychain. The .well-known remote source can ship default `mcp` entries
    with `enabled: false`; admin and managed directories add hard
    allowlist/denylist capability.
---

# MCP Support in Kilo Code

## Overview

Kilo Code supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io) as a first-class extension point for its VS Code, JetBrains, and CLI surfaces. Kilo CLI 1.0 is built from the [Kilo-Org/kilocode](https://github.com/Kilo-Org/kilocode) monorepo and shares a single core with the IDE extensions; the CLI binary `kilo` is what most of this document covers. The codebase is forked from OpenCode, so the MCP schema, CLI surface, and managed-config paths are recognisably OpenCode-shaped (renamed `opencode` to `kilo`) with Kilo-specific additions such as the Marketplace installer, the OAuth CLI commands (`kilo mcp auth` / `logout` / `debug`), and a JetBrains/VS Code front.

Kilo exposes both **local** MCP servers (stdio, launched as a child process) and **remote** MCP servers (HTTP, tried as Streamable HTTP and falling back to legacy SSE). Tools are auto-discovered via `tools/list` and presented to the model under namespaced names; the existing `permission` engine treats them as first-class tools. A [Kilo Marketplace](https://github.com/Kilo-Org/kilo-marketplace) curates shared Skills, MCP servers, and Agents that can be installed at project or global scope.

This document maps Kilo Code's MCP behavior to the schema used by Claudine's MCP catalog and provider wrappers.

## Protocol and Transports

Kilo documents two user-visible transport categories — **Local** and **Remote** — but the underlying runtime uses the upstream SDK's three transports:

| User-facing `type` | Runtime transport(s) | How it is added |
| :----------------- | :------------------- | :-------------- |
| `"local"` | `StdioClientTransport` | `kilo mcp add` (interactive) |
| `"remote"` | `StreamableHTTPClientTransport`, falling back to `SSEClientTransport` | `kilo mcp add` (interactive) |

Lifecycle behavior:

- **Local (stdio)** servers are spawned as child processes at session start. Failures surface as a "failed" status in `kilo mcp list` (and the `/mcps` toggle) and are not auto-retried mid-session. Stderr from the child is logged via the MCP service (`mcp stderr: ...`) but is not streamed to the user.
- **Remote** servers try Streamable HTTP first; on connection error (other than auth) they fall back to legacy SSE within the configured `timeout`. Auth errors break the loop and surface a `needs_auth` or `needs_client_registration` status that points the user at `kilo mcp auth <name>`.
- **Dynamic capability updates**: servers may send `tools/list_changed`; the MCP service refetches the tool list and publishes a `mcp.tools.changed` event so the TUI and agent refresh without a session restart.

The docs do not name a specific MCP protocol version date; treat the implemented version as observed rather than pinned.

## Configuration

Kilo's MCP servers live in the same `kilo.json[c]` config file as the rest of the configuration. The `mcp` key is a map of server name to server definition.

### File layout

| Scope | File | Notes |
| :---- | :--- | :---- |
| User | `~/.config/kilo/kilo.json[c]` | XDG config home on Linux/macOS, `%APPDATA%` on Windows; legacy `opencode.json[c]` is also read. |
| Project (root) | `kilo.json[c]` | Project-scoped; overrides user config when present. |
| Project (in-dir) | `.kilo/kilo.json[c]` (preferred) or `.kilocode/kilo.json[c]` (legacy) | Same precedence as the root form. |
| Plugin | `<plugin-root>/.kilocode/mcp.json` (or Marketplace install that writes `mcp` into the chosen config) | Bundled by some marketplace entries; project or global scope selected at install. |
| Managed | `/Library/Application Support/kilo/opencode.json` (macOS), `/etc/kilo/opencode.json` (Linux), `%ProgramData%\kilo\opencode.json` (Windows), plus the `ai.opencode.managed` MDM plist (macOS) | Admin-controlled, highest precedence. |
| Remote | `<provider>/.well-known/opencode` | Fetched when a remote provider supplies a `type: wellknown` credential; loaded first as the base layer; may ship `mcp` entries with `enabled: false` to opt out. |

### Precedence order

Files are merged, not replaced (arrays concatenated; objects deep-merged; scalars overwritten by later sources). The combined precedence from lowest to highest:

1. Remote config (`.well-known/opencode`)
2. Global config (`~/.config/kilo/...`)
3. Custom config (`KILO_CONFIG` env var)
4. Project config (`kilo.json[c]` / `.kilo/kilo.json[c]`)
5. `.kilo` / `.kilocode` directories (agents, commands, plugins, skills)
6. Inline config (`KILO_CONFIG_CONTENT`) — tagged `local`, above project configs
7. File-based managed config directory (`/Library/Application Support/kilo/`, `/etc/kilo/`, `%ProgramData%\kilo\`)
8. macOS managed preferences (`.mobileconfig` via MDM)

`KILO_DISABLE_PROJECT_CONFIG=true` skips tier 4 and 5, which is useful when Claudine wants to ship a clean effective config via inline injection.

## Server Definition Shape

A server definition under `mcp.<name>` looks like one of:

```jsonc
{
  "mcp": {
    "my-local-server": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem"],
      "environment": { "API_KEY": "..." },
      "disabled": false,
      "timeout": 10000
    },
    "my-remote-server": {
      "type": "remote",
      "url": "https://example.com/mcp",
      "headers": { "Authorization": "Bearer {env:MY_API_KEY}" },
      "oauth": false,
      "disabled": false,
      "timeout": 15000
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
| `oauth` | remote | OAuth object, or `false` to opt out and force header-based auth |
| `disabled` | both | Set to `true` to suppress the server without removing the entry |
| `timeout` | both | Per-server connection + tool-call timeout in ms (default `5000` for local, `30000` for remote, per source) |

The JSON parser at parse time also accepts the legacy `{enabled: false}` form for an entry used purely to disable a server (see the v2 union in `packages/opencode/src/config/config.ts`). The docs page uses `enabled: true`; the new Effect schema in `packages/core/src/config/mcp.ts` uses `disabled`. Both forms are honored.

### Environment and variable substitution

- `{env:VARIABLE_NAME}` expands a process env var.
- `{file:path/to/file}` reads a file's contents; paths can be absolute (`/`, `~`) or relative to the config directory.

If a substitution target is unset and has no default, the value is replaced with the empty string for env vars and the file is read at load.

### OAuth shape

```jsonc
{
  "oauth": {
    "clientId": "{env:MY_MCP_CLIENT_ID}",
    "clientSecret": "{env:MY_MCP_CLIENT_SECRET}",
    "scope": "tools:read tools:execute",
    "callbackPort": 4096,
    "redirectUri": "http://127.0.0.1:4096/callback"
  }
}
```

If `oauth` is omitted, Dynamic Client Registration is attempted; if the server rejects it, the MCP service marks the server as `needs_client_registration` and prints the JSON to add `clientId`/`clientSecret` to the config.

## Tools, Resources, and Prompts

### Tools

All MCP tools are auto-discovered and presented to the model under namespaced names. The Kilo CLI / JetBrains / VS Code surfaces use the convention `{server}_{tool}` in permission rules, e.g.:

```jsonc
{
  "permission": {
    "github_create_pull_request": "allow",
    "github_*": "ask"
  }
}
```

Discovery:

- `tools/list` is called once on connect with the per-server `timeout`.
- A `notifications/tools/list_changed` handler refetches the list and emits `mcp.tools.changed`.
- Errors during `tools/list` that stem from invalid `outputSchema` references fall back to a tolerant schema (omitting `outputSchema`).

Approval:

- Tools respect the same `allow` / `ask` / `deny` engine as built-in tools. The TUI's runtime auto-approve toggle (the "shield" button) covers MCP calls in lockstep.
- The Marketplace MCP install dialog displays the source author, requested parameters, and any platform-specific requirements before writing to the chosen config.

Result handling:

- `tools/call` responses are validated against `CallToolResultSchema` with `resetTimeoutOnProgress: true`, so progress notifications keep the idle timer happy.
- There is no documented per-call output-token cap or persistence-to-disk step; large outputs flow back to the model verbatim.

### Resources and prompts

The MCP source code in `packages/opencode/src/mcp/index.ts` lists both `resources` and `prompts` via `fetchFromClient`, so they are reachable from any Kilo front end that asks the model to call `readResource` / `getPrompt`. The Marketplace install path can also surface them in the VS Code `McpEditView` / JetBrains `McpEditDialog`. Kilo's primary docs do not describe a Kilo-native slash command or palette action that lists MCP prompts as first-class commands, so Claudine should treat prompts and resources as **partial** until a future doc pass promotes them.

## Roots, Sampling, and Elicitation

Kilo's MCP service does not implement the client-side capabilities of `roots/list`, `sampling/createMessage`, or elicitation from the perspective of an MCP server. There is no:

- public way for an MCP server to ask Kilo "what are your filesystem boundaries";
- documented support for an MCP server asking the model to make a nested LLM call; or
- elicitation path for an MCP server to collect structured user input through Kilo's UI.

Treat all three as `unknown` until proven otherwise; in practice this means an MCP server running against Kilo must either complete its work using only tools the server itself exposes, or rely on the host application (VS Code / JetBrains) for filesystem context.

## Import, Export, and Sync

Claudine can treat Kilo Code as an `import_sync` candidate, with caveats:

- **Import**: read the merged `kilo.json[c]` config(s) — including the XDG-resolved paths, `KILO_CONFIG`, `KILO_CONFIG_CONTENT`, and managed directories — and normalize the `mcp` key into Claudine's MCP catalog. Also consume the legacy `opencode.json[c]` paths Kilo still reads.
- **Export**: write provider-shaped JSON back to those files (`.kilo/kilo.json[c]` for project scope, `~/.config/kilo/kilo.jsonc` for user scope). Claudine should preserve comments by writing `.jsonc` only when the source was `.jsonc`, and use `jsonc-parser` (which `kilo mcp add` already does) for surgical edits.
- **Apply**: there is **no documented non-interactive apply path**. `kilo mcp add` is interactive only; it does not accept flags. Claudine should therefore:
  - write to the config file directly with `jsonc-parser`, atomic-rename style; then
  - call `kilo mcp auth` only when OAuth setup is required (and only when a TTY is available).

Merge semantics:

- Arrays are concatenated across scopes (e.g. `plugin`, `instructions`).
- Objects deep-merge with later sources winning conflicts.
- Scalar values are overwritten by the higher-precedence source.
- `disabled: true` (or `enabled: false`) wins for individual server entries regardless of where they appear.

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
- OAuth flows cannot complete in non-interactive `kilo run` mode; pre-authenticated servers or static headers are required for `run`.
- The runtime cannot tell the difference between `KILO_CONFIG_CONTENT` and a project config of the same keys at the same precedence tier; if you want strict control, also set `KILO_DISABLE_PROJECT_CONFIG=1`.
- There is no `--bare` analogue for Kilo; the closest is `KILO_DISABLE_PROJECT_CONFIG=1` + no explicit project file.

## Authorization and Credentials

OAuth 2.0 with PKCE is the default for remote MCP servers that advertise it. Flow:

1. The MCP service detects the 401 response.
2. Dynamic Client Registration (RFC 7591) is attempted unless the config supplies `clientId`/`clientSecret`.
3. The Kilo CLI opens the browser via the `open` package at the provider's authorization URL.
4. The MCP OAuth callback server (embedded in the CLI on a per-call port) catches the redirect and finishes the flow.
5. Tokens + refresh token + DCR state are written to `<Global.Path.data>/mcp-auth.json` (`~/.local/share/kilo/mcp-auth.json` on Linux/macOS, `%LOCALAPPDATA%\kilo\mcp-auth.json` on Windows) at file mode `0o600`.

Per-server UX:

- `kilo mcp auth [name]` — run the OAuth flow for one server.
- `kilo mcp auth list` — list OAuth-capable servers and their status (`authenticated` / `expired` / `not authenticated`).
- `kilo mcp logout [name]` — remove stored credentials.
- `kilo mcp debug <name>` — diagnose auth issues.

For stdio servers, secrets should be supplied via the per-server `environment` map (or via `{env:VAR}` substitution in `command` / `headers`); process env is otherwise inherited. There is no documented env scrub for MCP subprocesses — anything in the user's shell environment is reachable by a local MCP command unless the user cleanses it manually.

## Security Model

### Trust and allowlisting

- Project config is auto-loaded without a per-workspace trust gate. Marketplace installs surface an install-dialog preview before writing.
- Managed-config directories (`/Library/Application Support/kilo/`, `/etc/kilo/`, `%ProgramData%\kilo\`) are admin-only and take the highest file-based precedence.
- macOS MDM preferences (`ai.opencode.managed` plist) are the highest-precedence configuration source overall.
- Per-tool policy lives in the `permission` engine: `<server>_<tool>` rules or `<server>_*` globs are honored everywhere MCP surfaces touch the model.
- Remote `well-known/opencode` entries may ship with `enabled: false` and require the user to opt in by name.

### Environment and sandboxing

- Each stdio MCP server receives only its `environment` map plus inherited `process.env`. There is no documented scrub equivalent to `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`.
- The experimental sandbox (`experimental.sandbox` / `experimental.sandbox_restrict_network`) applies to model-originated shell commands and first-party HTTP tools (webfetch/websearch). It is explicitly documented to **NOT** isolate local MCP servers or plugin hooks. On Linux this is `bwrap`, on macOS `sandbox-exec`; Windows has no backend.
- A stdio `command` of `docker run ...` (or `podman run ...`) is patched to inject `--rm` automatically (`ensureDockerRm`) so that stopped MCP containers do not accumulate on the host.

### Response handling

- No native MCP result sanitization is documented.
- There is no documented output-size persistence/turndown step for MCP tool responses; large outputs are passed back to the model verbatim.
- Claudine's `protect` layer should treat MCP tool results as untrusted and scan them for prompt-injection patterns.

### Credential storage

- OAuth credentials live in `mcp-auth.json` on disk, not in the OS keychain. The file is `0o600`, on a XDG-aware path inside a directory that is excluded from macOS Spotlight.
- Client secrets supplied in `oauth.clientSecret` are read at flow time; they are not written back to `kilo.json[c]` verbatim if you substitute them via `{env:VAR}`.

## Mode-Specific Behavior

### Interactive TUI / VS Code / JetBrains

- `kilo mcp add` runs an interactive wizard that prompts for scope (project vs global), name, transport, command/URL, optional OAuth details, and so on.
- `/mcps` toggles the enabled servers in the chat panel.
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
| Server fails to start (local) | Reported as `failed` in `kilo mcp list` and the `/mcps` panel; stderr is logged but not surfaced to the model |
| `tools/list` returns invalid `outputSchema` | Falls back to a tolerant schema and retries once |
| `tools/list` returns transport error | Server is marked `failed`; `kilo mcp debug <name>` can probe further |
| Remote Streamable HTTP fails first | Falls back to SSE within the `timeout` window |
| Remote returns 401 | Dynamic Client Registration or stored-token refresh attempted; if both fail, the server is marked `needs_auth` |
| Remote requires pre-registered `clientId` | Server is marked `needs_client_registration` with a JSON snippet showing how to add `clientId` to config |
| OAuth token expired | `kilo mcp auth <name>` reports "expired credentials" and re-authenticates |
| Output too large | No documented turndown behavior; the full `tools/call` payload is returned to the model |
| Project config missing | Falls back to global + remote config; `/mcps` toggle shows whatever is enabled |

## Gaps

- No documented MCP protocol version date.
- Resources and prompts have no documented Kilo-native slash command or picker; treat them as `partial` rather than `full`.
- Sampling, elicitation, and roots are not surfaced and are not documented as client capabilities.
- The `tools/list` filter pattern (e.g. excluding tools by annotation) is not documented; only server-level disabling and namespaced permission rules are.
- `kilo mcp add` is interactive only; no documented non-interactive apply path.
- The CLI docs do not state a specific default `timeout` for local vs remote servers beyond the snippet examples (10000 ms / 15000 ms); the runtime defaults documented in source are 30s for the MCP service default but the legacy docs say 5s.
- Sandbox does NOT cover local MCP servers — confirm when scoping defense-in-depth.
- Managed `mcp` schema for organization-wide allowlists is **proposed** but not shipped (see "Enterprise MCP Controls" contribution doc).
- Kilo does not currently advertise an OpenCode-style TS extension for MCP servers; MCP marketplace entries are static definitions plus `{env:VAR}` substitution only.

## Claudine Integration Notes

- Treat Kilo as `support: runtime_injection`. The strongest path is `KILO_CONFIG_CONTENT` for non-interactive use and direct `kilo.json[c]` edits via `jsonc-parser` (matching `kilo mcp add`'s own approach) for persistent setup.
- Map Claudine's normalized catalog to Kilo's `mcp` object shape: prefer the v2 schema (`type`, `command`/`url`, `environment`, `oauth`, `disabled`, `timeout`); fall back to OpenCode-form `{enabled: false}` only for entries meant solely to disable a server.
- For one-run wrappers, prefer `KILO_CONFIG_CONTENT` (with `KILO_DISABLE_PROJECT_CONFIG=1` for strict control) over mutating `.kilo/kilo.json[c]` — Claudine should construct the full effective server list itself.
- The OAuth credential store at `<Global.Path.data>/mcp-auth.json` is plain-text JSON at `0o600`. Claudine should avoid reading or writing this file and instead rely on `kilo mcp auth` for interactive OAuth setup.
- The Marketplace install dialog is the closest analog to a guided "add server" UI; Claudine does not have direct access to it but can replicate its layout when presenting a server to the user.
- Treat MCP outputs as untrusted; the experimental sandbox does not isolate stdio MCP servers, and there is no native response sanitizer.
- Surfacing resources and prompts to the model is partially supported through the SDK but no Kilo-native UI is documented; expect to expose them via VS Code / JetBrains fronts rather than the CLI.
- Kilo CLI is a fork of OpenCode, so most Claudine code that handles OpenCode config (`OPENCODE_CONFIG`, `OPENCODE_CONFIG_CONTENT`) applies with `KILO_*` renames.
- Because `kilo mcp add` is interactive-only, Claudine should not call it from non-TTY contexts; use `jsonc-parser` edits to `kilo.jsonc` directly and rely on the user running `kilo mcp auth` themselves when OAuth is needed.

## Sources

- [Kilo Code MCP overview](https://kilocode.ai/docs/features/mcp/overview)
- [Using MCP in Kilo Code](https://kilocode.ai/docs/features/mcp/using-in-kilo-code)
- [MCP server transports (STDIO/SSE)](https://kilocode.ai/docs/features/mcp/server-transports)
- [What is MCP](https://kilocode.ai/docs/features/mcp/what-is-mcp)
- [MCP vs API](https://kilocode.ai/docs/features/mcp/mcp-vs-api)
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
- [`packages/opencode/src/cli/cmd/mcp.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/cli/cmd/mcp.ts) — `kilo mcp` source
- [`packages/opencode/src/mcp/index.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/mcp/index.ts) — runtime MCP service
- [`packages/opencode/src/mcp/auth.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/mcp/auth.ts) — OAuth token store
- [`packages/opencode/src/config/config.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/config.ts) — config precedence
- [`packages/opencode/src/config/managed.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/config/managed.ts) — managed-config paths
- [`packages/core/src/config/mcp.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/config/mcp.ts) — Effect v2 MCP schema
- [`packages/core/src/flag/flag.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/flag/flag.ts) — `KILO_*` env vars
- [`packages/core/src/global.ts`](https://github.com/Kilo-Org/kilocode/blob/main/packages/core/src/global.ts) — XDG paths
- [Enterprise MCP Controls (proposal)](https://kilocode.ai/docs/contributing/features/enterprise-mcp-controls)
