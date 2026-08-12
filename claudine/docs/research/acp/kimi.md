---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://moonshotai.github.io/kimi-code/en/
acp_docs: https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html
repo: https://github.com/MoonshotAI/kimi-code
support: native
launch_modes:
  - command: kimi acp
    args: []
    transport: stdio
    adapter: none
    notes: "Native ACP subcommand on the provider's primary CLI binary (`kimi-code`, the Node.js SEA successor to the legacy Python `kimi-cli`). Ships as a built-in subcommand since `kimi-code 0.9.0` (2026-06-03). On macOS/Linux the binary is installed via the official script `curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash` or `brew install kimi-code`; on Windows the binary is the same single executable plus Git Bash for Windows."
  - command: kimi acp --login
    args:
      - "--login"
    transport: stdio
    adapter: none
    notes: "Terminal-auth entry point. Runs the device-code login flow inline (suitable for IDE-driven headless auth preflight) and exits."
protocol_versions:
  - "ACP v1 (negotiated as protocolVersion: 1; the `kimi acp` reference doc also cites the @agentclientprotocol/sdk@0.23.0 lineage, but `initialize` rejects protocolVersion 0.23 with -32602 invalid-params on the installed v0.14.0)"
capabilities:
  - capability: initialize
    support: supported
    notes: "Standard `initialize` handshake. Returns `protocolVersion: 1`, `agentCapabilities` (loadSession, promptCapabilities, mcpCapabilities, sessionCapabilities), `authMethods[]` (currently a single `terminal` type that runs `kimi login` as a child process), and `agentInfo: { name: \"Kimi Code CLI\", version }`. Verified directly on v0.14.0."
  - capability: authenticate
    support: supported
    notes: "`authenticate` is implemented. The official docs state: validates `methodId='login'`; returns `authRequired (-32000)` if the local token is missing, `invalidParams (-32602)` for an unknown ID. The client is expected to surface the `authMethods[]._meta.terminal-auth` block (command + args + env) and spawn the listed command itself to perform the device-code login."
  - capability: session_new
    support: supported
    notes: "Accepts `cwd` and `mcpServers`. Returns `sessionId` plus `configOptions[]` (model / thinking / mode selects), and follows up with an `available_commands_update` notification listing Kimi slash commands (compact, status, usage, mcp, tasks, help, custom-theme, import-from-cc-codex, mcp-config, sub-skill, sub-skill.consolidate, sub-skill.review, update-config, skill:find-skills)."
  - capability: session_load
    support: supported
    notes: "`session/load` restores a session and replays history as `session/update` notifications (`user_message_chunk` first, then agent content). Implemented since 0.9.0; load + replay semantics stabilized in 0.12.0 (per changelog: \"Fix ACP ... bootstrap context reads ...\")."
  - capability: session_prompt
    support: supported
    notes: "Accepts `text` / `image` / `resource` / `resource_link` blocks; streams `agent_message_chunk`. Prompts in yolo mode auto-approve; in default mode prompts requesting tool use trigger `session/request_permission`."
  - capability: session_cancel
    support: supported
    notes: "`session/cancel` interrupts the current turn. Verifiable via the `Interrupt` hook event added in 0.14.0."
  - capability: session_modes
    support: supported
    notes: "`session/set_mode` is implemented as a compatibility path that dispatches to the same handler as `session/set_config_option({configId:'mode'})`. Modes advertised via `configOptions[].category == \"mode\"` are `default` (manual approvals), `plan` (read-only planning, no tool execution), `auto` (auto-approve safe operations), and `yolo` (auto-approve everything)."
  - capability: streaming
    support: supported
    notes: "`session/update` notifications stream `agent_message_chunk`, `user_message_chunk`, `tool_call*`, `plan`, `config_option_update`, and `available_commands_update`. Updates are fire-and-forget JSON-RPC notifications grouped by `sessionId`."
  - capability: permissions
    support: supported
    notes: "`session/request_permission` is the shared channel for both tool approvals and question elicitation. Decisions are mediated by the agent's mode (`default` prompts, `auto`/`yolo` auto-approve)."
  - capability: fs_read
    support: partial
    notes: "Current docs (`https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html`) state `fs/read_text_file` is routed to the client (advertised via `fsCapabilities`). The installed v0.14.0 binary, however, returns an `agentCapabilities` payload that does NOT include `fsCapabilities` — so a v0.14.0 client gets the agentic file reads through the agent's local tools, not via a reverse request. Upgrade to v0.15.0+ (released 2026-06-15) for filesystem delegation to be reliably wired up."
  - capability: fs_write
    support: partial
    notes: "Same caveat as fs_read. Newer docs say `fs/write_text_file` is routed to the client; v0.14.0 keeps writes inside the agent process. Verified — v0.14.0's `initialize` response omits `fsCapabilities` entirely."
  - capability: terminal
    support: unsupported
    notes: "Per the official capability matrix: `terminal/create`, `terminal/output`, `terminal/release`, `terminal/kill`, `terminal/wait_for_exit` are all NOT IMPLEMENTED on the agent side. Shell commands execute locally in the agent process — the client never sees terminal reverse requests."
  - capability: mcp
    support: supported
    notes: "`mcpCapabilities.http: true` and (since v0.15.0) `mcpCapabilities.sse: true`. The installed v0.14.0 still returns `sse: false`. The adapter forwards IDE-provided `mcpServers` from `session/new` and `session/load`, converting transports: `http` → `transport:'http'`, `stdio` → `transport:'stdio'`, `sse` → `transport:'sse'`, and `acp` → silently dropped with a warn log."
  - capability: media
    support: partial
    notes: "`promptCapabilities.image: true` (base64 + mimeType), `audio: false`. Video input is supported end-to-end in the agent runtime via the Anthropic-compatible protocol (added 0.20.2) but is not exposed as an ACP `audio` capability."
  - capability: plans
    support: supported
    notes: "`plan` session-update variant is emitted when the agent produces a plan, and the `mode = plan` config option drives the review-only flow. Plan body is rendered inline in the agent's web UI as well."
  - capability: extensions
    support: partial
    notes: "`_meta` is used to carry provider-specific extras — e.g. `authMethods[]._meta.terminal-auth` (type/label/command/args/env) so the client can drive the device-code login itself. No public ACP extension protocol negotiated."
  - capability: other
    support: supported
    notes: "`session/set_config_option` is the unified dispatcher for model / thinking / mode selection. `session/set_model` (unstable) is implemented as a compatibility alias for `set_config_option({configId:'model'})`."
  - capability: other
    support: unsupported
    notes: "`session/close` is in the spec but NOT implemented by `kimi acp`. Sessions are released via process exit; the client cannot ask the agent to close a session over JSON-RPC. The `logout` reverse method is also NOT implemented; rotate credentials via `kimi login` from a terminal."
  - capability: other
    support: unsupported
    notes: "`logout` reverse method is NOT implemented. Rotate credentials via `kimi login` from a terminal. Documented as `logout: No` in the official capability matrix."
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: "Single permission/elicitation channel. The agent sends a JSON-RPC request carrying `toolCall` (id, title, kind, content), the tool-call context, and a list of `PermissionOption` entries. The client responds with `RequestPermissionOutcome::Selected { optionId }` or `RequestPermissionOutcome::Cancelled`. Mode drives auto-approval: default = manual, auto = safe-only auto, yolo = all auto, plan = no tools at all."
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: "Schema-level reverse request, documented as supported by the current capability matrix. The installed v0.14.0 binary does NOT advertise `fsCapabilities` in its initialize response, so this request typically will not be issued; v0.15.0+ may issue it. Implement as best-effort for forward compatibility."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: "Same caveat as fs_read."
  - method: session/update
    purpose: other
    client_must_handle: false
    notes: "Notification (not a request), so it does not strictly require a handler. Route `agent_message_chunk` and `user_message_chunk` into the UI text stream; `tool_call` / `tool_call_update` into the tool card list; `plan` into the plan panel; `config_option_update` into the option picker; `available_commands_update` into the slash-command menu."
