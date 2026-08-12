---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
docs: https://kilo.ai/docs/code-with-ai/platforms/cli
acp_docs: https://zed.dev/acp/agent/kilo
repo: https://github.com/Kilo-Org/kilocode
support: native
launch_modes:
  - command: kilo
    args:
      - acp
      - --cwd
      - /path/to/project
    transport: stdio
    adapter:
    notes: "Primary CLI command. Uses newline-delimited JSON-RPC over stdin/stdout via @agentclientprotocol/sdk ndJsonStream, while also starting Kilo's local HTTP server internally for its own SDK calls."
  - command: npx
    args:
      - '@kilocode/cli@7.3.54'
      - acp
    transport: stdio
    adapter:
    notes: "Zed ACP registry manual launch form. Uses the npm CLI package rather than a separate adapter."
protocol_versions:
  - "ACP protocolVersion 1"
  - "@agentclientprotocol/sdk 0.21.0 in Kilo v7.4.1 source"
capabilities:
  - capability: initialize
    support: supported
    notes: "Observed initialize result returns protocolVersion 1, agentInfo Kilo, auth methods, and capabilities."
  - capability: authenticate
    support: supported
    notes: "authenticate accepts methodId kilo-login; the method is effectively a readiness check because login is performed out of band."
  - capability: session_new
    support: supported
    notes: "Creates a Kilo session for the requested cwd and registers supplied MCP servers."
  - capability: session_load
    support: supported
    notes: "loadSession, list, resume, close, and fork are advertised through sessionCapabilities."
  - capability: session_prompt
    support: supported
    notes: "Prompt content supports text, embedded context/resource links, and image/file inputs."
  - capability: session_cancel
    support: unsupported
    notes: "Agent implements cancel but returns UnsupportedOperationError for session/cancel."
  - capability: session_modes
    support: supported
    notes: "Config options expose model, effort, and mode; setSessionMode and setSessionConfigOption are implemented."
  - capability: streaming
    support: supported
    notes: "Emits session/update events for text chunks, thought chunks, tool calls, command lists, and usage."
  - capability: permissions
    support: supported
    notes: "Kilo permission.asked events are bridged to session/request_permission reverse requests."
  - capability: fs_read
    support: unsupported
    notes: "No source path calls AgentSideConnection readTextFile; Kilo reads files through its own local services."
  - capability: fs_write
    support: partial
    notes: "Only proposed edit approval uses writeTextFile to ask the client to write the patched file preview/content."
  - capability: terminal
    support: unsupported
    notes: "No ACP terminal/create, output, wait, kill, or release reverse requests are used; shell execution is Kilo's native tool path surfaced as tool_call updates."
  - capability: mcp
    support: supported
    notes: "initialize advertises MCP HTTP and SSE support; session mcpServers are registered into Kilo's internal MCP configuration."
  - capability: media
    support: supported
    notes: "initialize advertises image prompt capability; tool output can include image data URL attachments."
  - capability: plans
    support: unknown
    notes: "No explicit ACP plan update mapping found in the inspected ACP source."
  - capability: extensions
    support: partial
    notes: "Uses _meta for terminal-auth and prompt response metadata; no Kilo-specific custom JSON-RPC methods found."
reverse_requests:
  - method: session/request_permission
    purpose: permission
    client_must_handle: true
    notes: "Required for normal guarded tool use. If unavailable or failing, Kilo rejects the underlying permission request."
  - method: fs/write_text_file
    purpose: fs_write
    client_must_handle: false
    notes: "Capability-gated by method availability on the connection. Used after an edit permission is allowed to write the proposed patched content through the ACP client."
  - method: session/update
    purpose: other
    client_must_handle: true
    notes: "Although it is a notification rather than a request, clients must route it for usable streaming UI."
