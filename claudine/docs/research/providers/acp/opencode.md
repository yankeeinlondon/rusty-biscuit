---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://opencode.ai/docs
acp_docs: https://opencode.ai/docs/acp/
repo: https://github.com/anomalyco/opencode
support: native
launch_modes:
  - command: opencode
    args: ["acp"]
    transport: stdio
    adapter: none
    notes: "Native ACP mode embedded in the primary CLI binary. Verified against the installed v1.17.13 (`opencode acp --help` lists `start ACP (Agent Client Protocol) server`; `initialize` returns `protocolVersion: 1` with OpenCode-native capabilities). Process owns the JSON-RPC connection: stdout is the response stream, stdin is the request stream, stderr is reserved for `--print-logs` / `--log-level` (never JSON-RPC)."
  - command: opencode
    args: ["acp", "--port", "4096", "--hostname", "127.0.0.1"]
    transport: tcp
    adapter: none
    notes: "Same `acp` subcommand with the `--port` and `--hostname` flags exposed for HTTP/TCP-style clients. The CLI docs only document these flags in `opencode serve`; for ACP, stdio is the canonical transport and the TCP flags are present in the help output (verified via `opencode acp --help` on v1.17.13)."
  - command: opencode
    args: ["acp", "--pure"]
    transport: stdio
    adapter: none
    notes: "Pure variant skips external plugins (recommended for ACP clients that want to avoid third-party plugin contamination of the protocol stream). Verified to still negotiate `protocolVersion: 1` and the same capability set."
protocol_versions:
  - "ACP protocolVersion 1 (negotiated via initialize; matches schema v1.1.0 wire surface observed in OpenCode's @agentclientprotocol/sdk types)"
capabilities:
  - capability: initialize
    support: supported
    notes: "`initialize` returns `protocolVersion: 1`, `agentCapabilities` (loadSession, mcpCapabilities.http, mcpCapabilities.sse, promptCapabilities.embeddedContext, promptCapabilities.image, sessionCapabilities.{close,fork,list,resume}), `authMethods` (`opencode-login`), and `agentInfo` ({name: 'OpenCode', version: <InstallationVersion>}). Verified via local probe on v1.17.13."
  - capability: authenticate
    support: supported
    notes: "`authenticate` request with `methodId: 'opencode-login'` returns `{}`. Verified via local probe. Other method IDs return `UnknownAuthMethodError` → `-32602`."
  - capability: session_new
    support: supported
    notes: "`session/new` returns `{sessionId, configOptions[]}`. Immediately after the response, the agent emits a `session/update` notification with `available_commands_update` listing all slash commands and skills for the cwd. Verified via local probe."
  - capability: session_load
    support: supported
    notes: "`session/load` requires `{sessionId, cwd, mcpServers: []}` and returns `{configOptions}`. Same `available_commands_update` notification follows. Verified via local probe."
  - capability: session_prompt
    support: supported
    notes: "`session/prompt` is the primary turn-taking request. Returns `{stopReason, usage?, userMessageId?, _meta}`. `stopReason` is one of `end_turn`, `cancelled` (MessageAbortedError), `max_tokens` (MessageOutputLengthError), `refusal` (ContentFilterError), or `ServiceFailureError` (provider-side error). Provider auth errors raise `AuthRequiredError` → JSON-RPC `-32603`."
  - capability: session_cancel
    support: supported
    notes: "`session/cancel` is a NOTIFICATION (no `id`). Sending it as a request returns `-32601 Method not found`, which is the correct ACP behaviour. Handler calls `session.abort` on the underlying OpenCode session."
  - capability: session_modes
    support: supported
    notes: "`session/set_mode` and `session/set_config_option` are implemented. Modes are derived from non-hidden, non-subagent agents in the config (the `build` agent is the default primary). `set_config_option` supports `configId` values of `model`, `effort`, and `mode`."
  - capability: streaming
    support: supported
    notes: "Streaming flows through the single `session/update` notification. Observed `sessionUpdate` discriminators: `agent_message_chunk`, `agent_thought_chunk`, `user_message_chunk`, `tool_call`, `tool_call_update`, `available_commands_update`, `usage_update`. There is no separate `current_mode_update` or `plan` notification emitted by OpenCode."
  - capability: permissions
    support: supported
    notes: "Implemented via the `session/request_permission` reverse request. Handler emits three options: `allow_once` (optionId `once`), `allow_always` (optionId `always`), `reject_once` (optionId `reject`). Rejection or missing client handler auto-rejects."
  - capability: fs_read
    support: unsupported
    notes: "OpenCode does NOT issue `fs/read_text_file` reverse requests. All file reads happen in-process via the agent's own tool implementations. Probed: `fs/read_text_file` returned `-32601 Method not found`."
  - capability: fs_write
    support: partial
    notes: "OpenCode advertises `fs.writeTextFile` as a capability hint by using the connection's `writeTextFile` when present (it's typed in the ACP service as `Partial<Pick<AgentSideConnection, \"requestPermission\" | \"writeTextFile\">>`). It is used specifically when an `edit` tool call is approved with `allow_always`, to apply the proposed diff client-side via `apply_patch`. There is no generic `fs/write_text_file` reverse request and probing it returns `-32601`."
  - capability: terminal
    support: unsupported
    notes: "No `terminal/create` reverse requests. Shell commands execute in-process through the `bash` tool. Probed: `terminal/create` returns `-32601 Method not found`."
  - capability: mcp
    support: supported
    notes: "Client supplies `mcpServers: McpServer[]` at `session/new` / `session/load` / `session/resume`. Each entry is registered with the backing OpenCode server via `mcp.add`. Both `http` and `sse` MCP transports are advertised as supported."
  - capability: media
    support: partial
    notes: "Image content blocks (`{type:'image', mimeType, data, uri?}`) are accepted in prompts and reconstructed into OpenCode file parts (base64 data URLs or http(s) URLs). `promptCapabilities.audio` is NOT advertised. Plan mode update notifications are not emitted (ACP `Plan` variant not used)."
  - capability: plans
    support: unsupported
    notes: "OpenCode does not emit ACP `Plan` session updates; planning is internal to the agent (the TUI's own plan UI is not surfaced)."
  - capability: extensions
    support: supported
    notes: "OpenCode uses `_meta` for the `terminal-auth` capability negotiation during `initialize` (advertises a `command: 'opencode', args: ['auth','login']` if the client signals `_meta['terminal-auth']: true`). It does NOT implement `ext_method` or `ext_notification` — those pass through to the underlying SDK's unimplemented-method error."
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: "Required. OpenCode emits this on every permission-gated event (bash, edit, write, webfetch, external_directory, etc.). The client must respond with `{outcome: {outcome: 'selected', optionId: 'once'|'always'|'reject'}}` or `{outcome: {outcome: 'cancelled'}}`. Without a handler OpenCode rejects the permission and aborts the run."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: "Optional best-effort. OpenCode calls this only after `allow_always` on an `edit` permission, to apply the proposed diff via the client. If the client does not implement it, the edit still proceeds through OpenCode's own tool layer."
  - method: session/request_permission (allow_always writeProposedEdit)
    purpose: fs_write
    client_must_handle: false
    notes: "OpenCode's `permission.ts` calls `connection.writeTextFile({sessionId, path, content})` after a successful `allow_always` decision on an `edit` tool call, applying the diff client-side using the `diff` crate's `applyPatch`. This is the ONLY `fs/*` reverse request OpenCode will emit, and it requires the client to have advertised `fs.writeTextFile: true` at initialize. Without that, the write path falls back to OpenCode's local write tool."
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: "Not emitted. Probed and returned `-32601`."
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: "Not emitted. Probed and returned `-32601`."
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: "Not emitted. Probed and returned `-32601`."
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: "Not emitted."
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: "Not emitted."
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: false
    notes: "Not emitted."
