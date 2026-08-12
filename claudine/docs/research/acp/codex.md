---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://developers.openai.com/codex/cli
acp_docs: https://agentclientprotocol.com/
repo: https://github.com/openai/codex
support: adapter
launch_modes:
  - command: npx -y @agentclientprotocol/codex-acp
    args: []
    transport: stdio
    adapter: "@agentclientprotocol/codex-acp (ACP-org TypeScript adapter; bundles @openai/codex)"
    notes: >-
      Current recommended adapter (v1.1.0, 2026-07-02). Spawns the bundled Codex App
      Server over a private JSON-RPC channel and translates ACP JSON-RPC to/from it.
      Stderr is reserved for adapter/Codex logs; stdout carries newline-delimited
      JSON-RPC. Adapter name resolves at runtime to the value of `acp.agent({name})`
      — the agent advertises `name: "@agentclientprotocol/codex-acp", title: "Codex"`.
  - command: npx -y @zed-industries/codex-acp
    args: []
    transport: stdio
    adapter: "@zed-industries/codex-acp (DEPRECATED, pinned to v0.16.0)"
    notes: >-
      Legacy Zed-maintained adapter. npm description: "DEPRECATED — This package has
      been replaced by @agentclientprotocol/codex-acp." Last published 2026-06-08.
      The Zed repo's `main` branch remains accessible but has no tags after v0.16.0.
  - command: codex
    args: []
    transport: other
    adapter: none
    notes: >-
      The main Codex CLI binary (v0.142.5 installed locally) has no native ACP mode.
      Direct probes on 0.142.5: `codex --acp` → "unexpected argument '--acp'";
      `codex acp` → "unrecognized subcommand 'acp'". `codex app-server --listen
      stdio://` exposes the Codex App Server protocol, but that is a separate JSONL
      protocol, not ACP, and the adapter speaks it over a private channel rather
      than via the CLI's stdio surface.
protocol_versions:
  - "v1 (agent-client-protocol-schema 1.1.0)"
capabilities:
  - capability: initialize
    support: supported
    notes: >-
      Standard ACP `initialize` handshake with `ProtocolVersion` (1),
      `ClientCapabilities`, `Implementation`, and `authMethods`. Adapter emits
      `agentInfo` with `name: "@agentclientprotocol/codex-acp"`, `title: "Codex"`,
      `version: "1.1.0"` (from the bundled package.json).
  - capability: authenticate
    support: supported
    notes: >-
      Three advertised auth methods: `api-key` (always), `chat-gpt` (unless
      `NO_BROWSER=1`), and `gateway` (only when client opts in via
      `clientCapabilities.auth._meta.gateway === true`). API key is supplied via
      `_meta["api-key"].apiKey` in the `AuthenticateRequest`.
  - capability: session_new
    support: supported
    notes: >-
      `session/new` creates a conversation session tied to a working directory.
      Accepts optional `additionalDirectories` (absolute paths) and `mcpServers`.
      Response includes initial `models`, `modes`, and `configOptions` (mode, model,
      reasoning effort, and fast-mode toggle when supported by the chosen model).
  - capability: session_load
    support: supported
    notes: >-
      `session/load` resumes an existing session by id. Capability advertised
      (`loadSession: true`). Streams the conversation history back as
      `session/update` notifications before the response.
  - capability: session_prompt
    support: supported
    notes: >-
      `session/prompt` is the primary turn-taking method. `prompt` is
      `ContentBlock[]`; the adapter advertises
      `promptCapabilities.embeddedContext: true` and `promptCapabilities.image: true`.
      Cancellation is supported via the request signal.
  - capability: session_cancel
    support: supported
    notes: >-
      `session/cancel` is a notification that stops the current turn. Adapter also
      handles protocol-level `$/cancel_request` (forwarded to the request signal)
      so any in-flight request to the adapter can be aborted.
  - capability: session_modes
    support: supported
    notes: >-
      Three modes: `read-only` (requires approval to edit/run), `agent` (workspace
      write, no network), and `agent-full-access` (danger-full-access, network on).
      Initial mode configurable via `INITIAL_AGENT_MODE` env var. Switch with
      `session/set_mode`; agent emits `current_mode_update` notifications.
  - capability: streaming
    support: supported
    notes: >-
      `session/update` notifications stream text, thoughts, tool calls, plan, mode,
      commands, and config-option changes. Adapter also streams tool progress,
      file changes, MCP tool calls, terminal output, web search, image
      generation, image view, token usage, and review events.
  - capability: permissions
    support: supported
    notes: >-
      Reverse request `session/request_permission` is used for command execution,
      file change, MCP elicitation, and permissions-profile approvals. Approval
      options include `allow_once`/`allow_always`/`reject_once` plus Codex-specific
      decisions such as execpolicy amendment and network policy amendment.
  - capability: fs_read
    support: unsupported
    notes: >-
      The adapter does not register an `fs/read_text_file` handler. Filesystem
      reads happen inside the Codex App Server process via Codex's own tools.
      Clients advertising `fs.readTextFile: true` will not see reverse requests
      from this adapter.
  - capability: fs_write
    support: unsupported
    notes: >-
      Same as fs_read — writes go through Codex's built-in tools, not via
      `fs/write_text_file` reverse requests.
  - capability: terminal
    support: unsupported
    notes: >-
      The adapter does not register `terminal/*` handlers. Commands run inside
      the Codex App Server and surface as tool calls (`kind: "execute"`) with
      streamed progress. Output bytes are carried in
      `_meta.terminal_output_delta` (default) or `_meta.terminal_output`
      (when client opts in via `clientCapabilities._meta.terminal_output: true`).
  - capability: mcp
    support: partial
    notes: >-
      `mcpCapabilities.http: true`, `acp: false`, `sse: false`. Clients may pass
      HTTP-transport MCP servers in `session/new` (`mcpServers`) and Codex will
      manage them. SSE and ACP-protocol MCP servers are not exposed through the
      adapter. Codex-side MCP servers (managed via `codex mcp`) remain accessible
      through Codex's own tool surface.
  - capability: media
    support: supported
    notes: >-
      Images are accepted in `promptCapabilities.image`. Image generation and
      image view surface as `tool_call`/`tool_call_update` events via
      `session/update`.
  - capability: plans
    support: supported
    notes: >-
      Adapter emits `plan` events as `session/update` notifications. Plan items
      from thread history are replayed on `session/load`.
  - capability: extensions
    support: supported
    notes: >-
      Adapter registers three extension methods: `authentication/status`,
      `authentication/logout`, and the legacy `session/set_model`. `_meta`
      fields are used to carry Codex-specific data (e.g. `codex` approval params,
      `is_mcp_tool_approval`, `terminal_output_delta`).
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: >-
      Required for every approval flow. The client must present the supplied
      `options` (allow_once/allow_always/reject_once plus any Codex-specific
      decision variants) and reply with `Selected` carrying the chosen
      `option_id`, or `Cancelled`.
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: >-
      Not issued by the current adapter. Implement only as a matter of general
      ACP completeness.
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: >-
      Not issued by the current adapter. Implement only as a matter of general
      ACP completeness.
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: >-
      Not issued by the current adapter.
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: >-
      Not issued by the current adapter.
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: >-
      Not issued by the current adapter.
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: >-
      Not issued by the current adapter.
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: false
    notes: >-
      Not issued by the current adapter.
