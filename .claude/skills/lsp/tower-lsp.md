---
prompt: |-
    Do a deep dive on the `tower-lsp` crate:

    - provide a full description of it's functional footprint
    - describe the architecture that `tower-lsp` promotes
        - what is it's relationship to tower? 
        - what are the major feature areas or modules?
    - list all the feature flags which are exposed and what each feature flag provides
    - what "gotchas" do developers report running into when using tower-lsp? How can these obstacles be avoided?
    - provide 3-4 code examples demonstrating common use cases
last_updated: 2026-05-02
---
# `tower-lsp`

`tower-lsp` is a Rust framework for building Language Server Protocol servers. Its core value proposition is: implement a typed async `LanguageServer` trait, return your capabilities during `initialize`, and let the crate handle JSON-RPC framing, routing, cancellation, transport, and server-to-client communication.

The current published crate is `tower-lsp` `0.20.0`. It depends on `lsp-types` `0.94.1`, `tower` `0.4`, `tokio` by default, and exposes LSP 3.17-era APIs with an optional `proposed` flag for unstable proposed LSP types. The upstream project’s own feature matrix reports roughly 89% method coverage across LSP 3.0-3.17, with notable unsupported or partial areas called out below.

Primary references:

- [`tower-lsp` docs.rs](https://docs.rs/tower-lsp/latest/tower_lsp/)
- [`tower-lsp` feature flags on docs.rs](https://docs.rs/crate/tower-lsp/latest/features)
- [`tower-lsp` README](https://github.com/ebkalderon/tower-lsp)
- [`tower-lsp` `Cargo.toml`](https://github.com/ebkalderon/tower-lsp/blob/master/Cargo.toml)
- [`tower-lsp` feature coverage matrix](https://github.com/ebkalderon/tower-lsp/blob/master/FEATURES.md)

## Functional Footprint

`tower-lsp` gives you a high-level server framework, not just protocol types. Its main responsibilities are:

- JSON-RPC 2.0 transport framing for LSP messages.
- Request, response, and notification dispatch.
- A typed async `LanguageServer` trait covering most common LSP server methods.
- A cloneable `Client` handle for server-to-client requests and notifications.
- A `tower::Service` implementation for server-side request handling.
- A `Server` runner for stdio, TCP, or any compatible async reader/writer transport.
- Support for custom JSON-RPC request and notification methods through `LspService::build`.
- A re-export of `lsp_types` so consumers can use `tower_lsp::lsp_types::*`.
- A re-export of `async_trait` as `tower_lsp::async_trait`.

The crate is primarily for building language servers. It is not a general LSP client framework, and it does not provide a parser, compiler front end, indexer, text rope, virtual file system, or project model. You bring your own language analysis engine and store that state inside your backend.

The public surface centers on these items:

| Item                  | Role                                                                                                                                                                                        |
|-----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `LanguageServer`      | Trait implemented by your backend. `initialize` and `shutdown` are required; the rest have default no-op or empty-result implementations.                                                   |
| `Client`              | Cloneable handle for communicating back to the editor/client. Used for diagnostics, messages, configuration requests, workspace edits, refresh requests, and custom requests/notifications. |
| `LspService`          | Tower service that routes JSON-RPC requests into your `LanguageServer` implementation.                                                                                                      |
| `LspServiceBuilder`   | Builder used to register custom JSON-RPC methods before constructing the service.                                                                                                           |
| `Server`              | Transport driver that reads/writes LSP messages over stdio, TCP, or compatible streams.                                                                                                     |
| `ClientSocket`        | Loopback channel used by `Server` for server-to-client communication.                                                                                                                       |
| `jsonrpc` module      | Subset of JSON-RPC types used by LSP, including `Request`, `Response`, `Id`, `Error`, `ErrorCode`, and `Result`.                                                                            |
| `lsp_types` re-export | The protocol type definitions used in method params and return values.                                                                                                                      |
| `async_trait` macro   | Convenience re-export for implementing async trait methods.                                                                                                                                 |

The `LanguageServer` trait has 63 methods in the current docs. Required methods:

- `initialize`
- `shutdown`

Major provided method groups include:

- Lifecycle: `initialized`, `shutdown`
- Text sync: `did_open`, `did_change`, `did_save`, `did_close`, `will_save`, `will_save_wait_until`
- Navigation: `goto_definition`, `goto_declaration`, `goto_type_definition`, `goto_implementation`, `references`
- Editor intelligence: `completion`, `completion_resolve`, `hover`, `signature_help`
- Symbols and structure: `document_symbol`, `symbol`, `workspace_symbol_resolve`, `folding_range`, `selection_range`
- Diagnostics: `diagnostic`, `workspace_diagnostic`
- Code actions and edits: `code_action`, `code_action_resolve`, `rename`, `prepare_rename`, `formatting`, `range_formatting`, `on_type_formatting`
- Rich editor features: semantic tokens, inlay hints, inline values, code lenses, document links, document colors, linked editing ranges
- Workspace events: configuration changes, watched files, workspace folders, file create/rename/delete events
- Call/type hierarchy: call hierarchy and type hierarchy requests
- Commands: `execute_command`

The `Client` type covers server-to-client operations such as:

- `publish_diagnostics`
- `log_message`
- `show_message`
- `show_message_request`
- `show_document`
- `apply_edit`
- `configuration`
- `workspace_folders`
- `register_capability`
- `unregister_capability`
- `semantic_tokens_refresh`
- `code_lens_refresh`
- `inlay_hint_refresh`
- `inline_value_refresh`
- `workspace_diagnostic_refresh`
- `send_request`
- `send_notification`

## Architecture

`tower-lsp` promotes a trait-backed, service-driven architecture:

```text
Editor / LSP Client
        |
        | JSON-RPC 2.0 over stdio/TCP/etc.
        v
tower_lsp::Server
        |
        | framed Request / Response / Notification stream
        v
tower_lsp::LspService
        |
        | dispatch by LSP method name
        v
Your LanguageServer implementation
        |
        | server-to-client calls
        v
tower_lsp::Client
```

Your backend is usually a struct containing:

- A `Client`
- Document state
- Workspace/project state
- Parser/compiler/indexing state
- Configuration
- Background task handles or channels

Because `LanguageServer` methods take `&self`, mutable state is normally stored behind interior mutability, such as `Arc<RwLock<_>>`, `Arc<Mutex<_>>`, `DashMap`, channels, or task-local actors. This is convenient for async request handling, but it also means developers need to design state access carefully.

A typical backend looks like:

```rust
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    index: Arc<RwLock<ProjectIndex>>,
}
```

### Relationship To Tower

`tower-lsp` is based on Tower’s `Service` abstraction. Tower’s `Service<Request>` trait models an asynchronous request/response unit:

```rust
Service<Request> -> Future<Output = Result<Response, Error>>
```

`tower-lsp` maps LSP/JSON-RPC traffic into that model:

- `LspService` implements `tower::Service<Request>`.
- `Client` also implements `tower::Service<Request>`.
- The service layer makes the protocol dispatch transport-independent.
- In principle, Tower middleware can wrap service calls, though most `tower-lsp` users interact through the higher-level `LanguageServer` trait rather than hand-building Tower stacks.

This is the architectural split:

| Layer            | Responsibility                                                |
|------------------|---------------------------------------------------------------|
| `Server`         | Transport loop, framed reads/writes, stdio/TCP-compatible IO. |
| `LspService`     | Tower service boundary and method dispatch.                   |
| `LanguageServer` | User-defined behavior for LSP methods.                        |
| `Client`         | Outbound server-to-client requests and notifications.         |
| `lsp_types`      | Typed protocol structs and enums.                             |
| `jsonrpc`        | JSON-RPC envelope and error types.                            |

The crate’s design is opinionated: it prefers one large trait with default method implementations over a router-first or middleware-first API. That makes simple servers very quick to write, but it can feel rigid when you need unusual routing, custom protocol extensions, precise scheduling, or unusual concurrency behavior.

## Major Feature Areas And Modules

### `LanguageServer`

The central extension point. You implement methods matching LSP requests and notifications. Only `initialize` and `shutdown` are mandatory.

Use this for normal LSP features:

- Completion
- Hover
- Diagnostics
- Go to definition
- Formatting
- Code actions
- Workspace symbols
- Semantic tokens
- Inlay hints

### `Client`

The server’s handle back to the editor. It is cheap to clone and can be moved into background tasks.

Use it for:

- Publishing diagnostics after parsing or analysis
- Logging messages to the LSP client
- Asking the client for configuration
- Applying workspace edits
- Dynamically registering capabilities
- Sending custom requests and notifications

### `LspService`

The Tower service wrapping your backend. Most users create it with:

```rust
let (service, socket) = LspService::new(|client| Backend { client });
```

For custom methods, use:

```rust
let (service, socket) = LspService::build(|client| Backend { client })
    .custom_method("custom/request", Backend::custom_request)
    .finish();
```

### `Server`

The runtime driver. It takes async input/output streams plus a `ClientSocket`, then serves an `LspService`.

Common stdio shape:

```rust
let stdin = tokio::io::stdin();
let stdout = tokio::io::stdout();
Server::new(stdin, stdout, socket).serve(service).await;
```

### `jsonrpc`

Contains the crate’s JSON-RPC envelope types. You normally only import `tower_lsp::jsonrpc::Result`, but custom method work may need lower-level request/response/error types.

### `lsp_types`

Re-export of the `lsp-types` crate. Most method params, results, capabilities, diagnostics, ranges, positions, edits, and request marker types come from here.

### `tower-lsp-macros`

Internal companion crate used to generate much of the method dispatch machinery.

## Feature Flags

The published `0.20.0` crate exposes these Cargo-selectable features.

| Feature            | Default                    | Provides                                                                                                                                                                                   |
|--------------------|---------------------------:|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `default`          | Yes                        | Enables `runtime-tokio`.                                                                                                                                                                   |
| `runtime-tokio`    | Yes                        | Enables Tokio-based transport support by turning on the optional `tokio` and `tokio-util` dependencies. This is the normal choice for CLI language servers.                                |
| `runtime-agnostic` | No                         | Enables `async-codec-lite` instead of Tokio-specific codec support. Use when integrating with a non-Tokio runtime or a more custom async environment. Requires `default-features = false`. |
| `proposed`         | No                         | Enables `lsp-types/proposed`, exposing proposed LSP types. The README warns that proposed features have no semver guarantees and may break between releases.                               |
| `tokio`            | Yes, via `runtime-tokio`   | Optional dependency feature for `tokio`. Usually do not enable directly; prefer `runtime-tokio`.                                                                                           |
| `tokio-util`       | Yes, via `runtime-tokio`   | Optional dependency feature for `tokio-util` with codec support. Usually do not enable directly; prefer `runtime-tokio`.                                                                   |
| `async-codec-lite` | No, via `runtime-agnostic` | Optional dependency feature used by runtime-agnostic transport support. Usually do not enable directly; prefer `runtime-agnostic`.                                                         |

Normal Tokio dependency:

```toml
[dependencies]
tower-lsp = "0.20"
tokio = { version = "1", features = ["io-std", "macros", "rt-multi-thread"] }
```

Runtime-agnostic dependency:

```toml
[dependencies.tower-lsp]
version = "0.20"
default-features = false
features = ["runtime-agnostic"]
```

Proposed LSP types:

```toml
[dependencies]
tower-lsp = { version = "0.20", features = ["proposed"] }
```

## Protocol Coverage Notes

The upstream `FEATURES.md` matrix reports about `80.5/90`, or roughly `89.4%`, supported methods across tracked LSP versions.

Notable gaps and partial areas:

- Notebook document synchronization is not supported in the listed 3.17 matrix.
- `$/setTrace` and `$/logTrace` are listed as unsupported.
- `$/progress`, `window/workDoneProgress/create`, and `window/workDoneProgress/cancel` are listed as unsupported in the 3.15 section.
- `$/cancelRequest` is partial: client-to-server cancellation is implemented through async task cancellation, but server-to-client cancellation is not directly implemented. The matrix notes that raw notifications can be emitted manually with `Client::send_notification`.

## Gotchas And How To Avoid Them

### Handler Concurrency Can Reorder State Effects

The biggest architectural gotcha is concurrent handler execution. Upstream issue [\#284](https://github.com/ebkalderon/tower-lsp/issues/284) discusses this directly: `tower-lsp` buffers and executes pending tasks concurrently, which can create correctness hazards for stateful methods such as `didOpen`, `didChange`, and requests that depend on synchronized document state.

This matters because LSP document sync depends on order. `textDocument/didChange` notifications must be applied in the order received, and changes within a single notification must also be applied in order.

Avoid it by:

- Treating document synchronization methods as critical sections.
- Tracking document versions and ignoring stale updates.
- Keeping `did_open`, `did_change`, `did_close`, and similar methods short.
- Avoiding `.await` while holding a write lock on document state.
- Moving expensive analysis into background tasks that consume snapshots or queued work.
- Using an actor/task model for mutable project state if ordering matters.
- Considering `tower-lsp-server` or `async-lsp` if you need actively maintained concurrency fixes or different scheduling semantics.

### `&self` Methods Force Interior Mutability

All trait handlers use `&self`, so straightforward mutable fields do not work. Developers often discover this when trying to store open documents or counters directly in the backend.

Avoid it by using explicit state containers:

- `tokio::sync::RwLock<HashMap<Url, Document>>` for async shared document state.
- `DashMap` for concurrent maps with short operations.
- `mpsc` channels for an actor that owns mutable compiler/index state.
- `ArcSwap` or immutable snapshots for read-heavy analysis.

Do not hold a lock across calls back into the client unless you are certain no re-entrant request path can block on the same state.

### Long Work Inside Notifications Blocks Responsiveness

Developers have reported wanting a `tick` or background hook for expensive indexing/compilation work, as in issues [\#365](https://github.com/ebkalderon/tower-lsp/issues/365) and [\#432](https://github.com/ebkalderon/tower-lsp/issues/432). `tower-lsp` does not provide a scheduler for idle/background LSP work.

Avoid it by:

- Spawning background tasks from lifecycle or document events.
- Sending work over channels to a long-lived worker task.
- Publishing diagnostics asynchronously after analysis completes.
- Debouncing rapid `did_change` notifications.
- Using cancellation/version checks so old analysis results do not overwrite new ones.

### Calling The Client While Holding Locks Can Deadlock

Issue [\#386](https://github.com/ebkalderon/tower-lsp/issues/386) discusses cases where a server calls into the client while holding read/write state, and the client may call back into the server. Even if the spec discourages problematic ordering, real clients can behave differently.

Avoid it by:

- Copying out the data you need, dropping locks, then calling `self.client`.
- Keeping server-to-client calls outside `RwLock` or `Mutex` guards.
- Never awaiting client requests while holding exclusive project state.
- Designing client calls as separate phases after state mutation completes.

Risky pattern:

```rust
let mut docs = self.documents.write().await;
docs.insert(uri.clone(), text);
self.client.configuration(vec![]).await?; // Avoid: await while holding lock.
```

Safer pattern:

```rust
{
    let mut docs = self.documents.write().await;
    docs.insert(uri.clone(), text);
}

let config = self.client.configuration(vec![]).await?;
```

### Custom Methods Are Type-Sensitive

`LspServiceBuilder::custom_method` is useful, but the handler signature has to match what `tower-lsp`’s generated `Method` machinery expects. Issue [\#434](https://github.com/ebkalderon/tower-lsp/issues/434) shows a confusing compile error caused by non-`Send`/non-`Sync` async internals. Issue [\#409](https://github.com/ebkalderon/tower-lsp/issues/409) shows confusion around notifications with no params, where VS Code sent `null` params and deserialization did not match the expected no-param handler.

Avoid it by:

- Making custom params `Serialize + DeserializeOwned + Send + 'static`.
- Making any values captured by async custom methods `Send + Sync` unless you have isolated them behind a single-thread actor.
- Prefer typed params over `serde_json::Value` once the wire shape is known.
- For VS Code custom notifications, send an explicit object payload rather than no params or `null`.
- Use a dummy params struct if the client insists on sending a params field.

Example params shape:

```rust
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct CompileParams {
    project_id: String,
    rebuild: bool,
}
```

### Custom Methods Do Not Cleanly Override Built-In LSP Methods

Issue [\#393](https://github.com/ebkalderon/tower-lsp/issues/393) reports that registering a custom method with the same method name as a built-in LSP method, such as `textDocument/hover`, does not behave as a clean override path.

Avoid it by:

- Implementing built-in LSP methods through the `LanguageServer` trait whenever possible.
- Using custom method names for protocol extensions, such as `myServer/hoverWithActions`.
- Keeping editor-specific extensions separate from standard LSP method names.

### Pull Diagnostics Return Types Are Easy To Get Wrong

Newer pull diagnostic methods such as `diagnostic` and `workspace_diagnostic` return typed diagnostic report results, not `()`. Issue [\#425](https://github.com/ebkalderon/tower-lsp/issues/425) is an example of the compiler reporting an incompatible trait method type.

Avoid it by copying the exact signature from the rustdoc for your crate version:

```rust
async fn diagnostic(
    &self,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult> {
    // ...
}
```

### Server Process Exit Depends On Client Behavior

Issue [\#328](https://github.com/ebkalderon/tower-lsp/issues/328) discusses server processes remaining alive after VS Code exits. The LSP lifecycle expects a `shutdown` request followed by an `exit` notification, and `tower-lsp` also exits when stdin closes, but editor or parent-process behavior can still leave processes detached.

Avoid it by:

- Ensuring the editor extension sends `shutdown` and `exit`.
- Testing process cleanup in the target editor, not only from a shell.
- Considering a parent-process watcher if your distribution environment has known orphaning behavior.
- Keeping background tasks tied to server lifecycle and able to shut down promptly.

### `proposed` Has No Stability Guarantee

The `proposed` feature exposes unstable protocol types through `lsp-types/proposed`. The README explicitly warns that these APIs may break between releases.

Avoid it by:

- Using `proposed` only for experimental editor integrations.
- Gating your own code behind a feature flag too.
- Avoiding proposed protocol types in stable public APIs if your server is distributed broadly.

### Maintenance Cadence Matters

`tower-lsp` remains widely used, but the latest crates.io release is `0.20.0` from 2023. The repository has later open PRs for dependency updates, including `lsp-types`, `tower`, and Rust edition changes, but those are not published in `tower-lsp` as of the checked sources.

Avoid it by:

- Pinning known-good versions.
- Checking whether `tower-lsp-server`, the community fork, is a better fit for new work.
- Avoiding assumptions that latest `lsp-types` examples match `tower-lsp` `0.20.0`.
- Reading rustdoc for the exact crate version you depend on.

## Example 1: Minimal Stdio Server

```toml
[dependencies]
tower-lsp = "0.20"
tokio = { version = "1", features = ["io-std", "macros", "rt-multi-thread"] }
```

```rust
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "example-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(
                "Hello from tower-lsp".into(),
            )),
            range: None,
        }))
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("hello".into(), "Example completion".into()),
            CompletionItem::new_simple("goodbye".into(), "Another completion".into()),
        ])))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

## Example 2: Document Sync And Diagnostics

This example stores full document text and publishes a warning diagnostic for lines containing `TODO`.

```rust
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

impl Backend {
    async fn update_diagnostics(&self, uri: Url, version: Option<i32>, text: &str) {
        let diagnostics = text
            .lines()
            .enumerate()
            .filter_map(|(line, value)| {
                let start = value.find("TODO")?;

                Some(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line as u32,
                            character: start as u32,
                        },
                        end: Position {
                            line: line as u32,
                            character: (start + 4) as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("example-lsp".into()),
                    message: "TODO marker found".into(),
                    ..Default::default()
                })
            })
            .collect();

        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        let uri = doc.uri;
        let version = Some(doc.version);
        let text = doc.text;

        {
            let mut documents = self.documents.write().await;
            documents.insert(uri.clone(), text.clone());
        }

        self.update_diagnostics(uri, version, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let Some(change) = params.content_changes.into_iter().next() else {
            return;
        };

        {
            let mut documents = self.documents.write().await;
            documents.insert(uri.clone(), change.text.clone());
        }

        self.update_diagnostics(uri, Some(version), &change.text).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        {
            let mut documents = self.documents.write().await;
            documents.remove(&uri);
        }

        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
```

## Example 3: Background Analysis With Version Checks

This pattern keeps `did_change` fast, debounces analysis, and avoids publishing stale diagnostics.

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tower_lsp::Client;

#[derive(Clone, Debug)]
struct DocumentSnapshot {
    version: i32,
    text: String,
}

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentSnapshot>>>,
}

impl Backend {
    fn schedule_analysis(&self, uri: Url, snapshot: DocumentSnapshot) {
        let client = self.client.clone();
        let documents = self.documents.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(250)).await;

            let is_current = {
                let documents = documents.read().await;
                documents
                    .get(&uri)
                    .is_some_and(|current| current.version == snapshot.version)
            };

            if !is_current {
                return;
            }

            let diagnostics = analyze_document(&snapshot.text);

            client
                .publish_diagnostics(uri, diagnostics, Some(snapshot.version))
                .await;
        });
    }
}

fn analyze_document(text: &str) -> Vec<Diagnostic> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.len() > 100)
        .map(|(line, value)| Diagnostic {
            range: Range {
                start: Position {
                    line: line as u32,
                    character: 100,
                },
                end: Position {
                    line: line as u32,
                    character: value.len() as u32,
                },
            },
            severity: Some(DiagnosticSeverity::INFORMATION),
            source: Some("example-lsp".into()),
            message: "Line is longer than 100 characters".into(),
            ..Default::default()
        })
        .collect()
}
```

Use it from `did_change` like this:

```rust
async fn did_change(&self, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri;
    let version = params.text_document.version;
    let Some(change) = params.content_changes.into_iter().next() else {
        return;
    };

    let snapshot = DocumentSnapshot {
        version,
        text: change.text,
    };

    {
        let mut documents = self.documents.write().await;
        documents.insert(uri.clone(), snapshot.clone());
    }

    self.schedule_analysis(uri, snapshot);
}
```

## Example 4: Custom JSON-RPC Method

Custom methods are useful for editor-specific extensions that are outside standard LSP.

```rust
use serde::{Deserialize, Serialize};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[derive(Debug, Deserialize, Serialize)]
struct CompileProjectParams {
    project_id: String,
    rebuild: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct CompileProjectResult {
    accepted: bool,
}

impl Backend {
    async fn compile_project(
        &self,
        params: CompileProjectParams,
    ) -> Result<CompileProjectResult> {
        self.client
            .log_message(
                MessageType::INFO,
                format!("compile requested for {}", params.project_id),
            )
            .await;

        Ok(CompileProjectResult { accepted: true })
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend { client })
        .custom_method("example/compileProject", Backend::compile_project)
        .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
```

When using custom methods from VS Code or another client, prefer sending an explicit object for params:

```json
{
  "project_id": "main",
  "rebuild": true
}
```

Avoid relying on absent params or `null` params unless you have tested the exact client wire behavior.
