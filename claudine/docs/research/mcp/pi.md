---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://pi.dev/docs/latest
support: none
protocol:
  versions: []
  transports: []
  lifecycle: "No MCP lifecycle is implemented in Pi core."
  notes: |
    Pi explicitly does not implement the Model Context Protocol. Its extension API
    could theoretically host an MCP client, but no first-party or canonical
    third-party MCP extension exists today.
config_files: []
cli_params: []
env_vars: []
server_schema:
  transports: []
  command_fields: []
  http_fields: []
  env_shape: "N/A"
  auth_shape: "N/A"
  notes: |
    Pi has no native MCP server schema. Tools are registered by TypeScript
    extensions through `pi.registerTool()`, not via MCP server definitions.
server_capabilities:
  tools: none
  resources: none
  prompts: none
  tool_list_changed: false
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    MCP surfaces are not exposed. Pi provides built-in tools (`read`, `write`,
    `edit`, `bash`, `grep`, `find`, `ls`) and extension-registered tools only.
client_capabilities:
  roots: none
  sampling: none
  elicitation: none
  notes: |
    Pi does not act as an MCP client, so it does not expose MCP roots, sampling,
    or elicitation to servers.
tool_surface:
  discovery: "N/A"
  filtering: |
    Built-in and extension tools can be allowlisted or denylisted via `--tools`,
    `--exclude-tools`, `--no-builtin-tools`, and `--no-tools`. These filters are
    not MCP-specific.
  approval: |
    Pi has no built-in permission popups. Users can add confirmation flows via
    extensions such as `permission-gate.ts`.
  result_handling: "N/A"
  annotations_trusted: "N/A"
  notes: |
    MCP tool results are not applicable. Extension tools return `{ content, details }`
    objects that are rendered in the TUI and persisted in the session JSONL.
resource_surface:
  supported: false
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: "N/A"
  notes: |
    MCP resources are not supported. The closest equivalent is Pi's skill/prompt
    template and dynamic context extension APIs.
prompt_surface:
  supported: false
  invocation: "N/A"
  arguments: "N/A"
  exposure_model: "N/A"
  notes: |
    MCP prompts are not supported. Pi has its own prompt-template system invoked
    with `/templatename`.
sync_behavior:
  import_supported: false
  export_supported: false
  apply_supported: false
  merge_strategy: none
  notes: |
    There is no MCP config to import, export, or sync. Claudine should not attempt
    to manage MCP servers for Pi.
runtime_injection:
  supported: false
  mechanism: "N/A"
  limitations: |
    No runtime MCP injection path exists. A user could install a third-party
    extension that embeds an MCP client, but that extension would define its own
    config and security model and is out of scope for Claudine's provider wrapper.
authorization:
  oauth: false
  credential_storage: "N/A"
  token_scope: "N/A"
  stdio_secret_delivery: "N/A"
  notes: |
    No MCP authorization flow exists. API keys for LLM providers are passed via
    environment variables (`ANTHROPIC_API_KEY`, etc.) or `/login` OAuth for
    subscription providers.
security:
  tool_filtering: |
    `--tools`, `--exclude-tools`, `--no-builtin-tools`, and `--no-tools` filter
    the tool surface. There is no per-server or per-MCP-tool filtering.
  server_trust: |
    Pi has no MCP server trust model. Project-local extensions and settings are
    gated by project trust (`/trust`, `--approve`, `--no-approve`,
    `defaultProjectTrust`), but this is unrelated to MCP.
  env_sanitization: |
    Extensions and tools run with the user's process environment. Pi does not
    implement MCP-specific env sanitization.
  sandbox_interaction: |
    Pi does not sandbox core execution. Optional sandboxing is available through
    external patterns: Gondolin micro-VM, plain Docker, or OpenShell.
  response_filtering: |
    No MCP response filtering exists. Extension authors are responsible for
    sanitizing tool outputs.
  notes: |
    Pi runs with the permissions of the launching user and process. Extensions
    execute arbitrary TypeScript code with full system access.
gaps:
  - |
    Pi has no first-party MCP support. If a canonical third-party MCP extension
    emerges, it should be researched separately.
changes: []
requires_claudine_update: false
reason: ""
---

# MCP Support in Pi

## Overview

