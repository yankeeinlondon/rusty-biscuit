---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://moonshotai.github.io/kimi-cli/en/customization/mcp.md
support: import_sync
protocol:
  versions: ["unknown"]
  transports: [stdio, streamable_http]
  lifecycle: |
    MCP servers are discovered once when a Kimi Code CLI session starts. Stdio
    servers are spawned as local subprocesses; HTTP servers open remote
    Streamable HTTP connections. Initialization is asynchronous after the shell
    UI starts, and connection progress is shown in the status bar. Servers that
    fail to connect remain in a non-ready state; the public docs do not describe
    automatic reconnection or mid-session `list_changed` refresh.
  notes: |
    The CLI exposes the transport choice as `--transport stdio` or
    `--transport http`. The HTTP transport behaves as Streamable HTTP. Legacy
    SSE and HTTP+SSE transports are not documented. The docs do not state an
    explicit MCP protocol version date.
config_files:
  - os: all
    scope: user
    path: "~/.kimi/mcp.json"
    format: json
    notes: |
      User-scope MCP server definitions in the standard `mcpServers` object.
      The base directory can be relocated with `KIMI_SHARE_DIR`. No repo-level,
      system, or managed MCP config files are documented.
cli_params:
  - flag: "kimi mcp add --transport http <name> <url>"
    description: "Add a persistent Streamable HTTP MCP server."
    example: "kimi mcp add --transport http context7 https://mcp.context7.com/mcp"
  - flag: "kimi mcp add --transport http --auth oauth <name> <url>"
    description: "Add an OAuth-enabled Streamable HTTP MCP server."
    example: "kimi mcp add --transport http --auth oauth linear https://mcp.linear.app/mcp"
  - flag: "kimi mcp add --transport stdio <name> -- <command> [args...]"
    description: "Add a persistent stdio MCP server."
    example: "kimi mcp add --transport stdio chrome-devtools -- npx chrome-devtools-mcp@latest"
  - flag: "kimi mcp add --env KEY=VALUE"
    description: "Set an environment variable on a stdio server (repeatable)."
  - flag: "kimi mcp add --header 'Name: Value'"
    description: "Set a static header on an HTTP server (repeatable)."
  - flag: "kimi mcp list"
    description: "List configured MCP servers and their authorization status."
  - flag: "kimi mcp remove <name>"
    description: "Remove a configured MCP server."
  - flag: "kimi mcp auth <name>"
    description: "Complete an interactive OAuth flow for a server."
  - flag: "kimi mcp reset-auth <name>"
    description: "Clear cached OAuth tokens for a server."
  - flag: "kimi mcp test <name>"
    description: "Test connectivity and list tools exposed by a server."
  - flag: "--mcp-config-file <path>"
    description: "Load MCP servers from a config file for this run (repeatable)."
    example: "kimi --mcp-config-file ./mcp.json -p 'use tools'"
  - flag: "--mcp-config <json>"
    description: "Load MCP servers from an inline JSON string for this run (repeatable)."
    example: "kimi --mcp-config '{\"mcpServers\":{\"fs\":{\"command\":\"npx\"}}}' -p 'use tools'"
  - flag: "/mcp"
    description: "In interactive shell mode, show connected servers and loaded tools."
env_vars:
  - name: KIMI_SHARE_DIR
    effect: |
      Relocates the Kimi Code CLI data directory (default `~/.kimi`), which
      moves `mcp.json` and the `mcp-oauth/` credential directory.
server_schema:
  transports: ["stdio", "streamable_http"]
  command_fields: ["command", "args", "env"]
  http_fields: ["url", "headers"]
  env_shape: |
    `env` is an object mapping variable names to string values. The docs do not
    document shell-style variable expansion.
  auth_shape: |
    HTTP servers use static `headers` for tokens/API keys. OAuth-enabled servers
    are created with `--auth oauth`; tokens are cached in `~/.kimi/mcp-oauth/`
    and are not stored inside `mcp.json`.
  notes: |
    Server id is the map key under `mcpServers`. The stored config may include
    an optional `transport` field (e.g. `"http"`), but transport is normally
    implied by the presence of `command` versus `url`. Only stdio and HTTP
    transports are documented.
