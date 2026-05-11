---
prompt: |-
	Your task is to research the Language Server Protocol (LSP) and write your findings to the body of this document.

    Your research must answer the questions:

    - what is the history of LSP?
    - what major versions of the LSP specification exist? When was the latest one released?
    - describe the architecture of LSP's and give an example
    - what are some of the problems/gotchas that people often encounter when they 
last_updated: 2026-04-16
---
# Language Server Protocol

## History

The Language Server Protocol (LSP) was originally developed by Microsoft for **Visual Studio Code**, first appearing in 2015. Anders Hejlsberg, the lead architect of TypeScript, drove the vision: instead of every editor reimplementing language intelligence (autocomplete, go-to-definition, diagnostics, refactoring) per language, a single **Language Server** process could provide those features to any editor through a standardized protocol.

Before LSP, adding language support to an editor required writing a deeply integrated plugin using that editor's specific extension API. If you wanted Go support in VS Code, Emacs, Vim, and Eclipse, you wrote four separate integrations. LSP inverted this: write one language server, and any editor that speaks LSP can consume it.

Key milestones:

- **2015**: Microsoft introduces LSP with the initial release of VS Code. The first language servers were for TypeScript, CSS, and JSON.
- **June 27, 2016**: Microsoft announces a collaboration with **Red Hat** and **Codenvy** to standardize the protocol as an open standard, hosted on GitHub. This was the moment LSP moved from a VS Code implementation detail to an industry-wide initiative.
- **2016-2017**: The specification is refined through community input on GitHub. Editors like Emacs (via `lsp-mode`), Vim/Neovim (via `coc.nvim` and later built-in LSP), and Eclipse adopt the protocol.
- **2018-2020**: Rapid adoption across the industry. Language servers appear for most major languages: `rust-analyzer` (Rust), `gopls` (Go), `pyright` and `pylsp` (Python), `clangd` (C/C++), `typescript-language-server` (TypeScript), and many more.
- **2020s**: LSP becomes the de facto standard for language intelligence. Neovim builds native LSP client support directly into the editor (0.5+, 2021). Microsoft introduces the **Language Server Index Format (LSIF)** for pre-computed code navigation without a running server.

By the early 2020s, LSP had become what Gunasinghe and Marcus (2021) describe as the "norm" for language intelligence tool providers.

## Specification Versions

The LSP specification has evolved through a series of minor-version releases, all under the major version **3.x**. There was never a formal "1.0" or "2.0" release of the modern JSON-RPC-based protocol — the early VS Code protocol was retroactively called version 2.0, and version 3.0 was the first standardized release.

| Version             | Key Features                                                                                                                | Status               |
|---------------------|-----------------------------------------------------------------------------------------------------------------------------|----------------------|
| **2.0**             | Original VS Code protocol (retroactively named)                                                                             | Obsolete             |
| **3.0**             | First standardized release; basic requests, notifications, document sync                                                    | Obsolete             |
| **3.1** - **3.5**   | Incremental improvements: color providers, folding range, workspace folders                                                 | Obsolete             |
| **3.6** - **3.9**   | Code actions, call hierarchy, semantic highlighting proposals                                                               | Obsolete             |
| **3.10** - **3.13** | Hierarchical document symbols, linked editing                                                                               | Obsolete             |
| **3.14**            | Call hierarchy, semantic tokens (finalized), inline values                                                                  | Previous             |
| **3.15**            | Progress support (`$/progress`), diagnostic tags, work done progress                                                        | Previous             |
| **3.16**            | Semantic tokens, change annotations, pull diagnostics, `AnnotatedTextEdit`                                                  | Previous             |
| **3.17**            | Type hierarchy, inline values, inlay hints, notebook document support, position encodings (UTF-8/UTF-16/UTF-32), meta model | **Current (stable)** |
| **3.18**            | `SnippetTextEdit`, folding range refresh, diagnostics refresh, text document content requests, improved patterns            | **Upcoming (draft)** |

### Current Version

The **current stable specification is 3.17**. The **upcoming draft is 3.18**, which is under active development on GitHub. There is also a separate **Base Protocol 0.9** specification being developed, and the **LSIF (Language Server Index Format)** is at version 0.6.0.

