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

last_updated: 2026-02-23
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
| `@zed-industries/claude-agent-acp` | TypeScript | `npm i -g @zed-industries/claude-agent-acp` | Official Zed adapter, most mature |
| `claude-code-acp-rs` | Rust | `cargo install claude-code-acp-rs` | Uses `sacp` + `agent-client-protocol-schema` internally |
| `acp-claude-code` (Xuanwo) | TypeScript | npm / GitHub | Community alternative |

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

The `@zed-industries/claude-agent-acp` package may need manual patching at `sdk.mjs` line 6515: change `join(dirname, "entrypoints", "cli.js")` to `join(dirname, "claude-code", "cli.js")`. This fix resets on reinstall.

**13. `!Send` futures in the official Rust SDK**

The `agent-client-protocol` crate's connection futures are `!Send`, meaning you **must** use `tokio::task::LocalSet` and `spawn_local`. Standard `tokio::spawn` will not compile. This is a deliberate design choice to avoid `Arc<Mutex<>>` overhead, but it catches many developers off guard.

**14. Non-exhaustive enums everywhere**

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

The following examples show how to build a Rust ACP client using the official `agent-client-protocol` crate (v0.9.4). All examples use `tokio` as the async runtime.

### Dependencies

```toml
[dependencies]
agent-client-protocol = "0.9"
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["compat"] }
anyhow = "1"
```

### 1. Basic ACP Client: Connecting to the Agent

This example spawns a Claude Code ACP adapter as a subprocess and performs the full initialization → session → prompt lifecycle.

