---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://moonshotai.github.io/kimi-code/en/customization/mcp.md
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http, http_sse, sse]
  lifecycle: |
    MCP servers are discovered at session startup. Stdio servers are spawned as
    local subprocesses; HTTP servers open streamable-HTTP connections; SSE
    servers open legacy HTTP+SSE connections. Connection happens after the TUI
    shell UI starts so the interface is usable immediately; the welcome panel
    and `/mcp` slash command show live connection status. Servers that fail to
    connect are reported as not-ready in `/mcp` and the welcome panel. The
    public docs do not describe automatic reconnection or mid-session
    `list_changed` refresh semantics; one changelog entry mentions "runtime
    support for dynamic MCP server updates" (0.14.1, 2026-06-12) but the
    specifics of `list_changed` are not documented.
  notes: |
    The provider documents three transport kinds: `stdio`, `http` (streamable
    HTTP), and `sse` (legacy HTTP+SSE). Streamable HTTP and SSE are
    deliberately distinct from one another — the doc says "Prefer HTTP for new
    MCP servers, but use `transport: 'sse'` when a service still exposes only
    the older SSE transport." An explicit MCP protocol version date is not
    stated. The CLI is the Node.js-native successor to the Python-based
    `kimi-cli`; the legacy `kimi-cli` documentation remains online at
    `moonshotai.github.io/kimi-cli/`. Migration from `kimi-cli` to the current
    product moves MCP servers, config, sessions, and history; OAuth tokens and
    MCP service authorizations are NOT migrated (re-authorization is required).
config_files:
  - os: macos
    scope: user
    path: "~/.kimi-code/mcp.json"
    format: json
    notes: |
      User-scope MCP server definitions. May be relocated by setting
      `KIMI_CODE_HOME`. Stored as the standard `mcpServers` object. Merged on
      startup with any project-level `.kimi-code/mcp.json` — the project-level
      entry takes precedence and overrides the user-level entry when names
      collide.
  - os: linux
    scope: user
    path: "~/.kimi-code/mcp.json"
    format: json
    notes: "Same user-level layout as macOS."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.kimi-code\\mcp.json"
    format: json
    notes: "Same user-level layout as macOS/Linux (Windows path syntax)."
  - os: macos
    scope: repo
    path: ".kimi-code/mcp.json"
    format: json
    notes: |
      Project-scope MCP server definitions, effective only for the current
      repository. Same JSON shape as the user-level file. Recommended for
      server definitions shared across a team or repo (e.g., a docs crawler or
      CI helper). Note the doc warning: "stdio entries in a project-level
      `.kimi-code/mcp.json` execute local commands when a session starts. Only
      enable these in repositories you trust."
  - os: linux
    scope: repo
    path: ".kimi-code/mcp.json"
    format: json
    notes: "Project-scope MCP server definitions — same layout as macOS."
  - os: windows
    scope: repo
    path: ".kimi-code\\mcp.json"
    format: json
    notes: "Project-scope MCP server definitions — same layout as macOS/Linux."
  - os: macos
    scope: plugin
    path: "<plugin-root>/kimi.plugin.json (mcpServers field)"
    format: json
    notes: |
      Plugins can declare `mcpServers` in their manifest. Servers declared by a
      plugin are enabled by default; disable via `/plugins mcp disable <id>
      <server>` followed by `/reload`. `command` may be a PATH command or a
      path starting with `./` within the plugin root; `cwd` likewise must start
      with `./` and be within the plugin root, otherwise the server is
      ignored.
  - os: linux
    scope: plugin
    path: "<plugin-root>/kimi.plugin.json (mcpServers field)"
    format: json
    notes: "Plugin manifest MCP declarations — same shape as macOS."
  - os: windows
    scope: plugin
    path: "<plugin-root>\\kimi.plugin.json (mcpServers field)"
    format: json
    notes: "Plugin manifest MCP declarations — same shape as macOS/Linux."
cli_params:
  - flag: "/mcp"
    description: "In interactive TUI mode, list MCP servers and their per-session connection status (loaded in idle state)."
    example: "/mcp"
  - flag: "/mcp-config"
    description: "Built-in skill slash command. Interactively add, edit, or delete MCP server declarations without manually editing `mcp.json`; also handles MCP OAuth login. Available in the TUI idle state."
    example: "/mcp-config"
  - flag: "/mcp-config login <server-name>"
    description: "Complete a browser-based OAuth flow for a configured HTTP/SSE MCP server."
    example: "/mcp-config login linear"
  - flag: "/plugins mcp enable <id> <server>"
    description: "Enable an MCP server that was declared by a plugin; reload required."
    example: "/plugins mcp enable kimi-finance finance"
  - flag: "/plugins mcp disable <id> <server>"
    description: "Disable an MCP server that was declared by a plugin; reload required."
    example: "/plugins mcp disable kimi-finance finance"
  - flag: "kimi migrate"
    description: "Migrate config, MCP servers, input history, and sessions from a legacy `kimi-cli` installation. OAuth tokens are not migrated (re-authorization required)."
    example: "kimi migrate"