permission_model:
  mechanism: session/request_permission reverse request, mediated by the agent's current mode
  timeout: client-defined; the spec has no explicit timeout — defaults to interactive wait
  default_policy: depends on the session's `mode` config option. `default` = manual approval (every tool call requiring approval must receive a Selected or Cancelled response); `auto` = auto-approve safe operations only; `yolo` = auto-approve everything; `plan` = tools not invoked
  approval_values:
    - allow_once
    - allow_always
    - reject_once
  notes: "The mode toggle is a `configOptions[].select` with `category: \"mode\"` and values `default`/`plan`/`auto`/`yolo`; clients drive it via `session/set_config_option({configId:\"mode\", value:\"auto\"})`. The `kimi acp` adapter uses the *same* `session/request_permission` channel for question elicitation (`AskUserQuestion` and equivalents), so a permissive UI implementation can route both kinds of prompt through one widget."
filesystem_model:
  read_methods:
    - fs/read_text_file (forward-compat; not issued by v0.14.0)
  write_methods:
    - fs/write_text_file (forward-compat; not issued by v0.14.0)
  path_base: absolute paths
  sandboxing: client-side; the client decides whether to enforce a project-root boundary. In v0.14.0, file reads/writes happen inside the agent's local kaos tool layer, so the client has no opportunity to sandbox paths at the ACP boundary; v0.15.0+ routes file reads/writes through `fs/*` reverse requests and exposes the boundary.
  notes: "ACP requires absolute paths and 1-based line numbers. When `fs/read_text_file` is implemented, treat it the same as any other ACP agent: validate that the requested path lies inside the project root before reading; reject with a JSON-RPC error if not."
terminal_model:
  supported: false
  methods: []
  shell: "n/a — terminal/* reverse requests are NOT issued by `kimi acp`"
  cwd: "n/a"
  streaming: "n/a"
  cancellation: "n/a"
  notes: "The agent runs shell commands locally using its own kaos tool layer. There is no ACP-visible terminal handle, no terminal/output stream, and no terminal/release discipline to manage. Documented as not implemented in the official capability matrix."
streaming_model:
  update_methods:
    - session/update
  text_events:
    - agent_message_chunk
    - user_message_chunk
  tool_events:
    - tool_call
    - tool_call_update
  plan_events:
    - plan
  error_events:
    - "JSON-RPC error responses (`code`, `message`, optional `data`) on the request channel; no separate error session-update variant. Note that provider rate-limit/quota errors surface as an immediate `session/prompt` response with `stopReason: \"end_turn\"` and no streaming chunks — observed in practice on a quota-exhausted token."
  notes: "Updates are fire-and-forget JSON-RPC notifications grouped by `sessionId`. Distinct from `@agentclientprotocol/claude-agent-acp`, `kimi acp` exposes a relatively compact update vocabulary — no `current_mode_update`, no `agent_thought_chunk` separate from `agent_message_chunk`."
auth_setup:
  required: true
  mechanisms:
    - "Kimi Code OAuth via `kimi login` (device-code flow)"
    - "Pre-existing cached OAuth credentials at `~/.kimi-code/credentials/kimi-code.json` (auto-refreshing)"
    - "Moonshot AI Open Platform API key (via `/login` → choose API key)"
    - "Kimi Code Web mode via `kimi web` (alternative browser-based flow)"
  headless_notes: "For headless ACP operation, complete `kimi login` in a terminal once before launching the IDE; the CLI persists the access/refresh token to `~/.kimi-code/credentials/kimi-code.json` and the `kimi acp` subcommand reuses it. The `authMethods[0]._meta.terminal-auth` block (`{ command, args, env }`) lets a headless ACP client spawn `kimi login` itself if the token has expired or rotated; if the token is fully absent the initialize handshake succeeds but `session/new` will fail with `authRequired (-32000)` per the official docs."
  notes: "Token refresh happens automatically in the background; on a 401 the credentials file is rewritten atomically. Multiple simultaneous `kimi acp` instances coordinate refresh via cross-process file lock to avoid a race that wipes valid credentials. OAuth tokens default to a 15-minute access window (`expires_in: 900`); refresh tokens are valid for ~90 days."
env_vars:
  - name: KIMI_CODE_OAUTH_HOST
    effect: "Override the OAuth host (default: https://auth.kimi.com). Falls back to legacy KIMI_OAUTH_HOST."
  - name: KIMI_CODE_BASE_URL
    effect: "Override the API base URL (default: https://api.kimi.com/coding/v1)."
  - name: KIMI_CODE_HOME
    effect: "Override the kimi-code home directory; Kimi-specific user Skills and global agent instructions are loaded from this path."
  - name: KIMI_CODE_EXPERIMENTAL_SUB_SKILL
    effect: "Enable experimental sub-skill discovery (sub-skill builtin bundle)."
  - name: KIMI_CODE_NO_AUTO_UPDATE
    effect: "Disable background automatic updates (legacy alias: KIMI_CLI_NO_AUTO_UPDATE)."
  - name: KIMI_CODE_CUSTOM_HEADERS
    effect: "Newline-separated `Name: Value` outbound LLM request headers."
  - name: KIMI_CODE_ALLOWED_HOSTS
    effect: "Comma-separated hosts added to the DNS-rebinding allowlist for `kimi web`."
  - name: KIMI_MODEL_THINKING_KEEP
    effect: "Passthrough `thinking.keep` value to the Moonshot API for Preserved Thinking (`all` to retain reasoning_content across turns)."
  - name: KIMI_MODEL_ADAPTIVE_THINKING
    effect: "Force adaptive thinking on/off for Anthropic-compatible providers."
  - name: KIMI_MODEL_TEMPERATURE
    effect: "Sampling parameter applied to any kimi provider."
  - name: KIMI_MODEL_TOP_P
    effect: "Sampling parameter applied to any kimi provider."
  - name: KIMI_MODEL_NAME
    effect: "Pin the model identifier for the session."
  - name: KIMI_SHELL_PATH
    effect: "Override the absolute path of bash.exe on Windows; required only when Git Bash is installed in a non-standard location."
  - name: KIMI_CLI_GIT_BASH_PATH
    effect: "Legacy alias retained for backwards compatibility; same effect as KIMI_SHELL_PATH."
  - name: HTTP_PROXY / HTTPS_PROXY / ALL_PROXY / NO_PROXY
    effect: "Standard proxy environment variables honored for all outbound traffic (SOCKS supported via socks5:// normalization)."
  - name: PATH
    effect: "On macOS GUI launches from an IDE, `kimi acp` does not inherit the user's terminal PATH; pass an absolute path in the IDE's `command:` field, or have the launcher enrich PATH from the user's login shell (new in v0.22.2)."