permission_model:
  mechanism: "Kilo native permissions bridged to ACP session/request_permission."
  timeout: unknown
  default_policy: "If the ACP client lacks requestPermission or the reverse request fails, Kilo replies reject to the native permission request."
  approval_values:
    - once
    - always
    - reject
  notes: "ACP options are allow_once, allow_always, and reject_once. The selected ACP option is translated to Kilo permission replies once, always, or reject."
filesystem_model:
  read_methods: []
  write_methods:
    - fs/write_text_file
  path_base: "Kilo sessions are created with an ACP cwd. Permission docs say file-tool paths resolve first and are checked relative to the current worktree; external paths are relevant for external_directory permissions. ACP writeTextFile uses the filepath from tool metadata, usually an absolute or resolved path."
  sandboxing: "Kilo's security document says CLI permissions are not a sandbox. Recent releases add optional macOS/Linux sandbox controls outside the ACP layer."
  notes: "Do not rely on ACP fs/read_text_file for host-side reads; Kilo uses its own local filesystem services. A Claudine host should still enforce its own path policy before honoring writeTextFile."
terminal_model:
  supported: false
  methods: []
  shell: "Kilo native shell tool, not ACP terminal reverse requests."
  cwd: "ACP --cwd/session cwd controls the Kilo project directory."
  streaming: "Shell output is sent as tool_call_update content snapshots for bash tools."
  cancellation: "session/cancel is unsupported; ACP terminal kill is not used."
  notes: "Host-side command execution policy cannot be enforced via ACP terminal methods for Kilo. It must be enforced by Kilo permission responses, Kilo config, or an outer process sandbox."
streaming_model:
  update_methods:
    - session/update
  text_events:
    - agent_message_chunk
    - agent_thought_chunk
  tool_events:
    - tool_call
    - tool_call_update
  plan_events: []
  error_events:
    - tool_call_update
  notes: "Also emits available_commands_update after session creation/load/resume/fork and usage_update after prompt turns. Thought chunks come from reasoning parts."
auth_setup:
  required: true
  mechanisms:
    - "kilo auth login"
    - "provider API keys in Kilo auth storage"
    - "Kilo account or BYOK credentials depending on selected provider/model"
  headless_notes: "Run auth setup before launching ACP. initialize advertises a kilo-login auth method whose description tells users to run kilo auth login; authenticate does not perform the login itself."
  notes: "Local inspection found provider credentials in /Users/ken/.local/share/kilo/auth.json and a schema-only config at /Users/ken/.config/kilo/kilo.jsonc. Values were not copied into this document."
env_vars:
  - name: KILO_CLIENT
    effect: "Set internally to acp by the acp command; disables Kilo snapshots and lets subsystems identify ACP clients."
  - name: KILO_ACP_PROFILE
    effect: "When set to 1, writes ACP timing profile lines to stderr."
  - name: KILO_BIN_PATH
    effect: "npm wrapper override for selecting the packaged Kilo binary."
  - name: KILO_TREE_SITTER_WASM_DIR
    effect: "npm wrapper sets this to bundled tree-sitter WASM resources when unset."
rust_client:
  crate: agent-client-protocol
  connection_type: "Spawn kilo acp with piped stdin/stdout and wrap the child pipes in a ClientSideConnection over newline-delimited JSON-RPC."
  localset_required: true
  reverse_request_handlers:
    - session/request_permission
    - fs/write_text_file
    - session/update
  desktop_streaming_pattern: "Forward session/update notifications into a tokio mpsc channel consumed by the UI runtime; keep permission and write requests on request/response paths."
  notes: "Use the official crate for typed ACP where possible. Fall back to schema/JSON-RPC only if Claudine needs to support SDK 0.21.0 fields before the Rust crate exposes matching helpers."
