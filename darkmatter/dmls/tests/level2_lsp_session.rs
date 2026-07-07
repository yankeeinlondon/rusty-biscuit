//! Level-2 integration tests: full LSP conversations over an in-memory
//! connection pair (`lsp_server::Connection::memory()`), the fixture shape
//! ported from `iwes/tests/fixture.rs` (Apache-2.0, IWE project).
//!
//! These sessions are in-memory (no real terminal or network resource), so
//! they run ungated — the `level2_` prefix routes them to `just test-l2`.

use std::sync::mpsc;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

/// Client-side driver over the in-memory pair; joins the server thread on
/// drop-free shutdown so protocol failures surface as test failures.
struct ClientFixture {
    client: Connection,
    server_outcome: mpsc::Receiver<Result<(), String>>,
    next_id: i32,
    /// Server → client notifications buffered while awaiting a response (the
    /// server pushes `publishDiagnostics` out-of-band).
    notifications: Vec<Notification>,
}

impl ClientFixture {
    fn start() -> Self {
        let (server_side, client_side) = Connection::memory();
        let (outcome_tx, outcome_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = dmls::run_server(server_side, dmls::RunOptions::default())
                .map_err(|error| error.to_string());
            let _ = outcome_tx.send(result);
        });
        Self {
            client: client_side,
            server_outcome: outcome_rx,
            next_id: 0,
            notifications: Vec::new(),
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Response {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.send_request_with_id(id.clone(), method, params);
        self.expect_response(id)
    }

    fn send_request_with_id(&self, id: RequestId, method: &str, params: Value) {
        self.client
            .sender
            .send(Message::Request(Request::new(id, method.to_string(), params)))
            .expect("send request");
    }

    fn notify(&self, method: &str, params: Value) {
        self.client
            .sender
            .send(Message::Notification(Notification::new(
                method.to_string(),
                params,
            )))
            .expect("send notification");
    }

    fn expect_response(&mut self, id: RequestId) -> Response {
        loop {
            let message = self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("response before timeout");
            match message {
                Message::Response(response) if response.id == id => return response,
                Message::Notification(notification) => self.notifications.push(notification),
                // Server-initiated requests (progress create, watcher
                // registration) are fire-and-forget; ignore them.
                Message::Request(_) => {}
                other => panic!("unexpected message while waiting for response: {other:?}"),
            }
        }
    }

    /// Waits for the latest `publishDiagnostics` for `uri`, returning its
    /// `diagnostics` array. Drains buffered notifications first.
    fn wait_for_diagnostics(&mut self, uri: &str) -> Vec<Value> {
        loop {
            if let Some(diagnostics) = self.take_buffered_diagnostics(uri) {
                return diagnostics;
            }
            let message = self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("diagnostics before timeout");
            match message {
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(_) => {}
                other => panic!("unexpected message while waiting for diagnostics: {other:?}"),
            }
        }
    }

    fn take_buffered_diagnostics(&mut self, uri: &str) -> Option<Vec<Value>> {
        let position = self.notifications.iter().rposition(|notification| {
            notification.method == "textDocument/publishDiagnostics"
                && notification.params["uri"] == json!(uri)
        })?;
        let notification = self.notifications.remove(position);
        Some(
            notification.params["diagnostics"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
        )
    }

    fn initialize(&mut self, params: Value) -> Value {
        let response = self.request("initialize", params);
        assert!(
            response.error.is_none(),
            "initialize failed: {:?}",
            response.error
        );
        self.notify("initialized", json!({}));
        response.result.expect("initialize result")
    }

    fn shutdown(self) {
        let mut fixture = self;
        let response = fixture.request("shutdown", Value::Null);
        assert!(
            response.error.is_none(),
            "shutdown failed: {:?}",
            response.error
        );
        fixture.notify("exit", Value::Null);
        let outcome = fixture
            .server_outcome
            .recv_timeout(Duration::from_secs(10))
            .expect("server thread finished");
        assert_eq!(outcome, Ok(()), "server exited with error");
    }
}

fn neovim_like_initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Neovim", "version": "0.11.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-8", "utf-16", "utf-32"] },
            "workspace": { "configuration": true },
            "textDocument": { "foldingRange": { "lineFoldingOnly": true } }
        },
        "workspaceFolders": [
            { "uri": root_uri.as_str(), "name": "scratch" }
        ]
    })
}