```rust
use agent_client_protocol as acp;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// A minimal ACP client that prints session updates to stdout.
struct MinimalClient;

#[async_trait::async_trait(?Send)]
impl acp::MessageHandler<acp::ClientSide> for MinimalClient {
    // Permission request: auto-approve everything (for demo purposes)
    async fn request_permission(
        &self,
        params: acp::RequestPermissionRequest,
    ) -> anyhow::Result<acp::RequestPermissionResponse> {
        let first_allow = params
            .options
            .iter()
            .find(|o| matches!(o.kind, acp::PermissionOptionKind::AllowOnce))
            .or_else(|| params.options.first());

        Ok(acp::RequestPermissionResponse {
            outcome: match first_allow {
                Some(opt) => acp::RequestPermissionOutcome::Selected {
                    option_id: opt.id.clone(),
                },
                None => acp::RequestPermissionOutcome::Cancelled,
            },
        })
    }

    // Streaming session updates arrive here
    async fn session_notification(
        &self,
        params: acp::SessionNotification,
    ) -> anyhow::Result<()> {
        match &params.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(t) = &chunk.content {
                    print!("{}", t.text);
                }
            }
            acp::SessionUpdate::ToolCall(tc) => {
                println!("\n[tool: {} ({})]", tc.title, tc.tool_call_id);
            }
            acp::SessionUpdate::ToolCallUpdate(upd) => {
                println!("[tool update: {} → {:?}]", upd.tool_call_id, upd.status);
            }
            _ => {} // Always include wildcard — enums are non-exhaustive
        }
        Ok(())
    }

    // File read reverse request (stub — see section 2 for full impl)
    async fn read_text_file(
        &self,
        _params: acp::ReadTextFileRequest,
    ) -> anyhow::Result<acp::ReadTextFileResponse> {
        anyhow::bail!("fs/read_text_file not implemented")
    }

    // File write reverse request (stub)
    async fn write_text_file(
        &self,
        _params: acp::WriteTextFileRequest,
    ) -> anyhow::Result<acp::WriteTextFileResponse> {
        anyhow::bail!("fs/write_text_file not implemented")
    }

    // Terminal reverse requests (stubs — see section 3 for full impl)
    async fn create_terminal(
        &self,
        _params: acp::CreateTerminalRequest,
    ) -> anyhow::Result<acp::CreateTerminalResponse> {
        anyhow::bail!("terminal/create not implemented")
    }

    async fn terminal_output(
        &self,
        _params: acp::TerminalOutputRequest,
    ) -> anyhow::Result<acp::TerminalOutputResponse> {
        anyhow::bail!("terminal/output not implemented")
    }

    async fn wait_for_terminal_exit(
        &self,
        _params: acp::WaitForTerminalExitRequest,
    ) -> anyhow::Result<acp::WaitForTerminalExitResponse> {
        anyhow::bail!("terminal/wait_for_exit not implemented")
    }

    async fn kill_terminal_command(
        &self,
        _params: acp::KillTerminalCommandRequest,
    ) -> anyhow::Result<acp::KillTerminalCommandResponse> {
        anyhow::bail!("terminal/kill not implemented")
    }

    async fn release_terminal(
        &self,
        _params: acp::ReleaseTerminalRequest,
    ) -> anyhow::Result<acp::ReleaseTerminalResponse> {
        anyhow::bail!("terminal/release not implemented")
    }

    async fn ext_method(
        &self,
        _params: acp::ExtRequest,
    ) -> anyhow::Result<acp::ExtResponse> {
        anyhow::bail!("ext method not implemented")
    }

    async fn ext_notification(
        &self,
        _params: acp::ExtNotification,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn the ACP agent adapter as a subprocess
    let mut child = tokio::process::Command::new("claude-agent-acp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit()) // Let agent logs pass through
        .kill_on_drop(true)
        .spawn()?;

    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();

    // IMPORTANT: ACP SDK futures are !Send — must use LocalSet
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (conn, handle_io) = acp::ClientSideConnection::new(
                MinimalClient,
                outgoing,
                incoming,
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            // Drive the I/O loop in the background
            tokio::task::spawn_local(handle_io);

            // 1. Initialize — negotiate capabilities
            let init_response = conn
                .initialize(acp::InitializeRequest {
                    protocol_version: acp::V1,
                    client_capabilities: acp::ClientCapabilities {
                        fs: Some(acp::FileSystemCapability {
                            read_text_file: Some(true),
                            write_text_file: Some(true),
                        }),
                        terminal: Some(true),
                    },
                    client_info: Some(acp::Implementation {
                        name: "rusty-client".into(),
                        title: Some("Rusty ACP Client".into()),
                        version: "0.1.0".into(),
                    }),
                    meta: None,
                })
                .await?;

            println!(
                "Connected to agent: {:?} (protocol v{})",
                init_response.agent_info, init_response.protocol_version
            );

            // 2. Create session
            let session = conn
                .new_session(acp::NewSessionRequest {
                    cwd: std::env::current_dir()?,
                    mcp_servers: vec![],
                    meta: None,
                })
                .await?;

            println!("Session created: {}", session.session_id);

            // 3. Send prompt — streaming updates arrive via session_notification()
            let result = conn
                .prompt(acp::PromptRequest {
                    session_id: session.session_id,
                    prompt: vec!["What files are in this directory?".into()],
                    meta: None,
                })
                .await?;

            println!("\nDone. Stop reason: {:?}", result.stop_reason);
            Ok(())
        })
        .await
}
```

### 2. Handling Reverse Requests: File System Operations

When the agent needs to read or write files, it sends reverse requests to the client. Here is a full implementation of the file system handlers:

```rust
use std::path::PathBuf;

/// An ACP client that handles file system reverse requests.
struct FsCapableClient {
    /// Root directory for sandboxing file access.
    project_root: PathBuf,
}

impl FsCapableClient {
    fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// Validate that a path is within the project root (security boundary).
    fn validate_path(&self, path: &std::path::Path) -> anyhow::Result<PathBuf> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !canonical.starts_with(&self.project_root) {
            anyhow::bail!(
                "Path {} is outside project root {}",
                canonical.display(),
                self.project_root.display()
            );
        }
        Ok(canonical)
    }
}

#[async_trait::async_trait(?Send)]
impl acp::MessageHandler<acp::ClientSide> for FsCapableClient {
    async fn read_text_file(
        &self,
        params: acp::ReadTextFileRequest,
    ) -> anyhow::Result<acp::ReadTextFileResponse> {
        let path = self.validate_path(&params.path)?;
        let content = tokio::fs::read_to_string(&path).await?;

        // Apply optional line/limit filtering
        let filtered = match (params.line, params.limit) {
            (Some(start_line), Some(limit)) => {
                // ACP uses 1-based line numbers
                let start = (start_line as usize).saturating_sub(1);
                content
                    .lines()
                    .skip(start)
                    .take(limit as usize)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            (Some(start_line), None) => {
                let start = (start_line as usize).saturating_sub(1);
                content
                    .lines()
                    .skip(start)
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => content,
        };

        Ok(acp::ReadTextFileResponse { content: filtered })
    }

    async fn write_text_file(
        &self,
        params: acp::WriteTextFileRequest,
    ) -> anyhow::Result<acp::WriteTextFileResponse> {
        let path = self.validate_path(&params.path)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&path, &params.content).await?;
        Ok(acp::WriteTextFileResponse {})
    }

    async fn request_permission(
        &self,
        params: acp::RequestPermissionRequest,
    ) -> anyhow::Result<acp::RequestPermissionResponse> {
        // Log the permission request for audit
        if let Some(tc) = &params.tool_call {
            println!(
                "[permission] Agent requests: {} (kind: {:?})",
                tc.title, tc.kind
            );
        }

        // Auto-approve read operations, prompt for others
        let is_read = params
            .tool_call
            .as_ref()
            .map(|tc| matches!(tc.kind, Some(acp::ToolKind::Read)))
            .unwrap_or(false);

        let option_kind = if is_read {
            acp::PermissionOptionKind::AllowOnce
        } else {
            // In a real app, you'd show a UI prompt here
            acp::PermissionOptionKind::AllowOnce
        };

        let selected = params
            .options
            .iter()
            .find(|o| o.kind == option_kind)
            .or_else(|| params.options.first());

        Ok(acp::RequestPermissionResponse {
            outcome: match selected {
                Some(opt) => acp::RequestPermissionOutcome::Selected {
                    option_id: opt.id.clone(),
                },
                None => acp::RequestPermissionOutcome::Cancelled,
            },
        })
    }

    // ... (other handlers as in section 1)
    # // Remaining handlers omitted for brevity — see section 1 stub pattern
}
```

### 3. Handling Terminal Execution Requests

When the agent needs to run commands on the host system, it uses the terminal reverse request lifecycle. Here is a full implementation:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tracks active terminal processes.
struct TerminalManager {
    terminals: Arc<Mutex<HashMap<String, TerminalHandle>>>,
    next_id: Arc<Mutex<u64>>,
}

struct TerminalHandle {
    child: tokio::process::Child,
    stdout_buf: Vec<u8>,
    stderr_buf: Vec<u8>,
    output_limit: usize,
    exited: bool,
    exit_code: Option<i32>,
}

impl TerminalManager {
    fn new() -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    async fn next_terminal_id(&self) -> String {
        let mut id = self.next_id.lock().await;
        *id += 1;
        format!("term_{}", *id)
    }
}

/// An ACP client with full terminal execution support.
struct TerminalCapableClient {
    terminals: TerminalManager,
    project_root: PathBuf,
}

#[async_trait::async_trait(?Send)]
impl acp::MessageHandler<acp::ClientSide> for TerminalCapableClient {
    async fn create_terminal(
        &self,
        params: acp::CreateTerminalRequest,
    ) -> anyhow::Result<acp::CreateTerminalResponse> {
        let cwd = params
            .cwd
            .unwrap_or_else(|| self.project_root.clone());

        let output_limit = params.output_byte_limit.unwrap_or(1_048_576) as usize;

        let child = tokio::process::Command::new(&params.command)
            .args(params.args.unwrap_or_default())
            .envs(
                params
                    .env
                    .unwrap_or_default()
                    .into_iter()
                    .map(|e| (e.name, e.value)),
            )
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let terminal_id = self.terminals.next_terminal_id().await;
        let handle = TerminalHandle {
            child,
            stdout_buf: Vec::new(),
            stderr_buf: Vec::new(),
            output_limit,
            exited: false,
            exit_code: None,
        };

        self.terminals
            .terminals
            .lock()
            .await
            .insert(terminal_id.clone(), handle);

        Ok(acp::CreateTerminalResponse {
            terminal_id: terminal_id.into(),
        })
    }

