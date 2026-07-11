---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
docs: https://pi.dev/
acp_docs: https://agentclientprotocol.com/get-started/agents
repo: https://github.com/earendil-works/pi
support: adapter
launch_modes:
  - command: pi-acp
    args: []
    transport: stdio
    adapter: pi-acp
    notes: "Global adapter launch. The public ACP registry points to svkozak/pi-acp; @victor-software-house/pi-acp also exposes the same bin name."
  - command: npx
    args: ["-y", "@victor-software-house/pi-acp"]
    transport: stdio
    adapter: "@victor-software-house/pi-acp"
    notes: "No global install path documented by the active fork."
  - command: node
    args: ["/path/to/pi-acp/dist/index.js"]
    transport: stdio
    adapter: pi-acp
    notes: "Source-build launch for the Node-based svkozak adapter."
  - command: pi-acp
    args: ["--terminal-login"]
    transport: other
    adapter: pi-acp
    notes: "Out-of-band terminal-auth setup path, not the ACP JSON-RPC session transport."
protocol_versions:
  - "ACP protocolVersion 1"
  - "svkozak/pi-acp 0.0.31 depends on @agentclientprotocol/sdk ^0.26.0"
  - "@victor-software-house/pi-acp 0.17.1 depends on @agentclientprotocol/sdk ^0.22.1 and documents ACP v0.13.x schema alignment"
capabilities:
  - capability: initialize
    support: supported
    notes: "Both inspected adapters implement initialize and negotiate protocolVersion 1."
  - capability: authenticate
    support: partial
    notes: "Terminal Auth is advertised; authenticate itself is effectively a no-op because login happens by launching pi-acp --terminal-login."
  - capability: session_new
    support: supported
    notes: "Creates a Pi AgentSession or Pi RPC-backed session."
  - capability: session_load
    support: supported
    notes: "Supported through Pi session persistence and adapter session mapping or replay."
  - capability: session_prompt
    support: supported
    notes: "Prompts are translated into Pi prompts and stream back ACP session/update notifications."
  - capability: session_cancel
    support: supported
    notes: "The Victor fork documents session/cancel as AgentSession.abort(); the registry adapter also handles abort/cancel behavior through Pi."
  - capability: session_modes
    support: supported
    notes: "Victor maps thinking level and model changes through modes/configOptions; svkozak exposes model/thinking controls with more limited compatibility."
  - capability: streaming
    support: supported
    notes: "Streams message chunks, thought chunks in the Victor fork, tool calls, tool updates, usage, command updates, and config/mode updates."
  - capability: permissions
    support: unsupported
    notes: "Pi does not request ACP session/request_permission before tool execution."
  - capability: fs_read
    support: partial
    notes: "Victor routes Pi read through ACP fs/read_text_file when clientCapabilities.fs.readTextFile is true. svkozak does not delegate fs/*."
  - capability: fs_write
    support: unsupported
    notes: "Adapters do not advertise fs/write_text_file; Pi writes locally."
  - capability: terminal
    support: partial
    notes: "Victor delegates bash through ACP terminal/* when clientCapabilities.terminal is true; svkozak emits display-oriented terminal metadata but does not delegate terminal execution."
  - capability: mcp
    support: partial
    notes: "MCP server params are accepted/stored, but not wired through to Pi sessions."
  - capability: media
    support: partial
    notes: "Image prompts are advertised; audio is not. Embedded context is always advertised by Victor and environment-gated in svkozak."
  - capability: plans
    support: unsupported
    notes: "Pi has no native plan/TODO surface, so adapters do not emit agent_plan updates."
  - capability: extensions
    support: partial
    notes: "Victor supports pi-acp extMethod/extNotification namespace and Pi extension commands; svkozak documents that Pi extension slash commands are not currently supported."
