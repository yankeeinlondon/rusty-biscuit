---
prompt: |-
	Your task is to research crates which help Rust authors create an LSP and write your findings to the body of this document.
    
    - what features does the crate expose?
        - when should you use (and not use) each feature?
    - what is the functional footprint and goal of this crate?
    - provide a very simple Rust code example of creating a bespoke LSP
    - describe how easily this crate could be used to _extend_ an existing LSP implementation and how that would work
        - use `Markdown` as the example language you're focusing on

last_updated: 2026-04-16
---
# Rust LSP Crates

## Crate Landscape

| Crate              | Approach                    | Async | Latest             | Maintenance |
|--------------------|-----------------------------|-------|--------------------|-------------|
| `tower-lsp`        | Trait-based, opinionated    | Yes   | v0.20.0 (Aug 2023) | Stalled     |
| `tower-lsp-server` | Community fork of tower-lsp | Yes   | v0.23.0 (Dec 2025) | Active      |
| `async-lsp`        | Tower Layer middleware      | Yes   | v0.2.3 (Mar 2026)  | Active      |
| [`lsp-server`](./tower-lsp.md)       | Sync crossbeam scaffold     | No    | v0.7.9 (Aug 2025)  | Active      |
| `lsp-types`        | Type definitions only       | N/A   | v0.97.0            | Active      |
| `lspower`          | Fork of tower-lsp           | Yes   | v1.5.0 (Dec 2021)  | Archived    |

---

## `tower-lsp` and `tower-lsp-server`

### Goal

`tower-lsp` provides a complete, ergonomic framework for building LSP servers on top of the Tower service abstraction. You implement a single `LanguageServer` trait and the framework handles JSON-RPC routing, transport, lifecycle, and concurrency. `tower-lsp-server` is an actively maintained community fork with an identical API.

### Functional Footprint

- **`LanguageServer` trait** — 63 methods covering ~89% of LSP 3.0–3.17. Only `initialize` and `shutdown` are required; everything else has default no-op implementations.
- **`LspService`** — a `tower::Service<Request>` that routes incoming JSON-RPC messages to your trait methods.
- **`Client`** — cheaply cloneable handle for server-to-client communication (publish diagnostics, send notifications, request workspace edits, etc.).
- **`Server`** — transport layer generic over `AsyncRead + AsyncWrite`, defaults to stdio.
- **`LspServiceBuilder`** — registers custom JSON-RPC methods beyond the LSP spec.
- **`jsonrpc` module** — re-exports `Error`, `Result<T>`, `Response`, `Request`, `Id`, `ErrorCode`.

### Features

| Feature            | Default | When to use                                                                       | When not to use                                |
|--------------------|---------|-----------------------------------------------------------------------------------|------------------------------------------------|
| `runtime-tokio`    | Yes     | Standard tokio-based servers                                                      | If using async-std, smol, or targeting WASM    |
| `runtime-agnostic` | No      | Non-tokio runtimes or WASM targets                                                | If already using tokio (adds unnecessary deps) |
| `proposed`         | No      | You need bleeding-edge LSP 3.18 proposed features and accept no semver guarantees | Production servers that need stable APIs       |

### When to Use

- You want the fastest path to a working LSP server.
- You prefer an async, trait-driven API where you only implement the handlers you care about.
- You want built-in concurrency control, `$/cancelRequest` support, and transport abstraction out of the box.
- You want access to a `Client` handle for server-to-client calls without manual plumbing.

### When Not to Use

- You need synchronous message processing (e.g., integrating with a single-threaded parser).
- You need fine-grained control over the dispatch loop or want to avoid the Tower abstraction overhead.
- You need to build a language *client*, not just a server.
- You need correct in-order notification processing (tower-lsp processes notifications concurrently, which can cause ordering issues).

---

## `async-lsp`

### Goal

`async-lsp` takes a Tower Layer middleware approach to LSP. Where tower-lsp gives you a single monolithic trait, async-lsp gives you composable middleware layers for concurrency, lifecycle, panic handling, and tracing. It supports building both servers and clients.

### Functional Footprint

- **`LspService` trait** — core dispatch abstraction, not a concrete trait with 63 methods.
- **`Router`** — builder API for registering handlers for specific LSP methods, or an omnitrait via `LanguageServer` / `LanguageClient`.
- **`MainLoop`** — driver that processes messages.
- **`ServerSocket` / `ClientSocket`** — communication handles.
- **Middleware modules**: `concurrency`, `panic`, `tracing`, `server::Lifecycle`, `client_monitor`.

### When to Use

