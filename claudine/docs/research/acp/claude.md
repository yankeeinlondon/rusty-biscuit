---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: opencode
model: kimi-for-coding/k2p7
docs: https://code.claude.com/docs/en/overview
acp_docs: https://agentclientprotocol.com/
repo: https://github.com/anthropics/claude-code
support: adapter
launch_modes:
  - command: npx -y @zed-industries/claude-code-acp@latest
    args: []
    transport: stdio
    adapter: Zed Claude Code ACP (TypeScript)
    notes: Spawns the official Zed adapter as a stdio subprocess. The adapter launches the `claude` CLI internally and translates ACP JSON-RPC to the Claude Agent SDK protocol.
  - command: cargo install claude-code-acp-rs
    args: []
    transport: stdio
    adapter: claude-code-acp-rs (Rust)
    notes: Community Rust adapter that uses `sacp` and `agent-client-protocol-schema` internally. Install once, then run the binary as the ACP agent process.
  - command: claude
    args: []
    transport: other
    adapter: none
    notes: The main `claude` CLI has no native `--acp` or `acp serve` mode. Anthropic closed the ACP feature request as not planned because community adapters exist.
protocol_versions:
  - "v1"
capabilities:
  - capability: initialize
    support: supported
    notes: Standard ACP `initialize` handshake with capability negotiation and client info.
  - capability: authenticate
    support: partial
    notes: Auth is handled by the underlying `claude` CLI, not ACP directly. The adapter passes through whatever session/token the CLI has.
  - capability: session_new
    support: supported
    notes: "`session/new` creates a conversation session tied to a working directory."
  - capability: session_load
    support: supported
    notes: "`session/load` resumes an existing Claude Code session when the adapter and CLI support it."
  - capability: session_prompt
    support: supported
    notes: "`session/prompt` is the primary turn-taking method."
  - capability: session_cancel
    support: supported
    notes: "`session/cancel` is a notification that stops the current turn."
  - capability: streaming
    support: supported
    notes: "`session/update` notifications stream text, tool calls, plans, and mode changes."
  - capability: permissions
    support: supported
    notes: Reverse request `session/request_permission` is used for tool-call approval.
  - capability: fs_read
    support: supported
    notes: "`fs/read_text_file` reverse request when the client advertises filesystem read capability."
  - capability: fs_write
    support: supported
    notes: "`fs/write_text_file` reverse request when the client advertises filesystem write capability."
  - capability: terminal
    support: supported
    notes: Full `terminal/*` lifecycle (create, output, wait_for_exit, kill, release) is delegated to the client.
  - capability: mcp
    support: partial
    notes: Claude Code itself supports MCP servers; the ACP adapter may expose tools prefixed with `mcp__acp__`. MCP-over-ACP is unstable in the spec.
  - capability: plans
    support: partial
    notes: "`plan` session updates are emitted, but the Claude Agent SDK does not fully support plan mode and the TypeScript adapter errors out in plan mode."
  - capability: extensions
    support: supported
    notes: ACP `ext_method` / `ext_notification` can be used for adapter-specific extensions.
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: Required for any tool-call approval flow. The client must present options and return a selected option ID or cancellation.
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: true
    notes: Required if the client advertises filesystem read capability. The client reads the requested absolute path and returns content.
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: true
    notes: Required if the client advertises filesystem write capability. The client writes content to the requested absolute path.
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: true
    notes: Required if the client advertises terminal capability. Spawns a host command and returns a terminal ID.
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: true
    notes: Returns current stdout/stderr and exit status for a terminal handle without blocking.
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: true
    notes: Blocks until the terminal command exits and returns its final exit status.
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: true
    notes: Terminates the command but keeps the terminal handle valid for output retrieval.
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: true
    notes: Releases the terminal handle and kills the command if still running. Must be called to avoid resource leaks.
  - method: ext_method
    purpose: other
    client_must_handle: false
    notes: Optional extension requests specific to the adapter; clients can reject unsupported extension methods.
