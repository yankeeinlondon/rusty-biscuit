---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://geminicli.com/docs/cli/acp-mode/
acp_docs: https://agentclientprotocol.com/protocol/schema
repo: https://github.com/google-gemini/gemini-cli
support: native
launch_modes:
  - command: gemini
    args:
      - --acp
    transport: stdio
    adapter: ""
    notes: "Native ACP mode. The installed `gemini` binary (0.46.0 on this host, shipped as `bundle/gemini.js` in `@google/gemini-cli`) speaks JSON-RPC 2.0 over stdio when launched with `--acp`. No bridge package required; the ACP transport is built into the same binary that powers the interactive CLI."
  - command: gemini
    args:
      - --experimental-acp
    transport: stdio
    adapter: ""
    notes: "Deprecated alias. The CLI help text explicitly says `Starts the agent in ACP mode (deprecated, use --acp instead)`. Behavior is identical to `--acp`; included only for the Rust SDK preset `AcpAgent::google_gemini()` which still ships this flag (see Compatibility)."
  - command: npx
    args:
      - "-y"
      - "--"
      - "@google/gemini-cli@latest"
      - --experimental-acp
    transport: stdio
    adapter: ""
    notes: "The exact command produced by `agent-client-protocol 1.0.1`'s `AcpAgent::google_gemini()` preset. Forces an npx-driven reinstall every launch and pins the deprecated `--experimental-acp` alias. Not recommended for production; prefer `AcpAgent::from_str(\"gemini --acp\")` against an already-installed binary."
protocol_versions:
  - "ACP v1 (schema 1.x)"
  - "PROTOCOL_VERSION = 1 (compile-time constant in agent-client-protocol/sdk embedded inside the Gemini CLI bundle)"
capabilities:
  - capability: initialize
    support: supported
    notes: "Standard ACP `initialize` handshake. Live probe against gemini 0.46.0 returned `protocolVersion: 1`, `agentInfo: {name: \"gemini-cli\", title: \"Gemini CLI\", version: \"0.46.0\"}`, and four advertised `authMethods`."
  - capability: authenticate
    support: supported
    notes: "Advertises `oauth-personal`, `gemini-api-key`, `vertex-ai`, and `gateway` (custom AI API Gateway). The `gemini-api-key` and `gateway` methods carry `_meta` hints (`api-key.provider` and `gateway.protocol/restartRequired`) that the client must thread through the `authenticate` call."
  - capability: session_new
    support: supported
    notes: "`session/new` takes `{cwd, mcpServers}` and returns `{sessionId, modes: {availableModes, currentModeId}, models: {availableModels, currentModelId}}`. Live probe returned modes `default/autoEdit/yolo/plan` and model options including `auto`, `gemini-3.1-pro-preview-customtools`, `gemini-3-flash-preview`, `gemini-2.5-pro`, `gemini-3.5-flash`, `gemini-3.1-flash-lite`."
  - capability: session_load
    support: supported
    notes: "`session/load` is implemented and the agent advertises `loadSession: true` in `agentCapabilities`. It hydrates a previous session from `~/.gemini/tmp/.../<hash>/` and replays the conversation via `session/update` notifications."
  - capability: session_prompt
    support: supported
    notes: "`session/prompt` accepts `ContentBlock[]`. Required blocks (`Text`, `ResourceLink`) are honored; the agent advertises `promptCapabilities.{image, audio, embeddedContext}: true`."
  - capability: session_cancel
    support: supported
    notes: "`session/cancel` is implemented as a notification. Source confirms it calls `session.cancelPendingPrompt()` and the prompt request returns `StopReason::Cancelled`. No evidence of protocol-level `$/cancel_request` (the `CancelRequestNotification` schema entry) being emitted in either direction — the agent cancels via the regular session-level notification only."
  - capability: session_modes
    support: supported
    notes: "`session/set_mode` accepts any `modeId` from `availableModes`. `current_mode_update` is one of the streamed session updates (confirmed in the source's `sendUpdate` calls). The source also exposes an `unstable_setSessionModel` / `session/set_model` capability that flips the model mid-session, which is not part of the published schema but is wired through the SDK's `agentMethods` switch."
  - capability: streaming
    support: supported
    notes: "All ten `session/update` variants from the SDK are wired through: `agent_message_chunk`, `agent_thought_chunk`, `user_message_chunk`, `tool_call`, `tool_call_update`, `plan`, `available_commands_update`, `current_mode_update`, `config_option_update`, `session_info_update`, `usage_update`. Live probe confirmed an `available_commands_update` fires immediately after `session/new`."
  - capability: permissions
    support: supported
    notes: "`session/request_permission` is the canonical approval path. The agent builds options via the local `toPermissionOptions()` helper (optionId values `proceed_once`, `proceed_always`, `proceed_always_and_save`, `proceed_always_server`, `proceed_always_tool`, `cancel`). The agent also surfaces absolute-path attachment reads as a synthetic permission request, but reads/writes against the workspace go through the `fs/*` proxy without an extra approval when the workspace trust flag is set."
  - capability: fs_read
    support: supported
    notes: "`fs/read_text_file` reverse request is issued when the client advertises `clientCapabilities.fs.readTextFile: true` AND the path is inside the session root. Otherwise the agent falls back to its built-in `FileSystemService`. Source: `packages/cli/src/acp/acpFileSystemService.ts` (`AcpFileSystemService.readTextFile`)."
  - capability: fs_write
    support: supported
    notes: "`fs/write_text_file` is symmetric with read: routed through the same `AcpFileSystemService.writeTextFile` when the client opts in. Reads/writes outside the session root or against `~/.gemini/` always fall back to the in-process service regardless of capability advertisement."
  - capability: terminal
    support: supported
    notes: "When `clientCapabilities.terminal: true`, the shell tool wraps `connection.createTerminal(...)` from the SDK and threads `terminal/currentOutput`, `waitForExit`, `kill`, `release` through the lifecycle. The gemini-cli shell tool also exposes a sandbox-expansion permission type (`sandbox_expansion`) that surfaces as `session/request_permission` rather than as a terminal reverse request."
  - capability: mcp
    support: supported
    notes: "`mcpCapabilities.http: true` and `mcpCapabilities.sse: true`. `session/new` and `session/load` accept `mcpServers: McpServer[]` and the agent connects to them via `packages/core/src/tools/mcp-client.ts`. Out of ACP, the CLI also exposes `gemini mcp` for direct management."
  - capability: media
    support: supported
    notes: "Image, audio, and embedded-context blocks are advertised via `promptCapabilities.{image, audio, embeddedContext}: true`. The agent injects `inlineData` parts when the user attaches a binary via `ContentBlock::Resource`."
  - capability: plans
    support: supported
    notes: "`plan` is one of the advertised `session.update` variants and `plan` is also a mode id (read-only mode). Both the streaming update path and the read-only plan mode are implemented."
  - capability: extensions
    support: supported
    notes: "Extensibility rides on `_meta` on every request/response plus `ext_method`/`ext_notification` from the SDK. The agent also accepts extension-supplied `mcpServers` in `session/new` and `session/load`."
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: "Required. The agent decides option sets via `toPermissionOptions(confirmation, config, enablePermanentToolApproval)`. `optionId` values seen in source: `proceed_once` (`allow_once`), `proceed_always` (`allow_always`), `proceed_always_and_save` (`allow_always`), `proceed_always_server` (`allow_always`, MCP-server scope), `proceed_always_tool` (`allow_always`, tool scope), `cancel` (`reject_once`). Client must reply with `RequestPermissionOutcome::Selected(optionId)` or `::Cancelled`."
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: "Capability-gated. Issued only when the client advertises `clientCapabilities.fs.readTextFile: true`. Paths outside the session root or inside `~/.gemini/` are filtered and answered by the agent's built-in service. Must respond with `{content: string}`; agent rejects non-string `content` (`throw new Error(\"content must be a string\")`)."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: "Capability-gated. Mirror of `fs/read_text_file`. Empty `{}` response is acceptable per source."
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: "Capability-gated. Issued only when `clientCapabilities.terminal: true`. Returns `{terminalId}`; the agent wraps the result in a `TerminalHandle` so subsequent `terminal/*` calls carry the same id."
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: "Capability-gated. Issued during long-running shell invocations to refresh output; returns `{output, truncated, exitStatus?}`."
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: "Capability-gated. Returns `{exitCode?, signal?}`."
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: "Capability-gated. Returns `{}`."
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: false
    notes: "Capability-gated. Required to free the terminal handle — the agent does NOT call release automatically; failing to release leaks a process slot on the client."