permission_model:
  mechanism: "session/request_permission reverse request (in-process ACL plus client-side approval)"
  timeout: "no per-request timeout enforced by OpenCode; the prompt blocks the run-loop until a response arrives"
  default_policy: "no implicit default; every gated event requires Selected or Cancelled"
  approval_values:
    - "once (allow_once)"
    - "always (allow_always)"
    - "reject (reject_once)"
  notes: "OpenCode runs its own in-process ACL (per-agent permission rules: `build` is permissive, `plan` denies edits and prompts for bash). The ACP layer defers to it: when the ACL asks for approval, OpenCode emits a `session/request_permission` request to the client. The client never sees the raw ACL; it sees only the request and decides allow_once / allow_always / reject. On `session/cancel` the in-flight permission request is resolved with `Cancelled`. Permission kinds surface as `toolName` (e.g. `edit`, `bash`, `webfetch`, `external_directory`, `read`, `write`) — there is no separate `kind` field; clients should group by tool name or surface the suggested `title`."
filesystem_model:
  read_methods: []
  write_methods:
    - "fs/write_text_file (only on allow_always for edit, optional)"
  path_base: "absolute paths emitted by OpenCode (resolved through `path.resolve(cwd, value)` and `isAbsolute()` checks)"
  sandboxing: "client-side only; OpenCode will not sandbox itself — it operates on whatever paths the model chooses, gated by the ACL and the ACP permission loop"
  notes: "OpenCode does NOT delegate generic reads or writes to the client. File reads use OpenCode's own `read` tool; writes use the `write` and `edit` (apply_patch) tools. The only client-side filesystem request is the post-permission writeTextFile call described above. Clients that want to enforce sandboxing should reject `allow_always` writes and choose `reject` or `once` with their own validation in between."
terminal_model:
  supported: false
  methods: []
  shell: "in-process (Node.js `child_process.spawn` via the bash tool); not visible to the client"
  cwd: "absolute path supplied to the bash tool"
  streaming: "OpenCode streams `tool_call_update` notifications with `kind: 'execute'` while the shell runs; the full output appears in the final `tool_call_update` with `status: 'completed'`. There is no ACP `terminal/output` polling."
  cancellation: "`session/cancel` aborts the backing OpenCode session, which kills the running shell process"
  notes: "OpenCode's `bash` tool runs commands in-process. There is no client-side terminal lifecycle to manage. Clients should not advertise `terminal: true` expecting it to be honoured."
streaming_model:
  update_methods:
    - "session/update"
  text_events:
    - "agent_message_chunk (text deltas from assistant)"
    - "agent_thought_chunk (reasoning deltas when the model emits them)"
    - "user_message_chunk (replay only — sent when loading or forking a session to redeliver prior user text)"
  tool_events:
    - "tool_call (status: 'pending', emitted once per tool before execution)"
    - "tool_call_update (status: 'in_progress' | 'completed' | 'failed'; running snapshot includes stdout tail for `bash`; completed includes content/diff/image attachments; failed includes error message)"
  plan_events: []
  error_events:
    - "session/prompt error response with code -32603 / -32602 (provider auth, invalid config, model not found, mode not found, effort not found)"
    - "session/request_permission `cancelled` outcome (when the client returns Cancelled, the run terminates with stopReason=cancelled)"
  notes: "Single `session/update` notification carries all updates, discriminated by `sessionUpdate` (string). Verified variants on the installed binary: `available_commands_update` (after every session/new or session/load), `agent_message_chunk`, `agent_thought_chunk`, `user_message_chunk`, `tool_call`, `tool_call_update`, `usage_update` (one per prompt turn, carrying inputTokens / cacheRead / size / cost). `current_mode_update` and `plan` are not emitted. Updates have NO `id` (they are JSON-RPC notifications, not requests)."
auth_setup:
  required: true
  mechanisms:
    - "`opencode auth login` interactive OAuth/device flow (advertised as `authMethods[0]` at initialize)"
    - "Per-provider API keys via `opencode auth login` for any provider that supports key auth"
    - "Pre-authenticated credential store at `~/.local/share/opencode/auth.json` (or platform equivalent)"
    - "Plugin-based auth (e.g. `opencode-openai-codex-auth`) for Codex-style OAuth"
    - "Bedrock, Vertex, Foundry via dedicated env vars"
  headless_notes: "For headless ACP launches set the provider's env var (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `ZAI_API_KEY`) before starting `opencode acp`. The `--pure` flag is recommended to suppress plugin auto-load that may otherwise need a TTY. `authMethods[0]._meta['terminal-auth']` is emitted only if the client signals `_meta['terminal-auth']: true` at initialize — at that point the client may launch `opencode auth login` in a terminal it controls."
  notes: "`authenticate` request with `methodId: 'opencode-login'` returns `{}` immediately (the OAuth flow itself runs outside the ACP session via `opencode auth login`). Probed via local probe. Issue #34638 documents that `OPENCODE_CONFIG_CONTENT` and `OPENCODE_CONFIG_DIR` are ignored when launching `opencode acp` — the ACP process loads config from the user/launch directories only."
env_vars:
  - name: OPENCODE_CLIENT
    effect: "Set to `acp` by `opencode acp` to flag the in-process client as ACP (the agent and bus inspect this to suppress TUI-specific behaviour)."
  - name: OPENCODE_CONFIG_CONTENT
    effect: "Override config JSON; ignored by `opencode acp` (issue #34638 — the ACP handler does not propagate this override to the running instance)."
  - name: OPENCODE_CONFIG_DIR
    effect: "Override config directory; ignored by `opencode acp` (same issue)."
  - name: OPENCODE_LOG_LEVEL
    effect: "Log level for the OpenCode instance launched by `opencode acp`. Distinct from `--log-level`, which routes to stderr."
  - name: OPENCODE_SERVER_PASSWORD
    effect: "Basic auth password when `--port` is set; ACP clients on TCP should set this for non-loopback binds."
  - name: OPENCODE_SERVER_USERNAME
    effect: "Basic auth username (defaults to `opencode`); pairs with OPENCODE_SERVER_PASSWORD."
  - name: OPENCODE_INSTALL_DIR
    effect: "Custom install path used by the install script (not consumed by ACP runtime)."
  - name: XDG_BIN_DIR
    effect: "Custom bin directory used by the install script."
  - name: Provider-specific API keys (e.g. ANTHROPIC_API_KEY, OPENAI_API_KEY, GEMINI_API_KEY, ZAI_API_KEY)
    effect: "Authenticate the chosen provider without the interactive OAuth flow. Required for headless ACP usage."
  - name: NO_COLOR / FORCE_COLOR
    effect: "Inherited by OpenCode's terminal renderer; the ACP wire stream is not affected, but stderr logs honour these."
  - name: GIT_TERMINAL_PROMPT
    effect: "Standard git var; relevant when an OpenCode tool invokes git which prompts for credentials."
