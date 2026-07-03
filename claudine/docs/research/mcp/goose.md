---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
docs: https://goose-docs.ai/docs/getting-started/using-extensions
support: runtime_injection
protocol:
  versions: ["2025-06-18"]
  transports: [stdio, streamable_http]
  lifecycle: |
    Stdio extensions are spawned as local subprocesses at session start.
    Streamable HTTP extensions connect to a remote URL at session start.
    Goose advertises MCP roots during initialization and updates the root if
    the session working directory changes. The docs do not describe mid-session
    reconnection, retry, or dynamic capability refresh behavior.
  notes: |
    Goose was one of the earliest MCP adopters and ships 70+ documented
    extensions. It links MCP Roots to the 2025-06-18 spec and Sampling/Elicitation
    to the draft spec. Legacy SSE / HTTP+SSE transports are not documented.
config_files:
  - os: macos
    scope: user
    path: "~/Library/Application Support/Block/goose/config/config.yaml"
    format: yaml
    notes: |
      Primary user config on macOS. Holds the `extensions` map, provider/model
      settings, and `GOOSE_*` options. There is no documented repo-scoped MCP
      config file.
  - os: linux
    scope: user
    path: "~/.config/goose/config.yaml"
    format: yaml
    notes: |
      Primary user config on Linux. Same `extensions` map shape as macOS.
  - os: windows
    scope: user
    path: "%APPDATA%\\Block\\goose\\config\\config.yaml"
    format: yaml
    notes: |
      Primary user config on Windows. Same `extensions` map shape as macOS/Linux.
  - os: all
    scope: user
    path: "~/.config/goose/secrets.yaml (or platform equivalent)"
    format: yaml
    notes: |
      File-based fallback for API keys and secrets when the system keyring is
      unavailable or disabled via GOOSE_DISABLE_KEYRING. Not an MCP server
      definition file, but stdio extension secrets may be stored here and
      referenced from config.yaml.
  - os: all
    scope: user
    path: "~/.config/goose/permission.yaml"
    format: yaml
    notes: |
      Tool permission levels configured through `goose configure`. Works with
      `GOOSE_MODE` to govern MCP tool approval.
  - os: all
    scope: user
    path: "~/.config/goose/permissions/tool_permissions.json"
    format: json
    notes: |
      Runtime permission decisions (auto-managed). Affects MCP tools the same
      way it affects built-in tools.
  - os: all
    scope: managed
    path: "URL referenced by GOOSE_ALLOWLIST environment variable"
    format: yaml
    notes: |
      Admin-controlled extension allowlist fetched from a URL. Not a local file;
      restricts which extension install commands are permitted.
cli_params:
  - flag: "goose configure"
    description: |
      Interactive TUI for adding, removing, toggling, and configuring extensions
      (MCP servers) and tool permissions.
    example: "goose configure"
  - flag: "goose session --with-extension <command>"
    description: "Enable a stdio MCP extension for the current session only."
    example: |
      goose session --with-extension "npx -y @modelcontextprotocol/server-memory"
  - flag: "goose session --with-streamable-http-extension <url>"
    description: "Enable a remote Streamable HTTP MCP extension for the current session only."
    example: |
      goose session --with-streamable-http-extension "https://example.com/mcp"
  - flag: "goose session --with-builtin <id>"
    description: "Enable a built-in extension for the current session only."
    example: |
      goose session --with-builtin developer
  - flag: "goose run --with-extension <command>"
    description: "Enable a stdio MCP extension for a single non-interactive run."
    example: |
      goose run --with-extension "GITHUB_PERSONAL_ACCESS_TOKEN=<token> npx -y @modelcontextprotocol/server-github" -t "list my repos"
  - flag: "goose run --with-streamable-http-extension <url>"
    description: "Enable a remote Streamable HTTP extension for a single run."
    example: |
      goose run --with-streamable-http-extension "https://example.com/mcp" -t "use the extension"
  - flag: "goose run --with-builtin <id>"
    description: "Enable a built-in extension for a single run."
    example: |
      goose run --with-builtin "developer,computercontroller" -t "run tests"
  - flag: "goose mcp <name>"
    description: "Run an already-enabled MCP extension by name."
    example: "goose mcp github"
  - flag: "/extension <command>"
    description: "Add a stdio extension during an interactive session."
    example: "/extension npx -y @modelcontextprotocol/server-memory"
  - flag: "/builtin <names>"
    description: "Enable built-in extensions during an interactive session."
    example: "/builtin developer"
  - flag: "/prompts [--extension <name>]"
    description: "List MCP prompts, optionally filtered by extension."
    example: "/prompts --extension developer"
  - flag: "/prompt <n> [--info] [key=value...]"
    description: "Execute an MCP prompt by number with arguments."
    example: "/prompt 1 name=my-project"
  - flag: "/mode <auto|approve|smart_approve|chat>"
    description: "Change the permission mode mid-session."
    example: "/mode approve"
  - flag: "--debug"
    description: "Show full tool parameters and responses; useful for MCP troubleshooting."
    example: "goose run --debug --with-extension ..."
