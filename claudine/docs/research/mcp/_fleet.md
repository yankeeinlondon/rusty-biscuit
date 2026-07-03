---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/mcp/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local MCP config examples under {{state.user_dir}} when they exist.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> MCP is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **MCP** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **MCP** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the MCP research on **{{state.name}}** failed to complete!"
    warn: "The MCP research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---

## Skills

Use the 'claudine' skill.

## Scope

Research Model Context Protocol support for **{{state.desc}}**. This topic feeds
Claudine's MCP catalog, import/export/sync behavior, runtime injection, and provider
security posture. Write the result to `{{file}}` and include `$schema: ./_schema.yaml`
in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `docs`
- `support`
- `protocol`
- `config_files`
- `cli_params`
- `env_vars`
- `server_schema`
- `server_capabilities`
- `client_capabilities`
- `tool_surface`
- `resource_surface`
- `prompt_surface`
- `sync_behavior`
- `runtime_injection`
- `authorization`
- `security`
- `gaps`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: none` only when MCP support is clearly absent. Use `unknown` where the
current documentation does not prove the answer.

## Frontmatter Field Guide

Use this section as the authoritative meaning of each schema property.

### Identity and Docs

- `created`: Date this provider file was first created. Set only on first creation.
  Example: `created: 2026-07-02`
- `last_updated`: Date this research was verified. Always set to `{{ctx.today}}`.
- `agent`: Research runner. Set to `{{env.AGENT}}`.
- `model`: Research model. Set to `{{env.MODEL || 'default'}}`.
- `docs`: Primary official MCP docs URL for this provider. If no MCP-specific docs
  exist, use the best official config/integration docs and explain the gap in `gaps`
  or the body.

### MCP Concepts to Preserve

Do not reduce MCP to "tool server config." The protocol has distinct surfaces that
providers expose differently:

- **Tools** are model-controlled functions (`tools/list`, `tools/call`). They need
  visibility, approval, timeout, result-sanitization, and audit handling.
- **Resources** are application-controlled context identified by URI. Clients decide how
  users or models select them; resource templates and subscriptions may exist.
- **Prompts** are user-controlled prompt templates. Providers may expose them as slash
  commands, palette actions, or not at all.
- **Roots** are client-provided filesystem/workspace boundaries. Servers may ask for
  `roots/list`; provider clients decide what roots they expose.
- **Sampling** lets servers ask the client to make an LLM call. This is powerful and
  should have explicit user approval.
- **Elicitation** lets servers ask the client to collect structured user input. It must
  not be used for sensitive information.
- **Transports** matter: stdio is local subprocess JSON-RPC; Streamable HTTP is the
  modern remote transport; HTTP+SSE/SSE is legacy/compatibility; custom transports may
  exist.
- **Authorization** differs by transport: HTTP MCP may use OAuth-style authorization;
  stdio should receive credentials via environment/config/credential stores, not the
  HTTP auth flow.

Research must explicitly say which of these surfaces the provider supports, hides,
forwards, filters, or ignores.

### Support

`support` is the highest-value classification for Claudine:

- `import_sync`: Claudine can reasonably import/export/sync MCP servers with the
  provider's persistent config.
- `runtime_injection`: The provider can accept MCP servers for a single run without
  mutating user config.
- `manual_config`: The provider supports MCP, but only through user-edited config files
  or UI setup that Claudine should not mutate automatically.
- `partial`: MCP exists but important behavior is missing, unstable, plugin-only,
  one-transport-only, or not available in the CLI mode Claudine wraps.
- `none`: Clear evidence MCP is not supported.
- `unknown`: Current evidence is insufficient.

Choose the strongest true value for Claudine's integration path. For example, if a
provider supports both persistent config sync and one-run runtime injection, use
`runtime_injection` and describe sync support in `sync_behavior`.

Example:

```yaml
support: runtime_injection
```

### Protocol

`protocol` records protocol generation and transport behavior. Look for explicit
protocol version dates, supported transports, session handling, and whether legacy
HTTP+SSE/SSE is still accepted.

Example:

```yaml
protocol:
  versions: ["2025-06-18"]
  transports: [stdio, streamable_http]
  lifecycle: "Initializes each server at session start; reconnects failed stdio servers only after restart."
  notes: "Remote servers use Streamable HTTP; legacy SSE config is still accepted for compatibility."
```

### Config Files

`config_files` records persistent files that define MCP servers or MCP-related policy.
Use separate macOS, Linux, and Windows records for every filesystem path. Do not use
`os: all` for paths; Windows path syntax alone makes that ambiguous. Also use separate
records for user, repo, managed/system, and plugin scopes.

Example:

```yaml
config_files:
  - os: macos
    scope: user
    path: "~/.provider/mcp.json"
    format: json
    notes: "Primary user-level MCP server config on macOS."
  - os: linux
    scope: user
    path: "~/.config/provider/mcp.json"
    format: json
    notes: "Primary user-level MCP server config on Linux; verify exact XDG behavior."
  - os: windows
    scope: user
    path: "%APPDATA%\\Provider\\mcp.json"
    format: json
    notes: "Primary user-level MCP server config on Windows."
  - os: macos
    scope: repo
    path: ".provider/mcp.json"
    format: json
    notes: "Repo-local MCP servers; add Linux and Windows records explicitly."
  - os: windows
    scope: user
    path: "%APPDATA%\\Provider\\mcp.json"
    format: json
    notes: "Windows user config path."
