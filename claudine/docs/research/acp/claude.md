---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3
docs: https://code.claude.com/docs/en/overview
acp_docs: https://agentclientprotocol.com/
repo: https://github.com/anthropics/claude-code
support: adapter
launch_modes:
  - command: npx -y @agentclientprotocol/claude-agent-acp@latest
    args: []
    transport: stdio
    adapter: "@agentclientprotocol/claude-agent-acp (TypeScript) — the official community adapter, now under the agentclientprotocol org"
    notes: "Official-recommended launch. The package was renamed from @zed-industries/claude-agent-acp in v0.24.0 (April 2026) and is maintained at github.com/agentclientprotocol/claude-agent-acp. Spawns the underlying `claude` CLI via the Claude Agent SDK and translates ACP JSON-RPC to/from the SDK's NDJSON stream."
  - command: npx -y @zed-industries/claude-agent-acp@latest
    args: []
    transport: stdio
    adapter: "@zed-industries/claude-agent-acp (TypeScript) — Zed-published alias used November 2024–April 2026 (versions 0.17.0 through 0.23.x)"
    notes: "Still works for now (npm keeps the old name available), but the canonical name is the @agentclientprotocol one. Same adapter under both names."
  - command: npx -y @zed-industries/claude-code-acp@latest
    args: []
    transport: stdio
    adapter: "@zed-industries/claude-code-acp (TypeScript) — original packaging, pinned to v0.16.2 max"
    notes: "DEPRECATED. The Rust SDK preset `AcpAgent::zed_claude_code()` still calls this exact command, so anyone relying on the SDK preset is silently running a July-2025-era adapter (v0.16.2 at best), missing 31+ releases of improvements (auth/logout, fast mode, refusal fallback, 1M-context inference, cancellation, tool_call-before-permission, etc.). Workaround: use `AcpAgent::from_str(\"npx -y @agentclientprotocol/claude-agent-acp@latest\")` instead."
  - command: cargo install claude-code-acp-rs && claude-code-acp-rs
    args: []
    transport: stdio
    adapter: claude-code-acp-rs (Rust, community)
    notes: "Community Rust bridge. The crates.io name `claude-code-acp-rs` could not be fetched through the crates.io API at research time, so version, last update, and current maintenance status are `unknown`. Use with caution."
  - command: claude
    args: []
    transport: other
    adapter: none
    notes: "Negative probe: `claude --acp` returns `error: unknown option '--acp'`, and `claude acp` returns `unknown command \"acp\"` with a did-you-mean pointing at `claude mcp`. The `claude` CLI is a Bun-bundled JavaScript binary (BUILD_TIME 2026-07-03) and ships no native ACP entry point. Anthropic closed the upstream feature request anthropics/claude-code#6686 as `not planned` because community adapters exist."
protocol_versions:
  - "v1 (schema 1.1.0)"
capabilities:
  - capability: initialize
    support: supported
    notes: "Standard ACP `initialize` handshake with `ProtocolVersion` negotiation, `ClientCapabilities`, `Implementation`, and `authMethods`. The Anthropic/Anthropic Agent SDK adapter advertises `authMethods` (Claude login, Console login, Bedrock gateway) so the client knows what `authenticate` flows are available."
  - capability: authenticate
    support: supported
    notes: "`authenticate` request added to schema v1.1.0 (alongside the existing `auth_required` error path on `session/new`). Adapter landed ACP logout in v0.53.0 (2026-06-29) and added refusal-fallback consent dialog support in v0.55.0 (2026-07-02)."
  - capability: session_new
    support: supported
    notes: "`session/new` creates a conversation session tied to a working directory (`cwd` is required and must be an absolute path). New in v1.1.0 schema: `additionalDirectories` (absolute paths) and `mcpServers`."
  - capability: session_load
    support: supported
    notes: "`session/load` resumes an existing Claude Code session, agent must advertise `loadSession`. Adapter streams the conversation history back as notifications."
  - capability: session_cancel
    support: supported
    notes: "`session/cancel` is a notification that stops the current turn; the agent responds with `StopReason::Cancelled`. Supports protocol-level `$/cancel_request` for cancelling any pending request (added in v1.1.0; adapter support landed in v0.50.0 with force-cancellation on SDK query hang in v0.41.0)."
  - capability: session_prompt
    support: supported
    notes: "`session/prompt` is the primary turn-taking request. `prompt` is `ContentBlock[]`; the agent is required to accept `Text` and `ResourceLink`, and may accept `Image`/`Audio`/`EmbeddedContext` if it advertises those `promptCapabilities`."
  - capability: session_modes
    support: supported
    notes: "Schema v1.1.0 formal `session/set_mode` and `current_mode_update` notification. Adapter emits initial `modes` in `NewSessionResponse`/`LoadSessionResponse` (effort_level aka thought_level, model, permission_mode, fast mode) and emits `current_mode_update` (and the new session title update) at turn end."
  - capability: streaming
    support: supported
    notes: "`session/update` notifications stream `agent_message_chunk`, `agent_thought_chunk`, `user_message_chunk`, `tool_call`, `tool_call_update`, `plan`, `current_mode_update`, `available_commands_update`, and `config_option_update`. Updates are fire-and-forget JSON-RPC notifications (no `id`); group by `ContentChunk.message_id`."
  - capability: permissions
    support: partial
    notes: "`session/request_permission` reverse request is fully supported. CRITICAL CHANGE since v0.18.0: the adapter now delegates tool dispatch to Claude's BUILT-IN tools and no longer issues `fs/*` or `terminal/*` reverse requests even when the client advertises those capabilities. Clients still see permission prompts for write/Bash/etc., but file reads/writes and command execution happen in the SDK, not through the client. The documentation that framed those as \"delegated to the client\" is no longer accurate."
  - capability: fs_read
    support: unsupported
    notes: "Adapter (v0.18.0+) sends a read content `tool_call` instead of `fs/read_text_file`. The schema v1.1.0 method still exists and is supported by the SDK, but the official adapter does not use it — clients should not advertise `fs.readTextFile: true` expecting it to be honored by this adapter."
  - capability: fs_write
    support: unsupported
    notes: "Same as fs_read — the adapter surfaces writes via Claude's native Write/Edit tool calls rather than `fs/write_text_file`."
  - capability: terminal
    support: unsupported
    notes: "Adapter (v0.18.0+) surfaces Bash/PowerShell calls via Claude's built-in Bash tool rather than `terminal/create`. CLI commands can still appear in the agent transcript (and the adapter supports interactive/background terminals) but they bypass the ACP reverse-request lifecycle."
  - capability: mcp
    support: supported
    notes: "Schema v1.1.0 supports `mcpCapabilities.http` and `mcpCapabilities.sse`. Clients can pass an `McpServer[]` in `session/new`. The adapter also exposes Claude-Code MCP servers over `claude mcp` (the `claude` CLI manages them itself; this is the legacy MCP path used when not running under ACP)."
  - capability: plans
    support: supported
    notes: "`Plan` session update emitted when the agent writes a plan; the TypeScript adapter errors out in pure plan-mode (where the user reviews a plan before executing) because the Claude Agent SDK does not fully support plan mode. Normal streaming of plan entries as the agent works is fine."
  - capability: media
    support: partial
    notes: "`promptCapabilities.image` and `promptCapabilities.audio` are negotiated; the underlying Claude Agent SDK accepts images. MCP-tool output that is an image is now surfaced (v0.48.0 fixed dropped Bash image output)."
  - capability: extensions
    support: supported
    notes: "ACP `ext_method` / `ext_notification` / `_meta` fields are supported. Adapter currently uses `_meta` for Claude-specific extensions (e.g. `acp.mcp_elicitation`, experimental `additionalDirectories`, raw SDK passthrough)."
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: "Required for any tool-call approval flow. The client must present options and return a selected `option_id` via `RequestPermissionOutcome::Selected`, or `RequestPermissionOutcome::Cancelled` if the user cancels."
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: "Optional. The official `@agentclientprotocol/claude-agent-acp` adapter no longer issues this request (built-in tool path since v0.18.0). Implement only if you intend to support other agents, or as best-effort for older adapter versions."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: "Optional. Same caveat as fs_read."
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: "Optional. Adapter prefers built-in Bash tool since v0.18.0."
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: "Optional; only relevant if terminal/create is used."
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: "Optional; only relevant if terminal/create is used."
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: "Optional; only relevant if terminal/create is used."
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: false
    notes: "Optional; only relevant if terminal/create is used. ALWAYS call release on any terminal handle the adapter returned, to avoid handle leaks."