reverse_requests:
  - method: fs/read_text_file
    purpose: fs_read
    client_must_handle: false
    notes: "Victor only sends this when the client advertises fs.readTextFile. Required for remote-workspace read delegation, not for basic local use."
  - method: terminal/create
    purpose: terminal_create
    client_must_handle: false
    notes: "Victor only sends terminal requests when clientCapabilities.terminal is true."
  - method: terminal/output
    purpose: terminal_output
    client_must_handle: false
    notes: "Used through the SDK terminal handle/currentOutput path for delegated bash output polling."
  - method: terminal/wait_for_exit
    purpose: terminal_wait
    client_must_handle: false
    notes: "Used by Victor delegated bash to wait for command completion."
  - method: terminal/kill
    purpose: terminal_kill
    client_must_handle: false
    notes: "Used for aborts and timeouts in delegated bash."
  - method: terminal/release
    purpose: terminal_kill
    client_must_handle: false
    notes: "Used as best-effort cleanup after delegated bash exits, aborts, or times out."
  - method: session/request_permission
    purpose: permission
    client_must_handle: false
    notes: "Not emitted by inspected adapters."
permission_model:
  mechanism: "No ACP permission prompt bridge"
  timeout: "unknown"
  default_policy: "Pi tools run with the authority of the adapter or with the authority of the ACP client when Victor terminal delegation is enabled."
  approval_values: []
  notes: "A Claudine host must enforce policy before launch and inside optional fs/terminal reverse-request handlers; the adapter will not ask for approval."
filesystem_model:
  read_methods: ["local Pi read", "fs/read_text_file when Victor adapter sees clientCapabilities.fs.readTextFile"]
  write_methods: ["local Pi edit/write only"]
  path_base: "ACP session cwd must be absolute; Pi tool paths are resolved to absolute paths before delegated reads. Tool locations may include 1-based line numbers when inferable."
  sandboxing: "No built-in Pi sandbox. The client/host must sandbox the adapter process and decide whether to advertise fs.readTextFile."
  notes: "ACP read delegation is UTF-8 text only. The installed local Pi config directory was /Users/ken/.claudine/.pi/agent via PI_CODING_AGENT_DIR-style relocation."
terminal_model:
  supported: true
  methods: ["terminal/create", "terminal/output", "terminal/wait_for_exit", "terminal/kill", "terminal/release"]
  shell: "/bin/sh -c in the Victor adapter"
  cwd: "ACP session cwd or adapter-selected temporary cwd"
  streaming: "Victor polls terminal output snapshots and emits deltas into Pi's onData callback; svkozak can emit terminal-like metadata for display but does not delegate execution."
  cancellation: "Victor calls terminal.kill() on abort or timeout and releases the handle in a finally block."
  notes: "The hard-coded /bin/sh command is a portability concern for native Windows clients unless the ACP client translates or provides a POSIX shell."
streaming_model:
  update_methods: ["session/update"]
  text_events: ["agent_message_chunk", "agent_thought_chunk", "user_message_chunk"]
  tool_events: ["tool_call", "tool_call_update", "usage_update", "available_commands_update", "config_option_update", "current_mode_update", "session_info_update"]
  plan_events: []
  error_events: ["authRequired errors surfaced from auth/config failures", "invalidParams for invalid sessions"]
  notes: "Route text and thought chunks to transcript streams, tool_call/update to tool panels, usage_update to counters, command/config/mode updates to UI state stores."
auth_setup:
  required: true
  mechanisms: ["provider API key environment variables", "Pi /login OAuth or API-key storage", "pi-acp Terminal Auth"]
  headless_notes: "Headless ACP launch requires Pi to have at least one usable configured model. The local auth.json inspected on this host was empty, so this machine's Pi install was not ready for authenticated headless use without env vars or login."
  notes: "Terminal Auth advertises pi-acp --terminal-login so clients such as Zed can show an Authenticate action."