```

### CLI Params

`cli_params` is for MCP-specific commands and switches: add/list/remove/import/export,
enable/disable MCP mode, select servers, trust servers, or point to config files. Do not
include unrelated general CLI flags.

Example:

```yaml
cli_params:
  - flag: "mcp add"
    description: "Adds a persistent MCP server to provider config."
    example: "provider mcp add filesystem -- npx -y @modelcontextprotocol/server-filesystem ."
  - flag: "--mcp-config"
    description: "Uses an alternate MCP config file for this run."
    example: "provider run --mcp-config ./mcp.json \"inspect tools\""
```

### Environment Variables

`env_vars` captures variables that affect MCP config, runtime injection, auth, or server
visibility. Do not list provider-wide variables unless they change MCP behavior.

Example:

```yaml
env_vars:
  - name: PROVIDER_MCP_CONFIG
    effect: "Overrides the MCP config file path for the session."
  - name: PROVIDER_CONFIG_CONTENT
    effect: "Allows passing inline JSON config that can include MCP server definitions."
```

### Server Schema

`server_schema` describes the accepted shape of one MCP server definition, not the whole
config file. Capture transports and field names using provider-native keys.

- `transports`: Accepted transport kinds, such as `stdio`, `sse`, `streamable_http`,
  `http`, or provider-native names.
- `command_fields`: Keys used for command/stdio servers.
- `http_fields`: Keys used for remote HTTP/SSE servers.
- `env_shape`: How per-server env vars are represented.
- `auth_shape`: How auth headers/tokens/OAuth references are represented.
- `notes`: Required fields, unsupported transports, schema URLs, and provider quirks.

Example:

```yaml
server_schema:
  transports: ["stdio", "streamable_http", "sse"]
  command_fields: ["command", "args", "env", "cwd"]
  http_fields: ["url", "headers", "oauth"]
  env_shape: "env is an object mapping variable names to string values."
  auth_shape: "HTTP servers may use OAuth token storage; static headers are supported."
  notes: "Server id is the map key under mcpServers."
```

### Server Capabilities

`server_capabilities` records which MCP server features the provider exposes to users
and models. The provider may support MCP tools while ignoring resources/prompts, or it
may support resources in UI only.

Example:

```yaml
server_capabilities:
  tools: full
  resources: partial
  prompts: none
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: "Tools are available to the model; resources can be selected manually but are not auto-discovered by the model."
```

### Client Capabilities

`client_capabilities` records what the provider client offers back to MCP servers. This
is the side many provider docs omit, but it matters for security and interoperability:
roots define filesystem boundaries; sampling can trigger nested LLM calls; elicitation
can ask the user for structured input.

Example:

```yaml
client_capabilities:
  roots: partial
  sampling: none
  elicitation: unknown
  notes: "Provider exposes the workspace root to stdio servers but does not document sampling/createMessage support."
```

### Tool Surface

`tool_surface` is about model-visible tools, not server configuration. Capture how tools
are discovered, whether users can filter individual tools, how approvals work, and
whether results are sanitized before reaching the model.

Example:

```yaml
tool_surface:
  discovery: "Provider calls tools/list at server startup and refreshes when list_changed is received."
  filtering: "Server-level include/exclude lists can hide tools from the model."
  approval: "MCP tool calls use the same approval prompt as native tools."
  result_handling: "Text/image/resource_link results are passed to the model; tool errors surface with isError."
  annotations_trusted: "Tool annotations are displayed but not treated as trusted policy."
  notes: "No per-argument approval policy; approval prompt shows full arguments."
```

### Resource Surface

`resource_surface` records whether resource listing, reading, templates, subscriptions,
and URI schemes are surfaced. Distinguish resources from tools returning resource links.

Example:

```yaml
resource_surface:
  supported: true
  uri_schemes: ["file", "git", "https", "custom"]
  templates: true
  subscriptions: false
  exposure_model: "Resources appear in a picker; model cannot autonomously read arbitrary resources."
  notes: "Resource links returned by tools are clickable but are not necessarily in resources/list."
```

### Prompt Surface

`prompt_surface` records whether MCP prompt templates are exposed and how users invoke
them. Prompts are user-controlled by protocol design; do not describe them as automatic
model tools unless the provider actually does that.

Example:

```yaml
prompt_surface:
  supported: true
  invocation: "Displayed as slash commands under /mcp:<server>:<prompt>."
  arguments: "Prompt arguments are collected through a command form."
  exposure_model: "User-selected only; the model does not invoke prompts autonomously."
  notes: "Prompt list changes require restarting the session."