permission_model:
  mechanism: session/request_permission reverse request
  timeout: client-defined
  default_policy: no default; every tool call requiring approval must receive a Selected or Cancelled response
  approval_values:
    - allow_once
    - allow_always
    - reject_once
  notes: The client presents the options from the request and returns the selected option ID. If the prompt turn is cancelled, the client must respond with Cancelled.
filesystem_model:
  read_methods:
    - fs/read_text_file
  write_methods:
    - fs/write_text_file
  path_base: absolute paths only
  sandboxing: client-side; the client decides whether to enforce a project-root boundary
  notes: ACP requires absolute paths and 1-based line numbers. The client is responsible for sandboxing and validating paths before reading or writing.
terminal_model:
  supported: true
  methods:
    - terminal/create
    - terminal/output
    - terminal/wait_for_exit
    - terminal/kill
    - terminal/release
  shell: depends on host; Claude Code uses Bash on macOS/Linux and PowerShell on Windows when Git for Windows is absent
  cwd: absolute path supplied in CreateTerminalRequest
  streaming: polled via terminal/output
  cancellation: terminal/kill or terminal/release
  notes: The client must track terminal handles, reap processes, and always release handles. Output is byte-limited and truncated from the beginning when the limit is exceeded.
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
  notes: Updates are fire-and-forget notifications with no id. Use the ContentChunk.message_id field to group chunks belonging to the same message.
auth_setup:
  required: true
  mechanisms:
    - Claude Code login prompt on first use
    - ANTHROPIC_API_KEY for API-key auth
    - Pre-authenticated Claude Code session state
  headless_notes: For fully headless ACP operation, set ANTHROPIC_API_KEY or ensure the Claude Code CLI has already completed OAuth and stored its session. The adapter itself does not add new auth flows.
  notes: The adapter inherits authentication from the underlying `claude` CLI. In CI or daemon contexts, use an API key and avoid interactive OAuth.
env_vars:
  - name: ANTHROPIC_API_KEY
    effect: Allows the Claude CLI to authenticate without interactive login.
  - name: CLAUDE_CODE_DEBUG
    effect: Enables adapter/CLI debug logging; usually emitted to stderr to avoid corrupting the stdio JSON-RPC stream.
  - name: NO_BROWSER
    effect: Removes browser-based auth paths from advertised methods; useful in headless environments.
  - name: ANTHROPIC_BASE_URL
    effect: Optional proxy or alternative endpoint for Anthropic API calls.
  - name: awsAuthRefresh
    effect: When configured in Claude Code settings, can cause human-readable status messages to be written to stdout and corrupt the NDJSON stream.
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
  desktop_streaming_pattern: tokio::sync::mpsc from the notification handler to the UI thread; run the ACP client on a dedicated tokio runtime
  notes: As of agent-client-protocol 1.0.1 the SDK is Send/Sync and no longer requires tokio::task::LocalSet. Use AcpAgent::zed_claude_code() for the Zed adapter or AcpAgent::from_str for a custom adapter binary.
compatibility:
  - client: Zed
    status: works
    issue: none
    workaround: Use the built-in Claude Code via ACP support; adapter is fetched automatically.
  - client: JetBrains IDEs
    status: partial
    issue: Requires `~/.jetbrains/acp.json` configuration and a separately installed adapter.
    workaround: Point the registry to the `@zed-industries/claude-agent-acp` stdio command.
  - client: Neovim (CodeCompanion)
    status: works
    issue: none
    workaround: Configure the adapter as a stdio MCP/ACP agent.
  - client: agent-client-protocol Rust SDK 0.9.x
    status: partial
    issue: Connection futures were !Send and required LocalSet.
    workaround: Upgrade to agent-client-protocol 1.0.1 or later.