permission_model:
  mechanism: session/request_permission reverse request
  timeout: client-defined
  default_policy: no default; every approval request must receive a Selected or Cancelled response
  approval_values:
    - allow_once
    - allow_always
    - reject_once
  notes: >-
    Codex approval flows are richer than the generic ACP surface. Beyond
    allow/reject choices, the adapter may include Codex-specific decisions via
    `_meta.codex` on the option (execpolicy amendment, network policy amendment,
    root-grant, MCP tool-call approval with persist scope). Clients may either
    surface those to the user as raw options or render them as plain
    allow/reject buttons. On `session/cancel` or protocol-level `$/cancel_request`,
    the adapter propagates the abort signal to in-flight permission requests.
filesystem_model:
  read_methods: []
  write_methods: []
  path_base: not applicable — adapter does not invoke client filesystem reverse requests
  sandboxing: client-side via project-root policy; Codex also enforces sandbox mode
  notes: >-
    The Codex adapter does not delegate file I/O to the client. Path and sandbox
    policy is enforced by Codex's sandbox modes (`read-only`, `workspace-write`,
    `danger-full-access`) and the client's per-project approval rules.
terminal_model:
  supported: false
  methods: []
  shell: not applicable — commands run inside the Codex App Server
  cwd: not applicable
  streaming: not applicable
  cancellation: not applicable
  notes: >-
    The current adapter does not invoke `terminal/*` reverse requests. Tool
    executions surface as `tool_call` events with `kind: "execute"`; output
    deltas are delivered through `_meta.terminal_output_delta` (or
    `_meta.terminal_output` when negotiated). Implement terminal handlers as a
    general ACP client but do not expect them to fire with this adapter.
streaming_model:
  update_methods:
    - session/update
  text_events:
    - agent_message_chunk
    - agent_thought_chunk
    - user_message_chunk
  tool_events:
    - tool_call
    - tool_call_update
  plan_events:
    - plan
  error_events:
    - "JSON-RPC errors are returned on the request channel; notifications are fire-and-forget"
  notes: >-
    Updates are fire-and-forget notifications with no `id`. Text chunks include
    `messageId` (added in adapter v1.1.0) so clients can group parallel chunks.
    Mode changes surface via `current_mode_update`; slash command availability
    via `available_commands_update`; config option changes via
    `config_option_update`. Codex-specific tool kinds: command execution,
    file change, MCP tool call, dynamic tool call, collab agent tool call,
    web search, image generation, image view.
auth_setup:
  required: true
  mechanisms:
    - "ChatGPT OAuth login (methodId: chat-gpt)"
    - "API key via methodId: api-key and _meta[api-key].apiKey"
    - "Custom OpenAI-compatible gateway (methodId: gateway, capability-gated)"
  headless_notes: >-
    For headless operation, set `CODEX_API_KEY` (preferred) or `OPENAI_API_KEY`
    so the adapter can authenticate without a browser. Set `NO_BROWSER=1` to
    remove the `chat-gpt` method from the advertised list. Seed `~/.codex/auth.json`
    from a machine that completed ChatGPT login if running in a headless
    environment without API keys.
  notes: >-
    Adapter reads auth from `~/.codex/auth.json` via the bundled Codex runtime.
    ChatGPT tokens auto-refresh; corrupted `auth.json` triggers automatic logout
    (LLM-28118 fix).
env_vars:
  - name: CODEX_API_KEY
    effect: Preferred API key for the `api-key` auth method. Takes precedence over OPENAI_API_KEY.
  - name: OPENAI_API_KEY
    effect: Fallback API key for the `api-key` auth method.
  - name: NO_BROWSER
    effect: Removes the `chat-gpt` method from advertised auth methods.
  - name: CODEX_PATH
    effect: Run a specific Codex executable instead of the bundled `@openai/codex`.
  - name: CODEX_CONFIG
    effect: JSON object merged into the Codex session config.
  - name: MODEL_PROVIDER
    effect: Model provider id passed to Codex for new sessions (e.g. `openai`, `custom-gateway`).
  - name: DEFAULT_AUTH_REQUEST
    effect: ACP `AuthenticateRequest` JSON used when Codex requires authentication on first turn.
  - name: INITIAL_AGENT_MODE
    effect: "Initial `AgentMode` id: `read-only`, `agent`, or `agent-full-access`."
  - name: APP_SERVER_LOGS
    effect: Directory where adapter/Codex App Server logs are written.
  - name: CODEX_ACCESS_TOKEN
    effect: Direct ChatGPT/Codex access token for trusted automation.
  - name: CODEX_HOME
    effect: Root for Codex state (config, auth, logs, sessions).
rust_client:
  crate: agent-client-protocol
  connection_type: AcpAgent subprocess over stdio (JSON-RPC)
  localset_required: false
  reverse_request_handlers:
    - session/request_permission
  desktop_streaming_pattern: >-
    `tokio::sync::mpsc` from the notification handler to the UI thread; run the
    ACP client on a dedicated tokio runtime. Connection is `Send`/`Sync` as of
    agent-client-protocol 1.0.1.
  notes: >-
    Use `AcpAgent::from_str("npx -y @agentclientprotocol/codex-acp")` to launch
    the current adapter. The crate's preset helpers may point at the deprecated
    `@zed-industries/codex-acp` package; check before relying on a preset.
    Terminal/fs reverse requests are not used by this adapter but should be
    wired in as a general ACP client.
compatibility:
  - client: Zed
    status: works
    issue: >-
      Zed ships the Codex integration via ACP; the legacy `@zed-industries/codex-acp`
      package was maintained by Zed before being moved to the `agentclientprotocol`
      org in mid-2026.
    workaround: Use Zed's built-in Codex integration; the adapter is fetched automatically.
  - client: JetBrains IDEs
    status: partial
    issue: >-
      Session config options (`SessionConfigOption`) are intentionally disabled for
      JetBrains 2026.1 due to upstream issues (LLM-28118); the adapter detects the
      client by name and skips config-option emission. The rest of the protocol
      works.
    workaround: Upgrade to a JetBrains release after 2026.1 once the upstream issue is resolved.
  - client: Neovim (CodeCompanion)
    status: works
    issue: none known
    workaround: Configure the adapter as a stdio ACP agent via CodeCompanion's ACP support.
  - client: agent-client-protocol Rust SDK 0.9.x
    status: broken
    issue: Connection futures were `!Send` and required `tokio::task::LocalSet`.
    workaround: Upgrade to agent-client-protocol 1.0.1 or later.
  - client: agent-client-protocol Rust SDK 1.0.1
    status: works
    issue: None known. Schema is 1.1.0.
    workaround: None.
