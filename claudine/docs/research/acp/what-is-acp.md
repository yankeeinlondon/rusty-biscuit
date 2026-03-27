---
prompt: |-
    The ACP protocol sits on top of JSON-RPC and provides an open-source standard (spearheaded by the Zed and JetBrains teams) that does for AI coding agents what the Language Server Protocol (LSP) did for language tooling.

    Provide a full overview of the APC protocol.

    - describe the general semantics and syntax that APC uses
    - describe any/all major versions of the specification along with dates these versions became available
    - what endpoints or operations does the APC specification provide?
    - what are the uses-cases supported by the APC specification?
    - describe any common gotchas that developers describe hitting when using the APC specification along with any solutions or workarounds that help in avoiding these gotchas.
    - are there any similar specifications which ACP is competing with (for developer and product attention)
    - provide a simple code example of using APC in:
        - Typescript
        - Python
        - Rust

    Frontmatter:
    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)
    - make sure to set a `latest_version` property which should be the LATEST version of the specification

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
latest_version: v0.10.8
update_policy:
    - MajorVersion(latest_version)
    - Duration(1 year)
---

## ACP overview

The protocol name is **ACP (Agent Client Protocol)**.

ACP is a bidirectional JSON-RPC protocol for connecting:

- a **Client** (usually an editor, IDE, or terminal UI), and
- an **Agent** (a coding assistant process that plans, edits, runs tools, and reports progress).

Conceptually:

- **LSP** standardized editor ↔ language-tooling interactions.
- **ACP** standardizes client ↔ coding-agent interactions.

It is transport-agnostic, but today the primary transport is newline-delimited JSON over stdio.

## Semantics and syntax

### Core message model

ACP uses JSON-RPC 2.0 semantics:

- **Requests**: include `id`; must receive either `result` or `error`.
- **Notifications**: omit `id`; no response is expected.
- **Errors**: standard JSON-RPC error object (`code`, `message`, optional `data`).

### Transport rules

For stdio transport, ACP requires:

- UTF-8 JSON-RPC messages.
- One message per line (newline-delimited).
- No non-ACP output on stdout.
- Stderr may be used for logs.

### Session lifecycle

Typical sequence:

1. `initialize` (negotiate protocol version + capabilities)
2. optional `authenticate`
3. `session/new` or `session/load`
4. repeated `session/prompt` turns, with `session/update` notifications and optional tool/permission/fs/terminal interactions
5. optional `session/cancel` (and draft `$/cancel_request`)

### Capability gating

ACP is strongly capability-driven:

- Clients advertise supported features (fs, terminal).
- Agents advertise supported features (loadSession, prompt/media capabilities, MCP transport support, session capabilities).
- Missing capability means “unsupported”; callers should not invoke that method.

### Extensibility

ACP provides two formal extension mechanisms:

- `_meta` object fields on protocol types for custom metadata.
- Custom methods prefixed with `_` (e.g. `_vendor/method`).

## Specification versions and dates

There are two important versioning layers in ACP:

1. **Protocol major version** (integer negotiated via `initialize.protocolVersion`)
2. **Published spec/schema releases** (Git tags like `v0.10.8`)

### Protocol major versions

- **v1**: current stable major protocol version (used in `initialize`).
- **v2**: proposed (work-in-progress design document; not ratified at time of writing).

### Published spec release lines (first availability)

- `v0.4.x` first published: **2025-09-17** (`v0.4.0`)
- `v0.5.x` first published: **2025-10-23** (`v0.5.0`)
- `v0.6.x` first published: **2025-10-24** (`v0.6.0`)
- `v0.7.x` first published: **2025-11-25** (`v0.7.0`)
- `v0.8.x` first published: **2025-11-28** (`v0.8.0`)
- `v0.9.x` first published: **2025-12-01** (`v0.9.0`)
- `v0.10.x` first published: **2025-12-08** (GitHub release publication of `v0.10.0`)

Latest released spec at update time: **`v0.10.8` (published 2026-02-04)**.

## Operations/endpoints in ACP

Because ACP uses JSON-RPC, these are method names rather than HTTP endpoints.

### Stable agent-side methods (Client → Agent)

- `initialize`
- `authenticate`
- `session/new`
- `session/prompt`
- `session/load` (optional capability)
- `session/set_mode` (legacy path; being replaced by config options)
- `session/set_config_option`
- `session/cancel` (notification)