env_vars:
  - name: GOOSE_ALLOWLIST
    effect: |
      URL to a YAML allowlist of permitted extension install commands. When set,
      Goose will only install extensions whose command matches an entry in the
      list. Fetched on startup and cached; refetched on restart.
  - name: GOOSE_MODE
    effect: |
      Default permission mode: `auto`, `approve`, `smart_approve`, or `chat`.
      Governs whether MCP tool calls require user approval.
  - name: GOOSE_SEARCH_PATHS
    effect: |
      JSON array of extra directories prepended to PATH when spawning stdio
      extension processes.
  - name: GOOSE_OAUTH_CALLBACK_PORT
    effect: |
      Pin the local OAuth callback port to a fixed value. Required by some
      identity providers for MCP server OAuth and Databricks OAuth.
  - name: GOOSE_PATH_ROOT
    effect: |
      Override the root directory for all Goose data, config, and state files.
      Useful for isolated test/CI environments.
  - name: GOOSE_DISABLE_KEYRING
    effect: |
      Disable system keyring secret storage; secrets fall back to secrets.yaml.
  - name: GOOSE_MAX_TOOL_RESPONSE_SIZE
    effect: |
      Maximum character count for a single tool response before it is written
      to a temporary file instead of being inlined in the conversation.
  - name: GOOSE_SHELL
    effect: |
      Overrides the shell used by the Developer extension's shell tool; may
      affect how stdio extension commands are interpreted if invoked through
      a shell.
  - name: GOOSE_TOOLSHIM
    effect: |
      Enable tool-call interpretation, which can change how MCP tool results
      are summarized or presented to the model.
  - name: SECURITY_PROMPT_ENABLED
    effect: |
      Enable pattern-based prompt injection detection on tool calls,
      including MCP tool calls.
  - name: SECURITY_PROMPT_THRESHOLD
    effect: |
      Sensitivity threshold (0.01-1.0) for prompt injection detection.
  - name: SECURITY_PROMPT_CLASSIFIER_ENABLED
    effect: |
      Enable optional ML-based prompt injection detection with an external
      endpoint.
  - name: SECURITY_PROMPT_CLASSIFIER_ENDPOINT
    effect: |
      URL of the ML classification endpoint for prompt injection detection.
  - name: SECURITY_PROMPT_CLASSIFIER_TOKEN
    effect: |
      Authentication token for the ML classification endpoint.
server_schema:
  transports: [stdio, streamable_http]
  command_fields: ["type", "cmd", "args", "envs", "env_keys", "timeout", "name", "display_name", "description", "bundled", "enabled", "available_tools"]
  http_fields: ["type", "url", "timeout", "name", "display_name", "description", "enabled", "available_tools"]
  env_shape: |
    `envs` is an object mapping variable names to string values. `env_keys`
    lists required environment variable names that must be present. Secrets are
    normally resolved from the system keyring or from environment variables,
    not committed in config.yaml.
  auth_shape: |
    Stdio extensions receive secrets through the `envs` object or inherited
    process environment. Streamable HTTP extensions support custom headers
    (shown in the Figma setup) and OAuth via GOOSE_OAUTH_CALLBACK_PORT. The
    docs do not describe a provider-native OAuth token store.
  notes: |
    Extension id is the map key under `extensions`. `type` values observed in
    docs are `builtin` and `stdio`; remote Streamable HTTP extensions are
    configured through the TUI with a URL and likely use `type: streamable_http`
    or an equivalent internal representation. `available_tools` is a
    per-extension tool allowlist; leave empty to expose all tools.