#[test]
fn level2_initialize_open_change_shutdown() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join(".dmls.toml"),
        "[diagnostics]\ndebounce_ms = 150\n",
    )
    .unwrap();

    let mut fixture = ClientFixture::start();
    let result = fixture.initialize(neovim_like_initialize_params(workspace.path()));

    // Negotiation: first client-offered encoding DMLS supports.
    assert_eq!(result["capabilities"]["positionEncoding"], json!("utf-8"));
    // Full document sync with open/close.
    assert_eq!(
        result["capabilities"]["textDocumentSync"]["change"],
        json!(1)
    );
    assert_eq!(
        result["capabilities"]["textDocumentSync"]["openClose"],
        json!(true)
    );
    assert_eq!(result["serverInfo"]["name"], json!("dmls"));

    // Open → change → close a scratch document (full-sync lifecycle).
    let doc_uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": doc_uri.as_str(),
                "languageId": "markdown",
                "version": 1,
                "text": "---\ntitle: Scratch\n---\n\n# Hello\n"
            }
        }),
    );
    fixture.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": doc_uri.as_str(), "version": 2 },
            "contentChanges": [ { "text": "# Hello 💡\n" } ]
        }),
    );
    fixture.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": doc_uri.as_str() } }),
    );
    fixture.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": doc_uri.as_str() } }),
    );

    // Configuration reload path stays alive mid-session.
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "schema": { "strict": true } } } }),
    );

    // Hover is implemented now; the document was just closed, so it answers a
    // successful null (not MethodNotFound, not a hang).
    let response = fixture.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": doc_uri.as_str() },
            "position": { "line": 0, "character": 0 }
        }),
    );
    assert!(response.error.is_none(), "hover must not error: {:?}", response.error);
    assert_eq!(response.result, Some(Value::Null));

    // A genuinely unimplemented method still answers MethodNotFound.
    let response = fixture.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": doc_uri.as_str() },
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } },
            "context": { "diagnostics": [] }
        }),
    );
    assert_eq!(response.error.expect("codeAction unimplemented").code, -32601);

    fixture.shutdown();
}

#[test]
fn level2_default_negotiation_is_utf16() {
    let mut fixture = ClientFixture::start();
    let result = fixture.initialize(json!({
        "processId": null,
        "capabilities": {}
    }));
    assert_eq!(result["capabilities"]["positionEncoding"], json!("utf-16"));
    fixture.shutdown();
}

const DOC_A: &str = "---\ntitle: A\n---\n\n# Overview\n\nSee [top](#overview) and [b](b.md#target) and [missing](nope.md).\n\n## Details\n\n- one\n- two\n";
const DOC_B: &str = "# Target\n";

fn open(fixture: &ClientFixture, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": "markdown",
                "version": 1,
                "text": text
            }
        }),
    );
}

