---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://developers.openai.com/codex/mcp/
support: runtime_injection
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http]
  lifecycle: |
    Stdio servers are spawned as local subprocesses at session start. Streamable
    HTTP servers connect to a remote URL at session start. The docs do not describe
    mid-session reconnection, dynamic capability refresh, or retry behavior.
  notes: |
    Codex docs name only stdio and Streamable HTTP transports. Legacy SSE is not
    documented as supported. No explicit MCP protocol version date is published.
config_files:
  - os: all
    scope: user
    path: "$CODEX_HOME/config.toml (default ~/.codex/config.toml)"
    format: toml
    notes: |
      User-level MCP servers live under `[mcp_servers.<id>]`. The CLI, IDE
      extension, and app-server share this file. On Windows the default is
      %USERPROFILE%\.codex\config.toml.
  - os: all
    scope: repo
    path: ".codex/config.toml"
    format: toml
    notes: |
      Project-scoped overrides. Loaded only when the project is trusted. Multiple
      `.codex/config.toml` files may load, with the closest to the working
      directory winning for overlapping keys.
  - os: macos
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    notes: |
      Legacy/undocumented system config path still observed by some Codex clients.
      Current docs emphasize `requirements.toml` and `managed_config.toml` for
      system policy rather than a system `config.toml`.
  - os: linux
    scope: system
    path: "/etc/codex/config.toml"
    format: toml
    notes: |
      Same legacy/undocumented system config path as macOS.
  - os: all
    scope: managed
    path: "/etc/codex/managed_config.toml (Unix); ~/.codex/managed_config.toml (Windows)"
    format: toml
    notes: |
      Managed defaults that merge on top of user config. Can include MCP server
      defaults and policy.
  - os: all
    scope: managed
    path: '/etc/codex/requirements.toml (Unix); %ProgramData%\OpenAI\Codex\requirements.toml (Windows)'
    format: toml
    notes: |
      Admin-enforced requirements, including an `mcp_servers` allowlist that
      disables any configured server whose name and identity do not match.
  - os: all
    scope: plugin
    path: "plugin manifest (bundled) / config.toml under `plugins.<plugin>.mcp_servers.<server>`"
    format: toml
    notes: |
      Installed plugins can bundle MCP servers. User config controls on/off state
      and tool policy per plugin server.
cli_params:
  - flag: "codex mcp add <name> -- <command> [args...]"
    description: "Add a persistent stdio MCP server to ~/.codex/config.toml."
    example: "codex mcp add context7 -- npx -y @upstash/context7-mcp"
  - flag: "codex mcp add <name> --url <url>"
    description: "Add a persistent streamable HTTP MCP server."
    example: "codex mcp add figma --url https://mcp.figma.com/mcp"
  - flag: "codex mcp add --env KEY=VALUE"
    description: "Set an environment variable on a stdio server (repeatable)."
  - flag: "codex mcp add --bearer-token-env-var ENV_VAR"
    description: "Environment variable whose value is sent as a bearer token for HTTP servers."
  - flag: "codex mcp add --oauth-client-id ID --url <url>"
    description: "Pre-register an OAuth client id for a streamable HTTP server."
  - flag: "codex mcp add --oauth-resource RESOURCE"
    description: "OAuth resource parameter for the server."
  - flag: "codex mcp list [--json]"
    description: "List configured MCP servers."
  - flag: "codex mcp get <name> [--json]"
    description: "Show a specific server configuration."
  - flag: "codex mcp remove <name>"
    description: "Delete a stored MCP server definition."
  - flag: "codex mcp login <name> --scopes scope1,scope2"
    description: "Start OAuth login for a streamable HTTP server that supports OAuth."
  - flag: "codex mcp logout <name>"
    description: "Remove stored OAuth credentials for a streamable HTTP server."
  - flag: "-c mcp_servers.<name>.<key>=<value>"
    description: "Override an MCP server setting for a single run (TOML value)."
    example: "codex -c 'mcp_servers.playwright.enabled=false'"
  - flag: "--profile <name>"
    description: "Layer ~/.codex/<name>.config.toml on top of base user config."
env_vars:
  - name: CODEX_HOME
    effect: |
      Sets the root directory for Codex state, including config.toml. Defaults to
      ~/.codex on Unix and %USERPROFILE%\.codex on Windows. Changing it is the
      primary mechanism for runtime MCP injection without mutating the user's
      persistent config.