recent_changes:
  - date: 2026-06-29
    version: agent-client-protocol 1.0.1 / schema 1.2.0
    change: Official Rust SDK released with Send/Sync connections and a builder-based API.
    impact: Removes the LocalSet requirement; enables standard tokio::spawn and easier desktop-app integration.
  - date: 2026-06-29
    version: agent-client-protocol 1.0.1
    change: AcpAgent gained preset constructors for Zed Claude Code, Zed Codex, and Google Gemini.
    impact: Launching the Zed Claude adapter is now a single line in Rust.
quirks:
  - The `claude` CLI has no native ACP mode; every ACP integration requires an adapter bridge.
  - AWS Bedrock auth refresh can write human-readable status to stdout and corrupt the adapter's NDJSON stream. Remove `awsAuthRefresh` or filter non-JSON lines.
  - Initialization can take longer than 30 seconds; use a 60-second timeout for `initialize`.
  - Claude may call `Edit` instead of the adapter's `mcp__acp__Edit`, causing changes not to appear in the editor's diff view.
  - New sessions may default to Haiku instead of Sonnet; set the model explicitly in session config or environment.
  - Plan mode errors out in the TypeScript adapter because the Claude Agent SDK does not fully support it yet.
  - The TypeScript adapter sometimes needs a manual `sdk.mjs` path patch after reinstall.
  - Relative paths and 0-based indexing are common mistakes; ACP requires absolute paths and 1-based line numbers.
  - Terminal handle leaks occur if `terminal/release` is skipped.
gaps:
  - No official Anthropic-maintained ACP adapter; reliance on Zed/community bridges.
  - No documented headless auth flow specific to ACP; auth is inherited from the CLI's existing mechanisms.
  - Plan-mode behavior is incomplete.
  - MCP-over-ACP is unstable and adapter-specific.
changes:
  - Refreshed for agent-client-protocol 1.0.1 API.
  - Updated Rust examples to use the builder/ConnectionTo API instead of the 0.9.x MessageHandler API.
  - Removed the !Send/LocalSet guidance because 1.0.1 is Send/Sync.
  - Added `Claudine Integration Notes` and `Changelog` sections.
requires_claudine_update: true
reason: Claude Code ACP support is adapter-based and differs from native ACP providers. Claudine's future ACP client/adapter work needs dedicated launch-mode detection, reverse-request routing, permission policy integration, and terminal handle management for this provider.
---

## Overview

Claude Code is Anthropic's agentic coding assistant. As of July 2026 it does **not** implement the Agent Client Protocol (ACP) natively in its main `claude` CLI binary. Anthropic closed the [feature request (#6686)](https://github.com/anthropics/claude-code/issues/6686) as `not_planned` because community adapter implementations already exist.

ACP support for Claude Code is therefore provided by **adapter/bridge** processes that translate between:

1. **ACP** — JSON-RPC 2.0 over stdio, spoken by editors and ACP clients.
2. **Claude Agent SDK protocol** — the proprietary NDJSON-over-stdio protocol used by the `claude` CLI binary.

The best-known adapter is Zed's TypeScript package `@zed-industries/claude-agent-acp`. A Rust alternative, `claude-code-acp-rs`, is also available and uses `sacp` plus `agent-client-protocol-schema` internally.

For Claudine's future ACP client/adapter work this means Claude Code must be treated as an **adapter-launched provider**: the client spawns the adapter, negotiates ACP capabilities, and must be prepared to handle all agent-to-client reverse requests for permissions, filesystem access, and terminal execution.

## Launching ACP

### Zed TypeScript adapter

```bash
npx -y @zed-industries/claude-code-acp@latest
```

The adapter launches the `claude` CLI as a subprocess, translates ACP JSON-RPC requests to Claude Agent SDK calls, and converts SDK stream events back to ACP `session/update` notifications. All ACP traffic uses newline-delimited JSON-RPC 2.0 over stdio; stderr is reserved for adapter/CLI logs.

### Rust adapter

```bash
cargo install claude-code-acp-rs
claude-code-acp-rs
```

This is a community Rust implementation of the same bridge pattern.

