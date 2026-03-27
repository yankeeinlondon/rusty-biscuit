---
prompt: |-
    Do a deep dive on the ACP implementation that Codex CLI provides.

    - make sure to mention any quirks or gotchas that developers mention facing when interacting with Codex CLI as well any workarounds or ways to avoid issues

    After completing the deep dive, provide the following additional sections which cover writing code examples with Rust:

    1. Show how a Rust client can interact programmatically with the Agent using ACP
    2. Show explicitly how to handle "Reverse Requests" where the Agent asks the client to fulfill a tool request, a file read, etc. (as an Agent is not allowed to do this directly when operating via ACP)
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

## Codex ACP deep dive

### What "Codex ACP" means today

As of **2026-02-22**, Codex is exposed to ACP clients through **`codex-acp`** (the Zed-maintained adapter), not through a standalone `codex ... --acp` mode in the main Codex CLI binary.

The ACP ecosystem page explicitly lists Codex CLI support as:

- Codex CLI (**via Zed's adapter**)

In practice, this means your ACP client talks to `codex-acp` over ACP/JSON-RPC, and `codex-acp` bridges those calls into the Codex runtime.

### High-level architecture

1. ACP client starts `codex-acp` as a stdio subprocess.
2. `codex-acp` creates an `AgentSideConnection` and advertises ACP capabilities.
3. Prompt/session operations are delegated to Codex thread/session machinery.
4. Agent-originated reverse requests (`session/request_permission`, `fs/*`, `terminal/*`) are sent back to the ACP client.
5. Streaming progress is sent to the client via `session/update` notifications.

### Capability surface advertised by `codex-acp`

From the current implementation:

- Protocol version: `v1`
- Prompt capabilities: `embedded_context = true`, `image = true`
- MCP capability: `http = true`
- Session loading: `load_session = true`
- Session listing: enabled via `session_capabilities.list`
- Auth methods:
  - `chatgpt`
  - `codex-api-key`
  - `openai-api-key`

Important auth behavior:

- If `NO_BROWSER` is set, `chatgpt` auth is removed from advertised methods in `initialize`.

### Session lifecycle implemented by Codex ACP

Normal sequence:

1. `initialize`
2. (optional) `authenticate`
3. `session/new` or `session/load`
4. `session/prompt` with streamed `session/update`
5. Optional runtime controls:
   - `session/cancel`
   - `session/set_mode`
   - `session/set_model` (unstable support path)
   - `session/set_config_option`
   - `session/list` (unstable support path, but implemented)

### Reverse requests and where they are used

`codex-acp` uses reverse requests heavily:

- `session/request_permission`
  - command execution approvals
  - patch approvals
  - MCP elicitation prompts
- `fs/read_text_file` and `fs/write_text_file`
  - used when client advertises filesystem capabilities
  - otherwise falls back to local FS behavior
- `terminal/*`
  - driven by command execution/tool flows and terminal reporting

### Event mapping and streaming model

Codex runtime events are mapped into ACP `session/update` notifications:

- text/reasoning deltas -> `agent_message_chunk` / `agent_thought_chunk`
- tool starts -> `tool_call`
- tool state updates/results -> `tool_call_update`
- plan events -> `plan`
- commands availability -> `available_commands_update`
- mode/config changes -> current mode + config option updates

For terminal-like UX, the adapter also emits meta payloads such as `terminal_output` and `terminal_exit` when the client indicates terminal-output support.

## Known quirks and gotchas (with workarounds)

### 1) Parallel command updates previously stuck as `in_progress`

- Symptom: interleaved command runs could miss terminal completion updates.
- Tracking: issue `#154`, fixed by PR `#156`.
- Fix shipped: `v0.9.4` (2026-02-18).
- Workaround: upgrade to `v0.9.4+`.

### 2) Relative path and session-CWD edge cases

- Symptom: path resolution issues when ACP process cwd and session cwd diverge.
- Tracking:
  - relative apply_patch support fixed in PR `#152` (released in `v0.9.3`)
  - broader session-cwd path handling still tracked in open PR `#130`
- Workaround:
  - prefer absolute paths in your client when possible
  - launch adapter with working dir aligned to session cwd/worktree
  - stay on latest release

### 3) Remote/headless ChatGPT auth gap in ACP flows

- Symptom: ChatGPT auth may be unavailable in remote/headless ACP scenarios (`NO_BROWSER` removes it).
- Tracking: open issue `#149`.
- Reality check: Codex docs now include device-code auth (`codex login --device-auth`) for headless usage, but adapter-side behavior is still being tracked.
- Workaround:
  - prefer `CODEX_API_KEY` or `OPENAI_API_KEY` for remote ACP environments
  - if you need ChatGPT auth, run in an environment where browser/device flow is supported

### 4) Linux sandbox failures (`LandlockSandboxExecutableNotProvided`)

- Symptom: commands fail immediately in sandboxed Linux runs.
- Tracking: issue `#93`.
- Fixed by: PR `#91`.
- Fix shipped: `v0.5.1`.
- Workaround: upgrade to `v0.5.1+` (practically, use latest release).

### 5) MCP credential/keychain hash mismatch

- Symptom: MCP OAuth credential lookups mismatch between codex CLI and codex-acp.
- Root cause: differing JSON key ordering in hashed payload generation.
- Tracking: PR `#155`.
- Fix shipped: `v0.9.3`.
- Workaround: upgrade to `v0.9.3+`; if already affected, re-auth MCP credentials after upgrade.

### 6) Read-only approval flow regressions

- Symptom: permission confirmations in read-only mode could appear to hang.
- Tracking: issue `#158`.
- Fix: PR `#159` (post-issue fix on mainline).
- Workaround: use latest release build; if still affected, validate against the latest tagged version and file a reproduction with OS/client details.

## 1) Rust client: basic programmatic ACP interaction with Codex

Below is a minimal ACP client that spawns `codex-acp`, initializes, authenticates if needed, opens a session, and sends prompts.

```rust
use std::sync::Arc;

use agent_client_protocol as acp;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

struct BasicClient;

#[async_trait::async_trait(?Send)]
impl acp::Client for BasicClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        if let acp::SessionUpdate::AgentMessageChunk(chunk) = args.update {
            if let acp::ContentBlock::Text(text) = chunk.content {
                print!("{}", text.text);
            }
        }
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let mut child = tokio::process::Command::new("codex-acp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();

    let local = tokio::task::LocalSet::new();
    local
        .run_until(async move {
            let (conn, io_task) =
                acp::ClientSideConnection::new(Arc::new(BasicClient), outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            tokio::task::spawn_local(io_task);

            let init = conn
                .initialize(
                    acp::InitializeRequest::new(acp::V1)
                        .client_capabilities(acp::ClientCapabilities::new())
                        .client_info(acp::Implementation::new("my-acp-client", "0.1.0")),
                )
                .await?;

            if init
                .auth_methods
                .iter()
                .any(|m| m.id.0.as_ref() == "openai-api-key")
            {
                conn.authenticate(acp::AuthenticateRequest::new("openai-api-key"))
                    .await?;
            }

            let session = conn
                .new_session(acp::NewSessionRequest::new(std::env::current_dir()?))
                .await?;

            let _ = conn
                .prompt(acp::PromptRequest::new(
                    session.session_id.clone(),
                    vec!["Summarize this repository.".into()],
                ))
                .await?;

            Ok::<_, anyhow::Error>(())
        })
        .await?;

    Ok(())
}
```

## 2) Rust client: explicit reverse-request handling (`session/request_permission`, `fs/*`)

This extends the client so the agent can ask for approvals and file I/O through ACP.

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol as acp;

#[derive(Clone)]
struct HostClient {
    workspace_root: Arc<PathBuf>,
}

impl HostClient {
    fn validate_path(&self, path: &Path) -> acp::Result<PathBuf> {
        let abs = std::path::absolute(path)
            .map_err(|e| acp::Error::invalid_params().data(e.to_string()))?;
        let root = std::path::absolute(self.workspace_root.as_ref())
            .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;

        if abs.starts_with(&root) {
            Ok(abs)
        } else {
            Err(acp::Error::invalid_params()
                .data(format!("path outside workspace root: {}", abs.display())))
        }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for HostClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        let preferred = args.options.iter().find(|o| {
            matches!(
                o.kind,
                acp::PermissionOptionKind::AllowOnce | acp::PermissionOptionKind::AllowAlways
            )
        });

        match preferred {
            Some(opt) => Ok(acp::RequestPermissionResponse::new(
                acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                    opt.option_id.clone(),
                )),
            )),
            None => Ok(acp::RequestPermissionResponse::new(
                acp::RequestPermissionOutcome::Cancelled,
            )),
        }
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        let path = self.validate_path(&args.path)?;
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;
        Ok(acp::ReadTextFileResponse::new(content))
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        let path = self.validate_path(&args.path)?;
        tokio::fs::write(path, args.content)
            .await
            .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;
        Ok(acp::WriteTextFileResponse::new())
    }

    async fn session_notification(&self, _args: acp::SessionNotification) -> acp::Result<()> {
        Ok(())
    }
}
```

## 3) Rust client: responding to host command execution requests (`terminal/*`)

This example implements the terminal methods so the ACP agent can execute host commands.

```rust
use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol as acp;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
struct TerminalSnapshot {
    output: String,
    exit: acp::TerminalExitStatus,
}

#[derive(Clone, Default)]
struct TerminalStore {
    by_id: Arc<Mutex<HashMap<acp::TerminalId, TerminalSnapshot>>>,
}

#[async_trait::async_trait(?Send)]
impl acp::Client for TerminalStore {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, _args: acp::SessionNotification) -> acp::Result<()> {
        Ok(())
    }

    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        let mut cmd = tokio::process::Command::new(&args.command);
        cmd.args(args.args);
        if let Some(cwd) = args.cwd {
            cmd.current_dir(cwd);
        }
        for env in args.env {
            cmd.env(env.name, env.value);
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;

        let mut merged = String::from_utf8_lossy(&output.stdout).to_string();
        merged.push_str(&String::from_utf8_lossy(&output.stderr));

        let id = acp::TerminalId::new(Uuid::new_v4().to_string());
        let exit = acp::TerminalExitStatus::new()
            .exit_code(output.status.code().map(|c| c as u32))
            .signal(None::<String>);

        self.by_id
            .lock()
            .await
            .insert(id.clone(), TerminalSnapshot { output: merged, exit });

        Ok(acp::CreateTerminalResponse::new(id))
    }

    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        let map = self.by_id.lock().await;
        let snap = map
            .get(&args.terminal_id)
            .ok_or_else(|| acp::Error::resource_not_found(None))?;

        Ok(acp::TerminalOutputResponse::new(snap.output.clone(), false)
            .exit_status(snap.exit.clone()))
    }

    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        let map = self.by_id.lock().await;
        let snap = map
            .get(&args.terminal_id)
            .ok_or_else(|| acp::Error::resource_not_found(None))?;
        Ok(acp::WaitForTerminalExitResponse::new(snap.exit.clone()))
    }

    async fn kill_terminal_command(
        &self,
        args: acp::KillTerminalCommandRequest,
    ) -> acp::Result<acp::KillTerminalCommandResponse> {
        // In this snapshot-style implementation, commands are already finished.
        if self.by_id.lock().await.contains_key(&args.terminal_id) {
            Ok(acp::KillTerminalCommandResponse::new())
        } else {
            Err(acp::Error::resource_not_found(None))
        }
    }

    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        self.by_id.lock().await.remove(&args.terminal_id);
        Ok(acp::ReleaseTerminalResponse::new())
    }
}
```

For production use, replace this snapshot approach with long-lived child processes and incremental streaming.

## 4) Rust client: streaming agent text to Tauri/iced with `mpsc`

Pattern:

1. In `session_notification`, map ACP updates to UI events.
2. Send those events to an `mpsc` channel.
3. In your desktop runtime task, read channel events and emit framework messages.

```rust
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
enum UiEvent {
    AgentText(String),
    AgentThought(String),
    ToolTitle(String),
}

#[derive(Clone)]
struct UiBridge {
    ui_tx: mpsc::UnboundedSender<UiEvent>,
}

#[async_trait::async_trait(?Send)]
impl acp::Client for UiBridge {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        match args.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(t) = chunk.content {
                    let _ = self.ui_tx.send(UiEvent::AgentText(t.text));
                }
            }
            acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                if let acp::ContentBlock::Text(t) = chunk.content {
                    let _ = self.ui_tx.send(UiEvent::AgentThought(t.text));
                }
            }
            acp::SessionUpdate::ToolCall(tool_call) => {
                let _ = self.ui_tx.send(UiEvent::ToolTitle(tool_call.title));
            }
            _ => {}
        }
        Ok(())
    }
}
```

Framework bridging:

- Tauri: consume `ui_rx` in a background task and call `app_handle.emit("acp://event", payload)`.
- iced: convert `ui_rx` into a `Subscription`/stream and map events into your `Message` enum.

This keeps ACP I/O isolated from UI-thread rendering and avoids blocking your desktop event loop.

## Sources

- ACP agents page (Codex via adapter): <https://agentclientprotocol.com/overview/agents>
- Codex ACP adapter README: <https://github.com/zed-industries/codex-acp>
- Codex ACP adapter source (`initialize`, auth, sessions): <https://github.com/zed-industries/codex-acp/blob/main/src/codex_agent.rs>
- Codex ACP adapter source (reverse requests, tool/terminal mapping): <https://github.com/zed-industries/codex-acp/blob/main/src/thread.rs>
- Codex ACP adapter source (fs bridging/sandboxing): <https://github.com/zed-industries/codex-acp/blob/main/src/local_spawner.rs>
- Codex ACP releases: <https://github.com/zed-industries/codex-acp/releases>
- Parallel terminal update bug + fix: <https://github.com/zed-industries/codex-acp/issues/154>, <https://github.com/zed-industries/codex-acp/pull/156>
- Relative path/session cwd path issues: <https://github.com/zed-industries/codex-acp/pull/152>, <https://github.com/zed-industries/codex-acp/pull/130>
- Remote auth issue: <https://github.com/zed-industries/codex-acp/issues/149>
- Codex headless auth docs: <https://developers.openai.com/codex/auth/#login-on-headless-devices>
- Linux sandbox fix: <https://github.com/zed-industries/codex-acp/issues/93>, <https://github.com/zed-industries/codex-acp/pull/91>
- MCP key ordering fix: <https://github.com/zed-industries/codex-acp/pull/155>
- Read-only approval issue + fix: <https://github.com/zed-industries/codex-acp/issues/158>, <https://github.com/zed-industries/codex-acp/pull/159>
- ACP Rust SDK client example: <https://github.com/agentclientprotocol/rust-sdk/blob/main/examples/client.rs>
