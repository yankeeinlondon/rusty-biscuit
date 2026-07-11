---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://goose-docs.ai/
acp_docs: https://agentclientprotocol.com/
repo: https://github.com/aaif-goose/goose
support: native
launch_modes:
  - command: goose acp
    args:
      - --with-builtin NAME,...
    transport: stdio
    adapter: none
    notes: "Native stdio ACP agent. Boots crates/goose/src/acp/server.rs::serve() in a stdio JSON-RPC loop. The --with-builtin flag is a comma-separated list of builtin extension names; if omitted, defaults to `developer` (the in-process developer extension that supplies read/write/edit/shell via either in-process tools or ACP reverse requests)."
  - command: goose serve
    args:
      - --host 127.0.0.1
      - --port 3284
      - --tls
      - "--tls-cert-path PATH"
      - "--tls-key-path PATH"
      - "--platform cli|desktop"
      - --with-builtin NAME,...
      - --dangerously-unauthenticated
      - "--allowed-origin ORIGIN"
    transport: http
    adapter: none
    notes: "Native HTTP/WebSocket ACP transport via the `agent-client-protocol-http` crate (axum-based). Same Goosen AcpServer::new wiring as `goose acp`, mounted under `/acp`. Authentication is a constant-time-compare of the `X-Secret-Key` header (or `?token=` query for WebSockets) against `GOOSE_SERVER__SECRET_KEY` — the server refuses to start without either that env var or `--dangerously-unauthenticated`. `--platform` selects GoosePlatform (cli/desktop) for builtin extension behavior."
  - command: goosed
    args: []
    transport: http
    adapter: none
    notes: "Standalone HTTP binary from the `goose-server` crate, mounted under /acp. Marked as a transitional bridge in the source (`TODO(acp-migration)` comments in crates/goose-server/src/commands/agent.rs); long-term direction is to retire goosed in favor of `goose serve`. UI/desktop apps currently launch this binary directly."
  - command: goose tui
    args:
      - "..."
    transport: stdio
    adapter: "@aaif/goose (TypeScript TUI shipped with goose; auto-launches `goose acp` or connects to an HTTP server via `--server URL`)"
    notes: "Embedded reference TUI client. Auto-launches `goose acp` if no `--server` is provided; otherwise connects to `goose serve` over HTTP/WebSocket. Implements the full permission UI (y/a/n/N keyboard model described in the docs)."
protocol_versions:
  - "v1 (schema 1.1.0)"
  - "ACP HTTP transport (unstable_cancel_request)"
  - "Goose custom `_goose/unstable/...` namespace (stable within the goose ecosystem)"
capabilities:
  - capability: initialize
    support: supported
    notes: "Standard `initialize` handshake returning AgentCapabilities (loadSession, sessionCapabilities.{list,close}, promptCapabilities.{image,embedded_context}, mcpCapabilities.{http}, meta.goose.localInference when built with `--features local-inference`) and one advertised auth method: `id=\"goose-provider\"`, name `\"Configure Provider\"`. AgentInfo includes `name=\"goose\"` + version. `initialize` is also where goose reads `meta.goose.useLoginShellPath` and goose-specific client capability hints (mcpHostCapabilities, customNotifications, recipeParameterRequests)."
  - capability: authenticate
    support: supported
    notes: "`authenticate` is implemented but the immediate response is empty; effective authentication happens out-of-band (provider credentials come from env vars / `goose configure`, not from ACP)."
  - capability: session_new
    support: supported
    notes: "Required `cwd` (absolute, must exist). Optional `mcpServers: Vec<McpServer>` (Stdio mapped to ExtensionConfig::Stdio; Http mapped to ExtensionConfig::StreamableHttp; Sse explicitly rejected with `\"SSE is unsupported, migrate to streamable_http\"`). Recipe / goose-extensions meta also supported. The developer builtin extension is always enabled; additional MCP servers take precedence over config-file defaults unless `no-profile` is set."
  - capability: session_load
    support: supported
    notes: "`session/load` replays the persisted session history as `session/update` notifications on a separate connection. ACP-persisted sessions remain interoperable with goose's own CLI/TUI session history (both write the same SQLite-backed `sessions` table)."
  - capability: session_prompt
    support: supported
    notes: "Streaming prompt. Supports `Text`, `Image`, `EmbeddedResource`, and `ResourceLink` content blocks (audio is dropped — see `convert_acp_prompt_to_message` in acp/server.rs)."
  - capability: session_cancel
    support: supported
    notes: "Per-schema `session/cancel` notification is honored; cancellation race-condition fixes shipped in #9804. The standard `$/cancel_request` protocol-level cancellation is also implemented via the `unstable_cancel_request` feature in agent-client-protocol-http 1.0."
  - capability: session_modes
    support: supported
    notes: "Modes derive from the `GooseMode` enum: `auto` (bypass permissions), `smart_approve` (ask only on sensitive), `approve` (ask every time), `chat` (plan only — no tool execution). Exposed via `session/set_mode` and the `CurrentModeUpdate` notification. The `set_mode` reply (CurrentModeUpdate) is sent after the agent loop rebinds the mode."
  - capability: streaming
    support: supported
    notes: "Every standard `SessionUpdate` variant except `Plan` is emitted: AgentMessageChunk, UserMessageChunk, AgentThoughtChunk, ToolCall, ToolCallUpdate, CurrentModeUpdate, ConfigOptionUpdate, SessionInfoUpdate, AvailableCommandsUpdate, UsageUpdate. Plan is not currently emitted; live plan streaming during multi-step work is funneled through the `tool_chain_summary` meta on consecutive ToolCallUpdates."
  - capability: permissions
    support: supported
    notes: "Implemented as a first-class reverse request (`session/request_permission`), driven by every `ToolConfirmation` reaching the agent loop. The four option kinds are `allow_once / allow_always / reject_once / reject_always` with matching `option_id` strings; selecting a `reject_always` or `allow_always` flows back into goose's permission manager and persists across sessions."
  - capability: fs_read
    support: supported
    notes: "Conditional reverse request `fs/read_text_file` issued by the in-process developer extension's `read`/`edit` tools when client advertises `client_capabilities.fs.read_text_file: true`. Falls back to in-process file I/O otherwise."
  - capability: fs_write
    support: supported
    notes: "Conditional reverse request `fs/write_text_file` issued by `write`/`edit` developer tools when client advertises `client_capabilities.fs.write_text_file: true`."
  - capability: terminal
    support: supported
    notes: "Full lifecycle: `terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release`. Issued by the `shell` developer tool when client advertises `client_capabilities.terminal: true`. `output_byte_limit` defaults to a constant (`OUTPUT_LIMIT_BYTES`). `terminal/release` is always fired in cleanup (best-effort)."
  - capability: mcp
    support: partial
    notes: "Server-side MCP config via `session/new`'s `mcpServers` accepts Stdio + StreamableHttp; Sse is rejected with a hard error. Client-side MCP host capabilities are advertised via `client_capabilities_meta.goose.mcpHostCapabilities.extensions` so goose can register its own MCP servers with the host. Elicitation is route through tool streams (#9943 in v1.41)."
  - capability: media
    support: supported
    notes: "Images: `ImageContent` in `session/prompt`, plus re-rendered image tool results on `ToolCallUpdate.content`. Audio prompts are dropped (`ContentBlock::Audio` branch is intentionally empty)."
  - capability: plans
    support: unsupported
    notes: "There is no `SessionUpdate::Plan` construction in the current source. Live multi-tool chains produce a single tool-chain summary on the closing ToolCallUpdate (`meta.goose.toolChainSummary = { summary, count }`), but a dedicated `Plan` entry list is not surfaced over ACP."
  - capability: extensions
    support: supported
    notes: "ACP `_meta` is in heavy use across nearly every message update. Goose-specific namespace `_goose/` (for stable extensions) and `_goose/unstable/` (for in-flight experiments) is used for both reverse requests (`_goose/unstable/recipe_params`, `_goose/unstable/agent_mentions`, `_goose/unstable/session/steer`) and notifications (`_goose/unstable/session/update`). The catalogue is owned by the external `goose-sdk-types` crate."
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: "Required path: every ToolConfirmation in the stream surfaces here. Option IDs are the literal strings `allow_once`, `allow_always`, `reject_once`, `reject_always`. Selection is a `Selected{ id }` outcome; cancellation is `Cancelled`."
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: "Conditional. Issued only when `client_capabilities.fs.read_text_file: true`. Falls back to direct fs::read_to_string otherwise."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: "Conditional. Issued only when `client_capabilities.fs.write_text_file: true`."
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: "Conditional. Issued only when `client_capabilities.terminal: true`. Always followed by `terminal/wait_for_exit` (with optional timeout/`terminal/kill`) and finally `terminal/release`."
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: "Conditional — same caveat as terminal/create."
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: "Conditional — same caveat. `tokio::time::timeout` is used against the developer tool's `timeout_secs` argument."
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: "Conditional — fired only if `wait_for_exit` times out."
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: false
    notes: "Always fired (best-effort cleanup) at the end of every shell invocation."
  - method: elicitation/create
    purpose: tool
    client_must_handle: false
    notes: "Sent only when the client advertises `client_capabilities.elicitation.form`. Falls back to a logged cancel otherwise. Decline and cancel paths propagated end-to-end (#9437 in v1.41)."
  - method: _goose/unstable/recipe_params
    purpose: other
    client_must_handle: false
    notes: "Custom goose reverse request asking the client to prompt for a recipe's parameters. Schema lives in the external `goose-sdk-types` crate (constants exported through `pub use goose_sdk_types::custom_requests`). Visible via `crates/goose/src/acp/server/agent_requests.rs`."
  - method: _goose/unstable/agent_mentions
    purpose: other
    client_must_handle: false
    notes: "Custom list reverse request: clients see a Subrecipes + filesystem-summoned agents. Handler `on_list_agent_mentions` in `crates/goose/src/acp/server/agent_mentions.rs`."
  - method: _goose/unstable/session/steer
    purpose: other
    client_must_handle: false
    notes: "Custom steer reverse request for queued-steering semantics (`send_queued_steer_update`). Backed by `agent.on_steer_session` in `acp/server.rs`."
