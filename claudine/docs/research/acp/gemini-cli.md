---
prompt: |-
    Do a deep dive on the ACP implementation that Gemini CLI provides.

    - make sure to mention any quirks or gotchas that developers mention facing when interacting with Gemini CLI as well any workarounds or ways to avoid issues

    After completing the deep dive, provide the following additional sections which cover writing code examples with Rust:

    1. Show how a Rust client can interact programmatically with the Agent using ACP
    2. Show explicitly how to handle "Reverse Requests" where the Agent asks the client to fulfill a tool request, a file read, etc. (as a Kimi is not allowed to do this directly when operating via ACP)
    3. Show how a Rust client can respond to requests to execute commands on the host system
    4. Show how the Rust client we've created can use things like `mpsc` channels to send the Agent's streaming text to a desktop desktop app framework like Tauri or iced

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-22
update_policy:
    - Duration(6 mo)
---

## Gemini CLI ACP deep dive

As of **February 22, 2026**, Gemini CLI exposes ACP through `--experimental-acp` and runs an `AgentSideConnection` over newline-delimited JSON-RPC on stdio. The implementation currently lives in `packages/cli/src/zed-integration/zedIntegration.ts` and is used by IDE integrations (especially Zed).

### Startup and transport model

- ACP mode is enabled by `--experimental-acp` (`experimentalAcp` in config parsing).
- The CLI enters ACP mode by calling `runZedIntegration(...)`, building an ndjson stream and an `AgentSideConnection`.
- Gemini explicitly waits for connection close and then runs cleanup so telemetry flushes on shutdown.

### Initialize/auth/session flow

- `initialize` advertises:
    - auth methods: Google login, Gemini API key, Vertex AI
    - `loadSession: true`
    - prompt capabilities: image/audio/embedded context
    - MCP capabilities: HTTP + SSE
- `newSession`:
    - loads settings from the passed `cwd`
    - validates auth early
    - fails fast with 401 when API-key auth is selected but no key is present
    - installs ACP-backed filesystem service if client advertises `fs` capabilities
- `loadSession`:
    - restores history
    - resumes chat state
    - streams prior messages back to the client as `session/update` notifications

### Prompt turn behavior

During `session/prompt`:

- user content is normalized into Gemini parts (text, image, audio, resources)
- streamed model chunks are mapped to ACP updates:
    - `agent_message_chunk`
    - `agent_thought_chunk`
- function/tool calls are accumulated and executed in-loop
- tool status is streamed back with `tool_call` and `tool_call_update`
- cancellation aborts pending prompt work and returns `stopReason: cancelled`
- 429s are normalized to ACP errors with a user-friendly rate-limit message

### Reverse requests Gemini currently uses

Gemini’s ACP implementation (current main) issues client reverse requests for:

- `session/request_permission`
- `fs/read_text_file`
- `fs/write_text_file`

It does not currently invoke ACP `terminal/*` reverse methods in this integration path, even though ACP and the Rust SDK support them.

## Quirks, gotchas, and workarounds

### 1) TTY freeze in ACP mode (fixed)