Note: the LSP specification does not use explicit release dates. Versions are tagged as they are finalized on GitHub. Version 3.17 was finalized around **mid-2022**, and 3.18 is still in development as of 2026.

## Architecture

### Overview

LSP follows a **client-server architecture** using **JSON-RPC 2.0** over an arbitrary transport (typically stdio pipes, but also TCP sockets or other IPC mechanisms).

```text
+-------------------+        JSON-RPC 2.0        +-------------------+
|   Editor / IDE    |  <---------------------->  |  Language Server  |
|   (LSP Client)    |      (stdio / socket)       |                   |
+-------------------+                              +-------------------+
```

### Core Components

1. **LSP Client** — runs inside the editor. It tracks open documents, user cursor position, and dispatches requests to the language server. It also handles responses and renders results (completions, diagnostics, hovers, etc.).
2. **Language Server** — a standalone process that analyzes source code. It maintains an internal representation of the codebase (AST, type information, symbol index) and responds to client requests.
3. **Transport Layer** — carries JSON-RPC messages. The default is **stdio** (the editor spawns the server as a child process and communicates via stdin/stdout). Alternatively, TCP sockets or other IPC can be used.
4. **Protocol Messages** — three types:

    - **Requests**: client → server (expects a response). Example: `textDocument/definition`
    - **Notifications**: either direction (no response). Example: `textDocument/didOpen`, `textDocument/publishDiagnostics`
    - **Responses**: server → client (reply to a request). Contains `result` on success or `error` on failure.

### Message Format

Messages use an HTTP-like header + body format:

```text
Content-Length: 1234\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"textDocument/completion","params":{...}}
```

The `Content-Length` header is mandatory. The body is a JSON-RPC 2.0 message.

### Lifecycle

The protocol follows a strict lifecycle:

1. **Initialize** (`initialize` request) — Client sends its capabilities and the server root URI. Server responds with its own capabilities. This is the **capability negotiation** phase.
2. **Initialized** (`initialized` notification) — Client confirms initialization is complete.
3. **Document Synchronization** — Client sends `textDocument/didOpen`, `textDocument/didChange`, `textDocument/didClose` notifications to keep the server in sync with open files.
4. **Feature Requests** — Client sends requests like `textDocument/completion`, `textDocument/hover`, `textDocument/definition`, etc.
5. **Shutdown** (`shutdown` request) — Client asks server to shut down gracefully.
6. **Exit** (`exit` notification) — Client tells server to exit. Server must exit with code 0 if shutdown was received, or code 1 otherwise.

### Example: Go to Definition

Here is a concrete example of the "Go to Definition" flow:

**1. User opens a file:**

```json
// Client → Server (notification)
{
  "jsonrpc": "2.0",
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "uri": "file:///home/user/src/main.rs",
      "languageId": "rust",
      "version": 1,
      "text": "fn main() {\n    println!(\"hello\");\n}\n"
    }
  }
}
```

**2. User triggers "Go to Definition" on `println`:**

```json
// Client → Server (request)
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "textDocument/definition",
  "params": {
    "textDocument": { "uri": "file:///home/user/src/main.rs" },
    "position": { "line": 1, "character": 4 }
  }
}
```

**3. Server responds with the definition location:**

```json
// Server → Client (response)
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "uri": "file:///home/user/.rustup/toolchains/stable/lib/rustlib/src/rust/library/std/src/macros.rs",
    "range": {
      "start": { "line": 100, "character": 0 },
      "end": { "line": 100, "character": 50 }
    }
  }
}
```

The client then navigates the editor to that file and range.

### Capability Negotiation

During initialization, both sides declare what they support:

- **Client capabilities**: e.g., "I support snippets in completions", "I can handle annotated text edits", "My position encoding is UTF-16"
- **Server capabilities**: e.g., "I support textDocument/completion", "I support textDocument/definition", "I can provide diagnostics"

This allows servers to degrade gracefully when clients lack features, and vice versa.

### Multi-Language Support

When a user works with multiple languages, the editor typically starts a separate language server process for each language. For example, a workspace with `.rs`, `.js`, and `.css` files would spawn `rust-analyzer`, `typescript-language-server`, and the CSS language server as three independent processes.

## Common Problems and Gotchas

### UTF-16 Position Encoding

