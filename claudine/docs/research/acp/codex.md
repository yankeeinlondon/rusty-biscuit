---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://developers.openai.com/codex/cli
acp_docs: https://agentclientprotocol.com/
repo: https://github.com/openai/codex
support: adapter
launch_modes:
  - command: npx -y @agentclientprotocol/codex-acp
    args: []
    transport: stdio
    adapter: "@agentclientprotocol/codex-acp (ACP org TypeScript adapter)"
    notes: >-
      Current recommended adapter. Built on the Codex App Server, it starts Codex
      internally and translates ACP JSON-RPC to Codex operations. Stderr is reserved
      for adapter/Codex logs; stdout carries newline-delimited JSON-RPC.
  - command: npx -y @zed-industries/codex-acp
    args: []
    transport: stdio
    adapter: Zed codex-acp (Rust)
    notes: >-
      Legacy Zed-maintained adapter. As of mid-2026 development has moved to the
      agentclientprotocol/codex-acp repo; use the ACP-org package for new installs.
  - command: codex
    args: []
    transport: other
    adapter: none
    notes: >-
      The main Codex CLI binary has no native `--acp` or `acp serve` mode. It exposes
      `codex app-server --listen stdio://` for app-server protocol traffic, but that
      is not ACP and is not the transport used by the ACP adapter.
protocol_versions:
  - "v1"
capabilities:
  - capability: initialize
    support: supported
    notes: Standard ACP `initialize` handshake with capability negotiation and client info.
  - capability: authenticate
    support: supported
    notes: >-
      Adapter advertises `chatgpt`, `codex-api-key`, and `openai-api-key` methods.
      Clients call `authenticate` when the adapter requires auth.
  - capability: session_new
    support: supported
    notes: "`session/new` creates a conversation session tied to a working directory."
  - capability: session_load
    support: supported
    notes: "`session/load` resumes an existing Codex session when the session id is known."
  - capability: session_prompt
    support: supported
    notes: "`session/prompt` is the primary turn-taking method."
  - capability: session_cancel
    support: supported
    notes: "`session/cancel` stops the current turn."
  - capability: session_modes
    support: supported
    notes: "`session/set_mode` switches between read-only, agent, and agent-full-access modes."
  - capability: streaming
    support: supported
    notes: "`session/update` notifications stream text, tool calls, plans, and mode changes."
  - capability: permissions
    support: supported
    notes: Reverse request `session/request_permission` is used for tool-call and command approval.
  - capability: fs_read
    support: supported
    notes: "`fs/read_text_file` reverse request when the client advertises filesystem read capability."
  - capability: fs_write
    support: supported
    notes: "`fs/write_text_file` reverse request when the client advertises filesystem write capability."
  - capability: terminal
    support: supported
    notes: >-
      Full `terminal/*` lifecycle (create, output, wait_for_exit, kill, release) is
      delegated to the client.
  - capability: mcp
    support: supported
    notes: >-
      Client-provided MCP servers over command-based stdio config and HTTP transport.
      Codex itself supports MCP servers and the adapter exposes them through ACP.
  - capability: media
    support: supported
    notes: >-
      Images can be attached to prompts. The adapter also reports image-generation and
      image-view events through `session/update`.
  - capability: plans
    support: supported
    notes: Plan events are emitted as `session/update` notifications with `plan` updates.
  - capability: extensions
    support: partial
    notes: >-
      ACP `_meta` fields and underscore-prefixed custom methods can be used for
      adapter-specific extensions, but Codex-specific extensions are not formally
      documented outside the adapter source.
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: >-
      Required for any tool-call approval flow. The client must present options and
      return a selected option ID or cancellation.
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: true
    notes: >-
      Required if the client advertises filesystem read capability. The client reads
      the requested absolute path and returns content.
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: true
    notes: >-
      Required if the client advertises filesystem write capability. The client writes
      content to the requested absolute path.
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: true
    notes: >-
      Required if the client advertises terminal capability. Spawns a host command and
      returns a terminal ID.
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: true
    notes: >-
      Returns current stdout/stderr and exit status for a terminal handle without
      blocking.
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: true
    notes: >-
      Blocks until the terminal command exits and returns its final exit status.
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: true
    notes: >-
      Terminates the command but keeps the terminal handle valid for output retrieval.
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: true
    notes: >-
      Releases the terminal handle and kills the command if still running. Must be
      called to avoid resource leaks.