### Stable client-side methods (Agent → Client)

- `session/request_permission`
- `fs/read_text_file` (optional capability)
- `fs/write_text_file` (optional capability)
- `terminal/create` (optional capability)
- `terminal/output` (optional capability)
- `terminal/wait_for_exit` (optional capability)
- `terminal/kill` (optional capability)
- `terminal/release` (optional capability)
- `session/update` (notification; streaming updates for content, tool calls, plans, command/mode/config updates, etc.)

### Draft/unstable methods currently in flight

- `session/fork`
- `session/list`
- `session/resume`
- `session/set_model`
- `$/cancel_request` (generic per-request cancellation notification)

## Supported use cases

ACP supports a broad set of coding-agent interaction patterns:

- IDE/editor chat and coding assistance over a standard protocol.
- Streaming multi-step agent output (`session/update` chunks).
- Human-in-the-loop tool permissions (`session/request_permission`).
- File operations against client-side buffers/filesystem (`fs/*`).
- Terminal command execution and live output (`terminal/*`).
- Session persistence and reload (`session/load`), plus draft list/resume workflows.
- Session-level mode/model/reasoning controls via config options.
- Slash-command UX (`available_commands_update` + prompt command text).
- MCP server wiring per session (`mcpServers` in session setup).

## Common gotchas and practical workarounds

### 1) Calling optional methods without capability checks

Problem:
Agents/clients call `fs/*`, `terminal/*`, or session extras unconditionally.

Workaround:
Treat `initialize` as hard gating. Build a capability matrix once and branch all method calls from it.

### 2) Relative paths and wrong line indexing

Problem:
ACP expects absolute paths and 1-based line numbers; integrations often assume project-relative paths or 0-based indexing.

Workaround:
Normalize outbound paths to absolute, and convert line indexes at boundaries.

### 3) Assuming notifications get responses

Problem:
Implementers wait for responses to `session/update` or `session/cancel`.

Workaround:
Model notifications as fire-and-forget; only track request/response lifecycle for methods with `id`.

### 4) Cancellation handling gaps

Problem:
Cancellation is partially feature-specific (`session/cancel`) and implementers miss nested/pending operations.

Workaround:
On cancel:

- stop model/tool work quickly,
- resolve pending permission requests as cancelled,
- return a final `session/prompt` result with `stopReason: "cancelled"`.

For granular cancellation, adopt draft `$/cancel_request` where supported.

### 5) Session replay/resume mismatch across agents

Problem:
Some agents support `session/load`; others only support resume-like behavior.

Workaround:
Feature-detect and provide adapter behavior (e.g. use resume + local history cache when full load is unavailable).

### 6) Not releasing terminal handles

Problem:
Agents create terminals and forget to release, leading to lingering resources.

Workaround:
Enforce a `create -> wait/output -> release` lifecycle and use finally/defer cleanup.

### 7) Message boundary ambiguity in chunked output

Problem:
Consecutive message chunks can be hard to segment cleanly in UI without stable message identifiers.

Workaround:
Use draft message-id patterns (`messageId`/`userMessageId`) where available, and otherwise segment conservatively around update-type/state transitions.

## Similar specifications competing for attention

### MCP (Model Context Protocol)

- Focus: model/application ↔ external tools/resources/prompts.
- Relationship to ACP: complementary in many stacks (ACP sessions often configure MCP servers), but both compete for integration mindshare in agent tooling ecosystems.

### A2A (Agent2Agent)

- Focus: agent ↔ agent interoperability and task exchange.
- Relationship to ACP: ACP is primarily client ↔ agent, while A2A is agent ↔ agent. In practice, platform teams may choose one first depending on product priorities.

### LSP (Language Server Protocol)

- Not a direct replacement, but a strong conceptual predecessor for editor protocol standardization. ACP often gets evaluated alongside LSP extension strategies in IDE product planning.

## Simple code examples

### TypeScript

```ts
import * as acp from "@agentclientprotocol/sdk";

async function run(connection: acp.ClientSideConnection) {
  await connection.initialize({
    protocolVersion: acp.PROTOCOL_VERSION,
    clientCapabilities: {
      fs: { readTextFile: true, writeTextFile: true },
      terminal: true,
    },
  });

  const session = await connection.newSession({
    cwd: process.cwd(),
    mcpServers: [],
  });

  const result = await connection.prompt({
    sessionId: session.sessionId,
    prompt: [{ type: "text", text: "Summarize this repository." }],
  });

  console.log(result.stopReason);
}
```

