---
prompt: |-
    The ACP protocol sits on top of JSON-RPC and provides an open-source standard (spearheaded by the Zed and JetBrains teams) that does for AI coding agents what the Language Server Protocol (LSP) did for language tooling.

    Your task is to do research into who which Rust crates support the use of the ACP protocol. For each crate found:

    - Name of the library
    - URL (the primary URL for the software)
    - What features does the crate expose?
        - Detail when you should and should not use the various features exposed
    - How well does this crate cover the uses cases typically associated with the ACP protocol?
    - Which crates are most commonly compared to this crate? How do they compare?

    After detailing all the crates which cater to the ACP protocol, discuss how you might approach a bespoke/custom build instead of using one of these packages.

    - list out when you recommend using one of the crates found
    - list out when you recommend building a bespoke solution for ACP

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
update_policy:
    - Duration(3 mo)
---

# Rust Crates Supporting ACP (Agent Client Protocol)

The [Agent Client Protocol](https://agentclientprotocol.com/) is a JSON-RPC 2.0 based protocol that standardizes communication between code editors and AI coding agents. The protocol defines two message types — **Methods** (request-response pairs) and **Notifications** (one-way messages) — and communicates over any bidirectional stream (typically stdio between a parent editor process and a child agent process).

---

## 1. `agent-client-protocol` (Official SDK)

- **URL**: [crates.io](https://crates.io/crates/agent-client-protocol) | [docs.rs](https://docs.rs/agent-client-protocol/latest/agent_client_protocol/) | [GitHub](https://github.com/agentclientprotocol/rust-sdk)
- **Version**: 0.9.4
- **License**: Apache-2.0
- **Maintainer**: Agent Client Protocol organization (Zed Industries core contributors)
- **Downloads**: ~65k/month

### Features Exposed

The official SDK provides full implementations of both sides of the ACP protocol:

**Core Traits:**

| Trait | Purpose | When to Use |
|-------|---------|-------------|
| `Agent` | Server-side trait for building coding agents | Implement when building an AI agent that editors will connect to |
| `Client` | Client-side trait for building editor integrations | Implement when building an editor or tool that hosts agents |
| `Side` | Marker trait for connection endpoint typing | Used internally; rarely implemented directly |
| `MessageHandler` | Low-level protocol message dispatch | Only use if you need custom message routing beyond the Agent/Client traits |

**Connection Types:**

| Type | Purpose | When to Use |
|------|---------|-------------|
| `AgentSideConnection` | Wraps an `Agent` impl and provides `Client` methods | Use when building an agent — you implement `Agent`, connection gives you `Client` calls (e.g., request permissions) |
| `ClientSideConnection` | Wraps a `Client` impl and provides `Agent` methods | Use when building an editor — you implement `Client`, connection gives you `Agent` calls (e.g., send prompts) |

**Protocol Capabilities:**

- **Session management**: `session/new`, `session/load`, `session/cancel`, `session/set_mode`
- **Prompt handling**: `session/prompt` with streaming `AgentMessageChunk` responses
- **Permission system**: `session/request_permission` for tool call authorization
- **File system operations**: `fs/readTextFile`, `fs/writeTextFile` (opt-in via capabilities)
- **Terminal operations**: Create, execute, monitor, and kill terminal commands
- **MCP integration**: Model Context Protocol server discovery and tool provision
- **Extensibility**: `ext_method()` and `ext_notification()` for custom RPC extensions

**Content Types**: `TextContent`, `ImageContent`, `AudioContent`, `ResourceLink`, `EmbeddedResource`, `ToolCall`, `Plan`, `PlanEntry`

**Transport**: Transport-agnostic — works with any `AsyncRead + AsyncWrite` stream. Typically used with tokio stdio but supports TCP, Unix sockets, etc.

### When to Use

- **Use when**: You want full spec compliance, are building a production agent or client, need both sides of the protocol, want to stay current with protocol evolution.
- **Do not use when**: You only need schema types without the connection machinery (use `agent-client-protocol-schema` instead), or you need proxy/middleware capabilities (consider `sacp`).

### Protocol Coverage

**Comprehensive**. This is the reference implementation maintained by the protocol authors. It covers 100% of the ACP specification including initialization, authentication, session lifecycle, prompts, streaming, permissions, file system, terminals, MCP integration, modes, and extensions.

### Compared To

- **`agentic-coding-protocol`**: The predecessor crate by Zed Industries. `agent-client-protocol` supersedes it with a cleaner API, official status, and ongoing maintenance.
- **`vtcode-acp-client`**: Client-only. `agent-client-protocol` provides both sides.
- **`sacp`**: Extension layer that builds on top of `agent-client-protocol-schema`, not a replacement.

---

## 2. `agent-client-protocol-schema`

- **URL**: [crates.io](https://crates.io/crates/agent-client-protocol-schema) | [docs.rs](https://docs.rs/agent-client-protocol-schema)
- **Version**: 0.10.8
- **License**: Apache-2.0
- **Maintainer**: Agent Client Protocol organization

### Features Exposed

Pure type definitions and schema for ACP — no connection logic, no runtime.

**Key Components:**

- 100+ struct/enum types covering the entire ACP message schema
- `AgentSide` / `ClientSide` marker types
- All request/response/notification enums: `ClientRequest`, `AgentRequest`, `AgentNotification`, etc.
- `AgentCapabilities` / `ClientCapabilities` for capability negotiation
- Session, terminal, file system, and MCP types
- `schemars` integration for JSON Schema generation
- Builder utilities: `IntoOption`, `IntoMaybeUndefined`

### When to Use

- **Use when**: You are building a bespoke ACP implementation and want spec-accurate types without the official SDK's connection/transport layer. Also useful if you need JSON Schema generation from ACP types.
- **Do not use when**: You want a turnkey solution — this crate has no transport, no connection management, no message routing.

### Protocol Coverage

**Types only**. Covers 100% of the ACP message schema but provides zero runtime behavior. You must implement JSON-RPC framing, transport, and lifecycle yourself.

### Compared To

- **`agent-client-protocol`**: The full SDK depends on this crate. Use the schema crate alone only when building custom transport/routing.
- **`serde_json` + hand-rolled types**: The schema crate saves you from manually modeling 100+ types and tracking spec changes.

---

## 3. `agentic-coding-protocol` (Zed's Original / Predecessor)

- **URL**: [docs.rs](https://docs.rs/agentic-coding-protocol) | [GitHub](https://github.com/zed-industries/agentic-coding-protocol)
- **Version**: 0.0.11
- **License**: MIT
- **Maintainer**: Zed Industries (maxbrunsfeld, ConradIrwin, etc.)

### Features Exposed

The original protocol crate from Zed before the ACP specification was formalized:

- `AgentConnection` / `ClientConnection` for bidirectional communication
- `AnyAgentRequest` / `AnyClientRequest` enums for message dispatch
- Streaming via `AssistantMessageChunk` / `UserMessageChunk`
- Tool call management: `PushToolCall`, `UpdateToolCall`, `RequestToolCallConfirmation`
- Plan management: `UpdatePlan`, `PlanEntry`, `PlanEntryStatus`
- File operations: `ReadTextFile`, `WriteTextFile`
- Authentication: `Authenticate` method

### When to Use

- **Use when**: You are maintaining an existing project that already depends on this crate and cannot migrate yet.
- **Do not use when**: Starting a new project. This is the predecessor to `agent-client-protocol` and is no longer the canonical implementation. The API is less polished (19% documentation coverage vs 82% for the official SDK) and won't track future protocol changes.

### Protocol Coverage

**Partial/Dated**. Covers an early version of the protocol. Missing capabilities that have been added since the ACP specification stabilized (e.g., MCP integration, mode management, terminal operations were added later).

### Compared To

- **`agent-client-protocol`**: Direct successor. Prefer the official SDK for all new work.

---

## 4. `vtcode-acp-client`

- **URL**: [crates.io](https://crates.io/crates/vtcode-acp-client) | [docs.rs](https://docs.rs/vtcode-acp-client)
- **Version**: 0.82.1
- **License**: MIT
- **Maintainer**: VT Code project

### Features Exposed

A client-only ACP library with two API versions:

**V2 API (Recommended):**

- Full ACP protocol compliance
- Session lifecycle management (initialize, create sessions, send prompts)
- Capability negotiation
- SSE streaming for real-time updates
- Builder pattern for client configuration

**V1 API (Legacy/Deprecated):**

- HTTP-based communication
- Agent discovery and basic request/response

**Key Modules:**

- `capabilities` — Protocol versions, client/agent info, auth credentials
- `client_v2` — Primary `AcpClientV2` with builder pattern
- `session` — Session management with conversation turns
- `jsonrpc` — JSON-RPC 2.0 types
- `discovery` — Agent registry and detection
- `error` — Custom error types

### When to Use

- **Use when**: You are building a client/editor only and want a higher-level builder-pattern API with SSE streaming. Also useful if you need agent discovery and registry features.
- **Do not use when**: You need to build an agent (server-side) — this crate is client-only. Also avoid if you want the protocol authors' reference implementation.

### Protocol Coverage

**Client-side only, good coverage**. Comprehensive for the client role including discovery features not present in the official SDK. However, you cannot build an agent with this crate.

### Compared To

- **`agent-client-protocol`**: The official SDK provides both sides. `vtcode-acp-client` is more opinionated (builder pattern, SSE) but limited to client role.
- **Building custom**: If you need only a client and like the builder API, this saves effort. If you need both sides, use the official SDK.

---

## 5. `claude-code-acp-rs`

- **URL**: [crates.io](https://crates.io/crates/claude-code-acp-rs) | [GitHub](https://github.com/anthropics/claude-code-acp-rs)
- **Version**: 0.1.22
- **License**: (check repository)
- **Maintainer**: Community (not official Anthropic)

### Features Exposed

A complete ACP agent wrapping Claude Code — this is an **application**, not a library:

- Full ACP agent implementation for Claude Code
- File system watching and change detection
- Process execution and management
- Configuration file support (`~/.claude/settings.json`)
- Diagnostic mode (`--diagnostic`)

**Optional Feature:**

- `otel` — OpenTelemetry distributed tracing support

### When to Use

- **Use when**: You want to run Claude Code as an ACP agent in editors like Zed or JetBrains. Install via `cargo install claude-code-acp-rs`.
- **Do not use when**: Building your own agent or client — this is a ready-made binary, not a library for composition. It depends on `agent-client-protocol-schema` and `sacp` internally.

### Protocol Coverage

**Agent-side only**. Implements a specific Claude Code agent, not a general-purpose library.

### Compared To

- Not comparable to SDK crates — this is an end-user application, not a building block.

---

## 6. SACP Ecosystem (Symposium's Extensions to ACP)

- **URL**: [sacp crate](https://crates.io/crates/sacp) | [Blog post](https://smallcultfollowing.com/babysteps/blog/2025/10/08/symmacp/)
- **Maintainer**: Niko Matsakis / Symposium project
- **License**: (check repository)

The SACP ecosystem extends ACP with composable proxy/middleware capabilities. It consists of several crates:

| Crate | Purpose |
|-------|---------|
| `sacp` | Core protocol types and traits for SACP extensions |
| `sacp-conductor` | Orchestrates SACP proxy chains |
| `sacp-tokio` | Tokio-based utilities for SACP |
| `sacp-rmcp` | RMCP (MCP) integration for proxy components |
| `sacp-cookbook` | Example proxy implementations |
| `symposium-acp-proxy` | Base proxy infrastructure |

### Features Exposed

SACP adds four key extensions to ACP:

1. **Bidirectional conversation initialization** — Either side can provide initial state
2. **Tool provision by proxies** — Middleware can inject MCP tools into the agent
3. **Unprompted responses** — Agents can initiate messages without editor prompts
4. **Enriched conversation history** — Metadata and extended state beyond text

**Proxy Architecture:**

```
Editor ←→ Proxy₁ ←→ Proxy₂ ←→ … ←→ Agent
```

Each proxy can intercept, transform, and enhance messages flowing in either direction — like middleware for AI agents.

### When to Use

- **Use when**: You need composable agent middleware (e.g., inject context, add logging, provide extra tools, modify prompts). Also when building multi-agent orchestration that chains through proxies.
- **Do not use when**: You need a simple agent or client with no middleware layer. The proxy architecture adds complexity that isn't warranted for straightforward integrations.

### Protocol Coverage

**Extended superset of ACP**. Covers the base protocol through its dependency on `agent-client-protocol-schema` and adds proxy-specific capabilities. Not a standalone ACP implementation — it builds on top of the official schema.

### Compared To

- **`agent-client-protocol`**: SACP is complementary, not a replacement. Use the official SDK for standard agent/client and SACP when you need the proxy layer.

---

## Crate Comparison Summary

| Crate | Role | Full Protocol? | Agent? | Client? | Proxy? | Production Ready? |
|-------|------|:-:|:-:|:-:|:-:|:-:|
| `agent-client-protocol` | Official SDK | Yes | Yes | Yes | No | Yes |
| `agent-client-protocol-schema` | Types only | Types only | — | — | — | Yes |
| `agentic-coding-protocol` | Predecessor | Partial | Yes | Yes | No | Legacy |
| `vtcode-acp-client` | Client SDK | Client only | No | Yes | No | Yes |
| `claude-code-acp-rs` | Application | Agent only | Binary | No | No | Yes |
| `sacp` ecosystem | Proxy layer | Extended | Via proxy | Via proxy | Yes | Experimental |

---

## Building a Bespoke ACP Implementation

If none of the existing crates fit your needs, building a custom ACP implementation is feasible. The protocol is well-specified and the underlying technology stack is standard Rust.

### What You Would Need

1. **JSON-RPC 2.0 framing** — Parse `Content-Length` headers + JSON bodies over stdio (or your transport of choice). Crates like `serde_json` handle serialization; you write the framing loop.

2. **Message types** — Either use `agent-client-protocol-schema` for spec-accurate types (recommended) or hand-roll ~100 structs/enums from the [JSON schema](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/schema/schema.json).

3. **Async runtime** — `tokio` with `AsyncRead`/`AsyncWrite` for bidirectional communication. A `tokio::io::stdin()`/`stdout()` pair suffices for stdio transport.

4. **Request/response correlation** — Track JSON-RPC `id` fields to match responses to requests. A `HashMap<Id, oneshot::Sender<Result>>` pattern works well.

5. **Capability negotiation** — Implement the `initialize` handshake where agent and client advertise supported features.

6. **Streaming** — Handle `session/update` notifications for streaming agent responses back to the client.

### Skeleton Architecture

```
┌─────────────────────────────────────────┐
│  Your Agent / Client                     │
│                                          │
│  ┌──────────┐   ┌───────────────────┐   │
│  │ Business  │──▶│  Message Router   │   │
│  │  Logic    │◀──│  (id correlation) │   │
│  └──────────┘   └───────┬───────────┘   │
│                         │               │
│  ┌──────────────────────▼─────────────┐ │
│  │  JSON-RPC Framing Layer            │ │
│  │  (Content-Length + serde_json)      │ │
│  └──────────────────────┬─────────────┘ │
│                         │               │
│  ┌──────────────────────▼─────────────┐ │
│  │  Transport (stdio / TCP / socket)   │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

### Key Dependencies for Bespoke Build

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime, stdio/TCP streams |
| `serde` / `serde_json` | JSON serialization |
| `agent-client-protocol-schema` | (Optional) spec-accurate ACP types |
| `futures` | Stream combinators for message routing |
| `tracing` | Structured logging |

---

## When to Use an Existing Crate

- **You want spec compliance** — The official `agent-client-protocol` SDK tracks the specification. Hand-rolling types risks drift as the protocol evolves.
- **You need both sides** — Building an agent that also makes client calls (e.g., requesting permissions) is handled by the SDK's symmetric design.
- **Time to market matters** — The SDK handles JSON-RPC framing, id correlation, capability negotiation, and streaming out of the box.
- **You want ecosystem compatibility** — Agents built with the official SDK are tested against Zed and JetBrains. A bespoke implementation requires manual compatibility testing.
- **You need middleware/proxies** — Use `sacp` rather than building a proxy layer from scratch.
- **You only need a client** — `vtcode-acp-client` provides a polished builder-pattern API with agent discovery.

## When to Build Bespoke

- **You need a custom transport** — The official SDK is transport-agnostic but opinionated about `AsyncRead`/`AsyncWrite`. If you need HTTP, WebSocket, or a non-tokio runtime, a bespoke transport layer may be simpler than adapting the SDK.
- **You only need a subset of the protocol** — If your agent only handles `initialize` + `session/prompt` and ignores terminals, file system, and MCP, a slim bespoke implementation avoids pulling in 100+ types.
- **You have an existing JSON-RPC stack** — If your project already uses a JSON-RPC library (e.g., `jsonrpsee`, `tower-lsp`), adding ACP types on top may be cleaner than introducing a second RPC framework.
- **You need non-standard extensions** — While ACP supports `ext_method`/`ext_notification`, heavy customization beyond spec may be easier without fighting the SDK's abstractions.
- **You want minimal dependencies** — The official SDK pulls in `async-trait`, `futures`, `async-broadcast`, `derive_more`, `anyhow`, `log`, and `schemars`. A bespoke implementation using just `serde_json` + `tokio` has a much smaller dependency tree.
- **You are integrating into a `no_std` or embedded context** — None of the existing crates support `no_std`. A bespoke implementation could use synchronous stdio and avoid the tokio dependency entirely.

### Hybrid Approach (Recommended for Most Cases)

Use `agent-client-protocol-schema` for the type definitions and build your own transport/routing layer. This gives you:

- Spec-accurate types that track protocol evolution (types update via `cargo update`)
- Freedom to choose your own JSON-RPC framing, transport, and async runtime
- A much smaller dependency footprint than the full SDK
- Full control over error handling, logging, and connection lifecycle