[Pi](https://pi.dev/) is a minimal, extensible terminal coding agent harness. Its
core philosophy explicitly excludes built-in MCP support: the homepage and README
list "No MCP" as an intentional design choice, directing users who need MCP-like
capabilities to build [extensions](https://github.com/earendil-works/pi/tree/main/packages/coding-agent#extensions)
or use the [skills](https://github.com/earendil-works/pi/tree/main/packages/coding-agent#skills)
/README-based tool pattern instead.

This document records Pi's MCP posture for Claudine's MCP catalog. Because MCP is
absent from Pi core, Claudine should classify Pi as `support: none` and should not
attempt import, export, sync, or runtime injection of MCP servers.

## Protocol and Transports

Pi does not implement the Model Context Protocol in any form. It does not spawn
stdio MCP servers, connect to HTTP/SSE MCP endpoints, or support Streamable HTTP.
Its transport layer is concerned with LLM provider streaming (SSE, WebSocket,
etc.), not MCP.

## Configuration

Pi has no MCP-specific configuration files. Relevant persistent files include:

| File | Scope | Purpose |
| :--- | :---- | :------ |
| `~/.pi/agent/settings.json` | User | General settings (model, theme, compaction, resources) |
| `.pi/settings.json` | Project | Project overrides and resource loading |
| `~/.pi/agent/trust.json` | User | Saved project trust decisions |
| `~/.pi/agent/extensions/` | User | User-installed extensions |
| `.pi/extensions/` | Project | Project-local extensions |

None of these define MCP servers. Settings files do support `packages`,
`extensions`, `skills`, `prompts`, and `themes`, but those are Pi-native resource
paths, not MCP server definitions.

## Server Definition Shape

Not applicable. Pi extensions register tools programmatically via
`pi.registerTool({ name, description, parameters, execute })`. There is no
MCP-equivalent `mcpServers` object or server definition schema.

## Tools, Resources, and Prompts

### Tools

Pi exposes built-in tools (`read`, `write`, `edit`, `bash`, `grep`, `find`, `ls`)
and allows extensions to register additional tools. These are not MCP tools and do
not go through `tools/list` or `tools/call`. Tool results are stored in the session
JSONL as `{ content, details }` objects.

### Resources and prompts

MCP resources and prompts are not supported. Pi has its own prompt-template
feature (Markdown files in `~/.pi/agent/prompts/` or `.pi/prompts/`, invoked with
`/templatename`) and skill feature (Agent Skills standard via `/skill:name`).

## Roots, Sampling, and Elicitation

Pi does not act as an MCP client, so it does not expose:

- `roots/list` boundaries to servers
- `sampling/createMessage` (server-requested LLM calls)
- `completion/complete` or elicitation (server-requested user input)

The closest concept to roots is the working directory from which `pi` is launched;
built-in and extension tools can read relative paths from it.

## Import, Export, and Sync

Claudine cannot import, export, or sync MCP servers with Pi because Pi has no
native MCP config. Any future third-party MCP extension would need its own
integration path.

## Runtime Injection

There is no runtime injection mechanism for MCP servers. Claudine's Pi wrapper
should not attempt to inject MCP config or spawn MCP subprocesses on Pi's behalf.

The correct extension path for users who want MCP is to install or write a Pi
extension that embeds an MCP client; that extension would be responsible for its
own server discovery, config, and lifecycle.

## Authorization and Credentials

No MCP authorization exists. Pi authentication is limited to:

- API keys via environment variables (e.g., `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`)
- Subscription OAuth via `/login` for supported providers

These credentials are for LLM providers, not MCP servers.

## Security Model

Pi's security posture is intentionally minimal:

- **No built-in permission system.** Pi runs with the privileges of the launching
  user and process.
- **No MCP server trust gate.** Project trust applies to project-local settings,
  resources, and extensions, not to MCP servers.
- **Extensions run arbitrary code.** A Pi extension that adds MCP support would
  inherit the user's full environment and permissions.
- **Optional sandboxing.** Users can run Pi inside Gondolin, Docker, or OpenShell
  if they need isolation.

## Mode-Specific Behavior

MCP is not available in any mode:

- **Interactive mode:** No `/mcp` command or MCP panel.
- **Print/JSON mode (`-p`, `--mode json`):** No MCP injection or tool exposure.
- **RPC mode (`--mode rpc`):** No MCP JSON-RPC surface.
- **SDK:** No MCP client APIs.

## Failure Modes

Not applicable for MCP. Pi's built-in tool failures and extension errors are
surfaced in the TUI or JSONL event stream and handled by the agent loop's retry
logic (`retry.enabled`, `retry.maxRetries`, `retry.baseDelayMs`).

## Gaps

- Pi core has no MCP support.
- No canonical first-party or third-party MCP extension has been identified.
- If such an extension appears, it would need separate research for config format,
  transport support, security model, and sync behavior.

## Claudine Integration Notes

- Classify Pi as `support: none` in the MCP catalog.
- Do not expose `claudine mcp export pi --apply` or similar MCP sync commands.
- Do not inject MCP servers into Pi wrapper invocations.
- If Pi's extension ecosystem later produces a de facto MCP client extension,
  revisit this document rather than assuming native provider support.

## Sources

- [Pi homepage](https://pi.dev/)
- [Pi documentation](https://pi.dev/docs/latest)
- [Pi coding-agent README](https://github.com/earendil-works/pi/tree/main/packages/coding-agent)
- [Pi settings documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/settings.md)
- [Pi extension examples](https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/extensions)
- ["What if you don't need MCP at all?" — Mario Zechner](https://mariozechner.at/posts/2025-11-02-what-if-you-dont-need-mcp/)