#[test]
fn level2_layer0_provider_round_trips() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("a.md"), DOC_A).unwrap();
    std::fs::write(workspace.path().join("b.md"), DOC_B).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let a_uri = url::Url::from_file_path(workspace.path().join("a.md")).unwrap();
    let b_uri = url::Url::from_file_path(workspace.path().join("b.md")).unwrap();
    // Open both buffers so the graph holds them regardless of startup timing.
    open(&fixture, b_uri.as_str(), DOC_B);
    open(&fixture, a_uri.as_str(), DOC_A);

    // documentSymbol: Overview (H1) with Details (H2) nested inside it.
    let symbols = fixture
        .request(
            "textDocument/documentSymbol",
            json!({ "textDocument": { "uri": a_uri.as_str() } }),
        )
        .result
        .expect("symbols");
    let symbols = symbols.as_array().expect("nested symbols");
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0]["name"], json!("Overview"));
    assert_eq!(symbols[0]["children"][0]["name"], json!("Details"));

    // definition on `[b](b.md#target)` → a location in b.md.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": a_uri.as_str() },
                "position": { "line": 6, "character": 30 }
            }),
        )
        .result
        .expect("definition");
    let locations = definition.as_array().expect("definition array");
    assert_eq!(locations.len(), 1);
    assert!(
        locations[0]["uri"].as_str().unwrap().ends_with("b.md"),
        "definition should point at b.md: {:?}",
        locations[0]
    );

    // references on the self-anchor link `[top](#overview)` → the link itself.
    let references = fixture
        .request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": a_uri.as_str() },
                "position": { "line": 6, "character": 12 },
                "context": { "includeDeclaration": false }
            }),
        )
        .result
        .expect("references");
    assert!(!references.as_array().unwrap().is_empty(), "expected backlinks");

    // documentLink: at least the three in-body links resolve to targets.
    let links = fixture
        .request(
            "textDocument/documentLink",
            json!({ "textDocument": { "uri": a_uri.as_str() } }),
        )
        .result
        .expect("links");
    assert!(links.as_array().unwrap().len() >= 2);

    // foldingRange: frontmatter fold present (line 0 → 2).
    let folds = fixture
        .request(
            "textDocument/foldingRange",
            json!({ "textDocument": { "uri": a_uri.as_str() } }),
        )
        .result
        .expect("folds");
    let folds = folds.as_array().expect("fold array");
    assert!(
        folds.iter().any(|f| f["startLine"] == json!(0)),
        "expected a frontmatter fold at line 0: {folds:?}"
    );

    // completion: anchor completion after `b.md#` offers b.md's heading slug.
    let completions = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": a_uri.as_str() },
                "position": { "line": 6, "character": 34 }
            }),
        )
        .result
        .expect("completions");
    let items = completions.as_array().expect("completion array");
    assert!(
        items.iter().any(|item| item["label"] == json!("target")),
        "expected `target` anchor completion: {items:?}"
    );

    // hover on `[b](b.md#target)` previews b.md's title/heading.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": a_uri.as_str() },
                "position": { "line": 6, "character": 30 }
            }),
        )
        .result
        .expect("hover");
    assert!(
        hover["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("Target"),
        "hover should mention the target: {hover:?}"
    );

    fixture.shutdown();
}

#[test]
fn level2_broken_link_diagnostic_updates_on_edit() {
    let workspace = tempfile::tempdir().unwrap();
    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("notes.md")).unwrap();
    // A same-document anchor (resolves) plus a broken relative path.
    open(
        &fixture,
        uri.as_str(),
        "# Heading\n\n[ok](#heading) and [bad](nope.md)\n",
    );

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert_eq!(diagnostics.len(), 1, "expected one broken-link diagnostic: {diagnostics:?}");
    assert_eq!(diagnostics[0]["code"], json!("dm.links.broken_path"));
    assert_eq!(diagnostics[0]["source"], json!("darkmatter.links"));

    // Fix the broken link; diagnostics clear.
    fixture.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri.as_str(), "version": 2 },
            "contentChanges": [ { "text": "# Heading\n\n[ok](#heading) and [good](#heading)\n" } ]
        }),
    );
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(diagnostics.is_empty(), "diagnostics should clear: {diagnostics:?}");

    fixture.shutdown();
}

#[test]
fn level2_wiki_link_navigation_diagnostics_and_completion() {
    let workspace = tempfile::tempdir().unwrap();
    let notes = workspace.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("Target.md"), "# Target\n\n## Section\n").unwrap();
    let source = "# Source\n\n[[Target]] and [[Target#Section]] and [[No Such Note]]\n";
    std::fs::write(notes.join("Source.md"), source).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let source_uri = url::Url::from_file_path(notes.join("Source.md")).unwrap();
    let target_uri = url::Url::from_file_path(notes.join("Target.md")).unwrap();
    open(&fixture, target_uri.as_str(), "# Target\n\n## Section\n");
    open(&fixture, source_uri.as_str(), source);

    // definition on `[[Target]]` → Target.md.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": source_uri.as_str() },
                "position": { "line": 2, "character": 4 }
            }),
        )
        .result
        .expect("definition");
    let locations = definition.as_array().expect("definition array");
    assert_eq!(locations.len(), 1);
    assert!(locations[0]["uri"].as_str().unwrap().ends_with("Target.md"));

    // The unresolved wiki target publishes a `wiki.unresolved-target` warning.
    let diagnostics = fixture.wait_for_diagnostics(source_uri.as_str());
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == json!("wiki.unresolved-target")
                && d["source"] == json!("darkmatter.wiki")),
        "expected an unresolved wiki diagnostic: {diagnostics:?}"
    );

    // completion inside `[[Ta` offers the Target document.
    let completions = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": source_uri.as_str() },
                "position": { "line": 2, "character": 4 }
            }),
        )
        .result
        .expect("completions");
    assert!(
        completions
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["label"] == json!("Target")),
        "expected a Target completion: {completions:?}"
    );

    // hover on `[[Target#Section]]` mentions the resolved target.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": source_uri.as_str() },
                "position": { "line": 2, "character": 20 }
            }),
        )
        .result
        .expect("hover");
    assert!(
        hover["contents"]["value"].as_str().unwrap().contains("Section"),
        "hover should mention the heading: {hover:?}"
    );

    fixture.shutdown();
}