permission_model:
  mechanism: session/request_permission reverse request
  timeout: client-defined
  default_policy: no default; every tool call requiring approval must receive a Selected or Cancelled response
  approval_values:
    - allow_once
    - allow_always
    - reject_once
    - reject_always
  notes: >-
    The client presents the options from the request and returns the selected option
    ID. If the prompt turn is cancelled, the client must respond with Cancelled. Codex
    CLI itself supports `--ask-for-approval never | on-request | untrusted` and
    sandbox modes, but over ACP these are negotiated per request.
filesystem_model:
  read_methods:
    - fs/read_text_file
  write_methods:
    - fs/write_text_file
  path_base: absolute paths only
  sandboxing: client-side; the client decides whether to enforce a project-root boundary
  notes: >-
    ACP requires absolute paths and 1-based line numbers. The client is responsible
    for sandboxing and validating paths before reading or writing.
terminal_model:
  supported: true
  methods:
    - terminal/create
    - terminal/output
    - terminal/wait_for_exit
    - terminal/kill
    - terminal/release
  shell: depends on host; Codex uses the system shell configured by the adapter/Codex runtime
  cwd: absolute path supplied in CreateTerminalRequest
  streaming: polled via terminal/output
  cancellation: terminal/kill or terminal/release
  notes: >-
    The client must track terminal handles, reap processes, and always release handles.
    Output is byte-limited and truncated from the beginning when the limit is exceeded.
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
    - session/update does not carry errors; JSON-RPC errors are returned on the request channel
  notes: >-
    Updates are fire-and-forget notifications with no id. Codex additionally emits
    shell command, file change, permission request, MCP tool call, terminal output,
    reasoning, image generation, image view, token usage, and review events.
auth_setup:
  required: true
  mechanisms:
    - ChatGPT OAuth login
    - Device code authentication (beta, for headless)
    - CODEX_API_KEY
    - OPENAI_API_KEY
    - Custom OpenAI-compatible gateway when opted in
  headless_notes: >-
    For fully headless ACP operation, set OPENAI_API_KEY or CODEX_API_KEY, or seed
    `~/.codex/auth.json` from a machine that already completed ChatGPT login. Set
    NO_BROWSER=1 to hide browser-based auth from advertised methods.
  notes: >-
    The adapter inherits authentication from the underlying Codex runtime. In CI or
    daemon contexts, API keys are recommended.
env_vars:
  - name: OPENAI_API_KEY
    effect: Fallback API key used when the API-key auth method is selected.
  - name: CODEX_API_KEY
    effect: Preferred API key; takes precedence over OPENAI_API_KEY for API-key auth.
  - name: NO_BROWSER
    effect: Hides browser-based ChatGPT auth from advertised auth methods.
  - name: CODEX_PATH
    effect: Runs a specific Codex executable instead of the bundled package dependency.
  - name: CODEX_CONFIG
    effect: JSON object merged into the Codex session config.
  - name: MODEL_PROVIDER
    effect: Model provider passed to Codex for new sessions.
  - name: DEFAULT_AUTH_REQUEST
    effect: ACP auth request JSON used when Codex requires authentication.
  - name: INITIAL_AGENT_MODE
    effect: "Initial mode id: read-only, agent, or agent-full-access."
  - name: APP_SERVER_LOGS
    effect: Directory for adapter logs.
  - name: CODEX_ACCESS_TOKEN
    effect: Provides a ChatGPT or Codex access token for trusted automation.
  - name: CODEX_HOME
    effect: Root for Codex state, including config, auth, logs, and sessions.