server_capabilities:
  tools: full
  resources: partial
  prompts: partial
  tool_list_changed: true
  resource_subscribe: false
  resource_list_changed: false
  prompt_list_changed: false
  notes: |
    Tools are fully exposed to the model. Resources and prompts are surfaced
    indirectly: the Extension Manager exposes `list_resources` and
    `read_resource` tools, and prompts are exposed through `/prompts` slash
    commands. Native MCP resource subscriptions, resource templates, and prompt
    list_changed notifications are not documented as user-facing features.
client_capabilities:
  roots: full
  sampling: full
  elicitation: full
  notes: |
    Goose advertises roots support during MCP initialization and exposes the
    session working directory as a single root. MCP Sampling is automatically
    enabled, letting servers ask Goose's LLM for completions. MCP Elicitation
    is automatically enabled and renders forms in the CLI/Desktop UI.
tool_surface:
  discovery: |
    Goose calls `tools/list` at extension startup. The Extension Manager can
    search for and enable additional extensions mid-session. Recommended limit
    is 5 active extensions / 50 total tools.
  filtering: |
    Per-extension `available_tools` limits which tools are loaded. Tool
    permissions (Always Allow / Ask Before / Never Allow) override the global
    GOOSE_MODE for individual tools.
  approval: |
    MCP tools use the same permission model as built-in tools. GOOSE_MODE
    (`auto`, `smart_approve`, `approve`, `chat`) plus per-tool permission levels
    determine whether a user approval prompt is shown.
  result_handling: |
    Tool results are passed to the model. Responses larger than
    GOOSE_MAX_TOOL_RESPONSE_SIZE are written to a temporary file and referenced
    in the conversation instead of being inlined.
  annotations_trusted: unknown
  notes: |
    Tool names appear to the model in the form `<extension>__<tool>`. The
    Extension Manager enables/disables extensions dynamically during a session.
resource_surface:
  supported: true
  uri_schemes: []
  templates: false
  subscriptions: false
  exposure_model: |
    Resources are surfaced through Extension Manager tools (`list_resources`,
    `read_resource`) rather than a native resource picker or model-autonomous
    resource access. The user or model can ask Goose to read a resource.
  notes: |
    Native MCP resource listing, templates, and subscriptions are not
    documented as user-facing features.
prompt_surface:
  supported: true
  invocation: |
    `/prompts [--extension <name>]` lists available prompts; `/prompt <n>
    [--info] [key=value...]` executes a prompt with arguments.
  arguments: |
    Arguments are supplied as `key=value` pairs on the `/prompt` slash command.
  exposure_model: |
    User-selected only via slash commands; the model does not invoke prompts
    autonomously.
  notes: |
    Prompts are part of the MCP protocol surface, but Goose exposes them
    through slash commands rather than a dedicated prompt catalog.
sync_behavior:
  import_supported: true
  export_supported: true
  apply_supported: false
  merge_strategy: shallow
  notes: |
    Claudine can read and write `~/.config/goose/config.yaml` (and platform
    equivalents). There is no dedicated CLI for adding/removing extensions;
    `goose configure` is an interactive TUI, so `apply_supported` is false.
    Extension entries are replaced whole by map key; nested `envs` values are
    replaced by key.
runtime_injection:
  supported: true
  mechanism: |
    Pass `--with-extension`, `--with-streamable-http-extension`, or
    `--with-builtin` to `goose session` or `goose run` to enable MCP extensions
    for the current session/run without mutating `config.yaml`.
  limitations: |
    Runtime extensions are enabled only for the current session and are not
    persisted. They do not participate in the config.yaml merge; Claudine must
    build the desired command-line flags itself. Non-interactive runs cannot
    complete OAuth flows.