env_vars:
  - name: KIMI_CODE_HOME
    effect: |
      Overrides the Kimi Code data directory (default `~/.kimi-code`),
      relocating `mcp.json`, the `credentials/mcp/` directory, plugin installs,
      and every other piece of runtime state. Multiple `kimi` instances sharing
      the same `KIMI_CODE_HOME` will share config and credential files.
  - name: KIMI_CODE_NO_AUTO_UPDATE
    effect: |
      Disable the update preflight; legacy alias `KIMI_CLI_NO_AUTO_UPDATE` is
      also recognized. Not MCP-specific but the migrator that flips legacy
      `kimi-cli` values into the new names honours the old key.
  - name: HTTP_PROXY / HTTPS_PROXY / ALL_PROXY / NO_PROXY
    effect: |
      Standard proxy variables honored by all outbound traffic — model API
      calls, MCP servers, web tools, telemetry, sign-in, and update checks.
      Loopback hosts always bypass the proxy, so a localhost MCP server keeps
      working. Stdio MCP servers that run as Node child processes additionally
      honor `NODE_USE_ENV_PROXY` on Node 22.21+ / 24.5+; SOCKS proxying
      applies only to the CLI's own traffic.
server_schema:
  transports: ["stdio", "http", "sse"]
  command_fields: ["command", "args", "env", "cwd"]
  http_fields: ["url", "transport", "headers", "bearerTokenEnvVar"]
  env_shape: |
    `env` is an object mapping variable names to string values. It injects
    environment variables into the stdio child process and overlays the
    inherited environment. Shell-style variable expansion is NOT documented —
    use `bearerTokenEnvVar` to pull a bearer token from a shell environment
    variable by name; for other secrets, hard-code values or set the parent
    environment before launching `kimi`.
  auth_shape: |
    HTTP and SSE servers support two credential forms: a static `headers`
    object attached to every request, or `bearerTokenEnvVar` — the name of a
    shell environment variable that contains a bearer token. OAuth is
    handled through the `/mcp-config login <server-name>` interaction; tokens
    are cached under `$KIMI_CODE_HOME/credentials/mcp/<key>-<suffix>.json`
    (legacy kimi-cli stored them under `~/.kimi/mcp-oauth/`).
  notes: |
    Server id is the map key under `mcpServers`. Entries with a `command`
    field are stdio servers; entries with a `url` field and no `transport`
    are HTTP servers; for legacy SSE, set `transport` to `"sse"`. Additional
    optional fields documented for every server: `enabled` (boolean — disable
    the server), `startupTimeoutMs` (number — connection timeout, default
    `30000`), `toolTimeoutMs` (number — per-tool call timeout), `enabledTools`
    (string array — allowlist of tool names), `disabledTools` (string array —
    blocklist of tool names), and per-kind fields `env`/`cwd` for stdio,
    `headers`/`bearerTokenEnvVar` for HTTP/SSE.
server_capabilities:
  tools: full
  resources: unknown
  prompts: unknown
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    Tools are surfaced to the model with the callable name format
    `mcp__<server>__<tool>` and participate in the regular permission model.
    The runtime changelog mentions "runtime support for dynamic MCP server
    updates" (0.14.1) without specifying whether `list_changed` notifications
    are honored, so treat that capability as "unknown" until documentation
    describes it. Resources and prompts are NOT described as surfaced
    surfaces in any kimi-code or kimi-cli MCP page; treat them as unknown
    rather than absent.
tool_surface:
  discovery: |
    `tools/list` is called at server connect time. Welcome panel and `/mcp`
    show per-server connection status and tool counts. Each MCP server is
    also gated by `startupTimeoutMs` (default 30000ms).
  filtering: |
    Per-server include/exclude at the tool level via `enabledTools` /
    `disabledTools` arrays in the server definition. Per-tool approval at the
    permission-system level via `permission.rules` patterns in `config.toml`
    (e.g., `mcp__github__*` matches all tools under that server, individual
    tools via `mcp__<server>__<tool>`); the provider also accepts `*` and `**`
    wildcards. MCP tool parameters are not included in permission matching —
    only tool names match.
  approval: |
    MCP tool calls use the same approval mechanism as native tools. Calls that
    do not match any permission rule trigger an approval request; selecting
    "Approve for this session" in the approval dialog allows subsequent calls
    of the same kind within the current session. Permission rules persist in
    `[[permission.rules]]` tables of `config.toml`.
  result_handling: |
    Tool output is returned to the model via the normal tool result flow;
    the docs do not describe result sanitization, truncation, or
    prompt-injection filtering beyond what the model API platform does.
    Per-tool wall-clock timeout is `toolTimeoutMs` (no default value
    documented).
  annotations_trusted: |
    The docs do not discuss tool annotations (`title`, `readOnlyHint`,
    `destructiveHint`, etc.). Treat them as ignored unless explicit support
    is documented in the future.
  notes: |
    Plugins can enable or disable MCP servers declared in their manifest via
    `/plugins mcp enable|disable <id> <server>`; a `/reload` is required
    afterwards.