rust_client:
  crate: agent-client-protocol
  connection_type: AcpAgent subprocess over stdio (JSON-RPC)
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
  desktop_streaming_pattern: >-
    tokio::sync::mpsc from the notification handler to the UI thread; run the ACP
    client on a dedicated tokio runtime.
  notes: >-
    As of agent-client-protocol 1.0.1 the SDK is Send/Sync and no longer requires
    tokio::task::LocalSet. Use AcpAgent::from_str("npx -y @agentclientprotocol/codex-acp")
    to launch the current recommended adapter.
compatibility:
  - client: Zed
    status: works
    issue: none
    workaround: Use the built-in Codex via ACP support; the adapter is fetched automatically.
  - client: JetBrains IDEs
    status: partial
    issue: Requires `~/.jetbrains/acp.json` configuration and a separately installed adapter.
    workaround: Point the registry to the `@agentclientprotocol/codex-acp` stdio command.
  - client: Neovim (CodeCompanion)
    status: works
    issue: none
    workaround: Configure the adapter as a stdio ACP agent.
  - client: agent-client-protocol Rust SDK 0.9.x
    status: broken
    issue: Connection futures were !Send and required LocalSet; API has changed.
    workaround: Upgrade to agent-client-protocol 1.0.1 or later.
recent_changes:
  - date: 2026-07-02
    version: "@agentclientprotocol/codex-acp v1.1.0"
    change: >-
      Development moved from zed-industries/codex-acp to agentclientprotocol/codex-acp.
      The new adapter is built on the Codex App Server and is the recommended install.
    impact: >-
      New installs should use `npx -y @agentclientprotocol/codex-acp` instead of the
      Zed package.
  - date: 2026-06-29
    version: agent-client-protocol 1.0.1 / schema 1.1.0
    change: Official Rust SDK released with Send/Sync connections and a builder-based API.
    impact: >-
      Removes the LocalSet requirement; enables standard tokio::spawn and easier
      desktop-app integration.
  - date: 2026-06-08
    version: "@zed-industries/codex-acp v0.16.0"
    change: Final major Zed release before maintenance hand-off.
    impact: Existing Zed-adapter installs continue to work but will not receive new features.
quirks:
  - The Codex CLI has no native ACP mode; every ACP integration requires an adapter bridge.
  - The recommended adapter package moved from `@zed-industries/codex-acp` to `@agentclientprotocol/codex-acp`; old package references may stop getting updates.
  - `NO_BROWSER=1` removes ChatGPT OAuth from advertised auth methods, which can break flows that expect a browser sign-in.
  - API key auth is only recommended for headless/CI environments; ChatGPT subscription features are unavailable with API keys.
  - Initialization can take longer than 30 seconds; use a 60-second timeout for `initialize`.
  - Relative paths and 0-based indexing are common mistakes; ACP requires absolute paths and 1-based line numbers.
  - Terminal handle leaks occur if `terminal/release` is skipped.
  - The adapter is TypeScript/npm-based, so a Node.js runtime is required even when the client is written in Rust.
  - The bundled `@openai/codex` dependency in the npm package may lag behind the latest Codex CLI release; use `CODEX_PATH` to pin a newer binary.
  - Custom OpenAI-compatible gateways require client opt-in to the gateway auth capability.
gaps:
  - No official OpenAI-maintained ACP adapter; reliance on the ACP-org/Zed bridge.
  - No documented headless auth flow specific to ACP; auth is inherited from the CLI's existing mechanisms.
  - Codex-specific session config options and slash commands are not exhaustively documented as ACP surface area.
  - MCP-over-ACP behavior is supported but not formally standardized beyond the adapter implementation.