server_schema:
  transports: ["stdio", "streamable_http"]
  command_fields: ["command", "args", "env", "cwd", "env_vars", "experimental_environment"]
  http_fields: ["url", "bearer_token_env_var", "http_headers", "env_http_headers", "oauth_resource", "scopes"]
  env_shape: |
    `env` is an inline TOML table mapping variable names to string values.
    `env_vars` is an array of variable names to whitelist (or objects with
    `name` and optional `source`: `"local"` or `"remote"`).
  auth_shape: |
    Streamable HTTP servers support OAuth 2.0 (via `codex mcp login`, `scopes`,
    `oauth_resource`, and optional `--oauth-client-id`) and bearer tokens via
    `bearer_token_env_var`. Static `http_headers` and `env_http_headers` are also
    accepted. OAuth credentials are stored according to `mcp_oauth_credentials_store`
    (auto/file/keyring), not in config.
  notes: |
    Server id is the TOML table key `[mcp_servers.<id>]`. Codex infers transport
    from the presence of `command` (stdio) versus `url` (streamable HTTP); there is
    no explicit `type` field. Top-level `mcp_oauth_callback_port` and
    `mcp_oauth_callback_url` configure OAuth callback behavior.
server_capabilities:
  tools: full
  resources: none
  prompts: none
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    Codex exposes MCP tools to the model. It reads the MCP `instructions` field
    during initialization and uses it as server-wide guidance. Resources and
    prompts are not documented as surfaced to the user or model.
client_capabilities:
  roots: unknown
  sampling: unknown
  elicitation: full
  notes: |
    Codex supports MCP elicitation prompts and gates them through the granular
    `approval_policy` (`mcp_elicitations`). Roots and sampling are not documented.
tool_surface:
  discovery: |
    Codex loads tool definitions at session startup. The docs do not describe
    dynamic `tools/list` refresh or `list_changed` handling.
  filtering: |
    Per-server `enabled_tools` and `disabled_tools` lists control the tool surface.
    Managed `requirements.toml` can enforce an `mcp_servers` allowlist by server
    identity.
  approval: |
    Per-server `default_tools_approval_mode` and per-tool `tools.<tool>.approval_mode`
    support `auto`, `prompt`, and `approve`. These sit inside Codex's overall
    `approval_policy`, including granular `mcp_elicitations`.
  result_handling: |
    Tool text results are passed to the model; tool errors are surfaced. Specific
    size limits or sanitization are not documented.
  annotations_trusted: |
    Not documented. Codex does not appear to treat MCP tool annotations as trusted
    policy.
  notes: |
    The `required` flag causes `codex exec` to exit if an enabled server fails to
    initialize.
resource_surface:
  supported: false
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    MCP resources are not documented as a user- or model-selectable surface in
    Codex.
  notes: |
    Resource links returned inside tool results may be rendered, but there is no
    documented `resources/list` or subscription support.
prompt_surface:
  supported: false
  invocation: ""
  arguments: ""
  exposure_model: |
    MCP prompts are not documented as slash commands, palette entries, or
    automatic model tools.
  notes: |
    Codex consumes the server `instructions` field for guidance, but this is not
    the same as exposing MCP prompts.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: deep
  notes: |
    Claudine can read `~/.codex/config.toml` and `.codex/config.toml`, normalize
    server definitions, and write them back. `codex mcp add`, `codex mcp remove`,
    and related commands provide an apply path. Per-server tables replace by id
    when the same `[mcp_servers.<id>]` appears in multiple layers; general TOML
    tables merge deeply.
runtime_injection:
  supported: true
  mechanism: |
    Set `CODEX_HOME` to a temporary directory containing `.codex/config.toml` with
    the desired `[mcp_servers]` tables, then launch `codex` normally. This is how
    Claudine's wrapper performs one-run injection without editing the user's
    persistent `~/.codex/config.toml`.
  limitations: |
    No official one-run MCP flag exists. A shadow `CODEX_HOME` does not inherit
    the user's saved auth, OAuth tokens, history, or other settings, so the caller
    must re-supply anything needed for the run. Project-scoped `.codex/config.toml`
    may still load if the project is trusted.