```

### Sync Behavior

`sync_behavior` describes what Claudine can automate against persistent provider config.
Be precise about import versus export:

- `import_supported`: Claudine can read provider config and normalize it into its MCP
  catalog.
- `export_supported`: Claudine can write provider-shaped config from its catalog.
- `apply_supported`: Claudine can safely apply changes through provider CLI/API rather
  than editing files directly.
- `merge_strategy`: How provider config combines user/repo/managed/plugin sources.

Example:

```yaml
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: replace
  notes: "Provider reads one JSON file; Claudine must rewrite the mcpServers object atomically."
```

### Runtime Injection

`runtime_injection` is about one-run MCP server injection without permanently mutating
provider user or repo config. This is especially important for Claudine wrappers.

Example:

```yaml
runtime_injection:
  supported: true
  mechanism: "Set PROVIDER_CONFIG_CONTENT to a JSON object containing mcpServers before launching the child process."
  limitations: "Inline config replaces the normal user config for that run; must merge user settings manually."
```

If runtime injection is not supported, state the closest alternative and why it is not
safe for one-run use.

### Authorization

`authorization` records credential handling for MCP servers. For HTTP/Streamable HTTP,
look for OAuth support, token storage, token audience/resource binding, static headers,
and whether credentials are per-user or per-project. For stdio, look for explicit env
vars and whether secrets inherit from the provider process.

Example:

```yaml
authorization:
  oauth: true
  credential_storage: "OAuth tokens stored in ~/.provider/mcp-oauth-tokens.json."
  token_scope: "Per remote server URL; refresh token stored by provider."
  stdio_secret_delivery: "Per-server env object; process env is otherwise inherited."
  notes: "Static Authorization headers are accepted but discouraged for shared repo config."
```

### Security

`security` captures the provider's MCP trust and permission model. This field should
answer how dangerous MCP tools are constrained and whether Claudine needs additional
guardrails.

Example:

```yaml
security:
  tool_filtering: "Per-server allowedTools can hide tools before the model sees them."
  server_trust: "Repo MCP config is ignored until the project is trusted."
  env_sanitization: "Server env vars are explicit per server; process env is otherwise inherited."
  sandbox_interaction: "MCP server subprocesses run outside the provider sandbox unless launched through the sandbox wrapper."
  response_filtering: "No native MCP response sanitization; wrapper must inspect tool results."
  notes: "OAuth tokens are stored in the provider's credential store."
```

Look specifically for:

- server allowlists or denylists
- tool include/exclude filters
- repo trust gates
- admin/managed policy restrictions
- whether MCP subprocesses inherit user env
- where secrets are stored
- whether MCP tools run inside sandbox/container boundaries
- whether the provider scans or filters MCP tool results for prompt injection
- whether roots constrain filesystem-like MCP servers
- whether sampling and elicitation require explicit user consent

### Change Flags

- `changes`: Update-mode changelog entries. Fresh first-run docs should use `[]`.
- `requires_claudine_update`: Set `true` only when the research implies a Claudine code
  or generated metadata change, not merely because documentation changed.
- `reason`: Required when `requires_claudine_update` is `true`; otherwise use an empty
  string or omit if the schema allows.

## Research Questions

- Does the provider support MCP by import/sync, runtime injection, manual config, or not at all?
- Which MCP protocol version/generation and transports does it support: stdio,
  Streamable HTTP, legacy HTTP+SSE/SSE, or custom?
- Where are MCP server definitions stored by OS and scope?
- What server definition shape is accepted for command, HTTP/SSE, stdio, auth, and env?
- Does it expose MCP tools, resources, and prompts? If only tools, say so explicitly.
- Does it expose client capabilities to servers: roots, sampling, and elicitation?
- Are there CLI switches or commands for listing, importing, exporting, applying, or syncing servers?
- Can Claudine inject MCP servers for one run without mutating user config?
- How are server trust, tool filtering, environment sanitization, sandboxing, and response filtering handled?
- Which environment variables affect MCP behavior?
- Does MCP behavior differ between interactive, non-interactive, ACP, IDE, or server modes?
- Are repo-level MCP configs gated by project trust or safe mode?
- Are MCP approval prompts governed by the same permission model as native tools?
- Can individual MCP tools be hidden from the model separately from approval policy?
- Are MCP resources exposed to the user, the model, or both? Are URI templates or subscriptions supported?
- Are MCP prompts exposed as slash commands, command palette entries, or hidden?
- Are MCP roots derived from the workspace, provider config, launch cwd, or user selection?
- Can MCP servers request sampling or elicitation, and can the user deny those requests?
- Where are credentials stored, and can Claudine avoid reading or writing secrets?
- What happens when a configured MCP server fails to start, emits stderr, or hangs?

## Body Structure

- `## Overview`
- `## Protocol and Transports`
- `## Configuration`
- `## Server Definition Shape`
- `## Tools, Resources, and Prompts`
- `## Roots, Sampling, and Elicitation`
- `## Import, Export, and Sync`
- `## Runtime Injection`
- `## Authorization and Credentials`
- `## Security Model`
- `## Mode-Specific Behavior`
- `## Failure Modes`
- `## Gaps`
- `## Claudine Integration Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