server_capabilities:
  tools: full
  resources: none
  prompts: none
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    Tools are exposed to the model and can be inspected with `kimi mcp test`.
    Resources and prompts are not described as surfaced features.
client_capabilities:
  roots: unknown
  sampling: unknown
  elicitation: unknown
  notes: |
    The public documentation does not describe `roots/list`,
    `sampling/createMessage`, or elicitation support.
tool_surface:
  discovery: |
    `tools/list` is called at session startup. `kimi mcp test <name>` also
    enumerates available tools and reports connection status.
  filtering: |
    No per-server or per-tool include/exclude filtering is documented.
  approval: |
    MCP tool calls use the same approval mechanism as native tools; every MCP
    tool call prompts for confirmation in interactive mode. YOLO and AFK modes
    auto-approve MCP tool calls along with native tools.
  result_handling: |
    Tool return content is marked so the model can distinguish it from user
    instructions. Text results are passed to the model; no output-size limits
    or truncation behavior are documented.
  annotations_trusted: |
    Tool annotations are not discussed in the public docs.
  notes: |
    In `--print`/`--afk`/`--yolo` modes, MCP tool calls are auto-approved.
resource_surface:
  supported: false
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    MCP resources are not documented as a surfaced feature.
  notes: |
    Only tools are described. Resource links returned by tools, if any, are not
    discussed.
prompt_surface:
  supported: false
  invocation: ""
  arguments: ""
  exposure_model: |
    MCP prompts are not documented as a surfaced feature.
  notes: |
    The `/mcp` slash command shows connected servers and loaded tools, not
    prompt templates.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: true
  merge_strategy: replace
  notes: |
    Claudine can read and write `~/.kimi/mcp.json` and can apply changes
    through `kimi mcp add` and `kimi mcp remove`. There is a single user-scope
    file; repo-level or managed policy layers are not documented.
runtime_injection:
  supported: true
  mechanism: |
    Pass `--mcp-config-file <path>` or `--mcp-config <json>` to the main
    `kimi` command. Both options can be repeated. This loads MCP servers for
    the current run without mutating `~/.kimi/mcp.json`.
  limitations: |
    The docs do not specify whether runtime config merges with or replaces the
    default `~/.kimi/mcp.json`. OAuth authorization requires an interactive
    browser flow, so OAuth servers are unlikely to work in non-interactive
    `--print` runs.
authorization:
  oauth: true
  credential_storage: |
    OAuth tokens for MCP servers are stored as files in `~/.kimi/mcp-oauth/`.
    Other Kimi account credentials live in `~/.kimi/credentials/`.
  token_scope: |
    Per server name; `kimi mcp reset-auth <name>` clears the token for one
    server.
  stdio_secret_delivery: |
    Secrets are delivered through the per-server `env` object in `mcp.json` or
    via `--env` when adding the server. Process environment is otherwise
    inherited.
  notes: |
    Static API-key headers are stored in `mcp.json`; OAuth tokens are kept
    outside the config file.
security:
  tool_filtering: |
    No allowlist, denylist, or per-tool include/exclude mechanism is
    documented.
  server_trust: |
    No workspace-trust gate or managed-policy layer is documented for MCP
    servers. Users must rely on interactive approval and avoid YOLO/AFK mode
    with untrusted servers.
  env_sanitization: |
    Stdio servers inherit the provider process environment plus the explicit
    per-server `env` map. No credential-scrubbing behavior is documented.
  sandbox_interaction: |
    MCP servers run as ordinary local processes and are not isolated by any
    documented sandbox.
  response_filtering: |
    Tool output is tagged to help the model separate it from user
    instructions. No prompt-injection scan or result sanitization is
    documented.
  notes: |
    YOLO/AFK modes bypass the approval prompt for MCP tools, so wrappers
    should avoid those modes when exposing untrusted servers.