recent_changes:
  - date: 2026-07-02
    version: "@agentclientprotocol/codex-acp v1.1.0"
    change: >-
      ACP SDK bumped to v1.1.0. Added message IDs to text session chunks,
      boolean Fast mode config option support, completed image generation items
      surfaced as tool calls, goal changes emitted as session metadata, vscode-jsonrpc
      upgraded to v9. Bundled `@openai/codex` bumped to 0.142.5.
    impact: >-
      Clients can group text chunks by `messageId`. Clients that advertise
      `session.configOptions.boolean` receive a boolean Fast mode toggle instead
      of the legacy select. Image generation completion is now visible as a tool
      call rather than a free-floating event.
  - date: 2026-06-29
    version: "@agentclientprotocol/codex-acp v1.0.2"
    change: >-
      Bundled `@openai/codex` bumped to 0.142.3 and 0.142.4. Added `/goal` slash
      command support. Fixed skill listing to use session cwd. Removed Fast mode
      config option for models that don't support it.
    impact: >-
      New `/goal` slash command is advertised via `available_commands_update`.
      Fast mode config no longer appears when the active model lacks `fast`
      support.
  - date: 2026-06-26
    version: "@agentclientprotocol/codex-acp v1.0.1"
    change: >-
      ACP SDK bumped to 1.0.0. Added ACP request cancellation (`$/cancel_request`)
      handling. Mapped collab agent tool call events to `tool_call`. API-key auth
      now reads from `CODEX_API_KEY` / `OPENAI_API_KEY` env vars. Auto-skips
      ChatGPT login when already authenticated.
    impact: >-
      Adapter honors protocol-level `$/cancel_request` notifications. Collab agent
      tool calls (sub-agent activity) now appear in the ACP transcript.
  - date: 2026-06-23
    version: "@agentclientprotocol/codex-acp v1.0.0"
    change: >-
      First stable v1 release. Added `session/delete`, more informative
      permission approvals, embedded resource blob handling in prompts,
      `additionalDirectories` support, reasoning events streamed as agent
      thoughts, automatic logout on corrupted auth.json, and `session_config`
      negotiation. Bundled `@openai/codex` bumped to 0.139.0–0.141.0 across the
      cycle.
    impact: >-
      Stable v1.x contract. `agentCapabilities.sessionCapabilities.delete: {}`
      and `additionalDirectories: {}` are now advertised. Corrupted auth.json
      triggers `logout` and surfaces a re-auth error to the client.
  - date: 2026-06-08
    version: "@zed-industries/codex-acp v0.16.0"
    change: >-
      Final Zed-maintained release before the migration to the `agentclientprotocol`
      org. npm description later updated to mark the package deprecated.
    impact: >-
      Existing installs keep working but receive no further updates; new installs
      should use `@agentclientprotocol/codex-acp`.
quirks:
  - >-
    Codex CLI has no native ACP mode. Direct probes on the installed
    `codex-cli 0.142.5`: `codex --acp` returns "unexpected argument '--acp'";
    `codex acp` returns "unrecognized subcommand 'acp'". The only protocol-mode
    subcommand is `codex app-server --listen stdio://`, which speaks the Codex
    App Server protocol (not ACP) and is what the adapter uses internally over
    a private stdio channel.
  - >-
    Auth method names changed between the legacy Zed adapter and the current
    ACP-org adapter. The legacy docs used `chatgpt`, `codex-api-key`, and
    `openai-api-key`; the current adapter advertises `api-key`, `chat-gpt`,
    and `gateway`. `api-key` is supplied via `_meta["api-key"].apiKey` and
    the adapter picks up the value from `CODEX_API_KEY` / `OPENAI_API_KEY` env
    vars if set.
  - >-
    `gateway` auth is capability-gated: it appears in `authMethods` only when
    the client opts in by sending `clientCapabilities.auth._meta.gateway: true`
    in `initialize`. It uses `_meta["gateway"]` with `baseUrl`, `headers`, and
    optional `providerName`.
  - >-
    `mcpCapabilities.acp: false` and `mcpCapabilities.sse: false` — the adapter
    only honours HTTP-transport MCP servers passed via `session/new`. SSE and
    ACP-transport MCP servers are not exposed through this adapter.
  - >-
    Fast mode is conditional: `createFastModeConfigOption` only emits the
    `fast-mode` config option when `modelSupportsFast` returns true for the
    current model. Clients may see Fast mode appear or disappear as the active
    model changes.
  - >-
    JetBrains 2026.1 clients have session config options intentionally disabled
    (`isSessionConfigEnabled()` returns false for `clientInfo.name === ...`),
    so `SessionConfigOption` payloads are absent for that client class until
    the upstream issue (LLM-28118) is resolved.
  - >-
    Corrupted `~/.codex/auth.json` triggers an automatic logout via
    `codexAcpClient.logout()`. The next request that needs auth fails with
    `RequestError.authRequired()` unless `DEFAULT_AUTH_REQUEST` is set.
  - >-
    The adapter bundles `@openai/codex ^0.142.5` by default; if a client wants
    a different Codex binary, set `CODEX_PATH`. The bundled version is
    intentionally pinned — versions other than the one specified in
    `package.json` may not be compatible.
  - >-
    The adapter enforces a 2-second grace period between stdin close and
    `codex` SIGKILL: if stdin closes, the adapter closes the Codex process'
    stdin, then kills it after 2s if it has not exited.
  - >-
    Stderr is reserved for adapter/Codex logs (2 KB rolling tail retained for
    crash diagnostics). Clients should never write to the adapter's stderr.
  - >-
    `INITIAL_AGENT_MODE` env var is honoured only at session creation; it
    does not retroactively change the mode of an existing session.
gaps:
  - >-
    No official OpenAI-maintained ACP adapter; the canonical adapter is
    maintained by the ACP org. The legacy `@zed-industries/codex-acp` package
    is deprecated.
  - >-
    Empirical MCP-over-ACP behavior beyond the `http: true` capability is not
    formally documented outside the adapter source.
  - >-
    Codex-specific extension methods (`authentication/status`,
    `authentication/logout`, `session/set_model`) are documented only in the
    adapter source; they are not part of the published ACP spec.
  - >-
    Tool call payload formats for Codex-specific kinds (collab agent, web
    search, image generation, image view) are not formally documented; clients
    must consume them from the adapter source.