changes:
  - Refreshed for the new `@agentclientprotocol/codex-acp` adapter location.
  - Updated Rust examples to use agent-client-protocol 1.0.1 builder/ConnectionTo API.
  - Removed the !Send/LocalSet guidance because 1.0.1 is Send/Sync.
  - Added `Claudine Integration Notes` and `Changelog` sections.
  - Populated all schema frontmatter fields.
requires_claudine_update: true
reason: >-
  Codex CLI ACP support is adapter-based and differs from native ACP providers.
  Claudine's future ACP client/adapter work needs dedicated launch-mode detection,
  reverse-request routing, permission policy integration, and terminal handle
  management for this provider.
---

## Overview

Codex CLI is OpenAI's local coding agent. As of July 2026 it does **not** implement the Agent Client Protocol (ACP) natively in its main `codex` binary. Instead, ACP support is provided by an **adapter/bridge** process that translates between:

1. **ACP** — JSON-RPC 2.0 over stdio, spoken by editors and ACP clients.
2. **Codex runtime operations** — executed through the Codex App Server or the Codex CLI subprocess.

The recommended adapter is now maintained by the ACP project itself at [`agentclientprotocol/codex-acp`](https://github.com/agentclientprotocol/codex-acp) and distributed on npm as `@agentclientprotocol/codex-acp`. Zed's earlier Rust adapter, [`zed-industries/codex-acp`](https://github.com/zed-industries/codex-acp), is in maintenance mode and its README points new installs to the ACP-org package.

For Claudine's future ACP client/adapter work this means Codex CLI must be treated as an **adapter-launched provider**: the client spawns the adapter, negotiates ACP capabilities, and must be prepared to handle all agent-to-client reverse requests for permissions, filesystem access, and terminal execution.

## Launching ACP

### Current recommended adapter

```bash
npx -y @agentclientprotocol/codex-acp
```

The adapter starts the Codex App Server internally, translates ACP JSON-RPC requests into Codex operations, and maps Codex events back to ACP `session/update` notifications. All ACP traffic uses newline-delimited JSON-RPC 2.0 over stdio; stderr is reserved for adapter/Codex logs.

### Legacy Zed adapter

```bash
npx -y @zed-industries/codex-acp
```

This was the original Rust implementation. It still works but is no longer the recommended path for new installs.

### No native launch mode

The `codex` CLI itself does not accept `--acp`, `acp serve`, or similar flags. The `codex app-server --listen stdio://` command exposes the Codex App Server protocol, but that is a separate JSONL protocol, not ACP. ACP clients must use the adapter.

## Protocol and Capabilities

### Transport and framing

- **Transport**: stdio pipes between the ACP client and the adapter.
- **Framing**: newline-delimited JSON-RPC 2.0.
- **Encoding**: UTF-8.
- **Direction**: client sends requests/notifications to the agent; agent sends responses and reverse requests back to the client.

### Supported protocol version

The adapter targets ACP **v1**.

### Capability surface

| Area | Status | Notes |
|------|--------|-------|
| `initialize` | supported | Standard handshake with `ClientCapabilities` and `Implementation`. |
| `authenticate` | supported | Methods advertised include ChatGPT, `codex-api-key`, and `openai-api-key`. |
| `session/new`, `session/load`, `session/cancel` | supported | Normal session lifecycle; load resumes an existing session. |
| `session/prompt` | supported | Main turn-taking request with streaming response. |
| `session/set_mode` | supported | Switches between read-only, agent, and agent-full-access modes. |
| `session/request_permission` | supported | Reverse request for tool and command approvals. |
| `fs/read_text_file`, `fs/write_text_file` | supported | Only when client advertises filesystem capabilities. |
| `terminal/*` | supported | Only when client advertises terminal capability. |
| `session/update` streaming | supported | Text, tool, plan, and mode updates. |
| MCP | supported | Client-provided MCP servers over stdio config and HTTP transport. |
| Images | supported | Image inputs, image generation, and image view events. |
| Plan mode | supported | Plan events stream as `plan` updates. |
| Extensions | partial | `_meta` fields and underscore-prefixed custom methods available. |

## Reverse Requests

Because the agent process has no direct filesystem or terminal access, it sends reverse requests to the client. The following reverse requests must be handled by a Claudine ACP client when the corresponding capability is advertised.

### Permission requests

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "session/request_permission",
  "params": {
    "sessionId": "sess_abc123",
    "toolCall": {
      "toolCallId": "call_xyz",
      "title": "Write to /home/user/project/config.json",
      "kind": "edit"
    },
    "options": [
      {"optionId": "allow", "name": "Allow", "kind": "allow_once"},
      {"optionId": "always", "name": "Always Allow", "kind": "allow_always"},
      {"optionId": "deny", "name": "Deny", "kind": "reject_once"}
    ]
  }
}
```

The client must respond with `RequestPermissionOutcome::Selected` containing the chosen `option_id`, or `RequestPermissionOutcome::Cancelled`.

### Filesystem requests

Only sent when the client advertises filesystem capabilities:

```json
{"jsonrpc":"2.0","id":43,"method":"fs/read_text_file","params":{"sessionId":"sess_abc123","path":"/project/src/main.rs","line":10,"limit":50}}
```

```json
{"jsonrpc":"2.0","id":44,"method":"fs/write_text_file","params":{"sessionId":"sess_abc123","path":"/project/config.json","content":"new content..."}}
```

### Terminal requests

Only sent when the client advertises terminal capability:

```json
{"jsonrpc":"2.0","id":45,"method":"terminal/create","params":{"sessionId":"sess_abc123","command":"cargo","args":["build"],"cwd":"/project"}}
```

Lifecycle: `terminal/create` → `terminal/output` / `terminal/wait_for_exit` → `terminal/kill` (optional) → `terminal/release`.

## Permissions, Filesystem, and Terminal

### Permission policy

- The client is the authority for every tool call.
- There is no implicit default policy; the client must respond to each `session/request_permission`.
- If the user cancels the current turn, the client must still answer pending permission requests with `Cancelled`.

### Filesystem policy

- ACP paths must be absolute and line numbers are 1-based.
- The client enforces its own sandbox, typically by verifying the requested path is within the project root.
- Read and write are the only filesystem reverse requests in ACP v1.

### Terminal policy

- The client receives the full command, arguments, environment variables, and working directory.
- The client decides whether to allow the command, often via the same permission UI that handles `session/request_permission`.
- The client is responsible for process lifecycle, output buffers, truncation, and handle cleanup.

## Streaming and UI Integration

Streaming happens through `session/update` notifications. Common update types include:

| Update | Purpose |
|--------|---------|
| `AgentMessageChunk` | Incremental assistant text. |
| `AgentThoughtChunk` | Internal reasoning / extended thinking. |
| `UserMessageChunk` | User message replay during session load. |
| `ToolCall` | A new tool call has started. |
| `ToolCallUpdate` | Tool progress, status change, or final result. |
| `Plan` | Multi-step execution plan. |
| `AvailableCommandsUpdate` | Slash commands available in the session. |
| `CurrentModeUpdate` | Session mode change. |
| `ConfigOptionUpdate` | Session config option change. |

Because these are notifications, the client must route them into its UI event loop. A Rust desktop app typically uses `tokio::sync::mpsc` to forward updates from the ACP runtime thread to the UI framework (Tauri, iced, etc.).

## Authentication and Setup

Before an ACP session can run headlessly, the underlying Codex runtime must be authenticated. Options:

1. **Interactive login** — run `codex login` and complete OAuth in a browser, or use device code auth with `codex login --device-auth`.
2. **API key** — set `CODEX_API_KEY` or `OPENAI_API_KEY` so the runtime can authenticate without a browser.
3. **Access token** — pipe `CODEX_ACCESS_TOKEN` to `codex login --with-access-token` for trusted automation.
4. **Pre-existing session** — copy `~/.codex/auth.json` from a machine that already completed login.

In CI or daemon contexts, prefer API keys and set `NO_BROWSER=1` to hide browser-based auth. The adapter does not provide its own authentication mechanism; it inherits whatever auth the Codex runtime has.

## Compatibility, Quirks, and Workarounds

1. **No native ACP mode** — every integration requires an adapter. Do not expect a future `codex acp serve` command.
2. **Adapter migration** — the recommended package moved from `@zed-industries/codex-acp` to `@agentclientprotocol/codex-acp`. Update launch commands and registry configs.
3. **Node.js dependency** — even Rust clients must spawn a Node.js/npm process to run the adapter.
4. **API key vs ChatGPT feature gap** — API key auth is headless-friendly but lacks ChatGPT workspace features; ChatGPT auth requires a browser or device code flow.
5. **Initialization timeout** — the adapter can take longer than 30 seconds to initialize. Use a 60-second timeout or more.
6. **Path and indexing mistakes** — ACP requires absolute paths and 1-based line numbers. Relative paths and 0-based indexing are common integration bugs.
7. **Terminal handle leaks** — always call `terminal/release` when a terminal is no longer needed.
8. **Bundled Codex lag** — the npm package bundles a `@openai/codex` dependency that may lag behind the latest CLI release. Use `CODEX_PATH` to override.
9. **Historical `!Send` SDK** — `agent-client-protocol` 0.9.x required `tokio::task::LocalSet`. This was resolved in 1.0.1; modern code can use standard `tokio::spawn`.
10. **NO_BROWSER side effect** — setting this hides ChatGPT auth from the adapter's advertised methods, which can confuse clients that try to use it.

## Recent Changes

- **2026-07-02**: `@agentclientprotocol/codex-acp` v1.1.0 shipped as the new official adapter location, built on the Codex App Server.
- **2026-06-29**: `agent-client-protocol` 1.0.1 and `agent-client-protocol-schema` 1.1.0 shipped. The Rust SDK is now Send/Sync and uses a builder API.
- **2026-06-08**: `@zed-industries/codex-acp` v0.16.0 was the final major Zed release before maintenance hand-off to the ACP org.

## Rust Client Example

This example uses `agent-client-protocol` 1.0.1 with the current recommended adapter.

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
        ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest,
        SessionNotification, TextContent,
    },
};
use agent_client_protocol::{AcpAgent, Client};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let agent = AcpAgent::from_str("npx -y @agentclientprotocol/codex-acp")?;

    agent_client_protocol::Client
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
            let init_response = connection
                .send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;
            eprintln!("Agent: {:?}", init_response.agent_info);

            let session = connection
                .send_request(NewSessionRequest::new(std::env::current_dir()?))
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

The client can handle permission, filesystem, and terminal reverse requests with `on_receive_request`. The example below auto-approves reads but prompts for everything else.

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
    tokio::fs::write(&path, &request.content).await?;
    Ok(WriteTextFileResponse {})
}
```