gaps:
  - |
    Explicit MCP protocol version date is not stated.
  - |
    Only `stdio` and HTTP (`streamable_http`) transports are documented;
    legacy SSE is not mentioned.
  - |
    Whether resources and prompts are surfaced to the user or model is not
    documented; the docs only discuss tools.
  - |
    Roots, sampling, and elicitation support are not documented.
  - |
    No per-tool or per-server filtering/allowlist mechanism is documented.
  - |
    Merge semantics when multiple `--mcp-config-file`/`--mcp-config` values
    are supplied are not specified.
  - |
    Automatic reconnection, retry, and failure-recovery behavior for remote
    MCP servers is not documented.
  - |
    No repo-level, system, or managed MCP policy files are documented.
changes: []
requires_claudine_update: true
reason: |
  Claudine's claudine skill currently classifies Kimi as having no MCP
  support, but the official Kimi Code CLI docs describe persistent
  `~/.kimi/mcp.json` configuration, `kimi mcp` management commands, and
  one-run injection via `--mcp-config-file`/`--mcp-config`. Claudine's MCP
  catalog and provider wrapper metadata should be updated to treat Kimi as
  `import_sync` with `runtime_injection`.
---

# MCP Support in Kimi Code CLI

## Overview

Kimi Code CLI supports the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/)
as a tool-extension mechanism. MCP servers add external tools to the agent's
loop; the CLI itself already has built-in tools for file access, shell
commands, and web fetching. Configuration is persistent (in `~/.kimi/mcp.json`)
and can also be supplied for a single run via CLI flags.