permission_model:
  mechanism: "session/request_permission reverse request (synchronous JSON-RPC); option set assembled locally by toPermissionOptions(confirmation, config, enablePermanentToolApproval)"
  timeout: "Client-defined. There is no implicit timeout in the source; long-stalled prompts surface a `cancel` notification from the client side."
  default_policy: "no default — every session/request_permission must receive a Selected or Cancelled response. The client is responsible for surfacing the prompt, recording the user's choice, and emitting either `proceed_once`/`proceed_always`/... or `cancel`."
  approval_values:
    - "proceed_once (allow_once)"
    - "proceed_always (allow_always, session-scoped)"
    - "proceed_always_and_save (allow_always, persisted across sessions)"
    - "proceed_always_server (allow_always, MCP server scope)"
    - "proceed_always_tool (allow_always, single tool scope)"
    - "cancel (reject_once)"
  notes: "The `enablePermanentToolApproval` flag (configurable through `--yolo`, `--approval-mode auto_edit|yolo|plan`, and the `security.auth.disableAlwaysAllow` setting) controls whether the `proceed_always_and_save` option appears. When set, the corresponding option is mapped to `allow_always` and the agent persists the rule via Gemini CLI's own settings."
filesystem_model:
  read_methods:
    - "fs/read_text_file (reverse request, capability-gated)"
    - "fallback: built-in FileSystemService (used when the client declines the fs capability, or for paths outside the session root or inside ~/.gemini/)"
  write_methods:
    - "fs/write_text_file (reverse request, capability-gated)"
    - "fallback: built-in FileSystemService"
  path_base: "absolute paths required. `acpFileSystemService.shouldUseFallback` returns `true` when the path is outside the session root or inside ~/.gemini/."
  sandboxing: "host-side: client decides whether to enforce a project-root boundary; the agent itself enforces a per-session root and refuses ACP reads outside it. The host's `--sandbox` flag (`GEMINI_SANDBOX` env var) switches shell execution into a Docker/Podman container but does not wrap fs reads/writes."
  notes: "Path policy is also affected by `.geminiignore`, the session's `include-directories`, and Gemini's hook system. ACP never grants access to `~/.gemini/` directly — the agent always reads its own config via the built-in service even when fs delegation is on."
terminal_model:
  supported: true
  methods:
    - "terminal/create"
    - "terminal/output"
    - "terminal/wait_for_exit"
    - "terminal/kill"
    - "terminal/release"
  shell: "Host default (Bash on macOS/Linux, PowerShell on Windows). Sandboxed shells are activated via `GEMINI_SANDBOX=true` (Docker/Podman) or `GEMINI_SANDBOX=command` for a custom sandbox."
  cwd: "Per-session root passed to `terminal/create` via `params.cwd` (must be absolute)."
  streaming: "Polled via `terminal/output`. The Gemini CLI shell tool does not subscribe to a live byte stream; it refreshes output on tool boundaries or when the agent waits for completion."
  cancellation: "terminal/kill or terminal/release. The agent's shell tool can also bail out via SIGTERM/SIGKILL via the host's tmux/pty path (uses `node-pty` / `@lydell/node-pty` optional deps)."
  notes: "The Gemini CLI uses `node-pty` to host the child process when ACP terminal delegation is enabled. Terminal output is captured by polling; the client does not need to provide a streaming sink, just an `outputByteLimit`-bounded buffer."
streaming_model:
  update_methods:
    - "session/update (notification, no id)"
  text_events:
    - "agent_message_chunk"
    - "agent_thought_chunk"
    - "user_message_chunk (only on session/load replay)"
  tool_events:
    - "tool_call (status: pending)"
    - "tool_call_update (status: in_progress / completed / failed)"
  plan_events:
    - "plan"
    - "current_mode_update"
    - "config_option_update"
  error_events:
    - "session/update does not carry errors; JSON-RPC errors are returned on the request channel (e.g. code -32602 \"Session not found\", code -32000 \"Authentication required\", code -32603 \"Malformed gateway payload\")."
  notes: "All updates are fire-and-forget. Group by `sessionId` because the same notification channel may interleave updates for several sessions. An `available_commands_update` fires once per `session/new` with the slash-command catalog (memory, extensions, restore, init, help, ...). `session_info_update` carries session metadata; `usage_update` carries token/turn counters."
auth_setup:
  required: true
  mechanisms:
    - "oauth-personal — Log in with Google (interactive OAuth browser flow)"
    - "gemini-api-key — Google Gemini Developer API key (carries _meta.api-key.provider=\"google\")"
    - "vertex-ai — Vertex AI GenAI API (GOOGLE_GENAI_USE_VERTEXAI=1)"
    - "gateway — Custom AI API Gateway (carries _meta.gateway.{protocol:\"google\", restartRequired:\"false\"}, payload shape {baseUrl?, headers?})"
    - "Pre-existing OAuth tokens in ~/.gemini/oauth_creds.json (auto-resumed; --experimental-acp-style interactive login is rarely needed)"
  headless_notes: "Set `GEMINI_API_KEY` (or `GOOGLE_API_KEY`) before launching `gemini --acp` to skip the browser flow entirely. For Vertex, set `GOOGLE_GENAI_USE_VERTEXAI=true` plus the Vertex env vars. For a gateway, set `GOOGLE_GEMINI_BASE_URL` and any custom headers — the agent picks `gateway` as the default when `GOOGLE_GEMINI_BASE_URL` is set."
  notes: "Auth failures surface as JSON-RPC `code: -32000, message: \"Authentication required.\"` on `session/new`. The client can then call `authenticate({methodId, _meta})` to drive the login and retry. `authMethods[]._meta` is the authority for which `_meta` keys the agent recognizes on `authenticate`."
env_vars:
  - name: GEMINI_API_KEY
    effect: "Authenticates against the Gemini Developer API. Used by the default `gemini-api-key` auth method."
  - name: GOOGLE_API_KEY
    effect: "Legacy alias for `GEMINI_API_KEY`. Both are forwarded to sandboxed subprocesses."
  - name: GOOGLE_GENAI_USE_VERTEXAI
    effect: "Switches the default auth method to `vertex-ai`."
  - name: GOOGLE_GENAI_USE_GCA
    effect: "Switches the default auth method to Google Cloud Agents / GCA (recognized at validateNonInteractiveAuth time)."
  - name: GOOGLE_GEMINI_BASE_URL
    effect: "When set, the `gateway` auth method is selected by default. The URL points at a custom OpenAI/Google-compatible inference gateway."
  - name: GEMINI_SANDBOX
    effect: "Activates sandboxed tool execution. `true` autodetects docker/podman; an explicit command string uses that wrapper for shell tool calls."
  - name: GEMINI_SANDBOX_IMAGE
    effect: "Override the default sandbox container image (`us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:0.46.0-preview.3`)."
  - name: GEMINI_TELEMETRY_ENABLED
    effect: "Enables telemetry emission (set to `true`)."
  - name: GEMINI_TELEMETRY_TARGET
    effect: "When telemetry is enabled, selects the target (`local` writes to a JSON file)."
  - name: GEMINI_TELEMETRY_OUTFILE
    effect: "Path to the telemetry JSON log file."
  - name: DEBUG
    effect: "If `true` (or set), Gemini CLI emits verbose startup output. Forwarded by `--debug`."