Register the handlers on the builder before `connect_with`:

```rust
let project_root = std::env::current_dir()?;

Client
    .builder()
    .on_receive_request(
        |request: RequestPermissionRequest, responder, _cx| async move {
            responder.respond(handle_permission(request).await?)
        },
        agent_client_protocol::on_receive_request!(),
    )
    .on_receive_request(
        move |request: ReadTextFileRequest, responder, _cx| {
            let root = project_root.clone();
            async move { responder.respond(handle_read(request, root).await?) }
        },
        agent_client_protocol::on_receive_request!(),
    )
    .on_receive_request(
        move |request: WriteTextFileRequest, responder, _cx| {
            let root = project_root.clone();
            async move { responder.respond(handle_write(request, root).await?) }
        },
        agent_client_protocol::on_receive_request!(),
    )
    // ... notification handler and connect_with
```

## Rust Host Command Handling

A full terminal handler tracks spawned processes in a map and responds to each terminal reverse request.

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
        format!("term_{}", id).into()
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

The remaining handlers follow the same pattern: look up the `TerminalId`, operate on the `Child`, and return the corresponding response. Always implement `terminal/release` and kill the process if it is still running.

## Rust Desktop Streaming Bridge

To stream ACP events into a desktop UI, run the ACP client on a dedicated thread and forward `SessionNotification` values through an `mpsc` channel.