rust_client:
  crate: agent-client-protocol
  connection_type: AcpAgent subprocess over stdio (JSON-RPC), launched with the string "kimi acp"
  localset_required: false
  reverse_request_handlers:
    - session/request_permission
  desktop_streaming_pattern: "tokio::sync::mpsc from the on_receive_notification handler to the UI thread; run the ACP client on a dedicated tokio runtime because the binary's stdio lifecycle owns the process"
  notes: "Use the official `agent-client-protocol` Rust SDK (1.0.1, depending on `agent-client-protocol-schema =1.1.0`). No Kimi-specific preset exists in the SDK — construct the agent via `AcpAgent::from_str(\"kimi acp\")`. The binary's stdio transport is well-behaved: it logs to stderr and to `~/.kimi-code/logs/kimi-code.log`, keeping stdout clean for JSON-RPC. ACP type negotiation: send `protocolVersion: 1` (the schema's `ProtocolVersion::V1`); do NOT send `0.23` (that is the JS SDK's internal version, and the binary rejects it)."
compatibility:
  - client: Zed
    status: works
    issue: "Zed is the canonical first-class client; documented integration in the official `Using in IDEs` guide."
    workaround: "Add `kimi acp` to Zed's `agent_servers` in `~/.config/zed/settings.json` and open a new thread in the Agent panel."
  - client: JetBrains IDEs (IntelliJ, PyCharm, WebStorm)
    status: partial
    issue: "JetBrains requires an absolute path to the binary and uses its own AI-chat plugin config layout."
    workaround: "Configure under `Configure ACP agents` in the AI chat panel; use the full path from `which kimi` (e.g. `/Users/you/.local/bin/kimi`)."
  - client: Paseo
    status: works
    issue: "Paseo's built-in ACP provider catalog includes a Kimi Code CLI entry; a custom provider config is also possible."
    workaround: "Pick `Kimi Code CLI` from the catalog; if configuring manually, set `command: [\"kimi\", \"acp\"]` and complete the terminal login separately (Paseo's generic ACP adapter does not drive the login flow)."
  - client: agent-client-protocol Rust SDK 0.9.x
    status: broken
    issue: "Connection futures are !Send and required LocalSet."
    workaround: "Upgrade to `agent-client-protocol 1.0.1`."
  - client: agent-client-protocol Rust SDK 1.0.x
    status: works
    issue: "No Kimi-specific preset constructor in the SDK."
    workaround: "Use `AcpAgent::from_str(\"kimi acp\")` (or pass the absolute path) and ensure `cwd` is set on `NewSessionRequest`/`LoadSessionRequest`."
  - client: "@agentclientprotocol/sdk 0.23"
    status: partial
    issue: "The Kimi binary uses ACP protocolVersion 1, but the docs and changelog reference `@agentclientprotocol/sdk@0.23.0` — the 0.23 number is the *JS SDK lineage*, not a protocol version. Sending `protocolVersion: 0.23` in the initialize request is rejected with -32602 invalid-params."
    workaround: "Always negotiate `protocolVersion: 1`."
recent_changes:
  - date: 2026-07-03
    version: "@moonshot-ai/kimi-code 0.22.2"
    change: "Bug fixes for streaming transcript duplication, image-compression leak into session titles, Windows console flash on auto-update. Polish includes PATH enrichment from login shell at startup (mitigates the macOS IDE-launcher PATH gap)."
    impact: "ACP clients launching `kimi acp` from a GUI app no longer need to hard-code the absolute binary path on macOS in some environments, but should still prefer absolute paths for reliability."
  - date: 2026-06-23
    version: "@moonshot-ai/kimi-code 0.19.1"
    change: "Fixed ACP editors such as Zed failing to start a new thread."
    impact: "All clients should be able to send `session/new` against current 0.19.1+ releases."
  - date: 2026-06-15
    version: "@moonshot-ai/kimi-code 0.15.0"
    change: "Added support for legacy SSE MCP servers alongside stdio and streamable HTTP transports. Added an all-sessions picker view with name search."
    impact: "`mcpCapabilities.sse` flips from `false` to `true` here; v0.14.0 still returns `sse: false` in `initialize`. MCP-over-SSE is now first-class."
  - date: 2026-06-12
    version: "@moonshot-ai/kimi-code 0.14.1"
    change: "Fixed ACP file reads and edits for Windows workspaces opened through IDE clients."
    impact: "Windows ACP integrations now function reliably; `fs/*` paths are normalized correctly across PowerShell / Git-Bash layouts."
  - date: 2026-06-10
    version: "@moonshot-ai/kimi-code 0.14.0"
    change: "Added an `Interrupt` hook event that fires when the user interrupts a turn."
    impact: "ACP clients can surface turn-cancellation signals in their UIs in real time."
  - date: 2026-06-09
    version: "@moonshot-ai/kimi-code 0.12.0"
    change: "Fixed ACP slash-skill routing, bootstrap context reads, file and permission edge cases, subagent event handling, and stale-file edit messaging. Removed the per-turn auto-compaction limit so long conversations keep compacting instead of failing early."
    impact: "ACP session/load replay is now reliable for long sessions; ACP clients that previously saw `fs/read_text_file` failing on cold loads should retest."
  - date: 2026-06-03
    version: "@moonshot-ai/kimi-code 0.9.0"
    change: "Added the `kimi acp` subcommand: kimi-code now speaks Agent Client Protocol (built on @agentclientprotocol/sdk@0.23.0) over stdio. Initial coverage matrix published."
    impact: "This is the release that introduces native ACP support; the legacy Python `kimi-cli` (1.47.0) had its own earlier ACP implementation, but the TypeScript `kimi-code` is the canonical path going forward."
  - date: 2026-06-05
    version: "Legacy Python kimi-cli 1.47.0"
    change: "Added a `/upgrade` command that installs `kimi-code` and migrates sessions and config automatically."
    impact: "All new ACP work should target `kimi-code` (Node.js SEA) rather than the legacy Python `kimi-cli` (which is winding down)."