rust_client:
  crate: agent-client-protocol
  connection_type: "AcpAgent subprocess (`AcpAgent::from_str('opencode acp')` or `AcpAgent::from_str('opencode acp --pure')`) over stdio JSON-RPC"
  localset_required: false
  reverse_request_handlers:
    - "session/request_permission"
    - "fs/write_text_file (optional; only required if the client wants to render OpenCode's allow_always edit diffs client-side)"
  desktop_streaming_pattern: "tokio::sync::mpsc from the notification handler to the UI thread; run the ACP client on a dedicated tokio runtime; map session/update.discriminant to AgentEvent variants"
  notes: "`agent-client-protocol` 1.0.1 (schema 1.1.0) is the current Rust SDK. OpenCode negotiates `protocolVersion: 1`, so any 1.x release is compatible. AcpAgent::from_str supports the `opencode acp` invocation directly without a custom command. There is no preset for OpenCode (unlike Zed Codex / Claude) — clients should use `from_str` or a custom builder."
compatibility:
  - client: Zed
    status: works
    issue: "Zed is the canonical reference client for ACP; OpenCode's docs ship a Zed snippet (`{agent_servers: {'OpenCode': {command: 'opencode', args: ['acp']}}}`)."
    workaround: "No workaround needed."
  - client: JetBrains IDEs (Junie / AI Assistant)
    status: works
    issue: "OpenCode's docs ship a JetBrains snippet (`~/.jetbrains/acp.json`). Note: tool-name comparison breaks in Turkish/Azerbaijani locales due to `toLocaleLowerCase` (issue #35096) — Junie's UI may show empty permission prompts on such locales."
    workaround: "Run JetBrains in en-US locale, or wait for the opencode fix."
  - client: Avante.nvim
    status: works
    issue: "Docs include an Avante snippet; no known issues."
    workaround: "None."
  - client: CodeCompanion.nvim
    status: works
    issue: "Docs include a CodeCompanion snippet; no known issues."
    workaround: "None."
  - client: Xcode 27 beta 2
    status: partial
    issue: "Issue #34743 reports Xcode 27 beta 2 launching `opencode acp` ignores `opencode.json` and the model picked in the TUI, falling back to the default `opencode/big-pickle`. Root cause: Xcode's ACP launch path may not propagate the configured model."
    workaround: "Use the TUI to start the session, or set `OPENCODE_CONFIG_CONTENT` with the desired model (broken by #34638, so currently no clean workaround)."
  - client: Custom Rust clients (agent-client-protocol 1.0.x)
    status: works
    issue: "None known. The OpenCode wire surface is a subset of schema 1.1.0 so any 1.0.x SDK release works."
    workaround: "None."
  - client: Subagent-style ACP drivers (sync subagents)
    status: partial
    issue: "Issue #35073 reports that when a subagent path issues permission requests through ACP, the request can hang indefinitely because sync subagents are treated as interactive."
    workaround: "Avoid sync subagents that prompt; route them through the plan agent (`session/set_mode` to `plan`) or set `agent: build` explicitly."
quirks:
  - "`OPENCODE_CONFIG_CONTENT` and `OPENCODE_CONFIG_DIR` are ignored by `opencode acp` (issue #34638). The ACP process loads config from the on-disk `~/.config/opencode/opencode.json` and launch-directory `opencode.json` only. Tooling that wants a per-run proxy or custom provider must modify the on-disk config or use the agent's own provider override."
  - "Tool-name comparison uses `toLocaleLowerCase()` (permission.ts, tool.ts). In Turkish and Azerbaijani locales the dotted/dotless I capitalisation collides — `toLocaleLowerCase('EDIT')` produces `'edıt'` instead of `'edit'`, breaking tool-name matches. Issue #35096 (open)."
  - "Permission prompts in older versions surfaced the tool name instead of the actual command (issue #33949 — closed). After the fix, edit prompts include a `diff` content block; bash prompts include the command in the `title`. Clients should prefer the rendered content over the tool name."
  - "`session/unstable_forkSession` and `session/unstable_set_session_model` are referenced in OpenCode's TypeScript types (`agent.ts`) but are NOT exposed as JSON-RPC methods on the v1.1.0 wire surface — probing them returns `-32601 Method not found`. Clients should use `session/load` for fork-style behaviour and rely on `session/set_config_option` for model changes."
  - "`logout` and `session/delete` are NOT implemented (probed — both return `-32601`). `session/cancel` is a NOTIFICATION, not a request — sending it as a request also returns `-32601`."
  - "`notifications/initialized` is rejected as `Method not found`. OpenCode treats the handshake as complete after `initialize` returns — clients do not need to send any post-handshake notification."
  - "`session/resume` returns `-32603 Internal error: OpenCode service failure` when given an unknown `sessionId`. The method exists but is brittle to invalid input."
  - "Slash commands `/undo` and `/redo` are explicitly unsupported over ACP per the OpenCode docs (`opencode.ai/docs/acp/`). All other slash commands and skills are surfaced via the `available_commands_update` notification."
  - "The `bash` tool streams snapshots rather than full output; OpenCode's `tool_call_update` with `status: 'in_progress'` repeats the same `output` until it changes (the dedup uses `shellSnapshots`). Clients should treat repeated identical `output` as a heartbeat, not a new event."
  - "`session/set_config_option` validates against the current snapshot's model and mode list. The three configIds are `model` (`providerID/modelID`, optionally `/variant`), `effort` (variant name), and `mode` (agent name). Unknown values raise `InvalidConfigOptionError`, `InvalidModelError`, `InvalidEffortError`, or `InvalidModeError` — all surface as JSON-RPC `-32602`."
  - "Empty `prompt` arrays are accepted but produce a degenerate prompt that returns `{stopReason: 'end_turn'}` without any model call."
  - "ACP's `terminal-auth` extension is supported: if the client signals `_meta['terminal-auth']: true` at initialize, OpenCode attaches `{command: 'opencode', args: ['auth', 'login'], label: 'OpenCode Login'}` to the advertised `authMethods[0]._meta`."
gaps:
  - "No public v2 schema conformance test. OpenCode's source imports `@agentclientprotocol/sdk` and exposes the v1.1.0 surface; whether it implements any v2 (unstable) features is `unknown` without a probe on a build with v2 enabled."
  - "No documented protocol for terminal output streaming beyond the in-progress `tool_call_update` snapshots. Clients that need full live shell output cannot get it without running the bash command client-side themselves."
  - "Reverse-request `fs/write_text_file` is documented in the TypeScript types but the wire behaviour for non-`edit` / non-`allow_always` paths is not exercised by tests visible in the source."
  - "The `usage_update` notification includes `cost: {amount, currency: 'USD'}` but OpenCode's cost values come from configured per-model pricing — there is no enforcement of the `currency` field or the unit (dollars vs cents). Clients should not assume `USD` literally; treat it as the configured currency."
  - "Behaviour when the underlying OpenCode server fails to start (port already in use) is not documented; observed locally: stderr logs `disposing instance` and the process exits without an ACP error response, leaving the client to detect EOF on stdout."
changes:
  - "Initial research — fresh document. No prior version exists in this directory under that name."