```rust
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextChunk(String),
    ThoughtChunk(String),
    ToolCallStarted { id: String, title: String },
    ToolCallFinished { id: String, status: String },
    PermissionRequest { request_id: String, title: String, options: Vec<(String, String)> },
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
            let agent = AcpAgent::from_str("npx -y @agentclientprotocol/codex-acp")
                .expect("spawn adapter");

            agent_client_protocol::Client
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
                        .send_request(NewSessionRequest::new(project_dir))
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

Claudine currently wraps agentic CLIs through lifecycle hooks and event normalization, not through ACP. Adding ACP-based Codex CLI support would require:

1. **Adapter launch detection** — detect `npx -y @agentclientprotocol/codex-acp`, legacy `@zed-industries/codex-acp`, or a user-configured adapter binary.
2. **Capability negotiation** — advertise filesystem and terminal capabilities only when Claudine's policy engine permits them for the current repo.
3. **Reverse-request routing** — implement handlers for `session/request_permission`, `fs/*`, and `terminal/*` and route them through Claudine's existing `permissions`/`protect` layers.
4. **Streaming bridge** — forward `session/update` notifications into Claudine's event pipeline so that TTS, sound effects, logging, and messenger actions can trigger.
5. **Terminal isolation** — ensure that commands spawned via `terminal/create` respect Claudine's shell-audit, timeout, and deny-list rules.
6. **Headless auth** — require `CODEX_API_KEY`/`OPENAI_API_KEY` or verified pre-authentication before allowing non-interactive ACP launches.

Because Codex CLI has no native ACP mode, Claudine should treat it as an **adapter-launched provider** with a higher integration cost than providers that ship ACP natively.

## Changelog

- **2026-07-02**: Refreshed research for current ACP ecosystem and the new `@agentclientprotocol/codex-acp` adapter.
- **2026-07-02**: Replaced legacy 0.9.x Rust examples with builder/ConnectionTo API examples using agent-client-protocol 1.0.1.
- **2026-07-02**: Added `Claudine Integration Notes` and populated all schema frontmatter fields.
- **2026-07-02**: Documented adapter migration from Zed to the ACP org.

## Sources

- [Codex CLI Documentation](https://developers.openai.com/codex/cli)
- [Codex CLI Command Reference](https://developers.openai.com/codex/cli/reference)
- [Codex Authentication Docs](https://developers.openai.com/codex/auth)
- [Codex Environment Variables](https://developers.openai.com/codex/environment-variables)
- [Codex Non-interactive Mode](https://developers.openai.com/codex/noninteractive)
- [Agent Client Protocol Specification](https://agentclientprotocol.com/)
- [ACP Agents Overview](https://agentclientprotocol.com/overview/agents)
- [ACP Protocol v1 Overview](https://agentclientprotocol.com/protocol/v1/overview.md)
- [ACP Tool Calls](https://agentclientprotocol.com/protocol/v1/tool-calls.md)
- [ACP File System](https://agentclientprotocol.com/protocol/v1/file-system.md)
- [ACP Terminals](https://agentclientprotocol.com/protocol/v1/terminals.md)
- [ACP Rust SDK (docs.rs)](https://docs.rs/agent-client-protocol/latest/agent_client_protocol/)
- [ACP Rust SDK Repository](https://github.com/agentclientprotocol/rust-sdk)
- [Rust SDK yolo_one_shot_client example](https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/examples/yolo_one_shot_client.rs)
- [Official ACP Codex Adapter](https://github.com/agentclientprotocol/codex-acp)
- [Zed Codex ACP Adapter (legacy)](https://github.com/zed-industries/codex-acp)
