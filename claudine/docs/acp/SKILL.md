---
name: acp
description: Detailed information on the Agent Client Protocol (ACP), libraries to use in Rust and Typescript, background details on the underlying JSON-RPC standard. Also includes detailed strategies for interacting with claude code, codex, kimi-code, opencode, gemini-cli, and other Agentic CLI providers.
---

## What is ACP?

The protocol name is **ACP (Agent Client Protocol)**.

ACP is a bidirectional JSON-RPC protocol for connecting:

- a **Client** (usually an editor, IDE, or terminal UI), and
- an **Agent** (a coding assistant process that plans, edits, runs tools, and reports progress).

Conceptually:

- **LSP** standardized editor ↔ language-tooling interactions.
- **ACP** standardizes client ↔ coding-agent interactions.

It is transport-agnostic, but today the primary transport is newline-delimited JSON over stdio.

For more details, review the [What is ACP?](./what-is-acp.md) document.

## JSON-RPC semantics

ACP uses JSON-RPC 2.0 semantics:

- **Requests**: include `id`; must receive either `result` or `error`.
- **Notifications**: omit `id`; no response is expected.
- **Errors**: standard JSON-RPC error object (`code`, `message`, optional `data`).

For more details, review the [JSON-RPC](./json-rpc.md) document.

## Software Libraries for creating an ACP Client

- [Typescript Libraries](./typescript-libraries.md)
- [Rust Libraries](./rust-crates.md)


## Support for ACP

ACP is a relatively new standard but it has a lot of support already across both editors (clients) and Agentic providers (services).

- Read the document [Who Supports ACP](./who-supports-acp.md) for a detailed view of major providers already support ACP


### Agentic Use Cases

Using Rust as a programming language, the following documents provide details on how to interact with various Agentic CLI's