    async fn terminal_output(
        &self,
        params: acp::TerminalOutputRequest,
    ) -> anyhow::Result<acp::TerminalOutputResponse> {
        let mut terminals = self.terminals.terminals.lock().await;
        let handle = terminals
            .get_mut(params.terminal_id.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Unknown terminal: {}", params.terminal_id))?;

        // Read available output from child process
        if let Some(stdout) = handle.child.stdout.as_mut() {
            use tokio::io::AsyncReadExt;
            let mut buf = vec![0u8; 4096];
            // Non-blocking read of available data
            match tokio::time::timeout(
                std::time::Duration::from_millis(100),
                stdout.read(&mut buf),
            )
            .await
            {
                Ok(Ok(n)) if n > 0 => handle.stdout_buf.extend_from_slice(&buf[..n]),
                _ => {}
            }
        }

        let truncated = handle.stdout_buf.len() > handle.output_limit;
        let output = if truncated {
            String::from_utf8_lossy(&handle.stdout_buf[..handle.output_limit]).into_owned()
        } else {
            String::from_utf8_lossy(&handle.stdout_buf).into_owned()
        };

        Ok(acp::TerminalOutputResponse {
            output,
            truncated,
            exit_status: handle.exit_code,
        })
    }

    async fn wait_for_terminal_exit(
        &self,
        params: acp::WaitForTerminalExitRequest,
    ) -> anyhow::Result<acp::WaitForTerminalExitResponse> {
        let mut terminals = self.terminals.terminals.lock().await;
        let handle = terminals
            .get_mut(params.terminal_id.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Unknown terminal: {}", params.terminal_id))?;

        let status = handle.child.wait().await?;
        handle.exited = true;
        handle.exit_code = status.code();

        Ok(acp::WaitForTerminalExitResponse {
            exit_code: status.code(),
            signal: None, // Platform-specific; could use status.signal() on unix
        })
    }

    async fn kill_terminal_command(
        &self,
        params: acp::KillTerminalCommandRequest,
    ) -> anyhow::Result<acp::KillTerminalCommandResponse> {
        let mut terminals = self.terminals.terminals.lock().await;
        let handle = terminals
            .get_mut(params.terminal_id.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Unknown terminal: {}", params.terminal_id))?;

        handle.child.kill().await?;
        Ok(acp::KillTerminalCommandResponse {})
    }

    async fn release_terminal(
        &self,
        params: acp::ReleaseTerminalRequest,
    ) -> anyhow::Result<acp::ReleaseTerminalResponse> {
        let mut terminals = self.terminals.terminals.lock().await;

        if let Some(mut handle) = terminals.remove(params.terminal_id.as_ref()) {
            // Kill if still running, then drop
            if !handle.exited {
                let _ = handle.child.kill().await;
            }
        }

        Ok(acp::ReleaseTerminalResponse {})
    }

    // ... (permission, fs, session_notification handlers as above)
}
```

### 4. Streaming to a Desktop App via `mpsc` Channels

This example shows how to bridge ACP streaming updates to a desktop UI framework (Tauri or iced) using `tokio::sync::mpsc` channels. The ACP client runs on a background thread while the UI thread receives typed events through the channel.

```rust
use tokio::sync::mpsc;

/// Events sent from the ACP client to the UI layer.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent is streaming text content.
    TextChunk(String),
    /// Agent is reasoning internally (extended thinking).
    ThoughtChunk(String),
    /// A tool call has started.
    ToolCallStarted {
        id: String,
        title: String,
        kind: String,
    },
    /// A tool call has completed or failed.
    ToolCallFinished {
        id: String,
        status: String,
        content: Option<String>,
    },
    /// Agent requests permission for an action.
    PermissionRequest {
        request_id: String,
        title: String,
        options: Vec<(String, String)>, // (id, label)
    },
    /// The prompt turn has completed.
    TurnComplete {
        stop_reason: String,
    },
    /// An error occurred.
    Error(String),
}

/// Messages sent from the UI layer back to the ACP client.
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// Send a prompt to the agent.
    SendPrompt(String),
    /// Respond to a permission request.
    PermissionResponse {
        request_id: String,
        option_id: String,
    },
    /// Cancel the current operation.
    Cancel,
}

/// ACP client that bridges to UI via channels.
struct ChannelClient {
    event_tx: mpsc::UnboundedSender<AgentEvent>,
    /// For permission responses, the UI sends back through this.
    /// Keyed by JSON-RPC request id.
    pending_permissions: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<String>>>>,
}

#[async_trait::async_trait(?Send)]
impl acp::MessageHandler<acp::ClientSide> for ChannelClient {
    async fn session_notification(
        &self,
        params: acp::SessionNotification,
    ) -> anyhow::Result<()> {
        let event = match params.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(t) = chunk.content {
                    AgentEvent::TextChunk(t.text)
                } else {
                    return Ok(());
                }
            }
            acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                if let acp::ContentBlock::Text(t) = chunk.content {
                    AgentEvent::ThoughtChunk(t.text)
                } else {
                    return Ok(());
                }
            }
            acp::SessionUpdate::ToolCall(tc) => AgentEvent::ToolCallStarted {
                id: tc.tool_call_id.to_string(),
                title: tc.title,
                kind: format!("{:?}", tc.kind),
            },
            acp::SessionUpdate::ToolCallUpdate(upd) => {
                let content_text = upd.content.as_ref().and_then(|blocks| {
                    blocks.iter().find_map(|b| {
                        if let acp::ToolCallContent::Content(
                            acp::ContentBlock::Text(t),
                        ) = b
                        {
                            Some(t.text.clone())
                        } else {
                            None
                        }
                    })
                });
                AgentEvent::ToolCallFinished {
                    id: upd.tool_call_id.to_string(),
                    status: format!("{:?}", upd.status),
                    content: content_text,
                }
            }
            _ => return Ok(()),
        };

        let _ = self.event_tx.send(event);
        Ok(())
    }

    async fn request_permission(
        &self,
        params: acp::RequestPermissionRequest,
    ) -> anyhow::Result<acp::RequestPermissionResponse> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Store the oneshot sender for when the UI responds
        self.pending_permissions
            .lock()
            .await
            .insert(request_id.clone(), tx);

        // Notify UI that a permission decision is needed
        let title = params
            .tool_call
            .as_ref()
            .map(|tc| tc.title.clone())
            .unwrap_or_default();

        let options: Vec<(String, String)> = params
            .options
            .iter()
            .map(|o| (o.id.to_string(), o.name.clone()))
            .collect();

        let _ = self.event_tx.send(AgentEvent::PermissionRequest {
            request_id,
            title,
            options,
        });

        // Wait for UI to respond (with timeout)
        let option_id = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            rx,
        )
        .await
        .map_err(|_| anyhow::anyhow!("Permission request timed out"))?
        .map_err(|_| anyhow::anyhow!("Permission channel closed"))?;

        Ok(acp::RequestPermissionResponse {
            outcome: acp::RequestPermissionOutcome::Selected {
                option_id: option_id.into(),
            },
        })
    }

    // ... (fs and terminal handlers as in previous sections)
}