authorization:
  oauth: true
  credential_storage: |
    Secrets are stored in the system keyring when available (keychain on macOS,
    equivalent on Linux/Windows). If the keyring is unavailable or
    GOOSE_DISABLE_KEYRING is set, secrets fall back to `secrets.yaml` in plain
    text.
  token_scope: unknown
  stdio_secret_delivery: |
    Per-extension `envs` object in `config.yaml`, plus inherited process
    environment. The docs discourage storing secrets directly in config and
    recommend the keyring.
  notes: |
    Streamable HTTP extensions support custom headers (shown in the Figma
    setup). GOOSE_OAUTH_CALLBACK_PORT supports identity providers that require
    a fixed redirect URI. OAuth token refresh/storage details are not fully
    documented.
security:
  tool_filtering: |
    Per-extension `available_tools` hides tools before the model sees them.
    Tool permissions (Always Allow / Ask Before / Never Allow) and GOOSE_MODE
    govern approval. Adversary mode (`~/.config/goose/adversary.md`) can block
    specific tool calls.
  server_trust: |
    Goose checks external extensions for known malware before activation.
    GOOSE_ALLOWLIST restricts which extension install commands are permitted.
    There is no documented project-trust gate for repo-level MCP config because
    there is no repo-level MCP config.
  env_sanitization: |
    Stdio extensions receive the explicit `envs` map plus inherited process
    environment, plus Goose-set variables: `AGENT_SESSION_ID`, `GOOSE_TERMINAL`,
    and `AGENT`. There is no documented credential-scrubbing mode.
  sandbox_interaction: |
    Stdio extensions run as ordinary local processes. Goose Desktop supports an
    optional macOS sandbox via `GOOSE_SANDBOX` that restricts file/network/process
    access for the desktop app, but stdio extension subprocesses are not
    described as running inside that sandbox.
  response_filtering: |
    Prompt injection detection (pattern matching + optional ML classifier)
    scans tool calls, including MCP tool calls, before execution. Adversary
    mode provides a second LLM-based review. There is no documented MCP
    result/output sanitizer.
  notes: |
    Secrets should prefer the system keyring over `secrets.yaml`. Administrators
    should deploy GOOSE_ALLOWLIST to enforce an organization-wide extension
    surface.
gaps:
  - |
    No documented repo-level or project-level MCP config file; all persistent
    extension config lives in user-scoped `config.yaml`.
  - |
    The exact internal representation of Streamable HTTP extensions in
    `config.yaml` is not shown in public docs (only the TUI flow and deeplink
    format are documented).
  - |
    No dedicated CLI subcommands for listing, importing, exporting, or applying
    MCP server config without using the interactive `goose configure` TUI.
  - |
    OAuth token scope, refresh, and storage semantics for MCP servers are not
    fully documented.
  - |
    It is unclear whether MCP `tool_list_changed`, `resource_list_changed`, or
    `prompt_list_changed` notifications trigger dynamic UI updates.
  - |
    No documented managed config file path; managed control is limited to the
    GOOSE_ALLOWLIST URL.
changes:
  - |
    2026-07-02 — Initial research. Corrects Claudine's prior assumption that
    Goose has no MCP support. Goose has first-class MCP support with stdio and
    Streamable HTTP transports, persistent YAML config, runtime injection flags,
    roots, sampling, and elicitation.
requires_claudine_update: true
reason: |
  Claudine's skill and provider metadata currently classify Goose as having no
  MCP support, but Goose is a first-class MCP client with stdio and Streamable
  HTTP transports, persistent YAML extension config, runtime injection via
  `--with-extension`/`--with-streamable-http-extension`, and full roots/
  sampling/elicitation support. Claudine needs to update its MCP catalog
  mapping, provider support matrix, and wrapper injection path for Goose.
---

# MCP Support in Goose

## Overview