- You need correct in-order notification processing (notifications are processed synchronously, fixing tower-lsp's ordering bug).
- You want true Tower `Layer` middleware composition.
- You want `&mut self` handlers (avoids async locks for state mutation).
- You need to build a language client, not just a server.
- You want 100% documented API surface.

### When Not to Use

- You want the simplest possible API surface and don't need middleware composition.
- You want ergonomic helpers like `Client::show_message` (async-lsp is lower-level).
- You are migrating from tower-lsp and want minimal code changes (use `tower-lsp-server` instead).

---

## `lsp-server` + `lsp-types`

### Goal

`lsp-server` is the transport scaffold extracted from rust-analyzer. It provides synchronous crossbeam-channel-based I/O with typed message parsing. You own the main dispatch loop. `lsp-types` is the shared type definition library used by nearly every Rust LSP crate, covering 250+ types across LSP 3.0–3.17.

### Functional Footprint — `lsp-server`

- **`Connection`** — pair of `crossbeam-channel` endpoints for message passing.
- **`Message` enum** — `Request(Request)` / `Response(Response)` / `Notification(Notification)`.
- **`Request::extract::<P>(method)`** — deserialize typed params, returning `Result<(RequestId, P), ExtractError>`.
- **Initialize/shutdown helpers** — `connection.initialize()`, `connection.handle_shutdown()`.
- **`Connection::memory()`** — in-memory channel pairs for testing without I/O.
- **`IoThreads`** — handles for stdio/TCP I/O threads (joined on drop).

### Functional Footprint — `lsp-types`

- **250+ structs and enums** covering every LSP data type: `Position`, `Range`, `Diagnostic`, `CompletionItem`, `Hover`, `WorkspaceEdit`, `SemanticTokens`, `InlayHint`, etc.
- **`request` module** — 70+ request types with `Request` trait (`METHOD`, `Params`, `Result`).
- **`notification` module** — 26 notification types with `Notification` trait (`METHOD`, `Params`).
- **Feature flag `proposed`** — enables unstable 3.17+ features.

### Key Design: Separation of Concerns

`lsp-server` handles **how** to communicate (transport, framing, channels). `lsp-types` defines **what** to communicate (typed structures). They are bridged via the `Request::METHOD` constant and `.extract::<P>(method)` pattern. Notably, `lsp-server` does **not** depend on `lsp-types` — it works entirely with `serde_json::Value`.

### When to Use

- You need synchronous, single-threaded processing.
- You want maximum control over the dispatch loop (e.g., custom scheduling, priority queues).
- You are building something rust-analyzer-adjacent.
- You want the most battle-tested stack (rust-analyzer is the reference Rust LSP).

### When Not to Use

- You want async handlers and tokio integration.
- You want automatic trait-based dispatch (you must manually match on method strings).
- You want built-in concurrency control or cancellation.

---

## `lsp-types` (Shared Dependency)

### Goal

Pure type definitions for the Language Server Protocol. No I/O, no channels, no networking. Used by tower-lsp, tower-lsp-server, async-lsp, lsp-server, and virtually every other Rust LSP project.

### Features

| Feature    | When to use                                                    |
|------------|----------------------------------------------------------------|
| Default    | Stable LSP 3.16 types — suitable for all production servers    |
| `proposed` | Experimental/proposed LSP 3.17+ features; no semver guarantees |

### Key Modules

- `lsp_types::request` — all request types with typed params/results
- `lsp_types::notification` — all notification types with typed params
- `lsp_types::*` — `ServerCapabilities`, `ClientCapabilities`, `Diagnostic`, `CompletionItem`, `Hover`, `Location`, `TextEdit`, `WorkspaceEdit`, `SemanticTokens`, `InlayHint`, etc.

### When Not to Use

- You only need JSON-RPC and have no use for LSP-specific types.
- You are defining a custom protocol that doesn't follow the LSP spec.

---

## Simple Example: A Bespoke Markdown LSP

The following example uses `tower-lsp-server` (the actively maintained fork of `tower-lsp`) to build a minimal Markdown language server that provides hover information and document diagnostics.

```toml
# Cargo.toml
[dependencies]
tower-lsp-server = "0.23"
tokio = { version = "1", features = ["rt-multi-thread", "io-std", "macros"] }
serde_json = "1"
```

```rust
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};
use std::sync::Mutex;

struct MarkdownLsp {
    client: Client,
    documents: Mutex<std::collections::HashMap<Url, String>>,
}

#[tower_lsp_server::async_trait]
impl LanguageServer for MarkdownLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Markdown LSP initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.publish_markdown_diagnostics(&uri, &text).await;
        self.documents.lock().unwrap().insert(uri, text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.content_changes[0].text.clone();
        self.publish_markdown_diagnostics(&uri, &text).await;
        self.documents.lock().unwrap().insert(uri, text);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.documents.lock().unwrap();
        let Some(text) = docs.get(uri) else { return Ok(None) };

        let line = text.lines().nth(pos.line as usize);
        let Some(line_text) = line else { return Ok(None) };

        if line_text.contains("# ") {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Markdown heading".into(),
                }),
                range: Some(Range {
                    start: Position { line: pos.line, character: 0 },
                    end: Position { line: pos.line, character: line_text.len() as u32 },
                }),
            }));
        }
        Ok(None)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl MarkdownLsp {
    async fn publish_markdown_diagnostics(&self, uri: &Url, text: &str) {
        let mut diagnostics = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("# ") && !line.ends_with("\n") && i == 0 {
                // example: warn if first heading is not followed by a blank line
            }
            let heading_count = line.chars().take_while(|c| *c == '#').count();
            if heading_count > 6 {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: format!("Heading level {} exceeds maximum of 6", heading_count),
                    source: Some("markdown-lsp".into()),
                    ..Default::default()
                });
            }
        }
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| MarkdownLsp {
        client,
        documents: Mutex::new(std::collections::HashMap::new()),
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
```

---

## Extending an Existing LSP Implementation

There are three strategies for extending an existing LSP — shown here with Markdown as the target language.

### Strategy 1: Custom JSON-RPC Methods via `LspServiceBuilder`

Use this when you need to add proprietary extensions that aren't part of the LSP spec. Both `tower-lsp` and `tower-lsp-server` support this through the builder pattern.

```rust
let (service, socket) = LspService::build(|client| MarkdownLsp { client })
    .custom_method("markdown/validateLinks", MarkdownLsp::validate_links)
    .custom_method("markdown/toc", MarkdownLsp::table_of_contents)
    .finish();
```

Handlers returning `jsonrpc::Result<T>` are treated as requests; handlers returning `()` are notifications. This lets you add capabilities without modifying the core `LanguageServer` trait implementation.

### Strategy 2: Tower Middleware Layers

Use this when you want cross-cutting behavior — logging, metrics, request transformation — applied uniformly to all LSP messages. Since `LspService` implements `tower::Service<Request>`, you can wrap it with any Tower middleware.

```rust
use tower::ServiceBuilder;
use tower_lsp_server::LspService;

let (service, socket) = LspService::new(|client| MarkdownLsp { client });

let layered = ServiceBuilder::new()
    .layer(tower::layer::layer_fn(|inner| LoggingMiddleware { inner }))
    .service(service);
```

This approach works well for:

- Adding tracing/metrics to every request
- Rate limiting expensive operations (e.g., semantic token computation for large Markdown files)
- Injecting shared context (e.g., a Markdown parser instance) into every handler

### Strategy 3: Delegation / Wrapper Pattern

Use this when you want to wrap an existing LSP and selectively override behavior. For example, wrapping a general-purpose Markdown LSP to add project-specific lint rules.

```rust
struct ExtendedMarkdownLsp {
    inner: BaseMarkdownLsp,
    client: Client,
}

#[tower_lsp_server::async_trait]
impl LanguageServer for ExtendedMarkdownLsp {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut result = self.inner.initialize(params).await?;
        // Add extra capabilities on top of the base server
        result.capabilities.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
        Ok(result)
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.inner.did_change(params.clone()).await;
        // Add additional processing on top of the base implementation
        self.run_custom_lint_rules(&params.text_document.uri).await;
    }

    // Delegate everything else unchanged
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.inner.hover(params).await
    }

    async fn shutdown(&self) -> Result<()> {
        self.inner.shutdown().await
    }
}
```

This pattern is particularly useful when:

- You have an existing Markdown LSP (e.g., one using `pulldown-cmark` for parsing) and want to add project-specific validation rules without forking
- You want to compose multiple LSP backends — e.g., a Markdown LSP that also delegates certain queries to a spell-checker LSP
- You need to intercept and modify responses from the base server (e.g., adding custom diagnostics to the ones produced by the base server)

### Strategy 4: Router-Based Composition with `async-lsp`

If you use `async-lsp` instead of tower-lsp, you get more granular composition via the `Router` builder. You can register handlers for individual methods and stack middleware layers independently.

```rust
use async_lsp::{Router, LanguageServer};

let router = Router::new()
    .request::<lsp_types::request::HoverRequest>(|state, params| {
        // Custom Markdown hover logic
    })
    .notification::<lsp_types::notification::DidChangeTextDocument>(|state, params| {
        // Custom change handling
    })
    // Delegate unhandled methods to a wrapped inner server
    .fallback(inner_service);
```

This gives you per-method middleware control and makes it straightforward to compose multiple services.

---

## Choosing Between Crates

### Decision Guide

```text
Do you need async?
├── Yes
│   ├── Do you want the simplest API?
│   │   └── Use tower-lsp-server (active fork of tower-lsp)
│   ├── Do you need correct notification ordering or middleware composition?
│   │   └── Use async-lsp
│   └── Are you extending an existing tower-lsp project?
│       └── Use tower-lsp-server (drop-in replacement)
└── No (or you want sync)
    └── Use lsp-server + lsp-types
```

### For a Markdown LSP specifically

A Markdown LSP typically needs: document synchronization, diagnostics (lint rules), hover (heading/link info), document symbols (heading outline), completion (link targets), and code actions (fix common issues). Any of these crates can handle this workload. The choice depends on your architectural preference:

- **tower-lsp-server** — best for getting started quickly; the trait-based API maps naturally onto "I need hover, diagnostics, and symbols for Markdown files."
- **async-lsp** — best if you plan to compose the Markdown LSP with other services (e.g., a spell checker, a link validator) via middleware layers.
- **lsp-server** — best if you have a synchronous Markdown parser (e.g., `pulldown-cmark` in a single-threaded context) and want to avoid async runtime overhead.