resource_surface:
  supported: false
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    Resources are not documented as a surfaced feature. The public docs
    describe tools, permission rules, and OAuth for HTTP/SSE transports only.
  notes: |
    Resources are not described in either kimi-code or kimi-cli MCP
    documentation; no `resources/list`, `resources/read`,
    `resources/subscribe`, or template UI is mentioned. Marked `supported: false`
    here because there is no positive evidence of support; treat as
    `unknown` in the body if a newer revision adds `resources/list` /
    `resources/subscribe` behavior.
prompt_surface:
  supported: false
  invocation: ""
  arguments: ""
  exposure_model: |
    MCP prompts are not documented as a surfaced feature. Slash commands in
    the TUI (`/mcp`, `/mcp-config`, `/plugins mcp …`) are Kimi-native; none of
    them exposes MCP `prompts/list` content.
  notes: |
    Prompts are not described in either kimi-code or kimi-cli MCP
    documentation. Marked `supported: false` here because there is no positive
    evidence of support; treat as `unknown` in the body if a newer revision
    adds a prompts surface. There is no mention of `prompts/list_changed` in
    the changelog.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: replace
  notes: |
    Claudine can read `$KIMI_CODE_HOME/mcp.json` (default
    `~/.kimi-code/mcp.json`) and the project-level `.kimi-code/mcp.json`,
    normalize server definitions into its catalog, and write them back.
    Project-level entries override user-level entries with the same name
    (replace-per-server, no field-level merge). Plugins may also contribute
    `mcpServers` from their manifest; the `apply` path is `/mcp-config` for
    interactive TUI use, the `kimi migrate` command for importing from
    `kimi-cli`, and direct file edits otherwise.