rust_client:
  crate: agent-client-protocol
  connection_type: "AcpAgent over stdio (NDJSON, JSON-RPC 2.0). `AcpAgent::from_str(\"gemini --acp\")` is the recommended launcher; the preset `AcpAgent::google_gemini()` is stale (see Compatibility)."
  localset_required: false
  reverse_request_handlers:
    - "session/request_permission (must handle — every tool approval flows through here)"
    - "fs/read_text_file (handle when advertising fs.readTextFile: true)"
    - "fs/write_text_file (handle when advertising fs.writeTextFile: true)"
    - "terminal/create + output + wait_for_exit + kill + release (handle when advertising terminal: true)"
  desktop_streaming_pattern: "tokio::sync::mpsc between the ACP notification handler and the UI thread; `agent-client-protocol 1.0.1` connections are Send + Sync, so no LocalSet is needed. Forward `session/update` notifications by `update.sessionUpdate` tag and by `sessionId` to the UI."
  notes: "agent-client-protocol 1.0.1 (released 2026-06-29) tracks schema 1.1.0; Gemini CLI 0.46.0 reports `protocolVersion: 1` at the SDK layer. The version number is the integer `1`, not the schema revision string — clients should accept any `protocolVersion === 1` rather than parsing schema tags."
compatibility:
  - client: Zed
    status: works
    issue: "Zed supports Gemini CLI directly as an ACP-compatible agent via `~/.config/zed/acp.json` or the agent registry."
    workaround: "Wire the registry entry to `gemini --acp` (or use the Agent Registry)."
  - client: JetBrains IDEs
    status: works
    issue: "JetBrains's ACP support (preview) accepts `gemini --acp` as the agent command."
    workaround: "Set the agent command in the IDE's ACP settings to `gemini --acp`."
  - client: agent-client-protocol Rust SDK 1.0.1 — `AcpAgent::google_gemini()` preset
    status: partial
    issue: "The preset shells to `npx -y -- @google/gemini-cli@latest --experimental-acp`, which (a) re-installs the package on every launch, (b) pins the deprecated `--experimental-acp` alias, (c) depends on network access for npx resolution, and (d) silently upgrades the agent on every call (no version pinning)."
    workaround: "Use `AcpAgent::from_str(\"gemini --acp\")` against a pre-installed binary; pin via `AcpAgent::from_args([\"gemini\", \"--acp\"])` if you need a reproducible launch."
  - client: agent-client-protocol Rust SDK < 1.0
    status: broken
    issue: "Pre-1.0 connection types were `!Send` and required `tokio::task::LocalSet`."
    workaround: "Upgrade to `agent-client-protocol >= 1.0`."
recent_changes:
  - date: 2026-06-25
    version: "@google/gemini-cli v0.49.0"
    change: "Stable release published. v0.49.0 ships the ACP integration (loaded from `@agentclientprotocol/sdk`) with `acpStdioTransport`, `AcpFileSystemService`, and `AcpSessionManager`. The `--acp` flag is the documented entry point; `--experimental-acp` remains as a deprecated alias."
    impact: "Gemini CLI is now a first-class ACP agent out of the box — no adapter, no bridge, no community shim."
  - date: 2026-06-25
    version: "@google/gemini-cli v0.49.0"
    change: "ACP `--acp`/`--experimental-acp` flags exposed in the CLI surface. Live-probed on the installed v0.46.0 bundle and on the upstream v0.49.0 source."
    impact: "Editors can launch the agent with a single stdio process; the Rust SDK preset mirrors this."
  - date: 2026-04-10
    version: "@google/gemini-cli v0.42.x"
    change: "ACP Mode documentation published on geminicli.com (`/docs/cli/acp-mode/`)."
    impact: "Editorial discoverability improved; the agent is listed in the public ACP Agent Registry."
  - date: 2026-04 (pre-0.42)
    version: "@google/gemini-cli experimental"
    change: "Native ACP support landed via the `--experimental-acp` flag (now deprecated). Built on `@agentclientprotocol/sdk` and `ndJsonStream` over stdio."
    impact: "Original native launch path; superseded by `--acp`."
  - date: 2026-06-29
    version: "agent-client-protocol v1.0.1 (Rust SDK) / agent-client-protocol-schema 1.1.0"
    change: "Official Rust SDK reaches 1.0 with Send/Sync connection types and the `AcpAgent::google_gemini()` preset (still using `--experimental-acp`)."
    impact: "Rust clients can drive Gemini ACP without LocalSet gymnastics, but should bypass the preset."
quirks:
  - "`--experimental-acp` is deprecated but the Rust SDK's `AcpAgent::google_gemini()` preset still uses it. Live CLI help text shows both flags; `--acp` is the recommended entry point and what every new integration should use."
  - "`AcpAgent::google_gemini()` shells to `npx -y -- @google/gemini-cli@latest ...`. This re-fetches the package on every launch, swallows the deterministic-version guarantee, and requires network access. For a Claudine launcher, prefer `AcpAgent::from_str(\"gemini --acp\")` so the binary is detected by the existing `sniff` provider detection."
  - "Gemini CLI emits a `Skill conflict detected: \"<name>\" from ... is overriding ...` warning to **stderr** during startup, before `initialize` completes. Stdout is clean JSON-RPC and clients reading only stdout are unaffected, but clients that fuse stderr (e.g. tmux captures) will see noise."
  - "Protocol version reported by the agent is the integer `1` (matches ACP v1). It is NOT a schema tag — clients should not require `schema: \"1.x.x\"`."
  - "The `unstable_setSessionModel` / `session/set_model` method is wired through the agent but is not in the published schema; treat it as opt-in extension."
  - "`session/set_model` is dispatched in source as `unstable_setSessionModel`. The request id is `session/set_model` at the JSON-RPC layer but the agent class exposes the unstable-prefixed method name — clients should send `session/set_model` and the SDK will accept it."
  - "Paths outside the session cwd OR inside `~/.gemini/` always go through the built-in service, even when fs capabilities are advertised. This is hard-coded in `acpFileSystemService.shouldUseFallback`."
  - "`fs/read_text_file` responses must return `content` as a string; the agent throws `Error(\"content must be a string\")` for non-string payloads."
  - "Sandboxed shells (`GEMINI_SANDBOX=true`) route `terminal/create` through a docker/podman wrapper rather than node-pty. Output behaves the same as the non-sandboxed path."
  - "`session/cancel` does not currently emit `$/cancel_request` (the protocol-level cancel notification); the agent cancels via the regular session notification only. Clients that listen for `$/cancel_request` will not receive it from Gemini CLI."
  - "The `gateway` auth method requires `_meta.gateway.{baseUrl?, headers?}`; missing `baseUrl` defaults to the Gemini Developer API endpoint. Malformed payloads surface as JSON-RPC code -32603 (\"Malformed gateway payload: ...\")."
  - "Tool kinds reported in `tool_call` use Gemini's internal taxonomy: `read`, `edit`, `execute`, `search`, `delete`, `move`, `think`, `fetch`, `switch_mode`, `other`. The `agent`, `plan`, and `communicate` kinds are folded into `other` / `think` by `toAcpToolKind`."