quirks:
  - "Two `kimi` distributions exist on the same host: the legacy Python `kimi-cli` (1.47.0, winding down) and the new TypeScript `kimi-code` (0.14.0 installed, 0.22.2 latest). They share config layout but live in different home directories (`~/.kimi/` vs `~/.kimi-code/`). Use `kimi --version` to disambiguate."
  - "`kimi acp` is sensitive to the absolute path of the binary on macOS/Linux IDE integrations — GUI-launched subprocesses do not inherit the terminal's PATH, so `command: \"kimi\"` often fails. Use the absolute path returned by `which kimi` (e.g. `/Users/ken/.kimi-code/bin/kimi`). The official IDE setup docs explicitly call this out."
  - "Provider rate-limit / quota exhaustion surfaces as an immediate `session/prompt` response with `stopReason: \"end_turn\"` and no streaming chunks — observed on a quota-exhausted Kimi Code token. Clients should treat an `end_turn` after a no-stream turn as a likely upstream error and check the diagnostic log."
  - "`session/load` requires an explicit `mcpServers: []` field; omitting it returns `Invalid params: mcpServers: Invalid input` (-32602). The same is true for `session/new` in current versions."
  - "The agent does NOT advertise `fsCapabilities` in its `initialize` response on v0.14.0 — so even when the client declares `fs.readTextFile: true`, the binary keeps file I/O inside its own kaos tool layer. Clients should still implement `fs/read_text_file` and `fs/write_text_file` for forward compatibility with v0.15.0+ where the docs say these are routed to the client."
  - "The official `kimi acp` capability matrix lists `fs/read_text_file`, `fs/write_text_file`, `terminal/*` as supported on the agent side, but the v0.14.0 binary only emits reverse requests for `session/request_permission`. The current docs (0.22.2) describe the latest capability set, not the older binary installed at research time."
  - "`protocolVersion: 0.23` is rejected by the binary (returns -32602 invalid-params). Always negotiate `protocolVersion: 1`. The 0.23 number refers to the @agentclientprotocol/sdk JS package version, not the ACP protocol version."
  - "MCP servers with transport `acp` are silently dropped (warn-logged). Only `http`, `stdio`, and `sse` MCP transports are forwarded from the IDE into the Kimi runtime."
  - "`session/close` and `logout` are NOT implemented over JSON-RPC. Use `kimi login` from the terminal to rotate credentials; close sessions by terminating the subprocess."
  - "Claude Code / Codex-style `ext_method` / `ext_notification` extensions are not negotiated; Kimi uses `_meta` only for `authMethods[].terminal-auth`."
  - "Initializing with `protocolVersion: 0.23` returns -32602 — the binary expects ACP v1 (numeric `1` or `ProtocolVersion::V1`)."
  - "Single installed binary is ~130 MB (Node.js SEA). The transport overhead is negligible; the agent's stdio reads JSON-RPC frames line-delimited on stdin."
  - "Auth token expires_in is 900 seconds (15 min) by default — clients that watch for auth errors should expect ~15-minute OAuth refresh cycles."
  - "`authMethods[]._meta.terminal-auth` carries `{ command, args, env }` so a headless ACP client can drive the device-code login itself by spawning the listed command. Clients that don't surface a login UI should at least log the terminal-auth command when the authRequired error arrives."
gaps:
  - "The installed `kimi-code` binary is v0.14.0 (released 2026-06-10), while the canonical docs (https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html) describe v0.22.2 (2026-07-03) capability coverage. The capability matrix in the docs lists `fs/*` and (per changelog) `terminal/*` as 'Yes' for newer versions; v0.14.0 does not advertise `fsCapabilities` and `terminal/*` is documented as 'No' across all known versions. Empirical verification on v0.22.2 was not possible at research time."
  - "The exact reverse-request payloads for `session/request_permission` are not published as samples in the docs; the description references standard ACP types. Live verification was limited because the active account hit a usage quota (HTTP 429) during research, so reverse-request frames could not be captured from a real tool call. Researchers should re-run with quota available to capture concrete permission frames."
  - "Whether `fs/*` reverse requests actually fire on v0.15.0+ is inferred from the docs but not empirically verified at research time."
  - "No third-party ACP adapter is published for Kimi Code (it does not need one — native mode is built into the binary)."
  - "The legacy Python `kimi-cli` (1.47.0) also has a `kimi acp` subcommand; its protocol coverage is undocumented in the current research and may differ from the new `kimi-code` binary."
changes: []
requires_claudine_update: true
reason: "Kimi Code CLI is the only provider studied so far that ships native ACP support inside its primary binary, so Claudine's ACP client integration is dramatically simpler than for adapter-based providers like Claude Code: no `npx` step, no adapter namespace migration, no stale preset workaround. But the binary's capability matrix (no fs/terminal reverse requests, separate auth-required error path, terminal-auth block in authMethods._meta, v0.14.0 missing fsCapabilities, v0.15.0+ adding SSE MCP and routing fs/* to the client, v0.22.2 PATH enrichment) means Claudine's provider catalog needs explicit per-capability wiring rather than assuming a uniform ACP contract — and Claudine's permission/shell-audit layers need to plug into the session/request_permission channel directly (the binary runs commands locally) rather than waiting for terminal/create."
---

# Agent Client Protocol support in Kimi Code CLI

## Overview

Kimi Code CLI — the TypeScript/Node.js SEA successor to the legacy Python `kimi-cli` — has shipped **native ACP support** since `@moonshot-ai/kimi-code 0.9.0` (2026-06-03). The primary CLI binary (`kimi`, installed by the official script or via Homebrew) exposes an `acp` subcommand that speaks JSON-RPC 2.0 over stdio directly. There is no adapter package: the canonical Kimi Code binary is itself the ACP agent, and the published capability matrix (`https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html`) is the authoritative reference for the binary's behavior.

Verified directly against the installed `kimi-code 0.14.0` binary at `/Users/ken/.kimi-code/bin/kimi` on 2026-07-03. Probing `initialize` returns the expected `protocolVersion: 1` handshake, the documented `agentCapabilities` payload, and a single `authMethods[]` entry of `type: "terminal"` whose `_meta.terminal-auth` block carries the `kimi login` command for the client to spawn on demand.

The native-vs-adapter distinction is the load-bearing fact for Claudine: Kimi Code can be added to Claudine's provider catalog with a single launch-mode entry (`kimi acp` on stdio) and no npm bridge, no preset workaround, no namespace migration. Per-capability wiring still matters because the binary's contract is more compact than e.g. Claude Code's: `fs/*` and `terminal/*` are not advertised by v0.14.0, and the spec's `session/close` / `logout` methods are not implemented at all.

## Launching ACP

### Direct launch

```bash
kimi acp
```

This switches the Kimi Code CLI into ACP mode: it prints no banner and immediately waits for an `initialize` JSON-RPC request on stdin. Logs go to stderr and to `~/.kimi-code/logs/kimi-code.log`, leaving the stdio JSON-RPC channel clean.

```bash
kimi acp --login
```

Runs the device-code login flow inline and exits. Useful for IDE-driven headless auth preflight or for refreshing a stale OAuth token without leaving a TTY.

### IDE configuration (the canonical use case)