env_vars:
  - name: PI_ACP_ENABLE_EMBEDDED_CONTEXT
    effect: "svkozak adapter advertises promptCapabilities.embeddedContext only when set to true."
  - name: PI_ACP_DAEMON_DEBUG
    effect: "Victor adapter writes manifest/resource diagnostics to stderr when set to 1."
  - name: PI_ACP_SOCKET_DIR
    effect: "Victor daemon draft ADR uses it to relocate POSIX socket files."
  - name: PI_CODING_AGENT_DIR
    effect: "Overrides Pi agent config directory."
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: "Overrides Pi session storage directory unless a CLI --session-dir is passed."
  - name: PI_OFFLINE
    effect: "Disables Pi startup network operations."
  - name: PI_SKIP_VERSION_CHECK
    effect: "Disables Pi latest-version check."
  - name: PI_TELEMETRY
    effect: "Controls Pi install/update telemetry and provider attribution headers."
  - name: ANTHROPIC_API_KEY
    effect: "One of the documented provider API key variables Pi can use for headless auth."
  - name: OPENAI_API_KEY
    effect: "One of the documented provider API key variables Pi can use for headless auth."
  - name: GEMINI_API_KEY
    effect: "One of the documented provider API key variables Pi can use for headless auth."
rust_client:
  crate: agent-client-protocol
  connection_type: "ClientSideConnection over a spawned pi-acp stdio process"
  localset_required: false
  reverse_request_handlers: ["fs/read_text_file", "terminal/create", "terminal/output", "terminal/wait_for_exit", "terminal/kill", "terminal/release"]
  desktop_streaming_pattern: "Spawn pi-acp, drive ClientSideConnection, forward session/update notifications through tokio::sync::mpsc to the UI runtime."
  notes: "Use the official Rust crate for ACP framing/types. Do not speak Pi RPC directly unless deliberately bypassing ACP."
compatibility:
  - client: Zed
    status: works
    issue: "Pi is listed in the ACP agent registry via the pi-acp adapter."
    workaround: "Use the registry entry or configure a custom command pointing at pi-acp."
  - client: Zed Remote
    status: partial
    issue: "Remote-correct filesystem and terminal behavior depends on advertising fs.readTextFile and terminal capabilities to the Victor fork."
    workaround: "Use @victor-software-house/pi-acp and implement/advertise ACP fs and terminal client handlers."
  - client: Zed with ask-user-style Pi skills
    status: partial
    issue: "agentclientprotocol discussion #976 reports Pi ACP could not complete an ask-user skill because the user could not provide elicited input through Zed."
    workaround: "Avoid interactive Pi skills in ACP sessions until ACP/client elicitation support exists."
  - client: Non-Zed ACP clients
    status: unknown
    issue: "Both adapter READMEs focus development and compatibility on Zed."
    workaround: "Test initialize/session lifecycle and feature-gate fs, terminal, auth metadata, and extensions."
recent_changes:
  - date: "2026-05-19"
    version: "@victor-software-house/pi-acp 0.17.1"
    change: "Latest npm release inspected; active fork includes SDK embedding, session/config support, fs read delegation, terminal delegation, provider config, logout, and extension namespace."
    impact: "Most complete Pi ACP story, but published package requires Bun >=1.3 and Pi >=0.75.3."
  - date: "2026-06-17"
    version: "svkozak/pi-acp repository commit 49d6ec8"
    change: "Public registry adapter repository was updated after its npm 0.0.31 package line."
    impact: "Still the ACP registry target, but less capable than the Victor fork for reverse fs/terminal delegation."
  - date: "2026-06-30"
    version: "@earendil-works/pi-coding-agent 0.80.3"
    change: "Latest Pi coding-agent package on npm at research time."
    impact: "Primary Pi CLI remains separate from ACP; adapter compatibility should be checked against current Pi SDK."
  - date: "2026-07-03"
    version: "earendil-works/pi commit 23d1462"
    change: "Local clone of upstream Pi showed current project state; source search found Pi RPC mode but no native ACP mode in the primary CLI."
    impact: "Classification remains adapter."
quirks:
  - "The local installed pi binary was @mariozechner/pi-coding-agent 0.73.1, not the current @earendil-works package; it advertised no ACP command or flag."
  - "Running pi --mode acp locally produced no ACP handshake or documented ACP mode; the parser in current source accepts only text, json, or rpc modes."
  - "Victor's README/source and its acp-conformance.md drift: the table says fs/terminal delegation is not implemented, while README and source implement capability-gated read and terminal delegation. Treat source as authoritative."
  - "Victor terminal delegation hard-codes /bin/sh -c, which is not cross-platform for native Windows."
  - "MCP params are accepted but not connected to Pi, which is a MUST-level compliance gap in the Victor README."
  - "No permission reverse request exists, so host-side safety cannot rely on ACP session/request_permission."