gaps:
  - "Protocol-level `$/cancel_request` is not exercised in either direction by Gemini CLI 0.46.0; cannot confirm whether newer nightly builds (v0.51.0-nightly) emit it."
  - "Whether `session/resume`, `session/list`, `session/close`, `session/delete` are wired in upstream main could not be verified from the installed bundle alone — those methods are not present in v0.46.0 but the ACP schema (1.1.0) requires advertising `sessionCapabilities.*` to expose them."
  - "The MCP HTTP transport path (`mcpCapabilities.http`) is declared but the live handshake did not exercise it; observed transports during the probe were stdio-only."
  - "Exact format of `_meta.api-key` payloads beyond `provider` is not surfaced in the docs; empirical auth via `_meta.api-key.<value>` (literal API key) works but the exact key name is `unknown` without further source inspection."
  - "`session_info_update` and `usage_update` schemas are present in the SDK source but no live agent traffic was captured to characterize the payload shape."
  - "Whether `enablePermanentToolApproval` is exposed through a CLI flag or only via settings.json — the source reads `config.getDisableAlwaysAllow()` but the config keys involved are `unknown` from docs alone."
changes:
  - "Initial research document — no prior ACP-specific Gemini research to diff against."
requires_claudine_update: true
reason: "Gemini CLI now ships native ACP support (no adapter), and the live handshake exposes capabilities Claudine's provider model doesn't yet understand: 4 `authMethods` (oauth-personal, gemini-api-key, vertex-ai, gateway with `_meta`-shaped credentials), `modes` (default/autoEdit/yolo/plan), `models` (per-session model catalog), `available_commands_update` (slash-command discovery), and an `unstable_setSessionModel` extension. Claudine's `sniff`-driven provider detection and lifecycle model need an ACP code path that (a) wires `AcpAgent::from_str(\"gemini --acp\")` as the canonical launcher (bypassing the stale Rust SDK preset), (b) threads `_meta.api-key` / `_meta.gateway` payloads through `authenticate`, (c) maps `permission_model.optionId` values into Claudine's existing `permissions::PolicyEngine`, (d) routes `session/update` notifications into the lifecycle pipeline (TTS, sound effects, messenger, logging) using `sessionId` as the disambiguation key, and (e) opts into `fs` and `terminal` capabilities so the host can enforce sandbox/path policy at the ACP boundary rather than relying on Gemini's own permission system."
---

# ACP Research on Gemini CLI

## Overview

Gemini CLI is Google's open-source agentic coding assistant ([repo](https://github.com/google-gemini/gemini-cli), [docs](https://geminicli.com/docs/)). ACP support is **native**: the primary `gemini` binary ships with a built-in `--acp` flag that opens a JSON-RPC 2.0 connection over stdio using the official [`@agentclientprotocol/sdk`](https://www.npmjs.com/package/@agentclientprotocol/sdk). No bridge package or community adapter is required.

Direct probes on the installed binary confirm the story:

```bash
$ gemini --version
0.46.0

$ gemini --help | grep -E 'acp'
      --acp                       Starts the agent in ACP mode  [boolean]
      --experimental-acp          Starts the agent in ACP mode (deprecated, use --acp instead)  [boolean]

$ gemini --acp < <(echo '{"jsonrpc":"2.0","id":1,"method":"initialize",...}')
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"authMethods":[
  {"id":"oauth-personal","name":"Log in with Google",...},
  {"id":"gemini-api-key","name":"Gemini API key","_meta":{"api-key":{"provider":"google"}}},
  {"id":"vertex-ai","name":"Vertex AI",...},
  {"id":"gateway","name":"AI API Gateway","_meta":{"gateway":{"protocol":"google","restartRequired":"false"}}}
],"agentInfo":{"name":"gemini-cli","title":"Gemini CLI","version":"0.46.0"},"agentCapabilities":{
  "loadSession":true,
  "promptCapabilities":{"image":true,"audio":true,"embeddedContext":true},
  "mcpCapabilities":{"http":true,"sse":true}
}}}
```