authorization:
  oauth: true
  credential_storage: |
    MCP OAuth credentials are stored according to `mcp_oauth_credentials_store`
    (`auto`, `file`, or `keyring`). Bearer tokens for HTTP servers are referenced
    by `bearer_token_env_var` and are not stored in config.
  token_scope: |
    Per streamable HTTP server URL. `oauth_resource` can bind a resource
    parameter during login.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` table or the `env_vars`
    whitelist. Process environment inheritance is governed by
    `shell_environment_policy`.
  notes: |
    Static `http_headers` in shared repo config can leak credentials; prefer
    `env_http_headers` or `bearer_token_env_var`. OAuth flows require interactive
    `codex mcp login` unless pre-registered client ids are used.
security:
  tool_filtering: |
    Per-server `enabled_tools`/`disabled_tools`, per-tool `approval_mode`, and
    managed `requirements.toml` `mcp_servers` allowlists constrain the tool
    surface.
  server_trust: |
    Repo `.codex/config.toml` MCP servers load only for trusted projects. Managed
    `requirements.toml` can disable mismatched servers and enforce feature pins.
  env_sanitization: |
    `shell_environment_policy` controls which environment variables Codex passes
    to subprocesses, with default exclusion of secret-like keys. MCP stdio servers
    also use the explicit `env` table and `env_vars` whitelist.
  sandbox_interaction: |
    MCP stdio servers run as ordinary local processes and are not isolated by
    Codex's command sandbox, which applies to model-generated shell commands.
  response_filtering: |
    No native MCP response sanitization is documented. Claudine's `protect` layer
    should treat MCP tool results as untrusted.
  notes: |
    OAuth tokens are stored in the configured credential store. Enterprises should
    deploy `requirements.toml` with an explicit `mcp_servers` allowlist.
gaps:
  - |
    The official docs do not state which MCP protocol version date Codex
    implements.
  - |
    Whether MCP resources or prompts are exposed to the user or model is not
    documented beyond the `instructions` field.
  - |
    `roots/list`, `sampling/createMessage`, and dynamic capability refresh are not
    documented.
  - |
    Tool result size limits, retries, and native response sanitization are not
    documented.
  - |
    No first-class OS-level sandbox or container boundary for stdio MCP servers is
    described.
  - |
    Codex does not provide an official one-run MCP injection flag; injection relies
    on the `CODEX_HOME` side-effect.
changes: []
requires_claudine_update: false
reason: ""
---

# MCP Support in Codex CLI

## Overview

Codex CLI supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io)
as a tool-extension mechanism. MCP servers can be local stdio processes or remote
Streamable HTTP endpoints. Configuration lives in TOML files alongside other Codex
settings, and the same configuration is shared with the Codex IDE extension and
app-server.

This document maps Codex CLI's MCP behavior to the schema used by Claudine's MCP
catalog and provider wrappers.

## Protocol and Transports

Codex documents support for two MCP transports:

| Transport | Status | How it is added |
| :-------- | :----- | :-------------- |
| `stdio` | Primary | `codex mcp add <name> -- <command>` |
| `streamable_http` | Supported | `codex mcp add <name> --url <url>` |

The docs do not mention legacy SSE, WebSocket, or custom transports. No explicit
MCP protocol version date is published, so Claudine should treat the implemented
version as observed rather than pinned.

Lifecycle behavior is described only at a high level: stdio servers start as local
processes, and HTTP servers connect to a remote URL. Mid-session reconnection,
dynamic capability refresh, and retry policies are not documented.

## Configuration

MCP servers are configured under `[mcp_servers.<id>]` tables in `config.toml`.

### Scopes

| Scope | File | Shared | Trust gate |
| :---- | :--- | :----- | :--------- |
| User | `$CODEX_HOME/config.toml` (default `~/.codex/config.toml`) | No | None |
| Project | `.codex/config.toml` | Yes (git) | Requires trusted project |
| Managed defaults | `/etc/codex/managed_config.toml` (Unix); `~/.codex/managed_config.toml` (Windows) | Organization-wide | Admin-controlled |
| Requirements | `/etc/codex/requirements.toml` (Unix); `%ProgramData%\OpenAI\Codex\requirements.toml` (Windows) | Organization-wide | Admin-enforced |
| Plugin | Bundled in plugin manifest; policy in `plugins.<plugin>.mcp_servers.<server>` | With plugin | Plugin trust |

### Precedence

Effective config is assembled from managed defaults and requirements on top of
user and project config. Project-scoped `.codex/config.toml` files are loaded
only when the project is trusted. If multiple project config files exist along
the path from project root to working directory, the closest file wins for
overlapping keys.

### Project trust

Untrusted projects ignore project-local `.codex/config.toml`, hooks, and rules.
User and system layers still load. This matters for Claudine wrappers: repo-level
MCP servers should not be assumed active until the user has trusted the project.

## Server Definition Shape

A stdio server:

```toml
[mcp_servers.context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp"]
env = { "API_KEY" = "..." }
env_vars = ["LOCAL_TOKEN"]
cwd = "/path/to/working/dir"
enabled = true
required = false
startup_timeout_sec = 10
tool_timeout_sec = 60
enabled_tools = ["search"]
disabled_tools = []
default_tools_approval_mode = "prompt"

[mcp_servers.context7.tools.search]
approval_mode = "auto"
```

A streamable HTTP server:

```toml
[mcp_servers.figma]
url = "https://mcp.figma.com/mcp"
bearer_token_env_var = "FIGMA_OAUTH_TOKEN"
http_headers = { "X-Figma-Region" = "us-east-1" }
env_http_headers = { "Authorization" = "AUTH_VAR" }
scopes = ["files:read"]
oauth_resource = "https://mcp.figma.com/"
```

### Common fields

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `command` | stdio | Executable to spawn |
| `args` | stdio | Argument array |
| `env` | stdio | Inline table of environment variables |
| `env_vars` | stdio | Array of variable names to whitelist and forward |
| `cwd` | stdio | Working directory for the server process |
| `experimental_environment` | stdio | `local` or `remote` placement (experimental) |
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
| `default_tools_approval_mode` | all | Default approval behavior for this server's tools |
| `tools.<tool>.approval_mode` | all | Per-tool approval override |

The server id is the TOML table key; transport is inferred from the presence of
`command` versus `url`.

## Tools, Resources, and Prompts

### Tools

Codex exposes MCP tools to the model. Tool exposure is controlled by:

- `enabled_tools` / `disabled_tools`
- `default_tools_approval_mode` and `tools.<tool>.approval_mode`
- Managed `requirements.toml` `mcp_servers` allowlists

Approval modes are `auto`, `prompt`, and `approve`. The overall `approval_policy`
(including granular `mcp_elicitations`) also gates MCP-related prompts.

### Resources and prompts

The public documentation does not describe a user-facing resource picker or
prompt catalog. Codex reads the MCP `instructions` field returned during
initialization and uses it as server-wide guidance, but this is distinct from
exposing MCP resources or prompts. Claudine should assume resources and prompts
are not surfaced unless future documentation proves otherwise.

## Roots, Sampling, and Elicitation

| Capability | Status | Notes |
| :--------- | :----- | :---- |
| Roots | unknown | No documentation on `roots/list` or filesystem boundaries for MCP servers. |
| Sampling | unknown | No documentation on `sampling/createMessage`. |
| Elicitation | full | Supported and gated by `approval_policy.granular.mcp_elicitations`. |

## Import, Export, and Sync

Claudine can treat Codex as an import/sync provider:

- **Import**: read `~/.codex/config.toml` and `.codex/config.toml` and normalize
  `[mcp_servers.<id>]` tables into the MCP catalog.
- **Export**: write provider-shaped TOML back to those files.
- **Apply**: use `codex mcp add`, `codex mcp add --url`, and `codex mcp remove`
  to mutate configuration through the supported CLI.

Merge semantics:

- General TOML tables merge across layers.
- Per-server tables (`[mcp_servers.<id>]`) replace by id when the same id appears
  in multiple layers.
- Managed `requirements.toml` can disable servers that do not match an allowlist.

## Runtime Injection

Codex has no official one-run MCP flag, but it can be injected by redirecting
`CODEX_HOME`:

1. Create a temporary directory.
2. Write `.codex/config.toml` inside it with the desired `[mcp_servers]` tables.
3. Launch `codex` with `CODEX_HOME` pointing at the temporary directory.

This is the mechanism Claudine uses for wrapper-level runtime injection. It does
not mutate the user's persistent `~/.codex/config.toml`.

Limitations:

- The shadow home does not inherit saved authentication, OAuth tokens, history,
  or other user state.
- If the working directory is a trusted project, project-scoped `.codex/config.toml`
  may still load.
- OAuth flows generally require interactive `codex mcp login`; for non-interactive
  runs use bearer tokens or pre-registered client ids.

## Authorization and Credentials

| Pattern | Where configured | Credential storage |
| :------ | :--------------- | :----------------- |
| Static header | `http_headers` | In config file (not recommended for shared repos) |
| Header from env | `env_http_headers` | Env var referenced at runtime |
| Bearer token | `bearer_token_env_var` | Env var referenced at runtime |
| OAuth 2.0 | `scopes`, `oauth_resource`, `--oauth-client-id` | `mcp_oauth_credentials_store` (auto/file/keyring) |

For stdio servers, secrets should be passed through the `env` table or
`env_vars` whitelist rather than committed in config files.

## Security Model

### Trust and allowlisting

- Project `.codex/config.toml` servers load only for trusted projects.
- Managed `requirements.toml` provides admin-enforced allowlists and can disable
  any configured MCP server whose identity does not match.
- Feature flags in `requirements.toml` can disable MCP-related surfaces entirely.

### Environment and sandboxing

- MCP stdio servers inherit process environment according to
  `shell_environment_policy`, which can exclude secret-like keys by default.
- Per-server `env` and `env_vars` give explicit control over variables forwarded
  to a server.
- Codex's command sandbox applies to model-generated shell commands, not to MCP
  server processes. There is no documented OS-level sandbox around stdio MCP
  servers.

### Response handling

- No native MCP response sanitization is documented.
- Claudine's `protect` layer should treat MCP tool results as untrusted content.

## Mode-Specific Behavior

### Interactive TUI

- `/mcp` displays active MCP servers.
- `codex mcp login <name>` can complete OAuth flows.
- Project servers from `.codex/config.toml` are available only after the project
  is trusted.

### Non-interactive `codex exec`

- Reuses saved CLI authentication by default; use `CODEX_API_KEY` for API-key
  automation.
- OAuth login flows cannot run interactively; use bearer tokens or pre-registered
  OAuth configs.
- If an enabled MCP server has `required = true` and fails to initialize, the
  command exits with an error.
- Use `--ignore-user-config` to skip `$CODEX_HOME/config.toml` for a controlled
  automation environment.

### Codex as an MCP server

`codex mcp-server` runs Codex itself as a stdio MCP server, exposing its built-in
tools to an external MCP client. The external client is responsible for its own
approval UI.

## Failure Modes

| Failure | Behavior |
| :------ | :--------- |
| `required` server fails to start | `codex exec` exits with an error |
| Non-required server fails to start | Server unavailable; session continues |
| Stdio server exits mid-session | Not documented as auto-reconnected |
| HTTP server unreachable | Likely treated as unavailable; retry policy is not documented |
| Tool timeout | Aborted after `tool_timeout_sec` (default 60s) |
| Startup timeout | Aborted after `startup_timeout_sec` (default 10s) |
| Project config untrusted | `.codex/config.toml` MCP servers are ignored |

## Gaps

- Explicit MCP protocol version date is not stated.
- Resource and prompt surfaces are not documented as user/model-facing features.
- `roots/list`, sampling, and dynamic capability refresh are not documented.
- Tool result size limits, retries, and native sanitization are not documented.
- No documented OS-level sandbox for stdio MCP servers.
- No official one-run MCP injection flag; injection relies on the `CODEX_HOME`
  side-effect.

## Claudine Integration Notes

- Treat Codex as `support: runtime_injection`; import/export/sync is also
  supported and described in `sync_behavior`.
- Map Claudine's normalized catalog to Codex's `[mcp_servers.<id>]` TOML tables,
  preserving stdio versus streamable HTTP fields.
- For one-run wrappers, prefer a shadow `CODEX_HOME` with a generated
  `.codex/config.toml` rather than editing user config.
- Honor project trust: do not assume `.codex/config.toml` servers are active
  until the project is trusted.
- Defensively scan MCP tool results in the `protect` layer; Codex does not
  document native response sanitization.

## Sources

- [Codex MCP documentation](https://developers.openai.com/codex/mcp/)
- [Codex CLI reference](https://developers.openai.com/codex/cli/reference/)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference/)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables/)
- [Codex advanced configuration](https://developers.openai.com/codex/config-advanced/)
- [Codex managed configuration](https://developers.openai.com/codex/enterprise/managed-configuration/)
- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive/)
- [Model Context Protocol specification](https://modelcontextprotocol.io)