permission_model:
  mechanism: "session/request_permission reverse request; option kinds allow_once / allow_always / reject_once / reject_always"
  timeout: client-defined (goose has no implicit timeout; respond whenever the user picks)
  default_policy: "selected by GooseMode: `auto` = allow-once synthesized (no prompt); `smart_approve` / `approve` = always-prompt; `chat` = reject tool calls unconditionally"
  approval_values:
    - allow_once
    - allow_always
    - reject_once
    - reject_always
  notes: "The four option `id`s exactly mirror the schema strum-serialize snake_case of `PermissionOptionKind` (verified in `acp/server.rs:1885-1960`). Selecting `allow_always` / `reject_always` causes the choice to be persisted into the per-working-dir `permission.yaml` and re-applied on subsequent prompts in the same project. `Cancelled` and unknown `optionId` map to `Permission::Cancel` (the request is treated as a denial)."
filesystem_model:
  read_methods:
    - fs/read_text_file
  write_methods:
    - fs/write_text_file
  path_base: absolute
  sandboxing: client-side (goose does not enforce a project-root boundary itself; the controller decides which paths to accept)
  notes: "Filesystem reverse requests are gated by `apply_acp_extension_overrides` in `acp/server.rs` — it swaps the in-process `developer` extension for an RMCP-shaped `AcpTools` (in `acp/fs.rs`) only when the client advertises at least one of `client_capabilities.fs.read_text_file`, `fs.write_text_file`, or `terminal: true`. Tool responses carry `_goose/acp-aware: true` meta so `handle_tool_response` knows to skip its in-process content/location reconstruction step and trust what streamed in over `ToolCallUpdate`. Absolute paths only; 1-based line numbers."
terminal_model:
  supported: true
  methods:
    - terminal/create
    - terminal/output
    - terminal/wait_for_exit
    - terminal/kill
    - terminal/release
  shell: "passed through verbatim — goose does not interpret or wrap the command; `cwd` defaults to the project's working directory"
  cwd: "absolute, supplied in `terminal/create` (defaults to ACP session working dir)"
  streaming: "polled via `terminal/output` until `terminal/wait_for_exit` resolves; output is truncated from the beginning once `output_byte_limit` is exceeded (constant `OUTPUT_LIMIT_BYTES` in `acp/fs.rs`)"
  cancellation: "terminal/kill on timeout, terminal/release always"
  notes: "Lifecycle is collapsed into a single Rust method `acp_shell` in `crates/goose/src/acp/fs.rs`: create → optional timeout/wait → kill-on-timeout → release-on-cleanup. Clients that don't advertise `terminal: true` get the in-process `developer` extension's shell tool (which spawns its own tokio process via `tokio::process::Command` and bypasses ACP entirely)."
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
    - "no SessionUpdate::Plan; live multi-step chains are summarized on the closing tool_call_update under meta.goose.toolChainSummary"
  error_events:
    - "JSON-RPC errors on the request channel; no dedicated error update variant"
    - "ACP protocol-level `$/cancel_request` (unstable_cancel_request feature flag on the HTTP transport)"
  notes: "Goose custom notifications under `_goose/unstable/session/update` carry additional state — `StatusMessage` (system notifications), `UsageUpdate`, plus `SessionInfoUpdate` meta keys `goose.messageCount`, `goose.userSetName`, `goose.activeRunId`, and `goose.queuedSteer { messageId, runId }`. The custom notifications are sent only when the client opts in via `client_capabilities.meta.goose.customNotifications: true`. Pre-v1.41 clients should not rely on the custom notifications — they were promoted to a documented public surface (handled in `_goose/`) gradually."
auth_setup:
  required: false
  mechanisms:
    - "Provider credentials (OpenAI/Anthropic API keys, Bedrock/Vertex env, etc.) configured via `goose configure` or environment variables — not via ACP `authenticate`"
    - "`authenticate` request is recognized but resolves empty; effective auth is out of band"
    - "`GOOSE_SERVER__SECRET_KEY` is required for `goose serve` to accept connections (alternative: `--dangerously-unauthenticated` for local development)"
    - "`GOOSE_PROVIDER` + `GOOSE_MODEL` env vars select provider/model per session"
    - "`GOOSE_OAUTH_CALLBACK_PORT` (added v1.41) controls the OAuth redirect port"
  headless_notes: "For headless ACP launches, set `GOOSE_PROVIDER` and either `GOOSE_MODEL` + the provider-specific credential env var, or pre-seed the provider via `goose configure` (the resulting config and keychain entry are reused). For `goose serve`, also set `GOOSE_SERVER__SECRET_KEY` and provide it via `X-Secret-Key` header (or `?token=` for WebSocket clients). OAuth-based providers (Claude/OpenAI) need proactive token refresh (#8386) so they don't re-prompt every session."
  notes: "`authenticate` is treated as a no-op by goose itself; the authorization flow runs through the configured provider. Recent bugfixes (#9694) clear stale rejected OAuth credentials after refresh."
env_vars:
  - name: GOOSE_PROVIDER
    effect: "Selects the provider for the current session (e.g. `openai`, `anthropic`, `claude-acp`, `codex-acp`). Sets the default ACP `SessionConfigOption` for `provider`."
  - name: GOOSE_MODEL
    effect: "Selects the model for the current session (e.g. `gpt-5.2-codex`, `claude-sonnet-5`)."
  - name: GOOSE_FAST_MODEL
    effect: "Pins the small/fast model used for haiku-class tasks (added v1.41, #9296)."
  - name: GOOSE_SERVER__SECRET_KEY
    effect: "Shared secret required by `goose serve` for the `/acp` HTTP/WS endpoint. Constant-time-compared against `X-Secret-Key` header or `?token=` query parameter."
  - name: GOOSE_OAUTH_CALLBACK_PORT
    effect: "Stable OAuth redirect port (avoids races during re-auth)."
  - name: GOOSE_MAX_TOOL_RESPONSE_SIZE
    effect: "Tool output byte cap (added v1.41, #9256)."
  - name: GOOSE_MAX_TURNS
    effect: "Per-session max agent turns without user input (default 1000)."
  - name: GOOSE_CONTEXT_LIMIT
    effect: "Optional explicit context limit (string values now accepted; #9738 in v1.41)."
  - name: GOOSE_TUI_SCRIPT
    effect: "Override path to a prebuilt `ui/text/dist/tui.js` for the `goose tui` command."
  - name: GOOSE_TUI_NPM_SPEC
    effect: "Override the npm package spec used to resolve the goose TUI (default `@aaif/goose@latest`)."
  - name: GOOSE_DISABLE_SESSION_NAMING
    effect: "Skips automatic LLM-generated session name updates on `SessionInfoUpdate`."
  - name: ADDITIONAL_AGENT_SOURCE_ROOTS
    effect: "Space-separated (PATH-list) list of additional read-only source roots goose should treat as belonging to the session's project."
rust_client:
  crate: agent-client-protocol
  connection_type: "AcpAgent::from_str(\"goose acp\") for stdio; Axum + agent-client-protocol-http for the HTTP/WS path"
  localset_required: false
  reverse_request_handlers:
    - session/request_permission
    - fs/read_text_file
    - fs/write_text_file
    - terminal/create
    - terminal/output
    - terminal/wait_for_exit
    - terminal/kill
    - terminal/release
    - elicitation/create
    - _goose/unstable/recipe_params
    - _goose/unstable/agent_mentions
  desktop_streaming_pattern: "tokio::sync::mpsc from the `session/update` handler to the UI thread; AcpServer is async and doesn't require LocalSet."
  notes: "Goose's dependency line pins `agent-client-protocol = 1.0`, `agent-client-protocol-schema = 1.1` (with the `unstable` feature set, so the `_goose/unstable/*` namespace is reachable), and `agent-client-protocol-http = 1.0` (server + `unstable_cancel_request` features). Match the same crate features when building a client so the schema is current."