permission_model:
  mechanism: session/request_permission reverse request
  timeout: client-defined
  default_policy: no default; every tool call requiring approval must receive a Selected or Cancelled response
  approval_values:
    - allow_once
    - allow_always
    - reject_once
  notes: "The client receives the tool call details, optional allowed/denied category context, and a list of `PermissionOption` entries. Reply with one selected optionId or Cancelled. On `session/cancel`, the client MUST respond Cancelled to any pending permission requests (per schema v1.1.0)."
filesystem_model:
  read_methods:
    - fs/read_text_file (theoretical; not issued by current adapter)
  write_methods:
    - fs/write_text_file (theoretical; not issued by current adapter)
  path_base: absolute paths only
  sandboxing: client-side; the client decides whether to enforce a project-root boundary
  notes: "ACP requires absolute paths and 1-based line numbers. With the current TypeScript adapter, reads/writes are performed by Claude's built-in tools (Read/Edit/Write) inside the agent process — the client has no opportunity to sandbox paths at the filesystem boundary. Path policy must be enforced through Claude's permission system (settings.json rules, sandbox settings)."
terminal_model:
  supported: true
  methods:
    - terminal/create
    - terminal/output
    - terminal/wait_for_exit
    - terminal/kill
    - terminal/release
  shell: "depends on host; Claude Code uses Bash on macOS/Linux and PowerShell on Windows when Git for Windows is absent"
  cwd: "absolute path supplied in CreateTerminalRequest"
  streaming: "polled via terminal/output"
  cancellation: "terminal/kill or terminal/release"
  notes: "Schema-level support is complete; the TypeScript adapter does NOT use these methods (it executes commands through Claude's built-in Bash tool). Implement these handlers only as a matter of general ACP completeness for other adapters, or to support older versions of the TypeScript adapter (pre-v0.18.0)."
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
    - "session/update does not carry errors; JSON-RPC errors are returned on the request channel"
    - "$/cancel_request (v1.1.0) cancels in-flight requests from either side"
  notes: "Updates are fire-and-forget notifications with no `id`. Use the `ContentChunk.message_id` field to group chunks belonging to the same message. v0.53.0 fixed the order so the `tool_call` arrives BEFORE the `session/request_permission` reverse request — clients should be prepared to show the tool call detail along with the prompt."
auth_setup:
  required: true
  mechanisms:
    - "Claude Code login prompt on first use (interactive OAuth)"
    - "ANTHROPIC_API_KEY for API-key auth"
    - "Pre-authenticated Claude Code session state (~/.claude/)"
    - "Bedrock, Vertex, Foundry via dedicated env vars"
    - "Bedrock gateway authentication via awsAuthRefresh (added to adapter v0.34.0)"
    - "`authMethods` advertised by the adapter at initialize (Claude login, Console login, Bedrock) so the client can request `authenticate` directly"
  headless_notes: "For fully headless ACP operation, set ANTHROPIC_API_KEY or ensure the Claude Code CLI has already completed OAuth and stored its session. The adapter inherits whatever auth the CLI has, but now also exposes an `authenticate` method so a headless client can drive login itself rather than relying on the agent to fail with `auth_required`."
  notes: "Bedrock `awsAuthRefresh` config can write human-readable status messages to stdout and corrupt the adapter's NDJSON stream — known and documented quirk; remove the setting or filter non-JSON lines. Adapter added TUI login for remote environments in v0.26.0."