changes:
  - >-
    Verified `codex` CLI 0.142.5 has no native ACP entry point; recorded exact
    error strings for `codex --acp` and `codex acp`.
  - >-
    Corrected the auth method catalog: prior research listed `chatgpt`,
    `codex-api-key`, `openai-api-key`. The current adapter advertises
    `api-key`, `chat-gpt`, and `gateway` (capability-gated).
  - >-
    Documented new auth method: `gateway` for custom OpenAI-compatible
    gateways, surfaced only when client opts in via
    `clientCapabilities.auth._meta.gateway === true`.
  - >-
    Recorded session config options: the adapter emits `mode`, `model`,
    `reasoning effort`, and (when supported) `fast-mode` as
    `SessionConfigOption` entries. Boolean fast-mode support is detected via
    `clientCapabilities.session.configOptions.boolean`.
  - >-
    Clarified that `fs/read_text_file`, `fs/write_text_file`, and the
    `terminal/*` reverse requests are NOT issued by the current adapter. Path
    and terminal policy is enforced inside the Codex App Server.
  - >-
    Clarified `mcpCapabilities` flags: only `http: true`; `acp: false` and
    `sse: false`.
  - >-
    Recorded the JetBrains 2026.1 session_config disablement (LLM-28118) as
    an in-adapter workaround.
  - >-
    Added three extension methods (`authentication/status`,
    `authentication/logout`, legacy `session/set_model`) to the protocol
    surface.
  - >-
    Documented `INITIAL_AGENT_MODE`, `APP_SERVER_LOGS`, `MODEL_PROVIDER`, and
    `DEFAULT_AUTH_REQUEST` env vars as first-class adapter knobs.
  - >-
    Confirmed legacy `@zed-industries/codex-acp` v0.16.0 is the last Zed
    release (2026-06-08) and the npm description marks it deprecated.
  - >-
    Confirmed `@agentclientprotocol/codex-acp` v1.1.0 (2026-07-02) is the
    current adapter; bundled `@openai/codex` is 0.142.5 (matching the locally
    installed Codex CLI version).
requires_claudine_update: true
reason: >-
  Codex CLI ACP support is adapter-based and continues to evolve on a separate
  cadence from the underlying Codex CLI. To wire Codex into Claudine's lifecycle
  pipeline over ACP, Claudine needs launch-mode detection for both
  `@agentclientprotocol/codex-acp` and the deprecated `@zed-industries/codex-acp`;
  capability negotiation that includes session config options (mode, model,
  reasoning effort, fast mode) and HTTP-only MCP; reverse-request routing for
  the single reverse request the adapter actually issues (`session/request_permission`
  with Codex-specific approval options like execpolicy and network policy
  amendments); permission policy integration for the three Codex modes
  (`read-only`, `agent`, `agent-full-access`); and headless auth detection
  (API key vs ChatGPT OAuth vs custom gateway).
---

# Codex CLI and the Agent Client Protocol

## Overview

Codex CLI is OpenAI's local coding agent. As of **Codex CLI 0.142.5** (the version installed at research time, 2026-07-02), the main `codex` binary **does not implement the Agent Client Protocol natively**. Direct probes on the installed binary return `unexpected argument '--acp'` for `codex --acp` and `unrecognized subcommand 'acp'` for `codex acp` — the only protocol-mode subcommand the binary ships is `codex app-server --listen stdio://`, which speaks the Codex App Server protocol (a separate JSONL protocol), not ACP.

ACP support is therefore provided by an **adapter/bridge** process that translates between:

1. **ACP** — JSON-RPC 2.0 over stdio, schema v1 (1.1.0), spoken by editors and ACP clients.
2. **Codex App Server protocol** — a private JSON-RPC stream the adapter opens against the bundled Codex runtime.