### No native launch mode

The `claude` CLI itself does not accept `--acp`, `acp serve`, or similar flags. Any documentation or issue that implies otherwise refers to the adapter layer, not the primary binary.

## Protocol and Capabilities

### Transport and framing

- **Transport**: stdio pipes between the ACP client and the adapter.
- **Framing**: newline-delimited JSON-RPC 2.0.
- **Encoding**: UTF-8.
- **Direction**: client sends requests/notifications to the agent; agent sends responses and reverse requests back to the client.

### Supported protocol version

Both adapters target ACP **v1**. The official `agent-client-protocol` Rust crate also supports an opt-in `unstable_protocol_v2` feature, but Claude Code adapters currently negotiate v1.

### Capability surface

| Area | Status | Notes |
|------|--------|-------|
| `initialize` | supported | Standard handshake with `ClientCapabilities` and `Implementation`. |
| `session/new`, `session/load`, `session/cancel` | supported | Normal session lifecycle. |
| `session/prompt` | supported | Main turn-taking request with streaming response. |
| `session/request_permission` | supported | Reverse request for tool approvals. |
| `fs/read_text_file`, `fs/write_text_file` | supported | Only when client advertises filesystem capabilities. |
| `terminal/*` | supported | Only when client advertises terminal capability. |
| `session/update` streaming | supported | Text, tool, plan, and mode updates. |
| MCP | partial | Claude Code itself supports MCP; the adapter exposes tools prefixed `mcp__acp__`. |
| Plan mode | partial | `plan` updates stream, but the TypeScript adapter errors in plan mode. |
| `ext_method` / `ext_notification` | supported | Adapter-specific extensions. |

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
- The client decides whether to allow the command (often via the same permission UI that handles `session/request_permission`).
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

Before an ACP session can run headlessly, the underlying `claude` CLI must be authenticated. Options:

1. **Interactive login** — run `claude` once and complete OAuth in a browser.
2. **API key** — set `ANTHROPIC_API_KEY` so the CLI can authenticate without a browser.
3. **Pre-existing session** — reuse cached Claude Code credentials.

In CI or daemon contexts, prefer `ANTHROPIC_API_KEY`. The adapter does not provide its own authentication mechanism; it inherits whatever auth the CLI has.

## Compatibility, Quirks, and Workarounds

1. **No native ACP mode** — every integration requires an adapter. Do not expect a future `claude acp serve` command.
2. **Stdout pollution with AWS Bedrock** — if `awsAuthRefresh` is enabled, the CLI writes human-readable status to stdout and corrupts the NDJSON stream. Remove that setting or filter non-JSON lines.
3. **Initialization timeout** — the adapter can take longer than 30 seconds to initialize. Use a 60-second timeout or more.
4. **Missing `mcp__acp__Edit`** — Claude sometimes calls `Edit` instead of the adapter's prefixed tool, so changes may not appear in the editor's diff view. This is an adapter-level mapping issue.
5. **Default model fallback** — sessions may default to Haiku instead of Sonnet. Set the model explicitly in session configuration or environment variables.
6. **Plan mode errors** — the TypeScript adapter errors out in plan mode because the Claude Agent SDK does not fully support it.
7. **SDK pathing bug** — `@zed-industries/claude-agent-acp` may need a manual patch to `sdk.mjs` after reinstall (`entrypoints/cli.js` → `claude-code/cli.js`).
8. **Path and indexing mistakes** — ACP requires absolute paths and 1-based line numbers. Relative paths and 0-based indexing are common integration bugs.
9. **Terminal handle leaks** — always call `terminal/release` when a terminal is no longer needed.
10. **Historical `!Send` SDK** — `agent-client-protocol` 0.9.x required `tokio::task::LocalSet`. This was resolved in 1.0.1; modern code can use standard `tokio::spawn`.

## Recent Changes