Historically, LSP mandated UTF-16 code units for character offsets. This means characters outside the Basic Multilingual Plane (BMP) — such as many emoji, CJK extensions, or mathematical symbols — count as **two** positions. A string like `"a🦀b"` has the crab at position 1, but `b` is at position 3, not 2. This has been a persistent source of bugs in both clients and servers. Version 3.17 added negotiable position encodings (UTF-8, UTF-16, UTF-32), but backward compatibility means UTF-16 remains the default and all implementations must still handle it.

### Document Synchronization Desync

The client is the source of truth for open document content. The server learns about changes via `textDocument/didChange` notifications. If notifications are lost, reordered, or if the server processes them out of order, the server's internal document state can diverge from the client's. This leads to incorrect diagnostics, wrong completions, and phantom errors. Servers must carefully track document versions and handle incremental updates correctly.

### URI Encoding Inconsistencies

URIs are used to identify documents, but different clients encode URIs differently. For example, VS Code may encode a Windows path as `file:///c%3A/project/readme.md` while another client sends `file:///c:/project/readme.md`. Case sensitivity of drive letters is also inconsistent. Servers must normalize URIs or risk treating the same file as two different documents.

### Initialization Ordering

The protocol requires that the `initialize` request is the **first** message sent. If a client sends any other request or notification before `initialize` completes, the server may reject it with `ServerNotInitialized` (-32002). Race conditions in client implementations sometimes violate this ordering, especially when a server is slow to start.

### Cancellation Semantics

Request cancellation (`$/cancelRequest`) is a notification, not a guarantee. The server may have already computed the result by the time it receives the cancellation. Additionally, a cancelled request **must still return a response** — the server cannot simply drop it. The response should use `ErrorCodes.RequestCancelled` (-32800), but servers must be careful not to leave responses hanging.

### Version Skew and Capability Gating

The spec evolves frequently, and different clients/servers may implement different versions. A server built for 3.17 may send inlay hints to a client that only supports 3.15. The capability system is supposed to prevent this, but implementation bugs are common. Both sides must rigorously check capabilities before using features.

### Performance at Scale

Language servers for large codebases (e.g., `rust-analyzer` for the Rust compiler itself, or `clangd` for LLVM) can consume significant memory and CPU. The incremental synchronization model helps, but initial project indexing can take minutes. Servers must implement sophisticated caching and background indexing to remain responsive.

### One Server Per Client

The protocol assumes a 1:1 relationship between client and server. Sharing a single language server process between multiple editor instances or multiple tools is not supported. Each editor window typically spawns its own server process, doubling memory usage.

### Text Edit Ordering

TextEdit arrays describe changes from a single document state (S1 → S2). Edits must not overlap, but multiple edits can share the same start position (insertions). The order in the array determines the final text. Misunderstanding this "no intermediate state" model leads to incorrect edit application, especially when combining multiple edits from different features.

### Notebook Document Complexity

Version 3.17 added notebook document support, which significantly increased protocol complexity. Notebook documents have cells with their own language IDs, outputs, and metadata. Properly synchronizing notebook state requires handling nested document structures, and many servers still do not support this feature.

### Partial Results and Streaming

The protocol supports partial result progress for long-running requests (e.g., symbol search across a large workspace). Implementing this correctly requires careful token management and understanding of the `$/progress` notification lifecycle. Many implementations skip this feature entirely, leading to "all or nothing" behavior on large queries.

### No Built-in Authentication or Security

LSP has no authentication, encryption, or authorization mechanism. The transport is assumed to be local (stdio or localhost). Running a language server over a network socket without additional security layers is a security risk. This is rarely a problem in practice but matters for remote development scenarios.

## References

- [LSP Official Website](https://microsoft.github.io/language-server-protocol/)
- [LSP Specification 3.17 (Current)](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
- [LSP Specification 3.18 (Upcoming)](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/)
- [LSP GitHub Repository](https://github.com/microsoft/language-server-protocol)
- [LSP Overview](https://microsoft.github.io/language-server-protocol/overviews/lsp/overview)
- [Wikipedia: Language Server Protocol](https://en.wikipedia.org/wiki/Language_Server_Protocol)
- Gunasinghe, N.; Marcus, N. (2021). *Language Server Protocol and Implementation*. Apress. ISBN 978-1-4842-7791-1.
