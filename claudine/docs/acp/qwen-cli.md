---
prompt: |-
    Do a deep dive on the ACP implementation that Qwen CLI provides.

    - make sure to mention any quirks or gotchas that developers mention facing when interacting with Qwen CLI as well any workarounds or ways to avoid issues

    After completing the deep dive, provide the following additional sections which cover writing code examples with Rust:

    1. Show how a Rust client can interact programmatically with the Agent using ACP
    2. Show explicitly how to handle "Reverse Requests" where the Agent asks the client to fulfill a tool request, a file read, etc. (as an Agent is not allowed to do this directly when operating via ACP)
    3. Show how a Rust client can respond to requests to execute commands on the host system
    4. Show how the Rust client we've created can use things like `mpsc` channels to send the Agent's streaming text to a desktop desktop app framework like Tauri or iced

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
update_policy:
    - Duration(6 mo)
---

# Qwen CLI ACP Implementation Deep Dive

## Overview

Qwen Code (package: `@qwen-code/qwen-code`, binary: `qwen`) is an open-source AI coding agent from Alibaba's QwenLM team that operates as a terminal CLI. It implements the **Agent Client Protocol (ACP)** -- a JSON-RPC 2.0 based protocol that standardizes communication between code editors/IDEs and AI coding agents. ACP was originally championed by the Zed editor team and is now supported by multiple editors (Zed, JetBrains, Neovim, Marimo) and agents (Claude Code, Codex CLI, Gemini CLI, Goose, Qwen Code).

- **Repository**: <https://github.com/QwenLM/qwen-code>
- **ACP Specification**: <https://agentclientprotocol.com/>
- **ACP GitHub**: <https://github.com/agentclientprotocol/agent-client-protocol>
- **Rust SDK**: <https://docs.rs/agent-client-protocol> / <https://github.com/agentclientprotocol/rust-sdk>
- **Zed ACP Page**: <https://zed.dev/acp>

## Starting Qwen CLI in ACP Mode

### The `--acp` Flag

Qwen Code exposes ACP mode via the `--acp` CLI flag:

```bash
qwen --acp
```