- Symptom: `gemini --experimental-acp` could freeze in TTY mode.
- Fix: merged September 28, 2025 (PR #10089), by forcing non-interactive behavior when `experimentalAcp` is set.
- Workaround for older builds: run ACP without TTY allocation (for example, avoid `docker -t` when testing ACP).

### 2) Windows ACP request parsing bug (fixed in v0.9.0+)

- Symptom: client sends `initialize`, Gemini never responds on Windows.
- Root cause: line splitting used platform EOL instead of protocol newline framing.
- Fix: PR #10339 switched splitting to `\n`.
- Practical floor noted by contributors: use Gemini CLI `v0.9.0` or newer for this fix.

### 3) API-key ACP auth falsely failing with quota errors (fixed February 3, 2026)

- Symptom: ACP prompts fail with “exhausted daily quota” even with valid key.
- Root cause: workspace `.env`/trust resolution tied to wrong directory in ACP contexts + late auth validation.
- Fix: PR #18025 (merged 2026-02-03) corrected workspace env loading and early key validation.

### 4) OAuth cache behavior can differ in subprocess ACP launches (open report, closed stale)

- Reported in issue #12042: launching ACP from Python subprocess prompted login even when cached creds existed.
- Current practical workaround from affected users: use API-key auth (or configure explicit auth method in ACP client integrations).

### 5) Zed integration realities you should plan for

- Zed runs a managed Gemini CLI by default; set `"ignore_system_version": false` to force PATH/system install.
- Zed docs explicitly list feature gaps for Gemini integration (for example message editing/history checkpoint differences).
- “Loading...” incidents had multiple causes in the field: outdated Gemini builds, proxy/sandbox settings, credential cache/state drift, and packaging/runtime mismatches.

### 6) Docs drift gotcha

- `docs/cli/cli-reference.md` still lists `--experimental-zed-integration`, but runtime wiring in config is based on `experimentalAcp`.
- Use `--experimental-acp` as the canonical flag.

## 1) Rust client: interact programmatically with a Gemini ACP agent

```rust
use agent_client_protocol::{self as acp, Agent as _};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

struct DesktopClient;

#[async_trait::async_trait(?Send)]
impl acp::Client for DesktopClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Ok(acp::RequestPermissionResponse::new(acp::RequestPermissionOutcome::Cancelled))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        if let acp::SessionUpdate::AgentMessageChunk(chunk) = args.update {
            if let acp::ContentBlock::Text(text) = chunk.content {
                println!("agent: {}", text.text);
            }
        }
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let mut child = tokio::process::Command::new("gemini")
                .arg("--experimental-acp")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()?;

            let outgoing = child.stdin.take().unwrap().compat_write();
            let incoming = child.stdout.take().unwrap().compat();

            let (conn, io_task) =
                acp::ClientSideConnection::new(DesktopClient, outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });
            tokio::task::spawn_local(io_task);

            conn.initialize(acp::InitializeRequest {
                protocol_version: acp::V1,
                client_capabilities: acp::ClientCapabilities::new()
                    .fs(acp::FileSystemCapability::new().read_text_file(true).write_text_file(true))
                    .terminal(true),
                client_info: Some(acp::Implementation {
                    name: "my-rust-client".into(),
                    title: Some("My Rust Client".into()),
                    version: "0.1.0".into(),
                }),
                meta: None,
            })
            .await?;

            let session = conn
                .new_session(acp::NewSessionRequest {
                    cwd: std::env::current_dir()?,
                    mcp_servers: vec![],
                    meta: None,
                })
                .await?;

            let _ = conn
                .prompt(acp::PromptRequest {
                    session_id: session.session_id,
                    prompt: vec!["Summarize the current repo".into()],
                    meta: None,
                })
                .await?;

            Ok::<_, anyhow::Error>(())
        })
        .await
}
```

## 2) Rust client: explicit reverse-request handling (permissions + file IO)

This is the critical ACP inversion point: the agent can call your client for approval and host file access.

```rust
#[async_trait::async_trait(?Send)]
impl acp::Client for DesktopClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        // Example policy: prefer allow_once, else cancel.
        let selected = args.options.iter().find(|opt| {
            matches!(
                opt.kind,
                acp::PermissionOptionKind::AllowOnce | acp::PermissionOptionKind::AllowAlways
            )
        });

        let outcome = if let Some(opt) = selected {
            acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                opt.option_id.clone(),
            ))
        } else {
            acp::RequestPermissionOutcome::Cancelled
        };

        Ok(acp::RequestPermissionResponse::new(outcome))
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        let text = tokio::fs::read_to_string(&args.path)
            .await
            .map_err(acp::Error::into_internal_error)?;

        let sliced = if args.line.is_some() || args.limit.is_some() {
            let start = args.line.unwrap_or(1).saturating_sub(1) as usize;
            let limit = args.limit.unwrap_or(u32::MAX) as usize;
            text.lines()
                .skip(start)
                .take(limit)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            text
        };

        Ok(acp::ReadTextFileResponse::new(sliced))
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        tokio::fs::write(&args.path, args.content)
            .await
            .map_err(acp::Error::into_internal_error)?;
        Ok(acp::WriteTextFileResponse::new())
    }
}
```

## 3) Rust client: respond to host command execution requests (`terminal/*`)

ACP supports terminal reverse methods. Even if a specific agent path does not use them today, this is how your client can implement them.

```rust
use std::{collections::HashMap, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}}};
use tokio::{io::AsyncReadExt, process::{Child, Command}, sync::Mutex};

struct RunningTerminal {
    child: Child,
    output: Arc<Mutex<String>>,
    truncated: Arc<AtomicBool>,
}

struct DesktopClient {
    next_terminal_id: AtomicU64,
    terminals: Arc<Mutex<HashMap<acp::TerminalId, RunningTerminal>>>,
}

#[async_trait::async_trait(?Send)]
impl acp::Client for DesktopClient {
    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        let mut cmd = Command::new(&args.command);
        cmd.args(&args.args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(cwd) = &args.cwd {
            cmd.current_dir(cwd);
        }
        for ev in &args.env {
            cmd.env(&ev.name, &ev.value);
        }

        let mut child = cmd.spawn().map_err(acp::Error::into_internal_error)?;
        let output = Arc::new(Mutex::new(String::new()));
        let truncated = Arc::new(AtomicBool::new(false));
        let limit = args.output_byte_limit.unwrap_or(256 * 1024) as usize;

        for stream in [child.stdout.take(), child.stderr.take()] {
            if let Some(mut stream) = stream {
                let output = output.clone();
                let truncated = truncated.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    loop {
                        let n = match stream.read(&mut buf).await {
                            Ok(0) | Err(_) => break,
                            Ok(n) => n,
                        };
                        let chunk = String::from_utf8_lossy(&buf[..n]);
                        let mut out = output.lock().await;
                        out.push_str(&chunk);
                        if out.len() > limit {
                            let drop_len = out.len() - limit;
                            out.drain(..drop_len);
                            truncated.store(true, Ordering::Relaxed);
                        }
                    }
                });
            }
        }

        let id_num = self.next_terminal_id.fetch_add(1, Ordering::Relaxed);
        let terminal_id = acp::TerminalId::new(format!("term-{id_num}"));
        self.terminals.lock().await.insert(
            terminal_id.clone(),
            RunningTerminal { child, output, truncated },
        );

        Ok(acp::CreateTerminalResponse::new(terminal_id))
    }

    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        let mut map = self.terminals.lock().await;
        let rt = map.get_mut(&args.terminal_id).ok_or_else(acp::Error::invalid_params)?;

        let exit_status = rt
            .child
            .try_wait()
            .map_err(acp::Error::into_internal_error)?
            .map(|s| {
                acp::TerminalExitStatus::new()
                    .exit_code(s.code().and_then(|c| u32::try_from(c).ok()))
            });

        let output = rt.output.lock().await.clone();
        Ok(acp::TerminalOutputResponse::new(
            output,
            rt.truncated.load(Ordering::Relaxed),
        )
        .exit_status(exit_status))
    }

    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        let mut map = self.terminals.lock().await;
        let rt = map.get_mut(&args.terminal_id).ok_or_else(acp::Error::invalid_params)?;
        let status = rt.child.wait().await.map_err(acp::Error::into_internal_error)?;
        Ok(acp::WaitForTerminalExitResponse::new(
            acp::TerminalExitStatus::new().exit_code(status.code().and_then(|c| u32::try_from(c).ok())),
        ))
    }

    async fn kill_terminal_command(
        &self,
        args: acp::KillTerminalCommandRequest,
    ) -> acp::Result<acp::KillTerminalCommandResponse> {
        let mut map = self.terminals.lock().await;
        let rt = map.get_mut(&args.terminal_id).ok_or_else(acp::Error::invalid_params)?;
        rt.child.start_kill().map_err(acp::Error::into_internal_error)?;
        Ok(acp::KillTerminalCommandResponse::new())
    }

    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        if let Some(mut rt) = self.terminals.lock().await.remove(&args.terminal_id) {
            let _ = rt.child.start_kill();
        }
        Ok(acp::ReleaseTerminalResponse::new())
    }
}
```

## 4) Rust client: stream agent text to Tauri or iced with `mpsc`

Use a channel boundary at `session_notification` so UI remains decoupled from ACP transport.

```rust
use tokio::sync::mpsc;

#[derive(Debug, Clone, serde::Serialize)]
enum UiEvent {
    AgentText(String),
    AgentThought(String),
    ToolStatus(String),
}

struct DesktopClient {
    ui_tx: mpsc::UnboundedSender<UiEvent>,
}

#[async_trait::async_trait(?Send)]
impl acp::Client for DesktopClient {
    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        match args.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(text) = chunk.content {
                    let _ = self.ui_tx.send(UiEvent::AgentText(text.text));
                }
            }
            acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                if let acp::ContentBlock::Text(text) = chunk.content {
                    let _ = self.ui_tx.send(UiEvent::AgentThought(text.text));
                }
            }
            acp::SessionUpdate::ToolCallUpdate(update) => {
                let _ = self.ui_tx.send(UiEvent::ToolStatus(format!("{:?}", update.status)));
            }
            _ => {}
        }
        Ok(())
    }
}
```

Tauri bridge pattern:

- Background task receives `UiEvent` from `ui_rx`.
- Emit into frontend: `app_handle.emit("acp://event", payload)`.

iced bridge pattern:

- Convert `ui_rx` into an `iced::Subscription`.
- Map each `UiEvent` to your `Message` enum and update state in `update`.

## Sources

- [Gemini CLI ACP entry and agent implementation (`zedIntegration.ts`)](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/zed-integration/zedIntegration.ts)
- [Gemini ACP filesystem reverse request bridge (`fileSystemService.ts`)](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/zed-integration/fileSystemService.ts)
- [Gemini CLI flag wiring (`config.ts`)](https://github.com/google-gemini/gemini-cli/blob/main/packages/cli/src/config/config.ts)
- [Gemini CLI CLI reference (experimental ACP flag)](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/cli-reference.md)
- [Gemini PR #10089 (TTY freeze fix)](https://github.com/google-gemini/gemini-cli/pull/10089)
- [Gemini PR #10339 (Windows ACP stream parsing fix)](https://github.com/google-gemini/gemini-cli/pull/10339)
- [Gemini issue #7880 (Windows ACP request not read)](https://github.com/google-gemini/gemini-cli/issues/7880)
- [Gemini issue #14893 (API-key ACP quota false failure)](https://github.com/google-gemini/gemini-cli/issues/14893)
- [Gemini PR #18025 (ACP env/auth fixes)](https://github.com/google-gemini/gemini-cli/pull/18025)
- [Gemini issue #12042 (subprocess OAuth cache report)](https://github.com/google-gemini/gemini-cli/issues/12042)
- [Zed external agents docs (Gemini integration details)](https://zed.dev/docs/ai/external-agents)
- [Zed issue #38750 (“Loading…” thread and workarounds)](https://github.com/zed-industries/zed/issues/38750)
- [ACP Rust SDK example client](https://github.com/agentclientprotocol/rust-sdk/blob/main/examples/client.rs)
- [ACP Rust SDK `Client` trait (reverse request surface)](https://github.com/agentclientprotocol/rust-sdk/blob/main/src/agent-client-protocol/src/client.rs)
- [ACP schema crate (`ClientCapabilities`, reverse requests, terminal methods)](https://docs.rs/agent-client-protocol-schema/latest/agent_client_protocol_schema/)