recent_changes:
  - date: 2026-07-03
    version: "v1.17.13"
    change: "ACP mode is stable; `session/load`, `session/resume`, `session/close`, `session/set_mode`, `session/set_config_option`, and `session/list` all return success. Latest release on the `dev` branch."
    impact: "No client impact — wire surface unchanged from the prior research window."
  - date: 2026-07-03
    version: "v1.17.13 (issue #35096)"
    change: "ACP tool-name comparison broken in Turkish/Azerbaijani locales due to `toLocaleLowerCase`."
    impact: "JetBrains and any other client running in those locales may show blank permission prompts; mitigation is to run in en-US."
  - date: 2026-07-03
    version: "v1.17.13 (issue #35073)"
    change: "Subagent permission asks hang indefinitely (sync subagents treated as interactive)."
    impact: "Workaround: avoid sync subagents that prompt; use the plan agent or set `agent: build`."
  - date: 2026-07-01
    version: "v1.17.x (issue #34743)"
    change: "opencode ACP from Xcode 27 beta 2 uses default model `big-pickle`, ignoring opencode.json or model selected in TUI."
    impact: "No clean workaround; reported but unresolved."
  - date: 2026-06-30
    version: "v1.17.x (issue #34638)"
    change: "`opencode acp` mode ignores `OPENCODE_CONFIG_CONTENT` and `OPENCODE_CONFIG_DIR`."
    impact: "Blocks per-run proxy / custom provider injection (the Charon platform is the reported blocker). Reported but unresolved."
  - date: 2026-06-30
    version: "v1.17.x (issue #34551)"
    change: "Add reasoning effort/level selector in JetBrains AI Assistant via ACP."
    impact: "Closed/completed. The `effort` configOption is now negotiated."
  - date: 2026-06-27
    version: "v1.17.x (issue #34193)"
    change: "ACP session/fork support status."
    impact: "Closed/completed. `session/load` and `session/resume` are the fork-equivalent on the v1 wire surface."
  - date: 2026-06-26
    version: "v1.17.x (issue #33949)"
    change: "ACP permission prompts show tool name instead of the command."
    impact: "Closed/completed. Edit prompts now include a diff content block; bash prompts show the command in `title`."
  - date: 2026-06-29
    version: "agent-client-protocol 1.0.1 + schema 1.1.0"
    change: "ACP Rust SDK reached 1.0.x; connection types are Send/Sync; schema 1.1.0 is the current stable wire protocol."
    impact: "OpenCode negotiates `protocolVersion: 1` against any 1.0.x SDK release; no special handling needed."
requires_claudine_update: false
reason: "OpenCode is one of the eight providers already wired through the lifecycle/stream pipeline, so the existing Claudine integration does not need to change to add ACP support — adding ACP would be an *additional* integration surface, not a replacement. The launch command is `opencode acp` (no adapter bridge), which means Claudine can spawn the process directly. Capability negotiation will need to model the native ACP surface (capabilities for `available_commands_update`, `agent_message_chunk`, `tool_call`, etc.) but those are additive to the existing event model. The only future work that would *require* code changes is if Claudine decides to surface a Claude-style `claude-code-acp-rs`-style bridge instead of the native binary, or to add ACP-specific permission routing for the `allow_always` -> `fs/write_text_file` reverse request that OpenCode emits but no other native provider does — neither of which is on the immediate roadmap. The Rust SDK already supports `AcpAgent::from_str('opencode acp')` so the client integration would be a small isolated add."
---

# OpenCode ACP

OpenCode is the open-source coding agent from Anomaly (formerly SST). As of the installed binary **v1.17.13** (released 2026-07-01) OpenCode ships **native ACP support** inside its primary CLI binary — the `opencode acp` subcommand starts an in-process ACP server that speaks JSON-RPC 2.0 over stdio. There is no separate adapter package; ACP is the same TUI/server codebase running a different transport.

This document is the independent deep dive on that native support, intended as input for future Claudine ACP client/adapter work. It covers launch modes, capability negotiation, the (small) reverse-request surface, streaming, auth, and the Rust client integration path.

## Overview

**Support: native** — `opencode acp` is the primary CLI binary speaking ACP directly. There is no bridge process translating from a proprietary wire protocol.

Verified on 2026-07-03 against the installed binary at `~/.opencode/bin/opencode` v1.17.13:

```text
$ opencode acp --help
opencode acp

start ACP (Agent Client Protocol) server

Options:
  -h, --help         show help
  -v, --version      show version number
      --print-logs   print logs to stderr
      --log-level    log level (DEBUG, INFO, WARN, ERROR)
      --pure         run without external plugins
      --port         port to listen on (default 0)
      --hostname     hostname to listen on (default 127.0.0.1)
      --mdns         enable mDNS service discovery (defaults hostname to 0.0.0.0)
      --mdns-domain  custom domain name for mDNS service (default: opencode.local)
      --cors         additional domains to allow for CORS
      --cwd          working directory
```

And the live `initialize` handshake (captured locally on 2026-07-03 with stdin/stdout piped directly into `opencode acp`):

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": 1,
    "agentCapabilities": {
      "loadSession": true,
      "mcpCapabilities": { "http": true, "sse": true },
      "promptCapabilities": { "embeddedContext": true, "image": true },
      "sessionCapabilities": { "close": {}, "fork": {}, "list": {}, "resume": {} }
    },
    "authMethods": [
      {
        "description": "Run `opencode auth login` in the terminal",
        "name": "Login with opencode",
        "id": "opencode-login"
      }
    ],
    "agentInfo": { "name": "OpenCode", "version": "1.17.13" }
  }
}
```

The TypeScript source implementing all of this lives at [`packages/opencode/src/acp/`](https://github.com/anomalyco/opencode/tree/dev/packages/opencode/src/acp) and the CLI entrypoint is at [`packages/opencode/src/cli/cmd/acp.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/cli/cmd/acp.ts). The implementation imports `@agentclientprotocol/sdk` (the official TypeScript SDK) and an Effect-based internal service layer.

## Launching ACP

### stdio (canonical)

```bash
opencode acp [--pure] [--cwd <path>] [--log-level DEBUG|INFO|WARN|ERROR] [--print-logs]
```

The process owns the JSON-RPC connection on stdin (requests from client → agent) and stdout (responses and notifications from agent → client). stderr is reserved for log lines emitted via `--print-logs` or the configured `--log-level` and never carries JSON-RPC traffic. Verified via local probe.

`--pure` is recommended for any client that wants a deterministic plugin surface; it suppresses external plugin auto-load. `--cwd` overrides the agent's working directory without changing the client's CWD.

### TCP / mDNS

The same `opencode acp` subcommand accepts `--port`, `--hostname`, `--mdns`, and `--mdns-domain` flags (verified via help output). These exist for clients that prefer a TCP-shaped transport and rely on mDNS service discovery under `opencode.local`. The wire protocol on TCP is the same JSON-RPC 2.0 over newline-delimited JSON. **stdio is the canonical transport and is what every editor integration (Zed, JetBrains, Avante, CodeCompanion) uses in production.**

### Distinguishing from `opencode serve`

`opencode serve` is a *separate* subcommand that exposes OpenCode's OpenAPI 3.1 HTTP server (for the JavaScript SDK and HTTP clients). It does NOT speak ACP. Confirmed via `opencode serve --help`. Mixing the two is a common confusion; clients that want ACP must spawn `opencode acp`, not `opencode serve`.

## Protocol and Capabilities

### Transport and framing