compatibility:
  - client: Zed editor
    status: works
    issue: "Zed 1.5.0+ can install goose from the ACP Registry (`Install from Registry → goose`); older Zed users have to add a custom agent entry manually."
    workaround: "Use registry install on Zed 1.5.0+. Otherwise add `{ \"agent_servers\": { \"goose\": { \"type\": \"custom\", \"command\": \"goose\", \"args\": [\"acp\"] } } }` to your Zed settings (the VS Code extension `vscode-goose` uses the same shape)."
  - client: vscode-goose (VS Code extension)
    status: works
    issue: "Reference implementation published by the goose team; uses `goose acp` over stdio."
    workaround: "Install from the marketplace or build from https://github.com/aaif-goose/vscode-goose."
  - client: Goose TUI (`ui/text`)
    status: works
    issue: "TypeScript TUI shipped in `ui/text/`. Auto-spawns `goose acp` or connects to `goose serve` over HTTP."
    workaround: "`cd ui/text && npm install && npm start` (or `npm start -- --server http://HOST:PORT` for HTTP/WS)."
  - client: "Clients that don't advertise `client_capabilities.fs.*` or `terminal`"
    status: partial
    issue: "goose's in-process developer extension handles read/write/shell internally — clients see only `session/request_permission`, not fs/terminal reverse requests."
    workaround: "Advertise the fs/terminal capabilities if you want to host those operations; otherwise implement permission UI only and use the in-process path as fallback."
  - client: "HTTP clients with `goose serve` and CORS"
    status: partial
    issue: "Default origin policy is `loopback_and_null_and_file`; any value passed to `--allowed-origin` replaces loopback entirely (don't forget `app://localhost` for Tauri-style hosts)."
    workaround: "Pass `--allowed-origin http://localhost:5173 --allowed-origin app://localhost --allowed-origin https://app.example` etc. For local dev only, `--dangerously-unauthenticated` skips bearer-token requirement."
  - client: "MCP servers in client `context_servers`"
    status: works
    issue: "goose automatically makes them available alongside its own extensions — `context_servers` with stdio or HTTP transports is accepted; the deprecated SSE transport is rejected."
    workaround: "Migrate SSE servers to streamable_http if you want them inside an ACP-hosted goose."
quirks:
  - "**Repository moved from `block/goose` to `aaif-goose/goose`** under the Agentic AI Foundation (Linux Foundation). The block/goose URL still works but redirects; the official canonical repo is now github.com/aaif-goose/goose. (Note: the official ACP Registry page still references `https://github.com/block/goose` because the registry index is in the middle of being updated.)"
  - "goose is listed in the official ACP Registry at version 1.41.0 — Zed 1.5.0+ picks it up automatically via \"Install from Registry\"."
  - "`goose configure` does **not** require the ACP server to be running. The CLI's own provider-auth UI runs out of band; ACP `authenticate` always returns an empty response."
  - "`goose acp` always enables the `developer` builtin extension by default; `--with-builtin` adds more (e.g. `developer,memory,tutorial`). MCP servers provided in `session/new.mcpServers` then either replace or supplement these (recipe / `gooseExtensions` meta takes precedence)."
  - "When `session/request_permission` arrives, the four `optionId` strings are the schema strum-serialize snake_case of `PermissionOptionKind` — literally `allow_once`, `allow_always`, `reject_once`, `reject_always` — and `Cancelled` is the schema-canonical cancellation outcome (no `cancel` option in the list)."
  - "**SSE is explicitly rejected**: any `McpServer::Sse` in `session/new.mcpServers` returns `\"SSE is unsupported, migrate to streamable_http\"`. Only `McpServer::Stdio` and `McpServer::Http` (streamable_http) are accepted."
  - "**Image paths with spaces** are detected correctly in tool arguments only after v1.41 (bugfixes #9387 and #10098); earlier versions mis-escape shell-escaped image paths on some platforms."
  - "**`McpCapabilities.sse` is false** at initialize: goose advertises Streamable HTTP only. Clients should not assume SSE support."
  - "**`SessionUpdate::Plan` is unused** — goose has no Plan session-update construction as of v1.41. Multi-step planning surfaces as the `tool_chain_summary` meta on the closing ToolCallUpdate (#8995)."
  - "The `_goose/unstable/...` custom reverse-request namespace (`recipe_params`, `agent_mentions`, `session/steer`) is governed by the external `goose-sdk-types` crate. Version drift between `crates/goose` and `goose-sdk-types` is the leading indicator of \"what's new in the latest release.\""
  - "`goose serve` defaults to `127.0.0.1:3284` and refuses to start without `GOOSE_SERVER__SECRET_KEY` (or `--dangerously-unauthenticated`). WebSocket clients must use `?token=<secret>` because the browser WS API cannot set custom headers. The transport middleware rewrites the `Origin` header to `http://goose.local` after loopback validation per the agent-client-protocol-http crate's WS expectations."
  - "Local MCP app HTML (`/mcp-app-guest`) is served from a dynamically-bound loopback port (`0`) with a 300s TTL, 8 MiB body cap, and `frame-src` injected from the proxy query CSP."
  - "goose-server's standalone `goosed` binary is being retired in favor of `goose serve`; three `TODO(acp-migration)` comments in the source flag builtin-extension ownership and platform identity to migrate before deletion."
  - "Cursor is the most comparable parallel agent (ACP-native, also in the registry); cross-client parity is largely a matter of advertising `client_capabilities` consistently."
  - "ACP session ID is different from goose's own SQLite session id (and from any client-side session id). Both Claude and Codex ACP-wrappers in goose have similar semantic comments on telemetry correlation; goose-as-agent does not — the two ids map directly."
  - "Absolute paths and 1-based line numbers are required by ACP; goose itself is strict about this and rejects relative `cwd` in `session/new` (`validate_absolute_cwd` in `acp/server.rs`)."
  - "`apply_acp_extension_overrides` re-registers the developer extension with an ACP-aware wrapper the moment any of `fs.read_text_file`, `fs.write_text_file`, or `terminal: true` is advertised; clients that don't advertise any of them stay on the in-process developer tools and won't see fs/terminal reverse requests."
  - "Per the v1.41 changelog: TLS support for `goose serve` (#10088), pagination on `session/list` (#9199), tool-call availables exposed in ACP schema (#10097), thinking effort config option, context window size forwarded to clients (#9455), and silent `agentInfo`/`initialize` (#9765) were all landed in the same minor."
recent_changes:
  - date: 2026-07-03
    version: v1.41.0
    change: "TLS support for ACP serve (#10088); exposed available tools in ACP schema (#10097); paginated session/list in ACP (#9199); ACP context window size forwarded to clients (#9455); agentInfo included in initialize response (#9765)."
    impact: "Any new client capability that pulls discovered tools or ctx size from ACP will now Just Work without extra adapters."
  - date: 2026-07-03
    version: v1.41.0
    change: "Thinking effort config option (`thinking_effort`) over ACP (`SetSessionConfigOption` / `ConfigOptionUpdate`)."
    impact: "Drives goose's `ModelConfig.thinking_effort` from ACP. Pre-existing; landed late in v1.41 per the changelog '#10711'."
  - date: 2026-07-03
    version: v1.41.0
    change: "ACP cancel race condition fixed (#9804); recipe state applied during session load and fork (#9998); session info ACP method (#9729); images replayed on session load (#9496); ACP streaming chunks coalesced under one message id (#8788)."
    impact: "Stability fixes that affect every ACP client."
  - date: 2026-07-03
    version: v1.41.0
    change: "Last message snippets for ACP sessions (#9798); allow local file ACP origins (#10194); OTLP logging schema for cross-tool detection (#9713)."
    impact: "Useful when wrapping goose in a desktop IDE that passes file:// origins."
  - date: 2026-07-03
    version: v1.41.0
    change: "ACP module migrated to the new `agent-client-protocol-http` crate (#10082); ACP SDK upgraded; UI connected to ACP directly instead of `goosed` (#10081)."
    impact: "Confirms the long-term deprecation of `goosed` in favor of `goose serve`."
  - date: 2026-07-03
    version: v1.41.0
    change: "Per the major changelog block on the release page: hooks feature (#9093), Open-plugins + skills (#9112), TUI diff viewer (#9260), `/model` (#8747), `/status` (#9845), `/goal` (#9069), structured per-provider config (#8977), unified thinking effort (#9242), `goose://new-session` / `goose://resume` deep links (#9196 #9343), structured summon task load results (#9521), proactive OAuth token refresh (#8386)."
    impact: "These features aren't ACP-exclusive but are visible to ACP clients via session metadata, slash commands, and the provider.model_id surface."
  - date: 2026-06 (approx)
    version: pre-v1.41
    change: "Declarative provider catalogue split into `goose-providers` crate; remote `goosed` server docs (#9275)."
    impact: "Provider inventory backend is now exposed via `--transport provider_inventory`-shaped APIs in ACP; tracks `@codex/copilot` style splits."
  - date: 2026-04-07
    version: "AAIF migration announcement"
    change: "Repository moved from `block/goose` to `aaif-goose/goose` under the Agentic AI Foundation; docs redirects went live (`block.github.io/goose` → `goose-docs.ai`)."
    impact: "Old `block/goose` URLs continue to redirect; new issues/PRs go to `aaif-goose/goose`. The ACP Registry page still references block/goose and will be updated."
gaps:
  - "The full set of reverse requests under the `_goose/...` namespace is owned by the external `goose-sdk-types` crate; only a few (recipe_params, agent_mentions, session/steer) are used directly inside `crates/goose/src/acp/**`. Additional methods may exist on the goose side that this research has not surfaced."
  - "No first-party CHANGELOG.md was reachable at research time — the version-by-version drift of the `agent-client-protocol*` crate set, the `_goose/unstable/...` namespace, and the session-update surface is best reconstructed by inspecting `crates/goose/Cargo.toml` against `crates/goose/Cargo.lock`."
  - "Whether `goose serve` ever exposes `McpCapabilities` over ACP independent of the `McpServer` argument to `session/new` (i.e., whether the JSON-RPC server can itself advertise itself as a host) — appears `no`; not confirmed."
  - "`goose acp` was not actually launched on this host — `which goose` returns not found and the repo's `download_cli.sh` was not run because of non-interactive constraints. Negative probes in this document are based on source inspection plus documented behavior; behavior matches the assertions in `test_acp_client.py` but was not observed live."
  - "The ACP Registry page still references `https://github.com/block/goose` rather than `https://github.com/aaif-goose/goose`; this is a registry staleness finding rather than a goose problem, but it would surprise a new client."
  - "Whether `goose server`'s WS origin overwrite to `http://goose.local` ever conflicts with browsers that prefer `https://...` is undocumented; the docs warn \"set explicitly\" but the HTTP path is fine with `127.0.0.1`."