env_vars:
  - name: ANTHROPIC_API_KEY
    effect: "Allows the Claude CLI to authenticate without interactive login."
  - name: ANTHROPIC_BASE_URL
    effect: "Optional proxy or alternative endpoint for Anthropic API calls."
  - name: ANTHROPIC_MODEL
    effect: "Pin a specific model for the session (the adapter prioritizes this over settings.model)."
  - name: ANTHROPIC_SMALL_FAST_MODEL
    effect: "Pin the small/fast model used for haiku-class tasks."
  - name: ANTHROPIC_DEFAULT_OPUS_MODEL / ANTHROPIC_DEFAULT_SONNET_MODEL
    effect: "Pin a specific Opus/Sonnet version (1M-context suffix is auto-normalized)."
  - name: CLAUDE_CODE_DEBUG
    effect: "Enables adapter/CLI debug logging; emitted to stderr to avoid corrupting stdio JSON-RPC."
  - name: NO_BROWSER
    effect: "Removes browser-based auth paths from advertised methods; useful in headless environments."
  - name: CLAUDE_CODE_SHELL
    effect: "Override the shell used by Claude Code; if set to bash/zsh and executable, used preferentially."
  - name: CLAUDE_CONFIG_DIR
    effect: "Override where Claude Code keeps config files (the adapter reads this for settings.json / managed-settings.json)."
  - name: CLAUDE_CODE_EXECUTABLE
    effect: "Override the binary path used by the Claude Agent SDK inside the adapter (useful for self-hosted/static binaries)."
  - name: awsAuthRefresh
    effect: "When configured in Claude Code settings, can also write human-readable status messages to stdout and corrupt the NDJSON stream. Adapter added Bedrock gateway support in v0.34.0."
  - name: CLAUDECODE=1
    effect: "Appended to terminal commands spawned by the adapter to match default Claude Code behavior."
rust_client:
  crate: agent-client-protocol
  connection_type: AcpAgent subprocess over stdio (JSON-RPC)
  localset_required: false
  reverse_request_handlers:
    - session/request_permission
  desktop_streaming_pattern: "tokio::sync::mpsc from the notification handler to the UI thread; run the ACP client on a dedicated tokio runtime"
  notes: "agent-client-protocol 1.0.1 (June 29, 2026) is the current Rust SDK. The connection is `Send`/`Sync`, so `tokio::task::LocalSet` is no longer required. AcpAgent::zed_claude_code() PRESET IS STALE — it shells to the deprecated `@zed-industries/claude-code-acp@latest` npm package, which is pinned to v0.16.2 and lacks every improvement since. Prefer `AcpAgent::from_str(\"npx -y @agentclientprotocol/claude-agent-acp@latest\")` until upstream updates the preset. Other v1.0.1 sub-crates: agent-client-protocol-derive, agent-client-protocol-schema (=1.1.0), agent-client-protocol-cookbook, agent-client-protocol-conductor, agent-client-protocol-http, agent-client-protocol-polyfill, agent-client-protocol-rmcp, agent-client-protocol-trace-viewer."
compatibility:
  - client: Zed
    status: works
    issue: "Zed remains the canonical client; the Zed adapter was renamed alongside this package in April 2026."
    workaround: "Zed's built-in \"Claude Code\" integration fetches the adapter from the new namespace automatically."
  - client: JetBrains IDEs
    status: partial
    issue: "Requires `~/.jetbrains/acp.json` configuration and a separately installed adapter."
    workaround: "Point the registry at the `@agentclientprotocol/claude-agent-acp` stdio command (the `@zed-industries/...` name may still work via npm aliases)."
  - client: Neovim (CodeCompanion)
    status: works
    issue: "none known"
    workaround: "Configure the adapter as a stdio ACP agent."
  - client: agent-client-protocol Rust SDK 0.9.x
    status: broken
    issue: "Connection futures were !Send and required LocalSet."
    workaround: "Upgrade to agent-client-protocol 1.0.1 or later."
  - client: agent-client-protocol Rust SDK 1.0.x
    status: works
    issue: "AcpAgent::zed_claude_code() preset points to a deprecated npm package (`@zed-industries/claude-code-acp` ≤ v0.16.2), missing 31+ adapter releases."
    workaround: "Build the AcpAgent manually via `AcpAgent::from_str` with the current npm name, or wait for the next SDK release to refresh the preset."
recent_changes:
  - date: 2026-07-02
    version: "@agentclientprotocol/claude-agent-acp v0.55.0"
    change: "Added refusal-fallback consent dialog support; bumped Claude Agent SDK to 0.3.198. Adapter now flagged `cancelled` instead of `end_turn` on session interruption."
    impact: "Clients can implement a refusal fallback when the model refuses to act; cancellation mapping is now distinct from end-of-turn so UIs can show different visuals."
  - date: 2026-06-30
    version: "adapter v0.54.0 / v0.54.1"
    change: "Fast-mode session config support and a fix applying modelOverrides when resolving the availableModels allowlist."
    impact: "Clients can expose a 'fast mode' toggle through ACP `SessionConfigOption`."
  - date: 2026-06-29
    version: "adapter v0.53.0"
    change: "Added ACP `logout` request handling. Bumped Claude Agent SDK to 0.3.195."
    impact: "Adapter can now clear the local session credentials when a client asks it to."
  - date: 2026-06-29
    version: "agent-client-protocol-v1.0.1 + agent-client-protocol-schema 1.1.0"
    change: "Official Rust SDK reached 1.0.x; Send/Sync connection types; builder-based API; preset constructors `AcpAgent::zed_claude_code()`, `zed_codex()`, `google_gemini()`. Schema bumped to 1.1.0 which adds `authenticate`/`logout`, `session/close`/`delete`/`list`/`resume`, `AgentAuthCapabilities`, `SessionCapabilities`, `SessionConfigOption`, and protocol-level `$/cancel_request`."
    impact: "All of the Rust code samples in the prior research used legacy MessageHandler APIs; modern code uses `Client::builder().on_receive_request(...).on_receive_notification(...).connect_with(transport, async move |cx| { ... })`. Schema version is **1.1.0**, not 1.2.0 as the prior research recorded."
  - date: 2026-06-25
    version: "adapter v0.52.0"
    change: "Added `--version`/`-v` flag handling and pushed session title updates at turn end."
    impact: "Version discovery is now over a single command. Clients can render session titles as they update over time."
  - date: 2026-06-23
    version: "adapter v0.50.0"
    change: "Added ACP request cancellation signals (`$/cancel_request`)."
    impact: "The agent can now propagate cancellation to any in-flight request rather than only to the active prompt turn."
  - date: 2026-04-15
    version: "adapter v0.24.0"
    change: "Repository moved from `zed-industries/claude-agent-acp` to `agentclientprotocol/claude-agent-acp` and the npm package renamed to `@agentclientprotocol/claude-agent-acp`."
    impact: "Custom deployment scripts that hard-coded the older namespace need to update."
