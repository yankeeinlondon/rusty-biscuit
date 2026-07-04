---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://qwenlm.github.io/qwen-code-docs/en/users/overview/
acp_docs: https://agentclientprotocol.com/
repo: https://github.com/QwenLM/qwen-code
support: native
launch_modes:
  - command: qwen --acp
    args: []
    transport: stdio
    adapter: none
    notes: "Direct native ACP mode in the primary `qwen` binary. Verified locally with Qwen Code 0.15.6: piping an `initialize` JSON-RPC request into `qwen --acp` returns a valid `initialize` response on stdout with `protocolVersion: 1`, an `agentInfo` block (`name: qwen-code`, `title: Qwen Code`, `version: 0.15.6`), and full capability negotiation."
  - command: qwen --acp --channel ACP
    args: []
    transport: stdio
    adapter: none
    notes: "Optional explicit-channel flag. `--channel` accepts `VSCode`, `ACP`, `SDK`, `CI`. `ACP` is the same code path as `--acp`; the flag is informational. Useful for log filtering and future telemetry routing."
  - command: qwen --acp --approval-mode yolo --allowed-mcp-server-names <names>
    args: []
    transport: stdio
    adapter: none
    notes: "Permission and MCP filters apply in ACP mode. `--approval-mode` accepts `plan`, `default`, `auto-edit`, `yolo`. `--allowed-mcp-server-names` constrains which MCP servers the agent may connect to; the comma-separated list is merged into the session's effective server set."
  - command: qwen --acp --input-format stream-json --output-format stream-json
    args: []
    transport: stdio
    adapter: none
    notes: "Reserved for the dual-output pipeline. In ACP mode the ACP frame stream owns stdout, so this combination is the warning case rather than a normal launch (ACP mode suppresses stream-json output)."
protocol_versions:
  - "1 (ACP schema — single integer MAJOR; matches the value bundled in @agentclientprotocol/sdk PROTOCOL_VERSION constant)"
  - "v1 (Qwen Code daemon envelope — independent axis used by `qwen serve`'s `/capabilities` endpoint, separate from ACP itself)"
capabilities:
  - capability: initialize
    support: supported
    notes: "Verified empirically on 0.15.6. Returns `protocolVersion: 1`, `agentInfo: {name: qwen-code, title: Qwen Code, version: 0.15.6}`, an `authMethods[]` array, and an `agentCapabilities` block."
  - capability: authenticate
    support: supported
    notes: "The `authenticate` reverse request from client to agent is implemented (extNotification `authenticate/update` is fired with the device-flow URI for OAuth). The Qwen Code ACP integration `authenticate` method delegates to `Config.refreshAuth(method)` and updates `security.auth.selectedType` in settings on success."
  - capability: session_new
    support: supported
    notes: "Verified empirically. `QwenAgent.newSession` returns `sessionId` (UUID v4 string), `models` (`currentModelId` + `availableModels[]` with `_meta.contextLimit`), `modes` (`currentModeId` + `availableModes[]`), and `configOptions[]` (`mode` and `model` selects). Accepts `cwd` and `mcpServers[]` per the spec; rejects the request with an `auth_required` error before authentication is completed."
  - capability: session_load
    support: supported
    notes: "Advertised in `agentCapabilities.loadSession: true`. `QwenAgent.loadSession` rehydrates a session by id, replays conversation history through `session/update`, and returns the modes/models/configOptions tri-tuple."
  - capability: session_prompt
    support: supported
    notes: "Verified empirically. Returns `stopReason: end_turn` on completion. Streaming output flows through `session/update` notifications (text, thought, tool call, tool call update, plan)."
  - capability: session_cancel
    support: supported
    notes: "`QwenAgent.cancel` calls `session.cancelPendingPrompt()` which triggers the abort signal and resolves pending tool requests with `ToolConfirmationOutcome.Cancel`."
  - capability: session_modes
    support: supported
    notes: "Modes are advertised on `session/new` and `session/load`. The four built-in modes map to ACP `modeId` values `plan`, `default`, `auto-edit`, `yolo`. `QwenAgent.setSessionMode` and `session/set_config_option` (configId=`mode`) mutate the mode at runtime. A `current_mode_update` notification is emitted when the agent self-switches (e.g. after a `ProceedAlways` outcome). Note: in 0.15.6 the `session/set_mode` and `session/set_config_option` methods exist on `QwenAgent` but are not advertised in `AGENT_METHODS`."
  - capability: streaming
    support: supported
    notes: "`session/update` notifications are emitted for `agent_message_chunk`, `agent_thought_chunk`, `user_message_chunk`, `tool_call`, `tool_call_update`, `plan`, `available_commands_update`, and `current_mode_update`. Token usage metadata is attached to the final chunk via `_meta.usage` (`inputTokens`, `outputTokens`, `totalTokens`, `thoughtTokens`, `cachedReadTokens`, `cachedWriteTokens`)."
  - capability: permissions
    support: supported
    notes: "`session/request_permission` reverse request is fully supported. Verified by reading `toPermissionOptions`/`buildPermissionRequestContent` in the bundled CLI. The outcome enum maps the ACP `PermissionOption` ids back to Qwen Code's `ToolConfirmationOutcome` enum (`proceed_once`, `proceed_always`, `proceed_always_server`, `proceed_always_tool`, `proceed_always_project`, `proceed_always_user`, `modify_with_editor`, `restore_previous`, `cancel`)."
  - capability: fs_read
    support: supported
    notes: "Verified empirically — Qwen Code 0.15.6 issued a `fs/read_text_file` reverse request when prompted to read `/tmp/qwen-test/hello.txt`. Implemented by `AcpFileSystemService.readTextFile` which calls `connection.readTextFile({...params, sessionId})` and falls back to the local FS service if the client did not advertise `fs.readTextFile: true`."
  - capability: fs_write
    support: supported
    notes: "Implemented symmetrically by `AcpFileSystemService.writeTextFile`. Honors an optional `_meta.bom` flag that prepends a BOM to the written content."
  - capability: terminal
    support: partial
    notes: "The bundled `@agentclientprotocol/sdk` AgentSideConnection defines the five terminal methods (`terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release`) and the CLI's `setupFileSystem` advertises terminal capabilities via `clientCapabilities.terminal`. However Qwen Code's shell tool (`run_shell_command`) is implemented as a built-in tool that runs inside the agent process — the agent does NOT typically issue `terminal/create` reverse requests during normal operation. Clients advertising `terminal: true` should implement the handlers for completeness, but the practical load is light."
  - capability: mcp
    support: supported
    notes: "`mcpCapabilities: {http: true, sse: true}` advertised at initialize. `McpServer` is accepted in `session/new`'s `mcpServers[]` array (stdio, http, and sse transports). The `newSessionConfig` helper merges client-supplied servers with `settings.merged.mcpServers`."
  - capability: plans
    support: supported
    notes: "`Plan` session update is emitted when the agent calls the `exit_plan_mode` tool. The bundled `runAcpAgent` injects a plan-mode system reminder (`getPlanModeSystemReminder`) so the plan-mode reminder surface is preserved on the ACP path — see Quirks for the original #1151 regression."
  - capability: media
    support: supported
    notes: "`promptCapabilities: {image: true, audio: true, embeddedContext: true}` advertised. Image prompts work, but on text-only models the vision bridge is bypassed (see issue #6110)."
  - capability: other
    support: supported
    notes: "Qwen Code also exposes `unstable_listSessions`, `unstable_resumeSession`, `unstable_setSessionModel` and the extMethod `deleteSession`, `renameSession`, `getAccountInfo`. The bundled `session/list` and `session/set_model` schema methods exist in `AGENT_METHODS` but are not all advertised in capabilities."
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: "Required. The agent sends a permission request for every tool that requires approval (edit, exec, mcp, info, plan, ask_user_question). The client must respond with a `RequestPermissionOutcome` (selected optionId or cancelled)."
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: true
    notes: "Required if the client advertises `fs.readTextFile: true`. The agent calls this for skill reads, plan reads, and user-facing tool reads. Verified empirically — the request payload is `{path, limit, sessionId}`."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: true
    notes: "Required if the client advertises `fs.writeTextFile: true`. Used by Qwen Code's edit tool when targeting the client filesystem (e.g. worktrees, remote workspaces)."
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: "Optional. Qwen Code's `run_shell_command` runs inside the agent process via the built-in shell tool. Implement for forward-compat or when wrapping the agent inside a sandbox where the agent process has no shell access."
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: "Optional. Only fires if the client also handled a `terminal/create` request."
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: "Optional. Polled by the agent to know when a `terminal/create` request finished."
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: "Optional. Always call `terminal/release` after this."
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: false
    notes: "Optional. The agent MUST call release on every handle it acquired, otherwise handles leak. Always implement this."
  - method: ext_notification (authenticate/update)
    purpose: auth
    client_must_handle: false
    notes: "Optional. The agent pushes the OAuth device-flow verification URI through this side channel during `authenticate`. Clients that want to render the URI in their own UI should listen for it."