compatibility:
  - client: Zed
    status: partial
    issue: https://github.com/zed-industries/zed/issues/52782
    workaround: "Ensure request_permission UI handling is wired; otherwise bash/edit permission prompts can appear to hang."
  - client: Zed ACP Registry
    status: works
    issue: https://zed.dev/acp/agent/kilo
    workaround: "Install from registry or configure npx @kilocode/cli@7.3.54 acp manually."
  - client: acpx
    status: works
    issue: https://github.com/Kilo-Org/kilocode/issues/6766
    workaround: "Use kilo acp or npx @kilocode/cli acp; documentation was the reported gap."
  - client: OpenClaw/acpx orchestrators
    status: partial
    issue: https://github.com/Kilo-Org/kilocode/issues/8016
    workaround: "No ACP launch-site --model workaround was reported in that issue; configure Kilo's default model or use ACP config options after session creation where the client supports them."
recent_changes:
  - date: "2026-07-03"
    version: "7.4.1"
    change: "Latest npm and GitHub release observed; installed local CLI was 7.3.45."
    impact: "ACP source still uses @agentclientprotocol/sdk 0.21.0 and protocolVersion 1 in v7.4.1."
  - date: "2026-07-03"
    version: "7.4.1"
    change: "GitHub release notes include optional macOS/Linux sandbox improvements for agent shell and file-tool writes."
    impact: "Important security context, but not exposed as ACP terminal/fs delegation."
  - date: "2026-03-31"
    version:
    change: "Issue #8016 reported that kilo acp lacked a --model launch flag."
    impact: "Clients may need to select models through ACP config options or Kilo config rather than launch arguments."
  - date: "2026-03-08"
    version:
    change: "Issue #6766 requested ACP integration documentation and reported acpx compatibility."
    impact: "Confirms discoverability/documentation gap around an existing implementation."
quirks:
  - "The command is native, not a separate adapter, but source paths still live under packages/opencode because Kilo CLI is built from OpenCode-derived internals."
  - "kilo acp starts a local HTTP server for its internal SDK, then exposes ACP over stdio to the parent client."
  - "session/cancel exists on the Agent class but always returns UnsupportedOperationError."
  - "ACP terminal reverse requests are not used; shell commands are native Kilo tools surfaced as tool updates."
  - "The initialized auth method's terminal-auth metadata in v7.3.45 says command opencode auth login even though the user-facing Kilo command is kilo auth login."
  - "Local session HOME was /Users/ken/.claudine, but real Kilo state observed from logs and direct HOME=/Users/ken probes was under /Users/ken/.local/share/kilo and /Users/ken/.config/kilo."
gaps:
  - "Exact timeout semantics for session/request_permission were not found in Kilo ACP source."
  - "No documented line-number base for ACP locations was found in Kilo-specific docs; Kilo's ACP location mapper only forwards paths from tool metadata."
  - "No explicit plan update mapping was found; plan support should be treated as unknown until a live plan-producing run is captured."
  - "No live prompt turn was run during this research, so streaming findings are source-inspection based rather than captured transcript evidence."
changes: []
requires_claudine_update: true
reason: "Adding Kilo ACP support would require Claudine to add a native ACP provider launch path, implement request_permission and write_text_file reverse routing, route session/update streaming events, and account for Kilo's lack of ACP terminal and session/cancel support."
---

# Kilo Code ACP Research

## Overview

Kilo Code has native ACP support in the primary CLI package, not a separate adapter. The installed commands `kilo` and `kilocode` both resolve to the same `@kilocode/cli` binary wrapper, and `kilo --help` lists `kilo acp` as "start ACP (Agent Client Protocol) server." The official CLI docs also list `kilo acp` in the top-level command table.

The upstream implementation is in `packages/opencode/src/acp` because Kilo's CLI is built from OpenCode-derived internals, but the user-facing product and CLI package are Kilo. Source inspection at tag `v7.4.1` shows a real ACP agent implementation backed by `@agentclientprotocol/sdk` 0.21.0. The command starts Kilo's internal local HTTP server for its own SDK calls, then exposes ACP to the parent client over newline-delimited JSON-RPC on stdio.