changes: []
requires_claudine_update: false
reason: "Claudine already wraps the `goose` CLI directly (one of the 8 providers in its Provider enum) — this research documents existing capability, not a change that would force an immediate Claudine code modification. A future ACP-launch-mode for goose would be a feature addition, not a fix. No `--requires` regression surfaced from this research."
---

# Goose ACP Research

## Overview

goose is an open-source AI agent built in Rust. As of v1.41.0 (released 2026-07-03 — the day of this research), the `goose` CLI binary implements the Agent Client Protocol **natively**, with no adapter or bridge process required.

A standout detail: as of April 7, 2026 the project moved from `block/goose` to the [Agentic AI Foundation](https://aaif.io/) under the Linux Foundation. The canonical repository is now [github.com/aaif-goose/goose](https://github.com/aaif-goose/goose). Almost all the code that supports ACP lives in [`crates/goose/src/acp/`](https://github.com/aaif-goose/goose/tree/main/crates/goose/src/acp) and is wired through the `acp` and `serve` subcommands of `goose-cli`. The workspace declares direct dependencies on `agent-client-protocol 1.0`, `agent-client-protocol-schema 1.1` (with the `unstable` feature), and `agent-client-protocol-http 1.0` (server + `unstable_cancel_request` features).

Two distinct transports are exposed:

| Surface | Command | Transport |
|---------|---------|-----------|
| stdio JSON-RPC | `goose acp` | newline-delimited JSON over stdin/stdout |
| HTTP + WebSocket | `goose serve` | `agent-client-protocol-http` over an axum router (default `127.0.0.1:3284`), auth via `GOOSE_SERVER__SECRET_KEY` |

Both run the same `AcpServer::new` configuration built from `crates/goose/src/acp/server_factory.rs`. The standalone `goosed` binary in `crates/goose-server` is still shipped but is transitional — three `TODO(acp-migration)` comments in the source flag builtin-extension ownership and platform identity that need to move into `goose serve` before `goosed` is deleted. The desktop UI already launches `goosed` directly today (PR #10081 noted "UI connected to ACP directly instead of goosed" late in v1.41).

A reference TUI client is shipped at `ui/text/` in the same repository; it speaks ACP against either a local `goose acp` or an HTTP server, and renders the four-option permission UI (`y`/`a`/`n`/`N`) documented in the user guide.

Classification per the task brief: **native**, **not adapter**.

## Launching ACP

### Recommended: stdio (`goose acp`)

```bash
goose acp [--with-builtin NAME,NAME,...]
```

Boots a single-process JSON-RPC server speaking the ACP `v1` schema on stdin/stdout. Builtin extensions default to `developer` if `--with-builtin` is omitted. The dispatch loop lives in `crates/goose/src/acp/server.rs` (`GooseAcpAgent` + `serve()`); per-request handlers are split across `crates/goose/src/acp/server/{dispatch,new_session,load_session,list_sessions,fork_session,close_session,prompts,recipe,elicitation,...}.rs`.

Zed (1.5.0+) discovers this command automatically through the ACP Registry. Manual Zed setup:

```json
{
  "agent_servers": {
    "goose": {
      "type": "custom",
      "command": "goose",
      "args": ["acp"]
    }
  }
}
```

Negative probe on this host: `which goose` returns not found, and the official `download_cli.sh` was not run (this is a non-interactive research session that should not install software). All behavior below is reconstructed from `crates/goose/src/acp/**` and the `test_acp_client.py` test harness at the repo root, which spawns `cargo run -p goose-cli -- acp` and runs through `initialize`, `session/new`, `session/prompt`, and `session/load`.

### HTTP/WS (`goose serve`)

```bash
GOOSE_SERVER__SECRET_KEY="$(openssl rand -hex 32)" \
  goose serve \
    --host 127.0.0.1 \
    --port 3284
```

Same `AcpServer::new(AcpServerFactoryConfig { builtins, ... })` underneath, mounted under `/acp` on an axum router (`crates/goose/src/acp/transport/mod.rs`). Optional `--tls --tls-cert-path PATH --tls-key-path PATH` enables rustls-based TLS. Authentication is a constant-time-compare in `crates/goose/src/acp/transport/auth.rs`:

- `X-Secret-Key: <secret>` request header, or
- `?token=<secret>` query parameter (for WebSocket clients; the browser WS API cannot set custom headers)

If neither `GOOSE_SERVER__SECRET_KEY` is set nor `--dangerously-unauthenticated` is passed, the server refuses to start. CORS defaults to loopback + `null` + `file://`; passing any `--allowed-origin` replaces those defaults entirely.

Non-ACP auxiliary endpoints on the same port: `/health` and `/status` (200 → `"ok"`, CORS `*`), `/mcp-app-proxy` and `/mcp-app-guest` for MCP-app iframe HTML.

### Standalone `goosed`

The deprecated bridge binary, `goosed`, runs the same routing under `crates/goose-server/src/main.rs`. Treat it as a temporary alternative while `goose serve` finishes absorbing its desktop-specific responsibilities (builtin extension registration, `GoosePlatform::GooseDesktop` initialization).

### Reference TUI client (`goose tui`)

goose ships its own TypeScript TUI at `ui/text/`:

```bash
cd ui/text
npm install
npm start                                # auto-spawns `goose acp`
npm start -- --server http://HOST:PORT   # connect to a `goose serve` instance
npm start -- --text "What files are in this directory?"   # single-prompt and exit
```

The TUI is shipped via the npm package `@aaif/goose` (resolution order: `GOOSE_TUI_SCRIPT` → a local checkout → `npx --yes --package <spec> -- goose-tui`).

## Protocol and Capabilities

### Capability surface

| Area | Source-of-truth line | Status |
|------|----------------------|--------|
| `initialize` / `authenticate` | `crates/goose/src/acp/server.rs` `on_initialize` | supported |
| `session/new` | `crates/goose/src/acp/server.rs` `on_new_session` | supported |
| `session/load` | `crates/goose/src/acp/server.rs` `on_load_session` | supported |
| `session/prompt` | `crates/goose/src/acp/server.rs` `on_prompt` | supported |
| `session/cancel` | `crates/goose/src/acp/server.rs` `on_cancel` | supported |
| `session/list` | `crates/goose/src/acp/server.rs` `on_list_sessions` | supported (paginated since #9199) |
| `session/close` | `crates/goose/src/acp/server.rs` `on_close_session` | supported |
| `session/fork` | `crates/goose/src/acp/server.rs` `on_fork_session` | supported |
| `session/set_mode` | `crates/goose/src/acp/server.rs` `on_set_mode` | supported |
| `session/set_config_option` | dispatch on `provider` / `mode` / `model` / `thinking_effort` | supported |
| `session/load` history replay | `crates/goose/src/acp/server.rs` + `test_acp_client.py` | supported — replays persisted history as `session/update` notifications |
| `session/request_permission` | `crates/goose/src/acp/server.rs:1885` | supported |
| `fs/read_text_file` / `fs/write_text_file` | `crates/goose/src/acp/fs.rs` | conditional (gated on `client_capabilities.fs.*`) |
| `terminal/*` | `crates/goose/src/acp/fs.rs` `acp_shell` | conditional (gated on `client_capabilities.terminal`) |
| `elicitation/create` | `crates/goose/src/acp/server/elicitation.rs` | conditional (gated on `client_capabilities.elicitation.form`) |
| `agentCapabilities.loadSession` | `agent_client_protocol::schema::v1::AgentCapabilities` | true |
| `agentCapabilities.promptCapabilities.image` | " | true |
| `agentCapabilities.promptCapabilities.audio` | " | false (audio prompts dropped) |
| `agentCapabilities.promptCapabilities.embedded_context` | " | true |
| `agentCapabilities.sessionCapabilities.list` | " | true (paginated since #9199) |
| `agentCapabilities.sessionCapabilities.close` | " | true |
| `agentCapabilities.mcpCapabilities.http` | " | true |
| `agentCapabilities.mcpCapabilities.sse` | " | false (goose rejects SSE outright) |
| `agentCapabilities.meta.goose.localInference` | `agent_capabilities_meta()` | true **iff** built with `--features local-inference` |
| Protocol-level `$/cancel_request` | cargo `agent-client-protocol-http = ... features = ["server", "unstable_cancel_request"]` | supported |

The lone advertised `authMethods` entry is `AuthMethod::Agent(AuthMethodAgent::new("goose-provider", "Configure Provider"))` — the description is "Run `goose configure` to set up your AI provider and API key". Real authentication happens through `goose configure` / environment variables, not over ACP.

### MCP server config accepted on `session/new`

`McpServer` (per `crates/goose/src/acp/server.rs:377`):

| Variant | Mapping | Sse? |
|---------|---------|------|
| `McpServer::Stdio` | `ExtensionConfig::Stdio { name, cmd, args, envs: Envs, env_keys: vec![], timeout, cwd: None, bundled: Some(false), available_tools: vec![] }` | n/a |
| `McpServer::Http` | `ExtensionConfig::StreamableHttp { name, uri, envs: Envs::default(), env_keys: vec![], headers: HashMap, timeout, socket: None, bundled: Some(false), available_tools: vec![] }` | n/a |
| `McpServer::Sse` | `Err("SSE is unsupported, migrate to streamable_http".to_string())` | rejected |

`timeout` for both variants is read from `meta.timeout` (u64 seconds). If a recipe or `gooseExtensions` meta is provided, those win; if `mcpServers` is empty AND no recipe/gooseExtensions, the config-file defaults (`get_enabled_extensions_with_config`) and enabled plugin MCP servers are used.

## Reverse Requests

### Required: `session/request_permission`

goose's *only* reliably-issued standard reverse request is `session/request_permission`. Every `ToolConfirmation` reaching the agent loop surfaces here. The option `id`s are the literal strum-serialize snake_case of `PermissionOptionKind`:

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "session/request_permission",
  "params": {
    "sessionId": "<goose session id>",
    "toolCall": {
      "toolCallId": "tool_<n>",
      "title": "Read /path/to/file.rs",
      "kind": "read",
      "status": "pending",
      "content": [],
      "locations": [{"path": "/path/to/file.rs", "line": 1}],
      "rawInput": {"path": "/path/to/file.rs"}
    },
    "options": [
      { "optionId": "allow_always", "name": "allow_always", "kind": "allow_always" },
      { "optionId": "allow_once",   "name": "allow_once",   "kind": "allow_once"   },
      { "optionId": "reject_once",  "name": "reject_once",  "kind": "reject_once"  },
      { "optionId": "reject_always","name": "reject_always","kind": "reject_always"}
    ]
  }
}
```

Client behavior — respond with one of:

| `optionId` | Internal effect |
|------------|-----------------|
| `allow_once` | `PermissionConfirmation { principal_type: Tool, permission: AllowOnce }` |
| `allow_always` | `PermissionConfirmation { principal_type: Tool, permission: AlwaysAllow }` (also persisted into `permission.yaml`) |
| `reject_once` | `Tool, DenyOnce` |
| `reject_always` | `Tool, AlwaysDeny` (also persisted into `permission.yaml`) |
| *anything else* | `Tool, Cancel` |
| `Cancelled` outcome | `Tool, Cancel` |

Important: there is **no `cancel` option in the list**; cancellation must be communicated by sending the outcome `Cancelled`.

A Claudine-friendly client at minimum must handle `session/request_permission`. All other reverse requests are capability-gated.

### Conditional: filesystem (`fs/read_text_file`, `fs/write_text_file`)

Issued only when the client advertises `client_capabilities.fs.read_text_file` and/or `fs.write_text_file`. The path is absolute; line numbers are 1-based. Verified handler shape from `crates/goose/src/acp/fs.rs`:

```rust
let mut request = ReadTextFileRequest::new(session_id.clone(), path.to_path_buf());
if let Some(l) = line { request = request.line(l); }
if let Some(l) = limit { request = request.limit(l); }
let response = cx.send_request(request).block_task().await
    .map_err(|e| format!("{e:?}"))?;
Ok(response.content)
```

If the client does not advertise those capabilities, `read`/`write`/`edit` developer tools fall back to direct `fs::read_to_string` / `fs::write(...)` against the in-process developer extension. The ACP-aware wrapper tags every tool result with `_goose/acp-aware: true` in `_meta` so `handle_tool_response` knows to skip its in-process content/locations rebuild.

### Conditional: terminal (`terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release`)

All five are used by the `shell` developer tool when `client_capabilities.terminal: true`. The full lifecycle is wrapped in a single `acp_shell` Rust method:

```rust
let create_res = self.cx.send_request(
    CreateTerminalRequest::new(self.session_id.clone(), &params.command)
        .cwd(ctx.working_dir.clone())
        .output_byte_limit(OUTPUT_LIMIT_BYTES as u64),
).block_task().await?;
let terminal_id = create_res.terminal_id;
self.update_tool_call(ctx, ToolCallUpdateFields::new()
    .content(vec![ToolCallContent::Terminal(Terminal::new(terminal_id.clone()))]));
let result = self.run_terminal_to_completion(&terminal_id, params.timeout_secs).await;
let _ = self.cx.send_request(
    ReleaseTerminalRequest::new(self.session_id.clone(), terminal_id.clone())
).block_task().await;
```

`run_terminal_to_completion` issues `WaitForTerminalExitRequest` with a `tokio::time::timeout` against `params.timeout_secs`, then `KillTerminalRequest` on timeout, then `TerminalOutputRequest`. After cleanup, `terminal/release` is always fired (best-effort) — leak hazard if a client ever short-circuits that step. `OUTPUT_LIMIT_BYTES` is a module constant in `acp/fs.rs`.

### Conditional: `elicitation/create`

Used when the client advertises `client_capabilities.elicitation.form`. Decline and cancel are propagated end-to-end (#9437, #8999). Routes through `crates/goose/src/acp/server/elicitation.rs`; falls back to a logged cancel otherwise.

### Custom: `_goose/unstable/...`

Three custom reverse requests use the `_goose/unstable/` namespace. The catalogue is owned by the external `goose-sdk-types` crate (`pub use goose_sdk_types::custom_requests` in `crates/goose/src/acp/mod.rs`) — additional methods may exist there that this research did not surface:

| Method constant | Handler file | Purpose |
|------------------|--------------|---------|
| `_goose/unstable/recipe_params` | `crates/goose/src/acp/server/agent_requests.rs` | Ask client to prompt for recipe parameters |
| `_goose/unstable/agent_mentions` | `crates/goose/src/acp/server/agent_mentions.rs` (`on_list_agent_mentions`) | List Subrecipes + filesystem-summoned agents |
| `_goose/unstable/session/steer` | `crates/goose/src/acp/server.rs` (`on_steer_session`) | Steer an in-progress session from a queued prompt |

All three currently flow through the `agent.dispatch_custom_request` fallback in `dispatch.rs`.

## Permissions, Filesystem, and Terminal

### Permission policy

- The four option `id`s are exactly `allow_once / allow_always / reject_once / reject_always`. Clients pick one of those, or respond with `Cancelled`.
- The `GooseMode` enum on the agent side maps to a default policy:
  - `auto` → `bypassPermissions` (no `session/request_permission` is emitted — goose synthesizes an `AllowOnce` internally before forwarding to the agent loop).
  - `smart_approve` → `acceptEdits` (auto-approve edits; prompt for risky operations).
  - `approve` → `default` (always prompt).
  - `chat` → `plan` (no tool execution; `session/request_permission` for anything that looks like a tool call resolves to `RejectOnce`).
- `apply_acp_extension_overrides` in `crates/goose/src/acp/server.rs:1063-1081` is the single gate that decides whether the in-process `developer` extension is replaced with the ACP-aware wrapper. It activates when **any** of `client_capabilities.fs.read_text_file`, `client_capabilities.fs.write_text_file`, or `client_capabilities.terminal` is true. Otherwise the developer extension stays in-process and never issues fs/terminal reverse requests — clients should either set those capabilities to receive the reverse requests or implement only the permission handler and accept that fs/terminal are server-side.
- Selecting `allow_always` or `reject_always` causes the decision to be persisted into the per-working-dir `permission.yaml` (so re-launching the agent in the same project applies it).
- On the ACP layer, when a prompt is canceled, any pending `session/request_permission` is resolved with `Cancelled` by the dispatcher.

### Filesystem policy

- Absolute paths only. Relative paths to `cwd` are accepted in tool arguments, but `session/new.cwd` itself is validated by `validate_absolute_cwd` (`crates/goose/src/acp/server.rs`) — must exist and be absolute.
- Line numbers are 1-based.
- Tool-call locations extracted from `developer_extension` arguments (read/edit/write/shell) are surfaced on `ToolCallUpdate.locations` for client-side diff renderers.

### Terminal policy

- Lifecycle: create → wait_for_exit (with optional timeout-driven kill) → release. `terminal/release` always fires as a safety net.
- Output buffer is truncated from the beginning once `output_byte_limit` (defaults to `OUTPUT_LIMIT_BYTES` in `acp/fs.rs`) is exceeded.
- cwd defaults to the session's `cwd`. Environment variables are passed verbatim in `terminal/create.env`.
- ACP clients that don't advertise `terminal: true` get no reverse requests; `shell` runs entirely in-process via `tokio::process::Command`.

## Streaming and UI Integration

Streaming flows through `session/update` notifications. Goose emits nearly every standard variant:

| Update | Emitted? | Where |
|--------|---------|-------|
| `AgentMessageChunk` | yes | `handle_message_content` for assistant text |
| `UserMessageChunk` | yes | `handle_message_content` for user text |
| `AgentThoughtChunk` | yes | `handle_message_content` for thinking |
| `ToolCall` | yes | `handle_tool_request` — initial pending tool |
| `ToolCallUpdate` | yes | status / title / content / diff / locations / chain summary |
| `CurrentModeUpdate` | yes | after `session/set_mode` |
| `ConfigOptionUpdate` | yes | after every `session/set_config_option`, again after provider refresh |
| `SessionInfoUpdate` | yes | active run id, queued steer, session-name auto-update |
| `AvailableCommandsUpdate` | yes | at session setup |
| `UsageUpdate` | yes | at session setup + end of every prompt |
| `Plan` | **no** | no constructor in current source |

Group by `ContentChunk.message_id` to coalesce parallel streams. Schema v1.1.0 also dropped the `$/cancel_request` protocol-level cancellation on the HTTP transport.

**Custom notifications under `_goose/`:**

- `meta.goose.useLoginShellPath` — surfaced in `initialize` only (hint back to goose about whether to spawn subagents with the login-shell PATH).
- `meta.goose.localInference = {}` — only when the agent was built with `--features local-inference`.
- `meta.goose.messageCount` + `userSetName` — on every `SessionInfoUpdate` (auto-generated session names).
- `meta.goose.activeRunId` — opaque active run token (or null on completion).
- `meta.goose.queuedSteer = { messageId, runId }` — after a queued steer completes.
- `meta.goose.toolChainSummary = { summary, count }` — closing summary on multi-tool `ToolCallUpdate`.
- `meta.goose.toolCall = { toolName, extensionName }` — identity meta on every `ToolCall`/`ToolCallUpdate`.
- `_goose/acp-aware: true` — set by the ACP-aware developer wrapper so the server knows to skip its own content rebuild.
- `_goose/unstable/session/update` — paired with the standard `session/update` to carry status messages and usage (`GooseSessionUpdate::StatusMessage`, `GooseSessionUpdate::UsageUpdate`), only when the client opts in via `client_capabilities.meta.goose.customNotifications: true`.

The standard `UsageUpdate` (used/prompt) is always emitted; the standard `Cost { amount, currency: "USD" }` is added when `session.accumulated_cost` is non-null.

A Tauri / TUI / web UI should run the ACP client on its own tokio runtime, forward `SessionNotification` values through an `mpsc::UnboundedSender<AgentEvent>` to the UI thread, and reconcile notifications by `sessionId + (message_id | tool_call_id)` since chunks within a single session are not guaranteed to interleave cleanly across parallel streams.

## Authentication and Setup

goose-the-CLI uses `GooseMode` and a separate provider-config layer; ACP `authenticate` itself resolves empty. Real auth runs through:

1. **Provider credentials** — `goose configure` (writes YAML config + cross-platform keyring via the `keyring` crate) or environment variables per provider.
2. **Per-session provider/model** — `GOOSE_PROVIDER` and `GOOSE_MODEL` env vars, both honored by `goose acp`, `goose serve`, and the existing non-ACP CLI.
3. **`goose serve` shared secret** — `GOOSE_SERVER__SECRET_KEY` (or `--dangerously-unauthenticated`).
4. **OAuth redirect port** — `GOOSE_OAUTH_CALLBACK_PORT` for stable OAuth flows (#9209).
5. **ACP client-side `authMethods`** — goose advertises exactly one method, `id="goose-provider"`, name `"Configure Provider"` — a signal that the client should drive `goose configure` (via shell or documentation) rather than expect an interactive OAuth flow inside ACP itself.
6. **Headless guidance** — `GOOSE_MAX_TURNS` (default 1000), `GOOSE_CONTEXT_LIMIT`, `GOOSE_MAX_TOOL_RESPONSE_SIZE` (added in v1.41, #9256), and `GOOSE_FAST_MODEL` for haiku-class tasks (#9296).

For fully headless ACP launches, pre-seed the provider with `goose configure` or set `GOOSE_PROVIDER` + provider-specific credential env vars, then launch `goose acp` (or `goose serve` with the bearer secret). ACL/secret hygiene is up to the client — goose does not redacted `ANTHROPIC_API_KEY` etc. from logs.

## Compatibility, Quirks, and Workarounds

1. **Repo migration (April 2026)**: `block/goose` → `aaif-goose/goose`. Old URLs still redirect; new issues/PRs go to `aaif-goose/goose`. The official ACP Registry entry still points at `https://github.com/block/goose`.
2. **SSE is rejected**: any `McpServer::Sse` in `session/new.mcpServers` returns an error. Migrate to streamable_http.
3. **Optional fs/terminal capabilities**: clients that don't advertise `client_capabilities.fs.*` or `terminal` get no filesystem or shell reverse requests — goose runs read/write/shell in-process inside the developer extension. Set those capabilities to participate.
4. **goose serve CORS gotcha**: any `--allowed-origin` value replaces the loopback/null/file default wholesale — add all origins you need (including `app://localhost` for Tauri-style hosts).
5. **Custom `_goose/unstable/...` namespace**: keep the matching `goose-sdk-types` crate version when interoperating. The catalogue is external to `crates/goose/src/acp/**`.
6. **Image paths with spaces**: fixed in v1.41 (#9387, #10098) — earlier versions mis-escape shell-escaped image paths on some platforms.
7. **HTTP path TLS** requires `--tls-cert-path` + `--tls-key-path` alongside `--tls`; otherwise rustls starts without certs.
8. **`SessionUpdate::Plan` is unused**: multi-step chains surface as a closing summary on the `ToolCallUpdate` rather than a separate `Plan` entry list.
9. **`goosed` deprecation**: the standalone HTTP binary is being retired in favor of `goose serve`. The new (or recently-updated) desktop UI launches `goose serve` directly via PR #10081; older desktop builds still launch `goosed`.
10. **Tool-call ordering**: `_goose/acp-aware` no longer rebuilds content/diff in the server-side `handle_tool_response` because everything streamed into the live `ToolCallUpdate`; the closing `tool_chain_summary` is the only thing added retroactively for ≥2-tool chains.
11. **Pinned in v1.41 specifically**: `agent-client-protocol = 1.0` (Send/Sync connection types — `LocalSet` is no longer required), `agent-client-protocol-schema = 1.1` (with the `unstable` feature flag, which is what surfaces `_goose/unstable/...`), `agent-client-protocol-http = 1.0` (features `["server", "unstable_cancel_request"]`).
12. **OpenCode extension dispatch**: goose runs `--dangerously-unauthenticated` only as a local-dev escape hatch; do not enable it for any daemon that accepts remote browser traffic with the shell-capable developer extension loaded.

## Recent Changes

- **2026-07-03 / v1.41.0** (latest): TLS for ACP serve (#10088), available tools in ACP schema (#10097), paginated session/list (#9199), session info ACP method (#9729), `agentInfo` in initialize (#9765), context window size forwarded to clients (#9455), images replayed on session load (#9496), ACP streaming chunks coalesced under one message id (#8788), thinking effort config option (#10711). Per the v1.41 release page, this release also lands hooks (#9093), open-plugins + skills (#9112), TUI diff viewer (#9260), `/model` (#8747), `/status` (#9845), `/goal` (#9069), structured per-provider config (#8977), unified thinking effort (#9242), `goose://new-session`/`goose://resume` deep links (#9196, #9343), structured summon task load results (#9521), proactive OAuth token refresh (#8386), structured config (#8977), and the per-provider working-dir system-prompt injection (#8739).
- **2026-04-07**: Project transferred to the Agentic AI Foundation / Linux Foundation (`block/goose` → `aaif-goose/goose`). Docs redirects go live (`block.github.io/goose` → `goose-docs.ai`); CLAUDE.md / AGENTS.md paths inside the repo preserved.
- **2026 (pre-v1.41)**: provider catalog split into the `goose-providers` crate; remote `goosed` server docs (#9275); per-session working-dir system-prompt injection (#8739); pre-ToolUse denial in hooks (#9304).
- **Transport**: moved to the new `agent-client-protocol-http` crate (#10082, late in v1.41) — this is the dependency that brings the `unstable_cancel_request` capability. UI was connected to ACP directly in place of `goosed` (#10081) in the same release.

## Rust Client Example

goose's workspace itself is the canonical reference (`crates/goose/src/acp/server.rs` for the server side; `crates/goose-server/src/main.rs` for the wiring). A minimal client using `agent-client-protocol` 1.0 and the `agent-client-protocol-schema 1.1` (with `unstable`) feature:

```toml
# Cargo.toml
[dependencies]
agent-client-protocol          = "1.0"
agent-client-protocol-schema   = { version = "1.1", features = ["unstable"] }
tokio                          = { version = "1", features = ["full"] }
```

```rust
use agent_client_protocol::schema::v1::{
    ClientCapabilities, ContentBlock, FileSystemCapabilities, Implementation,
    InitializeRequest, NewSessionRequest, PromptRequest, SessionNotification,
    TextContent, ContentChunk, SessionUpdate,
};
use agent_client_protocol::util::MatchDispatchFrom;
use agent_client_protocol::{AcpAgent, Client};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = AcpAgent::from_str("goose acp")?;

    let caps = ClientCapabilities::new()
        .fs(FileSystemCapabilities {
            read_text_file: true,
            write_text_file: true,
        })
        .terminal(true)
        .meta(Some(serde_json::json!({
            "goose": {
                "customNotifications": true,
                "recipeParameterRequests": true,
                "mcpHostCapabilities": { "extensions": {} }
            }
        }).as_object().cloned().unwrap_or_default()));

    let init = InitializeRequest::new(ProtocolVersion::V1)
        .client_capabilities(caps)
        .client_info(Implementation::new("claudine", "0.1.0"));

    Client::builder()
        .name("claudine-goose")
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                async move {
                    if let SessionUpdate::AgentMessageChunk(chunk) = notification.update {
                        if let ContentBlock::Text(t) = chunk.content {
                            print!("{}", t.text);
                        }
                    }
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(agent, |connection| async move {
            let init_response = connection.send_request(init).block_task().await?;
            eprintln!("agent: {:?}", init_response.agent_info);

            let cwd = std::env::current_dir()?;
            let new_session = connection
                .send_request(NewSessionRequest::new(cwd, vec![]))
                .block_task()
                .await?;

            // Switch into approve mode so every tool call prompts:
            let _ = connection
                .send_request(
                    SetSessionModeRequest::new(
                        new_session.session_id.clone(),
                        SessionModeId::new("approve"),
                    ),
                )
                .block_task()
                .await?;

            let prompt = connection
                .send_request(PromptRequest::new(
                    new_session.session_id.clone(),
                    vec![ContentBlock::Text(TextContent::new(
                        "List the files in this directory.".into(),
                    ))],
                ))
                .block_task()
                .await?;

            eprintln!("stop reason: {:?}", prompt.stop_reason);
            Ok(())
        })
        .await?;
    Ok(())
}
```

For the HTTP transport, swap the `AcpAgent::from_str("goose acp")` line for a hand-rolled axum reqwest or `agent-client-protocol-http` client that connects to `http://127.0.0.1:3284/acp` and adds the `X-Secret-Key: $GOOSE_SERVER__SECRET_KEY` header.

## Rust Reverse Request Handling

Of the standard reverse requests, only `session/request_permission` is reliable. Add the fs/terminal handlers as a completeness pass for clients that want to participate in those paths:

```rust
use agent_client_protocol::schema::v1::{
    PermissionOptionKind, ReadTextFileRequest, ReadTextFileResponse,
    ReleaseTerminalRequest, ReleaseTerminalResponse, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, WaitForTerminalExitRequest, WaitForTerminalExitResponse,
    CreateTerminalRequest, CreateTerminalResponse, TerminalId,
    KillTerminalRequest, KillTerminalResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use std::path::{Path, PathBuf};
use tokio::process::Command;

async fn handle_permission(
    request: RequestPermissionRequest,
) -> anyhow::Result<RequestPermissionResponse> {
    // Default to the most-permissive single-shot option that isn't an "always" persistence:
    let option_id = request
        .options
        .iter()
        .find(|o| o.kind == PermissionOptionKind::AllowOnce)
        .map(|o| o.option_id.clone())
        .unwrap_or_else(|| {
            request
                .options
                .first()
                .map(|o| o.option_id.clone())
                .unwrap_or_default()
        });

    Ok(RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
        SelectedPermissionOutcome::new(option_id),
    )))
}

async fn handle_read(
    request: ReadTextFileRequest,
    root: PathBuf,
) -> anyhow::Result<ReadTextFileResponse> {
    let path = request.path.canonicalize()?;
    if !path.starts_with(&root) {
        anyhow::bail!("path {} outside project root {}", path.display(), root.display());
    }
    let content = tokio::fs::read_to_string(&path).await?;
    Ok(ReadTextFileResponse { content })
}

async fn handle_write(
    request: WriteTextFileRequest,
    root: PathBuf,
) -> anyhow::Result<WriteTextFileResponse> {
    let path = request.path.canonicalize()?;
    if !path.starts_with(&root) {
        anyhow::bail!("path {} outside project root {}", path.display(), root.display());
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, &request.content).await?;
    Ok(WriteTextFileResponse {})
}
```

Register them on the builder before `connect_with`:

```rust
let root = std::env::current_dir()?;

Client::builder()
    .on_receive_request(
        |request: RequestPermissionRequest, responder, _cx| async move {
            responder.respond(handle_permission(request).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
    .on_receive_request(
        |request: ReadTextFileRequest, responder, _cx| {
            let root = root.clone();
            async move { responder.respond(handle_read(request, root).await?) }
        },
        agent_client_protocol::on_receive_request!(),
    )
    .on_receive_request(
        |request: WriteTextFileRequest, responder, _cx| {
            let root = root.clone();
            async move { responder.respond(handle_write(request, root).await?) }
        },
        agent_client_protocol::on_receive_request!(),
    )
```

## Rust Host Command Handling

Implementers who want goose to delegate terminal work to the host need a complete terminal lifecycle. The `terminal/release` call is best-effort but should always be issued:

```rust
use agent_client_protocol::schema::v1::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

struct TerminalHandle {
    child: Option<Child>,
    stdout_buf: Vec<u8>,
    stderr_buf: Vec<u8>,
    output_limit: usize,
}

#[derive(Default, Clone)]
struct TerminalManager {
    terminals: Arc<Mutex<HashMap<TerminalId, TerminalHandle>>>,
}

async fn handle_create(
    request: CreateTerminalRequest,
    manager: &TerminalManager,
) -> anyhow::Result<CreateTerminalResponse> {
    let limit = request.output_byte_limit.unwrap_or(1_048_576) as usize;
    let child = Command::new(&request.command)
        .args(request.args)
        .envs(
            request
                .env
                .into_iter()
                .map(|e| (e.name, e.value)),
        )
        .current_dir(request.cwd.unwrap_or(std::env::current_dir()?))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let id = TerminalId::new(uuid::Uuid::new_v4().to_string());
    manager.terminals.lock().await.insert(
        id.clone(),
        TerminalHandle {
            child: Some(child),
            stdout_buf: Vec::new(),
            stderr_buf: Vec::new(),
            output_limit: limit,
        },
    );

    Ok(CreateTerminalResponse { terminal_id: id })
}

// terminal/output is a normal polled read; truncate from the beginning once the limit is
// exceeded, just like goose does with OUTPUT_LIMIT_BYTES in `acp/fs.rs`.
//
// terminal/wait_for_exit: tokio::time::timeout against an internal state mutation that
// records (ExitStatus) once the child has exited.
//
// terminal/kill: find the child, kill it; subsequent wait_for_exit should resolve.
//
// terminal/release: always call to drop the handle from `manager.terminals`,
// even when the process is still running. This is the leak-prevention step.
```

A reasonable implementation lives alongside `run_terminal_to_completion` in `crates/goose/src/acp/fs.rs` — that file is the canonical reference.

## Rust Desktop Streaming Bridge

A Tauri-friendly pattern: run the ACP client on its own tokio runtime, forward `SessionNotification` through an unbounded `mpsc`, and dispatch to the UI via Tauri's `AppHandle::emit`.

```rust
use agent_client_protocol::schema::v1::*;
use agent_client_protocol::{AcpAgent, Client};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextChunk(String),
    ThoughtChunk(String),
    ToolCallStarted { id: String, title: String },
    ToolCallUpdated { id: String, raw: serde_json::Value },
    PermissionRequest {
        request_id: String,
        tool_call: serde_json::Value,
        options: Vec<(String, String)>,
    },
    ModeUpdate(String),
    ConfigOptionUpdate(String),
    SessionInfoUpdate { title: String, run_id: Option<String> },
    UsageUpdate { used: u64, limit: u64 },
    TurnComplete { stop_reason: String },
    Error(String),
}

pub fn spawn_goose(
    project_dir: PathBuf,
) -> anyhow::Result<(mpsc::UnboundedReceiver<AgentEvent>, mpsc::UnboundedSender<String>)> {
    let (event_tx, event_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let (prompt_tx, mut prompt_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async move {
            // Pin a known version in production: `goose acp` instead of `@latest`.
            let agent = AcpAgent::from_str("goose acp")
                .expect("AcpAgent::from_str");

            Client::builder()
                .on_receive_notification(
                    {
                        let tx = event_tx.clone();
                        move |notification: SessionNotification, _cx| {
                            let tx = tx.clone();
                            async move {
                                let event = match notification.update {
                                    SessionUpdate::AgentMessageChunk(c) => match c.content {
                                        ContentBlock::Text(t) => Some(AgentEvent::TextChunk(t.text)),
                                        _ => None,
                                    },
                                    SessionUpdate::AgentThoughtChunk(c) => match c.content {
                                        ContentBlock::Text(t) => Some(AgentEvent::ThoughtChunk(t.text)),
                                        _ => None,
                                    },
                                    SessionUpdate::ToolCall(tc) => Some(AgentEvent::ToolCallStarted {
                                        id: tc.tool_call_id.to_string(),
                                        title: tc.title,
                                    }),
                                    SessionUpdate::ToolCallUpdate(tcu) => Some(
                                        AgentEvent::ToolCallUpdated {
                                            id: tcu.tool_call_id.to_string(),
                                            raw: serde_json::to_value(&tcu).unwrap_or_default(),
                                        },
                                    ),
                                    SessionUpdate::CurrentModeUpdate(m) => Some(AgentEvent::ModeUpdate(
                                        m.mode_id.to_string(),
                                    )),
                                    SessionUpdate::ConfigOptionUpdate(co) => Some(
                                        AgentEvent::ConfigOptionUpdate(
                                            serde_json::to_string(&co).unwrap_or_default(),
                                        ),
                                    ),
                                    SessionUpdate::SessionInfoUpdate(si) => Some(
                                        AgentEvent::SessionInfoUpdate {
                                            title: si.title.clone().unwrap_or_default(),
                                            run_id: si
                                                .meta
                                                .as_ref()
                                                .and_then(|m| m.get("goose"))
                                                .and_then(|g| g.get("activeRunId"))
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string()),
                                        },
                                    ),
                                    SessionUpdate::UsageUpdate(u) => Some(AgentEvent::UsageUpdate {
                                        used: u.used_tokens as u64,
                                        limit: u.total_tokens as u64,
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

// Tauri command:
#[tauri::command]
async fn send_prompt(
    state: tauri::State<'_, AppState>,
    prompt: String,
) -> Result<(), String> {
    state.prompt_tx.send(prompt).map_err(|e| e.to_string())
}

// Tauri listener (called once at app startup):
fn install_event_listener(
    mut event_rx: mpsc::UnboundedReceiver<AgentEvent>,
    handle: tauri::AppHandle,
) {
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                AgentEvent::TextChunk(text) => handle.emit("agent:text", text).ok(),
                AgentEvent::TurnComplete { stop_reason } => {
                    handle.emit("agent:done", stop_reason).ok()
                }
                AgentEvent::PermissionRequest {
                    request_id,
                    tool_call,
                    options,
                } => handle
                    .emit("agent:permission", serde_json::json!({
                        "id": request_id, "tool": tool_call, "options": options
                    }))
                    .ok(),
                _ => None,
            };
        }
    });
}

// iced equivalent:
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

Match against the canonical `acp/server.rs` (`/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/.opencode/skill/acp/SKILL.md` cross-ref) when adapting to a different ACP agent — the dispatcher patterns differ slightly between goose, Claude-Code, and Codex, especially around custom reverse requests.

## Claudine Integration Notes

A practical synthesis: Claudine already has the `goose` provider in its `Provider` enum; this research documents the existing-protocol layer for any future adapter-shaped work.

1. **Launch detection** — detect either `goose acp` (stdio) or `goose serve --port 3284` (HTTP) as the launch surface. `which goose` plus `--version` check (>= 1.41) is enough. The `goose cli` provider in Claudine (`crates/goose-cli`) already implements a non-ACP direct wrapper; an ACP adapter would be additive, not a replacement.
2. **Capability negotiation** — goose is opinionated: `fs/read_text_file`, `fs/write_text_file`, `terminal/*` reverse requests are capability-gated. Clients that don't advertise any of them stay in the in-process developer extension path; clients that do advertise them receive reverse requests *only* for the ones they advertised. Claudine must mirror that — advertise terminal support if it has a host shell, advertise fs capabilities if it wants to participate in read/write flows.
3. **Reverse-request routing** — only `session/request_permission` is reliable. Route it through Claudine's existing `permissions` machinery and `policy::PolicyEngine` (`claudine/docs/research/.../policy-engine.md`). For zoombie-style confirmations (e.g., `allow_always` / `reject_always` into `permission.yaml`), mirror goose's per-working-dir persistence pattern so subsequent prompts in the same project apply the prior decision.
4. **Custom `_goose/...` namespace** — implement the three reverse request handlers surfaced in this research (`recipe_params`, `agent_mentions`, `session/steer`) and accept the `_goose/unstable/session/update` notification to capture status messages and usage. Pin a `goose-sdk-types` crate version alongside `agent-client-protocol` to keep the schema aligned.
5. **Streaming bridge** — forward `session/update` notifications into Claudine's existing event pipeline. Group by `ContentChunk.message_id` and coalesce tool-chain summaries. The `meta.goose.*` namespace under `SessionInfoUpdate` is a free, opt-in source for auto-named sessions, active-run tracking, queued-steering visibility, and tool identity meta.
6. **Auth preconditions** — goose's `authenticate` returns empty; the real auth is `goose configure` + keyring, or per-provider env vars (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `DATABRICKS_TOKEN`, etc.). For a Claudine headless ACP launcher, pre-seed the provider and supply `GOOSE_PROVIDER` + `GOOSE_MODEL` rather than expecting an interactive flow inside ACP. For `goose serve`, set `GOOSE_SERVER__SECRET_KEY` and forward it via `X-Secret-Key`.
7. **Schema versioning** — pin `agent-client-protocol` 1.0 + `agent-client-protocol-schema` 1.1 (with `unstable`) and `agent-client-protocol-http` 1.0 with the `unstable_cancel_request` feature so the `$/cancel_request` propagation remains available.
8. **No force-multiplier change to existing integration** — Claudine's existing goose wrapper (one of its 8 providers) remains the right answer for non-ACP consumer flows. This research exists for the future ACP-launching path and does not flag a behavior regression in the current implementation.

## Changelog

- **2026-07-03**: Initial release of this research document. Captures the current (v1.41.0) state of native goose ACP support — stdio (`goose acp`), HTTP/WS (`goose serve` with `GOOSE_SERVER__SECRET_KEY`), full reverse-request lifecycle (permission, fs, terminal, elicitation, custom `_goose/unstable/...`), and the streaming/UI surface. Reflects the AAIF migration (`block/goose` → `aaif-goose/goose`), the `McpCapabilities` reality (HTTP only — SSE explicitly rejected), the missing `Plan` update, the v1.41-specific changelog block (TLS for serve, paginated session/list, agentInfo in initialize, context window forwarding, tool-call availables in schema), and the planned `goosed` retirement per the in-source `TODO(acp-migration)` comments.

## Sources

- [goose docs (new home)](https://goose-docs.ai/)
- [legacy `block.github.io/goose` (redirects to `goose-docs.ai`)](https://block.github.io/goose/)
- [goose GitHub repository (`aaif-goose/goose` after the AAIF move)](https://github.com/aaif-goose/goose)
- [v1.41.0 release notes](https://github.com/aaif-goose/goose/releases/tag/v1.41.0)
- ["goose in ACP Clients" user guide](https://goose-docs.ai/docs/guides/acp-clients)
- ["ACP Providers" user guide (goose acting as the *client* of remote ACP agents)](https://goose-docs.ai/docs/guides/acp-providers)
- [AAIF migration blog post (referenced from `aaif-goose/goose` README)](https://aaif.io/)
- [`crates/goose/src/acp/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/mod.rs) — public module surface
- [`crates/goose/src/acp/server.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/server.rs) — main ACP server (`GooseAcpAgent`, `serve()`)
- [`crates/goose/src/acp/fs.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/fs.rs) — `AcpTools`, fs/terminal reverse-request wrappers
- [`crates/goose/src/acp/transport/mod.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/transport/mod.rs) — axum router for HTTP/WS, CORS policy, `X-Secret-Key` auth middleware, MCP-app proxy
- [`crates/goose/src/acp/transport/auth.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/transport/auth.rs) — `check_acp_token` middleware (constant-time `X-Secret-Key` / `?token=` compare)
- [`crates/goose/src/acp/server_factory.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/server_factory.rs) — `AcpServer::new(AcpServerFactoryConfig { ... })`
- [`crates/goose/src/acp/response_builder.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/response_builder.rs) — mode/state/option builders
- [`crates/goose/src/acp/server/dispatch.rs`](https://github.com/aaif-goose/goose/tree/main/crates/goose/src/acp/server) — request-to-handler dispatch (`session/set_mode`, `session/set_config_option`, custom requests)
- [`crates/goose/src/acp/server/elicitation.rs`](https://github.com/aaif-goose/goose/tree/main/crates/goose/src/acp/server/elicitation.rs) — `elicitation/create` reverse request
- [`crates/goose/src/acp/server/agent_requests.rs`](https://github.com/aaif-goose/goose/tree/main/crates/goose/src/acp/server/agent_requests.rs) — `_goose/unstable/recipe_params`
- [`crates/goose/src/acp/server/agent_mentions.rs`](https://github.com/aaif-goose/goose/tree/main/crates/goose/src/acp/server/agent_mentions.rs) — `_goose/unstable/agent_mentions`
- [`crates/goose/src/acp/provider.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/acp/provider.rs) — `AcpProvider` (goose acting as the **client** of a remote ACP agent)
- [`crates/goose-cli/src/cli.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs) — `goose acp`, `goose serve`, all subcommands
- [`crates/goose-server/src/main.rs`](https://github.com/aaif-goose/goose/blob/main/crates/goose-server/src/main.rs) — standalone `goosed` HTTP binary (transitional)
- [`test_acp_client.py`](https://github.com/aaif-goose/goose/blob/main/test_acp_client.py) — upstream Python test harness that exercises `initialize`, `session/new`, `session/prompt`, `session/load`
- [`Cargo.toml` (workspace)](https://github.com/aaif-goose/goose/blob/main/Cargo.toml) — pins `agent-client-protocol = "1.0"`, `agent-client-protocol-schema = "1.1"` (with the `unstable` feature set), `agent-client-protocol-http = "1.0"` (with `server, unstable_cancel_request`)
- [Agent Client Protocol specification (`agentclientprotocol.com`)](https://agentclientprotocol.com/)
- [ACP schema reference (1.1.0)](https://agentclientprotocol.com/protocol/schema)
- [ACP Registry (goose is the registry's goose entry at v1.41.0)](https://agentclientprotocol.com/registry)
- [agent-client-protocol Rust crate (1.0.x)](https://docs.rs/agent-client-protocol/1.0.1/agent_client_protocol/)
- [agent-client-protocol-schema 1.1](https://docs.rs/agent-client-protocol-schema/1.1.0/agent_client_protocol_schema/)
- [vscode-goose (reference VS Code client)](https://github.com/aaif-goose/vscode-goose)
- [Zed editor](https://zed.dev/) and the `agent_servers` goose configuration shape documented in `goose-docs.ai/docs/guides/acp-clients`