The canonical adapter today is the TypeScript package [`@agentclientprotocol/codex-acp`](https://github.com/agentclientprotocol/codex-acp), currently at **v1.1.0** (released 2026-07-02). It bundles `@openai/codex ^0.142.5` as its Codex runtime and advertises itself over ACP as `name: "@agentclientprotocol/codex-acp", title: "Codex", version: "1.1.0"`. The earlier Rust adapter, [`@zed-industries/codex-acp`](https://github.com/zed-industries/codex-acp), shipped its last release **v0.16.0** on 2026-06-08 and is now flagged **DEPRECATED** on npm with the message *"This package has been replaced by @agentclientprotocol/codex-acp."*

For Claudine's future ACP client/adapter work this means Codex CLI must be treated as an **adapter-launched provider**: the client spawns the adapter, negotiates ACP capabilities, and must be prepared to handle the single reverse request the adapter actively issues — `session/request_permission` — with Codex-specific approval semantics (execpolicy amendments, network policy amendments, MCP tool-call persistence scopes).

## Launching ACP

### Current recommended adapter

```bash
npx -y @agentclientprotocol/codex-acp
```

The adapter bundles `@openai/codex ^0.142.5`, opens a private JSON-RPC channel against the bundled Codex App Server, and translates ACP requests to/from it. All ACP traffic uses newline-delimited JSON-RPC 2.0 over stdio; **stderr is reserved for adapter/Codex logs** and a 2 KB rolling tail is kept for crash diagnostics. The adapter enforces a 2-second grace period between stdin close and a `codex` SIGKILL.

Standalone single-file binaries (`codex-acp-<arch>-<os>`) are published in the GitHub release artifacts and can be unzipped and invoked directly when Node.js is unavailable. Building them locally requires `bun`.

### Legacy Zed adapter (deprecated)

```bash
npx -y @zed-industries/codex-acp
```

The Zed-published adapter remains resolvable on npm (last release v0.16.0, 2026-06-08) but the npm description explicitly marks it **DEPRECATED** in favor of `@agentclientprotocol/codex-acp`. The Zed `codex-acp` repository's `main` branch has no tags after v0.16.0 and is no longer receiving feature work.

### No native launch mode

Direct probes on `codex-cli 0.142.5`:

```text
$ codex --acp
error: unexpected argument '--acp' found

$ codex acp
error: unrecognized subcommand 'acp'

Usage: codex [OPTIONS] [PROMPT]
       codex [OPTIONS] <COMMAND] [ARGS]
```

The `codex app-server --listen stdio://` subcommand exposes the **Codex App Server protocol** (a separate JSONL protocol, not ACP). The adapter consumes that protocol over a private channel — it does not proxy ACP JSON-RPC over `codex app-server --listen`.

## Protocol and Capabilities

### Transport and framing

- **Transport**: stdio pipes between the ACP client and the adapter.
- **Framing**: newline-delimited JSON-RPC 2.0 (`vscode-jsonrpc ^9`).
- **Encoding**: UTF-8.
- **Direction**: client sends requests/notifications to the adapter; the adapter sends responses, reverse requests, notifications, and the `@agentclientprotocol/sdk` internal log lines on stderr.

### Supported protocol version

Both the adapter (v1.1.0) and the underlying `@agentclientprotocol/sdk` (v1.1.0) negotiate **ACP v1 / schema 1.1.0**. The adapter reports `protocolVersion: acp.PROTOCOL_VERSION` (= 1) in its `initialize` response.

### Capability surface

| Area | Status | Notes |
|------|--------|-------|
| `initialize` / `authenticate` / `logout` | supported | Advertises `agentCapabilities.auth.logout: {}`; `logout` request is handled. |
| `session/new` / `session/load` / `session/prompt` / `session/cancel` | supported | `cancel` is a notification; protocol-level `$/cancel_request` is also honored (forwarded to the request signal). |
| `session/resume` / `session/list` / `session/close` / `session/delete` | supported | Advertised in `agentCapabilities.sessionCapabilities`. |
| `session/set_mode` / `session/set_config_option` | supported | Three modes (read-only, agent, agent-full-access). Config options: mode, model, reasoning effort, fast mode (when model supports it). |
| `session/request_permission` | supported | The only reverse request the adapter actively issues. Carries Codex-specific decisions (`allow_once`, `allow_always`, execpolicy amendment, network policy amendment, MCP tool approval with persist scope). |
| `fs/read_text_file` / `fs/write_text_file` / `terminal/*` | unsupported (by current adapter) | The adapter does not register handlers for these methods. Reads, writes, and command execution happen inside the Codex App Server via Codex's own tools. |
| `session/update` streaming | supported | Text, thoughts, tool calls, plans, mode/commands/config updates, plus Codex-specific kinds. |
| MCP (`mcpCapabilities.http: true`, `acp: false`, `sse: false`) | partial | HTTP-transport MCP servers passed in `session/new` are accepted; ACP-protocol and SSE MCP servers are not exposed through the adapter. |
| Image (`promptCapabilities.image: true`) | supported | Image inputs, image generation completion (`tool_call`), and image view events. |
| Embedded context (`promptCapabilities.embeddedContext: true`) | supported | |
| Plan events | supported | `plan` session-update variant emitted; plan items replayed on `session/load`. |
| Extension methods (`authentication/status`, `authentication/logout`, legacy `session/set_model`) | supported | Underscore-free extension methods used by older clients. |
| Protocol-level `$/cancel_request` | supported | Adapter propagates the abort signal to in-flight requests and pending permission prompts. |

## Reverse Requests

The current adapter registers **exactly one reverse request method** for clients: `session/request_permission`. The remaining entries below are kept for schema completeness and for clients that want to wire up general ACP support for other agents sharing the same code path.

### Permission requests (required)

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "session/request_permission",
  "params": {
    "sessionId": "sess_abc123",
    "toolCall": {
      "toolCallId": "call_xyz",
      "title": "Run `cargo build`",
      "kind": "execute",
      "status": "pending",
      "rawInput": { "command": "cargo build", "cwd": "/project" }
    },
    "options": [
      { "optionId": "allow_once", "name": "Allow Once", "kind": "allow_once" },
      { "optionId": "allow_session", "name": "Allow for Session", "kind": "allow_always" },
      { "optionId": "apply_execpolicy_amendment", "name": "Allow Commands Starting With `cargo build`", "kind": "allow_always" },
      { "optionId": "decline", "name": "Reject", "kind": "reject_once" }
    ],
    "_meta": { "codex": { "params": { ... } } }
  }
}
```

The client must respond with `RequestPermissionOutcome::Selected` carrying the chosen `option_id`, or `RequestPermissionOutcome::Cancelled`. On `session/cancel` or `$/cancel_request`, the adapter propagates the abort signal and pending permission prompts may return errors instead of responses.

### Filesystem and terminal requests (schema-completeness only)

```json
{"jsonrpc":"2.0","id":43,"method":"fs/read_text_file","params":{"sessionId":"sess_abc123","path":"/project/src/main.rs","line":10,"limit":50}}
```

```json
{"jsonrpc":"2.0","id":44,"method":"terminal/create","params":{"sessionId":"sess_abc123","command":"cargo","args":["build"],"cwd":"/project","outputByteLimit":1048576}}
```

These schemas are stable for general ACP clients but the current `codex-acp` adapter does not initiate them. Implement them as general ACP clients; do not expect traffic from this provider.

## Permissions, Filesystem, and Terminal

### Permission policy

- The client is the authority for every approval request. There is no implicit default policy — every `session/request_permission` must receive a `Selected` or `Cancelled` response.
- Approval options go beyond the generic `allow_once` / `allow_always` / `reject_once` triple. The adapter can include Codex-specific decisions (execpolicy amendment for command patterns, network policy amendment for host allow/block, MCP tool-call approval with `persist: session | always`) via `_meta.codex` on the option. Clients may either surface them as raw option buttons or render them as plain allow/reject.
- On `session/cancel` or `$/cancel_request`, the adapter cancels any in-flight permission requests via the request signal.

### Filesystem policy

The adapter does not delegate file I/O to the client. Reads and writes happen inside the Codex App Server via Codex's own tools, governed by the active sandbox mode (`read-only`, `workspace-write`, or `danger-full-access`). Clients enforce their own project-root boundary; the adapter enforces the configured sandbox.

When `fs/read_text_file` / `fs/write_text_file` are implemented as a matter of general ACP client support: paths must be absolute, line numbers are 1-based, and the client validates paths before reading or writing.

### Terminal policy

The adapter does not delegate command execution to the client. Commands run inside the Codex App Server, surfaced as `tool_call` events with `kind: "execute"` and streamed progress via `tool_call_update`. Output bytes flow through `_meta.terminal_output_delta` (the default) or `_meta.terminal_output` (when negotiated via `clientCapabilities._meta.terminal_output: true`).

When `terminal/*` is implemented as a matter of general ACP client support: the client receives the full command, arguments, environment, and working directory, decides whether to allow it, and is responsible for process lifecycle, output buffers, byte-limit truncation (truncating from the beginning when `outputByteLimit` is exceeded), and the always-call `terminal/release` discipline.

## Streaming and UI Integration

Streaming flows through `session/update` notifications. The adapter maps Codex App Server events to ACP session-update variants.

| Update | Purpose |
|--------|---------|
| `AgentMessageChunk` | Incremental assistant text. Carries `messageId` (added in v1.1.0) for chunk grouping. |
| `AgentThoughtChunk` | Reasoning / extended thinking. |
| `UserMessageChunk` | User message replay during `session/load`. |
| `ToolCall` | A new tool call has started. Codex kinds: `execute`, `edit`, `search`, `fetch`, `other`, plus image/imageGen/collab/dynamic. |
| `ToolCallUpdate` | Tool progress, status change, or final result. |
| `Plan` | Multi-step plan entry. |
| `AvailableCommandsUpdate` | Slash commands available in the session (e.g. `/status`, `/mcp`, `/skills`, `/review`, `/review-branch`, `/review-commit`, `/compact`, `/goal`, `/logout`). |
| `CurrentModeUpdate` | Session mode change. |
| `ConfigOptionUpdate` | Session config option change. |

Codex also streams shell command execution, file change, permission request, MCP tool call, terminal output delta, reasoning, image generation, image view, web search, token usage, and review events. Notifications are fire-and-forget — group by `messageId` to disambiguate parallel streams.

A Rust desktop app typically uses `tokio::sync::mpsc` to forward updates from the ACP runtime thread to the UI framework (Tauri, iced, etc.).

## Authentication and Setup

The adapter inherits Codex CLI's authentication posture and additionally advertises an `authMethods` array at `initialize`. The advertised methods:

1. **`api-key`** (always advertised) — supply the key via `_meta["api-key"].apiKey` in the `AuthenticateRequest`. The adapter picks up `CODEX_API_KEY` (preferred) or `OPENAI_API_KEY` from the environment if no explicit key is supplied.
2. **`chat-gpt`** (advertised unless `NO_BROWSER=1`) — standard ChatGPT OAuth login. Corrupted `~/.codex/auth.json` triggers automatic logout and the next request fails with `RequestError.authRequired()`.
3. **`gateway`** (advertised only when `clientCapabilities.auth._meta.gateway === true`) — custom OpenAI-compatible gateway. The `AuthenticateRequest` carries `_meta["gateway"]` with `baseUrl`, `headers`, and optional `providerName`.

For headless operation: set `CODEX_API_KEY` (or `OPENAI_API_KEY`); set `NO_BROWSER=1` to remove the ChatGPT method from advertised options; or seed `~/.codex/auth.json` from a machine that already completed ChatGPT login. The `codex login --device-auth` headless flow is also available through the underlying Codex CLI.

## Compatibility, Quirks, and Workarounds

1. **No native ACP mode** — direct probes on Codex CLI 0.142.5 confirm `codex --acp` / `codex acp` are rejected. ACP clients must use the adapter.
2. **Adapter namespace moved** — `@zed-industries/codex-acp` is deprecated. New installs should use `@agentclientprotocol/codex-acp`.
3. **No `fs/*` or `terminal/*` reverse requests** — the adapter delegates file I/O and command execution to the Codex App Server. Implement those handlers as general ACP clients; do not expect them to fire with this provider.
4. **Auth method names changed** — current adapter advertises `api-key` (was `codex-api-key` / `openai-api-key`), `chat-gpt` (was `chatgpt`), and a new capability-gated `gateway` method.
5. **Gateway auth is capability-gated** — clients must opt in via `clientCapabilities.auth._meta.gateway: true` to receive the `gateway` method in `authMethods`.
6. **JetBrains 2026.1 has session_config disabled** — the adapter detects this client class by `clientInfo.name` and omits `SessionConfigOption` payloads until upstream LLM-28118 is resolved.
7. **Fast mode is conditional** — appears only when `modelSupportsFast(currentModel)` returns true. Clients may see the option appear or disappear as the active model changes.
8. **MCP capabilities are narrow** — only `mcpCapabilities.http: true`; `acp: false` and `sse: false`. ACP-protocol MCP servers cannot pass through this adapter.
9. **Initialization timeout** — the adapter can take more than 30 seconds to initialize on first launch (especially when ChatGPT OAuth prompts appear). Use a 60-second timeout for `initialize`.
10. **Path and indexing mistakes** — ACP requires absolute paths and 1-based line numbers. Relative paths and 0-based indexing are common integration bugs (relevant for general ACP support; this adapter does not invoke file reverse requests).
11. **Stdout pollution** — the adapter writes structured JSON-RPC to stdout and adapter/Codex logs to stderr; never write to the adapter's stderr.
12. **Bundled Codex version** — `@openai/codex ^0.142.5` is bundled in the npm package; use `CODEX_PATH` to override with a different binary. The bundled version is intentionally pinned.
13. **Corrupted auth.json** — automatically logs out and surfaces `RequestError.authRequired()` on the next request unless `DEFAULT_AUTH_REQUEST` is set.
14. **Adapter stdin close → Codex SIGKILL** — if the client closes stdin, the adapter closes Codex's stdin and SIGKILLs the process after a 2-second grace period.

## Recent Changes

- **2026-07-02** (`@agentclientprotocol/codex-acp` **v1.1.0**) — ACP SDK v1.1.0; `messageId` added to text session chunks; boolean Fast mode config option support; completed image generation items surfaced as `tool_call`; goal changes emitted as session metadata; `vscode-jsonrpc` upgraded to v9; bundled `@openai/codex` bumped to 0.142.5.
- **2026-06-29** (`v1.0.2`) — bundled `@openai/codex` bumped to 0.142.3 and 0.142.4; `/goal` slash command support; fixed skill listing to use session cwd; removed Fast mode config for models that don't support it.
- **2026-06-26** (`v1.0.1`) — ACP SDK bumped to 1.0.0; ACP request cancellation (`$/cancel_request`) handling; collab agent tool call events mapped to `tool_call`; API-key auth reads from `CODEX_API_KEY` / `OPENAI_API_KEY` env vars; auto-skip ChatGPT login when already authenticated.
- **2026-06-23** (`v1.0.0`) — first stable v1 release. Added `session/delete`, more informative permission approvals, embedded resource blob handling, `additionalDirectories` support, reasoning events streamed as agent thoughts, automatic logout on corrupted auth.json, and `session_config` negotiation. Bundled Codex bumped to 0.139.0–0.141.0 over the cycle.
- **2026-06-08** (`@zed-industries/codex-acp` **v0.16.0**) — final Zed-maintained release before migration to the `agentclientprotocol` org. npm description later updated to mark the package deprecated.
- **Earlier 2026 (v0.0.41–v0.0.46)** — auth logout capability advertised; gateway auth gated by client capabilities; Fast mode introduced; `/status`, `/mcp`, `/skills`, `/review*`, `/compact` slash commands; `session/close`; session config options (mode, model, reasoning effort); migrate to ACP SDK 0.28 API; thread-history routing fixed in v0.0.44.

## Rust Client Example

This example uses `agent-client-protocol 1.0.1` with the current `@agentclientprotocol/codex-acp` adapter:

```toml
[dependencies]
agent-client-protocol = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

```rust
use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ClientCapabilities, ContentBlock, Implementation,
        InitializeRequest, NewSessionRequest, PromptRequest, SessionNotification,
        TextContent,
    },
};
use agent_client_protocol::{AcpAgent, Client};
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use the explicit FromStr form to bypass any preset that may point at the
    // deprecated `@zed-industries/codex-acp` package.
    let agent = AcpAgent::from_str("npx -y @agentclientprotocol/codex-acp")?;

    Client
        .builder()
        .name("claudine-codex-client")
        .on_receive_notification(
            |notification: SessionNotification, _cx| async move {
                match notification.update {
                    SessionNotification::AgentMessageChunk(chunk) => {
                        if let ContentBlock::Text(t) = chunk.content {
                            print!("{}", t.text);
                        }
                    }
                    SessionNotification::ToolCall(tc) => {
                        eprintln!("\n[tool started: {}]", tc.title);
                    }
                    _ => {}
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(agent, |connection| async move {
            // The Codex adapter ignores fs/terminal capabilities, but we advertise
            // them anyway for forward-compatibility with other agents sharing this
            // client.
            let caps = ClientCapabilities::new()
                .terminal(true);

            let init = InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(caps)
                .client_info(Implementation {
                    name: "claudine".into(),
                    title: Some("Claudine".into()),
                    version: "0.1.0".into(),
                });

            let init_response = connection.send_request(init).block_task().await?;
            eprintln!("Agent: {:?}", init_response.agent_info);
            eprintln!("Auth methods: {:?}", init_response.auth_methods);

            let session = connection
                .send_request(NewSessionRequest::new(std::env::current_dir()?, vec![]))
                .block_task()
                .await?;

            let result = connection
                .send_request(PromptRequest::new(
                    session.session_id,
                    vec![ContentBlock::Text(TextContent::new(
                        "What files are in this directory?".into(),
                    ))],
                ))
                .block_task()
                .await?;

            eprintln!("\nStop reason: {:?}", result.stop_reason);
            Ok(())
        })
        .await?;

    Ok(())
}
```

## Rust Reverse Request Handling

The current Codex adapter only issues `session/request_permission`. The example below auto-approves a safe allow_once option but lets the user pick from the supplied options in a real UI:

```rust
use agent_client_protocol::schema::v1::{
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome,
};

async fn handle_permission(
    request: RequestPermissionRequest,
) -> anyhow::Result<RequestPermissionResponse> {
    // Pick the first `allow_once` option as a safe default; the UI can replace this.
    let option_id = request
        .options
        .iter()
        .find(|o| matches!(o.kind, agent_client_protocol::PermissionOptionKind::AllowOnce))
        .map(|o| o.option_id.clone())
        .or_else(|| request.options.first().map(|o| o.option_id.clone()))
        .unwrap_or_default();

    Ok(RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
        SelectedPermissionOutcome::new(option_id),
    )))
}
```

Register the handler on the builder before `connect_with`:

```rust
Client
    .builder()
    .on_receive_request(
        |request: RequestPermissionRequest, responder, _cx| async move {
            responder.respond(handle_permission(request).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
```

## Rust Host Command Handling

The current Codex adapter does not issue `terminal/*` reverse requests. For general ACP client support (other agents, or future Codex adapter versions), wire up terminal handlers as follows:

```rust
use agent_client_protocol::schema::v1::{
    CreateTerminalRequest, CreateTerminalResponse, KillTerminalRequest, KillTerminalResponse,
    ReleaseTerminalRequest, ReleaseTerminalResponse, TerminalId, TerminalOutputRequest,
    TerminalOutputResponse, WaitForTerminalExitRequest, WaitForTerminalExitResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

struct TerminalHandle {
    child: Child,
    stdout_buf: Vec<u8>,
    stderr_buf: Vec<u8>,
    exited: bool,
    exit_code: Option<i32>,
    output_limit: usize,
}

#[derive(Clone)]
struct TerminalManager {
    terminals: Arc<Mutex<HashMap<TerminalId, TerminalHandle>>>,
    next_id: Arc<Mutex<u64>>,
}

impl TerminalManager {
    fn new() -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    async fn next_id(&self) -> TerminalId {
        let mut id = self.next_id.lock().await;
        *id += 1;
        format!("term_{}", *id).into()
    }
}

async fn handle_create_terminal(
    request: CreateTerminalRequest,
    manager: &TerminalManager,
    default_root: PathBuf,
) -> anyhow::Result<CreateTerminalResponse> {
    let cwd = request.cwd.unwrap_or(default_root);
    let limit = request.output_byte_limit.unwrap_or(1_048_576) as usize;

    let child = tokio::process::Command::new(&request.command)
        .args(request.args)
        .envs(request.env.into_iter().map(|e| (e.name, e.value)))
        .current_dir(&cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let id = manager.next_id().await;
    manager.terminals.lock().await.insert(
        id.clone(),
        TerminalHandle {
            child,
            stdout_buf: Vec::new(),
            stderr_buf: Vec::new(),
            exited: false,
            exit_code: None,
            output_limit: limit,
        },
    );

    Ok(CreateTerminalResponse { terminal_id: id })
}
```

The remaining handlers (`terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release`) follow the same pattern: look up the `TerminalId`, operate on the `Child`, and return the corresponding response. Always implement `terminal/release` and kill the process if it is still running — handle leaks are a frequent production foot-gun.

## Rust Desktop Streaming Bridge

To stream ACP events into a desktop UI, run the ACP client on a dedicated thread and forward `SessionNotification` values through an `mpsc` channel. Use the current adapter namespace:

```rust
use tokio::sync::mpsc;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextChunk(String),
    ThoughtChunk(String),
    ToolCallStarted { id: String, title: String },
    ToolCallFinished { id: String, status: String },
    PermissionRequest { request_id: String, title: String, options: Vec<(String, String)> },
    ModeUpdate(String),
    CommandsUpdate(Vec<String>),
    TurnComplete { stop_reason: String },
    Error(String),
}

pub fn spawn_agent(
    project_dir: PathBuf,
) -> anyhow::Result<(mpsc::UnboundedReceiver<AgentEvent>, mpsc::UnboundedSender<String>)> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (prompt_tx, mut prompt_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async move {
            let agent = AcpAgent::from_str("npx -y @agentclientprotocol/codex-acp")?;

            Client
                .builder()
                .on_receive_notification(
                    {
                        let tx = event_tx.clone();
                        move |notification: SessionNotification, _cx| {
                            let tx = tx.clone();
                            async move {
                                let event = match notification.update {
                                    SessionNotification::AgentMessageChunk(chunk) => match chunk.content {
                                        ContentBlock::Text(t) => Some(AgentEvent::TextChunk(t.text)),
                                        _ => None,
                                    },
                                    SessionNotification::ToolCall(tc) => Some(AgentEvent::ToolCallStarted {
                                        id: tc.tool_call_id.to_string(),
                                        title: tc.title,
                                    }),
                                    _ => None,
                                };
                                if let Some(event) = event {
                                    let _ = tx.send(event);
                                }
                                Ok(())
                            }
                        }
                    },
                    agent_client_protocol::on_receive_notification!(),
                )
                .connect_with(agent, |connection| async move {
                    let _ = connection
                        .send_request(InitializeRequest::new(ProtocolVersion::V1))
                        .block_task()
                        .await?;

                    let session = connection
                        .send_request(NewSessionRequest::new(project_dir, vec![]))
                        .block_task()
                        .await?;

                    while let Some(prompt) = prompt_rx.recv().await {
                        match connection
                            .send_request(PromptRequest::new(
                                session.session_id.clone(),
                                vec![ContentBlock::Text(TextContent::new(prompt))],
                            ))
                            .block_task()
                            .await
                        {
                            Ok(response) => {
                                let _ = event_tx.send(AgentEvent::TurnComplete {
                                    stop_reason: format!("{:?}", response.stop_reason),
                                });
                            }
                            Err(e) => {
                                let _ = event_tx.send(AgentEvent::Error(e.to_string()));
                            }
                        }
                    }
                    Ok(())
                })
                .await
                .ok();
        });
    });

    Ok((event_rx, prompt_tx))
}
```

### Tauri usage

```rust
#[tauri::command]
async fn send_prompt(state: tauri::State<'_, AppState>, prompt: String) -> Result<(), String> {
    state.prompt_tx.send(prompt).map_err(|e| e.to_string())
}

fn listen(event_rx: mpsc::UnboundedReceiver<AgentEvent>, handle: tauri::AppHandle) {
    tokio::spawn(async move {
        let mut rx = event_rx;
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::TextChunk(text) => handle.emit("agent:text", text).ok(),
                AgentEvent::TurnComplete { stop_reason } => handle.emit("agent:done", stop_reason).ok(),
                _ => None,
            };
        }
    });
}
```

### iced usage

```rust
fn agent_subscription(
    event_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<AgentEvent>>>>,
) -> iced::Subscription<AgentEvent> {
    iced::subscription::channel(
        std::any::TypeId::of::<AgentEvent>(),
        100,
        |mut output| async move {
            let mut rx = event_rx
                .lock()
                .await
                .take()
                .expect("subscription already consumed");
            while let Some(event) = rx.recv().await {
                output.send(event).await.ok();
            }
            std::future::pending().await
        },
    )
}
```

## Claudine Integration Notes

Claudine currently wraps Codex CLI through lifecycle hooks and event normalization, not through ACP. Adding ACP-based Codex CLI support would require:

1. **Adapter launch detection** — detect one of two npm package shapes (`@agentclientprotocol/codex-acp` preferred; `@zed-industries/codex-acp` legacy, deprecated). Allow a user-configured adapter binary path for standalone bundles. Avoid `AcpAgent` presets that point at the deprecated npm name.
2. **Capability negotiation** — advertise HTTP-only MCP support (`mcpCapabilities.http: true`, `acp: false`, `sse: false`). Surface session config options (mode, model, reasoning effort, fast mode) so Claudine can present them in its UI. Opt in to the `gateway` auth capability only when the user has configured a custom OpenAI-compatible gateway.
3. **Reverse-request routing** — only `session/request_permission` reliably fires. Route it through Claudine's existing `permissions` / `protect` machinery. Render Codex-specific approval options (execpolicy amendments, network policy amendments, MCP tool-call persist scopes) either as raw buttons or as plain allow/reject; the choice affects how much of the Codex policy surface the user controls.
4. **Mode mapping** — translate the three Codex modes (`read-only`, `agent`, `agent-full-access`) into Claudine's per-project approval tiers. Honour `INITIAL_AGENT_MODE` for first-session launches.
5. **Streaming bridge** — forward `session/update` notifications into Claudine's event pipeline so TTS, sound effects, logging, and messenger actions can trigger. Group text chunks by `messageId` (added in v1.1.0) and dispatch `current_mode_update`, `available_commands_update`, and `config_option_update` to the lifecycle stack.
6. **Headless auth** — require `CODEX_API_KEY` / `OPENAI_API_KEY` or a verified pre-authenticated `~/.codex/auth.json` before allowing non-interactive ACP launches. Honour `NO_BROWSER=1` to drop the ChatGPT method.
7. **Adapter stdin close** — ensure the Codex lifecycle is terminated cleanly; the adapter SIGKILLs Codex 2 seconds after stdin close, so ensure all spawned processes are reaped.

Because Codex CLI has no native ACP mode and the recommended adapter is a TypeScript/npm bridge, Claudine should treat it as an **adapter-launched provider** with a higher integration cost than providers that ship ACP natively.

## Changelog

- **2026-07-03**: Refreshed for Codex CLI 0.142.5 and `@agentclientprotocol/codex-acp` v1.1.0. Verified the `codex` binary has no native ACP entry point; recorded exact error strings for `codex --acp` and `codex acp`. Corrected the auth method catalog (`api-key` / `chat-gpt` / `gateway` instead of the prior `chatgpt` / `codex-api-key` / `openai-api-key`). Documented the new `gateway` capability-gated auth method, the four `SessionConfigOption` kinds (mode, model, reasoning effort, fast mode), the JetBrains 2026.1 session_config disablement, the HTTP-only `mcpCapabilities` flags, the absence of `fs/*` / `terminal/*` reverse requests, the three extension methods (`authentication/status`, `authentication/logout`, legacy `session/set_model`), and the full env-var catalog (`INITIAL_AGENT_MODE`, `APP_SERVER_LOGS`, `MODEL_PROVIDER`, `DEFAULT_AUTH_REQUEST`). Confirmed `@zed-industries/codex-acp` v0.16.0 is the final Zed release (2026-06-08) and is marked deprecated on npm.

## Sources

- [Codex CLI Documentation](https://developers.openai.com/codex/cli)
- [`openai/codex` Repository](https://github.com/openai/codex)
- [Codex App Server protocol](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
- [Codex Authentication Docs](https://developers.openai.com/codex/auth)
- [Codex Slash Commands](https://developers.openai.com/codex/cli/slash-commands)
- [Agent Client Protocol Specification](https://agentclientprotocol.com/)
- [ACP Agents Overview](https://agentclientprotocol.com/overview/agents)
- [ACP Protocol v1 Overview](https://agentclientprotocol.com/protocol/v1/overview)
- [ACP Tool Calls](https://agentclientprotocol.com/protocol/v1/tool-calls)
- [ACP File System](https://agentclientprotocol.com/protocol/v1/file-system)
- [ACP Terminals](https://agentclientprotocol.com/protocol/v1/terminals)
- [ACP Extension Methods](https://agentclientprotocol.com/protocol/v1/extensibility)
- [ACP Rust SDK (docs.rs)](https://docs.rs/agent-client-protocol/1.0.1/agent_client_protocol/)
- [`agentclientprotocol/rust-sdk` Repository](https://github.com/agentclientprotocol/rust-sdk)
- [Rust SDK yolo_one_shot_client Example](https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/examples/yolo_one_shot_client.rs)
- [`@agentclientprotocol/codex-acp` Adapter](https://github.com/agentclientprotocol/codex-acp) — current
- [`@agentclientprotocol/codex-acp` on npm](https://www.npmjs.com/package/@agentclientprotocol/codex-acp)
- [Adapter README (raw)](https://raw.githubusercontent.com/agentclientprotocol/codex-acp/main/README.md)
- [Adapter `readme-dev.md`](https://raw.githubusercontent.com/agentclientprotocol/codex-acp/main/readme-dev.md)
- [Adapter `package.json`](https://raw.githubusercontent.com/agentclientprotocol/codex-acp/main/package.json)
- [Adapter Source — `index.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/index.ts)
- [Adapter Source — `CodexAcpServer.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/CodexAcpServer.ts)
- [Adapter Source — `CodexAuthMethod.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/CodexAuthMethod.ts)
- [Adapter Source — `CodexApprovalHandler.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/CodexApprovalHandler.ts)
- [Adapter Source — `CodexElicitationHandler.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/CodexElicitationHandler.ts)
- [Adapter Source — `CodexCommands.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/CodexCommands.ts)
- [Adapter Source — `AgentMode.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/AgentMode.ts)
- [Adapter Source — `FastModeConfig.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/FastModeConfig.ts)
- [Adapter Source — `TerminalOutputMode.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/TerminalOutputMode.ts)
- [Adapter Source — `AcpExtensions.ts`](https://github.com/agentclientprotocol/codex-acp/blob/main/src/AcpExtensions.ts)
- [Adapter Releases](https://github.com/agentclientprotocol/codex-acp/releases)
- [Legacy `@zed-industries/codex-acp` (deprecated)](https://github.com/zed-industries/codex-acp)
- [`@zed-industries/codex-acp` v0.16.0 Release](https://github.com/zed-industries/codex-acp/releases/tag/v0.16.0)
- [ACP Schema Reference](https://agentclientprotocol.com/protocol/schema)