- **Transport**: stdio pipes (TCP available via `--port` but not in production use).
- **Framing**: newline-delimited JSON-RPC 2.0 (`@agentclientprotocol/sdk`'s `ndJsonStream`).
- **Encoding**: UTF-8.
- **Direction**: client → agent (requests and notifications), agent → client (responses, notifications, and the small set of reverse requests).

### Protocol version

OpenCode negotiates `protocolVersion: 1` on every `initialize` (verified). This corresponds to the ACP schema at [`schema/v1/schema.json`](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/schema/v1/schema.json) (latest schema release: `v1.17.0`, dated 2026-06-29). The TypeScript SDK types OpenCode imports are from `@agentclientprotocol/sdk` v1.x; the wire surface is stable.

### Capability surface (verified)

| Area | Status | Evidence |
|------|--------|----------|
| `initialize` | supported | Live probe returned the full `agentCapabilities` payload above. |
| `authenticate` | supported | `authenticate` with `methodId: 'opencode-login'` returns `{}`. |
| `session/new` | supported | Returns `{sessionId, configOptions[]}`; immediately emits `available_commands_update`. |
| `session/load` | supported | Returns `{configOptions}`; emits `available_commands_update` and replays messages. |
| `session/list` | supported | Returns `{sessions: SessionInfo[]}`. |
| `session/resume` | supported | Returns `{configOptions}`; errors as `-32603` on unknown sessionId. |
| `session/close` | supported | Returns `{}`. |
| `session/set_mode` | supported | Validates against `snapshot.availableModes`. |
| `session/set_config_option` | supported | Three configIds: `model`, `effort`, `mode`. |
| `session/prompt` | supported | Returns `{stopReason, usage?, userMessageId?, _meta}`. |
| `session/cancel` (notification) | supported | Aborts the backing OpenCode session. |
| `session/update` (streaming) | supported | `agent_message_chunk`, `agent_thought_chunk`, `user_message_chunk`, `tool_call`, `tool_call_update`, `available_commands_update`, `usage_update`. |
| `session/request_permission` (reverse) | supported | Three options: `once` / `always` / `reject`. |
| `fs/write_text_file` (reverse) | partial | Only after `allow_always` on an `edit` permission; not a generic reverse-request surface. |
| `fs/read_text_file` (reverse) | unsupported | Probed → `-32601 Method not found`. |
| `terminal/*` (reverse) | unsupported | Probed → `-32601 Method not found`. |
| `logout` | unsupported | Probed → `-32601 Method not found`. |
| `session/delete` | unsupported | Probed → `-32601 Method not found`. |
| Plan updates | unsupported | No `Plan` session update is emitted. |
| `Plan` session update | unsupported | ACP `Plan` notification is not emitted. |
| MCP servers via `mcpServers: []` | supported | Registered via the backing OpenCode `mcp.add` API. |
| Image content | supported | Reconstructed from `data:` or `http(s):` URIs in `prompt[]`. |
| Audio content | unsupported | `promptCapabilities.audio` is not advertised. |
| `ext_method` / `ext_notification` | unsupported | Pass-through; the SDK returns the unimplemented-method error. |
| `_meta['terminal-auth']` | supported | Causes the `authMethods[0]._meta` to expose `{command, args, label}` for the client's terminal-launched login. |

## Reverse Requests

OpenCode's reverse-request surface is intentionally small — it runs tools in-process and only delegates to the client for permission and the optional post-approval edit write.

### `session/request_permission` (required)

Emitted on every permission-gated event. The handler in [`packages/opencode/src/acp/permission.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/permission.ts) presents three options:

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "session/request_permission",
  "params": {
    "sessionId": "ses_abc123",
    "toolCall": {
      "toolCallId": "call_xyz",
      "kind": "edit",
      "title": "src/auth.rs",
      "status": "pending",
      "locations": [{ "path": "/Users/ken/project/src/auth.rs" }],
      "content": [
        {
          "type": "diff",
          "path": "/Users/ken/project/src/auth.rs",
          "oldText": "fn old() {}",
          "newText": "fn new() {}\n"
        }
      ]
    },
    "options": [
      { "optionId": "once",   "name": "Allow once",     "kind": "allow_once" },
      { "optionId": "always", "name": "Always allow",   "kind": "allow_always" },
      { "optionId": "reject", "name": "Reject",         "kind": "reject_once" }
    ]
  }
}
```

Respond with:

```json
{ "outcome": { "outcome": "selected", "optionId": "once" } }
```

or

```json
{ "outcome": { "outcome": "cancelled" } }
```

If the client does not implement `requestPermission`, OpenCode auto-rejects (writes `reject` to the permission reply API) and aborts the run.

### `fs/write_text_file` (optional best-effort)

After an `allow_always` decision on an `edit` permission, OpenCode's `permission.ts` calls `connection.writeTextFile({sessionId, path, content})` to apply the proposed diff via the `diff` crate's `applyPatch`. This is the only `fs/*` reverse request OpenCode emits. Clients that want to render the diff but apply edits locally can implement this; clients that want OpenCode to apply edits should not implement it (the write falls back to OpenCode's local `write`/`edit` tool layer).

```json
{
  "jsonrpc": "2.0",
  "id": 43,
  "method": "fs/write_text_file",
  "params": {
    "sessionId": "ses_abc123",
    "path": "/Users/ken/project/src/auth.rs",
    "content": "fn new() {}\n"
  }
}
```

### What OpenCode does NOT request

- **No `fs/read_text_file`** — reads are in-process via the `read` tool.
- **No `terminal/create` / `terminal/output` / `terminal/wait_for_exit` / `terminal/kill` / `terminal/release`** — bash commands execute in-process via `child_process.spawn`.

## Permissions, Filesystem, and Terminal

### Permission policy

OpenCode runs its own in-process ACL before the ACP layer is reached. The ACL is per-agent:

- `build` — full access by default, gates write/edit/bash through ACP `session/request_permission` based on the user's global/agent permission rules.
- `plan` — denies edits by default; gates bash through the permission loop.

When the ACL decides a permission is needed, OpenCode fires `session/request_permission` to the client. The client is the final authority on `once` vs `always` vs `reject`. On `session/cancel`, in-flight permission requests are resolved with `Cancelled` and the run aborts with `stopReason: "cancelled"`.

### Filesystem policy

OpenCode owns filesystem access. It resolves paths with `path.resolve(cwd, value)` and validates absoluteness with `path.isAbsolute`. All reads/writes happen in-process; the client has no opportunity to sandbox at the filesystem boundary unless it implements the optional `fs/write_text_file` handler and uses it to gate or rewrite the path.

**Practical recommendation**: implement the permission handler strictly (default to `reject` for unknown tools, `once` for known tools, never blanket `always`) and skip `fs/write_text_file` to keep OpenCode in charge of file operations.

### Terminal policy

OpenCode runs shell commands via the `bash` tool. Output is streamed as `tool_call_update` notifications with `kind: 'execute'`:

- `pending` — emitted once when the bash tool starts.
- `in_progress` — emitted repeatedly with the same `output` until the snapshot changes (clients should treat repeated identical output as a heartbeat).
- `completed` — final output and exit code in `content[]` and `rawOutput`.
- `failed` — error message in `content[0].content.text`.

Clients do NOT manage a terminal lifecycle. `session/cancel` kills the running shell via `session.abort` on the backing OpenCode session.

## Streaming and UI Integration

Streaming flows through a single `session/update` notification with a `sessionUpdate` discriminator (string). Verified discriminators on the installed binary:

| `sessionUpdate` | Emitted when | Key fields |
|-----------------|--------------|------------|
| `agent_message_chunk` | Assistant text delta | `messageId`, `content: {type: 'text', text}` |
| `agent_thought_chunk` | Reasoning delta (when the model emits one) | `messageId`, `content: {type: 'text', text}` |
| `user_message_chunk` | Replay only (after `session/load` or `session/resume`) | `messageId`, `content: {type: 'text', text}` |
| `tool_call` | Tool starts | `toolCallId`, `title`, `kind`, `status: 'pending'`, `locations[]`, `rawInput` |
| `tool_call_update` | Tool progresses | `toolCallId`, `status`, `kind?`, `title?`, `content?`, `locations?`, `rawInput?`, `rawOutput?` |
| `available_commands_update` | After every `session/new`, `session/load`, `session/resume` | `availableCommands: [{name, description}]` |
| `usage_update` | After every `session/prompt` completes | `used`, `size`, `cost: {amount, currency: 'USD'}` |

Group updates by `messageId` to disambiguate parallel streams. Treat `tool_call_update` with the same `output` repeated as a heartbeat. There is no separate `current_mode_update` or `plan` notification — OpenCode does not emit them.

### `stopReason` values from `session/prompt`

```text
end_turn     — normal completion
cancelled    — MessageAbortedError (session/cancel)
max_tokens   — MessageOutputLengthError
refusal      — ContentFilterError
-service    — SessionFailureError (provider-side failure; -32603)
auth_required — ProviderAuthError (AuthRequiredError; client should run authenticate or surface to user)
```

## Authentication and Setup

### Headless auth preconditions

OpenCode does NOT auto-prompt when launched as `opencode acp`. Before the first prompt, at least one provider must be authenticated. The two viable paths:

1. **Pre-authenticate via the TUI**: run `opencode` once interactively, complete `opencode auth login`, exit, then `opencode acp` inherits the credentials from `~/.local/share/opencode/auth.json` (or platform equivalent).
2. **Set a provider API key** in the environment before launching:
   - `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `ZAI_API_KEY`, etc.
   - `OPENCODE_CONFIG_CONTENT` does **NOT** propagate via ACP (issue #34638), so the env var is the cleanest path.

### `authenticate` request

`authenticate` with `methodId: 'opencode-login'` returns `{}` immediately — the OAuth flow itself runs outside the ACP session via `opencode auth login`. Clients that want a self-contained login flow should signal `_meta['terminal-auth']: true` at `initialize`; OpenCode then attaches `{command: 'opencode', args: ['auth', 'login'], label: 'OpenCode Login'}` to `authMethods[0]._meta`, which clients like Zed render as a "login in terminal" button.

### Provider plugins

OpenCode auto-loads plugin packages (e.g. `opencode-openai-codex-auth`). The `--pure` flag suppresses this. For ACP clients that want deterministic provider behaviour, `--pure` is recommended.

## Compatibility, Quirks, and Workarounds

1. **`OPENCODE_CONFIG_CONTENT` and `OPENCODE_CONFIG_DIR` are ignored** by `opencode acp` (issue #34638, open). The ACP process reads only the on-disk user/launch-directory config. Tooling that wants a per-run provider override must either pre-stage the config file or use environment variables for provider keys.
2. **Turkish/Azerbaijani locale bug** — tool-name comparison uses `toLocaleLowerCase()` (in `permission.ts` and `tool.ts`); the Turkish dotted-I capitalisation breaks the lookup. Issue #35096 (open, 2026-07-03). Workaround: run clients in `en-US`.
3. **Subagent permission hangs** — sync subagents that try to prompt hang indefinitely because they're treated as interactive. Issue #35073 (open, 2026-07-03). Workaround: avoid sync subagents that prompt, or set `agent: build` explicitly.
4. **Xcode 27 beta 2 default model bug** — Xcode's ACP launch path ignores the configured model and falls back to `opencode/big-pickle`. Issue #34743 (open, 2026-07-01). No clean workaround; reported but unresolved.
5. **`logout` and `session/delete` are not implemented** — both return `-32601 Method not found`. Clients that need to discard credentials or delete sessions must do so outside the ACP session.
6. **`session/cancel` is a notification** — sending it as a request returns `-32601`. The correct form is `{"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"..."}}` with no `id`.
7. **`notifications/initialized` is rejected** — OpenCode treats the handshake as complete after `initialize`. Clients do not need to send any post-handshake notification.
8. **`/undo` and `/redo` slash commands are unsupported** over ACP (per OpenCode docs). All other slash commands and skills surface via `available_commands_update`.
9. **`session/resume` on an unknown `sessionId`** returns `-32603 Internal error: OpenCode service failure` rather than `-32602 Invalid params`. Clients should defensively retry or surface the error gracefully.
10. **`session/set_config_option` with `configId: 'effort'`** requires the model to have `variants`; calling it for a model without variants raises `InvalidEffortError`. The configId accepts three values: `model` (`providerID/modelID`, optionally `/variant`), `effort` (variant name), and `mode` (agent name).
11. **Empty `prompt: []` is accepted** but produces a degenerate prompt that returns `end_turn` without a model call. Clients should validate non-empty prompts upstream.
12. **`usage_update` cost field** uses `currency: 'USD'` but the unit and currency are determined by the configured per-model pricing, not by the wire value. Clients should display the `amount` as a configured-currency value, not assume USD literal.

## Recent Changes

- **2026-07-03 (v1.17.13)**: Tool-name `toLocaleLowerCase` bug (issue #35096). Impact: permission prompts may render blank in Turkish/Azerbaijani locales.
- **2026-07-03 (v1.17.x)**: Subagent permission hangs (issue #35073). Impact: sync subagents that prompt block indefinitely.
- **2026-07-01 (v1.17.x)**: Xcode 27 beta 2 default model (issue #34743). Impact: Xcode clients land on `big-pickle` regardless of config.
- **2026-06-30 (v1.17.x)**: `OPENCODE_CONFIG_CONTENT` / `OPENCODE_CONFIG_DIR` ignored (issue #34638). Impact: per-run config overrides via env var broken.
- **2026-06-30 (v1.17.x)**: Reasoning-effort selector for JetBrains (issue #34551, closed/completed). Impact: `effort` configOption now surfaces in JetBrains.
- **2026-06-27 (v1.17.x)**: ACP `session/fork` status (issue #34193, closed/completed). Impact: `session/load` and `session/resume` are the fork-equivalent on v1.
- **2026-06-26 (v1.17.x)**: Permission prompts show tool name instead of command (issue #33949, closed/completed). Impact: edit prompts now include a diff content block.
- **2026-06-29 (schema)**: ACP schema release `v1.17.0`. Impact: OpenCode's `protocolVersion: 1` remains compatible.
- **2026-06-29 (SDK)**: `agent-client-protocol` 1.0.1 with `agent-client-protocol-schema =1.1.0`. Impact: Rust SDK reaches 1.0; connection types are `Send`/`Sync`.
- **2026-06-23 / 2026-06-25 (adapter)**: Various adapter-level fixes in the TypeScript SDK that OpenCode consumes (out of scope of this doc; covered in the claude.md sibling research).
- **Earlier 2026**: Native ACP mode landed in OpenCode. Prior releases used a separate transport.

## Rust Client Example

The simplest path uses `agent-client-protocol` 1.0.1 with `AcpAgent::from_str`:

```toml
[dependencies]
agent-client-protocol = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

```rust
use agent_client_protocol::schema::v1::{
    ClientCapabilities, ContentBlock, Implementation, InitializeRequest, NewSessionRequest,
    PromptRequest, SessionNotification, TextContent,
};
use agent_client_protocol::{AcpAgent, Client, ProtocolVersion};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Spawn `opencode acp` directly. --pure suppresses plugin auto-load for
    // deterministic provider behaviour. --log-level keeps stderr usable.
    let agent = AcpAgent::from_str("opencode acp --pure --log-level WARN")?;

    Client.builder()
        .name("claudine-opencode-client")
        .on_receive_notification(
            |notification: SessionNotification, _cx| async move {
                match notification.update {
                    SessionNotification::AgentMessageChunk(chunk) => {
                        if let ContentBlock::Text(t) = chunk.content {
                            print!("{}", t.text);
                        }
                    }
                    SessionNotification::ToolCall(tc) => {
                        eprintln!("\n[tool: {}]", tc.title);
                    }
                    SessionNotification::AvailableCommandsUpdate(update) => {
                        eprintln!("\n[{} commands]", update.available_commands.len());
                    }
                    _ => {}
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(agent, |connection| async move {
            let init = InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(ClientCapabilities::default())
                .client_info(Implementation {
                    name: "claudine".into(),
                    title: Some("Claudine".into()),
                    version: env!("CARGO_PKG_VERSION").into(),
                });

            let init_response = connection.send_request(init).block_task().await?;
            eprintln!("agent: {:?}", init_response.agent_info);
            eprintln!("auth methods: {:?}", init_response.auth_methods);

            let cwd = std::env::current_dir()?;
            let session = connection
                .send_request(NewSessionRequest::new(cwd, vec![]))
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

            eprintln!("\nstop_reason: {:?}", result.stop_reason);
            Ok(())
        })
        .await?;

    Ok(())
}
```

OpenCode negotiates `protocolVersion: 1` against the SDK's `ProtocolVersion::V1`. The `--pure` flag is a defensive default; remove it if you want plugin support.

## Rust Reverse Request Handling

The only required handler is `session/request_permission`. The optional `fs/write_text_file` is shown for completeness.

```rust
use agent_client_protocol::schema::v1::{
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, WriteTextFileRequest, WriteTextFileResponse,
};
use std::path::{Path, PathBuf};

fn sandbox(path: &Path, root: &Path) -> anyhow::Result<PathBuf> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !canonical.starts_with(root) {
        anyhow::bail!("path {} is outside project root", canonical.display());
    }
    Ok(canonical)
}

async fn handle_permission(
    request: RequestPermissionRequest,
) -> anyhow::Result<RequestPermissionResponse> {
    // Strict default: never blanket-allow. Default unknown tools to "once" so
    // the user is prompted again on repeat use; known read-only tools get
    // "always"; bash/edit/write default to "once".
    let kind = request
        .tool_call
        .kind
        .as_ref()
        .map(|k| format!("{:?}", k))
        .unwrap_or_default();

    let option_id = match kind.as_str() {
        "Read" | "Search" => "always",
        _ => "once",
    };

    Ok(RequestPermissionResponse::new(
        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id.to_string())),
    ))
}

async fn handle_write(
    request: WriteTextFileRequest,
    root: PathBuf,
) -> anyhow::Result<WriteTextFileResponse> {
    // Only emitted by OpenCode after `allow_always` on an edit. Apply the
    // write through the sandbox. Without this handler, OpenCode's local
    // write/edit tools handle the path instead.
    let path = sandbox(&request.path, &root)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, &request.content).await?;
    Ok(WriteTextFileResponse {})
}
```

Register handlers on the builder before `connect_with`:

```rust
Client.builder()
    .on_receive_request(
        |request: RequestPermissionRequest, responder, _cx| async move {
            responder.respond(handle_permission(request).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
    .on_receive_request(
        |request: WriteTextFileRequest, responder, _cx| async move {
            let root = std::env::current_dir()?;
            responder.respond(handle_write(request, root).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
```

If you do not register the permission handler, every gated event is rejected and the agent aborts. The `fs/write_text_file` handler is genuinely optional.

## Rust Host Command Handling

OpenCode does NOT delegate command execution. All shell work happens in-process via the `bash` tool and surfaces as `session/update` notifications with `kind: 'execute'`. There is no client-side terminal handler to implement.

For completeness, the reverse-request surface that *would* apply if a future OpenCode build adds `terminal/create`:

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
    child: Option<Child>,
    output: String,
    exit_code: Option<i32>,
}

#[derive(Clone, Default)]
struct TerminalManager {
    inner: Arc<Mutex<HashMap<TerminalId, TerminalHandle>>>,
}

async fn handle_create(
    request: CreateTerminalRequest,
    manager: TerminalManager,
    root: PathBuf,
) -> anyhow::Result<CreateTerminalResponse> {
    let cwd = request.cwd.unwrap_or(root);
    let child = tokio::process::Command::new(&request.command)
        .args(request.args)
        .envs(request.env.into_iter().map(|e| (e.name, e.value)))
        .current_dir(&cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let id: TerminalId = format!("term_{}", uuid::Uuid::new_v4()).into();
    manager.inner.lock().await.insert(
        id.clone(),
        TerminalHandle { child: Some(child), output: String::new(), exit_code: None },
    );
    Ok(CreateTerminalResponse::new(id))
}

async fn handle_release(
    request: ReleaseTerminalRequest,
    manager: TerminalManager,
) -> anyhow::Result<ReleaseTerminalResponse> {
    if let Some(mut handle) = manager.inner.lock().await.remove(&request.terminal_id) {
        if let Some(mut child) = handle.child.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
    Ok(ReleaseTerminalResponse::default())
}
```

In practice these handlers are dead code with OpenCode today; implement them only if you intend to share the same code path with other ACP agents.

## Rust Desktop Streaming Bridge

To stream OpenCode's `session/update` events into a desktop UI, run the ACP client on a dedicated tokio runtime and forward through `mpsc`:

```rust
use agent_client_protocol::schema::v1::{
    ContentBlock, SessionNotification,
};
use std::str::FromStr;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextChunk(String),
    ThoughtChunk(String),
    AvailableCommands(Vec<(String, String)>),
    ToolCallStarted { id: String, title: String, kind: String },
    ToolCallProgress { id: String, output: Option<String> },
    ToolCallFinished { id: String, status: String },
    UsageUpdate { used: u64, size: u64, cost: f64, currency: String },
    TurnComplete { stop_reason: String },
    Error(String),
}

pub fn spawn_opencode_agent(
    project_dir: PathBuf,
) -> anyhow::Result<(mpsc::UnboundedReceiver<AgentEvent>, mpsc::UnboundedSender<String>)> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (prompt_tx, mut prompt_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        rt.block_on(async move {
            let agent = agent_client_protocol::AcpAgent::from_str("opencode acp --pure")?;
            agent_client_protocol::Client
                .builder()
                .name("claudine-opencode-desktop")
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
                                    SessionNotification::AgentThoughtChunk(chunk) => match chunk.content {
                                        ContentBlock::Text(t) => Some(AgentEvent::ThoughtChunk(t.text)),
                                        _ => None,
                                    },
                                    SessionNotification::AvailableCommandsUpdate(update) => Some(
                                        AgentEvent::AvailableCommands(
                                            update.available_commands.iter()
                                                .map(|c| (c.name.clone(), c.description.clone()))
                                                .collect()
                                        ),
                                    ),
                                    SessionNotification::ToolCall(tc) => Some(AgentEvent::ToolCallStarted {
                                        id: tc.tool_call_id.to_string(),
                                        title: tc.title,
                                        kind: format!("{:?}", tc.kind),
                                    }),
                                    SessionNotification::ToolCallUpdate(update) => {
                                        let status = format!("{:?}", update.fields.status);
                                        let output = update.fields.content.as_ref().and_then(|c| c.first()).and_then(|cc| match &cc.content {
                                            ContentBlock::Text(t) => Some(t.text.clone()),
                                            _ => None,
                                        });
                                        if output.is_some() {
                                            Some(AgentEvent::ToolCallProgress {
                                                id: update.tool_call_id.to_string(),
                                                output,
                                            })
                                        } else {
                                            Some(AgentEvent::ToolCallFinished {
                                                id: update.tool_call_id.to_string(),
                                                status,
                                            })
                                        }
                                    }
                                    SessionNotification::UsageUpdate(usage) => Some(AgentEvent::UsageUpdate {
                                        used: usage.used,
                                        size: usage.size,
                                        cost: usage.cost.as_ref().map(|c| c.amount).unwrap_or(0.0),
                                        currency: usage.cost.as_ref().map(|c| c.currency.clone()).unwrap_or_default(),
                                    }),
                                    _ => None,
                                };
                                if let Some(event) = event { let _ = tx.send(event); }
                                Ok(())
                            }
                        }
                    },
                    agent_client_protocol::on_receive_notification!(),
                )
                .connect_with(agent, |connection| async move {
                    use agent_client_protocol::schema::v1::{
                        InitializeRequest, NewSessionRequest, PromptRequest, ProtocolVersion,
                    };
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
                                vec![ContentBlock::Text(
                                    agent_client_protocol::schema::v1::TextContent::new(prompt)
                                )],
                            ))
                            .block_task()
                            .await
                        {
                            Ok(r) => { let _ = event_tx.send(AgentEvent::TurnComplete { stop_reason: format!("{:?}", r.stop_reason) }); }
                            Err(e) => { let _ = event_tx.send(AgentEvent::Error(e.to_string())); }
                        }
                    }
                    Ok(())
                })
                .await
                .ok();
        });
        Ok::<_, anyhow::Error>(())
    })?;

    Ok((event_rx, prompt_tx))
}
```

### Tauri usage

```rust
#[tauri::command]
async fn send_prompt(state: tauri::State<'_, AppState>, prompt: String) -> Result<(), String> {
    state.prompt_tx.send(prompt).map_err(|e| e.to_string())
}

fn start_listener(event_rx: mpsc::UnboundedReceiver<AgentEvent>, handle: tauri::AppHandle) {
    tokio::spawn(async move {
        let mut rx = event_rx;
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::TextChunk(text) => { let _ = handle.emit("agent:text", text); }
                AgentEvent::ToolCallStarted { id, title, kind } => {
                    let _ = handle.emit("agent:tool:start", serde_json::json!({"id": id, "title": title, "kind": kind}));
                }
                AgentEvent::TurnComplete { stop_reason } => { let _ = handle.emit("agent:done", stop_reason); }
                AgentEvent::Error(e) => { let _ = handle.emit("agent:error", e); }
                _ => {}
            }
        }
    });
}
```

### iced usage

```rust
fn agent_subscription(rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<AgentEvent>>>>) -> iced::Subscription<AgentEvent> {
    iced::subscription::channel(
        std::any::TypeId::of::<AgentEvent>(),
        100,
        |mut output| async move {
            let mut rx = rx.lock().await.take().expect("subscription already consumed");
            while let Some(event) = rx.recv().await {
                let _ = output.send(event).await;
            }
            std::future::pending().await
        },
    )
}
```

## Claudine Integration Notes

Adding ACP support for OpenCode inside Claudine is the lowest-friction of the eight providers, because the binary already speaks ACP natively. Practical work:

1. **Launch detection** — add `opencode` to the existing provider launcher roster and route ACP-mode launches to `opencode acp --pure --log-level WARN`. The `--pure` flag prevents plugin auto-load from contaminating the wire surface. The default `--cwd` of `process.cwd()` matches Claudine's existing launch-CWD resolution.
2. **Capability negotiation** — record the native capability set above in Claudine's `agents`/`provider_info` tables. The set is small (`agentCapabilities` keys are stable across versions), so a one-time capture in `provider_id` is sufficient.
3. **Reverse-request routing** — register a `session/request_permission` handler that funnels through Claudine's existing `permissions::PolicyEngine`. Default unknown tools to `reject`; surface `kind` and `title` in the policy UI; route the response back through the ACP connection. Skip the optional `fs/write_text_file` handler to keep file edits inside OpenCode's own tools.
4. **Streaming bridge** — map the `session/update` discriminators (`agent_message_chunk`, `agent_thought_chunk`, `tool_call`, `tool_call_update`, `available_commands_update`, `usage_update`) onto Claudine's existing `events` lifecycle. The five update types are a strict subset of the surface Claudine already handles from the proprietary OpenCode stream protocol, so the existing TUI/sound/messenger wiring needs no change.
5. **Auth preconditions** — require at least one provider credential before allowing a non-interactive ACP launch. Accept any of: env var (`ANTHROPIC_API_KEY`, etc.), pre-staged `auth.json`, or a successful `authenticate` round-trip. Surface `authMethods[0]._meta['terminal-auth']` to the user as a "log in via terminal" affordance when the client signals that capability.
6. **Quirk handling** — document issue #34638 in the launcher's preflight checklist so tooling that needs `OPENCODE_CONFIG_CONTENT` knows to fall back to file staging. Issue #35096 (Turkish locale) is a client-rendering bug; locale-pinning the TUI to `en-US` for ACP sessions avoids it. Issue #35073 (sync subagents) is the agent's responsibility but Claudine should set `agent: build` explicitly rather than rely on the default.
7. **Fallback path** — if `opencode acp` ever fails to negotiate `protocolVersion: 1`, fall back to the existing proprietary `opencode run --format json` streaming path. The capability check is the same `initialize` probe already used for the JSON-stream launcher.

Because OpenCode is already in Claudine's roster and the ACP wire surface is a strict subset of the proprietary stream surface, ACP support is purely additive — no existing Claudine behaviour changes.

## Sources

- [OpenCode main documentation](https://opencode.ai/docs)
- [OpenCode ACP support page](https://opencode.ai/docs/acp/)
- [OpenCode server documentation](https://opencode.ai/docs/server/)
- [`anomalyco/opencode` repository](https://github.com/anomalyco/opencode)
- [Release `v1.17.13`](https://github.com/anomalyco/opencode/releases/tag/v1.17.13) (latest, 2026-07-01)
- [ACP implementation source (`packages/opencode/src/acp/`)](https://github.com/anomalyco/opencode/tree/dev/packages/opencode/src/acp)
- [`agent.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/agent.ts)
- [`service.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/service.ts)
- [`event.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/event.ts)
- [`permission.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/permission.ts)
- [`tool.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/tool.ts)
- [`error.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/error.ts)
- [`config-option.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/config-option.ts)
- [`content.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/content.ts)
- [`usage.ts`](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/acp/usage.ts)
- [CLI entrypoint (`packages/opencode/src/cli/cmd/acp.ts`)](https://raw.githubusercontent.com/anomalyco/opencode/dev/packages/opencode/src/cli/cmd/acp.ts)
- [Agent Client Protocol specification](https://agentclientprotocol.com/)
- [ACP schema (v1.1.0)](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/schema/v1/schema.json)
- [`agentclientprotocol/agent-client-protocol` schema repo](https://github.com/agentclientprotocol/agent-client-protocol)
- [`agent-client-protocol` Rust SDK on crates.io](https://crates.io/crates/agent-client-protocol)
- Issue [#34638 — opencode acp mode ignores OPENCODE_CONFIG_CONTENT and OPENCODE_CONFIG_DIR](https://github.com/anomalyco/opencode/issues/34638)
- Issue [#34743 — opencode ACP from Xcode 27 beta 2 uses default model big-pickle](https://github.com/anomalyco/opencode/issues/34743)
- Issue [#35073 — subagent permission asks hang indefinitely](https://github.com/anomalyco/opencode/issues/35073)
- Issue [#35096 — ACP tool name comparison breaks in Turkish/Azerbaijani locale](https://github.com/anomalyco/opencode/issues/35096)
- Issue [#33949 — ACP permission prompts show tool name instead of the command](https://github.com/anomalyco/opencode/issues/33949) (closed)
- Issue [#34193 — ACP session/fork support status](https://github.com/anomalyco/opencode/issues/34193) (closed)
- Issue [#34551 — Add reasoning effort/level selector in JetBrains AI Assistant via ACP](https://github.com/anomalyco/opencode/issues/34551) (closed)
- Local ACP probe artifacts: `/tmp/acp-stream*.log` (recorded during research on 2026-07-03 against the installed binary)