This is therefore classified as `native`: launch `kilo acp` or the equivalent npm/binary command. There is no bridge process translating a proprietary Kilo stream into ACP; the primary CLI command itself hosts the ACP endpoint.

## Launching ACP

The installed CLI exposes:

```bash
kilo acp --cwd /path/to/project
```

The same command is available through the alias:

```bash
kilocode acp --cwd /path/to/project
```

The Zed ACP registry page gives this manual launch form:

```bash
npx @kilocode/cli@7.3.54 acp
```

Local installed version on this host:

```text
kilo --version => 7.3.45
kilocode --version => 7.3.45
```

Current npm `latest` observed during the run was `7.4.1`. The upstream `v7.4.1` Zed extension metadata launches `opencode acp` from release archives, while the npm package and Kilo docs expose `kilo acp`. Treat `opencode acp` in source/registry assets as an internal compatibility naming artifact, not a separate adapter.

Transport is stdio newline-delimited JSON-RPC. The `acp` command builds a `WritableStream` around `process.stdout`, a `ReadableStream` around `process.stdin`, passes them to `ndJsonStream`, and constructs an `AgentSideConnection`.

The `acp` command also accepts the shared network options used to start Kilo's internal server:

| Option | Meaning |
| --- | --- |
| `--cwd` | ACP session working directory, defaulting to the launch process cwd. |
| `--port` | Internal Kilo HTTP server port, default `0`. |
| `--hostname` | Internal server hostname, default `127.0.0.1`. |
| `--mdns` | Enable mDNS service discovery for the internal server. |
| `--mdns-domain` | mDNS name, default `kilo.local`. |
| `--print-logs`, `--log-level` | Logging controls; ACP JSON-RPC remains on stdout. |