quirks:
  - "Claude Code's `claude` CLI has no native ACP mode. Verified on 2.1.200 (BUILD_TIME 2026-07-03): `claude --acp` → `error: unknown option '--acp'`; `claude acp` → `unknown command \"acp\"` (did-you-mean `claude mcp`). No fix is expected — anthropics/claude-code#6686 closed as `not planned`."
  - "The Rust SDK preset `AcpAgent::zed_claude_code()` invokes `npx -y @zed-industries/claude-code-acp@latest`, a package whose last release was v0.16.2 in 2025. Pinned to that ancient version, it is missing auth methods, fast mode, refusal fallback, session/delete, 1M-context inference, `$/cancel_request` handling, and many other improvements. Use `AcpAgent::from_str(...)` with the new namespace instead."
  - "`v0.18.0` of the adapter (`Switch over to built-in Claude tools`) deliberately stopped replicating ACP filesystem/terminal reverse requests. Even if a client advertises `fs.readTextFile: true` and `terminal: true`, the adapter handles read/write/execute internally through the Claude Agent SDK. Implement those handlers as a general ACP client, but do not expect them to fire with the current adapter."
  - "`$` permission requests fire BEFORE the corresponding `session/update` for the tool call lands in some versions of the adapter; v0.53.0 fixed the ordering so `tool_call` arrives first. Both orderings are seen in the wild depending on adapter version."
  - "AWS Bedrock auth refresh (`awsAuthRefresh`) writes human-readable status messages to stdout and corrupts the NDJSON stream. Remove that setting or filter non-JSON lines."
  - "Plan-mode error in pure plan-mode (where the user reviews a plan before executing it) because the Claude Agent SDK's plan mode is not yet first-class. Streaming plan entries as the agent works is unaffected."
  - "Sessions default to `haiku` (or whatever the user's Default resolves to) on first launch; expose a SessionConfigOption or set `ANTHROPIC_MODEL` explicitly."
  - "Terminal handle leaks occur if `terminal/release` is skipped — relevant even for the current adapter because the Claude-built-in Bash tool can fail to terminate a process inside an MCP server that itself uses the terminal reverse request."
  - "`session/cancel` requires the client to reply with `Cancelled` to any pending `session/request_permission` reverse requests, per schema v1.1.0."
  - "Relative paths and 0-based indexing are common integration bugs — ACP requires absolute paths and 1-based line numbers."
gaps:
  - "No official Anthropic-maintained ACP adapter; reliance on Zed/community bridges under the new `agentclientprotocol` organization."
  - "Status of the `claude-code-acp-rs` Rust community adapter: crate could not be looked up through the public crates.io API at research time, so version, last update, maintenance state are `unknown`. The skill catalog at `.claude/skills/acp/claude.md` (the ACP topic summary) still records it as a viable alternative."
  - "Plan-mode (review-before-execute) support is incomplete in the TypeScript adapter because the Claude Agent SDK does not fully implement plan mode."
  - "MCP-over-ACP tooling is unstable across adapters — `promptCapabilities` / `McpServer` arrays only became standardized in schema 1.1.0."
  - "The TypeScript adapter's exact support for schema v1.1.0 fields such as `session/resume`, `session/list`, `session/close`, `session/delete`, `additionalDirectories`, and `AgentAuthCapabilities` is documented in the schema but not surfaced in the adapter changelog — empirical verification required before treating them as usable through this adapter."
changes:
  - "Confirmed Claude Code's `claude` binary (2.1.200) has no native ACP entry point; updated all launch-mode evidence with exact error strings and a Bun-bundle note."
  - "Updated adapter references: the official TypeScript adapter was renamed from `@zed-industries/claude-agent-acp` to `@agentclientprotocol/claude-agent-acp` as of v0.24.0 (April 2026); the repository moved to github.com/agentclientprotocol/claude-agent-acp. Old npm names still resolve but are tracked under the new namespace."
  - "Noted that the Rust SDK preset `AcpAgent::zed_claude_code()` is stale (still calls the deprecated `@zed-industries/claude-code-acp@latest` package, pinned to v0.16.2) and recommended `AcpAgent::from_str` as the workaround."
  - "Corrected schema version from 1.2.0 (incorrectly recorded in prior research) to 1.1.0 (current)."
  - "Reflected schema v1.1.0 additions: `authenticate`/`logout`, `session/close`/`delete`/`list`/`resume`, `$/cancel_request`, `AgentAuthCapabilities`, `SessionCapabilities`, `SessionConfigOption`, `additionalDirectories`. Marked `fs_*` and `terminal/*` reverse requests as `unsupported` because the adapter v0.18.0+ delegates tool dispatch to Claude's built-in tools."
  - "Recorded adapter feature launches since the prior research: v0.52 title-update, v0.53 ACP logout, v0.54 fast mode, v0.55 refusal-fallback consent dialog, v0.50 cancellation, v0.43 elicitation, v0.48 Bash image output & agent-selection dropdown, v0.36 session/delete & additionalDirectories, v0.34 Bedrock gateway auth."
  - "Cross-checked protocol versions: agent-client-protocol 1.0.1 depends on agent-client-protocol-schema =1.1.0 (not 1.2.0)."
  - "Marked the v0.18.0 'drop client fs/terminal reverse requests' change as a behavioral reversal that prior research did not capture — clients used to expect those reverse requests, but they no longer arrive with the current official adapter."
  - "Marked the v0.53.0 'tool_call before permission request' fix as a UI-ordering change that clients depending on a specific `session/request_permission` vs `session/update` ordering may need to handle both."
  - "Recorded ad-hoc errors from `claude --acp` and `claude acp` as direct evidence the binary has no ACP entry point."