The flag was previously `--experimental-acp` and was graduated to `--acp` in January 2026 (PR [#1355](https://github.com/QwenLM/qwen-code/pull/1355), issue [#1350](https://github.com/QwenLM/qwen-code/issues/1350)). The old flag is kept as a deprecated alias that emits a warning.

### Editor Configuration

**Zed** (`settings.json`):
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

**JetBrains** (AI Chat > Configure ACP Agent):
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

### Additional Flags

- `--approval-mode plan` -- restricts the agent to read-only tools (though this has bugs in ACP mode, see Quirks below)
- Authentication: `qwen auth` must be run separately before starting ACP mode. Supports Qwen OAuth and API keys.

## ACP Protocol Details

### Transport

ACP uses **JSON-RPC 2.0 over stdio**:

- The editor (client) spawns the agent as a subprocess
- The agent reads JSON-RPC messages from **stdin** and writes to **stdout**
- Messages are newline-delimited (`\n`) and **must not** contain embedded newlines
- All content is UTF-8 encoded
- The agent may write diagnostic logs to **stderr** (clients may capture or discard)
- HTTP and WebSocket transports are in draft/proposal phase and not yet standardized

### Protocol Version

Qwen Code currently implements **ACP v1** (`protocolVersion: 1`). JetBrains 2025.3+ requires ACP v2, creating an incompatibility (see Quirks).

### Message Categories

ACP defines three message categories:

1. **Requests** (Methods) -- require a response, include an `id` field
2. **Responses** -- include the request `id` and either `result` or `error`
3. **Notifications** -- one-way messages, no response expected

### Agent Methods (Client -> Agent)

These are methods the client sends to the agent:

| Method | Required | Description |
|--------|----------|-------------|
| `initialize` | Yes | Capability negotiation, protocol version, client/agent info |
| `authenticate` | Yes | Credential validation (OAuth, API key) |
| `session/new` | Yes | Create a new conversation session |
| `session/prompt` | Yes | Submit user input to the agent |
| `session/load` | No | Resume an existing session (replays history via `session/update`) |
| `session/set_mode` | No | Switch operating mode (ask, architect, code) |
| `session/set_model` | No | Change the model (added for JetBrains compat, PR #1521) |
| `session/cancel` | No | Interrupt processing (notification, no response) |

### Client Methods (Agent -> Client) -- "Reverse Requests"

These are **reverse requests** where the agent asks the client to perform operations on its behalf. This is the critical design difference from standalone CLI mode: in ACP mode, the agent cannot directly read files, write files, or execute commands. It must request these from the client.

| Method | Capability Gate | Description |
|--------|----------------|-------------|
| `session/request_permission` | Required | Ask user to authorize a tool execution |
| `fs/read_text_file` | `fs.readTextFile` | Read file contents (includes unsaved editor state) |
| `fs/write_text_file` | `fs.writeTextFile` | Write/create a file |
| `terminal/create` | `terminal` | Execute a shell command |
| `terminal/output` | `terminal` | Retrieve terminal output and exit status |
| `terminal/wait_for_exit` | `terminal` | Block until command completes |
| `terminal/kill` | `terminal` | Terminate a running process |
| `terminal/release` | `terminal` | Kill + release all resources |

### Notifications (Agent -> Client)

The agent sends `session/update` notifications to stream progress. The `sessionUpdate` field discriminates the update type:

| Update Type | Description |
|-------------|-------------|
| `agent_message_chunk` | Streamed text response from the model |
| `agent_thought_chunk` | Internal reasoning/chain-of-thought |
| `user_message_chunk` | Echo of user input |
| `tool_call` | Initial tool invocation report (with `toolCallId`, `title`, `kind`, `status`) |
| `tool_call_update` | Status changes during tool execution (pending -> in_progress -> completed/failed) |
| `plan` | Structured task breakdown with priorities |
| `current_mode_update` | Agent-initiated mode change |
| `current_model_update` | Model change notification |

### Tool Call Kinds

Tool calls include a `kind` field categorizing the action:

`read`, `edit`, `delete`, `move`, `search`, `execute`, `think`, `fetch`, `other`

### Permission Flow

Before executing sensitive operations, the agent sends `session/request_permission` with permission options:

- `allow_once` -- permit this single operation
- `allow_always` -- auto-approve similar operations
- `reject_once` -- deny this operation
- `reject_always` -- auto-deny similar operations
- `cancelled` -- the prompt turn was interrupted

### Content Block Types

ACP defines five content block types for prompts and responses:

1. **Text**: `{ "type": "text", "text": "..." }`
2. **Image**: `{ "type": "image", "mimeType": "image/png", "data": "<base64>" }` (requires `image` capability)
3. **Audio**: `{ "type": "audio", "mimeType": "audio/wav", "data": "<base64>" }` (requires `audio` capability)
4. **Embedded Resource**: `{ "type": "resource", "resource": { "uri": "file:///...", "text": "..." } }`
5. **Resource Link**: `{ "type": "resource_link", "uri": "file:///...", "name": "...", "mimeType": "..." }`

### Prompt Turn Lifecycle

1. Client sends `session/prompt` with `sessionId` and content blocks
2. Agent forwards to the LLM
3. Agent streams `session/update` notifications (text chunks, tool calls, plans)
4. If tools are needed: agent requests permission, executes tools, reports status
5. Tool results feed back to the LLM; cycle repeats
6. Agent responds with `stopReason`: `end_turn`, `max_tokens`, `max_turn_requests`, `refusal`, or `cancelled`

### Session Modes

Agents advertise available modes (e.g., "ask", "architect", "code") that control:
- System prompts used
- Available tools
- Whether permission is required before actions

Clients switch modes via `session/set_mode`. Agents can self-switch via `current_mode_update` notifications.

### Initialization Handshake

```
Client                          Agent
  |                               |
  |--- initialize (v1, caps) ---->|
  |<-- initialize response -------|
  |                               |
  |--- authenticate ------------->|
  |<-- auth response -------------|
  |                               |
  |--- session/new (cwd) -------->|
  |<-- sessionId -----------------|
  |                               |
  |--- session/prompt ----------->|
  |<-- session/update (stream) ---|
  |<-- session/update (stream) ---|
  |<-- prompt response ----------|
```

## Quirks and Gotchas

### 1. ACP v1 vs v2 Incompatibility with JetBrains

**Issue**: [#1502](https://github.com/QwenLM/qwen-code/issues/1502)

Qwen Code implements ACP v1, but JetBrains 2025.3+ requires ACP v2. The `prompt.turn` method used by JetBrains v2 is not supported, resulting in `"Prompt turn failed: Method not found"`. A partial fix was merged (PR #1521) adding `session/set_model`, but full v2 support is still incomplete.

**Workaround**: Use an older JetBrains IDE version or use Zed/Neovim which work with ACP v1.

### 2. Plan Mode Does Not Work Correctly in ACP Mode

**Issue**: [#1151](https://github.com/QwenLM/qwen-code/issues/1151)

When using `--approval-mode plan`, the agent still attempts to use edit tools and requests user permission, instead of restricting to read-only tools. The `exitPlanMode.ts` tool exists but the tool filtering logic is not applied correctly in ACP mode.

**Workaround**: None documented. Avoid relying on plan mode in ACP.

### 3. Subagent Mode Produces No `session/update` Output

**Issue**: [#952](https://github.com/QwenLM/qwen-code/issues/952)

When using subagent mode via ACP, the `session/update` notifications have empty `content` arrays, preventing monitoring of subagent progress. The tool calls show `in_progress` -> `completed` transitions but no intermediate content.

**Status**: Fixed in PR #992 (enhanced Zed integration with TodoWriteTool and TaskTool support).

### 4. Tool Call Not Found in Zed

**Issue**: [#1131](https://github.com/QwenLM/qwen-code/issues/1131)

Some users encounter repeated `"tool call not found"` errors in Zed. This appears to be a broader integration problem affecting multiple AI tools with Zed, not specific to Qwen Code. No workaround documented.

### 5. Mode Switching Fails via ACP

**Issue**: [#1295](https://github.com/QwenLM/qwen-code/issues/1295)

When switching modes via agent-shell in Emacs, the agent responds with success but does not actually change behavior. Slash commands like `/about` are treated as literal text rather than structured commands.

### 6. Authentication Persistence Issues

**Issue**: [#1855](https://github.com/QwenLM/qwen-code/issues/1855)

OAuth sessions persist after switching to an API key, causing 401 errors. Users who upgrade from free OAuth to paid API key plans cannot continue using Qwen Code without clearing cached credentials.

**Workaround**: Clear cached OAuth tokens before switching auth methods.

### 7. Telemetry Not Sent in ACP Mode

**Issue**: [#1014](https://github.com/QwenLM/qwen-code/issues/1014)

The `qwen-code.user_prompt` telemetry event is not emitted in ACP mode, even when telemetry is enabled.

### 8. Zed Connection Failures

**Issue**: [#963](https://github.com/QwenLM/qwen-code/issues/963)

Some users experience Zed failing to connect to qwen-code entirely, with the panel showing a perpetual "Loading..." indicator. This may be version-specific.

**Workaround**: Verify `qwen --version` works, validate JSON config syntax, and restart Zed.

### 9. `--acp` Flag in Tool Exclusion

A bug existed where certain tools were not properly excluded when the `--acp` flag was active. Fixed by including `--acp` in the tool exclusion check (mentioned in release notes).

### 10. File Path Handling

The `fs/read_text_file` had a bug where paths were not passed correctly to `read_many_files`. Fixed in a release (`pass paths to read_many_files in ACP`).

## Third-Party ACP Bridge

The [menhazalam/acp-qwen-code](https://github.com/menhazalam/acp-qwen-code) project provides an alternative ACP bridge:

- TypeScript-based Node.js bridge between ACP editors and the Qwen CLI
- Process isolation: each editor session spawns independent CLI processes with `AbortController`
- Uses Node.js streams converted to Web Streams API for bidirectional data flow
- Zod schema validation for protocol messages
- Configurable permission modes via `ACP_PERMISSION_MODE` env var:
  - `default`: prompts for approval
  - `acceptEdits`: auto-approves file modifications
  - `bypassPermissions`: auto-approves everything
- `ACP_PATH_TO_QWEN_CODE_EXECUTABLE` env var to specify CLI location
- `ACP_DEBUG=true` for diagnostic logging

---

## Rust Client Implementation

The `agent-client-protocol` crate (v0.9.4) provides the official Rust SDK for ACP. Below are working examples showing how to build a client that interacts with Qwen Code via ACP.

### Dependencies

```toml
[dependencies]
agent-client-protocol = "0.9"
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["compat"] }
anyhow = "1"
log = "0.4"
env_logger = "0.11"
```

### 1. Connecting to Qwen Code via ACP

This example spawns `qwen --acp` as a subprocess and communicates via stdio:

```rust
use agent_client_protocol::{self as acp, Agent as _};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Spawn qwen in ACP mode
    let mut child = tokio::process::Command::new("qwen")
        .args(["--acp"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped()) // Capture diagnostic logs
        .kill_on_drop(true)
        .spawn()?;

    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            // Create the connection with our Client implementation
            let (conn, handle_io) =
                acp::ClientSideConnection::new(
                    MyClient::new(),
                    outgoing,
                    incoming,
                    |fut| { tokio::task::spawn_local(fut); },
                );

            // Drive I/O in the background
            tokio::task::spawn_local(handle_io);

            // Step 1: Initialize with capability negotiation
            let init_response = conn.initialize(acp::InitializeRequest {
                protocol_version: acp::V1,
                client_capabilities: acp::ClientCapabilities {
                    fs: Some(acp::FsCapabilities {
                        read_text_file: Some(true),
                        write_text_file: Some(true),
                    }),
                    terminal: Some(true),
                    ..Default::default()
                },
                client_info: Some(acp::Implementation {
                    name: "my-rust-client".to_string(),
                    title: Some("My Rust ACP Client".to_string()),
                    version: "0.1.0".to_string(),
                }),
                meta: None,
            }).await?;

            log::info!("Connected to: {:?}", init_response.agent_info);

            // Step 2: Create a session
            let session = conn.new_session(acp::NewSessionRequest {
                mcp_servers: Vec::new(),
                cwd: std::env::current_dir()?,
                meta: None,
            }).await?;

            let session_id = session.session_id;

            // Step 3: Send a prompt
            let result = conn.prompt(acp::PromptRequest {
                session_id: session_id.clone(),
                prompt: vec!["List the files in the current directory".into()],
                meta: None,
            }).await?;

            log::info!("Prompt completed with stop reason: {:?}", result.stop_reason);

            drop(child);
            Ok(())
        })
        .await
}
```

### 2. Handling Reverse Requests

When running in ACP mode, Qwen Code cannot directly read files, write files, or execute commands. It sends **reverse requests** to the client. Here is a full `Client` trait implementation that handles all reverse request types:

```rust
use agent_client_protocol::{self as acp};
use std::collections::HashMap;
use std::sync::Mutex;

struct MyClient {
    terminals: Mutex<HashMap<String, TerminalState>>,
}

struct TerminalState {
    child: Option<tokio::process::Child>,
    output: String,
    exit_status: Option<i32>,
}

impl MyClient {
    fn new() -> Self {
        Self {
            terminals: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for MyClient {
    /// Handle permission requests from the agent.
    /// The agent asks before performing sensitive operations.
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        // Log what the agent wants to do
        log::info!(
            "Permission requested for tool: {}",
            args.tool_call.title
        );

        // Auto-approve all operations (adjust for your security model)
        // In production, present options to user and return their choice
        let option = args.options.first()
            .ok_or_else(|| acp::Error::internal("no permission options"))?;

        Ok(acp::RequestPermissionResponse {
            option_id: option.id.clone(),
            meta: None,
        })
    }

    /// Handle file read requests from the agent.
    /// The agent calls this instead of reading files directly.
    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        let path = &args.path; // Always absolute
        log::info!("Agent reading file: {}", path.display());

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| acp::Error::internal(format!("read failed: {e}")))?;

        // Respect optional line/limit params for partial reads
        let result = match (args.line, args.limit) {
            (Some(start), Some(limit)) => {
                let start = (start as usize).saturating_sub(1); // 1-indexed
                content.lines()
                    .skip(start)
                    .take(limit as usize)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            (Some(start), None) => {
                let start = (start as usize).saturating_sub(1);
                content.lines()
                    .skip(start)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => content,
        };

        Ok(acp::ReadTextFileResponse {
            content: result,
            meta: None,
        })
    }

    /// Handle file write requests from the agent.
    /// Creates the file if it does not exist.
    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        let path = &args.path;
        log::info!("Agent writing file: {}", path.display());

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| acp::Error::internal(format!("mkdir failed: {e}")))?;
        }

        tokio::fs::write(path, &args.content)
            .await
            .map_err(|e| acp::Error::internal(format!("write failed: {e}")))?;

        Ok(acp::WriteTextFileResponse { meta: None })
    }

    /// Handle session update notifications (streaming output).
    async fn session_notification(
        &self,
        args: acp::SessionNotification,
    ) -> acp::Result<(), acp::Error> {
        match &args.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                match &chunk.content {
                    acp::ContentBlock::Text(tc) => {
                        print!("{}", tc.text); // Stream text to stdout
                    }
                    _ => {}
                }
            }
            acp::SessionUpdate::ToolCall(tc) => {
                log::info!("Tool call [{}]: {} ({})",
                    tc.kind, tc.title, tc.status);
            }
            acp::SessionUpdate::ToolCallUpdate(tcu) => {
                if let Some(status) = &tcu.status {
                    log::info!("Tool update: {}", status);
                }
            }
            acp::SessionUpdate::Plan(plan) => {
                log::info!("Agent plan received");
            }
            _ => {}
        }
        Ok(())
    }

    // Extension points for custom methods
    async fn ext_method(
        &self,
        _args: acp::ExtRequest,
    ) -> acp::Result<acp::ExtResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(
        &self,
        _args: acp::ExtNotification,
    ) -> acp::Result<()> {
        Ok(())
    }
}
```

### 3. Handling Terminal/Command Execution Requests

When the agent needs to run shell commands, it sends `terminal/create` requests. The client must spawn the process, track its output, and respond to subsequent `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, and `terminal/release` requests:

```rust
use std::process::Stdio;
use tokio::io::AsyncReadExt;

#[async_trait::async_trait(?Send)]
impl acp::Client for MyClient {
    // ... (other methods from above) ...

    /// Create a terminal and execute a command.
    /// Returns immediately with a terminal ID; does not wait for completion.
    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        log::info!("Terminal create: {} {:?}", args.command, args.args);

        let child = tokio::process::Command::new(&args.command)
            .args(&args.args)
            .envs(
                args.env.iter()
                    .map(|e| (&e.name, &e.value))
            )
            .current_dir(&args.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| acp::Error::internal(
                format!("spawn failed: {e}")
            ))?;

        let terminal_id = format!("term-{}", child.id().unwrap_or(0));

        self.terminals.lock().unwrap().insert(
            terminal_id.clone(),
            TerminalState {
                child: Some(child),
                output: String::new(),
                exit_status: None,
            },
        );

        Ok(acp::CreateTerminalResponse {
            terminal_id,
            meta: None,
        })
    }

    /// Retrieve current terminal output.
    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        let terminals = self.terminals.lock().unwrap();
        let state = terminals.get(&args.terminal_id)
            .ok_or_else(|| acp::Error::internal("unknown terminal"))?;

        Ok(acp::TerminalOutputResponse {
            output: state.output.clone(),
            truncated: false,
            exit_status: state.exit_status.map(|code| acp::ExitStatus {
                exit_code: Some(code),
                signal: None,
            }),
            meta: None,
        })
    }

    /// Block until the terminal command completes.
    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        // In a real implementation, await child process completion
        let exit_status = {
            let mut terminals = self.terminals.lock().unwrap();
            let state = terminals.get_mut(&args.terminal_id)
                .ok_or_else(|| acp::Error::internal("unknown terminal"))?;
            state.exit_status.unwrap_or(0)
        };

        Ok(acp::WaitForTerminalExitResponse {
            exit_status: acp::ExitStatus {
                exit_code: Some(exit_status),
                signal: None,
            },
            meta: None,
        })
    }

    /// Terminate a running command.
    async fn kill_terminal_command(
        &self,
        args: acp::KillTerminalCommandRequest,
    ) -> acp::Result<acp::KillTerminalCommandResponse> {
        let mut terminals = self.terminals.lock().unwrap();
        if let Some(state) = terminals.get_mut(&args.terminal_id) {
            if let Some(ref mut child) = state.child {
                let _ = child.kill().await;
            }
        }
        Ok(acp::KillTerminalCommandResponse { meta: None })
    }

    /// Kill and release all resources for a terminal.
    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        let mut terminals = self.terminals.lock().unwrap();
        if let Some(mut state) = terminals.remove(&args.terminal_id) {
            if let Some(ref mut child) = state.child {
                let _ = child.kill().await;
            }
        }
        Ok(acp::ReleaseTerminalResponse { meta: None })
    }
}
```

### 4. Streaming to a Desktop App via Channels

This example shows how to bridge the ACP streaming notifications to a desktop application framework (Tauri, iced, etc.) using `tokio::sync::mpsc` channels:

```rust
use tokio::sync::mpsc;

/// Events sent from the ACP client to the UI layer.
#[derive(Debug, Clone)]
enum AgentEvent {
    /// Streamed text chunk from the agent
    TextChunk(String),
    /// Agent started a tool call
    ToolCallStarted {
        id: String,
        title: String,
        kind: String,
    },
    /// Tool call status update
    ToolCallUpdate {
        id: String,
        status: String,
    },
    /// Agent plan received
    PlanReceived(String),
    /// Prompt turn completed
    TurnComplete {
        stop_reason: String,
    },
    /// Permission requested -- UI should present options
    PermissionRequest {
        tool_title: String,
        /// Send the chosen option_id back through this channel
        response_tx: tokio::sync::oneshot::Sender<String>,
    },
    /// Error occurred
    Error(String),
}

/// ACP Client that forwards all events through an mpsc channel.
struct ChannelClient {
    event_tx: mpsc::UnboundedSender<AgentEvent>,
}

impl ChannelClient {
    fn new(event_tx: mpsc::UnboundedSender<AgentEvent>) -> Self {
        Self { event_tx }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for ChannelClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        // Create a oneshot channel for the UI to respond through
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.event_tx.send(AgentEvent::PermissionRequest {
            tool_title: args.tool_call.title.clone(),
            response_tx,
        }).map_err(|_| acp::Error::internal("channel closed"))?;

        // Wait for UI to respond with the chosen option ID
        let option_id = response_rx.await
            .map_err(|_| acp::Error::internal("permission response dropped"))?;

        Ok(acp::RequestPermissionResponse {
            option_id,
            meta: None,
        })
    }

    async fn session_notification(
        &self,
        args: acp::SessionNotification,
    ) -> acp::Result<(), acp::Error> {
        let event = match args.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                let text = match chunk.content {
                    acp::ContentBlock::Text(tc) => tc.text,
                    acp::ContentBlock::Image(_) => "[image]".into(),
                    acp::ContentBlock::Audio(_) => "[audio]".into(),
                    acp::ContentBlock::ResourceLink(rl) => {
                        format!("[resource: {}]", rl.uri)
                    }
                    acp::ContentBlock::Resource(_) => "[resource]".into(),
                };
                AgentEvent::TextChunk(text)
            }
            acp::SessionUpdate::ToolCall(tc) => {
                AgentEvent::ToolCallStarted {
                    id: tc.tool_call_id.clone(),
                    title: tc.title.clone(),
                    kind: format!("{:?}", tc.kind),
                }
            }
            acp::SessionUpdate::ToolCallUpdate(tcu) => {
                AgentEvent::ToolCallUpdate {
                    id: tcu.tool_call_id.clone(),
                    status: format!("{:?}", tcu.status),
                }
            }
            acp::SessionUpdate::Plan(_plan) => {
                AgentEvent::PlanReceived("Plan received".into())
            }
            _ => return Ok(()),
        };

        self.event_tx.send(event)
            .map_err(|_| acp::Error::internal("channel closed"))?;
        Ok(())
    }

    // File and terminal methods delegate to the host system
    // (same implementations as the MyClient examples above)

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        let content = tokio::fs::read_to_string(&args.path)
            .await
            .map_err(|e| acp::Error::internal(format!("{e}")))?;
        Ok(acp::ReadTextFileResponse { content, meta: None })
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        tokio::fs::write(&args.path, &args.content)
            .await
            .map_err(|e| acp::Error::internal(format!("{e}")))?;
        Ok(acp::WriteTextFileResponse { meta: None })
    }

    async fn create_terminal(
        &self,
        _args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        // Delegate to terminal handler (see Section 3)
        Err(acp::Error::method_not_found())
    }

    async fn terminal_output(
        &self,
        _args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn release_terminal(
        &self,
        _args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn wait_for_terminal_exit(
        &self,
        _args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn kill_terminal_command(
        &self,
        _args: acp::KillTerminalCommandRequest,
    ) -> acp::Result<acp::KillTerminalCommandResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_method(
        &self,
        _args: acp::ExtRequest,
    ) -> acp::Result<acp::ExtResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(
        &self,
        _args: acp::ExtNotification,
    ) -> acp::Result<()> {
        Ok(())
    }
}

/// Example: Driving the ACP client and consuming events in a UI loop.
async fn run_with_ui() -> anyhow::Result<()> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();

    // Spawn the ACP connection in a background task
    let acp_handle = tokio::task::spawn_local(async move {
        let mut child = tokio::process::Command::new("qwen")
            .args(["--acp"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("Failed to spawn qwen");

        let outgoing = child.stdin.take().unwrap().compat_write();
        let incoming = child.stdout.take().unwrap().compat();

        let (conn, handle_io) = acp::ClientSideConnection::new(
            ChannelClient::new(event_tx),
            outgoing,
            incoming,
            |fut| { tokio::task::spawn_local(fut); },
        );
        tokio::task::spawn_local(handle_io);

        // Initialize and create session
        conn.initialize(acp::InitializeRequest {
            protocol_version: acp::V1,
            client_capabilities: acp::ClientCapabilities::default(),
            client_info: Some(acp::Implementation {
                name: "desktop-app".into(),
                title: Some("Desktop App".into()),
                version: "0.1.0".into(),
            }),
            meta: None,
        }).await.expect("init failed");

        let session = conn.new_session(acp::NewSessionRequest {
            mcp_servers: Vec::new(),
            cwd: std::env::current_dir().unwrap(),
            meta: None,
        }).await.expect("session failed");

        // Send a prompt -- notifications flow through the channel
        let result = conn.prompt(acp::PromptRequest {
            session_id: session.session_id,
            prompt: vec!["Hello, what can you help me with?".into()],
            meta: None,
        }).await;

        if let Err(e) = result {
            log::error!("Prompt error: {e}");
        }
    });

    // UI event loop -- consume events from the channel
    // In Tauri: emit these as window events
    // In iced: send as Messages to your Application
    while let Some(event) = event_rx.recv().await {
        match event {
            AgentEvent::TextChunk(text) => {
                // Tauri: window.emit("agent-text", text)
                // iced: return Command::perform(async { Message::AgentText(text) })
                print!("{text}");
            }
            AgentEvent::ToolCallStarted { title, kind, .. } => {
                println!("\n[Tool: {title} ({kind})]");
            }
            AgentEvent::ToolCallUpdate { status, .. } => {
                println!("[Status: {status}]");
            }
            AgentEvent::TurnComplete { stop_reason } => {
                println!("\n[Done: {stop_reason}]");
                break;
            }
            AgentEvent::PermissionRequest { tool_title, response_tx } => {
                // In a real app, show a dialog and send back the user's choice
                println!("[Permission requested: {tool_title}]");
                let _ = response_tx.send("allow_once".into());
            }
            AgentEvent::Error(e) => {
                eprintln!("[Error: {e}]");
            }
            _ => {}
        }
    }

    acp_handle.await?;
    Ok(())
}
```

## Key Architectural Notes

1. **`LocalSet` Requirement**: The ACP Rust SDK uses `!Send` futures, requiring `tokio::task::LocalSet` and `spawn_local` instead of regular `tokio::spawn`.

2. **Bidirectional by Design**: ACP is fundamentally bidirectional. The `ClientSideConnection` provides methods to call the agent (`.initialize()`, `.prompt()`, etc.) while the `Client` trait implementation handles reverse calls from the agent.

3. **Capability Gating**: Always declare your capabilities in `InitializeRequest`. If you don't declare `fs.readTextFile: true`, the agent will not attempt file reads via your client. Undeclared capabilities result in the agent falling back to its own tools (which may fail in ACP mode).

4. **Usage Metrics**: Qwen Code returns token usage in `session/update` metadata (PR #1176): `promptTokens`, `completionTokens`, `totalTokens`, `costInMs`.

5. **Graceful Cancellation**: Send `session/cancel` (notification, no response expected). The agent will stop LLM requests and tool invocations, then respond to the original `session/prompt` with `stopReason: "cancelled"`.

## Sources

- [ACP Protocol Overview](https://agentclientprotocol.com/protocol/overview)
- [ACP Transport Specification](https://agentclientprotocol.com/protocol/transports)
- [ACP Tool Calls](https://agentclientprotocol.com/protocol/tool-calls)
- [ACP Terminals](https://agentclientprotocol.com/protocol/terminals)
- [ACP File System](https://agentclientprotocol.com/protocol/file-system)
- [ACP Prompt Turn](https://agentclientprotocol.com/protocol/prompt-turn)
- [ACP Session Setup](https://agentclientprotocol.com/protocol/session-setup)
- [ACP Session Modes](https://agentclientprotocol.com/protocol/session-modes)
- [ACP Initialization](https://agentclientprotocol.com/protocol/initialization)
- [ACP Content Blocks](https://agentclientprotocol.com/protocol/content)
- [Qwen Code GitHub](https://github.com/QwenLM/qwen-code)
- [Qwen Code Docs - Zed Integration](https://qwenlm.github.io/qwen-code-docs/en/users/integration-zed/)
- [Qwen Code Docs - JetBrains Integration](https://qwenlm.github.io/qwen-code-docs/en/users/integration-jetbrains/)
- [Rust ACP SDK (docs.rs)](https://docs.rs/agent-client-protocol)
- [Rust SDK Examples](https://github.com/agentclientprotocol/rust-sdk/blob/main/examples/client.rs)
- [ACP Bridge for Qwen Code](https://github.com/menhazalam/acp-qwen-code)
- [Zed ACP Agent Page](https://zed.dev/acp/agent/qwen-code)
- [ACP Intro Blog (Goose)](https://block.github.io/goose/blog/2025/10/24/intro-to-agent-client-protocol-acp/)
- [Issue #88 - Original ACP Support Request](https://github.com/QwenLM/qwen-code/issues/88)
- [Issue #952 - Subagent No Output](https://github.com/QwenLM/qwen-code/issues/952)
- [Issue #1131 - Tool Call Not Found](https://github.com/QwenLM/qwen-code/issues/1131)
- [Issue #1151 - Plan Mode Bug](https://github.com/QwenLM/qwen-code/issues/1151)
- [Issue #1295 - Mode Switching Fails](https://github.com/QwenLM/qwen-code/issues/1295)
- [Issue #1350 - Graduate --acp Flag](https://github.com/QwenLM/qwen-code/issues/1350)
- [Issue #1502 - JetBrains ACP v2 Incompatibility](https://github.com/QwenLM/qwen-code/issues/1502)
- [Issue #1855 - OAuth Persistence](https://github.com/QwenLM/qwen-code/issues/1855)
- [DeepWiki - Qwen Code IDE Integration](https://deepwiki.com/QwenLM/qwen-code/7-ide-integration)