gaps:
  - "No live initialize handshake was run against pi-acp because no pi-acp binary was installed locally and installing new global tools was outside this research task."
  - "Exact ACP wire method spellings for SDK terminal helper methods should be confirmed against the crate/SDK version Claudine adopts."
  - "Compatibility outside Zed is not well documented."
  - "The Victor daemon/runtime design is partly draft and POSIX-oriented; Windows support needs separate validation."
changes: []
requires_claudine_update: true
reason: "Adding Pi over ACP would require a new adapter launch target, capability negotiation, optional fs/terminal reverse-request handlers, streaming session/update routing, and auth preflight checks because Pi has no native ACP mode."
---

# Pi ACP Support

## Overview

Pi's ACP support is adapter-based. The primary Pi coding-agent CLI is not a native ACP server: the installed local `pi` binary reported version `0.73.1`, its help did not advertise an ACP command or flag, and the current `earendil-works/pi` source documents `text`, `json`, and `rpc` modes rather than an ACP mode. The Pi site and repository describe Pi as a minimal agent harness with an SDK and Pi-specific RPC mode; ACP is supplied by bridge processes.

The public ACP agents page lists Pi as available via the `pi-acp` adapter, and Zed's Pi agent page classifies it as an ACP adapter bridge using JSON-RPC 2.0 over stdio. The canonical registry/documentation link currently points to [`svkozak/pi-acp`](https://github.com/svkozak/pi-acp). That adapter bridges ACP JSON-RPC to Pi's own process integration surface and stores adapter session mappings.