> **Note:** The Kimi team is winding down `kimi-cli` in favor of
> [Kimi Code](https://github.com/MoonshotAI/kimi-code). The docs and existing
> installations remain available. This research reflects the published
> `kimi-cli` behavior.

## Protocol and Transports

Kimi Code CLI speaks two documented MCP transports:

| Transport | CLI flag | Notes |
| :-------- | :------- | :---- |
| `stdio` | `--transport stdio` | Local subprocess server. |
| Streamable HTTP | `--transport http` | Remote server over HTTP; the CLI flag calls it `http`. |

The docs do not name a specific MCP protocol version. Legacy SSE and HTTP+SSE
are not documented.

Lifecycle behavior:

- `tools/list` is called at session startup.
- Servers initialize asynchronously after the shell UI starts.
- Connection progress is shown in the status bar and web UI.
- Failed servers are not described as auto-reconnecting.

## Configuration

MCP servers are stored in the user-scope file:

- `~/.kimi/mcp.json` — standard `mcpServers` object.

The base directory can be relocated with the `KIMI_SHARE_DIR` environment
variable. There is no documented repo-level, system, or managed MCP config.

Example `~/.kimi/mcp.json`:

```json
{
  "mcpServers": {
    "context7": {
      "url": "https://mcp.context7.com/mcp",
      "headers": {
        "CONTEXT7_API_KEY": "your-key"
      }
    },
    "chrome-devtools": {
      "command": "npx",
      "args": ["chrome-devtools-mcp@latest"],
      "env": {
        "SOME_VAR": "value"
      }
    }
  }
}
```

## Server Definition Shape

- Server id is the key under `mcpServers`.
- Stdio servers use `command`, `args`, and optionally `env`.
- HTTP servers use `url` and optionally `headers`.
- An optional `transport` field may be stored (e.g. `"http"`).
- OAuth-enabled servers are flagged at add time (`--auth oauth`); tokens are
  not stored in `mcp.json`.

## Tools, Resources, and Prompts

### Tools

MCP tools are exposed to the model. `kimi mcp test <name>` reports the tool
names and descriptions a server provides. In interactive shell mode, `/mcp`
shows connected servers and loaded tools.

### Resources and prompts

The public documentation only describes MCP tools. Resources and prompts are
not documented as user-facing or model-facing surfaces.

## Roots, Sampling, and Elicitation

The docs do not describe MCP roots, sampling (`sampling/createMessage`), or
elicitation support. Claudine should treat these as unknown.

## Import, Export, and Sync

Claudine can treat Kimi Code CLI as an `import_sync` provider:

- **Import:** read `~/.kimi/mcp.json` and normalize server definitions.
- **Export:** write provider-shaped JSON back to `~/.kimi/mcp.json`.
- **Apply:** use `kimi mcp add`, `kimi mcp remove`, etc. to mutate config.

There is a single user-scope file, so merge semantics are effectively
replace-per-server.

## Runtime Injection

For one-run injection without mutating persistent config:

```bash
kimi --mcp-config-file /path/to/mcp.json -p "use the mcp tools"
kimi --mcp-config '{"mcpServers":{"fs":{"command":"npx","args":["-y","@modelcontextprotocol/server-filesystem","."]}}}' -p "list files"
```

Both options are repeatable. The docs do not clarify whether they merge with
`~/.kimi/mcp.json` or replace it.

## Authorization and Credentials

- **Static headers:** stored in `mcp.json` (`headers` map).
- **OAuth:** added with `--auth oauth`; authorized via `kimi mcp auth <name>`.
  Tokens are cached in `~/.kimi/mcp-oauth/`.
- **Stdio secrets:** delivered through the per-server `env` map or `--env`.

OAuth flows require an interactive browser, so pre-authorized or header-based
servers are needed for non-interactive runs.

## Security Model

- MCP tool calls use the same approval prompt as native tools.
- In YOLO/AFK modes, MCP tool calls are auto-approved.
- No allowlist, denylist, or per-tool filtering is documented.
- No workspace-trust gate or managed policy is documented.
- Stdio servers inherit the provider process environment plus explicit `env`.
- No sandbox boundary around MCP servers is documented.
- Tool output is marked to help the model distinguish it from user
  instructions; no additional prompt-injection filtering is documented.

## Mode-Specific Behavior

### Interactive shell mode

- `/mcp` shows connected servers and tools.
- `kimi mcp auth` can complete OAuth in a browser.
- Every MCP tool call prompts for approval unless YOLO/AFK is active.

### Non-interactive print mode (`--print`, `--quiet`)

- Implicitly enables `--afk`, so tool calls (including MCP) are auto-approved.
- OAuth cannot complete interactively; use static headers or pre-authorized
  servers.
- `--mcp-config-file`/`--mcp-config` are the practical injection path.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Server fails to start | Shown as non-ready in the status bar; no auto-reconnect documented |
| `kimi mcp test` fails | Reports connection status and tool count (zero if failed) |
| OAuth not completed | Server/tools unavailable until `kimi mcp auth` succeeds |
| Tool call timeout | Controlled by `mcp.client.tool_call_timeout_ms` (default 60 s) in `config.toml` |

## Gaps

See the frontmatter `gaps` list for the full set. Key unknowns include the
MCP protocol version, resource/prompt surfaces, roots/sampling/elicitation,
and runtime-config merge semantics.

## Claudine Integration Notes

- Treat Kimi Code CLI as `support: import_sync` with `runtime_injection`.
- Read/write the single user file `~/.kimi/mcp.json` (respecting
  `KIMI_SHARE_DIR`).
- Apply changes through `kimi mcp add` and `kimi mcp remove` when possible.
- For one-run wrappers, inject via `--mcp-config-file` or `--mcp-config`.
- Do not assume OAuth servers work in non-interactive `--print` mode.
- Defensively scan MCP tool results; Kimi does not document result
  sanitization beyond marking tool output boundaries.

## Sources

- [Model Context Protocol docs](https://moonshotai.github.io/kimi-cli/en/customization/mcp.md)
- [`kimi mcp` reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-mcp.md)
- [`kimi` command reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.md)
- [Config files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md)
- [Data locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.md)
- [Environment variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.md)
- [Print mode](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.md)
- [GitHub README](https://github.com/MoonshotAI/kimi-cli)