- **2026-06-29**: `agent-client-protocol` 1.0.1 and `agent-client-protocol-schema` 1.2.0 shipped. The Rust SDK is now Send/Sync and uses a builder API.
- **2026-06-29**: `AcpAgent::zed_claude_code()` was added, making it trivial to launch the Zed Claude adapter from Rust.
- Earlier 2026: Zed adapter fixes for parallel command updates, relative patch paths, and other adapter-specific issues. See the `claude-agent-acp` issue tracker for details.

## Rust Client Example

This example uses `agent-client-protocol` 1.0.1 with the Zed Claude adapter.

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = AcpAgent::zed_claude_code();

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

            let init_response = connection
                .send_request(init)
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
            let agent = AcpAgent::zed_claude_code();

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

Claudine currently wraps agentic CLIs through lifecycle hooks and event normalization, not through ACP. Adding ACP-based Claude Code support would require:

1. **Adapter launch detection** — detect `npx -y @zed-industries/claude-code-acp@latest`, `claude-code-acp-rs`, or a user-configured adapter binary.
2. **Capability negotiation** — advertise filesystem and terminal capabilities only when Claudine's policy engine permits them for the current repo.
3. **Reverse-request routing** — implement handlers for `session/request_permission`, `fs/*`, and `terminal/*` and route them through Claudine's existing `permissions`/`protect` layers.
4. **Streaming bridge** — forward `session/update` notifications into Claudine's event pipeline so that TTS, sound effects, logging, and messenger actions can trigger.
5. **Terminal isolation** — ensure that commands spawned via `terminal/create` respect Claudine's shell-audit, timeout, and deny-list rules.
6. **Headless auth** — require `ANTHROPIC_API_KEY` or verified pre-authentication before allowing non-interactive ACP launches.

Because Claude Code has no native ACP mode, Claudine should treat it as an **adapter-launched provider** with a higher integration cost than providers that ship ACP natively.

## Changelog

- **2026-07-02**: Refreshed research for current ACP ecosystem and `agent-client-protocol` 1.0.1.
- **2026-07-02**: Replaced legacy 0.9.x Rust examples with builder/ConnectionTo API examples.
- **2026-07-02**: Added `Claudine Integration Notes` and populated all schema frontmatter fields.

## Sources

- [Claude Code Documentation](https://code.claude.com/docs/en/overview)
- [Claude Code ACP Feature Request (#6686)](https://github.com/anthropics/claude-code/issues/6686)
- [Agent Client Protocol Specification](https://agentclientprotocol.com/)
- [ACP GitHub Repository](https://github.com/agentclientprotocol/agent-client-protocol)
- [ACP Rust SDK (docs.rs)](https://docs.rs/agent-client-protocol/latest/agent_client_protocol/)
- [ACP Schema Crate (docs.rs)](https://docs.rs/agent-client-protocol-schema/latest/agent_client_protocol_schema/)
- [Rust SDK Client Example](https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/examples/yolo_one_shot_client.rs)
- [Zed Claude Code ACP Adapter](https://github.com/zed-industries/claude-agent-acp)
- [claude-code-acp-rs (Rust adapter)](https://crates.io/crates/claude-code-acp-rs)
- [Zed Blog: Claude Code via ACP](https://zed.dev/blog/claude-code-via-acp)
- [JetBrains ACP Docs](https://www.jetbrains.com/help/ai-assistant/acp.html)
- [Claude Agent SDK Overview](https://platform.claude.com/docs/en/agent-sdk/overview)
- [Zed adapter JSON parsing issue (#69)](https://github.com/zed-industries/claude-agent-acp/issues/69)
- [Zed initialization timeout issue (#43819)](https://github.com/zed-industries/zed/issues/43819)
- [Zed missing Edit tool issue (#49525)](https://github.com/zed-industries/zed/issues/49525)
- [Zed default model issue (#41578)](https://github.com/zed-industries/zed/issues/41578)
- [Zed plan mode issue (#172)](https://github.com/zed-industries/claude-agent-acp/issues/172)