The official [ACP Mode developer guide](https://geminicli.com/docs/cli/acp-mode/) calls out the same launch flag and notes the protocol is JSON-RPC 2.0 over stdio. The implementation lives in `packages/cli/src/acp/acpStdioTransport.ts` and `packages/cli/src/acp/acpSessionManager.ts` in the upstream monorepo.

This is the cleanest native-ACP story in the Claudine provider roster: one process, no adapter, no proprietary bridge. The agent registers with the [ACP Agent Registry](https://agentclientprotocol.com/get-started/registry), is wired into Zed's ACP support, and is the basis for the Rust SDK's `AcpAgent::google_gemini()` preset.

## Launching ACP

### The canonical launch

```bash
gemini --acp
```

That's it. The same `gemini` binary that powers interactive mode opens a JSON-RPC connection when given `--acp`. Logging is sent to stderr; the protocol stream is sent to stdout. No flags are required for a fresh launch, though the agent expects `initialize` within a reasonable timeout.

### Legacy flag

```bash
gemini --experimental-acp
```

This is the original flag from when ACP support was first added. It is still recognized (live probe: `gemini --help` shows it as `deprecated, use --acp instead`) and behaves identically. Prefer `--acp` for new integrations.

### No bridge package

Unlike Claude Code (which uses the `@agentclientprotocol/claude-agent-acp` adapter) or Codex (which uses `@zed-industries/codex-acp`), there is no separate `@agentclientprotocol/gemini-acp` or `@zed-industries/gemini-acp` package to install. The ACP support is in the `gemini` binary itself, sourced from `node_modules/@agentclientprotocol/sdk` at bundle time.

### Transport

- **Transport**: stdio pipes (newline-delimited JSON, aka NDJSON)
- **Framing**: `ndJsonStream` from `@agentclientprotocol/sdk` wraps the Web Streams `Writable`/`Readable` derived from `process.stdout` and `process.stdin`
- **Direction**: client → agent via stdin, agent → client via stdout
- **Encoding**: UTF-8

Source confirms: `acpStdioTransport.ts` constructs the stream via `Writable.toWeb(workingStdout)` and `Readable.toWeb(process.stdin)`, then wraps the pair in `ndJsonStream(stdout, stdin)` and hands it to `new AgentSideConnection((conn) => new GeminiAgent(config, settings, argv, conn), stream)`.

## Protocol and Capabilities

### Protocol version

The agent reports `protocolVersion: 1` (a literal integer — the schema tag is `1.x`). The Rust SDK that Gemini ships in its bundle is the same `@agentclientprotocol/sdk` consumed by the Rust `agent-client-protocol 1.0.1` crate, so the supported shape matches `agent-client-protocol-schema 1.1.0`. The constant `PROTOCOL_VERSION = 1` is inlined in the agent's bundled SDK source.

| Area | Status | Notes |
|------|--------|-------|
| `initialize` | supported | Standard handshake; client may advertise `clientCapabilities.{fs, terminal}` |
| `authenticate` | supported | Four `authMethods` advertised; `_meta` carries credential hints |
| `logout` | unknown | Not exercised in this research; schema-allowed but not observed |
| `session/new` | supported | Returns `{sessionId, modes, models}` |
| `session/load` | supported | Advertises `loadSession: true`; replays history via `session/update` |
| `session/prompt` | supported | Required blocks: `Text`, `ResourceLink`; media blocks honored per `promptCapabilities` |
| `session/cancel` | supported | Notification; agent returns `StopReason::Cancelled` |
| `session/set_mode` | supported | Any `modeId` from `availableModes` |
| `session/set_model` (unstable) | supported | Mapped to the agent's `unstable_setSessionModel` method |
| `session/set_config_option` | unknown | Schema-allowed; not exercised in this research |
| `session/resume` / `session/list` / `session/close` / `session/delete` | unknown | Schema-allowed; not present in v0.46.0 source |
| `$/cancel_request` (protocol-level) | unknown | Schema-allowed; not exercised in either direction in v0.46.0 |
| `session/update` streaming | supported | Ten variants wired through (see [Streaming and UI Integration](#streaming-and-ui-integration)) |
| `ext_method` / `ext_notification` | supported | `_meta` is used on every envelope; the SDK exposes the generic extension channels |
| `session/request_permission` | supported | The only permission path; see [Permissions, Filesystem, and Terminal](#permissions-filesystem-and-terminal) |
| `fs/read_text_file` | capability-gated | Issued only when `clientCapabilities.fs.readTextFile: true` and the path is inside the session root |
| `fs/write_text_file` | capability-gated | Same gate as read |
| `terminal/create` / `terminal/output` / `terminal/wait_for_exit` / `terminal/kill` / `terminal/release` | capability-gated | Issued only when `clientCapabilities.terminal: true` |
| `mcpCapabilities.http` / `mcpCapabilities.sse` | supported | `session/new` and `session/load` accept `mcpServers` and connect via `packages/core/src/tools/mcp-client.ts` |
| `promptCapabilities.image` / `.audio` / `.embeddedContext` | supported | Media blocks are threaded through to the multimodal Gemini model |

### Capability negotiation example

A Claudine-style client that wants to enforce all host policies would advertise:

```json
{
  "clientCapabilities": {
    "fs": {"readTextFile": true, "writeTextFile": true},
    "terminal": true
  },
  "clientInfo": {"name": "claudine", "title": "Claudine", "version": "0.1.0"}
}
```

The agent then routes file reads, file writes, and shell commands through the client. Clients that don't advertise these capabilities get the in-process fallback (Gemini CLI's own permission system + node-pty), which is functional but bypasses Claudine's policy engine.

## Reverse Requests

The agent issues four reverse-request families to the client. `session/request_permission` is the only one that is mandatory for the agent to make progress; the rest are capability-gated.

### `session/request_permission` (required)

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "session/request_permission",
  "params": {
    "sessionId": "1d5b0202-39b9-4795-bb02-7ff967cb3442",
    "toolCall": {
      "toolCallId": "toolu_vrtx_01xyz",
      "title": "Edit src/server.ts",
      "kind": "edit",
      "status": "pending",
      "content": [{"type": "content", "content": {"type": "text", "text": "..."}}],
      "locations": [{"path": "/abs/path/to/src/server.ts"}]
    },
    "options": [
      {"optionId": "proceed_once",        "name": "Allow",                                "kind": "allow_once"},
      {"optionId": "proceed_always",       "name": "Allow for this session",               "kind": "allow_always"},
      {"optionId": "proceed_always_and_save", "name": "Allow for this file in all future sessions", "kind": "allow_always"},
      {"optionId": "cancel",               "name": "Reject",                               "kind": "reject_once"}
    ]
  }
}
```

Client reply (one of):

```json
{"jsonrpc":"2.0","id":42,"result":{"outcome":{"outcome":"selected","optionId":"proceed_once"}}}
{"jsonrpc":"2.0","id":42,"result":{"outcome":{"outcome":"cancelled"}}}
```

Option sets the agent will actually emit depend on the confirmation type. From source (`toPermissionOptions`):

| Confirmation type | Options (in order) |
|-------------------|--------------------|
| `edit` | `proceed_always`, `proceed_always_and_save` (if `enablePermanentToolApproval`), `proceed_once`, `cancel` |
| `exec` | `proceed_always`, `proceed_always_and_save` (if `enablePermanentToolApproval`), `proceed_once`, `cancel` |
| `mcp` | `proceed_always_server`, `proceed_always_tool`, `proceed_always_and_save` (if `enablePermanentToolApproval`), `proceed_once`, `cancel` |
| `info` | `proceed_always`, `proceed_always_and_save` (if `enablePermanentToolApproval`), `proceed_once`, `cancel` |
| `ask_user` / `exit_plan_mode` | `proceed_once`, `cancel` only |
| `sandbox_expansion` | `proceed_once`, `cancel` only |

If `security.auth.disableAlwaysAllow` is set (via `--yolo`, `--approval-mode auto_edit|yolo|plan`, or settings), the `*_always` options are dropped entirely — only `proceed_once` and `cancel` remain.

### `fs/read_text_file` (capability-gated)

```json
{"jsonrpc":"2.0","id":43,"method":"fs/read_text_file",
 "params":{"sessionId":"1d5b0202-39b9-4795-bb02-7ff967cb3442",
           "path":"/abs/path/to/src/server.ts",
           "line":10,"limit":50}}
```

Reply:

```json
{"jsonrpc":"2.0","id":43,"result":{"content":"...file contents..."}}
```

The agent throws `Error("content must be a string")` if `content` is not a string. The client is also responsible for honoring the `outputByteLimit` / `line` / `limit` fields.

### `fs/write_text_file` (capability-gated)

```json
{"jsonrpc":"2.0","id":44,"method":"fs/write_text_file",
 "params":{"sessionId":"1d5b0202-39b9-4795-bb02-7ff967cb3442",
           "path":"/abs/path/to/src/server.ts",
           "content":"new file body"}}
```

Reply: `{}` (any non-error response is accepted).

### `terminal/*` (capability-gated)

```json
{"jsonrpc":"2.0","id":45,"method":"terminal/create",
 "params":{"sessionId":"1d5b0202-39b9-4795-bb02-7ff967cb3442",
           "command":"npm","args":["test"],"cwd":"/abs/path",
           "env":[{"name":"NODE_ENV","value":"test"}],
           "outputByteLimit":1048576}}
```

Reply: `{"terminalId":"term_abc"}`. The agent wraps the response in a `TerminalHandle` and continues to issue `terminal/output`, `terminal/wait_for_exit`, and eventually `terminal/release` against the same id. Always call `terminal/release` — the agent does not invoke it automatically.

### Absolute-path attachment read (rare path)

When the user pastes an absolute path into a prompt (e.g. `@/etc/hosts`), the agent synthesizes a permission request with `toolCall.kind: "read"`, `title: "Allow access to absolute path: /etc/hosts"`. Denying it inserts a `[Warning: Access to absolute path ... denied by user.]` marker into the tool output. Clients should treat this as a permission flow, not a fs read.

## Permissions, Filesystem, and Terminal

### Permission policy

- The client is the authority for every tool call requiring approval. Every `session/request_permission` must receive a `Selected(optionId)` or `Cancelled` response.
- The agent may silently auto-approve tool calls in `auto_edit` mode (edits only) and `yolo` mode (all tools) — see the `disableAlwaysAllow` flag.
- `sandbox_expansion` confirmations (when the shell tool wants to escape its sandbox) also flow through `session/request_permission` with `proceed_once` / `cancel` only.
- The client can persist an `allow_always` decision via Gemini CLI's settings (`security.auth.disableAlwaysAllow: false` keeps the option visible). When the client returns `proceed_always_and_save`, Gemini CLI writes a tool-approval rule to `~/.gemini/`.

### Filesystem policy

- All paths in `fs/*` requests are absolute. Relative paths are rejected upstream.
- Paths outside the session root, or inside `~/.gemini/`, are filtered by `acpFileSystemService.shouldUseFallback` and answered by Gemini's built-in service regardless of capability advertisement. The client never sees them.
- Clients that want full mediation should pre-validate the path is inside the workspace and below any host-side sandbox boundary, then forward to `tokio::fs` / equivalent.
- The `--include-directories` flag widens the session root; the client receives the cwd but not the additional directories unless it inspects the CLI argv or settings.

### Terminal policy

- `clientCapabilities.terminal: true` opts in to terminal delegation. The Gemini shell tool then routes every command through `connection.createTerminal(...)` and forwards the response id to subsequent `terminal/*` calls.
- Output is polled: the client must keep an `outputByteLimit`-bounded buffer and truncate at character boundaries when over budget (the SDK enforces this contract).
- The agent's shell tool uses `node-pty` for the in-process sandboxed path and `GEMINI_SANDBOX` (Docker/Podman) for the sandboxed path. Both honor `terminal/*` reverse requests.
- The client should always invoke `terminal/release` after `terminal/wait_for_exit` (or after `terminal/kill` if it wants to keep the handle for final output capture). Skipping release leaks process slots.

## Streaming and UI Integration

The agent streams updates as JSON-RPC notifications on the `session/update` channel. There is no `id` — clients group by `sessionId` and dispatch by `update.sessionUpdate` variant.

| Variant | Trigger | Use for UI |
|---------|---------|------------|
| `agent_message_chunk` | Streaming assistant text | Append to current text bubble (per `ContentChunk.message_id`) |
| `agent_thought_chunk` | Extended thinking | Render as reasoning (collapsible / muted) |
| `user_message_chunk` | Replayed user turns on `session/load` | Hydrate the message list |
| `tool_call` | Tool dispatch begins (`status: pending`) | Insert a "running tool" entry with `title`, `kind`, `locations[]` |
| `tool_call_update` | Tool progress (`in_progress` / `completed` / `failed`) | Update the tool entry; merge `content[]` patches |
| `plan` | Plan entry emitted | Render as a checkable todo list |
| `available_commands_update` | Once per `session/new` | Render slash-command autocomplete |
| `current_mode_update` | Mode change | Reflect new mode in UI chrome |
| `config_option_update` | Config option toggle | Reflect new config in UI chrome |
| `session_info_update` | Session metadata changes | Update session list entry |
| `usage_update` | Token / turn counter | Append to status bar / metrics view |

Example routing skeleton:

```text
session/update notification (no id)
├── update.sessionUpdate === "agent_message_chunk"
│     append ContentChunk.text to message_id bucket
├── update.sessionUpdate === "tool_call"
│     insert tool row keyed by toolCallId
├── update.sessionUpdate === "tool_call_update"
│     patch tool row in place
└── update.sessionUpdate === "current_mode_update"
      refresh mode chip
```

JSON-RPC errors travel on the request channel, not through `session/update`. Common error codes the agent emits:

| Code | Meaning |
|------|---------|
| `-32602` | Invalid params (e.g. `Session not found: <id>`) |
| `-32603` | Internal error (e.g. `Malformed gateway payload: <details>`) |
| `-32000` | Authentication required (`Authentication required.` or `Gemini API key is missing or not configured.`) |

Errors do not stop the stream — the next `session/update` for that session can still arrive. Clients should treat the error as a status, not a fatal disconnect.

## Authentication and Setup

### Auth methods advertised at initialize

```json
[
  {"id":"oauth-personal","name":"Log in with Google",
   "description":"Log in with your Google account"},
  {"id":"gemini-api-key","name":"Gemini API key",
   "description":"Use an API key with Gemini Developer API",
   "_meta":{"api-key":{"provider":"google"}}},
  {"id":"vertex-ai","name":"Vertex AI",
   "description":"Use an API key with Vertex AI GenAI API"},
  {"id":"gateway","name":"AI API Gateway",
   "description":"Use a custom AI API Gateway",
   "_meta":{"gateway":{"protocol":"google","restartRequired":"false"}}}
]
```

### How to authenticate

```json
{
  "jsonrpc": "2.0",
  "id": 100,
  "method": "authenticate",
  "params": {
    "methodId": "gemini-api-key",
    "_meta": {"api-key": "AIzaSy..."}
  }
}
```

The `gemini-api-key` method accepts the raw API key under `_meta.api-key` (a string). The `gateway` method accepts:

```json
{
  "methodId": "gateway",
  "_meta": {
    "gateway": {
      "baseUrl": "https://my-gateway.example.com/v1",
      "headers": {"X-Tenant-Id": "abc-123"}
    }
  }
}
```

A malformed `gateway` payload yields JSON-RPC `-32603 Malformed gateway payload: <details>`.

### Headless preflight

For non-interactive launches, the easiest path is to set the env var before launching `gemini --acp`:

```bash
export GEMINI_API_KEY="AIzaSy..."        # Gemini Developer API
export GOOGLE_GENAI_USE_VERTEXAI=true    # Vertex AI (also set GOOGLE_CLOUD_PROJECT, etc.)
export GOOGLE_GEMINI_BASE_URL="https://my-gateway.example.com/v1"  # Custom gateway
```

When `GOOGLE_GEMINI_BASE_URL` is set, the agent defaults the auth method to `gateway`. Otherwise it defaults to `gemini-api-key`. OAuth credentials cached in `~/.gemini/oauth_creds.json` are auto-resumed (no interactive login required).

### Telemetry

For CI debugging, `GEMINI_TELEMETRY_ENABLED=true GEMINI_TELEMETRY_TARGET=local GEMINI_TELEMETRY_OUTFILE=/tmp/gemini-acp.json` writes a JSON event log of every ACP request/response. The `integration-tests/acp-telemetry.test.ts` test in the upstream monorepo demonstrates this pattern.

## Compatibility, Quirks, and Workarounds

1. **`AcpAgent::google_gemini()` preset is stale.** As of `agent-client-protocol 1.0.1` (June 29, 2026) the preset shells to `npx -y -- @google/gemini-cli@latest --experimental-acp`. It (a) re-installs the package on every launch, (b) uses the deprecated `--experimental-acp` flag, (c) depends on network access, and (d) silently upgrades the agent. Workaround: `AcpAgent::from_str("gemini --acp")`.
2. **stderr noise from skill loader.** Startup emits `Skill conflict detected: "<name>" from ... is overriding ...` lines to stderr before the agent finishes initializing. stdout is clean; clients reading only stdout are unaffected. The simplest mitigation is to silence stderr on the client side or route it through a separate log channel.
3. **`protocolVersion` is an integer.** Live probes return `protocolVersion: 1` — not a `1.x.x` schema tag. Clients should compare against the integer 1, not parse it as a version string.
4. **`fs/read_text_file` requires string `content`.** The agent throws `Error("content must be a string")` if `content` is non-string (number, object, null). Always reply with a UTF-8 string.
5. **Paths outside the session cwd bypass fs capabilities.** `acpFileSystemService.shouldUseFallback` is hard-coded to fall back when the path is outside the session root OR inside `~/.gemini/`. Clients cannot intercept these reads/writes even with `readTextFile: true`.
6. **`session/set_model` is exposed under the unstable method name `unstable_setSessionModel`.** The JSON-RPC method is `session/set_model`; the agent class method is `unstable_setSessionModel`. The SDK routes between them.
7. **No protocol-level `$/cancel_request` observed.** The agent cancels only via `session/cancel`. Clients that listen for `$/cancel_request` to cancel arbitrary in-flight requests will not see one from this provider.
8. **Tool kinds are Gemini's, not ACP's.** Gemini's internal `agent` / `plan` / `communicate` kinds are folded into `think` / `other` by `toAcpToolKind` before they hit the wire.
9. **`session/load` is fully functional but the live probe did not exercise it.** Behavior is inferred from `loadSession: true` in `agentCapabilities` plus the `loadSession` method in source.
10. **OAuth cache `~/.gemini/oauth_creds.json` is silently resumed.** A headless ACP launch will reuse any cached Google OAuth tokens without prompting. Useful for unattended daemons; surprising if you expected a fresh login.
11. **`enablePermanentToolApproval` is controlled by config, not a flag.** Source reads `config.getDisableAlwaysAllow()` — clients cannot force the `_and_save` option to appear via JSON-RPC; it's gated on the agent's local config.
12. **Sandboxed shell exec uses `node-pty` or Docker/Podman depending on `GEMINI_SANDBOX`.** Output and reverse-request behavior are identical, but lifecycle/cleanup semantics differ (Docker/Podman wrapper adds a layer of process management).

## Recent Changes

- **2026-06-25 — v0.49.0 stable.** First stable release of native ACP. The CLI exposes `--acp` and the deprecated `--experimental-acp`. The bundled SDK provides `acpStdioTransport`, `AcpFileSystemService`, and `AcpSessionManager`. Listed in the [ACP Agent Registry](https://agentclientprotocol.com/get-started/registry).
- **2026-04-10 — ACP Mode docs published.** [geminicli.com/docs/cli/acp-mode](https://geminicli.com/docs/cli/acp-mode/) ships with the canonical launch instructions.
- **2026-04 (pre-0.42) — Native ACP support lands.** The original `--experimental-acp` flag was added. Built directly on `@agentclientprotocol/sdk`; no bridge package.
- **2026-06-29 — `agent-client-protocol` v1.0.1 (Rust SDK).** First stable Rust SDK reaches Send/Sync connection types and ships the `AcpAgent::google_gemini()` preset (still uses the deprecated `--experimental-acp` flag).
- **2026-06-25 → 2026-07-03 — nightly v0.51.0 series.** Ongoing work tracked on the [releases page](https://github.com/google-gemini/gemini-cli/releases); v0.51.0-nightly.20260703.gf7af4e518 is the latest at research time. Recent commits touch Cloud Run egress, symbolic-link path resolution in memory imports, and Gemini API base URL updates for Vertex — none of these change the ACP surface.

## Rust Client Example

This example uses `agent-client-protocol 1.0.1` with the *direct* `AcpAgent::from_str("gemini --acp")` launcher (not the stale preset).

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
        TextContent,
    },
};
use agent_client_protocol::{AcpAgent, Client};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Direct binary launch — bypass the stale AcpAgent::google_gemini() preset.
    let agent = AcpAgent::from_str("gemini --acp")?;

    Client
        .builder()
        .name("claudine-gemini-client")
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
                    SessionNotification::CurrentModeUpdate(m) => {
                        eprintln!("[mode → {}]", m.mode_id);
                    }
                    _ => {}
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(agent, |connection| async move {
            let caps = ClientCapabilities::new()
                .fs(FileSystemCapabilities {
                    read_text_file: true,
                    write_text_file: true,
                })
                .terminal(true);

            let init = InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(caps)
                .client_info(Implementation {
                    name: "claudine".into(),
                    title: Some("Claudine".into()),
                    version: "0.1.0".into(),
                });

            let init_response = connection.send_request(init).block_task().await?;
            eprintln!(
                "Agent: {:?} v{} | auth methods: {:?}",
                init_response.agent_info.as_ref().map(|i| &i.title),
                init_response.agent_info.as_ref().map(|i| &i.version).cloned().unwrap_or_default(),
                init_response.auth_methods.iter().map(|m| &m.id).collect::<Vec<_>>(),
            );

            let session = connection
                .send_request(NewSessionRequest::new(
                    std::env::current_dir()?,
                    vec![],
                ))
                .block_task()
                .await?;

            eprintln!("Session {} (modes: {:?})", session.session_id, session.modes);

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

`session/request_permission` is required; the `fs/*` and `terminal/*` handlers are only relevant when the client advertises the corresponding capabilities.

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
    // Claudine policy could map optionId → PolicyEngine decision here.
    // For the snippet, default to allow_once.
    let option_id = request
        .options
        .iter()
        .find(|o| o.option_id == "proceed_once")
        .or_else(|| request.options.first())
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

Register on the builder before `connect_with`:

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

The full `terminal/*` surface, including lifecycle management:

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
            child: Some(child),
            stdout_buf: Vec::new(),
            stderr_buf: Vec::new(),
            exited: false,
            exit_code: None,
            output_limit: limit,
        },
    );

    Ok(CreateTerminalResponse { terminal_id: id })
}

// handle_terminal_output / handle_wait_for_exit / handle_kill / handle_release
// follow the same pattern: lock the manager, mutate the handle, return the
// corresponding response. Always invoke release to free the handle.
```

Always implement `terminal/release` — skipping it leaks process slots and pollutes the client-side handle map.

## Rust Desktop Streaming Bridge

Forward `session/update` notifications to a desktop UI through an `mpsc` channel:

```rust
use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ContentBlock, Implementation, InitializeRequest, NewSessionRequest,
        PromptRequest, SessionNotification, TextContent,
    },
};
use agent_client_protocol::{AcpAgent, Client};
use std::str::FromStr;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextChunk { session_id: String, text: String },
    ThoughtChunk { session_id: String, text: String },
    ToolCallStarted { session_id: String, tool_call_id: String, title: String },
    ToolCallUpdated { session_id: String, tool_call_id: String, status: String },
    ModeUpdate { session_id: String, mode_id: String },
    AvailableCommands { session_id: String, names: Vec<String> },
    TurnComplete { session_id: String, stop_reason: String },
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
            let agent = AcpAgent::from_str("gemini --acp")?;

            Client
                .builder()
                .name("claudine-desktop")
                .on_receive_notification(
                    {
                        let tx = event_tx.clone();
                        move |notification: SessionNotification, _cx| {
                            let tx = tx.clone();
                            let session_id = notification.session_id.to_string();
                            async move {
                                let event = match notification.update {
                                    SessionNotification::AgentMessageChunk(chunk) => {
                                        if let ContentBlock::Text(t) = chunk.content {
                                            Some(AgentEvent::TextChunk {
                                                session_id: session_id.clone(),
                                                text: t.text,
                                            })
                                        } else {
                                            None
                                        }
                                    }
                                    SessionNotification::AgentThoughtChunk(chunk) => {
                                        if let ContentBlock::Text(t) = chunk.content {
                                            Some(AgentEvent::ThoughtChunk {
                                                session_id: session_id.clone(),
                                                text: t.text,
                                            })
                                        } else {
                                            None
                                        }
                                    }
                                    SessionNotification::ToolCall(tc) => Some(AgentEvent::ToolCallStarted {
                                        session_id: session_id.clone(),
                                        tool_call_id: tc.tool_call_id.to_string(),
                                        title: tc.title,
                                    }),
                                    SessionNotification::ToolCallUpdate(u) => Some(AgentEvent::ToolCallUpdated {
                                        session_id: session_id.clone(),
                                        tool_call_id: u.tool_call_id.to_string(),
                                        status: format!("{:?}", u.status),
                                    }),
                                    SessionNotification::CurrentModeUpdate(m) => Some(AgentEvent::ModeUpdate {
                                        session_id: session_id.clone(),
                                        mode_id: m.mode_id,
                                    }),
                                    SessionNotification::AvailableCommandsUpdate(u) => {
                                        let names = u.available_commands.iter().map(|c| c.name.clone()).collect();
                                        Some(AgentEvent::AvailableCommands {
                                            session_id: session_id.clone(),
                                            names,
                                        })
                                    }
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
                        .send_request(
                            InitializeRequest::new(ProtocolVersion::V1)
                                .client_info(Implementation {
                                    name: "claudine-desktop".into(),
                                    title: Some("Claudine Desktop".into()),
                                    version: "0.1.0".into(),
                                }),
                        )
                        .block_task()
                        .await?;

                    let session = connection
                        .send_request(NewSessionRequest::new(project_dir, vec![]))
                        .block_task()
                        .await?;
                    let session_id = session.session_id.clone();

                    while let Some(prompt) = prompt_rx.recv().await {
                        match connection
                            .send_request(PromptRequest::new(
                                session_id.clone(),
                                vec![ContentBlock::Text(TextContent::new(prompt))],
                            ))
                            .block_task()
                            .await
                        {
                            Ok(response) => {
                                let _ = event_tx.send(AgentEvent::TurnComplete {
                                    session_id: session_id.to_string(),
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
async fn send_prompt(
    state: tauri::State<'_, AppState>,
    prompt: String,
) -> Result<(), String> {
    state.prompt_tx.send(prompt).map_err(|e| e.to_string())
}

fn listen(event_rx: mpsc::UnboundedReceiver<AgentEvent>, handle: tauri::AppHandle) {
    tokio::spawn(async move {
        let mut rx = event_rx;
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::TextChunk { session_id, text } => {
                    handle.emit("agent:text", (session_id, text)).ok();
                }
                AgentEvent::TurnComplete { session_id, stop_reason } => {
                    handle.emit("agent:done", (session_id, stop_reason)).ok();
                }
                AgentEvent::ToolCallStarted { session_id, tool_call_id, title } => {
                    handle.emit("agent:tool", (session_id, tool_call_id, title)).ok();
                }
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

Adding Gemini CLI as an ACP provider to Claudine is materially easier than Claude Code (no adapter) but introduces net-new concepts Claudine's provider model doesn't yet model.

1. **Launch detection.** Extend `sniff` to recognize `gemini` and probe `--version`. Set `gemini --acp` as the canonical launcher; bypass the Rust SDK's `AcpAgent::google_gemini()` preset by passing `AcpAgent::from_str("gemini --acp")` instead.
2. **Capability negotiation.** Advertise `clientCapabilities.fs.{readTextFile, writeTextFile}` and `terminal: true` so the agent routes file and shell operations through Claudine. Without this, the agent falls back to its built-in file system service (read-only against `~/.gemini/`) and its own node-pty shell, bypassing Claudine's policy and shell-audit layers.
3. **Auth preflight.** Before launching, ensure at least one of: `GEMINI_API_KEY`, `GOOGLE_GENAI_USE_VERTEXAI=1`, `GOOGLE_GEMINI_BASE_URL`, or cached `~/.gemini/oauth_creds.json`. For unattended daemons, prefer the API-key or gateway env paths. If the agent reports `-32000 Authentication required.`, drive `authenticate({methodId, _meta})` based on the advertised `authMethods` and retry `session/new`.
4. **Permission model mapping.** Translate the Gemini option set (`proceed_once`, `proceed_always`, `proceed_always_and_save`, `proceed_always_server`, `proceed_always_tool`, `cancel`) into Claudine's `PolicyEngine` decisions. The `_always` options should write a Claudine-level allow rule; `cancel` should map to a deny. Disable `_always` options when the host's policy mode is `yolo` or `auto_edit`.
5. **Reverse-request routing.** Wire `session/request_permission`, `fs/read_text_file`, `fs/write_text_file`, and the full `terminal/*` lifecycle into the same handler surface Claudine uses for other agents. Enforce path policy (sandbox root, `~/.gemini/` exclusion) at the boundary.
6. **Streaming bridge.** Forward `session/update` notifications into the lifecycle pipeline. Group by `sessionId` and dispatch by `update.sessionUpdate`. The variants Claudine's other providers emit are a strict subset of Gemini's — `agent_message_chunk`, `agent_thought_chunk`, `tool_call`, `tool_call_update` need first-class routing; `plan`, `current_mode_update`, `config_option_update`, `session_info_update`, `usage_update`, and `available_commands_update` are Gemini-specific additions.
7. **stderr hygiene.** Route `gemini --acp`'s stderr to a separate channel (not the JSON-RPC parser). Startup emits `Skill conflict detected:` warnings that are noise from the protocol's perspective.
8. **Schema versioning.** Accept `protocolVersion: 1` as the only contract — don't try to compare schema tag strings. Reserve `unstable_setSessionModel` for explicit opt-in (it's not in the published schema).
9. **Session modes / models.** The `session/new` response carries `modes` (default/autoEdit/yolo/plan) and `models` (the model catalog). Claudine should expose both as first-class UI controls and translate `--approval-mode` and `--model` flag values into `session/set_mode` and `session/set_model` calls.
10. **Lifecycle interop.** The agent supports `--experimental-acp` for backward compatibility but emits a deprecation notice; warn users once if the host sees the deprecated flag in `argv`, then transparently rewrite to `--acp`.

Because Gemini CLI has the cleanest ACP story in the provider roster, it's the right benchmark for the ACP code path in Claudine. Once that path is solid, the same wiring generalizes to any future provider that ships native ACP (Claude Code is the obvious pending migration if Anthropic ever adopts it).

## Changelog

- **2026-07-03**: Initial research document for Gemini CLI ACP support. Verified `gemini --acp` against the installed v0.46.0 binary (live handshake, modes, models, available_commands_update). Cross-referenced against `agent-client-protocol 1.0.1` (`AcpAgent::google_gemini()` preset is stale). Recorded native ACP support, four advertised authMethods (`oauth-personal`, `gemini-api-key`, `vertex-ai`, `gateway`), ten `session/update` variants, and the full `terminal/*` lifecycle. Documented the `_meta`-shaped credentials, the `toPermissionOptions` option set, the `acpFileSystemService.shouldUseFallback` boundary, and the stderr startup noise.

## Sources

- [Gemini CLI homepage](https://geminicli.com/)
- [Gemini CLI repository](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI ACP Mode developer guide](https://geminicli.com/docs/cli/acp-mode/)
- [Gemini CLI IDE Integration overview](https://geminicli.com/docs/ide-integration/)
- [Gemini CLI Headless mode docs](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI Authentication docs](https://geminicli.com/docs/get-started/authentication/)
- [Gemini CLI Configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI releases](https://github.com/google-gemini/gemini-cli/releases)
- [Gemini CLI v0.49.0 changelog](https://github.com/google-gemini/gemini-cli/releases/tag/v0.49.0)
- [Agent Client Protocol home](https://agentclientprotocol.com/)
- [ACP Introduction](https://agentclientprotocol.com/overview/introduction)
- [ACP Schema reference (v1.x)](https://agentclientprotocol.com/protocol/schema)
- [ACP Agent Registry](https://agentclientprotocol.com/get-started/registry)
- [ACP Rust SDK home](https://github.com/agentclientprotocol/rust-sdk)
- [`agent-client-protocol` 1.0.1 on docs.rs](https://docs.rs/agent-client-protocol/1.0.1/agent_client_protocol/) — `AcpAgent::google_gemini()` preset at [`role/acp/acp_agent.rs`](https://docs.rs/agent-client-protocol/1.0.1/src/agent_client_protocol/acp_agent.rs.html#103-106)
- [`agent-client-protocol-schema` 1.1.0 on docs.rs](https://docs.rs/agent-client-protocol-schema/1.1.0/agent_client_protocol_schema/)
- [`@agentclientprotocol/sdk` on npm](https://www.npmjs.com/package/@agentclientprotocol/sdk)
- [Local inspection] `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/gemini-STIONCRJ.js` — `acpStdioTransport.ts` (line ~15,385), `acpFileSystemService.ts` (~14,919), `acpSessionManager.ts` (~14,975), `GeminiAgent.initialize` (~15,220), `toPermissionOptions` (~13,507), `toAcpToolKind` (~13,599), `buildAvailableModes` (~13,620), `buildAvailableModels` (~13,647)
- [Local inspection] `/Users/ken/.gemini/oauth_creds.json` — cached Google OAuth tokens confirming headless resumption
- [Local inspection] `/Users/ken/.gemini/settings.json` — `{general.previewFeatures: true, security.auth.selectedType: "gemini-api-key", ...}`
- [Live probe] `gemini --acp < initialize.json` — full ACP handshake returned `protocolVersion: 1`, four auth methods, full modes/models payload, and an `available_commands_update` notification