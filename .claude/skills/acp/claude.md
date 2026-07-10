---
prompt: |-
    Do a deep dive on the ACP implementation that Claude Code CLI provides.

    - make sure to mention any quirks or gotchas that developers mention facing when interacting with Claude Code CLI as well any workarounds or ways to avoid issues

    After completing the deep dive, provide the following additional sections which cover writing code examples with Rust:

    1. Show how a Rust client can interact programmatically with the Agent using ACP
    2. Show explicitly how to handle "Reverse Requests" where the Agent asks the client to fulfill a tool request, a file read, etc. (as an Agent is not allowed to do this directly when operating via ACP)
    3. Show how a Rust client can respond to requests to execute commands on the host system
    4. Show how the Rust client we've created can use things like `mpsc` channels to send the Agent's streaming text to a desktop desktop app framework like Tauri or iced

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-07-02
update_policy:
    - Duration(6 mo)
---

# Claude Code CLI and ACP: Deep Dive

## How Claude Code Relates to ACP

Claude Code CLI does **not** natively implement the Agent Client Protocol (ACP). Anthropic closed the [feature request (issue #6686)](https://github.com/anthropics/claude-code/issues/6686) as NOT_PLANNED because community adapter implementations already exist. Instead, Claude Code is exposed as an ACP agent through **bridge/adapter** layers that translate between:

1. **ACP** (JSON-RPC 2.0 over stdio) — the standardized protocol that editors speak
2. **Claude Agent SDK protocol** (NDJSON over subprocess stdio) — the proprietary protocol that the `claude` CLI binary uses

### The Two-Layer Architecture

```
Editor / Your Rust Client (ACP Client)
    │
    │  ACP (JSON-RPC 2.0, newline-delimited, over stdio)
    │
ACP Agent Adapter (bridge process)
    │
    │  Claude Agent SDK (NDJSON over subprocess stdio)
    │
claude CLI binary
    │
    │  HTTPS (Anthropic Messages API)
    │
Claude API
```

### Available Adapter Implementations

| Adapter | Language | Install | Notes |
|---------|----------|---------|-------|
| `@zed-industries/claude-code-acp` | TypeScript | `npx -y @zed-industries/claude-code-acp@latest` | Official Zed adapter, most mature |
| `claude-code-acp-rs` | Rust | `cargo install claude-code-acp-rs` | Uses `sacp` + `agent-client-protocol-schema` internally |

### What the Adapter Does

The adapter process (e.g., `claude-agent-acp`) acts as a protocol translator:

1. Reads ACP JSON-RPC requests from **stdin**
2. Converts them to Claude Agent SDK calls (spawns the `claude` CLI as a subprocess)
3. Converts Claude SDK stream events back to ACP `session/update` notifications
4. Writes ACP JSON-RPC responses/notifications to **stdout**
5. All logging goes to **stderr** (to avoid corrupting the protocol stream)

Internally, the TypeScript adapter creates a built-in MCP server (`createMcpServer()`) that exposes tools prefixed with `mcp__acp__` (e.g., `mcp__acp__Edit`, `mcp__acp__Read`, `mcp__acp__Bash`). When Claude requests a tool use, the adapter extracts metadata, checks permissions, executes or delegates to the client via reverse requests, and converts results back to ACP format.

## ACP Protocol Essentials (for Claude Code)

### Transport

- **Newline-delimited JSON-RPC 2.0** over stdio
- One JSON message per line, delimited by `\n`
- No embedded newlines within messages
- UTF-8 encoding required
- Only valid ACP messages on stdout; stderr for logging

### Message Types

**Requests** (have `id`, expect a response):

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}
```

**Responses** (match a request's `id`):

```json
{"jsonrpc":"2.0","id":1,"result":{...}}
```

**Notifications** (no `id`, fire-and-forget):

```json
{"jsonrpc":"2.0","method":"session/update","params":{...}}
```

### Session Lifecycle

```
Client → Agent: initialize        (negotiate protocol version + capabilities)
Client → Agent: authenticate      (optional, if agent requires auth)
Client → Agent: session/new       (create session, receive sessionId)
Client → Agent: session/prompt    (send user message)
Agent → Client: session/update    (streaming: text chunks, tool calls, plans)
Agent → Client: session/request_permission  (reverse request for tool approval)
Client → Agent: response          (approve/deny the permission)
Agent → Client: session/prompt response     (final result with stopReason)
```

### Streaming Updates During a Prompt Turn

While processing a `session/prompt`, the agent sends `session/update` notifications:

| Update Kind | Description |
|-------------|-------------|
| `agent_message_chunk` | Incremental text response from the model |
| `agent_thought_chunk` | Internal reasoning (extended thinking) |
| `user_message_chunk` | User message replay (during session load) |
| `tool_call` | Tool invocation with status `pending` |
| `tool_call_update` | Tool progress/completion with status changes and content |
| `plan` | Multi-step execution plan with entries and priorities |
| `available_commands_update` | Slash commands available in the session |
| `current_mode_update` | Agent mode change notification |

### Stop Reasons

- `EndTurn` — LLM finished naturally
- `MaxTokens` — token limit reached
- `MaxTurnRequests` — too many LLM requests in one turn
- `Refusal` — agent refuses to continue
- `Cancelled` — client sent `session/cancel`

## Reverse Requests (Agent → Client)

ACP is **bidirectional**. While the client normally sends requests to the agent, the agent can also send requests **back** to the client. These reverse requests are central to ACP because the agent process itself has no direct filesystem or terminal access — it must ask the client to perform these operations.

### Permission Requests

When the agent wants to execute a tool that requires approval:

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

### File System Operations (capability-gated)

Only sent if the client declared `fs` capabilities during initialization:

```json
{"jsonrpc":"2.0","id":43,"method":"fs/read_text_file","params":{"sessionId":"sess_abc123","path":"/project/src/main.rs","line":10,"limit":50}}
```

```json
{"jsonrpc":"2.0","id":44,"method":"fs/write_text_file","params":{"sessionId":"sess_abc123","path":"/project/config.json","content":"new content..."}}
```

### Terminal Operations (capability-gated)

Only sent if the client declared `terminal` capability:

```json
{"jsonrpc":"2.0","id":45,"method":"terminal/create","params":{"sessionId":"sess_abc123","command":"cargo","args":["build"],"cwd":"/project"}}
```

Terminal lifecycle: `terminal/create` → `terminal/output` / `terminal/wait_for_exit` → `terminal/kill` → `terminal/release`

## Quirks, Gotchas, and Known Issues

### Protocol-Level Gotchas

**1. Calling optional methods without capability checks**

Always check `initialize` response capabilities before calling `fs/*`, `terminal/*`, or `session/load`. Build a capability matrix once during initialization and branch all method calls from it.

**2. Relative paths**

ACP requires **absolute paths** and **1-based line numbers**. Many integrations accidentally use project-relative paths or 0-based indexing. Normalize all paths before sending.

**3. Notifications vs. requests confusion**

`session/update` and `session/cancel` are **notifications** (no `id`). Do not wait for responses to them. Only methods with an `id` field get responses.

**4. Cancellation handling**

On cancel: stop model/tool work, resolve pending permission requests as cancelled, return `stopReason: "cancelled"` (not an error response). Agents may still send a few `session/update` notifications after cancel before the final response.

**5. Terminal handle leaks**

Always follow `terminal/create` → `terminal/wait_for_exit`/`terminal/output` → `terminal/release`. Forgetting `terminal/release` leaks resources in the agent process.

**6. Message boundary ambiguity**

Consecutive `agent_message_chunk` updates lack stable message identifiers. Use draft `messageId` patterns where available, and otherwise segment conservatively around update-type/state transitions.

### Claude Code ACP-Specific Issues

**7. JSON parsing errors from stdout pollution** ([Issue #69](https://github.com/zed-industries/claude-agent-acp/issues/69))

When AWS Bedrock auth refresh is configured, the Claude Code CLI writes human-readable status messages ("Attempting...") to stdout, corrupting the NDJSON protocol stream. **Workaround**: Remove `awsAuthRefresh` config and authenticate manually, or filter non-JSON lines from stdout before parsing.

**8. Initialization timeout** ([Issue #43819](https://github.com/zed-industries/zed/issues/43819))

Agents fail to initialize within Zed's hardcoded 30-second timeout. No clear fix; affects multiple platforms. If building your own client, use a generous timeout (60s+) for the `initialize` request.

**9. Missing `mcp__acp__Edit` tool** ([Issue #49525](https://github.com/zed-industries/zed/issues/49525))

Claude sometimes uses `Edit` instead of `mcp__acp__Edit`, preventing changes from appearing in the editor's agent changes view. This is an adapter-level issue where the MCP tool mapping isn't always respected.

**10. Default model fallback** ([Issue #41578](https://github.com/zed-industries/zed/issues/41578))

Claude Code through ACP may default to Haiku instead of Sonnet for new sessions. **Workaround**: Explicitly set the model in your session configuration or environment variables.

**11. Plan mode errors** ([Issue #172](https://github.com/zed-industries/claude-agent-acp/issues/172))

The Claude Code ACP adapter errors out in plan mode since the Agent SDK doesn't fully support it yet.

**12. SDK pathing bug**

The `@zed-industries/claude-code-acp` package may need manual patching at `sdk.mjs` line 6515: change `join(dirname, "entrypoints", "cli.js")` to `join(dirname, "claude-code", "cli.js")`. This fix resets on reinstall.

**13. Non-exhaustive enums everywhere**

`ContentBlock`, `SessionUpdate`, `StopReason`, `ToolCallStatus`, and `ErrorCode` in `agent-client-protocol-schema` are all `#[non_exhaustive]`. Always include wildcard (`_`) arms in match expressions, or your code will break when new variants are added.

## Editor Support

| Editor | ACP Support | Agent Adapter |
|--------|-------------|---------------|
| **Zed** | Built-in | `claude-agent-acp`, `claude-code-acp-rs`, Gemini CLI, Codex CLI |
| **JetBrains** (all IDEs) | Built-in via AI Assistant | Via `~/.jetbrains/acp.json` config |
| **Neovim** | Via CodeCompanion plugin | Any ACP agent |
| **Kiro** (AWS) | Native CLI `kiro-cli acp` | Kiro Agent |

Debug ACP traffic in Zed: Command Palette → `dev: open acp logs`

---

## Rust Code Examples

The following examples show how to build a Rust ACP client using the official `agent-client-protocol` crate (v1.0.1). All examples use `tokio` as the async runtime.

As of 1.0.1 the SDK is `Send`/`Sync`, so standard `tokio::spawn` works and you no longer need `tokio::task::LocalSet`.

### Dependencies

```toml
[dependencies]
agent-client-protocol = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

### 1. Basic ACP Client: Connecting to the Agent

This example spawns the Zed Claude Code ACP adapter as a subprocess and performs the full initialization → session → prompt lifecycle.

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
async fn main() -> anyhow::Result<()> {
    let agent = AcpAgent::zed_claude_code();

    Client
        .builder()
        .name("rusty-claude-client")
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
                    name: "rusty-client".into(),
                    title: Some("Rusty ACP Client".into()),
                    version: "0.1.0".into(),
                });

            let init_response = connection
                .send_request(init)
                .block_task()
                .await?;
            println!("Connected to agent: {:?}", init_response.agent_info);

            let session = connection
                .send_request(NewSessionRequest::new(std::env::current_dir()?))
                .block_task()
                .await?;
            println!("Session created: {}", session.session_id);

            let result = connection
                .send_request(PromptRequest::new(
                    session.session_id,
                    vec![ContentBlock::Text(TextContent::new(
                        "What files are in this directory?".into(),
                    ))],
                ))
                .block_task()
                .await?;

            println!("\nDone. Stop reason: {:?}", result.stop_reason);
            Ok(())
        })
        .await?;

    Ok(())
}
```

### 2. Handling Reverse Requests: File System Operations

When the agent needs to read or write files, it sends reverse requests to the client. Register handlers with `on_receive_request` before `connect_with`.

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
            "Path {} is outside project root {}",
            canonical.display(),
            root.display()
        );
    }
    Ok(canonical)
}

async fn handle_permission(
    request: RequestPermissionRequest,
) -> anyhow::Result<RequestPermissionResponse> {
    // Auto-select the first option in this demo; in a real app show a UI prompt.
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

    // ACP uses 1-based line numbers.
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

Register the handlers on the builder:

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

### 3. Handling Terminal Execution Requests

When the agent needs to run commands on the host system, it uses the terminal reverse request lifecycle. Track spawned processes in a map and respond to each request.

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

The remaining handlers look up the `TerminalId`, operate on the `Child`, and return the corresponding response. Always implement `terminal/release` and kill the process if it is still running.

### 4. Streaming to a Desktop App via `mpsc` Channels

Run the ACP client on a dedicated thread and forward `SessionNotification` values through an `mpsc` channel. The 1.0.1 SDK is `Send`/`Sync`, so a multi-threaded Tokio runtime works.

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
                                    SessionNotification::AgentMessageChunk(chunk) => {
                                        match chunk.content {
                                            ContentBlock::Text(t) => Some(AgentEvent::TextChunk(t.text)),
                                            _ => None,
                                        }
                                    }
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

#### Usage from Tauri

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

#### Usage from iced

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

## Choosing Your Approach

| Approach | When to Use | Complexity |
|----------|------------|------------|
| **`agent-client-protocol` crate** | Full ACP client/agent with spec compliance | Medium — handles framing, routing, capabilities |
| **`sacp` crate** | Need proxy/middleware chains, MCP tool injection | Higher — more powerful but more ceremony |
| **`claude-codes` crate** | Drive Claude CLI directly without ACP | Lower — simpler protocol, but Claude-only |
| **`cc-sdk` crate** | Batteries-included Claude SDK with hooks and permissions | Lower — most opinionated but most complete |
| **Bespoke with `agent-client-protocol-schema`** | Custom transport, minimal deps, partial protocol | Varies — you own the I/O layer |

## Sources

- [Agent Client Protocol Specification](https://agentclientprotocol.com/)
- [ACP GitHub Repository](https://github.com/agentclientprotocol/agent-client-protocol)
- [ACP Rust SDK (docs.rs)](https://docs.rs/agent-client-protocol/latest/agent_client_protocol/)
- [ACP Schema Crate (docs.rs)](https://docs.rs/agent-client-protocol-schema/latest/agent_client_protocol_schema/)
- [SACP Crate (docs.rs)](https://docs.rs/sacp/latest/sacp/)
- [Rust SDK Client Example](https://github.com/agentclientprotocol/rust-sdk/blob/main/examples/client.rs)
- [Claude Code ACP Feature Request (Issue #6686)](https://github.com/anthropics/claude-code/issues/6686)
- [Zed Claude Code ACP Adapter](https://github.com/zed-industries/claude-agent-acp)
- [claude-code-acp-rs (Rust adapter)](https://crates.io/crates/claude-code-acp-rs)
- [Xuanwo/acp-claude-code](https://github.com/Xuanwo/acp-claude-code)
- [Zed Blog: Claude Code via ACP](https://zed.dev/blog/claude-code-via-acp)
- [JetBrains ACP Docs](https://www.jetbrains.com/help/ai-assistant/acp.html)
- [Claude Agent SDK Overview](https://platform.claude.com/docs/en/agent-sdk/overview)
- [JSON Parsing Error (Issue #69)](https://github.com/zed-industries/claude-agent-acp/issues/69)
- [Initialization Timeout (Issue #43819)](https://github.com/zed-industries/zed/issues/43819)
- [Missing Edit Tool (Issue #49525)](https://github.com/zed-industries/zed/issues/49525)
- [Default Model Issue (Issue #41578)](https://github.com/zed-industries/zed/issues/41578)
- [Plan Mode Errors (Issue #172)](https://github.com/zed-industries/claude-agent-acp/issues/172)