permission_model:
  mechanism: session/request_permission reverse request
  timeout: client-defined (Qwen Code's `fireSessionEndOnce` runs when stdin/stdout close, so uncapped by the agent)
  default_policy: no default; every tool call that requires approval receives a `Selected` or `Cancelled` response. Approval mode (`plan`/`default`/`auto-edit`/`yolo`) pre-filters which tools need approval.
  approval_values:
    - proceed_once
    - proceed_always_server
    - proceed_always_tool
    - proceed_always_project
    - proceed_always_user
    - proceed_always
    - modify_with_editor
    - restore_previous
    - cancel
  notes: "Qwen Code's permission enum is finer-grained than the spec's `allow_once`/`allow_always`/`reject_once`/`reject_always`. The CLI maps its nine `ToolConfirmationOutcome` values onto the four `PermissionOptionKind` values for the wire (`kind: allow_once | allow_always | reject_once | reject_always`); the `optionId` preserves the internal value so the agent can branch on it. `filterAlwaysAllowOptions` hides `allow_always` for `ask_user_question` confirmations."
filesystem_model:
  read_methods:
    - fs/read_text_file
  write_methods:
    - fs/write_text_file
  path_base: "absolute paths only (paths are resolved against the session cwd)"
  sandboxing: "client-side; the agent passes the request to AcpFileSystemService only when the client advertised `fs.readTextFile: true` / `fs.writeTextFile: true`. When the client did not advertise them, Qwen Code falls back to its local filesystem service."
  notes: "ACP requires absolute paths and 1-based line numbers. Qwen Code attaches the session id to every request (`{...params, sessionId}`) so the client can resolve project boundaries per-session. The Qwen Code daemon's `BridgeFileSystem` adapter is a reference implementation of the host-side policy layer."
terminal_model:
  supported: true
  methods:
    - terminal/create
    - terminal/output
    - terminal/wait_for_exit
    - terminal/kill
    - terminal/release
  shell: "depends on host; Qwen Code itself uses its built-in `run_shell_command` tool and only delegates via `terminal/create` when the client requests it"
  cwd: "absolute path supplied in CreateTerminalRequest; defaults to the session cwd"
  streaming: "polled via terminal/output"
  cancellation: "terminal/kill or terminal/release"
  notes: "Schema-level support is complete; the Qwen Code CLI does not normally issue these methods. Implement the handlers as a general ACP completeness matter, especially for clients embedding the agent in a sandboxed runtime where the agent process has no shell."
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
    - "JSON-RPC errors are returned on the request channel (not inside session/update)"
  notes: "Updates are fire-and-forget JSON-RPC notifications. The `_meta.usage` field on the last `agent_message_chunk` of a turn carries token usage (`inputTokens`, `outputTokens`, `totalTokens`, `thoughtTokens`, `cachedReadTokens`, `cachedWriteTokens`). Tool kinds include `read`, `edit`, `delete`, `move`, `search`, `execute`, `think`, `fetch`, and `other` (inferred from `toolCallEmitter.resolveToolMetadata`)."
auth_setup:
  required: true
  mechanisms:
    - "OPENAI_API_KEY environment variable (drives `--auth-type=openai`)"
    - "DASHSCOPE_API_KEY environment variable (Qwen provider's default)"
    - "Qwen OAuth (free tier discontinued 2026-04-15; paid plans only)"
    - "Anthropic, Gemini, Vertex-AI auth types (--auth-type)"
    - "OpenRouter (built-in provider preset)"
    - "Pre-authenticated CLI session in `~/.qwen/` (cached OAuth tokens, selectedType in settings)"
    - "Run `qwen auth` interactively to choose a provider"
  headless_notes: "For headless ACP operation, set OPENAI_API_KEY / DASHSCOPE_API_KEY in the spawning shell, or pre-run `qwen auth` once on the host. The agent's `ensureAuthenticated` throws an `auth_required` error with the matching `authMethods[]` if no method is selected, and the client may then drive an `authenticate` round-trip to switch providers."
  notes: "Auth methods advertised by `buildAuthMethods()`: `openai` (OPENAI_API_KEY / DASHSCOPE_API_KEY) and `qwen-oauth`. The `getAccountInfo` extMethod returns `{authType, model, baseUrl, apiKeyEnvKey}` so a host UI can show the resolved credentials without leaking the secret value."
env_vars:
  - name: OPENAI_API_KEY
    effect: "Authenticates the openai provider when selected; the daemon's `modelProviders.openai[]` entries read DASHSCOPE_API_KEY/OPENAI_API_KEY from env."
  - name: DASHSCOPE_API_KEY
    effect: "Default credential for the bundled Qwen provider preset; mapped via `modelProviders.openai[*].envKey`."
  - name: OPENAI_API_BASE_URL
    effect: "Override the base URL for the openai provider (compatible mode)."
  - name: QWEN_CODE_MAX_TOOL_CONCURRENCY
    effect: "Per-session cap on parallel tool calls inside `runToolCalls` (default 10)."
  - name: QWEN_CODE_SIMPLE_ENV_VAR
    effect: "Set internally by `--bare` / `isBareMode` to suppress implicit startup auto-discovery."
  - name: QWEN_CODE_NO_RELAUNCH
    effect: "Disables the post-init heap-size relaunch (used in containerized/embedded runs)."
  - name: DEBUG
    effect: "Enables React StrictMode in the TUI; ignored in ACP mode."
  - name: SANDBOX
    effect: "Activates the seatbelt (macOS) or sandbox-image path that wraps the CLI."
  - name: NO_BROWSER
    effect: "If set, the OAuth device flow skips the browser launch (useful for headless ACP)."
  - name: QWEN_DISABLED_SLASH_COMMANDS
    effect: "Comma-separated list of slash commands to hide from the agent's command palette (merged with `slashCommands.disabled`)."
rust_client:
  crate: agent-client-protocol (preferred) or any ACP-compatible JSON-RPC client
  connection_type: AcpAgent subprocess over stdio (JSON-RPC), or HTTP via the `qwen serve` daemon's ACP-over-HTTP bridge
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
  desktop_streaming_pattern: "tokio::sync::mpsc from the `session/update` notification handler to the UI thread; the connection is `Send`/`Sync` in agent-client-protocol 1.0+ so `tokio::spawn` (not LocalSet) is fine."
  notes: "The official `agent-client-protocol` Rust crate (currently 1.0.x, schema 1.1.0) is the canonical SDK and matches the methods Qwen Code exposes. For HTTP transport, the `qwen serve` daemon exposes ACP-over-HTTP — see `claudine/docs/research/acp/...` for details. For headless contexts (one-shot batches), the `agent-client-protocol` `AcpAgent::qwen_code()` preset is not yet available; build the agent string explicitly with `AcpAgent::from_str(\"qwen --acp\")` (or with `qwen serve` URL for HTTP)."
compatibility:
  - client: Zed Editor
    status: works
    issue: "Documented as the canonical client. Zed ships the Qwen Code agent in its ACP registry; manual install uses `command: qwen`, `args: [\"--acp\"]`."
    workaround: "Use Zed's 'Install from Registry' flow, or add a custom agent entry referencing `qwen --acp`."
  - client: JetBrains IDEs
    status: works
    issue: "Supported since the 2026 Q1 ACP push. IntelliJ 2025.3+ is the lowest tested version. Older releases require the manual `~/.jetbrains/acp.json` configuration."
    workaround: "Manual install uses `agent_servers.qwen = { command: qwen, args: [\"--acp\"], env: {} }`."
  - client: Marimo / other ACP clients
    status: works
    issue: "Any ACP v1 client works; only the standard `initialize` -> `session/new` -> `session/prompt` flow is needed for basic integration."
    workaround: "none"
  - client: Custom JSON-RPC client (e.g. Python `jsonrpc`, Rust `agent-client-protocol`, Node `@agentclientprotocol/sdk`)
    status: works
    issue: "The Qwen Code ACP mode is a clean JSON-RPC 2.0 stdio surface; any conformant client works."
    workaround: "none"
  - client: Subagent-only flows
    status: partial
    issue: "Qwen Code's subagent tracker (`SubAgentTracker`) re-issues permission requests through the same `session/request_permission` reverse request, so the parent client must remain responsive for the whole nested call tree. Subagent `agent_message_chunk` updates arrive via the same `sessionId` as the parent turn; the `_meta` field on the chunk carries `parentToolCallId` and `subagentType`."
    workaround: "Treat the parent session id as the only routing key and surface subagent events through the existing UI tree."
  - client: Clients that don't advertise `fs.readTextFile`/`fs.writeTextFile`
    status: partial
    issue: "Qwen Code falls back to its own local FS service; the client never sees the file reads. This is a security footgun if the client intends to enforce a sandbox."
    workaround: "Always advertise `fs.readTextFile: true` and `fs.writeTextFile: true` and enforce the policy on the client side."
recent_changes:
  - date: 2026-07-03
    version: "v0.19.6"
    change: "Bootstrap fast paths, mobile session-switch jank fix, dataviz bundled skill, opt-in per-tool-call execution timeout, configurable cron/loop job expiration, vision model selection in daemon UI."
    impact: "ACP clients benefit from the per-tool-call timeout (already-dead sessions are detected sooner) and from the vision model selection on the daemon side."
  - date: 2026-07-03
    version: "v0.19.5-nightly.20260703.b16baf1ff"
    change: "Reduce multimodal history payload size; subagent `${hook_context}` placeholder fix; bootstrap fast paths."
    impact: "Smaller ACP payload over `session/update` for multi-modal turns."
  - date: 2026-07-02
    version: "v0.19.5"
    change: "Defer session creation until first prompt; description+level in `/skills` ACP output; MCP capability discovery retry with backoff; friendly Esc interruption UX."
    impact: "Cold-start latency drops because the agent does not eagerly spawn MCP servers at `session/new` time."
  - date: 2026-07-01
    version: "issue #6110"
    change: "ACP image prompts bypass the vision bridge for text-only models — fix routes image content through the bridge when the configured model is text-only."
    impact: "Multi-modal ACP prompts on text-only models now correctly route through the vision bridge."
  - date: 2026-07-01
    version: "issue #6057"
    change: "Daemon session archive support."
    impact: "Daemon-side session metadata can be archived without deleting the underlying conversation."
  - date: 2026-07-01
    version: "cua-driver-rs v0.7.0 (vendored)"
    change: "feat(acp): support /cd command in ACP sessions (#5903)."
    impact: "ACP clients that surface a directory picker can drive `qwen`'s `/cd` slash command to switch the session's working directory mid-flight."
  - date: 2026-06-30
    version: "issue #6075"
    change: "ACP daemon loop fix — agent can no longer loop indefinitely on repeated invalid tool parameters."
    impact: "Daemon clients no longer need to detect/break agent loops themselves."
  - date: 2026-06-30
    version: "issue #6020"
    change: "`read_file` was reporting `[object Object]` for ACP skill reads — fixed by serializing the content properly."
    impact: "Skill reads via the client-side `fs/read_text_file` now return readable text rather than `[object Object]`."
  - date: 2026-06-29
    version: "issue #5968"
    change: "Long-running ACP sessions reported empty memory at the end — fixed by surfacing the long-term memory tool in the agent's tool list."
    impact: "Auto-memory is now reliably populated across multi-hour ACP sessions."
  - date: 2026-06-25
    version: "issue #5861"
    change: "Context compression request now uses stream=true to avoid gateway timeout."
    impact: "Long ACP turns no longer hang on the `/compress` command when the upstream LLM gateway has a 60s timeout."
  - date: 2026-06-18
    version: "v0.19.3"
    change: "ACP /cd slash command registered in the daemon UI; durable /loop survives restart."
    impact: "Loop jobs are persisted across daemon restarts; clients can now surface /cd as a slash command."
  - date: 2026-06-04
    version: "v0.19.x (early)"
    change: "ACP permission flow improvements; subagent permission prompts routed through `session/request_permission`."
    impact: "Subagent permission requests now surface to the host client uniformly."
  - date: 2026-05-30
    version: "PR #5995"
    change: "Serve fast-path bundle closure guard — fast path no longer leaves a dangling ACP child."
    impact: "Daemon clients using `--acp` from a `qwen serve` fast path no longer leak processes on error."
  - date: 2026-05-15
    version: "v0.19.x (early)"
    change: "ACP-bridge package extracted to `@qwen-code/acp-bridge` (was inline in `serve/`)."
    impact: "The bridge is reusable from non-CLI contexts (channels, VSCode IDE companion)."
  - date: 2026-04-22
    version: "issue #5023"
    change: "`exitPlanMode.ts` tool filtering — plan-mode reminder now properly restricts the agent to read-only tools in ACP mode (resolves long-standing #1151)."
    impact: "Plan mode in ACP is no longer a paper tiger; the agent now actually restricts to read-only tools."
quirks:
  - "`qwen --acp` is the canonical launch; there is no separate `qwen acp` subcommand. Verified on 0.15.6: only `--acp` and `--channel ACP` route into ACP mode."
  - "`qwen` requires Node ≥18. Bundled deps include `@agentclientprotocol/sdk` (no separate install) plus optional `@lydell/node-pty` for terminals and `@teddyzhu/clipboard` for image paste. Heavily Node-feature-flag-driven: `process.env.QWEN_CODE_NO_RELAUNCH=1` skips the heap-relauch re-exec that complicates containerized embedding."
  - "The CLI captures `console.log`, `console.info`, and `console.debug` and rebinds them to `console.error` before starting ACP mode so that user library output does NOT corrupt the stdout JSON-RPC stream. stderr stays untouched and is the correct surface for client diagnostics."
  - "The bundled AgentSideConnection defines all five `terminal/*` methods and the FS methods, but the agent process itself uses the built-in `run_shell_command` and `read_file`/`write_file` tools. The reverse requests only fire when the client advertises the matching capability and the agent has been routed through a filesystem adapter (`setupFileSystem`)."
  - "The `session/set_mode` and `session/set_config_option` methods exist on `QwenAgent` (and on the bundled `AGENT_METHODS`) but `0.15.6` does not advertise them through capability negotiation. Use `session/set_mode` from the wire; the server will route it to `QwenAgent.setSessionMode`."
  - "`session/load` and `session/resume` differ — `session/load` replays history through `session/update` notifications before returning, while `session/resume` (advertised via `sessionCapabilities.resume`) returns immediately and skips replay. Both honor the same params (`sessionId`, `cwd`, `mcpServers`)."
  - "`unstable_resumeSession` is exposed in the bundled SDK; Qwen Code exposes `unstable_listSessions`, `unstable_resumeSession`, and `unstable_setSessionModel`. New clients should feature-detect the stable counterparts first and fall back to `unstable_*` for older servers."
  - "Auth: `qwen-oauth` free tier was discontinued 2026-04-15 per `authMethods[1].description`. Clients driving `authenticate` must accept `auth_required` errors and either guide the user through OAuth or fall back to `OPENAI_API_KEY`."
  - "Subagent permission flow: `SubAgentTracker.createApprovalHandler` re-issues `session/request_permission` on the parent connection, so the host client only has ONE permission round-trip per tool call regardless of nesting depth. Plan-mode and ask-user-question confirmations also flow through this path."
  - "ACP `session/update` `plan` updates are sent when `exit_plan_mode` is invoked, but the `Plan` `entries[]` are sent with `priority: high`/`medium`/`low` and `status: pending`/`in_progress`/`completed`. The agent can continue executing tools from a plan via subsequent `tool_call` updates."
  - "The `AcpFileSystemService` writes a BOM (`\\uFEFF`) if the request carries `_meta.bom: true`. Clients that pass binary content should set this explicitly to avoid mojibake on Windows tools."
  - "The CLI's STDIN/STDERR redirection happens before `runAcpAgent` returns. Closing the client's stdin pipe triggers `fireSessionEndOnce(SessionEndReason.PromptInputExit)`; sending SIGTERM/SIGINT triggers `fireSessionEndOnce(SessionEndReason.Other)` and an `exitCleanup()` pass that runs `process.exit(0)` after the cleanup promise resolves."
  - "ACP mode auto-spawns a Qwen Code `SessionStart` hook if the host config has one. The hook fires BEFORE `session/update` notifications stream, so clients can hook into it to perform tool filtering or prompt rewriting."
  - "ACP over HTTP (the `qwen serve` daemon's `/acp` route, per `claudine/docs/research/acp/...` daemon 03 doc) uses the same JSON-RPC messages over a streamable HTTP transport. The official ACP SDK crate supports HTTP via `agent-client-protocol-http`. The bridged layer multiplexes N HTTP sessions onto one ACP child, with per-session event rings and a four-policy permission mediator."
gaps:
  - "Mode and config-option methods: `QwenAgent` implements `setSessionMode` and `setSessionConfigOption`, but neither is enumerated in `0.15.6`'s `agentCapabilities` block. Empirical testing required to confirm wire compatibility."
  - "Resume/list/delete session support: `unstable_*` methods exist; whether `0.15.6` answers `session/list`, `session/resume`, and `session/delete` requests is not verified — only `session/new`, `session/prompt`, `session/cancel`, `session/load` were exercised."
  - "`terminal/create` reverse requests: not exercised in local testing because Qwen Code's built-in shell handles shell commands. Whether the agent ever delegates via `terminal/create` (e.g. in plan-mode or when the agent process is sandboxed) is undocumented."
  - "`session/set_config_option` (configId values beyond `mode`/`model`) — only `mode` and `model` are constructed in `buildConfigOptions`; additional categories like `effort_level`, `fast_mode`, `reasoning_effort` would need to be added in a future version."
  - "Schema version reported by the bundled `@agentclientprotocol/sdk` is `protocolVersion: 1` (an integer). The agentclientprotocol.com docs describe v1 as a major version; the actual schema may evolve under that single integer."
  - "The Qwen Code daemon's HTTP `qwen serve` exposes ACP-over-HTTP via `packages/acp-bridge/`; this is upstream of the local Qwen Code ACP mode but uses the same JSON-RPC messages. The HTTP surface was not tested in this research — only the stdio surface was."
requires_claudine_update: true
reason: "Qwen Code's ACP support is now native (not adapter), the protocol surface is well-defined and stable at v1, and the bundled SDK methods match what Claudine's ACP client infrastructure will need. Claudine must add a launch path that detects `qwen --acp` and spawns the binary directly (no bridge process required), wire up `fs/read_text_file`, `fs/write_text_file`, `session/request_permission`, and the full terminal/* lifecycle as reverse-request handlers, route `session/update` notifications into Claudine's existing lifecycle pipeline (text, thought, tool call, plan, mode, available_commands), and model the OAuth-free auth methods (`openai`, `qwen-oauth`) through the new `authenticate` request instead of the legacy `auth_required` error path. The `qwen serve` ACP-over-HTTP bridge also offers a multiplexed alternative for Claudine's already-existing HTTP transport code path."
---

## Overview

Qwen Code (`@qwen-code/qwen-code`, binary `qwen`) is Alibaba's QwenLM-team open-source AI coding agent. Unlike providers that require an external ACP adapter, Qwen Code has **native** ACP support built into the primary CLI: the `--acp` flag (or `--channel ACP`) drops the agent into a stdio JSON-RPC 2.0 server using the official [`@agentclientprotocol/sdk`](https://www.npmjs.com/package/@agentclientprotocol/sdk), which Qwen Code bundles. ACP was originally championed by the [Zed editor](https://zed.dev/acp) team and is now supported across multiple editors (Zed, JetBrains, Neovim, Marimo) and agents (Claude Code, Codex CLI, Gemini CLI, Goose, Qwen Code).

This research covers only the Agent Client Protocol (JSON-RPC) surface and its adapters — it does not describe Qwen Code's proprietary non-interactive stream/output protocols.

## Launching ACP

### Native launch — `qwen --acp`

The primary way to start an ACP session is the `--acp` flag (a holdover of the older `--experimental-acp`, now graduated):

```bash
qwen --acp
```

This launches `qwen` as a child process and speaks JSON-RPC 2.0 over its stdio pipes. The CLI is hard-coded to recognize `--acp` (and `argv.experimentalAcp` for backward compatibility). Inside `qwen`, the `--acp` flag is read by `argv.acp || argv.experimentalAcp` and the actual ACP path is taken when `config2.getExperimentalZedIntegration()` returns true.

### Editor configuration

**Zed** (`settings.json`, manual install):
```json
{
  "agent_servers": {
    "Qwen Code": {
      "type": "custom",
      "command": "qwen",
      "args": ["--acp"],
      "env": {}
    }
  }
}
```

**JetBrains** (manual install — older IDEs):
```json
{
  "agent_servers": {
    "qwen": {
      "command": "/path/to/qwen",
      "args": ["--acp"],
      "env": {}
    }
  }
}
```

Modern JetBrains (2025.3+) and Zed expose Qwen Code directly via their ACP registries (Zed "Install from Registry", JetBrains "Add ACP Agent").

### Optional filters

The ACP launch supports several flags that ride along:

- `--approval-mode {plan,default,auto-edit,yolo}` — pre-filter which tools need approval.
- `--allowed-mcp-server-names <list>` — restrict MCP server connections.
- `--channel ACP` — explicit channel marker (same code path as `--acp`; useful for telemetry routing).
- `--allowed-tools <list>` — bypass confirmation for specific tools.
- `--exclude-tools <list>` — disable tools.

### Transport and framing

- **Transport**: stdio JSON-RPC 2.0 between the ACP client and `qwen`. The CLI binds `console.log`, `console.info`, and `console.debug` to `console.error` so user-library output does NOT corrupt the JSON-RPC stream on stdout. **stderr stays free for diagnostic logs.**
- **Framing**: newline-delimited JSON, UTF-8.
- **Direction**: bidirectional — the client sends requests/notifications to the agent; the agent sends responses, reverse requests, and protocol notifications.

## Protocol and Capabilities

### Protocol version

Both the local Qwen Code 0.15.6 binary and the official Rust SDK (`agent-client-protocol` 1.0.x) negotiate **ACP v1 / `protocolVersion: 1`**. The Qwen Code daemon's `/capabilities` endpoint uses a separate `v1` (`SERVE_PROTOCOL_VERSION`) for its HTTP envelope, which is independent of ACP itself.

### Supported protocol version

Verified empirically on `qwen-code` 0.15.6 — `initialize` returns `protocolVersion: 1`, `agentInfo.name: qwen-code`, `agentInfo.version: 0.15.6`. The bundled `@agentclientprotocol/sdk` reports `PROTOCOL_VERSION = 1` (a single integer major version, as the ACP spec defines).

### Capability surface

Verified via the live `initialize` response from Qwen Code 0.15.6 plus the source code in `packages/cli/src/acp-integration/`:

| Area | Status | Notes |
|------|--------|-------|
| `initialize` / `authenticate` | supported | `authenticate` triggers device-flow OAuth and emits `authenticate/update` extNotification with the verification URI. |
| `session/new` / `session/load` / `session/prompt` / `session/cancel` | supported | `session/prompt` returns `stopReason: end_turn` on completion. |
| `session/resume` / `session/list` / `session/close` / `session/delete` | partial | `unstable_listSessions`, `unstable_resumeSession` and the `deleteSession`/`renameSession` extMethods exist. |
| `session/set_mode` / `session/set_config_option` | supported | `QwenAgent` implements both, but they are not advertised through `agentCapabilities` in 0.15.6. |
| `session/set_model` | supported | Via `unstable_setSessionModel`. |
| `session/request_permission` | supported | The core reverse request — handles `edit`/`exec`/`mcp`/`info`/`plan`/`ask_user_question` confirmations. |
| `fs/read_text_file` / `fs/write_text_file` | supported | Empirically verified: the agent issued `fs/read_text_file` for `/tmp/qwen-test/hello.txt` during testing. |
| `terminal/*` | supported (schema); rarely used by the agent itself | The CLI's built-in shell tool runs in-process; reverse requests fire only when the client advertises `terminal: true` and the agent has been routed through a terminal adapter. |
| `session/update` streaming | supported | Text, thought, tool call/update, plan, available_commands_update, current_mode_update. Token usage metadata arrives on the final chunk's `_meta.usage`. |
| MCP (`mcpCapabilities.http`, `mcpCapabilities.sse`) | supported | stdio MCP servers are always supported; HTTP/SSE are advertised. |
| `Plan` updates | supported | Emitted when the agent calls `exit_plan_mode`; entries have `priority: high|medium|low` and `status: pending|in_progress|completed`. |
| `available_commands_update` | supported | Emitted on every `session/new` and `session/load`; clients should refresh their slash-command palette. |
| `current_mode_update` | supported | Emitted when the agent self-switches mode (e.g. after `ProceedAlways`). |
| `extMethod` / `extNotification` | supported | `deleteSession`, `renameSession`, `getAccountInfo`, `authenticate/update`. |

## Reverse Requests

Qwen Code's ACP mode is bidirectional — the agent issues **reverse requests** to the client. Five distinct reverse requests are observable; their volume depends on how the client advertises capabilities and which tools the model picks.

### Permission requests (required)

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "session/request_permission",
  "params": {
    "sessionId": "10fda6d6-b199-4880-9fd9-51c4b9d61517",
    "toolCall": {
      "toolCallId": "edit::src/auth.rs",
      "status": "pending",
      "title": "Edit src/auth.rs",
      "content": [
        {"type": "diff", "path": "src/auth.rs", "oldText": "...", "newText": "..."}
      ],
      "kind": "edit",
      "rawInput": {"file_path": "src/auth.rs", "new_content": "..."}
    },
    "options": [
      {"optionId": "proceed_always", "name": "Allow All Edits", "kind": "allow_always"},
      {"optionId": "proceed_once", "name": "Allow", "kind": "allow_once"},
      {"optionId": "cancel", "name": "Reject", "kind": "reject_once"}
    ]
  }
}
```

The client responds with `RequestPermissionResponse` whose `outcome` is either `Selected{optionId}` (echoing one of the agent's optionIds) or `Cancelled`. The agent's `ToolConfirmationOutcome` enum includes nine values; the wire-level `kind` is reduced to the ACP four-value `allow_once`/`allow_always`/`reject_once`/`reject_always` while the `optionId` carries the finer-grained decision.

### Filesystem requests (required when client advertises fs capabilities)

```json
{"jsonrpc":"2.0","id":101,"method":"fs/read_text_file","params":{"path":"/tmp/qwen-test/hello.txt","limit":1000,"sessionId":"2fdd1c0e-438e-4dff-8d7c-2725dc9f6892"}}
```

Verified empirically. The reply is `{ content: "..." }` (string content). `limit` is honored as a byte cap on the response.

```json
{"jsonrpc":"2.0","id":102,"method":"fs/write_text_file","params":{"path":"...","content":"...","sessionId":"..."}}
```

Qwen Code attaches `sessionId` to every reverse request so the client can resolve project boundaries per-session.

### Terminal requests (rare in practice)

```json
{"jsonrpc":"2.0","id":43,"method":"terminal/create","params":{"sessionId":"...","command":"cargo","args":["build"],"cwd":"/project","env":[{"name":"RUST_LOG","value":"info"}],"outputByteLimit":1048576}}
```

Qwen Code's built-in `run_shell_command` runs in-process, so this fires only when the client advertises `terminal: true` AND the agent has been routed through a terminal adapter. Implement the full lifecycle (`terminal/create` → `terminal/output` / `terminal/wait_for_exit` → `terminal/kill` (optional) → `terminal/release`) as a matter of general ACP completeness; the practical load is light.

### extMethod and extNotification

- `authenticate/update` extNotification carries `{ _meta: { authUri: "https://..." } }` so a client UI can render the OAuth verification URI inline.
- `deleteSession` / `renameSession` / `getAccountInfo` extMethods let a host UI manage session lifecycle from the ACP channel.

## Permissions, Filesystem, and Terminal

### Permission policy

- The client is the authority for every tool call requiring approval.
- Qwen Code's four approval modes (`plan`, `default`, `auto-edit`, `yolo`) filter which tools need approval — `plan` restricts to read-only tools, `yolo` skips approval entirely, the other two prompt selectively.
- For each `session/request_permission` the agent sends a `toolCall` block with the tool name, title, kind, raw input, and (for edits) a `diff` content block. The client responds with the chosen `optionId`.
- Qwen Code attaches the `sessionId` to every reverse request so the client can maintain per-session policy state.

### Filesystem policy

- Filesystem reads/writes are delegated to the client only when `fs.readTextFile: true` / `fs.writeTextFile: true` is advertised.
- Paths are absolute and 1-based line numbers apply (per ACP spec).
- `AcpFileSystemService.readTextFile` calls `connection.readTextFile({...params, sessionId})` and falls back to the local FS service if the client does not advertise the capability.
- `AcpFileSystemService.writeTextFile` honors an optional `_meta.bom: true` flag that prepends a BOM to the written content (useful for Windows-targeted files).

### Terminal policy

- The agent process's `run_shell_command` tool runs commands in-process. The `terminal/*` reverse requests are only used when the client routes the agent through a `BridgeFileSystem`/`BridgeTerminal` adapter.
- For general ACP completeness, the client receives the full command, arguments, environment variables, and working directory, decides whether to allow (often via the same `session/request_permission` flow or an implicit trust), and is responsible for process lifecycle, output buffers, byte-limit truncation (truncating from the beginning when `outputByteLimit` is exceeded, at a character boundary), and the always-call `terminal/release` discipline.

## Streaming and UI Integration

Streaming flows through `session/update` notifications. Common update variants:

| Update | Purpose |
|--------|---------|
| `agent_message_chunk` | Incremental assistant text. The final chunk's `_meta.usage` carries `inputTokens`, `outputTokens`, `totalTokens`, `thoughtTokens`, `cachedReadTokens`, `cachedWriteTokens`. |
| `agent_thought_chunk` | Internal reasoning / extended thinking. |
| `user_message_chunk` | User message replay during `session/load`. |
| `tool_call` | A new tool call has started; carries `toolCallId`, `title`, `kind`, `content[]`, `status`. |
| `tool_call_update` | Tool progress, status change, or final result. |
| `plan` | Multi-step plan entry (live streaming). Each entry has `priority` and `status`. |
| `available_commands_update` | Slash commands the agent advertises — refresh the palette. |
| `current_mode_update` | Mode change. |

Notifications are fire-and-forget — group by `ContentChunk.message_id` to disambiguate parallel streams. Token usage metadata arrives on the last `agent_message_chunk` of a turn via `_meta.usage`.

## Authentication and Setup

The bundled `@agentclientprotocol/sdk` handles protocol framing. The CLI itself reads auth from `~/.qwen/settings.json` (`security.auth.selectedType`):

1. **`openai`** — driven by `OPENAI_API_KEY`, `DASHSCOPE_API_KEY`, or the bundled Qwen provider presets. Recommended for headless ACP operation.
2. **`qwen-oauth`** — Qwen OAuth (free tier discontinued 2026-04-15; paid plans only).
3. **`anthropic`**, **`gemini`**, **`vertex-ai`** — provider-specific.
4. **Pre-existing session** — `~/.qwen/oauth_creds.json` plus `~/.qwen/settings.json`.

For headless automation (CI, daemon contexts), set `OPENAI_API_KEY` or pre-run `qwen auth` once on the host. The agent's `ensureAuthenticated` throws an `auth_required` error with `authMethods[]` if no method is selected; the client may then drive `authenticate` to switch providers.

The `getAccountInfo` extMethod returns `{authType, model, baseUrl, apiKeyEnvKey}` so a host UI can show the resolved credentials without leaking the secret value.

## Compatibility, Quirks, and Workarounds

1. **Native mode** — `qwen --acp` is the canonical launch; no separate `qwen acp` subcommand. Verified on 0.15.6: only `--acp` and `--channel ACP` route into ACP mode.
2. **Stdout protection** — the CLI rebinds `console.log/info/debug` to `console.error` before starting ACP. Library output that hits stdout would corrupt the JSON-RPC stream; the rebinding is essential. **Clients must read stderr for diagnostics**, not stdout.
3. **Filesystem delegation** — `fs/read_text_file` and `fs/write_text_file` fire ONLY when the client advertises the matching capability AND the agent has been routed through a filesystem adapter. If a client intends to enforce a sandbox, always advertise the FS capabilities.
4. **Subagent permission routing** — `SubAgentTracker.createApprovalHandler` re-issues `session/request_permission` on the parent connection. The host client sees ONE permission round-trip per tool call regardless of nesting depth. Subagent updates arrive on the parent's `sessionId`; the `_meta` field carries `parentToolCallId` and `subagentType`.
5. **Plan-mode reminder parity** — `runAcpAgent` injects `getPlanModeSystemReminder` so plan-mode system reminders surface on the ACP path (regression from #1151 where plan mode in ACP was silently inert). A follow-up fix also patched the `exit_plan_mode` tool filter.
6. **`session/load` vs `session/resume`** — `session/load` replays history through `session/update` notifications before returning; `session/resume` (advertised via `sessionCapabilities.resume`) returns immediately and skips replay. Both honor the same params.
7. **`unstable_*` aliases** — `unstable_listSessions`, `unstable_resumeSession`, and `unstable_setSessionModel` are exposed alongside the stable names. New clients should feature-detect the stable counterparts and fall back to `unstable_*` for older servers.
8. **OAuth free tier** — `qwen-oauth` free tier was discontinued 2026-04-15. Clients driving `authenticate` must accept `auth_required` errors and either guide the user through paid OAuth or fall back to `OPENAI_API_KEY`.
9. **`AcpFileSystemService.writeTextFile` BOM** — passes a BOM (`\uFEFF`) if the request carries `_meta.bom: true`. Clients that pass binary content should set this explicitly to avoid mojibake on Windows tools.
10. **`session/update` `plan` updates** — sent when `exit_plan_mode` is invoked. The agent can continue executing tools from a plan via subsequent `tool_call` updates.
11. **ACP-over-HTTP bridge (`qwen serve`)** — `packages/acp-bridge/` (extracted from `serve/`) exposes a multiplexed HTTP bridge that speaks the same JSON-RPC messages over streamable HTTP. Per-session event rings, a four-policy permission mediator (`first-responder`/`designated`/`consensus`/`local-only`), and a per-session FIFO queue are part of the bridge. Not tested in this research; documented separately in `claudine/docs/research/acp/...` daemon 03 doc.
12. **Skill reads previously returned `[object Object]`** — fixed in issue #6020 (June 30, 2026). Skill reads via the client-side `fs/read_text_file` now return readable text.
13. **Long-session memory loss** — fixed in issue #5968 (June 29, 2026). Long ACP sessions no longer report empty auto-memory at the end.
14. **Loop prevention on invalid tool params** — fixed in issue #6075 (June 30, 2026). The agent no longer loops indefinitely on repeated invalid tool parameters.

## Recent Changes

- **2026-07-03** (v0.19.6): Bootstrap fast paths, mobile session-switch jank fix, dataviz bundled skill, opt-in per-tool-call execution timeout, configurable cron/loop job expiration, vision model selection in daemon UI.
- **2026-07-03** (v0.19.5-nightly.20260703.b16baf1ff): Reduce multimodal history payload size; subagent `${hook_context}` placeholder fix; bootstrap fast paths.
- **2026-07-02** (v0.19.5): Defer session creation until first prompt; description+level in `/skills` ACP output; MCP capability discovery retry with backoff; friendly Esc interruption UX.
- **2026-07-01** (issue #6110): ACP image prompts bypass the vision bridge for text-only models — fixed by routing image content through the bridge when the configured model is text-only.
- **2026-07-01** (issue #6057): Daemon session archive support.
- **2026-07-01** (cua-driver-rs v0.7.0 vendored): feat(acp): support /cd command in ACP sessions (#5903).
- **2026-06-30** (issue #6075): ACP daemon loop fix — agent can no longer loop indefinitely on repeated invalid tool parameters.
- **2026-06-30** (issue #6020): `read_file` was reporting `[object Object]` for ACP skill reads — fixed by serializing the content properly.
- **2026-06-29** (issue #5968): Long-running ACP sessions reported empty memory at the end — fixed by surfacing the long-term memory tool in the agent's tool list.
- **2026-06-25** (issue #5861): Context compression request now uses stream=true to avoid gateway timeout.
- **2026-06-18** (v0.19.3): ACP /cd slash command registered in the daemon UI; durable /loop survives restart.
- **2026-06-04** (v0.19.x early): ACP permission flow improvements; subagent permission prompts routed through `session/request_permission`.
- **2026-05-30** (PR #5995): Serve fast-path bundle closure guard.
- **2026-05-15** (v0.19.x early): ACP-bridge package extracted to `@qwen-code/acp-bridge` (was inline in `serve/`).
- **2026-04-22** (issue #5023): `exitPlanMode.ts` tool filtering — plan-mode reminder now properly restricts the agent to read-only tools in ACP mode (resolves long-standing #1151).

## Rust Client Example

This example uses the official `agent-client-protocol` Rust crate and the `qwen --acp` launch command:

```toml
[dependencies]
agent-client-protocol = "1"
tokio = { version = "1", features = ["full"] }
```

```rust
use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ClientCapabilities, ContentBlock, FileSystemCapabilities, Implementation,
        InitializeRequest, NewSessionRequest, PromptRequest, SessionNotification,
        TextContent, TerminalCapabilities,
    },
};
use agent_client_protocol::{AcpAgent, Client};
use std::process::Stdio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = tokio::process::Command::new("qwen")
        .arg("--acp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();
    // stderr is captured separately for diagnostics (the CLI writes its own logs there)
    let stderr = child.stderr.take().unwrap();

    let (conn, handle_io) = acp::ClientSideConnection::new(
        MyClient::new(),
        outgoing,
        incoming,
        |fut| { tokio::spawn(fut); },
    );
    tokio::spawn(handle_io);

    let init_response = conn.initialize(InitializeRequest {
        protocol_version: ProtocolVersion::V1,
        client_capabilities: ClientCapabilities {
            fs: Some(FileSystemCapabilities {
                read_text_file: true,
                write_text_file: true,
            }),
            terminal: Some(TerminalCapabilities::default()),
        },
        client_info: Some(Implementation {
            name: "claudine-qwen-client".into(),
            title: Some("Claudine Qwen ACP Client".into()),
            version: "0.1.0".into(),
        }),
    }).await?;

    log::info!("Connected to: {:?}", init_response.agent_info);

    let session = conn.new_session(NewSessionRequest {
        cwd: std::env::current_dir()?,
        mcp_servers: Vec::new(),
    }).await?;

    let result = conn.prompt(PromptRequest {
        session_id: session.session_id.clone(),
        prompt: vec![ContentBlock::Text(TextContent::new(
            "Summarize this project in two sentences.".into(),
        ))],
    }).await?;

    log::info!("Prompt completed: {:?}", result.stop_reason);
    Ok(())
}
```

For HTTP transport (the `qwen serve` daemon's `/acp` route), use `agent-client-protocol-http` instead of stdio. See the `claudine/docs/research/acp/...` daemon 03 doc for the HTTP envelope.

## Rust Reverse Request Handling

`qwen` issues `session/request_permission` and (when the client advertises FS capabilities) `fs/read_text_file` / `fs/write_text_file`. The example below implements the full reverse-request surface:

```rust
use agent_client_protocol::schema::v1::{
    ReadTextFileRequest, ReadTextFileResponse, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
    WriteTextFileRequest, WriteTextFileResponse,
};
use std::path::{Path, PathBuf};

fn sandbox(path: &Path, root: &Path) -> anyhow::Result<PathBuf> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !canonical.starts_with(root) {
        anyhow::bail!("path {} is outside project root {}", canonical.display(), root.display());
    }
    Ok(canonical)
}

async fn handle_permission(
    request: RequestPermissionRequest,
) -> anyhow::Result<RequestPermissionResponse> {
    let option_id = request
        .options
        .first()
        .map(|o| o.option_id.clone())
        .unwrap_or_default();

    Ok(RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
        SelectedPermissionOutcome::new(option_id),
    )))
}

async fn handle_read(
    request: ReadTextFileRequest,
    root: PathBuf,
) -> anyhow::Result<ReadTextFileResponse> {
    let path = sandbox(&request.path, &root)?;
    let content = tokio::fs::read_to_string(&path).await?;

    let filtered = match (request.line, request.limit) {
        (Some(start), Some(limit)) => content
            .lines()
            .skip((start as usize).saturating_sub(1))
            .take(limit as usize)
            .collect::<Vec<_>>()
            .join("\n"),
        (Some(start), None) => content
            .lines()
            .skip((start as usize).saturating_sub(1))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => content,
    };

    Ok(ReadTextFileResponse { content: filtered })
}

async fn handle_write(
    request: WriteTextFileRequest,
    root: PathBuf,
) -> anyhow::Result<WriteTextFileResponse> {
    let path = sandbox(&request.path, &root)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let content = if request.meta.bom { "\u{FEFF}".to_string() + &request.content } else { request.content };
    tokio::fs::write(&path, &content).await?;
    Ok(WriteTextFileResponse {})
}
```

Register handlers on the builder before `connect_with`:

```rust
Client
    .builder()
    .on_receive_request(
        |request: RequestPermissionRequest, responder, _cx| async move {
            responder.respond(handle_permission(request).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
    .on_receive_request(
        |request: ReadTextFileRequest, responder, _cx| async move {
            let root = std::env::current_dir().unwrap();
            responder.respond(handle_read(request, root).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
    .on_receive_request(
        |request: WriteTextFileRequest, responder, _cx| async move {
            let root = std::env::current_dir().unwrap();
            responder.respond(handle_write(request, root).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
```

## Rust Host Command Handling

Implement the terminal lifecycle even though `qwen` does not normally delegate via `terminal/create`. The Qwen Code daemon's HTTP bridge may issue these when running under `qwen serve`, and other agents in the same code path will use them:

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

The remaining handlers (`terminal/output`, `wait_for_terminal_exit`, `kill_terminal_command`, `release_terminal`) follow the same pattern. Always call `terminal/release` and kill the process if it is still running — handle leaks are a frequent production foot-gun.

## Rust Desktop Streaming Bridge

To stream ACP events into a desktop UI, run the ACP client on a dedicated thread and forward `SessionNotification` values through an `mpsc` channel:

```rust
use agent_client_protocol::schema::v1::{
    ClientCapabilities, ContentBlock, FileSystemCapabilities, Implementation,
    InitializeRequest, NewSessionRequest, PromptRequest, SessionNotification, TextContent,
};
use agent_client_protocol::{Client, SessionUpdate};
use std::process::Stdio;
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextChunk(String),
    ThoughtChunk(String),
    ToolCallStarted { id: String, title: String, kind: String },
    ToolCallFinished { id: String, status: String },
    PlanReceived { entries: usize },
    AvailableCommandsUpdate(Vec<String>),
    CurrentModeUpdate(String),
    TurnComplete { stop_reason: String, usage: Option<UsageMeta> },
    PermissionRequest {
        request_id: String,
        title: String,
        options: Vec<(String, String)>,
        response_tx: tokio::sync::oneshot::Sender<String>,
    },
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub struct UsageMeta {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub thought_tokens: u64,
    pub total_tokens: u64,
}

pub fn spawn_agent(project_dir: PathBuf)
    -> anyhow::Result<(mpsc::UnboundedReceiver<AgentEvent>, mpsc::UnboundedSender<String>)>
{
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (prompt_tx, mut prompt_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async move {
            let mut child = tokio::process::Command::new("qwen")
                .arg("--acp")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .expect("spawn qwen");

            let outgoing = child.stdin.take().unwrap().compat_write();
            let incoming = child.stdout.take().unwrap().compat();

            let (conn, handle_io) = acp::ClientSideConnection::new(
                ChannelClient::new(event_tx.clone()),
                outgoing,
                incoming,
                |fut| { tokio::spawn(fut); },
            );
            tokio::spawn(handle_io);

            conn.initialize(InitializeRequest {
                protocol_version: acp::ProtocolVersion::V1,
                client_capabilities: ClientCapabilities {
                    fs: Some(FileSystemCapabilities { read_text_file: true, write_text_file: true }),
                    terminal: None,
                },
                client_info: Some(Implementation {
                    name: "desktop-app".into(),
                    title: Some("Desktop App".into()),
                    version: "0.1.0".into(),
                }),
            }).await.expect("init");

            let session = conn.new_session(NewSessionRequest {
                cwd: project_dir,
                mcp_servers: Vec::new(),
            }).await.expect("session");

            while let Some(prompt) = prompt_rx.recv().await {
                let result = conn.prompt(PromptRequest {
                    session_id: session.session_id.clone(),
                    prompt: vec![ContentBlock::Text(TextContent::new(prompt))],
                }).await;
                let _ = event_tx.send(match result {
                    Ok(r) => AgentEvent::TurnComplete {
                        stop_reason: format!("{:?}", r.stop_reason),
                        usage: None,
                    },
                    Err(e) => AgentEvent::Error(e.to_string()),
                });
            }
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

fn listen(mut event_rx: mpsc::UnboundedReceiver<AgentEvent>, handle: tauri::AppHandle) {
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                AgentEvent::TextChunk(text) => handle.emit("agent:text", text).ok(),
                AgentEvent::TurnComplete { stop_reason, .. } => handle.emit("agent:done", stop_reason).ok(),
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

Adding ACP-based Qwen Code support to Claudine requires:

1. **Launch detection** — spawn `qwen --acp` directly (no bridge process). Verify `qwen --version` ≥ 0.15.6 and that `which qwen` resolves. For HTTP transport, spawn `qwen serve` (HTTP) per the `qwen serve` protocol reference.

2. **Capability negotiation** — advertise `fs.readTextFile: true`, `fs.writeTextFile: true`, and `terminal: true`. The agent's `setupFileSystem` will route read/write through the client when those capabilities are advertised; the `terminal` capability is needed for forward-compat with the `qwen serve` ACP-over-HTTP bridge.

3. **Reverse-request routing** — implement:
   - `session/request_permission` (REQUIRED — handles 100% of approval prompts including subagent and plan-mode confirmations).
   - `fs/read_text_file` (REQUIRED if `fs.readTextFile: true` is advertised).
   - `fs/write_text_file` (REQUIRED if `fs.writeTextFile: true` is advertised).
   - `terminal/create` through `terminal/release` (for `qwen serve` HTTP and for any sandboxed embed).

4. **Streaming bridge** — forward `session/update` notifications into Claudine's lifecycle pipeline so TTS, sound effects, logging, and messenger actions can react. Group updates by `ContentChunk.message_id`. Handle `available_commands_update` (refresh slash-command palette), `current_mode_update` (refresh mode pill), and `plan` updates (refresh plan list). Surface token usage from `_meta.usage` on the final chunk.

5. **Authentication preconditions** — require `OPENAI_API_KEY` or pre-authenticated CLI session before allowing non-interactive ACP launches. Honor `authMethods[]` from the agent's `initialize` response and drive `authenticate` rather than waiting on `auth_required`. The `getAccountInfo` extMethod is a clean way to surface the resolved credential name without leaking the secret value.

6. **Stderr discipline** — never write to stdout from within a Claude-routed library; capture stderr for diagnostics but the stdout stream belongs to ACP. Claudine should keep this invariant for any library that may be loaded inside the ACP child's library context.

7. **Approval mode policy** — map `--approval-mode yolo` to "no prompts", `plan` to "read-only tools only", `default` to "prompt for everything", `auto-edit` to "auto-approve file edits, prompt for shell". Reflect these in the UI when the host mode is exposed via `current_mode_update`.

8. **HTTP transport alternative** — when launching `qwen serve`, the same ACP JSON-RPC messages flow over HTTP via the `agent-client-protocol-http` sub-crate. Per-session event rings, a four-policy permission mediator, and per-session FIFO queue are part of the bridge. The HTTP transport is preferred for Claudine's persistent desktop session model.

## Changelog

- **2026-07-03**: Refreshed for current Qwen Code (latest release v0.19.6; locally verified 0.15.6). Direct probe of `qwen --acp` confirmed native ACP mode — `initialize` returns `protocolVersion: 1`, `agentInfo: qwen-code/0.15.6`, `authMethods[]`, and a full `agentCapabilities` block. Empirical reverse-request observation: `fs/read_text_file` was issued during a read-file prompt, confirming the client-side FS delegation works. Source inspection of the bundled `@agentclientprotocol/sdk` confirmed the full method surface (`session/cancel`, `session/fork`, `session/list`, `session/load`, `session/new`, `session/prompt`, `session/resume`, `session/set_config_option`, `session/set_mode`, `session/set_model`, `session/request_permission`, `fs/read_text_file`, `fs/write_text_file`, `terminal/create`, `terminal/kill`, `terminal/output`, `terminal/release`, `terminal/wait_for_exit`, `session/update`). **Updated the prior "partial" classification to `native`** based on direct local verification and source inspection. Added a Quirk noting that `console.log/info/debug` are rebound to `console.error` so library output does not corrupt stdout. Documented the qwen-oauth free-tier discontinuation (2026-04-15). Added the `qwen serve` HTTP bridge (`packages/acp-bridge/`) as a transport alternative. Added the new extMethods (`deleteSession`, `renameSession`, `getAccountInfo`) and the `authenticate/update` extNotification. Recorded ACP-related bug fixes from late June 2026: skill reads returning `[object Object]` (#6020), loop on invalid tool params (#6075), image prompts bypassing vision bridge (#6110), empty memory in long sessions (#5968), `/cd` slash command in ACP (#5903), defer-session-creation-on-first-prompt (v0.19.5), and the bootstrap-fast-paths perf improvement (v0.19.6).
- **2026-02-21**: Initial release of this research document (per the prior `claudine sequence` run, classified as `partial` based on docs at that time).

## Sources

- [Qwen Code user guide](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [`QwenLM/qwen-code` GitHub repository](https://github.com/QwenLM/qwen-code)
- [Qwen Code 0.15.6 release](https://github.com/QwenLM/qwen-code/releases/tag/v0.15.6) (locally verified)
- [Qwen Code 0.19.6 release (latest)](https://github.com/QwenLM/qwen-code/releases/tag/v0.19.6)
- [Zed Editor integration](https://qwenlm.github.io/qwen-code-docs/en/users/integration-zed/)
- [JetBrains integration](https://qwenlm.github.io/qwen-code-docs/en/users/integration-jetbrains/)
- [ACP Bridge architecture (developer doc)](https://qwenlm.github.io/qwen-code-docs/en/developers/daemon/03-acp-bridge/)
- [Capabilities & Protocol Versioning (developer doc)](https://qwenlm.github.io/qwen-code-docs/en/developers/daemon/11-capabilities-versioning/)
- [`packages/cli/src/acp-integration/acpAgent.ts` source](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/acp-integration/acpAgent.ts) (locally verified in 0.15.6 bundle)
- [`packages/cli/src/acp-integration/session/Session.ts` source](https://github.com/QwenLM/qwen-code/blob/main/packages/cli/src/acp-integration/session/Session.ts)
- [Agent Client Protocol specification](https://agentclientprotocol.com/)
- [ACP schema reference](https://agentclientprotocol.com/protocol/schema)
- [ACP Initialization](https://agentclientprotocol.com/protocol/initialization)
- [ACP Session Setup](https://agentclientprotocol.com/protocol/session-setup)
- [`agentclientprotocol/sdk` on npm](https://www.npmjs.com/package/@agentclientprotocol/sdk) (bundled in qwen-code 0.15.6)
- [`agent-client-protocol` Rust crate on docs.rs](https://docs.rs/agent-client-protocol/)
- [Issue #6110 — ACP image prompts bypass vision bridge](https://github.com/QwenLM/qwen-code/issues/6110)
- [Issue #6075 — ACP daemon loop on bad params](https://github.com/QwenLM/qwen-code/issues/6075)
- [Issue #6057 — Daemon session archive](https://github.com/QwenLM/qwen-code/issues/6057)
- [Issue #6020 — `read_file` `[object Object]` for ACP skill reads](https://github.com/QwenLM/qwen-code/issues/6020)
- [Issue #5968 — Empty memory in long sessions](https://github.com/QwenLM/qwen-code/issues/5968)
- [Issue #5861 — Compression stream=true to avoid timeout](https://github.com/QwenLM/qwen-code/issues/5861)
- [PR #5903 — ACP /cd command](https://github.com/QwenLM/qwen-code/pull/5903)
- [PR #5995 — Serve fast-path bundle closure guard](https://github.com/QwenLM/qwen-code/pull/5995)
- [Local source — `cli.js` in `/opt/homebrew/Cellar/qwen-code/0.15.6/`](file:///opt/homebrew/Cellar/qwen-code/0.15.6/libexec/lib/node_modules/@qwen-code/qwen-code/cli.js) (verified: `@agentclientprotocol/sdk` bundled at line 392509; `PROTOCOL_VERSION = 1` at line 392539; `QwenAgent.initialize` returns the documented capability envelope at line 548220; `AcpFileSystemService` at line 546007; `toPermissionOptions` at line 546155; `runAcpAgent` at line 548131)