requires_claudine_update: true
reason: "Claude Code ACP support is still adapter-based, but the official adapter and the Rust SDK have both moved on in ways that affect Claudine's wiring: the npm namespace moved to @agentclientprotocol, the Rust SDK preset still references the deprecated npm name, the adapter no longer uses filesystem or terminal reverse requests (so Claudine's permission and shell-audit layers need to plug into Claude Code's built-in tools via session/request_permission rather than via fs/terminal handlers), and the protocol gained authenticate/logout/session-modes capabilities that future Claudine launcher detection and capability negotiation will need to model."
---

## Overview

Claude Code is Anthropic's agentic coding assistant. As of Claude Code 2.1.200 (BUILD_TIME 2026-07-03, the version installed at research time) it does **not** implement the Agent Client Protocol natively in its main `claude` CLI binary. Direct probes return `error: unknown option '--acp'` and `unknown command "acp"`; Anthropic closed the upstream feature request [anthropics/claude-code#6686](https://github.com/anthropics/claude-code/issues/6686) as `not planned` because community adapter implementations exist.

ACP support is therefore provided by an **adapter/bridge** process that translates between:

1. **ACP** — JSON-RPC 2.0 over stdio (the standard transport), spoken by editors and ACP clients. Schema v1.1.0.
2. **Claude Agent SDK protocol** — the proprietary NDJSON-over-stdio interface used by the `@anthropic-ai/claude-agent-sdk` runtime, which in turn launches the `claude` CLI binary.