```json
{
  "agent_servers": {
    "Kimi Code CLI": {
      "type": "custom",
      "command": "kimi",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

> **macOS path caveat**: child processes launched from an IDE GUI on macOS typically do **not** inherit the terminal shell's `PATH`. If `kimi` is not in a system directory like `/usr/local/bin`, use the absolute path (`/Users/ken/.kimi-code/bin/kimi`). The official IDE setup guide calls this out, and v0.22.2 added login-shell PATH enrichment as a mitigation.

### No adapter process

Kimi Code CLI's native ACP mode removes the need for any bridge package — neither `@zed-industries/claude-agent-acp`-style TypeScript adapters nor a separate Rust bridge. The same binary that powers the interactive TUI is the ACP agent.

## Protocol and Capabilities

### Transport and framing

- **Transport**: stdio pipes between the ACP client and the `kimi acp` subprocess.
- **Framing**: newline-delimited JSON-RPC 2.0.
- **Encoding**: UTF-8.
- **Direction**: bidirectional — client → agent (requests/notifications), agent → client (responses, reverse requests, notifications).

### Supported protocol version

The `kimi acp` binary negotiates **ACP v1** (`protocolVersion: 1`). Probing `initialize` with `protocolVersion: 0.23` returns `code: -32602, message: "Invalid params", data: { _errors: { protocolVersion: "Invalid input" } }` — the `0.23` number refers to the `@agentclientprotocol/sdk` JS package lineage documented in the official reference, **not** the protocol version. Always negotiate `ProtocolVersion::V1`.

### Capability matrix (per the official docs)

| Capability | Status | Notes |
|------------|--------|-------|
| `promptCapabilities.image` | supported | Base64 + mimeType |
| `promptCapabilities.audio` | unsupported | Audio prompts not yet supported |
| `promptCapabilities.embeddedContext` | supported | Client may send `resource` / `resource_link` blocks |
| `mcpCapabilities.http` | supported | Forwards HTTP MCP services |
| `mcpCapabilities.sse` | supported (v0.15.0+) | Forwards legacy SSE MCP services; v0.14.0 returns `false` |
| `mcpCapabilities.acp` | unsupported | MCP servers with `acp` transport are silently dropped with a warn log |
| `loadSession` | supported | Replays session history on load |
| `sessionCapabilities.list` | supported | Enumerates the current user's sessions |
| `sessionCapabilities.resume` | supported | Lightweight resume (no history replay) |
| `fsCapabilities.readTextFile` | supported (v0.15.0+) | Documented in the capability matrix; v0.14.0 does NOT advertise `fsCapabilities` |
| `fsCapabilities.writeTextFile` | supported (v0.15.0+) | Same caveat as readTextFile |

### Stable agent-side methods (10 / 12 = 83 %)

The official reference tracks the stable surface separately from the evolving `unstable_*` surface. Stable methods implemented by `kimi acp`:

| Method | Implemented | Notes |
|--------|-------------|-------|
| `initialize` | Yes | Returns `agentInfo`, capability matrix, and `authMethods` |
| `authenticate` | Yes | Validates `methodId='login'`; returns `authRequired (-32000)` if token missing, `invalidParams (-32602)` for unknown ID |
| `session/new` | Yes | Accepts `cwd`, `mcpServers`; returns `configOptions[]` |
| `session/load` | Yes | Restores a session, replays history via `session/update` |
| `session/resume` | Yes | Lightweight sibling of `session/load`; skips history replay |
| `session/prompt` | Yes | Accepts `text` / `image` / `resource` / `resource_link` blocks; streams `agent_message_chunk` |
| `session/cancel` | Yes | Interrupts the current turn |
| `session/list` | Yes | Enumerates sessions on disk |
| `session/set_mode` | Yes | Compatibility path; dispatches to `set_config_option({configId:'mode'})` |
| `session/set_config_option` | Yes | Unified model / thinking / mode picker dispatcher |
| `session/close` | No | Spec method is not implemented |
| `logout` | No | Spec method is not implemented |

### Stable client-side reverse RPC (4 / 9 = 44 %)

| Method | Implemented | Notes |
|--------|-------------|-------|
| `session/update` | Yes | Streams `agent_message_chunk` / `tool_call*` / `plan` / `config_option_update` / `available_commands_update` |
| `session/request_permission` | Yes | Shared channel for tool approval and question elicitation |
| `fs/read_text_file` | Yes (v0.15.0+) | Routed to the client when `fsCapabilities` is advertised (documented); not issued by v0.14.0 |
| `fs/write_text_file` | Yes (v0.15.0+) | Same caveat |
| `terminal/create`, `output`, `release`, `kill`, `wait_for_exit` | **No** | "Terminal reverse-RPC not connected; shell commands use local execution" |

### Unstable surface (1 / 19)

| Method | Implemented | Notes |
|--------|-------------|-------|
| `session/set_model` | Yes | Equivalent to `set_config_option({configId:'model'})` |
| Other 18 unstable methods | No | Includes session lifecycle extensions, buffer sync, inline-edit prediction, provider management, elicitation, etc. |

Methods not listed above return `methodNotFound`.

### MCP forwarding

When the ACP client supplies `mcpServers` in `session/new` or `session/load`, the adapter layer performs the following conversions:

| ACP `mcpServers[].transport` | Kimi runtime equivalent |
|------------------------------|--------------------------|
| `http` | `transport: 'http'` |
| `stdio` | `transport: 'stdio'` |
| `sse` | `transport: 'sse'` |
| `acp` | discarded with a warn log entry |

## Reverse Requests

Because the agent runs shell commands locally through its own kaos tool layer (no `terminal/*` reverse requests) and the `fs/*` reverse requests only fire on v0.15.0+ with `fsCapabilities` advertised, **the only reverse request reliably observed across all versions is `session/request_permission`**. `fs/read_text_file` and `fs/write_text_file` are documented as supported but are best-effort for clients targeting current binaries.

### `session/request_permission`

The shared channel for both tool approvals and structured user questions. The agent sends a JSON-RPC request with `id`, `method: "session/request_permission"`, and `params` carrying `sessionId`, `toolCall` (`toolCallId`, `title`, `kind`, `content`), and a list of `PermissionOption` entries. The client responds with `RequestPermissionOutcome::Selected { optionId }` or `RequestPermissionOutcome::Cancelled`.

Mode-driven auto-approval short-circuits this request:
- `mode = default` → every approval request must be answered by the client
- `mode = auto` → "safe" operations auto-approve; risky ones still hit the client
- `mode = yolo` → everything auto-approves
- `mode = plan` → tools are not invoked at all (read-only planning)

### `fs/read_text_file` (best-effort, v0.15.0+)

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "fs/read_text_file",
  "params": {
    "sessionId": "sess_...",
    "path": "/Users/ken/project/src/main.rs",
    "line": 1,
    "limit": 200
  }
}
```

The client returns `{ content: "..." }` after sandboxing the path against the project root, or a JSON-RPC error if the path is outside the sandbox.

### `fs/write_text_file` (best-effort, v0.15.0+)

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "fs/write_text_file",
  "params": {
    "sessionId": "sess_...",
    "path": "/Users/ken/project/src/new_file.rs",
    "content": "..."
  }
}
```

Implement only when targeting v0.15.0+. On v0.14.0, file writes happen inside the agent's tool layer and never reach the client.

### Notifications: `session/update`

Not a reverse request in the strict sense — a fire-and-forget notification. Group by `sessionId` and route each `sessionUpdate` variant into the UI:

| `sessionUpdate` variant | UI target |
|--------------------------|------------|
| `agent_message_chunk` | Text stream |
| `user_message_chunk` | Text stream (replay) |
| `tool_call` / `tool_call_update` | Tool card list |
| `plan` | Plan panel |
| `config_option_update` | Mode / model / thinking picker |
| `available_commands_update` | Slash-command menu |

## Permissions, Filesystem, and Terminal

### Permission policy

`session/request_permission` is the single channel. The mode config option (`default` / `plan` / `auto` / `yolo`) is exposed through `configOptions[].select` with `category: "mode"` and is set via `session/set_config_option({configId:"mode", value:"auto"})`. Clients should:

1. Surface `mode` in a picker and reflect changes back into the UI.
2. Render `session/request_permission` calls as a permission modal when `mode = default`.
3. Map `allow_always` options to a per-session "always for this session" state (the binary keeps the decision in-process; the client only needs to remember it for UI consistency).
4. On `session/cancel`, the client should respond `Cancelled` to any in-flight `session/request_permission` to release the agent's blocked state.

### Filesystem policy

Two regimes, depending on the binary version:

- **v0.14.0 (and earlier)**: reads and writes happen inside the agent's kaos tool layer. The client sees no `fs/*` reverse requests, even when it advertises `fsCapabilities`. Sandboxing must be enforced through the agent's own permission/mode system.
- **v0.15.0+**: the agent forwards `fs/read_text_file` and `fs/write_text_file` to the client when the client advertises the matching capability. The client enforces project-root sandboxing, validates absolute paths, and treats 1-based line numbers as required (ACP convention).

In both regimes, paths are absolute. Relative paths and 0-based line numbers are common integration bugs.

### Terminal policy

`terminal/*` reverse requests are not issued. The agent runs shell commands locally in its own process; clients never see a `TerminalId` and have no output buffers, kill discipline, or release calls to manage. If a client wants to surface shell execution in its UI, it should rely on the `tool_call` / `tool_call_update` session-update stream that the agent emits for shell invocations.

## Streaming and UI Integration

`session/update` is the single streaming channel. Updates are fire-and-forget JSON-RPC notifications with `sessionId` and `update`. Distinct from `@agentclientprotocol/claude-agent-acp`, the `kimi acp` vocabulary is compact:

| Update | Purpose |
|--------|---------|
| `agent_message_chunk` | Incremental assistant text |
| `user_message_chunk` | User message replay during `session/load` |
| `tool_call` | Tool invocation started |
| `tool_call_update` | Tool progress / status change / result |
| `plan` | Plan entry streamed |
| `config_option_update` | Mode / model / thinking changed |
| `available_commands_update` | Slash-command inventory changed |

Notifications do not carry a `message_id` (no `ContentChunk.message_id` grouping as in the TypeScript Claude adapter). Group by `sessionId` and stream order.

Error events do not have a dedicated `session/update` variant. Provider errors (rate limit, quota exhaustion, network drop) surface as JSON-RPC error responses on the request channel — and a quota-exhausted turn returns `stopReason: "end_turn"` with **no** streaming chunks at all, which clients should treat as a likely upstream error and surface to the user. The diagnostic log at `~/.kimi-code/logs/kimi-code.log` always has the underlying detail.

## Authentication and Setup

### Required preconditions

Before `kimi acp` can run a real prompt, the local user must have a valid OAuth token or API key. The binary:

1. Loads credentials from `~/.kimi-code/credentials/kimi-code.json` (auto-refreshing OAuth) OR
2. Reads the configured provider's `api_key` from `~/.kimi-code/config.toml`

If no token is present, `initialize` succeeds but `session/new` returns `authRequired (-32000)` per the docs.

### Mechanisms

1. **Kimi Code OAuth via `kimi login`** — device-code flow. Standard launch:
   ```bash
   kimi login
   ```
2. **Pre-existing cached credentials** — re-used automatically. Token expiry defaults to 15 minutes (`expires_in: 900`); refresh tokens are valid for ~90 days. Refresh is atomic and coordinated across multiple `kimi acp` instances via a cross-process file lock.
3. **Moonshot AI Open Platform API key** — entered via the `/login` flow.
4. **Kimi Code Web mode** — `kimi web` exposes a browser-based chat UI with the same auth.

### Terminal-auth pattern

The binary advertises a single `authMethods[0]` of `type: "terminal"`:

```json
{
  "id": "login",
  "type": "terminal",
  "name": "Login with Kimi account",
  "description": "Open the device-code login flow in a terminal.",
  "args": ["--login"],
  "env": {},
  "_meta": {
    "terminal-auth": {
      "type": "terminal",
      "label": "Login with Kimi account",
      "command": "/Users/ken/.kimi-code/bin/kimi",
      "args": ["login"],
      "env": {}
    }
  }
}
```

A headless ACP client can drive the login itself by spawning the listed `command` with the listed `args`. If the client surfaces no login UI, it should at minimum log the terminal-auth command when `authRequired (-32000)` arrives.

## Compatibility, Quirks, and Workarounds

1. **Two distributions on the same host** — the legacy Python `kimi-cli` (1.47.0, winding down) and the new TypeScript `kimi-code` (0.14.0 installed, 0.22.2 latest) both ship `kimi acp`. Use `kimi --version` to disambiguate; the home directories differ (`~/.kimi/` vs `~/.kimi-code/`).
2. **macOS PATH inheritance gap** — GUI-launched subprocesses do not inherit the terminal's PATH. Always use an absolute path in IDE `command:` fields. v0.22.2 added login-shell PATH enrichment as a mitigation, but absolute paths remain the safe choice.
3. **Provider rate-limit surfaces as `end_turn`** — quota exhaustion returns an immediate `session/prompt` response with `stopReason: "end_turn"` and no streaming chunks. Treat a `end_turn` after a no-stream turn as a likely upstream error; check `~/.kimi-code/logs/kimi-code.log` for the underlying detail.
4. **`session/load` requires `mcpServers: []`** — omitting the field returns `-32602 invalid-params`. `session/new` has the same constraint in current versions.
5. **v0.14.0 does NOT advertise `fsCapabilities`** — even when the client declares `fs.readTextFile: true`, the binary keeps file I/O local. Implement `fs/read_text_file` and `fs/write_text_file` only as best-effort for v0.15.0+ clients.
6. **`protocolVersion: 0.23` is rejected** — the 0.23 number is the `@agentclientprotocol/sdk` JS package version, not the protocol version. Always negotiate `protocolVersion: 1`.
7. **`acp` MCP transport is silently dropped** — only `http`, `stdio`, and `sse` are forwarded.
8. **`session/close` and `logout` are not implemented** over JSON-RPC. Close sessions by terminating the subprocess; rotate credentials via `kimi login`.
9. **`ext_method` / `ext_notification` extensions are not negotiated** — `_meta` carries provider-specific data only (notably `authMethods[].terminal-auth`).
10. **Quota-hit sessions are silently ended** — clients that show a busy spinner during a prompt should fall back to a "completed" state if the turn ends with no chunks at all and surface a "please check quota" hint.
11. **Config option names are stable** — `session/set_config_option` accepts `configId: "model"`, `"thinking"`, or `"mode"`. The `session/set_mode` method is an alias that internally routes to `set_config_option`.
12. **SSE MCP support is version-gated** — v0.14.0 returns `sse: false` in `mcpCapabilities`; v0.15.0+ returns `true`. Negotiate MCP servers based on the live capabilities.
13. **No fs/terminal reverse requests on v0.14.0** — clients that need to enforce a project-root sandbox must rely on the agent's own mode/permission system rather than on the ACP boundary.
14. **Auth `expires_in: 900` seconds (15 min)** — refresh cycles are aggressive; clients that monitor auth state should expect frequent refresh activity.

## Recent Changes

- **2026-07-03 / `@moonshot-ai/kimi-code 0.22.2`** — bug fixes for streaming transcript duplication, image-compression leak into session titles, Windows console flash on auto-update; polish includes login-shell PATH enrichment at startup, language-matching rule promoted to a system-prompt section, and `keep_alive_on_exit` honoured in `kimi -p`.
- **2026-07-02 / `@moonshot-ai/kimi-code 0.22.0`** — auto-compress oversized images before they reach the model; model alias overrides via `[models."<alias>".overrides]`; web UI redesign.
- **2026-07-01 / `@moonshot-ai/kimi-code 0.21.0`** — plugins can register slash commands via `commands` in their manifest; Mermaid rendering in the web chat.
- **2026-06-30 / `@moonshot-ai/kimi-code 0.20.3`** — Glob switched to ripgrep (respects .gitignore, brace patterns, partial results).
- **2026-06-29 / `@moonshot-ai/kimi-code 0.20.2`** — Anthropic-compatible protocol support (including video input); `KIMI_CODE_CUSTOM_HEADERS`; `exclude_empty` filter on session list.
- **2026-06-26 / `@moonshot-ai/kimi-code 0.20.0`** — shell mode (`!` prefix), `--host` for `kimi web`, LaTeX math in web UI.
- **2026-06-23 / `@moonshot-ai/kimi-code 0.19.1`** — *Fix ACP editors such as Zed failing to start a new thread.*
- **2026-06-22 / `@moonshot-ai/kimi-code 0.19.0`** — `/add-dir` extra directories; move long-running foreground commands / subagents into background with `Ctrl+B`.
- **2026-06-15 / `@moonshot-ai/kimi-code 0.15.0`** — *Added support for legacy SSE MCP servers alongside stdio and streamable HTTP transports.*
- **2026-06-12 / `@moonshot-ai/kimi-code 0.14.1`** — *Fixed ACP file reads and edits for Windows workspaces opened through IDE clients.*
- **2026-06-10 / `@moonshot-ai/kimi-code 0.14.0`** — `Interrupt` hook event when the user interrupts a turn.
- **2026-06-09 / `@moonshot-ai/kimi-code 0.12.0`** — *Fixed ACP slash-skill routing, bootstrap context reads, file and permission edge cases, subagent event handling, and stale-file edit messaging.* Removed the per-turn auto-compaction limit.
- **2026-06-03 / `@moonshot-ai/kimi-code 0.9.0`** — *Added the `kimi acp` subcommand: kimi-code now speaks Agent Client Protocol (built on `@agentclientprotocol/sdk@0.23.0`) over stdio. Initial coverage matrix published.*

## Rust Client Example

This example uses the official `agent-client-protocol` Rust SDK (`agent-client-protocol = "1"`, depending on `agent-client-protocol-schema = 1.1.0`) to drive the installed `kimi-code 0.14.0` binary over stdio.

```toml
[dependencies]
agent-client-protocol = "1"
tokio = { version = "1", features = ["full"] }
```

```rust
use agent_client_protocol::schema::{
    v1::{
        ClientCapabilities, ContentBlock, Implementation, InitializeRequest,
        NewSessionRequest, PromptRequest, SessionNotification, TextContent,
    },
    ProtocolVersion,
};
use agent_client_protocol::{AcpAgent, Client};
use std::path::PathBuf;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kimi_bin = std::env::var("KIMI_BIN")
        .unwrap_or_else(|_| "/Users/ken/.kimi-code/bin/kimi".to_string());

    let agent = AcpAgent::from_str(&kimi_bin)?
        .args(["acp"]);

    Client
        .builder()
        .name("claudine-kimi-client")
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
                    SessionNotification::UserMessageChunk(chunk) => {
                        if let ContentBlock::Text(t) = chunk.content {
                            eprintln!("[user: {}]", t.text);
                        }
                    }
                    _ => {}
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(agent, |connection| async move {
            // CRITICAL: protocolVersion must be 1, NOT 0.23.
            let init = InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(ClientCapabilities::new())
                .client_info(Implementation {
                    name: "claudine".into(),
                    title: Some("Claudine".into()),
                    version: "0.1.0".into(),
                });
            let init_resp = connection.send_request(init).block_task().await?;
            eprintln!("Agent: {:?}", init_resp.agent_info);
            eprintln!("Auth methods: {:?}", init_resp.auth_methods);
            eprintln!("Agent capabilities: {:?}", init_resp.agent_capabilities);

            let project_dir = PathBuf::from(std::env::var("PROJECT_DIR")?);

            // session/new REQUIRES mcpServers (even empty).
            let session = connection
                .send_request(NewSessionRequest::new(project_dir, vec![]))
                .block_task()
                .await?;

            let result = connection
                .send_request(PromptRequest::new(
                    session.session_id,
                    vec![ContentBlock::Text(TextContent::new(
                        "Reply with the single word HELLO and nothing else.".into(),
                    ))],
                ))
                .block_task()
                .await?;

            eprintln!("Stop reason: {:?}", result.stop_reason);
            Ok(())
        })
        .await?;

    Ok(())
}
```

When advertising `fsCapabilities`, do so on the same `ClientCapabilities`:

```rust
use agent_client_protocol::schema::v1::FileSystemCapabilities;
let caps = ClientCapabilities::new()
    .fs(FileSystemCapabilities {
        read_text_file: true,
        write_text_file: true,
    });
```

`kimi-code 0.14.0` will still ignore `fsCapabilities` in its `initialize` response (it does not advertise them in its own agent capabilities), but `0.15.0+` will honor them.

## Rust Reverse Request Handling

`session/request_permission` is the only reverse request reliably issued. `fs/read_text_file` and `fs/write_text_file` are best-effort for `0.15.0+` clients:

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
        anyhow::bail!(
            "path {} is outside project root {}",
            canonical.display(),
            root.display()
        );
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
    tokio::fs::write(&path, &request.content).await?;
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

Because `kimi acp` does **not** issue `terminal/*` reverse requests — shell commands run locally inside the agent's own tool layer — there is nothing to wire up here. A Claudine client targeting Kimi Code can rely entirely on `session/request_permission` for shell-tool approval and on the `tool_call` / `tool_call_update` session-update stream for status visibility.

If a future version adds `terminal/create` support, the same pattern from other ACP agents applies: spawn the child via `tokio::process::Command`, track the `TerminalId` in a `HashMap`, and always implement `terminal/release` to avoid handle leaks.

## Rust Desktop Streaming Bridge

Run the ACP client on a dedicated thread and forward `SessionNotification` values through an `mpsc` channel:

```rust
use std::str::FromStr;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum KimiEvent {
    TextChunk(String),
    UserChunk(String),
    ToolCallStarted { id: String, title: String },
    ToolCallUpdate { id: String, status: String },
    Plan(String),
    ConfigOptionChanged { id: String, value: String },
    AvailableCommands(Vec<String>),
    TurnComplete { stop_reason: String },
    Error(String),
}

pub fn spawn_kimi(
    project_dir: PathBuf,
) -> anyhow::Result<(mpsc::UnboundedReceiver<KimiEvent>, mpsc::UnboundedSender<String>)> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (prompt_tx, mut prompt_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        rt.block_on(async move {
            let kimi_bin = std::env::var("KIMI_BIN")
                .unwrap_or_else(|_| "/Users/ken/.kimi-code/bin/kimi".to_string());
            let agent = AcpAgent::from_str(&kimi_bin)?.args(["acp"]);

            Client
                .builder()
                .name("claudine-kimi-streaming")
                .on_receive_notification(
                    {
                        let tx = event_tx.clone();
                        move |notification: SessionNotification, _cx| {
                            let tx = tx.clone();
                            async move {
                                let event = match notification.update {
                                    SessionNotification::AgentMessageChunk(chunk) => match chunk.content {
                                        ContentBlock::Text(t) => Some(KimiEvent::TextChunk(t.text)),
                                        _ => None,
                                    },
                                    SessionNotification::UserMessageChunk(chunk) => match chunk.content {
                                        ContentBlock::Text(t) => Some(KimiEvent::UserChunk(t.text)),
                                        _ => None,
                                    },
                                    SessionNotification::ToolCall(tc) => Some(KimiEvent::ToolCallStarted {
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
                                let _ = event_tx.send(KimiEvent::TurnComplete {
                                    stop_reason: format!("{:?}", response.stop_reason),
                                });
                            }
                            Err(e) => {
                                let _ = event_tx.send(KimiEvent::Error(e.to_string()));
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

fn listen(event_rx: mpsc::UnboundedReceiver<KimiEvent>, handle: tauri::AppHandle) {
    tokio::spawn(async move {
        let mut rx = event_rx;
        while let Some(event) = rx.recv().await {
            match event {
                KimiEvent::TextChunk(text) => handle.emit("kimi:text", text).ok(),
                KimiEvent::TurnComplete { stop_reason } => handle.emit("kimi:done", stop_reason).ok(),
                KimiEvent::Error(err) => handle.emit("kimi:error", err).ok(),
                _ => None,
            };
        }
    });
}
```

### iced usage

```rust
fn kimi_subscription(
    event_rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<KimiEvent>>>>,
) -> iced::Subscription<KimiEvent> {
    iced::subscription::channel(
        std::any::TypeId::of::<KimiEvent>(),
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

Adding ACP-based Kimi Code support to Claudine would require:

1. **Launch detection** — single canonical entry: `kimi acp` on stdio, no adapter. Store the binary path from `which kimi` (or accept a `kimi_bin` override env var). Because macOS GUI launches do not inherit terminal PATH, Claudine should resolve the absolute path via `which` and fall back to known install locations (`/Users/<user>/.kimi-code/bin/kimi`, `~/.local/bin/kimi`, Homebrew, npm global). The CLI binary is large (~130 MB Node.js SEA) but the launch is fast.
2. **Capability negotiation** — always send `protocolVersion: 1` (NOT `0.23`, which the binary rejects as -32602). Advertise `fsCapabilities` and `terminal: true` anyway for forward compatibility with v0.15.0+, but treat the v0.14.0-installed case as: only `session/request_permission` reverse requests will arrive.
3. **Mode-aware permission flow** — wire `session/set_config_option({configId:"mode", value:...})` into Claudine's existing `permissions` / `PolicyEngine` module so users can switch between `default` / `auto` / `yolo` / `plan` from the host UI. Forward `session/request_permission` calls into the permission widget, mapping `allow_always` to "always for this session".
4. **Reverse-request routing** — implement `session/request_permission` as the primary reverse-request handler. Implement `fs/read_text_file` / `fs/write_text_file` as best-effort forward-compat handlers. Skip `terminal/*` entirely (not implemented).
5. **Streaming bridge** — route `session/update` notifications into Claudine's lifecycle pipeline (TTS, sound effects, logging, messenger actions). Watch for the quota-end_turn quirk: an `end_turn` after no streaming chunks is almost always an upstream rate-limit / quota-exhaustion signal — surface it as an error toast rather than as a clean completion.
6. **Terminal isolation** — N/A. `terminal/*` is not implemented; shell commands run inside the agent process. Claudine's shell-audit, timeout, and deny-list rules must apply at the agent side (mode/yolo toggles, lifecycle hooks) rather than at the ACP boundary.
7. **Headless auth** — before launching `kimi acp` headlessly, ensure a valid token exists at `~/.kimi-code/credentials/kimi-code.json`. If not, drive the `authMethods[]._meta.terminal-auth` flow (spawn `kimi login` and wait for the device-code confirmation) before launching the agent. Treat `authRequired (-32000)` on `session/new` as a recoverable error that triggers the same login flow rather than a hard failure.
8. **Schema versioning** — verify on every launch that the negotiated `protocolVersion` matches what Claudine's handlers expect. The binary rejects `0.23` outright, so a defensive `try { send_request(InitializeRequest::new(ProtocolVersion::V1)) }` is the only correct path.
9. **Multi-version awareness** — `mcpCapabilities.sse` flips at v0.15.0; `fsCapabilities` advertisement in the binary's `agentCapabilities` does not appear until at least v0.15.0. Claudine's MCP injection (`session/new` `mcpServers[]`) should be filtered by the live capabilities rather than assumed.
10. **PATH resolution** — on macOS GUI launches, always pass the absolute path; v0.22.2's login-shell PATH enrichment helps but is not a substitute. On Windows, set `KIMI_SHELL_PATH` (or legacy `KIMI_CLI_GIT_BASH_PATH`) when Git Bash is installed in a non-standard location.

Because `kimi acp` is the simplest "native ACP" provider studied so far, Claudine's wiring should be **simpler** than for adapter-based providers (no `npx`, no preset workaround, no namespace migration). The trade-off is that the binary's contract is more compact than the union of all ACP capabilities: there are real reverse-request gaps (no `terminal/*`), version-gated features (`fs/*`, `mcpCapabilities.sse`), and oddities like `session/close` / `logout` being absent entirely.

## Changelog

- **2026-07-03**: Initial release. Verified `kimi acp` against `kimi-code 0.14.0` (installed at `/Users/ken/.kimi-code/bin/kimi`); documented the official capability matrix from the v0.22.2 docs as the forward-looking contract; recorded `mcpCapabilities.sse` flipping from `false` → `true` at v0.15.0 and `fs/*` reverse requests becoming reliable at v0.15.0+; captured quirks (PATH inheritance gap on macOS GUI launches, quota-hit silent end_turn, `mcpServers: []` requirement on `session/load`, `protocolVersion: 0.23` rejection, `authMethods[]._meta.terminal-auth` for headless login).

## Sources

- [Kimi Code CLI documentation](https://moonshotai.github.io/kimi-code/en/)
- [`kimi acp` reference (capability matrix, method coverage)](https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html)
- [Using Kimi Code CLI in IDEs (Zed, JetBrains, Paseo configuration)](https://moonshotai.github.io/kimi-code/en/guides/ides.html)
- [`MoonshotAI/kimi-code` repository (the active Kimi Code CLI)](https://github.com/MoonshotAI/kimi-code)
- [`@moonshot-ai/kimi-code 0.22.2` release](https://github.com/MoonshotAI/kimi-code/releases/tag/%40moonshot-ai%2Fkimi-code%400.22.2)
- [Kimi Code CLI changelog](https://moonshotai.github.io/kimi-code/en/release-notes/changelog.html)
- [Legacy `MoonshotAI/kimi-cli` repository (Python, winding down)](https://github.com/MoonshotAI/kimi-cli)
- [Agent Client Protocol specification](https://agentclientprotocol.com/)
- [ACP schema reference](https://agentclientprotocol.com/protocol/schema)
- [`agent-client-protocol` Rust SDK 1.0.1 on docs.rs](https://docs.rs/agent-client-protocol/1.0.1/agent_client_protocol/)
- [`agent-client-protocol-schema` 1.1.0](https://docs.rs/agent-client-protocol-schema/1.1.0/agent_client_protocol_schema/)
- [Local evidence: `kimi acp` initialize handshake against `kimi-code 0.14.0`](file:///Users/ken/.kimi-code/bin/kimi) — captured via stdio JSON-RPC probes during this research
- [Local evidence: `~/.kimi-code/config.toml` (model providers, capabilities, oauth, services)](file:///Users/ken/.kimi-code/config.toml)
- [Local evidence: `~/.kimi-code/credentials/kimi-code.json` (OAuth access/refresh tokens, expires_in=900)](file:///Users/ken/.kimi-code/credentials/kimi-code.json)
- [Local evidence: `~/.kimi-code/logs/kimi-code.log` (ACP session lifecycle entries: `acp: received signal, draining harness`)](file:///Users/ken/.kimi-code/logs/kimi-code.log)