runtime_injection:
  supported: false
  mechanism: |
    No documented mechanism. The kimi-code CLI does NOT expose
    `--mcp-config-file`, `--mcp-config`, or any equivalent runtime-injection
    flag (verified against `kimi-command` and `data-locations` pages). The
    closest non-mutating alternative is to write a per-invocation project-
    level `.kimi-code/mcp.json` in a scratch working directory and launch
    `kimi` from there — but this is a file mutation, not in-memory injection.
  limitations: |
    Even the project-level file approach mutates persistent filesystem state
    in the working directory. There is no documented environment variable or
    inline-config mechanism that injects servers without writing
    `mcp.json`. Plugin-installed MCP servers change behavior globally because
    they live in the user-level directory. OAuth flows require a TUI login
    step (`/mcp-config login <name>`), so unauthenticated `--yolo` /
    `--print` runs cannot bootstrap OAuth servers.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens for MCP servers live in
    `$KIMI_CODE_HOME/credentials/mcp/<key>-<suffix>.json` (per the data-
    locations page). The credentials directory has permissions
    `0700`/`0600` (read/write only by the current user). Managed provider
    credentials are stored as `credentials/<name>.json`. The legacy `kimi-cli`
    stored MCP tokens under `~/.kimi/mcp-oauth/`, and the migration does not
    carry those across.
  token_scope: |
    Per-MCP-server name; `/mcp-config login <server-name>` opens the
    interactive browser flow; clearing requires deleting the file under
    `credentials/mcp/` (`/logout` does not clear MCP credentials, unlike
    provider credentials).
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` object in `mcp.json`
    when the entry uses `command`, or via `bearerTokenEnvVar` (HTTP/SSE only)
    which names a shell environment variable whose value becomes the bearer
    token. There is no documented shell-style expansion (`${VAR}`) for other
    fields.
  notes: |
    Static API-key headers live in `mcp.json` (HTTP/SSE only); OAuth tokens
    are kept outside the config file in `credentials/mcp/`. The provider does
    not document callback-port pinning, DCR, or discovery metadata overrides
    for MCP OAuth — the standard `/mcp-config login` flow is the only
    documented entry point.
security:
  tool_filtering: |
    Per-tool include and exclude via `enabledTools` and `disabledTools` on
    each server definition. Per-tool or per-server approval via
    `permission.rules` in `config.toml` with patterns such as `mcp__<server>__*`
    or `mcp__<server>__<tool>`. Plugins disable their own MCP servers via
    `/plugins mcp disable`. No server allowlist or denylist at the
    provider-wide level is documented; users rely on per-tool approval and
    inspect server names at approval time.
  server_trust: |
    Project-level `.kimi-code/mcp.json` requires no extra approval gate — the
    docs warn "Only enable these in repositories you trust" but do not
    describe a workspace-trust dialog or a project file approval prompt.
    Plugin-bundled MCP servers can be disabled from `/plugins`. There is no
    documented managed policy or admin-config layer for kimi-code.
  env_sanitization: |
    Stdio servers inherit the provider process environment plus any explicit
    `env` map. No credential-scrubbing behavior is documented; secrets placed
    directly in `env` will be visible to the child process. The HTTP/SSE
    `bearerTokenEnvVar` indirection keeps bearer tokens out of the config
    file but they are still readable by anyone with shell access.
  sandbox_interaction: |
    MCP servers run as ordinary local processes and are not isolated by any
    documented sandbox. No container or OS-level boundary is described.
  response_filtering: |
    The docs do not describe scanning or filtering of MCP tool results for
    prompt-injection patterns. Tool return content is passed to the model in
    the standard tool-result envelope.
  notes: |
    YOLO mode (`--yolo` or `/yolo`) and Auto mode (`--auto` or `/auto`)
    bypass approval prompts for MCP tool calls the same way they do for native
    tools. Web-mode and ACP-mode behavior around MCP permissions is not
    documented separately; treat ACP as inheriting TUI behavior.
gaps:
  - |
    Explicit MCP protocol version date is not stated.
  - |
    Resources and prompts are not described as surfaced surfaces; the
    documentation only discusses tools. Treat both as `unknown` rather than
    `none`.
  - |
    Roots, sampling, and elicitation support are not described in the kimi-
    code MCP docs. Treat all three as `unknown`.
  - |
    Behavior on `list_changed` notifications (tools, prompts, resources) is
    not documented. The 0.14.1 changelog mentions "runtime support for
    dynamic MCP server updates" but does not specify which list_changed
    signals are honored.
  - |
    Reconnection behavior for failed stdio / HTTP / SSE servers is not
    described; there is no documented retry budget or backoff strategy.
  - |
    Tool annotation handling (`title`, `readOnlyHint`, `destructiveHint`,
    `openWorldHint`) is not documented.
  - |
    OAuth flow details — Dynamic Client Registration, callback-port pinning,
    metadata discovery — are not described for the `/mcp-config login` flow.
  - |
    Web-mode, ACP-mode, and non-interactive `--print` mode behaviors around
    MCP approval are not separately documented.
changes:
  - "Switched primary references from `kimi-cli` (Python/uv, deprecated upstream) to `kimi-code` (Node.js native, current upstream). Prior research was based on `https://moonshotai.github.io/kimi-cli/`; updated to `https://moonshotai.github.io/kimi-code/en/customization/mcp.md`."
  - "Transports expanded from `stdio` + `streamable_http` to `stdio`, `http` (streamable HTTP), and `sse` (legacy HTTP+SSE); the kimi-code docs treat HTTP and SSE as deliberately distinct."
  - "Config files restructured: user-level `~/.kimi-code/mcp.json` and project-level `.kimi-code/mcp.json` (working-directory-local), versus the prior single user-level `~/.kimi/mcp.json`."
  - "Removed the documented `kimi mcp add/list/remove/auth/reset-auth/test` subcommands: kimi-code does not ship a `kimi mcp` subcommand (verified against `kimi --help`); management moves into the TUI via `/mcp` (view status) and `/mcp-config` (interactive add/edit/delete + OAuth login)."
  - "Removed the documented `--mcp-config-file` and `--mcp-config` CLI flags: kimi-code does not provide them (verified against the `kimi-command` reference and `kimi --help` output). `runtime_injection.supported` is now `false`."
  - "Server schema added six optional fields that the provider documents for every server entry: `enabled` (boolean), `startupTimeoutMs` (number, default 30000), `toolTimeoutMs` (number), `enabledTools` (string array — tool allowlist), `disabledTools` (string array — tool blocklist), and stdio-only `cwd` (string) plus HTTP/SSE-only `bearerTokenEnvVar` (string — name of a shell env var carrying a bearer token)."
  - "Tool naming convention documented: MCP tools are called as `mcp__<server>__<tool>`, with `*` and `**` wildcards accepted in `permission.rules` patterns in `config.toml`. Permission rules support `decision ∈ {allow, deny, ask}` with `scope ∈ {turn-override, session-runtime, project, user}`."
  - "Authorization storage moved from `~/.kimi/mcp-oauth/` (legacy kimi-cli) to `$KIMI_CODE_HOME/credentials/mcp/<key>-<suffix>.json` (kimi-code, default `~/.kimi-code/credentials/mcp/`); the migrator does NOT carry tokens across — re-authorization is required post-migration."
  - "Env vars: `KIMI_SHARE_DIR` (legacy kimi-cli) replaced by `KIMI_CODE_HOME` (kimi-code). Added the standard proxy variables (`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY`) — these are honored for MCP traffic and are scoped via `NODE_USE_ENV_PROXY` for stdio Node child processes."
  - "Plugin integration: kimi-code plugins can declare `mcpServers` in their manifest (file `kimi.plugin.json`); these load on plugin enable, run with `KIMI_CODE_HOME` and `KIMI_PLUGIN_ROOT` set, can be disabled via `/plugins mcp disable <id> <server>`, and have no equivalent in the legacy kimi-cli."
  - "Migration added: `kimi migrate` migrates `kimi-cli` config, MCP servers, sessions, and history into kimi-code; OAuth tokens and MCP service authorizations are NOT migrated."
  - "Removed `auto` approval annotation (the prior `requirements_claudine_update: true` block referenced an unsupported Claude Code path); corrected `requires_claudine_update` to `true` again because the kimi-code config layer in Claudine should track the new `~/.kimi-code/mcp.json` + `.kimi-code/mcp.json` paths, the plugin `mcpServers` surface, and the absence of `--mcp-config` injection."