### Python

```python
from acp import PROTOCOL_VERSION, connect_to_agent, text_block
from acp.schema import ClientCapabilities, Implementation

async def run(conn):
    await conn.initialize(
        protocol_version=PROTOCOL_VERSION,
        client_capabilities=ClientCapabilities(),
        client_info=Implementation(
            name="example-client",
            title="Example Client",
            version="0.1.0",
        ),
    )

    session = await conn.new_session(mcp_servers=[], cwd="/abs/path/to/project")

    result = await conn.prompt(
        session_id=session.session_id,
        prompt=[text_block("Summarize this repository.")],
    )

    print(result.stop_reason)
```

### Rust

```rust
use agent_client_protocol as acp;

async fn run(conn: &acp::ClientSideConnection<impl acp::Client>) -> anyhow::Result<()> {
    conn.initialize(acp::InitializeRequest {
        protocol_version: acp::V1,
        client_capabilities: acp::ClientCapabilities::default(),
        client_info: Some(acp::Implementation {
            name: "example-client".into(),
            title: Some("Example Client".into()),
            version: "0.1.0".into(),
        }),
        meta: None,
    }).await?;

    let session = conn.new_session(acp::NewSessionRequest {
        cwd: std::env::current_dir()?,
        mcp_servers: vec![],
        meta: None,
    }).await?;

    let response = conn.prompt(acp::PromptRequest {
        session_id: session.session_id,
        prompt: vec!["Summarize this repository.".into()],
        meta: None,
    }).await?;

    println!("stop reason: {:?}", response.stop_reason);
    Ok(())
}
```

## Sources

- ACP main repo: <https://github.com/agentclientprotocol/agent-client-protocol>
- ACP releases API: <https://api.github.com/repos/agentclientprotocol/agent-client-protocol/releases?per_page=100>
- ACP tags API: <https://api.github.com/repos/agentclientprotocol/agent-client-protocol/tags?per_page=100>
- ACP changelog: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/CHANGELOG.md>
- ACP protocol overview: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/overview.mdx>
- ACP initialization: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/initialization.mdx>
- ACP transports: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/transports.mdx>
- ACP session setup: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/session-setup.mdx>
- ACP prompt turn: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/prompt-turn.mdx>
- ACP file system: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/file-system.mdx>
- ACP terminals: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/terminals.mdx>
- ACP tool calls: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/tool-calls.mdx>
- ACP slash commands: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/slash-commands.mdx>
- ACP session modes: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/session-modes.mdx>
- ACP session config options: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/session-config-options.mdx>
- ACP draft session list: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/protocol/draft/session-list.mdx>
- ACP draft schema meta (stable): <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/schema/meta.json>
- ACP draft schema meta (unstable): <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/schema/meta.unstable.json>
- ACP TypeScript SDK docs: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/libraries/typescript.mdx>
- ACP Python SDK docs: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/libraries/python.mdx>
- ACP Rust SDK docs: <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/libraries/rust.mdx>
- ACP TypeScript example: <https://raw.githubusercontent.com/agentclientprotocol/typescript-sdk/main/src/examples/client.ts>
- ACP Python example: <https://raw.githubusercontent.com/agentclientprotocol/python-sdk/main/examples/client.py>
- ACP Rust example: <https://raw.githubusercontent.com/agentclientprotocol/rust-sdk/main/examples/client.rs>
- ACP RFD (message IDs): <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/rfds/message-id.mdx>
- ACP RFD (request cancellation): <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/rfds/request-cancellation.mdx>
- ACP RFD (session resume): <https://raw.githubusercontent.com/agentclientprotocol/agent-client-protocol/main/docs/rfds/session-resume.mdx>
- MCP spec releases: <https://api.github.com/repos/modelcontextprotocol/specification/releases?per_page=20>
- MCP docs: <https://modelcontextprotocol.io/specification/2025-06-18>
- A2A repo: <https://github.com/a2aproject/A2A>
- A2A releases API: <https://api.github.com/repos/a2aproject/A2A/releases?per_page=20>