There is also an active fork, [`victor-software-house/pi-acp`](https://github.com/victor-software-house/pi-acp), published as `@victor-software-house/pi-acp`. It embeds Pi through the `@earendil-works/pi-coding-agent` SDK and exposes an ACP agent over stdio. It is more featureful than the registry adapter: it supports thought streaming, richer session/config operations, optional ACP filesystem read delegation, optional ACP terminal delegation, provider control, logout, and an extension namespace.

The adapter-vs-native distinction is the important integration fact: Claudine should launch an adapter, not `pi`, when it wants ACP. Launching `pi --mode rpc` gives a Pi-specific JSONL protocol, not ACP.

## Launching ACP

The ACP process is the adapter:

```bash
pi-acp
```

or, for the Victor fork without a global install:

```bash
npx -y @victor-software-house/pi-acp
```

or, from a source checkout of the Node-based adapter:

```bash
node /path/to/pi-acp/dist/index.js
```

The transport is newline-delimited JSON-RPC over stdio. The adapter must keep stdout reserved for ACP frames; logs belong on stderr.

This is distinct from launching Pi itself:

```bash
pi --mode rpc
```

Pi RPC mode is useful for custom integrations, but it is a proprietary Pi JSONL protocol. A Claudine ACP client should not speak it unless Claudine is deliberately implementing its own Pi adapter.

Authentication setup uses a separate terminal-auth launch:

```bash
pi-acp --terminal-login
```

That path starts Pi in an interactive terminal so a user can run provider login or configure API keys. It is not the stdio JSON-RPC agent session.

## Protocol and Capabilities

Both inspected adapters negotiate ACP `protocolVersion: 1`. The svkozak adapter source returns `1` when the requested version is not exactly `1`; its package depends on `@agentclientprotocol/sdk ^0.26.0`. The Victor adapter does the same protocol negotiation, depends on `@agentclientprotocol/sdk ^0.22.1`, and its PRD records ACP v0.13.x schema alignment.

Capability summary:

| Area | Support | Notes |
|---|---|---|
| Initialize | Supported | Returns agent info, auth methods, and capabilities. |
| Authentication | Partial | Terminal Auth is advertised; `authenticate` is effectively out-of-band/no-op. |
| New session | Supported | Creates a Pi session through Pi RPC or SDK. |
| Load/list/resume/close/fork session | Partial to supported | Victor supports list, load, close, resume, and preview fork; svkozak supports load/list with fewer lifecycle surfaces. |
| Prompt | Supported | ACP prompt content is translated to Pi input. |
| Cancel | Supported | Victor maps cancel to `AgentSession.abort()`. |
| Modes/config | Supported in Victor | Model and thinking level are exposed through config options and backward-compatible mode/model surfaces. |
| Streaming | Supported | Message chunks, tool calls, tool updates, usage, command updates, and config/mode updates. Victor also streams thoughts. |
| Permissions | Unsupported | No `session/request_permission` bridge. |
| Filesystem read | Partial | Victor can route reads through ACP `fs/read_text_file` when the client advertises it. |
| Filesystem write | Unsupported | Pi edit/write operate locally; `fs/write_text_file` is not advertised. |
| Terminal | Partial | Victor can route bash through ACP terminal methods when the client advertises terminal support. |
| MCP | Partial/non-compliant | MCP server params are accepted but not wired through to Pi. |
| Media | Partial | Images are supported; audio is not. Embedded context is always advertised by Victor and env-gated in svkozak. |
| Plans | Unsupported | Pi has no plan/TODO abstraction to translate to ACP plan updates. |
| Extensions | Partial | Victor exposes `pi-acp/` extension methods and Pi extension commands; svkozak does not support extension slash commands. |

## Reverse Requests

Basic local use does not require a Claudine client to implement reverse requests beyond receiving `session/update` notifications. The registry adapter does not delegate ACP filesystem or terminal operations.

The Victor fork can send reverse requests if the client advertises capabilities:

```json
{
  "method": "fs/read_text_file",
  "params": {
    "sessionId": "session-id",
    "path": "/absolute/path/to/file.rs"
  }
}
```

The adapter uses this for Pi's `read` tool when `clientCapabilities.fs.readTextFile` is true. This is optional for basic use but important for remote-editor correctness, because it lets the editor decide whether the authoritative file is local, remote, buffered, or sandboxed.

For terminal delegation, Victor creates a terminal for Pi's `bash` tool, polls output, waits for exit, kills on timeout/abort, and releases the handle:

```json
{
  "method": "terminal/create",
  "params": {
    "sessionId": "session-id",
    "command": "/bin/sh",
    "args": ["-c", "cargo check"],
    "cwd": "/absolute/workspace",
    "env": []
  }
}
```

The related client-side operations are `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, and `terminal/release` through the SDK terminal handle. These are capability-gated; do not advertise terminal support unless Claudine is ready to enforce command policy and lifecycle cleanup.

The adapters do not emit `session/request_permission`. If Claudine needs user or policy approval, it must happen before the adapter launches, before advertising reverse capabilities, or inside Claudine's own reverse-request handlers.

## Permissions, Filesystem, and Terminal

Pi does not include a built-in ACP permission model. Its own docs say it runs with the permissions of the user and process that launched it unless the user containerizes or sandboxes it. Pi also intentionally does not have built-in permission popups; its philosophy is to let extensions or external containers implement that policy.

For Claudine, that means:

- If Claudine launches `pi-acp` without fs/terminal capabilities, Pi reads, writes, edits, and runs bash in the adapter process environment.
- If Claudine advertises `fs.readTextFile` to the Victor adapter, Claudine becomes responsible for resolving absolute paths, checking sandbox policy, choosing whether unsaved editor buffers win over disk, and returning UTF-8 text.
- If Claudine advertises terminal support, Claudine becomes responsible for command approval, cwd validation, environment filtering, process start, output streaming, timeout/cancel handling, kill, and release.

Path conventions are mostly absolute. Victor requires or synthesizes an ACP session cwd and passes absolute paths into the read operation. The svkozak README says relative Pi tool locations are resolved against session cwd before being emitted, and edit locations use inferred 1-based line numbers when a unique `oldText` match exists.

One portability issue is explicit: Victor's terminal bridge launches `/bin/sh -c <command>`. That is suitable for Unix-like environments and many remote Linux workspaces, but it is not a native Windows command path. A cross-platform Claudine terminal handler should either avoid advertising terminal support on Windows for this adapter, provide a compatible shell, or confirm the adapter has gained platform-specific command selection.

## Streaming and UI Integration

Adapters emit ACP `session/update` notifications. The important update types are:

| Update | UI routing |
|---|---|
| `agent_message_chunk` | Assistant transcript stream. |
| `agent_thought_chunk` | Thought/reasoning lane; Victor supports this live and during replay. |
| `tool_call` | Create or open a tool panel with title, kind, raw input, and optional locations. |
| `tool_call_update` | Append tool progress, terminal-style output, result text, status, and structured diffs. |
| `usage_update` | Update token/cost/context counters. |
| `available_commands_update` | Refresh slash-command palette. |
| `config_option_update` | Refresh model/thinking selectors. |
| `current_mode_update` | Refresh current thinking/mode display. |
| `session_info_update` | Refresh session name/status. |
| `user_message_chunk` | Replay only; reconstruct historical transcript. |

The UI event loop should treat ACP notifications as ordered events per session. Text chunks can append to open transcript nodes. Tool events should be keyed by `toolCallId` so updates can mutate the same visual record. Diff content belongs in file-change views, not in the assistant text stream. Usage/config/mode updates should go to application state stores.

## Authentication and Setup

Pi must have usable provider credentials before headless ACP use. Setup options are:

- API key environment variables such as `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, and provider-specific variables documented by Pi.
- Pi `/login` in interactive mode, storing OAuth or API-key credentials in `~/.pi/agent/auth.json`.
- `pi-acp --terminal-login`, advertised through ACP Terminal Auth so clients can show an authenticate action.
- Custom providers/models in `~/.pi/agent/models.json`, with keys resolved from literals, environment references, stored auth, or command outputs depending on configuration.

Local evidence: this host has a `pi` binary at `/Users/ken/.bun/bin/pi`, version `0.73.1`, linked to `@mariozechner/pi-coding-agent`. The local Pi agent state under `/Users/ken/.claudine/.pi/agent` contained `auth.json` with `{}`, so no stored provider credentials were present there. That does not prove the user has no credentials elsewhere; it only means this inspected Pi config root was not ready for authenticated headless sessions without environment variables or login.

Environment variables relevant to ACP launch and setup:

| Variable | Effect |
|---|---|
| `PI_ACP_ENABLE_EMBEDDED_CONTEXT` | svkozak adapter advertises embedded context only when `true`. |
| `PI_ACP_DAEMON_DEBUG` | Victor adapter emits resource/manifest diagnostics to stderr when `1`. |
| `PI_ACP_SOCKET_DIR` | Victor daemon ADR uses this for POSIX socket placement. |
| `PI_CODING_AGENT_DIR` | Overrides Pi config directory. |
| `PI_CODING_AGENT_SESSION_DIR` | Overrides Pi session directory unless `--session-dir` wins. |
| `PI_OFFLINE` | Disables startup network operations. |
| `PI_SKIP_VERSION_CHECK` | Disables version update check. |
| `PI_TELEMETRY` | Controls install/update telemetry and provider attribution headers. |
| Provider API key vars | Feed provider auth for headless model availability. |

## Compatibility, Quirks, and Workarounds

Zed is the primary target. Zed's ACP registry lists Pi through `pi-acp`, and both adapter READMEs center compatibility on Zed. Other ACP clients may work, but should be tested for auth metadata, command updates, config options, terminal metadata, fs reverse requests, and extension behavior.

The active Victor fork and its docs contain a drift: `README.md` and source implement `fs/read_text_file` and terminal delegation, but `docs/architecture/acp-conformance.md` still lists fs/terminal delegation as not implemented. I treated source and README as authoritative and recorded the conformance table as stale.

MCP is a significant gap. The adapter may accept `mcpServers` in `session/new` or `session/load`, but the Victor README states they are not wired through to Pi. An ACP client should not assume configured MCP servers are available inside Pi sessions.

Interactive/elicitation-style Pi skills are risky in ACP. An Agent Client Protocol discussion from April 14, 2026 reports that a user could not complete a Pi ACP ask-user skill in Zed because there was no way to send the requested input. Until ACP/client elicitation is available and tested, Claudine should avoid relying on interactive Pi skills in ACP mode.

Native Windows needs extra caution. Pi itself targets cross-platform Node runtimes, but the Victor ACP terminal bridge currently uses `/bin/sh`. Claudine should not advertise ACP terminal capability to that adapter on Windows unless a compatible shell path is guaranteed or the adapter changes.

## Recent Changes

- 2026-05-12: An `earendil-works/pi` discussion proposed ACP support and noted that a community `pi-acp` adapter already worked with Zed, while Pi's own RPC mode remained Pi-specific.
- 2026-05-19: `@victor-software-house/pi-acp` published `0.17.1`, the latest npm release observed. Its package depends on `@agentclientprotocol/sdk ^0.22.1` and `@earendil-works/pi-coding-agent ^0.75.3`.
- 2026-06-17: `svkozak/pi-acp` latest cloned commit was `49d6ec8` with package version `0.0.31`; this remains the adapter linked from the ACP agents page and Zed Pi page.
- 2026-06-30: `@earendil-works/pi-coding-agent` latest npm version observed was `0.80.3`.
- 2026-07-03: Local clone of `earendil-works/pi` at commit `23d1462` showed current Pi source still has Pi RPC mode, not native ACP mode.

## Rust Client Example

Use the official `agent-client-protocol` Rust crate when Claudine is the ACP client. The adapter is just a child process over stdio:

```rust
use std::process::Stdio;
use tokio::process::Command;

async fn spawn_pi_acp() -> anyhow::Result<()> {
    let mut child = Command::new("pi-acp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().expect("pi-acp stdin");
    let stdout = child.stdout.take().expect("pi-acp stdout");

    // Wire stdout/stdin into agent_client_protocol::ClientSideConnection.
    // Implement the Client trait for reverse requests and session/update routing.
    drop((stdin, stdout));
    Ok(())
}
```

For Pi, do not use a lower-level JSON-RPC implementation unless the official crate is missing a method needed by the selected adapter version. The standard crate gives Claudine typed initialize/session methods and typed reverse-request handlers.

## Rust Reverse Request Handling

The client implementation should feature-gate reverse handlers based on what Claudine actually wants to expose:

```rust
struct ClaudineAcpClient {
    policy: HostPolicy,
}

impl ClaudineAcpClient {
    async fn read_text_file(&self, session_id: String, path: String) -> anyhow::Result<String> {
        let path = std::path::PathBuf::from(path);
        self.policy.check_read(&session_id, &path)?;
        let text = tokio::fs::read_to_string(path).await?;
        Ok(text)
    }
}
```

For Pi, `fs/read_text_file` is optional and only needed for Victor read delegation. If Claudine does not want to provide editor-buffer or remote-filesystem reads, do not advertise `clientCapabilities.fs.readTextFile`.

Permission requests are currently not emitted by the Pi adapters. Claudine should still implement a generic `session/request_permission` handler for future ACP agents, but Pi ACP usability does not depend on it today.

## Rust Host Command Handling

Only advertise terminal support when Claudine is ready to run commands under its own policy:

```rust
async fn create_terminal(req: CreateTerminalRequest, policy: &HostPolicy) -> anyhow::Result<TerminalId> {
    policy.check_command(&req.command, &req.args, &req.cwd)?;

    let mut cmd = tokio::process::Command::new(&req.command);
    cmd.args(&req.args)
        .current_dir(&req.cwd)
        .env_clear()
        .envs(policy.filtered_env(req.env));

    let child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    Ok(register_child(child).await)
}
```

For Victor Pi ACP, expect `/bin/sh -c <command>` today. On Windows, either withhold `clientCapabilities.terminal`, provide a POSIX-compatible shell environment, or add adapter-specific launch detection before enabling delegated bash.

The terminal lifecycle must support output snapshots or equivalent output reads, wait-for-exit, kill, and release. Release should be idempotent because adapters may call it after timeout or abort paths.

## Rust Desktop Streaming Bridge

Use an internal event enum and an `mpsc` channel to decouple ACP IO from desktop rendering:

```rust
enum PiAcpUiEvent {
    Text { session_id: String, delta: String },
    Thought { session_id: String, delta: String },
    ToolStarted { session_id: String, tool_call_id: String, title: String },
    ToolUpdated { session_id: String, tool_call_id: String, status: String },
    Usage { session_id: String },
}

async fn handle_session_update(
    tx: tokio::sync::mpsc::Sender<PiAcpUiEvent>,
    update: SessionUpdate,
) -> anyhow::Result<()> {
    match update.kind() {
        "agent_message_chunk" => tx.send(PiAcpUiEvent::Text {
            session_id: update.session_id().to_owned(),
            delta: update.text().unwrap_or_default().to_owned(),
        }).await?,
        "agent_thought_chunk" => tx.send(PiAcpUiEvent::Thought {
            session_id: update.session_id().to_owned(),
            delta: update.text().unwrap_or_default().to_owned(),
        }).await?,
        _ => {}
    }
    Ok(())
}
```

In Tauri, receive on a Tokio task and emit window events on the app handle. In iced, bridge into a `Subscription` or command channel. Keep ACP parsing and UI mutation on separate tasks so slow rendering cannot block the JSON-RPC reader.

## Claudine Integration Notes

Adding Pi ACP support to Claudine would require adapter detection rather than native Pi detection. The launch target should prefer the ACP registry adapter when matching Zed's ecosystem, but Claudine may want an explicit option for the Victor fork because it exposes richer reverse-request behavior.

Required pieces:

- Launch config for `pi-acp` and `npx -y @victor-software-house/pi-acp`.
- Auth preflight that checks whether Pi has a usable model or can show Terminal Auth.
- Initialize negotiation that records protocol version, prompt capabilities, session capabilities, MCP gap, fs read support, terminal support, and thought streaming.
- Streaming bridge from `session/update` to Claudine's normalized UI/event model.
- Reverse-request router for `fs/read_text_file` and terminal lifecycle methods, but only when Claudine intentionally advertises those capabilities.
- Host policy enforcement before returning file contents or running commands.
- Platform guard for Victor terminal delegation on Windows because of `/bin/sh`.
- MCP expectation management: do not claim Pi ACP sessions receive client MCP servers until the adapter and Pi SDK wire that path.

Claudine should treat Pi ACP as useful today for transcript/tool streaming and editor launch, but not as a complete host-side security protocol. Host policy remains Claudine's job.

## Changelog

Initial research document.

## Sources

- [Pi website](https://pi.dev/)
- [earendil-works/pi repository](https://github.com/earendil-works/pi)
- [Pi coding-agent README, local clone `/tmp/pi-earendil/packages/coding-agent/README.md`](https://github.com/earendil-works/pi/tree/main/packages/coding-agent)
- [Pi RPC mode docs, local clone `/tmp/pi-earendil/packages/coding-agent/docs/rpc.md`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/rpc.md)
- [Pi providers docs, local clone `/tmp/pi-earendil/packages/coding-agent/docs/providers.md`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/providers.md)
- [Agent Client Protocol agents page](https://agentclientprotocol.com/get-started/agents)
- [Agent Client Protocol architecture](https://agentclientprotocol.com/get-started/architecture)
- [Agent Client Protocol Rust library page](https://agentclientprotocol.com/libraries/rust)
- [Zed Pi ACP agent page](https://zed.dev/acp/agent/pi)
- [svkozak/pi-acp repository](https://github.com/svkozak/pi-acp)
- [victor-software-house/pi-acp repository](https://github.com/victor-software-house/pi-acp)
- [earendil-works/pi discussion #4444: Supporting the Agent Client Protocol](https://github.com/earendil-works/pi/discussions/4444)
- [earendil-works/pi issue #175: ACP Support](https://github.com/earendil-works/pi/issues/175)
- [agentclientprotocol discussion #976: Please support Elicitation](https://github.com/orgs/agentclientprotocol/discussions/976)
- Local installed Pi inspection: `/Users/ken/.bun/bin/pi`, package `@mariozechner/pi-coding-agent` version `0.73.1`, and `/Users/ken/.claudine/.pi/agent/auth.json`.
- Local adapter source inspection: `/tmp/pi-acp-svkozak` at commit `49d6ec8` and `/tmp/pi-acp-victor` at commit `0ef24b2`.