requires_claudine_update: true
reason: |
  Four Claudine behaviors are now provable rather than guessed: kimi-code is a
  two-config-file provider (user + project) rather than single user-level;
  auth lives at `credentials/mcp/` and the migrator does not bridge from
  `~/.kimi/mcp-oauth/`; runtime injection via `--mcp-config-file` /
  `--mcp-config` does not exist on kimi-code (the prior research treated
  `runtime_injection` as `true` — it must flip to `false`, with the closest
  non-mutating alternative being a per-run project-level file); and plugins
  contribute MCP server declarations through their manifest (`mcpServers`
  field in `kimi.plugin.json`) — a new surface that Claudine's plugin-aware
  sync should expose. The catalog and config_files list should also be
  retargeted to the `~/.kimi-code` paths (respecting `KIMI_CODE_HOME`); the
  security/review view must treat `enabled` and `enabledTools`/`disabledTools`
  as first-class fields rather than guessing; the resource/prompt surfaces
  should be marked `unknown` rather than `none` to avoid instructing
  downstream consumers to deny resources they may not have tested; and the
  mode-specific failure / `list_changed` handling must be marked `unknown`
  rather than reported as a clean lifecycle.
---

# MCP Support in Kimi Code CLI

## Overview

Kimi Code CLI supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/)
as a tool-extension mechanism alongside its built-in tools (`Read`, `Bash`,
`Grep`, …). For Claudine, the strongest integration path is **`import_sync`**:
two persistent config files — user-level `~/.kimi-code/mcp.json` and
project-level `.kimi-code/mcp.json` — can be read and normalized into
Claudine's MCP catalog, written back as a whole, and managed interactively
via the `/mcp` and `/mcp-config` slash commands. There is **no documented
runtime-injection flag**, so one-run wrappers must write a per-run project
file or accept that they are mutating persistent state.

> **Note:** The Kimi team wound down the prior `kimi-cli` (Python/uv) product
> in favor of [Kimi Code](https://moonshotai.github.io/kimi-code/) — a Node.js
> native binary that also exposes a comprehensive MCP surface. This research
> reflects the live kimi-code docs; the legacy `kimi-cli` material is treated
> as background only.

Surface inventory (one-line):

- **Tools** — exposed: `mcp__<server>__<tool>` callable names; per-tool
  include/exclude via `enabledTools` / `disabledTools` on each server
  definition; permission rules via `permission.rules` in `config.toml`.
- **Resources** — unknown: the docs only describe tools. Treat as
  unverified rather than absent.
- **Prompts** — unknown: the docs only describe tools. Treat as
  unverified rather than absent.
- **Roots** — unknown: not described in the kimi-code MCP docs.
- **Sampling** — unknown: not described.
- **Elicitation** — unknown: not described.
- **Channels (vendor extension)** — not applicable; this is a Kimi-specific
  surface that the MCP docs do not mention.

## Protocol and Transports

Kimi Code CLI accepts three documented MCP transports:

| Transport | `mcp.json` shape | Notes |
| :-------- | :--------------- | :---- |
| `stdio` | `{ "command": "…", "args": [ … ], "env": { … }, "cwd": "…" }` | Local subprocess server. |
| `http` (streamable HTTP) | `{ "url": "https://…", "headers": { … }, "bearerTokenEnvVar": "…" }` | Remote server over streamable HTTP. Default for `url` entries without a `transport` field. |
| `sse` (legacy) | `{ "url": "https://…", "transport": "sse", "headers": { … } }` | Legacy HTTP+SSE endpoint. The docs explicitly say "Prefer HTTP for new MCP servers, but use `transport: 'sse'` when a service still exposes only the older SSE transport." |

The documentation does not name an explicit MCP protocol version date.
WebSocket transports are not described; this is interesting because the
kimi-code changelog mentions "runtime support for dynamic MCP server updates"
(0.14.1, 2026-06-12), but the specifics — particularly whether `list_changed`
notifications are honored and whether servers are reconnected if they crash —
are not described.

Lifecycle behavior:

- `tools/list` is called at server connect time, after the TUI shell UI is
  usable.
- Per-server connection timeout is `startupTimeoutMs` (default 30000ms).
- The welcome panel and `/mcp` slash command show live connection status.
- Failed servers stay in a not-ready state in `/mcp`; automatic
  reconnection is not documented.

## Configuration

MCP servers live in two locations (per OS):

- User-scope: `$KIMI_CODE_HOME/mcp.json` (default `~/.kimi-code/mcp.json`,
  `%USERPROFILE%\.kimi-code\mcp.json` on Windows).
- Repo-scope: `./.kimi-code/mcp.json` in the working directory.

Both files share the same JSON shape. When the same server name is defined in
both, the project-level entry wins entirely (per-server replace, no
field-level merge). The base data directory is relocated with
`KIMI_CODE_HOME`; multiple instances sharing that directory will share
config and credentials.

Example (project-level):

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
      "env": { "SOME_VAR": "value" },
      "cwd": "/srv/scratch"
    },
    "linear": {
      "url": "https://mcp.linear.app/mcp"
    },
    "legacy-events": {
      "transport": "sse",
      "url": "https://mcp.example.com/sse",
      "bearerTokenEnvVar": "LEGACY_EVENTS_TOKEN"
    }
  }
}
```

> **Warning (from the docs):** "stdio entries in a project-level
> `.kimi-code/mcp.json` execute local commands when a session starts. Only
> enable these in repositories you trust."

## Server Definition Shape

A server definition in either `mcp.json` file looks like:

```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "${GITHUB_TOKEN}" },
      "cwd": "/srv/work",
      "enabled": true,
      "startupTimeoutMs": 30000,
      "toolTimeoutMs": 60000,
      "enabledTools": ["create_issue", "search_issues"],
      "disabledTools": ["delete_repo"]
    },
    "docs": {
      "url": "https://mcp.docs.example.com/mcp",
      "headers": { "X-Org": "kimi" },
      "bearerTokenEnvVar": "DOCS_TOKEN",
      "startupTimeoutMs": 30000,
      "toolTimeoutMs": 30000,
      "disabledTools": ["purge"]
    }
  }
}
```

### Field reference

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `command` | stdio | Executable to spawn |
| `args` | stdio | Argument array |
| `env` | stdio | Map of environment variables overlaid on the inherited environment |
| `cwd` | stdio | Working directory for the server process |
| `url` | http, sse | Endpoint URL |
| `transport` | http, sse | Set to `"sse"` to force the legacy SSE transport; otherwise the entry is streamable HTTP |
| `headers` | http, sse | Static request headers appended to every request |
| `bearerTokenEnvVar` | http, sse | Name of a shell environment variable that contains the bearer token |
| `enabled` | all | Set to `false` to disable this server |
| `startupTimeoutMs` | all | Connection timeout in milliseconds; default `30000` |
| `toolTimeoutMs` | all | Per-tool-call timeout in milliseconds |
| `enabledTools` | all | Allowlist of tool names from this server |
| `disabledTools` | all | Blocklist of tool names from this server |

The `mcp.json` shape is shared with plugin manifests (`mcpServers` in
`kimi.plugin.json`); the same fields apply.

## Tools, Resources, and Prompts

### Tools

MCP tools are surfaced to the model with callable names of the form
`mcp__<server>__<tool>`. Tools are discovered at server connect time
(`tools/list`); the welcome panel and `/mcp` slash command show counts and
connection status.

Per-tool filtering is applied via the `enabledTools` and `disabledTools`
arrays on each server definition (when both are set, the docs are silent on
precedence — assume `disabledTools` wins as with most providers). The
provider-level permission system, configured under
`[[permission.rules]]` in `config.toml`, supports patterns such as:

```toml
[[permission.rules]]
decision = "allow"
pattern = "mcp__github__*"