/// A frontmatter document with an inline `$schema` declaring a required
/// `title` and an enum `status`; `title` is intentionally missing.
const SCHEMA_DOC: &str = "---\n$schema:\n  title: string(required)\n  status: enum(draft, published)\nstatus: draft\n---\n\n# Doc\n";

#[test]
fn level2_frontmatter_schema_intelligence() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), SCHEMA_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), SCHEMA_DOC);

    // Criterion 5-precision: the missing required `title` is diagnosed with a
    // stable code, ranged against the concrete frontmatter (not a line map).
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == json!("dm.schema.missing_required")
                && d["source"] == json!("darkmatter.schema")),
        "expected a missing-required diagnostic: {diagnostics:?}"
    );

    // Key completion offers the schema-declared `title` (a required key).
    let completions = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 4, "character": 0 }
            }),
        )
        .result
        .expect("completions");
    assert!(
        completions
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["label"] == json!("title")),
        "expected a `title` key completion: {completions:?}"
    );

    // Value completion after `status:` offers the enum members.
    let values = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 4, "character": 8 }
            }),
        )
        .result
        .expect("value completions");
    assert!(
        values
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["label"] == json!("published")),
        "expected enum value completion: {values:?}"
    );

    // Hover on the `status` key describes the enum.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 4, "character": 2 }
            }),
        )
        .result
        .expect("hover");
    assert!(
        hover["contents"]["value"].as_str().unwrap().contains("draft"),
        "hover should describe the enum: {hover:?}"
    );

    fixture.shutdown();
}

#[test]
fn level2_claudine_extension_is_pure_config() {
    // Criterion 6: a Claudine prompt activates a schema baseline through
    // configuration alone — no Claudine-specific code path exists in DMLS.
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join(".dmls.toml"),
        "[schema.extensions.claudine]\npath = \"claudine.yaml\"\nglobs = [\".claude/**\"]\n",
    )
    .unwrap();
    std::fs::write(
        workspace.path().join("claudine.yaml"),
        "$schema:\n  provider: enum(claude, openai; required)\n  model: string\n",
    )
    .unwrap();
    let claude_dir = workspace.path().join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();
    let prompt = "---\nprovider: bogus\n---\n\n# Prompt\n";
    std::fs::write(claude_dir.join("prompt.md"), prompt).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(claude_dir.join("prompt.md")).unwrap();
    open(&fixture, uri.as_str(), prompt);

    // The extension baseline validates the prompt: `provider: bogus` is not an
    // enum member, so a `darkmatter.schema` diagnostic is published.
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(
        diagnostics.iter().any(|d| d["source"] == json!("darkmatter.schema")),
        "expected an extension-schema diagnostic: {diagnostics:?}"
    );

    // Completion offers the extension-declared `model` key (present keys are
    // excluded, so only the not-yet-typed `model` from Claudine shows).
    let completions = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 1, "character": 0 }
            }),
        )
        .result
        .expect("completions");
    assert!(
        completions
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["label"] == json!("model")),
        "expected the Claudine `model` key completion: {completions:?}"
    );

    fixture.shutdown();
}

#[test]
fn level2_cancelled_request_answers_request_cancelled() {
    let mut fixture = ClientFixture::start();
    fixture.initialize(json!({ "processId": null, "capabilities": {} }));

    // A cancellation that beats its request: the router's ledger answers
    // the request with RequestCanceled (-32800) instead of dispatching it.
    fixture.notify("$/cancelRequest", json!({ "id": 99 }));
    let id = RequestId::from(99);
    fixture.send_request_with_id(id.clone(), "textDocument/hover", json!({}));
    let response = fixture.expect_response(id);
    let error = response.error.expect("cancelled request answers an error");
    assert_eq!(error.code, -32800);

    fixture.shutdown();
}