[Goose](https://goose-docs.ai/) (maintained by the Agentic AI Foundation) is a
general-purpose open-source AI agent with a desktop app, CLI, and API. It was
one of the earliest adopters of the [Model Context Protocol
(MCP)](https://modelcontextprotocol.io/) and ships with 70+ documented MCP
extensions. Extensions provide tools, resources, and prompts to the model using
stdio or Streamable HTTP transports.

This document maps Goose's MCP behavior to the schema used by Claudine's MCP
catalog and provider wrappers.

## Protocol and Transports

Goose speaks MCP over two documented transports:

| Transport | Status | How it is added |
| :-------- | :----- | :-------------- |
| `stdio` | Primary | `goose configure` → Command-line Extension, or `--with-extension` |
| `streamable_http` | Primary | `goose configure` → Remote Extension, or `--with-streamable-http-extension` |

- The [MCP Roots guide](https://goose-docs.ai/docs/guides/mcp-roots) references
  the [2025-06-18 MCP spec](https://modelcontextprotocol.io/specification/2025-06-18/client/roots).
- [MCP Sampling](https://goose-docs.ai/docs/guides/mcp-sampling) and
  [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation) link to
  the draft MCP spec.
- Legacy SSE / HTTP+SSE transports are not documented as supported.

Lifecycle behavior is described only at a high level: stdio extensions are
spawned as local subprocesses and Streamable HTTP extensions connect to a remote
URL at session start. Goose advertises roots during initialization and updates
the root if the working directory changes. Mid-session reconnection, retry, and
dynamic capability refresh are not documented.

## Configuration

Goose uses YAML configuration files for persistent settings.

### Primary config file

| OS | Path |
| :- | :--- |
| macOS | `~/Library/Application Support/Block/goose/config/config.yaml` |
| Linux | `~/.config/goose/config.yaml` |
| Windows | `%APPDATA%\Block\goose\config\config.yaml` |

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

### Related files

- `permission.yaml` — tool permission levels configured via `goose configure`.
- `secrets.yaml` — file-based fallback for API keys and secrets.
- `permissions/tool_permissions.json` — auto-managed runtime permission
  decisions.
- `~/.config/goose/adversary.md` — optional adversary reviewer rules.

### Precedence

Environment variables override config file values, which override defaults.
There is no documented repo-scoped MCP config file.

## Server Definition Shape

A single extension under `extensions` in `config.yaml` accepts these fields:

| Field | Applies to | Description |
| :---- | :--------- | :---------- |
| `type` | all | `builtin`, `stdio`, or `streamable_http` (inferred for remote) |
| `name` | all | Internal name |
| `display_name` | all | Human-readable name |
| `description` | all | Description shown in UI |
| `enabled` | all | Whether the extension is active |
| `bundled` | all | Whether the extension ships with Goose |
| `timeout` | all | Tool call timeout in seconds |
| `available_tools` | all | Allowlist of tool names; empty means all |
| `cmd` | stdio | Command to execute |
| `args` | stdio | Argument array |
| `envs` | stdio | Map of environment variables |
| `env_keys` | stdio | Required environment variable names |
| `url` | streamable_http | Streamable HTTP endpoint URL |

The public docs do not show a complete `streamable_http` YAML example, but the
TUI flow and deeplink format (`type=streamable_http&url=...`) confirm the
transport.

## Tools, Resources, and Prompts

### Tools

Goose exposes MCP tools to the model. Tool names are namespaced by extension
(e.g., `github__list_repos`). The [Extension
Manager](https://goose-docs.ai/docs/mcp/extension-manager-mcp) recommends
keeping enabled extensions to 5 or fewer and total tools to 50 or fewer for
best performance.

Tool discovery:

- `tools/list` is called at extension startup.
- The Extension Manager can search for and enable additional extensions
  mid-session.

Tool filtering:

- `available_tools` on each extension limits which tools are loaded.
- Tool permissions (Always Allow / Ask Before / Never Allow) override the
  global permission mode.

### Resources

Goose does not expose a native MCP resource picker. Resources are surfaced
indirectly through the Extension Manager's `list_resources` and `read_resource`
tools. A user or the model can ask Goose to read a resource, but there is no
URI template or subscription support documented.

### Prompts

MCP prompts are exposed through slash commands in interactive sessions:

- `/prompts [--extension <name>]` — list available prompts.
- `/prompt <n> [--info] [key=value...]` — execute a prompt with arguments.

Prompts are user-selected only; the model does not invoke them autonomously.

## Roots, Sampling, and Elicitation

### Roots

Goose advertises roots support during MCP initialization. The root list
contains a single entry: the current session working directory. If the working
directory changes, Goose updates the root and notifies connected extensions.

### Sampling

[MCP Sampling](https://goose-docs.ai/docs/guides/mcp-sampling) is automatically
enabled. MCP servers can ask Goose's configured LLM for completions, allowing
extensions to perform contextual analysis before returning results.

### Elicitation

[MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation) is
automatically enabled. When a server requests structured user input, Goose
renders a form in the CLI or Desktop UI. The request times out after 5 minutes
if not answered.

## Import, Export, and Sync

Claudine can treat Goose as an `import_sync` provider with `apply_supported:
false`:

- **Import**: read `~/.config/goose/config.yaml` and normalize the `extensions`
  map into the MCP catalog.
- **Export**: write provider-shaped YAML back to `config.yaml`.
- **Apply**: Goose has no dedicated CLI for adding/removing extensions; the
  only supported mutation path is the interactive `goose configure` TUI, so
  Claudine should edit `config.yaml` directly.

Merge semantics:

- Extension entries are replaced whole by map key.
- Nested `envs` values are replaced by key.
- Environment variables take precedence over config file values.

## Runtime Injection

For one-run injection without mutating `config.yaml`, Goose provides:

- `--with-extension <command>` — add a stdio extension for the session/run.
- `--with-streamable-http-extension <url>` — add a remote Streamable HTTP
  extension.
- `--with-builtin <id>` — enable a built-in extension.

Example:

```bash
goose run \
  --with-extension "GITHUB_PERSONAL_ACCESS_TOKEN=$TOKEN npx -y @modelcontextprotocol/server-github" \
  -t "list my repositories"
```

Limitations:

- Runtime extensions are not persisted.
- They do not participate in the `config.yaml` merge; Claudine must build the
  flags itself.
- Non-interactive runs cannot complete OAuth flows.

## Authorization and Credentials

Goose supports multiple credential patterns for extensions:

| Pattern | Where configured | Storage |
| :------ | :--------------- | :------ |
| Stdio env var | `envs` map in `config.yaml` | In config or inherited env |
| Stdio env var | System keyring | OS credential store |
| Streamable HTTP header | TUI custom headers | Config or keyring |
| Streamable HTTP OAuth | `GOOSE_OAUTH_CALLBACK_PORT` | Not fully documented |

For stdio extensions, secrets should be passed through the `envs` object or
resolved from the keyring. `GOOSE_OAUTH_CALLBACK_PORT` pins the OAuth callback
port for identity providers that require an exact `redirect_uri`.

## Security Model

### Trust and allowlisting

- Goose checks external extensions for known malware before activation.
- `GOOSE_ALLOWLIST` restricts which extension install commands are permitted.
- There is no project-trust gate because there is no repo-level MCP config.

### Tool filtering and permissions

- `available_tools` limits which tools an extension exposes.
- Tool permission levels (Always Allow / Ask Before / Never Allow) and
  `GOOSE_MODE` govern approval.
- Adversary mode (`~/.config/goose/adversary.md`) provides an independent
  LLM-based review of tool calls.

### Environment and sandboxing

- Stdio extensions inherit the user's process environment plus explicit `envs`,
  plus Goose-set variables `AGENT_SESSION_ID`, `GOOSE_TERMINAL`, and `AGENT`.
- Goose Desktop supports an optional macOS sandbox via `GOOSE_SANDBOX`, but
  stdio extension subprocesses are not described as running inside it.

### Response handling

- Prompt injection detection scans tool calls (including MCP tool calls) before
  execution using pattern matching and an optional ML classifier.
- There is no documented native MCP result sanitizer.

## Mode-Specific Behavior

### Interactive mode

- Extensions can be added/toggled via `goose configure` or slash commands
  (`/extension`, `/builtin`).
- OAuth flows can complete through the UI.
- MCP prompts are available via `/prompts` and `/prompt`.
- Working directory changes update MCP roots.

### Non-interactive / headless mode (`goose run`)

- OAuth flows cannot complete; pre-authenticated servers or env-based secrets
  are required.
- Runtime extension injection works through `--with-extension`,
  `--with-streamable-http-extension`, and `--with-builtin`.
- Use `--no-session` for CI runs where session storage is not needed.

### Goose as an MCP server

`goose mcp <name>` runs an already-enabled MCP extension by name. `goose acp`
runs Goose itself as an ACP agent server over stdio for ACP-compatible clients.

## Failure Modes

| Failure | Behavior |
| :------ | :------- |
| Extension fails to start | Error shown in session; extension tools unavailable |
| Streamable HTTP unreachable | Connection error at startup |
| Tool timeout | Per-extension `timeout` aborts the call |
| Large tool output | Written to temp file and referenced when over `GOOSE_MAX_TOOL_RESPONSE_SIZE` |
| Elicitation timeout | Cancelled after 5 minutes if user does not respond |
| Prompt injection detected | Security alert with Allow/Deny choice |
| Disallowed extension install | Rejected if `GOOSE_ALLOWLIST` is set and command is not matched |

## Gaps

- No documented repo-level MCP config file.
- Exact YAML representation of Streamable HTTP extensions is not shown in
  public docs.
- No dedicated CLI for listing/importing/exporting MCP servers without the
  interactive TUI.
- OAuth token scope, refresh, and storage semantics are not fully documented.
- Dynamic capability update behavior (`list_changed` notifications) is not
  explicitly documented.
- No documented managed config file path beyond the `GOOSE_ALLOWLIST` URL.

## Claudine Integration Notes

- Treat Goose as `support: runtime_injection` with `sync_behavior` describing
  file-based import/export.
- Map Claudine's normalized MCP catalog to Goose's `extensions` map in
  `~/.config/goose/config.yaml`.
- For one-run wrappers, inject servers with `--with-extension` and
  `--with-streamable-http-extension`; built-ins with `--with-builtin`.
- There is no safe apply CLI; Claudine must write `config.yaml` directly.
- Do not assume repo-level MCP config exists; Goose extension config is
  user-scoped.
- Honor `GOOSE_MODE` and tool permissions when deciding whether MCP tool calls
  need extra guardrails.
- Defensively scan MCP tool results; Goose provides prompt-injection detection
  on tool calls but no documented result sanitizer.

## Changelog

- **2026-07-02** — Initial research. Discovered that Goose has first-class MCP
  support (stdio and Streamable HTTP), persistent YAML extension config,
  runtime injection flags, roots, sampling, and elicitation. Corrects
  Claudine's prior "no MCP support" classification.

## Sources

- [Goose homepage](https://goose-docs.ai/)
- [Using Extensions — MCP overview](https://goose-docs.ai/docs/getting-started/using-extensions)
- [Configuration Files](https://goose-docs.ai/docs/guides/config-files)
- [Environment Variables](https://goose-docs.ai/docs/guides/environment-variables)
- [CLI Commands](https://goose-docs.ai/docs/guides/goose-cli-commands)
- [MCP Roots](https://goose-docs.ai/docs/guides/mcp-roots)
- [MCP Sampling](https://goose-docs.ai/docs/guides/mcp-sampling)
- [MCP Elicitation](https://goose-docs.ai/docs/guides/mcp-elicitation)
- [Extension Manager](https://goose-docs.ai/docs/mcp/extension-manager-mcp)
- [Extension Allowlist](https://goose-docs.ai/docs/guides/allowlist)
- [goose Permission Modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions)
- [Managing Tool Permissions](https://goose-docs.ai/docs/guides/managing-tools/tool-permissions)
- [Prompt Injection Detection](https://goose-docs.ai/docs/guides/security/prompt-injection-detection)
- [Adversary Mode](https://goose-docs.ai/docs/guides/security/adversary-mode)
- [Figma Extension (remote Streamable HTTP example)](https://goose-docs.ai/docs/mcp/figma-mcp)
- [Goose GitHub repository](https://github.com/aaif-goose/goose)