[[permission.rules]]
decision = "deny"
pattern = "mcp__filesystem__write_file"

[[permission.rules]]
decision = "ask"
pattern = "mcp__filesystem__*"
```

Patterns use the tool name only; `MCP tool parameters are not included in
permission matching`. The pattern accepts `*` and `**` wildcards; the schema
also supports `scope ∈ {turn-override, session-runtime, project, user}` and
`decision ∈ {allow, deny, ask}`.

An MCP tool call that does not match a permission rule triggers an approval
prompt; selecting "Approve for this session" allows subsequent calls of the
same kind within the current session. Per-tool wall-clock timeout is
`toolTimeoutMs`. There is no documented prompt-injection scanner on the
result side; tool output is delivered to the model in the standard
tool-result envelope.

### Resources and prompts

The kimi-code MCP documentation only describes tools; resources and prompts
are not described as surfaced surfaces. The provider does not expose an MCP
`resources/list`-aware UI in `/mcp`, does not document template URIs, and
does not advertise subscriptions. Treat both as **unknown** rather than
absent — they may exist but are not described.

## Roots, Sampling, and Elicitation

The kimi-code MCP documentation does not describe `roots/list`,
`sampling/createMessage`, or elicitation. Claudine should treat all three
as **unknown** for this provider.

> **Note:** The kimi-code `Bash` tool sandbox (where applicable) constrains
> shell commands, but MCP servers are not described as inheriting any
> filesystem boundary from it.

## Import, Export, and Sync

Claudine can treat Kimi Code CLI as an `import_sync` provider:

- **Import:** read `$KIMI_CODE_HOME/mcp.json` (default
  `~/.kimi-code/mcp.json`) and project-level `.kimi-code/mcp.json`,
  normalize server definitions into Claudine's MCP catalog, and surface
  plugin-bundled `mcpServers` from `kimi.plugin.json` manifests as a
  third source.
- **Export:** write provider-shaped JSON back to either file. The catalog
  should preserve unknown fields rather than overwrite them, since the
  schema evolves (e.g., `bearerTokenEnvVar` and `enabledTools` are recent).
- **Apply:** use the in-TUI `/mcp-config` command for interactive mutation;
  use direct file edits for scripted mutation.

Merge semantics are per-name, per-scope, replace-whole — there is no
field-level merge across the user-level and project-level files. Plugin-
bundled servers are managed through `/plugins mcp enable|disable <id>
<server>` and are stored separately in `$KIMI_CODE_HOME/plugins/`.

## Runtime Injection

Runtime injection is **not supported** on Kimi Code CLI: there is no
`--mcp-config-file`, no `--mcp-config` flag, and no equivalent CLI surface
that would let a wrapper hand MCP definitions to one invocation without
persistent state. Verified against the `kimi-command` reference and
`kimi --help` output, which lists `--skills-dir`, `--add-dir`, `--yolo`,
`--auto`, `--plan`, `--prompt`, `--output-format`, `--session`,
`--continue`, `--model`, plus the subcommands `login` / `acp` / `server` /
`web` / `doctor` / `export` / `migrate` / `upgrade` / `provider` / `vis`. No
MCP-runtime flag appears.

The closest non-mutating alternative is to write a per-invocation
project-level `.kimi-code/mcp.json` in a scratch working directory and
launch `kimi` from that directory. This:

- mutates persistent filesystem state in the working directory,
- requires the wrapper to chdir or use `--work-dir`,
- still requires `/mcp-config login <name>` for OAuth servers — OAuth
  cannot complete in a non-interactive `--print` / `--yolo` run,
- may collide with a real project-level file if the user has one.

For one-run wrappers, this is the only documented alternative and it is not
safe for ephemeral use.

## Authorization and Credentials

- **Static headers:** stored in `mcp.json` under `headers` (HTTP/SSE only).
- **Bearer via env var:** `bearerTokenEnvVar` names a shell environment
  variable whose value becomes the bearer token (HTTP/SSE only). This
  keeps bearer tokens out of the config file but they remain readable by
  anyone with shell access.
- **OAuth:** interactive browser flow via `/mcp-config login <name>`;
  tokens are cached in
  `$KIMI_CODE_HOME/credentials/mcp/<key>-<suffix>.json` (default
  `~/.kimi-code/credentials/mcp/`). The credentials directory has
  permissions 0700/0600 (read/write by the current user only). `/logout`
  does NOT clear MCP credentials — delete the file under `credentials/mcp/`
  for that.
- **Stdio secrets:** delivered through the per-server `env` map. Secrets
  placed directly in `env` are visible to the child process and to anyone
  reading the file. No documented `${VAR}` shell expansion exists for
  other fields.

OAuth DCR, callback-port pinning, and metadata discovery overrides are not
described for the `/mcp-config login` flow.

> **Migration note:** Tokens from a legacy `kimi-cli` install were stored
> under `~/.kimi/mcp-oauth/`. `kimi migrate` does NOT carry OAuth or
> MCP-authorization tokens across; re-authorization via `/mcp-config login
> <name>` is required after migration.

## Security Model

- **Tool filtering:** `enabledTools` / `disabledTools` per server, plus
  `permission.rules` patterns in `config.toml`. No provider-wide allowlist
  or denylist of MCP servers is documented.
- **Server trust:** project-level `.kimi-code/mcp.json` does not require
  a workspace-trust approval (unlike Claude Code); the docs simply warn
  "Only enable these in repositories you trust." Plugin-bundled servers
  can be disabled in `/plugins`. Managed/administrative policy is not
  described.
- **Env sanitization:** stdio servers inherit the provider process
  environment plus any explicit `env`. The provider does not document
  scrubbing of credentials or isolation of MCP subprocesses.
- **Sandbox interaction:** MCP servers are ordinary local processes; no
  container or OS-level sandbox is described.
- **Response filtering:** no documented prompt-injection scan or result
  sanitization for MCP tool output.
- **Approval:** `--yolo` / `/yolo` and `--auto` / `/auto` auto-approve
  MCP tool calls exactly as they do for native tools; pre-approved
  permission rules still apply.
- **ACP and Web mode:** ACP behavior is not separately documented; Web
  mode (`kimi web`, `/web`) hosts the same TUI inside a browser, so MCP
  behavior is presumed identical.

## Mode-Specific Behavior

### Interactive TUI

- `/mcp` shows connected servers and tool counts.
- `/mcp-config` opens the interactive server editor and the OAuth login
  flow.
- `/plugins mcp enable|disable <id> <server>` enables/disables plugin
  servers (requires `/reload` afterwards).
- Project-level `.kimi-code/mcp.json` requires no approval dialog — it is
  loaded at startup with the same trust posture as the rest of the project.

### Non-interactive / print mode (`-p`, `--print`)

- `--auto` is implied for `-p` mode; MCP tool calls are auto-approved just
  like native ones.
- OAuth flows cannot complete (no interactive browser); OAuth servers
  remain unauthenticated and their tools are unavailable unless a token
  is pre-seeded via `bearerTokenEnvVar` or static headers.
- There is no `--mcp-config` / `--mcp-config-file` equivalent; injection
  must go through the file system.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Server fails to start | Marked not-ready in `/mcp` and the welcome panel; no documented retry budget. |
| OAuth not completed | Server listed as unauthenticated in `/mcp`; tool calls fail at call time. |
| `toolTimeoutMs` exceeded | Tool call returns a timeout error from the client; specifics not documented. |
| Server URL invalid | `--mcp-config` / `startupTimeoutMs` (default 30000ms) elapses; server marked not-ready. |
| Project `mcp.json` malformed | Server fails to load — specifics not documented; check `kimi doctor` for config validation. |
| Legacy `~/.kimi/mcp-oauth/` token reused after `kimi migrate` | Token is NOT migrated; re-run `/mcp-config login <name>`. |

## Gaps

See the frontmatter `gaps` list for the full set. Major unverified areas:

- Resources and prompts are `unknown` (not `none`); the docs only describe
  tools.
- `list_changed` behavior is undocumented despite a 2026-06-12 changelog
  note.
- Reconnection, retry, and idle-timeout behavior are not documented.
- Tool annotation handling is not described.
- Roots, sampling, and elicitation are not described.

## Claudine Integration Notes

- Treat Kimi Code CLI as `support: import_sync`. Map the catalog to the
  server definition shape including the new optional fields (`enabled`,
  `enabledTools`, `disabledTools`, `startupTimeoutMs`, `toolTimeoutMs`,
  `cwd`, `bearerTokenEnvVar`); preserve unknown fields on round-trip.
- Read both `$KIMI_CODE_HOME/mcp.json` (default `~/.kimi-code/mcp.json`)
  and project-level `.kimi-code/mcp.json`. Honor `KIMI_CODE_HOME`.
- Read plugin `mcpServers` from `kimi.plugin.json` manifests (or the legacy
  `.kimi-plugin/plugin.json` location) as a third sync source, surfaced
  via `plugins` scope with `apply_supported: false` (the apply path is
  `/plugins mcp enable|disable`).
- Apply changes through `/mcp-config` (interactive) or direct file edits
  (scripted). The legacy `kimi mcp add` command does not exist on
  kimi-code.
- Do NOT assume a `--mcp-config` or `--mcp-config-file` runtime-injection
  flag — none exists. One-run wrappers must accept the trade-off of
  writing a project-level file.
- Do NOT assume OAuth can complete in non-interactive mode; pre-seed
  bearer tokens via `bearerTokenEnvVar` or static `headers` for
  automated runs.
- Treat resources and prompts as `unknown` (not `none`) so downstream
  consumers do not assume the surface is permanently absent.
- Defensively scan MCP tool results in Claudine's `protect` layer;
  Kimi Code does not provide native response sanitization.

## Sources

- [Kimi Code: Model Context Protocol](https://moonshotai.github.io/kimi-code/en/customization/mcp.md)
- [Kimi Code: `kimi` Command](https://moonshotai.github.io/kimi-code/en/reference/kimi-command.md)
- [Kimi Code: Configuration files](https://moonshotai.github.io/kimi-code/en/configuration/config-files.md)
- [Kimi Code: Data locations](https://moonshotai.github.io/kimi-code/en/configuration/data-locations.md)
- [Kimi Code: Environment variables](https://moonshotai.github.io/kimi-code/en/configuration/env-vars.md)
- [Kimi Code: Slash commands](https://moonshotai.github.io/kimi-code/en/reference/slash-commands.md)
- [Kimi Code: Plugins](https://moonshotai.github.io/kimi-code/en/customization/plugins.md)
- [Kimi Code: Migrating from kimi-cli](https://moonshotai.github.io/kimi-code/en/guides/migration.md)
- [Kimi Code: Changelog](https://moonshotai.github.io/kimi-code/en/release-notes/changelog.md)
- [Kimi Code repository](https://github.com/MoonshotAI/kimi-code)
- Legacy: [Kimi CLI: Model Context Protocol](https://moonshotai.github.io/kimi-cli/en/customization/mcp.md)
- Legacy: [Kimi CLI: `kimi mcp` Subcommand](https://moonshotai.github.io/kimi-cli/en/reference/kimi-mcp.md)
- Legacy: [`kimi` Command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.md)
- Local observation: `~/.kimi-code/bin/kimi --version` returns `0.14.0`
  (single-binary install method); `kimi --help` lists
  `login` / `acp` / `server` / `web` / `doctor` / `export` / `migrate` /
  `upgrade` / `provider` / `vis` subcommands; there is **no** `kimi mcp`
  subcommand and no `--mcp-config` / `--mcp-config-file` flag. Local
  `~/.kimi-code/mcp.json` does not exist (no user-scope servers
  configured); the legacy `~/.kimi/mcp.json` does not exist either; the
  legacy migrator log at `~/.kimi/.migrated-to-kimi-code` reports
  `mcp.droppedServers: []` and `mcp.mergedServers: []` (no MCP servers to
  carry across), with `droppedKeys` listing the top-level TOML `mcp` key
  rather than any server entry.