/// Spawn the ACP client on a background LocalSet and return channel handles.
pub fn spawn_acp_client(
    agent_command: &str,
    project_dir: PathBuf,
) -> anyhow::Result<(
    mpsc::UnboundedReceiver<AgentEvent>,
    mpsc::UnboundedSender<UiCommand>,
)> {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
    let agent_command = agent_command.to_string();

    // The ACP SDK requires a LocalSet — run it on a dedicated thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");

        let local_set = tokio::task::LocalSet::new();

        local_set.block_on(&rt, async move {
            let pending_permissions: Arc<
                Mutex<HashMap<String, tokio::sync::oneshot::Sender<String>>>,
            > = Arc::new(Mutex::new(HashMap::new()));

            let client = ChannelClient {
                event_tx: event_tx.clone(),
                pending_permissions: pending_permissions.clone(),
            };

            // Spawn the ACP agent adapter
            let mut child = tokio::process::Command::new(&agent_command)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::inherit())
                .kill_on_drop(true)
                .spawn()
                .expect("Failed to spawn ACP agent");

            let outgoing = child.stdin.take().unwrap().compat_write();
            let incoming = child.stdout.take().unwrap().compat();

            let (conn, handle_io) = acp::ClientSideConnection::new(
                client,
                outgoing,
                incoming,
                |fut| { tokio::task::spawn_local(fut); },
            );

            tokio::task::spawn_local(handle_io);

            // Initialize
            let _ = conn
                .initialize(acp::InitializeRequest {
                    protocol_version: acp::V1,
                    client_capabilities: acp::ClientCapabilities {
                        fs: Some(acp::FileSystemCapability {
                            read_text_file: Some(true),
                            write_text_file: Some(true),
                        }),
                        terminal: Some(true),
                    },
                    client_info: Some(acp::Implementation {
                        name: "desktop-app".into(),
                        title: Some("Desktop ACP Client".into()),
                        version: "0.1.0".into(),
                    }),
                    meta: None,
                })
                .await;

            // Create session
            let session = conn
                .new_session(acp::NewSessionRequest {
                    cwd: project_dir,
                    mcp_servers: vec![],
                    meta: None,
                })
                .await
                .expect("Failed to create session");

            let session_id = session.session_id.clone();

            // Process UI commands
            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    UiCommand::SendPrompt(text) => {
                        let result = conn
                            .prompt(acp::PromptRequest {
                                session_id: session_id.clone(),
                                prompt: vec![text.into()],
                                meta: None,
                            })
                            .await;

                        match result {
                            Ok(resp) => {
                                let _ = event_tx.send(AgentEvent::TurnComplete {
                                    stop_reason: format!("{:?}", resp.stop_reason),
                                });
                            }
                            Err(e) => {
                                let _ = event_tx
                                    .send(AgentEvent::Error(e.to_string()));
                            }
                        }
                    }
                    UiCommand::PermissionResponse {
                        request_id,
                        option_id,
                    } => {
                        if let Some(tx) =
                            pending_permissions.lock().await.remove(&request_id)
                        {
                            let _ = tx.send(option_id);
                        }
                    }
                    UiCommand::Cancel => {
                        conn.cancel(acp::CancelNotification {
                            session_id: session_id.clone(),
                        })
                        .await;
                    }
                }
            }
        });
    });

    Ok((event_rx, cmd_tx))
}
```

#### Usage from Tauri

```rust
use tauri::Manager;