Observed initialize probe:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientInfo":{"name":"claudine-probe","version":"0"},"clientCapabilities":{"fs":{"readTextFile":true,"writeTextFile":true},"terminal":true,"_meta":{"terminal-auth":true}}}}
```

The installed CLI returned `protocolVersion: 1`, `agentInfo.name: "Kilo"`, and `agentInfo.version: "7.3.45"`.

## Protocol and Capabilities

Kilo's ACP implementation supports ACP protocol version 1. This was verified two ways:

| Evidence | Finding |
| --- | --- |
| Installed CLI initialize probe | Returned `protocolVersion: 1`. |
| Upstream source | `initialize` returns `protocolVersion: 1`. |
| Package manifest | `packages/opencode/package.json` depends on `@agentclientprotocol/sdk` `0.21.0`. |

Advertised initialize capabilities from the installed CLI:

```json
{
  "agentCapabilities": {
    "loadSession": true,
    "mcpCapabilities": {
      "http": true,
      "sse": true
    },
    "promptCapabilities": {
      "embeddedContext": true,
      "image": true
    },
    "sessionCapabilities": {
      "close": {},
      "fork": {},
      "list": {},
      "resume": {}
    }
  }
}
```

Capability summary:

| Area | Support | Notes |
| --- | --- | --- |
| Initialize | Supported | Returns protocol version, auth methods, agent info, and capabilities. |
| Authentication | Partial | `authenticate` accepts `kilo-login`, but actual login is out of band. |
| New session | Supported | Creates an internal Kilo session for the requested `cwd`. |
| Load/list/resume/close/fork | Supported | Advertised through `sessionCapabilities` and implemented in source. |
| Prompt | Supported | Text, embedded context/resource links, files, and images map into Kilo prompt parts. |
| Cancel | Unsupported | `session/cancel` is implemented but returns `UnsupportedOperationError`. |
| Modes/model/effort | Supported | Exposed through config options and setters. |
| Streaming | Supported | Uses `session/update` notifications. |
| Permissions | Supported | Native permission prompts are bridged to `session/request_permission`. |
| Filesystem read | Unsupported | No ACP `readTextFile` call found; Kilo reads through native local services. |
| Filesystem write | Partial | Proposed edit approval can call `writeTextFile` on the ACP client. |
| Terminal | Unsupported as ACP terminal | Shell activity is a native Kilo tool surfaced as tool updates; no `terminal/*` reverse requests found. |
| MCP | Supported | HTTP and SSE MCP capabilities are advertised; session MCP servers are registered internally. |
| Auth media | Supported for images | Image prompt capability is advertised, and image data URL attachments can appear in tool output. |
| Plans | Unknown | No explicit ACP plan update mapping found in the inspected source. |
| Extensions | Partial | Uses `_meta` for terminal-auth and response metadata; no Kilo-specific custom methods found. |

## Reverse Requests

Kilo can send these client-facing ACP calls:

| Method | Purpose | Required for usability | Notes |
| --- | --- | --- | --- |
| `session/request_permission` | Permission prompt | Yes | Required when Kilo asks approval for guarded tools such as shell/edit. If the connection does not expose `requestPermission`, Kilo rejects the native permission. |
| `fs/write_text_file` | Client-side file write | No, capability-gated | Used only for proposed edit content after an edit permission is allowed and the connection has `writeTextFile`. |
| `session/update` | Streaming notification | Yes for UI | Not a request, but clients must process it for text, thought, tool, commands, and usage updates. |

Representative permission request shape from source:

```json
{
  "sessionId": "ses_...",
  "toolCall": {
    "toolCallId": "call-or-permission-id",
    "status": "pending",
    "title": "bash",
    "rawInput": {
      "command": "..."
    },
    "kind": "execute",
    "locations": []
  },
  "options": [
    { "optionId": "once", "kind": "allow_once", "name": "Allow once" },
    { "optionId": "always", "kind": "allow_always", "name": "Always allow" },
    { "optionId": "reject", "kind": "reject_once", "name": "Reject" }
  ]
}
```

For edit permissions, if the metadata contains `filepath` and `diff`, Kilo applies the unified diff to current file content and sends the resulting full content through `writeTextFile`:

```json
{
  "sessionId": "ses_...",
  "path": "/absolute/or/resolved/path",
  "content": "patched file content"
}
```

## Permissions, Filesystem, and Terminal

Kilo's permission model remains Kilo-native. The ACP layer is a UI bridge over internal `permission.asked` events. A client should return one of the advertised options:

| ACP option | Kilo reply |
| --- | --- |
| `once` | `once` |
| `always` | `always` |
| `reject` or any non-selected outcome | `reject` |

If request handling fails, Kilo logs the error and rejects the permission. This is a good default for host-side safety: Claudine should do the same when a request cannot be evaluated confidently.

Filesystem behavior is mixed. Kilo does not delegate ordinary reads through ACP. It reads files through its own local services in the process launched by the client. File-tool permissions are documented as resolving paths first and then checking them relative to the current worktree; absolute paths matter mainly for external-directory permissions and shell commands outside the worktree.

The only observed ACP filesystem reverse call is `writeTextFile` for approved edit diffs. A Claudine client must still apply its own path policy before honoring that write. Kilo's own `SECURITY.md` states that CLI permissions are a UX feature and not a sandbox. Recent Kilo releases mention optional macOS/Linux sandbox controls for shell and file-tool writes, but this is outside the ACP terminal/filesystem delegation surface.

Terminal behavior is not ACP terminal behavior. Kilo can run shell commands as native tools, but the ACP implementation does not call `terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, or `terminal/release`. Shell output appears in `tool_call_update` content snapshots for bash tools. Claudine cannot enforce command execution policy by implementing ACP terminal handlers for Kilo; it must enforce through permission responses, Kilo configuration, and outer process sandboxing.

## Streaming and UI Integration

Kilo maps internal event-stream records into `session/update` notifications:

| Kilo/internal event | ACP update | UI route |
| --- | --- | --- |
| Assistant text delta | `agent_message_chunk` | Chat transcript text stream. |
| Reasoning delta | `agent_thought_chunk` | Collapsible thought/reasoning stream. |
| Tool part first seen | `tool_call` | Create a tool row. |
| Tool running/completed/error | `tool_call_update` | Update tool status, output, locations, and raw output. |
| Available commands | `available_commands_update` | Slash command palette or command registry. |
| Usage | `usage_update` | Token/cost/context meter. |

Tool kind mapping is source-defined:

| Kilo tool | ACP kind |
| --- | --- |
| `bash`, `shell` | `execute` |
| `webfetch` | `fetch` |
| `edit`, `patch`, `write` | `edit` |
| `grep`, `glob`, repository/context tools | `search` |
| `read` | `read` |
| Other tools | `other` |

The UI loop should treat `session/update` as the single inbound event bus. Request/response reverse calls such as `session/request_permission` should remain on a blocking approval path, while notification handling stays non-blocking.

## Authentication and Setup

Kilo ACP requires Kilo/provider authentication to already be usable. `initialize` advertises one auth method:

```json
{
  "id": "kilo-login",
  "name": "Login with Kilo",
  "description": "Run `kilo auth login` in the terminal"
}
```

When the client advertises `_meta.terminal-auth: true`, Kilo v7.3.45 returned terminal-auth metadata with command `opencode` and args `["auth", "login"]`. That appears to be an internal naming artifact; the user-facing command is `kilo auth login`.

Local inspection found:

| Path | Finding |
| --- | --- |
| `/Users/ken/.kilo` | Not present. Negative probe. |
| `/Users/ken/.local/share/kilo/auth.json` | Present, containing provider API credentials. Values were not copied. |
| `/Users/ken/.config/kilo/kilo.jsonc` | Present with `$schema: "https://app.kilo.ai/config.json"`. |
| `/Users/ken/.local/share/kilo/kilo.db` | Present; logs show Kilo opening this database. |
| `/Users/ken/.local/share/kilo/log/*.log` | Present; logs show config probes and Kilo session/export activity. |

The session environment had `HOME=/Users/ken/.claudine`, which caused a bare `~/.kilo` probe to point at `/Users/ken/.claudine/.kilo`. Direct probes against `/Users/ken` found the Kilo state above.

Environment variables observed in source/wrapper:

| Variable | Effect |
| --- | --- |
| `KILO_CLIENT` | Set internally to `acp`; disables snapshots for ACP clients. |
| `KILO_ACP_PROFILE` | When `1`, emits ACP timing profile lines to stderr. |
| `KILO_BIN_PATH` | npm wrapper override for selecting the packaged binary. |
| `KILO_TREE_SITTER_WASM_DIR` | npm wrapper points bundled binaries at tree-sitter WASM resources. |

## Compatibility, Quirks, and Workarounds

Known compatibility notes:

| Client | Status | Evidence | Workaround |
| --- | --- | --- | --- |
| Zed ACP Registry | Works | Zed registry page lists Kilo and manual launch. | Install from registry or configure the npm command manually. |
| Zed issue #52782 | Partial/bug report | A user reported Kilo hanging when waiting for bash approval while UI did not reflect the prompt. | Ensure `session/request_permission` is handled and visible. |
| acpx | Works according to issue report | Kilo issue #6766 says Kilo has complete ACP v1 support and works with acpx, but lacked docs. | Use `kilo acp`; expect documentation gaps. |
| OpenClaw/acpx orchestrators | Partial | Kilo issue #8016 reports no `--model` flag for `kilo acp`. | Configure Kilo's default model or use ACP config options after session creation when available. |

Quirks:

- `kilo acp` starts an internal HTTP server even though the ACP transport to the host is stdio.
- Source and Zed extension assets may say `opencode acp`; npm/Kilo docs say `kilo acp`.
- `session/cancel` is present but unsupported.
- ACP terminal reverse requests are unused; shell execution is a native Kilo tool.
- Ordinary file reads are not host-delegated through ACP.
- The v7.3.45 terminal-auth metadata says `opencode auth login`, while user-facing auth is `kilo auth login`.
- ACP clients do not support Kilo snapshots; source disables snapshots when `KILO_CLIENT === "acp"`.

## Recent Changes

Recent changes affecting ACP-adjacent behavior:

| Date | Version | Change | Impact |
| --- | --- | --- | --- |
| 2026-07-03 | 7.4.1 | Latest npm/GitHub release observed during research. | Source still uses ACP protocol v1 and `@agentclientprotocol/sdk` 0.21.0. |
| 2026-07-03 | 7.4.1 | Release notes mention optional macOS/Linux sandbox improvements for agent shell and file-tool writes. | Security-relevant, but not ACP terminal/fs delegation. |
| 2026-03-31 | Issue #8016 | Reported missing `--model` flag for `kilo acp`. | Launch-site model selection may be unavailable. |
| 2026-03-08 | Issue #6766 | Requested ACP docs and reported acpx compatibility. | Confirms ACP implementation existed before docs caught up. |

## Rust Client Example

Use the official `agent-client-protocol` crate as the first choice for a Claudine Rust client. Spawn `kilo acp` with piped stdin/stdout, wrap those pipes in the crate's client-side connection, then call `initialize`, `authenticate`, `session/new`, and `session/prompt`.

Sketch:

```rust
use std::process::Stdio;
use tokio::process::Command;

async fn spawn_kilo_acp(cwd: &std::path::Path) -> anyhow::Result<()> {
    let mut child = Command::new("kilo")
        .arg("acp")
        .arg("--cwd")
        .arg(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");

    // Wrap stdout/stdin in agent-client-protocol's client-side connection.
    // Then initialize with protocolVersion 1 and client capabilities for
    // request_permission plus write_text_file if Claudine is willing to handle it.
    drop((stdin, stdout));
    Ok(())
}
```

If the Rust crate version available in Claudine lags the TypeScript SDK fields used by Kilo 0.21.0, use the official schema crate or a narrow JSON-RPC layer for those fields while preserving the same method names and newline framing.

## Rust Reverse Request Handling

A Claudine ACP client must implement `session/request_permission` before Kilo is usable in guarded workflows. The handler should:

1. Normalize the tool kind, tool title, raw input, and locations into Claudine's policy engine.
2. Apply host policy before showing a UI prompt.
3. Return `once`, `always`, or `reject` by selecting one of Kilo's provided options.
4. Default to reject on parse failure, policy uncertainty, timeout, or UI cancellation.

For `fs/write_text_file`, treat Kilo as proposing a client-side write. Validate the path against Claudine's workspace policy, ensure parent directories and symlinks do not escape the allowed root, then perform an atomic write or return an ACP error. This request is capability-gated; Claudine can omit write capability if it wants Kilo to rely on its own native edit path instead of host-side writes.

## Rust Host Command Handling

Kilo does not ask the ACP client to execute host commands through `terminal/*`. The Rust client should not wait for `terminal/create` or `terminal/output` when integrating Kilo. Instead:

- Treat `session/request_permission` with tool kind `execute` as the approval point for shell commands.
- Use Kilo config and permission rules to reduce which commands Kilo may ask for.
- Use an outer process sandbox if Claudine needs hard host isolation.
- Render command progress from `tool_call` and `tool_call_update`.

Because `session/cancel` is unsupported, host-side cancellation should terminate the child process or close stdin/stdout rather than relying on a graceful ACP cancel request.

## Rust Desktop Streaming Bridge

Use a channel boundary between the ACP reader task and the desktop UI:

```rust
use tokio::sync::mpsc;

enum KiloUiEvent {
    Text { session_id: String, message_id: String, delta: String },
    Thought { session_id: String, message_id: String, delta: String },
    Tool { session_id: String, update: serde_json::Value },
    Usage { session_id: String, used: u64, size: u64 },
    Commands { session_id: String, names: Vec<String> },
}

fn bridge_updates(tx: mpsc::Sender<KiloUiEvent>, update: serde_json::Value) {
    // Decode session/update and send typed UI events.
    // Tauri can forward these via AppHandle::emit; iced can drain the receiver
    // from a subscription/task and update application state.
    let _ = (tx, update);
}
```

Keep reverse requests off this fire-and-forget path. Permission and write requests need a response, so they should call into a policy/UI service and await a result while the streaming task continues to drain notifications.

## Claudine Integration Notes

Adding Kilo ACP support to Claudine would require:

- Provider detection for `kilo` and `kilocode`, with version probing via `--version`.
- A native ACP launch profile: `kilo acp --cwd <workspace>`, not an adapter package.
- JSON-RPC stdio framing with no non-ACP stdout tolerance.
- Initialize capability negotiation for protocol v1, MCP HTTP/SSE, image/embedded context, session list/resume/fork/close, and config options.
- Reverse request handling for `session/request_permission`.
- Optional `fs/write_text_file` support gated by Claudine's host filesystem policy.
- Streaming routing for `agent_message_chunk`, `agent_thought_chunk`, `tool_call`, `tool_call_update`, `available_commands_update`, and `usage_update`.
- Explicit handling for unsupported `session/cancel` and absent ACP terminal methods.
- Auth preflight that detects Kilo login/provider credentials without exposing secrets.
- A policy decision about whether to rely on Kilo native file/shell permissions, wrap the child in an outer sandbox, or both.

The load-bearing integration fact is that Kilo's ACP endpoint is native but not fully host-delegated. Claudine can get standard ACP streaming and permission prompts, but it cannot enforce filesystem reads or command execution through ACP terminal/filesystem reverse requests alone.

## Changelog

Initial research file created on 2026-07-03.

## Sources

- [Kilo Code CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo CLI command reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo repository](https://github.com/Kilo-Org/kilocode)
- [Kilo v7.4.1 release notes](https://github.com/Kilo-Org/kilocode/releases)
- [Zed ACP registry entry for Kilo](https://zed.dev/acp/agent/kilo)
- [Zed issue #52782: ACP integration with kilo-cli hangs expecting user approval](https://github.com/zed-industries/zed/issues/52782)
- [Kilo issue #6766: ACP integration documentation and acpx compatibility guide](https://github.com/Kilo-Org/kilocode/issues/6766)
- [Kilo issue #8016: add --model flag to ACP server mode](https://github.com/Kilo-Org/kilocode/issues/8016)
- [Kilo source: ACP command](https://github.com/Kilo-Org/kilocode/blob/v7.4.1/packages/opencode/src/cli/cmd/acp.ts)
- [Kilo source: ACP agent](https://github.com/Kilo-Org/kilocode/blob/v7.4.1/packages/opencode/src/acp/agent.ts)
- [Kilo source: ACP service](https://github.com/Kilo-Org/kilocode/blob/v7.4.1/packages/opencode/src/acp/service.ts)
- [Kilo source: ACP event bridge](https://github.com/Kilo-Org/kilocode/blob/v7.4.1/packages/opencode/src/acp/event.ts)
- [Kilo source: ACP permission bridge](https://github.com/Kilo-Org/kilocode/blob/v7.4.1/packages/opencode/src/acp/permission.ts)
- [Kilo source: ACP tool mapping](https://github.com/Kilo-Org/kilocode/blob/v7.4.1/packages/opencode/src/acp/tool.ts)
- [Kilo source: Zed extension metadata](https://github.com/Kilo-Org/kilocode/blob/v7.4.1/packages/extensions/zed/extension.toml)
- Local inspection: installed `@kilocode/cli` 7.3.45, `kilo acp --help`, initialize JSON-RPC probe, `/Users/ken/.local/share/kilo`, and `/Users/ken/.config/kilo`.