The canonical adapter today is the TypeScript package `@agentclientprotocol/claude-agent-acp`, maintained in the [`agentclientprotocol/claude-agent-acp`](https://github.com/agentclientprotocol/claude-agent-acp) repository (formerly `zed-industries/claude-agent-acp`, formerly `@zed-industries/claude-code-acp`). It currently sits at **v0.55.0** (released 2026-07-02). A community Rust bridge `claude-code-acp-rs` is referenced in the rust-acp skill, but its crates.io listing could not be retrieved during this research, so its current status is `unknown`.

## Launching ACP

### Recommended: the official TypeScript adapter

```bash
npx -y @agentclientprotocol/claude-agent-acp@latest
```

Launching this binary opens a stdio JSON-RPC v1 stream using the schema at [`agentclientprotocol.com/protocol/schema`](https://agentclientprotocol.com/protocol/schema). The adapter launches the `claude` CLI via the Claude Agent SDK, translates incoming ACP requests to SDK calls, and forwards SDK stream events back as ACP `session/update` notifications. stderr is reserved for adapter/CLI logs to avoid corrupting the JSON-RPC stream.

### Legacy npm names that still work

```bash
npx -y @zed-industries/claude-agent-acp@latest   # alias through v0.24.0
npx -y @zed-industries/claude-code-acp@latest   # pinned to ≤ v0.16.2
```

The first alias was renamed in the same move that put the package under the new `agentclientprotocol` namespace; both names resolve. The second resolves to an adapter that is over a year stale (released before rename to `@zed-industries/claude-agent-acp`).

> **The Rust SDK preset `AcpAgent::zed_claude_code()` is broken.** At `agent-client-protocol 1.0.1` it still runs `npx -y @zed-industries/claude-code-acp@latest`, which pins to the 0.16.2 epoch. Use `AcpAgent::from_str("npx -y @agentclientprotocol/claude-agent-acp@latest")` to get a current adapter from Rust.

### No native launch mode

Direct verification on a freshly installed Claude Code 2.1.200:

```text
$ claude --acp
error: unknown option '--acp'

$ claude acp
✘ unknown command "acp"
  └ Did you mean claude mcp?

Run claude --help to list commands, or claude -p "acp" to send as a prompt.
```

The `claude` binary itself is a Bun-bundled JavaScript application (the Mach-O `__BUN` segment is present in the Mach-O loadable image). Anthropic has not announced and does not appear to be planning native ACP support.

## Protocol and Capabilities

### Transport and framing

- **Transport**: stdio pipes between the ACP client and the adapter. (`agent-client-protocol-http` is available in the Rust SDK for HTTP/WebSocket transports, but the official TypeScript adapter only speaks stdio.)
- **Framing**: newline-delimited JSON-RPC 2.0.
- **Encoding**: UTF-8.
- **Direction**: client sends requests/notifications to the agent; the agent sends responses, reverse requests, and protocol notifications to the client.

### Supported protocol version

Both the TypeScript adapter (v0.55.0) and the official Rust SDK (`agent-client-protocol 1.0.1` depending on `agent-client-protocol-schema =1.1.0`) negotiate **ACP v1 / schema 1.1.0**. The Rust crate exposes an opt-in `unstable_protocol_v2` feature, but neither adapter nor Claude's Agent SDK use it.

### Capability surface

| Area | Status | Notes |
|------|--------|-------|
| `initialize` / `authenticate` / `logout` | supported | Schema v1.1.0 trio. Adapter advertises `authMethods` for Claude login, Console login, and Bedrock; clients drive an `authenticate` round-trip instead of waiting for `auth_required`. |
| `session/new` / `session/load` / `session/prompt` / `session/cancel` | supported | Normal session lifecycle. |
| `session/resume` / `session/list` / `session/close` / `session/delete` | partial | Schema v1.1.0 supports these; adapter support is partial — `session/delete` is in v0.36+, the others are visible in `NewSessionResponse`/`LoadSessionResponse` payloads but not always presented as client-facing methods. |
| `session/set_mode` / `session/set_config_option` | supported | Schema v1.1.0 first-class. Adapter emits initial `modes` (effort_level/thought_level, model, permission_mode, fast) and changes via `current_mode_update`. |
| `session/request_permission` | supported | Reverse request for tool approvals; v0.53.0 fixed ordering so `tool_call` arrives before the request. |
| `fs/read_text_file` / `fs/write_text_file` / `terminal/*` | unsupported (by current adapter) | Adapter v0.18.0+ delegates reads/writes/execution to Claude's built-in tools and does NOT issue these reverse requests. Implement as a general client; do not expect traffic from this provider. |
| `session/update` streaming | supported | Text, thought, tool call/update, plan, mode/config/commands updates. |
| MCP (`mcpCapabilities.http` / `mcpCapabilities.sse`) | supported | Schema v1.1.0. Adapter v0.27 exposes raw SDK messages to clients, v0.43 experimental elicitation. MCP server management remains possible via `claude mcp` CLI outside ACP. |
| Plan mode | partial | Normal plan streaming works; pure review-before-execute plan mode errors because the Claude Agent SDK does not yet fully support it. |
| `ext_method` / `ext_notification` / `_meta` | supported | Adapter currently uses `_meta` for Claude-specific extras such as `acp.mcp_elicitation`, `additionalDirectories`, raw SDK passthrough. |
| Protocol-level `$/cancel_request` | supported | Schema v1.1.0. Adapter v0.50.0 added cancellation handling; v0.41.0 added force-cancellation on SDK query hang. |

## Reverse Requests

Because the canonical adapter (v0.18.0+) delegates filesystem reads, filesystem writes, and command execution to Claude's built-in tools, **only one reverse request is reliably observed in practice**: `session/request_permission`. The remaining entries below are kept for schema completeness and for clients that want to support other agents in the same code path.

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
      "title": "Edit src/auth.rs",
      "kind": "edit",
      "content": [...]
    },
    "options": [
      {"optionId": "allow", "name": "Allow", "kind": "allow_once"},
      {"optionId": "always", "name": "Always allow", "kind": "allow_always"},
      {"optionId": "deny", "name": "Deny", "kind": "reject_once"}
    ]
  }
}
```

Respond with `RequestPermissionOutcome::Selected` (chosen `optionId`) or `RequestPermissionOutcome::Cancelled`.

### Filesystem and terminal requests (schema-completeness only)

```json
{"jsonrpc":"2.0","id":43,"method":"fs/read_text_file","params":{"sessionId":"sess_abc123","path":"/project/src/main.rs","line":10,"limit":50,"sessionId":"sess_abc123"}}
```

```json
{"jsonrpc":"2.0","id":44,"method":"terminal/create","params":{"sessionId":"sess_abc123","command":"cargo","args":["build"],"cwd":"/project","env":[{"name":"RUST_LOG","value":"info"}],"outputByteLimit":1048576}}
```

Lifecycle: `terminal/create` → `terminal/output` / `terminal/wait_for_exit` → `terminal/kill` (optional) → `terminal/release`. Schemas for these end up in clients that wire up general ACP, but the current `claude-agent-acp` adapter will not initiate them.

## Permissions, Filesystem, and Terminal

### Permission policy

- The client is the authority for every tool call requiring approval. There is no implicit default — every `session/request_permission` must receive a `Selected` or `Cancelled` response.
- On `session/cancel`, the client MUST respond with `Cancelled` to any in-flight `session/request_permission` requests (schema v1.1.0 specification).
- The `kind` field of `PermissionOptionKind` accepts `allow_once`, `allow_always`, `reject_once`, and `reject_always` (Claude's setting system typically renders `allow_always` as a settings.json rule).

### Filesystem policy

With the current `claude-agent-acp` adapter, reads and writes happen *inside* the Claude Agent SDK process and never reach the client. Path policy, sandboxing, and redaction must be implemented through Claude Code's own permission and sandboxing systems (`settings.json`, `--permission-mode`, `sandbox.*` settings) rather than at the ACP boundary.

When `fs/read_text_file` and `fs/write_text_file` are implemented for general ACP support: paths must be absolute and 1-based line numbers; the client enforces sandboxing (typically by checking that the requested path lies inside the project root); reads and writes are the only filesystem reverse requests in ACP v1.

### Terminal policy

Same story as filesystem: the Claude Agent SDK runs commands through its built-in Bash tool. For general ACP support: the client receives the full command, arguments, environment variables, and working directory, then decides whether to allow the command (often via the same permission UI as `session/request_permission`) and is responsible for process lifecycle, output buffers, byte-limit truncation (truncating from the beginning when `outputByteLimit` is exceeded, at a character boundary), and the always-call `terminal/release` discipline.

## Streaming and UI Integration

Streaming flows through `session/update` notifications. Common update variants:

| Update | Purpose |
|--------|---------|
| `AgentMessageChunk` | Incremental assistant text. |
| `AgentThoughtChunk` | Internal reasoning / extended thinking. |
| `UserMessageChunk` | User message replay during session load. |
| `ToolCall` | A new tool call has started. |
| `ToolCallUpdate` | Tool progress, status change, or final result. |
| `Plan` | Multi-step plan entry (live streaming, not review-mode). |
| `AvailableCommandsUpdate` | Slash commands the agent advertises. |
| `CurrentModeUpdate` | Mode change. |
| `ConfigOptionUpdate` | Session config option change. |

Notifications are fire-and-forget — group by `ContentChunk.message_id` to disambiguate parallel streams. Route these events into whatever the host UI's event loop is; a desktop bridge usually means `tokio::sync::mpsc` between the ACP runtime and the UI framework thread.

`$/cancel_request` (schema v1.1.0) is the protocol-level cancellation signal — distinct from `session/cancel`, which only cancels the active prompt turn.

## Authentication and Setup

The TypeScript adapter inherits the `claude` CLI's auth posture and additionally advertises an `authMethods` array at initialize. The supported methods:

1. **Interactive login** — run `claude` once and complete OAuth in a browser, or let the adapter prompt via `authenticate`.
2. **`ANTHROPIC_API_KEY`** — set the env var to authenticate without a browser.
3. **Pre-existing session** — reuse cached Claude Code credentials in `~/.claude/`.
4. **Bedrock, Vertex, Foundry** — env vars (`CLAUDE_CODE_USE_BEDROCK=1`, etc.) and the dedicated Bedrock gateway authentication added to the adapter in v0.34.0.
5. **TUI login for remote environments** — added in adapter v0.26.0.
6. **`NO_BROWSER=1`** — removes the interactive browser flow from advertised methods.

For headless automation (CI, daemon contexts), use `ANTHROPIC_API_KEY`. The adapter does not add a new auth channel of its own — it lets the client pick one of the advertised methods and authenticate normally.

## Compatibility, Quirks, and Workarounds

1. **No native ACP mode** — every integration is an adapter bridge. Confirmed via `claude --acp` / `claude acp` on Claude Code 2.1.200.
2. **Rust preset pins an old adapter** — `AcpAgent::zed_claude_code()` from `agent-client-protocol 1.0.1` invokes the deprecated `@zed-industries/claude-code-acp@latest` (≤ v0.16.2), missing 31+ releases. Use `AcpAgent::from_str("npx -y @agentclientprotocol/claude-agent-acp@latest")` until upstream refreshes the preset.
3. **Dropped filesystem/terminal reverse requests** (adapter v0.18.0) — clients that adapted to the prior behavior must now rely on Claude Code's own permission UI; `fs/read_text_file` and `terminal/create` no longer arrive from the official adapter.
4. **Tool call ordering** — pre-v0.53.0, `session/request_permission` could arrive before the corresponding `session/update`. The ordering was fixed in v0.53.0; both orderings are seen depending on adapter version.
5. **Stdout pollution with AWS Bedrock** — `awsAuthRefresh` writes human-readable status messages to stdout and corrupts the NDJSON stream. Remove the setting or filter non-JSON lines.
6. **Initialization timeout** — the adapter can take longer than 30 seconds to initialize; use a 60-second timeout for `initialize` to absorb first-launch OAuth prompting.
7. **Default model fallback** — sessions may default to Haiku. Set a model explicitly via `ANTHROPIC_MODEL` or a `SessionConfigOption`.
8. **Plan-mode error in pure plan-mode** — the Claude Agent SDK does not fully implement review-plan-before-execute. Live plan streaming as the agent works is fine.
9. **Terminal handle leaks** — always call `terminal/release` when a `terminal/*` reverse request returns a `TerminalId`. This is also relevant because Claude's built-in Bash tool can fire from inside an MCP server that itself uses `terminal/create`.
10. **`session/cancel` semantics** — on receiving `session/cancel`, the client MUST respond `Cancelled` to any pending `session/request_permission` (schema v1.1.0). Forgetting to do so deadlocks the protocol.
11. **Path and indexing mistakes** — ACP requires absolute paths and 1-based line numbers. Relative paths and 0-based indexing are common integration bugs.

## Recent Changes

- **2026-07-02**: `@agentclientprotocol/claude-agent-acp` **v0.55.0** — refusal-fallback consent dialog, bumped Claude Agent SDK to 0.3.198, cancellation marker distinction (cancelled vs end_turn).
- **2026-06-30**: Adapter v0.54.0/v0.54.1 — fast-mode session config; modelOverrides correctly applied to availableModels allowlist.
- **2026-06-29**: Adapter v0.53.0 — first-class ACP `logout` request handling; SDK bump to Claude Agent SDK 0.3.195; `tool_call` now emitted before `session/request_permission`.
- **2026-06-29**: `agent-client-protocol-v1.0.1` + `agent-client-protocol-schema 1.1.0` — Rust SDK reaches 1.0; connection types are Send/Sync; preset constructors for Zed Claude, Zed Codex, Google Gemini; **schema is 1.1.0** (not 1.2.0 as the prior research recorded).
- **2026-06-25**: Adapter v0.52.0 — `--version` flag; session title updates pushed at turn end.
- **2026-06-23**: Adapter v0.50.0 — `$/cancel_request` propagation; v0.41.0 force-cancellation on SDK query hang.
- **2026-06-19**: Adapter v0.48.0 — agent-selection dropdown in config options; Bash tool image output surfaced; deduplicated streamed assistant blocks; update to new ACP SDK patterns.
- **2026-06-09**: Adapter v0.43.0 — experimental elicitation; ACP SDK update to 0.25.0.
- **2026-05-18**: Adapter v0.36.0 — experimental `session/delete`; `additionalDirectories` schema field for `session/new`.
- **2026-05-15**: Adapter v0.34.0 — Bedrock gateway authentication.
- **2026-04 (v0.24.0)**: **Repository moved from `zed-industries/claude-agent-acp` to `agentclientprotocol/claude-agent-acp`**; npm package renamed from `@zed-industries/claude-agent-acp` to `@agentclientprotocol/claude-agent-acp`.
- **2026-03-26 / v0.22.0**: Adapter v0.22.0 — stable `session/list` method via `ListSessionsRequest`; explicit `session/close`; meta param for testing additional directories.
- **Earlier 2026**: Bumped Claude Agent SDK continuously; v0.18.0 dropped replicated ACP filesystem/terminal tool wrappers in favor of Claude's built-in tools.

## Rust Client Example

This example uses `agent-client-protocol 1.0.1` with the *new* adapter namespace:

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
    // IMPORTANT: AcpAgent::zed_claude_code() points at the deprecated npm package.
    // Build it explicitly with the current namespace so the adapter is recent.
    let agent = AcpAgent::from_str("npx -y @agentclientprotocol/claude-agent-acp@latest")?;

    Client
        .builder()
        .name("claudine-claude-client")
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
            eprintln!("Agent: {:?}", init_response.agent_info);

            if !init_response.auth_methods.is_empty() {
                eprintln!("Agent offers auth: {:?}", init_response.auth_methods);
            }

            let session = connection
                .send_request(
                    NewSessionRequest::new(std::env::current_dir()?, vec![]),
                )
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

The adapter only reliably issues `session/request_permission` (per the v0.18.0+ delegation to built-in tools). The example handles permission and still implements `fs/read_text_file` / `fs/write_text_file` for general ACP completeness:

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
    // Pick the most permissive single-shot option.
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

The adapter delegates command execution to Claude's built-in Bash tool, so a full Claude client does not need to implement `terminal/*`. For general ACP support, the boilerplate is the same as any `agent-client-protocol` 1.0.1 client:

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

The remaining handlers follow the same pattern: look up the `TerminalId`, operate on the `Child`, and return the corresponding response. Always implement `terminal/release` and kill the process if it is still running — handle leaks are a frequent production foot-gun.

## Rust Desktop Streaming Bridge

To stream ACP events into a desktop UI, run the ACP client on a dedicated thread and forward `SessionNotification` values through an `mpsc` channel. Use the *current* adapter namespace:

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
            // Pin a known-good adapter version instead of `@latest` in production.
            let agent = AcpAgent::from_str(
                "npx -y @agentclientprotocol/claude-agent-acp@^0.55.0",
            )?;

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

Claudine currently wraps agentic CLIs through lifecycle hooks and event normalization, not through ACP. Adding ACP-based Claude Code support would require:

1. **Launch detection** — detect one of three npm package shapes (`@agentclientprotocol/claude-agent-acp` preferred; `@zed-industries/claude-agent-acp` legacy alias; never `@zed-industries/claude-code-acp`). The `claude-code-acp-rs` crate is a fourth candidate if its maintenance state can be verified.
2. **Capability negotiation** — because the official adapter no longer honors `fs.readTextFile` / `terminal: true` (v0.18.0+), Claudine's permission and shell-audit layers must plug into Claude Code's *own* permission UI via `session/request_permission`. Advertise `fs`/`terminal` capabilities anyway for forward-compatibility with adapters and proxy chains.
3. **Rust preset workaround** — until upstream updates `AcpAgent::zed_claude_code()`, Claudine's Rust launcher should call `AcpAgent::from_str("npx -y @agentclientprotocol/claude-agent-acp@latest")` (or pin a known version) so clients get a current adapter.
4. **Reverse-request routing** — only `session/request_permission` reliably fires; route it through Claudine's existing `permissions`/`protect` machinery. Optionally implement `fs/*` and `terminal/*` as general ACP clients for other providers sharing the same code path.
5. **Streaming bridge** — forward `session/update` notifications into Claudine's lifecycle pipeline so TTS, sound effects, logging, and messenger actions can react. Group updates by `ContentChunk.message_id`. Also handle the schema v1.1.0 protocol notifications (`current_mode_update`, `available_commands_update`, `config_option_update`) and tool-call ordering quirks (tool_call before permission in v0.53.0+).
6. **Terminal isolation** — when `terminal/create` does fire (older adapter or other agents), enforce Claudine's shell-audit, timeout, and deny-list rules before the command runs. Track handles and always call `release`.
7. **Headless auth** — require `ANTHROPIC_API_KEY` or pre-authenticated session before allowing non-interactive ACP launches; honor `authMethods` from the adapter and drive `authenticate`/`logout` rather than waiting on `auth_required`.
8. **Schema versioning** — verify on every launch whether the negotiated `ProtocolVersion` matches what Claudine's handlers expect (currently schema 1.1.0); preserve an `unstable_protocol_v2` feature flag for tracking upstream work.

Because Claude Code has no native ACP mode and the most-trusted adapter has just changed namespace and lost its filesystem/terminal reverse requests, Claudine should treat it as an **adapter-launched provider** with a higher integration cost than providers that ship ACP natively.

## Changelog

- **2026-07-03**: Refreshed for current Claude Code (2.1.200), `@agentclientprotocol/claude-agent-acp` (v0.55.0), and `agent-client-protocol` 1.0.1 / schema 1.1.0. Added direct-probe evidence that the `claude` binary has no `--acp` flag and no `acp` command. Corrected the prior schema version record (`1.2.0` → `1.1.0`). Documented the April-2026 GitHub org move and npm rename. Recorded the v0.18.0 delegation reversal that ended filesystem/terminal reverse requests through the official adapter. Flagged the `AcpAgent::zed_claude_code()` preset as stale and pointed to `AcpAgent::from_str` as the workaround.
- **2026-07-02**: Initial release of this research document (per the prior `claudine sequence` run).

## Sources

- [Claude Code documentation](https://code.claude.com/docs/en/overview)
- [`anthropics/claude-code` repository](https://github.com/anthropics/claude-code) (closed-source CLI; CHANGELOG is the public surface)
- [`anthropics/claude-code` CHANGELOG (raw)](https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md)
- [Issue #6686 — Feature Request: Add support for Agent Client Protocol (ACP), closed as `not planned`](https://github.com/anthropics/claude-code/issues/6686)
- [Agent Client Protocol specification](https://agentclientprotocol.com/)
- [ACP schema reference (v1.1.0)](https://agentclientprotocol.com/protocol/schema)
- [ACP Rust SDK (agentclientprotocol/rust-sdk) v1.0.1](https://github.com/agentclientprotocol/rust-sdk/releases/tag/v1.0.1)
- [`agent-client-protocol` crate on docs.rs (1.0.1)](https://docs.rs/agent-client-protocol/1.0.1/agent_client_protocol/) — `AcpAgent::zed_claude_code()` preset at `role/acp/acp_agent.rs`
- [`agent-client-protocol-schema` 1.1.0](https://docs.rs/agent-client-protocol-schema/1.1.0/agent_client_protocol_schema/)
- [`agentclientprotocol/claude-agent-acp` (the official adapter, formerly `zed-industries/claude-agent-acp`)](https://github.com/agentclientprotocol/claude-agent-acp)
- [`@agentclientprotocol/claude-agent-acp` on npm](https://www.npmjs.com/package/@agentclientprotocol/claude-agent-acp)
- [Adapter CHANGELOG (raw)](https://raw.githubusercontent.com/agentclientprotocol/claude-agent-acp/main/CHANGELOG.md)
- [Claude Agent SDK overview](https://platform.claude.com/docs/en/agent-sdk/overview)
- [Rust SDK Client Example (yolo_one_shot_client.rs)](https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/examples/yolo_one_shot_client.rs)
- [Adapter issue tracker — for known-quirks context](https://github.com/agentclientprotocol/claude-agent-acp/issues)