#[tauri::command]
async fn send_prompt(
    state: tauri::State<'_, AppState>,
    prompt: String,
) -> Result<(), String> {
    state
        .cmd_tx
        .send(UiCommand::SendPrompt(prompt))
        .map_err(|e| e.to_string())
}

fn setup_event_listener(app: &tauri::App, mut event_rx: mpsc::UnboundedReceiver<AgentEvent>) {
    let handle = app.handle().clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match &event {
                AgentEvent::TextChunk(text) => {
                    handle.emit("agent:text", text).ok();
                }
                AgentEvent::ToolCallStarted { title, .. } => {
                    handle.emit("agent:tool-start", title).ok();
                }
                AgentEvent::TurnComplete { stop_reason } => {
                    handle.emit("agent:done", stop_reason).ok();
                }
                _ => {}
            }
        }
    });
}
```

#### Usage from iced

```rust
use iced::futures::SinkExt;

/// iced subscription that receives AgentEvents.
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
                .expect("Subscription already consumed");

            while let Some(event) = rx.recv().await {
                output.send(event).await.ok();
            }

            // Keep alive
            std::future::pending().await
        },
    )
}

impl Application for MyApp {
    type Message = AppMessage;

    fn update(&mut self, message: AppMessage) -> iced::Task<AppMessage> {
        match message {
            AppMessage::AgentEvent(AgentEvent::TextChunk(text)) => {
                self.response_buffer.push_str(&text);
                iced::Task::none()
            }
            AppMessage::AgentEvent(AgentEvent::TurnComplete { .. }) => {
                self.is_loading = false;
                iced::Task::none()
            }
            AppMessage::UserSubmit => {
                let prompt = self.input_text.clone();
                self.input_text.clear();
                self.is_loading = true;
                self.cmd_tx.send(UiCommand::SendPrompt(prompt)).ok();
                iced::Task::none()
            }
            _ => iced::Task::none(),
        }
    }

    fn subscription(&self) -> iced::Subscription<AppMessage> {
        agent_subscription(self.event_rx.clone()).map(AppMessage::AgentEvent)
    }
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
